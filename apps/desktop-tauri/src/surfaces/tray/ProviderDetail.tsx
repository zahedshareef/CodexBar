import { useEffect, useState } from "react";
import type { PaceSnapshot, ProviderChartData, ProviderUsageSnapshot } from "../../types/bridge";
import { getProviderChartData } from "../../lib/tauri";
import { useLocale } from "../../hooks/useLocale";
import UsageBar from "./UsageBar";
import PaceBadge from "./PaceBadge";
import { paceCategory } from "./paceCategory";
import {
  SimpleBarChart,
  StackedBarChart,
} from "../../components/MiniBarChart";
import type { LocaleKey } from "../../i18n/keys";

interface ProviderDetailProps {
  provider: ProviderUsageSnapshot;
  hideEmail: boolean;
  resetRelative: boolean;
  onBack: () => void;
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

export default function ProviderDetail({
  provider,
  hideEmail,
  resetRelative: _resetRelative,
  onBack,
}: ProviderDetailProps) {
  const { t } = useLocale();
  const [chartData, setChartData] = useState<ProviderChartData | null>(null);

  useEffect(() => {
    let cancelled = false;
    setChartData(null);
    getProviderChartData(provider.providerId, provider.accountEmail ?? undefined)
      .then((data) => {
        if (!cancelled) setChartData(data);
      })
      .catch(() => {/* chart data is best-effort */});
    return () => {
      cancelled = true;
    };
  }, [provider.providerId]);

  const email = provider.accountEmail
    ? hideEmail
      ? maskEmail(provider.accountEmail)
      : provider.accountEmail
    : null;

  const windows: { labelKey: LocaleKey; snap: typeof provider.primary }[] = [
    { labelKey: "DetailWindowPrimary", snap: provider.primary },
  ];
  if (provider.secondary) windows.push({ labelKey: "DetailWindowSecondary", snap: provider.secondary });
  if (provider.modelSpecific) windows.push({ labelKey: "DetailWindowModelSpecific", snap: provider.modelSpecific });
  if (provider.tertiary) windows.push({ labelKey: "DetailWindowTertiary", snap: provider.tertiary });

  const hasCostHistory =
    chartData !== null && chartData.costHistory.length > 0;
  const hasCreditsHistory =
    chartData !== null && chartData.creditsHistory.length > 0;
  const hasUsageBreakdown =
    chartData !== null && chartData.usageBreakdown.length > 0;

  return (
    <div className="tray-detail">
      <button className="tray-detail__back" onClick={onBack} type="button">
        ← {t("DetailBackButton")}
      </button>

      <div className="tray-detail__head">
        <h2 className="tray-detail__name">{provider.displayName}</h2>
        {provider.planName && (
          <span className="tray-detail__plan">{provider.planName}</span>
        )}
        {email && <span className="tray-detail__email">{email}</span>}
        {provider.accountOrganization && (
          <span className="tray-detail__org">{provider.accountOrganization}</span>
        )}
      </div>

      {provider.error && (
        <div className="tray-detail__error">
          <span>⚠</span> {provider.error}
        </div>
      )}

      <div className="tray-detail__windows">
        {windows.map((w) => (
          <div key={w.labelKey} className="tray-detail__window">
            <UsageBar window={w.snap} label={t(w.labelKey)} />
            <div className="tray-detail__window-meta">
              {w.snap.windowMinutes != null && (
                <span>{w.snap.windowMinutes}{t("DetailWindowMinutesSuffix")}</span>
              )}
              {w.snap.resetDescription && (
                <span>{w.snap.resetDescription}</span>
              )}
              {w.snap.isExhausted && (
                <span className="tray-detail__exhausted">{t("DetailWindowExhausted")}</span>
              )}
            </div>
          </div>
        ))}
      </div>

      {/* Pace indicator */}
      {provider.pace && (
        <div className="tray-detail__pace">
          <div className="tray-detail__pace-header">
            <span className="tray-detail__pace-title">{t("DetailPaceTitle")}</span>
            <span className="tray-detail__pace-label-group">
              <span
                className="tray-detail__pace-label"
                data-pace={paceCategory(provider.pace.stage)}
              >
                {t(paceStageKey(provider.pace.stage))} ({provider.pace.deltaPercent >= 0 ? "+" : ""}
                {provider.pace.deltaPercent.toFixed(1)}%)
              </span>
              <PaceBadge pace={provider.pace} showDelta={false} />
            </span>
          </div>
          <div className="tray-detail__pace-bars">
            <div className="tray-detail__pace-track">
              <div
                className="tray-detail__pace-fill tray-detail__pace-fill--expected"
                style={{ width: `${provider.pace.expectedUsedPercent.toFixed(1)}%` }}
                title={`Expected: ${provider.pace.expectedUsedPercent.toFixed(1)}%`}
              />
            </div>
            <div className="tray-detail__pace-track">
              <div
                className="tray-detail__pace-fill"
                data-pace={paceCategory(provider.pace.stage)}
                style={{ width: `${provider.pace.actualUsedPercent.toFixed(1)}%` }}
                title={`Actual: ${provider.pace.actualUsedPercent.toFixed(1)}%`}
              />
            </div>
          </div>
          {provider.pace.etaSeconds != null && !provider.pace.willLastToReset && (
            <span className="tray-detail__pace-eta">
              ⚠ {t("DetailPaceRunsOutIn").replace(
                "{}",
                String(Math.round(provider.pace.etaSeconds / 3600)),
              )}
            </span>
          )}
          {provider.pace.willLastToReset && (
            <span className="tray-detail__pace-ok">
              ✓ {t("DetailPaceWillLastToReset")}
            </span>
          )}
        </div>
      )}

      {provider.cost && (
        <div className="tray-detail__cost">
          <h3>{t("DetailCostTitle")} — {provider.cost.period}</h3>
          <div className="tray-detail__cost-row">
            <span>{t("DetailCostUsed")}</span>
            <strong>
              {provider.cost.formattedUsed ||
                formatCurrency(provider.cost.used, provider.cost.currencyCode)}
            </strong>
          </div>
          {provider.cost.limit != null && (
            <div className="tray-detail__cost-row">
              <span>{t("DetailCostLimit")}</span>
              <strong>
                {provider.cost.formattedLimit ||
                  formatCurrency(
                    provider.cost.limit,
                    provider.cost.currencyCode,
                  )}
              </strong>
            </div>
          )}
          {provider.cost.remaining != null && (
            <div className="tray-detail__cost-row">
              <span>{t("DetailCostRemaining")}</span>
              <strong>
                {formatCurrency(
                  provider.cost.remaining,
                  provider.cost.currencyCode,
                )}
              </strong>
            </div>
          )}
          {provider.cost.resetsAt && (
            <div className="tray-detail__cost-row">
              <span>{t("DetailCostResets")}</span>
              <span>{provider.cost.resetsAt}</span>
            </div>
          )}
        </div>
      )}

      {/* Charts section */}
      {(hasCostHistory || hasCreditsHistory || hasUsageBreakdown) && (
        <div className="tray-detail__charts">
          {hasCostHistory && (
            <SimpleBarChart
              points={chartData!.costHistory}
              label={t("DetailChartCost")}
              color="#5d87ff"
              formatValue={(v) => `$${v.toFixed(2)}`}
            />
          )}
          {hasCreditsHistory && (
            <SimpleBarChart
              points={chartData!.creditsHistory}
              label={t("DetailChartCredits")}
              color="#06d6a0"
              formatValue={(v) => v.toFixed(1)}
            />
          )}
          {hasUsageBreakdown && (
            <StackedBarChart
              points={chartData!.usageBreakdown}
              label={t("DetailChartUsageBreakdown")}
              height={64}
            />
          )}
        </div>
      )}

      <div className="tray-detail__footer">
        <span className="tray-detail__source">{provider.sourceLabel}</span>
        <span className="tray-detail__updated">
          {t("DetailUpdatedPrefix")} {provider.updatedAt}
        </span>
      </div>
    </div>
  );
}

