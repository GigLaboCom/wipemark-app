#!/usr/bin/env python3
"""Is the opacity gain `k` stable? — `k*` over a corpus, per profile and row.

What it is for
--------------
Step E12-R4 of the E12-R series (`docs/plan/E12-R4-corpus-analytics.md`
§4.1), filed 2026-10-08 by the coordinator from the owner's spec
`wipemark-recon-spec-2026-10-08` (`04-corpus-analytics.md` §1; the owner's
S8: D154 stays — `k != 1` is another profile, never a per-image `k`). The
second proof sweeps a gain `k` over 0..1.6 by 0.02 and keeps
`k* = argmin E` (`crates/wipemark-pixels/src/verify.rs`); a mark is proved
only when `|k* - 1| <= 0.06`. If the vendor's opacity drifted from file to
file, `k*` would show it. This script reads `k*` off the CLI's
`inspect --json` — `found[].scores.gain`, and `found[].refusal.k` for a
refusal by gain — over R1's baseline, and says whether `k` has one mode
near 1, a second mode tied to a size or a source, or no mode at all.

What it does
------------
1. Reads every record of the folders given: R1's baseline
   (`golden/baseline/<commit>/<id>.json`, the shape `scripts/regress.py`
   writes — `class`, `variant` and the CLI's `inspect` JSON), or a folder of
   bare `wipemark-cli inspect --json` outputs. `--class` keeps only those
   classes (default: `recon-png` and `gemini-midtone`, §4.1's input);
   `--variant` only those variants (`recon-png:png` is the 21 originals,
   `fixture-png` the four committed crops of four of them).
2. Takes every pass-1 finding of the visible pass: its profile, its row
   (`searched` when the search placed it), its size, its verdict, and `k*`
   (`scores.gain`; a refusal by gain carries the same number as
   `refusal.k`). A finding is **accepted** (verified), **refused by gain**,
   or **refused otherwise** (edges, out of range, transparent, opaque).
   Another profile that lost the same place (`also_tried`) carries only a
   refusal; its `k` is listed apart and never pooled — V2 refused by gain on
   a V1 picture is not a V1 mark drifting.
3. Per profile and row, prints: a histogram of `k*` by 0.01 (accepted,
   refused by gain and refused otherwise apart; `k*` lies on the sweep's
   0.02 grid, so every other bin is empty by construction), the median and
   the MAD (the median of `|k - median|`, unscaled), the share outside
   `|k - 1| <= 0.06`, the modes, and for the refused, `k*` against the size
   and the source (class and variant).
4. Ends each profile and row with a **mechanical reading** against §4.1's
   table: one mode within 0.06 of 1 holding at least 90 % of the findings
   with MAD <= 0.02 is row 1 (stable); two or more modes, each with
   MAD <= 0.02, is row 2 (another profile — the sizes and sources of the
   second mode are printed); anything else is row 3 (a scatter). The
   reading is arithmetic; the conclusion is the report's.
5. With `--out DIR`, writes `gain.md` (all of the above) and `gain.csv`
   (one line per finding: id, class, variant, profile, row, size, verdict,
   why, k, and `also_tried` rows marked as such).

How to run it
-------------
    python3 scripts/analytics/gain.py run golden/baseline/<commit>/ [MORE_DIRS …] \
        [--class recon-png --class gemini-midtone] [--all-classes] [--variant GLOB …] [--out DIR]
    python3 scripts/analytics/gain.py selftest

What it needs
-------------
Python 3.10+ and nothing else (no numpy, no Pillow). The input is R1's
baseline, taken by the host with a release CLI (`scripts/regress.py
baseline`); the owner's pictures never go into git, and this script never
opens a picture.

What its output means
---------------------
Row 1 — one mode near 1, MAD <= 0.02: `k` is stable; D154 confirmed,
nothing to do. Row 2 — a second mode (e.g. 0.92) with MAD <= 0.02, tied to
a size or a tier: another profile — a new map or profile by D154 (an
R11-shaped calibration), never a per-image `k` (S8). Row 3 — a wide
scatter with no mode, depending on the background: not opacity but an
error of the map or `L` — §4.4 and R9's R-lm; `k` untouched. Exit codes are
the repository's four: 0 done, 2 usage or nothing to read.
"""

import argparse
import csv
import fnmatch
import io
import json
import os
import statistics
import sys

DEFAULT_CLASSES = ("recon-png", "gemini-midtone")
GAIN_TOL = 0.06  # the shipped profiles' `verify.gain`
STABLE_MAD = 0.02  # §4.1: MAD <= 0.02
STABLE_SHARE = 0.90  # row 1: the mode near 1 holds this share  [tunable]
MODE_SHARE = 0.10  # a mode holds at least this share of the findings  [tunable]
MODE_MIN = 3  # … and at least this many  [tunable]
MODE_WINDOW = 0.05  # a mode's members lie within this of it  [tunable]
EPS = 1e-6


def records(dirs):
    """(id, class, variant, inspect JSON) for every JSON record in the folders."""
    out = []
    for d in dirs:
        names = sorted(n for n in os.listdir(d) if n.endswith(".json"))
        for n in names:
            if n in ("index.json", "summary.json", "reproduce.json"):
                continue
            with open(os.path.join(d, n)) as f:
                try:
                    rec = json.load(f)
                except ValueError:
                    continue
            if isinstance(rec, dict) and "inspect" in rec:
                ij = (rec.get("inspect") or {}).get("json")
                out.append((rec.get("id") or n[:-5], rec.get("class"), rec.get("variant"), ij))
            elif isinstance(rec, dict) and "visible" in rec:
                out.append((n[:-5], None, None, rec))
    return out


def findings(recs, classes):
    """Every pass-1 finding, and every `also_tried` refusal with a `k`, as dicts."""
    main, lost = [], []
    for fid, cls, variant, ij in recs:
        if classes is not None and cls not in classes:
            continue
        vis = ij.get("visible") if isinstance(ij, dict) else None
        if not isinstance(vis, dict):
            continue
        for f in vis.get("found") or []:
            if f.get("pass", 1) != 1:
                continue
            scores = f.get("scores") or {}
            refusal = f.get("refusal") or {}
            k = scores.get("gain")
            if not isinstance(k, (int, float)):
                k = refusal.get("k")
            if not isinstance(k, (int, float)):
                continue  # refused before anything was measured (opaque)
            if f.get("verdict") == "verified":
                verdict = "accepted"
            elif refusal.get("why") == "gain":
                verdict = "refused-gain"
            else:
                verdict = "refused-other"
            row = f.get("row")
            main.append({
                "id": fid, "class": cls, "variant": variant, "profile": f.get("profile"),
                "row": "searched" if row is None else str(row),
                "size": (f.get("rect") or {}).get("size"), "verdict": verdict,
                "why": refusal.get("why"), "k": float(k),
            })
            for t in f.get("also_tried") or []:
                r = t.get("refusal") or {}
                if isinstance(r.get("k"), (int, float)):
                    lost.append({
                        "id": fid, "class": cls, "variant": variant, "profile": t.get("profile"),
                        "lost_to": f.get("profile"), "size": (f.get("rect") or {}).get("size"),
                        "why": r.get("why"), "k": float(r["k"]),
                    })
    return main, lost


def bin_of(k):
    return round(round(k / 0.01) * 0.01, 2)


def mad(values):
    m = statistics.median(values)
    return statistics.median([abs(v - m) for v in values])


def modes(values):
    """Local maxima of the 0.01 histogram, smoothed over three 0.02 steps, that
    hold at least MODE_SHARE of the values; each with its members and their MAD."""
    n = len(values)
    hist = {}
    for v in values:
        hist[bin_of(v)] = hist.get(bin_of(v), 0) + 1
    def near(b, w):
        return sum(c for k, c in hist.items() if abs(k - b) <= w + EPS)
    peaks = []
    for b in sorted(hist):
        mass = near(b, 0.02)
        if mass < max(MODE_MIN, MODE_SHARE * n):
            continue
        # a local maximum of the smoothed mass, ties to the bin with more of its own
        better = [o for o in hist if 0 < abs(o - b) <= 0.04 + EPS and
                  (near(o, 0.02), hist[o]) > (mass, hist[b])]
        if not better:
            peaks.append(b)
    out = []
    for p in sorted(peaks):
        if out and abs(p - out[-1]["at"]) <= 0.04 + EPS:
            continue
        members = [v for v in values if abs(v - p) <= MODE_WINDOW + EPS]
        out.append({"at": p, "n": len(members), "share": len(members) / n,
                    "median": statistics.median(members), "mad": mad(members)})
    return out


def reading(values, ms):
    if not values:
        return "nothing to read"
    near_one = [m for m in ms if abs(m["at"] - 1.0) <= GAIN_TOL + EPS]
    if (len(ms) == 1 and near_one and near_one[0]["mad"] <= STABLE_MAD + EPS
            and near_one[0]["share"] >= STABLE_SHARE):
        return "row 1 of §4.1 — one mode near 1, k stable"
    if len(ms) >= 2 and all(m["mad"] <= STABLE_MAD + EPS for m in ms):
        return "row 2 of §4.1 — modes at " + ", ".join(f"{m['at']:.2f}" for m in ms) + ": another profile"
    return "row 3 of §4.1 — no single tight mode: a scatter"


def analyse(main, lost):
    """The report as text, and the CSV rows."""
    out = io.StringIO()
    groups = {}
    for f in main:
        groups.setdefault((f["profile"], f["row"]), []).append(f)
    if not groups:
        print("No finding with a k* in the records given.", file=out)
    for (profile, row), fs in sorted(groups.items(), key=lambda kv: (str(kv[0][0]), kv[0][1])):
        ks = [f["k"] for f in fs]
        print(f"## {profile}, row {row}: {len(fs)} finding(s)", file=out)
        counts = {v: sum(1 for f in fs if f["verdict"] == v) for v in ("accepted", "refused-gain", "refused-other")}
        print("  " + ", ".join(f"{v} {c}" for v, c in counts.items()), file=out)
        print("  k*     accepted  refused-gain  refused-other", file=out)
        lo, hi = bin_of(min(ks)), bin_of(max(ks))
        b = lo
        while b <= hi + EPS:
            row_counts = [sum(1 for f in fs if f["verdict"] == v and bin_of(f["k"]) == round(b, 2))
                          for v in ("accepted", "refused-gain", "refused-other")]
            if any(row_counts) or len(ks) < 60:
                print(f"  {b:4.2f}   {row_counts[0]:8d}  {row_counts[1]:12d}  {row_counts[2]:13d}", file=out)
            b = round(b + 0.01, 2)
        outside = sum(1 for k in ks if abs(k - 1.0) > GAIN_TOL + EPS)
        print(f"  median {statistics.median(ks):.3f}  MAD {mad(ks):.3f}  "
              f"outside |k-1| <= {GAIN_TOL}: {outside} of {len(ks)} ({100.0 * outside / len(ks):.1f} %)", file=out)
        for v in ("accepted", "refused-gain"):
            sub = [f["k"] for f in fs if f["verdict"] == v]
            if sub:
                print(f"  {v}: median {statistics.median(sub):.3f}  MAD {mad(sub):.3f}  n {len(sub)}", file=out)
        ms = modes(ks)
        for m in ms:
            print(f"  mode at {m['at']:.2f}: {m['n']} ({100.0 * m['share']:.1f} %), "
                  f"median {m['median']:.3f}, MAD {m['mad']:.3f}", file=out)
        if len(ms) >= 2:
            for m in ms[1:] if abs(ms[0]["at"] - 1.0) <= GAIN_TOL + EPS else ms:
                members = [f for f in fs if abs(f["k"] - m["at"]) <= MODE_WINDOW + EPS]
                sizes = sorted({f["size"] for f in members}, key=lambda s: (s is None, s))
                sources = sorted({f"{f['class']}:{f['variant']}" for f in members})
                print(f"  mode at {m['at']:.2f}: sizes {sizes}, sources {sources}", file=out)
        refused = [f for f in fs if f["verdict"] != "accepted"]
        if refused:
            print("  refused — k* against the size and the source:", file=out)
            for f in sorted(refused, key=lambda f: (f["k"], str(f["id"]))):
                print(f"    k* {f['k']:.2f}  size {f['size']}  {f['class']}:{f['variant']}  {f['id']}  ({f['why']})", file=out)
        print(f"  mechanical reading: {reading(ks, ms)}", file=out)
        print(file=out)
    if lost:
        print(f"## also_tried — another profile that lost the place, never pooled: {len(lost)}", file=out)
        by = {}
        for f in lost:
            by.setdefault((f["profile"], f["lost_to"], f["why"]), []).append(f["k"])
        for (p, to, why), ks in sorted(by.items(), key=lambda kv: str(kv[0])):
            print(f"  {p} lost to {to} ({why}): n {len(ks)}, k* {min(ks):.2f}–{max(ks):.2f}, median {statistics.median(ks):.3f}", file=out)
    rows = [[f["id"], f["class"], f["variant"], f["profile"], f["row"], f["size"], f["verdict"], f["why"], f"{f['k']:.2f}"]
            for f in main]
    rows += [[f["id"], f["class"], f["variant"], f["profile"], "also_tried", f["size"], "lost-to-" + str(f["lost_to"]),
              f["why"], f"{f['k']:.2f}"] for f in lost]
    return out.getvalue(), rows


def cmd_run(args):
    for d in args.dirs:
        if not os.path.isdir(d):
            print(f"{d}: not a folder", file=sys.stderr)
            return 2
    classes = None if args.all_classes else set(args.cls or DEFAULT_CLASSES)
    recs = records(args.dirs)
    if args.variant:
        recs = [r for r in recs if any(fnmatch.fnmatch(str(r[2]), v) for v in args.variant)]
    if not recs:
        print("no record in the folders given", file=sys.stderr)
        return 2
    main, lost = findings(recs, classes)
    text, rows = analyse(main, lost)
    head = (f"# k* over {len(recs)} record(s) in {', '.join(args.dirs)}; classes "
            f"{'all' if classes is None else ', '.join(sorted(classes))}\n\n")
    print(head + text, end="")
    if args.out:
        os.makedirs(args.out, exist_ok=True)
        with open(os.path.join(args.out, "gain.md"), "w") as f:
            f.write(head + text)
        with open(os.path.join(args.out, "gain.csv"), "w", newline="") as f:
            w = csv.writer(f)
            w.writerow(["id", "class", "variant", "profile", "row", "size", "verdict", "why", "k"])
            w.writerows(rows)
    return 0


# ── selftest ─────────────────────────────────────────────────────────────────


def fake_record(fid, cls, variant, found):
    return {"id": fid, "class": cls, "variant": variant,
            "inspect": {"exit": 1, "json": {"visible": {"examined": True, "found": found, "restored": []}}}}


def fake_finding(k, verdict="verified", why=None, row=0, size=96.0, profile="gemini-sparkle-v1", also=()):
    refusal = None if why is None else {"why": why, **({"k": k} if why == "gain" else {})}
    return {"profile": profile, "pass": 1, "rect": {"x": 1888.0, "y": 1888.0, "size": size},
            "placed": "row" if row is not None else "searched", "row": row, "verdict": verdict, "refusal": refusal,
            "scores": {"gain": k, "edge_ratio": 0.07, "out_of_range": 0.0, "holes": 0}, "also_tried": list(also)}


def selftest():
    import tempfile
    failures = []

    def check(ok, what):
        print(("ok   " if ok else "FAIL ") + what)
        if not ok:
            failures.append(what)

    # 1. Stable: 21 at 1.00 and 1.02, 0.98 — row 1.
    ks = [1.0] * 15 + [1.02] * 3 + [0.98] * 3
    main, lost = findings([(f"s{i}", "recon-png", "png", fake_record(f"s{i}", "recon-png", "png", [fake_finding(k)])["inspect"]["json"])
                           for i, k in enumerate(ks)], {"recon-png"})
    text, _ = analyse(main, lost)
    check(len(main) == 21 and "median 1.000  MAD 0.000" in text, "a stable corpus: median 1.000, MAD 0.000")
    check("row 1 of §4.1" in text, "a stable corpus reads as row 1")
    check("outside |k-1| <= 0.06: 0 of 21" in text, "a stable corpus: none outside the tolerance")

    # 2. A second mode at 0.92, all of it at size 48 — row 2, the size named.
    fs = [fake_finding(k) for k in [1.0] * 12 + [1.02] * 2] + \
         [fake_finding(k, verdict="refused", why="gain", size=48.0, row=1) for k in [0.92] * 6 + [0.90, 0.94]]
    main, lost = findings([(f"m{i}", "gemini-midtone", "gray-50", {"visible": {"found": [f]}}) for i, f in enumerate(fs)],
                          {"gemini-midtone"})
    # one profile-and-row per size here; pool them to see the two modes
    for f in main:
        f["row"] = "pooled"
    text, _ = analyse(main, lost)
    check("row 2 of §4.1" in text and "modes at 0.92, 1.00" in text, "a second tight mode reads as row 2")
    check("sizes [48.0]" in text, "the second mode names its size")

    # 3. A scatter, 0.70..1.30 — row 3.
    ks = [0.70 + 0.02 * i for i in range(31)]
    main, lost = findings([(f"w{i}", "recon-png", "png", {"visible": {"found": [fake_finding(k)]}}) for i, k in enumerate(ks)],
                          None)
    text, _ = analyse(main, lost)
    check("row 3 of §4.1" in text, "a scatter reads as row 3")

    # 4. also_tried is listed apart and never pooled; a refusal by gain is counted as one.
    v2 = {"profile": "gemini-sparkle-v2", "verified": False, "refusal": {"why": "gain", "k": 0.72}}
    main, lost = findings([("a", "recon-png", "png", {"visible": {"found": [fake_finding(1.0, also=[v2])]}}),
                           ("b", "recon-png", "png", {"visible": {"found": [fake_finding(1.12, verdict="refused", why="gain")]}}),
                           ("c", "recon-png", "png", {"visible": {"found": [fake_finding(1.0, verdict="refused", why="out-of-range")]}})],
                          {"recon-png"})
    text, rows = analyse(main, lost)
    check(len(main) == 3 and len(lost) == 1 and lost[0]["k"] == 0.72, "also_tried's k is kept apart")
    check("accepted 1, refused-gain 1, refused-other 1" in text, "the three verdicts are counted apart")
    check("gemini-sparkle-v2 lost to gemini-sparkle-v1 (gain): n 1" in text, "also_tried is reported on its own")
    check(any(r[4] == "also_tried" for r in rows), "the CSV marks also_tried rows")

    # 5. The record shapes: R1's records and a bare CLI JSON, through files.
    with tempfile.TemporaryDirectory() as tmp:
        with open(os.path.join(tmp, "x.json"), "w") as f:
            json.dump(fake_record("x", "recon-png", "png", [fake_finding(1.0)]), f)
        with open(os.path.join(tmp, "bare.json"), "w") as f:
            json.dump({"container": "png", "visible": {"examined": True, "found": [fake_finding(0.98)], "restored": []}}, f)
        with open(os.path.join(tmp, "index.json"), "w") as f:
            json.dump({"files": ["x"]}, f)
        recs = records([tmp])
        check(len(recs) == 2, f"a record and a bare CLI JSON are both read, index.json skipped: {len(recs)}")
        main, _ = findings(recs, None)
        check(sorted(f["k"] for f in main) == [0.98, 1.0], "both findings' k* are read")
        main, _ = findings(recs, {"recon-png"})
        check(len(main) == 1, "--class keeps only the class asked for (a bare JSON has none)")

    # 6. A pass-2 finding and an opaque refusal (no scores) are not counted.
    opaque = {"profile": "gemini-sparkle-v1", "pass": 1, "rect": {"size": 96.0}, "row": 0, "verdict": "refused",
              "refusal": {"why": "opaque", "holes": 9}, "scores": None, "also_tried": []}
    second = dict(fake_finding(1.0), **{"pass": 2})
    main, _ = findings([("o", "recon-png", "png", {"visible": {"found": [opaque, second]}})], None)
    check(main == [], "a pass-2 finding and a refusal with no k are not counted")

    print(f"selftest: {'all passed' if not failures else str(len(failures)) + ' failed'}")
    return 1 if failures else 0


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("run", help="k* over R1 baseline folders")
    r.add_argument("dirs", nargs="+")
    r.add_argument("--class", dest="cls", action="append", help="keep this class (repeatable)")
    r.add_argument("--all-classes", action="store_true", help="every class, and bare CLI JSONs")
    r.add_argument("--variant", action="append",
                   help="keep records of this variant (repeatable, globs allowed): `png` is the 21 originals")
    r.add_argument("--out")
    sub.add_parser("selftest")
    args = ap.parse_args(argv)
    if args.cmd == "selftest":
        return selftest()
    return cmd_run(args)


if __name__ == "__main__":
    sys.exit(main())
