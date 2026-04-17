import type { RateWindowSnapshot } from "../../types/bridge";

interface UsageBarProps {
  window: RateWindowSnapshot;
  label?: string;
  compact?: boolean;
}

type UsageLevel = "normal" | "high" | "critical" | "exhausted";

function usageLevel(pct: number, exhausted: boolean): UsageLevel {
  if (exhausted) return "exhausted";
  if (pct >= 90) return "critical";
  if (pct >= 70) return "high";
  return "normal";
}

export default function UsageBar({ window: w, label, compact }: UsageBarProps) {
  const pct = Math.min(100, Math.max(0, w.usedPercent));
  const level = usageLevel(pct, w.isExhausted);

  return (
    <div className={`usage-bar ${compact ? "usage-bar--compact" : ""}`}>
      {label && <span className="usage-bar__label">{label}</span>}
      <div className="usage-bar__track">
        <div
          className="usage-bar__fill"
          data-level={level}
          style={{ width: `${pct}%` }}
        />
      </div>
      <span className="usage-bar__pct">{pct.toFixed(0)}%</span>
    </div>
  );
}
