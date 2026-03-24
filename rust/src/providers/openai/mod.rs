//! OpenAI/ChatGPT provider implementation
//!
//! Provides usage data scraping from OpenAI dashboard using browser automation.

pub mod friendly_errors;
pub mod scraper;

// Re-exports for error handling and dashboard scraping
#[allow(unused_imports)]
pub use friendly_errors::{
    extract_auth_status, extract_signed_in_email, friendly_error, is_logged_out,
    OpenAIWebErrorKind,
};
#[allow(unused_imports)]
pub use scraper::{
    CreditsHistoryEntry, OpenAIDashboardData, UsageBreakdown, parse_dashboard_json,
    OPENAI_DASHBOARD_SCRAPE_SCRIPT,
};
