//! Detached Settings window: opens Settings/About in a separate window
//! so the tray panel stays open.

use tauri::{Emitter, Manager, PhysicalPosition, WebviewUrl};

const SETTINGS_LABEL: &str = "settings";
const SETTINGS_WIDTH: f64 = 496.0;
const SETTINGS_HEIGHT: f64 = 580.0;

/// Open the detached Settings window, or focus it if already open.
///
/// When the window already exists, emits `settings-change-tab` so the
/// frontend can switch to the requested tab without a full reload.
pub fn open_or_focus(app: &tauri::AppHandle, tab: &str) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(SETTINGS_LABEL) {
        window.set_focus().map_err(|e| e.to_string())?;
        app.emit_to(SETTINGS_LABEL, "settings-change-tab", tab)
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    let url = WebviewUrl::App(format!("index.html?window=settings&tab={tab}").into());

    let win = tauri::WebviewWindowBuilder::new(app, SETTINGS_LABEL, url)
        .title("CodexBar Settings")
        .inner_size(SETTINGS_WIDTH, SETTINGS_HEIGHT)
        .decorations(true)
        .theme(Some(tauri::Theme::Dark))
        .resizable(true)
        .build()
        .map_err(|e| e.to_string())?;

    // Force native title bar dark mode. The builder .theme() only sets the
    // webview CSS color-scheme; the Window-level set_theme() is needed to
    // propagate DWMWA_USE_IMMERSIVE_DARK_MODE to the native title bar.
    let _ = win.set_theme(Some(tauri::Theme::Dark));

    // Manually center: Tauri's .center() is unreliable on Windows when
    // called from async commands. Compute position from the primary monitor.
    if let Ok(Some(monitor)) = win.primary_monitor() {
        let pos = monitor.position();
        let size = monitor.size();
        let scale = win.scale_factor().unwrap_or(1.0);
        let win_w = (SETTINGS_WIDTH * scale) as i32;
        let win_h = (SETTINGS_HEIGHT * scale) as i32;
        let x = pos.x + (size.width as i32 - win_w) / 2;
        let y = pos.y + (size.height as i32 - win_h) / 2;
        let _ = win.set_position(PhysicalPosition::new(x, y));
    }

    Ok(())
}
