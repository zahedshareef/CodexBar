//! Claude provider implementation

mod oauth;
mod web_api;

use async_trait::async_trait;
use regex_lite::Regex;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::core::{
    FetchContext, Provider, ProviderError, ProviderFetchResult, ProviderId, ProviderMetadata,
    RateWindow, SourceMode, UsageSnapshot,
};

pub use oauth::ClaudeOAuthFetcher;
pub use web_api::ClaudeWebApiFetcher;

/// Claude provider implementation
pub struct ClaudeProvider {
    metadata: ProviderMetadata,
    web_fetcher: ClaudeWebApiFetcher,
    oauth_fetcher: ClaudeOAuthFetcher,
}

impl ClaudeProvider {
    pub fn new() -> Self {
        Self {
            metadata: ProviderMetadata {
                id: ProviderId::Claude,
                display_name: "Claude",
                session_label: "Session (5h)",
                weekly_label: "Weekly",
                supports_opus: true,
                supports_credits: true,
                default_enabled: true,
                is_primary: true,
                dashboard_url: Some("https://claude.ai/settings/usage"),
                status_page_url: Some("https://status.anthropic.com"),
            },
            web_fetcher: ClaudeWebApiFetcher::new(),
            oauth_fetcher: ClaudeOAuthFetcher::new(),
        }
    }
}

impl Default for ClaudeProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for ClaudeProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Claude
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<ProviderFetchResult, ProviderError> {
        match ctx.source_mode {
            SourceMode::Auto => {
                // Try OAuth first, then Web, then CLI
                if let Ok(result) = self.fetch_via_oauth(ctx).await {
                    return Ok(result);
                }
                if let Ok(result) = self.fetch_via_web(ctx).await {
                    return Ok(result);
                }
                self.fetch_via_cli(ctx).await
            }
            SourceMode::OAuth => self.fetch_via_oauth(ctx).await,
            SourceMode::Web => self.fetch_via_web(ctx).await,
            SourceMode::Cli => self.fetch_via_cli(ctx).await,
        }
    }

    fn available_sources(&self) -> Vec<SourceMode> {
        vec![
            SourceMode::Auto,
            SourceMode::OAuth,
            SourceMode::Web,
            SourceMode::Cli,
        ]
    }

    fn supports_oauth(&self) -> bool {
        true
    }

    fn supports_web(&self) -> bool {
        true
    }

    fn supports_cli(&self) -> bool {
        true
    }

    fn detect_version(&self) -> Option<String> {
        detect_claude_version()
    }
}

impl ClaudeProvider {
    async fn fetch_via_oauth(
        &self,
        _ctx: &FetchContext,
    ) -> Result<ProviderFetchResult, ProviderError> {
        tracing::debug!("Attempting OAuth fetch for Claude");
        self.oauth_fetcher.fetch().await
    }

    async fn fetch_via_web(
        &self,
        ctx: &FetchContext,
    ) -> Result<ProviderFetchResult, ProviderError> {
        tracing::debug!("Attempting Web API fetch for Claude");

        // Check for manual cookie header first
        if let Some(ref cookie_header) = ctx.manual_cookie_header {
            tracing::debug!("Using manual cookie header");
            return self
                .web_fetcher
                .fetch_with_cookie_header(cookie_header)
                .await;
        }

        // Otherwise, try to extract cookies from browser
        self.web_fetcher.fetch_with_cookies().await
    }

    async fn fetch_via_cli(
        &self,
        _ctx: &FetchContext,
    ) -> Result<ProviderFetchResult, ProviderError> {
        tracing::debug!("Attempting CLI probe for Claude");

        // Check if claude CLI exists
        let claude_path = which_claude().ok_or_else(|| {
            ProviderError::NotInstalled(
                "Claude CLI not found. Install from https://docs.claude.ai/claude-code".to_string(),
            )
        })?;

        // Run claude CLI with /usage command via stdin
        // We spawn claude in non-interactive mode and send /usage
        #[cfg(windows)]
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut cmd = Command::new(&claude_path);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("TERM", "xterm-256color")
            .env("NO_COLOR", "1"); // Try to disable colors
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let mut child = cmd
            .spawn()
            .map_err(|e| ProviderError::Other(format!("Failed to spawn claude: {}", e)))?;

        // Send /usage command
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(b"/usage\n").await;
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            let _ = stdin.write_all(b"/exit\n").await;
            drop(stdin);
        }

        // Wait for output with timeout
        let output =
            tokio::time::timeout(std::time::Duration::from_secs(30), child.wait_with_output())
                .await
                .map_err(|_| ProviderError::Timeout)?
                .map_err(|e| ProviderError::Other(format!("Claude CLI failed: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}\n{}", stdout, stderr);

        // Check for common error conditions
        let lowered = combined.to_lowercase();
        if lowered.contains("not logged in") || lowered.contains("login required") {
            return Err(ProviderError::AuthRequired);
        }
        if lowered.contains("token expired") || lowered.contains("token_expired") {
            return Err(ProviderError::OAuth(
                "Token expired. Run `claude login` to refresh.".to_string(),
            ));
        }
        if lowered.contains("authentication_error") {
            return Err(ProviderError::OAuth(
                "Authentication error. Run `claude login`.".to_string(),
            ));
        }

        // Parse the usage output
        self.parse_cli_output(&combined)
    }

    /// Parse Claude CLI /usage output
    fn parse_cli_output(&self, output: &str) -> Result<ProviderFetchResult, ProviderError> {
        let clean = strip_ansi(output);
        let clean_lower = clean.to_lowercase();

        if clean.trim().is_empty() {
            return Err(ProviderError::Parse(
                "Empty output from Claude CLI".to_string(),
            ));
        }

        // Parse session percent: "X% used" or "X% left"
        let mut session_percent: Option<f64> = None;
        let mut weekly_percent: Option<f64> = None;
        let mut opus_percent: Option<f64> = None;

        // Look for "Current session" section
        if let Some(session_pct) = extract_percent_near_label(&clean, "current session") {
            session_percent = Some(session_pct);
        }

        // Look for "Current week" section
        if let Some(weekly_pct) = extract_percent_near_label(&clean, "current week") {
            weekly_percent = Some(weekly_pct);
        }

        // Look for Opus/Sonnet specific
        if let Some(opus_pct) = extract_percent_near_label(&clean, "opus") {
            opus_percent = Some(opus_pct);
        } else if let Some(sonnet_pct) = extract_percent_near_label(&clean, "sonnet") {
            opus_percent = Some(sonnet_pct);
        }

        // Fallback: collect all percentages in order
        if session_percent.is_none() {
            let all_percents = extract_all_percents(&clean);
            if !all_percents.is_empty() {
                session_percent = Some(all_percents[0]);
            }
            if all_percents.len() > 1 && weekly_percent.is_none() {
                weekly_percent = Some(all_percents[1]);
            }
            if all_percents.len() > 2 && opus_percent.is_none() {
                opus_percent = Some(all_percents[2]);
            }
        }

        // Extract identity info
        let email = extract_email(&clean);
        let login_method = extract_login_method(&clean);

        // Extract reset times
        let session_reset = extract_reset_description(&clean, "current session");
        let weekly_reset = extract_reset_description(&clean, "current week");
        let short_form_reset = if clean_lower.contains("out of extra usage") {
            extract_inline_reset_description(&clean)
        } else {
            None
        };
        let session_reset = session_reset.or(short_form_reset);

        if session_percent.is_none() && clean_lower.contains("out of extra usage") {
            session_percent = Some(100.0);
        }

        // Build usage snapshot
        let session_used = session_percent.unwrap_or(0.0);
        let primary = RateWindow::with_details(
            session_used,
            Some(300), // 5 hour session window
            None,      // Could parse reset time
            session_reset,
        );

        let mut usage = UsageSnapshot::new(primary);

        if let Some(weekly_used) = weekly_percent {
            let secondary = RateWindow::with_details(
                weekly_used,
                Some(10080), // weekly (7 * 24 * 60)
                None,
                weekly_reset,
            );
            usage = usage.with_secondary(secondary);
        }

        if let Some(opus_used) = opus_percent {
            let model_specific = RateWindow::with_details(opus_used, Some(10080), None, None);
            usage = usage.with_model_specific(model_specific);
        }

        if let Some(method) = login_method {
            usage = usage.with_login_method(&method);
        } else {
            usage = usage.with_login_method("Claude (CLI)");
        }

        if let Some(email) = email {
            usage = usage.with_email(&email);
        }

        Ok(ProviderFetchResult::new(usage, "cli"))
    }
}

/// Try to find the claude CLI binary
fn which_claude() -> Option<std::path::PathBuf> {
    // Check common locations on Windows
    let possible_paths = [
        // In PATH
        which::which("claude").ok(),
        // AppData locations
        dirs::data_local_dir().map(|p| p.join("Programs").join("claude").join("claude.exe")),
        // npm global install
        dirs::data_dir().map(|p| p.join("npm").join("claude.cmd")),
    ];

    possible_paths.into_iter().flatten().find(|p| p.exists())
}

/// Detect the version of the claude CLI
fn detect_claude_version() -> Option<String> {
    let claude_path = which_claude()?;

    #[cfg(windows)]
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut cmd = std::process::Command::new(claude_path);
    cmd.args(["--version"]);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output().ok()?;

    if output.status.success() {
        let version_str = String::from_utf8_lossy(&output.stdout);
        extract_version(&version_str)
    } else {
        None
    }
}

/// Extract version number from a string like "claude 1.2.3"
fn extract_version(s: &str) -> Option<String> {
    let re = regex_lite::Regex::new(r"(\d+(?:\.\d+)+)").ok()?;
    re.find(s).map(|m| m.as_str().to_string())
}

/// Strip ANSI escape codes from text
fn strip_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1B' {
            // Skip CSI sequences: ESC[...letter
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            // Skip OSC sequences: ESC]...BEL
            } else if chars.peek() == Some(&']') {
                for next in chars.by_ref() {
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

/// Extract percentage near a label (e.g., "Current session")
/// Returns the percentage as "used" (not remaining)
fn extract_percent_near_label(text: &str, label: &str) -> Option<f64> {
    let label_lower = label.to_lowercase();
    let lines: Vec<&str> = text.lines().collect();

    // Find the line containing the label
    for (idx, line) in lines.iter().enumerate() {
        if line.to_lowercase().contains(&label_lower) {
            // Look in the next few lines for a percentage
            for next_line in lines.iter().skip(idx).take(12) {
                if let Some(pct) = parse_percent_line(next_line) {
                    return Some(pct);
                }
            }
        }
    }

    None
}

/// Parse a line containing "X% used" or "X% left"
/// Returns the percentage as used (converts "left" to used)
fn parse_percent_line(line: &str) -> Option<f64> {
    // Match patterns like "45% used" or "55% left"
    let re = Regex::new(r"(\d{1,3})\s*%\s*(used|left)").ok()?;

    if let Some(caps) = re.captures(&line.to_lowercase())
        && let Some(value_match) = caps.get(1)
        && let Some(kind_match) = caps.get(2)
    {
        let value: f64 = value_match.as_str().parse().ok()?;
        let kind = kind_match.as_str();

        // Convert to "used" percentage
        if kind == "left" {
            Some((100.0 - value).max(0.0))
        } else {
            Some(value.min(100.0))
        }
    } else {
        None
    }
}

/// Extract all percentages from text in order
fn extract_all_percents(text: &str) -> Vec<f64> {
    let re = match Regex::new(r"(\d{1,3})\s*%\s*(used|left)") {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let mut results = Vec::new();
    let lower = text.to_lowercase();

    for caps in re.captures_iter(&lower) {
        if let (Some(val_match), Some(kind_match)) = (caps.get(1), caps.get(2))
            && let Ok(val) = val_match.as_str().parse::<f64>()
        {
            let kind = kind_match.as_str();
            let used = if kind == "left" {
                (100.0 - val).max(0.0)
            } else {
                val.min(100.0)
            };
            results.push(used);
        }
    }

    results
}

/// Extract email address from text
fn extract_email(text: &str) -> Option<String> {
    // Try explicit patterns first
    let patterns = [
        r"Account:\s*([^\s@]+@[^\s@]+\.[^\s]+)",
        r"Email:\s*([^\s@]+@[^\s@]+\.[^\s]+)",
        r"([A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,})",
    ];

    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern)
            && let Some(caps) = re.captures(text)
            && let Some(m) = caps.get(1)
        {
            return Some(m.as_str().trim().to_string());
        }
    }

    None
}

/// Extract login method / plan name from text
fn extract_login_method(text: &str) -> Option<String> {
    // Look for explicit "Login method:" line
    if let Ok(re) = Regex::new(r"(?i)login\s+method:\s*(.+)")
        && let Some(caps) = re.captures(text)
        && let Some(m) = caps.get(1)
    {
        let method = m.as_str().trim();
        if !method.is_empty() {
            return Some(clean_plan_name(method));
        }
    }

    // Look for "Claude <plan>" patterns
    if let Ok(re) = Regex::new(r"(?i)(claude\s+(?:max|pro|ultra|team|free)[a-z0-9\s._-]*)")
        && let Some(caps) = re.captures(text)
        && let Some(m) = caps.get(1)
    {
        let plan = m.as_str().trim();
        if !plan.to_lowercase().contains("code") {
            return Some(clean_plan_name(plan));
        }
    }

    None
}

/// Extract reset description near a label
fn extract_reset_description(text: &str, label: &str) -> Option<String> {
    let label_lower = label.to_lowercase();
    let lines: Vec<&str> = text.lines().collect();

    for (idx, line) in lines.iter().enumerate() {
        if line.to_lowercase().contains(&label_lower) {
            // Look in the next few lines for "Resets"
            for next_line in lines.iter().skip(idx).take(14) {
                let lower = next_line.to_lowercase();
                if lower.contains("resets") {
                    // Extract the reset info
                    if let Some(pos) = lower.find("resets") {
                        let reset_part = &next_line[pos..];
                        return Some(reset_part.trim().to_string());
                    }
                }
            }
        }
    }

    None
}

/// Extract a "resets ..." suffix from a short single-line status.
fn extract_inline_reset_description(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let pos = lower.find("resets")?;
    Some(text[pos..].trim().to_string())
}

/// Clean up a plan name by removing ANSI codes and extra whitespace
fn clean_plan_name(text: &str) -> String {
    let cleaned = strip_ansi(text);
    // Remove bracketed codes like [22m
    let re = Regex::new(r"\[\d+m").unwrap_or_else(|_| Regex::new(".^").unwrap());
    let result = re.replace_all(&cleaned, "");
    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_current_cli_usage_screen() {
        let provider = ClaudeProvider::new();
        let output = r#"
Status   Config   Usage

  Current session
  ██████████████████████████████████████████████████ 100% used
  Resets 12pm (America/Bogota)

  Current week (all models)
  ████████████████████████▌                          49% used
  Resets Apr 3, 2pm (America/Bogota)

  Extra usage
  ██▍                                                4% used
  $3.31 / $70.00 spent · Resets Apr 1 (America/Bogota)
"#;

        let result = provider.parse_cli_output(output).expect("should parse");

        assert_eq!(result.source_label, "cli");
        assert_eq!(result.usage.primary.used_percent, 100.0);
        assert_eq!(
            result.usage.primary.reset_description.as_deref(),
            Some("Resets 12pm (America/Bogota)")
        );

        let weekly = result
            .usage
            .secondary
            .expect("weekly usage should be present");
        assert_eq!(weekly.used_percent, 49.0);
        assert_eq!(
            weekly.reset_description.as_deref(),
            Some("Resets Apr 3, 2pm (America/Bogota)")
        );
    }

    #[test]
    fn parses_exhausted_short_form_as_full_session_usage() {
        let provider = ClaudeProvider::new();
        let output = "You're out of extra usage · resets 12pm (America/Bogota)";

        let result = provider.parse_cli_output(output).expect("should parse");

        assert_eq!(result.usage.primary.used_percent, 100.0);
        assert_eq!(
            result.usage.primary.reset_description.as_deref(),
            Some("resets 12pm (America/Bogota)")
        );
    }
}
