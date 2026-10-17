#!/usr/bin/env python3
"""The two test tiers' measurements, read from a CI run's log (D805-D807).

What it is for
--------------
The owner decided (2026-10-17) that the slow tests leave CI, as two tiers,
each defined by a measured rule written down with the runs it came from:

    the slow tier          every test libtest said "has been running for
                           over 60 seconds" about in an ordinary gate run
                           (gate.yml) - ignored without its crate's
                           `slow-tests` feature, run by scripts/test-slow.sh
    heavy under coverage   every other test libtest flagged in a coverage
                           run (coverage.yml) - `#[cfg_attr(coverage, ignore)]`

A test enters or leaves a tier only by a later run's log. This script is
how such a log is read: the rule's input (`flagged`), the per-binary times
(`binaries`), what a coverage run is left with once both tiers are skipped
(`replay`), and how much slower a test is under the instrumentation than in
the gate (`ratio`). It produced D805-D807's figures from gate run
38977019041 and coverage run 38977019032.

How the replay works
--------------------
libtest runs a binary's tests in name order on N threads (the runner's
cores: four on GitHub's ubuntu-24.04 for a public repository), starting the
next test the moment one finishes, and prints `test <name> ... ok` as each
finishes. With GitHub's timestamp on every line, each test's start is the
binary's "Running" line (the first N) or the completion that freed its
thread, and its duration follows. Two checks hold the reconstruction: each
flagged test's "over 60 seconds" line falls 60 s after its start, and a
replay that skips nothing gives back the binary's own "finished in". The
replay then runs the same durations, in the same order on N threads,
without the flagged tests. Those durations were measured beside the flagged
tests; alone, a test runs no slower, so the replay is an upper bound.

Getting a log
-------------
    gh run view <run id> --log > run.log      # job, step, timestamped line

(tab-separated; the colour codes are stripped here). A plain `cargo test`
output works for `binaries` and `flagged` too (no job, no timestamps). For
a local run that `replay` and `ratio` can read, put it through `stamp`,
which writes each line as GitHub's log does, under the job `local`:

    RUST_TEST_THREADS=4 cargo llvm-cov --no-report --locked -p wipemark-pixels 2>&1 \
        | scripts/test-tiers.py stamp > local.log

Usage
-----
    scripts/test-tiers.py flagged  <log> [--job NAME]
    scripts/test-tiers.py binaries <log> [--job NAME]
    scripts/test-tiers.py replay   <log> [--job NAME] [--threads 4]
    scripts/test-tiers.py ratio    <gate log> <coverage log> [--job gate] [--threads 4]
    <command> 2>&1 | scripts/test-tiers.py stamp > <log>

`--job` keeps the lines of the jobs whose name starts with NAME (the gate
log holds `gate`, `macos` and `native`). In a gate log only the
`cargo test --workspace` step is read, where the rule is measured.

Needs python3, nothing else. Output: tables on stdout; `flagged` prints one
`<job>  <binary>  <source>  <test>` line per flagged test, the list a tier
is made from.
"""
import argparse
import datetime as dt
import heapq
import re
import sys

ANSI = re.compile(r"\x1b\[[0-9;]*m")
STAMP = re.compile(r"^\ufeff?(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d+)Z (.*)$")
RUNNING = re.compile(
    r"Running (?:unittests )?(\S+) \((?:\S*/)?(?:deps|examples)/([A-Za-z0-9_]+)-([0-9a-f]{16})\)"
)
RESULT = re.compile(r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored.*finished in ([0-9.]+)s")
DONE = re.compile(r"^test (\S+) \.\.\. (ok|FAILED)$")
FLAGGED = re.compile(r"^test (\S+) has been running for over 60 seconds")
GROUP = re.compile(r"##\[group\]Run (.*)")


def lines(path, job):
    """(job, timestamp or None, text) for every line of the log, colour stripped."""
    for raw in open(path, encoding="utf-8", errors="replace"):
        line = ANSI.sub("", raw.rstrip("\n"))
        parts = line.split("\t")
        if len(parts) >= 3:
            name, text = parts[0], "\t".join(parts[2:])
        else:
            name, text = "", line
        if job and not name.startswith(job):
            continue
        m = STAMP.match(text)
        if m:
            yield name, dt.datetime.fromisoformat(m.group(1)[:26]), m.group(2)
        else:
            yield name, None, text.lstrip("\ufeff")


def binaries(path, job):
    """Every test binary run, in order: its job, name, source, hash, start,
    "finished in", passed and ignored counts, flagged tests and completions.

    In a job that runs more than one `cargo test` step, only the
    `cargo test --workspace` step is read; a job without step markers (a
    coverage job, plain cargo output) is read whole.
    """
    out = []
    cur = None
    step = {}  # job -> True while its current step is read
    package = {}  # job -> the last unit-test binary's crate: cargo runs a
    # package's `src/` binary before its integration tests and examples
    for name, t, text in lines(path, job):
        g = GROUP.search(text)
        if g:
            cmd = g.group(1)
            step[name] = not cmd.startswith("cargo ") or cmd.startswith(
                ("cargo test --workspace", "cargo llvm-cov")
            )
            cur = None
            continue
        if not step.get(name, True):
            continue
        r = RUNNING.search(text)
        if r:
            if r.group(1).startswith("src/"):
                package[name] = r.group(2)
            cur = dict(job=name, pkg=package.get(name, "?"), src=r.group(1), name=r.group(2),
                       hash=r.group(3), start=t, secs=None, passed=0, ignored=0, flagged={},
                       done={})
            out.append(cur)
            continue
        if cur is None or cur["secs"] is not None or name != cur["job"]:
            continue
        f = FLAGGED.match(text)
        if f:
            cur["flagged"][f.group(1)] = t
            continue
        d = DONE.match(text)
        if d:
            cur["done"][d.group(1)] = t
            continue
        res = RESULT.search(text)
        if res:
            cur["passed"], cur["ignored"] = int(res.group(1)), int(res.group(3))
            cur["secs"] = float(res.group(4))
    return out


def reconstruct(b, threads):
    """Each finished test's start and duration, in seconds from the binary's start."""
    order = sorted(b["done"])
    finishes = sorted(b["done"].values())
    start = {n: b["start"] for n in order[:threads]}
    for n, t in zip(order[threads:], finishes):
        start[n] = t
    dur = {n: max(0.0, (b["done"][n] - start[n]).total_seconds()) for n in order}
    return order, start, dur


def replay(order, dur, skip, threads):
    free = [0.0] * threads
    end = 0.0
    for n in order:
        if n in skip:
            continue
        s = heapq.heappop(free)
        end = max(end, s + dur[n])
        heapq.heappush(free, s + dur[n])
    return end


def cmd_flagged(a):
    for b in binaries(a.log, a.job):
        for n in sorted(b["flagged"]):
            print(f"{(b['job'] or '-').split(' ')[0]}\t{b['pkg']}\t{b['src']}\t{n}")


def cmd_binaries(a):
    total = 0.0
    print(f"{'job':<10}{'package':<20}{'source':<32}{'secs':>9}{'tests':>7}{'>60s':>6}")
    for b in binaries(a.log, a.job):
        secs = "cut" if b["secs"] is None else f"{b['secs']:.2f}"
        total += b["secs"] or 0.0
        print(f"{(b['job'] or '-').split(' ')[0]:<10}{b['pkg']:<20}{b['src']:<32}{secs:>9}"
              f"{b['passed'] + b['ignored']:>7}{len(b['flagged']):>6}")
    print(f"\nfinished binaries: {total:.0f} s ({total / 60:.1f} min)")


def cmd_replay(a):
    before = after = 0.0
    worst = 0.0
    print(f"{'package':<20}{'source':<32}{'secs':>9}{'replay':>9}{'skipped':>9}{'flagged':>8}{'left':>6}")
    for b in binaries(a.log, a.job):
        if b["start"] is None:
            sys.exit("replay needs GitHub's timestamps: a log from `gh run view --log`")
        order, start, dur = reconstruct(b, a.threads)
        skip = set(b["flagged"])
        for n, ft in b["flagged"].items():
            if n in start:
                worst = max(worst, abs((ft - start[n]).total_seconds() - 60.0))
        whole = replay(order, dur, set(), a.threads)
        left = replay(order, dur, skip, a.threads)
        secs = "cut" if b["secs"] is None else f"{b['secs']:.1f}"
        before += b["secs"] or 0.0
        after += left
        print(f"{b['pkg']:<20}{b['src']:<32}{secs:>9}{whole:>9.1f}{left:>9.1f}{len(skip):>8}"
              f"{len(set(order) - skip):>6}")
    print(f"\nthreads {a.threads}; a flagged test's line is {worst:.1f} s off its start + 60 s at worst")
    print(f"finished binaries: {before:.0f} s ({before / 60:.1f} min)")
    print(f"the same, every flagged test skipped: {after:.0f} s ({after / 60:.1f} min)")


def cmd_ratio(a):
    gate = {b["hash"]: b for b in binaries(a.gate, a.job)}
    cov = {b["hash"]: b for b in binaries(a.coverage, None)}
    for h, g in gate.items():
        if not g["done"]:
            continue
        _, _, gd = reconstruct(g, a.threads)
        c = cov.get(h)
        cd = reconstruct(c, a.threads)[2] if c and c["done"] else {}
        where = "not reached under coverage" if c is None else f"coverage {c['secs']} s"
        print(f"== {g['pkg']} {g['src']}: gate {g['secs']} s, {where}")
        for n in sorted(gd, key=gd.get, reverse=True):
            if n in cd and gd[n] >= 0.05:
                print(f"   {gd[n]:9.2f} s gate {cd[n]:9.2f} s coverage  x{cd[n] / gd[n]:5.1f}  {n}")
            else:
                print(f"   {gd[n]:9.2f} s gate  {n}")


def cmd_stamp(_):
    for line in sys.stdin:
        now = dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%f")
        sys.stdout.write(f"local\tlocal\t{now}0Z {line}")
        sys.stdout.flush()


def main():
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = p.add_subparsers(dest="cmd", required=True)
    sub.add_parser("stamp")
    for name in ("flagged", "binaries", "replay"):
        s = sub.add_parser(name)
        s.add_argument("log")
        s.add_argument("--job", default=None)
        s.add_argument("--threads", type=int, default=4)
    s = sub.add_parser("ratio")
    s.add_argument("gate")
    s.add_argument("coverage")
    s.add_argument("--job", default="gate")
    s.add_argument("--threads", type=int, default=4)
    a = p.parse_args()
    {"flagged": cmd_flagged, "binaries": cmd_binaries, "replay": cmd_replay, "ratio": cmd_ratio,
     "stamp": cmd_stamp}[a.cmd](a)


if __name__ == "__main__":
    main()
