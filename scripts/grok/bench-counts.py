#!/usr/bin/env python3
"""The restoration bench's counts for Grok on X, per slice and per size (E12-6 for Grok, G5).

What it is for, and who asked
-----------------------------
The coordinator's task `wipemark-task-grok-x-mark-2026-10-11` (G5, 2026-10-11)
asks for the bench with the shipped Grok profiles — the white mark over the
bench's 60 flat and text backgrounds, the black one over their negatives, as
the FDnCNN step ran it (`dev/plan/reports/E12-R10-grok-fdncnn-2026-10-11.sh`),
slices png, jpeg420-q95 (X's own re-encode), jpeg420-q90 and resize-0.9, the
product's config R8d — and for its proved / restored / refused counts and the
median and 5th-percentile PSNR over the mark's ROI. `scripts/bench/report.py`
writes the series' long report; this prints the few numbers the task asks for,
one table per run, from the same `results.jsonl`.

What it does
------------
1. Reads `recon_bench run`'s lines (one per case, slice and file).
2. Keeps one blend model (`--model`, `encoded` by default: the profile's own;
   `linear-light` is the bench's second model, not what X draws) and one
   config (`--config`, `R8d`).
3. Per slice: cases; proved (`detection.verdict` verified); restored (a
   result written, `restored`); refused, by reason (`detection.refusal.why`);
   not found; a mark left after a restoration (`marks_left` with `restored`)
   and why — outline, texture, a smoothed patch (D496), holes; the PSNR over
   the ROI of every restored case, median and 5th percentile, against the
   input's. Then the same per size (the case's canvas), for the row slices.

How to run it
-------------
    python3 scripts/grok/bench-counts.py RESULTS.jsonl [MORE.jsonl …] [--model encoded] [--config R8d]
    python3 scripts/grok/bench-counts.py selftest

`selftest` counts four made-up lines — a proof restored clean, one restored
with a smoothed patch left, a refusal by edges, a mark not found, and a
line of the other model left out — and checks the table's row.

What it needs
-------------
Python 3.11+, nothing else.

What its output means
---------------------
Markdown tables on stdout: per run, then per size. "proved" counts the second
proof's verdict; "restored" what the product wrote; "left" the restorations
the product says still carry the mark (exit 3). Exit status: 0, or 2 for a
file that cannot be read.
"""

import argparse
import collections
import json
import statistics
import sys

SLICES = ("png", "jpeg420-q95", "jpeg420-q90", "resize-0.9")


def pct(values, q):
    if not values:
        return None
    v = sorted(values)
    return v[min(len(v) - 1, max(0, round(q * (len(v) - 1))))]


def left_why(m):
    why = []
    if m.get("outline_left"):
        why.append("outline")
    if m.get("texture_left"):
        why.append("texture")
    if m.get("smoothed"):
        why.append("smoothed")
    if m.get("holes"):
        why.append("holes")
    return "+".join(why) or "other"


def table(lines, key, keys):
    out = ["| " + key + " | cases | proved | restored | refused (why) | not found | left after a restoration (why) | "
           "PSNR_ROI median / p5 (input median) |", "|---|---|---|---|---|---|---|---|"]
    for k in keys:
        ls = [l for l in lines if l["_key"] == k]
        if not ls:
            continue
        det = [l.get("detection") or {} for l in ls]
        proved = sum(d.get("verdict") == "verified" for d in det)
        restored = [l for l in ls if l.get("restored")]
        refused = collections.Counter((d.get("refusal") or {}).get("why") for d in det if d.get("verdict") == "refused")
        notfound = sum(not d.get("found") for d in det)
        left = collections.Counter(left_why(l.get("measures") or {}) for l in restored if l.get("marks_left"))
        ps = [l["psnr_roi"] for l in restored if l.get("psnr_roi") is not None]
        pin = [l["psnr_roi_input"] for l in restored if l.get("psnr_roi_input") is not None]
        ref = ", ".join(f"{w} {n}" for w, n in refused.most_common()) or "—"
        lft = ", ".join(f"{w} {n}" for w, n in left.most_common()) or "—"
        psnr = (f"{statistics.median(ps):.1f} / {pct(ps, 0.05):.1f} ({statistics.median(pin):.1f})" if ps else "—")
        out.append(f"| {k} | {len(ls)} | {proved} | {len(restored)} | {sum(refused.values())} ({ref}) | {notfound} | "
                   f"{sum(left.values())} ({lft}) | {psnr} |")
    return "\n".join(out)


def selftest():
    def line(verdict, restored=False, left=False, why=None, found=True, model="encoded", psnr=50.0):
        return {"model": model, "config": "R8d", "slice": "png", "size": [784, 1168], "restored": restored,
                "marks_left": left, "psnr_roi": psnr if restored else None, "psnr_roi_input": 20.0,
                "measures": {"smoothed": left}, "case": "x.encoded",
                "detection": {"found": found, "verdict": verdict, "refusal": {"why": why} if why else None}}
    lines = [line("verified", True), line("verified", True, True), line("refused", why="edges"),
             line(None, found=False), line("verified", True, model="linear-light")]
    lines = [l for l in lines if l["model"] == "encoded"]
    for l in lines:
        l["_key"] = l["slice"]
    row = table(lines, "slice", SLICES).splitlines()[2]
    want = "| png | 4 | 2 | 2 | 1 (edges 1) | 1 | 1 (smoothed 1) | 50.0 / 50.0 (20.0) |"
    print(("ok  " if row == want else "FAIL") + f" {row}")
    return 0 if row == want else 1


def main():
    if sys.argv[1:] == ["selftest"]:
        sys.exit(selftest())
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("results", nargs="+")
    ap.add_argument("--model", default="encoded")
    ap.add_argument("--config", default="R8d")
    a = ap.parse_args()
    for path in a.results:
        try:
            lines = [json.loads(x) for x in open(path) if x.strip()]
        except (OSError, ValueError) as e:
            print(f"bench-counts: {path}: {e}", file=sys.stderr)
            sys.exit(2)
        lines = [l for l in lines if l.get("model") == a.model and l.get("config") == a.config]
        profiles = sorted({l.get("profile") for l in lines})
        print(f"\n### {path} — {', '.join(map(str, profiles))}, {a.model}, {a.config}\n")
        for l in lines:
            l["_key"] = l.get("slice")
        print(table(lines, "slice", SLICES))
        rows = [l for l in lines if l.get("slice") != "resize-0.9"]
        for l in rows:
            l["_key"] = "×".join(str(v) for v in l.get("size") or []) or l["case"].split(".")[0]
        print()
        print(table(rows, "size (row slices)", sorted({l["_key"] for l in rows})))


if __name__ == "__main__":
    main()
