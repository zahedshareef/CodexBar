//! Amp provider implementation
//!
//! Amp is Sourcegraph's AI coding assistant
//! Fetches usage data from Amp's local config or API

use async_trait::async_trait;
use std::path::PathBuf;

use crate::core::{
    FetchContext, Provider, ProviderId, ProviderError, ProviderFetchResult,
    ProviderMetadata, RateWindow, SourceMode, UsageSnapshot,
};

/// Amp provider (Sourcegraph)
pub struct AmpProvider {
    metadata: ProviderMetadata,
}

impl AmpProvider {
    pub fn new() -> Self {
        Self {
            metadata: ProviderMetadata {
                id: ProviderId::Amp,
                display_name: "Amp",
                session_label: "Usage",
                weekly_label: "Monthly",
                supports_opus: false,
                supports_credits: true,
                default_enabled: false,
                is_primary: false,
                dashboard_url: Some("https://sourcegraph.com/cody/manage"),
                status_page_url: Some("https://sourcegraphstatus.com"),
            },
        }
    }

    /// Get Amp config directory
    fn get_amp_config_path() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            dirs::config_dir().map(|p| p.join("amp"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            dirs::home_dir().map(|p| p.join(".amp"))
        }
    }

    /// Get Sourcegraph/Cody config directory (Amp might use this)
    fn get_cody_config_path() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            dirs::config_dir().map(|p| p.join("sourcegraph-cody"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            dirs::home_dir().map(|p| p.join(".sourcegraph"))
        }
    }

    /// Read Amp/Sourcegraph access token
    async fn read_access_token(&self, ctx: &FetchContext) -> Result<String, ProviderError> {
        // Check ctx.api_key first (from settings)
        if let Some(ref api_key) = ctx.api_key {
            if !api_key.is_empty() {
                return Ok(api_key.clone());
            }
        }

        // Check environment variables as fallback
        if let Ok(token) = std::env::var("SRC_ACCESS_TOKEN") {
            return Ok(token);
        }
        if let Ok(token) = std::env::var("AMP_ACCESS_TOKEN") {
            return Ok(token);
        }

        // Check Amp config
        if let Some(amp_path) = Self::get_amp_config_path() {
            let config_file = amp_path.join("config.json");
            if config_file.exists() {
                if let Ok(content) = tokio::fs::read_to_string(&config_file).await {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(token) = json.get("accessToken").and_then(|v| v.as_str()) {
                            return Ok(token.to_string());
                        }
                    }
                }
            }
        }

        // Check Cody/Sourcegraph config
        if let Some(cody_path) = Self::get_cody_config_path() {
            let config_file = cody_path.join("config.json");
            if config_file.exists() {
                if let Ok(content) = tokio::fs::read_to_string(&config_file).await {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(token) = json.get("accessToken").and_then(|v| v.as_str()) {
                            return Ok(token.to_string());
                        }
                    }
                }
            }
        }

        Err(ProviderError::AuthRequired)
    }

    /// Fetch usage via Sourcegraph API
    async fn fetch_via_web(&self, ctx: &FetchContext) -> Result<UsageSnapshot, ProviderError> {
        let token = self.read_access_token(ctx).await?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ProviderError::Other(e.to_string()))?;

        // Sourcegraph Cody usage API
        let resp = client
            .get("https://sourcegraph.com/.api/cody/current-user/usage")
            .header("Authorization", format!("token {}", token))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ProviderError::AuthRequired);
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        self.parse_usage_response(&json)
    }

    fn parse_usage_response(&self, json: &serde_json::Value) -> Result<UsageSnapshot, ProviderError> {
        // Parse Sourcegraph/Amp usage response
        let used = json.get("completionsUsed")
            .or_else(|| json.get("used"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let limit = json.get("completionsLimit")
            .or_else(|| json.get("limit"))
            .and_then(|v| v.as_f64())
            .unwrap_or(500.0);

        let used_percent = if limit > 0.0 {
            (used / limit) * 100.0
        } else {
            0.0
        };

        let plan = json.get("plan")
            .or_else(|| json.get("tier"))
            .and_then(|v| v.as_str())
            .unwrap_or("Pro");

        let reset_time = json.get("resetAt")
            .or_else(|| json.get("periodEnd"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let primary_window = RateWindow::with_details(used_percent, None, None, reset_time);
        let usage = UsageSnapshot::new(primary_window)
            .with_login_method(plan);

        Ok(usage)
    }

    /// Probe for Amp installation
    async fn probe_cli(&self, ctx: &FetchContext) -> Result<UsageSnapshot, ProviderError> {
        // Check ctx.api_key first
        let has_api_key = ctx.api_key.as_ref().map(|k| !k.is_empty()).unwrap_or(false);

        let has_env = std::env::var("SRC_ACCESS_TOKEN").is_ok()
            || std::env::var("AMP_ACCESS_TOKEN").is_ok();

        let has_amp_config = Self::get_amp_config_path()
            .map(|p| p.join("config.json").exists())
            .unwrap_or(false);

        let has_cody_config = Self::get_cody_config_path()
            .map(|p| p.join("config.json").exists())
            .unwrap_or(false);

        if has_api_key || has_env || has_amp_config || has_cody_config {
            let usage = UsageSnapshot::new(RateWindow::new(0.0))
                .with_login_method("Amp (configured)");
            Ok(usage)
        } else {
            Err(ProviderError::NotInstalled(
                "Amp not configured. Set SRC_ACCESS_TOKEN environment variable or configure Amp.".to_string()
            ))
        }
    }
}

impl Default for AmpProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for AmpProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Amp
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<ProviderFetchResult, ProviderError> {
        tracing::debug!("Fetching Amp usage");

        match ctx.source_mode {
            SourceMode::Auto => {
                if let Ok(usage) = self.fetch_via_web(ctx).await {
                    return Ok(ProviderFetchResult::new(usage, "web"));
                }
                let usage = self.probe_cli(ctx).await?;
                Ok(ProviderFetchResult::new(usage, "cli"))
            }
            SourceMode::Web => {
                let usage = self.fetch_via_web(ctx).await?;
                Ok(ProviderFetchResult::new(usage, "web"))
            }
            SourceMode::Cli => {
                let usage = self.probe_cli(ctx).await?;
                Ok(ProviderFetchResult::new(usage, "cli"))
            }
            SourceMode::OAuth => {
                Err(ProviderError::UnsupportedSource(SourceMode::OAuth))
            }
        }
    }

    fn available_sources(&self) -> Vec<SourceMode> {
        vec![SourceMode::Auto, SourceMode::Web, SourceMode::Cli]
    }

    fn supports_web(&self) -> bool {
        true
    }

    fn supports_cli(&self) -> bool {
        true
    }
}
