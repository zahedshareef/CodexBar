//! Proof/debug harness for the Tauri desktop shell.
//!
//! Activated by the `CODEXBAR_PROOF_MODE` environment variable.  The value
//! specifies a target surface and optional settings tab to display on
//! startup, e.g.:
//!
//!   - `trayPanel`          — show the tray panel
//!   - `popOut`             — show the pop-out dashboard
//!   - `popOut:provider:codex` — show a provider pop-out
//!   - `settings`           — show settings (General tab)
//!   - `settings:apiKeys`   — show settings on the API Keys tab
//!   - `settings:cookies`   — show settings on the Cookies tab
//!   - `settings:about`     — show settings on the About tab
//!
//! In proof mode the shell immediately transitions to the requested surface
//! and suppresses blur-dismiss so the window stays visible for automated
//! screenshot capture.

use std::sync::{LazyLock, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::commands::get_provider_catalog;
use crate::events;
use crate::shell;
use crate::state::AppState;
use crate::surface::SurfaceMode;
use crate::surface_target::{SurfaceTarget, is_supported_provider_id, is_supported_settings_tab};
use crate::tray_menu;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofStatePayload {
    pub mode: String,
    pub target: SurfaceTarget,
    pub window_rect: Option<ProofRect>,
    pub tray_anchor: Option<ProofRect>,
    pub work_area: Option<ProofRect>,
    pub menu_path: Option<String>,
    pub menu_items: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct ProofMenuSnapshot {
    menu_path: Option<String>,
    menu_items: Vec<String>,
}

static PROOF_MENU_SNAPSHOT: LazyLock<Mutex<ProofMenuSnapshot>> =
    LazyLock::new(|| Mutex::new(ProofMenuSnapshot::default()));
/// Proof configuration parsed from `CODEXBAR_PROOF_MODE`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofConfig {
    /// The surface to show on startup (serialized as the camelCase id).
    pub target_surface: String,
    /// Optional settings tab id (e.g. `"apiKeys"`, `"cookies"`).
    pub settings_tab: Option<String>,
    /// Optional target payload for richer proof routing, such as
    /// `"provider:codex"` for pop-out provider views.
    pub target_payload: Option<String>,
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

        let (surface_str, payload) = if let Some((s, t)) = raw.split_once(':') {
            (s, Some(t.to_string()))
        } else {
            (raw, None)
        };

        let Some(surface_mode) = SurfaceMode::parse(surface_str) else {
            tracing::warn!("CODEXBAR_PROOF_MODE: unknown surface '{surface_str}', ignoring");
            return None;
        };

        if !proof_payload_is_supported(surface_mode, payload.as_deref()) {
            tracing::warn!("CODEXBAR_PROOF_MODE: unsupported target '{raw}', ignoring");
            return None;
        }

        Some(ProofConfig {
            target_surface: surface_str.to_string(),
            settings_tab: (surface_str == SurfaceMode::Settings.as_str())
                .then_some(payload.clone())
                .flatten(),
            target_payload: payload,
        })
    }

    /// Resolve the target `SurfaceMode` enum value.
    pub fn surface_mode(&self) -> SurfaceMode {
        SurfaceMode::parse(&self.target_surface).unwrap_or(SurfaceMode::TrayPanel)
    }

    pub fn surface_target(&self) -> SurfaceTarget {
        match self.surface_mode() {
            SurfaceMode::Hidden | SurfaceMode::TrayPanel => SurfaceTarget::Summary,
            SurfaceMode::PopOut => self
                .target_payload
                .as_deref()
                .and_then(SurfaceTarget::parse)
                .filter(|target| target.mode() == SurfaceMode::PopOut)
                .unwrap_or(SurfaceTarget::Dashboard),
            SurfaceMode::Settings => SurfaceTarget::Settings {
                tab: self
                    .settings_tab
                    .clone()
                    .unwrap_or_else(|| "general".into()),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofCommand {
    OpenTrayPanel,
    OpenNativeMenu,
    OpenDashboard,
    OpenProvider { provider_id: String },
    OpenSettings { tab: String },
    OpenAboutPath,
    HideSurface,
}

impl ProofCommand {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "open-tray-panel" => Some(Self::OpenTrayPanel),
            "open-native-menu" => Some(Self::OpenNativeMenu),
            "open-dashboard" => Some(Self::OpenDashboard),
            "open-about-path" => Some(Self::OpenAboutPath),
            "hide-surface" => Some(Self::HideSurface),
            _ => {
                if let Some(provider_id) = raw.strip_prefix("open-provider:")
                    && is_supported_provider_id(provider_id)
                {
                    return Some(Self::OpenProvider {
                        provider_id: provider_id.to_string(),
                    });
                }

                if let Some(tab) = raw.strip_prefix("open-settings:")
                    && is_supported_settings_tab(tab)
                {
                    return Some(Self::OpenSettings {
                        tab: tab.to_string(),
                    });
                }

                None
            }
        }
    }
}

/// Immediately transition to the proof-mode target surface.
///
/// Called from the Tauri `setup` closure when proof mode is active.
pub fn activate(app: &AppHandle) {
    clear_menu_snapshot();
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
    let _ = shell::transition_to_target(app, target, config.surface_target(), position);
    emit_state_changed(app);
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

pub fn capture_state(app: &AppHandle) -> Result<ProofStatePayload, String> {
    let (mode, target, tray_anchor) = {
        let st = app
            .try_state::<Mutex<AppState>>()
            .ok_or_else(|| "app state unavailable".to_string())?;
        let guard = st.lock().unwrap();
        (
            guard.surface_machine.current().as_str().to_string(),
            guard.current_target.clone(),
            guard.tray_anchor.map(|anchor| ProofRect {
                x: anchor.x,
                y: anchor.y,
                width: anchor.width,
                height: anchor.height,
            }),
        )
    };

    let menu_snapshot = PROOF_MENU_SNAPSHOT.lock().unwrap().clone();
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window unavailable".to_string())?;

    Ok(ProofStatePayload {
        mode,
        target,
        window_rect: window_rect(&window),
        tray_anchor: tray_anchor.clone(),
        work_area: resolve_work_area(&window, tray_anchor.as_ref()),
        menu_path: menu_snapshot.menu_path,
        menu_items: menu_snapshot.menu_items,
    })
}

pub fn run_command(app: &AppHandle, command: ProofCommand) -> Result<ProofStatePayload, String> {
    ensure_proof_mode(app)?;
    clear_menu_snapshot();

    match command {
        ProofCommand::OpenTrayPanel => {
            let position =
                shell::tray_panel_position(app).or_else(|| shell::shortcut_panel_position(app));
            shell::reopen_to_target(
                app,
                SurfaceMode::TrayPanel,
                SurfaceTarget::Summary,
                position,
            )?;
        }
        ProofCommand::OpenNativeMenu => {
            set_menu_snapshot(Some("tray".into()), native_menu_items());
        }
        ProofCommand::OpenDashboard => {
            shell::transition_to_target(app, SurfaceMode::PopOut, SurfaceTarget::Dashboard, None)?;
        }
        ProofCommand::OpenProvider { provider_id } => {
            shell::transition_to_target(
                app,
                SurfaceMode::PopOut,
                SurfaceTarget::Provider { provider_id },
                None,
            )?;
        }
        ProofCommand::OpenSettings { tab } => {
            shell::transition_to_target(
                app,
                SurfaceMode::Settings,
                SurfaceTarget::Settings { tab },
                None,
            )?;
        }
        ProofCommand::OpenAboutPath => {
            transition_about_path(app)?;
        }
        ProofCommand::HideSurface => {
            shell::hide_to_tray(app)?;
        }
    }

    let payload = capture_state(app)?;
    events::emit_proof_state_changed(app, &payload);
    Ok(payload)
}

pub fn ensure_proof_mode(app: &AppHandle) -> Result<(), String> {
    if is_proof_mode(app) {
        Ok(())
    } else {
        Err("proof harness is disabled".into())
    }
}

pub fn emit_state_changed(app: &AppHandle) {
    if let Ok(payload) = capture_state(app) {
        events::emit_proof_state_changed(app, &payload);
    }
}

fn clear_menu_snapshot() {
    set_menu_snapshot(None, Vec::new());
}

fn transition_about_path(app: &AppHandle) -> Result<(), String> {
    persist_about_path_snapshot(
        shell::transition_to_target(
            app,
            SurfaceMode::Settings,
            SurfaceTarget::Settings {
                tab: "about".into(),
            },
            None,
        )
        .map(|_| ()),
    )
}

fn persist_about_path_snapshot(result: Result<(), String>) -> Result<(), String> {
    match result {
        Ok(()) => {
            set_menu_snapshot(Some("tray/about".into()), native_menu_items());
            Ok(())
        }
        Err(err) => {
            clear_menu_snapshot();
            Err(err)
        }
    }
}

fn set_menu_snapshot(menu_path: Option<String>, menu_items: Vec<String>) {
    let mut snapshot = PROOF_MENU_SNAPSHOT.lock().unwrap();
    snapshot.menu_path = menu_path;
    snapshot.menu_items = menu_items;
}

#[cfg(test)]
fn menu_snapshot() -> ProofMenuSnapshot {
    PROOF_MENU_SNAPSHOT.lock().unwrap().clone()
}

fn native_menu_items() -> Vec<String> {
    let providers = get_provider_catalog();
    tray_menu::proof_menu_items(&tray_menu::build_tray_menu(&providers))
}

fn proof_payload_is_supported(surface_mode: SurfaceMode, payload: Option<&str>) -> bool {
    match (surface_mode, payload) {
        (SurfaceMode::Hidden | SurfaceMode::TrayPanel, None) => true,
        (SurfaceMode::Hidden | SurfaceMode::TrayPanel, Some(_)) => false,
        (SurfaceMode::Settings, None) => true,
        (SurfaceMode::Settings, Some(tab)) => is_supported_settings_tab(tab),
        (SurfaceMode::PopOut, None) => true,
        (SurfaceMode::PopOut, Some(raw_target)) => {
            let Some(target) = SurfaceTarget::parse(raw_target) else {
                return false;
            };

            match target {
                SurfaceTarget::Dashboard => true,
                SurfaceTarget::Provider { provider_id } => is_supported_provider_id(&provider_id),
                _ => false,
            }
        }
    }
}

fn window_rect(window: &WebviewWindow) -> Option<ProofRect> {
    if !window.is_visible().ok()? {
        return None;
    }

    let position = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    Some(ProofRect {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    })
}

fn resolve_work_area(window: &WebviewWindow, tray_anchor: Option<&ProofRect>) -> Option<ProofRect> {
    let monitors = window.available_monitors().ok()?;
    if let Some(anchor) = tray_anchor
        && let Some(monitor) = monitor_containing_rect(&monitors, anchor)
    {
        return Some(monitor_work_area_rect(monitor));
    }

    if let Ok(Some(monitor)) = window.current_monitor() {
        return Some(monitor_work_area_rect(&monitor));
    }

    window
        .primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| monitor_work_area_rect(&monitor))
}

fn monitor_containing_rect<'a>(
    monitors: &'a [tauri::Monitor],
    rect: &ProofRect,
) -> Option<&'a tauri::Monitor> {
    let center_x = rect.x + rect.width as i32 / 2;
    let center_y = rect.y + rect.height as i32 / 2;

    monitors.iter().find(|monitor| {
        let position = monitor.position();
        let size = monitor.size();
        center_x >= position.x
            && center_x < position.x + size.width as i32
            && center_y >= position.y
            && center_y < position.y + size.height as i32
    })
}

fn monitor_work_area_rect(monitor: &tauri::Monitor) -> ProofRect {
    let work_area = monitor.work_area();
    ProofRect {
        x: work_area.position.x,
        y: work_area.position.y,
        width: work_area.size.width,
        height: work_area.size.height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn parse_settings_about_proof_target() {
        with_proof_mode_env(Some("settings:about"), || {
            let cfg = ProofConfig::from_env().unwrap();
            assert_eq!(cfg.target_surface, "settings");
            assert_eq!(cfg.settings_tab.as_deref(), Some("about"));
        });
    }

    #[test]
    fn parse_provider_popout_proof_target() {
        with_proof_mode_env(Some("popOut:provider:codex"), || {
            let cfg = ProofConfig::from_env().unwrap();
            assert_eq!(cfg.target_surface, "popOut");
            assert_eq!(cfg.target_payload.as_deref(), Some("provider:codex"));
            assert_eq!(
                cfg.surface_target(),
                SurfaceTarget::Provider {
                    provider_id: "codex".into()
                }
            );
        });
    }

    #[test]
    fn parse_proof_command_for_native_menu() {
        let command = ProofCommand::parse("open-native-menu").unwrap();
        assert_eq!(command, ProofCommand::OpenNativeMenu);
    }

    #[test]
    fn parse_proof_command_for_about_entry_path() {
        let command = ProofCommand::parse("open-about-path").unwrap();
        assert_eq!(command, ProofCommand::OpenAboutPath);
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
    fn invalid_settings_tab_returns_none() {
        with_proof_mode_env(Some("settings:security"), || {
            assert!(ProofConfig::from_env().is_none());
        });
    }

    #[test]
    fn invalid_provider_target_returns_none() {
        with_proof_mode_env(Some("popOut:provider:not-a-provider"), || {
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
    fn parse_proof_command_rejects_unknown_provider() {
        assert!(ProofCommand::parse("open-provider:not-a-provider").is_none());
    }

    #[test]
    fn parse_proof_command_rejects_unknown_settings_tab() {
        assert!(ProofCommand::parse("open-settings:security").is_none());
    }

    #[test]
    fn about_path_snapshot_persists_only_after_success() {
        clear_menu_snapshot();

        let result = persist_about_path_snapshot(Ok(()));

        assert!(result.is_ok());
        let snapshot = menu_snapshot();
        assert_eq!(snapshot.menu_path.as_deref(), Some("tray/about"));
        assert!(!snapshot.menu_items.is_empty());
    }

    #[test]
    fn about_path_snapshot_clears_on_failure() {
        set_menu_snapshot(Some("tray".into()), vec!["About CodexBar".into()]);

        let result = persist_about_path_snapshot(Err("boom".into()));

        assert_eq!(result.unwrap_err(), "boom");
        let snapshot = menu_snapshot();
        assert!(snapshot.menu_path.is_none());
        assert!(snapshot.menu_items.is_empty());
    }
}
