import { describe, expect, it } from "vitest";
import {
  normalizeTheme,
  THEME_COOKIE,
  THEMES,
  themeAttribute,
  themeCookie,
} from "../../src/lib/theme";

// The theme control's choice (TH-1, spec/index.md "Themes").
describe("theme", () => {
  it("offers following the device first, then light and dark: the two passing themes, and no other", () => {
    expect(THEMES).toEqual(["device", "light", "dark"]);
  });

  it("reads anything that is not a choice as following the device", () => {
    expect(normalizeTheme("light")).toBe("light");
    expect(normalizeTheme("dark")).toBe("dark");
    for (const v of [
      undefined,
      null,
      "",
      "Dark",
      "high-contrast",
      '"><script>',
    ]) {
      expect(normalizeTheme(v)).toBe("device");
    }
  });

  it("puts an attribute on <html> only for an explicit choice", () => {
    expect(themeAttribute("device")).toBe("");
    expect(themeAttribute("light")).toBe('data-color-scheme="light"');
    expect(themeAttribute("dark")).toBe('data-color-scheme="dark"');
  });

  it("keeps the choice a year, for the whole site, Secure on HTTPS", () => {
    expect(themeCookie("dark", false)).toBe(
      `${THEME_COOKIE}=dark; Path=/; Max-Age=31536000; SameSite=Lax`,
    );
    expect(themeCookie("light", true)).toMatch(/; Secure$/);
  });
});
