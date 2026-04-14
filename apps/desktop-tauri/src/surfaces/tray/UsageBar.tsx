import type { RateWindowSnapshot } from "../../types/bridge";

interface UsageBarProps {
  window: RateWindowSnapshot;
  label?: string;
  compact?: boolean;
}

function barColor(pct: number, exhausted: boolean): string {
  if (exhausted) return "#ff6c6c";
  if (pct >= 90) return "#ff9f43";
  if (pct >= 70) return "#ffd166";
  return "#5d87ff";
}

export default function UsageBar({ window: w, label, compact }: UsageBarProps) {
  const pct = Math.min(100, Math.max(0, w.usedPercent));
  const color = barColor(pct, w.isExhausted);

  return (
    <div className={`usage-bar ${compact ? "usage-bar--compact" : ""}`}>
      {label && <span className="usage-bar__label">{label}</span>}
      <div className="usage-bar__track">
        <div
          className="usage-bar__fill"
          style={{ width: `${pct}%`, background: color }}
        />
      </div>
      <span className="usage-bar__pct">{pct.toFixed(0)}%</span>
    </div>
  );
}
