import { error, type RequestHandler } from "@sveltejs/kit";
import { getAuth, sameOrigin } from "#lib/auth-instance.js";

// "Stay signed in" (CQ-8). The hook has already looked up the session and, this being an ordinary
// request, counted it as activity; this only says whether a session is still there. It calls no API,
// so it is not an audited read.
export const POST: RequestHandler = ({ request, locals }) => {
  const auth = getAuth();
  if (!auth) error(404, "not found");
  if (!sameOrigin(request, auth.config.publicUrl))
    error(403, "cross-site request refused");
  return new Response(null, { status: locals.user ? 204 : 401 });
};
