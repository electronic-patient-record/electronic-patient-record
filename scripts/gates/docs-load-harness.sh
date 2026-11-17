#!/bin/sh
# Gate docs-load-harness (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
# The Node tools' own tests, against fakes: no build, no database. The load harness (spec/performance-testing.md)
# and the HL7 sample sender (operations/hl7/samples, IH-11).
set -eu
cd "$(dirname "$0")/../.."
node --test scripts/load/test_load.mjs
node --test scripts/hl7/test_send.mjs
