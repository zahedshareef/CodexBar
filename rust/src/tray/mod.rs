//! System tray module for CodexBar
//!
//! Provides icon types and tray management for the Windows system tray

#![allow(unused_imports)]

pub mod blink;
pub mod icon;
pub mod icon_twist;
pub mod manager;
pub mod menu_invalidation;
pub mod render;
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
