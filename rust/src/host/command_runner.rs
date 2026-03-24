//! Command Runner
//!
//! Executes CLI commands with output capture.
//! On Windows, uses standard process spawning with output capture.
//! Designed for running interactive CLI tools like `codex` and `claude`.

#![allow(dead_code)]

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Command runner configuration
#[derive(Debug, Clone)]
pub struct CommandOptions {
    /// Number of rows for terminal (advisory)
    pub rows: u16,
    /// Number of columns for terminal (advisory)
    pub cols: u16,
    /// Command timeout
    pub timeout: Duration,
    /// Stop early after this idle time
    pub idle_timeout: Option<Duration>,
    /// Working directory
    pub working_directory: Option<PathBuf>,
    /// Extra arguments to pass
    pub extra_args: Vec<String>,
    /// Initial delay before sending input
    pub initial_delay: Duration,
    /// Send enter key every N seconds
    pub send_enter_every: Option<Duration>,
    /// Send specific input when substring is seen
    pub send_on_substrings: HashMap<String, String>,
    /// Stop when URL is detected
    pub stop_on_url: bool,
    /// Stop when any of these substrings are seen
    pub stop_on_substrings: Vec<String>,
    /// Time to wait after stop condition before returning
    pub settle_after_stop: Duration,
}

impl Default for CommandOptions {
    fn default() -> Self {
        Self {
            rows: 50,
            cols: 160,
            timeout: Duration::from_secs(20),
            idle_timeout: None,
            working_directory: None,
            extra_args: Vec::new(),
            initial_delay: Duration::from_millis(400),
            send_enter_every: None,
            send_on_substrings: HashMap::new(),
            stop_on_url: false,
            stop_on_substrings: Vec::new(),
            settle_after_stop: Duration::from_millis(250),
        }
    }
}

/// Result of running a command
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Captured output text
    pub text: String,
    /// Whether the command timed out
    pub timed_out: bool,
    /// Exit code if available
    pub exit_code: Option<i32>,
}

/// Command runner errors
#[derive(Debug, Clone)]
pub enum CommandError {
    /// Binary not found in PATH
    BinaryNotFound(String),
    /// Failed to launch process
    LaunchFailed(String),
    /// Command timed out
    TimedOut,
    /// IO error
    IoError(String),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::BinaryNotFound(bin) => {
                write!(f, "Binary '{}' not found. Install it or add to PATH.", bin)
            }
            CommandError::LaunchFailed(msg) => write!(f, "Failed to launch process: {}", msg),
            CommandError::TimedOut => write!(f, "Command timed out"),
            CommandError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for CommandError {}

/// Command runner for executing CLI tools
pub struct CommandRunner {
    /// Environment variables to add
    env_additions: HashMap<String, String>,
}

impl CommandRunner {
    pub fn new() -> Self {
        Self {
            env_additions: HashMap::new(),
        }
    }

    /// Add an environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_additions.insert(key.into(), value.into());
        self
    }

    /// Find a binary in PATH
    pub fn which(binary: &str) -> Option<PathBuf> {
        which::which(binary).ok()
    }

    /// Run a command and capture output
    pub fn run(
        &self,
        binary: &str,
        input: Option<&str>,
        options: &CommandOptions,
    ) -> Result<CommandResult, CommandError> {
        // Find the binary
        let binary_path = if std::path::Path::new(binary).exists() {
            PathBuf::from(binary)
        } else {
            Self::which(binary).ok_or_else(|| CommandError::BinaryNotFound(binary.to_string()))?
        };

        // Build the command
        let mut cmd = Command::new(&binary_path);

        // Add extra args
        for arg in &options.extra_args {
            cmd.arg(arg);
        }

        // Set working directory
        if let Some(dir) = &options.working_directory {
            cmd.current_dir(dir);
        }

        // Set up environment
        let mut env = std::env::vars().collect::<HashMap<_, _>>();
        for (k, v) in &self.env_additions {
            env.insert(k.clone(), v.clone());
        }

        // Set terminal environment
        env.insert("TERM".to_string(), "xterm-256color".to_string());
        env.insert("COLORTERM".to_string(), "truecolor".to_string());

        for (k, v) in &env {
            cmd.env(k, v);
        }

        // Set up stdio
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Hide console window on Windows
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        // Spawn the process
        let mut child = cmd
            .spawn()
            .map_err(|e| CommandError::LaunchFailed(e.to_string()))?;

        let start = Instant::now();
        let deadline = start + options.timeout;

        // Handle input if provided
        if let Some(input_text) = input {
            if let Some(stdin) = child.stdin.take() {
                use std::io::Write;
                let mut stdin = stdin;
                // Wait for initial delay
                std::thread::sleep(options.initial_delay);
                // Send input
                let _ = stdin.write_all(input_text.as_bytes());
                let _ = stdin.write_all(b"\n");
                let _ = stdin.flush();
            }
        }

        // Capture output
        let output = self.capture_output(&mut child, options, deadline)?;

        // Wait for process to finish (with timeout)
        let exit_code = match child.try_wait() {
            Ok(Some(status)) => status.code(),
            Ok(None) => {
                // Process still running, kill it
                let _ = child.kill();
                let _ = child.wait();
                None
            }
            Err(_) => None,
        };

        let timed_out = Instant::now() >= deadline && output.is_empty();

        Ok(CommandResult {
            text: output,
            timed_out,
            exit_code,
        })
    }

    /// Capture output from a running process
    fn capture_output(
        &self,
        child: &mut Child,
        options: &CommandOptions,
        deadline: Instant,
    ) -> Result<String, CommandError> {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let mut output = String::new();
        #[allow(unused_assignments)]
        let mut last_output_time = Instant::now();

        // Read stdout in a separate thread
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| CommandError::IoError("Failed to capture stdout".to_string()))?;

        let reader = BufReader::new(stdout);

        for line_result in reader.lines() {
            if stop_flag.load(Ordering::SeqCst) {
                break;
            }

            if Instant::now() >= deadline {
                break;
            }

            match line_result {
                Ok(line) => {
                    output.push_str(&line);
                    output.push('\n');
                    last_output_time = Instant::now();

                    // Check stop conditions
                    if options.stop_on_url && (line.contains("https://") || line.contains("http://")) {
                        std::thread::sleep(options.settle_after_stop);
                        break;
                    }

                    for stop_substr in &options.stop_on_substrings {
                        if line.contains(stop_substr) {
                            std::thread::sleep(options.settle_after_stop);
                            stop_flag.store(true, Ordering::SeqCst);
                            break;
                        }
                    }
                }
                Err(_) => break,
            }

            // Check idle timeout
            if let Some(idle_timeout) = options.idle_timeout {
                if last_output_time.elapsed() > idle_timeout {
                    break;
                }
            }
        }

        Ok(output)
    }

    /// Run a command asynchronously
    pub async fn run_async(
        &self,
        binary: &str,
        input: Option<&str>,
        options: &CommandOptions,
    ) -> Result<CommandResult, CommandError> {
        let binary = binary.to_string();
        let input = input.map(|s| s.to_string());
        let options = options.clone();
        let env = self.env_additions.clone();

        tokio::task::spawn_blocking(move || {
            let runner = CommandRunner {
                env_additions: env,
            };
            runner.run(&binary, input.as_deref(), &options)
        })
        .await
        .map_err(|e| CommandError::LaunchFailed(e.to_string()))?
    }
}

impl Default for CommandRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Rolling buffer for substring matching across chunk boundaries
pub struct RollingBuffer {
    max_needle_len: usize,
    tail: Vec<u8>,
}

impl RollingBuffer {
    pub fn new(max_needle_len: usize) -> Self {
        Self {
            max_needle_len,
            tail: Vec::with_capacity(max_needle_len),
        }
    }

    /// Append data and return the combined buffer for searching
    pub fn append(&mut self, data: &[u8]) -> Vec<u8> {
        if data.is_empty() {
            return Vec::new();
        }

        let mut combined = Vec::with_capacity(self.tail.len() + data.len());
        combined.extend_from_slice(&self.tail);
        combined.extend_from_slice(data);

        // Keep only the tail for next search
        if self.max_needle_len > 1 && combined.len() >= self.max_needle_len - 1 {
            self.tail = combined[combined.len() - (self.max_needle_len - 1)..].to_vec();
        } else {
            self.tail = combined.clone();
        }

        combined
    }

    /// Reset the buffer
    pub fn reset(&mut self) {
        self.tail.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_options_default() {
        let opts = CommandOptions::default();
        assert_eq!(opts.rows, 50);
        assert_eq!(opts.cols, 160);
        assert_eq!(opts.timeout, Duration::from_secs(20));
    }

    #[test]
    fn test_rolling_buffer() {
        let mut buf = RollingBuffer::new(5);

        let result = buf.append(b"hello");
        assert_eq!(result, b"hello");

        let result = buf.append(b" world");
        // Should include tail from previous
        assert!(result.len() > 6);
    }

    #[test]
    fn test_command_runner_new() {
        let runner = CommandRunner::new();
        assert!(runner.env_additions.is_empty());
    }

    #[test]
    fn test_command_runner_with_env() {
        let runner = CommandRunner::new()
            .with_env("FOO", "bar")
            .with_env("BAZ", "qux");

        assert_eq!(runner.env_additions.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(runner.env_additions.get("BAZ"), Some(&"qux".to_string()));
    }

    #[test]
    fn test_error_display() {
        let err = CommandError::BinaryNotFound("codex".to_string());
        assert!(err.to_string().contains("codex"));

        let err = CommandError::TimedOut;
        assert!(err.to_string().contains("timed out"));
    }
}
