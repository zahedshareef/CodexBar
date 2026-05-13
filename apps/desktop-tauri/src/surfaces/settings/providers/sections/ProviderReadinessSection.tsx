import type { ProviderDetail } from "../../../../types/bridge";

export type ProviderReadinessState = "ready" | "disabled" | "error" | "waiting";

export interface ProviderReadiness {
  state: ProviderReadinessState;
  title: string;
  detail: string;
}

interface Props {
  provider: ProviderDetail;
  busy: boolean;
  onEnable: () => void;
  onRefresh: () => void;
  onSwitchAccount: () => void;
  onCopyError: () => void;
}

export function deriveProviderReadiness(
  provider: ProviderDetail,
): ProviderReadiness {
  if (!provider.enabled) {
    return {
      state: "disabled",
      title: "Provider disabled",
      detail: "Enable this provider before CodexBar refreshes usage.",
    };
  }

  if (provider.lastError) {
    return {
      state: "error",
      title: "Refresh failed",
      detail: provider.lastError,
    };
  }

  if (!provider.hasSnapshot) {
    return {
      state: "waiting",
      title: "No usage snapshot yet",
      detail: "Refresh this provider after signing in or adding credentials.",
    };
  }

  return {
    state: "ready",
    title: "Ready",
    detail: provider.sourceLabel
      ? `Usage is refreshing from ${provider.sourceLabel}.`
      : "Usage is refreshing normally.",
  };
}

export function ProviderReadinessSection({
  provider,
  busy,
  onEnable,
  onRefresh,
  onSwitchAccount,
  onCopyError,
}: Props) {
  const readiness = deriveProviderReadiness(provider);
  const canSwitchAccount =
    Boolean(provider.dashboardUrl) &&
    (readiness.state === "waiting" || readiness.state === "error");

  return (
    <section
      className={`provider-readiness provider-readiness--${readiness.state}`}
    >
      <div className="provider-readiness__status" aria-hidden />
      <div className="provider-readiness__body">
        <strong>{readiness.title}</strong>
        <p>{readiness.detail}</p>
      </div>
      <div className="provider-readiness__actions">
        {readiness.state === "disabled" && (
          <button
            type="button"
            className="credential-btn credential-btn--primary"
            disabled={busy}
            onClick={onEnable}
          >
            Enable
          </button>
        )}
        {readiness.state !== "ready" && readiness.state !== "disabled" && (
          <button
            type="button"
            className="credential-btn credential-btn--primary"
            disabled={busy}
            onClick={onRefresh}
          >
            Refresh
          </button>
        )}
        {canSwitchAccount && (
          <button
            type="button"
            className="credential-btn"
            disabled={busy}
            onClick={onSwitchAccount}
          >
            Sign in
          </button>
        )}
        {readiness.state === "error" && (
          <button
            type="button"
            className="credential-btn"
            onClick={onCopyError}
          >
            Copy error
          </button>
        )}
      </div>
    </section>
  );
}
