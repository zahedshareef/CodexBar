import { useCallback, useMemo, useState } from "react";
import type { BootstrapState, ProviderUsageSnapshot } from "../types/bridge";
import { setSurfaceMode } from "../lib/tauri";
import { useProviders } from "../hooks/useProviders";
import { useSettings } from "../hooks/useSettings";
import { useUpdateState } from "../hooks/useUpdateState";
import { useLocale } from "../hooks/useLocale";
import ProviderCard from "./tray/ProviderCard";
import ProviderDetail from "./tray/ProviderDetail";
import UpdateBanner from "../components/UpdateBanner";
import SurfaceHeader from "./shared/SurfaceHeader";
import SurfaceSummary from "./shared/SurfaceSummary";
import SurfaceEmpty from "./shared/SurfaceEmpty";

/** Sort: highest primary used% first, then alphabetical by name. */
function sortProviders(list: ProviderUsageSnapshot[]): ProviderUsageSnapshot[] {
  return [...list].sort((a, b) => {
    const diff = b.primary.usedPercent - a.primary.usedPercent;
    if (Math.abs(diff) > 0.01) return diff;
    return a.displayName.localeCompare(b.displayName);
  });
}

export default function TrayPanel({ state }: { state: BootstrapState }) {
  const { providers, isRefreshing, refresh, lastRefresh } = useProviders();
  const { settings } = useSettings(state.settings);
  const { updateState, checkNow, download, apply, dismiss, openRelease } =
    useUpdateState();
  const { t } = useLocale();
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
  const handleBack = useCallback(() => setSelectedId(null), []);

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

  const headerActions = [
    { icon: "⚙", title: t("TooltipSettings"), onClick: openSettings },
    { icon: "⧉", title: t("TooltipPopOut"), onClick: openPopOut },
  ];

  const banner = (
    <UpdateBanner
      updateState={updateState}
      onCheck={checkNow}
      onDownload={download}
      onApply={apply}
      onDismiss={dismiss}
      onOpenRelease={openRelease}
    />
  );

  if (sorted.length === 0) {
    return (
      <main className="shell shell--tray-panel">
        <SurfaceHeader
          onRefresh={refresh}
          isRefreshing={isRefreshing}
          actions={headerActions}
        />
        {banner}
        <SurfaceEmpty isLoading={isRefreshing} onSettings={openSettings} />
      </main>
    );
  }

  return (
    <main className="shell shell--tray-panel">
      <SurfaceHeader
        onRefresh={refresh}
        isRefreshing={isRefreshing}
        actions={headerActions}
      />
      {banner}
      <SurfaceSummary
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
