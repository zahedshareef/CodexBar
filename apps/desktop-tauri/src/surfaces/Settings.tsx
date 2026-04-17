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
  ProviderTokenAccountsBridge,
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
import {
  getApiKeyProviders,
  getApiKeys,
  getAppInfo,
  getCachedProviders,
  getManualCookies,
  getTokenAccountProviders,
  getTokenAccounts,
  addTokenAccount,
  removeTokenAccount,
  setActiveTokenAccount,
  importBrowserCookies,
  listDetectedBrowsers,
  removeApiKey,
  removeManualCookie,
  setApiKey,
  setManualCookie,
  setSurfaceMode,
} from "../lib/tauri";

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

const TAB_META: { id: SettingsTab; label: string; icon: string }[] = [
  { id: "general", label: "General", icon: "⚙" },
  { id: "providers", label: "Providers", icon: "◉" },
  { id: "display", label: "Display", icon: "◧" },
  { id: "apiKeys", label: "API Keys", icon: "🔑" },
  { id: "cookies", label: "Cookies", icon: "🍪" },
  { id: "tokenAccounts", label: "Tokens", icon: "🪙" },
  { id: "advanced", label: "Advanced", icon: "⌘" },
  { id: "about", label: "About", icon: "ℹ" },
];

// ── main component ──────────────────────────────────────────────────

function isSettingsTab(value: string): value is SettingsTab {
  return TAB_META.some((t) => t.id === value);
}

export default function Settings({ state }: { state: BootstrapState }) {
  const { settings, saving, error, update } = useSettings(state.settings);
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
        {TAB_META.map((t) => (
          <button
            key={t.id}
            role="tab"
            aria-selected={activeTab === t.id}
            className={`settings-tab ${activeTab === t.id ? "settings-tab--active" : ""}`}
            onClick={() => handleTabClick(t.id)}
          >
            <span className="settings-tab__icon">{t.icon}</span>
            {t.label}
          </button>
        ))}
      </nav>

      {/* status bar */}
      {(saving || error) && (
        <div className={`settings-status ${error ? "settings-status--error" : ""}`}>
          {saving ? "Saving…" : error}
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
  return (
    <section className="settings-section">
      <h3 className="settings-section__title">Startup</h3>
      <Field label="Start at login" description="Launch CodexBar automatically when you sign in.">
        <Toggle
          checked={settings.startAtLogin}
          disabled={saving}
          onChange={(v) => set({ startAtLogin: v })}
        />
      </Field>
      <Field label="Start minimized" description="Hide the main window on launch; only the tray icon is visible.">
        <Toggle
          checked={settings.startMinimized}
          disabled={saving}
          onChange={(v) => set({ startMinimized: v })}
        />
      </Field>

      <h3 className="settings-section__title">Refresh</h3>
      <Field label="Refresh interval" description="Seconds between automatic provider refreshes (0 = manual).">
        <NumberInput
          value={settings.refreshIntervalSecs}
          min={0}
          max={3600}
          step={30}
          disabled={saving}
          onChange={(v) => set({ refreshIntervalSecs: v })}
        />
      </Field>

      <h3 className="settings-section__title">Notifications</h3>
      <Field label="Show notifications" description="Display desktop alerts for usage thresholds.">
        <Toggle
          checked={settings.showNotifications}
          disabled={saving}
          onChange={(v) => set({ showNotifications: v })}
        />
      </Field>
      <Field label="Sound alerts" description="Play a sound when usage thresholds are hit.">
        <Toggle
          checked={settings.soundEnabled}
          disabled={saving}
          onChange={(v) => set({ soundEnabled: v })}
        />
      </Field>
      {settings.soundEnabled && (
        <Field label="Alert volume" description="Volume for threshold alert sounds (0–100).">
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

      <h3 className="settings-section__title">Usage Thresholds</h3>
      <Field label="High usage warning (%)" description="Show a warning when usage exceeds this percentage.">
        <NumberInput
          value={settings.highUsageThreshold}
          min={0}
          max={100}
          step={5}
          disabled={saving}
          onChange={(v) => set({ highUsageThreshold: v })}
        />
      </Field>
      <Field label="Critical usage alert (%)" description="Show a critical alert when usage exceeds this percentage.">
        <NumberInput
          value={settings.criticalUsageThreshold}
          min={0}
          max={100}
          step={5}
          disabled={saving}
          onChange={(v) => set({ criticalUsageThreshold: v })}
        />
      </Field>

      <h3 className="settings-section__title">Keyboard</h3>
      <Field label="Global shortcut" description="Key combination to toggle the tray panel.">
        <TextInput
          value={settings.globalShortcut}
          placeholder="Ctrl+Shift+U"
          disabled={saving}
          onChange={(v) => set({ globalShortcut: v })}
        />
      </Field>
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
  onNavigate,
}: TabProps & {
  providers: ProviderCatalogEntry[];
  onNavigate: (tab: SettingsTab) => void;
}) {
  const [selectedId, setSelectedId] = useState<string | null>(
    providers[0]?.id ?? null,
  );
  const [snapshots, setSnapshots] = useState<ProviderUsageSnapshot[]>([]);
  const [apiKeyProviders, setApiKeyProviders] = useState<
    ApiKeyProviderInfoBridge[]
  >([]);

  useEffect(() => {
    void Promise.all([getCachedProviders(), getApiKeyProviders()]).then(
      ([snaps, akps]) => {
        setSnapshots(snaps);
        setApiKeyProviders(akps);
      },
    );
  }, []);

  const enabled = new Set(settings.enabledProviders);

  const toggle = (id: string, on: boolean) => {
    const next = new Set(enabled);
    if (on) next.add(id);
    else next.delete(id);
    set({ enabledProviders: [...next].sort() });
  };

  const setMetric = (id: string, metric: string) => {
    const next = { ...settings.providerMetrics, [id]: metric as MetricPreference };
    set({ providerMetrics: next });
  };

  const snapshotMap = new Map(snapshots.map((s) => [s.providerId, s]));
  const apiKeyMap = new Map(apiKeyProviders.map((p) => [p.id, p]));

  const selectedProvider = providers.find((p) => p.id === selectedId) ?? null;
  const selectedSnap = selectedId ? snapshotMap.get(selectedId) ?? null : null;
  const selectedApiKey = selectedId ? apiKeyMap.get(selectedId) ?? null : null;
  const isEnabled = selectedId ? enabled.has(selectedId) : false;
  const { icon: selIcon, color: selColor } = selectedId
    ? getProviderMeta(selectedId)
    : { icon: "●", color: "#5d87ff" };

  return (
    <div className="provider-split">
      {/* ── Sidebar ── */}
      <div className="provider-sidebar">
        {providers.map((p) => {
          const { icon, color } = getProviderMeta(p.id);
          const isOn = enabled.has(p.id);
          const isSelected = p.id === selectedId;
          return (
            <div
              key={p.id}
              className={`provider-sidebar-item${isSelected ? " provider-sidebar-item--selected" : ""}`}
              onClick={() => setSelectedId(p.id)}
            >
              <span className="provider-sidebar-icon" style={{ color }}>
                {icon}
              </span>
              <span className="provider-sidebar-name">{p.displayName}</span>
              {isOn && <span className="provider-sidebar-dot" />}
              {/* stop click propagation so toggle click doesn't also select */}
              <span onClick={(e) => e.stopPropagation()}>
                <Toggle
                  checked={isOn}
                  disabled={saving}
                  onChange={(v) => toggle(p.id, v)}
                />
              </span>
            </div>
          );
        })}
      </div>

      {/* ── Detail panel ── */}
      <div className="provider-detail">
        {!selectedProvider ? (
          <div className="provider-detail-empty">
            Select a provider to see details.
          </div>
        ) : (
          <>
            {/* Header */}
            <div className="provider-detail-header">
              <span
                className="provider-detail-icon"
                style={{ color: selColor }}
              >
                {selIcon}
              </span>
              <span className="provider-detail-title">
                {selectedProvider.displayName}
              </span>
              <Toggle
                checked={isEnabled}
                disabled={saving}
                onChange={(v) => toggle(selectedProvider.id, v)}
              />
            </div>

            {/* Info grid */}
            <dl className="provider-detail-grid">
              <dt>State</dt>
              <dd>{isEnabled ? "Enabled" : "Disabled"}</dd>
              {selectedSnap && (
                <>
                  <dt>Source</dt>
                  <dd>{selectedSnap.sourceLabel || "-"}</dd>

                  <dt>Updated</dt>
                  <dd>{relativeAgo(selectedSnap.updatedAt)}</dd>

                  {!settings.hidePersonalInfo && selectedSnap.accountEmail && (
                    <>
                      <dt>Account</dt>
                      <dd>{selectedSnap.accountEmail}</dd>
                    </>
                  )}

                  {selectedSnap.planName && (
                    <>
                      <dt>Plan</dt>
                      <dd>{selectedSnap.planName}</dd>
                    </>
                  )}

                  {!settings.hidePersonalInfo &&
                    selectedSnap.accountOrganization && (
                      <>
                        <dt>Organization</dt>
                        <dd>{selectedSnap.accountOrganization}</dd>
                      </>
                    )}

                  {selectedSnap.error && (
                    <>
                      <dt>Error</dt>
                      <dd className="error">{selectedSnap.error}</dd>
                    </>
                  )}
                </>
              )}
            </dl>

            {/* Usage section */}
            {selectedSnap && isEnabled && (
              <div className="provider-detail-section">
                <h4>Usage</h4>
                <UsageBar
                  label="Session"
                  rate={selectedSnap.primary}
                  color={selColor}
                />
                {selectedSnap.secondary && (
                  <UsageBar
                    label="Weekly"
                    rate={selectedSnap.secondary}
                    color={selColor}
                  />
                )}
                {selectedSnap.tertiary && (
                  <UsageBar
                    label="Tertiary"
                    rate={selectedSnap.tertiary}
                    color={selColor}
                  />
                )}
              </div>
            )}

            {/* Settings section */}
            <div className="provider-detail-section">
              <h4>Settings</h4>
              <Field label="Metric">
                <Select
                  value={
                    settings.providerMetrics[selectedProvider.id] ?? "automatic"
                  }
                  options={METRIC_OPTIONS}
                  disabled={saving}
                  onChange={(v) => setMetric(selectedProvider.id, v)}
                />
              </Field>
            </div>

            {/* Credentials section */}
            {(selectedProvider.cookieDomain !== null || selectedApiKey) && (
              <div className="provider-detail-section">
                <h4>Credentials</h4>
                <div className="provider-detail-actions">
                  {selectedProvider.cookieDomain !== null && (
                    <button
                      className="credential-btn"
                      onClick={() => onNavigate("cookies")}
                    >
                      Configure Cookie
                    </button>
                  )}
                  {selectedApiKey && (
                    <>
                      <button
                        className="credential-btn"
                        onClick={() => onNavigate("apiKeys")}
                      >
                        Configure API Key
                      </button>
                      {selectedApiKey.dashboardUrl && (
                        <a
                          className="credential-card__link"
                          href={selectedApiKey.dashboardUrl}
                          target="_blank"
                          rel="noopener noreferrer"
                        >
                          Open dashboard ↗
                        </a>
                      )}
                    </>
                  )}
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}

// ── Display ──────────────────────────────────────────────────────────

function DisplayTab({ settings, set, saving }: TabProps) {
  return (
    <section className="settings-section">
      <h3 className="settings-section__title">Tray icon</h3>
      <Field label="Tray icon mode" description="Single unified icon or one icon per enabled provider.">
        <Select
          value={settings.trayIconMode}
          disabled={saving}
          options={[
            { value: "single", label: "Single" },
            { value: "perProvider", label: "Per provider" },
          ]}
          onChange={(v) => set({ trayIconMode: v as TrayIconMode })}
        />
      </Field>
      <Field label="Show provider icons" description="Display provider icons in the tray switcher.">
        <Toggle
          checked={settings.switcherShowsIcons}
          disabled={saving}
          onChange={(v) => set({ switcherShowsIcons: v })}
        />
      </Field>
      <Field
        label="Prefer highest usage"
        description="Show the provider closest to its limit in the merged tray display."
      >
        <Toggle
          checked={settings.menuBarShowsHighestUsage}
          disabled={saving}
          onChange={(v) => set({ menuBarShowsHighestUsage: v })}
        />
      </Field>
      <Field
        label="Show percent in tray"
        description="Replace usage bar with provider branding + percentage text."
      >
        <Toggle
          checked={settings.menuBarShowsPercent}
          disabled={saving}
          onChange={(v) => set({ menuBarShowsPercent: v })}
        />
      </Field>

      <Field label="Display mode" description="Level of detail shown in the menu bar label.">
        <Select
          value={settings.menuBarDisplayMode}
          disabled={saving}
          options={[
            { value: "detailed", label: "Detailed" },
            { value: "compact", label: "Compact" },
            { value: "minimal", label: "Minimal" },
          ]}
          onChange={(v) => set({ menuBarDisplayMode: v as MenuBarDisplayMode })}
        />
      </Field>

      <h3 className="settings-section__title">Usage rendering</h3>
      <Field label="Show as used" description="Display usage bars as consumed rather than remaining.">
        <Toggle
          checked={settings.showAsUsed}
          disabled={saving}
          onChange={(v) => set({ showAsUsed: v })}
        />
      </Field>
      <Field
        label="Show credits & extra usage"
        description="Display credit balance and additional usage information."
      >
        <Toggle
          checked={settings.showCreditsExtraUsage}
          disabled={saving}
          onChange={(v) => set({ showCreditsExtraUsage: v })}
        />
      </Field>
      <Field
        label="Show all token accounts"
        description="List all token accounts in provider menus instead of collapsing them."
      >
        <Toggle
          checked={settings.showAllTokenAccountsInMenu}
          disabled={saving}
          onChange={(v) => set({ showAllTokenAccountsInMenu: v })}
        />
      </Field>

      <h3 className="settings-section__title">Animations</h3>
      <Field label="Enable animations" description="Smooth transitions and animated progress bars.">
        <Toggle
          checked={settings.enableAnimations}
          disabled={saving}
          onChange={(v) => set({ enableAnimations: v })}
        />
      </Field>
      <Field label="Surprise animations" description="Fun confetti and particle effects at milestones.">
        <Toggle
          checked={settings.surpriseAnimations}
          disabled={saving}
          onChange={(v) => set({ surpriseAnimations: v })}
        />
      </Field>

      <h3 className="settings-section__title">Privacy</h3>
      <Field label="Hide personal info" description="Mask emails and account names in the UI.">
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
  return (
    <section className="settings-section">
      <h3 className="settings-section__title">Updates</h3>
      <Field label="Update channel" description="Stable for production releases, Beta for early access.">
        <Select
          value={settings.updateChannel}
          disabled={saving}
          options={[
            { value: "stable", label: "Stable" },
            { value: "beta", label: "Beta" },
          ]}
          onChange={(v) => set({ updateChannel: v as UpdateChannel })}
        />
      </Field>
      <Field
        label="Auto-download updates"
        description="Download available updates in the background automatically."
      >
        <Toggle
          checked={settings.autoDownloadUpdates}
          disabled={saving}
          onChange={(v) => set({ autoDownloadUpdates: v })}
        />
      </Field>
      <Field
        label="Install updates on quit"
        description="Apply a downloaded update when you next quit CodexBar."
      >
        <Toggle
          checked={settings.installUpdatesOnQuit}
          disabled={saving}
          onChange={(v) => set({ installUpdatesOnQuit: v })}
        />
      </Field>

      <h3 className="settings-section__title">Language</h3>
      <Field label="Interface language" description="Language used throughout the UI.">
        <Select
          value={settings.uiLanguage}
          disabled={saving}
          options={[
            { value: "english", label: "English" },
            { value: "chinese", label: "中文" },
          ]}
          onChange={(v) => set({ uiLanguage: v as Language })}
        />
      </Field>

      <h3 className="settings-section__title">Time</h3>
      <Field label="Reset time relative" description="Show reset countdowns as relative times (e.g. 'in 3h').">
        <Toggle
          checked={settings.resetTimeRelative}
          disabled={saving}
          onChange={(v) => set({ resetTimeRelative: v })}
        />
      </Field>

      <h3 className="settings-section__title">Credentials &amp; Security</h3>
      <Field
        label="Avoid keychain prompts (Claude)"
        description="Skip keychain credential reads for Claude to prevent OS permission dialogs."
      >
        <Toggle
          checked={settings.claudeAvoidKeychainPrompts}
          disabled={saving || settings.disableKeychainAccess}
          onChange={(v) => set({ claudeAvoidKeychainPrompts: v })}
        />
      </Field>
      <Field
        label="Disable all keychain access"
        description="Turn off credential/keychain reads for all providers. Also enables the Claude option above."
      >
        <Toggle
          checked={settings.disableKeychainAccess}
          disabled={saving}
          onChange={(v) => set({ disableKeychainAccess: v })}
        />
      </Field>

      <h3 className="settings-section__title">Debug</h3>
      <Field
        label="Show debug settings"
        description="Reveal troubleshooting and developer surfaces in the UI."
      >
        <Toggle
          checked={settings.showDebugSettings}
          disabled={saving}
          onChange={(v) => set({ showDebugSettings: v })}
        />
      </Field>
    </section>
  );
}

// ── API Keys ─────────────────────────────────────────────────────────

function ApiKeysTab({ providers }: { providers: ProviderCatalogEntry[] }) {
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
      <h3 className="settings-section__title">API Keys</h3>
      <p className="settings-section__hint">
        Configure API keys for providers that use token-based authentication.
        Keys are stored locally and never transmitted.
      </p>

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
  const [providers, setProviders] = useState<TokenAccountSupportBridge[]>([]);
  const [selectedProviderId, setSelectedProviderId] = useState("");
  const [providerData, setProviderData] =
    useState<ProviderTokenAccountsBridge | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Add-account form
  const [addLabel, setAddLabel] = useState("");
  const [addToken, setAddToken] = useState("");

  useEffect(() => {
    getTokenAccountProviders()
      .then(setProviders)
      .catch((err: unknown) =>
        setError(err instanceof Error ? err.message : String(err)),
      );
  }, []);

  useEffect(() => {
    if (!selectedProviderId) {
      setProviderData(null);
      return;
    }
    setBusy(true);
    setError(null);
    getTokenAccounts(selectedProviderId)
      .then((data) => {
        setProviderData(data);
      })
      .catch((err: unknown) =>
        setError(err instanceof Error ? err.message : String(err)),
      )
      .finally(() => setBusy(false));
  }, [selectedProviderId]);

  const handleAdd = async () => {
    if (!selectedProviderId || !addLabel.trim() || !addToken.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const data = await addTokenAccount(
        selectedProviderId,
        addLabel.trim(),
        addToken.trim(),
      );
      setProviderData(data);
      setAddLabel("");
      setAddToken("");
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleRemove = async (accountId: string) => {
    if (!selectedProviderId) return;
    setBusy(true);
    setError(null);
    try {
      const data = await removeTokenAccount(selectedProviderId, accountId);
      setProviderData(data);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleSetActive = async (accountId: string) => {
    if (!selectedProviderId) return;
    setBusy(true);
    setError(null);
    try {
      const data = await setActiveTokenAccount(selectedProviderId, accountId);
      setProviderData(data);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const placeholder = providerData?.support.placeholder ?? "Paste token…";

  return (
    <section className="settings-section">
      <h3 className="settings-section__title">Token Accounts</h3>
      <p className="settings-section__hint">
        Manage multiple session tokens or API tokens per provider. The active
        account is used for all fetches. Only providers that require manual
        tokens appear here.
      </p>

      {error && (
        <div className="settings-status settings-status--error">{error}</div>
      )}

      <div className="settings-field">
        <div className="settings-field__text">
          <span className="settings-field__label">Provider</span>
        </div>
        <div className="settings-field__control">
          <Select
            value={selectedProviderId}
            options={[
              { value: "", label: "Select provider…" },
              ...providers.map((p) => ({
                value: p.providerId,
                label: p.displayName,
              })),
            ]}
            onChange={(v) => {
              setSelectedProviderId(v);
              setAddLabel("");
              setAddToken("");
              setError(null);
            }}
            disabled={busy}
          />
        </div>
      </div>

      {selectedProviderId && providerData && (
        <>
          <p className="settings-section__hint">{providerData.support.subtitle}</p>

          <h3 className="settings-section__title">Saved Accounts</h3>
          {providerData.accounts.length > 0 ? (
            <ul className="credential-list">
              {providerData.accounts.map((acct) => (
                <li key={acct.id} className="credential-card">
                  <div className="credential-card__header">
                    <div className="credential-card__info">
                      <strong>{acct.label}</strong>
                      <span className="credential-card__meta">
                        {acct.isActive && (
                          <span className="credential-card__badge credential-card__badge--set">
                            Active
                          </span>
                        )}
                        <span className="credential-card__date">
                          Added {acct.addedAt}
                        </span>
                        {acct.lastUsed && (
                          <span className="credential-card__date">
                            · Used {acct.lastUsed}
                          </span>
                        )}
                      </span>
                    </div>
                    <div className="credential-card__actions">
                      {!acct.isActive && (
                        <button
                          className="credential-btn credential-btn--secondary"
                          disabled={busy}
                          onClick={() => void handleSetActive(acct.id)}
                        >
                          Set Active
                        </button>
                      )}
                      <button
                        className="credential-btn credential-btn--danger"
                        disabled={busy}
                        onClick={() => void handleRemove(acct.id)}
                      >
                        Remove
                      </button>
                    </div>
                  </div>
                </li>
              ))}
            </ul>
          ) : (
            <p className="credential-empty">No accounts saved for this provider.</p>
          )}

          <h3 className="settings-section__title">Add Account</h3>
          <div className="credential-add-form">
            <input
              className="text-input"
              type="text"
              placeholder="Label (e.g. Work, Personal)…"
              value={addLabel}
              onChange={(e) => setAddLabel(e.target.value)}
              disabled={busy}
            />
            <textarea
              className="text-input credential-textarea"
              placeholder={placeholder}
              rows={3}
              value={addToken}
              onChange={(e) => setAddToken(e.target.value)}
              disabled={busy}
            />
            <button
              className="credential-btn credential-btn--primary"
              disabled={busy || !addLabel.trim() || !addToken.trim()}
              onClick={() => void handleAdd()}
            >
              Add Account
            </button>
          </div>
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
