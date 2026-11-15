#!/bin/sh
# Checks for scripts/preflight.sh (LC-12): each condition broken on purpose must stop the run (exit 3) with a message
# that names the fix, and a good machine must pass. The Podman VM is faked with EPR_PREFLIGHT_* values, and the ports
# are taken by a listener this script starts, so it runs the same with or without podman, java or Postgres.
set -eu
unset DB_URL EPR_CI_PG_PORT PW_PORT EPR_PREFLIGHT_VM_TIME EPR_PREFLIGHT_VM_MEMORY_MB EPR_PREFLIGHT_VM_DISK_FREE_GB \
  EPR_PREFLIGHT_JAR EPR_PREFLIGHT_PODMAN 2>/dev/null || true
root="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d)"
listener=""
trap '[ -z "$listener" ] || { kill "$listener"; wait "$listener" || true; } 2>/dev/null; rm -rf "$work"' EXIT
fails=0

port_file="$work/port"
python3 -c '
import socket, sys, time
s = socket.socket()
s.bind(("127.0.0.1", 0))
s.listen()
open(sys.argv[1], "w").write(str(s.getsockname()[1]))
time.sleep(600)
' "$port_file" &
listener=$!
for _ in 1 2 3 4 5 6 7 8 9 10; do [ -s "$port_file" ] && break; sleep 0.2; done
taken="$(cat "$port_file")"
free="$(python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])')"

# A java on PATH, so the jar check runs here as on a machine with java.
mkdir -p "$work/bin"
printf '#!/bin/sh\nexit 0\n' >"$work/bin/java"
chmod +x "$work/bin/java"
touch "$work/validator_cli.jar"

run() { # env assignments...; prints "<exit> <stderr>"
  out="$(env PATH="$work/bin:$PATH" PW_PORT="$free" EPR_PREFLIGHT_JAR="$work/validator_cli.jar" \
    EPR_PREFLIGHT_PODMAN=1 EPR_PREFLIGHT_VM_TIME="$(date -u +%s)" EPR_PREFLIGHT_VM_MEMORY_MB=32000 \
    EPR_PREFLIGHT_VM_DISK_FREE_GB=100 "$@" sh "$root/scripts/preflight.sh" 2>&1)" && code=0 || code=$?
  printf '%s %s' "$code" "$out"
}

expect() { # what, text the fix must contain, env assignments...
  what="$1"
  fix="$2"
  shift 2
  result="$(run "$@")"
  case "$result" in
    "3 "*"fix: "*"$fix"*) echo "ok: $what" ;;
    *)
      echo "FAIL: $what: expected exit 3 and a fix naming '$fix'; got: $result" >&2
      fails=$((fails + 1))
      ;;
  esac
}

result="$(run)"
case "$result" in
  "0 preflight: ok") echo "ok: a good machine passes" ;;
  *) echo "FAIL: a good machine passes: got: $result" >&2; fails=$((fails + 1)) ;;
esac

expect "PW_PORT in use" "PW_PORT=" PW_PORT="$taken"
expect "EPR_CI_PG_PORT in use" "unset EPR_CI_PG_PORT" EPR_CI_PG_PORT="$taken"
expect "DB_URL names a local port with nothing on it" "unset DB_URL" \
  DB_URL="postgres://loco:loco@127.0.0.1:$free/electronic_patient_record_test"
expect "the validator jar is missing" "sh scripts/fhir-validator.sh" EPR_PREFLIGHT_JAR="$work/missing.jar"
expect "the VM clock is ahead" "sudo date -u -s" EPR_PREFLIGHT_VM_TIME="$(($(date -u +%s) + 300))"
expect "the VM clock is behind" "sudo date -u -s" EPR_PREFLIGHT_VM_TIME="$(($(date -u +%s) - 300))"
expect "the VM has 12 GB of memory" "podman machine set --memory" EPR_PREFLIGHT_VM_MEMORY_MB=11920
expect "the VM disk is nearly full" "podman system prune" EPR_PREFLIGHT_VM_DISK_FREE_GB=3

result="$(run DB_URL="postgres://loco:loco@127.0.0.1:$taken/x")"
case "$result" in
  "0 "*) echo "ok: DB_URL naming a listening port passes" ;;
  *) echo "FAIL: DB_URL naming a listening port passes: got: $result" >&2; fails=$((fails + 1)) ;;
esac

result="$(run PW_PORT="$taken" EPR_PREFLIGHT_VM_MEMORY_MB=11920)"
case "$result" in
  "3 "*"PW_PORT"*"--memory"*"2 problem(s)"*) echo "ok: every problem is reported, not only the first" ;;
  *) echo "FAIL: every problem is reported: got: $result" >&2; fails=$((fails + 1)) ;;
esac

if [ "$fails" -ne 0 ]; then
  echo "$fails check(s) failed" >&2
  exit 1
fi
echo "all preflight checks passed"
