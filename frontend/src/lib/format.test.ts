import { describe, expect, it } from "vitest";

import { fmtProgress, fmtSp, timeAgo, timeFraction } from "./format";

describe("format helpers", () => {
  it("pairs percentage with its fraction", () => {
    expect(fmtProgress(210, 362)).toBe("58% — 210/362 SP");
    expect(fmtProgress(0, 0)).toBe("0% — 0/0 SP");
  });

  it("trims whole story points, keeps halves", () => {
    expect(fmtSp(8)).toBe("8");
    expect(fmtSp(2.5)).toBe("2.5");
  });

  it("computes clamped time fractions", () => {
    expect(timeFraction("2026-01-01", "2026-01-11", "2026-01-06")).toBeCloseTo(0.5);
    expect(timeFraction("2026-01-01", "2026-01-11", "2025-12-01")).toBe(0);
    expect(timeFraction("2026-01-01", "2026-01-11", "2026-02-01")).toBe(1);
  });

  it("reports never for missing timestamps", () => {
    expect(timeAgo(null)).toBe("never");
  });
});
