// The patient summary (RC-8, first slice): what is already held, read-only, with the latest reading
// of each vital. It adds no rule of its own: values, flags and notices are the service's words.
import { trends, type StoredObservation } from "./vitals.js";

/**
 * The newest standing reading of each vital, in the order of `trends`. An entry in error is never
 * the latest. A newest reading with no value stays as it is (with its reason): an older reading is
 * not put in its place, because that would show a stale value as current.
 */
export function latestReadings(
  observations: StoredObservation[],
): StoredObservation[] {
  return trends(observations).flatMap((t) => (t.rows[0] ? [t.rows[0]] : []));
}
