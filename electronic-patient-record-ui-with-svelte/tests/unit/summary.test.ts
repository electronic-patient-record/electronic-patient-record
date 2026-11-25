import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { STRINGS } from "../../src/lib/strings";
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

// AG-5: the summary said results and problems were "not built" and that "nothing is held for these", when the service
// stored both. The two lists are held to the service's own route table, so that a section cannot go stale in silence.
describe("what the summary says it does not show (AG-5)", () => {
  const routes = readFileSync(
    "../electronic-patient-record-service-with-rust/src/routes.rs",
    "utf8",
  );
  const route = (name: string) => `"/api/patients/{id}/${name}"`;
  const HELD = {
    problems: "problems",
    results: "results",
    relatives: "relative-history",
  } as const;
  const NOT_KEPT = {
    medications: ["medications", "prescriptions", "medication-requests"],
    immunisations: ["immunisations", "immunizations", "vaccinations"],
    appointments: ["appointments"],
  } as const;
  it("names as held only what a route of the service holds, and all it lists", () => {
    expect(Object.keys(STRINGS.summaryElsewhereItems).sort()).toEqual(
      Object.keys(HELD).sort(),
    );
    for (const path of Object.values(HELD)) {
      expect(routes, path).toContain(route(path));
    }
  });
  it("names as not kept only what no route of the service holds", () => {
    expect(Object.keys(STRINGS.summaryNotKeptItems).sort()).toEqual(
      Object.keys(NOT_KEPT).sort(),
    );
    for (const path of Object.values(NOT_KEPT).flat()) {
      expect(routes, path).not.toContain(route(path));
    }
  });
  it("never says that nothing is held", () => {
    for (const words of [
      STRINGS.summaryElsewhereNote,
      STRINGS.summaryNotKeptNote,
    ]) {
      expect(words.toLowerCase()).not.toContain("nothing is held");
    }
    expect(STRINGS.summaryElsewhereNote).toContain("can hold");
  });
});
