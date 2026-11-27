// Refusals of treatment on the banner (RF-7, first slice): what the record says is refused, in its own words, found in
// seconds. "None recorded" is what this record knows and never "none", and a list that could not be read is louder than
// an empty one: treat it as not known. A role that may not read refusals (403) is shown no line at all.
// The wording is the author's and is UNREVIEWED: no clinician has timed or read it (RF-7, RF-1).
import { call } from "./api.js";
import { TEXT } from "./i18n.js";

/** One refusal as the API lists it: only what the banner needs. */
export interface ListedRefusal {
  id: string;
  from_merged_record: boolean;
  refusal: {
    treatment: string;
    state: "stands" | "doubted" | "withdrawn" | "entered-in-error";
    identity: "certain" | "uncertain";
    // What the emergency sheet shows; the banner needs none of it.
    kind?: Kind;
    accepts?: string;
    circumstances?: string;
    if_life_at_risk?: "yes" | "no" | "not-stated";
    form?: {
      written: Known;
      signed: Known;
      witnessed: Known;
      witnesses?: number;
      seen: "original" | "copy" | "not-seen";
      kept?: string;
    };
    made_on?: string;
    confirmed_on?: string;
    said_by?: SaidBy;
    power_seen?: Known;
    doubt?: string;
    withdrawal?: {
      how: "spoken" | "written" | "conduct";
      noticed_by: string;
      at: string;
    };
  };
}

export type Known = "yes" | "no" | "not-known";
export type Kind =
  | "spoken-now"
  | "advance"
  | "order"
  | "representative"
  | "parent"
  | "carried"
  | "reported";
export type SaidBy =
  | "patient"
  | "attorney"
  | "guardian"
  | "parent"
  | "relative"
  | "clinician"
  | "other";

export interface RefusalData {
  refusals: ListedRefusal[];
  summary: { state: "recorded" | "all-withdrawn" | "none-recorded" };
}

export interface RefusalLoad {
  status: number;
  data: RefusalData | null;
}

export async function loadRefusals(
  patientId: string,
  purpose?: string,
): Promise<RefusalLoad> {
  const res = await call<RefusalData>(
    "GET",
    `/api/patients/${patientId}/treatment-refusals`,
    undefined,
    purpose,
  );
  return { status: res.status, data: res.status === 200 ? res.body : null };
}

/** The banner's line: `show` false for a role that may not read refusals; `loud` for one recorded, or one unreadable. */
export function bannerRefusals(load: RefusalLoad | null): {
  show: boolean;
  text: string;
  loud: boolean;
} {
  if (load === null)
    return { show: true, text: TEXT.refusalChecking, loud: false };
  if (load.status === 403) return { show: false, text: "", loud: false };
  if (load.status !== 200 || !load.data) {
    return { show: true, text: TEXT.refusalLoadFailed, loud: true };
  }
  const { summary, refusals } = load.data;
  if (summary.state === "none-recorded") {
    return { show: true, text: TEXT.refusalNoneRecorded, loud: false };
  }
  if (summary.state === "all-withdrawn") {
    return { show: true, text: TEXT.refusalAllWithdrawn, loud: false };
  }
  // Those that stand first, then those in doubt; each in the record's own words, with what is uncertain about it.
  const order = { stands: 0, doubted: 1 } as const;
  const shown = refusals
    .filter(
      (r) => r.refusal.state === "stands" || r.refusal.state === "doubted",
    )
    .sort(
      (a, b) =>
        order[a.refusal.state as "stands" | "doubted"] -
        order[b.refusal.state as "stands" | "doubted"],
    )
    .map((r) => {
      const marks = [
        r.refusal.state === "doubted" ? TEXT.refusalDoubted : null,
        r.refusal.identity === "uncertain"
          ? TEXT.refusalIdentityUncertain
          : null,
        r.from_merged_record ? TEXT.refusalFromMerged : null,
      ].filter((m): m is string => m !== null);
      return marks.length > 0
        ? `${r.refusal.treatment} (${marks.join(", ")})`
        : r.refusal.treatment;
    });
  return { show: true, text: shown.join("; "), loud: true };
}
