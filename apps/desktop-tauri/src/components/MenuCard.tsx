import { useEffect, useState } from "react";
import type {
  PaceSnapshot,
  ProviderChartData,
  ProviderUsageSnapshot,
  RateWindowSnapshot,
} from "../types/bridge";
import { getProviderChartData } from "../lib/tauri";
import { useLocale } from "../hooks/useLocale";
import type { LocaleKey } from "../i18n/keys";
import UsageBar from "../surfaces/tray/UsageBar";
import PaceBadge from "../surfaces/tray/PaceBadge";
import { paceCategory } from "../surfaces/tray/paceCategory";
import { SimpleBarChart, StackedBarChart } from "./MiniBarChart";

interface MenuCardProps {
  provider: ProviderUsageSnapshot;
  hideEmail: boolean;
  /** Optional back handler. When omitted the card renders without a back row,
   *  mirroring the full-card variant used by the pop-out split view. */
  onBack?: () => void;
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

interface MetricEntry {
  labelKey: LocaleKey;
  snap: RateWindowSnapshot;
}

/**
 * Compact provider card mirroring the SwiftUI `UsageMenuCardView`.
 *
 * Layout, in order, matches the upstream reference:
 *   1. Header   — name · email (right)          /  source-updated · plan
 *   2. Divider
 *   3. Metrics  — label · usage bar · "N% used" · reset countdown
 *   4. Pace     — expected vs. actual bars + ETA
 *   5. Cost     — period header, used / limit / remaining rows
 *   6. Charts   — cost history, credits history, usage breakdown
 *   7. Footer   — source label + updated timestamp
 *
 * All provider data stays siloed per repo convention — nothing from other
 * providers is ever surfaced here.
 */
export default function MenuCard({
  provider,
  hideEmail,
  onBack,
}: MenuCardProps) {
  const { t } = useLocale();
  const [chartData, setChartData] = useState<ProviderChartData | null>(null);

  useEffect(() => {
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
    { labelKey: "DetailWindowPrimary", snap: provider.primary },
  ];
  if (provider.secondary)
    metrics.push({ labelKey: "DetailWindowSecondary", snap: provider.secondary });
  if (provider.modelSpecific)
    metrics.push({
      labelKey: "DetailWindowModelSpecific",
      snap: provider.modelSpecific,
    });
  if (provider.tertiary)
    metrics.push({ labelKey: "DetailWindowTertiary", snap: provider.tertiary });

  const hasCostHistory =
    chartData !== null && chartData.costHistory.length > 0;
  const hasCreditsHistory =
    chartData !== null && chartData.creditsHistory.length > 0;
  const hasUsageBreakdown =
    chartData !== null && chartData.usageBreakdown.length > 0;
  const hasCharts = hasCostHistory || hasCreditsHistory || hasUsageBreakdown;

  return (
    <article className="menu-card">
      {onBack && (
        <button className="menu-card__back" onClick={onBack} type="button">
          ← {t("DetailBackButton")}
        </button>
      )}

      <header className="menu-card__header">
        <div className="menu-card__title-row">
          <span className="menu-card__name">{provider.displayName}</span>
          {email && <span className="menu-card__email">{email}</span>}
        </div>
        <div className="menu-card__subtitle-row">
          <span className="menu-card__subtitle">
            {provider.sourceLabel}
            {" · "}
            {t("DetailUpdatedPrefix")} {provider.updatedAt}
          </span>
          {provider.planName && (
            <span className="menu-card__plan">{provider.planName}</span>
          )}
        </div>
        {provider.accountOrganization && (
          <div className="menu-card__org">{provider.accountOrganization}</div>
        )}
      </header>

      {provider.error && (
        <div className="menu-card__error" role="alert">
          <span aria-hidden>⚠</span> {provider.error}
        </div>
      )}

      <div className="menu-card__divider" />

      <section className="menu-card__metrics">
        {metrics.map((m) => (
          <div key={m.labelKey} className="menu-metric">
            <UsageBar window={m.snap} label={t(m.labelKey)} />
            {(m.snap.resetDescription ||
              m.snap.windowMinutes != null ||
              m.snap.isExhausted) && (
              <div className="menu-metric__meta">
                {m.snap.resetDescription && (
                  <span>{m.snap.resetDescription}</span>
                )}
                {m.snap.windowMinutes != null && (
                  <span>
                    {m.snap.windowMinutes}
                    {t("DetailWindowMinutesSuffix")}
                  </span>
                )}
                {m.snap.isExhausted && (
                  <span className="menu-metric__exhausted">
                    {t("DetailWindowExhausted")}
                  </span>
                )}
              </div>
            )}
          </div>
        ))}
      </section>

      {provider.pace && (
        <>
          <div className="menu-card__divider" />
          <section className="menu-card__section menu-card__pace">
            <div className="menu-card__pace-header">
              <span className="menu-card__section-title">
                {t("DetailPaceTitle")}
              </span>
              <span className="menu-card__pace-label-group">
                <span
                  className="menu-card__pace-label"
                  data-pace={paceCategory(provider.pace.stage)}
                >
                  {t(paceStageKey(provider.pace.stage))} (
                  {provider.pace.deltaPercent >= 0 ? "+" : ""}
                  {provider.pace.deltaPercent.toFixed(1)}%)
                </span>
                <PaceBadge pace={provider.pace} showDelta={false} />
              </span>
            </div>
            <div className="menu-card__pace-bars">
              <div className="menu-card__pace-track">
                <div
                  className="menu-card__pace-fill menu-card__pace-fill--expected"
                  style={{
                    width: `${provider.pace.expectedUsedPercent.toFixed(1)}%`,
                  }}
                  title={`Expected: ${provider.pace.expectedUsedPercent.toFixed(1)}%`}
                />
              </div>
              <div className="menu-card__pace-track">
                <div
                  className="menu-card__pace-fill"
                  data-pace={paceCategory(provider.pace.stage)}
                  style={{
                    width: `${provider.pace.actualUsedPercent.toFixed(1)}%`,
                  }}
                  title={`Actual: ${provider.pace.actualUsedPercent.toFixed(1)}%`}
                />
              </div>
            </div>
            {provider.pace.etaSeconds != null &&
              !provider.pace.willLastToReset && (
                <span className="menu-card__pace-eta">
                  ⚠{" "}
                  {t("DetailPaceRunsOutIn").replace(
                    "{}",
                    String(Math.round(provider.pace.etaSeconds / 3600)),
                  )}
                </span>
              )}
            {provider.pace.willLastToReset && (
              <span className="menu-card__pace-ok">
                ✓ {t("DetailPaceWillLastToReset")}
              </span>
            )}
          </section>
        </>
      )}

      {provider.cost && (
        <>
          <div className="menu-card__divider" />
          <section className="menu-card__section menu-card__cost">
            <div className="menu-card__section-title">
              {t("DetailCostTitle")} — {provider.cost.period}
            </div>
            <dl className="menu-card__rows">
              <div className="menu-card__row">
                <dt>{t("DetailCostUsed")}</dt>
                <dd>
                  {provider.cost.formattedUsed ||
                    formatCurrency(
                      provider.cost.used,
                      provider.cost.currencyCode,
                    )}
                </dd>
              </div>
              {provider.cost.limit != null && (
                <div className="menu-card__row">
                  <dt>{t("DetailCostLimit")}</dt>
                  <dd>
                    {provider.cost.formattedLimit ||
                      formatCurrency(
                        provider.cost.limit,
                        provider.cost.currencyCode,
                      )}
                  </dd>
                </div>
              )}
              {provider.cost.remaining != null && (
                <div className="menu-card__row">
                  <dt>{t("DetailCostRemaining")}</dt>
                  <dd>
                    {formatCurrency(
                      provider.cost.remaining,
                      provider.cost.currencyCode,
                    )}
                  </dd>
                </div>
              )}
              {provider.cost.resetsAt && (
                <div className="menu-card__row">
                  <dt>{t("DetailCostResets")}</dt>
                  <dd>{provider.cost.resetsAt}</dd>
                </div>
              )}
            </dl>
          </section>
        </>
      )}

      {hasCharts && (
        <>
          <div className="menu-card__divider" />
          <section className="menu-card__section menu-card__charts">
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
        </>
      )}
    </article>
  );
}
