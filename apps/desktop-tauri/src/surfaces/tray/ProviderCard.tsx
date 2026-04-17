import type { ProviderUsageSnapshot } from "../../types/bridge";
import { useLocale } from "../../hooks/useLocale";
import UsageBar from "./UsageBar";
import PaceBadge from "./PaceBadge";
import { useResetCountdown } from "./useResetCountdown";

interface ProviderCardProps {
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

export default function ProviderCard({
  provider,
  selected,
  hideEmail,
  resetRelative,
  onSelect,
}: ProviderCardProps) {
  const { t } = useLocale();
  const hasError = provider.error !== null;
  const countdown = useResetCountdown(
    provider.primary.resetsAt,
    provider.primary.resetDescription,
  );
  const reset = resetRelative && provider.primary.resetsAt
    ? countdown
    : provider.primary.resetDescription;
  const email = provider.accountEmail
    ? hideEmail
      ? maskEmail(provider.accountEmail)
      : provider.accountEmail
    : null;

  return (
    <button
      className={`tray-card ${selected ? "tray-card--selected" : ""} ${hasError ? "tray-card--error" : ""}`}
      onClick={onSelect}
      type="button"
    >
      <div className="tray-card__header">
        <span className="tray-card__name">{provider.displayName}</span>
        {provider.planName && (
          <span className="tray-card__plan">{provider.planName}</span>
        )}
      </div>

      <div className="tray-card__pace-row">
        <UsageBar window={provider.primary} compact />
        {provider.pace && <PaceBadge pace={provider.pace} />}
      </div>

      <div className="tray-card__meta">
        {email && <span className="tray-card__email">{email}</span>}
        {reset && (
          <span
            className={
              resetRelative && provider.primary.resetsAt
                ? "tray-card__reset tray-card__reset--countdown"
                : "tray-card__reset"
            }
          >
            {reset}
          </span>
        )}
        {hasError && (
          <span className="tray-card__err" title={provider.error ?? ""}>
            ⚠ {t("TrayCardErrorBadge")}
          </span>
        )}
      </div>
    </button>
  );
}
