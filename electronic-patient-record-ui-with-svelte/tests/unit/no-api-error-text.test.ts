import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

// What the API says in an error never reaches a page, with one exception: the 422 list of problems
// (`errors`), which is the service's own fixed wording about a record it refused (CQ-11). Anything
// else, a 5xx above all, is logged by the service and shown here as the page's own words
// (`STRINGS.error`). Idea from the patient-administration-system project.
function files(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) files(p, out);
    else if (/\.(svelte|ts)$/.test(name)) out.push(p);
  }
  return out;
}

const sources = files(join(process.cwd(), "src")).map((p) => [p, readFileSync(p, "utf8")] as const);

describe("API error text on pages", () => {
  it("shows a list of problems only when the status is 422", () => {
    const bad: string[] = [];
    for (const [path, text] of sources) {
      const lines = text.split("\n");
      lines.forEach((line, i) => {
        if (!/errors\??\.join\(/.test(line)) return;
        // The 422 test is on this line or the six before it.
        const near = lines.slice(Math.max(0, i - 6), i + 1).join("\n");
        if (!/status === 422/.test(near)) bad.push(`${path}:${i + 1}: ${line.trim()}`);
      });
    }
    expect(bad, bad.join("\n")).toEqual([]);
  });

  it("never shows the `error` or `message` field of an API answer", () => {
    const bad: string[] = [];
    for (const [path, text] of sources) {
      text.split("\n").forEach((line, i) => {
        if (/\b(res|body|answer|response)\??\.(body\??\.)?(error|message)\b/.test(line) && !/params\./.test(line)) {
          bad.push(`${path}:${i + 1}: ${line.trim()}`);
        }
      });
    }
    expect(bad, bad.join("\n")).toEqual([]);
  });
});
