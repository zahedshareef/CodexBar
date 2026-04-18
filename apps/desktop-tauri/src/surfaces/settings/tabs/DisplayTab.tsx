import { useLocale } from "../../../hooks/useLocale";
import { Field, Select, Toggle } from "../../../components/FormControls";
import type {
  Language,
  ThemePreference,
} from "../../../types/bridge";
import type { TabProps } from "../../Settings";

export default function DisplayTab({ settings, set, saving }: TabProps) {
  const { t } = useLocale();
  return (
    <>
      <section className="settings-section">
        <h3 className="settings-section__title">
          {t("SectionUsageRendering")}
        </h3>
        <div className="settings-section__group">
          <Field
            label={t("ShowCreditsExtra")}
            description={t("ShowCreditsExtraHelper")}
            leading
          >
            <Toggle
              checked={settings.showCreditsExtraUsage}
              disabled={saving}
              onChange={(v) => set({ showCreditsExtraUsage: v })}
            />
          </Field>
        </div>
      </section>

      <section className="settings-section">
        <h3 className="settings-section__title">{t("PrivacyTitle")}</h3>
        <div className="settings-section__group">
          <Field
            label={t("HidePersonalInfo")}
            description={t("HidePersonalInfoHelper")}
            leading
          >
            <Toggle
              checked={settings.hidePersonalInfo}
              disabled={saving}
              onChange={(v) => set({ hidePersonalInfo: v })}
            />
          </Field>
        </div>
      </section>

      {/* ── Appearance ───────────────────────────────────────────── */}
      <section className="settings-section">
        <h3 className="settings-section__title">{t("SectionTheme")}</h3>
        <div className="settings-section__group">
          <Field label={t("ThemeLabel")} description={t("ThemeHelper")}>
            <Select
              value={settings.theme}
              disabled={saving}
              options={[
                { value: "auto", label: t("ThemeAutoOption") },
                { value: "light", label: t("ThemeLightOption") },
                { value: "dark", label: t("ThemeDarkOption") },
              ]}
              onChange={(v) => set({ theme: v as ThemePreference })}
            />
          </Field>
        </div>
      </section>

      {/* ── Language ─────────────────────────────────────────────── */}
      <section className="settings-section">
        <h3 className="settings-section__title">{t("SectionLanguage")}</h3>
        <div className="settings-section__group">
          <Field label={t("InterfaceLanguage")}>
            <Select
              value={settings.uiLanguage}
              disabled={saving}
              options={[
                { value: "english", label: t("LanguageEnglishOption") },
                { value: "chinese", label: t("LanguageChineseOption") },
              ]}
              onChange={(v) => set({ uiLanguage: v as Language })}
            />
          </Field>
        </div>
      </section>
    </>
  );
}
