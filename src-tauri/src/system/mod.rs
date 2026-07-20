//! System telemetry. Kept free of any Tauri types so it can be unit-tested and
//! reused from the tray worker without dragging the app handle around.

pub mod accent;
pub mod clean;
pub mod memory;
pub mod process;
pub mod settings;
