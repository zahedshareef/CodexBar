import { useCallback, useEffect, useState } from "react";
import type {
  BootstrapState,
  ProviderCatalogEntry,
  ProviderUsageSnapshot,
  RateWindowSnapshot,
  SettingsTabId,
  SettingsUpdate,
} from "../types/bridge";
import { useSettings } from "../hooks/useSettings";
import { useSurfaceTarget } from "../hooks/useSurfaceMode";
import { useLocale } from "../hooks/useLocale";
import type { LocaleKey } from "../i18n/keys";
import {
  ProvidersSidebar,
  type ProviderSidebarRow,
  type ProviderSidebarStatus,
} from "./settings/providers/ProvidersSidebar";
import { ProviderDetailPane } from "./settings/providers/ProviderDetailPane";
import {
  getCachedProviders,
  reorderProviders,
  setSurfaceMode,
} from "../lib/tauri";
import GeneralTab from "./settings/tabs/GeneralTab";
import DisplayTab from "./settings/tabs/DisplayTab";
import AdvancedTab from "./settings/tabs/AdvancedTab";
import ApiKeysTab from "./settings/tabs/ApiKeysTab";
import CookiesTab from "./settings/tabs/CookiesTab";
import TokenAccountsTab from "./settings/tabs/TokenAccountsTab";
import AboutTab from "./settings/tabs/AboutTab";

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

// ── Tab props shared with extracted tab components ──────────────────

export interface TabProps {
  settings: BootstrapState["settings"];
  set: (p: SettingsUpdate) => void;
  saving: boolean;
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
