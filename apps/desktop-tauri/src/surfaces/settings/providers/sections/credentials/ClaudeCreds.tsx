import { useEffect, useState } from "react";
import type { LocaleKey } from "../../../../../i18n/keys";
import { getSettingsSnapshot, updateSettings } from "../../../../../lib/tauri";

interface Props {
  t: (key: LocaleKey) => string;
}

/**
 * Claude-specific credential options.
 *
 * Port of the `ProviderId::Claude` branch of the "Options" block in
 * `rust/src/native_ui/preferences.rs::render_provider_detail_panel` (~6662).
 * Currently exposes the "Avoid keychain prompts" toggle. The broader
 * `disable_keychain_access` master switch lives in the Advanced tab and
 * is intentionally not duplicated here.
 */
export function ClaudeCreds({ t }: Props) {
  const [value, setValue] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let cancelled = false;
    getSettingsSnapshot()
      .then((s) => !cancelled && setValue(s.claudeAvoidKeychainPrompts))
      .catch((e) => !cancelled && setError(String(e)));
    return () => {
      cancelled = true;
    };
  }, []);

  const toggle = async (next: boolean) => {
    setSaving(true);
    try {
      const updated = await updateSettings({
        claudeAvoidKeychainPrompts: next,
      });
      setValue(updated.claudeAvoidKeychainPrompts);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  if (value === null) return null;

  return (
    <section className="provider-detail-section">
      <h4>{t("CredentialsSectionTitle")}</h4>
      <label className="provider-detail-toggle">
        <input
          type="checkbox"
          checked={value}
          disabled={saving}
          onChange={(e) => void toggle(e.target.checked)}
        />
        <span>
          <span className="provider-detail-toggle__label">
            {t("ProviderClaudeAvoidKeychainPrompts")}
          </span>
          <span className="provider-detail-toggle__helper">
            {t("ProviderClaudeAvoidKeychainPromptsHelp")}
          </span>
        </span>
      </label>
      {error && <div className="provider-detail-error">{error}</div>}
    </section>
  );
}
