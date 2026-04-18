//! Centralized shell behavior: surface transitions, window positioning,
//! and helpers shared across tray, shortcut, and single-instance entry points.

use std::sync::{LazyLock, Mutex};

use tauri::{AppHandle, Manager, WebviewWindow};

use crate::events;
use crate::proof_harness;
use crate::state::AppState;
use crate::surface::{SurfaceMode, SurfaceTransition, WindowProperties};
use crate::surface_target::SurfaceTarget;
use crate::window_positioner::{self, PanelSize, Rect};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellTransitionRequest {
    pub mode: SurfaceMode,
    pub target: SurfaceTarget,
    pub position: Option<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SurfaceSnapshot {
    mode: SurfaceMode,
    target: SurfaceTarget,
}

enum TransitionResolution {
    ModeChange {
        transition: SurfaceTransition,
        target: SurfaceTarget,
    },
    SameModeRetarget {
        mode: SurfaceMode,
        target: SurfaceTarget,
    },
    SameModeReopen {
        mode: SurfaceMode,
        target: SurfaceTarget,
    },
    Noop {
        mode: SurfaceMode,
    },
}

struct HideToTrayPlan {
    previous: SurfaceSnapshot,
    transition: Option<SurfaceTransition>,
    target: SurfaceTarget,
}

static SHELL_TRANSITION_SERIAL: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

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

pub fn transition_to_target(
    app: &AppHandle,
    mode: SurfaceMode,
    target: SurfaceTarget,
    position: Option<(i32, i32)>,
) -> Result<SurfaceMode, String> {
    apply_transition_request_with_strategy(
        app,
        ShellTransitionRequest {
            mode,
            target,
            position,
        },
        false,
    )
}

pub fn reopen_to_target(
    app: &AppHandle,
    mode: SurfaceMode,
    target: SurfaceTarget,
    position: Option<(i32, i32)>,
) -> Result<SurfaceMode, String> {
    apply_transition_request_with_strategy(
        app,
        ShellTransitionRequest {
            mode,
            target,
            position,
        },
        true,
    )
}

fn apply_transition_request_with_strategy(
    app: &AppHandle,
    request: ShellTransitionRequest,
    force_same_mode_apply: bool,
) -> Result<SurfaceMode, String> {
    apply_transition_request(app, request, force_same_mode_apply)
}

pub fn hide_to_tray(app: &AppHandle) -> Result<SurfaceMode, String> {
    hide_to_tray_if_current(app, |_| true).map(|mode| mode.unwrap_or(SurfaceMode::Hidden))
}

pub fn hide_to_tray_if_current<P>(
    app: &AppHandle,
    is_eligible: P,
) -> Result<Option<SurfaceMode>, String>
where
    P: FnOnce(SurfaceMode) -> bool,
{
    let _transition_guard = SHELL_TRANSITION_SERIAL.lock().unwrap();
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window unavailable".to_string())?;
    let st = app
        .try_state::<Mutex<AppState>>()
        .ok_or_else(|| "app state unavailable".to_string())?;
    let plan = {
        let mut guard = st.lock().unwrap();
        prepare_hide_to_tray_if_current(&mut guard, is_eligible)
    };

    let Some(plan) = plan else {
        return Ok(None);
    };

    if let Some(transition) = plan.transition {
        apply_transition(app, &window, &transition, &plan.previous, plan.target, None).map(Some)
    } else {
        let _ = window.hide();
        Ok(Some(SurfaceMode::Hidden))
    }
}

#[allow(dead_code)]
pub fn hide_to_tray_state(state: &mut AppState) {
    let _ = prepare_hide_to_tray_if_current(state, |_| true);
}

fn prepare_hide_to_tray_if_current<P>(
    state: &mut AppState,
    is_eligible: P,
) -> Option<HideToTrayPlan>
where
    P: FnOnce(SurfaceMode) -> bool,
{
    let current = state.surface_machine.current();
    if !is_eligible(current) {
        return None;
    }

    let previous = current_surface_snapshot(state);
    let transition = state.hide_surface();
    Some(HideToTrayPlan {
        previous,
        transition,
        target: state.current_target.clone(),
    })
}

fn apply_transition_request(
    app: &AppHandle,
    request: ShellTransitionRequest,
    force_same_mode_apply: bool,
) -> Result<SurfaceMode, String> {
    let _transition_guard = SHELL_TRANSITION_SERIAL.lock().unwrap();
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window unavailable".to_string())?;
    let st = app
        .try_state::<Mutex<AppState>>()
        .ok_or_else(|| "app state unavailable".to_string())?;

    let (previous, resolution) = {
        let guard = st.lock().unwrap();
        let previous = current_surface_snapshot(&guard);
        let resolution = resolve_transition_request(&guard, &request, force_same_mode_apply);
        (previous, resolution)
    };
    let position = resolve_transition_position(
        request.position,
        &resolution,
        should_synthesize_default_position(&resolution),
        || default_surface_position(app, request.mode),
    )
    .or_else(|| preserved_visible_mode_change_position(&window, &resolution));

    match resolution {
        TransitionResolution::ModeChange { transition, target } => {
            apply_transition(app, &window, &transition, &previous, target, position)
        }
        TransitionResolution::SameModeRetarget { mode, target } => {
            apply_same_mode_target_update(app, &window, mode, target, position)
        }
        TransitionResolution::SameModeReopen { mode, target } => {
            let transition = SurfaceTransition {
                from: mode,
                to: mode,
                properties: mode.window_properties(),
            };
            apply_transition(app, &window, &transition, &previous, target, position)
        }
        TransitionResolution::Noop { mode } => Ok(mode),
    }
}

fn resolve_transition_request(
    state: &AppState,
    request: &ShellTransitionRequest,
    force_same_mode_apply: bool,
) -> TransitionResolution {
    let mode = state.surface_machine.current();
    let target = AppState::resolved_target_for_mode(request.mode, Some(request.target.clone()));

    if mode != request.mode {
        TransitionResolution::ModeChange {
            transition: SurfaceTransition {
                from: mode,
                to: request.mode,
                properties: request.mode.window_properties(),
            },
            target,
        }
    } else if state.current_target != target {
        TransitionResolution::SameModeRetarget { mode, target }
    } else if force_same_mode_apply {
        TransitionResolution::SameModeReopen { mode, target }
    } else {
        TransitionResolution::Noop { mode }
    }
}

fn current_surface_snapshot(state: &AppState) -> SurfaceSnapshot {
    SurfaceSnapshot {
        mode: state.surface_machine.current(),
        target: state.current_target.clone(),
    }
}

fn resolve_transition_position<F>(
    requested: Option<(i32, i32)>,
    resolution: &TransitionResolution,
    synthesize_default: bool,
    fallback: F,
) -> Option<(i32, i32)>
where
    F: FnOnce() -> Option<(i32, i32)>,
{
    if requested.is_some() {
        return requested;
    }

    match resolution {
        TransitionResolution::ModeChange { .. } | TransitionResolution::SameModeReopen { .. }
            if synthesize_default =>
        {
            fallback()
        }
        TransitionResolution::ModeChange { .. }
        | TransitionResolution::SameModeReopen { .. }
        | TransitionResolution::SameModeRetarget { .. }
        | TransitionResolution::Noop { .. } => None,
    }
}

fn reclamp_preserved_visible_position(
    current_top_left: (i32, i32),
    monitor_rect: &Rect,
    destination_mode: SurfaceMode,
    scale_factor: f64,
) -> (i32, i32) {
    window_positioner::clamp_position_to_work_area(
        current_top_left.0,
        current_top_left.1,
        monitor_rect,
        &surface_panel_size(destination_mode),
        scale_factor,
    )
}

fn monitor_for_preserved_visible_position(
    monitors: &[(Rect, f64)],
    current_top_left: (i32, i32),
    current_size: Option<(u32, u32)>,
) -> Option<(Rect, f64)> {
    if let Some((rect, scale_factor)) = monitors
        .iter()
        .find(|(rect, _)| point_in_rect(rect, current_top_left.0, current_top_left.1))
    {
        return Some((*rect, *scale_factor));
    }

    if let Some((width, height)) = current_size {
        let center_x = current_top_left.0 + width as i32 / 2;
        let center_y = current_top_left.1 + height as i32 / 2;
        if let Some((rect, scale_factor)) = monitors
            .iter()
            .find(|(rect, _)| point_in_rect(rect, center_x, center_y))
        {
            return Some((*rect, *scale_factor));
        }
    }

    None
}

fn current_monitor_work_area(
    window: &WebviewWindow,
    current_top_left: (i32, i32),
) -> Option<(Rect, f64)> {
    let current_size = window
        .outer_size()
        .ok()
        .map(|size| (size.width, size.height));

    if let Ok(monitors) = window.available_monitors() {
        let monitor_work_areas = monitors
            .iter()
            .map(|monitor| (monitor_work_area_rect(monitor), monitor.scale_factor()))
            .collect::<Vec<_>>();
        if let Some(monitor) = monitor_for_preserved_visible_position(
            &monitor_work_areas,
            current_top_left,
            current_size,
        ) {
            return Some(monitor);
        }
    }

    if let Ok(Some(monitor)) = window.current_monitor() {
        return Some((monitor_work_area_rect(&monitor), monitor.scale_factor()));
    }

    let monitor = window.primary_monitor().ok()??;
    Some((monitor_work_area_rect(&monitor), monitor.scale_factor()))
}

fn preserved_visible_mode_change_position(
    window: &WebviewWindow,
    resolution: &TransitionResolution,
) -> Option<(i32, i32)> {
    let TransitionResolution::ModeChange { transition, .. } = resolution else {
        return None;
    };

    if !transition.from.window_properties().visible || !transition.to.window_properties().visible {
        return None;
    }

    let current = window.outer_position().ok()?;
    let current_top_left = (current.x, current.y);
    let (monitor_rect, scale_factor) = current_monitor_work_area(window, current_top_left)?;

    Some(reclamp_preserved_visible_position(
        current_top_left,
        &monitor_rect,
        transition.to,
        scale_factor,
    ))
}

fn should_synthesize_default_position(resolution: &TransitionResolution) -> bool {
    match resolution {
        TransitionResolution::ModeChange { transition, .. } => {
            transition.from == SurfaceMode::Hidden
        }
        TransitionResolution::SameModeReopen { .. } => true,
        TransitionResolution::SameModeRetarget { .. } | TransitionResolution::Noop { .. } => false,
    }
}

fn restore_surface_snapshot(state: &mut AppState, snapshot: &SurfaceSnapshot) {
    if snapshot.mode == SurfaceMode::Hidden {
        let _ = state.hide_surface();
    } else {
        let _ = state.transition_surface(snapshot.mode, snapshot.target.clone());
    }
}

fn commit_surface_snapshot(app: &AppHandle, snapshot: &SurfaceSnapshot) -> Result<(), String> {
    let st = app
        .try_state::<Mutex<AppState>>()
        .ok_or_else(|| "app state unavailable".to_string())?;
    let mut guard = st.lock().unwrap();
    restore_surface_snapshot(&mut guard, snapshot);
    Ok(())
}

fn hidden_surface_snapshot() -> SurfaceSnapshot {
    SurfaceSnapshot {
        mode: SurfaceMode::Hidden,
        target: SurfaceTarget::Summary,
    }
}

fn restore_recovery_surface<F>(
    recovery: &SurfaceSnapshot,
    mut apply_properties: F,
) -> Result<(), String>
where
    F: FnMut(&WindowProperties) -> Result<(), String>,
{
    apply_properties(&recovery.mode.window_properties())
}

fn recovery_snapshot_for_failed_transition(
    transition: &SurfaceTransition,
    previous: &SurfaceSnapshot,
    requested_target: &SurfaceTarget,
) -> SurfaceSnapshot {
    if previous.mode == SurfaceMode::Hidden {
        SurfaceSnapshot {
            mode: transition.to,
            target: requested_target.clone(),
        }
    } else {
        previous.clone()
    }
}

fn apply_same_mode_target_update(
    app: &AppHandle,
    window: &WebviewWindow,
    mode: SurfaceMode,
    target: SurfaceTarget,
    position: Option<(i32, i32)>,
) -> Result<SurfaceMode, String> {
    if let Some((x, y)) = position {
        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    }
    let _ = window.show();
    let _ = window.set_focus();
    commit_surface_snapshot(
        app,
        &SurfaceSnapshot {
            mode,
            target: target.clone(),
        },
    )?;
    events::emit_surface_mode_changed(app, mode, mode, target);
    proof_harness::sync_after_surface_transition(app);
    Ok(mode)
}

fn apply_transition(
    app: &AppHandle,
    window: &WebviewWindow,
    transition: &SurfaceTransition,
    previous: &SurfaceSnapshot,
    current_target: SurfaceTarget,
    position: Option<(i32, i32)>,
) -> Result<SurfaceMode, String> {
    if let Some((x, y)) = position {
        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    }

    match apply_window_properties(window, &transition.properties) {
        Ok(()) => {
            commit_surface_snapshot(
                app,
                &SurfaceSnapshot {
                    mode: transition.to,
                    target: current_target.clone(),
                },
            )?;
            events::emit_surface_mode_changed(app, transition.from, transition.to, current_target);
            proof_harness::sync_after_surface_transition(app);
            Ok(transition.to)
        }
        Err(err) => {
            let recovery =
                recovery_snapshot_for_failed_transition(transition, previous, &current_target);
            if let Err(recovery_err) = restore_recovery_surface(&recovery, |properties| {
                apply_window_properties(window, properties)
            }) {
                let hidden = hidden_surface_snapshot();
                if let Err(hide_err) = window.hide().map_err(|e| e.to_string()) {
                    tracing::warn!(
                        "shell: failed to restore recovery surface during {:?} -> {:?} after reverting to {:?}: apply error: {}; recovery error: {}; hide error: {}",
                        transition.from,
                        transition.to,
                        recovery.mode,
                        err,
                        recovery_err,
                        hide_err
                    );
                    return Err(format!(
                        "failed to recover shell surface after {:?} -> {:?}: apply error: {}; recovery error: {}; hide error: {}",
                        transition.from, transition.to, err, recovery_err, hide_err
                    ));
                }
                commit_surface_snapshot(app, &hidden)?;
                events::emit_surface_mode_changed(
                    app,
                    transition.from,
                    hidden.mode,
                    hidden.target.clone(),
                );
                proof_harness::sync_after_surface_transition(app);
                tracing::warn!(
                    "shell: failed to restore recovery surface during {:?} -> {:?} after reverting to {:?}; forcing hidden surface: apply error: {}; recovery error: {}",
                    transition.from,
                    transition.to,
                    recovery.mode,
                    err,
                    recovery_err
                );
                return Ok(hidden.mode);
            }
            commit_surface_snapshot(app, &recovery)?;
            events::emit_surface_mode_changed(
                app,
                transition.from,
                recovery.mode,
                recovery.target.clone(),
            );
            proof_harness::sync_after_surface_transition(app);
            tracing::warn!(
                "shell: recovered from window-property failure during {:?} -> {:?} by reapplying {:?}: {}",
                transition.from,
                transition.to,
                recovery.mode,
                err
            );
            Ok(recovery.mode)
        }
    }
}

/// Toggle the tray panel: hide if currently showing, show at `position` otherwise.
pub fn toggle_tray_panel(app: &AppHandle, position: Option<(i32, i32)>) {
    let current = {
        let st = app.state::<Mutex<AppState>>();
        st.lock().unwrap().surface_machine.current()
    };

    if current == SurfaceMode::TrayPanel {
        let _ = hide_to_tray(app);
    } else {
        let _ = transition_to_target(
            app,
            SurfaceMode::TrayPanel,
            SurfaceTarget::Summary,
            position,
        );
    }
}

/// Panel dimensions derived from the tray-panel surface mode properties.
fn surface_panel_size(mode: SurfaceMode) -> PanelSize {
    let props = mode.window_properties();
    PanelSize {
        width: props.width as u32,
        height: props.height as u32,
    }
}

fn tray_panel_size() -> PanelSize {
    surface_panel_size(SurfaceMode::TrayPanel)
}

#[derive(Clone, Copy)]
struct MonitorPlacement {
    bounds: Rect,
    work_area: Rect,
    scale_factor: f64,
}

fn monitor_work_area_rect(monitor: &tauri::Monitor) -> Rect {
    let work_area = monitor.work_area();
    Rect {
        x: work_area.position.x,
        y: work_area.position.y,
        width: work_area.size.width,
        height: work_area.size.height,
    }
}

fn monitor_placement(monitor: &tauri::Monitor) -> MonitorPlacement {
    let position = monitor.position();
    let size = monitor.size();

    MonitorPlacement {
        bounds: Rect {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
        },
        work_area: monitor_work_area_rect(monitor),
        scale_factor: monitor.scale_factor(),
    }
}

fn popout_position(
    anchor_rect: Option<&Rect>,
    monitor: &MonitorPlacement,
    panel_size: &PanelSize,
) -> (i32, i32) {
    window_positioner::calculate_popout_position(
        anchor_rect,
        &monitor.work_area,
        panel_size,
        monitor.scale_factor,
    )
}

fn inferred_tray_anchor_rect(monitor: &MonitorPlacement) -> Rect {
    const SYNTHETIC_TRAY_ICON_SIZE: u32 = 24;
    const SYNTHETIC_TRAY_EDGE_PADDING: i32 = 8;

    let work_right = monitor.work_area.x + monitor.work_area.width as i32;
    let work_bottom = monitor.work_area.y + monitor.work_area.height as i32;
    let bounds_top = monitor.bounds.y;
    let bounds_bottom = monitor.bounds.y + monitor.bounds.height as i32;
    let top_gap = monitor.work_area.y - bounds_top;
    let bottom_gap = bounds_bottom - work_bottom;

    let x = work_right - SYNTHETIC_TRAY_ICON_SIZE as i32 - SYNTHETIC_TRAY_EDGE_PADDING;
    let y = if top_gap > bottom_gap {
        monitor.work_area.y - SYNTHETIC_TRAY_ICON_SIZE as i32 - SYNTHETIC_TRAY_EDGE_PADDING
    } else {
        work_bottom + SYNTHETIC_TRAY_EDGE_PADDING
    };

    Rect {
        x,
        y,
        width: SYNTHETIC_TRAY_ICON_SIZE,
        height: SYNTHETIC_TRAY_ICON_SIZE,
    }
}

fn inferred_tray_panel_position_for_monitor(monitor: &MonitorPlacement) -> (i32, i32) {
    window_positioner::calculate_panel_position(
        &inferred_tray_anchor_rect(monitor),
        &monitor.work_area,
        &tray_panel_size(),
        monitor.scale_factor,
    )
}

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

    Some(inferred_tray_panel_position_for_monitor(&monitor))
}

fn tray_anchor_rect(anchor: crate::state::TrayAnchor) -> Rect {
    Rect {
        x: anchor.x,
        y: anchor.y,
        width: anchor.width,
        height: anchor.height,
    }
}

fn monitor_placement_for_anchor(
    monitors: &[MonitorPlacement],
    anchor: crate::state::TrayAnchor,
) -> Option<MonitorPlacement> {
    let anchor_cx = anchor.x + anchor.width as i32 / 2;
    let anchor_cy = anchor.y + anchor.height as i32 / 2;

    monitor_placement_containing_point(monitors, anchor_cx, anchor_cy)
}

fn monitor_placement_containing_point(
    monitors: &[MonitorPlacement],
    x: i32,
    y: i32,
) -> Option<MonitorPlacement> {
    monitors
        .iter()
        .find(|monitor| point_in_rect(&monitor.bounds, x, y))
        .copied()
}

fn monitor_for_anchor(
    monitors: &[tauri::Monitor],
    anchor: crate::state::TrayAnchor,
) -> Option<&tauri::Monitor> {
    let anchor_cx = anchor.x + anchor.width as i32 / 2;
    let anchor_cy = anchor.y + anchor.height as i32 / 2;

    monitor_containing_point(monitors, anchor_cx, anchor_cy)
}

fn monitor_containing_point(
    monitors: &[tauri::Monitor],
    x: i32,
    y: i32,
) -> Option<&tauri::Monitor> {
    monitors.iter().find(|monitor| {
        let pos = monitor.position();
        let size = monitor.size();
        point_in_rect(
            &Rect {
                x: pos.x,
                y: pos.y,
                width: size.width,
                height: size.height,
            },
            x,
            y,
        )
    })
}

fn point_in_rect(rect: &Rect, x: i32, y: i32) -> bool {
    x >= rect.x && x < rect.x + rect.width as i32 && y >= rect.y && y < rect.y + rect.height as i32
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

fn visible_surface_position_for_mode_with_fallbacks(
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

    // Pick the monitor that contains the stored top-left; otherwise fall back
    // to the primary monitor's work area so the window remains reachable.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hide_to_tray_resets_hidden_target_to_summary() {
        let mut state = AppState::new();
        state.current_target = SurfaceTarget::Settings {
            tab: "about".into(),
        };

        hide_to_tray_state(&mut state);

        assert_eq!(state.surface_machine.current(), SurfaceMode::Hidden);
        assert_eq!(state.current_target, SurfaceTarget::Summary);
    }

    #[test]
    fn conditional_hide_to_tray_updates_matching_surface() {
        let mut state = AppState::new();
        state.transition_surface(SurfaceMode::TrayPanel, SurfaceTarget::Summary);

        let plan =
            prepare_hide_to_tray_if_current(&mut state, |mode| mode == SurfaceMode::TrayPanel)
                .expect("tray panel should be eligible");

        assert_eq!(plan.previous.mode, SurfaceMode::TrayPanel);
        assert_eq!(state.surface_machine.current(), SurfaceMode::Hidden);
        assert_eq!(state.current_target, SurfaceTarget::Summary);
        assert_eq!(plan.target, SurfaceTarget::Summary);
    }

    #[test]
    fn conditional_hide_to_tray_leaves_non_matching_surface_alone() {
        let mut state = AppState::new();
        state.transition_surface(SurfaceMode::PopOut, SurfaceTarget::Dashboard);

        let plan =
            prepare_hide_to_tray_if_current(&mut state, |mode| mode == SurfaceMode::TrayPanel);

        assert!(plan.is_none());
        assert_eq!(state.surface_machine.current(), SurfaceMode::PopOut);
        assert_eq!(state.current_target, SurfaceTarget::Dashboard);
    }

    #[test]
    fn same_mode_about_request_resolves_as_retarget() {
        let mut state = AppState::new();
        state.transition_surface(
            SurfaceMode::Settings,
            SurfaceTarget::Settings {
                tab: "general".into(),
            },
        );

        let resolution = resolve_transition_request(
            &state,
            &ShellTransitionRequest {
                mode: SurfaceMode::Settings,
                target: SurfaceTarget::Settings {
                    tab: "about".into(),
                },
                position: None,
            },
            false,
        );

        match resolution {
            TransitionResolution::SameModeRetarget { mode, target } => {
                assert_eq!(mode, SurfaceMode::Settings);
                assert_eq!(
                    target,
                    SurfaceTarget::Settings {
                        tab: "about".into()
                    }
                );
            }
            _ => panic!("expected same-mode retarget"),
        }
    }

    #[test]
    fn same_mode_provider_request_resolves_as_retarget() {
        let mut state = AppState::new();
        state.transition_surface(SurfaceMode::PopOut, SurfaceTarget::Dashboard);

        let resolution = resolve_transition_request(
            &state,
            &ShellTransitionRequest {
                mode: SurfaceMode::PopOut,
                target: SurfaceTarget::Provider {
                    provider_id: "codex".into(),
                },
                position: None,
            },
            false,
        );

        match resolution {
            TransitionResolution::SameModeRetarget { mode, target } => {
                assert_eq!(mode, SurfaceMode::PopOut);
                assert_eq!(
                    target,
                    SurfaceTarget::Provider {
                        provider_id: "codex".into()
                    }
                );
            }
            _ => panic!("expected same-mode retarget"),
        }
    }

    #[test]
    fn same_mode_reopen_request_resolves_as_update() {
        let mut state = AppState::new();
        state.transition_surface(SurfaceMode::TrayPanel, SurfaceTarget::Summary);

        let resolution = resolve_transition_request(
            &state,
            &ShellTransitionRequest {
                mode: SurfaceMode::TrayPanel,
                target: SurfaceTarget::Summary,
                position: Some((10, 20)),
            },
            true,
        );

        match resolution {
            TransitionResolution::SameModeReopen { mode, target } => {
                assert_eq!(mode, SurfaceMode::TrayPanel);
                assert_eq!(target, SurfaceTarget::Summary);
            }
            _ => panic!("expected same-mode reopen update"),
        }
    }

    #[test]
    fn same_mode_retarget_skips_default_position_synthesis() {
        let resolution = TransitionResolution::SameModeRetarget {
            mode: SurfaceMode::Settings,
            target: SurfaceTarget::Settings {
                tab: "about".into(),
            },
        };
        let mut fallback_called = false;

        let position = resolve_transition_position(None, &resolution, false, || {
            fallback_called = true;
            Some((10, 20))
        });

        assert_eq!(position, None);
        assert!(
            !fallback_called,
            "same-mode retarget should not request a default position"
        );
    }

    #[test]
    fn same_mode_retarget_preserves_explicit_position() {
        let resolution = TransitionResolution::SameModeRetarget {
            mode: SurfaceMode::PopOut,
            target: SurfaceTarget::Provider {
                provider_id: "codex".into(),
            },
        };

        let position = resolve_transition_position(Some((10, 20)), &resolution, false, || {
            panic!("explicit same-mode retarget position should be used directly")
        });

        assert_eq!(position, Some((10, 20)));
    }

    #[test]
    fn same_mode_reopen_still_uses_default_position() {
        let resolution = TransitionResolution::SameModeReopen {
            mode: SurfaceMode::TrayPanel,
            target: SurfaceTarget::Summary,
        };
        let mut fallback_called = false;

        let position = resolve_transition_position(None, &resolution, true, || {
            fallback_called = true;
            Some((42, 24))
        });

        assert_eq!(position, Some((42, 24)));
        assert!(
            fallback_called,
            "same-mode reopen should still synthesize a default position"
        );
    }

    #[test]
    fn visible_mode_change_skips_default_position_synthesis() {
        let resolution = TransitionResolution::ModeChange {
            transition: SurfaceTransition {
                from: SurfaceMode::PopOut,
                to: SurfaceMode::Settings,
                properties: SurfaceMode::Settings.window_properties(),
            },
            target: SurfaceTarget::Settings {
                tab: "about".into(),
            },
        };
        let mut fallback_called = false;

        let position = resolve_transition_position(
            None,
            &resolution,
            should_synthesize_default_position(&resolution),
            || {
                fallback_called = true;
                Some((20, 30))
            },
        );

        assert_eq!(position, None);
        assert!(
            !fallback_called,
            "visible-to-visible mode changes should preserve the current window position"
        );
    }

    #[test]
    fn larger_visible_destination_reclamps_preserved_top_left() {
        let current_top_left = (1492, 512);
        let monitor = Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };

        let reclamped = reclamp_preserved_visible_position(
            current_top_left,
            &monitor,
            SurfaceMode::Settings,
            1.0,
        );

        assert_eq!(reclamped, (1416, 492));
    }

    #[test]
    fn preserved_visible_monitor_prefers_top_left_for_straddling_window() {
        let monitors = vec![
            (
                Rect {
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                },
                1.0,
            ),
            (
                Rect {
                    x: 1920,
                    y: 0,
                    width: 1920,
                    height: 1080,
                },
                1.25,
            ),
        ];

        let selected =
            monitor_for_preserved_visible_position(&monitors, (1800, 120), Some((600, 700)))
                .expect("straddling window should resolve from its preserved top-left");

        assert_eq!(selected.0.x, 0);
        assert_eq!(selected.1, 1.0);
    }

    #[test]
    fn visible_surface_position_falls_back_to_current_monitor_without_available_monitors() {
        let current_monitor = MonitorPlacement {
            bounds: Rect {
                x: 1920,
                y: 0,
                width: 1920,
                height: 1080,
            },
            work_area: Rect {
                x: 1920,
                y: 0,
                width: 1920,
                height: 1080,
            },
            scale_factor: 1.25,
        };
        let primary_monitor = MonitorPlacement {
            bounds: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            work_area: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            scale_factor: 1.0,
        };
        let anchor = crate::state::TrayAnchor {
            x: 10,
            y: 10,
            width: 16,
            height: 16,
        };

        let position = visible_surface_position_for_mode_with_fallbacks(
            SurfaceMode::PopOut,
            None,
            Some(anchor),
            Some(current_monitor),
            Some(((2000, 120), (600, 700))),
            Some(primary_monitor),
        );

        assert_eq!(
            position,
            Some(window_positioner::calculate_popout_position(
                None,
                &current_monitor.work_area,
                &surface_panel_size(SurfaceMode::PopOut),
                current_monitor.scale_factor,
            ))
        );
    }

    #[test]
    fn visible_surface_position_anchor_lookup_uses_monitor_bounds() {
        let anchor_monitor = MonitorPlacement {
            bounds: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            work_area: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1040,
            },
            scale_factor: 1.0,
        };
        let current_monitor = MonitorPlacement {
            bounds: Rect {
                x: 1920,
                y: 0,
                width: 1920,
                height: 1080,
            },
            work_area: Rect {
                x: 1920,
                y: 0,
                width: 1920,
                height: 1080,
            },
            scale_factor: 1.25,
        };
        let anchor = crate::state::TrayAnchor {
            x: 1800,
            y: 1040,
            width: 24,
            height: 24,
        };

        let position = visible_surface_position_for_mode_with_fallbacks(
            SurfaceMode::PopOut,
            Some(&[anchor_monitor, current_monitor]),
            Some(anchor),
            Some(current_monitor),
            None,
            None,
        );

        assert_eq!(
            position,
            Some(window_positioner::calculate_popout_position(
                Some(&tray_anchor_rect(anchor)),
                &anchor_monitor.work_area,
                &surface_panel_size(SurfaceMode::PopOut),
                anchor_monitor.scale_factor,
            ))
        );
    }

    #[test]
    fn visible_surface_position_settings_surface_uses_tray_anchor_position_when_available() {
        let anchor_monitor = MonitorPlacement {
            bounds: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            work_area: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1040,
            },
            scale_factor: 1.0,
        };
        let current_monitor = MonitorPlacement {
            bounds: Rect {
                x: 1920,
                y: 0,
                width: 1920,
                height: 1080,
            },
            work_area: Rect {
                x: 1920,
                y: 0,
                width: 1920,
                height: 1080,
            },
            scale_factor: 1.25,
        };
        let anchor = crate::state::TrayAnchor {
            x: 1800,
            y: 1040,
            width: 24,
            height: 24,
        };

        let position = visible_surface_position_for_mode_with_fallbacks(
            SurfaceMode::Settings,
            Some(&[anchor_monitor, current_monitor]),
            Some(anchor),
            Some(current_monitor),
            None,
            None,
        );

        assert_eq!(
            position,
            Some(window_positioner::calculate_popout_position(
                Some(&tray_anchor_rect(anchor)),
                &anchor_monitor.work_area,
                &surface_panel_size(SurfaceMode::Settings),
                anchor_monitor.scale_factor,
            ))
        );
    }

    #[test]
    fn inferred_tray_anchor_defaults_to_bottom_right_of_work_area() {
        let monitor = MonitorPlacement {
            bounds: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            work_area: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1040,
            },
            scale_factor: 1.0,
        };

        let anchor = inferred_tray_anchor_rect(&monitor);

        assert_eq!(anchor.x, 1888);
        assert_eq!(anchor.y, 1048);
        assert_eq!(anchor.width, 24);
        assert_eq!(anchor.height, 24);
    }

    #[test]
    fn inferred_tray_anchor_supports_top_taskbar_layouts() {
        let monitor = MonitorPlacement {
            bounds: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            work_area: Rect {
                x: 0,
                y: 40,
                width: 1920,
                height: 1040,
            },
            scale_factor: 1.0,
        };

        let anchor = inferred_tray_anchor_rect(&monitor);

        assert_eq!(anchor.x, 1888);
        assert_eq!(anchor.y, 8);
    }

    #[test]
    fn inferred_tray_panel_position_uses_tray_style_corner_fallback() {
        let monitor = MonitorPlacement {
            bounds: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            work_area: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1040,
            },
            scale_factor: 1.0,
        };

        let position = inferred_tray_panel_position_for_monitor(&monitor);

        assert_eq!(
            position,
            window_positioner::calculate_panel_position(
                &Rect {
                    x: 1888,
                    y: 1048,
                    width: 24,
                    height: 24,
                },
                &monitor.work_area,
                &tray_panel_size(),
                monitor.scale_factor,
            )
        );
    }

    #[test]
    fn hidden_mode_change_still_uses_default_position() {
        let resolution = TransitionResolution::ModeChange {
            transition: SurfaceTransition {
                from: SurfaceMode::Hidden,
                to: SurfaceMode::Settings,
                properties: SurfaceMode::Settings.window_properties(),
            },
            target: SurfaceTarget::Settings {
                tab: "general".into(),
            },
        };
        let mut fallback_called = false;

        let position = resolve_transition_position(
            None,
            &resolution,
            should_synthesize_default_position(&resolution),
            || {
                fallback_called = true;
                Some((64, 48))
            },
        );

        assert_eq!(position, Some((64, 48)));
        assert!(
            fallback_called,
            "hidden opens should still synthesize default placement"
        );
    }

    #[test]
    fn failed_hide_transition_recovers_previous_visible_surface() {
        let previous = SurfaceSnapshot {
            mode: SurfaceMode::PopOut,
            target: SurfaceTarget::Provider {
                provider_id: "codex".into(),
            },
        };
        let transition = SurfaceTransition {
            from: SurfaceMode::PopOut,
            to: SurfaceMode::Hidden,
            properties: SurfaceMode::Hidden.window_properties(),
        };

        let recovery = recovery_snapshot_for_failed_transition(
            &transition,
            &previous,
            &SurfaceTarget::Summary,
        );

        assert_eq!(recovery, previous);
    }

    #[test]
    fn failed_show_transition_from_hidden_keeps_requested_visible_surface() {
        let previous = SurfaceSnapshot {
            mode: SurfaceMode::Hidden,
            target: SurfaceTarget::Summary,
        };
        let transition = SurfaceTransition {
            from: SurfaceMode::Hidden,
            to: SurfaceMode::TrayPanel,
            properties: SurfaceMode::TrayPanel.window_properties(),
        };

        let recovery = recovery_snapshot_for_failed_transition(
            &transition,
            &previous,
            &SurfaceTarget::Summary,
        );

        assert_eq!(
            recovery,
            SurfaceSnapshot {
                mode: SurfaceMode::TrayPanel,
                target: SurfaceTarget::Summary,
            }
        );
    }

    #[test]
    fn restore_surface_snapshot_reverts_mode_and_target() {
        let previous = SurfaceSnapshot {
            mode: SurfaceMode::Settings,
            target: SurfaceTarget::Settings {
                tab: "about".into(),
            },
        };
        let mut state = AppState::new();
        state.hide_surface();

        restore_surface_snapshot(&mut state, &previous);

        assert_eq!(state.surface_machine.current(), SurfaceMode::Settings);
        assert_eq!(state.current_target, previous.target);
    }

    #[test]
    fn visible_recovery_propagates_visibility_errors() {
        let recovery = SurfaceSnapshot {
            mode: SurfaceMode::TrayPanel,
            target: SurfaceTarget::Summary,
        };

        let err = restore_recovery_surface(&recovery, |_| Err("show failed".into()))
            .expect_err("visible recovery should fail when properties are not restored");

        assert_eq!(err, "show failed");
    }

    #[test]
    fn hidden_recovery_reapplies_hidden_properties() {
        let recovery = SurfaceSnapshot {
            mode: SurfaceMode::Hidden,
            target: SurfaceTarget::Summary,
        };

        let mut applied_hidden = false;
        let restored = restore_recovery_surface(&recovery, |properties| {
            applied_hidden = !properties.visible;
            Ok(())
        });

        assert!(restored.is_ok());
        assert!(applied_hidden);
    }

    #[test]
    fn hidden_surface_snapshot_matches_non_visible_shell_state() {
        assert_eq!(
            hidden_surface_snapshot(),
            SurfaceSnapshot {
                mode: SurfaceMode::Hidden,
                target: SurfaceTarget::Summary,
            }
        );
    }
}
