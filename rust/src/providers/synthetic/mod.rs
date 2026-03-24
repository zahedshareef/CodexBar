//! Synthetic provider implementation
//!
//! Synthetic is an AI coding assistant
//! Fetches usage data from Synthetic's local config or API

use async_trait::async_trait;
use std::path::PathBuf;

use crate::core::{
    FetchContext, Provider, ProviderId, ProviderError, ProviderFetchResult,
    ProviderMetadata, RateWindow, SourceMode, UsageSnapshot,
};

/// Synthetic provider
pub struct SyntheticProvider {
    metadata: ProviderMetadata,
}

impl SyntheticProvider {
    pub fn new() -> Self {
        Self {
            metadata: ProviderMetadata {
                id: ProviderId::Synthetic,
                display_name: "Synthetic",
                session_label: "Usage",
                weekly_label: "Monthly",
                supports_opus: false,
                supports_credits: true,
                default_enabled: false,
                is_primary: false,
                dashboard_url: Some("https://synthetic.computer/account"),
                status_page_url: None,
            },
        }
    }

    /// Get Synthetic config directory
    fn get_synthetic_config_path() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            dirs::config_dir().map(|p| p.join("synthetic"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            dirs::home_dir().map(|p| p.join(".synthetic"))
        }
    }

    /// Read Synthetic access token
    async fn read_access_token(&self, ctx: &FetchContext) -> Result<String, ProviderError> {
        // Check ctx.api_key first (from settings)
        if let Some(ref api_key) = ctx.api_key {
            if !api_key.is_empty() {
                return Ok(api_key.clone());
            }
        }

        // Check environment variables as fallback
        if let Ok(token) = std::env::var("SYNTHETIC_API_KEY") {
            return Ok(token);
        }
        if let Ok(token) = std::env::var("SYNTHETIC_ACCESS_TOKEN") {
            return Ok(token);
        }

        // Check config file
        if let Some(config_path) = Self::get_synthetic_config_path() {
            let config_file = config_path.join("config.json");
            if config_file.exists() {
                if let Ok(content) = tokio::fs::read_to_string(&config_file).await {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(token) = json.get("apiKey")
                            .or_else(|| json.get("accessToken"))
                            .and_then(|v| v.as_str())
                        {
                            return Ok(token.to_string());
                        }
                    }
                }
            }

            // Also check credentials.json
            let creds_file = config_path.join("credentials.json");
            if creds_file.exists() {
                if let Ok(content) = tokio::fs::read_to_string(&creds_file).await {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(token) = json.get("apiKey")
                            .or_else(|| json.get("token"))
                            .and_then(|v| v.as_str())
                        {
                            return Ok(token.to_string());
                        }
                    }
                }
            }
        }

        Err(ProviderError::AuthRequired)
    }

    /// Fetch usage via Synthetic API
    async fn fetch_via_web(&self, ctx: &FetchContext) -> Result<UsageSnapshot, ProviderError> {
        let token = self.read_access_token(ctx).await?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ProviderError::Other(e.to_string()))?;

        // Synthetic usage API (hypothetical - adjust based on actual API)
        let resp = client
            .get("https://api.synthetic.computer/v1/usage")
            .header("Authorization", format!("Bearer {}", token))
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
        // Parse Synthetic usage response
        let used = json.get("usage")
            .or_else(|| json.get("used"))
            .or_else(|| json.get("tokensUsed"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let limit = json.get("limit")
            .or_else(|| json.get("quota"))
            .or_else(|| json.get("tokensLimit"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1000000.0);

        let used_percent = if limit > 0.0 {
            (used / limit) * 100.0
        } else {
            0.0
        };

        let plan = json.get("plan")
            .or_else(|| json.get("tier"))
            .or_else(|| json.get("subscription"))
            .and_then(|v| v.as_str())
            .unwrap_or("Synthetic");

        let reset_time = json.get("resetAt")
            .or_else(|| json.get("periodEnd"))
            .or_else(|| json.get("resetsAt"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let primary_window = RateWindow::with_details(used_percent, None, None, reset_time);
        let usage = UsageSnapshot::new(primary_window)
            .with_login_method(plan);

        Ok(usage)
    }

    /// Probe for Synthetic installation
    async fn probe_cli(&self, ctx: &FetchContext) -> Result<UsageSnapshot, ProviderError> {
        // Check ctx.api_key first
        let has_api_key = ctx.api_key.as_ref().map(|k| !k.is_empty()).unwrap_or(false);

        let has_env = std::env::var("SYNTHETIC_API_KEY").is_ok()
            || std::env::var("SYNTHETIC_ACCESS_TOKEN").is_ok();

        let has_config = Self::get_synthetic_config_path()
            .map(|p| p.join("config.json").exists() || p.join("credentials.json").exists())
            .unwrap_or(false);

        if has_api_key || has_env || has_config {
            let usage = UsageSnapshot::new(RateWindow::new(0.0))
                .with_login_method("Synthetic (configured)");
            Ok(usage)
        } else {
            Err(ProviderError::NotInstalled(
                "Synthetic not configured. Set SYNTHETIC_API_KEY environment variable or configure Synthetic.".to_string()
            ))
        }
    }
}

impl Default for SyntheticProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for SyntheticProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Synthetic
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<ProviderFetchResult, ProviderError> {
        tracing::debug!("Fetching Synthetic usage");

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
