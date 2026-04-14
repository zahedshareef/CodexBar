//! Main egui application - Modern refined menubar popup
//! Clean, spacious design with rich visual hierarchy

use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, Rect, RichText, Rounding, Stroke, Vec2,
};
#[cfg(debug_assertions)]
use image::ColorType;
#[cfg(debug_assertions)]
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::charts::{
    ChartPoint, CostHistoryChart, CreditsHistoryChart, ServiceUsage, UsageBreakdownChart,
    UsageBreakdownPoint,
};
use super::preferences::PreferencesWindow;
use super::provider_icons::ProviderIconCache;
use super::theme::{FontSize, Radius, Spacing, Theme, provider_color, status_color};
use crate::browser::cookies::get_cookie_header;
use crate::core::{
    FetchContext, OpenAIDashboardCacheStore, PersonalInfoRedactor, Provider, ProviderFetchResult,
    ProviderId, RateWindow,
};
use crate::core::{TokenAccountStore, TokenAccountSupport};
use crate::cost_scanner::get_daily_cost_history;
use crate::locale::{LocaleKey, get_text as locale_text};
use crate::login::LoginPhase;
use crate::notifications::NotificationManager;
use crate::providers::*;
use crate::settings::{ApiKeys, Language, ManualCookies, Settings, TrayIconMode, UpdateChannel};
use crate::shortcuts::{ShortcutManager, parse_shortcut};
use crate::status::{StatusLevel, fetch_provider_status, get_status_page_url};
use crate::tray::{
    LoadingPattern, ProviderUsage, SurpriseAnimation, TrayMenuAction, UnifiedTrayManager,
};
use crate::updater::{self, UpdateInfo, UpdateState};

#[cfg(windows)]
fn find_main_window() -> Option<windows::Win32::Foundation::HWND> {
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GWL_EXSTYLE, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, WS_EX_TOOLWINDOW,
    };

    struct SearchState {
        pid: u32,
        preferred: Option<HWND>,
        fallback: Option<HWND>,
    }

    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let state = unsafe { &mut *(lparam.0 as *mut SearchState) };

        let mut process_id = 0u32;
        let _ = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) };
        if process_id != state.pid {
            return true.into();
        }

        let ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 };
        if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
            return true.into();
        }

        if state.fallback.is_none() {
            state.fallback = Some(hwnd);
        }

        let text_len = unsafe { GetWindowTextLengthW(hwnd) };
        if text_len > 0 {
            let mut buf = vec![0u16; text_len as usize + 1];
            let copied = unsafe { GetWindowTextW(hwnd, &mut buf) };
            if copied > 0 {
                let title = String::from_utf16_lossy(&buf[..copied as usize]);
                if title == "CodexBar" {
                    state.preferred = Some(hwnd);
                    return false.into();
                }
            }
        }

        true.into()
    }

    let mut state = SearchState {
        pid: std::process::id(),
        preferred: None,
        fallback: None,
    };

    unsafe {
        let _ = EnumWindows(
            Some(enum_windows_proc),
            LPARAM((&mut state as *mut SearchState) as isize),
        );
    }

    state.preferred.or(state.fallback)
}

#[cfg(windows)]
fn restore_main_window() {
    use windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, HWND_NOTOPMOST, HWND_TOP, HWND_TOPMOST, IsIconic, SW_RESTORE, SW_SHOW,
        SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SetForegroundWindow, SetWindowPos, ShowWindow,
    };

    unsafe {
        if let Some(hwnd) = find_main_window()
            && !hwnd.is_invalid()
        {
            tracing::debug!("restore_main_window: found hwnd {:?}", hwnd);
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            } else {
                let _ = ShowWindow(hwnd, SW_SHOW);
            }
            // On the 2019 Parallels guest, ShowWindow + SetForegroundWindow alone can
            // leave the window behind Parallels Tools. Nudge the Z-order explicitly first.
            let _ = SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
            );
            let _ = SetWindowPos(
                hwnd,
                HWND_NOTOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
            );
            let _ = SetWindowPos(
                hwnd,
                HWND_TOP,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
            );
            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);
        } else {
            tracing::warn!(
                "restore_main_window: find_main_window returned None — popup may appear behind other windows"
            );
        }
    }
}

#[cfg(windows)]
fn show_main_window_no_focus() {
    use windows::Win32::UI::WindowsAndMessaging::{
        HWND_TOP, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
        SetWindowPos, ShowWindow,
    };

    unsafe {
        if let Some(hwnd) = find_main_window()
            && !hwnd.is_invalid()
        {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            let _ = SetWindowPos(
                hwnd,
                HWND_TOP,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
            );
        }
    }
}

#[cfg(not(windows))]
fn show_main_window_no_focus() {}

#[cfg(not(windows))]
fn restore_main_window() {}

#[cfg(windows)]
fn is_remote_session() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_REMOTESESSION};

    unsafe { GetSystemMetrics(SM_REMOTESESSION) != 0 }
}

#[cfg(windows)]
fn is_ssh_session() -> bool {
    std::env::var_os("SSH_CONNECTION").is_some() || std::env::var_os("SSH_CLIENT").is_some()
}

#[cfg_attr(not(windows), allow(dead_code))]
fn launch_block_reason(is_ssh: bool, is_remote: bool) -> Option<&'static str> {
    if is_ssh {
        Some(ssh_session_error_message())
    } else if is_remote {
        Some(remote_session_error_message())
    } else {
        None
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
fn ssh_session_error_message() -> &'static str {
    "CodexBar can't render its native window from an SSH session on this machine.\n\nOpen it from the logged-in Windows desktop session instead, or use the CLI over SSH:\n\n  codexbar usage -p claude\n\nThe startup log is written to %TEMP%\\codexbar_launch.log."
}

#[cfg_attr(not(windows), allow(dead_code))]
fn remote_session_error_message() -> &'static str {
    "CodexBar can't render its native window inside a Windows Remote Desktop session on this machine.\n\nRun it from the local desktop session instead, or use the CLI while connected over RDP:\n\n  codexbar usage -p claude\n\nThe startup log is written to %TEMP%\\codexbar_launch.log."
}

fn append_launch_log(message: &str) {
    let path = std::env::temp_dir().join("codexbar_launch.log");
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| {
            use std::io::Write;
            writeln!(file, "{}", message)
        });
}

#[cfg(windows)]
fn show_remote_session_error_dialog() {
    use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
    use windows::core::{HSTRING, w};

    let message = HSTRING::from(remote_session_error_message());

    unsafe {
        let _ = MessageBoxW(None, &message, w!("CodexBar"), MB_OK | MB_ICONERROR);
    }
}

fn build_native_options() -> eframe::NativeOptions {
    let force_start_visible = std::env::var_os("CODEXBAR_START_VISIBLE").is_some();
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([360.0, 500.0])
        .with_min_inner_size([320.0, 240.0])
        .with_clamp_size_to_monitor_size(true)
        .with_visible(force_start_visible)
        // Forced-visible startup is for automation/proof flows. Keep the native
        // popup chrome consistent with normal tray-open behavior unless we later
        // detect that the tray is unavailable and explicitly switch to popout.
        .with_resizable(false)
        .with_decorations(false)
        .with_transparent(false)
        .with_taskbar(false)
        .with_always_on_top()
        .with_title("CodexBar");

    eframe::NativeOptions {
        viewport,
        persist_window: false, // Don't persist window state
        ..Default::default()
    }
}

fn startup_popout_mode(force_start_visible: bool, tray_manager_available: bool) -> bool {
    !tray_manager_available && !force_start_visible
}

#[cfg(debug_assertions)]
fn save_color_image_to_png(path: &std::path::Path, image: &egui::ColorImage) -> anyhow::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let mut rgba = Vec::with_capacity(image.pixels.len() * 4);
    for pixel in &image.pixels {
        rgba.extend_from_slice(&pixel.to_srgba_unmultiplied());
    }

    image::save_buffer(
        path,
        &rgba,
        image.size[0] as u32,
        image.size[1] as u32,
        ColorType::Rgba8,
    )?;

    Ok(())
}

#[cfg(debug_assertions)]
#[derive(Clone, Debug)]
struct DebugTabTarget {
    name: String,
    rect: Rect,
    hovered: bool,
    contains_pointer: bool,
    clicked: bool,
    pointer_button_down_on: bool,
    interact_pointer_pos: Option<egui::Pos2>,
}

#[cfg(debug_assertions)]
#[derive(Clone, Copy, Debug)]
struct DebugPointerSnapshot {
    latest_pos: Option<egui::Pos2>,
    interact_pos: Option<egui::Pos2>,
    primary_down: bool,
    primary_pressed: bool,
    primary_released: bool,
    primary_clicked: bool,
}

#[cfg(debug_assertions)]
fn rect_json(rect: Rect) -> String {
    format!(
        "{{\"min_x\":{:.1},\"min_y\":{:.1},\"max_x\":{:.1},\"max_y\":{:.1},\"center_x\":{:.1},\"center_y\":{:.1}}}",
        rect.min.x,
        rect.min.y,
        rect.max.x,
        rect.max.y,
        rect.center().x,
        rect.center().y
    )
}

#[cfg(debug_assertions)]
fn pos_json(pos: Option<egui::Pos2>) -> String {
    pos.map(|pos| format!("{{\"x\":{:.1},\"y\":{:.1}}}", pos.x, pos.y))
        .unwrap_or_else(|| "null".to_string())
}

#[cfg(debug_assertions)]
fn status_message_json(
    status: &Option<super::preferences::PreferencesDebugStatusMessage>,
) -> String {
    status
        .as_ref()
        .map(|status| {
            format!(
                "{{\"message\":\"{}\",\"is_error\":{}}}",
                status.message.replace('\\', "\\\\").replace('\"', "\\\""),
                status.is_error
            )
        })
        .unwrap_or_else(|| "null".to_string())
}

#[cfg(debug_assertions)]
fn string_list_json(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| format!("\"{}\"", value.replace('\\', "\\\\").replace('\"', "\\\"")))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", values)
}

#[cfg(debug_assertions)]
fn string_json(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('\"', "\\\""))
}

#[cfg(debug_assertions)]
fn debug_window_mode(is_popout_mode: bool) -> &'static str {
    if is_popout_mode { "popout" } else { "popup" }
}

#[cfg(test)]
#[test]
fn debug_window_mode_reports_popup_and_popout() {
    assert_eq!(debug_window_mode(false), "popup");
    assert_eq!(debug_window_mode(true), "popout");
}

enum TrayUpdatePlan<'a> {
    Single(&'a ProviderUsage),
    Merged(&'a [ProviderUsage]),
}

#[derive(Debug, Clone)]
enum PerProviderTrayRuntimeState {
    Usage {
        provider_id: ProviderId,
        usage: ProviderUsage,
    },
    Error {
        provider_id: ProviderId,
        error: String,
    },
}

#[derive(Debug, Clone)]
enum TrayRuntimeState {
    SingleUsage(ProviderUsage),
    SingleError {
        provider_name: String,
        error: String,
    },
    MergedUsage(Vec<ProviderUsage>),
    PerProvider(Vec<PerProviderTrayRuntimeState>),
}

fn choose_tray_update_plan<'a>(
    provider_usages: &'a [ProviderUsage],
    settings: &Settings,
) -> Option<TrayUpdatePlan<'a>> {
    if provider_usages.is_empty() {
        return None;
    }

    if settings.tray_icon_mode == TrayIconMode::PerProvider || settings.merge_tray_icons {
        return Some(TrayUpdatePlan::Merged(provider_usages));
    }

    let provider = match (
        settings.menu_bar_shows_highest_usage,
        settings.menu_bar_display_mode.as_str(),
    ) {
        (true, _) | (_, "minimal") => provider_usages.iter().max_by(|a, b| {
            a.session_percent
                .partial_cmp(&b.session_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        _ => provider_usages.first(),
    }?;

    Some(TrayUpdatePlan::Single(provider))
}

fn provider_usage_from_data(provider: &ProviderData, settings: &Settings) -> Option<ProviderUsage> {
    let provider_id = ProviderId::from_cli_name(&provider.name)?;
    let session_percent = provider.session_percent?;
    let metric_pref = settings.get_provider_metric(provider_id);
    let preferred_percent = provider.get_preferred_metric(metric_pref);
    let used_percent = if metric_pref == crate::settings::MetricPreference::Credits {
        100.0 - preferred_percent
    } else {
        preferred_percent
    };
    let weekly_percent = provider.weekly_percent.unwrap_or(session_percent);

    Some(ProviderUsage {
        name: provider.display_name.clone(),
        session_percent: used_percent,
        weekly_percent,
    })
}

fn choose_tray_runtime_state(
    providers: &[ProviderData],
    settings: &Settings,
) -> Option<TrayRuntimeState> {
    match settings.tray_icon_mode {
        TrayIconMode::PerProvider => {
            let states = providers
                .iter()
                .filter_map(|provider| {
                    let provider_id = ProviderId::from_cli_name(&provider.name)?;
                    if let Some(error) = provider.error.as_ref() {
                        Some(PerProviderTrayRuntimeState::Error {
                            provider_id,
                            error: error.clone(),
                        })
                    } else {
                        provider_usage_from_data(provider, settings)
                            .map(|usage| PerProviderTrayRuntimeState::Usage { provider_id, usage })
                    }
                })
                .collect::<Vec<_>>();

            if states.is_empty() {
                None
            } else {
                Some(TrayRuntimeState::PerProvider(states))
            }
        }
        TrayIconMode::Single => {
            if let Some(provider) = providers.iter().find(|provider| provider.error.is_some()) {
                return Some(TrayRuntimeState::SingleError {
                    provider_name: provider.display_name.clone(),
                    error: provider.error.clone().unwrap_or_default(),
                });
            }

            let provider_usages = providers
                .iter()
                .filter_map(|provider| provider_usage_from_data(provider, settings))
                .collect::<Vec<_>>();

            match choose_tray_update_plan(&provider_usages, settings) {
                Some(TrayUpdatePlan::Single(provider)) => {
                    Some(TrayRuntimeState::SingleUsage(provider.clone()))
                }
                Some(TrayUpdatePlan::Merged(usages)) => {
                    Some(TrayRuntimeState::MergedUsage(usages.to_vec()))
                }
                None => None,
            }
        }
    }
}

fn should_recreate_tray_manager(
    previous_enabled_provider_ids: &[ProviderId],
    previous_tray_icon_mode: TrayIconMode,
    settings: &Settings,
) -> bool {
    previous_tray_icon_mode != settings.tray_icon_mode
        || previous_enabled_provider_ids != settings.get_enabled_provider_ids()
}

#[cfg(debug_assertions)]
#[allow(clippy::too_many_arguments)]
fn write_debug_state_with_targets_file(
    path: &std::path::Path,
    selected_tab: &SelectedTab,
    preferences_open: bool,
    preferences_tab: &super::preferences::PreferencesTab,
    tab_targets: &[DebugTabTarget],
    viewport_outer_rect: Option<Rect>,
    preferences_tab_targets: &[DebugTabTarget],
    preferences_viewport_outer_rect: Option<Rect>,
    preferences_settings: &super::preferences::PreferencesDebugSettingsSnapshot,
    tray_state_json: &str,
    is_popout_mode: bool,
    api_key_status: &Option<super::preferences::PreferencesDebugStatusMessage>,
    cookie_status: &Option<super::preferences::PreferencesDebugStatusMessage>,
    pointer_snapshot: DebugPointerSnapshot,
) -> anyhow::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let selected_tab = match selected_tab {
        SelectedTab::Summary => "summary".to_string(),
        SelectedTab::Provider(provider_id) => {
            format!("provider:{}", provider_id.cli_name())
        }
    };
    let preferences_tab = match preferences_tab {
        super::preferences::PreferencesTab::General => "general",
        super::preferences::PreferencesTab::Providers => "providers",
        super::preferences::PreferencesTab::Display => "display",
        super::preferences::PreferencesTab::ApiKeys => "api_keys",
        super::preferences::PreferencesTab::Cookies => "cookies",
        super::preferences::PreferencesTab::Advanced => "advanced",
        super::preferences::PreferencesTab::About => "about",
        super::preferences::PreferencesTab::Accounts => "accounts",
        super::preferences::PreferencesTab::Shortcuts => "shortcuts",
        super::preferences::PreferencesTab::Preferences => "preferences",
    };
    let tab_targets_json = tab_targets
        .iter()
        .map(|target| {
            format!(
                "{{\"name\":\"{}\",\"rect\":{},\"hovered\":{},\"contains_pointer\":{},\"clicked\":{},\"pointer_button_down_on\":{},\"interact_pointer_pos\":{}}}",
                target.name.replace('\\', "\\\\").replace('\"', "\\\""),
                rect_json(target.rect),
                target.hovered,
                target.contains_pointer,
                target.clicked,
                target.pointer_button_down_on,
                pos_json(target.interact_pointer_pos)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let viewport_outer_rect_json = viewport_outer_rect
        .map(rect_json)
        .unwrap_or_else(|| "null".to_string());
    let preferences_tab_targets_json = preferences_tab_targets
        .iter()
        .map(|target| {
            format!(
                "{{\"name\":\"{}\",\"rect\":{},\"hovered\":{},\"contains_pointer\":{},\"clicked\":{},\"pointer_button_down_on\":{},\"interact_pointer_pos\":{}}}",
                target.name.replace('\\', "\\\\").replace('\"', "\\\""),
                rect_json(target.rect),
                target.hovered,
                target.contains_pointer,
                target.clicked,
                target.pointer_button_down_on,
                pos_json(target.interact_pointer_pos)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let preferences_viewport_outer_rect_json = preferences_viewport_outer_rect
        .map(rect_json)
        .unwrap_or_else(|| "null".to_string());
    let enabled_providers_json = string_list_json(&preferences_settings.enabled_providers);
    let api_key_status_json = status_message_json(api_key_status);
    let cookie_status_json = status_message_json(cookie_status);
    let pointer_snapshot_json = format!(
        "{{\"latest_pos\":{},\"interact_pos\":{},\"primary_down\":{},\"primary_pressed\":{},\"primary_released\":{},\"primary_clicked\":{}}}",
        pos_json(pointer_snapshot.latest_pos),
        pos_json(pointer_snapshot.interact_pos),
        pointer_snapshot.primary_down,
        pointer_snapshot.primary_pressed,
        pointer_snapshot.primary_released,
        pointer_snapshot.primary_clicked
    );

    let payload = format!(
        "{{\"selected_tab\":\"{}\",\"preferences_open\":{},\"preferences_tab\":\"{}\",\"viewport_outer_rect\":{},\"preferences_viewport_outer_rect\":{},\"enabled_providers\":{},\"refresh_interval_secs\":{},\"menu_bar_display_mode\":{},\"reset_time_relative\":{},\"surprise_animations\":{},\"show_as_used\":{},\"show_credits_extra_usage\":{},\"merge_tray_icons\":{},\"switcher_shows_icons\":{},\"menu_bar_shows_highest_usage\":{},\"menu_bar_shows_percent\":{},\"show_all_token_accounts_in_menu\":{},\"tray_icon_mode\":{},\"window_mode\":{},\"selected_provider\":{},\"tray_state\":{},\"api_key_status\":{},\"cookie_status\":{},\"pointer\":{},\"tab_targets\":[{}],\"preferences_tab_targets\":[{}]}}\n",
        selected_tab.replace('\\', "\\\\").replace('\"', "\\\""),
        preferences_open,
        preferences_tab,
        viewport_outer_rect_json,
        preferences_viewport_outer_rect_json,
        enabled_providers_json,
        preferences_settings.refresh_interval_secs,
        string_json(&preferences_settings.menu_bar_display_mode),
        preferences_settings.reset_time_relative,
        preferences_settings.surprise_animations,
        preferences_settings.show_as_used,
        preferences_settings.show_credits_extra_usage,
        preferences_settings.merge_tray_icons,
        preferences_settings.switcher_shows_icons,
        preferences_settings.menu_bar_shows_highest_usage,
        preferences_settings.menu_bar_shows_percent,
        preferences_settings.show_all_token_accounts_in_menu,
        string_json(&preferences_settings.tray_icon_mode),
        string_json(debug_window_mode(is_popout_mode)),
        preferences_settings
            .selected_provider
            .as_ref()
            .map(|provider| string_json(provider))
            .unwrap_or_else(|| "null".to_string()),
        tray_state_json,
        api_key_status_json,
        cookie_status_json,
        pointer_snapshot_json,
        tab_targets_json,
        preferences_tab_targets_json
    );
    std::fs::write(path, payload)?;
    Ok(())
}

#[derive(Clone, Debug)]
pub struct ProviderData {
    pub name: String,
    pub display_name: String,
    pub account: Option<String>, // Account email for display
    pub session_percent: Option<f64>,
    pub session_reset: Option<String>,
    pub weekly_percent: Option<f64>,
    pub weekly_reset: Option<String>,
    pub monthly_percent: Option<f64>, // Tertiary (30-day) usage for Infini
    pub monthly_reset: Option<String>,
    pub model_percent: Option<f64>,
    pub model_name: Option<String>,
    pub plan: Option<String>,
    pub error: Option<String>,
    pub dashboard_url: Option<String>,
    pub pace_percent: Option<f64>,
    pub pace_lasts_to_reset: bool,
    pub cost_used: Option<String>,
    pub credits_remaining: Option<f64>,
    pub credits_percent: Option<f64>,
    pub status_level: StatusLevel,
    pub status_description: Option<String>,
    pub cost_history: Vec<(String, f64)>,
    pub credits_history: Vec<(String, f64)>,
    pub usage_breakdown: Vec<UsageBreakdownPoint>,
}

impl ProviderData {
    fn placeholder(id: ProviderId) -> Self {
        Self {
            name: id.cli_name().to_string(),
            display_name: id.display_name().to_string(),
            account: None,
            session_percent: None,
            session_reset: None,
            weekly_percent: None,
            weekly_reset: None,
            monthly_percent: None,
            monthly_reset: None,
            model_percent: None,
            model_name: None,
            plan: None,
            error: None,
            dashboard_url: None,
            pace_percent: None,
            pace_lasts_to_reset: false,
            cost_used: None,
            credits_remaining: None,
            credits_percent: None,
            status_level: StatusLevel::Unknown,
            status_description: None,
            cost_history: Vec::new(),
            credits_history: Vec::new(),
            usage_breakdown: Vec::new(),
        }
    }

    fn from_result(
        id: ProviderId,
        result: &ProviderFetchResult,
        metadata: &crate::core::ProviderMetadata,
        reset_time_relative: bool,
        ui_language: Language,
    ) -> Self {
        let snapshot = &result.usage;
        let (pace_percent, pace_lasts) = calculate_pace(&snapshot.primary);

        let (cost_used, credits_remaining, credits_percent) = if let Some(ref cost) = result.cost {
            if cost.period == "Credits" {
                // Use the limit from the cost snapshot if available, otherwise default to 1000
                let scale = cost.limit.unwrap_or(1000.0);
                let remaining = cost.used;
                let percent = if scale > 0.0 {
                    (remaining / scale * 100.0).clamp(0.0, 100.0)
                } else {
                    0.0
                };
                (None, Some(remaining), Some(percent))
            } else {
                (Some(cost.format_used()), None, None)
            }
        } else {
            (None, None, None)
        };

        Self {
            name: id.cli_name().to_string(),
            display_name: id.display_name().to_string(),
            account: snapshot.account_email.clone(), // Account email if available
            session_percent: Some(snapshot.primary.used_percent),
            session_reset: snapshot
                .primary
                .resets_at
                .map(|t| format_reset_time(t, reset_time_relative, ui_language)),
            weekly_percent: snapshot.secondary.as_ref().map(|s| s.used_percent),
            weekly_reset: snapshot.secondary.as_ref().and_then(|s| {
                s.resets_at
                    .map(|t| format_reset_time(t, reset_time_relative, ui_language))
            }),
            monthly_percent: snapshot.tertiary.as_ref().map(|t| t.used_percent),
            monthly_reset: snapshot.tertiary.as_ref().and_then(|t| {
                t.resets_at
                    .map(|r| format_reset_time(r, reset_time_relative, ui_language))
            }),
            model_percent: snapshot.model_specific.as_ref().map(|m| m.used_percent),
            model_name: snapshot
                .model_specific
                .as_ref()
                .and_then(|m| m.reset_description.clone()),
            plan: snapshot.login_method.clone(),
            error: None,
            dashboard_url: metadata.dashboard_url.map(|s| s.to_string()),
            pace_percent,
            pace_lasts_to_reset: pace_lasts,
            cost_used,
            credits_remaining,
            credits_percent,
            status_level: StatusLevel::Unknown,
            status_description: None,
            cost_history: Vec::new(),
            credits_history: Vec::new(),
            usage_breakdown: Vec::new(),
        }
    }

    fn from_error(id: ProviderId, error: String) -> Self {
        Self {
            name: id.cli_name().to_string(),
            display_name: id.display_name().to_string(),
            account: None,
            session_percent: None,
            session_reset: None,
            weekly_percent: None,
            weekly_reset: None,
            monthly_percent: None,
            monthly_reset: None,
            model_percent: None,
            model_name: None,
            plan: None,
            error: Some(error),
            dashboard_url: None,
            pace_percent: None,
            pace_lasts_to_reset: false,
            cost_used: None,
            credits_remaining: None,
            credits_percent: None,
            status_level: StatusLevel::Unknown,
            status_description: None,
            cost_history: Vec::new(),
            credits_history: Vec::new(),
            usage_breakdown: Vec::new(),
        }
    }

    /// Get the preferred metric percent based on the MetricPreference setting
    pub fn get_preferred_metric(&self, pref: crate::settings::MetricPreference) -> f64 {
        match pref {
            crate::settings::MetricPreference::Session => self.session_percent.unwrap_or(0.0),
            crate::settings::MetricPreference::Weekly => self
                .weekly_percent
                .unwrap_or_else(|| self.session_percent.unwrap_or(0.0)),
            crate::settings::MetricPreference::Model => self
                .model_percent
                .unwrap_or_else(|| self.session_percent.unwrap_or(0.0)),
            crate::settings::MetricPreference::Credits => {
                // For credits, we show the credits_percent (remaining as percentage of full scale)
                self.credits_percent
                    .unwrap_or_else(|| self.session_percent.unwrap_or(0.0))
            }
            crate::settings::MetricPreference::Average => {
                // Average of all available metrics
                let mut sum = 0.0;
                let mut count = 0;
                if let Some(v) = self.session_percent {
                    sum += v;
                    count += 1;
                }
                if let Some(v) = self.weekly_percent {
                    sum += v;
                    count += 1;
                }
                if let Some(v) = self.model_percent {
                    sum += v;
                    count += 1;
                }
                if count > 0 { sum / count as f64 } else { 0.0 }
            }
            crate::settings::MetricPreference::Automatic => {
                // Automatic: prefer the highest available metric (most concerning)
                let session = self.session_percent.unwrap_or(0.0);
                let weekly = self.weekly_percent.unwrap_or(0.0);
                let model = self.model_percent.unwrap_or(0.0);
                session.max(weekly).max(model)
            }
        }
    }
}

fn should_show_provider(provider: &ProviderData) -> bool {
    provider.session_percent.is_some()
        || provider.weekly_percent.is_some()
        || provider.monthly_percent.is_some()
        || provider.model_percent.is_some()
        || provider.error.is_some()
}

fn main_window_summary_height(visible_provider_count: usize, has_enabled_provider: bool) -> f32 {
    if visible_provider_count > 0 {
        (164.0 + visible_provider_count as f32 * 54.0).clamp(270.0, 500.0)
    } else if has_enabled_provider {
        220.0
    } else {
        340.0
    }
}

fn format_reset_time(
    reset: chrono::DateTime<chrono::Utc>,
    relative: bool,
    lang: Language,
) -> String {
    if relative {
        let now = chrono::Utc::now();
        let diff = reset - now;

        if diff.num_seconds() <= 0 {
            return locale_text(lang, LocaleKey::ResetInProgress).to_string();
        }

        let hours = diff.num_hours();
        let minutes = (diff.num_minutes() % 60).abs();

        if hours >= 24 {
            let days = hours / 24;
            let remaining_hours = hours % 24;
            format!("{}d {}h", days, remaining_hours)
        } else {
            format!("{}h {}m", hours, minutes)
        }
    } else {
        // Absolute time format using local timezone
        // Include date if not today
        use chrono::Local;
        let local_time = reset.with_timezone(&Local);
        let today = Local::now().date_naive();
        let reset_date = local_time.date_naive();

        if reset_date == today {
            local_time.format("%I:%M %p").to_string()
        } else if reset_date == today + chrono::Days::new(1) {
            let tomorrow_template = locale_text(lang, LocaleKey::TomorrowAt);
            tomorrow_template
                .replace("{}", &local_time.format("%I:%M %p").to_string())
                .to_string()
        } else {
            local_time.format("%b %d, %I:%M %p").to_string()
        }
    }
}

fn calculate_pace(rate_window: &RateWindow) -> (Option<f64>, bool) {
    let Some(window_minutes) = rate_window.window_minutes else {
        return (None, false);
    };
    let Some(resets_at) = rate_window.resets_at else {
        return (None, false);
    };

    let now = chrono::Utc::now();
    let time_remaining = resets_at - now;
    let remaining_minutes = time_remaining.num_minutes() as f64;

    if remaining_minutes <= 0.0 {
        return (None, false);
    }

    let total_minutes = window_minutes as f64;
    let elapsed_minutes = total_minutes - remaining_minutes;

    if elapsed_minutes <= 0.0 {
        return (None, false);
    }

    let expected_percent = (elapsed_minutes / total_minutes) * 100.0;
    let actual_percent = rate_window.used_percent;
    let pace = actual_percent - expected_percent;
    let lasts_to_reset = actual_percent <= expected_percent;

    (Some(pace), lasts_to_reset)
}

fn usage_display_percent(used_percent: f64, show_as_used: bool) -> f64 {
    let used_percent = used_percent.clamp(0.0, 100.0);
    if show_as_used {
        used_percent
    } else {
        100.0 - used_percent
    }
}

fn usage_display_label(display_percent: f64, show_as_used: bool, lang: Language) -> String {
    if show_as_used {
        locale_text(lang, LocaleKey::UsedPercent)
            .replace("{:.0}", &format!("{:.0}", display_percent))
    } else {
        locale_text(lang, LocaleKey::RemainingPercent)
            .replace("{:.0}", &format!("{:.0}", display_percent))
    }
}

fn load_usage_breakdown_points(
    provider_id: ProviderId,
    account_email: Option<&str>,
) -> Vec<UsageBreakdownPoint> {
    if provider_id != ProviderId::Codex {
        return Vec::new();
    }

    // Require account_email to validate cache belongs to current account
    // Without it, we risk showing stale data from a different account
    let Some(account_email) = account_email else {
        return Vec::new();
    };

    let Some(cache) = OpenAIDashboardCacheStore::load() else {
        return Vec::new();
    };

    // Verify cache belongs to current account
    if !cache.account_email.eq_ignore_ascii_case(account_email) {
        return Vec::new();
    }

    cache
        .snapshot
        .usage_breakdown
        .iter()
        .map(|point| {
            let services = point
                .services
                .iter()
                .map(|service| ServiceUsage {
                    service: service.service.clone(),
                    credits_used: service.credits_used,
                })
                .collect();
            UsageBreakdownPoint::new(point.day.clone(), services)
        })
        .collect()
}

fn load_credits_history_points(
    provider_id: ProviderId,
    account_email: Option<&str>,
) -> Vec<(String, f64)> {
    if provider_id != ProviderId::Codex {
        return Vec::new();
    }

    let Some(account_email) = account_email else {
        return Vec::new();
    };

    let Some(cache) = OpenAIDashboardCacheStore::load() else {
        return Vec::new();
    };

    if !cache.account_email.eq_ignore_ascii_case(account_email) {
        return Vec::new();
    }

    let breakdown = if !cache.snapshot.daily_breakdown.is_empty() {
        &cache.snapshot.daily_breakdown
    } else if !cache.snapshot.usage_breakdown.is_empty() {
        &cache.snapshot.usage_breakdown
    } else {
        return Vec::new();
    };

    let mut points: Vec<(String, f64)> = breakdown
        .iter()
        .map(|day| (day.day.clone(), day.total_credits_used))
        .collect();

    points.sort_by(|a, b| a.0.cmp(&b.0));
    points
}

fn random_surprise_delay() -> Duration {
    use rand::Rng;
    let mut rng = rand::rng();
    let secs = rng.random_range(30..300);
    Duration::from_secs(secs)
}

struct SharedState {
    providers: Vec<ProviderData>,
    summary_providers: Vec<ProviderData>,
    selected_tab: SelectedTab,
    last_refresh: Instant,
    initial_refresh_pending: bool,
    is_refreshing: bool,
    loading_pattern: LoadingPattern,
    loading_phase: f64,
    surprise_animation: Option<SurpriseAnimation>,
    surprise_frame: u32,
    next_surprise_time: Instant,
    update_available: Option<UpdateInfo>,
    update_checked: bool,
    update_dismissed: bool,
    update_check_in_progress: bool,
    update_state: UpdateState,
    login_provider: Option<String>,
    login_phase: LoginPhase,
    login_message: Option<String>,
}

#[cfg(debug_assertions)]
#[derive(Clone, Debug)]
enum DebugTrayOverride {
    Single {
        state: String,
        provider_name: Option<String>,
        session_percent: Option<f64>,
        weekly_percent: Option<f64>,
        error: Option<String>,
    },
    PerProvider(HashMap<ProviderId, DebugProviderTrayOverride>),
}

#[cfg(debug_assertions)]
#[derive(Clone, Debug)]
struct DebugProviderTrayOverride {
    state: String,
    session_percent: Option<f64>,
    weekly_percent: Option<f64>,
    error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelectedTab {
    Summary,
    Provider(ProviderId),
}

fn open_update_destination(update: &UpdateInfo) {
    let _ = open::that(&update.download_url);
}

fn startup_last_refresh(now: Instant) -> Instant {
    now
}

fn should_auto_refresh(
    initial_refresh_pending: bool,
    is_refreshing: bool,
    last_refresh: Instant,
    refresh_interval_secs: u64,
    now: Instant,
) -> bool {
    if refresh_interval_secs == 0 || is_refreshing {
        return false;
    }

    if initial_refresh_pending {
        return true;
    }

    now.checked_duration_since(last_refresh)
        .is_some_and(|elapsed| elapsed > Duration::from_secs(refresh_interval_secs))
}

fn start_update_download(state: Arc<Mutex<SharedState>>, update: UpdateInfo) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!("Failed to create runtime for update download: {}", e);
                if let Ok(mut s) = state.lock() {
                    s.update_state = UpdateState::Failed(e.to_string());
                }
                return;
            }
        };

        rt.block_on(async move {
            if let Ok(mut s) = state.lock() {
                s.update_state = UpdateState::Downloading(0.0);
            }
            let (progress_tx, _) = tokio::sync::watch::channel(UpdateState::Available);
            match updater::download_update(&update, progress_tx).await {
                Ok(path) => {
                    if let Ok(mut s) = state.lock() {
                        s.update_state = UpdateState::Ready(path);
                    }
                }
                Err(e) => {
                    if let Ok(mut s) = state.lock() {
                        s.update_state = UpdateState::Failed(e);
                    }
                }
            }
        });
    });
}

fn start_update_check(
    state: Arc<Mutex<SharedState>>,
    update_channel: UpdateChannel,
    auto_download: bool,
) {
    if let Ok(mut s) = state.lock() {
        if s.update_check_in_progress {
            return;
        }
        s.update_check_in_progress = true;
    } else {
        return;
    }

    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!("Failed to create tokio runtime for update check: {}", e);
                if let Ok(mut s) = state.lock() {
                    s.update_check_in_progress = false;
                }
                return;
            }
        };

        rt.block_on(async move {
            if let Some(update) = updater::check_for_updates_with_channel(update_channel).await {
                let should_download = {
                    if let Ok(mut s) = state.lock() {
                        s.update_available = Some(update.clone());
                        s.update_checked = true;
                        s.update_dismissed = false;
                        s.update_state = UpdateState::Available;
                        auto_download && update.supports_auto_download()
                    } else {
                        false
                    }
                };

                if should_download {
                    let (progress_tx, mut progress_rx) =
                        tokio::sync::watch::channel(UpdateState::Available);
                    let state_clone = Arc::clone(&state);

                    if let Ok(mut s) = state_clone.lock() {
                        s.update_state = UpdateState::Downloading(0.0);
                    }

                    let progress_state = Arc::clone(&state_clone);
                    tokio::spawn(async move {
                        while progress_rx.changed().await.is_ok() {
                            let new_state = progress_rx.borrow().clone();
                            if let Ok(mut s) = progress_state.lock() {
                                s.update_state = new_state;
                            }
                        }
                    });

                    match updater::download_update(&update, progress_tx).await {
                        Ok(path) => {
                            if let Ok(mut s) = state_clone.lock() {
                                s.update_state = UpdateState::Ready(path);
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to download update: {}", e);
                            if let Ok(mut s) = state_clone.lock() {
                                s.update_state = UpdateState::Failed(e);
                            }
                        }
                    }
                }
            } else if let Ok(mut s) = state.lock() {
                s.update_checked = true;
            }

            if let Ok(mut s) = state.lock() {
                s.update_check_in_progress = false;
            }
        });
    });
}

pub struct CodexBarApp {
    state: Arc<Mutex<SharedState>>,
    settings: Settings,
    tray_manager: Option<UnifiedTrayManager>,
    tray_action_rx: Option<Receiver<TrayMenuAction>>,
    preferences_window: PreferencesWindow,
    shortcut_manager: Option<ShortcutManager>,
    icon_cache: ProviderIconCache,
    was_refreshing: bool, // Track previous frame's refresh state
    pending_main_window_layout: bool,
    anchor_main_window_to_pointer: bool,
    /// When set, the next layout_main_window call will anchor the popup
    /// to this physical-pixel position (typically the tray icon location).
    tray_anchor_pos: Option<(i32, i32)>,
    /// Whether the window is in popout mode (decorated/resizable) vs tray panel mode (borderless/fixed).
    is_popout_mode: bool,
    notification_manager: Arc<Mutex<NotificationManager>>,
    #[cfg(debug_assertions)]
    test_input_queue: super::test_server::TestInputQueue,
    #[cfg(debug_assertions)]
    pending_test_screenshot_path: Option<PathBuf>,
    #[cfg(debug_assertions)]
    pending_test_screenshot_delay_frames: u8,
    #[cfg(debug_assertions)]
    pending_test_state_dump_path: Option<PathBuf>,
    #[cfg(debug_assertions)]
    pending_test_state_dump_delay_frames: u8,
    #[cfg(debug_assertions)]
    pending_test_event_batches: VecDeque<Vec<egui::Event>>,
    #[cfg(debug_assertions)]
    debug_tab_targets: Vec<DebugTabTarget>,
    #[cfg(debug_assertions)]
    debug_viewport_outer_rect: Option<Rect>,
    #[cfg(debug_assertions)]
    last_debug_tab_targets: Vec<DebugTabTarget>,
    #[cfg(debug_assertions)]
    last_debug_viewport_outer_rect: Option<Rect>,
    #[cfg(debug_assertions)]
    last_debug_pointer_snapshot: DebugPointerSnapshot,
    #[cfg(debug_assertions)]
    latched_debug_tab_targets: Vec<DebugTabTarget>,
    #[cfg(debug_assertions)]
    latched_debug_viewport_outer_rect: Option<Rect>,
    #[cfg(debug_assertions)]
    latched_debug_pointer_snapshot: DebugPointerSnapshot,
    #[cfg(debug_assertions)]
    debug_tray_override: Option<DebugTrayOverride>,
    first_update_logged: bool,
}

#[cfg(any(test, not(windows)))]
fn prepend_font(fonts: &mut FontDefinitions, family: FontFamily, font_name: &str) {
    let entries = fonts.families.entry(family).or_default();
    if !entries.iter().any(|existing| existing == font_name) {
        entries.insert(0, font_name.to_owned());
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
fn append_font(fonts: &mut FontDefinitions, family: FontFamily, font_name: &str) {
    let entries = fonts.families.entry(family).or_default();
    if !entries.iter().any(|existing| existing == font_name) {
        entries.push(font_name.to_owned());
    }
}

#[cfg(not(windows))]
fn add_font_if_present(fonts: &mut FontDefinitions, font_name: &str, path: &str) {
    if let Ok(font_data) = std::fs::read(path) {
        fonts
            .font_data
            .insert(font_name.to_owned(), FontData::from_owned(font_data).into());
        prepend_font(fonts, FontFamily::Proportional, font_name);
        prepend_font(fonts, FontFamily::Monospace, font_name);
    }
}

#[cfg(windows)]
fn add_font_fallback_if_present(fonts: &mut FontDefinitions, font_name: &str, path: &str) {
    if let Ok(font_data) = std::fs::read(path) {
        fonts
            .font_data
            .insert(font_name.to_owned(), FontData::from_owned(font_data).into());
        append_font(fonts, FontFamily::Proportional, font_name);
        append_font(fonts, FontFamily::Monospace, font_name);
    }
}

#[cfg(windows)]
fn cjk_font_candidates() -> &'static [(&'static str, &'static str)] {
    &[
        ("msyh", "C:\\Windows\\Fonts\\msyh.ttc"),
        ("msyhbd", "C:\\Windows\\Fonts\\msyhbd.ttc"),
        ("simsun", "C:\\Windows\\Fonts\\simsun.ttc"),
        ("simhei", "C:\\Windows\\Fonts\\simhei.ttf"),
        ("deng", "C:\\Windows\\Fonts\\Deng.ttf"),
        (
            "wqy_zenhei",
            "Z:\\usr\\share\\fonts\\truetype\\wqy\\wqy-zenhei.ttc",
        ),
        (
            "droid_fallback",
            "Z:\\usr\\share\\fonts\\truetype\\droid\\DroidSansFallbackFull.ttf",
        ),
    ]
}

#[cfg(not(windows))]
fn cjk_font_candidates() -> &'static [(&'static str, &'static str)] {
    &[
        ("wqy_zenhei", "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc"),
        (
            "droid_fallback",
            "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
        ),
    ]
}

impl CodexBarApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load Windows symbol + CJK fallback fonts so Chinese UI text renders correctly.
        let mut fonts = FontDefinitions::default();
        if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\seguisym.ttf") {
            fonts.font_data.insert(
                "segoe_symbols".to_owned(),
                FontData::from_owned(font_data).into(),
            );
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .push("segoe_symbols".to_owned());
        }

        // Keep CJK fonts as fallbacks on Windows so Latin text stays on the
        // default UI stack while Chinese glyphs still render when needed.
        #[cfg(windows)]
        for (name, path) in cjk_font_candidates() {
            add_font_fallback_if_present(&mut fonts, name, path);
        }
        #[cfg(not(windows))]
        for (name, path) in cjk_font_candidates() {
            add_font_if_present(&mut fonts, name, path);
        }
        cc.egui_ctx.set_fonts(fonts);

        let settings = Settings::load();
        let enabled_ids = settings.get_enabled_provider_ids();

        let placeholders: Vec<ProviderData> = enabled_ids
            .iter()
            .map(|&id| ProviderData::placeholder(id))
            .collect();

        let state = Arc::new(Mutex::new(SharedState {
            providers: placeholders.clone(),
            summary_providers: placeholders,
            selected_tab: SelectedTab::Summary,
            last_refresh: startup_last_refresh(Instant::now()),
            initial_refresh_pending: true,
            is_refreshing: false,
            loading_pattern: LoadingPattern::random(),
            loading_phase: 0.0,
            surprise_animation: None,
            surprise_frame: 0,
            next_surprise_time: Instant::now() + random_surprise_delay(),
            update_available: None,
            update_checked: false,
            update_dismissed: false,
            update_check_in_progress: false,
            update_state: UpdateState::Idle,
            login_provider: None,
            login_phase: LoginPhase::Idle,
            login_message: None,
        }));

        // Initialize system tray based on settings
        let tray_manager = match UnifiedTrayManager::new(&settings) {
            Ok(tm) => Some(tm),
            Err(e) => {
                tracing::warn!("Failed to create tray manager: {}", e);
                None
            }
        };
        let tray_action_rx = if tray_manager.is_some() {
            let (tx, rx) = mpsc::channel::<TrayMenuAction>();
            let repaint_ctx = cc.egui_ctx.clone();
            std::thread::spawn(move || {
                loop {
                    let action = UnifiedTrayManager::check_events()
                        .or_else(UnifiedTrayManager::check_tray_click_events);
                    if let Some(action) = action {
                        if matches!(
                            action,
                            TrayMenuAction::PopOut
                                | TrayMenuAction::PopOutProvider(_)
                                | TrayMenuAction::TrayLeftClick { .. }
                        ) {
                            // Egui viewport commands alone can be ignored while minimized.
                            // Force a native restore first so the update loop wakes up.
                            restore_main_window();
                            repaint_ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                            repaint_ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        } else if matches!(
                            action,
                            TrayMenuAction::Settings | TrayMenuAction::TrayRightClick
                        ) {
                            // Show main window so update() runs, but don't steal focus.
                            show_main_window_no_focus();
                            repaint_ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        }
                        if tx.send(action).is_err() {
                            break;
                        }
                        repaint_ctx.request_repaint();
                    } else {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                }
            });
            Some(rx)
        } else {
            None
        };

        // Check for updates in background (using configured update channel)
        start_update_check(
            Arc::clone(&state),
            settings.update_channel,
            settings.auto_download_updates,
        );

        // Initialize keyboard shortcuts with custom shortcut from settings
        let shortcut_manager = match ShortcutManager::new() {
            Ok(mut sm) => {
                // Apply custom shortcut from settings if configured
                if let Some((modifiers, key)) = parse_shortcut(&settings.global_shortcut) {
                    if let Err(e) = sm.set_open_menu_shortcut(modifiers, key) {
                        tracing::warn!(
                            "Failed to set custom shortcut '{}': {}",
                            settings.global_shortcut,
                            e
                        );
                    } else {
                        tracing::info!(
                            "Keyboard shortcut registered: {}",
                            settings.global_shortcut
                        );
                    }
                } else {
                    tracing::info!("Keyboard shortcut registered: Ctrl+Shift+U (default)");
                }
                Some(sm)
            }
            Err(e) => {
                tracing::warn!("Failed to register keyboard shortcuts: {}", e);
                None
            }
        };

        let force_start_visible = std::env::var_os("CODEXBAR_START_VISIBLE").is_some();
        let tray_manager_available = tray_manager.is_some();
        let start_visible_without_tray = force_start_visible || !tray_manager_available;
        let start_in_popout_mode = startup_popout_mode(force_start_visible, tray_manager_available);

        // Initialize test input queue and start server (debug builds only)
        #[cfg(debug_assertions)]
        let test_input_queue = {
            let q = super::test_server::create_queue();
            super::test_server::start_server(q.clone(), cc.egui_ctx.clone());
            q
        };

        if start_visible_without_tray {
            if force_start_visible {
                tracing::info!("CODEXBAR_START_VISIBLE is set; opening main window at startup");
                append_launch_log("CodexBarApp::new forced visible via CODEXBAR_START_VISIBLE");
            } else {
                tracing::warn!(
                    "Tray manager unavailable; opening main window so the app remains reachable"
                );
                append_launch_log("CodexBarApp::new tray unavailable; opening main window");
            }
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::Visible(true));
        } else {
            // Tray-first: window starts hidden in both debug and release.
            // Automation/test flows use the test server's OpenWindow command to show it.
            append_launch_log("CodexBarApp::new startup hidden to tray");
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        append_launch_log("CodexBarApp::new completed");

        Self {
            state,
            settings,
            tray_manager,
            tray_action_rx,
            preferences_window: PreferencesWindow::new(),
            shortcut_manager,
            icon_cache: ProviderIconCache::new(),
            was_refreshing: false,
            pending_main_window_layout: start_visible_without_tray,
            anchor_main_window_to_pointer: false,
            tray_anchor_pos: None,
            is_popout_mode: start_in_popout_mode,
            notification_manager: Arc::new(Mutex::new(NotificationManager::new())),
            #[cfg(debug_assertions)]
            test_input_queue,
            #[cfg(debug_assertions)]
            pending_test_screenshot_path: None,
            #[cfg(debug_assertions)]
            pending_test_screenshot_delay_frames: 0,
            #[cfg(debug_assertions)]
            pending_test_state_dump_path: None,
            #[cfg(debug_assertions)]
            pending_test_state_dump_delay_frames: 0,
            #[cfg(debug_assertions)]
            pending_test_event_batches: VecDeque::new(),
            #[cfg(debug_assertions)]
            debug_tab_targets: Vec::new(),
            #[cfg(debug_assertions)]
            debug_viewport_outer_rect: None,
            #[cfg(debug_assertions)]
            last_debug_tab_targets: Vec::new(),
            #[cfg(debug_assertions)]
            last_debug_viewport_outer_rect: None,
            #[cfg(debug_assertions)]
            last_debug_pointer_snapshot: DebugPointerSnapshot {
                latest_pos: None,
                interact_pos: None,
                primary_down: false,
                primary_pressed: false,
                primary_released: false,
                primary_clicked: false,
            },
            #[cfg(debug_assertions)]
            latched_debug_tab_targets: Vec::new(),
            #[cfg(debug_assertions)]
            latched_debug_viewport_outer_rect: None,
            #[cfg(debug_assertions)]
            latched_debug_pointer_snapshot: DebugPointerSnapshot {
                latest_pos: None,
                interact_pos: None,
                primary_down: false,
                primary_pressed: false,
                primary_released: false,
                primary_clicked: false,
            },
            #[cfg(debug_assertions)]
            debug_tray_override: None,
            first_update_logged: false,
        }
    }

    fn pending_update_to_install_on_quit(&self) -> Option<PathBuf> {
        let effective_settings = self.preferences_window.current_settings();
        if !effective_settings.install_updates_on_quit {
            return None;
        }

        let ready_update = self.state.lock().ok().and_then(|state| {
            if let UpdateState::Ready(path) = &state.update_state {
                Some(path.clone())
            } else {
                None
            }
        });

        ready_update.or_else(updater::get_pending_update)
    }

    fn quit_application(&self) -> ! {
        if let Some(installer_path) = self.pending_update_to_install_on_quit()
            && let Err(e) = updater::apply_update(&installer_path)
        {
            tracing::error!("Failed to apply pending update on quit: {}", e);
        }

        std::process::exit(0);
    }

    #[cfg(debug_assertions)]
    fn open_main_window_for_testing(&mut self, ctx: &egui::Context) {
        tracing::debug!("Opening main window via test server");
        self.is_popout_mode = true;
        restore_main_window();
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.request_repaint();
        self.pending_main_window_layout = true;
        self.anchor_main_window_to_pointer = true;
    }

    /// Exercises the same code-path as a real tray left-click: restore → position
    /// near tray → focus.  Unlike `open_main_window_for_testing` this uses the
    /// `TrayLeftClick` handler so proof scripts can verify the hybrid popup flow.
    #[cfg(debug_assertions)]
    fn simulate_tray_left_click(&mut self, ctx: &egui::Context) {
        // Resolve the tray icon rect for realistic anchor coordinates.
        let (tray_x, tray_y) = self
            .tray_manager
            .as_ref()
            .and_then(|tm| tm.rect())
            .map(|r| {
                (
                    r.position.x as i32 + r.size.width as i32 / 2,
                    r.position.y as i32,
                )
            })
            .unwrap_or((0, 0));

        tracing::info!(
            "simulate_tray_left_click: tray_x={}, tray_y={}",
            tray_x,
            tray_y
        );

        // Mirror the real tray-click polling-thread path: restore window first
        // so that SetForegroundWindow succeeds while we still own input focus.
        restore_main_window();
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));

        // Then run the same handler that update() uses for TrayLeftClick.
        self.is_popout_mode = false;
        if let Ok(mut state) = self.state.lock() {
            state.selected_tab = SelectedTab::Summary;
        }
        self.tray_anchor_pos = Some((tray_x, tray_y));
        self.pending_main_window_layout = true;
        self.anchor_main_window_to_pointer = false;
        self.layout_main_window(ctx, false);
    }

    #[cfg(debug_assertions)]
    fn request_test_screenshot(&mut self, path: PathBuf, ctx: &egui::Context) {
        tracing::debug!("Scheduling test screenshot for {}", path.display());
        self.open_main_window_for_testing(ctx);
        self.pending_test_screenshot_path = Some(path);
        self.pending_test_screenshot_delay_frames = 1;
    }

    #[cfg(debug_assertions)]
    fn request_test_preferences_screenshot(&mut self, path: PathBuf, ctx: &egui::Context) {
        tracing::debug!(
            "Scheduling preferences test screenshot for {}",
            path.display()
        );
        self.preferences_window.request_screenshot_for_testing(path);
        ctx.request_repaint();
    }

    #[cfg(debug_assertions)]
    fn request_test_state_dump(&mut self, path: PathBuf, ctx: &egui::Context) {
        tracing::debug!("Scheduling test state dump for {}", path.display());
        self.open_main_window_for_testing(ctx);
        self.pending_test_state_dump_path = Some(path);
        self.pending_test_state_dump_delay_frames = 1;
    }

    #[cfg(debug_assertions)]
    fn select_tab_for_testing(&mut self, tab: &str, ctx: &egui::Context) {
        let normalized = tab.trim().to_ascii_lowercase();
        if let Ok(mut state) = self.state.lock() {
            if normalized == "summary" {
                state.selected_tab = SelectedTab::Summary;
            } else if let Some(provider_id) = ProviderId::from_cli_name(&normalized) {
                state.selected_tab = SelectedTab::Provider(provider_id);
            } else {
                tracing::warn!("Unknown test tab selection: {}", tab);
                return;
            }
        }
        self.open_main_window_for_testing(ctx);
    }

    #[cfg(debug_assertions)]
    fn set_single_tray_state_for_testing(
        &mut self,
        state: &str,
        provider_name: Option<&str>,
        session_percent: Option<f64>,
        weekly_percent: Option<f64>,
        error: Option<&str>,
    ) {
        self.debug_tray_override = Some(DebugTrayOverride::Single {
            state: state.to_string(),
            provider_name: provider_name.map(str::to_string),
            session_percent,
            weekly_percent,
            error: error.map(str::to_string),
        });
    }

    #[cfg(debug_assertions)]
    fn set_provider_tray_state_for_testing(
        &mut self,
        provider: &str,
        state: &str,
        session_percent: Option<f64>,
        weekly_percent: Option<f64>,
        error: Option<&str>,
    ) {
        let Some(provider_id) = ProviderId::from_cli_name(provider) else {
            tracing::warn!("Unknown provider for tray test state: {}", provider);
            return;
        };
        let mut overrides = match self.debug_tray_override.take() {
            Some(DebugTrayOverride::PerProvider(existing)) => existing,
            _ => HashMap::new(),
        };
        overrides.insert(
            provider_id,
            DebugProviderTrayOverride {
                state: state.to_string(),
                session_percent,
                weekly_percent,
                error: error.map(str::to_string),
            },
        );
        self.debug_tray_override = Some(DebugTrayOverride::PerProvider(overrides));
    }

    #[cfg(debug_assertions)]
    fn set_runtime_provider_state_for_testing(
        &mut self,
        provider: &str,
        state: &str,
        session_percent: Option<f64>,
        weekly_percent: Option<f64>,
        error: Option<&str>,
    ) {
        let normalized = provider.trim().to_ascii_lowercase();
        let Ok(mut shared_state) = self.state.lock() else {
            return;
        };
        let Some(provider_data) = shared_state
            .providers
            .iter_mut()
            .find(|item| item.name.eq_ignore_ascii_case(&normalized))
        else {
            tracing::warn!("Unknown runtime provider for test state: {}", provider);
            return;
        };

        match state {
            "error" => {
                provider_data.session_percent = None;
                provider_data.weekly_percent = None;
                provider_data.error = Some(error.unwrap_or("Test error").to_string());
            }
            "clear" => {
                provider_data.session_percent = None;
                provider_data.weekly_percent = None;
                provider_data.error = None;
            }
            _ => {
                let session_percent = session_percent.unwrap_or(0.0);
                provider_data.session_percent = Some(session_percent);
                provider_data.weekly_percent = Some(weekly_percent.unwrap_or(session_percent));
                provider_data.error = None;
            }
        }

        shared_state.summary_providers = shared_state.providers.clone();
        self.debug_tray_override = None;
    }

    #[cfg(debug_assertions)]
    fn queue_test_pointer_batches(&mut self, batches: impl IntoIterator<Item = Vec<egui::Event>>) {
        self.pending_test_event_batches.extend(batches);
    }

    #[cfg(debug_assertions)]
    fn record_debug_view_state(&mut self, ctx: &egui::Context) {
        self.debug_tab_targets.clear();
        self.debug_viewport_outer_rect = ctx.input(|i| i.viewport().outer_rect);
    }

    #[cfg(debug_assertions)]
    fn queue_test_click(
        &mut self,
        ctx: &egui::Context,
        pos: egui::Pos2,
        button: egui::PointerButton,
    ) {
        self.open_main_window_for_testing(ctx);
        self.queue_test_pointer_batches(build_test_click_event_batches(pos, button));
    }

    #[cfg(debug_assertions)]
    fn apply_debug_tray_override(&self, tray: &UnifiedTrayManager) -> bool {
        match (&self.debug_tray_override, self.settings.tray_icon_mode) {
            (
                Some(DebugTrayOverride::Single {
                    state,
                    provider_name,
                    session_percent,
                    weekly_percent,
                    error,
                }),
                TrayIconMode::Single,
            ) => {
                tray.set_single_state_for_testing(
                    state,
                    provider_name.as_deref(),
                    *session_percent,
                    *weekly_percent,
                    error.as_deref(),
                );
                true
            }
            (Some(DebugTrayOverride::PerProvider(overrides)), TrayIconMode::PerProvider) => {
                for (provider_id, override_state) in overrides {
                    tray.set_provider_state_for_testing(
                        *provider_id,
                        &override_state.state,
                        override_state.session_percent,
                        override_state.weekly_percent,
                        override_state.error.as_deref(),
                    );
                }
                true
            }
            _ => false,
        }
    }

    fn layout_main_window(&mut self, ctx: &egui::Context, anchor_to_pointer: bool) {
        let Some(outer_rect) = ctx.input(|i| i.viewport().outer_rect) else {
            return;
        };
        let Some(work_area) = work_area_rect(ctx) else {
            return;
        };

        let margin = 12.0;
        let gap = 10.0;
        let min_size = egui::vec2(320.0, 240.0);
        let max_w = (work_area.width() - margin * 2.0).max(min_size.x);
        let max_h = (work_area.height() - margin * 2.0).max(min_size.y);
        let target_height = if let Ok(state) = self.state.lock() {
            match state.selected_tab {
                SelectedTab::Summary => main_window_summary_height(
                    state
                        .summary_providers
                        .iter()
                        .filter(|provider| should_show_provider(provider))
                        .count(),
                    !self.settings.get_enabled_provider_ids().is_empty(),
                ),
                SelectedTab::Provider(_) => 500.0,
            }
        } else {
            500.0
        };
        let target_size = egui::vec2(360.0_f32.min(max_w), target_height.min(max_h));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(target_size));

        // Tray-anchored positioning: convert physical tray icon coords to logical
        let tray_anchor = self.tray_anchor_pos.take().map(|(px, py)| {
            let ppp = ctx.pixels_per_point().max(0.1);
            egui::pos2(px as f32 / ppp, py as f32 / ppp)
        });

        let (target_x, target_y) = if let Some(tray_pos) = tray_anchor {
            // Anchor the popup near the tray icon.  Prefer above (bottom
            // taskbar) or below (top taskbar) based on available space.
            let space_above = tray_pos.y - work_area.min.y - margin;
            let space_below = work_area.max.y - tray_pos.y - margin;
            let x = tray_pos.x - target_size.x * 0.5;
            let y = if space_above >= target_size.y + gap || space_above > space_below {
                tray_pos.y - target_size.y - gap
            } else {
                tray_pos.y + gap
            };
            (x, y)
        } else if anchor_to_pointer {
            // For keyboard-shortcut opens, keep the popup on the left side
            // and vertically centered so it doesn't appear pinned to the
            // taskbar area.
            let center_x = work_area.min.x + work_area.width() * 0.22;
            (
                center_x - target_size.x * 0.5,
                work_area.min.y + (work_area.height() - target_size.y) * 0.5,
            )
        } else {
            let anchor = outer_rect.center();
            let space_above = anchor.y - work_area.min.y - margin;
            let space_below = work_area.max.y - anchor.y - margin;
            let x = anchor.x - target_size.x * 0.5;
            let y = if space_below >= target_size.y + gap || space_below >= space_above {
                anchor.y + gap
            } else {
                anchor.y - target_size.y - gap
            };
            (x, y)
        };

        let min_x = work_area.min.x + margin;
        let min_y = work_area.min.y + margin;
        let max_x = (work_area.max.x - target_size.x - margin).max(min_x);
        let max_y = (work_area.max.y - target_size.y - margin).max(min_y);
        let x = if max_x <= min_x {
            min_x
        } else {
            target_x.clamp(min_x, max_x)
        };
        let y = if max_y <= min_y {
            min_y
        } else {
            target_y.clamp(min_y, max_y)
        };

        // Apply window chrome based on mode: borderless panel vs decorated popout
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(self.is_popout_mode));
        ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(self.is_popout_mode));

        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(x, y)));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);

        self.pending_main_window_layout = false;
        self.anchor_main_window_to_pointer = false;
    }

    fn refresh_providers(&self) {
        let state = Arc::clone(&self.state);
        let enabled_ids = self.settings.get_enabled_provider_ids();
        let manual_cookies = ManualCookies::load();
        let api_keys = ApiKeys::load();
        let reset_time_relative = self.settings.reset_time_relative;
        let ui_language = self.settings.ui_language;
        // Load token accounts for account switching support
        let token_accounts = TokenAccountStore::new().load().unwrap_or_default();
        // Notification manager for usage alerts
        let notification_manager = Arc::clone(&self.notification_manager);
        let settings = self.settings.clone();

        std::thread::spawn(move || {
            if let Ok(mut s) = state.lock() {
                s.initial_refresh_pending = false;
                s.is_refreshing = true;
                s.loading_pattern = LoadingPattern::random();
                s.loading_phase = 0.0;
                s.providers = enabled_ids
                    .iter()
                    .map(|&id| ProviderData::placeholder(id))
                    .collect();
                if let SelectedTab::Provider(id) = s.selected_tab
                    && !enabled_ids.contains(&id)
                {
                    s.selected_tab = SelectedTab::Summary;
                }
            }

            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!("Failed to create tokio runtime: {}", e);
                    if let Ok(mut s) = state.lock() {
                        s.is_refreshing = false;
                    }
                    return;
                }
            };
            rt.block_on(async {
                // Clear any stale OAuth env vars at the start of refresh
                // This ensures account switches take effect immediately
                const OAUTH_ENV_KEYS: &[&str] = &[
                    "CODEXBAR_CLAUDE_OAUTH_TOKEN",
                    "ZED_API_TOKEN",
                    "ZAI_API_TOKEN",
                ];
                for key in OAUTH_ENV_KEYS {
                    // SAFETY: no concurrent threads are reading these env vars at this point
                    unsafe { std::env::remove_var(key) };
                }

                let handles: Vec<_> = enabled_ids
                    .iter()
                    .enumerate()
                    .map(|(idx, &id)| {
                        // Check for active token account first
                        let active_token = token_accounts
                            .get(&id)
                            .and_then(|data| data.active_account())
                            .map(|account| account.token.clone());

                        // Check for environment override from token account (e.g., for Zai/Claude OAuth)
                        let env_override = active_token
                            .as_ref()
                            .and_then(|token| TokenAccountSupport::env_override(id, token));

                        // Set env override if present - providers will read from env vars
                        // Note: We don't clear env vars here to avoid race conditions with
                        // concurrent provider fetches. Instead, we pass api_key directly
                        // through FetchContext for providers that support it.
                        if let Some(ref env_vars) = env_override {
                            for (key, value) in env_vars {
                                // SAFETY: called sequentially before spawning provider fetch tasks
                                unsafe { std::env::set_var(key, value) };
                            }
                        }

                        // Determine cookie header: active token account > manual cookie > browser extraction
                        let cookie_header = if env_override.is_some() {
                            None
                        } else if let Some(ref token) = active_token {
                            // Use active account's token, normalized for this provider
                            Some(TokenAccountSupport::normalized_cookie_header(id, token))
                        } else {
                            // Fallback to manual cookie or browser extraction
                            let manual_cookie =
                                manual_cookies.get(id.cli_name()).map(|s| s.to_string());
                            manual_cookie.or_else(|| {
                                // Try browser cookie extraction if no manual cookie
                                id.cookie_domain().and_then(|domain| {
                                    get_cookie_header(domain).ok().filter(|h| !h.is_empty())
                                })
                            })
                        };

                        let api_key = if env_override.is_some() {
                            // If we have env override, extract API key from it
                            env_override
                                .as_ref()
                                .and_then(|env| env.values().next().cloned())
                        } else {
                            api_keys.get(id.cli_name()).map(|s| s.to_string())
                        };

                        let ctx = FetchContext {
                            manual_cookie_header: cookie_header,
                            api_key,
                            ..FetchContext::default()
                        };
                        let state = Arc::clone(&state);
                        tokio::spawn(async move {
                            let provider = create_provider(id);
                            let metadata = provider.metadata().clone();
                            let provider_name = id.cli_name().to_string();

                            let (usage_result, status_result) = tokio::join!(
                                async {
                                    tokio::time::timeout(
                                        std::time::Duration::from_secs(5),
                                        provider.fetch_usage(&ctx),
                                    )
                                    .await
                                },
                                async {
                                    tokio::time::timeout(
                                        std::time::Duration::from_secs(5),
                                        fetch_provider_status(&provider_name),
                                    )
                                    .await
                                }
                            );

                            let mut result = match usage_result {
                                Ok(Ok(result)) => ProviderData::from_result(
                                    id,
                                    &result,
                                    &metadata,
                                    reset_time_relative,
                                    ui_language,
                                ),
                                Ok(Err(e)) => ProviderData::from_error(id, e.to_string()),
                                Err(_) => ProviderData::from_error(id, "Timeout".to_string()),
                            };

                            if let Ok(Some(status)) = status_result {
                                result.status_level = status.level;
                                result.status_description = Some(status.description);
                            }

                            if result.error.is_none() {
                                result.usage_breakdown =
                                    load_usage_breakdown_points(id, result.account.as_deref());
                            }

                            let provider_name_lower = provider_name.to_lowercase();
                            if provider_name_lower == "codex" || provider_name_lower == "claude" {
                                result.cost_history =
                                    get_daily_cost_history(&provider_name_lower, 30);
                            }

                            if result.error.is_none() {
                                result.credits_history =
                                    load_credits_history_points(id, result.account.as_deref());
                            }

                            if let Ok(mut s) = state.lock()
                                && idx < s.providers.len()
                            {
                                s.providers[idx] = result;
                            }
                        })
                    })
                    .collect();

                for handle in handles {
                    let _ = handle.await;
                }
            });

            if let Ok(mut s) = state.lock() {
                s.summary_providers = s.providers.clone();
                s.last_refresh = Instant::now();
                s.is_refreshing = false;

                // Check for usage alerts and send notifications
                for provider in &s.providers {
                    if let Some(provider_id) = ProviderId::from_cli_name(&provider.name) {
                        // Check primary session usage
                        if let Some(session_percent) = provider.session_percent
                            && let Ok(mut nm) = notification_manager.lock()
                        {
                            nm.check_and_notify(provider_id, session_percent, &settings);
                            nm.check_session_transition(provider_id, session_percent, &settings);
                        }
                        // Note: Infini alerts are based on the highest usage across all windows
                        // The primary (5-hour) window is used as the main indicator
                    }
                }
            }
        });
    }

    /// Get an animated value that smoothly transitions to the target over 300ms.
    ///
    /// This helper provides consistent animation behavior for progress bar fills
    /// and other numeric value transitions.
    ///
    /// # Arguments
    /// * `ctx` - The egui context for animation state
    /// * `id` - A unique identifier for tracking this animation
    /// * `target` - The target value to animate towards
    ///
    /// # Returns
    /// The current animated value, which will smoothly approach the target
    #[allow(dead_code)]
    fn get_animated_value(ctx: &egui::Context, id: egui::Id, target: f32) -> f32 {
        ctx.animate_value_with_time(id, target, 0.3)
    }
}

fn work_area_rect(ctx: &egui::Context) -> Option<Rect> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::RECT as WinRect;
        use windows::Win32::UI::WindowsAndMessaging::{
            SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
        };

        let mut rect = WinRect::default();
        let ok = unsafe {
            SystemParametersInfoW(
                SPI_GETWORKAREA,
                0,
                Some((&mut rect as *mut WinRect).cast()),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )
            .is_ok()
        };

        if ok {
            let pixels_per_point = ctx.pixels_per_point().max(0.1);
            return Some(Rect::from_min_max(
                egui::pos2(
                    rect.left as f32 / pixels_per_point,
                    rect.top as f32 / pixels_per_point,
                ),
                egui::pos2(
                    rect.right as f32 / pixels_per_point,
                    rect.bottom as f32 / pixels_per_point,
                ),
            ));
        }
    }

    ctx.input(|i| {
        i.viewport()
            .monitor_size
            .map(|size| Rect::from_min_size(egui::pos2(0.0, 0.0), size))
    })
}

fn create_provider(id: ProviderId) -> Box<dyn Provider> {
    match id {
        ProviderId::Claude => Box::new(ClaudeProvider::new()),
        ProviderId::Codex => Box::new(CodexProvider::new()),
        ProviderId::Cursor => Box::new(CursorProvider::new()),
        ProviderId::Gemini => Box::new(GeminiProvider::new()),
        ProviderId::Copilot => Box::new(CopilotProvider::new()),
        ProviderId::Antigravity => Box::new(AntigravityProvider::new()),
        ProviderId::Factory => Box::new(FactoryProvider::new()),
        ProviderId::Zai => Box::new(ZaiProvider::new()),
        ProviderId::Kiro => Box::new(KiroProvider::new()),
        ProviderId::VertexAI => Box::new(VertexAIProvider::new()),
        ProviderId::Augment => Box::new(AugmentProvider::new()),
        ProviderId::MiniMax => Box::new(MiniMaxProvider::new()),
        ProviderId::OpenCode => Box::new(OpenCodeProvider::new()),
        ProviderId::Kimi => Box::new(KimiProvider::new()),
        ProviderId::KimiK2 => Box::new(KimiK2Provider::new()),
        ProviderId::Amp => Box::new(AmpProvider::new()),
        ProviderId::Warp => Box::new(WarpProvider::new()),
        ProviderId::Ollama => Box::new(OllamaProvider::new()),
        ProviderId::OpenRouter => Box::new(OpenRouterProvider::new()),
        ProviderId::Synthetic => Box::new(SyntheticProvider::new()),
        ProviderId::JetBrains => Box::new(JetBrainsProvider::new()),
        ProviderId::Alibaba => Box::new(AlibabaProvider::new()),
        ProviderId::NanoGPT => Box::new(NanoGPTProvider::new()),
        ProviderId::Infini => Box::new(InfiniProvider::default()),
    }
}

impl eframe::App for CodexBarApp {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, _raw_input: &mut egui::RawInput) {
        #[cfg(debug_assertions)]
        {
            let drained_inputs: Vec<_> = if let Ok(mut queue) = self.test_input_queue.lock() {
                queue.drain(..).collect()
            } else {
                Vec::new()
            };
            for input in drained_inputs {
                match input {
                    super::test_server::TestInput::OpenWindow => {
                        self.open_main_window_for_testing(_ctx);
                    }
                    super::test_server::TestInput::SelectTab { tab } => {
                        self.select_tab_for_testing(&tab, _ctx);
                    }
                    super::test_server::TestInput::SelectPreferencesTab { tab } => {
                        self.preferences_window.select_tab_for_testing(&tab);
                        _ctx.request_repaint();
                    }
                    super::test_server::TestInput::SelectPreferencesProvider { provider } => {
                        self.preferences_window
                            .select_provider_for_testing(&provider);
                        _ctx.request_repaint();
                    }
                    super::test_server::TestInput::SetSingleTrayState {
                        state,
                        provider,
                        session_percent,
                        weekly_percent,
                        error,
                    } => {
                        self.set_single_tray_state_for_testing(
                            &state,
                            provider.as_deref(),
                            session_percent,
                            weekly_percent,
                            error.as_deref(),
                        );
                        _ctx.request_repaint();
                    }
                    super::test_server::TestInput::SetProviderTrayState {
                        provider,
                        state,
                        session_percent,
                        weekly_percent,
                        error,
                    } => {
                        self.set_provider_tray_state_for_testing(
                            &provider,
                            &state,
                            session_percent,
                            weekly_percent,
                            error.as_deref(),
                        );
                        _ctx.request_repaint();
                    }
                    super::test_server::TestInput::SetProviderEnabled { provider, enabled } => {
                        self.preferences_window
                            .set_provider_enabled_for_testing(&provider, enabled);
                        self.debug_tray_override = None;
                        _ctx.request_repaint();
                    }
                    super::test_server::TestInput::SetRuntimeProviderState {
                        provider,
                        state,
                        session_percent,
                        weekly_percent,
                        error,
                    } => {
                        self.set_runtime_provider_state_for_testing(
                            &provider,
                            &state,
                            session_percent,
                            weekly_percent,
                            error.as_deref(),
                        );
                        _ctx.request_repaint();
                    }
                    super::test_server::TestInput::SetRefreshInterval { seconds } => {
                        self.preferences_window
                            .set_refresh_interval_for_testing(seconds);
                        _ctx.request_repaint();
                    }
                    super::test_server::TestInput::SetDisplaySetting { name, enabled } => {
                        self.preferences_window
                            .set_display_setting_for_testing(&name, enabled);
                        self.debug_tray_override = None;
                        _ctx.request_repaint();
                    }
                    super::test_server::TestInput::SetDisplayMode { mode } => {
                        self.preferences_window.set_display_mode_for_testing(&mode);
                        self.debug_tray_override = None;
                        _ctx.request_repaint();
                    }
                    super::test_server::TestInput::SetApiKeyInput { provider, value } => {
                        self.preferences_window
                            .set_api_key_input_for_testing(&provider, &value);
                        _ctx.request_repaint();
                    }
                    super::test_server::TestInput::SubmitApiKey => {
                        self.preferences_window.submit_api_key_for_testing();
                        _ctx.request_repaint();
                    }
                    super::test_server::TestInput::SetCookieInput { provider, value } => {
                        self.preferences_window
                            .set_cookie_input_for_testing(&provider, &value);
                        _ctx.request_repaint();
                    }
                    super::test_server::TestInput::SubmitCookie => {
                        self.preferences_window.submit_cookie_for_testing();
                        _ctx.request_repaint();
                    }
                    super::test_server::TestInput::SaveState { path } => {
                        self.request_test_state_dump(PathBuf::from(path), _ctx);
                    }
                    super::test_server::TestInput::SaveScreenshot { path } => {
                        self.request_test_screenshot(PathBuf::from(path), _ctx);
                    }
                    super::test_server::TestInput::SavePreferencesScreenshot { path } => {
                        self.request_test_preferences_screenshot(PathBuf::from(path), _ctx);
                    }
                    super::test_server::TestInput::MovePointer { x, y } => {
                        let pos = egui::pos2(x, y);
                        self.queue_test_pointer_batches([vec![egui::Event::PointerMoved(pos)]]);
                        tracing::debug!("Injected test pointer move to ({}, {})", x, y);
                    }
                    super::test_server::TestInput::Click { x, y } => {
                        let pos = egui::pos2(x, y);
                        self.queue_test_click(_ctx, pos, egui::PointerButton::Primary);
                        tracing::debug!("Injected staged test click at ({}, {})", x, y);
                    }
                    super::test_server::TestInput::DoubleClick { x, y } => {
                        let pos = egui::pos2(x, y);
                        self.open_main_window_for_testing(_ctx);
                        self.queue_test_pointer_batches([
                            vec![egui::Event::PointerMoved(pos)],
                            vec![egui::Event::PointerButton {
                                pos,
                                button: egui::PointerButton::Primary,
                                pressed: true,
                                modifiers: egui::Modifiers::NONE,
                            }],
                            vec![egui::Event::PointerMoved(pos)],
                            vec![egui::Event::PointerButton {
                                pos,
                                button: egui::PointerButton::Primary,
                                pressed: false,
                                modifiers: egui::Modifiers::NONE,
                            }],
                            vec![egui::Event::PointerMoved(pos)],
                            vec![egui::Event::PointerButton {
                                pos,
                                button: egui::PointerButton::Primary,
                                pressed: true,
                                modifiers: egui::Modifiers::NONE,
                            }],
                            vec![egui::Event::PointerMoved(pos)],
                            vec![egui::Event::PointerButton {
                                pos,
                                button: egui::PointerButton::Primary,
                                pressed: false,
                                modifiers: egui::Modifiers::NONE,
                            }],
                        ]);
                        tracing::debug!("Injected staged test double-click at ({}, {})", x, y);
                    }
                    super::test_server::TestInput::RightClick { x, y } => {
                        let pos = egui::pos2(x, y);
                        self.queue_test_click(_ctx, pos, egui::PointerButton::Secondary);
                        tracing::debug!("Injected staged test right-click at ({}, {})", x, y);
                    }
                    super::test_server::TestInput::SimulateTrayLeftClick => {
                        self.simulate_tray_left_click(_ctx);
                    }
                }
            }
        }

        #[cfg(debug_assertions)]
        if let Some(events) = self.pending_test_event_batches.pop_front() {
            _raw_input.events.extend(events);
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.first_update_logged {
            let viewport_outer_rect = ctx.input(|i| i.viewport().outer_rect);
            append_launch_log(&format!(
                "first update: pending_main_window_layout={} anchor_main_window_to_pointer={} viewport_outer_rect={:?}",
                self.pending_main_window_layout,
                self.anchor_main_window_to_pointer,
                viewport_outer_rect
            ));
            self.first_update_logged = true;
        }

        // Intercept window close: hide to tray instead of exiting
        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        if self.pending_main_window_layout {
            self.layout_main_window(ctx, self.anchor_main_window_to_pointer);
        }

        #[cfg(debug_assertions)]
        if let Some(path) = self.pending_test_screenshot_path.clone() {
            if self.pending_test_screenshot_delay_frames > 0 {
                self.pending_test_screenshot_delay_frames -= 1;
                ctx.request_repaint();
            } else if self.pending_main_window_layout {
                ctx.request_repaint();
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(path)));
                self.pending_test_screenshot_path = None;
                ctx.request_repaint();
            }
        }

        // Check keyboard shortcuts without holding an immutable borrow of self
        // while triggering layout changes.
        let mut shortcut_triggered = false;
        if let Some(shortcut_mgr) = self.shortcut_manager.as_ref() {
            while shortcut_mgr.check_events() {
                shortcut_triggered = true;
            }
        }
        if shortcut_triggered {
            tracing::info!("Keyboard shortcut triggered - focusing window");
            self.is_popout_mode = false;
            if let Ok(mut state) = self.state.lock() {
                state.selected_tab = SelectedTab::Summary;
            }
            self.pending_main_window_layout = true;
            self.anchor_main_window_to_pointer = true;
            self.layout_main_window(ctx, true);
        }

        #[cfg(debug_assertions)]
        {
            let screenshot_events: Vec<_> = ctx.input(|i| {
                i.events
                    .iter()
                    .filter_map(|event| match event {
                        egui::Event::Screenshot {
                            user_data, image, ..
                        } => Some((user_data.clone(), image.clone())),
                        _ => None,
                    })
                    .collect()
            });

            for (user_data, image) in screenshot_events {
                if let Some(path) = user_data
                    .data
                    .as_ref()
                    .and_then(|data| data.downcast_ref::<PathBuf>())
                {
                    if let Err(error) = save_color_image_to_png(path, &image) {
                        tracing::warn!(
                            "Failed to save test screenshot to {}: {}",
                            path.display(),
                            error
                        );
                    } else {
                        tracing::info!("Saved test screenshot to {}", path.display());
                    }
                }
            }
        }

        // Auto-refresh check
        let should_refresh = {
            if self.settings.refresh_interval_secs == 0 {
                false
            } else if let Ok(state) = self.state.lock() {
                should_auto_refresh(
                    state.initial_refresh_pending,
                    state.is_refreshing,
                    state.last_refresh,
                    self.settings.refresh_interval_secs,
                    Instant::now(),
                )
            } else {
                false
            }
        };
        if should_refresh {
            self.refresh_providers();
        }

        // Get state
        let (
            providers,
            _summary_providers,
            selected_tab,
            is_refreshing,
            loading_pattern,
            loading_phase,
            surprise_state,
            update_info,
            update_download_state,
            login_state,
        ) = {
            if let Ok(mut state) = self.state.lock() {
                if state.is_refreshing {
                    state.loading_phase += 0.05;
                    if state.loading_phase > 1.0 {
                        state.loading_phase -= 1.0;
                    }
                }

                let surprise = if self.settings.surprise_animations && !state.is_refreshing {
                    if let Some(anim) = state.surprise_animation {
                        state.surprise_frame += 1;
                        if state.surprise_frame >= anim.duration_frames() {
                            state.surprise_animation = None;
                            state.surprise_frame = 0;
                            state.next_surprise_time = Instant::now() + random_surprise_delay();
                            None
                        } else {
                            Some((anim, state.surprise_frame))
                        }
                    } else if Instant::now() >= state.next_surprise_time {
                        let anim = SurpriseAnimation::random();
                        state.surprise_animation = Some(anim);
                        state.surprise_frame = 0;
                        Some((anim, 0))
                    } else {
                        None
                    }
                } else {
                    None
                };

                let update = if state.update_dismissed {
                    None
                } else {
                    state.update_available.clone()
                };

                let update_download_state = state.update_state.clone();

                let login_state = (
                    state.login_provider.clone(),
                    state.login_phase,
                    state.login_message.clone(),
                );

                (
                    state.providers.clone(),
                    state.summary_providers.clone(),
                    state.selected_tab,
                    state.is_refreshing,
                    state.loading_pattern,
                    state.loading_phase,
                    surprise,
                    update,
                    update_download_state,
                    login_state,
                )
            } else {
                (
                    Vec::new(),
                    Vec::new(),
                    SelectedTab::Summary,
                    false,
                    LoadingPattern::default(),
                    0.0,
                    None,
                    None,
                    UpdateState::Idle,
                    (None, LoginPhase::Idle, None),
                )
            }
        };

        let (_login_provider, login_phase, _login_message) = login_state;
        let is_logging_in = _login_provider.is_some() && login_phase != LoginPhase::Idle;

        ctx.request_repaint_after(
            if is_refreshing || surprise_state.is_some() || is_logging_in {
                Duration::from_millis(50)
            } else {
                Duration::from_millis(200)
            },
        );

        // Update tray icon
        if let Some(ref tray) = self.tray_manager {
            #[cfg(debug_assertions)]
            let handled_by_debug_override = self.apply_debug_tray_override(tray);
            #[cfg(not(debug_assertions))]
            let handled_by_debug_override = false;

            if handled_by_debug_override {
                // The debug tray override already applied the intended runtime state.
            } else if is_refreshing {
                tray.show_loading(loading_pattern, loading_phase);
            } else if let Some((anim, frame)) = surprise_state {
                // Use first provider with data for surprise animation
                if let Some(provider) = providers.iter().find(|p| p.session_percent.is_some()) {
                    let session = provider.session_percent.unwrap_or(0.0);
                    let weekly = provider.weekly_percent.unwrap_or(session);
                    tray.show_surprise(anim, frame, session, weekly);
                }
            } else {
                match choose_tray_runtime_state(&providers, &self.settings) {
                    Some(TrayRuntimeState::SingleUsage(provider)) => {
                        tray.update_usage(
                            provider.session_percent,
                            provider.weekly_percent,
                            &provider.name,
                        );
                    }
                    Some(TrayRuntimeState::SingleError {
                        provider_name,
                        error,
                    }) => {
                        tray.show_error(&provider_name, &error);
                    }
                    Some(TrayRuntimeState::MergedUsage(usages)) => {
                        tray.update_merged(&usages);
                    }
                    Some(TrayRuntimeState::PerProvider(states)) => {
                        for state in states {
                            match state {
                                PerProviderTrayRuntimeState::Usage { provider_id, usage } => {
                                    tray.update_provider_usage(
                                        provider_id,
                                        usage.session_percent,
                                        usage.weekly_percent,
                                    );
                                }
                                PerProviderTrayRuntimeState::Error { provider_id, error } => {
                                    tray.show_provider_error(provider_id, &error);
                                }
                            }
                        }
                    }
                    None => {}
                }
            }

            let mut tray_actions = Vec::new();
            if let Some(ref action_rx) = self.tray_action_rx {
                while let Ok(action) = action_rx.try_recv() {
                    tray_actions.push(action);
                }
            }
            for action in tray_actions {
                match action {
                    TrayMenuAction::Quit => self.quit_application(),
                    TrayMenuAction::TrayRightClick => {
                        // Show the native context menu at the cursor position.
                        // This blocks (modal TrackPopupMenu) until the menu is
                        // dismissed; the resulting MenuEvent is picked up by the
                        // polling thread and forwarded as a normal TrayMenuAction
                        // in the next frame.
                        #[cfg(target_os = "windows")]
                        if let Some(ref tray_manager) = self.tray_manager {
                            if let Some(hwnd) = find_main_window() {
                                tray_manager.show_context_menu(hwnd.0 as isize);
                            }
                        }
                        // Return to hidden state after the menu closes.
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    }
                    TrayMenuAction::TrayLeftClick { tray_x, tray_y } => {
                        tracing::info!(
                            "TrayLeftClick action received: tray_x={}, tray_y={}",
                            tray_x,
                            tray_y
                        );
                        self.is_popout_mode = false;
                        if let Ok(mut state) = self.state.lock() {
                            state.selected_tab = SelectedTab::Summary;
                        }
                        self.tray_anchor_pos = Some((tray_x, tray_y));
                        self.pending_main_window_layout = true;
                        self.anchor_main_window_to_pointer = false;
                        self.layout_main_window(ctx, false);
                    }
                    TrayMenuAction::PopOut => {
                        self.is_popout_mode = true;
                        if let Ok(mut state) = self.state.lock() {
                            state.selected_tab = SelectedTab::Summary;
                        }
                        self.pending_main_window_layout = true;
                        self.anchor_main_window_to_pointer = true;
                        self.layout_main_window(ctx, true);
                    }
                    TrayMenuAction::PopOutProvider(provider_name) => {
                        self.is_popout_mode = true;
                        if let Some(provider_id) = ProviderId::from_cli_name(&provider_name) {
                            if let Ok(mut state) = self.state.lock() {
                                state.selected_tab = SelectedTab::Provider(provider_id);
                            }
                            self.pending_main_window_layout = true;
                            self.anchor_main_window_to_pointer = true;
                            self.layout_main_window(ctx, true);
                        }
                    }
                    TrayMenuAction::Refresh => {
                        if !is_refreshing {
                            self.refresh_providers();
                            // Ensure animations advance even if window is hidden/minimized.
                            ctx.request_repaint();
                        }
                    }
                    TrayMenuAction::RefreshProvider(provider_name) => {
                        if !is_refreshing {
                            tracing::info!(
                                "Refreshing provider tray request for {}; using shared refresh pass",
                                provider_name
                            );
                            self.refresh_providers();
                            ctx.request_repaint();
                        }
                    }
                    TrayMenuAction::Settings => {
                        self.preferences_window.open();
                        // Move main window off-screen so only settings viewport is visible.
                        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                            -10000.0, -10000.0,
                        )));
                    }
                    TrayMenuAction::CheckForUpdates => {
                        let effective_settings = self.preferences_window.current_settings();
                        start_update_check(
                            Arc::clone(&self.state),
                            effective_settings.update_channel,
                            effective_settings.auto_download_updates,
                        );
                    }
                    TrayMenuAction::ToggleProvider(provider_name) => {
                        // Toggle provider enabled state
                        if let Some(provider_id) = ProviderId::from_cli_name(&provider_name) {
                            self.settings.toggle_provider(provider_id);
                            if let Err(e) = self.settings.save() {
                                tracing::error!("Failed to save settings: {}", e);
                            }
                            // Refresh to update the UI with new provider list
                            self.refresh_providers();
                        }
                    }
                }
            }
        }

        // Apply refined style
        let mut style = (*ctx.style()).clone();
        style.visuals.window_fill = Theme::BG_PRIMARY;
        style.visuals.panel_fill = Theme::BG_PRIMARY;
        style.visuals.widgets.noninteractive.bg_fill = Theme::BG_SECONDARY;
        style.visuals.widgets.inactive.bg_fill = Theme::CARD_BG;
        style.visuals.widgets.hovered.bg_fill = Theme::CARD_BG_HOVER;
        style.visuals.widgets.active.bg_fill = Theme::ACCENT_PRIMARY;
        style.visuals.selection.bg_fill = Theme::selection_overlay();
        style.visuals.selection.stroke = Stroke::new(1.0, Theme::ACCENT_PRIMARY);
        ctx.set_style(style);

        // Handle keyboard shortcuts
        ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Comma) {
                self.preferences_window.open();
            }
        });

        #[cfg(debug_assertions)]
        self.record_debug_view_state(ctx);

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Theme::BG_PRIMARY).inner_margin(Spacing::SM))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                    // ════════════════════════════════════════════════════════════
                    // UPDATE BANNER
                    // ════════════════════════════════════════════════════════════
                    if let Some(ref update) = update_info {
                        egui::Frame::none()
                            .fill(Theme::ACCENT_PRIMARY)
                            .rounding(Rounding::same(Radius::LG))
                            .inner_margin(Spacing::MD)
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        // Icon based on state
                                        let icon = match &update_download_state {
                                            UpdateState::Ready(_) => "✓",
                                            UpdateState::Failed(_) => "⚠",
                                            UpdateState::Downloading(_) => "↓",
                                            _ => "🎉",
                                        };
                                        ui.label(RichText::new(icon).size(FontSize::MD).color(Color32::WHITE));
                                        ui.add_space(Spacing::XS);

                                        // Message based on state
                                        let ui_lang = self.settings.ui_language;
                                        let message = match &update_download_state {
                                            UpdateState::Downloading(progress) => {
                                                let template = locale_text(ui_lang, LocaleKey::UpdateDownloadingMessage);
                                                template.replace("{}", &update.version).replace("{:.0}", &format!("{:.0}", progress * 100.0))
                                            }
                                            UpdateState::Ready(_) => {
                                                let template = locale_text(ui_lang, LocaleKey::UpdateReadyMessage);
                                                template.replace("{}", &update.version)
                                            }
                                            UpdateState::Failed(e) => {
                                                let template = locale_text(ui_lang, LocaleKey::UpdateFailedMessage);
                                                template.replace("{}", e)
                                            }
                                            _ => {
                                                let template = locale_text(ui_lang, LocaleKey::UpdateAvailableMessage);
                                                template.replace("{}", &update.version)
                                            }
                                        };
                                        ui.label(
                                            RichText::new(message)
                                                .size(FontSize::BASE)
                                                .color(Color32::WHITE)
                                                .strong(),
                                        );

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            // Dismiss button
                                        if ui.add(
                                            egui::Button::new(RichText::new("✕").size(FontSize::SM).color(Color32::WHITE))
                                                .fill(Color32::TRANSPARENT)
                                                .stroke(Stroke::NONE)
                                        ).clicked()
                                            && let Ok(mut s) = self.state.lock() {
                                                s.update_dismissed = true;
                                            }

                                            // Action button based on state
                                            match &update_download_state {
                                                UpdateState::Ready(path) => {
                                                    if update.supports_auto_apply() {
                                                        let installer_path = path.clone();
                                                        if ui.add(
                                                            egui::Button::new(
                                                                RichText::new(locale_text(ui_lang, LocaleKey::UpdateRestartAndUpdate))
                                                                    .size(FontSize::SM)
                                                                    .color(Theme::ACCENT_PRIMARY),
                                                            )
                                                            .fill(Color32::WHITE)
                                                            .rounding(Rounding::same(Radius::SM))
                                                        ).clicked()
                                                            && let Err(e) = updater::apply_update(&installer_path) {
                                                                tracing::error!("Failed to apply update: {}", e);
                                                            }
                                                    } else if ui.add(
                                                        egui::Button::new(
                                                            RichText::new(locale_text(ui_lang, LocaleKey::UpdateDownload))
                                                                .size(FontSize::SM)
                                                                .color(Theme::ACCENT_PRIMARY),
                                                        )
                                                        .fill(Color32::WHITE)
                                                        .rounding(Rounding::same(Radius::SM))
                                                    ).clicked() {
                                                        open_update_destination(update);
                                                    }
                                                }
                                                UpdateState::Downloading(_) => {
                                                    ui.spinner();
                                                }
                                                UpdateState::Failed(_) => {
                                                    if update.supports_auto_download()
                                                        && ui.add(
                                                            egui::Button::new(
                                                                RichText::new(locale_text(ui_lang, LocaleKey::UpdateRetry))
                                                                    .size(FontSize::SM)
                                                                    .color(Theme::ACCENT_PRIMARY),
                                                            )
                                                            .fill(Color32::WHITE)
                                                            .rounding(Rounding::same(Radius::SM))
                                                        ).clicked() {
                                                            start_update_download(
                                                                Arc::clone(&self.state),
                                                                update.clone(),
                                                            );
                                                        }
                                                    if ui.add(
                                                        egui::Button::new(
                                                            RichText::new(locale_text(ui_lang, LocaleKey::UpdateDownload))
                                                                .size(FontSize::SM)
                                                                .color(Color32::WHITE),
                                                        )
                                                        .fill(Color32::TRANSPARENT)
                                                        .stroke(Stroke::new(1.0, Color32::WHITE))
                                                        .rounding(Rounding::same(Radius::SM))
                                                    ).clicked() {
                                                        open_update_destination(update);
                                                    }
                                                }
                                                _ => {
                                                    if ui.add(
                                                        egui::Button::new(
                                                            RichText::new(locale_text(ui_lang, LocaleKey::UpdateDownload))
                                                                .size(FontSize::SM)
                                                                .color(Theme::ACCENT_PRIMARY),
                                                        )
                                                        .fill(Color32::WHITE)
                                                        .rounding(Rounding::same(Radius::SM))
                                                    ).clicked() {
                                                        if update.supports_auto_download() {
                                                            start_update_download(
                                                                Arc::clone(&self.state),
                                                                update.clone(),
                                                            );
                                                        } else {
                                                            open_update_destination(update);
                                                        }
                                                    }
                                                }
                                            }
                                        });
                                    });

                                    // Show download progress bar when downloading
                                    if let UpdateState::Downloading(progress) = &update_download_state {
                                        ui.add_space(Spacing::XS);
                                        let bar_width = ui.available_width();
                                        let bar_height = 4.0;
                                        let (rect, _) = ui.allocate_exact_size(Vec2::new(bar_width, bar_height), egui::Sense::hover());

                                        // Track (semi-transparent white)
                                        ui.painter().rect_filled(rect, Rounding::same(2.0), Color32::from_rgba_unmultiplied(255, 255, 255, 80));

                                        // Fill (solid white)
                                        let fill_w = rect.width() * progress.clamp(0.0, 1.0);
                                        if fill_w > 0.0 {
                                            let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, bar_height));
                                            ui.painter().rect_filled(fill_rect, Rounding::same(2.0), Color32::WHITE);
                                        }
                                    }
                                });
                            });
                        ui.add_space(Spacing::MD);
                    }

                    // ════════════════════════════════════════════════════════════
                    // MAIN CONTENT — Summary-first drill-down navigation
                    // ════════════════════════════════════════════════════════════
                    {
                        let visible_providers: Vec<(usize, &ProviderData)> = providers.iter()
                            .enumerate()
                            .filter(|(_, p)| should_show_provider(p))
                            .collect();
                        let has_enabled = !self.settings.get_enabled_provider_ids().is_empty();
                        let show_credits = self.settings.show_credits_extra_usage;
                        let show_as_used = self.settings.show_as_used;
                        let hide_personal_info = self.settings.hide_personal_info;
                        let ui_language = self.settings.ui_language;
                        let mut manual_refresh_requested = false;
                        let mut account_switch_provider: Option<String> = None;

                        if !visible_providers.is_empty() {
                            match selected_tab {
                                SelectedTab::Summary => {
                                    // ── Status line — calm, integrated ──
                                    {
                                        let mut errors = 0usize;
                                        let mut warnings = 0usize;
                                        let mut max_pct: f64 = 0.0;
                                        for (_, p) in &visible_providers {
                                            if p.error.is_some() { errors += 1; }
                                            if p.status_level == StatusLevel::Major || p.status_level == StatusLevel::Partial { warnings += 1; }
                                            for pct in [p.session_percent, p.weekly_percent, p.monthly_percent, p.model_percent].into_iter().flatten() {
                                                if pct > max_pct { max_pct = pct; }
                                            }
                                        }
                                        let (banner_text, banner_color) = if errors > 0 {
                                            (format!("{} provider{} need attention", errors, if errors > 1 { "s" } else { "" }), Theme::RED)
                                        } else if max_pct >= 90.0 {
                                            ("A provider is nearing its quota".to_string(), Theme::ORANGE)
                                        } else if warnings > 0 {
                                            (format!("{} provider{} degraded", warnings, if warnings > 1 { "s" } else { "" }), Theme::ORANGE)
                                        } else {
                                            (format!("{} provider{} operational", visible_providers.len(), if visible_providers.len() != 1 { "s" } else { "" }), Theme::GREEN)
                                        };
                                        let (rect, _) = ui.allocate_exact_size(
                                            Vec2::new(ui.available_width(), 20.0), egui::Sense::hover());
                                        ui.painter().circle_filled(
                                            egui::pos2(rect.min.x + 4.0, rect.center().y),
                                            3.0, banner_color,
                                        );
                                        ui.painter().text(
                                            egui::pos2(rect.min.x + 14.0, rect.center().y),
                                            egui::Align2::LEFT_CENTER,
                                            &banner_text,
                                            egui::FontId::proportional(FontSize::XS),
                                            Theme::TEXT_MUTED,
                                        );
                                        ui.add_space(2.0);
                                        draw_horizontal_separator(ui, 0.0);
                                        ui.add_space(Spacing::XS);
                                    }

                                    // ── Provider usage cards ──
                                    for (idx, (_, provider)) in visible_providers.iter().enumerate() {
                                        if idx > 0 {
                                            ui.add_space(Spacing::XS);
                                        }

                                        let brand_color = provider_color(&provider.name);
                                        let card_h = 56.0;
                                        let avail_w = ui.available_width();

                                        let (rect, response) = ui.allocate_exact_size(
                                            Vec2::new(avail_w, card_h), egui::Sense::click());

                                        let card_fill = if response.hovered() {
                                            Color32::from_rgb(40, 40, 47)
                                        } else {
                                            Color32::from_rgb(33, 33, 39)
                                        };
                                        let card_stroke = if response.hovered() {
                                            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 18))
                                        } else {
                                            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 10))
                                        };
                                        ui.painter().rect(
                                            rect,
                                            Rounding::same(Radius::MD),
                                            card_fill,
                                            card_stroke,
                                        );

                                        let left_x = rect.min.x + 10.0;
                                        let right_x = rect.max.x - 12.0;
                                        let top_y = rect.min.y + 9.0;

                                        // Row 1: [icon] Provider Name ............. Session · 72% used
                                        let icon_sz = 14.0;
                                        if let Some(tex) = self.icon_cache.get_icon(ui.ctx(), &provider.name, icon_sz as u32) {
                                            let ir = Rect::from_min_size(
                                                egui::pos2(left_x, top_y + 1.0),
                                                Vec2::splat(icon_sz));
                                            ui.painter().image(tex.id(), ir,
                                                Rect::from_min_max(egui::pos2(0.0,0.0), egui::pos2(1.0,1.0)), brand_color);
                                        } else {
                                            let fallback = provider.name.chars().next().unwrap_or('?').to_uppercase().to_string();
                                            ui.painter().text(
                                                egui::pos2(left_x + icon_sz / 2.0, top_y + icon_sz / 2.0),
                                                egui::Align2::CENTER_CENTER,
                                                fallback, egui::FontId::proportional(icon_sz), brand_color);
                                        }

                                        let name_x = left_x + icon_sz + 6.0;
                                        ui.painter().text(
                                            egui::pos2(name_x, top_y + 1.0),
                                            egui::Align2::LEFT_TOP,
                                            &provider.display_name,
                                            egui::FontId::proportional(FontSize::BASE),
                                            Theme::TEXT_PRIMARY,
                                        );

                                        // Primary metric: fallback chain session → weekly → monthly → model
                                        // Each variant carries (percent, metric_tag, detail_text)
                                        let primary_metric: Option<(f64, &str, Option<String>)> =
                                            provider.session_percent.map(|p| (
                                                p,
                                                locale_text(ui_language, LocaleKey::ProviderSession),
                                                provider.session_reset.as_deref().map(|r|
                                                    format!("{} {}", locale_text(ui_language, LocaleKey::MetricResetsIn), r)),
                                            ))
                                            .or_else(|| provider.weekly_percent.map(|p| (
                                                p,
                                                locale_text(ui_language, LocaleKey::ProviderWeekly),
                                                provider.weekly_reset.as_deref().map(|r|
                                                    format!("{} {}", locale_text(ui_language, LocaleKey::MetricResetsIn), r)),
                                            )))
                                            .or_else(|| provider.monthly_percent.map(|p| (
                                                p,
                                                locale_text(ui_language, LocaleKey::ProviderMonthly),
                                                provider.monthly_reset.as_deref().map(|r|
                                                    format!("{} {}", locale_text(ui_language, LocaleKey::MetricResetsIn), r)),
                                            )))
                                            .or_else(|| provider.model_percent.map(|p| (
                                                p,
                                                provider.model_name.as_deref()
                                                    .unwrap_or(locale_text(ui_language, LocaleKey::ProviderModel)),
                                                None,
                                            )));

                                        if let Some(ref error) = provider.error {
                                            let short = if error.len() > 24 { format!("{}...", &error[..21]) } else { error.clone() };
                                            ui.painter().text(
                                                egui::pos2(right_x, top_y + 1.0),
                                                egui::Align2::RIGHT_TOP,
                                                &short,
                                                egui::FontId::proportional(FontSize::XS),
                                                Theme::RED,
                                            );
                                        } else if let Some((pct, tag, _)) = &primary_metric {
                                            let display_pct = usage_display_percent(*pct, show_as_used);
                                            let pct_label = usage_display_label(display_pct, show_as_used, ui_language);
                                            let label = format!("{} · {}", tag, pct_label);
                                            ui.painter().text(
                                                egui::pos2(right_x - 14.0, top_y + 2.0),
                                                egui::Align2::RIGHT_TOP,
                                                &label,
                                                egui::FontId::proportional(FontSize::INDICATOR),
                                                Theme::TEXT_SECONDARY,
                                            );
                                        }

                                        // Row 2: progress bar + detail text
                                        let bar_y = top_y + 23.0;
                                        let bar_h = 4.0;

                                        if provider.error.is_none() {
                                            if let Some((pct, _, ref detail)) = primary_metric {
                                                let display_pct = usage_display_percent(pct, show_as_used);
                                                let bar_w = (right_x - 14.0) - name_x;

                                                // Track
                                                let track = Rect::from_min_size(
                                                    egui::pos2(name_x, bar_y), Vec2::new(bar_w, bar_h));
                                                ui.painter().rect_filled(
                                                    track,
                                                    Rounding::same(2.0),
                                                    Color32::from_rgba_unmultiplied(255, 255, 255, 20),
                                                );

                                                // Fill
                                                let fill_w = bar_w * (display_pct as f32 / 100.0).clamp(0.0, 1.0);
                                                if fill_w > 0.0 {
                                                    ui.painter().rect_filled(
                                                        Rect::from_min_size(track.min, Vec2::new(fill_w, bar_h)),
                                                        Rounding::same(2.0), brand_color);
                                                }

                                                // Detail text (below bar, muted)
                                                if let Some(detail_text) = detail {
                                                    let detail_y = bar_y + bar_h + 4.0;
                                                    ui.painter().text(
                                                        egui::pos2(name_x, detail_y),
                                                        egui::Align2::LEFT_TOP,
                                                        detail_text,
                                                        egui::FontId::proportional(FontSize::INDICATOR),
                                                        Theme::TEXT_MUTED,
                                                    );
                                                }
                                            }

                                            // Status dot — only when non-operational
                                            if provider.status_level != StatusLevel::Operational
                                                && provider.status_level != StatusLevel::Unknown
                                            {
                                                let detail_y = bar_y + bar_h + 4.0;
                                                let dot_color = status_color(provider.status_level);
                                                ui.painter().circle_filled(
                                                    egui::pos2(right_x - 14.0, detail_y + 4.0), 2.5, dot_color);
                                            }
                                        }

                                        // Drill-down chevron (subtle, within card bounds)
                                        ui.painter().text(
                                            egui::pos2(rect.max.x - 10.0, rect.center().y),
                                            egui::Align2::RIGHT_CENTER,
                                            "›",
                                            egui::FontId::proportional(FontSize::SM),
                                            if response.hovered() { Theme::TEXT_SECONDARY } else { Theme::TEXT_MUTED },
                                        );

                                        if response.clicked()
                                            && let Ok(mut state) = self.state.lock()
                                            && let Some(pid) =
                                                ProviderId::from_cli_name(&provider.name)
                                        {
                                            state.selected_tab = SelectedTab::Provider(pid);
                                        }

                                        #[cfg(debug_assertions)]
                                        self.debug_tab_targets.push(DebugTabTarget {
                                            name: provider.name.clone(),
                                            rect,
                                            hovered: response.hovered(),
                                            contains_pointer: response.contains_pointer(),
                                            clicked: response.clicked(),
                                            pointer_button_down_on: response.is_pointer_button_down_on(),
                                            interact_pointer_pos: response.interact_pointer_pos(),
                                        });
                                    }
                                }
                                SelectedTab::Provider(selected_id) => {
                                    // ── Back to Summary ──
                                    let back_resp = ui.add(
                                        egui::Button::new(
                                            RichText::new("‹ Summary")
                                                .size(FontSize::XS)
                                                .color(Theme::TEXT_SECONDARY)
                                        ).fill(Color32::TRANSPARENT).stroke(Stroke::NONE)
                                    );
                                    if back_resp.clicked()
                                        && let Ok(mut state) = self.state.lock()
                                    {
                                        state.selected_tab = SelectedTab::Summary;
                                    }
                                    #[cfg(debug_assertions)]
                                    self.debug_tab_targets.push(DebugTabTarget {
                                        name: "back_to_summary".to_string(),
                                        rect: back_resp.rect,
                                        hovered: back_resp.hovered(),
                                        contains_pointer: back_resp.contains_pointer(),
                                        clicked: back_resp.clicked(),
                                        pointer_button_down_on: back_resp.is_pointer_button_down_on(),
                                        interact_pointer_pos: back_resp.interact_pointer_pos(),
                                    });
                                    ui.add_space(Spacing::XXS);

                                    // ── Provider Detail ──
                                    if let Some((_, sel_p)) = visible_providers.iter()
                                        .find(|(_, p)| ProviderId::from_cli_name(&p.name) == Some(selected_id))
                                    {
                                        let (refresh, switch) = draw_provider_detail_card(
                                            ui, sel_p, &mut self.icon_cache,
                                            show_credits, show_as_used, hide_personal_info, ui_language,
                                        );
                                        manual_refresh_requested = refresh;
                                        account_switch_provider = switch;
                                    } else {
                                        // Selected provider no longer visible, go back
                                        if let Ok(mut state) = self.state.lock() {
                                            state.selected_tab = SelectedTab::Summary;
                                        }
                                    }
                                }
                            }
                        } else if has_enabled {
                            // ── Enabled providers, waiting for data ──
                            ui.add_space(Spacing::SM);
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.add_space(Spacing::XS);
                                ui.label(
                                    RichText::new(locale_text(self.settings.ui_language, LocaleKey::StateLoadingProviders))
                                        .size(FontSize::XS).color(Theme::TEXT_MUTED),
                                );
                            });
                            ui.add_space(Spacing::SM);
                        } else {
                            // ── First run — no providers configured ──
                            ui.add_space(Spacing::MD);
                            ui.label(
                                RichText::new("No providers configured")
                                    .size(FontSize::SM).color(Theme::TEXT_SECONDARY)
                            );
                            ui.add_space(Spacing::XXS);
                            ui.label(
                                RichText::new("Add a provider in Settings to start monitoring usage.")
                                    .size(FontSize::XS).color(Theme::TEXT_MUTED)
                            );
                            ui.add_space(Spacing::MD);
                            draw_horizontal_separator(ui, 0.0);
                            ui.add_space(Spacing::SM);

                            let button_size = Vec2::new(ui.available_width(), 26.0);
                            if ui.add(egui::Button::new(
                                RichText::new("Connect Claude").size(FontSize::XS).color(Color32::WHITE)
                            ).fill(provider_color("claude")).rounding(Rounding::same(Radius::SM)).min_size(button_size)).clicked() {
                                self.preferences_window.active_tab = super::preferences::PreferencesTab::Providers;
                                self.preferences_window.selected_provider = Some(ProviderId::Claude);
                                self.preferences_window.open();
                            }
                            ui.add_space(Spacing::XXS);
                            if ui.add(egui::Button::new(
                                RichText::new("Connect OpenAI").size(FontSize::XS).color(Color32::WHITE)
                            ).fill(provider_color("codex")).rounding(Rounding::same(Radius::SM)).min_size(button_size)).clicked() {
                                self.preferences_window.active_tab = super::preferences::PreferencesTab::Providers;
                                self.preferences_window.selected_provider = Some(ProviderId::Codex);
                                self.preferences_window.open();
                            }
                            ui.add_space(Spacing::XXS);
                            if ui.add(egui::Button::new(
                                RichText::new("Other Providers…").size(FontSize::XS).color(Theme::TEXT_SECONDARY)
                            ).fill(Color32::TRANSPARENT).stroke(Stroke::new(1.0, Theme::BORDER_SUBTLE)).rounding(Rounding::same(Radius::SM)).min_size(button_size)).clicked() {
                                self.preferences_window.active_tab = super::preferences::PreferencesTab::Providers;
                                self.preferences_window.open();
                            }
                            ui.add_space(Spacing::SM);
                        }

                        // Handle refresh / account switch
                        if manual_refresh_requested && !is_refreshing {
                            self.refresh_providers();
                        }
                        if let Some(provider_name) = account_switch_provider
                            && let Some(provider_id) = ProviderId::from_cli_name(&provider_name) {
                                self.preferences_window.active_tab = super::preferences::PreferencesTab::Providers;
                                self.preferences_window.selected_provider = Some(provider_id);
                                self.preferences_window.open();
                            }
                    }

                    // ════════════════════════════════════════════════════════════
                    // FOOTER
                    // ════════════════════════════════════════════════════════════
                    ui.add_space(Spacing::XXS);
                    draw_horizontal_separator(ui, 0.0);
                    ui.add_space(2.0);

                    ui.horizontal(|ui| {
                        let refresh_resp = ui.add(
                            egui::Button::new(
                                RichText::new(format!("↻  {}", locale_text(self.settings.ui_language, LocaleKey::ActionRefresh)))
                                    .size(FontSize::XS)
                                    .color(Theme::TEXT_SECONDARY)
                            )
                            .min_size(Vec2::new(74.0, 22.0))
                            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
                            .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 12)))
                            .rounding(Rounding::same(Radius::PILL))
                        );
                        if refresh_resp.clicked() && !is_refreshing {
                            self.refresh_providers();
                        }

                        #[cfg(debug_assertions)]
                        self.debug_tab_targets.push(DebugTabTarget {
                            name: "menu:refresh".to_string(),
                            rect: refresh_resp.rect,
                            hovered: refresh_resp.hovered(),
                            contains_pointer: refresh_resp.contains_pointer(),
                            clicked: refresh_resp.clicked(),
                            pointer_button_down_on: refresh_resp.is_pointer_button_down_on(),
                            interact_pointer_pos: refresh_resp.interact_pointer_pos(),
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let settings_resp = ui.add(
                                egui::Button::new(
                                    RichText::new(format!("⚙  {}", locale_text(self.settings.ui_language, LocaleKey::MenuSettings)))
                                        .size(FontSize::XS)
                                        .color(Theme::TEXT_SECONDARY)
                                )
                                .min_size(Vec2::new(78.0, 22.0))
                                .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
                                .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 12)))
                                .rounding(Rounding::same(Radius::PILL))
                            );
                            if settings_resp.clicked() {
                                self.preferences_window.open();
                            }

                            #[cfg(debug_assertions)]
                            self.debug_tab_targets.push(DebugTabTarget {
                                name: "menu:settings".to_string(),
                                rect: settings_resp.rect,
                                hovered: settings_resp.hovered(),
                                contains_pointer: settings_resp.contains_pointer(),
                                clicked: settings_resp.clicked(),
                                pointer_button_down_on: settings_resp.is_pointer_button_down_on(),
                                interact_pointer_pos: settings_resp.interact_pointer_pos(),
                            });
                        });
                    });
                    ui.add_space(4.0);
                }); // end ScrollArea
            });

        // Show preferences window
        self.preferences_window.show(ctx);

        let mut refresh_requested = self.preferences_window.take_refresh_requested();
        let previous_enabled_provider_ids = self.settings.get_enabled_provider_ids();
        let previous_ui_language = self.settings.ui_language;
        let previous_tray_icon_mode = self.settings.tray_icon_mode;

        // Atomically consume settings changes so the flag is cleared in both
        // PreferencesWindow and the shared viewport state in one shot.
        if let Some(new_settings) = self.preferences_window.take_settings_if_changed() {
            let language_changed = new_settings.ui_language != previous_ui_language;
            self.settings = new_settings;
            if let Err(e) = self.settings.save() {
                tracing::error!("Failed to save settings: {}", e);
            }
            let enabled_provider_ids = self.settings.get_enabled_provider_ids();
            if previous_enabled_provider_ids != enabled_provider_ids {
                refresh_requested = true;
            }
            if should_recreate_tray_manager(
                &previous_enabled_provider_ids,
                previous_tray_icon_mode,
                &self.settings,
            ) {
                self.tray_manager = match UnifiedTrayManager::new(&self.settings) {
                    Ok(tm) => Some(tm),
                    Err(e) => {
                        tracing::warn!("Failed to recreate tray manager: {}", e);
                        None
                    }
                };
            }
            // If language changed, refresh the tray menu/tooltip to update localized strings
            if language_changed && let Some(ref tray_manager) = self.tray_manager {
                tray_manager.refresh_language();
            }
        }

        // Check if preferences window requested a provider refresh
        if refresh_requested {
            self.refresh_providers();
        }

        if let Ok(state) = self.state.lock() {
            let runtime_provider_errors = state
                .providers
                .iter()
                .filter_map(|provider| {
                    let provider_id = ProviderId::from_cli_name(&provider.name)?;
                    let error = provider.error.as_ref()?.trim();
                    if error.is_empty() {
                        None
                    } else {
                        Some((provider_id, error.to_string()))
                    }
                })
                .collect();
            self.preferences_window
                .set_runtime_provider_errors(runtime_provider_errors);
        }

        // Reload preferences snapshot when refresh completes (is_refreshing transitions to false)
        if self.preferences_window.is_open {
            let is_refreshing = self.state.lock().map(|s| s.is_refreshing).unwrap_or(false);
            if self.was_refreshing && !is_refreshing {
                // Refresh just completed - reload the snapshot
                self.preferences_window.reload_snapshot();
            }
            self.was_refreshing = is_refreshing;
        }

        #[cfg(debug_assertions)]
        if let Some(path) = self.pending_test_state_dump_path.clone() {
            if self.pending_test_state_dump_delay_frames > 0 {
                self.pending_test_state_dump_delay_frames -= 1;
                ctx.request_repaint();
            } else {
                self.pending_test_state_dump_path = None;
                let (
                    preferences_tab_targets,
                    preferences_viewport_outer_rect,
                    preferences_settings,
                    api_key_status,
                    cookie_status,
                ) = self.preferences_window.debug_snapshot();
                let preferences_tab_targets = preferences_tab_targets
                    .into_iter()
                    .map(|target| DebugTabTarget {
                        name: target.name,
                        rect: target.rect,
                        hovered: target.hovered,
                        contains_pointer: target.contains_pointer,
                        clicked: target.clicked,
                        pointer_button_down_on: target.pointer_button_down_on,
                        interact_pointer_pos: target.interact_pointer_pos,
                    })
                    .collect::<Vec<_>>();
                let tray_state_json = self
                    .tray_manager
                    .as_ref()
                    .and_then(|tray| serde_json::to_string(&tray.debug_snapshot()).ok())
                    .unwrap_or_else(|| "null".to_string());
                if let Err(error) = write_debug_state_with_targets_file(
                    &path,
                    &selected_tab,
                    self.preferences_window.is_open,
                    &self.preferences_window.active_tab,
                    if self.latched_debug_tab_targets.is_empty() {
                        &self.last_debug_tab_targets
                    } else {
                        &self.latched_debug_tab_targets
                    },
                    self.latched_debug_viewport_outer_rect
                        .or(self.last_debug_viewport_outer_rect),
                    &preferences_tab_targets,
                    preferences_viewport_outer_rect,
                    &preferences_settings,
                    &tray_state_json,
                    self.is_popout_mode,
                    &api_key_status,
                    &cookie_status,
                    if self.latched_debug_tab_targets.is_empty() {
                        self.last_debug_pointer_snapshot
                    } else {
                        self.latched_debug_pointer_snapshot
                    },
                ) {
                    tracing::warn!(
                        "Failed to write test state dump to {}: {}",
                        path.display(),
                        error
                    );
                } else {
                    tracing::info!("Wrote test state dump to {}", path.display());
                }
            }
        }

        #[cfg(debug_assertions)]
        {
            self.last_debug_tab_targets = self.debug_tab_targets.clone();
            self.last_debug_viewport_outer_rect = self.debug_viewport_outer_rect;
            self.last_debug_pointer_snapshot = ctx.input(|i| DebugPointerSnapshot {
                latest_pos: i.pointer.latest_pos(),
                interact_pos: i.pointer.interact_pos(),
                primary_down: i.pointer.button_down(egui::PointerButton::Primary),
                primary_pressed: i.pointer.button_pressed(egui::PointerButton::Primary),
                primary_released: i.pointer.button_released(egui::PointerButton::Primary),
                primary_clicked: i.pointer.button_clicked(egui::PointerButton::Primary),
            });
            let should_latch_pointer = self.last_debug_pointer_snapshot.latest_pos.is_some()
                || self.last_debug_pointer_snapshot.interact_pos.is_some()
                || self.last_debug_pointer_snapshot.primary_down
                || self.last_debug_pointer_snapshot.primary_pressed
                || self.last_debug_pointer_snapshot.primary_released
                || self.last_debug_pointer_snapshot.primary_clicked;
            let should_latch_tabs = self.last_debug_tab_targets.iter().any(|target| {
                target.hovered
                    || target.contains_pointer
                    || target.clicked
                    || target.pointer_button_down_on
                    || target.interact_pointer_pos.is_some()
            });
            if should_latch_pointer || should_latch_tabs {
                self.latched_debug_tab_targets = self.last_debug_tab_targets.clone();
                self.latched_debug_viewport_outer_rect = self.last_debug_viewport_outer_rect;
                self.latched_debug_pointer_snapshot = self.last_debug_pointer_snapshot;
            }
        }
    }
}

#[cfg(debug_assertions)]
fn build_test_click_event_batches(
    pos: egui::Pos2,
    button: egui::PointerButton,
) -> [Vec<egui::Event>; 4] {
    [
        vec![egui::Event::PointerMoved(pos)],
        vec![egui::Event::PointerButton {
            pos,
            button,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        }],
        vec![egui::Event::PointerMoved(pos)],
        vec![egui::Event::PointerButton {
            pos,
            button,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    ]
}

/// Draw a provider detail card.
/// Structure: Header -> Divider -> Metrics (Session, Weekly, Model) -> Credits -> Cost
/// Returns (refresh_requested, account_switch_provider_name)
fn draw_provider_detail_card(
    ui: &mut egui::Ui,
    provider: &ProviderData,
    _icon_cache: &mut ProviderIconCache,
    show_credits_extra: bool,
    show_as_used: bool,
    hide_personal_info: bool,
    ui_language: Language,
) -> (bool, Option<String>) {
    let mut refresh_requested = false;
    let mut account_switch_requested: Option<String> = None;
    let brand_color = provider_color(&provider.name);
    let display_account = provider.account.as_ref().map(|account| {
        PersonalInfoRedactor::redact_email(Some(account.as_str()), hide_personal_info)
    });
    let subtitle_text = if let Some(error) = &provider.error {
        error.clone()
    } else if let Some(status_desc) = &provider.status_description
        && provider.status_level != StatusLevel::Operational
        && provider.status_level != StatusLevel::Unknown
    {
        let status_template = locale_text(ui_language, LocaleKey::StatusLabel);
        status_template.replace("{}", status_desc)
    } else {
        locale_text(ui_language, LocaleKey::StatusJustUpdated).to_string()
    };
    let subtitle_color = if provider.error.is_some() {
        Theme::RED
    } else if provider.status_level != StatusLevel::Operational
        && provider.status_level != StatusLevel::Unknown
    {
        status_color(provider.status_level)
    } else {
        Theme::TEXT_MUTED
    };

    ui.vertical(|ui| {
        // ═══════════════════════════════════════════════════════════════════
        // HEADER — provider identity block
        // ═══════════════════════════════════════════════════════════════════

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(&provider.display_name)
                    .size(FontSize::MD)
                    .color(Theme::TEXT_PRIMARY)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(account) = &display_account
                    && !account.is_empty()
                {
                    ui.label(
                        RichText::new(account)
                            .size(FontSize::SM)
                            .color(Theme::TEXT_SECONDARY),
                    );
                }
            });
        });

        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(&subtitle_text)
                    .size(FontSize::XS)
                    .color(subtitle_color),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(plan) = &provider.plan {
                    ui.label(
                        RichText::new(plan)
                            .size(FontSize::XS)
                            .color(Theme::TEXT_SECONDARY),
                    );
                }
            });
        });

        // ═══════════════════════════════════════════════════════════════════
        // DIVIDER — only if content follows
        // ═══════════════════════════════════════════════════════════════════
        let has_metrics = provider.session_percent.is_some()
            || provider.weekly_percent.is_some()
            || provider.monthly_percent.is_some();
        let has_credits = provider.credits_remaining.is_some();
        let has_cost = provider.cost_used.is_some();
        let has_usage_breakdown = !provider.usage_breakdown.is_empty();

        if has_metrics || provider.error.is_some() || has_credits || has_cost || has_usage_breakdown
        {
            ui.add_space(Spacing::XXS);
            draw_horizontal_separator(ui, 0.0);
        }

        // ═══════════════════════════════════════════════════════════════════
        // METRICS — each row: title, bar, info line
        // ═══════════════════════════════════════════════════════════════════
        if has_metrics {
            ui.add_space(Spacing::SM);

            // Session metric (primary) - no pace indicator for session
            if let Some(session_pct) = provider.session_percent {
                draw_metric_row(
                    ui,
                    MetricRow {
                        title: locale_text(ui_language, LocaleKey::ProviderSession),
                        percent: session_pct,
                        show_as_used,
                        reset_text: provider.session_reset.as_deref(),
                        color: brand_color,
                        pace_percent: None, // No pace for session
                        pace_lasts_to_reset: false,
                        ui_language,
                    },
                );
            }

            // Weekly metric (secondary) - includes pace indicator
            if let Some(weekly_pct) = provider.weekly_percent {
                ui.add_space(10.0);

                draw_metric_row(
                    ui,
                    MetricRow {
                        title: locale_text(ui_language, LocaleKey::ProviderWeekly),
                        percent: weekly_pct,
                        show_as_used,
                        reset_text: provider.weekly_reset.as_deref(),
                        color: brand_color,
                        pace_percent: provider.pace_percent,
                        pace_lasts_to_reset: provider.pace_lasts_to_reset,
                        ui_language,
                    },
                );
            }

            // Monthly metric (tertiary)
            if let Some(monthly_pct) = provider.monthly_percent {
                ui.add_space(10.0);

                draw_metric_row(
                    ui,
                    MetricRow {
                        title: locale_text(ui_language, LocaleKey::ProviderMonthly),
                        percent: monthly_pct,
                        show_as_used,
                        reset_text: provider.monthly_reset.as_deref(),
                        color: brand_color,
                        pace_percent: None, // No pace for monthly
                        pace_lasts_to_reset: false,
                        ui_language,
                    },
                );
            }

            // Model-specific metric
            if let Some(model_pct) = provider.model_percent {
                ui.add_space(10.0);

                let model_label = provider
                    .model_name
                    .as_deref()
                    .unwrap_or(locale_text(ui_language, LocaleKey::ProviderModel));
                draw_metric_row(
                    ui,
                    MetricRow {
                        title: model_label,
                        percent: model_pct,
                        show_as_used,
                        reset_text: None,
                        color: brand_color,
                        pace_percent: None, // No pace for model
                        pace_lasts_to_reset: false,
                        ui_language,
                    },
                );
            }

            ui.add_space(Spacing::XXS);
        } else if provider.error.is_some() {
            ui.add_space(Spacing::XXS);
            ui.label(
                RichText::new(locale_text(ui_language, LocaleKey::StatusUnableToGetUsage))
                    .size(FontSize::XS)
                    .color(Theme::TEXT_MUTED),
            );
            ui.add_space(Spacing::XXS);
        }

        // ═══════════════════════════════════════════════════════════════════
        // CREDITS SECTION
        // ═══════════════════════════════════════════════════════════════════
        if show_credits_extra && let Some(credits) = provider.credits_remaining {
            if has_metrics {
                draw_horizontal_separator(ui, 0.0);
            }
            ui.add_space(Spacing::SM);

            let bar_width = ui.available_width();

            ui.label(
                RichText::new(locale_text(ui_language, LocaleKey::CreditsTitle))
                    .size(FontSize::BASE)
                    .color(Theme::TEXT_PRIMARY)
                    .strong(),
            );

            if let Some(credits_pct) = provider.credits_percent {
                ui.add_space(4.0);
                let bar_height = 6.0;
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(bar_width, bar_height), egui::Sense::hover());

                ui.painter()
                    .rect_filled(rect, Rounding::same(3.0), Theme::progress_track());

                let fill_w = rect.width() * (credits_pct as f32 / 100.0).clamp(0.0, 1.0);
                if fill_w > 0.0 {
                    let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, bar_height));
                    ui.painter()
                        .rect_filled(fill_rect, Rounding::same(3.0), brand_color);
                }
            }

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let remaining_template = locale_text(ui_language, LocaleKey::RemainingAmount);
                ui.label(
                    RichText::new(remaining_template.replace("{:.2}", &format!("{:.2}", credits)))
                        .size(FontSize::XS)
                        .color(Theme::TEXT_PRIMARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(locale_text(ui_language, LocaleKey::Tokens1K))
                            .size(FontSize::XS)
                            .color(Theme::TEXT_SECONDARY),
                    );
                });
            });

            ui.add_space(Spacing::XXS);
            if draw_menu_item(
                ui,
                "⊕",
                locale_text(ui_language, LocaleKey::ActionBuyCredits),
            ) && let Some(ref url) = provider.dashboard_url
            {
                let _ = open::that(url);
            }

            // Credits history chart
            if !provider.credits_history.is_empty() {
                ui.add_space(8.0);
                let chart_points: Vec<ChartPoint> = provider
                    .credits_history
                    .iter()
                    .map(|(date, value)| ChartPoint::new(date.clone(), *value))
                    .collect();
                let mut chart = CreditsHistoryChart::new(chart_points);
                chart.show(ui);
            }
        }

        // ═══════════════════════════════════════════════════════════════════
        // USAGE BREAKDOWN
        // ═══════════════════════════════════════════════════════════════════
        if has_usage_breakdown {
            if has_metrics || has_credits {
                draw_horizontal_separator(ui, 0.0);
            }
            ui.add_space(Spacing::SM);

            ui.label(
                RichText::new(locale_text(ui_language, LocaleKey::SectionUsageBreakdown))
                    .size(FontSize::BASE)
                    .color(Theme::TEXT_PRIMARY)
                    .strong(),
            );
            ui.add_space(4.0);

            let mut chart = UsageBreakdownChart::new(provider.usage_breakdown.clone());
            chart.show(ui);
        }

        // ═══════════════════════════════════════════════════════════════════
        // COST
        // ═══════════════════════════════════════════════════════════════════
        if show_credits_extra && (provider.cost_used.is_some() || !provider.cost_history.is_empty())
        {
            if has_metrics || has_credits || has_usage_breakdown {
                draw_horizontal_separator(ui, 0.0);
            }
            ui.add_space(Spacing::SM);

            // Title: "Cost" - .font(.body).fontWeight(.medium)
            ui.label(
                RichText::new(locale_text(ui_language, LocaleKey::SectionCost))
                    .size(FontSize::BASE)
                    .color(Theme::TEXT_PRIMARY)
                    .strong(),
            );

            ui.add_space(4.0);

            // Cost details - Today and Last 30 days - .font(.caption)
            if !provider.cost_history.is_empty() {
                let total_30d: f64 = provider.cost_history.iter().map(|(_, cost)| cost).sum();
                let today_cost: f64 = provider.cost_history.last().map(|(_, c)| *c).unwrap_or(0.0);

                let today_template = locale_text(ui_language, LocaleKey::TodayCost);
                let total30d_template = locale_text(ui_language, LocaleKey::Last30DaysCost);
                ui.label(
                    RichText::new(today_template.replace("{:.2}", &format!("{:.2}", today_cost)))
                        .size(FontSize::XS)
                        .color(Theme::TEXT_PRIMARY),
                );
                ui.label(
                    RichText::new(total30d_template.replace("{:.2}", &format!("{:.2}", total_30d)))
                        .size(FontSize::XS)
                        .color(Theme::TEXT_PRIMARY),
                );
            } else if let Some(cost_used) = &provider.cost_used {
                ui.label(
                    RichText::new(cost_used)
                        .size(FontSize::XS)
                        .color(Theme::TEXT_PRIMARY),
                );
            }

            // Cost history chart
            if !provider.cost_history.is_empty() {
                ui.add_space(8.0);
                let chart_points: Vec<ChartPoint> = provider
                    .cost_history
                    .iter()
                    .map(|(date, cost)| ChartPoint::new(date.clone(), *cost))
                    .collect();
                let mut chart = CostHistoryChart::new(chart_points, brand_color);
                chart.show(ui);
            }
        }

        // ═══════════════════════════════════════════════════════════════════
        // ACTION LINKS
        // ═══════════════════════════════════════════════════════════════════
        let has_dashboard = provider.dashboard_url.is_some();
        let has_status_issue = provider.status_level != StatusLevel::Operational;
        let has_error = provider.error.is_some();

        if has_dashboard || has_status_issue || has_error {
            if has_metrics || has_credits || has_cost || has_usage_breakdown {
                draw_horizontal_separator(ui, 0.0);
            }
            ui.add_space(Spacing::XXS);

            // Vertical action links like macOS
            // Refresh button - first action
            if draw_menu_item(ui, "↻", locale_text(ui_language, LocaleKey::ActionRefresh)) {
                refresh_requested = true;
            }

            // Switch Account link - only show for providers that support token accounts
            if TokenAccountSupport::is_supported(
                ProviderId::from_cli_name(&provider.name).unwrap_or(ProviderId::Claude),
            ) && draw_menu_item(
                ui,
                "⇄",
                locale_text(ui_language, LocaleKey::ActionSwitchAccount),
            ) {
                account_switch_requested = Some(provider.name.clone());
            }

            // Usage Dashboard link
            if let Some(ref url) = provider.dashboard_url {
                let dashboard_url = url.clone();
                if draw_menu_item(
                    ui,
                    "↗",
                    locale_text(ui_language, LocaleKey::ActionUsageDashboard),
                ) {
                    let _ = open::that(&dashboard_url);
                }
            }

            // Status Page link
            if let Some(status_url) = get_status_page_url(&provider.name)
                && draw_menu_item(
                    ui,
                    "!",
                    locale_text(ui_language, LocaleKey::ActionStatusPage),
                )
            {
                let _ = open::that(status_url);
            }

            // Copy Error link
            if let Some(ref error) = provider.error {
                let error_text = error.clone();
                if draw_menu_item(
                    ui,
                    "⧉",
                    locale_text(ui_language, LocaleKey::ActionCopyError),
                ) && let Ok(mut clipboard) = arboard::Clipboard::new()
                {
                    let _ = clipboard.set_text(&error_text);
                }
            }

            ui.add_space(Spacing::XXS);
        }

        (refresh_requested, account_switch_requested)
    })
    .inner
}

/// Draw a horizontal separator with left padding
fn draw_horizontal_separator(ui: &mut egui::Ui, left_padding: f32) {
    ui.horizontal(|ui| {
        ui.add_space(left_padding);
        let sep_rect = ui.available_rect_before_wrap();
        let sep_width = sep_rect.width() - left_padding;
        ui.painter().hline(
            sep_rect.left()..=(sep_rect.left() + sep_width),
            sep_rect.top(),
            Stroke::new(1.0, Theme::SEPARATOR),
        );
    });
}

/// Draw a single metric row - macOS style matching SwiftUI MetricRow.
/// Structure: Title (.body.medium) → Progress bar (with optional pace marker) →
/// X% used | Pace status | Resets in Xh (.footnote)
struct MetricRow<'a> {
    title: &'a str,
    percent: f64,
    show_as_used: bool,
    reset_text: Option<&'a str>,
    color: Color32,
    /// Difference between actual and expected usage. Positive means ahead of expected, negative means behind.
    pace_percent: Option<f64>,
    /// Whether current usage will last until reset (on track or ahead).
    pace_lasts_to_reset: bool,
    ui_language: Language,
}

fn draw_metric_row(ui: &mut egui::Ui, metric: MetricRow<'_>) {
    let MetricRow {
        title,
        percent,
        show_as_used,
        reset_text,
        color,
        pace_percent,
        pace_lasts_to_reset,
        ui_language,
    } = metric;

    // Title
    ui.label(
        RichText::new(title)
            .size(FontSize::BASE)
            .color(Theme::TEXT_PRIMARY)
            .strong(),
    );

    ui.add_space(3.0);

    let display_percent = usage_display_percent(percent, show_as_used);
    let display_pace_percent = pace_percent.map(|pace| if show_as_used { pace } else { -pace });

    // Progress bar
    let bar_width = ui.available_width();
    let bar_height = 5.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(bar_width, bar_height), egui::Sense::hover());

    // Track
    ui.painter()
        .rect_filled(rect, Rounding::same(3.0), Theme::progress_track());

    // Fill
    let fill_w = rect.width() * (display_percent as f32 / 100.0).clamp(0.0, 1.0);
    if fill_w > 0.0 {
        let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, bar_height));
        ui.painter()
            .rect_filled(fill_rect, Rounding::same(3.0), color);
    }

    // Pace marker - thin vertical line showing expected usage position
    if let Some(pace_diff) = display_pace_percent {
        // pace_percent is the difference (actual - expected), so expected = actual - pace_diff
        let expected_position = (display_percent - pace_diff).clamp(0.0, 100.0);
        let marker_x = rect.min.x + rect.width() * (expected_position as f32 / 100.0);

        // Draw 2px wide vertical line in a contrasting color (white with transparency)
        let marker_color = Color32::from_rgba_unmultiplied(255, 255, 255, 180);
        let marker_width = 2.0;
        let marker_rect = Rect::from_min_size(
            egui::pos2(marker_x - marker_width / 2.0, rect.min.y),
            Vec2::new(marker_width, bar_height),
        );
        ui.painter()
            .rect_filled(marker_rect, Rounding::same(1.0), marker_color);
    }

    ui.add_space(3.0);

    // Info row: X% used (left) | Pace status | Resets in Xh (right)
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(usage_display_label(
                display_percent,
                show_as_used,
                ui_language,
            ))
            .size(FontSize::XS)
            .color(Theme::TEXT_PRIMARY),
        );

        // Pace status indicator
        if display_pace_percent.is_some() {
            ui.add_space(8.0);
            let (pace_text, pace_color) = if pace_lasts_to_reset {
                (
                    locale_text(ui_language, LocaleKey::PaceOnTrack),
                    Theme::GREEN,
                )
            } else {
                (
                    locale_text(ui_language, LocaleKey::PaceBehind),
                    Theme::YELLOW,
                )
            };
            ui.label(
                RichText::new(pace_text)
                    .size(FontSize::XS)
                    .color(pace_color),
            );
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(reset) = reset_text {
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        locale_text(ui_language, LocaleKey::MetricResetsIn),
                        reset
                    ))
                    .size(FontSize::XS)
                    .color(Theme::TEXT_SECONDARY),
                );
            }
        });
    });
}

/// Draw a menu item button - macOS style compact
fn draw_menu_item(ui: &mut egui::Ui, icon: &str, label: &str) -> bool {
    let available_width = ui.available_width();

    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(available_width, 24.0), egui::Sense::click());

    let is_hovered = response.hovered();

    let bg_fill = if is_hovered {
        Color32::from_rgba_unmultiplied(255, 255, 255, 10)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter()
        .rect_filled(rect, Rounding::same(Radius::SM), bg_fill);

    let text_color = if is_hovered {
        Theme::TEXT_PRIMARY
    } else {
        Theme::TEXT_SECONDARY
    };

    let icon_bg = Rect::from_center_size(
        egui::pos2(rect.min.x + 10.0, rect.center().y),
        Vec2::new(14.0, 14.0),
    );
    ui.painter().rect_filled(
        icon_bg,
        Rounding::same(Radius::PILL),
        Color32::from_rgba_unmultiplied(255, 255, 255, if is_hovered { 18 } else { 10 }),
    );

    let icon_pos = egui::pos2(icon_bg.center().x, rect.center().y);
    ui.painter().text(
        icon_pos,
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(FontSize::INDICATOR),
        text_color,
    );

    // Label
    let label_pos = egui::pos2(rect.min.x + 24.0, rect.center().y);
    ui.painter().text(
        label_pos,
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(FontSize::XS),
        text_color,
    );

    ui.painter().text(
        egui::pos2(rect.max.x - 8.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        "›",
        egui::FontId::proportional(FontSize::INDICATOR),
        if is_hovered {
            Theme::TEXT_SECONDARY
        } else {
            Theme::TEXT_MUTED
        },
    );

    response.clicked()
}

/// Run the application
pub fn run() -> anyhow::Result<()> {
    #[cfg(windows)]
    if let Some(message) = launch_block_reason(is_ssh_session(), is_remote_session()) {
        if is_remote_session() {
            show_remote_session_error_dialog();
        }
        anyhow::bail!(message);
    }

    // Delete any corrupted window state
    if let Some(data_dir) = dirs::data_dir() {
        let state_file = data_dir.join("CodexBar").join("data").join("app.ron");
        if state_file.exists() {
            let _ = std::fs::remove_file(&state_file);
        }
    }

    let options = build_native_options();
    append_launch_log(&format!(
        "native_ui::run starting with options: {:?}",
        options.viewport
    ));

    eframe::run_native(
        "CodexBar",
        options,
        Box::new(|cc| Ok(Box::new(CodexBarApp::new(cc)))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        PerProviderTrayRuntimeState, ProviderData, TrayRuntimeState, TrayUpdatePlan, append_font,
        build_test_click_event_batches, choose_tray_runtime_state, choose_tray_update_plan,
        debug_window_mode, launch_block_reason, main_window_summary_height, prepend_font,
        remote_session_error_message, should_auto_refresh, should_recreate_tray_manager,
        should_show_provider, ssh_session_error_message, startup_last_refresh, startup_popout_mode,
    };
    use crate::core::ProviderId;
    use crate::settings::{Settings, TrayIconMode};
    use crate::status::StatusLevel;
    use crate::tray::ProviderUsage;
    use egui::{Event, FontDefinitions, FontFamily, PointerButton, pos2};
    use std::time::{Duration, Instant};

    fn test_provider() -> ProviderData {
        ProviderData {
            name: "codex".to_string(),
            display_name: "Codex".to_string(),
            account: None,
            session_percent: None,
            session_reset: None,
            weekly_percent: None,
            weekly_reset: None,
            monthly_percent: None,
            monthly_reset: None,
            model_percent: None,
            model_name: None,
            plan: None,
            error: None,
            dashboard_url: None,
            pace_percent: None,
            pace_lasts_to_reset: false,
            cost_used: None,
            credits_remaining: None,
            credits_percent: None,
            status_level: StatusLevel::Unknown,
            status_description: None,
            cost_history: Vec::new(),
            credits_history: Vec::new(),
            usage_breakdown: Vec::new(),
        }
    }

    fn tray_usage(name: &str, session_percent: f64) -> ProviderUsage {
        ProviderUsage {
            name: name.to_string(),
            session_percent,
            weekly_percent: session_percent,
        }
    }

    #[test]
    fn append_font_keeps_existing_font_order() {
        let mut fonts = FontDefinitions::default();
        let family = FontFamily::Proportional;
        fonts
            .families
            .entry(family.clone())
            .or_default()
            .extend(["default-ui".to_string(), "emoji".to_string()]);

        append_font(&mut fonts, family.clone(), "cjk-fallback");

        let entries = fonts.families.get(&family).cloned().unwrap_or_default();
        assert_eq!(
            entries[entries.len().saturating_sub(3)..],
            [
                "default-ui".to_string(),
                "emoji".to_string(),
                "cjk-fallback".to_string()
            ]
        );
    }

    #[test]
    fn prepend_font_puts_new_font_first_without_duplication() {
        let mut fonts = FontDefinitions::default();
        let family = FontFamily::Proportional;
        fonts
            .families
            .entry(family.clone())
            .or_default()
            .extend(["default-ui".to_string(), "emoji".to_string()]);

        prepend_font(&mut fonts, family.clone(), "cjk-fallback");
        prepend_font(&mut fonts, family.clone(), "cjk-fallback");

        let entries = fonts.families.get(&family).cloned().unwrap_or_default();
        assert_eq!(entries.first().map(String::as_str), Some("cjk-fallback"));
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.as_str() == "cjk-fallback")
                .count(),
            1
        );
    }

    #[test]
    fn forced_visible_startup_stays_in_popup_mode() {
        assert!(!startup_popout_mode(true, true));
        assert!(!startup_popout_mode(true, false));
    }

    #[test]
    fn trayless_startup_uses_popout_mode_only_without_forced_visible() {
        assert!(startup_popout_mode(false, false));
        assert!(!startup_popout_mode(false, true));
    }

    #[test]
    fn debug_window_mode_reports_popup_and_popout() {
        assert_eq!(debug_window_mode(false), "popup");
        assert_eq!(debug_window_mode(true), "popout");
    }

    #[test]
    fn remote_session_error_mentions_cli_and_log() {
        let message = remote_session_error_message();

        assert!(message.contains("Remote Desktop"));
        assert!(message.contains("codexbar usage -p claude"));
        assert!(message.contains("%TEMP%\\codexbar_launch.log"));
    }

    #[test]
    fn ssh_session_error_mentions_cli_and_log() {
        let message = ssh_session_error_message();

        assert!(message.contains("SSH session"));
        assert!(message.contains("codexbar usage -p claude"));
        assert!(message.contains("%TEMP%\\codexbar_launch.log"));
    }

    #[test]
    fn launch_block_reason_prefers_ssh_message() {
        let message = launch_block_reason(true, true).expect("message");

        assert_eq!(message, ssh_session_error_message());
    }

    #[test]
    fn launch_block_reason_allows_local_interactive_session() {
        assert_eq!(launch_block_reason(false, false), None);
    }

    #[test]
    fn should_show_provider_when_any_usage_exists() {
        let mut provider = test_provider();
        provider.weekly_percent = Some(42.0);

        assert!(should_show_provider(&provider));
    }

    #[test]
    fn should_show_provider_when_error_exists() {
        let mut provider = test_provider();
        provider.error = Some("Auth required".to_string());

        assert!(should_show_provider(&provider));
    }

    #[test]
    fn should_hide_provider_without_usage_or_error() {
        let provider = test_provider();

        assert!(!should_show_provider(&provider));
    }

    #[test]
    fn summary_window_height_shrinks_for_small_provider_lists() {
        assert_eq!(main_window_summary_height(2, true), 272.0);
    }

    #[test]
    fn summary_window_height_caps_for_long_provider_lists() {
        assert_eq!(main_window_summary_height(12, true), 500.0);
    }

    #[test]
    fn startup_last_refresh_does_not_backdate_time() {
        let now = Instant::now();
        let startup = startup_last_refresh(now);

        assert_eq!(startup, now);
    }

    #[test]
    fn should_auto_refresh_on_initial_load_without_backdating() {
        let now = Instant::now();

        assert!(should_auto_refresh(true, false, now, 30, now));
    }

    #[test]
    fn should_auto_refresh_after_interval_elapsed() {
        let now = Instant::now();
        let last_refresh = now.checked_sub(Duration::from_secs(31)).unwrap_or(now);

        assert!(should_auto_refresh(false, false, last_refresh, 30, now));
        assert!(!should_auto_refresh(false, false, now, 30, now));
    }

    #[test]
    fn test_click_batches_match_staged_egui_sequence() {
        let pos = pos2(222.5, 32.0);
        let batches = build_test_click_event_batches(pos, PointerButton::Primary);

        assert!(matches!(batches[0].as_slice(), [
            Event::PointerMoved(moved_pos),
        ] if *moved_pos == pos));
        assert!(matches!(batches[1].as_slice(), [
            Event::PointerButton { pos: pressed_pos, button: PointerButton::Primary, pressed: true, .. },
        ] if *pressed_pos == pos));
        assert!(matches!(batches[2].as_slice(), [
            Event::PointerMoved(moved_pos),
        ] if *moved_pos == pos));
        assert!(matches!(batches[3].as_slice(), [
            Event::PointerButton { pos: released_pos, button: PointerButton::Primary, pressed: false, .. },
        ] if *released_pos == pos));
    }

    #[test]
    fn tray_plan_merges_when_merge_setting_is_enabled() {
        let settings = Settings {
            merge_tray_icons: true,
            tray_icon_mode: TrayIconMode::Single,
            ..Settings::default()
        };
        let usages = vec![tray_usage("Codex", 10.0), tray_usage("Claude", 60.0)];

        let plan = choose_tray_update_plan(&usages, &settings);

        assert!(matches!(plan, Some(TrayUpdatePlan::Merged(items)) if items.len() == 2));
    }

    #[test]
    fn tray_plan_merges_in_per_provider_mode() {
        let settings = Settings {
            tray_icon_mode: TrayIconMode::PerProvider,
            ..Settings::default()
        };
        let usages = vec![tray_usage("Codex", 10.0), tray_usage("Claude", 60.0)];

        let plan = choose_tray_update_plan(&usages, &settings);

        assert!(matches!(plan, Some(TrayUpdatePlan::Merged(items)) if items.len() == 2));
    }

    #[test]
    fn tray_plan_picks_highest_usage_in_minimal_single_mode() {
        let settings = Settings {
            merge_tray_icons: false,
            tray_icon_mode: TrayIconMode::Single,
            menu_bar_display_mode: "minimal".to_string(),
            ..Settings::default()
        };
        let usages = vec![tray_usage("Codex", 10.0), tray_usage("Claude", 60.0)];

        let plan = choose_tray_update_plan(&usages, &settings);

        assert!(matches!(
            plan,
            Some(TrayUpdatePlan::Single(provider))
                if provider.name == "Claude" && (provider.session_percent - 60.0).abs() < f64::EPSILON
        ));
    }

    #[test]
    fn tray_plan_picks_highest_usage_when_display_setting_is_enabled() {
        let settings = Settings {
            merge_tray_icons: false,
            tray_icon_mode: TrayIconMode::Single,
            menu_bar_display_mode: "detailed".to_string(),
            menu_bar_shows_highest_usage: true,
            ..Settings::default()
        };
        let usages = vec![tray_usage("Codex", 10.0), tray_usage("Claude", 60.0)];

        let plan = choose_tray_update_plan(&usages, &settings);

        assert!(matches!(
            plan,
            Some(TrayUpdatePlan::Single(provider))
                if provider.name == "Claude" && (provider.session_percent - 60.0).abs() < f64::EPSILON
        ));
    }

    #[test]
    fn tray_plan_uses_first_provider_in_non_minimal_single_mode() {
        let settings = Settings {
            merge_tray_icons: false,
            tray_icon_mode: TrayIconMode::Single,
            menu_bar_display_mode: "detailed".to_string(),
            ..Settings::default()
        };
        let usages = vec![tray_usage("Codex", 10.0), tray_usage("Claude", 60.0)];

        let plan = choose_tray_update_plan(&usages, &settings);

        assert!(matches!(
            plan,
            Some(TrayUpdatePlan::Single(provider))
                if provider.name == "Codex" && (provider.session_percent - 10.0).abs() < f64::EPSILON
        ));
    }

    #[test]
    fn tray_runtime_state_prefers_error_in_single_merged_mode() {
        let settings = Settings {
            merge_tray_icons: true,
            tray_icon_mode: TrayIconMode::Single,
            ..Settings::default()
        };
        let mut codex = test_provider();
        codex.name = "codex".to_string();
        codex.display_name = "Codex".to_string();
        codex.session_percent = Some(42.0);
        codex.weekly_percent = Some(42.0);
        let claude = ProviderData::from_error(ProviderId::Claude, "Token expired".to_string());

        let plan = choose_tray_runtime_state(&[codex, claude], &settings);

        assert!(matches!(
            plan,
            Some(TrayRuntimeState::SingleError { provider_name, error })
                if provider_name == "Claude" && error == "Token expired"
        ));
    }

    #[test]
    fn tray_runtime_state_emits_per_provider_errors() {
        let settings = Settings {
            tray_icon_mode: TrayIconMode::PerProvider,
            ..Settings::default()
        };
        let mut codex = test_provider();
        codex.name = "codex".to_string();
        codex.display_name = "Codex".to_string();
        codex.session_percent = Some(42.0);
        codex.weekly_percent = Some(42.0);
        let claude = ProviderData::from_error(ProviderId::Claude, "Auth failed".to_string());

        let plan = choose_tray_runtime_state(&[codex, claude], &settings);

        assert!(matches!(
            plan,
            Some(TrayRuntimeState::PerProvider(states))
                if states.iter().any(|state| matches!(
                    state,
                    PerProviderTrayRuntimeState::Error { provider_id, error }
                        if *provider_id == ProviderId::Claude && error == "Auth failed"
                ))
        ));
    }

    #[test]
    fn tray_manager_recreates_when_mode_changes() {
        let previous_enabled = Settings::default().get_enabled_provider_ids();
        let settings = Settings {
            tray_icon_mode: TrayIconMode::PerProvider,
            ..Settings::default()
        };

        assert!(should_recreate_tray_manager(
            &previous_enabled,
            TrayIconMode::Single,
            &settings
        ));
    }

    #[test]
    fn tray_manager_recreates_when_enabled_provider_set_changes() {
        let previous_enabled = Settings::default().get_enabled_provider_ids();
        let mut settings = Settings::default();
        settings.enabled_providers.remove("claude");

        assert!(should_recreate_tray_manager(
            &previous_enabled,
            TrayIconMode::Single,
            &settings
        ));
    }

    #[test]
    fn tray_manager_stays_when_mode_and_providers_match() {
        let previous_enabled = Settings::default().get_enabled_provider_ids();
        let settings = Settings::default();

        assert!(!should_recreate_tray_manager(
            &previous_enabled,
            TrayIconMode::Single,
            &settings
        ));
    }
}
