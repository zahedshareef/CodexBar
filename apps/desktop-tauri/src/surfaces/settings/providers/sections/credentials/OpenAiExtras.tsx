import type { LocaleKey } from "../../../../../i18n/keys";

interface Props {
  t: (key: LocaleKey) => string;
}

/**
 * OpenAI/Codex-specific detail help.
 *
 * Port of the help strings below the `ProviderId::Codex` toggles in
 * `rust/src/native_ui/preferences.rs::render_provider_detail_panel` (~6625).
 * The toggles themselves (`codex_historical_tracking`,
 * `codex_openai_web_extras`) are not yet persisted through
 * `update_settings` in the Tauri bridge, so this component shows the
 * upstream hint copy only. The toggles will be surfaced once they join
 * the SettingsUpdate bridge (tracked alongside Phase 6e token-accounts).
 */
export function OpenAiExtras({ t }: Props) {
  return (
    <section className="provider-detail-section">
      <h4>{t("CredentialsSectionTitle")}</h4>
      <div className="provider-detail-helper">
        {t("ProviderCodexHistoryHelp")}
      </div>
      <div className="provider-detail-helper">
        {t("CredsOpenAiHistoryHelp")}
      </div>
    </section>
  );
}
