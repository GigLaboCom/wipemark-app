#!/usr/bin/env python3
"""How long `wipemark-cli inspect` takes on a picture, CLI against CLI (E12-6 for Grok, G5).

What it is for, and who asked
-----------------------------
The coordinator's task `wipemark-task-grok-x-mark-2026-10-11` (G5) asks for the
Gemini regression with Grok's profiles shipped. It passes every gate, and its
only attention is time: every file examined 1.5–2.6 times slower than the
baseline — a baseline measured on another machine, and a run that shared its
cores with the workspace tests and the bench. This script measures what the two
Grok profiles cost by themselves, on one idle machine: the same pictures through
a CLI built with them and one built without them (the catalogue of the commit
before they were shipped), each run several times, the median kept.

What it does
------------
1. For every picture and every CLI (`--cli NAME=PATH`, two or more), runs
   `<cli> inspect <picture> --json` `--runs` times (3) and keeps the median wall
   time.
2. Prints a table: per picture, each CLI's median and the ratio of each to the
   first.

How to run it
-------------
    python3 scripts/grok/time-cli.py --cli without=PATH --cli with=PATH PICTURE [PICTURE …] [--runs 3]
    python3 scripts/grok/time-cli.py selftest

`selftest` times `true` against `sleep 0.05` (a CLI that ignores its arguments)
and checks the ratio reads the second as the slower.

What it needs
-------------
Python 3.11+; the CLIs to compare. Run it on an idle machine: anything else
running is in the times.

What its output means
---------------------
A Markdown table of medians in seconds and ratios. It times the whole process —
start, decode, both passes, the JSON — as a person waits for it.
"""

import argparse
import os
import statistics
import subprocess
import sys
import time


def median_time(cmd, runs):
    times = []
    for _ in range(runs):
        t = time.perf_counter()
        subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        times.append(time.perf_counter() - t)
    return statistics.median(times)


def table(clis, pictures, runs, argv=lambda cli, pic: [cli, "inspect", pic, "--json"]):
    names = [n for n, _ in clis]
    rows = ["| picture | " + " | ".join(f"{n}, s" for n in names) + " | " +
            " | ".join(f"{n} / {names[0]}" for n in names[1:]) + " |",
            "|---|" + "---|" * (2 * len(names) - 1)]
    ratios = []
    for pic in pictures:
        ts = [median_time(argv(path, pic), runs) for _, path in clis]
        ratios.append([t / ts[0] for t in ts[1:]])
        rows.append(f"| `{os.path.basename(pic)}` | " + " | ".join(f"{t:.2f}" for t in ts) + " | " +
                    " | ".join(f"{t / ts[0]:.2f}" for t in ts[1:]) + " |")
    return "\n".join(rows), ratios


def selftest():
    out, ratios = table([("fast", "true"), ("slow", "sleep")], ["0.05"], 3, argv=lambda cli, pic: [cli, pic])
    ok = ratios[0][0] > 1.0
    print(out)
    print("selftest: " + ("passed" if ok else "failed"))
    return 0 if ok else 1


def main():
    if sys.argv[1:] == ["selftest"]:
        sys.exit(selftest())
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("pictures", nargs="+")
    ap.add_argument("--cli", action="append", required=True, metavar="NAME=PATH")
    ap.add_argument("--runs", type=int, default=3)
    a = ap.parse_args()
    clis = [tuple(c.split("=", 1)) for c in a.cli]
    if len(clis) < 2 or any(len(c) != 2 for c in clis):
        print("time-cli: two or more --cli NAME=PATH", file=sys.stderr)
        sys.exit(2)
    print(table(clis, a.pictures, a.runs)[0])


if __name__ == "__main__":
    main()
