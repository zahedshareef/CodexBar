import { useCallback, useEffect, useState, type ReactElement, type ReactNode } from "react";
import type {
  BootstrapState,
  SettingsTabId,
  SettingsUpdate,
} from "../types/bridge";
import { useSettings } from "../hooks/useSettings";
import { useSurfaceTarget } from "../hooks/useSurfaceMode";
import { useLocale } from "../hooks/useLocale";
import type { LocaleKey } from "../i18n/keys";
import { setSurfaceMode } from "../lib/tauri";
import GeneralTab from "./settings/tabs/GeneralTab";
import DisplayTab from "./settings/tabs/DisplayTab";
import AdvancedTab from "./settings/tabs/AdvancedTab";
import ApiKeysTab from "./settings/tabs/ApiKeysTab";
import CookiesTab from "./settings/tabs/CookiesTab";
import TokenAccountsTab from "./settings/tabs/TokenAccountsTab";
import AboutTab from "./settings/tabs/AboutTab";
import ProvidersTab from "./settings/tabs/ProvidersTab";

// ── tab types ────────────────────────────────────────────────────────

type SettingsTab = SettingsTabId;

// Inline monochrome SVG icons stand in for the upstream macOS SF Symbols
// (gearshape / square.grid.2x2 / eye / key / book.closed / circle.hexagongrid /
//  slider.horizontal.3 / info.circle). They render in `currentColor` so they
//  pick up the same secondary/accent text color as the tab label.
const ICON_SIZE = 16;

function Svg({ children }: { children: ReactNode }) {
  return (
    <svg
      width={ICON_SIZE}
      height={ICON_SIZE}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.4}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      {children}
    </svg>
  );
}

const TabIcons: Record<SettingsTab, ReactElement> = {
  general: (
    <Svg>
      <circle cx="8" cy="8" r="2" />
      <path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4" />
    </Svg>
  ),
  providers: (
    <Svg>
      <rect x="2" y="2" width="5" height="5" rx="1" />
      <rect x="9" y="2" width="5" height="5" rx="1" />
      <rect x="2" y="9" width="5" height="5" rx="1" />
      <rect x="9" y="9" width="5" height="5" rx="1" />
    </Svg>
  ),
  display: (
    <Svg>
      <circle cx="8" cy="8" r="3.25" />
      <path d="M8 1.75v1.5M8 12.75v1.5M1.75 8h1.5M12.75 8h1.5M3.5 3.5l1.05 1.05M11.45 11.45l1.05 1.05M3.5 12.5l1.05-1.05M11.45 4.55l1.05-1.05" />
    </Svg>
  ),
  apiKeys: (
    <Svg>
      <circle cx="5.5" cy="8" r="2.5" />
      <path d="M8 8h6M11.5 8v2M14 8v1.5" />
    </Svg>
  ),
  cookies: (
    <Svg>
      <path d="M14.25 8.5a6.25 6.25 0 1 1-6.75-6.74 2.4 2.4 0 0 0 2.5 2.5 2.4 2.4 0 0 0 2.5 2.5 2.4 2.4 0 0 0 1.75 1.74Z" />
      <circle cx="6" cy="8" r="0.6" fill="currentColor" stroke="none" />
      <circle cx="9.5" cy="10.5" r="0.6" fill="currentColor" stroke="none" />
      <circle cx="5.5" cy="11" r="0.6" fill="currentColor" stroke="none" />
    </Svg>
  ),
  tokenAccounts: (
    <Svg>
      <path d="M8 1.5l5.5 3v7L8 14.5 2.5 11.5v-7Z" />
      <path d="M2.5 4.5L8 7.5l5.5-3M8 7.5v7" />
    </Svg>
  ),
  advanced: (
    <Svg>
      <path d="M2 4h8M2 8h5M2 12h10" />
      <circle cx="11.5" cy="4" r="1.4" />
      <circle cx="8.5" cy="8" r="1.4" />
      <circle cx="13" cy="12" r="1.4" />
    </Svg>
  ),
  about: (
    <Svg>
      <circle cx="8" cy="8" r="6.25" />
      <path d="M8 7v4" />
      <circle cx="8" cy="5" r="0.6" fill="currentColor" stroke="none" />
    </Svg>
  ),
};

// Tab order mirrors upstream PreferencesView (General, Providers, Display,
// Advanced, About) with the Win-port-specific credential tabs grouped after
// Providers so they remain reachable but don't push parity tabs out of view.
const TAB_META: { id: SettingsTab; labelKey: LocaleKey }[] = [
  { id: "general", labelKey: "TabGeneral" },
  { id: "providers", labelKey: "TabProviders" },
  { id: "display", labelKey: "TabDisplay" },
  { id: "apiKeys", labelKey: "TabApiKeys" },
  { id: "cookies", labelKey: "TabCookies" },
  { id: "tokenAccounts", labelKey: "TabTokenAccounts" },
  { id: "advanced", labelKey: "TabAdvanced" },
  { id: "about", labelKey: "TabAbout" },
];

function isSettingsTab(value: string): value is SettingsTab {
  return TAB_META.some((t) => t.id === value);
}

export default function Settings({ state }: { state: BootstrapState }) {
  const { settings, saving, error, update } = useSettings(state.settings);
  const { t } = useLocale();
  const shellTarget = useSurfaceTarget("settings");
  const initialTab: SettingsTab =
    shellTarget?.kind === "settings" && isSettingsTab(shellTarget.tab)
      ? shellTarget.tab
      : "general";
  const [activeTab, setActiveTab] = useState<SettingsTab>(initialTab);

  useEffect(() => {
    if (shellTarget?.kind !== "settings" || !isSettingsTab(shellTarget.tab)) {
      return;
    }

    const nextTab: SettingsTab = shellTarget.tab;
    setActiveTab((current) => (current === nextTab ? current : nextTab));
  }, [shellTarget]);

  const set = (patch: SettingsUpdate) => void update(patch);
  const handleTabClick = useCallback((tab: SettingsTab) => {
    setActiveTab(tab);
    void setSurfaceMode("settings", { kind: "settings", tab });
  }, []);

  return (
    <div className="settings">
      {/* tab bar */}
      <nav className="settings-tabs" role="tablist">
        {TAB_META.map((tab) => (
          <button
            key={tab.id}
            role="tab"
            aria-selected={activeTab === tab.id}
            className={`settings-tab ${activeTab === tab.id ? "settings-tab--active" : ""}`}
            onClick={() => handleTabClick(tab.id)}
          >
            <span className="settings-tab__icon">{TabIcons[tab.id]}</span>
            {t(tab.labelKey)}
          </button>
        ))}
      </nav>

      {/* status bar */}
      {(saving || error) && (
        <div
          className={`settings-status ${error ? "settings-status--error" : ""}`}
        >
          {saving ? t("SettingsStatusSaving") : error}
        </div>
      )}

      {/* tab panels */}
      <div className="settings-body">
        {activeTab === "general" && (
          <GeneralTab settings={settings} set={set} saving={saving} />
        )}
        {activeTab === "providers" && (
          <ProvidersTab
            settings={settings}
            providers={state.providers}
            set={set}
            saving={saving}
          />
        )}
        {activeTab === "display" && (
          <DisplayTab settings={settings} set={set} saving={saving} />
        )}
        {activeTab === "advanced" && (
          <AdvancedTab settings={settings} set={set} saving={saving} />
        )}
        {activeTab === "apiKeys" && <ApiKeysTab providers={state.providers} />}
        {activeTab === "cookies" && <CookiesTab providers={state.providers} />}
        {activeTab === "tokenAccounts" && <TokenAccountsTab />}
        {activeTab === "about" && <AboutTab />}
      </div>
    </div>
  );
}

// ── Tab props shared with extracted tab components ──────────────────

export interface TabProps {
  settings: BootstrapState["settings"];
  set: (p: SettingsUpdate) => void;
  saving: boolean;
}
