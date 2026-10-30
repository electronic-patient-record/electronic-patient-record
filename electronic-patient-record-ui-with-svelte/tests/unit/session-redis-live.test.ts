// Opt-in: runs the shared-store behaviour against a real Redis through node-redis.
//   EPR_TEST_REDIS_URL=redis://:password@127.0.0.1:6391/0 pnpm exec vitest run session-redis-live
// Skipped (not failed) when the variable is unset, so the ordinary suite needs no Redis.
import { randomBytes } from "node:crypto";
import { describe, expect, it } from "vitest";
import { backendFrom } from "../../src/lib/auth-instance";
import { SessionStore } from "../../src/lib/session";

const url = process.env.EPR_TEST_REDIS_URL;

describe.skipIf(!url)("session store on a real Redis", () => {
  const key = randomBytes(32).toString("base64");
  const init = {
    accessToken: "ACCESS-LIVE",
    refreshToken: "REFRESH-LIVE",
    idToken: null,
    accessExpiresAt: 9999999999,
    subject: "live-sub",
    name: null,
  };
  const replica = () =>
    new SessionStore(60, 600, undefined, backendFrom(url!, key));

  it("shares a session and a single-use attempt between two connections, and expires with a TTL", async () => {
    const a = replica();
    const b = replica();
    const made = await a.create(init);
    expect((await b.get(made.id))?.refreshToken).toBe("REFRESH-LIVE");

    const attempt = await a.beginLogin({
      state: "s",
      nonce: "n",
      verifier: "v",
      returnTo: "/",
    });
    const taken = await Promise.all([
      a.takeLogin(attempt),
      b.takeLogin(attempt),
    ]);
    expect(taken.filter((t) => t !== null)).toHaveLength(1);

    expect((await a.delete(made.id))?.subject).toBe("live-sub");
    expect(await b.get(made.id)).toBeNull();
  });

  it("lets a record expire by TTL", async () => {
    const short = new SessionStore(1, 600, undefined, backendFrom(url!, key));
    const made = await short.create(init);
    await new Promise((r) => setTimeout(r, 2200));
    expect(await short.get(made.id)).toBeNull();
  }, 10_000);
});
