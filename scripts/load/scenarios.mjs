// What the load harness asks the service to do (spec/performance-testing.md). Each scenario is one thing a user
// or a system does, timed end to end: `op(ctx)` resolves to { status }, the worst status when it is several
// requests. Every request the service answers is audited, so every scenario also measures the audit chain (CQ-18).
//
// The data is synthetic: names are `Load<stamp><letters>`, birth dates are spread so the duplicate check finds
// nothing to refuse, and no identifier is real.

const JSON_HEADERS = { "content-type": "application/json" };
const letters = "abcdefghijklmnopqrstuvwxyz";

/** Letters for `n`, so each synthetic name is distinct: 0 is "a", 25 "z", 26 "ba". */
export function word(n) {
  let s = "";
  do {
    s = letters[n % 26] + s;
    n = Math.floor(n / 26);
  } while (n > 0);
  return s;
}

/** A synthetic patient for registration number `n`. */
export function newPatient(stamp, n) {
  const day = String((n % 28) + 1).padStart(2, "0");
  const month = String((Math.floor(n / 28) % 12) + 1).padStart(2, "0");
  const year = 1920 + (Math.floor(n / 336) % 100);
  return {
    names: [{ family: `Load${stamp}${word(n)}`, given: [`G${word(n * 7 + 3)}`] }],
    birth_date: `${year}-${month}-${day}`,
  };
}

function observation(n) {
  return {
    observation: {
      code: "8867-4",
      value: 60 + (n % 50),
      unit: "/min",
      range: { low: 60, high: 100 },
      effective_at: new Date(Date.now() - 3600_000 - (n % 1000) * 1000).toISOString(),
      status: "final",
    },
    reason: "load test",
  };
}

function allergy(n) {
  return {
    allergy: { substance: `Load substance ${word(n)}`, status: "active" },
    reason: "load test",
  };
}

/** The context the scenarios share: where the service is, a counter for new records, and the seeded patients. */
export function context(base, { stamp = Date.now().toString(36) } = {}) {
  let counter = 0;
  const ctx = {
    base,
    stamp,
    ids: [],
    next: () => counter++,
    pick: () => ctx.ids[Math.floor(Math.random() * ctx.ids.length)],
    get: (path) => fetch(`${base}${path}`, { headers: { accept: "application/json" } }).then(drain),
    send: (method, path, body) =>
      fetch(`${base}${path}`, { method, headers: JSON_HEADERS, body: JSON.stringify(body) }).then(drain),
  };
  return ctx;
}

/** Read the whole body, so a request's time includes it and the connection is free for the next. */
async function drain(r) {
  const text = await r.text();
  return { status: r.status, text };
}

async function register(ctx) {
  const r = await ctx.send("POST", "/api/patients", { patient: newPatient(ctx.stamp, ctx.next()), reason: "load test" });
  if (r.status === 201) {
    try {
      r.id = JSON.parse(r.text).id;
    } catch {
      /* no id: the status says it failed */
    }
  }
  return r;
}

/** The reads the chart page makes for one patient, side by side as the page does; the time is the slowest. */
export const CHART_PARTS = [
  "",
  "/allergies",
  "/observations",
  "/results",
  "/referrals",
  "/adjustments",
  "/watch-list",
  "/death",
];

async function chartOpen(ctx) {
  const id = ctx.pick();
  const parts = await Promise.all(CHART_PARTS.map((p) => ctx.get(`/api/patients/${id}${p}`)));
  return { status: Math.max(...parts.map((p) => p.status)) };
}

export const SCENARIOS = {
  register: {
    what: "Register a patient (POST /api/patients): validation, the duplicate check, the write and its audit row",
    op: register,
  },
  read: {
    what: "Open a patient's identity record (GET /api/patients/{id})",
    op: (ctx) => ctx.get(`/api/patients/${ctx.pick()}`),
  },
  search: {
    what: "Search by family-name prefix (GET /api/patients?family=), at most 50 back",
    op: (ctx) => ctx.get(`/api/patients?family=Load${ctx.stamp}${letters[Math.floor(Math.random() * 26)]}`),
  },
  "chart-open": {
    what: `Open a chart: the ${CHART_PARTS.length} reads the chart page makes, side by side`,
    op: chartOpen,
  },
  "fhir-read": {
    what: "Read a FHIR Patient (GET /fhir/R4/Patient/{id})",
    op: (ctx) => ctx.get(`/fhir/R4/Patient/${ctx.pick()}`),
  },
  "fhir-search": {
    what: "Search FHIR Patients by family (GET /fhir/R4/Patient?family=)",
    op: (ctx) => ctx.get(`/fhir/R4/Patient?family=Load${ctx.stamp}${letters[Math.floor(Math.random() * 26)]}`),
  },
  compare: {
    what: "Compare two supplied records (POST /api/patients/compare): the matcher, and an audit row",
    op: (ctx) => {
      const n = ctx.next();
      return ctx.send("POST", "/api/patients/compare", { a: newPatient(ctx.stamp, n), b: newPatient(ctx.stamp, n + 1) });
    },
  },
  "record-observation": {
    what: "Record a vital sign (POST /api/patients/{id}/observations)",
    op: (ctx) => ctx.send("POST", `/api/patients/${ctx.pick()}/observations`, observation(ctx.next())),
  },
  "record-allergy": {
    what: "Record an allergy (POST /api/patients/{id}/allergies)",
    op: (ctx) => ctx.send("POST", `/api/patients/${ctx.pick()}/allergies`, allergy(ctx.next())),
  },
};

/** A day's mix, by weight: mostly reading, some writing. A guess, not a measured clinic; spec/performance-testing.md. */
export const MIX = [
  ["chart-open", 30],
  ["search", 20],
  ["read", 15],
  ["fhir-read", 10],
  ["record-observation", 10],
  ["register", 5],
  ["record-allergy", 5],
  ["compare", 5],
];

SCENARIOS.mixed = {
  what: `A mix: ${MIX.map(([n, w]) => `${n} ${w}%`).join(", ")}`,
  op: (ctx) => {
    let r = Math.random() * MIX.reduce((s, [, w]) => s + w, 0);
    for (const [name, w] of MIX) {
      r -= w;
      if (r < 0) return SCENARIOS[name].op(ctx);
    }
    return SCENARIOS[MIX[0][0]].op(ctx);
  },
};

/**
 * Patients to read, each with an allergy and three observations so a chart has something in it. Seeding is not
 * timed. Fails when the service refuses a seed, since every later number would be about refusals.
 */
export async function seed(ctx, patients) {
  for (let i = 0; i < patients; i++) {
    const r = await register(ctx);
    if (r.status !== 201 || !r.id) throw new Error(`seeding: registration answered ${r.status}: ${r.text.slice(0, 300)}`);
    ctx.ids.push(r.id);
    const a = await ctx.send("POST", `/api/patients/${r.id}/allergies`, allergy(ctx.next()));
    if (a.status >= 300) throw new Error(`seeding: allergy answered ${a.status}: ${a.text.slice(0, 300)}`);
    for (let k = 0; k < 3; k++) {
      const o = await ctx.send("POST", `/api/patients/${r.id}/observations`, observation(ctx.next()));
      if (o.status >= 300) throw new Error(`seeding: observation answered ${o.status}: ${o.text.slice(0, 300)}`);
    }
  }
}
