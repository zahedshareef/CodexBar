//! Native egui-based UI for CodexBar
//!
//! Provides a native Windows popup window with macOS-style design

mod app;
mod charts;
mod preferences;
mod provider_icons;
#[cfg(debug_assertions)]
pub(crate) mod test_server;
mod theme;

pub use app::run;
