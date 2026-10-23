import { error, redirect, type RequestHandler } from "@sveltejs/kit";
import {
  cookieName,
  localeFrom,
  getAuth,
  LOGIN_COOKIE,
  SESSION_COOKIE,
} from "#lib/auth-instance.js";
import { localizePath } from "#lib/locales.js";
import { forwardedProto } from "#lib/forwarded-proto.js";
import { isHttps } from "#lib/security-headers.js";

export const GET: RequestHandler = async ({ url, cookies, request }) => {
  const auth = getAuth();
  if (!auth) error(404, "not found");
  const secure = isHttps(url, forwardedProto(request));
  const loginCookie = cookieName(LOGIN_COOKIE, secure);
  const loginId = cookies.get(loginCookie);
  cookies.delete(loginCookie, { path: "/" });

  let done;
  try {
    done = await auth.finishLogin(loginId, {
      code: url.searchParams.get("code"),
      state: url.searchParams.get("state"),
      error: url.searchParams.get("error"),
    });
  } catch {
    // The reason is deliberately not shown: it could help an attacker.
    error(400, "Sign-in failed. Close this window and try again.");
  }
  cookies.set(cookieName(SESSION_COOKIE, secure), done.session.id, {
    path: "/",
    httpOnly: true,
    sameSite: "lax",
    secure,
    maxAge: auth.config.absoluteSeconds,
  });
  redirect(303, localizePath(done.returnTo, localeFrom(cookies)));
};
