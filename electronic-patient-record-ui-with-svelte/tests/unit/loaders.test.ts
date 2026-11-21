import { afterEach, describe, expect, it, vi } from "vitest";
import { loadAmendments } from "../../src/lib/amendments";
import { loadSuspicion } from "../../src/lib/suspicions";
import { loadAllergies } from "../../src/lib/allergies";
import { loadAssertions, loadDeath } from "../../src/lib/death";
import { lastActivityAt, noteActivity } from "../../src/lib/idle";
import { loadWatchList } from "../../src/lib/watch-list";

afterEach(() => vi.unstubAllGlobals());

function answer(status: number, body: unknown): void {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => new Response(JSON.stringify(body), { status })),
  );
}

describe("loaders keep a failed look apart from nothing found", () => {
  it("the watch list is null unless the answer is 200", async () => {
    answer(200, { entries: [{ id: "w1" }] });
    expect(await loadWatchList("p1")).toEqual({
      status: 200,
      entries: [{ id: "w1" }],
    });
    answer(500, {});
    expect(await loadWatchList("p1")).toEqual({ status: 500, entries: null });
  });

  it("the death answer is null unless the answer is 200", async () => {
    answer(200, { kind: "none" });
    expect((await loadDeath("p1")).answer).toEqual({ kind: "none" });
    answer(403, {});
    expect(await loadDeath("p1", { background: true })).toEqual({
      status: 403,
      answer: null,
    });
  });

  it("assertions are an empty list unless the answer is 200", async () => {
    answer(200, { assertions: [{ id: "a1" }] });
    expect((await loadAssertions("p1")).assertions).toEqual([{ id: "a1" }]);
    answer(404, {});
    expect(await loadAssertions("p1")).toEqual({ status: 404, assertions: [] });
  });

  it("allergies carry data on 200 and offer break-glass only on a 403 that says so", async () => {
    answer(200, { allergies: [] });
    expect((await loadAllergies("p1")).data).toEqual({ allergies: [] });
    answer(403, { break_glass_offer: true });
    expect(await loadAllergies("p1")).toEqual({
      status: 403,
      data: null,
      offer: true,
    });
    answer(403, {});
    expect(await loadAllergies("p1", "treatment")).toEqual({
      status: 403,
      data: null,
    });
  });
});

describe("amendments and suspicions", () => {
  it("amendments are null unless the answer is 200", async () => {
    answer(200, { amendments: [{ id: "a1" }] });
    expect(
      (await loadAmendments("p1", { purpose: "treatment" })).amendments,
    ).toEqual([{ id: "a1" }]);
    answer(500, {});
    expect(await loadAmendments("p1")).toEqual({
      status: 500,
      amendments: null,
    });
  });

  it("a suspicion history is null unless the answer is 200", async () => {
    answer(200, { entries: [], pathways: [] });
    expect((await loadSuspicion("p1", "r1")).history).toEqual({
      entries: [],
      pathways: [],
    });
    answer(404, {});
    const missing = await loadSuspicion("p1", "r1");
    expect(missing.status).toBe(404);
    expect(missing.history).toBeNull();
    expect(missing.pathways).toEqual([]);
  });
});

describe("activity", () => {
  it("remembers the time of the last activity", () => {
    noteActivity(12345);
    expect(lastActivityAt()).toBe(12345);
  });
});
