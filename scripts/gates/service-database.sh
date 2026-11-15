#!/bin/sh
# Gate service-database (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../../electronic-patient-record-service-with-rust"
# Every ignored (database-gated) test wherever it lives, every binary even after a failure; DB_URL names the
# database. The FHIR validator tests are `#[ignore]` too, but need java and the jar: the fhir-validator gate runs them.
# EPR_CHAOS_SEED pins the hostile-input sweep (PA-12) so a red run repeats.
EPR_CHAOS_SEED="${EPR_CHAOS_SEED:-20261028}" \
  cargo test --workspace --locked --no-fail-fast -- --ignored --test-threads=1 --skip fhir_validator_
