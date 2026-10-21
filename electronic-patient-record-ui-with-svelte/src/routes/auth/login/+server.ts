import { error, redirect, type RequestHandler } from "@sveltejs/kit";
import { cookieName, getAuth, LOGIN_COOKIE } from "#lib/auth-instance.js";
import { isHttps } from "#lib/security-headers.js";

export const GET: RequestHandler = async ({ url, cookies, request }) => {
  const auth = getAuth();
  if (!auth) error(404, "not found");
  let started;
  try {
    started = await auth.startLogin(url.searchParams.get("returnTo"));
  } catch {
    error(502, "The identity provider is not reachable.");
  }
  const secure = isHttps(url, request.headers.get("x-forwarded-proto"));
  // Lax, not Strict: the provider sends the browser back with a cross-site navigation.
  cookies.set(cookieName(LOGIN_COOKIE, secure), started.loginId, {
    path: "/",
    httpOnly: true,
    sameSite: "lax",
    secure,
    maxAge: 600,
  });
  // Only to the identity provider's own origin; anything else fails closed.
  redirect(302, started.url, {
    external: [new URL(auth.config.issuer).origin],
  });
};
