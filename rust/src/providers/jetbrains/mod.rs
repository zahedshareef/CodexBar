//! JetBrains AI provider implementation
//!
//! Fetches usage data from JetBrains IDE local configuration
//! JetBrains AI Assistant stores quota info in XML configuration files

#![allow(dead_code)]

use async_trait::async_trait;
use std::path::PathBuf;

use crate::core::{
    FetchContext, Provider, ProviderId, ProviderError, ProviderFetchResult,
    ProviderMetadata, RateWindow, SourceMode, UsageSnapshot,
};

/// JetBrains AI provider
pub struct JetBrainsProvider {
    metadata: ProviderMetadata,
}

impl JetBrainsProvider {
    pub fn new() -> Self {
        Self {
            metadata: ProviderMetadata {
                id: ProviderId::JetBrains,
                display_name: "JetBrains AI",
                session_label: "Credits",
                weekly_label: "Monthly",
                supports_opus: false,
                supports_credits: true,
                default_enabled: false,
                is_primary: false,
                dashboard_url: Some("https://www.jetbrains.com/ai/"),
                status_page_url: None,
            },
        }
    }

    /// Get JetBrains config directory
    fn get_jetbrains_config_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        // JetBrains stores config in AppData/Roaming on Windows
        if let Some(config_dir) = dirs::config_dir() {
            // JetBrains products: IntelliJ IDEA, PyCharm, WebStorm, etc.
            let products = [
                "JetBrains/IntelliJIdea*",
                "JetBrains/PyCharm*",
                "JetBrains/WebStorm*",
                "JetBrains/GoLand*",
                "JetBrains/CLion*",
                "JetBrains/Rider*",
                "JetBrains/PhpStorm*",
                "JetBrains/RubyMine*",
                "JetBrains/DataGrip*",
                "JetBrains/DataSpell*",
                "Google/AndroidStudio*",
            ];

            for product in products {
                let base = product.split('*').next().unwrap_or(product);
                let product_dir = config_dir.join(base);
                if product_dir.exists() {
                    // Find versioned subdirectories
                    if let Ok(entries) = std::fs::read_dir(&product_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_dir() {
                                dirs.push(path);
                            }
                        }
                    }
                }
            }
        }

        dirs
    }

    /// Find AI Assistant config file
    fn find_ai_config_file() -> Option<PathBuf> {
        let config_dirs = Self::get_jetbrains_config_dirs();

        for config_dir in config_dirs {
            // AI Assistant config is typically in options/ai.assistant.xml or similar
            let possible_paths = [
                config_dir.join("options").join("ai-assistant.xml"),
                config_dir.join("options").join("aiAssistant.xml"),
                config_dir.join("options").join("ai.xml"),
                config_dir.join("options").join("AIAssistantQuotaManager2.xml"),
            ];

            for path in possible_paths {
                if path.exists() {
                    return Some(path);
                }
            }
        }

        None
    }

    /// Read usage from local XML config
    async fn read_local_config(&self) -> Result<UsageSnapshot, ProviderError> {
        let config_file = Self::find_ai_config_file().ok_or_else(|| {
            ProviderError::NotInstalled(
                "JetBrains AI Assistant not found. Install from JetBrains IDE Marketplace.".to_string()
            )
        })?;

        let content = tokio::fs::read_to_string(&config_file).await
            .map_err(|e| ProviderError::Other(format!("Failed to read config: {}", e)))?;

        self.parse_xml_config(&content)
    }

    /// Parse JetBrains AI XML config
    fn parse_xml_config(&self, content: &str) -> Result<UsageSnapshot, ProviderError> {
        // Parse XML to extract quota info
        // JetBrains AI stores quota as:
        // <component name="AiAssistant">
        //   <option name="usedCredits" value="123" />
        //   <option name="creditLimit" value="1000" />
        // </component>

        let mut used_credits = 0.0;
        let mut credit_limit = 1000.0; // Default monthly limit

        // Simple XML parsing (not using full XML parser to avoid dependency)
        for line in content.lines() {
            let line = line.trim();

            if line.contains("usedCredits") || line.contains("used_credits") || line.contains("creditsUsed") {
                if let Some(value) = Self::extract_xml_value(line) {
                    used_credits = value;
                }
            }

            if line.contains("creditLimit") || line.contains("credit_limit") || line.contains("creditsLimit") || line.contains("monthlyLimit") {
                if let Some(value) = Self::extract_xml_value(line) {
                    credit_limit = value;
                }
            }
        }

        let used_percent = if credit_limit > 0.0 {
            (used_credits / credit_limit) * 100.0
        } else {
            0.0
        };

        let usage = UsageSnapshot::new(RateWindow::new(used_percent))
            .with_login_method("JetBrains AI");

        Ok(usage)
    }

    /// Extract numeric value from XML attribute
    fn extract_xml_value(line: &str) -> Option<f64> {
        // Look for value="123" pattern
        if let Some(start) = line.find("value=\"") {
            let rest = &line[start + 7..];
            if let Some(end) = rest.find('"') {
                let value_str = &rest[..end];
                return value_str.parse().ok();
            }
        }
        None
    }

    /// Check if JetBrains AI is installed
    fn is_installed() -> bool {
        Self::find_ai_config_file().is_some()
    }
}

impl Default for JetBrainsProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for JetBrainsProvider {
    fn id(&self) -> ProviderId {
        ProviderId::JetBrains
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<ProviderFetchResult, ProviderError> {
        tracing::debug!("Fetching JetBrains AI usage");

        match ctx.source_mode {
            SourceMode::Auto | SourceMode::Cli => {
                let usage = self.read_local_config().await?;
                Ok(ProviderFetchResult::new(usage, "local"))
            }
            SourceMode::Web | SourceMode::OAuth => {
                // JetBrains AI doesn't have web API access
                Err(ProviderError::UnsupportedSource(ctx.source_mode))
            }
        }
    }

    fn available_sources(&self) -> Vec<SourceMode> {
        vec![SourceMode::Auto, SourceMode::Cli]
    }

    fn supports_web(&self) -> bool {
        false
    }

    fn supports_cli(&self) -> bool {
        true
    }
}
