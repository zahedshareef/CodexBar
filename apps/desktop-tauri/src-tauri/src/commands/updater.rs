//! Updater lifecycle commands: check, download, apply, dismiss, and release-page linking.
//!
//! State transitions are mirrored through [`events::emit_update_state_changed`]
//! so the frontend can react without polling.

use std::sync::Mutex;

use tauri::Manager;

use super::open_url_in_browser;
use crate::events;
use crate::state::{AppState, UpdateState, UpdateStatePayload};

#[tauri::command]
pub fn get_update_state(state: tauri::State<'_, Mutex<AppState>>) -> UpdateStatePayload {
    state
        .lock()
        .map(|guard| guard.update_payload())
        .unwrap_or_else(|_| UpdateState::default().to_payload())
}

#[tauri::command]
pub async fn check_for_updates(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<UpdateStatePayload, String> {
    // Guard: skip if already checking or downloading.
    {
        let mut guard = state.lock().map_err(|e| e.to_string())?;
        match guard.update_state {
            UpdateState::Checking | UpdateState::Downloading(_) => {
                return Ok(guard.update_payload());
            }
            _ => {}
        }
        guard.update_state = UpdateState::Checking;
        guard.update_info = None;
        guard.installer_path = None;
    }

    let checking_payload = {
        let guard = state.lock().map_err(|e| e.to_string())?;
        guard.update_payload()
    };
    events::emit_update_state_changed(&app, &checking_payload);

    let settings = codexbar::settings::Settings::load();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        codexbar::updater::check_for_updates_with_channel(settings.update_channel),
    )
    .await;

    let (new_state, new_info) = match result {
        Ok(Some(info)) => (UpdateState::Available(info.version.clone()), Some(info)),
        Ok(None) => (UpdateState::Idle, None),
        Err(_) => (
            UpdateState::Error("Update check timed out".to_string()),
            None,
        ),
    };

    let payload = {
        let mut guard = state.lock().map_err(|e| e.to_string())?;
        guard.update_state = new_state;
        guard.update_info = new_info;
        guard.last_update_check_ms = Some(chrono::Utc::now().timestamp_millis());
        guard.update_payload()
    };
    events::emit_update_state_changed(&app, &payload);

    Ok(payload)
}

#[tauri::command]
pub async fn download_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<UpdateStatePayload, String> {
    let info = {
        let guard = state.lock().map_err(|e| e.to_string())?;
        match &guard.update_state {
            UpdateState::Available(_) | UpdateState::Error(_) => {}
            UpdateState::Downloading(_) => return Ok(guard.update_payload()),
            _ => return Err("No update available to download".to_string()),
        }
        guard
            .update_info
            .clone()
            .ok_or("No update information available")?
    };

    if !info.supports_auto_download() {
        return Err(
            "This update does not support automatic download. Open the release page instead."
                .to_string(),
        );
    }

    let initial_payload = {
        let mut guard = state.lock().map_err(|e| e.to_string())?;
        guard.update_state = UpdateState::Downloading(0.0);
        guard.update_payload()
    };
    events::emit_update_state_changed(&app, &initial_payload);

    let app_handle = app.clone();
    tokio::spawn(async move {
        let (tx, mut rx) = tokio::sync::watch::channel(codexbar::updater::UpdateState::Available);

        let info_for_download = info.clone();
        let download_handle = tokio::spawn(async move {
            codexbar::updater::download_update(&info_for_download, tx).await
        });

        let app_for_progress = app_handle.clone();
        let progress_handle = tokio::spawn(async move {
            while rx.changed().await.is_ok() {
                let backend_state = rx.borrow().clone();
                if let codexbar::updater::UpdateState::Downloading(progress) = backend_state {
                    let st = app_for_progress.state::<Mutex<AppState>>();
                    let payload = {
                        let mut guard = st.lock().unwrap();
                        guard.update_state = UpdateState::Downloading(progress);
                        guard.update_payload()
                    };
                    events::emit_update_state_changed(&app_for_progress, &payload);
                }
            }
        });

        let final_payload = match download_handle.await {
            Ok(Ok(path)) => {
                let st = app_handle.state::<Mutex<AppState>>();
                let mut guard = st.lock().unwrap();
                guard.update_state = UpdateState::Ready;
                guard.installer_path = Some(path);
                guard.update_payload()
            }
            Ok(Err(e)) => {
                let st = app_handle.state::<Mutex<AppState>>();
                let mut guard = st.lock().unwrap();
                guard.update_state = UpdateState::Error(e);
                guard.update_payload()
            }
            Err(join_err) => {
                let st = app_handle.state::<Mutex<AppState>>();
                let mut guard = st.lock().unwrap();
                guard.update_state =
                    UpdateState::Error(format!("Download task failed: {join_err}"));
                guard.update_payload()
            }
        };
        events::emit_update_state_changed(&app_handle, &final_payload);
        progress_handle.abort();
    });

    Ok(initial_payload)
}

#[tauri::command]
pub fn apply_update(state: tauri::State<'_, Mutex<AppState>>) -> Result<(), String> {
    let (path, expected_sha256) = {
        let guard = state.lock().map_err(|e| e.to_string())?;
        let path = guard
            .installer_path
            .clone()
            .ok_or("No downloaded update available to apply")?;
        let expected_sha256 = guard
            .update_info
            .as_ref()
            .and_then(|info| info.expected_sha256.clone())
            .ok_or("Missing SHA256 digest for downloaded update")?;
        (path, expected_sha256)
    };
    codexbar::updater::verify_installer_hash(&path, &expected_sha256)?;
    codexbar::updater::apply_update(&path)
}

#[tauri::command]
pub fn dismiss_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<UpdateStatePayload, String> {
    let payload = {
        let mut guard = state.lock().map_err(|e| e.to_string())?;
        guard.update_state = UpdateState::Idle;
        guard.update_info = None;
        guard.installer_path = None;
        guard.update_payload()
    };
    events::emit_update_state_changed(&app, &payload);
    Ok(payload)
}

#[tauri::command]
pub fn open_release_page(state: tauri::State<'_, Mutex<AppState>>) -> Result<(), String> {
    let url = {
        let guard = state.lock().map_err(|e| e.to_string())?;
        guard
            .update_info
            .as_ref()
            .map(|info| info.release_url.clone())
            .ok_or("No update information available")?
    };
    open_url_in_browser(&url)
}
