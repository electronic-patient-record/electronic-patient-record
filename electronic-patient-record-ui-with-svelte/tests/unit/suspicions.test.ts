import { describe, expect, it } from "vitest";
import { outcomeText, targetMark, type Due } from "../../src/lib/suspicions";
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
