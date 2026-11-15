// The load harness's measuring parts (spec/performance-testing.md): statistics, the two ways of driving load,
// budgets and the comparison with a baseline. No dependencies: Node 26 only. Tested by test_load.mjs.

/** The value at quantile `q` (0 to 1) of `sorted`, by the nearest-rank method; null when there is none. */
export function quantile(sorted, q) {
  if (sorted.length === 0) return null;
  const rank = Math.ceil(q * sorted.length);
  return sorted[Math.min(sorted.length - 1, Math.max(0, rank - 1))];
}

const round = (x) => (x === null ? null : Math.round(x * 100) / 100);

/**
 * A step's summary from its latencies (ms) and outcomes. An outcome is an HTTP status, or "error" for a request that
 * got no answer. A status of 400 or more counts as a failure, as does "error"; `expected` lists statuses that are
 * the scenario's right answer although they are 4xx (none do yet).
 */
export function summarise({ latencies, outcomes, elapsedSeconds, expected = [] }) {
  const sorted = [...latencies].sort((a, b) => a - b);
  const statuses = {};
  let failures = 0;
  for (const o of outcomes) {
    statuses[o] = (statuses[o] ?? 0) + 1;
    if (o === "error" || (Number(o) >= 400 && !expected.includes(Number(o)))) failures += 1;
  }
  const n = sorted.length;
  const mean = n ? sorted.reduce((s, x) => s + x, 0) / n : null;
  return {
    requests: n,
    per_second: elapsedSeconds > 0 ? round(n / elapsedSeconds) : null,
    min_ms: round(sorted[0] ?? null),
    mean_ms: round(mean),
    p50_ms: round(quantile(sorted, 0.5)),
    p90_ms: round(quantile(sorted, 0.9)),
    p95_ms: round(quantile(sorted, 0.95)),
    p99_ms: round(quantile(sorted, 0.99)),
    max_ms: round(sorted[n - 1] ?? null),
    failures,
    failure_rate: n ? round(failures / n) : null,
    statuses,
  };
}

/** Time one call of `op`, which resolves to { status } (or throws, which is the outcome "error"). */
async function timed(op, now) {
  const t = now();
  let outcome;
  try {
    outcome = (await op()).status;
  } catch {
    outcome = "error";
  }
  return { ms: now() - t, outcome };
}

/**
 * Closed loop: `concurrency` workers each send the next request when the last one answers, for `seconds`, after
 * `warmupSeconds` whose requests are not counted. It measures what the service sustains; when the service slows,
 * the load falls with it, so it hides queueing (use `openLoop` for that).
 */
export async function closedLoop({ op, concurrency, seconds, warmupSeconds = 0, now = () => performance.now() }) {
  const latencies = [];
  const outcomes = [];
  const warmEnd = now() + warmupSeconds * 1000;
  const end = warmEnd + seconds * 1000;
  let started = null;
  await Promise.all(
    Array.from({ length: concurrency }, async () => {
      while (now() < end) {
        const counting = now() >= warmEnd;
        if (counting && started === null) started = now();
        const r = await timed(op, now);
        if (counting) {
          latencies.push(r.ms);
          outcomes.push(r.outcome);
        }
      }
    }),
  );
  const elapsedSeconds = started === null ? 0 : (now() - started) / 1000;
  return summarise({ latencies, outcomes, elapsedSeconds });
}

/**
 * Open loop: requests are sent at a fixed `rate` a second whatever the service does, for `seconds`. Each latency
 * runs from the moment the request was due, not when it was sent, so a stall is charged to every request it
 * delayed (no coordinated omission). At most `maxInFlight` wait at once; a request due past that is not sent and
 * counts as "dropped", a failure, since a clinician would have waited for it.
 */
export async function openLoop({ op, rate, seconds, maxInFlight = 1024, now = () => performance.now() }) {
  const latencies = [];
  const outcomes = [];
  const interval = 1000 / rate;
  const total = Math.floor(rate * seconds);
  const start = now();
  let inFlight = 0;
  const pending = [];
  for (let i = 0; i < total; i++) {
    const due = start + i * interval;
    const wait = due - now();
    if (wait > 0) await new Promise((r) => setTimeout(r, wait));
    if (inFlight >= maxInFlight) {
      latencies.push(now() - due);
      outcomes.push("dropped");
      continue;
    }
    inFlight += 1;
    pending.push(
      (async () => {
        let outcome;
        try {
          outcome = (await op()).status;
        } catch {
          outcome = "error";
        }
        latencies.push(now() - due);
        outcomes.push(outcome);
        inFlight -= 1;
      })(),
    );
  }
  await Promise.all(pending);
  const summary = summarise({
    latencies,
    outcomes: outcomes.map((o) => (o === "dropped" ? "error" : o)),
    elapsedSeconds: (now() - start) / 1000,
  });
  summary.target_per_second = rate;
  summary.dropped = outcomes.filter((o) => o === "dropped").length;
  return summary;
}

/**
 * Each step checked against the budgets for its scenario. A budget is { p50_ms?, p95_ms?, p99_ms?,
 * max_failure_rate? }; a step whose scenario has none is "no budget". `latency` false checks only the failure rate
 * (a smoke run on a shared machine). Returns the checks and whether all passed.
 */
export function checkBudgets(steps, budgets, { latency = true } = {}) {
  const checks = [];
  for (const s of steps) {
    const b = budgets[s.scenario];
    if (!b) {
      checks.push({ scenario: s.scenario, concurrency: s.concurrency, measure: "-", result: "no budget" });
      continue;
    }
    const maxFailure = b.max_failure_rate ?? 0;
    checks.push({
      scenario: s.scenario,
      concurrency: s.concurrency,
      measure: "failure_rate",
      limit: maxFailure,
      value: s.failure_rate,
      result: s.failure_rate !== null && s.failure_rate <= maxFailure ? "pass" : "FAIL",
    });
    if (!latency) continue;
    for (const key of ["p50_ms", "p95_ms", "p99_ms"]) {
      if (b[key] === undefined) continue;
      if (b.max_concurrency !== undefined && s.concurrency > b.max_concurrency) continue;
      checks.push({
        scenario: s.scenario,
        concurrency: s.concurrency,
        measure: key,
        limit: b[key],
        value: s[key],
        result: s[key] !== null && s[key] <= b[key] ? "pass" : "FAIL",
      });
    }
  }
  return { checks, passed: checks.every((c) => c.result !== "FAIL") };
}

/**
 * This run's steps against a baseline run's: a step is a regression when its p95 grew, or its rate fell, by more
 * than `tolerance` (0.15 is 15 percent, about the noise of one laptop run). Steps in only one run are listed, not
 * judged.
 */
export function compareRuns(baseline, current, { tolerance = 0.15 } = {}) {
  const key = (s) => `${s.scenario}@${s.concurrency}`;
  const before = new Map(baseline.steps.map((s) => [key(s), s]));
  const rows = [];
  for (const s of current.steps) {
    const b = before.get(key(s));
    if (!b) {
      rows.push({ step: key(s), result: "new" });
      continue;
    }
    before.delete(key(s));
    const p95 = b.p95_ms ? s.p95_ms / b.p95_ms - 1 : 0;
    const rate = b.per_second ? s.per_second / b.per_second - 1 : 0;
    const regressed = p95 > tolerance || rate < -tolerance;
    rows.push({
      step: key(s),
      p95_change: round(p95),
      rate_change: round(rate),
      result: regressed ? "REGRESSED" : "ok",
    });
  }
  for (const k of before.keys()) rows.push({ step: k, result: "missing" });
  return { rows, regressed: rows.some((r) => r.result === "REGRESSED") };
}

/** A fixed-width text table of `rows` (objects) with the given columns. */
export function table(rows, columns) {
  const cell = (v) => (v === null || v === undefined ? "-" : String(v));
  const widths = columns.map((c) => Math.max(c.length, ...rows.map((r) => cell(r[c]).length)));
  const line = (vals) => vals.map((v, i) => v.padEnd(widths[i])).join("  ").trimEnd();
  return [line(columns), line(widths.map((w) => "-".repeat(w))), ...rows.map((r) => line(columns.map((c) => cell(r[c]))))].join(
    "\n",
  );
}
