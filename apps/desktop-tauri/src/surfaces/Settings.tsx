import { useCallback, useEffect, useState } from "react";
import type {
  ApiKeyInfoBridge,
  ApiKeyProviderInfoBridge,
  AppInfoBridge,
  BootstrapState,
  CookieInfoBridge,
  DetectedBrowserBridge,
  Language,
  MenuBarDisplayMode,
  MetricPreference,
  ProviderCatalogEntry,
  TokenAccountSupportBridge,
  ProviderUsageSnapshot,
  RateWindowSnapshot,
  SettingsTabId,
  SettingsUpdate,
  TrayIconMode,
  UpdateChannel,
} from "../types/bridge";
import { useSettings } from "../hooks/useSettings";
import { useSurfaceTarget } from "../hooks/useSurfaceMode";
import { useUpdateState } from "../hooks/useUpdateState";
import { useLocale } from "../hooks/useLocale";
import { formatRelativeUpdated } from "../lib/relativeTime";
import type { LocaleKey } from "../i18n/keys";
import {
  ProvidersSidebar,
  type ProviderSidebarRow,
  type ProviderSidebarStatus,
} from "./settings/providers/ProvidersSidebar";
import { ProviderDetailPane } from "./settings/providers/ProviderDetailPane";
import { TokenAccountsPanel } from "./settings/tokens/TokenAccountsPanel";
import {
  getApiKeyProviders,
  getApiKeys,
  getAppInfo,
  getCachedProviders,
  getManualCookies,
  getTokenAccountProviders,
  importBrowserCookies,
  listDetectedBrowsers,
  playNotificationSound,
  registerGlobalShortcut,
  removeApiKey,
  removeManualCookie,
  reorderProviders,
  setApiKey,
  setManualCookie,
  setSurfaceMode,
  unregisterGlobalShortcut,
} from "../lib/tauri";
import { ShortcutCapture } from "../components/ShortcutCapture";

// ── tiny reusable controls ──────────────────────────────────────────

function Toggle({
  checked,
  onChange,
  disabled,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      className={`toggle ${checked ? "toggle--on" : ""}`}
      disabled={disabled}
      onClick={() => onChange(!checked)}
    />
  );
}

function Select({
  value,
  options,
  onChange,
  disabled,
}: {
  value: string;
  options: { value: string; label: string }[];
  onChange: (v: string) => void;
  disabled?: boolean;
}) {
  return (
    <select
      className="select"
      value={value}
      disabled={disabled}
      onChange={(e) => onChange(e.target.value)}
    >
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}

function NumberInput({
  value,
  min,
  max,
  step,
  onChange,
  disabled,
}: {
  value: number;
  min?: number;
  max?: number;
  step?: number;
  onChange: (v: number) => void;
  disabled?: boolean;
}) {
  return (
    <input
      type="number"
      className="number-input"
      value={value}
      min={min}
      max={max}
      step={step}
      disabled={disabled}
      onChange={(e) => {
        const n = Number(e.target.value);
        if (!Number.isNaN(n)) onChange(n);
      }}
    />
  );
}

function TextInput({
  value,
  placeholder,
  onChange,
  disabled,
}: {
  value: string;
  placeholder?: string;
  onChange: (v: string) => void;
  disabled?: boolean;
}) {
  return (
    <input
      type="text"
      className="text-input"
      value={value}
      placeholder={placeholder}
      disabled={disabled}
      onChange={(e) => onChange(e.target.value)}
    />
  );
}

// ── field row ────────────────────────────────────────────────────────

function Field({
  label,
  description,
  children,
}: {
  label: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="settings-field">
      <div className="settings-field__text">
        <span className="settings-field__label">{label}</span>
        {description && (
          <span className="settings-field__desc">{description}</span>
        )}
      </div>
      <div className="settings-field__control">{children}</div>
    </div>
  );
}

// ── tab types ────────────────────────────────────────────────────────

type SettingsTab = SettingsTabId;

const TAB_META: { id: SettingsTab; labelKey: LocaleKey; icon: string }[] = [
  { id: "general", labelKey: "TabGeneral", icon: "⚙" },
  { id: "providers", labelKey: "TabProviders", icon: "◉" },
  { id: "display", labelKey: "TabDisplay", icon: "◧" },
  { id: "apiKeys", labelKey: "TabApiKeys", icon: "🔑" },
  { id: "cookies", labelKey: "TabCookies", icon: "🍪" },
  { id: "tokenAccounts", labelKey: "TabTokenAccounts", icon: "🪙" },
  { id: "advanced", labelKey: "TabAdvanced", icon: "⌘" },
  { id: "about", labelKey: "TabAbout", icon: "ℹ" },
];

// ── main component ──────────────────────────────────────────────────

function isSettingsTab(value: string): value is SettingsTab {
  return TAB_META.some((t) => t.id === value);
}

export default function Settings({ state }: { state: BootstrapState }) {
  const { settings, saving, error, update } = useSettings(state.settings);
  const { t } = useLocale();
  const shellTarget = useSurfaceTarget("settings");
  const initialTab: SettingsTab =
    shellTarget?.kind === "settings" && isSettingsTab(shellTarget.tab)
      ? shellTarget.tab
      : "general";
  const [activeTab, setActiveTab] = useState<SettingsTab>(initialTab);

  useEffect(() => {
    if (shellTarget?.kind !== "settings" || !isSettingsTab(shellTarget.tab)) {
      return;
    }

    const nextTab: SettingsTab = shellTarget.tab;
    setActiveTab((current) =>
      current === nextTab ? current : nextTab,
    );
  }, [shellTarget]);

  const set = (patch: SettingsUpdate) => void update(patch);
  const handleTabClick = useCallback((tab: SettingsTab) => {
    setActiveTab(tab);
    void setSurfaceMode("settings", { kind: "settings", tab });
  }, []);

  return (
    <div className="settings">
      {/* tab bar */}
      <nav className="settings-tabs" role="tablist">
        {TAB_META.map((tab) => (
          <button
            key={tab.id}
            role="tab"
            aria-selected={activeTab === tab.id}
            className={`settings-tab ${activeTab === tab.id ? "settings-tab--active" : ""}`}
            onClick={() => handleTabClick(tab.id)}
          >
            <span className="settings-tab__icon">{tab.icon}</span>
            {t(tab.labelKey)}
          </button>
        ))}
      </nav>

      {/* status bar */}
      {(saving || error) && (
        <div className={`settings-status ${error ? "settings-status--error" : ""}`}>
          {saving ? t("SettingsStatusSaving") : error}
        </div>
      )}

      {/* tab panels */}
      <div className="settings-body">
        {activeTab === "general" && (
          <GeneralTab settings={settings} set={set} saving={saving} />
        )}
        {activeTab === "providers" && (
          <ProvidersTab
            settings={settings}
            providers={state.providers}
            set={set}
            saving={saving}
            onNavigate={handleTabClick}
          />
        )}
        {activeTab === "display" && (
          <DisplayTab settings={settings} set={set} saving={saving} />
        )}
        {activeTab === "advanced" && (
          <AdvancedTab settings={settings} set={set} saving={saving} />
        )}
        {activeTab === "apiKeys" && (
          <ApiKeysTab providers={state.providers} />
        )}
        {activeTab === "cookies" && (
          <CookiesTab providers={state.providers} />
        )}
        {activeTab === "tokenAccounts" && <TokenAccountsTab />}
        {activeTab === "about" && <AboutTab />}
      </div>
    </div>
  );
}

// ── General ──────────────────────────────────────────────────────────

interface TabProps {
  settings: BootstrapState["settings"];
  set: (p: SettingsUpdate) => void;
  saving: boolean;
}

function GeneralTab({ settings, set, saving }: TabProps) {
  const { t } = useLocale();
  const [playingSound, setPlayingSound] = useState(false);
  const [shortcutError, setShortcutError] = useState<string | null>(null);

  const handleTestSound = useCallback(() => {
    setPlayingSound(true);
    void playNotificationSound().catch(() => {});
    window.setTimeout(() => setPlayingSound(false), 1500);
  }, []);

  const commitShortcut = useCallback(
    async (accelerator: string) => {
      setShortcutError(null);
      try {
        // Best-effort capture registration (emits global-shortcut-triggered).
        // Persisting via update_settings re-registers with the default
        // window-toggle handler, which is what we ultimately want.
        await registerGlobalShortcut(accelerator).catch(() => {});
        set({ globalShortcut: accelerator });
      } catch (err: unknown) {
        setShortcutError(err instanceof Error ? err.message : String(err));
      }
    },
    [set],
  );

  const clearShortcut = useCallback(async () => {
    setShortcutError(null);
    try {
      await unregisterGlobalShortcut().catch(() => {});
      set({ globalShortcut: "" });
    } catch (err: unknown) {
      setShortcutError(err instanceof Error ? err.message : String(err));
    }
  }, [set]);

  return (
    <section className="settings-section">
      <h3 className="settings-section__title">{t("StartupSettings")}</h3>
      <Field label={t("StartAtLogin")} description={t("StartAtLoginHelper")}>
        <Toggle
          checked={settings.startAtLogin}
          disabled={saving}
          onChange={(v) => set({ startAtLogin: v })}
        />
      </Field>
      <Field label={t("StartMinimized")} description={t("StartMinimizedHelper")}>
        <Toggle
          checked={settings.startMinimized}
          disabled={saving}
          onChange={(v) => set({ startMinimized: v })}
        />
      </Field>

      {/* Refresh interval lives on the Advanced tab (Phase 8). */}
      {/* TODO(Phase 7): General tab may need re-layout after Refresh move */}

      <h3 className="settings-section__title">{t("SectionNotifications")}</h3>
      <Field
        label={t("ShowNotifications")}
        description={t("ShowNotificationsHelper")}
      >
        <Toggle
          checked={settings.showNotifications}
          disabled={saving}
          onChange={(v) => set({ showNotifications: v })}
        />
      </Field>
      <Field label={t("SoundEnabled")} description={t("SoundEnabledHelper")}>
        <div className="sound-enabled-row">
          <Toggle
            checked={settings.soundEnabled}
            disabled={saving}
            onChange={(v) => set({ soundEnabled: v })}
          />
          <button
            type="button"
            className="shortcut-capture__button shortcut-capture__button--ghost"
            disabled={saving || !settings.soundEnabled || playingSound}
            onClick={handleTestSound}
          >
            {playingSound
              ? t("NotificationTestSoundPlaying")
              : t("NotificationTestSound")}
          </button>
        </div>
      </Field>
      {settings.soundEnabled && (
        <Field label={t("SoundVolume")} description={t("SoundVolumeHelper")}>
          <NumberInput
            value={settings.soundVolume}
            min={0}
            max={100}
            step={5}
            disabled={saving}
            onChange={(v) => set({ soundVolume: v })}
          />
        </Field>
      )}

      <h3 className="settings-section__title">{t("SectionUsageThresholds")}</h3>
      <Field
        label={t("HighUsageAlert")}
        description={t("HighUsageWarningHelper")}
      >
        <NumberInput
          value={settings.highUsageThreshold}
          min={0}
          max={100}
          step={5}
          disabled={saving}
          onChange={(v) => set({ highUsageThreshold: v })}
        />
      </Field>
      <Field
        label={t("CriticalUsageAlert")}
        description={t("CriticalUsageWarningHelper")}
      >
        <NumberInput
          value={settings.criticalUsageThreshold}
          min={0}
          max={100}
          step={5}
          disabled={saving}
          onChange={(v) => set({ criticalUsageThreshold: v })}
        />
      </Field>

      <h3 className="settings-section__title">{t("SectionKeyboard")}</h3>
      <Field
        label={t("GlobalShortcutFieldLabel")}
        description={t("GlobalShortcutToggleHelper")}
      >
        <ShortcutCapture
          value={settings.globalShortcut}
          disabled={saving}
          onCommit={(accel) => void commitShortcut(accel)}
          onClear={() => void clearShortcut()}
        />
      </Field>
      {shortcutError && (
        <p className="settings-section__error">{shortcutError}</p>
      )}
      <p className="settings-section__hint">{t("ShortcutRecordingHint")}</p>
    </section>
  );
}

// ── Providers ────────────────────────────────────────────────────────

const METRIC_OPTIONS = [
  { value: "automatic", label: "Automatic" },
  { value: "session", label: "Session" },
  { value: "weekly", label: "Weekly" },
  { value: "model", label: "Model" },
  { value: "credits", label: "Credits" },
  { value: "average", label: "Average" },
];

const PROVIDER_ICON_MAP: Record<string, { icon: string; color: string }> = {
  codex:       { icon: "◆", color: "#49a3b0" },
  claude:      { icon: "◈", color: "#cc7c5e" },
  cursor:      { icon: "▸", color: "#00bfa5" },
  gemini:      { icon: "✦", color: "#ab87ea" },
  copilot:     { icon: "⬡", color: "#a855f7" },
  antigravity: { icon: "◉", color: "#60ba7e" },
  factory:     { icon: "◎", color: "#ff6b35" },
  droid:       { icon: "◎", color: "#ff6b35" },
  zai:         { icon: "Z", color: "#e85a6a" },
  "z.ai":      { icon: "Z", color: "#e85a6a" },
  kiro:        { icon: "K", color: "#ff9900" },
  vertexai:    { icon: "△", color: "#4285f4" },
  augment:     { icon: "A", color: "#6366f1" },
  minimax:     { icon: "M", color: "#fe603c" },
  opencode:    { icon: "○", color: "#3b82f6" },
  kimi:        { icon: "☽", color: "#fe603c" },
  kimik2:      { icon: "☽", color: "#4c00ff" },
  amp:         { icon: "⚡", color: "#dc2626" },
  jetbrains:   { icon: "J", color: "#ff3399" },
  alibaba:     { icon: "阿", color: "#ff6a00" },
  nanogpt:     { icon: "N", color: "#687fa1" },
};

function getProviderMeta(id: string): { icon: string; color: string } {
  return PROVIDER_ICON_MAP[id.toLowerCase()] ?? { icon: "●", color: "#5d87ff" };
}

function relativeAgo(isoString: string): string {
  const diff = Date.now() - new Date(isoString).getTime();
  const secs = Math.round(Math.abs(diff) / 1000);
  if (secs < 60) return `${secs}s ago`;
  const mins = Math.round(secs / 60);
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.round(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  return `${Math.round(hrs / 24)}d ago`;
}

function relativeIn(isoString: string): string {
  const diff = new Date(isoString).getTime() - Date.now();
  if (diff <= 0) return "now";
  const secs = Math.round(diff / 1000);
  if (secs < 60) return `in ${secs}s`;
  const mins = Math.round(secs / 60);
  if (mins < 60) return `in ${mins}m`;
  const hrs = Math.round(mins / 60);
  if (hrs < 24) return `in ${hrs}h`;
  return `in ${Math.round(hrs / 24)}d`;
}

function UsageBar({
  label,
  rate,
  color,
}: {
  label: string;
  rate: RateWindowSnapshot;
  color: string;
}) {
  const pct = rate.isExhausted ? 100 : Math.min(100, rate.usedPercent);
  const resetHint = rate.resetsAt
    ? `Resets ${relativeIn(rate.resetsAt)}`
    : rate.resetDescription ?? null;

  return (
    <div className="provider-usage-bar">
      <div className="provider-usage-bar__header">
        <span className="provider-usage-bar__label">{label}</span>
        <span
          className="provider-usage-bar__pct"
          style={rate.isExhausted ? { color: "#ff6c6c" } : undefined}
        >
          {rate.isExhausted ? "Exhausted" : `${pct.toFixed(0)}%`}
        </span>
      </div>
      <div className="provider-usage-bar__track">
        <div
          className="provider-usage-bar__fill"
          style={{ width: `${pct}%`, background: color }}
        />
      </div>
      {resetHint && (
        <span className="provider-usage-bar__reset">{resetHint}</span>
      )}
    </div>
  );
}

function ProvidersTab({
  settings,
  providers,
  set,
  saving,
}: TabProps & {
  providers: ProviderCatalogEntry[];
  onNavigate: (tab: SettingsTab) => void;
}) {
  const { t } = useLocale();
  const [selectedId, setSelectedId] = useState<string | null>(
    providers[0]?.id ?? null,
  );
  const [snapshots, setSnapshots] = useState<ProviderUsageSnapshot[]>([]);
  // Locally-owned catalog order so drag-reorder feels instant before the
  // backend `reorder_providers` round-trip settles.
  const [orderedProviders, setOrderedProviders] =
    useState<ProviderCatalogEntry[]>(providers);

  useEffect(() => {
    setOrderedProviders(providers);
  }, [providers]);

  useEffect(() => {
    void getCachedProviders().then(setSnapshots);
  }, []);

  const enabled = new Set(settings.enabledProviders);

  const toggle = (id: string, on: boolean) => {
    const next = new Set(enabled);
    if (on) next.add(id);
    else next.delete(id);
    set({ enabledProviders: [...next].sort() });
  };

  const snapshotMap = new Map(snapshots.map((s) => [s.providerId, s]));

  const rows: ProviderSidebarRow[] = orderedProviders.map((p) => {
    const isOn = enabled.has(p.id);
    const snap = snapshotMap.get(p.id) ?? null;
    return {
      id: p.id,
      displayName: p.displayName,
      enabled: isOn,
      status: deriveProviderStatus(isOn, snap),
      subtitlePrimary: providerSidebarSubtitle(p.id, isOn, snap, t),
      subtitleSecondary: providerSidebarMetric(snap),
    };
  });

  const handleReorder = (ids: string[]) => {
    const byId = new Map(orderedProviders.map((p) => [p.id, p]));
    const next = ids
      .map((id) => byId.get(id))
      .filter((p): p is ProviderCatalogEntry => Boolean(p));
    setOrderedProviders(next);
    void reorderProviders(ids).catch(() => {
      // Roll back to the server-provided order on failure.
      setOrderedProviders(providers);
    });
  };

  return (
    <div className="provider-split">
      <ProvidersSidebar
        providers={rows}
        selectedId={selectedId}
        onSelect={setSelectedId}
        onReorder={handleReorder}
        onToggleEnabled={toggle}
        disabled={saving}
      />
      <ProviderDetailPane providerId={selectedId} />
    </div>
  );
}

// ── Provider sidebar subtitle helpers (port of
//    rust/src/native_ui/preferences.rs::provider_sidebar_subtitle). ─────

function deriveProviderStatus(
  isEnabled: boolean,
  snap: ProviderUsageSnapshot | null,
): ProviderSidebarStatus {
  if (!isEnabled) return "disabled";
  if (!snap) return "loading";
  if (snap.error) return "error";
  const updatedMs = new Date(snap.updatedAt).getTime();
  if (Number.isFinite(updatedMs)) {
    const ageMins = (Date.now() - updatedMs) / 60_000;
    if (ageMins > 10) return "stale";
  }
  return "ok";
}

/**
 * Minimal port of `provider_sidebar_source_hint`
 * (rust/src/native_ui/preferences.rs:3753). When we have a live snapshot the
 * backend-supplied `sourceLabel` wins; otherwise we fall back to the neutral
 * "Not detected" / "Disabled" copy.
 */
function providerSidebarSubtitle(
  providerId: string,
  isEnabled: boolean,
  snap: ProviderUsageSnapshot | null,
  t: (key: LocaleKey) => string,
): string {
  if (!isEnabled) {
    return `${t("ProviderDisabled")} — ${providerSourceHintShort(providerId, t)}`;
  }
  if (!snap) {
    return t("ProviderNotDetected");
  }
  const source = snap.sourceLabel || providerSourceHintShort(providerId, t);
  return source;
}

function providerSourceHintShort(
  providerId: string,
  t: (key: LocaleKey) => string,
): string {
  const id = providerId.toLowerCase();
  switch (id) {
    case "cursor":
    case "factory":
    case "droid":
    case "kimi":
    case "kimik2":
    case "augment":
    case "opencode":
    case "amp":
    case "ollama":
    case "alibaba":
    case "infini":
      return t("ProviderSourceWebShort");
    case "gemini":
    case "antigravity":
    case "jetbrains":
      return t("ProviderSourceCliShort");
    case "copilot":
      return t("ProviderSourceGithubApiShort");
    case "zai":
    case "vertexai":
    case "openrouter":
    case "synthetic":
    case "nanogpt":
    case "warp":
      return t("ProviderSourceApiShort");
    case "kiro":
      return t("ProviderSourceKiroEnvShort");
    case "claude":
    case "codex":
    case "minimax":
    default:
      return t("ProviderSourceAutoShort");
  }
}

function providerSidebarMetric(
  snap: ProviderUsageSnapshot | null,
): string | undefined {
  if (!snap) return undefined;
  const rate = snap.primary;
  if (!rate) return undefined;
  if (rate.isExhausted) return "100%";
  if (Number.isFinite(rate.usedPercent)) {
    return `${Math.round(Math.min(100, rate.usedPercent))}%`;
  }
  return undefined;
}

// ── Display ──────────────────────────────────────────────────────────

function DisplayTab({ settings, set, saving }: TabProps) {
  const { t } = useLocale();
  return (
    <section className="settings-section">
      <h3 className="settings-section__title">{t("SectionUsageRendering")}</h3>
      <Field
        label={t("ShowCreditsExtra")}
        description={t("ShowCreditsExtraHelper")}
      >
        <Toggle
          checked={settings.showCreditsExtraUsage}
          disabled={saving}
          onChange={(v) => set({ showCreditsExtraUsage: v })}
        />
      </Field>

      <h3 className="settings-section__title">{t("PrivacyTitle")}</h3>
      <Field
        label={t("HidePersonalInfo")}
        description={t("HidePersonalInfoHelper")}
      >
        <Toggle
          checked={settings.hidePersonalInfo}
          disabled={saving}
          onChange={(v) => set({ hidePersonalInfo: v })}
        />
      </Field>
    </section>
  );
}

// ── Advanced ─────────────────────────────────────────────────────────

function AdvancedTab({ settings, set, saving }: TabProps) {
  const { t } = useLocale();
  const { updateState, checkNow } = useUpdateState();
  const lastCheckedDisplay = formatRelativeUpdated(
    updateState.lastCheckedAt,
    t,
  );

  return (
    <section className="settings-section">
      {/* ── Refresh ───────────────────────────────────────────────── */}
      <h3 className="settings-section__title">{t("SectionRefresh")}</h3>
      <Field
        label={t("RefreshIntervalLabel")}
        description={t("RefreshIntervalHelper")}
      >
        <NumberInput
          value={settings.refreshIntervalSecs}
          min={0}
          max={3600}
          step={30}
          disabled={saving}
          onChange={(v) => set({ refreshIntervalSecs: v })}
        />
      </Field>

      {/* ── Menu Bar ──────────────────────────────────────────────── */}
      <h3 className="settings-section__title">{t("MenuBar")}</h3>
      <Field
        label={t("TrayIconModeLabel")}
        description={t("TrayIconModeHelper")}
      >
        <Select
          value={settings.trayIconMode}
          disabled={saving}
          options={[
            { value: "single", label: t("TrayIconModeSingle") },
            { value: "perProvider", label: t("TrayIconModePerProvider") },
          ]}
          onChange={(v) => set({ trayIconMode: v as TrayIconMode })}
        />
      </Field>
      <Field
        label={t("ShowProviderIcons")}
        description={t("ShowProviderIconsHelper")}
      >
        <Toggle
          checked={settings.switcherShowsIcons}
          disabled={saving}
          onChange={(v) => set({ switcherShowsIcons: v })}
        />
      </Field>
      <Field
        label={t("PreferHighestUsage")}
        description={t("PreferHighestUsageHelper")}
      >
        <Toggle
          checked={settings.menuBarShowsHighestUsage}
          disabled={saving}
          onChange={(v) => set({ menuBarShowsHighestUsage: v })}
        />
      </Field>
      <Field
        label={t("ShowPercentInTray")}
        description={t("ShowPercentInTrayHelper")}
      >
        <Toggle
          checked={settings.menuBarShowsPercent}
          disabled={saving}
          onChange={(v) => set({ menuBarShowsPercent: v })}
        />
      </Field>
      <Field label={t("DisplayModeLabel")} description={t("DisplayModeHelper")}>
        <Select
          value={settings.menuBarDisplayMode}
          disabled={saving}
          options={[
            { value: "detailed", label: t("DisplayModeDetailed") },
            { value: "compact", label: t("DisplayModeCompact") },
            { value: "minimal", label: t("DisplayModeMinimal") },
          ]}
          onChange={(v) => set({ menuBarDisplayMode: v as MenuBarDisplayMode })}
        />
      </Field>
      <Field label={t("ShowAsUsedLabel")} description={t("ShowAsUsedHelper")}>
        <Toggle
          checked={settings.showAsUsed}
          disabled={saving}
          onChange={(v) => set({ showAsUsed: v })}
        />
      </Field>
      <Field
        label={t("ShowAllTokenAccountsLabel")}
        description={t("ShowAllTokenAccountsHelper")}
      >
        <Toggle
          checked={settings.showAllTokenAccountsInMenu}
          disabled={saving}
          onChange={(v) => set({ showAllTokenAccountsInMenu: v })}
        />
      </Field>

      {/* ── Fun ───────────────────────────────────────────────────── */}
      <h3 className="settings-section__title">{t("Fun")}</h3>
      <Field
        label={t("EnableAnimationsLabel")}
        description={t("EnableAnimationsHelper")}
      >
        <Toggle
          checked={settings.enableAnimations}
          disabled={saving}
          onChange={(v) => set({ enableAnimations: v })}
        />
      </Field>
      <Field
        label={t("SurpriseAnimationsLabel")}
        description={t("SurpriseAnimationsHelper")}
      >
        <Toggle
          checked={settings.surpriseAnimations}
          disabled={saving}
          onChange={(v) => set({ surpriseAnimations: v })}
        />
      </Field>

      {/* ── Credentials & Security ───────────────────────────────── */}
      <h3 className="settings-section__title">
        {t("SectionCredentialsSecurity")}
      </h3>
      <Field
        label={t("AvoidKeychainPromptsLabel")}
        description={t("AvoidKeychainPromptsHelper")}
      >
        <Toggle
          checked={settings.claudeAvoidKeychainPrompts}
          disabled={saving || settings.disableKeychainAccess}
          onChange={(v) => set({ claudeAvoidKeychainPrompts: v })}
        />
      </Field>
      <Field
        label={t("DisableAllKeychainLabel")}
        description={t("DisableAllKeychainHelper")}
      >
        <Toggle
          checked={settings.disableKeychainAccess}
          disabled={saving}
          onChange={(v) => set({ disableKeychainAccess: v })}
        />
      </Field>

      {/* ── Debug ────────────────────────────────────────────────── */}
      <h3 className="settings-section__title">{t("SectionDebug")}</h3>
      <Field
        label={t("ShowDebugSettingsLabel")}
        description={t("ShowDebugSettingsHelper")}
      >
        <Toggle
          checked={settings.showDebugSettings}
          disabled={saving}
          onChange={(v) => set({ showDebugSettings: v })}
        />
      </Field>

      {/* ── Updates ──────────────────────────────────────────────── */}
      <h3 className="settings-section__title">{t("Updates")}</h3>
      <Field
        label={t("UpdateChannelChoice")}
        description={t("UpdateChannelChoiceHelper")}
      >
        <Select
          value={settings.updateChannel}
          disabled={saving}
          options={[
            { value: "stable", label: t("UpdateChannelStableOption") },
            { value: "beta", label: t("UpdateChannelBetaOption") },
          ]}
          onChange={(v) => set({ updateChannel: v as UpdateChannel })}
        />
      </Field>
      <Field
        label={t("AutoDownloadUpdates")}
        description={t("AutoDownloadUpdatesHelper")}
      >
        <Toggle
          checked={settings.autoDownloadUpdates}
          disabled={saving}
          onChange={(v) => set({ autoDownloadUpdates: v })}
        />
      </Field>
      <Field
        label={t("InstallUpdatesOnQuit")}
        description={t("InstallUpdatesOnQuitHelper")}
      >
        <Toggle
          checked={settings.installUpdatesOnQuit}
          disabled={saving}
          onChange={(v) => set({ installUpdatesOnQuit: v })}
        />
      </Field>
      <Field label={t("LastUpdated")}>
        <div className="settings-field__row">
          <span className="settings-field__value">{lastCheckedDisplay}</span>
          <button
            type="button"
            className="credential-btn"
            disabled={updateState.status === "checking"}
            onClick={() => checkNow()}
          >
            {t("TrayCheckForUpdates")}
          </button>
        </div>
      </Field>

      {/* ── Language ─────────────────────────────────────────────── */}
      <h3 className="settings-section__title">{t("SectionLanguage")}</h3>
      <Field label={t("InterfaceLanguage")}>
        <Select
          value={settings.uiLanguage}
          disabled={saving}
          options={[
            { value: "english", label: t("LanguageEnglishOption") },
            { value: "chinese", label: t("LanguageChineseOption") },
          ]}
          onChange={(v) => set({ uiLanguage: v as Language })}
        />
      </Field>

      {/* ── Time ─────────────────────────────────────────────────── */}
      <h3 className="settings-section__title">{t("SectionTime")}</h3>
      <Field
        label={t("ResetTimeRelative")}
        description={t("ResetTimeRelativeHelper")}
      >
        <Toggle
          checked={settings.resetTimeRelative}
          disabled={saving}
          onChange={(v) => set({ resetTimeRelative: v })}
        />
      </Field>
    </section>
  );
}

// ── API Keys ─────────────────────────────────────────────────────────

function ApiKeysTab({ providers }: { providers: ProviderCatalogEntry[] }) {
  const { t } = useLocale();
  const [keys, setKeys] = useState<ApiKeyInfoBridge[]>([]);
  const [apiKeyProviders, setApiKeyProviders] = useState<
    ApiKeyProviderInfoBridge[]
  >([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Which provider is currently being edited
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");
  const [editLabel, setEditLabel] = useState("");

  const reload = useCallback(async () => {
    try {
      const [k, p] = await Promise.all([getApiKeys(), getApiKeyProviders()]);
      setKeys(k);
      setApiKeyProviders(p);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const handleSave = async (providerId: string) => {
    if (!editValue.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const next = await setApiKey(
        providerId,
        editValue.trim(),
        editLabel.trim() || undefined,
      );
      setKeys(next);
      setEditingId(null);
      setEditValue("");
      setEditLabel("");
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleRemove = async (providerId: string) => {
    setBusy(true);
    setError(null);
    try {
      const next = await removeApiKey(providerId);
      setKeys(next);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  // Build a lookup of provider display names
  const providerNames = new Map(providers.map((p) => [p.id, p.displayName]));

  // Merge: show api-key providers with their saved state
  const keyMap = new Map(keys.map((k) => [k.providerId, k]));

  return (
    <section className="settings-section">
      <h3 className="settings-section__title">{t("SectionApiKeys")}</h3>
      <p className="settings-section__hint">{t("ApiKeysTabHint")}</p>

      {error && (
        <div className="settings-status settings-status--error">{error}</div>
      )}

      <ul className="credential-list">
        {apiKeyProviders.map((p) => {
          const saved = keyMap.get(p.id);
          const isEditing = editingId === p.id;
          const displayName = providerNames.get(p.id) ?? p.displayName;

          return (
            <li key={p.id} className="credential-card">
              <div className="credential-card__header">
                <div className="credential-card__info">
                  <strong>{displayName}</strong>
                  <span className="credential-card__meta">
                    {saved ? (
                      <>
                        <span className="credential-card__badge credential-card__badge--set">
                          Configured
                        </span>
                        <span className="credential-card__masked">
                          {saved.maskedKey}
                        </span>
                        {saved.label && (
                          <span className="credential-card__label">
                            {saved.label}
                          </span>
                        )}
                        <span className="credential-card__date">
                          Saved {saved.savedAt}
                        </span>
                      </>
                    ) : (
                      <span className="credential-card__badge credential-card__badge--unset">
                        Not set
                      </span>
                    )}
                  </span>
                </div>
                <div className="credential-card__actions">
                  {!isEditing && (
                    <button
                      className="credential-btn"
                      disabled={busy}
                      onClick={() => {
                        setEditingId(p.id);
                        setEditValue("");
                        setEditLabel(saved?.label ?? "");
                      }}
                    >
                      {saved ? "Update" : "Add Key"}
                    </button>
                  )}
                  {saved && !isEditing && (
                    <button
                      className="credential-btn credential-btn--danger"
                      disabled={busy}
                      onClick={() => void handleRemove(p.id)}
                    >
                      Remove
                    </button>
                  )}
                </div>
              </div>

              {p.help && !isEditing && (
                <p className="credential-card__help">{p.help}</p>
              )}

              {p.dashboardUrl && !isEditing && (
                <a
                  className="credential-card__link"
                  href={p.dashboardUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  Open dashboard ↗
                </a>
              )}

              {isEditing && (
                <div className="credential-card__edit">
                  <input
                    type="password"
                    className="text-input credential-card__input"
                    placeholder="Paste API key…"
                    autoComplete="off"
                    value={editValue}
                    onChange={(e) => setEditValue(e.target.value)}
                    disabled={busy}
                  />
                  <input
                    type="text"
                    className="text-input credential-card__input credential-card__input--label"
                    placeholder="Label (optional)"
                    value={editLabel}
                    onChange={(e) => setEditLabel(e.target.value)}
                    disabled={busy}
                  />
                  <div className="credential-card__edit-actions">
                    <button
                      className="credential-btn credential-btn--primary"
                      disabled={busy || !editValue.trim()}
                      onClick={() => void handleSave(p.id)}
                    >
                      Save
                    </button>
                    <button
                      className="credential-btn"
                      disabled={busy}
                      onClick={() => {
                        setEditingId(null);
                        setEditValue("");
                        setEditLabel("");
                      }}
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              )}
            </li>
          );
        })}
      </ul>
    </section>
  );
}

// ── Cookies ──────────────────────────────────────────────────────────

function CookiesTab({ providers }: { providers: ProviderCatalogEntry[] }) {
  const [cookies, setCookies] = useState<CookieInfoBridge[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Browser import state
  const [browsers, setBrowsers] = useState<DetectedBrowserBridge[]>([]);
  const [browsersLoaded, setBrowsersLoaded] = useState(false);
  const [importProviderId, setImportProviderId] = useState("");
  const [importBrowserType, setImportBrowserType] = useState("");
  const [importStatus, setImportStatus] = useState<string | null>(null);
  const [importError, setImportError] = useState<string | null>(null);

  // Add-cookie form state
  const [addProviderId, setAddProviderId] = useState("");
  const [addCookieValue, setAddCookieValue] = useState("");

  const reload = useCallback(async () => {
    try {
      setCookies(await getManualCookies());
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  // Lazy-load browser list on first render
  useEffect(() => {
    listDetectedBrowsers()
      .then((list) => {
        setBrowsers(list);
        setBrowsersLoaded(true);
        if (list.length > 0) setImportBrowserType(list[0].browserType);
      })
      .catch(() => {
        setBrowsersLoaded(true);
      });
  }, []);

  // Only show providers with a cookie domain
  const cookieProviders = providers.filter((p) => p.cookieDomain !== null);

  const handleAdd = async () => {
    if (!addProviderId || !addCookieValue.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const next = await setManualCookie(addProviderId, addCookieValue.trim());
      setCookies(next);
      setAddProviderId("");
      setAddCookieValue("");
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleRemove = async (providerId: string) => {
    setBusy(true);
    setError(null);
    try {
      const next = await removeManualCookie(providerId);
      setCookies(next);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleBrowserImport = async () => {
    if (!importProviderId || !importBrowserType) return;
    setBusy(true);
    setImportError(null);
    setImportStatus(null);
    try {
      const next = await importBrowserCookies(importProviderId, importBrowserType);
      setCookies(next);
      setImportStatus("Cookies imported successfully.");
      setImportProviderId("");
    } catch (err: unknown) {
      setImportError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="settings-section">
      <h3 className="settings-section__title">Saved Cookies</h3>
      <p className="settings-section__hint">
        Manual cookie overrides for browser-authenticated providers. These are
        used when automatic browser cookie extraction is unavailable.
      </p>

      {error && (
        <div className="settings-status settings-status--error">{error}</div>
      )}

      {cookies.length > 0 ? (
        <ul className="credential-list">
          {cookies.map((c) => (
            <li key={c.providerId} className="credential-card">
              <div className="credential-card__header">
                <div className="credential-card__info">
                  <strong>{c.provider}</strong>
                  <span className="credential-card__meta">
                    <span className="credential-card__badge credential-card__badge--set">
                      Saved
                    </span>
                    <span className="credential-card__date">
                      {c.savedAt}
                    </span>
                  </span>
                </div>
                <div className="credential-card__actions">
                  <button
                    className="credential-btn credential-btn--danger"
                    disabled={busy}
                    onClick={() => void handleRemove(c.providerId)}
                  >
                    Remove
                  </button>
                </div>
              </div>
            </li>
          ))}
        </ul>
      ) : (
        <p className="credential-empty">No manual cookies saved.</p>
      )}

      {/* ── Browser import ── */}
      {browsersLoaded && browsers.length > 0 && (
        <>
          <h3 className="settings-section__title">Import from Browser</h3>
          <p className="settings-section__hint">
            Extract cookies automatically from a signed-in browser.
            The browser must be installed on this machine and you must be
            signed in to the provider in that browser.
          </p>

          {importError && (
            <div className="settings-status settings-status--error">{importError}</div>
          )}
          {importStatus && (
            <div className="settings-status settings-status--ok">{importStatus}</div>
          )}

          <div className="credential-add-form">
            <Select
              value={importProviderId}
              options={[
                { value: "", label: "Select provider…" },
                ...cookieProviders.map((p) => ({
                  value: p.id,
                  label: p.displayName,
                })),
              ]}
              onChange={setImportProviderId}
              disabled={busy}
            />
            <Select
              value={importBrowserType}
              options={browsers.map((b) => ({
                value: b.browserType,
                label: `${b.displayName} (${b.profileCount} profile${b.profileCount !== 1 ? "s" : ""})`,
              }))}
              onChange={setImportBrowserType}
              disabled={busy}
            />
            <button
              className="credential-btn credential-btn--primary"
              disabled={busy || !importProviderId || !importBrowserType}
              onClick={() => void handleBrowserImport()}
            >
              Import Cookies
            </button>
          </div>
        </>
      )}

      {browsersLoaded && browsers.length === 0 && (
        <>
          <h3 className="settings-section__title">Import from Browser</h3>
          <p className="settings-section__hint">
            No supported browsers detected on this machine, or automatic cookie
            extraction is unavailable (requires Windows with Chrome, Edge, Brave,
            or Firefox installed). Use the manual paste form below instead.
          </p>
        </>
      )}

      <h3 className="settings-section__title">Add Cookie Manually</h3>
      <div className="credential-add-form">
        <Select
          value={addProviderId}
          options={[
            { value: "", label: "Select provider…" },
            ...cookieProviders.map((p) => ({
              value: p.id,
              label: p.displayName,
            })),
          ]}
          onChange={setAddProviderId}
          disabled={busy}
        />
        <textarea
          className="text-input credential-textarea"
          placeholder="Paste cookie header value…"
          rows={3}
          value={addCookieValue}
          onChange={(e) => setAddCookieValue(e.target.value)}
          disabled={busy}
        />
        <button
          className="credential-btn credential-btn--primary"
          disabled={busy || !addProviderId || !addCookieValue.trim()}
          onClick={() => void handleAdd()}
        >
          Save Cookie
        </button>
      </div>
    </section>
  );
}

// ── Token Accounts ────────────────────────────────────────────────────

function TokenAccountsTab() {
  const { t } = useLocale();
  const [providers, setProviders] = useState<TokenAccountSupportBridge[]>([]);
  const [selectedProviderId, setSelectedProviderId] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getTokenAccountProviders()
      .then(setProviders)
      .catch((err: unknown) =>
        setError(err instanceof Error ? err.message : String(err)),
      );
  }, []);

  return (
    <section className="settings-section">
      <h3 className="settings-section__title">{t("SectionTokenAccounts")}</h3>
      <p className="settings-section__hint">{t("TokenAccountTabHint")}</p>

      {error && (
        <div className="settings-status settings-status--error">{error}</div>
      )}

      {providers.length === 0 ? (
        <p className="credential-empty">{t("TokenAccountNoSupported")}</p>
      ) : (
        <>
          <div className="settings-field">
            <div className="settings-field__text">
              <span className="settings-field__label">
                {t("TokenAccountProviderLabel")}
              </span>
            </div>
            <div className="settings-field__control">
              <Select
                value={selectedProviderId}
                options={[
                  {
                    value: "",
                    label: t("TokenAccountProviderPlaceholder"),
                  },
                  ...providers.map((p) => ({
                    value: p.providerId,
                    label: p.displayName,
                  })),
                ]}
                onChange={(v) => {
                  setSelectedProviderId(v);
                  setError(null);
                }}
              />
            </div>
          </div>

          {selectedProviderId && (
            <TokenAccountsPanel
              providerId={selectedProviderId}
              compact={false}
            />
          )}
        </>
      )}
    </section>
  );
}

// ── About ─────────────────────────────────────────────────────────────

function AboutTab() {
  const [appInfo, setAppInfo] = useState<AppInfoBridge | null>(null);
  const { updateState, checkNow, download, apply, openRelease } =
    useUpdateState();
  const [hasChecked, setHasChecked] = useState(false);

  useEffect(() => {
    void getAppInfo().then(setAppInfo);
  }, []);

  const handleCheck = () => {
    setHasChecked(true);
    checkNow();
  };

  if (!appInfo) {
    return (
      <section className="settings-section">
        <p className="settings-section__hint">Loading…</p>
      </section>
    );
  }

  const isBusy =
    updateState.status === "checking" ||
    updateState.status === "downloading";

  return (
    <section className="settings-section about-section">
      <div className="about-header">
        <div className="about-icon">⬡</div>
        <div className="about-title-block">
          <h2 className="about-title">{appInfo.name}</h2>
          <p className="about-version">
            v{appInfo.version}
            {appInfo.buildNumber !== "dev" && (
              <span className="about-build"> · Build {appInfo.buildNumber}</span>
            )}
          </p>
        </div>
      </div>

      <p className="about-tagline">{appInfo.tagline}</p>

      <div className="about-details">
        <div className="about-detail-row">
          <span className="about-detail-label">Update channel</span>
          <span className="about-detail-value">{appInfo.updateChannel}</span>
        </div>
      </div>

      <div className="about-actions">
        <button
          className="credential-btn credential-btn--primary"
          disabled={isBusy}
          onClick={handleCheck}
        >
          {updateState.status === "checking"
            ? "Checking…"
            : "Check for Updates"}
        </button>

        {updateState.status === "available" && (
          <div className="about-update-row">
            <span className="about-update-msg">
              Update {updateState.version} available
            </span>
            {updateState.canDownload ? (
              <button
                className="credential-btn credential-btn--primary"
                onClick={download}
              >
                Download
              </button>
            ) : (
              <button className="credential-btn" onClick={openRelease}>
                View Release
              </button>
            )}
          </div>
        )}

        {updateState.status === "downloading" && (
          <span className="about-update-msg">
            Downloading…
            {updateState.progress != null &&
              ` ${Math.round(updateState.progress * 100)}%`}
          </span>
        )}

        {updateState.status === "ready" && (
          <div className="about-update-row">
            <span className="about-update-msg">Update ready to install</span>
            {updateState.canApply ? (
              <button
                className="credential-btn credential-btn--primary"
                onClick={apply}
              >
                Install &amp; Restart
              </button>
            ) : (
              <button className="credential-btn" onClick={openRelease}>
                View Release
              </button>
            )}
          </div>
        )}

        {updateState.status === "error" && (
          <span className="about-update-msg">
            Error: {updateState.error}
          </span>
        )}

        {updateState.status === "idle" && hasChecked && (
          <span className="about-update-msg">You&apos;re up to date!</span>
        )}
      </div>

      <div className="about-links">
        <a
          className="about-link"
          href="https://github.com/NessZerra/Win-CodexBar"
          target="_blank"
          rel="noopener noreferrer"
        >
          GitHub
        </a>
        <span className="about-link-sep">·</span>
        <a
          className="about-link"
          href="https://codexbar.app"
          target="_blank"
          rel="noopener noreferrer"
        >
          Website
        </a>
      </div>

      <p className="about-copyright">
        NessZerra — Windows Version. MIT License.
      </p>
    </section>
  );
}
