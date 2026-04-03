//! NanoGPT provider implementation
//!
//! Fetches usage data from NanoGPT's REST API
//! Requires API key for authentication

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::core::{
    FetchContext, Provider, ProviderError, ProviderFetchResult, ProviderId, ProviderMetadata,
    RateWindow, SourceMode, UsageSnapshot,
};

/// NanoGPT API base URL
const NANOGPT_API_BASE: &str = "https://nano-gpt.com/api/subscription/v1";

/// Windows Credential Manager target for NanoGPT API token
const NANOGPT_CREDENTIAL_TARGET: &str = "codexbar-nanogpt";

/// NanoGPT subscription usage response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageResponse {
    active: bool,
    limits: UsageLimits,
    allow_overage: bool,
    period: BillingPeriod,
    daily_input_tokens: UsageMetric,
    weekly_input_tokens: UsageMetric,
    daily_images: Option<UsageMetric>,
    state: String,
    grace_until: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageLimits {
    weekly_input_tokens: f64,
    daily_input_tokens: f64,
    daily_images: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageMetric {
    used: f64,
    remaining: f64,
    percent_used: f64,
    reset_at: i64, // Millisecond epoch
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BillingPeriod {
    current_period_end: String,
}

impl NanoGPTProvider {
    pub fn new() -> Self {
        Self {
            metadata: ProviderMetadata {
                id: ProviderId::NanoGPT,
                display_name: "NanoGPT",
                session_label: "Daily",
                weekly_label: "Weekly",
                supports_opus: false,
                supports_credits: false,
                default_enabled: false,
                is_primary: false,
                dashboard_url: Some("https://nano-gpt.com/usage"),
                status_page_url: None,
            },
        }
    }

    /// Get API token from ctx, Windows Credential Manager, or env
    fn get_api_token(api_key: Option<&str>) -> Result<String, ProviderError> {
        if let Some(key) = api_key
            && !key.is_empty()
        {
            return Ok(key.to_string());
        }

        match keyring::Entry::new(NANOGPT_CREDENTIAL_TARGET, "api_token") {
            Ok(entry) => match entry.get_password() {
                Ok(token) => Ok(token),
                Err(_) => std::env::var("NANOGPT_API_KEY").map_err(|_| {
                    ProviderError::NotInstalled(
                        "NanoGPT API key not found. Set in Preferences → Providers or NANOGPT_API_KEY environment variable.".to_string(),
                    )
                }),
            },
            Err(_) => std::env::var("NANOGPT_API_KEY").map_err(|_| {
                ProviderError::NotInstalled(
                    "NanoGPT API key not found. Set in Preferences → Providers or NANOGPT_API_KEY environment variable.".to_string(),
                )
            }),
        }
    }

    /// Convert millisecond epoch to DateTime<Utc>
    fn ms_to_datetime(ms: i64) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp_millis(ms)
    }

    /// Fetch usage from NanoGPT API
    async fn fetch_usage_api(&self, ctx: &FetchContext) -> Result<UsageSnapshot, ProviderError> {
        let api_key = Self::get_api_token(ctx.api_key.as_deref())?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(ctx.web_timeout))
            .build()
            .map_err(|e| ProviderError::Other(e.to_string()))?;

        let resp = client
            .get(format!("{}/usage", NANOGPT_API_BASE))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Accept", "application/json")
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ProviderError::AuthRequired);
        }

        if !resp.status().is_success() {
            return Err(ProviderError::Other(format!(
                "NanoGPT API returned status {}",
                resp.status()
            )));
        }

        let response_text = resp
            .text()
            .await
            .map_err(|e| ProviderError::Other(e.to_string()))?;

        let usage: UsageResponse = serde_json::from_str(&response_text)
            .map_err(|e| ProviderError::Parse(format!("Failed to parse usage response: {}", e)))?;

        if !usage.active {
            return Err(ProviderError::AuthRequired);
        }

        // Daily input tokens (primary)
        let daily_percent = (usage.daily_input_tokens.percent_used * 100.0).clamp(0.0, 100.0);
        let mut daily_window = RateWindow::new(daily_percent);
        if let Some(reset_at) = Self::ms_to_datetime(usage.daily_input_tokens.reset_at) {
            daily_window.resets_at = Some(reset_at);
        }
        daily_window.reset_description = Some(format!(
            "{:.0}/{:.0} tokens",
            usage.daily_input_tokens.used, usage.limits.daily_input_tokens
        ));

        // Weekly input tokens (secondary)
        let weekly_percent = (usage.weekly_input_tokens.percent_used * 100.0).clamp(0.0, 100.0);
        let mut weekly_window = RateWindow::new(weekly_percent);
        if let Some(reset_at) = Self::ms_to_datetime(usage.weekly_input_tokens.reset_at) {
            weekly_window.resets_at = Some(reset_at);
        }
        weekly_window.reset_description = Some(format!(
            "{:.0}/{:.0} tokens",
            usage.weekly_input_tokens.used, usage.limits.weekly_input_tokens
        ));

        let snapshot = UsageSnapshot::new(daily_window)
            .with_secondary(weekly_window)
            .with_login_method(format!(
                "{:.0} weekly limit",
                usage.limits.weekly_input_tokens
            ));

        Ok(snapshot)
    }
}

/// NanoGPT provider
pub struct NanoGPTProvider {
    metadata: ProviderMetadata,
}

impl Default for NanoGPTProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for NanoGPTProvider {
    fn id(&self) -> ProviderId {
        ProviderId::NanoGPT
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<ProviderFetchResult, ProviderError> {
        tracing::debug!("Fetching NanoGPT usage");

        match ctx.source_mode {
            SourceMode::Auto | SourceMode::OAuth => {
                let usage = self.fetch_usage_api(ctx).await?;
                Ok(ProviderFetchResult::new(usage, "api"))
            }
            SourceMode::Web | SourceMode::Cli => {
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
