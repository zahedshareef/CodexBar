import { useEffect, useState } from "react";
import type { LocaleKey } from "../../../../../i18n/keys";
import type { GeminiCliStatus } from "../../../../../types/bridge";
import {
  getGeminiCliSignedIn,
  openPath,
  openProviderDashboard,
} from "../../../../../lib/tauri";

interface Props {
  providerId: string;
  t: (key: LocaleKey) => string;
}

/**
 * Gemini CLI credentials row.
 *
 * Port of the `ProviderId::Gemini` branch in
 * `rust/src/native_ui/preferences.rs::render_provider_detail_panel` (~5570).
 * Shows OAuth-credential presence + path + a button that opens the
 * credentials folder (when signed in) or a hint to install the CLI.
 */
export function GeminiCliCreds({ providerId, t }: Props) {
  const [status, setStatus] = useState<GeminiCliStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    getGeminiCliSignedIn()
      .then((s) => !cancelled && setStatus(s))
      .catch((e) => !cancelled && setError(String(e)));
    return () => {
      cancelled = true;
    };
  }, []);

  if (!status) return null;

  const statusLabel = status.signedIn
    ? t("CredsStatusAuthenticated")
    : t("CredsStatusNotSignedIn");

  const handleOpenFolder = () => {
    if (!status.credentialsPath) return;
    void openPath(status.credentialsPath).catch((e) => setError(String(e)));
  };

  const handleSetup = () => {
    // No CLI-install auto-flow; we open the upstream project page via the
    // provider's dashboard invariant (Gemini provider advertises it).
    void openProviderDashboard(providerId).catch((e) => setError(String(e)));
  };

  return (
    <section className="provider-detail-section">
      <h4>{t("CredentialsSectionTitle")}</h4>
      <dl className="provider-detail-grid">
        <div style={{ display: "contents" }}>
          <dt>{t("CredsGeminiCliLabel")}</dt>
          <dd>{statusLabel}</dd>
        </div>
        {status.credentialsPath && (
          <div style={{ display: "contents" }}>
            <dt>{t("CredsGeminiCliHelperPrefix")}</dt>
            <dd className="provider-detail-grid__mono">
              {status.credentialsPath}
            </dd>
          </div>
        )}
      </dl>
      {!status.signedIn && (
        <div className="provider-detail-helper">
          {t("CredsGeminiCliSetupHelp")}
        </div>
      )}
      <div className="provider-detail-actions">
        {status.signedIn && status.credentialsPath && (
          <button
            type="button"
            className="btn btn--ghost"
            onClick={handleOpenFolder}
          >
            {t("CredsOpenFolderAction")}
          </button>
        )}
        {!status.signedIn && (
          <button type="button" className="btn btn--ghost" onClick={handleSetup}>
            {t("CredsGeminiCliSetupAction")}
          </button>
        )}
      </div>
      {error && <div className="provider-detail-error">{error}</div>}
    </section>
  );
}
