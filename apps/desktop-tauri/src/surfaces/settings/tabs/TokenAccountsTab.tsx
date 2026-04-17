import { useEffect, useState } from "react";
import { useLocale } from "../../../hooks/useLocale";
import { getTokenAccountProviders } from "../../../lib/tauri";
import { Select } from "../../../components/FormControls";
import { TokenAccountsPanel } from "../tokens/TokenAccountsPanel";
import type { TokenAccountSupportBridge } from "../../../types/bridge";

export default function TokenAccountsTab() {
  const { t } = useLocale();
  const [providers, setProviders] = useState<TokenAccountSupportBridge[]>([]);
  const [selectedProviderId, setSelectedProviderId] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getTokenAccountProviders()
      .then(setProviders)
      .catch((err: unknown) =>
        setError(err instanceof Error ? err.message : String(err)),
      );
  }, []);

  return (
    <section className="settings-section">
      <h3 className="settings-section__title">{t("SectionTokenAccounts")}</h3>
      <p className="settings-section__hint">{t("TokenAccountTabHint")}</p>

      {error && (
        <div className="settings-status settings-status--error">{error}</div>
      )}

      {providers.length === 0 ? (
        <p className="credential-empty">{t("TokenAccountNoSupported")}</p>
      ) : (
        <>
          <div className="settings-section__group">
            <div className="settings-field">
              <div className="settings-field__text">
                <span className="settings-field__label">
                  {t("TokenAccountProviderLabel")}
                </span>
              </div>
              <div className="settings-field__control">
                <Select
                  value={selectedProviderId}
                  options={[
                    {
                      value: "",
                      label: t("TokenAccountProviderPlaceholder"),
                    },
                    ...providers.map((p) => ({
                      value: p.providerId,
                      label: p.displayName,
                    })),
                  ]}
                  onChange={(v) => {
                    setSelectedProviderId(v);
                    setError(null);
                  }}
                />
              </div>
            </div>
          </div>

          {selectedProviderId && (
            <TokenAccountsPanel
              providerId={selectedProviderId}
              compact={false}
            />
          )}
        </>
      )}
    </section>
  );
}
