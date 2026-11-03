import { describe, expect, it } from "vitest";
import {
  clearBody,
  queueHeadline,
  type QueueEntry,
} from "../../src/lib/referral-queue";

const e: QueueEntry = {
  id: "1",
  received_at: "2026-10-21T09:00:12.000000Z",
  agent: "clinic-feed",
  kind: "unknown-id",
  referral_ref: "R-404",
  event: "outcome",
  reasons: [],
  cleared: null,
};

describe("unmatched referral reply text", () => {
  it("names the event, the sender's reference and when", () => {
    expect(queueHeadline(e)).toBe(
      "outcome · reference R-404 · received 2026-10-21 09:00 UTC",
    );
  });
  it("says so when there is no reference or no event", () => {
    expect(queueHeadline({ ...e, referral_ref: null, event: null })).toBe(
      "no event · no reference · received 2026-10-21 09:00 UTC",
    );
  });
  it("sends nothing without a reason, and a hand match nothing without a referral id", () => {
    expect(clearBody("dismissed", "", "  ")).toBeNull();
    expect(clearBody("matched", "", "same patient")).toBeNull();
    expect(clearBody("matched", "  ", "same patient")).toBeNull();
  });
  it("trims, and a dismissal never sends a referral id", () => {
    expect(clearBody("dismissed", "ignored", " dup ")).toEqual({
      how: "dismissed",
      reason: "dup",
    });
    expect(clearBody("matched", " r-1 ", " same ")).toEqual({
      how: "matched",
      referral_id: "r-1",
      reason: "same",
    });
  });
});
