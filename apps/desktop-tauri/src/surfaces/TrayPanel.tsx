import { useCallback, useMemo, useState } from "react";
import type { BootstrapState, ProviderUsageSnapshot } from "../types/bridge";
import { setSurfaceMode } from "../lib/tauri";
import { useProviders } from "../hooks/useProviders";
import { useSettings } from "../hooks/useSettings";
import { useUpdateState } from "../hooks/useUpdateState";
import { useLocale } from "../hooks/useLocale";
import ProviderRow from "../components/ProviderRow";
import MenuCard from "../components/MenuCard";
import MenuSurface, {
  MenuSummary,
  MenuEmpty,
} from "../components/MenuSurface";
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

  if (selected) {
    return (
      <MenuSurface
        variant="tray"
        onRefresh={refresh}
        isRefreshing={isRefreshing}
        actions={headerActions}
        banner={banner}
      >
        <MenuCard
          provider={selected}
          hideEmail={settings.hidePersonalInfo}
          onBack={handleBack}
        />
      </MenuSurface>
    );
  }

  if (sorted.length === 0) {
    return (
      <MenuSurface
        variant="tray"
        onRefresh={refresh}
        isRefreshing={isRefreshing}
        actions={headerActions}
        banner={banner}
      >
        <MenuEmpty isLoading={isRefreshing} onSettings={openSettings} />
      </MenuSurface>
    );
  }

  return (
    <MenuSurface
      variant="tray"
      onRefresh={refresh}
      isRefreshing={isRefreshing}
      actions={headerActions}
      banner={banner}
      summary={
        <MenuSummary
          total={sorted.length}
          errorCount={errorCount}
          isRefreshing={isRefreshing}
          lastRefresh={lastRefresh}
        />
      }
    >
      <div className="menu-list">
        {sorted.map((p) => (
          <ProviderRow
            key={p.providerId}
            provider={p}
            selected={selectedId === p.providerId}
            hideEmail={settings.hidePersonalInfo}
            resetRelative={settings.resetTimeRelative}
            onSelect={() => setSelectedId(p.providerId)}
          />
        ))}
      </div>
    </MenuSurface>
  );
}
