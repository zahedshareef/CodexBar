//! System tray module for CodexBar
//!
//! Provides icon types and tray management for the Windows system tray

#![allow(unused_imports)]

pub mod icon;
pub mod render;

// Legacy egui-shell submodules live under rust/legacy/tray/.
// They remain compiled so the existing API surface keeps working.
#[path = "../../legacy/tray/blink.rs"]
pub mod blink;
#[path = "../../legacy/tray/icon_twist.rs"]
pub mod icon_twist;
#[path = "../../legacy/tray/manager.rs"]
pub mod manager;
#[path = "../../legacy/tray/menu_invalidation.rs"]
pub mod menu_invalidation;
#[path = "../../legacy/tray/weekly_indicator.rs"]
pub mod weekly_indicator;

pub use blink::{BlinkConfig, BlinkOutput, BlinkState, EyeBlinkSystem, MotionEffect};
pub use icon::LoadingPattern;
pub use icon_twist::{Decoration, DecorationKind, EyeShape, IconFeatures, IconTwist};
pub use manager::{
    IconOverlay, MultiTrayManager, ProviderUsage, SurpriseAnimation, TrayManager, TrayMenuAction,
    UnifiedTrayManager,
};
pub use menu_invalidation::{
    MENU_OPEN_REFRESH_DELAY, MenuDirtyState, MenuInvalidationTracker, MenuSection, StalenessChecker,
};
pub use render::{TRAY_ICON_SIZE, render_bar_icon_rgba};
pub use weekly_indicator::{
    UsageDisplayMode, WEEKLY_INDICATOR_HEIGHT, WeeklyIndicatorColors, WeeklyIndicatorConfig,
    WeeklyIndicatorDrawData, calculate_weekly_remaining,
};
