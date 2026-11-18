// A sandbox for integrators and training (TL-9): load the synthetic patients in `operations/sandbox/dataset.json`
// into a running service, through its API, so every record is checked and audited as any other.
//
//   node scripts/sandbox/load.mjs --yes-this-is-a-sandbox [--url http://127.0.0.1:1241]
//
// With login on, set EPR_SANDBOX_BEARER to a clinician's token (scripts/keycloak-dev-token.sh); the service decides
// what it may do. A second run adds nothing: a patient whose family name is already on record is skipped, with its
// observations and allergies. The data is synthetic (see `problems`): it must never be loaded into a live record, so
// the script refuses to run without the flag.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
export const DATASET = join(here, "../../operations/sandbox/dataset.json");
export const NATIONAL_NUMBER_SYSTEM = "https://fhir.nhs.uk/Id/nhs-number";
const FLAG = "--yes-this-is-a-sandbox";

/** Whether `value` passes the national number's modulus-11 check and is not one digit ten times. */
export function modulus11(value) {
  if (!/^\d{10}$/.test(value) || /^(\d)\1{9}$/.test(value)) return false;
  const sum = [...value.slice(0, 9)].reduce((s, d, i) => s + Number(d) * (10 - i), 0);
  const check = 11 - (sum % 11);
  if (check === 10) return false;
  return (check === 11 ? 0 : check) === Number(value[9]);
}

/**
 * What makes the dataset not plainly synthetic, or not loadable: a family name that does not start with `Sandbox`; a
 * national number outside the 999 range (reserved for testing, never issued) or failing modulus 11; a repeated
 * family, key or number; a birth date that is not a past calendar day.
 */
export function problems(dataset, today = new Date().toISOString().slice(0, 10)) {
  const out = [];
  const seen = { key: new Set(), family: new Set(), number: new Set() };
  const once = (kind, value, at) => {
    if (seen[kind].has(value)) out.push(`${at}: ${kind} ${value} is repeated`);
    seen[kind].add(value);
  };
  for (const [i, entry] of (dataset.patients ?? []).entries()) {
    const at = entry.key ?? `patient ${i}`;
    once("key", entry.key, at);
    const names = entry.patient?.names ?? [];
    if (names.length === 0) out.push(`${at}: no name`);
    for (const n of names) {
      if (!/^Sandbox[A-Z][a-z]+$/.test(n.family ?? ""))
        out.push(`${at}: family name ${n.family} does not start with Sandbox`);
      else once("family", n.family, at);
    }
    for (const id of entry.patient?.identifiers ?? []) {
      if (id.system !== NATIONAL_NUMBER_SYSTEM) continue;
      if (!id.value.startsWith("999")) out.push(`${at}: national number ${id.value} is not in the 999 test range`);
      if (!modulus11(id.value)) out.push(`${at}: national number ${id.value} fails modulus 11`);
      once("number", id.value, at);
    }
    const born = entry.patient?.birth_date ?? "";
    const valid = /^\d{4}-\d{2}-\d{2}$/.test(born) && !Number.isNaN(Date.parse(`${born}T00:00:00Z`));
    if (!valid || born >= today) out.push(`${at}: birth date ${born} is not a past calendar day`);
  }
  if (out.length === 0 && (dataset.patients ?? []).length === 0) out.push("the dataset has no patients");
  return out;
}

/** An observation as the service takes it: measured `hoursAgo` hours before `now`. */
export function observationBody(o, now, hoursAgo) {
  return {
    observation: {
      ...o,
      effective_at: new Date(now.getTime() - hoursAgo * 3_600_000).toISOString(),
      status: "final",
    },
    reason: "sandbox",
  };
}

/** Parse the command line: the flag is required. */
export function parse(argv) {
  const opts = { url: process.env.EPR_SANDBOX_URL || "http://127.0.0.1:1241", confirmed: false };
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === FLAG) opts.confirmed = true;
    else if (argv[i] === "--url") opts.url = argv[++i];
    else throw new Error(`unknown option ${argv[i]}`);
  }
  return opts;
}

/** Load `dataset` into the service at `url`; `fetchFn` is `fetch` or a fake. Returns what it did. */
export async function load(dataset, { url, bearer, fetchFn = fetch, now = new Date(), log = console.log }) {
  const headers = { "content-type": "application/json" };
  if (bearer) headers.authorization = `Bearer ${bearer}`;
  const call = async (method, path, body) => {
    const r = await fetchFn(`${url}${path}`, { method, headers, body: body ? JSON.stringify(body) : undefined });
    const text = await r.text();
    if (r.status >= 300) throw new Error(`${method} ${path}: ${r.status} ${text.slice(0, 300)}`);
    return text ? JSON.parse(text) : null;
  };
  const done = { created: 0, skipped: 0, observations: 0, allergies: 0 };
  for (const entry of dataset.patients) {
    const family = entry.patient.names[0].family;
    const found = await call("GET", `/api/patients?family=${encodeURIComponent(family)}`);
    if ((found?.results ?? []).some((r) => r.patient?.names?.some((n) => n.family === family))) {
      done.skipped++;
      continue;
    }
    const made = await call("POST", "/api/patients", { patient: entry.patient, reason: "sandbox" });
    done.created++;
    for (const [i, o] of (entry.observations ?? []).entries()) {
      await call("POST", `/api/patients/${made.id}/observations`, observationBody(o, now, i + 1));
      done.observations++;
    }
    for (const a of entry.allergies ?? []) {
      await call("POST", `/api/patients/${made.id}/allergies`, { allergy: a, reason: "sandbox" });
      done.allergies++;
    }
  }
  log(
    `sandbox: ${done.created} patients made, ${done.skipped} already there, ${done.observations} observations, ${done.allergies} allergies`,
  );
  return done;
}

async function main() {
  const opts = parse(process.argv.slice(2));
  if (!opts.confirmed) {
    console.error(`refusing: the sandbox is synthetic data and must never reach a live record. Pass ${FLAG}.`);
    process.exit(2);
  }
  const dataset = JSON.parse(readFileSync(DATASET, "utf8"));
  const wrong = problems(dataset);
  if (wrong.length) {
    console.error(`refusing: the dataset is not plainly synthetic:\n${wrong.join("\n")}`);
    process.exit(2);
  }
  await load(dataset, { url: opts.url, bearer: process.env.EPR_SANDBOX_BEARER });
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  main().catch((e) => {
    console.error(`sandbox: ${e.message}`);
    process.exit(1);
  });
}
