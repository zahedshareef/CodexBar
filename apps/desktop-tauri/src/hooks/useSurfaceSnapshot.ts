import { useSyncExternalStore } from "react";
import { listen } from "@tauri-apps/api/event";
import type {
  CurrentSurfaceState,
  SurfaceMode,
  SurfaceTarget,
} from "../types/bridge";
import { getCurrentSurfaceState } from "../lib/tauri";

export interface SurfaceSnapshot {
  mode: SurfaceMode;
  target: SurfaceTarget;
}

// Internal store allows null target during bootstrap before the first
// surface state arrives from the Rust backend.
interface InternalSnapshot {
  mode: SurfaceMode;
  target: SurfaceTarget | null;
}

interface SurfaceModePayload {
  mode: SurfaceMode;
  previous: SurfaceMode;
  target: SurfaceTarget;
}

function defaultTarget(mode: SurfaceMode): SurfaceTarget {
  switch (mode) {
    case "trayPanel":
      return { kind: "summary" };
    case "popOut":
      return { kind: "dashboard" };
    case "settings":
      return { kind: "settings", tab: "general" };
    default:
      return { kind: "summary" };
  }
}

let currentSnapshot: InternalSnapshot = {
  mode: "hidden",
  target: null,
};

const subscribers = new Set<() => void>();
let storeStarted = false;
let snapshotVersion = 0;

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

function publishSnapshot(next: InternalSnapshot): void {
  if (
    currentSnapshot.mode === next.mode &&
    sameTarget(currentSnapshot.target, next.target)
  ) {
    return;
  }

  currentSnapshot = next;
  snapshotVersion += 1;
  subscribers.forEach((notify) => notify());
}

function publishBootstrapSnapshot(
  next: InternalSnapshot,
  bootstrapVersion: number,
): void {
  if (snapshotVersion !== bootstrapVersion) {
    return;
  }

  publishSnapshot(next);
}

function loadInitialSnapshot(bootstrapVersion: number): void {
  void getCurrentSurfaceState()
    .then((current: CurrentSurfaceState) => {
      publishBootstrapSnapshot(
        { mode: current.mode, target: current.target },
        bootstrapVersion,
      );
    })
    .catch(() => {});
}

function startSurfaceStore(): void {
  if (storeStarted) {
    return;
  }

  storeStarted = true;

  void listen<SurfaceModePayload>("surface-mode-changed", (event) => {
    publishSnapshot({
      mode: event.payload.mode,
      target: event.payload.target,
    });
  })
    .then(() => {
      loadInitialSnapshot(snapshotVersion);
    })
    .catch(() => {
      loadInitialSnapshot(snapshotVersion);
    });
}

function subscribe(notify: () => void): () => void {
  startSurfaceStore();
  subscribers.add(notify);

  return () => {
    subscribers.delete(notify);
  };
}

// Expose a stable SurfaceSnapshot with a non-null target (falling back to
// a sensible per-mode default while the Rust bridge hasn't responded yet).
let cachedPublic: SurfaceSnapshot = {
  mode: currentSnapshot.mode,
  target: defaultTarget(currentSnapshot.mode),
};

function getSnapshot(): SurfaceSnapshot {
  const { mode, target } = currentSnapshot;
  const resolvedTarget = target ?? defaultTarget(mode);

  if (
    cachedPublic.mode === mode &&
    sameTarget(cachedPublic.target, resolvedTarget)
  ) {
    return cachedPublic;
  }

  cachedPublic = { mode, target: resolvedTarget };
  return cachedPublic;
}

export function useSurfaceSnapshot(): SurfaceSnapshot {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}
