import { useLocale } from "../../hooks/useLocale";

interface SurfaceSummaryProps {
  total: number;
  errorCount: number;
  isRefreshing: boolean;
  lastRefresh: { providerCount: number; errorCount: number } | null;
}

export default function SurfaceSummary({
  total,
  errorCount,
  isRefreshing,
  lastRefresh,
}: SurfaceSummaryProps) {
  const { t } = useLocale();
  const parts: string[] = [];
  parts.push(`${total} ${t("SummaryProvidersLabel")}`);
  if (isRefreshing) {
    parts.push(t("SummaryRefreshing"));
  } else if (lastRefresh && lastRefresh.errorCount > 0) {
    parts.push(`${lastRefresh.errorCount} ${t("SummaryFailed")}`);
  }
  if (!isRefreshing && errorCount > 0) {
    parts.push(`${errorCount} ${t("SummaryWithErrors")}`);
  }

  return <div className="surface-summary">{parts.join(" · ")}</div>;
}
