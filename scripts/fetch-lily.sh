#!/bin/sh
# Fetch the pinned commit of the Lily Design System monorepo into vendor/lily-design-system (MP-2, MP-3).
#
#   sh scripts/fetch-lily.sh          # clone or move to LILY_COMMIT; prints the directory
#   LILY_SOURCE=~/git/lilydesignsystem/lily-design-system sh scripts/fetch-lily.sh   # from a local clone, faster
#
# Neither Lily's Swift package nor its Kotlin libraries are published yet (checked 2026-11-23), so the mobile apps build
# against this one commit: the Swift package by path, the Kotlin build by includeBuild. Only the two directories the
# apps need are checked out (sparse, depth 1). Moving the pin is a commit of its own that says what changed in Lily.
# When Lily publishes, this script goes (MP-3). See spec/mobile-apps/index.md §5.
set -eu

# A git hook (pre-push runs the gates) sets GIT_DIR and the like to this repository's own. Every git command below must act
# on the Lily checkout only, so drop them: with GIT_DIR set, `git -C vendor/...` would change this repository's remote,
# sparse checkout and shallow list (it did, once, on 2026-11-23).
unset $(git rev-parse --local-env-vars) 2>/dev/null || true

LILY_COMMIT=5df76fa2a2618dbe0095e17f5f77d76489c4bf1f
source="${LILY_SOURCE:-https://github.com/LilyDesignSystem/lily-design-system.git}"
root="$(cd "$(dirname "$0")/.." && pwd)"
# LILY_DIR is for scripts/test_fetch_lily.sh only.
dir="${LILY_DIR:-$root/vendor/lily-design-system}"

if [ "$(git -C "$dir" rev-parse HEAD 2>/dev/null || true)" = "$LILY_COMMIT" ]; then
  echo "$dir"
  exit 0
fi

if [ ! -d "$dir/.git" ]; then
  mkdir -p "$dir"
  git -C "$dir" init -q
  git -C "$dir" remote add origin "$source"
fi
# Never touch a repository other than the checkout's own.
top="$(cd "$dir" && git rev-parse --show-toplevel 2>/dev/null || true)"
if [ "$top" != "$(cd "$dir" && pwd -P)" ]; then
  echo "fetch-lily: $dir is not its own git repository (git sees '$top'); refusing to change it" >&2
  exit 1
fi
git -C "$dir" remote set-url origin "$source"
git -C "$dir" sparse-checkout set --cone lily-design-system-swift lily-design-system-kotlin
git -C "$dir" fetch -q --depth 1 origin "$LILY_COMMIT"
git -C "$dir" -c advice.detachedHead=false checkout -q "$LILY_COMMIT"
test "$(git -C "$dir" rev-parse HEAD)" = "$LILY_COMMIT"
echo "$dir"
