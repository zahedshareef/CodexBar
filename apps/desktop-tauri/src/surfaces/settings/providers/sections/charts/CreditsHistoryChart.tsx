import { LineChart } from "../../../../../components/charts/LineChart";
import type { DailyCostPoint } from "../../../../../types/bridge";

interface Props {
  data: DailyCostPoint[];
  title: string;
  ariaLabel: string;
}

/**
 * Port target: the credits_history line/area in
 * `rust/src/native_ui/preferences.rs::render_provider_detail_panel`.
 */
export function CreditsHistoryChart({ data, title, ariaLabel }: Props) {
  const recent = data.slice(-30);
  const points = recent.map((p) => ({ label: p.date, value: p.value }));
  return (
    <div className="provider-detail-chart">
      <div className="provider-detail-chart__title">{title}</div>
      <LineChart
        data={points}
        color="var(--chart-credits, #06d6a0)"
        ariaLabel={ariaLabel}
        valueFormatter={(v) => v.toFixed(1)}
      />
    </div>
  );
}
