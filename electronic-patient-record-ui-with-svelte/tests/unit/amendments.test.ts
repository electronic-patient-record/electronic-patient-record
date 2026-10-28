import { describe, expect, it } from "vitest";
import {
  aboutText,
  askBody,
  disagreeBody,
  dueText,
  openCounts,
  ownStepsFor,
  stateText,
  targetLines,
  stepBody,
  stepsFor,
  type AmendmentView,
  type State,
} from "../../src/lib/amendments";

const view = (
  state: State,
  over: Partial<AmendmentView["request"]> = {},
  top: Partial<AmendmentView> = {},
): AmendmentView => ({
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
    expect(stepsFor(view({ state: "requested" }))).toEqual([
      "accept",
      "deny",
      "extend",
    ]);
  });
  it("offers the correction after agreement, and no second extension", () => {
    expect(stepsFor(view({ state: "accepted", by: "d", reason: "r" }))).toEqual(
      ["apply", "extend"],
    );
    expect(
      stepsFor(
        view(
          { state: "accepted", by: "d", reason: "r" },
          { extension: "busy" },
        ),
      ),
    ).toEqual(["apply"]);
    expect(
      stepsFor(view({ state: "requested" }, { extension: "busy" })),
    ).toEqual(["accept", "deny"]);
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
  const f = {
    reason: "  the chart shows 62  ",
    ground: "" as const,
    version: "",
  };
  it("needs a reason, trimmed, and names the version of the request it is made on", () => {
    expect(stepBody("accept", 3, f)).toEqual({
      ok: true,
      body: {
        expected_version: 3,
        step: "accept",
        reason: "the chart shows 62",
      },
    });
    expect(stepBody("accept", 3, { ...f, reason: "   " })).toEqual({
      ok: false,
      problem: "Give a reason.",
    });
    expect(stepBody("extend", 1, { ...f, reason: "" }).ok).toBe(false);
  });
  it("needs a ground to refuse", () => {
    expect(stepBody("deny", 1, f)).toEqual({
      ok: false,
      problem: "Choose a ground.",
    });
    expect(stepBody("deny", 1, { ...f, ground: "other" })).toEqual({
      ok: true,
      body: {
        expected_version: 1,
        step: "deny",
        ground: "other",
        reason: "the chart shows 62",
      },
    });
  });
  it("sends a version only if it is a whole number of 1 or more", () => {
    const g = { reason: "", ground: "" as const };
    expect(stepBody("apply", 2, { ...g, version: "3" })).toEqual({
      ok: true,
      body: { expected_version: 2, step: "apply", version: 3 },
    });
    for (const v of ["", "0", "-1", "2.5", "x"])
      expect(stepBody("apply", 2, { ...g, version: v }).ok).toBe(false);
  });
});

describe("what a row says", () => {
  it("says where it stands and, while open, when it is due and whether it is overdue", () => {
    expect(stateText({ state: "applied", version: 4 })).toContain("version 4");
    expect(dueText(view({ state: "requested" }), "UTC")).toContain(
      "A decision is due",
    );
    expect(
      dueText(view({ state: "requested" }, {}, { overdue: true }), "UTC"),
    ).toContain("This is overdue.");
    expect(
      dueText(view({ state: "withdrawn" }, {}, { overdue: true }), "UTC"),
    ).toBe("");
  });
  it("still says something for a state it does not know", () => {
    expect(stateText({ state: "new" } as unknown as State)).toContain(
      "cannot show",
    );
  });
});

describe("the patient's side", () => {
  it("offers withdrawal only before a decision, and a statement only once, after a refusal", () => {
    expect(ownStepsFor(view({ state: "requested" }))).toEqual(["withdraw"]);
    expect(
      ownStepsFor(view({ state: "accepted", by: "d", reason: "r" })),
    ).toEqual([]);
    const denied = {
      state: "denied",
      by: "d",
      ground: "other",
      reason: "r",
    } as const;
    expect(ownStepsFor(view(denied))).toEqual(["disagree"]);
    expect(
      ownStepsFor(view({ ...denied, disagreement: "I disagree" })),
    ).toEqual([]);
    expect(ownStepsFor(view({ state: "withdrawn" }))).toEqual([]);
  });
  it("asks only about a chosen reading, in words, naming the version the patient read", () => {
    expect(askBody(null, "x")).toEqual({
      ok: false,
      problem: "Choose a record.",
    });
    expect(
      askBody({ kind: "Observation", id: "o1", version: 2 }, "  "),
    ).toEqual({ ok: false, problem: "Say what should change." });
    expect(
      askBody({ kind: "Observation", id: "o1", version: 2 }, " it was 62 "),
    ).toEqual({
      ok: true,
      body: {
        kind: "Observation",
        target_id: "o1",
        version: 2,
        text: "it was 62",
      },
    });
  });
  it("keeps a statement exactly as written, and sends none that is blank", () => {
    expect(disagreeBody(2, "  ")).toEqual({
      ok: false,
      problem: "Write your statement.",
    });
    expect(disagreeBody(2, "  I disagree.\n")).toEqual({
      ok: true,
      body: {
        expected_version: 2,
        step: "disagree",
        statement: "  I disagree.\n",
      },
    });
  });
});

describe("naming what a request is about, and counting what waits", () => {
  const obs = {
    id: "o1",
    patient_id: "p1",
    version: 1,
    author: "n",
    recorded_at: "2026-10-26T10:00:00Z",
    site_zone: "UTC",
    code: "8867-4",
    value: 72,
    unit: "/min",
    effective_at: "2026-10-26T13:30:00Z",
    status: "final",
    flag: null,
  } as const;
  const allergy = {
    id: "g1",
    patient_id: "p1",
    version: 2,
    author: "n",
    recorded_at: "2026-10-26T10:00:00Z",
    allergy: { substance: "penicillin", criticality: "high", status: "active" },
  } as const;
  it("names a reading or an allergy, and falls back to the kind when the record is not loaded", () => {
    const lines = targetLines([obs], [allergy], "UTC");
    const on = (kind: string, id: string) =>
      view({ state: "requested" }, { target: { kind, id, version: 1 } });
    expect(aboutText(on("Observation", "o1"), lines)).toContain("Heart rate");
    expect(aboutText(on("AllergyIntolerance", "g1"), lines)).toContain(
      "penicillin",
    );
    expect(aboutText(on("Observation", "gone"), lines)).toBe("Observation");
    // The same id under the other kind is not the same record.
    expect(aboutText(on("AllergyIntolerance", "o1"), lines)).toBe(
      "AllergyIntolerance",
    );
  });
  it("counts only what is undecided, and the overdue among it", () => {
    const rows = [
      view({ state: "requested" }, {}, { overdue: true }),
      view({ state: "accepted", by: "d", reason: "r" }),
      view({ state: "withdrawn" }, {}, { overdue: true }),
      view({ state: "applied", version: 2 }),
    ];
    expect(openCounts(rows)).toEqual({ open: 2, overdue: 1 });
    expect(openCounts([])).toEqual({ open: 0, overdue: 0 });
  });
});
