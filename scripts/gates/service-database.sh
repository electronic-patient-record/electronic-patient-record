#!/bin/sh
# Gate service-database (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../../electronic-patient-record-service-with-rust"
# Every ignored (database-gated) test wherever it lives, every binary even after a failure; DB_URL names the
# database. The FHIR validator tests are `#[ignore]` too, but need java and the jar: the fhir-validator gate runs them.
# EPR_CHAOS_SEED pins the hostile-input sweep (PA-12) so a red run repeats.
export EPR_CHAOS_SEED="${EPR_CHAOS_SEED:-20261028}"
# `cargo test`, one test at a time (they share one database). cargo-nextest is opt-in with EPR_DB_RUNNER=nextest: it
# runs each test in its own process, so a test that sets a process-wide setting cannot reach another, but measured
# here it took 109 s for these 214 tests against 50 s, because every test pays for a new process and a new app boot
# (spec/local-ci.md). The `database` profile in .config/nextest.toml keeps nextest to one test at a time.
if [ "${EPR_DB_RUNNER:-cargo}" = nextest ] && command -v cargo-nextest >/dev/null 2>&1; then
  cargo nextest run --profile database --workspace --locked --no-fail-fast --run-ignored only -E 'not test(fhir_validator_)'
else
  cargo test --workspace --locked --no-fail-fast -- --ignored --test-threads=1 --skip fhir_validator_
fi
