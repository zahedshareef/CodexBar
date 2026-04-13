//! Proof/debug harness for the Tauri desktop shell.
//!
//! Activated by the `CODEXBAR_PROOF_MODE` environment variable.  The value
//! specifies a target surface and optional settings tab to display on
//! startup, e.g.:
//!
//!   - `trayPanel`          — show the tray panel
//!   - `popOut`             — show the pop-out dashboard
//!   - `settings`           — show settings (General tab)
//!   - `settings:apiKeys`   — show settings on the API Keys tab
//!   - `settings:cookies`   — show settings on the Cookies tab
//!
//! In proof mode the shell immediately transitions to the requested surface
//! and suppresses blur-dismiss so the window stays visible for automated
//! screenshot capture.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::shell;
use crate::state::AppState;
use crate::surface::SurfaceMode;

/// Proof configuration parsed from `CODEXBAR_PROOF_MODE`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofConfig {
    /// The surface to show on startup (serialized as the camelCase id).
    pub target_surface: String,
    /// Optional settings tab id (e.g. `"apiKeys"`, `"cookies"`).
    pub settings_tab: Option<String>,
}

impl ProofConfig {
    /// Read proof configuration from the environment.
    ///
    /// Returns `None` when `CODEXBAR_PROOF_MODE` is unset or empty.
    pub fn from_env() -> Option<Self> {
        let raw = std::env::var("CODEXBAR_PROOF_MODE").ok()?;
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }

        let (surface_str, tab) = if let Some((s, t)) = raw.split_once(':') {
            (s, Some(t.to_string()))
        } else {
            (raw, None)
        };

        if SurfaceMode::parse(surface_str).is_none() {
            tracing::warn!("CODEXBAR_PROOF_MODE: unknown surface '{surface_str}', ignoring");
            return None;
        }

        Some(ProofConfig {
            target_surface: surface_str.to_string(),
            settings_tab: tab,
        })
    }

    /// Resolve the target `SurfaceMode` enum value.
    pub fn surface_mode(&self) -> SurfaceMode {
        SurfaceMode::parse(&self.target_surface).unwrap_or(SurfaceMode::TrayPanel)
    }
}

/// Immediately transition to the proof-mode target surface.
///
/// Called from the Tauri `setup` closure when proof mode is active.
pub fn activate(app: &AppHandle) {
    let config = {
        let st = app.state::<Mutex<AppState>>();
        st.lock().unwrap().proof_config.clone()
    };

    let Some(config) = config else { return };
    let target = config.surface_mode();

    tracing::info!(
        "proof-harness: activating surface={} tab={:?}",
        config.target_surface,
        config.settings_tab,
    );

    let position = proof_window_position(app);
    shell::transition_surface(app, target, position);
}

/// Calculate a predictable, centered-ish window position for proof captures.
fn proof_window_position(app: &AppHandle) -> Option<(i32, i32)> {
    let window = app.get_webview_window("main")?;
    let monitor = window.primary_monitor().ok()??;
    let pos = monitor.position();
    let size = monitor.size();
    // Place roughly centred: 25% from left, 20% from top.
    let x = pos.x + (size.width as i32 / 4);
    let y = pos.y + (size.height as i32 / 5);
    Some((x, y))
}

/// Returns `true` when proof mode is active in the shared state.
pub fn is_proof_mode(app: &AppHandle) -> bool {
    app.try_state::<Mutex<AppState>>()
        .map(|st| st.lock().unwrap().proof_config.is_some())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    // SAFETY: These tests manipulate env vars for the current process only.
    // They are not run in parallel with other env-dependent tests.

    #[test]
    fn parse_simple_surface() {
        unsafe { std::env::set_var("CODEXBAR_PROOF_MODE", "trayPanel") };
        let cfg = ProofConfig::from_env().unwrap();
        assert_eq!(cfg.target_surface, "trayPanel");
        assert!(cfg.settings_tab.is_none());
        assert_eq!(cfg.surface_mode(), SurfaceMode::TrayPanel);
        unsafe { std::env::remove_var("CODEXBAR_PROOF_MODE") };
    }

    #[test]
    fn parse_settings_with_tab() {
        unsafe { std::env::set_var("CODEXBAR_PROOF_MODE", "settings:apiKeys") };
        let cfg = ProofConfig::from_env().unwrap();
        assert_eq!(cfg.target_surface, "settings");
        assert_eq!(cfg.settings_tab.as_deref(), Some("apiKeys"));
        assert_eq!(cfg.surface_mode(), SurfaceMode::Settings);
        unsafe { std::env::remove_var("CODEXBAR_PROOF_MODE") };
    }

    #[test]
    fn empty_env_returns_none() {
        unsafe { std::env::set_var("CODEXBAR_PROOF_MODE", "") };
        assert!(ProofConfig::from_env().is_none());
        unsafe { std::env::remove_var("CODEXBAR_PROOF_MODE") };
    }

    #[test]
    fn unset_env_returns_none() {
        unsafe { std::env::remove_var("CODEXBAR_PROOF_MODE") };
        assert!(ProofConfig::from_env().is_none());
    }

    #[test]
    fn invalid_surface_returns_none() {
        unsafe { std::env::set_var("CODEXBAR_PROOF_MODE", "bogus") };
        assert!(ProofConfig::from_env().is_none());
        unsafe { std::env::remove_var("CODEXBAR_PROOF_MODE") };
    }

    #[test]
    fn pop_out_surface() {
        unsafe { std::env::set_var("CODEXBAR_PROOF_MODE", "popOut") };
        let cfg = ProofConfig::from_env().unwrap();
        assert_eq!(cfg.surface_mode(), SurfaceMode::PopOut);
        unsafe { std::env::remove_var("CODEXBAR_PROOF_MODE") };
    }
}
