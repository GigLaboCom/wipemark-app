#!/usr/bin/env python3
"""D530's bench tables: the search's proofs with its gain windows off and on.

What it is for
--------------
The owner's question of 2026-10-11 (the coordinator's task
`recon/search-gain-resampled`; report
`docs/plan/reports/E12-R-search-gain-2026-10-11.md`). The bench is run once
per catalogue — the search's gain windows off, and at each value compared —
by `E12-R-search-gain-2026-10-11-bench.sh`; this script puts the runs side
by side so every figure of the report is one a script reproduces.

What it does
------------
1. Reads every `results-<catalogue>-<slices>.jsonl` given (the name says
   which catalogue: `off`, `0.98`, `1.00`, `bands` …; error lines counted
   and skipped).
2. Table 1, the `R-k` marks (drawn at k = 0.93, D154; the close-out's G4
   columns): per catalogue, config and kind of slice (`at its row` or
   `resampled`), n, refused by gain, proved at the row, proved by the
   search, not found, and of those the search proved, the median ROI PSNR
   and how many the restoration says left a mark (`marks_left`).
3. Table 2, the marks drawn at k = 1 with the profile's own blend
   (`encoded`): per catalogue, config, kind and row, n, proved (a verified
   finding at the mark's place), proved by the search, refused by gain, and
   the median ROI PSNR of the restored.
4. Table 3, what moved: per config and kind, against `off`, how many cases
   went from proved to not proved and the other way, by drawn k.

How to run it
-------------
    python3 docs/plan/reports/E12-R-search-gain-2026-10-11-tally.py bench/out/search-gain/results-*.jsonl
    python3 docs/plan/reports/E12-R-search-gain-2026-10-11-tally.py selftest

What it needs
-------------
Python 3.10+, the standard library.

What its output means
---------------------
Markdown tables. The report quotes them: the weak marks' "proved by the
search" on the resampled slices should fall from 148/165 towards 0 under the
chosen windows, every at-row figure should read as with the windows off, and
the k = 1 marks lost (proved → refused by gain) are the windows' price.
"""

import collections
import json
import os
import re
import statistics
import sys


def kind_of(slice_id):
    return "resampled" if "resize" in slice_id else "at its row"


def catalogue_of(path):
    m = re.match(r"results-(.+)-(resampled|atrow)\.jsonl$", os.path.basename(path))
    return m.group(1) if m else os.path.basename(path)


def read(paths):
    rows, errors = [], 0
    for p in paths:
        cat = catalogue_of(p)
        with open(p) as f:
            for line in f:
                if not line.strip():
                    continue
                r = json.loads(line)
                if "error" in r:
                    errors += 1
                    continue
                r["_cat"] = cat
                rows.append(r)
    return rows, errors


def med(xs):
    xs = [x for x in xs if x is not None]
    return round(statistics.median(xs), 2) if xs else "–"


def proved(r, placed=None):
    d = r.get("detection") or {}
    return d.get("verdict") == "verified" and (placed is None or d.get("placed") == placed)


def refused_by_gain(r):
    return ((r.get("detection") or {}).get("refusal") or {}).get("why") == "gain"


def table(head, rows):
    out = ["| " + " | ".join(head) + " |", "|" + "---|" * len(head)]
    out += ["| " + " | ".join(str(c) for c in row) + " |" for row in rows]
    return "\n".join(out)


def tables(rows):
    md = []
    weak = [r for r in rows if float(r.get("k") or 1) != 1.0]
    g = collections.defaultdict(list)
    for r in weak:
        g[(r["_cat"], r["config"], kind_of(r["slice"]))].append(r)
    t1 = []
    for (cat, cfg, kind), rs in sorted(g.items()):
        s = [r for r in rs if proved(r, "searched")]
        t1.append([cat, cfg, kind, len(rs), sum(map(refused_by_gain, rs)), sum(proved(r, "row") for r in rs),
                   len(s), sum(1 for r in rs if not (r.get("detection") or {}).get("found")),
                   med([r["psnr_roi"] for r in s]), sum(1 for r in s if r.get("marks_left"))])
    md += ["### The R-k marks (k = 0.93)", "", table(
        ["catalogue", "config", "slices", "n", "refused by gain", "proved at the row", "proved by the search",
         "not found", "PSNR med (search)", "said left (search)"], t1), ""]

    strong = [r for r in rows if float(r.get("k") or 1) == 1.0 and r.get("model") == "encoded"]
    g = collections.defaultdict(list)
    for r in strong:
        g[(r["_cat"], r["config"], kind_of(r["slice"]), r["row"])].append(r)
    t2 = []
    for (cat, cfg, kind, row), rs in sorted(g.items()):
        t2.append([cat, cfg, kind, row, len(rs), sum(proved(r) for r in rs), sum(proved(r, "searched") for r in rs),
                   sum(map(refused_by_gain, rs)), med([r["psnr_roi"] for r in rs if r.get("restored")])])
    md += ["### The marks at the profile's own opacity (k = 1, encoded)", "", table(
        ["catalogue", "config", "slices", "row", "n", "proved", "proved by the search", "refused by gain",
         "PSNR med (restored)"], t2), ""]

    by_case = collections.defaultdict(dict)
    for r in rows:
        key = (r["config"], r["case_dir"], r["slice"], r["encoder"])
        by_case[key][r["_cat"]] = r
    moved = collections.Counter()
    for (cfg, _, sl, _), cats in by_case.items():
        base = cats.get("off")
        if base is None:
            continue
        for cat, r in cats.items():
            if cat == "off":
                continue
            a, b = proved(base), proved(r)
            if a != b:
                moved[(cat, cfg, kind_of(sl), f"k={float(r.get('k') or 1):.2f} {r.get('model')}",
                       "proved → not" if a else "not → proved")] += 1
    t3 = [list(k) + [n] for k, n in sorted(moved.items())]
    md += ["### What moved against `off`", "", table(["catalogue", "config", "slices", "drawn", "how", "cases"], t3)
           if t3 else "Nothing moved.", ""]
    return "\n".join(md)


def selftest():
    def r(cat, k, sl, verdict, placed, why=None, model="encoded", row="v1-96", case="c1"):
        det = {"found": verdict is not None, "verdict": verdict, "placed": placed,
               "refusal": {"why": why} if why else None}
        return {"_cat": cat, "k": k, "slice": sl, "config": "R0", "detection": det, "psnr_roi": 40.0,
                "restored": verdict == "verified", "marks_left": False, "model": model, "row": row,
                "case_dir": case, "encoder": "pillow"}
    rows = [r("off", 0.93, "resize-0.9", "verified", "searched"),
            r("bands", 0.93, "resize-0.9", "refused", "searched", "gain"),
            r("off", 1.0, "resize-0.9", "verified", "searched", case="c2"),
            r("bands", 1.0, "resize-0.9", "verified", "searched", case="c2"),
            r("off", 0.93, "png", "refused", "row", "gain", case="c3"),
            r("bands", 0.93, "png", "refused", "row", "gain", case="c3")]
    md = tables(rows)
    problems = []
    if "| bands | R0 | resampled | 1 | 1 | 0 | 0 | 0 |" not in md:
        problems.append("the weak resampled row under bands")
    if "| off | R0 | resampled | 1 | 0 | 0 | 1 | 0 | 40.0 | 0 |" not in md:
        problems.append("the weak resampled row off")
    if "| bands | R0 | resampled | k=0.93 encoded | proved → not | 1 |" not in md:
        problems.append("the move")
    if "k=1.00 encoded | proved" in md:
        problems.append("a k = 1 case that did not move is listed")
    if catalogue_of("x/results-1.00-atrow.jsonl") != "1.00":
        problems.append("catalogue_of")
    for p in problems:
        print("FAIL", p)
    print("ok" if not problems else f"{len(problems)} failed")
    return 1 if problems else 0


def main(argv):
    if argv[:1] == ["selftest"]:
        return selftest()
    if not argv:
        print(__doc__.split("How to run it")[1].split("What it needs")[0])
        return 2
    rows, errors = read(argv)
    print(f"{len(rows)} results, {errors} errors\n")
    print(tables(rows))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
