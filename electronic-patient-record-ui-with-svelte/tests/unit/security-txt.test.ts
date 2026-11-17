// The RFC 9116 contact file and the disclosure policy it points to (SECURITY.md) agree, and neither has lapsed.
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const txt = readFileSync(
  resolve(process.cwd(), "static/.well-known/security.txt"),
  "utf8",
);
const policy = readFileSync(resolve(process.cwd(), "../SECURITY.md"), "utf8");
const securityPolicy = readFileSync(
  resolve(process.cwd(), "../docs/security/policy.md"),
  "utf8",
);

function fields(text: string): Map<string, string[]> {
  const out = new Map<string, string[]>();
  for (const line of text.split("\n")) {
    const m = /^([A-Za-z-]+):\s*(.+)$/.exec(line);
    if (m && !line.startsWith("#"))
      out.set(m[1], [...(out.get(m[1]) ?? []), m[2].trim()]);
  }
  return out;
}

const DAY = 24 * 60 * 60 * 1000;

function reviewDate(doc: string): Date {
  const m = /\| Next review \| (\d{4}-\d{2}-\d{2})/.exec(doc);
  expect(m, 'a "| Next review | YYYY-MM-DD" row').not.toBeNull();
  return new Date(`${m![1]}T00:00:00Z`);
}

describe("security.txt", () => {
  const f = fields(txt);

  it("has the fields RFC 9116 requires, once each where it says once", () => {
    expect(f.get("Contact")?.length ?? 0).toBeGreaterThan(0);
    expect(f.get("Expires")).toHaveLength(1);
  });

  it("expires in the future, and no more than a year ahead (RFC 9116 advises less than a year)", () => {
    const expires = new Date(f.get("Expires")![0]);
    expect(Number.isNaN(expires.getTime())).toBe(false);
    expect(expires.getTime()).toBeGreaterThan(Date.now());
    expect(expires.getTime() - Date.now()).toBeLessThanOrEqual(366 * DAY);
  });

  it("gives the same contact as SECURITY.md, and points to it", () => {
    for (const contact of f.get("Contact")!) {
      expect(contact).toMatch(/^(mailto:|https:\/\/)/);
      expect(policy).toContain(contact.replace(/^mailto:/, ""));
    }
    expect(f.get("Policy")?.[0]).toMatch(/\/SECURITY\.md$/);
  });
});

describe("the security policies are within their review dates", () => {
  it.each([
    ["SECURITY.md", policy],
    ["docs/security/policy.md", securityPolicy],
  ])("%s", (_name, doc) => {
    expect(reviewDate(doc).getTime()).toBeGreaterThan(Date.now());
  });
});
