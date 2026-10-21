// The process-wide sign-in singleton, built from the environment. Sign-in is
// off (null) when EPR_OIDC_ISSUER is unset; a bad value throws at startup.
import * as env from "$app/env/private";
import type { Cookies } from "@sveltejs/kit";
import { Auth } from "./auth.js";
import { DEFAULT_LOCALE, normalizeLocale, type Locale } from "./locales.js";
import { loadOidcConfig } from "./oidc.js";
import { SessionStore, type Session } from "./session.js";

let instance: Auth | null | undefined;

export function getAuth(): Auth | null {
  if (instance === undefined) {
    const config = loadOidcConfig({
      issuer: env.EPR_OIDC_ISSUER,
      clientId: env.EPR_OIDC_CLIENT_ID,
      publicUrl: env.EPR_PUBLIC_URL,
      idleSeconds: env.EPR_SESSION_IDLE_SECONDS,
      absoluteSeconds: env.EPR_SESSION_ABSOLUTE_SECONDS,
    });
    instance = config
      ? new Auth(
          config,
          new SessionStore(config.idleSeconds, config.absoluteSeconds),
        )
      : null;
  }
  return instance;
}

/** `__Host-` pins a cookie to this host, path `/` and HTTPS; it needs `Secure`, so only on HTTPS. */
export function cookieName(base: string, secure: boolean): string {
  return secure ? `__Host-${base}` : base;
}

export const SESSION_COOKIE = "epr_session";
export const LOGIN_COOKIE = "epr_login";

export function sessionFrom(
  auth: Auth | null,
  cookies: Cookies,
  secure: boolean,
  options: { activity?: boolean } = {},
): Session | null {
  return auth
    ? auth.store.get(cookies.get(cookieName(SESSION_COOKIE, secure)), options)
    : null;
}

/**
 * State-changing requests must come from this site. SvelteKit already checks
 * form posts; this also covers the JSON proxy and the sign-out form.
 */
export function sameOrigin(request: Request, expectedOrigin: string): boolean {
  return request.headers.get("origin") === expectedOrigin;
}

/** The reader's remembered locale, for redirects from the unlocalized `/auth` routes. */
export function localeFrom(cookies: Cookies): Locale {
  return normalizeLocale(cookies.get("locale")) ?? DEFAULT_LOCALE;
}
