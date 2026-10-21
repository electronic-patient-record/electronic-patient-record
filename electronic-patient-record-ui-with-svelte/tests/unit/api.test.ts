import { afterEach, describe, expect, it, vi } from "vitest";
import { call } from "../../src/lib/api";

afterEach(() => vi.unstubAllGlobals());

describe("call", () => {
  it("sends the purpose of use only when given, and a JSON content type only with a body", async () => {
    const fetchMock = vi.fn(
      async () => new Response('{"ok":true}', { status: 200 }),
    );
    vi.stubGlobal("fetch", fetchMock);
    await call("GET", "/api/me/patients", undefined, "patient-access");
    await call("POST", "/api/patients", { a: 1 });
    const [first, second] = fetchMock.mock.calls.map(
      (c) => (c as unknown as [string, RequestInit])[1],
    );
    expect(first?.headers).toEqual({ "x-purpose-of-use": "patient-access" });
    expect(second?.headers).toEqual({ "content-type": "application/json" });
  });

  it("turns a network failure into status 0 and an empty body into null", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        throw new Error("down");
      }),
    );
    expect((await call("GET", "/x")).status).toBe(0);
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(null, { status: 204 })),
    );
    expect(await call("GET", "/x")).toEqual({ status: 204, body: null });
  });

  it("marks a background call so the server does not count it as activity", async () => {
    const fetchMock = vi.fn(async () => new Response("{}", { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    await call("GET", "/api/patients/x", undefined, undefined, {
      background: true,
    });
    await call("GET", "/api/patients/x");
    const [bg, fg] = fetchMock.mock.calls.map(
      (c) => (c as unknown as [string, RequestInit])[1],
    );
    expect(bg?.headers).toEqual({ "x-epr-background": "1" });
    expect(fg?.headers).toEqual({});
  });
});
