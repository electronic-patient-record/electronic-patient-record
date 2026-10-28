#!/bin/sh
# Use the versioned hooks in .githooks/ for this clone (sets core.hooksPath, local config only).
set -eu
cd "$(git rev-parse --show-toplevel)"
git config core.hooksPath .githooks
echo "hooks installed: git will run .githooks/pre-commit (skip once with --no-verify)"
