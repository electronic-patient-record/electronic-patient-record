import * as env from "$app/env/private";
import { error, json, type RequestHandler } from "@sveltejs/kit";
import {
  cookieName,
  getAuth,
  sameOrigin,
  SESSION_COOKIE,
} from "#lib/auth-instance.js";
import {
  BACKGROUND_HEADER,
  isBackground,
  proxyTarget,
  purposeHeader,
} from "#lib/proxy.js";
import { isHttps } from "#lib/security-headers.js";

// The backend-for-frontend proxy. The browser never talks to the API directly and
// never holds a token: this adds the signed-in reader's access token server-side.
const forward: RequestHandler = async ({ params, url, request, cookies }) => {
  const base = env.EPR_API_URL || "http://localhost:5150";
  const target = proxyTarget(base, params.path ?? "", url.search);
  if (!target) error(404, "not found");

  const headers: Record<string, string> = {
    "content-type": request.headers.get("content-type") ?? "application/json",
  };
  const purpose = purposeHeader(request.headers.get("x-purpose-of-use"));
  if (purpose) headers["x-purpose-of-use"] = purpose;
  const auth = getAuth();
  if (auth) {
    if (
      request.method !== "GET" &&
      !sameOrigin(request, auth.config.publicUrl)
    ) {
      error(403, "cross-site request refused");
    }
    const secure = isHttps(url, request.headers.get("x-forwarded-proto"));
    const id = cookies.get(cookieName(SESSION_COOKIE, secure));
    const session = auth.store.get(id, {
      activity: !isBackground(request.headers.get(BACKGROUND_HEADER)),
    });
    const token = session ? await auth.accessToken(session) : null;
    if (!token) {
      cookies.delete(cookieName(SESSION_COOKIE, secure), { path: "/" });
      return json({ error: "sign in required" }, { status: 401 });
    }
    headers.authorization = `Bearer ${token}`;
  }

  const upstream = await fetch(target, {
    method: request.method,
    headers,
    // Bytes, not text: a photo is binary and text decoding would corrupt it.
    body: request.method === "GET" ? undefined : await request.arrayBuffer(),
  });
  return new Response(upstream.body, {
    status: upstream.status,
    headers: {
      "content-type":
        upstream.headers.get("content-type") ?? "application/json",
      // What an image response needs to stay private and unsniffed.
      ...Object.fromEntries(
        [
          "cache-control",
          "x-content-type-options",
          "etag",
          "content-disposition",
        ]
          .map((h) => [h, upstream.headers.get(h)] as const)
          .filter(([, v]) => v !== null) as [string, string][],
      ),
    },
  });
};

export const GET = forward;
export const POST = forward;
export const PUT = forward;
