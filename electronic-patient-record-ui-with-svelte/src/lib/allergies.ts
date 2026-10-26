// Allergies (RC-5, in part) as the pages see them, and the one line the patient banner shows.
//
// The line must never read as "none" unless a clinician said so: every state where the system does
// not KNOW says so in words ("not recorded", "not shown to your role", "could not be loaded"), and
// each of them adds that this does not mean there are none.
import { siteWhenText } from "./access-log.js";
import { call } from "./api.js";
import { STRINGS } from "./strings.js";

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

const RISK: Record<string, string> = {
  high: STRINGS.allergyRiskHigh,
  low: STRINGS.allergyRiskLow,
  "unable-to-assess": STRINGS.allergyRiskUnassessed,
};


/** The banner's allergy line, and whether it carries a high-risk allergy (shown more strongly). */
export function bannerAllergies(load: AllergyLoad | null, siteZone?: string): {
  text: string;
  high: boolean;
} {
  if (load === null) return { text: STRINGS.allergyChecking, high: false };
  if (load.status === 403) {
    return { text: STRINGS.allergyNotShown, high: false };
  }
  if (load.status !== 200 || !load.data) {
    return { text: STRINGS.allergyLoadFailed, high: false };
  }
  const s = load.data.summary;
  switch (s.state) {
    case "active":
      return {
        text: s.items
          .map((i) => {
            const risk = i.criticality ? RISK[i.criticality] : undefined;
            return risk ? `${i.substance} (${risk})` : i.substance;
          })
          .join(", "),
        high: s.items.some((i) => i.criticality === "high"),
      };
    case "no-known":
      return {
        text: STRINGS.allergyNoKnown.replace("{when}", siteWhenText(s.asserted_at, siteZone)),
        high: false,
      };
    case "none-active":
      return {
        text: STRINGS.allergyNoneActive.replace("{n}", String(s.inactive)),
        high: false,
      };
    default:
      return { text: STRINGS.bannerAllergiesUnknown, high: false };
  }
}
