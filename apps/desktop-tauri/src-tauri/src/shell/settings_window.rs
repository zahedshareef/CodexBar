//! Detached Settings window: opens Settings/About in a separate window
//! so the tray panel stays open.

use tauri::{Emitter, Manager, PhysicalPosition, WebviewUrl};

const SETTINGS_LABEL: &str = "settings";
const SETTINGS_WIDTH: f64 = 720.0;
const SETTINGS_HEIGHT: f64 = 580.0;

/// Force the DWM caption bar to match our dark background so the
/// residual non-client area left by `decorations(false)` is invisible.
#[cfg(windows)]
fn force_dark_caption(win: &tauri::WebviewWindow) {
    use raw_window_handle::HasWindowHandle;
    use std::ffi::c_void;

    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmSetWindowAttribute(
            hwnd: isize,
            attr: u32,
            data: *const c_void,
            size: u32,
        ) -> i32;
    }

    let Ok(handle) = win.window_handle() else { return };
    let raw_window_handle::RawWindowHandle::Win32(h) = handle.as_raw() else {
        return;
    };
    let hwnd = h.hwnd.get() as isize;

    const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
    const DWMWA_CAPTION_COLOR: u32 = 35;
    // COLORREF 0x00BBGGRR — #1e1e1e → R=G=B=0x1E
    let dark_mode: u32 = 1;
    let caption_color: u32 = 0x001E1E1E;

    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &raw const dark_mode as *const c_void,
            4,
        );
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_COLOR,
            &raw const caption_color as *const c_void,
            4,
        );
    }
}

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
        .decorations(false)
        .shadow(false)
        .theme(Some(tauri::Theme::Dark))
        .resizable(true)
        .build()
        .map_err(|e| e.to_string())?;

    // Force DWM caption to dark so the residual non-client strip is invisible
    #[cfg(windows)]
    force_dark_caption(&win);

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
