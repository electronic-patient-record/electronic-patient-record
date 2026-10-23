// The sign-in flow itself: discovery, the authorization redirect, the code
// exchange, refresh and sign-out. It takes its config, store and `fetch` as
// arguments so tests can drive it with a stub identity provider; the SvelteKit
// wiring is in `auth-instance.ts`.
//
// The ID token is decoded, not signature-checked. That is allowed when it comes
// straight from the token endpoint over TLS (OIDC Core 3.1.3.7), and nothing
// here trusts it for access: the API verifies the access token on every call.
import {
  authorizeUrl,
  challengeFor,
  checkIdToken,
  decodeJwtPayload,
  isSecureOrLoopback,
  randomToken,
  safeReturnTo,
  type Endpoints,
  type OidcConfig,
} from "./oidc.js";
import type { Session, SessionStore } from "./session.js";

export class AuthError extends Error {
  constructor(
    message: string,
    /** The OAuth `error` code the provider answered with, if any (e.g. `invalid_grant`). */
    readonly code?: string,
  ) {
    super(message);
  }
}

/** The provider could not be reached or answered badly: the session is kept and the caller may retry. */
export class AuthUnavailableError extends Error {}

/** Refresh the access token when it has less than this long to live. */
export const REFRESH_MARGIN_SECONDS = 30;
const TIMEOUT_MS = 10_000;

export class Auth {
  private endpointsPromise: Promise<Endpoints> | null = null;
  private refreshing = new Map<string, Promise<boolean>>();

  constructor(
    readonly config: OidcConfig,
    readonly store: SessionStore,
    private fetchFn: typeof fetch = fetch,
    private now: () => number = () => Math.floor(Date.now() / 1000),
  ) {}

  private endpoints(): Promise<Endpoints> {
    this.endpointsPromise ??= this.discover().catch((e) => {
      this.endpointsPromise = null; // try again on the next request
      throw e;
    });
    return this.endpointsPromise;
  }

  private async discover(): Promise<Endpoints> {
    const res = await this.fetchFn(
      `${this.config.issuer}/.well-known/openid-configuration`,
      {
        signal: AbortSignal.timeout(TIMEOUT_MS),
      },
    );
    if (!res.ok) throw new AuthError(`discovery failed: ${res.status}`);
    const doc = (await res.json()) as Partial<Endpoints>;
    if (doc.issuer !== this.config.issuer)
      throw new AuthError("discovery issuer mismatch");
    for (const key of ["authorization_endpoint", "token_endpoint"] as const) {
      const value = doc[key];
      if (typeof value !== "string" || !isSecureOrLoopback(new URL(value))) {
        throw new AuthError(`discovery ${key} is missing or insecure`);
      }
    }
    return doc as Endpoints;
  }

  /** Starts a sign-in. Returns the id for the login cookie and the URL to send the browser to. */
  async startLogin(
    returnTo: string | null,
  ): Promise<{ loginId: string; url: string }> {
    const endpoints = await this.endpoints();
    const verifier = randomToken(48);
    const state = randomToken();
    const nonce = randomToken();
    const loginId = this.store.beginLogin({
      state,
      nonce,
      verifier,
      returnTo: safeReturnTo(returnTo),
    });
    const url = authorizeUrl(this.config, endpoints, {
      state,
      nonce,
      challenge: await challengeFor(verifier),
    });
    return { loginId, url };
  }

  /** Completes a sign-in from the callback. Returns the new session and where to send the reader. */
  async finishLogin(
    loginId: string | undefined,
    params: { code: string | null; state: string | null; error: string | null },
  ): Promise<{ session: Session; returnTo: string }> {
    const attempt = this.store.takeLogin(loginId); // single use, even on failure
    if (!attempt) throw new AuthError("no sign-in in progress");
    if (params.error)
      throw new AuthError("the identity provider refused the sign-in");
    if (!params.code || !params.state || params.state !== attempt.state)
      throw new AuthError("state mismatch");

    const endpoints = await this.endpoints();
    const token = await this.tokenRequest(endpoints, {
      grant_type: "authorization_code",
      code: params.code,
      redirect_uri: `${this.config.publicUrl}/auth/callback`,
      code_verifier: attempt.verifier,
    });
    const claims = decodeJwtPayload(token.id_token ?? "");
    const problem = checkIdToken(claims, {
      issuer: this.config.issuer,
      clientId: this.config.clientId,
      nonce: attempt.nonce,
    });
    if (problem) throw new AuthError(problem);

    const session = this.store.create({
      accessToken: token.access_token,
      refreshToken: token.refresh_token ?? null,
      idToken: token.id_token ?? null,
      accessExpiresAt: this.now() + token.expires_in,
      subject: String(claims!.sub),
      name: typeof claims!.name === "string" ? claims!.name : null,
    });
    return { session, returnTo: attempt.returnTo };
  }

  /**
   * The access token to send to the API, refreshed if it is about to expire.
   * Null means the session is over (and has been removed).
   */
  async accessToken(session: Session): Promise<string | null> {
    if (session.accessExpiresAt - this.now() > REFRESH_MARGIN_SECONDS)
      return session.accessToken;
    let pending = this.refreshing.get(session.id);
    if (!pending) {
      // One refresh at a time per session: refresh tokens may rotate.
      pending = this.refresh(session).finally(() =>
        this.refreshing.delete(session.id),
      );
      this.refreshing.set(session.id, pending);
    }
    return (await pending) ? session.accessToken : null;
  }

  /**
   * True when refreshed; false when the provider has refused the refresh token for good
   * (`invalid_grant`, or none held), in which case the session is deleted. Any other failure
   * (network, timeout, a 5xx, a malformed answer) throws `AuthUnavailableError` and keeps the
   * session, so a provider blip does not sign every clinician out.
   */
  private async refresh(session: Session): Promise<boolean> {
    if (!session.refreshToken) {
      this.store.delete(session.id);
      return false;
    }
    try {
      const token = await this.tokenRequest(await this.endpoints(), {
        grant_type: "refresh_token",
        refresh_token: session.refreshToken,
      });
      session.accessToken = token.access_token;
      session.refreshToken = token.refresh_token ?? session.refreshToken;
      session.accessExpiresAt = this.now() + token.expires_in;
      return true;
    } catch (e) {
      if (e instanceof AuthError && e.code === "invalid_grant") {
        this.store.delete(session.id);
        return false;
      }
      throw new AuthUnavailableError("sign-in service unavailable", {
        cause: e,
      });
    }
  }

  /**
   * Ends the session here at once, then asks the provider to end its own
   * (back-channel, so the browser is never redirected away). A provider
   * failure does not keep the reader signed in here.
   */
  async signOut(sessionId: string | undefined): Promise<void> {
    const session = this.store.delete(sessionId);
    if (!session?.refreshToken) return;
    try {
      const endpoints = await this.endpoints();
      if (!endpoints.end_session_endpoint) return;
      await this.fetchFn(endpoints.end_session_endpoint, {
        method: "POST",
        headers: { "content-type": "application/x-www-form-urlencoded" },
        body: new URLSearchParams({
          client_id: this.config.clientId,
          refresh_token: session.refreshToken,
        }),
        signal: AbortSignal.timeout(TIMEOUT_MS),
      });
    } catch {
      // Best effort: the local session is already gone.
    }
  }

  private async tokenRequest(
    endpoints: Endpoints,
    form: Record<string, string>,
  ): Promise<{
    access_token: string;
    refresh_token?: string;
    id_token?: string;
    expires_in: number;
  }> {
    const res = await this.fetchFn(endpoints.token_endpoint, {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({ client_id: this.config.clientId, ...form }),
      signal: AbortSignal.timeout(TIMEOUT_MS),
    });
    if (!res.ok) {
      let code: string | undefined;
      try {
        const err = (await res.json()) as { error?: unknown };
        if (typeof err.error === "string") code = err.error;
      } catch {
        // No JSON body: a gateway error page, say. Not a refusal of the grant.
      }
      throw new AuthError(`token endpoint returned ${res.status}`, code);
    }
    const body = (await res.json()) as Record<string, unknown>;
    if (
      typeof body.access_token !== "string" ||
      typeof body.expires_in !== "number"
    ) {
      throw new AuthError("token response is malformed");
    }
    return body as {
      access_token: string;
      refresh_token?: string;
      id_token?: string;
      expires_in: number;
    };
  }
}
