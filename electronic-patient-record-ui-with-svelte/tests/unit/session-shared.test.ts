import { randomBytes } from "node:crypto";
import { describe, expect, it } from "vitest";
import { backendFrom } from "../../src/lib/auth-instance";
import { RedisBackend, type RedisLike } from "../../src/lib/session-backends";
import { open, parseKey, seal, storageKey } from "../../src/lib/session-crypto";
import { LOGIN_TTL_SECONDS, SessionStore } from "../../src/lib/session";

/** An in-memory stand-in for Redis that honours EX against a fake clock. */
function fakeRedis(clock: { t: number }) {
  const data = new Map<string, { value: string; expires: number }>();
  const redis: RedisLike = {
    async get(key) {
      const e = data.get(key);
      if (!e || e.expires <= clock.t) return null;
      return e.value;
    },
    async set(key, value, { EX }) {
      data.set(key, { value, expires: clock.t + EX });
      return "OK";
    },
    async getDel(key) {
      // Atomic, like Redis GETDEL: no await between the read and the delete.
      const e = data.get(key);
      data.delete(key);
      return !e || e.expires <= clock.t ? null : e.value;
    },
    async del(key) {
      return data.delete(key) ? 1 : 0;
    },
  };
  return { redis, data };
}

const init = {
  accessToken: "ACCESS-TOKEN-PLAINTEXT",
  refreshToken: "REFRESH-TOKEN-PLAINTEXT",
  idToken: "ID-TOKEN-PLAINTEXT",
  accessExpiresAt: 9999999999,
  subject: "sub-1",
  name: "Dr Example",
};

function replicas(idle = 100, absolute = 1000) {
  const clock = { t: 1000 };
  const { redis, data } = fakeRedis(clock);
  const key = randomBytes(32);
  const make = () =>
    new SessionStore(
      idle,
      absolute,
      () => clock.t,
      new RedisBackend(redis, key),
    );
  return { clock, data, a: make(), b: make(), key };
}

describe("session encryption", () => {
  const key = randomBytes(32);

  it("round-trips and never contains the plaintext", () => {
    const sealed = seal(key, "k1", "REFRESH-TOKEN-PLAINTEXT");
    expect(sealed).not.toContain("REFRESH");
    expect(open(key, "k1", sealed)).toBe("REFRESH-TOKEN-PLAINTEXT");
  });

  it("refuses a tampered record, another storage key, another key and junk", () => {
    const sealed = seal(key, "k1", "secret");
    const [iv, tag, body] = sealed.split(".");
    const flipped = `${iv}.${tag}.${body!.slice(0, -2)}${body!.endsWith("AA") ? "BB" : "AA"}`;
    expect(open(key, "k1", flipped)).toBeNull();
    expect(open(key, "k2", sealed)).toBeNull(); // moved under another key
    expect(open(randomBytes(32), "k1", sealed)).toBeNull();
    for (const junk of ["", "a.b", "a.b.c.d", "...", "x.y.z"])
      expect(open(key, "k1", junk)).toBeNull();
  });

  it("seals the same text differently each time", () => {
    expect(seal(key, "k", "same")).not.toBe(seal(key, "k", "same"));
  });

  it("takes exactly 32 bytes, in base64 or base64url", () => {
    const raw = randomBytes(32);
    expect(parseKey(raw.toString("base64")).equals(raw)).toBe(true);
    expect(parseKey(raw.toString("base64url")).equals(raw)).toBe(true);
    expect(() => parseKey(randomBytes(16).toString("base64"))).toThrow(
      /32 bytes/,
    );
    expect(() => parseKey("")).toThrow();
  });

  it("derives a storage key that is a hash, not the id", () => {
    const k = storageKey("p:", "the-cookie-value");
    expect(k).toMatch(/^p:[0-9a-f]{64}$/);
    expect(k).not.toContain("the-cookie-value");
  });
});

describe("a session store shared by two replicas", () => {
  it("a session made on one replica is found, and kept alive, on the other", async () => {
    const { clock, a, b } = replicas();
    const made = await a.create(init);
    clock.t += 60;
    expect((await b.get(made.id))?.subject).toBe("sub-1"); // counts as activity
    clock.t += 60; // 120 s since creation, but only 60 since the other replica saw it
    expect(await a.get(made.id)).not.toBeNull();
  });

  it("ends an idle session on every replica, and a background look does not extend it", async () => {
    const { clock, a, b } = replicas(100, 1000);
    const { id } = await a.create(init);
    clock.t += 90;
    expect(await b.get(id, { activity: false })).not.toBeNull();
    clock.t += 20; // 110 s since the last real activity
    expect(await a.get(id)).toBeNull();
    expect(await b.get(id)).toBeNull();
  });

  it("ends a session at the absolute limit however active", async () => {
    const { clock, a, b } = replicas(100, 250);
    const { id } = await a.create(init);
    for (let i = 0; i < 4; i++) {
      clock.t += 60;
      const side = i % 2 ? a : b;
      const live = await side.get(id);
      if (clock.t - 1000 <= 250) expect(live).not.toBeNull();
      else expect(live).toBeNull();
    }
  });

  it("sign-out on one replica signs out the other", async () => {
    const { a, b } = replicas();
    const { id } = await a.create(init);
    expect((await b.delete(id))?.subject).toBe("sub-1");
    expect(await a.get(id)).toBeNull();
  });

  it("a login attempt begun on one replica is single-use across both", async () => {
    const { a, b } = replicas();
    const id = await a.beginLogin({
      state: "s",
      nonce: "n",
      verifier: "v",
      returnTo: "/",
    });
    const [x, y] = await Promise.all([b.takeLogin(id), a.takeLogin(id)]);
    expect([x, y].filter((r) => r !== null)).toHaveLength(1);
    expect(await a.takeLogin(id)).toBeNull();
  });

  it("an attempt expires", async () => {
    const { clock, a, b } = replicas();
    const id = await a.beginLogin({
      state: "s",
      nonce: "n",
      verifier: "v",
      returnTo: "/",
    });
    clock.t += LOGIN_TTL_SECONDS + 1;
    expect(await b.takeLogin(id)).toBeNull();
  });

  it("a refreshed token written by one replica is read by the other", async () => {
    const { a, b } = replicas();
    const made = await a.create(init);
    const mine = (await a.get(made.id))!;
    mine.accessToken = "ROTATED";
    await a.save(mine);
    expect((await b.get(made.id))?.accessToken).toBe("ROTATED");
  });

  it("stores neither a token nor the cookie value in the clear", async () => {
    const { data, a } = replicas();
    const made = await a.create(init);
    await a.beginLogin({
      state: "STATE-PLAINTEXT",
      nonce: "n",
      verifier: "VERIFIER-PLAINTEXT",
      returnTo: "/",
    });
    expect(data.size).toBe(2);
    for (const [k, e] of data) {
      for (const secret of [
        made.id,
        "ACCESS-TOKEN",
        "REFRESH-TOKEN",
        "ID-TOKEN",
        "STATE-PLAINTEXT",
        "VERIFIER-PLAINTEXT",
        "sub-1",
      ]) {
        expect(k).not.toContain(secret);
        expect(e.value).not.toContain(secret);
      }
    }
  });

  it("a replica with another key cannot read the session", async () => {
    const { clock, data, a } = replicas();
    const made = await a.create(init);
    const other = new SessionStore(
      100,
      1000,
      () => clock.t,
      new RedisBackend(fakeRedisOver(data, clock), randomBytes(32)),
    );
    expect(await other.get(made.id)).toBeNull();
  });

  it("does not count sessions, so the cap is Redis memory, not this process", async () => {
    const { a } = replicas();
    expect(await a.size()).toBeNull();
  });
});

function fakeRedisOver(
  data: Map<string, { value: string; expires: number }>,
  clock: { t: number },
): RedisLike {
  const redis: RedisLike = {
    async get(k) {
      const e = data.get(k);
      return !e || e.expires <= clock.t ? null : e.value;
    },
    async set(k, v, { EX }) {
      data.set(k, { value: v, expires: clock.t + EX });
      return "OK";
    },
    async getDel(k) {
      const e = data.get(k);
      data.delete(k);
      return !e || e.expires <= clock.t ? null : e.value;
    },
    async del(k) {
      return data.delete(k) ? 1 : 0;
    },
  };
  return redis;
}

describe("choosing the backend", () => {
  const goodKey = randomBytes(32).toString("base64");
  it("is memory when no Redis is set", () => {
    expect(backendFrom("", "").constructor.name).toBe("MemoryBackend");
  });
  it("stops startup on half a setting", () => {
    expect(() => backendFrom("redis://127.0.0.1:1", "")).toThrow(
      /needs EPR_SESSION_ENCRYPTION_KEY/,
    );
    expect(() => backendFrom("", goodKey)).toThrow(
      /without EPR_SESSION_REDIS_URL/,
    );
    expect(() => backendFrom("redis://127.0.0.1:1", "too-short")).toThrow(
      /32 bytes/,
    );
  });
});
