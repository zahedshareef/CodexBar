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
import { getProviderIcon } from "../components/providers/providerIcons";
import { openProviderDashboard, openProviderStatusPage } from "../lib/tauri";
import { DEMO_ENABLED, DEMO_PROVIDERS } from "../lib/demoProviders";

/** Provider IDs that have a dashboard URL in the backend */
const HAS_DASHBOARD = new Set([
  "codex", "claude", "copilot", "cursor", "gemini", "antigravity",
  "factory", "augment", "kilo", "amp", "openrouter", "warp", "zai",
  "minimax", "kiro", "opencode",
]);
/** Provider IDs that have a status page URL in the backend */
const HAS_STATUS_PAGE = new Set([
  "codex", "claude", "copilot", "cursor", "gemini",
]);

function getProviderStatus(
  p: ProviderUsageSnapshot,
): "ok" | "warning" | "exhausted" | "error" {
  if (p.error) return "error";
  if (p.primary.isExhausted) return "exhausted";
  if (p.primary.usedPercent > 80) return "warning";
  return "ok";
}
void getProviderStatus;

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
  // Default to overview like macOS — shows all cards sorted by priority
  const [selectedProviderId, setSelectedProviderId] = useState<string | null>(
    null,
  );

  // Cards to display based on mode
  // Overview: all providers in the grid — non-error first, then errors
  // Detail: only the selected provider's card (macOS shows single provider)
  const visibleProviders = useMemo(() => {
    if (selectedProviderId === null) {
      // Overview: show all providers (they have data, email, or error), non-error first
      const normal = sorted.filter((p) => !p.error);
      const errors = sorted.filter((p) => !!p.error);
      return [...normal, ...errors];
    }
    // Detail: show ONLY the selected provider (macOS behavior — no appended errors)
    const match = sorted.find((p) => p.providerId === selectedProviderId);
    if (!match) {
      const normal = sorted.filter((p) => !p.error);
      const errors = sorted.filter((p) => !!p.error);
      return [...normal, ...errors];
    }
    return [match];
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

  // DEBUG: report computed widths
  useEffect(() => {
    const timer = setTimeout(() => {
      const sel = (s: string) => document.querySelector<HTMLElement>(s);
      const w = (el: HTMLElement | null) => el ? el.offsetWidth : -1;
      const surface = sel(".menu-surface--tray");
      const body = sel(".menu-surface__body");
      const stack = sel(".menu-stack");
      const item = sel(".menu-stack__item");
      const card = sel(".menu-card");
      const header = sel(".menu-card__header");
      const errBlock = sel(".menu-card__error-block");
      const errText = sel(".menu-card__error-text");
      const dbg = document.createElement("div");
      dbg.style.cssText = "position:fixed;top:0;left:0;background:yellow;color:black;font:8px monospace;z-index:99999;padding:2px;width:100%;box-sizing:border-box";
      dbg.textContent = `s:${w(surface)} b:${w(body)} sk:${w(stack)} it:${w(item)} c:${w(card)} h:${w(header)} eb:${w(errBlock)} et:${w(errText)}`;
      document.body.appendChild(dbg);
    }, 2000);
    return () => clearTimeout(timer);
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

  // Icon parity with macOS MenuDescriptor: only Refresh has an SF Symbol.
  // Settings / About / Quit render as plain text rows (no icon column).
  const footerRows: MenuFooterRow[] = [
    { icon: "↻", label: "Refresh", onClick: refresh },
    { icon: "", label: "Settings\u2026", onClick: openSettings },
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
          <span className="provider-grid__label">All</span>
        </button>
        {providers.map((p) => (
          <button
            key={p.providerId}
            type="button"
            className={`provider-grid__item${p.providerId === selectedProviderId ? " provider-grid__item--active" : ""}`}
            onClick={() => handleGridClick(p.providerId)}
            title={p.displayName}
          >
            <ProviderIcon providerId={p.providerId} size={16} />
            <span className="provider-grid__label">{p.displayName}</span>
            {/* Weekly indicator bar — matches macOS ProviderSwitcherView */}
            {!p.error && (
              <span
                className="provider-grid__weekly-track"
                style={{
                  "--weekly-pct": `${Math.max(0, Math.min(100, p.primary.remainingPercent))}%`,
                  "--weekly-color": getProviderIcon(p.providerId).brandColor,
                } as React.CSSProperties}
              />
            )}
          </button>
        ))}
      </div>
      <div className="provider-grid__divider" />
      <div className="menu-stack">
        {visibleProviders.map((p, idx) => {
          const isSelected =
            selectedProviderId !== null && p.providerId === selectedProviderId;
          return (
            <Fragment key={p.providerId}>
              {idx > 0 && <div className="menu-stack__sep" />}
              <div
                className={`menu-stack__item${isSelected ? " menu-stack__item--selected" : ""}`}
                id={`card-${p.providerId}`}
              >
                <MenuCard provider={p} hideEmail={settings.hidePersonalInfo} />
              </div>
            </Fragment>
          );
        })}
      </div>
      {/* Context actions — detail mode only, matches macOS actionsSection */}
      {selectedProviderId && (HAS_DASHBOARD.has(selectedProviderId) || HAS_STATUS_PAGE.has(selectedProviderId)) && (
        <div className="context-actions">
          <div className="context-actions__divider" />
          {HAS_DASHBOARD.has(selectedProviderId) && (
            <button
              type="button"
              className="context-actions__btn"
              onClick={() => void openProviderDashboard(selectedProviderId)}
            >
              <span className="context-actions__icon" aria-hidden>
                <svg width="13" height="13" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
                  <rect x="2" y="9" width="2.5" height="5" rx="0.6" fill="currentColor" />
                  <rect x="6.75" y="6" width="2.5" height="8" rx="0.6" fill="currentColor" />
                  <rect x="11.5" y="3" width="2.5" height="11" rx="0.6" fill="currentColor" />
                </svg>
              </span>
              Usage Dashboard
            </button>
          )}
          {HAS_STATUS_PAGE.has(selectedProviderId) && (
            <button
              type="button"
              className="context-actions__btn"
              onClick={() => void openProviderStatusPage(selectedProviderId)}
            >
              <span className="context-actions__icon" aria-hidden>
                <svg width="14" height="13" viewBox="0 0 18 14" fill="none" xmlns="http://www.w3.org/2000/svg">
                  <path d="M1 7H4L5.5 3L8 11L10.5 5L12 7H17" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" fill="none" />
                </svg>
              </span>
              Status Page
            </button>
          )}
        </div>
      )}
    </MenuSurface>
  );
}
