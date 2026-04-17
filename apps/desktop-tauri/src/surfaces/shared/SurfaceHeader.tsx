import { useLocale } from "../../hooks/useLocale";

export interface SurfaceHeaderAction {
  icon: string;
  title: string;
  onClick: () => void;
}

interface SurfaceHeaderProps {
  onRefresh: () => void;
  isRefreshing: boolean;
  actions: SurfaceHeaderAction[];
}

export default function SurfaceHeader({
  onRefresh,
  isRefreshing,
  actions,
}: SurfaceHeaderProps) {
  const { t } = useLocale();
  return (
    <header className="surface-header">
      <h1 className="surface-header__title">CodexBar</h1>
      <div className="surface-header__actions">
        <button
          className="surface-action-btn"
          onClick={onRefresh}
          disabled={isRefreshing}
          title={t("TooltipRefresh")}
          type="button"
        >
          <span className={isRefreshing ? "spin" : ""}>↻</span>
        </button>
        {actions.map((action) => (
          <button
            key={action.title}
            className="surface-action-btn"
            onClick={action.onClick}
            title={action.title}
            type="button"
          >
            {action.icon}
          </button>
        ))}
      </div>
    </header>
  );
}
