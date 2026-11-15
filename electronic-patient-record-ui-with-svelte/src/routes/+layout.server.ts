import { getAuth } from "#lib/auth-instance.js";
import { normalizeTheme, THEME_COOKIE } from "#lib/theme.js";
import type { LayoutServerLoad } from "./$types";

// Only whether sign-in exists, who is signed in and the idle limit (for the warning); never a token. And the theme
// this browser chose (TH-1), so the control shows it from the first render.
export const load: LayoutServerLoad = ({ locals, cookies }) => ({
  theme: normalizeTheme(cookies.get(THEME_COOKIE)),
  signIn: {
    enabled: getAuth() !== null,
    name: locals.user?.name ?? locals.user?.subject ?? null,
    idleSeconds: locals.user ? (getAuth()?.config.idleSeconds ?? null) : null,
  },
});
