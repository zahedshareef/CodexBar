//! Global keyboard shortcuts for CodexBar
//!
//! Provides system-wide hotkeys to open the menu

#![allow(dead_code)]

use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager,
    hotkey::{Code, HotKey, Modifiers},
};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// Keyboard shortcut manager
pub struct ShortcutManager {
    manager: GlobalHotKeyManager,
    open_menu_id: u32,
    triggered: Arc<AtomicBool>,
}

impl ShortcutManager {
    /// Create a new shortcut manager with default shortcuts
    pub fn new() -> anyhow::Result<Self> {
        let manager = GlobalHotKeyManager::new()?;

        // Default shortcut: Ctrl+Shift+U (U for Usage)
        let open_menu = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyU);
        let open_menu_id = open_menu.id();

        manager.register(open_menu)?;

        Ok(Self {
            manager,
            open_menu_id,
            triggered: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Register a custom shortcut for opening the menu
    pub fn set_open_menu_shortcut(
        &mut self,
        modifiers: Modifiers,
        key: Code,
    ) -> anyhow::Result<()> {
        // Unregister old shortcut
        let old = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyU);
        let _ = self.manager.unregister(old);

        // Register new shortcut
        let new_hotkey = HotKey::new(Some(modifiers), key);
        self.open_menu_id = new_hotkey.id();
        self.manager.register(new_hotkey)?;

        Ok(())
    }

    /// Check if the open menu shortcut was triggered
    /// Call this in your event loop
    pub fn check_events(&self) -> bool {
        if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.id == self.open_menu_id {
                return true;
            }
        }
        false
    }

    /// Get the triggered flag (for async usage)
    pub fn triggered_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.triggered)
    }
}

impl Drop for ShortcutManager {
    fn drop(&mut self) {
        // Unregister all shortcuts on drop
        let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyU);
        let _ = self.manager.unregister(hotkey);
    }
}

/// Parse a shortcut string like "Ctrl+Shift+U" into modifiers and key
pub fn parse_shortcut(s: &str) -> Option<(Modifiers, Code)> {
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
    if parts.is_empty() {
        return None;
    }

    let mut modifiers = Modifiers::empty();
    let mut key_code = None;

    for part in parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "shift" => modifiers |= Modifiers::SHIFT,
            "alt" => modifiers |= Modifiers::ALT,
            "super" | "win" | "meta" => modifiers |= Modifiers::SUPER,
            // Single letter keys
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
            // Function keys
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
            // Special keys
            "space" => key_code = Some(Code::Space),
            "enter" | "return" => key_code = Some(Code::Enter),
            "escape" | "esc" => key_code = Some(Code::Escape),
            "tab" => key_code = Some(Code::Tab),
            _ => {}
        }
    }

    key_code.map(|k| (modifiers, k))
}

/// Format a shortcut for display
pub fn format_shortcut(modifiers: Modifiers, key: Code) -> String {
    let mut parts = Vec::new();

    if modifiers.contains(Modifiers::CONTROL) {
        parts.push("Ctrl");
    }
    if modifiers.contains(Modifiers::SHIFT) {
        parts.push("Shift");
    }
    if modifiers.contains(Modifiers::ALT) {
        parts.push("Alt");
    }
    if modifiers.contains(Modifiers::SUPER) {
        parts.push("Win");
    }

    let key_name = match key {
        Code::KeyA => "A",
        Code::KeyB => "B",
        Code::KeyC => "C",
        Code::KeyD => "D",
        Code::KeyE => "E",
        Code::KeyF => "F",
        Code::KeyG => "G",
        Code::KeyH => "H",
        Code::KeyI => "I",
        Code::KeyJ => "J",
        Code::KeyK => "K",
        Code::KeyL => "L",
        Code::KeyM => "M",
        Code::KeyN => "N",
        Code::KeyO => "O",
        Code::KeyP => "P",
        Code::KeyQ => "Q",
        Code::KeyR => "R",
        Code::KeyS => "S",
        Code::KeyT => "T",
        Code::KeyU => "U",
        Code::KeyV => "V",
        Code::KeyW => "W",
        Code::KeyX => "X",
        Code::KeyY => "Y",
        Code::KeyZ => "Z",
        Code::Digit0 => "0",
        Code::Digit1 => "1",
        Code::Digit2 => "2",
        Code::Digit3 => "3",
        Code::Digit4 => "4",
        Code::Digit5 => "5",
        Code::Digit6 => "6",
        Code::Digit7 => "7",
        Code::Digit8 => "8",
        Code::Digit9 => "9",
        Code::F1 => "F1",
        Code::F2 => "F2",
        Code::F3 => "F3",
        Code::F4 => "F4",
        Code::F5 => "F5",
        Code::F6 => "F6",
        Code::F7 => "F7",
        Code::F8 => "F8",
        Code::F9 => "F9",
        Code::F10 => "F10",
        Code::F11 => "F11",
        Code::F12 => "F12",
        Code::Space => "Space",
        Code::Enter => "Enter",
        Code::Escape => "Esc",
        Code::Tab => "Tab",
        _ => "?",
    };

    parts.push(key_name);
    parts.join("+")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shortcut() {
        let (mods, key) = parse_shortcut("Ctrl+Shift+U").unwrap();
        assert!(mods.contains(Modifiers::CONTROL));
        assert!(mods.contains(Modifiers::SHIFT));
        assert_eq!(key, Code::KeyU);

        let (mods, key) = parse_shortcut("Alt+F1").unwrap();
        assert!(mods.contains(Modifiers::ALT));
        assert_eq!(key, Code::F1);
    }

    #[test]
    fn test_format_shortcut() {
        let s = format_shortcut(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyU);
        assert_eq!(s, "Ctrl+Shift+U");
    }
}
