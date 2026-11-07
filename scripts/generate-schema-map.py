#!/usr/bin/env python3
"""Generate spec/schema-map.md: every table and view the migrations create, with its columns (task OF-6).

    python3 scripts/generate-schema-map.py          # write the page
    python3 scripts/generate-schema-map.py --check  # exit 1 if it is out of date (the docs job)

It reads the CREATE TABLE and CREATE VIEW statements in the service's migrations, so a new table appears in the
page only after the page is regenerated, and the docs job fails until it is. A column's meaning is not described:
the migrations carry few comments, so only the name and SQL type are listed. A table created some other way (a
statement built at run time, such as the monthly audit partitions) is not listed.
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
MIGRATIONS = ROOT / "electronic-patient-record-service-with-rust" / "migrations"
OUT = ROOT / "spec" / "schema-map.md"

SKIP = ("PRIMARY", "UNIQUE", "CHECK", "CONSTRAINT", "FOREIGN", "LIKE", "EXCLUDE")


def body_after(text, start):
    """The text inside the parentheses that open at text[start], and the index after the closing one."""
    depth = 0
    for i in range(start, len(text)):
        if text[i] == "(":
            depth += 1
        elif text[i] == ")":
            depth -= 1
            if depth == 0:
                return text[start + 1 : i]
    return ""


def split_top(body):
    parts, depth, cur = [], 0, ""
    for ch in body:
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        if ch == "," and depth == 0:
            parts.append(cur)
            cur = ""
        else:
            cur += ch
    if cur.strip():
        parts.append(cur)
    return parts


def purpose(text):
    for line in text.splitlines():
        if line.startswith("//!"):
            return line[3:].strip().rstrip(".") + "."
    return ""


def generate():
    tables, views = {}, {}
    for path in sorted(MIGRATIONS.glob("*.sql")):
        text = path.read_text()
        migration = path.stem
        for m in re.finditer(r"CREATE TABLE (?:IF NOT EXISTS )?([a-z_][a-z0-9_]*) \(", text):
            name = m.group(1)
            cols = []
            body = re.sub(r"--[^\n]*", "", body_after(text, m.end() - 1))
            for part in split_top(body):
                words = part.split()
                if not words or words[0].upper() in SKIP:
                    continue
                cols.append(f"`{words[0]}` {words[1].upper() if len(words) > 1 else ''}".strip())
            entry = tables.setdefault(name, {"migrations": [], "columns": cols, "purpose": purpose(text)})
            if migration not in entry["migrations"]:
                entry["migrations"].append(migration)
        # A table a later migration drops is no longer in the schema.
        for m in re.finditer(r"DROP TABLE (?:IF EXISTS )?([a-z_][a-z0-9_]*)", re.sub(r"--[^\n]*", "", text)):
            tables.pop(m.group(1), None)
        for m in re.finditer(r"CREATE (?:OR REPLACE )?VIEW ([a-z_][a-z0-9_]*)", text):
            views.setdefault(m.group(1), [])
            if migration not in views[m.group(1)]:
                views[m.group(1)].append(migration)
    out = [
        "# Schema map",
        "",
        "Every table and view the service's migrations create. **Generated** by "
        "[`scripts/generate-schema-map.py`](../scripts/generate-schema-map.py) from the `CREATE TABLE` and `CREATE VIEW` "
        "statements in `electronic-patient-record-service-with-rust/migrations/`; the docs job fails when this page is out of "
        "date (task OF-6). Do not edit it by hand.",
        "",
        "Not listed: tables made by a statement built at run time (the monthly `audit_events` partitions), and what a column "
        "means (the migrations carry few comments, so only the name and SQL type are shown). The clinical tables are append-only: "
        "a change is a new row, and triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`.",
        "",
        f"{len(tables)} tables, {len(views)} views.",
        "",
        "## Tables",
        "",
        "| Table | Created in | Columns |",
        "|---|---|---|",
    ]
    for name in sorted(tables):
        t = tables[name]
        cols = ", ".join(t["columns"]) or "(copies another table)"
        out.append(f"| `{name}` | {', '.join(f'`{x}`' for x in t['migrations'])} | {cols} |")
    out += ["", "## Views", "", "| View | Created in |", "|---|---|"]
    for name in sorted(views):
        out.append(f"| `{name}` | {', '.join(f'`{x}`' for x in views[name])} |")
    return "\n".join(out) + "\n"


def main():
    text = generate()
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            print("spec/schema-map.md is out of date: run python3 scripts/generate-schema-map.py")
            return 1
        print("spec/schema-map.md is up to date")
        return 0
    OUT.write_text(text)
    print(f"wrote {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
