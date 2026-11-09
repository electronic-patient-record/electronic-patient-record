// The UI's words in the reader's locale (WL-8).
//
// `STRINGS` (`strings.ts`) is the English source and its key set is the contract. A locale with a
// table of its own (`TABLES`) must give every key: the table is typed `Strings`, so a missing or
// extra key is a type error, and `tests/unit/i18n.test.ts` checks that no value is empty and that
// every `{placeholder}` is kept. A locale with no table reads English for every key.
//
// Pages and `src/lib` read `TEXT`, which answers in the current locale. Where the current locale
// comes from is set once per side, because one module serves both:
//   - the server (`hooks.server.ts`): the request's locale, kept per request in an
//     AsyncLocalStorage, so two requests rendered at once never see each other's locale;
//   - the browser (`routes/+layout.svelte`): `page.data.locale`, which is reactive, so a template
//     that reads `TEXT` updates when the locale changes.
// Until either is set (unit tests, module start-up) the locale is English.
//
// No translation here is reviewed: see `REVIEW` and `governance/translation-review.md`.
import { STRINGS } from "./strings.js";
import { CY } from "./strings.cy.js";
import { DEFAULT_LOCALE, type Locale } from "./locales.js";

/** The shape of `STRINGS` with every literal widened to `string`: what a locale's table must fill. */
export type Widen<T> = {
  readonly [K in keyof T]: T[K] extends string ? string : Widen<T[K]>;
};
export type Strings = Widen<typeof STRINGS>;

/** English, typed as a table like any other. */
export const ENGLISH: Strings = STRINGS;

/** The locales with a table of their own. Every other locale reads {@link ENGLISH}. */
export const TABLES = {
  "cy-001": CY,
} as const satisfies Partial<Record<Locale, Strings>>;

const BY_LOCALE: Partial<Record<Locale, Strings>> = TABLES;

/** Who has reviewed each locale's table. `reviewed` may be true only with a named reviewer and a date. */
export interface Review {
  /** How the table was written. */
  written: "source" | "machine";
  /** Whether a speaker of the language has read every string. */
  reviewed: boolean;
  /** The reviewer's name, or null while nobody has. */
  reviewer: string | null;
  /** The day of the review (YYYY-MM-DD), or null. */
  date: string | null;
}

/** The review status of each table. Nothing may claim review without a name (AGENTS.md). */
export const REVIEW: Record<"en-001" | keyof typeof TABLES, Review> = {
  "en-001": { written: "source", reviewed: false, reviewer: null, date: null },
  // Machine-written on 2026-11-09 for WL-8. Not read by a Welsh speaker, nor by a clinician who
  // speaks Welsh. It must not be called reviewed until a named reviewer has read it.
  "cy-001": { written: "machine", reviewed: false, reviewer: null, date: null },
};

/** The table for `locale`: its own, or English. */
export function strings(locale: Locale | null | undefined): Strings {
  return (locale && BY_LOCALE[locale]) || ENGLISH;
}

let source: () => Locale = () => DEFAULT_LOCALE;

/** Say where the current locale comes from (once on the server, once in the browser). */
export function setLocaleSource(read: () => Locale | null | undefined): void {
  source = () => read() ?? DEFAULT_LOCALE;
}

/** The current locale, as the source set by {@link setLocaleSource} reports it. */
export function currentLocale(): Locale {
  return source();
}

/**
 * The words in the current locale. Each top-level key is a getter, so a read is decided when it
 * happens, not when a module loads: never copy a value from `TEXT` into a module-level constant.
 */
export const TEXT: Strings = Object.defineProperties(
  {},
  Object.fromEntries(
    Object.keys(STRINGS).map((key) => [
      key,
      {
        enumerable: true,
        get: () => strings(source())[key as keyof Strings],
      },
    ]),
  ),
) as Strings;
