use std::collections::HashMap;

use codexbar::core::ProviderId;
use codexbar::settings::{ApiKeys, ManualCookies, Settings};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeDiagnostics {
    pub app_version: String,
    pub platform: String,
    pub enabled_providers: Vec<String>,
    pub provider_cookie_sources: HashMap<String, String>,
    pub has_manual_cookies: Vec<String>,
    pub has_api_keys: Vec<String>,
    pub hide_personal_info: bool,
    pub refresh_interval_secs: u64,
}

fn safe_diagnostics_from(
    settings: Settings,
    cookies: ManualCookies,
    api_keys: ApiKeys,
) -> SafeDiagnostics {
    let mut enabled_providers = settings
        .get_enabled_provider_ids()
        .into_iter()
        .map(|id| id.cli_name().to_string())
        .collect::<Vec<_>>();
    enabled_providers.sort();

    let provider_cookie_sources = ProviderId::all()
        .iter()
        .map(|id| {
            (
                id.cli_name().to_string(),
                settings.cookie_source(*id).to_string(),
            )
        })
        .collect::<HashMap<_, _>>();

    let mut has_manual_cookies = cookies
        .get_all_for_display()
        .into_iter()
        .map(|entry| entry.provider_id)
        .collect::<Vec<_>>();
    has_manual_cookies.sort();

    let mut has_api_keys = api_keys
        .get_all_for_display()
        .into_iter()
        .map(|entry| entry.provider_id)
        .collect::<Vec<_>>();
    has_api_keys.sort();

    SafeDiagnostics {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
        enabled_providers,
        provider_cookie_sources,
        has_manual_cookies,
        has_api_keys,
        hide_personal_info: settings.hide_personal_info,
        refresh_interval_secs: settings.refresh_interval_secs,
    }
}

#[tauri::command]
pub fn get_safe_diagnostics() -> SafeDiagnostics {
    safe_diagnostics_from(Settings::load(), ManualCookies::load(), ApiKeys::load())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_diagnostics_contains_no_secret_values() {
        let settings = Settings::default();
        let mut cookies = ManualCookies::default();
        cookies.set("codex", "session=secret-cookie-value");
        let mut keys = ApiKeys::default();
        keys.set("openrouter", "sk-secret-api-key", None);

        let payload = safe_diagnostics_from(settings, cookies, keys);
        let json = serde_json::to_string(&payload).expect("serialize diagnostics");

        assert!(json.contains("codex"));
        assert!(json.contains("openrouter"));
        assert!(!json.contains("secret-cookie-value"));
        assert!(!json.contains("sk-secret-api-key"));
        assert!(!json.contains("session="));
    }
}
