import { useEffect, useState } from "react";
import { useUpdateState } from "../../../hooks/useUpdateState";
import { getAppInfo } from "../../../lib/tauri";
import type { AppInfoBridge } from "../../../types/bridge";
import codexbarIcon from "../../../assets/codexbar-icon.png";

export default function AboutTab() {
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
          <h2 className="about-title">{appInfo.name}</h2>
          <p className="about-version">
            Version {appInfo.version}
            {appInfo.buildNumber !== "dev" && ` (${appInfo.buildNumber})`}
          </p>
          <p className="about-tagline">{appInfo.tagline}</p>
        </div>
      </div>

      <div className="about-links">
        <a
          className="about-link"
          href="https://github.com/NessZerra/Win-CodexBar"
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
      </div>

      <div className="about-divider" />

      <div className="about-actions">
        <button
          className="credential-btn credential-btn--primary"
          disabled={isBusy}
          onClick={handleCheck}
        >
          {updateState.status === "checking"
            ? "Checking…"
            : "Check for Updates…"}
        </button>

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
            Downloading…
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
                Install &amp; Restart
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
          <span className="about-update-msg">You&apos;re up to date!</span>
        )}
      </div>

      <p className="about-copyright">
        NessZerra — Windows Version. MIT License.
      </p>
    </section>
  );
}
