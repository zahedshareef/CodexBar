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
    enforce_daily_limit: bool,
    period: BillingPeriod,
    daily: UsageMetric,
    monthly: UsageMetric,
    state: String,
    grace_until: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageLimits {
    daily: f64,
    monthly: f64,
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
                weekly_label: "Monthly",
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

    fn usage_snapshot_from_response(usage: UsageResponse) -> Result<UsageSnapshot, ProviderError> {
        if !usage.active {
            return Err(ProviderError::AuthRequired);
        }

        // NanoGPT documents these as usage units, not tokens or dollars.
        let daily_percent = (usage.daily.percent_used * 100.0).clamp(0.0, 100.0);
        let mut daily_window = RateWindow::new(daily_percent);
        if let Some(reset_at) = Self::ms_to_datetime(usage.daily.reset_at) {
            daily_window.resets_at = Some(reset_at);
        }
        daily_window.reset_description = Some(format!(
            "{:.0}/{:.0} units",
            usage.daily.used, usage.limits.daily
        ));

        let monthly_percent = (usage.monthly.percent_used * 100.0).clamp(0.0, 100.0);
        let mut monthly_window = RateWindow::new(monthly_percent);
        if let Some(reset_at) = Self::ms_to_datetime(usage.monthly.reset_at) {
            monthly_window.resets_at = Some(reset_at);
        }
        monthly_window.reset_description = Some(format!(
            "{:.0}/{:.0} units",
            usage.monthly.used, usage.limits.monthly
        ));

        let period_note = if let Some(grace_until) = usage.grace_until.as_deref() {
            format!("{} until {}", usage.state, grace_until)
        } else if !usage.period.current_period_end.is_empty() {
            format!("{} until {}", usage.state, usage.period.current_period_end)
        } else if usage.enforce_daily_limit {
            format!("{:.0} monthly units", usage.limits.monthly)
        } else {
            usage.state
        };

        Ok(UsageSnapshot::new(daily_window)
            .with_secondary(monthly_window)
            .with_login_method(period_note))
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

        Self::usage_snapshot_from_response(usage)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_documented_daily_and_monthly_usage() {
        let response: UsageResponse = serde_json::from_value(serde_json::json!({
            "active": true,
            "limits": {
                "daily": 5000.0,
                "monthly": 60000.0
            },
            "enforceDailyLimit": true,
            "daily": {
                "used": 125.0,
                "remaining": 4875.0,
                "percentUsed": 0.025,
                "resetAt": 1738540800000_i64
            },
            "monthly": {
                "used": 3000.0,
                "remaining": 57000.0,
                "percentUsed": 0.05,
                "resetAt": 1739404800000_i64
            },
            "period": {
                "currentPeriodEnd": "2025-02-13T23:59:59.000Z"
            },
            "state": "active",
            "graceUntil": null
        }))
        .expect("documented response should deserialize");

        let usage = NanoGPTProvider::usage_snapshot_from_response(response)
            .expect("documented response should parse");

        assert!((usage.primary.used_percent - 2.5).abs() < 0.0001);
        assert_eq!(
            usage.primary.reset_description.as_deref(),
            Some("125/5000 units")
        );

        let secondary = usage.secondary.expect("monthly usage should be present");
        assert!((secondary.used_percent - 5.0).abs() < 0.0001);
        assert_eq!(
            secondary.reset_description.as_deref(),
            Some("3000/60000 units")
        );
        assert_eq!(
            usage.login_method.as_deref(),
            Some("active until 2025-02-13T23:59:59.000Z")
        );
    }

    #[test]
    fn inactive_subscription_requires_auth() {
        let response: UsageResponse = serde_json::from_value(serde_json::json!({
            "active": false,
            "limits": {
                "daily": 5000.0,
                "monthly": 60000.0
            },
            "enforceDailyLimit": true,
            "daily": {
                "used": 0.0,
                "remaining": 5000.0,
                "percentUsed": 0.0,
                "resetAt": 1738540800000_i64
            },
            "monthly": {
                "used": 0.0,
                "remaining": 60000.0,
                "percentUsed": 0.0,
                "resetAt": 1739404800000_i64
            },
            "period": {
                "currentPeriodEnd": "2025-02-13T23:59:59.000Z"
            },
            "state": "inactive",
            "graceUntil": null
        }))
        .expect("inactive response should deserialize");

        let err = NanoGPTProvider::usage_snapshot_from_response(response)
            .expect_err("inactive subscription should require auth");
        assert!(matches!(err, ProviderError::AuthRequired));
    }
}
