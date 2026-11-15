// The load harness (OS-5, spec/performance-testing.md). No dependencies: Node 26 only.
//
//   BASE=http://127.0.0.1:1241 node scripts/load/run.mjs [options]
//   sh scripts/load/run-local.sh [options]          builds the service, brings up a scratch database, runs this
//
// The service must run with login off (`APP_ENV=container-dev`) against a database it may fill: the harness
// registers synthetic patients and writes to them.
//
// Options:
//   --profile NAME        smoke | standard | stress | soak (default standard); the other options override it
//   --scenarios a,b       which scenarios (--list shows them); default all but `mixed`, or `mixed` for stress/soak
//   --concurrency 1,8,32  closed-loop workers per step
//   --seconds N           counted seconds per step;  --warmup N  uncounted seconds before each step
//   --repeat N            run each step N times and report the median, with the spread of p95
//   --rate N              open loop at N requests a second instead of workers (soak uses it)
//   --minutes N           soak length;  --window N  soak window in seconds (default 30)
//   --seed-patients N     patients registered (untimed) before the steps, each with an allergy and 3 vitals
//   --budgets FILE        budgets (default scripts/load/budgets.json)
//   --latency-budgets     check latency budgets too (on in standard and stress; smoke and soak check failures only)
//   --no-latency-budgets  check failures only, whatever the profile (a measurement on a shared machine)
//   --out FILE            write the run as JSON;  --baseline FILE  compare with an earlier run's JSON
//   --tolerance X         regression tolerance for --baseline (default 0.15)
//   --list                list the scenarios and profiles, and exit
//
// Exit status: 0 passed; 1 a budget was broken, a soak drifted, or a step regressed against the baseline; 2 a usage
// error; 3 the service could not be reached or refused the seeding.

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import os from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { checkBudgets, closedLoop, compareRuns, openLoop, table } from "./lib.mjs";
import { context, MIX, SCENARIOS, seed } from "./scenarios.mjs";

const here = dirname(fileURLToPath(import.meta.url));

export const PROFILES = {
  smoke: { what: "every scenario once, briefly: it works, and nothing fails", concurrency: [2], seconds: 3, warmup: 1, repeat: 1, seedPatients: 20, latencyBudgets: false },
  standard: { what: "every scenario at 1, 8 and 32 workers, three times", concurrency: [1, 8, 32], seconds: 10, warmup: 2, repeat: 3, seedPatients: 200, latencyBudgets: true },
  stress: { what: "the mix with workers doubling from 1 to 256, until a budget breaks", concurrency: [1, 2, 4, 8, 16, 32, 64, 128, 256], seconds: 10, warmup: 2, repeat: 1, seedPatients: 200, latencyBudgets: true, scenarios: ["mixed"], stopAtBreak: true },
  soak: { what: "the mix at a fixed rate for a long time, in windows: does it drift", rate: 50, minutes: 10, window: 30, seedPatients: 200, latencyBudgets: false, scenarios: ["mixed"] },
};

function parse(argv) {
  const o = {};
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const value = () => {
      if (i + 1 >= argv.length) throw new Error(`${a} needs a value`);
      return argv[++i];
    };
    const number = () => {
      const v = Number(value());
      if (!Number.isFinite(v) || v < 0) throw new Error(`${a} needs a number`);
      return v;
    };
    switch (a) {
      case "--profile": o.profile = value(); break;
      case "--scenarios": o.scenarios = value().split(","); break;
      case "--concurrency": o.concurrency = value().split(",").map(Number); break;
      case "--seconds": o.seconds = number(); break;
      case "--warmup": o.warmup = number(); break;
      case "--repeat": o.repeat = number(); break;
      case "--rate": o.rate = number(); break;
      case "--minutes": o.minutes = number(); break;
      case "--window": o.window = number(); break;
      case "--seed-patients": o.seedPatients = number(); break;
      case "--budgets": o.budgets = value(); break;
      case "--latency-budgets": o.latencyBudgets = true; break;
      case "--no-latency-budgets": o.latencyBudgets = false; break;
      case "--out": o.out = value(); break;
      case "--baseline": o.baseline = value(); break;
      case "--tolerance": o.tolerance = number(); break;
      case "--list": o.list = true; break;
      default: throw new Error(`unknown option ${a}`);
    }
  }
  const profile = PROFILES[o.profile ?? "standard"];
  if (!profile) throw new Error(`unknown profile ${o.profile}: ${Object.keys(PROFILES).join(", ")}`);
  const settings = { ...profile, ...o, profile: o.profile ?? "standard" };
  settings.scenarios ??= Object.keys(SCENARIOS).filter((s) => s !== "mixed");
  for (const s of settings.scenarios) if (!SCENARIOS[s]) throw new Error(`unknown scenario ${s}`);
  if (settings.concurrency?.some((c) => !Number.isInteger(c) || c < 1)) throw new Error("--concurrency needs whole numbers of 1 or more");
  return settings;
}

const median = (xs) => {
  const s = [...xs].sort((a, b) => a - b);
  return s.length ? s[Math.floor((s.length - 1) / 2)] : null;
};

/** The median of each measure over repeats, with the spread of p95 and the outcomes added up. */
function combine(runs) {
  if (runs.length === 1) return runs[0];
  const out = {};
  for (const k of Object.keys(runs[0])) {
    if (typeof runs[0][k] === "number") out[k] = median(runs.map((r) => r[k]));
  }
  out.statuses = {};
  for (const r of runs) for (const [s, n] of Object.entries(r.statuses)) out.statuses[s] = (out.statuses[s] ?? 0) + n;
  out.failures = runs.reduce((s, r) => s + r.failures, 0);
  out.requests = runs.reduce((s, r) => s + r.requests, 0);
  out.failure_rate = out.requests ? Math.round((out.failures / out.requests) * 10000) / 10000 : null;
  out.p95_spread_ms = [Math.min(...runs.map((r) => r.p95_ms)), Math.max(...runs.map((r) => r.p95_ms))];
  out.repeats = runs.length;
  return out;
}

function meta(settings, base) {
  const git = (...args) => {
    try {
      return execFileSync("git", args, { cwd: here, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }).trim();
    } catch {
      return null;
    }
  };
  return {
    statement: "One run on one machine. Not a capacity claim: it says what this build did here, once.",
    at: new Date().toISOString(),
    commit: git("rev-parse", "HEAD"),
    dirty: git("status", "--porcelain", "--untracked-files=no") !== "",
    base,
    note: process.env.LOAD_NOTE ?? null,
    host: { os: `${os.type()} ${os.release()}`, arch: os.arch(), cpus: os.cpus().length, cpu: os.cpus()[0]?.model ?? null, memory_gib: Math.round(os.totalmem() / 2 ** 30) },
    node: process.version,
    settings: { ...settings, list: undefined, out: undefined, baseline: undefined },
  };
}

async function main() {
  let settings;
  try {
    settings = parse(process.argv.slice(2));
  } catch (e) {
    console.error(`load: ${e.message}`);
    return 2;
  }
  if (settings.list) {
    console.log("Scenarios:");
    for (const [name, s] of Object.entries(SCENARIOS)) console.log(`  ${name.padEnd(20)} ${s.what}`);
    console.log("Profiles:");
    for (const [name, p] of Object.entries(PROFILES)) console.log(`  ${name.padEnd(20)} ${p.what}`);
    return 0;
  }
  const base = process.env.BASE ?? "http://127.0.0.1:1241";
  const budgets = JSON.parse(readFileSync(settings.budgets ?? join(here, "budgets.json"), "utf8")).budgets;
  const ctx = context(base);
  try {
    const live = await fetch(`${base}/_health/live`);
    if (!live.ok) throw new Error(`/_health/live answered ${live.status}`);
    await seed(ctx, settings.seedPatients);
  } catch (e) {
    console.error(`load: the service at ${base} could not be used: ${e.message}`);
    return 3;
  }
  console.error(`load: ${settings.profile}, ${ctx.ids.length} patients seeded at ${base}`);

  const steps = [];
  let windows = null;
  if (settings.rate) {
    const total = (settings.minutes ?? 0) * 60 || settings.seconds || 10;
    const window = Math.min(settings.window ?? total, total);
    windows = [];
    for (const name of settings.scenarios) {
      for (let t = 0; t < total; t += window) {
        const s = await openLoop({ op: () => SCENARIOS[name].op(ctx), rate: settings.rate, seconds: Math.min(window, total - t) });
        windows.push({ scenario: name, from_s: t, ...s });
        console.error(`load: ${name} ${t}s +${window}s: p95 ${s.p95_ms} ms, ${s.failures} failed, ${s.dropped} dropped`);
      }
      const mine = windows.filter((w) => w.scenario === name);
      const all = combine(mine);
      steps.push({ scenario: name, concurrency: `rate ${settings.rate}`, ...all });
    }
  } else {
    for (const name of settings.scenarios) {
      for (const c of settings.concurrency) {
        const runs = [];
        for (let r = 0; r < settings.repeat; r++) {
          runs.push(await closedLoop({ op: () => SCENARIOS[name].op(ctx), concurrency: c, seconds: settings.seconds, warmupSeconds: settings.warmup }));
        }
        const step = { scenario: name, concurrency: c, ...combine(runs) };
        steps.push(step);
        console.error(`load: ${name} x${c}: ${step.per_second}/s, p95 ${step.p95_ms} ms, ${step.failures} failed`);
        if (settings.stopAtBreak) {
          const { passed } = checkBudgets([step], budgets, { latency: true });
          if (!passed) {
            console.error(`load: ${name} broke its budget at ${c} workers; stopping`);
            break;
          }
        }
      }
    }
  }

  const run = { ...meta(settings, base), seeded: ctx.ids.length, steps, windows };
  const budgetsResult = checkBudgets(steps, budgets, { latency: settings.latencyBudgets });
  run.budgets = budgetsResult;
  let passed = budgetsResult.passed;

  // A soak passes when no window failed and the last window's p95 is within 1.5 times the first's.
  if (windows && windows.length > 1) {
    run.drift = [];
    for (const name of settings.scenarios) {
      const mine = windows.filter((w) => w.scenario === name);
      const first = mine[0].p95_ms;
      const last = mine[mine.length - 1].p95_ms;
      const ratio = first ? Math.round((last / first) * 100) / 100 : null;
      const ok = ratio !== null && ratio <= 1.5 && mine.every((w) => w.failures === 0);
      run.drift.push({ scenario: name, first_p95_ms: first, last_p95_ms: last, ratio, result: ok ? "pass" : "FAIL" });
      passed &&= ok;
    }
  }
  if (settings.baseline) {
    run.comparison = compareRuns(JSON.parse(readFileSync(settings.baseline, "utf8")), run, { tolerance: settings.tolerance ?? 0.15 });
    passed &&= !run.comparison.regressed;
  }
  run.passed = passed;

  const columns = ["scenario", "concurrency", "per_second", "p50_ms", "p95_ms", "p99_ms", "max_ms", "failures", "requests"];
  console.log(table(steps, columns));
  const broken = budgetsResult.checks.filter((c) => c.result === "FAIL");
  console.log(`\nBudgets (${settings.latencyBudgets ? "latency and failures" : "failures only"}): ${broken.length ? `${broken.length} broken` : "all met"}`);
  if (broken.length) console.log(table(broken, ["scenario", "concurrency", "measure", "limit", "value"]));
  if (run.drift) console.log(`\nDrift:\n${table(run.drift, ["scenario", "first_p95_ms", "last_p95_ms", "ratio", "result"])}`);
  if (run.comparison) console.log(`\nAgainst ${settings.baseline}:\n${table(run.comparison.rows, ["step", "p95_change", "rate_change", "result"])}`);
  console.log(`\n${run.statement}\n${passed ? "passed" : "FAILED"}`);
  if (settings.out) writeFileSync(settings.out, `${JSON.stringify(run, null, 1)}\n`);
  return passed ? 0 : 1;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) process.exitCode = await main();
export { parse, combine };
