import { afterEach, describe, expect, it, vi } from "vitest";
import { call, fetchBlob, sendBytes } from "../../src/lib/api";

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

  it("sends an Idempotency-Key only when given", async () => {
    const fetchMock = vi.fn(async () => new Response("{}", { status: 201 }));
    vi.stubGlobal("fetch", fetchMock);
    await call("POST", "/api/patients", { a: 1 }, undefined, {
      idempotencyKey: "k-1",
    });
    await call("POST", "/api/patients", { a: 1 });
    const [keyed, plain] = fetchMock.mock.calls.map(
      (c) => (c as unknown as [string, RequestInit])[1],
    );
    expect(keyed?.headers).toEqual({
      "content-type": "application/json",
      "idempotency-key": "k-1",
    });
    expect(plain?.headers).toEqual({ "content-type": "application/json" });
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

describe("call and a list's next page", () => {
  it("returns the next page's path when the API names one, and none otherwise", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(
        async () =>
          new Response('{"results":[]}', {
            status: 200,
            headers: { link: '</api/patients?cursor=c1>; rel="next"' },
          }),
      ),
    );
    expect((await call("GET", "/api/patients?family=S")).next).toBe(
      "/api/patients?cursor=c1",
    );
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response('{"results":[]}', { status: 200 })),
    );
    expect("next" in (await call("GET", "/api/patients?family=S"))).toBe(false);
  });
});

describe("fetchBlob and sendBytes", () => {
  it("fetchBlob returns the bytes and the next cursor, a null body on refusal, and status 0 on failure", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(
        async () =>
          new Response("png", {
            status: 200,
            headers: { "x-next-before": "c9" },
          }),
      ),
    );
    const ok = await fetchBlob("/api/x", "treatment");
    expect(ok.status).toBe(200);
    expect(ok.next).toBe("c9");
    expect(await ok.body?.text()).toBe("png");
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response("no", { status: 404 })),
    );
    expect(await fetchBlob("/api/x")).toEqual({ status: 404, body: null });
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        throw new Error("down");
      }),
    );
    expect(await fetchBlob("/api/x")).toEqual({ status: 0, body: null });
  });

  it("sendBytes sends the type and purpose, parses JSON, and survives a bad body or a failure", async () => {
    const fetchMock = vi.fn(
      async () => new Response('{"a":1}', { status: 201 }),
    );
    vi.stubGlobal("fetch", fetchMock);
    const blob = new Blob(["x"], { type: "image/png" });
    expect(await sendBytes("PUT", "/api/p", blob, "treatment")).toEqual({
      status: 201,
      body: { a: 1 },
    });
    const init = (
      fetchMock.mock.calls[0] as unknown as [string, RequestInit]
    )[1];
    expect(init.headers).toEqual({
      "content-type": "image/png",
      "x-purpose-of-use": "treatment",
    });
    await sendBytes("POST", "/api/p", null);
    const second = (
      fetchMock.mock.calls[1] as unknown as [string, RequestInit]
    )[1];
    expect(second.headers).toEqual({
      "content-type": "application/octet-stream",
    });
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response("<html>", { status: 502 })),
    );
    expect(await sendBytes("PUT", "/api/p", blob)).toEqual({
      status: 502,
      body: null,
    });
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        throw new Error("down");
      }),
    );
    expect(await sendBytes("PUT", "/api/p", blob)).toEqual({
      status: 0,
      body: null,
    });
  });
});
