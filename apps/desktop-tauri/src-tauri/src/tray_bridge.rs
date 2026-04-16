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

#[derive(Debug, Clone, Copy)]
struct MonitorScaleInfo {
    physical_x: i32,
    physical_y: i32,
    physical_width: u32,
    physical_height: u32,
    logical_x: f64,
    logical_y: f64,
    logical_width: f64,
    logical_height: f64,
    scale_factor: f64,
}

impl MonitorScaleInfo {
    fn from_monitor(monitor: &tauri::Monitor) -> Self {
        let scale_factor = monitor.scale_factor();
        let safe_scale = if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor
        } else {
            1.0
        };
        let position = monitor.position();
        let size = monitor.size();

        Self {
            physical_x: position.x,
            physical_y: position.y,
            physical_width: size.width,
            physical_height: size.height,
            logical_x: position.x as f64 / safe_scale,
            logical_y: position.y as f64 / safe_scale,
            logical_width: size.width as f64 / safe_scale,
            logical_height: size.height as f64 / safe_scale,
            scale_factor: safe_scale,
        }
    }
}

fn scale_factor_for_logical_rect(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    monitors: &[MonitorScaleInfo],
) -> Option<f64> {
    let center_x = x + width * 0.5;
    let center_y = y + height * 0.5;

    monitors
        .iter()
        .filter(|monitor| {
            center_x >= monitor.logical_x
                && center_x < monitor.logical_x + monitor.logical_width
                && center_y >= monitor.logical_y
                && center_y < monitor.logical_y + monitor.logical_height
        })
        .max_by(|left, right| left.scale_factor.total_cmp(&right.scale_factor))
        .map(|monitor| monitor.scale_factor)
}

fn scale_factor_for_physical_point(x: i32, y: i32, monitors: &[MonitorScaleInfo]) -> Option<f64> {
    monitors
        .iter()
        .find(|monitor| {
            x >= monitor.physical_x
                && x < monitor.physical_x + monitor.physical_width as i32
                && y >= monitor.physical_y
                && y < monitor.physical_y + monitor.physical_height as i32
        })
        .map(|monitor| monitor.scale_factor)
}

fn logical_to_physical_anchor(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    scale_factor: f64,
) -> TrayAnchor {
    let safe_scale = if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    };

    TrayAnchor {
        x: (x * safe_scale).round() as i32,
        y: (y * safe_scale).round() as i32,
        width: ((width * safe_scale).round().max(1.0)) as u32,
        height: ((height * safe_scale).round().max(1.0)) as u32,
    }
}

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
    let monitors = app
        .get_webview_window("main")
        .and_then(|window| window.available_monitors().ok())
        .unwrap_or_default()
        .into_iter()
        .map(|monitor| MonitorScaleInfo::from_monitor(&monitor))
        .collect::<Vec<_>>();

    let anchor = match (rect.position, rect.size) {
        (tauri::Position::Physical(position), tauri::Size::Physical(size)) => TrayAnchor {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
        },
        (tauri::Position::Logical(position), tauri::Size::Logical(size)) => {
            let scale = scale_factor_for_logical_rect(
                position.x,
                position.y,
                size.width,
                size.height,
                &monitors,
            )
            .unwrap_or(1.0);
            logical_to_physical_anchor(position.x, position.y, size.width, size.height, scale)
        }
        (tauri::Position::Physical(position), tauri::Size::Logical(size)) => {
            let scale =
                scale_factor_for_physical_point(position.x, position.y, &monitors).unwrap_or(1.0);
            TrayAnchor {
                x: position.x,
                y: position.y,
                width: ((size.width * scale).round().max(1.0)) as u32,
                height: ((size.height * scale).round().max(1.0)) as u32,
            }
        }
        (tauri::Position::Logical(position), tauri::Size::Physical(size)) => {
            let scale = scale_factor_for_logical_rect(
                position.x,
                position.y,
                size.width as f64,
                size.height as f64,
                &monitors,
            )
            .unwrap_or(1.0);
            TrayAnchor {
                x: (position.x * scale).round() as i32,
                y: (position.y * scale).round() as i32,
                width: size.width,
                height: size.height,
            }
        }
    };

    if let Some(st) = app.try_state::<Mutex<AppState>>() {
        let mut guard = st.lock().unwrap();
        guard.tray_anchor = Some(anchor);
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
                button,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                let app = tray.app_handle();
                store_anchor(app, &rect);
                if button == MouseButton::Left {
                    let position = shell::tray_panel_position(app);
                    shell::toggle_tray_panel(app, position);
                }
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
                let _ =
                    shell::reopen_to_target(app, request.mode, request.target, request.position);
            } else {
                let _ = shell::transition_to_target(
                    app,
                    request.mode,
                    request.target,
                    request.position,
                );
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

    #[test]
    fn logical_tray_anchor_uses_matching_monitor_scale() {
        let monitors = vec![
            MonitorScaleInfo {
                physical_x: 0,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
                logical_x: 0.0,
                logical_y: 0.0,
                logical_width: 1920.0,
                logical_height: 1080.0,
                scale_factor: 1.0,
            },
            MonitorScaleInfo {
                physical_x: 1920,
                physical_y: 0,
                physical_width: 2560,
                physical_height: 1440,
                logical_x: 960.0,
                logical_y: 0.0,
                logical_width: 1280.0,
                logical_height: 720.0,
                scale_factor: 2.0,
            },
        ];

        let scale = scale_factor_for_logical_rect(1500.0, 500.0, 12.0, 12.0, &monitors)
            .expect("matching monitor scale");
        let anchor = logical_to_physical_anchor(1500.0, 500.0, 12.0, 12.0, scale);

        assert_eq!(anchor.x, 3000);
        assert_eq!(anchor.y, 1000);
        assert_eq!(anchor.width, 24);
        assert_eq!(anchor.height, 24);
    }
}
