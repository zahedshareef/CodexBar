import { describe, it, expect, vi } from "vitest";
import { formatRelativeUpdated } from "./relativeTime";
import type { LocaleKey } from "../i18n/keys";

// Simple translator stub — echoes the key plus whatever replacement the
// helper substitutes into the "{}" placeholder so we can assert structure.
const t = (key: LocaleKey): string => {
  switch (key) {
    case "NeverUpdated":
      return "Never";
    case "UpdatedJustNow":
      return "just now";
    case "UpdatedMinutesAgo":
      return "{} min ago";
    case "UpdatedHoursAgo":
      return "{} hr ago";
    case "UpdatedDaysAgo":
      return "{} d ago";
    default:
      return key;
  }
};

describe("formatRelativeUpdated", () => {
  const NOW = Date.parse("2024-06-01T12:00:00Z");

  it("returns 'Never' when timestamp is null or undefined", () => {
    expect(formatRelativeUpdated(null, t, NOW)).toBe("Never");
    expect(formatRelativeUpdated(undefined, t, NOW)).toBe("Never");
  });

  it("treats future timestamps as 'just now' (clamps negative diffs)", () => {
    expect(formatRelativeUpdated(NOW + 60_000, t, NOW)).toBe("just now");
  });

  it("uses 'just now' for sub-minute diffs", () => {
    expect(formatRelativeUpdated(NOW - 1_000, t, NOW)).toBe("just now");
    expect(formatRelativeUpdated(NOW - 59_000, t, NOW)).toBe("just now");
  });

  it("renders minutes for sub-hour diffs", () => {
    expect(formatRelativeUpdated(NOW - 5 * 60_000, t, NOW)).toBe("5 min ago");
    expect(formatRelativeUpdated(NOW - 59 * 60_000, t, NOW)).toBe("59 min ago");
  });

  it("renders hours for sub-day diffs", () => {
    expect(formatRelativeUpdated(NOW - 60 * 60_000, t, NOW)).toBe("1 hr ago");
    expect(formatRelativeUpdated(NOW - 23 * 3600_000, t, NOW)).toBe(
      "23 hr ago",
    );
  });

  it("renders days beyond 24h", () => {
    expect(formatRelativeUpdated(NOW - 24 * 3600_000, t, NOW)).toBe("1 d ago");
    expect(formatRelativeUpdated(NOW - 9 * 24 * 3600_000, t, NOW)).toBe(
      "9 d ago",
    );
  });

  it("defaults `nowMs` to Date.now()", () => {
    const spy = vi.spyOn(Date, "now").mockReturnValue(NOW);
    try {
      expect(formatRelativeUpdated(NOW - 120_000, t)).toBe("2 min ago");
    } finally {
      spy.mockRestore();
    }
  });
});
