import { Fragment, useCallback, useEffect, useMemo, useState } from "react";
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
import { DEMO_ENABLED, DEMO_PROVIDERS } from "../lib/demoProviders";

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
 * Tray popover surface — two modes like macOS CodexBar:
 * 1. Overview (default): provider grid + all cards stacked
 * 2. Detail: click a provider in grid → show only that provider's card
 */
export default function TrayPanel({ state }: { state: BootstrapState }) {
  const { providers: realProviders, isRefreshing, refresh } = useProviders();
  const providers = DEMO_ENABLED ? DEMO_PROVIDERS : realProviders;
  const { settings } = useSettings(state.settings);
  const { updateState, checkNow, download, apply, dismiss, openRelease } =
    useUpdateState();
  const { t } = useLocale();

  const sorted = useMemo(() => sortProviders(providers), [providers]);

  // null = overview (all providers), string = single provider detail
  // Default to first provider (highest usage) like macOS
  const [selectedProviderId, setSelectedProviderId] = useState<string | null>(
    () => {
      const s = sortProviders(providers);
      // Pick first non-error provider
      const first = s.find((p) => !p.error);
      return first ? first.providerId : null;
    },
  );

  // Cards to display based on mode
  // Detail mode: selected provider card + error cards (like macOS)
  const visibleProviders = useMemo(() => {
    if (selectedProviderId === null) return sorted;
    const match = sorted.find((p) => p.providerId === selectedProviderId);
    if (!match) return sorted;
    const errors = sorted.filter((p) => p.error && p.providerId !== selectedProviderId);
    return [match, ...errors];
  }, [sorted, selectedProviderId]);

  // Dynamically size the Tauri window to fit content, capped at 800px.
  useEffect(() => {
    const TRAY_WIDTH = 310;
    const MAX_HEIGHT = 800;
    // Temporarily unconstrain height to measure natural content height
    const root = document.documentElement;
    const surface = root.querySelector<HTMLElement>(".menu-surface--tray");
    if (surface) {
      const prev = surface.style.height;
      surface.style.height = "auto";
      requestAnimationFrame(() => {
        const contentHeight = surface.scrollHeight;
        surface.style.height = prev || "";
        const height = Math.min(Math.max(contentHeight, 200), MAX_HEIGHT);
        void getCurrentWindow().setSize(new LogicalSize(TRAY_WIDTH, height));
      });
    } else {
      void getCurrentWindow().setSize(new LogicalSize(TRAY_WIDTH, MAX_HEIGHT));
    }
  }, [visibleProviders]);

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

  const handleGridClick = useCallback(
    (providerId: string | null) => {
      setSelectedProviderId(providerId);
      // In overview mode, scroll to card if clicking a specific provider
      if (providerId !== null) {
        const el = document.getElementById(`card-${providerId}`);
        if (el) {
          el.scrollIntoView({ behavior: "smooth", block: "start" });
        }
      }
    },
    [],
  );

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
      <div className="provider-grid">
        {/* Overview button — first item like macOS */}
        <button
          type="button"
          className={`provider-grid__item${selectedProviderId === null ? " provider-grid__item--active" : ""}`}
          onClick={() => handleGridClick(null)}
          title="Overview"
        >
          <span className="provider-grid__icon-overview">⊞</span>
          <span className="provider-grid__label">Over…</span>
        </button>
        {sorted.map((p) => (
          <button
            key={p.providerId}
            type="button"
            className={`provider-grid__item${p.providerId === selectedProviderId ? " provider-grid__item--active" : ""}`}
            onClick={() => handleGridClick(p.providerId)}
            title={p.displayName}
          >
            <ProviderIcon providerId={p.providerId} size={16} />
            <span className="provider-grid__label">{p.displayName}</span>
            <span
              className="provider-grid__dot"
              data-status={getProviderStatus(p)}
            />
          </button>
        ))}
      </div>
      <div className="provider-grid__divider" />
      <div className="menu-stack">
        {visibleProviders.map((p, idx) => (
          <Fragment key={p.providerId}>
            {idx > 0 && <div className="menu-stack__sep" />}
            <div
              className={`menu-stack__item${idx === 0 ? " menu-stack__item--selected" : ""}`}
              id={`card-${p.providerId}`}
            >
              <MenuCard provider={p} hideEmail={settings.hidePersonalInfo} />
            </div>
          </Fragment>
        ))}
      </div>
    </MenuSurface>
  );
}
