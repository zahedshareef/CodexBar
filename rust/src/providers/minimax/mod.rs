//! MiniMax provider implementation
//!
//! Fetches usage data from MiniMax AI API
//! MiniMax stores API keys locally or in environment

mod local_storage;

// Re-exports for local storage import
#[allow(unused_imports)]
pub use local_storage::{MiniMaxLocalStorageImporter, MiniMaxSession, ImportError};

use async_trait::async_trait;
use std::path::PathBuf;

use crate::core::{
    FetchContext, Provider, ProviderId, ProviderError, ProviderFetchResult,
    ProviderMetadata, RateWindow, SourceMode, UsageSnapshot,
};

/// MiniMax API region
#[derive(Debug, Clone, Copy, PartialEq)]
enum MiniMaxRegion {
    Global,
    ChinaMainland,
}

/// MiniMax provider
pub struct MiniMaxProvider {
    metadata: ProviderMetadata,
}

impl MiniMaxProvider {
    pub fn new() -> Self {
        Self {
            metadata: ProviderMetadata {
                id: ProviderId::MiniMax,
                display_name: "MiniMax",
                session_label: "Usage",
                weekly_label: "Monthly",
                supports_opus: false,
                supports_credits: true,
                default_enabled: false,
                is_primary: false,
                dashboard_url: Some("https://platform.minimaxi.com/user-center"),
                status_page_url: None,
            },
        }
    }

    /// Get MiniMax config directory
    fn get_minimax_config_path() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            dirs::config_dir().map(|p| p.join("minimax"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            dirs::home_dir().map(|p| p.join(".minimax"))
        }
    }

    /// Read MiniMax API key
    async fn read_api_key(&self) -> Result<(String, String), ProviderError> {
        // Check environment variables first
        if let (Ok(group_id), Ok(api_key)) = (
            std::env::var("MINIMAX_GROUP_ID"),
            std::env::var("MINIMAX_API_KEY")
        ) {
            return Ok((group_id, api_key));
        }

        // Check config file
        let config_path = Self::get_minimax_config_path()
            .ok_or_else(|| ProviderError::NotInstalled("MiniMax config not found".to_string()))?;

        let config_file = config_path.join("config.json");
        if config_file.exists() {
            let content = tokio::fs::read_to_string(&config_file).await
                .map_err(|e| ProviderError::Other(e.to_string()))?;

            let json: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| ProviderError::Parse(e.to_string()))?;

            let group_id = json.get("group_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let api_key = json.get("api_key")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            if let (Some(gid), Some(key)) = (group_id, api_key) {
                return Ok((gid, key));
            }
        }

        Err(ProviderError::AuthRequired)
    }

    /// API base URLs for different regions
    fn api_base_url(region: MiniMaxRegion) -> &'static str {
        match region {
            MiniMaxRegion::Global => "https://api.minimax.io",
            MiniMaxRegion::ChinaMainland => "https://api.minimaxi.com",
        }
    }

    /// Fetch usage via MiniMax API with region fallback
    async fn fetch_via_web(&self) -> Result<UsageSnapshot, ProviderError> {
        let (group_id, api_key) = self.read_api_key().await?;

        // Try global endpoint first, fall back to China mainland on 401/403
        match self.fetch_from_region(&group_id, &api_key, MiniMaxRegion::Global).await {
            Ok(usage) => Ok(usage),
            Err(ProviderError::AuthRequired) => {
                // Retry with China mainland endpoint
                self.fetch_from_region(&group_id, &api_key, MiniMaxRegion::ChinaMainland).await
            }
            Err(e) => Err(e),
        }
    }

    /// Fetch from a specific region endpoint
    async fn fetch_from_region(
        &self,
        group_id: &str,
        api_key: &str,
        region: MiniMaxRegion,
    ) -> Result<UsageSnapshot, ProviderError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ProviderError::Other(e.to_string()))?;

        let base_url = Self::api_base_url(region);
        let resp = client
            .get(format!(
                "{}/v1/billing/usage?group_id={}",
                base_url, group_id
            ))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("MM-API-Source", "CodexBar")
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED
            || resp.status() == reqwest::StatusCode::FORBIDDEN
        {
            return Err(ProviderError::AuthRequired);
        }

        if !resp.status().is_success() {
            return Err(ProviderError::Other(format!(
                "MiniMax API returned status {}",
                resp.status()
            )));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        self.parse_usage_response(&json)
    }

    fn parse_usage_response(&self, json: &serde_json::Value) -> Result<UsageSnapshot, ProviderError> {
        // Parse MiniMax billing response
        let base_resp = json.get("base_resp");
        if let Some(base) = base_resp {
            let status_code = base.get("status_code").and_then(|v| v.as_i64()).unwrap_or(-1);
            if status_code != 0 {
                return Err(ProviderError::Parse(
                    base.get("status_msg")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error")
                        .to_string()
                ));
            }
        }

        let used_credits = json.get("used_amount")
            .or_else(|| json.get("total_amount"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let credit_limit = json.get("total_quota")
            .or_else(|| json.get("quota"))
            .and_then(|v| v.as_f64())
            .unwrap_or(100.0);

        let used_percent = if credit_limit > 0.0 {
            (used_credits / credit_limit) * 100.0
        } else {
            0.0
        };

        let plan = json.get("plan_type")
            .or_else(|| json.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("MiniMax");

        let usage = UsageSnapshot::new(RateWindow::new(used_percent))
            .with_login_method(plan);

        Ok(usage)
    }

    /// Probe for MiniMax installation (credentials check)
    async fn probe_cli(&self) -> Result<UsageSnapshot, ProviderError> {
        // Check if API key is configured
        let has_env_vars = std::env::var("MINIMAX_API_KEY").is_ok();
        let has_config = Self::get_minimax_config_path()
            .map(|p| p.join("config.json").exists())
            .unwrap_or(false);

        if has_env_vars || has_config {
            let usage = UsageSnapshot::new(RateWindow::new(0.0))
                .with_login_method("MiniMax (configured)");
            Ok(usage)
        } else {
            Err(ProviderError::NotInstalled(
                "MiniMax API not configured. Set MINIMAX_API_KEY and MINIMAX_GROUP_ID environment variables".to_string()
            ))
        }
    }
}

impl Default for MiniMaxProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for MiniMaxProvider {
    fn id(&self) -> ProviderId {
        ProviderId::MiniMax
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<ProviderFetchResult, ProviderError> {
        tracing::debug!("Fetching MiniMax usage");

        match ctx.source_mode {
            SourceMode::Auto => {
                if let Ok(usage) = self.fetch_via_web().await {
                    return Ok(ProviderFetchResult::new(usage, "web"));
                }
                let usage = self.probe_cli().await?;
                Ok(ProviderFetchResult::new(usage, "cli"))
            }
            SourceMode::Web => {
                let usage = self.fetch_via_web().await?;
                Ok(ProviderFetchResult::new(usage, "web"))
            }
            SourceMode::Cli => {
                let usage = self.probe_cli().await?;
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
