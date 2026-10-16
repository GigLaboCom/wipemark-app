#!/usr/bin/env python3
"""Does the search prove a weaker Grok mark on a resized picture? (E12-6 for Grok, G2/G5; D573)

What it is for, and who asked
-----------------------------
The coordinator's task `wipemark-task-grok-x-mark-2026-10-11` (G2) asks whether
D530's `search.gain` bands apply to Grok's profiles. D530 measured Gemini: a
mark resampled with its picture reads `k*` 1.02–1.06 in the search, one drawn at
`k = 0.93` reads 0.96–1.00 and was proved, so the bands `[1.02, 1.06]` keep the
weak one out. The restoration bench draws its `R-k` case only for a canonical,
unfitted row (`recon_bench.rs`, `cases`), and every Grok map is fitted, so the
bench has no weaker Grok mark to show. This script draws one: over the bench's
own clean canvases, the very map of each case at its row with `k = 0.93`, and
the same at `k = 1`; each resized by 0.9 the way the bench's slice is
(`scripts/bench/encode.py`, Pillow's bicubic to `round(0.9 × size)`, PNG); and
asks the real CLI what it proves.

What it does
------------
1. Reads a `recon_bench gen` run (`--run`): every case of the `encoded` model,
   its clean canvas (`gt.png`), its map (`meta.json`'s `map`, from
   `crates/wipemark-pixels/marks/grok/<map>.wma`), its rectangle and logo.
2. Draws the map as the bench does (`wipemark_pixels::drawn`: a sample under
   7/255 with no sample of 20/255 within two pixels to 0; then
   `I = round(k·α·L + (1 − k·α)·O)` per channel, clamped) at `k = 1` — checked
   against the bench's own `marked.png` (the largest difference, which must be
   0) — and at `--k` (0.93).
3. Writes, per case, the weak composite at its row (PNG) and both composites
   resized by 0.9 (PNG), under `--out`.
4. Runs `<cli> inspect <file> --json` on the three (`--jobs` at a time) and
   keeps, per file, the finding of the profile — verdict, how it was placed,
   `k*`, the refusal. With `--clean`, every resized picture proved is also
   cleaned (`<cli> clean <file> -o <file>.cleaned.png --json`), and what clean
   says is kept: whether a mark is left, and whether by an outline.
5. Prints a table: per kind (weak at the row, weak resized, full resized) the
   cases, proved, refused by reason, not found, and the `k*` of every proof —
   and what D530's bands would make of the resized proofs.

How to run it
-------------
    python3 scripts/grok/weak-resized.py --run reports/…/bench/run-a \\
        --cli target/release/wipemark-cli --out reports/…/weak [--k 0.93] [--jobs 8] [--clean]
    python3 scripts/grok/weak-resized.py selftest

`selftest` (no run, no CLI) checks the two pieces the bench is matched by:
`drawn` drops a faint sample far from the mark's body and keeps one beside
it, and `composite` blends `k·α` toward the logo and rounds half up, leaving
every pixel the map does not touch alone.

What it needs
-------------
Python 3.11+ with numpy and Pillow 12.3.0; a `recon_bench gen` run over a
Grok profile (its `gt.png` canvases and `meta.json`); the release CLI built
from this tree. Local only — never a CI step (the owner, 2026-10-11).

What its output means
---------------------
"proved" at the weak row or the weak resized is a mark of another opacity
proved — the failure D470 and D530 exist to prevent; "proved" on the full
resized is what a band would cost. `weak.jsonl` under `--out` holds every line.
Exit status: 0 done; 1 the `k = 1` composite is not the bench's; 2 a usage error.
"""

import argparse
import collections
import concurrent.futures
import json
import os
import struct
import subprocess
import sys

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
MARKS = os.path.join(REPO, "crates", "wipemark-pixels", "marks", "grok")
CAPTURE_NOISE = 7.0 / 255.0  # geometry.rs
BODY = 20.0 / 255.0
NEAR = 2


def read_wma(path):
    b = open(path, "rb").read()
    if b[:4] != b"WMA1":
        raise ValueError(f"{path}: not a .wma")
    w, h, depth = struct.unpack("<HHB", b[4:9])
    if depth == 16:
        v = np.frombuffer(b[9:9 + 2 * w * h], "<u2").astype(np.float64) / 65535.0
    else:
        v = np.frombuffer(b[9:9 + w * h], np.uint8).astype(np.float64) / 255.0
    return v.reshape(h, w)


def drawn(a):
    from numpy.lib.stride_tricks import sliding_window_view as sw

    body = np.pad(a >= BODY, NEAR)
    near = sw(body, (2 * NEAR + 1, 2 * NEAR + 1)).any(axis=(-1, -2))
    out = a.copy()
    out[(a > 0) & (a < CAPTURE_NOISE) & ~near] = 0.0
    return out


def composite(canvas, a, x, y, logo, k):
    out = canvas.astype(np.float64).copy()
    h, w = a.shape
    A = np.clip(k * a, 0, 1)[..., None]
    region = out[y:y + h, x:x + w]
    out[y:y + h, x:x + w] = A * np.array(logo) + (1 - A) * region
    sel = np.zeros(out.shape[:2], bool)
    sel[y:y + h, x:x + w] = A[..., 0] > 0
    res = canvas.astype(np.float64).copy()
    res[sel] = np.clip(np.floor(out[sel] + 0.5), 0, 255)
    return res.astype(np.uint8)


def resized(img, scale=0.9):
    im = Image.fromarray(img)
    w, h = im.size
    return im.resize((int(w * scale + 0.5), int(h * scale + 0.5)), Image.Resampling.BICUBIC)


def cleaned(cli, path, profile, out):
    """`clean` of one picture: whether it says a mark is left, and why."""
    if os.path.exists(out):
        os.remove(out)
    p = subprocess.run([cli, "clean", path, "-o", out, "--json"], capture_output=True, text=True)
    try:
        rep = json.loads(p.stdout)["report"]
    except (json.JSONDecodeError, KeyError):
        return {"clean_error": p.stderr.strip()[:200]}
    r = [x for x in (rep.get("visible") or {}).get("restored", []) if x["profile"] == profile]
    return {"clean_exit": p.returncode, "marks_left": rep.get("marks_left"),
            "outline_left": bool(r and r[0].get("outline_left")), "outline": r[0].get("outline") if r else None}


def inspect(cli, path, profile):
    p = subprocess.run([cli, "inspect", path, "--json"], capture_output=True, text=True)
    try:
        j = json.loads(p.stdout)
    except json.JSONDecodeError:
        return {"error": p.stderr.strip()[:200]}
    found = [f for f in (j.get("visible") or {}).get("found", []) if f["profile"] == profile]
    if not found:
        return {"found": False}
    f = found[0]
    return {"found": True, "verdict": f["verdict"], "placed": f["placed"], "k": (f.get("scores") or {}).get("gain"),
            "why": (f.get("refusal") or {}).get("why"), "width": f["rect"]["size"]}


def selftest():
    fails = []
    a = np.zeros((9, 9))
    a[4, 4] = 0.30  # the body
    a[4, 6] = 0.01  # faint, two pixels from it: kept
    a[0, 0] = 0.01  # faint and alone: dropped
    d = drawn(a)
    if not (d[4, 6] == 0.01 and d[0, 0] == 0.0 and d[4, 4] == 0.30):
        fails.append("drawn")
    canvas = np.full((9, 9, 3), 100, np.uint8)
    out = composite(canvas, d, 0, 0, [255.0, 255.0, 255.0], 0.5)
    want = np.floor(0.15 * 255 + 0.85 * 100 + 0.5)
    if not (out[4, 4, 0] == want and out[1, 1, 0] == 100 and out[0, 0, 0] == 100):
        fails.append("composite")
    for f in ("drawn", "composite"):
        print(("FAIL " if f in fails else "ok   ") + f)
    print("selftest: " + ("passed" if not fails else f"{len(fails)} failed"))
    return 1 if fails else 0


def lefts(proved):
    said = [r for r in proved if "marks_left" in r]
    left = sum(bool(r["marks_left"]) for r in said)
    outline = sum(r.get("outline_left", False) for r in said)
    return f"{left} of {len(said)} ({outline} by an outline)"


def main():
    if sys.argv[1:] == ["selftest"]:
        sys.exit(selftest())
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--run", required=True)
    ap.add_argument("--cli", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--k", type=float, default=0.93)
    ap.add_argument("--jobs", type=int, default=8)
    ap.add_argument("--clean", action="store_true", help="also clean every resized picture proved, and say what clean says")
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    cases = [json.loads(x) for x in open(os.path.join(a.run, "index.jsonl"))]
    cases = [c for c in cases if c["case"].endswith(".encoded")]
    jobs, worst = [], 0
    for c in cases:
        meta = json.load(open(os.path.join(a.run, c["case_dir"], "meta.json")))
        canvas = np.asarray(Image.open(os.path.join(a.run, c["bg_dir"], "gt.png")).convert("RGB"))
        amap = drawn(read_wma(os.path.join(MARKS, meta["map"] + ".wma")))
        x, y = meta["pixels"]["x"], meta["pixels"]["y"]
        full = composite(canvas, amap, x, y, meta["blend"]["logo"], 1.0)
        bench = np.asarray(Image.open(os.path.join(a.run, c["case_dir"], "marked.png")).convert("RGB"))
        worst = max(worst, int(np.abs(full.astype(int) - bench.astype(int)).max()))
        weak = composite(canvas, amap, x, y, meta["blend"]["logo"], a.k)
        stem = os.path.join(a.out, c["case_dir"].replace("/", "__"))
        files = {"weak-row": stem + ".weak.png", "weak-resized": stem + ".weak-r09.png",
                 "full-resized": stem + ".full-r09.png"}
        Image.fromarray(weak).save(files["weak-row"])
        resized(weak).save(files["weak-resized"])
        resized(full).save(files["full-resized"])
        for kind, f in files.items():
            jobs.append((c, meta["profile"], kind, f))
    if worst != 0:
        print(f"weak-resized: the k = 1 composite is {worst} levels off the bench's marked.png", file=sys.stderr)
        sys.exit(1)
    with concurrent.futures.ThreadPoolExecutor(a.jobs) as ex:
        results = list(ex.map(lambda j: (j, inspect(a.cli, j[3], j[1])), jobs))
    if a.clean:
        todo = [(j, r) for j, r in results if r.get("verdict") == "verified" and j[2] != "weak-row"]
        with concurrent.futures.ThreadPoolExecutor(a.jobs) as ex:
            done = list(ex.map(lambda jr: cleaned(a.cli, jr[0][3], jr[0][1], jr[0][3] + ".cleaned.png"), todo))
        for (j, r), c in zip(todo, done):
            r.update(c)
    with open(os.path.join(a.out, "weak.jsonl"), "w") as f:
        for (c, prof, kind, path), r in results:
            f.write(json.dumps({"case": c["case_dir"], "profile": prof, "kind": kind, **r}) + "\n")
    print(f"k = 1 composites equal the bench's marked.png (largest difference {worst}); k = {a.k} drawn.\n")
    print("| kind | cases | proved | k* of the proofs | refused (why) | not found | under D530's bands | clean says a mark is left |")
    print("|---|---|---|---|---|---|---|---|")
    for kind in ("weak-row", "weak-resized", "full-resized"):
        rs = [r for (c, p, k, f), r in results if k == kind]
        proved = [r for r in rs if r.get("verdict") == "verified"]
        ks = collections.Counter(round(r["k"], 2) for r in proved)
        why = collections.Counter(r.get("why") for r in rs if r.get("verdict") == "refused")

        def band(r):
            w, k = r["width"], r["k"]
            return (0.96 <= k <= 1.06) if w < 64 else (1.02 <= k <= 1.06)
        kept = sum(band(r) for r in proved if r.get("placed") == "searched")
        print(f"| {kind} | {len(rs)} | {len(proved)} | {', '.join(f'{k}: {n}' for k, n in sorted(ks.items())) or '—'} | "
              f"{sum(why.values())} ({', '.join(f'{w} {n}' for w, n in why.most_common()) or '—'}) | "
              f"{sum(not r.get('found') for r in rs)} | {kept} of the searched proofs kept | "
              f"{lefts(proved) if a.clean and kind != 'weak-row' else '—'} |")


if __name__ == "__main__":
    main()
