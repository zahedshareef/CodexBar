import { describe, expect, it } from "vitest";
import type { ProviderDetail } from "../../../../types/bridge";
import { deriveProviderReadiness } from "./ProviderReadinessSection";

function provider(overrides: Partial<ProviderDetail> = {}): ProviderDetail {
  return {
    id: "codex",
    displayName: "Codex",
    enabled: true,
    email: "user@example.test",
    plan: "Pro",
    authType: "CLI",
    sourceLabel: "Codex CLI",
    organization: null,
    lastUpdated: new Date().toISOString(),
    session: null,
    weekly: null,
    modelSpecific: null,
    tertiary: null,
    extraRateWindows: [],
    cost: null,
    pace: null,
    lastError: null,
    dashboardUrl: "https://chatgpt.com",
    statusPageUrl: null,
    changelogUrl: null,
    buyCreditsUrl: null,
    hasSnapshot: true,
    cookieSource: null,
    region: null,
    ...overrides,
  };
}

describe("deriveProviderReadiness", () => {
  it("flags disabled providers before other setup states", () => {
    expect(
      deriveProviderReadiness(
        provider({ enabled: false, lastError: "ignored while disabled" }),
      ),
    ).toMatchObject({ state: "disabled", title: "Provider disabled" });
  });

  it("surfaces refresh errors", () => {
    expect(
      deriveProviderReadiness(provider({ lastError: "not signed in" })),
    ).toMatchObject({ state: "error", detail: "not signed in" });
  });

  it("prompts for refresh when no snapshot exists", () => {
    expect(
      deriveProviderReadiness(provider({ hasSnapshot: false })),
    ).toMatchObject({ state: "waiting", title: "No usage snapshot yet" });
  });

  it("reports ready when the provider has a clean snapshot", () => {
    expect(deriveProviderReadiness(provider())).toMatchObject({
      state: "ready",
      title: "Ready",
    });
  });
});
