#!/usr/bin/env python3
"""A baseline of the GitHub Actions CI (task LC-1): runs by outcome, wall time, and minutes per job.

Usage: python3 scripts/ci-baseline.py [--runs N] [--job-runs M]

Reads with the `gh` CLI (needs `gh auth login`); changes nothing. Wall time is the run's creation to its last
update, job minutes are each job's start to its end, summed (an approximation of billed minutes: GitHub
rounds each job up to a whole minute, which this does not). The job table covers the newest M completed
runs, because it costs one API call per run. Failures are listed by failed job and step so a person can
sort them into ours and the platform's: this script does not decide that.
"""
import collections
import datetime as dt
import json
import statistics
import subprocess
import sys


def gh(*args):
    out = subprocess.run(["gh", *args], check=True, capture_output=True, text=True).stdout
    return json.loads(out)


def when(s):
    return dt.datetime.fromisoformat(s.replace("Z", "+00:00"))


def pct(values, p):
    values = sorted(values)
    return values[min(len(values) - 1, int(round(p * (len(values) - 1))))]


def main(argv):
    runs_n, job_runs = 200, 30
    args = argv[1:]
    while args:
        flag = args.pop(0)
        if flag == "--runs" and args:
            runs_n = int(args.pop(0))
        elif flag == "--job-runs" and args:
            job_runs = int(args.pop(0))
        else:
            print(__doc__, file=sys.stderr)
            return 2
    runs = gh("run", "list", "--limit", str(runs_n), "--json",
              "databaseId,conclusion,status,createdAt,updatedAt,event")
    done = [r for r in runs if r["status"] == "completed"]
    first, last = min(r["createdAt"] for r in done), max(r["createdAt"] for r in done)
    days = max(1.0, (when(last) - when(first)).total_seconds() / 86400)
    by = collections.Counter(r["conclusion"] for r in done)
    print(f"{len(done)} completed runs, {first[:10]} to {last[:10]} ({days:.1f} days, {len(done) / days:.1f} a day)")
    for k in ("success", "failure", "cancelled"):
        print(f"  {k:10} {by.get(k, 0):4}  {100 * by.get(k, 0) / len(done):4.0f}%")
    walls = [(when(r["updatedAt"]) - when(r["createdAt"])).total_seconds() / 60
             for r in done if r["conclusion"] == "success"]
    if walls:
        print(f"wall minutes of successful runs: median {statistics.median(walls):.1f}, "
              f"p90 {pct(walls, 0.9):.1f}, max {max(walls):.1f}")
    per_job = collections.defaultdict(list)
    totals, fails = [], []
    for r in done[:job_runs]:
        jobs = gh("run", "view", str(r["databaseId"]), "--json", "jobs")["jobs"]
        total = 0.0
        for j in jobs:
            if j["conclusion"] in ("skipped", None) or not j.get("completedAt"):
                continue
            m = (when(j["completedAt"]) - when(j["startedAt"])).total_seconds() / 60
            total += m
            if r["conclusion"] == "success":
                per_job[j["name"]].append(m)
            if j["conclusion"] == "failure" and j["name"] != "ci-ok":
                steps = ", ".join(s["name"] for s in j["steps"] if s["conclusion"] == "failure")
                fails.append((r["createdAt"][:16], j["name"], steps))
        if r["conclusion"] == "success":
            totals.append(total)
    if totals:
        print(f"job minutes per successful run (newest {len(totals)}): median {statistics.median(totals):.1f}, "
              f"max {max(totals):.1f}")
    print(f"\n{'job':58} {'runs':>4} {'median':>7} {'max':>6}")
    for name, v in sorted(per_job.items(), key=lambda kv: -statistics.median(kv[1])):
        print(f"{name[:58]:58} {len(v):4} {statistics.median(v):7.1f} {max(v):6.1f}")
    if fails:
        print(f"\nfailed jobs among the newest {job_runs} runs (sort these into ours and the platform's by hand):")
        for at, name, steps in fails:
            print(f"  {at}  {name}: {steps}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
