#!/usr/bin/env python3
"""G4's table: the R-k marks "proved by the search", before and after D470, per config and slice kind.

What it is for
--------------
The E12-R close-out (the coordinator's task `wipemark-task-recon-closeout-grok-ready-2026-10-10`,
2026-10-10; `docs/plan/E12-R-closeout-grok-ready.md`, G4) asks the bench rerun on the merged `feat` to
confirm that "proved by the search" reads 0 for every config after D470. This script puts the bench's
§7 table of 2026-10-09 (before D470, `E12-R-container-runs-2026-10-09/r5/bench-r0-2026-10-09.md`) beside
the close-out's (`E12-R-closeout-2026-10-10/bench/bench-2026-10-10.md`), summed per config over the
slices where the mark sits at its row and over the resampled slices, so the report's figure is one a
script reproduces.

What it does
------------
1. Reads the `R-k` table of §7 from each report (the lines after "A mark drawn at k = 0.93").
2. Per config, sums n, "refused by gain", "proved at the row", "proved by the search" and "not found"
   over two kinds of slice: `at its row` (png, every JPEG and WebP without a resize) and `resampled`
   (`resize-0.9`, `resize-1.1`, `jpeg420-q90+resize-0.9`).
3. Prints a Markdown table: config, kind, n, then each count before → after.

How to run it
-------------
    python3 docs/plan/reports/E12-R-closeout-2026-10-10-g4.py [BEFORE.md AFTER.md]

What it needs
-------------
Python 3, the standard library. The two reports are committed.

What its output means
---------------------
"proved by the search" after D470 is 0 at the row and unchanged on the resampled slices: D470 refuses a
search proof only where a row of the profile refused the mark by gain at that row's place, and a
resampled mark is not at a row's place.
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
BEFORE = os.path.join(HERE, "E12-R-container-runs-2026-10-09", "r5", "bench-r0-2026-10-09.md")
AFTER = os.path.join(HERE, "E12-R-closeout-2026-10-10", "bench", "bench-2026-10-10.md")
COLS = ["n", "refused by gain", "proved at the row", "proved by the search", "not found"]


def rk(path):
    rows, on = {}, False
    with open(path) as f:
        for line in f:
            if line.startswith("A mark drawn at k = 0.93"):
                on = True
                continue
            if on and line.startswith("## "):
                break
            if on and line.startswith("| R") and "---" not in line:
                c = [x.strip() for x in line.strip().strip("|").split("|")]
                cfg, sl = c[0], c[1]
                kind = "resampled" if "resize" in sl else "at its row"
                acc = rows.setdefault((cfg, kind), [0] * len(COLS))
                for i, v in enumerate(c[3:8]):
                    acc[i] += int(v)
    return rows


def main(argv):
    before, after = (argv[1], argv[2]) if len(argv) == 3 else (BEFORE, AFTER)
    a, b = rk(before), rk(after)
    print("| config | slices | n | refused by gain | proved at the row | proved by the search | not found |")
    print("|---|---|---|---|---|---|---|")
    for key in sorted(set(a) | set(b)):
        x, y = a.get(key), b.get(key)
        cells = []
        for i in range(len(COLS)):
            xs = "–" if x is None else str(x[i])
            ys = "–" if y is None else str(y[i])
            cells.append(ys if xs == ys else f"{xs} → {ys}")
        print(f"| {key[0]} | {key[1]} | " + " | ".join(cells) + " |")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
