#!/usr/bin/env python3
"""Where is the mark in each file? — sub-pixel alignment of a pair's crops (stage 1).

What it is for
--------------
Step E12-R11 of the E12-R series (`docs/plan/E12-R11-grok-map.md` §4.1,
the "position varies" row), filed 2026-10-08 by the coordinator from the
owner's spec `wipemark-recon-spec-2026-10-08` (`08-grok-evaluation.md`
§2); the container part, written 2026-10-09 before any capture exists.
When `invariance.py` reads a pair as "position varies" — the mean soft, the
unexplained scatter carried by a per-file shift — the mark is not at one
integer offset. This script measures each file's offset to a fraction of a
pixel, writes the crops moved back onto one position, and hands them to
`invariance.py` again: stable after it means a map exists and the rows
become a search (R11 §4.1); unstable means as before.

What it does
------------
1. Reads the same list `invariance.py` reads (CSV/TSV, TOML or the stage-0
   JSON manifest — see its header, "Input") and crops every pair the same
   way, at one integer offset (`invariance.pairs_of`).
2. Per file, the mark's **departure** from its own background,
   `d_i = Y_i − Ŷ_i` (luma; `Ŷ_i` the ring's quadratic, `invariance.
   quadratic`, over a ring that starts `--guard + --reach` pixels outside
   the rectangle so that a mark moved by up to `--reach` never falls in
   it). Working on the departure takes the background's level away; the
   NCC takes its contrast with the mark away.
3. The **window** is the mark's rectangle grown by `--guard` (2). The
   template is the mean of the files' departures over the window, each
   normalised (zero mean, unit norm) — or, with `--reference first`, the
   first file's.
4. Per file, the offset `δ = (dx, dy)` that maximises the NCC between the
   template and the file's departure **sampled at `p + δ`**:
   * every integer shift within ±`--reach` (4) px;
   * a parabola through the peak and its two neighbours, per axis
     (clamped to ±½ px);
   * a grid of ⅛ px over ±⅜ px around that, then a grid of 1/32 px over
     ±3/32 px around the best — the offset is quantised to 1/32 px, so
     the method's own error is at most 1/64 px; noise and the kernel add
     to it (the self-test holds the whole to 0.1 px).
   A shifted sample is **bicubic** (Keys, `a = −½`, Catmull–Rom),
   separable, with the edges clamped.
5. With the mean template, the offsets are centred on their mean (the
   template's own position is the pair's mean position), the template is
   rebuilt from the aligned departures, and steps 4–5 run `--iterations`
   (3) times in all: the first template is the blurred mean, each next one
   is sharper.
6. Writes each crop moved by its offset, `aligned(p) = crop(p + δ)` with
   the same kernel, as `<out>/<pair>/<file>.npy` (float32, h × w × 3,
   8-bit levels, not rounded) and a `crops.csv` that `invariance.py run`
   reads as its list (`crop`, `aligned`, the picture's `width`/`height`,
   the rectangle inside the crop). Writes `offsets.csv` (one line per
   file: source, size, file, dx, dy, the NCC at the integer peak and at
   the end) and `offsets.md`: per pair the scatter — mean, standard
   deviation, the 95th percentile and the maximum of |dx| and |dy| — and
   a **mechanical reading**: both standard deviations under `--steady`
   (0.25 px) `[tunable]` → "position steady", otherwise "position
   varies". The conclusion is the stage-1 report's.

Sign: `δ` is where the file's mark sits relative to the template (`+dx`
to the right, `+dy` down); the crop sampled at `p + δ` puts it back.

How to run it
-------------
    python3 scripts/grok/align.py run LIST --out DIR [--root DIR] [--with-held-out] \
        [--reference mean|first] [--reach 4] [--iterations 3] [--guard 2] [--ring 6] \
        [--crop-factor 1.5] [--steady 0.25]
    python3 scripts/grok/invariance.py run DIR/crops.csv --out DIR2
    python3 scripts/grok/align.py selftest

What it needs
-------------
Python 3.11+, numpy and Pillow (a venv is fine — written against numpy
2.5.3, Pillow 12.3.0), and `invariance.py` beside it (imported). No scipy.
`selftest` synthesises its pictures (`invariance.synth_pair`).

What its output means
---------------------
`offsets.md`'s reading per pair, and `crops.csv` for the second pass of
`invariance.py`: "one map" there, on these crops, is R11's "a map exists
after alignment"; anything else is "no map". A scatter that is a constant
shift is not a moving mark — it is a rectangle in the list that is off,
and the stage-1 report says which. Exit codes: 0 done (selftest: all
passed), 1 a selftest check failed, 2 usage or a refusal.
"""

import argparse
import csv
import os
import sys
import tempfile

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import invariance as inv  # noqa: E402

KEYS_A = -0.5  # Catmull–Rom
REACH = 4  # px  [tunable]
ITERATIONS = 3  # [tunable]
STEADY = 0.25  # px  [tunable]
TOLERANCE = 0.1  # px: the self-test's bound on a recovered offset


# ── the kernel ───────────────────────────────────────────────────────────────


def keys(x):
    x = np.abs(x)
    a = KEYS_A
    near = (a + 2) * x ** 3 - (a + 3) * x ** 2 + 1
    far = a * x ** 3 - 5 * a * x ** 2 + 8 * a * x - 4 * a
    return np.where(x <= 1, near, np.where(x < 2, far, 0.0))


def sample_axis(a, pos, axis):
    """`a` sampled at the real positions `pos` along `axis`: bicubic, clamped."""
    i = np.floor(pos).astype(int)
    t = pos - i
    n = a.shape[axis]
    shape = [1] * a.ndim
    shape[axis] = len(pos)
    out = 0.0
    for off in (-1, 0, 1, 2):
        w = keys(t - off).reshape(shape)
        out = out + np.take(a, np.clip(i + off, 0, n - 1), axis=axis) * w
    return out


def shifted(a, dx, dy, xs=None, ys=None):
    """`a` sampled at `(y + dy, x + dx)` for `y` in `ys`, `x` in `xs` (all of it by default)."""
    h, w = a.shape[:2]
    xs = np.arange(w) if xs is None else xs
    ys = np.arange(h) if ys is None else ys
    return sample_axis(sample_axis(a, ys + float(dy), 0), xs + float(dx), 1)


# ── the offsets ──────────────────────────────────────────────────────────────


def departure(crop, mark, guard, reach, ring):
    d = inv.distance(crop.shape[:2], mark)
    ring_mask = (d > guard + reach) & (d <= guard + reach + ring)
    return (crop - inv.quadratic(crop, mark, ring_mask)) @ inv.LUMA


def window(shape, mark, guard):
    h, w = shape
    x, y, mw, mh = mark
    return np.arange(max(x - guard, 0), min(x + mw + guard, w)), np.arange(max(y - guard, 0), min(y + mh + guard, h))


def unit(v):
    v = v - v.mean()
    n = float(np.sqrt((v * v).sum()))
    return v / n if n > 0 else v


def ncc(d, dx, dy, xs, ys, template):
    return float((unit(shifted(d, dx, dy, xs, ys)) * template).sum())


def best_offset(d, template, xs, ys, reach):
    """(dx, dy, ncc at the integer peak, ncc at the end), header step 4."""
    grid = {(dx, dy): ncc(d, dx, dy, xs, ys, template)
            for dy in range(-reach, reach + 1) for dx in range(-reach, reach + 1)}
    (ix, iy), peak = max(grid.items(), key=lambda kv: kv[1])

    def vertex(fm, f0, fp):
        den = fm - 2.0 * f0 + fp
        return 0.0 if den >= 0 else float(np.clip(0.5 * (fm - fp) / den, -0.5, 0.5))

    fx = vertex(grid.get((ix - 1, iy), peak), peak, grid.get((ix + 1, iy), peak))
    fy = vertex(grid.get((ix, iy - 1), peak), peak, grid.get((ix, iy + 1), peak))
    cx, cy = ix + fx, iy + fy
    best = ncc(d, cx, cy, xs, ys, template)
    for step in (1.0 / 8.0, 1.0 / 32.0):
        ox, oy = cx, cy
        for ky in range(-3, 4):
            for kx in range(-3, 4):
                v = ncc(d, ox + kx * step, oy + ky * step, xs, ys, template)
                if v > best:
                    best, cx, cy = v, ox + kx * step, oy + ky * step
    return cx, cy, peak, best


def align_pair(pair, guard=inv.GUARD, reach=REACH, ring=inv.RING, iterations=ITERATIONS, reference="mean"):
    """Per file (dx, dy, ncc at the integer peak, ncc at the end)."""
    ds = [departure(c, pair.mark, guard, reach, ring) for c in pair.crops]
    xs, ys = window(ds[0].shape, pair.mark, guard)
    offs = np.zeros((len(ds), 2))
    found = []
    for _ in range(iterations):
        if reference == "first":
            template = unit(ds[0][np.ix_(ys, xs)])
        else:
            template = unit(np.mean([unit(shifted(d, o[0], o[1], xs, ys)) for d, o in zip(ds, offs)], axis=0))
        found = [best_offset(d, template, xs, ys, reach) for d in ds]
        offs = np.array([(f[0], f[1]) for f in found])
        if reference != "first":
            offs = offs - offs.mean(axis=0)
    return [(float(o[0]), float(o[1]), f[2], f[3]) for o, f in zip(offs, found)]


def scatter(offsets, steady):
    dx = np.array([o[0] for o in offsets])
    dy = np.array([o[1] for o in offsets])
    row = {}
    for name, v in (("dx", dx), ("dy", dy)):
        row[name] = (float(v.mean()), float(v.std(ddof=1)) if len(v) > 1 else 0.0,
                     float(np.percentile(np.abs(v), 95)), float(np.abs(v).max()))
    ok = row["dx"][1] <= steady and row["dy"][1] <= steady
    row["reading"] = "position steady" if ok else "position varies"
    return row


# ── writing ──────────────────────────────────────────────────────────────────


def write(out, aligned):
    """`aligned`: [(pair, offsets)]. Writes the crops, crops.csv, offsets.csv, offsets.md."""
    os.makedirs(out, exist_ok=True)
    md = ["# Stage 1 — offsets (`scripts/grok/align.py`)", "",
          "| source | size | files | dx mean | dx sd | dx p95 | dx max | dy mean | dy sd | dy p95 | dy max | reading |",
          "|---|---|---|---|---|---|---|---|---|---|---|---|"]
    with open(os.path.join(out, "crops.csv"), "w", newline="") as fc, \
            open(os.path.join(out, "offsets.csv"), "w", newline="") as fo:
        wc, wo = csv.writer(fc), csv.writer(fo)
        wc.writerow(["path", "source", "crop", "aligned", "width", "height",
                     "mark_x", "mark_y", "mark_w", "mark_h", "held_out", "dx", "dy"])
        wo.writerow(["source", "size", "file", "dx", "dy", "ncc_integer", "ncc_final"])
        for pair, offsets, steady in aligned:
            folder = os.path.join(out, pair.key)
            os.makedirs(folder, exist_ok=True)
            x, y, w, h = pair.mark
            for name, crop, (dx, dy, n0, n1) in zip(pair.names, pair.crops, offsets):
                stem = os.path.splitext(name)[0]
                rel = os.path.join(pair.key, stem + ".npy")
                np.save(os.path.join(out, rel), shifted(crop, dx, dy).astype(np.float32))
                wc.writerow([rel, pair.source, 1, 1, pair.W, pair.H, x, y, w, h, 0, f"{dx:.4f}", f"{dy:.4f}"])
                wo.writerow([pair.source, f"{pair.W}x{pair.H}", name, f"{dx:.4f}", f"{dy:.4f}", f"{n0:.5f}", f"{n1:.5f}"])
            s = scatter(offsets, steady)
            cells = " | ".join(f"{v:+.3f}" if i == 0 else f"{v:.3f}" for k in ("dx", "dy") for i, v in enumerate(s[k]))
            md.append(f"| {pair.source} | {pair.W}x{pair.H} | {len(offsets)} | {cells} | {s['reading']} |")
    md += ["", "Offsets in pixels, centred on the pair's mean (or relative to the first file with "
           "`--reference first`). The reading is arithmetic; the conclusion is the stage-1 report's.", ""]
    text = "\n".join(md)
    with open(os.path.join(out, "offsets.md"), "w") as f:
        f.write(text)
    return text


def run(rows, args, out):
    pairs = inv.pairs_of(rows, args)
    aligned = []
    for p in pairs:
        if len(p.crops) < inv.MIN_FILES:
            print(f"skipped — {p.key}: {len(p.crops)} files (fewer than {inv.MIN_FILES})", file=sys.stderr)
            continue
        offsets = align_pair(p, args.guard, args.reach, args.ring, args.iterations, args.reference)
        aligned.append((p, offsets, args.steady))
    return write(out, aligned)


# ── selftest ─────────────────────────────────────────────────────────────────


def selftest():
    failures = []

    def check(ok, what):
        print(("ok   " if ok else "FAIL ") + what)
        if not ok:
            failures.append(what)

    w = np.array([keys(0.3 - k) for k in (-1, 0, 1, 2)])
    check(abs(w.sum() - 1.0) < 1e-12 and keys(0.0) == 1.0 and keys(1.0) == 0.0,
          "the kernel sums to 1 and interpolates")
    ramp = np.tile(np.arange(20.0), (5, 1))[..., None].repeat(3, axis=2)
    moved = shifted(ramp, 0.375, 0.0)
    check(np.allclose(moved[:, 2:-3, 0], ramp[:, 2:-3, 0] + 0.375),
          "a shift by 0.375 px moves a ramp by 0.375 (sampled at p + δ)")

    with tempfile.TemporaryDirectory() as tmp:
        opts = inv.defaults()
        listing, truth = inv.synth_pair(tmp, "jitter", 40, 4, jitter=1.5)
        pair = inv.pairs_of(inv.read_rows(listing), opts)[0]
        offsets = align_pair(pair)
        t = np.array(truth)
        t = t - t.mean(axis=0)
        got = np.array([(o[0], o[1]) for o in offsets])
        err = float(np.abs(got - t).max())
        check(err <= TOLERANCE, f"±1.5 px of jitter recovered within {TOLERANCE} px: worst {err:.4f} px")
        check(scatter(offsets, STEADY)["reading"] == "position varies", "… and its scatter reads as varying")

        out = os.path.join(tmp, "aligned")
        write(out, [(pair, offsets, STEADY)])
        st, _ = inv.one(os.path.join(out, "crops.csv"))
        check(st["reading"] == "one map" and st["aligned"],
              f"the aligned crops read as one map: {st['why']}")
        check(inv.stage1_line([(st, None)]) == "a map exists after alignment",
              f"… and the stage-1 line says so: {inv.stage1_line([(st, None)])}")

        fixed, _ = inv.synth_pair(tmp, "fixed", 20, 1)
        pair = inv.pairs_of(inv.read_rows(fixed), opts)[0]
        offsets = align_pair(pair)
        worst = max(max(abs(o[0]), abs(o[1])) for o in offsets)
        check(worst <= TOLERANCE and scatter(offsets, STEADY)["reading"] == "position steady",
              f"a fixed mark is found where it is: worst offset {worst:.4f} px, position steady")
    print(f"selftest: {'all passed' if not failures else str(len(failures)) + ' failed'}")
    return 1 if failures else 0


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("run")
    r.add_argument("list")
    r.add_argument("--out", required=True)
    inv.options(r)
    r.add_argument("--reference", choices=("mean", "first"), default="mean")
    r.add_argument("--reach", type=int, default=REACH)
    r.add_argument("--iterations", type=int, default=ITERATIONS)
    r.add_argument("--steady", type=float, default=STEADY)
    sub.add_parser("selftest")
    args = ap.parse_args(argv)
    try:
        if args.cmd == "selftest":
            return selftest()
        print(run(inv.read_rows(args.list, args.root), args, args.out))
        return 0
    except inv.Refusal as e:
        print(f"refused: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
