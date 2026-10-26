import { describe, expect, it } from "vitest";
import { conflictList, conflictText, notSamePersonText, patientProblemList, refusalText, type ConflictCode } from "../../src/lib/conflicts";
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

describe("a refused merge in the page's language", () => {
  it("leads with 'these do not look like the same person' and lists each reason, in every locale", () => {
    for (const locale of LOCALES) {
      const t = notSamePersonText([{ code: "different_nhs_numbers" }, { code: "names_swapped" }], locale);
      expect(t, locale).toContain(conflictText({ code: "different_nhs_numbers" }, locale)!);
      expect(t).toContain("; ");
      if (locale !== "en-001") expect(t).not.toMatch(/^These do not look/);
    }
    expect(notSamePersonText([{ code: "names_swapped" }], "en-001")).toBe(
      "These do not look like the same person: given and family names appear swapped",
    );
  });
  it("gives null, so the API's English message shows whole, when there are no codes or one is unknown", () => {
    expect(notSamePersonText(undefined, "de-001")).toBeNull();
    expect(notSamePersonText([], "de-001")).toBeNull();
    expect(notSamePersonText([{ code: "names_swapped" }, { code: "brand_new" }], "de-001")).toBeNull();
  });
});

describe("a refused merge or unmerge by code", () => {
  const CODES = [
    "reason_required",
    "same_record",
    "drill_and_real",
    "not_merged",
    "source_already_merged",
    "target_already_merged",
  ];
  it("says every code in every locale, and names the other record where the refusal does", () => {
    for (const locale of LOCALES) {
      for (const code of CODES) {
        const t = refusalText(code, "abc-123", locale);
        expect(t, `${locale} ${code}`).toBeTruthy();
        expect(t).not.toContain("{id}");
      }
      expect(refusalText("source_already_merged", "abc-123", locale)).toContain("abc-123");
      expect(refusalText("target_already_merged", "abc-123", locale)).toContain("abc-123");
    }
    expect(refusalText("same_record", undefined, "en-001")).toBe("A record cannot be merged into itself.");
    expect(refusalText("not_merged", undefined, "de-001")).not.toMatch(/^That record/);
  });
  it("gives null, so the API's English shows, for no code, an unknown code, or a missing id", () => {
    expect(refusalText(undefined, undefined, "de-001")).toBeNull();
    expect(refusalText("brand_new", undefined, "de-001")).toBeNull();
    expect(refusalText("source_already_merged", undefined, "de-001")).toBeNull();
    expect(refusalText("source_already_merged", "", "de-001")).toBeNull();
  });
});

describe("a refused patient", () => {
  const CODES = ["family_name_required", "birth_date_invalid", "nhs_number_placeholder", "nhs_number_check_digit", "identifier_incomplete"];
  const english = CODES.map((c) => `english ${c}`);
  it("says every code in every translated locale, and leaves English readers the API's own words", () => {
    for (const locale of LOCALES) {
      const t = patientProblemList(english, CODES, locale);
      expect(t).toHaveLength(CODES.length);
      if (locale.startsWith("en")) expect(t, locale).toEqual(english);
      else t.forEach((line, i) => expect(line, `${locale} ${CODES[i]}`).not.toMatch(/^english /));
    }
  });
  it("keeps the API's English for an unknown code, and the whole English list when the codes do not line up", () => {
    expect(patientProblemList(["a", "b"], ["brand_new", "family_name_required"], "de-001")[0]).toBe("a");
    expect(patientProblemList(["a", "b"], ["family_name_required"], "de-001")).toEqual(["a", "b"]);
    expect(patientProblemList(["a"], undefined, "de-001")).toEqual(["a"]);
    expect(patientProblemList(["a"], [7], "de-001")).toEqual(["a"]);
  });
});
