import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { LOCALES } from "../../src/lib/locales";

// `governance/translation-review.md` is the register of who has reviewed which translation. A row
// may say "reviewed" only with a named reviewer and a date, so a translation cannot be called
// reviewed by accident (AGENTS.md).
const text = readFileSync(
  resolve(process.cwd(), "../governance/translation-review.md"),
  "utf8",
);
const rows = text
  .split("\n")
  .filter(
    (l) =>
      l.startsWith("| ") && !l.startsWith("| Locale") && !l.startsWith("| ---"),
  )
  .map((l) =>
    l
      .split("|")
      .slice(1, -1)
      .map((c) => c.trim()),
  );

describe("translation review register", () => {
  it("has exactly one row for every locale", () => {
    expect(rows.map((r) => r[0]).sort()).toEqual([...LOCALES].sort());
  });
  it("uses a known status, and `reviewed` names a reviewer and a date", () => {
    for (const [locale, , status, reviewer, date] of rows) {
      expect(
        ["source", "machine-written", "reviewed"],
        `${locale}: ${status}`,
      ).toContain(status);
      if (status === "reviewed") {
        expect(reviewer, `${locale} reviewer`).not.toMatch(/^(none|n\/a|)$/i);
        expect(date, `${locale} date`).toMatch(/^\d{4}-\d{2}-\d{2}$/);
      } else {
        expect(
          reviewer,
          `${locale}: ${status} must not name a reviewer`,
        ).toMatch(/^(none|n\/a)$/i);
      }
    }
  });
  it("makes English the only source", () => {
    expect(rows.filter((r) => r[2] === "source").map((r) => r[0])).toEqual([
      "en-001",
    ]);
  });
});
