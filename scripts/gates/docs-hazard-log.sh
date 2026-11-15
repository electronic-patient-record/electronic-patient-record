#!/bin/sh
# Gate docs-hazard-log (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../.."
python3 scripts/check-hazard-log.py --report .
python3 scripts/test_check_hazard_log.py
