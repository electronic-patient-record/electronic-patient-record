import { error, redirect, type RequestHandler } from "@sveltejs/kit";
import {
  cookieName,
  localeFrom,
  getAuth,
  sameOrigin,
  SESSION_COOKIE,
} from "#lib/auth-instance.js";
import { localizePath } from "#lib/locales.js";
import { isHttps } from "#lib/security-headers.js";

export const POST: RequestHandler = async ({ url, cookies, request }) => {
  const auth = getAuth();
  if (!auth) error(404, "not found");
  if (!sameOrigin(request, auth.config.publicUrl))
    error(403, "cross-site request refused");
  const name = cookieName(
    SESSION_COOKIE,
    isHttps(url, request.headers.get("x-forwarded-proto")),
  );
  await auth.signOut(cookies.get(name));
  cookies.delete(name, { path: "/" });
  redirect(303, localizePath("/", localeFrom(cookies)));
};
