import { describe, it, expect } from "vitest";
import type { Auth } from "../../src/lib/auth";
import { AuthUnavailableError } from "../../src/lib/auth";
import { createProxyHandler } from "../../src/lib/proxy-handler";

const PUBLIC = "http://127.0.0.1:3000";

function setup(opts: {
  token?: () => Promise<string | null>;
  fetchFn?: typeof fetch;
  loginOff?: boolean;
  timeoutMs?: number;
}) {
  const seen: { target: string; headers: Record<string, string> }[] = [];
  const deleted: string[] = [];
  const fetchFn =
    opts.fetchFn ??
    ((async (target: string, init: RequestInit) => {
      seen.push({ target, headers: init.headers as Record<string, string> });
      return new Response('{"ok":true}', {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    }) as unknown as typeof fetch);
  const auth = {
    config: { publicUrl: PUBLIC },
    store: { get: (id: string | undefined) => (id ? { id } : null) },
    accessToken: opts.token ?? (async () => "tok"),
  } as unknown as Auth;
  const handler = createProxyHandler({
    apiUrl: "http://api.test",
    getAuth: () => (opts.loginOff ? null : auth),
    cookieName: (base) => base,
    sessionCookie: "epr_session",
    sameOrigin: (r, origin) => r.headers.get("origin") === origin,
    forwardedProto: () => null,
    fetchFn,
    timeoutMs: opts.timeoutMs,
  });
  const call = (
    path: string,
    init: RequestInit = {},
    cookie: string | undefined = "sid",
  ) =>
    handler({
      params: { path },
      url: new URL(`${PUBLIC}/api/proxy/${path}`),
      request: new Request(`${PUBLIC}/api/proxy/${path}`, init),
      cookies: {
        get: () => cookie,
        delete: (name: string) => deleted.push(name),
      },
    } as unknown as Parameters<typeof handler>[0]);
  return { call, seen, deleted };
}

describe("proxy handler answers", () => {
  it("pass on where a cut export continues, and nothing else of the API's headers", async () => {
    const t = setup({
      fetchFn: (async () =>
        new Response("a,b\n", {
          status: 200,
          headers: { "content-type": "text/csv", "x-next-before": "41", "x-secret": "no" },
        })) as unknown as typeof fetch,
    });
    const res = await t.call("api/me/patients/x/access-log?format=csv");
    expect(res.headers.get("x-next-before")).toBe("41");
    expect(res.headers.get("x-secret")).toBeNull();
  });
});

describe("proxy handler", () => {
  it("forwards the token, a known purpose and the API version header", async () => {
    const t = setup({});
    const res = await t.call("api/patients", {
      headers: {
        "x-purpose-of-use": "treatment",
        "accepts-version": "2026-10-23",
      },
    });
    expect(res.status).toBe(200);
    expect(t.seen[0].target).toBe("http://api.test/api/patients");
    expect(t.seen[0].headers).toMatchObject({
      authorization: "Bearer tok",
      "x-purpose-of-use": "treatment",
      "accepts-version": "2026-10-23",
    });
  });

  it("drops a malformed version header and an unknown purpose", async () => {
    const t = setup({});
    await t.call("api/patients", {
      headers: { "x-purpose-of-use": "snooping", "accepts-version": "a b" },
    });
    expect(t.seen[0].headers["accepts-version"]).toBeUndefined();
    expect(t.seen[0].headers["x-purpose-of-use"]).toBeUndefined();
  });

  it("answers 502 JSON when the API is unreachable", async () => {
    const t = setup({
      fetchFn: (async () => {
        throw new TypeError("fetch failed");
      }) as unknown as typeof fetch,
    });
    const res = await t.call("api/patients");
    expect(res.status).toBe(502);
    expect(res.headers.get("content-type")).toContain("application/json");
    expect(await res.json()).toEqual({ error: "the service is unreachable" });
  });

  it("answers 504 JSON when the API is too slow, using a real abort signal", async () => {
    const t = setup({
      timeoutMs: 20,
      fetchFn: ((_: string, init: RequestInit) =>
        new Promise((_resolve, reject) => {
          init.signal!.addEventListener("abort", () =>
            reject(init.signal!.reason),
          );
        })) as unknown as typeof fetch,
    });
    const res = await t.call("api/patients");
    expect(res.status).toBe(504);
    expect(await res.json()).toEqual({ error: "the service timed out" });
  });

  it("answers 503 and keeps the cookie when the sign-in provider is unavailable", async () => {
    const t = setup({
      token: async () => {
        throw new AuthUnavailableError("down");
      },
    });
    const res = await t.call("api/patients");
    expect(res.status).toBe(503);
    expect(res.headers.get("retry-after")).toBe("5");
    expect(t.deleted).toEqual([]);
    expect(t.seen).toEqual([]);
  });

  it("answers 401 and clears the cookie when there is no valid session", async () => {
    const t = setup({ token: async () => null });
    const res = await t.call("api/patients");
    expect(res.status).toBe(401);
    expect(t.deleted).toEqual(["epr_session"]);
  });

  it("refuses a cross-site write, and a path outside /api", async () => {
    const t = setup({});
    await expect(
      t.call("api/patients", {
        method: "POST",
        body: "{}",
        headers: { origin: "http://evil.test" },
      }),
    ).rejects.toMatchObject({ status: 403 });
    await expect(t.call("fhir/R4/Patient")).rejects.toMatchObject({
      status: 404,
    });
  });

  it("forwards without a token when sign-in is off", async () => {
    const t = setup({ loginOff: true });
    const res = await t.call("api/patients");
    expect(res.status).toBe(200);
    expect(t.seen[0].headers.authorization).toBeUndefined();
  });
});
