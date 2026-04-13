import type { UpdateStatePayload } from "../types/bridge";

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
  if (updateState.status === "idle") return null;

  const cls = `update-banner update-banner--${updateState.status}`;

  return (
    <div className={cls}>
      {updateState.status === "checking" && (
        <span>Checking for updates…</span>
      )}

      {updateState.status === "available" && (
        <>
          <span>
            Update <strong>{updateState.version}</strong> available
          </span>
          <div className="update-banner__actions">
            {updateState.canDownload ? (
              <button
                className="update-banner__action"
                onClick={onDownload}
                type="button"
              >
                Download
              </button>
            ) : (
              <button
                className="update-banner__action"
                onClick={onOpenRelease}
                type="button"
              >
                View Release
              </button>
            )}
            <button
              className="update-banner__action update-banner__action--dismiss"
              onClick={onDismiss}
              type="button"
            >
              ✕
            </button>
          </div>
        </>
      )}

      {updateState.status === "downloading" && (
        <div className="update-banner__downloading">
          <span>
            Downloading update
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
            Update
            {updateState.version && (
              <>
                {" "}
                <strong>{updateState.version}</strong>
              </>
            )}{" "}
            ready to install
          </span>
          <div className="update-banner__actions">
            {updateState.canApply ? (
              <button
                className="update-banner__action"
                onClick={onApply}
                type="button"
              >
                Install &amp; Restart
              </button>
            ) : (
              <button
                className="update-banner__action"
                onClick={onOpenRelease}
                type="button"
              >
                View Release
              </button>
            )}
            <button
              className="update-banner__action update-banner__action--dismiss"
              onClick={onDismiss}
              type="button"
            >
              ✕
            </button>
          </div>
        </>
      )}

      {updateState.status === "error" && (
        <>
          <span>Update failed{updateState.error && <>: {updateState.error}</>}</span>
          <div className="update-banner__actions">
            <button
              className="update-banner__action"
              onClick={onCheck}
              type="button"
            >
              Retry
            </button>
            <button
              className="update-banner__action update-banner__action--dismiss"
              onClick={onDismiss}
              type="button"
            >
              ✕
            </button>
          </div>
        </>
      )}
    </div>
  );
}
