import { describe, expect, it } from "vitest";
import { nextPath } from "../../src/lib/next-link";

describe("nextPath", () => {
  it("reads the next page of a list as a path under /api/", () => {
    expect(
      nextPath('</api/patients?family=Smi&limit=50&cursor=AbC_-1>; rel="next"'),
    ).toBe("/api/patients?family=Smi&limit=50&cursor=AbC_-1");
    expect(nextPath('  </api/x>;rel="next"  ')).toBe("/api/x");
  });

  it("drops anything that could send the proxy or the browser somewhere else", () => {
    for (const bad of [
      null,
      undefined,
      "",
      '<https://evil.example/api/x>; rel="next"',
      '<//evil.example/api/x>; rel="next"',
      '</api//evil.example>; rel="next"',
      '</api/../admin>; rel="next"',
      '</api\\x>; rel="next"',
      '</other/x>; rel="next"',
      '</api/x>; rel="prev"',
      '</api/x>; rel="next", </api/y>; rel="next"',
      "</api/x>",
    ])
      expect(nextPath(bad), String(bad)).toBeNull();
  });
});
