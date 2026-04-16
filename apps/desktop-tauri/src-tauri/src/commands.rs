use std::collections::HashSet;
use std::sync::Mutex;

use codexbar::core::{FetchContext, Provider, ProviderFetchResult, ProviderId, RateWindow};
use codexbar::providers::*;
use codexbar::settings::{ApiKeys, Language, ManualCookies, Settings, TrayIconMode, UpdateChannel};
use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::events;
use crate::proof_harness::{self, ProofCommand, ProofStatePayload};
use crate::state::{AppState, UpdateState, UpdateStatePayload};
use crate::surface::SurfaceMode;
use crate::surface_target::{SurfaceTarget, is_supported_provider_id, is_supported_settings_tab};

// ── Bridge snapshot types ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateWindowSnapshot {
    pub used_percent: f64,
    pub remaining_percent: f64,
    pub window_minutes: Option<u32>,
    pub resets_at: Option<String>,
    pub reset_description: Option<String>,
    pub is_exhausted: bool,
}

impl RateWindowSnapshot {
    fn from_rate_window(rw: &RateWindow) -> Self {
        Self {
            used_percent: rw.used_percent,
            remaining_percent: rw.remaining_percent(),
            window_minutes: rw.window_minutes,
            resets_at: rw.resets_at.map(|dt| dt.to_rfc3339()),
            reset_description: rw.reset_description.clone(),
            is_exhausted: rw.is_exhausted(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostSnapshotBridge {
    pub used: f64,
    pub limit: Option<f64>,
    pub remaining: Option<f64>,
    pub currency_code: String,
    pub period: String,
    pub resets_at: Option<String>,
    pub formatted_used: String,
    pub formatted_limit: Option<String>,
}

/// A frontend-friendly snapshot of one provider's usage data.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageSnapshot {
    pub provider_id: String,
    pub display_name: String,
    pub primary: RateWindowSnapshot,
    pub secondary: Option<RateWindowSnapshot>,
    pub model_specific: Option<RateWindowSnapshot>,
    pub tertiary: Option<RateWindowSnapshot>,
    pub cost: Option<CostSnapshotBridge>,
    pub plan_name: Option<String>,
    pub account_email: Option<String>,
    pub source_label: String,
    pub updated_at: String,
    pub error: Option<String>,
}

impl ProviderUsageSnapshot {
    fn from_fetch_result(id: ProviderId, result: &ProviderFetchResult) -> Self {
        let usage = &result.usage;
        Self {
            provider_id: id.cli_name().to_string(),
            display_name: id.display_name().to_string(),
            primary: RateWindowSnapshot::from_rate_window(&usage.primary),
            secondary: usage
                .secondary
                .as_ref()
                .map(RateWindowSnapshot::from_rate_window),
            model_specific: usage
                .model_specific
                .as_ref()
                .map(RateWindowSnapshot::from_rate_window),
            tertiary: usage
                .tertiary
                .as_ref()
                .map(RateWindowSnapshot::from_rate_window),
            cost: result.cost.as_ref().map(|c| CostSnapshotBridge {
                used: c.used,
                limit: c.limit,
                remaining: c.remaining(),
                currency_code: c.currency_code.clone(),
                period: c.period.clone(),
                resets_at: c.resets_at.map(|dt| dt.to_rfc3339()),
                formatted_used: c.format_used(),
                formatted_limit: c.format_limit(),
            }),
            plan_name: usage.login_method.clone(),
            account_email: usage.account_email.clone(),
            source_label: result.source_label.clone(),
            updated_at: usage.updated_at.to_rfc3339(),
            error: None,
        }
    }

    fn from_error(id: ProviderId, error: String) -> Self {
        Self {
            provider_id: id.cli_name().to_string(),
            display_name: id.display_name().to_string(),
            primary: RateWindowSnapshot {
                used_percent: 0.0,
                remaining_percent: 100.0,
                window_minutes: None,
                resets_at: None,
                reset_description: None,
                is_exhausted: false,
            },
            secondary: None,
            model_specific: None,
            tertiary: None,
            cost: None,
            plan_name: None,
            account_email: None,
            source_label: String::new(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapState {
    contract_version: &'static str,
    surface_modes: Vec<SurfaceModeDescriptor>,
    commands: Vec<BridgeCommandDescriptor>,
    events: Vec<BridgeEventDescriptor>,
    providers: Vec<ProviderCatalogEntry>,
    settings: SettingsSnapshot,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceModeDescriptor {
    id: &'static str,
    label: &'static str,
    description: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeCommandDescriptor {
    id: &'static str,
    description: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeEventDescriptor {
    id: &'static str,
    description: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentSurfaceState {
    pub mode: String,
    pub target: SurfaceTarget,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCatalogEntry {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) cookie_domain: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    enabled_providers: Vec<String>,
    refresh_interval_secs: u64,
    start_at_login: bool,
    show_notifications: bool,
    tray_icon_mode: &'static str,
    show_as_used: bool,
    surprise_animations: bool,
    enable_animations: bool,
    reset_time_relative: bool,
    menu_bar_display_mode: String,
    hide_personal_info: bool,
    update_channel: &'static str,
    global_shortcut: String,
    ui_language: &'static str,
}

#[tauri::command]
pub fn get_bootstrap_state() -> BootstrapState {
    BootstrapState {
        contract_version: "v1",
        surface_modes: surface_modes(),
        commands: bridge_commands(),
        events: bridge_events(),
        providers: provider_catalog(),
        settings: SettingsSnapshot::from(Settings::load()),
    }
}

#[tauri::command]
pub fn get_provider_catalog() -> Vec<ProviderCatalogEntry> {
    provider_catalog()
}

#[tauri::command]
pub fn get_settings_snapshot() -> SettingsSnapshot {
    SettingsSnapshot::from(Settings::load())
}

impl From<Settings> for SettingsSnapshot {
    fn from(settings: Settings) -> Self {
        let mut enabled_providers = settings.enabled_providers.into_iter().collect::<Vec<_>>();
        enabled_providers.sort();

        Self {
            enabled_providers,
            refresh_interval_secs: settings.refresh_interval_secs,
            start_at_login: settings.start_at_login,
            show_notifications: settings.show_notifications,
            tray_icon_mode: tray_icon_mode_label(settings.tray_icon_mode),
            show_as_used: settings.show_as_used,
            surprise_animations: settings.surprise_animations,
            enable_animations: settings.enable_animations,
            reset_time_relative: settings.reset_time_relative,
            menu_bar_display_mode: settings.menu_bar_display_mode,
            hide_personal_info: settings.hide_personal_info,
            update_channel: update_channel_label(settings.update_channel),
            global_shortcut: settings.global_shortcut,
            ui_language: language_label(settings.ui_language),
        }
    }
}

fn provider_catalog() -> Vec<ProviderCatalogEntry> {
    ProviderId::all()
        .iter()
        .map(|provider| ProviderCatalogEntry {
            id: provider.cli_name().to_string(),
            display_name: provider.display_name().to_string(),
            cookie_domain: provider.cookie_domain().map(ToString::to_string),
        })
        .collect()
}

fn surface_modes() -> Vec<SurfaceModeDescriptor> {
    vec![
        SurfaceModeDescriptor {
            id: "hidden",
            label: "Hidden",
            description: "No window is visible; the tray icon remains active.",
        },
        SurfaceModeDescriptor {
            id: "trayPanel",
            label: "Tray panel",
            description: "Borderless anchored panel opened from a tray left click.",
        },
        SurfaceModeDescriptor {
            id: "popOut",
            label: "Pop out",
            description: "Decorated window for a richer, persistent provider view.",
        },
        SurfaceModeDescriptor {
            id: "settings",
            label: "Settings",
            description: "Dedicated settings surface for provider and shell configuration.",
        },
    ]
}

fn bridge_commands() -> Vec<BridgeCommandDescriptor> {
    vec![
        BridgeCommandDescriptor {
            id: "get_bootstrap_state",
            description: "Load the shell contract, provider catalog, and persisted settings snapshot.",
        },
        BridgeCommandDescriptor {
            id: "get_provider_catalog",
            description: "List providers available to the desktop shell from the shared Rust engine.",
        },
        BridgeCommandDescriptor {
            id: "get_settings_snapshot",
            description: "Read persisted settings from the existing Rust settings file format.",
        },
        BridgeCommandDescriptor {
            id: "refresh_providers",
            description: "Async refresh of provider usage snapshots with per-provider event updates.",
        },
        BridgeCommandDescriptor {
            id: "get_cached_providers",
            description: "Return the most recent provider usage snapshots from the in-memory cache.",
        },
        BridgeCommandDescriptor {
            id: "update_settings",
            description: "Persist a partial settings update through the shared Rust settings facade.",
        },
        BridgeCommandDescriptor {
            id: "set_surface_mode",
            description: "Switch the shell to a visible surface using a required typed target.",
        },
        BridgeCommandDescriptor {
            id: "get_current_surface_mode",
            description: "Read the current coarse shell surface mode.",
        },
        BridgeCommandDescriptor {
            id: "get_current_surface_state",
            description: "Read the current coarse shell mode together with its typed target.",
        },
        BridgeCommandDescriptor {
            id: "get_proof_state",
            description: "Dump proof-harness state including surface target, window rect, tray anchor, and work-area evidence.",
        },
        BridgeCommandDescriptor {
            id: "run_proof_command",
            description: "Drive deterministic proof-harness transitions such as tray, native menu, dashboard, provider, settings, about, and hide.",
        },
        BridgeCommandDescriptor {
            id: "get_update_state",
            description: "Get the current app-update lifecycle state.",
        },
        BridgeCommandDescriptor {
            id: "check_for_updates",
            description: "Trigger an update check against the configured channel.",
        },
        BridgeCommandDescriptor {
            id: "download_update",
            description: "Download an available update in the background with progress events.",
        },
        BridgeCommandDescriptor {
            id: "apply_update",
            description: "Launch the downloaded installer and exit the application.",
        },
        BridgeCommandDescriptor {
            id: "dismiss_update",
            description: "Dismiss the current update notification and reset to idle.",
        },
        BridgeCommandDescriptor {
            id: "open_release_page",
            description: "Open the release page for the available update in the default browser.",
        },
        BridgeCommandDescriptor {
            id: "get_api_keys",
            description: "List stored API keys for configured providers.",
        },
        BridgeCommandDescriptor {
            id: "get_api_key_providers",
            description: "List providers that support API-key authentication and related help metadata.",
        },
        BridgeCommandDescriptor {
            id: "set_api_key",
            description: "Store or replace an API key for a provider.",
        },
        BridgeCommandDescriptor {
            id: "remove_api_key",
            description: "Delete a stored API key for a provider.",
        },
        BridgeCommandDescriptor {
            id: "get_manual_cookies",
            description: "List manually stored provider cookies.",
        },
        BridgeCommandDescriptor {
            id: "set_manual_cookie",
            description: "Store or replace a manual provider cookie value.",
        },
        BridgeCommandDescriptor {
            id: "remove_manual_cookie",
            description: "Delete a stored manual provider cookie.",
        },
        BridgeCommandDescriptor {
            id: "get_app_info",
            description: "Read app metadata displayed in the shell About surface.",
        },
    ]
}

fn bridge_events() -> Vec<BridgeEventDescriptor> {
    vec![
        BridgeEventDescriptor {
            id: "surface-mode-changed",
            description: "Emitted when the shell changes coarse mode or typed target.",
        },
        BridgeEventDescriptor {
            id: "provider-updated",
            description: "Emitted as provider usage snapshots refresh in the shared backend.",
        },
        BridgeEventDescriptor {
            id: "refresh-started",
            description: "Emitted when a provider refresh cycle begins.",
        },
        BridgeEventDescriptor {
            id: "refresh-complete",
            description: "Emitted when a provider refresh cycle completes.",
        },
        BridgeEventDescriptor {
            id: "update-state-changed",
            description: "Emitted when updater state changes in the backend.",
        },
        BridgeEventDescriptor {
            id: "login-phase-changed",
            description: "Emitted when a provider login flow advances between phases.",
        },
        BridgeEventDescriptor {
            id: "proof-state-changed",
            description: "Emitted when the proof harness updates menu evidence or visible shell state for parity capture.",
        },
    ]
}

fn tray_icon_mode_label(mode: TrayIconMode) -> &'static str {
    match mode {
        TrayIconMode::Single => "single",
        TrayIconMode::PerProvider => "perProvider",
    }
}

fn update_channel_label(channel: UpdateChannel) -> &'static str {
    match channel {
        UpdateChannel::Stable => "stable",
        UpdateChannel::Beta => "beta",
    }
}

fn language_label(language: Language) -> &'static str {
    match language {
        Language::English => "english",
        Language::Chinese => "chinese",
    }
}

// ── Settings mutation ─────────────────────────────────────────────────

/// Partial settings update — every field is optional so the frontend can
/// send only what changed.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SettingsUpdate {
    pub enabled_providers: Option<Vec<String>>,
    pub refresh_interval_secs: Option<u64>,
    pub start_at_login: Option<bool>,
    pub show_notifications: Option<bool>,
    pub tray_icon_mode: Option<String>,
    pub show_as_used: Option<bool>,
    pub surprise_animations: Option<bool>,
    pub enable_animations: Option<bool>,
    pub reset_time_relative: Option<bool>,
    pub menu_bar_display_mode: Option<String>,
    pub hide_personal_info: Option<bool>,
    pub update_channel: Option<String>,
    pub global_shortcut: Option<String>,
    pub ui_language: Option<String>,
}

fn parse_tray_icon_mode(s: &str) -> Option<TrayIconMode> {
    match s {
        "single" => Some(TrayIconMode::Single),
        "perProvider" => Some(TrayIconMode::PerProvider),
        _ => None,
    }
}

fn parse_update_channel(s: &str) -> Option<UpdateChannel> {
    match s {
        "stable" => Some(UpdateChannel::Stable),
        "beta" => Some(UpdateChannel::Beta),
        _ => None,
    }
}

fn parse_language(s: &str) -> Option<Language> {
    match s {
        "english" => Some(Language::English),
        "chinese" => Some(Language::Chinese),
        _ => None,
    }
}

#[tauri::command]
pub fn update_settings(
    app: tauri::AppHandle,
    patch: SettingsUpdate,
) -> Result<SettingsSnapshot, String> {
    let mut settings = Settings::load();

    // If the shortcut is changing, validate and re-register before persisting.
    if let Some(ref new_shortcut) = patch.global_shortcut
        && *new_shortcut != settings.global_shortcut
    {
        crate::shortcut_bridge::reregister_shortcut(&app, &settings.global_shortcut, new_shortcut)?;
    }

    if let Some(providers) = patch.enabled_providers {
        settings.enabled_providers = providers.into_iter().collect::<HashSet<_>>();
    }
    if let Some(v) = patch.refresh_interval_secs {
        settings.refresh_interval_secs = v;
    }
    if let Some(v) = patch.start_at_login {
        settings.start_at_login = v;
    }
    if let Some(v) = patch.show_notifications {
        settings.show_notifications = v;
    }
    if let Some(ref s) = patch.tray_icon_mode
        && let Some(mode) = parse_tray_icon_mode(s)
    {
        settings.tray_icon_mode = mode;
    }
    if let Some(v) = patch.show_as_used {
        settings.show_as_used = v;
    }
    if let Some(v) = patch.surprise_animations {
        settings.surprise_animations = v;
    }
    if let Some(v) = patch.enable_animations {
        settings.enable_animations = v;
    }
    if let Some(v) = patch.reset_time_relative {
        settings.reset_time_relative = v;
    }
    if let Some(v) = patch.menu_bar_display_mode {
        settings.menu_bar_display_mode = v;
    }
    if let Some(v) = patch.hide_personal_info {
        settings.hide_personal_info = v;
    }
    if let Some(ref s) = patch.update_channel
        && let Some(ch) = parse_update_channel(s)
    {
        settings.update_channel = ch;
    }
    if let Some(v) = patch.global_shortcut {
        settings.global_shortcut = v;
    }
    if let Some(ref s) = patch.ui_language
        && let Some(lang) = parse_language(s)
    {
        settings.ui_language = lang;
    }

    settings.save().map_err(|e| e.to_string())?;

    Ok(SettingsSnapshot::from(settings))
}

// ── Surface-mode commands ────────────────────────────────────────────

#[tauri::command]
pub fn set_surface_mode(
    mode: String,
    target: SurfaceTarget,
    window: tauri::WebviewWindow,
) -> Result<String, String> {
    let mode = SurfaceMode::parse(&mode).ok_or_else(|| format!("unknown surface mode: {mode}"))?;
    let target = validate_surface_target(mode, target)?;

    crate::shell::transition_to_target(window.app_handle(), mode, target, None)
        .map(|mode| mode.as_str().to_string())
}

#[tauri::command]
pub fn get_current_surface_mode(state: tauri::State<'_, Mutex<AppState>>) -> String {
    state
        .lock()
        .unwrap()
        .surface_machine
        .current()
        .as_str()
        .to_string()
}

#[tauri::command]
pub fn get_current_surface_state(state: tauri::State<'_, Mutex<AppState>>) -> CurrentSurfaceState {
    let guard = state.lock().unwrap();
    CurrentSurfaceState {
        mode: guard.surface_machine.current().as_str().to_string(),
        target: guard.current_target.clone(),
    }
}

#[tauri::command]
pub fn get_proof_state(app: tauri::AppHandle) -> Result<ProofStatePayload, String> {
    proof_harness::ensure_proof_mode(&app)?;
    proof_harness::capture_state(&app)
}

#[tauri::command]
pub fn run_proof_command(
    app: tauri::AppHandle,
    command: String,
) -> Result<ProofStatePayload, String> {
    let command =
        ProofCommand::parse(&command).ok_or_else(|| format!("unknown proof command: {command}"))?;
    proof_harness::run_command(&app, command)
}

fn validate_surface_target(
    mode: SurfaceMode,
    target: SurfaceTarget,
) -> Result<SurfaceTarget, String> {
    if mode == SurfaceMode::Hidden {
        return Err("set_surface_mode only supports visible surfaces".into());
    }

    if target.mode() != mode {
        return Err(format!(
            "surface target '{}' is not valid for mode '{}'",
            target_label(&target),
            mode.as_str()
        ));
    }

    match &target {
        SurfaceTarget::Provider { provider_id } if !is_supported_provider_id(provider_id) => {
            return Err(format!("unsupported provider target: {provider_id}"));
        }
        SurfaceTarget::Settings { tab } if !is_supported_settings_tab(tab) => {
            return Err(format!("unsupported settings target: {tab}"));
        }
        _ => {}
    }

    Ok(target)
}

fn target_label(target: &SurfaceTarget) -> String {
    match target {
        SurfaceTarget::Summary => "summary".into(),
        SurfaceTarget::Dashboard => "dashboard".into(),
        SurfaceTarget::Provider { provider_id } => format!("provider:{provider_id}"),
        SurfaceTarget::Settings { tab } => format!("settings:{tab}"),
    }
}

// ── Provider refresh commands ────────────────────────────────────────

/// Instantiate the concrete provider for a given ID.
fn create_provider(id: ProviderId) -> Box<dyn Provider> {
    match id {
        ProviderId::Claude => Box::new(ClaudeProvider::new()),
        ProviderId::Codex => Box::new(CodexProvider::new()),
        ProviderId::Cursor => Box::new(CursorProvider::new()),
        ProviderId::Gemini => Box::new(GeminiProvider::new()),
        ProviderId::Copilot => Box::new(CopilotProvider::new()),
        ProviderId::Antigravity => Box::new(AntigravityProvider::new()),
        ProviderId::Factory => Box::new(FactoryProvider::new()),
        ProviderId::Zai => Box::new(ZaiProvider::new()),
        ProviderId::Kiro => Box::new(KiroProvider::new()),
        ProviderId::VertexAI => Box::new(VertexAIProvider::new()),
        ProviderId::Augment => Box::new(AugmentProvider::new()),
        ProviderId::MiniMax => Box::new(MiniMaxProvider::new()),
        ProviderId::OpenCode => Box::new(OpenCodeProvider::new()),
        ProviderId::Kimi => Box::new(KimiProvider::new()),
        ProviderId::KimiK2 => Box::new(KimiK2Provider::new()),
        ProviderId::Amp => Box::new(AmpProvider::new()),
        ProviderId::Warp => Box::new(WarpProvider::new()),
        ProviderId::Ollama => Box::new(OllamaProvider::new()),
        ProviderId::OpenRouter => Box::new(OpenRouterProvider::new()),
        ProviderId::Synthetic => Box::new(SyntheticProvider::new()),
        ProviderId::JetBrains => Box::new(JetBrainsProvider::new()),
        ProviderId::Alibaba => Box::new(AlibabaProvider::new()),
        ProviderId::NanoGPT => Box::new(NanoGPTProvider::new()),
        ProviderId::Infini => Box::new(InfiniProvider::default()),
    }
}

/// Build a `FetchContext` for a provider using persisted cookies/keys.
fn build_fetch_context(
    id: ProviderId,
    cookies: &ManualCookies,
    api_keys: &ApiKeys,
) -> FetchContext {
    let manual_cookie = cookies.get(id.cli_name()).map(|s| s.to_string());

    // Try browser cookie extraction as fallback when no manual cookie is set.
    // On non-Windows this is a harmless no-op that returns an error.
    let cookie_header = manual_cookie.or_else(|| {
        id.cookie_domain().and_then(|domain| {
            codexbar::browser::cookies::get_cookie_header(domain)
                .ok()
                .filter(|h| !h.is_empty())
        })
    });

    let api_key = api_keys.get(id.cli_name()).map(|s| s.to_string());

    FetchContext {
        manual_cookie_header: cookie_header,
        api_key,
        ..FetchContext::default()
    }
}

const PROVIDER_FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// Core refresh logic, usable from both the Tauri command and tray menu actions.
pub(crate) async fn do_refresh_providers(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<Mutex<AppState>>();

    {
        let mut guard = state.lock().map_err(|e| e.to_string())?;
        if guard.is_refreshing {
            return Ok(());
        }
        guard.is_refreshing = true;
        guard.provider_cache.clear();
    }

    events::emit_refresh_started(app);

    // Load settings and credential stores once, outside the hot loop.
    let settings = Settings::load();
    let enabled_ids = settings.get_enabled_provider_ids();
    let manual_cookies = ManualCookies::load();
    let api_keys = ApiKeys::load();

    // Spawn one task per enabled provider.
    let mut handles = Vec::with_capacity(enabled_ids.len());

    for id in &enabled_ids {
        let id = *id;
        let app_handle = app.clone();
        let ctx = build_fetch_context(id, &manual_cookies, &api_keys);

        handles.push(tokio::spawn(async move {
            let provider = create_provider(id);

            let snapshot = match tokio::time::timeout(
                PROVIDER_FETCH_TIMEOUT,
                provider.fetch_usage(&ctx),
            )
            .await
            {
                Ok(Ok(result)) => ProviderUsageSnapshot::from_fetch_result(id, &result),
                Ok(Err(e)) => ProviderUsageSnapshot::from_error(id, e.to_string()),
                Err(_) => ProviderUsageSnapshot::from_error(id, "Timeout".to_string()),
            };

            // Emit per-provider update event.
            events::emit_provider_updated(&app_handle, &snapshot);

            // Append to the cache.
            let st = app_handle.state::<Mutex<AppState>>();
            if let Ok(mut guard) = st.lock() {
                guard.provider_cache.push(snapshot);
            }
        }));
    }

    // Await all tasks.
    for handle in handles {
        let _ = handle.await;
    }

    // Finalise.
    let error_count = {
        let mut guard = state.lock().map_err(|e| e.to_string())?;
        guard.is_refreshing = false;
        guard
            .provider_cache
            .iter()
            .filter(|s| s.error.is_some())
            .count()
    };

    events::emit_refresh_complete(app, enabled_ids.len(), error_count);

    Ok(())
}

#[tauri::command]
pub async fn refresh_providers(app: tauri::AppHandle) -> Result<(), String> {
    do_refresh_providers(&app).await
}

#[tauri::command]
pub fn get_cached_providers(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Vec<ProviderUsageSnapshot> {
    state
        .lock()
        .map(|guard| guard.provider_cache.clone())
        .unwrap_or_default()
}

// ── Credential store commands ─────────────────────────────────────────

/// Bridge-friendly API key info (secrets masked).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyInfoBridge {
    pub provider_id: String,
    pub provider: String,
    pub masked_key: String,
    pub saved_at: String,
    pub label: Option<String>,
}

/// Bridge-friendly saved cookie info.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieInfoBridge {
    pub provider_id: String,
    pub provider: String,
    pub saved_at: String,
}

/// Bridge-friendly provider config info for the API keys tab.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyProviderInfoBridge {
    pub id: String,
    pub display_name: String,
    pub env_var: Option<String>,
    pub help: Option<String>,
    pub dashboard_url: Option<String>,
}

/// App metadata for the About tab.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfoBridge {
    pub name: String,
    pub version: String,
    pub build_number: String,
    pub update_channel: String,
    pub tagline: String,
}

#[tauri::command]
pub fn get_api_keys() -> Vec<ApiKeyInfoBridge> {
    let keys = ApiKeys::load();
    keys.get_all_for_display()
        .into_iter()
        .map(|info| ApiKeyInfoBridge {
            provider_id: info.provider_id,
            provider: info.provider,
            masked_key: info.masked_key,
            saved_at: info.saved_at,
            label: info.label,
        })
        .collect()
}

#[tauri::command]
pub fn get_api_key_providers() -> Vec<ApiKeyProviderInfoBridge> {
    codexbar::settings::get_api_key_providers()
        .into_iter()
        .map(|p| ApiKeyProviderInfoBridge {
            id: p.id.cli_name().to_string(),
            display_name: p.name.to_string(),
            env_var: p.api_key_env_var.map(|s| s.to_string()),
            help: p.api_key_help.map(|s| s.to_string()),
            dashboard_url: p.dashboard_url.map(|s| s.to_string()),
        })
        .collect()
}

#[tauri::command]
pub fn set_api_key(
    provider_id: String,
    api_key: String,
    label: Option<String>,
) -> Result<Vec<ApiKeyInfoBridge>, String> {
    let mut keys = ApiKeys::load();
    keys.set(&provider_id, &api_key, label.as_deref());
    keys.save().map_err(|e| e.to_string())?;
    Ok(get_api_keys())
}

#[tauri::command]
pub fn remove_api_key(provider_id: String) -> Result<Vec<ApiKeyInfoBridge>, String> {
    let mut keys = ApiKeys::load();
    keys.remove(&provider_id);
    keys.save().map_err(|e| e.to_string())?;
    Ok(get_api_keys())
}

#[tauri::command]
pub fn get_manual_cookies() -> Vec<CookieInfoBridge> {
    let cookies = ManualCookies::load();
    cookies
        .get_all_for_display()
        .into_iter()
        .map(|info| CookieInfoBridge {
            provider_id: info.provider_id,
            provider: info.provider,
            saved_at: info.saved_at,
        })
        .collect()
}

#[tauri::command]
pub fn set_manual_cookie(
    provider_id: String,
    cookie_header: String,
) -> Result<Vec<CookieInfoBridge>, String> {
    let mut cookies = ManualCookies::load();
    cookies.set(&provider_id, &cookie_header);
    cookies.save().map_err(|e| e.to_string())?;
    Ok(get_manual_cookies())
}

#[tauri::command]
pub fn remove_manual_cookie(provider_id: String) -> Result<Vec<CookieInfoBridge>, String> {
    let mut cookies = ManualCookies::load();
    cookies.remove(&provider_id);
    cookies.save().map_err(|e| e.to_string())?;
    Ok(get_manual_cookies())
}

#[tauri::command]
pub fn get_app_info() -> AppInfoBridge {
    let settings = Settings::load();
    AppInfoBridge {
        name: "CodexBar".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_number: option_env!("BUILD_NUMBER").unwrap_or("dev").to_string(),
        update_channel: update_channel_label(settings.update_channel).to_string(),
        tagline: "May your tokens never run out—keep agent limits in view.".to_string(),
    }
}

// ── Updater commands ─────────────────────────────────────────────────

#[tauri::command]
pub fn get_update_state(state: tauri::State<'_, Mutex<AppState>>) -> UpdateStatePayload {
    state
        .lock()
        .map(|guard| guard.update_payload())
        .unwrap_or_else(|_| UpdateState::default().to_payload())
}

#[tauri::command]
pub async fn check_for_updates(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<UpdateStatePayload, String> {
    // Guard: skip if already checking or downloading.
    {
        let mut guard = state.lock().map_err(|e| e.to_string())?;
        match guard.update_state {
            UpdateState::Checking | UpdateState::Downloading(_) => {
                return Ok(guard.update_payload());
            }
            _ => {}
        }
        guard.update_state = UpdateState::Checking;
        guard.update_info = None;
        guard.installer_path = None;
    }

    let checking_payload = {
        let guard = state.lock().map_err(|e| e.to_string())?;
        guard.update_payload()
    };
    events::emit_update_state_changed(&app, &checking_payload);

    let settings = Settings::load();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        codexbar::updater::check_for_updates_with_channel(settings.update_channel),
    )
    .await;

    let (new_state, new_info) = match result {
        Ok(Some(info)) => (UpdateState::Available(info.version.clone()), Some(info)),
        Ok(None) => (UpdateState::Idle, None),
        Err(_) => (
            UpdateState::Error("Update check timed out".to_string()),
            None,
        ),
    };

    let payload = {
        let mut guard = state.lock().map_err(|e| e.to_string())?;
        guard.update_state = new_state;
        guard.update_info = new_info;
        guard.update_payload()
    };
    events::emit_update_state_changed(&app, &payload);

    Ok(payload)
}

#[tauri::command]
pub async fn download_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<UpdateStatePayload, String> {
    // Validate current state and extract info.
    let info = {
        let guard = state.lock().map_err(|e| e.to_string())?;
        match &guard.update_state {
            UpdateState::Available(_) | UpdateState::Error(_) => {}
            UpdateState::Downloading(_) => return Ok(guard.update_payload()),
            _ => return Err("No update available to download".to_string()),
        }
        guard
            .update_info
            .clone()
            .ok_or("No update information available")?
    };

    if !info.supports_auto_download() {
        return Err(
            "This update does not support automatic download. Open the release page instead."
                .to_string(),
        );
    }

    // Set initial downloading state.
    let initial_payload = {
        let mut guard = state.lock().map_err(|e| e.to_string())?;
        guard.update_state = UpdateState::Downloading(0.0);
        guard.update_payload()
    };
    events::emit_update_state_changed(&app, &initial_payload);

    // Spawn background download with progress events.
    let app_handle = app.clone();
    tokio::spawn(async move {
        let (tx, mut rx) = tokio::sync::watch::channel(codexbar::updater::UpdateState::Available);

        let info_for_download = info.clone();
        let download_handle = tokio::spawn(async move {
            codexbar::updater::download_update(&info_for_download, tx).await
        });

        // Progress watcher: emit events as download progresses.
        let app_for_progress = app_handle.clone();
        let progress_handle = tokio::spawn(async move {
            while rx.changed().await.is_ok() {
                let backend_state = rx.borrow().clone();
                if let codexbar::updater::UpdateState::Downloading(progress) = backend_state {
                    let st = app_for_progress.state::<Mutex<AppState>>();
                    let payload = {
                        let mut guard = st.lock().unwrap();
                        guard.update_state = UpdateState::Downloading(progress);
                        guard.update_payload()
                    };
                    events::emit_update_state_changed(&app_for_progress, &payload);
                }
            }
        });

        // Wait for download to complete.
        let final_payload = match download_handle.await {
            Ok(Ok(path)) => {
                let st = app_handle.state::<Mutex<AppState>>();
                let mut guard = st.lock().unwrap();
                guard.update_state = UpdateState::Ready;
                guard.installer_path = Some(path);
                guard.update_payload()
            }
            Ok(Err(e)) => {
                let st = app_handle.state::<Mutex<AppState>>();
                let mut guard = st.lock().unwrap();
                guard.update_state = UpdateState::Error(e);
                guard.update_payload()
            }
            Err(join_err) => {
                let st = app_handle.state::<Mutex<AppState>>();
                let mut guard = st.lock().unwrap();
                guard.update_state =
                    UpdateState::Error(format!("Download task failed: {join_err}"));
                guard.update_payload()
            }
        };
        events::emit_update_state_changed(&app_handle, &final_payload);
        progress_handle.abort();
    });

    Ok(initial_payload)
}

#[tauri::command]
pub fn apply_update(state: tauri::State<'_, Mutex<AppState>>) -> Result<(), String> {
    let path = {
        let guard = state.lock().map_err(|e| e.to_string())?;
        guard
            .installer_path
            .clone()
            .ok_or("No downloaded update available to apply")?
    };
    // Spawns installer and exits the process.
    codexbar::updater::apply_update(&path)
}

#[tauri::command]
pub fn dismiss_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<UpdateStatePayload, String> {
    let payload = {
        let mut guard = state.lock().map_err(|e| e.to_string())?;
        guard.update_state = UpdateState::Idle;
        guard.update_info = None;
        guard.installer_path = None;
        guard.update_payload()
    };
    events::emit_update_state_changed(&app, &payload);
    Ok(payload)
}

#[tauri::command]
pub fn open_release_page(state: tauri::State<'_, Mutex<AppState>>) -> Result<(), String> {
    let url = {
        let guard = state.lock().map_err(|e| e.to_string())?;
        guard
            .update_info
            .as_ref()
            .map(|info| info.release_url.clone())
            .ok_or("No update information available")?
    };
    open_url_in_browser(&url)
}

fn open_url_in_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", url])
            .spawn()
            .map_err(|e| format!("Failed to open URL: {e}"))?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let opener = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        std::process::Command::new(opener)
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{bridge_commands, bridge_events, validate_surface_target};
    use crate::surface::SurfaceMode;
    use crate::surface_target::SurfaceTarget;

    #[test]
    fn validate_surface_target_accepts_matching_target() {
        let target = validate_surface_target(
            SurfaceMode::Settings,
            SurfaceTarget::Settings {
                tab: "apiKeys".into(),
            },
        )
        .unwrap();

        assert_eq!(
            target,
            SurfaceTarget::Settings {
                tab: "apiKeys".into()
            }
        );
    }

    #[test]
    fn validate_surface_target_rejects_mismatched_target() {
        let error = validate_surface_target(
            SurfaceMode::TrayPanel,
            SurfaceTarget::Settings {
                tab: "apiKeys".into(),
            },
        )
        .unwrap_err();

        assert!(error.contains("not valid for mode 'trayPanel'"));
    }

    #[test]
    fn validate_surface_target_rejects_hidden_mode() {
        let error =
            validate_surface_target(SurfaceMode::Hidden, SurfaceTarget::Summary).unwrap_err();

        assert!(error.contains("only supports visible surfaces"));
    }

    #[test]
    fn validate_surface_target_rejects_unknown_provider() {
        let error = validate_surface_target(
            SurfaceMode::PopOut,
            SurfaceTarget::Provider {
                provider_id: "not-a-provider".into(),
            },
        )
        .unwrap_err();

        assert!(error.contains("unsupported provider target"));
    }

    #[test]
    fn validate_surface_target_rejects_unknown_settings_tab() {
        let error = validate_surface_target(
            SurfaceMode::Settings,
            SurfaceTarget::Settings {
                tab: "security".into(),
            },
        )
        .unwrap_err();

        assert!(error.contains("unsupported settings target"));
    }

    #[test]
    fn bootstrap_contract_lists_current_surface_commands() {
        let ids = bridge_commands()
            .into_iter()
            .map(|descriptor| descriptor.id)
            .collect::<Vec<_>>();

        assert!(ids.contains(&"set_surface_mode"));
        assert!(ids.contains(&"get_current_surface_mode"));
        assert!(ids.contains(&"get_current_surface_state"));
        assert!(ids.contains(&"get_app_info"));
        assert!(!ids.contains(&"get_proof_config"));
    }

    #[test]
    fn bootstrap_contract_lists_surface_mode_changed_event() {
        let ids = bridge_events()
            .into_iter()
            .map(|descriptor| descriptor.id)
            .collect::<Vec<_>>();

        assert!(ids.contains(&"surface-mode-changed"));
    }
}
