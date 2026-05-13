import { useEffect, useState } from "react";
import { useLocale } from "../../../hooks/useLocale";
import { useUpdateState } from "../../../hooks/useUpdateState";
import { getAppInfo } from "../../../lib/tauri";
import { Field, Select, Toggle } from "../../../components/FormControls";
import type { AppInfoBridge, UpdateChannel } from "../../../types/bridge";
import type { TabProps } from "../../Settings";
import codexbarIcon from "../../../assets/codexbar-icon.png";

export default function AboutTab({ settings, set, saving }: TabProps) {
  const { t } = useLocale();
  const [appInfo, setAppInfo] = useState<AppInfoBridge | null>(null);
  const { updateState, checkNow, download, apply, openRelease } =
    useUpdateState();
  const [hasChecked, setHasChecked] = useState(false);

  useEffect(() => {
    void getAppInfo().then(setAppInfo);
  }, []);

  const handleCheck = () => {
    setHasChecked(true);
    checkNow();
  };

  if (!appInfo) {
    return (
      <section className="settings-section">
        <p className="settings-section__hint">Loading…</p>
      </section>
    );
  }

  const isBusy =
    updateState.status === "checking" ||
    updateState.status === "downloading";

  return (
    <section className="settings-section about-section">
      <div className="about-header">
        <img className="about-icon" src={codexbarIcon} alt="CodexBar" />
        <div className="about-title-block">
          <div className="about-title-row">
            <h2 className="about-title">{appInfo.name}</h2>
            <span className="about-pill">Windows</span>
          </div>
          <p className="about-version">
            Version {appInfo.version}
            {appInfo.buildNumber !== "dev" && ` (${appInfo.buildNumber})`}
          </p>
          <p className="about-tagline">{appInfo.tagline}</p>
        </div>
      </div>

      <div className="about-meta-grid">
        <div>
          <span>Version</span>
          <strong>{appInfo.version}</strong>
        </div>
        <div>
          <span>Channel</span>
          <strong>{appInfo.updateChannel}</strong>
        </div>
        <div>
          <span>Build</span>
          <strong>{appInfo.buildNumber}</strong>
        </div>
      </div>

      <div className="about-links">
        <a
          className="about-link"
          href="https://github.com/Finesssee/Win-CodexBar"
          target="_blank"
          rel="noopener noreferrer"
        >
          GitHub
        </a>
        <a
          className="about-link"
          href="https://codexbar.app"
          target="_blank"
          rel="noopener noreferrer"
        >
          Website
        </a>
        <a
          className="about-link"
          href="https://github.com/steipete/CodexBar"
          target="_blank"
          rel="noopener noreferrer"
        >
          Original Project
        </a>
      </div>

      <div className="about-update-panel">
        <div className="about-update-panel__header">
          <div>
            <h3>{t("UpdatesTitle")}</h3>
            <p>{t("UpdateChannelChoiceHelper")}</p>
          </div>
          <button
            className="credential-btn credential-btn--primary"
            disabled={isBusy}
            onClick={handleCheck}
          >
            {updateState.status === "checking"
              ? "Checking..."
              : "Check for updates"}
          </button>
        </div>

        <div className="about-update-controls">
          <Field
            label={t("AutoDownloadUpdates")}
            description={t("AutoDownloadUpdatesHelper")}
            leading
          >
            <Toggle
              checked={settings.autoDownloadUpdates}
              disabled={saving}
              onChange={(v) => set({ autoDownloadUpdates: v })}
            />
          </Field>

          <div className="about-channel-row">
            <Field label={t("UpdateChannelChoice")}>
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
          </div>
        </div>

        <div className="about-actions">
          {updateState.status === "available" && (
            <div className="about-update-row">
              <span className="about-update-msg">
                Update {updateState.version} available
              </span>
              {updateState.canDownload ? (
                <button
                  className="credential-btn credential-btn--primary"
                  onClick={download}
                >
                  Download
                </button>
              ) : (
                <button className="credential-btn" onClick={openRelease}>
                  View Release
                </button>
              )}
            </div>
          )}

          {updateState.status === "downloading" && (
            <span className="about-update-msg">
              Downloading...
              {updateState.progress != null &&
                ` ${Math.round(updateState.progress * 100)}%`}
            </span>
          )}

          {updateState.status === "ready" && (
            <div className="about-update-row">
              <span className="about-update-msg">Update ready to install</span>
              {updateState.canApply ? (
                <button
                  className="credential-btn credential-btn--primary"
                  onClick={apply}
                >
                  Install &amp; restart
                </button>
              ) : (
                <button className="credential-btn" onClick={openRelease}>
                  View Release
                </button>
              )}
            </div>
          )}

          {updateState.status === "error" && (
            <span className="about-update-msg">
              Error: {updateState.error}
            </span>
          )}

          {updateState.status === "idle" && hasChecked && (
            <span className="about-update-msg">You&apos;re up to date.</span>
          )}
        </div>
      </div>

      <p className="about-copyright">
        Windows port by NessZerra. Based on{" "}
        <a
          className="about-link about-link--inline"
          href="https://github.com/steipete/CodexBar"
          target="_blank"
          rel="noopener noreferrer"
        >
          CodexBar
        </a>{" "}
        by steipete and contributors. MIT License.
      </p>
    </section>
  );
}
