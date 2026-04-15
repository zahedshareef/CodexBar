import { invoke } from "@tauri-apps/api/core";
import type {
  ApiKeyInfoBridge,
  ApiKeyProviderInfoBridge,
  AppInfoBridge,
  BootstrapState,
  CookieInfoBridge,
  ProofConfig,
  ProviderCatalogEntry,
  ProviderUsageSnapshot,
  SettingsSnapshot,
  SettingsUpdate,
  UpdateStatePayload,
} from "../types/bridge";

export function getBootstrapState(): Promise<BootstrapState> {
  return invoke<BootstrapState>("get_bootstrap_state");
}

export function getProviderCatalog(): Promise<ProviderCatalogEntry[]> {
  return invoke<ProviderCatalogEntry[]>("get_provider_catalog");
}

export function getSettingsSnapshot(): Promise<SettingsSnapshot> {
  return invoke<SettingsSnapshot>("get_settings_snapshot");
}

export function updateSettings(
  patch: SettingsUpdate,
): Promise<SettingsSnapshot> {
  return invoke<SettingsSnapshot>("update_settings", { patch });
}

export function setSurfaceMode(mode: string, target?: string): Promise<string> {
  return invoke<string>("set_surface_mode", { mode, target: target ?? null });
}

export function getCurrentSurfaceMode(): Promise<string> {
  return invoke<string>("get_current_surface_mode");
}

export function refreshProviders(): Promise<void> {
  return invoke<void>("refresh_providers");
}

export function getCachedProviders(): Promise<ProviderUsageSnapshot[]> {
  return invoke<ProviderUsageSnapshot[]>("get_cached_providers");
}

export function getUpdateState(): Promise<UpdateStatePayload> {
  return invoke<UpdateStatePayload>("get_update_state");
}

export function checkForUpdates(): Promise<UpdateStatePayload> {
  return invoke<UpdateStatePayload>("check_for_updates");
}

export function downloadUpdate(): Promise<UpdateStatePayload> {
  return invoke<UpdateStatePayload>("download_update");
}

export function applyUpdate(): Promise<void> {
  return invoke<void>("apply_update");
}

export function dismissUpdate(): Promise<UpdateStatePayload> {
  return invoke<UpdateStatePayload>("dismiss_update");
}

export function openReleasePage(): Promise<void> {
  return invoke<void>("open_release_page");
}

// ── Credential store bridge ──────────────────────────────────────────

export function getApiKeys(): Promise<ApiKeyInfoBridge[]> {
  return invoke<ApiKeyInfoBridge[]>("get_api_keys");
}

export function getApiKeyProviders(): Promise<ApiKeyProviderInfoBridge[]> {
  return invoke<ApiKeyProviderInfoBridge[]>("get_api_key_providers");
}

export function setApiKey(
  providerId: string,
  apiKey: string,
  label?: string,
): Promise<ApiKeyInfoBridge[]> {
  return invoke<ApiKeyInfoBridge[]>("set_api_key", {
    providerId,
    apiKey,
    label: label ?? null,
  });
}

export function removeApiKey(providerId: string): Promise<ApiKeyInfoBridge[]> {
  return invoke<ApiKeyInfoBridge[]>("remove_api_key", { providerId });
}

export function getManualCookies(): Promise<CookieInfoBridge[]> {
  return invoke<CookieInfoBridge[]>("get_manual_cookies");
}

export function setManualCookie(
  providerId: string,
  cookieHeader: string,
): Promise<CookieInfoBridge[]> {
  return invoke<CookieInfoBridge[]>("set_manual_cookie", {
    providerId,
    cookieHeader,
  });
}

export function removeManualCookie(
  providerId: string,
): Promise<CookieInfoBridge[]> {
  return invoke<CookieInfoBridge[]>("remove_manual_cookie", { providerId });
}

export function getAppInfo(): Promise<AppInfoBridge> {
  return invoke<AppInfoBridge>("get_app_info");
}

// ── Proof harness bridge ─────────────────────────────────────────────

export function getProofConfig(): Promise<ProofConfig | null> {
  return invoke<ProofConfig | null>("get_proof_config");
}
