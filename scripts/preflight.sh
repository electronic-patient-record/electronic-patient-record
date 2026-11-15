#!/bin/sh
# Checks before a full local run (LC-12): the traps that have failed a run here, each stopped before any gate runs,
# with a message that names the fix. `scripts/ci-local.sh --full` runs it; EPR_CI_SKIP_PREFLIGHT=1 skips it.
#
#   sh scripts/preflight.sh    exit 0 when every check passes, 3 when any fails (all are reported)
#
# A check whose tool is missing (no podman, no java) passes here: ci-local.sh already shows that gate as SKIP.
# For scripts/test_preflight.sh, each measurement can be replaced: EPR_PREFLIGHT_VM_TIME (seconds since 1970),
# EPR_PREFLIGHT_VM_MEMORY_MB, EPR_PREFLIGHT_VM_DISK_FREE_GB, EPR_PREFLIGHT_JAR (a path), EPR_PREFLIGHT_PODMAN=1
# (act as if podman answers). Each threshold is a variable too.
set -u
root="$(cd "$(dirname "$0")/.." && pwd)"

max_skew="${EPR_PREFLIGHT_MAX_CLOCK_SKEW:-30}"          # seconds; Keycloak tokens fail beyond a minute or so
min_memory_mb="${EPR_PREFLIGHT_MIN_VM_MEMORY_MB:-16000}" # the API image: two 4.5 GB compiles at once (AGENTS.md)
min_disk_gb="${EPR_PREFLIGHT_MIN_VM_DISK_GB:-20}"        # the image builds; a full VM disk failed them on 2026-11-13

problems=0
problem() { # what is wrong, how to fix it
  printf 'preflight: %s\n  fix: %s\n' "$1" "$2" >&2
  problems=$((problems + 1))
}

in_use() { # port: true when something on this machine is listening there
  python3 -c '
import socket, sys
s = socket.socket()
s.settimeout(0.5)
sys.exit(0 if s.connect_ex(("127.0.0.1", int(sys.argv[1]))) == 0 else 1)
' "$1"
}

suggest_port() {
  python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])'
}

podman_answers() {
  [ "${EPR_PREFLIGHT_PODMAN:-}" = "1" ] && return 0
  command -v podman >/dev/null 2>&1 && podman info >/dev/null 2>&1
}

vm() { # a command inside the Podman VM (macOS and Windows; on Linux podman runs on the host)
  podman machine ssh "$1" 2>/dev/null
}

# The browser tests' port (PW_PORT, default 4183): Playwright refuses to start when it is taken.
pw_port="${PW_PORT:-4183}"
if in_use "$pw_port"; then
  problem "port $pw_port (PW_PORT) is in use, so the browser tests cannot start their server" \
    "PW_PORT=$(suggest_port) sh scripts/ci-local.sh --full   (or stop what listens: lsof -nP -iTCP:$pw_port -sTCP:LISTEN)"
fi

# Postgres: a chosen port must be free, and a given server must answer.
if [ -n "${EPR_CI_PG_PORT:-}" ] && in_use "$EPR_CI_PG_PORT"; then
  problem "port $EPR_CI_PG_PORT (EPR_CI_PG_PORT) is in use, so the throwaway Postgres cannot start there" \
    "unset EPR_CI_PG_PORT to take a free port, or choose another: EPR_CI_PG_PORT=$(suggest_port)"
fi
if [ -n "${DB_URL:-}" ]; then
  hostport="$(printf '%s' "$DB_URL" | sed -E 's|^[a-z]+://([^@/]*@)?([^/?]*).*|\2|')"
  host="${hostport%:*}"
  port="${hostport##*:}"
  [ "$port" != "$hostport" ] || port=5432
  case "$host" in localhost | 127.0.0.1 | '[::1]' | ::1)
    if ! in_use "$port"; then
      problem "DB_URL names a Postgres on port $port of this machine, and nothing listens there" \
        "start that server, or unset DB_URL so the run starts its own throwaway Postgres 18"
    fi
    ;;
  esac
fi

# The FHIR validator jar: fetching it in the middle of a run is 200 MB and can fail offline.
if [ "${EPR_PREFLIGHT_JAR+set}" = set ]; then
  jar="$EPR_PREFLIGHT_JAR"
else
  version="$(sed -n 's/^VERSION=//p' "$root/scripts/fhir-validator.sh")"
  jar="$root/.cache/fhir-validator/$version/validator_cli.jar"
fi
if command -v java >/dev/null 2>&1 && [ ! -f "$jar" ]; then
  problem "the FHIR validator jar is not here ($jar), and the fhir-validator gate would download 200 MB mid-run" \
    "sh scripts/fhir-validator.sh   (fetches it once and checks its SHA-256)"
fi

# The Podman VM: its clock, its memory and its disk.
if podman_answers && { [ "${EPR_PREFLIGHT_PODMAN:-}" = "1" ] || podman machine inspect >/dev/null 2>&1; }; then
  host_time="$(date -u +%s)"
  vm_time="${EPR_PREFLIGHT_VM_TIME:-$(vm 'date -u +%s')}"
  if [ -n "$vm_time" ]; then
    skew=$((vm_time - host_time))
    [ "$skew" -ge 0 ] || skew=$((-skew))
    if [ "$skew" -gt "$max_skew" ]; then
      problem "the Podman VM's clock is ${skew} s from this machine's, so Keycloak tokens look expired (AGENTS.md)" \
        "podman machine ssh \"sudo date -u -s @\$(date -u +%s)\""
    fi
  fi
  memory_mb="${EPR_PREFLIGHT_VM_MEMORY_MB:-$(vm "free -m | awk '/^Mem:/ {print \$2}'")}"
  if [ -n "$memory_mb" ] && [ "$memory_mb" -lt "$min_memory_mb" ]; then
    problem "the Podman VM has ${memory_mb} MB of memory; the API image needs about 16 GB, or rustc is killed (AGENTS.md)" \
      "podman machine stop && podman machine set --memory 32768 && podman machine start   (stops every container)"
  fi
  disk_gb="${EPR_PREFLIGHT_VM_DISK_FREE_GB:-$(vm "df -BG --output=avail /var | tail -1 | tr -dc 0-9")}"
  if [ -n "$disk_gb" ] && [ "$disk_gb" -lt "$min_disk_gb" ]; then
    problem "the Podman VM has ${disk_gb} GB of disk free; the image builds need about ${min_disk_gb} GB" \
      "podman system prune   (removes stopped containers and unused images; read what it lists first)"
  fi
fi

if [ "$problems" -gt 0 ]; then
  printf 'preflight: %s problem(s); nothing was run. Skip with EPR_CI_SKIP_PREFLIGHT=1.\n' "$problems" >&2
  exit 3
fi
echo "preflight: ok"
