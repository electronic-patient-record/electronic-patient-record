#!/bin/sh
# Gate docs-load-harness (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
# The load harness's own tests (spec/performance-testing.md), against a fake service: no build, no database.
set -eu
cd "$(dirname "$0")/../.."
node --test scripts/load/test_load.mjs
