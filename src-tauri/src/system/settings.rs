//! Persisted user settings.
//!
//! Stored as JSON in the app config directory. Unknown or missing fields fall
//! back to defaults so a settings file written by an older build still loads.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClickAction {
    None,
    OpenMemora,
    OpenMemoryPage,
    Optimize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    // Tray appearance
    pub show_tray_percentage: bool,
    /// Seconds between tray icon refreshes. The spec's choices are 1/2/5/10/30.
    pub tray_interval_secs: u64,
    pub warning_threshold: u8,
    pub high_threshold: u8,
    pub critical_threshold: u8,

    // Tray interactions
    pub single_click: ClickAction,
    pub double_click: ClickAction,
    pub middle_click: ClickAction,

    // Window behaviour
    pub minimize_to_tray: bool,
    pub close_to_tray: bool,
    pub start_with_windows: bool,
    pub show_optimization_notifications: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_tray_percentage: true,
            tray_interval_secs: 2,
            warning_threshold: 70,
            high_threshold: 85,
            critical_threshold: 95,
            single_click: ClickAction::OpenMemora,
            double_click: ClickAction::OpenMemoryPage,
            // Off by default: optimizing is a system-wide action and a stray
            // middle click should not trigger one.
            middle_click: ClickAction::None,
            minimize_to_tray: false,
            close_to_tray: true,
            start_with_windows: false,
            show_optimization_notifications: true,
        }
    }
}

impl Settings {
    /// Clamps values that would otherwise produce a nonsensical tray.
    pub fn sanitized(mut self) -> Self {
        self.tray_interval_secs = self.tray_interval_secs.clamp(1, 30);
        self.warning_threshold = self.warning_threshold.clamp(1, 99);
        self.high_threshold = self.high_threshold.clamp(self.warning_threshold + 1, 99);
        self.critical_threshold = self.critical_threshold.clamp(self.high_threshold + 1, 100);
        self
    }
}

/// In-memory settings plus the path they persist to.
pub struct Store {
    path: PathBuf,
    current: Mutex<Settings>,
}

impl Store {
    pub fn load(path: PathBuf) -> Self {
        let current = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
            .unwrap_or_default()
            .sanitized();

        Self {
            path,
            current: Mutex::new(current),
        }
    }

    pub fn get(&self) -> Settings {
        self.current.lock().unwrap().clone()
    }

    /// Replaces settings and writes them to disk. A write failure is returned
    /// rather than swallowed, so the UI can say the change did not persist.
    pub fn set(&self, next: Settings) -> Result<Settings, String> {
        let next = next.sanitized();
        *self.current.lock().unwrap() = next.clone();

        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("Could not create {dir:?}: {e}"))?;
        }
        let json = serde_json::to_string_pretty(&next).map_err(|e| e.to_string())?;
        std::fs::write(&self.path, json)
            .map_err(|e| format!("Could not save settings: {e}"))?;

        Ok(next)
    }
}

/// Registers or removes Memora from the per-user startup list.
///
/// HKCU only — never HKLM, which would affect every account on the machine.
#[cfg(windows)]
pub fn set_start_with_windows(enabled: bool) -> Result<(), String> {
    use windows::core::{w, PCWSTR};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY,
        HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ,
    };

    let exe = std::env::current_exe()
        .map_err(|e| format!("Cannot locate the Memora executable: {e}"))?;
    let command: Vec<u16> = format!("\"{}\"", exe.display())
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    // SAFETY: the key handle is closed on every path; buffers outlive the calls.
    unsafe {
        let mut key = HKEY::default();
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            None,
            KEY_SET_VALUE,
            &mut key,
        )
        .ok()
        .map_err(|e| format!("Cannot open the startup key: {e}"))?;

        let result = if enabled {
            let bytes = std::slice::from_raw_parts(
                command.as_ptr() as *const u8,
                command.len() * 2,
            );
            RegSetValueExW(key, w!("Memora"), None, REG_SZ, Some(bytes))
                .ok()
                .map_err(|e| format!("Cannot register startup entry: {e}"))
        } else {
            // Absent is the desired state, so "not found" is a success.
            match RegDeleteValueW(key, w!("Memora")).ok() {
                Ok(()) => Ok(()),
                Err(_) => Ok(()),
            }
        };

        let _ = RegCloseKey(key);
        let _ = PCWSTR::null();
        result
    }
}

#[cfg(not(windows))]
pub fn set_start_with_windows(_enabled: bool) -> Result<(), String> {
    Err("Only available on Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_are_kept_in_order() {
        let s = Settings {
            warning_threshold: 90,
            high_threshold: 50, // below warning
            critical_threshold: 60,
            ..Default::default()
        }
        .sanitized();

        assert!(s.warning_threshold < s.high_threshold);
        assert!(s.high_threshold < s.critical_threshold);
    }

    #[test]
    fn interval_is_clamped_to_a_sane_range() {
        let fast = Settings {
            tray_interval_secs: 0,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(fast.tray_interval_secs, 1);

        let slow = Settings {
            tray_interval_secs: 9999,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(slow.tray_interval_secs, 30);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        // A file written by an older build without the newer keys.
        let partial: Settings = serde_json::from_str(r#"{"showTrayPercentage": false}"#).unwrap();
        assert!(!partial.show_tray_percentage);
        assert_eq!(partial.tray_interval_secs, Settings::default().tray_interval_secs);
    }

    #[test]
    fn round_trips_through_disk() {
        let dir = std::env::temp_dir().join("memora-settings-test");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("settings.json");

        let store = Store::load(path.clone());
        assert_eq!(store.get().tray_interval_secs, 2);

        let mut next = store.get();
        next.tray_interval_secs = 10;
        next.show_tray_percentage = false;
        store.set(next).expect("save");

        let reloaded = Store::load(path);
        assert_eq!(reloaded.get().tray_interval_secs, 10);
        assert!(!reloaded.get().show_tray_percentage);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
