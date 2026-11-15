#!/bin/sh
# Gate fhir-validator (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../../electronic-patient-record-service-with-rust"
# The official HL7 validator on every resource kind the API writes (needs java; the jar is fetched once, pinned).
FHIR_VALIDATOR_JAR="$(sh ../scripts/fhir-validator.sh)" cargo test --test fhir_validator -- --ignored --test-threads=1
