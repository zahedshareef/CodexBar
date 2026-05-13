import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import type { BootstrapState, ProviderUsageSnapshot } from "../types/bridge";
import { openSettingsWindow, setSurfaceMode } from "../lib/tauri";
import { reanchorTrayPanel } from "../lib/tauri";
import { useProviders } from "../hooks/useProviders";
import { useSettings } from "../hooks/useSettings";
import { useUpdateState } from "../hooks/useUpdateState";
import { useFormattedResetTime } from "../hooks/useFormattedResetTime";
import MenuCard from "../components/MenuCard";
import MenuSurface, { MenuEmpty } from "../components/MenuSurface";
import UpdateBanner from "../components/UpdateBanner";
import { ProviderIcon } from "../components/providers/ProviderIcon";
import { getProviderIcon } from "../components/providers/providerIcons";
import { openProviderDashboard, openProviderStatusPage } from "../lib/tauri";
import { DEMO_ENABLED, DEMO_PROVIDERS } from "../lib/demoProviders";

/** Provider IDs that have a dashboard URL in the backend */
const HAS_DASHBOARD = new Set([
  "codex",
  "claude",
  "copilot",
  "cursor",
  "gemini",
  "antigravity",
  "factory",
  "augment",
  "kilo",
  "amp",
  "openrouter",
  "warp",
  "zai",
  "minimax",
  "kiro",
  "opencode",
]);

/** Provider IDs that have a status page URL in the backend */
const HAS_STATUS_PAGE = new Set([
  "codex",
  "claude",
  "copilot",
  "cursor",
  "gemini",
]);

type ProviderStatus = "ok" | "warning" | "exhausted" | "error";

function getProviderStatus(p: ProviderUsageSnapshot): ProviderStatus {
  if (p.error) return "error";
  if (p.primary.isExhausted) return "exhausted";
  if (p.primary.usedPercent >= 80) return "warning";
  return "ok";
}

function statusLabel(status: ProviderStatus): string {
  switch (status) {
    case "error":
      return "Needs attention";
    case "exhausted":
      return "Exhausted";
    case "warning":
      return "Running low";
    case "ok":
    default:
      return "Healthy";
  }
}

function clampPercent(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.min(100, Math.max(0, value));
}

function remainingPercent(provider: ProviderUsageSnapshot): number {
  return Math.round(clampPercent(provider.primary.remainingPercent));
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

export default function TrayPanel({ state }: { state: BootstrapState }) {
  const {
    providers: realProviders,
    isRefreshing,
    refresh,
    hasCachedData,
  } = useProviders();
  const providers = DEMO_ENABLED ? DEMO_PROVIDERS : realProviders;
  const { settings } = useSettings(state.settings);
  const { updateState, checkNow, download, apply, dismiss, openRelease } =
    useUpdateState();

  const sorted = useMemo(() => sortProviders(providers), [providers]);
  const [selectedProviderId, setSelectedProviderId] = useState<string | null>(
    null,
  );

  const activeProvider = useMemo(() => {
    if (sorted.length === 0) return null;
    if (selectedProviderId) {
      return sorted.find((p) => p.providerId === selectedProviderId) ?? sorted[0];
    }
    return sorted[0];
  }, [selectedProviderId, sorted]);

  const [layoutReady, setLayoutReady] = useState(false);
  const layoutReadyRef = useRef(false);
  const resizeRunRef = useRef(0);

  useEffect(() => {
    if (
      selectedProviderId &&
      !sorted.some((p) => p.providerId === selectedProviderId)
    ) {
      setSelectedProviderId(null);
    }
  }, [selectedProviderId, sorted]);

  // Dynamically size the Tauri window to fit content, capped for tray use.
  useEffect(() => {
    const TRAY_WIDTH = 408;
    const MAX_HEIGHT = 760;
    const MIN_HEIGHT = 260;

    const resize = async () => {
      const run = ++resizeRunRef.current;
      const win = getCurrentWindow();
      const surface = document.querySelector<HTMLElement>(".menu-surface--tray");
      if (!surface) return;

      const body = surface.querySelector<HTMLElement>(".menu-surface__body");
      const stack = surface.querySelector<HTMLElement>(".menu-stack");
      const previous = {
        htmlOverflow: document.documentElement.style.overflow,
        bodyOverflow: document.body.style.overflow,
        bodyMinHeight: document.body.style.minHeight,
        surfaceMaxHeight: surface.style.maxHeight,
        surfaceOverflow: surface.style.overflow,
        bodyInnerOverflow: body?.style.overflow,
        bodyFlex: body?.style.flex,
        stackOverflow: stack?.style.overflow,
      };
      let committedHeight = false;

      document.documentElement.style.overflow = "visible";
      document.body.style.overflow = "visible";
      document.body.style.minHeight = "0";
      surface.style.maxHeight = "none";
      surface.style.overflow = "visible";
      if (body) {
        body.style.overflow = "visible";
        body.style.flex = "0 0 auto";
      }
      if (stack) stack.style.overflow = "visible";

      try {
        if (!layoutReadyRef.current) {
          await win.setSize(new LogicalSize(TRAY_WIDTH, MAX_HEIGHT));
          for (let i = 0; i < 20; i++) {
            await new Promise<void>((r) => setTimeout(r, 50));
            if (document.documentElement.clientHeight >= MAX_HEIGHT - 20) break;
          }
        }

        await new Promise<void>((r) => requestAnimationFrame(() => r()));
        await new Promise<void>((r) => requestAnimationFrame(() => r()));

        if (run !== resizeRunRef.current) return;

        const surfaceRect = surface.getBoundingClientRect();
        let maxBottom = surfaceRect.bottom;
        for (const el of surface.querySelectorAll("*")) {
          const r = (el as HTMLElement).getBoundingClientRect();
          if (r.height > 0 && r.bottom > maxBottom) maxBottom = r.bottom;
        }

        const contentHeight = Math.ceil(maxBottom - surfaceRect.top) + 4;
        const height = Math.min(Math.max(contentHeight, MIN_HEIGHT), MAX_HEIGHT);

        surface.style.maxHeight = `${height}px`;
        committedHeight = true;

        await win.setSize(new LogicalSize(TRAY_WIDTH, height));
        await reanchorTrayPanel().catch(() => {});

        if (run === resizeRunRef.current) {
          layoutReadyRef.current = true;
          setLayoutReady(true);
        }
      } finally {
        if (!committedHeight) {
          surface.style.maxHeight = previous.surfaceMaxHeight;
        }
        surface.style.overflow = previous.surfaceOverflow;
        document.documentElement.style.overflow = previous.htmlOverflow;
        document.body.style.overflow = previous.bodyOverflow;
        document.body.style.minHeight = previous.bodyMinHeight;
        if (body) {
          body.style.overflow = previous.bodyInnerOverflow ?? "";
          body.style.flex = previous.bodyFlex ?? "";
        }
        if (stack) stack.style.overflow = previous.stackOverflow ?? "";
      }
    };

    const t0 = setTimeout(() => void resize(), layoutReadyRef.current ? 50 : 100);

    return () => {
      clearTimeout(t0);
    };
  }, [activeProvider, sorted, isRefreshing, hasCachedData]);

  const openSettings = useCallback(() => {
    void openSettingsWindow("general").finally(() => {
      void getCurrentWindow().close();
    });
  }, []);

  const openPopOut = useCallback(() => {
    setSurfaceMode("popOut", { kind: "dashboard" });
  }, []);

  const openAbout = useCallback(() => {
    void openSettingsWindow("about").finally(() => {
      void getCurrentWindow().close();
    });
  }, []);

  const closePanel = useCallback(() => {
    void getCurrentWindow().close();
  }, []);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (!e.ctrlKey || e.shiftKey || e.altKey || e.metaKey) return;
      switch (e.key.toLowerCase()) {
        case "r":
          e.preventDefault();
          refresh();
          break;
        case ",":
          e.preventDefault();
          openSettings();
          break;
        case "q":
          e.preventDefault();
          closePanel();
          break;
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [refresh, openSettings, closePanel]);

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

  const errorCount = sorted.filter((p) => !!p.error).length;
  const warningCount = sorted.filter(
    (p) => getProviderStatus(p) === "warning" || getProviderStatus(p) === "exhausted",
  ).length;

  const headerSubtitle = isRefreshing
    ? "Refreshing provider data"
    : sorted.length === 0
      ? "No providers configured"
      : [
          `${sorted.length} active`,
          errorCount > 0 ? `${errorCount} need attention` : null,
          warningCount > 0 ? `${warningCount} running low` : null,
        ]
          .filter(Boolean)
          .join(" · ");

  const activeProviderId = activeProvider?.providerId ?? null;

  return (
    <div
      className={`tray-panel-reveal${layoutReady ? " tray-panel-reveal--ready" : ""}`}
    >
      <MenuSurface
        variant="tray"
        onRefresh={refresh}
        isRefreshing={isRefreshing}
        actions={[]}
        banner={banner}
      >
        <div className="tray-modern">
          <TrayHeader
            subtitle={headerSubtitle}
            isRefreshing={isRefreshing}
            onRefresh={refresh}
            onPopOut={openPopOut}
            onSettings={openSettings}
            onClose={closePanel}
          />

          {sorted.length === 0 ? (
            <MenuEmpty
              isLoading={isRefreshing && !hasCachedData}
              onSettings={openSettings}
            />
          ) : (
            <>
              <section className="tray-provider-list" aria-label="Providers">
                {sorted.map((provider) => (
                  <ProviderSummaryRow
                    key={provider.providerId}
                    provider={provider}
                    selected={provider.providerId === activeProviderId}
                    resetTimeRelative={settings.resetTimeRelative}
                    onSelect={() => setSelectedProviderId(provider.providerId)}
                  />
                ))}
              </section>

              {activeProvider && (
                <section className="tray-modern-detail" aria-label="Provider details">
                  <div className="tray-modern-detail__bar">
                    <div>
                      <span className="tray-modern-detail__eyebrow">
                        Details
                      </span>
                      <span className="tray-modern-detail__title">
                        {activeProvider.displayName}
                      </span>
                    </div>
                    <ProviderActions providerId={activeProvider.providerId} />
                  </div>
                  <div className="menu-stack">
                    <div
                      className="menu-stack__item menu-stack__item--selected"
                      id={`card-${activeProvider.providerId}`}
                    >
                      <MenuCard
                        provider={activeProvider}
                        hideEmail={settings.hidePersonalInfo}
                        resetTimeRelative={settings.resetTimeRelative}
                      />
                    </div>
                  </div>
                </section>
              )}

              <div className="tray-modern-footer">
                <button type="button" onClick={openAbout}>
                  About
                </button>
                <button type="button" onClick={closePanel}>
                  Close
                </button>
              </div>
            </>
          )}
        </div>
      </MenuSurface>
    </div>
  );
}

function TrayHeader({
  subtitle,
  isRefreshing,
  onRefresh,
  onPopOut,
  onSettings,
  onClose,
}: {
  subtitle: string;
  isRefreshing: boolean;
  onRefresh: () => void;
  onPopOut: () => void;
  onSettings: () => void;
  onClose: () => void;
}) {
  return (
    <header className="tray-modern-header">
      <div className="tray-modern-brand">
        <span className="tray-modern-brand__mark" aria-hidden>
          <MeterIcon />
        </span>
        <span className="tray-modern-brand__text">
          <span className="tray-modern-brand__title">CodexBar</span>
          <span className="tray-modern-brand__subtitle">{subtitle}</span>
        </span>
      </div>
      <div className="tray-modern-actions" aria-label="Tray commands">
        <TrayIconButton
          title="Refresh (Ctrl+R)"
          onClick={onRefresh}
          disabled={isRefreshing}
        >
          <RefreshIcon spinning={isRefreshing} />
        </TrayIconButton>
        <TrayIconButton title="Pop out" onClick={onPopOut}>
          <PopOutIcon />
        </TrayIconButton>
        <TrayIconButton title="Settings (Ctrl+,)" onClick={onSettings}>
          <SettingsIcon />
        </TrayIconButton>
        <TrayIconButton title="Close (Ctrl+Q)" onClick={onClose}>
          <CloseIcon />
        </TrayIconButton>
      </div>
    </header>
  );
}

function ProviderSummaryRow({
  provider,
  selected,
  resetTimeRelative,
  onSelect,
}: {
  provider: ProviderUsageSnapshot;
  selected: boolean;
  resetTimeRelative: boolean;
  onSelect: () => void;
}) {
  const status = getProviderStatus(provider);
  const remaining = remainingPercent(provider);
  const brand = getProviderIcon(provider.providerId).brandColor;
  const resetText = useFormattedResetTime(
    provider.primary.resetsAt,
    provider.primary.resetDescription,
    resetTimeRelative,
  );
  const primaryLabel = provider.primaryLabel ?? "Usage";
  const metricText = provider.error ? "Fix" : `${remaining}% left`;
  const meta = provider.error
    ? provider.error
    : [primaryLabel, resetText].filter(Boolean).join(" · ");

  const style = {
    "--provider-brand": brand,
    "--remaining-pct": provider.error ? "0%" : `${remaining}%`,
  } as CSSProperties;

  return (
    <button
      type="button"
      className={`tray-provider-row${selected ? " tray-provider-row--selected" : ""}`}
      data-status={status}
      onClick={onSelect}
      style={style}
      aria-pressed={selected}
    >
      <span className="tray-provider-row__accent" aria-hidden />
      <ProviderIcon providerId={provider.providerId} size={28} />
      <span className="tray-provider-row__main">
        <span className="tray-provider-row__top">
          <span className="tray-provider-row__name">{provider.displayName}</span>
          {provider.planName && (
            <span className="tray-provider-row__plan">{provider.planName}</span>
          )}
        </span>
        <span className="tray-provider-row__meta">{meta}</span>
        <span className="tray-provider-row__track" aria-hidden>
          <span className="tray-provider-row__fill" />
        </span>
      </span>
      <span className="tray-provider-row__side">
        <span className="tray-provider-row__metric">{metricText}</span>
        <span className="tray-status-chip" data-status={status}>
          {statusLabel(status)}
        </span>
      </span>
    </button>
  );
}

function ProviderActions({ providerId }: { providerId: string }) {
  const canDashboard = HAS_DASHBOARD.has(providerId);
  const canStatus = HAS_STATUS_PAGE.has(providerId);

  if (!canDashboard && !canStatus) return null;

  return (
    <div className="tray-provider-actions">
      {canDashboard && (
        <button
          type="button"
          className="tray-command-button"
          onClick={() => void openProviderDashboard(providerId)}
        >
          <DashboardIcon />
          Dashboard
        </button>
      )}
      {canStatus && (
        <button
          type="button"
          className="tray-command-button"
          onClick={() => void openProviderStatusPage(providerId)}
        >
          <PulseIcon />
          Status
        </button>
      )}
    </div>
  );
}

function TrayIconButton({
  title,
  disabled,
  onClick,
  children,
}: {
  title: string;
  disabled?: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      className="tray-icon-button"
      title={title}
      aria-label={title}
      disabled={disabled}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function SvgIcon({ children }: { children: ReactNode }) {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.55"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      {children}
    </svg>
  );
}

function MeterIcon() {
  return (
    <SvgIcon>
      <path d="M2.5 12.5h11" />
      <path d="M4 10V6.5" />
      <path d="M8 10V3.5" />
      <path d="M12 10V7.5" />
    </SvgIcon>
  );
}

function RefreshIcon({ spinning }: { spinning?: boolean }) {
  return (
    <span className={spinning ? "spin" : undefined}>
      <SvgIcon>
        <path d="M13 5.2A5.4 5.4 0 0 0 3.7 3.7L2.5 5" />
        <path d="M2.5 2.2V5h2.8" />
        <path d="M3 10.8a5.4 5.4 0 0 0 9.3 1.5l1.2-1.3" />
        <path d="M13.5 13.8V11h-2.8" />
      </SvgIcon>
    </span>
  );
}

function PopOutIcon() {
  return (
    <SvgIcon>
      <rect x="3" y="5" width="8" height="8" rx="1.4" />
      <path d="M8.5 3H13v4.5" />
      <path d="M8.5 7.5 13 3" />
    </SvgIcon>
  );
}

function SettingsIcon() {
  return (
    <SvgIcon>
      <circle cx="8" cy="8" r="2.2" />
      <path d="M8 1.8v1.5M8 12.7v1.5M2.6 4.1l1.1.9M12.3 11l1.1.9M1.8 8h1.5M12.7 8h1.5M2.6 11.9l1.1-.9M12.3 5l1.1-.9" />
    </SvgIcon>
  );
}

function CloseIcon() {
  return (
    <SvgIcon>
      <path d="M4.5 4.5 11.5 11.5" />
      <path d="M11.5 4.5 4.5 11.5" />
    </SvgIcon>
  );
}

function DashboardIcon() {
  return (
    <SvgIcon>
      <path d="M3 13V8" />
      <path d="M8 13V4" />
      <path d="M13 13V6.5" />
      <path d="M2 13.5h12" />
    </SvgIcon>
  );
}

function PulseIcon() {
  return (
    <SvgIcon>
      <path d="M1.5 8h3l1.4-4 3 8 1.6-4h4" />
    </SvgIcon>
  );
}
