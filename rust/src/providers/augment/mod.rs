//! Augment provider implementation
//!
//! Fetches usage data from Augment Code AI
//! Augment stores auth tokens and config locally

mod keepalive;

// Re-exports for future session management
#[allow(unused_imports)]
pub use keepalive::{AugmentSessionKeepalive, KeepaliveConfig};

use async_trait::async_trait;
use std::path::PathBuf;

use crate::core::{
    FetchContext, Provider, ProviderError, ProviderFetchResult, ProviderId, ProviderMetadata,
    RateWindow, SourceMode, UsageSnapshot,
};

/// Augment provider
pub struct AugmentProvider {
    metadata: ProviderMetadata,
}

impl AugmentProvider {
    pub fn new() -> Self {
        Self {
            metadata: ProviderMetadata {
                id: ProviderId::Augment,
                display_name: "Augment",
                session_label: "Session",
                weekly_label: "Monthly",
                supports_opus: false,
                supports_credits: true,
                default_enabled: false,
                is_primary: false,
                dashboard_url: Some("https://app.augmentcode.com/account"),
                status_page_url: Some("https://status.augmentcode.com"),
            },
        }
    }

    /// Get Augment config directory
    fn get_augment_config_path() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            dirs::config_dir().map(|p| p.join("augment"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            dirs::home_dir().map(|p| p.join(".augment"))
        }
    }

    /// Find Augment CLI
    fn which_augment() -> Option<PathBuf> {
        let possible_paths = [
            which::which("augment").ok(),
            #[cfg(target_os = "windows")]
            dirs::data_local_dir().map(|p| p.join("Programs").join("Augment").join("augment.exe")),
            #[cfg(not(target_os = "windows"))]
            None,
        ];

        possible_paths.into_iter().flatten().find(|p| p.exists())
    }

    /// Read Augment auth token
    async fn read_auth_token(&self) -> Result<String, ProviderError> {
        let config_path = Self::get_augment_config_path()
            .ok_or_else(|| ProviderError::NotInstalled("Augment config not found".to_string()))?;

        // Check for token file
        let token_file = config_path.join("auth.json");
        if token_file.exists() {
            let content = tokio::fs::read_to_string(&token_file)
                .await
                .map_err(|e| ProviderError::Other(e.to_string()))?;

            let json: serde_json::Value =
                serde_json::from_str(&content).map_err(|e| ProviderError::Parse(e.to_string()))?;

            if let Some(token) = json.get("access_token").and_then(|v| v.as_str()) {
                return Ok(token.to_string());
            }
        }

        // Check for credentials in VS Code extension settings
        let vscode_settings = Self::get_vscode_augment_settings().await;
        if let Some(token) = vscode_settings {
            return Ok(token);
        }

        Err(ProviderError::AuthRequired)
    }

    async fn get_vscode_augment_settings() -> Option<String> {
        #[cfg(target_os = "windows")]
        let settings_path = dirs::config_dir().map(|p| {
            p.join("Code")
                .join("User")
                .join("globalStorage")
                .join("augment.augment-vscode")
                .join("auth.json")
        });
        #[cfg(not(target_os = "windows"))]
        let settings_path = dirs::config_dir().map(|p| {
            p.join("Code")
                .join("User")
                .join("globalStorage")
                .join("augment.augment-vscode")
                .join("auth.json")
        });

        if let Some(path) = settings_path
            && path.exists()
            && let Ok(content) = tokio::fs::read_to_string(&path).await
            && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
            && let Some(token) = json.get("accessToken").and_then(|v| v.as_str())
        {
            return Some(token.to_string());
        }

        None
    }

    /// Fetch usage via Augment API
    async fn fetch_via_web(&self) -> Result<UsageSnapshot, ProviderError> {
        let token = self.read_auth_token().await?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ProviderError::Other(e.to_string()))?;

        let resp = client
            .get("https://api.augmentcode.com/v1/user/usage")
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ProviderError::AuthRequired);
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
        let used = json
            .get("used_credits")
            .or_else(|| json.get("usage"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let limit = json
            .get("credit_limit")
            .or_else(|| json.get("limit"))
            .and_then(|v| v.as_f64())
            .unwrap_or(100.0);

        let used_percent = if limit > 0.0 {
            (used / limit) * 100.0
        } else {
            0.0
        };

        let email = json
            .get("email")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let plan = json
            .get("plan")
            .or_else(|| json.get("subscription"))
            .and_then(|v| v.as_str())
            .unwrap_or("Augment");

        let mut usage = UsageSnapshot::new(RateWindow::new(used_percent)).with_login_method(plan);

        if let Some(email) = email {
            usage = usage.with_email(email);
        }

        Ok(usage)
    }

    /// Probe CLI for detection
    async fn probe_cli(&self) -> Result<UsageSnapshot, ProviderError> {
        // Check if Augment is installed (VS Code extension or CLI)
        let augment_path = Self::which_augment();
        let config_path = Self::get_augment_config_path();

        if augment_path.map(|p| p.exists()).unwrap_or(false)
            || config_path.map(|p| p.exists()).unwrap_or(false)
        {
            let usage =
                UsageSnapshot::new(RateWindow::new(0.0)).with_login_method("Augment (installed)");
            Ok(usage)
        } else {
            Err(ProviderError::NotInstalled(
                "Augment not found. Install from https://www.augmentcode.com".to_string(),
            ))
        }
    }
}

impl Default for AugmentProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for AugmentProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Augment
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<ProviderFetchResult, ProviderError> {
        tracing::debug!("Fetching Augment usage");

        match ctx.source_mode {
            SourceMode::Auto => {
                if let Ok(usage) = self.fetch_via_web().await {
                    return Ok(ProviderFetchResult::new(usage, "web"));
                }
                let usage = self.probe_cli().await?;
                Ok(ProviderFetchResult::new(usage, "cli"))
            }
            SourceMode::Web => {
                let usage = self.fetch_via_web().await?;
                Ok(ProviderFetchResult::new(usage, "web"))
            }
            SourceMode::Cli => {
                let usage = self.probe_cli().await?;
                Ok(ProviderFetchResult::new(usage, "cli"))
            }
            SourceMode::OAuth => Err(ProviderError::UnsupportedSource(SourceMode::OAuth)),
        }
    }

    fn available_sources(&self) -> Vec<SourceMode> {
        vec![SourceMode::Auto, SourceMode::Web, SourceMode::Cli]
    }

    fn supports_web(&self) -> bool {
        true
    }

    fn supports_cli(&self) -> bool {
        true
    }
}
