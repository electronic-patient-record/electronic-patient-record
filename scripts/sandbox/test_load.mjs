// Tests for the sandbox loader (TL-9): `node --test scripts/sandbox/`. The dataset is plainly synthetic, the checks
// catch data that is not, the flag is required, and a load against a fake service makes each patient once.

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { DATASET, load, modulus11, NATIONAL_NUMBER_SYSTEM, parse, problems } from "./load.mjs";

const dataset = JSON.parse(readFileSync(DATASET, "utf8"));

test("the dataset is plainly synthetic: Sandbox names, 999-range numbers that pass modulus 11, past birth dates", () => {
  assert.deepEqual(problems(dataset), []);
  assert.ok(dataset.patients.length >= 20);
  for (const p of dataset.patients) {
    assert.ok(p.patient.names[0].family.startsWith("Sandbox"));
    const n = p.patient.identifiers.find((i) => i.system === NATIONAL_NUMBER_SYSTEM).value;
    assert.ok(n.startsWith("999") && modulus11(n), n);
  }
});

test("modulus 11 accepts a valid number and refuses a bad check digit, a repeat and a wrong length", () => {
  assert.equal(modulus11("9434765919"), true);
  assert.equal(modulus11("9434765918"), false);
  assert.equal(modulus11("9999999999"), false);
  assert.equal(modulus11("943476591"), false);
});

test("the checks catch a real-looking name, a number outside the test range, a repeat and a future birth", () => {
  const one = (over) => ({
    key: "k",
    patient: {
      names: [{ family: "SandboxOak", given: ["Ann"] }],
      birth_date: "1980-01-01",
      identifiers: [{ system: NATIONAL_NUMBER_SYSTEM, value: "9991000003" }],
      ...over,
    },
  });
  const said = (patients) => problems({ patients }, "2026-11-18").join("\n");
  assert.match(said([one({ names: [{ family: "Jones", given: ["Ann"] }] })]), /does not start with Sandbox/);
  assert.match(said([one({ identifiers: [{ system: NATIONAL_NUMBER_SYSTEM, value: "9434765919" }] })]), /999 test range/);
  assert.match(said([one({ identifiers: [{ system: NATIONAL_NUMBER_SYSTEM, value: "9991000004" }] })]), /modulus 11/);
  assert.match(said([one({}), { ...one({}), key: "k2" }]), /repeated/);
  assert.match(said([one({ birth_date: "2030-01-01" })]), /past calendar day/);
  assert.match(problems({ patients: [] }).join(), /no patients/);
});

test("the loader refuses to run without the sandbox flag", () => {
  assert.equal(parse([]).confirmed, false);
  assert.equal(parse(["--yes-this-is-a-sandbox"]).confirmed, true);
  assert.throws(() => parse(["--live"]), /unknown option/);
  const r = spawnSync(process.execPath, [new URL("./load.mjs", import.meta.url).pathname], { encoding: "utf8" });
  assert.equal(r.status, 2);
  assert.match(r.stderr, /refusing/);
});

test("a load makes each patient once with its observations and allergies, and a second load adds nothing", async () => {
  const store = new Map();
  const sent = [];
  const fake = async (url, { method, body }) => {
    const path = new URL(url).pathname;
    const q = new URL(url).searchParams;
    const reply = (status, value) => ({ status, text: async () => JSON.stringify(value) });
    sent.push(`${method} ${path}`);
    if (method === "GET" && path === "/api/patients") {
      const family = q.get("family");
      return reply(200, { results: [...store.values()].filter((p) => p.names[0].family.startsWith(family)).map((patient) => ({ patient })), limit: 50 });
    }
    if (method === "POST" && path === "/api/patients") {
      const id = `p${store.size}`;
      store.set(id, JSON.parse(body).patient);
      return reply(201, { id });
    }
    if (method === "POST" && /\/(observations|allergies)$/.test(path)) {
      const b = JSON.parse(body);
      if (b.observation) assert.equal(b.observation.status, "final");
      return reply(201, {});
    }
    return reply(404, {});
  };
  const quiet = () => {};
  const first = await load(dataset, { url: "http://sandbox.invalid", fetchFn: fake, log: quiet });
  assert.equal(first.created, dataset.patients.length);
  assert.equal(first.skipped, 0);
  const obs = dataset.patients.reduce((n, p) => n + (p.observations ?? []).length, 0);
  assert.equal(first.observations, obs);
  const second = await load(dataset, { url: "http://sandbox.invalid", fetchFn: fake, log: quiet });
  assert.deepEqual(second, { created: 0, skipped: dataset.patients.length, observations: 0, allergies: 0 });
  assert.equal(store.size, dataset.patients.length);
});

test("a refusal from the service stops the load with what it said", async () => {
  const refuse = async () => ({ status: 403, text: async () => '{"error":"forbidden"}' });
  await assert.rejects(load(dataset, { url: "http://x.invalid", fetchFn: refuse, log: () => {} }), /403/);
});
