import { describe, expect, it } from "vitest";
import { idleWarning, WARN_SECONDS } from "../../src/lib/idle";

describe("idleWarning (CQ-8)", () => {
  const idle = 900;
  it("says nothing while there is time left", () => {
    expect(idleWarning(0, 0, idle)).toBeNull();
    expect(idleWarning((idle - WARN_SECONDS - 1) * 1000, 0, idle)).toBeNull();
  });
  it("warns in the last minutes with the whole minutes left, never fewer than one", () => {
    expect(idleWarning((idle - WARN_SECONDS) * 1000, 0, idle)).toBe(2);
    expect(idleWarning((idle - 61) * 1000, 0, idle)).toBe(2);
    expect(idleWarning((idle - 30) * 1000, 0, idle)).toBe(1);
  });
  it("says 0 once the time is up, and nothing when sign-in is off", () => {
    expect(idleWarning((idle + 5) * 1000, 0, idle)).toBe(0);
    expect(idleWarning(10_000_000, 0, null)).toBeNull();
  });
});
