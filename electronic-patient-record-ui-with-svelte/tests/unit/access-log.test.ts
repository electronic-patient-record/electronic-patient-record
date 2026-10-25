import { describe as suite, expect, it } from "vitest";
import { describe, logQuery, NO_FILTER, whenText, type LogEntry } from "../../src/lib/access-log";

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
    expect(logQuery({ ...NO_FILTER, from: "2026-10-01", to: "2026-10-31" })).toBe("?from=2026-10-01&to=2026-11-01");
    expect(logQuery({ ...NO_FILTER, to: "2026-12-31" })).toBe("?to=2027-01-01");
    expect(logQuery({ ...NO_FILTER, to: "2028-02-28" })).toBe("?to=2028-02-29");
  });
  it("sends who and outcome, and keeps the filter on the next page", () => {
    expect(logQuery({ ...NO_FILTER, by: "staff", refused: "true" }, 4)).toBe("?by=staff&refused=true&before=4");
  });
  it("sends nothing at all for a date that is not a real day", () => {
    for (const bad of ["2026-02-30", "2026-13-01", "yesterday", "2026-1-1", "26-01-01"]) {
      expect(logQuery({ ...NO_FILTER, from: bad })).toBeNull();
      expect(logQuery({ ...NO_FILTER, to: bad })).toBeNull();
    }
  });
});
