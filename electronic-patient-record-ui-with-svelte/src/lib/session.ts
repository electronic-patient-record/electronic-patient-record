// Server-side session store for the backend-for-frontend. The browser holds one
// opaque, random id in an httpOnly cookie; the tokens never leave the server.
// This store is in process memory: a restart signs everyone out, and it cannot
// be shared by several UI replicas (tracked in tasks.md, SE-9 "Not built").
import { randomToken } from "./oidc.js";

export interface Session {
  id: string;
  accessToken: string;
  refreshToken: string | null;
  idToken: string | null;
  /** Epoch seconds at which the access token expires. */
  accessExpiresAt: number;
  subject: string;
  name: string | null;
  createdAt: number;
  lastSeen: number;
}

export interface LoginAttempt {
  state: string;
  nonce: string;
  verifier: string;
  returnTo: string;
  createdAt: number;
}

export const MAX_SESSIONS = 10_000;
export const LOGIN_TTL_SECONDS = 600;

export class SessionStore {
  private sessions = new Map<string, Session>();
  private attempts = new Map<string, LoginAttempt>();

  constructor(
    private idleSeconds: number,
    private absoluteSeconds: number,
    private now: () => number = () => Math.floor(Date.now() / 1000),
  ) {}

  create(init: Omit<Session, "id" | "createdAt" | "lastSeen">): Session {
    this.sweep();
    if (this.sessions.size >= MAX_SESSIONS) {
      // Refuse rather than evict someone mid-task.
      throw new Error("too many sessions");
    }
    const now = this.now();
    // A fresh id on every sign-in: no session fixation.
    const session: Session = {
      ...init,
      id: randomToken(),
      createdAt: now,
      lastSeen: now,
    };
    this.sessions.set(session.id, session);
    return session;
  }

  /**
   * Returns the live session, or null. The request counts as activity unless `activity` is false
   * (a background look, which must not keep an unattended workstation signed in).
   */
  get(
    id: string | undefined,
    { activity = true }: { activity?: boolean } = {},
  ): Session | null {
    if (!id) return null;
    const session = this.sessions.get(id);
    if (!session) return null;
    const now = this.now();
    if (
      now - session.lastSeen > this.idleSeconds ||
      now - session.createdAt > this.absoluteSeconds
    ) {
      this.sessions.delete(id);
      return null;
    }
    if (activity) session.lastSeen = now;
    return session;
  }

  delete(id: string | undefined): Session | null {
    if (!id) return null;
    const session = this.sessions.get(id) ?? null;
    this.sessions.delete(id);
    return session;
  }

  size(): number {
    return this.sessions.size;
  }

  beginLogin(attempt: Omit<LoginAttempt, "createdAt">): string {
    this.sweep();
    if (this.attempts.size >= MAX_SESSIONS)
      throw new Error("too many sign-in attempts");
    const id = randomToken();
    this.attempts.set(id, { ...attempt, createdAt: this.now() });
    return id;
  }

  /** A login attempt is single-use: taking it removes it. */
  takeLogin(id: string | undefined): LoginAttempt | null {
    if (!id) return null;
    const attempt = this.attempts.get(id) ?? null;
    this.attempts.delete(id);
    if (!attempt || this.now() - attempt.createdAt > LOGIN_TTL_SECONDS)
      return null;
    return attempt;
  }

  private sweep(): void {
    const now = this.now();
    for (const [id, s] of this.sessions) {
      if (
        now - s.lastSeen > this.idleSeconds ||
        now - s.createdAt > this.absoluteSeconds
      )
        this.sessions.delete(id);
    }
    for (const [id, a] of this.attempts) {
      if (now - a.createdAt > LOGIN_TTL_SECONDS) this.attempts.delete(id);
    }
  }
}
