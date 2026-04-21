//! Alibaba (Tongyi Qianwen) provider implementation
//!
//! Fetches usage data from Tongyi Qianwen using browser cookies

use async_trait::async_trait;

use crate::browser::cookies::get_cookie_header;
use crate::core::{
    FetchContext, Provider, ProviderError, ProviderFetchResult, ProviderId, ProviderMetadata,
    RateWindow, SourceMode, UsageSnapshot,
};

const ALIBABA_INTL_COOKIE_DOMAIN: &str = "modelstudio.console.alibabacloud.com";
const ALIBABA_CN_COOKIE_DOMAIN: &str = "bailian.console.aliyun.com";
const ALIBABA_INTL_GATEWAY: &str = "https://modelstudio.console.alibabacloud.com";
const ALIBABA_CN_GATEWAY: &str = "https://bailian.console.aliyun.com";
const ALIBABA_LEGACY_DOMAIN: &str = "tongyi.aliyun.com";

pub struct AlibabaProvider {
    metadata: ProviderMetadata,
}

impl AlibabaProvider {
    pub fn new() -> Self {
        Self {
            metadata: ProviderMetadata {
                id: ProviderId::Alibaba,
                display_name: "Alibaba",
                session_label: "Daily",
                weekly_label: "Monthly",
                supports_opus: false,
                supports_credits: false,
                default_enabled: false,
                is_primary: false,
                dashboard_url: Some("https://bailian.console.aliyun.com"),
                status_page_url: None,
            },
        }
    }

    fn get_auth_cookies(
        &self,
        ctx: &FetchContext,
    ) -> Result<(String, &'static str), ProviderError> {
        if let Some(ref cookie_header) = ctx.manual_cookie_header
            && !cookie_header.trim().is_empty()
        {
            // Guess region from cookie contents
            let gateway = if cookie_header.contains("alibabacloud") {
                ALIBABA_INTL_GATEWAY
            } else {
                ALIBABA_CN_GATEWAY
            };
            return Ok((cookie_header.clone(), gateway));
        }

        // Try international domain first, then China mainland, then legacy
        for (domain, gateway) in [
            (ALIBABA_INTL_COOKIE_DOMAIN, ALIBABA_INTL_GATEWAY),
            (ALIBABA_CN_COOKIE_DOMAIN, ALIBABA_CN_GATEWAY),
            (ALIBABA_LEGACY_DOMAIN, ALIBABA_CN_GATEWAY),
        ] {
            if let Ok(cookies) = get_cookie_header(domain)
                && !cookies.is_empty()
            {
                return Ok((cookies, gateway));
            }
        }

        Err(ProviderError::AuthRequired)
    }

    async fn fetch_via_web(&self, ctx: &FetchContext) -> Result<UsageSnapshot, ProviderError> {
        let (cookies, gateway) = self.get_auth_cookies(ctx)?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ProviderError::Other(e.to_string()))?;

        let resp = client
            .get(format!("{}/api/user/info", gateway))
            .header("Cookie", &cookies)
            .header("Accept", "application/json")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            if status.as_u16() == 401 || status.as_u16() == 403 {
                return Err(ProviderError::AuthRequired);
            }
            return Err(ProviderError::Other(format!("API error: {}", status)));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        self.parse_usage_response(&json)
    }

    fn parse_usage_response(
        &self,
        json: &serde_json::Value,
    ) -> Result<UsageSnapshot, ProviderError> {
        let data = json.get("data").unwrap_or(json);

        let daily_used = data
            .get("dailyUsed")
            .or_else(|| data.get("daily_used"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let daily_limit = data
            .get("dailyLimit")
            .or_else(|| data.get("daily_limit"))
            .and_then(|v| v.as_f64())
            .unwrap_or(500.0);
        let daily_percent = if daily_limit > 0.0 {
            (daily_used / daily_limit) * 100.0
        } else {
            0.0
        };

        let monthly_used = data
            .get("monthlyUsed")
            .or_else(|| data.get("monthly_used"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let monthly_limit = data
            .get("monthlyLimit")
            .or_else(|| data.get("monthly_limit"))
            .and_then(|v| v.as_f64())
            .unwrap_or(10000.0);
        let monthly_percent = if monthly_limit > 0.0 {
            (monthly_used / monthly_limit) * 100.0
        } else {
            0.0
        };

        let primary = RateWindow::new(daily_percent);
        let secondary = RateWindow::new(monthly_percent);

        let plan = data
            .get("planName")
            .or_else(|| data.get("plan_name"))
            .or_else(|| data.get("vipType"))
            .and_then(|v| v.as_str())
            .unwrap_or("Free");

        let nickname = data
            .get("nickname")
            .or_else(|| data.get("userName"))
            .and_then(|v| v.as_str());

        let mut usage = UsageSnapshot::new(primary)
            .with_secondary(secondary)
            .with_login_method(plan);

        if let Some(name) = nickname {
            usage = usage.with_email(name.to_string());
        }

        Ok(usage)
    }
}

impl Default for AlibabaProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for AlibabaProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Alibaba
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<ProviderFetchResult, ProviderError> {
        tracing::debug!("Fetching Alibaba usage");

        match ctx.source_mode {
            SourceMode::Auto | SourceMode::Web => {
                let usage = self.fetch_via_web(ctx).await?;
                Ok(ProviderFetchResult::new(usage, "web"))
            }
            SourceMode::Cli => Err(ProviderError::UnsupportedSource(SourceMode::Cli)),
            SourceMode::OAuth => Err(ProviderError::UnsupportedSource(SourceMode::OAuth)),
        }
    }

    fn available_sources(&self) -> Vec<SourceMode> {
        vec![SourceMode::Auto, SourceMode::Web]
    }

    fn supports_web(&self) -> bool {
        true
    }

    fn supports_cli(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_usage_response_standard() {
        let provider = AlibabaProvider::new();
        let json = serde_json::json!({
            "data": {
                "dailyUsed": 50.0,
                "dailyLimit": 500.0,
                "monthlyUsed": 2000.0,
                "monthlyLimit": 10000.0,
                "planName": "Pro",
                "nickname": "test_user"
            }
        });
        let usage = provider.parse_usage_response(&json).unwrap();
        assert!((usage.primary.used_percent - 10.0).abs() < 0.01);
        assert!(usage.secondary.is_some());
        let sec = usage.secondary.unwrap();
        assert!((sec.used_percent - 20.0).abs() < 0.01);
        assert_eq!(usage.login_method.as_deref(), Some("Pro"));
        assert_eq!(usage.account_email.as_deref(), Some("test_user"));
    }

    #[test]
    fn test_parse_usage_response_empty() {
        let provider = AlibabaProvider::new();
        let json = serde_json::json!({});
        let usage = provider.parse_usage_response(&json).unwrap();
        assert!((usage.primary.used_percent - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_usage_response_snake_case() {
        let provider = AlibabaProvider::new();
        let json = serde_json::json!({
            "data": {
                "daily_used": 100.0,
                "daily_limit": 200.0,
                "monthly_used": 5000.0,
                "monthly_limit": 10000.0,
                "plan_name": "Enterprise"
            }
        });
        let usage = provider.parse_usage_response(&json).unwrap();
        assert!((usage.primary.used_percent - 50.0).abs() < 0.01);
        assert_eq!(usage.login_method.as_deref(), Some("Enterprise"));
    }
}
