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
    fn SetWindowLongPtrW(hwnd: isize, index: i32, new: isize) -> isize;
    fn GetWindowLongPtrW(hwnd: isize, index: i32) -> isize;
    fn SetWindowPos(
        hwnd: isize,
        after: isize,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        flags: u32,
    ) -> i32;
}

#[cfg(windows)]
#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateSolidBrush(color: u32) -> isize;
}

/// Force the DWM caption bar to dark (#1e1e1e) so the residual
/// non-client area left by `decorations(false)` is invisible.
#[cfg(windows)]
pub fn force_dark_caption(win: &tauri::WebviewWindow) {
    use raw_window_handle::HasWindowHandle;

    let Ok(handle) = win.window_handle() else {
        tracing::warn!("dwm: couldn't get window handle");
        return;
    };
    let raw_window_handle::RawWindowHandle::Win32(h) = handle.as_raw() else {
        tracing::warn!("dwm: not a Win32 handle");
        return;
    };

    const GA_ROOT: u32 = 2;
    let inner = h.hwnd.get() as isize;
    let hwnd = unsafe { GetAncestor(inner, GA_ROOT) };
    let hwnd = if hwnd != 0 { hwnd } else { inner };
    tracing::info!("dwm: inner={inner:#x} root={hwnd:#x}");

    const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
    const DWMWA_CAPTION_COLOR: u32 = 35;
    let dark_mode: u32 = 1;
    let caption_color: u32 = 0x001E1E1E;

    unsafe {
        let r1 = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &raw const dark_mode as *const c_void,
            4,
        );
        let r2 = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_COLOR,
            &raw const caption_color as *const c_void,
            4,
        );
        tracing::info!("dwm: dark_mode={r1:#x} caption_color={r2:#x}");

        // Also set the window class background brush to dark
        const GCL_HBRBACKGROUND: i32 = -10;
        let brush = CreateSolidBrush(0x001E1E1E);
        if brush != 0 {
            SetWindowLongPtrW(hwnd, GCL_HBRBACKGROUND, brush);
        }

        // Force frame recalculation
        const SWP_FRAMECHANGED: u32 = 0x0020;
        const SWP_NOMOVE: u32 = 0x0002;
        const SWP_NOSIZE: u32 = 0x0001;
        const SWP_NOZORDER: u32 = 0x0004;
        SetWindowPos(
            hwnd,
            0,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER,
        );
    }
}

#[cfg(not(windows))]
pub fn force_dark_caption(_win: &tauri::WebviewWindow) {}
