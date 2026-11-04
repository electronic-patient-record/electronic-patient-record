import { describe, expect, it } from "vitest";
import {
  acknowledgeBody,
  itemHeadline,
  untimedWarning,
  type WorkItem,
} from "../../src/lib/referral-worklist";
import { isWorklistPage } from "../../src/lib/feed-count.svelte";

const item: WorkItem = {
  referral_id: "r1",
  patient_id: "p1",
  version: 3,
  state: "declined",
  why: "reply",
  recorded_at: "2026-10-21T09:00:12.000000Z",
};

describe("the referral reply list", () => {
  it("says what came back in words and when", () => {
    expect(itemHeadline(item)).toBe("Declined · 2026-10-21 09:00 UTC");
  });
  it("says a timeout in its own words, not as a reply", () => {
    expect(
      itemHeadline({ ...item, state: "requested", why: "no-response" }),
    ).toBe("No answer yet · 2026-10-21 09:00 UTC");
    expect(
      itemHeadline({ ...item, state: "accepted", why: "no-outcome" }),
    ).toBe("Accepted, no outcome yet · 2026-10-21 09:00 UTC");
  });
  it("warns about priorities with no waiting period, and says nothing when all are set", () => {
    expect(untimedWarning(undefined)).toBeNull();
    expect(untimedWarning({ no_response: [], no_outcome: [] })).toBeNull();
    expect(
      untimedWarning({
        no_response: ["urgent"],
        no_outcome: ["urgent", "stat"],
      }),
    ).toBe(
      "No waiting period is set for: Urgent, Immediately. A referral of those priorities that goes quiet will not appear here.",
    );
  });
  it("acknowledges the version it showed, so a newer reply is never swallowed", () => {
    expect(acknowledgeBody(item)).toEqual({ version: 3 });
  });
  it("fetches the count on the list and a patient's record only", () => {
    expect(isWorklistPage("/en-001/referrals")).toBe(true);
    expect(isWorklistPage("/en-001/patients/abc")).toBe(true);
    expect(isWorklistPage("/en-001/patients/abc/")).toBe(true);
    for (const p of [
      "/en-001/patients",
      "/en-001/patients/new",
      "/en-001/patients/compare",
      "/en-001/patients/abc/access",
      "/en-001/admin/feed",
      "/en-001/my-record",
      "/en-001",
    ])
      expect(isWorklistPage(p), p).toBe(false);
  });
});
