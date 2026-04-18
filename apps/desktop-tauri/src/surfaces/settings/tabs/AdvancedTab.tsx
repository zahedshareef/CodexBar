import { useLocale } from "../../../hooks/useLocale";
import { useUpdateState } from "../../../hooks/useUpdateState";
import { formatRelativeUpdated } from "../../../lib/relativeTime";
import { Field, NumberInput, Select, Toggle } from "../../../components/FormControls";
import type {
  MenuBarDisplayMode,
  TrayIconMode,
  UpdateChannel,
} from "../../../types/bridge";
import type { TabProps } from "../../Settings";

export default function AdvancedTab({ settings, set, saving }: TabProps) {
  const { t } = useLocale();
  const { updateState, checkNow } = useUpdateState();
  const lastCheckedDisplay = formatRelativeUpdated(
    updateState.lastCheckedAt,
    t,
  );

  return (
    <>
      {/* ── Refresh ───────────────────────────────────────────────── */}
      <section className="settings-section">
        <h3 className="settings-section__title">{t("SectionRefresh")}</h3>
        <div className="settings-section__group">
          <Field
            label={t("RefreshIntervalLabel")}
            description={t("RefreshIntervalHelper")}
          >
            <NumberInput
              value={settings.refreshIntervalSecs}
              min={0}
              max={3600}
              step={30}
              disabled={saving}
              onChange={(v) => set({ refreshIntervalSecs: v })}
            />
          </Field>
        </div>
      </section>

      {/* ── Menu Bar ──────────────────────────────────────────────── */}
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
          <Field label={t("ShowAsUsedLabel")} description={t("ShowAsUsedHelper")}>
            <Toggle
              checked={settings.showAsUsed}
              disabled={saving}
              onChange={(v) => set({ showAsUsed: v })}
            />
          </Field>
          <Field
            label={t("ShowAllTokenAccountsLabel")}
            description={t("ShowAllTokenAccountsHelper")}
          >
            <Toggle
              checked={settings.showAllTokenAccountsInMenu}
              disabled={saving}
              onChange={(v) => set({ showAllTokenAccountsInMenu: v })}
            />
          </Field>
        </div>
      </section>

      {/* ── Fun ───────────────────────────────────────────────────── */}
      <section className="settings-section">
        <h3 className="settings-section__title">{t("Fun")}</h3>
        <div className="settings-section__group">
          <Field
            label={t("EnableAnimationsLabel")}
            description={t("EnableAnimationsHelper")}
          >
            <Toggle
              checked={settings.enableAnimations}
              disabled={saving}
              onChange={(v) => set({ enableAnimations: v })}
            />
          </Field>
          <Field
            label={t("SurpriseAnimationsLabel")}
            description={t("SurpriseAnimationsHelper")}
          >
            <Toggle
              checked={settings.surpriseAnimations}
              disabled={saving}
              onChange={(v) => set({ surpriseAnimations: v })}
            />
          </Field>
        </div>
      </section>

      {/* ── Credentials & Security ───────────────────────────────── */}
      <section className="settings-section">
        <h3 className="settings-section__title">
          {t("SectionCredentialsSecurity")}
        </h3>
        <div className="settings-section__group">
          <Field
            label={t("AvoidKeychainPromptsLabel")}
            description={t("AvoidKeychainPromptsHelper")}
          >
            <Toggle
              checked={settings.claudeAvoidKeychainPrompts}
              disabled={saving || settings.disableKeychainAccess}
              onChange={(v) => set({ claudeAvoidKeychainPrompts: v })}
            />
          </Field>
          <Field
            label={t("DisableAllKeychainLabel")}
            description={t("DisableAllKeychainHelper")}
          >
            <Toggle
              checked={settings.disableKeychainAccess}
              disabled={saving}
              onChange={(v) => set({ disableKeychainAccess: v })}
            />
          </Field>
        </div>
      </section>

      {/* ── Debug ────────────────────────────────────────────────── */}
      <section className="settings-section">
        <h3 className="settings-section__title">{t("SectionDebug")}</h3>
        <div className="settings-section__group">
          <Field
            label={t("ShowDebugSettingsLabel")}
            description={t("ShowDebugSettingsHelper")}
          >
            <Toggle
              checked={settings.showDebugSettings}
              disabled={saving}
              onChange={(v) => set({ showDebugSettings: v })}
            />
          </Field>
        </div>
      </section>

      {/* ── Updates ──────────────────────────────────────────────── */}
      <section className="settings-section">
        <h3 className="settings-section__title">{t("Updates")}</h3>
        <div className="settings-section__group">
          <Field
            label={t("UpdateChannelChoice")}
            description={t("UpdateChannelChoiceHelper")}
          >
            <Select
              value={settings.updateChannel}
              disabled={saving}
              options={[
                { value: "stable", label: t("UpdateChannelStableOption") },
                { value: "beta", label: t("UpdateChannelBetaOption") },
              ]}
              onChange={(v) => set({ updateChannel: v as UpdateChannel })}
            />
          </Field>
          <Field
            label={t("AutoDownloadUpdates")}
            description={t("AutoDownloadUpdatesHelper")}
          >
            <Toggle
              checked={settings.autoDownloadUpdates}
              disabled={saving}
              onChange={(v) => set({ autoDownloadUpdates: v })}
            />
          </Field>
          <Field
            label={t("InstallUpdatesOnQuit")}
            description={t("InstallUpdatesOnQuitHelper")}
          >
            <Toggle
              checked={settings.installUpdatesOnQuit}
              disabled={saving}
              onChange={(v) => set({ installUpdatesOnQuit: v })}
            />
          </Field>
          <Field label={t("LastUpdated")}>
            <div className="settings-field__row">
              <span className="settings-field__value">
                {lastCheckedDisplay}
              </span>
              <button
                type="button"
                className="credential-btn"
                disabled={updateState.status === "checking"}
                onClick={() => checkNow()}
              >
                {t("TrayCheckForUpdates")}
              </button>
            </div>
          </Field>
        </div>
      </section>

      {/* ── Time ─────────────────────────────────────────────────── */}
      <section className="settings-section">
        <h3 className="settings-section__title">{t("SectionTime")}</h3>
        <div className="settings-section__group">
          <Field
            label={t("ResetTimeRelative")}
            description={t("ResetTimeRelativeHelper")}
          >
            <Toggle
              checked={settings.resetTimeRelative}
              disabled={saving}
              onChange={(v) => set({ resetTimeRelative: v })}
            />
          </Field>
        </div>
      </section>
    </>
  );
}
