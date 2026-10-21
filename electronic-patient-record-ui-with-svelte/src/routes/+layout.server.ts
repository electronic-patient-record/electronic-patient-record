import { getAuth } from "#lib/auth-instance.js";
import type { LayoutServerLoad } from "./$types";

// Only whether sign-in exists, who is signed in and the idle limit (for the warning); never a token.
export const load: LayoutServerLoad = ({ locals }) => ({
  signIn: {
    enabled: getAuth() !== null,
    name: locals.user?.name ?? locals.user?.subject ?? null,
    idleSeconds: locals.user ? (getAuth()?.config.idleSeconds ?? null) : null,
  },
});
