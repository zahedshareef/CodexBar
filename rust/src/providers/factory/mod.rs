//! Droid (Factory) provider implementation
//!
//! Fetches usage data from Factory.ai (Droid)
//! Uses browser cookies or WorkOS refresh tokens for authentication

use async_trait::async_trait;
use serde::Deserialize;

use crate::browser::cookies::CookieExtractor;
use crate::browser::detection::BrowserDetector;
use crate::core::{
    FetchContext, Provider, ProviderId, ProviderError, ProviderFetchResult,
    ProviderMetadata, RateWindow, SourceMode, UsageSnapshot,
};

/// Factory.ai API endpoints
const FACTORY_AUTH_URL: &str = "https://app.factory.ai/api/app/auth/me";
const FACTORY_USAGE_URL: &str = "https://app.factory.ai/api/organization/subscription/usage";

/// Factory usage response
#[derive(Debug, Deserialize)]
struct FactoryUsageResponse {
    #[serde(default)]
    standard: Option<FactoryUsageWindow>,
    #[serde(default)]
    premium: Option<FactoryUsageWindow>,
}

#[derive(Debug, Deserialize)]
struct FactoryUsageWindow {
    used: Option<f64>,
    allowance: Option<f64>,
}

/// Factory auth response
#[derive(Debug, Deserialize)]
struct FactoryAuthResponse {
    user: Option<FactoryUser>,
    organization: Option<FactoryOrganization>,
}

#[derive(Debug, Deserialize)]
struct FactoryUser {
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FactoryOrganization {
    name: Option<String>,
    tier: Option<String>,
    #[serde(rename = "planName")]
    plan_name: Option<String>,
}

/// Droid (Factory) provider
pub struct FactoryProvider {
    metadata: ProviderMetadata,
}

impl FactoryProvider {
    pub fn new() -> Self {
        Self {
            metadata: ProviderMetadata {
                id: ProviderId::Factory,
                display_name: "Droid",
                session_label: "Standard",
                weekly_label: "Premium",
                supports_opus: false,
                supports_credits: true,
                default_enabled: false,
                is_primary: false,
                dashboard_url: Some("https://app.factory.ai"),
                status_page_url: None,
            },
        }
    }

    /// Get cookies for Factory.ai from browser
    fn get_cookies(&self) -> Result<String, ProviderError> {
        let browsers = BrowserDetector::detect_all();

        if browsers.is_empty() {
            return Err(ProviderError::NoCookies);
        }

        // Try each browser to find Factory cookies
        for browser in &browsers {
            if let Ok(cookies) = CookieExtractor::extract_for_domain(browser, "app.factory.ai") {
                if !cookies.is_empty() {
                    // Convert to cookie header string
                    let cookie_str = cookies.iter()
                        .map(|c| c.to_header_value())
                        .collect::<Vec<_>>()
                        .join("; ");
                    return Ok(cookie_str);
                }
            }
        }

        Err(ProviderError::NoCookies)
    }

    /// Fetch auth info from Factory API
    async fn fetch_auth_info(&self, cookies: &str) -> Result<FactoryAuthResponse, ProviderError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ProviderError::Other(e.to_string()))?;

        let resp = client
            .get(FACTORY_AUTH_URL)
            .header("Cookie", cookies)
            .header("Accept", "application/json")
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ProviderError::AuthRequired);
        }

        if !resp.status().is_success() {
            return Err(ProviderError::Other(format!(
                "Factory auth API returned status {}",
                resp.status()
            )));
        }

        resp.json().await
            .map_err(|e| ProviderError::Parse(e.to_string()))
    }

    /// Fetch usage from Factory API
    async fn fetch_usage_api(&self, cookies: &str) -> Result<FactoryUsageResponse, ProviderError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ProviderError::Other(e.to_string()))?;

        let resp = client
            .get(FACTORY_USAGE_URL)
            .header("Cookie", cookies)
            .header("Accept", "application/json")
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ProviderError::AuthRequired);
        }

        if !resp.status().is_success() {
            return Err(ProviderError::Other(format!(
                "Factory usage API returned status {}",
                resp.status()
            )));
        }

        resp.json().await
            .map_err(|e| ProviderError::Parse(e.to_string()))
    }

    /// Fetch usage via web cookies
    async fn fetch_via_web(&self) -> Result<UsageSnapshot, ProviderError> {
        let cookies = self.get_cookies()?;

        // Fetch auth info and usage in parallel conceptually, but sequentially here
        let auth_info = self.fetch_auth_info(&cookies).await.ok();
        let usage_data = self.fetch_usage_api(&cookies).await?;

        // Calculate standard tokens usage
        let standard_percent = if let Some(ref standard) = usage_data.standard {
            let used = standard.used.unwrap_or(0.0);
            let allowance = standard.allowance.unwrap_or(1.0);
            if allowance > 0.0 {
                (used / allowance) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Calculate premium tokens usage
        let premium_percent = if let Some(ref premium) = usage_data.premium {
            let used = premium.used.unwrap_or(0.0);
            let allowance = premium.allowance.unwrap_or(1.0);
            if allowance > 0.0 {
                (used / allowance) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };

        let mut usage = UsageSnapshot::new(RateWindow::new(standard_percent));

        // Add premium as secondary
        if usage_data.premium.is_some() {
            usage = usage.with_secondary(RateWindow::new(premium_percent));
        }

        // Add auth info
        if let Some(auth) = auth_info {
            if let Some(user) = auth.user {
                if let Some(email) = user.email {
                    usage = usage.with_email(email);
                }
            }
            if let Some(org) = auth.organization {
                // Build login method from tier and plan
                let tier = org.tier.unwrap_or_else(|| "Droid".to_string());
                let plan = org.plan_name.unwrap_or_default();
                let login_method = if plan.is_empty() {
                    tier
                } else {
                    format!("{} ({})", tier, plan)
                };
                usage = usage.with_login_method(login_method);

                if let Some(org_name) = org.name {
                    usage = usage.with_organization(org_name);
                }
            }
        } else {
            usage = usage.with_login_method("Droid");
        }

        Ok(usage)
    }
}

impl Default for FactoryProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for FactoryProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Factory
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<ProviderFetchResult, ProviderError> {
        tracing::debug!("Fetching Droid (Factory) usage");

        match ctx.source_mode {
            SourceMode::Auto | SourceMode::Web => {
                let usage = self.fetch_via_web().await?;
                Ok(ProviderFetchResult::new(usage, "web"))
            }
            SourceMode::Cli | SourceMode::OAuth => {
                // Droid doesn't have CLI or OAuth support
                Err(ProviderError::UnsupportedSource(ctx.source_mode))
            }
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
