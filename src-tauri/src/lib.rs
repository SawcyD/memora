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

#[tauri::command]
fn system_accent() -> Accent {
    system::accent::accent()
}

#[tauri::command]
fn list_processes() -> Result<Vec<system::process::ProcessInfo>, String> {
    system::process::enumerate()
}

/// The UI disables the privileged methods when this is false, rather than
/// offering a toggle that would fail on click.
#[tauri::command]
fn is_elevated() -> bool {
    system::clean::is_elevated()
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
            tray::update(&app, &snap);
        }
        std::thread::sleep(Duration::from_secs(1));
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(CleanTask::default())
        .invoke_handler(tauri::generate_handler![
            memory_snapshot,
            system_accent,
            list_processes,
            is_elevated,
            start_optimization,
            cancel_optimization
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").expect("main window");

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

            // Close-to-tray: the meter is the reason Memora keeps running with
            // no window, so closing hides instead of exiting. Exit is available
            // from the tray menu.
            let handle = app.handle().clone();
            window.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if let Some(w) = handle.get_webview_window("main") {
                        let _ = w.hide();
                    }
                }
            });

            spawn_sampler(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
