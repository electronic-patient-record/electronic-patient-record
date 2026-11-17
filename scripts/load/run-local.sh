#!/bin/sh
# Load test against a release build of the service on a scratch database (spec/performance-testing.md).
#
#   sh scripts/load/run-local.sh [run.mjs options]      e.g. --profile smoke, --profile standard --out results.json
#
# It builds the service in release, makes a scratch database (scripts/scratch-db.sh: DB_URL names a server to
# use, else a throwaway Postgres 18 is started), migrates it, starts the service with login off (`container-dev`)
# on a free port, runs scripts/load/run.mjs against it, then stops the service and drops the database.
# EPR_LOAD_SKIP_BUILD=1 uses the release binary already built; EPR_LOAD_POOL sets DB_MAX_CONNECTIONS (default 20);
# EPR_LOAD_LOG_LEVEL the service's log level (default warn, so logging is not what is measured).
#
# While the harness runs, what the database's busy sessions are waiting on is sampled from pg_stat_activity five
# times a second and counted at the end (target/load/waits.txt): a `Lock advisory` row is a request waiting for the
# audit chain's lock (CQ-18). EPR_LOAD_SAMPLE_WAITS=0 turns it off.
#
# Exit status: run.mjs's (0 passed, 1 a budget broken, 2 usage), or 3 when the service or database could not be
# brought up.
set -eu
root="$(cd "$(dirname "$0")/../.." && pwd)"
service="$root/electronic-patient-record-service-with-rust"
logs="$root/target/load"
mkdir -p "$logs"
. "$root/scripts/scratch-db.sh"

binary="$service/target/release/electronic-patient-record"
if [ "${EPR_LOAD_SKIP_BUILD:-}" != 1 ]; then
  echo "load: building the service (release)" >&2
  (cd "$service" && cargo build --locked --release --quiet --bin electronic-patient-record)
fi
if [ ! -x "$binary" ]; then
  echo "load: no release binary at $binary" >&2
  exit 3
fi

pid=""
sampler=""
stop_all() {
  for p in $sampler $pid; do
    kill "$p" 2>/dev/null || true
    wait "$p" 2>/dev/null || true
  done
  sampler=""
  pid=""
  clean_up
}
trap stop_all EXIT
trap 'stop_all; exit 130' INT TERM

if ! { scratch_server && create_scratch; }; then
  echo "load: $why" >&2
  exit 3
fi
port="$(free_port)"
# The HL7 listener on a free loopback port, for the hl7-* scenarios (passed to run.mjs as --mllp); 64 connections so
# the listener's own limit is not what a 32-worker step measures. EPR_LOAD_MLLP=0 leaves it off.
mllp_args=""
if [ "${EPR_LOAD_MLLP:-1}" = 1 ]; then
  mllp_port="$(free_port)"
  while [ "$mllp_port" = "$port" ]; do mllp_port="$(free_port)"; done
  export EPR_MLLP_BIND="127.0.0.1:$mllp_port" EPR_MLLP_AGENT=mllp:load EPR_MLLP_MAX_CONNECTIONS=64
  mllp_args="--mllp 127.0.0.1:$mllp_port"
fi
export APP_ENV=container-dev PORT="$port" BINDING=127.0.0.1 DB_ALLOW_INSECURE=true \
  DB_MAX_CONNECTIONS="${EPR_LOAD_POOL:-20}" DB_MIN_CONNECTIONS=2 LOG_LEVEL="${EPR_LOAD_LOG_LEVEL:-warn}"
"$binary" db migrate >"$logs/migrate.log" 2>&1 || {
  echo "load: the migration failed: see $logs/migrate.log" >&2
  exit 3
}
"$binary" start >"$logs/service.log" 2>&1 &
pid=$!
base="http://127.0.0.1:$port"
tries=0
until curl -fsS "$base/_health/live" >/dev/null 2>&1; do
  tries=$((tries + 1))
  if [ "$tries" -gt 120 ] || ! kill -0 "$pid" 2>/dev/null; then
    echo "load: the service did not start: see $logs/service.log" >&2
    exit 3
  fi
  sleep 0.5
done

# What the busy sessions wait on, by kind; "CPU" is a session running, not waiting.
if [ "${EPR_LOAD_SAMPLE_WAITS:-1}" = 1 ]; then
  : >"$logs/waits.tsv"
  (
    while :; do
      "$psql" -At -F "$(printf '\t')" -d "$DB_URL" -c "SELECT coalesce(wait_event_type, 'CPU'), coalesce(wait_event, 'running')
        FROM pg_stat_activity WHERE datname = current_database() AND state = 'active' AND pid <> pg_backend_pid()" \
        >>"$logs/waits.tsv" 2>/dev/null || true
      sleep 0.2
    done
  ) &
  sampler=$!
fi

status=0
LOAD_NOTE="${LOAD_NOTE:-release build, container-dev (login off), pool ${DB_MAX_CONNECTIONS}, log ${LOG_LEVEL}, scratch database}" \
  BASE="$base" node "$root/scripts/load/run.mjs" $mllp_args "$@" || status=$?
if [ -n "$sampler" ]; then
  kill "$sampler" 2>/dev/null || true
  wait "$sampler" 2>/dev/null || true
  sampler=""
  {
    echo "Database waits while the harness ran: samples of busy sessions, five a second, by what each waited on."
    echo "\`Lock advisory\` is a request waiting for the audit chain's lock (CQ-18); CPU is a session running."
    sort "$logs/waits.tsv" | uniq -c | sort -rn
  } >"$logs/waits.txt"
  cat "$logs/waits.txt" >&2
fi
exit "$status"
