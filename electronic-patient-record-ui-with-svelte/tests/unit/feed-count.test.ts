import { afterEach, describe, expect, it, vi } from "vitest";
import {
  deathReportsCount,
  feedCount,
  referralQueueCount,
  refreshDeathReportsCount,
  refreshFeedCount,
  refreshReferralQueueCount,
  refreshWorklistCount,
  worklistCount,
} from "../../src/lib/feed-count.svelte";

afterEach(() => vi.unstubAllGlobals());

function answer(status: number, body: unknown): void {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => new Response(JSON.stringify(body), { status })),
  );
}

const counters = [
  ["feed", feedCount, refreshFeedCount, "/api/feed/refusals/count"],
  [
    "referral queue",
    referralQueueCount,
    refreshReferralQueueCount,
    "/api/referral-queue/count",
  ],
  [
    "death reports",
    deathReportsCount,
    refreshDeathReportsCount,
    "/api/death-reports/count",
  ],
  [
    "worklist",
    worklistCount,
    refreshWorklistCount,
    "/api/referrals/worklist/count",
  ],
] as const;

describe.each(counters)("the %s count", (_name, state, refresh, path) => {
  it("shows the number on 200 and from the right endpoint", async () => {
    answer(200, { open: 3 });
    await refresh();
    expect(state.open).toBe(3);
    expect((vi.mocked(fetch).mock.calls[0] as unknown as [string])[0]).toBe(
      `/api/proxy${path}`,
    );
  });

  it("shows nothing for a refusal, and nothing for a non-integer", async () => {
    answer(403, { open: 3 });
    await refresh();
    expect(state.open).toBeNull();
    answer(200, { open: "3" });
    await refresh();
    expect(state.open).toBeNull();
    answer(200, null);
    await refresh();
    expect(state.open).toBeNull();
  });
});
