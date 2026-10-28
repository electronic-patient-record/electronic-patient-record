import { afterEach, describe, expect, it } from "vitest";
import { clock } from "../../src/lib/data-age";
import { siteWhenText, whenText } from "../../src/lib/access-log";
import { ageOn, calendarDay } from "../../src/lib/patient";

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
  it("a site time shows the site's clock with the UTC time beside it, under any process zone", () => {
    const at = "2026-10-21T13:30:00.000000Z";
    for (const tz of ZONES) {
      expect(under(tz, () => siteWhenText(at, "America/New_York"))).toMatch(
        /^2026-10-21 09:30 (EDT|GMT-4) \(13:30 UTC\)$/,
      );
      expect(under(tz, () => siteWhenText(at, "Pacific/Auckland"))).toMatch(
        /^2026-10-22 02:30 (NZDT|GMT\+13) \(13:30 UTC\)$/,
      );
    }
  });
  it("a site time follows a daylight-saving change, not a fixed offset", () => {
    expect(siteWhenText("2026-03-08T06:59:00Z", "America/New_York")).toMatch(
      /^2026-03-08 01:59 (EST|GMT-5) \(06:59 UTC\)$/,
    );
    expect(siteWhenText("2026-03-08T07:01:00Z", "America/New_York")).toMatch(
      /^2026-03-08 03:01 (EDT|GMT-4) \(07:01 UTC\)$/,
    );
  });
  it("no zone, UTC, an unknown zone, or something that is not a time says what it always did", () => {
    const at = "2026-10-21T13:30:00.000000Z";
    expect(siteWhenText(at)).toBe("2026-10-21 13:30 UTC");
    expect(siteWhenText(at, "UTC")).toBe("2026-10-21 13:30 UTC");
    expect(siteWhenText(at, "Not/AZone")).toBe("2026-10-21 13:30 UTC");
    expect(siteWhenText("soon", "America/New_York")).toBe("soon");
  });
  it("server times shown as UTC do not move with the reader's zone", () => {
    for (const tz of ZONES)
      expect(under(tz, () => whenText("2026-10-21T23:30:00.000000Z"))).toBe(
        "2026-10-21 23:30 UTC",
      );
  });
  it("the read-at clock is in the reader's zone and names it", () => {
    const at = new Date("2026-10-24T10:00:00Z");
    expect(under("UTC", () => clock(at, "en-GB"))).toBe("10:00 UTC");
    expect(under("America/New_York", () => clock(at, "en-GB"))).toMatch(
      /^06:00 (GMT-4|EDT)$/,
    );
    expect(under("Pacific/Auckland", () => clock(at, "en-GB"))).toMatch(
      /^23:00 (GMT\+13|NZDT)$/,
    );
  });
  it("the clock follows a daylight-saving change, not a fixed offset", () => {
    // New York left daylight time on 2026-11-01 at 06:00 UTC.
    const before = new Date("2026-11-01T05:30:00Z");
    const after = new Date("2026-11-01T06:30:00Z");
    expect(under("America/New_York", () => clock(before, "en-GB"))).toMatch(
      /^01:30 /,
    );
    expect(under("America/New_York", () => clock(after, "en-GB"))).toMatch(
      /^01:30 /,
    );
    expect(under("America/New_York", () => clock(before, "en-GB"))).not.toBe(
      under("America/New_York", () => clock(after, "en-GB")),
    );
  });
  it("age is counted on the site's calendar day, the same for every reader whatever their own zone", () => {
    // 03:30 UTC on 25 October is still the evening of the 24th in New York.
    const now = new Date("2026-10-25T03:30:00Z");
    for (const tz of ZONES) {
      // The reader's zone does not matter; the site's does.
      expect(under(tz, () => ageOn("2025-10-25", now, "UTC"))).toEqual({
        months: 12,
        approximate: false,
      });
      expect(
        under(tz, () => ageOn("2025-10-25", now, "America/New_York")),
      ).toEqual({ months: 11, approximate: false });
      expect(
        under(tz, () => ageOn("2025-10-25", now, "Pacific/Auckland")),
      ).toEqual({ months: 12, approximate: false });
    }
    // No zone given is UTC, as before; an unknown zone falls back to UTC rather than failing the banner.
    expect(ageOn("2025-10-25", now)).toEqual({
      months: 12,
      approximate: false,
    });
    expect(ageOn("2025-10-25", now, "Not/AZone")).toEqual({
      months: 12,
      approximate: false,
    });
  });
  it("a birthday arrives at the site's midnight", () => {
    // Born 2020-03-01: five on 1 March in the site's zone, four the day before, and the leap day counts.
    const london = (iso: string) =>
      ageOn("2020-03-01", new Date(iso), "Europe/London")?.years;
    expect(london("2025-02-28T23:30:00Z")).toBe(4);
    expect(london("2025-03-01T00:30:00Z")).toBe(5);
    // 2026-03-29 00:30 UTC is 01:30 BST in London, after the clocks went forward: same day, same age.
    expect(london("2026-03-29T00:30:00Z")).toBe(6);
  });
  it("calendarDay is the day in the zone, and UTC for a zone it does not know", () => {
    const t = new Date("2026-10-25T23:30:00Z");
    expect(calendarDay(t, "UTC")).toEqual({ year: 2026, month: 10, day: 25 });
    expect(calendarDay(t, "Pacific/Auckland")).toEqual({
      year: 2026,
      month: 10,
      day: 26,
    });
    expect(calendarDay(t, "America/New_York")).toEqual({
      year: 2026,
      month: 10,
      day: 25,
    });
    expect(calendarDay(t, "nonsense")).toEqual({
      year: 2026,
      month: 10,
      day: 25,
    });
  });
});
