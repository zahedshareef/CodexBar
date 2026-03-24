//! Codex API client for fetching usage information
//!
//! Uses OAuth tokens stored by the Codex CLI in ~/.codex/auth.json

use crate::core::{CostSnapshot, ProviderError, RateWindow, UsageSnapshot};
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use std::path::PathBuf;

const DEFAULT_BASE_URL: &str = "https://chatgpt.com/backend-api";
const USAGE_PATH: &str = "/wham/usage";

/// Codex API client
pub struct CodexApi {
    client: reqwest::Client,
    home_dir: PathBuf,
}

impl CodexApi {
    pub fn new() -> Self {
        // Build client with proper TLS settings
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            home_dir: dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")),
        }
    }

    /// Fetch usage information from Codex API
    /// Returns (UsageSnapshot, optional CostSnapshot)
    pub async fn fetch_usage(&self) -> Result<(UsageSnapshot, Option<CostSnapshot>), ProviderError> {
        // Load credentials
        let creds = self.load_credentials()?;

        // Build request URL
        let base_url = self.resolve_base_url();
        let url = format!("{}{}", base_url, USAGE_PATH);

        // Build request
        let mut request = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", creds.access_token))
            .header("User-Agent", "CodexBar")
            .header("Accept", "application/json")
            .timeout(std::time::Duration::from_secs(30));

        if let Some(account_id) = &creds.account_id {
            if !account_id.is_empty() {
                request = request.header("ChatGPT-Account-Id", account_id);
            }
        }

        let response = request.send().await?;

        if response.status() == 401 || response.status() == 403 {
            return Err(ProviderError::AuthRequired);
        }

        if !response.status().is_success() {
            return Err(ProviderError::Other(format!(
                "Codex API returned {}",
                response.status()
            )));
        }

        // Parse as raw JSON first for flexibility
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        self.build_result_from_json(&json)
    }

    fn load_credentials(&self) -> Result<CodexCredentials, ProviderError> {
        let auth_path = self.get_auth_path();

        if !auth_path.exists() {
            return Err(ProviderError::NotInstalled(
                "Codex auth.json not found. Run 'codex' to log in.".to_string(),
            ));
        }

        let content = std::fs::read_to_string(&auth_path)
            .map_err(|e| ProviderError::Other(format!("Failed to read Codex credentials: {}", e)))?;

        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ProviderError::Parse(format!("Invalid Codex credentials JSON: {}", e)))?;

        // Check for OPENAI_API_KEY first
        if let Some(api_key) = json.get("OPENAI_API_KEY").and_then(|v| v.as_str()) {
            let trimmed = api_key.trim();
            if !trimmed.is_empty() {
                return Ok(CodexCredentials {
                    access_token: trimmed.to_string(),
                    refresh_token: None,
                    account_id: None,
                });
            }
        }

        // Otherwise, look for tokens object
        let tokens = json.get("tokens").ok_or_else(|| {
            ProviderError::Parse("Codex auth.json exists but contains no tokens.".to_string())
        })?;

        let access_token = tokens.get("access_token")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ProviderError::Parse("Missing access_token in Codex credentials".to_string()))?
            .to_string();

        let refresh_token = tokens.get("refresh_token")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let account_id = tokens.get("account_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        Ok(CodexCredentials {
            access_token,
            refresh_token,
            account_id,
        })
    }

    fn get_auth_path(&self) -> PathBuf {
        // Check CODEX_HOME env var
        if let Ok(codex_home) = std::env::var("CODEX_HOME") {
            let trimmed = codex_home.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed).join("auth.json");
            }
        }

        self.home_dir.join(".codex").join("auth.json")
    }

    fn resolve_base_url(&self) -> String {
        // Check CODEX_HOME for config.toml
        let config_path = if let Ok(codex_home) = std::env::var("CODEX_HOME") {
            let trimmed = codex_home.trim();
            if !trimmed.is_empty() {
                PathBuf::from(trimmed).join("config.toml")
            } else {
                self.home_dir.join(".codex").join("config.toml")
            }
        } else {
            self.home_dir.join(".codex").join("config.toml")
        };

        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Some(base_url) = parse_chatgpt_base_url(&content) {
                let normalized = normalize_base_url(&base_url);
                // Only allow HTTPS URLs for custom base URLs to prevent token exfiltration
                if normalized.starts_with("https://") || normalized.starts_with("http://127.0.0.1") || normalized.starts_with("http://localhost") {
                    return normalized;
                }
                tracing::warn!("Ignoring insecure custom chatgpt_base_url (must be HTTPS): {}", normalized);
            }
        }

        DEFAULT_BASE_URL.to_string()
    }

    fn build_result_from_json(&self, json: &serde_json::Value) -> Result<(UsageSnapshot, Option<CostSnapshot>), ProviderError> {
        // Extract plan type
        let plan_type = json.get("plan_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Extract rate limit info - handle multiple possible structures
        let (primary, secondary) = self.extract_rate_limits(json);

        // Build login method string
        let login_method = plan_type.as_ref().map(|pt| {
            match pt.as_str() {
                "guest" => "Guest".to_string(),
                "free" => "ChatGPT Free".to_string(),
                "go" => "ChatGPT Go".to_string(),
                "plus" => "ChatGPT Plus".to_string(),
                "pro" => "ChatGPT Pro".to_string(),
                "team" => "ChatGPT Team".to_string(),
                "business" => "ChatGPT Business".to_string(),
                "enterprise" => "ChatGPT Enterprise".to_string(),
                "education" | "edu" => "ChatGPT Education".to_string(),
                other => format!("ChatGPT {}", capitalize(other)),
            }
        });

        let mut usage = UsageSnapshot::new(primary);
        if let Some(sec) = secondary {
            usage = usage.with_secondary(sec);
        }
        if let Some(method) = login_method {
            usage = usage.with_login_method(method);
        }

        // Extract credits if present
        let cost = self.extract_credits(json);

        Ok((usage, cost))
    }

    fn extract_rate_limits(&self, json: &serde_json::Value) -> (RateWindow, Option<RateWindow>) {
        // Try rate_limit object
        if let Some(rate_limit) = json.get("rate_limit") {
            let primary = rate_limit.get("primary_window")
                .map(|w| self.parse_window(w))
                .unwrap_or_else(|| RateWindow::new(0.0));

            let secondary = rate_limit.get("secondary_window")
                .map(|w| self.parse_window(w));

            return (primary, secondary);
        }

        // Try rate_limits array
        if let Some(rate_limits) = json.get("rate_limits").and_then(|v| v.as_array()) {
            if let Some(first) = rate_limits.first() {
                let primary = self.parse_window(first);
                let secondary = rate_limits.get(1).map(|w| self.parse_window(w));
                return (primary, secondary);
            }
        }

        // Try direct fields
        let used_percent = json.get("used_percent")
            .or_else(|| json.get("usage_percent"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        (RateWindow::new(used_percent), None)
    }

    fn parse_window(&self, window: &serde_json::Value) -> RateWindow {
        let used_percent = window.get("used_percent")
            .or_else(|| window.get("usage_percent"))
            .and_then(|v| v.as_f64())
            .or_else(|| window.get("used_percent").and_then(|v| v.as_i64()).map(|i| i as f64))
            .unwrap_or(0.0);

        let window_minutes = window.get("limit_window_seconds")
            .and_then(|v| v.as_i64())
            .map(|s| (s / 60) as u32);

        let reset_at = window.get("reset_at")
            .and_then(|v| v.as_i64())
            .and_then(|ts| Utc.timestamp_opt(ts, 0).single());

        RateWindow::with_details(used_percent, window_minutes, reset_at, None)
    }

    fn extract_credits(&self, json: &serde_json::Value) -> Option<CostSnapshot> {
        let credits = json.get("credits")?;

        let has_credits = credits.get("has_credits")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !has_credits {
            return None;
        }

        let unlimited = credits.get("unlimited")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if unlimited {
            return None;
        }

        let balance = credits.get("balance")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        Some(CostSnapshot::new(balance, "USD", "Credits"))
    }

    fn build_result(&self, response: UsageResponse) -> Result<(UsageSnapshot, Option<CostSnapshot>), ProviderError> {
        // Extract primary rate window
        let primary = if let Some(ref rate_limit) = response.rate_limit {
            if let Some(ref primary_window) = rate_limit.primary_window {
                RateWindow::with_details(
                    primary_window.used_percent as f64,
                    primary_window.limit_window_seconds.map(|s| (s / 60) as u32),
                    timestamp_to_datetime(primary_window.reset_at),
                    None,
                )
            } else {
                RateWindow::new(0.0)
            }
        } else {
            RateWindow::new(0.0)
        };

        // Extract secondary rate window
        let secondary = response.rate_limit.as_ref()
            .and_then(|rl| rl.secondary_window.as_ref())
            .map(|window| {
                RateWindow::with_details(
                    window.used_percent as f64,
                    window.limit_window_seconds.map(|s| (s / 60) as u32),
                    timestamp_to_datetime(window.reset_at),
                    None,
                )
            });

        // Build usage snapshot
        let login_method = response.plan_type.as_ref().map(|pt| {
            match pt.as_str() {
                "guest" => "Guest".to_string(),
                "free" => "ChatGPT Free".to_string(),
                "go" => "ChatGPT Go".to_string(),
                "plus" => "ChatGPT Plus".to_string(),
                "pro" => "ChatGPT Pro".to_string(),
                "team" => "ChatGPT Team".to_string(),
                "business" => "ChatGPT Business".to_string(),
                "enterprise" => "ChatGPT Enterprise".to_string(),
                "education" | "edu" => "ChatGPT Education".to_string(),
                other => format!("ChatGPT {}", capitalize(other)),
            }
        });

        let mut usage = UsageSnapshot::new(primary);
        if let Some(sec) = secondary {
            usage = usage.with_secondary(sec);
        }
        if let Some(method) = login_method {
            usage = usage.with_login_method(method);
        }

        // Build cost snapshot if credits are present
        let cost = response.credits.as_ref().and_then(|credits| {
            if credits.has_credits() {
                let balance = credits.balance.unwrap_or(0.0);
                if credits.unlimited() {
                    None // Unlimited credits, no need to show
                } else {
                    Some(CostSnapshot::new(balance, "USD", "Credits"))
                }
            } else {
                None
            }
        });

        Ok((usage, cost))
    }
}

impl Default for CodexApi {
    fn default() -> Self {
        Self::new()
    }
}

// --- Data structures ---

struct CodexCredentials {
    access_token: String,
    refresh_token: Option<String>,
    account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UsageResponse {
    plan_type: Option<String>,
    rate_limit: Option<RateLimitDetails>,
    credits: Option<CreditDetails>,
}

#[derive(Debug, Deserialize)]
struct RateLimitDetails {
    primary_window: Option<WindowSnapshot>,
    secondary_window: Option<WindowSnapshot>,
}

#[derive(Debug, Deserialize)]
struct WindowSnapshot {
    used_percent: i32,
    reset_at: Option<i64>,
    limit_window_seconds: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CreditDetails {
    has_credits: Option<bool>,
    unlimited: Option<bool>,
    balance: Option<f64>,
}

impl CreditDetails {
    // Helper to safely check has_credits
    fn has_credits(&self) -> bool {
        self.has_credits.unwrap_or(false)
    }

    fn unlimited(&self) -> bool {
        self.unlimited.unwrap_or(false)
    }
}

// --- Helper functions ---

fn timestamp_to_datetime(timestamp: Option<i64>) -> Option<DateTime<Utc>> {
    timestamp.and_then(|ts| Utc.timestamp_opt(ts, 0).single())
}

fn parse_chatgpt_base_url(config_content: &str) -> Option<String> {
    for line in config_content.lines() {
        // Skip comments
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        // Look for chatgpt_base_url = "..."
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            if key == "chatgpt_base_url" {
                let mut value = value.trim();
                // Remove quotes
                if (value.starts_with('"') && value.ends_with('"')) ||
                   (value.starts_with('\'') && value.ends_with('\'')) {
                    value = &value[1..value.len()-1];
                }
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

fn normalize_base_url(url: &str) -> String {
    let mut trimmed = url.trim().to_string();
    if trimmed.is_empty() {
        return DEFAULT_BASE_URL.to_string();
    }

    // Remove trailing slashes
    while trimmed.ends_with('/') {
        trimmed.pop();
    }

    // Add /backend-api if needed
    if (trimmed.starts_with("https://chatgpt.com") || trimmed.starts_with("https://chat.openai.com"))
        && !trimmed.contains("/backend-api")
    {
        trimmed.push_str("/backend-api");
    }

    trimmed
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}
