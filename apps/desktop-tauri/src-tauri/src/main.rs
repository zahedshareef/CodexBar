mod commands;
mod events;
mod proof_harness;
mod shell;
mod shortcut_bridge;
mod state;
mod surface;
mod surface_target;
mod tray_bridge;
mod window_positioner;

use std::sync::Mutex;

use state::AppState;
use surface::SurfaceMode;
use surface_target::SurfaceTarget;
use tauri::Manager;

fn should_hide_close_request(mode: SurfaceMode) -> bool {
    matches!(
        mode,
        SurfaceMode::TrayPanel | SurfaceMode::PopOut | SurfaceMode::Settings
    )
}

fn main() {
    let proof_config = proof_harness::ProofConfig::from_env();
    let is_proof_mode = proof_config.is_some();

    let mut initial_state = AppState::new();
    initial_state.proof_config = proof_config;

    tauri::Builder::default()
        .manage(Mutex::new(initial_state))
        .plugin(shortcut_bridge::plugin())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let position = shell::shortcut_panel_position(app);
            let _ = shell::reopen_to_target(
                app,
                SurfaceMode::TrayPanel,
                SurfaceTarget::Summary,
                position,
            );
        }))
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap_state,
            commands::get_provider_catalog,
            commands::get_settings_snapshot,
            commands::update_settings,
            commands::set_surface_mode,
            commands::get_current_surface_mode,
            commands::get_current_surface_state,
            commands::refresh_providers,
            commands::get_cached_providers,
            commands::get_update_state,
            commands::check_for_updates,
            commands::download_update,
            commands::apply_update,
            commands::dismiss_update,
            commands::open_release_page,
            commands::get_api_keys,
            commands::get_api_key_providers,
            commands::set_api_key,
            commands::remove_api_key,
            commands::get_manual_cookies,
            commands::set_manual_cookie,
            commands::remove_manual_cookie,
            commands::get_app_info,
            commands::get_proof_config,
        ])
        .setup(move |app| {
            if let Some(window) = app.get_webview_window("main") {
                window.hide()?;
            }
            tray_bridge::setup(app)?;
            shortcut_bridge::register(app.handle());

            // In proof mode, immediately show the target surface.
            if is_proof_mode {
                proof_harness::activate(app.handle());
            }

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::Focused(false) => {
                // Suppress blur-dismiss in proof mode so the window stays
                // visible for automated screenshot capture.
                if proof_harness::is_proof_mode(window.app_handle()) {
                    return;
                }
                // Blur in TrayPanel mode → auto-hide.
                let Some(st) = window.try_state::<Mutex<AppState>>() else {
                    return;
                };
                if st.lock().unwrap().surface_machine.current() == SurfaceMode::TrayPanel {
                    let _ = shell::hide_to_tray(window.app_handle());
                }
            }
            tauri::WindowEvent::CloseRequested { api, .. } => {
                // Close visible shell surfaces → hide instead of quitting.
                let Some(st) = window.try_state::<Mutex<AppState>>() else {
                    return;
                };
                let guard = st.lock().unwrap();
                let cur = guard.surface_machine.current();
                if should_hide_close_request(cur) {
                    api.prevent_close();
                    drop(guard);
                    let _ = shell::hide_to_tray(window.app_handle());
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("failed to run CodexBar desktop shell");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_request_hides_tray_first_surfaces() {
        assert!(should_hide_close_request(SurfaceMode::TrayPanel));
        assert!(should_hide_close_request(SurfaceMode::PopOut));
        assert!(should_hide_close_request(SurfaceMode::Settings));
    }

    #[test]
    fn close_request_leaves_hidden_surface_alone() {
        assert!(!should_hide_close_request(SurfaceMode::Hidden));
    }
}
