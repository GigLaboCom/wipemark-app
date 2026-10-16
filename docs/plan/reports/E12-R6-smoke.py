#!/usr/bin/env python3
"""R6 against R0, file by file, over a recon_bench run (E12-R6's smoke).

What it is for
--------------
Written by the agent implementing E12-R6 (the planar inverse), 2026-10-09,
for the coordinator of `GigLaboCom/wipemark-app`. `scripts/bench/report.py`
gives each config's slice medians and the gates A1-A7; on the subsampled
slices those medians mix two effects — files R6 now restores that R0
refused, and files both restore, restored better. This separates them, and
says how each refusal R6 lifts ends (the exit and what is said left), which
level B's L2/L3 ask about, and what R6 does to the `R-k` composites
(D154 through the search, E12-R5's finding). Figures from a smoke run in the
container are a smoke, not results; the host's full run is level A.

What it does
------------
Reads a `results.jsonl` holding configs R0 and R6 (one `recon_bench run`
with `--config R0 --config R6`). Pairs every file of R0 with the same file
(case, slice, encoder, background) under R6. For each slice it prints:

1. pairs, how many each config restored, and how many R6 lifted (R0 not
   restored, R6 restored) or dropped (the other way);
2. over the files both restored, k = 1 and `encoded`: the median and the
   5th percentile of `psnr_roi(R6) - psnr_roi(R0)`, and the median ΔE2000
   change; the median `chroma` measure under each;
3. of the lifted files: their R0 refusal, and R6's exit and what is said
   left (`outline_left`, `texture_left`);
4. the `R-k` composites (k = 0.93): verified at the row / by the search
   under each config;
5. whether any file outside the subsampled JPEG slices differs at all
   (`restored_sha256`) — the route's byte-equality (A4 / L1).

Usage
-----
    python3 docs/plan/reports/E12-R6-smoke.py RESULTS.jsonl

Needs only the Python standard library.

Output
------
A plain-text table per slice. "lifted" are refusals R6 turned into
restorations; a lifted file that ends at exit 3 goes to `attention` in
level B, not to success.
"""
import collections
import json
import statistics
import sys


def key(r):
    return (r["case_dir"], r["slice"], r["encoder"], r["file"])


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: E12-R6-smoke.py RESULTS.jsonl")
    rows = [json.loads(l) for l in open(sys.argv[1]) if l.strip()]
    by = {c: {key(r): r for r in rows if r.get("config") == c and "error" not in r} for c in ("R0", "R6")}
    if not by["R0"] or not by["R6"]:
        sys.exit("the file needs both R0 and R6")
    slices = sorted({k[1] for k in by["R0"]})
    moved_elsewhere = 0
    for sl in slices:
        pairs = [(by["R0"][k], by["R6"][k]) for k in by["R0"] if k[1] == sl and k in by["R6"]]
        enc = [(a, b) for a, b in pairs if float(a.get("k", 1)) == 1.0 and a.get("model") == "encoded"]
        both = [(a, b) for a, b in enc if a["restored"] and b["restored"]]
        lifted = [(a, b) for a, b in enc if not a["restored"] and b["restored"]]
        dropped = [(a, b) for a, b in enc if a["restored"] and not b["restored"]]
        if not sl.startswith("jpeg420"):
            moved_elsewhere += sum(a["restored_sha256"] != b["restored_sha256"] for a, b in pairs)
        print(f"== {sl}: {len(pairs)} pairs, encoded k=1 {len(enc)}: R0 restored {sum(a['restored'] for a, _ in enc)}, "
              f"R6 {sum(b['restored'] for _, b in enc)}; lifted {len(lifted)}, dropped {len(dropped)}")
        if both:
            d = sorted(b["psnr_roi"] - a["psnr_roi"] for a, b in both)
            de = [b["de2000_roi"] - a["de2000_roi"] for a, b in both]
            p5 = d[max(0, round(0.05 * (len(d) - 1)))]
            ch0 = statistics.median(a["measures"]["chroma"] for a, _ in both)
            ch6 = statistics.median(b["measures"]["chroma"] for _, b in both)
            print(f"   both restored ({len(both)}): ΔPSNR median {statistics.median(d):+.2f} dB, p5 {p5:+.2f} dB; "
                  f"ΔE2000 median {statistics.median(de):+.2f}; chroma median R0 {ch0:.2f} → R6 {ch6:.2f}")
        if lifted:
            why = collections.Counter((a["detection"].get("refusal") or {}).get("why", "not found") for a, _ in lifted)
            ends = collections.Counter(
                (b["exit"], bool(b["measures"]["outline_left"]), bool(b["measures"]["texture_left"])) for _, b in lifted)
            print(f"   lifted: R0 refused by {dict(why)}; R6 (exit, outline_left, texture_left): {dict(ends)}")
        rk = [(a, b) for a, b in pairs if abs(float(a.get("k", 1)) - 0.93) < 1e-3]
        if rk:
            def where(r):
                d = r["detection"]
                if d.get("verdict") != "verified":
                    return "not proved"
                return f"proved ({d.get('placed')})"
            c0 = collections.Counter(where(a) for a, _ in rk)
            c6 = collections.Counter(where(b) for _, b in rk)
            print(f"   R-k 0.93 ({len(rk)}): R0 {dict(c0)}; R6 {dict(c6)}")
    print(f"== outside jpeg420-*: {moved_elsewhere} restored rasters differ between R0 and R6")


if __name__ == "__main__":
    main()
