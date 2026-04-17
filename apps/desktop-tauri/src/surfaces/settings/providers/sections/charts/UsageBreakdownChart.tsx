import type { DailyUsageBreakdown } from "../../../../../types/bridge";

/**
 * UsageBreakdownChart — horizontal stacked bars, one row per day, each
 * segment proportional to that service's share of the day's total
 * credits. Rendered as pure SVG.
 *
 * Port target: the usage_breakdown stacked bar cluster in
 * `rust/src/native_ui/preferences.rs::render_provider_detail_panel`.
 *
 * Phase 6f uses a deterministic placeholder palette; the full service
 * palette is Phase 10.
 */

interface Props {
  data: DailyUsageBreakdown[];
  title: string;
  ariaLabel: string;
}

// Deterministic placeholder palette — Phase 10 will swap this for the
// theme's full service-color tokens.
const PALETTE = [
  "#5d87ff",
  "#06d6a0",
  "#ffd166",
  "#ef476f",
  "#a78bfa",
  "#38bdf8",
  "#fb923c",
  "#4ade80",
];

function serviceColor(service: string, ordered: string[]): string {
  const idx = ordered.indexOf(service);
  return PALETTE[(idx < 0 ? 0 : idx) % PALETTE.length];
}

export function UsageBreakdownChart({ data, title, ariaLabel }: Props) {
  const recent = data.slice(-14);
  if (recent.length === 0) return null;

  const allServices = Array.from(
    new Set(recent.flatMap((d) => d.services.map((s) => s.service))),
  ).sort();

  const rowHeight = 14;
  const rowGap = 2;
  const labelWidth = 52;
  const totalWidth = 280;
  const barAreaWidth = totalWidth - labelWidth;
  const svgHeight = recent.length * (rowHeight + rowGap);

  // Max total across rows so bar lengths are comparable day-over-day.
  const max = Math.max(...recent.map((d) => d.totalCreditsUsed), 0.0001);

  return (
    <div className="provider-detail-chart">
      <div className="provider-detail-chart__title">{title}</div>
      <div className="chart chart--stacked">
        <svg
          width={totalWidth}
          height={svgHeight}
          viewBox={`0 0 ${totalWidth} ${svgHeight}`}
          className="chart__svg"
          role="img"
          aria-label={ariaLabel}
        >
          {recent.map((day, rowIdx) => {
            const y = rowIdx * (rowHeight + rowGap);
            const rowWidth = (day.totalCreditsUsed / max) * barAreaWidth;
            let xOffset = labelWidth;
            const sorted = [...day.services].sort((a, b) =>
              a.service.localeCompare(b.service),
            );
            return (
              <g key={day.day}>
                <text
                  x={0}
                  y={y + rowHeight - 3}
                  fontSize={10}
                  className="chart__row-label"
                  fill="var(--provider-row-text-secondary, #888)"
                >
                  {day.day.slice(-5)}
                </text>
                {sorted.map((svc) => {
                  const w =
                    day.totalCreditsUsed > 0
                      ? (svc.creditsUsed / day.totalCreditsUsed) * rowWidth
                      : 0;
                  const rect = (
                    <rect
                      key={`${day.day}-${svc.service}`}
                      x={xOffset}
                      y={y}
                      width={Math.max(0.5, w)}
                      height={rowHeight}
                      fill={serviceColor(svc.service, allServices)}
                      opacity={0.9}
                      rx={1}
                    >
                      <title>
                        {day.day} {svc.service}: {svc.creditsUsed.toFixed(2)}
                      </title>
                    </rect>
                  );
                  xOffset += w;
                  return rect;
                })}
              </g>
            );
          })}
        </svg>
        {allServices.length > 0 && (
          <div className="chart__legend">
            {allServices.slice(0, 8).map((svc) => (
              <span key={svc} className="chart__legend-item">
                <span
                  className="chart__legend-dot"
                  style={{ background: serviceColor(svc, allServices) }}
                />
                {svc}
              </span>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
