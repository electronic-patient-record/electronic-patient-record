import { beforeEach, describe, expect, it, vi } from "vitest";

// The session and sign-in are stood in for: these tests are about what the hook does with them.
const auth = vi.hoisted(() => ({
  getAuth: vi.fn(() => ({ fake: "auth" })),
  sessionFrom: vi.fn(),
}));
vi.mock("#lib/auth-instance.js", () => auth);

import { handle, init, LOCALE_COOKIE } from "../../src/hooks.server";
import { BACKGROUND_HEADER } from "../../src/lib/proxy";

function input(
  path: string,
  {
    method = "GET",
    headers = {},
    cookie,
  }: {
    method?: string;
    headers?: Record<string, string>;
    cookie?: string;
  } = {},
) {
  const url = new URL(`https://epr.example${path}`);
  const set = vi.fn();
  const event = {
    url,
    request: new Request(url, { method, headers }),
    cookies: {
      get: (name: string) => (name === LOCALE_COOKIE ? cookie : undefined),
      set,
    },
    locals: {} as Record<string, unknown>,
  };
  let transform: ((p: { html: string; done: boolean }) => unknown) | undefined;
  const resolve = vi.fn(
    async (
      _e: unknown,
      options?: { transformPageChunk?: typeof transform },
    ) => {
      transform = options?.transformPageChunk;
      return new Response("ok");
    },
  );
  return {
    event,
    resolve,
    set,
    html: () =>
      transform?.({
        html: '<html lang="%lang%" dir="%dir%">',
        done: true,
      }),
    run: () => handle({ event, resolve } as never),
  };
}

/** What a thrown `redirect()` carries, or null when nothing was thrown. */
async function redirected(run: () => Promise<unknown>) {
  try {
    await run();
    return null;
  } catch (e) {
    return e as { status: number; location: string };
  }
}

beforeEach(() => {
  auth.getAuth.mockClear();
  auth.sessionFrom.mockReset();
  auth.sessionFrom.mockResolvedValue(null);
});

describe("hooks.server (CQ-35)", () => {
  it("builds sign-in at startup, so a bad configuration stops the server there", () => {
    init();
    expect(auth.getAuth).toHaveBeenCalledOnce();
  });

  it("does not count a background look as activity (H-025)", async () => {
    await input("/en-001/patients", {
      headers: { [BACKGROUND_HEADER]: "1" },
    }).run();
    expect(auth.sessionFrom.mock.calls[0][3]).toEqual({ activity: false });
  });

  it("counts anything else as activity", async () => {
    await input("/en-001/patients").run();
    expect(auth.sessionFrom.mock.calls[0][3]).toEqual({ activity: true });
    await input("/en-001/patients", {
      headers: { [BACKGROUND_HEADER]: "0" },
    }).run();
    expect(auth.sessionFrom.mock.calls[1][3]).toEqual({ activity: true });
  });

  it("gives the page only the subject and name of a session, never its tokens", async () => {
    auth.sessionFrom.mockResolvedValue({
      subject: "s1",
      name: "A Nurse",
      accessToken: "secret",
    });
    const i = input("/en-001/patients");
    await i.run();
    expect(i.event.locals.user).toEqual({ subject: "s1", name: "A Nurse" });
  });

  it("says no one is signed in when there is no session", async () => {
    const i = input("/en-001/patients");
    await i.run();
    expect(i.event.locals.user).toBeNull();
  });

  it("sends a bare address to the remembered locale before the browser's", async () => {
    const r = await redirected(
      input("/patients?q=1", {
        cookie: "cy-001",
        headers: { "accept-language": "de" },
      }).run,
    );
    expect(r?.status).toBe(302);
    expect(r?.location).toBe("/cy-001/patients?q=1");
  });

  it("uses the browser's language when nothing is remembered", async () => {
    const r = await redirected(
      input("/patients", { headers: { "accept-language": "ar" } }).run,
    );
    expect(r?.location).toMatch(/^\/ar[-/]/);
  });

  it("does not redirect a write, which would lose its body", async () => {
    const i = input("/patients", { method: "POST" });
    expect(await redirected(i.run)).toBeNull();
    expect(i.resolve).toHaveBeenCalledOnce();
  });

  it("remembers a locale once, and only when it changes", async () => {
    const changed = input("/cy-001/patients", { cookie: "en-001" });
    await changed.run();
    expect(changed.event.locals.locale).toBe("cy-001");
    expect(changed.set).toHaveBeenCalledOnce();
    expect(changed.set.mock.calls[0].slice(0, 2)).toEqual([
      LOCALE_COOKIE,
      "cy-001",
    ]);
    const same = input("/cy-001/patients", { cookie: "cy-001" });
    await same.run();
    expect(same.set).not.toHaveBeenCalled();
  });

  it("writes the language and direction into the page", async () => {
    const arabic = input("/ar-001/patients");
    await arabic.run();
    expect(arabic.html()).toBe('<html lang="ar-001" dir="rtl">');
    const welsh = input("/cy-001/patients");
    await welsh.run();
    expect(welsh.html()).toBe('<html lang="cy-001" dir="ltr">');
  });

  it("sets the security headers on what it returns", async () => {
    const res = (await input("/en-001/patients").run()) as Response;
    expect(res.headers.get("x-content-type-options")).toBe("nosniff");
  });

  it("still answers when the response's headers cannot be changed", async () => {
    const i = input("/en-001/patients");
    const frozen = new Response("ok");
    Object.defineProperty(frozen, "headers", {
      value: {
        set() {
          throw new TypeError("immutable");
        },
      },
    });
    i.resolve.mockResolvedValue(frozen);
    expect(await i.run()).toBe(frozen);
  });
});
