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
    StartAtLoginHelper,
    StartMinimizedHelper,

    // Notification settings (Preferences)
    ShowNotificationsHelper,
    SoundEnabledHelper,
    HighUsageThresholdHelper,
    CriticalUsageThresholdHelper,

    // Notification settings (Preferences)
    ShowNotifications,
    SoundEnabled,
    SoundVolume,
    HighUsageThreshold,
    HighUsageAlert,
    CriticalUsageThreshold,
    CriticalUsageAlert,

    // Display settings (Preferences)
    UsageDisplay,
    ShowUsageAsUsed,
    ShowUsageAsUsedHelper,
    ResetTimeRelative,
    ResetTimeRelativeHelper,
    ShowCreditsExtra,
    ShowCreditsExtraHelper,
    TrayIcon,
    MergeTrayIcons,
    MergeTrayIconsHelper,
    PerProviderTrayIcons,
    PerProviderTrayIconsHelper,

    // Provider settings (Preferences)
    ProviderEnabled,
    ProviderDisabled,
    ProviderInfo,
    ProviderUsage,
    AuthType,
    DataSource,
    TrackingItem,
    MainWindowLiveUsageData,
    StartTrackingUsage,
    ClickTrayIconForMetrics,

    // Browser cookie import (Preferences)
    BrowserCookieImport,
    ImportFromBrowser,
    NoCookiesFoundInBrowser,
    SelectBrowser,
    ImportCookies,
    ImportSuccess,
    ImportFailed,
    SaveFailed,
    CookiesAutoImport,
    QuickActions,
    OpenProviderDashboard,
    OllamaNoDashboard,

    // API Keys tab (Preferences)
    ApiKeysTitle,
    ApiKeysDescription,
    AddKey,
    KeySet,
    KeyRequired,
    Remove,
    GetKey,

    // Cookies tab (Preferences)
    SavedCookies,
    AddManualCookie,
    CookieHeader,
    PasteHere,
    DeleteCookie,
    CookieSaved,
    CookieDeleted,

    // Advanced tab (Preferences)
    RefreshSettings,
    Animations,
    MenuBar,
    Fun,
    GlobalShortcut,
    Privacy,
    Updates,
    UpdateChannel,
    UpdateChannelStable,
    UpdateChannelBeta,
    Never,
    LastUpdated,
    NeverUpdated,
    MinutesAgo,
    HoursAgo,
    DaysAgo,
    BuiltWithRust,
    OriginalMacOSVersion,
    Links,
    BuildInfo,
    EnabledProviders,
    Appearance,
    ThemeSelection,
    LightMode,
    DarkMode,

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
            LocaleKey::StartAtLoginHelper => "Login automatically after system startup",
            LocaleKey::StartMinimizedHelper => "Start minimized to system tray",

            // Notification settings
            LocaleKey::ShowNotifications => "Show Notifications",
            LocaleKey::ShowNotificationsHelper => "Alert when usage thresholds are reached",
            LocaleKey::SoundEnabled => "Sound Alerts",
            LocaleKey::SoundEnabledHelper => "Play sound when thresholds are reached",
            LocaleKey::SoundVolume => "Alert Volume",
            LocaleKey::HighUsageThreshold => "High Usage Threshold",
            LocaleKey::HighUsageThresholdHelper => "Show warning at this usage level",
            LocaleKey::HighUsageAlert => "High Usage Alert",
            LocaleKey::CriticalUsageThreshold => "Critical Usage Threshold",
            LocaleKey::CriticalUsageThresholdHelper => "Show critical alert at this level",
            LocaleKey::CriticalUsageAlert => "Critical Alert",

            // Display settings
            LocaleKey::UsageDisplay => "Usage Display",
            LocaleKey::ShowUsageAsUsed => "Show Usage as Used",
            LocaleKey::ShowUsageAsUsedHelper => "Display as used percentage instead of remaining",
            LocaleKey::ResetTimeRelative => "Relative Reset Time",
            LocaleKey::ResetTimeRelativeHelper => "Show \"2h 30m\" instead of \"3:00 PM\"",
            LocaleKey::ShowCreditsExtra => "Show Credits & Extra Usage",
            LocaleKey::ShowCreditsExtraHelper => "Display credit balance and extra usage info",
            LocaleKey::TrayIcon => "Tray Icon",
            LocaleKey::MergeTrayIcons => "Merge Tray Icons",
            LocaleKey::MergeTrayIconsHelper => "Show all providers in a single tray icon",
            LocaleKey::PerProviderTrayIcons => "Per-Provider Icons",
            LocaleKey::PerProviderTrayIconsHelper => {
                "Show separate tray icon for each enabled provider"
            }

            // Provider settings
            LocaleKey::ProviderEnabled => "Enabled",
            LocaleKey::ProviderDisabled => "Disabled",
            LocaleKey::ProviderInfo => "Info",
            LocaleKey::ProviderUsage => "Usage",
            LocaleKey::AuthType => "Authentication",
            LocaleKey::DataSource => "Data Source",
            LocaleKey::TrackingItem => "Tracked Item",
            LocaleKey::MainWindowLiveUsageData => "Live usage data in main window",
            LocaleKey::StartTrackingUsage => "Enable to start tracking usage",
            LocaleKey::ClickTrayIconForMetrics => "Click tray icon for live metrics",

            // Browser cookie import
            LocaleKey::BrowserCookieImport => "Browser Cookie Import",
            LocaleKey::ImportFromBrowser => "Import {} cookies from browser",
            LocaleKey::NoCookiesFoundInBrowser => "No cookies found in {}. Please log in first.",
            LocaleKey::SelectBrowser => "Select browser...",
            LocaleKey::ImportCookies => "Import Cookies",
            LocaleKey::ImportSuccess => "Imported cookies for {}",
            LocaleKey::ImportFailed => "Import failed: {}",
            LocaleKey::SaveFailed => "Save failed: {}",
            LocaleKey::CookiesAutoImport => {
                "Cookies are automatically imported from Chrome, Edge, Brave and Firefox"
            }
            LocaleKey::QuickActions => "Quick Actions",
            LocaleKey::OpenProviderDashboard => "Open {} Dashboard",
            LocaleKey::OllamaNoDashboard => "Ollama runs locally, no dashboard",

            // API Keys tab
            LocaleKey::ApiKeysTitle => "API Keys",
            LocaleKey::ApiKeysDescription => {
                "Configure access tokens for providers that require authentication."
            }
            LocaleKey::AddKey => "+ Add Key",
            LocaleKey::KeySet => "Set",
            LocaleKey::KeyRequired => "Key Required",
            LocaleKey::Remove => "Remove",
            LocaleKey::GetKey => "Get key →",

            // Cookies tab
            LocaleKey::SavedCookies => "Saved Cookies",
            LocaleKey::AddManualCookie => "Add Manual Cookie",
            LocaleKey::CookieHeader => "Cookie Header",
            LocaleKey::PasteHere => "Paste here...",
            LocaleKey::DeleteCookie => "Delete",
            LocaleKey::CookieSaved => "Saved {} cookies",
            LocaleKey::CookieDeleted => "Deleted cookies for {}",

            // Advanced tab
            LocaleKey::RefreshSettings => "Refresh",
            LocaleKey::Animations => "Animations",
            LocaleKey::MenuBar => "Menu Bar",
            LocaleKey::Fun => "Fun",
            LocaleKey::GlobalShortcut => "Global Shortcut",
            LocaleKey::Privacy => "Privacy",
            LocaleKey::Updates => "Updates",
            LocaleKey::UpdateChannel => "Update Channel",
            LocaleKey::UpdateChannelStable => "Stable",
            LocaleKey::UpdateChannelBeta => "Beta",
            LocaleKey::Never => "Never",
            LocaleKey::LastUpdated => "Updated",
            LocaleKey::NeverUpdated => "Never updated",
            LocaleKey::MinutesAgo => "{} minutes ago",
            LocaleKey::HoursAgo => "{} hours ago",
            LocaleKey::DaysAgo => "{} days ago",
            LocaleKey::BuiltWithRust => "Built with Rust + egui",
            LocaleKey::OriginalMacOSVersion => "Original macOS version",
            LocaleKey::Links => "Links",
            LocaleKey::BuildInfo => "Build Info",
            LocaleKey::EnabledProviders => "Enabled Providers",
            LocaleKey::Appearance => "Appearance",
            LocaleKey::ThemeSelection => "Theme",
            LocaleKey::LightMode => "Light",
            LocaleKey::DarkMode => "Dark",

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
            LocaleKey::StartAtLoginHelper => "登录后自动启动 CodexBar",
            LocaleKey::StartMinimizedHelper => "启动后停留在系统托盘",

            // Notification settings
            LocaleKey::ShowNotifications => "显示通知",
            LocaleKey::ShowNotificationsHelper => "达到用量阈值时提醒",
            LocaleKey::SoundEnabled => "声音提示",
            LocaleKey::SoundEnabledHelper => "达到阈值时播放提示音",
            LocaleKey::SoundVolume => "提示音音量",
            LocaleKey::HighUsageThreshold => "高用量阈值",
            LocaleKey::HighUsageThresholdHelper => "在该用量水平显示预警",
            LocaleKey::HighUsageAlert => "高位预警",
            LocaleKey::CriticalUsageThreshold => "紧急用量阈值",
            LocaleKey::CriticalUsageThresholdHelper => "在该水平显示严重告警",
            LocaleKey::CriticalUsageAlert => "严重告警",

            // Display settings
            LocaleKey::ShowUsageAsUsed => "显示已用用量",
            LocaleKey::ShowUsageAsUsedHelper => "显示为已使用百分比（而非剩余）",
            LocaleKey::ResetTimeRelative => "相对重置时间",
            LocaleKey::ResetTimeRelativeHelper => "显示\"2h 30m\"而不是\"3:00 PM\"",
            LocaleKey::ShowCreditsExtra => "显示额度与扩展用量",
            LocaleKey::ShowCreditsExtraHelper => "显示额度余额和额外用量信息",
            LocaleKey::UsageDisplay => "用量显示",
            LocaleKey::TrayIcon => "托盘图标",
            LocaleKey::MergeTrayIcons => "合并托盘图标",
            LocaleKey::MergeTrayIconsHelper => "将所有服务商显示在一个托盘图标中",
            LocaleKey::PerProviderTrayIcons => "按服务商分图标",
            LocaleKey::PerProviderTrayIconsHelper => "每个启用的服务商显示独立托盘图标",

            // Provider settings
            LocaleKey::ProviderEnabled => "已启用",
            LocaleKey::ProviderDisabled => "已禁用",
            LocaleKey::ProviderInfo => "信息",
            LocaleKey::ProviderUsage => "用量",
            LocaleKey::AuthType => "认证方式",
            LocaleKey::DataSource => "数据来源",
            LocaleKey::TrackingItem => "追踪项",
            LocaleKey::MainWindowLiveUsageData => "主窗口实时用量数据",
            LocaleKey::StartTrackingUsage => "启用后开始追踪用量",
            LocaleKey::ClickTrayIconForMetrics => "点击托盘图标查看实时指标",

            // Browser cookie import
            LocaleKey::BrowserCookieImport => "浏览器 Cookie 导入",
            LocaleKey::ImportFromBrowser => "从浏览器导入 {} 的 Cookies",
            LocaleKey::NoCookiesFoundInBrowser => "在 {} 的 {} 中未找到 Cookies。请先确认已登录",
            LocaleKey::SelectBrowser => "请选择浏览器...",
            LocaleKey::ImportCookies => "导入 Cookies",
            LocaleKey::ImportSuccess => "已为 {} 导入 Cookies",
            LocaleKey::ImportFailed => "导入失败：{}",
            LocaleKey::SaveFailed => "保存失败：{}",
            LocaleKey::CookiesAutoImport => {
                "Cookies 会自动从 Chrome、Edge、Brave 和 Firefox 中提取"
            }
            LocaleKey::QuickActions => "快捷操作",
            LocaleKey::OpenProviderDashboard => "→ 打开 {} 仪表盘",
            LocaleKey::OllamaNoDashboard => "Ollama 在本地运行，无仪表盘",

            // API Keys tab
            LocaleKey::ApiKeysTitle => "API 密钥",
            LocaleKey::ApiKeysDescription => "为需要认证的服务商配置访问令牌。",
            LocaleKey::AddKey => "+ 添加密钥",
            LocaleKey::KeySet => "✓ 已设置",
            LocaleKey::KeyRequired => "需要密钥",
            LocaleKey::Remove => "移除",
            LocaleKey::GetKey => "获取密钥 →",

            // Cookies tab
            LocaleKey::SavedCookies => "已保存的 Cookies",
            LocaleKey::AddManualCookie => "添加手动 Cookie",
            LocaleKey::CookieHeader => "Cookie 头",
            LocaleKey::PasteHere => "在这里粘贴...",
            LocaleKey::DeleteCookie => "删除",
            LocaleKey::CookieSaved => "已保存 {} 个 Cookies",
            LocaleKey::CookieDeleted => "已删除 {} 的 Cookies",

            // Advanced tab
            LocaleKey::RefreshSettings => "刷新",
            LocaleKey::Animations => "动画",
            LocaleKey::MenuBar => "菜单栏",
            LocaleKey::Fun => "趣味",
            LocaleKey::GlobalShortcut => "全局快捷键",
            LocaleKey::Privacy => "隐私",
            LocaleKey::Updates => "更新",
            LocaleKey::UpdateChannel => "更新通道",
            LocaleKey::UpdateChannelStable => "稳定版",
            LocaleKey::UpdateChannelBeta => "测试预览版",
            LocaleKey::Never => "从不",
            LocaleKey::LastUpdated => "上次更新",
            LocaleKey::NeverUpdated => "从未更新",
            LocaleKey::MinutesAgo => "{} 分钟前更新",
            LocaleKey::HoursAgo => "{} 小时前更新",
            LocaleKey::DaysAgo => "{} 天前更新",
            LocaleKey::BuiltWithRust => "基于 Rust + egui 构建",
            LocaleKey::OriginalMacOSVersion => "原始 macOS 版本",
            LocaleKey::Links => "链接",
            LocaleKey::BuildInfo => "构建信息",
            LocaleKey::EnabledProviders => "已启用服务商",
            LocaleKey::Appearance => "外观",
            LocaleKey::ThemeSelection => "主题",
            LocaleKey::LightMode => "浅色",
            LocaleKey::DarkMode => "深色",

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
