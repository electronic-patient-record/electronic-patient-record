// The BFF proxy handler, with its dependencies passed in so it can be unit-tested without a server.
// The route file `routes/api/proxy/[...path]/+server.ts` wires the real ones.
import { error, json, type RequestHandler } from "@sveltejs/kit";
import { AuthUnavailableError, type Auth } from "./auth.js";
import {
  BACKGROUND_HEADER,
  isBackground,
  proxyTarget,
  purposeHeader,
  versionHeader,
} from "./proxy.js";
import { isHttps } from "./security-headers.js";

/** How long the API may take to answer before the proxy gives up with a 504. */
export const UPSTREAM_TIMEOUT_MS = 30_000;

export interface ProxyDeps {
  apiUrl: string;
  getAuth: () => Auth | null;
  cookieName: (base: string, secure: boolean) => string;
  sessionCookie: string;
  sameOrigin: (request: Request, publicUrl: string) => boolean;
  forwardedProto: (request: Request) => string | null;
  fetchFn?: typeof fetch;
  timeoutMs?: number;
}

export function createProxyHandler(deps: ProxyDeps): RequestHandler {
  const fetchFn = deps.fetchFn ?? fetch;
  return async ({ params, url, request, cookies }) => {
    const target = proxyTarget(deps.apiUrl, params.path ?? "", url.search);
    if (!target) error(404, "not found");

    const headers: Record<string, string> = {
      "content-type": request.headers.get("content-type") ?? "application/json",
    };
    const purpose = purposeHeader(request.headers.get("x-purpose-of-use"));
    if (purpose) headers["x-purpose-of-use"] = purpose;
    // API versions are chosen by this header, never by the path (spec/09-api-surface.md).
    const version = versionHeader(request.headers.get("accepts-version"));
    if (version) headers["accepts-version"] = version;
    const auth = deps.getAuth();
    if (auth) {
      if (
        request.method !== "GET" &&
        !deps.sameOrigin(request, auth.config.publicUrl)
      ) {
        error(403, "cross-site request refused");
      }
      const secure = isHttps(url, deps.forwardedProto(request));
      const name = deps.cookieName(deps.sessionCookie, secure);
      const session = auth.store.get(cookies.get(name), {
        activity: !isBackground(request.headers.get(BACKGROUND_HEADER)),
      });
      let token: string | null;
      try {
        token = session ? await auth.accessToken(session) : null;
      } catch (e) {
        if (!(e instanceof AuthUnavailableError)) throw e;
        // The provider is down, not the reader signed out: keep the session and the cookie.
        return json(
          { error: "sign-in service unavailable, try again" },
          { status: 503, headers: { "retry-after": "5" } },
        );
      }
      if (!token) {
        cookies.delete(name, { path: "/" });
        return json({ error: "sign in required" }, { status: 401 });
      }
      headers.authorization = `Bearer ${token}`;
    }

    let upstream: Response;
    try {
      upstream = await fetchFn(target, {
        method: request.method,
        headers,
        // Bytes, not text: a photo is binary and text decoding would corrupt it.
        body:
          request.method === "GET" ? undefined : await request.arrayBuffer(),
        signal: AbortSignal.timeout(deps.timeoutMs ?? UPSTREAM_TIMEOUT_MS),
      });
    } catch (e) {
      const timedOut = e instanceof Error && e.name === "TimeoutError";
      return json(
        {
          error: timedOut
            ? "the service timed out"
            : "the service is unreachable",
        },
        { status: timedOut ? 504 : 502 },
      );
    }
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
            // Where the next page or the rest of a cut export starts (the access log's CSV).
            "x-next-before",
          ]
            .map((h) => [h, upstream.headers.get(h)] as const)
            .filter(([, v]) => v !== null) as [string, string][],
        ),
      },
    });
  };
}
