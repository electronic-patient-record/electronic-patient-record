import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  readThemeCookie,
  SYSTEM_THEME,
} from "@lilydesignsystem/svelte-theme-picker";
import {
  normalizeTheme,
  PASSING_THEMES,
  THEMES,
  themeAttribute,
} from "../../src/lib/theme";

// The theme control's choice (TH-1, spec/index.md "Themes"). The control is Lily's ThemePicker; this is what the
// server reads from its cookie.
describe("theme", () => {
  it("offers following the device first, then light and dark: the two passing themes, and no other", () => {
    expect(SYSTEM_THEME).toBe("system");
    expect(THEMES).toEqual(["system", "light", "dark"]);
    expect(PASSING_THEMES).toEqual(["light", "dark"]);
  });

  it("reads anything that is not a choice as following the device, the old value 'device' included", () => {
    expect(normalizeTheme("light")).toBe("light");
    expect(normalizeTheme("dark")).toBe("dark");
    expect(normalizeTheme("system")).toBe("system");
    for (const v of [
      undefined,
      null,
      "",
      "device",
      "Dark",
      "high-contrast",
      '"><script>',
    ]) {
      expect(normalizeTheme(v)).toBe("system");
    }
  });

  it("puts an attribute on <html> only for an explicit choice", () => {
    expect(themeAttribute("system")).toBe("");
    expect(themeAttribute("light")).toBe('data-theme="light"');
    expect(themeAttribute("dark")).toBe('data-theme="dark"');
  });

  it("the picker reads back only a choice it offers, so a forged cookie never reaches a stylesheet URL", () => {
    expect(readThemeCookie("epr-theme=dark", "epr-theme", THEMES)).toBe("dark");
    expect(readThemeCookie("epr-theme=..%2Fx", "epr-theme", THEMES)).toBe("");
  });

  it("every passing theme has the stylesheet the picker loads", () => {
    for (const t of PASSING_THEMES) {
      const css = readFileSync(
        join(process.cwd(), "static/themes", `${t}.css`),
        "utf8",
      );
      expect(css, `${t}.css holds no tokens: app.css has them`).not.toMatch(
        /--[a-z-]+\s*:/,
      );
    }
  });
});
