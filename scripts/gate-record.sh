#!/bin/sh
# A signed local gate record (LC-6). See spec/local-ci.md.
#
#   sh scripts/gate-record.sh write <results.tsv>   write and sign target/ci-local/<commit>.json for HEAD
#   sh scripts/gate-record.sh verify <commit>       check that record's signature, commit and tree
#
# `scripts/ci-local.sh --full` calls `write` after its gates, and `--verify <commit>` calls `verify`. A record is
# self-attested: it says what this machine ran, signed with the author's key. It is not a hosted build, and it is not
# evidence for the hazard log or the safety case (OQ-LC3, whose recommendation is hosted runs only); the record says
# so in its own text.
#
# results.tsv holds one gate a line: name, result, seconds, note, separated by tabs.
# `write` refuses when a tracked file differs from HEAD; untracked files are counted in the record.
# The key is git's `user.signingkey` (EPR_CI_SIGNING_KEY overrides), and `verify` reads git's
# `gpg.ssh.allowedSignersFile` (EPR_CI_ALLOWED_SIGNERS overrides). Both run in the repository of the current directory.
set -eu
namespace=epr-ci-local
statement="Self-attested, not a hosted build. This record says what one machine ran on one commit, signed with its author's key; it is not evidence for the hazard log or the safety case (OQ-LC3)."

repo="$(git rev-parse --show-toplevel)"
dir="$repo/target/ci-local"

die() {
  echo "gate record: $1" >&2
  exit 1
}

version() { # command...: its first line of output, or "not found"
  if command -v "$1" >/dev/null 2>&1; then
    "$@" 2>&1 | head -1
  else
    echo "not found"
  fi
}

write() {
  results="$1"
  [ -f "$results" ] || die "no results file: $results"
  if [ -n "$(git -C "$repo" status --porcelain --untracked-files=no)" ]; then
    die "tracked files differ from HEAD, so no record is written (commit or stash them, then run again)"
  fi
  key="${EPR_CI_SIGNING_KEY:-$(git -C "$repo" config --get user.signingkey || true)}"
  [ -n "$key" ] || die "no signing key (set git's user.signingkey, or EPR_CI_SIGNING_KEY)"
  commit="$(git -C "$repo" rev-parse HEAD)"
  mkdir -p "$dir"
  record="$dir/$commit.json"
  # Written and signed beside the record, and moved into place only when signed, so a failure leaves an earlier
  # record of this commit as it was.
  draft="$dir/.draft-$$.json"
  COMMIT="$commit" \
    TREE="$(git -C "$repo" rev-parse 'HEAD^{tree}')" \
    UNTRACKED="$(git -C "$repo" status --porcelain | grep -c '^??' || true)" \
    STATEMENT="$statement" \
    OS="$(uname -s)" ARCH="$(uname -m)" \
    RUSTC="$(version rustc --version)" CARGO="$(version cargo --version)" \
    NODE="$(version node --version)" PNPM="$(version pnpm --version)" \
    PODMAN="$(version podman --version)" JAVA="$(version java -version)" \
    PYTHON="$(version python3 --version)" \
    python3 - "$results" "$draft" <<'EOF'
import json, os, sys, datetime
gates = []
with open(sys.argv[1], encoding="utf-8") as f:
    for line in f:
        parts = line.rstrip("\n").split("\t")
        if len(parts) < 3:
            continue
        name, result, seconds = parts[:3]
        note = parts[3] if len(parts) > 3 else ""
        gates.append({"gate": name, "result": result,
                      "seconds": int(seconds) if seconds.isdigit() else None, "note": note})
env = os.environ
record = {
    "kind": "local gate record",
    "statement": env["STATEMENT"],
    "commit": env["COMMIT"],
    "tree": env["TREE"],
    "untracked_files": int(env["UNTRACKED"] or 0),
    "written_at": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "host": {"os": env["OS"], "arch": env["ARCH"]},
    "tools": {k: env[k.upper()] for k in ("rustc", "cargo", "node", "pnpm", "podman", "java", "python")},
    "gates": gates,
    "passed": bool(gates) and all(g["result"] == "pass" for g in gates),
}
with open(sys.argv[2], "w", encoding="utf-8") as f:
    json.dump(record, f, indent=2)
    f.write("\n")
EOF
  if ! ssh-keygen -Y sign -q -f "$key" -n "$namespace" "$draft" 2>"$dir/sign.err"; then
    rm -f "$draft" "$draft.sig"
    die "signing failed, so no new record is kept: $(head -1 "$dir/sign.err") (is the key in ssh-agent? ssh-add -l)"
  fi
  mv "$draft.sig" "$record.sig"
  mv "$draft" "$record"
  echo "gate record: $record (signed; self-attested, not a hosted build)"
}

verify() {
  commit="$(git -C "$repo" rev-parse --verify "$1^{commit}" 2>/dev/null)" || die "no such commit: $1"
  record="$dir/$commit.json"
  [ -f "$record" ] || die "no record for $commit (run sh scripts/ci-local.sh --full on that commit)"
  [ -f "$record.sig" ] || die "the record for $commit has no signature"
  signers="${EPR_CI_ALLOWED_SIGNERS:-$(git -C "$repo" config --get gpg.ssh.allowedSignersFile || true)}"
  [ -n "$signers" ] && [ -f "$signers" ] || die "no allowed signers file (set git's gpg.ssh.allowedSignersFile, or EPR_CI_ALLOWED_SIGNERS)"
  principal="$(ssh-keygen -Y find-principals -f "$signers" -s "$record.sig" 2>/dev/null | head -1)"
  [ -n "$principal" ] || die "the signature's key is not in $signers"
  ssh-keygen -Y verify -f "$signers" -I "$principal" -n "$namespace" -s "$record.sig" <"$record" >/dev/null 2>&1 ||
    die "the signature does not match the record: it was changed after signing, or signed for another purpose"
  python3 - "$record" "$commit" "$(git -C "$repo" rev-parse "$commit^{tree}")" "$statement" <<'EOF' || exit 1
import json, sys
r = json.load(open(sys.argv[1], encoding="utf-8"))
problems = []
if r.get("commit") != sys.argv[2]:
    problems.append("it names commit %s" % r.get("commit"))
if r.get("tree") != sys.argv[3]:
    problems.append("its tree is not the commit's")
if r.get("statement") != sys.argv[4]:
    problems.append("its statement is not the self-attested one")
if problems:
    print("gate record: signed, but " + "; ".join(problems), file=sys.stderr)
    sys.exit(1)
gates = r["gates"]
failed = [g["gate"] + " " + g["result"] for g in gates if g["result"] != "pass"]
print("gate record: %s verified, signed by an allowed key; %d gates, %s" % (
    sys.argv[2][:12], len(gates), "all passed" if r["passed"] else "not passed: " + ", ".join(failed)))
print("  " + r["statement"])
EOF
}

case "${1:-}" in
  write) write "${2:?results file}" ;;
  verify) verify "${2:?commit}" ;;
  *)
    echo "usage: sh scripts/gate-record.sh write <results.tsv> | verify <commit>" >&2
    exit 64
    ;;
esac
