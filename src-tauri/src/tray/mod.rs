//! System tray: a live memory meter that works with the main window closed.

pub mod icon;

use std::collections::HashMap;
use std::sync::Mutex;

use tauri::image::Image;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::system::accent::{self, Rgb};
use crate::system::memory::MemorySnapshot;
use crate::system::settings::{ClickAction, Settings};
use icon::UsageState;

/// Rendering a 32x32 icon costs a supersampled pass per pixel, so each distinct
/// (percent, state) pair is rasterized once and reused. Bounded at 101 x 4.
#[derive(Default)]
struct IconCache {
    entries: HashMap<(u8, u8), Vec<u8>>,
}

impl IconCache {
    fn get(&mut self, pct: u8, state: UsageState, accent: Rgb, digits: bool) -> Vec<u8> {
        // `digits` is part of the key: toggling the setting must not serve a
        // stale icon rendered under the previous choice.
        let key = (pct, (state as u8) << 1 | u8::from(digits));
        self.entries
            .entry(key)
            .or_insert_with(|| icon::render(pct, state, accent, digits))
            .clone()
    }
}

pub struct TrayState<R: Runtime> {
    cache: Mutex<IconCache>,
    /// Last percent pushed to the shell. Updates are skipped when the rounded
    /// value has not moved, which is what keeps Explorer from flickering.
    last_percent: Mutex<Option<u8>>,
    /// When the icon was last pushed, so the configured interval is honoured
    /// even though the sampler ticks every second for the graph.
    last_update: Mutex<Option<std::time::Instant>>,
    accent: Rgb,
    /// Held because `TrayIcon` exposes no menu getter; refreshing the
    /// informational rows requires the original handles.
    usage_item: MenuItem<R>,
    available_item: MenuItem<R>,
}

const MENU_OPEN: &str = "tray.open";
const MENU_USAGE: &str = "tray.usage";
const MENU_AVAILABLE: &str = "tray.available";
const MENU_EXIT: &str = "tray.exit";

/// Builds the tray menu and returns it alongside the two informational rows,
/// which the caller keeps so their text can be refreshed in place.
fn build_menu<R: Runtime>(
    app: &AppHandle<R>,
) -> tauri::Result<(Menu<R>, MenuItem<R>, MenuItem<R>)> {
    let open = MenuItem::with_id(app, MENU_OPEN, "Open Memora", true, None::<&str>)?;
    // Informational rows: disabled, matching how Windows shows read-only status
    // in a tray menu. Text is refreshed on every sample.
    let usage = MenuItem::with_id(app, MENU_USAGE, "Memory usage: —", false, None::<&str>)?;
    let available = MenuItem::with_id(app, MENU_AVAILABLE, "Available: —", false, None::<&str>)?;
    let exit = MenuItem::with_id(app, MENU_EXIT, "Exit Memora", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &open,
            &PredefinedMenuItem::separator(app)?,
            &usage,
            &available,
            &PredefinedMenuItem::separator(app)?,
            &exit,
        ],
    )?;
    Ok((menu, usage, available))
}

fn show_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn on_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    match event.id().as_ref() {
        MENU_OPEN => show_window(app),
        MENU_EXIT => app.exit(0),
        _ => {}
    }
}

/// Performs a configured click action. Each is emitted rather than executed
/// directly where the UI owns the behaviour.
fn run_action<R: Runtime>(app: &AppHandle<R>, action: ClickAction) {
    match action {
        ClickAction::None => {}
        ClickAction::OpenMemora => show_window(app),
        ClickAction::OpenMemoryPage => {
            show_window(app);
            let _ = app.emit_to("main", "tray://navigate", "memory");
        }
        ClickAction::Optimize => {
            let _ = app.emit_to("main", "tray://optimize", ());
        }
    }
}

fn on_tray_event<R: Runtime>(tray: &TrayIcon<R>, event: TrayIconEvent) {
    let app = tray.app_handle();
    // Right click is handled by the shell's own menu, so it never appears here.
    let settings = app.state::<crate::system::settings::Store>().get();

    match event {
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } => run_action(app, settings.single_click),
        TrayIconEvent::DoubleClick {
            button: MouseButton::Left,
            ..
        } => run_action(app, settings.double_click),
        TrayIconEvent::Click {
            button: MouseButton::Middle,
            button_state: MouseButtonState::Up,
            ..
        } => run_action(app, settings.middle_click),
        _ => {}
    }
}

/// Creates the tray icon. Called once during setup.
pub fn init<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<TrayIcon<R>> {
    let (menu, usage_item, available_item) = build_menu(app)?;

    app.manage(TrayState {
        cache: Mutex::new(IconCache::default()),
        last_percent: Mutex::new(None),
        last_update: Mutex::new(None),
        accent: accent::accent().tray_rgb(),
        usage_item,
        available_item,
    });

    TrayIconBuilder::with_id("memora")
        .menu(&menu)
        // The left click must reach our handler rather than opening the menu.
        .show_menu_on_left_click(false)
        .tooltip("Memora")
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(on_tray_event)
        .build(app)
}

/// Pushes a new sample to the tray, mutating the existing icon in place.
///
/// Skips all shell calls when the rounded percentage has not changed, so a
/// steady system produces no tray traffic at all.
pub fn update<R: Runtime>(app: &AppHandle<R>, snap: &MemorySnapshot, settings: &Settings) {
    let Some(tray) = app.tray_by_id("memora") else {
        return;
    };
    let state = app.state::<TrayState<R>>();

    // The sampler ticks every second for the graph; the tray only refreshes on
    // the user's chosen interval.
    {
        let mut last_update = state.last_update.lock().unwrap();
        let due = last_update.is_none_or(|t| {
            t.elapsed() >= std::time::Duration::from_secs(settings.tray_interval_secs)
        });
        if !due {
            return;
        }
        *last_update = Some(std::time::Instant::now());
    }

    let pct = snap.percent_in_use.round().clamp(0.0, 100.0) as u8;
    {
        let mut last = state.last_percent.lock().unwrap();
        if *last == Some(pct) {
            return;
        }
        *last = Some(pct);
    }

    let usage = UsageState::from_percent(
        pct,
        settings.warning_threshold,
        settings.high_threshold,
        settings.critical_threshold,
    );

    let rgba =
        state
            .cache
            .lock()
            .unwrap()
            .get(pct, usage, state.accent, settings.show_tray_percentage);
    let _ = tray.set_icon(Some(Image::new_owned(rgba, icon::SIZE, icon::SIZE)));

    let gb = |b: u64| b as f64 / 1024f64.powi(3);
    let _ = tray.set_tooltip(Some(format!(
        "Memora\nMemory: {pct}%\nUsed: {:.1} GB\nAvailable: {:.1} GB",
        gb(snap.physical_in_use),
        gb(snap.physical_available),
    )));

    let _ = state.usage_item.set_text(format!("Memory usage: {pct}%"));
    let _ = state
        .available_item
        .set_text(format!("Available: {:.1} GB", gb(snap.physical_available)));
}
