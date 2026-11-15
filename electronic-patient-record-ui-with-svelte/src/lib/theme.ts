// The theme a user chose (TH-1, spec/index.md "Themes"). Both themes offered pass WCAG 2.2 AA; "device" follows
// `prefers-color-scheme`, and is what anyone gets who has not chosen. The choice is kept in a cookie in this browser,
// not on the server, so it is this user's alone and never set for someone else.

/** The cookie holding the choice. */
export const THEME_COOKIE = "epr-theme";

/** The choices, in the order the control lists them. */
export const THEMES = ["device", "light", "dark"] as const;
export type Theme = (typeof THEMES)[number];

/** A cookie value as a theme, or "device" for anything else (missing, old or forged). */
export function normalizeTheme(value: string | undefined | null): Theme {
  return (THEMES as readonly string[]).includes(value ?? "")
    ? (value as Theme)
    : "device";
}

/** What `%theme%` in app.html becomes: an attribute that forces a theme, or nothing to follow the device. */
export function themeAttribute(theme: Theme): string {
  return theme === "device" ? "" : `data-color-scheme="${theme}"`;
}

/** The cookie that keeps `theme` for a year in this browser. */
export function themeCookie(theme: Theme, secure: boolean): string {
  return `${THEME_COOKIE}=${theme}; Path=/; Max-Age=${60 * 60 * 24 * 365}; SameSite=Lax${secure ? "; Secure" : ""}`;
}
