import { BarChart } from "../../../../../components/charts/BarChart";
import { providerCostColor } from "../../../../../components/charts/chartPalette";
import type { DailyCostPoint } from "../../../../../types/bridge";

interface Props {
  data: DailyCostPoint[];
  title: string;
  ariaLabel: string;
  providerId: string;
  animations: boolean;
  surprise: boolean;
  emptyMessage: string;
}

/**
 * Port target: the cost_history bar cluster in
 * `rust/src/native_ui/preferences.rs::render_provider_detail_panel`.
 * Phase 10 wires through per-provider palette tokens + animation flags.
 */
export function CostHistoryChart({
  data,
  title,
  ariaLabel,
  providerId,
  animations,
  surprise,
  emptyMessage,
}: Props) {
  const recent = data.slice(-30);
  const points = recent.map((p) => ({ label: p.date, value: p.value }));
  return (
    <div className="provider-detail-chart">
      <div className="provider-detail-chart__title">{title}</div>
      <BarChart
        data={points}
        color={providerCostColor(providerId)}
        ariaLabel={ariaLabel}
        valueFormatter={(v) => `$${v.toFixed(2)}`}
        animations={animations}
        surprise={surprise}
        emptyMessage={emptyMessage}
      />
    </div>
  );
}
