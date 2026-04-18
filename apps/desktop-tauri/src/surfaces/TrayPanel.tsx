import { useCallback, useMemo, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { BootstrapState, ProviderUsageSnapshot } from "../types/bridge";
import { setSurfaceMode } from "../lib/tauri";
import { useProviders } from "../hooks/useProviders";
import { useSettings } from "../hooks/useSettings";
import { useUpdateState } from "../hooks/useUpdateState";
import { useLocale } from "../hooks/useLocale";
import MenuCard from "../components/MenuCard";
import MenuSurface, {
  MenuEmpty,
  type MenuFooterRow,
} from "../components/MenuSurface";
import UpdateBanner from "../components/UpdateBanner";
import { ProviderIcon } from "../components/providers/ProviderIcon";

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
  const [selectedIdx, setSelectedIdx] = useState(0);
  const activeIdx = Math.min(selectedIdx, Math.max(0, sorted.length - 1));
  const activeProvider = sorted[activeIdx] ?? null;

  const openSettings = useCallback(() => {
    setSurfaceMode("settings", { kind: "settings", tab: "general" });
  }, []);
  const openPopOut = useCallback(() => {
    setSurfaceMode("popOut", { kind: "dashboard" });
  }, []);
  const openAbout = useCallback(() => {
    setSurfaceMode("settings", { kind: "settings", tab: "about" });
  }, []);
  const quitApp = useCallback(() => {
    void getCurrentWindow().close();
  }, []);

  const headerActions = [
    { icon: "⧉", title: t("TooltipPopOut"), onClick: openPopOut },
  ];

  const footerRows: MenuFooterRow[] = [
    { icon: "⚙", label: t("TooltipSettings"), onClick: openSettings },
    { icon: "ℹ", label: "About CodexBar", onClick: openAbout },
    { icon: "✕", label: "Quit", onClick: quitApp },
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
        footerRows={footerRows}
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
      footerRows={footerRows}
    >
      {sorted.length > 1 && (
        <div className="provider-tabs">
          {sorted.map((p, idx) => (
            <button
              key={p.providerId}
              type="button"
              className={`provider-tabs__tab${idx === activeIdx ? " provider-tabs__tab--active" : ""}`}
              onClick={() => setSelectedIdx(idx)}
              title={p.displayName}
            >
              <ProviderIcon providerId={p.providerId} size={20} />
            </button>
          ))}
        </div>
      )}
      {activeProvider && (
        <div className="menu-stack">
          <div className="menu-stack__item">
            <MenuCard
              provider={activeProvider}
              hideEmail={settings.hidePersonalInfo}
            />
          </div>
        </div>
      )}
    </MenuSurface>
  );
}
