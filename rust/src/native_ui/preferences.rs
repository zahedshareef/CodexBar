//! Preferences window for CodexBar
//!
//! A refined settings interface inspired by Linear and Apple Settings.
//! Design principle: Precision Calm - clear hierarchy, generous spacing, subtle depth.

#![allow(dead_code)] // Legacy show_* methods kept for potential future use

use eframe::egui::{self, Color32, Rect, RichText, Rounding, Stroke, Vec2};
use image::ColorType;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::provider_icons::ProviderIconCache;
use super::theme::{FontSize, Radius, Spacing, Theme, provider_color, provider_icon};
use crate::browser::cookies::get_cookie_header_from_browser;
use crate::browser::detection::{BrowserDetector, BrowserType};
use crate::core::{ProviderAccountData, TokenAccount, TokenAccountStore, TokenAccountSupport};
use crate::core::{ProviderId, WidgetProviderEntry, WidgetSnapshot, WidgetSnapshotStore};
use crate::locale::{LocaleKey, get_text as locale_text};
use crate::settings::{
    ApiKeys, Language, ManualCookies, Settings, TrayIconMode, get_api_key_providers,
};
use crate::shortcuts::format_shortcut;
use std::collections::HashMap;

// Thread-local icon cache for viewport rendering
thread_local! {
    static VIEWPORT_ICON_CACHE: RefCell<ProviderIconCache> = RefCell::new(ProviderIconCache::new());
}

#[cfg(debug_assertions)]
fn save_color_image_to_png(path: &std::path::Path, image: &egui::ColorImage) -> anyhow::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let mut rgba = Vec::with_capacity(image.pixels.len() * 4);
    for pixel in &image.pixels {
        rgba.extend_from_slice(&pixel.to_srgba_unmultiplied());
    }

    image::save_buffer(
        path,
        &rgba,
        image.size[0] as u32,
        image.size[1] as u32,
        ColorType::Rgba8,
    )?;

    Ok(())
}

#[cfg(debug_assertions)]
fn append_preferences_screenshot_debug_log(message: &str) {
    let path = std::env::temp_dir().join("codexbar_preferences_screenshot.log");
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| {
            use std::io::Write;
            writeln!(file, "{}", message)
        });
}

/// Which preferences tab is active
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PreferencesTab {
    #[default]
    General,
    Providers,
    Display,
    ApiKeys,
    Cookies,
    Advanced,
    About,
    /// Consolidated: Providers + API Keys + Cookies
    Accounts,
    /// Extracted shortcuts surface
    Shortcuts,
    /// Consolidated: General + Display + Advanced
    Preferences,
}

impl PreferencesTab {
    #[cfg(debug_assertions)]
    fn from_test_label(label: &str) -> Option<Self> {
        let normalized = label.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "general" => Some(PreferencesTab::General),
            "providers" => Some(PreferencesTab::Providers),
            "display" => Some(PreferencesTab::Display),
            "api_keys" | "apikeys" | "api-keys" => Some(PreferencesTab::ApiKeys),
            "cookies" => Some(PreferencesTab::Cookies),
            "advanced" => Some(PreferencesTab::Advanced),
            "about" => Some(PreferencesTab::About),
            "accounts" => Some(PreferencesTab::Accounts),
            "shortcuts" => Some(PreferencesTab::Shortcuts),
            "preferences" => Some(PreferencesTab::Preferences),
            _ => None,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            PreferencesTab::General => "General",
            PreferencesTab::Providers => "Providers",
            PreferencesTab::Display => "Display",
            PreferencesTab::ApiKeys => "API Keys",
            PreferencesTab::Cookies => "Cookies",
            PreferencesTab::Advanced => "Advanced",
            PreferencesTab::About => "About",
            PreferencesTab::Accounts => "Accounts",
            PreferencesTab::Shortcuts => "Shortcuts",
            PreferencesTab::Preferences => "Preferences",
        }
    }

    fn icon(&self) -> &'static str {
        // Coherent set: all single-weight outlined geometric symbols
        // that render consistently across egui/Windows font stacks
        match self {
            PreferencesTab::General | PreferencesTab::Preferences => "⚙",
            PreferencesTab::Providers | PreferencesTab::Accounts => "⊞",
            PreferencesTab::Display => "◐",
            PreferencesTab::ApiKeys => "⊞",
            PreferencesTab::Cookies => "⊟",
            PreferencesTab::Advanced => "⚡",
            PreferencesTab::About => "ⓘ",
            PreferencesTab::Shortcuts => "⌗",
        }
    }

    /// The 4 intent-driven top-level categories shown in navigation
    fn top_level_tabs() -> &'static [PreferencesTab] {
        &[
            PreferencesTab::Preferences,
            PreferencesTab::Accounts,
            PreferencesTab::Shortcuts,
            PreferencesTab::About,
        ]
    }
}

#[cfg(debug_assertions)]
#[derive(Clone, Debug)]
pub(crate) struct PreferencesDebugTabTarget {
    pub name: String,
    pub rect: Rect,
    pub hovered: bool,
    pub contains_pointer: bool,
    pub clicked: bool,
    pub pointer_button_down_on: bool,
    pub interact_pointer_pos: Option<egui::Pos2>,
}

#[cfg(debug_assertions)]
#[derive(Clone, Debug)]
pub(crate) struct PreferencesDebugStatusMessage {
    pub message: String,
    pub is_error: bool,
}

#[cfg(debug_assertions)]
#[derive(Clone, Debug)]
pub(crate) struct PreferencesDebugSettingsSnapshot {
    pub enabled_providers: Vec<String>,
    pub refresh_interval_secs: u64,
    pub menu_bar_display_mode: String,
    pub reset_time_relative: bool,
    pub surprise_animations: bool,
    pub show_as_used: bool,
    pub show_credits_extra_usage: bool,
    pub merge_tray_icons: bool,
    pub tray_icon_mode: String,
    pub selected_provider: Option<String>,
}

fn set_merge_tray_icons(settings: &mut Settings, enabled: bool) {
    settings.merge_tray_icons = enabled;
    if enabled {
        settings.tray_icon_mode = TrayIconMode::Single;
    }
}

fn set_per_provider_tray_icons(settings: &mut Settings, enabled: bool) {
    settings.tray_icon_mode = if enabled {
        TrayIconMode::PerProvider
    } else {
        TrayIconMode::Single
    };
    if enabled {
        settings.merge_tray_icons = false;
    }
}

fn preferences_section_title(tab: PreferencesTab) -> &'static str {
    match tab {
        PreferencesTab::Display => "Display",
        PreferencesTab::Advanced => "Advanced",
        _ => "General",
    }
}

fn preferences_section_subtitle(tab: PreferencesTab) -> &'static str {
    match tab {
        PreferencesTab::Display => "Tune how the menu bar looks, reads, and prioritizes detail.",
        PreferencesTab::Advanced => {
            "Control refresh cadence, privacy behavior, and power-user knobs."
        }
        _ => "Core behavior, startup defaults, alerts, and update preferences.",
    }
}

fn preferences_tab_shell_label(ui_language: Language) -> &'static str {
    match ui_language {
        Language::Chinese => "偏好设置",
        _ => "Preferences",
    }
}

/// Preferences window state
pub struct PreferencesWindow {
    pub is_open: bool,
    pub active_tab: PreferencesTab,
    preferences_section: PreferencesTab,
    pub settings: Settings,
    pub settings_changed: bool,
    cookies: ManualCookies,
    new_cookie_provider: String,
    new_cookie_value: String,
    cookie_status_msg: Option<(String, bool)>,
    api_keys: ApiKeys,
    new_api_key_provider: String,
    new_api_key_value: String,
    show_api_key_input: bool,
    api_key_status_msg: Option<(String, bool)>,
    // Selected provider in Providers tab (sidebar selection)
    pub selected_provider: Option<ProviderId>,
    // Browser cookie import state
    selected_browser: Option<BrowserType>,
    browser_import_status: Option<(String, bool)>, // (message, is_error)
    // Icon cache for provider SVG icons
    icon_cache: ProviderIconCache,
    // Shared state for viewport
    shared_state: Arc<Mutex<PreferencesSharedState>>,
    // Place settings viewport beside the main menu window on next show()
    needs_viewport_placement: bool,
}

/// Shared state that can be accessed from viewport
#[derive(Clone)]
struct PreferencesSharedState {
    is_open: bool,
    active_tab: PreferencesTab,
    preferences_section: PreferencesTab,
    settings: Settings,
    settings_changed: bool,
    cookies: ManualCookies,
    new_cookie_provider: String,
    new_cookie_value: String,
    cookie_status_msg: Option<(String, bool)>,
    api_keys: ApiKeys,
    new_api_key_provider: String,
    new_api_key_value: String,
    show_api_key_input: bool,
    api_key_status_msg: Option<(String, bool)>,
    selected_provider: Option<ProviderId>,
    selected_browser: Option<BrowserType>,
    browser_import_status: Option<(String, bool)>,
    refresh_requested: bool,
    cached_snapshot: Option<WidgetSnapshot>,
    runtime_provider_errors: HashMap<ProviderId, String>,
    // Token accounts data
    token_accounts: HashMap<ProviderId, ProviderAccountData>,
    new_account_label: String,
    new_account_token: String,
    show_add_account_input: bool,
    token_account_status_msg: Option<(String, bool)>,
    // Keyboard shortcut editing
    shortcut_input: String,
    shortcut_status_msg: Option<(String, bool)>,
    #[cfg(debug_assertions)]
    debug_tab_targets: Vec<PreferencesDebugTabTarget>,
    #[cfg(debug_assertions)]
    debug_viewport_outer_rect: Option<Rect>,
    #[cfg(debug_assertions)]
    pending_screenshot_path: Option<PathBuf>,
    #[cfg(debug_assertions)]
    pending_screenshot_delay_frames: u8,
    #[cfg(debug_assertions)]
    pending_screenshot_attempts: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ProviderSidebarStyle {
    frame_fill: Option<Color32>,
    frame_stroke: Option<Stroke>,
    inner_margin: f32,
    item_spacing_y: f32,
    row_height: f32,
    row_corner_radius: f32,
    selected_fill: Color32,
    selected_stroke: Stroke,
    hover_fill: Color32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ProviderDetailChrome {
    control_fill: Color32,
    control_fill_hover: Color32,
    control_fill_active: Color32,
    control_stroke: Stroke,
    control_inner_margin_x: f32,
    control_inner_margin_y: f32,
    info_grid_spacing_x: f32,
    info_grid_spacing_y: f32,
    section_gap: f32,
    detail_label_width: f32,
    picker_label_width: f32,
    metric_bar_width: f32,
    refresh_button_size: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ProvidersSurfacePalette {
    shell_fill: Color32,
    content_fill: Color32,
    detail_fill: Color32,
    detail_stroke: Stroke,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SettingsNavChrome {
    selected_fill: Color32,
    selected_stroke: Stroke,
    hover_fill: Color32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ProviderDetailTextChrome {
    subtitle: Color32,
    section_title: Color32,
    helper: Color32,
    info_label: Color32,
    secondary_value: Color32,
}

fn active_provider_sidebar_style() -> ProviderSidebarStyle {
    ProviderSidebarStyle {
        frame_fill: Some(Color32::from_rgba_unmultiplied(255, 255, 255, 8)),
        frame_stroke: Some(Stroke::new(1.0, Theme::BORDER_SUBTLE.gamma_multiply(0.56))),
        inner_margin: Spacing::SM,
        item_spacing_y: 0.0,
        row_height: 58.0,
        row_corner_radius: 7.0,
        selected_fill: Color32::from_rgba_unmultiplied(255, 255, 255, 11),
        selected_stroke: Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 18)),
        hover_fill: Color32::from_rgba_unmultiplied(255, 255, 255, 4),
    }
}

fn providers_surface_palette() -> ProvidersSurfacePalette {
    ProvidersSurfacePalette {
        shell_fill: Color32::from_rgb(56, 56, 64),
        content_fill: Color32::from_rgb(54, 54, 62),
        detail_fill: Color32::from_rgb(58, 58, 66),
        detail_stroke: Stroke::new(1.0, Theme::BORDER_SUBTLE.gamma_multiply(0.24)),
    }
}

fn settings_nav_chrome() -> SettingsNavChrome {
    SettingsNavChrome {
        selected_fill: Color32::from_rgba_unmultiplied(255, 255, 255, 10),
        selected_stroke: Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 20)),
        hover_fill: Color32::from_rgba_unmultiplied(255, 255, 255, 4),
    }
}

fn provider_detail_text_chrome() -> ProviderDetailTextChrome {
    ProviderDetailTextChrome {
        subtitle: Theme::TEXT_SECONDARY.gamma_multiply(1.16),
        section_title: Theme::TEXT_PRIMARY.gamma_multiply(0.84),
        helper: Theme::TEXT_SECONDARY.gamma_multiply(1.08),
        info_label: Theme::TEXT_SECONDARY.gamma_multiply(1.12),
        secondary_value: Theme::TEXT_SECONDARY.gamma_multiply(1.02),
    }
}

fn provider_detail_chrome() -> ProviderDetailChrome {
    ProviderDetailChrome {
        control_fill: Color32::from_rgba_unmultiplied(255, 255, 255, 5),
        control_fill_hover: Color32::from_rgba_unmultiplied(255, 255, 255, 8),
        control_fill_active: Color32::from_rgba_unmultiplied(255, 255, 255, 11),
        control_stroke: Stroke::new(1.0, Theme::BORDER_SUBTLE.gamma_multiply(0.28)),
        control_inner_margin_x: 8.0,
        control_inner_margin_y: 0.0,
        info_grid_spacing_x: 14.0,
        info_grid_spacing_y: 6.0,
        section_gap: 12.0,
        detail_label_width: 92.0,
        picker_label_width: 92.0,
        metric_bar_width: 220.0,
        refresh_button_size: 24.0,
    }
}

fn provider_detail_max_content_width() -> f32 {
    404.0
}

impl Default for PreferencesWindow {
    fn default() -> Self {
        let settings = Settings::load();
        let cookies = ManualCookies::load();
        let api_keys = ApiKeys::load();
        let token_accounts = TokenAccountStore::new().load().unwrap_or_default();

        let shared_state = Arc::new(Mutex::new(PreferencesSharedState {
            is_open: false,
            active_tab: PreferencesTab::General,
            preferences_section: PreferencesTab::General,
            settings: settings.clone(),
            settings_changed: false,
            cookies: cookies.clone(),
            new_cookie_provider: String::new(),
            new_cookie_value: String::new(),
            cookie_status_msg: None,
            api_keys: api_keys.clone(),
            new_api_key_provider: String::new(),
            new_api_key_value: String::new(),
            show_api_key_input: false,
            api_key_status_msg: None,
            selected_provider: None,
            selected_browser: None,
            browser_import_status: None,
            refresh_requested: false,
            cached_snapshot: WidgetSnapshotStore::load(),
            runtime_provider_errors: HashMap::new(),
            token_accounts: token_accounts.clone(),
            new_account_label: String::new(),
            new_account_token: String::new(),
            show_add_account_input: false,
            token_account_status_msg: None,
            shortcut_input: settings.global_shortcut.clone(),
            shortcut_status_msg: None,
            #[cfg(debug_assertions)]
            debug_tab_targets: Vec::new(),
            #[cfg(debug_assertions)]
            debug_viewport_outer_rect: None,
            #[cfg(debug_assertions)]
            pending_screenshot_path: None,
            #[cfg(debug_assertions)]
            pending_screenshot_delay_frames: 0,
            #[cfg(debug_assertions)]
            pending_screenshot_attempts: 0,
        }));

        Self {
            is_open: false,
            active_tab: PreferencesTab::General,
            preferences_section: PreferencesTab::General,
            settings,
            settings_changed: false,
            cookies,
            new_cookie_provider: String::new(),
            new_cookie_value: String::new(),
            cookie_status_msg: None,
            api_keys,
            new_api_key_provider: String::new(),
            new_api_key_value: String::new(),
            show_api_key_input: false,
            api_key_status_msg: None,
            selected_provider: None,
            selected_browser: None,
            browser_import_status: None,
            icon_cache: ProviderIconCache::new(),
            shared_state,
            needs_viewport_placement: false,
        }
    }
}

impl PreferencesWindow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.is_open = true;
        self.needs_viewport_placement = true;
        self.settings = Settings::load();
        self.cookies = ManualCookies::load();
        self.api_keys = ApiKeys::load();
        self.settings_changed = false;
        self.cookie_status_msg = None;
        self.api_key_status_msg = None;
        self.new_api_key_value.clear();
        self.show_api_key_input = false;

        // Sync to shared state and reload snapshot for fresh data after any background refreshes
        if let Ok(mut state) = self.shared_state.lock() {
            state.is_open = true;
            state.active_tab = self.active_tab;
            state.preferences_section = self.preferences_section;
            state.settings = self.settings.clone();
            state.cookies = self.cookies.clone();
            state.api_keys = self.api_keys.clone();
            state.settings_changed = false;
            state.cached_snapshot = WidgetSnapshotStore::load();
            state.selected_provider = self.selected_provider;
            state.shortcut_input = self.settings.global_shortcut.clone();
            state.shortcut_status_msg = None;
            state.debug_viewport_outer_rect = None;
        }
    }

    pub fn close(&mut self) {
        // Flush any unsaved settings through the same atomic path used per-frame.
        if let Some(settings) = self.take_settings_if_changed() {
            let _ = settings.save();
        }

        self.is_open = false;
        if let Ok(mut state) = self.shared_state.lock() {
            state.is_open = false;
            state.debug_viewport_outer_rect = None;
        }
        self.needs_viewport_placement = false;
    }

    #[cfg(debug_assertions)]
    pub(crate) fn select_tab_for_testing(&mut self, tab: &str) {
        let target_tab = match PreferencesTab::from_test_label(tab) {
            Some(tab) => tab,
            None => {
                tracing::warn!("Unknown preferences test tab selection: {}", tab);
                return;
            }
        };

        if !self.is_open {
            self.open();
        }

        self.active_tab = target_tab;
        if matches!(
            target_tab,
            PreferencesTab::General | PreferencesTab::Display | PreferencesTab::Advanced
        ) {
            self.preferences_section = target_tab;
        }
        if let Ok(mut state) = self.shared_state.lock() {
            state.is_open = true;
            state.active_tab = target_tab;
            if matches!(
                target_tab,
                PreferencesTab::General | PreferencesTab::Display | PreferencesTab::Advanced
            ) {
                state.preferences_section = target_tab;
            }
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn select_provider_for_testing(&mut self, provider: &str) {
        let Some(provider_id) = ProviderId::from_cli_name(provider.trim()) else {
            tracing::warn!("Unknown preferences test provider selection: {}", provider);
            return;
        };

        if !self.is_open {
            self.open();
        }

        self.active_tab = PreferencesTab::Providers;
        self.selected_provider = Some(provider_id);
        if let Ok(mut state) = self.shared_state.lock() {
            state.is_open = true;
            state.active_tab = PreferencesTab::Providers;
            state.selected_provider = Some(provider_id);
        }
    }

    /// Check if a refresh was requested and reset the flag
    pub fn take_refresh_requested(&mut self) -> bool {
        if let Ok(mut state) = self.shared_state.lock()
            && state.refresh_requested
        {
            state.refresh_requested = false;
            return true;
        }
        false
    }

    /// Atomically check if settings changed and return the new settings if so.
    /// Clears the flag in both `PreferencesWindow` and the shared viewport state
    /// so duplicate saves cannot happen across frames.
    pub fn take_settings_if_changed(&mut self) -> Option<Settings> {
        if let Ok(mut state) = self.shared_state.lock()
            && state.settings_changed
        {
            state.settings_changed = false;
            self.settings = state.settings.clone();
            self.settings_changed = false;
            return Some(self.settings.clone());
        }
        self.settings_changed = false;
        None
    }

    pub fn current_settings(&self) -> Settings {
        if let Ok(state) = self.shared_state.lock() {
            state.settings.clone()
        } else {
            self.settings.clone()
        }
    }

    /// Reload the cached snapshot from disk (call after refresh completes)
    pub fn reload_snapshot(&mut self) {
        if let Ok(mut state) = self.shared_state.lock() {
            state.cached_snapshot = WidgetSnapshotStore::load();
        }
    }

    pub fn set_runtime_provider_errors(&mut self, errors: HashMap<ProviderId, String>) {
        if let Ok(mut state) = self.shared_state.lock() {
            state.runtime_provider_errors = errors;
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn debug_snapshot(
        &self,
    ) -> (
        Vec<PreferencesDebugTabTarget>,
        Option<Rect>,
        PreferencesDebugSettingsSnapshot,
        Option<PreferencesDebugStatusMessage>,
        Option<PreferencesDebugStatusMessage>,
    ) {
        if let Ok(state) = self.shared_state.lock() {
            let mut enabled_providers = state
                .settings
                .enabled_providers
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            enabled_providers.sort();
            (
                state.debug_tab_targets.clone(),
                state.debug_viewport_outer_rect,
                PreferencesDebugSettingsSnapshot {
                    enabled_providers,
                    refresh_interval_secs: state.settings.refresh_interval_secs,
                    menu_bar_display_mode: state.settings.menu_bar_display_mode.clone(),
                    reset_time_relative: state.settings.reset_time_relative,
                    surprise_animations: state.settings.surprise_animations,
                    show_as_used: state.settings.show_as_used,
                    show_credits_extra_usage: state.settings.show_credits_extra_usage,
                    merge_tray_icons: state.settings.merge_tray_icons,
                    tray_icon_mode: match state.settings.tray_icon_mode {
                        TrayIconMode::Single => "single".to_string(),
                        // Match settings.json serialization so debug exports and persisted
                        // state speak the same enum value.
                        TrayIconMode::PerProvider => "perprovider".to_string(),
                    },
                    selected_provider: state
                        .selected_provider
                        .map(|provider_id| provider_id.cli_name().to_string()),
                },
                state
                    .api_key_status_msg
                    .as_ref()
                    .map(|(message, is_error)| PreferencesDebugStatusMessage {
                        message: message.clone(),
                        is_error: *is_error,
                    }),
                state.cookie_status_msg.as_ref().map(|(message, is_error)| {
                    PreferencesDebugStatusMessage {
                        message: message.clone(),
                        is_error: *is_error,
                    }
                }),
            )
        } else {
            (
                Vec::new(),
                None,
                PreferencesDebugSettingsSnapshot {
                    enabled_providers: Vec::new(),
                    refresh_interval_secs: 0,
                    menu_bar_display_mode: "detailed".to_string(),
                    reset_time_relative: false,
                    surprise_animations: false,
                    show_as_used: false,
                    show_credits_extra_usage: false,
                    merge_tray_icons: false,
                    tray_icon_mode: "single".to_string(),
                    selected_provider: None,
                },
                None,
                None,
            )
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn request_screenshot_for_testing(&mut self, path: PathBuf) {
        if !self.is_open {
            self.open();
        }
        if let Ok(mut state) = self.shared_state.lock() {
            append_preferences_screenshot_debug_log(&format!(
                "request path={} is_open={} active_tab={:?}",
                path.display(),
                state.is_open,
                state.active_tab
            ));
            state.is_open = true;
            state.pending_screenshot_path = Some(path);
            state.pending_screenshot_delay_frames = 3;
            state.pending_screenshot_attempts = 0;
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn set_api_key_input_for_testing(&mut self, provider: &str, value: &str) {
        if !self.is_open {
            self.open();
        }

        self.active_tab = PreferencesTab::ApiKeys;
        if let Ok(mut state) = self.shared_state.lock() {
            state.is_open = true;
            state.active_tab = PreferencesTab::ApiKeys;
            state.new_api_key_provider = provider.trim().to_string();
            state.new_api_key_value = value.to_string();
            state.show_api_key_input = true;
            state.api_key_status_msg = None;
            state.cookie_status_msg = None;
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn set_provider_enabled_for_testing(&mut self, provider: &str, enabled: bool) {
        if !self.is_open {
            self.open();
        }

        self.active_tab = PreferencesTab::Providers;
        if let Ok(mut state) = self.shared_state.lock() {
            state.is_open = true;
            state.active_tab = PreferencesTab::Providers;
            let provider = provider.trim().to_string();
            if enabled {
                state.settings.enabled_providers.insert(provider);
            } else {
                state.settings.enabled_providers.remove(&provider);
            }
            state.settings_changed = true;
            state.refresh_requested = true;
            state.api_key_status_msg = None;
            state.cookie_status_msg = None;
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn set_refresh_interval_for_testing(&mut self, seconds: u64) {
        if !self.is_open {
            self.open();
        }

        self.active_tab = PreferencesTab::Advanced;
        if let Ok(mut state) = self.shared_state.lock() {
            state.is_open = true;
            state.active_tab = PreferencesTab::Advanced;
            state.settings.refresh_interval_secs = seconds;
            state.settings_changed = true;
            state.api_key_status_msg = None;
            state.cookie_status_msg = None;
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn set_display_setting_for_testing(&mut self, name: &str, enabled: bool) {
        if !self.is_open {
            self.open();
        }

        self.active_tab = PreferencesTab::Display;
        if let Ok(mut state) = self.shared_state.lock() {
            state.is_open = true;
            state.active_tab = PreferencesTab::Display;
            match name.trim() {
                "reset_time_relative" => state.settings.reset_time_relative = enabled,
                "surprise_animations" => state.settings.surprise_animations = enabled,
                "show_as_used" => state.settings.show_as_used = enabled,
                "show_credits_extra_usage" => state.settings.show_credits_extra_usage = enabled,
                "merge_tray_icons" => set_merge_tray_icons(&mut state.settings, enabled),
                "per_provider_tray_icons" => {
                    set_per_provider_tray_icons(&mut state.settings, enabled);
                }
                other => {
                    tracing::warn!("Unknown display test setting: {}", other);
                    return;
                }
            }
            state.settings_changed = true;
            state.api_key_status_msg = None;
            state.cookie_status_msg = None;
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn set_display_mode_for_testing(&mut self, mode: &str) {
        if !self.is_open {
            self.open();
        }

        self.active_tab = PreferencesTab::Display;
        if let Ok(mut state) = self.shared_state.lock() {
            let mode = mode.trim();
            if !matches!(mode, "minimal" | "compact" | "detailed") {
                tracing::warn!("Unknown display mode for testing: {}", mode);
                return;
            }
            state.is_open = true;
            state.active_tab = PreferencesTab::Display;
            state.settings.menu_bar_display_mode = mode.to_string();
            state.settings_changed = true;
            state.api_key_status_msg = None;
            state.cookie_status_msg = None;
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn submit_api_key_for_testing(&mut self) {
        if !self.is_open {
            self.open();
        }

        if let Ok(mut state) = self.shared_state.lock() {
            let ui_language = state.settings.ui_language;
            let provider = state.new_api_key_provider.trim().to_string();
            let value = state.new_api_key_value.trim().to_string();
            let provider_name = ProviderId::from_cli_name(&provider)
                .map(|id| id.display_name().to_string())
                .unwrap_or_else(|| provider.clone());
            state.cookie_status_msg = None;

            if provider.is_empty() || value.is_empty() {
                state.api_key_status_msg = Some((
                    locale_text(ui_language, LocaleKey::SaveFailed)
                        .replace("{}", "missing provider or value"),
                    true,
                ));
                return;
            }

            state.api_keys.set(&provider, &value, None);
            if let Err(error) = state.api_keys.save() {
                state.api_key_status_msg = Some((
                    locale_text(ui_language, LocaleKey::SaveFailed)
                        .replace("{}", &error.to_string()),
                    true,
                ));
            } else {
                state.api_key_status_msg = Some((
                    locale_text(ui_language, LocaleKey::ApiKeySaved).replace("{}", &provider_name),
                    false,
                ));
                state.show_api_key_input = false;
                state.new_api_key_value.clear();
            }
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn set_cookie_input_for_testing(&mut self, provider: &str, value: &str) {
        if !self.is_open {
            self.open();
        }

        self.active_tab = PreferencesTab::Cookies;
        if let Ok(mut state) = self.shared_state.lock() {
            state.is_open = true;
            state.active_tab = PreferencesTab::Cookies;
            state.new_cookie_provider = provider.trim().to_string();
            state.new_cookie_value = value.to_string();
            state.api_key_status_msg = None;
            state.cookie_status_msg = None;
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn submit_cookie_for_testing(&mut self) {
        if !self.is_open {
            self.open();
        }

        if let Ok(mut state) = self.shared_state.lock() {
            let ui_language = state.settings.ui_language;
            let provider = state.new_cookie_provider.trim().to_string();
            let value = state.new_cookie_value.clone();
            state.api_key_status_msg = None;

            if provider.is_empty() || value.is_empty() {
                state.cookie_status_msg = Some((
                    locale_text(ui_language, LocaleKey::SaveFailed)
                        .replace("{}", "missing provider or value"),
                    true,
                ));
                return;
            }

            state.cookies.set(&provider, &value);
            if let Err(error) = state.cookies.save() {
                state.cookie_status_msg = Some((
                    locale_text(ui_language, LocaleKey::SaveFailed)
                        .replace("{}", &error.to_string()),
                    true,
                ));
            } else {
                let provider_name = ProviderId::from_cli_name(&provider)
                    .map(|id| id.display_name().to_string())
                    .unwrap_or_else(|| provider.clone());
                state.cookie_status_msg = Some((
                    locale_text(ui_language, LocaleKey::CookieSavedForProvider)
                        .replace("{}", &provider_name),
                    false,
                ));
                state.new_cookie_provider.clear();
                state.new_cookie_value.clear();
            }
        }
    }

    /// Show the preferences window as a separate native OS window
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.is_open {
            return;
        }

        let shared_state = Arc::clone(&self.shared_state);
        let settings_viewport_id = egui::ViewportId::from_hash_of("settings_viewport");
        let work_area = work_area_rect(ctx);
        let main_outer_rect = ctx.input(|i| i.viewport().outer_rect);

        let preferred_size = egui::vec2(720.0, 660.0);
        let default_min_size = egui::vec2(520.0, 420.0);
        let margin = 12.0;
        let settings_size = if let Some(area) = work_area {
            let max_w = (area.width() - margin * 2.0).max(360.0);
            let max_h = (area.height() - margin * 2.0).max(360.0);
            egui::vec2(preferred_size.x.min(max_w), preferred_size.y.min(max_h))
        } else {
            preferred_size
        };
        let settings_min_size = egui::vec2(
            default_min_size.x.min(settings_size.x),
            default_min_size.y.min(settings_size.y),
        );
        let settings_position = if self.needs_viewport_placement {
            match (main_outer_rect, work_area) {
                (Some(main_rect), Some(area)) => Some(settings_position_near_main_window(
                    main_rect,
                    settings_size,
                    area,
                )),
                _ => None,
            }
        } else {
            None
        };

        let mut builder = egui::ViewportBuilder::default()
            .with_title(locale_text(
                self.settings.ui_language,
                LocaleKey::MenuSettings,
            ))
            .with_inner_size([settings_size.x, settings_size.y])
            .with_min_inner_size([settings_min_size.x, settings_min_size.y])
            .with_clamp_size_to_monitor_size(true)
            .with_resizable(true);
        if let Some(position) = settings_position {
            builder = builder.with_position(position);
        }

        let mut pending_viewport_screenshot_request: Option<PathBuf> = None;
        ctx.show_viewport_immediate(settings_viewport_id, builder, |ctx, _class| {
            // Check if window was closed
            if ctx.input(|i| i.viewport().close_requested())
                && let Ok(mut state) = shared_state.lock()
            {
                state.is_open = false;
            }

            // Apply dark theme
            let mut style = (*ctx.style()).clone();
            style.visuals.window_fill = Theme::BG_SECONDARY;
            style.visuals.panel_fill = Theme::BG_SECONDARY;
            style.visuals.widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
            style.visuals.widgets.inactive.bg_fill = Theme::CARD_BG;
            style.visuals.widgets.hovered.bg_fill = Theme::CARD_BG_HOVER;
            style.visuals.widgets.active.bg_fill = Theme::ACCENT_PRIMARY;
            ctx.set_style(style);

            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(Theme::BG_SECONDARY)
                        .inner_margin(Spacing::MD),
                )
                .show(ctx, |ui| {
                    render_settings_ui(ui, &shared_state);
                });

            #[cfg(debug_assertions)]
            {
                let mut pending_path = None;
                if let Ok(mut state) = shared_state.lock() {
                    if state.pending_screenshot_delay_frames > 0 {
                        append_preferences_screenshot_debug_log(&format!(
                            "delay path={} frames_left={}",
                            state
                                .pending_screenshot_path
                                .as_ref()
                                .map(|path| path.display().to_string())
                                .unwrap_or_else(|| "<none>".to_string()),
                            state.pending_screenshot_delay_frames
                        ));
                        state.pending_screenshot_delay_frames -= 1;
                        ctx.request_repaint();
                    } else if let Some(path) = state.pending_screenshot_path.clone() {
                        if state.pending_screenshot_attempts < 8 {
                            state.pending_screenshot_attempts += 1;
                            append_preferences_screenshot_debug_log(&format!(
                                "attempt path={} attempt={}",
                                path.display(),
                                state.pending_screenshot_attempts
                            ));
                            pending_path = Some(path);
                            ctx.request_repaint();
                        } else {
                            append_preferences_screenshot_debug_log(&format!(
                                "attempt_limit path={} attempts={}",
                                path.display(),
                                state.pending_screenshot_attempts
                            ));
                        }
                    }
                }

                let screenshot_events: Vec<_> = ctx.input(|i| {
                    i.events
                        .iter()
                        .filter_map(|event| match event {
                            egui::Event::Screenshot {
                                user_data, image, ..
                            } => Some((user_data.clone(), image.clone())),
                            _ => None,
                        })
                        .collect()
                });

                for (user_data, image) in screenshot_events {
                    if let Some(path) = user_data
                        .data
                        .as_ref()
                        .and_then(|data| data.downcast_ref::<PathBuf>())
                    {
                        append_preferences_screenshot_debug_log(&format!(
                            "event_received path={} size={}x{}",
                            path.display(),
                            image.size[0],
                            image.size[1]
                        ));
                        if let Err(error) = save_color_image_to_png(path, &image) {
                            append_preferences_screenshot_debug_log(&format!(
                                "save_failed path={} error={}",
                                path.display(),
                                error
                            ));
                            tracing::warn!(
                                "Failed to save preferences test screenshot to {}: {}",
                                path.display(),
                                error
                            );
                        } else {
                            append_preferences_screenshot_debug_log(&format!(
                                "save_ok path={}",
                                path.display()
                            ));
                            tracing::info!(
                                "Saved preferences test screenshot to {}",
                                path.display()
                            );
                            if let Ok(mut state) = shared_state.lock()
                                && state.pending_screenshot_path.as_ref() == Some(path)
                            {
                                state.pending_screenshot_path = None;
                                state.pending_screenshot_delay_frames = 0;
                                state.pending_screenshot_attempts = 0;
                            }
                        }
                    }
                }

                if let Some(path) = pending_path {
                    append_preferences_screenshot_debug_log(&format!(
                        "queue_root_viewport_screenshot path={}",
                        path.display()
                    ));
                    pending_viewport_screenshot_request = Some(path);
                    ctx.request_repaint();
                }
            }
        });

        if let Some(path) = pending_viewport_screenshot_request {
            append_preferences_screenshot_debug_log(&format!(
                "send_root_viewport_screenshot path={} viewport={:?}",
                path.display(),
                settings_viewport_id
            ));
            ctx.send_viewport_cmd_to(
                settings_viewport_id,
                egui::ViewportCommand::Screenshot(egui::UserData::new(path)),
            );
            ctx.request_repaint();
        }

        ctx.send_viewport_cmd_to(settings_viewport_id, egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd_to(
            settings_viewport_id,
            egui::ViewportCommand::Minimized(false),
        );

        if settings_position.is_some() {
            ctx.send_viewport_cmd_to(settings_viewport_id, egui::ViewportCommand::Focus);
            self.needs_viewport_placement = false;
        }

        // Sync state back from shared state
        if let Ok(state) = self.shared_state.lock() {
            self.is_open = state.is_open;
            self.settings = state.settings.clone();
            self.settings_changed = state.settings_changed;
            self.active_tab = state.active_tab;
        }
    }

    fn show_general_tab(&mut self, ui: &mut egui::Ui) {
        let lang = self.settings.ui_language;

        // LANGUAGE section
        section_header(ui, locale_text(lang, LocaleKey::InterfaceLanguage));

        settings_card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(locale_text(lang, LocaleKey::InterfaceLanguage))
                        .size(FontSize::MD)
                        .color(Theme::TEXT_PRIMARY),
                );
                ui.add_space(Spacing::MD);

                // Language selector ComboBox
                let current_language = self.settings.ui_language;
                let current_label = current_language.display_name();

                egui::ComboBox::from_id_salt("language_selector")
                    .selected_text(current_label)
                    .show_ui(ui, |ui| {
                        for lang in Language::all() {
                            if ui
                                .selectable_label(
                                    self.settings.ui_language == *lang,
                                    lang.display_name(),
                                )
                                .clicked()
                            {
                                self.settings.ui_language = *lang;
                                self.settings_changed = true;
                            }
                        }
                    });
            });
        });

        ui.add_space(Spacing::LG);

        // STARTUP section
        section_header(ui, locale_text(lang, LocaleKey::StartupSettings));

        settings_card(ui, |ui| {
            let mut start_at_login = self.settings.start_at_login;
            if setting_toggle(
                ui,
                locale_text(lang, LocaleKey::StartAtLogin),
                locale_text(lang, LocaleKey::StartAtLoginHelper),
                &mut start_at_login,
            ) {
                if let Err(e) = self.settings.set_start_at_login(start_at_login) {
                    tracing::error!("Failed to set start at login: {}", e);
                } else {
                    self.settings_changed = true;
                }
            }

            setting_divider(ui);

            let mut start_minimized = self.settings.start_minimized;
            if setting_toggle(
                ui,
                locale_text(lang, LocaleKey::StartMinimized),
                locale_text(lang, LocaleKey::StartMinimizedHelper),
                &mut start_minimized,
            ) {
                self.settings.start_minimized = start_minimized;
                self.settings_changed = true;
            }
        });

        ui.add_space(Spacing::LG);

        // NOTIFICATIONS section
        section_header(ui, locale_text(lang, LocaleKey::ShowNotifications));

        settings_card(ui, |ui| {
            let mut show_notifications = self.settings.show_notifications;
            if setting_toggle(
                ui,
                locale_text(lang, LocaleKey::ShowNotifications),
                locale_text(lang, LocaleKey::ShowNotificationsHelper),
                &mut show_notifications,
            ) {
                self.settings.show_notifications = show_notifications;
                self.settings_changed = true;
            }

            setting_divider(ui);

            // Sound effects toggle
            let mut sound_enabled = self.settings.sound_enabled;
            if setting_toggle(
                ui,
                locale_text(lang, LocaleKey::SoundEnabled),
                locale_text(lang, LocaleKey::SoundEnabledHelper),
                &mut sound_enabled,
            ) {
                self.settings.sound_enabled = sound_enabled;
                self.settings_changed = true;
            }

            // Sound volume slider (only show if sound is enabled)
            if sound_enabled {
                setting_divider(ui);

                ui.vertical(|ui| {
                    let mut volume = self.settings.sound_volume as i32;

                    // Title row with volume badge on right
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(locale_text(lang, LocaleKey::SoundVolume))
                                .size(FontSize::MD)
                                .color(Theme::TEXT_PRIMARY),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            egui::Frame::none()
                                .fill(Theme::ACCENT_PRIMARY.gamma_multiply(0.15))
                                .rounding(Rounding::same(10.0))
                                .inner_margin(egui::Margin::symmetric(10.0, 3.0))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(format!("{}%", volume))
                                            .size(FontSize::SM)
                                            .color(Theme::ACCENT_PRIMARY)
                                            .strong(),
                                    );
                                });
                        });
                    });

                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(locale_text(lang, LocaleKey::SoundVolume))
                            .size(FontSize::SM)
                            .color(Theme::TEXT_MUTED),
                    );
                    ui.add_space(6.0);

                    ui.style_mut().visuals.widgets.inactive.bg_fill = Theme::BG_TERTIARY;
                    ui.style_mut().visuals.widgets.hovered.bg_fill = Theme::CARD_BG_HOVER;
                    ui.style_mut().visuals.widgets.active.bg_fill = Theme::ACCENT_PRIMARY;

                    let slider = ui.add(
                        egui::Slider::new(&mut volume, 0..=100)
                            .show_value(false)
                            .trailing_fill(true),
                    );

                    if slider.changed() {
                        self.settings.sound_volume = volume as u8;
                        self.settings_changed = true;
                    }
                });
            }

            setting_divider(ui);

            // High warning threshold
            ui.vertical(|ui| {
                let mut threshold = self.settings.high_usage_threshold as i32;

                // Title row with percentage badge on right
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(locale_text(lang, LocaleKey::HighUsageAlert))
                            .size(FontSize::MD)
                            .color(Theme::TEXT_PRIMARY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Percentage pill badge
                        egui::Frame::none()
                            .fill(Theme::ACCENT_PRIMARY.gamma_multiply(0.15))
                            .rounding(Rounding::same(10.0))
                            .inner_margin(egui::Margin::symmetric(10.0, 3.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!("{}%", threshold))
                                        .size(FontSize::SM)
                                        .color(Theme::ACCENT_PRIMARY)
                                        .strong(),
                                );
                            });
                    });
                });

                ui.add_space(2.0);
                ui.label(
                    RichText::new(locale_text(lang, LocaleKey::HighUsageThresholdHelper))
                        .size(FontSize::SM)
                        .color(Theme::TEXT_MUTED),
                );
                ui.add_space(6.0);

                // Full-width slider
                ui.style_mut().visuals.widgets.inactive.bg_fill = Theme::BG_TERTIARY;
                ui.style_mut().visuals.widgets.hovered.bg_fill = Theme::CARD_BG_HOVER;
                ui.style_mut().visuals.widgets.active.bg_fill = Theme::ACCENT_PRIMARY;

                let slider = ui.add(
                    egui::Slider::new(&mut threshold, 50..=95)
                        .show_value(false)
                        .trailing_fill(true),
                );

                if slider.changed() && threshold as f64 != self.settings.high_usage_threshold {
                    self.settings.high_usage_threshold = threshold as f64;
                    self.settings_changed = true;
                }
            });

            setting_divider(ui);

            // Critical alert threshold
            ui.vertical(|ui| {
                let mut threshold = self.settings.critical_usage_threshold as i32;
                let badge_color = Color32::from_rgb(239, 68, 68); // Red

                // Title row with percentage badge on right
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(locale_text(lang, LocaleKey::CriticalUsageAlert))
                            .size(FontSize::MD)
                            .color(Theme::TEXT_PRIMARY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Percentage pill badge - red tint for critical
                        egui::Frame::none()
                            .fill(badge_color.gamma_multiply(0.15))
                            .rounding(Rounding::same(10.0))
                            .inner_margin(egui::Margin::symmetric(10.0, 3.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!("{}%", threshold))
                                        .size(FontSize::SM)
                                        .color(badge_color)
                                        .strong(),
                                );
                            });
                    });
                });

                ui.add_space(2.0);
                ui.label(
                    RichText::new(locale_text(lang, LocaleKey::CriticalUsageThresholdHelper))
                        .size(FontSize::SM)
                        .color(Theme::TEXT_MUTED),
                );
                ui.add_space(6.0);

                // Full-width slider
                ui.style_mut().visuals.widgets.inactive.bg_fill = Theme::BG_TERTIARY;
                ui.style_mut().visuals.widgets.hovered.bg_fill = Theme::CARD_BG_HOVER;
                ui.style_mut().visuals.widgets.active.bg_fill = badge_color;

                let slider = ui.add(
                    egui::Slider::new(&mut threshold, 80..=100)
                        .show_value(false)
                        .trailing_fill(true),
                );

                if slider.changed() && threshold as f64 != self.settings.critical_usage_threshold {
                    self.settings.critical_usage_threshold = threshold as f64;
                    self.settings_changed = true;
                }
            });
        });
    }

    fn show_providers_tab(&mut self, ui: &mut egui::Ui, available_height: f32) {
        let providers = ProviderId::all();

        // Ensure a provider is selected
        if self.selected_provider.is_none() && !providers.is_empty() {
            self.selected_provider = Some(providers[0]);
        }

        // Calculate dimensions - responsive sidebar width
        let total_width = ui.available_width();
        let sidebar_width = (total_width * 0.45).clamp(140.0, 180.0); // 45% of width, 140-180px range
        let gap = Spacing::SM;
        let detail_width = (total_width - sidebar_width - gap).max(150.0);
        let panel_height = available_height;

        // Side-by-side layout with explicit sizes
        ui.allocate_ui_with_layout(
            Vec2::new(total_width, panel_height),
            egui::Layout::left_to_right(egui::Align::TOP),
            |ui| {
                // LEFT SIDEBAR (fixed 200px)
                ui.allocate_ui_with_layout(
                    Vec2::new(sidebar_width, panel_height),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        self.draw_provider_sidebar(ui, providers, panel_height);
                    },
                );

                ui.add_space(gap);

                // RIGHT DETAIL PANEL (fills remaining)
                ui.allocate_ui_with_layout(
                    Vec2::new(detail_width, panel_height),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        self.draw_provider_detail(ui, panel_height);
                    },
                );
            },
        );
    }

    fn draw_provider_sidebar(
        &mut self,
        ui: &mut egui::Ui,
        providers: &[ProviderId],
        available_height: f32,
    ) {
        egui::Frame::none()
            .inner_margin(Spacing::XS)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("provider_sidebar")
                    .max_height(available_height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.style_mut().spacing.item_spacing.y = 4.0;

                        for provider_id in providers {
                            let provider_name = provider_id.cli_name();
                            let is_selected = self.selected_provider.as_ref() == Some(provider_id);
                            let is_enabled =
                                self.settings.enabled_providers.contains(provider_name);

                            // Add padding around each row
                            let frame_response = egui::Frame::none()
                                .inner_margin(egui::Margin::symmetric(8.0, 8.0))
                                .rounding(Rounding::same(Radius::MD))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        // Make the row fill available width
                                        ui.set_min_width(ui.available_width());
                                        ui.set_min_height(32.0);

                                        // Icon
                                        let icon_size = 20.0;
                                        if let Some(texture) = self.icon_cache.get_icon(
                                            ui.ctx(),
                                            provider_name,
                                            icon_size as u32,
                                        ) {
                                            ui.add(
                                                egui::Image::new(texture)
                                                    .fit_to_exact_size(Vec2::splat(icon_size)),
                                            );
                                        } else {
                                            ui.label(
                                                RichText::new(provider_icon(provider_name))
                                                    .size(FontSize::MD)
                                                    .color(provider_color(provider_name)),
                                            );
                                        }

                                        ui.add_space(8.0);

                                        // Provider name as plain label (no hover effect)
                                        let text_color = if is_selected {
                                            Theme::TEXT_PRIMARY
                                        } else if is_enabled {
                                            Theme::TEXT_SECONDARY
                                        } else {
                                            Theme::TEXT_MUTED
                                        };

                                        ui.label(
                                            RichText::new(provider_id.display_name())
                                                .size(FontSize::SM)
                                                .color(text_color),
                                        );

                                        // Spacer to push checkbox to right
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                // Checkbox
                                                let mut enabled = is_enabled;
                                                if ui.checkbox(&mut enabled, "").changed() {
                                                    if enabled {
                                                        self.settings
                                                            .enabled_providers
                                                            .insert(provider_name.to_string());
                                                    } else {
                                                        self.settings
                                                            .enabled_providers
                                                            .remove(provider_name);
                                                    }
                                                    self.settings_changed = true;
                                                }
                                            },
                                        );
                                    });
                                });

                            // Check hover and click on the frame
                            let frame_rect = frame_response.response.rect;
                            let row_response = ui.interact(
                                frame_rect,
                                ui.make_persistent_id(format!("row_{}", provider_name)),
                                egui::Sense::click(),
                            );
                            let is_hovered = row_response.hovered();

                            if row_response.clicked() {
                                self.selected_provider = Some(*provider_id);
                            }

                            // Draw the selection/hover ring on top
                            if is_selected || is_hovered {
                                let fill = Theme::ACCENT_PRIMARY.gamma_multiply(0.15);
                                let stroke = if is_selected {
                                    Stroke::new(1.5, Theme::ACCENT_PRIMARY)
                                } else {
                                    Stroke::new(1.0, Theme::ACCENT_PRIMARY.gamma_multiply(0.6))
                                };
                                ui.painter().rect(frame_rect, Radius::MD, fill, stroke);
                            }
                        }
                    });
            });
    }

    fn draw_provider_detail(&mut self, ui: &mut egui::Ui, available_height: f32) {
        if let Some(selected_id) = self.selected_provider {
            egui::Frame::none()
                .fill(Theme::BG_SECONDARY)
                .rounding(Rounding::same(Radius::LG))
                .inner_margin(Spacing::MD)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("provider_detail")
                        .max_height(available_height - Spacing::MD * 2.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            self.draw_provider_detail_panel(ui, &selected_id);
                        });
                });
        } else {
            // Placeholder
            egui::Frame::none()
                .fill(Theme::BG_SECONDARY)
                .rounding(Rounding::same(Radius::LG))
                .inner_margin(Spacing::MD)
                .show(ui, |ui| {
                    ui.set_min_height(available_height);
                    ui.vertical_centered(|ui| {
                        ui.add_space(available_height / 3.0);
                        ui.label(
                            RichText::new("Select a provider")
                                .size(FontSize::MD)
                                .color(Theme::TEXT_MUTED),
                        );
                    });
                });
        }
    }

    fn draw_provider_detail_panel(&mut self, ui: &mut egui::Ui, provider_id: &ProviderId) {
        let lang = self.settings.ui_language;
        let provider_name = provider_id.cli_name();
        let display_name = provider_id.display_name();
        let is_enabled = self.settings.enabled_providers.contains(provider_name);
        let color = provider_color(provider_name);

        // ═══════════════════════════════════════════════════════════
        // HEADER - Icon, name, enable toggle
        // ═══════════════════════════════════════════════════════════
        ui.horizontal(|ui| {
            // Large brand icon with background
            egui::Frame::none()
                .fill(color.gamma_multiply(0.15))
                .rounding(Rounding::same(Radius::MD))
                .inner_margin(Spacing::SM)
                .show(ui, |ui| {
                    let icon_size = 32.0;
                    if let Some(texture) =
                        self.icon_cache
                            .get_icon(ui.ctx(), provider_name, icon_size as u32)
                    {
                        ui.add(egui::Image::new(texture).fit_to_exact_size(Vec2::splat(icon_size)));
                    } else {
                        ui.label(
                            RichText::new(provider_icon(provider_name))
                                .size(icon_size)
                                .color(color),
                        );
                    }
                });

            ui.add_space(Spacing::SM);

            ui.vertical(|ui| {
                ui.label(
                    RichText::new(display_name)
                        .size(FontSize::XL)
                        .color(Theme::TEXT_PRIMARY)
                        .strong(),
                );
                ui.horizontal(|ui| {
                    // Status indicator dot
                    let status_color = if is_enabled {
                        Theme::GREEN
                    } else {
                        Theme::TEXT_MUTED
                    };
                    ui.label(RichText::new("●").size(FontSize::XS).color(status_color));
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(if is_enabled { "Enabled" } else { "Disabled" })
                            .size(FontSize::SM)
                            .color(status_color),
                    );
                });
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Enable/disable toggle styled as a switch
                let mut enabled = is_enabled;
                if ui.checkbox(&mut enabled, "").changed() {
                    if enabled {
                        self.settings
                            .enabled_providers
                            .insert(provider_name.to_string());
                    } else {
                        self.settings.enabled_providers.remove(provider_name);
                    }
                    self.settings_changed = true;
                }
            });
        });

        ui.add_space(Spacing::LG);

        // ═══════════════════════════════════════════════════════════
        // INFO SECTION - Provider-specific information
        // ═══════════════════════════════════════════════════════════
        section_header(ui, locale_text(lang, LocaleKey::ProviderInfo));

        settings_card(ui, |ui| {
            // Authentication type
            let auth_type = match provider_name {
                "openai" | "gemini" | "openrouter" | "nanogpt" => "API 密钥",
                "claude" | "cursor" | "kimi" => "浏览器会话",
                "ollama" => "本地运行（无需认证）",
                "windsurf" => "浏览器会话",
                _ => "浏览器会话",
            };
            self.draw_info_row(ui, locale_text(lang, LocaleKey::AuthType), auth_type);
            setting_divider(ui);

            // Data source
            let data_source = match provider_name {
                "openai" => "OpenAI API Usage Dashboard",
                "gemini" => "Google AI Studio",
                "claude" => "Anthropic Web Console",
                "cursor" => "Cursor Settings API",
                "ollama" => "Local Ollama Server",
                "openrouter" => "OpenRouter Dashboard",
                "nanogpt" => "NanoGPT Subscription API",
                "windsurf" => "Windsurf API",
                "kimi" => "Kimi Web Console",
                _ => "Provider API",
            };
            self.draw_info_row(ui, locale_text(lang, LocaleKey::DataSource), data_source);
            setting_divider(ui);

            // Rate limit info
            let rate_info = match provider_name {
                "claude" => "每日消息上限",
                "cursor" => "每月请求上限",
                "openai" => "Token 用量与额度",
                "gemini" => "每分钟请求数",
                "nanogpt" => "订阅用量单位与上限",
                _ => "用量追踪",
            };
            self.draw_info_row(ui, locale_text(lang, LocaleKey::TrackingItem), rate_info);
        });

        ui.add_space(Spacing::LG);

        // ═══════════════════════════════════════════════════════════
        // USAGE SECTION - Link to main window
        // ═══════════════════════════════════════════════════════════
        section_header(ui, locale_text(lang, LocaleKey::ProviderUsage));

        settings_card(ui, |ui| {
            if is_enabled {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📊").size(FontSize::LG));
                    ui.add_space(Spacing::SM);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("主窗口实时用量数据")
                                .size(FontSize::MD)
                                .color(Theme::TEXT_PRIMARY),
                        );
                        ui.label(
                            RichText::new("点击托盘图标查看实时指标")
                                .size(FontSize::SM)
                                .color(Theme::TEXT_MUTED),
                        );
                    });
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("⏸")
                            .size(FontSize::LG)
                            .color(Theme::TEXT_MUTED),
                    );
                    ui.add_space(Spacing::SM);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("服务商已禁用")
                                .size(FontSize::MD)
                                .color(Theme::TEXT_MUTED),
                        );
                        ui.label(
                            RichText::new("启用后开始追踪用量")
                                .size(FontSize::SM)
                                .color(Theme::TEXT_DIM),
                        );
                    });
                });
            }
        });

        ui.add_space(Spacing::LG);

        // ═══════════════════════════════════════════════════════════
        // BROWSER COOKIE IMPORT - Only for cookie-based providers
        // ═══════════════════════════════════════════════════════════
        if provider_id.cookie_domain().is_some() {
            self.draw_browser_cookie_import(ui, provider_id);
            ui.add_space(Spacing::LG);
        }

        // ═══════════════════════════════════════════════════════════
        // QUICK ACTIONS
        // ═══════════════════════════════════════════════════════════
        section_header(ui, locale_text(lang, LocaleKey::QuickActions));

        settings_card(ui, |ui| {
            // Provider-specific quick actions
            match provider_name {
                "openai" => {
                    if text_button(ui, "→ 打开 OpenAI 仪表盘", Theme::ACCENT_PRIMARY) {
                        let _ = open::that("https://platform.openai.com/usage");
                    }
                }
                "claude" => {
                    if text_button(ui, "→ 打开 Claude 控制台", Theme::ACCENT_PRIMARY) {
                        let _ = open::that("https://console.anthropic.com/");
                    }
                }
                "gemini" => {
                    if text_button(ui, "→ 打开 Google AI Studio", Theme::ACCENT_PRIMARY) {
                        let _ = open::that("https://aistudio.google.com/");
                    }
                }
                "cursor" => {
                    if text_button(ui, "→ 打开 Cursor 设置", Theme::ACCENT_PRIMARY) {
                        let _ = open::that("https://www.cursor.com/settings");
                    }
                }
                "nanogpt" => {
                    if text_button(ui, "→ 打开 NanoGPT 仪表盘", Theme::ACCENT_PRIMARY) {
                        let _ = open::that("https://nano-gpt.com/usage");
                    }
                }
                "ollama" => {
                    ui.label(
                        RichText::new("Ollama 在本地运行，无仪表盘")
                            .size(FontSize::SM)
                            .color(Theme::TEXT_MUTED),
                    );
                }
                _ => {
                    ui.label(
                        RichText::new("暂无可用快捷操作")
                            .size(FontSize::SM)
                            .color(Theme::TEXT_MUTED),
                    );
                }
            }
        });
    }

    fn draw_info_row(&self, ui: &mut egui::Ui, label: &str, value: &str) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(label)
                    .size(FontSize::SM)
                    .color(Theme::TEXT_MUTED),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(value)
                        .size(FontSize::SM)
                        .color(Theme::TEXT_SECONDARY),
                );
            });
        });
    }

    fn draw_browser_cookie_import(&mut self, ui: &mut egui::Ui, provider_id: &ProviderId) {
        let lang = self.settings.ui_language;
        section_header(ui, locale_text(lang, LocaleKey::BrowserCookieImport));

        settings_card(ui, |ui| {
            let domain = provider_id.cookie_domain().unwrap_or("unknown");

            ui.label(
                RichText::new(format!("从浏览器导入 {} 的 Cookies", domain))
                    .size(FontSize::SM)
                    .color(Theme::TEXT_MUTED),
            );

            ui.add_space(Spacing::SM);

            // Detect available browsers
            let browsers = BrowserDetector::detect_all();

            if browsers.is_empty() {
                ui.label(
                    RichText::new("未检测到受支持的浏览器")
                        .size(FontSize::SM)
                        .color(Theme::YELLOW),
                );
            } else {
                // Browser selection dropdown
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Browser")
                            .size(FontSize::MD)
                            .color(Theme::TEXT_PRIMARY),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let selected_text = self
                            .selected_browser
                            .map(|b| b.display_name())
                            .unwrap_or("请选择浏览器...");

                        egui::ComboBox::from_id_salt("browser_select")
                            .selected_text(selected_text)
                            .show_ui(ui, |ui| {
                                for browser in &browsers {
                                    let browser_type = browser.browser_type;
                                    if ui
                                        .selectable_label(
                                            self.selected_browser == Some(browser_type),
                                            browser_type.display_name(),
                                        )
                                        .clicked()
                                    {
                                        self.selected_browser = Some(browser_type);
                                        self.browser_import_status = None;
                                    }
                                }
                            });
                    });
                });

                ui.add_space(Spacing::MD);

                // Import button
                let can_import = self.selected_browser.is_some();
                if ui
                    .add_enabled(
                        can_import,
                        egui::Button::new(RichText::new("导入 Cookies").size(FontSize::SM).color(
                            if can_import {
                                Color32::WHITE
                            } else {
                                Theme::TEXT_MUTED
                            },
                        ))
                        .fill(if can_import {
                            Theme::ACCENT_PRIMARY
                        } else {
                            Theme::BG_TERTIARY
                        })
                        .rounding(Rounding::same(Radius::MD))
                        .min_size(Vec2::new(120.0, 36.0)),
                    )
                    .clicked()
                {
                    // Attempt to import cookies from selected browser
                    if let Some(browser_type) = self.selected_browser {
                        // Find the detected browser matching the selected type
                        let browsers = BrowserDetector::detect_all();
                        if let Some(browser) =
                            browsers.iter().find(|b| b.browser_type == browser_type)
                        {
                            match get_cookie_header_from_browser(domain, browser) {
                                Ok(cookie_header) if !cookie_header.is_empty() => {
                                    // Save the cookie
                                    self.cookies.set(provider_id.cli_name(), &cookie_header);
                                    if let Err(e) = self.cookies.save() {
                                        self.browser_import_status =
                                            Some((format!("保存失败：{}", e), true));
                                    } else {
                                        self.browser_import_status = Some((
                                            format!(
                                                "已为 {} 导入 Cookies",
                                                provider_id.display_name()
                                            ),
                                            false,
                                        ));
                                    }
                                }
                                Ok(_) => {
                                    self.browser_import_status = Some((
                                        format!(
                                            "在 {} 的 {} 中未找到 Cookies。请先确认已登录。",
                                            browser_type.display_name(),
                                            domain
                                        ),
                                        true,
                                    ));
                                }
                                Err(e) => {
                                    self.browser_import_status =
                                        Some((format!("导入失败：{}", e), true));
                                }
                            }
                        } else {
                            self.browser_import_status = Some((
                                format!("未找到浏览器 {}", browser_type.display_name()),
                                true,
                            ));
                        }
                    }
                }

                // Status message
                if let Some((msg, is_error)) = &self.browser_import_status {
                    ui.add_space(Spacing::SM);
                    let color = if *is_error { Theme::RED } else { Theme::GREEN };
                    ui.label(RichText::new(msg).size(FontSize::SM).color(color));
                }
            }
        });
    }

    fn show_display_tab(&mut self, ui: &mut egui::Ui) {
        let lang = self.settings.ui_language;
        section_header(ui, locale_text(lang, LocaleKey::UsageDisplay));

        settings_card(ui, |ui| {
            let mut show_as_used = self.settings.show_as_used;
            if setting_toggle(
                ui,
                "按已使用显示用量",
                "显示为已使用百分比（而非剩余）",
                &mut show_as_used,
            ) {
                self.settings.show_as_used = show_as_used;
                self.settings_changed = true;
            }

            setting_divider(ui);

            let mut reset_time_relative = self.settings.reset_time_relative;
            if setting_toggle(
                ui,
                "相对重置时间",
                "显示“2h 30m”而不是“3:00 PM”",
                &mut reset_time_relative,
            ) {
                self.settings.reset_time_relative = reset_time_relative;
                self.settings_changed = true;
            }

            setting_divider(ui);

            let mut show_credits_extra = self.settings.show_credits_extra_usage;
            if setting_toggle(
                ui,
                "显示额度与扩展用量",
                "显示额度余额和额外用量信息",
                &mut show_credits_extra,
            ) {
                self.settings.show_credits_extra_usage = show_credits_extra;
                self.settings_changed = true;
            }
        });

        ui.add_space(Spacing::SM);

        section_header(ui, locale_text(lang, LocaleKey::TrayIcon));

        settings_card(ui, |ui| {
            let mut merge_icons = self.settings.merge_tray_icons;
            if setting_toggle(
                ui,
                "合并托盘图标",
                "将所有服务商显示在一个托盘图标中",
                &mut merge_icons,
            ) {
                set_merge_tray_icons(&mut self.settings, merge_icons);
                self.settings_changed = true;
            }

            setting_divider(ui);

            let mut per_provider = self.settings.tray_icon_mode == TrayIconMode::PerProvider;
            if setting_toggle(
                ui,
                "按服务商分图标",
                "每个启用的服务商显示独立托盘图标",
                &mut per_provider,
            ) {
                set_per_provider_tray_icons(&mut self.settings, per_provider);
                self.settings_changed = true;
            }
        });
    }

    fn show_api_keys_tab(&mut self, ui: &mut egui::Ui) {
        section_header(ui, "API Keys");

        ui.label(
            RichText::new("为需要认证的服务商配置访问令牌。")
                .size(FontSize::SM)
                .color(Theme::TEXT_MUTED),
        );

        ui.add_space(Spacing::MD);

        // Status message
        if let Some((msg, is_error)) = &self.api_key_status_msg {
            status_message(ui, msg, *is_error);
            ui.add_space(Spacing::SM);
        }

        // Provider cards - one per provider
        let api_key_providers = get_api_key_providers();

        for provider_info in &api_key_providers {
            let provider_id = provider_info.id.cli_name();
            let has_key = self.api_keys.has_key(provider_id);
            let is_enabled = self.settings.enabled_providers.contains(provider_id);
            let icon = provider_icon(provider_id);
            let color = provider_color(provider_id);

            // Card with left accent bar
            let accent_color = if has_key {
                Theme::GREEN
            } else if is_enabled {
                Theme::ORANGE
            } else {
                Theme::BG_TERTIARY
            };

            egui::Frame::none()
                .fill(Theme::BG_SECONDARY)
                .rounding(Rounding::same(Radius::MD))
                .inner_margin(egui::Margin::same(0.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Left accent bar - reduced height for compact layout
                        let bar_rect = Rect::from_min_size(ui.cursor().min, Vec2::new(3.0, 48.0));
                        ui.painter().rect_filled(
                            bar_rect,
                            Rounding {
                                nw: Radius::MD,
                                sw: Radius::MD,
                                ne: 0.0,
                                se: 0.0,
                            },
                            accent_color,
                        );
                        ui.add_space(3.0);

                        // Content - reduced padding for compact layout
                        ui.vertical(|ui| {
                            ui.add_space(Spacing::XS);

                            // Row 1: Icon, Name, Status badge, and Add Key button (right-aligned)
                            ui.horizontal(|ui| {
                                ui.add_space(Spacing::XS);
                                ui.label(RichText::new(icon).size(FontSize::LG).color(color));
                                ui.add_space(Spacing::XS);
                                ui.label(
                                    RichText::new(provider_info.name)
                                        .size(FontSize::MD)
                                        .color(Theme::TEXT_PRIMARY)
                                        .strong(),
                                );

                                ui.add_space(Spacing::XS);

                                if has_key {
                                    badge(ui, "✓ Set", Theme::GREEN);
                                } else if is_enabled {
                                    // Smaller pill-shaped badge with solid orange background
                                    egui::Frame::none()
                                        .fill(Theme::ORANGE)
                                        .rounding(Rounding::same(Radius::PILL))
                                        .inner_margin(egui::Margin::symmetric(Spacing::XS, 2.0))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new("需要密钥")
                                                    .size(FontSize::XS)
                                                    .color(Color32::BLACK),
                                            );
                                        });
                                }

                                // Right-aligned: Add Key button for providers without keys
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.add_space(Spacing::XS);
                                        if !has_key && primary_button(ui, "+ 添加密钥") {
                                            self.new_api_key_provider = provider_id.to_string();
                                            self.show_api_key_input = true;
                                            self.new_api_key_value.clear();
                                        }
                                    },
                                );
                            });

                            // Row 2: Single line with env var, masked key, and actions
                            ui.horizontal(|ui| {
                                ui.add_space(Spacing::XS);

                                // Env var info
                                if let Some(env_var) = provider_info.api_key_env_var {
                                    ui.label(
                                        RichText::new(format!("环境变量：{}", env_var))
                                            .size(FontSize::XS)
                                            .color(Theme::TEXT_MUTED)
                                            .monospace(),
                                    );
                                }

                                if has_key {
                                    ui.add_space(Spacing::SM);
                                    // Show masked key inline
                                    if let Some(key_info) = self
                                        .api_keys
                                        .get_all_for_display()
                                        .iter()
                                        .find(|k| k.provider_id == provider_id)
                                    {
                                        ui.label(
                                            RichText::new(&key_info.masked_key)
                                                .size(FontSize::XS)
                                                .color(Theme::TEXT_MUTED)
                                                .monospace(),
                                        );
                                    }

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.add_space(Spacing::XS);
                                            if small_button(ui, "Remove", Theme::RED) {
                                                self.api_keys.remove(provider_id);
                                                let _ = self.api_keys.save();
                                                self.api_key_status_msg = Some((
                                                    format!(
                                                        "已移除 {} 的 API key",
                                                        provider_info.name
                                                    ),
                                                    false,
                                                ));
                                            }
                                        },
                                    );
                                } else {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.add_space(Spacing::XS);
                                            match provider_info.dashboard_url {
                                                Some(url)
                                                    if text_button(
                                                        ui,
                                                        "Get key →",
                                                        Theme::ACCENT_PRIMARY,
                                                    ) =>
                                                {
                                                    let _ = open::that(url);
                                                }
                                                _ => {}
                                            }
                                        },
                                    );
                                }
                            });

                            ui.add_space(Spacing::XS);
                        });
                    });
                });

            ui.add_space(Spacing::XS);
        }

        // API Key input modal
        if self.show_api_key_input {
            ui.add_space(Spacing::MD);

            let provider_name = ProviderId::from_cli_name(&self.new_api_key_provider)
                .map(|id| id.display_name())
                .unwrap_or(&self.new_api_key_provider);

            egui::Frame::none()
                .fill(Theme::BG_TERTIARY)
                .stroke(Stroke::new(1.0, Theme::ACCENT_PRIMARY.gamma_multiply(0.4)))
                .rounding(Rounding::same(Radius::LG))
                .inner_margin(Spacing::LG)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(format!("为 {} 输入 API Key", provider_name))
                            .size(FontSize::MD)
                            .color(Theme::TEXT_PRIMARY)
                            .strong(),
                    );

                    ui.add_space(Spacing::SM);

                    let text_edit = egui::TextEdit::singleline(&mut self.new_api_key_value)
                        .password(true)
                        .desired_width(ui.available_width())
                        .hint_text("在这里粘贴 API key...");
                    ui.add(text_edit);

                    ui.add_space(Spacing::MD);

                    ui.horizontal(|ui| {
                        let can_save = !self.new_api_key_value.trim().is_empty();

                        if ui
                            .add_enabled(
                                can_save,
                                egui::Button::new(
                                    RichText::new("保存")
                                        .size(FontSize::SM)
                                        .color(Color32::WHITE),
                                )
                                .fill(if can_save {
                                    Theme::GREEN
                                } else {
                                    Theme::BG_TERTIARY
                                })
                                .rounding(Rounding::same(Radius::SM))
                                .min_size(Vec2::new(80.0, 32.0)),
                            )
                            .clicked()
                        {
                            self.api_keys.set(
                                &self.new_api_key_provider,
                                self.new_api_key_value.trim(),
                                None,
                            );
                            if let Err(e) = self.api_keys.save() {
                                self.api_key_status_msg = Some((format!("保存失败：{}", e), true));
                            } else {
                                self.api_key_status_msg =
                                    Some((format!("已保存 {} 的 API key", provider_name), false));
                                self.show_api_key_input = false;
                                self.new_api_key_value.clear();
                            }
                        }

                        ui.add_space(Spacing::XS);

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("取消")
                                        .size(FontSize::SM)
                                        .color(Theme::TEXT_MUTED),
                                )
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::new(1.0, Theme::BORDER_SUBTLE))
                                .rounding(Rounding::same(Radius::SM)),
                            )
                            .clicked()
                        {
                            self.show_api_key_input = false;
                            self.new_api_key_value.clear();
                        }
                    });
                });
        }
    }

    fn show_cookies_tab(&mut self, ui: &mut egui::Ui) {
        section_header(ui, "Browser Cookies");

        ui.label(
            RichText::new("Cookies 会自动从 Chrome、Edge、Brave 和 Firefox 中提取。")
                .size(FontSize::SM)
                .color(Theme::TEXT_MUTED),
        );

        ui.add_space(Spacing::LG);

        // Status message
        if let Some((msg, is_error)) = &self.cookie_status_msg {
            status_message(ui, msg, *is_error);
            ui.add_space(Spacing::MD);
        }

        // Saved cookies
        let saved_cookies = self.cookies.get_all_for_display();

        if !saved_cookies.is_empty() {
            section_header(ui, "Saved Cookies");

            settings_card(ui, |ui| {
                let mut to_remove: Option<String> = None;
                let len = saved_cookies.len();

                for (i, cookie_info) in saved_cookies.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&cookie_info.provider)
                                .size(FontSize::MD)
                                .color(Theme::TEXT_PRIMARY),
                        );
                        ui.label(
                            RichText::new(format!("· {}", &cookie_info.saved_at))
                                .size(FontSize::SM)
                                .color(Theme::TEXT_MUTED),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if small_button(ui, "Remove", Theme::RED) {
                                to_remove = Some(cookie_info.provider_id.clone());
                            }
                        });
                    });

                    if i < len - 1 {
                        setting_divider(ui);
                    }
                }

                if let Some(provider_id) = to_remove {
                    self.cookies.remove(&provider_id);
                    let _ = self.cookies.save();
                    self.cookie_status_msg =
                        Some((format!("已移除 {} 的 Cookie", provider_id), false));
                }
            });

            ui.add_space(Spacing::XL);
        }

        // Add manual cookie
        section_header(ui, "Add Manual Cookie");

        settings_card(ui, |ui| {
            // Provider selection row
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("服务商")
                        .size(FontSize::MD)
                        .color(Theme::TEXT_PRIMARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("cookie_provider")
                        .selected_text(if self.new_cookie_provider.is_empty() {
                            "请选择..."
                        } else {
                            &self.new_cookie_provider
                        })
                        .show_ui(ui, |ui| {
                            let web_providers = ["claude", "cursor", "kimi"];
                            for provider_name in web_providers {
                                match ProviderId::from_cli_name(provider_name) {
                                    Some(id)
                                        if ui
                                            .selectable_label(
                                                self.new_cookie_provider == provider_name,
                                                id.display_name(),
                                            )
                                            .clicked() =>
                                    {
                                        self.new_cookie_provider = provider_name.to_string();
                                    }
                                    _ => {}
                                }
                            }
                        });
                });
            });

            ui.add_space(Spacing::MD);
            setting_divider(ui);
            ui.add_space(Spacing::SM);

            // Cookie header label
            ui.label(
                RichText::new("Cookie 头")
                    .size(FontSize::MD)
                    .color(Theme::TEXT_PRIMARY),
            );
            ui.add_space(Spacing::SM);

            // Styled text input with visible border and rounded corners
            egui::Frame::none()
                .fill(Theme::INPUT_BG)
                .stroke(Stroke::new(1.0, Theme::BORDER_SUBTLE))
                .rounding(Rounding::same(Radius::SM))
                .inner_margin(Spacing::SM)
                .show(ui, |ui| {
                    let text_edit = egui::TextEdit::multiline(&mut self.new_cookie_value)
                        .desired_width(ui.available_width())
                        .desired_rows(4)
                        .frame(false)
                        .hint_text("粘贴浏览器开发者工具中的 Cookie 头");
                    ui.add(text_edit);
                });

            ui.add_space(Spacing::LG);

            // Save button - filled primary style with proper sizing
            let can_save =
                !self.new_cookie_provider.is_empty() && !self.new_cookie_value.is_empty();

            if ui
                .add_enabled(
                    can_save,
                    egui::Button::new(RichText::new("保存 Cookie").size(FontSize::SM).color(
                        if can_save {
                            Color32::WHITE
                        } else {
                            Theme::TEXT_MUTED
                        },
                    ))
                    .fill(if can_save {
                        Theme::ACCENT_PRIMARY
                    } else {
                        Theme::BG_TERTIARY
                    })
                    .stroke(if can_save {
                        Stroke::NONE
                    } else {
                        Stroke::new(1.0, Theme::BORDER_SUBTLE)
                    })
                    .rounding(Rounding::same(Radius::MD))
                    .min_size(Vec2::new(120.0, 36.0)),
                )
                .clicked()
            {
                self.cookies
                    .set(&self.new_cookie_provider, &self.new_cookie_value);
                if let Err(e) = self.cookies.save() {
                    self.cookie_status_msg = Some((format!("保存失败：{}", e), true));
                } else {
                    let provider_name = ProviderId::from_cli_name(&self.new_cookie_provider)
                        .map(|id| id.display_name().to_string())
                        .unwrap_or_else(|| self.new_cookie_provider.clone());
                    self.cookie_status_msg =
                        Some((format!("已保存 {} 的 Cookie", provider_name), false));
                    self.new_cookie_provider.clear();
                    self.new_cookie_value.clear();
                }
            }
        });
    }

    fn show_advanced_tab(&mut self, ui: &mut egui::Ui) {
        section_header(ui, "Refresh");

        settings_card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("自动刷新间隔")
                            .size(FontSize::MD)
                            .color(Theme::TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new("获取用量数据的频率")
                            .size(FontSize::SM)
                            .color(Theme::TEXT_MUTED),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let intervals = [
                        (0, "手动"),
                        (60, "1 min"),
                        (120, "2 min"),
                        (300, "5 min"),
                        (600, "10 min"),
                        (900, "15 min"),
                    ];

                    egui::Frame::none()
                        .fill(Theme::BG_TERTIARY)
                        .stroke(Stroke::new(1.0, Theme::BORDER_SUBTLE))
                        .rounding(Rounding::same(Radius::SM))
                        .inner_margin(egui::Margin::symmetric(Spacing::XS, 2.0))
                        .show(ui, |ui| {
                            egui::ComboBox::from_id_salt("refresh_interval")
                                .selected_text(
                                    intervals
                                        .iter()
                                        .find(|(secs, _)| {
                                            *secs == self.settings.refresh_interval_secs
                                        })
                                        .map(|(_, label)| *label)
                                        .unwrap_or("5 min"),
                                )
                                .show_ui(ui, |ui| {
                                    for (secs, label) in intervals {
                                        if ui
                                            .selectable_value(
                                                &mut self.settings.refresh_interval_secs,
                                                secs,
                                                label,
                                            )
                                            .changed()
                                        {
                                            self.settings_changed = true;
                                        }
                                    }
                                });
                        });
                });
            });
        });

        ui.add_space(Spacing::SM);

        section_header(ui, "Animations");

        settings_card(ui, |ui| {
            let mut enable_animations = self.settings.enable_animations;
            if setting_toggle(
                ui,
                "Enable animations",
                "Animate charts and UI transitions",
                &mut enable_animations,
            ) {
                self.settings.enable_animations = enable_animations;
                self.settings_changed = true;
            }
        });

        ui.add_space(Spacing::SM);

        section_header(ui, "Menu Bar");

        settings_card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("显示模式")
                            .size(FontSize::MD)
                            .color(Theme::TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new("菜单栏显示的详细程度")
                            .size(FontSize::SM)
                            .color(Theme::TEXT_MUTED),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let display_modes = [
                        ("minimal", "Minimal"),
                        ("compact", "Compact"),
                        ("detailed", "Detailed"),
                    ];

                    egui::Frame::none()
                        .fill(Theme::BG_TERTIARY)
                        .stroke(Stroke::new(1.0, Theme::BORDER_SUBTLE))
                        .rounding(Rounding::same(Radius::SM))
                        .inner_margin(egui::Margin::symmetric(Spacing::XS, 2.0))
                        .show(ui, |ui| {
                            egui::ComboBox::from_id_salt("display_mode")
                                .selected_text(
                                    display_modes
                                        .iter()
                                        .find(|(val, _)| {
                                            *val == self.settings.menu_bar_display_mode
                                        })
                                        .map(|(_, label)| *label)
                                        .unwrap_or("Detailed"),
                                )
                                .show_ui(ui, |ui| {
                                    for (value, label) in display_modes {
                                        if ui
                                            .selectable_value(
                                                &mut self.settings.menu_bar_display_mode,
                                                value.to_string(),
                                                label,
                                            )
                                            .changed()
                                        {
                                            self.settings_changed = true;
                                        }
                                    }
                                });
                        });
                });
            });
        });

        ui.add_space(Spacing::SM);

        section_header(ui, "Fun");

        settings_card(ui, |ui| {
            let mut surprise = self.settings.surprise_animations;
            if setting_toggle(
                ui,
                "Surprise me",
                "Random animations on tray icon",
                &mut surprise,
            ) {
                self.settings.surprise_animations = surprise;
                self.settings_changed = true;
            }
        });
    }

    fn show_about_tab(&mut self, ui: &mut egui::Ui) {
        ui.add_space(Spacing::LG);

        // Version row
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("CodexBar")
                    .size(FontSize::LG)
                    .color(Theme::TEXT_PRIMARY)
                    .strong(),
            );
            ui.label(
                RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                    .size(FontSize::SM)
                    .color(Theme::TEXT_MUTED),
            );
        });
        ui.add_space(Spacing::XS);
        ui.label(
            RichText::new("CodexBar 的 Windows 移植版本。在系统托盘中追踪 AI 服务商用量。")
                .size(FontSize::SM)
                .color(Theme::TEXT_SECONDARY),
        );

        ui.add_space(Spacing::LG);
        setting_divider(ui);
        ui.add_space(Spacing::SM);

        // Links row
        ui.horizontal(|ui| {
            if ui.link("GitHub Repository").clicked() {
                let _ = open::that("https://github.com/Finesssee/Win-CodexBar");
            }
            ui.label(RichText::new("·").color(Theme::TEXT_DIM));
            if ui.link("原始 macOS 版本").clicked() {
                let _ = open::that("https://github.com/steipete/CodexBar");
            }
        });

        ui.add_space(Spacing::SM);

        // Check for updates row
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("检查更新")
                            .size(FontSize::SM)
                            .color(Theme::TEXT_PRIMARY),
                    )
                    .stroke(Stroke::new(1.0, Theme::BORDER_SUBTLE))
                    .fill(Theme::CARD_BG)
                    .rounding(Rounding::same(Radius::SM)),
                )
                .clicked()
            {
                let _ = open::that("https://github.com/Finesssee/Win-CodexBar/releases");
            }
        });

        ui.add_space(Spacing::LG);
        setting_divider(ui);
        ui.add_space(Spacing::SM);

        // Build info row
        ui.label(
            RichText::new("基于 Rust + egui 构建")
                .size(FontSize::XS)
                .color(Theme::TEXT_DIM),
        );
    }
}

fn settings_position_near_main_window(
    main_rect: Rect,
    settings_size: Vec2,
    monitor_size: Rect,
) -> egui::Pos2 {
    let margin = 12.0;
    let gap = 12.0;

    let right_space = monitor_size.max.x - main_rect.max.x - gap - margin;
    let left_space = main_rect.min.x - monitor_size.min.x - gap - margin;
    let bottom_space = monitor_size.max.y - main_rect.max.y - gap - margin;
    let top_space = main_rect.min.y - monitor_size.min.y - gap - margin;

    let mut best_side = "right";
    let mut best_space = right_space;
    for (side, space) in [
        ("left", left_space),
        ("bottom", bottom_space),
        ("top", top_space),
    ] {
        if space > best_space {
            best_side = side;
            best_space = space;
        }
    }

    let min_x = monitor_size.min.x + margin;
    let min_y = monitor_size.min.y + margin;
    let max_x = (monitor_size.max.x - settings_size.x - margin).max(min_x);
    let max_y = (monitor_size.max.y - settings_size.y - margin).max(min_y);
    let clamp_x = |value: f32| {
        if max_x <= min_x {
            min_x
        } else {
            value.clamp(min_x, max_x)
        }
    };
    let clamp_y = |value: f32| {
        if max_y <= min_y {
            min_y
        } else {
            value.clamp(min_y, max_y)
        }
    };

    let (x, y) = match best_side {
        "right" => (clamp_x(main_rect.max.x + gap), clamp_y(main_rect.min.y)),
        "left" => (
            clamp_x(main_rect.min.x - settings_size.x - gap),
            clamp_y(main_rect.min.y),
        ),
        "bottom" => (clamp_x(main_rect.min.x), clamp_y(main_rect.max.y + gap)),
        _ => (
            clamp_x(main_rect.min.x),
            clamp_y(main_rect.min.y - settings_size.y - gap),
        ),
    };

    egui::pos2(x, y)
}

fn work_area_rect(ctx: &egui::Context) -> Option<Rect> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::RECT as WinRect;
        use windows::Win32::UI::WindowsAndMessaging::{
            SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
        };

        let mut rect = WinRect::default();
        let ok = unsafe {
            SystemParametersInfoW(
                SPI_GETWORKAREA,
                0,
                Some((&mut rect as *mut WinRect).cast()),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )
            .is_ok()
        };

        if ok {
            let pixels_per_point = ctx.pixels_per_point().max(0.1);
            return Some(Rect::from_min_max(
                egui::pos2(
                    rect.left as f32 / pixels_per_point,
                    rect.top as f32 / pixels_per_point,
                ),
                egui::pos2(
                    rect.right as f32 / pixels_per_point,
                    rect.bottom as f32 / pixels_per_point,
                ),
            ));
        }
    }

    ctx.input(|i| {
        i.viewport()
            .monitor_size
            .map(|size| Rect::from_min_size(egui::pos2(0.0, 0.0), size))
    })
}

// ════════════════════════════════════════════════════════════════════════════════
// VIEWPORT SETTINGS UI RENDERER
// ════════════════════════════════════════════════════════════════════════════════

/// Render the settings UI inside the viewport using shared state
fn render_settings_ui(ui: &mut egui::Ui, shared_state: &Arc<Mutex<PreferencesSharedState>>) {
    // Get current tab and language from shared state
    let (active_tab, preferences_section, ui_language) = if let Ok(state) = shared_state.lock() {
        (
            state.active_tab,
            state.preferences_section,
            state.settings.ui_language,
        )
    } else {
        (
            PreferencesTab::Preferences,
            PreferencesTab::General,
            Language::English,
        )
    };

    #[cfg(debug_assertions)]
    if let Ok(mut state) = shared_state.lock() {
        state.debug_tab_targets.clear();
        state.debug_viewport_outer_rect = ui.ctx().input(|i| i.viewport().outer_rect);
    }

    // Map legacy tabs to the consolidated categories for rendering
    let effective_tab = match active_tab {
        PreferencesTab::General | PreferencesTab::Display | PreferencesTab::Advanced => {
            PreferencesTab::Preferences
        }
        PreferencesTab::Providers | PreferencesTab::ApiKeys | PreferencesTab::Cookies => {
            PreferencesTab::Accounts
        }
        other => other,
    };

    let surface_palette = providers_surface_palette();
    let shell_fill = surface_palette.shell_fill;
    let content_fill = surface_palette.content_fill;
    let shell_rect = ui.max_rect();
    ui.painter().rect_filled(shell_rect, 0.0, shell_fill);

    ui.vertical(|ui| {
        // ═══════════════════════════════════════════════════════════
        // COMMAND STRIP — 4 intent-driven tabs
        // ═══════════════════════════════════════════════════════════
        let tabs = PreferencesTab::top_level_tabs();
        let tab_height = 28.0;
        let nav_padding = Spacing::SM;
        let nav_chrome = settings_nav_chrome();

        let (tab_bar_rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), tab_height),
            egui::Sense::hover(),
        );

        let tab_count = tabs.len() as f32;
        let tab_width = (tab_bar_rect.width() - nav_padding * 2.0) / tab_count;

        for (i, tab) in tabs.iter().enumerate() {
            let is_selected = effective_tab == *tab;

            let tab_rect = Rect::from_min_size(
                egui::pos2(
                    tab_bar_rect.min.x + nav_padding + i as f32 * tab_width,
                    tab_bar_rect.min.y,
                ),
                Vec2::new(tab_width, tab_height),
            );

            let response = ui.interact(
                tab_rect,
                ui.id().with(format!("tab_{}", tab.label())),
                egui::Sense::click(),
            );

            if is_selected {
                let selected_rect = tab_rect.shrink2(Vec2::new(7.0, 5.0));
                ui.painter().rect_filled(
                    selected_rect,
                    Rounding::same(6.0),
                    nav_chrome.selected_fill,
                );
                ui.painter().rect_stroke(
                    selected_rect,
                    Rounding::same(6.0),
                    nav_chrome.selected_stroke,
                );
            } else if response.hovered() {
                let hover_rect = tab_rect.shrink2(Vec2::new(7.0, 5.0));
                ui.painter()
                    .rect_filled(hover_rect, Rounding::same(6.0), nav_chrome.hover_fill);
            }

            let icon_color = if is_selected {
                Theme::TEXT_PRIMARY
            } else {
                Theme::TEXT_SECONDARY.gamma_multiply(1.14)
            };
            let label_color = if is_selected {
                Theme::TEXT_PRIMARY
            } else {
                Theme::TAB_TEXT_INACTIVE.gamma_multiply(1.18)
            };

            let tab_label = match *tab {
                PreferencesTab::Preferences => preferences_tab_shell_label(ui_language),
                PreferencesTab::Accounts => locale_text(ui_language, LocaleKey::TabProviders),
                PreferencesTab::Shortcuts => locale_text(ui_language, LocaleKey::TabShortcuts),
                PreferencesTab::About => locale_text(ui_language, LocaleKey::TabAbout),
                _ => tab.label(),
            };

            // Icon + label centered
            let center = tab_rect.center();
            let label_galley = ui.painter().layout_no_wrap(
                tab_label.to_string(),
                egui::FontId::proportional(FontSize::SM),
                Color32::WHITE,
            );
            let total_content_width = 13.0 + 4.0 + label_galley.size().x;
            let start_x = center.x - total_content_width / 2.0;

            ui.painter().text(
                egui::pos2(start_x, center.y),
                egui::Align2::LEFT_CENTER,
                tab.icon(),
                egui::FontId::proportional(12.0),
                icon_color,
            );
            ui.painter().text(
                egui::pos2(start_x + 17.0, center.y),
                egui::Align2::LEFT_CENTER,
                tab_label,
                egui::FontId::proportional(FontSize::SM),
                label_color,
            );

            if response.clicked()
                && let Ok(mut state) = shared_state.lock()
            {
                state.active_tab = *tab;
            }

            #[cfg(debug_assertions)]
            if let Ok(mut state) = shared_state.lock() {
                state.debug_tab_targets.push(PreferencesDebugTabTarget {
                    name: format!(
                        "preferences:{}",
                        tab.label().to_lowercase().replace(' ', "_")
                    ),
                    rect: tab_rect,
                    hovered: response.hovered(),
                    contains_pointer: response.contains_pointer(),
                    clicked: response.clicked(),
                    pointer_button_down_on: response.is_pointer_button_down_on(),
                    interact_pointer_pos: response.interact_pointer_pos(),
                });
            }
        }

        // Separator
        let separator_rect = Rect::from_min_size(
            egui::pos2(tab_bar_rect.min.x + nav_padding, tab_bar_rect.max.y),
            Vec2::new(ui.available_width(), 1.0),
        );
        ui.painter()
            .rect_filled(separator_rect, 0.0, Theme::SEPARATOR);

        // ═══════════════════════════════════════════════════════════
        // TAB CONTENT
        // ═══════════════════════════════════════════════════════════
        let content_height = ui.available_height();

        match effective_tab {
            PreferencesTab::Accounts => {
                // Accounts = Providers sidebar/detail + API Keys + Cookies
                render_providers_tab_layout(ui, content_height, shared_state);
            }
            PreferencesTab::Preferences => {
                egui::Frame::none()
                    .fill(content_fill)
                    .inner_margin(egui::Margin::symmetric(Spacing::MD, Spacing::MD))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("settings_content_preferences")
                            .max_height(content_height)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                render_preferences_section_selector(
                                    ui,
                                    shared_state,
                                    preferences_section,
                                );
                                ui.add_space(Spacing::LG);
                                match preferences_section {
                                    PreferencesTab::Display => render_display_tab(ui, shared_state),
                                    PreferencesTab::Advanced => {
                                        render_advanced_tab(ui, shared_state)
                                    }
                                    _ => render_general_tab(ui, shared_state),
                                }
                                ui.add_space(Spacing::XL);
                            });
                    });
            }
            PreferencesTab::Shortcuts => {
                egui::Frame::none()
                    .fill(content_fill)
                    .inner_margin(egui::Margin::symmetric(Spacing::MD, Spacing::MD))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("settings_content_shortcuts")
                            .max_height(content_height)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                render_shortcuts_tab(ui, shared_state);
                                ui.add_space(Spacing::XL);
                            });
                    });
            }
            PreferencesTab::About => {
                egui::Frame::none()
                    .fill(content_fill)
                    .inner_margin(egui::Margin::symmetric(Spacing::MD, Spacing::MD))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("settings_content_about")
                            .max_height(content_height)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                render_about_tab(ui, shared_state);
                                ui.add_space(Spacing::XL);
                            });
                    });
            }
            _ => {
                // Fallback for any legacy tab reference
                egui::Frame::none()
                    .fill(content_fill)
                    .inner_margin(egui::Margin::symmetric(Spacing::MD, Spacing::MD))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("settings_content_fallback")
                            .max_height(content_height)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                render_general_tab(ui, shared_state);
                                ui.add_space(Spacing::XXL);
                            });
                    });
            }
        }
    });
}

/// Render Providers tab with macOS-style sidebar + detail layout
fn render_providers_tab_layout(
    ui: &mut egui::Ui,
    available_height: f32,
    shared_state: &Arc<Mutex<PreferencesSharedState>>,
) {
    // macOS metrics
    let sidebar_style = active_provider_sidebar_style();
    let sidebar_corner_radius = 12.0; // sidebarCornerRadius
    let icon_size = 16.0;
    let total_width = ui.available_width();
    let sidebar_width = (total_width * 0.42).clamp(244.0, 286.0);
    let detail_width = (total_width - sidebar_width - Spacing::SM).max(0.0);
    let surface_palette = providers_surface_palette();
    let detail_fill = surface_palette.detail_fill;
    let detail_stroke = surface_palette.detail_stroke;

    // Get selected provider
    let selected_provider = if let Ok(state) = shared_state.lock() {
        state.selected_provider
    } else {
        None
    };

    let providers = ProviderId::all();
    let selected = selected_provider.unwrap_or(providers[0]);

    // Create a horizontal layout with two fixed regions
    let sidebar_rect = ui.available_rect_before_wrap();

    // LEFT SIDEBAR - Fixed width region
    let _sidebar_response = ui.allocate_ui_with_layout(
        Vec2::new(sidebar_width, available_height),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            let mut frame = egui::Frame::none()
                .rounding(Rounding::same(sidebar_corner_radius))
                .inner_margin(sidebar_style.inner_margin);
            if let Some(fill) = sidebar_style.frame_fill {
                frame = frame.fill(fill);
            }
            if let Some(stroke) = sidebar_style.frame_stroke {
                frame = frame.stroke(stroke);
            }
            frame.show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("provider_sidebar_scroll_v3")
                    .max_height(available_height - Spacing::LG * 2.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for provider_id in providers {
                            let is_selected = *provider_id == selected;
                            let is_enabled = if let Ok(state) = shared_state.lock() {
                                state
                                    .settings
                                    .enabled_providers
                                    .contains(provider_id.cli_name())
                            } else {
                                true
                            };
                            let row_height =
                                provider_sidebar_row_height(*provider_id, is_enabled, shared_state);
                            let row_width = sidebar_width - sidebar_style.inner_margin * 2.0;

                            ui.add_space(sidebar_style.item_spacing_y);

                            let (rect, response) = ui.allocate_exact_size(
                                Vec2::new(row_width, row_height),
                                egui::Sense::click(),
                            );

                            // Use a light selection stroke/fill instead of a chunky slab.
                            if is_selected {
                                response.scroll_to_me(Some(egui::Align::Center));
                                ui.painter().rect_filled(
                                    rect,
                                    Rounding::same(sidebar_style.row_corner_radius),
                                    sidebar_style.selected_fill,
                                );
                                ui.painter().rect_stroke(
                                    rect,
                                    Rounding::same(sidebar_style.row_corner_radius),
                                    sidebar_style.selected_stroke,
                                );
                            } else if response.hovered() {
                                ui.painter().rect_filled(
                                    rect,
                                    Rounding::same(sidebar_style.row_corner_radius),
                                    sidebar_style.hover_fill,
                                );
                            }

                            let content_rect = rect.shrink2(Vec2::new(4.0, 2.0));
                            let mut checkbox_clicked = false;
                            ui.scope_builder(
                                egui::UiBuilder::new()
                                    .max_rect(content_rect)
                                    .layout(egui::Layout::left_to_right(egui::Align::Min)),
                                |ui| {
                                    render_provider_sidebar_row(
                                        ui,
                                        *provider_id,
                                        is_enabled,
                                        is_selected,
                                        shared_state,
                                        icon_size,
                                        &mut checkbox_clicked,
                                    );
                                },
                            );

                            // Handle clicks
                            if response.clicked()
                                && !checkbox_clicked
                                && let Ok(mut state) = shared_state.lock()
                            {
                                state.selected_provider = Some(*provider_id);
                            }

                            ui.add_space(0.0);
                        }
                    });
            });
        },
    );

    // Move cursor to the right of sidebar
    let detail_rect = egui::Rect::from_min_size(
        egui::pos2(
            sidebar_rect.min.x + sidebar_width + Spacing::MD,
            sidebar_rect.min.y,
        ),
        Vec2::new(detail_width, available_height),
    );

    // RIGHT PANEL - Detail view
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(detail_rect), |ui| {
        egui::Frame::none()
            .fill(detail_fill)
            .stroke(detail_stroke)
            .rounding(Rounding::same(12.0))
            .inner_margin(egui::Margin::symmetric(Spacing::MD, Spacing::SM))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("provider_detail_scroll_v3")
                    .max_height(available_height - Spacing::SM)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let max_content_width = provider_detail_max_content_width();
                        let available = ui.available_width();
                        if available > max_content_width {
                            ui.add_space((available - max_content_width) * 0.5);
                        }
                        ui.set_max_width(max_content_width.min(available));
                        render_provider_detail_panel(ui, selected, shared_state);
                    });
            });
    });
}

fn render_provider_sidebar_row(
    ui: &mut egui::Ui,
    provider_id: ProviderId,
    is_enabled: bool,
    is_selected: bool,
    shared_state: &Arc<Mutex<PreferencesSharedState>>,
    icon_size: f32,
    checkbox_clicked: &mut bool,
) {
    let provider_name = provider_id.cli_name();
    let brand_color = provider_color(provider_name);
    let ui_language = if let Ok(state) = shared_state.lock() {
        state.settings.ui_language
    } else {
        Language::English
    };
    let entry = if let Ok(state) = shared_state.lock() {
        state
            .cached_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.entry_for(provider_id).cloned())
    } else {
        None
    };
    let runtime_error = if let Ok(state) = shared_state.lock() {
        state.runtime_provider_errors.get(&provider_id).cloned()
    } else {
        None
    };
    let subtitle = provider_sidebar_subtitle(
        provider_id,
        is_enabled,
        entry.as_ref(),
        runtime_error.as_deref(),
        ui_language,
    );
    let (subtitle_primary, subtitle_secondary) = provider_sidebar_display_lines(&subtitle);

    render_provider_sidebar_reorder_handle(ui, is_selected);
    ui.add_space(3.0);

    render_provider_sidebar_icon(ui, provider_name, brand_color, icon_size);
    ui.add_space(5.0);

    let name_color = if is_selected {
        Theme::TEXT_PRIMARY
    } else if is_enabled {
        Theme::TEXT_PRIMARY.gamma_multiply(1.02)
    } else {
        Theme::TEXT_SECONDARY.gamma_multiply(1.30)
    };
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 1.0;
        ui.horizontal(|ui| {
            let mut name_text = RichText::new(provider_preferences_display_name(provider_id))
                .size(FontSize::SM)
                .color(name_color);
            if is_selected {
                name_text = name_text.strong();
            }
            ui.label(name_text);
            if is_enabled {
                ui.add_space(3.0);
                let (dot_rect, _) =
                    ui.allocate_exact_size(Vec2::new(6.0, 6.0), egui::Sense::hover());
                ui.painter().circle_filled(
                    dot_rect.center(),
                    1.35,
                    Theme::GREEN.gamma_multiply(if is_selected { 0.58 } else { 0.42 }),
                );
            }
        });

        let primary_subtitle_color = if is_selected {
            Theme::TEXT_SECONDARY.gamma_multiply(1.32)
        } else if is_enabled {
            Theme::TEXT_SECONDARY.gamma_multiply(1.18)
        } else {
            Theme::TEXT_SECONDARY.gamma_multiply(1.22)
        };
        ui.label(
            RichText::new(subtitle_primary.as_str())
                .size(FontSize::XS)
                .color(primary_subtitle_color),
        );
        if let Some(secondary_line) = subtitle_secondary.as_deref() {
            let secondary_subtitle_color = if is_selected {
                Theme::TEXT_DIM.gamma_multiply(1.16)
            } else if is_enabled {
                Theme::TEXT_DIM.gamma_multiply(1.04)
            } else {
                Theme::TEXT_DIM.gamma_multiply(1.08)
            };
            ui.label(
                RichText::new(secondary_line)
                    .size(FontSize::XS)
                    .color(secondary_subtitle_color),
            );
        }
    });

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        *checkbox_clicked = render_provider_sidebar_checkbox(ui, is_enabled);
    });
}

fn provider_sidebar_subtitle(
    provider_id: ProviderId,
    is_enabled: bool,
    entry: Option<&WidgetProviderEntry>,
    runtime_error: Option<&str>,
    ui_language: Language,
) -> String {
    let source_hint = provider_sidebar_source_hint(provider_id, ui_language);

    if !is_enabled {
        return format!(
            "{} — {}\n{}",
            locale_text(ui_language, LocaleKey::ProviderDisabled),
            provider_disabled_detail_hint(provider_id, runtime_error, ui_language),
            locale_text(ui_language, LocaleKey::ProviderUsageNotFetchedYet)
        );
    }

    let Some(entry) = entry else {
        return format!(
            "{}\n{}",
            locale_text(ui_language, LocaleKey::ProviderNotDetected),
            locale_text(ui_language, LocaleKey::ProviderLastFetchFailed)
        );
    };

    let status_detail = if provider_sidebar_has_usage(entry) {
        provider_sidebar_updated_display(entry.updated_at, ui_language)
    } else {
        locale_text(ui_language, LocaleKey::ProviderUsageNotFetchedYet).to_string()
    };

    format!("{source_hint}\n{status_detail}")
}

fn provider_sidebar_display_lines(subtitle: &str) -> (String, Option<String>) {
    let (primary, secondary) = subtitle
        .split_once('\n')
        .map(|(primary, secondary)| (primary, Some(secondary)))
        .unwrap_or((subtitle, None));

    let primary = ellipsize_text(primary, 40);
    let secondary = secondary.map(|line| ellipsize_text(line, 22));

    (primary, secondary)
}

fn provider_detail_display_text(subtitle: &str) -> String {
    if let Some((primary, secondary)) = subtitle.split_once(" • ") {
        return format!("{primary}\n{secondary}");
    }

    subtitle.to_string()
}

fn ellipsize_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let truncated: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{truncated}…")
}

fn provider_sidebar_row_height(
    _provider_id: ProviderId,
    _is_enabled: bool,
    _shared_state: &Arc<Mutex<PreferencesSharedState>>,
) -> f32 {
    58.0
}

fn provider_disabled_detail_hint(
    provider_id: ProviderId,
    runtime_error: Option<&str>,
    ui_language: Language,
) -> String {
    match provider_id {
        ProviderId::Cursor | ProviderId::OpenCode | ProviderId::Kimi => {
            locale_text(ui_language, LocaleKey::ProviderSourceWebShort).to_string()
        }
        ProviderId::Copilot => {
            locale_text(ui_language, LocaleKey::ProviderSourceGithubApiShort).to_string()
        }
        ProviderId::Zai | ProviderId::Synthetic | ProviderId::OpenRouter | ProviderId::NanoGPT => {
            locale_text(ui_language, LocaleKey::ProviderSourceApiShort).to_string()
        }
        ProviderId::MiniMax | ProviderId::Alibaba | ProviderId::Codex => {
            locale_text(ui_language, LocaleKey::ProviderSourceAutoShort).to_string()
        }
        ProviderId::Kiro => kiro_disabled_detail_hint(runtime_error, ui_language),
        ProviderId::Claude
        | ProviderId::Factory
        | ProviderId::Gemini
        | ProviderId::Antigravity
        | ProviderId::VertexAI
        | ProviderId::Augment
        | ProviderId::KimiK2
        | ProviderId::Amp
        | ProviderId::Warp
        | ProviderId::Ollama
        | ProviderId::JetBrains
        | ProviderId::Infini => format!(
            "{} {}",
            provider_id.cli_name(),
            locale_text(ui_language, LocaleKey::ProviderNotDetected)
        ),
    }
}

fn kiro_disabled_detail_hint(runtime_error: Option<&str>, ui_language: Language) -> String {
    let source = locale_text(ui_language, LocaleKey::ProviderSourceKiroEnvShort);
    if let Some(error) = runtime_error {
        let trimmed = error.trim();
        if !trimmed.is_empty() {
            return format!("{source}: {trimmed}");
        }
    }
    if crate::providers::kiro::find_kiro_cli().is_none() {
        format!("{source}: kiro-cli: No such file or directory")
    } else {
        source.to_string()
    }
}

fn provider_preferences_display_name(provider_id: ProviderId) -> &'static str {
    match provider_id {
        ProviderId::Factory => "Droid",
        _ => provider_id.display_name(),
    }
}

fn provider_sidebar_source_hint(provider_id: ProviderId, ui_language: Language) -> String {
    let source_key = match provider_id {
        ProviderId::Cursor
        | ProviderId::Factory
        | ProviderId::Kimi
        | ProviderId::KimiK2
        | ProviderId::Augment
        | ProviderId::OpenCode
        | ProviderId::Amp
        | ProviderId::Ollama
        | ProviderId::Alibaba
        | ProviderId::Infini => LocaleKey::ProviderSourceWebShort,
        ProviderId::Claude => LocaleKey::ProviderSourceAutoShort,
        ProviderId::Gemini | ProviderId::Antigravity | ProviderId::JetBrains => {
            LocaleKey::ProviderSourceCliShort
        }
        ProviderId::Copilot => LocaleKey::ProviderSourceGithubApiShort,
        ProviderId::Zai
        | ProviderId::VertexAI
        | ProviderId::OpenRouter
        | ProviderId::Synthetic
        | ProviderId::NanoGPT
        | ProviderId::Warp => LocaleKey::ProviderSourceApiShort,
        ProviderId::Kiro => LocaleKey::ProviderSourceKiroEnvShort,
        ProviderId::Codex | ProviderId::MiniMax => LocaleKey::ProviderSourceAutoShort,
    };

    locale_text(ui_language, source_key).to_string()
}

fn provider_detail_source_display(provider_id: ProviderId, ui_language: Language) -> String {
    match provider_id {
        // Claude's settings pane reflects the configured automatic source label here.
        ProviderId::Claude => {
            locale_text(ui_language, LocaleKey::ProviderSourceAutoShort).to_string()
        }
        _ => provider_sidebar_source_hint(provider_id, ui_language),
    }
}

fn cursor_cookie_source_label(source: &str, ui_language: Language) -> String {
    match source {
        "manual" => "Manual".to_string(),
        _ => locale_text(ui_language, LocaleKey::Automatic).to_string(),
    }
}

fn cursor_cookie_source_help(source: &str, ui_language: Language) -> String {
    match source {
        "manual" => "Paste a Cookie header from a cursor.com request.".to_string(),
        _ => locale_text(ui_language, LocaleKey::ProviderCursorCookieSourceHelp).to_string(),
    }
}

fn opencode_cookie_source_label(source: &str, ui_language: Language) -> String {
    match source {
        "manual" => "Manual".to_string(),
        _ => locale_text(ui_language, LocaleKey::Automatic).to_string(),
    }
}

fn opencode_cookie_source_help(source: &str) -> String {
    match source {
        "manual" => "Paste a Cookie header from the billing page.".to_string(),
        _ => "Automatic imports browser cookies from opencode.ai.".to_string(),
    }
}

fn factory_cookie_source_label(source: &str, ui_language: Language) -> String {
    match source {
        "manual" => "Manual".to_string(),
        _ => locale_text(ui_language, LocaleKey::Automatic).to_string(),
    }
}

fn factory_cookie_source_help(source: &str) -> String {
    match source {
        "manual" => "Paste a Cookie header from Factory.".to_string(),
        _ => "Automatic imports browser cookies and WorkOS sessions.".to_string(),
    }
}

fn alibaba_cookie_source_label(source: &str, ui_language: Language) -> String {
    match source {
        "manual" => "Manual".to_string(),
        _ => locale_text(ui_language, LocaleKey::Automatic).to_string(),
    }
}

fn alibaba_cookie_source_help(source: &str) -> String {
    match source {
        "manual" => "Paste a Cookie header from Model Studio or Bailian.".to_string(),
        _ => "Automatic imports browser cookies from Model Studio / Bailian.".to_string(),
    }
}

fn kimi_cookie_source_label(source: &str, ui_language: Language) -> String {
    match source {
        "manual" => "Manual".to_string(),
        "off" => locale_text(ui_language, LocaleKey::ProviderDisabled).to_string(),
        _ => locale_text(ui_language, LocaleKey::Automatic).to_string(),
    }
}

fn kimi_cookie_source_help(source: &str) -> String {
    match source {
        "manual" => "Paste a cookie header or the kimi-auth token value.".to_string(),
        "off" => "Kimi cookies are disabled.".to_string(),
        _ => "Automatic imports browser cookies.".to_string(),
    }
}

fn minimax_cookie_source_label(source: &str, ui_language: Language) -> String {
    match source {
        "manual" => "Manual".to_string(),
        _ => locale_text(ui_language, LocaleKey::Automatic).to_string(),
    }
}

fn minimax_cookie_source_help(source: &str) -> String {
    match source {
        "manual" => "Paste a Cookie header from the Coding Plan page.".to_string(),
        _ => "Automatic imports browser cookies and Coding Plan tokens.".to_string(),
    }
}

fn augment_cookie_source_label(source: &str, ui_language: Language) -> String {
    match source {
        "manual" => "Manual".to_string(),
        _ => locale_text(ui_language, LocaleKey::Automatic).to_string(),
    }
}

fn augment_cookie_source_help(source: &str) -> String {
    match source {
        "manual" => "Paste a Cookie header from the Augment dashboard.".to_string(),
        _ => "Automatic imports browser cookies.".to_string(),
    }
}

fn amp_cookie_source_label(source: &str, ui_language: Language) -> String {
    match source {
        "manual" => "Manual".to_string(),
        _ => locale_text(ui_language, LocaleKey::Automatic).to_string(),
    }
}

fn amp_cookie_source_help(source: &str) -> String {
    match source {
        "manual" => "Paste a Cookie header from Amp settings.".to_string(),
        _ => "Automatic imports browser cookies.".to_string(),
    }
}

fn ollama_cookie_source_label(source: &str, ui_language: Language) -> String {
    match source {
        "manual" => "Manual".to_string(),
        _ => locale_text(ui_language, LocaleKey::Automatic).to_string(),
    }
}

fn ollama_cookie_source_help(source: &str) -> String {
    match source {
        "manual" => "Paste a Cookie header from Ollama settings.".to_string(),
        _ => "Automatic imports browser cookies.".to_string(),
    }
}

fn alibaba_region_label(region: &str) -> &'static str {
    match region {
        "cn" => "China Mainland (Bailian)",
        _ => "International (Model Studio)",
    }
}

fn alibaba_dashboard_url(region: &str) -> &'static str {
    match region {
        "cn" => "https://bailian.console.aliyun.com/cn-beijing/?tab=model#/efm/coding_plan",
        _ => {
            "https://modelstudio.console.alibabacloud.com/ap-southeast-1/?tab=coding-plan#/efm/detail"
        }
    }
}

fn zai_region_label(region: &str) -> &'static str {
    match region {
        "china" => "China Mainland (BigModel)",
        _ => "Global",
    }
}

fn minimax_region_label(region: &str) -> &'static str {
    match region {
        "china" => "China Mainland (.com)",
        _ => "Global (.io)",
    }
}

fn gemini_cli_credentials_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".gemini").join("oauth_creds.json"))
}

fn gemini_cli_signed_in() -> bool {
    gemini_cli_credentials_path()
        .map(|path| path.exists())
        .unwrap_or(false)
}

fn vertexai_credentials_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
        && !path.trim().is_empty()
    {
        return Some(PathBuf::from(path));
    }

    dirs::config_dir().map(|config| {
        config
            .join("gcloud")
            .join("application_default_credentials.json")
    })
}

fn compact_credentials_path(path: &str) -> String {
    let normalized_path = path.replace('\\', "/");
    let path_ref = Path::new(&normalized_path);
    let components: Vec<&str> = normalized_path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();

    if components.len() >= 2 {
        format!(
            "{}/{}",
            components[components.len() - 2],
            components[components.len() - 1]
        )
    } else if let Some(file_name) = path_ref.file_name() {
        file_name.to_string_lossy().into_owned()
    } else {
        path.to_string()
    }
}

fn vertexai_signed_in() -> bool {
    vertexai_credentials_path()
        .map(|path| path.exists())
        .unwrap_or(false)
}

fn jetbrains_detected_ide_paths() -> Vec<PathBuf> {
    let Some(config_dir) = dirs::config_dir() else {
        return Vec::new();
    };
    let product_roots = [config_dir.join("JetBrains"), config_dir.join("Google")];
    let mut paths = Vec::new();

    for root in product_roots {
        if !root.exists() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    paths.push(path);
                }
            }
        }
    }

    paths.sort();
    paths
}

fn jetbrains_detected_ide_path() -> Option<PathBuf> {
    jetbrains_detected_ide_paths().into_iter().next()
}

fn jetbrains_display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn should_show_token_accounts_section(
    provider_id: ProviderId,
    shared_state: &Arc<Mutex<PreferencesSharedState>>,
) -> bool {
    let Some(support) = TokenAccountSupport::for_provider(provider_id) else {
        return false;
    };

    let (accounts_data, show_add_input, settings) = if let Ok(state) = shared_state.lock() {
        (
            state
                .token_accounts
                .get(&provider_id)
                .cloned()
                .unwrap_or_default(),
            state.show_add_account_input,
            state.settings.clone(),
        )
    } else {
        (ProviderAccountData::default(), false, Settings::default())
    };

    if !support.requires_manual_cookie_source {
        return true;
    }

    if provider_id == ProviderId::Cursor && settings.cursor_cookie_source == "manual" {
        return true;
    }

    if provider_id == ProviderId::OpenCode && settings.opencode_cookie_source == "manual" {
        return true;
    }

    if provider_id == ProviderId::Factory && settings.factory_cookie_source == "manual" {
        return true;
    }

    if provider_id == ProviderId::Alibaba && settings.alibaba_cookie_source == "manual" {
        return true;
    }

    if provider_id == ProviderId::MiniMax {
        if !settings.minimax_api_token.trim().is_empty() {
            return false;
        }
        if settings.minimax_cookie_source == "manual" {
            return true;
        }
    }

    if provider_id == ProviderId::Augment && settings.augment_cookie_source == "manual" {
        return true;
    }

    if provider_id == ProviderId::Amp && settings.amp_cookie_source == "manual" {
        return true;
    }

    if provider_id == ProviderId::Ollama && settings.ollama_cookie_source == "manual" {
        return true;
    }

    !accounts_data.accounts.is_empty() || show_add_input
}

fn provider_sidebar_has_usage(entry: &WidgetProviderEntry) -> bool {
    entry.primary.is_some()
        || entry.secondary.is_some()
        || entry.tertiary.is_some()
        || entry.credits_remaining.is_some()
        || entry.code_review_remaining_percent.is_some()
        || entry.token_usage.is_some()
}

fn provider_sidebar_updated_display(
    updated_at: chrono::DateTime<chrono::Utc>,
    ui_language: Language,
) -> String {
    let now = chrono::Utc::now();
    let diff = now - updated_at;
    if diff.num_seconds() < 60 {
        locale_text(ui_language, LocaleKey::UpdatedJustNow).to_string()
    } else if diff.num_minutes() < 60 {
        locale_text(ui_language, LocaleKey::UpdatedMinutesAgo)
            .replace("{}", &diff.num_minutes().to_string())
    } else if diff.num_hours() < 24 {
        locale_text(ui_language, LocaleKey::UpdatedHoursAgo)
            .replace("{}", &diff.num_hours().to_string())
    } else {
        locale_text(ui_language, LocaleKey::UpdatedDaysAgo)
            .replace("{}", &diff.num_days().to_string())
    }
}

fn provider_detail_subtitle(
    provider_id: ProviderId,
    is_enabled: bool,
    entry: Option<&WidgetProviderEntry>,
    runtime_error: Option<&str>,
    source_display: &str,
    updated_display: &str,
    ui_language: Language,
) -> String {
    if !is_enabled {
        return format!(
            "{} • {}",
            provider_disabled_detail_hint(provider_id, runtime_error, ui_language),
            locale_text(ui_language, LocaleKey::ProviderUsageNotFetchedYet)
        );
    }

    let Some(entry) = entry else {
        return format!(
            "{} • {}",
            locale_text(ui_language, LocaleKey::ProviderNotDetected),
            locale_text(ui_language, LocaleKey::ProviderLastFetchFailed)
        );
    };

    let status_detail = if provider_sidebar_has_usage(entry) {
        updated_display.to_string()
    } else {
        locale_text(ui_language, LocaleKey::ProviderUsageNotFetchedYet).to_string()
    };

    format!("{source_display} • {status_detail}")
}

fn provider_detail_status_value(
    provider_id: ProviderId,
    is_enabled: bool,
    entry: Option<&WidgetProviderEntry>,
    runtime_error: Option<&str>,
    ui_language: Language,
) -> String {
    if !is_enabled {
        return provider_disabled_detail_hint(provider_id, runtime_error, ui_language);
    }

    if entry.is_none() {
        return locale_text(ui_language, LocaleKey::ProviderLastFetchFailed).to_string();
    }

    locale_text(ui_language, LocaleKey::AllSystemsOperational).to_string()
}

fn shows_shared_provider_settings(provider_id: ProviderId) -> bool {
    !matches!(
        provider_id,
        ProviderId::Gemini
            | ProviderId::Antigravity
            | ProviderId::OpenCode
            | ProviderId::MiniMax
            | ProviderId::Factory
            | ProviderId::Kimi
            | ProviderId::Copilot
            | ProviderId::Alibaba
            | ProviderId::Amp
            | ProviderId::Augment
            | ProviderId::Infini
            | ProviderId::JetBrains
            | ProviderId::KimiK2
            | ProviderId::NanoGPT
            | ProviderId::Ollama
            | ProviderId::OpenRouter
            | ProviderId::Synthetic
            | ProviderId::VertexAI
            | ProviderId::Warp
            | ProviderId::Zai
    )
}

fn render_provider_sidebar_reorder_handle(ui: &mut egui::Ui, is_selected: bool) {
    let dot_color = if is_selected {
        Theme::TEXT_DIM.gamma_multiply(0.32)
    } else {
        Theme::TEXT_DIM.gamma_multiply(0.16)
    };
    let (rect, _) = ui.allocate_exact_size(Vec2::new(5.0, 13.0), egui::Sense::hover());
    let painter = ui.painter();

    for row in 0..3 {
        for col in 0..2 {
            let center = egui::pos2(
                rect.min.x + 1.35 + col as f32 * 2.7,
                rect.min.y + 2.7 + row as f32 * 3.2,
            );
            painter.circle_filled(center, 0.42, dot_color);
        }
    }
}

fn render_provider_sidebar_icon(
    ui: &mut egui::Ui,
    provider_name: &str,
    brand_color: Color32,
    icon_size: f32,
) {
    let has_svg_icon = VIEWPORT_ICON_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(texture) = cache.get_icon(ui.ctx(), provider_name, icon_size as u32) {
            ui.add(egui::Image::new(texture).fit_to_exact_size(Vec2::splat(icon_size)));
            true
        } else {
            false
        }
    });

    if !has_svg_icon {
        ui.label(
            RichText::new(provider_icon(provider_name))
                .size(icon_size)
                .color(brand_color),
        );
    }
}

fn render_provider_sidebar_checkbox(ui: &mut egui::Ui, is_enabled: bool) -> bool {
    let checkbox_size = 8.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(checkbox_size), egui::Sense::click());
    let border = Theme::BORDER_SUBTLE.gamma_multiply(if is_enabled { 0.66 } else { 0.46 });
    let fill = if is_enabled {
        Color32::from_rgba_unmultiplied(255, 255, 255, 3)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, Rounding::same(3.0), fill);
    ui.painter()
        .rect_stroke(rect, Rounding::same(3.0), Stroke::new(0.85, border));
    if is_enabled {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "✓",
            egui::FontId::proportional(6.5),
            Theme::TEXT_SECONDARY.gamma_multiply(0.98),
        );
    }
    response.clicked()
}

/// Render the provider detail panel (right side)
fn render_provider_detail_panel(
    ui: &mut egui::Ui,
    provider_id: ProviderId,
    shared_state: &Arc<Mutex<PreferencesSharedState>>,
) {
    let detail_chrome = provider_detail_chrome();
    let text_chrome = provider_detail_text_chrome();
    {
        let widgets = &mut ui.style_mut().visuals.widgets;
        widgets.inactive.bg_fill = detail_chrome.control_fill;
        widgets.inactive.weak_bg_fill = detail_chrome.control_fill;
        widgets.inactive.fg_stroke.color = Theme::TEXT_PRIMARY;
        widgets.hovered.bg_fill = detail_chrome.control_fill_hover;
        widgets.hovered.weak_bg_fill = detail_chrome.control_fill_hover;
        widgets.hovered.fg_stroke.color = Theme::TEXT_PRIMARY;
        widgets.active.bg_fill = detail_chrome.control_fill_active;
        widgets.active.weak_bg_fill = detail_chrome.control_fill_active;
        widgets.active.fg_stroke.color = Theme::TEXT_PRIMARY;
        widgets.open.bg_fill = detail_chrome.control_fill_active;
        widgets.open.weak_bg_fill = detail_chrome.control_fill_active;
        widgets.open.fg_stroke.color = Theme::TEXT_PRIMARY;
    }

    let brand_color = provider_color(provider_id.cli_name());

    // Get current language from shared state
    let ui_language = if let Ok(state) = shared_state.lock() {
        state.settings.ui_language
    } else {
        Language::English
    };

    let is_enabled = if let Ok(state) = shared_state.lock() {
        state
            .settings
            .enabled_providers
            .contains(provider_id.cli_name())
    } else {
        true
    };

    // Use cached snapshot data for this provider (loaded once, not every frame)
    let entry = if let Ok(state) = shared_state.lock() {
        state
            .cached_snapshot
            .as_ref()
            .and_then(|s| s.entry_for(provider_id).cloned())
    } else {
        None
    };
    let runtime_error = if let Ok(state) = shared_state.lock() {
        state.runtime_provider_errors.get(&provider_id).cloned()
    } else {
        None
    };

    // Extract data from entry or use defaults
    let primary_rate = entry.as_ref().and_then(|e| e.primary.clone());
    let secondary_rate = entry.as_ref().and_then(|e| e.secondary.clone());
    let tertiary_rate = entry.as_ref().and_then(|e| e.tertiary.clone());
    let credits_remaining = entry.as_ref().and_then(|e| e.credits_remaining);
    let code_review_percent = entry.as_ref().and_then(|e| e.code_review_remaining_percent);
    let token_usage = entry.as_ref().and_then(|e| e.token_usage.clone());
    let updated_at = entry.as_ref().map(|e| e.updated_at);
    let source_display = provider_detail_source_display(provider_id, ui_language);
    let updated_display = if let Some(ts) = updated_at {
        let now = chrono::Utc::now();
        let diff = now - ts;
        if diff.num_seconds() < 60 {
            locale_text(ui_language, LocaleKey::UpdatedJustNow).to_string()
        } else if diff.num_minutes() < 60 {
            locale_text(ui_language, LocaleKey::UpdatedMinutesAgo)
                .replace("{}", &diff.num_minutes().to_string())
        } else if diff.num_hours() < 24 {
            locale_text(ui_language, LocaleKey::UpdatedHoursAgo)
                .replace("{}", &diff.num_hours().to_string())
        } else {
            locale_text(ui_language, LocaleKey::UpdatedDaysAgo)
                .replace("{}", &diff.num_days().to_string())
        }
    } else {
        locale_text(ui_language, LocaleKey::NeverUpdated).to_string()
    };
    let detail_subtitle = provider_detail_subtitle(
        provider_id,
        is_enabled,
        entry.as_ref(),
        runtime_error.as_deref(),
        &source_display,
        &updated_display,
        ui_language,
    );
    let detail_subtitle_display = provider_detail_display_text(&detail_subtitle);
    let detail_status = provider_detail_status_value(
        provider_id,
        is_enabled,
        entry.as_ref(),
        runtime_error.as_deref(),
        ui_language,
    );

    // ═══════════════════════════════════════════════════════════
    // HEADER - Icon, name, version, refresh, toggle
    // ═══════════════════════════════════════════════════════════
    ui.horizontal(|ui| {
        // Large provider icon (28x28) - use SVG if available
        let icon_size = 24.0;
        let has_svg = VIEWPORT_ICON_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            if let Some(texture) =
                cache.get_icon(ui.ctx(), provider_id.cli_name(), icon_size as u32)
            {
                ui.add(egui::Image::new(texture).fit_to_exact_size(Vec2::splat(icon_size)));
                true
            } else {
                false
            }
        });

        if !has_svg {
            ui.label(
                RichText::new(provider_icon(provider_id.cli_name()))
                    .size(icon_size)
                    .color(brand_color),
            );
        }

        ui.add_space(10.0);

        let controls_reserve = detail_chrome.refresh_button_size + 54.0;
        let text_width = (ui.available_width() - controls_reserve).max(140.0);

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;

            ui.horizontal(|ui| {
                ui.add_sized(
                    [text_width, 18.0],
                    egui::Label::new(
                        RichText::new(provider_preferences_display_name(provider_id))
                            .size(FontSize::MD)
                            .color(Theme::TEXT_PRIMARY)
                            .strong(),
                    ),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut enabled = is_enabled;
                    if switch_toggle(
                        ui,
                        egui::Id::new(format!("detail_toggle_{}", provider_id.cli_name())),
                        &mut enabled,
                    ) && let Ok(mut state) = shared_state.lock()
                    {
                        let name = provider_id.cli_name().to_string();
                        if enabled {
                            state.settings.enabled_providers.insert(name);
                        } else {
                            state.settings.enabled_providers.remove(&name);
                        }
                        state.settings_changed = true;
                    }

                    ui.add_space(12.0);

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("↻")
                                    .size(FontSize::SM)
                                    .color(text_chrome.section_title),
                            )
                            .fill(detail_chrome.control_fill)
                            .stroke(detail_chrome.control_stroke)
                            .rounding(Rounding::same(Radius::SM))
                            .min_size(Vec2::splat(detail_chrome.refresh_button_size)),
                        )
                        .clicked()
                        && let Ok(mut state) = shared_state.lock()
                    {
                        state.refresh_requested = true;
                    }
                });
            });

            ui.add_sized(
                [text_width, 30.0],
                egui::Label::new(
                    RichText::new(detail_subtitle_display)
                        .size(FontSize::XS)
                        .color(text_chrome.subtitle),
                )
                .wrap(),
            );
        });
    });

    ui.add_space(6.0);

    // ═══════════════════════════════════════════════════════════
    // INFO GRID - Mac-like provider summary
    // ═══════════════════════════════════════════════════════════
    if !is_enabled {
        egui::Grid::new("provider_disabled_info_grid")
            .num_columns(2)
            .spacing([
                detail_chrome.info_grid_spacing_x,
                detail_chrome.info_grid_spacing_y,
            ])
            .show(ui, |ui| {
                info_row(
                    ui,
                    locale_text(ui_language, LocaleKey::State),
                    locale_text(ui_language, LocaleKey::ProviderDisabled),
                    detail_chrome.detail_label_width,
                );
                info_row(
                    ui,
                    locale_text(ui_language, LocaleKey::Source),
                    &source_display,
                    detail_chrome.detail_label_width,
                );
                info_row(
                    ui,
                    locale_text(ui_language, LocaleKey::Version),
                    locale_text(ui_language, LocaleKey::ProviderNotDetected),
                    detail_chrome.detail_label_width,
                );
                info_row(
                    ui,
                    locale_text(ui_language, LocaleKey::Updated),
                    locale_text(ui_language, LocaleKey::ProviderNotFetchedYetTitle),
                    detail_chrome.detail_label_width,
                );
                info_row(
                    ui,
                    locale_text(ui_language, LocaleKey::Status),
                    &detail_status,
                    detail_chrome.detail_label_width,
                );
                if provider_id != ProviderId::Cursor {
                    info_row(
                        ui,
                        locale_text(ui_language, LocaleKey::ProviderUsage),
                        locale_text(ui_language, LocaleKey::ProviderDisabledNoRecentData),
                        detail_chrome.detail_label_width,
                    );
                }
            });

        if provider_id == ProviderId::Cursor {
            provider_detail_section_title(ui, locale_text(ui_language, LocaleKey::ProviderUsage));
        }

        ui.add_space(detail_chrome.section_gap);
    } else {
        let version_display = locale_text(ui_language, LocaleKey::ProviderNotDetected);
        egui::Grid::new("provider_info_grid")
            .num_columns(2)
            .spacing([
                detail_chrome.info_grid_spacing_x,
                detail_chrome.info_grid_spacing_y,
            ])
            .show(ui, |ui| {
                info_row(
                    ui,
                    locale_text(ui_language, LocaleKey::State),
                    locale_text(ui_language, LocaleKey::ProviderEnabled),
                    detail_chrome.detail_label_width,
                );
                info_row(
                    ui,
                    locale_text(ui_language, LocaleKey::Source),
                    &source_display,
                    detail_chrome.detail_label_width,
                );
                info_row(
                    ui,
                    locale_text(ui_language, LocaleKey::Version),
                    version_display,
                    detail_chrome.detail_label_width,
                );
                info_row(
                    ui,
                    locale_text(ui_language, LocaleKey::Updated),
                    &updated_display,
                    detail_chrome.detail_label_width,
                );
                info_row(
                    ui,
                    locale_text(ui_language, LocaleKey::Status),
                    &detail_status,
                    detail_chrome.detail_label_width,
                );
            });

        provider_detail_section_title(ui, locale_text(ui_language, LocaleKey::ProviderUsage));
    }

    // Helper to format reset time
    let format_reset = |rate: &crate::core::RateWindow| -> Option<String> {
        // First try to compute from resets_at timestamp
        if let Some(ts) = rate.resets_at {
            let now = chrono::Utc::now();
            let diff = ts - now;
            if diff.num_seconds() <= 0 {
                return Some(locale_text(ui_language, LocaleKey::ResetInProgress).to_string());
            } else if diff.num_hours() >= 24 {
                return Some(
                    locale_text(ui_language, LocaleKey::ResetsInDaysHours)
                        .replace("{}", &diff.num_days().to_string())
                        .replace("{}", &(diff.num_hours() % 24).to_string()),
                );
            } else {
                return Some(
                    locale_text(ui_language, LocaleKey::ResetsInHoursMinutes)
                        .replace("{}", &diff.num_hours().to_string())
                        .replace("{}", &(diff.num_minutes() % 60).to_string()),
                );
            }
        }
        // Fall back to reset_description if available (for CLI/web sources without parsed timestamp)
        rate.reset_description.clone()
    };

    let show_as_used = if let Ok(state) = shared_state.lock() {
        state.settings.show_as_used
    } else {
        true
    };

    // Session usage bar (primary rate)
    if is_enabled && let Some(ref rate) = primary_rate {
        let (percent, label) = usage_display(rate.used_percent, show_as_used, ui_language);
        let reset_str = format_reset(rate);
        usage_bar_row(
            ui,
            locale_text(ui_language, LocaleKey::ProviderSessionLabel),
            percent as f32,
            &label,
            reset_str.as_deref(),
            brand_color,
            detail_chrome.detail_label_width,
            detail_chrome.metric_bar_width,
        );
        ui.add_space(4.0);
    }

    // Weekly usage bar (secondary rate)
    if is_enabled && let Some(ref rate) = secondary_rate {
        let (percent, label) = usage_display(rate.used_percent, show_as_used, ui_language);
        let reset_str = format_reset(rate);
        usage_bar_row(
            ui,
            locale_text(ui_language, LocaleKey::ProviderWeeklyLabel),
            percent as f32,
            &label,
            reset_str.as_deref(),
            brand_color,
            detail_chrome.detail_label_width,
            detail_chrome.metric_bar_width,
        );
        ui.add_space(4.0);
    }

    // Tertiary rate (e.g., code review)
    if is_enabled && let Some(ref rate) = tertiary_rate {
        let (percent, label) = usage_display(rate.used_percent, show_as_used, ui_language);
        let reset_str = rate.reset_description.as_deref();
        usage_bar_row(
            ui,
            locale_text(ui_language, LocaleKey::ProviderCodeReviewLabel),
            percent as f32,
            &label,
            reset_str,
            brand_color,
            detail_chrome.detail_label_width,
            detail_chrome.metric_bar_width,
        );
        ui.add_space(4.0);
    }

    // Code review (if available and no tertiary rate)
    // Note: code_review_remaining_percent is the REMAINING percent, so convert to used
    match (is_enabled, code_review_percent) {
        (true, Some(remaining)) if tertiary_rate.is_none() => {
            let used = 100.0 - remaining;
            let (percent, label) = usage_display(used, show_as_used, ui_language);
            usage_bar_row(
                ui,
                locale_text(ui_language, LocaleKey::ProviderCodeReviewLabel),
                percent as f32,
                &label,
                None,
                brand_color,
                detail_chrome.detail_label_width,
                detail_chrome.metric_bar_width,
            );
        }
        _ => {}
    }

    let codex_missing_usage_details = provider_id == ProviderId::Codex
        && entry
            .as_ref()
            .map(|entry| {
                !provider_sidebar_has_usage(entry)
                    && entry.account_email.is_none()
                    && entry.login_method.is_none()
            })
            .unwrap_or(true);

    if provider_id == ProviderId::Codex && credits_remaining.is_none() {
        ui.add_space(2.0);
        provider_detail_text_row(
            ui,
            locale_text(ui_language, LocaleKey::CreditsLabel),
            locale_text(ui_language, LocaleKey::ProviderCodexCreditsUnavailable),
            detail_chrome.detail_label_width,
            true,
        );
    }

    if provider_id == ProviderId::Cursor {
        ui.add_space(2.0);
        provider_detail_text_row(
            ui,
            locale_text(ui_language, LocaleKey::CreditsLabel),
            locale_text(ui_language, LocaleKey::ProviderCursorCreditsHelp),
            detail_chrome.detail_label_width,
            true,
        );
    }

    if codex_missing_usage_details {
        ui.add_space(Spacing::XS);
        provider_detail_section_title(
            ui,
            locale_text(ui_language, LocaleKey::ProviderCodexLastFetchFailedTitle),
        );
        provider_detail_helper_text(
            ui,
            locale_text(ui_language, LocaleKey::ProviderCodexNotRunningHelp),
        );
        ui.add_space(detail_chrome.section_gap);
    }

    if shows_shared_provider_settings(provider_id) {
        provider_detail_section_title(
            ui,
            locale_text(ui_language, LocaleKey::ProviderSettingsTitle),
        );

        if let Some(credits) = credits_remaining {
            ui.add_space(4.0);
            provider_detail_text_row(
                ui,
                locale_text(ui_language, LocaleKey::CreditsLabel),
                &locale_text(ui_language, LocaleKey::CreditsLeft)
                    .replace("{:.1}", &format!("{:.1}", credits)),
                detail_chrome.detail_label_width,
                true,
            );
            ui.add_space(Spacing::XS);
        }

        let (today_cost, today_tokens, monthly_cost, monthly_tokens) =
            if let Some(ref usage) = token_usage {
                (
                    usage.session_cost_usd.unwrap_or(0.0),
                    usage.session_tokens.unwrap_or(0),
                    usage.last_30_days_cost_usd.unwrap_or(0.0),
                    usage.last_30_days_tokens.unwrap_or(0),
                )
            } else {
                (0.0, 0, 0.0, 0)
            };

        let show_cost_block = (!matches!(
            provider_id,
            ProviderId::Codex | ProviderId::Cursor | ProviderId::Claude
        )) || token_usage.is_some();
        if show_cost_block {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [detail_chrome.detail_label_width, 20.0],
                    egui::Label::new(
                        RichText::new(locale_text(ui_language, LocaleKey::CostTitle))
                            .size(FontSize::XS)
                            .color(text_chrome.info_label),
                    ),
                );
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(
                            locale_text(ui_language, LocaleKey::TodayCostFull)
                                .replace("{:.2}", &format!("{:.2}", today_cost))
                                .replace("{}", &today_tokens.to_string()),
                        )
                        .size(FontSize::XS)
                        .color(Theme::TEXT_PRIMARY.gamma_multiply(0.98)),
                    );
                    ui.label(
                        RichText::new(
                            locale_text(ui_language, LocaleKey::Last30DaysCostFull)
                                .replace("{:.2}", &format!("{:.2}", monthly_cost))
                                .replace("{}", &monthly_tokens.to_string()),
                        )
                        .size(FontSize::XS)
                        .color(text_chrome.secondary_value),
                    );
                });
            });
        }

        provider_detail_picker_row(
            ui,
            locale_text(ui_language, LocaleKey::MenuBarMetric),
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    egui::ComboBox::from_id_salt(format!("metric_{}", provider_id.cli_name()))
                        .selected_text(locale_text(ui_language, LocaleKey::Automatic))
                        .width(116.0)
                        .show_ui(ui, |ui| {
                            let _ = ui.selectable_label(
                                true,
                                locale_text(ui_language, LocaleKey::Automatic),
                            );
                            let _ = ui.selectable_label(
                                false,
                                locale_text(ui_language, LocaleKey::ProviderSessionLabel),
                            );
                            let _ = ui.selectable_label(
                                false,
                                locale_text(ui_language, LocaleKey::ProviderWeeklyLabel),
                            );
                        });
                });
            },
        );

        ui.add_space(3.0);
        provider_detail_helper_text(ui, locale_text(ui_language, LocaleKey::MenuBarMetricHelper));

        ui.add_space(detail_chrome.section_gap);
    }

    if provider_id == ProviderId::Codex {
        let mut codex_usage_source = if let Ok(state) = shared_state.lock() {
            state.settings.codex_usage_source.clone()
        } else {
            "auto".to_string()
        };
        provider_detail_picker_row(
            ui,
            locale_text(ui_language, LocaleKey::UsageSource),
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    egui::ComboBox::from_id_salt("codex_usage_source")
                        .selected_text(codex_usage_source_label(&codex_usage_source, ui_language))
                        .width(116.0)
                        .show_ui(ui, |ui| {
                            let _ = ui.selectable_value(
                                &mut codex_usage_source,
                                "auto".to_string(),
                                locale_text(ui_language, LocaleKey::Automatic),
                            );
                            let _ = ui.selectable_value(
                                &mut codex_usage_source,
                                "oauth".to_string(),
                                locale_text(ui_language, LocaleKey::OAuth),
                            );
                            let _ = ui.selectable_value(
                                &mut codex_usage_source,
                                "cli".to_string(),
                                locale_text(ui_language, LocaleKey::ProviderSourceCliShort),
                            );
                        });
                });
            },
        );
        if let Ok(mut state) = shared_state.lock()
            && state.settings.codex_usage_source != codex_usage_source
        {
            state.settings.codex_usage_source = codex_usage_source;
            state.settings_changed = true;
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, locale_text(ui_language, LocaleKey::AutoFallbackHelp));
        ui.add_space(detail_chrome.section_gap);

        let codex_openai_web_extras = if let Ok(state) = shared_state.lock() {
            state.settings.codex_openai_web_extras
        } else {
            true
        };

        if codex_openai_web_extras {
            let mut codex_cookie_source = if let Ok(state) = shared_state.lock() {
                state.settings.codex_cookie_source.clone()
            } else {
                "auto".to_string()
            };
            provider_detail_picker_row(
                ui,
                locale_text(ui_language, LocaleKey::ProviderOpenAiCookies),
                detail_chrome.picker_label_width,
                text_chrome.info_label,
                |ui| {
                    provider_detail_select_frame(ui, detail_chrome, |ui| {
                        egui::ComboBox::from_id_salt("codex_cookie_source")
                            .selected_text(codex_cookie_source_label(
                                &codex_cookie_source,
                                ui_language,
                            ))
                            .width(116.0)
                            .show_ui(ui, |ui| {
                                let _ = ui.selectable_value(
                                    &mut codex_cookie_source,
                                    "auto".to_string(),
                                    locale_text(ui_language, LocaleKey::Automatic),
                                );
                                let _ = ui.selectable_value(
                                    &mut codex_cookie_source,
                                    "manual".to_string(),
                                    "Manual",
                                );
                                let _ = ui.selectable_value(
                                    &mut codex_cookie_source,
                                    "off".to_string(),
                                    locale_text(ui_language, LocaleKey::ProviderDisabled),
                                );
                            });
                    });
                },
            );
            if let Ok(mut state) = shared_state.lock()
                && state.settings.codex_cookie_source != codex_cookie_source
            {
                state.settings.codex_cookie_source = codex_cookie_source.clone();
                state.settings_changed = true;
            }

            ui.add_space(2.0);
            let cookie_help = match codex_cookie_source.as_str() {
                "manual" => "Paste a Cookie header from a chatgpt.com request.",
                "off" => "Disable OpenAI dashboard cookie usage.",
                _ => locale_text(ui_language, LocaleKey::ProviderCodexAutoImportHelp),
            };
            provider_detail_helper_text(ui, cookie_help);
            ui.add_space(detail_chrome.section_gap);
        }
    } else if provider_id == ProviderId::Cursor {
        let mut cursor_cookie_source = if let Ok(state) = shared_state.lock() {
            state.settings.cursor_cookie_source.clone()
        } else {
            "auto".to_string()
        };
        provider_detail_picker_row(
            ui,
            locale_text(ui_language, LocaleKey::ProviderCookieSource),
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    egui::ComboBox::from_id_salt(format!(
                        "cookie_source_{}",
                        provider_id.cli_name()
                    ))
                    .selected_text(cursor_cookie_source_label(
                        &cursor_cookie_source,
                        ui_language,
                    ))
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        let _ = ui.selectable_value(
                            &mut cursor_cookie_source,
                            "auto".to_string(),
                            locale_text(ui_language, LocaleKey::Automatic),
                        );
                        let _ = ui.selectable_value(
                            &mut cursor_cookie_source,
                            "manual".to_string(),
                            "Manual",
                        );
                    });
                });
            },
        );
        if let Ok(mut state) = shared_state.lock()
            && state.settings.cursor_cookie_source != cursor_cookie_source
        {
            state.settings.cursor_cookie_source = cursor_cookie_source.clone();
            state.settings_changed = true;
        }

        ui.add_space(2.0);
        provider_detail_helper_text(
            ui,
            &cursor_cookie_source_help(&cursor_cookie_source, ui_language),
        );
        ui.add_space(detail_chrome.section_gap);
    } else if provider_id == ProviderId::OpenCode {
        let mut opencode_cookie_source = if let Ok(state) = shared_state.lock() {
            state.settings.opencode_cookie_source.clone()
        } else {
            "auto".to_string()
        };
        provider_detail_picker_row(
            ui,
            locale_text(ui_language, LocaleKey::ProviderCookieSource),
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    egui::ComboBox::from_id_salt(format!(
                        "cookie_source_{}",
                        provider_id.cli_name()
                    ))
                    .selected_text(opencode_cookie_source_label(
                        &opencode_cookie_source,
                        ui_language,
                    ))
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        let _ = ui.selectable_value(
                            &mut opencode_cookie_source,
                            "auto".to_string(),
                            locale_text(ui_language, LocaleKey::Automatic),
                        );
                        let _ = ui.selectable_value(
                            &mut opencode_cookie_source,
                            "manual".to_string(),
                            "Manual",
                        );
                    });
                });
            },
        );
        if let Ok(mut state) = shared_state.lock()
            && state.settings.opencode_cookie_source != opencode_cookie_source
        {
            state.settings.opencode_cookie_source = opencode_cookie_source.clone();
            state.settings_changed = true;
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, &opencode_cookie_source_help(&opencode_cookie_source));
        ui.add_space(detail_chrome.section_gap);

        let mut workspace_id = if let Ok(state) = shared_state.lock() {
            state.settings.opencode_workspace_id.clone()
        } else {
            String::new()
        };

        provider_detail_picker_row(
            ui,
            "Workspace ID",
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    let text_edit = egui::TextEdit::singleline(&mut workspace_id)
                        .desired_width(168.0)
                        .hint_text("wrk_...");
                    let _ = ui.add(text_edit);
                });
            },
        );
        if let Ok(mut state) = shared_state.lock()
            && state.settings.opencode_workspace_id != workspace_id
        {
            state.settings.opencode_workspace_id = workspace_id;
            state.settings_changed = true;
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, "Optional override if workspace lookup fails.");
        ui.add_space(detail_chrome.section_gap);
    } else if provider_id == ProviderId::MiniMax {
        let mut minimax_cookie_source = if let Ok(state) = shared_state.lock() {
            state.settings.minimax_cookie_source.clone()
        } else {
            "auto".to_string()
        };
        let mut minimax_api_region = if let Ok(state) = shared_state.lock() {
            state.settings.minimax_api_region.clone()
        } else {
            "global".to_string()
        };
        let mut minimax_api_token = if let Ok(state) = shared_state.lock() {
            state.settings.minimax_api_token.clone()
        } else {
            String::new()
        };
        let mut minimax_cookie_header = if let Ok(state) = shared_state.lock() {
            state.settings.minimax_cookie_header.clone()
        } else {
            String::new()
        };

        provider_detail_picker_row(
            ui,
            locale_text(ui_language, LocaleKey::ProviderCookieSource),
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    egui::ComboBox::from_id_salt(format!(
                        "cookie_source_{}",
                        provider_id.cli_name()
                    ))
                    .selected_text(minimax_cookie_source_label(
                        &minimax_cookie_source,
                        ui_language,
                    ))
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        let _ = ui.selectable_value(
                            &mut minimax_cookie_source,
                            "auto".to_string(),
                            locale_text(ui_language, LocaleKey::Automatic),
                        );
                        let _ = ui.selectable_value(
                            &mut minimax_cookie_source,
                            "manual".to_string(),
                            "Manual",
                        );
                    });
                });
            },
        );
        if let Ok(mut state) = shared_state.lock()
            && state.settings.minimax_cookie_source != minimax_cookie_source
        {
            state.settings.minimax_cookie_source = minimax_cookie_source.clone();
            state.settings_changed = true;
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, &minimax_cookie_source_help(&minimax_cookie_source));
        ui.add_space(detail_chrome.section_gap);

        provider_detail_picker_row(
            ui,
            "API region",
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    egui::ComboBox::from_id_salt("minimax_api_region")
                        .selected_text(minimax_region_label(&minimax_api_region))
                        .width(160.0)
                        .show_ui(ui, |ui| {
                            let _ = ui.selectable_value(
                                &mut minimax_api_region,
                                "global".to_string(),
                                "Global (.io)",
                            );
                            let _ = ui.selectable_value(
                                &mut minimax_api_region,
                                "china".to_string(),
                                "China Mainland (.com)",
                            );
                        });
                });
            },
        );
        if let Ok(mut state) = shared_state.lock()
            && state.settings.minimax_api_region != minimax_api_region
        {
            state.settings.minimax_api_region = minimax_api_region.clone();
            state.settings_changed = true;
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, "Use global (.io) or China mainland (.com).");
        ui.add_space(detail_chrome.section_gap);

        provider_detail_picker_row(
            ui,
            "API token",
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    let text_edit = egui::TextEdit::singleline(&mut minimax_api_token)
                        .password(true)
                        .desired_width(224.0)
                        .hint_text("Paste API token...");
                    let _ = ui.add(text_edit);
                });
            },
        );
        if let Ok(mut state) = shared_state.lock()
            && state.settings.minimax_api_token != minimax_api_token
        {
            state.settings.minimax_api_token = minimax_api_token.clone();
            state.settings_changed = true;
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, "Stored in config.json. Paste a MiniMax key.");
        ui.add_space(6.0);
        if text_button(ui, "Open Coding Plan", Theme::ACCENT_PRIMARY) {
            let url = if minimax_api_region == "china" {
                "https://platform.minimaxi.com/user-center"
            } else {
                "https://platform.minimax.io/user-center"
            };
            let _ = open::that(url);
        }
        ui.add_space(detail_chrome.section_gap);

        if minimax_cookie_source == "manual" {
            provider_detail_picker_row(
                ui,
                "Cookie header",
                detail_chrome.picker_label_width,
                text_chrome.info_label,
                |ui| {
                    provider_detail_select_frame(ui, detail_chrome, |ui| {
                        let text_edit = egui::TextEdit::singleline(&mut minimax_cookie_header)
                            .password(true)
                            .desired_width(224.0)
                            .hint_text("Cookie: ...");
                        let _ = ui.add(text_edit);
                    });
                },
            );
            if let Ok(mut state) = shared_state.lock()
                && state.settings.minimax_cookie_header != minimax_cookie_header
            {
                state.settings.minimax_cookie_header = minimax_cookie_header;
                state.settings_changed = true;
            }

            ui.add_space(6.0);
            if text_button(ui, "Open Coding Plan", Theme::ACCENT_PRIMARY) {
                let url = if minimax_api_region == "china" {
                    "https://platform.minimaxi.com/user-center"
                } else {
                    "https://platform.minimax.io/user-center"
                };
                let _ = open::that(url);
            }
            ui.add_space(detail_chrome.section_gap);
        }
    } else if provider_id == ProviderId::Factory {
        let mut factory_cookie_source = if let Ok(state) = shared_state.lock() {
            state.settings.factory_cookie_source.clone()
        } else {
            "auto".to_string()
        };
        provider_detail_picker_row(
            ui,
            locale_text(ui_language, LocaleKey::ProviderCookieSource),
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    egui::ComboBox::from_id_salt(format!(
                        "cookie_source_{}",
                        provider_id.cli_name()
                    ))
                    .selected_text(factory_cookie_source_label(
                        &factory_cookie_source,
                        ui_language,
                    ))
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        let _ = ui.selectable_value(
                            &mut factory_cookie_source,
                            "auto".to_string(),
                            locale_text(ui_language, LocaleKey::Automatic),
                        );
                        let _ = ui.selectable_value(
                            &mut factory_cookie_source,
                            "manual".to_string(),
                            "Manual",
                        );
                    });
                });
            },
        );
        if let Ok(mut state) = shared_state.lock()
            && state.settings.factory_cookie_source != factory_cookie_source
        {
            state.settings.factory_cookie_source = factory_cookie_source.clone();
            state.settings_changed = true;
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, &factory_cookie_source_help(&factory_cookie_source));
        ui.add_space(detail_chrome.section_gap);
    } else if provider_id == ProviderId::Alibaba {
        let mut alibaba_cookie_source = if let Ok(state) = shared_state.lock() {
            state.settings.alibaba_cookie_source.clone()
        } else {
            "auto".to_string()
        };
        let mut alibaba_api_region = if let Ok(state) = shared_state.lock() {
            state.settings.alibaba_api_region.clone()
        } else {
            "intl".to_string()
        };
        let mut alibaba_api_key = if let Ok(state) = shared_state.lock() {
            state
                .api_keys
                .get("alibaba")
                .unwrap_or_default()
                .to_string()
        } else {
            String::new()
        };
        let mut alibaba_cookie_header = if let Ok(state) = shared_state.lock() {
            state.settings.alibaba_cookie_header.clone()
        } else {
            String::new()
        };

        provider_detail_picker_row(
            ui,
            locale_text(ui_language, LocaleKey::ProviderCookieSource),
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    egui::ComboBox::from_id_salt(format!(
                        "cookie_source_{}",
                        provider_id.cli_name()
                    ))
                    .selected_text(alibaba_cookie_source_label(
                        &alibaba_cookie_source,
                        ui_language,
                    ))
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        let _ = ui.selectable_value(
                            &mut alibaba_cookie_source,
                            "auto".to_string(),
                            locale_text(ui_language, LocaleKey::Automatic),
                        );
                        let _ = ui.selectable_value(
                            &mut alibaba_cookie_source,
                            "manual".to_string(),
                            "Manual",
                        );
                    });
                });
            },
        );
        if let Ok(mut state) = shared_state.lock()
            && state.settings.alibaba_cookie_source != alibaba_cookie_source
        {
            state.settings.alibaba_cookie_source = alibaba_cookie_source.clone();
            state.settings_changed = true;
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, &alibaba_cookie_source_help(&alibaba_cookie_source));
        ui.add_space(detail_chrome.section_gap);

        provider_detail_picker_row(
            ui,
            "Gateway region",
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    egui::ComboBox::from_id_salt("alibaba_api_region")
                        .selected_text(alibaba_region_label(&alibaba_api_region))
                        .width(220.0)
                        .show_ui(ui, |ui| {
                            let _ = ui.selectable_value(
                                &mut alibaba_api_region,
                                "intl".to_string(),
                                "International (Model Studio)",
                            );
                            let _ = ui.selectable_value(
                                &mut alibaba_api_region,
                                "cn".to_string(),
                                "China Mainland (Bailian)",
                            );
                        });
                });
            },
        );
        if let Ok(mut state) = shared_state.lock()
            && state.settings.alibaba_api_region != alibaba_api_region
        {
            state.settings.alibaba_api_region = alibaba_api_region.clone();
            state.settings_changed = true;
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, "Use the international or China mainland gateway.");
        ui.add_space(detail_chrome.section_gap);

        provider_detail_picker_row(
            ui,
            "API key",
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    let text_edit = egui::TextEdit::singleline(&mut alibaba_api_key)
                        .password(true)
                        .desired_width(224.0)
                        .hint_text("cpk-...");
                    let _ = ui.add(text_edit);
                });
            },
        );
        if let Ok(mut state) = shared_state.lock() {
            let current = state
                .api_keys
                .get("alibaba")
                .unwrap_or_default()
                .to_string();
            if current != alibaba_api_key {
                if alibaba_api_key.trim().is_empty() {
                    state.api_keys.remove("alibaba");
                } else {
                    state.api_keys.set("alibaba", &alibaba_api_key, None);
                }
                state.settings_changed = true;
            }
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, "Stored in CodexBar. Paste a Coding Plan key.");
        ui.add_space(6.0);
        if text_button(ui, "Open Coding Plan", Theme::ACCENT_PRIMARY) {
            let _ = open::that(alibaba_dashboard_url(&alibaba_api_region));
        }
        ui.add_space(detail_chrome.section_gap);

        if alibaba_cookie_source == "manual" {
            provider_detail_picker_row(
                ui,
                "Cookie header",
                detail_chrome.picker_label_width,
                text_chrome.info_label,
                |ui| {
                    provider_detail_select_frame(ui, detail_chrome, |ui| {
                        let text_edit = egui::TextEdit::singleline(&mut alibaba_cookie_header)
                            .password(true)
                            .desired_width(224.0)
                            .hint_text("Cookie: ...");
                        let _ = ui.add(text_edit);
                    });
                },
            );
            if let Ok(mut state) = shared_state.lock()
                && state.settings.alibaba_cookie_header != alibaba_cookie_header
            {
                state.settings.alibaba_cookie_header = alibaba_cookie_header;
                state.settings_changed = true;
            }

            ui.add_space(6.0);
            if text_button(ui, "Open Coding Plan", Theme::ACCENT_PRIMARY) {
                let _ = open::that(alibaba_dashboard_url(&alibaba_api_region));
            }
            ui.add_space(detail_chrome.section_gap);
        }
    } else if provider_id == ProviderId::Kimi {
        let mut kimi_cookie_source = if let Ok(state) = shared_state.lock() {
            state.settings.kimi_cookie_source.clone()
        } else {
            "auto".to_string()
        };
        provider_detail_picker_row(
            ui,
            locale_text(ui_language, LocaleKey::ProviderCookieSource),
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    egui::ComboBox::from_id_salt(format!(
                        "cookie_source_{}",
                        provider_id.cli_name()
                    ))
                    .selected_text(kimi_cookie_source_label(&kimi_cookie_source, ui_language))
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        let _ = ui.selectable_value(
                            &mut kimi_cookie_source,
                            "auto".to_string(),
                            locale_text(ui_language, LocaleKey::Automatic),
                        );
                        let _ = ui.selectable_value(
                            &mut kimi_cookie_source,
                            "manual".to_string(),
                            "Manual",
                        );
                        let _ = ui.selectable_value(
                            &mut kimi_cookie_source,
                            "off".to_string(),
                            locale_text(ui_language, LocaleKey::ProviderDisabled),
                        );
                    });
                });
            },
        );
        if let Ok(mut state) = shared_state.lock()
            && state.settings.kimi_cookie_source != kimi_cookie_source
        {
            state.settings.kimi_cookie_source = kimi_cookie_source.clone();
            state.settings_changed = true;
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, &kimi_cookie_source_help(&kimi_cookie_source));
        ui.add_space(detail_chrome.section_gap);

        if kimi_cookie_source == "manual" {
            let mut manual_cookie_header = if let Ok(state) = shared_state.lock() {
                state.settings.kimi_manual_cookie_header.clone()
            } else {
                String::new()
            };

            provider_detail_picker_row(
                ui,
                "",
                detail_chrome.picker_label_width,
                text_chrome.info_label,
                |ui| {
                    provider_detail_select_frame(ui, detail_chrome, |ui| {
                        let text_edit = egui::TextEdit::singleline(&mut manual_cookie_header)
                            .password(true)
                            .desired_width(224.0)
                            .hint_text("Cookie: ... or paste the kimi-auth token value");
                        let _ = ui.add(text_edit);
                    });
                },
            );
            if let Ok(mut state) = shared_state.lock()
                && state.settings.kimi_manual_cookie_header != manual_cookie_header
            {
                state.settings.kimi_manual_cookie_header = manual_cookie_header;
                state.settings_changed = true;
            }

            ui.add_space(6.0);
            if text_button(ui, "Open Console", Theme::ACCENT_PRIMARY) {
                let _ = open::that("https://www.kimi.com/code/console");
            }
            ui.add_space(detail_chrome.section_gap);
        }
    } else if provider_id == ProviderId::Gemini {
        let has_gemini_creds = gemini_cli_signed_in();
        let creds_path = gemini_cli_credentials_path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "~/.gemini/oauth_creds.json".to_string());
        let compact_creds_path = compact_credentials_path(&creds_path);

        provider_detail_text_row(
            ui,
            "Gemini CLI",
            if has_gemini_creds {
                "Authenticated"
            } else {
                "Not signed in"
            },
            detail_chrome.picker_label_width,
            true,
        );
        ui.add_space(2.0);
        provider_detail_helper_text(
            ui,
            &format!("Uses OAuth credentials from {}.", compact_creds_path),
        );
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            let button_label = if has_gemini_creds {
                "Open Google AI Studio"
            } else {
                "Setup Gemini CLI"
            };
            if primary_button(ui, button_label) {
                let target = if has_gemini_creds {
                    "https://aistudio.google.com/"
                } else {
                    "https://github.com/google-gemini/gemini-cli"
                };
                let _ = open::that(target);
            }
        });
        ui.add_space(detail_chrome.section_gap);
    } else if provider_id == ProviderId::VertexAI {
        let has_vertexai_creds = vertexai_signed_in();
        let creds_path = vertexai_credentials_path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "%APPDATA%/gcloud/application_default_credentials.json".to_string());
        let compact_creds_path = compact_credentials_path(&creds_path);

        provider_detail_text_row(
            ui,
            "Google Cloud",
            if has_vertexai_creds {
                "Authenticated"
            } else {
                "Not signed in"
            },
            detail_chrome.picker_label_width,
            true,
        );
        ui.add_space(2.0);
        provider_detail_helper_text(
            ui,
            &format!("Uses Google Cloud credentials from {}.", compact_creds_path),
        );
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            let button_label = if has_vertexai_creds {
                "Open Vertex AI Console"
            } else {
                "Setup Google Cloud Auth"
            };
            if primary_button(ui, button_label) {
                let target = if has_vertexai_creds {
                    "https://console.cloud.google.com/vertex-ai"
                } else {
                    "https://cloud.google.com/sdk/gcloud/reference/auth/application-default/login"
                };
                let _ = open::that(target);
            }
        });
        ui.add_space(detail_chrome.section_gap);
    } else if provider_id == ProviderId::Antigravity {
        provider_detail_text_row(
            ui,
            "Antigravity App",
            "Managed in app",
            detail_chrome.picker_label_width,
            true,
        );
        ui.add_space(2.0);
        provider_detail_helper_text(ui, "Open Antigravity to sign in, then refresh CodexBar.");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if primary_button(ui, "Refresh after sign-in")
                && let Ok(mut state) = shared_state.lock()
            {
                state.refresh_requested = true;
            }
        });
        ui.add_space(detail_chrome.section_gap);
    } else if provider_id == ProviderId::Augment {
        let mut augment_cookie_source = if let Ok(state) = shared_state.lock() {
            state.settings.augment_cookie_source.clone()
        } else {
            "auto".to_string()
        };
        let mut augment_cookie_header = if let Ok(state) = shared_state.lock() {
            state.settings.augment_cookie_header.clone()
        } else {
            String::new()
        };

        provider_detail_picker_row(
            ui,
            locale_text(ui_language, LocaleKey::ProviderCookieSource),
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    egui::ComboBox::from_id_salt(format!(
                        "cookie_source_{}",
                        provider_id.cli_name()
                    ))
                    .selected_text(augment_cookie_source_label(
                        &augment_cookie_source,
                        ui_language,
                    ))
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        let _ = ui.selectable_value(
                            &mut augment_cookie_source,
                            "auto".to_string(),
                            locale_text(ui_language, LocaleKey::Automatic),
                        );
                        let _ = ui.selectable_value(
                            &mut augment_cookie_source,
                            "manual".to_string(),
                            "Manual",
                        );
                    });
                });
            },
        );
        if let Ok(mut state) = shared_state.lock()
            && state.settings.augment_cookie_source != augment_cookie_source
        {
            state.settings.augment_cookie_source = augment_cookie_source.clone();
            state.settings_changed = true;
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, &augment_cookie_source_help(&augment_cookie_source));
        ui.add_space(8.0);

        if primary_button(ui, "Refresh Session")
            && let Ok(mut state) = shared_state.lock()
        {
            state.refresh_requested = true;
        }

        if augment_cookie_source == "manual" {
            ui.add_space(detail_chrome.section_gap);
            provider_detail_picker_row(
                ui,
                "Cookie header",
                detail_chrome.picker_label_width,
                text_chrome.info_label,
                |ui| {
                    provider_detail_select_frame(ui, detail_chrome, |ui| {
                        let text_edit = egui::TextEdit::singleline(&mut augment_cookie_header)
                            .password(true)
                            .desired_width(224.0)
                            .hint_text("Cookie: ...");
                        let _ = ui.add(text_edit);
                    });
                },
            );
            if let Ok(mut state) = shared_state.lock()
                && state.settings.augment_cookie_header != augment_cookie_header
            {
                state.settings.augment_cookie_header = augment_cookie_header;
                state.settings_changed = true;
            }

            ui.add_space(2.0);
            provider_detail_helper_text(
                ui,
                "Paste a Cookie header or cURL capture from the Augment dashboard.",
            );
            ui.add_space(8.0);
            if text_button(ui, "Open Augment", Theme::ACCENT_PRIMARY) {
                let _ = open::that("https://app.augmentcode.com");
            }
        }
        ui.add_space(detail_chrome.section_gap);
    } else if provider_id == ProviderId::Amp {
        let mut amp_cookie_source = if let Ok(state) = shared_state.lock() {
            state.settings.amp_cookie_source.clone()
        } else {
            "auto".to_string()
        };
        let mut amp_cookie_header = if let Ok(state) = shared_state.lock() {
            state.settings.amp_cookie_header.clone()
        } else {
            String::new()
        };

        provider_detail_picker_row(
            ui,
            locale_text(ui_language, LocaleKey::ProviderCookieSource),
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    egui::ComboBox::from_id_salt(format!(
                        "cookie_source_{}",
                        provider_id.cli_name()
                    ))
                    .selected_text(amp_cookie_source_label(&amp_cookie_source, ui_language))
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        let _ = ui.selectable_value(
                            &mut amp_cookie_source,
                            "auto".to_string(),
                            locale_text(ui_language, LocaleKey::Automatic),
                        );
                        let _ = ui.selectable_value(
                            &mut amp_cookie_source,
                            "manual".to_string(),
                            "Manual",
                        );
                    });
                });
            },
        );
        if let Ok(mut state) = shared_state.lock()
            && state.settings.amp_cookie_source != amp_cookie_source
        {
            state.settings.amp_cookie_source = amp_cookie_source.clone();
            state.settings_changed = true;
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, &amp_cookie_source_help(&amp_cookie_source));
        ui.add_space(detail_chrome.section_gap);

        if amp_cookie_source == "manual" {
            provider_detail_picker_row(
                ui,
                "",
                detail_chrome.picker_label_width,
                text_chrome.info_label,
                |ui| {
                    provider_detail_select_frame(ui, detail_chrome, |ui| {
                        let text_edit = egui::TextEdit::singleline(&mut amp_cookie_header)
                            .password(true)
                            .desired_width(224.0)
                            .hint_text("Cookie: ...");
                        let _ = ui.add(text_edit);
                    });
                },
            );
            if let Ok(mut state) = shared_state.lock()
                && state.settings.amp_cookie_header != amp_cookie_header
            {
                state.settings.amp_cookie_header = amp_cookie_header;
                state.settings_changed = true;
            }

            ui.add_space(6.0);
            if text_button(ui, "Open Amp Settings", Theme::ACCENT_PRIMARY) {
                let _ = open::that("https://ampcode.com/settings");
            }
            ui.add_space(detail_chrome.section_gap);
        }
    } else if provider_id == ProviderId::Ollama {
        let mut ollama_cookie_source = if let Ok(state) = shared_state.lock() {
            state.settings.ollama_cookie_source.clone()
        } else {
            "auto".to_string()
        };
        let mut ollama_cookie_header = if let Ok(state) = shared_state.lock() {
            state.settings.ollama_cookie_header.clone()
        } else {
            String::new()
        };

        provider_detail_picker_row(
            ui,
            locale_text(ui_language, LocaleKey::ProviderCookieSource),
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    egui::ComboBox::from_id_salt(format!(
                        "cookie_source_{}",
                        provider_id.cli_name()
                    ))
                    .selected_text(ollama_cookie_source_label(
                        &ollama_cookie_source,
                        ui_language,
                    ))
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        let _ = ui.selectable_value(
                            &mut ollama_cookie_source,
                            "auto".to_string(),
                            locale_text(ui_language, LocaleKey::Automatic),
                        );
                        let _ = ui.selectable_value(
                            &mut ollama_cookie_source,
                            "manual".to_string(),
                            "Manual",
                        );
                    });
                });
            },
        );
        if let Ok(mut state) = shared_state.lock()
            && state.settings.ollama_cookie_source != ollama_cookie_source
        {
            state.settings.ollama_cookie_source = ollama_cookie_source.clone();
            state.settings_changed = true;
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, &ollama_cookie_source_help(&ollama_cookie_source));
        ui.add_space(detail_chrome.section_gap);

        if ollama_cookie_source == "manual" {
            provider_detail_picker_row(
                ui,
                "",
                detail_chrome.picker_label_width,
                text_chrome.info_label,
                |ui| {
                    provider_detail_select_frame(ui, detail_chrome, |ui| {
                        let text_edit = egui::TextEdit::singleline(&mut ollama_cookie_header)
                            .password(true)
                            .desired_width(224.0)
                            .hint_text("Cookie: ...");
                        let _ = ui.add(text_edit);
                    });
                },
            );
            if let Ok(mut state) = shared_state.lock()
                && state.settings.ollama_cookie_header != ollama_cookie_header
            {
                state.settings.ollama_cookie_header = ollama_cookie_header;
                state.settings_changed = true;
            }

            ui.add_space(6.0);
            if text_button(ui, "Open Ollama Settings", Theme::ACCENT_PRIMARY) {
                let _ = open::that("https://ollama.com/settings");
            }
            ui.add_space(detail_chrome.section_gap);
        }
    } else if provider_id == ProviderId::Copilot {
        let has_copilot_token = if let Ok(state) = shared_state.lock() {
            state
                .api_keys
                .get("copilot")
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false)
        } else {
            false
        };

        provider_detail_text_row(
            ui,
            "GitHub Login",
            if has_copilot_token {
                "Stored"
            } else {
                "Not signed in"
            },
            detail_chrome.picker_label_width,
            true,
        );
        ui.add_space(2.0);
        provider_detail_helper_text(ui, "Sign in with GitHub Device Flow.");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            let button_label = if has_copilot_token {
                "Sign in again"
            } else {
                "Sign in with GitHub"
            };
            if primary_button(ui, button_label) {
                let _ = open::that("https://github.com/settings/copilot");
            }
        });
        ui.add_space(detail_chrome.section_gap);
    } else if provider_id == ProviderId::Zai {
        let mut zai_api_key = if let Ok(state) = shared_state.lock() {
            state.api_keys.get("zai").unwrap_or_default().to_string()
        } else {
            String::new()
        };
        let mut zai_api_region = if let Ok(state) = shared_state.lock() {
            state.settings.zai_api_region.clone()
        } else {
            "global".to_string()
        };

        provider_detail_picker_row(
            ui,
            "API region",
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    egui::ComboBox::from_id_salt("zai_api_region")
                        .selected_text(zai_region_label(&zai_api_region))
                        .width(168.0)
                        .show_ui(ui, |ui| {
                            let _ = ui.selectable_value(
                                &mut zai_api_region,
                                "global".to_string(),
                                "Global",
                            );
                            let _ = ui.selectable_value(
                                &mut zai_api_region,
                                "china".to_string(),
                                "China Mainland (BigModel)",
                            );
                        });
                });
            },
        );
        if let Ok(mut state) = shared_state.lock()
            && state.settings.zai_api_region != zai_api_region
        {
            state.settings.zai_api_region = zai_api_region.clone();
            state.settings_changed = true;
        }

        ui.add_space(2.0);
        provider_detail_helper_text(
            ui,
            "Use BigModel for the China mainland endpoints (open.bigmodel.cn).",
        );
        ui.add_space(detail_chrome.section_gap);

        provider_detail_picker_row(
            ui,
            "API key",
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    let text_edit = egui::TextEdit::singleline(&mut zai_api_key)
                        .password(true)
                        .desired_width(224.0)
                        .hint_text("Paste API token...");
                    let _ = ui.add(text_edit);
                });
            },
        );
        if let Ok(mut state) = shared_state.lock() {
            let current = state.api_keys.get("zai").unwrap_or_default().to_string();
            if current != zai_api_key {
                if zai_api_key.trim().is_empty() {
                    state.api_keys.remove("zai");
                } else {
                    state.api_keys.set("zai", &zai_api_key, None);
                }
                state.settings_changed = true;
            }
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, "Stored in CodexBar. Paste a z.ai token.");
        ui.add_space(8.0);

        if text_button(ui, "Open z.ai Dashboard", Theme::ACCENT_PRIMARY) {
            let _ = open::that("https://z.ai/dashboard");
        }
        ui.add_space(detail_chrome.section_gap);
    } else if provider_id == ProviderId::Warp {
        let mut warp_api_key = if let Ok(state) = shared_state.lock() {
            state.api_keys.get("warp").unwrap_or_default().to_string()
        } else {
            String::new()
        };

        provider_detail_picker_row(
            ui,
            "API key",
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    let text_edit = egui::TextEdit::singleline(&mut warp_api_key)
                        .password(true)
                        .desired_width(224.0)
                        .hint_text("wk-...");
                    let _ = ui.add(text_edit);
                });
            },
        );
        if let Ok(mut state) = shared_state.lock() {
            let current = state.api_keys.get("warp").unwrap_or_default().to_string();
            if current != warp_api_key {
                if warp_api_key.trim().is_empty() {
                    state.api_keys.remove("warp");
                } else {
                    state.api_keys.set("warp", &warp_api_key, None);
                }
                state.settings_changed = true;
            }
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, "Stored in CodexBar. Create a Warp key.");
        ui.add_space(8.0);

        if text_button(ui, "Open Warp API Key Guide", Theme::ACCENT_PRIMARY) {
            let _ = open::that("https://docs.warp.dev/reference/cli/api-keys");
        }
        ui.add_space(detail_chrome.section_gap);
    } else if provider_id == ProviderId::OpenRouter {
        let mut openrouter_api_key = if let Ok(state) = shared_state.lock() {
            state
                .api_keys
                .get("openrouter")
                .unwrap_or_default()
                .to_string()
        } else {
            String::new()
        };

        provider_detail_picker_row(
            ui,
            "API key",
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    let text_edit = egui::TextEdit::singleline(&mut openrouter_api_key)
                        .password(true)
                        .desired_width(224.0)
                        .hint_text("sk-or-...");
                    let _ = ui.add(text_edit);
                });
            },
        );
        if let Ok(mut state) = shared_state.lock() {
            let current = state
                .api_keys
                .get("openrouter")
                .unwrap_or_default()
                .to_string();
            if current != openrouter_api_key {
                if openrouter_api_key.trim().is_empty() {
                    state.api_keys.remove("openrouter");
                } else {
                    state.api_keys.set("openrouter", &openrouter_api_key, None);
                }
                state.settings_changed = true;
            }
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, "Stored in CodexBar. Paste an OpenRouter key.");
        ui.add_space(8.0);

        if text_button(ui, "Open OpenRouter Credits", Theme::ACCENT_PRIMARY) {
            let _ = open::that("https://openrouter.ai/settings/credits");
        }
        ui.add_space(detail_chrome.section_gap);
    } else if provider_id == ProviderId::Synthetic {
        let mut synthetic_api_key = if let Ok(state) = shared_state.lock() {
            state
                .api_keys
                .get("synthetic")
                .unwrap_or_default()
                .to_string()
        } else {
            String::new()
        };

        provider_detail_picker_row(
            ui,
            "API key",
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    let text_edit = egui::TextEdit::singleline(&mut synthetic_api_key)
                        .password(true)
                        .desired_width(224.0)
                        .hint_text("Paste key...");
                    let _ = ui.add(text_edit);
                });
            },
        );
        if let Ok(mut state) = shared_state.lock() {
            let current = state
                .api_keys
                .get("synthetic")
                .unwrap_or_default()
                .to_string();
            if current != synthetic_api_key {
                if synthetic_api_key.trim().is_empty() {
                    state.api_keys.remove("synthetic");
                } else {
                    state.api_keys.set("synthetic", &synthetic_api_key, None);
                }
                state.settings_changed = true;
            }
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, "Stored in CodexBar. Paste a Synthetic key.");
        ui.add_space(detail_chrome.section_gap);
    } else if provider_id == ProviderId::NanoGPT {
        let mut nanogpt_api_key = if let Ok(state) = shared_state.lock() {
            state
                .api_keys
                .get("nanogpt")
                .unwrap_or_default()
                .to_string()
        } else {
            String::new()
        };

        provider_detail_picker_row(
            ui,
            "API key",
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    let text_edit = egui::TextEdit::singleline(&mut nanogpt_api_key)
                        .password(true)
                        .desired_width(224.0)
                        .hint_text("ngpt_...");
                    let _ = ui.add(text_edit);
                });
            },
        );
        if let Ok(mut state) = shared_state.lock() {
            let current = state
                .api_keys
                .get("nanogpt")
                .unwrap_or_default()
                .to_string();
            if current != nanogpt_api_key {
                if nanogpt_api_key.trim().is_empty() {
                    state.api_keys.remove("nanogpt");
                } else {
                    state.api_keys.set("nanogpt", &nanogpt_api_key, None);
                }
                state.settings_changed = true;
            }
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, "Stored in CodexBar. Paste a NanoGPT key.");
        ui.add_space(8.0);
        if text_button(ui, "Open NanoGPT API", Theme::ACCENT_PRIMARY) {
            let _ = open::that("https://nano-gpt.com/api");
        }
        ui.add_space(detail_chrome.section_gap);
    } else if provider_id == ProviderId::Infini {
        let mut infini_api_key = if let Ok(state) = shared_state.lock() {
            state.api_keys.get("infini").unwrap_or_default().to_string()
        } else {
            String::new()
        };

        provider_detail_picker_row(
            ui,
            "API key",
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    let text_edit = egui::TextEdit::singleline(&mut infini_api_key)
                        .password(true)
                        .desired_width(224.0)
                        .hint_text("sk-...");
                    let _ = ui.add(text_edit);
                });
            },
        );
        if let Ok(mut state) = shared_state.lock() {
            let current = state.api_keys.get("infini").unwrap_or_default().to_string();
            if current != infini_api_key {
                if infini_api_key.trim().is_empty() {
                    state.api_keys.remove("infini");
                } else {
                    state.api_keys.set("infini", &infini_api_key, None);
                }
                state.settings_changed = true;
            }
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, "Stored in CodexBar. Paste an Infini Cloud key.");
        ui.add_space(8.0);
        if text_button(ui, "Open Infini Cloud", Theme::ACCENT_PRIMARY) {
            let _ = open::that("https://cloud.infini-ai.com");
        }
        ui.add_space(detail_chrome.section_gap);
    } else if provider_id == ProviderId::JetBrains {
        let detected_paths = jetbrains_detected_ide_paths();
        let detected_path = detected_paths.first().cloned();
        let mut ide_base_path = if let Ok(state) = shared_state.lock() {
            state.settings.jetbrains_ide_base_path.clone()
        } else {
            String::new()
        };
        let mut ide_selection = if ide_base_path.trim().is_empty() {
            String::new()
        } else {
            ide_base_path.clone()
        };

        provider_detail_text_row(
            ui,
            "JetBrains IDE",
            if detected_path.is_some() || !ide_base_path.trim().is_empty() {
                "Detected"
            } else {
                "Not detected"
            },
            detail_chrome.picker_label_width,
            true,
        );
        ui.add_space(2.0);
        let helper = if ide_base_path.trim().is_empty() {
            if let Some(path) = detected_path.as_ref() {
                format!(
                    "Using detected IDE config at {}.",
                    compact_credentials_path(&path.display().to_string())
                )
            } else {
                "Install a JetBrains IDE with AI Assistant enabled, then refresh CodexBar."
                    .to_string()
            }
        } else {
            format!(
                "Using custom IDE base path {}.",
                compact_credentials_path(&ide_base_path)
            )
        };
        provider_detail_helper_text(ui, &helper);
        ui.add_space(detail_chrome.section_gap);

        if !detected_paths.is_empty() {
            provider_detail_picker_row(
                ui,
                "JetBrains IDE",
                detail_chrome.picker_label_width,
                text_chrome.info_label,
                |ui| {
                    provider_detail_select_frame(ui, detail_chrome, |ui| {
                        egui::ComboBox::from_id_salt("jetbrains_ide_selection")
                            .selected_text(if ide_selection.is_empty() {
                                "Auto-detect".to_string()
                            } else {
                                jetbrains_display_name(Path::new(&ide_selection))
                            })
                            .width(220.0)
                            .show_ui(ui, |ui| {
                                let _ = ui.selectable_value(
                                    &mut ide_selection,
                                    String::new(),
                                    "Auto-detect",
                                );
                                for path in &detected_paths {
                                    let path_string = path.display().to_string();
                                    let _ = ui.selectable_value(
                                        &mut ide_selection,
                                        path_string.clone(),
                                        jetbrains_display_name(path),
                                    );
                                }
                            });
                    });
                },
            );
            ui.add_space(2.0);
            provider_detail_helper_text(ui, "Select the JetBrains IDE to monitor.");
            ui.add_space(detail_chrome.section_gap);
        }

        if ide_selection != ide_base_path {
            ide_base_path = ide_selection.clone();
        }

        provider_detail_picker_row(
            ui,
            "Custom path",
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    let text_edit = egui::TextEdit::singleline(&mut ide_base_path)
                        .desired_width(260.0)
                        .hint_text("%APPDATA%/JetBrains/IntelliJIdea...");
                    let _ = ui.add(text_edit);
                });
            },
        );
        if let Ok(mut state) = shared_state.lock()
            && state.settings.jetbrains_ide_base_path != ide_base_path
        {
            state.settings.jetbrains_ide_base_path = ide_base_path.clone();
            state.settings_changed = true;
        }

        ui.add_space(8.0);
        if primary_button(ui, "Refresh Detection")
            && let Ok(mut state) = shared_state.lock()
        {
            state.refresh_requested = true;
        }
        ui.add_space(detail_chrome.section_gap);
    } else if provider_id == ProviderId::KimiK2 {
        let mut kimi_k2_api_key = if let Ok(state) = shared_state.lock() {
            state.api_keys.get("kimik2").unwrap_or_default().to_string()
        } else {
            String::new()
        };

        provider_detail_picker_row(
            ui,
            "API key",
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    let text_edit = egui::TextEdit::singleline(&mut kimi_k2_api_key)
                        .password(true)
                        .desired_width(224.0)
                        .hint_text("Paste API key...");
                    let _ = ui.add(text_edit);
                });
            },
        );
        if let Ok(mut state) = shared_state.lock() {
            let current = state.api_keys.get("kimik2").unwrap_or_default().to_string();
            if current != kimi_k2_api_key {
                if kimi_k2_api_key.trim().is_empty() {
                    state.api_keys.remove("kimik2");
                } else {
                    state.api_keys.set("kimik2", &kimi_k2_api_key, None);
                }
                state.settings_changed = true;
            }
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, "Stored in CodexBar. Paste a Moonshot key.");
        ui.add_space(8.0);

        if text_button(ui, "Open API Keys", Theme::ACCENT_PRIMARY) {
            let _ = open::that("https://kimi-k2.ai/user-center/api-keys");
        }
        ui.add_space(detail_chrome.section_gap);
    } else if provider_id == ProviderId::Claude {
        let mut claude_usage_source = if let Ok(state) = shared_state.lock() {
            state.settings.claude_usage_source.clone()
        } else {
            "auto".to_string()
        };
        provider_detail_picker_row(
            ui,
            locale_text(ui_language, LocaleKey::UsageSource),
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    egui::ComboBox::from_id_salt("claude_usage_source")
                        .selected_text(claude_usage_source_label(&claude_usage_source, ui_language))
                        .width(116.0)
                        .show_ui(ui, |ui| {
                            let _ = ui.selectable_value(
                                &mut claude_usage_source,
                                "auto".to_string(),
                                locale_text(ui_language, LocaleKey::Automatic),
                            );
                            let _ = ui.selectable_value(
                                &mut claude_usage_source,
                                "oauth".to_string(),
                                locale_text(ui_language, LocaleKey::OAuth),
                            );
                            let _ = ui.selectable_value(
                                &mut claude_usage_source,
                                "web".to_string(),
                                locale_text(ui_language, LocaleKey::Web),
                            );
                            let _ = ui.selectable_value(
                                &mut claude_usage_source,
                                "cli".to_string(),
                                locale_text(ui_language, LocaleKey::ProviderSourceCliShort),
                            );
                        });
                });
                ui.add_space(8.0);
                ui.label(
                    RichText::new(locale_text(ui_language, LocaleKey::ProviderSourceOauthWeb))
                        .size(FontSize::XS)
                        .color(text_chrome.secondary_value),
                );
            },
        );
        if let Ok(mut state) = shared_state.lock()
            && state.settings.claude_usage_source != claude_usage_source
        {
            state.settings.claude_usage_source = claude_usage_source;
            state.settings_changed = true;
        }

        ui.add_space(2.0);
        provider_detail_helper_text(ui, locale_text(ui_language, LocaleKey::AutoFallbackHelp));
        ui.add_space(detail_chrome.section_gap);

        let mut claude_cookie_source = if let Ok(state) = shared_state.lock() {
            state.settings.claude_cookie_source.clone()
        } else {
            "auto".to_string()
        };
        provider_detail_picker_row(
            ui,
            locale_text(ui_language, LocaleKey::ProviderClaudeCookies),
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    egui::ComboBox::from_id_salt("claude_cookie_source")
                        .selected_text(claude_cookie_source_label(
                            &claude_cookie_source,
                            ui_language,
                        ))
                        .width(116.0)
                        .show_ui(ui, |ui| {
                            let _ = ui.selectable_value(
                                &mut claude_cookie_source,
                                "auto".to_string(),
                                locale_text(ui_language, LocaleKey::Automatic),
                            );
                            let _ = ui.selectable_value(
                                &mut claude_cookie_source,
                                "manual".to_string(),
                                "Manual",
                            );
                        });
                });
            },
        );
        if let Ok(mut state) = shared_state.lock()
            && state.settings.claude_cookie_source != claude_cookie_source
        {
            state.settings.claude_cookie_source = claude_cookie_source;
            state.settings_changed = true;
        }

        ui.add_space(2.0);
        provider_detail_helper_text(
            ui,
            locale_text(ui_language, LocaleKey::ProviderClaudeCookiesHelp),
        );
        ui.add_space(detail_chrome.section_gap);
    }

    let shows_generic_usage_source = !matches!(
        provider_id,
        ProviderId::Codex
            | ProviderId::Cursor
            | ProviderId::Gemini
            | ProviderId::Antigravity
            | ProviderId::OpenCode
            | ProviderId::MiniMax
            | ProviderId::Factory
            | ProviderId::Kimi
            | ProviderId::Copilot
            | ProviderId::Alibaba
            | ProviderId::Amp
            | ProviderId::Augment
            | ProviderId::Claude
            | ProviderId::Infini
            | ProviderId::JetBrains
            | ProviderId::KimiK2
            | ProviderId::NanoGPT
            | ProviderId::Ollama
            | ProviderId::OpenRouter
            | ProviderId::Synthetic
            | ProviderId::VertexAI
            | ProviderId::Warp
            | ProviderId::Zai
    );

    let shows_options_section = provider_id == ProviderId::Codex
        || provider_id == ProviderId::Claude
        || shows_generic_usage_source;

    if shows_options_section && !matches!(provider_id, ProviderId::Cursor | ProviderId::Claude) {
        if provider_id == ProviderId::Codex {
            provider_detail_section_title_compact(
                ui,
                locale_text(ui_language, LocaleKey::ProviderOptionsTitle),
            );
        } else {
            provider_detail_section_title(
                ui,
                locale_text(ui_language, LocaleKey::ProviderOptionsTitle),
            );
        }
    }

    if shows_generic_usage_source {
        provider_detail_picker_row(
            ui,
            locale_text(ui_language, LocaleKey::UsageSource),
            detail_chrome.picker_label_width,
            text_chrome.info_label,
            |ui| {
                provider_detail_select_frame(ui, detail_chrome, |ui| {
                    egui::ComboBox::from_id_salt(format!("source_{}", provider_id.cli_name()))
                        .selected_text(locale_text(ui_language, LocaleKey::Automatic))
                        .width(104.0)
                        .show_ui(ui, |ui| {
                            let _ = ui.selectable_label(
                                true,
                                locale_text(ui_language, LocaleKey::Automatic),
                            );
                            let _ = ui.selectable_label(
                                false,
                                locale_text(ui_language, LocaleKey::OAuth),
                            );
                            let _ = ui
                                .selectable_label(false, locale_text(ui_language, LocaleKey::Api));
                        });
                });
                ui.add_space(8.0);
                ui.label(
                    RichText::new(locale_text(ui_language, LocaleKey::ProviderSourceOauthWeb))
                        .size(FontSize::XS)
                        .color(text_chrome.secondary_value),
                );
            },
        );

        ui.add_space(2.0);
        provider_detail_helper_text(ui, locale_text(ui_language, LocaleKey::AutoFallbackHelp));
    }

    if provider_id == ProviderId::Codex {
        let mut historical_tracking_enabled = if let Ok(state) = shared_state.lock() {
            state.settings.codex_historical_tracking
        } else {
            false
        };
        if provider_detail_compact_toggle_row(
            ui,
            locale_text(ui_language, LocaleKey::ProviderHistoricalTracking),
            locale_text(ui_language, LocaleKey::ProviderCodexHistoryHelp),
            Some(&mut historical_tracking_enabled),
            true,
        ) && let Ok(mut state) = shared_state.lock()
        {
            state.settings.codex_historical_tracking = historical_tracking_enabled;
            state.settings_changed = true;
        }

        provider_detail_soft_divider(ui);

        let mut show_web_extras = if let Ok(state) = shared_state.lock() {
            state.settings.codex_openai_web_extras
        } else {
            true
        };

        if provider_detail_compact_toggle_row(
            ui,
            locale_text(ui_language, LocaleKey::ProviderOpenAiWebExtras),
            locale_text(ui_language, LocaleKey::ProviderOpenAiWebExtrasHelp),
            Some(&mut show_web_extras),
            true,
        ) && let Ok(mut state) = shared_state.lock()
        {
            state.settings.codex_openai_web_extras = show_web_extras;
            state.settings_changed = true;
        }
    } else if provider_id == ProviderId::Claude {
        provider_detail_section_title(
            ui,
            locale_text(ui_language, LocaleKey::ProviderOptionsTitle),
        );

        let mut avoid_keychain_prompts = if let Ok(state) = shared_state.lock() {
            state.settings.claude_avoid_keychain_prompts
        } else {
            false
        };

        if provider_detail_compact_toggle_row(
            ui,
            locale_text(ui_language, LocaleKey::ProviderClaudeAvoidKeychainPrompts),
            locale_text(
                ui_language,
                LocaleKey::ProviderClaudeAvoidKeychainPromptsHelp,
            ),
            Some(&mut avoid_keychain_prompts),
            true,
        ) && let Ok(mut state) = shared_state.lock()
        {
            state.settings.claude_avoid_keychain_prompts = avoid_keychain_prompts;
            state.settings_changed = true;
        }
    }

    // ═══════════════════════════════════════════════════════════
    // ACCOUNTS SECTION - Token account switching (only for supported providers)
    // ═══════════════════════════════════════════════════════════
    if should_show_token_accounts_section(provider_id, shared_state) {
        ui.add_space(Spacing::LG);
        render_accounts_section(ui, provider_id, shared_state);
    }
}

fn provider_detail_select_frame(
    ui: &mut egui::Ui,
    chrome: ProviderDetailChrome,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::none()
        .fill(chrome.control_fill)
        .stroke(chrome.control_stroke)
        .rounding(Rounding::same(Radius::SM))
        .inner_margin(egui::Margin::symmetric(
            chrome.control_inner_margin_x,
            chrome.control_inner_margin_y,
        ))
        .show(ui, add_contents);
}

fn provider_detail_picker_row(
    ui: &mut egui::Ui,
    label: &str,
    label_width: f32,
    label_color: Color32,
    add_picker: impl FnOnce(&mut egui::Ui),
) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [label_width, 20.0],
            egui::Label::new(
                RichText::new(label)
                    .size(FontSize::SM)
                    .color(label_color)
                    .strong(),
            ),
        );
        ui.add_space(10.0);
        add_picker(ui);
    });
}

fn claude_usage_source_label(value: &str, ui_language: Language) -> String {
    match value {
        "oauth" => locale_text(ui_language, LocaleKey::OAuth).to_string(),
        "web" => locale_text(ui_language, LocaleKey::Web).to_string(),
        "cli" => locale_text(ui_language, LocaleKey::ProviderSourceCliShort).to_string(),
        _ => locale_text(ui_language, LocaleKey::Automatic).to_string(),
    }
}

fn codex_usage_source_label(value: &str, ui_language: Language) -> String {
    match value {
        "oauth" => locale_text(ui_language, LocaleKey::OAuth).to_string(),
        "cli" => locale_text(ui_language, LocaleKey::ProviderSourceCliShort).to_string(),
        _ => locale_text(ui_language, LocaleKey::Automatic).to_string(),
    }
}

fn codex_cookie_source_label(value: &str, ui_language: Language) -> String {
    match value {
        "manual" => "Manual".to_string(),
        "off" => locale_text(ui_language, LocaleKey::ProviderDisabled).to_string(),
        _ => locale_text(ui_language, LocaleKey::Automatic).to_string(),
    }
}

fn claude_cookie_source_label(value: &str, ui_language: Language) -> String {
    match value {
        "manual" => "Manual".to_string(),
        _ => locale_text(ui_language, LocaleKey::Automatic).to_string(),
    }
}

fn provider_detail_compact_toggle_row(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: &str,
    value: Option<&mut bool>,
    enabled: bool,
) -> bool {
    let mut changed = false;
    let text_chrome = provider_detail_text_chrome();
    let content_width = ui.available_width();
    let toggle_slot_width = if value.is_some() { 44.0 } else { 0.0 };
    let text_width = (content_width - toggle_slot_width - 12.0).max(140.0);

    ui.spacing_mut().item_spacing.y = 8.0;
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(text_width, 0.0),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                ui.label(
                    RichText::new(title)
                        .size(FontSize::SM)
                        .color(Theme::TEXT_PRIMARY)
                        .strong(),
                );

                if !subtitle.is_empty() {
                    ui.label(
                        RichText::new(subtitle)
                            .size(FontSize::XS)
                            .color(text_chrome.helper),
                    );
                }
            },
        );

        if let Some(value) = value {
            ui.add_space(12.0);
            ui.allocate_ui_with_layout(
                Vec2::new(toggle_slot_width, 20.0),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    if enabled {
                        if switch_toggle(ui, format!("switch_{}", title), value) {
                            changed = true;
                        }
                    } else {
                        let _ = switch_toggle_visual(ui, format!("switch_{}", title), value, false);
                    }
                },
            );
        }
    });

    changed
}

fn provider_detail_section_title(ui: &mut egui::Ui, text: &str) {
    let text_chrome = provider_detail_text_chrome();
    ui.add_space(Spacing::SM);
    ui.label(
        RichText::new(text)
            .size(FontSize::SM)
            .color(text_chrome.section_title)
            .strong(),
    );
    ui.add_space(4.0);
}

fn provider_detail_section_title_compact(ui: &mut egui::Ui, text: &str) {
    let text_chrome = provider_detail_text_chrome();
    ui.add_space(4.0);
    ui.label(
        RichText::new(text)
            .size(FontSize::SM)
            .color(text_chrome.section_title)
            .strong(),
    );
    ui.add_space(2.0);
}

fn provider_detail_helper_text(ui: &mut egui::Ui, text: &str) {
    let text_chrome = provider_detail_text_chrome();
    ui.label(
        RichText::new(text)
            .size(FontSize::XS)
            .color(text_chrome.helper),
    );
}

fn provider_detail_soft_divider(ui: &mut egui::Ui) {
    ui.add_space(6.0);
    let rect = Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width(), 1.0));
    ui.painter()
        .rect_filled(rect, 0.0, Theme::SEPARATOR.gamma_multiply(0.48));
    ui.add_space(7.0);
}

/// Helper: Info grid row
fn info_row(ui: &mut egui::Ui, label: &str, value: &str, label_width: f32) {
    let text_chrome = provider_detail_text_chrome();
    ui.add_sized(
        [label_width, 17.0],
        egui::Label::new(
            RichText::new(label)
                .size(FontSize::XS)
                .color(text_chrome.info_label),
        ),
    );
    ui.add_sized(
        [ui.available_width(), 17.0],
        egui::Label::new(
            RichText::new(value)
                .size(FontSize::XS)
                .color(Theme::TEXT_PRIMARY),
        )
        .wrap(),
    );
    ui.end_row();
}

fn provider_detail_text_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
    label_width: f32,
    strong_label: bool,
) {
    let text_chrome = provider_detail_text_chrome();
    ui.horizontal(|ui| {
        let mut label_text = RichText::new(label)
            .size(FontSize::SM)
            .color(text_chrome.secondary_value);
        if strong_label {
            label_text = label_text.color(text_chrome.info_label).strong();
        }
        ui.add_sized([label_width, 18.0], egui::Label::new(label_text));
        ui.label(
            RichText::new(value)
                .size(FontSize::XS)
                .color(Theme::TEXT_PRIMARY),
        );
    });
}

/// Helper: Usage bar row with label, percentage, info text
#[allow(clippy::too_many_arguments)]
fn usage_bar_row(
    ui: &mut egui::Ui,
    label: &str,
    percent: f32,
    info: &str,
    reset: Option<&str>,
    color: Color32,
    label_width: f32,
    bar_width: f32,
) {
    let text_chrome = provider_detail_text_chrome();
    ui.horizontal(|ui| {
        ui.add_sized(
            [label_width, 17.0],
            egui::Label::new(
                RichText::new(label)
                    .size(FontSize::XS)
                    .color(text_chrome.info_label),
            ),
        );

        ui.vertical(|ui| {
            let bar_height = 5.0;
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(bar_width, bar_height), egui::Sense::hover());

            ui.painter()
                .rect_filled(rect, Rounding::same(3.0), Theme::progress_track());

            ui.painter().rect_filled(
                Rect::from_min_size(
                    rect.min,
                    Vec2::new(rect.width() * (percent / 100.0).clamp(0.0, 1.0), bar_height),
                ),
                Rounding::same(3.0),
                color,
            );

            ui.add_space(3.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(info)
                        .size(FontSize::XS)
                        .color(Theme::TEXT_PRIMARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(reset_text) = reset {
                        ui.label(
                            RichText::new(reset_text)
                                .size(FontSize::XS)
                                .color(text_chrome.secondary_value),
                        );
                    }
                });
            });
        });
    });
}

fn usage_display(used_percent: f64, show_as_used: bool, lang: Language) -> (f64, String) {
    let used_percent = used_percent.clamp(0.0, 100.0);
    let display_percent = if show_as_used {
        used_percent
    } else {
        100.0 - used_percent
    };

    let label = if show_as_used {
        locale_text(lang, LocaleKey::ShowUsedPercent)
            .replace("{:.0}", &format!("{:.0}", display_percent))
    } else {
        locale_text(lang, LocaleKey::ShowRemainingPercent)
            .replace("{:.0}", &format!("{:.0}", display_percent))
    };

    (display_percent, label)
}

fn metric_preference_text(
    preference: crate::settings::MetricPreference,
    lang: Language,
) -> &'static str {
    match preference {
        crate::settings::MetricPreference::Automatic => locale_text(lang, LocaleKey::Automatic),
        crate::settings::MetricPreference::Session => {
            locale_text(lang, LocaleKey::ProviderSessionLabel)
        }
        crate::settings::MetricPreference::Weekly => {
            locale_text(lang, LocaleKey::ProviderWeeklyLabel)
        }
        crate::settings::MetricPreference::Model => locale_text(lang, LocaleKey::ProviderModel),
        crate::settings::MetricPreference::Credits => locale_text(lang, LocaleKey::CreditsLabel),
        crate::settings::MetricPreference::Average => locale_text(lang, LocaleKey::Average),
    }
}

fn refresh_interval_text(seconds: u64, lang: Language) -> String {
    match seconds {
        0 => locale_text(lang, LocaleKey::Never).to_string(),
        30 => locale_text(lang, LocaleKey::RefreshInterval30Sec).to_string(),
        60 => locale_text(lang, LocaleKey::RefreshInterval1Min).to_string(),
        300 => locale_text(lang, LocaleKey::RefreshInterval5Min).to_string(),
        600 => locale_text(lang, LocaleKey::RefreshInterval10Min).to_string(),
        _ => seconds.to_string(),
    }
}

/// Render Accounts section for token account switching
fn render_accounts_section(
    ui: &mut egui::Ui,
    provider_id: ProviderId,
    shared_state: &Arc<Mutex<PreferencesSharedState>>,
) {
    let text_chrome = provider_detail_text_chrome();
    let support = match TokenAccountSupport::for_provider(provider_id) {
        Some(s) => s,
        None => return,
    };
    let ui_language = if let Ok(state) = shared_state.lock() {
        state.settings.ui_language
    } else {
        Language::English
    };

    provider_detail_section_title(
        ui,
        locale_text(ui_language, LocaleKey::ProviderAccountsTitle),
    );
    ui.label(
        RichText::new(support.title)
            .size(FontSize::XS)
            .color(Theme::TEXT_PRIMARY),
    );
    ui.add_space(2.0);
    ui.label(
        RichText::new(support.subtitle)
            .size(FontSize::XS)
            .color(text_chrome.helper),
    );
    ui.add_space(Spacing::SM);

    // Get current accounts for this provider
    let (accounts_data, show_add, status_msg) = if let Ok(state) = shared_state.lock() {
        let data = state
            .token_accounts
            .get(&provider_id)
            .cloned()
            .unwrap_or_default();
        (
            data,
            state.show_add_account_input,
            state.token_account_status_msg.clone(),
        )
    } else {
        (ProviderAccountData::default(), false, None)
    };

    // Status message
    if let Some((msg, is_error)) = &status_msg {
        let color = if *is_error { Theme::RED } else { Theme::GREEN };
        ui.label(RichText::new(msg).size(FontSize::SM).color(color));
        ui.add_space(Spacing::SM);
    }

    // List existing accounts with radio buttons
    if !accounts_data.accounts.is_empty() {
        let active_idx = accounts_data.clamped_active_index();

        for (idx, account) in accounts_data.accounts.iter().enumerate() {
            let is_active = idx == active_idx;

            ui.horizontal(|ui| {
                // Radio button
                if ui.radio(is_active, "").clicked() && !is_active {
                    // Set as active
                    if let Ok(mut state) = shared_state.lock() {
                        if let Some(data) = state.token_accounts.get_mut(&provider_id) {
                            data.set_active(idx);
                        }
                        // Save to disk
                        let store = TokenAccountStore::new();
                        if let Err(e) = store.save(&state.token_accounts) {
                            state.token_account_status_msg = Some((
                                locale_text(ui_language, LocaleKey::SaveFailed)
                                    .replace("{}", &e.to_string()),
                                true,
                            ));
                        } else {
                            state.token_account_status_msg = Some((
                                locale_text(ui_language, LocaleKey::AccountSwitched).to_string(),
                                false,
                            ));
                        }
                    }
                }

                ui.add_space(4.0);

                // Account label
                ui.label(
                    RichText::new(account.display_name())
                        .size(FontSize::SM)
                        .color(if is_active {
                            Theme::TEXT_PRIMARY
                        } else {
                            text_chrome.secondary_value
                        }),
                );

                // Truncated token preview
                let token_preview = if account.token.len() > 16 {
                    format!("{}...", &account.token[..12])
                } else {
                    account.token.clone()
                };
                ui.label(
                    RichText::new(token_preview)
                        .size(FontSize::XS)
                        .color(text_chrome.helper)
                        .monospace(),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Remove button
                    let account_id = account.id;
                    if small_button(ui, locale_text(ui_language, LocaleKey::Remove), Theme::RED)
                        && let Ok(mut state) = shared_state.lock()
                    {
                        if let Some(data) = state.token_accounts.get_mut(&provider_id) {
                            data.remove_account(account_id);
                        }
                        // Save to disk
                        let store = TokenAccountStore::new();
                        if let Err(e) = store.save(&state.token_accounts) {
                            state.token_account_status_msg = Some((
                                locale_text(ui_language, LocaleKey::SaveFailed)
                                    .replace("{}", &e.to_string()),
                                true,
                            ));
                        } else {
                            state.token_account_status_msg = Some((
                                locale_text(ui_language, LocaleKey::AccountRemoved).to_string(),
                                false,
                            ));
                        }
                    }
                });
            });

            ui.add_space(4.0);
        }

        ui.add_space(Spacing::SM);
    }

    // Add Account button or input form
    if show_add {
        // Input form for adding new account
        egui::Frame::none()
            .fill(Theme::BG_TERTIARY)
            .stroke(Stroke::new(1.0, Theme::ACCENT_PRIMARY.gamma_multiply(0.4)))
            .rounding(Rounding::same(Radius::MD))
            .inner_margin(Spacing::MD)
            .show(ui, |ui| {
                ui.label(
                    RichText::new(locale_text(ui_language, LocaleKey::AddAccount))
                        .size(FontSize::MD)
                        .color(Theme::TEXT_PRIMARY)
                        .strong(),
                );

                ui.add_space(Spacing::SM);

                // Label input
                ui.label(
                    RichText::new(locale_text(ui_language, LocaleKey::Label))
                        .size(FontSize::SM)
                        .color(Theme::TEXT_SECONDARY),
                );
                let mut label = if let Ok(state) = shared_state.lock() {
                    state.new_account_label.clone()
                } else {
                    String::new()
                };
                let label_edit = egui::TextEdit::singleline(&mut label)
                    .desired_width(ui.available_width())
                    .hint_text(locale_text(ui_language, LocaleKey::AccountLabelHint));
                if ui.add(label_edit).changed()
                    && let Ok(mut state) = shared_state.lock()
                {
                    state.new_account_label = label;
                }

                ui.add_space(Spacing::SM);

                // Token input
                ui.label(
                    RichText::new(locale_text(ui_language, LocaleKey::Token))
                        .size(FontSize::SM)
                        .color(Theme::TEXT_SECONDARY),
                );
                let mut token = if let Ok(state) = shared_state.lock() {
                    state.new_account_token.clone()
                } else {
                    String::new()
                };
                let token_edit = egui::TextEdit::singleline(&mut token)
                    .password(true)
                    .desired_width(ui.available_width())
                    .hint_text(support.placeholder);
                if ui.add(token_edit).changed()
                    && let Ok(mut state) = shared_state.lock()
                {
                    state.new_account_token = token;
                }

                ui.add_space(Spacing::MD);

                ui.horizontal(|ui| {
                    let (can_save, label_val, token_val) = if let Ok(state) = shared_state.lock() {
                        let can = !state.new_account_label.trim().is_empty()
                            && !state.new_account_token.trim().is_empty();
                        (
                            can,
                            state.new_account_label.clone(),
                            state.new_account_token.clone(),
                        )
                    } else {
                        (false, String::new(), String::new())
                    };

                    if ui
                        .add_enabled(
                            can_save,
                            egui::Button::new(
                                RichText::new(locale_text(ui_language, LocaleKey::Save))
                                    .size(FontSize::SM)
                                    .color(Color32::WHITE),
                            )
                            .fill(if can_save {
                                Theme::GREEN
                            } else {
                                Theme::BG_TERTIARY
                            })
                            .rounding(Rounding::same(Radius::SM))
                            .min_size(Vec2::new(80.0, 32.0)),
                        )
                        .clicked()
                        && let Ok(mut state) = shared_state.lock()
                    {
                        // Create new account
                        let account = TokenAccount::new(label_val.trim(), token_val.trim());

                        // Add to provider data
                        let data = state.token_accounts.entry(provider_id).or_default();
                        data.add_account(account);

                        // Save to disk
                        let store = TokenAccountStore::new();
                        if let Err(e) = store.save(&state.token_accounts) {
                            state.token_account_status_msg = Some((
                                locale_text(ui_language, LocaleKey::SaveFailed)
                                    .replace("{}", &e.to_string()),
                                true,
                            ));
                        } else {
                            state.token_account_status_msg = Some((
                                locale_text(ui_language, LocaleKey::AccountAdded).to_string(),
                                false,
                            ));
                            state.new_account_label.clear();
                            state.new_account_token.clear();
                            state.show_add_account_input = false;
                        }
                    }

                    ui.add_space(Spacing::XS);

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(locale_text(ui_language, LocaleKey::Cancel))
                                    .size(FontSize::SM)
                                    .color(Theme::TEXT_MUTED),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::new(1.0, Theme::BORDER_SUBTLE))
                            .rounding(Rounding::same(Radius::SM)),
                        )
                        .clicked()
                        && let Ok(mut state) = shared_state.lock()
                    {
                        state.show_add_account_input = false;
                        state.new_account_label.clear();
                        state.new_account_token.clear();
                    }
                });
            });
    } else {
        // Add Account button
        let add_account_text = format!("+ {}", locale_text(ui_language, LocaleKey::AddAccount));
        if primary_button(ui, &add_account_text)
            && let Ok(mut state) = shared_state.lock()
        {
            state.show_add_account_input = true;
            state.new_account_label.clear();
            state.new_account_token.clear();
            state.token_account_status_msg = None;
        }
    }
}

/// Render General tab for viewport
fn render_general_tab(ui: &mut egui::Ui, shared_state: &Arc<Mutex<PreferencesSharedState>>) {
    // Get current language from shared state
    let ui_language = if let Ok(state) = shared_state.lock() {
        state.settings.ui_language
    } else {
        Language::English
    };

    preferences_pane_header(
        ui,
        preferences_section_title(PreferencesTab::General),
        preferences_section_subtitle(PreferencesTab::General),
    );

    // LANGUAGE section - at the top of General tab
    section_header(ui, locale_text(ui_language, LocaleKey::InterfaceLanguage));

    settings_card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(locale_text(ui_language, LocaleKey::InterfaceLanguage))
                    .size(FontSize::MD)
                    .color(Theme::TEXT_PRIMARY),
            );
            ui.add_space(Spacing::MD);

            // Language selector ComboBox
            let current_language = if let Ok(state) = shared_state.lock() {
                state.settings.ui_language
            } else {
                Language::English
            };
            let current_label = current_language.display_name();

            egui::ComboBox::from_id_salt("language_selector_viewport")
                .selected_text(current_label)
                .show_ui(ui, |ui| {
                    for lang in Language::all() {
                        let is_selected = current_language == *lang;
                        if ui
                            .selectable_label(is_selected, lang.display_name())
                            .clicked()
                            && let Ok(mut state) = shared_state.lock()
                        {
                            state.settings.ui_language = *lang;
                            state.settings_changed = true;
                        }
                    }
                });
        });
    });

    ui.add_space(Spacing::MD);

    section_header(ui, locale_text(ui_language, LocaleKey::StartupSettings));

    settings_card(ui, |ui| {
        let mut start_at_login = if let Ok(state) = shared_state.lock() {
            state.settings.start_at_login
        } else {
            false
        };

        if setting_toggle(
            ui,
            locale_text(ui_language, LocaleKey::StartAtLogin),
            locale_text(ui_language, LocaleKey::StartAtLoginHelper),
            &mut start_at_login,
        ) && let Ok(mut state) = shared_state.lock()
        {
            if let Err(e) = state.settings.set_start_at_login(start_at_login) {
                tracing::error!("Failed to set start at login: {}", e);
            } else {
                state.settings_changed = true;
            }
        }

        setting_divider(ui);

        let mut start_minimized = if let Ok(state) = shared_state.lock() {
            state.settings.start_minimized
        } else {
            false
        };

        if setting_toggle(
            ui,
            locale_text(ui_language, LocaleKey::StartMinimized),
            locale_text(ui_language, LocaleKey::StartMinimizedHelper),
            &mut start_minimized,
        ) && let Ok(mut state) = shared_state.lock()
        {
            state.settings.start_minimized = start_minimized;
            state.settings_changed = true;
        }
    });

    ui.add_space(Spacing::MD);

    section_header(ui, locale_text(ui_language, LocaleKey::ShowNotifications));

    settings_card(ui, |ui| {
        let mut show_notifications = if let Ok(state) = shared_state.lock() {
            state.settings.show_notifications
        } else {
            true
        };

        if setting_toggle(
            ui,
            locale_text(ui_language, LocaleKey::ShowNotifications),
            locale_text(ui_language, LocaleKey::ShowNotificationsHelper),
            &mut show_notifications,
        ) && let Ok(mut state) = shared_state.lock()
        {
            state.settings.show_notifications = show_notifications;
            state.settings_changed = true;
        }

        setting_divider(ui);

        // Sound effects toggle
        let mut sound_enabled = if let Ok(state) = shared_state.lock() {
            state.settings.sound_enabled
        } else {
            true
        };

        if setting_toggle(
            ui,
            locale_text(ui_language, LocaleKey::SoundEnabled),
            locale_text(ui_language, LocaleKey::SoundEnabledHelper),
            &mut sound_enabled,
        ) && let Ok(mut state) = shared_state.lock()
        {
            state.settings.sound_enabled = sound_enabled;
            state.settings_changed = true;
        }

        // Sound volume slider (only show if sound is enabled)
        if sound_enabled {
            setting_divider(ui);

            ui.vertical(|ui| {
                let mut volume = if let Ok(state) = shared_state.lock() {
                    state.settings.sound_volume as i32
                } else {
                    100
                };

                // Title row with volume badge on right
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(locale_text(ui_language, LocaleKey::SoundVolume))
                            .size(FontSize::MD)
                            .color(Theme::TEXT_PRIMARY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        egui::Frame::none()
                            .fill(Theme::ACCENT_PRIMARY.gamma_multiply(0.15))
                            .rounding(Rounding::same(10.0))
                            .inner_margin(egui::Margin::symmetric(10.0, 3.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!("{}%", volume))
                                        .size(FontSize::SM)
                                        .color(Theme::ACCENT_PRIMARY)
                                        .strong(),
                                );
                            });
                    });
                });

                ui.add_space(2.0);
                ui.label(
                    RichText::new(locale_text(ui_language, LocaleKey::SoundVolume))
                        .size(FontSize::SM)
                        .color(Theme::TEXT_MUTED),
                );
                ui.add_space(6.0);

                ui.style_mut().visuals.widgets.inactive.bg_fill = Theme::BG_TERTIARY;

                let slider = ui.add(
                    egui::Slider::new(&mut volume, 0..=100)
                        .show_value(false)
                        .trailing_fill(true),
                );

                if slider.changed()
                    && let Ok(mut state) = shared_state.lock()
                {
                    state.settings.sound_volume = volume as u8;
                    state.settings_changed = true;
                }
            });
        }

        setting_divider(ui);

        // High warning threshold
        ui.vertical(|ui| {
            let mut threshold = if let Ok(state) = shared_state.lock() {
                state.settings.high_usage_threshold as i32
            } else {
                70
            };

            // Title row with percentage badge on right
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(locale_text(ui_language, LocaleKey::HighUsageAlert))
                        .size(FontSize::MD)
                        .color(Theme::TEXT_PRIMARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::Frame::none()
                        .fill(Theme::ACCENT_PRIMARY.gamma_multiply(0.15))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(egui::Margin::symmetric(10.0, 3.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("{}%", threshold))
                                    .size(FontSize::SM)
                                    .color(Theme::ACCENT_PRIMARY)
                                    .strong(),
                            );
                        });
                });
            });

            ui.add_space(2.0);
            ui.label(
                RichText::new(locale_text(
                    ui_language,
                    LocaleKey::HighUsageThresholdHelper,
                ))
                .size(FontSize::SM)
                .color(Theme::TEXT_MUTED),
            );
            ui.add_space(6.0);

            ui.style_mut().visuals.widgets.inactive.bg_fill = Theme::BG_TERTIARY;

            let slider = ui.add(
                egui::Slider::new(&mut threshold, 50..=95)
                    .show_value(false)
                    .trailing_fill(true),
            );

            if slider.changed()
                && let Ok(mut state) = shared_state.lock()
            {
                state.settings.high_usage_threshold = threshold as f64;
                state.settings_changed = true;
            }
        });

        setting_divider(ui);

        // Critical alert threshold
        ui.vertical(|ui| {
            let mut threshold = if let Ok(state) = shared_state.lock() {
                state.settings.critical_usage_threshold as i32
            } else {
                90
            };

            let badge_color = Color32::from_rgb(239, 68, 68);

            // Title row with percentage badge on right
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(locale_text(ui_language, LocaleKey::CriticalUsageAlert))
                        .size(FontSize::MD)
                        .color(Theme::TEXT_PRIMARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::Frame::none()
                        .fill(badge_color.gamma_multiply(0.15))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(egui::Margin::symmetric(10.0, 3.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("{}%", threshold))
                                    .size(FontSize::SM)
                                    .color(badge_color)
                                    .strong(),
                            );
                        });
                });
            });

            ui.add_space(2.0);
            ui.label(
                RichText::new(locale_text(
                    ui_language,
                    LocaleKey::CriticalUsageThresholdHelper,
                ))
                .size(FontSize::SM)
                .color(Theme::TEXT_MUTED),
            );
            ui.add_space(6.0);

            ui.style_mut().visuals.widgets.inactive.bg_fill = Theme::BG_TERTIARY;

            let slider = ui.add(
                egui::Slider::new(&mut threshold, 80..=100)
                    .show_value(false)
                    .trailing_fill(true),
            );

            if slider.changed()
                && let Ok(mut state) = shared_state.lock()
            {
                state.settings.critical_usage_threshold = threshold as f64;
                state.settings_changed = true;
            }
        });
    });

    ui.add_space(Spacing::MD);

    section_header(ui, locale_text(ui_language, LocaleKey::UpdatesTitle));

    settings_card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(locale_text(ui_language, LocaleKey::UpdateChannelChoice))
                        .size(FontSize::MD)
                        .color(Theme::TEXT_PRIMARY),
                );
                ui.label(
                    RichText::new(locale_text(
                        ui_language,
                        LocaleKey::UpdateChannelChoiceHelper,
                    ))
                    .size(FontSize::SM)
                    .color(Theme::TEXT_MUTED),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let current_channel = if let Ok(state) = shared_state.lock() {
                    state.settings.update_channel
                } else {
                    crate::settings::UpdateChannel::Stable
                };

                egui::Frame::none()
                    .fill(Theme::BG_TERTIARY)
                    .stroke(Stroke::new(1.0, Theme::BORDER_SUBTLE))
                    .rounding(Rounding::same(Radius::SM))
                    .inner_margin(egui::Margin::symmetric(Spacing::XS, 2.0))
                    .show(ui, |ui| {
                        let channels = [
                            (
                                crate::settings::UpdateChannel::Stable,
                                locale_text(ui_language, LocaleKey::UpdateChannelStable),
                            ),
                            (
                                crate::settings::UpdateChannel::Beta,
                                locale_text(ui_language, LocaleKey::UpdateChannelBeta),
                            ),
                        ];

                        let mut selected = current_channel;
                        egui::ComboBox::from_id_salt("update_channel")
                            .selected_text(
                                channels
                                    .iter()
                                    .find(|(ch, _)| *ch == selected)
                                    .map(|(_, label)| *label)
                                    .unwrap_or(locale_text(
                                        ui_language,
                                        LocaleKey::UpdateChannelStable,
                                    )),
                            )
                            .show_ui(ui, |ui| {
                                for (channel, label) in channels {
                                    if ui.selectable_value(&mut selected, channel, label).changed()
                                        && let Ok(mut state) = shared_state.lock()
                                    {
                                        state.settings.update_channel = selected;
                                        state.settings_changed = true;
                                    }
                                }
                            });
                    });
            });

            setting_divider(ui);

            let mut auto_download_updates = if let Ok(state) = shared_state.lock() {
                state.settings.auto_download_updates
            } else {
                true
            };

            if setting_toggle(
                ui,
                locale_text(ui_language, LocaleKey::AutoDownloadUpdates),
                locale_text(ui_language, LocaleKey::AutoDownloadUpdatesHelper),
                &mut auto_download_updates,
            ) && let Ok(mut state) = shared_state.lock()
            {
                state.settings.auto_download_updates = auto_download_updates;
                state.settings_changed = true;
            }

            setting_divider(ui);

            let mut install_updates_on_quit = if let Ok(state) = shared_state.lock() {
                state.settings.install_updates_on_quit
            } else {
                false
            };

            if setting_toggle(
                ui,
                locale_text(ui_language, LocaleKey::InstallUpdatesOnQuit),
                locale_text(ui_language, LocaleKey::InstallUpdatesOnQuitHelper),
                &mut install_updates_on_quit,
            ) && let Ok(mut state) = shared_state.lock()
            {
                state.settings.install_updates_on_quit = install_updates_on_quit;
                state.settings_changed = true;
            }
        });
    });
}

/// Render Shortcuts tab (extracted from General tab)
fn render_shortcuts_tab(ui: &mut egui::Ui, shared_state: &Arc<Mutex<PreferencesSharedState>>) {
    let ui_language = if let Ok(state) = shared_state.lock() {
        state.settings.ui_language
    } else {
        Language::English
    };

    section_header(
        ui,
        locale_text(ui_language, LocaleKey::KeyboardShortcutsTitle),
    );

    settings_card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(locale_text(ui_language, LocaleKey::GlobalShortcutLabel))
                        .size(FontSize::SM)
                        .color(Theme::TEXT_PRIMARY),
                );
                ui.label(
                    RichText::new(locale_text(ui_language, LocaleKey::GlobalShortcutHelper))
                        .size(FontSize::XS)
                        .color(Theme::TEXT_SECONDARY),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (mut shortcut_input, _) = if let Ok(state) = shared_state.lock() {
                    (
                        state.shortcut_input.clone(),
                        state.shortcut_status_msg.clone(),
                    )
                } else {
                    ("Ctrl+Shift+U".to_string(), None)
                };

                egui::Frame::none()
                    .fill(Theme::BG_TERTIARY)
                    .stroke(Stroke::new(1.0, Theme::BORDER_SUBTLE))
                    .rounding(Rounding::same(Radius::SM))
                    .inner_margin(egui::Margin::symmetric(Spacing::SM, 4.0))
                    .show(ui, |ui| {
                        let text_edit = egui::TextEdit::singleline(&mut shortcut_input)
                            .desired_width(120.0)
                            .hint_text(locale_text(
                                ui_language,
                                LocaleKey::ShortcutHintPlaceholder,
                            ));
                        let response = ui.add(text_edit);

                        if response.changed()
                            && let Ok(mut state) = shared_state.lock()
                        {
                            state.shortcut_input = shortcut_input.clone();
                        }

                        if response.lost_focus() {
                            let shortcut_str = shortcut_input.trim().to_string();
                            if !shortcut_str.is_empty() {
                                if let Some((modifiers, key)) =
                                    crate::shortcuts::parse_shortcut(&shortcut_str)
                                {
                                    let formatted = format_shortcut(modifiers, key);
                                    if let Ok(mut state) = shared_state.lock() {
                                        state.settings.global_shortcut = formatted.clone();
                                        state.shortcut_input = formatted;
                                        state.settings_changed = true;
                                        state.shortcut_status_msg = Some((
                                            locale_text(ui_language, LocaleKey::Saved).to_string(),
                                            false,
                                        ));
                                    }
                                } else if let Ok(mut state) = shared_state.lock() {
                                    state.shortcut_status_msg = Some((
                                        locale_text(ui_language, LocaleKey::InvalidFormat)
                                            .to_string(),
                                        true,
                                    ));
                                }
                            }
                        }
                    });
            });
        });

        // Render status feedback below the input using the shared helper
        let status_msg = if let Ok(state) = shared_state.lock() {
            state.shortcut_status_msg.clone()
        } else {
            None
        };
        if let Some((msg, is_error)) = &status_msg {
            ui.add_space(Spacing::XS);
            status_message(ui, msg, *is_error);
        }

        ui.add_space(4.0);
        ui.label(
            RichText::new(locale_text(ui_language, LocaleKey::ShortcutFormatHint))
                .size(FontSize::XS)
                .color(Theme::TEXT_MUTED),
        );
    });
}

/// Render Display tab for viewport
fn render_display_tab(ui: &mut egui::Ui, shared_state: &Arc<Mutex<PreferencesSharedState>>) {
    // Get current language from shared state
    let ui_language = if let Ok(state) = shared_state.lock() {
        state.settings.ui_language
    } else {
        Language::English
    };

    preferences_pane_header(
        ui,
        preferences_section_title(PreferencesTab::Display),
        preferences_section_subtitle(PreferencesTab::Display),
    );

    section_header(ui, locale_text(ui_language, LocaleKey::Appearance));

    settings_card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Menu bar display mode")
                        .size(FontSize::SM)
                        .color(Theme::TEXT_PRIMARY),
                );
                ui.label(
                    RichText::new("Choose how much menu bar detail CodexBar keeps visible.")
                        .size(FontSize::XS)
                        .color(Theme::TEXT_SECONDARY),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut selected_mode = if let Ok(state) = shared_state.lock() {
                    state.settings.menu_bar_display_mode.clone()
                } else {
                    "detailed".to_string()
                };

                egui::ComboBox::from_id_salt("menu_bar_display_mode_viewport")
                    .selected_text(match selected_mode.as_str() {
                        "minimal" => "Minimal",
                        "compact" => "Compact",
                        _ => "Detailed",
                    })
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        for (value, label) in [
                            ("minimal", "Minimal"),
                            ("compact", "Compact"),
                            ("detailed", "Detailed"),
                        ] {
                            if ui
                                .selectable_value(&mut selected_mode, value.to_string(), label)
                                .changed()
                                && let Ok(mut state) = shared_state.lock()
                            {
                                state.settings.menu_bar_display_mode = selected_mode.clone();
                                state.settings_changed = true;
                            }
                        }
                    });
            });
        });

        setting_divider(ui);

        let mut relative_time = if let Ok(state) = shared_state.lock() {
            state.settings.reset_time_relative
        } else {
            true
        };

        if setting_toggle(
            ui,
            locale_text(ui_language, LocaleKey::ResetTimeRelative),
            locale_text(ui_language, LocaleKey::ResetTimeRelativeHelper),
            &mut relative_time,
        ) && let Ok(mut state) = shared_state.lock()
        {
            state.settings.reset_time_relative = relative_time;
            state.settings_changed = true;
        }
    });

    ui.add_space(Spacing::MD);

    section_header(ui, locale_text(ui_language, LocaleKey::ShowUsageAsUsed));

    settings_card(ui, |ui| {
        let mut show_as_used = if let Ok(state) = shared_state.lock() {
            state.settings.show_as_used
        } else {
            false
        };

        if setting_toggle(
            ui,
            locale_text(ui_language, LocaleKey::ShowUsageAsUsed),
            locale_text(ui_language, LocaleKey::ShowUsageAsUsedHelper),
            &mut show_as_used,
        ) && let Ok(mut state) = shared_state.lock()
        {
            state.settings.show_as_used = show_as_used;
            state.settings_changed = true;
        }

        setting_divider(ui);

        let mut show_credits_extra = if let Ok(state) = shared_state.lock() {
            state.settings.show_credits_extra_usage
        } else {
            true
        };

        if setting_toggle(
            ui,
            locale_text(ui_language, LocaleKey::ShowCreditsExtra),
            locale_text(ui_language, LocaleKey::ShowCreditsExtraHelper),
            &mut show_credits_extra,
        ) && let Ok(mut state) = shared_state.lock()
        {
            state.settings.show_credits_extra_usage = show_credits_extra;
            state.settings_changed = true;
        }
    });

    ui.add_space(Spacing::MD);

    section_header(ui, locale_text(ui_language, LocaleKey::MergeTrayIcons));

    settings_card(ui, |ui| {
        let mut merge_icons = if let Ok(state) = shared_state.lock() {
            state.settings.merge_tray_icons
        } else {
            true
        };

        if setting_toggle(
            ui,
            locale_text(ui_language, LocaleKey::MergeTrayIcons),
            locale_text(ui_language, LocaleKey::MergeTrayIconsHelper),
            &mut merge_icons,
        ) && let Ok(mut state) = shared_state.lock()
        {
            set_merge_tray_icons(&mut state.settings, merge_icons);
            state.settings_changed = true;
        }

        setting_divider(ui);

        let mut per_provider = if let Ok(state) = shared_state.lock() {
            state.settings.tray_icon_mode == TrayIconMode::PerProvider
        } else {
            false
        };

        if setting_toggle(
            ui,
            locale_text(ui_language, LocaleKey::PerProviderTrayIcons),
            locale_text(ui_language, LocaleKey::PerProviderTrayIconsHelper),
            &mut per_provider,
        ) && let Ok(mut state) = shared_state.lock()
        {
            set_per_provider_tray_icons(&mut state.settings, per_provider);
            state.settings_changed = true;
        }
    });
}

/// Render API Keys tab for viewport
fn render_api_keys_tab(ui: &mut egui::Ui, shared_state: &Arc<Mutex<PreferencesSharedState>>) {
    // Get current language from shared state
    let ui_language = if let Ok(state) = shared_state.lock() {
        state.settings.ui_language
    } else {
        Language::English
    };

    section_header(ui, locale_text(ui_language, LocaleKey::ApiKeysTitle));

    ui.label(
        RichText::new(locale_text(ui_language, LocaleKey::ApiKeysDescription))
            .size(FontSize::SM)
            .color(Theme::TEXT_MUTED),
    );

    ui.add_space(Spacing::MD);

    // Status message
    let status_msg = if let Ok(state) = shared_state.lock() {
        state.api_key_status_msg.clone()
    } else {
        None
    };

    if let Some((msg, is_error)) = &status_msg {
        status_message(ui, msg, *is_error);
        ui.add_space(Spacing::SM);
    }

    // Get state for rendering
    let (api_keys_data, settings_data, show_input, input_provider) =
        if let Ok(state) = shared_state.lock() {
            (
                state.api_keys.clone(),
                state.settings.clone(),
                state.show_api_key_input,
                state.new_api_key_provider.clone(),
            )
        } else {
            return;
        };

    // Provider cards - one per provider
    let api_key_providers = get_api_key_providers();

    for provider_info in &api_key_providers {
        let provider_id = provider_info.id.cli_name();
        let has_key = api_keys_data.has_key(provider_id);
        let is_enabled = settings_data.enabled_providers.contains(provider_id);
        let icon = provider_icon(provider_id);
        let color = provider_color(provider_id);

        // Card with left accent bar
        let accent_color = if has_key {
            Theme::GREEN
        } else if is_enabled {
            Theme::ORANGE
        } else {
            Theme::BG_TERTIARY
        };

        egui::Frame::none()
            .fill(Theme::BG_SECONDARY)
            .rounding(Rounding::same(Radius::MD))
            .inner_margin(egui::Margin::same(0.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Left accent bar
                    let bar_rect = Rect::from_min_size(ui.cursor().min, Vec2::new(3.0, 48.0));
                    ui.painter().rect_filled(
                        bar_rect,
                        Rounding {
                            nw: Radius::MD,
                            sw: Radius::MD,
                            ne: 0.0,
                            se: 0.0,
                        },
                        accent_color,
                    );
                    ui.add_space(3.0);

                    // Content
                    ui.vertical(|ui| {
                        ui.add_space(Spacing::XS);

                        // Row 1: Icon, Name, Status badge, and Add Key button
                        ui.horizontal(|ui| {
                            ui.add_space(Spacing::XS);

                            // Try to render SVG icon from cache
                            let icon_size = 20.0;
                            VIEWPORT_ICON_CACHE.with(|cache| {
                                if let Some(texture) = cache.borrow_mut().get_icon(
                                    ui.ctx(),
                                    provider_id,
                                    icon_size as u32,
                                ) {
                                    ui.add(
                                        egui::Image::new(texture)
                                            .fit_to_exact_size(Vec2::splat(icon_size)),
                                    );
                                } else {
                                    ui.label(RichText::new(icon).size(FontSize::LG).color(color));
                                }
                            });

                            ui.add_space(Spacing::XS);
                            ui.label(
                                RichText::new(provider_info.name)
                                    .size(FontSize::MD)
                                    .color(Theme::TEXT_PRIMARY)
                                    .strong(),
                            );

                            ui.add_space(Spacing::XS);

                            if has_key {
                                badge(
                                    ui,
                                    locale_text(ui_language, LocaleKey::KeySet),
                                    Theme::GREEN,
                                );
                            } else if is_enabled {
                                egui::Frame::none()
                                    .fill(Theme::ORANGE)
                                    .rounding(Rounding::same(Radius::PILL))
                                    .inner_margin(egui::Margin::symmetric(Spacing::XS, 2.0))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(locale_text(
                                                ui_language,
                                                LocaleKey::KeyRequired,
                                            ))
                                            .size(FontSize::XS)
                                            .color(Color32::BLACK),
                                        );
                                    });
                            }

                            // Right-aligned: Add Key button
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.add_space(Spacing::XS);
                                    if !has_key
                                        && primary_button(
                                            ui,
                                            locale_text(ui_language, LocaleKey::AddKey),
                                        )
                                        && let Ok(mut state) = shared_state.lock()
                                    {
                                        state.new_api_key_provider = provider_id.to_string();
                                        state.show_api_key_input = true;
                                        state.new_api_key_value.clear();
                                    }
                                },
                            );
                        });

                        // Row 2: Env var, masked key, and actions
                        ui.horizontal(|ui| {
                            ui.add_space(Spacing::XS);

                            if let Some(env_var) = provider_info.api_key_env_var {
                                ui.label(
                                    RichText::new(format!(
                                        "{}: {}",
                                        locale_text(ui_language, LocaleKey::EnvironmentVariable),
                                        env_var
                                    ))
                                    .size(FontSize::XS)
                                    .color(Theme::TEXT_MUTED)
                                    .monospace(),
                                );
                            }

                            if has_key {
                                ui.add_space(Spacing::SM);
                                if let Some(key_info) = api_keys_data
                                    .get_all_for_display()
                                    .iter()
                                    .find(|k| k.provider_id == provider_id)
                                {
                                    ui.label(
                                        RichText::new(&key_info.masked_key)
                                            .size(FontSize::XS)
                                            .color(Theme::TEXT_MUTED)
                                            .monospace(),
                                    );
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.add_space(Spacing::XS);
                                        if small_button(
                                            ui,
                                            locale_text(ui_language, LocaleKey::Remove),
                                            Theme::RED,
                                        ) && let Ok(mut state) = shared_state.lock()
                                        {
                                            state.api_keys.remove(provider_id);
                                            let _ = state.api_keys.save();
                                            state.api_key_status_msg = Some((
                                                locale_text(ui_language, LocaleKey::ApiKeyRemoved)
                                                    .replace("{}", provider_info.name),
                                                false,
                                            ));
                                        }
                                    },
                                );
                            } else {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.add_space(Spacing::XS);
                                        if let Some(url) = provider_info.dashboard_url
                                            && text_button(
                                                ui,
                                                locale_text(ui_language, LocaleKey::GetKey),
                                                Theme::ACCENT_PRIMARY,
                                            )
                                        {
                                            let _ = open::that(url);
                                        }
                                    },
                                );
                            }
                        });

                        ui.add_space(Spacing::XS);
                    });
                });
            });

        ui.add_space(Spacing::XS);
    }

    // API Key input modal
    if show_input {
        ui.add_space(Spacing::MD);

        let provider_name = ProviderId::from_cli_name(&input_provider)
            .map(|id| id.display_name())
            .unwrap_or(&input_provider);

        egui::Frame::none()
            .fill(Theme::BG_TERTIARY)
            .stroke(Stroke::new(1.0, Theme::ACCENT_PRIMARY.gamma_multiply(0.4)))
            .rounding(Rounding::same(Radius::LG))
            .inner_margin(Spacing::LG)
            .show(ui, |ui| {
                ui.label(
                    RichText::new(
                        locale_text(ui_language, LocaleKey::EnterApiKeyFor)
                            .replace("{}", provider_name),
                    )
                    .size(FontSize::MD)
                    .color(Theme::TEXT_PRIMARY)
                    .strong(),
                );

                ui.add_space(Spacing::SM);

                // Get current value for text edit
                let mut current_value = if let Ok(state) = shared_state.lock() {
                    state.new_api_key_value.clone()
                } else {
                    String::new()
                };

                let text_edit = egui::TextEdit::singleline(&mut current_value)
                    .password(true)
                    .desired_width(ui.available_width())
                    .hint_text(locale_text(ui_language, LocaleKey::PasteApiKeyHere));
                let response = ui.add(text_edit);

                if response.changed()
                    && let Ok(mut state) = shared_state.lock()
                {
                    state.new_api_key_value = current_value.clone();
                }

                ui.add_space(Spacing::MD);

                ui.horizontal(|ui| {
                    let can_save = !current_value.trim().is_empty();

                    if ui
                        .add_enabled(
                            can_save,
                            egui::Button::new(
                                RichText::new(locale_text(ui_language, LocaleKey::Save))
                                    .size(FontSize::SM)
                                    .color(Color32::WHITE),
                            )
                            .fill(if can_save {
                                Theme::GREEN
                            } else {
                                Theme::BG_TERTIARY
                            })
                            .rounding(Rounding::same(Radius::SM))
                            .min_size(Vec2::new(80.0, 32.0)),
                        )
                        .clicked()
                        && let Ok(mut state) = shared_state.lock()
                    {
                        let provider = state.new_api_key_provider.clone();
                        let value = state.new_api_key_value.trim().to_string();
                        state.api_keys.set(&provider, &value, None);
                        if let Err(e) = state.api_keys.save() {
                            state.api_key_status_msg = Some((
                                locale_text(ui_language, LocaleKey::SaveFailed)
                                    .replace("{}", &e.to_string()),
                                true,
                            ));
                        } else {
                            state.api_key_status_msg = Some((
                                locale_text(ui_language, LocaleKey::ApiKeySaved)
                                    .replace("{}", provider_name),
                                false,
                            ));
                            state.show_api_key_input = false;
                            state.new_api_key_value.clear();
                        }
                    }

                    ui.add_space(Spacing::XS);

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(locale_text(ui_language, LocaleKey::Cancel))
                                    .size(FontSize::SM)
                                    .color(Theme::TEXT_MUTED),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::new(1.0, Theme::BORDER_SUBTLE))
                            .rounding(Rounding::same(Radius::SM)),
                        )
                        .clicked()
                        && let Ok(mut state) = shared_state.lock()
                    {
                        state.show_api_key_input = false;
                        state.new_api_key_value.clear();
                    }
                });
            });
    }
}

/// Render Cookies tab for viewport
fn render_cookies_tab(ui: &mut egui::Ui, shared_state: &Arc<Mutex<PreferencesSharedState>>) {
    let ui_language = if let Ok(state) = shared_state.lock() {
        state.settings.ui_language
    } else {
        Language::English
    };
    section_header(ui, locale_text(ui_language, LocaleKey::BrowserCookiesTitle));

    ui.label(
        RichText::new(locale_text(ui_language, LocaleKey::CookiesAutoImport))
            .size(FontSize::SM)
            .color(Theme::TEXT_MUTED),
    );

    ui.add_space(Spacing::MD);

    // Status message
    let status_msg = if let Ok(state) = shared_state.lock() {
        state.cookie_status_msg.clone()
    } else {
        None
    };

    if let Some((msg, is_error)) = &status_msg {
        status_message(ui, msg, *is_error);
        ui.add_space(Spacing::MD);
    }

    // Get saved cookies
    let saved_cookies = if let Ok(state) = shared_state.lock() {
        state.cookies.get_all_for_display()
    } else {
        Vec::new()
    };

    if !saved_cookies.is_empty() {
        section_header(ui, locale_text(ui_language, LocaleKey::SavedCookies));

        settings_card(ui, |ui| {
            let mut to_remove: Option<String> = None;
            let len = saved_cookies.len();

            for (i, cookie_info) in saved_cookies.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&cookie_info.provider)
                            .size(FontSize::MD)
                            .color(Theme::TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new(format!("· {}", &cookie_info.saved_at))
                            .size(FontSize::SM)
                            .color(Theme::TEXT_MUTED),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if small_button(ui, locale_text(ui_language, LocaleKey::Remove), Theme::RED)
                        {
                            to_remove = Some(cookie_info.provider_id.clone());
                        }
                    });
                });

                if i < len - 1 {
                    setting_divider(ui);
                }
            }

            if let Some(provider_id) = to_remove
                && let Ok(mut state) = shared_state.lock()
            {
                state.cookies.remove(&provider_id);
                let _ = state.cookies.save();
                state.cookie_status_msg = Some((
                    locale_text(ui_language, LocaleKey::CookieRemovedForProvider)
                        .replace("{}", &provider_id),
                    false,
                ));
            }
        });

        ui.add_space(Spacing::LG);
    }

    // Add manual cookie
    section_header(ui, locale_text(ui_language, LocaleKey::AddManualCookie));

    settings_card(ui, |ui| {
        // Get current provider selection
        let current_provider = if let Ok(state) = shared_state.lock() {
            state.new_cookie_provider.clone()
        } else {
            String::new()
        };

        // Provider selection row
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(locale_text(ui_language, LocaleKey::Provider))
                    .size(FontSize::MD)
                    .color(Theme::TEXT_PRIMARY),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let combo = egui::ComboBox::from_id_salt("cookie_provider_viewport")
                    .selected_text(if current_provider.is_empty() {
                        locale_text(ui_language, LocaleKey::SelectPlaceholder)
                    } else {
                        &current_provider
                    })
                    .show_ui(ui, |ui| {
                        let web_providers = ["claude", "cursor", "kimi"];
                        for provider_name in web_providers {
                            if let Some(id) = ProviderId::from_cli_name(provider_name)
                                && ui
                                    .selectable_label(
                                        current_provider == provider_name,
                                        id.display_name(),
                                    )
                                    .clicked()
                                && let Ok(mut state) = shared_state.lock()
                            {
                                state.new_cookie_provider = provider_name.to_string();
                            }
                        }
                    });
                let _ = combo;
            });
        });

        ui.add_space(Spacing::MD);
        setting_divider(ui);
        ui.add_space(Spacing::SM);

        // Cookie header label
        ui.label(
            RichText::new(locale_text(ui_language, LocaleKey::CookieHeader))
                .size(FontSize::MD)
                .color(Theme::TEXT_PRIMARY),
        );
        ui.add_space(Spacing::SM);

        // Get current cookie value
        let mut current_value = if let Ok(state) = shared_state.lock() {
            state.new_cookie_value.clone()
        } else {
            String::new()
        };

        // Styled text input
        egui::Frame::none()
            .fill(Theme::INPUT_BG)
            .stroke(Stroke::new(1.0, Theme::BORDER_SUBTLE))
            .rounding(Rounding::same(Radius::SM))
            .inner_margin(Spacing::SM)
            .show(ui, |ui| {
                let text_edit = egui::TextEdit::multiline(&mut current_value)
                    .desired_width(ui.available_width())
                    .desired_rows(4)
                    .frame(false)
                    .hint_text(locale_text(ui_language, LocaleKey::PasteHere));
                let response = ui.add(text_edit);

                if response.changed()
                    && let Ok(mut state) = shared_state.lock()
                {
                    state.new_cookie_value = current_value.clone();
                }
            });

        ui.add_space(Spacing::MD);

        // Re-fetch current provider for save button check
        let (save_provider, save_value) = if let Ok(state) = shared_state.lock() {
            (
                state.new_cookie_provider.clone(),
                state.new_cookie_value.clone(),
            )
        } else {
            (String::new(), String::new())
        };

        let can_save = !save_provider.is_empty() && !save_value.is_empty();

        if ui
            .add_enabled(
                can_save,
                egui::Button::new(
                    RichText::new(locale_text(ui_language, LocaleKey::Save))
                        .size(FontSize::SM)
                        .color(if can_save {
                            Color32::WHITE
                        } else {
                            Theme::TEXT_MUTED
                        }),
                )
                .fill(if can_save {
                    Theme::ACCENT_PRIMARY
                } else {
                    Theme::BG_TERTIARY
                })
                .stroke(if can_save {
                    Stroke::NONE
                } else {
                    Stroke::new(1.0, Theme::BORDER_SUBTLE)
                })
                .rounding(Rounding::same(Radius::MD))
                .min_size(Vec2::new(120.0, 36.0)),
            )
            .clicked()
            && let Ok(mut state) = shared_state.lock()
        {
            let provider = state.new_cookie_provider.clone();
            let value = state.new_cookie_value.clone();
            state.cookies.set(&provider, &value);
            if let Err(e) = state.cookies.save() {
                state.cookie_status_msg = Some((
                    locale_text(ui_language, LocaleKey::SaveFailed).replace("{}", &e.to_string()),
                    true,
                ));
            } else {
                let provider_name = ProviderId::from_cli_name(&provider)
                    .map(|id| id.display_name().to_string())
                    .unwrap_or_else(|| provider.clone());
                state.cookie_status_msg = Some((
                    locale_text(ui_language, LocaleKey::CookieSavedForProvider)
                        .replace("{}", &provider_name),
                    false,
                ));
                state.new_cookie_provider.clear();
                state.new_cookie_value.clear();
            }
        }
    });
}

/// Render Advanced tab for viewport
fn render_advanced_tab(ui: &mut egui::Ui, shared_state: &Arc<Mutex<PreferencesSharedState>>) {
    let ui_language = if let Ok(state) = shared_state.lock() {
        state.settings.ui_language
    } else {
        Language::English
    };
    preferences_pane_header(
        ui,
        preferences_section_title(PreferencesTab::Advanced),
        preferences_section_subtitle(PreferencesTab::Advanced),
    );

    section_header(ui, locale_text(ui_language, LocaleKey::RefreshSettings));

    settings_card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(locale_text(ui_language, LocaleKey::AutoRefreshInterval))
                    .size(FontSize::MD)
                    .color(Theme::TEXT_PRIMARY),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let current_interval = if let Ok(state) = shared_state.lock() {
                    state.settings.refresh_interval_secs
                } else {
                    60
                };

                let intervals = [0, 30, 60, 300, 600];
                let current_label = refresh_interval_text(current_interval, ui_language);

                // Style combobox to match theme
                ui.style_mut().visuals.widgets.inactive.bg_fill = Theme::BG_TERTIARY;
                ui.style_mut().visuals.widgets.inactive.weak_bg_fill = Theme::BG_TERTIARY;
                ui.style_mut().visuals.widgets.hovered.bg_fill = Theme::CARD_BG_HOVER;
                ui.style_mut().visuals.widgets.active.bg_fill = Theme::CARD_BG;
                ui.style_mut().visuals.widgets.inactive.rounding = Rounding::same(Radius::SM);
                ui.style_mut().visuals.widgets.hovered.rounding = Rounding::same(Radius::SM);
                ui.style_mut().visuals.widgets.active.rounding = Rounding::same(Radius::SM);

                egui::ComboBox::from_id_salt("refresh_interval")
                    .selected_text(current_label)
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for value in intervals {
                            let label = refresh_interval_text(value, ui_language);
                            if ui
                                .selectable_label(current_interval == value, label)
                                .clicked()
                                && let Ok(mut state) = shared_state.lock()
                            {
                                state.settings.refresh_interval_secs = value;
                                state.settings_changed = true;
                            }
                        }
                    });
            });
        });
    });

    ui.add_space(Spacing::MD);
    section_header(ui, locale_text(ui_language, LocaleKey::PrivacyTitle));

    settings_card(ui, |ui| {
        let mut hide_personal_info = if let Ok(state) = shared_state.lock() {
            state.settings.hide_personal_info
        } else {
            false
        };

        if setting_toggle(
            ui,
            locale_text(ui_language, LocaleKey::HidePersonalInfo),
            locale_text(ui_language, LocaleKey::HidePersonalInfoHelper),
            &mut hide_personal_info,
        ) && let Ok(mut state) = shared_state.lock()
        {
            state.settings.hide_personal_info = hide_personal_info;
            state.settings_changed = true;
        }
    });

    ui.add_space(Spacing::MD);
    section_header(ui, locale_text(ui_language, LocaleKey::Fun));

    settings_card(ui, |ui| {
        let mut surprise = if let Ok(state) = shared_state.lock() {
            state.settings.surprise_animations
        } else {
            false
        };

        if setting_toggle(
            ui,
            locale_text(ui_language, LocaleKey::Fun),
            locale_text(ui_language, LocaleKey::SurpriseAnimationsHelper),
            &mut surprise,
        ) && let Ok(mut state) = shared_state.lock()
        {
            state.settings.surprise_animations = surprise;
            state.settings_changed = true;
        }
    });
}

/// Render About tab for viewport
fn render_about_tab(ui: &mut egui::Ui, shared_state: &Arc<Mutex<PreferencesSharedState>>) {
    let ui_language = if let Ok(state) = shared_state.lock() {
        state.settings.ui_language
    } else {
        Language::English
    };
    let git_commit = option_env!("GIT_COMMIT").unwrap_or("dev");
    let build_date = option_env!("BUILD_DATE").unwrap_or("unknown");

    ui.add_space(Spacing::LG);
    ui.vertical_centered(|ui| {
        egui::Frame::none()
            .fill(Theme::BG_SECONDARY)
            .stroke(Stroke::new(1.0, Theme::ACCENT_PRIMARY.gamma_multiply(0.22)))
            .rounding(Rounding::same(16.0))
            .inner_margin(egui::Margin::same(18.0))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("◐")
                            .size(34.0)
                            .color(Theme::ACCENT_PRIMARY.gamma_multiply(1.08)),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("CodexBar")
                            .size(FontSize::XL)
                            .color(Theme::TEXT_PRIMARY)
                            .strong(),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                            .size(FontSize::SM)
                            .color(Theme::TEXT_SECONDARY),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(format!("Built {}", build_date))
                            .size(FontSize::XS)
                            .color(Theme::TEXT_MUTED),
                    );
                });
            });

        ui.add_space(Spacing::MD);
        ui.label(
            RichText::new(locale_text(ui_language, LocaleKey::AboutDescription))
                .size(FontSize::SM)
                .color(Theme::TEXT_SECONDARY),
        );
        ui.label(
            RichText::new(locale_text(ui_language, LocaleKey::AboutDescriptionLine2))
                .size(FontSize::SM)
                .color(Theme::TEXT_SECONDARY),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new("May your tokens never run out.")
                .size(FontSize::XS)
                .color(Theme::TEXT_MUTED),
        );
    });

    ui.add_space(Spacing::LG);
    section_header(ui, locale_text(ui_language, LocaleKey::Links));
    settings_card(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                if ui
                    .link(locale_text(ui_language, LocaleKey::ViewOnGitHub))
                    .clicked()
                {
                    let _ = open::that("https://github.com/Finesssee/Win-CodexBar");
                }
                ui.label(RichText::new("·").color(Theme::TEXT_DIM));
                if ui
                    .link(locale_text(ui_language, LocaleKey::OriginalMacOSVersion))
                    .clicked()
                {
                    let _ = open::that("https://github.com/steipete/CodexBar");
                }
            });
            ui.add_space(Spacing::SM);
            ui.horizontal(|ui| {
                if text_button(
                    ui,
                    locale_text(ui_language, LocaleKey::SubmitIssue),
                    Theme::ACCENT_PRIMARY,
                ) {
                    let _ = open::that("https://github.com/Finesssee/Win-CodexBar/issues");
                }
                ui.add_space(Spacing::SM);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(locale_text(ui_language, LocaleKey::TrayCheckForUpdates))
                                .size(FontSize::SM)
                                .color(Theme::TEXT_PRIMARY),
                        )
                        .stroke(Stroke::new(1.0, Theme::BORDER_SUBTLE))
                        .fill(Theme::CARD_BG)
                        .rounding(Rounding::same(Radius::SM)),
                    )
                    .clicked()
                {
                    let _ = open::that("https://github.com/Finesssee/Win-CodexBar/releases");
                }
            });
            ui.add_space(Spacing::XS);
            ui.label(
                RichText::new("Updates and issues open in your browser.")
                    .size(FontSize::XS)
                    .color(Theme::TEXT_MUTED),
            );
        });
    });

    ui.add_space(Spacing::MD);
    section_header(ui, locale_text(ui_language, LocaleKey::BuildInfo));
    settings_card(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(locale_text(ui_language, LocaleKey::MaintainedBy))
                    .size(FontSize::SM)
                    .color(Theme::TEXT_PRIMARY),
            );
            ui.add_space(Spacing::XS);
            ui.label(
                RichText::new("MIT License")
                    .size(FontSize::SM)
                    .color(Theme::TEXT_MUTED),
            );
            ui.add_space(Spacing::SM);
            ui.label(
                RichText::new(format!(
                    "{}: {}",
                    locale_text(ui_language, LocaleKey::CommitLabel),
                    git_commit
                ))
                .size(FontSize::SM)
                .color(Theme::TEXT_SECONDARY)
                .monospace(),
            );
            ui.label(
                RichText::new(format!(
                    "{}: {}",
                    locale_text(ui_language, LocaleKey::BuildDateLabel),
                    build_date
                ))
                .size(FontSize::SM)
                .color(Theme::TEXT_SECONDARY),
            );
        });
    });
}

/// Render Providers tab for viewport
fn render_providers_tab(
    ui: &mut egui::Ui,
    _available_height: f32,
    shared_state: &Arc<Mutex<PreferencesSharedState>>,
) {
    let ui_language = if let Ok(state) = shared_state.lock() {
        state.settings.ui_language
    } else {
        Language::English
    };
    section_header(ui, locale_text(ui_language, LocaleKey::EnabledProviders));

    let providers = ProviderId::all();

    for provider_id in providers {
        let is_enabled = if let Ok(state) = shared_state.lock() {
            state
                .settings
                .enabled_providers
                .contains(provider_id.cli_name())
        } else {
            true
        };

        settings_card(ui, |ui| {
            ui.horizontal(|ui| {
                let brand_color = provider_color(provider_id.cli_name());

                ui.label(
                    RichText::new(provider_icon(provider_id.cli_name()))
                        .size(FontSize::LG)
                        .color(brand_color),
                );

                ui.add_space(8.0);

                ui.label(
                    RichText::new(provider_id.display_name())
                        .size(FontSize::MD)
                        .color(Theme::TEXT_PRIMARY),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut enabled = is_enabled;
                    if switch_toggle(
                        ui,
                        egui::Id::new(format!("provider_{}", provider_id.cli_name())),
                        &mut enabled,
                    ) && let Ok(mut state) = shared_state.lock()
                    {
                        let name = provider_id.cli_name().to_string();
                        if enabled {
                            state.settings.enabled_providers.insert(name);
                        } else {
                            state.settings.enabled_providers.remove(&name);
                        }
                        state.settings_changed = true;
                    }
                });
            });
        });

        ui.add_space(Spacing::XS);
    }
}

// ════════════════════════════════════════════════════════════════════════════════
// HELPER COMPONENTS - Refined, reusable UI elements
// ════════════════════════════════════════════════════════════════════════════════

/// Section header - subtle, uppercase
fn section_header(ui: &mut egui::Ui, text: &str) {
    let display_text = if text.is_ascii() {
        text.to_uppercase()
    } else {
        text.to_string()
    };
    ui.add_space(Spacing::LG);
    ui.label(
        RichText::new(display_text)
            .size(FontSize::XS)
            .color(Theme::TEXT_SECTION)
            .strong(),
    );
    ui.add_space(Spacing::SM);
}

fn preferences_pane_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(
        RichText::new(title)
            .size(FontSize::LG)
            .color(Theme::TEXT_PRIMARY)
            .strong(),
    );
    ui.add_space(2.0);
    ui.label(
        RichText::new(subtitle)
            .size(FontSize::SM)
            .color(Theme::TEXT_SECONDARY),
    );
    ui.add_space(Spacing::SM);
}

fn render_preferences_section_selector(
    ui: &mut egui::Ui,
    shared_state: &Arc<Mutex<PreferencesSharedState>>,
    current: PreferencesTab,
) {
    let sections = [
        PreferencesTab::General,
        PreferencesTab::Display,
        PreferencesTab::Advanced,
    ];

    egui::Frame::none()
        .fill(Theme::NAV_BG.gamma_multiply(0.84))
        .stroke(Stroke::new(1.0, Theme::BORDER_SUBTLE.gamma_multiply(0.56)))
        .rounding(Rounding::same(Radius::MD))
        .inner_margin(egui::Margin::symmetric(5.0, 5.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for section in sections {
                    let is_selected = current == section;
                    let button = egui::Button::new(
                        RichText::new(preferences_section_title(section))
                            .size(FontSize::SM)
                            .color(if is_selected {
                                Theme::TEXT_PRIMARY
                            } else {
                                Theme::TEXT_SECONDARY
                            }),
                    )
                    .fill(if is_selected {
                        Theme::BG_SECONDARY
                    } else {
                        Color32::TRANSPARENT
                    })
                    .stroke(if is_selected {
                        Stroke::new(1.0, Theme::ACCENT_PRIMARY.gamma_multiply(0.34))
                    } else {
                        Stroke::NONE
                    })
                    .rounding(Rounding::same(Radius::SM))
                    .min_size(Vec2::new(90.0, 29.0));

                    if ui.add(button).clicked()
                        && let Ok(mut state) = shared_state.lock()
                    {
                        state.preferences_section = section;
                    }
                }
            });
        });
}

/// Settings card container - light grouping via spacing, no heavy chrome
fn settings_card(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(Theme::CARD_BG.gamma_multiply(0.24))
        .stroke(Stroke::new(1.0, Theme::BORDER_SUBTLE.gamma_multiply(0.26)))
        .rounding(Rounding::same(Radius::MD))
        .inner_margin(egui::Margin::symmetric(Spacing::MD, Spacing::SM))
        .show(ui, content);
}

/// Divider line between settings in a card
fn setting_divider(ui: &mut egui::Ui) {
    ui.add_space(Spacing::SM);
    let rect = Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width(), 1.0));
    ui.painter()
        .rect_filled(rect, 0.0, Theme::SEPARATOR.gamma_multiply(0.62));
    ui.add_space(Spacing::SM + 1.0);
}

/// iOS-style switch toggle component
/// Size: 36x20 pixels with animated knob position
fn switch_toggle_visual(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    value: &mut bool,
    enabled: bool,
) -> bool {
    let desired_size = Vec2::new(36.0, 20.0);
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(desired_size, sense);

    let mut changed = false;
    if enabled && response.clicked() {
        *value = !*value;
        changed = true;
    }

    // Animate the knob position
    let animation_progress = ui.ctx().animate_bool_responsive(egui::Id::new(id), *value);

    // Track colors
    let mut track_color = if animation_progress > 0.5 {
        Theme::ACCENT_PRIMARY
    } else {
        Theme::BG_TERTIARY
    };
    let mut knob_color = Color32::WHITE;
    let mut track_stroke = Stroke::new(1.0, Color32::TRANSPARENT);

    if !enabled {
        track_color = track_color.gamma_multiply(0.42);
        knob_color = Color32::from_rgba_unmultiplied(255, 255, 255, 160);
        track_stroke = Stroke::new(1.0, Theme::BORDER_SUBTLE.gamma_multiply(0.45));
    } else if response.hovered() {
        track_stroke = Stroke::new(1.0, Theme::BORDER_SUBTLE.gamma_multiply(0.65));
    }

    // Draw track (rounded rectangle)
    let track_rounding = rect.height() / 2.0;
    ui.painter().rect(
        rect,
        Rounding::same(track_rounding),
        track_color,
        track_stroke,
    );

    // Knob properties
    let knob_margin = 2.0;
    let knob_diameter = rect.height() - knob_margin * 2.0;
    let knob_travel = rect.width() - knob_diameter - knob_margin * 2.0;

    // Interpolate knob position
    let knob_x = rect.min.x + knob_margin + (knob_travel * animation_progress);
    let knob_center = egui::pos2(knob_x + knob_diameter / 2.0, rect.center().y);

    // Draw knob (white circle)
    ui.painter()
        .circle_filled(knob_center, knob_diameter / 2.0, knob_color);

    changed
}

fn switch_toggle(ui: &mut egui::Ui, id: impl std::hash::Hash, value: &mut bool) -> bool {
    switch_toggle_visual(ui, id, value, true)
}

/// Toggle setting row - iOS-style switch on right, title and subtitle on left
fn setting_toggle(ui: &mut egui::Ui, title: &str, subtitle: &str, value: &mut bool) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(title)
                    .size(FontSize::SM)
                    .color(Theme::TEXT_PRIMARY),
            );
            if !subtitle.is_empty() {
                ui.label(
                    RichText::new(subtitle)
                        .size(FontSize::XS)
                        .color(Theme::TEXT_SECONDARY),
                );
            }
        });

        // Switch on the right
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if switch_toggle(ui, format!("switch_{}", title), value) {
                changed = true;
            }
        });
    });

    changed
}

/// Status message banner
fn status_message(ui: &mut egui::Ui, msg: &str, is_error: bool) {
    let (bg_color, text_color, icon) = if is_error {
        (
            Color32::from_rgba_unmultiplied(224, 80, 72, 25),
            Theme::RED,
            "✕",
        )
    } else {
        (
            Color32::from_rgba_unmultiplied(74, 198, 104, 25),
            Theme::GREEN,
            "✓",
        )
    };

    egui::Frame::none()
        .fill(bg_color)
        .rounding(Rounding::same(Radius::SM))
        .inner_margin(Spacing::SM)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).size(FontSize::SM).color(text_color));
                ui.add_space(Spacing::XS);
                ui.label(RichText::new(msg).size(FontSize::SM).color(text_color));
            });
        });
}

/// Small badge
fn badge(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::none()
        .fill(color.gamma_multiply(0.15))
        .rounding(Rounding::same(Radius::XS))
        .inner_margin(egui::Margin::symmetric(Spacing::XS, 2.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(FontSize::XS).color(color));
        });
}

/// Small text button
fn small_button(ui: &mut egui::Ui, text: &str, color: Color32) -> bool {
    ui.add(
        egui::Button::new(RichText::new(text).size(FontSize::SM).color(color))
            .fill(color.gamma_multiply(0.1))
            .rounding(Rounding::same(Radius::SM)),
    )
    .clicked()
}

/// Text-only button (no background)
fn text_button(ui: &mut egui::Ui, text: &str, color: Color32) -> bool {
    ui.add(
        egui::Button::new(RichText::new(text).size(FontSize::SM).color(color))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE),
    )
    .clicked()
}

/// Primary action button
fn primary_button(ui: &mut egui::Ui, text: &str) -> bool {
    ui.add(
        egui::Button::new(RichText::new(text).size(FontSize::SM).color(Color32::WHITE))
            .fill(Theme::ACCENT_PRIMARY)
            .rounding(Rounding::same(Radius::SM)),
    )
    .clicked()
}

#[cfg(test)]
mod tests {
    use super::{
        PreferencesSharedState, PreferencesTab, PreferencesWindow, active_provider_sidebar_style,
        alibaba_cookie_source_label, alibaba_region_label, amp_cookie_source_label,
        augment_cookie_source_label, compact_credentials_path, cursor_cookie_source_label,
        factory_cookie_source_label, gemini_cli_credentials_path, kimi_cookie_source_label,
        minimax_cookie_source_label, minimax_region_label, ollama_cookie_source_label,
        opencode_cookie_source_label, provider_detail_chrome, provider_detail_display_text,
        provider_detail_max_content_width, provider_detail_source_display,
        provider_detail_status_value, provider_detail_subtitle, provider_detail_text_chrome,
        provider_sidebar_display_lines, provider_sidebar_subtitle, providers_surface_palette,
        render_about_tab, set_merge_tray_icons, set_per_provider_tray_icons, settings_nav_chrome,
        should_show_token_accounts_section, shows_shared_provider_settings,
        vertexai_credentials_path, zai_region_label,
    };
    use crate::browser::detection::BrowserType;
    use crate::core::{ProviderAccountData, ProviderId, WidgetProviderEntry};
    use crate::settings::Language;
    use crate::settings::{Settings, TrayIconMode};
    use chrono::{Duration, Utc};
    use eframe::egui::{self, CentralPanel};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    #[test]
    fn enabling_merge_forces_single_tray_mode() {
        let mut settings = Settings {
            tray_icon_mode: TrayIconMode::PerProvider,
            ..Settings::default()
        };

        set_merge_tray_icons(&mut settings, true);

        assert!(settings.merge_tray_icons);
        assert_eq!(settings.tray_icon_mode, TrayIconMode::Single);
    }

    #[test]
    fn enabling_per_provider_clears_merge_mode() {
        let mut settings = Settings {
            merge_tray_icons: true,
            ..Settings::default()
        };

        set_per_provider_tray_icons(&mut settings, true);

        assert!(!settings.merge_tray_icons);
        assert_eq!(settings.tray_icon_mode, TrayIconMode::PerProvider);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn test_tab_parser_supports_top_level_settings_tabs() {
        assert_eq!(
            PreferencesTab::from_test_label("preferences"),
            Some(PreferencesTab::Preferences)
        );
        assert_eq!(
            PreferencesTab::from_test_label("accounts"),
            Some(PreferencesTab::Accounts)
        );
        assert_eq!(
            PreferencesTab::from_test_label("shortcuts"),
            Some(PreferencesTab::Shortcuts)
        );
        assert_eq!(
            PreferencesTab::from_test_label("about"),
            Some(PreferencesTab::About)
        );
    }

    #[test]
    fn viewport_about_tab_no_longer_renders_hero_icon() {
        let ctx = egui::Context::default();
        let shared_state = PreferencesWindow::default().shared_state.clone();

        ctx.begin_pass(egui::RawInput::default());
        CentralPanel::default().show(&ctx, |ui| {
            render_about_tab(ui, &shared_state);
        });
        let full_output = ctx.end_pass();
        let rendered = format!("{:?}", full_output.shapes);

        assert!(
            !rendered.contains('◆'),
            "about viewport still renders old hero icon"
        );
        assert!(rendered.contains("CodexBar"));
    }

    #[test]
    fn active_provider_sidebar_style_uses_layered_selection_for_mac_like_sidebar() {
        let style = active_provider_sidebar_style();

        assert_eq!(
            style.frame_fill,
            Some(eframe::egui::Color32::from_rgba_unmultiplied(
                255, 255, 255, 8
            ))
        );
        assert_eq!(
            style.frame_stroke,
            Some(eframe::egui::Stroke::new(
                1.0,
                super::Theme::BORDER_SUBTLE.gamma_multiply(0.56)
            ))
        );
        assert_eq!(style.inner_margin, super::Spacing::SM);
        assert_eq!(style.item_spacing_y, 0.0);
        assert_eq!(style.row_height, 58.0);
        assert_eq!(style.row_corner_radius, 7.0);
        assert_eq!(
            style.selected_fill,
            eframe::egui::Color32::from_rgba_unmultiplied(255, 255, 255, 11)
        );
        assert_eq!(
            style.selected_stroke,
            eframe::egui::Stroke::new(
                1.0,
                eframe::egui::Color32::from_rgba_unmultiplied(255, 255, 255, 18)
            )
        );
        assert_eq!(
            style.hover_fill,
            eframe::egui::Color32::from_rgba_unmultiplied(255, 255, 255, 4)
        );
    }

    #[test]
    fn providers_surface_palette_adds_subtle_layer_separation() {
        let palette = providers_surface_palette();

        assert_eq!(
            palette.shell_fill,
            eframe::egui::Color32::from_rgb(56, 56, 64)
        );
        assert_eq!(
            palette.content_fill,
            eframe::egui::Color32::from_rgb(54, 54, 62)
        );
        assert_eq!(
            palette.detail_fill,
            eframe::egui::Color32::from_rgb(58, 58, 66)
        );
        assert_eq!(
            palette.detail_stroke,
            eframe::egui::Stroke::new(1.0, super::Theme::BORDER_SUBTLE.gamma_multiply(0.24))
        );
    }

    #[test]
    fn settings_nav_chrome_keeps_toolbar_pill_subtle() {
        let chrome = settings_nav_chrome();

        assert_eq!(
            chrome.selected_fill,
            eframe::egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10)
        );
        assert_eq!(
            chrome.selected_stroke,
            eframe::egui::Stroke::new(
                1.0,
                eframe::egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20)
            )
        );
        assert_eq!(
            chrome.hover_fill,
            eframe::egui::Color32::from_rgba_unmultiplied(255, 255, 255, 4)
        );
    }

    #[test]
    fn provider_detail_text_chrome_keeps_right_pane_readable() {
        let chrome = provider_detail_text_chrome();

        assert_eq!(
            chrome.subtitle,
            super::Theme::TEXT_SECONDARY.gamma_multiply(1.16)
        );
        assert_eq!(
            chrome.section_title,
            super::Theme::TEXT_PRIMARY.gamma_multiply(0.84)
        );
        assert_eq!(
            chrome.helper,
            super::Theme::TEXT_SECONDARY.gamma_multiply(1.08)
        );
        assert_eq!(
            chrome.info_label,
            super::Theme::TEXT_SECONDARY.gamma_multiply(1.12)
        );
        assert_eq!(
            chrome.secondary_value,
            super::Theme::TEXT_SECONDARY.gamma_multiply(1.02)
        );
    }

    #[test]
    fn provider_detail_chrome_uses_roomier_controls_and_spacing() {
        let chrome = provider_detail_chrome();

        assert_eq!(
            chrome.control_fill,
            eframe::egui::Color32::from_rgba_unmultiplied(255, 255, 255, 5)
        );
        assert_eq!(
            chrome.control_fill_hover,
            eframe::egui::Color32::from_rgba_unmultiplied(255, 255, 255, 8)
        );
        assert_eq!(
            chrome.control_fill_active,
            eframe::egui::Color32::from_rgba_unmultiplied(255, 255, 255, 11)
        );
        assert_eq!(
            chrome.control_stroke,
            eframe::egui::Stroke::new(1.0, super::Theme::BORDER_SUBTLE.gamma_multiply(0.28))
        );
        assert_eq!(chrome.control_inner_margin_x, 8.0);
        assert_eq!(chrome.control_inner_margin_y, 0.0);
        assert_eq!(chrome.info_grid_spacing_x, 14.0);
        assert_eq!(chrome.info_grid_spacing_y, 6.0);
        assert_eq!(chrome.section_gap, 12.0);
        assert_eq!(chrome.detail_label_width, 92.0);
        assert_eq!(chrome.picker_label_width, 92.0);
        assert_eq!(chrome.metric_bar_width, 220.0);
        assert_eq!(chrome.refresh_button_size, 24.0);
    }

    #[test]
    fn provider_detail_max_content_width_matches_roomier_mac_like_layout() {
        assert_eq!(provider_detail_max_content_width(), 404.0);
    }

    #[test]
    fn provider_sidebar_subtitle_uses_source_hint_for_disabled_rows() {
        let subtitle =
            provider_sidebar_subtitle(ProviderId::Copilot, false, None, None, Language::English);

        assert_eq!(subtitle, "Disabled — github api\nusage not fetched yet");
        assert_eq!(
            provider_sidebar_subtitle(ProviderId::MiniMax, false, None, None, Language::English),
            "Disabled — auto\nusage not fetched yet"
        );
    }

    #[test]
    fn provider_sidebar_subtitle_uses_detection_failure_for_enabled_missing_snapshot() {
        let subtitle =
            provider_sidebar_subtitle(ProviderId::Codex, true, None, None, Language::English);

        assert_eq!(subtitle, "not detected\nlast fetch failed");
    }

    #[test]
    fn provider_sidebar_subtitle_uses_not_fetched_status_for_empty_enabled_entry() {
        let entry = WidgetProviderEntry::new(ProviderId::Cursor, Utc::now() - Duration::minutes(3));

        let subtitle = provider_sidebar_subtitle(
            ProviderId::Cursor,
            true,
            Some(&entry),
            None,
            Language::English,
        );

        assert_eq!(subtitle, "web\nusage not fetched yet");
    }

    #[test]
    fn provider_detail_subtitle_uses_detection_failure_for_missing_snapshot() {
        let subtitle = provider_detail_subtitle(
            ProviderId::Codex,
            true,
            None,
            None,
            "auto",
            "Never updated",
            Language::English,
        );

        assert_eq!(subtitle, "not detected • last fetch failed");
    }

    #[test]
    fn provider_detail_subtitle_uses_disabled_usage_copy_for_disabled_provider() {
        let subtitle = provider_detail_subtitle(
            ProviderId::Copilot,
            false,
            None,
            None,
            "github api",
            "Never updated",
            Language::English,
        );

        assert_eq!(subtitle, "github api • usage not fetched yet");
    }

    #[test]
    fn provider_sidebar_subtitle_uses_provider_specific_disabled_hint_when_needed() {
        let subtitle =
            provider_sidebar_subtitle(ProviderId::Claude, false, None, None, Language::English);
        assert_eq!(
            subtitle,
            "Disabled — claude not detected\nusage not fetched yet"
        );

        let cursor_subtitle =
            provider_sidebar_subtitle(ProviderId::Cursor, false, None, None, Language::English);
        assert_eq!(cursor_subtitle, "Disabled — web\nusage not fetched yet");
    }

    #[test]
    fn provider_sidebar_subtitle_uses_runtime_error_for_kiro_disabled_detail() {
        let subtitle = provider_sidebar_subtitle(
            ProviderId::Kiro,
            false,
            None,
            Some("kiro-cli: No such file or directory"),
            Language::English,
        );

        assert_eq!(
            subtitle,
            "Disabled — kiro env: kiro-cli: No such file or directory\nusage not fetched yet"
        );
    }

    #[test]
    fn provider_detail_subtitle_uses_provider_specific_disabled_hint_when_needed() {
        let subtitle = provider_detail_subtitle(
            ProviderId::Claude,
            false,
            None,
            None,
            "auto",
            "Never updated",
            Language::English,
        );

        assert_eq!(subtitle, "claude not detected • usage not fetched yet");
    }

    #[test]
    fn provider_detail_subtitle_uses_runtime_error_for_kiro_disabled_detail() {
        let subtitle = provider_detail_subtitle(
            ProviderId::Kiro,
            false,
            None,
            Some("kiro-cli: No such file or directory"),
            "kiro env",
            "Never updated",
            Language::English,
        );

        assert_eq!(
            subtitle,
            "kiro env: kiro-cli: No such file or directory • usage not fetched yet"
        );
    }

    #[test]
    fn provider_detail_status_value_uses_runtime_error_for_disabled_kiro() {
        let status = provider_detail_status_value(
            ProviderId::Kiro,
            false,
            None,
            Some("kiro-cli: No such file or directory"),
            Language::English,
        );

        assert_eq!(status, "kiro env: kiro-cli: No such file or directory");
    }

    #[test]
    fn provider_detail_status_value_uses_last_fetch_failed_for_enabled_missing_snapshot() {
        let status =
            provider_detail_status_value(ProviderId::Codex, true, None, None, Language::English);

        assert_eq!(status, "last fetch failed");
    }

    #[test]
    fn provider_detail_status_value_uses_disabled_for_generic_disabled_provider() {
        let status =
            provider_detail_status_value(ProviderId::Claude, false, None, None, Language::English);

        assert_eq!(status, "claude not detected");
    }

    #[test]
    fn provider_detail_status_value_uses_provider_specific_hint_without_runtime_error() {
        let status =
            provider_detail_status_value(ProviderId::Kiro, false, None, None, Language::English);

        assert!(status.starts_with("kiro env"));
    }

    #[test]
    fn shared_provider_settings_hide_for_dedicated_api_panes() {
        assert!(!shows_shared_provider_settings(ProviderId::Infini));
        assert!(!shows_shared_provider_settings(ProviderId::Synthetic));
        assert!(shows_shared_provider_settings(ProviderId::Claude));
        assert!(shows_shared_provider_settings(ProviderId::Cursor));
    }

    #[test]
    fn shared_provider_settings_stay_disabled_for_dedicated_provider_family() {
        let dedicated_provider_family = [
            ProviderId::Gemini,
            ProviderId::Antigravity,
            ProviderId::OpenCode,
            ProviderId::MiniMax,
            ProviderId::Factory,
            ProviderId::Kimi,
            ProviderId::Copilot,
            ProviderId::Alibaba,
            ProviderId::Amp,
            ProviderId::Augment,
            ProviderId::Infini,
            ProviderId::JetBrains,
            ProviderId::KimiK2,
            ProviderId::NanoGPT,
            ProviderId::Ollama,
            ProviderId::OpenRouter,
            ProviderId::Synthetic,
            ProviderId::VertexAI,
            ProviderId::Warp,
            ProviderId::Zai,
        ];

        for provider_id in dedicated_provider_family {
            assert!(
                !shows_shared_provider_settings(provider_id),
                "{provider_id:?} should not render shared provider settings"
            );
        }
    }

    #[test]
    fn provider_sidebar_display_lines_clamp_to_two_visual_lines() {
        let (primary, secondary) = provider_sidebar_display_lines(
            "Disabled — kiro env: kiro-cli: No such file or directory\nusage not fetched yet",
        );

        assert_eq!(primary, "Disabled — kiro env: kiro-cli: No such …");
        assert_eq!(secondary.as_deref(), Some("usage not fetched yet"));
    }

    #[test]
    fn provider_detail_display_text_breaks_detail_subtitle_into_two_lines() {
        let display = provider_detail_display_text(
            "kiro env: kiro-cli: No such file or directory • usage not fetched yet",
        );

        assert_eq!(
            display,
            "kiro env: kiro-cli: No such file or directory\nusage not fetched yet"
        );
    }

    #[test]
    fn cursor_cookie_source_label_matches_supported_modes() {
        assert_eq!(
            cursor_cookie_source_label("auto", Language::English),
            "Automatic"
        );
        assert_eq!(
            cursor_cookie_source_label("manual", Language::English),
            "Manual"
        );
    }

    #[test]
    fn token_accounts_section_shows_for_cursor_manual_cookie_mode() {
        let shared_state = Arc::new(Mutex::new(PreferencesSharedState {
            is_open: false,
            active_tab: PreferencesTab::Providers,
            settings: Settings {
                cursor_cookie_source: "manual".to_string(),
                ..Settings::default()
            },
            settings_changed: false,
            cookies: Default::default(),
            new_cookie_provider: String::new(),
            new_cookie_value: String::new(),
            cookie_status_msg: None,
            api_keys: Default::default(),
            new_api_key_provider: String::new(),
            new_api_key_value: String::new(),
            show_api_key_input: false,
            api_key_status_msg: None,
            selected_provider: Some(ProviderId::Cursor),
            selected_browser: Some(BrowserType::Chrome),
            browser_import_status: None,
            refresh_requested: false,
            cached_snapshot: None,
            runtime_provider_errors: HashMap::new(),
            token_accounts: HashMap::new(),
            new_account_label: String::new(),
            new_account_token: String::new(),
            show_add_account_input: false,
            token_account_status_msg: None,
            shortcut_input: String::new(),
            shortcut_status_msg: None,
            #[cfg(debug_assertions)]
            debug_tab_targets: Vec::new(),
            #[cfg(debug_assertions)]
            debug_viewport_outer_rect: None,
            #[cfg(debug_assertions)]
            pending_screenshot_path: None,
            #[cfg(debug_assertions)]
            pending_screenshot_delay_frames: 0,
            #[cfg(debug_assertions)]
            pending_screenshot_attempts: 0,
        }));

        assert!(should_show_token_accounts_section(
            ProviderId::Cursor,
            &shared_state
        ));
    }

    #[test]
    fn opencode_cookie_source_label_matches_supported_modes() {
        assert_eq!(
            opencode_cookie_source_label("auto", Language::English),
            "Automatic"
        );
        assert_eq!(
            opencode_cookie_source_label("manual", Language::English),
            "Manual"
        );
    }

    #[test]
    fn token_accounts_section_shows_for_opencode_manual_cookie_mode() {
        let shared_state = Arc::new(Mutex::new(PreferencesSharedState {
            is_open: false,
            active_tab: PreferencesTab::Providers,
            settings: Settings {
                opencode_cookie_source: "manual".to_string(),
                ..Settings::default()
            },
            settings_changed: false,
            cookies: Default::default(),
            new_cookie_provider: String::new(),
            new_cookie_value: String::new(),
            cookie_status_msg: None,
            api_keys: Default::default(),
            new_api_key_provider: String::new(),
            new_api_key_value: String::new(),
            show_api_key_input: false,
            api_key_status_msg: None,
            selected_provider: Some(ProviderId::OpenCode),
            selected_browser: Some(BrowserType::Chrome),
            browser_import_status: None,
            refresh_requested: false,
            cached_snapshot: None,
            runtime_provider_errors: HashMap::new(),
            token_accounts: HashMap::new(),
            new_account_label: String::new(),
            new_account_token: String::new(),
            show_add_account_input: false,
            token_account_status_msg: None,
            shortcut_input: String::new(),
            shortcut_status_msg: None,
            #[cfg(debug_assertions)]
            debug_tab_targets: Vec::new(),
            #[cfg(debug_assertions)]
            debug_viewport_outer_rect: None,
            #[cfg(debug_assertions)]
            pending_screenshot_path: None,
            #[cfg(debug_assertions)]
            pending_screenshot_delay_frames: 0,
            #[cfg(debug_assertions)]
            pending_screenshot_attempts: 0,
        }));

        assert!(should_show_token_accounts_section(
            ProviderId::OpenCode,
            &shared_state
        ));
    }

    #[test]
    fn factory_cookie_source_label_matches_supported_modes() {
        assert_eq!(
            factory_cookie_source_label("auto", Language::English),
            "Automatic"
        );
        assert_eq!(
            factory_cookie_source_label("manual", Language::English),
            "Manual"
        );
    }

    #[test]
    fn token_accounts_section_shows_for_factory_manual_cookie_mode() {
        let shared_state = Arc::new(Mutex::new(PreferencesSharedState {
            is_open: false,
            active_tab: PreferencesTab::Providers,
            settings: Settings {
                factory_cookie_source: "manual".to_string(),
                ..Settings::default()
            },
            settings_changed: false,
            cookies: Default::default(),
            new_cookie_provider: String::new(),
            new_cookie_value: String::new(),
            cookie_status_msg: None,
            api_keys: Default::default(),
            new_api_key_provider: String::new(),
            new_api_key_value: String::new(),
            show_api_key_input: false,
            api_key_status_msg: None,
            selected_provider: Some(ProviderId::Factory),
            selected_browser: Some(BrowserType::Chrome),
            browser_import_status: None,
            refresh_requested: false,
            cached_snapshot: None,
            runtime_provider_errors: HashMap::new(),
            token_accounts: HashMap::new(),
            new_account_label: String::new(),
            new_account_token: String::new(),
            show_add_account_input: false,
            token_account_status_msg: None,
            shortcut_input: String::new(),
            shortcut_status_msg: None,
            #[cfg(debug_assertions)]
            debug_tab_targets: Vec::new(),
            #[cfg(debug_assertions)]
            debug_viewport_outer_rect: None,
            #[cfg(debug_assertions)]
            pending_screenshot_path: None,
            #[cfg(debug_assertions)]
            pending_screenshot_delay_frames: 0,
            #[cfg(debug_assertions)]
            pending_screenshot_attempts: 0,
        }));

        assert!(should_show_token_accounts_section(
            ProviderId::Factory,
            &shared_state
        ));
    }

    #[test]
    fn alibaba_cookie_source_label_matches_supported_modes() {
        assert_eq!(
            alibaba_cookie_source_label("auto", Language::English),
            "Automatic"
        );
        assert_eq!(
            alibaba_cookie_source_label("manual", Language::English),
            "Manual"
        );
    }

    #[test]
    fn token_accounts_section_shows_for_alibaba_manual_cookie_mode() {
        let shared_state = Arc::new(Mutex::new(PreferencesSharedState {
            is_open: false,
            active_tab: PreferencesTab::Providers,
            settings: Settings {
                alibaba_cookie_source: "manual".to_string(),
                ..Settings::default()
            },
            settings_changed: false,
            cookies: Default::default(),
            new_cookie_provider: String::new(),
            new_cookie_value: String::new(),
            cookie_status_msg: None,
            api_keys: Default::default(),
            new_api_key_provider: String::new(),
            new_api_key_value: String::new(),
            show_api_key_input: false,
            api_key_status_msg: None,
            selected_provider: Some(ProviderId::Alibaba),
            selected_browser: Some(BrowserType::Chrome),
            browser_import_status: None,
            refresh_requested: false,
            cached_snapshot: None,
            runtime_provider_errors: HashMap::new(),
            token_accounts: HashMap::new(),
            new_account_label: String::new(),
            new_account_token: String::new(),
            show_add_account_input: false,
            token_account_status_msg: None,
            shortcut_input: String::new(),
            shortcut_status_msg: None,
            #[cfg(debug_assertions)]
            debug_tab_targets: Vec::new(),
            #[cfg(debug_assertions)]
            debug_viewport_outer_rect: None,
            #[cfg(debug_assertions)]
            pending_screenshot_path: None,
            #[cfg(debug_assertions)]
            pending_screenshot_delay_frames: 0,
            #[cfg(debug_assertions)]
            pending_screenshot_attempts: 0,
        }));

        assert!(should_show_token_accounts_section(
            ProviderId::Alibaba,
            &shared_state
        ));
    }

    #[test]
    fn kimi_cookie_source_label_matches_supported_modes() {
        assert_eq!(
            kimi_cookie_source_label("auto", Language::English),
            "Automatic"
        );
        assert_eq!(
            kimi_cookie_source_label("manual", Language::English),
            "Manual"
        );
        assert_eq!(
            kimi_cookie_source_label("off", Language::English),
            "Disabled"
        );
    }

    #[test]
    fn minimax_cookie_source_label_matches_supported_modes() {
        assert_eq!(
            minimax_cookie_source_label("auto", Language::English),
            "Automatic"
        );
        assert_eq!(
            minimax_cookie_source_label("manual", Language::English),
            "Manual"
        );
    }

    #[test]
    fn augment_cookie_source_label_matches_supported_modes() {
        assert_eq!(
            augment_cookie_source_label("auto", Language::English),
            "Automatic"
        );
        assert_eq!(
            augment_cookie_source_label("manual", Language::English),
            "Manual"
        );
    }

    #[test]
    fn amp_cookie_source_label_matches_supported_modes() {
        assert_eq!(
            amp_cookie_source_label("auto", Language::English),
            "Automatic"
        );
        assert_eq!(
            amp_cookie_source_label("manual", Language::English),
            "Manual"
        );
    }

    #[test]
    fn ollama_cookie_source_label_matches_supported_modes() {
        assert_eq!(
            ollama_cookie_source_label("auto", Language::English),
            "Automatic"
        );
        assert_eq!(
            ollama_cookie_source_label("manual", Language::English),
            "Manual"
        );
    }

    #[test]
    fn minimax_region_label_matches_supported_modes() {
        assert_eq!(minimax_region_label("global"), "Global (.io)");
        assert_eq!(minimax_region_label("china"), "China Mainland (.com)");
    }

    #[test]
    fn alibaba_region_label_matches_supported_modes() {
        assert_eq!(alibaba_region_label("intl"), "International (Model Studio)");
        assert_eq!(alibaba_region_label("cn"), "China Mainland (Bailian)");
    }

    #[test]
    fn zai_region_label_matches_supported_modes() {
        assert_eq!(zai_region_label("global"), "Global");
        assert_eq!(zai_region_label("china"), "China Mainland (BigModel)");
    }

    #[test]
    fn gemini_cli_credentials_path_points_to_expected_location() {
        let path = gemini_cli_credentials_path().expect("home dir should be available in tests");
        assert!(path.ends_with(PathBuf::from(".gemini").join("oauth_creds.json")));
    }

    #[test]
    fn compact_credentials_path_keeps_only_tail_segments() {
        assert_eq!(
            compact_credentials_path("C:\\Users\\mac\\.gemini\\oauth_creds.json"),
            ".gemini/oauth_creds.json"
        );
        assert_eq!(
            compact_credentials_path(
                "C:\\Users\\mac\\AppData\\Roaming\\gcloud\\application_default_credentials.json"
            ),
            "gcloud/application_default_credentials.json"
        );
    }

    #[test]
    fn vertexai_credentials_path_points_to_expected_location() {
        let path = vertexai_credentials_path().expect("config dir should be available in tests");
        assert!(
            path.ends_with(PathBuf::from("gcloud").join("application_default_credentials.json"))
        );
    }

    #[test]
    fn token_accounts_section_shows_for_minimax_manual_cookie_mode_without_api_token() {
        let shared_state = Arc::new(Mutex::new(PreferencesSharedState {
            is_open: false,
            active_tab: PreferencesTab::Providers,
            settings: Settings {
                minimax_cookie_source: "manual".to_string(),
                minimax_api_token: String::new(),
                ..Settings::default()
            },
            settings_changed: false,
            cookies: Default::default(),
            new_cookie_provider: String::new(),
            new_cookie_value: String::new(),
            cookie_status_msg: None,
            api_keys: Default::default(),
            new_api_key_provider: String::new(),
            new_api_key_value: String::new(),
            show_api_key_input: false,
            api_key_status_msg: None,
            selected_provider: Some(ProviderId::MiniMax),
            selected_browser: Some(BrowserType::Chrome),
            browser_import_status: None,
            refresh_requested: false,
            cached_snapshot: None,
            runtime_provider_errors: HashMap::new(),
            token_accounts: HashMap::new(),
            new_account_label: String::new(),
            new_account_token: String::new(),
            show_add_account_input: false,
            token_account_status_msg: None,
            shortcut_input: String::new(),
            shortcut_status_msg: None,
            #[cfg(debug_assertions)]
            debug_tab_targets: Vec::new(),
            #[cfg(debug_assertions)]
            debug_viewport_outer_rect: None,
            #[cfg(debug_assertions)]
            pending_screenshot_path: None,
            #[cfg(debug_assertions)]
            pending_screenshot_delay_frames: 0,
            #[cfg(debug_assertions)]
            pending_screenshot_attempts: 0,
        }));

        assert!(should_show_token_accounts_section(
            ProviderId::MiniMax,
            &shared_state
        ));
    }

    #[test]
    fn token_accounts_section_hides_for_minimax_api_token_mode() {
        let shared_state = Arc::new(Mutex::new(PreferencesSharedState {
            is_open: false,
            active_tab: PreferencesTab::Providers,
            settings: Settings {
                minimax_cookie_source: "manual".to_string(),
                minimax_api_token: "mmx-secret".to_string(),
                ..Settings::default()
            },
            settings_changed: false,
            cookies: Default::default(),
            new_cookie_provider: String::new(),
            new_cookie_value: String::new(),
            cookie_status_msg: None,
            api_keys: Default::default(),
            new_api_key_provider: String::new(),
            new_api_key_value: String::new(),
            show_api_key_input: false,
            api_key_status_msg: None,
            selected_provider: Some(ProviderId::MiniMax),
            selected_browser: Some(BrowserType::Chrome),
            browser_import_status: None,
            refresh_requested: false,
            cached_snapshot: None,
            runtime_provider_errors: HashMap::new(),
            token_accounts: HashMap::new(),
            new_account_label: String::new(),
            new_account_token: String::new(),
            show_add_account_input: false,
            token_account_status_msg: None,
            shortcut_input: String::new(),
            shortcut_status_msg: None,
            #[cfg(debug_assertions)]
            debug_tab_targets: Vec::new(),
            #[cfg(debug_assertions)]
            debug_viewport_outer_rect: None,
            #[cfg(debug_assertions)]
            pending_screenshot_path: None,
            #[cfg(debug_assertions)]
            pending_screenshot_delay_frames: 0,
            #[cfg(debug_assertions)]
            pending_screenshot_attempts: 0,
        }));

        assert!(!should_show_token_accounts_section(
            ProviderId::MiniMax,
            &shared_state
        ));
    }

    #[test]
    fn token_accounts_section_shows_for_augment_manual_cookie_mode() {
        let shared_state = Arc::new(Mutex::new(PreferencesSharedState {
            is_open: false,
            active_tab: PreferencesTab::Providers,
            settings: Settings {
                augment_cookie_source: "manual".to_string(),
                ..Settings::default()
            },
            settings_changed: false,
            cookies: Default::default(),
            new_cookie_provider: String::new(),
            new_cookie_value: String::new(),
            cookie_status_msg: None,
            api_keys: Default::default(),
            new_api_key_provider: String::new(),
            new_api_key_value: String::new(),
            show_api_key_input: false,
            api_key_status_msg: None,
            selected_provider: Some(ProviderId::Augment),
            selected_browser: Some(BrowserType::Chrome),
            browser_import_status: None,
            refresh_requested: false,
            cached_snapshot: None,
            runtime_provider_errors: HashMap::new(),
            token_accounts: HashMap::new(),
            new_account_label: String::new(),
            new_account_token: String::new(),
            show_add_account_input: false,
            token_account_status_msg: None,
            shortcut_input: String::new(),
            shortcut_status_msg: None,
            #[cfg(debug_assertions)]
            debug_tab_targets: Vec::new(),
            #[cfg(debug_assertions)]
            debug_viewport_outer_rect: None,
            #[cfg(debug_assertions)]
            pending_screenshot_path: None,
            #[cfg(debug_assertions)]
            pending_screenshot_delay_frames: 0,
            #[cfg(debug_assertions)]
            pending_screenshot_attempts: 0,
        }));

        assert!(should_show_token_accounts_section(
            ProviderId::Augment,
            &shared_state
        ));
    }

    #[test]
    fn token_accounts_section_shows_for_amp_manual_cookie_mode() {
        let shared_state = Arc::new(Mutex::new(PreferencesSharedState {
            is_open: false,
            active_tab: PreferencesTab::Providers,
            settings: Settings {
                amp_cookie_source: "manual".to_string(),
                ..Settings::default()
            },
            settings_changed: false,
            cookies: Default::default(),
            new_cookie_provider: String::new(),
            new_cookie_value: String::new(),
            cookie_status_msg: None,
            api_keys: Default::default(),
            new_api_key_provider: String::new(),
            new_api_key_value: String::new(),
            show_api_key_input: false,
            api_key_status_msg: None,
            selected_provider: Some(ProviderId::Amp),
            selected_browser: Some(BrowserType::Chrome),
            browser_import_status: None,
            refresh_requested: false,
            cached_snapshot: None,
            runtime_provider_errors: HashMap::new(),
            token_accounts: HashMap::new(),
            new_account_label: String::new(),
            new_account_token: String::new(),
            show_add_account_input: false,
            token_account_status_msg: None,
            shortcut_input: String::new(),
            shortcut_status_msg: None,
            #[cfg(debug_assertions)]
            debug_tab_targets: Vec::new(),
            #[cfg(debug_assertions)]
            debug_viewport_outer_rect: None,
            #[cfg(debug_assertions)]
            pending_screenshot_path: None,
            #[cfg(debug_assertions)]
            pending_screenshot_delay_frames: 0,
            #[cfg(debug_assertions)]
            pending_screenshot_attempts: 0,
        }));

        assert!(should_show_token_accounts_section(
            ProviderId::Amp,
            &shared_state
        ));
    }

    #[test]
    fn token_accounts_section_shows_for_ollama_manual_cookie_mode() {
        let shared_state = Arc::new(Mutex::new(PreferencesSharedState {
            is_open: false,
            active_tab: PreferencesTab::Providers,
            settings: Settings {
                ollama_cookie_source: "manual".to_string(),
                ..Settings::default()
            },
            settings_changed: false,
            cookies: Default::default(),
            new_cookie_provider: String::new(),
            new_cookie_value: String::new(),
            cookie_status_msg: None,
            api_keys: Default::default(),
            new_api_key_provider: String::new(),
            new_api_key_value: String::new(),
            show_api_key_input: false,
            api_key_status_msg: None,
            selected_provider: Some(ProviderId::Ollama),
            selected_browser: Some(BrowserType::Chrome),
            browser_import_status: None,
            refresh_requested: false,
            cached_snapshot: None,
            runtime_provider_errors: HashMap::new(),
            token_accounts: HashMap::new(),
            new_account_label: String::new(),
            new_account_token: String::new(),
            show_add_account_input: false,
            token_account_status_msg: None,
            shortcut_input: String::new(),
            shortcut_status_msg: None,
            #[cfg(debug_assertions)]
            debug_tab_targets: Vec::new(),
            #[cfg(debug_assertions)]
            debug_viewport_outer_rect: None,
            #[cfg(debug_assertions)]
            pending_screenshot_path: None,
            #[cfg(debug_assertions)]
            pending_screenshot_delay_frames: 0,
            #[cfg(debug_assertions)]
            pending_screenshot_attempts: 0,
        }));

        assert!(should_show_token_accounts_section(
            ProviderId::Ollama,
            &shared_state
        ));
    }

    #[test]
    fn provider_detail_source_display_uses_auto_for_cursor() {
        assert_eq!(
            provider_detail_source_display(ProviderId::Cursor, Language::English),
            "web"
        );
        assert_eq!(
            provider_detail_source_display(ProviderId::Claude, Language::English),
            "auto"
        );
        assert_eq!(
            provider_detail_source_display(ProviderId::Copilot, Language::English),
            "github api"
        );
    }

    #[test]
    fn token_accounts_section_stays_hidden_for_cursor_without_manual_state() {
        let shared_state = Arc::new(Mutex::new(PreferencesSharedState {
            is_open: false,
            active_tab: PreferencesTab::Providers,
            settings: Settings::default(),
            settings_changed: false,
            cookies: Default::default(),
            new_cookie_provider: String::new(),
            new_cookie_value: String::new(),
            cookie_status_msg: None,
            api_keys: Default::default(),
            new_api_key_provider: String::new(),
            new_api_key_value: String::new(),
            show_api_key_input: false,
            api_key_status_msg: None,
            selected_provider: Some(ProviderId::Cursor),
            selected_browser: Some(BrowserType::Chrome),
            browser_import_status: None,
            refresh_requested: false,
            cached_snapshot: None,
            runtime_provider_errors: HashMap::new(),
            token_accounts: HashMap::new(),
            new_account_label: String::new(),
            new_account_token: String::new(),
            show_add_account_input: false,
            token_account_status_msg: None,
            shortcut_input: String::new(),
            shortcut_status_msg: None,
            #[cfg(debug_assertions)]
            debug_tab_targets: Vec::new(),
            #[cfg(debug_assertions)]
            debug_viewport_outer_rect: None,
            #[cfg(debug_assertions)]
            pending_screenshot_path: None,
            #[cfg(debug_assertions)]
            pending_screenshot_delay_frames: 0,
            #[cfg(debug_assertions)]
            pending_screenshot_attempts: 0,
        }));

        assert!(!should_show_token_accounts_section(
            ProviderId::Cursor,
            &shared_state
        ));

        if let Ok(mut state) = shared_state.lock() {
            state.token_accounts.insert(
                ProviderId::Cursor,
                ProviderAccountData {
                    version: 1,
                    accounts: vec![crate::core::TokenAccount::new("Main", "Cookie: abc")],
                    active_index: 0,
                },
            );
        }

        assert!(should_show_token_accounts_section(
            ProviderId::Cursor,
            &shared_state
        ));
    }

    #[test]
    fn request_screenshot_for_testing_opens_preferences_and_waits_extra_frames() {
        let mut window = PreferencesWindow::default();

        window.request_screenshot_for_testing(PathBuf::from("C:\\temp\\prefs.png"));

        assert!(window.is_open);
        let state = window.shared_state.lock().expect("shared state lock");
        assert!(state.is_open);
        assert_eq!(
            state.pending_screenshot_path.as_deref(),
            Some(std::path::Path::new("C:\\temp\\prefs.png"))
        );
        assert_eq!(state.pending_screenshot_delay_frames, 3);
        assert_eq!(state.pending_screenshot_attempts, 0);
    }
}
