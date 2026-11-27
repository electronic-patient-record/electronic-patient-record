#!/usr/bin/env python3
"""The review greps of spec/rust-checked-arithmetic (RCA-ENF-5, RCA-ENF-7), as a gate.

Production Rust only: `tests/`, `benches/`, `examples/` and everything from the first `#[cfg(test)]` of a file are skipped
(RCA-EXC-5). Fails on:

- `saturating_*`, `wrapping_*`, `overflowing_*` (RCA-INT-4, RCA-INT-5);
- a checked result unwrapped: `checked_*(..).unwrap()`, `.expect`, `.unwrap_unchecked`, `.unwrap_or_default` (RCA-NONE-1);
- a parse or lookup whose error is discarded and replaced (RCA-ENF-7), unless the statement carries `RCA-IN-3: absent only`.

The list of `unwrap_or` fallbacks (RCA-NONE-3) is for review, not a failure: `--list-fallbacks` prints it.
Run from anywhere; exit 0 when clean.
"""

import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
# `.claude` holds agent worktrees (other checkouts of this repository) and settings: never this tree's code.
SKIP_DIRS = {"target", "node_modules", ".git", ".claude", "tests", "benches", "examples", "fuzz", "locales"}

RULES = [
    ("RCA-INT-4/5", re.compile(r"\b(saturating|wrapping|overflowing)_\w+")),
    (
        "RCA-NONE-1",
        re.compile(r"checked_\w+\([^;{}]*?\)\s*\.(unwrap|expect|unwrap_unchecked|unwrap_or_default)\b", re.S),
    ),
    (
        "RCA-ENF-7",
        re.compile(r"\.parse(::<[^>]+>)?\(\)[^;{}]*?\.(ok\(\)|unwrap_or|unwrap_or_default|unwrap_or_else)\b", re.S),
    ),
    (
        "RCA-ENF-7",
        re.compile(
            r"(env::var|var_os|get|get_param|query)\([^;{}]*?\.ok\(\)[^;{}]*?\.(unwrap_or|unwrap_or_default|unwrap_or_else)\b",
            re.S,
        ),
    ),
]
FALLBACKS = re.compile(r"checked_\w+\([^;{}]*?\)\s*\.(unwrap_or|unwrap_or_else|map_or|map_or_else)\b", re.S)
MARKER = "RCA-IN-3: absent only"

# Known debt (QA-17): hits that exist today, as file -> (rule, how many, why it is still there). A new hit anywhere fails;
# a count that goes down must be lowered here, so the list only shrinks.
SERVICE = "electronic-patient-record-service-with-rust/"
KNOWN = {
    SERVICE + "src/authz.rs": ("RCA-ENF-7", 1, "an unreadable request-id header becomes empty, then a new id; to review"),
}


def production_sources():
    for base, dirs, files in os.walk(ROOT):
        dirs[:] = [d for d in dirs if d not in SKIP_DIRS]
        for name in files:
            if name.endswith(".rs"):
                yield os.path.join(base, name)


def production_text(path):
    with open(path, encoding="utf-8") as f:
        text = f.read()
    cut = text.find("#[cfg(test)]")
    return text if cut < 0 else text[:cut]


def lines_of(text, match):
    first = text.count("\n", 0, match.start()) + 1
    return first, text[match.start() : match.end()]


def main(argv):
    listing = "--list-fallbacks" in argv
    failures = []
    for path in sorted(production_sources()):
        text = production_text(path)
        rel = os.path.relpath(path, ROOT)
        if listing:
            for m in FALLBACKS.finditer(text):
                line, _ = lines_of(text, m)
                print(f"{rel}:{line}")
            continue
        for rule, pattern in RULES:
            for m in pattern.finditer(text):
                line, snippet = lines_of(text, m)
                statement_start = text.rfind(";", 0, m.start()) + 1
                statement_end = text.find(";", m.end())
                statement = text[statement_start : statement_end if statement_end >= 0 else len(text)]
                if rule == "RCA-ENF-7" and MARKER in statement:
                    continue
                failures.append(f"{rel}:{line}: {rule}: {' '.join(snippet.split())[:110]}")
    seen = {}
    fresh = []
    for f in failures:
        path, _, rest = f.partition(":")
        rule = rest.split(": ")[1]
        known = KNOWN.get(path)
        seen[path] = seen.get(path, 0) + 1
        if known and known[0] == rule and seen[path] <= known[1]:
            continue
        fresh.append(f)
    for path, (_, count, _) in KNOWN.items():
        if not listing and seen.get(path, 0) < count:
            fresh.append(f"{path}: fewer known hits than listed ({seen.get(path, 0)} < {count}): lower KNOWN in this script")
    failures = fresh
    for f in failures:
        print(f)
    if failures:
        print(f"{len(failures)} hit(s) against spec/rust-checked-arithmetic (see the rule IDs).", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
