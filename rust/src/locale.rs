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
    // Tab names (Preferences)
    TabGeneral,
    TabProviders,
    TabDisplay,
    TabApiKeys,
    TabCookies,
    TabAdvanced,
    TabAbout,

    // General settings (Preferences)
    InterfaceLanguage,
    StartupSettings,
    StartAtLogin,
    StartMinimized,

    // Display settings (Preferences)
    ShowNotifications,
    HighUsageThreshold,
    CriticalUsageThreshold,
    ShowUsageAsUsed,

    // About (Preferences)
    AboutTitle,
    Version,

    // Main popup - Header actions
    ActionRefreshAll,
    ActionSettings,
    ActionClose,

    // Main popup - Provider section
    ProviderAccount,
    ProviderSession,
    ProviderWeekly,
    ProviderModel,
    ProviderPlan,
    ProviderNextReset,
    ProviderNoRecentUsage,
    ProviderNotSignedIn,

    // Main popup - Loading/Empty/Error states (non-happy-path)
    StateLoadingProviders,
    StateNoProviderData,
    StateNoProviderSelected,
    StateError,
    StateRetry,
    StateDownload,
    StateRestartAndUpdate,

    // Main popup - Credits
    CreditsTitle,

    // Main popup - Update banner (non-happy-path)
    UpdateRestartAndUpdate,
    UpdateRetry,
    UpdateDownload,
    UpdateDownloading,
    UpdateReady,
    UpdateFailed,

    // Main popup - Settings button
    ButtonOpenProviderSettings,

    // Main popup - Bottom menu (Actions)
    MenuSettings,
    MenuAbout,
    MenuQuit,

    // Main popup - Status strings
    StatusJustUpdated,
    StatusUnableToGetUsage,

    // Main popup - Provider detail actions
    ActionRefresh,
    ActionSwitchAccount,
    ActionUsageDashboard,
    ActionStatusPage,
    ActionCopyError,
    ActionBuyCredits,

    // Main popup - Pace status
    PaceOnTrack,
    PaceBehind,

    // Main popup - Reset prefix
    MetricResetsIn,

    // Main popup - Section titles
    SectionUsageBreakdown,
    SectionCost,
}

impl LocaleKey {
    fn english(self) -> &'static str {
        match self {
            // Tab names
            LocaleKey::TabGeneral => "General",
            LocaleKey::TabProviders => "Providers",
            LocaleKey::TabDisplay => "Display",
            LocaleKey::TabApiKeys => "API Keys",
            LocaleKey::TabCookies => "Cookies",
            LocaleKey::TabAdvanced => "Advanced",
            LocaleKey::TabAbout => "About",

            // General settings
            LocaleKey::InterfaceLanguage => "Interface Language",
            LocaleKey::StartupSettings => "Startup",
            LocaleKey::StartAtLogin => "Start at Login",
            LocaleKey::StartMinimized => "Start Minimized",

            // Display settings
            LocaleKey::ShowNotifications => "Show Notifications",
            LocaleKey::HighUsageThreshold => "High Usage Threshold",
            LocaleKey::CriticalUsageThreshold => "Critical Usage Threshold",
            LocaleKey::ShowUsageAsUsed => "Show Usage as Used",

            // About
            LocaleKey::AboutTitle => "About CodexBar",
            LocaleKey::Version => "Version",

            // Main popup - Header actions
            LocaleKey::ActionRefreshAll => "Refresh All",
            LocaleKey::ActionSettings => "Settings",
            LocaleKey::ActionClose => "✕",

            // Main popup - Provider section
            LocaleKey::ProviderAccount => "Account",
            LocaleKey::ProviderSession => "Session",
            LocaleKey::ProviderWeekly => "Weekly",
            LocaleKey::ProviderModel => "Model",
            LocaleKey::ProviderPlan => "Plan",
            LocaleKey::ProviderNextReset => "Next Reset",
            LocaleKey::ProviderNoRecentUsage => "No recent usage",
            LocaleKey::ProviderNotSignedIn => "Not signed in",

            // Main popup - Loading/Empty/Error states
            LocaleKey::StateLoadingProviders => "Loading providers...",
            LocaleKey::StateNoProviderData => "No provider data.",
            LocaleKey::StateNoProviderSelected => "No provider selected.",
            LocaleKey::StateError => "Error",
            LocaleKey::StateRetry => "Retry",
            LocaleKey::StateDownload => "Download",
            LocaleKey::StateRestartAndUpdate => "Restart & Update",

            // Main popup - Credits
            LocaleKey::CreditsTitle => "Credits",

            // Main popup - Update banner (non-happy-path)
            LocaleKey::UpdateRestartAndUpdate => "Restart & Update",
            LocaleKey::UpdateRetry => "Retry",
            LocaleKey::UpdateDownload => "Download",
            LocaleKey::UpdateDownloading => "Downloading",
            LocaleKey::UpdateReady => "Ready to install",
            LocaleKey::UpdateFailed => "Update failed",

            // Main popup - Settings button
            LocaleKey::ButtonOpenProviderSettings => "Open provider settings",

            // Main popup - Bottom menu (Actions)
            LocaleKey::MenuSettings => "Settings...",
            LocaleKey::MenuAbout => "About CodexBar",
            LocaleKey::MenuQuit => "Quit",

            // Main popup - Status strings
            LocaleKey::StatusJustUpdated => "Just updated",
            LocaleKey::StatusUnableToGetUsage => "Unable to get usage",

            // Main popup - Provider detail actions
            LocaleKey::ActionRefresh => "Refresh",
            LocaleKey::ActionSwitchAccount => "Switch account...",
            LocaleKey::ActionUsageDashboard => "Usage dashboard",
            LocaleKey::ActionStatusPage => "Status page",
            LocaleKey::ActionCopyError => "Copy error",
            LocaleKey::ActionBuyCredits => "Buy credits...",

            // Main popup - Pace status
            LocaleKey::PaceOnTrack => "On track",
            LocaleKey::PaceBehind => "Behind",

            // Main popup - Reset prefix
            LocaleKey::MetricResetsIn => "Resets in",

            // Main popup - Section titles
            LocaleKey::SectionUsageBreakdown => "Usage Breakdown",
            LocaleKey::SectionCost => "Cost",
        }
    }

    fn chinese(self) -> &'static str {
        match self {
            // Tab names
            LocaleKey::TabGeneral => "通用",
            LocaleKey::TabProviders => "服务商",
            LocaleKey::TabDisplay => "显示",
            LocaleKey::TabApiKeys => "API 密钥",
            LocaleKey::TabCookies => "Cookies",
            LocaleKey::TabAdvanced => "高级",
            LocaleKey::TabAbout => "关于",

            // General settings
            LocaleKey::InterfaceLanguage => "界面语言",
            LocaleKey::StartupSettings => "启动",
            LocaleKey::StartAtLogin => "开机启动",
            LocaleKey::StartMinimized => "最小化启动",

            // Display settings
            LocaleKey::ShowNotifications => "显示通知",
            LocaleKey::HighUsageThreshold => "高用量阈值",
            LocaleKey::CriticalUsageThreshold => "紧急用量阈值",
            LocaleKey::ShowUsageAsUsed => "显示已用用量",

            // About
            LocaleKey::AboutTitle => "关于 CodexBar",
            LocaleKey::Version => "版本",

            // Main popup - Header actions
            LocaleKey::ActionRefreshAll => "刷新全部",
            LocaleKey::ActionSettings => "设置",
            LocaleKey::ActionClose => "✕",

            // Main popup - Provider section
            LocaleKey::ProviderAccount => "账号",
            LocaleKey::ProviderSession => "本次会话",
            LocaleKey::ProviderWeekly => "本周",
            LocaleKey::ProviderModel => "模型",
            LocaleKey::ProviderPlan => "套餐",
            LocaleKey::ProviderNextReset => "下次重置",
            LocaleKey::ProviderNoRecentUsage => "暂无用量",
            LocaleKey::ProviderNotSignedIn => "未登录",

            // Main popup - Loading/Empty/Error states
            LocaleKey::StateLoadingProviders => "正在加载服务商...",
            LocaleKey::StateNoProviderData => "暂无服务商数据。",
            LocaleKey::StateNoProviderSelected => "尚未选择服务商。",
            LocaleKey::StateError => "错误",
            LocaleKey::StateRetry => "重试",
            LocaleKey::StateDownload => "下载",
            LocaleKey::StateRestartAndUpdate => "重启并更新",

            // Main popup - Credits
            LocaleKey::CreditsTitle => "额度",

            // Main popup - Update banner (non-happy-path)
            LocaleKey::UpdateRestartAndUpdate => "重启并更新",
            LocaleKey::UpdateRetry => "重试",
            LocaleKey::UpdateDownload => "下载",
            LocaleKey::UpdateDownloading => "下载中",
            LocaleKey::UpdateReady => "准备安装",
            LocaleKey::UpdateFailed => "更新失败",

            // Main popup - Settings button
            LocaleKey::ButtonOpenProviderSettings => "打开服务商设置",

            // Main popup - Bottom menu (Actions)
            LocaleKey::MenuSettings => "设置...",
            LocaleKey::MenuAbout => "关于 CodexBar",
            LocaleKey::MenuQuit => "退出",

            // Main popup - Status strings
            LocaleKey::StatusJustUpdated => "刚刚更新",
            LocaleKey::StatusUnableToGetUsage => "无法获取用量",

            // Main popup - Provider detail actions
            LocaleKey::ActionRefresh => "刷新",
            LocaleKey::ActionSwitchAccount => "切换账号...",
            LocaleKey::ActionUsageDashboard => "用量仪表盘",
            LocaleKey::ActionStatusPage => "状态页面",
            LocaleKey::ActionCopyError => "复制错误",
            LocaleKey::ActionBuyCredits => "购买额度...",

            // Main popup - Pace status
            LocaleKey::PaceOnTrack => "进度正常",
            LocaleKey::PaceBehind => "进度滞后",

            // Main popup - Reset prefix
            LocaleKey::MetricResetsIn => "重置于",

            // Main popup - Section titles
            LocaleKey::SectionUsageBreakdown => "用量明细",
            LocaleKey::SectionCost => "费用",
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
            // Tab names
            LocaleKey::TabGeneral,
            LocaleKey::TabProviders,
            LocaleKey::TabDisplay,
            LocaleKey::TabApiKeys,
            LocaleKey::TabCookies,
            LocaleKey::TabAdvanced,
            LocaleKey::TabAbout,
            // General settings
            LocaleKey::InterfaceLanguage,
            LocaleKey::StartupSettings,
            LocaleKey::StartAtLogin,
            LocaleKey::StartMinimized,
            // Display settings
            LocaleKey::ShowNotifications,
            LocaleKey::HighUsageThreshold,
            LocaleKey::CriticalUsageThreshold,
            LocaleKey::ShowUsageAsUsed,
            // About
            LocaleKey::AboutTitle,
            LocaleKey::Version,
            // Main popup - Header actions
            LocaleKey::ActionRefreshAll,
            LocaleKey::ActionSettings,
            LocaleKey::ActionClose,
            // Main popup - Provider section
            LocaleKey::ProviderAccount,
            LocaleKey::ProviderSession,
            LocaleKey::ProviderWeekly,
            LocaleKey::ProviderModel,
            LocaleKey::ProviderPlan,
            LocaleKey::ProviderNextReset,
            LocaleKey::ProviderNoRecentUsage,
            LocaleKey::ProviderNotSignedIn,
            // Main popup - Loading/Empty/Error states
            LocaleKey::StateLoadingProviders,
            LocaleKey::StateNoProviderData,
            LocaleKey::StateNoProviderSelected,
            LocaleKey::StateError,
            LocaleKey::StateRetry,
            LocaleKey::StateDownload,
            LocaleKey::StateRestartAndUpdate,
            // Main popup - Credits
            LocaleKey::CreditsTitle,
            // Main popup - Update banner (non-happy-path)
            LocaleKey::UpdateRestartAndUpdate,
            LocaleKey::UpdateRetry,
            LocaleKey::UpdateDownload,
            LocaleKey::UpdateDownloading,
            LocaleKey::UpdateReady,
            LocaleKey::UpdateFailed,
            // Main popup - Settings button
            LocaleKey::ButtonOpenProviderSettings,
            // Main popup - Bottom menu (Actions)
            LocaleKey::MenuSettings,
            LocaleKey::MenuAbout,
            LocaleKey::MenuQuit,
            // Main popup - Status strings
            LocaleKey::StatusJustUpdated,
            LocaleKey::StatusUnableToGetUsage,
            // Main popup - Provider detail actions
            LocaleKey::ActionRefresh,
            LocaleKey::ActionSwitchAccount,
            LocaleKey::ActionUsageDashboard,
            LocaleKey::ActionStatusPage,
            LocaleKey::ActionCopyError,
            LocaleKey::ActionBuyCredits,
            // Main popup - Pace status
            LocaleKey::PaceOnTrack,
            LocaleKey::PaceBehind,
            // Main popup - Reset prefix
            LocaleKey::MetricResetsIn,
            // Main popup - Section titles
            LocaleKey::SectionUsageBreakdown,
            LocaleKey::SectionCost,
        ];

        for key in keys {
            // English should not be empty
            let english = key.english();
            assert!(!english.is_empty(), "English string for {:?} is empty", key);

            // Chinese should not be empty
            let chinese = key.chinese();
            assert!(!chinese.is_empty(), "Chinese string for {:?} is empty", key);
        }
    }
}
