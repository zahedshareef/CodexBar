//! System tray manager with dynamic usage bar icon
//!
//! Creates a system tray icon that shows session and weekly usage as two horizontal bars

#![allow(dead_code)]

use image::{ImageBuffer, Rgba, RgbaImage};
#[cfg(debug_assertions)]
use serde::Serialize;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{CheckMenuItem, ContextMenu, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
};

use super::icon::{LoadingPattern, UsageLevel};
use crate::core::ProviderId;
use crate::locale::{self, IconOverlayStatus};
use crate::settings::{Language, Settings, TrayIconMode};
use crate::status::IndicatorStatusLevel;

const ICON_SIZE: u32 = 32;

/// Surprise animation types (matching macOS CodexBar)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SurpriseAnimation {
    /// No animation
    #[allow(dead_code)]
    None,
    /// Bars flash bright white briefly (like eyes blinking)
    Blink,
    /// Bars wiggle left/right (Claude arms/legs effect)
    Wiggle,
    /// Bars pulse in intensity
    Pulse,
    /// Rainbow color sweep
    Rainbow,
    /// Icon tilts slightly (Codex hat tilt effect)
    Tilt,
}

impl SurpriseAnimation {
    /// Get a random animation type
    pub fn random() -> Self {
        use rand::Rng;
        let mut rng = rand::rng();
        match rng.random_range(0..5) {
            0 => SurpriseAnimation::Blink,
            1 => SurpriseAnimation::Wiggle,
            2 => SurpriseAnimation::Pulse,
            3 => SurpriseAnimation::Rainbow,
            _ => SurpriseAnimation::Tilt,
        }
    }

    /// Duration of the animation in frames (at ~60fps)
    pub fn duration_frames(&self) -> u32 {
        match self {
            SurpriseAnimation::None => 0,
            SurpriseAnimation::Blink => 8,    // Quick flash
            SurpriseAnimation::Wiggle => 20,  // Shake back and forth
            SurpriseAnimation::Pulse => 30,   // Slow pulse
            SurpriseAnimation::Rainbow => 40, // Color sweep
            SurpriseAnimation::Tilt => 24,    // Tilt and return
        }
    }
}

/// Icon overlay types for status indicators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum IconOverlay {
    /// No overlay - normal display
    #[default]
    None,
    /// Error state - grayed out icon with X
    Error,
    /// Stale data - dim icon with clock indicator
    #[allow(dead_code)]
    Stale,
    /// Incident - warning badge overlay
    Incident,
    /// Partial outage - orange badge
    Partial,
}

/// Provider usage data for merged icon mode
#[derive(Clone, Debug)]
pub struct ProviderUsage {
    pub name: String,
    pub session_percent: f64,
    #[allow(dead_code)]
    pub weekly_percent: f64,
}

/// Tray state for tooltip relocalization
#[derive(Debug, Clone, Default)]
pub enum TrayTooltipState {
    /// Default/initial state (no data yet)
    #[default]
    Default,
    /// Normal usage display with provider data
    Normal {
        provider_name: String,
        session_percent: f64,
        weekly_percent: f64,
    },
    /// Usage with status overlay
    WithStatus {
        provider_name: String,
        session_percent: f64,
        weekly_percent: f64,
        overlay: IconOverlay,
    },
    /// Credits mode
    Credits {
        provider_name: String,
        credits_percent: f64,
    },
    /// Loading state
    Loading,
    /// No providers available
    NoProviders,
    /// Merged providers display
    Merged { providers: Vec<ProviderUsage> },
    /// Error state
    Error {
        provider_name: String,
        error_msg: String,
    },
}

#[cfg(debug_assertions)]
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DebugProviderTraySnapshot {
    pub provider: String,
    pub state_kind: String,
    pub tooltip: Option<String>,
}

#[cfg(debug_assertions)]
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DebugTraySnapshot {
    pub mode: String,
    pub icon_count: usize,
    pub state_kind: Option<String>,
    pub primary_provider: Option<String>,
    pub tooltip: Option<String>,
    pub providers: Vec<DebugProviderTraySnapshot>,
}

#[cfg(debug_assertions)]
fn tray_tooltip_state_kind(state: &TrayTooltipState) -> &'static str {
    match state {
        TrayTooltipState::Default => "default",
        TrayTooltipState::Normal { .. } => "normal",
        TrayTooltipState::WithStatus { .. } => "with_status",
        TrayTooltipState::Credits { .. } => "credits",
        TrayTooltipState::Loading => "loading",
        TrayTooltipState::NoProviders => "no_providers",
        TrayTooltipState::Merged { .. } => "merged",
        TrayTooltipState::Error { .. } => "error",
    }
}

#[cfg(debug_assertions)]
fn provider_tooltip_state_kind(state: &ProviderTooltipState) -> &'static str {
    match state {
        ProviderTooltipState::Default => "default",
        ProviderTooltipState::Normal { .. } => "normal",
        ProviderTooltipState::WithStatus { .. } => "with_status",
        ProviderTooltipState::Loading => "loading",
        ProviderTooltipState::Error { .. } => "error",
    }
}

#[cfg(debug_assertions)]
fn overlay_status(overlay: IconOverlay) -> Option<IconOverlayStatus> {
    match overlay {
        IconOverlay::None => None,
        IconOverlay::Error => Some(IconOverlayStatus::Error),
        IconOverlay::Stale => Some(IconOverlayStatus::Stale),
        IconOverlay::Incident => Some(IconOverlayStatus::Incident),
        IconOverlay::Partial => Some(IconOverlayStatus::Partial),
    }
}

fn provider_loading_tooltip(provider_id: ProviderId, lang: Language) -> String {
    let loading = locale::get_text(lang, locale::LocaleKey::TrayLoading);
    let provider_loading = loading.strip_prefix("CodexBar - ").unwrap_or(loading);
    format!("{} - {}", provider_id.display_name(), provider_loading)
}

#[cfg(debug_assertions)]
fn debug_single_tooltip_text(state: &TrayTooltipState, lang: Language) -> String {
    match state {
        TrayTooltipState::Default | TrayTooltipState::Loading => {
            locale::get_text(lang, locale::LocaleKey::TrayLoading).to_string()
        }
        TrayTooltipState::Normal {
            provider_name,
            session_percent,
            weekly_percent,
        } => locale::tray_tooltip(provider_name, *session_percent, *weekly_percent),
        TrayTooltipState::WithStatus {
            provider_name,
            session_percent,
            weekly_percent,
            overlay,
        } => locale::tray_tooltip_with_status(
            provider_name,
            *session_percent,
            *weekly_percent,
            overlay_status(*overlay),
        ),
        TrayTooltipState::Credits {
            provider_name,
            credits_percent,
        } => locale::tray_tooltip_credits(provider_name, *credits_percent),
        TrayTooltipState::NoProviders => {
            locale::get_text(lang, locale::LocaleKey::TrayNoProviders).to_string()
        }
        TrayTooltipState::Merged { providers } => {
            let tooltip_lines: Vec<String> = providers
                .iter()
                .take(4)
                .map(|p| {
                    format!(
                        "{}: {:.0}% / {:.0}%",
                        p.name, p.session_percent, p.weekly_percent
                    )
                })
                .collect();
            format!("CodexBar\n{}", tooltip_lines.join("\n"))
        }
        TrayTooltipState::Error {
            provider_name,
            error_msg,
        } => format!("{}: {}", provider_name, error_msg),
    }
}

#[cfg(debug_assertions)]
fn debug_provider_tooltip_text(
    provider_id: ProviderId,
    state: &ProviderTooltipState,
    lang: Language,
) -> String {
    match state {
        ProviderTooltipState::Default | ProviderTooltipState::Loading => {
            provider_loading_tooltip(provider_id, lang)
        }
        ProviderTooltipState::Normal {
            session_percent,
            weekly_percent,
        } => locale::tray_tooltip(
            provider_id.display_name(),
            *session_percent,
            *weekly_percent,
        ),
        ProviderTooltipState::WithStatus {
            session_percent,
            weekly_percent,
            overlay,
        } => locale::tray_tooltip_with_status(
            provider_id.display_name(),
            *session_percent,
            *weekly_percent,
            overlay_status(*overlay),
        ),
        ProviderTooltipState::Error { error_msg } => {
            format!("{}: {}", provider_id.display_name(), error_msg)
        }
    }
}

/// System tray manager
pub struct TrayManager {
    tray_icon: TrayIcon,
    /// Native right-click menu (not attached to the tray icon).
    /// Kept separate so tray-icon never auto-shows it on left-click.
    context_menu: RefCell<Menu>,
    /// Provider menu items for updating with status prefixes
    provider_menu_items: RefCell<HashMap<ProviderId, CheckMenuItem>>,
    /// Read-only native tray status rows showing provider usage at a glance.
    provider_open_items: RefCell<HashMap<ProviderId, MenuItem>>,
    /// Last rendered provider row labels so menu rebuilds can restore live state.
    provider_open_labels: RefCell<HashMap<ProviderId, String>>,
    last_usage_signature: Cell<Option<u64>>,
    last_merged_signature: Cell<Option<u64>>,
    /// Current tooltip state for language relocalization
    tooltip_state: RefCell<TrayTooltipState>,
}

type SingleTrayMenuBundle = (
    Menu,
    HashMap<ProviderId, CheckMenuItem>,
    HashMap<ProviderId, MenuItem>,
    HashMap<ProviderId, String>,
);

impl TrayManager {
    fn default_provider_open_label(provider_id: ProviderId) -> String {
        format!("{}  Loading...", provider_id.display_name())
    }

    fn format_provider_open_label(provider_id: ProviderId, detail: &str) -> String {
        format!("{}  {}", provider_id.display_name(), detail)
    }

    fn build_menu(lang: Language, settings: &Settings) -> anyhow::Result<SingleTrayMenuBundle> {
        let menu = Menu::new();

        // Top-level provider status rows (read-only; quick glance at each provider)
        let mut provider_open_items = HashMap::new();
        let mut provider_open_labels = HashMap::new();
        for provider_id in settings.get_enabled_provider_ids() {
            let label = Self::default_provider_open_label(provider_id);
            let item = MenuItem::with_id(
                format!("status_{}", provider_id.cli_name()),
                &label,
                false, // non-clickable status row
                None,
            );
            menu.append(&item)?;
            provider_open_labels.insert(provider_id, label);
            provider_open_items.insert(provider_id, item);
        }

        menu.append(&PredefinedMenuItem::separator())?;

        let refresh_item = MenuItem::with_id(
            "refresh",
            locale::get_text(lang, locale::LocaleKey::TrayRefreshAll),
            true,
            None,
        );
        menu.append(&refresh_item)?;

        let popout_item = MenuItem::with_id(
            "popout",
            locale::get_text(lang, locale::LocaleKey::TrayPopOutDashboard),
            true,
            None,
        );
        menu.append(&popout_item)?;

        menu.append(&PredefinedMenuItem::separator())?;

        let providers_submenu = Submenu::new(
            locale::get_text(lang, locale::LocaleKey::TrayProviders),
            true,
        );
        let mut provider_menu_items = HashMap::new();
        for provider_id in ProviderId::all() {
            let cli_name = provider_id.cli_name();
            let display_name = provider_id.display_name();
            let is_enabled = settings.is_provider_enabled(*provider_id);
            let item_id = format!("provider_{}", cli_name);
            let check_item = CheckMenuItem::with_id(&item_id, display_name, true, is_enabled, None);
            providers_submenu.append(&check_item)?;
            provider_menu_items.insert(*provider_id, check_item);
        }
        menu.append(&providers_submenu)?;

        menu.append(&PredefinedMenuItem::separator())?;

        let settings_item = MenuItem::with_id(
            "settings",
            locale::get_text(lang, locale::LocaleKey::TraySettings),
            true,
            None,
        );
        menu.append(&settings_item)?;

        let updates_item = MenuItem::with_id(
            "updates",
            locale::get_text(lang, locale::LocaleKey::TrayCheckForUpdates),
            true,
            None,
        );
        menu.append(&updates_item)?;

        menu.append(&PredefinedMenuItem::separator())?;

        let quit_item = MenuItem::with_id(
            "quit",
            locale::get_text(lang, locale::LocaleKey::TrayQuit),
            true,
            None,
        );
        menu.append(&quit_item)?;

        Ok((
            menu,
            provider_menu_items,
            provider_open_items,
            provider_open_labels,
        ))
    }

    fn set_provider_open_label(&self, provider_id: ProviderId, label: String) {
        if let Some(item) = self.provider_open_items.borrow().get(&provider_id) {
            item.set_text(&label);
        }
        self.provider_open_labels
            .borrow_mut()
            .insert(provider_id, label);
    }

    /// Create a new tray manager with default icon
    pub fn new() -> anyhow::Result<Self> {
        let settings = Settings::load();
        let lang = settings.ui_language;
        let (menu, provider_menu_items, provider_open_items, provider_open_labels) =
            Self::build_menu(lang, &settings)?;

        let icon = create_bar_icon(0.0, 0.0, IconOverlay::None);

        let lang = Settings::load().ui_language;
        let tray_icon = TrayIconBuilder::new()
            .with_tooltip(locale::get_text(lang, locale::LocaleKey::TrayLoading))
            .with_icon(icon)
            .build()?;

        Ok(Self {
            tray_icon,
            context_menu: RefCell::new(menu),
            provider_menu_items: RefCell::new(provider_menu_items),
            provider_open_items: RefCell::new(provider_open_items),
            provider_open_labels: RefCell::new(provider_open_labels),
            last_usage_signature: Cell::new(None),
            last_merged_signature: Cell::new(None),
            tooltip_state: RefCell::new(TrayTooltipState::Default),
        })
    }

    /// Update the tray icon based on usage percentages (single provider mode)
    pub fn update_usage(&self, session_percent: f64, weekly_percent: f64, provider_name: &str) {
        // Store the state for language relocalization
        *self.tooltip_state.borrow_mut() = TrayTooltipState::Normal {
            provider_name: provider_name.to_string(),
            session_percent,
            weekly_percent,
        };

        let tooltip = locale::tray_tooltip(provider_name, session_percent, weekly_percent);
        let _ = self.tray_icon.set_tooltip(Some(&tooltip));
        if let Some(provider_id) = ProviderId::from_cli_name(provider_name) {
            self.set_provider_open_label(
                provider_id,
                Self::format_provider_open_label(
                    provider_id,
                    &format!(
                        "Session {:.0}%  Weekly {:.0}%",
                        session_percent, weekly_percent
                    ),
                ),
            );
        }

        if !self.should_update_usage(
            session_percent,
            weekly_percent,
            provider_name,
            IconOverlay::None,
        ) {
            return;
        }

        let icon = create_bar_icon(session_percent, weekly_percent, IconOverlay::None);
        let _ = self.tray_icon.set_icon(Some(icon));
    }

    /// Update the tray icon with an overlay (error, stale, incident)
    pub fn update_usage_with_overlay(
        &self,
        session_percent: f64,
        weekly_percent: f64,
        provider_name: &str,
        overlay: IconOverlay,
    ) {
        // Store the state for language relocalization
        *self.tooltip_state.borrow_mut() = TrayTooltipState::WithStatus {
            provider_name: provider_name.to_string(),
            session_percent,
            weekly_percent,
            overlay,
        };

        let status = match overlay {
            IconOverlay::None => None,
            IconOverlay::Error => Some(IconOverlayStatus::Error),
            IconOverlay::Stale => Some(IconOverlayStatus::Stale),
            IconOverlay::Incident => Some(IconOverlayStatus::Incident),
            IconOverlay::Partial => Some(IconOverlayStatus::Partial),
        };

        let tooltip = locale::tray_tooltip_with_status(
            provider_name,
            session_percent,
            weekly_percent,
            status,
        );
        let _ = self.tray_icon.set_tooltip(Some(&tooltip));
        if let Some(provider_id) = ProviderId::from_cli_name(provider_name) {
            self.set_provider_open_label(
                provider_id,
                Self::format_provider_open_label(
                    provider_id,
                    &format!(
                        "Session {:.0}%  Weekly {:.0}%",
                        session_percent, weekly_percent
                    ),
                ),
            );
        }

        if !self.should_update_usage(session_percent, weekly_percent, provider_name, overlay) {
            return;
        }

        let icon = create_bar_icon(session_percent, weekly_percent, overlay);
        let _ = self.tray_icon.set_icon(Some(icon));
    }

    /// Show error state on the tray icon
    #[allow(dead_code)]
    pub fn show_error(&self, provider_name: &str, error_msg: &str) {
        // Store the state for language relocalization
        *self.tooltip_state.borrow_mut() = TrayTooltipState::Error {
            provider_name: provider_name.to_string(),
            error_msg: error_msg.to_string(),
        };

        let icon = create_bar_icon(0.0, 0.0, IconOverlay::Error);
        let _ = self.tray_icon.set_icon(Some(icon));
        let tooltip = format!("{}: {}", provider_name, error_msg);
        let _ = self.tray_icon.set_tooltip(Some(&tooltip));
        if let Some(provider_id) = ProviderId::from_cli_name(provider_name) {
            self.set_provider_open_label(
                provider_id,
                Self::format_provider_open_label(provider_id, &format!("Error: {}", error_msg)),
            );
        }
    }

    /// Show stale data indicator
    #[allow(dead_code)]
    pub fn show_stale(
        &self,
        session_percent: f64,
        weekly_percent: f64,
        provider_name: &str,
        age_minutes: u64,
    ) {
        // Store the state for language relocalization (as overlay state)
        *self.tooltip_state.borrow_mut() = TrayTooltipState::WithStatus {
            provider_name: provider_name.to_string(),
            session_percent,
            weekly_percent,
            overlay: IconOverlay::Stale,
        };

        let icon = create_bar_icon(session_percent, weekly_percent, IconOverlay::Stale);
        let _ = self.tray_icon.set_icon(Some(icon));

        let tooltip = format!(
            "{}：会话 {}% | 周度 {}%（数据 {} 分钟前）",
            provider_name, session_percent as i32, weekly_percent as i32, age_minutes
        );
        let _ = self.tray_icon.set_tooltip(Some(&tooltip));
    }

    /// Update the tray icon showing credits mode (thicker bar when weekly exhausted)
    /// This shows a thick credits bar when weekly quota is exhausted but credits remain
    pub fn update_credits_mode(&self, credits_percent: f64, provider_name: &str) {
        // Store the state for language relocalization
        *self.tooltip_state.borrow_mut() = TrayTooltipState::Credits {
            provider_name: provider_name.to_string(),
            credits_percent,
        };

        let icon = create_credits_icon(credits_percent);
        let _ = self.tray_icon.set_icon(Some(icon));

        let tooltip = locale::tray_tooltip_credits(provider_name, credits_percent);
        let _ = self.tray_icon.set_tooltip(Some(&tooltip));
    }

    /// Update the tray icon showing multiple providers (merged mode)
    pub fn update_merged(&self, providers: &[ProviderUsage]) {
        if providers.is_empty() {
            // Store the state for language relocalization
            *self.tooltip_state.borrow_mut() = TrayTooltipState::NoProviders;

            let icon = create_bar_icon(0.0, 0.0, IconOverlay::None);
            let _ = self.tray_icon.set_icon(Some(icon));
            let lang = Settings::load().ui_language;
            let _ = self.tray_icon.set_tooltip(Some(locale::get_text(
                lang,
                locale::LocaleKey::TrayNoProviders,
            )));
            let provider_ids: Vec<ProviderId> =
                self.provider_open_labels.borrow().keys().copied().collect();
            for provider_id in provider_ids {
                self.set_provider_open_label(
                    provider_id,
                    Self::default_provider_open_label(provider_id),
                );
            }
            return;
        }

        // Store the state for language relocalization
        *self.tooltip_state.borrow_mut() = TrayTooltipState::Merged {
            providers: providers.to_vec(),
        };

        let signature = Self::merged_signature(providers);
        if self.last_merged_signature.get() == Some(signature) {
            // Still update tooltip to reflect current data even if icon unchanged
            let tooltip_lines: Vec<String> = providers
                .iter()
                .take(4)
                .map(|p| format!("{}: {}%", p.name, p.session_percent as i32))
                .collect();
            let tooltip = format!("CodexBar\n{}", tooltip_lines.join("\n"));
            let _ = self.tray_icon.set_tooltip(Some(&tooltip));
            return;
        }

        let icon = create_merged_icon(providers);
        let _ = self.tray_icon.set_icon(Some(icon));

        // Build tooltip with all providers
        let tooltip_lines: Vec<String> = providers
            .iter()
            .take(4) // Limit tooltip length
            .map(|p| format!("{}: {}%", p.name, p.session_percent as i32))
            .collect();
        let tooltip = format!("CodexBar\n{}", tooltip_lines.join("\n"));
        let _ = self.tray_icon.set_tooltip(Some(&tooltip));

        for usage in providers {
            if let Some(provider_id) = ProviderId::from_cli_name(&usage.name) {
                self.set_provider_open_label(
                    provider_id,
                    Self::format_provider_open_label(
                        provider_id,
                        &format!(
                            "Session {:.0}%  Weekly {:.0}%",
                            usage.session_percent, usage.weekly_percent
                        ),
                    ),
                );
            }
        }

        self.last_merged_signature.set(Some(signature));
    }

    /// Show loading animation on the tray icon
    pub fn show_loading(&self, pattern: LoadingPattern, phase: f64) {
        // Store the state for language relocalization
        *self.tooltip_state.borrow_mut() = TrayTooltipState::Loading;

        let primary = pattern.value(phase);
        let secondary = pattern.value(phase + pattern.secondary_offset());

        // Clear signatures so the next real update isn't skipped
        self.last_usage_signature.set(None);
        self.last_merged_signature.set(None);

        let icon = create_loading_icon(primary, secondary);
        let _ = self.tray_icon.set_icon(Some(icon));
        let lang = Settings::load().ui_language;
        let _ = self
            .tray_icon
            .set_tooltip(Some(locale::get_text(lang, locale::LocaleKey::TrayLoading)));
        let provider_ids: Vec<ProviderId> =
            self.provider_open_labels.borrow().keys().copied().collect();
        for provider_id in provider_ids {
            self.set_provider_open_label(
                provider_id,
                Self::default_provider_open_label(provider_id),
            );
        }
    }

    /// Show morph animation on the tray icon (Unbraid effect)
    /// Progress goes from 0.0 (knot/logo) to 1.0 (usage bars)
    pub fn show_morph(&self, progress: f64, session_percent: f64, weekly_percent: f64) {
        // Store the state for language relocalization (loading during morph)
        *self.tooltip_state.borrow_mut() = TrayTooltipState::Loading;

        let icon = create_morph_icon(progress, session_percent, weekly_percent);
        let _ = self.tray_icon.set_icon(Some(icon));
        let lang = Settings::load().ui_language;
        let _ = self
            .tray_icon
            .set_tooltip(Some(locale::get_text(lang, locale::LocaleKey::TrayLoading)));
    }

    /// Show a surprise animation frame
    pub fn show_surprise(
        &self,
        animation: SurpriseAnimation,
        frame: u32,
        session_percent: f64,
        weekly_percent: f64,
    ) {
        let icon = create_surprise_icon(animation, frame, session_percent, weekly_percent);
        let _ = self.tray_icon.set_icon(Some(icon));
    }

    /// Update provider menu item labels with status prefixes (colored dots)
    ///
    /// Takes a map of provider IDs to their current status levels and updates
    /// the corresponding menu item labels to show status dots for non-operational providers.
    pub fn update_provider_statuses(&self, statuses: &HashMap<ProviderId, IndicatorStatusLevel>) {
        for (provider_id, check_item) in self.provider_menu_items.borrow().iter() {
            let base_name = provider_id.display_name();
            if let Some(status_level) = statuses.get(provider_id) {
                let prefix = status_level.status_prefix();
                let new_label = format!("{}{}", prefix, base_name);
                check_item.set_text(&new_label);
            } else {
                // No status info, show plain name
                check_item.set_text(base_name);
            }
        }
    }

    /// Update a single provider's menu item label with status prefix
    pub fn update_provider_status(
        &self,
        provider_id: ProviderId,
        status_level: IndicatorStatusLevel,
    ) {
        if let Some(check_item) = self.provider_menu_items.borrow().get(&provider_id) {
            let base_name = provider_id.display_name();
            let prefix = status_level.status_prefix();
            let new_label = format!("{}{}", prefix, base_name);
            check_item.set_text(&new_label);
        }
    }

    /// Clear status prefix from a provider's menu item (revert to plain name)
    pub fn clear_provider_status(&self, provider_id: ProviderId) {
        if let Some(check_item) = self.provider_menu_items.borrow().get(&provider_id) {
            check_item.set_text(provider_id.display_name());
        }
    }

    /// Check for menu events
    pub fn check_events() -> Option<TrayMenuAction> {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            return tray_action_from_event_id(event.id.0.as_str());
        }
        None
    }

    /// Show the right-click context menu at the current cursor position.
    ///
    /// # Safety
    /// `hwnd` must be a valid window handle on the calling thread.
    #[cfg(target_os = "windows")]
    pub fn show_context_menu(&self, hwnd: isize) {
        unsafe {
            self.context_menu
                .borrow()
                .show_context_menu_for_hwnd(hwnd, None)
        };
    }

    /// Refresh the tray menu and tooltip with the current language
    /// This is called when the user changes the language setting
    pub fn refresh_language(&self) {
        let settings = Settings::load();
        let lang = settings.ui_language;

        if let Ok((menu, provider_menu_items, provider_open_items, mut provider_open_labels)) =
            Self::build_menu(lang, &settings)
        {
            for (provider_id, label) in self.provider_open_labels.borrow().iter() {
                provider_open_labels.insert(*provider_id, label.clone());
                if let Some(item) = provider_open_items.get(provider_id) {
                    item.set_text(label);
                }
            }
            self.provider_menu_items.replace(provider_menu_items);
            self.provider_open_items.replace(provider_open_items);
            self.provider_open_labels.replace(provider_open_labels);
            *self.context_menu.borrow_mut() = menu;
        }

        // Relocalize the tooltip based on the current state (not reset to loading)
        let tooltip = self.relocalize_tooltip(lang);
        let _ = self.tray_icon.set_tooltip(Some(&tooltip));
    }

    /// Relocalize tooltip based on the preserved state
    fn relocalize_tooltip(&self, lang: crate::settings::Language) -> String {
        let state = self.tooltip_state.borrow();
        match &*state {
            TrayTooltipState::Default => {
                locale::get_text(lang, locale::LocaleKey::TrayLoading).to_string()
            }
            TrayTooltipState::Normal {
                provider_name,
                session_percent,
                weekly_percent,
            } => locale::tray_tooltip(provider_name, *session_percent, *weekly_percent),
            TrayTooltipState::WithStatus {
                provider_name,
                session_percent,
                weekly_percent,
                overlay,
            } => {
                let status = match overlay {
                    IconOverlay::None => None,
                    IconOverlay::Error => Some(IconOverlayStatus::Error),
                    IconOverlay::Stale => Some(IconOverlayStatus::Stale),
                    IconOverlay::Incident => Some(IconOverlayStatus::Incident),
                    IconOverlay::Partial => Some(IconOverlayStatus::Partial),
                };
                locale::tray_tooltip_with_status(
                    provider_name,
                    *session_percent,
                    *weekly_percent,
                    status,
                )
            }
            TrayTooltipState::Credits {
                provider_name,
                credits_percent,
            } => locale::tray_tooltip_credits(provider_name, *credits_percent),
            TrayTooltipState::Loading => {
                locale::get_text(lang, locale::LocaleKey::TrayLoading).to_string()
            }
            TrayTooltipState::NoProviders => {
                locale::get_text(lang, locale::LocaleKey::TrayNoProviders).to_string()
            }
            TrayTooltipState::Merged { providers } => {
                let tooltip_lines: Vec<String> = providers
                    .iter()
                    .take(4)
                    .map(|p| format!("{}: {}%", p.name, p.session_percent as i32))
                    .collect();
                format!("CodexBar\n{}", tooltip_lines.join("\n"))
            }
            TrayTooltipState::Error {
                provider_name,
                error_msg,
            } => {
                format!("{}: {}", provider_name, error_msg)
            }
        }
    }

    #[cfg(debug_assertions)]
    pub fn debug_snapshot(&self) -> DebugTraySnapshot {
        let settings = Settings::load();
        let state = self.tooltip_state.borrow();
        let state_kind = tray_tooltip_state_kind(&state).to_string();
        let tooltip = Some(debug_single_tooltip_text(&state, settings.ui_language));
        let primary_provider = match &*state {
            TrayTooltipState::Normal { provider_name, .. }
            | TrayTooltipState::WithStatus { provider_name, .. }
            | TrayTooltipState::Credits { provider_name, .. }
            | TrayTooltipState::Error { provider_name, .. } => Some(provider_name.clone()),
            _ => None,
        };
        let providers = match &*state {
            TrayTooltipState::Merged { providers } => providers
                .iter()
                .map(|provider| DebugProviderTraySnapshot {
                    provider: provider.name.clone(),
                    state_kind: "merged_member".to_string(),
                    tooltip: None,
                })
                .collect(),
            _ => Vec::new(),
        };

        DebugTraySnapshot {
            mode: "single".to_string(),
            icon_count: 1,
            state_kind: Some(state_kind),
            primary_provider,
            tooltip,
            providers,
        }
    }
}

impl TrayManager {
    fn usage_signature(
        session_percent: f64,
        weekly_percent: f64,
        provider_name: &str,
        overlay: IconOverlay,
    ) -> u64 {
        let mut hasher = DefaultHasher::new();
        let session_tenths = (session_percent * 10.0).round() as i32;
        let weekly_tenths = (weekly_percent * 10.0).round() as i32;
        session_tenths.hash(&mut hasher);
        weekly_tenths.hash(&mut hasher);
        provider_name.hash(&mut hasher);
        overlay.hash(&mut hasher);
        hasher.finish()
    }

    fn should_update_usage(
        &self,
        session_percent: f64,
        weekly_percent: f64,
        provider_name: &str,
        overlay: IconOverlay,
    ) -> bool {
        let signature =
            Self::usage_signature(session_percent, weekly_percent, provider_name, overlay);
        if self.last_usage_signature.get() == Some(signature) {
            return false;
        }
        self.last_usage_signature.set(Some(signature));
        true
    }

    fn merged_signature(providers: &[ProviderUsage]) -> u64 {
        let mut hasher = DefaultHasher::new();
        providers.len().hash(&mut hasher);
        for p in providers.iter().take(8) {
            p.name.hash(&mut hasher);
            let session_tenths = (p.session_percent * 10.0).round() as i32;
            let weekly_tenths = (p.weekly_percent * 10.0).round() as i32;
            session_tenths.hash(&mut hasher);
            weekly_tenths.hash(&mut hasher);
        }
        hasher.finish()
    }
}

/// Tray menu actions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayMenuAction {
    PopOut,
    PopOutProvider(String),
    Refresh,
    RefreshProvider(String),
    Settings,
    CheckForUpdates,
    ToggleProvider(String),
    /// Left-click on tray icon: open the egui popup anchored near the tray area.
    /// Coordinates are physical pixels.
    TrayLeftClick {
        tray_x: i32,
        tray_y: i32,
    },
    /// Right-click on tray icon: show the native context menu.
    TrayRightClick,
    Quit,
}

fn tray_action_from_event_id(id_str: &str) -> Option<TrayMenuAction> {
    if id_str == "quit" {
        Some(TrayMenuAction::Quit)
    } else if id_str == "popout" {
        Some(TrayMenuAction::PopOut)
    } else if id_str == "refresh" {
        Some(TrayMenuAction::Refresh)
    } else if id_str == "settings" {
        Some(TrayMenuAction::Settings)
    } else if id_str == "updates" {
        Some(TrayMenuAction::CheckForUpdates)
    } else if id_str.starts_with("status_") {
        // Read-only status rows in the single-tray menu — no action.
        None
    } else if let Some(provider_name) = id_str.strip_prefix("popout_provider_") {
        Some(TrayMenuAction::PopOutProvider(provider_name.to_string()))
    } else if let Some(provider_name) = id_str.strip_prefix("refresh_provider_") {
        Some(TrayMenuAction::RefreshProvider(provider_name.to_string()))
    } else {
        id_str
            .strip_prefix("provider_")
            .map(|provider_name| TrayMenuAction::ToggleProvider(provider_name.to_string()))
    }
}

/// Per-provider tray state for tooltip relocalization
#[derive(Debug, Clone, Default)]
pub enum ProviderTooltipState {
    /// Default/initial state (no data yet)
    #[default]
    Default,
    /// Normal usage display
    Normal {
        session_percent: f64,
        weekly_percent: f64,
    },
    /// Usage with status overlay
    WithStatus {
        session_percent: f64,
        weekly_percent: f64,
        overlay: IconOverlay,
    },
    /// Loading state
    Loading,
    /// Error state
    Error { error_msg: String },
}

/// Multi-provider tray manager for per-provider icon mode
/// Creates and manages one tray icon per enabled provider
pub struct MultiTrayManager {
    /// Map of provider ID to their individual tray icon
    provider_icons: HashMap<ProviderId, TrayIcon>,
    /// Right-click menus kept separate from tray icons to prevent left-click showing a menu.
    provider_menus: RefCell<HashMap<ProviderId, Menu>>,
    provider_signatures: RefCell<HashMap<ProviderId, u64>>,
    /// Current tooltip states for language relocalization (per provider)
    tooltip_states: RefCell<HashMap<ProviderId, ProviderTooltipState>>,
    /// Disabled status rows shown inside each provider tray menu.
    provider_menu_detail_items: RefCell<HashMap<ProviderId, MenuItem>>,
    /// Last rendered menu detail text so menu rebuilds can restore state.
    provider_menu_detail_labels: RefCell<HashMap<ProviderId, String>>,
}

impl MultiTrayManager {
    /// Create a new multi-tray manager with icons for enabled providers
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            provider_icons: HashMap::new(),
            provider_menus: RefCell::new(HashMap::new()),
            provider_signatures: RefCell::new(HashMap::new()),
            tooltip_states: RefCell::new(HashMap::new()),
            provider_menu_detail_items: RefCell::new(HashMap::new()),
            provider_menu_detail_labels: RefCell::new(HashMap::new()),
        })
    }

    fn menu_detail_from_tooltip(provider_id: ProviderId, tooltip: String) -> String {
        let usage_prefix = format!("{}: ", provider_id.display_name());
        let loading_prefix = format!("{} - ", provider_id.display_name());
        tooltip
            .strip_prefix(&usage_prefix)
            .or_else(|| tooltip.strip_prefix(&loading_prefix))
            .unwrap_or(&tooltip)
            .to_string()
    }

    fn set_provider_menu_detail(&self, provider_id: ProviderId, label: String) {
        if let Some(item) = self.provider_menu_detail_items.borrow().get(&provider_id) {
            item.set_text(&label);
        }
        self.provider_menu_detail_labels
            .borrow_mut()
            .insert(provider_id, label);
    }

    fn build_provider_menu(
        provider_id: ProviderId,
        lang: Language,
        detail_label: &str,
    ) -> anyhow::Result<(Menu, MenuItem)> {
        let menu = Menu::new();

        let header = MenuItem::with_id(
            format!("header_{}", provider_id.cli_name()),
            provider_id.display_name(),
            false,
            None,
        );
        menu.append(&header)?;

        let detail_item = MenuItem::with_id(
            format!("status_{}", provider_id.cli_name()),
            detail_label,
            false,
            None,
        );
        menu.append(&detail_item)?;

        menu.append(&PredefinedMenuItem::separator())?;

        let open_item = MenuItem::with_id(
            format!("popout_provider_{}", provider_id.cli_name()),
            locale::get_text(lang, locale::LocaleKey::TrayProviderPopOut),
            true,
            None,
        );
        menu.append(&open_item)?;

        let refresh_item = MenuItem::with_id(
            format!("refresh_provider_{}", provider_id.cli_name()),
            locale::get_text(lang, locale::LocaleKey::TrayProviderRefresh),
            true,
            None,
        );
        menu.append(&refresh_item)?;

        menu.append(&PredefinedMenuItem::separator())?;

        let settings_item = MenuItem::with_id(
            "settings",
            locale::get_text(lang, locale::LocaleKey::TrayProviderSettings),
            true,
            None,
        );
        menu.append(&settings_item)?;

        menu.append(&PredefinedMenuItem::separator())?;

        let quit_item = MenuItem::with_id(
            "quit",
            locale::get_text(lang, locale::LocaleKey::TrayProviderQuit),
            true,
            None,
        );
        menu.append(&quit_item)?;

        Ok((menu, detail_item))
    }

    /// Sync tray icons with enabled providers
    /// Adds icons for newly enabled providers and removes icons for disabled ones
    pub fn sync_providers(&mut self, enabled_providers: &[ProviderId]) -> anyhow::Result<()> {
        // Remove icons for providers that are no longer enabled
        let enabled_set: std::collections::HashSet<_> = enabled_providers.iter().collect();
        self.provider_icons.retain(|id, _| enabled_set.contains(id));
        self.provider_menus
            .borrow_mut()
            .retain(|id, _| enabled_set.contains(id));
        self.provider_signatures
            .borrow_mut()
            .retain(|id, _| enabled_set.contains(id));
        self.tooltip_states
            .borrow_mut()
            .retain(|id, _| enabled_set.contains(id));
        self.provider_menu_detail_items
            .borrow_mut()
            .retain(|id, _| enabled_set.contains(id));
        self.provider_menu_detail_labels
            .borrow_mut()
            .retain(|id, _| enabled_set.contains(id));

        // Add icons for newly enabled providers
        for provider_id in enabled_providers {
            if !self.provider_icons.contains_key(provider_id)
                && let Ok(icon) = self.create_provider_icon(*provider_id)
            {
                self.provider_icons.insert(*provider_id, icon);
            }
        }

        Ok(())
    }

    /// Create a tray icon for a specific provider
    fn create_provider_icon(&self, provider_id: ProviderId) -> anyhow::Result<TrayIcon> {
        let lang = Settings::load().ui_language;
        let detail_label = Self::menu_detail_from_tooltip(
            provider_id,
            provider_loading_tooltip(provider_id, lang),
        );
        let (menu, detail_item) = Self::build_provider_menu(provider_id, lang, &detail_label)?;
        let icon = create_bar_icon(0.0, 0.0, IconOverlay::None);
        let tooltip = provider_loading_tooltip(provider_id, lang);

        let tray_icon = TrayIconBuilder::new()
            .with_tooltip(&tooltip)
            .with_icon(icon)
            .build()?;

        self.provider_menus.borrow_mut().insert(provider_id, menu);
        self.provider_menu_detail_items
            .borrow_mut()
            .insert(provider_id, detail_item);
        self.provider_menu_detail_labels
            .borrow_mut()
            .insert(provider_id, detail_label);

        Ok(tray_icon)
    }

    /// Update a specific provider's tray icon
    pub fn update_provider(
        &self,
        provider_id: ProviderId,
        session_percent: f64,
        weekly_percent: f64,
    ) {
        if let Some(tray_icon) = self.provider_icons.get(&provider_id) {
            // Store the state for language relocalization
            self.tooltip_states.borrow_mut().insert(
                provider_id,
                ProviderTooltipState::Normal {
                    session_percent,
                    weekly_percent,
                },
            );

            let signature = TrayManager::usage_signature(
                session_percent,
                weekly_percent,
                provider_id.display_name(),
                IconOverlay::None,
            );
            let mut sigs = self.provider_signatures.borrow_mut();
            if sigs.get(&provider_id) != Some(&signature) {
                sigs.insert(provider_id, signature);
            }

            let icon = create_bar_icon(session_percent, weekly_percent, IconOverlay::None);
            let _ = tray_icon.set_icon(Some(icon));

            let tooltip =
                locale::tray_tooltip(provider_id.display_name(), session_percent, weekly_percent);
            let _ = tray_icon.set_tooltip(Some(&tooltip));
            self.set_provider_menu_detail(
                provider_id,
                Self::menu_detail_from_tooltip(provider_id, tooltip),
            );
        }
    }

    /// Update a specific provider's tray icon with an overlay
    pub fn update_provider_with_overlay(
        &self,
        provider_id: ProviderId,
        session_percent: f64,
        weekly_percent: f64,
        overlay: IconOverlay,
    ) {
        if let Some(tray_icon) = self.provider_icons.get(&provider_id) {
            // Store the state for language relocalization
            self.tooltip_states.borrow_mut().insert(
                provider_id,
                ProviderTooltipState::WithStatus {
                    session_percent,
                    weekly_percent,
                    overlay,
                },
            );

            let signature = TrayManager::usage_signature(
                session_percent,
                weekly_percent,
                provider_id.display_name(),
                overlay,
            );
            let mut sigs = self.provider_signatures.borrow_mut();
            if sigs.get(&provider_id) != Some(&signature) {
                sigs.insert(provider_id, signature);
            }

            let icon = create_bar_icon(session_percent, weekly_percent, overlay);
            let _ = tray_icon.set_icon(Some(icon));

            let status = match overlay {
                IconOverlay::None => None,
                IconOverlay::Error => Some(IconOverlayStatus::Error),
                IconOverlay::Stale => Some(IconOverlayStatus::Stale),
                IconOverlay::Incident => Some(IconOverlayStatus::Incident),
                IconOverlay::Partial => Some(IconOverlayStatus::Partial),
            };

            let tooltip = locale::tray_tooltip_with_status(
                provider_id.display_name(),
                session_percent,
                weekly_percent,
                status,
            );
            let _ = tray_icon.set_tooltip(Some(&tooltip));
            self.set_provider_menu_detail(
                provider_id,
                Self::menu_detail_from_tooltip(provider_id, tooltip),
            );
        }
    }

    /// Show loading state for a specific provider
    pub fn show_provider_loading(
        &self,
        provider_id: ProviderId,
        pattern: LoadingPattern,
        phase: f64,
    ) {
        if let Some(tray_icon) = self.provider_icons.get(&provider_id) {
            // Store the state for language relocalization
            self.tooltip_states
                .borrow_mut()
                .insert(provider_id, ProviderTooltipState::Loading);

            let primary = pattern.value(phase);
            let secondary = pattern.value(phase + pattern.secondary_offset());

            let icon = create_loading_icon(primary, secondary);
            let _ = tray_icon.set_icon(Some(icon));
            let lang = Settings::load().ui_language;
            let loading = provider_loading_tooltip(provider_id, lang);
            let _ = tray_icon.set_tooltip(Some(&loading));
            self.set_provider_menu_detail(
                provider_id,
                Self::menu_detail_from_tooltip(provider_id, loading),
            );
        }
    }

    /// Show error state for a specific provider
    pub fn show_provider_error(&self, provider_id: ProviderId, error_msg: &str) {
        if let Some(tray_icon) = self.provider_icons.get(&provider_id) {
            // Store the state for language relocalization
            self.tooltip_states.borrow_mut().insert(
                provider_id,
                ProviderTooltipState::Error {
                    error_msg: error_msg.to_string(),
                },
            );

            let icon = create_bar_icon(0.0, 0.0, IconOverlay::Error);
            let _ = tray_icon.set_icon(Some(icon));
            let tooltip = format!("{}: {}", provider_id.display_name(), error_msg);
            let _ = tray_icon.set_tooltip(Some(&tooltip));
            self.set_provider_menu_detail(
                provider_id,
                Self::menu_detail_from_tooltip(provider_id, tooltip),
            );
        }
    }

    /// Get the number of active provider icons
    pub fn icon_count(&self) -> usize {
        self.provider_icons.len()
    }

    /// Check if a provider has an icon
    pub fn has_provider(&self, provider_id: ProviderId) -> bool {
        self.provider_icons.contains_key(&provider_id)
    }

    /// Show the right-click context menu for the provider whose tray icon
    /// has the given `icon_id`, at the current cursor position.
    ///
    /// # Safety
    /// `hwnd` must be a valid window handle on the calling thread.
    #[cfg(target_os = "windows")]
    pub fn show_context_menu(&self, hwnd: isize, icon_id: &str) {
        let provider_id = self
            .provider_icons
            .iter()
            .find(|(_, ti)| ti.id().0 == icon_id)
            .map(|(pid, _)| *pid);
        if let Some(pid) = provider_id {
            if let Some(menu) = self.provider_menus.borrow().get(&pid) {
                unsafe { menu.show_context_menu_for_hwnd(hwnd, None) };
            }
        }
    }

    /// Refresh all provider tray icons with the current language
    /// This rebuilds menus and updates tooltips for all provider icons
    pub fn refresh_language(&self) {
        let lang = Settings::load().ui_language;

        // Rebuild menus for all provider icons
        for (provider_id, _tray_icon) in &self.provider_icons {
            let detail_label = self
                .provider_menu_detail_labels
                .borrow()
                .get(provider_id)
                .cloned()
                .unwrap_or_else(|| {
                    Self::menu_detail_from_tooltip(
                        *provider_id,
                        provider_loading_tooltip(*provider_id, lang),
                    )
                });

            if let Ok((menu, detail_item)) =
                Self::build_provider_menu(*provider_id, lang, &detail_label)
            {
                self.provider_menu_detail_items
                    .borrow_mut()
                    .insert(*provider_id, detail_item);
                self.provider_menus.borrow_mut().insert(*provider_id, menu);
            }

            // Relocalize the tooltip based on the preserved state (not reset to loading)
            let tooltip = self.relocalize_provider_tooltip(*provider_id, lang);
            let _ = _tray_icon.set_tooltip(Some(&tooltip));
            self.set_provider_menu_detail(
                *provider_id,
                Self::menu_detail_from_tooltip(*provider_id, tooltip),
            );
        }
    }

    /// Relocalize a provider's tooltip based on its preserved state
    fn relocalize_provider_tooltip(
        &self,
        provider_id: ProviderId,
        lang: crate::settings::Language,
    ) -> String {
        let states = self.tooltip_states.borrow();
        let state = states.get(&provider_id);

        match state {
            None => {
                // No state yet, use loading
                provider_loading_tooltip(provider_id, lang)
            }
            Some(ProviderTooltipState::Default) => provider_loading_tooltip(provider_id, lang),
            Some(ProviderTooltipState::Normal {
                session_percent,
                weekly_percent,
            }) => locale::tray_tooltip(
                provider_id.display_name(),
                *session_percent,
                *weekly_percent,
            ),
            Some(ProviderTooltipState::WithStatus {
                session_percent,
                weekly_percent,
                overlay,
            }) => {
                let status = match overlay {
                    IconOverlay::None => None,
                    IconOverlay::Error => Some(IconOverlayStatus::Error),
                    IconOverlay::Stale => Some(IconOverlayStatus::Stale),
                    IconOverlay::Incident => Some(IconOverlayStatus::Incident),
                    IconOverlay::Partial => Some(IconOverlayStatus::Partial),
                };
                locale::tray_tooltip_with_status(
                    provider_id.display_name(),
                    *session_percent,
                    *weekly_percent,
                    status,
                )
            }
            Some(ProviderTooltipState::Loading) => provider_loading_tooltip(provider_id, lang),
            Some(ProviderTooltipState::Error { error_msg }) => {
                format!("{}: {}", provider_id.display_name(), error_msg)
            }
        }
    }

    #[cfg(debug_assertions)]
    pub fn debug_snapshot(&self) -> DebugTraySnapshot {
        let settings = Settings::load();
        let states = self.tooltip_states.borrow();
        let mut providers = states
            .iter()
            .map(|(provider_id, state)| DebugProviderTraySnapshot {
                provider: provider_id.cli_name().to_string(),
                state_kind: provider_tooltip_state_kind(state).to_string(),
                tooltip: Some(debug_provider_tooltip_text(
                    *provider_id,
                    state,
                    settings.ui_language,
                )),
            })
            .collect::<Vec<_>>();
        providers.sort_by(|a, b| a.provider.cmp(&b.provider));

        DebugTraySnapshot {
            mode: "perprovider".to_string(),
            icon_count: self.provider_icons.len(),
            state_kind: None,
            primary_provider: None,
            tooltip: None,
            providers,
        }
    }
}

/// Unified tray icon manager that supports both single and per-provider modes
pub enum UnifiedTrayManager {
    /// Single icon mode (original behavior)
    Single(TrayManager),
    /// Per-provider icon mode
    PerProvider(MultiTrayManager),
}

impl UnifiedTrayManager {
    /// Create a new unified tray manager based on settings
    pub fn new(settings: &Settings) -> anyhow::Result<Self> {
        match settings.tray_icon_mode {
            TrayIconMode::Single => Ok(UnifiedTrayManager::Single(TrayManager::new()?)),
            TrayIconMode::PerProvider => {
                let mut multi = MultiTrayManager::new()?;
                let enabled = settings.get_enabled_provider_ids();
                multi.sync_providers(&enabled)?;
                Ok(UnifiedTrayManager::PerProvider(multi))
            }
        }
    }

    /// Check if we need to recreate the manager due to mode change
    pub fn needs_mode_switch(&self, new_mode: TrayIconMode) -> bool {
        matches!(
            (self, new_mode),
            (UnifiedTrayManager::Single(_), TrayIconMode::PerProvider)
                | (UnifiedTrayManager::PerProvider(_), TrayIconMode::Single)
        )
    }

    /// Check for menu events (delegates to TrayManager's static method)
    pub fn check_events() -> Option<TrayMenuAction> {
        TrayManager::check_events()
    }

    /// Check for tray icon click events (separate from menu events).
    /// Returns a `TrayLeftClick` when the user left-clicks the tray icon,
    /// or `TrayRightClick` when the user right-clicks (so the app can show
    /// the context menu manually — no menu is attached to the tray icon).
    pub fn check_tray_click_events() -> Option<TrayMenuAction> {
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    rect,
                    ..
                } => {
                    let tray_x = rect.position.x as i32 + rect.size.width as i32 / 2;
                    let tray_y = rect.position.y as i32;
                    tracing::info!(
                        "tray left-click detected: tray_x={}, tray_y={}, rect=({},{} {}x{})",
                        tray_x,
                        tray_y,
                        rect.position.x,
                        rect.position.y,
                        rect.size.width,
                        rect.size.height,
                    );
                    return Some(TrayMenuAction::TrayLeftClick { tray_x, tray_y });
                }
                TrayIconEvent::Click {
                    button: MouseButton::Right,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    tracing::info!("tray right-click detected — requesting context menu");
                    return Some(TrayMenuAction::TrayRightClick);
                }
                _ => {}
            }
        }
        None
    }

    /// Show the right-click context menu at the current cursor position.
    ///
    /// # Safety
    /// `hwnd` must be a valid window handle on the calling thread.
    #[cfg(target_os = "windows")]
    pub fn show_context_menu(&self, hwnd: isize) {
        match self {
            UnifiedTrayManager::Single(tm) => tm.show_context_menu(hwnd),
            UnifiedTrayManager::PerProvider(multi) => {
                // In per-provider mode, show the first provider's menu.
                // (The right-click event doesn't reliably carry enough info
                // to identify which icon was clicked in all edge cases.)
                if let Some((&pid, _)) = multi.provider_icons.iter().next() {
                    if let Some(menu) = multi.provider_menus.borrow().get(&pid) {
                        unsafe { menu.show_context_menu_for_hwnd(hwnd, None) };
                    }
                }
            }
        }
    }

    /// Returns the screen rect of the first tray icon (for positioning the popup).
    pub fn rect(&self) -> Option<tray_icon::Rect> {
        match self {
            UnifiedTrayManager::Single(tm) => tm.tray_icon.rect(),
            UnifiedTrayManager::PerProvider(multi) => multi
                .provider_icons
                .values()
                .next()
                .and_then(|icon| icon.rect()),
        }
    }

    /// Show loading animation
    pub fn show_loading(&self, pattern: LoadingPattern, phase: f64) {
        match self {
            UnifiedTrayManager::Single(tm) => tm.show_loading(pattern, phase),
            UnifiedTrayManager::PerProvider(_) => {
                // In per-provider mode, we could animate all icons or skip
                // For now, skip loading animation in per-provider mode
            }
        }
    }

    /// Show surprise animation
    pub fn show_surprise(&self, anim: SurpriseAnimation, frame: u32, session: f64, weekly: f64) {
        match self {
            UnifiedTrayManager::Single(tm) => tm.show_surprise(anim, frame, session, weekly),
            UnifiedTrayManager::PerProvider(_) => {
                // Skip surprise in per-provider mode
            }
        }
    }

    pub fn show_error(&self, provider_name: &str, error_msg: &str) {
        if let UnifiedTrayManager::Single(tm) = self {
            tm.show_error(provider_name, error_msg);
        }
    }

    /// Update usage for a single provider display
    pub fn update_usage(&self, session_percent: f64, weekly_percent: f64, tooltip_name: &str) {
        match self {
            UnifiedTrayManager::Single(tm) => {
                tm.update_usage(session_percent, weekly_percent, tooltip_name)
            }
            UnifiedTrayManager::PerProvider(_) => {
                // Per-provider mode doesn't use single update
            }
        }
    }

    pub fn update_provider_usage(
        &self,
        provider_id: ProviderId,
        session_percent: f64,
        weekly_percent: f64,
    ) {
        if let UnifiedTrayManager::PerProvider(multi) = self {
            multi.update_provider(provider_id, session_percent, weekly_percent);
        }
    }

    pub fn show_provider_error(&self, provider_id: ProviderId, error_msg: &str) {
        if let UnifiedTrayManager::PerProvider(multi) = self {
            multi.show_provider_error(provider_id, error_msg);
        }
    }

    /// Update merged display for all providers
    pub fn update_merged(&self, usages: &[ProviderUsage]) {
        match self {
            UnifiedTrayManager::Single(tm) => tm.update_merged(usages),
            UnifiedTrayManager::PerProvider(multi) => {
                // Update each provider's individual icon
                for usage in usages {
                    if let Some(id) = crate::core::ProviderId::from_cli_name(&usage.name) {
                        multi.update_provider(id, usage.session_percent, usage.weekly_percent);
                    }
                }
            }
        }
    }

    #[cfg(debug_assertions)]
    pub fn set_single_state_for_testing(
        &self,
        state: &str,
        provider_name: Option<&str>,
        session_percent: Option<f64>,
        weekly_percent: Option<f64>,
        error: Option<&str>,
    ) {
        if let UnifiedTrayManager::Single(tm) = self {
            match state {
                "loading" => tm.show_loading(LoadingPattern::default(), 0.0),
                "error" => tm.show_error(
                    provider_name.unwrap_or("CodexBar"),
                    error.unwrap_or("Test error"),
                ),
                _ => {
                    let session_percent = session_percent.unwrap_or(0.0);
                    let weekly_percent = weekly_percent.unwrap_or(session_percent);
                    tm.update_usage(
                        session_percent,
                        weekly_percent,
                        provider_name.unwrap_or("CodexBar"),
                    );
                }
            }
        }
    }

    #[cfg(debug_assertions)]
    pub fn set_provider_state_for_testing(
        &self,
        provider_id: ProviderId,
        state: &str,
        session_percent: Option<f64>,
        weekly_percent: Option<f64>,
        error: Option<&str>,
    ) {
        if let UnifiedTrayManager::PerProvider(multi) = self {
            match state {
                "loading" => {
                    multi.show_provider_loading(provider_id, LoadingPattern::default(), 0.0);
                }
                "error" => {
                    multi.show_provider_error(provider_id, error.unwrap_or("Test error"));
                }
                _ => {
                    let session_percent = session_percent.unwrap_or(0.0);
                    let weekly_percent = weekly_percent.unwrap_or(session_percent);
                    multi.update_provider(provider_id, session_percent, weekly_percent);
                }
            }
        }
    }

    /// Refresh tray menu and tooltip with the current language
    /// This is called when the user changes the language setting
    pub fn refresh_language(&self) {
        match self {
            UnifiedTrayManager::Single(tm) => tm.refresh_language(),
            UnifiedTrayManager::PerProvider(multi) => multi.refresh_language(),
        }
    }

    #[cfg(debug_assertions)]
    pub fn debug_snapshot(&self) -> DebugTraySnapshot {
        match self {
            UnifiedTrayManager::Single(tm) => tm.debug_snapshot(),
            UnifiedTrayManager::PerProvider(multi) => multi.debug_snapshot(),
        }
    }
}

/// Create a bar icon showing session and weekly usage with optional overlay
fn create_bar_icon(session_percent: f64, weekly_percent: f64, overlay: IconOverlay) -> Icon {
    let mut img: RgbaImage = ImageBuffer::new(ICON_SIZE, ICON_SIZE);

    // Fill with transparent background
    for pixel in img.pixels_mut() {
        *pixel = Rgba([0, 0, 0, 0]);
    }

    // Draw rounded background - dimmed if error/stale
    let bg_alpha = match overlay {
        IconOverlay::Error | IconOverlay::Stale => 180,
        _ => 255,
    };
    let bg_color = Rgba([60, 60, 70, bg_alpha]);
    for y in 2..ICON_SIZE - 2 {
        for x in 2..ICON_SIZE - 2 {
            img.put_pixel(x, y, bg_color);
        }
    }

    // Bar dimensions
    let bar_left = 4u32;
    let bar_right = ICON_SIZE - 4;
    let bar_width = bar_right - bar_left;

    // Color adjustment for error/stale states
    let color_adjust = |r: u8, g: u8, b: u8| -> (u8, u8, u8) {
        match overlay {
            IconOverlay::Error => {
                // Grayscale
                let gray = ((r as u16 + g as u16 + b as u16) / 3) as u8;
                (gray, gray, gray)
            }
            IconOverlay::Stale => {
                // Dim colors by 40%
                (
                    (r as f32 * 0.6) as u8,
                    (g as f32 * 0.6) as u8,
                    (b as f32 * 0.6) as u8,
                )
            }
            _ => (r, g, b),
        }
    };

    // Session bar (top, thicker) - y: 8 to 14
    let session_level = UsageLevel::from_percent(session_percent);
    let (sr, sg, sb) = session_level.color();
    let (sr, sg, sb) = color_adjust(sr, sg, sb);
    let session_fill = ((session_percent / 100.0) * bar_width as f64) as u32;

    // Track (gray)
    for y in 8..15 {
        for x in bar_left..bar_right {
            img.put_pixel(x, y, Rgba([80, 80, 90, 255]));
        }
    }
    // Fill (colored)
    for y in 8..15 {
        for x in bar_left..(bar_left + session_fill).min(bar_right) {
            img.put_pixel(x, y, Rgba([sr, sg, sb, 255]));
        }
    }

    // Weekly bar (bottom, thinner) - y: 18 to 22
    let weekly_level = UsageLevel::from_percent(weekly_percent);
    let (wr, wg, wb) = weekly_level.color();
    let (wr, wg, wb) = color_adjust(wr, wg, wb);
    let weekly_fill = ((weekly_percent / 100.0) * bar_width as f64) as u32;

    // Track (gray)
    for y in 18..23 {
        for x in bar_left..bar_right {
            img.put_pixel(x, y, Rgba([80, 80, 90, 255]));
        }
    }
    // Fill (colored)
    for y in 18..23 {
        for x in bar_left..(bar_left + weekly_fill).min(bar_right) {
            img.put_pixel(x, y, Rgba([wr, wg, wb, 255]));
        }
    }

    // Draw overlay badge
    draw_overlay_badge(&mut img, overlay);

    let rgba = img.into_raw();
    Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).expect("Failed to create icon")
}

/// Draw overlay badge on the icon (bottom-right corner)
fn draw_overlay_badge(img: &mut RgbaImage, overlay: IconOverlay) {
    match overlay {
        IconOverlay::None => {}
        IconOverlay::Error => {
            // Red X in bottom-right corner
            let badge_color = Rgba([255, 60, 60, 255]);
            // Draw a small X (6x6 pixels in corner)
            for i in 0..6 {
                // Diagonal line \
                let x = ICON_SIZE - 8 + i;
                let y = ICON_SIZE - 8 + i;
                if x < ICON_SIZE && y < ICON_SIZE {
                    img.put_pixel(x, y, badge_color);
                }
                // Diagonal line /
                let x2 = ICON_SIZE - 3 - i;
                let y2 = ICON_SIZE - 8 + i;
                if x2 < ICON_SIZE && y2 < ICON_SIZE {
                    img.put_pixel(x2, y2, badge_color);
                }
            }
        }
        IconOverlay::Stale => {
            // Clock indicator - small dot in corner
            let badge_color = Rgba([180, 180, 180, 255]);
            // Draw a small circle (clock symbol)
            for dy in 0..4 {
                for dx in 0..4 {
                    let x = ICON_SIZE - 6 + dx;
                    let y = ICON_SIZE - 6 + dy;
                    if x < ICON_SIZE && y < ICON_SIZE {
                        img.put_pixel(x, y, badge_color);
                    }
                }
            }
        }
        IconOverlay::Incident => {
            // Red warning badge
            let badge_color = Rgba([244, 67, 54, 255]);
            // Draw filled circle in corner
            for dy in 0..6 {
                for dx in 0..6 {
                    let x = ICON_SIZE - 8 + dx;
                    let y = ICON_SIZE - 8 + dy;
                    if x < ICON_SIZE && y < ICON_SIZE {
                        img.put_pixel(x, y, badge_color);
                    }
                }
            }
        }
        IconOverlay::Partial => {
            // Orange warning badge
            let badge_color = Rgba([255, 152, 0, 255]);
            // Draw filled circle in corner
            for dy in 0..6 {
                for dx in 0..6 {
                    let x = ICON_SIZE - 8 + dx;
                    let y = ICON_SIZE - 8 + dy;
                    if x < ICON_SIZE && y < ICON_SIZE {
                        img.put_pixel(x, y, badge_color);
                    }
                }
            }
        }
    }
}

/// Create a credits icon showing a thick single bar for credits mode
/// Used when weekly quota is exhausted but paid credits remain
fn create_credits_icon(credits_percent: f64) -> Icon {
    let mut img: RgbaImage = ImageBuffer::new(ICON_SIZE, ICON_SIZE);

    // Fill with transparent background
    for pixel in img.pixels_mut() {
        *pixel = Rgba([0, 0, 0, 0]);
    }

    // Draw rounded background
    let bg_color = Rgba([60, 60, 70, 255]);
    for y in 2..ICON_SIZE - 2 {
        for x in 2..ICON_SIZE - 2 {
            img.put_pixel(x, y, bg_color);
        }
    }

    // Bar dimensions - thick bar for credits (16px like macOS version)
    let bar_left = 4u32;
    let bar_right = ICON_SIZE - 4;
    let bar_width = bar_right - bar_left;

    // Credits bar - centered and thick (y: 8 to 24)
    let bar_y_start = 8u32;
    let bar_y_end = 24u32;

    // Cyan/blue color for credits
    let credits_color = Rgba([64, 196, 255, 255]);
    let credits_fill = ((credits_percent / 100.0) * bar_width as f64) as u32;

    // Track (gray)
    for y in bar_y_start..bar_y_end {
        for x in bar_left..bar_right {
            img.put_pixel(x, y, Rgba([80, 80, 90, 255]));
        }
    }
    // Fill (cyan)
    for y in bar_y_start..bar_y_end {
        for x in bar_left..(bar_left + credits_fill).min(bar_right) {
            img.put_pixel(x, y, credits_color);
        }
    }

    let rgba = img.into_raw();
    Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).expect("Failed to create icon")
}

/// Create a merged icon showing multiple providers stacked
fn create_merged_icon(providers: &[ProviderUsage]) -> Icon {
    let mut img: RgbaImage = ImageBuffer::new(ICON_SIZE, ICON_SIZE);

    // Fill with transparent background
    for pixel in img.pixels_mut() {
        *pixel = Rgba([0, 0, 0, 0]);
    }

    // Draw rounded background
    let bg_color = Rgba([60, 60, 70, 255]);
    for y in 2..ICON_SIZE - 2 {
        for x in 2..ICON_SIZE - 2 {
            img.put_pixel(x, y, bg_color);
        }
    }

    // Bar dimensions
    let bar_left = 4u32;
    let bar_right = ICON_SIZE - 4;
    let bar_width = bar_right - bar_left;

    // Calculate bar positions based on provider count
    let provider_count = providers.len().min(4); // Max 4 bars
    if provider_count == 0 {
        let rgba = img.into_raw();
        return Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).expect("Failed to create icon");
    }

    // Calculate bar height and spacing to fit within icon
    let total_height = ICON_SIZE - 8; // Leave margin
    let bar_height = (total_height / provider_count as u32).min(6);
    let spacing = if provider_count > 1 {
        (total_height - (bar_height * provider_count as u32)) / (provider_count as u32 - 1).max(1)
    } else {
        0
    };

    for (i, provider) in providers.iter().take(4).enumerate() {
        let y_start = 4 + (i as u32 * (bar_height + spacing));
        let y_end = (y_start + bar_height).min(ICON_SIZE - 4);

        let level = UsageLevel::from_percent(provider.session_percent);
        let (r, g, b) = level.color();
        let fill_width = ((provider.session_percent / 100.0) * bar_width as f64) as u32;

        // Draw track (gray)
        for y in y_start..y_end {
            for x in bar_left..bar_right {
                img.put_pixel(x, y, Rgba([80, 80, 90, 255]));
            }
        }

        // Draw fill (colored)
        for y in y_start..y_end {
            for x in bar_left..(bar_left + fill_width).min(bar_right) {
                img.put_pixel(x, y, Rgba([r, g, b, 255]));
            }
        }
    }

    let rgba = img.into_raw();
    Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).expect("Failed to create icon")
}

/// Create a loading animation icon with animated bars
fn create_loading_icon(primary_percent: f64, secondary_percent: f64) -> Icon {
    let mut img: RgbaImage = ImageBuffer::new(ICON_SIZE, ICON_SIZE);

    // Fill with transparent background
    for pixel in img.pixels_mut() {
        *pixel = Rgba([0, 0, 0, 0]);
    }

    // Draw rounded background
    let bg_color = Rgba([60, 60, 70, 255]);
    for y in 2..ICON_SIZE - 2 {
        for x in 2..ICON_SIZE - 2 {
            img.put_pixel(x, y, bg_color);
        }
    }

    // Bar dimensions
    let bar_left = 4u32;
    let bar_right = ICON_SIZE - 4;
    let bar_width = bar_right - bar_left;

    // Loading color - cyan/blue gradient
    let loading_color = Rgba([64, 196, 255, 255]);

    // Primary bar (top) - y: 8 to 14
    let primary_fill = ((primary_percent / 100.0) * bar_width as f64) as u32;
    for y in 8..15 {
        for x in bar_left..bar_right {
            img.put_pixel(x, y, Rgba([80, 80, 90, 255]));
        }
    }
    for y in 8..15 {
        for x in bar_left..(bar_left + primary_fill).min(bar_right) {
            img.put_pixel(x, y, loading_color);
        }
    }

    // Secondary bar (bottom) - y: 18 to 22
    let secondary_fill = ((secondary_percent / 100.0) * bar_width as f64) as u32;
    for y in 18..23 {
        for x in bar_left..bar_right {
            img.put_pixel(x, y, Rgba([80, 80, 90, 255]));
        }
    }
    for y in 18..23 {
        for x in bar_left..(bar_left + secondary_fill).min(bar_right) {
            img.put_pixel(x, y, loading_color);
        }
    }

    let rgba = img.into_raw();
    Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).expect("Failed to create icon")
}

/// Create a surprise animation icon frame
fn create_surprise_icon(
    animation: SurpriseAnimation,
    frame: u32,
    session_percent: f64,
    weekly_percent: f64,
) -> Icon {
    let mut img: RgbaImage = ImageBuffer::new(ICON_SIZE, ICON_SIZE);

    // Fill with transparent background
    for pixel in img.pixels_mut() {
        *pixel = Rgba([0, 0, 0, 0]);
    }

    // Draw rounded background
    let bg_color = Rgba([60, 60, 70, 255]);
    for y in 2..ICON_SIZE - 2 {
        for x in 2..ICON_SIZE - 2 {
            img.put_pixel(x, y, bg_color);
        }
    }

    // Bar dimensions
    let bar_left = 4u32;
    let bar_right = ICON_SIZE - 4;
    let bar_width = bar_right - bar_left;

    // Calculate animation parameters
    let total_frames = animation.duration_frames().max(1);
    let progress = frame as f64 / total_frames as f64;

    // Color and position modifiers based on animation type
    let (color_mod, x_offset, y_offset) = match animation {
        SurpriseAnimation::None => ((1.0, 1.0, 1.0), 0i32, 0i32),
        SurpriseAnimation::Blink => {
            // Flash to white and back
            let flash = if progress < 0.5 {
                progress * 2.0 // Fade to white
            } else {
                (1.0 - progress) * 2.0 // Fade back
            };
            let blend = 1.0 + flash * 0.8; // Boost brightness
            ((blend, blend, blend), 0, 0)
        }
        SurpriseAnimation::Wiggle => {
            // Shake left and right
            let shake = (progress * std::f64::consts::PI * 6.0).sin(); // 3 full oscillations
            let offset = (shake * 2.0) as i32; // +/- 2 pixels
            ((1.0, 1.0, 1.0), offset, 0)
        }
        SurpriseAnimation::Pulse => {
            // Gentle pulse - grow and shrink brightness
            let pulse = (progress * std::f64::consts::PI * 2.0).sin(); // One full cycle
            let intensity = 1.0 + pulse * 0.3; // +/- 30% brightness
            ((intensity, intensity, intensity), 0, 0)
        }
        SurpriseAnimation::Rainbow => {
            // Sweep through rainbow colors
            let hue = progress * 360.0;
            let (r, g, b) = hsv_to_rgb(hue, 0.8, 1.0);
            (
                (
                    r as f64 / 255.0 * 2.0,
                    g as f64 / 255.0 * 2.0,
                    b as f64 / 255.0 * 2.0,
                ),
                0,
                0,
            )
        }
        SurpriseAnimation::Tilt => {
            // Tilt effect - slight diagonal shift that returns
            let tilt = (progress * std::f64::consts::PI).sin(); // 0 -> 1 -> 0
            let x_off = (tilt * 2.0) as i32; // +2 pixels at peak
            let y_off = (tilt * 1.0) as i32; // +1 pixel at peak (slight diagonal)
            ((1.0, 1.0, 1.0), x_off, y_off)
        }
    };

    // Session bar (top) - y: 8 to 14
    let session_level = UsageLevel::from_percent(session_percent);
    let (sr, sg, sb) = session_level.color();
    let sr = ((sr as f64 * color_mod.0).min(255.0)) as u8;
    let sg = ((sg as f64 * color_mod.1).min(255.0)) as u8;
    let sb = ((sb as f64 * color_mod.2).min(255.0)) as u8;
    let session_fill = ((session_percent / 100.0) * bar_width as f64) as u32;

    // Track (gray)
    for y in 8..15 {
        for x in bar_left..bar_right {
            let adjusted_x = (x as i32 + x_offset)
                .max(bar_left as i32)
                .min(bar_right as i32 - 1) as u32;
            let adjusted_y = (y + y_offset).max(4).min(ICON_SIZE as i32 - 4) as u32;
            img.put_pixel(adjusted_x, adjusted_y, Rgba([80, 80, 90, 255]));
        }
    }
    // Fill (colored with animation)
    for y in 8..15 {
        for x in bar_left..(bar_left + session_fill).min(bar_right) {
            let adjusted_x = (x as i32 + x_offset)
                .max(bar_left as i32)
                .min(bar_right as i32 - 1) as u32;
            let adjusted_y = (y + y_offset).max(4).min(ICON_SIZE as i32 - 4) as u32;
            img.put_pixel(adjusted_x, adjusted_y, Rgba([sr, sg, sb, 255]));
        }
    }

    // Weekly bar (bottom) - y: 18 to 22
    let weekly_level = UsageLevel::from_percent(weekly_percent);
    let (wr, wg, wb) = weekly_level.color();
    let wr = ((wr as f64 * color_mod.0).min(255.0)) as u8;
    let wg = ((wg as f64 * color_mod.1).min(255.0)) as u8;
    let wb = ((wb as f64 * color_mod.2).min(255.0)) as u8;
    let weekly_fill = ((weekly_percent / 100.0) * bar_width as f64) as u32;

    // Track (gray)
    for y in 18..23 {
        for x in bar_left..bar_right {
            let adjusted_x = (x as i32 + x_offset)
                .max(bar_left as i32)
                .min(bar_right as i32 - 1) as u32;
            let adjusted_y = (y + y_offset).max(4).min(ICON_SIZE as i32 - 4) as u32;
            img.put_pixel(adjusted_x, adjusted_y, Rgba([80, 80, 90, 255]));
        }
    }
    // Fill (colored with animation)
    for y in 18..23 {
        for x in bar_left..(bar_left + weekly_fill).min(bar_right) {
            let adjusted_x = (x as i32 + x_offset)
                .max(bar_left as i32)
                .min(bar_right as i32 - 1) as u32;
            let adjusted_y = (y + y_offset).max(4).min(ICON_SIZE as i32 - 4) as u32;
            img.put_pixel(adjusted_x, adjusted_y, Rgba([wr, wg, wb, 255]));
        }
    }

    let rgba = img.into_raw();
    Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).expect("Failed to create icon")
}

/// Create a morph animation icon frame (logo/knot -> bars transition)
/// This is the "Unbraid" animation from Swift - morphs from interlaced ribbons to usage bars
fn create_morph_icon(progress: f64, session_percent: f64, weekly_percent: f64) -> Icon {
    let mut img: RgbaImage = ImageBuffer::new(ICON_SIZE, ICON_SIZE);
    let t = progress.clamp(0.0, 1.0) as f32;

    // Fill with transparent background
    for pixel in img.pixels_mut() {
        *pixel = Rgba([0, 0, 0, 0]);
    }

    // Draw rounded background
    let bg_color = Rgba([60, 60, 70, 255]);
    for y in 2..ICON_SIZE - 2 {
        for x in 2..ICON_SIZE - 2 {
            img.put_pixel(x, y, bg_color);
        }
    }

    let center_x = ICON_SIZE as f32 / 2.0;
    let center_y = ICON_SIZE as f32 / 2.0;
    let ribbon_color = Rgba([200, 200, 210, 255]);

    // Morphing segments - three ribbons that transform into two bars
    // Segment 1: Upper ribbon -> top bar
    let seg1_start_y = center_y + 2.0;
    let seg1_end_y = 11.0; // Final top bar position
    let seg1_y = lerp(seg1_start_y, seg1_end_y, t);
    let seg1_start_angle = -30.0_f32;
    let seg1_end_angle = 0.0_f32;
    let seg1_angle = lerp(seg1_start_angle, seg1_end_angle, t);
    let seg1_start_len = 16.0_f32;
    let seg1_end_len = 24.0_f32;
    let seg1_len = lerp(seg1_start_len, seg1_end_len, t);
    let seg1_thickness = lerp(3.5, 7.0, t);

    draw_rotated_ribbon(
        &mut img,
        center_x,
        seg1_y,
        seg1_len,
        seg1_thickness,
        seg1_angle,
        ribbon_color,
    );

    // Segment 2: Lower ribbon -> bottom bar
    let seg2_start_y = center_y - 2.0;
    let seg2_end_y = 20.0; // Final bottom bar position
    let seg2_y = lerp(seg2_start_y, seg2_end_y, t);
    let seg2_start_angle = 210.0_f32 - 180.0; // Normalize to -30 to 30 range
    let seg2_end_angle = 0.0_f32;
    let seg2_angle = lerp(seg2_start_angle, seg2_end_angle, t);
    let seg2_start_len = 16.0_f32;
    let seg2_end_len = 24.0_f32;
    let seg2_len = lerp(seg2_start_len, seg2_end_len, t);
    let seg2_thickness = lerp(3.5, 5.0, t);

    draw_rotated_ribbon(
        &mut img,
        center_x,
        seg2_y,
        seg2_len,
        seg2_thickness,
        seg2_angle,
        ribbon_color,
    );

    // Segment 3: Side ribbon that fades out
    let seg3_alpha = ((1.0 - t * 1.1).max(0.0) * 255.0) as u8;
    if seg3_alpha > 10 {
        let seg3_y = lerp(center_y, center_y - 6.0, t);
        let seg3_angle = lerp(90.0, 0.0, t);
        let seg3_len = lerp(16.0, 8.0, t);
        let seg3_thickness = lerp(3.5, 1.8, t);
        let fading_color = Rgba([200, 200, 210, seg3_alpha]);
        draw_rotated_ribbon(
            &mut img,
            center_x,
            seg3_y,
            seg3_len,
            seg3_thickness,
            seg3_angle,
            fading_color,
        );
    }

    // Cross-fade in colored fill bars near the end of the morph
    if t > 0.55 {
        let bar_t = ((t - 0.55) / 0.45).min(1.0);
        let bar_alpha = (bar_t * 200.0) as u8;

        // Bar dimensions
        let bar_left = 4u32;
        let bar_right = ICON_SIZE - 4;
        let bar_width = bar_right - bar_left;

        // Session bar fill color
        let session_level = UsageLevel::from_percent(session_percent);
        let (sr, sg, sb) = session_level.color();
        let session_fill = ((session_percent / 100.0) * bar_width as f64) as u32;

        // Draw session bar fill with alpha
        for y in 8..15 {
            for x in bar_left..(bar_left + session_fill).min(bar_right) {
                let existing = img.get_pixel(x, y);
                let blended = blend_alpha(existing, &Rgba([sr, sg, sb, bar_alpha]));
                img.put_pixel(x, y, blended);
            }
        }

        // Weekly bar fill color
        let weekly_level = UsageLevel::from_percent(weekly_percent);
        let (wr, wg, wb) = weekly_level.color();
        let weekly_fill = ((weekly_percent / 100.0) * bar_width as f64) as u32;

        // Draw weekly bar fill with alpha
        for y in 18..23 {
            for x in bar_left..(bar_left + weekly_fill).min(bar_right) {
                let existing = img.get_pixel(x, y);
                let blended = blend_alpha(existing, &Rgba([wr, wg, wb, bar_alpha]));
                img.put_pixel(x, y, blended);
            }
        }
    }

    let rgba = img.into_raw();
    Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).expect("Failed to create icon")
}

/// Draw a rotated ribbon/rounded rectangle
fn draw_rotated_ribbon(
    img: &mut RgbaImage,
    cx: f32,
    cy: f32,
    length: f32,
    thickness: f32,
    angle_deg: f32,
    color: Rgba<u8>,
) {
    let angle_rad = angle_deg.to_radians();
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();

    let half_len = length / 2.0;
    let half_thick = thickness / 2.0;

    // Draw the ribbon by iterating over a bounding box and checking if pixels are inside
    let bound = (half_len + half_thick) as i32 + 2;

    for dy in -bound..=bound {
        for dx in -bound..=bound {
            // Rotate point back to ribbon-local coordinates
            let px = dx as f32 * cos_a + dy as f32 * sin_a;
            let py = -dx as f32 * sin_a + dy as f32 * cos_a;

            // Check if inside rounded rectangle
            let in_length = px.abs() <= half_len;
            let in_thickness = py.abs() <= half_thick;

            // Rounded ends
            let in_left_cap = (px + half_len).powi(2) + py.powi(2) <= half_thick.powi(2);
            let in_right_cap = (px - half_len).powi(2) + py.powi(2) <= half_thick.powi(2);

            if (in_length && in_thickness) || in_left_cap || in_right_cap {
                let final_x = (cx + dx as f32) as i32;
                let final_y = (cy + dy as f32) as i32;

                if final_x >= 0
                    && final_x < ICON_SIZE as i32
                    && final_y >= 0
                    && final_y < ICON_SIZE as i32
                {
                    let existing = img.get_pixel(final_x as u32, final_y as u32);
                    let blended = blend_alpha(existing, &color);
                    img.put_pixel(final_x as u32, final_y as u32, blended);
                }
            }
        }
    }
}

/// Linear interpolation
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Blend two colors with alpha
fn blend_alpha(base: &Rgba<u8>, overlay: &Rgba<u8>) -> Rgba<u8> {
    let oa = overlay[3] as f32 / 255.0;
    let ba = base[3] as f32 / 255.0;

    if oa < 0.01 {
        return *base;
    }

    let out_a = oa + ba * (1.0 - oa);
    if out_a < 0.01 {
        return Rgba([0, 0, 0, 0]);
    }

    let r = (overlay[0] as f32 * oa + base[0] as f32 * ba * (1.0 - oa)) / out_a;
    let g = (overlay[1] as f32 * oa + base[1] as f32 * ba * (1.0 - oa)) / out_a;
    let b = (overlay[2] as f32 * oa + base[2] as f32 * ba * (1.0 - oa)) / out_a;

    Rgba([r as u8, g as u8, b as u8, (out_a * 255.0) as u8])
}

/// Convert HSV to RGB (h: 0-360, s: 0-1, v: 0-1)
fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_bar_icon() {
        // Just verify it doesn't panic
        let _icon = create_bar_icon(50.0, 25.0, IconOverlay::None);
        let _icon = create_bar_icon(0.0, 0.0, IconOverlay::None);
        let _icon = create_bar_icon(100.0, 100.0, IconOverlay::None);
    }

    #[test]
    fn test_create_bar_icon_with_overlays() {
        // Test all overlay types
        let _icon = create_bar_icon(50.0, 25.0, IconOverlay::Error);
        let _icon = create_bar_icon(50.0, 25.0, IconOverlay::Stale);
        let _icon = create_bar_icon(50.0, 25.0, IconOverlay::Incident);
        let _icon = create_bar_icon(50.0, 25.0, IconOverlay::Partial);
    }

    #[test]
    fn test_usage_signature_detects_changes() {
        let sig1 = TrayManager::usage_signature(10.0, 20.0, "Claude", IconOverlay::None);
        let sig1_repeat = TrayManager::usage_signature(10.0, 20.0, "Claude", IconOverlay::None);
        let sig_overlay = TrayManager::usage_signature(10.0, 20.0, "Claude", IconOverlay::Error);
        let sig_provider = TrayManager::usage_signature(10.0, 20.0, "Codex", IconOverlay::None);
        let sig_values = TrayManager::usage_signature(11.0, 20.0, "Claude", IconOverlay::None);

        assert_eq!(sig1, sig1_repeat, "same inputs should yield same signature");
        assert_ne!(sig1, sig_overlay, "overlay changes should change signature");
        assert_ne!(
            sig1, sig_provider,
            "provider name changes should change signature"
        );
        assert_ne!(
            sig1, sig_values,
            "usage values changes should change signature"
        );
    }

    #[test]
    fn test_merged_signature_tracks_list_content() {
        let providers_a = vec![
            ProviderUsage {
                name: "Claude".into(),
                session_percent: 10.0,
                weekly_percent: 20.0,
            },
            ProviderUsage {
                name: "Codex".into(),
                session_percent: 30.0,
                weekly_percent: 40.0,
            },
        ];
        let providers_b = vec![
            ProviderUsage {
                name: "Claude".into(),
                session_percent: 10.0,
                weekly_percent: 20.0,
            },
            ProviderUsage {
                name: "Codex".into(),
                session_percent: 30.0,
                weekly_percent: 50.0,
            },
        ];
        let providers_c = vec![ProviderUsage {
            name: "Claude".into(),
            session_percent: 10.0,
            weekly_percent: 20.0,
        }];

        let sig_a1 = TrayManager::merged_signature(&providers_a);
        let sig_a2 = TrayManager::merged_signature(&providers_a);
        let sig_b = TrayManager::merged_signature(&providers_b);
        let sig_c = TrayManager::merged_signature(&providers_c);

        assert_eq!(
            sig_a1, sig_a2,
            "same provider list should yield stable signature"
        );
        assert_ne!(sig_a1, sig_b, "value change should alter signature");
        assert_ne!(
            sig_a1, sig_c,
            "length/content change should alter signature"
        );
    }

    #[test]
    fn test_tray_action_from_event_id_maps_provider_open() {
        // Per-provider tray menus still use popout_provider_ for explicit popout items
        assert_eq!(
            tray_action_from_event_id("popout_provider_codex"),
            Some(TrayMenuAction::PopOutProvider("codex".to_string()))
        );
    }

    #[test]
    fn test_tray_action_status_rows_are_inert() {
        // Single-tray status rows must not trigger any action
        assert_eq!(tray_action_from_event_id("status_claude"), None);
        assert_eq!(tray_action_from_event_id("status_codex"), None);
        assert_eq!(tray_action_from_event_id("status_cursor"), None);
    }

    #[test]
    fn test_tray_action_from_event_id_maps_provider_refresh() {
        assert_eq!(
            tray_action_from_event_id("refresh_provider_cursor"),
            Some(TrayMenuAction::RefreshProvider("cursor".to_string()))
        );
    }

    #[test]
    fn test_tray_action_from_event_id_maps_provider_toggle() {
        assert_eq!(
            tray_action_from_event_id("provider_claude"),
            Some(TrayMenuAction::ToggleProvider("claude".to_string()))
        );
    }

    #[test]
    fn test_tray_tooltip_state_preserves_normal_state() {
        let state = TrayTooltipState::Normal {
            provider_name: "Claude".to_string(),
            session_percent: 50.0,
            weekly_percent: 25.0,
        };

        match &state {
            TrayTooltipState::Normal {
                provider_name,
                session_percent,
                weekly_percent,
            } => {
                assert_eq!(provider_name, "Claude");
                assert_eq!(*session_percent, 50.0);
                assert_eq!(*weekly_percent, 25.0);
            }
            _ => panic!("Expected Normal state"),
        }
    }

    #[test]
    fn test_tray_tooltip_state_preserves_with_status() {
        let state = TrayTooltipState::WithStatus {
            provider_name: "Codex".to_string(),
            session_percent: 75.0,
            weekly_percent: 30.0,
            overlay: IconOverlay::Error,
        };

        match &state {
            TrayTooltipState::WithStatus {
                provider_name,
                session_percent,
                weekly_percent,
                overlay,
            } => {
                assert_eq!(provider_name, "Codex");
                assert_eq!(*session_percent, 75.0);
                assert_eq!(*weekly_percent, 30.0);
                assert_eq!(*overlay, IconOverlay::Error);
            }
            _ => panic!("Expected WithStatus state"),
        }
    }

    #[test]
    fn test_tray_tooltip_state_preserves_credits() {
        let state = TrayTooltipState::Credits {
            provider_name: "OpenAI".to_string(),
            credits_percent: 80.0,
        };

        match &state {
            TrayTooltipState::Credits {
                provider_name,
                credits_percent,
            } => {
                assert_eq!(provider_name, "OpenAI");
                assert_eq!(*credits_percent, 80.0);
            }
            _ => panic!("Expected Credits state"),
        }
    }

    #[test]
    fn test_tray_tooltip_state_preserves_error() {
        let state = TrayTooltipState::Error {
            provider_name: "Claude".to_string(),
            error_msg: "Connection failed".to_string(),
        };

        match &state {
            TrayTooltipState::Error {
                provider_name,
                error_msg,
            } => {
                assert_eq!(provider_name, "Claude");
                assert_eq!(error_msg, "Connection failed");
            }
            _ => panic!("Expected Error state"),
        }
    }

    #[test]
    fn test_tray_tooltip_state_default_is_default() {
        let state: TrayTooltipState = Default::default();
        assert!(matches!(state, TrayTooltipState::Default));
    }

    #[test]
    fn test_provider_tooltip_state_preserves_normal() {
        let state = ProviderTooltipState::Normal {
            session_percent: 60.0,
            weekly_percent: 40.0,
        };

        match &state {
            ProviderTooltipState::Normal {
                session_percent,
                weekly_percent,
            } => {
                assert_eq!(*session_percent, 60.0);
                assert_eq!(*weekly_percent, 40.0);
            }
            _ => panic!("Expected Normal state"),
        }
    }

    #[test]
    fn test_provider_tooltip_state_preserves_with_status() {
        let state = ProviderTooltipState::WithStatus {
            session_percent: 70.0,
            weekly_percent: 35.0,
            overlay: IconOverlay::Incident,
        };

        match &state {
            ProviderTooltipState::WithStatus {
                session_percent,
                weekly_percent,
                overlay,
            } => {
                assert_eq!(*session_percent, 70.0);
                assert_eq!(*weekly_percent, 35.0);
                assert_eq!(*overlay, IconOverlay::Incident);
            }
            _ => panic!("Expected WithStatus state"),
        }
    }

    #[test]
    fn test_provider_tooltip_state_preserves_error() {
        let state = ProviderTooltipState::Error {
            error_msg: "Auth failed".to_string(),
        };

        match &state {
            ProviderTooltipState::Error { error_msg } => {
                assert_eq!(error_msg, "Auth failed");
            }
            _ => panic!("Expected Error state"),
        }
    }

    #[test]
    fn test_provider_tooltip_state_default_is_default() {
        let state: ProviderTooltipState = Default::default();
        assert!(matches!(state, ProviderTooltipState::Default));
    }

    #[test]
    fn test_tray_tooltip_state_merged_preserves_providers() {
        let providers = vec![
            ProviderUsage {
                name: "Claude".into(),
                session_percent: 10.0,
                weekly_percent: 20.0,
            },
            ProviderUsage {
                name: "Codex".into(),
                session_percent: 30.0,
                weekly_percent: 40.0,
            },
        ];

        let state = TrayTooltipState::Merged {
            providers: providers.clone(),
        };

        match &state {
            TrayTooltipState::Merged { providers: p } => {
                assert_eq!(p.len(), 2);
                assert_eq!(p[0].name, "Claude");
                assert_eq!(p[1].name, "Codex");
            }
            _ => panic!("Expected Merged state"),
        }
    }

    #[test]
    fn debug_kind_reports_merged_state() {
        let state = TrayTooltipState::Merged {
            providers: vec![ProviderUsage {
                name: "Claude".into(),
                session_percent: 10.0,
                weekly_percent: 20.0,
            }],
        };

        assert_eq!(tray_tooltip_state_kind(&state), "merged");
    }

    #[test]
    fn debug_kind_reports_per_provider_error_state() {
        let state = ProviderTooltipState::Error {
            error_msg: "boom".to_string(),
        };

        assert_eq!(provider_tooltip_state_kind(&state), "error");
    }

    #[test]
    fn debug_single_tooltip_text_reports_loading_copy() {
        let tooltip = debug_single_tooltip_text(&TrayTooltipState::Loading, Language::English);
        assert_eq!(tooltip, "CodexBar - Loading...");
    }

    #[test]
    fn debug_single_tooltip_text_reports_error_copy() {
        let tooltip = debug_single_tooltip_text(
            &TrayTooltipState::Error {
                provider_name: "Codex".to_string(),
                error_msg: "Auth failed".to_string(),
            },
            Language::English,
        );
        assert_eq!(tooltip, "Codex: Auth failed");
    }

    #[test]
    fn debug_provider_tooltip_text_reports_provider_loading_copy() {
        let tooltip = debug_provider_tooltip_text(
            ProviderId::Claude,
            &ProviderTooltipState::Loading,
            Language::English,
        );
        assert_eq!(tooltip, "Claude - Loading...");
    }

    #[test]
    fn debug_provider_tooltip_text_reports_provider_error_copy() {
        let tooltip = debug_provider_tooltip_text(
            ProviderId::Codex,
            &ProviderTooltipState::Error {
                error_msg: "Token expired".to_string(),
            },
            Language::English,
        );
        assert_eq!(tooltip, "Codex: Token expired");
    }
}
