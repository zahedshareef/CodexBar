import { useLocale } from "../../../hooks/useLocale";
import { Field, Select, Toggle } from "../../../components/FormControls";
import type {
  Language,
  MenuBarDisplayMode,
  ThemePreference,
  TrayIconMode,
} from "../../../types/bridge";
import type { TabProps } from "../../Settings";

export default function DisplayTab({ settings, set, saving }: TabProps) {
  const { t } = useLocale();
  return (
    <>
      {/* ── Menu bar ─────────────────────────────────────────────── */}
      <section className="settings-section">
        <h3 className="settings-section__title">{t("MenuBar")}</h3>
        <div className="settings-section__group">
          <Field
            label={t("TrayIconModeLabel")}
            description={t("TrayIconModeHelper")}
          >
            <Select
              value={settings.trayIconMode}
              disabled={saving}
              options={[
                { value: "single", label: t("TrayIconModeSingle") },
                { value: "perProvider", label: t("TrayIconModePerProvider") },
              ]}
              onChange={(v) => set({ trayIconMode: v as TrayIconMode })}
            />
          </Field>
          <Field
            label={t("ShowProviderIcons")}
            description={t("ShowProviderIconsHelper")}
            leading
          >
            <Toggle
              checked={settings.switcherShowsIcons}
              disabled={saving}
              onChange={(v) => set({ switcherShowsIcons: v })}
            />
          </Field>
          <Field
            label={t("PreferHighestUsage")}
            description={t("PreferHighestUsageHelper")}
            leading
          >
            <Toggle
              checked={settings.menuBarShowsHighestUsage}
              disabled={saving}
              onChange={(v) => set({ menuBarShowsHighestUsage: v })}
            />
          </Field>
          <Field
            label={t("ShowPercentInTray")}
            description={t("ShowPercentInTrayHelper")}
            leading
          >
            <Toggle
              checked={settings.menuBarShowsPercent}
              disabled={saving}
              onChange={(v) => set({ menuBarShowsPercent: v })}
            />
          </Field>
          <Field
            label={t("DisplayModeLabel")}
            description={t("DisplayModeHelper")}
          >
            <Select
              value={settings.menuBarDisplayMode}
              disabled={saving}
              options={[
                { value: "detailed", label: t("DisplayModeDetailed") },
                { value: "compact", label: t("DisplayModeCompact") },
                { value: "minimal", label: t("DisplayModeMinimal") },
              ]}
              onChange={(v) =>
                set({ menuBarDisplayMode: v as MenuBarDisplayMode })
              }
            />
          </Field>
        </div>
      </section>

      {/* ── Menu content ─────────────────────────────────────────── */}
      <section className="settings-section">
        <h3 className="settings-section__title">Menu Content</h3>
        <div className="settings-section__group">
          <Field
            label={t("ShowAsUsedLabel")}
            description={t("ShowAsUsedHelper")}
            leading
          >
            <Toggle
              checked={settings.showAsUsed}
              disabled={saving}
              onChange={(v) => set({ showAsUsed: v })}
            />
          </Field>
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
          <Field
            label={t("ShowAllTokenAccountsLabel")}
            description={t("ShowAllTokenAccountsHelper")}
            leading
          >
            <Toggle
              checked={settings.showAllTokenAccountsInMenu}
              disabled={saving}
              onChange={(v) => set({ showAllTokenAccountsInMenu: v })}
            />
          </Field>
          <Field
            label={t("ResetTimeRelative")}
            description={t("ResetTimeRelativeHelper")}
            leading
          >
            <Toggle
              checked={settings.resetTimeRelative}
              disabled={saving}
              onChange={(v) => set({ resetTimeRelative: v })}
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
