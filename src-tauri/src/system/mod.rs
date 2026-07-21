//! System telemetry. Kept free of any Tauri types so it can be unit-tested and
//! reused from the tray worker without dragging the app handle around.

pub mod accent;
pub mod automation;
pub mod clean;
pub mod history;
pub mod memory;
pub mod minimize;
pub mod process;
pub mod settings;
