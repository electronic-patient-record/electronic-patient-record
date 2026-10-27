import { describe, expect, it } from "vitest";
import { dueText, stateText, stepBody, stepsFor, type AmendmentView, type State } from "../../src/lib/amendments";

const view = (state: State, over: Partial<AmendmentView["request"]> = {}, top: Partial<AmendmentView> = {}): AmendmentView => ({
  id: "a1",
  patient_id: "p1",
  version: 1,
  request: {
    target: { kind: "Observation", id: "o1", version: 1 },
    requester: "pat-1",
    text: "my pulse was never that fast",
    received_at: "2026-10-26T13:00:00Z",
    extension: null,
    state,
    ...over,
  },
  notice: null,
  due: "2026-12-25T13:00:00Z",
  overdue: false,
  ...top,
});

describe("which steps a clinician is offered (the service still decides)", () => {
  it("offers a decision and the one extension on an undecided request", () => {
    expect(stepsFor(view({ state: "requested" }))).toEqual(["accept", "deny", "extend"]);
  });
  it("offers the correction after agreement, and no second extension", () => {
    expect(stepsFor(view({ state: "accepted", by: "d", reason: "r" }))).toEqual(["apply", "extend"]);
    expect(stepsFor(view({ state: "accepted", by: "d", reason: "r" }, { extension: "busy" }))).toEqual(["apply"]);
    expect(stepsFor(view({ state: "requested" }, { extension: "busy" }))).toEqual(["accept", "deny"]);
  });
  it("offers nothing on a final request", () => {
    for (const s of [
      { state: "applied", version: 2 },
      { state: "denied", by: "d", ground: "other", reason: "r" },
      { state: "withdrawn" },
    ] as State[]) {
      expect(stepsFor(view(s))).toEqual([]);
    }
  });
});

describe("a step as a request", () => {
  const f = { reason: "  the chart shows 62  ", ground: "" as const, version: "" };
  it("needs a reason, trimmed, and names the version of the request it is made on", () => {
    expect(stepBody("accept", 3, f)).toEqual({
      ok: true,
      body: { expected_version: 3, step: "accept", reason: "the chart shows 62" },
    });
    expect(stepBody("accept", 3, { ...f, reason: "   " })).toEqual({ ok: false, problem: "Give a reason." });
    expect(stepBody("extend", 1, { ...f, reason: "" }).ok).toBe(false);
  });
  it("needs a ground to refuse", () => {
    expect(stepBody("deny", 1, f)).toEqual({ ok: false, problem: "Choose a ground." });
    expect(stepBody("deny", 1, { ...f, ground: "other" })).toEqual({
      ok: true,
      body: { expected_version: 1, step: "deny", ground: "other", reason: "the chart shows 62" },
    });
  });
  it("sends a version only if it is a whole number of 1 or more", () => {
    const g = { reason: "", ground: "" as const };
    expect(stepBody("apply", 2, { ...g, version: "3" })).toEqual({
      ok: true,
      body: { expected_version: 2, step: "apply", version: 3 },
    });
    for (const v of ["", "0", "-1", "2.5", "x"]) expect(stepBody("apply", 2, { ...g, version: v }).ok).toBe(false);
  });
});

describe("what a row says", () => {
  it("says where it stands and, while open, when it is due and whether it is overdue", () => {
    expect(stateText({ state: "applied", version: 4 })).toContain("version 4");
    expect(dueText(view({ state: "requested" }), "UTC")).toContain("A decision is due");
    expect(dueText(view({ state: "requested" }, {}, { overdue: true }), "UTC")).toContain("This is overdue.");
    expect(dueText(view({ state: "withdrawn" }, {}, { overdue: true }), "UTC")).toBe("");
  });
  it("still says something for a state it does not know", () => {
    expect(stateText({ state: "new" } as unknown as State)).toContain("cannot show");
  });
});
