import { afterEach, describe, expect, it } from "vitest";
import { clock } from "../../src/lib/data-age";
import { whenText } from "../../src/lib/access-log";
import { ageOn } from "../../src/lib/patient";

// RC-9: the same instant must read the same under any process time zone, except where the page
// deliberately shows the reader's own zone, and then it must name it.
const ZONES = ["UTC", "America/New_York", "Pacific/Auckland"];
const original = process.env.TZ;
afterEach(() => {
  if (original === undefined) delete process.env.TZ;
  else process.env.TZ = original;
});
const under = <T>(tz: string, f: () => T): T => {
  process.env.TZ = tz;
  return f();
};

describe("time zones (RC-9)", () => {
  it("server times shown as UTC do not move with the reader's zone", () => {
    for (const tz of ZONES)
      expect(under(tz, () => whenText("2026-10-21T23:30:00.000000Z"))).toBe("2026-10-21 23:30 UTC");
  });
  it("the read-at clock is in the reader's zone and names it", () => {
    const at = new Date("2026-10-24T10:00:00Z");
    expect(under("UTC", () => clock(at, "en-GB"))).toBe("10:00 UTC");
    expect(under("America/New_York", () => clock(at, "en-GB"))).toMatch(/^06:00 (GMT-4|EDT)$/);
    expect(under("Pacific/Auckland", () => clock(at, "en-GB"))).toMatch(/^23:00 (GMT\+13|NZDT)$/);
  });
  it("the clock follows a daylight-saving change, not a fixed offset", () => {
    // New York left daylight time on 2026-11-01 at 06:00 UTC.
    const before = new Date("2026-11-01T05:30:00Z");
    const after = new Date("2026-11-01T06:30:00Z");
    expect(under("America/New_York", () => clock(before, "en-GB"))).toMatch(/^01:30 /);
    expect(under("America/New_York", () => clock(after, "en-GB"))).toMatch(/^01:30 /);
    expect(under("America/New_York", () => clock(before, "en-GB"))).not.toBe(
      under("America/New_York", () => clock(after, "en-GB")),
    );
  });
  it("age is counted on the UTC date, whatever the zone (current behaviour, not a claim it is ideal)", () => {
    // 2026-10-24 23:30 in New York is already the 25th in UTC: a person born 2025-10-25 is 12 months.
    const now = new Date("2026-10-25T03:30:00Z");
    for (const tz of ZONES) expect(under(tz, () => ageOn("2025-10-25", now))).toEqual({ months: 12, approximate: false });
  });
});
