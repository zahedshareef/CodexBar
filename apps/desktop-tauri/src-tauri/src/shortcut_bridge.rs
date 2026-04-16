//! Global keyboard shortcut registration for toggling the tray panel.
//!
//! Reads the persisted `global_shortcut` setting (e.g. `"Ctrl+Shift+U"`)
//! and registers it through the Tauri global-shortcut plugin. The shortcut
//! toggles the tray panel via the surface state machine.

use std::sync::Mutex;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::shell;
use crate::state::AppState;
use crate::surface::SurfaceMode;
use crate::surface_target::SurfaceTarget;

/// Parse a settings shortcut string (e.g. `"Ctrl+Shift+U"`) into a Tauri `Shortcut`.
///
/// Public so that callers (e.g. the settings command) can validate a shortcut
/// string before persisting it.
pub fn parse_shortcut(s: &str) -> Option<Shortcut> {
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
                let current = {
                    let st = app.state::<Mutex<AppState>>();
                    st.lock().unwrap().surface_machine.current()
                };

                if current == SurfaceMode::TrayPanel {
                    let _ = shell::hide_to_tray(app);
                } else {
                    let position = shell::shortcut_panel_position(app);
                    let _ = shell::transition_to_target(
                        app,
                        SurfaceMode::TrayPanel,
                        SurfaceTarget::Summary,
                        position,
                    );
                }
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

/// Live-swap the global shortcut: unregister `old` and register `new`.
///
/// Called from `update_settings` when the user changes the shortcut in the
/// Settings UI. Returns `Err` with a user-facing message when the new
/// shortcut string cannot be parsed or registration fails. On error the old
/// shortcut is left registered (best-effort).
pub fn reregister_shortcut(app: &AppHandle, old: &str, new: &str) -> Result<(), String> {
    let new_shortcut = parse_shortcut(new).ok_or_else(|| {
        format!("Invalid shortcut \"{new}\". Use a combination like Ctrl+Shift+U.")
    })?;

    // Unregister the previous shortcut (ignore errors — it may not be registered).
    if let Some(old_shortcut) = parse_shortcut(old) {
        let _ = app.global_shortcut().unregister(old_shortcut);
    }

    app.global_shortcut().register(new_shortcut).map_err(|e| {
        // Best-effort: try to restore the old shortcut.
        if let Some(old_shortcut) = parse_shortcut(old) {
            let _ = app.global_shortcut().register(old_shortcut);
        }
        format!("Failed to register shortcut \"{new}\": {e}")
    })?;

    tracing::info!("Re-registered global shortcut: {old} → {new}");
    Ok(())
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

    #[test]
    fn parse_super_key() {
        let s = parse_shortcut("Super+A").unwrap();
        assert_eq!(s.key, Code::KeyA);
        assert!(s.mods.contains(Modifiers::SUPER));
    }

    #[test]
    fn parse_win_alias() {
        let s = parse_shortcut("Win+Z").unwrap();
        assert_eq!(s.key, Code::KeyZ);
        assert!(s.mods.contains(Modifiers::SUPER));
    }

    #[test]
    fn parse_meta_alias() {
        let s = parse_shortcut("Meta+F12").unwrap();
        assert_eq!(s.key, Code::F12);
        assert!(s.mods.contains(Modifiers::SUPER));
    }

    #[test]
    fn parse_special_keys() {
        assert_eq!(parse_shortcut("Ctrl+Space").unwrap().key, Code::Space);
        assert_eq!(parse_shortcut("Alt+Enter").unwrap().key, Code::Enter);
        assert_eq!(parse_shortcut("Ctrl+Tab").unwrap().key, Code::Tab);
        assert_eq!(parse_shortcut("Ctrl+Escape").unwrap().key, Code::Escape);
    }

    #[test]
    fn parse_return_alias() {
        assert_eq!(parse_shortcut("Ctrl+Return").unwrap().key, Code::Enter);
    }

    #[test]
    fn parse_esc_alias() {
        assert_eq!(parse_shortcut("Ctrl+Esc").unwrap().key, Code::Escape);
    }

    #[test]
    fn parse_control_alias() {
        let s = parse_shortcut("Control+Shift+B").unwrap();
        assert!(s.mods.contains(Modifiers::CONTROL));
        assert!(s.mods.contains(Modifiers::SHIFT));
        assert_eq!(s.key, Code::KeyB);
    }

    #[test]
    fn parse_all_function_keys() {
        for i in 1..=12u8 {
            let input = format!("F{i}");
            let s = parse_shortcut(&input);
            assert!(s.is_some(), "F{i} should parse");
        }
    }

    #[test]
    fn parse_no_key_returns_none() {
        // Only modifiers, no actual key
        assert!(parse_shortcut("Ctrl+Shift").is_none());
    }

    #[test]
    fn parse_whitespace_tolerant() {
        let s = parse_shortcut("Ctrl + Shift + U").unwrap();
        assert_eq!(s.key, Code::KeyU);
        assert!(s.mods.contains(Modifiers::CONTROL));
        assert!(s.mods.contains(Modifiers::SHIFT));
    }
}
