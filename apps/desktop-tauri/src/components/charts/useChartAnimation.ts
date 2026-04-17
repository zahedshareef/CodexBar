import { useEffect, useRef, useState } from "react";

/**
 * Drives a 0→1 chart entrance animation using requestAnimationFrame.
 *
 * Mirrors the egui timing in `rust/src/native_ui/charts.rs`:
 *   - TOTAL_ANIMATION_MS = 600
 *   - STAGGER_PER_BAR_MS = 20
 *   - Ease-out curve: 1 - (1 - t)^3
 *
 * The returned `barProgress(i)` helper returns the per-bar eased
 * progress; callers multiply it against the natural bar height / point
 * Y to produce the entrance effect. When `enabled` is false the hook
 * short-circuits to 1.0 so there is no animation (respects the
 * `enableAnimations` setting and `prefers-reduced-motion`).
 *
 * `deps` resets the animation whenever the dataset identity changes
 * (e.g. switching providers or tabs).
 */
export const TOTAL_ANIMATION_MS = 600;
export const STAGGER_PER_BAR_MS = 20;

export interface ChartAnimation {
  /** Global 0..1 eased progress (stagger-agnostic). */
  progress: number;
  /** Per-item 0..1 eased progress with stagger. */
  barProgress: (index: number) => number;
  /** True until every staggered bar has reached 1.0. */
  running: boolean;
}

export function useChartAnimation(
  count: number,
  enabled: boolean,
  deps: ReadonlyArray<unknown> = [],
): ChartAnimation {
  const [elapsed, setElapsed] = useState(0);
  const startRef = useRef<number | null>(null);
  const rafRef = useRef<number | null>(null);
  const prefersReduced = usePrefersReducedMotion();
  const skip = !enabled || prefersReduced || count === 0;

  useEffect(() => {
    if (skip) {
      setElapsed(Number.POSITIVE_INFINITY);
      return;
    }

    startRef.current = null;
    setElapsed(0);

    const totalMs = TOTAL_ANIMATION_MS + count * STAGGER_PER_BAR_MS;

    const tick = (now: number) => {
      if (startRef.current == null) startRef.current = now;
      const ms = now - startRef.current;
      setElapsed(ms);
      if (ms < totalMs) {
        rafRef.current = requestAnimationFrame(tick);
      }
    };

    rafRef.current = requestAnimationFrame(tick);
    return () => {
      if (rafRef.current != null) cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [skip, count, ...deps]);

  const easeOut = (t: number) => 1 - Math.pow(1 - t, 3);
  const clamp01 = (v: number) => (v < 0 ? 0 : v > 1 ? 1 : v);

  const progress = skip ? 1 : clamp01(elapsed / TOTAL_ANIMATION_MS);
  const barProgress = (i: number) => {
    if (skip) return 1;
    const barElapsed = Math.max(0, elapsed - i * STAGGER_PER_BAR_MS);
    return easeOut(clamp01(barElapsed / TOTAL_ANIMATION_MS));
  };
  const totalMs = TOTAL_ANIMATION_MS + count * STAGGER_PER_BAR_MS;
  const running = !skip && elapsed < totalMs;

  return { progress: easeOut(progress), barProgress, running };
}

function usePrefersReducedMotion(): boolean {
  const [reduced, setReduced] = useState(false);

  useEffect(() => {
    if (typeof window === "undefined" || !window.matchMedia) return;
    const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
    const update = () => setReduced(mq.matches);
    update();
    mq.addEventListener?.("change", update);
    return () => {
      mq.removeEventListener?.("change", update);
    };
  }, []);

  return reduced;
}
