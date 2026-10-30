// Where sessions live (OS-8). `SessionStore` (session.ts) holds the policy: idle and absolute
// expiry, fresh ids, single-use login attempts. A backend only stores and fetches.
import { open, seal, storageKey } from "./session-crypto.js";
import type { LoginAttempt, Session } from "./session.js";

export interface SessionBackend {
  getSession(id: string): Promise<Session | null>;
  /** Stores or replaces. `ttlSeconds` is how long the record may live without another write. */
  putSession(session: Session, ttlSeconds: number): Promise<void>;
  deleteSession(id: string): Promise<Session | null>;
  /** The number of live sessions, or null when the backend does not count them. */
  countSessions(): Promise<number | null>;
  putAttempt(
    id: string,
    attempt: LoginAttempt,
    ttlSeconds: number,
  ): Promise<void>;
  /** Atomic: of several callers (several replicas) exactly one gets the attempt. */
  takeAttempt(id: string): Promise<LoginAttempt | null>;
  /** Drop what has expired (memory only). */
  sweep(
    isLive: (s: Session) => boolean,
    attemptLive: (a: LoginAttempt) => boolean,
  ): void;
}

/** In this process only: a restart signs everyone out, and replicas do not share it. */
export class MemoryBackend implements SessionBackend {
  private sessions = new Map<string, Session>();
  private attempts = new Map<string, LoginAttempt>();

  async getSession(id: string) {
    return this.sessions.get(id) ?? null;
  }
  async putSession(session: Session) {
    this.sessions.set(session.id, session);
  }
  async deleteSession(id: string) {
    const s = this.sessions.get(id) ?? null;
    this.sessions.delete(id);
    return s;
  }
  async countSessions() {
    return this.sessions.size;
  }
  async putAttempt(id: string, attempt: LoginAttempt) {
    this.attempts.set(id, attempt);
  }
  async takeAttempt(id: string) {
    const a = this.attempts.get(id) ?? null;
    this.attempts.delete(id);
    return a;
  }
  attemptCount(): number {
    return this.attempts.size;
  }
  sweep(
    isLive: (s: Session) => boolean,
    attemptLive: (a: LoginAttempt) => boolean,
  ) {
    for (const [id, s] of this.sessions)
      if (!isLive(s)) this.sessions.delete(id);
    for (const [id, a] of this.attempts)
      if (!attemptLive(a)) this.attempts.delete(id);
  }
}

/** The few Redis commands used; node-redis's client satisfies it. */
export interface RedisLike {
  get(key: string): Promise<string | null>;
  set(key: string, value: string, options: { EX: number }): Promise<unknown>;
  getDel(key: string): Promise<string | null>;
  del(key: string): Promise<number>;
}

const SESSION_PREFIX = "epr:ui:session:";
const ATTEMPT_PREFIX = "epr:ui:login:";

/**
 * Shared by every UI replica. Records are encrypted (AES-256-GCM, bound to the key) and keyed by a
 * hash of the id, so the Redis holds no token and no value that works as a cookie. Expiry is the
 * Redis TTL. Sessions are not counted: the cap that protects memory is the deployer's `maxmemory`.
 */
export class RedisBackend implements SessionBackend {
  constructor(
    private redis: RedisLike,
    private key: Buffer,
  ) {}

  async getSession(id: string) {
    const k = storageKey(SESSION_PREFIX, id);
    const sealed = await this.redis.get(k);
    if (!sealed) return null;
    const text = open(this.key, k, sealed);
    return text ? (JSON.parse(text) as Session) : null;
  }
  async putSession(session: Session, ttlSeconds: number) {
    const k = storageKey(SESSION_PREFIX, session.id);
    await this.redis.set(k, seal(this.key, k, JSON.stringify(session)), {
      EX: Math.max(1, Math.ceil(ttlSeconds)),
    });
  }
  async deleteSession(id: string) {
    const k = storageKey(SESSION_PREFIX, id);
    const sealed = await this.redis.getDel(k);
    if (!sealed) return null;
    const text = open(this.key, k, sealed);
    return text ? (JSON.parse(text) as Session) : null;
  }
  async countSessions() {
    return null;
  }
  async putAttempt(id: string, attempt: LoginAttempt, ttlSeconds: number) {
    const k = storageKey(ATTEMPT_PREFIX, id);
    await this.redis.set(k, seal(this.key, k, JSON.stringify(attempt)), {
      EX: Math.max(1, Math.ceil(ttlSeconds)),
    });
  }
  async takeAttempt(id: string) {
    const k = storageKey(ATTEMPT_PREFIX, id);
    const sealed = await this.redis.getDel(k); // atomic: one replica wins
    if (!sealed) return null;
    const text = open(this.key, k, sealed);
    return text ? (JSON.parse(text) as LoginAttempt) : null;
  }
  sweep() {
    // Redis expires its own keys.
  }
}
