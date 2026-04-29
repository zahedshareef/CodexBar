import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const tauriMocks = vi.hoisted(() => ({
  getCachedProviders: vi.fn(),
  refreshProviders: vi.fn(),
  refreshProvidersIfStale: vi.fn(),
}));

const eventMocks = vi.hoisted(() => ({
  listen: vi.fn(),
}));

vi.mock("../lib/tauri", () => tauriMocks);

vi.mock("@tauri-apps/api/event", () => eventMocks);

import { useProviders } from "./useProviders";

describe("useProviders", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    tauriMocks.getCachedProviders.mockResolvedValue([]);
    tauriMocks.refreshProviders.mockResolvedValue(undefined);
    tauriMocks.refreshProvidersIfStale.mockResolvedValue(undefined);
    eventMocks.listen.mockResolvedValue(() => {});
  });

  it("uses stale-aware refresh on mount", async () => {
    renderHook(() => useProviders());

    await waitFor(() => {
      expect(tauriMocks.refreshProvidersIfStale).toHaveBeenCalledTimes(1);
    });
    expect(tauriMocks.refreshProviders).not.toHaveBeenCalled();
  });

  it("manual refresh uses forced refresh", async () => {
    const { result } = renderHook(() => useProviders());

    await waitFor(() => {
      expect(tauriMocks.refreshProvidersIfStale).toHaveBeenCalledTimes(1);
    });

    act(() => {
      result.current.refresh();
    });

    expect(tauriMocks.refreshProviders).toHaveBeenCalledTimes(1);
  });

  it("reports cached data when cached providers are loaded", async () => {
    tauriMocks.getCachedProviders.mockResolvedValue([
      {
        providerId: "codex",
        displayName: "Codex",
        primary: {
          usedPercent: 25,
          remainingPercent: 75,
          windowMinutes: null,
          resetsAt: null,
          resetDescription: null,
          isExhausted: false,
          reservePercent: null,
          reserveDescription: null,
        },
        primaryLabel: "Session",
        secondary: null,
        secondaryLabel: null,
        modelSpecific: null,
        tertiary: null,
        extraRateWindows: [],
        cost: null,
        planName: null,
        accountEmail: null,
        sourceLabel: "CLI",
        updatedAt: new Date().toISOString(),
        error: null,
        pace: null,
        accountOrganization: null,
        trayStatusLabel: "25%",
        fetchDurationMs: null,
      },
    ]);

    const { result } = renderHook(() => useProviders());

    await waitFor(() => {
      expect(result.current.hasCachedData).toBe(true);
    });
  });
});
