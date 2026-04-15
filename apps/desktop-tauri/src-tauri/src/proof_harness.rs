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
use crate::surface_target::SurfaceTarget;

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

    /// Resolve the proof-mode target contract for the requested surface.
    pub fn surface_target(&self) -> SurfaceTarget {
        match self.surface_mode() {
            SurfaceMode::Settings => SurfaceTarget::Settings {
                tab: self
                    .settings_tab
                    .clone()
                    .unwrap_or_else(|| "general".to_string()),
            },
            mode => SurfaceTarget::default_for_mode(mode),
        }
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
    let mode = config.surface_mode();
    let target = config.surface_target();

    tracing::info!(
        "proof-harness: activating surface={} tab={:?}",
        config.target_surface,
        config.settings_tab,
    );

    let position = proof_window_position(app);
    shell::transition_surface(app, mode, Some(target), position);
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
    use std::sync::{LazyLock, Mutex};

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn with_proof_mode_env(value: Option<&str>, test: impl FnOnce()) {
        let _guard = ENV_LOCK.lock().unwrap();
        let prev = std::env::var("CODEXBAR_PROOF_MODE").ok();

        match value {
            Some(value) => unsafe { std::env::set_var("CODEXBAR_PROOF_MODE", value) },
            None => unsafe { std::env::remove_var("CODEXBAR_PROOF_MODE") },
        }

        test();

        match prev {
            Some(prev) => unsafe { std::env::set_var("CODEXBAR_PROOF_MODE", prev) },
            None => unsafe { std::env::remove_var("CODEXBAR_PROOF_MODE") },
        }
    }

    #[test]
    fn parse_simple_surface() {
        with_proof_mode_env(Some("trayPanel"), || {
            let cfg = ProofConfig::from_env().unwrap();
            assert_eq!(cfg.target_surface, "trayPanel");
            assert!(cfg.settings_tab.is_none());
            assert_eq!(cfg.surface_mode(), SurfaceMode::TrayPanel);
        });
    }

    #[test]
    fn parse_settings_with_tab() {
        with_proof_mode_env(Some("settings:apiKeys"), || {
            let cfg = ProofConfig::from_env().unwrap();
            assert_eq!(cfg.target_surface, "settings");
            assert_eq!(cfg.settings_tab.as_deref(), Some("apiKeys"));
            assert_eq!(cfg.surface_mode(), SurfaceMode::Settings);
            assert_eq!(
                cfg.surface_target(),
                SurfaceTarget::Settings {
                    tab: "apiKeys".into()
                }
            );
        });
    }

    #[test]
    fn empty_env_returns_none() {
        with_proof_mode_env(Some(""), || {
            assert!(ProofConfig::from_env().is_none());
        });
    }

    #[test]
    fn unset_env_returns_none() {
        with_proof_mode_env(None, || {
            assert!(ProofConfig::from_env().is_none());
        });
    }

    #[test]
    fn invalid_surface_returns_none() {
        with_proof_mode_env(Some("bogus"), || {
            assert!(ProofConfig::from_env().is_none());
        });
    }

    #[test]
    fn pop_out_surface() {
        with_proof_mode_env(Some("popOut"), || {
            let cfg = ProofConfig::from_env().unwrap();
            assert_eq!(cfg.surface_mode(), SurfaceMode::PopOut);
            assert_eq!(cfg.surface_target(), SurfaceTarget::Dashboard);
        });
    }

    #[test]
    fn settings_without_tab_uses_general_target() {
        with_proof_mode_env(Some("settings"), || {
            let cfg = ProofConfig::from_env().unwrap();
            assert_eq!(
                cfg.surface_target(),
                SurfaceTarget::Settings {
                    tab: "general".into()
                }
            );
        });
    }
}
