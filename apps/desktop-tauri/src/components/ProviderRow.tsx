import type { ProviderUsageSnapshot } from "../types/bridge";
import { useLocale } from "../hooks/useLocale";
import { useResetCountdown } from "../surfaces/tray/useResetCountdown";
import PaceBadge from "../surfaces/tray/PaceBadge";
import UsageBadge from "./UsageBadge";

interface ProviderRowProps {
  provider: ProviderUsageSnapshot;
  selected: boolean;
  hideEmail: boolean;
  resetRelative: boolean;
  onSelect: () => void;
}

function maskEmail(email: string): string {
  const at = email.indexOf("@");
  if (at <= 1) return "••••@••••";
  return email[0] + "•".repeat(at - 1) + email.slice(at);
}

/**
 * Compact single-provider row used by both the tray and pop-out surfaces.
 *
 * Mirrors the density of a native NSMenuItem: two tight lines — primary
 * (status dot · name · plan · usage badge) and secondary (reset / email ·
 * pace). Mirrors the structure of `MenuContent.row` while keeping the
 * affordance that the row is clickable to drill into the full
 * `UsageMenuCardView` (in `MenuCard`).
 */
export default function ProviderRow({
  provider,
  selected,
  hideEmail,
  resetRelative,
  onSelect,
}: ProviderRowProps) {
  const { t } = useLocale();
  const hasError = provider.error !== null;
  const countdown = useResetCountdown(
    provider.primary.resetsAt,
    provider.primary.resetDescription,
  );
  const reset =
    resetRelative && provider.primary.resetsAt
      ? countdown
      : provider.primary.resetDescription;
  const email = provider.accountEmail
    ? hideEmail
      ? maskEmail(provider.accountEmail)
      : provider.accountEmail
    : null;

  // Status dot mirrors `IconRemainingResolver` / tray status indicator.
  let state: "ok" | "warn" | "critical" | "error" = "ok";
  if (hasError) state = "error";
  else if (provider.primary.isExhausted) state = "critical";
  else if (provider.primary.usedPercent >= 90) state = "critical";
  else if (provider.primary.usedPercent >= 70) state = "warn";

  const secondaryParts: string[] = [];
  if (provider.planName) secondaryParts.push(provider.planName);
  if (reset) secondaryParts.push(reset);
  else if (email) secondaryParts.push(email);

  return (
    <button
      className="menu-row"
      data-selected={selected || undefined}
      data-error={hasError || undefined}
      onClick={onSelect}
      type="button"
    >
      <span className="menu-row__dot" data-state={state} />
      <div className="menu-row__body">
        <div className="menu-row__line menu-row__line--primary">
          <span className="menu-row__name">{provider.displayName}</span>
          {hasError ? (
            <span className="menu-row__err" title={provider.error ?? ""}>
              ⚠ {t("TrayCardErrorBadge")}
            </span>
          ) : (
            <UsageBadge window={provider.primary} />
          )}
        </div>
        {(secondaryParts.length > 0 || provider.pace) && (
          <div className="menu-row__line menu-row__line--secondary">
            {secondaryParts.length > 0 && (
              <span className="menu-row__meta">
                {secondaryParts.join(" · ")}
              </span>
            )}
            {provider.pace && <PaceBadge pace={provider.pace} />}
          </div>
        )}
      </div>
    </button>
  );
}
