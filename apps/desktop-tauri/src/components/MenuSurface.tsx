import type { ReactNode } from "react";
import { useLocale } from "../hooks/useLocale";

export interface MenuSurfaceAction {
  icon: string;
  title: string;
  onClick: () => void;
}

interface MenuSurfaceProps {
  variant: "tray" | "popout";
  onRefresh: () => void;
  isRefreshing: boolean;
  actions: MenuSurfaceAction[];
  summary?: ReactNode;
  banner?: ReactNode;
  children: ReactNode;
}

/**
 * Flush, compact container that both `TrayPanel` and `PopOutPanel` consume.
 *
 * Mirrors the upstream macOS `MenuContent`: a narrow VStack(spacing: 8)
 * inside an NSMenu-like popover (310pt wide, vertical 6 / horizontal 10
 * padding, no hero framing). The body holds a stack of full provider
 * cards (`MenuCard`) — one per enabled provider — exactly like upstream.
 */
export default function MenuSurface({
  variant,
  onRefresh,
  isRefreshing,
  actions,
  summary,
  banner,
  children,
}: MenuSurfaceProps) {
  const { t } = useLocale();
  return (
    <div className={`menu-surface menu-surface--${variant}`}>
      <header className="menu-surface__header">
        <span className="menu-surface__title">CodexBar</span>
        <div className="menu-surface__actions">
          <button
            className="menu-surface__btn"
            onClick={onRefresh}
            disabled={isRefreshing}
            title={t("TooltipRefresh")}
            type="button"
          >
            <span className={isRefreshing ? "spin" : ""}>↻</span>
          </button>
          {actions.map((action) => (
            <button
              key={action.title}
              className="menu-surface__btn"
              onClick={action.onClick}
              title={action.title}
              type="button"
            >
              {action.icon}
            </button>
          ))}
        </div>
      </header>
      {banner}
      {summary}
      <div className="menu-surface__body">{children}</div>
    </div>
  );
}

interface MenuSummaryProps {
  total: number;
  errorCount: number;
  isRefreshing: boolean;
  lastRefresh: { providerCount: number; errorCount: number } | null;
}

export function MenuSummary({
  total,
  errorCount,
  isRefreshing,
  lastRefresh,
}: MenuSummaryProps) {
  const { t } = useLocale();
  const parts: string[] = [`${total} ${t("SummaryProvidersLabel")}`];
  if (isRefreshing) {
    parts.push(t("SummaryRefreshing"));
  } else if (lastRefresh && lastRefresh.errorCount > 0) {
    parts.push(`${lastRefresh.errorCount} ${t("SummaryFailed")}`);
  }
  if (!isRefreshing && errorCount > 0) {
    parts.push(`${errorCount} ${t("SummaryWithErrors")}`);
  }
  return <div className="menu-surface__summary">{parts.join(" · ")}</div>;
}

interface MenuEmptyProps {
  isLoading: boolean;
  onSettings: () => void;
}

export function MenuEmpty({ isLoading, onSettings }: MenuEmptyProps) {
  const { t } = useLocale();

  if (isLoading) {
    return (
      <div className="menu-surface__empty">
        <div className="menu-surface__spinner" />
        <p>{t("FetchingProviderData")}</p>
      </div>
    );
  }

  return (
    <div className="menu-surface__empty">
      <p>{t("NoProvidersConfigured")}</p>
      <p className="menu-surface__hint">{t("EnableProvidersHint")}</p>
      <button
        className="menu-surface__primary-btn"
        onClick={onSettings}
        type="button"
      >
        {t("OpenSettingsButton")}
      </button>
    </div>
  );
}
