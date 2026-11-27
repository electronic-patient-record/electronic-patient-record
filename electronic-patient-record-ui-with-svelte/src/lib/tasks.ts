// Tasks that rules made for a team (TL-3): the shapes `GET /api/tasks` answers, and the few rules the page applies.
// A task is a pointer: the title is the rule's own words, and the record opens through the patient's page.

import { calendarDay } from "./patient.js";

/** The record a task came from. */
export interface TaskSource {
  kind: "result" | "observation" | "referral" | "treatment-refusal";
  id: string;
}

/** One open task on a team's list (`GET /api/tasks?team=`). */
export interface TaskItem {
  id: string;
  version: number;
  patient_id: string;
  rule: string;
  rule_version: number;
  team: string;
  status: "open" | "done" | "cancelled";
  title: string;
  /** YYYY-MM-DD, the site's day. */
  due_on: string;
  reminders: string[];
  source: TaskSource;
  /** YYYY-MM-DD, the latest reminder sent, or null. */
  reminded_on: string | null;
}

/** The list's answer. */
export interface TaskList {
  team: string;
  items: TaskItem[];
  limit: number;
  more: boolean;
  site_time_zone: string;
}

/**
 * The teams a task can be for: the staff roles. The server decides whose list it is (only a caller who holds the
 * role sees it), so this is a choice of which to ask for, not a grant.
 */
export const TEAMS = [
  "clinician",
  "nurse",
  "pharmacist",
  "allied-health",
  "receptionist",
  "clinical-admin",
  "on-duty-clinician",
] as const;

/** Where on the patient's page a task's source is shown. A result has no section of its own yet. */
export function sourceAnchor(kind: TaskSource["kind"]): string {
  switch (kind) {
    case "observation":
      return "#vitals-h";
    case "referral":
      return "#referrals-h";
    case "treatment-refusal":
      return "#refusals-h";
    default:
      return "";
  }
}

/** Whether a task's due day has passed, compared as site days (`YYYY-MM-DD` sorts as a date). */
export function overdue(dueOn: string, today: string): boolean {
  return dueOn < today;
}

/** A close's reason is 1 to 500 characters once trimmed, as the server checks. */
export function reasonProblem(reason: string): "required" | "tooLong" | null {
  const r = reason.trim();
  if (r.length === 0) return "required";
  if ([...r].length > 500) return "tooLong";
  return null;
}

/** Today at the site, `YYYY-MM-DD`, in `zone` (an IANA name; an unknown one is UTC). */
export function siteToday(now: Date, zone: string): string {
  const d = calendarDay(now, zone);
  const two = (n: number) => String(n).padStart(2, "0");
  return `${d.year}-${two(d.month)}-${two(d.day)}`;
}
