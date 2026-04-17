import { useCallback, useEffect, useState } from "react";
import { useLocale } from "../../../hooks/useLocale";
import {
  getApiKeyProviders,
  getApiKeys,
  removeApiKey,
  setApiKey,
} from "../../../lib/tauri";
import type {
  ApiKeyInfoBridge,
  ApiKeyProviderInfoBridge,
  ProviderCatalogEntry,
} from "../../../types/bridge";

export default function ApiKeysTab({ providers }: { providers: ProviderCatalogEntry[] }) {
  const { t } = useLocale();
  const [keys, setKeys] = useState<ApiKeyInfoBridge[]>([]);
  const [apiKeyProviders, setApiKeyProviders] = useState<
    ApiKeyProviderInfoBridge[]
  >([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Which provider is currently being edited
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");
  const [editLabel, setEditLabel] = useState("");

  const reload = useCallback(async () => {
    try {
      const [k, p] = await Promise.all([getApiKeys(), getApiKeyProviders()]);
      setKeys(k);
      setApiKeyProviders(p);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const handleSave = async (providerId: string) => {
    if (!editValue.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const next = await setApiKey(
        providerId,
        editValue.trim(),
        editLabel.trim() || undefined,
      );
      setKeys(next);
      setEditingId(null);
      setEditValue("");
      setEditLabel("");
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleRemove = async (providerId: string) => {
    setBusy(true);
    setError(null);
    try {
      const next = await removeApiKey(providerId);
      setKeys(next);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  // Build a lookup of provider display names
  const providerNames = new Map(providers.map((p) => [p.id, p.displayName]));

  // Merge: show api-key providers with their saved state
  const keyMap = new Map(keys.map((k) => [k.providerId, k]));

  return (
    <section className="settings-section">
      <h3 className="settings-section__title">{t("SectionApiKeys")}</h3>
      <p className="settings-section__hint">{t("ApiKeysTabHint")}</p>

      {error && (
        <div className="settings-status settings-status--error">{error}</div>
      )}

      <ul className="credential-list">
        {apiKeyProviders.map((p) => {
          const saved = keyMap.get(p.id);
          const isEditing = editingId === p.id;
          const displayName = providerNames.get(p.id) ?? p.displayName;

          return (
            <li key={p.id} className="credential-card">
              <div className="credential-card__header">
                <div className="credential-card__info">
                  <strong>{displayName}</strong>
                  <span className="credential-card__meta">
                    {saved ? (
                      <>
                        <span className="credential-card__badge credential-card__badge--set">
                          Configured
                        </span>
                        <span className="credential-card__masked">
                          {saved.maskedKey}
                        </span>
                        {saved.label && (
                          <span className="credential-card__label">
                            {saved.label}
                          </span>
                        )}
                        <span className="credential-card__date">
                          Saved {saved.savedAt}
                        </span>
                      </>
                    ) : (
                      <span className="credential-card__badge credential-card__badge--unset">
                        Not set
                      </span>
                    )}
                  </span>
                </div>
                <div className="credential-card__actions">
                  {!isEditing && (
                    <button
                      className="credential-btn"
                      disabled={busy}
                      onClick={() => {
                        setEditingId(p.id);
                        setEditValue("");
                        setEditLabel(saved?.label ?? "");
                      }}
                    >
                      {saved ? "Update" : "Add Key"}
                    </button>
                  )}
                  {saved && !isEditing && (
                    <button
                      className="credential-btn credential-btn--danger"
                      disabled={busy}
                      onClick={() => void handleRemove(p.id)}
                    >
                      Remove
                    </button>
                  )}
                </div>
              </div>

              {p.help && !isEditing && (
                <p className="credential-card__help">{p.help}</p>
              )}

              {p.dashboardUrl && !isEditing && (
                <a
                  className="credential-card__link"
                  href={p.dashboardUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  Open dashboard ↗
                </a>
              )}

              {isEditing && (
                <div className="credential-card__edit">
                  <input
                    type="password"
                    className="text-input credential-card__input"
                    placeholder="Paste API key…"
                    autoComplete="off"
                    value={editValue}
                    onChange={(e) => setEditValue(e.target.value)}
                    disabled={busy}
                  />
                  <input
                    type="text"
                    className="text-input credential-card__input credential-card__input--label"
                    placeholder="Label (optional)"
                    value={editLabel}
                    onChange={(e) => setEditLabel(e.target.value)}
                    disabled={busy}
                  />
                  <div className="credential-card__edit-actions">
                    <button
                      className="credential-btn credential-btn--primary"
                      disabled={busy || !editValue.trim()}
                      onClick={() => void handleSave(p.id)}
                    >
                      Save
                    </button>
                    <button
                      className="credential-btn"
                      disabled={busy}
                      onClick={() => {
                        setEditingId(null);
                        setEditValue("");
                        setEditLabel("");
                      }}
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              )}
            </li>
          );
        })}
      </ul>
    </section>
  );
}
