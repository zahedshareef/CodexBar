export type SurfaceMode = "hidden" | "trayPanel" | "popOut" | "settings";
export type VisibleSurfaceMode = Exclude<SurfaceMode, "hidden">;
export type SettingsTabId =
  | "general"
  | "providers"
  | "display"
  | "apiKeys"
  | "cookies"
  | "tokenAccounts"
  | "advanced"
  | "about";
export type ProofProviderId =
  | "codex"
  | "claude"
  | "cursor"
  | "factory"
  | "gemini"
  | "antigravity"
  | "copilot"
  | "zai"
  | "minimax"
  | "kiro"
  | "vertexai"
  | "augment"
  | "opencode"
  | "kimi"
  | "kimik2"
  | "amp"
  | "warp"
  | "ollama"
  | "openrouter"
  | "synthetic"
  | "jetbrains"
  | "alibaba"
  | "nanogpt"
  | "infini";

export type TrayPanelSurfaceTarget = { kind: "summary" };
export type PopOutSurfaceTarget =
  | { kind: "dashboard" }
  | { kind: "provider"; providerId: string };
export type SettingsSurfaceTarget = { kind: "settings"; tab: SettingsTabId };

export type SurfaceTarget =
  | TrayPanelSurfaceTarget
  | PopOutSurfaceTarget
  | SettingsSurfaceTarget;

export type SurfaceTargetForMode<M extends VisibleSurfaceMode> =
  M extends "trayPanel"
    ? TrayPanelSurfaceTarget
    : M extends "popOut"
      ? PopOutSurfaceTarget
      : SettingsSurfaceTarget;

export interface CurrentSurfaceState {
  mode: SurfaceMode;
  target: SurfaceTarget;
}

export interface ProofRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface ProofStatePayload {
  mode: SurfaceMode;
  target: SurfaceTarget;
  windowRect: ProofRect | null;
  trayAnchor: ProofRect | null;
  workArea: ProofRect | null;
  menuPath: string | null;
  menuItems: string[];
}

export type ProofCommand =
  | "open-tray-panel"
  | "open-native-menu"
  | "open-dashboard"
  | "open-about-path"
  | "hide-surface"
  | `open-provider:${ProofProviderId}`
  | `open-settings:${SettingsTabId}`;

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
  startMinimized: boolean;
  showNotifications: boolean;
  soundEnabled: boolean;
  soundVolume: number;
  highUsageThreshold: number;
  criticalUsageThreshold: number;
  trayIconMode: string;
  switcherShowsIcons: boolean;
  menuBarShowsHighestUsage: boolean;
  menuBarShowsPercent: boolean;
  showAsUsed: boolean;
  showCreditsExtraUsage: boolean;
  showAllTokenAccountsInMenu: boolean;
  surpriseAnimations: boolean;
  enableAnimations: boolean;
  resetTimeRelative: boolean;
  menuBarDisplayMode: string;
  hidePersonalInfo: boolean;
  updateChannel: string;
  autoDownloadUpdates: boolean;
  installUpdatesOnQuit: boolean;
  globalShortcut: string;
  uiLanguage: string;
  claudeAvoidKeychainPrompts: boolean;
  disableKeychainAccess: boolean;
  showDebugSettings: boolean;
  providerMetrics: Record<string, string>;
}

/** Partial settings object — only include fields you want to change. */
export interface SettingsUpdate {
  enabledProviders?: string[];
  refreshIntervalSecs?: number;
  startAtLogin?: boolean;
  startMinimized?: boolean;
  showNotifications?: boolean;
  soundEnabled?: boolean;
  soundVolume?: number;
  highUsageThreshold?: number;
  criticalUsageThreshold?: number;
  trayIconMode?: string;
  switcherShowsIcons?: boolean;
  menuBarShowsHighestUsage?: boolean;
  menuBarShowsPercent?: boolean;
  showAsUsed?: boolean;
  showCreditsExtraUsage?: boolean;
  showAllTokenAccountsInMenu?: boolean;
  surpriseAnimations?: boolean;
  enableAnimations?: boolean;
  resetTimeRelative?: boolean;
  menuBarDisplayMode?: string;
  hidePersonalInfo?: boolean;
  updateChannel?: string;
  autoDownloadUpdates?: boolean;
  installUpdatesOnQuit?: boolean;
  globalShortcut?: string;
  uiLanguage?: string;
  claudeAvoidKeychainPrompts?: boolean;
  disableKeychainAccess?: boolean;
  showDebugSettings?: boolean;
  /** Map of provider CLI name → metric preference label. */
  providerMetrics?: Record<string, string>;
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

export interface PaceSnapshot {
  stage: "on_track" | "slightly_ahead" | "ahead" | "far_ahead" | "slightly_behind" | "behind" | "far_behind";
  deltaPercent: number;
  willLastToReset: boolean;
  etaSeconds: number | null;
  expectedUsedPercent: number;
  actualUsedPercent: number;
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
  pace: PaceSnapshot | null;
  accountOrganization: string | null;
  trayStatusLabel: string | null;
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

export interface DetectedBrowserBridge {
  browserType: string;
  displayName: string;
  profileCount: number;
}

export interface AppInfoBridge {
  name: string;
  version: string;
  buildNumber: string;
  updateChannel: string;
  tagline: string;
}

// ── Chart data types ─────────────────────────────────────────────────

export interface DailyCostPoint {
  date: string;
  value: number;
}

export interface ServiceUsagePoint {
  service: string;
  creditsUsed: number;
}

export interface DailyUsageBreakdown {
  day: string;
  services: ServiceUsagePoint[];
  totalCreditsUsed: number;
}

export interface ProviderChartData {
  providerId: string;
  costHistory: DailyCostPoint[];
  creditsHistory: DailyCostPoint[];
  usageBreakdown: DailyUsageBreakdown[];
}

// ── Token account types ──────────────────────────────────────────────

export interface TokenAccountSupportBridge {
  providerId: string;
  displayName: string;
  title: string;
  subtitle: string;
  placeholder: string;
}

export interface TokenAccountBridge {
  id: string;
  label: string;
  addedAt: string;
  lastUsed: string | null;
  isActive: boolean;
}

export interface ProviderTokenAccountsBridge {
  providerId: string;
  support: TokenAccountSupportBridge;
  accounts: TokenAccountBridge[];
  activeIndex: number;
}
