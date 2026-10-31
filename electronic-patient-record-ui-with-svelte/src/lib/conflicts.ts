// The reasons a link cannot be automatic, translated (CQ-38). Pure: the page passes its locale.
import type { Locale } from "./locales.js";
import {
  conflictWords,
  translatedPatientProblemWords,
  refusalWords,
  type ConflictWords,
  type PatientProblemWords,
  type RefusalWords,
} from "./translations.js";

/** One reason as the matcher sends it (`ConflictCode`, tagged by `code`). Unknown codes are kept. */
export type ConflictCode =
  | { code: "different_nhs_numbers" }
  | { code: "same_nhs_different_birth_dates" }
  | { code: "same_nhs_different_names" }
  | { code: "different_ihis" }
  | { code: "same_ihi_different_birth_dates" }
  | { code: "same_ihi_different_names" }
  | { code: "names_sound_alike" }
  | { code: "names_swapped" }
  | { code: "extra_given_name" }
  | { code: "field_unavailable"; field: string }
  | {
      code: "field_below_minimum";
      field: string;
      similarity: number;
      minimum: number;
    }
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
      ? new Intl.NumberFormat(locale, {
          minimumFractionDigits: digits,
          maximumFractionDigits: digits,
        }).format(n)
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
    case "different_ihis":
      return w.differentIhis;
    case "same_ihi_different_birth_dates":
      return w.sameIhiDifferentBirthDates;
    case "same_ihi_different_names":
      return w.sameIhiDifferentNames;
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
      return f && s && m
        ? w.fieldBelowMinimum
            .replace("{field}", f)
            .replace("{similarity}", s)
            .replace("{minimum}", m)
        : null;
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
export function conflictList(
  english: string[],
  codes: ConflictCode[] | undefined,
  locale: Locale,
): string[] {
  if (!codes || codes.length !== english.length) return english;
  return codes.map((c, i) => conflictText(c, locale) ?? english[i]!);
}

/**
 * A refused merge's reasons in `locale`: "These do not look like the same person: <reasons>". Null when
 * the API sent no codes, or any one cannot be translated, so the page shows the API's English message
 * whole rather than a partial translation.
 */
export function notSamePersonText(
  codes: ConflictCode[] | undefined,
  locale: Locale,
): string | null {
  if (!codes || codes.length === 0) return null;
  const parts = codes.map((c) => conflictText(c, locale));
  if (parts.some((p) => p === null)) return null;
  return `${conflictWords(locale).notSamePerson} ${parts.join("; ")}`;
}

const REFUSAL: Record<string, keyof RefusalWords> = {
  reason_required: "reasonRequired",
  same_record: "sameRecord",
  drill_and_real: "drillAndReal",
  not_merged: "notMerged",
  source_already_merged: "sourceAlreadyMerged",
  target_already_merged: "targetAlreadyMerged",
};

/**
 * A refused merge or unmerge in `locale`, from the API's `refusal` code (and `refusal_into`, the other
 * record's id). Null for no code, a code this does not know, or a code that needs an id and has none,
 * so the page shows the API's English message.
 */
export function refusalText(
  code: unknown,
  into: unknown,
  locale: Locale,
): string | null {
  const key = typeof code === "string" ? REFUSAL[code] : undefined;
  if (!key) return null;
  const text = refusalWords(locale)[key];
  if (!text.includes("{id}")) return text;
  return typeof into === "string" && into ? text.replace("{id}", into) : null;
}

const PATIENT_PROBLEM: Record<string, keyof PatientProblemWords> = {
  family_name_required: "familyNameRequired",
  birth_date_invalid: "birthDateInvalid",
  nhs_number_placeholder: "nhsNumberPlaceholder",
  nhs_number_check_digit: "nhsNumberCheckDigit",
  identifier_incomplete: "identifierIncomplete",
};

/**
 * The reasons a patient was refused (422 `errors` and `error_codes`, same order), each in `locale`,
 * and the API's English for any code this does not know, or for a locale with no translation (English
 * readers see the API's own words). If the lengths differ (an older API sent no
 * codes) the English list is shown whole, never fewer reasons than the API gave.
 */
export function patientProblemList(
  english: string[],
  codes: unknown,
  locale: Locale,
): string[] {
  if (!Array.isArray(codes) || codes.length !== english.length) return english;
  const w = translatedPatientProblemWords(locale);
  if (!w) return english;
  return codes.map((c, i) => {
    const key = typeof c === "string" ? PATIENT_PROBLEM[c] : undefined;
    return key ? w[key] : english[i]!;
  });
}
