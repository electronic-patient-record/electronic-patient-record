import { describe, expect, it } from "vitest";
import {
  EMPTY_FORM,
  EMPTY_OUTCOME,
  buildOutcome,
  buildSuspicion,
  outcomeText,
  targetMark,
  type Due,
  type Pathway,
} from "../../src/lib/suspicions";
import { STRINGS } from "../../src/lib/strings";
import { CY } from "../../src/lib/strings.cy";

const due = (overdue: boolean): Due => ({
  id: "diagnosis",
  text: "Diagnosed",
  days: 28,
  due_on: "2026-12-07",
  overdue,
});

describe("suspicion on a referral", () => {
  it("reads a passed day as overdue while open, and as passed at the outcome once ended", () => {
    expect(targetMark(due(false), false)).toBe("due");
    expect(targetMark(due(true), false)).toBe("overdue");
    expect(targetMark(due(true), true)).toBe("passed-at-outcome");
    expect(targetMark(due(false), true)).toBe("due");
  });

  it("words each outcome from its own fields and invents none", () => {
    const w = STRINGS.suspicionOutcomes;
    expect(
      outcomeText(
        { kind: "confirmed", diagnosis: "Adenocarcinoma", detail: "biopsy" },
        w,
      ),
    ).toBe("Confirmed: Adenocarcinoma");
    expect(
      outcomeText({ kind: "ruled-out", by: "Dr Owen", detail: "echo" }, w),
    ).toBe("Ruled out by Dr Owen");
    expect(
      outcomeText({ kind: "withdrawn", detail: "patient moved away" }, w),
    ).toBe("Withdrawn: patient moved away");
    expect(outcomeText({ kind: "ruled-out", detail: "echo" }, w)).toBe(
      "Ruled out by ",
    );
  });

  it("has every outcome and clock word in Welsh too (machine-written, unreviewed)", () => {
    expect(Object.keys(CY.suspicionOutcomes).sort()).toEqual(
      Object.keys(STRINGS.suspicionOutcomes).sort(),
    );
    expect(Object.keys(CY.suspicionClockFrom).sort()).toEqual(
      Object.keys(STRINGS.suspicionClockFrom).sort(),
    );
  });
});

const WALES: Pathway = {
  id: "suspected-cancer-wales",
  version: 2,
  title: "Suspected cancer (Wales)",
  approved_by: "",
  priority: "urgent",
  needs_agreement: false,
  band: [],
  factor: [],
  subtypes: {
    scheme: "wales-primary-cancer-site",
    title: "Primary cancer site",
    required: true,
    code: [{ code: "03", name: "Lower GI" }],
  },
};
const DEMENTIA: Pathway = {
  ...WALES,
  id: "dementia",
  version: 2,
  needs_agreement: true,
  subtypes: {
    scheme: "generic-dementia-type",
    title: "Type of dementia",
    required: false,
    code: [{ code: "vascular", name: "Vascular dementia" }],
  },
};

describe("raising a suspicion", () => {
  const filled = {
    ...EMPTY_FORM,
    pathway: "suspected-cancer-wales",
    condition: " Suspected lower GI cancer ",
    point: "2026-11-09T09:30",
    source: "clinician" as const,
    by: "Dr Gwen Hughes",
    groundsKind: "criterion" as const,
    groundsDetail: "NG12",
    told: "not-told" as const,
    subtype: "03",
  };

  it("defaults nothing a clinician must decide", () => {
    const { problems } = buildSuspicion({ ...filled, told: "", subtype: "" }, [
      WALES,
    ]);
    expect(problems).toEqual(["told", "subtype"]);
    expect(buildSuspicion(EMPTY_FORM, [WALES]).problems).toContain("pathway");
    expect(
      buildSuspicion({ ...filled, pathway: "dementia", subtype: "" }, [
        WALES,
        DEMENTIA,
      ]).problems,
    ).toEqual(["agreed"]);
    expect(
      buildSuspicion({ ...filled, groundsKind: "result" }, [WALES]).problems,
    ).toEqual(["result"]);
  });

  it("sends the pathway's version, the subtype with its list, and the time as UTC", () => {
    const { body, problems } = buildSuspicion(filled, [WALES]);
    expect(problems).toEqual([]);
    const s = (body as { suspicion: Record<string, unknown> }).suspicion;
    expect(s.pathway_version).toBe(2);
    expect(s.subtype).toEqual({
      scheme: "wales-primary-cancer-site",
      code: "03",
    });
    expect(s.condition).toBe("Suspected lower GI cancer");
    expect(s.patient_told).toBe("not-told");
    expect(String(s.point_of_suspicion)).toMatch(/Z$/);
    expect(s.agreed).toBeUndefined();
    const declined = buildSuspicion(
      { ...filled, pathway: "dementia", subtype: "", agreed: "no" },
      [WALES, DEMENTIA],
    ).body as { suspicion: Record<string, unknown> };
    expect(declined.suspicion.agreed).toBe(false);
    expect(declined.suspicion.subtype).toBeUndefined();
  });
});

describe("recording an outcome", () => {
  it("asks each kind for what it carries and sends only that, against the version seen", () => {
    expect(buildOutcome(EMPTY_OUTCOME, 1).problems).toEqual(["kind", "detail"]);
    expect(
      buildOutcome({ ...EMPTY_OUTCOME, kind: "confirmed", detail: "biopsy" }, 1)
        .problems,
    ).toEqual(["diagnosis"]);
    expect(
      buildOutcome({ ...EMPTY_OUTCOME, kind: "ruled-out", detail: "echo" }, 1)
        .problems,
    ).toEqual(["by"]);
    const { body } = buildOutcome(
      {
        kind: "ruled-out",
        diagnosis: "left over",
        by: " Dr Owen ",
        detail: "normal echo",
      },
      3,
    );
    expect(body).toEqual({
      outcome: { kind: "ruled-out", by: "Dr Owen", detail: "normal echo" },
      expected_version: 3,
    });
  });
});
