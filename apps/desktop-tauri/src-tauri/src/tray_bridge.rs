//! System tray icon setup: left-click toggle, right-click native menu.

use std::sync::Mutex;

use crate::commands::ProviderCatalogEntry;
use tauri::image::Image;
use tauri::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::shell;
use crate::state::{AppState, TrayAnchor};
use crate::surface::SurfaceMode;
use crate::surface_target::SurfaceTarget;

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrayMenuEntry {
    id: Option<String>,
    label: String,
    children: Vec<Self>,
    is_separator: bool,
}

impl TrayMenuEntry {
    fn item(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            label: label.into(),
            children: Vec::new(),
            is_separator: false,
        }
    }

    fn submenu(label: impl Into<String>, children: Vec<Self>) -> Self {
        Self {
            id: None,
            label: label.into(),
            children,
            is_separator: false,
        }
    }

    fn separator() -> Self {
        Self {
            id: None,
            label: String::new(),
            children: Vec::new(),
            is_separator: true,
        }
    }
}

/// Build the native context menu shown on tray right-click.
fn build_tray_menu(providers: &[ProviderCatalogEntry]) -> Vec<TrayMenuEntry> {
    let mut menu = vec![
        TrayMenuEntry::item("show_panel", "Show Panel"),
        TrayMenuEntry::item("pop_out", "Pop Out Dashboard"),
        TrayMenuEntry::item("settings", "Settings"),
        TrayMenuEntry::item("about", "About"),
        TrayMenuEntry::separator(),
    ];
    if !providers.is_empty() {
        menu.push(TrayMenuEntry::submenu(
            "Providers",
            providers
                .iter()
                .map(|provider| {
                    TrayMenuEntry::item(
                        format!("provider:{}", provider.id),
                        format!("Open {}", provider.display_name),
                    )
                })
                .collect(),
        ));
        menu.push(TrayMenuEntry::separator());
    }
    menu.extend([
        TrayMenuEntry::item("refresh", "Refresh All"),
        TrayMenuEntry::separator(),
        TrayMenuEntry::item("quit", "Quit CodexBar"),
    ]);
    menu
}

fn build_native_tray_menu(
    app: &AppHandle,
    providers: &[ProviderCatalogEntry],
) -> tauri::Result<Menu<tauri::Wry>> {
    let spec = build_tray_menu(providers);
    let entries = spec
        .iter()
        .map(|entry| build_native_menu_entry(app, entry))
        .collect::<tauri::Result<Vec<_>>>()?;
    let item_refs = entries
        .iter()
        .map(NativeMenuEntry::as_item)
        .collect::<Vec<_>>();

    Menu::with_items(app, &item_refs)
}

fn resolve_menu_target(id: &str) -> Option<shell::ShellTransitionRequest> {
    match id {
        "show_panel" => Some(shell::ShellTransitionRequest {
            mode: SurfaceMode::TrayPanel,
            target: SurfaceTarget::Summary,
            position: None,
        }),
        "pop_out" => Some(shell::ShellTransitionRequest {
            mode: SurfaceMode::PopOut,
            target: SurfaceTarget::Dashboard,
            position: None,
        }),
        "settings" => Some(shell::ShellTransitionRequest {
            mode: SurfaceMode::Settings,
            target: SurfaceTarget::Settings {
                tab: "general".into(),
            },
            position: None,
        }),
        "about" => Some(shell::ShellTransitionRequest {
            mode: SurfaceMode::Settings,
            target: SurfaceTarget::Settings {
                tab: "about".into(),
            },
            position: None,
        }),
        _ if id.starts_with("provider:") => Some(shell::ShellTransitionRequest {
            mode: SurfaceMode::PopOut,
            target: SurfaceTarget::parse(id)?,
            position: None,
        }),
        _ => None,
    }
}

enum MenuAction {
    Transition(shell::ShellTransitionRequest),
    Refresh,
    Quit,
}

fn resolve_menu_action(id: &str) -> Option<MenuAction> {
    match id {
        "refresh" => Some(MenuAction::Refresh),
        "quit" => Some(MenuAction::Quit),
        _ => resolve_menu_target(id).map(MenuAction::Transition),
    }
}

/// Store the tray icon bounds from a click event into shared state.
fn store_anchor(app: &AppHandle, rect: &tauri::Rect) {
    let (x, y) = match rect.position {
        tauri::Position::Physical(p) => (p.x, p.y),
        tauri::Position::Logical(l) => (l.x as i32, l.y as i32),
    };
    let (width, height) = match rect.size {
        tauri::Size::Physical(s) => (s.width, s.height),
        tauri::Size::Logical(l) => (l.width as u32, l.height as u32),
    };

    if let Some(st) = app.try_state::<Mutex<AppState>>() {
        let mut guard = st.lock().unwrap();
        guard.tray_anchor = Some(TrayAnchor {
            x,
            y,
            width,
            height,
        });
    }
}

/// Initialise the system tray icon, context menu, and event handlers.
///
/// - **Left-click** toggles the custom tray panel via the surface state machine.
/// - **Right-click** opens the native context menu with shell actions.
pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let menu = build_native_tray_menu(app.handle(), &crate::commands::get_provider_catalog())?;

    // Embed the icon at compile time so it works regardless of working directory.
    let icon_bytes = include_bytes!("../../../../rust/icons/icon.png");
    let icon = Image::from_bytes(icon_bytes)?;

    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .tooltip("CodexBar Desktop")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                let app = tray.app_handle();
                store_anchor(app, &rect);
                let position = shell::tray_panel_position(app);
                shell::toggle_tray_panel(app, position);
            }
        })
        .on_menu_event(|app, event| {
            handle_menu_event(app, event.id().as_ref());
        })
        .build(app)?;

    Ok(())
}

/// Route a native menu-item click to the corresponding shell action.
fn handle_menu_event(app: &AppHandle, id: &str) {
    match resolve_menu_action(id) {
        Some(MenuAction::Transition(mut request)) => {
            if id == "show_panel" {
                request.position =
                    shell::tray_panel_position(app).or_else(|| shell::shortcut_panel_position(app));
                let _ = shell::reopen_to_target(app, request.mode, request.target, request.position);
            } else {
                let _ =
                    shell::transition_to_target(app, request.mode, request.target, request.position);
            }
        }
        Some(MenuAction::Refresh) => {
            let handle = app.clone();
            tauri::async_runtime::spawn(async move {
                let _ = crate::commands::do_refresh_providers(&handle).await;
            });
        }
        Some(MenuAction::Quit) => {
            app.exit(0);
        }
        None => {}
    }
}

#[allow(dead_code)]
fn menu_contains(menu: &[TrayMenuEntry], id: &str) -> bool {
    menu.iter().any(|entry| {
        entry.id.as_deref() == Some(id)
            || (!entry.children.is_empty() && menu_contains(&entry.children, id))
    })
}

enum NativeMenuEntry {
    Item(MenuItem<tauri::Wry>),
    Submenu(Submenu<tauri::Wry>),
    Separator(PredefinedMenuItem<tauri::Wry>),
}

impl NativeMenuEntry {
    fn as_item(&self) -> &dyn IsMenuItem<tauri::Wry> {
        match self {
            Self::Item(item) => item,
            Self::Submenu(item) => item,
            Self::Separator(item) => item,
        }
    }
}

fn build_native_menu_entry(
    app: &AppHandle,
    entry: &TrayMenuEntry,
) -> tauri::Result<NativeMenuEntry> {
    if entry.is_separator {
        return Ok(NativeMenuEntry::Separator(PredefinedMenuItem::separator(
            app,
        )?));
    }

    if !entry.children.is_empty() {
        let children = entry
            .children
            .iter()
            .map(|child| build_native_menu_entry(app, child))
            .collect::<tauri::Result<Vec<_>>>()?;
        let child_refs = children
            .iter()
            .map(NativeMenuEntry::as_item)
            .collect::<Vec<_>>();

        return Ok(NativeMenuEntry::Submenu(Submenu::with_items(
            app,
            &entry.label,
            true,
            &child_refs,
        )?));
    }

    Ok(NativeMenuEntry::Item(MenuItem::with_id(
        app,
        entry.id.clone().unwrap_or_default(),
        &entry.label,
        true,
        None::<&str>,
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_provider_catalog() -> Vec<ProviderCatalogEntry> {
        vec![
            ProviderCatalogEntry {
                id: "codex".into(),
                display_name: "Codex".into(),
                cookie_domain: None,
            },
            ProviderCatalogEntry {
                id: "claude".into(),
                display_name: "Claude".into(),
                cookie_domain: None,
            },
        ]
    }

    #[test]
    fn tray_menu_includes_about_and_provider_entries() {
        let menu = build_tray_menu(&sample_provider_catalog());
        assert!(menu_contains(&menu, "about"));
        assert!(menu_contains(&menu, "provider:codex"));
        assert!(menu_contains(&menu, "quit"));
    }

    #[test]
    fn settings_menu_routes_to_settings_about_target() {
        let action = resolve_menu_target("about").expect("about target");
        assert_eq!(action.mode, SurfaceMode::Settings);
        assert_eq!(
            action.target,
            SurfaceTarget::Settings {
                tab: "about".into()
            }
        );
    }

    #[test]
    fn provider_menu_routes_to_provider_popout_target() {
        let action = resolve_menu_target("provider:codex").expect("provider target");
        assert_eq!(action.mode, SurfaceMode::PopOut);
        assert_eq!(
            action.target,
            SurfaceTarget::Provider {
                provider_id: "codex".into()
            }
        );
    }
}
