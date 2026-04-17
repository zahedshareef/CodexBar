import { useEffect, useState } from "react";
import type { LocaleKey } from "../../../../../i18n/keys";
import type { VertexAiStatus } from "../../../../../types/bridge";
import {
  getVertexAiStatus,
  openPath,
  openProviderDashboard,
} from "../../../../../lib/tauri";

interface Props {
  providerId: string;
  t: (key: LocaleKey) => string;
}

/**
 * Vertex AI / Google Cloud credentials row.
 *
 * Port of the `ProviderId::VertexAI` branch in
 * `rust/src/native_ui/preferences.rs::render_provider_detail_panel` (~5611).
 */
export function VertexAiCreds({ providerId, t }: Props) {
  const [status, setStatus] = useState<VertexAiStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    getVertexAiStatus()
      .then((s) => !cancelled && setStatus(s))
      .catch((e) => !cancelled && setError(String(e)));
    return () => {
      cancelled = true;
    };
  }, []);

  if (!status) return null;

  const statusLabel = status.hasCredentials
    ? t("CredsStatusAuthenticated")
    : t("CredsStatusNotSignedIn");

  const handleOpenFolder = () => {
    if (!status.credentialsPath) return;
    void openPath(status.credentialsPath).catch((e) => setError(String(e)));
  };

  const handleSetup = () => {
    void openProviderDashboard(providerId).catch((e) => setError(String(e)));
  };

  return (
    <section className="provider-detail-section">
      <h4>{t("CredentialsSectionTitle")}</h4>
      <dl className="provider-detail-grid">
        <div style={{ display: "contents" }}>
          <dt>{t("CredsVertexAiLabel")}</dt>
          <dd>{statusLabel}</dd>
        </div>
        {status.credentialsPath && (
          <div style={{ display: "contents" }}>
            <dt>{t("CredsVertexAiHelperPrefix")}</dt>
            <dd className="provider-detail-grid__mono">
              {status.credentialsPath}
            </dd>
          </div>
        )}
      </dl>
      {!status.hasCredentials && (
        <div className="provider-detail-helper">
          {t("CredsVertexAiSetupHelp")}
        </div>
      )}
      <div className="provider-detail-actions">
        {status.hasCredentials && status.credentialsPath && (
          <button
            type="button"
            className="btn btn--ghost"
            onClick={handleOpenFolder}
          >
            {t("CredsOpenFolderAction")}
          </button>
        )}
        {!status.hasCredentials && (
          <button type="button" className="btn btn--ghost" onClick={handleSetup}>
            {t("CredsVertexAiSetupAction")}
          </button>
        )}
      </div>
      {error && <div className="provider-detail-error">{error}</div>}
    </section>
  );
}
