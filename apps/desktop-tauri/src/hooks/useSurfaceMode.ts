import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type {
  CurrentSurfaceState,
  SurfaceMode,
  SurfaceTarget,
} from "../types/bridge";
import { getCurrentSurfaceState } from "../lib/tauri";

interface SurfaceModePayload {
  mode: SurfaceMode;
  previous: SurfaceMode;
  target: SurfaceTarget;
}

interface SurfaceSnapshot {
  mode: SurfaceMode;
  target: SurfaceTarget | null;
}

export function useSurfaceSnapshot(): SurfaceSnapshot {
  const [snapshot, setSnapshot] = useState<SurfaceSnapshot>({
    mode: "hidden",
    target: null,
  });

  useEffect(() => {
    let cancelled = false;

    getCurrentSurfaceState().then((current: CurrentSurfaceState) => {
      if (!cancelled) {
        setSnapshot({
          mode: current.mode,
          target: current.target,
        });
      }
    });

    const unlisten = listen<SurfaceModePayload>(
      "surface-mode-changed",
      (event) => {
        setSnapshot({
          mode: event.payload.mode,
          target: event.payload.target,
        });
      },
    );

    return () => {
      cancelled = true;
      unlisten.then((fn) => fn());
    };
  }, []);

  return snapshot;
}

/**
 * Subscribe to the current surface mode.
 *
 * Reads the initial mode from the Rust backend, then keeps in sync via
 * the `surface-mode-changed` Tauri event.
 */
export function useSurfaceMode(): SurfaceMode {
  return useSurfaceSnapshot().mode;
}

/**
 * Subscribe to the current surface target for a given coarse mode.
 *
 * This keeps same-mode retargets visible inside already-mounted surfaces without
 * promoting root routing to full mode+target snapshots.
 */
export function useSurfaceTarget(mode?: SurfaceMode): SurfaceTarget | null {
  const snapshot = useSurfaceSnapshot();

  if (mode !== undefined && snapshot.mode !== mode) {
    return null;
  }

  return snapshot.target;
}
