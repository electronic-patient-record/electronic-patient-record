#!/bin/sh
# Gate docs-status (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../.."
sh scripts/generate-status.sh --check
python3 scripts/generate-requirements-trace.py --check
python3 scripts/generate-measures.py --check
