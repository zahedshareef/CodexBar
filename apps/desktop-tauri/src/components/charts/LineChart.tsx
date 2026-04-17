/**
 * LineChart — dependency-free SVG line chart primitive (with optional
 * area fill).
 *
 * Port target: the line/area visuals inside
 * `rust/src/native_ui/preferences.rs::render_provider_detail_panel`
 * (credits_history chart).
 *
 * Phase 6f scope: static polyline + optional area. Hover tooltips and
 * entrance animation land in Phase 10.
 */

export interface LineChartPoint {
  label: string;
  value: number;
}

export interface LineChartProps {
  data: LineChartPoint[];
  color?: string;
  height?: number;
  valueFormatter?: (n: number) => string;
  ariaLabel: string;
  /** When true, render a faint filled area under the line. Defaults true. */
  area?: boolean;
}

const DEFAULT_COLOR = "var(--chart-accent, #06d6a0)";
const SVG_WIDTH = 280;

export function LineChart({
  data,
  color = DEFAULT_COLOR,
  height = 56,
  valueFormatter,
  ariaLabel,
  area = true,
}: LineChartProps) {
  if (data.length === 0) return null;

  const fmt = valueFormatter ?? ((v: number) => v.toFixed(2));
  const values = data.map((p) => p.value);
  const max = Math.max(...values, 0.0001);
  const min = Math.min(...values, 0);
  const range = Math.max(max - min, 0.0001);

  const plotHeight = Math.max(1, height - 4);
  const pad = 2;
  const usableWidth = SVG_WIDTH - pad * 2;

  // Build polyline points and matching area path.
  const step = data.length > 1 ? usableWidth / (data.length - 1) : 0;
  const coords = data.map((p, i) => {
    const x = pad + i * step;
    const y = pad + plotHeight - ((p.value - min) / range) * plotHeight;
    return { x, y };
  });

  // Degenerate single-point case: draw a flat segment across the full width.
  if (coords.length === 1) {
    coords.push({ x: pad + usableWidth, y: coords[0].y });
  }

  const polyline = coords.map((c) => `${c.x.toFixed(1)},${c.y.toFixed(1)}`).join(" ");

  const areaPath = area
    ? [
        `M ${coords[0].x.toFixed(1)} ${(pad + plotHeight).toFixed(1)}`,
        ...coords.map((c) => `L ${c.x.toFixed(1)} ${c.y.toFixed(1)}`),
        `L ${coords[coords.length - 1].x.toFixed(1)} ${(pad + plotHeight).toFixed(1)}`,
        "Z",
      ].join(" ")
    : null;

  return (
    <div className="chart chart--line">
      <svg
        width={SVG_WIDTH}
        height={height}
        viewBox={`0 0 ${SVG_WIDTH} ${height}`}
        className="chart__svg"
        role="img"
        aria-label={ariaLabel}
      >
        {areaPath && <path d={areaPath} fill={color} opacity={0.18} />}
        <polyline
          points={polyline}
          fill="none"
          stroke={color}
          strokeWidth={1.5}
          strokeLinejoin="round"
          strokeLinecap="round"
          opacity={0.95}
        />
        {data.map((p, i) => (
          <circle
            key={`${p.label}-${i}`}
            cx={coords[i].x}
            cy={coords[i].y}
            r={1.6}
            fill={color}
          >
            <title>
              {p.label}: {fmt(p.value)}
            </title>
          </circle>
        ))}
      </svg>
      <div className="chart__axis">
        <span>{data[0].label.slice(-5)}</span>
        <span className="chart__axis-max">{fmt(max)}</span>
        <span>{data[data.length - 1].label.slice(-5)}</span>
      </div>
    </div>
  );
}
