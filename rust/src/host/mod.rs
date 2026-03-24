//! Host module for process management and command execution

pub mod command_runner;

// Re-exports for future CLI integration
#[allow(unused_imports)]
pub use command_runner::{CommandError, CommandOptions, CommandResult, CommandRunner, RollingBuffer};
