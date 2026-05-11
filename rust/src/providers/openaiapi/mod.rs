//! OpenAI API balance provider.
//!
//! Tracks optional platform credit balance from OpenAI's billing credit grants endpoint.

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use reqwest::Client;
use serde::Deserialize;

use crate::core::{
    CostSnapshot, FetchContext, Provider, ProviderError, ProviderFetchResult, ProviderId,
    ProviderMetadata, RateWindow, SourceMode, UsageSnapshot,
};

const OPENAI_CREDIT_GRANTS_URL: &str = "https://api.openai.com/v1/dashboard/billing/credit_grants";
const OPENAI_API_CREDENTIAL_TARGET: &str = "codexbar-openaiapi";

#[derive(Debug, Deserialize)]
struct CreditGrantsResponse {
    total_granted: f64,
    total_used: f64,
    total_available: f64,
    grants: Option<CreditGrantList>,
}

#[derive(Debug, Deserialize)]
struct CreditGrantList {
    data: Vec<CreditGrant>,
}

#[derive(Debug, Deserialize)]
struct CreditGrant {
    expires_at: Option<i64>,
}

pub struct OpenAIApiProvider {
    metadata: ProviderMetadata,
    client: Client,
}

impl OpenAIApiProvider {
    pub fn new() -> Self {
        Self {
            metadata: ProviderMetadata {
                id: ProviderId::OpenAIApi,
                display_name: "OpenAI API",
                session_label: "Credits",
                weekly_label: "Balance",
                supports_opus: false,
                supports_credits: true,
                default_enabled: false,
                is_primary: false,
                dashboard_url: Some("https://platform.openai.com/usage"),
                status_page_url: Some("https://status.openai.com"),
            },
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    fn api_key(api_key: Option<&str>) -> Result<String, ProviderError> {
        resolve_api_key(
            api_key,
            OPENAI_API_CREDENTIAL_TARGET,
            &["OPENAI_API_KEY", "OPENAI_PLATFORM_API_KEY"],
        )
    }

    async fn fetch_api(&self, api_key: &str) -> Result<ProviderFetchResult, ProviderError> {
        let response = self
            .client
            .get(OPENAI_CREDIT_GRANTS_URL)
            .bearer_auth(api_key)
            .header("Accept", "application/json")
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::FORBIDDEN {
            return Err(ProviderError::AuthRequired);
        }
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ProviderError::AuthRequired);
        }
        if !response.status().is_success() {
            return Err(ProviderError::Other(format!(
                "OpenAI API credit balance returned status {}",
                response.status()
            )));
        }

        let decoded: CreditGrantsResponse = response.json().await.map_err(|e| {
            ProviderError::Parse(format!("Failed to parse OpenAI API credit grants: {e}"))
        })?;
        Ok(result_from_grants(&decoded))
    }
}

fn result_from_grants(grants: &CreditGrantsResponse) -> ProviderFetchResult {
    let used_percent = if grants.total_granted > 0.0 {
        grants.total_used / grants.total_granted * 100.0
    } else if grants.total_available > 0.0 {
        0.0
    } else {
        100.0
    };
    let next_expiry = grants.grants.as_ref().and_then(|list| {
        list.data
            .iter()
            .filter_map(|grant| grant.expires_at)
            .filter_map(|ts| Utc.timestamp_opt(ts, 0).single())
            .filter(|date| *date > Utc::now())
            .min()
    });

    let mut primary = RateWindow::with_details(
        used_percent,
        None,
        next_expiry,
        Some(format!("${:.2} available", grants.total_available.max(0.0))),
    );
    if grants.total_granted <= 0.0 && grants.total_available > 0.0 {
        primary.used_percent = 0.0;
    }

    let usage = UsageSnapshot::new(primary).with_login_method(format!(
        "API balance: ${:.2}",
        grants.total_available.max(0.0)
    ));
    let cost = CostSnapshot::new(grants.total_used.max(0.0), "USD", "API credits")
        .with_limit(grants.total_granted.max(0.0));
    let cost = if let Some(expiry) = next_expiry {
        cost.with_resets_at(expiry)
    } else {
        cost
    };
    ProviderFetchResult::new(usage, "api").with_cost(cost)
}

impl Default for OpenAIApiProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for OpenAIApiProvider {
    fn id(&self) -> ProviderId {
        ProviderId::OpenAIApi
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<ProviderFetchResult, ProviderError> {
        match ctx.source_mode {
            SourceMode::Auto | SourceMode::OAuth => {
                let api_key = Self::api_key(ctx.api_key.as_deref())?;
                self.fetch_api(&api_key).await
            }
            SourceMode::Web | SourceMode::Cli => {
                Err(ProviderError::UnsupportedSource(ctx.source_mode))
            }
        }
    }

    fn available_sources(&self) -> Vec<SourceMode> {
        vec![SourceMode::Auto, SourceMode::OAuth]
    }
}

fn resolve_api_key(
    explicit: Option<&str>,
    credential_target: &str,
    env_names: &[&str],
) -> Result<String, ProviderError> {
    if let Some(key) = explicit
        && !key.trim().is_empty()
    {
        return Ok(key.trim().to_string());
    }
    if let Ok(entry) = keyring::Entry::new(credential_target, "api_key")
        && let Ok(key) = entry.get_password()
        && !key.trim().is_empty()
    {
        return Ok(key);
    }
    for env in env_names {
        if let Ok(key) = std::env::var(env)
            && !key.trim().is_empty()
        {
            return Ok(key);
        }
    }
    Err(ProviderError::NotInstalled(format!(
        "API key not found. Set {} in Preferences or environment.",
        env_names.join(" / ")
    )))
}

#[allow(dead_code)]
fn _assert_datetime_send(_: DateTime<Utc>) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_api_credit_snapshot_formats_available_balance() {
        let result = result_from_grants(&CreditGrantsResponse {
            total_granted: 100.0,
            total_used: 25.0,
            total_available: 75.0,
            grants: None,
        });
        assert_eq!(result.usage.primary.used_percent, 25.0);
        assert_eq!(result.cost.unwrap().remaining(), Some(75.0));
    }
}
