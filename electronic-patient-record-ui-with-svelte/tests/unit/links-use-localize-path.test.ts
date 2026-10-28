import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

// A link or redirect to a page of this app must go through `localizePath`, or it drops the
// locale prefix. ESLint's `svelte/no-navigation-without-resolve` is off because it knows only
// SvelteKit's `resolve`, so this test is the guard. Allowed without it: an in-page `#anchor`, the
// `/auth/…` endpoints (not localized), and the redirect to the identity provider.
function files(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) files(p, out);
    else if (/\.(svelte|ts)$/.test(name)) out.push(p);
  }
  return out;
}

const NAVIGATES = /\bhref=|\bgoto\(|\bredirect\(/;
const ALLOWED = [
  /localizePath\(/,
  /href="#/,
  /href="\/auth\//,
  /redirect\(\s*30[12]\s*,\s*started\.url/,
  /redirect\(\s*30[12],?\s*$/, // the identity-provider redirect, argument on the next line
];

describe("links and redirects", () => {
  const sources = files(join(process.cwd(), "src")).filter(
    (p) => !p.endsWith("strings.ts"),
  );

  it("found the pages", () => {
    expect(sources.length).toBeGreaterThan(20);
  });

  it("go through localizePath", () => {
    const bad: string[] = [];
    for (const path of sources) {
      const lines = readFileSync(path, "utf8").split("\n");
      lines.forEach((line, i) => {
        if (!NAVIGATES.test(line)) return;
        // A call may carry its argument on the next line.
        const here = `${line}\n${lines[i + 1] ?? ""}`;
        if (!ALLOWED.some((re) => re.test(line) || re.test(here))) {
          bad.push(`${path}:${i + 1}: ${line.trim()}`);
        }
      });
    }
    expect(bad, bad.join("\n")).toEqual([]);
  });
});
