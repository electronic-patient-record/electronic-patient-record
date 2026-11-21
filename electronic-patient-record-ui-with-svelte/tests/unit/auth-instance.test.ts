import { randomBytes } from "node:crypto";
import type { Cookies } from "@sveltejs/kit";
import { afterEach, describe, expect, it, vi } from "vitest";

const client = vi.hoisted(() => ({
  on: vi.fn(),
  connect: vi.fn(async () => undefined),
  get: vi.fn(async () => "v"),
  set: vi.fn(async () => "OK"),
  getDel: vi.fn(async () => "v"),
  del: vi.fn(async () => 1),
}));
vi.mock("redis", () => ({ createClient: vi.fn(() => client) }));

import type { Auth } from "../../src/lib/auth";
import {
  backendFrom,
  cookieName,
  localeFrom,
  sameOrigin,
  sessionFrom,
} from "../../src/lib/auth-instance";
import { MemoryBackend, RedisBackend } from "../../src/lib/session-backends";

afterEach(() => vi.restoreAllMocks());

const cookies = (values: Record<string, string>) =>
  ({ get: (k: string) => values[k] }) as unknown as Cookies;

describe("the session backend from the environment", () => {
  it("is memory with neither setting, and refuses half a setting either way", () => {
    expect(backendFrom("", "")).toBeInstanceOf(MemoryBackend);
    expect(() => backendFrom("", "key")).toThrow(
      /without EPR_SESSION_REDIS_URL/,
    );
    expect(() => backendFrom("redis://x", "")).toThrow(
      /needs EPR_SESSION_ENCRYPTION_KEY/,
    );
  });

  it("is Redis with both, connecting once and only when first used", async () => {
    const key = randomBytes(32).toString("base64");
    const backend = backendFrom("redis://x", key);
    expect(backend).toBeInstanceOf(RedisBackend);
    expect(client.connect).not.toHaveBeenCalled();
    const redis = (
      backend as unknown as {
        redis: Record<string, (...args: unknown[]) => Promise<unknown>>;
      }
    ).redis;
    await redis.get("a");
    await redis.set("a", "b", { EX: 1 });
    await redis.getDel("a");
    await redis.del("a");
    expect(client.connect).toHaveBeenCalledTimes(1);
    const onError = client.on.mock.calls[0]![1] as (e: Error) => void;
    const log = vi.spyOn(console, "error").mockImplementation(() => {});
    onError(new Error("down"));
    expect(log).toHaveBeenCalledWith("session store (Redis):", "down");
  });
});

describe("cookies and origin", () => {
  it("pins a cookie to the host only on HTTPS", () => {
    expect(cookieName("epr_session", true)).toBe("__Host-epr_session");
    expect(cookieName("epr_session", false)).toBe("epr_session");
  });

  it("accepts only the site's own origin", () => {
    const req = (origin?: string) =>
      new Request("https://x/", origin ? { headers: { origin } } : {});
    expect(sameOrigin(req("https://x"), "https://x")).toBe(true);
    expect(sameOrigin(req("https://evil"), "https://x")).toBe(false);
    expect(sameOrigin(req(), "https://x")).toBe(false);
  });

  it("falls back to the default locale when the cookie is absent or unknown", () => {
    expect(localeFrom(cookies({ locale: "xx-nowhere" }))).toBe(
      localeFrom(cookies({})),
    );
  });
});

describe("looking up the session", () => {
  it("is null without sign-in, passes the cookie to the store, and shows signed out when the store fails", async () => {
    expect(await sessionFrom(null, cookies({}), false)).toBeNull();
    const get = vi.fn(async () => ({ subject: "s" }));
    const auth = { store: { get } } as unknown as Auth;
    const got = await sessionFrom(
      auth,
      cookies({ "__Host-epr_session": "abc" }),
      true,
      { activity: false },
    );
    expect(got).toEqual({ subject: "s" });
    expect(get).toHaveBeenCalledWith("abc", { activity: false });
    const failing = {
      store: {
        get: async () => {
          throw new Error("redis down");
        },
      },
    } as unknown as Auth;
    const log = vi.spyOn(console, "error").mockImplementation(() => {});
    expect(await sessionFrom(failing, cookies({}), false)).toBeNull();
    expect(log).toHaveBeenCalledWith("session lookup failed:", "redis down");
  });
});
