// The process-wide sign-in singleton, built from the environment. Sign-in is
// off (null) when EPR_OIDC_ISSUER is unset; a bad value throws at startup.
import * as env from "$app/env/private";
import type { Cookies } from "@sveltejs/kit";
import { Auth } from "./auth.js";
import { DEFAULT_LOCALE, normalizeLocale, type Locale } from "./locales.js";
import { loadOidcConfig } from "./oidc.js";
import { createClient } from "redis";
import {
  MemoryBackend,
  RedisBackend,
  type SessionBackend,
} from "./session-backends.js";
import { parseKey } from "./session-crypto.js";
import { SessionStore, type Session } from "./session.js";

let instance: Auth | null | undefined;

/**
 * Memory unless `EPR_SESSION_REDIS_URL` is set. Redis needs `EPR_SESSION_ENCRYPTION_KEY` too (32
 * bytes, base64): tokens are encrypted before they leave this process. Half a setting stops startup.
 */
export function backendFrom(
  redisUrl: string,
  encryptionKey: string,
): SessionBackend {
  if (!redisUrl) {
    if (encryptionKey)
      throw new Error(
        "EPR_SESSION_ENCRYPTION_KEY is set without EPR_SESSION_REDIS_URL",
      );
    return new MemoryBackend();
  }
  if (!encryptionKey)
    throw new Error("EPR_SESSION_REDIS_URL needs EPR_SESSION_ENCRYPTION_KEY");
  const key = parseKey(encryptionKey);
  const client = createClient({ url: redisUrl });
  client.on("error", (e) => console.error("session store (Redis):", e.message));
  let connecting: Promise<unknown> | null = null;
  const ready = async () => {
    connecting ??= client.connect();
    await connecting;
  };
  return new RedisBackend(
    {
      get: async (k) => (await ready(), client.get(k)),
      set: async (k, v, o) => (await ready(), client.set(k, v, o)),
      getDel: async (k) => (await ready(), client.getDel(k)),
      del: async (k) => (await ready(), client.del(k)),
    },
    key,
  );
}

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
          new SessionStore(
            config.idleSeconds,
            config.absoluteSeconds,
            undefined,
            backendFrom(
              env.EPR_SESSION_REDIS_URL,
              env.EPR_SESSION_ENCRYPTION_KEY,
            ),
          ),
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

export async function sessionFrom(
  auth: Auth | null,
  cookies: Cookies,
  secure: boolean,
  options: { activity?: boolean } = {},
): Promise<Session | null> {
  if (!auth) return null;
  try {
    return await auth.store.get(
      cookies.get(cookieName(SESSION_COOKIE, secure)),
      options,
    );
  } catch (e) {
    // The shared store is unreachable: show the reader as signed out, but delete nothing.
    console.error("session lookup failed:", e instanceof Error ? e.message : e);
    return null;
  }
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
