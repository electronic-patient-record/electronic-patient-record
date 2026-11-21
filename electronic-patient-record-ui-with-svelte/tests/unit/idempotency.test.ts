import { describe, expect, it } from "vitest";
import { newIdempotencyKey } from "../../src/lib/idempotency";

describe("a key for one create", () => {
  it("is a UUID and differs each time", () => {
    const a = newIdempotencyKey();
    expect(a).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/,
    );
    expect(newIdempotencyKey()).not.toBe(a);
  });
});
