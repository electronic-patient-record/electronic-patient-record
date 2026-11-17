import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { KEPT } from "../../src/lib/privacy";
import { STRINGS } from "../../src/lib/strings";
import { CY } from "../../src/lib/strings.cy";

// The privacy notice lists everything this app keeps in the browser. This reads the source: a new cookie or storage
// key, or a renamed one, fails here until the notice says so.

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((f) => {
    const p = join(dir, f);
    return statSync(p).isDirectory() ? files(p) : [p];
  });
}
const sources = files(join(process.cwd(), "src"))
  .filter((p) => /\.(ts|svelte|js)$/.test(p))
  .map((p) => ({ p, text: readFileSync(p, "utf8") }));

describe("privacy notice", () => {
  it("names every cookie and storage key the code defines, and no other", () => {
    const defined = new Set<string>();
    for (const { text } of sources) {
      // `export const X_COOKIE = "name"` and the purpose's `STORAGE_KEY = "name"`.
      for (const m of text.matchAll(
        /export const (?:\w+_COOKIE|STORAGE_KEY)\s*=\s*"([^"]+)"/g,
      ))
        defined.add(m[1]);
    }
    expect([...defined].sort()).toEqual(KEPT.map((k) => k.name).sort());
  });

  it("every cookie the code sets goes through a named constant, never a bare string", () => {
    for (const { p, text } of sources) {
      for (const m of text.matchAll(/cookies\.set\(\s*("[^"]*"|'[^']*')/g))
        throw new Error(`${p} sets the cookie ${m[1]} by a bare string`);
    }
  });

  it("nothing uses localStorage or writes document.cookie by hand: the notice lists no such use", () => {
    for (const { p, text } of sources) {
      expect(text, p).not.toMatch(/\blocalStorage\s*\.|window\.localStorage/);
      expect(text, p).not.toMatch(/document\.cookie\s*=/);
    }
  });

  it("the theme picker keeps its choice in the listed cookie and nowhere else", () => {
    const layout = readFileSync(
      join(process.cwd(), "src/routes/+layout.svelte"),
      "utf8",
    );
    expect(layout).toMatch(/cookieName=\{THEME_COOKIE\}/);
    expect(layout).toMatch(/storageKey=""/);
  });

  it("every row has its words in English and in Welsh", () => {
    for (const k of KEPT) {
      for (const table of [STRINGS, CY]) {
        const row = table.privacyRows[k.row];
        for (const part of [row.what, row.when, row.kept])
          expect(part.trim().length).toBeGreaterThan(0);
        expect(table.privacyPlaces[k.place].trim().length).toBeGreaterThan(0);
      }
    }
  });
});
