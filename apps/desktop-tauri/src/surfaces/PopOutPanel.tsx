import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  BootstrapState,
  ProviderUsageSnapshot,
  SurfaceTarget,
} from "../types/bridge";
import { setSurfaceMode } from "../lib/tauri";
import { useSurfaceTarget } from "../hooks/useSurfaceMode";
import { useProviders } from "../hooks/useProviders";
import { useSettings } from "../hooks/useSettings";
import { useUpdateState } from "../hooks/useUpdateState";
import ProviderCard from "./tray/ProviderCard";
import ProviderDetail from "./tray/ProviderDetail";
import UpdateBanner from "../components/UpdateBanner";

/** Sort: highest primary used% first, then alphabetical by name. */
function sortProviders(
  list: ProviderUsageSnapshot[],
): ProviderUsageSnapshot[] {
  return [...list].sort((a, b) => {
    const diff = b.primary.usedPercent - a.primary.usedPercent;
    if (Math.abs(diff) > 0.01) return diff;
    return a.displayName.localeCompare(b.displayName);
  });
}

export default function PopOutPanel({
  state,
  initialTarget,
}: {
  state: BootstrapState;
  initialTarget?: SurfaceTarget | null;
}) {
  const { providers, isRefreshing, refresh, lastRefresh } = useProviders();
  const { settings } = useSettings(state.settings);
  const { updateState, checkNow, download, apply, dismiss, openRelease } =
    useUpdateState();
  const [selectedId, setSelectedId] = useState<string | null>(
    initialTarget?.kind === "provider" ? initialTarget.providerId : null,
  );
  const shellTarget = useSurfaceTarget("popOut");

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

  const goTray = useCallback(() => {
    setSurfaceMode("trayPanel", { kind: "summary" });
  }, []);

  const toggleSelect = useCallback((id: string) => {
    const nextSelectedId = selectedId === id ? null : id;
    setSelectedId(nextSelectedId);
    void setSurfaceMode(
      "popOut",
      nextSelectedId === null
        ? { kind: "dashboard" }
        : { kind: "provider", providerId: id },
    );
  }, [selectedId]);

  const handleBack = useCallback(() => {
    setSelectedId(null);
    void setSurfaceMode("popOut", { kind: "dashboard" });
  }, []);

  useEffect(() => {
    if (!shellTarget) {
      return;
    }

    if (shellTarget.kind === "provider") {
      setSelectedId((current) =>
        current === shellTarget.providerId ? current : shellTarget.providerId,
      );
      return;
    }

    setSelectedId(null);
  }, [shellTarget]);

  // Loading
  if (isRefreshing && sorted.length === 0) {
    return (
      <main className="shell shell--popout">
        <PopOutHeader
          onRefresh={refresh}
          isRefreshing={isRefreshing}
          onSettings={openSettings}
          onTray={goTray}
        />
        <UpdateBanner updateState={updateState} onCheck={checkNow} onDownload={download} onApply={apply} onDismiss={dismiss} onOpenRelease={openRelease} />
        <div className="popout-empty">
          <div className="tray-empty__spinner" />
          <p>Fetching provider data…</p>
        </div>
      </main>
    );
  }

  // Empty
  if (!isRefreshing && sorted.length === 0) {
    return (
      <main className="shell shell--popout">
        <PopOutHeader
          onRefresh={refresh}
          isRefreshing={isRefreshing}
          onSettings={openSettings}
          onTray={goTray}
        />
        <UpdateBanner updateState={updateState} onCheck={checkNow} onDownload={download} onApply={apply} onDismiss={dismiss} onOpenRelease={openRelease} />
        <div className="popout-empty">
          <p>No providers configured.</p>
          <p className="popout-empty__hint">
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

  // Main view
  return (
    <main className="shell shell--popout">
      <PopOutHeader
        onRefresh={refresh}
        isRefreshing={isRefreshing}
        onSettings={openSettings}
        onTray={goTray}
      />

      <UpdateBanner updateState={updateState} onCheck={checkNow} onDownload={download} onApply={apply} onDismiss={dismiss} onOpenRelease={openRelease} />

      <PopOutSummary
        total={sorted.length}
        errorCount={errorCount}
        isRefreshing={isRefreshing}
        lastRefresh={lastRefresh}
      />

      <div className={`popout-body ${selected ? "popout-body--split" : ""}`}>
        <div className="popout-list">
          {sorted.map((p) => (
            <ProviderCard
              key={p.providerId}
              provider={p}
              selected={selectedId === p.providerId}
              hideEmail={settings.hidePersonalInfo}
              resetRelative={settings.resetTimeRelative}
              onSelect={() => toggleSelect(p.providerId)}
            />
          ))}
        </div>

        {selected && (
          <div className="popout-detail">
            <ProviderDetail
              provider={selected}
              hideEmail={settings.hidePersonalInfo}
              resetRelative={settings.resetTimeRelative}
              onBack={handleBack}
            />
          </div>
        )}
      </div>
    </main>
  );
}

// ── Header ───────────────────────────────────────────────────────────

function PopOutHeader({
  onRefresh,
  isRefreshing,
  onSettings,
  onTray,
}: {
  onRefresh: () => void;
  isRefreshing: boolean;
  onSettings: () => void;
  onTray: () => void;
}) {
  return (
    <header className="popout-header">
      <h1 className="popout-header__title">CodexBar</h1>
      <div className="popout-header__actions">
        <button
          className="popout-action-btn"
          onClick={onRefresh}
          disabled={isRefreshing}
          title="Refresh"
          type="button"
        >
          <span className={isRefreshing ? "spin" : ""}>↻</span>
        </button>
        <button
          className="popout-action-btn"
          onClick={onSettings}
          title="Settings"
          type="button"
        >
          ⚙
        </button>
        <button
          className="popout-action-btn"
          onClick={onTray}
          title="Back to tray"
          type="button"
        >
          ⊟
        </button>
      </div>
    </header>
  );
}

// ── Summary strip ────────────────────────────────────────────────────

function PopOutSummary({
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
  } else if (lastRefresh && lastRefresh.errorCount > 0) {
    parts.push(`${lastRefresh.errorCount} failed`);
  }
  if (!isRefreshing && errorCount > 0) {
    parts.push(`${errorCount} with errors`);
  }

  return <div className="popout-summary">{parts.join(" · ")}</div>;
}
