import { describe, expect, it } from "vitest";
import { Busy } from "../../src/lib/busy.svelte";

describe("one write at a time", () => {
  it("ignores a call made while one is in flight and clears the flag afterwards", async () => {
    const busy = new Busy();
    let release!: () => void;
    let runs = 0;
    const first = busy.run(
      () =>
        new Promise<void>((resolve) => {
          runs += 1;
          release = resolve;
        }),
    );
    expect(busy.on).toBe(true);
    await busy.run(async () => {
      runs += 1;
    });
    expect(runs).toBe(1);
    release();
    await first;
    expect(busy.on).toBe(false);
  });

  it("clears the flag when the work throws, so the next call runs", async () => {
    const busy = new Busy();
    await expect(
      busy.run(async () => {
        throw new Error("refused");
      }),
    ).rejects.toThrow("refused");
    expect(busy.on).toBe(false);
    let ran = false;
    await busy.run(async () => {
      ran = true;
    });
    expect(ran).toBe(true);
  });
});
