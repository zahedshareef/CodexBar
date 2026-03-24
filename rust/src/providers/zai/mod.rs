//! z.ai provider implementation
//!
//! Fetches usage data from z.ai's quota API
//! Uses API token stored in Windows Credential Manager

pub mod mcp_details;

// Re-exports for MCP details menu
#[allow(unused_imports)]
pub use mcp_details::{McpDetailsMenu, ZaiLimitEntry, ZaiLimitType, ZaiLimitUnit, ZaiUsageDetail, ZaiUsageSnapshot};

use async_trait::async_trait;
use serde::Deserialize;

use crate::core::{
    FetchContext, Provider, ProviderId, ProviderError, ProviderFetchResult,
    ProviderMetadata, RateWindow, SourceMode, UsageSnapshot,
};

/// z.ai API endpoint for quota/usage
const ZAI_API_URL: &str = "https://api.z.ai/api/monitor/usage/quota/limit";

/// Windows Credential Manager target for z.ai API token
const ZAI_CREDENTIAL_TARGET: &str = "codexbar-zai";

/// z.ai quota response structure
#[derive(Debug, Deserialize)]
struct ZaiQuotaResponse {
    /// List of quota limits
    #[serde(default)]
    limits: Vec<ZaiLimit>,
}

#[derive(Debug, Deserialize)]
struct ZaiLimit {
    /// Limit type (e.g., "tokens", "mcp")
    #[serde(rename = "type")]
    limit_type: Option<String>,
    /// Used amount
    used: Option<f64>,
    /// Total limit
    limit: Option<f64>,
    /// Reset time (ISO 8601)
    #[serde(rename = "resetAt")]
    reset_at: Option<String>,
}

/// z.ai provider
pub struct ZaiProvider {
    metadata: ProviderMetadata,
}

impl ZaiProvider {
    pub fn new() -> Self {
        Self {
            metadata: ProviderMetadata {
                id: ProviderId::Zai,
                display_name: "z.ai",
                session_label: "Tokens",
                weekly_label: "MCP",
                supports_opus: false,
                supports_credits: true,
                default_enabled: false,
                is_primary: false,
                dashboard_url: Some("https://z.ai/dashboard"),
                status_page_url: None,
            },
        }
    }

    /// Get API token from ctx, Windows Credential Manager, or env
    fn get_api_token(api_key: Option<&str>) -> Result<String, ProviderError> {
        // Check ctx.api_key first (from settings)
        if let Some(key) = api_key {
            if !key.is_empty() {
                return Ok(key.to_string());
            }
        }

        // Try Windows Credential Manager
        match keyring::Entry::new(ZAI_CREDENTIAL_TARGET, "api_token") {
            Ok(entry) => match entry.get_password() {
                Ok(token) => Ok(token),
                Err(_) => {
                    // Try environment variable as fallback
                    std::env::var("ZAI_API_TOKEN").map_err(|_| {
                        ProviderError::NotInstalled(
                            "z.ai API token not found. Set in Preferences → Providers or ZAI_API_TOKEN environment variable.".to_string()
                        )
                    })
                }
            },
            Err(_) => {
                // Try environment variable as fallback
                std::env::var("ZAI_API_TOKEN").map_err(|_| {
                    ProviderError::NotInstalled(
                        "z.ai API token not found. Set in Preferences → Providers or ZAI_API_TOKEN environment variable.".to_string()
                    )
                })
            }
        }
    }

    /// Fetch usage from z.ai API
    async fn fetch_usage_api(&self, ctx: &FetchContext) -> Result<UsageSnapshot, ProviderError> {
        let api_token = Self::get_api_token(ctx.api_key.as_deref())?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ProviderError::Other(e.to_string()))?;

        let resp = client
            .get(ZAI_API_URL)
            .header("Authorization", format!("Bearer {}", api_token))
            .header("Accept", "application/json")
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ProviderError::AuthRequired);
        }

        if !resp.status().is_success() {
            return Err(ProviderError::Other(format!(
                "z.ai API returned status {}",
                resp.status()
            )));
        }

        let resp_bytes = resp.bytes().await
            .map_err(|e| ProviderError::Other(e.to_string()))?;

        // Handle empty response body (can happen with wrong region/endpoint)
        if resp_bytes.is_empty() {
            return Err(ProviderError::Parse(
                "Empty response body from z.ai API. Check API region and token.".to_string()
            ));
        }

        let quota: ZaiQuotaResponse = serde_json::from_slice(&resp_bytes)
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        self.parse_quota_response(&quota)
    }

    fn parse_quota_response(&self, quota: &ZaiQuotaResponse) -> Result<UsageSnapshot, ProviderError> {
        // Find tokens limit (primary/session)
        let tokens_limit = quota.limits.iter()
            .find(|l| l.limit_type.as_deref() == Some("tokens"));

        // Find MCP limit (weekly/secondary)
        let mcp_limit = quota.limits.iter()
            .find(|l| l.limit_type.as_deref() == Some("mcp"));

        // Calculate session (tokens) usage percentage
        let session_percent = if let Some(tokens) = tokens_limit {
            let used = tokens.used.unwrap_or(0.0);
            let limit = tokens.limit.unwrap_or(0.0);
            if limit > 0.0 {
                (used / limit) * 100.0
            } else if used > 0.0 {
                // No limit field but usage exists - don't report 0%
                100.0
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Calculate MCP usage percentage
        let mcp_percent = if let Some(mcp) = mcp_limit {
            let used = mcp.used.unwrap_or(0.0);
            let limit = mcp.limit.unwrap_or(0.0);
            if limit > 0.0 {
                (used / limit) * 100.0
            } else if used > 0.0 {
                100.0
            } else {
                0.0
            }
        } else {
            0.0
        };

        let mut usage = UsageSnapshot::new(RateWindow::new(session_percent))
            .with_login_method("z.ai");

        // Add secondary (MCP) usage if available
        if mcp_limit.is_some() {
            usage = usage.with_secondary(RateWindow::new(mcp_percent));
        }

        Ok(usage)
    }
}

impl Default for ZaiProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for ZaiProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Zai
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<ProviderFetchResult, ProviderError> {
        tracing::debug!("Fetching z.ai usage");

        // z.ai only supports OAuth/API token - no CLI or web cookie fallback
        match ctx.source_mode {
            SourceMode::Auto | SourceMode::OAuth => {
                let usage = self.fetch_usage_api(ctx).await?;
                Ok(ProviderFetchResult::new(usage, "oauth"))
            }
            SourceMode::Web | SourceMode::Cli => {
                // z.ai doesn't support web cookies or CLI
                Err(ProviderError::UnsupportedSource(ctx.source_mode))
            }
        }
    }

    fn available_sources(&self) -> Vec<SourceMode> {
        vec![SourceMode::Auto, SourceMode::OAuth]
    }

    fn supports_web(&self) -> bool {
        false
    }

    fn supports_cli(&self) -> bool {
        false
    }
}
