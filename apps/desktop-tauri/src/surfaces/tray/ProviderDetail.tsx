import type { ProviderUsageSnapshot } from "../../types/bridge";
import UsageBar from "./UsageBar";

interface ProviderDetailProps {
  provider: ProviderUsageSnapshot;
  hideEmail: boolean;
  resetRelative: boolean;
  onBack: () => void;
}

function maskEmail(email: string): string {
  const at = email.indexOf("@");
  if (at <= 1) return "••••@••••";
  return email[0] + "•".repeat(at - 1) + email.slice(at);
}

function formatCurrency(amount: number, code: string): string {
  try {
    return new Intl.NumberFormat("en-US", {
      style: "currency",
      currency: code,
    }).format(amount);
  } catch {
    return `${code} ${amount.toFixed(2)}`;
  }
}

export default function ProviderDetail({
  provider,
  hideEmail,
  resetRelative: _resetRelative,
  onBack,
}: ProviderDetailProps) {
  const email = provider.accountEmail
    ? hideEmail
      ? maskEmail(provider.accountEmail)
      : provider.accountEmail
    : null;

  const windows: { label: string; snap: typeof provider.primary }[] = [
    { label: "Primary", snap: provider.primary },
  ];
  if (provider.secondary) windows.push({ label: "Secondary", snap: provider.secondary });
  if (provider.modelSpecific) windows.push({ label: "Model-specific", snap: provider.modelSpecific });
  if (provider.tertiary) windows.push({ label: "Tertiary", snap: provider.tertiary });

  return (
    <div className="tray-detail">
      <button className="tray-detail__back" onClick={onBack} type="button">
        ← Back
      </button>

      <div className="tray-detail__head">
        <h2 className="tray-detail__name">{provider.displayName}</h2>
        {provider.planName && (
          <span className="tray-detail__plan">{provider.planName}</span>
        )}
        {email && <span className="tray-detail__email">{email}</span>}
      </div>

      {provider.error && (
        <div className="tray-detail__error">
          <span>⚠</span> {provider.error}
        </div>
      )}

      <div className="tray-detail__windows">
        {windows.map((w) => (
          <div key={w.label} className="tray-detail__window">
            <UsageBar window={w.snap} label={w.label} />
            <div className="tray-detail__window-meta">
              {w.snap.windowMinutes != null && (
                <span>{w.snap.windowMinutes}m window</span>
              )}
              {w.snap.resetDescription && (
                <span>{w.snap.resetDescription}</span>
              )}
              {w.snap.isExhausted && (
                <span className="tray-detail__exhausted">Exhausted</span>
              )}
            </div>
          </div>
        ))}
      </div>

      {provider.cost && (
        <div className="tray-detail__cost">
          <h3>Cost — {provider.cost.period}</h3>
          <div className="tray-detail__cost-row">
            <span>Used</span>
            <strong>
              {provider.cost.formattedUsed ||
                formatCurrency(provider.cost.used, provider.cost.currencyCode)}
            </strong>
          </div>
          {provider.cost.limit != null && (
            <div className="tray-detail__cost-row">
              <span>Limit</span>
              <strong>
                {provider.cost.formattedLimit ||
                  formatCurrency(
                    provider.cost.limit,
                    provider.cost.currencyCode,
                  )}
              </strong>
            </div>
          )}
          {provider.cost.remaining != null && (
            <div className="tray-detail__cost-row">
              <span>Remaining</span>
              <strong>
                {formatCurrency(
                  provider.cost.remaining,
                  provider.cost.currencyCode,
                )}
              </strong>
            </div>
          )}
          {provider.cost.resetsAt && (
            <div className="tray-detail__cost-row">
              <span>Resets</span>
              <span>{provider.cost.resetsAt}</span>
            </div>
          )}
        </div>
      )}

      <div className="tray-detail__footer">
        <span className="tray-detail__source">{provider.sourceLabel}</span>
        <span className="tray-detail__updated">
          Updated {provider.updatedAt}
        </span>
      </div>
    </div>
  );
}
