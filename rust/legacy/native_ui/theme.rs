//! Theme: CodexBar Windows Utility Dark
//!
//! A restrained, premium dark theme for a serious Windows desktop utility.
//! Deep charcoal/slate tones with steel-blue accents. Designed for information
//! density with clear visual hierarchy — not a generic dark mode or macOS clone.

#![allow(dead_code)]

use egui::Color32;

/// CodexBar Theme — "Operator Dashboard" Dark
pub struct Theme;

impl Theme {
    // ═══════════════════════════════════════════════════════════════════
    // BACKGROUNDS — Deep charcoal/slate layers
    // ═══════════════════════════════════════════════════════════════════

    /// Window canvas — the deepest layer
    pub const BG_PRIMARY: Color32 = Color32::from_rgb(22, 22, 26);

    /// Secondary surface — cards, panels
    pub const BG_SECONDARY: Color32 = Color32::from_rgb(30, 30, 36);

    /// Tertiary — nested elements, subtle grouping
    pub const BG_TERTIARY: Color32 = Color32::from_rgb(40, 40, 48);

    /// Card/panel background
    pub const CARD_BG: Color32 = Color32::from_rgb(34, 34, 40);

    /// Card background on hover
    pub const CARD_BG_HOVER: Color32 = Color32::from_rgb(44, 44, 52);

    /// Elevated surface (modals, popovers, dropdowns)
    pub const SURFACE_ELEVATED: Color32 = Color32::from_rgb(38, 38, 44);

    /// Input field background — slightly recessed
    pub const INPUT_BG: Color32 = Color32::from_rgb(18, 18, 22);

    // ═══════════════════════════════════════════════════════════════════
    // ZONE BACKGROUNDS — purpose-specific surface colors
    // ═══════════════════════════════════════════════════════════════════

    /// Navigation bar / tab strip background
    pub const NAV_BG: Color32 = Color32::from_rgb(26, 26, 32);

    /// Footer/action bar background
    pub const FOOTER_BG: Color32 = Color32::from_rgb(26, 26, 32);

    /// Settings sidebar background
    pub const SIDEBAR_BG: Color32 = Color32::from_rgb(26, 26, 32);

    /// Active/selected nav item fill
    pub const NAV_ACTIVE_BG: Color32 = Color32::from_rgb(34, 34, 42);

    // ═══════════════════════════════════════════════════════════════════
    // ACCENT COLORS — Steel blue / cyan system
    // ═══════════════════════════════════════════════════════════════════

    /// Primary accent — steel blue
    pub const ACCENT_PRIMARY: Color32 = Color32::from_rgb(86, 156, 214);

    /// Primary accent on hover — lighter steel
    pub const ACCENT_HOVER: Color32 = Color32::from_rgb(110, 175, 230);

    /// Primary accent muted — for subtle emphasis
    pub const ACCENT_MUTED: Color32 = Color32::from_rgb(60, 120, 180);

    /// Secondary accent — cooler slate-blue
    pub const ACCENT_SECONDARY: Color32 = Color32::from_rgb(100, 140, 200);

    /// Tertiary accent — subtle cyan tint
    pub const ACCENT_TERTIARY: Color32 = Color32::from_rgb(80, 170, 200);

    // ═══════════════════════════════════════════════════════════════════
    // TAB COLORS
    // ═══════════════════════════════════════════════════════════════════

    /// Tab container background
    pub const TAB_CONTAINER: Color32 = Color32::from_rgb(26, 26, 32);

    /// Tab inactive fill
    pub const TAB_INACTIVE: Color32 = Color32::from_rgb(34, 34, 40);

    /// Tab active fill
    pub const TAB_ACTIVE: Color32 = Color32::from_rgb(86, 156, 214);

    /// Tab text inactive
    pub const TAB_TEXT_INACTIVE: Color32 = Color32::from_rgb(130, 130, 142);

    /// Tab text active
    pub const TAB_TEXT_ACTIVE: Color32 = Color32::WHITE;

    // ═══════════════════════════════════════════════════════════════════
    // TEXT COLORS — High-readability slate grays
    // ═══════════════════════════════════════════════════════════════════

    /// Primary text — bright white with slight warmth
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(235, 235, 240);

    /// Secondary text — readable but de-emphasized
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(152, 152, 166);

    /// Muted text — timestamps, metadata
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(96, 96, 110);

    /// Dimmed text — placeholder, disabled
    pub const TEXT_DIM: Color32 = Color32::from_rgb(66, 66, 78);

    /// Section header text — uppercase labels (brightened for mac parity)
    pub const TEXT_SECTION: Color32 = Color32::from_rgb(142, 142, 158);

    // ═══════════════════════════════════════════════════════════════════
    // BORDERS & SEPARATORS — Subtle depth cues
    // ═══════════════════════════════════════════════════════════════════

    /// Separator line
    pub const SEPARATOR: Color32 = Color32::from_rgb(48, 48, 56);

    /// Card/panel border
    pub const CARD_BORDER: Color32 = Color32::from_rgb(48, 48, 56);

    /// Focused/accent border
    pub const CARD_BORDER_ACCENT: Color32 = Color32::from_rgb(86, 156, 214);

    /// Subtle border for inputs
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(56, 56, 66);

    // ═══════════════════════════════════════════════════════════════════
    // USAGE/STATUS COLORS — Muted but distinct
    // ═══════════════════════════════════════════════════════════════════

    /// Green — operational / healthy (0-50% usage)
    pub const GREEN: Color32 = Color32::from_rgb(74, 198, 104);
    pub const USAGE_GREEN: Color32 = Self::GREEN;

    /// Blue — accent blue
    pub const BLUE: Color32 = Color32::from_rgb(86, 156, 214);

    /// Yellow — caution (50-75% usage)
    pub const YELLOW: Color32 = Color32::from_rgb(230, 196, 60);

    /// Orange — warning (75-90% usage)
    pub const ORANGE: Color32 = Color32::from_rgb(224, 142, 50);
    pub const USAGE_ORANGE: Color32 = Self::ORANGE;

    /// Red — critical (90-100% usage)
    pub const RED: Color32 = Color32::from_rgb(224, 80, 72);

    /// Cyan
    pub const CYAN: Color32 = Color32::from_rgb(80, 190, 220);

    /// Progress bar track
    pub fn progress_track() -> Color32 {
        Color32::from_rgba_unmultiplied(96, 96, 110, 50)
    }

    // ═══════════════════════════════════════════════════════════════════
    // BADGES — Status indicators
    // ═══════════════════════════════════════════════════════════════════

    pub const BADGE_SUCCESS: Color32 = Color32::from_rgb(74, 198, 104);
    pub const BADGE_WARNING: Color32 = Color32::from_rgb(224, 142, 50);
    pub const BADGE_ERROR: Color32 = Color32::from_rgb(224, 80, 72);
    pub const BADGE_INFO: Color32 = Color32::from_rgb(86, 156, 214);

    // ═══════════════════════════════════════════════════════════════════
    // SPECIAL EFFECTS
    // ═══════════════════════════════════════════════════════════════════

    pub fn shadow() -> Color32 {
        Color32::from_rgba_unmultiplied(0, 0, 0, 70)
    }

    pub fn selection_overlay() -> Color32 {
        Color32::from_rgba_unmultiplied(86, 156, 214, 28)
    }

    pub fn hover_overlay() -> Color32 {
        Color32::from_rgba_unmultiplied(255, 255, 255, 6)
    }

    pub fn glow_overlay() -> Color32 {
        Color32::from_rgba_unmultiplied(86, 156, 214, 20)
    }

    pub fn progress_glow() -> Color32 {
        Color32::from_rgba_unmultiplied(86, 156, 214, 30)
    }

    pub fn gradient_start() -> Color32 {
        Color32::from_rgba_unmultiplied(86, 156, 214, 8)
    }

    pub fn gradient_end() -> Color32 {
        Color32::from_rgba_unmultiplied(80, 170, 200, 6)
    }

    // ═══════════════════════════════════════════════════════════════════
    // METHODS — Usage-based coloring
    // ═══════════════════════════════════════════════════════════════════

    pub fn usage_color(percent: f64) -> Color32 {
        if percent <= 50.0 {
            Self::GREEN
        } else if percent <= 75.0 {
            Self::YELLOW
        } else if percent <= 90.0 {
            Self::ORANGE
        } else {
            Self::RED
        }
    }

    pub fn usage_track_color(_percent: f64) -> Color32 {
        Self::progress_track()
    }

    pub fn usage_glow_color(percent: f64) -> Color32 {
        let base = Self::usage_color(percent);
        Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), 35)
    }

    pub fn menu_hover() -> Color32 {
        Color32::from_rgba_unmultiplied(255, 255, 255, 6)
    }

    pub fn button_gradient_top() -> Color32 {
        Color32::from_rgb(96, 170, 224)
    }

    pub fn button_gradient_bottom() -> Color32 {
        Color32::from_rgb(76, 146, 200)
    }
}

use crate::status::StatusLevel;

/// Get color for a provider status level
pub fn status_color(level: StatusLevel) -> Color32 {
    match level {
        StatusLevel::Operational => Theme::GREEN,
        StatusLevel::Degraded => Theme::YELLOW,
        StatusLevel::Partial => Theme::ORANGE,
        StatusLevel::Major => Theme::RED,
        StatusLevel::Unknown => Theme::TEXT_MUTED,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROVIDER ICONS — Clean geometric symbols
// ═══════════════════════════════════════════════════════════════════════════

/// Provider icons — distinctive symbols
pub fn provider_icon(name: &str) -> &'static str {
    match name.to_lowercase().as_str() {
        "codex" => "◆",
        "claude" => "◈",
        "cursor" => "▸",
        "gemini" => "✦",
        "copilot" => "⬡",
        "antigravity" => "◉",
        "factory" | "droid" => "◎",
        "zai" | "z.ai" => "Z",
        "kiro" => "K",
        "vertexai" | "vertex ai" => "△",
        "augment" => "A",
        "minimax" => "M",
        "opencode" => "○",
        "kimi" => "☽",
        "kimik2" | "kimi k2" => "☽",
        "amp" => "⚡",
        "synthetic" => "◇",
        "jetbrains" | "jetbrains ai" => "J",
        "alibaba" | "tongyi" => "阿",
        "nanogpt" => "N",
        _ => "●",
    }
}

/// Provider brand colors — matching original CodexBar
pub fn provider_color(name: &str) -> Color32 {
    match name.to_lowercase().as_str() {
        "claude" => Color32::from_rgb(204, 124, 94), // Warm terracotta
        "codex" => Color32::from_rgb(73, 163, 176),  // Teal
        "gemini" => Color32::from_rgb(171, 135, 234), // Purple
        "cursor" => Color32::from_rgb(0, 191, 165),  // Teal green
        "copilot" => Color32::from_rgb(168, 85, 247), // Vibrant purple
        "jetbrains" | "jetbrains ai" => Color32::from_rgb(255, 51, 153), // Hot pink
        "antigravity" => Color32::from_rgb(96, 186, 126), // Soft green
        "augment" => Color32::from_rgb(99, 102, 241), // Indigo
        "amp" => Color32::from_rgb(220, 38, 38),     // Red
        "factory" | "droid" => Color32::from_rgb(255, 107, 53), // Orange
        "kimi" => Color32::from_rgb(254, 96, 60),    // Coral
        "kimik2" | "kimi k2" => Color32::from_rgb(76, 0, 255), // Electric blue
        "kiro" => Color32::from_rgb(255, 153, 0),    // Amber
        "opencode" => Color32::from_rgb(59, 130, 246), // Blue
        "minimax" => Color32::from_rgb(254, 96, 60), // Coral
        "vertexai" | "vertex ai" => Color32::from_rgb(66, 133, 244), // Google blue
        "zai" | "z.ai" => Color32::from_rgb(232, 90, 106), // Rose
        "synthetic" => Color32::from_rgb(20, 20, 20), // Near black
        "alibaba" | "tongyi" => Color32::from_rgb(255, 106, 0), // Alibaba orange
        "nanogpt" => Color32::from_rgb(104, 127, 161), // Muted blue-grey
        _ => Theme::ACCENT_PRIMARY,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SPACING CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

pub struct Spacing;

impl Spacing {
    pub const XXS: f32 = 4.0;
    pub const XS: f32 = 6.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 20.0;
    pub const XXL: f32 = 28.0;
}

/// Rounding constants — tight, confident corners
pub struct Radius;

impl Radius {
    pub const XS: f32 = 3.0;
    pub const SM: f32 = 5.0;
    pub const MD: f32 = 8.0;
    pub const LG: f32 = 12.0;
    pub const XL: f32 = 16.0;
    pub const PILL: f32 = 100.0;
}

/// Font sizes — bolder hierarchy for small windows
pub struct FontSize;

impl FontSize {
    pub const XS: f32 = 11.0;
    pub const SM: f32 = 12.0;
    pub const BASE: f32 = 13.0;
    pub const MD: f32 = 14.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 18.0;
    pub const XXL: f32 = 22.0;

    /// Navigation/tab label size
    pub const NAV: f32 = 12.0;

    /// Small badge/indicator size
    pub const INDICATOR: f32 = 10.0;
}
