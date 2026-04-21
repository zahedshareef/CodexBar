//! OpenCode Go provider implementation
//!
//! Separate workspace surface that shares the `opencode.ai` cookie domain with
//! the OpenCode provider. Resolves the workspace ID, then scrapes the `/go`
//! usage page for rolling/weekly/monthly windows.

use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use uuid::Uuid;

use crate::core::{
    FetchContext, Provider, ProviderError, ProviderFetchResult, ProviderId, ProviderMetadata,
    RateWindow, SourceMode, UsageSnapshot,
};

const BASE_URL: &str = "https://opencode.ai";
const SERVER_URL: &str = "https://opencode.ai/_server";
const WORKSPACES_SERVER_ID: &str =
    "def39973159c7f0483d8793a822b8dbb10d067e12c65455fcb4608459ba0234f";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

pub struct OpenCodeGoProvider {
    metadata: ProviderMetadata,
    client: Client,
}

impl OpenCodeGoProvider {
    pub fn new() -> Self {
        Self {
            metadata: ProviderMetadata {
                id: ProviderId::OpenCodeGo,
                display_name: "OpenCode Go",
                session_label: "Rolling",
                weekly_label: "Weekly",
                supports_opus: false,
                supports_credits: false,
                default_enabled: false,
                is_primary: false,
                dashboard_url: Some("https://opencode.ai"),
                status_page_url: None,
            },
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    async fn fetch_workspace_id(&self, cookie_header: &str) -> Result<String, ProviderError> {
        let url = format!("{}?id={}", SERVER_URL, WORKSPACES_SERVER_ID);
        let response = self
            .client
            .get(&url)
            .header("Cookie", cookie_header)
            .header("X-Server-Id", WORKSPACES_SERVER_ID)
            .header("X-Server-Instance", format!("server-fn:{}", Uuid::new_v4()))
            .header("User-Agent", USER_AGENT)
            .header("Origin", BASE_URL)
            .header("Referer", BASE_URL)
            .header(
                "Accept",
                "text/javascript, application/json;q=0.9, */*;q=0.8",
            )
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            if status.as_u16() == 401 || status.as_u16() == 403 {
                return Err(ProviderError::AuthRequired);
            }
            return Err(ProviderError::Other(format!(
                "OpenCode workspace API returned {}",
                status
            )));
        }

        let text = response.text().await?;
        if Self::looks_signed_out(&text) {
            return Err(ProviderError::AuthRequired);
        }

        let ids = Self::parse_workspace_ids(&text);
        ids.into_iter()
            .next()
            .ok_or_else(|| ProviderError::Parse("No workspace ID found".to_string()))
    }

    async fn fetch_usage_page(
        &self,
        workspace_id: &str,
        cookie_header: &str,
    ) -> Result<String, ProviderError> {
        let url = format!("{}/workspace/{}/go", BASE_URL, workspace_id);
        let response = self
            .client
            .get(&url)
            .header("Cookie", cookie_header)
            .header("User-Agent", USER_AGENT)
            .header("Referer", BASE_URL)
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            if status.as_u16() == 401 || status.as_u16() == 403 {
                return Err(ProviderError::AuthRequired);
            }
            return Err(ProviderError::Other(format!(
                "OpenCode Go usage page returned {}",
                status
            )));
        }

        let text = response.text().await?;
        if Self::looks_signed_out(&text) {
            return Err(ProviderError::AuthRequired);
        }
        Ok(text)
    }

    fn parse_usage_text(text: &str) -> Result<UsageSnapshot, ProviderError> {
        let now = Utc::now();

        let rolling = Self::extract_window(text, &["rollingUsage", "rolling_usage", "rolling"])
            .ok_or_else(|| ProviderError::Parse("Missing rolling usage window".to_string()))?;
        let weekly = Self::extract_window(text, &["weeklyUsage", "weekly_usage", "weekly"]);
        let monthly = Self::extract_window(text, &["monthlyUsage", "monthly_usage", "monthly"]);

        let primary = RateWindow::with_details(
            rolling.0,
            Some(300),
            Some(now + chrono::Duration::seconds(rolling.1)),
            None,
        );
        let mut snap = UsageSnapshot::new(primary).with_login_method("OpenCode Go");

        if let Some((pct, reset)) = weekly {
            snap = snap.with_secondary(RateWindow::with_details(
                pct,
                Some(10080),
                Some(now + chrono::Duration::seconds(reset)),
                None,
            ));
        }

        if let Some((pct, reset)) = monthly {
            snap = snap.with_tertiary(RateWindow::with_details(
                pct,
                Some(43200),
                Some(now + chrono::Duration::seconds(reset)),
                None,
            ));
        }

        Ok(snap)
    }

    /// Extract `(percent, resetInSec)` for a usage block by name.
    fn extract_window(text: &str, names: &[&str]) -> Option<(f64, i64)> {
        for name in names {
            let percent_pattern = format!(
                r#"{}[^}}]*?(?:usagePercent|usedPercent|percentUsed|percent)\s*[:=]\s*([0-9]+(?:\.[0-9]+)?)"#,
                name
            );
            let reset_pattern = format!(
                r#"{}[^}}]*?(?:resetInSec|resetInSeconds|resetSeconds|resetSec)\s*[:=]\s*([0-9]+)"#,
                name
            );

            let percent = Self::extract_number(&percent_pattern, text);
            if let Some(p) = percent {
                let reset = Self::extract_number(&reset_pattern, text)
                    .map(|n| n as i64)
                    .unwrap_or(0);
                let p = if p <= 1.0 { p * 100.0 } else { p };
                return Some((p.clamp(0.0, 100.0), reset.max(0)));
            }
        }
        None
    }

    fn extract_number(pattern: &str, text: &str) -> Option<f64> {
        let re = regex_lite::Regex::new(pattern).ok()?;
        re.captures(text)?.get(1)?.as_str().parse().ok()
    }

    fn parse_workspace_ids(text: &str) -> Vec<String> {
        let pattern = r#"(wrk_[A-Za-z0-9_-]+)"#;
        let re = match regex_lite::Regex::new(pattern) {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        let mut seen = Vec::new();
        for caps in re.captures_iter(text) {
            if let Some(m) = caps.get(1) {
                let s = m.as_str().to_string();
                if !seen.contains(&s) {
                    seen.push(s);
                }
            }
        }
        seen
    }

    fn looks_signed_out(text: &str) -> bool {
        let lower = text.to_lowercase();
        lower.contains("auth/authorize")
            || lower.contains("\"signin\"")
            || lower.contains("please sign in")
    }

    async fn fetch_with_cookies(
        &self,
        cookie_header: &str,
    ) -> Result<UsageSnapshot, ProviderError> {
        let workspace_id = self.fetch_workspace_id(cookie_header).await?;
        let page = self.fetch_usage_page(&workspace_id, cookie_header).await?;
        Self::parse_usage_text(&page)
    }
}

impl Default for OpenCodeGoProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for OpenCodeGoProvider {
    fn id(&self) -> ProviderId {
        ProviderId::OpenCodeGo
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<ProviderFetchResult, ProviderError> {
        tracing::debug!("Fetching OpenCode Go usage");

        match ctx.source_mode {
            SourceMode::Auto | SourceMode::Web => {
                if let Some(ref cookie_header) = ctx.manual_cookie_header {
                    let usage = self.fetch_with_cookies(cookie_header).await?;
                    return Ok(ProviderFetchResult::new(usage, "web"));
                }

                #[cfg(windows)]
                {
                    use crate::browser::cookies::{Cookie, CookieExtractor};
                    use crate::browser::detection::BrowserDetector;

                    for browser in BrowserDetector::detect_all() {
                        if let Ok(cookies) =
                            CookieExtractor::extract_for_domain(&browser, "opencode.ai")
                        {
                            let cookie_header: String = cookies
                                .iter()
                                .map(|c: &Cookie| format!("{}={}", c.name, c.value))
                                .collect::<Vec<_>>()
                                .join("; ");
                            if !cookie_header.is_empty() {
                                match self.fetch_with_cookies(&cookie_header).await {
                                    Ok(usage) => {
                                        return Ok(ProviderFetchResult::new(usage, "web"));
                                    }
                                    Err(ProviderError::AuthRequired) => continue,
                                    Err(e) => return Err(e),
                                }
                            }
                        }
                    }
                }

                Err(ProviderError::AuthRequired)
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
    fn parses_workspace_ids() {
        let text = r#"{ id: "wrk_abc123", name: "x" } { id: "wrk_def456" }"#;
        let ids = OpenCodeGoProvider::parse_workspace_ids(text);
        assert_eq!(
            ids,
            vec!["wrk_abc123".to_string(), "wrk_def456".to_string()]
        );
    }

    #[test]
    fn parses_usage_blocks() {
        let text = r#"
            rollingUsage: { usagePercent: 42.5, resetInSec: 3600 }
            weeklyUsage: { usagePercent: 0.13, resetInSec: 86400 }
            monthlyUsage: { usagePercent: 7, resetInSec: 2592000 }
        "#;
        let snap = OpenCodeGoProvider::parse_usage_text(text).unwrap();
        assert!((snap.primary.used_percent - 42.5).abs() < 0.001);
        let secondary = snap.secondary.expect("weekly");
        // 0.13 normalized as fraction → 13%
        assert!((secondary.used_percent - 13.0).abs() < 0.001);
        let tertiary = snap.tertiary.expect("monthly");
        assert!((tertiary.used_percent - 7.0).abs() < 0.001);
    }
}
