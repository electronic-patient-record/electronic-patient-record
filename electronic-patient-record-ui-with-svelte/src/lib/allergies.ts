// Allergies (RC-5, in part) as the pages see them, and the one line the patient banner shows.
//
// The line must never read as "none" unless a clinician said so: every state where the system does
// not KNOW says so in words ("not recorded", "not shown to your role", "could not be loaded"), and
// each of them adds that this does not mean there are none.
import { siteWhenText } from "./access-log.js";
import { call } from "./api.js";
import { TEXT } from "./i18n.js";
import type { Notice } from "./vitals.js";

export type Criticality = "low" | "high" | "unable-to-assess";
export type AllergyStatus = "active" | "inactive" | "entered-in-error";
export type Category = "food" | "medication" | "environment" | "biologic";

export interface AllergyRecord {
  substance: string;
  coding?: { system: string; code: string } | null;
  category?: Category | null;
  criticality?: Criticality | null;
  reaction?: string | null;
  onset?: string | null;
  status: AllergyStatus;
}

export interface StoredAllergy {
  id: string;
  patient_id: string;
  version: number;
  author: string;
  recorded_at: string;
  allergy: AllergyRecord;
}

export type AllergySummary =
  | {
      state: "active";
      items: { substance: string; criticality?: Criticality | null }[];
    }
  | { state: "no-known"; asserted_at: string; asserted_by: string }
  | { state: "none-active"; inactive: number }
  | { state: "not-recorded" };

export interface AllergyData {
  allergies: StoredAllergy[];
  statement: {
    version: number;
    author: string;
    recorded_at: string;
    no_known_allergies: boolean;
  } | null;
  summary: AllergySummary;
  /** What a reader must be told about each allergy with an amendment request (RC-4), by id. */
  amendment_notices?: Record<string, Notice[]>;
}

/** One allergy in a line: the substance, its risk in words, and its status. */
export function allergyLine(a: StoredAllergy): string {
  return [
    a.allergy.substance,
    a.allergy.criticality
      ? `(${TEXT.allergyCriticalities[a.allergy.criticality]})`
      : "",
    `· ${TEXT.allergyStatusLabels[a.allergy.status]}`,
  ]
    .filter(Boolean)
    .join(" ");
}

/** What the API said: the status, and the data when it was 200. */
export interface AllergyLoad {
  status: number;
  data: AllergyData | null;
  /** On a 403: the API says break-the-glass could lift the refusal. */
  offer?: boolean;
}

export async function loadAllergies(
  patientId: string,
  purpose?: string,
  options: { background?: boolean } = {},
): Promise<AllergyLoad> {
  const res = await call<AllergyData>(
    "GET",
    `/api/patients/${patientId}/allergies`,
    undefined,
    purpose,
    options,
  );
  const offer =
    res.status === 403 &&
    (res.body as { break_glass_offer?: boolean } | null)?.break_glass_offer ===
      true;
  return {
    status: res.status,
    data: res.status === 200 ? res.body : null,
    ...(offer ? { offer: true } : {}),
  };
}

// Read at each call, not at module load, so the words follow the reader's locale (`i18n.ts`).
const riskWords = (): Record<string, string> => ({
  high: TEXT.allergyRiskHigh,
  low: TEXT.allergyRiskLow,
  "unable-to-assess": TEXT.allergyRiskUnassessed,
});

/** One active allergy in words, with its risk when one was stated. */
function itemWords(i: {
  substance: string;
  criticality?: string | null;
}): string {
  const risk = i.criticality ? riskWords()[i.criticality] : undefined;
  return risk ? `${i.substance} (${risk})` : i.substance;
}

/** How long a list of allergies on the strip of a small screen may be before low-risk ones are counted. */
export const STRIP_ALLERGIES = 60;

/**
 * The allergy line for the strip that stays in sight on a small screen (XP-3). What may be life-critical stays (the
 * maintainer's answers to OQ-XP2): **every allergy is named that is marked high risk, or whose risk was not stated or not
 * assessed, however many there are**, because an unknown risk is not a low one. Only those recorded as low risk may be
 * counted: of them it gives what fits, and how many more there are as `+n`; never fewer without the number. The words for
 * "not recorded", "not shown to you" and a failed look are the banner's own and are never shortened.
 *
 * The text of a reaction ("anaphylaxis") is not read: one recorded as low risk may be counted whatever it says.
 */
export function stripAllergies(
  load: AllergyLoad | null,
  siteZone?: string,
): { text: string; high: boolean } {
  const line = bannerAllergies(load, siteZone);
  const summary = load?.status === 200 ? load.data?.summary : undefined;
  if (summary?.state !== "active" || line.text.length <= STRIP_ALLERGIES) {
    return line;
  }
  const high = summary.items.filter((i) => i.criticality === "high");
  const low = summary.items.filter((i) => i.criticality === "low");
  // Whatever else it says, or does not say, is an unknown risk.
  const unknown = summary.items.filter(
    (i) => i.criticality !== "high" && i.criticality !== "low",
  );
  const shown = [...high, ...unknown].map((i) => itemWords(i));
  for (const item of low) {
    const words = itemWords(item);
    // One that is too long by itself is still given: a number alone names nothing.
    if (
      shown.length > 0 &&
      [...shown, words].join(", ").length > STRIP_ALLERGIES
    ) {
      break;
    }
    shown.push(words);
  }
  const more = summary.items.length - shown.length;
  const text = shown.join(", ");
  return { text: more > 0 ? `${text}, +${more}` : text, high: line.high };
}

/** The banner's allergy line, and whether it carries a high-risk allergy (shown more strongly). */
export function bannerAllergies(
  load: AllergyLoad | null,
  siteZone?: string,
): {
  text: string;
  high: boolean;
} {
  if (load === null) return { text: TEXT.allergyChecking, high: false };
  if (load.status === 403) {
    return { text: TEXT.allergyNotShown, high: false };
  }
  if (load.status !== 200 || !load.data) {
    return { text: TEXT.allergyLoadFailed, high: false };
  }
  const s = load.data.summary;
  switch (s.state) {
    case "active":
      return {
        text: s.items.map((i) => itemWords(i)).join(", "),
        high: s.items.some((i) => i.criticality === "high"),
      };
    case "no-known":
      return {
        text: TEXT.allergyNoKnown.replace(
          "{when}",
          siteWhenText(s.asserted_at, siteZone),
        ),
        high: false,
      };
    case "none-active":
      return {
        text: TEXT.allergyNoneActive.replace("{n}", String(s.inactive)),
        high: false,
      };
    default:
      return { text: TEXT.bannerAllergiesUnknown, high: false };
  }
}
