// The task page's rules (TL-3): where a task's source sits on the patient's page, when it is overdue, what a close's
// reason must be, and today at the site.
import { describe, expect, it } from "vitest";
import {
  overdue,
  reasonProblem,
  siteToday,
  sourceAnchor,
  TEAMS,
} from "../../src/lib/tasks.js";
import { STRINGS } from "../../src/lib/strings.js";

describe("tasks", () => {
  it("links an observation and a referral to their sections, and a result to the record", () => {
    expect(sourceAnchor("observation")).toBe("#vitals-h");
    expect(sourceAnchor("referral")).toBe("#referrals-h");
    expect(sourceAnchor("treatment-refusal")).toBe("#refusals-h");
    expect(sourceAnchor("result")).toBe("");
  });

  it("is overdue only after its due day, on site days", () => {
    expect(overdue("2026-11-17", "2026-11-18")).toBe(true);
    expect(overdue("2026-11-18", "2026-11-18")).toBe(false);
    expect(overdue("2026-11-19", "2026-11-18")).toBe(false);
  });

  it("needs a reason of 1 to 500 characters once trimmed, as the service checks", () => {
    expect(reasonProblem("  ")).toBe("required");
    expect(reasonProblem("reviewed")).toBeNull();
    expect(reasonProblem("x".repeat(500))).toBeNull();
    expect(reasonProblem("x".repeat(501))).toBe("tooLong");
    // Characters, not UTF-16 units: 500 emoji are allowed.
    expect(reasonProblem("🩺".repeat(500))).toBeNull();
  });

  it("counts today on the site's calendar, not the reader's", () => {
    const at = new Date("2026-11-18T23:30:00Z");
    expect(siteToday(at, "UTC")).toBe("2026-11-18");
    expect(siteToday(at, "Pacific/Auckland")).toBe("2026-11-19");
    expect(siteToday(at, "America/New_York")).toBe("2026-11-18");
    expect(siteToday(at, "Not/AZone")).toBe("2026-11-18");
  });

  it("offers only staff roles as teams, each with a name to show", () => {
    for (const t of TEAMS) expect(STRINGS.roleNames[t]).toBeTruthy();
    expect(TEAMS).not.toContain("patient");
    expect(TEAMS).not.toContain("auditor");
  });
});
