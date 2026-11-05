import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  CATALOGUE,
  CATEGORIES,
  adjustmentDetail,
  adjustmentName,
  bannerAdjustments,
  changeBody,
  emptyAdjustment,
  recordBody,
  type StoredAdjustment,
} from "../../src/lib/adjustments";
import { STRINGS } from "../../src/lib/strings";

const row = (over: Partial<StoredAdjustment["adjustment"]> = {}, top = {}) =>
  ({
    id: "a1",
    patient_id: "p1",
    version: 1,
    author: "nurse-1",
    recorded_at: "2026-11-05T10:00:00Z",
    adjustment: {
      code: "large-print",
      note: null,
      status: "active",
      review_by: null,
      reason: "they asked",
      ...over,
    },
    category: "communication",
    display: "Large print",
    overdue: false,
    ...top,
  }) as StoredAdjustment;

describe("the catalogue (PA-6)", () => {
  it("is the service's, code for code and category for category", () => {
    const rust = readFileSync(
      "../electronic-patient-record-service-with-rust/src/adjustments.rs",
      "utf8",
    );
    const block = rust.slice(
      rust.indexOf("pub const CATALOGUE"),
      rust.indexOf("/// The most characters"),
    );
    const service = [
      ...block.matchAll(/\(\s*"([a-z-]+)",\s*"([a-z-]+)",/g),
    ].map((m) => `${m[1]}/${m[2]}`);
    const here = CATEGORIES.flatMap((c) =>
      CATALOGUE[c].map((code) => `${c}/${code}`),
    );
    expect(here.sort()).toEqual(service.sort());
    expect(here).toHaveLength(23);
  });

  it("has words for every code and category", () => {
    for (const c of CATEGORIES) {
      expect(STRINGS.adjustmentCategories[c]).toBeTruthy();
      for (const code of CATALOGUE[c])
        expect(STRINGS.adjustmentCodes[code]).toBeTruthy();
    }
  });
});

describe("adjustmentName", () => {
  it("shows the note beside the need", () => {
    expect(
      adjustmentName(row({ code: "interpreter-spoken", note: "Polish" })),
    ).toBe("Spoken language interpreter (Polish)");
  });
  it("keeps the service's words for a code this page does not know", () => {
    expect(
      adjustmentName(row({ code: "new-code" }, { display: "A new need" })),
    ).toBe("A new need");
  });
});

describe("the banner (H-048)", () => {
  it("says nothing when the page did not ask", () => {
    expect(bannerAdjustments(null)).toEqual({ kind: "none" });
  });
  it("says none are recorded, in words, when the list is empty", () => {
    expect(bannerAdjustments({ status: 200, adjustments: [] })).toEqual({
      kind: "status",
      text: STRINGS.bannerNeedsNone,
    });
  });
  it("does not read a failed look as none", () => {
    expect(bannerAdjustments({ status: 500, adjustments: null })).toEqual({
      kind: "status",
      text: STRINGS.bannerNeedsFailed,
    });
  });
  it("says when a role may not see them", () => {
    expect(bannerAdjustments({ status: 403, adjustments: null })).toEqual({
      kind: "status",
      text: STRINGS.bannerNeedsNotShown,
    });
  });
  it("lists what stands, and leaves out what ended", () => {
    const b = bannerAdjustments({
      status: 200,
      adjustments: [
        row({ code: "braille" }),
        row({ code: "step-free", status: "ended" }),
      ],
    });
    expect(b.kind).toBe("needs");
    if (b.kind !== "needs") return;
    expect(b.text).toContain("Braille");
    expect(b.text).not.toContain("Step-free");
    expect(b.overdue).toBeNull();
  });
  it("says only ended needs are none standing", () => {
    expect(
      bannerAdjustments({
        status: 200,
        adjustments: [row({ status: "ended" })],
      }),
    ).toEqual({ kind: "status", text: STRINGS.bannerNeedsNone });
  });
  it("says when a review is overdue", () => {
    const b = bannerAdjustments({
      status: 200,
      adjustments: [row({}, { overdue: true })],
    });
    expect(b.kind === "needs" && b.overdue).toContain("1");
  });
});

describe("recordBody", () => {
  const form = {
    ...emptyAdjustment(),
    code: "braille",
    reason: " they asked ",
  } as const;
  it("sends what the service needs, trimmed", () => {
    expect(recordBody({ ...form, note: " " })).toEqual({
      ok: true,
      body: {
        code: "braille",
        note: null,
        review_by: null,
        reason: "they asked",
      },
    });
  });
  it("needs a need", () => {
    expect(recordBody(emptyAdjustment())).toEqual({
      ok: false,
      problem: STRINGS.adjustmentCodeNeeded,
    });
  });
  it("needs a language for a spoken interpreter", () => {
    expect(recordBody({ ...form, code: "interpreter-spoken" })).toEqual({
      ok: false,
      problem: STRINGS.adjustmentLanguageNeeded,
    });
  });
  it("needs a reason", () => {
    expect(recordBody({ ...form, reason: " " })).toEqual({
      ok: false,
      problem: STRINGS.adjustmentReasonNeeded,
    });
  });
});

describe("changeBody", () => {
  it("carries the next date on a review", () => {
    expect(changeBody("review", 2, "2027-01-01", "checked")).toEqual({
      ok: true,
      body: {
        expected_version: 2,
        action: "review",
        review_by: "2027-01-01",
        reason: "checked",
      },
    });
  });
  it("drops any date on an ending, and needs a reason", () => {
    const built = changeBody("end", 2, "2027-01-01", "done");
    expect(built.ok && built.body.review_by).toBeNull();
    expect(changeBody("end", 2, "", " ").ok).toBe(false);
  });
});

describe("adjustmentDetail", () => {
  it("says there is no review date when none is set", () => {
    expect(adjustmentDetail(row())).toContain(STRINGS.adjustmentNoReview);
  });
  it("says a review is overdue", () => {
    expect(
      adjustmentDetail(row({ review_by: "2020-01-01" }, { overdue: true })),
    ).toContain("2020-01-01");
    expect(
      adjustmentDetail(row({ review_by: "2020-01-01" }, { overdue: true })),
    ).toContain("overdue");
  });
  it("says when an ended need ended", () => {
    expect(adjustmentDetail(row({ status: "ended" }))).toContain("Ended");
  });
});
