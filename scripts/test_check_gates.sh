#!/bin/sh
# Checks for scripts/check-gates.py (LC-3): the real workflow passes, and a copy with an inline gate added, a call to
# a gate script that does not exist, or a listed step removed fails.
set -eu
root="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
fails=0
wf="$root/.github/workflows/ci.yml"

expect() { # what, 0 or 1, workflow file
  if python3 "$root/scripts/check-gates.py" "$3" >"$work/out" 2>&1; then got=0; else got=1; fi
  if [ "$got" = "$2" ]; then
    echo "ok: $1"
  else
    echo "FAIL: $1: expected exit $2, got $got: $(cat "$work/out")" >&2
    fails=$((fails + 1))
  fi
}

expect "the workflow passes" 0 "$wf"
awk '{ print } $0 == "      - run: sh scripts/gates/matcher-fmt.sh" { print "      - run: cargo test --release" }' "$wf" >"$work/inline.yml"
grep -qx '      - run: cargo test --release' "$work/inline.yml" || { echo "FAIL: could not add an inline gate" >&2; fails=$((fails + 1)); }
expect "an inline gate fails" 1 "$work/inline.yml"
sed 's|scripts/gates/matcher-fmt.sh|scripts/gates/no-such-gate.sh|' "$wf" >"$work/missing.yml"
expect "a call to a missing gate script fails" 1 "$work/missing.yml"
grep -v 'pipx install podman-compose' "$wf" >"$work/stale.yml"
expect "a listed step that is gone fails (the list stays true)" 1 "$work/stale.yml"

if [ "$fails" -ne 0 ]; then
  echo "$fails check(s) failed" >&2
  exit 1
fi
echo "all check-gates checks passed"
