import { Fragment, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
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
 * Tray popover surface — provider icon grid at top, with all provider
 * cards stacked vertically below. Mirrors the upstream macOS CodexBar
 * popover: the grid is for quick scanning + scrolling, while every
 * provider's full card is rendered in sequence separated by dividers.
 */
export default function TrayPanel({ state }: { state: BootstrapState }) {
  const { providers, isRefreshing, refresh } = useProviders();
  const { settings } = useSettings(state.settings);
  const { updateState, checkNow, download, apply, dismiss, openRelease } =
    useUpdateState();
  const { t } = useLocale();

  const sorted = useMemo(() => sortProviders(providers), [providers]);
  const [selectedIdx, setSelectedIdx] = useState(0);
  const activeIdx = Math.min(selectedIdx, Math.max(0, sorted.length - 1));
  const surfaceRef = useRef<HTMLDivElement>(null);

  // Auto-resize the Tauri window to fit content (max 660px like macOS)
  useEffect(() => {
    const el = surfaceRef.current;
    if (!el) return;
    const TRAY_WIDTH = 300;
    const MAX_HEIGHT = 660;
    const frame = requestAnimationFrame(() => {
      const contentHeight = Math.min(MAX_HEIGHT, Math.max(180, el.scrollHeight));
      void getCurrentWindow().setSize(new LogicalSize(TRAY_WIDTH, contentHeight));
    });
    return () => cancelAnimationFrame(frame);
  }, [sorted, isRefreshing]);

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
    { icon: "↻", label: "Refresh", onClick: refresh },
    { icon: "", label: "Settings…", onClick: openSettings },
    { icon: "", label: "About CodexBar", onClick: openAbout },
    { icon: "", label: "Quit", onClick: quitApp },
  ];

  const handleGridClick = useCallback((idx: number, providerId: string) => {
    setSelectedIdx(idx);
    const el = document.getElementById(`card-${providerId}`);
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "start" });
    }
  }, []);

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
      <div ref={surfaceRef}>
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
      </div>
    );
  }

  return (
    <div ref={surfaceRef}>
      <MenuSurface
        variant="tray"
        onRefresh={refresh}
        isRefreshing={isRefreshing}
        actions={headerActions}
        banner={banner}
        footerRows={footerRows}
      >
      <div className="provider-grid">
        {sorted.map((p, idx) => (
          <button
            key={p.providerId}
            type="button"
            className={`provider-grid__item${idx === activeIdx ? " provider-grid__item--active" : ""}`}
            onClick={() => handleGridClick(idx, p.providerId)}
            title={p.displayName}
          >
            <ProviderIcon providerId={p.providerId} size={32} />
            <span className="provider-grid__label">{p.displayName}</span>
            <span
              className="provider-grid__dot"
              data-status={getProviderStatus(p)}
            />
          </button>
        ))}
      </div>
      <div className="menu-stack">
        {sorted.map((p, idx) => (
          <Fragment key={p.providerId}>
            {idx > 0 && <div className="menu-stack__sep" />}
            <div className="menu-stack__item" id={`card-${p.providerId}`}>
              <MenuCard provider={p} hideEmail={settings.hidePersonalInfo} />
            </div>
          </Fragment>
        ))}
      </div>
      </MenuSurface>
    </div>
  );
}
