//! Login flow runners for various providers
//!
//! Runs CLI login commands and captures output/URLs

#![allow(dead_code)]

use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use regex_lite::Regex;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Result of a login attempt
#[derive(Debug, Clone)]
pub struct LoginResult {
    pub outcome: LoginOutcome,
    pub output: String,
    pub auth_link: Option<String>,
}

/// Outcome of login attempt
#[derive(Debug, Clone)]
pub enum LoginOutcome {
    Success,
    TimedOut,
    Failed { status: i32 },
    MissingBinary,
    LaunchFailed(String),
}

/// Phase of the login process
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoginPhase {
    Idle,
    Requesting,
    WaitingBrowser,
    Complete,
}

/// Run Claude CLI login
pub async fn run_claude_login<F>(timeout_secs: u64, on_phase: F) -> LoginResult
where
    F: Fn(LoginPhase) + Send + 'static,
{
    run_cli_login("claude", &["/login"], timeout_secs, on_phase, &[
        "Successfully logged in",
        "Login successful",
        "Logged in successfully",
    ]).await
}

/// Run Codex CLI login
pub async fn run_codex_login<F>(timeout_secs: u64, on_phase: F) -> LoginResult
where
    F: Fn(LoginPhase) + Send + 'static,
{
    run_cli_login("codex", &["auth", "login"], timeout_secs, on_phase, &[
        "Successfully logged in",
        "Login successful",
        "Logged in successfully",
    ]).await
}

/// Run Gemini/gcloud login
pub async fn run_gemini_login<F>(timeout_secs: u64, on_phase: F) -> LoginResult
where
    F: Fn(LoginPhase) + Send + 'static,
{
    run_cli_login("gcloud", &["auth", "login"], timeout_secs, on_phase, &[
        "You are now logged in",
        "Credentials saved",
    ]).await
}

/// Run Copilot/GitHub device flow login
pub async fn run_copilot_login<F>(timeout_secs: u64, on_phase: F) -> LoginResult
where
    F: Fn(LoginPhase) + Send + 'static,
{
    run_cli_login("gh", &["auth", "login", "-w"], timeout_secs, on_phase, &[
        "Logged in as",
        "Authentication complete",
    ]).await
}

/// Generic CLI login runner
async fn run_cli_login<F>(
    binary: &str,
    args: &[&str],
    timeout_secs: u64,
    on_phase: F,
    success_markers: &[&str],
) -> LoginResult
where
    F: Fn(LoginPhase) + Send + 'static,
{
    // Check if binary exists
    let binary_path = match which::which(binary) {
        Ok(p) => p,
        Err(_) => {
            return LoginResult {
                outcome: LoginOutcome::MissingBinary,
                output: format!("{} not found in PATH", binary),
                auth_link: None,
            };
        }
    };

    on_phase(LoginPhase::Requesting);

    // Spawn process
    #[cfg(windows)]
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut cmd = Command::new(&binary_path);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let mut child = match cmd.spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return LoginResult {
                outcome: LoginOutcome::LaunchFailed(e.to_string()),
                output: String::new(),
                auth_link: None,
            };
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let mut output = String::new();
    let mut auth_link = None;
    let url_regex = Regex::new(r"https?://[A-Za-z0-9._~:/?#\[\]@!$&'()*+,;=%-]+").unwrap();

    // Read output with timeout
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);

    // Read stdout
    if let Some(stdout) = stdout {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            output.push_str(&line);
            output.push('\n');

            // Check for URL (indicates browser login)
            if auth_link.is_none() {
                if let Some(m) = url_regex.find(&line) {
                    auth_link = Some(m.as_str().to_string());
                    on_phase(LoginPhase::WaitingBrowser);

                    // Open the URL in browser
                    let _ = open::that(m.as_str());
                }
            }

            // Check for success markers
            for marker in success_markers {
                if line.contains(marker) {
                    on_phase(LoginPhase::Complete);
                    let _ = child.kill();
                    return LoginResult {
                        outcome: LoginOutcome::Success,
                        output,
                        auth_link,
                    };
                }
            }

            // Check timeout
            if start.elapsed() > timeout {
                let _ = child.kill();
                return LoginResult {
                    outcome: LoginOutcome::TimedOut,
                    output,
                    auth_link,
                };
            }
        }
    }

    // Read stderr too
    if let Some(stderr) = stderr {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            output.push_str(&line);
            output.push('\n');

            if auth_link.is_none() {
                if let Some(m) = url_regex.find(&line) {
                    auth_link = Some(m.as_str().to_string());
                    on_phase(LoginPhase::WaitingBrowser);
                    let _ = open::that(m.as_str());
                }
            }

            for marker in success_markers {
                if line.contains(marker) {
                    on_phase(LoginPhase::Complete);
                    let _ = child.kill();
                    return LoginResult {
                        outcome: LoginOutcome::Success,
                        output,
                        auth_link,
                    };
                }
            }

            if start.elapsed() > timeout {
                let _ = child.kill();
                return LoginResult {
                    outcome: LoginOutcome::TimedOut,
                    output,
                    auth_link,
                };
            }
        }
    }

    // Wait for process to complete
    match child.wait() {
        Ok(status) => {
            if status.success() {
                on_phase(LoginPhase::Complete);
                LoginResult {
                    outcome: LoginOutcome::Success,
                    output,
                    auth_link,
                }
            } else {
                LoginResult {
                    outcome: LoginOutcome::Failed {
                        status: status.code().unwrap_or(-1),
                    },
                    output,
                    auth_link,
                }
            }
        }
        Err(e) => LoginResult {
            outcome: LoginOutcome::LaunchFailed(e.to_string()),
            output,
            auth_link,
        },
    }
}

/// Open a URL in the default browser
pub fn open_auth_url(url: &str) -> anyhow::Result<()> {
    open::that(url)?;
    Ok(())
}
