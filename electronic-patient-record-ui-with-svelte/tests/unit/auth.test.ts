import { describe, it, expect } from "vitest";
import { Auth, AuthError } from "../../src/lib/auth";
import { loadOidcConfig } from "../../src/lib/oidc";
import { SessionStore } from "../../src/lib/session";

const ISSUER = "http://127.0.0.1:8081/realms/epr";
const cfg = loadOidcConfig({
  issuer: ISSUER,
  clientId: "epr-web",
  publicUrl: "http://127.0.0.1:3000",
  idleSeconds: "900",
  absoluteSeconds: "28800",
})!;

const discovery = {
  issuer: ISSUER,
  authorization_endpoint: `${ISSUER}/protocol/openid-connect/auth`,
  token_endpoint: `${ISSUER}/protocol/openid-connect/token`,
  end_session_endpoint: `${ISSUER}/protocol/openid-connect/logout`,
};
const jwt = (claims: object) =>
  `h.${Buffer.from(JSON.stringify(claims)).toString("base64url")}.s`;
const reply = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });

/** A stub identity provider that records each request and answers from `handlers`. */
function idp(handlers: {
  token?: (form: URLSearchParams) => Response;
  discovery?: object;
}) {
  const calls: { url: string; form: URLSearchParams | null }[] = [];
  const fetchFn = (async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input);
    const form = init?.body instanceof URLSearchParams ? init.body : null;
    calls.push({ url, form });
    if (url.endsWith("/.well-known/openid-configuration"))
      return reply(handlers.discovery ?? discovery);
    if (url.endsWith("/token")) return handlers.token!(form!);
    if (url.endsWith("/logout")) return new Response(null, { status: 204 });
    return new Response("?", { status: 404 });
  }) as typeof fetch;
  return { fetchFn, calls };
}

function make(handlers: Parameters<typeof idp>[0], clock = { t: 1000 }) {
  const stub = idp(handlers);
  const store = new SessionStore(900, 28800, () => clock.t);
  return {
    auth: new Auth(cfg, store, stub.fetchFn, () => clock.t),
    store,
    clock,
    calls: stub.calls,
  };
}

async function signIn(m: ReturnType<typeof make>, tokenOverride?: object) {
  const { loginId, url } = await m.auth.startLogin("/en-001/patients");
  const q = new URL(url).searchParams;
  return {
    loginId,
    state: q.get("state")!,
    nonce: q.get("nonce")!,
    tokenOverride,
  };
}

const goodToken = (nonce: string, extra: object = {}) =>
  reply({
    access_token: "access-1",
    refresh_token: "refresh-1",
    id_token: jwt({
      iss: ISSUER,
      aud: "epr-web",
      nonce,
      sub: "user-1",
      name: "Dr Example",
    }),
    expires_in: 300,
    ...extra,
  });

describe("Auth sign-in", () => {
  it("completes a code exchange with the PKCE verifier and creates a session", async () => {
    let nonce = "";
    const m = make({ token: () => goodToken(nonce) });
    const a = await signIn(m);
    nonce = a.nonce;
    const { session, returnTo } = await m.auth.finishLogin(a.loginId, {
      code: "c",
      state: a.state,
      error: null,
    });
    expect(returnTo).toBe("/en-001/patients");
    expect(session.subject).toBe("user-1");
    expect(session.name).toBe("Dr Example");
    const form = m.calls.find((c) => c.url.endsWith("/token"))!.form!;
    expect(form.get("grant_type")).toBe("authorization_code");
    expect(form.get("code_verifier")!.length).toBeGreaterThanOrEqual(43);
    expect(form.get("redirect_uri")).toBe(
      "http://127.0.0.1:3000/auth/callback",
    );
    expect(m.store.get(session.id)).not.toBeNull();
  });

  it("refuses a state mismatch, a provider error, and a missing or reused login", async () => {
    const m = make({ token: () => goodToken("x") });
    const a = await signIn(m);
    await expect(
      m.auth.finishLogin(a.loginId, { code: "c", state: "other", error: null }),
    ).rejects.toThrow(AuthError);
    // the attempt was consumed by the failure: a retry with the right state is refused too
    await expect(
      m.auth.finishLogin(a.loginId, { code: "c", state: a.state, error: null }),
    ).rejects.toThrow(/no sign-in/);
    const b = await signIn(m);
    await expect(
      m.auth.finishLogin(b.loginId, {
        code: null,
        state: b.state,
        error: "access_denied",
      }),
    ).rejects.toThrow();
    await expect(
      m.auth.finishLogin(undefined, { code: "c", state: "s", error: null }),
    ).rejects.toThrow(/no sign-in/);
    expect(m.store.size()).toBe(0);
  });

  it("refuses an id token with the wrong nonce", async () => {
    const m = make({ token: () => goodToken("not-the-nonce") });
    const a = await signIn(m);
    await expect(
      m.auth.finishLogin(a.loginId, { code: "c", state: a.state, error: null }),
    ).rejects.toThrow(/nonce/);
    expect(m.store.size()).toBe(0);
  });

  it("refuses a discovery document for another issuer, or with an insecure endpoint", async () => {
    const other = make({
      discovery: { ...discovery, issuer: "http://127.0.0.1:8081/realms/other" },
    });
    await expect(other.auth.startLogin("/")).rejects.toThrow(/issuer/);
    const insecure = make({
      discovery: {
        ...discovery,
        token_endpoint: "http://idp.example.org/token",
      },
    });
    await expect(insecure.auth.startLogin("/")).rejects.toThrow(/insecure/);
  });

  it("never lets a hostile returnTo through", async () => {
    const m = make({ token: () => goodToken("x") });
    const { loginId } = await m.auth.startLogin("//evil.example");
    expect(m.store.takeLogin(loginId)?.returnTo).toBe("/");
  });
});

describe("Auth tokens and sign-out", () => {
  async function signedIn(m: ReturnType<typeof make>, nonceBox: { n: string }) {
    const a = await signIn(m);
    nonceBox.n = a.nonce;
    return (
      await m.auth.finishLogin(a.loginId, {
        code: "c",
        state: a.state,
        error: null,
      })
    ).session;
  }

  it("uses a fresh token as is and refreshes one near expiry, once for concurrent calls", async () => {
    const box = { n: "" };
    let refreshes = 0;
    const m = make({
      token: (form) => {
        if (form.get("grant_type") === "refresh_token") {
          refreshes++;
          return reply({
            access_token: "access-2",
            refresh_token: "refresh-2",
            expires_in: 300,
          });
        }
        return goodToken(box.n);
      },
    });
    const session = await signedIn(m, box);
    expect(await m.auth.accessToken(session)).toBe("access-1");
    m.clock.t += 290; // 10 s left: inside the margin
    const [x, y] = await Promise.all([
      m.auth.accessToken(session),
      m.auth.accessToken(session),
    ]);
    expect([x, y]).toEqual(["access-2", "access-2"]);
    expect(refreshes).toBe(1);
    expect(session.refreshToken).toBe("refresh-2");
  });

  it("ends the session when the refresh is refused", async () => {
    const box = { n: "" };
    const m = make({
      token: (form) =>
        form.get("grant_type") === "refresh_token"
          ? reply({ error: "invalid_grant" }, 400)
          : goodToken(box.n),
    });
    const session = await signedIn(m, box);
    m.clock.t += 295;
    expect(await m.auth.accessToken(session)).toBeNull();
    expect(m.store.get(session.id)).toBeNull();
  });

  it("signs out locally first, and tells the provider with the refresh token", async () => {
    const box = { n: "" };
    const m = make({ token: () => goodToken(box.n) });
    const session = await signedIn(m, box);
    await m.auth.signOut(session.id);
    expect(m.store.get(session.id)).toBeNull();
    const logout = m.calls.find((c) => c.url.endsWith("/logout"))!;
    expect(logout.form!.get("refresh_token")).toBe("refresh-1");
    expect(logout.form!.get("client_id")).toBe("epr-web");
  });

  it("still signs out locally when the provider is unreachable", async () => {
    const box = { n: "" };
    const m = make({ token: () => goodToken(box.n) });
    const session = await signedIn(m, box);
    const down = new Auth(
      cfg,
      m.store,
      (async () => {
        throw new Error("down");
      }) as typeof fetch,
      () => m.clock.t,
    );
    await down.signOut(session.id);
    expect(m.store.get(session.id)).toBeNull();
  });
});
