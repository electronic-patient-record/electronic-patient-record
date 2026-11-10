import { describe, expect, it } from "vitest";
import { buildWatchEntry } from "../../src/lib/watch-list";

describe("adding to a watch list", () => {
  it("needs a reason, and sends an end day only when one is given", () => {
    expect(buildWatchEntry("  ", "").problem).toBe("reason");
    expect(buildWatchEntry(" Raised troponin ", "").body).toEqual({
      entry: { reason: "Raised troponin" },
    });
    expect(buildWatchEntry("Discharge follow-up", "2026-12-01").body).toEqual({
      entry: { reason: "Discharge follow-up", until: "2026-12-01" },
    });
  });
});
