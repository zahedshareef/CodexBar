//! Local cost-usage scanner for Codex and Claude
//!
//! Scans local JSONL log files to aggregate token usage and calculate costs

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

/// Cost summary from scanning local logs
#[derive(Debug, Clone, Default)]
pub struct CostSummary {
    /// Total cost in USD for the period
    pub total_cost_usd: f64,
    /// Total input tokens
    pub input_tokens: u64,
    /// Total output tokens
    pub output_tokens: u64,
    /// Total cached input tokens
    pub cached_tokens: u64,
    /// Number of sessions/conversations scanned
    pub sessions_count: u32,
    /// Cost breakdown by model
    pub by_model: HashMap<String, f64>,
    /// Period start date
    pub period_start: Option<NaiveDate>,
    /// Period end date
    pub period_end: Option<NaiveDate>,
}

impl CostSummary {
    pub fn format_total(&self) -> String {
        format!("${:.2}", self.total_cost_usd)
    }
}

/// Codex token pricing (per 1M tokens, as of 2024)
struct CodexPricing;

impl CodexPricing {
    fn cost_usd(model: &str, input: u64, cached: u64, output: u64) -> f64 {
        // Default to GPT-4o pricing
        let (input_price, cached_price, output_price) = match model.to_lowercase().as_str() {
            m if m.contains("gpt-4o-mini") => (0.15, 0.075, 0.60),
            m if m.contains("gpt-4o") => (2.50, 1.25, 10.00),
            m if m.contains("gpt-4-turbo") => (10.00, 5.00, 30.00),
            m if m.contains("gpt-4") => (30.00, 15.00, 60.00),
            m if m.contains("o1-mini") => (3.00, 1.50, 12.00),
            m if m.contains("o1") => (15.00, 7.50, 60.00),
            _ => (2.50, 1.25, 10.00), // Default to GPT-4o
        };

        let input_cost = (input as f64 / 1_000_000.0) * input_price;
        let cached_cost = (cached as f64 / 1_000_000.0) * cached_price;
        let output_cost = (output as f64 / 1_000_000.0) * output_price;

        input_cost + cached_cost + output_cost
    }
}

/// Claude token pricing (per 1M tokens, as of 2024)
struct ClaudePricing;

impl ClaudePricing {
    fn cost_usd(model: &str, input: u64, cache_create: u64, cache_read: u64, output: u64) -> f64 {
        let (input_price, cache_create_price, cache_read_price, output_price) =
            match model.to_lowercase().as_str() {
                m if m.contains("opus") => (15.00, 18.75, 1.50, 75.00),
                m if m.contains("sonnet") => (3.00, 3.75, 0.30, 15.00),
                m if m.contains("haiku") => (0.25, 0.30, 0.03, 1.25),
                _ => (3.00, 3.75, 0.30, 15.00), // Default to Sonnet
            };

        let input_cost = (input as f64 / 1_000_000.0) * input_price;
        let cache_create_cost = (cache_create as f64 / 1_000_000.0) * cache_create_price;
        let cache_read_cost = (cache_read as f64 / 1_000_000.0) * cache_read_price;
        let output_cost = (output as f64 / 1_000_000.0) * output_price;

        input_cost + cache_create_cost + cache_read_cost + output_cost
    }
}

/// JSONL event structures for Codex
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct CodexEvent {
    #[serde(rename = "type")]
    event_type: Option<String>,
    event_msg: Option<CodexEventMsg>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct CodexEventMsg {
    #[serde(rename = "type")]
    msg_type: Option<String>,
    input_tokens: Option<u64>,
    cached_input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

/// JSONL event structures for Claude
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ClaudeEvent {
    #[serde(rename = "type")]
    event_type: Option<String>,
    message: Option<ClaudeMessage>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ClaudeMessage {
    model: Option<String>,
    usage: Option<ClaudeUsage>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ClaudeUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
}

/// Cost usage scanner
pub struct CostScanner {
    days: u32,
}

impl CostScanner {
    /// Create a new scanner for the last N days
    pub fn new(days: u32) -> Self {
        Self { days }
    }

    /// Scan Codex local logs
    pub fn scan_codex(&self) -> CostSummary {
        let sessions_dir = self.get_codex_sessions_dir();
        if !sessions_dir.exists() {
            return CostSummary::default();
        }

        let mut summary = CostSummary::default();
        let today = Utc::now().date_naive();
        let start_date = today - Duration::days(self.days as i64);

        summary.period_start = Some(start_date);
        summary.period_end = Some(today);

        // Iterate through date-based directory structure
        for days_ago in 0..self.days {
            let date = today - Duration::days(days_ago as i64);
            let year = date.format("%Y").to_string();
            let month = date.format("%m").to_string();
            let day = date.format("%d").to_string();

            let day_dir = sessions_dir.join(&year).join(&month).join(&day);
            if !day_dir.exists() {
                continue;
            }

            if let Ok(entries) = fs::read_dir(&day_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "jsonl") {
                        self.parse_codex_file(&path, &mut summary);
                    }
                }
            }
        }

        summary
    }

    /// Scan Claude local logs
    pub fn scan_claude(&self) -> CostSummary {
        let projects_dir = self.get_claude_projects_dir();
        if !projects_dir.exists() {
            return CostSummary::default();
        }

        let mut summary = CostSummary::default();
        let today = Utc::now().date_naive();
        let start_date = today - Duration::days(self.days as i64);
        let cutoff = Utc::now() - Duration::days(self.days as i64);

        summary.period_start = Some(start_date);
        summary.period_end = Some(today);

        // Walk through projects directory
        self.scan_claude_dir(&projects_dir, &cutoff, &mut summary);

        summary
    }

    fn get_codex_sessions_dir(&self) -> PathBuf {
        if let Ok(codex_home) = std::env::var("CODEX_HOME") {
            let trimmed = codex_home.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed).join("sessions");
            }
        }

        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".codex")
            .join("sessions")
    }

    fn get_claude_projects_dir(&self) -> PathBuf {
        if let Ok(claude_config) = std::env::var("CLAUDE_CONFIG_DIR") {
            let trimmed = claude_config.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed).join("projects");
            }
        }

        // Try ~/.claude/projects first
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let claude_dir = home.join(".claude").join("projects");
        if claude_dir.exists() {
            return claude_dir;
        }

        // Fallback to ~/.config/claude/projects
        home.join(".config").join("claude").join("projects")
    }

    fn parse_codex_file(&self, path: &PathBuf, summary: &mut CostSummary) {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return,
        };

        let reader = BufReader::new(file);
        let mut current_model = String::from("gpt-4o");
        let mut session_cost = 0.0;
        let mut has_tokens = false;

        for line in reader.lines().map_while(Result::ok) {
            if let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) {
                // Check for model in turn_context
                if let Some(model) = event.get("model").and_then(|m| m.as_str()) {
                    current_model = model.to_string();
                }

                // Check for token_count events
                if let Some(event_msg) = event.get("event_msg") {
                    if event_msg.get("type").and_then(|t| t.as_str()) == Some("token_count") {
                        let input = event_msg
                            .get("input_tokens")
                            .and_then(|t| t.as_u64())
                            .unwrap_or(0);
                        let cached = event_msg
                            .get("cached_input_tokens")
                            .and_then(|t| t.as_u64())
                            .unwrap_or(0);
                        let output = event_msg
                            .get("output_tokens")
                            .and_then(|t| t.as_u64())
                            .unwrap_or(0);

                        summary.input_tokens += input;
                        summary.cached_tokens += cached;
                        summary.output_tokens += output;

                        let cost = CodexPricing::cost_usd(&current_model, input, cached, output);
                        session_cost += cost;
                        has_tokens = true;

                        *summary.by_model.entry(current_model.clone()).or_insert(0.0) += cost;
                    }
                }
            }
        }

        if has_tokens {
            summary.total_cost_usd += session_cost;
            summary.sessions_count += 1;
        }
    }

    fn scan_claude_dir(&self, dir: &PathBuf, cutoff: &DateTime<Utc>, summary: &mut CostSummary) {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                self.scan_claude_dir(&path, cutoff, summary);
            } else if path.extension().is_some_and(|e| e == "jsonl") {
                // Check file modification time
                if let Ok(metadata) = fs::metadata(&path) {
                    if let Ok(modified) = metadata.modified() {
                        let modified_dt: DateTime<Utc> = modified.into();
                        if modified_dt >= *cutoff {
                            self.parse_claude_file(&path, summary);
                        }
                    }
                }
            }
        }
    }

    fn parse_claude_file(&self, path: &PathBuf, summary: &mut CostSummary) {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return,
        };

        let reader = BufReader::new(file);
        let mut session_cost = 0.0;
        let mut has_tokens = false;

        for line in reader.lines().map_while(Result::ok) {
            if let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) {
                // Look for assistant messages with usage
                if event.get("type").and_then(|t| t.as_str()) == Some("assistant") {
                    if let Some(message) = event.get("message") {
                        let model = message
                            .get("model")
                            .and_then(|m| m.as_str())
                            .unwrap_or("claude-3-5-sonnet");

                        if let Some(usage) = message.get("usage") {
                            let input = usage
                                .get("input_tokens")
                                .and_then(|t| t.as_u64())
                                .unwrap_or(0);
                            let output = usage
                                .get("output_tokens")
                                .and_then(|t| t.as_u64())
                                .unwrap_or(0);
                            let cache_create = usage
                                .get("cache_creation_input_tokens")
                                .and_then(|t| t.as_u64())
                                .unwrap_or(0);
                            let cache_read = usage
                                .get("cache_read_input_tokens")
                                .and_then(|t| t.as_u64())
                                .unwrap_or(0);

                            summary.input_tokens += input;
                            summary.output_tokens += output;
                            summary.cached_tokens += cache_create + cache_read;

                            let cost = ClaudePricing::cost_usd(
                                model,
                                input,
                                cache_create,
                                cache_read,
                                output,
                            );
                            session_cost += cost;
                            has_tokens = true;

                            *summary.by_model.entry(model.to_string()).or_insert(0.0) += cost;
                        }
                    }
                }
            }
        }

        if has_tokens {
            summary.total_cost_usd += session_cost;
            summary.sessions_count += 1;
        }
    }
}

/// Check if any cost usage sources are available
#[allow(dead_code)]
pub fn has_cost_usage_sources() -> bool {
    let scanner = CostScanner::new(1);
    scanner.get_codex_sessions_dir().exists() || scanner.get_claude_projects_dir().exists()
}

/// Get daily cost history for the last N days
/// Returns Vec of (date_string, cost_usd) sorted by date
pub fn get_daily_cost_history(provider: &str, days: u32) -> Vec<(String, f64)> {
    let scanner = CostScanner::new(days);
    let today = Utc::now().date_naive();
    let mut daily_costs: HashMap<String, f64> = HashMap::new();

    // Initialize all days with 0
    for days_ago in 0..days {
        let date = today - Duration::days(days_ago as i64);
        let date_str = date.format("%Y-%m-%d").to_string();
        daily_costs.insert(date_str, 0.0);
    }

    match provider {
        "codex" => {
            // Scan Codex logs by day
            let sessions_dir = scanner.get_codex_sessions_dir();
            if sessions_dir.exists() {
                for days_ago in 0..days {
                    let date = today - Duration::days(days_ago as i64);
                    let date_str = date.format("%Y-%m-%d").to_string();
                    let year = date.format("%Y").to_string();
                    let month = date.format("%m").to_string();
                    let day = date.format("%d").to_string();

                    let day_dir = sessions_dir.join(&year).join(&month).join(&day);
                    if day_dir.exists() {
                        let mut day_cost = 0.0;
                        if let Ok(entries) = fs::read_dir(&day_dir) {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                if path.extension().is_some_and(|e| e == "jsonl") {
                                    day_cost += scan_codex_file_cost(&path);
                                }
                            }
                        }
                        daily_costs.insert(date_str, day_cost);
                    }
                }
            }
        }
        "claude" => {
            // For Claude, we need to check file modification times
            // This is more complex, so we'll approximate using the summary for now
            let summary = scanner.scan_claude();
            if summary.total_cost_usd > 0.0 && days > 0 {
                // Distribute evenly for now (TODO: actual daily breakdown)
                let daily = summary.total_cost_usd / days as f64;
                for (_, cost) in daily_costs.iter_mut() {
                    *cost = daily;
                }
            }
        }
        _ => {}
    }

    // Convert to sorted vector
    let mut result: Vec<(String, f64)> = daily_costs.into_iter().collect();
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

/// Scan a single Codex file and return its cost
fn scan_codex_file_cost(path: &PathBuf) -> f64 {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return 0.0,
    };

    let reader = BufReader::new(file);
    let mut current_model = String::from("gpt-4o");
    let mut total_cost = 0.0;

    for line in reader.lines().map_while(Result::ok) {
        if let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) {
            if let Some(model) = event.get("model").and_then(|m| m.as_str()) {
                current_model = model.to_string();
            }

            if let Some(event_msg) = event.get("event_msg") {
                if event_msg.get("type").and_then(|t| t.as_str()) == Some("token_count") {
                    let input = event_msg
                        .get("input_tokens")
                        .and_then(|t| t.as_u64())
                        .unwrap_or(0);
                    let cached = event_msg
                        .get("cached_input_tokens")
                        .and_then(|t| t.as_u64())
                        .unwrap_or(0);
                    let output = event_msg
                        .get("output_tokens")
                        .and_then(|t| t.as_u64())
                        .unwrap_or(0);

                    total_cost += CodexPricing::cost_usd(&current_model, input, cached, output);
                }
            }
        }
    }

    total_cost
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codex_pricing() {
        // Test GPT-4o pricing: $2.50/1M input, $10/1M output
        let cost = CodexPricing::cost_usd("gpt-4o", 1_000_000, 0, 1_000_000);
        assert!((cost - 12.50).abs() < 0.01);
    }

    #[test]
    fn test_claude_pricing() {
        // Test Sonnet pricing: $3/1M input, $15/1M output
        let cost = ClaudePricing::cost_usd("claude-3-5-sonnet", 1_000_000, 0, 0, 1_000_000);
        assert!((cost - 18.0).abs() < 0.01);
    }
}
