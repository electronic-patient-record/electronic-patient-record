import { describe as suite, expect, it } from "vitest";
import { describe, whenText, type LogEntry } from "../../src/lib/access-log";

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
