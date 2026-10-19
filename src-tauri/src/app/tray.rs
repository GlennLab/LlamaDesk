//! System tray widget: a live status string and the window controls.
//!
//! The compact status line is derived from the current [`MonitorSnapshot`] by the
//! pure [`status_text`] function. A Rust-side listener on [`EVENT_SNAPSHOT`]
//! pushes it to the tray tooltip/title and to the disabled status menu item, so
//! the tray reuses the existing snapshot emit path instead of adding a second
//! polling loop.
//!
//! Platform notes: the tooltip is unsupported on Linux (the indicator label is
//! shown instead) and the title is unsupported on Windows; each platform simply
//! ignores the unsupported setter. Left-click-to-toggle is only delivered where
//! the platform reports tray click events — Linux AppIndicator always opens the
//! menu instead, so the menu's "Show / Hide window" item is the portable path.

use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Listener, Manager, Window, WindowEvent};

use crate::app::state::EVENT_SNAPSHOT;
use crate::domain::snapshot::{ConnectionState, MonitorSnapshot};

/// Label of the single application window (see `tauri.conf.json`).
const MAIN_WINDOW: &str = "main";

/// Identifier that lets the tray icon be looked up from the app handle.
const TRAY_ID: &str = "llamadesk-tray";

/// Status menu item identifier.
const MENU_STATUS: &str = "status";
/// Show/hide window menu item identifier.
const MENU_TOGGLE: &str = "toggle-window";
/// Quit menu item identifier.
const MENU_QUIT: &str = "quit";

/// Placeholder for a value that is not available, matching the frontend
/// contract's `PLACEHOLDER` (`—`). A missing figure is never rendered as `0`.
const PLACEHOLDER: &str = "—";

/// Builds the tray icon, its menu, and the snapshot listener.
///
/// Must be called once from the Tauri `setup` hook (main thread). Errors are
/// returned rather than swallowed so a broken tray surfaces at startup.
pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let initial = status_text(&MonitorSnapshot::default());
    let status = MenuItem::with_id(app, MENU_STATUS, initial.clone(), false, None::<&str>)?;
    let toggle = MenuItem::with_id(app, MENU_TOGGLE, "Show / Hide window", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit LlamaDesk", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&status, &separator, &toggle, &quit])?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        // Where the platform reports tray clicks, a left click toggles the
        // window instead of opening the menu.
        .show_menu_on_left_click(false)
        .tooltip(initial.clone())
        .title(initial)
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(on_tray_icon_event);
    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder.build(app)?;

    let status_item = status.clone();
    let handle = app.clone();
    app.listen(EVENT_SNAPSHOT, move |event| {
        match serde_json::from_str::<MonitorSnapshot>(event.payload()) {
            Ok(snapshot) => update_status(&handle, &status_item, &status_text(&snapshot)),
            Err(error) => log::warn!("tray: cannot parse {EVENT_SNAPSHOT}: {error}"),
        }
    });

    Ok(())
}

/// Hides the main window instead of exiting when it is closed (close-to-tray).
///
/// Runs from the Tauri window-event hook. The monitor loops are independent
/// tokio tasks, so they keep running while the window is hidden; the tray "Quit"
/// item is the only way to actually exit once this is active.
pub fn handle_window_event(window: &Window, event: &WindowEvent) {
    if window.label() != MAIN_WINDOW {
        return;
    }
    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window.hide();
    }
}

/// Builds the compact status line shown in the tray tooltip/title and in the
/// disabled status menu item.
///
/// A connected snapshot reports the loaded-model count and the first GPU's
/// utilisation; a missing utilisation is rendered as [`PLACEHOLDER`]. The
/// disconnected and error states report only the state, so stale figures are
/// never shown as if they were live.
pub fn status_text(snapshot: &MonitorSnapshot) -> String {
    match snapshot.connection {
        ConnectionState::Disconnected => "disconnected".to_string(),
        ConnectionState::Error => "error".to_string(),
        ConnectionState::Connected => {
            let models = snapshot.loaded_models.len();
            let noun = if models == 1 { "model" } else { "models" };
            format!(
                "connected · {models} {noun} · GPU {}",
                gpu_utilization(snapshot)
            )
        }
    }
}

/// Formats the first GPU's utilisation as a whole percent, or [`PLACEHOLDER`]
/// when no GPU or no utilisation figure is available.
fn gpu_utilization(snapshot: &MonitorSnapshot) -> String {
    snapshot
        .gpus
        .first()
        .and_then(|gpu| gpu.utilization_percent)
        .map(|percent| format!("{percent:.0}%"))
        .unwrap_or_else(|| PLACEHOLDER.to_string())
}

/// Handles a tray menu selection.
fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        MENU_TOGGLE => toggle_window(app),
        MENU_QUIT => app.exit(0),
        _ => {}
    }
}

/// Toggles the main window on a left click of the tray icon.
fn on_tray_icon_event(tray: &TrayIcon, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        toggle_window(tray.app_handle());
    }
}

/// Shows and focuses the main window when hidden, hides it when visible.
fn toggle_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        log::warn!("tray: window {MAIN_WINDOW} is missing");
        return;
    };
    if matches!(window.is_visible(), Ok(true)) {
        let _ = window.hide();
    } else {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Pushes `text` to the tray tooltip, title and the disabled status item.
fn update_status(app: &AppHandle, status_item: &MenuItem<tauri::Wry>, text: &str) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if let Err(error) = tray.set_tooltip(Some(text)) {
            log::warn!("tray: cannot set tooltip: {error}");
        }
        if let Err(error) = tray.set_title(Some(text)) {
            log::warn!("tray: cannot set title: {error}");
        }
    }
    if let Err(error) = status_item.set_text(text) {
        log::warn!("tray: cannot set status item text: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::hardware::GpuStats;

    /// Builds a connected snapshot with `models` loaded models and `gpus`.
    fn connected(models: usize, gpus: Vec<GpuStats>) -> MonitorSnapshot {
        let mut snapshot = MonitorSnapshot {
            connection: ConnectionState::Connected,
            gpus,
            ..MonitorSnapshot::default()
        };
        snapshot.loaded_models = (0..models).map(|i| format!("model-{i}")).collect();
        snapshot
    }

    /// A GPU reporting `utilization` percent, or nothing when `None`.
    fn gpu(utilization: Option<f64>) -> GpuStats {
        GpuStats {
            index: 0,
            name: "GPU".to_string(),
            utilization_percent: utilization,
            memory_total_mib: None,
            memory_used_mib: None,
            temperature_c: None,
            power_draw_w: None,
            power_limit_w: None,
            fan_speed_percent: None,
        }
    }

    #[test]
    fn disconnected_shows_only_state() {
        assert_eq!(status_text(&MonitorSnapshot::default()), "disconnected");
    }

    #[test]
    fn disconnected_hides_stale_numbers() {
        let mut snapshot = connected(4, vec![gpu(Some(90.0))]);
        snapshot.connection = ConnectionState::Disconnected;

        assert_eq!(status_text(&snapshot), "disconnected");
    }

    #[test]
    fn error_shows_only_state() {
        let mut snapshot = connected(2, vec![gpu(Some(10.0))]);
        snapshot.connection = ConnectionState::Error;

        assert_eq!(status_text(&snapshot), "error");
    }

    #[test]
    fn connected_reports_model_count_and_gpu_utilization() {
        let snapshot = connected(2, vec![gpu(Some(45.4))]);

        assert_eq!(status_text(&snapshot), "connected · 2 models · GPU 45%");
    }

    #[test]
    fn connected_single_model_is_not_pluralised() {
        let snapshot = connected(1, vec![gpu(Some(0.0))]);

        assert_eq!(status_text(&snapshot), "connected · 1 model · GPU 0%");
    }

    #[test]
    fn connected_zero_models_reports_zero() {
        let snapshot = connected(0, vec![gpu(Some(45.0))]);

        assert_eq!(status_text(&snapshot), "connected · 0 models · GPU 45%");
    }

    #[test]
    fn missing_gpu_utilization_is_placeholder_not_zero() {
        let snapshot = connected(3, vec![gpu(None)]);

        assert_eq!(status_text(&snapshot), "connected · 3 models · GPU —");
    }

    #[test]
    fn no_gpus_is_placeholder() {
        let snapshot = connected(1, Vec::new());

        assert_eq!(status_text(&snapshot), "connected · 1 model · GPU —");
    }
}
