//! Windows DWM helpers for controlling the non-client caption area.
//!
//! Even with `decorations(false)`, Windows keeps a thin caption strip.
//! These helpers paint it dark so it blends with the webview background.

#[cfg(windows)]
use std::ffi::c_void;

#[cfg(windows)]
#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmSetWindowAttribute(hwnd: isize, attr: u32, data: *const c_void, size: u32) -> i32;
}

#[cfg(windows)]
#[link(name = "user32")]
unsafe extern "system" {
    fn GetAncestor(hwnd: isize, flags: u32) -> isize;
}

/// Force the DWM caption bar to dark (#1e1e1e) so the residual
/// non-client area left by `decorations(false)` is invisible.
#[cfg(windows)]
pub fn force_dark_caption(win: &tauri::WebviewWindow) {
    use raw_window_handle::HasWindowHandle;

    let Ok(handle) = win.window_handle() else { return };
    let raw_window_handle::RawWindowHandle::Win32(h) = handle.as_raw() else {
        return;
    };
    // The raw_window_handle may return the inner WebView2 HWND.
    // GetAncestor(GA_ROOT) gives us the top-level native window that
    // DWM actually paints the caption on.
    const GA_ROOT: u32 = 2;
    let inner = h.hwnd.get() as isize;
    let hwnd = unsafe { GetAncestor(inner, GA_ROOT) };
    let hwnd = if hwnd != 0 { hwnd } else { inner };

    const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
    const DWMWA_CAPTION_COLOR: u32 = 35;
    let dark_mode: u32 = 1;
    // COLORREF 0x00BBGGRR — #1e1e1e → R=G=B=0x1E
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

#[cfg(not(windows))]
pub fn force_dark_caption(_win: &tauri::WebviewWindow) {}
