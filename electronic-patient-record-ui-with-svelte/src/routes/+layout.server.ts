import { getAuth } from "#lib/auth-instance.js";
import type { LayoutServerLoad } from "./$types";

// Only whether sign-in exists and who is signed in; never a token.
export const load: LayoutServerLoad = ({ locals }) => ({
  signIn: {
    enabled: getAuth() !== null,
    name: locals.user?.name ?? locals.user?.subject ?? null,
  },
});
