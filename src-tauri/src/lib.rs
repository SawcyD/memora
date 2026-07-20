mod system;

use std::time::Duration;
use tauri::{Emitter, Manager};

use system::{accent::Accent, memory::MemorySnapshot};

#[tauri::command]
fn memory_snapshot() -> Result<MemorySnapshot, String> {
    system::memory::snapshot()
}

#[tauri::command]
fn system_accent() -> Accent {
    system::accent::accent()
}

/// Pushes a snapshot to the window on a fixed cadence so the graph has a single
/// sampling source. The UI never polls on its own.
fn spawn_sampler(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        if let Ok(snap) = system::memory::snapshot() {
            let _ = app.emit("memory://sample", snap);
        }
        std::thread::sleep(Duration::from_secs(1));
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![memory_snapshot, system_accent])
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

            spawn_sampler(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
