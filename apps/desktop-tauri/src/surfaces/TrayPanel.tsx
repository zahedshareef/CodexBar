import { useCallback, useMemo, useState } from "react";
import type { BootstrapState, ProviderUsageSnapshot } from "../types/bridge";
import { setSurfaceMode } from "../lib/tauri";
import { useProviders } from "../hooks/useProviders";
import { useSettings } from "../hooks/useSettings";
import { useUpdateState } from "../hooks/useUpdateState";
import ProviderCard from "./tray/ProviderCard";
import ProviderDetail from "./tray/ProviderDetail";
import UpdateBanner from "../components/UpdateBanner";

/** Sort: highest primary used% first, then alphabetical by name. */
function sortProviders(list: ProviderUsageSnapshot[]): ProviderUsageSnapshot[] {
  return [...list].sort((a, b) => {
    const diff = b.primary.usedPercent - a.primary.usedPercent;
    if (Math.abs(diff) > 0.01) return diff;
    return a.displayName.localeCompare(b.displayName);
  });
}

export default function TrayPanel({
  state,
}: {
  state: BootstrapState;
}) {
  const { providers, isRefreshing, refresh, lastRefresh } = useProviders();
  const { settings } = useSettings(state.settings);
  const { updateState, checkNow, download, apply, dismiss, openRelease } =
    useUpdateState();
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const sorted = useMemo(() => sortProviders(providers), [providers]);

  const selected = useMemo(
    () => sorted.find((p) => p.providerId === selectedId) ?? null,
    [sorted, selectedId],
  );

  const errorCount = useMemo(
    () => sorted.filter((p) => p.error !== null).length,
    [sorted],
  );

  const openSettings = useCallback(() => {
    setSurfaceMode("settings", { kind: "settings", tab: "general" });
  }, []);

  const openPopOut = useCallback(() => {
    setSurfaceMode("popOut", { kind: "dashboard" });
  }, []);

  const handleBack = useCallback(() => {
    setSelectedId(null);
  }, []);

  // Detail drill-in view
  if (selected) {
    return (
      <main className="shell shell--tray-panel">
        <ProviderDetail
          provider={selected}
          hideEmail={settings.hidePersonalInfo}
          resetRelative={settings.resetTimeRelative}
          onBack={handleBack}
        />
      </main>
    );
  }

  // Loading state
  if (isRefreshing && sorted.length === 0) {
    return (
      <main className="shell shell--tray-panel">
        <TrayHeader
          onRefresh={refresh}
          isRefreshing={isRefreshing}
          onSettings={openSettings}
          onPopOut={openPopOut}
        />
        <UpdateBanner updateState={updateState} onCheck={checkNow} onDownload={download} onApply={apply} onDismiss={dismiss} onOpenRelease={openRelease} />
        <div className="tray-empty">
          <div className="tray-empty__spinner" />
          <p>Fetching provider data…</p>
        </div>
      </main>
    );
  }

  // Empty state
  if (!isRefreshing && sorted.length === 0) {
    return (
      <main className="shell shell--tray-panel">
        <TrayHeader
          onRefresh={refresh}
          isRefreshing={isRefreshing}
          onSettings={openSettings}
          onPopOut={openPopOut}
        />
        <UpdateBanner updateState={updateState} onCheck={checkNow} onDownload={download} onApply={apply} onDismiss={dismiss} onOpenRelease={openRelease} />
        <div className="tray-empty">
          <p>No providers configured.</p>
          <p className="tray-empty__hint">
            Enable providers in Settings to see usage data.
          </p>
          <button
            className="tray-btn tray-btn--primary"
            onClick={openSettings}
            type="button"
          >
            Open Settings
          </button>
        </div>
      </main>
    );
  }

  // Main list view
  return (
    <main className="shell shell--tray-panel">
      <TrayHeader
        onRefresh={refresh}
        isRefreshing={isRefreshing}
        onSettings={openSettings}
        onPopOut={openPopOut}
      />

      <UpdateBanner updateState={updateState} onCheck={checkNow} onDownload={download} onApply={apply} onDismiss={dismiss} onOpenRelease={openRelease} />

      <TraySummary
        total={sorted.length}
        errorCount={errorCount}
        isRefreshing={isRefreshing}
        lastRefresh={lastRefresh}
      />

      <div className="tray-list">
        {sorted.map((p) => (
          <ProviderCard
            key={p.providerId}
            provider={p}
            selected={selectedId === p.providerId}
            hideEmail={settings.hidePersonalInfo}
            resetRelative={settings.resetTimeRelative}
            onSelect={() => setSelectedId(p.providerId)}
          />
        ))}
      </div>
    </main>
  );
}

// ── Header ───────────────────────────────────────────────────────────

function TrayHeader({
  onRefresh,
  isRefreshing,
  onSettings,
  onPopOut,
}: {
  onRefresh: () => void;
  isRefreshing: boolean;
  onSettings: () => void;
  onPopOut: () => void;
}) {
  return (
    <header className="tray-header">
      <h1 className="tray-header__title">CodexBar</h1>
      <div className="tray-header__actions">
        <button
          className="tray-icon-btn"
          onClick={onRefresh}
          disabled={isRefreshing}
          title="Refresh"
          type="button"
        >
          <span className={isRefreshing ? "spin" : ""}>↻</span>
        </button>
        <button
          className="tray-icon-btn"
          onClick={onSettings}
          title="Settings"
          type="button"
        >
          ⚙
        </button>
        <button
          className="tray-icon-btn"
          onClick={onPopOut}
          title="Pop out"
          type="button"
        >
          ⧉
        </button>
      </div>
    </header>
  );
}

// ── Summary strip ────────────────────────────────────────────────────

function TraySummary({
  total,
  errorCount,
  isRefreshing,
  lastRefresh,
}: {
  total: number;
  errorCount: number;
  isRefreshing: boolean;
  lastRefresh: { providerCount: number; errorCount: number } | null;
}) {
  const parts: string[] = [];
  parts.push(`${total} provider${total !== 1 ? "s" : ""}`);
  if (isRefreshing) {
    parts.push("refreshing…");
  } else if (lastRefresh) {
    if (lastRefresh.errorCount > 0) {
      parts.push(`${lastRefresh.errorCount} failed`);
    }
  }
  if (!isRefreshing && errorCount > 0) {
    parts.push(`${errorCount} with errors`);
  }

  return <div className="tray-summary">{parts.join(" · ")}</div>;
}
