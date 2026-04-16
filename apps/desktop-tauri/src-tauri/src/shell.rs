//! Centralized shell behavior: surface transitions, window positioning,
//! and helpers shared across tray, shortcut, and single-instance entry points.

use std::sync::{LazyLock, Mutex};

use tauri::{AppHandle, Manager, WebviewWindow};

use crate::events;
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
    let transition = state.transition_surface(SurfaceMode::Hidden, Some(SurfaceTarget::Summary));
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

    match resolution {
        TransitionResolution::ModeChange { transition, target } => apply_transition(
            app,
            &window,
            &transition,
            &previous,
            target,
            request.position,
        ),
        TransitionResolution::SameModeRetarget { mode, target } => {
            apply_same_mode_target_update(app, &window, mode, target, request.position)
        }
        TransitionResolution::SameModeReopen { mode, target } => {
            let transition = SurfaceTransition {
                from: mode,
                to: mode,
                properties: mode.window_properties(),
            };
            apply_transition(
                app,
                &window,
                &transition,
                &previous,
                target,
                request.position,
            )
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

fn restore_surface_snapshot(state: &mut AppState, snapshot: &SurfaceSnapshot) {
    let _ = state.transition_surface(snapshot.mode, Some(snapshot.target.clone()));
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

fn restore_recovery_visibility<F>(
    recovery: &SurfaceSnapshot,
    mut show_and_focus: F,
) -> Result<(), String>
where
    F: FnMut() -> Result<(), String>,
{
    if recovery.mode.window_properties().visible {
        show_and_focus()?;
    }

    Ok(())
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
            Ok(transition.to)
        }
        Err(err) => {
            let recovery =
                recovery_snapshot_for_failed_transition(transition, previous, &current_target);
            if let Err(recovery_err) = restore_recovery_visibility(&recovery, || {
                window.show().map_err(|e| e.to_string())?;
                window.set_focus().map_err(|e| e.to_string())?;
                Ok(())
            }) {
                let hidden = hidden_surface_snapshot();
                if let Err(hide_err) = window.hide().map_err(|e| e.to_string()) {
                    tracing::warn!(
                        "shell: failed to re-establish recovery visibility during {:?} -> {:?} after restoring {:?}: apply error: {}; recovery error: {}; hide error: {}",
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
                tracing::warn!(
                    "shell: failed to re-establish recovery visibility during {:?} -> {:?} after restoring {:?}; forcing hidden surface: apply error: {}; recovery error: {}",
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
            tracing::warn!(
                "shell: recovered from window-property failure during {:?} -> {:?} by restoring {:?}: {}",
                transition.from,
                transition.to,
                recovery.mode,
                err
            );
            Ok(recovery.mode)
        }
    }
}

/// Perform a surface transition, apply window properties, and emit the event.
/// Optionally repositions the window at `position` (physical pixels) before showing.
#[allow(dead_code)]
pub fn transition_surface(
    app: &AppHandle,
    mode: SurfaceMode,
    target: Option<SurfaceTarget>,
    position: Option<(i32, i32)>,
) {
    let target = target.unwrap_or_else(|| SurfaceTarget::default_for_mode(mode));
    let _ = transition_to_target(app, mode, target, position);
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
        state.transition_surface(SurfaceMode::TrayPanel, Some(SurfaceTarget::Summary));

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
        state.transition_surface(SurfaceMode::PopOut, Some(SurfaceTarget::Dashboard));

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
            Some(SurfaceTarget::Settings {
                tab: "general".into(),
            }),
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
        state.transition_surface(SurfaceMode::PopOut, Some(SurfaceTarget::Dashboard));

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
        state.transition_surface(SurfaceMode::TrayPanel, Some(SurfaceTarget::Summary));

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
        state.transition_surface(SurfaceMode::Hidden, Some(SurfaceTarget::Summary));

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

        let err = restore_recovery_visibility(&recovery, || Err("show failed".into()))
            .expect_err("visible recovery should fail when visibility is not restored");

        assert_eq!(err, "show failed");
    }

    #[test]
    fn hidden_recovery_does_not_require_visibility_restore() {
        let recovery = SurfaceSnapshot {
            mode: SurfaceMode::Hidden,
            target: SurfaceTarget::Summary,
        };

        let restored = restore_recovery_visibility(&recovery, || Err("should not run".into()));

        assert!(restored.is_ok());
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
