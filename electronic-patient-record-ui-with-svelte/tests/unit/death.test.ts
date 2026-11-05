import { describe, expect, it } from "vitest";
import {
  assertionLine,
  bannerDeath,
  emptyReport,
  reportBody,
  stepBody,
  stepsFor,
  type Answer,
  type Status,
  type StoredAssertion,
} from "../../src/lib/death";
import { STRINGS } from "../../src/lib/strings";

const asserted = (
  over: Partial<Extract<Answer, { answer: "asserted" }>> = {},
): Answer => ({
  answer: "asserted",
  status: "reported",
  confirmed: false,
  occurred: "2026-11-01",
  dates_differ: false,
  holds_contact: true,
  ...over,
});

describe("the banner line", () => {
  it("says nothing when it did not ask, and when no assertion stands", () => {
    expect(bannerDeath(null)).toEqual({ kind: "none" });
    expect(
      bannerDeath({
        status: 200,
        answer: { answer: "not-known-to-have-died", holds_contact: false },
      }),
    ).toEqual({ kind: "none" });
  });

  it("never reads a failed look as nothing reported", () => {
    for (const status of [0, 403, 500]) {
      expect(bannerDeath({ status, answer: null }).kind).toBe("unknown");
    }
  });

  it("marks a report as not confirmed and says that contact is held", () => {
    const line = bannerDeath({ status: 200, answer: asserted() });
    expect(line.kind).toBe("asserted");
    if (line.kind !== "asserted") return;
    expect(line.text).toContain("Not confirmed");
    expect(line.text).toContain("2026-11-01");
    expect(line.text).toContain(STRINGS.bannerDeathHeld);
  });

  it("says when a confirmed death has no date or the reports differ", () => {
    const line = bannerDeath({
      status: 200,
      answer: asserted({
        status: "registered",
        confirmed: true,
        occurred: null,
        dates_differ: true,
      }),
    });
    if (line.kind !== "asserted") throw new Error("expected a line");
    expect(line.text).toContain("HAS DIED (Registered)");
    expect(line.text).toContain(STRINGS.bannerDeathNoDate);
    expect(line.text).toContain(STRINGS.bannerDeathDatesDiffer);
  });

  it("no string in this feature says alive", () => {
    const text = JSON.stringify(STRINGS).toLowerCase();
    expect(text).not.toContain("alive");
  });
});

describe("the steps offered", () => {
  it("go forward only, and withdrawal is final", () => {
    const table: [Status, Status[]][] = [
      [
        "reported",
        ["verified", "certified", "registered", "withdrawn-in-error"],
      ],
      ["verified", ["certified", "registered", "withdrawn-in-error"]],
      ["certified", ["registered", "withdrawn-in-error"]],
      ["registered", ["withdrawn-in-error"]],
      ["withdrawn-in-error", []],
    ];
    for (const [from, to] of table) expect(stepsFor(from)).toEqual(to);
  });
});

describe("what is sent", () => {
  it("a report needs a source, a date or why not, and a reason", () => {
    expect(reportBody(emptyReport())).toEqual({
      ok: false,
      problem: STRINGS.deathSourceNeeded,
    });
    const base = { ...emptyReport(), source: "hospital" as const };
    expect(reportBody(base)).toEqual({
      ok: false,
      problem: STRINGS.deathDateNeeded,
    });
    expect(reportBody({ ...base, date: "2026-11" })).toEqual({
      ok: false,
      problem: STRINGS.deathReasonNeeded,
    });
    expect(
      reportBody({ ...base, date: " 2026-11 ", reason: " rang " }),
    ).toEqual({
      ok: true,
      body: {
        source: "hospital",
        occurred: { kind: "known", at: "2026-11" },
        reason: "rang",
      },
    });
    expect(
      reportBody({ ...base, noDateReason: "not said", reason: "rang" }),
    ).toEqual({
      ok: true,
      body: {
        source: "hospital",
        occurred: { kind: "not-known", reason: "not said" },
        reason: "rang",
      },
    });
  });

  it("a step needs a reason", () => {
    expect(stepBody("verified", 1, " ").ok).toBe(false);
    expect(stepBody("verified", 3, " seen ")).toEqual({
      ok: true,
      body: { expected_version: 3, to: "verified", reason: "seen" },
    });
  });
});

it("an assertion reads as one line", () => {
  const a: StoredAssertion = {
    id: "a",
    patient_id: "p",
    version: 1,
    author: "x",
    recorded_at: "2026-11-02T10:00:00Z",
    assertion: {
      source: "care-home",
      reported_by: { kind: "person", id: "x" },
      occurred: { kind: "not-known", reason: "found at home" },
      status: "reported",
      reason: "rang",
    },
  };
  const line = assertionLine(a, "UTC");
  expect(line).toContain("Reported, from a care home.");
  expect(line).toContain("No date: found at home.");
});
