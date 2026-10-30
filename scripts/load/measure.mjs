// Load measurement for the service (OS-5). No dependencies: Node 26 `fetch` only.
//
//   BASE=http://127.0.0.1:5190 node scripts/load/measure.mjs [seconds-per-step]
//
// The service must run with login off (the `container-dev` profile) against a test database.
// It registers patients with unique names and birth dates (so the duplicate check finds nothing
// to refuse), then reads and searches them. Every request is audited by the service; the audit
// chain's single advisory lock (CQ-18) is the thing the write numbers measure.
const BASE = process.env.BASE ?? "http://127.0.0.1:5190";
const SECONDS = Number(process.argv[2] ?? 10);
const LEVELS = (process.env.CONCURRENCY ?? "1,8,32").split(",").map(Number);
const JSON_HEADERS = { "content-type": "application/json" };

let counter = 0;
const stamp = Date.now().toString(36);
const letters = "abcdefghijklmnopqrstuvwxyz";
function word(n) {
  let s = "";
  do { s = letters[n % 26] + s; n = Math.floor(n / 26); } while (n > 0);
  return s;
}
function newPatient() {
  const n = counter++;
  const day = String((n % 28) + 1).padStart(2, "0");
  const month = String((Math.floor(n / 28) % 12) + 1).padStart(2, "0");
  const year = 1920 + (Math.floor(n / 336) % 100);
  return {
    patient: {
      names: [{ family: `Load${stamp}${word(n)}`, given: [`G${word(n * 7 + 3)}`] }],
      birth_date: `${year}-${month}-${day}`,
    },
    reason: "load measurement",
  };
}

const create = async () => {
  const r = await fetch(`${BASE}/api/patients`, { method: "POST", headers: JSON_HEADERS, body: JSON.stringify(newPatient()) });
  const body = await r.json().catch(() => ({}));
  return { status: r.status, id: body.id };
};

async function step(name, concurrency, seconds, op) {
  const latencies = [];
  const statuses = {};
  const deadline = performance.now() + seconds * 1000;
  const started = performance.now();
  await Promise.all(Array.from({ length: concurrency }, async () => {
    while (performance.now() < deadline) {
      const t = performance.now();
      let status;
      try { status = (await op()).status; } catch { status = "error"; }
      latencies.push(performance.now() - t);
      statuses[status] = (statuses[status] ?? 0) + 1;
    }
  }));
  const elapsed = (performance.now() - started) / 1000;
  latencies.sort((a, b) => a - b);
  const q = (p) => latencies[Math.min(latencies.length - 1, Math.floor(p * latencies.length))];
  return {
    name, concurrency, requests: latencies.length,
    per_second: +(latencies.length / elapsed).toFixed(1),
    p50_ms: +q(0.5).toFixed(1), p95_ms: +q(0.95).toFixed(1), p99_ms: +q(0.99).toFixed(1),
    statuses,
  };
}

// Seed some patients to read and search, outside the timed steps.
const ids = [];
for (let i = 0; i < 200; i++) { const r = await create(); if (r.id) ids.push(r.id); }
const pick = () => ids[Math.floor(Math.random() * ids.length)];

const results = [];
for (const c of LEVELS) {
  results.push(await step("write (register, audited)", c, SECONDS, create));
  results.push(await step("read (chart open, audited)", c, SECONDS, () => fetch(`${BASE}/api/patients/${pick()}`)));
  results.push(await step("search (family prefix, audited)", c, SECONDS, () => fetch(`${BASE}/api/patients?family=Load${stamp}a`)));
}
console.log(JSON.stringify({ base: BASE, seconds_per_step: SECONDS, seeded: ids.length, results }, null, 1));
