mod system;
mod tray;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Emitter, Manager, WindowEvent};

use system::{accent::Accent, memory::MemorySnapshot};

#[tauri::command]
fn memory_snapshot() -> Result<MemorySnapshot, String> {
    system::memory::snapshot()
}

/// The deeper breakdown for the Memory page. Separate from the 1 Hz sample
/// because it enumerates processes to find the compression store.
#[tauri::command]
fn memory_detail() -> Result<system::memory::MemoryDetail, String> {
    system::memory::detail()
}

#[tauri::command]
fn system_accent() -> Accent {
    system::accent::accent()
}

/// Holds the previous CPU reading so successive calls can report a rate.
#[derive(Default)]
struct ProcessSampler(Mutex<Option<system::process::Sampler>>);

#[tauri::command]
fn list_processes(
    state: tauri::State<'_, ProcessSampler>,
) -> Result<Vec<system::process::ProcessInfo>, String> {
    let mut slot = state.0.lock().unwrap();
    slot.get_or_insert_with(system::process::Sampler::new).sample()
}

/// Trims a single process, for the Processes page context menu.
#[tauri::command]
fn trim_process(pid: u32) -> Result<u64, String> {
    system::clean::trim_process(pid)
}

/// Terminates a process. The UI must confirm before calling this.
#[tauri::command]
fn end_process(pid: u32) -> Result<(), String> {
    system::process::terminate(pid)
}

/// The UI disables the privileged methods when this is false, rather than
/// offering a toggle that would fail on click.
#[tauri::command]
fn is_elevated() -> bool {
    system::clean::is_elevated()
}

#[tauri::command]
fn get_settings(store: tauri::State<'_, system::settings::Store>) -> system::settings::Settings {
    store.get()
}

/// Saves settings and applies the ones with effects outside Memora.
#[tauri::command]
fn update_settings(
    store: tauri::State<'_, system::settings::Store>,
    settings: system::settings::Settings,
) -> Result<system::settings::Settings, String> {
    let previous = store.get();
    let saved = store.set(settings)?;

    // Only touch the registry when the toggle actually changed, so opening
    // Settings does not rewrite the user's startup entry.
    if saved.start_with_windows != previous.start_with_windows {
        if let Err(e) = system::settings::set_start_with_windows(saved.start_with_windows) {
            // Roll the stored value back so the UI does not claim a state that
            // Windows did not accept.
            let _ = store.set(previous);
            return Err(e);
        }
    }

    Ok(saved)
}

#[tauri::command]
fn list_history(
    store: tauri::State<'_, system::history::Store>,
) -> Vec<system::history::Record> {
    store.list()
}

#[tauri::command]
fn clear_history(store: tauri::State<'_, system::history::Store>) -> Result<(), String> {
    store.clear()
}

/// Formats bytes for a toast, where there is no room for a units table.
fn short_bytes(bytes: i64) -> String {
    let gb = 1024f64.powi(3);
    let mb = 1024f64.powi(2);
    let v = bytes.unsigned_abs() as f64;
    if v >= gb {
        format!("{:.1} GB", v / gb)
    } else {
        format!("{:.0} MB", v / mb)
    }
}

/// Sends a Windows toast for a finished run, if the user asked for them.
///
/// The wording says "immediately" on purpose: trimming moves pages to the
/// standby list and the increase decays, so a bare "freed X" in a notification
/// the user cannot click into would overstate the result.
fn result_notification(report: &system::clean::CleanReport) -> (&'static str, String) {
    let title = if report.cancelled {
        "Memora stopped optimizing memory"
    } else {
        "Memora finished optimizing memory"
    };

    let change = if report.recovered >= 0 {
        format!("Available memory rose {} immediately", short_bytes(report.recovered))
    } else {
        // A negative delta is normal when the system allocated during the run;
        // reporting it as a gain would be a lie.
        format!("Available memory fell {} during the run", short_bytes(report.recovered))
    };

    let mut body = format!(
        "{change}. {} processes trimmed, {} skipped, in {:.1} s.",
        report.processes_trimmed,
        report.processes_skipped,
        report.duration_ms as f64 / 1000.0,
    );
    if !report.unavailable.is_empty() {
        body.push_str(" Some methods could not run.");
    }

    (title, body)
}

/// Note on delivery: Windows only renders a toast for an app whose
/// AppUserModelID is backed by a Start Menu shortcut, which the installer
/// creates. Running the bare exe from `target/` has no such shortcut, so the
/// shell accepts the toast and silently discards it — `show()` still returns
/// `Ok`, and no key appears under
/// `HKCU\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings`.
/// Notifications therefore cannot be observed from `tauri dev`; test them
/// against an installed build.
fn notify_result(app: &tauri::AppHandle, report: &system::clean::CleanReport) {
    use tauri_plugin_notification::NotificationExt;

    if !app
        .state::<system::settings::Store>()
        .get()
        .show_optimization_notifications
    {
        return;
    }

    let (title, body) = result_notification(report);
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        eprintln!("[memora] notification failed: {e}");
    }
}

fn notify_failure(app: &tauri::AppHandle, error: &str) {
    use tauri_plugin_notification::NotificationExt;

    // Failures are reported regardless of the toggle: it governs routine
    // result notifications, and silently swallowing an error is worse.
    if let Err(e) = app
        .notification()
        .builder()
        .title("Memora could not optimize memory")
        .body(error)
        .show()
    {
        eprintln!("[memora] failure notification failed: {e}");
    }
}

/// Shared cancellation flag for the in-flight optimization, if any.
#[derive(Default)]
struct CleanTask(Mutex<Option<system::clean::Cancel>>);

#[tauri::command]
fn cancel_optimization(state: tauri::State<'_, CleanTask>) {
    if let Some(flag) = state.0.lock().unwrap().as_ref() {
        flag.store(true, Ordering::Relaxed);
    }
}

/// Starts an optimization pass on a worker thread and returns immediately.
///
/// Progress arrives on `clean://progress` and the report on `clean://done`, so
/// the window never blocks on the run.
#[tauri::command]
fn start_optimization(
    app: tauri::AppHandle,
    state: tauri::State<'_, CleanTask>,
    methods: Vec<system::clean::Method>,
    excluded: Vec<u32>,
    source: Option<system::history::Source>,
) -> Result<(), String> {
    // Persisted name-based exclusions always apply, on top of any pids the
    // caller passed for this run only.
    let excluded_names = app.state::<system::settings::Store>().get().excluded_processes;
    let mut slot = state.0.lock().unwrap();
    if slot.as_ref().is_some_and(|f| !f.load(Ordering::Relaxed)) {
        return Err("An optimization is already running".into());
    }

    let cancel: system::clean::Cancel = Arc::new(AtomicBool::new(false));
    *slot = Some(cancel.clone());
    drop(slot);

    std::thread::spawn(move || {
        let progress_app = app.clone();
        let result = system::clean::run(&methods, &excluded, &excluded_names, cancel, move |p| {
            let _ = progress_app.emit("clean://progress", p);
        });

        match result {
            Ok(report) => {
                let _ = app.emit("clean://done", &report);
                notify_result(&app, &report);

                // Recorded before the delayed measurement so a run survives in
                // history even if Memora exits during the 30 second wait.
                let record = system::history::Record::from_report(
                    source.unwrap_or(system::history::Source::Manual),
                    &methods,
                    &report,
                );
                let at = record.at;
                if let Err(e) = app.state::<system::history::Store>().append(&record) {
                    eprintln!("[memora] history append failed: {e}");
                }

                // Working-set trimming moves pages to the standby list, and the
                // OS faults them back as processes resume. Re-measuring after a
                // delay is the only honest way to report what actually stuck.
                let available_before = report.available_before;
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(30));
                    if let Ok(later) = system::memory::snapshot() {
                        let settled = later.physical_available as i64 - available_before as i64;
                        let _ = app.emit("clean://settled", settled);
                        let _ = app.state::<system::history::Store>().set_settled(at, settled);
                    }
                });
            }
            Err(e) => {
                notify_failure(&app, &e);
                let failed = system::history::Record {
                    at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0),
                    source: source.unwrap_or(system::history::Source::Manual),
                    outcome: system::history::RunOutcome::Failed { error: e.clone() },
                    methods: methods.clone(),
                    ..Default::default()
                };
                let _ = app.state::<system::history::Store>().append(&failed);
                let _ = app.emit("clean://failed", e);
            }
        }
    });

    Ok(())
}

/// Samples memory on a background thread and fans the result out to both the
/// window and the tray, so the graph and the meter can never disagree.
///
/// Rasterizing happens here rather than on the UI thread; `tray::update` bails
/// out early when the rounded percentage has not moved.
fn spawn_sampler(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        if let Ok(snap) = system::memory::snapshot() {
            let _ = app.emit("memory://sample", snap);
            let settings = app.state::<system::settings::Store>().get();
            tray::update(&app, &snap, &settings);
        }
        // Fixed 1 Hz: the graph needs a steady series regardless of how often
        // the tray chooses to redraw.
        std::thread::sleep(Duration::from_secs(1));
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be registered first. Two instances would race on settings.json
        // and history.jsonl, and would show two tray meters.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // A second launch surfaces the running window instead of starting
            // over — the same behaviour as clicking the tray icon.
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(CleanTask::default())
        .manage(ProcessSampler::default())
        .invoke_handler(tauri::generate_handler![
            memory_snapshot,
            memory_detail,
            system_accent,
            list_processes,
            trim_process,
            end_process,
            is_elevated,
            get_settings,
            update_settings,
            list_history,
            clear_history,
            start_optimization,
            cancel_optimization
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").expect("main window");

            // Settings must exist before the tray, which reads click actions
            // from them on every event.
            let config_dir = app
                .path()
                .app_config_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            app.manage(system::settings::Store::load(config_dir.join("settings.json")));
            app.manage(system::history::Store::new(config_dir.join("history.jsonl")));

            // Mica is the main-surface backdrop per the design rules. It needs
            // Windows 11 build 22000+; older builds fall back to a solid theme
            // background, which is an acceptable degradation.
            #[cfg(windows)]
            {
                use tauri::window::{Color, Effect, EffectsBuilder};

                // `state` and `radius` on EffectsBuilder are macOS-only, so Mica
                // is configured by effect alone here.
                if window
                    .set_effects(EffectsBuilder::new().effect(Effect::Mica).build())
                    .is_err()
                {
                    let _ = window.set_background_color(Some(Color(243, 243, 243, 255)));
                }
            }

            tray::init(app.handle())?;

            // The window-state plugin restores whatever state Memora exited in,
            // including minimized. Combined with the tray that leaves the app
            // apparently missing on launch, so a minimized restore is undone.
            if window.is_minimized().unwrap_or(false) {
                let _ = window.unminimize();
            }

            let handle = app.handle().clone();
            window.on_window_event(move |event| {
                let settings = handle.state::<system::settings::Store>().get();
                let hide = || {
                    if let Some(w) = handle.get_webview_window("main") {
                        let _ = w.hide();
                    }
                };

                match event {
                    // Close-to-tray: the meter is the reason Memora keeps
                    // running without a window. Exit lives in the tray menu.
                    WindowEvent::CloseRequested { api, .. } if settings.close_to_tray => {
                        api.prevent_close();
                        hide();
                    }
                    // Minimize-to-tray removes Memora from the taskbar too.
                    WindowEvent::Resized(_) if settings.minimize_to_tray => {
                        if let Some(w) = handle.get_webview_window("main") {
                            if w.is_minimized().unwrap_or(false) {
                                hide();
                            }
                        }
                    }
                    _ => {}
                }
            });

            spawn_sampler(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod notification_tests {
    use super::*;
    use system::clean::CleanReport;

    fn report(recovered: i64, cancelled: bool) -> CleanReport {
        CleanReport {
            available_before: 8_000_000_000,
            available_after: (8_000_000_000i64 + recovered) as u64,
            recovered,
            processes_trimmed: 18,
            processes_skipped: 7,
            errors: 0,
            duration_ms: 800,
            cancelled,
            details: Vec::new(),
            unavailable: Vec::new(),
        }
    }

    #[test]
    fn gains_are_described_as_immediate() {
        let (title, body) = result_notification(&report(1_288_490_188, false));
        assert_eq!(title, "Memora finished optimizing memory");
        // The qualifier is the whole point: the increase decays.
        assert!(body.contains("immediately"), "{body}");
        assert!(body.contains("1.2 GB"), "{body}");
        assert!(body.contains("18 processes trimmed"), "{body}");
        assert!(body.contains("0.8 s"), "{body}");
    }

    /// A run can end with less memory available than it started with. That must
    /// never be phrased as a gain.
    #[test]
    fn losses_are_not_reported_as_gains() {
        let (_, body) = result_notification(&report(-500_000_000, false));
        assert!(body.contains("fell"), "{body}");
        assert!(!body.contains("rose"), "{body}");
        // No stray minus sign: the direction is carried by the wording.
        assert!(!body.contains("-"), "{body}");
    }

    #[test]
    fn cancellation_does_not_claim_completion() {
        let (title, _) = result_notification(&report(0, true));
        assert!(!title.contains("finished"), "{title}");
        assert!(title.contains("stopped"), "{title}");
    }

    #[test]
    fn unavailable_methods_are_mentioned() {
        let mut r = report(0, false);
        r.unavailable.push("Clear standby memory: requires administrator".into());
        let (_, body) = result_notification(&r);
        assert!(body.contains("could not run"), "{body}");
    }

    #[test]
    fn byte_formatting_picks_sane_units() {
        assert_eq!(short_bytes(1_288_490_188), "1.2 GB");
        assert_eq!(short_bytes(52_428_800), "50 MB");
        assert_eq!(short_bytes(-52_428_800), "50 MB");
    }
}
