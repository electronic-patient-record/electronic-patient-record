// The reasons a link cannot be automatic, translated (CQ-38). Pure: the page passes its locale.
import type { Locale } from "./locales.js";
import { conflictWords, type ConflictWords } from "./translations.js";

/** One reason as the matcher sends it (`ConflictCode`, tagged by `code`). Unknown codes are kept. */
export type ConflictCode =
  | { code: "different_nhs_numbers" }
  | { code: "same_nhs_different_birth_dates" }
  | { code: "same_nhs_different_names" }
  | { code: "names_sound_alike" }
  | { code: "names_swapped" }
  | { code: "extra_given_name" }
  | { code: "field_unavailable"; field: string }
  | { code: "field_below_minimum"; field: string; similarity: number; minimum: number }
  | { code: "evidence_thin"; percent_present: number }
  | { code: string; [more: string]: unknown };

const FIELD: Record<string, keyof ConflictWords> = {
  given_name: "givenName",
  family_name: "familyName",
  birth_date: "birthDate",
};

/** One code as a sentence in `locale`, or null when it is a code, a field or a number this does not know. */
export function conflictText(c: ConflictCode, locale: Locale): string | null {
  const w = conflictWords(locale);
  const num = (n: unknown, digits: number) =>
    typeof n === "number" && Number.isFinite(n)
      ? new Intl.NumberFormat(locale, { minimumFractionDigits: digits, maximumFractionDigits: digits }).format(n)
      : null;
  const field = (f: unknown) => {
    const key = typeof f === "string" ? FIELD[f] : undefined;
    return key ? w[key] : null;
  };
  switch (c.code) {
    case "different_nhs_numbers":
      return w.differentNhsNumbers;
    case "same_nhs_different_birth_dates":
      return w.sameNhsDifferentBirthDates;
    case "same_nhs_different_names":
      return w.sameNhsDifferentNames;
    case "names_sound_alike":
      return w.namesSoundAlike;
    case "names_swapped":
      return w.namesSwapped;
    case "extra_given_name":
      return w.extraGivenName;
    case "field_unavailable": {
      const f = field(c.field);
      return f ? w.fieldUnavailable.replace("{field}", f) : null;
    }
    case "field_below_minimum": {
      const f = field(c.field);
      const s = num(c.similarity, 2);
      const m = num(c.minimum, 2);
      return f && s && m ? w.fieldBelowMinimum.replace("{field}", f).replace("{similarity}", s).replace("{minimum}", m) : null;
    }
    case "evidence_thin": {
      const p = num(c.percent_present, 0);
      return p ? w.evidenceThin.replace("{percent}", p) : null;
    }
    default:
      return null;
  }
}

/**
 * The reasons to show: each code translated, and the English text for any the page cannot translate.
 * The codes and the English list come in the same order and length; if they do not (an older API sent
 * no codes), the English list is shown whole, never fewer reasons than the API gave.
 */
export function conflictList(english: string[], codes: ConflictCode[] | undefined, locale: Locale): string[] {
  if (!codes || codes.length !== english.length) return english;
  return codes.map((c, i) => conflictText(c, locale) ?? english[i]!);
}
