import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { GAP_KEYS, gapText } from "../../src/lib/decision-ability.js";
import { STRINGS } from "../../src/lib/strings.js";

// The service's own source: each gap the page translates is one it can send, word for word.
const rust = readFileSync(
  join(
    process.cwd(),
    "../electronic-patient-record-service-with-rust/electronic-patient-record-domain/src/decision_ability.rs",
  ),
  "utf8",
);
const gapsFn = rust.slice(
  rust.indexOf("pub fn gaps("),
  rust.indexOf("pub fn age_minutes("),
);
const sent = [...gapsFn.matchAll(/out\.push\("([^"]+)"\)/g)].map((m) => m[1]);

describe("what a judgement does not record", () => {
  it("translates exactly the gaps the service sends", () => {
    expect(sent.length).toBeGreaterThan(0);
    expect(Object.keys(GAP_KEYS).sort()).toEqual([...sent].sort());
  });

  it("shows a gap it does not know as the service said it", () => {
    expect(gapText("whether they were conscious is not known", STRINGS)).toBe(
      STRINGS.daGapConscious,
    );
    expect(gapText("a new gap", STRINGS)).toBe("a new gap");
  });
});
