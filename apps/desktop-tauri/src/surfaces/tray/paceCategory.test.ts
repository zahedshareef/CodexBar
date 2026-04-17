import { describe, it, expect } from "vitest";
import { paceCategory, paceCategoryKey } from "./paceCategory";
import type { PaceSnapshot } from "../../types/bridge";

type Stage = PaceSnapshot["stage"];

describe("paceCategory", () => {
  it("collapses the 7-stage pace enum into 4 buckets", () => {
    const map: Record<Stage, ReturnType<typeof paceCategory>> = {
      far_behind: "slow",
      behind: "slow",
      slightly_behind: "slow",
      on_track: "steady",
      slightly_ahead: "racing",
      ahead: "racing",
      far_ahead: "burning",
    };
    for (const [stage, expected] of Object.entries(map) as [Stage, string][]) {
      expect(paceCategory(stage)).toBe(expected);
    }
  });

  it("falls back to 'steady' for unexpected stage values", () => {
    // @ts-expect-error — deliberately exercising the default arm.
    expect(paceCategory("wat")).toBe("steady");
  });

  it("maps each category to a distinct locale key", () => {
    const keys = (["slow", "steady", "racing", "burning"] as const).map(
      paceCategoryKey,
    );
    expect(new Set(keys).size).toBe(4);
    expect(keys).toEqual([
      "TrayPaceBadgeSlow",
      "TrayPaceBadgeSteady",
      "TrayPaceBadgeRacing",
      "TrayPaceBadgeBurning",
    ]);
  });
});
