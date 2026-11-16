// The theme a user chose (TH-1, spec/index.md "Themes"). The control is Lily's ThemePicker
// (`@lilydesignsystem/svelte-theme-picker`), which writes the cookie and sets `data-theme` on <html> in the page;
// this module is what the server needs to send the next page in the same theme. Both themes offered pass WCAG 2.2
// AA; "system" follows `prefers-color-scheme`, and is what anyone gets who has not chosen. The choice is kept in a
// cookie in this browser, not on the server, so it is this user's alone and never set for someone else.
import { SYSTEM_THEME } from "@lilydesignsystem/svelte-theme-picker";

/** The cookie holding the choice (the picker's `cookieName`). */
export const THEME_COOKIE = "epr-theme";

/** The themes offered: the two passing themes, and no other. Each is a stylesheet in `static/themes/`. */
export const PASSING_THEMES = ["light", "dark"] as const;

/** The choices, in the order the control lists them: following the device first. */
export const THEMES = [SYSTEM_THEME, ...PASSING_THEMES] as const;
export type Theme = (typeof THEMES)[number];

/** A cookie value as a choice, or "system" for anything else (missing, old, such as "device", or forged). */
export function normalizeTheme(value: string | undefined | null): Theme {
  return (THEMES as readonly string[]).includes(value ?? "")
    ? (value as Theme)
    : SYSTEM_THEME;
}

/** What `%theme%` in app.html becomes: an attribute that forces a theme, or nothing to follow the device. */
export function themeAttribute(theme: Theme): string {
  return theme === SYSTEM_THEME ? "" : `data-theme="${theme}"`;
}
