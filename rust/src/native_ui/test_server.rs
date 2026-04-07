//! Test input server for automated UI testing without moving the real cursor.
//!
//! Listens on a local TCP port and accepts JSON commands to inject synthetic
//! pointer events into the egui event loop.

use eframe::egui;
use serde::Deserialize;
use std::io::Read;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

/// A synthetic input event to inject into the egui loop.
pub enum TestInput {
    OpenWindow,
    SelectTab { tab: String },
    SelectPreferencesTab { tab: String },
    SetProviderEnabled { provider: String, enabled: bool },
    SetRefreshInterval { seconds: u64 },
    SetDisplaySetting { name: String, enabled: bool },
    SetApiKeyInput { provider: String, value: String },
    SubmitApiKey,
    SetCookieInput { provider: String, value: String },
    SubmitCookie,
    SaveState { path: String },
    SaveScreenshot { path: String },
    SavePreferencesScreenshot { path: String },
    Click { x: f32, y: f32 },
    DoubleClick { x: f32, y: f32 },
    RightClick { x: f32, y: f32 },
}

/// Thread-safe queue of pending test inputs.
pub type TestInputQueue = Arc<Mutex<Vec<TestInput>>>;

/// Create a new empty test input queue.
pub fn create_queue() -> TestInputQueue {
    Arc::new(Mutex::new(Vec::new()))
}

/// Start a TCP server on `127.0.0.1:19400` that accepts JSON test commands.
///
/// Each connection can send one JSON object per line:
/// ```json
/// {"type":"open_window"}
/// {"type":"select_tab","tab":"claude"}
/// {"type":"select_preferences_tab","tab":"about"}
/// {"type":"set_provider_enabled","provider":"claude","enabled":false}
/// {"type":"set_refresh_interval","seconds":300}
/// {"type":"set_display_setting","name":"show_as_used","enabled":false}
/// {"type":"set_api_key_input","provider":"openrouter","value":"sk-test"}
/// {"type":"submit_api_key"}
/// {"type":"set_cookie_input","provider":"claude","value":"sessionKey=test"}
/// {"type":"submit_cookie"}
/// {"type":"save_state","path":"C:\\Users\\mac\\Desktop\\codexbar-state.json"}
/// {"type":"save_screenshot","path":"C:\\Users\\mac\\Desktop\\codexbar-probe.png"}
/// {"type":"save_preferences_screenshot","path":"C:\\Users\\mac\\Desktop\\codexbar-preferences.png"}
/// {"type":"click","x":100,"y":200}
/// {"type":"double_click","x":100,"y":200}
/// {"type":"right_click","x":100,"y":200}
/// ```
pub fn start_server(queue: TestInputQueue, repaint_ctx: egui::Context) {
    std::thread::spawn(move || {
        let listener = match TcpListener::bind("127.0.0.1:19400") {
            Ok(l) => l,
            Err(e) => {
                tracing::warn!("Test server failed to bind: {e}");
                return;
            }
        };
        tracing::info!("Test input server listening on 127.0.0.1:19400");

        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Test server accept error: {e}");
                    continue;
                }
            };

            let mut buf = String::new();
            if let Err(e) = stream.read_to_string(&mut buf) {
                tracing::warn!("Test server read error: {e}");
                continue;
            }

            for line in buf.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                match parse_test_input(line) {
                    Some(input) => {
                        if let Ok(mut q) = queue.lock() {
                            q.push(input);
                        }
                        repaint_ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        repaint_ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        repaint_ctx.request_repaint();
                    }
                    None => {
                        tracing::warn!("Test server: unrecognised input: {line}");
                    }
                }
            }
        }
    });
}

fn parse_test_input(json: &str) -> Option<TestInput> {
    #[derive(Deserialize)]
    struct RawTestInput {
        #[serde(rename = "type")]
        kind: String,
        path: Option<String>,
        tab: Option<String>,
        name: Option<String>,
        provider: Option<String>,
        value: Option<String>,
        enabled: Option<bool>,
        seconds: Option<u64>,
        x: Option<f32>,
        y: Option<f32>,
    }

    let input: RawTestInput = serde_json::from_str(json).ok()?;
    match input.kind.as_str() {
        "open_window" => Some(TestInput::OpenWindow),
        "save_screenshot" => Some(TestInput::SaveScreenshot { path: input.path? }),
        "save_preferences_screenshot" => {
            Some(TestInput::SavePreferencesScreenshot { path: input.path? })
        }
        "select_tab" => Some(TestInput::SelectTab { tab: input.tab? }),
        "select_preferences_tab" => Some(TestInput::SelectPreferencesTab { tab: input.tab? }),
        "set_provider_enabled" => Some(TestInput::SetProviderEnabled {
            provider: input.provider?,
            enabled: input.enabled?,
        }),
        "set_refresh_interval" => Some(TestInput::SetRefreshInterval {
            seconds: input.seconds?,
        }),
        "set_display_setting" => Some(TestInput::SetDisplaySetting {
            name: input.name?,
            enabled: input.enabled?,
        }),
        "set_api_key_input" => Some(TestInput::SetApiKeyInput {
            provider: input.provider?,
            value: input.value?,
        }),
        "submit_api_key" => Some(TestInput::SubmitApiKey),
        "set_cookie_input" => Some(TestInput::SetCookieInput {
            provider: input.provider?,
            value: input.value?,
        }),
        "submit_cookie" => Some(TestInput::SubmitCookie),
        "save_state" => Some(TestInput::SaveState { path: input.path? }),
        "double_click" => Some(TestInput::DoubleClick {
            x: input.x?,
            y: input.y?,
        }),
        "right_click" => Some(TestInput::RightClick {
            x: input.x?,
            y: input.y?,
        }),
        "click" => Some(TestInput::Click {
            x: input.x?,
            y: input.y?,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_test_input, TestInput};

    #[test]
    fn parses_open_window_without_coordinates() {
        assert!(matches!(
            parse_test_input(r#"{"type":"open_window"}"#),
            Some(TestInput::OpenWindow)
        ));
    }

    #[test]
    fn parses_click_with_coordinates() {
        assert!(matches!(
            parse_test_input(r#"{"type":"click","x":220,"y":34}"#),
            Some(TestInput::Click { x, y }) if (x - 220.0).abs() < f32::EPSILON && (y - 34.0).abs() < f32::EPSILON
        ));
    }

    #[test]
    fn parses_save_screenshot_path() {
        assert!(matches!(
            parse_test_input(r#"{"type":"save_screenshot","path":"C:\\temp\\probe.png"}"#),
            Some(TestInput::SaveScreenshot { path }) if path == r#"C:\temp\probe.png"#
        ));
    }

    #[test]
    fn parses_save_preferences_screenshot_path() {
        assert!(matches!(
            parse_test_input(r#"{"type":"save_preferences_screenshot","path":"C:\\temp\\prefs.png"}"#),
            Some(TestInput::SavePreferencesScreenshot { path }) if path == r#"C:\temp\prefs.png"#
        ));
    }

    #[test]
    fn parses_save_state_path() {
        assert!(matches!(
            parse_test_input(r#"{"type":"save_state","path":"C:\\temp\\state.json"}"#),
            Some(TestInput::SaveState { path }) if path == r#"C:\temp\state.json"#
        ));
    }

    #[test]
    fn parses_select_tab_name() {
        assert!(matches!(
            parse_test_input(r#"{"type":"select_tab","tab":"claude"}"#),
            Some(TestInput::SelectTab { tab }) if tab == "claude"
        ));
    }

    #[test]
    fn parses_select_preferences_tab_name() {
        assert!(matches!(
            parse_test_input(r#"{"type":"select_preferences_tab","tab":"about"}"#),
            Some(TestInput::SelectPreferencesTab { tab }) if tab == "about"
        ));
    }

    #[test]
    fn parses_set_api_key_input() {
        assert!(matches!(
            parse_test_input(r#"{"type":"set_api_key_input","provider":"openrouter","value":"sk-test"}"#),
            Some(TestInput::SetApiKeyInput { provider, value })
                if provider == "openrouter" && value == "sk-test"
        ));
    }

    #[test]
    fn parses_set_provider_enabled() {
        assert!(matches!(
            parse_test_input(r#"{"type":"set_provider_enabled","provider":"claude","enabled":false}"#),
            Some(TestInput::SetProviderEnabled { provider, enabled })
                if provider == "claude" && !enabled
        ));
    }

    #[test]
    fn parses_set_refresh_interval() {
        assert!(matches!(
            parse_test_input(r#"{"type":"set_refresh_interval","seconds":300}"#),
            Some(TestInput::SetRefreshInterval { seconds }) if seconds == 300
        ));
    }

    #[test]
    fn parses_set_display_setting() {
        assert!(matches!(
            parse_test_input(r#"{"type":"set_display_setting","name":"show_as_used","enabled":false}"#),
            Some(TestInput::SetDisplaySetting { name, enabled })
                if name == "show_as_used" && !enabled
        ));
    }

    #[test]
    fn parses_submit_api_key() {
        assert!(matches!(
            parse_test_input(r#"{"type":"submit_api_key"}"#),
            Some(TestInput::SubmitApiKey)
        ));
    }

    #[test]
    fn parses_set_cookie_input() {
        assert!(matches!(
            parse_test_input(r#"{"type":"set_cookie_input","provider":"claude","value":"sessionKey=test"}"#),
            Some(TestInput::SetCookieInput { provider, value })
                if provider == "claude" && value == "sessionKey=test"
        ));
    }

    #[test]
    fn parses_submit_cookie() {
        assert!(matches!(
            parse_test_input(r#"{"type":"submit_cookie"}"#),
            Some(TestInput::SubmitCookie)
        ));
    }
}
