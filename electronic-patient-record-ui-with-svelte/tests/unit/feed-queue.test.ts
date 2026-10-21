import { describe, expect, it } from "vitest";
import {
  headline,
  readable,
  resolveBody,
  type Refusal,
} from "../../src/lib/feed-queue";

const r: Refusal = {
  id: "1",
  received_at: "2026-10-21T09:00:12.000000Z",
  agent: "mllp:adt-feed",
  peer: "10.0.0.5",
  event: "A08",
  control_id: "M-1",
  kind: "conflict",
  reasons: ["the birth date differs"],
  resolution: null,
};

describe("refused message text", () => {
  it("names the event, the sender's control id and when", () => {
    expect(headline(r)).toBe(
      "A08 · message M-1 · received 2026-10-21 09:00 UTC",
    );
  });
  it("shows HL7 segments on separate lines", () => {
    expect(readable("MSH|a\rPID|1\r")).toBe("MSH|a\nPID|1");
    expect(readable("MSH|a\r\nPID|1")).toBe("MSH|a\nPID|1");
  });
  it("sends a reason, trimmed, and nothing at all without one", () => {
    expect(resolveBody("handled", "  fixed by hand ")).toEqual({
      outcome: "handled",
      reason: "fixed by hand",
    });
    expect(resolveBody("dismissed", "   ")).toBeNull();
  });
});
