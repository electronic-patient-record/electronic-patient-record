import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { PRODUCTION_STEPS, plain } from "../../src/lib/path-to-production";

// PR-7. The page's list is a copy of the evidence checklist's table. This reads the checklist and fails when a
// line is added, removed, reordered or reworded on one side only, or when an estimate differs, so the page
// cannot say something the checklist does not.
const checklist = readFileSync(
  join(process.cwd(), "../governance/production-readiness.md"),
  "utf8",
);

/** The rows of the first table: area, question, evidence, today, estimate. */
function rows(markdown: string): string[][] {
  const lines = markdown.split("\n");
  const start = lines.findIndex((l) => l.startsWith("| Area | Question |"));
  const out: string[][] = [];
  for (const line of lines.slice(start + 2)) {
    if (!line.startsWith("|")) break;
    out.push(
      line
        .slice(1, -1)
        .split(" | ")
        .map((cell) => cell.trim()),
    );
  }
  return out;
}

describe("path to production", () => {
  const table = rows(checklist);

  it("found the checklist's table and its estimate column", () => {
    expect(checklist).toMatch(
      /^\| Area \| Question \| Evidence needed \| What exists today \| Estimate \|$/m,
    );
    expect(table.length).toBeGreaterThanOrEqual(10);
  });

  it("lists the checklist's lines, in its order and its words, with its estimates", () => {
    expect(
      PRODUCTION_STEPS.map((s) => [
        s.area,
        s.question,
        s.today,
        `${s.estimate}%`,
      ]),
    ).toEqual(
      table.map(([area, question, , today, estimate]) => [
        area,
        question,
        today,
        estimate,
      ]),
    );
  });

  it("keeps every estimate a whole number from 0 to 100, on the slider's steps of 5", () => {
    for (const s of PRODUCTION_STEPS) {
      expect(Number.isInteger(s.estimate)).toBe(true);
      expect(s.estimate).toBeGreaterThanOrEqual(0);
      expect(s.estimate).toBeLessThanOrEqual(100);
      expect(s.estimate % 5).toBe(0);
      expect(s.why.trim()).not.toBe("");
    }
  });

  // Nothing here has been looked at on a real deployment, which is what 100 means.
  it("claims no line is done", () => {
    expect(PRODUCTION_STEPS.filter((s) => s.estimate === 100)).toEqual([]);
  });

  it("shows the checklist's text without its Markdown marks", () => {
    expect(plain("**No backup procedure exists**")).toBe(
      "No backup procedure exists",
    );
    expect(plain("`/metrics` exists (off by default)")).toBe(
      "/metrics exists (off by default)",
    );
  });
});
