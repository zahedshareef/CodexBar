// Placeholder emitters for vertical slices — suppress dead-code until wired.
#![allow(dead_code)]

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::commands::ProviderUsageSnapshot;
use crate::state::UpdateStatePayload;
use crate::surface::SurfaceMode;

// ── Event name constants ─────────────────────────────────────────────

pub const SURFACE_MODE_CHANGED: &str = "surface-mode-changed";
pub const PROVIDER_UPDATED: &str = "provider-updated";
pub const REFRESH_STARTED: &str = "refresh-started";
pub const REFRESH_COMPLETE: &str = "refresh-complete";
pub const UPDATE_STATE_CHANGED: &str = "update-state-changed";
pub const LOGIN_PHASE_CHANGED: &str = "login-phase-changed";

// ── Payloads ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceModePayload {
    pub mode: &'static str,
    pub previous: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshCompletePayload {
    pub provider_count: usize,
    pub error_count: usize,
}

// ── Emit helpers ─────────────────────────────────────────────────────

pub fn emit_surface_mode_changed(app: &AppHandle, from: SurfaceMode, to: SurfaceMode) {
    let _ = app.emit(
        SURFACE_MODE_CHANGED,
        SurfaceModePayload {
            mode: to.as_str(),
            previous: from.as_str(),
        },
    );
}

pub fn emit_provider_updated(app: &AppHandle, snapshot: &ProviderUsageSnapshot) {
    let _ = app.emit(PROVIDER_UPDATED, snapshot);
}

pub fn emit_refresh_started(app: &AppHandle) {
    let _ = app.emit(REFRESH_STARTED, ());
}

pub fn emit_refresh_complete(app: &AppHandle, provider_count: usize, error_count: usize) {
    let _ = app.emit(
        REFRESH_COMPLETE,
        RefreshCompletePayload {
            provider_count,
            error_count,
        },
    );
}

pub fn emit_update_state_changed(app: &AppHandle, payload: &UpdateStatePayload) {
    let _ = app.emit(UPDATE_STATE_CHANGED, payload);
}

pub fn emit_login_phase_changed(app: &AppHandle) {
    let _ = app.emit(LOGIN_PHASE_CHANGED, ());
}
