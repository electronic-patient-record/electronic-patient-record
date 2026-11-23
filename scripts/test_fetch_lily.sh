#!/bin/sh
# Checks for scripts/fetch-lily.sh: run as a git hook runs it, with GIT_DIR naming another repository, it changes nothing
# in that repository (remote, sparse checkout, shallow list, FETCH_HEAD, core.bare). No network: the source is a path
# that does not exist, so the fetch fails after the steps that once changed the wrong repository.
set -eu
root="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
fails=0

unset $(git rev-parse --local-env-vars) 2>/dev/null || true
git init -q "$work/host"
git -C "$work/host" remote add origin https://example.invalid/host.git
before="$(git -C "$work/host" config --list --show-origin | sort)"

status=0
GIT_DIR="$work/host/.git" GIT_WORK_TREE="$work/host" LILY_DIR="$work/lily" LILY_SOURCE="$work/missing" \
  sh "$root/scripts/fetch-lily.sh" >/dev/null 2>&1 || status=$?

after="$(git -C "$work/host" config --list --show-origin | sort)"
check() { # what, condition result
  if [ "$2" = yes ]; then echo "ok: $1"; else echo "FAIL: $1" >&2; fails=$((fails + 1)); fi
}
check "the fetch from a missing source fails" "$([ "$status" -ne 0 ] && echo yes || echo no)"
check "the host repository's config is unchanged" "$([ "$before" = "$after" ] && echo yes || echo no)"
check "the host is not made shallow" "$([ ! -e "$work/host/.git/shallow" ] && echo yes || echo no)"
check "the host has no FETCH_HEAD" "$([ ! -e "$work/host/.git/FETCH_HEAD" ] && echo yes || echo no)"
check "the host has no sparse-checkout file" "$([ ! -e "$work/host/.git/info/sparse-checkout" ] && echo yes || echo no)"
check "the Lily directory got its own repository" "$([ -d "$work/lily/.git" ] && echo yes || echo no)"

if [ "$fails" -ne 0 ]; then
  echo "$fails check(s) failed" >&2
  exit 1
fi
echo "all fetch-lily checks passed"
