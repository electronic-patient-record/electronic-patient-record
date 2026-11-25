import { describe, expect, it } from "vitest";
import {
  type AllergyLoad,
  bannerAllergies,
  STRIP_ALLERGIES,
  stripAllergies,
} from "../../src/lib/allergies";

const load = (
  names: string[],
  high = 0,
  rest: string | null = "low",
): AllergyLoad =>
  ({
    status: 200,
    data: {
      allergies: [],
      statement: null,
      summary: {
        state: "active",
        items: names.map((substance, i) => ({
          substance,
          criticality: i < high ? "high" : rest,
        })),
      },
    },
  }) as unknown as AllergyLoad;

describe("stripAllergies (XP-3)", () => {
  it("is the banner's own words for anything but a long list", () => {
    const other = (summary: unknown, status = 200): AllergyLoad =>
      ({
        status,
        data:
          status === 200 ? { allergies: [], statement: null, summary } : null,
      }) as unknown as AllergyLoad;
    for (const l of [
      null,
      other(null, 403),
      other(null, 500),
      other({ state: "not-recorded" }),
      other({ state: "none-active", inactive: 3 }),
      other({ state: "no-known", asserted_at: "2026-10-21T09:00:00Z" }),
      load(["Penicillin", "Latex"], 1),
    ]) {
      expect(stripAllergies(l, "UTC")).toEqual(bannerAllergies(l, "UTC"));
    }
  });

  it("gives what fits of a long list, and always the number left out", () => {
    const names = [...Array(9).keys()].map((i) => `Substance ${i}`);
    const { text, high } = stripAllergies(load(names, 1));
    expect(high).toBe(true);
    expect(text.startsWith("Substance 0 (high risk), Substance 1")).toBe(true);
    const shown = text.split(", ").length - 1;
    expect(text.endsWith(`+${9 - shown}`)).toBe(true);
    expect(text.length).toBeLessThanOrEqual(STRIP_ALLERGIES + 6);
  });

  it("names every high-risk allergy, however many, and counts only low-risk ones", () => {
    const names = [...Array(12).keys()].map((i) => `Substance number ${i}`);
    const { text, high } = stripAllergies(load(names, 9));
    expect(high).toBe(true);
    for (const n of names.slice(0, 9)) {
      expect(text).toContain(`${n} (high risk)`);
    }
    expect(text.endsWith(", +3")).toBe(true);
    // Wherever it is in the list.
    const last = load(names);
    const items = (
      last.data?.summary as { items: { criticality: string | null }[] }
    ).items;
    if (items[11]) items[11].criticality = "high";
    expect(stripAllergies(last).text).toMatch(
      /^Substance number 11 \(high risk\), .*\+\d+$/,
    );
  });

  it("names one that is too long by itself, and the number after it", () => {
    const long = "x".repeat(100);
    expect(stripAllergies(load([long, "Latex"])).text).toBe(
      `${long} (low risk), +1`,
    );
    expect(stripAllergies(load([long])).text).toBe(`${long} (low risk)`);
  });

  it("names every allergy whose risk is not known, since that is not low", () => {
    const names = [...Array(12).keys()].map((i) => `Substance number ${i}`);
    for (const unknown of [
      null,
      "unable-to-assess",
      "a-word-it-does-not-know",
    ]) {
      const { text } = stripAllergies(load(names, 1, unknown));
      for (const n of names) expect(text, String(unknown)).toContain(n);
      expect(text, String(unknown)).not.toMatch(/\+\d+$/);
    }
    // High-risk ones first, then the unknown, then what fits of the low.
    const mixed = load(["Low", "Unstated", "High"]);
    const items = (
      mixed.data?.summary as {
        items: { substance: string; criticality: string | null }[];
      }
    ).items;
    if (items[1]) items[1].criticality = null;
    if (items[2]) items[2].criticality = "high";
    if (items[0]) items[0].substance = "Low".repeat(30);
    expect(stripAllergies(mixed).text).toBe("High (high risk), Unstated, +1");
  });
});
