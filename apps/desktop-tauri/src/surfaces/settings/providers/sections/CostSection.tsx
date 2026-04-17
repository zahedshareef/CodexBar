import type { CostSnapshotBridge } from "../../../../types/bridge";
import type { LocaleKey } from "../../../../i18n/keys";

interface Props {
  cost: CostSnapshotBridge | null;
  t: (key: LocaleKey) => string;
}

/**
 * Today / 30-day cost + reset hint. Port of the cost block in
 * `rust/src/native_ui/preferences.rs::render_provider_detail_panel`.
 */
export function CostSection({ cost, t }: Props) {
  if (!cost) return null;

  const rows: { label: string; value: string | null }[] = [
    { label: t("DetailCostUsed"), value: cost.formattedUsed },
    { label: t("DetailCostLimit"), value: cost.formattedLimit },
    {
      label: t("DetailCostResets"),
      value: cost.resetsAt ?? null,
    },
  ];
  const visible = rows.filter(
    (r): r is { label: string; value: string } =>
      !!r.value && r.value.length > 0,
  );

  return (
    <section className="provider-detail-section">
      <h4>{t("DetailCostTitle")}</h4>
      <dl className="provider-detail-grid">
        {visible.map((r) => (
          <div key={r.label} style={{ display: "contents" }}>
            <dt>{r.label}</dt>
            <dd>{r.value}</dd>
          </div>
        ))}
      </dl>
    </section>
  );
}
