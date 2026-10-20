import type { Handle } from "@sveltejs/kit/hooks";
import { redirect } from "@sveltejs/kit";
import {
  DEFAULT_LOCALE,
  isRtl,
  isUnlocalized,
  localeFromAcceptLanguage,
  localizePath,
  normalizeLocale,
  splitPath,
} from "#lib/locales.js";
import { applySecurityHeaders, isHttps } from "#lib/security-headers.js";

/** Remembers the reader's locale, to choose one for a bare, unprefixed URL. */
export const LOCALE_COOKIE = "locale";

// BFF entry point. The browser holds no token. Sign-in and the session cookie
// are not built yet (tasks SE-7, SF-4); until then the API runs with login off
// in development only.
export const handle: Handle = async (input) => {
  const response = await localeHandle(input);
  const { event } = input;
  try {
    applySecurityHeaders(
      response.headers,
      isHttps(event.url, event.request.headers.get("x-forwarded-proto")),
    );
  } catch {
    // Immutable headers (a redirect response from `fetch`): leave as is.
  }
  return response;
};

const localeHandle: Handle = async ({ event, resolve }) => {
  const { pathname, search } = event.url;
  if (!isUnlocalized(pathname)) {
    const { locale, rest } = splitPath(pathname);
    const reading = event.request.method === "GET" || event.request.method === "HEAD";
    if (!locale && reading) {
      const to =
        normalizeLocale(event.cookies.get(LOCALE_COOKIE)) ??
        localeFromAcceptLanguage(event.request.headers.get("accept-language")) ??
        DEFAULT_LOCALE;
      redirect(302, `${localizePath(rest, to)}${search}`);
    }
    if (locale) {
      event.locals.locale = locale;
      if (event.cookies.get(LOCALE_COOKIE) !== locale) {
        event.cookies.set(LOCALE_COOKIE, locale, {
          path: "/",
          maxAge: 60 * 60 * 24 * 365,
          sameSite: "lax",
          httpOnly: false,
          secure: event.url.protocol === "https:",
        });
      }
    }
  }
  const locale = event.locals.locale ?? DEFAULT_LOCALE;
  return resolve(event, {
    transformPageChunk: ({ html }) =>
      html
        .replace('lang="%lang%"', `lang="${locale}"`)
        .replace('dir="%dir%"', `dir="${isRtl(locale) ? "rtl" : "ltr"}"`),
  });
};
