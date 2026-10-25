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

export type LogFilter = {
  from: string;
  to: string;
  by: "" | "you" | "staff";
  refused: "" | "true" | "false";
};
export const NO_FILTER: LogFilter = { from: "", to: "", by: "", refused: "" };

/** `YYYY-MM-DD` as a UTC time, or null when it is not a real day. */
function day(v: string): number | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(v);
  if (!m) return null;
  const [y, mo, d] = [Number(m[1]), Number(m[2]), Number(m[3])];
  const t = Date.UTC(y, mo - 1, d);
  const back = new Date(t);
  return back.getUTCFullYear() === y && back.getUTCMonth() === mo - 1 && back.getUTCDate() === d ? t : null;
}

/**
 * The query string for the access log: the filter, and the page to continue from. The reader's last
 * day is included, but the API's `to` is exclusive, so the next day is sent. Null when a date is
 * not a real day (the page then says so and sends nothing).
 */
export function logQuery(f: LogFilter, before?: number): string | null {
  const q = new URLSearchParams();
  if (f.from) {
    if (day(f.from) === null) return null;
    q.set("from", f.from);
  }
  if (f.to) {
    const t = day(f.to);
    if (t === null) return null;
    q.set("to", new Date(t + 86_400_000).toISOString().slice(0, 10));
  }
  if (f.by) q.set("by", f.by);
  if (f.refused) q.set("refused", f.refused);
  if (before !== undefined) q.set("before", String(before));
  const text = q.toString();
  return text ? `?${text}` : "";
}
