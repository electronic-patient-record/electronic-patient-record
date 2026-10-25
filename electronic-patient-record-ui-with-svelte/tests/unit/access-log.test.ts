import { describe as suite, expect, it } from "vitest";
import { dayStart, describe, LOG_PURPOSES, logQuery, NO_FILTER, readerZone, whenText, type LogEntry } from "../../src/lib/access-log";

const e = (over: Partial<LogEntry> = {}): LogEntry => ({
  id: 1,
  at: "2026-10-21T09:00:12.000000Z",
  by: "staff",
  did: "viewed",
  what: "identity record",
  refused: false,
  purpose: "treatment",
  break_glass: false,
  ...over,
});

suite("access log text", () => {
  it("says a member of staff viewed your identity record, with the purpose", () => {
    expect(describe(e())).toBe(
      "A member of staff viewed your identity record. Purpose: treatment.",
    );
  });
  it("adds the role and organisation the token carried, when recorded", () => {
    expect(
      describe(e({ roles: ["clinician"], organisations: ["riverside"] })),
    ).toBe(
      "A member of staff (clinician, riverside) viewed your identity record. Purpose: treatment.",
    );
    expect(describe(e({ roles: ["new-role"] }))).toContain("(new-role)");
  });
  it("does not call a proxy a member of staff", () => {
    expect(describe(e({ roles: ["proxy"], organisations: ["riverside"] }))).toBe(
      "Someone acting as a proxy viewed your identity record. Purpose: treatment.",
    );
  });
  it("says you when it was you, and says a refusal plainly", () => {
    expect(describe(e({ by: "you", purpose: null }))).toBe(
      "You viewed your identity record.",
    );
    expect(
      describe(e({ did: "was refused access to", refused: true })),
    ).toContain("was refused access to your identity record");
  });
  it("puts the purpose in words, and shows a code it does not know as it is", () => {
    expect(describe(e({ purpose: "operations" }))).toContain("Purpose: running the service.");
    expect(describe(e({ purpose: "patient-access" }))).toContain("Purpose: you or your proxy looking at the record.");
    expect(describe(e({ purpose: "new-purpose" }))).toContain("Purpose: new-purpose.");
  });
  it("flags an emergency opening", () => {
    expect(describe(e({ break_glass: true }))).toContain(
      "Opened in an emergency.",
    );
  });
  it("shows words it does not know instead of hiding them", () => {
    expect(describe(e({ did: "exported", what: "photo" }))).toContain(
      "exported your photo",
    );
  });
  it("shortens the timestamp and leaves odd ones alone", () => {
    expect(whenText("2026-10-21T09:00:12.000000Z")).toBe(
      "2026-10-21 09:00 UTC",
    );
    expect(whenText("yesterday")).toBe("yesterday");
  });
});

suite("logQuery (SE-6 filter)", () => {
  it("sends nothing for no filter and the page to continue from", () => {
    expect(logQuery(NO_FILTER)).toBe("");
    expect(logQuery(NO_FILTER, 9)).toBe("?before=9");
  });
  it("sends the reader's last day as the next day, because the API's `to` is exclusive", () => {
    expect(logQuery({ ...NO_FILTER, from: "2026-10-01", to: "2026-10-31" })).toBe("?from=2026-10-01T00%3A00%3A00Z&to=2026-11-01T00%3A00%3A00Z");
    expect(logQuery({ ...NO_FILTER, to: "2026-12-31" })).toBe("?to=2027-01-01T00%3A00%3A00Z");
    expect(logQuery({ ...NO_FILTER, to: "2028-02-28" })).toBe("?to=2028-02-29T00%3A00%3A00Z");
  });
  it("sends who and outcome, and keeps the filter on the next page", () => {
    expect(logQuery({ ...NO_FILTER, by: "staff", refused: "true" }, 4)).toBe("?by=staff&refused=true&before=4");
  });
  it("sends the purpose, and only a code the API knows is offered", () => {
    expect(logQuery({ ...NO_FILTER, purpose: "treatment" })).toBe("?purpose=treatment");
    expect(LOG_PURPOSES).toEqual(["treatment", "emergency", "operations", "payment", "research", "patient-access", "audit"]);
  });
  it("sends nothing at all for a date that is not a real day", () => {
    for (const bad of ["2026-02-30", "2026-13-01", "yesterday", "2026-1-1", "26-01-01"]) {
      expect(logQuery({ ...NO_FILTER, from: bad })).toBeNull();
      expect(logQuery({ ...NO_FILTER, to: bad })).toBeNull();
    }
  });
});

suite("days in the patient's zone (SE-6, RC-9)", () => {
  const range = (from: string, to: string, zone: string) => {
    const q = new URLSearchParams(logQuery({ ...NO_FILTER, from, to }, undefined, zone) ?? "");
    return [q.get("from"), q.get("to")];
  };
  it("starts a day where the patient's midnight is, not UTC's", () => {
    // Auckland is UTC+13 in summer time: its 21 October begins at 11:00 UTC on the 20th.
    expect(range("2026-10-21", "2026-10-21", "Pacific/Auckland")).toEqual(["2026-10-20T11:00:00Z", "2026-10-21T11:00:00Z"]);
    // New York is UTC-4 until 1 November, UTC-5 after.
    expect(range("2026-10-21", "2026-10-21", "America/New_York")).toEqual(["2026-10-21T04:00:00Z", "2026-10-22T04:00:00Z"]);
  });
  it("is right across a clock change, where a day is 23 or 25 hours long", () => {
    // London left summer time at 01:00 UTC on 25 October: that day is 25 hours.
    expect(range("2026-10-25", "2026-10-25", "Europe/London")).toEqual(["2026-10-24T23:00:00Z", "2026-10-26T00:00:00Z"]);
    // New York left it on 1 November: 1 November starts in EDT, 2 November in EST.
    expect(range("2026-11-01", "2026-11-01", "America/New_York")).toEqual(["2026-11-01T04:00:00Z", "2026-11-02T05:00:00Z"]);
    // Auckland entered summer time on 27 September: that day is 23 hours.
    expect(range("2026-09-27", "2026-09-27", "Pacific/Auckland")).toEqual(["2026-09-26T12:00:00Z", "2026-09-27T11:00:00Z"]);
  });
  it("keeps UTC as it was and refuses a day that is not real in any zone", () => {
    expect(dayStart("2026-10-21", "UTC")).toBe(Date.UTC(2026, 9, 21));
    expect(dayStart("2026-02-30", "Europe/London")).toBeNull();
    expect(logQuery({ ...NO_FILTER, to: "2026-13-01" }, undefined, "Europe/London")).toBeNull();
  });
  it("shows an instant in the patient's zone and names the zone", () => {
    expect(whenText("2026-10-21T09:00:12.000000Z", "UTC")).toBe("2026-10-21 09:00 UTC");
    expect(whenText("2026-10-21T09:00:12.000000Z", "Europe/London")).toBe("2026-10-21 10:00 GMT+1");
    expect(whenText("2026-10-21T23:30:00.000000Z", "Pacific/Auckland")).toBe("2026-10-22 12:30 GMT+13");
    expect(whenText("not a time", "Europe/London")).toBe("not a time");
  });
  it("finds a zone, and falls back to UTC rather than none", () => {
    expect(typeof readerZone()).toBe("string");
    expect(readerZone().length).toBeGreaterThan(0);
  });
});
