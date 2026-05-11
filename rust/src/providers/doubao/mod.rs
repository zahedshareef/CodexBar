//! Doubao / Volcengine Ark provider implementation.
//!
//! Probes Ark chat-completions with a one-token request and reads rate-limit headers.

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use reqwest::Client;
use serde_json::json;

use crate::core::{
    FetchContext, Provider, ProviderError, ProviderFetchResult, ProviderId, ProviderMetadata,
    RateWindow, SourceMode, UsageSnapshot,
};

const DOUBAO_API_URL: &str = "https://ark.cn-beijing.volces.com/api/coding/v3/chat/completions";
const DOUBAO_CREDENTIAL_TARGET: &str = "codexbar-doubao";
const PROBE_MODELS: &[&str] = &[
    "doubao-seed-2.0-code",
    "doubao-1.5-pro-32k",
    "doubao-lite-32k",
];

pub struct DoubaoProvider {
    metadata: ProviderMetadata,
    client: Client,
}

impl DoubaoProvider {
    pub fn new() -> Self {
        Self {
            metadata: ProviderMetadata {
                id: ProviderId::Doubao,
                display_name: "Doubao",
                session_label: "Requests",
                weekly_label: "Usage",
                supports_opus: false,
                supports_credits: false,
                default_enabled: false,
                is_primary: false,
                dashboard_url: Some(
                    "https://console.volcengine.com/ark/region:ark+cn-beijing/usage",
                ),
                status_page_url: None,
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
            DOUBAO_CREDENTIAL_TARGET,
            &["ARK_API_KEY", "DOUBAO_API_KEY", "VOLCENGINE_API_KEY"],
        )
    }

    async fn fetch_api(&self, api_key: &str) -> Result<UsageSnapshot, ProviderError> {
        let mut last_error = None;
        for model in PROBE_MODELS {
            match self.probe(api_key, model).await {
                Ok(snapshot) => return Ok(snapshot),
                Err(error @ ProviderError::AuthRequired) => return Err(error),
                Err(error) => {
                    last_error = Some(error);
                }
            }
        }
        Err(last_error
            .unwrap_or_else(|| ProviderError::Other("All Doubao probe models failed".into())))
    }

    async fn probe(&self, api_key: &str, model: &str) -> Result<UsageSnapshot, ProviderError> {
        let response = self
            .client
            .post(DOUBAO_API_URL)
            .bearer_auth(api_key)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": model,
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "hi"}],
            }))
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ProviderError::AuthRequired);
        }

        let status = response.status();
        if status != reqwest::StatusCode::OK && status != reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(ProviderError::Other(format!(
                "Doubao probe model {model} returned status {status}"
            )));
        }

        let headers = response.headers().clone();
        let body: serde_json::Value = response.json().await.unwrap_or_else(|_| json!({}));
        Ok(snapshot_from_headers(&headers, &body))
    }
}

fn snapshot_from_headers(
    headers: &reqwest::header::HeaderMap,
    body: &serde_json::Value,
) -> UsageSnapshot {
    let remaining = int_header(headers, "x-ratelimit-remaining-requests");
    let limit = int_header(headers, "x-ratelimit-limit-requests");
    let resets_at = string_header(headers, "x-ratelimit-reset-requests").and_then(parse_reset_time);

    let (used_percent, detail) = if let (Some(remaining), Some(limit)) = (remaining, limit) {
        let used = (limit - remaining).max(0);
        let percent = if limit > 0 {
            used as f64 / limit as f64 * 100.0
        } else {
            0.0
        };
        (percent, format!("{used}/{limit} requests"))
    } else if let Some(total_tokens) = body
        .get("usage")
        .and_then(|usage| usage.get("total_tokens"))
        .and_then(|value| value.as_i64())
    {
        (0.0, format!("Active - {total_tokens} tokens observed"))
    } else {
        (0.0, "Active - check dashboard for details".to_string())
    };

    let mut window = RateWindow::with_details(used_percent, None, resets_at, Some(detail));
    if window.used_percent.is_nan() {
        window.used_percent = 0.0;
    }
    UsageSnapshot::new(window)
}

fn int_header(headers: &reqwest::header::HeaderMap, name: &str) -> Option<i64> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<i64>().ok())
}

fn string_header(headers: &reqwest::header::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string())
}

fn parse_reset_time(value: String) -> Option<DateTime<Utc>> {
    let trimmed = value.trim();
    if let Ok(ts) = trimmed.parse::<i64>() {
        return Utc.timestamp_opt(ts, 0).single();
    }
    DateTime::parse_from_rfc3339(trimmed)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

impl Default for DoubaoProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for DoubaoProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Doubao
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<ProviderFetchResult, ProviderError> {
        match ctx.source_mode {
            SourceMode::Auto | SourceMode::OAuth => {
                let api_key = Self::api_key(ctx.api_key.as_deref())?;
                Ok(ProviderFetchResult::new(
                    self.fetch_api(&api_key).await?,
                    "api",
                ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue};

    #[test]
    fn doubao_snapshot_uses_rate_limit_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-ratelimit-remaining-requests",
            HeaderValue::from_static("25"),
        );
        headers.insert(
            "x-ratelimit-limit-requests",
            HeaderValue::from_static("100"),
        );
        let snapshot = snapshot_from_headers(&headers, &json!({}));
        assert_eq!(snapshot.primary.used_percent, 75.0);
    }
}
