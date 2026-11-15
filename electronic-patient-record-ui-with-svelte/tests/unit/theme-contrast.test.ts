import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

// The two passing themes (TH-1, spec/index.md "Themes"): every colour pair a page uses meets WCAG 2.2 AA in light
// and in dark. axe checks rendered text on the pages; it does not check every non-text edge (an input's border, the
// focus ring), so this checks the tokens themselves. A new token, or a pair below its ratio, fails here.

const css = readFileSync(join(process.cwd(), "src/app.css"), "utf8");

function tokens(block: string): Record<string, string> {
  const out: Record<string, string> = {};
  for (const m of block.matchAll(/--([a-z-]+):\s*(#[0-9a-fA-F]{6})\s*;/g))
    out[m[1]] = m[2].toLowerCase();
  return out;
}

const light = tokens(
  css.slice(css.indexOf(":root {"), css.indexOf("}", css.indexOf(":root {"))),
);
const darkStart = css.indexOf(':root:not([data-color-scheme="light"])');
const dark = tokens(css.slice(darkStart, css.indexOf("}", darkStart)));

/** WCAG 2.2 relative luminance of a #rrggbb colour. */
export function luminance(hex: string): number {
  const [r, g, b] = [1, 3, 5].map((i) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** WCAG 2.2 contrast ratio of two colours, 1 to 21. */
export function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

// [foreground token, background token, the ratio it must reach, why]
const PAIRS: [string, string, number, string][] = [
  ["fg", "bg", 4.5, "body text (1.4.3)"],
  ["muted", "bg", 4.5, "small and secondary text (1.4.3)"],
  ["accent", "bg", 4.5, "links (1.4.3)"],
  ["fg", "warn-bg", 4.5, "text in a notice or the banner (1.4.3)"],
  ["accent", "warn-bg", 4.5, "a link in a notice or the banner (1.4.3)"],
  ["muted", "warn-bg", 4.5, "secondary text in a notice (1.4.3)"],
  [
    "control-border",
    "bg",
    3,
    "the edge of an input, select or textarea (1.4.11)",
  ],
  ["accent", "bg", 3, "the focus ring (1.4.11)"],
  ["accent", "warn-bg", 3, "the focus ring inside a notice (1.4.11)"],
  [
    "warn-border",
    "warn-bg",
    3,
    "a notice's edge against its own background (1.4.11)",
  ],
  ["warn-border", "bg", 3, "a notice's edge against the page (1.4.11)"],
];

describe("theme contrast", () => {
  it("reads both themes' tokens from app.css", () => {
    expect(Object.keys(light).length).toBeGreaterThan(5);
    expect(light.bg).toBe("#ffffff");
    expect(dark.bg).not.toBe(light.bg);
  });

  it("dark sets every token light sets, so none falls back to a light colour on a dark page", () => {
    expect(Object.keys(dark).sort()).toEqual(Object.keys(light).sort());
  });

  it("computes WCAG's ratio: black on white is 21, a colour on itself is 1", () => {
    expect(contrast("#000000", "#ffffff")).toBeCloseTo(21, 5);
    expect(contrast("#767676", "#767676")).toBe(1);
    expect(contrast("#767676", "#ffffff")).toBeCloseTo(4.54, 2);
  });

  for (const [name, theme] of [
    ["light", light],
    ["dark", dark],
  ] as const) {
    for (const [fg, bg, min, why] of PAIRS) {
      it(`${name}: --${fg} on --${bg} reaches ${min}:1, for ${why}`, () => {
        expect(
          theme[fg],
          `--${fg} is not defined in the ${name} theme`,
        ).toBeDefined();
        expect(
          theme[bg],
          `--${bg} is not defined in the ${name} theme`,
        ).toBeDefined();
        expect(contrast(theme[fg], theme[bg])).toBeGreaterThanOrEqual(min);
      });
    }
  }

  it("form controls draw their edge with --control-border, not the decorative --border", () => {
    for (const selector of [
      /\ninput \{[^}]*\}/,
      /\nselect,\ntextarea \{[^}]*\}/,
    ]) {
      const rule = css.match(selector)?.[0] ?? "";
      expect(rule, String(selector)).toContain("var(--control-border)");
    }
  });

  it("the page follows the device unless the user chose: app.html forces no scheme, only a placeholder", () => {
    const html = readFileSync(join(process.cwd(), "src/app.html"), "utf8");
    expect(html).not.toMatch(/<html[^>]*data-color-scheme/);
    expect(html).toMatch(/<html[^>]*%theme%/);
  });

  it("dark by choice and dark by the device set the same tokens", () => {
    const start = css.indexOf(':root[data-color-scheme="dark"]');
    const chosen = tokens(css.slice(start, css.indexOf("}", start)));
    expect(chosen).toEqual(dark);
  });
});
