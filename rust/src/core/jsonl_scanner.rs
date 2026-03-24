//! JSONL Scanner with Caching
//!
//! Incremental log file parsing for Codex and Claude session logs.
//! Supports file-level caching to avoid re-parsing unchanged files.

#![allow(dead_code)]

use crate::core::{CostUsagePricing, ProviderId};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Cache for scanned file data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CostUsageCache {
    /// Last scan timestamp in milliseconds
    pub last_scan_unix_ms: i64,
    /// Per-file usage data
    pub files: HashMap<String, CostUsageFileUsage>,
    /// Aggregated daily data: day_key -> model -> [input, cached, output]
    pub days: HashMap<String, HashMap<String, Vec<i32>>>,
}

/// Per-file usage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostUsageFileUsage {
    /// File modification time in milliseconds
    pub mtime_unix_ms: i64,
    /// File size in bytes
    pub size: i64,
    /// Daily usage data extracted from this file
    pub days: HashMap<String, HashMap<String, Vec<i32>>>,
    /// Bytes parsed so far (for incremental parsing)
    pub parsed_bytes: Option<i64>,
    /// Last model seen (for delta calculations)
    pub last_model: Option<String>,
    /// Last token totals (for delta calculations)
    pub last_totals: Option<CodexTotals>,
}

/// Running totals for Codex token counting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexTotals {
    pub input: i32,
    pub cached: i32,
    pub output: i32,
}

/// Result of parsing a Codex file
#[derive(Debug)]
pub struct CodexParseResult {
    /// Daily usage: day_key -> model -> [input, cached, output]
    pub days: HashMap<String, HashMap<String, Vec<i32>>>,
    /// Bytes parsed
    pub parsed_bytes: i64,
    /// Last model seen
    pub last_model: Option<String>,
    /// Last totals seen
    pub last_totals: Option<CodexTotals>,
}

/// Day range for scanning
pub struct CostUsageDayRange {
    pub since_key: String,
    pub until_key: String,
    pub scan_since_key: String,
    pub scan_until_key: String,
}

impl CostUsageDayRange {
    pub fn new(since: NaiveDate, until: NaiveDate) -> Self {
        let since_minus_one = since - chrono::Duration::days(1);
        let until_plus_one = until + chrono::Duration::days(1);

        Self {
            since_key: Self::day_key(since),
            until_key: Self::day_key(until),
            scan_since_key: Self::day_key(since_minus_one),
            scan_until_key: Self::day_key(until_plus_one),
        }
    }

    pub fn day_key(date: NaiveDate) -> String {
        date.format("%Y-%m-%d").to_string()
    }

    pub fn is_in_range(day_key: &str, since: &str, until: &str) -> bool {
        day_key >= since && day_key <= until
    }

    pub fn parse_day_key(key: &str) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(key, "%Y-%m-%d").ok()
    }
}

/// JSONL Scanner for cost/usage logs
pub struct JsonlScanner;

impl JsonlScanner {
    /// Get default Codex sessions root directory
    pub fn default_codex_sessions_root() -> Option<PathBuf> {
        // Check CODEX_HOME environment variable
        if let Ok(home) = std::env::var("CODEX_HOME") {
            let home = home.trim();
            if !home.is_empty() {
                return Some(PathBuf::from(home).join("sessions"));
            }
        }

        // Default to ~/.codex/sessions
        dirs::home_dir().map(|h| h.join(".codex").join("sessions"))
    }

    /// Get default Claude projects roots
    pub fn default_claude_projects_roots() -> Vec<PathBuf> {
        let mut roots = Vec::new();

        // Check CLAUDE_CONFIG_DIR
        if let Ok(config_dir) = std::env::var("CLAUDE_CONFIG_DIR") {
            let path = PathBuf::from(config_dir.trim()).join("projects");
            if path.exists() {
                roots.push(path);
            }
        }

        // Default locations
        if let Some(home) = dirs::home_dir() {
            let default_path = home.join(".claude").join("projects");
            if default_path.exists() && !roots.contains(&default_path) {
                roots.push(default_path);
            }
        }

        roots
    }

    /// List Codex session files in the given date range
    pub fn list_codex_session_files(
        root: &Path,
        scan_since_key: &str,
        scan_until_key: &str,
    ) -> Vec<PathBuf> {
        let mut files = Vec::new();

        let Some(mut date) = CostUsageDayRange::parse_day_key(scan_since_key) else {
            return files;
        };
        let Some(until_date) = CostUsageDayRange::parse_day_key(scan_until_key) else {
            return files;
        };

        while date <= until_date {
            let year = format!("{:04}", date.year());
            let month = format!("{:02}", date.month());
            let day = format!("{:02}", date.day());

            let day_dir = root.join(&year).join(&month).join(&day);

            if let Ok(entries) = fs::read_dir(&day_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e.eq_ignore_ascii_case("jsonl")) {
                        files.push(path);
                    }
                }
            }

            date += chrono::Duration::days(1);
        }

        files
    }

    /// Parse a Codex JSONL file
    pub fn parse_codex_file(
        file_path: &Path,
        range: &CostUsageDayRange,
        start_offset: i64,
        initial_model: Option<String>,
        initial_totals: Option<CodexTotals>,
    ) -> std::io::Result<CodexParseResult> {
        let file = File::open(file_path)?;
        let file_size = file.metadata()?.len() as i64;

        let mut reader = BufReader::new(file);
        if start_offset > 0 {
            reader.seek(SeekFrom::Start(start_offset as u64))?;
        }

        let mut current_model = initial_model;
        let mut previous_totals = initial_totals;
        let mut days: HashMap<String, HashMap<String, Vec<i32>>> = HashMap::new();
        let mut parsed_bytes = start_offset;

        let mut line = String::new();
        while reader.read_line(&mut line)? > 0 {
            parsed_bytes += line.len() as i64;

            // Quick check for relevant lines
            if !line.contains("\"type\":\"event_msg\"") && !line.contains("\"type\":\"turn_context\"") {
                line.clear();
                continue;
            }

            // Skip event_msg without token_count
            if line.contains("\"type\":\"event_msg\"") && !line.contains("\"token_count\"") {
                line.clear();
                continue;
            }

            // Parse JSON
            if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&line) {
                let msg_type = obj.get("type").and_then(|v| v.as_str());
                let timestamp = obj.get("timestamp").and_then(|v| v.as_str());

                if let (Some(msg_type), Some(ts)) = (msg_type, timestamp) {
                    // Extract day key from timestamp
                    let day_key = if ts.len() >= 10 {
                        &ts[..10]
                    } else {
                        line.clear();
                        continue;
                    };

                    if !CostUsageDayRange::is_in_range(day_key, &range.scan_since_key, &range.scan_until_key) {
                        line.clear();
                        continue;
                    }

                    if msg_type == "turn_context" {
                        // Extract model from turn_context
                        if let Some(payload) = obj.get("payload") {
                            if let Some(model) = payload.get("model").and_then(|v| v.as_str()) {
                                current_model = Some(model.to_string());
                            } else if let Some(info) = payload.get("info") {
                                if let Some(model) = info.get("model").and_then(|v| v.as_str()) {
                                    current_model = Some(model.to_string());
                                }
                            }
                        }
                    } else if msg_type == "event_msg" {
                        if let Some(payload) = obj.get("payload") {
                            if payload.get("type").and_then(|v| v.as_str()) != Some("token_count") {
                                line.clear();
                                continue;
                            }

                            let info = payload.get("info");

                            // Get model
                            let model = info
                                .and_then(|i| i.get("model").or(i.get("model_name")))
                                .or(payload.get("model"))
                                .or(obj.get("model"))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                                .or(current_model.clone())
                                .unwrap_or_else(|| "gpt-5".to_string());

                            // Calculate deltas
                            let (delta_input, delta_cached, delta_output) = if let Some(total) =
                                info.and_then(|i| i.get("total_token_usage"))
                            {
                                let input = total.get("input_tokens").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                                let cached = total
                                    .get("cached_input_tokens")
                                    .or(total.get("cache_read_input_tokens"))
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0) as i32;
                                let output = total.get("output_tokens").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

                                let delta_input = (input - previous_totals.as_ref().map_or(0, |t| t.input)).max(0);
                                let delta_cached = (cached - previous_totals.as_ref().map_or(0, |t| t.cached)).max(0);
                                let delta_output = (output - previous_totals.as_ref().map_or(0, |t| t.output)).max(0);

                                previous_totals = Some(CodexTotals { input, cached, output });

                                (delta_input, delta_cached, delta_output)
                            } else if let Some(last) = info.and_then(|i| i.get("last_token_usage")) {
                                let input = last.get("input_tokens").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                                let cached = last
                                    .get("cached_input_tokens")
                                    .or(last.get("cache_read_input_tokens"))
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0) as i32;
                                let output = last.get("output_tokens").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

                                (input.max(0), cached.max(0), output.max(0))
                            } else {
                                line.clear();
                                continue;
                            };

                            if delta_input == 0 && delta_cached == 0 && delta_output == 0 {
                                line.clear();
                                continue;
                            }

                            // Normalize model name and add to days
                            let norm_model = CostUsagePricing::normalize_codex_model(&model);
                            let cached_clamp = delta_cached.min(delta_input);

                            let day_models = days.entry(day_key.to_string()).or_default();
                            let packed = day_models.entry(norm_model).or_insert_with(|| vec![0, 0, 0]);
                            packed[0] += delta_input;
                            packed[1] += cached_clamp;
                            packed[2] += delta_output;
                        }
                    }
                }
            }

            line.clear();
        }

        Ok(CodexParseResult {
            days,
            parsed_bytes: file_size.max(parsed_bytes),
            last_model: current_model,
            last_totals: previous_totals,
        })
    }

    /// Load cache from disk
    pub fn load_cache(provider: ProviderId, cache_root: Option<&Path>) -> CostUsageCache {
        let cache_path = Self::cache_path(provider, cache_root);

        if let Ok(contents) = fs::read_to_string(&cache_path) {
            if let Ok(cache) = serde_json::from_str(&contents) {
                return cache;
            }
        }

        CostUsageCache::default()
    }

    /// Save cache to disk
    pub fn save_cache(provider: ProviderId, cache: &CostUsageCache, cache_root: Option<&Path>) {
        let cache_path = Self::cache_path(provider, cache_root);

        if let Some(parent) = cache_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if let Ok(json) = serde_json::to_string_pretty(cache) {
            let _ = fs::write(&cache_path, json);
        }
    }

    fn cache_path(provider: ProviderId, cache_root: Option<&Path>) -> PathBuf {
        let root = cache_root
            .map(|p| p.to_path_buf())
            .or_else(|| dirs::cache_dir().map(|d| d.join("CodexBar")))
            .unwrap_or_else(|| PathBuf::from("."));

        root.join(format!("{}_cost_cache.json", provider.cli_name()))
    }
}

use chrono::Datelike;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_day_range() {
        let since = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let until = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
        let range = CostUsageDayRange::new(since, until);

        assert_eq!(range.since_key, "2026-01-15");
        assert_eq!(range.until_key, "2026-01-20");
        assert_eq!(range.scan_since_key, "2026-01-14");
        assert_eq!(range.scan_until_key, "2026-01-21");
    }

    #[test]
    fn test_is_in_range() {
        assert!(CostUsageDayRange::is_in_range("2026-01-15", "2026-01-10", "2026-01-20"));
        assert!(!CostUsageDayRange::is_in_range("2026-01-05", "2026-01-10", "2026-01-20"));
        assert!(!CostUsageDayRange::is_in_range("2026-01-25", "2026-01-10", "2026-01-20"));
    }

    #[test]
    fn test_parse_day_key() {
        let date = CostUsageDayRange::parse_day_key("2026-01-15");
        assert!(date.is_some());
        let date = date.unwrap();
        assert_eq!(date.year(), 2026);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 15);
    }
}
