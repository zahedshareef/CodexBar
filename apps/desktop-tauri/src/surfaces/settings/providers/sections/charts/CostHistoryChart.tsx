import { BarChart } from "../../../../../components/charts/BarChart";
import type { DailyCostPoint } from "../../../../../types/bridge";

interface Props {
  data: DailyCostPoint[];
  title: string;
  ariaLabel: string;
}

/**
 * Port target: the cost_history bar cluster in
 * `rust/src/native_ui/preferences.rs::render_provider_detail_panel`.
 */
export function CostHistoryChart({ data, title, ariaLabel }: Props) {
  const recent = data.slice(-30);
  const points = recent.map((p) => ({ label: p.date, value: p.value }));
  return (
    <div className="provider-detail-chart">
      <div className="provider-detail-chart__title">{title}</div>
      <BarChart
        data={points}
        color="var(--chart-cost, #5d87ff)"
        ariaLabel={ariaLabel}
        valueFormatter={(v) => `$${v.toFixed(2)}`}
      />
    </div>
  );
}
