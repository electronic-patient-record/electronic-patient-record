// Watch lists (WL-6), as a clinician sees them on a patient's page. The service decides the state
// (`active`, `ended`, `lapsed` once its end day has passed) and who may end an entry (its owner only);
// this file holds no copy of either. An entry always has a reason.
import { call } from "./api.js";

export type WatchState = "active" | "ended" | "lapsed";

export interface WatchEntry {
  id: string;
  version: number;
  owner: string;
  patient_id: string;
  since: string;
  recorded_at: string;
  state: WatchState;
  reason: string;
  until: string | null;
  end_reason: string | null;
}

export interface WatchLoad {
  status: number;
  entries: WatchEntry[] | null;
}

export async function loadWatchList(patientId: string): Promise<WatchLoad> {
  const res = await call<{ entries: WatchEntry[] }>(
    "GET",
    `/api/patients/${patientId}/watch-list`,
  );
  return {
    status: res.status,
    entries: res.status === 200 ? res.body.entries : null,
  };
}

/** The body for adding to one's own list, or what is missing; an end day is optional. */
export function buildWatchEntry(
  reason: string,
  until: string,
): { body: unknown; problem: "reason" | null } {
  if (!reason.trim()) return { body: null, problem: "reason" };
  return {
    body: {
      entry: { reason: reason.trim(), ...(until ? { until } : {}) },
    },
    problem: null,
  };
}
