import * as env from "$app/env/private";
import {
  cookieName,
  getAuth,
  sameOrigin,
  SESSION_COOKIE,
} from "#lib/auth-instance.js";
import { forwardedProto } from "#lib/forwarded-proto.js";
import { createProxyHandler } from "#lib/proxy-handler.js";

// The backend-for-frontend proxy. The browser never talks to the API directly and
// never holds a token: the handler adds the signed-in reader's access token server-side.
const forward = createProxyHandler({
  apiUrl: env.EPR_API_URL || "http://localhost:5150",
  getAuth,
  cookieName,
  sessionCookie: SESSION_COOKIE,
  sameOrigin,
  forwardedProto,
});

export const GET = forward;
export const POST = forward;
export const PUT = forward;
