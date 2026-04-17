import { useEffect, useState } from "react";
import type { LocaleKey } from "../../../../../i18n/keys";
import type { KiroStatus } from "../../../../../types/bridge";
import { getKiroStatus, openPath } from "../../../../../lib/tauri";

interface Props {
  t: (key: LocaleKey) => string;
}

/**
 * Kiro CLI availability row.
 *
 * Backed by the Phase-4 `get_kiro_status` IPC which returns availability
 * plus a hint string (either the detected CLI path or a not-found error).
 */
export function KiroCreds({ t }: Props) {
  const [status, setStatus] = useState<KiroStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    getKiroStatus()
      .then((s) => !cancelled && setStatus(s))
      .catch((e) => !cancelled && setError(String(e)));
    return () => {
      cancelled = true;
    };
  }, []);

  if (!status) return null;

  const statusLabel = status.available
    ? t("CredsStatusAvailable")
    : t("CredsStatusUnavailable");

  const handleOpenFolder = () => {
    if (!status.hint) return;
    void openPath(status.hint).catch((e) => setError(String(e)));
  };

  return (
    <section className="provider-detail-section">
      <h4>{t("CredentialsSectionTitle")}</h4>
      <dl className="provider-detail-grid">
        <div style={{ display: "contents" }}>
          <dt>{t("CredsKiroLabel")}</dt>
          <dd>{statusLabel}</dd>
        </div>
        {status.available && status.hint && (
          <div style={{ display: "contents" }}>
            <dt>{t("CredsKiroHelperAvailablePrefix")}</dt>
            <dd className="provider-detail-grid__mono">{status.hint}</dd>
          </div>
        )}
      </dl>
      {!status.available && (
        <div className="provider-detail-helper">
          {t("CredsKiroHelperMissing")}
        </div>
      )}
      {status.available && status.hint && (
        <div className="provider-detail-actions">
          <button
            type="button"
            className="btn btn--ghost"
            onClick={handleOpenFolder}
          >
            {t("CredsOpenFolderAction")}
          </button>
        </div>
      )}
      {error && <div className="provider-detail-error">{error}</div>}
    </section>
  );
}
