# A synthetic sandbox

A synthetic dataset for integrators and training (TL-9), and one command that loads it into a running service. No real
person is in it: every family name starts with `Sandbox`, and every national number is in the 999 range, reserved for
testing and never issued, and passes the modulus-11 check, so the service accepts it as it would a real one.

| File | What it holds |
|---|---|
| `dataset.json` | 24 patients (a name, a birth date, a national number), a heart rate for each, a temperature for every third, and an allergy for every fourth |

```sh
node scripts/sandbox/load.mjs --yes-this-is-a-sandbox --url http://127.0.0.1:1241
```

The loader goes through the API, so every record is checked and audited as any other; it refuses to run without the
flag, and refuses a dataset that is not plainly synthetic (`problems` in `scripts/sandbox/load.mjs`). A second run adds
nothing: a patient whose family name is already on record is skipped, with its observations and allergies. With login
on, set `EPR_SANDBOX_BEARER` to a clinician's token (`scripts/keycloak-dev-token.sh`); the service decides what it may
do. `EPR_SANDBOX_URL` sets the address in place of `--url`.

**Never load this into a live record.** The flag is the only guard: the loader cannot tell a sandbox from a live
service. Tested by `scripts/sandbox/test_load.mjs` (gate `docs-load-harness`) against a fake service; run once by hand
against a real one on a fresh database (2026-11-18: 24 patients, 32 observations, 6 allergies; a second run added
nothing).
