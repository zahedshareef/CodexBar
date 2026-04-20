import { useCallback, useEffect, useState } from "react";
import type {
  PaceSnapshot,
  ProviderChartData,
  ProviderUsageSnapshot,
  RateWindowSnapshot,
} from "../types/bridge";
import { getProviderChartData } from "../lib/tauri";
import { useLocale } from "../hooks/useLocale";
import type { LocaleKey } from "../i18n/keys";
import { paceCategory } from "../surfaces/tray/paceCategory";
import { SimpleBarChart, StackedBarChart } from "./MiniBarChart";
import { DEMO_ENABLED } from "../lib/demoProviders";

/** Small copy-to-clipboard button matching macOS CopyIconButton (doc.on.doc → checkmark). */
function CopyIconButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(text).catch(() => {});
    setCopied(true);
    setTimeout(() => setCopied(false), 900);
  }, [text]);
  return (
    <button
      type="button"
      className="menu-card__copy-btn"
      onClick={handleCopy}
      aria-label={copied ? "Copied" : "Copy error"}
      title={copied ? "Copied" : "Copy error"}
    >
      {copied ? "✓" : (
        <svg width="12" height="12" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
          <rect x="5" y="5" width="9" height="9" rx="1.5" stroke="currentColor" strokeWidth="1.5"/>
          <path d="M11 3V2.5A1.5 1.5 0 009.5 1H2.5A1.5 1.5 0 001 2.5v7A1.5 1.5 0 002.5 11H3" stroke="currentColor" strokeWidth="1.5"/>
        </svg>
      )}
    </button>
  );
}

interface MenuCardProps {
  provider: ProviderUsageSnapshot;
  hideEmail: boolean;
}

function maskEmail(email: string): string {
  const at = email.indexOf("@");
  if (at <= 1) return "••••@••••";
  return email[0] + "•".repeat(at - 1) + email.slice(at);
}

function formatCurrency(amount: number, code: string): string {
  try {
    return new Intl.NumberFormat("en-US", {
      style: "currency",
      currency: code,
    }).format(amount);
  } catch {
    return `${code} ${amount.toFixed(2)}`;
  }
}

/**
 * Format a backend `updatedAt` timestamp as a short relative string
 * ("just now", "2m ago", "3h ago", "5d ago"). If the value isn't a parseable
 * ISO datetime, return it unchanged so manual / preformatted strings still
 * render verbatim.
 */
function formatRelative(updatedAt: string): string {
  const ts = Date.parse(updatedAt);
  if (Number.isNaN(ts)) return updatedAt;
  const diffSec = Math.max(0, Math.round((Date.now() - ts) / 1000));
  if (diffSec < 60) return "just now";
  const diffMin = Math.round(diffSec / 60);
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHr = Math.round(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;
  const diffDay = Math.round(diffHr / 24);
  return `${diffDay}d ago`;
}

function paceStageKey(stage: PaceSnapshot["stage"]): LocaleKey {
  switch (stage) {
    case "on_track":
      return "DetailPaceOnTrack";
    case "slightly_ahead":
      return "DetailPaceSlightlyAhead";
    case "ahead":
      return "DetailPaceAhead";
    case "far_ahead":
      return "DetailPaceFarAhead";
    case "slightly_behind":
      return "DetailPaceSlightlyBehind";
    case "behind":
      return "DetailPaceBehind";
    case "far_behind":
      return "DetailPaceFarBehind";
    default:
      return "DetailPaceOnTrack";
  }
}

type UsageLevel = "normal" | "high" | "critical" | "exhausted";
function levelOf(remainPct: number, exhausted: boolean): UsageLevel {
  if (exhausted) return "exhausted";
  if (remainPct <= 5) return "critical";
  if (remainPct <= 25) return "high";
  return "normal";
}

interface MetricEntry {
  label: string;
  snap: RateWindowSnapshot;
}

/**
 * Single metric row inside the card — mirrors upstream `MetricRow`:
 *   • title (body / medium)
 *   • UsageProgressBar (capsule, 6pt)
 *   • HStack: "N% used"  ··  reset countdown (right-aligned, secondary)
 */
function MetricRow({
  title,
  snap,
  exhaustedLabel,
}: {
  title: string;
  snap: RateWindowSnapshot;
  exhaustedLabel: string;
}) {
  const pct = Math.min(100, Math.max(0, snap.usedPercent));
  const remain = 100 - pct;
  const level = levelOf(remain, snap.isExhausted);
  return (
    <div className="menu-metric">
      <span className="menu-metric__title">{title}</span>
      <div className="menu-metric__bar">
        <div className="menu-metric__bar-fill" data-level={level} style={{ width: `${remain}%` }} />
      </div>
      <div className="menu-metric__row">
        <span className="menu-metric__pct">{Math.round(100 - pct)}% left</span>
        {snap.resetDescription && (
          <span className="menu-metric__reset">{snap.resetDescription}</span>
        )}
      </div>
      {snap.isExhausted && (
        <div className="menu-metric__exhausted">{exhaustedLabel}</div>
      )}
      {snap.reservePercent != null && (
        <div className="menu-metric__row menu-metric__reserve">
          <span className="menu-metric__pct">{Math.round(snap.reservePercent)}% in reserve</span>
          {snap.reserveDescription && (
            <span className="menu-metric__reset">{snap.reserveDescription}</span>
          )}
        </div>
      )}
    </div>
  );
}

/**
 * Provider card — direct mirror of SwiftUI `UsageMenuCardView`.
 *
 * Layout (top to bottom):
 *   1. Header VStack(spacing: 3)
 *        – HStack: providerName (headline/semibold)  ··  email (subheadline/secondary, right)
 *        – HStack: subtitle "source · updated"        ··  plan (footnote/secondary, right)
 *   2. Divider (1pt)
 *   3. VStack(spacing: 12)
 *        – Metrics group VStack(spacing: 12) of MetricRow
 *        – (Divider) Cost group: title (body/medium) + session line + month line (footnote)
 *        – (Divider) Pace group (Tauri-only addition; placed last)
 *        – (Divider) Charts group (Tauri-only addition; placed last)
 *
 * Padding: horizontal 16, vertical 2 (matches upstream UsageMenuCardView).
 */
export default function MenuCard({ provider, hideEmail }: MenuCardProps) {
  const { t } = useLocale();
  const [chartData, setChartData] = useState<ProviderChartData | null>(null);

  useEffect(() => {
    if (DEMO_ENABLED) return; // skip chart fetch in demo mode
    let cancelled = false;
    setChartData(null);
    getProviderChartData(
      provider.providerId,
      provider.accountEmail ?? undefined,
    )
      .then((data) => {
        if (!cancelled) setChartData(data);
      })
      .catch(() => {
        /* chart data is best-effort */
      });
    return () => {
      cancelled = true;
    };
  }, [provider.providerId]);

  const email = provider.accountEmail
    ? hideEmail
      ? maskEmail(provider.accountEmail)
      : provider.accountEmail
    : null;

  const metrics: MetricEntry[] = [
    { label: provider.primaryLabel ?? t("DetailWindowPrimary"), snap: provider.primary },
  ];
  if (provider.secondary)
    metrics.push({ label: provider.secondaryLabel ?? t("DetailWindowSecondary"), snap: provider.secondary });
  if (provider.modelSpecific)
    metrics.push({
      label: t("DetailWindowModelSpecific"),
      snap: provider.modelSpecific,
    });
  if (provider.tertiary)
    metrics.push({ label: t("DetailWindowTertiary"), snap: provider.tertiary });

  const hasCostHistory = chartData !== null && chartData.costHistory.length > 0;
  const hasCreditsHistory =
    chartData !== null && chartData.creditsHistory.length > 0;
  const hasUsageBreakdown =
    chartData !== null && chartData.usageBreakdown.length > 0;
  const hasCharts = hasCostHistory || hasCreditsHistory || hasUsageBreakdown;
  const hasMetrics = metrics.length > 0;
  const hasCost = !!provider.cost;
  const hasPace = !!provider.pace;
  const hasDetails = !provider.error && (hasMetrics || hasCost || hasPace || hasCharts);

  return (
    <article className={`menu-card${provider.error ? " menu-card--error" : ""}`}>
      <header className="menu-card__header">
        <div className="menu-card__title-row">
          <div className="menu-card__name-group">
            <span className="menu-card__name">{provider.displayName}</span>
            {!provider.error && email && <span className="menu-card__email">{email}</span>}
          </div>
        </div>
        {provider.error ? (
          <div className="menu-card__subtitle-row">
            <span className="menu-card__subtitle menu-card__subtitle--error">
              {provider.error}
            </span>
            <CopyIconButton text={provider.error} />
          </div>
        ) : (
          <div className="menu-card__subtitle-row">
            <span className="menu-card__subtitle">
              {t("DetailUpdatedPrefix")} {formatRelative(provider.updatedAt)}
            </span>
            {provider.planName && (
              <span className="menu-card__plan-badge">{provider.planName}</span>
            )}
          </div>
        )}
      </header>

      {hasDetails && <div className="menu-card__divider" />}

      {hasDetails && (
        <div className="menu-card__content">
          {hasMetrics && (
            <section className="menu-card__group menu-card__metrics">
              {metrics.map((m) => (
                <MetricRow
                  key={m.label}
                  title={m.label}
                  snap={m.snap}
                  exhaustedLabel={t("DetailWindowExhausted")}
                />
              ))}
            </section>
          )}

          {hasMetrics && hasCost && <div className="menu-card__divider" />}

          {provider.cost && (
            <section className="menu-card__group menu-card__cost">
              <div className="menu-card__group-title">
                {t("DetailCostTitle")} — {provider.cost.period}
              </div>
              <div className="menu-card__cost-line">
                {t("DetailCostUsed")}:{" "}
                {provider.cost.formattedUsed ||
                  formatCurrency(provider.cost.used, provider.cost.currencyCode)}
                {provider.cost.limit != null && (
                  <>
                    {" / "}
                    {provider.cost.formattedLimit ||
                      formatCurrency(provider.cost.limit, provider.cost.currencyCode)}
                  </>
                )}
              </div>
              {provider.cost.remaining != null && (
                <div className="menu-card__cost-line menu-card__cost-line--muted">
                  {t("DetailCostRemaining")}:{" "}
                  {formatCurrency(provider.cost.remaining, provider.cost.currencyCode)}
                </div>
              )}
              {provider.cost.resetsAt && (
                <div className="menu-card__cost-line menu-card__cost-line--muted">
                  {t("DetailCostResets")}: {provider.cost.resetsAt}
                </div>
              )}
            </section>
          )}

          {(hasMetrics || hasCost) && hasPace && <div className="menu-card__divider" />}

          {provider.pace && (
            <section className="menu-card__group menu-card__pace">
              <div className="menu-card__pace-header">
                <span className="menu-card__group-title">{t("DetailPaceTitle")}</span>
                <span
                  className="menu-card__pace-label"
                  data-pace={paceCategory(provider.pace.stage)}
                >
                  {t(paceStageKey(provider.pace.stage))} (
                  {provider.pace.deltaPercent >= 0 ? "+" : ""}
                  {provider.pace.deltaPercent.toFixed(1)}%)
                </span>
              </div>
              <div className="menu-card__pace-bars">
                <div className="menu-card__pace-track" title="Expected">
                  <div
                    className="menu-card__pace-fill menu-card__pace-fill--expected"
                    style={{ width: `${provider.pace.expectedUsedPercent.toFixed(1)}%` }}
                  />
                </div>
                <div className="menu-card__pace-track" title="Actual">
                  <div
                    className="menu-card__pace-fill"
                    data-pace={paceCategory(provider.pace.stage)}
                    style={{ width: `${provider.pace.actualUsedPercent.toFixed(1)}%` }}
                  />
                </div>
              </div>
              {provider.pace.etaSeconds != null && !provider.pace.willLastToReset && (
                <div className="menu-card__pace-eta">
                  ⚠{" "}
                  {t("DetailPaceRunsOutIn").replace(
                    "{}",
                    String(Math.round(provider.pace.etaSeconds / 3600)),
                  )}
                </div>
              )}
              {provider.pace.willLastToReset && (
                <div className="menu-card__pace-ok">
                  ✓ {t("DetailPaceWillLastToReset")}
                </div>
              )}
            </section>
          )}

          {(hasMetrics || hasCost || hasPace) && hasCharts && (
            <div className="menu-card__divider" />
          )}

          {hasCharts && (
            <section className="menu-card__group menu-card__charts">
              {hasCostHistory && (
                <SimpleBarChart
                  points={chartData!.costHistory}
                  label={t("DetailChartCost")}
                  color="var(--accent)"
                  formatValue={(v) => `$${v.toFixed(2)}`}
                />
              )}
              {hasCreditsHistory && (
                <SimpleBarChart
                  points={chartData!.creditsHistory}
                  label={t("DetailChartCredits")}
                  color="var(--provider-status-ok)"
                  formatValue={(v) => v.toFixed(1)}
                />
              )}
              {hasUsageBreakdown && (
                <StackedBarChart
                  points={chartData!.usageBreakdown}
                  label={t("DetailChartUsageBreakdown")}
                  height={56}
                />
              )}
            </section>
          )}
        </div>
      )}
    </article>
  );
}
