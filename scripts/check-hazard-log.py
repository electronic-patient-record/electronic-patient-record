#!/usr/bin/env python3
"""Check the clinical hazard log (task AS-1): governance/clinical-safety/hazard-log.csv.

Usage: python3 scripts/check-hazard-log.py [repo-root]

It checks the shape, not the clinical judgement: the columns, unique ids, severity and likelihood in
1..5 with score = severity x likelihood, a known status, evidence paths that exist, and that no
hazard is `accepted` or `closed` without a named reviewer. Only a person (the Clinical Safety
Officer, OQ-2) may set those, so an automated change cannot close a hazard.
"""
import csv
import os
import sys

COLUMNS = ["id", "title", "hazard", "cause", "effect", "severity", "likelihood", "score",
           "controls", "evidence", "residual", "status", "reviewed_by", "added"]
STATUSES = {"open", "controlled-unreviewed", "accepted", "closed"}
LOG = os.path.join("governance", "clinical-safety", "hazard-log.csv")


def problems(rows, header, root):
    found = []
    if header != COLUMNS:
        return [f"columns must be exactly: {','.join(COLUMNS)}"]
    seen = set()
    for n, r in enumerate(rows, start=2):
        where = f"{LOG}:{n} ({r.get('id') or '?'})"
        if not r["id"].startswith("H-") or not r["id"][2:].isdigit():
            found.append(f"{where}: id must look like H-001")
        if r["id"] in seen:
            found.append(f"{where}: duplicate id")
        seen.add(r["id"])
        for col in ("title", "hazard", "cause", "effect", "residual", "added"):
            if not r[col].strip():
                found.append(f"{where}: {col} is empty")
        try:
            s, l, score = int(r["severity"]), int(r["likelihood"]), int(r["score"])
            if not (1 <= s <= 5 and 1 <= l <= 5):
                found.append(f"{where}: severity and likelihood must be 1..5")
            elif score != s * l:
                found.append(f"{where}: score must be severity x likelihood ({s * l})")
        except ValueError:
            found.append(f"{where}: severity, likelihood and score must be integers")
        if r["status"] not in STATUSES:
            found.append(f"{where}: status must be one of {sorted(STATUSES)}")
        if r["status"] in ("accepted", "closed") and not r["reviewed_by"].strip():
            found.append(f"{where}: {r['status']} needs reviewed_by (a named person)")
        if r["status"] == "controlled-unreviewed" and not r["controls"].strip():
            found.append(f"{where}: controlled-unreviewed needs controls")
        for path in filter(None, (p.strip() for p in r["evidence"].split(";"))):
            if not os.path.exists(os.path.join(root, path)):
                found.append(f"{where}: evidence path does not exist: {path}")
    return found


def main(argv):
    root = argv[1] if len(argv) > 1 else "."
    with open(os.path.join(root, LOG), newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        header = reader.fieldnames
        rows = list(reader)
    found = problems(rows, header, root)
    for p in found:
        print(p, file=sys.stderr)
    if found:
        print(f"{len(found)} problem(s) in the hazard log", file=sys.stderr)
        return 1
    open_n = sum(r["status"] == "open" for r in rows)
    print(f"hazard log ok: {len(rows)} hazards, {open_n} open, none accepted or closed without a reviewer")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
