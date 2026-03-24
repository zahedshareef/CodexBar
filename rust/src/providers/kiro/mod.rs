//! Kiro provider implementation
//!
//! Fetches usage data from Kiro (Amazon's AI coding assistant)
//! Uses kiro-cli for authentication and usage fetching

pub mod version;

// Re-exports for version compatibility checking
#[allow(unused_imports)]
pub use version::{
    detect_version, find_kiro_cli, get_version, is_compatible, is_installed, KiroVersion,
};

use async_trait::async_trait;
use chrono::Datelike;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use regex_lite::Regex;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::core::{
    FetchContext, Provider, ProviderId, ProviderError, ProviderFetchResult,
    ProviderMetadata, RateWindow, SourceMode, UsageSnapshot,
};

/// Kiro provider (AWS AI assistant)
pub struct KiroProvider {
    metadata: ProviderMetadata,
}

impl KiroProvider {
    pub fn new() -> Self {
        Self {
            metadata: ProviderMetadata {
                id: ProviderId::Kiro,
                display_name: "Kiro",
                session_label: "Session",
                weekly_label: "Monthly",
                supports_opus: false,
                supports_credits: true,
                default_enabled: false,
                is_primary: false,
                dashboard_url: Some("https://kiro.dev/account"),
                status_page_url: Some("https://health.aws.amazon.com"),
            },
        }
    }

    /// Get Kiro config directory
    fn get_kiro_config_path() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            dirs::config_dir().map(|p| p.join("Kiro"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            dirs::home_dir().map(|p| p.join(".kiro"))
        }
    }

    /// Find Kiro CLI binary
    fn which_kiro() -> Option<PathBuf> {
        // Try kiro-cli first (the official CLI name)
        if let Ok(path) = which::which("kiro-cli") {
            return Some(path);
        }
        // Fall back to kiro
        if let Ok(path) = which::which("kiro") {
            return Some(path);
        }

        #[cfg(target_os = "windows")]
        {
            let possible_paths = [
                dirs::data_local_dir().map(|p| p.join("Programs").join("Kiro").join("kiro-cli.exe")),
                Some(PathBuf::from("C:\\Program Files\\Kiro\\kiro-cli.exe")),
            ];
            for path in possible_paths.into_iter().flatten() {
                if path.exists() {
                    return Some(path);
                }
            }
        }

        None
    }

    /// Check if user is logged in by running `kiro-cli whoami`
    async fn ensure_logged_in(&self) -> Result<(), ProviderError> {
        let cli_path = Self::which_kiro().ok_or_else(|| {
            ProviderError::NotInstalled("kiro-cli not found. Install from https://kiro.dev".to_string())
        })?;

        #[cfg(windows)]
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut cmd = Command::new(&cli_path);
        cmd.arg("whoami")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd.output()
            .await
            .map_err(|e| ProviderError::Other(format!("Failed to run kiro-cli: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
        let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
        let combined = format!("{} {}", stdout, stderr);

        if combined.contains("not logged in") || combined.contains("login required") {
            return Err(ProviderError::AuthRequired);
        }

        if !output.status.success() {
            return Err(ProviderError::Other(format!(
                "kiro-cli whoami failed with status {}",
                output.status.code().unwrap_or(-1)
            )));
        }

        Ok(())
    }

    /// Fetch usage via kiro-cli
    async fn fetch_via_cli(&self) -> Result<UsageSnapshot, ProviderError> {
        // First ensure we're logged in
        self.ensure_logged_in().await?;

        let cli_path = Self::which_kiro().ok_or_else(|| {
            ProviderError::NotInstalled("kiro-cli not found".to_string())
        })?;

        // Run the usage command
        #[cfg(windows)]
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut cmd = Command::new(&cli_path);
        cmd.args(["chat", "--no-interactive", "/usage"])
            .env("TERM", "xterm-256color")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd.output()
            .await
            .map_err(|e| ProviderError::Other(format!("Failed to run kiro-cli: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = if stdout.trim().is_empty() { &stderr } else { &stdout };

        // Check for login errors
        let lowered = combined.to_lowercase();
        if lowered.contains("not logged in")
            || lowered.contains("login required")
            || lowered.contains("failed to initialize auth portal")
            || lowered.contains("kiro-cli login")
            || lowered.contains("oauth error")
        {
            return Err(ProviderError::AuthRequired);
        }

        self.parse_cli_output(&combined)
    }

    /// Parse CLI output to extract usage information
    fn parse_cli_output(&self, output: &str) -> Result<UsageSnapshot, ProviderError> {
        let stripped = Self::strip_ansi(output);
        let trimmed = stripped.trim();

        if trimmed.is_empty() {
            return Err(ProviderError::Parse("Empty output from kiro-cli".to_string()));
        }

        let lowered = stripped.to_lowercase();
        if lowered.contains("could not retrieve usage information") {
            return Err(ProviderError::Parse("Kiro CLI could not retrieve usage information".to_string()));
        }

        // Parse plan name from "| KIRO FREE" or similar (legacy format)
        let mut plan_name = "Kiro".to_string();
        if let Ok(re) = Regex::new(r"\|\s*(KIRO\s+\w+)") {
            if let Some(caps) = re.captures(&stripped) {
                if let Some(m) = caps.get(1) {
                    plan_name = m.as_str().trim().to_string();
                }
            }
        }

        // Parse plan name from "Plan: Q Developer Pro" (new format, kiro-cli 1.24+)
        let mut matched_new_format = false;
        if let Ok(re) = Regex::new(r"Plan:\s*(.+)") {
            if let Some(caps) = re.captures(&stripped) {
                if let Some(m) = caps.get(1) {
                    let plan_line = m.as_str().trim();
                    if let Some(first_line) = plan_line.lines().next() {
                        plan_name = first_line.trim().to_string();
                        matched_new_format = true;
                    }
                }
            }
        }

        // Check if this is a managed plan with no usage data
        let is_managed_plan = lowered.contains("managed by admin")
            || lowered.contains("managed by organization");

        // Parse reset date from "resets on 01/01"
        let mut reset_date: Option<chrono::DateTime<chrono::Utc>> = None;
        if let Ok(re) = Regex::new(r"resets on (\d{2}/\d{2})") {
            if let Some(caps) = re.captures(&stripped) {
                if let Some(m) = caps.get(1) {
                    reset_date = Self::parse_reset_date(m.as_str());
                }
            }
        }

        // Parse credits percentage from progress bar like "████...█ X%"
        let mut credits_percent: f64 = 0.0;
        let mut matched_percent = false;
        if let Ok(re) = Regex::new(r"█+\s*(\d+)%") {
            if let Some(caps) = re.captures(&stripped) {
                if let Some(m) = caps.get(1) {
                    credits_percent = m.as_str().parse().unwrap_or(0.0);
                    matched_percent = true;
                }
            }
        }

        // Parse credits used/total from "(X.XX of Y covered in plan)"
        let mut credits_used: f64 = 0.0;
        let mut credits_total: f64 = 50.0; // default free tier
        let mut matched_credits = false;
        if let Ok(re) = Regex::new(r"\((\d+\.?\d*)\s+of\s+(\d+)\s+covered") {
            if let Some(caps) = re.captures(&stripped) {
                if let (Some(used), Some(total)) = (caps.get(1), caps.get(2)) {
                    credits_used = used.as_str().parse().unwrap_or(0.0);
                    credits_total = total.as_str().parse().unwrap_or(50.0);
                    matched_credits = true;
                }
            }
        }

        // Calculate percent from credits if we didn't get it from the progress bar
        if !matched_percent && matched_credits && credits_total > 0.0 {
            credits_percent = (credits_used / credits_total) * 100.0;
        }

        // Parse bonus credits from "Bonus credits: X.XX/Y credits used, expires in Z days"
        let mut bonus_window: Option<RateWindow> = None;
        if let Ok(re) = Regex::new(r"Bonus credits:\s*(\d+\.?\d*)/(\d+)") {
            if let Some(caps) = re.captures(&stripped) {
                if let (Some(used), Some(total)) = (caps.get(1), caps.get(2)) {
                    let bonus_used: f64 = used.as_str().parse().unwrap_or(0.0);
                    let bonus_total: f64 = total.as_str().parse().unwrap_or(0.0);
                    if bonus_total > 0.0 {
                        let bonus_percent = (bonus_used / bonus_total) * 100.0;

                        // Try to get expiry days
                        let mut expiry_desc: Option<String> = None;
                        if let Ok(exp_re) = Regex::new(r"expires in (\d+) days?") {
                            if let Some(exp_caps) = exp_re.captures(&stripped) {
                                if let Some(days) = exp_caps.get(1) {
                                    expiry_desc = Some(format!("expires in {}d", days.as_str()));
                                }
                            }
                        }

                        bonus_window = Some(RateWindow::with_details(
                            bonus_percent,
                            None,
                            None,
                            expiry_desc,
                        ));
                    }
                }
            }
        }

        // Managed plans in new format may omit usage metrics
        if matched_new_format && is_managed_plan && !matched_percent && !matched_credits {
            let usage = UsageSnapshot::new(RateWindow::new(0.0))
                .with_login_method(&plan_name);
            return Ok(usage);
        }

        // Require at least some pattern to match
        if !matched_percent && !matched_credits {
            if matched_new_format || plan_name != "Kiro" {
                // We got a plan name but no usage data
                let usage = UsageSnapshot::new(RateWindow::new(0.0))
                    .with_login_method(&plan_name);
                return Ok(usage);
            }
            // If we have the CLI but can't parse, at least report it's installed
            let usage = UsageSnapshot::new(RateWindow::new(0.0))
                .with_login_method("Kiro (installed)");
            return Ok(usage);
        }

        let primary = RateWindow::with_details(
            credits_percent,
            None, // monthly, no fixed window
            reset_date,
            None,
        );

        let mut usage = UsageSnapshot::new(primary)
            .with_login_method(&plan_name);

        if let Some(bonus) = bonus_window {
            usage = usage.with_secondary(bonus);
        }

        Ok(usage)
    }

    /// Strip ANSI escape sequences from text
    fn strip_ansi(text: &str) -> String {
        // Simple ANSI stripping - remove escape sequences
        let mut result = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\x1B' {
                // Skip escape sequence
                if chars.peek() == Some(&'[') {
                    chars.next(); // consume '['
                    // Skip until we hit a letter
                    while let Some(&next) = chars.peek() {
                        chars.next();
                        if next.is_ascii_alphabetic() {
                            break;
                        }
                    }
                } else if chars.peek() == Some(&']') {
                    // OSC sequence - skip until BEL or ST
                    while let Some(next) = chars.next() {
                        if next == '\x07' || next == '\\' {
                            break;
                        }
                    }
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Parse reset date from MM/DD format
    fn parse_reset_date(date_str: &str) -> Option<chrono::DateTime<chrono::Utc>> {
        let parts: Vec<&str> = date_str.split('/').collect();
        if parts.len() != 2 {
            return None;
        }

        let month: u32 = parts[0].parse().ok()?;
        let day: u32 = parts[1].parse().ok()?;

        let now = chrono::Utc::now();
        let current_year = now.year();

        // Try current year first
        if let Some(date) = chrono::NaiveDate::from_ymd_opt(current_year, month, day) {
            let datetime = date.and_hms_opt(0, 0, 0)?;
            let utc = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(datetime, chrono::Utc);
            if utc > now {
                return Some(utc);
            }
        }

        // If in the past, use next year
        if let Some(date) = chrono::NaiveDate::from_ymd_opt(current_year + 1, month, day) {
            let datetime = date.and_hms_opt(0, 0, 0)?;
            return Some(chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(datetime, chrono::Utc));
        }

        None
    }
}

impl Default for KiroProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for KiroProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Kiro
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<ProviderFetchResult, ProviderError> {
        tracing::debug!("Fetching Kiro usage");

        match ctx.source_mode {
            SourceMode::Auto | SourceMode::Cli => {
                let usage = self.fetch_via_cli().await?;
                Ok(ProviderFetchResult::new(usage, "cli"))
            }
            SourceMode::Web => {
                // Kiro doesn't have a direct web API, use CLI
                let usage = self.fetch_via_cli().await?;
                Ok(ProviderFetchResult::new(usage, "cli"))
            }
            SourceMode::OAuth => {
                Err(ProviderError::UnsupportedSource(SourceMode::OAuth))
            }
        }
    }

    fn available_sources(&self) -> Vec<SourceMode> {
        vec![SourceMode::Auto, SourceMode::Cli]
    }

    fn supports_web(&self) -> bool {
        false
    }

    fn supports_cli(&self) -> bool {
        true
    }
}
