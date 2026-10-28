import { describe, expect, it } from "vitest";
import { clock, dataAge, STALE_AFTER_MS } from "../../src/lib/data-age";

describe("dataAge (EC-12)", () => {
  const read = new Date("2026-10-24T10:00:00Z");
  const at = (ms: number) => new Date(read.getTime() + ms);

  it("says nothing before anything was read", () => {
    expect(dataAge(null, read, false)).toBeNull();
    expect(dataAge(null, read, true)).toBeNull();
  });
  it("is current just after a read and while the last look worked", () => {
    expect(dataAge(read, read, false, "en-GB")?.stale).toBe(false);
    expect(dataAge(read, at(STALE_AFTER_MS), false, "en-GB")?.stale).toBe(
      false,
    );
  });
  it("is doubtful once the time since the read passes the limit", () => {
    expect(dataAge(read, at(STALE_AFTER_MS + 1), false)?.stale).toBe(true);
  });
  it("is doubtful whenever the last look failed, however fresh the earlier read", () => {
    expect(dataAge(read, at(1000), true)?.stale).toBe(true);
  });
  it("shows the time of the read, not of now", () => {
    const shown = dataAge(read, at(10 * 60_000), false, "en-GB");
    expect(shown?.readAt).toBe(clock(read, "en-GB"));
  });
});
