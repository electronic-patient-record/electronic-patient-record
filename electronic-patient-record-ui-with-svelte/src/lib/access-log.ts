// The patient-visible access log (SE-6): the API's entries, turned into a sentence.
import { STRINGS } from "./strings.js";

export type LogEntry = {
  id: number;
  at: string;
  by: "you" | "staff";
  roles?: string[];
  organisations?: string[];
  did: string;
  what: string;
  refused: boolean;
  purpose: string | null;
  break_glass: boolean;
};

export type LogPage = { entries: LogEntry[]; next: number | null };

const DID = STRINGS.accessLogDid as Record<string, string>;
const WHAT = STRINGS.accessLogWhat as Record<string, string>;

/** "2026-10-21T09:00:00.000000Z" as "2026-10-21 09:00 UTC"; anything else is shown as given. */
export function whenText(at: string): string {
  const m = /^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2})/.exec(at);
  return m ? `${m[1]} ${m[2]} UTC` : at;
}

const ROLE = STRINGS.roleNames as Record<string, string>;

/** "A member of staff (clinician, riverside)": the role and organisation the token carried, if recorded. */
function staff(e: LogEntry): string {
  // A proxy is not staff: the patient must be able to tell who is reading their record for them (PA-5).
  if ((e.roles ?? []).includes("proxy")) return STRINGS.accessLogProxy;
  const bits = [
    ...(e.roles ?? []).map((r) => ROLE[r] ?? r),
    ...(e.organisations ?? []),
  ];
  return bits.length > 0
    ? `${STRINGS.accessLogStaff} (${bits.join(", ")})`
    : STRINGS.accessLogStaff;
}

/** One entry as a sentence. Unknown words from a newer API are shown, not hidden. */
export function describe(e: LogEntry): string {
  const who = e.by === "you" ? STRINGS.accessLogYou : staff(e);
  const did = DID[e.did] ?? e.did;
  const what = WHAT[e.what] ?? e.what;
  const parts = [`${who} ${did} ${STRINGS.accessLogYour} ${what}.`];
  if (e.purpose) parts.push(`${STRINGS.accessLogPurpose}: ${e.purpose}.`);
  if (e.break_glass) parts.push(STRINGS.accessLogBreakGlass);
  return parts.join(" ");
}
