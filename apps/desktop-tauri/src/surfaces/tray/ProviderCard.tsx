import type { ProviderUsageSnapshot } from "../../types/bridge";
import UsageBar from "./UsageBar";

interface ProviderCardProps {
  provider: ProviderUsageSnapshot;
  selected: boolean;
  hideEmail: boolean;
  resetRelative: boolean;
  onSelect: () => void;
}

function resetText(
  snap: ProviderUsageSnapshot,
  relative: boolean,
): string | null {
  const desc = snap.primary.resetDescription;
  if (!desc) return null;
  if (relative && snap.primary.resetsAt) {
    return `Resets ${desc}`;
  }
  return desc;
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
  const hasError = provider.error !== null;
  const reset = resetText(provider, resetRelative);
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

      <UsageBar window={provider.primary} compact />

      <div className="tray-card__meta">
        {email && <span className="tray-card__email">{email}</span>}
        {reset && <span className="tray-card__reset">{reset}</span>}
        {hasError && (
          <span className="tray-card__err" title={provider.error ?? ""}>
            ⚠ Error
          </span>
        )}
      </div>
    </button>
  );
}
