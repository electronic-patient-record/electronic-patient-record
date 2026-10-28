import { describe, expect, it } from "vitest";
import { LOCALES } from "../../src/lib/locales";
import {
  LOG_FORM,
  LOG_FORM_ENGLISH,
  LOG_FORM_KEYS,
  logFormStrings,
} from "../../src/lib/translations";
import { LOG_PURPOSES } from "../../src/lib/access-log";

// The words are machine-written and unreviewed. These tests check shape, not quality: every locale has
// every word, keeps the {zone} slot, and is not English passed off as a translation.
const OTHERS = LOCALES.filter((l) => l !== "en-001");

describe("access-log filter form translations (machine-written, not reviewed)", () => {
  it("covers every locale but English, which falls back to the English table", () => {
    expect(Object.keys(LOG_FORM).sort()).toEqual([...OTHERS].sort());
    expect(logFormStrings("en-001")).toBe(LOG_FORM_ENGLISH);
  });
  for (const locale of OTHERS) {
    it(`${locale}: every word is present, differs from English, and the zone slot survives`, () => {
      const t = logFormStrings(locale);
      for (const key of LOG_FORM_KEYS) {
        expect(t[key], `${locale}.${key}`).toBeTruthy();
        expect(t[key], `${locale}.${key} is still English`).not.toBe(
          LOG_FORM_ENGLISH[key],
        );
      }
      expect(t.zone).toContain("{zone}");
      expect(t.zone.split("{zone}").length).toBe(2);
      for (const code of LOG_PURPOSES) {
        expect(t.purposes[code], `${locale}.purposes.${code}`).toBeTruthy();
        expect(t.purposes[code]).not.toBe(LOG_FORM_ENGLISH.purposes[code]);
      }
      expect(Object.keys(t.purposes).sort()).toEqual([...LOG_PURPOSES].sort());
    });
  }
});
