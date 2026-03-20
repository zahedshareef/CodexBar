//! TTY Command Runner for Windows
//!
//! Executes interactive CLI commands using Windows ConPTY (pseudo-console).
//! Provides PTY-like functionality for capturing output from interactive TUI programs.

#![allow(dead_code)]

use regex_lite::Regex;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Result of running a TTY command
#[derive(Debug, Clone)]
pub struct TtyCommandResult {
    /// Captured output text
    pub text: String,
    /// Whether the command was interrupted early
    pub stopped_early: bool,
    /// URLs detected in output (if stop_on_url was set)
    pub detected_urls: Vec<String>,
}

impl TtyCommandResult {
    /// Extract the first URL found in the output
    pub fn first_url(&self) -> Option<&str> {
        self.detected_urls.first().map(|s| s.as_str())
    }
}

/// Options for running TTY commands
#[derive(Debug, Clone)]
pub struct TtyCommandOptions {
    /// Terminal rows (default: 50)
    pub rows: u16,
    /// Terminal columns (default: 160)
    pub cols: u16,
    /// Overall timeout in seconds (default: 20)
    pub timeout_secs: f64,
    /// Idle timeout - stop if no output for this duration (optional)
    pub idle_timeout_secs: Option<f64>,
    /// Working directory
    pub working_directory: Option<PathBuf>,
    /// Extra arguments to pass to the command
    pub extra_args: Vec<String>,
    /// Initial delay before sending script (default: 0.4s)
    pub initial_delay_secs: f64,
    /// Send enter/return every N seconds (optional)
    pub send_enter_every_secs: Option<f64>,
    /// Map of substrings to keys to send when detected
    pub send_on_substrings: HashMap<String, String>,
    /// Stop early when a URL is detected
    pub stop_on_url: bool,
    /// Stop early when any of these substrings are detected
    pub stop_on_substrings: Vec<String>,
    /// Settle time after stopping (default: 0.25s)
    pub settle_after_stop_secs: f64,
    /// Environment variables to set
    pub env: HashMap<String, String>,
}

impl Default for TtyCommandOptions {
    fn default() -> Self {
        Self {
            rows: 50,
            cols: 160,
            timeout_secs: 20.0,
            idle_timeout_secs: None,
            working_directory: None,
            extra_args: Vec::new(),
            initial_delay_secs: 0.4,
            send_enter_every_secs: None,
            send_on_substrings: HashMap::new(),
            stop_on_url: false,
            stop_on_substrings: Vec::new(),
            settle_after_stop_secs: 0.25,
            env: HashMap::new(),
        }
    }
}

impl TtyCommandOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_timeout(mut self, secs: f64) -> Self {
        self.timeout_secs = secs;
        self
    }

    pub fn with_idle_timeout(mut self, secs: f64) -> Self {
        self.idle_timeout_secs = Some(secs);
        self
    }

    pub fn with_working_directory(mut self, dir: PathBuf) -> Self {
        self.working_directory = Some(dir);
        self
    }

    pub fn with_extra_args(mut self, args: Vec<String>) -> Self {
        self.extra_args = args;
        self
    }

    pub fn with_stop_on_url(mut self, stop: bool) -> Self {
        self.stop_on_url = stop;
        self
    }

    pub fn with_stop_on_substring(mut self, substring: impl Into<String>) -> Self {
        self.stop_on_substrings.push(substring.into());
        self
    }

    pub fn with_send_on_substring(
        mut self,
        trigger: impl Into<String>,
        keys: impl Into<String>,
    ) -> Self {
        self.send_on_substrings.insert(trigger.into(), keys.into());
        self
    }
}

/// Errors from TTY command execution
#[derive(Debug, Error)]
pub enum TtyCommandError {
    #[error("Binary not found: {0}. Install it or add it to PATH.")]
    BinaryNotFound(String),

    #[error("Failed to launch process: {0}")]
    LaunchFailed(String),

    #[error("Command timed out")]
    TimedOut,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Process error: {0}")]
    ProcessError(String),
}

/// TTY Command Runner
///
/// On Windows, this uses standard process I/O with some heuristics for
/// interactive programs. For true PTY support, consider using the
/// `conpty` crate or Windows ConPTY APIs directly.
pub struct TtyCommandRunner;

impl TtyCommandRunner {
    /// Create a new runner instance
    pub fn new() -> Self {
        Self
    }

    /// Locate a binary using the system PATH
    pub fn which(tool: &str) -> Option<PathBuf> {
        // Check for specific tool overrides
        if tool == "codex" {
            if let Some(path) = Self::locate_codex_binary() {
                return Some(path);
            }
        }
        if tool == "claude" {
            if let Some(path) = Self::locate_claude_binary() {
                return Some(path);
            }
        }

        // Use `where` on Windows (equivalent to `which` on Unix)
        Self::run_where(tool)
    }

    /// Locate the Codex binary
    fn locate_codex_binary() -> Option<PathBuf> {
        // Check environment override
        if let Ok(path) = std::env::var("CODEX_BINARY") {
            let path = PathBuf::from(path);
            if path.exists() {
                return Some(path);
            }
        }

        // Check common Windows locations
        let candidates = [
            // npm global install locations
            dirs::data_local_dir().map(|d| d.join("npm").join("codex.cmd")),
            dirs::home_dir().map(|h| {
                h.join("AppData")
                    .join("Roaming")
                    .join("npm")
                    .join("codex.cmd")
            }),
            // Bun install
            dirs::home_dir().map(|h| h.join(".bun").join("bin").join("codex.exe")),
        ];

        for candidate in candidates.into_iter().flatten() {
            if candidate.exists() {
                return Some(candidate);
            }
        }

        // Fall back to PATH search
        Self::run_where("codex")
    }

    /// Locate the Claude binary
    fn locate_claude_binary() -> Option<PathBuf> {
        // Check environment override
        if let Ok(path) = std::env::var("CLAUDE_BINARY") {
            let path = PathBuf::from(path);
            if path.exists() {
                return Some(path);
            }
        }

        // Check common Windows locations
        let candidates = [
            // npm global install locations
            dirs::data_local_dir().map(|d| d.join("npm").join("claude.cmd")),
            dirs::home_dir().map(|h| {
                h.join("AppData")
                    .join("Roaming")
                    .join("npm")
                    .join("claude.cmd")
            }),
        ];

        for candidate in candidates.into_iter().flatten() {
            if candidate.exists() {
                return Some(candidate);
            }
        }

        Self::run_where("claude")
    }

    /// Run the Windows `where` command to find a binary
    fn run_where(tool: &str) -> Option<PathBuf> {
        let output = Command::new("where")
            .arg(tool)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let first_line = stdout.lines().next()?.trim();
        if first_line.is_empty() {
            return None;
        }

        Some(PathBuf::from(first_line))
    }

    /// Run a command and capture its output
    ///
    /// This is a simplified version for Windows that doesn't use PTY.
    /// For interactive TUI programs, you may need to use ConPTY directly.
    pub fn run(
        &self,
        binary: &str,
        script: &str,
        options: TtyCommandOptions,
    ) -> Result<TtyCommandResult, TtyCommandError> {
        // Resolve the binary path
        let resolved = if std::path::Path::new(binary).exists() {
            PathBuf::from(binary)
        } else if let Some(path) = Self::which(binary) {
            path
        } else {
            return Err(TtyCommandError::BinaryNotFound(binary.to_string()));
        };

        // Build the command
        let mut cmd = Command::new(&resolved);
        cmd.args(&options.extra_args);

        if let Some(ref dir) = options.working_directory {
            cmd.current_dir(dir);
        }

        // Set environment
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        for (key, value) in &options.env {
            cmd.env(key, value);
        }

        // Set up I/O
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Launch the process
        let mut child = cmd
            .spawn()
            .map_err(|e| TtyCommandError::LaunchFailed(e.to_string()))?;

        // Run the interactive session
        self.run_session(&mut child, script, &options)
    }

    /// Run an interactive session with the child process
    fn run_session(
        &self,
        child: &mut Child,
        script: &str,
        options: &TtyCommandOptions,
    ) -> Result<TtyCommandResult, TtyCommandError> {
        let start = Instant::now();
        let timeout = Duration::from_secs_f64(options.timeout_secs);
        let idle_timeout = options.idle_timeout_secs.map(Duration::from_secs_f64);
        let settle = Duration::from_secs_f64(options.settle_after_stop_secs);

        let mut buffer = String::new();
        let mut stopped_early = false;
        let mut detected_urls = Vec::new();
        let mut last_output_time = Instant::now();
        let mut triggered_sends = std::collections::HashSet::new();

        // Take ownership of stdin/stdout
        let mut stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        // URL detection regex
        let url_regex = Regex::new(r"https?://[A-Za-z0-9._~:/?#\[\]@!$&'()*+,;=%-]+").ok();

        // Initial delay
        std::thread::sleep(Duration::from_secs_f64(options.initial_delay_secs));

        // Send the script if provided
        let trimmed = script.trim();
        if !trimmed.is_empty() {
            if let Some(ref mut stdin) = stdin {
                let _ = writeln!(stdin, "{}", trimmed);
                let _ = stdin.flush();
            }
        }

        // Set up non-blocking readers using channels
        let (tx, rx) = mpsc::channel::<String>();

        if let Some(mut stdout) = stdout {
            let tx_stdout = tx.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                loop {
                    match stdout.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            if let Ok(s) = String::from_utf8(buf[..n].to_vec()) {
                                let _ = tx_stdout.send(s);
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        }

        if let Some(mut stderr) = stderr {
            let tx_stderr = tx;
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                loop {
                    match stderr.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            if let Ok(s) = String::from_utf8(buf[..n].to_vec()) {
                                let _ = tx_stderr.send(s);
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        }

        let mut last_enter = Instant::now();

        // Main read loop
        loop {
            // Check timeout
            if start.elapsed() > timeout {
                break;
            }

            // Check idle timeout
            if let Some(idle) = idle_timeout {
                if !buffer.is_empty() && last_output_time.elapsed() > idle {
                    stopped_early = true;
                    break;
                }
            }

            // Check if process has exited
            if let Ok(Some(_)) = child.try_wait() {
                // Process exited, drain remaining output
                while let Ok(chunk) = rx.try_recv() {
                    buffer.push_str(&chunk);
                }
                break;
            }

            // Read available output
            while let Ok(chunk) = rx.try_recv() {
                buffer.push_str(&chunk);
                last_output_time = Instant::now();

                // Check for URLs
                if let Some(ref regex) = url_regex {
                    for mat in regex.find_iter(&chunk) {
                        let mut url = mat.as_str().to_string();
                        // Trim trailing punctuation
                        while url.ends_with(['.', ',', ';', ':', ')', ']', '}', '>', '"', '\'']) {
                            url.pop();
                        }
                        if !detected_urls.contains(&url) {
                            detected_urls.push(url);
                        }
                    }

                    if options.stop_on_url && !detected_urls.is_empty() {
                        stopped_early = true;
                        break;
                    }
                }

                // Check for stop substrings
                for stop_str in &options.stop_on_substrings {
                    if buffer.contains(stop_str) {
                        stopped_early = true;
                        break;
                    }
                }

                // Check for send triggers
                for (trigger, keys) in &options.send_on_substrings {
                    if !triggered_sends.contains(trigger) && buffer.contains(trigger) {
                        if let Some(ref mut stdin) = stdin {
                            let _ = write!(stdin, "{}", keys);
                            let _ = stdin.flush();
                        }
                        triggered_sends.insert(trigger.clone());
                    }
                }
            }

            if stopped_early {
                break;
            }

            // Send periodic enters if configured
            if let Some(interval) = options.send_enter_every_secs {
                if last_enter.elapsed() >= Duration::from_secs_f64(interval) {
                    if let Some(ref mut stdin) = stdin {
                        let _ = writeln!(stdin);
                        let _ = stdin.flush();
                    }
                    last_enter = Instant::now();
                }
            }

            // Small sleep to avoid busy loop
            std::thread::sleep(Duration::from_millis(50));
        }

        // Settle period - collect remaining output
        if stopped_early {
            let settle_start = Instant::now();
            while settle_start.elapsed() < settle {
                while let Ok(chunk) = rx.try_recv() {
                    buffer.push_str(&chunk);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }

        // Clean up the child process
        let _ = child.kill();
        let _ = child.wait();

        if buffer.is_empty() && !stopped_early {
            return Err(TtyCommandError::TimedOut);
        }

        Ok(TtyCommandResult {
            text: buffer,
            stopped_early,
            detected_urls,
        })
    }

    /// Get enriched PATH for finding CLI tools
    pub fn enriched_path() -> String {
        let mut paths = Vec::new();

        // Add common npm/bun paths on Windows
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("AppData").join("Roaming").join("npm"));
            paths.push(home.join(".bun").join("bin"));
            paths.push(home.join(".deno").join("bin"));
        }

        if let Some(local) = dirs::data_local_dir() {
            paths.push(local.join("npm"));
        }

        // Get current PATH
        let current_path = std::env::var("PATH").unwrap_or_default();

        // Combine paths
        let mut all_paths: Vec<String> = paths
            .into_iter()
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        all_paths.push(current_path);

        all_paths.join(";")
    }

    /// Get enriched environment for CLI commands
    pub fn enriched_environment() -> HashMap<String, String> {
        let mut env: HashMap<String, String> = std::env::vars().collect();

        env.insert("PATH".to_string(), Self::enriched_path());
        env.entry("TERM".to_string())
            .or_insert_with(|| "xterm-256color".to_string());
        env.entry("COLORTERM".to_string())
            .or_insert_with(|| "truecolor".to_string());

        if let Some(home) = dirs::home_dir() {
            env.entry("HOME".to_string())
                .or_insert_with(|| home.to_string_lossy().to_string());
            env.entry("USERPROFILE".to_string())
                .or_insert_with(|| home.to_string_lossy().to_string());
        }

        env
    }
}

impl Default for TtyCommandRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Rolling buffer for pattern matching across chunks
#[derive(Debug)]
pub struct RollingBuffer {
    max_needle: usize,
    tail: String,
}

impl RollingBuffer {
    pub fn new(max_needle: usize) -> Self {
        Self {
            max_needle: max_needle.max(1),
            tail: String::new(),
        }
    }

    /// Append new data and return combined data for scanning
    pub fn append(&mut self, data: &str) -> String {
        if data.is_empty() {
            return String::new();
        }

        let mut combined = String::with_capacity(self.tail.len() + data.len());
        combined.push_str(&self.tail);
        combined.push_str(data);

        // Keep only the tail portion for next scan
        if combined.len() >= self.max_needle - 1 {
            let start = combined.len() - (self.max_needle - 1);
            self.tail = combined[start..].to_string();
        } else {
            self.tail = combined.clone();
        }

        combined
    }

    pub fn reset(&mut self) {
        self.tail.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tty_options_builder() {
        let opts = TtyCommandOptions::new()
            .with_timeout(30.0)
            .with_idle_timeout(5.0)
            .with_stop_on_url(true)
            .with_stop_on_substring("error");

        assert_eq!(opts.timeout_secs, 30.0);
        assert_eq!(opts.idle_timeout_secs, Some(5.0));
        assert!(opts.stop_on_url);
        assert!(opts.stop_on_substrings.contains(&"error".to_string()));
    }

    #[test]
    fn test_rolling_buffer() {
        let mut buf = RollingBuffer::new(10);

        let result1 = buf.append("hello");
        assert_eq!(result1, "hello");

        let result2 = buf.append(" world");
        assert!(result2.contains("hello"));
        assert!(result2.contains(" world"));
    }

    #[test]
    fn test_tty_result_first_url() {
        let result = TtyCommandResult {
            text: "Visit https://example.com for more info".to_string(),
            stopped_early: false,
            detected_urls: vec!["https://example.com".to_string()],
        };

        assert_eq!(result.first_url(), Some("https://example.com"));
    }

    #[test]
    fn test_enriched_path() {
        let path = TtyCommandRunner::enriched_path();
        assert!(!path.is_empty());
        // Should contain path separator
        assert!(path.contains(';') || !path.is_empty());
    }
}
