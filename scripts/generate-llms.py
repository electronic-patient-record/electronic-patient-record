#!/usr/bin/env python3
"""Generate llms.txt and llms.json at the repository root from one table, so the two cannot drift.

    python3 scripts/generate-llms.py          # write both
    python3 scripts/generate-llms.py --check  # exit 1 if either is out of date (the docs job)

Every path in the table must exist; a missing file fails the run. Edit the table below, not the outputs.
"""
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
REPO = "https://github.com/electronic-patient-record/electronic-patient-record"

NAME = "Electronic Patient Record"
SUMMARY = (
    "Open-source electronic patient record software: a safe, append-only, audited longitudinal patient record "
    "with FHIR R4 as both data model and API, and explainable patient identity matching. A Rust matcher library, "
    "a Rust service (loco.rs, PostgreSQL) and a SvelteKit 3 / Svelte 5 UI, specified in spec/. "
    "Pre-release scaffold (milestone M0): NOT FOR CLINICAL USE; do not enter real patient data; no accreditation, "
    "certification or clinical validation is claimed."
)
NOTES = [
    "The specification in spec/ and each project's spec/index.md is the source of truth; code and docs follow it. A behavioural change is one commit with a spec edit, a code edit and a test edit.",
    "One item, one commit, one push, then look at CI. Stage named files, never `git add -A`.",
    "Say plainly what is not built or not tested: every CHANGELOG.md entry states what was not done or not verified. Never weaken a guard or census test to get green.",
    "Clinical risk: a change that could alter what a clinician sees, decides or signs updates governance/clinical-safety/hazard-log.csv in the same commit. Never set a hazard to accepted or closed; only the Clinical Safety Officer does.",
    "The clinical record is append-only: no delete path for clinical data. Absent evidence is null with a reason, never zero; a missing field blocks an automatic patient link.",
    "API versioning is the Accepts-version request header on the unversioned /api path, never /api/v1. Use Podman, not Docker.",
    "Translations are machine-written until a speaker reviews them. Never claim a translation is reviewed.",
    "Never put real patient data in examples, tests or fixtures; use synthetic data.",
]
STACK = {
    "matcher": "Rust library (pure, deterministic, explainable identity matching), published as electronic-patient-record-matcher",
    "service": "Rust, loco.rs (Axum, SeaORM), PostgreSQL 18, REST and FHIR R4",
    "ui": "SvelteKit 3, Svelte 5 runes, backend-for-frontend proxy",
}

# (section, [(title, path, description)])
SECTIONS = [
    ("Start here", [
        ("README", "README.md", "What the project is, the three subprojects, how to run the API from a fresh clone."),
        ("AGENTS", "AGENTS.md", "How work is done here: spec first, one commit per item, the gates, and the things that are easy to get wrong."),
        ("Specification index", "spec/index.md", "Entry point to the specification."),
        ("Implementation status", "spec/14-implementation-status.md", "Generated: exactly what is built and how many tests pass."),
        ("Tasks", "tasks.md", "The task queue with what landed, what is open and what was not verified."),
        ("Changelog", "CHANGELOG.md", "Every change, newest first, each stating what was not done or not verified."),
    ]),
    ("Specification", [
        ("Purpose and vision", "spec/01-purpose-and-vision.md", "The problem and what this provides."),
        ("Scope", "spec/02-scope.md", "What is in and out."),
        ("Domain model", "spec/05-domain-model.md", "Entities and relationships."),
        ("Functional requirements", "spec/06-functional-requirements.md", "Numbered functional requirements."),
        ("Non-functional requirements", "spec/07-non-functional-requirements.md", "Safety, security, performance, accessibility."),
        ("Architecture", "spec/08-architecture.md", "Components and data flow."),
        ("API surface", "spec/09-api-surface.md", "Routes, versioning by header, error bodies."),
        ("Persistence", "spec/10-persistence.md", "Append-only tables and audit triggers."),
        ("Testing strategy", "spec/11-testing-strategy.md", "Gates, census tests, oracles."),
        ("Compliance", "spec/12-compliance.md", "The regulatory position and what is not claimed."),
        ("Roadmap", "spec/15-roadmap.md", "Milestones M0 onward."),
        ("Open questions", "spec/16-open-questions.md", "Decisions that need a maintainer, a clinician or the Clinical Safety Officer."),
        ("Plan", "plan.md", "The long plan behind the task queue."),
    ]),
    ("Subprojects", [
        ("Matcher AGENTS", "electronic-patient-record-matcher-with-rust/AGENTS.md", "Working in the matching library."),
        ("Matcher spec", "electronic-patient-record-matcher-with-rust/spec/index.md", "What the matcher does and refuses."),
        ("Service AGENTS", "electronic-patient-record-service-with-rust/AGENTS.md", "Working in the loco.rs service."),
        ("Service spec", "electronic-patient-record-service-with-rust/spec/index.md", "What the service does, row by row."),
        ("UI AGENTS", "electronic-patient-record-ui-with-svelte/AGENTS.md", "Working in the SvelteKit UI."),
        ("UI spec", "electronic-patient-record-ui-with-svelte/spec/index.md", "What each page does."),
    ]),
    ("Clinical safety and governance", [
        ("Hazard log", "governance/clinical-safety/hazard-log.csv", "The hazard log (never edit a status to accepted or closed)."),
        ("Risk management plan", "governance/clinical-safety/risk-management-plan.md", "How hazards are managed."),
        ("Governance", "GOVERNANCE.md", "How decisions are made."),
        ("Security policy", "SECURITY.md", "Reporting a vulnerability."),
    ]),
    ("Agent skill", [
        ("Skill", "electronic-patient-record-skill/SKILL.md", "A skill for agents working in this repository."),
    ]),
]
OPTIONAL = [
    ("Contributing", "CONTRIBUTING.md", "How to contribute."),
    ("AI statement", "AI_STATEMENT.md", "How AI is used in this project."),
    ("RFC", "RFC.md", "Request for comments."),
    ("News", "NEWS.md", "Project news."),
    ("Maintainers", "MAINTAINERS.md", "Who maintains this."),
]
COMMANDS = {
    "matcher": "cd electronic-patient-record-matcher-with-rust && cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test",
    "service": "cd electronic-patient-record-service-with-rust && cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test",
    "service database tests": "TEST_DB_PORT=5543 TEST_REDIS_PORT=6390 podman compose -f compose.test.yaml up -d --wait, then DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test cargo test -- --ignored --test-threads=1",
    "ui": "cd electronic-patient-record-ui-with-svelte && pnpm run check && pnpm exec vitest run && PW_PORT=4183 CI=true pnpm exec playwright test",
    "docs": "python3 scripts/check-links.py . && sh scripts/generate-status.sh --check && python3 scripts/generate-llms.py --check && python3 scripts/check-hazard-log.py",
}


def link(path: str) -> str:
    if not (ROOT / path).exists():
        sys.exit(f"generate-llms: {path} does not exist; fix the table in scripts/generate-llms.py")
    return f"{REPO}/blob/main/{path}"


def txt() -> str:
    out = [f"# {NAME}", "", f"> {SUMMARY}", ""]
    out += [f"- {n}" for n in NOTES] + [""]
    for section, links in SECTIONS:
        out += [f"## {section}", ""]
        out += [f"- [{t}]({link(p)}): {d}" for t, p, d in links] + [""]
    out += ["## Optional", ""] + [f"- [{t}]({link(p)}): {d}" for t, p, d in OPTIONAL] + [""]
    out += ["## Commands", ""] + [f"- {k}: `{v}`" for k, v in COMMANDS.items()]
    return "\n".join(out) + "\n"


def as_json() -> str:
    def item(t, p, d):
        return {"title": t, "url": link(p), "path": p, "description": d}

    doc = {
        "name": NAME,
        "summary": SUMMARY,
        "repository": REPO,
        "status": "pre-release scaffold (M0); not for clinical use",
        "stack": STACK,
        "notes": NOTES,
        "sections": [{"name": s, "links": [item(*l) for l in links]} for s, links in SECTIONS],
        "optional": [item(*l) for l in OPTIONAL],
        "commands": COMMANDS,
    }
    return json.dumps(doc, indent=2, ensure_ascii=False) + "\n"


def main() -> None:
    outputs = {"llms.txt": txt(), "llms.json": as_json()}
    if "--check" in sys.argv:
        stale = [n for n, body in outputs.items() if not (ROOT / n).exists() or (ROOT / n).read_text() != body]
        if stale:
            sys.exit(f"{', '.join(stale)} out of date: run python3 scripts/generate-llms.py")
        print("llms.txt and llms.json are up to date")
        return
    for name, body in outputs.items():
        (ROOT / name).write_text(body)
        print(f"wrote {name} ({len(body)} bytes)")


if __name__ == "__main__":
    main()
