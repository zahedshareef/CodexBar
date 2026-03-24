//! Ollama provider implementation
//!
//! Fetches usage data by scraping the Ollama settings page
//! Uses session cookies from browser or manual input

use async_trait::async_trait;
use regex_lite::Regex;

use crate::core::{
    FetchContext, Provider, ProviderId, ProviderError, ProviderFetchResult,
    ProviderMetadata, RateWindow, SourceMode, UsageSnapshot,
};

/// Ollama settings page URL
const OLLAMA_SETTINGS_URL: &str = "https://ollama.com/settings";

/// Ollama provider
pub struct OllamaProvider {
    metadata: ProviderMetadata,
}

impl OllamaProvider {
    pub fn new() -> Self {
        Self {
            metadata: ProviderMetadata {
                id: ProviderId::Ollama,
                display_name: "Ollama",
                session_label: "Session",
                weekly_label: "Weekly",
                supports_opus: false,
                supports_credits: false,
                default_enabled: false,
                is_primary: false,
                dashboard_url: Some("https://ollama.com/settings"),
                status_page_url: None,
            },
        }
    }

    /// Fetch usage by scraping ollama.com/settings
    async fn fetch_usage_web(&self, ctx: &FetchContext) -> Result<UsageSnapshot, ProviderError> {
        let cookie_header = self.resolve_cookie_header(ctx)?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(ctx.web_timeout))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|e| ProviderError::Other(e.to_string()))?;

        let resp = client
            .get(OLLAMA_SETTINGS_URL)
            .header("Cookie", &cookie_header)
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36",
            )
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED
            || resp.status() == reqwest::StatusCode::FORBIDDEN
        {
            return Err(ProviderError::AuthRequired);
        }

        // Check for redirect to login page
        let final_url = resp.url().to_string();
        if final_url.contains("/login") || final_url.contains("/signin") {
            return Err(ProviderError::AuthRequired);
        }

        if !resp.status().is_success() {
            return Err(ProviderError::Other(format!(
                "Ollama returned status {}",
                resp.status()
            )));
        }

        let html = resp
            .text()
            .await
            .map_err(|e| ProviderError::Other(e.to_string()))?;

        self.parse_usage_html(&html)
    }

    /// Resolve cookie header from manual cookies, browser import, or context
    fn resolve_cookie_header(&self, ctx: &FetchContext) -> Result<String, ProviderError> {
        // Check manual cookie header first
        if let Some(ref cookie) = ctx.manual_cookie_header {
            if !cookie.is_empty() {
                return Ok(cookie.clone());
            }
        }

        // Try browser cookie extraction
        use crate::browser::cookies::get_cookie_header;
        match get_cookie_header("ollama.com") {
            Ok(header) if !header.is_empty() => Ok(header),
            _ => Err(ProviderError::NoCookies),
        }
    }

    /// Parse usage data from the Ollama settings HTML page
    fn parse_usage_html(&self, html: &str) -> Result<UsageSnapshot, ProviderError> {
        // Check if we're signed out
        if html.contains("Sign in") && !html.contains("Cloud Usage") && !html.contains("Session usage") {
            return Err(ProviderError::AuthRequired);
        }

        let session_percent = self.parse_usage_block(&["Session usage", "Hourly usage"], html);
        let weekly_percent = self.parse_usage_block(&["Weekly usage"], html);

        if session_percent.is_none() && weekly_percent.is_none() {
            return Err(ProviderError::Parse(
                "Could not find usage data on Ollama settings page".to_string(),
            ));
        }

        let primary = RateWindow::new(session_percent.unwrap_or(0.0));
        let mut usage = UsageSnapshot::new(primary);

        // Parse plan name
        if let Some(plan) = self.parse_plan_name(html) {
            usage = usage.with_login_method(&plan);
        }

        // Parse account email
        if let Some(email) = self.parse_account_email(html) {
            usage = usage.with_login_method(&email);
        }

        if let Some(weekly) = weekly_percent {
            usage = usage.with_secondary(RateWindow::new(weekly));
        }

        Ok(usage)
    }

    /// Parse a usage block by looking for a label then extracting the percentage
    fn parse_usage_block(&self, labels: &[&str], html: &str) -> Option<f64> {
        for label in labels {
            if let Some(pos) = html.find(label) {
                let tail = &html[pos..];
                let window = &tail[..tail.len().min(800)];

                // Try "XX% used" pattern
                let used_re = Regex::new(r"(\d+(?:\.\d+)?)\s*%\s*used").ok()?;
                if let Some(caps) = used_re.captures(window) {
                    if let Ok(val) = caps[1].parse::<f64>() {
                        return Some(val);
                    }
                }

                // Try "width: XX%" pattern (progress bar CSS)
                let width_re = Regex::new(r"width:\s*(\d+(?:\.\d+)?)%").ok()?;
                if let Some(caps) = width_re.captures(window) {
                    if let Ok(val) = caps[1].parse::<f64>() {
                        return Some(val);
                    }
                }
            }
        }
        None
    }

    /// Parse plan name from "Cloud Usage" section
    fn parse_plan_name(&self, html: &str) -> Option<String> {
        let re = Regex::new(r#"Cloud Usage\s*</span>\s*<span[^>]*>([^<]+)</span>"#).ok()?;
        re.captures(html)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().trim().to_string())
    }

    /// Parse account email from the page
    fn parse_account_email(&self, html: &str) -> Option<String> {
        let re = Regex::new(r#"[\w.+-]+@[\w-]+\.[\w.-]+"#).ok()?;
        re.find(html).map(|m| m.as_str().to_string())
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Ollama
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<ProviderFetchResult, ProviderError> {
        tracing::debug!("Fetching Ollama usage");

        match ctx.source_mode {
            SourceMode::Auto | SourceMode::Web => {
                let usage = self.fetch_usage_web(ctx).await?;
                Ok(ProviderFetchResult::new(usage, "web"))
            }
            SourceMode::OAuth | SourceMode::Cli => {
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
