#!/bin/sh
# Gate ui-unit (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../../electronic-patient-record-ui-with-svelte"
# Arguments go to vitest: CI adds --coverage.
pnpm exec vitest run "$@"
