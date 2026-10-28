import { describe, expect, it } from "vitest";
import {
  VITALS,
  assessmentText,
  enteredInErrorRecord,
  formToRequest,
  noticeText,
  nowInZone,
  parseNumber,
  rangeText,
  trends,
  unitLabel,
  valueText,
  vitalLine,
  vitalName,
  wallTimeToUtc,
  type Notice,
  type StoredObservation,
  type VitalForm,
} from "../../src/lib/vitals";

const base: StoredObservation = {
  id: "o1",
  patient_id: "p1",
  version: 1,
  author: "nurse-1",
  recorded_at: "2026-10-26T10:00:00.000000Z",
  site_zone: "America/New_York",
  code: "8867-4",
  value: 72,
  unit: "/min",
  effective_at: "2026-10-26T13:30:00.000000Z",
  status: "final",
  flag: null,
};

const form: VitalForm = {
  vital: "heartRate",
  value: "72",
  noValue: false,
  absentReason: "",
  low: "",
  high: "",
  measuredAt: "2026-10-26T09:30",
  siteZone: "America/New_York",
};

describe("a number from the form (H-034: an empty box is never 0)", () => {
  it("reads plain numbers, a decimal comma, and a sign", () => {
    expect(parseNumber("72")).toBe(72);
    expect(parseNumber(" 36.6 ")).toBe(36.6);
    expect(parseNumber("36,6")).toBe(36.6);
    expect(parseNumber("-1")).toBe(-1);
    expect(parseNumber("0")).toBe(0);
  });
  it("refuses everything else, so nothing becomes 0 by accident", () => {
    for (const bad of [
      "",
      "  ",
      "abc",
      "7 2",
      "1e3",
      "0x10",
      ".5",
      "5.",
      "--1",
      "1,2,3",
      "Infinity",
      "NaN",
      "72bpm",
    ]) {
      expect(parseNumber(bad), JSON.stringify(bad)).toBeNull();
    }
  });
});

describe("what a row says", () => {
  it("names the vital, or shows the code itself for one it does not know", () => {
    expect(vitalName("8867-4")).toBe("Heart rate");
    expect(vitalName("2160-0")).toBe("2160-0");
    for (const v of VITALS) expect(vitalName(v.loinc)).not.toBe(v.loinc);
  });
  it("shows the unit as written, and an unknown unit as it came", () => {
    expect(unitLabel("mm[Hg]")).toBe("mmHg");
    expect(unitLabel("Cel")).toBe("°C");
    expect(unitLabel("furlongs")).toBe("furlongs");
    expect(valueText(base)).toBe("72 /min");
    expect(valueText({ ...base, value: 36.6, unit: "Cel" })).toBe("36.6 °C");
  });
  it("says why there is no value, and never shows 0 or a dash for it", () => {
    for (const [reason, word] of [
      ["not-done", "not done"],
      ["declined", "declined"],
      ["failed", "attempt failed"],
      ["unknown", "not known"],
    ] as const) {
      const t = valueText({
        ...base,
        value: null,
        unit: null,
        absent_reason: reason,
      });
      expect(t).toBe(`No value: ${word}`);
    }
    expect(
      valueText({ ...base, value: null, unit: null, absent_reason: null }),
    ).toBe("No value: no reason given");
    expect(valueText({ ...base, value: null })).not.toMatch(/\b0\b|—|-/);
  });
  it("a real zero is shown as 0, because it is a value", () => {
    expect(valueText({ ...base, value: 0 })).toBe("0 /min");
  });
});

describe("where a value sits (H-034: never normal unless the API computed it from a range)", () => {
  it("says each flag in words", () => {
    expect(assessmentText({ ...base, flag: "critical-low" })).toBe(
      "Critically low",
    );
    expect(assessmentText({ ...base, flag: "low" })).toBe("Low");
    expect(assessmentText({ ...base, flag: "normal" })).toBe(
      "Within the range given",
    );
    expect(assessmentText({ ...base, flag: "high" })).toBe("High");
    expect(assessmentText({ ...base, flag: "critical-high" })).toBe(
      "Critically high",
    );
  });
  it("says not assessed, and why, for no flag; never normal", () => {
    expect(assessmentText({ ...base, flag: null })).toBe(
      "Not assessed (no range given)",
    );
    expect(assessmentText({ ...base, flag: undefined as never })).toBe(
      "Not assessed (no range given)",
    );
    expect(
      assessmentText({
        ...base,
        value: null,
        absent_reason: "failed",
        flag: null,
      }),
    ).toBe("Not assessed (no value)");
    // Even a flag the API sent cannot make a missing value normal.
    expect(assessmentText({ ...base, value: null, flag: "normal" })).toBe(
      "Not assessed (no value)",
    );
    expect(assessmentText({ ...base, flag: null })).not.toMatch(
      /normal|ok|within/i,
    );
  });
  it("shows the range given in words", () => {
    expect(rangeText({ low: 60, high: 100 })).toBe("60 to 100");
    expect(rangeText({ low: 60 })).toBe("from 60");
    expect(rangeText({ high: 100 })).toBe("up to 100");
    expect(rangeText({})).toBeNull();
    expect(rangeText(null)).toBeNull();
    expect(rangeText(undefined)).toBeNull();
  });
  it("one line holds the name, value, assessment and the site's time with UTC beside it", () => {
    expect(vitalLine({ ...base, flag: "high" }, "America/New_York")).toMatch(
      /^Heart rate · 72 \/min · High · 2026-10-26 09:30 (EDT|GMT-4) \(13:30 UTC\)$/,
    );
    expect(vitalLine(base)).toBe(
      "Heart rate · 72 /min · Not assessed (no range given) · 2026-10-26 13:30 UTC",
    );
  });
});

describe("the measured time is the site's wall clock, sent as UTC", () => {
  it("converts New York time, in summer and winter", () => {
    expect(wallTimeToUtc("2026-10-26T09:30", "America/New_York")).toBe(
      "2026-10-26T13:30:00Z",
    );
    expect(wallTimeToUtc("2026-12-01T09:30", "America/New_York")).toBe(
      "2026-12-01T14:30:00Z",
    );
    expect(wallTimeToUtc("2026-10-26T09:30", "UTC")).toBe(
      "2026-10-26T09:30:00Z",
    );
    expect(wallTimeToUtc("2026-10-26T09:30", "Pacific/Auckland")).toBe(
      "2026-10-25T20:30:00Z",
    );
  });
  it("is exact on either side of a clock change", () => {
    // New York went forward at 02:00 EST on 2026-03-08: 01:59 is EST, 03:00 is EDT.
    expect(wallTimeToUtc("2026-03-08T01:59", "America/New_York")).toBe(
      "2026-03-08T06:59:00Z",
    );
    expect(wallTimeToUtc("2026-03-08T03:00", "America/New_York")).toBe(
      "2026-03-08T07:00:00Z",
    );
    // It went back at 02:00 EDT on 2026-11-01: 00:59 is EDT, 02:00 is EST.
    expect(wallTimeToUtc("2026-11-01T00:59", "America/New_York")).toBe(
      "2026-11-01T04:59:00Z",
    );
    expect(wallTimeToUtc("2026-11-01T02:00", "America/New_York")).toBe(
      "2026-11-01T07:00:00Z",
    );
  });
  it("reads the skipped hour as an hour earlier and the repeated hour as the first of two", () => {
    expect(wallTimeToUtc("2026-03-08T02:30", "America/New_York")).toBe(
      "2026-03-08T06:30:00Z",
    );
    expect(wallTimeToUtc("2026-11-01T01:30", "America/New_York")).toBe(
      "2026-11-01T05:30:00Z",
    );
  });
  it("gives null for anything that is not a real time or zone", () => {
    for (const bad of [
      "",
      "2026-10-26",
      "2026-10-26 09:30",
      "2026-02-30T09:30",
      "2026-10-26T25:00",
      "2026-13-01T09:30",
      "soon",
    ]) {
      expect(wallTimeToUtc(bad, "UTC"), bad).toBeNull();
    }
    expect(wallTimeToUtc("2026-10-26T09:30", "Not/AZone")).toBeNull();
  });
  it("shows now on the site's clock for the form's default", () => {
    const at = new Date("2026-10-26T13:30:45Z");
    expect(nowInZone(at, "America/New_York")).toBe("2026-10-26T09:30");
    expect(nowInZone(at, "UTC")).toBe("2026-10-26T13:30");
    expect(nowInZone(at, "Pacific/Auckland")).toBe("2026-10-27T02:30");
  });
});

describe("the form as a request", () => {
  it("sends the code, the vital's own unit, the UTC time and the number", () => {
    const r = formToRequest(form);
    expect(r).toEqual({
      ok: true,
      record: {
        code: "8867-4",
        effective_at: "2026-10-26T13:30:00Z",
        status: "final",
        value: 72,
        unit: "/min",
      },
    });
  });
  it("every vital sends its own fixed unit; the form cannot choose another", () => {
    for (const v of VITALS) {
      const r = formToRequest({ ...form, vital: v.key });
      expect(r.ok && r.record.unit, v.key).toBe(v.unit);
      expect(r.ok && r.record.code, v.key).toBe(v.loinc);
    }
  });
  it("never sends an empty or unreadable value as 0", () => {
    for (const bad of ["", " ", "abc", "1e3"]) {
      const r = formToRequest({ ...form, value: bad });
      expect(r.ok, JSON.stringify(bad)).toBe(false);
    }
    expect(formToRequest({ ...form, value: "0" }).ok).toBe(true);
  });
  it("sends no value and a reason when there is none, and refuses that without a reason", () => {
    const r = formToRequest({
      ...form,
      value: "999",
      noValue: true,
      absentReason: "declined",
    });
    expect(r).toEqual({
      ok: true,
      record: {
        code: "8867-4",
        effective_at: "2026-10-26T13:30:00Z",
        status: "final",
        absent_reason: "declined",
      },
    });
    const none = formToRequest({ ...form, noValue: true, absentReason: "" });
    expect(none.ok).toBe(false);
  });
  it("sends the range only as the numbers given, each end optional, and refuses an unreadable end", () => {
    const both = formToRequest({ ...form, low: "60", high: "100" });
    expect(both.ok && both.record.range).toEqual({ low: 60, high: 100 });
    const lowOnly = formToRequest({ ...form, low: "60" });
    expect(lowOnly.ok && lowOnly.record.range).toEqual({ low: 60 });
    const none = formToRequest(form);
    expect(none.ok && "range" in none.record).toBe(false);
    for (const bad of [
      { low: "x" },
      { high: "1e2" },
      { low: "60", high: "" + "abc" },
    ]) {
      expect(formToRequest({ ...form, ...bad }).ok).toBe(false);
    }
  });
  it("refuses a time that is not real", () => {
    expect(formToRequest({ ...form, measuredAt: "" }).ok).toBe(false);
    expect(formToRequest({ ...form, measuredAt: "2026-02-30T09:30" }).ok).toBe(
      false,
    );
  });
});

describe("enteredInErrorRecord", () => {
  it("keeps every stored field and changes only the status", () => {
    const o = { ...base, range: { low: 60, high: 100 }, critical: { low: 40 } };
    expect(enteredInErrorRecord(o)).toEqual({
      code: "8867-4",
      value: 72,
      unit: "/min",
      range: { low: 60, high: 100 },
      critical: { low: 40 },
      effective_at: "2026-10-26T13:30:00.000000Z",
      status: "entered-in-error",
    });
  });
  it("keeps a missing value missing with its reason, never as 0", () => {
    const r = enteredInErrorRecord({
      ...base,
      value: null,
      unit: null,
      absent_reason: "failed",
    });
    expect(r).not.toHaveProperty("value");
    expect(r).not.toHaveProperty("unit");
    expect(r.absent_reason).toBe("failed");
  });
});

describe("trends", () => {
  const at = (
    n: number,
    over: Partial<StoredObservation> = {},
  ): StoredObservation => ({
    ...base,
    id: `o${n}`,
    effective_at: `2026-10-26T1${n}:00:00Z`,
    ...over,
  });

  it("groups by vital in the page's order, newest first, and leaves out entries in error", () => {
    const out = trends([
      at(1, { code: "8310-5", unit: "Cel", value: 36.6 }),
      at(1, { value: 70 }),
      at(3, { value: 90 }),
      at(2, { value: 80 }),
      at(4, { value: 999, status: "entered-in-error" }),
    ]);
    expect(out.map((t) => t.code)).toEqual(["8867-4", "8310-5"]);
    expect(out[0]!.rows.map((o) => o.value)).toEqual([90, 80, 70]);
  });

  it("keeps a reading with no value as a row, so a gap is not a missing reading or a zero", () => {
    const out = trends([
      at(1, { value: null, unit: null, absent_reason: "declined" }),
      at(2),
    ]);
    expect(out[0]!.rows).toHaveLength(2);
    expect(valueText(out[0]!.rows[1]!)).toBe("No value: declined");
  });

  it("puts a code the page does not know last, under its code, and nothing at all gives nothing", () => {
    const out = trends([at(1, { code: "99999-9" }), at(2)]);
    expect(out.map((t) => t.name)).toEqual(["Heart rate", "99999-9"]);
    expect(trends([])).toEqual([]);
  });
});

describe("the notice beside a disputed record (RC-4, H-035)", () => {
  it("tells a pending request without saying it was wrong or right", () => {
    expect(noticeText({ notice: "pending" })).toBe(
      "The patient has asked for this record to be corrected. No decision has been recorded.",
    );
  });

  it("names the version a correction was made in", () => {
    expect(noticeText({ notice: "amended", version: 3 })).toContain(
      "version 3",
    );
  });

  it("gives a refusal with the ground and the patient's statement, as the patient wrote it", () => {
    const text = noticeText({
      notice: "refused",
      ground: "accurate-and-complete",
      disagreement: "  My pulse was never that fast.  ",
    });
    expect(text).toContain("the record is accurate and complete as it stands.");
    expect(text).toContain(
      "The patient's statement of disagreement: My pulse was never that fast.",
    );
  });

  it("says there is no statement when none was given, and never drops a ground it does not know", () => {
    expect(
      noticeText({ notice: "refused", ground: "other", disagreement: null }),
    ).toContain("The patient has given no statement.");
    expect(noticeText({ notice: "refused", ground: "new-ground" })).toContain(
      "for a reason this page cannot show.",
    );
  });

  it("still tells a notice kind it does not know", () => {
    expect(
      noticeText({ notice: "something-new" } as unknown as Notice),
    ).toContain("There is a request to correct this record.");
  });
});
