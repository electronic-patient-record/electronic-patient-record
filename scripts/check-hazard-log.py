#!/usr/bin/env python3
"""Check the clinical hazard log (task AS-1): governance/clinical-safety/hazard-log.csv.

Usage: python3 scripts/check-hazard-log.py [repo-root]
       python3 scripts/check-hazard-log.py --base <git-ref> --author <login> [repo-root]

It checks the shape, not the clinical judgement: the columns, unique ids, severity and likelihood in
1..5 with score = severity x likelihood, a known status, an ISO `added` date, evidence paths that
exist, and that no hazard is `accepted` or `closed` without a named reviewer. Only a person (the
Clinical Safety Officer, OQ-2) may set those, so an automated change cannot close a hazard.

With `--base` it also compares the log with that revision (CQ-21). A change that moves a hazard into
`accepted` or `closed`, adds one in either status, or changes `reviewed_by` fails unless `--author`
is listed in the reviewer allow-list AS IT IS IN THE BASE revision (so a change cannot add its own
author). No row may be removed. The list is empty until a Clinical Safety Officer is named.
"""
import csv
import datetime
import io
import os
import subprocess
import sys

COLUMNS = ["id", "title", "hazard", "cause", "effect", "severity", "likelihood", "score",
           "controls", "evidence", "residual", "status", "reviewed_by", "added"]
STATUSES = {"open", "controlled-unreviewed", "accepted", "closed"}
LOG = os.path.join("governance", "clinical-safety", "hazard-log.csv")
REVIEWERS = os.path.join("governance", "clinical-safety", "reviewers.txt")
SIGNED_OFF = {"accepted", "closed"}


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
        if r["added"].strip():
            try:
                if len(r["added"]) != 10:
                    raise ValueError
                datetime.date.fromisoformat(r["added"])
            except ValueError:
                found.append(f"{where}: added must be a real YYYY-MM-DD date")
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


def parse_reviewers(text):
    """Logins allowed to sign a hazard off: one per line; blank lines and `#` comments ignored."""
    return {line.strip() for line in text.splitlines() if line.strip() and not line.strip().startswith("#")}


def transitions(base_rows, head_rows, author, reviewers):
    """Sign-off changes between two revisions of the log that `author` may not make."""
    found = []
    allowed = author in reviewers
    base = {r["id"]: r for r in base_rows}
    head_ids = {r["id"] for r in head_rows}
    for gone in sorted(set(base) - head_ids):
        found.append(f"{LOG}: {gone} was removed; a hazard is never removed (record why it no longer applies)")
    for r in head_rows:
        before = base.get(r["id"])
        was = before["status"] if before else None
        if r["status"] in SIGNED_OFF and was != r["status"] and not allowed:
            found.append(f"{LOG}: {r['id']} set to {r['status']} by {author}, who is not on {REVIEWERS}")
        if before and before["reviewed_by"] != r["reviewed_by"] and not allowed:
            found.append(f"{LOG}: {r['id']} reviewed_by changed by {author}, who is not on {REVIEWERS}")
    return found


def git_show(root, ref, path):
    """The file at `ref`, or None when it does not exist there."""
    try:
        return subprocess.run(["git", "-C", root, "show", f"{ref}:{path}"], check=True,
                              capture_output=True, text=True).stdout
    except subprocess.CalledProcessError:
        return None


def read_rows(text):
    reader = csv.DictReader(io.StringIO(text))
    return reader.fieldnames, list(reader)


def main(argv):
    args = argv[1:]
    base_ref = author = None
    while args and args[0].startswith("--"):
        flag = args.pop(0)
        if flag == "--base" and args:
            base_ref = args.pop(0)
        elif flag == "--author" and args:
            author = args.pop(0)
        else:
            print(f"unknown or incomplete option {flag}", file=sys.stderr)
            return 2
    if (base_ref is None) != (author is None):
        print("--base and --author go together", file=sys.stderr)
        return 2
    root = args[0] if args else "."
    with open(os.path.join(root, LOG), newline="", encoding="utf-8") as f:
        header, rows = read_rows(f.read())
    found = problems(rows, header, root)
    if base_ref is not None:
        base_text = git_show(root, base_ref, LOG)
        if base_text is not None:
            reviewers = parse_reviewers(git_show(root, base_ref, REVIEWERS) or "")
            found += transitions(read_rows(base_text)[1], rows, author, reviewers)
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
