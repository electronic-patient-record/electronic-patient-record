import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

// Every page must pass the axe audit (WCAG 2.2 AA). This fails when a page under `src/routes` is
// never opened by an end-to-end spec that runs axe, so a new page cannot skip the audit. It checks
// that a spec which runs axe visits the page, not that axe ran on it after every state: the audit
// itself is in the specs. Idea from the patient-administration-system project.
function pages(dir: string, base = ""): string[] {
  const out: string[] = [];
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) out.push(...pages(p, `${base}/${name}`));
    else if (name === "+page.svelte") out.push(base || "/");
  }
  return out;
}

const specDir = join(process.cwd(), "tests/e2e");
const visited = new Set<string>();
for (const name of readdirSync(specDir).filter((n) => n.endsWith(".spec.ts"))) {
  const text = readFileSync(join(specDir, name), "utf8");
  if (!text.includes("AxeBuilder")) continue;
  for (const m of text.matchAll(/goto\(\s*([`"'])(.*?)\1(?:\s*\+\s*[\w.]+)?\s*[,)]/g)) {
    // Locale prefix off; a template hole or concatenated id is a `[id]`.
    let path = m[2].replace(/^\/(?:\$\{locale\}|[a-z]{2}-[a-z0-9]{2,3})(?=\/|$)/, "") || "/";
    path = path.replace(/\$\{[^}]*\}/g, "[id]").replace(/\/[0-9a-f]{8}-[0-9a-f-]{27}/g, "/[id]");
    if (/\/$/.test(path) && path !== "/") path += "[id]";
    visited.add(path);
  }
}

describe("axe route coverage", () => {
  it("found the pages and the specs", () => {
    expect(pages(join(process.cwd(), "src/routes")).length).toBeGreaterThanOrEqual(8);
    expect(visited.size).toBeGreaterThanOrEqual(6);
  });
  it("opens every page in a spec that runs axe", () => {
    const missing = pages(join(process.cwd(), "src/routes")).filter((route) => !visited.has(route));
    expect(missing, `pages no axe spec opens: ${missing.join(", ")} (opened: ${[...visited].join(", ")})`).toEqual([]);
  });
});
