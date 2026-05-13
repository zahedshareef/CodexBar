import { useCallback, useState } from "react";
import { useLocale } from "../../../hooks/useLocale";
import { playNotificationSound, quitApp } from "../../../lib/tauri";
import { Field, NumberInput, Select, Toggle } from "../../../components/FormControls";
import type { TabProps } from "../../Settings";

const FIRST_RUN_SETUP_DISMISSED_KEY = "codexbar:first-run-setup-dismissed";
const FIRST_RUN_PROVIDER_DEFAULTS = ["codex", "claude", "gemini"];

const REFRESH_CADENCE_OPTIONS: { value: string; label: string }[] = [
  { value: "0", label: "Manual" },
  { value: "60", label: "1 minute" },
  { value: "300", label: "5 minutes" },
  { value: "900", label: "15 minutes" },
  { value: "1800", label: "30 minutes" },
  { value: "3600", label: "1 hour" },
];

function readFirstRunDismissed(): boolean {
  try {
    return window.localStorage.getItem(FIRST_RUN_SETUP_DISMISSED_KEY) === "1";
  } catch {
    return false;
  }
}

function writeFirstRunDismissed(): void {
  try {
    window.localStorage.setItem(FIRST_RUN_SETUP_DISMISSED_KEY, "1");
  } catch {
    // Non-critical; the card can reappear if storage is unavailable.
  }
}

function FirstRunSetup({ settings, set, saving, openTab }: TabProps) {
  const [dismissed, setDismissed] = useState(readFirstRunDismissed);
  const enabledCount = settings.enabledProviders.length;
  const hasCliDefaults = FIRST_RUN_PROVIDER_DEFAULTS.every((provider) =>
    settings.enabledProviders.includes(provider),
  );

  if (dismissed) {
    return null;
  }

  const enableCliDefaults = () => {
    const next = Array.from(
      new Set([...settings.enabledProviders, ...FIRST_RUN_PROVIDER_DEFAULTS]),
    ).sort();
    set({ enabledProviders: next });
  };

  const dismiss = () => {
    writeFirstRunDismissed();
    setDismissed(true);
  };

  return (
    <section className="settings-section first-run-setup">
      <div className="first-run-setup__header">
        <div className="first-run-setup__copy">
          <h3>First-run setup</h3>
          <p>
            Enable the providers you use, then keep diagnostics ready for
            provider refresh issues.
          </p>
        </div>
        <button
          type="button"
          className="credential-btn"
          onClick={dismiss}
          aria-label="Dismiss first-run setup"
        >
          Dismiss
        </button>
      </div>

      <div className="first-run-setup__actions">
        <button
          type="button"
          className="first-run-setup__action"
          onClick={() => openTab?.("providers")}
        >
          <span className="first-run-setup__index">1</span>
          <span>
            <strong>Providers</strong>
            <small>{enabledCount} enabled</small>
          </span>
        </button>

        <button
          type="button"
          className="first-run-setup__action"
          disabled={saving || hasCliDefaults}
          onClick={enableCliDefaults}
        >
          <span className="first-run-setup__index">2</span>
          <span>
            <strong>CLI defaults</strong>
            <small>{hasCliDefaults ? "Enabled" : "Codex, Claude, Gemini"}</small>
          </span>
        </button>

        <button
          type="button"
          className="first-run-setup__action"
          onClick={() => openTab?.("advanced")}
        >
          <span className="first-run-setup__index">3</span>
          <span>
            <strong>Diagnostics</strong>
            <small>Safe support snapshot</small>
          </span>
        </button>

        <button
          type="button"
          className="first-run-setup__action"
          onClick={() => openTab?.("about")}
        >
          <span className="first-run-setup__index">4</span>
          <span>
            <strong>Updates</strong>
            <small>{settings.updateChannel}</small>
          </span>
        </button>
      </div>
    </section>
  );
}

export default function GeneralTab(props: TabProps) {
  const { settings, set, saving } = props;
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
      <FirstRunSetup {...props} />

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
            <Select
              value={String(settings.refreshIntervalSecs)}
              disabled={saving}
              options={REFRESH_CADENCE_OPTIONS}
              onChange={(v) => set({ refreshIntervalSecs: Number(v) })}
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
