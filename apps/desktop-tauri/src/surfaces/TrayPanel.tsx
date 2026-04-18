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

function getProviderStatus(
  p: ProviderUsageSnapshot,
): "ok" | "warning" | "exhausted" | "error" {
  if (p.error) return "error";
  if (p.primary.isExhausted) return "exhausted";
  if (p.primary.usedPercent > 80) return "warning";
  return "ok";
}

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
  const openProviders = useCallback(() => {
    setSurfaceMode("settings", { kind: "settings", tab: "providers" });
  }, []);
  const quitApp = useCallback(() => {
    void getCurrentWindow().close();
  }, []);

  const headerActions = [
    { icon: "⧉", title: t("TooltipPopOut"), onClick: openPopOut },
  ];

  const footerRows: MenuFooterRow[] = [
    { icon: "", label: "Settings…", onClick: openSettings },
    { icon: "", label: "About CodexBar", onClick: openAbout },
    { icon: "", label: "Quit", onClick: quitApp },
  ];

  // Provider-specific action rows (mirrors upstream MenuDescriptor)
  const actionRows = activeProvider ? [
    { icon: "🔑", label: "Add Account…", onClick: openProviders },
    { icon: "📊", label: "Usage Dashboard", onClick: openPopOut },
  ] : [];

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
              <ProviderIcon providerId={p.providerId} size={22} />
              <span className="provider-tabs__label">{p.displayName}</span>
              <span className="provider-tabs__dot" data-status={getProviderStatus(p)} />
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
      {actionRows.length > 0 && (
        <nav className="menu-actions" aria-label="Provider actions">
          {actionRows.map((row) => (
            <button
              key={row.label}
              type="button"
              className="menu-actions__row"
              onClick={row.onClick}
            >
              <span className="menu-actions__icon" aria-hidden>{row.icon}</span>
              <span>{row.label}</span>
            </button>
          ))}
        </nav>
      )}
    </MenuSurface>
  );
}
