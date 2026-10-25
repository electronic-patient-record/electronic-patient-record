import { describe, expect, it } from "vitest";
import { conflictList, conflictText, type ConflictCode } from "../../src/lib/conflicts";
import { LOCALES } from "../../src/lib/locales";

// The nine codes, with the exact English the matcher (0.3.0) writes for each beside them.
const CODES: [ConflictCode, string][] = [
  [{ code: "different_nhs_numbers" }, "different verified NHS numbers"],
  [{ code: "same_nhs_different_birth_dates" }, "same NHS number but different birth dates"],
  [{ code: "same_nhs_different_names" }, "same NHS number but different names"],
  [{ code: "names_sound_alike" }, "names sound alike but are spelled differently"],
  [{ code: "names_swapped" }, "given and family names appear swapped"],
  [{ code: "extra_given_name" }, "one record has a given name the other lacks"],
  [{ code: "field_unavailable", field: "birth_date" }, "birth date not available to confirm the link"],
  [{ code: "field_below_minimum", field: "family_name", similarity: 0.42, minimum: 0.9 }, "family name similarity 0.42 below 0.90"],
  [{ code: "evidence_thin", percent_present: 40 }, "only 40% of the evidence is present"],
];

describe("conflict codes in every locale (CQ-38; translations machine-written, unreviewed)", () => {
  it("English says exactly what the matcher's English text says", () => {
    for (const [code, english] of CODES) expect(conflictText(code, "en-001")).toBe(english);
  });
  for (const locale of LOCALES.filter((l) => l !== "en-001")) {
    it(`${locale}: each of the nine codes has a sentence of its own, with its numbers`, () => {
      const seen = new Set<string>();
      for (const [code, english] of CODES) {
        const t = conflictText(code, locale);
        expect(t, `${locale} ${code.code}`).toBeTruthy();
        expect(t, `${locale} ${code.code} is still English`).not.toBe(english);
        expect(t).not.toContain("{");
        seen.add(t!);
      }
      expect(seen.size).toBe(CODES.length);
      // The numbers survive, in the locale's own digits.
      const digits = (n: number, d: number) =>
        new Intl.NumberFormat(locale, { minimumFractionDigits: d, maximumFractionDigits: d }).format(n);
      expect(conflictText(CODES[7]![0], locale)).toContain(digits(0.42, 2));
      expect(conflictText(CODES[7]![0], locale)).toContain(digits(0.9, 2));
      expect(conflictText(CODES[8]![0], locale)).toContain(digits(40, 0));
    });
  }
  it("falls back to the English text for a code, a field or a number it does not know, never to nothing", () => {
    const english = ["a reason from a newer matcher", "another"];
    expect(conflictList(english, [{ code: "brand_new" }, { code: "field_unavailable", field: "shoe_size" }], "de-001")).toEqual(english);
    expect(conflictText({ code: "evidence_thin", percent_present: Number.NaN }, "de-001")).toBeNull();
    // One known and one unknown: the known is translated, the other keeps its English.
    const mixed = conflictList(["different verified NHS numbers", "a newer reason"], [{ code: "different_nhs_numbers" }, { code: "brand_new" }], "de-001");
    expect(mixed).toEqual(["unterschiedliche geprüfte NHS-Nummern", "a newer reason"]);
  });
  it("shows the English list whole when the codes are missing or do not match it in length", () => {
    const english = ["one", "two"];
    expect(conflictList(english, undefined, "de-001")).toBe(english);
    expect(conflictList(english, [{ code: "names_swapped" }], "de-001")).toBe(english);
    expect(conflictList([], [], "de-001")).toEqual([]);
  });
});
