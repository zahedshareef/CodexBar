//! Settings management for CodexBar
//!
//! Handles persistent configuration including:
//! - Enabled/disabled providers
//! - Refresh interval
//! - Manual cookies
//! - Other user preferences

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::core::ProviderId;

/// UI language for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// English (default)
    #[default]
    English,
    /// Chinese (Simplified)
    Chinese,
}

impl Language {
    /// Get the display name for this language
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Chinese => "中文",
        }
    }

    /// Get all available languages
    pub fn all() -> &'static [Language] {
        &[Language::English, Language::Chinese]
    }
}

/// Update channel for receiving updates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    #[default]
    Stable,
    Beta,
}

impl UpdateChannel {
    /// Get the display name for this channel
    pub fn display_name(&self) -> &'static str {
        match self {
            UpdateChannel::Stable => "Stable",
            UpdateChannel::Beta => "Beta",
        }
    }

    /// Get a description for this channel
    pub fn description(&self) -> &'static str {
        match self {
            UpdateChannel::Stable => "Receive stable, tested releases",
            UpdateChannel::Beta => "Get early access to new features",
        }
    }
}

/// Tray icon display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TrayIconMode {
    /// Single tray icon showing the primary provider or merged view
    #[default]
    Single,
    /// One tray icon per enabled provider
    PerProvider,
}

impl TrayIconMode {
    /// Get the display name for this mode
    pub fn display_name(&self) -> &'static str {
        match self {
            TrayIconMode::Single => "Single Icon",
            TrayIconMode::PerProvider => "Per Provider",
        }
    }

    /// Get a description for this mode
    pub fn description(&self) -> &'static str {
        match self {
            TrayIconMode::Single => "Show one tray icon for all providers",
            TrayIconMode::PerProvider => "Show a separate tray icon for each enabled provider",
        }
    }
}

/// Metric preference for display in tray and UI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MetricPreference {
    #[default]
    Automatic,
    Session,
    Weekly,
    Model,
    Credits,
    Average,
}

impl MetricPreference {
    /// Get all available metric preferences
    pub fn all() -> &'static [MetricPreference] {
        &[
            MetricPreference::Automatic,
            MetricPreference::Session,
            MetricPreference::Weekly,
            MetricPreference::Model,
            MetricPreference::Credits,
            MetricPreference::Average,
        ]
    }

    /// Get the display name for this metric
    pub fn display_name(&self) -> &'static str {
        match self {
            MetricPreference::Automatic => "Automatic",
            MetricPreference::Session => "Session",
            MetricPreference::Weekly => "Weekly",
            MetricPreference::Model => "Model",
            MetricPreference::Credits => "Credits",
            MetricPreference::Average => "Average",
        }
    }

    /// Get a description for this metric
    pub fn description(&self) -> &'static str {
        match self {
            MetricPreference::Automatic => "Automatically select the best metric",
            MetricPreference::Session => "Current session usage",
            MetricPreference::Weekly => "Weekly usage limit",
            MetricPreference::Model => "Model-specific limit",
            MetricPreference::Credits => "Credit balance",
            MetricPreference::Average => "Average across metrics",
        }
    }
}

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Enabled provider IDs (by CLI name)
    pub enabled_providers: HashSet<String>,

    /// Refresh interval in seconds (0 = manual only)
    pub refresh_interval_secs: u64,

    /// Whether to start minimized
    pub start_minimized: bool,

    /// Whether to start at login
    pub start_at_login: bool,

    /// Whether to show notifications
    pub show_notifications: bool,

    /// Whether to play sound effects for threshold alerts
    pub sound_enabled: bool,

    /// Sound volume for alerts (0-100)
    pub sound_volume: u8,

    /// High usage threshold for warnings (percentage)
    pub high_usage_threshold: f64,

    /// Critical usage threshold for alerts (percentage)
    pub critical_usage_threshold: f64,

    /// Merge mode: show all enabled providers in a single tray icon
    pub merge_tray_icons: bool,

    /// Tray icon display mode: single icon or per-provider icons
    #[serde(default)]
    pub tray_icon_mode: TrayIconMode,

    /// Show provider icons in the merged switcher UI
    #[serde(default = "default_true")]
    pub switcher_shows_icons: bool,

    /// Prefer the provider closest to its limit in merged menu bar display
    #[serde(default)]
    pub menu_bar_shows_highest_usage: bool,

    /// Replace bar-only tray display with provider branding plus percent text where supported
    #[serde(default)]
    pub menu_bar_shows_percent: bool,

    /// Show usage bars as "used" (true) or "remaining" (false)
    pub show_as_used: bool,

    /// Enable random "surprise" animations (blinks, wiggles)
    pub surprise_animations: bool,

    /// Enable UI animations (chart entrances, transitions)
    pub enable_animations: bool,

    /// Show reset times as relative (e.g., "2h 30m" instead of "3:00 PM")
    pub reset_time_relative: bool,

    /// Menu bar display mode: "minimal", "compact", or "detailed"
    pub menu_bar_display_mode: String,

    /// Show credits and extra usage information in the UI
    pub show_credits_extra_usage: bool,

    /// Show all token accounts in provider menus instead of collapsing behind switchers
    #[serde(default)]
    pub show_all_token_accounts_in_menu: bool,

    /// Preferred Claude usage source for the provider settings UI
    #[serde(default = "default_claude_usage_source")]
    pub claude_usage_source: String,

    /// Preferred Codex usage source for the provider settings UI
    #[serde(default = "default_codex_usage_source")]
    pub codex_usage_source: String,

    /// Preferred Codex cookie source for the provider settings UI
    #[serde(default = "default_codex_cookie_source")]
    pub codex_cookie_source: String,

    /// Whether Codex historical tracking is enabled in the settings UI
    #[serde(default)]
    pub codex_historical_tracking: bool,

    /// Whether Codex OpenAI web extras are enabled in the settings UI
    #[serde(default = "default_codex_openai_web_extras")]
    pub codex_openai_web_extras: bool,

    /// Preferred Claude cookie source for the provider settings UI
    #[serde(default = "default_claude_cookie_source")]
    pub claude_cookie_source: String,

    /// Preferred Cursor cookie source for the provider settings UI
    #[serde(default = "default_cursor_cookie_source")]
    pub cursor_cookie_source: String,

    /// Preferred OpenCode cookie source for the provider settings UI
    #[serde(default = "default_opencode_cookie_source")]
    pub opencode_cookie_source: String,

    /// Optional OpenCode workspace ID override for the provider settings UI
    #[serde(default)]
    pub opencode_workspace_id: String,

    /// Preferred Factory cookie source for the provider settings UI
    #[serde(default = "default_factory_cookie_source")]
    pub factory_cookie_source: String,

    /// Preferred Alibaba cookie source for the provider settings UI
    #[serde(default = "default_alibaba_cookie_source")]
    pub alibaba_cookie_source: String,

    /// Manual Alibaba cookie header for the provider settings UI
    #[serde(default)]
    pub alibaba_cookie_header: String,

    /// Preferred Alibaba API gateway region for the provider settings UI
    #[serde(default = "default_alibaba_api_region")]
    pub alibaba_api_region: String,

    /// Preferred Kimi cookie source for the provider settings UI
    #[serde(default = "default_kimi_cookie_source")]
    pub kimi_cookie_source: String,

    /// Manual Kimi cookie header or token value for the provider settings UI
    #[serde(default)]
    pub kimi_manual_cookie_header: String,

    /// Preferred MiniMax cookie source for the provider settings UI
    #[serde(default = "default_minimax_cookie_source")]
    pub minimax_cookie_source: String,

    /// Preferred Augment cookie source for the provider settings UI
    #[serde(default = "default_augment_cookie_source")]
    pub augment_cookie_source: String,

    /// Manual Augment cookie header for the provider settings UI
    #[serde(default)]
    pub augment_cookie_header: String,

    /// Preferred Amp cookie source for the provider settings UI
    #[serde(default = "default_amp_cookie_source")]
    pub amp_cookie_source: String,

    /// Manual Amp cookie header for the provider settings UI
    #[serde(default)]
    pub amp_cookie_header: String,

    /// Preferred Ollama cookie source for the provider settings UI
    #[serde(default = "default_ollama_cookie_source")]
    pub ollama_cookie_source: String,

    /// Manual Ollama cookie header for the provider settings UI
    #[serde(default)]
    pub ollama_cookie_header: String,

    /// Preferred z.ai API region for the provider settings UI
    #[serde(default = "default_zai_api_region")]
    pub zai_api_region: String,

    /// Optional JetBrains IDE base path override for the provider settings UI
    #[serde(default)]
    pub jetbrains_ide_base_path: String,

    /// Manual MiniMax cookie header for the provider settings UI
    #[serde(default)]
    pub minimax_cookie_header: String,

    /// MiniMax API token for the provider settings UI
    #[serde(default)]
    pub minimax_api_token: String,

    /// MiniMax API region for the provider settings UI
    #[serde(default = "default_minimax_api_region")]
    pub minimax_api_region: String,

    /// Whether prompt-free Claude keychain credential reads are enabled
    #[serde(default)]
    pub claude_avoid_keychain_prompts: bool,

    /// Show debug-oriented settings and troubleshooting surfaces
    #[serde(default)]
    pub show_debug_settings: bool,

    /// Disable credential/keychain-style reads where supported
    #[serde(default)]
    pub disable_keychain_access: bool,

    /// Hide personal info (emails, account names) for streaming/sharing
    pub hide_personal_info: bool,

    /// Update channel for receiving updates (Stable or Beta)
    pub update_channel: UpdateChannel,

    /// Per-provider metric preference for tray display
    #[serde(default)]
    pub provider_metrics: HashMap<String, MetricPreference>,

    /// Global keyboard shortcut to open the menu (e.g., "Ctrl+Shift+U")
    #[serde(default = "default_global_shortcut")]
    pub global_shortcut: String,

    /// Automatically download updates in the background
    #[serde(default)]
    pub auto_download_updates: bool,

    /// Install pending updates when quitting the application
    #[serde(default)]
    pub install_updates_on_quit: bool,

    /// UI language for the application (English default for backward compatibility)
    #[serde(default)]
    pub ui_language: Language,
}

fn default_global_shortcut() -> String {
    "Ctrl+Shift+U".to_string()
}

fn default_true() -> bool {
    true
}

fn default_claude_usage_source() -> String {
    "auto".to_string()
}

fn default_codex_usage_source() -> String {
    "auto".to_string()
}

fn default_codex_cookie_source() -> String {
    "auto".to_string()
}

fn default_codex_openai_web_extras() -> bool {
    true
}

fn default_claude_cookie_source() -> String {
    "auto".to_string()
}

fn default_cursor_cookie_source() -> String {
    "auto".to_string()
}

fn default_opencode_cookie_source() -> String {
    "auto".to_string()
}

fn default_factory_cookie_source() -> String {
    "auto".to_string()
}

fn default_alibaba_cookie_source() -> String {
    "auto".to_string()
}

fn default_alibaba_api_region() -> String {
    "intl".to_string()
}

fn default_kimi_cookie_source() -> String {
    "auto".to_string()
}

fn default_minimax_cookie_source() -> String {
    "auto".to_string()
}

fn default_augment_cookie_source() -> String {
    "auto".to_string()
}

fn default_amp_cookie_source() -> String {
    "auto".to_string()
}

fn default_ollama_cookie_source() -> String {
    "auto".to_string()
}

fn default_zai_api_region() -> String {
    "global".to_string()
}

fn default_minimax_api_region() -> String {
    "global".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        let mut enabled = HashSet::new();
        // Default enabled providers
        enabled.insert("claude".to_string());
        enabled.insert("codex".to_string());

        Self {
            enabled_providers: enabled,
            refresh_interval_secs: 300, // 5 minutes
            start_minimized: false,
            start_at_login: false,
            show_notifications: true,
            sound_enabled: true,
            sound_volume: 100,
            high_usage_threshold: 70.0,
            critical_usage_threshold: 90.0,
            merge_tray_icons: false, // Show single provider by default
            tray_icon_mode: TrayIconMode::default(), // Single icon by default
            switcher_shows_icons: true,
            menu_bar_shows_highest_usage: false,
            menu_bar_shows_percent: false,
            show_as_used: true,         // Show as "used" by default
            surprise_animations: false, // Off by default
            enable_animations: true,    // Animations enabled by default
            reset_time_relative: true,  // Show relative times by default
            menu_bar_display_mode: "detailed".to_string(), // Detailed mode by default
            show_credits_extra_usage: true, // Show credits + extra usage by default
            show_all_token_accounts_in_menu: false,
            claude_usage_source: default_claude_usage_source(),
            codex_usage_source: default_codex_usage_source(),
            codex_cookie_source: default_codex_cookie_source(),
            codex_historical_tracking: false,
            codex_openai_web_extras: default_codex_openai_web_extras(),
            claude_cookie_source: default_claude_cookie_source(),
            cursor_cookie_source: default_cursor_cookie_source(),
            opencode_cookie_source: default_opencode_cookie_source(),
            opencode_workspace_id: String::new(),
            factory_cookie_source: default_factory_cookie_source(),
            alibaba_cookie_source: default_alibaba_cookie_source(),
            alibaba_cookie_header: String::new(),
            alibaba_api_region: default_alibaba_api_region(),
            kimi_cookie_source: default_kimi_cookie_source(),
            kimi_manual_cookie_header: String::new(),
            minimax_cookie_source: default_minimax_cookie_source(),
            augment_cookie_source: default_augment_cookie_source(),
            augment_cookie_header: String::new(),
            amp_cookie_source: default_amp_cookie_source(),
            amp_cookie_header: String::new(),
            ollama_cookie_source: default_ollama_cookie_source(),
            ollama_cookie_header: String::new(),
            zai_api_region: default_zai_api_region(),
            jetbrains_ide_base_path: String::new(),
            minimax_cookie_header: String::new(),
            minimax_api_token: String::new(),
            minimax_api_region: default_minimax_api_region(),
            claude_avoid_keychain_prompts: false,
            show_debug_settings: false,
            disable_keychain_access: false,
            hide_personal_info: false, // Show personal info by default
            update_channel: UpdateChannel::default(), // Stable by default
            provider_metrics: HashMap::new(), // Empty = use Automatic for all
            global_shortcut: default_global_shortcut(), // Ctrl+Shift+U by default
            auto_download_updates: false, // Require explicit opt-in for background downloads
            install_updates_on_quit: false, // Don't auto-install on quit by default
            ui_language: Language::default(), // English by default
        }
    }
}

impl Settings {
    /// Get the settings file path
    pub fn settings_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("CodexBar").join("settings.json"))
    }

    /// Load settings from disk
    pub fn load() -> Self {
        #[allow(unused_mut)]
        let mut settings = match Self::settings_path() {
            Some(path) if path.exists() => match std::fs::read_to_string(&path) {
                Ok(content) => {
                    serde_json::from_str(content.trim_start_matches('\u{feff}')).unwrap_or_default()
                }
                Err(_) => Self::default(),
            },
            _ => Self::default(),
        };

        // Sync autostart toggle with actual registry state
        #[cfg(target_os = "windows")]
        {
            settings.start_at_login = Self::is_start_at_login_enabled();
        }

        settings
    }

    /// Save settings to disk
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::settings_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine settings path"))?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;

        Ok(())
    }

    fn start_at_login_command(exe_path: &std::path::Path) -> String {
        format!("\"{}\"", exe_path.display())
    }

    #[cfg(target_os = "windows")]
    pub fn apply_start_at_login_registry(enabled: bool) -> anyhow::Result<()> {
        use winreg::RegKey;
        use winreg::enums::*;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let run_key = hkcu.open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Run",
            KEY_READ | KEY_WRITE,
        )?;

        if enabled {
            let exe_path = std::env::current_exe()?;
            let command = Self::start_at_login_command(&exe_path);
            run_key.set_value("CodexBar", &command)?;
        } else {
            let _ = run_key.delete_value("CodexBar");
        }

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn apply_start_at_login_registry(_enabled: bool) -> anyhow::Result<()> {
        Ok(())
    }

    /// Set start at login (updates Windows registry)
    pub fn set_start_at_login(&mut self, enabled: bool) -> anyhow::Result<()> {
        self.start_at_login = enabled;
        Self::apply_start_at_login_registry(enabled)?;
        Ok(())
    }

    /// Check if start at login is actually enabled in registry
    #[cfg(target_os = "windows")]
    pub fn is_start_at_login_enabled() -> bool {
        use winreg::RegKey;
        use winreg::enums::*;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(run_key) = hkcu.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run") {
            run_key.get_value::<String, _>("CodexBar").is_ok()
        } else {
            false
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn is_start_at_login_enabled() -> bool {
        false
    }

    /// Check if a provider is enabled
    pub fn is_provider_enabled(&self, id: ProviderId) -> bool {
        self.enabled_providers.contains(id.cli_name())
    }

    /// Enable a provider
    pub fn enable_provider(&mut self, id: ProviderId) {
        self.enabled_providers.insert(id.cli_name().to_string());
    }

    /// Disable a provider
    pub fn disable_provider(&mut self, id: ProviderId) {
        self.enabled_providers.remove(id.cli_name());
    }

    /// Toggle a provider's enabled state
    pub fn toggle_provider(&mut self, id: ProviderId) -> bool {
        let name = id.cli_name().to_string();
        if self.enabled_providers.contains(&name) {
            self.enabled_providers.remove(&name);
            false
        } else {
            self.enabled_providers.insert(name);
            true
        }
    }

    /// Get list of enabled provider IDs
    pub fn get_enabled_provider_ids(&self) -> Vec<ProviderId> {
        ProviderId::all()
            .iter()
            .filter(|id| self.is_provider_enabled(**id))
            .copied()
            .collect()
    }

    /// Get all available providers with their enabled status
    pub fn get_all_providers_status(&self) -> Vec<ProviderStatus> {
        ProviderId::all()
            .iter()
            .map(|id| ProviderStatus {
                id: id.cli_name().to_string(),
                name: id.display_name().to_string(),
                enabled: self.is_provider_enabled(*id),
            })
            .collect()
    }

    /// Get the metric preference for a provider
    pub fn get_provider_metric(&self, id: ProviderId) -> MetricPreference {
        self.provider_metrics
            .get(id.cli_name())
            .copied()
            .unwrap_or_default()
    }

    /// Set the metric preference for a provider
    pub fn set_provider_metric(&mut self, id: ProviderId, metric: MetricPreference) {
        self.provider_metrics
            .insert(id.cli_name().to_string(), metric);
    }
}

/// Provider status for settings UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub id: String,
    pub name: String,
    pub enabled: bool,
}

/// Refresh interval options
#[derive(Debug, Clone, Serialize)]
pub struct RefreshIntervalOption {
    pub value: u64,
    pub label: String,
}

/// Get available refresh interval options
pub fn get_refresh_interval_options() -> Vec<RefreshIntervalOption> {
    vec![
        RefreshIntervalOption {
            value: 60,
            label: "1 minute".to_string(),
        },
        RefreshIntervalOption {
            value: 120,
            label: "2 minutes".to_string(),
        },
        RefreshIntervalOption {
            value: 300,
            label: "5 minutes".to_string(),
        },
        RefreshIntervalOption {
            value: 600,
            label: "10 minutes".to_string(),
        },
        RefreshIntervalOption {
            value: 900,
            label: "15 minutes".to_string(),
        },
        RefreshIntervalOption {
            value: 1800,
            label: "30 minutes".to_string(),
        },
    ]
}

/// Manual cookie storage
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ManualCookies {
    /// Provider ID -> cookie header mapping
    pub cookies: HashMap<String, ManualCookieEntry>,
}

/// A single manual cookie entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualCookieEntry {
    pub cookie_header: String,
    pub saved_at: String,
}

impl ManualCookies {
    /// Get the cookies file path
    pub fn cookies_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("CodexBar").join("manual_cookies.json"))
    }

    /// Load manual cookies from disk
    pub fn load() -> Self {
        if let Some(path) = Self::cookies_path()
            && path.exists()
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            return serde_json::from_str(&content).unwrap_or_default();
        }
        Self::default()
    }

    /// Save manual cookies to disk
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::cookies_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine cookies path"))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;

        Ok(())
    }

    /// Get cookie for a provider
    pub fn get(&self, provider_id: &str) -> Option<&str> {
        self.cookies
            .get(provider_id)
            .map(|e| e.cookie_header.as_str())
    }

    /// Set cookie for a provider
    pub fn set(&mut self, provider_id: &str, cookie_header: &str) {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string();
        self.cookies.insert(
            provider_id.to_string(),
            ManualCookieEntry {
                cookie_header: cookie_header.to_string(),
                saved_at: now,
            },
        );
    }

    /// Remove cookie for a provider
    pub fn remove(&mut self, provider_id: &str) {
        self.cookies.remove(provider_id);
    }

    /// Get all saved cookies for UI display
    pub fn get_all_for_display(&self) -> Vec<SavedCookieInfo> {
        self.cookies
            .iter()
            .map(|(id, entry)| {
                let provider_name = ProviderId::from_cli_name(id)
                    .map(|p| p.display_name().to_string())
                    .unwrap_or_else(|| id.clone());

                SavedCookieInfo {
                    provider_id: id.clone(),
                    provider: provider_name,
                    saved_at: entry.saved_at.clone(),
                }
            })
            .collect()
    }
}

/// Info about a saved cookie for UI display
#[derive(Debug, Clone, Serialize)]
pub struct SavedCookieInfo {
    pub provider_id: String,
    pub provider: String,
    pub saved_at: String,
}

/// API key storage for providers that need tokens
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiKeys {
    /// Provider ID -> API key mapping
    pub keys: HashMap<String, ApiKeyEntry>,
}

/// A single API key entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyEntry {
    pub api_key: String,
    pub saved_at: String,
    /// Optional label for the key (e.g., "Personal", "Work")
    #[serde(default)]
    pub label: Option<String>,
}

impl ApiKeys {
    /// Get the API keys file path
    pub fn keys_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("CodexBar").join("api_keys.json"))
    }

    /// Load API keys from disk
    pub fn load() -> Self {
        if let Some(path) = Self::keys_path()
            && path.exists()
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            return serde_json::from_str(&content).unwrap_or_default();
        }
        Self::default()
    }

    /// Save API keys to disk
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::keys_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine API keys path"))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;

        Ok(())
    }

    /// Get API key for a provider
    pub fn get(&self, provider_id: &str) -> Option<&str> {
        self.keys.get(provider_id).map(|e| e.api_key.as_str())
    }

    /// Set API key for a provider
    pub fn set(&mut self, provider_id: &str, api_key: &str, label: Option<&str>) {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string();
        self.keys.insert(
            provider_id.to_string(),
            ApiKeyEntry {
                api_key: api_key.to_string(),
                saved_at: now,
                label: label.map(|s| s.to_string()),
            },
        );
    }

    /// Remove API key for a provider
    pub fn remove(&mut self, provider_id: &str) {
        self.keys.remove(provider_id);
    }

    /// Check if a provider has an API key configured
    pub fn has_key(&self, provider_id: &str) -> bool {
        self.keys
            .get(provider_id)
            .map(|e| !e.api_key.is_empty())
            .unwrap_or(false)
    }

    /// Get all saved API keys for UI display (with masked values)
    pub fn get_all_for_display(&self) -> Vec<SavedApiKeyInfo> {
        self.keys
            .iter()
            .map(|(id, entry)| {
                let provider_name = ProviderId::from_cli_name(id)
                    .map(|p| p.display_name().to_string())
                    .unwrap_or_else(|| id.clone());

                // Mask the key for display (show first 4 and last 4 chars)
                let masked = if entry.api_key.len() > 12 {
                    format!(
                        "{}...{}",
                        &entry.api_key[..4],
                        &entry.api_key[entry.api_key.len() - 4..]
                    )
                } else if entry.api_key.len() > 4 {
                    format!("{}...", &entry.api_key[..4])
                } else {
                    "****".to_string()
                };

                SavedApiKeyInfo {
                    provider_id: id.clone(),
                    provider: provider_name,
                    masked_key: masked,
                    saved_at: entry.saved_at.clone(),
                    label: entry.label.clone(),
                }
            })
            .collect()
    }
}

/// Info about a saved API key for UI display
#[derive(Debug, Clone, Serialize)]
pub struct SavedApiKeyInfo {
    pub provider_id: String,
    pub provider: String,
    pub masked_key: String,
    pub saved_at: String,
    pub label: Option<String>,
}

/// Provider configuration info
#[derive(Debug, Clone)]
pub struct ProviderConfigInfo {
    pub id: ProviderId,
    pub name: &'static str,
    pub requires_api_key: bool,
    pub api_key_env_var: Option<&'static str>,
    pub api_key_help: Option<&'static str>,
    pub config_file_path: Option<&'static str>,
    pub dashboard_url: Option<&'static str>,
}

/// Get configuration info for providers that need API keys
pub fn get_api_key_providers() -> Vec<ProviderConfigInfo> {
    vec![
        ProviderConfigInfo {
            id: ProviderId::Alibaba,
            name: "Alibaba Coding Plan",
            requires_api_key: true,
            api_key_env_var: Some("ALIBABA_CODING_PLAN_API_KEY"),
            api_key_help: Some("Get your Coding Plan API key from Alibaba Model Studio / Bailian"),
            config_file_path: Some("~/.codexbar/config.json"),
            dashboard_url: Some(
                "https://modelstudio.console.alibabacloud.com/ap-southeast-1/?tab=coding-plan#/efm/detail",
            ),
        },
        ProviderConfigInfo {
            id: ProviderId::Amp,
            name: "Amp (Sourcegraph)",
            requires_api_key: true,
            api_key_env_var: Some("SRC_ACCESS_TOKEN"),
            api_key_help: Some("Get your token from Sourcegraph → Settings → Access Tokens"),
            config_file_path: Some("~/.amp/config.json"),
            dashboard_url: Some("https://sourcegraph.com/cody/manage"),
        },
        ProviderConfigInfo {
            id: ProviderId::Synthetic,
            name: "Synthetic",
            requires_api_key: true,
            api_key_env_var: Some("SYNTHETIC_API_KEY"),
            api_key_help: Some("Get your API key from Synthetic → Account → API Keys"),
            config_file_path: Some("~/.synthetic/config.json"),
            dashboard_url: Some("https://synthetic.computer/account"),
        },
        ProviderConfigInfo {
            id: ProviderId::Copilot,
            name: "GitHub Copilot",
            requires_api_key: true,
            api_key_env_var: Some("GITHUB_TOKEN"),
            api_key_help: Some("GitHub Personal Access Token with copilot scope"),
            config_file_path: None,
            dashboard_url: Some("https://github.com/settings/copilot"),
        },
        ProviderConfigInfo {
            id: ProviderId::Zai,
            name: "z.ai",
            requires_api_key: true,
            api_key_env_var: Some("ZAI_API_TOKEN"),
            api_key_help: Some("Get your API token from z.ai Dashboard → Settings"),
            config_file_path: None,
            dashboard_url: Some("https://z.ai/dashboard"),
        },
        ProviderConfigInfo {
            id: ProviderId::Warp,
            name: "Warp",
            requires_api_key: true,
            api_key_env_var: Some("WARP_API_KEY"),
            api_key_help: Some(
                "Get your API key from Warp → Settings → API Keys (docs.warp.dev/reference/cli/api-keys)",
            ),
            config_file_path: None,
            dashboard_url: Some("https://docs.warp.dev/reference/cli/api-keys"),
        },
        ProviderConfigInfo {
            id: ProviderId::OpenRouter,
            name: "OpenRouter",
            requires_api_key: true,
            api_key_env_var: Some("OPENROUTER_API_KEY"),
            api_key_help: Some("Get your API key from openrouter.ai/settings/keys"),
            config_file_path: None,
            dashboard_url: Some("https://openrouter.ai/settings/credits"),
        },
        ProviderConfigInfo {
            id: ProviderId::NanoGPT,
            name: "NanoGPT",
            requires_api_key: true,
            api_key_env_var: Some("NANOGPT_API_KEY"),
            api_key_help: Some("Get your API key from nano-gpt.com/api"),
            config_file_path: None,
            dashboard_url: Some("https://nano-gpt.com/api"),
        },
        ProviderConfigInfo {
            id: ProviderId::Infini,
            name: "Infini AI",
            requires_api_key: true,
            api_key_env_var: Some("INFINI_API_KEY"),
            api_key_help: Some("Get your API key from Infini Cloud → Settings → API Keys"),
            config_file_path: None,
            dashboard_url: Some("https://cloud.infini-ai.com"),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert!(settings.enabled_providers.contains("claude"));
        assert!(settings.enabled_providers.contains("codex"));
        assert_eq!(settings.refresh_interval_secs, 300);
        assert!(settings.show_notifications);
        assert_eq!(settings.high_usage_threshold, 70.0);
        assert_eq!(settings.critical_usage_threshold, 90.0);
    }

    #[test]
    fn test_settings_provider_enabled() {
        let settings = Settings::default();
        assert!(settings.is_provider_enabled(ProviderId::Claude));
        assert!(settings.is_provider_enabled(ProviderId::Codex));
        assert!(!settings.is_provider_enabled(ProviderId::Gemini));
    }

    #[test]
    fn test_settings_toggle_provider() {
        let mut settings = Settings::default();

        // Claude starts enabled
        assert!(settings.is_provider_enabled(ProviderId::Claude));

        // Toggle off
        let enabled = settings.toggle_provider(ProviderId::Claude);
        assert!(!enabled);
        assert!(!settings.is_provider_enabled(ProviderId::Claude));

        // Toggle back on
        let enabled = settings.toggle_provider(ProviderId::Claude);
        assert!(enabled);
        assert!(settings.is_provider_enabled(ProviderId::Claude));
    }

    #[test]
    fn test_settings_get_enabled_provider_ids() {
        let settings = Settings::default();
        let enabled = settings.get_enabled_provider_ids();
        assert!(enabled.contains(&ProviderId::Claude));
        assert!(enabled.contains(&ProviderId::Codex));
    }

    #[test]
    fn test_settings_get_all_providers_status() {
        let settings = Settings::default();
        let status = settings.get_all_providers_status();
        assert_eq!(status.len(), ProviderId::all().len());

        let claude_status = status.iter().find(|s| s.id == "claude").unwrap();
        assert_eq!(claude_status.name, "Claude");
        assert!(claude_status.enabled);

        let gemini_status = status.iter().find(|s| s.id == "gemini").unwrap();
        assert!(!gemini_status.enabled);
    }

    #[test]
    fn test_refresh_interval_options() {
        let options = get_refresh_interval_options();
        assert!(!options.is_empty());
        assert!(options.iter().any(|o| o.value == 60));
        assert!(options.iter().any(|o| o.value == 300));
    }

    #[test]
    fn test_manual_cookies_default() {
        let cookies = ManualCookies::default();
        assert!(cookies.cookies.is_empty());
    }

    #[test]
    fn test_manual_cookies_set_get_remove() {
        let mut cookies = ManualCookies::default();

        // Set a cookie
        cookies.set("claude", "session=abc123");
        assert_eq!(cookies.get("claude"), Some("session=abc123"));

        // Remove it
        cookies.remove("claude");
        assert_eq!(cookies.get("claude"), None);
    }

    #[test]
    fn test_start_at_login_command_uses_only_the_executable_path() {
        let path =
            std::path::PathBuf::from(r"C:\Program Files\CodexBar\codexbar-desktop-tauri.exe");
        let command = Settings::start_at_login_command(&path);
        assert_eq!(
            command,
            "\"C:\\Program Files\\CodexBar\\codexbar-desktop-tauri.exe\""
        );
        assert!(!command.contains("menubar"));
    }

    #[test]
    fn test_language_defaults_to_english() {
        let settings = Settings::default();
        assert_eq!(settings.ui_language, Language::English);
    }

    #[test]
    fn test_language_all_variants_available() {
        let languages = Language::all();
        assert_eq!(languages.len(), 2);
        assert!(languages.contains(&Language::English));
        assert!(languages.contains(&Language::Chinese));
    }

    #[test]
    fn test_language_display_names() {
        assert_eq!(Language::English.display_name(), "English");
        assert_eq!(Language::Chinese.display_name(), "中文");
    }

    #[test]
    fn test_settings_load_missing_language_field_defaults_to_english() {
        // Simulate loading legacy settings JSON without ui_language field
        let legacy_json = r#"{
            "enabled_providers": ["claude", "codex"],
            "refresh_interval_secs": 300,
            "start_minimized": false,
            "ui_language": "english"
        }"#;

        let settings: Result<Settings, _> = serde_json::from_str(legacy_json);
        assert!(settings.is_ok());
        let settings = settings.unwrap();
        assert_eq!(settings.ui_language, Language::English);
    }

    #[test]
    fn test_settings_roundtrip_with_language() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create settings with Chinese language
        let settings = Settings {
            ui_language: Language::Chinese,
            ..Settings::default()
        };

        // Save to a temp file
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let json = serde_json::to_string_pretty(&settings).expect("Failed to serialize settings");
        temp_file
            .write_all(json.as_bytes())
            .expect("Failed to write settings");
        let path = temp_file.path().to_path_buf();

        // Read back and verify
        let content = std::fs::read_to_string(&path).expect("Failed to read settings");
        let loaded: Settings =
            serde_json::from_str(&content).expect("Failed to deserialize settings");

        assert_eq!(loaded.ui_language, Language::Chinese);
    }

    #[test]
    fn test_settings_with_utf8_bom_parses_perprovider_tray_mode() {
        let json = "\u{feff}{\n            \"enabled_providers\": [\"claude\", \"codex\"],\n            \"refresh_interval_secs\": 300,\n            \"tray_icon_mode\": \"perprovider\"\n        }";

        let settings: Settings = serde_json::from_str(json.trim_start_matches('\u{feff}')).unwrap();

        assert_eq!(settings.tray_icon_mode, TrayIconMode::PerProvider);
    }

    #[test]
    fn test_language_serde_serialization() {
        // Test that Language serializes to lowercase string
        let english = Language::English;
        let chinese = Language::Chinese;

        let english_json = serde_json::to_string(&english).unwrap();
        let chinese_json = serde_json::to_string(&chinese).unwrap();

        assert_eq!(english_json, "\"english\"");
        assert_eq!(chinese_json, "\"chinese\"");
    }

    #[test]
    fn test_language_serde_deserialization() {
        // Test that lowercase strings deserialize correctly
        let english: Language = serde_json::from_str("\"english\"").unwrap();
        let chinese: Language = serde_json::from_str("\"chinese\"").unwrap();

        assert_eq!(english, Language::English);
        assert_eq!(chinese, Language::Chinese);
    }
}
