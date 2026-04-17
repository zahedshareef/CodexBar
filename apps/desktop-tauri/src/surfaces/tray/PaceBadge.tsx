import type { PaceSnapshot } from "../../types/bridge";
import { useLocale } from "../../hooks/useLocale";
import { paceCategory, paceCategoryKey } from "./paceCategory";

interface PaceBadgeProps {
  pace: PaceSnapshot;
  showDelta?: boolean;
}

/**
 * Compact pill that renders a pace category (slow / steady / racing / burning)
 * next to a provider's usage meter. Mirrors the colour-coding used by the
 * egui pop-out so the two surfaces read the same at a glance.
 */
export default function PaceBadge({ pace, showDelta = true }: PaceBadgeProps) {
  const { t } = useLocale();
  const category = paceCategory(pace.stage);
  const label = t(paceCategoryKey(category));
  const deltaSign = pace.deltaPercent >= 0 ? "+" : "";

  return (
    <span
      className="tray-pace-badge"
      data-pace={category}
      title={`${label} (${deltaSign}${pace.deltaPercent.toFixed(1)}%)`}
    >
      {label}
      {showDelta && (
        <span className="tray-pace-badge__delta">
          {deltaSign}
          {pace.deltaPercent.toFixed(0)}%
        </span>
      )}
    </span>
  );
}
