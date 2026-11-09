import { render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import PatientBanner from "../../src/lib/PatientBanner.svelte";
import {
  ENGLISH,
  REVIEW,
  setLocaleSource,
  strings,
  TABLES,
  TEXT,
} from "../../src/lib/i18n";
import { LOCALES, type Locale } from "../../src/lib/locales";
import type { Stored } from "../../src/lib/patient";
import { STRINGS } from "../../src/lib/strings";
import { CY } from "../../src/lib/strings.cy";
import { logFormStrings } from "../../src/lib/translations";

vi.mock("../../src/lib/api.js", async (real) => ({
  ...(await real<typeof import("../../src/lib/api")>()),
  fetchBlob: vi.fn(async () => ({ status: 500, body: null })),
}));

// WL-8. The Welsh table is machine-written and unreviewed: these tests check its shape (every key,
// no empty word, every placeholder kept) and that nothing calls it reviewed, never its quality.

/** Every leaf of a table as [dotted key, value]. */
function leaves(t: object, at = ""): [string, unknown][] {
  return Object.entries(t).flatMap(([k, v]) =>
    v !== null && typeof v === "object"
      ? leaves(v as object, `${at}${k}.`)
      : [[`${at}${k}`, v] as [string, unknown]],
  );
}

/** The `{placeholder}` slots of a string, sorted, repeats kept. */
const slots = (s: string) => (s.match(/\{[^}]*\}/g) ?? []).sort();

const english = new Map(leaves(STRINGS) as [string, string][]);

// Words that are the same in Welsh as in English. Anything else equal to English is untranslated.
const SAME_IN_WELSH = new Set(["adjustmentCodes.braille"]);

afterEach(() => setLocaleSource(() => undefined));

describe.each(Object.entries(TABLES))(
  "%s table (machine-written, not reviewed)",
  (locale, table) => {
    const own = new Map(leaves(table));

    it("has exactly the keys of STRINGS, nested ones included", () => {
      expect([...own.keys()].sort()).toEqual([...english.keys()].sort());
    });

    it("gives every key a non-empty string", () => {
      for (const [key, value] of own) {
        expect(typeof value, `${locale}.${key}`).toBe("string");
        expect((value as string).trim(), `${locale}.${key}`).not.toBe("");
      }
    });

    it("keeps every {placeholder}, key by key", () => {
      for (const [key, en] of english) {
        expect(slots(own.get(key) as string), `${locale}.${key}`).toEqual(
          slots(en),
        );
      }
    });

    it("is not English passed off as a translation", () => {
      const same = [...english]
        .filter(([key, en]) => own.get(key) === en && !SAME_IN_WELSH.has(key))
        .map(([key]) => key);
      expect(same).toEqual([]);
    });

    it("has a review record", () => {
      expect(REVIEW[locale as keyof typeof REVIEW]).toBeDefined();
    });
  },
);

describe("review record", () => {
  it("names a reviewer and a date for anything called reviewed", () => {
    for (const [locale, r] of Object.entries(REVIEW)) {
      if (r.reviewed) {
        expect(r.reviewer?.trim(), `${locale} reviewer`).toBeTruthy();
        expect(r.date, `${locale} date`).toMatch(/^\d{4}-\d{2}-\d{2}$/);
      } else {
        expect(
          r.reviewer,
          `${locale}: unreviewed names no reviewer`,
        ).toBeNull();
        expect(r.date, `${locale}: unreviewed has no date`).toBeNull();
      }
    }
  });

  it("records Welsh as machine-written and not reviewed", () => {
    expect(REVIEW["cy-001"]).toEqual({
      written: "machine",
      reviewed: false,
      reviewer: null,
      date: null,
    });
  });
});

describe("choosing a table", () => {
  it("gives Welsh its own table and every other locale English", () => {
    for (const locale of LOCALES) {
      expect(strings(locale)).toBe(locale === "cy-001" ? CY : ENGLISH);
    }
    expect(strings(null)).toBe(ENGLISH);
  });

  it("answers TEXT in the current locale, English until one is set", () => {
    expect(TEXT.navHome).toBe(STRINGS.navHome);
    let locale: Locale = "cy-001";
    setLocaleSource(() => locale);
    expect(TEXT.navHome).toBe(CY.navHome);
    expect(TEXT.accessLogPurposes.treatment).toBe(
      CY.accessLogPurposes.treatment,
    );
    locale = "de-001";
    expect(TEXT.navHome).toBe(STRINGS.navHome);
    expect(Object.keys(TEXT).sort()).toEqual(Object.keys(STRINGS).sort());
  });

  it("gives the /my-record filter form the same Welsh words as the full table", () => {
    const form = logFormStrings("cy-001");
    expect(form.filterTitle).toBe(CY.accessLogFilterTitle);
    expect(form.purposes).toBe(CY.accessLogPurposes);
  });
});

describe("a component in each locale", () => {
  const stored = {
    id: "p1",
    version: 1,
    author: "a",
    site_time_zone: "UTC",
    patient: {
      names: [{ family: "Rivers", given: ["Morgan"] }],
      identifiers: [],
    },
  } as unknown as Stored;

  it("renders Welsh at cy-001", () => {
    setLocaleSource(() => "cy-001");
    const { container } = render(PatientBanner, {
      stored,
      today: new Date("2026-11-05T12:00:00Z"),
    });
    expect(screen.getByRole("region", { name: CY.bannerLabel })).toBeTruthy();
    expect(container.textContent).toContain(CY.bannerNoBirthDate);
    expect(container.textContent).not.toContain(STRINGS.bannerNoBirthDate);
  });

  it("renders English at en-001", () => {
    setLocaleSource(() => "en-001");
    const { container } = render(PatientBanner, {
      stored,
      today: new Date("2026-11-05T12:00:00Z"),
    });
    expect(
      screen.getByRole("region", { name: STRINGS.bannerLabel }),
    ).toBeTruthy();
    expect(container.textContent).toContain(STRINGS.bannerNoBirthDate);
  });
});
