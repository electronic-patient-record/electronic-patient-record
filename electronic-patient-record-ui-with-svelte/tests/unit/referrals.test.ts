import { afterEach, describe, expect, it, vi } from "vitest";
import {
  loadReferrals,
  PRIORITIES,
  PROVISIONAL,
} from "../../src/lib/referrals";

afterEach(() => vi.unstubAllGlobals());

describe("loading a patient's referrals", () => {
  it("returns the list on 200 and sends the purpose and background marker", async () => {
    const fetchMock = vi.fn(
      async () => new Response('{"referrals":[{"id":"r1"}]}', { status: 200 }),
    );
    vi.stubGlobal("fetch", fetchMock);
    const got = await loadReferrals("p1", {
      background: true,
      purpose: "treatment",
    });
    expect(got.status).toBe(200);
    expect(got.referrals).toEqual([{ id: "r1" }]);
    const [url, init] = fetchMock.mock.calls[0] as unknown as [
      string,
      RequestInit,
    ];
    expect(url).toBe("/api/proxy/api/patients/p1/referrals");
    expect(init.headers).toEqual({
      "x-epr-background": "1",
      "x-purpose-of-use": "treatment",
    });
  });

  it("returns null, never an empty list, on any other status", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response("{}", { status: 403 })),
    );
    expect(await loadReferrals("p1")).toEqual({ status: 403, referrals: null });
  });

  it("names the priorities and the provisional states", () => {
    expect(PRIORITIES).toEqual(["routine", "urgent", "asap", "stat"]);
    expect(PROVISIONAL).toContain("deferred");
  });
});
