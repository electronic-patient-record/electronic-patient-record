import { describe, expect, it } from "vitest";
import { healthLines, type FeedHealth } from "../../src/lib/feed-health";
import {
  headline,
  isFingerprint,
  type Refusal,
} from "../../src/lib/feed-queue";

// IH-8: the feed's health in words, never by colour alone.
const f: FeedHealth = {
  sender: "mllp:adt-feed",
  state: "critical",
  window: "day",
  reasons: ["no message for longer than this window allows"],
  since: "2026-11-17T09:00:00.000000Z",
  last_message_at: "2026-11-17T07:30:00.000000Z",
  refusals_last_hour: 2,
  open_refusals: 3,
};

describe("feed health", () => {
  it("says the state first, in words, then why, the last message, the counts, the window and since when", () => {
    expect(healthLines(f)).toEqual([
      "Critical: mllp:adt-feed",
      "no message for longer than this window allows",
      "Last message: 2026-11-17 07:30 UTC",
      "Refused in the last hour: 2. Waiting for a person: 3.",
      "Judged by the weekday daytime limits.",
      "In this state since 2026-11-17 09:00 UTC.",
    ]);
  });
  it("a feed that has sent nothing is unknown, never healthy, and says so", () => {
    const lines = healthLines({
      ...f,
      state: "unknown",
      reasons: [],
      last_message_at: null,
      since: null,
    });
    expect(lines[0]).toBe("Unknown, nothing received yet: mllp:adt-feed");
    expect(lines).toContain("Last message: none since the record began");
  });
});

describe("a refusal's headline", () => {
  const r: Refusal = {
    id: "1",
    received_at: "2026-11-17T09:00:12.000000Z",
    agent: "mllp:adt-feed",
    peer: null,
    event: "A08",
    control_id: "M-1",
    kind: "conflict",
    reasons: [],
    resolution: null,
  };
  it("names the sender's own id, but leaves out a fingerprint (IH-3)", () => {
    expect(headline(r)).toBe(
      "A08 · message M-1 · received 2026-11-17 09:00 UTC",
    );
    for (const fp of ["sha256:" + "ab".repeat(32), "d1.k1:" + "A".repeat(43)]) {
      expect(isFingerprint(fp)).toBe(true);
      expect(headline({ ...r, control_id: fp })).toBe(
        "A08 · received 2026-11-17 09:00 UTC",
      );
    }
    for (const id of ["M-1", "20261117-1", "sha256:", "d1.k1:short"])
      expect(isFingerprint(id)).toBe(false);
  });
});
