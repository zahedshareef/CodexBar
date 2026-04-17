import { useEffect, useState } from "react";
import { getProviderChartData, getSettingsSnapshot } from "../../../../../lib/tauri";
import type { ProviderChartData, SettingsSnapshot } from "../../../../../types/bridge";
import type { useLocale } from "../../../../../hooks/useLocale";
import { CostHistoryChart } from "./CostHistoryChart";
import { CreditsHistoryChart } from "./CreditsHistoryChart";
import { UsageBreakdownChart } from "./UsageBreakdownChart";

type T = ReturnType<typeof useLocale>["t"];

interface Props {
  providerId: string;
  accountEmail: string | null;
  t: T;
}

type TabKey = "cost" | "credits" | "usage";

/**
 * Charts tabs block for the Settings → Providers detail pane.
 *
 * Port target: cost_history / credits_history / usage_breakdown blocks
 * in `rust/src/native_ui/preferences.rs::render_provider_detail_panel`.
 *
 * Phase 10: fetches the latest settings snapshot so animation and
 * surprise-me flags feed through to each chart component.
 */
export function ChartsSection({ providerId, accountEmail, t }: Props) {
  const [data, setData] = useState<ProviderChartData | null>(null);
  const [active, setActive] = useState<TabKey | null>(null);
  const [flags, setFlags] = useState<{ animations: boolean; surprise: boolean }>({
    animations: true,
    surprise: false,
  });

  useEffect(() => {
    let cancelled = false;
    setData(null);
    setActive(null);
    getProviderChartData(providerId, accountEmail ?? undefined)
      .then((d) => {
        if (!cancelled) setData(d);
      })
      .catch(() => {
        if (!cancelled) setData(null);
      });
    return () => {
      cancelled = true;
    };
  }, [providerId, accountEmail]);

  useEffect(() => {
    let cancelled = false;
    getSettingsSnapshot()
      .then((s: SettingsSnapshot) => {
        if (!cancelled) {
          setFlags({
            animations: s.enableAnimations,
            surprise: s.surpriseAnimations,
          });
        }
      })
      .catch(() => {
        // Keep defaults on failure.
      });
    return () => {
      cancelled = true;
    };
  }, [providerId]);

  if (!data) return null;

  const hasCost = data.costHistory.length > 0;
  const hasCredits = data.creditsHistory.length > 0;
  const hasUsage = data.usageBreakdown.length > 0;

  if (!hasCost && !hasCredits && !hasUsage) return null;

  const available: TabKey[] = [];
  if (hasCost) available.push("cost");
  if (hasCredits) available.push("credits");
  if (hasUsage) available.push("usage");

  const current: TabKey = active && available.includes(active) ? active : available[0];
  const emptyMsg = t("DetailChartEmpty");

  const tabLabel = (k: TabKey): string => {
    if (k === "cost") return t("DetailChartCost");
    if (k === "credits") return t("DetailChartCredits");
    return t("DetailChartUsageBreakdown");
  };

  return (
    <section className="provider-detail-section provider-detail-charts">
      <div className="provider-detail-charts__tabs" role="tablist">
        {available.map((k) => (
          <button
            key={k}
            type="button"
            role="tab"
            aria-selected={k === current}
            className="provider-detail-charts__tab"
            data-active={k === current ? "true" : "false"}
            onClick={() => setActive(k)}
          >
            {tabLabel(k)}
          </button>
        ))}
      </div>
      <div className="provider-detail-charts__body" role="tabpanel">
        {current === "cost" && (
          <CostHistoryChart
            data={data.costHistory}
            title={t("DetailChartCost")}
            ariaLabel={t("DetailChartCost")}
            providerId={providerId}
            animations={flags.animations}
            surprise={flags.surprise}
            emptyMessage={emptyMsg}
          />
        )}
        {current === "credits" && (
          <CreditsHistoryChart
            data={data.creditsHistory}
            title={t("DetailChartCredits")}
            ariaLabel={t("DetailChartCredits")}
            providerId={providerId}
            animations={flags.animations}
            surprise={flags.surprise}
            emptyMessage={emptyMsg}
          />
        )}
        {current === "usage" && (
          <UsageBreakdownChart
            data={data.usageBreakdown}
            title={t("DetailChartUsageBreakdown")}
            ariaLabel={t("DetailChartUsageBreakdown")}
            animations={flags.animations}
            surprise={flags.surprise}
            emptyMessage={emptyMsg}
          />
        )}
      </div>
    </section>
  );
}
