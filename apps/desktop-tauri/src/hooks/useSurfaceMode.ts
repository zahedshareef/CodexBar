import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { SurfaceMode } from "../types/bridge";
import { getCurrentSurfaceMode } from "../lib/tauri";

interface SurfaceModePayload {
  mode: SurfaceMode;
  previous: SurfaceMode;
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

    getCurrentSurfaceMode().then((m) => {
      if (!cancelled) setMode(m);
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
