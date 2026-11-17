#!/bin/sh
# Mutation testing (CQ-30): run cargo-mutants on one crate's examined files (its .cargo/mutants.toml), or only on the
# lines a branch changed.
#
#   sh scripts/mutants.sh matcher|service [--in-diff <base>] [-j N]
#
# Not a CI gate: a full run takes hours on the service (PR-8 declined a scheduled run). Run it by hand before a
# release and after changing an examined file; `--in-diff main` tests only the mutants on lines changed since main,
# which is minutes. Each missed mutant is killed by a new test or listed, with its reason, in
# docs/maintainability/mutation-testing.md. Output goes to mutants.out/ in the crate (git-ignored).
set -eu
root="$(cd "$(dirname "$0")/.." && pwd)"
case "${1:-}" in
  matcher) crate="$root/electronic-patient-record-matcher-with-rust" ;;
  service) crate="$root/electronic-patient-record-service-with-rust" ;;
  *) echo "usage: sh scripts/mutants.sh matcher|service [--in-diff <base>] [-j N]" >&2; exit 64 ;;
esac
shift
jobs=4
diff=""
while [ $# -gt 0 ]; do
  case "$1" in
    --in-diff) diff="$2"; shift 2 ;;
    -j) jobs="$2"; shift 2 ;;
    *) echo "mutants: unknown argument $1" >&2; exit 64 ;;
  esac
done
command -v cargo-mutants >/dev/null 2>&1 || { echo "mutants: cargo install cargo-mutants" >&2; exit 69; }
cd "$crate"
# Line tables only: each job builds its own copy of the crate, and full debug info is most of the disk it takes.
export CARGO_PROFILE_DEV_DEBUG=line-tables-only
if [ -n "$diff" ]; then
  patch="$(mktemp)"
  trap 'rm -f "$patch"' EXIT
  git diff "$diff" -- . >"$patch"
  cargo mutants -j "$jobs" --in-diff "$patch"
else
  cargo mutants -j "$jobs"
fi
