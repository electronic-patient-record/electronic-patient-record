#!/bin/sh
# Gate container-ui (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../../."
# The tag is the first argument: CI passes epr-ui:ci.
podman build -f electronic-patient-record-ui-with-svelte/Containerfile -t "${1:-epr-ui:local}" .
