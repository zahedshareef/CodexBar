//! Config command implementation
//!
//! Utilities for validating and inspecting configuration.

use clap::{Parser, Subcommand};

use crate::core::TokenAccountStore;
use crate::settings::{ManualCookies, Settings};

/// Arguments for the config command
#[derive(Parser, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Validate configuration files
    Validate,
    /// Dump configuration to stdout
    Dump {
        /// Output format: json or toml
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// Show configuration file paths
    Path,
}

/// Run the config command
pub async fn run(args: ConfigArgs) -> anyhow::Result<()> {
    match args.command {
        ConfigCommand::Validate => validate_config().await,
        ConfigCommand::Dump { format } => dump_config(&format).await,
        ConfigCommand::Path => show_paths().await,
    }
}

/// Validate configuration files
async fn validate_config() -> anyhow::Result<()> {
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Check settings
    print!("Checking settings.json... ");
    if let Some(path) = Settings::settings_path() {
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str::<Settings>(&content) {
                        Ok(_) => println!("OK"),
                        Err(e) => {
                            println!("INVALID");
                            errors.push(format!("settings.json: {}", e));
                        }
                    }
                }
                Err(e) => {
                    println!("ERROR");
                    errors.push(format!("settings.json: Could not read file: {}", e));
                }
            }
        } else {
            println!("NOT FOUND (using defaults)");
            warnings.push("settings.json: File does not exist, using defaults".to_string());
        }
    } else {
        println!("ERROR");
        errors.push("settings.json: Could not determine config path".to_string());
    }

    // Check manual cookies
    print!("Checking manual_cookies.json... ");
    if let Some(path) = ManualCookies::cookies_path() {
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str::<ManualCookies>(&content) {
                        Ok(_) => println!("OK"),
                        Err(e) => {
                            println!("INVALID");
                            errors.push(format!("manual_cookies.json: {}", e));
                        }
                    }
                }
                Err(e) => {
                    println!("ERROR");
                    errors.push(format!("manual_cookies.json: Could not read file: {}", e));
                }
            }
        } else {
            println!("NOT FOUND (none configured)");
        }
    } else {
        println!("SKIP");
    }

    // Check token accounts
    print!("Checking token-accounts.json... ");
    let store = TokenAccountStore::new();
    let path = TokenAccountStore::default_path();
    if path.exists() {
        match store.load() {
            Ok(_) => println!("OK"),
            Err(e) => {
                println!("INVALID");
                errors.push(format!("token-accounts.json: {}", e));
            }
        }
    } else {
        println!("NOT FOUND (none configured)");
    }

    // Print summary
    println!();
    if errors.is_empty() && warnings.is_empty() {
        println!("Configuration is valid.");
    } else {
        if !warnings.is_empty() {
            println!("Warnings:");
            for w in &warnings {
                println!("  - {}", w);
            }
        }
        if !errors.is_empty() {
            println!("Errors:");
            for e in &errors {
                println!("  - {}", e);
            }
            anyhow::bail!("Configuration validation failed with {} error(s).", errors.len());
        }
    }

    Ok(())
}

/// Dump configuration to stdout
async fn dump_config(format: &str) -> anyhow::Result<()> {
    let settings = Settings::load();

    match format.to_lowercase().as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&settings)?;
            println!("{}", json);
        }
        "toml" => {
            let toml = toml::to_string_pretty(&settings)?;
            println!("{}", toml);
        }
        _ => {
            anyhow::bail!("Unknown format '{}'. Supported formats: json, toml", format);
        }
    }

    Ok(())
}

/// Show configuration file paths
async fn show_paths() -> anyhow::Result<()> {
    println!("Configuration paths:");

    if let Some(path) = Settings::settings_path() {
        let exists = if path.exists() { "" } else { " (not found)" };
        println!("  Settings:       {}{}", path.display(), exists);
    } else {
        println!("  Settings:       (could not determine path)");
    }

    if let Some(path) = ManualCookies::cookies_path() {
        let exists = if path.exists() { "" } else { " (not found)" };
        println!("  Manual cookies: {}{}", path.display(), exists);
    } else {
        println!("  Manual cookies: (could not determine path)");
    }

    let token_path = TokenAccountStore::default_path();
    let exists = if token_path.exists() { "" } else { " (not found)" };
    println!("  Token accounts: {}{}", token_path.display(), exists);

    // Show config directory
    if let Some(config_dir) = dirs::config_dir() {
        let codexbar_dir = config_dir.join("CodexBar");
        println!();
        println!("Config directory: {}", codexbar_dir.display());
    }

    Ok(())
}
