// Reasonable adjustments and communication needs (PA-6, H-048), as staff see and act on them. Pure, apart
// from the loader. The service holds the rules; this offers the catalogue in the reader's words and sends
// nothing without what the service requires. Who is acting is the token: nothing here names a person.
// A need informs staff. Nothing here changes what a person is offered.
import { siteWhenText } from "./access-log.js";
import { call } from "./api.js";
import { TEXT } from "./i18n.js";

/** One need whose check is overdue (`GET /api/overdue-reviews`): a pointer, with no code, note or reason. */
export interface OverdueReview {
  adjustment_id: string;
  patient_id: string;
  version: number;
  review_by: string;
  contact_held: boolean;
}

/** The catalogue, by category. `tests/unit/adjustments.test.ts` fails if it differs from the service's. */
export const CATALOGUE = {
  interpreting: [
    "interpreter-spoken",
    "interpreter-bsl",
    "interpreter-deafblind",
  ],
  communication: [
    "easy-read",
    "large-print",
    "braille",
    "audio",
    "hearing-loop",
    "pictorial-pain-scale",
    "text-not-calls",
  ],
  "physical-access": [
    "step-free",
    "wheelchair-space",
    "hoist",
    "accessible-toilet",
    "assistance-dog",
  ],
  environment: ["quiet-space", "low-lighting", "no-waiting-room"],
  understanding: ["extra-time", "carer-present"],
  appointments: ["longer-appointment", "first-or-last-slot", "same-clinician"],
} as const;

export type Category = keyof typeof CATALOGUE;
export type Code = (typeof CATALOGUE)[Category][number];
export const CATEGORIES = Object.keys(CATALOGUE) as Category[];

export interface StoredAdjustment {
  id: string;
  patient_id: string;
  version: number;
  author: string;
  recorded_at: string;
  adjustment: {
    code: string;
    note: string | null;
    status: "active" | "ended";
    review_by: string | null;
    reason: string;
  };
  category: string;
  /** The service's English words, for a code this page does not know. */
  display: string;
  overdue: boolean;
}

export interface AdjustmentLoad {
  status: number;
  adjustments: StoredAdjustment[] | null;
}

/** Every adjustment. Any status but 200 is a look that failed, never "none recorded". */
export async function loadAdjustments(
  patientId: string,
  options: { background?: boolean; purpose?: string } = {},
): Promise<AdjustmentLoad> {
  const res = await call<{ adjustments: StoredAdjustment[] }>(
    "GET",
    `/api/patients/${patientId}/adjustments`,
    undefined,
    options.purpose,
    options,
  );
  return {
    status: res.status,
    adjustments: res.status === 200 ? res.body.adjustments : null,
  };
}

/** The need in the reader's words, with its note. A code this page does not know keeps the service's words. */
export function adjustmentName(a: StoredAdjustment): string {
  const words =
    (TEXT.adjustmentCodes as Record<string, string>)[a.adjustment.code] ??
    a.display;
  return a.adjustment.note ? `${words} (${a.adjustment.note})` : words;
}

export const active = (rows: StoredAdjustment[]) =>
  rows.filter((a) => a.adjustment.status === "active");

export type BannerAdjustments =
  | { kind: "none" }
  | { kind: "status"; text: string }
  | { kind: "needs"; text: string; overdue: string | null };

/**
 * The banner's line. Nothing while loading, or when the page did not ask (a patient's own view). A failed
 * look says so, and an empty list says "none recorded": silence on either would read as "no needs".
 */
export function bannerAdjustments(
  load: AdjustmentLoad | null,
): BannerAdjustments {
  if (!load) return { kind: "none" };
  if (load.status === 403)
    return { kind: "status", text: TEXT.bannerNeedsNotShown };
  if (!load.adjustments)
    return { kind: "status", text: TEXT.bannerNeedsFailed };
  const standing = active(load.adjustments);
  if (standing.length === 0)
    return { kind: "status", text: TEXT.bannerNeedsNone };
  const late = standing.filter((a) => a.overdue).length;
  return {
    kind: "needs",
    text: TEXT.bannerNeeds.replace(
      "{needs}",
      standing.map(adjustmentName).join("; "),
    ),
    overdue:
      late === 0 ? null : TEXT.bannerNeedsOverdue.replace("{n}", String(late)),
  };
}

export interface AdjustmentForm {
  code: Code | "";
  note: string;
  reviewBy: string;
  reason: string;
}

export const emptyAdjustment = (): AdjustmentForm => ({
  code: "",
  note: "",
  reviewBy: "",
  reason: "",
});

type Built =
  { ok: true; body: Record<string, unknown> } | { ok: false; problem: string };

/** The body of a new adjustment, or the sentence saying what to fix. */
export function recordBody(f: AdjustmentForm): Built {
  if (!f.code) return { ok: false, problem: TEXT.adjustmentCodeNeeded };
  const note = f.note.trim();
  if (f.code === "interpreter-spoken" && !note)
    return { ok: false, problem: TEXT.adjustmentLanguageNeeded };
  const reason = f.reason.trim();
  if (!reason) return { ok: false, problem: TEXT.adjustmentReasonNeeded };
  return {
    ok: true,
    body: {
      code: f.code,
      note: note || null,
      review_by: f.reviewBy.trim() || null,
      reason,
    },
  };
}

/** The body of a review or an ending, or the sentence saying what to fix. */
export function changeBody(
  action: "review" | "end",
  expectedVersion: number,
  reviewBy: string,
  reason: string,
): Built {
  const why = reason.trim();
  if (!why) return { ok: false, problem: TEXT.adjustmentReasonNeeded };
  return {
    ok: true,
    body: {
      expected_version: expectedVersion,
      action,
      review_by: action === "review" ? reviewBy.trim() || null : null,
      reason: why,
    },
  };
}

/** One adjustment's detail: when it was written, and its review date or that it has none. */
export function adjustmentDetail(a: StoredAdjustment, zone?: string): string {
  const when = siteWhenText(a.recorded_at, zone);
  if (a.adjustment.status === "ended")
    return TEXT.adjustmentEndedOn.replace("{when}", when);
  const review = a.adjustment.review_by
    ? (a.overdue
        ? TEXT.adjustmentReviewOverdue
        : TEXT.adjustmentReviewBy
      ).replace("{date}", a.adjustment.review_by)
    : TEXT.adjustmentNoReview;
  return `${TEXT.adjustmentRecorded.replace("{when}", when)} ${review}`;
}
