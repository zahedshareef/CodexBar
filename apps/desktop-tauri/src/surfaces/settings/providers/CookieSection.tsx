import { useCallback, useEffect, useState } from "react";
import {
  getManualCookies,
  importBrowserCookies,
  listDetectedBrowsers,
  removeManualCookie,
  setManualCookie,
} from "../../../lib/tauri";
import { Select } from "../../../components/FormControls";
import type {
  CookieInfoBridge,
  DetectedBrowserBridge,
} from "../../../types/bridge";

interface Props {
  providerId: string;
  cookieDomain: string | null;
}

/**
 * Per-provider browser cookie management. Renders nothing for providers
 * that do not have a cookieDomain (i.e. don't authenticate via web cookies).
 */
export function CookieSection({ providerId, cookieDomain }: Props) {
  const [saved, setSaved] = useState<CookieInfoBridge | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [browsers, setBrowsers] = useState<DetectedBrowserBridge[]>([]);
  const [browsersLoaded, setBrowsersLoaded] = useState(false);
  const [browserType, setBrowserType] = useState("");
  const [importStatus, setImportStatus] = useState<string | null>(null);
  const [importError, setImportError] = useState<string | null>(null);

  const [pasteValue, setPasteValue] = useState("");

  const reload = useCallback(async (signal: { stale: boolean }) => {
    try {
      const cookies = await getManualCookies();
      if (signal.stale) return;
      setSaved(cookies.find((c) => c.providerId === providerId) ?? null);
    } catch (err: unknown) {
      if (signal.stale) return;
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      if (!signal.stale) setLoaded(true);
    }
  }, [providerId]);

  useEffect(() => {
    if (cookieDomain === null) return;
    const signal = { stale: false };
    setLoaded(false);
    setError(null);
    setImportError(null);
    setImportStatus(null);
    setPasteValue("");
    void reload(signal);
    return () => { signal.stale = true; };
  }, [reload, cookieDomain]);

  useEffect(() => {
    if (cookieDomain === null) return;
    listDetectedBrowsers()
      .then((list) => {
        setBrowsers(list);
        setBrowsersLoaded(true);
        if (list.length > 0) setBrowserType(list[0].browserType);
      })
      .catch(() => {
        setBrowsersLoaded(true);
      });
  }, [cookieDomain]);

  if (cookieDomain === null) return null;
  if (!loaded) return null;

  const handleRemove = async () => {
    setBusy(true);
    setError(null);
    try {
      const next = await removeManualCookie(providerId);
      setSaved(next.find((c) => c.providerId === providerId) ?? null);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleImport = async () => {
    if (!browserType) return;
    setBusy(true);
    setImportError(null);
    setImportStatus(null);
    try {
      const next = await importBrowserCookies(providerId, browserType);
      setSaved(next.find((c) => c.providerId === providerId) ?? null);
      setImportStatus("Cookies imported successfully.");
    } catch (err: unknown) {
      setImportError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const handlePaste = async () => {
    if (!pasteValue.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const next = await setManualCookie(providerId, pasteValue.trim());
      setSaved(next.find((c) => c.providerId === providerId) ?? null);
      setPasteValue("");
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="provider-detail-section">
      <h4>Browser Cookies</h4>

      {error && (
        <div className="settings-status settings-status--error">{error}</div>
      )}

      {saved ? (
        <ul className="credential-list">
          <li className="credential-card">
            <div className="credential-card__header">
              <div className="credential-card__info">
                <span className="credential-card__meta">
                  <span className="credential-card__badge credential-card__badge--set">
                    Saved
                  </span>
                  <span className="credential-card__date">{saved.savedAt}</span>
                </span>
              </div>
              <div className="credential-card__actions">
                <button
                  className="credential-btn credential-btn--danger"
                  disabled={busy}
                  onClick={() => void handleRemove()}
                >
                  Remove
                </button>
              </div>
            </div>
          </li>
        </ul>
      ) : (
        <p className="credential-empty">No cookie saved.</p>
      )}

      {browsersLoaded && browsers.length > 0 && (
        <>
          {importError && (
            <div className="settings-status settings-status--error">
              {importError}
            </div>
          )}
          {importStatus && (
            <div className="settings-status settings-status--ok">
              {importStatus}
            </div>
          )}
          <div className="credential-add-form">
            <Select
              value={browserType}
              options={browsers.map((b) => ({
                value: b.browserType,
                label: `${b.displayName} (${b.profileCount} profile${b.profileCount !== 1 ? "s" : ""})`,
              }))}
              onChange={setBrowserType}
              disabled={busy}
            />
            <button
              className="credential-btn credential-btn--primary"
              disabled={busy || !browserType}
              onClick={() => void handleImport()}
            >
              Import from Browser
            </button>
          </div>
        </>
      )}

      <div className="credential-add-form">
        <textarea
          className="text-input credential-textarea"
          placeholder="Paste cookie header value…"
          rows={3}
          value={pasteValue}
          onChange={(e) => setPasteValue(e.target.value)}
          disabled={busy}
        />
        <button
          className="credential-btn credential-btn--primary"
          disabled={busy || !pasteValue.trim()}
          onClick={() => void handlePaste()}
        >
          Save Cookie
        </button>
      </div>
    </section>
  );
}
