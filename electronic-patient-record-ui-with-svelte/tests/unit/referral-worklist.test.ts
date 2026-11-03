import { describe, expect, it } from "vitest";
import {
  acknowledgeBody,
  itemHeadline,
  type WorkItem,
} from "../../src/lib/referral-worklist";
import { isWorklistPage } from "../../src/lib/feed-count.svelte";

const item: WorkItem = {
  referral_id: "r1",
  patient_id: "p1",
  version: 3,
  state: "declined",
  recorded_at: "2026-10-21T09:00:12.000000Z",
};

describe("the referral reply list", () => {
  it("says what came back in words and when", () => {
    expect(itemHeadline(item)).toBe("Declined · 2026-10-21 09:00 UTC");
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
