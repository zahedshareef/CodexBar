//! Window-property application and the hide-to-tray flow.

use std::sync::Mutex;

use tauri::{AppHandle, Manager, WebviewWindow};

use crate::state::AppState;
use crate::surface::{SurfaceMode, SurfaceTransition, WindowProperties};
use crate::surface_target::SurfaceTarget;

use super::SHELL_TRANSITION_SERIAL;
use super::transition::{SurfaceSnapshot, apply_transition, current_surface_snapshot};

pub(super) struct HideToTrayPlan {
    pub previous: SurfaceSnapshot,
    pub transition: Option<SurfaceTransition>,
    pub target: SurfaceTarget,
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

pub(super) fn prepare_hide_to_tray_if_current<P>(
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
