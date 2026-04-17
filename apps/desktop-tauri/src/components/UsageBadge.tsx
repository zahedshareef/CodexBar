import type { RateWindowSnapshot } from "../types/bridge";

type UsageLevel = "normal" | "high" | "critical" | "exhausted";

function levelOf(pct: number, exhausted: boolean): UsageLevel {
  if (exhausted) return "exhausted";
  if (pct >= 90) return "critical";
  if (pct >= 70) return "high";
  return "normal";
}

interface UsageBadgeProps {
  window: RateWindowSnapshot;
  /** Render "N% used" (true, default) or "N% left". */
  showUsed?: boolean;
}

/**
 * Compact right-aligned percent pill used in the menu-bar style provider
 * rows. Mirrors the upstream `UsageMenuCardView` percent label, which is
 * always colour-coded by usage severity.
 */
export default function UsageBadge({ window: w, showUsed = true }: UsageBadgeProps) {
  const pct = Math.min(100, Math.max(0, w.usedPercent));
  const level = levelOf(pct, w.isExhausted);
  const value = showUsed ? pct : Math.max(0, 100 - pct);
  return (
    <span className="usage-badge" data-level={level}>
      {value.toFixed(0)}%
    </span>
  );
}
