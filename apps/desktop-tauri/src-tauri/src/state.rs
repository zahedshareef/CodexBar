// Future-use fields/variants — suppress until vertical slices consume them.
#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;

use crate::commands::ProviderUsageSnapshot;
use crate::proof_harness::ProofConfig;
use crate::surface::{SurfaceMode, SurfaceStateMachine, SurfaceTransition};
use crate::surface_target::SurfaceTarget;

/// App-update lifecycle tracking.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum UpdateState {
    #[default]
    Idle,
    Checking,
    Available(String),
    Downloading(f32),
    Ready,
    Error(String),
}

/// Serializable update-state payload for the frontend bridge.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatePayload {
    pub status: &'static str,
    pub version: Option<String>,
    pub error: Option<String>,
    pub progress: Option<f32>,
    pub release_url: Option<String>,
    pub can_download: bool,
    pub can_apply: bool,
}

impl UpdateState {
    pub fn to_payload(&self) -> UpdateStatePayload {
        match self {
            Self::Idle => UpdateStatePayload {
                status: "idle",
                version: None,
                error: None,
                progress: None,
                release_url: None,
                can_download: false,
                can_apply: false,
            },
            Self::Checking => UpdateStatePayload {
                status: "checking",
                version: None,
                error: None,
                progress: None,
                release_url: None,
                can_download: false,
                can_apply: false,
            },
            Self::Available(v) => UpdateStatePayload {
                status: "available",
                version: Some(v.clone()),
                error: None,
                progress: None,
                release_url: None,
                can_download: false,
                can_apply: false,
            },
            Self::Downloading(p) => UpdateStatePayload {
                status: "downloading",
                version: None,
                error: None,
                progress: Some(*p),
                release_url: None,
                can_download: false,
                can_apply: false,
            },
            Self::Ready => UpdateStatePayload {
                status: "ready",
                version: None,
                error: None,
                progress: None,
                release_url: None,
                can_download: false,
                can_apply: false,
            },
            Self::Error(e) => UpdateStatePayload {
                status: "error",
                version: None,
                error: Some(e.clone()),
                progress: None,
                release_url: None,
                can_download: false,
                can_apply: false,
            },
        }
    }
}

/// Tray icon anchor in physical pixels, used for panel positioning.
#[derive(Debug, Clone, Copy)]
pub struct TrayAnchor {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Central app state behind `Mutex` for Tauri managed state.
///
/// Access in commands via `state: tauri::State<'_, SharedAppState>`.
pub struct AppState {
    pub surface_machine: SurfaceStateMachine,
    pub current_target: SurfaceTarget,
    pub tray_anchor: Option<TrayAnchor>,
    pub provider_cache: Vec<ProviderUsageSnapshot>,
    pub is_refreshing: bool,
    pub update_state: UpdateState,
    /// Full update metadata from the last successful check.
    pub update_info: Option<codexbar::updater::UpdateInfo>,
    /// Path to a downloaded installer ready to apply.
    pub installer_path: Option<PathBuf>,
    /// Proof-harness configuration (set when `CODEXBAR_PROOF_MODE` is active).
    pub proof_config: Option<ProofConfig>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            surface_machine: SurfaceStateMachine::new(),
            current_target: SurfaceTarget::Summary,
            tray_anchor: None,
            provider_cache: Vec::new(),
            is_refreshing: false,
            update_state: UpdateState::Idle,
            update_info: None,
            installer_path: None,
            proof_config: None,
        }
    }

    pub fn transition_surface(
        &mut self,
        mode: SurfaceMode,
        target: Option<SurfaceTarget>,
    ) -> Option<SurfaceTransition> {
        let transition = self.surface_machine.transition(mode)?;
        self.current_target = match mode {
            SurfaceMode::Hidden => SurfaceTarget::Summary,
            _ => match target {
                Some(target) if target.mode() == mode => target,
                _ => SurfaceTarget::default_for_mode(mode),
            },
        };

        Some(transition)
    }

    /// Build an enriched update payload using the stored update info.
    pub fn update_payload(&self) -> UpdateStatePayload {
        let mut p = self.update_state.to_payload();
        if let Some(ref info) = self.update_info {
            if p.version.is_none() {
                p.version = Some(info.version.clone());
            }
            p.release_url = Some(info.release_url.clone());
            p.can_download = info.supports_auto_download();
            p.can_apply = info.supports_auto_apply();
        }
        p
    }
}

/// The type registered as Tauri managed state.
pub type SharedAppState = Mutex<AppState>;

#[cfg(test)]
mod tests {
    use super::AppState;
    use crate::surface::SurfaceMode;
    use crate::surface_target::SurfaceTarget;

    #[test]
    fn transition_applies_explicit_target_on_mode_change() {
        let mut state = AppState::new();

        let transition = state.transition_surface(
            SurfaceMode::Settings,
            Some(SurfaceTarget::Settings {
                tab: "apiKeys".into(),
            }),
        );

        assert!(transition.is_some());
        assert_eq!(
            state.current_target,
            SurfaceTarget::Settings {
                tab: "apiKeys".into()
            }
        );
    }

    #[test]
    fn transition_applies_summary_target_for_tray_panel() {
        let mut state = AppState::new();

        let transition =
            state.transition_surface(SurfaceMode::TrayPanel, Some(SurfaceTarget::Summary));

        assert!(transition.is_some());
        assert_eq!(state.current_target, SurfaceTarget::Summary);
    }

    #[test]
    fn transition_applies_dashboard_target_for_pop_out() {
        let mut state = AppState::new();

        let transition =
            state.transition_surface(SurfaceMode::PopOut, Some(SurfaceTarget::Dashboard));

        assert!(transition.is_some());
        assert_eq!(state.current_target, SurfaceTarget::Dashboard);
    }

    #[test]
    fn noop_transition_keeps_existing_target() {
        let mut state = AppState::new();
        state.transition_surface(
            SurfaceMode::Settings,
            Some(SurfaceTarget::Settings {
                tab: "apiKeys".into(),
            }),
        );

        let transition = state.transition_surface(
            SurfaceMode::Settings,
            Some(SurfaceTarget::Settings {
                tab: "cookies".into(),
            }),
        );

        assert!(transition.is_none());
        assert_eq!(
            state.current_target,
            SurfaceTarget::Settings {
                tab: "apiKeys".into()
            }
        );
    }

    #[test]
    fn hidden_transition_resets_target_to_summary() {
        let mut state = AppState::new();
        state.transition_surface(
            SurfaceMode::Settings,
            Some(SurfaceTarget::Settings {
                tab: "apiKeys".into(),
            }),
        );

        let transition = state.transition_surface(SurfaceMode::Hidden, None);

        assert!(transition.is_some());
        assert_eq!(state.current_target, SurfaceTarget::Summary);
    }

    #[test]
    fn incompatible_target_falls_back_to_mode_default() {
        let mut state = AppState::new();

        state.transition_surface(
            SurfaceMode::PopOut,
            Some(SurfaceTarget::Settings {
                tab: "general".into(),
            }),
        );

        assert_eq!(state.current_target, SurfaceTarget::Dashboard);
    }
}
