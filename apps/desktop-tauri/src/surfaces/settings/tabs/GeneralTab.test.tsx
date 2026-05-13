import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { SettingsSnapshot } from "../../../types/bridge";

vi.mock("../../../hooks/useLocale", () => ({
  useLocale: () => ({ t: (key: string) => key }),
}));

vi.mock("../../../lib/tauri", () => ({
  playNotificationSound: vi.fn(),
  quitApp: vi.fn(),
}));

import GeneralTab from "./GeneralTab";

function settings(overrides: Partial<SettingsSnapshot> = {}): SettingsSnapshot {
  return {
    enabledProviders: [],
    refreshIntervalSecs: 300,
    startAtLogin: false,
    startMinimized: true,
    showNotifications: true,
    soundEnabled: false,
    soundVolume: 80,
    highUsageThreshold: 75,
    criticalUsageThreshold: 90,
    trayIconMode: "single",
    switcherShowsIcons: true,
    menuBarShowsHighestUsage: true,
    menuBarShowsPercent: true,
    showAsUsed: true,
    showCreditsExtraUsage: true,
    showAllTokenAccountsInMenu: false,
    providerChangelogLinksEnabled: true,
    surpriseAnimations: false,
    enableAnimations: true,
    resetTimeRelative: true,
    menuBarDisplayMode: "detailed",
    hidePersonalInfo: false,
    updateChannel: "stable",
    autoDownloadUpdates: false,
    installUpdatesOnQuit: false,
    globalShortcut: "",
    uiLanguage: "english",
    theme: "auto",
    claudeAvoidKeychainPrompts: false,
    disableKeychainAccess: false,
    showDebugSettings: false,
    providerMetrics: {},
    ...overrides,
  };
}

describe("GeneralTab first-run setup", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it("enables the common CLI providers from the setup card", () => {
    const set = vi.fn();
    render(
      <GeneralTab
        settings={settings({ enabledProviders: ["cursor"] })}
        set={set}
        saving={false}
      />,
    );

    fireEvent.click(screen.getByText("CLI defaults"));

    expect(set).toHaveBeenCalledWith({
      enabledProviders: ["claude", "codex", "cursor", "gemini"],
    });
  });

  it("opens provider diagnostics from the setup card", () => {
    const openTab = vi.fn();
    render(
      <GeneralTab
        settings={settings()}
        set={vi.fn()}
        saving={false}
        openTab={openTab}
      />,
    );

    fireEvent.click(screen.getByText("Diagnostics"));

    expect(openTab).toHaveBeenCalledWith("advanced");
  });

  it("dismisses the setup card", () => {
    render(
      <GeneralTab settings={settings()} set={vi.fn()} saving={false} />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "Dismiss first-run setup" }),
    );

    expect(screen.queryByText("First-run setup")).not.toBeInTheDocument();
    expect(
      window.localStorage.getItem("codexbar:first-run-setup-dismissed"),
    ).toBe("1");
  });
});
