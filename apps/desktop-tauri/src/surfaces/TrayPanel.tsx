import { useCallback, useMemo } from "react";
import type { BootstrapState, ProviderUsageSnapshot } from "../types/bridge";
import { setSurfaceMode } from "../lib/tauri";
import { useProviders } from "../hooks/useProviders";
import { useSettings } from "../hooks/useSettings";
import { useUpdateState } from "../hooks/useUpdateState";
import { useLocale } from "../hooks/useLocale";
import MenuCard from "../components/MenuCard";
import MenuSurface, { MenuSummary, MenuEmpty } from "../components/MenuSurface";
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

/**
 * Tray popover surface — a vertical stack of full provider cards
 * mirroring the upstream macOS `MenuContent` (which renders one
 * `UsageMenuCardView` per enabled provider). No drill-in: every
 * provider's full metrics, cost, pace, and charts are visible at once,
 * separated by a 1pt divider, exactly like upstream.
 */
export default function TrayPanel({ state }: { state: BootstrapState }) {
  const { providers, isRefreshing, refresh, lastRefresh } = useProviders();
  const { settings } = useSettings(state.settings);
  const { updateState, checkNow, download, apply, dismiss, openRelease } =
    useUpdateState();
  const { t } = useLocale();

  const sorted = useMemo(() => sortProviders(providers), [providers]);
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
      <div className="menu-stack">
        {sorted.map((p, idx) => (
          <div key={p.providerId} className="menu-stack__item">
            {idx > 0 && <div className="menu-stack__sep" />}
            <MenuCard provider={p} hideEmail={settings.hidePersonalInfo} />
          </div>
        ))}
      </div>
    </MenuSurface>
  );
}
