import { useCallback, useEffect, useState } from "react";
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

const TAB_META: { id: SettingsTab; labelKey: LocaleKey; icon: string }[] = [
  { id: "general", labelKey: "TabGeneral", icon: "⚙" },
  { id: "providers", labelKey: "TabProviders", icon: "◉" },
  { id: "display", labelKey: "TabDisplay", icon: "◧" },
  { id: "apiKeys", labelKey: "TabApiKeys", icon: "🔑" },
  { id: "cookies", labelKey: "TabCookies", icon: "🍪" },
  { id: "tokenAccounts", labelKey: "TabTokenAccounts", icon: "🪙" },
  { id: "advanced", labelKey: "TabAdvanced", icon: "⌘" },
  { id: "about", labelKey: "TabAbout", icon: "ℹ" },
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
            <span className="settings-tab__icon">{tab.icon}</span>
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
