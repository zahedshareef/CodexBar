// Central icon registry (Phase 12).
//
// Re-exports the provider icon registry from
// `components/providers/providerIcons.ts` and adds a theme-aware set of
// status / UI icons. Status icons use `currentColor` so callers can tint
// them with `color:` / CSS variables (e.g. `var(--provider-status-ok)`).
//
// Keep Rust-side registries authoritative — when new providers are added in
// `rust/src/native_ui/provider_icons.rs`, mirror them into
// `components/providers/providerIcons.ts`. This module should not duplicate
// that list.

import {
  getProviderIcon,
  PROVIDER_ICON_REGISTRY,
  type ProviderIcon,
} from "../components/providers/providerIcons";

export { getProviderIcon, PROVIDER_ICON_REGISTRY };
export type { ProviderIcon };

/** Identifier for a non-provider status / UI glyph. */
export type StatusIconId =
  | "ok"
  | "stale"
  | "error"
  | "loading"
  | "disabled"
  | "warning"
  | "info";

export interface StatusIcon {
  id: StatusIconId;
  /** Raw SVG markup. Fill colors reference `currentColor` so the SVG picks
   *  up the theme token applied by the surrounding CSS. */
  svg: string;
  /**
   * Default CSS custom property that callers are expected to apply via
   * `color: var(…)` when they want theme-aware tinting. The registry itself
   * does not set colors — it only hands the caller a hint.
   */
  colorVar: string;
}

const BASE_VIEWBOX = 'viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg"';

const STATUS_ICONS: Record<StatusIconId, StatusIcon> = {
  ok: {
    id: "ok",
    svg: `<svg ${BASE_VIEWBOX} fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3,8.5 6.5,12 13,4.5"/></svg>`,
    colorVar: "--provider-status-ok",
  },
  stale: {
    id: "stale",
    svg: `<svg ${BASE_VIEWBOX} fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="8" cy="8" r="6"/><polyline points="8,4.5 8,8 10.5,9.5"/></svg>`,
    colorVar: "--provider-status-stale",
  },
  error: {
    id: "error",
    svg: `<svg ${BASE_VIEWBOX} fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="8" cy="8" r="6"/><line x1="5.5" y1="5.5" x2="10.5" y2="10.5"/><line x1="10.5" y1="5.5" x2="5.5" y2="10.5"/></svg>`,
    colorVar: "--provider-status-error",
  },
  loading: {
    id: "loading",
    svg: `<svg ${BASE_VIEWBOX} fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M8 2a6 6 0 1 1-5.196 3" opacity="0.85"/></svg>`,
    colorVar: "--provider-status-loading",
  },
  disabled: {
    id: "disabled",
    svg: `<svg ${BASE_VIEWBOX} fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="8" cy="8" r="6"/><line x1="4" y1="4" x2="12" y2="12"/></svg>`,
    colorVar: "--provider-status-disabled",
  },
  warning: {
    id: "warning",
    svg: `<svg ${BASE_VIEWBOX} fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 2.5 L14 13 H2 Z"/><line x1="8" y1="6.5" x2="8" y2="9.5"/><circle cx="8" cy="11.5" r="0.75" fill="currentColor" stroke="none"/></svg>`,
    colorVar: "--provider-status-stale",
  },
  info: {
    id: "info",
    svg: `<svg ${BASE_VIEWBOX} fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="8" cy="8" r="6"/><line x1="8" y1="7" x2="8" y2="11.5"/><circle cx="8" cy="4.75" r="0.75" fill="currentColor" stroke="none"/></svg>`,
    colorVar: "--accent",
  },
};

export function getStatusIcon(id: StatusIconId): StatusIcon {
  return STATUS_ICONS[id];
}

export const STATUS_ICON_REGISTRY = STATUS_ICONS;
