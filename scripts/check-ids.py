#!/usr/bin/env python3
"""Each open question and each task has its own id.

Usage: python3 scripts/check-ids.py [repo-root]

Two collisions reached main on 2026-11-25 (`OQ-MS1`, already manual steps; `OQ-OS1`, already rate limits), so an answer to one question could be read as an answer to another. Checked: the first cell of each row
of `spec/16-open-questions.md`, where `OQ-MS1 to OQ-MS5` is five ids, and the id of each `- [ ] **ID` line of `tasks.md`.
Not checked: ids used in a page and in no row.
"""
import os
import re
import sys

QUESTION = re.compile(r"OQ-([A-Z]*)(\d+)")
TASK = re.compile(r"^- \[[ x]\] \*\*([A-Z]+-\d+)\b")


def questions(text):
    """Every question id a row's first cell gives, in order."""
    out = []
    for line in text.splitlines():
        if not line.startswith("| OQ-"):
            continue
        cell = line.split("|")[1]
        ids = QUESTION.findall(cell)
        if " to " in cell and len(ids) == 2 and ids[0][0] == ids[1][0]:
            out += [f"OQ-{ids[0][0]}{n}" for n in range(int(ids[0][1]), int(ids[1][1]) + 1)]
        else:
            out += [f"OQ-{letters}{n}" for letters, n in ids]
    return out


def tasks(text):
    """Every task id."""
    return [m.group(1) for m in map(TASK.match, text.splitlines()) if m]


def repeated(ids):
    """The ids that appear more than once, sorted."""
    return sorted({i for i in ids if ids.count(i) > 1})


def self_test():
    rows = "| OQ-MS1 to OQ-MS3 | a |\n| OQ-MS2 | b |\n| OQ-5 | c |\nOQ-5 in words\n| OQ-A1, OQ-A2 | d |"
    assert questions(rows) == ["OQ-MS1", "OQ-MS2", "OQ-MS3", "OQ-MS2", "OQ-5", "OQ-A1", "OQ-A2"], questions(rows)
    assert repeated(questions(rows)) == ["OQ-MS2"]
    lines = "- [ ] **AB-1 (S)** x\n- [x] **AB-1 (S)** y\n- [ ] **AB-10 (S)**\nsee **AB-10**"
    assert repeated(tasks(lines)) == ["AB-1"], tasks(lines)


def main(argv):
    self_test()
    root = argv[1] if len(argv) > 1 else "."
    failed = False
    for path, read in ((os.path.join("spec", "16-open-questions.md"), questions), ("tasks.md", tasks)):
        with open(os.path.join(root, path), encoding="utf-8") as f:
            ids = read(f.read())
        if not ids:
            print(f"{path}: no id was read; the format changed", file=sys.stderr)
            failed = True
        for again in repeated(ids):
            print(f"{path}: {again} is used more than once", file=sys.stderr)
            failed = True
        print(f"{path}: {len(ids)} ids")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
