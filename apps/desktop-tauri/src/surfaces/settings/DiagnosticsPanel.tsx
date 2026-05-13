import { useCallback, useMemo, useState } from "react";
import { getCredentialStorageStatus, getSafeDiagnostics } from "../../lib/tauri";
import type {
  CredentialStorageStatus,
  SafeDiagnostics,
} from "../../types/bridge";

interface DiagnosticsSnapshot {
  generatedAt: string;
  diagnostics: SafeDiagnostics;
  credentialStorage: CredentialStorageStatus;
}

function buildDiagnosticsPayload(snapshot: DiagnosticsSnapshot): string {
  return JSON.stringify(snapshot, null, 2);
}

async function writeClipboard(text: string): Promise<void> {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }

  const textarea = document.createElement("textarea");
  textarea.value = text;
  textarea.setAttribute("readonly", "true");
  textarea.style.position = "fixed";
  textarea.style.left = "-9999px";
  document.body.appendChild(textarea);
  textarea.select();
  try {
    if (!document.execCommand("copy")) {
      throw new Error("copy command failed");
    }
  } finally {
    document.body.removeChild(textarea);
  }
}

export function DiagnosticsPanel() {
  const [snapshot, setSnapshot] = useState<DiagnosticsSnapshot | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const payload = useMemo(
    () => (snapshot ? buildDiagnosticsPayload(snapshot) : ""),
    [snapshot],
  );

  const collect = useCallback(async (): Promise<string> => {
    setLoading(true);
    setStatus(null);
    try {
      const [diagnostics, credentialStorage] = await Promise.all([
        getSafeDiagnostics(),
        getCredentialStorageStatus(),
      ]);
      const nextSnapshot: DiagnosticsSnapshot = {
        generatedAt: new Date().toISOString(),
        diagnostics,
        credentialStorage,
      };
      setSnapshot(nextSnapshot);
      setStatus("Diagnostics ready.");
      return buildDiagnosticsPayload(nextSnapshot);
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      setStatus(`Diagnostics failed: ${message}`);
      throw err;
    } finally {
      setLoading(false);
    }
  }, []);

  const copy = useCallback(async () => {
    let text = payload;
    try {
      if (!text) {
        text = await collect();
      }
      await writeClipboard(text);
      setStatus("Diagnostics copied.");
    } catch {
      if (text) {
        setStatus("Clipboard unavailable; select the JSON below.");
      }
    }
  }, [collect, payload]);

  const diagnostics = snapshot?.diagnostics ?? null;
  const credentialStorage = snapshot?.credentialStorage ?? null;

  return (
    <section className="settings-section diagnostics-section">
      <h3 className="settings-section__title">Diagnostics</h3>
      <div className="diagnostics-panel">
        <div className="diagnostics-panel__header">
          <div>
            <h4>Provider diagnostics</h4>
            <p>Safe provider and credential-store metadata. No secrets.</p>
          </div>
          <div className="diagnostics-panel__actions">
            <button
              type="button"
              className="credential-btn"
              disabled={loading}
              onClick={() => void collect()}
            >
              {loading ? "Refreshing..." : diagnostics ? "Refresh" : "Run"}
            </button>
            <button
              type="button"
              className="credential-btn credential-btn--primary"
              disabled={loading}
              onClick={() => void copy()}
            >
              Copy JSON
            </button>
          </div>
        </div>

        <div className="diagnostics-panel__stats">
          <div>
            <span>Enabled</span>
            <strong>{diagnostics?.enabledProviders.length ?? "-"}</strong>
          </div>
          <div>
            <span>Manual cookies</span>
            <strong>{diagnostics?.hasManualCookies.length ?? "-"}</strong>
          </div>
          <div>
            <span>API keys</span>
            <strong>{diagnostics?.hasApiKeys.length ?? "-"}</strong>
          </div>
          <div>
            <span>Refresh</span>
            <strong>
              {diagnostics ? `${diagnostics.refreshIntervalSecs}s` : "-"}
            </strong>
          </div>
        </div>

        {credentialStorage && (
          <div className="diagnostics-panel__storage">
            <span>{credentialStorage.manualCookies}</span>
            <span>{credentialStorage.apiKeys}</span>
            <span>{credentialStorage.tokenAccounts}</span>
          </div>
        )}

        <textarea
          className="diagnostics-panel__output"
          readOnly
          spellCheck={false}
          value={payload || "Run diagnostics to generate a safe JSON snapshot."}
        />

        {status && <p className="settings-section__hint">{status}</p>}
      </div>
    </section>
  );
}
