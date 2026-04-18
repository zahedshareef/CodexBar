//! Shared library surface for CodexBar.
//!
//! This keeps the current Rust implementation usable from the existing CLI/bin
//! while giving the rewrite a stable crate dependency for future shells.

pub mod browser;
pub mod cli;
pub mod core;
pub mod cost_scanner;
pub mod host;
pub mod locale;
pub mod logging;
pub mod login;
pub mod notifications;
pub mod providers;
pub mod settings;
pub mod shortcuts;
pub mod sound;

// Legacy egui shell modules live under rust/legacy/. They are gated behind
// the "legacy" feature so the default build (Tauri shell) does not compile them.
#[cfg(feature = "legacy")]
#[path = "../legacy/native_ui/mod.rs"]
pub mod native_ui;
#[cfg(feature = "legacy")]
#[path = "../legacy/single_instance.rs"]
pub mod single_instance;
pub mod status;
pub mod tray;
pub mod updater;
pub mod wsl;
