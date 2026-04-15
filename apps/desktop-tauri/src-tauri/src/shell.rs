//! Centralized shell behavior: surface transitions, window positioning,
//! and helpers shared across tray, shortcut, and single-instance entry points.

use std::sync::Mutex;

use tauri::{AppHandle, Manager, WebviewWindow};

use crate::events;
use crate::state::AppState;
use crate::surface::{SurfaceMode, WindowProperties};
use crate::window_positioner::{self, PanelSize, Rect};

/// Apply the window properties dictated by a surface mode.
pub fn apply_window_properties(
    window: &WebviewWindow,
    props: &WindowProperties,
) -> Result<(), String> {
    let map_err = |e: tauri::Error| e.to_string();

    window.set_decorations(props.decorations).map_err(map_err)?;
    window.set_resizable(props.resizable).map_err(map_err)?;
    window
        .set_always_on_top(props.always_on_top)
        .map_err(map_err)?;

    if props.visible {
        let size = tauri::LogicalSize::new(props.width, props.height);
        window.set_size(size).map_err(map_err)?;

        if let (Some(min_w), Some(min_h)) = (props.min_width, props.min_height) {
            window
                .set_min_size(Some(tauri::LogicalSize::new(min_w, min_h)))
                .map_err(map_err)?;
        } else {
            window
                .set_min_size::<tauri::LogicalSize<f64>>(None)
                .map_err(map_err)?;
        }

        window.show().map_err(map_err)?;
        window.set_focus().map_err(map_err)?;
    } else {
        window.hide().map_err(map_err)?;
    }

    Ok(())
}

/// Perform a surface transition, apply window properties, and emit the event.
/// Optionally repositions the window at `position` (physical pixels) before showing.
pub fn transition_surface(app: &AppHandle, mode: SurfaceMode, position: Option<(i32, i32)>) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Some(st) = app.try_state::<Mutex<AppState>>() else {
        return;
    };

    let transition = {
        let mut guard = st.lock().unwrap();
        guard.transition_surface(mode, None)
    };

    if let Some(t) = transition {
        // Position before showing so the window doesn't flash at the old location.
        if let Some((x, y)) = position {
            let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
        }
        let _ = apply_window_properties(&window, &t.properties);
        events::emit_surface_mode_changed(app, t.from, t.to);
    }
}

/// Toggle the tray panel: hide if currently showing, show at `position` otherwise.
pub fn toggle_tray_panel(app: &AppHandle, position: Option<(i32, i32)>) {
    let current = {
        let st = app.state::<Mutex<AppState>>();
        st.lock().unwrap().surface_machine.current()
    };

    if current == SurfaceMode::TrayPanel {
        transition_surface(app, SurfaceMode::Hidden, None);
    } else {
        transition_surface(app, SurfaceMode::TrayPanel, position);
    }
}

/// Panel dimensions derived from the tray-panel surface mode properties.
fn tray_panel_size() -> PanelSize {
    let props = SurfaceMode::TrayPanel.window_properties();
    PanelSize {
        width: props.width as u32,
        height: props.height as u32,
    }
}

/// Calculate panel position anchored to the saved tray icon rectangle.
pub fn tray_panel_position(app: &AppHandle) -> Option<(i32, i32)> {
    let anchor = {
        let st = app.state::<Mutex<AppState>>();
        st.lock().unwrap().tray_anchor
    }?;

    let window = app.get_webview_window("main")?;
    let monitors = window.available_monitors().ok()?;

    // Find the monitor whose bounds contain the tray icon.
    let monitor = monitors.iter().find(|m| {
        let p = m.position();
        let s = m.size();
        anchor.x >= p.x
            && anchor.x < p.x + s.width as i32
            && anchor.y >= p.y
            && anchor.y < p.y + s.height as i32
    })?;

    let pos = monitor.position();
    let size = monitor.size();
    let scale = monitor.scale_factor();

    let icon_rect = Rect {
        x: anchor.x,
        y: anchor.y,
        width: anchor.width,
        height: anchor.height,
    };
    let monitor_rect = Rect {
        x: pos.x,
        y: pos.y,
        width: size.width,
        height: size.height,
    };

    Some(window_positioner::calculate_panel_position(
        &icon_rect,
        &monitor_rect,
        &tray_panel_size(),
        scale,
    ))
}

/// Calculate panel position for shortcut/second-instance opens (22 % left, centred).
pub fn shortcut_panel_position(app: &AppHandle) -> Option<(i32, i32)> {
    let window = app.get_webview_window("main")?;
    let monitor = window.primary_monitor().ok()??;
    let pos = monitor.position();
    let size = monitor.size();
    let scale = monitor.scale_factor();

    let monitor_rect = Rect {
        x: pos.x,
        y: pos.y,
        width: size.width,
        height: size.height,
    };

    Some(window_positioner::calculate_shortcut_position(
        &monitor_rect,
        &tray_panel_size(),
        scale,
    ))
}
