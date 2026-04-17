import { useLocale } from "../../../hooks/useLocale";
import { Field, Toggle } from "../../../components/FormControls";
import type { TabProps } from "../../Settings";

export default function DisplayTab({ settings, set, saving }: TabProps) {
  const { t } = useLocale();
  return (
    <section className="settings-section">
      <h3 className="settings-section__title">{t("SectionUsageRendering")}</h3>
      <Field
        label={t("ShowCreditsExtra")}
        description={t("ShowCreditsExtraHelper")}
      >
        <Toggle
          checked={settings.showCreditsExtraUsage}
          disabled={saving}
          onChange={(v) => set({ showCreditsExtraUsage: v })}
        />
      </Field>

      <h3 className="settings-section__title">{t("PrivacyTitle")}</h3>
      <Field
        label={t("HidePersonalInfo")}
        description={t("HidePersonalInfoHelper")}
      >
        <Toggle
          checked={settings.hidePersonalInfo}
          disabled={saving}
          onChange={(v) => set({ hidePersonalInfo: v })}
        />
      </Field>
    </section>
  );
}
