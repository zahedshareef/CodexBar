// Future-use fields/variants — suppress until vertical slices consume them.
#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;

use crate::commands::ProviderUsageSnapshot;
use crate::surface::SurfaceStateMachine;

/// App-update lifecycle tracking.
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateState {
    Idle,
    Checking,
    Available(String),
    Downloading(f32),
    Ready,
    Error(String),
}

impl Default for UpdateState {
    fn default() -> Self {
        Self::Idle
    }
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
    pub tray_anchor: Option<TrayAnchor>,
    pub provider_cache: Vec<ProviderUsageSnapshot>,
    pub is_refreshing: bool,
    pub update_state: UpdateState,
    /// Full update metadata from the last successful check.
    pub update_info: Option<codexbar::updater::UpdateInfo>,
    /// Path to a downloaded installer ready to apply.
    pub installer_path: Option<PathBuf>,
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
            tray_anchor: None,
            provider_cache: Vec::new(),
            is_refreshing: false,
            update_state: UpdateState::Idle,
            update_info: None,
            installer_path: None,
        }
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
