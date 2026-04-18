use std::collections::HashSet;
use std::sync::Mutex;

use codexbar::core::{
    FetchContext, OpenAIDashboardCacheStore, ProviderFetchResult, ProviderId, RateWindow,
    TokenAccount, TokenAccountStore, TokenAccountSupport, instantiate_provider,
};
use codexbar::cost_scanner::get_daily_cost_history;
use codexbar::locale;
use codexbar::settings::{
    ApiKeys, Language, ManualCookies, MetricPreference, Settings, ThemePreference, TrayIconMode,
    UpdateChannel,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{Emitter, Manager};

use crate::events;
use crate::proof_harness::{self, ProofCommand, ProofStatePayload};
use crate::state::{AppState, UpdateState, UpdateStatePayload};
use crate::surface::SurfaceMode;
use crate::surface_target::SurfaceTarget;

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

/// Pace prediction snapshot for tray/bridge display.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaceSnapshot {
    pub stage: &'static str,
    pub delta_percent: f64,
    pub will_last_to_reset: bool,
    pub eta_seconds: Option<f64>,
    pub expected_used_percent: f64,
    pub actual_used_percent: f64,
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
    pub pace: Option<PaceSnapshot>,
    pub account_organization: Option<String>,
    pub tray_status_label: Option<String>,
}

fn pace_stage_str(stage: codexbar::core::PaceStage) -> &'static str {
    use codexbar::core::PaceStage;
    match stage {
        PaceStage::OnTrack => "on_track",
        PaceStage::SlightlyAhead => "slightly_ahead",
        PaceStage::Ahead => "ahead",
        PaceStage::FarAhead => "far_ahead",
        PaceStage::SlightlyBehind => "slightly_behind",
        PaceStage::Behind => "behind",
        PaceStage::FarBehind => "far_behind",
    }
}

impl ProviderUsageSnapshot {
    fn from_fetch_result(id: ProviderId, result: &ProviderFetchResult) -> Self {
        let usage = &result.usage;

        let pace =
            codexbar::core::UsagePace::weekly(&usage.primary, None, 10080).map(|p| PaceSnapshot {
                stage: pace_stage_str(p.stage),
                delta_percent: p.delta_percent,
                will_last_to_reset: p.will_last_to_reset,
                eta_seconds: p.eta_seconds,
                expected_used_percent: p.expected_used_percent,
                actual_used_percent: p.actual_used_percent,
            });

        let tray_status_label = {
            let pct = format!("{:.0}%", usage.primary.used_percent);
            if let Some(desc) = &usage.primary.reset_description {
                Some(format!("{pct} • resets in {desc}"))
            } else {
                Some(pct)
            }
        };

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
            pace,
            account_organization: usage.account_organization.clone(),
            tray_status_label,
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
            pace: None,
            account_organization: None,
            tray_status_label: None,
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
    start_minimized: bool,
    show_notifications: bool,
    sound_enabled: bool,
    sound_volume: u8,
    high_usage_threshold: f64,
    critical_usage_threshold: f64,
    tray_icon_mode: &'static str,
    switcher_shows_icons: bool,
    menu_bar_shows_highest_usage: bool,
    menu_bar_shows_percent: bool,
    show_as_used: bool,
    show_credits_extra_usage: bool,
    show_all_token_accounts_in_menu: bool,
    surprise_animations: bool,
    enable_animations: bool,
    reset_time_relative: bool,
    menu_bar_display_mode: String,
    hide_personal_info: bool,
    update_channel: &'static str,
    auto_download_updates: bool,
    install_updates_on_quit: bool,
    global_shortcut: String,
    ui_language: &'static str,
    theme: &'static str,
    claude_avoid_keychain_prompts: bool,
    disable_keychain_access: bool,
    show_debug_settings: bool,
    provider_metrics: std::collections::HashMap<String, &'static str>,
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

        let provider_metrics = settings
            .provider_metrics
            .into_iter()
            .map(|(k, v)| (k, metric_preference_label(v)))
            .collect();

        Self {
            enabled_providers,
            refresh_interval_secs: settings.refresh_interval_secs,
            start_at_login: settings.start_at_login,
            start_minimized: settings.start_minimized,
            show_notifications: settings.show_notifications,
            sound_enabled: settings.sound_enabled,
            sound_volume: settings.sound_volume,
            high_usage_threshold: settings.high_usage_threshold,
            critical_usage_threshold: settings.critical_usage_threshold,
            tray_icon_mode: tray_icon_mode_label(settings.tray_icon_mode),
            switcher_shows_icons: settings.switcher_shows_icons,
            menu_bar_shows_highest_usage: settings.menu_bar_shows_highest_usage,
            menu_bar_shows_percent: settings.menu_bar_shows_percent,
            show_as_used: settings.show_as_used,
            show_credits_extra_usage: settings.show_credits_extra_usage,
            show_all_token_accounts_in_menu: settings.show_all_token_accounts_in_menu,
            surprise_animations: settings.surprise_animations,
            enable_animations: settings.enable_animations,
            reset_time_relative: settings.reset_time_relative,
            menu_bar_display_mode: settings.menu_bar_display_mode,
            hide_personal_info: settings.hide_personal_info,
            update_channel: update_channel_label(settings.update_channel),
            auto_download_updates: settings.auto_download_updates,
            install_updates_on_quit: settings.install_updates_on_quit,
            global_shortcut: settings.global_shortcut,
            ui_language: language_label(settings.ui_language),
            theme: theme_label(settings.theme),
            claude_avoid_keychain_prompts: settings.claude_avoid_keychain_prompts,
            disable_keychain_access: settings.disable_keychain_access,
            show_debug_settings: settings.show_debug_settings,
            provider_metrics,
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
            id: "list_detected_browsers",
            description: "Return browsers detected on this machine that CodexBar can import cookies from.",
        },
        BridgeCommandDescriptor {
            id: "import_browser_cookies",
            description: "Extract and persist cookies for a provider from a detected browser.",
        },
        BridgeCommandDescriptor {
            id: "get_app_info",
            description: "Read app metadata displayed in the shell About surface.",
        },
        BridgeCommandDescriptor {
            id: "get_provider_chart_data",
            description: "Return cost history, credits history, and usage breakdown chart data for a provider.",
        },
        BridgeCommandDescriptor {
            id: "get_token_account_providers",
            description: "List providers that support token accounts (multi-account session/API tokens).",
        },
        BridgeCommandDescriptor {
            id: "get_token_accounts",
            description: "Load token accounts for a provider.",
        },
        BridgeCommandDescriptor {
            id: "add_token_account",
            description: "Add a token account for a provider.",
        },
        BridgeCommandDescriptor {
            id: "remove_token_account",
            description: "Remove a token account by UUID.",
        },
        BridgeCommandDescriptor {
            id: "set_active_token_account",
            description: "Set the active token account for a provider.",
        },
        BridgeCommandDescriptor {
            id: "reorder_providers",
            description: "Persist a new provider display order and return refreshed summaries.",
        },
        BridgeCommandDescriptor {
            id: "set_provider_cookie_source",
            description: "Set the preferred cookie/credential source for a provider.",
        },
        BridgeCommandDescriptor {
            id: "get_provider_cookie_source",
            description: "Read the preferred cookie/credential source for a provider.",
        },
        BridgeCommandDescriptor {
            id: "set_provider_region",
            description: "Set the preferred API region for a provider (Alibaba, Z.ai, MiniMax).",
        },
        BridgeCommandDescriptor {
            id: "get_provider_region",
            description: "Read the preferred API region for a provider.",
        },
        BridgeCommandDescriptor {
            id: "get_provider_cookie_source_options",
            description: "List supported cookie/credential source options for a provider.",
        },
        BridgeCommandDescriptor {
            id: "get_provider_region_options",
            description: "List supported API region options for a provider (empty if none).",
        },
        BridgeCommandDescriptor {
            id: "get_gemini_cli_signed_in",
            description: "Detect whether the Gemini CLI is signed in locally.",
        },
        BridgeCommandDescriptor {
            id: "get_vertexai_status",
            description: "Detect VertexAI application default credentials.",
        },
        BridgeCommandDescriptor {
            id: "list_jetbrains_detected_ides",
            description: "List detected JetBrains/Google IDE config directories.",
        },
        BridgeCommandDescriptor {
            id: "set_jetbrains_ide_path",
            description: "Persist an explicit JetBrains IDE config path override.",
        },
        BridgeCommandDescriptor {
            id: "get_kiro_status",
            description: "Detect availability of the Kiro CLI.",
        },
        BridgeCommandDescriptor {
            id: "register_global_shortcut",
            description: "Register a global keyboard shortcut that emits `global-shortcut-triggered` events.",
        },
        BridgeCommandDescriptor {
            id: "unregister_global_shortcut",
            description: "Unregister the currently-captured global shortcut.",
        },
        BridgeCommandDescriptor {
            id: "is_remote_session",
            description: "Return true when running inside an SSH or Windows Remote Desktop session.",
        },
        BridgeCommandDescriptor {
            id: "get_launch_block_reason",
            description: "Return a user-facing reason when the native shell should not launch (SSH/RDP).",
        },
        BridgeCommandDescriptor {
            id: "get_work_area_rect",
            description: "Return the current monitor's work area in physical pixels (excludes the taskbar / Dock / panel).",
        },
        BridgeCommandDescriptor {
            id: "play_notification_sound",
            description: "Play the short notification chime used after refreshes.",
        },
        BridgeCommandDescriptor {
            id: "open_provider_dashboard",
            description: "Open a provider's external dashboard URL in the default browser.",
        },
        BridgeCommandDescriptor {
            id: "open_provider_status_page",
            description: "Open a provider's external status page URL in the default browser.",
        },
        BridgeCommandDescriptor {
            id: "get_provider_detail",
            description: "Return the aggregated identity/usage/pace/cost snapshot backing the Settings provider detail pane.",
        },
        BridgeCommandDescriptor {
            id: "trigger_provider_login",
            description: "Trigger a provider's login flow (CLI-based where available).",
        },
        BridgeCommandDescriptor {
            id: "revoke_provider_credentials",
            description: "Revoke or remove stored credentials (API keys + manual cookies) for a provider.",
        },
        BridgeCommandDescriptor {
            id: "get_locale_strings",
            description: "Return every localized UI string for the requested language (or current language when None).",
        },
        BridgeCommandDescriptor {
            id: "set_ui_language",
            description: "Persist the UI language and emit `locale-changed` so frontends can refetch strings.",
        },
        BridgeCommandDescriptor {
            id: "open_path",
            description: "Open a filesystem path (file or folder) in the OS file manager.",
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
        BridgeEventDescriptor {
            id: "global-shortcut-triggered",
            description: "Emitted when the user-registered global shortcut (via register_global_shortcut) fires.",
        },
        BridgeEventDescriptor {
            id: "locale-changed",
            description: "Emitted when the persisted UI language changes. Payload: serialized language label.",
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

fn theme_label(theme: ThemePreference) -> &'static str {
    match theme {
        ThemePreference::Auto => "auto",
        ThemePreference::Light => "light",
        ThemePreference::Dark => "dark",
    }
}

fn parse_theme(s: &str) -> Option<ThemePreference> {
    match s {
        "auto" => Some(ThemePreference::Auto),
        "light" => Some(ThemePreference::Light),
        "dark" => Some(ThemePreference::Dark),
        _ => None,
    }
}

fn metric_preference_label(pref: MetricPreference) -> &'static str {
    match pref {
        MetricPreference::Automatic => "automatic",
        MetricPreference::Session => "session",
        MetricPreference::Weekly => "weekly",
        MetricPreference::Model => "model",
        MetricPreference::Credits => "credits",
        MetricPreference::Average => "average",
    }
}

fn parse_metric_preference(s: &str) -> Option<MetricPreference> {
    match s {
        "automatic" => Some(MetricPreference::Automatic),
        "session" => Some(MetricPreference::Session),
        "weekly" => Some(MetricPreference::Weekly),
        "model" => Some(MetricPreference::Model),
        "credits" => Some(MetricPreference::Credits),
        "average" => Some(MetricPreference::Average),
        _ => None,
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
    pub start_minimized: Option<bool>,
    pub show_notifications: Option<bool>,
    pub sound_enabled: Option<bool>,
    pub sound_volume: Option<u8>,
    pub high_usage_threshold: Option<f64>,
    pub critical_usage_threshold: Option<f64>,
    pub tray_icon_mode: Option<String>,
    pub switcher_shows_icons: Option<bool>,
    pub menu_bar_shows_highest_usage: Option<bool>,
    pub menu_bar_shows_percent: Option<bool>,
    pub show_as_used: Option<bool>,
    pub show_credits_extra_usage: Option<bool>,
    pub show_all_token_accounts_in_menu: Option<bool>,
    pub surprise_animations: Option<bool>,
    pub enable_animations: Option<bool>,
    pub reset_time_relative: Option<bool>,
    pub menu_bar_display_mode: Option<String>,
    pub hide_personal_info: Option<bool>,
    pub update_channel: Option<String>,
    pub auto_download_updates: Option<bool>,
    pub install_updates_on_quit: Option<bool>,
    pub global_shortcut: Option<String>,
    pub ui_language: Option<String>,
    pub theme: Option<String>,
    pub claude_avoid_keychain_prompts: Option<bool>,
    pub disable_keychain_access: Option<bool>,
    pub show_debug_settings: Option<bool>,
    /// Map of provider CLI name → metric preference label.
    pub provider_metrics: Option<std::collections::HashMap<String, String>>,
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
        settings.set_start_at_login(v).map_err(|e| e.to_string())?;
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
        && settings.ui_language != lang
    {
        settings.ui_language = lang;
        let _ = app.emit(events::LOCALE_CHANGED, language_label(lang));
    }
    if let Some(ref s) = patch.theme
        && let Some(theme) = parse_theme(s)
    {
        settings.theme = theme;
    }
    if let Some(v) = patch.start_minimized {
        settings.start_minimized = v;
    }
    if let Some(v) = patch.sound_enabled {
        settings.sound_enabled = v;
    }
    if let Some(v) = patch.sound_volume {
        settings.sound_volume = v;
    }
    if let Some(v) = patch.high_usage_threshold {
        settings.high_usage_threshold = v.clamp(0.0, 100.0);
    }
    if let Some(v) = patch.critical_usage_threshold {
        settings.critical_usage_threshold = v.clamp(0.0, 100.0);
    }
    if let Some(v) = patch.switcher_shows_icons {
        settings.switcher_shows_icons = v;
    }
    if let Some(v) = patch.menu_bar_shows_highest_usage {
        settings.menu_bar_shows_highest_usage = v;
    }
    if let Some(v) = patch.menu_bar_shows_percent {
        settings.menu_bar_shows_percent = v;
    }
    if let Some(v) = patch.show_credits_extra_usage {
        settings.show_credits_extra_usage = v;
    }
    if let Some(v) = patch.show_all_token_accounts_in_menu {
        settings.show_all_token_accounts_in_menu = v;
    }
    if let Some(v) = patch.auto_download_updates {
        settings.auto_download_updates = v;
    }
    if let Some(v) = patch.install_updates_on_quit {
        settings.install_updates_on_quit = v;
    }
    if let Some(v) = patch.claude_avoid_keychain_prompts {
        settings.claude_avoid_keychain_prompts = v;
    }
    if let Some(v) = patch.disable_keychain_access {
        settings.disable_keychain_access = v;
        if v {
            settings.claude_avoid_keychain_prompts = true;
        }
    }
    if let Some(v) = patch.show_debug_settings {
        settings.show_debug_settings = v;
    }
    if let Some(metrics_map) = patch.provider_metrics {
        for (provider, label) in metrics_map {
            if let Some(pref) = parse_metric_preference(&label) {
                settings.provider_metrics.insert(provider, pref);
            }
        }
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
            let provider = instantiate_provider(id);

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

    // Update tray menu labels, icon, and tooltip once after the full refresh cycle.
    {
        let cached = {
            let guard = state.lock().map_err(|e| e.to_string())?;
            guard.provider_cache.clone()
        };
        crate::tray_bridge::update_tray_status_items(app, &cached);
        crate::tray_bridge::update_tray_icon_and_tooltip(app, &cached);

        // Fire OS notifications for any usage-threshold crossings.
        let cli_map = codexbar::core::cli_name_map();
        if let Ok(mut guard) = state.lock() {
            for snapshot in &cached {
                if snapshot.error.is_none()
                    && let Some(&provider) = cli_map.get(snapshot.provider_id.as_str())
                {
                    guard.notification_manager.check_and_notify(
                        provider,
                        snapshot.primary.used_percent,
                        &settings,
                    );
                    guard.notification_manager.check_session_transition(
                        provider,
                        snapshot.primary.used_percent,
                        &settings,
                    );
                }
            }
        }
    }

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

// ── Token account commands ────────────────────────────────────────────

/// Bridge-friendly token account support descriptor for a provider.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenAccountSupportBridge {
    pub provider_id: String,
    pub display_name: String,
    pub title: String,
    pub subtitle: String,
    pub placeholder: String,
}

/// Bridge-friendly token account (token value is never exposed).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenAccountBridge {
    pub id: String,
    pub label: String,
    pub added_at: String,
    pub last_used: Option<String>,
    pub is_active: bool,
}

/// Bridge-friendly provider token accounts snapshot.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderTokenAccountsBridge {
    pub provider_id: String,
    pub support: TokenAccountSupportBridge,
    pub accounts: Vec<TokenAccountBridge>,
    pub active_index: usize,
}

fn format_token_account_date(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt: chrono::DateTime<chrono::Utc>| dt.format("%b %d, %Y").to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

fn build_provider_token_accounts(
    provider_id: ProviderId,
    support: &TokenAccountSupport,
    accounts: Vec<TokenAccount>,
    active_index: usize,
) -> ProviderTokenAccountsBridge {
    let support_bridge = TokenAccountSupportBridge {
        provider_id: provider_id.cli_name().to_string(),
        display_name: provider_id.display_name().to_string(),
        title: support.title.to_string(),
        subtitle: support.subtitle.to_string(),
        placeholder: support.placeholder.to_string(),
    };
    let account_bridges: Vec<TokenAccountBridge> = accounts
        .iter()
        .enumerate()
        .map(|(i, a)| TokenAccountBridge {
            id: a.id.to_string(),
            label: a.label.clone(),
            added_at: format_token_account_date(a.added_at),
            last_used: a.last_used.map(format_token_account_date),
            is_active: i == active_index,
        })
        .collect();
    ProviderTokenAccountsBridge {
        provider_id: provider_id.cli_name().to_string(),
        support: support_bridge,
        accounts: account_bridges,
        active_index,
    }
}

/// List all providers that support token accounts.
#[tauri::command]
pub fn get_token_account_providers() -> Vec<TokenAccountSupportBridge> {
    ProviderId::all()
        .iter()
        .filter_map(|&id| {
            TokenAccountSupport::for_provider(id).map(|s| TokenAccountSupportBridge {
                provider_id: id.cli_name().to_string(),
                display_name: id.display_name().to_string(),
                title: s.title.to_string(),
                subtitle: s.subtitle.to_string(),
                placeholder: s.placeholder.to_string(),
            })
        })
        .collect()
}

/// Load token accounts for a single provider.
#[tauri::command]
pub fn get_token_accounts(provider_id: String) -> Result<ProviderTokenAccountsBridge, String> {
    let id = ProviderId::from_cli_name(&provider_id)
        .ok_or_else(|| format!("Unknown provider: {provider_id}"))?;
    let support = TokenAccountSupport::for_provider(id)
        .ok_or_else(|| format!("Provider {provider_id} does not support token accounts"))?;
    let store = TokenAccountStore::new();
    let data = store.load_provider(id).map_err(|e| e.to_string())?;
    let active = data.clamped_active_index();
    Ok(build_provider_token_accounts(
        id,
        &support,
        data.accounts,
        active,
    ))
}

/// Add a token account for a provider.
#[tauri::command]
pub fn add_token_account(
    provider_id: String,
    label: String,
    token: String,
) -> Result<ProviderTokenAccountsBridge, String> {
    let id = ProviderId::from_cli_name(&provider_id)
        .ok_or_else(|| format!("Unknown provider: {provider_id}"))?;
    let support = TokenAccountSupport::for_provider(id)
        .ok_or_else(|| format!("Provider {provider_id} does not support token accounts"))?;
    let store = TokenAccountStore::new();
    let mut data = store.load_provider(id).map_err(|e| e.to_string())?;
    data.add_account(TokenAccount::new(label, token));
    store.save_provider(id, &data).map_err(|e| e.to_string())?;
    let active = data.clamped_active_index();
    Ok(build_provider_token_accounts(
        id,
        &support,
        data.accounts,
        active,
    ))
}

/// Remove a token account by UUID string.
#[tauri::command]
pub fn remove_token_account(
    provider_id: String,
    account_id: String,
) -> Result<ProviderTokenAccountsBridge, String> {
    let id = ProviderId::from_cli_name(&provider_id)
        .ok_or_else(|| format!("Unknown provider: {provider_id}"))?;
    let support = TokenAccountSupport::for_provider(id)
        .ok_or_else(|| format!("Provider {provider_id} does not support token accounts"))?;
    let uuid = uuid::Uuid::parse_str(&account_id).map_err(|e| e.to_string())?;
    let store = TokenAccountStore::new();
    let mut data = store.load_provider(id).map_err(|e| e.to_string())?;
    data.remove_account(uuid);
    store.save_provider(id, &data).map_err(|e| e.to_string())?;
    let active = data.clamped_active_index();
    Ok(build_provider_token_accounts(
        id,
        &support,
        data.accounts,
        active,
    ))
}

/// Set the active token account for a provider by UUID string.
#[tauri::command]
pub fn set_active_token_account(
    provider_id: String,
    account_id: String,
) -> Result<ProviderTokenAccountsBridge, String> {
    let id = ProviderId::from_cli_name(&provider_id)
        .ok_or_else(|| format!("Unknown provider: {provider_id}"))?;
    let support = TokenAccountSupport::for_provider(id)
        .ok_or_else(|| format!("Provider {provider_id} does not support token accounts"))?;
    let uuid = uuid::Uuid::parse_str(&account_id).map_err(|e| e.to_string())?;
    let store = TokenAccountStore::new();
    let mut data = store.load_provider(id).map_err(|e| e.to_string())?;
    data.set_active_by_id(uuid);
    store.save_provider(id, &data).map_err(|e| e.to_string())?;
    let active = data.clamped_active_index();
    Ok(build_provider_token_accounts(
        id,
        &support,
        data.accounts,
        active,
    ))
}

// ── Browser cookie import commands ────────────────────────────────────

/// Bridge-friendly detected browser entry.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedBrowserBridge {
    /// Stable key used when calling `import_browser_cookies`.
    pub browser_type: String,
    pub display_name: String,
    pub profile_count: usize,
}

/// List all browsers detected on this machine that CodexBar can read cookies from.
///
/// On non-Windows platforms (e.g. Linux CI) this returns an empty list because
/// DPAPI is unavailable; the UI should hide/disable the import button in that case.
#[tauri::command]
pub fn list_detected_browsers() -> Vec<DetectedBrowserBridge> {
    use codexbar::browser::detection::BrowserDetector;

    BrowserDetector::detect_all()
        .into_iter()
        .map(|b| DetectedBrowserBridge {
            browser_type: browser_type_key(b.browser_type).to_string(),
            display_name: b.browser_type.display_name().to_string(),
            profile_count: b.profiles.len(),
        })
        .collect()
}

/// Import cookies for `provider_id` from the named browser and persist them as
/// a manual-cookie override, replacing any existing entry for that provider.
///
/// `browser_type` must be one of the keys returned by `list_detected_browsers`
/// (e.g. `"chrome"`, `"edge"`, `"brave"`).
///
/// Returns the updated manual-cookies list on success.
#[tauri::command]
pub fn import_browser_cookies(
    provider_id: String,
    browser_type: String,
) -> Result<Vec<CookieInfoBridge>, String> {
    use codexbar::browser::cookies::{CookieError, CookieExtractor};
    use codexbar::browser::detection::BrowserDetector;
    use codexbar::core::ProviderId;

    // Resolve the provider to get its cookie domain.
    let pid = ProviderId::all()
        .iter()
        .find(|p| p.cli_name() == provider_id.as_str())
        .copied()
        .ok_or_else(|| format!("Unknown provider: {provider_id}"))?;

    let domain = pid
        .cookie_domain()
        .ok_or_else(|| format!("Provider '{provider_id}' does not use cookie authentication"))?;

    // Find the requested browser.
    let browsers = BrowserDetector::detect_all();
    let browser = browsers
        .into_iter()
        .find(|b| browser_type_key(b.browser_type) == browser_type.as_str())
        .ok_or_else(|| format!("Browser '{browser_type}' not found or not installed"))?;

    // Extract the cookie header.
    let cookies = CookieExtractor::extract_for_domain(&browser, domain).map_err(|e| match e {
        CookieError::Dpapi(msg) => format!("DPAPI error: {msg}"),
        other => other.to_string(),
    })?;

    if cookies.is_empty() {
        return Err(format!(
            "No cookies found for {domain} in {}. Make sure you are signed in to that site in the browser.",
            browser.browser_type.display_name()
        ));
    }

    let cookie_header = CookieExtractor::build_cookie_header(&cookies);

    // Persist as manual cookie.
    let mut manual = ManualCookies::load();
    manual.set(&provider_id, &cookie_header);
    manual.save().map_err(|e| e.to_string())?;

    Ok(get_manual_cookies())
}

/// Map `BrowserType` to a stable lowercase string key used in the IPC bridge.
fn browser_type_key(bt: codexbar::browser::detection::BrowserType) -> &'static str {
    use codexbar::browser::detection::BrowserType;
    match bt {
        BrowserType::Chrome => "chrome",
        BrowserType::Edge => "edge",
        BrowserType::Brave => "brave",
        BrowserType::Arc => "arc",
        BrowserType::Firefox => "firefox",
        BrowserType::Chromium => "chromium",
    }
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
        guard.last_update_check_ms = Some(chrono::Utc::now().timestamp_millis());
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

// ── Provider chart data ──────────────────────────────────────────────

/// A single (date, value) point for cost or credits history charts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyCostPoint {
    pub date: String,
    pub value: f64,
}

/// A single service's usage within a day for the stacked usage breakdown chart.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceUsagePoint {
    pub service: String,
    pub credits_used: f64,
}

/// One day's stacked usage breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsageBreakdown {
    pub day: String,
    pub services: Vec<ServiceUsagePoint>,
    pub total_credits_used: f64,
}

/// Full chart data bundle for one provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderChartData {
    pub provider_id: String,
    pub cost_history: Vec<DailyCostPoint>,
    pub credits_history: Vec<DailyCostPoint>,
    pub usage_breakdown: Vec<DailyUsageBreakdown>,
}

#[tauri::command]
pub fn get_provider_chart_data(
    provider_id: String,
    account_email: Option<String>,
) -> ProviderChartData {
    // Cost history (available for any provider that has local JSONL cost data)
    let raw_cost = get_daily_cost_history(&provider_id, 30);
    let cost_history: Vec<DailyCostPoint> = raw_cost
        .into_iter()
        .map(|(date, value)| DailyCostPoint { date, value })
        .collect();

    // Credits history + usage breakdown — only for codex/openai providers from dashboard cache
    let (credits_history, usage_breakdown) =
        load_openai_dashboard_chart_data(&provider_id, account_email.as_deref());

    ProviderChartData {
        provider_id,
        cost_history,
        credits_history,
        usage_breakdown,
    }
}

fn load_openai_dashboard_chart_data(
    provider_id: &str,
    account_email: Option<&str>,
) -> (Vec<DailyCostPoint>, Vec<DailyUsageBreakdown>) {
    // Only codex (openai) provider has dashboard cache data
    if provider_id != "codex" && provider_id != "openai" {
        return (Vec::new(), Vec::new());
    }

    let Some(account_email) = account_email else {
        return (Vec::new(), Vec::new());
    };

    let Some(cache) = OpenAIDashboardCacheStore::load() else {
        return (Vec::new(), Vec::new());
    };

    if !cache.account_email.eq_ignore_ascii_case(account_email) {
        return (Vec::new(), Vec::new());
    }

    let snapshot = &cache.snapshot;

    // Pick daily_breakdown if available, else usage_breakdown
    let breakdown_source = if !snapshot.daily_breakdown.is_empty() {
        &snapshot.daily_breakdown
    } else if !snapshot.usage_breakdown.is_empty() {
        &snapshot.usage_breakdown
    } else {
        return (Vec::new(), Vec::new());
    };

    let credits_history: Vec<DailyCostPoint> = breakdown_source
        .iter()
        .map(|d| DailyCostPoint {
            date: d.day.clone(),
            value: d.total_credits_used,
        })
        .collect();

    let usage_breakdown: Vec<DailyUsageBreakdown> = snapshot
        .usage_breakdown
        .iter()
        .map(|d| DailyUsageBreakdown {
            day: d.day.clone(),
            services: d
                .services
                .iter()
                .map(|s| ServiceUsagePoint {
                    service: s.service.clone(),
                    credits_used: s.credits_used,
                })
                .collect(),
            total_credits_used: d.total_credits_used,
        })
        .collect();

    (credits_history, usage_breakdown)
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

// ════════════════════════════════════════════════════════════════════════════════
// PHASE 4 — Provider ordering, cookie source, region, credential detection,
// global shortcut capture, session/environment introspection, quick actions.
// ════════════════════════════════════════════════════════════════════════════════

// ── Provider summaries + ordering ─────────────────────────────────────

/// Lightweight provider entry returned to the UI after a reorder.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSummary {
    pub id: String,
    pub display_name: String,
    pub enabled: bool,
    pub order: u32,
}

/// Canonicalise a requested provider order: keep requested ids that match a
/// real `ProviderId`, drop duplicates, and append any canonical ids that were
/// omitted (preserving their canonical order).
fn apply_provider_order(requested: &[String]) -> Vec<String> {
    let canonical: Vec<String> = ProviderId::all()
        .iter()
        .map(|p| p.cli_name().to_string())
        .collect();
    let valid: std::collections::HashSet<&str> = canonical.iter().map(|s| s.as_str()).collect();

    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut out: Vec<String> = Vec::with_capacity(canonical.len());

    for id in requested {
        if valid.contains(id.as_str()) && seen.insert(id.clone()) {
            out.push(id.clone());
        }
    }
    for id in &canonical {
        if seen.insert(id.clone()) {
            out.push(id.clone());
        }
    }

    out
}

/// Build `ProviderSummary` list honouring the persisted `provider_order`.
fn build_provider_summaries(settings: &Settings) -> Vec<ProviderSummary> {
    let order = if settings.provider_order.is_empty() {
        apply_provider_order(&[])
    } else {
        apply_provider_order(&settings.provider_order)
    };

    let by_id: std::collections::HashMap<String, &ProviderId> = ProviderId::all()
        .iter()
        .map(|p| (p.cli_name().to_string(), p))
        .collect();

    order
        .iter()
        .enumerate()
        .filter_map(|(idx, id)| {
            by_id.get(id).map(|p| ProviderSummary {
                id: id.clone(),
                display_name: p.display_name().to_string(),
                enabled: settings.enabled_providers.contains(id),
                order: idx as u32,
            })
        })
        .collect()
}

#[tauri::command]
pub fn reorder_providers(ids: Vec<String>) -> Result<Vec<ProviderSummary>, String> {
    let mut settings = Settings::load();
    settings.provider_order = apply_provider_order(&ids);
    settings.save().map_err(|e| e.to_string())?;
    Ok(build_provider_summaries(&settings))
}

// ── Per-provider cookie source + region ───────────────────────────────

/// Providers that expose a user-facing cookie-source picker, mapped to their
/// corresponding `Settings` field accessor.
fn provider_cookie_source_field<'a>(
    settings: &'a mut Settings,
    provider_id: &str,
) -> Option<&'a mut String> {
    match provider_id {
        "codex" => Some(&mut settings.codex_cookie_source),
        "claude" => Some(&mut settings.claude_cookie_source),
        "cursor" => Some(&mut settings.cursor_cookie_source),
        "opencode" => Some(&mut settings.opencode_cookie_source),
        "factory" => Some(&mut settings.factory_cookie_source),
        "alibaba" => Some(&mut settings.alibaba_cookie_source),
        "kimi" | "kimik2" => Some(&mut settings.kimi_cookie_source),
        "minimax" => Some(&mut settings.minimax_cookie_source),
        "augment" => Some(&mut settings.augment_cookie_source),
        "amp" => Some(&mut settings.amp_cookie_source),
        "ollama" => Some(&mut settings.ollama_cookie_source),
        _ => None,
    }
}

fn provider_cookie_source_lookup(settings: &Settings, provider_id: &str) -> Option<String> {
    let copy = match provider_id {
        "codex" => &settings.codex_cookie_source,
        "claude" => &settings.claude_cookie_source,
        "cursor" => &settings.cursor_cookie_source,
        "opencode" => &settings.opencode_cookie_source,
        "factory" => &settings.factory_cookie_source,
        "alibaba" => &settings.alibaba_cookie_source,
        "kimi" | "kimik2" => &settings.kimi_cookie_source,
        "minimax" => &settings.minimax_cookie_source,
        "augment" => &settings.augment_cookie_source,
        "amp" => &settings.amp_cookie_source,
        "ollama" => &settings.ollama_cookie_source,
        _ => return None,
    };
    Some(copy.clone())
}

fn provider_cookie_source_set(
    settings: &mut Settings,
    provider_id: &str,
    source: String,
) -> Result<(), String> {
    let field = provider_cookie_source_field(settings, provider_id)
        .ok_or_else(|| format!("Provider '{provider_id}' does not expose a cookie source"))?;
    *field = source;
    Ok(())
}

#[tauri::command]
pub fn set_provider_cookie_source(provider_id: String, source: String) -> Result<(), String> {
    let mut settings = Settings::load();
    provider_cookie_source_set(&mut settings, &provider_id, source)?;
    settings.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_provider_cookie_source(provider_id: String) -> Result<Option<String>, String> {
    Ok(provider_cookie_source_lookup(
        &Settings::load(),
        &provider_id,
    ))
}

fn provider_region_field<'a>(
    settings: &'a mut Settings,
    provider_id: &str,
) -> Option<&'a mut String> {
    match provider_id {
        "alibaba" => Some(&mut settings.alibaba_api_region),
        "zai" => Some(&mut settings.zai_api_region),
        "minimax" => Some(&mut settings.minimax_api_region),
        _ => None,
    }
}

fn provider_region_lookup(settings: &Settings, provider_id: &str) -> Option<String> {
    let copy = match provider_id {
        "alibaba" => &settings.alibaba_api_region,
        "zai" => &settings.zai_api_region,
        "minimax" => &settings.minimax_api_region,
        _ => return None,
    };
    Some(copy.clone())
}

fn provider_region_set(
    settings: &mut Settings,
    provider_id: &str,
    region: String,
) -> Result<(), String> {
    let field = provider_region_field(settings, provider_id)
        .ok_or_else(|| format!("Provider '{provider_id}' does not have a region picker"))?;
    *field = region;
    Ok(())
}

#[tauri::command]
pub fn set_provider_region(provider_id: String, region: String) -> Result<(), String> {
    let mut settings = Settings::load();
    provider_region_set(&mut settings, &provider_id, region)?;
    settings.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_provider_region(provider_id: String) -> Result<Option<String>, String> {
    Ok(provider_region_lookup(&Settings::load(), &provider_id))
}

// ── Phase 6c — cookie source & region option catalogs ────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CookieSourceOption {
    pub value: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegionOption {
    pub value: String,
    pub label: String,
}

fn cookie_option(
    lang: Language,
    value: &str,
    auto_desc: &str,
    manual_desc: &str,
    off_desc: Option<&str>,
) -> CookieSourceOption {
    let (label, description) = match value {
        "auto" => (
            locale::get_text(lang, locale::LocaleKey::Automatic).to_string(),
            auto_desc.to_string(),
        ),
        "manual" => (
            locale::get_text(lang, locale::LocaleKey::CookieSourceManual).to_string(),
            manual_desc.to_string(),
        ),
        "off" => (
            locale::get_text(lang, locale::LocaleKey::ProviderDisabled).to_string(),
            off_desc.unwrap_or("").to_string(),
        ),
        other => (other.to_string(), String::new()),
    };
    CookieSourceOption {
        value: value.to_string(),
        label,
        description: if description.is_empty() {
            None
        } else {
            Some(description)
        },
    }
}

/// Returns the catalog of cookie source options for a given provider,
/// mirroring the `egui` ComboBox choices in `preferences.rs`.
/// Empty vec means the provider does not expose a cookie-source picker.
pub fn cookie_source_options_for(provider_id: &str, lang: Language) -> Vec<CookieSourceOption> {
    match provider_id {
        "codex" => vec![
            cookie_option(
                lang,
                "auto",
                locale::get_text(lang, locale::LocaleKey::ProviderCodexAutoImportHelp),
                "Paste a Cookie header from a chatgpt.com request.",
                Some("Disable OpenAI dashboard cookie usage."),
            ),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a Cookie header from a chatgpt.com request.",
                None,
            ),
            cookie_option(
                lang,
                "off",
                "",
                "",
                Some("Disable OpenAI dashboard cookie usage."),
            ),
        ],
        "claude" => vec![
            cookie_option(
                lang,
                "auto",
                locale::get_text(lang, locale::LocaleKey::ProviderClaudeCookiesHelp),
                "",
                None,
            ),
            cookie_option(
                lang,
                "manual",
                "",
                locale::get_text(lang, locale::LocaleKey::ProviderClaudeCookiesHelp),
                None,
            ),
        ],
        "cursor" => vec![
            cookie_option(
                lang,
                "auto",
                locale::get_text(lang, locale::LocaleKey::ProviderCursorCookieSourceHelp),
                "",
                None,
            ),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a Cookie header from a cursor.com request.",
                None,
            ),
        ],
        "opencode" => vec![
            cookie_option(
                lang,
                "auto",
                "Automatic imports browser cookies from opencode.ai.",
                "",
                None,
            ),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a Cookie header from the billing page.",
                None,
            ),
        ],
        "factory" => vec![
            cookie_option(
                lang,
                "auto",
                "Automatic imports browser cookies and WorkOS sessions.",
                "",
                None,
            ),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a Cookie header from Factory.",
                None,
            ),
        ],
        "alibaba" => vec![
            cookie_option(
                lang,
                "auto",
                "Automatic imports browser cookies from Model Studio / Bailian.",
                "",
                None,
            ),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a Cookie header from Model Studio or Bailian.",
                None,
            ),
        ],
        "kimi" | "kimik2" => vec![
            cookie_option(lang, "auto", "Automatic imports browser cookies.", "", None),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a cookie header or the kimi-auth token value.",
                None,
            ),
            cookie_option(lang, "off", "", "", Some("Kimi cookies are disabled.")),
        ],
        "minimax" => vec![
            cookie_option(
                lang,
                "auto",
                "Automatic imports browser cookies and Coding Plan tokens.",
                "",
                None,
            ),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a Cookie header from the Coding Plan page.",
                None,
            ),
        ],
        "augment" => vec![
            cookie_option(lang, "auto", "Automatic imports browser cookies.", "", None),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a Cookie header from the Augment dashboard.",
                None,
            ),
        ],
        "amp" => vec![
            cookie_option(lang, "auto", "Automatic imports browser cookies.", "", None),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a Cookie header from Amp settings.",
                None,
            ),
        ],
        "ollama" => vec![
            cookie_option(lang, "auto", "Automatic imports browser cookies.", "", None),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a Cookie header from Ollama settings.",
                None,
            ),
        ],
        _ => Vec::new(),
    }
}

/// Returns the API region options for a given provider.
/// Empty vec means the provider has no region picker.
pub fn region_options_for(provider_id: &str) -> Vec<RegionOption> {
    match provider_id {
        "alibaba" => vec![
            RegionOption {
                value: "intl".to_string(),
                label: "International (Model Studio)".to_string(),
            },
            RegionOption {
                value: "cn".to_string(),
                label: "China Mainland (Bailian)".to_string(),
            },
        ],
        "zai" => vec![
            RegionOption {
                value: "global".to_string(),
                label: "Global".to_string(),
            },
            RegionOption {
                value: "china".to_string(),
                label: "China Mainland (BigModel)".to_string(),
            },
        ],
        "minimax" => vec![
            RegionOption {
                value: "global".to_string(),
                label: "Global (.io)".to_string(),
            },
            RegionOption {
                value: "china".to_string(),
                label: "China Mainland (.com)".to_string(),
            },
        ],
        _ => Vec::new(),
    }
}

#[tauri::command]
pub fn get_provider_cookie_source_options(
    provider_id: String,
) -> Result<Vec<CookieSourceOption>, String> {
    let lang = Settings::load().ui_language;
    Ok(cookie_source_options_for(&provider_id, lang))
}

#[tauri::command]
pub fn get_provider_region_options(provider_id: String) -> Result<Vec<RegionOption>, String> {
    Ok(region_options_for(&provider_id))
}

// ── Credential detection ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiCliStatus {
    pub signed_in: bool,
    pub credentials_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VertexAiStatus {
    pub has_credentials: bool,
    pub credentials_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JetbrainsIde {
    pub id: String,
    pub display_name: String,
    pub path: String,
    pub detected: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroStatus {
    pub available: bool,
    pub hint: Option<String>,
}

fn gemini_cli_credentials_path() -> Option<std::path::PathBuf> {
    codexbar::host::session::gemini_cli_credentials_path()
}

fn vertexai_credentials_path_raw() -> Option<std::path::PathBuf> {
    codexbar::host::session::vertexai_credentials_path()
}

fn jetbrains_detected_ide_paths() -> Vec<std::path::PathBuf> {
    codexbar::host::session::jetbrains_detected_ide_paths()
}

#[tauri::command]
pub fn get_gemini_cli_signed_in() -> Result<GeminiCliStatus, String> {
    let path = gemini_cli_credentials_path();
    let signed_in = path.as_ref().map(|p| p.exists()).unwrap_or(false);
    Ok(GeminiCliStatus {
        signed_in,
        credentials_path: path.map(|p| p.to_string_lossy().into_owned()),
    })
}

#[tauri::command]
pub fn get_vertexai_status() -> Result<VertexAiStatus, String> {
    let path = vertexai_credentials_path_raw();
    let has = path.as_ref().map(|p| p.exists()).unwrap_or(false);
    Ok(VertexAiStatus {
        has_credentials: has,
        credentials_path: path.map(|p| p.to_string_lossy().into_owned()),
    })
}

#[tauri::command]
pub fn list_jetbrains_detected_ides() -> Result<Vec<JetbrainsIde>, String> {
    let settings = Settings::load();
    let override_path = settings.jetbrains_ide_base_path.clone();

    let mut entries: Vec<JetbrainsIde> = jetbrains_detected_ide_paths()
        .into_iter()
        .map(|p| {
            let display = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string());
            JetbrainsIde {
                id: display.to_lowercase(),
                display_name: display,
                path: p.to_string_lossy().into_owned(),
                detected: true,
            }
        })
        .collect();

    // If the user has an override that isn't already in the detected list,
    // surface it explicitly with `detected: false`.
    if !override_path.is_empty() && !entries.iter().any(|e| e.path == override_path) {
        let path_buf = std::path::PathBuf::from(&override_path);
        let display = path_buf
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| override_path.clone());
        entries.push(JetbrainsIde {
            id: format!("override::{display}").to_lowercase(),
            display_name: display,
            path: override_path,
            detected: false,
        });
    }

    Ok(entries)
}

#[tauri::command]
pub fn set_jetbrains_ide_path(path: String) -> Result<(), String> {
    let mut settings = Settings::load();
    settings.jetbrains_ide_base_path = path;
    settings.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_kiro_status() -> Result<KiroStatus, String> {
    if let Some(path) = codexbar::providers::kiro::find_kiro_cli() {
        Ok(KiroStatus {
            available: true,
            hint: Some(path.to_string_lossy().into_owned()),
        })
    } else {
        Ok(KiroStatus {
            available: false,
            hint: Some("kiro-cli: not found on PATH or known install locations".into()),
        })
    }
}

/// Open a filesystem path in the OS file manager (Finder / Explorer /
/// xdg-open). Non-existent paths are rejected so the UI gets immediate
/// feedback instead of a silent no-op shell launch.
#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Path is empty".into());
    }
    let pb = std::path::PathBuf::from(trimmed);
    if !pb.exists() {
        return Err(format!("Path not found: {trimmed}"));
    }
    // When given a file, open its parent directory so the file is highlighted
    // in a useful way across platforms without needing per-OS --select flags.
    let target = if pb.is_file() {
        pb.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| pb.clone())
    } else {
        pb.clone()
    };
    let target_str = target.to_string_lossy().into_owned();

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&target_str)
            .spawn()
            .map_err(|e| format!("Failed to open path: {e}"))?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let opener = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        std::process::Command::new(opener)
            .arg(&target_str)
            .spawn()
            .map_err(|e| format!("Failed to open path: {e}"))?;
    }
    Ok(())
}

// ── Global shortcut capture (user-driven, emits events) ───────────────

#[tauri::command]
pub fn register_global_shortcut(app: tauri::AppHandle, accelerator: String) -> Result<(), String> {
    use tauri::Emitter;
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

    let shortcut = crate::shortcut_bridge::parse_shortcut(&accelerator)
        .ok_or_else(|| format!("Invalid shortcut \"{accelerator}\". Use e.g. Ctrl+Shift+U."))?;

    // Best-effort cleanup of any prior capture registration.
    let _ = app.global_shortcut().unregister(shortcut);

    let accel_emit = accelerator.clone();
    app.global_shortcut()
        .on_shortcut(shortcut, move |app, _sc, event| {
            if event.state == ShortcutState::Pressed {
                let _ = app.emit("global-shortcut-triggered", accel_emit.clone());
            }
        })
        .map_err(|e| format!("Failed to register shortcut \"{accelerator}\": {e}"))?;

    Ok(())
}

#[tauri::command]
pub fn unregister_global_shortcut(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    // We don't know which accelerator was registered — unregister_all is a
    // too-wide hammer (it would also drop the persistent tray-toggle binding),
    // so re-register that afterwards.
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| format!("Failed to clear shortcuts: {e}"))?;
    crate::shortcut_bridge::register(&app);
    Ok(())
}

// ── Session / environment ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkAreaRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[tauri::command]
pub fn is_remote_session() -> Result<bool, String> {
    Ok(codexbar::host::session::is_ssh_session() || codexbar::host::session::is_remote_session())
}

#[tauri::command]
pub fn get_launch_block_reason() -> Result<Option<String>, String> {
    Ok(codexbar::host::session::current_launch_block_reason().map(|s| s.to_string()))
}

#[tauri::command]
pub fn get_work_area_rect(app: tauri::AppHandle) -> Result<WorkAreaRect, String> {
    use tauri::Manager;

    // Prefer the OS-native probe on Windows because it reliably excludes the
    // taskbar; Tauri's monitor API forwards to the same APIs but we keep the
    // direct path to preserve parity with the egui build.
    if let Some(area) = codexbar::host::session::primary_work_area_pixels() {
        return Ok(WorkAreaRect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height,
        });
    }

    // Cross-platform fallback (macOS: NSScreen.visibleFrame; Linux: GTK /
    // X11 work-area) via Tauri's monitor wrapper. Require a window so tao's
    // screen backend is initialised.
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window is not available".to_string())?;

    let monitor = window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .or_else(|| window.primary_monitor().ok().flatten())
        .ok_or_else(|| "No monitor detected".to_string())?;

    let work_area = monitor.work_area();
    Ok(WorkAreaRect {
        x: work_area.position.x,
        y: work_area.position.y,
        width: work_area.size.width as i32,
        height: work_area.size.height as i32,
    })
}

// ── Misc UX ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn play_notification_sound() -> Result<(), String> {
    // Use the shared sound helper, honouring the user's `sound_enabled` flag.
    let settings = Settings::load();
    codexbar::sound::play_alert(codexbar::sound::AlertSound::Success, &settings);
    Ok(())
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

fn dashboard_url_for_provider(provider_id: &str) -> Option<String> {
    codexbar::settings::get_api_key_providers()
        .into_iter()
        .find(|p| p.id.cli_name() == provider_id)
        .and_then(|p| p.dashboard_url.map(|s| s.to_string()))
}

fn status_page_url_for_provider(provider_id: &str) -> Option<String> {
    let id = ProviderId::from_cli_name(provider_id)?;
    let provider = instantiate_provider(id);
    provider.metadata().status_page_url.map(|s| s.to_string())
}

#[tauri::command]
pub fn open_provider_dashboard(provider_id: String) -> Result<(), String> {
    let url = dashboard_url_for_provider(&provider_id)
        .ok_or_else(|| format!("No dashboard URL registered for provider '{provider_id}'"))?;
    open_url_in_browser(&url)
}

#[tauri::command]
pub fn open_provider_status_page(provider_id: String) -> Result<(), String> {
    let url = status_page_url_for_provider(&provider_id)
        .ok_or_else(|| format!("No status page URL registered for provider '{provider_id}'"))?;
    open_url_in_browser(&url)
}

#[tauri::command]
pub fn trigger_provider_login(provider_id: String) -> Result<(), String> {
    // TODO(6b): replace fallthrough once LoginPhase events land. The login
    // runners live in `codexbar::login` but are async-oriented and tightly
    // coupled to the egui UI's phase callbacks. For the Tauri shell we
    // currently surface the dashboard URL.
    if let Some(url) = dashboard_url_for_provider(&provider_id) {
        return open_url_in_browser(&url);
    }
    Err(format!(
        "Login flow for '{provider_id}' is not yet wired through the Tauri shell"
    ))
}

// ── Provider detail pane (Phase 6b) ──────────────────────────────────

/// DTO for the provider detail pane in the Settings Providers tab.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDetail {
    pub id: String,
    pub display_name: String,
    pub enabled: bool,

    // Identity
    pub email: Option<String>,
    pub plan: Option<String>,
    pub auth_type: Option<String>,
    pub source_label: Option<String>,
    pub organization: Option<String>,
    pub last_updated: Option<String>,

    // Usage windows — reuse existing RateWindowSnapshot shape.
    pub session: Option<RateWindowSnapshot>,
    pub weekly: Option<RateWindowSnapshot>,
    pub model_specific: Option<RateWindowSnapshot>,
    pub tertiary: Option<RateWindowSnapshot>,

    // Cost / pace.
    pub cost: Option<CostSnapshotBridge>,
    pub pace: Option<PaceSnapshot>,

    // Error / state.
    pub last_error: Option<String>,

    // URLs for quick-actions (button visibility).
    pub dashboard_url: Option<String>,
    pub status_page_url: Option<String>,
    pub buy_credits_url: Option<String>,

    // True if the shared backend has produced any snapshot yet.
    pub has_snapshot: bool,

    // Phase 6c — currently-persisted cookie source & region for round-tripping
    // into the settings UI pickers. `None` for providers that do not support
    // one of the pickers.
    pub cookie_source: Option<String>,
    pub region: Option<String>,
}

fn build_provider_detail(provider_id: &str) -> Result<ProviderDetail, String> {
    let id = ProviderId::from_cli_name(provider_id)
        .ok_or_else(|| format!("Unknown provider id: {provider_id}"))?;

    let settings = Settings::load();
    let enabled = settings
        .enabled_providers
        .iter()
        .any(|p| p == id.cli_name());

    let provider = instantiate_provider(id);
    let metadata = provider.metadata();

    Ok(ProviderDetail {
        id: id.cli_name().to_string(),
        display_name: id.display_name().to_string(),
        enabled,
        email: None,
        plan: None,
        auth_type: None,
        source_label: None,
        organization: None,
        last_updated: None,
        session: None,
        weekly: None,
        model_specific: None,
        tertiary: None,
        cost: None,
        pace: None,
        last_error: None,
        dashboard_url: metadata.dashboard_url.map(|s| s.to_string()),
        status_page_url: metadata.status_page_url.map(|s| s.to_string()),
        // Buy-credits currently mirrors the dashboard URL for providers that
        // support credit top-ups; refine once a dedicated URL lands upstream.
        buy_credits_url: if metadata.supports_credits {
            metadata.dashboard_url.map(|s| s.to_string())
        } else {
            None
        },
        has_snapshot: false,
        cookie_source: provider_cookie_source_lookup(&settings, id.cli_name()),
        region: provider_region_lookup(&settings, id.cli_name()),
    })
}

#[tauri::command]
pub fn get_provider_detail(
    app: tauri::AppHandle,
    provider_id: String,
) -> Result<ProviderDetail, String> {
    let mut detail = build_provider_detail(&provider_id)?;

    // Merge the latest cached snapshot, if any.
    let state = app.state::<Mutex<AppState>>();
    if let Ok(guard) = state.lock()
        && let Some(snap) = guard
            .provider_cache
            .iter()
            .find(|s| s.provider_id == detail.id)
    {
        detail.email = snap.account_email.clone();
        detail.plan = snap.plan_name.clone();
        detail.organization = snap.account_organization.clone();
        detail.source_label = if snap.source_label.is_empty() {
            None
        } else {
            Some(snap.source_label.clone())
        };
        detail.last_updated = Some(snap.updated_at.clone());
        if snap.error.is_none() {
            detail.session = Some(snap.primary.clone());
            detail.weekly = snap.secondary.clone();
            detail.model_specific = snap.model_specific.clone();
            detail.tertiary = snap.tertiary.clone();
            detail.cost = snap.cost.clone();
            detail.pace = snap.pace.clone();
        }
        detail.last_error = snap.error.clone();
        detail.has_snapshot = true;
    }

    Ok(detail)
}

#[tauri::command]
pub fn revoke_provider_credentials(provider_id: String) -> Result<(), String> {
    // Best-effort: drop both a stored API key and any manual cookie so the
    // caller can follow up with a fresh login or import. Missing entries are
    // silently ignored; only I/O errors propagate.
    let mut keys = ApiKeys::load();
    keys.remove(&provider_id);
    keys.save().map_err(|e| e.to_string())?;

    let mut cookies = ManualCookies::load();
    cookies.remove(&provider_id);
    cookies.save().map_err(|e| e.to_string())?;

    Ok(())
}

// ── Locale / i18n commands ───────────────────────────────────────────

/// Snapshot of every localized UI string in a given language.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocaleStrings {
    /// Serialized language code (`"english"` or `"chinese"`).
    pub language: &'static str,
    /// Map of serialized `LocaleKey` variant name → localized text.
    pub entries: HashMap<&'static str, &'static str>,
}

fn locale_strings_for(lang: Language) -> LocaleStrings {
    let mut entries = HashMap::with_capacity(locale::LocaleKey::ALL.len());
    for (key, name) in locale::LocaleKey::ALL {
        entries.insert(*name, locale::get_text(lang, *key));
    }
    LocaleStrings {
        language: language_label(lang),
        entries,
    }
}

/// Return every UI string for the requested language.
///
/// When `language` is `None`, the user's current persisted language is used.
/// The `language` argument accepts either the short code (`"en"`, `"zh"`),
/// the persisted label (`"english"`, `"chinese"`), or the full name
/// (`"English"`, `"Chinese"`, `"中文"`).
#[tauri::command]
pub fn get_locale_strings(language: Option<String>) -> Result<LocaleStrings, String> {
    let lang = match language.as_deref() {
        None => locale::current_language(),
        Some(raw) => {
            parse_locale_language(raw).ok_or_else(|| format!("unknown language code: {raw}"))?
        }
    };
    Ok(locale_strings_for(lang))
}

fn parse_locale_language(raw: &str) -> Option<Language> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "en" | "en-us" | "english" => Some(Language::English),
        "zh" | "zh-cn" | "zh-hans" | "chinese" | "中文" => Some(Language::Chinese),
        _ => None,
    }
}

/// Persist the UI language and emit a `locale-changed` event so the
/// frontend can refetch its locale table without a restart.
#[tauri::command]
pub fn set_ui_language(app: tauri::AppHandle, language: String) -> Result<(), String> {
    let lang =
        parse_locale_language(&language).ok_or_else(|| format!("unknown language: {language}"))?;
    let mut settings = Settings::load();
    if settings.ui_language == lang {
        return Ok(());
    }
    settings.ui_language = lang;
    settings.save().map_err(|e| e.to_string())?;
    let _ = app.emit(events::LOCALE_CHANGED, language_label(lang));
    Ok(())
}

#[cfg(test)]
mod locale_tests {
    use super::*;

    #[test]
    fn locale_strings_roundtrip_english() {
        let bundle = locale_strings_for(Language::English);
        assert_eq!(bundle.language, "english");
        assert_eq!(
            bundle.entries.get("TabGeneral").copied(),
            Some("General"),
            "TabGeneral should resolve to English"
        );
        assert_eq!(bundle.entries.len(), locale::LocaleKey::ALL.len());
    }

    #[test]
    fn locale_strings_roundtrip_chinese() {
        let bundle = locale_strings_for(Language::Chinese);
        assert_eq!(bundle.language, "chinese");
        assert_eq!(bundle.entries.get("TabGeneral").copied(), Some("通用"));
        assert_eq!(bundle.entries.len(), locale::LocaleKey::ALL.len());
    }

    #[test]
    fn locale_strings_contains_every_variant() {
        let bundle = locale_strings_for(Language::English);
        for (_, name) in locale::LocaleKey::ALL {
            assert!(
                bundle.entries.contains_key(name),
                "missing key in locale bundle: {name}"
            );
        }
    }

    #[test]
    fn parse_locale_language_accepts_aliases() {
        assert!(matches!(
            parse_locale_language("en"),
            Some(Language::English)
        ));
        assert!(matches!(
            parse_locale_language("English"),
            Some(Language::English)
        ));
        assert!(matches!(
            parse_locale_language("zh"),
            Some(Language::Chinese)
        ));
        assert!(matches!(
            parse_locale_language("Chinese"),
            Some(Language::Chinese)
        ));
        assert!(matches!(
            parse_locale_language("中文"),
            Some(Language::Chinese)
        ));
        assert!(parse_locale_language("klingon").is_none());
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ProviderSummary, apply_provider_order, bridge_commands, bridge_events,
        provider_cookie_source_lookup, provider_region_lookup, validate_surface_target,
    };
    use crate::surface::SurfaceMode;
    use crate::surface_target::SurfaceTarget;
    use codexbar::core::ProviderId;
    use codexbar::host::session::launch_block_reason;
    use codexbar::settings::{Language, Settings};

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

    #[test]
    fn bootstrap_contract_lists_phase4_commands() {
        let ids = bridge_commands()
            .into_iter()
            .map(|descriptor| descriptor.id)
            .collect::<Vec<_>>();

        for expected in [
            "reorder_providers",
            "set_provider_cookie_source",
            "get_provider_cookie_source",
            "set_provider_region",
            "get_provider_region",
            "get_gemini_cli_signed_in",
            "get_vertexai_status",
            "list_jetbrains_detected_ides",
            "set_jetbrains_ide_path",
            "get_kiro_status",
            "register_global_shortcut",
            "unregister_global_shortcut",
            "is_remote_session",
            "get_launch_block_reason",
            "get_work_area_rect",
            "play_notification_sound",
            "open_provider_dashboard",
            "trigger_provider_login",
            "revoke_provider_credentials",
        ] {
            assert!(ids.contains(&expected), "missing command id: {expected}");
        }
    }

    #[test]
    fn bootstrap_contract_lists_global_shortcut_event() {
        let ids = bridge_events()
            .into_iter()
            .map(|descriptor| descriptor.id)
            .collect::<Vec<_>>();

        assert!(ids.contains(&"global-shortcut-triggered"));
    }

    #[test]
    fn apply_provider_order_dedupes_and_appends_unknown_canonical() {
        // Request only "codex" and "claude" — remaining canonical ids should
        // be appended after, preserving canonical order.
        let order = apply_provider_order(&["codex".to_string(), "claude".to_string()]);
        assert_eq!(order[0], "codex");
        assert_eq!(order[1], "claude");
        // Every canonical id appears exactly once.
        let mut sorted = order.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), order.len());
        // Every canonical id is present.
        let canonical = codexbar::core::ProviderId::all()
            .iter()
            .map(|p| p.cli_name().to_string())
            .collect::<Vec<_>>();
        for id in &canonical {
            assert!(order.contains(id), "missing canonical id: {id}");
        }
    }

    #[test]
    fn apply_provider_order_ignores_unknown_ids() {
        let order = apply_provider_order(&["not-a-provider".to_string(), "codex".to_string()]);
        assert_eq!(order[0], "codex");
        assert!(!order.iter().any(|id| id == "not-a-provider"));
    }

    #[test]
    fn provider_summaries_reflect_settings_order() {
        let canonical_len = codexbar::core::ProviderId::all().len();
        let s = Settings::default();
        let summaries: Vec<ProviderSummary> = super::build_provider_summaries(&s);
        assert_eq!(summaries.len(), canonical_len);
        // Index is assigned in emission order.
        for (i, s) in summaries.iter().enumerate() {
            assert_eq!(s.order, i as u32);
        }
    }

    #[test]
    fn provider_cookie_source_lookup_roundtrips_known_providers() {
        let mut s = Settings::default();
        super::provider_cookie_source_set(&mut s, "codex", "cli-config".to_string()).unwrap();
        assert_eq!(
            provider_cookie_source_lookup(&s, "codex").as_deref(),
            Some("cli-config")
        );
        assert!(provider_cookie_source_lookup(&s, "unknown-provider").is_none());
    }

    #[test]
    fn provider_region_lookup_roundtrips_known_providers() {
        let mut s = Settings::default();
        super::provider_region_set(&mut s, "alibaba", "china".to_string()).unwrap();
        assert_eq!(
            provider_region_lookup(&s, "alibaba").as_deref(),
            Some("china")
        );
        // Non-regional providers return None.
        assert!(provider_region_lookup(&s, "claude").is_none());
    }

    #[test]
    fn provider_cookie_source_set_rejects_unknown_provider() {
        let mut s = Settings::default();
        let err = super::provider_cookie_source_set(&mut s, "nope", "x".into()).unwrap_err();
        assert!(err.contains("nope"));
    }

    #[test]
    fn provider_region_set_rejects_non_regional_provider() {
        let mut s = Settings::default();
        let err = super::provider_region_set(&mut s, "claude", "global".into()).unwrap_err();
        assert!(err.contains("claude"));
    }

    #[test]
    fn launch_block_reason_helper_returns_none_when_not_blocked() {
        assert!(launch_block_reason(false, false).is_none());
    }

    #[test]
    fn launch_block_reason_helper_prefers_ssh() {
        let msg = launch_block_reason(true, true).unwrap();
        assert!(msg.contains("SSH"));
    }

    // ── Phase 6b — provider detail pane ────────────────────────────

    #[test]
    fn build_provider_detail_populates_identity_urls() {
        let detail = super::build_provider_detail("claude").expect("known provider");
        assert_eq!(detail.id, "claude");
        assert_eq!(detail.display_name, "Claude");
        // Claude advertises a status page URL in its metadata.
        assert!(detail.status_page_url.is_some());
        // No snapshot yet — empty usage bars and no error.
        assert!(detail.session.is_none());
        assert!(detail.last_error.is_none());
        assert!(!detail.has_snapshot);
    }

    #[test]
    fn build_provider_detail_rejects_unknown_provider() {
        let err = super::build_provider_detail("not-a-provider").unwrap_err();
        assert!(err.contains("not-a-provider"));
    }

    #[test]
    fn provider_detail_roundtrips_through_serde() {
        let detail = super::build_provider_detail("codex").expect("known provider");
        let json = serde_json::to_string(&detail).expect("serialize");
        // camelCase rename survives the round-trip.
        assert!(json.contains("\"displayName\""));
        assert!(json.contains("\"hasSnapshot\""));
        assert!(json.contains("\"statusPageUrl\""));
    }

    #[test]
    fn pace_stage_serializes_to_snake_case_string() {
        use codexbar::core::PaceStage;
        assert_eq!(super::pace_stage_str(PaceStage::OnTrack), "on_track");
        assert_eq!(
            super::pace_stage_str(PaceStage::SlightlyAhead),
            "slightly_ahead"
        );
        assert_eq!(super::pace_stage_str(PaceStage::FarAhead), "far_ahead");
        assert_eq!(
            super::pace_stage_str(PaceStage::SlightlyBehind),
            "slightly_behind"
        );
        assert_eq!(super::pace_stage_str(PaceStage::Behind), "behind");
        assert_eq!(super::pace_stage_str(PaceStage::FarBehind), "far_behind");
    }

    #[test]
    fn bootstrap_contract_lists_phase6b_commands() {
        let ids = bridge_commands()
            .into_iter()
            .map(|descriptor| descriptor.id)
            .collect::<Vec<_>>();

        for expected in ["get_provider_detail", "open_provider_status_page"] {
            assert!(ids.contains(&expected), "missing command id: {expected}");
        }
    }

    #[test]
    fn bootstrap_contract_lists_chart_data_command() {
        let ids = bridge_commands()
            .into_iter()
            .map(|descriptor| descriptor.id)
            .collect::<Vec<_>>();
        assert!(
            ids.contains(&"get_provider_chart_data"),
            "get_provider_chart_data must be advertised to the bridge",
        );
    }

    #[test]
    fn chart_data_serde_roundtrip_preserves_fields() {
        use super::{DailyCostPoint, DailyUsageBreakdown, ProviderChartData, ServiceUsagePoint};

        let original = ProviderChartData {
            provider_id: "codex".into(),
            cost_history: vec![
                DailyCostPoint {
                    date: "2025-01-01".into(),
                    value: 1.25,
                },
                DailyCostPoint {
                    date: "2025-01-02".into(),
                    value: 0.0,
                },
            ],
            credits_history: vec![DailyCostPoint {
                date: "2025-01-01".into(),
                value: 42.0,
            }],
            usage_breakdown: vec![DailyUsageBreakdown {
                day: "2025-01-01".into(),
                services: vec![
                    ServiceUsagePoint {
                        service: "gpt-4o".into(),
                        credits_used: 10.0,
                    },
                    ServiceUsagePoint {
                        service: "gpt-4o-mini".into(),
                        credits_used: 3.5,
                    },
                ],
                total_credits_used: 13.5,
            }],
        };

        let json = serde_json::to_string(&original).expect("serialize");
        assert!(
            json.contains("\"providerId\":\"codex\""),
            "camelCase providerId: {json}"
        );
        assert!(json.contains("\"costHistory\""));
        assert!(json.contains("\"creditsHistory\""));
        assert!(json.contains("\"usageBreakdown\""));
        assert!(json.contains("\"creditsUsed\":10.0"));
        assert!(json.contains("\"totalCreditsUsed\":13.5"));

        let back: ProviderChartData = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.provider_id, "codex");
        assert_eq!(back.cost_history.len(), 2);
        assert_eq!(back.cost_history[0].date, "2025-01-01");
        assert_eq!(back.credits_history[0].value, 42.0);
        assert_eq!(back.usage_breakdown[0].services.len(), 2);
        assert_eq!(back.usage_breakdown[0].total_credits_used, 13.5);
    }

    #[test]
    fn chart_data_for_unknown_provider_is_empty() {
        let data =
            super::get_provider_chart_data("this-provider-definitely-does-not-exist".into(), None);
        assert_eq!(data.provider_id, "this-provider-definitely-does-not-exist");
        assert!(data.credits_history.is_empty());
        assert!(data.usage_breakdown.is_empty());
    }

    #[test]
    fn chart_data_requires_account_email_for_codex() {
        let data = super::get_provider_chart_data("codex".into(), None);
        assert_eq!(data.provider_id, "codex");
        assert!(data.credits_history.is_empty());
        assert!(data.usage_breakdown.is_empty());
    }

    #[test]
    fn bootstrap_contract_lists_phase6c_commands() {
        let ids = bridge_commands()
            .into_iter()
            .map(|descriptor| descriptor.id)
            .collect::<Vec<_>>();
        for expected in [
            "get_provider_cookie_source_options",
            "get_provider_region_options",
        ] {
            assert!(ids.contains(&expected), "missing command id: {expected}");
        }
    }

    #[test]
    fn cookie_options_for_cookie_supporting_provider() {
        let opts = super::cookie_source_options_for("codex", Language::English);
        let values: Vec<_> = opts.iter().map(|o| o.value.as_str()).collect();
        assert_eq!(values, vec!["auto", "manual", "off"]);
        assert!(opts.iter().any(|o| o.label == "Automatic"));
        assert!(opts.iter().any(|o| o.label == "Manual"));
        assert!(opts.iter().any(|o| o.label == "Disabled"));
    }

    #[test]
    fn cookie_options_empty_for_providers_without_picker() {
        assert!(super::cookie_source_options_for("anthropic", Language::English).is_empty());
        assert!(super::cookie_source_options_for("unknown", Language::English).is_empty());
    }

    #[test]
    fn region_options_for_regional_provider() {
        let opts = super::region_options_for("alibaba");
        let values: Vec<_> = opts.iter().map(|o| o.value.as_str()).collect();
        assert_eq!(values, vec!["intl", "cn"]);
    }

    #[test]
    fn region_options_empty_for_non_regional_provider() {
        assert!(super::region_options_for("claude").is_empty());
        assert!(super::region_options_for("codex").is_empty());
    }

    #[test]
    fn cookie_source_option_roundtrips_serde() {
        let opt = super::CookieSourceOption {
            value: "auto".to_string(),
            label: "Automatic".to_string(),
            description: Some("Imports browser cookies.".to_string()),
        };
        let json = serde_json::to_string(&opt).unwrap();
        let back: super::CookieSourceOption = serde_json::from_str(&json).unwrap();
        assert_eq!(opt, back);
    }

    #[test]
    fn region_option_roundtrips_serde() {
        let opt = super::RegionOption {
            value: "intl".to_string(),
            label: "International".to_string(),
        };
        let json = serde_json::to_string(&opt).unwrap();
        let back: super::RegionOption = serde_json::from_str(&json).unwrap();
        assert_eq!(opt, back);
    }

    // ── Phase 6d — credential detection UIs ────────────────────────

    #[test]
    fn bootstrap_contract_lists_phase6d_open_path() {
        let ids = bridge_commands()
            .into_iter()
            .map(|descriptor| descriptor.id)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"open_path"));
    }

    #[test]
    fn open_path_rejects_empty_path() {
        let err = super::open_path(String::new()).unwrap_err();
        assert!(err.to_lowercase().contains("empty"));
    }

    #[test]
    fn open_path_rejects_missing_path() {
        let err =
            super::open_path("/definitely/not/a/real/path/codexbar-phase6d".into()).unwrap_err();
        assert!(err.contains("not found"));
    }

    // ── Phase 13 — E2E IPC harness ─────────────────────────────────
    //
    // Build the full bootstrap payload and prove that every shared
    // `ProviderId` variant ends up in the provider catalog with a
    // non-empty id + display name. If a new provider is added to the
    // enum but never wired through the desktop catalog, this test will
    // fail with `missing provider in bootstrap catalog: <id>`.

    #[test]
    fn bootstrap_payload_exposes_every_provider_variant() {
        let payload = super::get_bootstrap_state();

        let catalog_ids: std::collections::HashSet<String> = payload
            .providers
            .iter()
            .map(|entry| entry.id.clone())
            .collect();

        for entry in &payload.providers {
            assert!(!entry.id.is_empty(), "provider entry has empty id");
            assert!(
                !entry.display_name.is_empty(),
                "provider {} has empty display_name",
                entry.id
            );
        }

        for provider in ProviderId::all() {
            let expected = provider.cli_name().to_string();
            assert!(
                catalog_ids.contains(&expected),
                "missing provider in bootstrap catalog: {expected}"
            );
        }

        assert_eq!(
            catalog_ids.len(),
            ProviderId::all().len(),
            "bootstrap catalog size drifted from ProviderId::all()"
        );

        // Sanity — payload must also round-trip through JSON cleanly so
        // the TypeScript bridge never sees a partially-populated record.
        let encoded = serde_json::to_string(&payload).expect("serialize bootstrap");
        assert!(encoded.contains("contractVersion"));
        assert!(encoded.contains("\"providers\""));
        assert!(encoded.contains("\"settings\""));
    }
}
