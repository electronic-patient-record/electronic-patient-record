import { describe, it, expect } from "vitest";
import { SessionStore, LOGIN_TTL_SECONDS } from "../../src/lib/session";

const init = {
  accessToken: "a",
  refreshToken: "r",
  idToken: "i",
  accessExpiresAt: 1000,
  subject: "u",
  name: "U",
};

function store(idle = 100, abs = 1000) {
  const clock = { t: 0 };
  return { clock, s: new SessionStore(idle, abs, () => clock.t) };
}

describe("SessionStore", () => {
  it("issues a fresh unguessable id per session", async () => {
    const { s } = store();
    const a = await s.create(init);
    const b = await s.create(init);
    expect(a.id).not.toBe(b.id);
    expect(a.id.length).toBeGreaterThanOrEqual(43); // 256 bits, base64url
  });
  it("ends an idle session, but activity keeps it alive", async () => {
    const { s, clock } = store(100, 1000);
    const { id } = await s.create(init);
    clock.t = 90;
    expect(await s.get(id)).not.toBeNull();
    clock.t = 180; // 90 s since last activity
    expect(await s.get(id)).not.toBeNull();
    clock.t = 281; // 101 s idle
    expect(await s.get(id)).toBeNull();
    expect(await s.size()).toBe(0);
  });
  it("does not count a background request as activity (CQ-8)", async () => {
    const { s, clock } = store(100, 1000);
    const { id } = await s.create(init);
    // A page polling every 60 s with nobody at the keyboard: the session still ends on time.
    for (clock.t = 60; clock.t <= 60 * 5; clock.t += 60) {
      const live = await s.get(id, { activity: false });
      if (clock.t <= 100) expect(live).not.toBeNull();
      else expect(live).toBeNull();
    }
    expect(await s.size()).toBe(0);
  });
  it("ends a session at the absolute limit however active", async () => {
    const { s, clock } = store(100, 250);
    const { id } = await s.create(init);
    for (clock.t = 50; clock.t <= 250; clock.t += 50)
      expect(await s.get(id)).not.toBeNull();
    clock.t = 251;
    expect(await s.get(id)).toBeNull();
  });
  it("returns null for a missing, unknown or deleted id", async () => {
    const { s } = store();
    expect(await s.get(undefined)).toBeNull();
    expect(await s.get("nope")).toBeNull();
    const { id } = await s.create(init);
    s.delete(id);
    expect(await s.get(id)).toBeNull();
  });
  it("makes a login attempt single-use and short-lived", async () => {
    const { s, clock } = store();
    const attempt = { state: "s", nonce: "n", verifier: "v", returnTo: "/" };
    const id = await s.beginLogin(attempt);
    expect((await s.takeLogin(id))?.state).toBe("s");
    expect(await s.takeLogin(id)).toBeNull();
    const late = await s.beginLogin(attempt);
    clock.t = LOGIN_TTL_SECONDS + 1;
    expect(await s.takeLogin(late)).toBeNull();
  });
});
