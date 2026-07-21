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

/// Processes trimmed by a minimize rule and not yet restored. The executable
/// name guards against stale pids being reused by Windows.
#[derive(Default)]
struct MinimizeTrimState(Mutex<std::collections::HashMap<u32, String>>);

#[derive(Default)]
struct MinimizeMonitorStatus(AtomicBool);

#[tauri::command]
fn minimize_monitor_available(state: tauri::State<'_, MinimizeMonitorStatus>) -> bool {
    state.0.load(Ordering::Relaxed)
}

#[tauri::command]
fn list_processes(
    state: tauri::State<'_, ProcessSampler>,
    minimize_state: tauri::State<'_, MinimizeTrimState>,
) -> Result<Vec<system::process::ProcessInfo>, String> {
    let mut slot = state.0.lock().unwrap();
    let mut processes = slot
        .get_or_insert_with(system::process::Sampler::new)
        .sample()?;
    let live: std::collections::HashMap<u32, String> = processes
        .iter()
        .map(|p| (p.pid, p.name.to_ascii_lowercase()))
        .collect();
    let mut trimmed = minimize_state.0.lock().unwrap();
    trimmed.retain(|pid, name| live.get(pid).is_some_and(|live_name| live_name == name));
    for process in &mut processes {
        process.minimized_trimmed = trimmed
            .get(&process.pid)
            .is_some_and(|name| name == &process.name.to_ascii_lowercase());
    }
    Ok(processes)
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
fn list_history(store: tauri::State<'_, system::history::Store>) -> Vec<system::history::Record> {
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
        format!(
            "Available memory rose {} immediately",
            short_bytes(report.recovered)
        )
    } else {
        // A negative delta is normal when the system allocated during the run;
        // reporting it as a gain would be a lie.
        format!(
            "Available memory fell {} during the run",
            short_bytes(report.recovered)
        )
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

impl CleanTask {
    fn in_flight(&self) -> bool {
        self.0.lock().unwrap().is_some()
    }

    /// Releases only the run that owns this flag. The identity check matters
    /// if a cancelled worker finishes after a later run has already started.
    fn finish(&self, completed: &system::clean::Cancel) {
        let mut slot = self.0.lock().unwrap();
        if slot
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, completed))
        {
            *slot = None;
        }
    }
}

/// Clears the single-flight slot even if the cleaning worker panics.
struct CleanRunLease {
    app: tauri::AppHandle,
    flag: system::clean::Cancel,
}

impl Drop for CleanRunLease {
    fn drop(&mut self) {
        self.app.state::<CleanTask>().finish(&self.flag);
    }
}

#[cfg(test)]
mod clean_task_tests {
    use super::*;

    fn flag() -> system::clean::Cancel {
        Arc::new(AtomicBool::new(false))
    }

    #[test]
    fn finishing_a_run_releases_the_single_flight_slot() {
        let task = CleanTask::default();
        let active = flag();
        *task.0.lock().unwrap() = Some(active.clone());

        assert!(task.in_flight());
        task.finish(&active);
        assert!(!task.in_flight());
    }

    #[test]
    fn cancellation_does_not_release_the_slot_before_the_worker_finishes() {
        let task = CleanTask::default();
        let active = flag();
        *task.0.lock().unwrap() = Some(active.clone());

        active.store(true, Ordering::Relaxed);
        assert!(task.in_flight());
        task.finish(&active);
        assert!(!task.in_flight());
    }

    #[test]
    fn stale_worker_cannot_clear_a_newer_run() {
        let task = CleanTask::default();
        let stale = flag();
        let current = flag();
        *task.0.lock().unwrap() = Some(current);

        task.finish(&stale);
        assert!(task.in_flight());
    }
}

/// Automation scheduling state. Separate from the config, which lives in
/// settings: this is runtime bookkeeping and is deliberately not persisted, so
/// a restart never inherits a stale cooldown or suspension.
#[derive(Default)]
struct AutomationEngine(Mutex<system::automation::Engine>);

#[tauri::command]
fn resume_rule(engine: tauri::State<'_, AutomationEngine>, rule: String) {
    engine.0.lock().unwrap().resume(&rule);
}

/// Rules the engine has suspended for repeatedly recovering little memory.
#[tauri::command]
fn suspended_rules(
    engine: tauri::State<'_, AutomationEngine>,
    store: tauri::State<'_, system::settings::Store>,
) -> Vec<String> {
    let config = store.get().automation;
    let engine = engine.0.lock().unwrap();
    config
        .active()
        .map(|p| {
            p.rules
                .iter()
                .filter(|r| engine.is_suspended(r))
                .map(|r| r.id.clone())
                .collect()
        })
        .unwrap_or_default()
}

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
    methods: Vec<system::clean::Method>,
    excluded: Vec<u32>,
    source: Option<system::history::Source>,
) -> Result<(), String> {
    start_optimization_inner(
        app,
        methods,
        excluded,
        source.unwrap_or(system::history::Source::Manual),
    )
}

/// The run itself, callable from both the command and the automation
/// evaluator. Both go through the same single-flight guard, so an automatic
/// run can never collide with one the user started.
fn start_optimization_inner(
    app: tauri::AppHandle,
    methods: Vec<system::clean::Method>,
    excluded: Vec<u32>,
    source: system::history::Source,
) -> Result<(), String> {
    // Persisted name-based exclusions always apply, on top of any pids the
    // caller passed for this run only.
    let excluded_names = app
        .state::<system::settings::Store>()
        .get()
        .excluded_processes;
    let state = app.state::<CleanTask>();
    let mut slot = state.0.lock().unwrap();
    if slot.is_some() {
        return Err("An optimization is already running".into());
    }

    let cancel: system::clean::Cancel = Arc::new(AtomicBool::new(false));
    *slot = Some(cancel.clone());
    drop(slot);

    std::thread::spawn(move || {
        // The lease fixes a deadlock where a completed run remained in the
        // slot forever, blocking every later manual and automatic run.
        let lease = CleanRunLease {
            app: app.clone(),
            flag: cancel.clone(),
        };
        let progress_app = app.clone();
        let result = system::clean::run(&methods, &excluded, &excluded_names, cancel, move |p| {
            let _ = progress_app.emit("clean://progress", p);
        });
        // A completed report must be actionable immediately; the separate
        // 30-second settled measurement is not part of the active run.
        drop(lease);

        match result {
            Ok(report) => {
                let _ = app.emit("clean://done", &report);
                notify_result(&app, &report);

                // Recorded before the delayed measurement so a run survives in
                // history even if Memora exits during the 30 second wait.
                let record =
                    system::history::Record::from_report(source.clone(), &methods, &report);
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
                        let _ = app
                            .state::<system::history::Store>()
                            .set_settled(at, settled);

                        // The effectiveness gate needs the settled figure: a
                        // rule that keeps recovering nothing suspends itself.
                        if let system::history::Source::Automation { rule } = &source {
                            app.state::<AutomationEngine>()
                                .0
                                .lock()
                                .unwrap()
                                .record_settled(rule, settled);
                        }
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
                    source: source.clone(),
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
    let started = std::time::Instant::now();

    std::thread::spawn(move || loop {
        if let Ok(snap) = system::memory::snapshot() {
            let _ = app.emit("memory://sample", snap);
            let settings = app.state::<system::settings::Store>().get();
            tray::update(&app, &snap, &settings);

            // Automation is the least important thing on this thread: a panic
            // here must not take the tray meter and graph down with it.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                evaluate_automation(&app, &settings, &snap, started.elapsed().as_millis() as u64)
            }));
            if result.is_err() {
                eprintln!("[memora] automation evaluator panicked; disabled for this session");
                break;
            }
        }
        // Fixed 1 Hz: the graph needs a steady series regardless of how often
        // the tray chooses to redraw.
        std::thread::sleep(Duration::from_secs(1));
    });
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MinimizeTrimNotice {
    process: String,
    pid: u32,
    working_set_before: u64,
    working_set_after: u64,
}

/// Applies delayed minimize rules without doing work in the Windows callback.
/// A restore event cancels the pending action, and the window state is checked
/// again immediately before touching the process.
fn spawn_minimize_worker(
    app: tauri::AppHandle,
    receiver: std::sync::mpsc::Receiver<system::minimize::Event>,
) {
    use std::collections::HashMap;
    use std::sync::mpsc::RecvTimeoutError;
    use std::time::Instant;
    use system::minimize::EventKind;

    #[derive(Clone, Copy)]
    struct Pending {
        pid: u32,
        due: Instant,
    }

    std::thread::spawn(move || {
        let mut pending: HashMap<isize, Pending> = HashMap::new();
        let mut last_trimmed: HashMap<String, Instant> = HashMap::new();

        loop {
            let timeout = pending
                .values()
                .map(|p| p.due.saturating_duration_since(Instant::now()))
                .min()
                .unwrap_or(Duration::from_secs(60));

            match receiver.recv_timeout(timeout) {
                Ok(event) => match event.kind {
                    EventKind::Minimized => {
                        let delay = app
                            .state::<system::settings::Store>()
                            .get()
                            .minimize_trim
                            .delay_secs;
                        pending.insert(
                            event.hwnd,
                            Pending {
                                pid: event.pid,
                                due: Instant::now() + Duration::from_secs(delay),
                            },
                        );
                    }
                    EventKind::Restored => {
                        pending.remove(&event.hwnd);
                        app.state::<MinimizeTrimState>()
                            .0
                            .lock()
                            .unwrap()
                            .remove(&event.pid);
                    }
                },
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }

            let now = Instant::now();
            let due: Vec<(isize, Pending)> = pending
                .iter()
                .filter(|(_, item)| item.due <= now)
                .map(|(hwnd, item)| (*hwnd, *item))
                .collect();

            for (hwnd, item) in due {
                pending.remove(&hwnd);
                if !system::minimize::is_minimized(hwnd) || app.state::<CleanTask>().in_flight() {
                    continue;
                }

                let settings = app.state::<system::settings::Store>().get();
                let config = settings.minimize_trim;
                if !config.enabled {
                    continue;
                }

                let Ok(processes) = system::process::enumerate() else {
                    continue;
                };
                let Some(process) = processes.into_iter().find(|p| p.pid == item.pid) else {
                    continue;
                };
                let name = process.name.to_ascii_lowercase();
                let minimum_bytes = config.minimum_working_set_mb * 1024 * 1024;
                if !process.accessible
                    || !config.includes(&name)
                    || settings.excluded_processes.binary_search(&name).is_ok()
                    || process.working_set < minimum_bytes
                    || last_trimmed
                        .get(&name)
                        .is_some_and(|at| at.elapsed() < Duration::from_secs(config.cooldown_secs))
                {
                    continue;
                }

                let started = Instant::now();
                let result = system::clean::trim_process(process.pid);
                let record = system::history::Record::from_minimize(
                    process.name.clone(),
                    process.pid,
                    process.working_set,
                    result.clone(),
                    started.elapsed().as_millis() as u64,
                );
                if let Err(e) = app.state::<system::history::Store>().append(&record) {
                    eprintln!("[memora] minimize history append failed: {e}");
                }

                if let Ok(after) = result {
                    last_trimmed.insert(name.clone(), Instant::now());
                    if system::minimize::is_minimized(hwnd) {
                        app.state::<MinimizeTrimState>()
                            .0
                            .lock()
                            .unwrap()
                            .insert(process.pid, name);
                    }
                    let _ = app.emit(
                        "minimize://trimmed",
                        MinimizeTrimNotice {
                            process: process.name,
                            pid: process.pid,
                            working_set_before: process.working_set,
                            working_set_after: after,
                        },
                    );
                }
            }
        }
    });
}

/// Evaluates automation for one tick and performs the decision.
///
/// `now_ms` is monotonic time since launch, not wall clock: a clock change or
/// DST shift must never fire a rule.
fn evaluate_automation(
    app: &tauri::AppHandle,
    settings: &system::settings::Settings,
    snap: &MemorySnapshot,
    now_ms: u64,
) {
    use system::automation::{Context, Decision};

    let ctx = Context {
        now_ms,
        percent_in_use: snap.percent_in_use,
        idle_secs: system::automation::idle_secs(),
        foreground_busy: system::automation::foreground_busy(),
        elevated: system::clean::is_elevated(),
        run_in_flight: app.state::<CleanTask>().in_flight(),
    };

    let decision = app
        .state::<AutomationEngine>()
        .0
        .lock()
        .unwrap()
        .evaluate(&settings.automation, ctx);

    match decision {
        Decision::Idle => {}

        Decision::Blocked { rule, gate } => {
            // Only recorded when automation is actually on. Logging every tick
            // while disabled would bury the informative entries.
            if settings.automation.enabled && gate != system::automation::Gate::Disabled {
                let record = system::history::Record {
                    at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0),
                    source: system::history::Source::Automation { rule },
                    outcome: system::history::RunOutcome::Blocked {
                        gate: gate.describe().to_string(),
                    },
                    ..Default::default()
                };
                let _ = app.state::<system::history::Store>().append(&record);
            }
        }

        Decision::Run { rule, methods } => {
            let _ = app.emit("automation://run", &rule);
            if let Err(e) = start_optimization_inner(
                app.clone(),
                methods,
                Vec::new(),
                system::history::Source::Automation { rule },
            ) {
                eprintln!("[memora] automation could not start a run: {e}");
            }
        }
    }
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(CleanTask::default())
        .manage(ProcessSampler::default())
        .manage(MinimizeTrimState::default())
        .manage(MinimizeMonitorStatus::default())
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
            resume_rule,
            suspended_rules,
            minimize_monitor_available,
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
            app.manage(system::settings::Store::load(
                config_dir.join("settings.json"),
            ));
            app.manage(system::history::Store::new(
                config_dir.join("history.jsonl"),
            ));
            app.manage(AutomationEngine::default());

            // Register on the Tauri message-loop thread; the callback only
            // enqueues tiny event records and the worker handles all policy.
            #[cfg(windows)]
            {
                let (sender, receiver) = std::sync::mpsc::channel();
                match system::minimize::install(sender) {
                    Ok(()) => {
                        app.state::<MinimizeMonitorStatus>()
                            .0
                            .store(true, Ordering::Relaxed);
                        spawn_minimize_worker(app.handle().clone(), receiver);
                    }
                    Err(error) => {
                        // This experimental feature must never prevent the
                        // core monitor and manual cleaner from starting.
                        eprintln!("[memora] minimize monitor unavailable: {error}");
                    }
                }
            }

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
        r.unavailable
            .push("Clear standby memory: requires administrator".into());
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
