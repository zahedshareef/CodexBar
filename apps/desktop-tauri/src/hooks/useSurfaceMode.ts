import { useSyncExternalStore } from "react";
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

let currentSnapshot: SurfaceSnapshot = {
  mode: "hidden",
  target: null,
};

const subscribers = new Set<() => void>();
let storeStarted = false;

function sameTarget(
  left: SurfaceTarget | null,
  right: SurfaceTarget | null,
): boolean {
  if (left === right) return true;
  if (left === null || right === null) return false;
  if (left.kind !== right.kind) return false;

  switch (left.kind) {
    case "provider":
      return right.kind === "provider" && left.providerId === right.providerId;
    case "settings":
      return right.kind === "settings" && left.tab === right.tab;
    default:
      return true;
  }
}

function publishSnapshot(next: SurfaceSnapshot): void {
  if (
    currentSnapshot.mode === next.mode &&
    sameTarget(currentSnapshot.target, next.target)
  ) {
    return;
  }

  currentSnapshot = next;
  subscribers.forEach((notify) => notify());
}

function startSurfaceStore(): void {
  if (storeStarted) {
    return;
  }

  storeStarted = true;

  void getCurrentSurfaceState()
    .then((current: CurrentSurfaceState) => {
      publishSnapshot({
        mode: current.mode,
        target: current.target,
      });
    })
    .catch(() => {});

  void listen<SurfaceModePayload>("surface-mode-changed", (event) => {
    publishSnapshot({
      mode: event.payload.mode,
      target: event.payload.target,
    });
  }).catch(() => {});
}

function subscribe(notify: () => void): () => void {
  startSurfaceStore();
  subscribers.add(notify);

  return () => {
    subscribers.delete(notify);
  };
}

function getSnapshot(): SurfaceSnapshot {
  return currentSnapshot;
}

export function useSurfaceSnapshot(): SurfaceSnapshot {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
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
