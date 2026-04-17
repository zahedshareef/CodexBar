import { useLocale } from "../../hooks/useLocale";

interface SurfaceEmptyProps {
  isLoading: boolean;
  onSettings: () => void;
}

export default function SurfaceEmpty({
  isLoading,
  onSettings,
}: SurfaceEmptyProps) {
  const { t } = useLocale();

  if (isLoading) {
    return (
      <div className="surface-empty">
        <div className="surface-empty__spinner" />
        <p>{t("FetchingProviderData")}</p>
      </div>
    );
  }

  return (
    <div className="surface-empty">
      <p>{t("NoProvidersConfigured")}</p>
      <p className="surface-empty__hint">{t("EnableProvidersHint")}</p>
      <button
        className="tray-btn tray-btn--primary"
        onClick={onSettings}
        type="button"
      >
        {t("OpenSettingsButton")}
      </button>
    </div>
  );
}
