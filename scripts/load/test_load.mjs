// Tests for the load harness (spec/performance-testing.md): `node --test scripts/load/`.
// The statistics, both load shapes, budgets, the baseline comparison, the options, and whole runs against a fake
// service that answers well, answers badly, or is not there.

import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtempSync, readFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { checkBudgets, closedLoop, compareRuns, openLoop, quantile, summarise } from "./lib.mjs";
import { parse } from "./run.mjs";
import { CHART_PARTS, newPatient, word } from "./scenarios.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

test("quantile is nearest-rank, and null for no data", () => {
  const xs = Array.from({ length: 100 }, (_, i) => i + 1);
  assert.equal(quantile(xs, 0.5), 50);
  assert.equal(quantile(xs, 0.95), 95);
  assert.equal(quantile(xs, 0.99), 99);
  assert.equal(quantile(xs, 1), 100);
  assert.equal(quantile([7], 0.5), 7);
  assert.equal(quantile([], 0.5), null);
});

test("summarise counts 4xx, 5xx and no answer as failures, and 2xx and 3xx as not", () => {
  const s = summarise({ latencies: [1, 2, 3, 4, 5], outcomes: [200, 201, 304, 422, "error"], elapsedSeconds: 1 });
  assert.equal(s.requests, 5);
  assert.equal(s.failures, 2);
  assert.equal(s.failure_rate, 0.4);
  assert.equal(s.per_second, 5);
  assert.deepEqual(s.statuses, { 200: 1, 201: 1, 304: 1, 422: 1, error: 1 });
  assert.equal(s.p50_ms, 3);
  assert.equal(s.max_ms, 5);
});

test("summarise of nothing is nulls, not zeros", () => {
  const s = summarise({ latencies: [], outcomes: [], elapsedSeconds: 0 });
  assert.equal(s.requests, 0);
  assert.equal(s.p95_ms, null);
  assert.equal(s.per_second, null);
  assert.equal(s.failure_rate, null);
});

test("a closed loop keeps the given number of requests in flight and leaves the warm-up out", async () => {
  let inFlight = 0;
  let most = 0;
  let calls = 0;
  const s = await closedLoop({
    concurrency: 4,
    seconds: 0.3,
    warmupSeconds: 0.1,
    op: async () => {
      calls += 1;
      inFlight += 1;
      most = Math.max(most, inFlight);
      await sleep(5);
      inFlight -= 1;
      return { status: 200 };
    },
  });
  assert.equal(most, 4);
  assert.ok(s.requests > 0 && s.requests < calls, `counted ${s.requests} of ${calls}`);
  assert.ok(s.p50_ms >= 4, `p50 ${s.p50_ms}`);
  assert.equal(s.failures, 0);
});

test("a thrown request is the outcome error, a failure", async () => {
  const s = await closedLoop({ concurrency: 1, seconds: 0.05, op: async () => { await sleep(1); throw new Error("reset"); } });
  assert.ok(s.requests > 0);
  assert.equal(s.failures, s.requests);
  assert.ok(s.statuses.error > 0);
});

test("an open loop charges a stall to every request it delayed (no coordinated omission)", async () => {
  // A service that answers one request at a time, 5 ms each, after a 200 ms stall. At 100 a second the requests
  // due during the stall queue behind it; their latency counts from when each was due, so the median shows the
  // queue. A closed loop would have sent fewer requests and reported only the one slow one.
  let busyUntil = 0;
  const s = await openLoop({
    rate: 100,
    seconds: 0.5,
    op: async () => {
      // One at a time, 5 ms each, after a 200 ms stall at the start.
      const now = performance.now();
      const start = Math.max(now, busyUntil || now + 200);
      busyUntil = start + 5;
      await sleep(busyUntil - now);
      return { status: 200 };
    },
  });
  assert.equal(s.requests, 50);
  assert.equal(s.target_per_second, 100);
  // Request i is due at 10i ms and answered at 205 + 5i ms, so it waited 205 - 5i: the median about 80 ms, where the
  // service's own time is 5.
  assert.ok(s.p50_ms > 50, `the queue behind the stall shows in the median: p50 ${s.p50_ms}`);
  assert.ok(s.max_ms >= 195, `the first request waited out the stall: max ${s.max_ms}`);
});

test("an open loop drops what is past its in-flight limit, and counts it as failed", async () => {
  const s = await openLoop({ rate: 200, seconds: 0.2, maxInFlight: 2, op: async () => { await sleep(100); return { status: 200 }; } });
  assert.ok(s.dropped > 0);
  assert.equal(s.failures, s.dropped);
});

test("budgets: latency and failures, failures only, and a scenario with none", () => {
  const steps = [
    { scenario: "search", concurrency: 8, p95_ms: 250, failure_rate: 0 },
    { scenario: "read", concurrency: 8, p95_ms: 9999, failure_rate: 0 },
    { scenario: "other", concurrency: 1, p95_ms: 1, failure_rate: 0 },
  ];
  const budgets = { search: { p95_ms: 200, max_failure_rate: 0 }, read: { max_failure_rate: 0 } };
  const full = checkBudgets(steps, budgets);
  assert.equal(full.passed, false);
  assert.deepEqual(full.checks.filter((c) => c.result === "FAIL").map((c) => `${c.scenario} ${c.measure}`), ["search p95_ms"]);
  assert.ok(full.checks.some((c) => c.scenario === "other" && c.result === "no budget"));
  assert.equal(checkBudgets(steps, budgets, { latency: false }).passed, true);
  assert.equal(checkBudgets([{ scenario: "read", concurrency: 1, failure_rate: 0.01 }], budgets).passed, false);
  assert.equal(checkBudgets([{ scenario: "read", concurrency: 1, failure_rate: null }], budgets).passed, false, "no requests is not a pass");
});

test("a baseline comparison flags a slower p95 or a lower rate past the tolerance, and lists what is new or gone", () => {
  const base = { steps: [
    { scenario: "read", concurrency: 8, p95_ms: 10, per_second: 1000 },
    { scenario: "search", concurrency: 8, p95_ms: 10, per_second: 1000 },
    { scenario: "gone", concurrency: 1, p95_ms: 1, per_second: 1 },
  ] };
  const now = { steps: [
    { scenario: "read", concurrency: 8, p95_ms: 11, per_second: 950 },
    { scenario: "search", concurrency: 8, p95_ms: 13, per_second: 1000 },
    { scenario: "new", concurrency: 1, p95_ms: 1, per_second: 1 },
  ] };
  const c = compareRuns(base, now, { tolerance: 0.15 });
  const by = Object.fromEntries(c.rows.map((r) => [r.step, r.result]));
  assert.deepEqual(by, { "read@8": "ok", "search@8": "REGRESSED", "new@1": "new", "gone@1": "missing" });
  assert.equal(c.regressed, true);
  assert.equal(compareRuns(base, base).regressed, false);
});

test("options: profiles, overrides and refusals", () => {
  assert.equal(parse([]).profile, "standard");
  assert.deepEqual(parse(["--profile", "stress"]).scenarios, ["mixed"]);
  const s = parse(["--profile", "smoke", "--seconds", "1", "--scenarios", "read,search", "--concurrency", "1,4"]);
  assert.equal(s.seconds, 1);
  assert.deepEqual(s.scenarios, ["read", "search"]);
  assert.deepEqual(s.concurrency, [1, 4]);
  assert.equal(s.latencyBudgets, false);
  assert.equal(parse(["--profile", "standard", "--no-latency-budgets"]).latencyBudgets, false);
  assert.ok(!parse([]).scenarios.includes("mixed"), "mixed is not in the default list: it repeats the others");
  for (const bad of [["--profile", "x"], ["--scenarios", "nope"], ["--seconds", "x"], ["--concurrency", "0"], ["--bogus"], ["--out"]]) {
    assert.throws(() => parse(bad), undefined, bad.join(" "));
  }
});

test("synthetic patients are distinct and look like no one", () => {
  const seen = new Set();
  for (let n = 0; n < 5000; n++) {
    const p = newPatient("t", n);
    const key = `${p.names[0].family}|${p.birth_date}`;
    assert.ok(!seen.has(key), key);
    seen.add(key);
    assert.match(p.birth_date, /^(19|20)\d\d-(0[1-9]|1[0-2])-(0[1-9]|1\d|2[0-8])$/);
    assert.ok(p.names[0].family.startsWith("Load"));
  }
  assert.equal(word(0), "a");
  assert.equal(word(26), "ba");
});

/** A fake service: every route answers 200 (a registration 201 with an id), except `fail` paths, which answer 500. */
function fakeService({ fail = () => false } = {}) {
  const seen = [];
  const server = createServer((req, res) => {
    let body = "";
    req.on("data", (d) => (body += d));
    req.on("end", () => {
      seen.push(`${req.method} ${req.url.split("?")[0].replace(/[0-9a-f-]{36}/g, "{id}")}`);
      res.setHeader("content-type", "application/json");
      if (fail(req)) {
        res.statusCode = 500;
        return res.end("{}");
      }
      if (req.method === "POST" && req.url === "/api/patients") {
        res.statusCode = 201;
        return res.end(JSON.stringify({ id: crypto.randomUUID() }));
      }
      res.statusCode = req.method === "POST" ? 201 : 200;
      res.end("{}");
    });
  });
  return new Promise((resolve) => server.listen(0, "127.0.0.1", () => resolve({ server, seen, base: `http://127.0.0.1:${server.address().port}` })));
}

function run(base, args) {
  return new Promise((resolve) => {
    const child = spawn(process.execPath, [join(here, "run.mjs"), ...args], { env: { ...process.env, BASE: base } });
    let out = "";
    child.stdout.on("data", (d) => (out += d));
    child.stderr.on("data", (d) => (out += d));
    child.on("close", (code) => resolve({ code, out }));
  });
}

const QUICK = ["--profile", "smoke", "--seconds", "0.2", "--warmup", "0", "--seed-patients", "2"];

test("a whole run against a service that answers well passes, writes its JSON, and covers every scenario", async () => {
  const { server, seen, base } = await fakeService();
  const out = join(mkdtempSync(join(tmpdir(), "epr-load-")), "run.json");
  try {
    const r = await run(base, [...QUICK, "--out", out]);
    assert.equal(r.code, 0, r.out);
    const result = JSON.parse(readFileSync(out, "utf8"));
    assert.equal(result.passed, true);
    assert.equal(result.seeded, 2);
    assert.match(result.statement, /Not a capacity claim/);
    assert.ok(result.host.cpus > 0);
    assert.equal(result.steps.length, 9);
    for (const part of CHART_PARTS) assert.ok(seen.includes(`GET /api/patients/{id}${part}`), `chart reads ${part || "the patient"}`);
    for (const route of ["POST /api/patients/compare", "GET /fhir/R4/Patient/{id}", "GET /fhir/R4/Patient", "POST /api/patients/{id}/observations"]) {
      assert.ok(seen.includes(route), route);
    }
  } finally {
    server.close();
  }
});

test("a run where one scenario fails exits 1 and names it", async () => {
  const { server, base } = await fakeService({ fail: (req) => req.url.startsWith("/api/patients?family=") });
  try {
    const r = await run(base, [...QUICK, "--scenarios", "read,search"]);
    assert.equal(r.code, 1, r.out);
    assert.match(r.out, /search\s+2\s+failure_rate/);
    assert.match(r.out, /FAILED/);
  } finally {
    server.close();
  }
});

test("a run against a regression baseline exits 1", async () => {
  const { server, base } = await fakeService();
  const dir = mkdtempSync(join(tmpdir(), "epr-load-"));
  try {
    const first = await run(base, [...QUICK, "--scenarios", "read", "--out", join(dir, "a.json")]);
    assert.equal(first.code, 0, first.out);
    // A baseline 1000 times faster than anything possible: this run is a regression against it.
    const a = JSON.parse(readFileSync(join(dir, "a.json"), "utf8"));
    a.steps[0].p95_ms /= 1000;
    a.steps[0].per_second *= 1000;
    const { writeFileSync } = await import("node:fs");
    writeFileSync(join(dir, "fast.json"), JSON.stringify(a));
    const r = await run(base, [...QUICK, "--scenarios", "read", "--baseline", join(dir, "fast.json")]);
    assert.equal(r.code, 1, r.out);
    assert.match(r.out, /REGRESSED/);
  } finally {
    server.close();
  }
});

test("a run with no service exits 3, and a bad option exits 2", async () => {
  const { server, base } = await fakeService();
  await new Promise((r) => server.close(r));
  const gone = await run(base, QUICK);
  assert.equal(gone.code, 3, gone.out);
  assert.match(gone.out, /could not be used/);
  const bad = await run(base, ["--profile", "nope"]);
  assert.equal(bad.code, 2, bad.out);
});

test("a refused seed is exit 3: numbers about refusals would mean nothing", async () => {
  const { server, base } = await fakeService({ fail: (req) => req.method === "POST" });
  try {
    const r = await run(base, QUICK);
    assert.equal(r.code, 3, r.out);
    assert.match(r.out, /seeding: registration answered 500/);
  } finally {
    server.close();
  }
});
