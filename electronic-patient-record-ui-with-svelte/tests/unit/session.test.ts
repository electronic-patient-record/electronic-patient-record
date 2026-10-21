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
  it("issues a fresh unguessable id per session", () => {
    const { s } = store();
    const a = s.create(init);
    const b = s.create(init);
    expect(a.id).not.toBe(b.id);
    expect(a.id.length).toBeGreaterThanOrEqual(43); // 256 bits, base64url
  });
  it("ends an idle session, but activity keeps it alive", () => {
    const { s, clock } = store(100, 1000);
    const { id } = s.create(init);
    clock.t = 90;
    expect(s.get(id)).not.toBeNull();
    clock.t = 180; // 90 s since last activity
    expect(s.get(id)).not.toBeNull();
    clock.t = 281; // 101 s idle
    expect(s.get(id)).toBeNull();
    expect(s.size()).toBe(0);
  });
  it("does not count a background request as activity (CQ-8)", () => {
    const { s, clock } = store(100, 1000);
    const { id } = s.create(init);
    // A page polling every 60 s with nobody at the keyboard: the session still ends on time.
    for (clock.t = 60; clock.t <= 60 * 5; clock.t += 60) {
      const live = s.get(id, { activity: false });
      if (clock.t <= 100) expect(live).not.toBeNull();
      else expect(live).toBeNull();
    }
    expect(s.size()).toBe(0);
  });
  it("ends a session at the absolute limit however active", () => {
    const { s, clock } = store(100, 250);
    const { id } = s.create(init);
    for (clock.t = 50; clock.t <= 250; clock.t += 50)
      expect(s.get(id)).not.toBeNull();
    clock.t = 251;
    expect(s.get(id)).toBeNull();
  });
  it("returns null for a missing, unknown or deleted id", () => {
    const { s } = store();
    expect(s.get(undefined)).toBeNull();
    expect(s.get("nope")).toBeNull();
    const { id } = s.create(init);
    s.delete(id);
    expect(s.get(id)).toBeNull();
  });
  it("makes a login attempt single-use and short-lived", () => {
    const { s, clock } = store();
    const attempt = { state: "s", nonce: "n", verifier: "v", returnTo: "/" };
    const id = s.beginLogin(attempt);
    expect(s.takeLogin(id)?.state).toBe("s");
    expect(s.takeLogin(id)).toBeNull();
    const late = s.beginLogin(attempt);
    clock.t = LOGIN_TTL_SECONDS + 1;
    expect(s.takeLogin(late)).toBeNull();
  });
});
