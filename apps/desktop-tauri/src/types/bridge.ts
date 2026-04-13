export type SurfaceMode = "hidden" | "trayPanel" | "popOut" | "settings";

export interface SurfaceModeDescriptor {
  id: string;
  label: string;
  description: string;
}

export interface BridgeCommandDescriptor {
  id: string;
  description: string;
}

export interface BridgeEventDescriptor {
  id: string;
  description: string;
}

export interface ProviderCatalogEntry {
  id: string;
  displayName: string;
  cookieDomain: string | null;
}

export interface SettingsSnapshot {
  enabledProviders: string[];
  refreshIntervalSecs: number;
  startAtLogin: boolean;
  showNotifications: boolean;
  trayIconMode: string;
  showAsUsed: boolean;
  surpriseAnimations: boolean;
  enableAnimations: boolean;
  resetTimeRelative: boolean;
  menuBarDisplayMode: string;
  hidePersonalInfo: boolean;
  updateChannel: string;
  globalShortcut: string;
  uiLanguage: string;
}

/** Partial settings object — only include fields you want to change. */
export interface SettingsUpdate {
  enabledProviders?: string[];
  refreshIntervalSecs?: number;
  startAtLogin?: boolean;
  showNotifications?: boolean;
  trayIconMode?: string;
  showAsUsed?: boolean;
  surpriseAnimations?: boolean;
  enableAnimations?: boolean;
  resetTimeRelative?: boolean;
  menuBarDisplayMode?: string;
  hidePersonalInfo?: boolean;
  updateChannel?: string;
  globalShortcut?: string;
  uiLanguage?: string;
}

export interface BootstrapState {
  contractVersion: string;
  surfaceModes: SurfaceModeDescriptor[];
  commands: BridgeCommandDescriptor[];
  events: BridgeEventDescriptor[];
  providers: ProviderCatalogEntry[];
  settings: SettingsSnapshot;
}

// ── Provider usage snapshot types ────────────────────────────────────

export interface RateWindowSnapshot {
  usedPercent: number;
  remainingPercent: number;
  windowMinutes: number | null;
  resetsAt: string | null;
  resetDescription: string | null;
  isExhausted: boolean;
}

export interface CostSnapshotBridge {
  used: number;
  limit: number | null;
  remaining: number | null;
  currencyCode: string;
  period: string;
  resetsAt: string | null;
  formattedUsed: string;
  formattedLimit: string | null;
}

export interface ProviderUsageSnapshot {
  providerId: string;
  displayName: string;
  primary: RateWindowSnapshot;
  secondary: RateWindowSnapshot | null;
  modelSpecific: RateWindowSnapshot | null;
  tertiary: RateWindowSnapshot | null;
  cost: CostSnapshotBridge | null;
  planName: string | null;
  accountEmail: string | null;
  sourceLabel: string;
  updatedAt: string;
  error: string | null;
}

export interface RefreshCompletePayload {
  providerCount: number;
  errorCount: number;
}

// ── Update state types ───────────────────────────────────────────────

export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "ready"
  | "error";

export interface UpdateStatePayload {
  status: UpdateStatus;
  version: string | null;
  error: string | null;
  progress: number | null;
  releaseUrl: string | null;
  canDownload: boolean;
  canApply: boolean;
}

// ── Credential store types ───────────────────────────────────────────

export interface ApiKeyInfoBridge {
  providerId: string;
  provider: string;
  maskedKey: string;
  savedAt: string;
  label: string | null;
}

export interface ApiKeyProviderInfoBridge {
  id: string;
  displayName: string;
  envVar: string | null;
  help: string | null;
  dashboardUrl: string | null;
}

export interface CookieInfoBridge {
  providerId: string;
  provider: string;
  savedAt: string;
}

export interface AppInfoBridge {
  name: string;
  version: string;
  buildNumber: string;
  updateChannel: string;
  tagline: string;
}

// ── Proof harness types ──────────────────────────────────────────────

export interface ProofConfig {
  targetSurface: SurfaceMode;
  settingsTab: string | null;
}
