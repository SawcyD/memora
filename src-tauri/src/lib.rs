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
) -> Result<(), String> {
    let mut slot = state.0.lock().unwrap();
    if slot.as_ref().is_some_and(|f| !f.load(Ordering::Relaxed)) {
        return Err("An optimization is already running".into());
    }

    let cancel: system::clean::Cancel = Arc::new(AtomicBool::new(false));
    *slot = Some(cancel.clone());
    drop(slot);

    std::thread::spawn(move || {
        let progress_app = app.clone();
        let result = system::clean::run(&methods, &excluded, cancel, move |p| {
            let _ = progress_app.emit("clean://progress", p);
        });

        match result {
            Ok(report) => {
                let _ = app.emit("clean://done", &report);

                // Working-set trimming moves pages to the standby list, and the
                // OS faults them back as processes resume. Re-measuring after a
                // delay is the only honest way to report what actually stuck.
                let available_before = report.available_before;
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(30));
                    if let Ok(later) = system::memory::snapshot() {
                        let _ = app.emit(
                            "clean://settled",
                            later.physical_available as i64 - available_before as i64,
                        );
                    }
                });
            }
            Err(e) => {
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
        .plugin(tauri_plugin_opener::init())
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
