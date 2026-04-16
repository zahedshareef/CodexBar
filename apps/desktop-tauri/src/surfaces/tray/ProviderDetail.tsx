import { useEffect, useState } from "react";
import type { PaceSnapshot, ProviderChartData, ProviderUsageSnapshot } from "../../types/bridge";
import { getProviderChartData } from "../../lib/tauri";
import UsageBar from "./UsageBar";
import {
  SimpleBarChart,
  StackedBarChart,
} from "../../components/MiniBarChart";

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

function paceLabel(pace: PaceSnapshot): string {
  const sign = pace.deltaPercent >= 0 ? "+" : "";
  const pct = `${sign}${pace.deltaPercent.toFixed(1)}%`;
  switch (pace.stage) {
    case "on_track":
      return `On track (${pct})`;
    case "slightly_ahead":
      return `Slightly ahead (${pct})`;
    case "ahead":
      return `Ahead (${pct})`;
    case "far_ahead":
      return `Far ahead (${pct})`;
    case "slightly_behind":
      return `Slightly behind (${pct})`;
    case "behind":
      return `Behind (${pct})`;
    case "far_behind":
      return `Far behind (${pct})`;
    default:
      return pct;
  }
}

function paceColor(stage: PaceSnapshot["stage"]): string {
  switch (stage) {
    case "on_track":
      return "#06d6a0";
    case "slightly_ahead":
    case "ahead":
      return "#5d87ff";
    case "far_ahead":
      return "#a78bfa";
    case "slightly_behind":
      return "#ffd166";
    case "behind":
      return "#fb923c";
    case "far_behind":
      return "#ef476f";
    default:
      return "#8b95b0";
  }
}

export default function ProviderDetail({
  provider,
  hideEmail,
  resetRelative: _resetRelative,
  onBack,
}: ProviderDetailProps) {
  const [chartData, setChartData] = useState<ProviderChartData | null>(null);

  useEffect(() => {
    let cancelled = false;
    getProviderChartData(provider.providerId)
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

  const windows: { label: string; snap: typeof provider.primary }[] = [
    { label: "Primary", snap: provider.primary },
  ];
  if (provider.secondary) windows.push({ label: "Secondary", snap: provider.secondary });
  if (provider.modelSpecific) windows.push({ label: "Model-specific", snap: provider.modelSpecific });
  if (provider.tertiary) windows.push({ label: "Tertiary", snap: provider.tertiary });

  const hasCostHistory =
    chartData !== null && chartData.costHistory.length > 0;
  const hasCreditsHistory =
    chartData !== null && chartData.creditsHistory.length > 0;
  const hasUsageBreakdown =
    chartData !== null && chartData.usageBreakdown.length > 0;

  return (
    <div className="tray-detail">
      <button className="tray-detail__back" onClick={onBack} type="button">
        ← Back
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
          <div key={w.label} className="tray-detail__window">
            <UsageBar window={w.snap} label={w.label} />
            <div className="tray-detail__window-meta">
              {w.snap.windowMinutes != null && (
                <span>{w.snap.windowMinutes}m window</span>
              )}
              {w.snap.resetDescription && (
                <span>{w.snap.resetDescription}</span>
              )}
              {w.snap.isExhausted && (
                <span className="tray-detail__exhausted">Exhausted</span>
              )}
            </div>
          </div>
        ))}
      </div>

      {/* Pace indicator */}
      {provider.pace && (
        <div className="tray-detail__pace">
          <div className="tray-detail__pace-header">
            <span className="tray-detail__pace-title">Pace</span>
            <span
              className="tray-detail__pace-label"
              style={{ color: paceColor(provider.pace.stage) }}
            >
              {paceLabel(provider.pace)}
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
                style={{
                  width: `${provider.pace.actualUsedPercent.toFixed(1)}%`,
                  background: paceColor(provider.pace.stage),
                }}
                title={`Actual: ${provider.pace.actualUsedPercent.toFixed(1)}%`}
              />
            </div>
          </div>
          {provider.pace.etaSeconds != null && !provider.pace.willLastToReset && (
            <span className="tray-detail__pace-eta">
              ⚠ Runs out in ~{Math.round(provider.pace.etaSeconds / 3600)}h
            </span>
          )}
          {provider.pace.willLastToReset && (
            <span className="tray-detail__pace-ok">
              ✓ Will last to reset
            </span>
          )}
        </div>
      )}

      {provider.cost && (
        <div className="tray-detail__cost">
          <h3>Cost — {provider.cost.period}</h3>
          <div className="tray-detail__cost-row">
            <span>Used</span>
            <strong>
              {provider.cost.formattedUsed ||
                formatCurrency(provider.cost.used, provider.cost.currencyCode)}
            </strong>
          </div>
          {provider.cost.limit != null && (
            <div className="tray-detail__cost-row">
              <span>Limit</span>
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
              <span>Remaining</span>
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
              <span>Resets</span>
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
              label="Cost (30 days)"
              color="#5d87ff"
              formatValue={(v) => `$${v.toFixed(2)}`}
            />
          )}
          {hasCreditsHistory && (
            <SimpleBarChart
              points={chartData!.creditsHistory}
              label="Credits used (30 days)"
              color="#06d6a0"
              formatValue={(v) => v.toFixed(1)}
            />
          )}
          {hasUsageBreakdown && (
            <StackedBarChart
              points={chartData!.usageBreakdown}
              label="Usage by service (30 days)"
              height={64}
            />
          )}
        </div>
      )}

      <div className="tray-detail__footer">
        <span className="tray-detail__source">{provider.sourceLabel}</span>
        <span className="tray-detail__updated">
          Updated {provider.updatedAt}
        </span>
      </div>
    </div>
  );
}

