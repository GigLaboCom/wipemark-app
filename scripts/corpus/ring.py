#!/usr/bin/env python3
"""Is the background under the mark what the file's group says it is?

What it is for
--------------
Step E12-R2 of the E12-R series (`docs/plan/E12-R2-corpora.md` §4.1),
filed 2026-10-08 by the coordinator from the owner's spec
`wipemark-recon-spec-2026-10-08` (`02-data-collection.md` §1; the owner's
S2), the container part written 2026-10-09. The `gemini-midtone` set is
Gemini outputs whose corner under the mark is grey, black, white,
saturated, a gradient or a texture — on one background (saturated green,
D240–D243) the opacity `α` and the logo's colour `L` cannot be told apart,
and neither can the blend models. A file only helps if its corner really is
the background its group names. This is the check before a file enters the
set: the host runs it on every file the owner delivers, and
`scripts/corpus/manifest.py add` runs it again and records its numbers in
the row (D304).

What it does
------------
1. Reads the catalogue (`manifests/marks.v1.json`) for the profile and takes
   **the row the catalogue would use for the picture's size**: the first
   placement row, in the catalogue's order, whose `when` matches the size
   and whose rectangle fits inside the picture (`--row N` takes another).
   The product tries every row (`wipemark_pixels::propose::rows`); the
   catalogue lists the specific row before the general one (V1: 96 at 64
   when both sides are ≥ 1025, then 48 at 32), so the first that answers is
   the mark's. The rectangle is restated as `scripts/analytics/bias.py` and
   `propose.rs::row_rect` state it: a `corner` row at its map's own size
   (`margin` in from the corner; the map's size is read from the `.wma`
   header under `crates/wipemark-pixels/marks/`), a `rect` row at its own
   rectangle. A row that **resamples** its map (V2's half-size outputs,
   `1376×768` …) is handled, not refused: this check needs only the
   square, never `α`, and the row's own `rect` is that square; the output
   says `resample: true`. A size no row answers for is a refusal.
2. Takes a ring `--ring` pixels wide (8, §4.1) **outside** the mark's
   square, clipped at the picture's edge — the mark is wholly inside its
   square, so the ring is the background and nothing else.
3. Per channel (R, G, B, 8-bit levels as stored; Pillow decodes the PNG):
   the ring's **mean**, its **standard deviation** (the *spread*) and the
   RMS **residual** of a quadratic `k0 + k1·u + k2·v + k3·u² + k4·uv + k5·v²`
   fitted to the ring by least squares — `calibrate.rs`'s `background` basis,
   `u = (x + 0.5 − cx)/s`, `v = (y + 0.5 − cy)/s`, `s` the square's side.
4. The verdict, by the group given (`--group`):
   * `gray-*`, `black`, `white`, `sat-*` **pass when the spread is under
     `SPREAD_LEVELS` (8 levels, `[tunable]`) in every channel**;
   * `gradient` passes when the quadratic describes the ring — the residual
     under `SMOOTH_RESIDUAL` (2 levels, `[tunable]`) in every channel;
   * `texture` has no rule: its numbers are recorded, and it passes.
5. What the ring **looks like**, whatever the group said: a flat ring by its
   level (`black` ≤ 24 everywhere, `white` ≥ 231 everywhere, `gray-25`,
   `gray-50`, `gray-75` for a neutral ring — channels within 12 of each
   other — at the nearest of 64, 123, 190; `sat-red`, `sat-green`,
   `sat-blue` for a ring whose largest channel is ≥ 128 and another ≤ 24;
   `flat` otherwise); a ring past the spread rule is `gradient` when the
   quadratic describes it, `texture` when its residual is spread evenly over
   its four sides (the largest side's RMS within `EVEN_SIDES` × the
   smallest's, `[tunable]`), and `drop` otherwise — an edge or an object on
   one side is neither. A file that fails is told where it could go
   (`gradient`, `texture`, or `drop`); the manifest says which (§4.1). A
   flat group whose level is not its own gets a **note** — never a failure:
   §4.1's rule is the spread, the level is the owner's label to correct.

How to run it
-------------
    python3 scripts/corpus/ring.py PICTURE.png [MORE.png …] --profile gemini-sparkle-v1 --group gray-50
    python3 scripts/corpus/ring.py PICTURE.png --profile gemini-sparkle-v2 --group black --json
    python3 scripts/corpus/ring.py PICTURE.png --profile gemini-sparkle-v1 --group white --row 1 --ring 8
    python3 scripts/corpus/ring.py selftest

`manifest.py` imports `measure` from this file (same folder).

What it needs
-------------
Python 3.10+, numpy and Pillow (a venv is fine; nothing in the repository
depends on them — checked with numpy 2.5.3, Pillow 12.3.0). The pictures are
the owner's and never enter git; `selftest` synthesises its own into a
temporary folder.

What its output means
---------------------
Per file, one line: the verdict (`pass`/`FAIL`), the row and its rectangle,
per channel `mean ± spread (residual)`, what the ring looks like and, for a
failure, where the file could move. `--json` prints a list of objects, one
per file, with the same facts (`mean`, `sd`, `residual` per channel as
`[R, G, B]`, `spread` and `residual_max`, `rect` as `[x, y, w, h]`, `row`,
`resample`, `pass`, `looks_like`, `move_to`, `notes`). Exit codes are the
repository's: **0** every file passed, **1** a file failed its group's rule
(a finding), **2** usage or a refusal (no row answers for the size, no
ring, a picture that is not 8-bit).
"""

import argparse
import json
import os
import struct
import sys
import tempfile

import numpy as np

REPO = os.environ.get("WIPEMARK_REPO") or os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CATALOGUE = os.path.join(REPO, "manifests", "marks.v1.json")
MARKS = os.path.join(REPO, "crates", "wipemark-pixels", "marks")

RING = 8  # §4.1: a ring 8 px wide outside the mark's square
SPREAD_LEVELS = 8.0  # §4.1: a background group passes under this spread, every channel  [tunable]
SMOOTH_RESIDUAL = 2.0  # a ring a quadratic describes to this RMS is smooth (a gradient)  [tunable]
EVEN_SIDES = 3.0  # a texture's residual is even over the ring's four sides, within this ratio  [tunable]
MIN_RING = 30  # calibrate.rs: a ring of fewer pixels is no ring
MIN_SIDE = 30  # a side of the ring with fewer pixels is not compared

DARK, LIGHT = 24.0, 231.0  # "≈ black", "≈ white", "≈ 0 in a channel"  [tunable]
NEUTRAL = 12.0  # a grey's channels lie within this of each other  [tunable]
SATURATED = 128.0  # a saturated ring's largest channel is at least this  [tunable]
GREYS = {"gray-25": 64.0, "gray-50": 123.0, "gray-75": 190.0}  # §4.1: ≈ 64, 118–128, ≈ 190

FLAT_GROUPS = ("gray-25", "gray-50", "gray-75", "black", "white", "sat-red", "sat-blue")
GROUPS = FLAT_GROUPS + ("gradient", "texture")
CHANNELS = "RGB"


class Refusal(Exception):
    pass


# ── the catalogue: which square the mark is in ───────────────────────────────


def catalogue():
    with open(CATALOGUE) as f:
        return json.load(f)


def profile_of(pid):
    for p in catalogue()["profiles"]:
        if p["id"] == pid:
            return p
    raise Refusal(f"no profile {pid!r} in {CATALOGUE}")


def wma_size(name):
    """(width, height) from a `.wma` header — `WMA1`, then u16 width, u16 height."""
    for root, _, files in os.walk(MARKS):
        if name in files:
            with open(os.path.join(root, name), "rb") as f:
                head = f.read(9)
            if head[:4] != b"WMA1":
                raise Refusal(f"{name}: not a WMA1 file")
            return struct.unpack_from("<HH", head, 4)
    raise Refusal(f"{name}: not under {MARKS}")


def when_matches(when, w, h):
    return ((when.get("width") is None or when["width"] == w) and (when.get("height") is None or when["height"] == h)
            and (when.get("min_width") is None or w >= when["min_width"])
            and (when.get("min_height") is None or h >= when["min_height"])
            and (when.get("max_width") is None or w <= when["max_width"])
            and (when.get("max_height") is None or h <= when["max_height"]))


def row_rect(profile, i, W, H):
    """Row `i`'s square in a W×H picture, as `propose.rs::row_rect` puts it:
    (x, y, w, h, resample), or None when the row does not answer."""
    r = profile["placements"][i]
    if not when_matches(r.get("when") or {}, W, H):
        return None
    if "rect" in r:
        x, y, w, h = r["rect"]
        if x + w > W or y + h > H:
            return None
        return (x, y, w, h, bool(r.get("resample")))
    asset = next(a for a in profile["alpha"] if a["id"] == r["alpha"])
    mw, mh = wma_size(asset["asset"])
    mx, my = r["margin"]
    if W < mx + mw or H < my + mh:
        return None
    corner = r["corner"]
    x = W - mx - mw if corner.endswith("right") else mx
    y = H - my - mh if corner.startswith("bottom") else my
    return (x, y, mw, mh, False)


def the_row(profile, W, H, row=None):
    """(row index, (x, y, w, h, resample)): `row` when given, else the first that answers."""
    rows = range(len(profile["placements"])) if row is None else [row]
    for i in rows:
        if not 0 <= i < len(profile["placements"]):
            raise Refusal(f"{profile['id']} has no row {i}")
        rect = row_rect(profile, i, W, H)
        if rect is not None:
            return i, rect
    which = "no row of" if row is None else f"row {row} of"
    raise Refusal(f"{which} {profile['id']} answers for {W}×{H}")


# ── the ring ─────────────────────────────────────────────────────────────────


def load(path):
    from PIL import Image
    with Image.open(path) as im:
        if im.mode not in ("RGB", "RGBA", "P", "L", "LA", "PA"):
            raise Refusal(f"{path}: mode {im.mode} — this check reads 8-bit pictures")
        return np.asarray(im.convert("RGB"), dtype=np.float64)


def basis(u, v):
    return np.stack([np.ones_like(u), u, v, u * u, u * v, v * v], axis=-1)


def ring_of(img, rect, ring=RING):
    """The ring's pixel coordinates (xs, ys) and a side label per pixel
    (0 top, 1 bottom, 2 left, 3 right), clipped at the picture's edge."""
    H, W, _ = img.shape
    x, y, w, h = rect
    x0, y0 = max(x - ring, 0), max(y - ring, 0)
    x1, y1 = min(x + w + ring, W), min(y + h + ring, H)
    yy, xx = np.mgrid[y0:y1, x0:x1]
    inside = (xx >= x) & (xx < x + w) & (yy >= y) & (yy < y + h)
    rx, ry = xx[~inside], yy[~inside]
    side = np.where(ry < y, 0, np.where(ry >= y + h, 1, np.where(rx < x, 2, 3)))
    return rx, ry, side


def measure_ring(img, rect, ring=RING):
    """Per channel: mean, sd, quadratic residual (RMS); and per side the
    residual RMS (largest over channels). None when the ring is too small."""
    x, y, w, h = rect
    rx, ry, side = ring_of(img, rect, ring)
    if rx.size < MIN_RING:
        return None
    cx, cy = x + w / 2.0, y + h / 2.0
    s = float(max(w, h, 1))
    A = basis((rx + 0.5 - cx) / s, (ry + 0.5 - cy) / s)
    mean, sd, resid, err = [], [], [], []
    for c in range(3):
        vals = img[ry, rx, c]
        k, *_ = np.linalg.lstsq(A, vals, rcond=None)
        e = vals - A @ k
        mean.append(float(vals.mean()))
        sd.append(float(vals.std()))
        resid.append(float(np.sqrt(np.mean(e * e))))
        err.append(e)
    sides = []
    for sd_id in range(4):
        sel = side == sd_id
        if sel.sum() >= MIN_SIDE:
            sides.append(max(float(np.sqrt(np.mean(e[sel] ** 2))) for e in err))
    return {"n": int(rx.size), "mean": mean, "sd": sd, "residual": resid, "sides": sides}


def looks_like(m):
    """What the ring is, by its numbers alone."""
    mean, spread, resid = m["mean"], max(m["sd"]), max(m["residual"])
    if spread >= SPREAD_LEVELS:
        if resid <= SMOOTH_RESIDUAL:
            return "gradient"
        sides = m["sides"]
        if len(sides) >= 2 and max(sides) <= EVEN_SIDES * max(min(sides), 1e-9):
            return "texture"
        return "drop"
    if all(v <= DARK for v in mean):
        return "black"
    if all(v >= LIGHT for v in mean):
        return "white"
    if max(mean) - min(mean) <= NEUTRAL:
        level = sum(mean) / 3.0
        return min(GREYS, key=lambda g: abs(GREYS[g] - level))
    top = int(np.argmax(mean))
    if mean[top] >= SATURATED and min(mean) <= DARK:
        return "sat-" + ("red", "green", "blue")[top]
    return "flat"


def verdict(group, m):
    """(pass, move_to, notes) for a file the owner filed under `group`."""
    notes = []
    seen = looks_like(m)
    if group in FLAT_GROUPS:
        ok = all(v < SPREAD_LEVELS for v in m["sd"])
        if not ok:
            # Past the spread rule: a gradient, a texture, or nothing this set can use.
            return False, seen if seen in ("gradient", "texture", "drop") else "drop", notes
        if seen != group:
            notes.append(f"the ring looks like {seen}, not {group}: check the label")
        return True, None, notes
    if group == "gradient":
        ok = all(v <= SMOOTH_RESIDUAL for v in m["residual"])
        if not ok:
            return False, "texture" if seen == "texture" else "drop", notes
        if max(m["sd"]) < SPREAD_LEVELS:
            notes.append(f"the ring is flat (spread {max(m['sd']):.2f}): it looks like {seen}")
        return True, None, notes
    if group == "texture":
        if seen != "texture":
            notes.append(f"the ring looks like {seen}")
        return True, None, notes
    raise Refusal(f"group {group!r}: one of {', '.join(GROUPS)}")


def measure(path, profile_id, group, ring=RING, row=None):
    """The check of §4.1 on one file, as a dict (what `--json` prints and
    `manifest.py` records)."""
    if group not in GROUPS:
        raise Refusal(f"group {group!r}: one of {', '.join(GROUPS)}")
    img = load(path)
    H, W, _ = img.shape
    profile = profile_of(profile_id)
    i, (x, y, w, h, resample) = the_row(profile, W, H, row)
    m = measure_ring(img, (x, y, w, h), ring)
    if m is None:
        raise Refusal(f"{path}: no ring around {(x, y, w, h)} inside {W}×{H}")
    ok, move_to, notes = verdict(group, m)
    return {
        "file": path, "size": [W, H], "profile": profile_id, "row": i, "rect": [x, y, w, h],
        "resample": resample, "ring": ring, "ring_pixels": m["n"], "group": group,
        "mean": [round(v, 3) for v in m["mean"]], "sd": [round(v, 3) for v in m["sd"]],
        "residual": [round(v, 3) for v in m["residual"]],
        "spread": round(max(m["sd"]), 3), "residual_max": round(max(m["residual"]), 3),
        "rule": {"spread_levels": SPREAD_LEVELS, "smooth_residual": SMOOTH_RESIDUAL, "even_sides": EVEN_SIDES},
        "pass": ok, "looks_like": looks_like(m), "move_to": move_to, "notes": notes,
    }


def line(r):
    ch = "  ".join(f"{CHANNELS[c]} {r['mean'][c]:6.1f} ± {r['sd'][c]:5.2f} ({r['residual'][c]:4.2f})" for c in range(3))
    head = f"{'pass' if r['pass'] else 'FAIL'}  {r['file']}  {r['group']}"
    rect = f"row {r['row']} {tuple(r['rect'])}{' resampled' if r['resample'] else ''}"
    tail = f"looks like {r['looks_like']}" + (f"; move to {r['move_to']}" if r["move_to"] else "")
    out = f"{head}\n      {rect}, ring {r['ring']} px ({r['ring_pixels']}): {ch}; {tail}"
    for n in r["notes"]:
        out += f"\n      note: {n}"
    return out


# ── selftest ─────────────────────────────────────────────────────────────────


def write_png(path, arr):
    from PIL import Image
    Image.fromarray(np.clip(np.rint(arr), 0, 255).astype(np.uint8), "RGB").save(path)


def busy(size, seed):
    """A picture that is anything but flat: strong noise over a diagonal ramp."""
    rng = np.random.default_rng(seed)
    yy, xx = np.mgrid[0:size, 0:size]
    base = (xx + yy) * (200.0 / (2 * size)) + 20
    return np.repeat(base[:, :, None], 3, axis=2) + rng.normal(0, 40, (size, size, 3))


def with_zone(img, rect, fill):
    """`img` with the zone around `rect` (a margin of 1.5× its side, §4.1)
    replaced by `fill(yy, xx)` — the rest of the picture stays busy."""
    x, y, w, h = rect
    m = int(1.5 * w)
    H, W, _ = img.shape
    y0, y1, x0, x1 = max(y - m, 0), min(y + h + m, H), max(x - m, 0), min(x + w + m, W)
    yy, xx = np.mgrid[y0:y1, x0:x1]
    out = img.copy()
    out[y0:y1, x0:x1, :] = fill(yy, xx)
    return out


def selftest():
    failures = []

    def check(ok, what):
        print(("ok   " if ok else "FAIL ") + what)
        if not ok:
            failures.append(what)

    v1, v2 = profile_of("gemini-sparkle-v1"), profile_of("gemini-sparkle-v2")
    check(the_row(v1, 2048, 2048) == (0, (1888, 1888, 96, 96, False)), "V1 at 2048 takes row 0, 96 at 64")
    check(the_row(v1, 1024, 1024) == (1, (944, 944, 48, 48, False)), "V1 at 1024 takes row 1, 48 at 32")
    check(the_row(v2, 1376, 768) == (9, (1232, 624, 48, 48, True)), "V2 at 1376×768 takes its resampled row, rect as given")
    check(the_row(v2, 1024, 1024) == (1, (917, 917, 36, 36, False)), "V2 at 1024 takes row 1, 36 at 71")
    try:
        the_row(v2, 500, 500)
        check(False, "a size no row answers for is refused")
    except Refusal:
        check(True, "a size no row answers for is refused")

    rng = np.random.default_rng(7)
    with tempfile.TemporaryDirectory(prefix="ring-selftest-") as tmp:
        size = 2048
        _, (x, y, w, h, _) = the_row(v1, size, size)

        def run(name, img, group, profile="gemini-sparkle-v1"):
            p = os.path.join(tmp, name)
            write_png(p, img)
            return measure(p, profile, group)

        # a_flat_corner_passes_and_a_gradient_does_not — the picture is busy
        # everywhere but the zone, so the rule has to be about the ring.
        flat = with_zone(busy(size, 1), (x, y, w, h),
                         lambda yy, xx: 123.0 + rng.normal(0, 1.0, yy.shape + (3,)))
        ramp = with_zone(busy(size, 2), (x, y, w, h),
                         lambda yy, xx: np.repeat((60.0 + 0.4 * (xx - x + 150))[:, :, None], 3, axis=2)
                         + rng.normal(0, 0.5, yy.shape + (3,)))
        a, b = run("flat.png", flat, "gray-50"), run("ramp.png", ramp, "gray-50")
        check(a["pass"] and a["spread"] < 2 and a["looks_like"] == "gray-50",
              f"a_flat_corner_passes_and_a_gradient_does_not: flat grey passes (spread {a['spread']})")
        check(not b["pass"] and b["move_to"] == "gradient",
              f"a_flat_corner_passes_and_a_gradient_does_not: a gradient fails and moves to gradient "
              f"(spread {b['spread']}, residual {b['residual_max']}, move to {b['move_to']})")
        g = run("ramp-as-gradient.png", ramp, "gradient")
        check(g["pass"], "the same gradient filed as gradient passes")

        tex = with_zone(busy(size, 3), (x, y, w, h), lambda yy, xx: 120.0 + rng.normal(0, 18, yy.shape + (3,)))
        t = run("texture.png", tex, "gray-50")
        check(not t["pass"] and t["move_to"] == "texture", f"a fine texture fails a grey group and moves to texture ({t['move_to']})")

        # An object on one side: the ring's right side is busy, the other three flat.
        edge = with_zone(busy(size, 4), (x, y, w, h),
                         lambda yy, xx: 128.0 + np.where((xx >= x + w)[:, :, None], rng.normal(0, 60, yy.shape + (3,)), 0.0))
        e = run("edge.png", edge, "white")
        check(not e["pass"] and e["move_to"] == "drop", f"something on one side of the ring is dropped ({e['move_to']}, sides uneven)")

        dark = with_zone(busy(size, 5), (x, y, w, h), lambda yy, xx: 190.0 + rng.normal(0, 1.0, yy.shape + (3,)))
        d = run("mislabelled.png", dark, "gray-50")
        check(d["pass"] and d["looks_like"] == "gray-75" and d["notes"],
              "a flat ring at another level passes with a note naming the level it looks like")

        red = with_zone(busy(size, 6), (x, y, w, h),
                        lambda yy, xx: np.stack([np.full(yy.shape, 220.0), np.full(yy.shape, 4.0), np.full(yy.shape, 30.0)], axis=-1))
        r = run("red.png", red, "sat-red")
        check(r["pass"] and r["looks_like"] == "sat-red", "a saturated red ring passes as sat-red")

        small = np.full((768, 1376, 3), 250.0)
        s = run("v2-half.png", small, "white", profile="gemini-sparkle-v2")
        check(s["pass"] and s["resample"] and s["rect"] == [1232, 624, 48, 48], "a resampled V2 row is measured on its own rect")

        out = json.loads(json.dumps([a, b]))
        check(set(out[0]) >= {"mean", "sd", "residual", "spread", "rect", "row", "pass", "move_to", "looks_like"},
              "the JSON carries every field manifest.py records")
    print(f"selftest: {'all passed' if not failures else str(len(failures)) + ' failed'}")
    return 1 if failures else 0


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    if argv[:1] == ["selftest"]:
        return selftest()
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("pictures", nargs="+")
    ap.add_argument("--profile", required=True, help="a catalogue profile id, e.g. gemini-sparkle-v1")
    ap.add_argument("--group", required=True, choices=GROUPS)
    ap.add_argument("--ring", type=int, default=RING)
    ap.add_argument("--row", type=int, help="this placement row, not the first that answers")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args(argv)
    results, refused = [], 0
    for p in args.pictures:
        try:
            results.append(measure(p, args.profile, args.group, args.ring, args.row))
        except Refusal as e:
            print(f"refused: {e}", file=sys.stderr)
            refused += 1
    if args.json:
        json.dump(results, sys.stdout, indent=1)
        print()
    else:
        for r in results:
            print(line(r))
    if refused:
        return 2
    return 0 if all(r["pass"] for r in results) else 1


if __name__ == "__main__":
    sys.exit(main())
