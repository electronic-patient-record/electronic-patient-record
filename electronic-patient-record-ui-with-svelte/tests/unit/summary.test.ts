import { describe, expect, it } from "vitest";
import { latestReadings } from "../../src/lib/summary";
import type { StoredObservation } from "../../src/lib/vitals";

const o = (over: Partial<StoredObservation>): StoredObservation => ({
  id: "o1",
  patient_id: "p1",
  version: 1,
  author: "n",
  recorded_at: "2026-10-26T14:00:00Z",
  site_zone: "UTC",
  code: "8867-4",
  value: 72,
  unit: "/min",
  effective_at: "2026-10-26T13:00:00Z",
  status: "final",
  flag: null,
  ...over,
});

describe("latestReadings", () => {
  it("takes the newest standing reading of each vital", () => {
    const got = latestReadings([
      o({ id: "a", effective_at: "2026-10-25T13:00:00Z" }),
      o({ id: "b", effective_at: "2026-10-26T13:00:00Z" }),
      o({ id: "c", code: "8310-5", unit: "Cel", value: 36 }),
    ]);
    expect(got.map((r) => r.id).sort()).toEqual(["b", "c"]);
  });

  it("never offers an entry in error, even if it is the newest", () => {
    const got = latestReadings([
      o({ id: "a", effective_at: "2026-10-25T13:00:00Z" }),
      o({
        id: "x",
        status: "entered-in-error",
        effective_at: "2026-10-27T13:00:00Z",
      }),
    ]);
    expect(got.map((r) => r.id)).toEqual(["a"]);
  });

  it("keeps a newest reading with no value rather than showing an older value as current", () => {
    const got = latestReadings([
      o({ id: "a", effective_at: "2026-10-25T13:00:00Z" }),
      o({
        id: "n",
        value: null,
        unit: null,
        absent_reason: "failed",
        effective_at: "2026-10-26T13:00:00Z",
      }),
    ]);
    expect(got.map((r) => r.id)).toEqual(["n"]);
  });

  it("is empty with no readings", () => {
    expect(latestReadings([])).toEqual([]);
  });
});
