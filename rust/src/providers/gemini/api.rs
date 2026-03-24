//! Gemini API client for fetching quota information
//!
//! Uses Google Cloud Code Private API with OAuth tokens from ~/.gemini/oauth_creds.json

use crate::core::{FetchContext, ProviderError, RateWindow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const QUOTA_ENDPOINT: &str = "https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuota";
const TOKEN_REFRESH_ENDPOINT: &str = "https://oauth2.googleapis.com/token";

/// Gemini API client
pub struct GeminiApi {
    client: reqwest::Client,
    home_dir: PathBuf,
}

impl GeminiApi {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            home_dir: dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")),
        }
    }

    /// Fetch quota information from the Gemini API
    /// Returns (primary RateWindow, optional model-specific RateWindow, optional email)
    /// Note: Gemini quota API requires OAuth tokens, not API keys
    pub async fn fetch_quota(&self, _ctx: &FetchContext) -> Result<(RateWindow, Option<RateWindow>, Option<String>), ProviderError> {
        // Gemini quota endpoint requires OAuth credentials (not API keys)
        // Always load OAuth credentials from ~/.gemini/oauth_creds.json
        let mut creds = self.load_credentials()?;

        // Check if token needs refresh
        if creds.is_expired() {
            tracing::debug!("Gemini token expired, refreshing...");
            creds = self.refresh_token(&creds).await?;
        }

        let access_token = creds.access_token.clone()
            .ok_or_else(|| ProviderError::AuthRequired)?;

        // Fetch quota
        let response = self.client
            .post(QUOTA_ENDPOINT)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .body("{}")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        if response.status() == 401 {
            return Err(ProviderError::AuthRequired);
        }

        if !response.status().is_success() {
            return Err(ProviderError::Other(format!(
                "Gemini API returned {}",
                response.status()
            )));
        }

        let quota_response: QuotaResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        // Since we use OAuth, we can use the credentials we already loaded for email extraction
        self.parse_quota_response(quota_response, Some(&creds))
    }

    fn load_credentials(&self) -> Result<OAuthCredentials, ProviderError> {
        let creds_path = self.home_dir.join(".gemini").join("oauth_creds.json");

        if !creds_path.exists() {
            return Err(ProviderError::NotInstalled(
                "Not logged in to Gemini. Run 'gemini' in Terminal to authenticate.".to_string(),
            ));
        }

        let content = std::fs::read_to_string(&creds_path)
            .map_err(|e| ProviderError::Other(format!("Failed to read Gemini credentials: {}", e)))?;

        serde_json::from_str(&content)
            .map_err(|e| ProviderError::Parse(format!("Invalid Gemini credentials: {}", e)))
    }

    async fn refresh_token(&self, creds: &OAuthCredentials) -> Result<OAuthCredentials, ProviderError> {
        let refresh_token = creds.refresh_token.as_ref()
            .ok_or_else(|| ProviderError::AuthRequired)?;

        // Get OAuth client credentials from Gemini CLI
        let client_creds = self.extract_oauth_client_credentials()?;

        let params = [
            ("client_id", client_creds.client_id.as_str()),
            ("client_secret", client_creds.client_secret.as_str()),
            ("refresh_token", refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ];

        let response = self.client
            .post(TOKEN_REFRESH_ENDPOINT)
            .form(&params)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ProviderError::AuthRequired);
        }

        let refresh_response: TokenRefreshResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        // Update stored credentials
        let mut new_creds = creds.clone();
        new_creds.access_token = Some(refresh_response.access_token.clone());
        if let Some(id_token) = &refresh_response.id_token {
            new_creds.id_token = Some(id_token.clone());
        }
        if let Some(expires_in) = refresh_response.expires_in {
            let expiry_ms = (chrono::Utc::now().timestamp() as f64 + expires_in) * 1000.0;
            new_creds.expiry_date = Some(expiry_ms);
        }

        // Save updated credentials
        self.save_credentials(&new_creds)?;

        tracing::info!("Gemini token refreshed successfully");
        Ok(new_creds)
    }

    fn save_credentials(&self, creds: &OAuthCredentials) -> Result<(), ProviderError> {
        let creds_path = self.home_dir.join(".gemini").join("oauth_creds.json");
        let content = serde_json::to_string_pretty(creds)
            .map_err(|e| ProviderError::Parse(e.to_string()))?;
        std::fs::write(&creds_path, content)
            .map_err(|e| ProviderError::Other(format!("Failed to save credentials: {}", e)))?;
        Ok(())
    }

    fn extract_oauth_client_credentials(&self) -> Result<OAuthClientCredentials, ProviderError> {
        // Try to read OAuth client credentials from Gemini CLI installation
        // The Gemini CLI stores these in its internal config
        // Fall back to environment variables if CLI not found
        if let Some(home) = dirs::home_dir() {
            let cli_config = home.join(".gemini").join("client_config.json");
            if cli_config.exists() {
                if let Ok(content) = std::fs::read_to_string(&cli_config) {
                    if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let (Some(id), Some(secret)) = (
                            config.get("client_id").and_then(|v| v.as_str()),
                            config.get("client_secret").and_then(|v| v.as_str()),
                        ) {
                            return Ok(OAuthClientCredentials {
                                client_id: id.to_string(),
                                client_secret: secret.to_string(),
                            });
                        }
                    }
                }
            }
        }

        // Fall back to environment variables
        let client_id = std::env::var("GEMINI_CLIENT_ID")
            .map_err(|_| ProviderError::NotInstalled("GEMINI_CLIENT_ID not set".to_string()))?;
        let client_secret = std::env::var("GEMINI_CLIENT_SECRET")
            .map_err(|_| ProviderError::NotInstalled("GEMINI_CLIENT_SECRET not set".to_string()))?;

        Ok(OAuthClientCredentials {
            client_id,
            client_secret,
        })
    }

    fn parse_quota_response(
        &self,
        response: QuotaResponse,
        creds: Option<&OAuthCredentials>,
    ) -> Result<(RateWindow, Option<RateWindow>, Option<String>), ProviderError> {
        let buckets = response.buckets.ok_or_else(|| {
            ProviderError::Parse("No quota buckets in response".to_string())
        })?;

        if buckets.is_empty() {
            return Err(ProviderError::Parse("Empty quota buckets".to_string()));
        }

        // Group quotas by model, keeping lowest per model
        let mut model_quotas: std::collections::HashMap<String, (f64, Option<String>)> =
            std::collections::HashMap::new();

        for bucket in buckets {
            if let (Some(model_id), Some(fraction)) = (bucket.model_id, bucket.remaining_fraction) {
                let entry = model_quotas.entry(model_id).or_insert((1.0, None));
                if fraction < entry.0 {
                    *entry = (fraction, bucket.reset_time);
                }
            }
        }

        // Find Flash and Pro quotas
        let flash_quota = model_quotas.iter()
            .filter(|(k, _)| k.to_lowercase().contains("flash"))
            .min_by(|a, b| a.1.0.partial_cmp(&b.1.0).unwrap_or(std::cmp::Ordering::Equal));

        let pro_quota = model_quotas.iter()
            .filter(|(k, _)| k.to_lowercase().contains("pro"))
            .min_by(|a, b| a.1.0.partial_cmp(&b.1.0).unwrap_or(std::cmp::Ordering::Equal));

        // Build primary RateWindow from the most constrained quota
        let (primary_fraction, primary_reset) = if let Some((_, (frac, reset))) = pro_quota {
            (*frac, reset.clone())
        } else if let Some((_, (frac, reset))) = flash_quota {
            (*frac, reset.clone())
        } else if let Some((_, (frac, reset))) = model_quotas.iter().next() {
            (*frac, reset.clone())
        } else {
            (1.0, None)
        };

        let primary_percent_used = (1.0 - primary_fraction) * 100.0;
        let primary_reset_at = primary_reset.as_ref().and_then(|s| parse_iso_date(s));

        let primary = RateWindow::with_details(
            primary_percent_used,
            Some(1440), // 24 hours
            primary_reset_at,
            None,
        );

        // Model-specific window for Flash if Pro is primary
        let model_specific = if pro_quota.is_some() {
            flash_quota.map(|(_, (frac, reset))| {
                let percent_used = (1.0 - frac) * 100.0;
                let reset_at = reset.as_ref().and_then(|s| parse_iso_date(s));
                RateWindow::with_details(percent_used, Some(1440), reset_at, None)
            })
        } else {
            None
        };

        // Extract email from ID token
        let email = creds
            .and_then(|c| c.id_token.as_ref())
            .and_then(|token| extract_email_from_jwt(token));

        Ok((primary, model_specific, email))
    }
}

impl Default for GeminiApi {
    fn default() -> Self {
        Self::new()
    }
}

// --- Data structures ---

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OAuthCredentials {
    access_token: Option<String>,
    id_token: Option<String>,
    refresh_token: Option<String>,
    expiry_date: Option<f64>, // milliseconds since epoch
}

impl OAuthCredentials {
    fn is_expired(&self) -> bool {
        if let Some(expiry_ms) = self.expiry_date {
            let expiry_secs = expiry_ms / 1000.0;
            let now_secs = chrono::Utc::now().timestamp() as f64;
            now_secs > expiry_secs
        } else {
            false
        }
    }
}

#[derive(Debug)]
struct OAuthClientCredentials {
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Deserialize)]
struct TokenRefreshResponse {
    access_token: String,
    id_token: Option<String>,
    expires_in: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct QuotaResponse {
    buckets: Option<Vec<QuotaBucket>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuotaBucket {
    remaining_fraction: Option<f64>,
    reset_time: Option<String>,
    model_id: Option<String>,
    token_type: Option<String>,
}

// --- Helper functions ---

fn parse_iso_date(s: &str) -> Option<DateTime<Utc>> {
    // Try with fractional seconds first
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }

    // Try without fractional seconds
    if let Ok(dt) = chrono::DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ") {
        return Some(dt.with_timezone(&Utc));
    }

    None
}

fn extract_email_from_jwt(token: &str) -> Option<String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }

    // Decode base64url payload
    let mut payload = parts[1].replace('-', "+").replace('_', "/");

    // Add padding if needed
    let remainder = payload.len() % 4;
    if remainder > 0 {
        payload.push_str(&"=".repeat(4 - remainder));
    }

    let decoded = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &payload,
    ).ok()?;

    let json: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    json.get("email").and_then(|v| v.as_str()).map(|s| s.to_string())
}
