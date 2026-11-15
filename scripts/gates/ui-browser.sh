#!/bin/sh
# Gate ui-browser (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../../electronic-patient-record-ui-with-svelte"
# PW_PORT chooses the port (another project may hold the default); ci-local.sh sets 4183.
CI=true pnpm exec playwright test
