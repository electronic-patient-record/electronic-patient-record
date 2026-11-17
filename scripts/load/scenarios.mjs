// What the load harness asks the service to do (spec/performance-testing.md). Each scenario is one thing a user
// or a system does, timed end to end: `op(ctx)` resolves to { status }, the worst status when it is several
// requests. Every request the service answers is audited, so every scenario also measures the audit chain (CQ-18).
//
// The data is synthetic: names are `Load<stamp><letters>`, birth dates are spread so the duplicate check finds
// nothing to refuse, and no identifier is real.

import { answer, open } from "../hl7/send.mjs";

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
export function context(base, { stamp = Date.now().toString(36), mllp = null } = {}) {
  let counter = 0;
  const ctx = {
    base,
    stamp,
    ids: [],
    // The HL7 listener, when the run has one (--mllp host:port): open connections kept for reuse, and the record
    // numbers of the patients the feed registered, for updates.
    mllp,
    feed: { idle: [], mrns: [] },
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

/** An HL7 v2 time (MSH-7, EVN-2) for now, in UTC: the feed reads it to the minute, and the same minute is not older. */
export function hl7Now(at = new Date()) {
  return at.toISOString().replace(/[-:T]/g, "").slice(0, 14);
}

/** An ADT message for synthetic patient `n` with record number `mrn`; `phone` makes an update differ. */
export function adtMessage(stamp, event, n, mrn, control, { phone = "", at = hl7Now() } = {}) {
  const p = newPatient(stamp, n);
  const structure = event === "A28" || event === "A31" ? "ADT_A05" : "ADT_A01";
  return [
    `MSH|^~\\&|LOAD|CLINIC|EPR|HOSP|${at}||ADT^${event}^${structure}|${control}|P|2.5`,
    `EVN|${event}|${at}`,
    `PID|1||${mrn}^^^LOCAL||${p.names[0].family}^${p.names[0].given[0]}||${p.birth_date.replaceAll("-", "")}|F|||||${phone}`,
  ].join("\r");
}

/** Send one message on a pooled MLLP connection: `AA` is 200, any other answer 422 (a failure), no answer throws. */
async function mllpSend(ctx, message) {
  if (!ctx.mllp) throw new Error("this scenario needs the HL7 listener: --mllp host:port");
  const send = ctx.feed.idle.pop() ?? (await open(ctx.mllp));
  let ack;
  try {
    ack = await send(message);
  } catch (e) {
    send.close();
    throw e;
  }
  ctx.feed.idle.push(send);
  return { status: answer(ack).code === "AA" ? 200 : 422, text: ack };
}

async function hl7Register(ctx) {
  const n = ctx.next();
  const mrn = `L${ctx.stamp}${word(n)}`;
  const r = await mllpSend(ctx, adtMessage(ctx.stamp, "A28", n, mrn, `LR${ctx.stamp}${n}`));
  if (r.status === 200) ctx.feed.mrns.push([mrn, n]);
  return r;
}

async function hl7Update(ctx) {
  const [mrn, n] = ctx.feed.mrns[Math.floor(Math.random() * ctx.feed.mrns.length)];
  const k = ctx.next();
  const phone = `01632${String(960000 + (k % 1000)).padStart(6, "0")}`;
  return mllpSend(ctx, adtMessage(ctx.stamp, "A08", n, mrn, `LU${ctx.stamp}${k}`, { phone }));
}

/** Close the pooled HL7 connections at the end of a run. */
export function closeFeed(ctx) {
  for (const send of ctx.feed.idle.splice(0)) send.close();
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
  "hl7-register": {
    what: "An HL7 A28 for a new patient over MLLP: read, matched, created, audited, answered AA (needs --mllp)",
    needs: "mllp",
    op: hl7Register,
  },
  "hl7-update": {
    what: "An HL7 A08 changing a feed patient's phone over MLLP: matched, the order check, updated, AA (needs --mllp)",
    needs: "mllp",
    op: hl7Update,
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
  // Patients the feed registered, for hl7-update; a fifth of the others, at least one.
  if (ctx.mllp) {
    for (let i = 0; i < Math.max(1, Math.ceil(patients / 5)); i++) {
      const r = await hl7Register(ctx);
      if (r.status !== 200) throw new Error(`seeding: the HL7 listener answered ${r.text.slice(0, 300)}`);
    }
  }
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
