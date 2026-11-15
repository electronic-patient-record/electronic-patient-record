#!/bin/sh
# Gate ui-check-tests (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../../electronic-patient-record-ui-with-svelte"
pnpm run check:tests
