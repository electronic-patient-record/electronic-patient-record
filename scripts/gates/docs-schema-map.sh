#!/bin/sh
# Gate docs-schema-map (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../.."
python3 scripts/generate-schema-map.py --check
