import { describe, it, expect } from "vitest";
import {
  authorizeUrl,
  challengeFor,
  checkIdToken,
  decodeJwtPayload,
  loadOidcConfig,
  safeReturnTo,
} from "../../src/lib/oidc";

const raw = {
  issuer: "http://127.0.0.1:8081/realms/epr/",
  clientId: "",
  publicUrl: "http://127.0.0.1:3000/",
  idleSeconds: "",
  absoluteSeconds: "",
};

describe("loadOidcConfig", () => {
  it("is off without an issuer", () => {
    expect(loadOidcConfig({ ...raw, issuer: "" })).toBeNull();
  });
  it("applies defaults and normalises URLs", () => {
    expect(loadOidcConfig(raw)).toEqual({
      issuer: "http://127.0.0.1:8081/realms/epr",
      clientId: "epr-web",
      publicUrl: "http://127.0.0.1:3000",
      idleSeconds: 900,
      absoluteSeconds: 28800,
    });
  });
  it("refuses plain http to a remote issuer or public URL", () => {
    expect(() =>
      loadOidcConfig({ ...raw, issuer: "http://idp.example.org/realms/epr" }),
    ).toThrow(/https/);
    expect(() =>
      loadOidcConfig({ ...raw, publicUrl: "http://epr.example.org" }),
    ).toThrow(/https/);
  });
  it("requires the public URL and sane timeouts rather than silently defaulting", () => {
    expect(() => loadOidcConfig({ ...raw, publicUrl: "" })).toThrow(
      /EPR_PUBLIC_URL/,
    );
    expect(() => loadOidcConfig({ ...raw, idleSeconds: "abc" })).toThrow(
      /IDLE/,
    );
    expect(() => loadOidcConfig({ ...raw, absoluteSeconds: "0" })).toThrow(
      /ABSOLUTE/,
    );
  });
});

describe("PKCE", () => {
  it("matches the RFC 7636 appendix B example", async () => {
    expect(
      await challengeFor("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
    ).toBe("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
  });
  it("puts the S256 challenge, state, nonce and redirect URI in the authorize URL", () => {
    const cfg = loadOidcConfig(raw)!;
    const url = new URL(
      authorizeUrl(
        cfg,
        {
          issuer: cfg.issuer,
          authorization_endpoint: `${cfg.issuer}/protocol/openid-connect/auth`,
          token_endpoint: "x",
        },
        { state: "s", nonce: "n", challenge: "c" },
      ),
    );
    expect(url.searchParams.get("response_type")).toBe("code");
    expect(url.searchParams.get("code_challenge_method")).toBe("S256");
    expect(url.searchParams.get("code_challenge")).toBe("c");
    expect(url.searchParams.get("redirect_uri")).toBe(
      "http://127.0.0.1:3000/auth/callback",
    );
    expect(url.searchParams.get("client_id")).toBe("epr-web");
  });
});

describe("safeReturnTo", () => {
  it("keeps same-site paths", () => {
    expect(safeReturnTo("/en-001/patients?x=1")).toBe("/en-001/patients?x=1");
  });
  it("turns anything that could leave the site into /", () => {
    for (const v of [
      "//evil.example",
      "https://evil.example",
      "/\\evil",
      "evil",
      "",
      null,
      undefined,
      "/a\nb",
    ]) {
      expect(safeReturnTo(v as string)).toBe("/");
    }
  });
});

describe("id token checks", () => {
  const jwt = (claims: object) =>
    `h.${Buffer.from(JSON.stringify(claims)).toString("base64url")}.s`;
  const expected = { issuer: "i", clientId: "c", nonce: "n" };
  const good = { iss: "i", aud: "c", nonce: "n", sub: "u" };
  it("accepts matching claims, aud as string or array", () => {
    expect(checkIdToken(decodeJwtPayload(jwt(good)), expected)).toBeNull();
    expect(
      checkIdToken(
        decodeJwtPayload(jwt({ ...good, aud: ["x", "c"] })),
        expected,
      ),
    ).toBeNull();
  });
  it("refuses a wrong issuer, audience, nonce, missing subject or non-JWT", () => {
    expect(
      checkIdToken(decodeJwtPayload(jwt({ ...good, iss: "z" })), expected),
    ).toMatch(/issuer/);
    expect(
      checkIdToken(decodeJwtPayload(jwt({ ...good, aud: "z" })), expected),
    ).toMatch(/audience/);
    expect(
      checkIdToken(decodeJwtPayload(jwt({ ...good, nonce: "z" })), expected),
    ).toMatch(/nonce/);
    expect(
      checkIdToken(decodeJwtPayload(jwt({ ...good, sub: "" })), expected),
    ).toMatch(/subject/);
    expect(checkIdToken(decodeJwtPayload("nope"), expected)).toMatch(
      /not a JWT/,
    );
    expect(decodeJwtPayload("a.!!!.c")).toBeNull();
  });
});
