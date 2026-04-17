/**
 * BarChart — dependency-free SVG bar chart primitive.
 *
 * Port target: the bar-chart visuals inside
 * `rust/src/native_ui/preferences.rs::render_provider_detail_panel`
 * (cost_history / credits_history bars).
 *
 * Phase 6f scope: static bars only. Hover tooltip animation and full
 * palette theming are deferred to Phase 10; an `onHoverPoint` hook is
 * intentionally left off the props so Phase 10 can add it without a
 * breaking change.
 */

export interface BarChartPoint {
  label: string;
  value: number;
}

export interface BarChartProps {
  data: BarChartPoint[];
  color?: string;
  height?: number;
  valueFormatter?: (n: number) => string;
  ariaLabel: string;
}

const DEFAULT_COLOR = "var(--chart-accent, #5d87ff)";
const BAR_GAP = 2;
const SVG_WIDTH = 280;

export function BarChart({
  data,
  color = DEFAULT_COLOR,
  height = 56,
  valueFormatter,
  ariaLabel,
}: BarChartProps) {
  if (data.length === 0) return null;

  const fmt = valueFormatter ?? ((v: number) => v.toFixed(2));
  const max = Math.max(...data.map((p) => p.value), 0.0001);

  const barWidth = Math.max(
    1,
    Math.floor((SVG_WIDTH - (data.length - 1) * BAR_GAP) / data.length),
  );
  const actualWidth = data.length * barWidth + (data.length - 1) * BAR_GAP;
  const plotHeight = Math.max(1, height - 4);

  return (
    <div className="chart chart--bar">
      <svg
        width={actualWidth}
        height={height}
        viewBox={`0 0 ${actualWidth} ${height}`}
        className="chart__svg"
        role="img"
        aria-label={ariaLabel}
      >
        {data.map((p, i) => {
          const barH = Math.max(1, (p.value / max) * plotHeight);
          const x = i * (barWidth + BAR_GAP);
          const y = height - barH;
          return (
            <rect
              key={`${p.label}-${i}`}
              x={x}
              y={y}
              width={barWidth}
              height={barH}
              fill={color}
              opacity={p.value === 0 ? 0.25 : 0.9}
              rx={1}
            >
              <title>
                {p.label}: {fmt(p.value)}
              </title>
            </rect>
          );
        })}
      </svg>
      <div className="chart__axis">
        <span>{data[0].label.slice(-5)}</span>
        <span className="chart__axis-max">{fmt(max)}</span>
        <span>{data[data.length - 1].label.slice(-5)}</span>
      </div>
    </div>
  );
}
