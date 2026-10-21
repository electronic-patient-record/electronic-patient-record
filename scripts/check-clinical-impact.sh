#!/bin/sh
# A change labelled `clinical-impact` must touch the hazard log (task AS-1).
#   sh scripts/check-clinical-impact.sh <base-ref>     e.g. origin/main
# It lists the files changed between the base and HEAD and fails unless the hazard log is among them.
# The label is checked by the workflow; this script only answers "did the log change?".
set -eu
base=${1:?usage: check-clinical-impact.sh <base-ref>}
log=governance/clinical-safety/hazard-log.csv
if git diff --name-only "$base"...HEAD | grep -qx "$log"; then
  echo "ok: $log is part of this change"
else
  echo "A change labelled clinical-impact must update $log (add or revise the hazard it touches)." >&2
  exit 1
fi
