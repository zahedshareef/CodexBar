import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type {
  CurrentSurfaceState,
  SurfaceMode,
  SurfaceTarget,
} from "../types/bridge";
import { getCurrentSurfaceMode, getCurrentSurfaceState } from "../lib/tauri";

interface SurfaceModePayload {
  mode: SurfaceMode;
  previous: SurfaceMode;
  target: SurfaceTarget;
}

/**
 * Subscribe to the current surface mode.
 *
 * Reads the initial mode from the Rust backend, then keeps in sync via
 * the `surface-mode-changed` Tauri event.
 */
export function useSurfaceMode(): SurfaceMode {
  const [mode, setMode] = useState<SurfaceMode>("hidden");

  useEffect(() => {
    let cancelled = false;

    getCurrentSurfaceMode().then((current) => {
      if (!cancelled) {
        setMode(current);
      }
    });

    const unlisten = listen<SurfaceModePayload>(
      "surface-mode-changed",
      (event) => {
        setMode(event.payload.mode);
      },
    );

    return () => {
      cancelled = true;
      unlisten.then((fn) => fn());
    };
  }, []);

  return mode;
}

/**
 * Subscribe to the current surface target for a given coarse mode.
 *
 * This keeps same-mode retargets visible inside already-mounted surfaces without
 * promoting root routing to full mode+target snapshots.
 */
export function useSurfaceTarget(mode?: SurfaceMode): SurfaceTarget | null {
  const [target, setTarget] = useState<SurfaceTarget | null>(null);

  useEffect(() => {
    let cancelled = false;

    getCurrentSurfaceState().then((current: CurrentSurfaceState) => {
      if (cancelled) {
        return;
      }

      setTarget(mode === undefined || current.mode === mode ? current.target : null);
    });

    const unlisten = listen<SurfaceModePayload>(
      "surface-mode-changed",
      (event) => {
        setTarget(
          mode === undefined || event.payload.mode === mode
            ? event.payload.target
            : null,
        );
      },
    );

    return () => {
      cancelled = true;
      unlisten.then((fn) => fn());
    };
  }, [mode]);

  return target;
}
