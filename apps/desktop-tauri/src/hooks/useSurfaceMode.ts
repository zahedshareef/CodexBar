import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { SurfaceMode, SurfaceTarget } from "../types/bridge";
import { getCurrentSurfaceState } from "../lib/tauri";

interface SurfaceModePayload {
  mode: SurfaceMode;
  previous: SurfaceMode;
  target: SurfaceTarget;
}

export interface SurfaceState {
  mode: SurfaceMode;
  target: SurfaceTarget;
}

/**
 * Subscribe to the current surface mode.
 *
 * Reads the initial mode from the Rust backend, then keeps in sync via
 * the `surface-mode-changed` Tauri event.
 */
export function useSurfaceMode(): SurfaceState {
  const [surface, setSurface] = useState<SurfaceState>({
    mode: "hidden",
    target: { kind: "summary" },
  });

  useEffect(() => {
    let cancelled = false;

    getCurrentSurfaceState().then((current) => {
      if (!cancelled) {
        setSurface(current);
      }
    });

    const unlisten = listen<SurfaceModePayload>(
      "surface-mode-changed",
      (event) => {
        setSurface({
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

  return surface;
}
