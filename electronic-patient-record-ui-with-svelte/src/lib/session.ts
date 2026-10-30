// Server-side session store for the backend-for-frontend. The browser holds one
// opaque, random id in an httpOnly cookie; the tokens never leave the server.
// This file holds the policy (idle and absolute expiry, fresh ids, single-use login
// attempts); where records live is a backend (session-backends.ts): in this process's memory
// by default (a restart signs everyone out and replicas do not share it), or in Redis,
// encrypted, when several UI replicas must share sign-ins (OS-8).
import { MemoryBackend, type SessionBackend } from "./session-backends.js";
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
  constructor(
    private idleSeconds: number,
    private absoluteSeconds: number,
    private now: () => number = () => Math.floor(Date.now() / 1000),
    private backend: SessionBackend = new MemoryBackend(),
  ) {}

  private live = (s: Session): boolean => {
    const now = this.now();
    return (
      now - s.lastSeen <= this.idleSeconds &&
      now - s.createdAt <= this.absoluteSeconds
    );
  };

  /** Seconds a record may sit unwritten: until it would be idle or past its absolute limit. */
  private ttl(s: Session): number {
    const now = this.now();
    return Math.min(
      this.idleSeconds - (now - s.lastSeen),
      this.absoluteSeconds - (now - s.createdAt),
    );
  }

  async create(
    init: Omit<Session, "id" | "createdAt" | "lastSeen">,
  ): Promise<Session> {
    this.backend.sweep(this.live, this.attemptLive);
    const count = await this.backend.countSessions();
    if (count !== null && count >= MAX_SESSIONS) {
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
    await this.backend.putSession(session, this.ttl(session));
    return session;
  }

  /**
   * Returns the live session, or null. The request counts as activity unless `activity` is false
   * (a background look, which must not keep an unattended workstation signed in).
   */
  async get(
    id: string | undefined,
    { activity = true }: { activity?: boolean } = {},
  ): Promise<Session | null> {
    if (!id) return null;
    const session = await this.backend.getSession(id);
    if (!session) return null;
    if (!this.live(session)) {
      await this.backend.deleteSession(id);
      return null;
    }
    if (activity) {
      session.lastSeen = this.now();
      await this.backend.putSession(session, this.ttl(session));
    }
    return session;
  }

  /** Writes a session this process changed (a refreshed token), keeping its expiry. */
  async save(session: Session): Promise<void> {
    await this.backend.putSession(session, this.ttl(session));
  }

  async delete(id: string | undefined): Promise<Session | null> {
    if (!id) return null;
    return this.backend.deleteSession(id);
  }

  async size(): Promise<number | null> {
    return this.backend.countSessions();
  }

  async beginLogin(attempt: Omit<LoginAttempt, "createdAt">): Promise<string> {
    this.backend.sweep(this.live, this.attemptLive);
    if (
      this.backend instanceof MemoryBackend &&
      this.backend.attemptCount() >= MAX_SESSIONS
    )
      throw new Error("too many sign-in attempts");
    const id = randomToken();
    await this.backend.putAttempt(
      id,
      { ...attempt, createdAt: this.now() },
      LOGIN_TTL_SECONDS,
    );
    return id;
  }

  /** A login attempt is single-use: taking it removes it, whichever replica takes it. */
  async takeLogin(id: string | undefined): Promise<LoginAttempt | null> {
    if (!id) return null;
    const attempt = await this.backend.takeAttempt(id);
    if (!attempt || !this.attemptLive(attempt)) return null;
    return attempt;
  }

  private attemptLive = (a: LoginAttempt): boolean =>
    this.now() - a.createdAt <= LOGIN_TTL_SECONDS;
}
