#!/bin/sh
# Fetch the official HL7 FHIR validator (validator_cli.jar, Apache-2.0, hapifhir/org.hl7.fhir.core), pinned
# to one release and checked against the SHA-256 GitHub publishes for that release asset. Prints the jar
# path on stdout. The jar is about 200 MB and is kept under .cache/ (git-ignored), never committed.
#
#   FHIR_VALIDATOR_JAR=$(sh scripts/fhir-validator.sh)
#   FHIR_VALIDATOR_JAR=$FHIR_VALIDATOR_JAR cargo test --test fhir_validator -- --ignored
#
# Bump VERSION and SHA256 together; the digest comes from
#   gh api repos/hapifhir/org.hl7.fhir.core/releases/tags/<version> -q '.assets[]|select(.name=="validator_cli.jar").digest'
set -eu
VERSION=7.0.3
SHA256=57aa43fb40cec5de41def81c927725fd85908eeb71509b7f9313e04d9b8ec79b
root="$(cd "$(dirname "$0")/.." && pwd)"
dir="$root/.cache/fhir-validator/$VERSION"
jar="$dir/validator_cli.jar"
sum() { if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1; }
if [ ! -f "$jar" ] || [ "$(sum "$jar")" != "$SHA256" ]; then
  mkdir -p "$dir"
  tmp="$jar.part"
  curl -fL --retry 3 -o "$tmp" \
    "https://github.com/hapifhir/org.hl7.fhir.core/releases/download/$VERSION/validator_cli.jar" >&2
  if [ "$(sum "$tmp")" != "$SHA256" ]; then
    rm -f "$tmp"
    echo "validator_cli.jar $VERSION: SHA-256 does not match the pinned digest" >&2
    exit 1
  fi
  mv "$tmp" "$jar"
fi
echo "$jar"
