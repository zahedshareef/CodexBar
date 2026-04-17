import { useCallback, useState } from "react";
import { useLocale } from "../../../hooks/useLocale";
import {
  playNotificationSound,
  registerGlobalShortcut,
  unregisterGlobalShortcut,
} from "../../../lib/tauri";
import { ShortcutCapture } from "../../../components/ShortcutCapture";
import { Field, NumberInput, Toggle } from "../../../components/FormControls";
import type { TabProps } from "../../Settings";

export default function GeneralTab({ settings, set, saving }: TabProps) {
  const { t } = useLocale();
  const [playingSound, setPlayingSound] = useState(false);
  const [shortcutError, setShortcutError] = useState<string | null>(null);

  const handleTestSound = useCallback(() => {
    setPlayingSound(true);
    void playNotificationSound().catch(() => {});
    window.setTimeout(() => setPlayingSound(false), 1500);
  }, []);

  const commitShortcut = useCallback(
    async (accelerator: string) => {
      setShortcutError(null);
      try {
        // Best-effort capture registration (emits global-shortcut-triggered).
        // Persisting via update_settings re-registers with the default
        // window-toggle handler, which is what we ultimately want.
        await registerGlobalShortcut(accelerator).catch(() => {});
        set({ globalShortcut: accelerator });
      } catch (err: unknown) {
        setShortcutError(err instanceof Error ? err.message : String(err));
      }
    },
    [set],
  );

  const clearShortcut = useCallback(async () => {
    setShortcutError(null);
    try {
      await unregisterGlobalShortcut().catch(() => {});
      set({ globalShortcut: "" });
    } catch (err: unknown) {
      setShortcutError(err instanceof Error ? err.message : String(err));
    }
  }, [set]);

  return (
    <section className="settings-section">
      <h3 className="settings-section__title">{t("StartupSettings")}</h3>
      <Field label={t("StartAtLogin")} description={t("StartAtLoginHelper")}>
        <Toggle
          checked={settings.startAtLogin}
          disabled={saving}
          onChange={(v) => set({ startAtLogin: v })}
        />
      </Field>
      <Field label={t("StartMinimized")} description={t("StartMinimizedHelper")}>
        <Toggle
          checked={settings.startMinimized}
          disabled={saving}
          onChange={(v) => set({ startMinimized: v })}
        />
      </Field>

      {/* Refresh interval lives on the Advanced tab (Phase 8). */}
      {/* TODO(Phase 7): General tab may need re-layout after Refresh move */}

      <h3 className="settings-section__title">{t("SectionNotifications")}</h3>
      <Field
        label={t("ShowNotifications")}
        description={t("ShowNotificationsHelper")}
      >
        <Toggle
          checked={settings.showNotifications}
          disabled={saving}
          onChange={(v) => set({ showNotifications: v })}
        />
      </Field>
      <Field label={t("SoundEnabled")} description={t("SoundEnabledHelper")}>
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

      <h3 className="settings-section__title">{t("SectionUsageThresholds")}</h3>
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

      <h3 className="settings-section__title">{t("SectionKeyboard")}</h3>
      <Field
        label={t("GlobalShortcutFieldLabel")}
        description={t("GlobalShortcutToggleHelper")}
      >
        <ShortcutCapture
          value={settings.globalShortcut}
          disabled={saving}
          onCommit={(accel) => void commitShortcut(accel)}
          onClear={() => void clearShortcut()}
        />
      </Field>
      {shortcutError && (
        <p className="settings-section__error">{shortcutError}</p>
      )}
      <p className="settings-section__hint">{t("ShortcutRecordingHint")}</p>
    </section>
  );
}
