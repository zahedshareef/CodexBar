//! DeepSeek provider implementation.
//!
//! Fetches API account balance from DeepSeek's `/user/balance` endpoint.

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use crate::core::{
    FetchContext, Provider, ProviderError, ProviderFetchResult, ProviderId, ProviderMetadata,
    RateWindow, SourceMode, UsageSnapshot,
};

const DEEPSEEK_API_BASE: &str = "https://api.deepseek.com";
const DEEPSEEK_CREDENTIAL_TARGET: &str = "codexbar-deepseek";

#[derive(Debug, Deserialize)]
struct BalanceResponse {
    #[serde(default)]
    is_available: bool,
    #[serde(default)]
    balance_infos: Vec<BalanceInfo>,
}

#[derive(Debug, Deserialize, Clone)]
struct BalanceInfo {
    currency: String,
    total_balance: String,
    granted_balance: String,
    topped_up_balance: String,
}

pub struct DeepSeekProvider {
    metadata: ProviderMetadata,
    client: Client,
}

impl DeepSeekProvider {
    pub fn new() -> Self {
        Self {
            metadata: ProviderMetadata {
                id: ProviderId::DeepSeek,
                display_name: "DeepSeek",
                session_label: "Balance",
                weekly_label: "Balance",
                supports_opus: false,
                supports_credits: true,
                default_enabled: false,
                is_primary: false,
                dashboard_url: Some("https://platform.deepseek.com/usage"),
                status_page_url: Some("https://status.deepseek.com"),
            },
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    fn get_api_key(api_key: Option<&str>) -> Result<String, ProviderError> {
        if let Some(key) = api_key
            && !key.trim().is_empty()
        {
            return Ok(key.trim().to_string());
        }

        if let Ok(entry) = keyring::Entry::new(DEEPSEEK_CREDENTIAL_TARGET, "api_key")
            && let Ok(token) = entry.get_password()
            && !token.trim().is_empty()
        {
            return Ok(token);
        }

        for env in ["DEEPSEEK_API_KEY", "DEEPSEEK_KEY"] {
            if let Ok(token) = std::env::var(env)
                && !token.trim().is_empty()
            {
                return Ok(token);
            }
        }

        Err(ProviderError::NotInstalled(
            "DeepSeek API key not found. Set it in Preferences → Providers, DEEPSEEK_API_KEY, or DEEPSEEK_KEY."
                .to_string(),
        ))
    }

    async fn fetch_usage_api(&self, ctx: &FetchContext) -> Result<UsageSnapshot, ProviderError> {
        let api_key = Self::get_api_key(ctx.api_key.as_deref())?;

        let resp = self
            .client
            .get(format!("{DEEPSEEK_API_BASE}/user/balance"))
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Accept", "application/json")
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ProviderError::AuthRequired);
        }
        if !resp.status().is_success() {
            return Err(ProviderError::Other(format!(
                "DeepSeek API returned status {}",
                resp.status()
            )));
        }

        let balance: BalanceResponse = resp.json().await.map_err(|e| {
            ProviderError::Parse(format!("Failed to parse DeepSeek balance response: {e}"))
        })?;
        Ok(Self::snapshot_from_balance(balance))
    }

    fn snapshot_from_balance(balance: BalanceResponse) -> UsageSnapshot {
        let selected = balance
            .balance_infos
            .iter()
            .find(|info| info.currency.eq_ignore_ascii_case("USD"))
            .cloned()
            .or_else(|| balance.balance_infos.first().cloned());

        let Some(info) = selected else {
            let mut window = RateWindow::new(100.0);
            window.reset_description = Some("No balance information returned".to_string());
            return UsageSnapshot::new(window).with_login_method("Balance unavailable");
        };

        let total = parse_money(&info.total_balance);
        let granted = parse_money(&info.granted_balance);
        let topped_up = parse_money(&info.topped_up_balance);
        let symbol = currency_symbol(&info.currency);

        let mut window = RateWindow::new(if !balance.is_available || total <= 0.0 {
            100.0
        } else {
            0.0
        });

        window.reset_description = if !balance.is_available {
            Some("Balance unavailable for API calls".to_string())
        } else if total <= 0.0 {
            Some(format!(
                "{symbol}0.00 — add credits at platform.deepseek.com"
            ))
        } else {
            Some(format!(
                "{symbol}{total:.2} (Paid: {symbol}{topped_up:.2} / Granted: {symbol}{granted:.2})"
            ))
        };

        UsageSnapshot::new(window).with_login_method(format!(
            "{} balance: {symbol}{total:.2}",
            info.currency.to_uppercase()
        ))
    }
}

impl Default for DeepSeekProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for DeepSeekProvider {
    fn id(&self) -> ProviderId {
        ProviderId::DeepSeek
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<ProviderFetchResult, ProviderError> {
        match ctx.source_mode {
            SourceMode::Auto | SourceMode::OAuth => Ok(ProviderFetchResult::new(
                self.fetch_usage_api(ctx).await?,
                "api",
            )),
            SourceMode::Web | SourceMode::Cli => {
                Err(ProviderError::UnsupportedSource(ctx.source_mode))
            }
        }
    }

    fn available_sources(&self) -> Vec<SourceMode> {
        vec![SourceMode::Auto, SourceMode::OAuth]
    }
}

fn parse_money(value: &str) -> f64 {
    value.parse::<f64>().unwrap_or(0.0)
}

fn currency_symbol(currency: &str) -> &'static str {
    match currency.to_uppercase().as_str() {
        "CNY" | "RMB" => "¥",
        _ => "$",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_usd_balance_and_formats_paid_and_granted() {
        let snapshot = DeepSeekProvider::snapshot_from_balance(BalanceResponse {
            is_available: true,
            balance_infos: vec![
                BalanceInfo {
                    currency: "CNY".into(),
                    total_balance: "10".into(),
                    granted_balance: "1".into(),
                    topped_up_balance: "9".into(),
                },
                BalanceInfo {
                    currency: "USD".into(),
                    total_balance: "3.50".into(),
                    granted_balance: "0.50".into(),
                    topped_up_balance: "3.00".into(),
                },
            ],
        });

        assert_eq!(snapshot.primary.used_percent, 0.0);
        assert_eq!(
            snapshot.primary.reset_description.as_deref(),
            Some("$3.50 (Paid: $3.00 / Granted: $0.50)")
        );
    }

    #[test]
    fn exhausted_when_balance_unavailable() {
        let snapshot = DeepSeekProvider::snapshot_from_balance(BalanceResponse {
            is_available: false,
            balance_infos: vec![BalanceInfo {
                currency: "USD".into(),
                total_balance: "1".into(),
                granted_balance: "1".into(),
                topped_up_balance: "0".into(),
            }],
        });

        assert_eq!(snapshot.primary.used_percent, 100.0);
        assert_eq!(
            snapshot.primary.reset_description.as_deref(),
            Some("Balance unavailable for API calls")
        );
    }
}
