//! Personal Info Redaction
//!
//! Redacts email addresses and other personal information for privacy,
//! useful when streaming or sharing screen.

use regex_lite::Regex;
use std::sync::OnceLock;

/// Placeholder text for redacted emails
pub const EMAIL_PLACEHOLDER: &str = "Hidden";

/// Get the compiled email regex
fn email_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        // Case-insensitive email pattern
        Regex::new(r"(?i)[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}").expect("Invalid email regex")
    })
}

/// Personal information redactor
pub struct PersonalInfoRedactor;

impl PersonalInfoRedactor {
    /// Redact a single email address if privacy mode is enabled
    ///
    /// # Arguments
    /// * `email` - The email address to potentially redact
    /// * `is_enabled` - Whether privacy/redaction mode is enabled
    ///
    /// # Returns
    /// The original email if disabled, or "Hidden" if enabled
    pub fn redact_email(email: Option<&str>, is_enabled: bool) -> String {
        match email {
            Some(e) if !e.trim().is_empty() => {
                if is_enabled {
                    EMAIL_PLACEHOLDER.to_string()
                } else {
                    e.to_string()
                }
            }
            _ => String::new(),
        }
    }

    /// Redact all email addresses in a text string
    ///
    /// # Arguments
    /// * `text` - The text containing potential email addresses
    /// * `is_enabled` - Whether privacy/redaction mode is enabled
    ///
    /// # Returns
    /// The text with all emails replaced with "Hidden" if enabled
    pub fn redact_emails_in_text(text: Option<&str>, is_enabled: bool) -> Option<String> {
        let text = text?;
        if !is_enabled {
            return Some(text.to_string());
        }

        let regex = email_regex();
        Some(regex.replace_all(text, EMAIL_PLACEHOLDER).to_string())
    }

    /// Partially redact an email, showing first few chars and domain
    ///
    /// Example: "user@example.com" -> "u***@example.com"
    pub fn partial_redact_email(email: Option<&str>, is_enabled: bool) -> String {
        match email {
            Some(e) if !e.trim().is_empty() => {
                if !is_enabled {
                    return e.to_string();
                }

                // Split on @ to get local and domain parts
                if let Some((local, domain)) = e.split_once('@') {
                    if local.is_empty() {
                        return EMAIL_PLACEHOLDER.to_string();
                    }
                    // Show first char, replace rest with ***
                    let first_char: String = local.chars().take(1).collect();
                    format!("{}***@{}", first_char, domain)
                } else {
                    EMAIL_PLACEHOLDER.to_string()
                }
            }
            _ => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_email_disabled() {
        let email = "test@example.com";
        assert_eq!(
            PersonalInfoRedactor::redact_email(Some(email), false),
            email
        );
    }

    #[test]
    fn test_redact_email_enabled() {
        let email = "test@example.com";
        assert_eq!(
            PersonalInfoRedactor::redact_email(Some(email), true),
            EMAIL_PLACEHOLDER
        );
    }

    #[test]
    fn test_redact_email_none() {
        assert_eq!(PersonalInfoRedactor::redact_email(None, true), "");
        assert_eq!(PersonalInfoRedactor::redact_email(Some(""), true), "");
        assert_eq!(PersonalInfoRedactor::redact_email(Some("  "), true), "");
    }

    #[test]
    fn test_redact_emails_in_text() {
        let text = "Contact me at user@example.com or admin@test.org for help";
        let result = PersonalInfoRedactor::redact_emails_in_text(Some(text), true);
        assert_eq!(
            result,
            Some("Contact me at Hidden or Hidden for help".to_string())
        );
    }

    #[test]
    fn test_redact_emails_disabled() {
        let text = "Contact me at user@example.com";
        let result = PersonalInfoRedactor::redact_emails_in_text(Some(text), false);
        assert_eq!(result, Some(text.to_string()));
    }

    #[test]
    fn test_partial_redact_email() {
        assert_eq!(
            PersonalInfoRedactor::partial_redact_email(Some("john@example.com"), true),
            "j***@example.com"
        );
        assert_eq!(
            PersonalInfoRedactor::partial_redact_email(Some("test@domain.org"), false),
            "test@domain.org"
        );
    }
}
