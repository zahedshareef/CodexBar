//! Centralized shell behavior: surface transitions, window positioning,
//! and helpers shared across tray, shortcut, and single-instance entry points.

use std::sync::Mutex;

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
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window unavailable".to_string())?;
    let st = app
        .try_state::<Mutex<AppState>>()
        .ok_or_else(|| "app state unavailable".to_string())?;
    let (transition, current_target) = {
        let mut guard = st.lock().unwrap();
        let transition =
            guard.transition_surface(SurfaceMode::Hidden, Some(SurfaceTarget::Summary));
        guard.current_target = SurfaceTarget::Summary;
        (transition, guard.current_target.clone())
    };

    if let Some(transition) = transition {
        apply_transition(app, &window, &transition, current_target, None)
    } else {
        let _ = window.hide();
        Ok(SurfaceMode::Hidden)
    }
}

#[allow(dead_code)]
pub fn hide_to_tray_state(state: &mut AppState) {
    let _ = state.transition_surface(SurfaceMode::Hidden, Some(SurfaceTarget::Summary));
    state.current_target = SurfaceTarget::Summary;
}

fn apply_transition_request(
    app: &AppHandle,
    request: ShellTransitionRequest,
    force_same_mode_apply: bool,
) -> Result<SurfaceMode, String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window unavailable".to_string())?;
    let st = app
        .try_state::<Mutex<AppState>>()
        .ok_or_else(|| "app state unavailable".to_string())?;

    let resolution = {
        let mut guard = st.lock().unwrap();
        resolve_transition_request(&mut guard, &request, force_same_mode_apply)
    };

    match resolution {
        TransitionResolution::ModeChange { transition, target } => {
            apply_transition(app, &window, &transition, target, request.position)
        }
        TransitionResolution::SameModeRetarget { mode, target } => {
            apply_same_mode_target_update(app, &window, mode, target, request.position)
        }
        TransitionResolution::SameModeReopen { mode, target } => {
            let transition = SurfaceTransition {
                from: mode,
                to: mode,
                properties: mode.window_properties(),
            };
            apply_transition(app, &window, &transition, target, request.position)
        }
        TransitionResolution::Noop { mode } => Ok(mode),
    }
}

fn resolve_transition_request(
    state: &mut AppState,
    request: &ShellTransitionRequest,
    force_same_mode_apply: bool,
) -> TransitionResolution {
    let previous_target = state.current_target.clone();
    match state.transition_surface(request.mode, Some(request.target.clone())) {
        Some(transition) => TransitionResolution::ModeChange {
            transition,
            target: state.current_target.clone(),
        },
        None if state.current_target != previous_target => TransitionResolution::SameModeRetarget {
            mode: state.surface_machine.current(),
            target: state.current_target.clone(),
        },
        None if force_same_mode_apply => TransitionResolution::SameModeReopen {
            mode: state.surface_machine.current(),
            target: state.current_target.clone(),
        },
        None => TransitionResolution::Noop {
            mode: state.surface_machine.current(),
        },
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
    events::emit_surface_mode_changed(app, mode, mode, target);
    Ok(mode)
}

fn apply_transition(
    app: &AppHandle,
    window: &WebviewWindow,
    transition: &SurfaceTransition,
    current_target: SurfaceTarget,
    position: Option<(i32, i32)>,
) -> Result<SurfaceMode, String> {
    if let Some((x, y)) = position {
        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    }

    let apply_result = apply_window_properties(window, &transition.properties);
    if apply_result.is_err() {
        let _ = window.show();
        let _ = window.set_focus();
    }

    events::emit_surface_mode_changed(app, transition.from, transition.to, current_target);
    match apply_result {
        Ok(()) => Ok(transition.to),
        Err(err) => {
            tracing::warn!(
                "shell: recovered from window-property failure during {:?} -> {:?}: {}",
                transition.from,
                transition.to,
                err
            );
            Ok(transition.to)
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
    fn same_mode_about_request_resolves_as_retarget() {
        let mut state = AppState::new();
        state.transition_surface(
            SurfaceMode::Settings,
            Some(SurfaceTarget::Settings {
                tab: "general".into(),
            }),
        );

        let resolution = resolve_transition_request(
            &mut state,
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
            &mut state,
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
            &mut state,
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
}
