import type { UpdateStatePayload } from "../types/bridge";
import { useLocale } from "../hooks/useLocale";

interface UpdateBannerProps {
  updateState: UpdateStatePayload;
  onCheck: () => void;
  onDownload: () => void;
  onApply: () => void;
  onDismiss: () => void;
  onOpenRelease: () => void;
}

export default function UpdateBanner({
  updateState,
  onCheck,
  onDownload,
  onApply,
  onDismiss,
  onOpenRelease,
}: UpdateBannerProps) {
  const { t } = useLocale();
  if (updateState.status === "idle") return null;

  const cls = `update-banner update-banner--${updateState.status}`;

  return (
    <div className={cls}>
      {updateState.status === "checking" && (
        <span>{t("BannerCheckingForUpdates")}</span>
      )}

      {updateState.status === "available" && (
        <>
          <span>
            {t("BannerUpdateAvailablePrefix")} <strong>{updateState.version}</strong>
          </span>
          <div className="update-banner__actions">
            {updateState.canDownload ? (
              <button
                className="update-banner__action"
                onClick={onDownload}
                type="button"
              >
                {t("BannerDownloadButton")}
              </button>
            ) : (
              <button
                className="update-banner__action"
                onClick={onOpenRelease}
                type="button"
              >
                {t("BannerViewRelease")}
              </button>
            )}
            <button
              className="update-banner__action update-banner__action--dismiss"
              onClick={onDismiss}
              type="button"
              title={t("BannerDismiss")}
            >
              ✕
            </button>
          </div>
        </>
      )}

      {updateState.status === "downloading" && (
        <div className="update-banner__downloading">
          <span>
            {t("BannerDownloadingPrefix")}
            {updateState.version && (
              <>
                {" "}
                <strong>{updateState.version}</strong>
              </>
            )}
            {updateState.progress != null &&
              ` — ${Math.round(updateState.progress * 100)}%`}
          </span>
          {updateState.progress != null && (
            <div className="update-banner__progress">
              <div
                className="update-banner__progress-bar"
                style={{
                  width: `${Math.round(updateState.progress * 100)}%`,
                }}
              />
            </div>
          )}
        </div>
      )}

      {updateState.status === "ready" && (
        <>
          <span>
            {t("BannerUpdateAvailablePrefix")}
            {updateState.version && (
              <>
                {" "}
                <strong>{updateState.version}</strong>
              </>
            )}{" "}
            {t("BannerReadyToInstallSuffix")}
          </span>
          <div className="update-banner__actions">
            {updateState.canApply ? (
              <button
                className="update-banner__action"
                onClick={onApply}
                type="button"
              >
                {t("BannerInstallRestart")}
              </button>
            ) : (
              <button
                className="update-banner__action"
                onClick={onOpenRelease}
                type="button"
              >
                {t("BannerViewRelease")}
              </button>
            )}
            <button
              className="update-banner__action update-banner__action--dismiss"
              onClick={onDismiss}
              type="button"
              title={t("BannerDismiss")}
            >
              ✕
            </button>
          </div>
        </>
      )}

      {updateState.status === "error" && (
        <>
          <span>
            {t("BannerUpdateFailedPrefix")}
            {updateState.error && <>: {updateState.error}</>}
          </span>
          <div className="update-banner__actions">
            <button
              className="update-banner__action"
              onClick={onCheck}
              type="button"
            >
              {t("BannerRetry")}
            </button>
            <button
              className="update-banner__action update-banner__action--dismiss"
              onClick={onDismiss}
              type="button"
              title={t("BannerDismiss")}
            >
              ✕
            </button>
          </div>
        </>
      )}
    </div>
  );
}
