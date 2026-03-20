//! Cursor API client for fetching usage information
//!
//! Uses browser cookies to authenticate with cursor.com API

use crate::browser::cookies::get_cookie_header;
use crate::core::{CostSnapshot, ProviderError, RateWindow};
use chrono::{DateTime, Utc};
use serde::Deserialize;

const BASE_URL: &str = "https://cursor.com";
const COOKIE_DOMAINS: [&str; 2] = ["cursor.com", "cursor.sh"];
type CursorUsageResult = (
    RateWindow,
    Option<CostSnapshot>,
    Option<String>,
    Option<String>,
);

/// Cursor API client
pub struct CursorApi {
    client: reqwest::Client,
}

impl CursorApi {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Fetch usage information from Cursor API
    /// Returns (primary RateWindow, optional CostSnapshot, optional email, optional plan_type)
    pub async fn fetch_usage(&self) -> Result<CursorUsageResult, ProviderError> {
        // Try to get cookies from browser
        let cookie_header = self.get_cookie_header()?;

        // Fetch usage summary and user info in parallel
        let (usage_result, user_result) = tokio::join!(
            self.fetch_usage_summary(&cookie_header),
            self.fetch_user_info(&cookie_header)
        );

        let usage_summary = usage_result?;
        let user_info = user_result.ok();

        self.build_result(usage_summary, user_info)
    }

    fn get_cookie_header(&self) -> Result<String, ProviderError> {
        for domain in COOKIE_DOMAINS {
            match get_cookie_header(domain) {
                Ok(header) if !header.is_empty() => {
                    tracing::debug!("Found Cursor cookies for {}", domain);
                    return Ok(header);
                }
                Ok(_) => {
                    tracing::debug!("No cookies for {}", domain);
                }
                Err(e) => {
                    tracing::debug!("Cookie error for {}: {}", domain, e);
                }
            }
        }

        Err(ProviderError::NoCookies)
    }

    async fn fetch_usage_summary(
        &self,
        cookie_header: &str,
    ) -> Result<UsageSummary, ProviderError> {
        let url = format!("{}/api/usage-summary", BASE_URL);

        let response = self
            .client
            .get(&url)
            .header("Cookie", cookie_header)
            .header("Accept", "application/json")
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await?;

        if response.status() == 401 || response.status() == 403 {
            return Err(ProviderError::AuthRequired);
        }

        if !response.status().is_success() {
            return Err(ProviderError::Other(format!(
                "Cursor API returned {}",
                response.status()
            )));
        }

        response
            .json()
            .await
            .map_err(|e| ProviderError::Parse(e.to_string()))
    }

    async fn fetch_user_info(&self, cookie_header: &str) -> Result<UserInfo, ProviderError> {
        let url = format!("{}/api/auth/me", BASE_URL);

        let response = self
            .client
            .get(&url)
            .header("Cookie", cookie_header)
            .header("Accept", "application/json")
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ProviderError::Other(
                "Failed to fetch user info".to_string(),
            ));
        }

        response
            .json()
            .await
            .map_err(|e| ProviderError::Parse(e.to_string()))
    }

    fn build_result(
        &self,
        summary: UsageSummary,
        user_info: Option<UserInfo>,
    ) -> Result<CursorUsageResult, ProviderError> {
        // Parse billing cycle end date
        let billing_end = summary
            .billing_cycle_end
            .as_ref()
            .and_then(|s| parse_iso_date(s));

        // Extract plan usage for primary rate window
        let (percent_used, cost_snapshot) = if let Some(individual) = &summary.individual_usage {
            if let Some(plan) = &individual.plan {
                // Get raw values (in cents)
                let used_cents = plan.used.unwrap_or(0) as f64;
                let limit_cents = plan.limit.unwrap_or(0) as f64;

                let percent = if limit_cents > 0.0 {
                    (used_cents / limit_cents) * 100.0
                } else {
                    plan.total_percent_used.unwrap_or(0.0) * 100.0
                };

                // Build cost snapshot
                let mut cost = CostSnapshot::new(used_cents / 100.0, "USD", "Monthly");
                if limit_cents > 0.0 {
                    cost = cost.with_limit(limit_cents / 100.0);
                }
                if let Some(reset) = billing_end {
                    cost = cost.with_resets_at(reset);
                }

                (percent, Some(cost))
            } else {
                (0.0, None)
            }
        } else {
            (0.0, None)
        };

        // Build primary rate window
        let primary = RateWindow::with_details(
            percent_used,
            None, // Monthly, not fixed window
            billing_end,
            None,
        );

        // Format plan type
        let plan_type = summary
            .membership_type
            .as_ref()
            .map(|t| match t.to_lowercase().as_str() {
                "enterprise" => "Cursor Enterprise".to_string(),
                "pro" => "Cursor Pro".to_string(),
                "hobby" => "Cursor Hobby".to_string(),
                "team" => "Cursor Team".to_string(),
                other => format!("Cursor {}", capitalize(other)),
            });

        let email = user_info.as_ref().and_then(|u| u.email.clone());

        Ok((primary, cost_snapshot, email, plan_type))
    }
}

impl Default for CursorApi {
    fn default() -> Self {
        Self::new()
    }
}

// --- API Response Types ---

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageSummary {
    billing_cycle_start: Option<String>,
    billing_cycle_end: Option<String>,
    membership_type: Option<String>,
    limit_type: Option<String>,
    is_unlimited: Option<bool>,
    individual_usage: Option<IndividualUsage>,
    team_usage: Option<TeamUsage>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndividualUsage {
    plan: Option<PlanUsage>,
    on_demand: Option<OnDemandUsage>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlanUsage {
    enabled: Option<bool>,
    used: Option<i64>,
    limit: Option<i64>,
    remaining: Option<i64>,
    breakdown: Option<PlanBreakdown>,
    auto_percent_used: Option<f64>,
    api_percent_used: Option<f64>,
    total_percent_used: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlanBreakdown {
    included: Option<i64>,
    bonus: Option<i64>,
    total: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OnDemandUsage {
    enabled: Option<bool>,
    used: Option<i64>,
    limit: Option<i64>,
    remaining: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamUsage {
    on_demand: Option<OnDemandUsage>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserInfo {
    email: Option<String>,
    email_verified: Option<bool>,
    name: Option<String>,
    sub: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    picture: Option<String>,
}

// --- Helper functions ---

fn parse_iso_date(s: &str) -> Option<DateTime<Utc>> {
    // Try with fractional seconds
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }

    // Try without fractional seconds
    if let Ok(dt) = chrono::DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ") {
        return Some(dt.with_timezone(&Utc));
    }

    None
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}
