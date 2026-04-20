//! Public position API: default placement, tray/shortcut/inferred panel
//! positions, and remembered Settings geometry.

use std::sync::Mutex;

use tauri::{AppHandle, Manager};

use crate::state::AppState;
use crate::surface::SurfaceMode;
use crate::window_positioner;

use super::geometry::{
    MonitorPlacement, monitor_for_anchor, monitor_placement, monitor_placement_containing_point,
    monitor_placement_for_anchor, monitor_work_area_rect, popout_position, surface_panel_size,
    tray_anchor_rect, tray_panel_size,
};

pub fn inferred_tray_panel_position(app: &AppHandle) -> Option<(i32, i32)> {
    let window = app.get_webview_window("main")?;
    let monitor = window
        .primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| monitor_placement(&monitor))
        .or_else(|| {
            window
                .current_monitor()
                .ok()
                .flatten()
                .map(|monitor| monitor_placement(&monitor))
        })?;

    Some(super::geometry::inferred_tray_panel_position_for_monitor(
        &monitor,
    ))
}

fn current_tray_anchor(app: &AppHandle) -> Option<crate::state::TrayAnchor> {
    let st = app.try_state::<Mutex<AppState>>()?;
    st.lock().ok()?.tray_anchor
}

fn visible_surface_position_for_mode(app: &AppHandle, mode: SurfaceMode) -> Option<(i32, i32)> {
    let window = app.get_webview_window("main")?;
    let monitor_placements = window
        .available_monitors()
        .ok()
        .map(|monitors| monitors.iter().map(monitor_placement).collect::<Vec<_>>());
    let current_monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .map(|monitor| monitor_placement(&monitor));
    let current_window_bounds = match (window.outer_position(), window.outer_size()) {
        (Ok(position), Ok(size)) => Some(((position.x, position.y), (size.width, size.height))),
        _ => None,
    };
    let primary_monitor = window
        .primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| monitor_placement(&monitor));

    visible_surface_position_for_mode_with_fallbacks(
        mode,
        monitor_placements.as_deref(),
        current_tray_anchor(app),
        current_monitor,
        current_window_bounds,
        primary_monitor,
    )
}

pub(super) fn visible_surface_position_for_mode_with_fallbacks(
    mode: SurfaceMode,
    monitor_placements: Option<&[MonitorPlacement]>,
    tray_anchor: Option<crate::state::TrayAnchor>,
    current_monitor: Option<MonitorPlacement>,
    current_window_bounds: Option<((i32, i32), (u32, u32))>,
    primary_monitor: Option<MonitorPlacement>,
) -> Option<(i32, i32)> {
    let panel_size = surface_panel_size(mode);

    if let Some(anchor) = tray_anchor
        && let Some(monitors) = monitor_placements
        && let Some(monitor) = monitor_placement_for_anchor(monitors, anchor)
    {
        return Some(popout_position(
            Some(&tray_anchor_rect(anchor)),
            &monitor,
            &panel_size,
        ));
    }

    if let Some(monitor) = current_monitor {
        return Some(popout_position(None, &monitor, &panel_size));
    }

    if let Some(monitors) = monitor_placements
        && let Some((current_top_left, current_size)) = current_window_bounds
        && let Some(monitor) = monitor_placement_containing_point(
            monitors,
            current_top_left.0 + current_size.0 as i32 / 2,
            current_top_left.1 + current_size.1 as i32 / 2,
        )
    {
        return Some(popout_position(None, &monitor, &panel_size));
    }

    let monitor = primary_monitor?;
    Some(popout_position(None, &monitor, &panel_size))
}

pub fn default_surface_position(app: &AppHandle, mode: SurfaceMode) -> Option<(i32, i32)> {
    match mode {
        SurfaceMode::Hidden => None,
        SurfaceMode::TrayPanel => tray_panel_position(app)
            .or_else(|| inferred_tray_panel_position(app))
            .or_else(|| shortcut_panel_position(app)),
        SurfaceMode::PopOut => visible_surface_position_for_mode(app, mode),
        SurfaceMode::Settings => remembered_settings_position(app)
            .or_else(|| visible_surface_position_for_mode(app, mode)),
    }
}

/// Load persisted Settings geometry and clamp it into the current monitor's
/// work area so a monitor layout change can't leave the window off-screen.
fn remembered_settings_position(app: &AppHandle) -> Option<(i32, i32)> {
    let stored = crate::geometry_store::load(SurfaceMode::Settings)?;
    let window = app.get_webview_window("main")?;
    let monitors = window.available_monitors().ok()?;

    let placement = monitors
        .iter()
        .find(|m| {
            let wa = m.work_area();
            let x = wa.position.x;
            let y = wa.position.y;
            let w = wa.size.width as i32;
            let h = wa.size.height as i32;
            stored.x >= x && stored.x < x + w && stored.y >= y && stored.y < y + h
        })
        .map(monitor_placement)
        .or_else(|| {
            window
                .primary_monitor()
                .ok()
                .flatten()
                .map(|m| monitor_placement(&m))
        })?;

    let panel_size = surface_panel_size(SurfaceMode::Settings);
    Some(window_positioner::clamp_position_to_work_area(
        stored.x,
        stored.y,
        &placement.work_area,
        &panel_size,
        placement.scale_factor,
    ))
}

/// Persist the current position (and size, when resizable) of the main window
/// when it is hosting the Settings surface. Called from the Tauri window-event
/// pump so user drags are captured even without an explicit close.
pub fn remember_current_geometry_if_settings(window: &tauri::Window) {
    let app = window.app_handle();
    let Some(st) = app.try_state::<Mutex<AppState>>() else {
        return;
    };
    let current_mode = {
        let guard = st.lock().unwrap();
        guard.surface_machine.current()
    };
    if !crate::geometry_store::should_remember(current_mode) {
        return;
    }

    let Ok(pos) = window.outer_position() else {
        return;
    };
    let size = window.outer_size().ok();
    crate::geometry_store::save(
        current_mode,
        crate::geometry_store::StoredGeometry {
            x: pos.x,
            y: pos.y,
            width: size.map(|s| s.width),
            height: size.map(|s| s.height),
        },
    );
}

/// Calculate panel position anchored to the saved tray icon rectangle.
pub fn tray_panel_position(app: &AppHandle) -> Option<(i32, i32)> {
    let anchor = current_tray_anchor(app)?;

    let window = app.get_webview_window("main")?;
    let monitors = window.available_monitors().ok()?;

    let monitor = monitor_for_anchor(&monitors, anchor)?;
    let scale = monitor.scale_factor();

    Some(window_positioner::calculate_panel_position(
        &tray_anchor_rect(anchor),
        &monitor_work_area_rect(monitor),
        &tray_panel_size(),
        scale,
    ))
}

/// Calculate panel position for shortcut/second-instance opens (22 % left, centred).
pub fn shortcut_panel_position(app: &AppHandle) -> Option<(i32, i32)> {
    let window = app.get_webview_window("main")?;
    let monitor = window.primary_monitor().ok()??;
    let scale = monitor.scale_factor();

    Some(window_positioner::calculate_shortcut_position(
        &monitor_work_area_rect(&monitor),
        &tray_panel_size(),
        scale,
    ))
}
