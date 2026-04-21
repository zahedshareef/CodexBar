//! Detached Settings window: opens Settings/About in a separate window
//! so the tray panel stays open.

use tauri::{Emitter, Manager, WebviewUrl};

const SETTINGS_LABEL: &str = "settings";

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
        .inner_size(496.0, 580.0)
        .decorations(true)
        .resizable(true)
        .build()
        .map_err(|e| e.to_string())?;

    // Explicitly center on the monitor — the builder's .center() is
    // unreliable on Windows when called from an async Tauri command.
    let _ = win.center();

    Ok(())
}
