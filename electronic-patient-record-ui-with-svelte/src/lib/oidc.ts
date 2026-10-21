// OpenID Connect helpers for the backend-for-frontend sign-in (authorization
// code flow with PKCE). Pure functions only, so they can be unit-tested; the
// network calls live in `auth-server.ts`.

export interface OidcConfig {
  /** The realm URL, e.g. `http://127.0.0.1:8081/realms/epr`. No trailing slash. */
  issuer: string;
  clientId: string;
  /** Where the browser reaches this UI; the redirect URI is `<publicUrl>/auth/callback`. */
  publicUrl: string;
  idleSeconds: number;
  absoluteSeconds: number;
}

const LOOPBACK = new Set(["localhost", "127.0.0.1", "[::1]"]);

/** Plain `http://` is allowed only for loopback, as in the API's Keycloak guard. */
export function isSecureOrLoopback(url: URL): boolean {
  return (
    url.protocol === "https:" ||
    (url.protocol === "http:" && LOOPBACK.has(url.hostname))
  );
}

/**
 * Builds the config from raw environment values. Returns null when sign-in is
 * not configured (no issuer); throws when it is configured but unsafe, so a
 * typo never silently turns sign-in off.
 */
export function loadOidcConfig(raw: {
  issuer: string;
  clientId: string;
  publicUrl: string;
  idleSeconds: string;
  absoluteSeconds: string;
}): OidcConfig | null {
  if (!raw.issuer) return null;
  const issuer = new URL(raw.issuer);
  if (!isSecureOrLoopback(issuer))
    throw new Error("EPR_OIDC_ISSUER must be https unless it is loopback");
  if (!raw.publicUrl)
    throw new Error("EPR_PUBLIC_URL is required when EPR_OIDC_ISSUER is set");
  const publicUrl = new URL(raw.publicUrl);
  if (!isSecureOrLoopback(publicUrl))
    throw new Error("EPR_PUBLIC_URL must be https unless it is loopback");
  const idleSeconds = positive(
    raw.idleSeconds,
    900,
    "EPR_SESSION_IDLE_SECONDS",
  );
  const absoluteSeconds = positive(
    raw.absoluteSeconds,
    8 * 3600,
    "EPR_SESSION_ABSOLUTE_SECONDS",
  );
  return {
    issuer: raw.issuer.replace(/\/+$/, ""),
    clientId: raw.clientId || "epr-web",
    publicUrl: publicUrl.origin,
    idleSeconds,
    absoluteSeconds,
  };
}

function positive(value: string, fallback: number, name: string): number {
  if (!value) return fallback;
  const n = Number(value);
  if (!Number.isInteger(n) || n <= 0)
    throw new Error(`${name} must be a positive integer`);
  return n;
}

export function base64url(bytes: Uint8Array): string {
  return Buffer.from(bytes).toString("base64url");
}

export function randomToken(bytes = 32): string {
  return base64url(crypto.getRandomValues(new Uint8Array(bytes)));
}

/** RFC 7636 S256: BASE64URL(SHA256(verifier)). */
export async function challengeFor(verifier: string): Promise<string> {
  const digest = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(verifier),
  );
  return base64url(new Uint8Array(digest));
}

export interface Endpoints {
  issuer: string;
  authorization_endpoint: string;
  token_endpoint: string;
  end_session_endpoint?: string;
}

export function authorizeUrl(
  cfg: OidcConfig,
  endpoints: Endpoints,
  p: { state: string; nonce: string; challenge: string },
): string {
  const u = new URL(endpoints.authorization_endpoint);
  u.search = new URLSearchParams({
    response_type: "code",
    client_id: cfg.clientId,
    redirect_uri: `${cfg.publicUrl}/auth/callback`,
    scope: "openid profile email",
    state: p.state,
    nonce: p.nonce,
    code_challenge: p.challenge,
    code_challenge_method: "S256",
  }).toString();
  return u.toString();
}

/**
 * Accepts only a same-site absolute path, so a crafted `returnTo` cannot send
 * the reader to another site after sign-in. Anything else becomes `/`.
 */
export function safeReturnTo(value: string | null | undefined): string {
  if (
    !value ||
    !value.startsWith("/") ||
    value.startsWith("//") ||
    value.includes("\\")
  )
    return "/";
  if (/[\u0000-\u001f]/.test(value)) return "/";
  return value;
}

/** Decodes a JWT payload without verifying it (see the note in `auth-server.ts`). */
export function decodeJwtPayload(jwt: string): Record<string, unknown> | null {
  const parts = jwt.split(".");
  if (parts.length !== 3) return null;
  try {
    const value: unknown = JSON.parse(
      Buffer.from(parts[1]!, "base64url").toString("utf8"),
    );
    return value && typeof value === "object" && !Array.isArray(value)
      ? (value as Record<string, unknown>)
      : null;
  } catch {
    return null;
  }
}

/** Checks the ID token claims that bind it to this login attempt. Returns a reason on failure. */
export function checkIdToken(
  claims: Record<string, unknown> | null,
  expected: { issuer: string; clientId: string; nonce: string },
): string | null {
  if (!claims) return "id token is not a JWT";
  if (claims.iss !== expected.issuer) return "id token issuer mismatch";
  const aud = claims.aud;
  const audiences = Array.isArray(aud) ? aud : [aud];
  if (!audiences.includes(expected.clientId))
    return "id token audience mismatch";
  if (claims.nonce !== expected.nonce) return "id token nonce mismatch";
  if (typeof claims.sub !== "string" || claims.sub === "")
    return "id token has no subject";
  return null;
}
