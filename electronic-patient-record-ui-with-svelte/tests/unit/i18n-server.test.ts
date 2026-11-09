import { describe, expect, it, vi } from "vitest";

// The session and sign-in are stood in for: this test is about the locale each request renders in.
vi.mock("#lib/auth-instance.js", () => ({
  getAuth: vi.fn(() => ({})),
  sessionFrom: vi.fn(async () => null),
}));

import { handle } from "../../src/hooks.server";
import { TEXT } from "../../src/lib/i18n";
import { STRINGS } from "../../src/lib/strings";
import { CY } from "../../src/lib/strings.cy";

/** Run the server hook for `path`; the page "renders" TEXT after a pause, as a slow load would. */
function request(path: string, pause: Promise<void>) {
  const url = new URL(`https://epr.example${path}`);
  const event = {
    url,
    request: new Request(url),
    cookies: { get: () => undefined, set: vi.fn() },
    locals: {} as Record<string, unknown>,
  };
  const seen: string[] = [];
  const resolve = async () => {
    seen.push(TEXT.navHome);
    await pause;
    seen.push(TEXT.navHome, TEXT.sexes.female);
    return new Response("ok");
  };
  return { seen, done: handle({ event, resolve } as never) };
}

describe("server rendering in the request's locale (WL-8)", () => {
  it("keeps each request's words to itself when two render at once", async () => {
    let go!: () => void;
    const pause = new Promise<void>((r) => (go = r));
    const cy = request("/cy-001/patients", pause);
    const en = request("/en-001/patients", pause);
    const de = request("/de-001/patients", pause);
    go();
    await Promise.all([cy.done, en.done, de.done]);
    expect(cy.seen).toEqual([CY.navHome, CY.navHome, CY.sexes.female]);
    expect(en.seen).toEqual([
      STRINGS.navHome,
      STRINGS.navHome,
      STRINGS.sexes.female,
    ]);
    // A locale with no table of its own reads English.
    expect(de.seen).toEqual(en.seen);
    // Outside any request, English.
    expect(TEXT.navHome).toBe(STRINGS.navHome);
  });
});
