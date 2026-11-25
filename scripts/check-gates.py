#!/usr/bin/env python3
"""One definition per gate (LC-3).

Every `run:` step in .github/workflows/ci.yml either calls a gate script, `sh scripts/gates/<name>.sh`, which
scripts/ci-local.sh calls too, or is listed below with the reason it is not a gate: setup, or a check that only GitHub
can run, or a gate not moved yet. A new inline `run:` gate fails here until it is a script or listed with a reason.

Run from anywhere in the repository, with no argument or a workflow file to check instead (the test does);
exit 0 when every step is accounted for.
"""

import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
WORKFLOW = os.path.join(ROOT, ".github", "workflows", "ci.yml")
LOCAL = os.path.join(ROOT, "scripts", "ci-local.sh")
GATES = os.path.join(ROOT, "scripts", "gates")

# (job, start of the step's command): why the step is not a gate script.
NOT_GATES = {
    ("changes", 'case "$EVENT" in'): "setup: decides which jobs run (LC-7)",
    ("ui", "pnpm install --frozen-lockfile"): "setup: installs the UI's packages",
    ("ui", "pnpm exec playwright install"): "setup: installs the browser",
    ("container", "pipx install podman-compose"): "setup: installs podman-compose",
    ("secrets", "curl -sSfL -o gitleaks.tgz"): "setup: installs gitleaks, pinned by checksum",
    ("secrets", "./gitleaks git"): "GitHub only: the secret scan of the full history",
    ("hazard-log", "sh scripts/check-clinical-impact.sh"): "GitHub only: reads the pull request's label",
    ("hazard-signoff", "python3 scripts/check-hazard-log.py --base"): "GitHub only: reads the pull request's author",
    ("hazard-signoff", "python3 scripts/check-hazard-log.py --touched"): "not a gate: a hint in the job's summary, which "
                                                                        "fails nothing by what it lists (PR-2)",
    ("ci-ok", "echo '${{ toJSON(needs) }}'"): "GitHub only: the aggregate of the jobs",
    ("fresh-clone", 'want="$(sed'): "GitHub only: compares the pin with CI's own toolchain",
    ("pnpm-audit", "pnpm audit"): "GitHub only: needs the advisory service; not a local gate",
    ("api-db", "PGPASSWORD=loco psql"): "setup: applies ci/postgres-init to the job's database; locally "
                                         "scripts/scratch-db.sh applies it to each scratch database",
    ("container", "set -u"): "CI only: brings up compose.yaml on a clean runner and checks the API refuses to start "
                            "with no keys; locally its ports and container names are often taken",
    ("performance", "EPR_LOAD_SKIP_BUILD=1"): "measurement, not a gate: the nightly standard load run, kept as an "
                                             "artefact; its gate is service-load-smoke, which ci-local.sh runs",
    ("container", "set -eu"): "CI only: brings up compose.yaml with keys and checks /_health/live and a 401; locally "
                             "its ports and container names are often taken",
}

# Gates still inline, moved a batch at a time (LC-3). This list only shrinks; it is empty now, and a new entry needs a
# reason in the commit that adds it.
NOT_MOVED_YET = set()

# Gates that are GitHub actions with no `run:` step and no local twin, listed so the gap is visible.
GITHUB_ONLY_ACTIONS = ["CodeQL (github/codeql-action)", "cargo deny (EmbarkStudios/cargo-deny-action)"]

GATE_CALL = re.compile(r"^sh scripts/gates/([a-z0-9-]+)\.sh(?: .*)?$")


def run_steps(text):
    """Yield (job, command) for each `run:` step, without a YAML library: two-space jobs, `- run:` steps."""
    job = None
    lines = text.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        m = re.match(r"^  ([a-z][a-z0-9-]*):\s*$", line)
        if m:
            job = m.group(1)
        m = re.match(r"^(\s*)(?:- )?run: ?(.*)$", line)
        if m and job and not line.strip().startswith("#"):
            indent, value = len(m.group(1)), m.group(2)
            if not value.strip():  # `defaults: run:` holds settings, not a command
                i += 1
                continue
            if value in ("|", ">", "|-", ">-"):
                body = []
                i += 1
                while i < len(lines) and (not lines[i].strip() or len(lines[i]) - len(lines[i].lstrip()) > indent):
                    body.append(lines[i].strip())
                    i += 1
                yield job, "\n".join(b for b in body if b)
                continue
            command = value
            while command.endswith("\\") and i + 1 < len(lines):
                i += 1
                command = command[:-1].rstrip() + " " + lines[i].strip()
            yield job, command
        i += 1


def longest(job, first, table):
    """The entry of `table` for this job whose prefix is the longest one `first` starts with."""
    keys = [k for k in table if k[0] == job and first.startswith(k[1])]
    return max(keys, key=lambda k: len(k[1])) if keys else None


def main():
    workflow = open(sys.argv[1] if len(sys.argv) > 1 else WORKFLOW, encoding="utf-8").read()
    local = open(LOCAL, encoding="utf-8").read()
    scripts = sorted(f[:-3] for f in os.listdir(GATES) if f.endswith(".sh"))
    problems = []
    used_not_gates, used_not_moved, called = set(), set(), set()

    for job, command in run_steps(workflow):
        first = command.splitlines()[0] if command else ""
        m = GATE_CALL.match(first)
        if m:
            name = m.group(1)
            called.add(name)
            if name not in scripts:
                problems.append(f"{job}: calls scripts/gates/{name}.sh, which does not exist")
            continue
        key = longest(job, first, NOT_GATES)
        if key:
            used_not_gates.add(key)
            continue
        key = longest(job, first, NOT_MOVED_YET)
        if key:
            used_not_moved.add(key)
            continue
        problems.append(f"{job}: `{first[:80]}` is an inline gate: move it to scripts/gates/<name>.sh, "
                        "or list it in scripts/check-gates.py with the reason it is not a gate")

    for name in scripts:
        if f"scripts/gates/{name}.sh" not in local and f"gate_script {name}" not in local:
            problems.append(f"scripts/gates/{name}.sh is not run by scripts/ci-local.sh, so it has no local twin")
    for key in sorted(set(NOT_GATES) - used_not_gates):
        problems.append(f"listed as not a gate, but no such step: {key}")
    for key in sorted(NOT_MOVED_YET - used_not_moved):
        problems.append(f"listed as not moved yet, but no such step (take it off the list): {key}")

    if problems:
        print("\n".join(f"check-gates: {p}" for p in problems), file=sys.stderr)
        return 1
    print(f"gates ok: {len(called)} gate scripts called by ci.yml, of {len(scripts)}; "
          f"{len(used_not_gates)} steps not gates; {len(used_not_moved)} gates not moved yet; "
          f"GitHub only, no run step: {', '.join(GITHUB_ONLY_ACTIONS)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
