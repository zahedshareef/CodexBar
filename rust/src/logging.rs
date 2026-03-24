//! Logging configuration using tracing

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize the logging system
pub fn init(verbose: bool, json: bool) -> anyhow::Result<()> {
    let filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    if json {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json().with_writer(std::io::stderr))
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_writer(std::io::stderr))
            .init();
    }

    Ok(())
}
