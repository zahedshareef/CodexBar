//! Global keyboard shortcut registration for toggling the tray panel.
//!
//! Reads the persisted `global_shortcut` setting (e.g. `"Ctrl+Shift+U"`)
//! and registers it through the Tauri global-shortcut plugin. The shortcut
//! toggles the tray panel via the surface state machine.

use tauri::AppHandle;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::shell;

/// Parse a settings shortcut string (e.g. `"Ctrl+Shift+U"`) into a Tauri `Shortcut`.
fn parse_shortcut(s: &str) -> Option<Shortcut> {
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
    if parts.is_empty() {
        return None;
    }

    let mut mods = Modifiers::empty();
    let mut key_code: Option<Code> = None;

    for part in &parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "shift" => mods |= Modifiers::SHIFT,
            "alt" => mods |= Modifiers::ALT,
            "super" | "win" | "meta" => mods |= Modifiers::SUPER,
            k if k.len() == 1 => {
                key_code = match k.chars().next()? {
                    'a' => Some(Code::KeyA),
                    'b' => Some(Code::KeyB),
                    'c' => Some(Code::KeyC),
                    'd' => Some(Code::KeyD),
                    'e' => Some(Code::KeyE),
                    'f' => Some(Code::KeyF),
                    'g' => Some(Code::KeyG),
                    'h' => Some(Code::KeyH),
                    'i' => Some(Code::KeyI),
                    'j' => Some(Code::KeyJ),
                    'k' => Some(Code::KeyK),
                    'l' => Some(Code::KeyL),
                    'm' => Some(Code::KeyM),
                    'n' => Some(Code::KeyN),
                    'o' => Some(Code::KeyO),
                    'p' => Some(Code::KeyP),
                    'q' => Some(Code::KeyQ),
                    'r' => Some(Code::KeyR),
                    's' => Some(Code::KeyS),
                    't' => Some(Code::KeyT),
                    'u' => Some(Code::KeyU),
                    'v' => Some(Code::KeyV),
                    'w' => Some(Code::KeyW),
                    'x' => Some(Code::KeyX),
                    'y' => Some(Code::KeyY),
                    'z' => Some(Code::KeyZ),
                    '0' => Some(Code::Digit0),
                    '1' => Some(Code::Digit1),
                    '2' => Some(Code::Digit2),
                    '3' => Some(Code::Digit3),
                    '4' => Some(Code::Digit4),
                    '5' => Some(Code::Digit5),
                    '6' => Some(Code::Digit6),
                    '7' => Some(Code::Digit7),
                    '8' => Some(Code::Digit8),
                    '9' => Some(Code::Digit9),
                    _ => None,
                };
            }
            "f1" => key_code = Some(Code::F1),
            "f2" => key_code = Some(Code::F2),
            "f3" => key_code = Some(Code::F3),
            "f4" => key_code = Some(Code::F4),
            "f5" => key_code = Some(Code::F5),
            "f6" => key_code = Some(Code::F6),
            "f7" => key_code = Some(Code::F7),
            "f8" => key_code = Some(Code::F8),
            "f9" => key_code = Some(Code::F9),
            "f10" => key_code = Some(Code::F10),
            "f11" => key_code = Some(Code::F11),
            "f12" => key_code = Some(Code::F12),
            "space" => key_code = Some(Code::Space),
            "enter" | "return" => key_code = Some(Code::Enter),
            "escape" | "esc" => key_code = Some(Code::Escape),
            "tab" => key_code = Some(Code::Tab),
            _ => {}
        }
    }

    let key = key_code?;
    let m = if mods.is_empty() { None } else { Some(mods) };
    Some(Shortcut::new(m, key))
}

/// Build the Tauri global-shortcut plugin with the tray-panel toggle handler.
pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let position = shell::shortcut_panel_position(app);
                shell::toggle_tray_panel(app, position);
            }
        })
        .build()
}

/// Register the persisted global shortcut from settings.
///
/// Call this in the Tauri `setup` closure after the plugin is initialised.
pub fn register(app: &AppHandle) {
    let settings = codexbar::settings::Settings::load();
    let shortcut_str = &settings.global_shortcut;

    let Some(shortcut) = parse_shortcut(shortcut_str) else {
        tracing::warn!("Could not parse global shortcut: {shortcut_str}");
        return;
    };

    match app.global_shortcut().register(shortcut) {
        Ok(()) => {
            tracing::info!("Registered global shortcut: {shortcut_str}");
        }
        Err(e) => {
            tracing::warn!("Failed to register global shortcut '{shortcut_str}': {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ctrl_shift_u() {
        let s = parse_shortcut("Ctrl+Shift+U").unwrap();
        assert_eq!(s.key, Code::KeyU);
        assert!(s.mods.contains(Modifiers::CONTROL));
        assert!(s.mods.contains(Modifiers::SHIFT));
    }

    #[test]
    fn parse_alt_f1() {
        let s = parse_shortcut("Alt+F1").unwrap();
        assert_eq!(s.key, Code::F1);
        assert!(s.mods.contains(Modifiers::ALT));
    }

    #[test]
    fn parse_single_letter_no_mods() {
        let s = parse_shortcut("A").unwrap();
        assert_eq!(s.key, Code::KeyA);
        assert!(s.mods.is_empty());
    }

    #[test]
    fn parse_empty_returns_none() {
        assert!(parse_shortcut("").is_none());
    }

    #[test]
    fn parse_invalid_returns_none() {
        assert!(parse_shortcut("Ctrl+???").is_none());
    }

    #[test]
    fn parse_digit() {
        let s = parse_shortcut("Ctrl+5").unwrap();
        assert_eq!(s.key, Code::Digit5);
        assert!(s.mods.contains(Modifiers::CONTROL));
    }
}
