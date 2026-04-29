import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type {
  ProviderUsageSnapshot,
  RefreshCompletePayload,
} from "../types/bridge";
import {
  getCachedProviders,
  refreshProviders,
  refreshProvidersIfStale,
} from "../lib/tauri";

export interface UseProvidersResult {
  /** Current provider snapshots (updated live as each provider completes). */
  providers: ProviderUsageSnapshot[];
  /** True while a refresh cycle is in progress. */
  isRefreshing: boolean;
  /** Trigger a manual refresh. No-op if already refreshing. */
  refresh: () => void;
  /** Summary from the last completed refresh cycle, if any. */
  lastRefresh: RefreshCompletePayload | null;
  /** True when the hook has provider data that can stay visible during refresh. */
  hasCachedData: boolean;
}

/**
 * Subscribe to live provider usage data.
 *
 * On mount the hook:
 *  1. Loads any cached providers already in AppState.
 *  2. Fires `refresh_providers` to kick off a fresh fetch cycle.
 *  3. Listens for `provider-updated` events and merges each snapshot
 *     into the local array (upsert by providerId).
 *  4. Listens for `refresh-started` / `refresh-complete` to track loading.
 */
export function useProviders(): UseProvidersResult {
  const [providers, setProviders] = useState<ProviderUsageSnapshot[]>([]);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [lastRefresh, setLastRefresh] = useState<RefreshCompletePayload | null>(
    null,
  );
  const refreshingRef = useRef(false);

  // Upsert a single snapshot into the providers array.
  const upsert = useCallback((snapshot: ProviderUsageSnapshot) => {
    setProviders((prev) => {
      const idx = prev.findIndex(
        (p) => p.providerId === snapshot.providerId,
      );
      if (idx >= 0) {
        const next = [...prev];
        next[idx] = snapshot;
        return next;
      }
      return [...prev, snapshot];
    });
  }, []);

  const refresh = useCallback(() => {
    if (refreshingRef.current) return;
    refreshingRef.current = true;
    setIsRefreshing(true);
    refreshProviders().catch(() => {
      refreshingRef.current = false;
      setIsRefreshing(false);
    });
  }, []);

  useEffect(() => {
    let cancelled = false;

    // Load existing cache first.
    getCachedProviders().then((cached) => {
      if (!cancelled && cached.length > 0) {
        setProviders(cached);
      }
    });

    // Event listeners.
    const unlistenUpdated = listen<ProviderUsageSnapshot>(
      "provider-updated",
      (event) => {
        if (!cancelled) upsert(event.payload);
      },
    );

    const unlistenStarted = listen("refresh-started", () => {
      if (!cancelled) {
        refreshingRef.current = true;
        setIsRefreshing(true);
      }
    });

    const unlistenComplete = listen<RefreshCompletePayload>(
      "refresh-complete",
      (event) => {
        if (!cancelled) {
          refreshingRef.current = false;
          setIsRefreshing(false);
          setLastRefresh(event.payload);
        }
      },
    );

    // Kick off the initial refresh, but let the backend reuse fresh cache.
    refreshProvidersIfStale().catch(() => {
      if (!cancelled) {
        refreshingRef.current = false;
        setIsRefreshing(false);
      }
    });

    return () => {
      cancelled = true;
      unlistenUpdated.then((fn) => fn());
      unlistenStarted.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
    };
  }, [refresh, upsert]);

  return {
    providers,
    isRefreshing,
    refresh,
    lastRefresh,
    hasCachedData: providers.length > 0,
  };
}
