//! GitHub Copilot API client for fetching usage information
//!
//! Uses GitHub OAuth token stored in Windows Credential Manager

use crate::core::{ProviderError, RateWindow, UsageSnapshot};
use chrono::{DateTime, Utc};
use serde::Deserialize;

const API_URL: &str = "https://api.github.com/copilot_internal/user";

// Credential Manager targets to try
const CREDENTIAL_TARGETS: &[&str] = &[
    "codexbar-copilot",       // Our own storage
    "git:https://github.com", // GitHub CLI / Git Credential Manager
    "github.com",             // Alternative format
];

/// Copilot API client
pub struct CopilotApi {
    client: reqwest::Client,
}

impl CopilotApi {
    pub fn new() -> Self {
        // Build client with proper settings to avoid "builder error"
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client }
    }

    /// Fetch usage information from Copilot API
    pub async fn fetch_usage(&self, api_key: Option<&str>) -> Result<UsageSnapshot, ProviderError> {
        // Load token from provided api_key or credential store
        let token = self.load_token(api_key)?;

        // Build request with required headers
        let response = self
            .client
            .get(API_URL)
            .header("Authorization", format!("token {}", token))
            .header("Accept", "application/json")
            .header("Editor-Version", "vscode/1.96.2")
            .header("Editor-Plugin-Version", "copilot-chat/0.26.7")
            .header("User-Agent", "GitHubCopilotChat/0.26.7")
            .header("X-Github-Api-Version", "2025-04-01")
            .send()
            .await
            .map_err(|e| ProviderError::Other(format!("Request failed: {}", e)))?;

        if response.status() == 401 || response.status() == 403 {
            return Err(ProviderError::AuthRequired);
        }

        if !response.status().is_success() {
            return Err(ProviderError::Other(format!(
                "GitHub Copilot API returned {}",
                response.status()
            )));
        }

        let usage_response: CopilotUsageResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        self.build_snapshot(usage_response)
    }

    fn load_token(&self, api_key: Option<&str>) -> Result<String, ProviderError> {
        // Check api_key first (from settings/ctx)
        if let Some(key) = api_key {
            if !key.is_empty() && key.chars().all(|c| c.is_ascii_graphic()) {
                tracing::debug!("Using Copilot token from settings");
                return Ok(key.to_string());
            }
        }

        // Try each credential target as fallback
        for target in CREDENTIAL_TARGETS {
            if let Some(token) = self.try_load_credential(target) {
                // Git Credential Manager stores as "username:password" or just password
                // Extract just the password/token part
                let trimmed = token.trim();
                let actual_token = if let Some((_user, pass)) = trimmed.split_once(':') {
                    // Format is "username:token" - take the token part
                    pass.trim()
                } else {
                    trimmed
                };

                // Validate token - must be non-empty and contain only valid characters
                if !actual_token.is_empty() && actual_token.chars().all(|c| c.is_ascii_graphic()) {
                    tracing::debug!("Found Copilot token in credential target: {}", target);
                    return Ok(actual_token.to_string());
                }
            }
        }

        // If no token found, return error with instructions
        Err(ProviderError::NotInstalled(
            "GitHub token not found. Store a GitHub Personal Access Token in Windows Credential Manager with target 'codexbar-copilot'.".to_string()
        ))
    }

    #[cfg(target_os = "windows")]
    fn try_load_credential(&self, target: &str) -> Option<String> {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::Security::Credentials::{
            CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
        };

        let target_wide: Vec<u16> = OsStr::new(target)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut credential: *mut CREDENTIALW = std::ptr::null_mut();

        let result = unsafe {
            CredReadW(
                PCWSTR(target_wide.as_ptr()),
                CRED_TYPE_GENERIC,
                0,
                &mut credential,
            )
        };

        if result.is_err() {
            return None;
        }

        let token = unsafe {
            let cred = &*credential;
            if cred.CredentialBlobSize == 0 || cred.CredentialBlob.is_null() {
                CredFree(credential as *mut std::ffi::c_void);
                return None;
            }

            let blob =
                std::slice::from_raw_parts(cred.CredentialBlob, cred.CredentialBlobSize as usize);

            let token = String::from_utf8_lossy(blob).to_string();
            CredFree(credential as *mut std::ffi::c_void);
            token
        };

        let trimmed = token.trim();
        if !trimmed.is_empty() {
            Some(trimmed.to_string())
        } else {
            None
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn try_load_credential(&self, _target: &str) -> Option<String> {
        None
    }

    fn build_snapshot(
        &self,
        response: CopilotUsageResponse,
    ) -> Result<UsageSnapshot, ProviderError> {
        // Build primary rate window from premium_interactions
        let primary = response
            .quota_snapshots
            .premium_interactions
            .as_ref()
            .map(|snapshot| {
                let used_percent = (100.0 - snapshot.percent_remaining).max(0.0);
                RateWindow::with_details(
                    used_percent,
                    None, // Window not provided
                    parse_iso_date(&response.quota_reset_date),
                    None,
                )
            })
            .unwrap_or_else(|| RateWindow::new(0.0));

        // Build secondary rate window from chat
        let secondary = response.quota_snapshots.chat.as_ref().map(|snapshot| {
            let used_percent = (100.0 - snapshot.percent_remaining).max(0.0);
            RateWindow::with_details(
                used_percent,
                None,
                parse_iso_date(&response.quota_reset_date),
                None,
            )
        });

        // Format plan type
        let plan_type = format!("Copilot {}", capitalize(&response.copilot_plan));

        let mut usage = UsageSnapshot::new(primary);
        if let Some(sec) = secondary {
            usage = usage.with_secondary(sec);
        }
        usage = usage.with_login_method(plan_type);

        Ok(usage)
    }
}

impl Default for CopilotApi {
    fn default() -> Self {
        Self::new()
    }
}

// --- API Response Types ---

#[derive(Debug, Deserialize)]
struct CopilotUsageResponse {
    quota_snapshots: QuotaSnapshots,
    copilot_plan: String,
    assigned_date: String,
    quota_reset_date: String,
}

#[derive(Debug, Deserialize)]
struct QuotaSnapshots {
    premium_interactions: Option<QuotaSnapshot>,
    chat: Option<QuotaSnapshot>,
}

#[derive(Debug, Deserialize)]
struct QuotaSnapshot {
    entitlement: f64,
    remaining: f64,
    percent_remaining: f64,
    quota_id: String,
}

// --- Helper functions ---

fn parse_iso_date(s: &str) -> Option<DateTime<Utc>> {
    // Try RFC3339 first
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }

    // Try without timezone
    if let Ok(dt) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(DateTime::from_naive_utc_and_offset(
            dt.and_hms_opt(0, 0, 0)?,
            Utc,
        ));
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
