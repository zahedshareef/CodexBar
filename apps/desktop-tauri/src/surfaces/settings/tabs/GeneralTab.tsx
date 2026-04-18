import { useCallback, useState } from "react";
import { useLocale } from "../../../hooks/useLocale";
import { playNotificationSound, quitApp } from "../../../lib/tauri";
import { Field, NumberInput, Toggle } from "../../../components/FormControls";
import type { TabProps } from "../../Settings";

export default function GeneralTab({ settings, set, saving }: TabProps) {
  const { t } = useLocale();
  const [playingSound, setPlayingSound] = useState(false);

  const handleTestSound = useCallback(() => {
    setPlayingSound(true);
    void playNotificationSound().catch(() => {});
    window.setTimeout(() => setPlayingSound(false), 1500);
  }, []);

  const handleQuit = useCallback(() => {
    void quitApp().catch(() => window.close());
  }, []);

  return (
    <>
      <section className="settings-section">
        <h3 className="settings-section__title">{t("StartupSettings")}</h3>
        <div className="settings-section__group">
          <Field label={t("StartAtLogin")} description={t("StartAtLoginHelper")} leading>
            <Toggle
              checked={settings.startAtLogin}
              disabled={saving}
              onChange={(v) => set({ startAtLogin: v })}
            />
          </Field>
          <Field
            label={t("StartMinimized")}
            description={t("StartMinimizedHelper")}
            leading
          >
            <Toggle
              checked={settings.startMinimized}
              disabled={saving}
              onChange={(v) => set({ startMinimized: v })}
            />
          </Field>
        </div>
      </section>

      <section className="settings-section">
        <h3 className="settings-section__title">
          {t("SectionNotifications")}
        </h3>
        <div className="settings-section__group">
          <Field
            label={t("ShowNotifications")}
            description={t("ShowNotificationsHelper")}
            leading
          >
            <Toggle
              checked={settings.showNotifications}
              disabled={saving}
              onChange={(v) => set({ showNotifications: v })}
            />
          </Field>
          <Field label={t("SoundEnabled")} description={t("SoundEnabledHelper")} leading>
            <div className="sound-enabled-row">
              <Toggle
                checked={settings.soundEnabled}
                disabled={saving}
                onChange={(v) => set({ soundEnabled: v })}
              />
              <button
                type="button"
                className="shortcut-capture__button shortcut-capture__button--ghost"
                disabled={saving || !settings.soundEnabled || playingSound}
                onClick={handleTestSound}
              >
                {playingSound
                  ? t("NotificationTestSoundPlaying")
                  : t("NotificationTestSound")}
              </button>
            </div>
          </Field>
          {settings.soundEnabled && (
            <Field label={t("SoundVolume")} description={t("SoundVolumeHelper")}>
              <NumberInput
                value={settings.soundVolume}
                min={0}
                max={100}
                step={5}
                disabled={saving}
                onChange={(v) => set({ soundVolume: v })}
              />
            </Field>
          )}
        </div>
      </section>

      <section className="settings-section">
        <h3 className="settings-section__title">
          {t("SectionUsageThresholds")}
        </h3>
        <div className="settings-section__group">
          <Field
            label={t("HighUsageAlert")}
            description={t("HighUsageWarningHelper")}
          >
            <NumberInput
              value={settings.highUsageThreshold}
              min={0}
              max={100}
              step={5}
              disabled={saving}
              onChange={(v) => set({ highUsageThreshold: v })}
            />
          </Field>
          <Field
            label={t("CriticalUsageAlert")}
            description={t("CriticalUsageWarningHelper")}
          >
            <NumberInput
              value={settings.criticalUsageThreshold}
              min={0}
              max={100}
              step={5}
              disabled={saving}
              onChange={(v) => set({ criticalUsageThreshold: v })}
            />
          </Field>
        </div>
      </section>

      {/* ── Automation ───────────────────────────────────────────── */}
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

      {/* ── Quit ─────────────────────────────────────────────────── */}
      <section className="settings-section">
        <div className="settings-quit-row">
          <button
            type="button"
            className="credential-btn credential-btn--primary"
            onClick={handleQuit}
          >
            {t("TrayQuit")}
          </button>
        </div>
      </section>
    </>
  );
}
