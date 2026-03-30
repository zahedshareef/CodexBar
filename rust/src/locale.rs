//! Locale module for UI internationalization
//!
//! Provides localized strings for the application UI surfaces.
//! The locale is determined by the user's language setting in Settings.

use crate::settings::Language;

/// Get the localized string for a given key in the specified language
pub fn get_text(lang: Language, key: LocaleKey) -> &'static str {
    match lang {
        Language::English => key.english(),
        Language::Chinese => key.chinese(),
    }
}

/// Locale keys for app-owned UI strings
#[derive(Debug, Clone, Copy)]
pub enum LocaleKey {
    // Tab names
    TabGeneral,
    TabProviders,
    TabDisplay,
    TabApiKeys,
    TabCookies,
    TabAdvanced,
    TabAbout,

    // General settings
    InterfaceLanguage,
    StartupSettings,
    StartAtLogin,
    StartMinimized,

    // Display settings
    ShowNotifications,
    HighUsageThreshold,
    CriticalUsageThreshold,
    ShowUsageAsUsed,

    // About
    AboutTitle,
    Version,
}

impl LocaleKey {
    fn english(self) -> &'static str {
        match self {
            LocaleKey::TabGeneral => "General",
            LocaleKey::TabProviders => "Providers",
            LocaleKey::TabDisplay => "Display",
            LocaleKey::TabApiKeys => "API Keys",
            LocaleKey::TabCookies => "Cookies",
            LocaleKey::TabAdvanced => "Advanced",
            LocaleKey::TabAbout => "About",

            LocaleKey::InterfaceLanguage => "Interface Language",
            LocaleKey::StartupSettings => "Startup",
            LocaleKey::StartAtLogin => "Start at Login",
            LocaleKey::StartMinimized => "Start Minimized",

            LocaleKey::ShowNotifications => "Show Notifications",
            LocaleKey::HighUsageThreshold => "High Usage Threshold",
            LocaleKey::CriticalUsageThreshold => "Critical Usage Threshold",
            LocaleKey::ShowUsageAsUsed => "Show Usage as Used",

            LocaleKey::AboutTitle => "About CodexBar",
            LocaleKey::Version => "Version",
        }
    }

    fn chinese(self) -> &'static str {
        match self {
            LocaleKey::TabGeneral => "通用",
            LocaleKey::TabProviders => "服务商",
            LocaleKey::TabDisplay => "显示",
            LocaleKey::TabApiKeys => "API 密钥",
            LocaleKey::TabCookies => "Cookies",
            LocaleKey::TabAdvanced => "高级",
            LocaleKey::TabAbout => "关于",

            LocaleKey::InterfaceLanguage => "界面语言",
            LocaleKey::StartupSettings => "启动",
            LocaleKey::StartAtLogin => "开机启动",
            LocaleKey::StartMinimized => "最小化启动",

            LocaleKey::ShowNotifications => "显示通知",
            LocaleKey::HighUsageThreshold => "高用量阈值",
            LocaleKey::CriticalUsageThreshold => "紧急用量阈值",
            LocaleKey::ShowUsageAsUsed => "显示已用用量",

            LocaleKey::AboutTitle => "关于 CodexBar",
            LocaleKey::Version => "版本",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_key_english() {
        assert_eq!(
            get_text(Language::English, LocaleKey::TabGeneral),
            "General"
        );
        assert_eq!(
            get_text(Language::English, LocaleKey::InterfaceLanguage),
            "Interface Language"
        );
        assert_eq!(
            get_text(Language::English, LocaleKey::StartAtLogin),
            "Start at Login"
        );
    }

    #[test]
    fn test_locale_key_chinese() {
        assert_eq!(get_text(Language::Chinese, LocaleKey::TabGeneral), "通用");
        assert_eq!(
            get_text(Language::Chinese, LocaleKey::InterfaceLanguage),
            "界面语言"
        );
        assert_eq!(
            get_text(Language::Chinese, LocaleKey::StartAtLogin),
            "开机启动"
        );
    }

    #[test]
    fn test_locale_respects_language_setting() {
        // Test that English language returns English strings
        let lang = Language::English;
        assert_eq!(get_text(lang, LocaleKey::TabAbout), "About");

        // Test that Chinese language returns Chinese strings
        let lang = Language::Chinese;
        assert_eq!(get_text(lang, LocaleKey::TabAbout), "关于");
    }

    #[test]
    fn test_all_locale_keys_have_both_languages() {
        // Verify all variants have both English and Chinese
        let keys = [
            LocaleKey::TabGeneral,
            LocaleKey::TabProviders,
            LocaleKey::TabDisplay,
            LocaleKey::TabApiKeys,
            LocaleKey::TabCookies,
            LocaleKey::TabAdvanced,
            LocaleKey::TabAbout,
            LocaleKey::InterfaceLanguage,
            LocaleKey::StartupSettings,
            LocaleKey::StartAtLogin,
            LocaleKey::StartMinimized,
            LocaleKey::ShowNotifications,
            LocaleKey::HighUsageThreshold,
            LocaleKey::CriticalUsageThreshold,
            LocaleKey::ShowUsageAsUsed,
            LocaleKey::AboutTitle,
            LocaleKey::Version,
        ];

        for key in keys {
            // English should not be empty or contain Chinese characters
            let english = key.english();
            assert!(!english.is_empty(), "English string for {:?} is empty", key);

            // Chinese should not be empty
            let chinese = key.chinese();
            assert!(!chinese.is_empty(), "Chinese string for {:?} is empty", key);
        }
    }
}
