#!/usr/bin/env python3
"""Grok-on-X's opacity maps, one per picture size X produced (E12-6 for Grok, G3).

What it is for, and who asked
-----------------------------
The coordinator's task `wipemark-task-grok-x-mark-2026-10-11` (G2, G3; the
owner's word of 2026-10-11: "Grok puts its visible mark only in X … we just
need to take its mark off") asks for the mark X's Grok stamps to be shipped
as two catalogue profiles, `grok-x-dark` (logo 0) and `grok-x-light` (logo
255), with **one map per size X produced**, measured where a flat capture of
that size exists: the light variant from a black background (`α = I/255`),
the dark from a white one (`α = 1 − I/255`), and the two compared where both
exist. The first look (`first-look.py`, 2026-10-11) measured the dark map on
one flat grey picture at 1792×1008; the captures check (`captures-check.py`,
the same day) found that Grok draws its mark *at* the picture's size — at
784 px wide the 149-px map drawn down is 4–6 % off the mark on its soft
edge — so a size with a flat capture gets the capture's own map. This script
makes those maps; it is the one place they are made, and the README beside
them (`crates/wipemark-pixels/marks/grok/README.md`) names what it printed.
The owner's white captures of the same afternoon showed the two colours are
not one shape (step 4), and step 5 is what that made necessary.

What it does, step by step — `fit`
----------------------------------
1. **The reference** (`--reference`, the first look's flat grey 1792×1008,
   the dark variant): the rectangle and the ring's quadratic `B` exactly as
   `first-look.py look` finds them (`reference_rect`, `ring_fit`), and its
   dark map `α = 1 − I_Y/B_Y` over that rectangle — luma, BT.601 — cleaned
   as `first-look.py` cleans it (negatives to 0; a sample under a floor —
   three times the ring's own `α` spread, at least 0.01 — farther than two
   pixels from one over it, to 0). Its row: that rectangle's margins.
2. **Every flat capture** (`--flat`, black or white, any size): the mark
   found with the reference's depth as `first-look.py`'s `locate` finds it
   (every whole-pixel width, then 1/16 px), the whole-pixel rectangle around
   it grown by `--border` (2) pixels, the ring's quadratic `B` around that,
   the corner told black or white by `scripts/corpus/ring.py`'s
   `looks_like`, and the variant by `first-look.py`'s `variant_fit`. Black
   under a light mark: `α = (I_Y − B_Y)/(255 − B_Y)`; white under a dark
   one: `α = (B_Y − I_Y)/B_Y`; anything else is refused by name. Captures of
   one size and one variant are averaged sample for sample (two pictures of
   one size put the mark on the same pixel; the rectangles must agree, or
   the size is refused), then cleaned as in 1.
3. **A size with no flat capture** (`--drawn WxH=PICTURE`): the mark found in
   that picture as in 2, and the reference map drawn there by the area
   integral at its sub-pixel place and width (`first-look.py`'s `draw`),
   on the picture's own pixel grid, grown by the border — a map *drawn*,
   not measured, and said so.
4. **One shape, one opacity?** Per size with both a white mark (off black)
   and a black one (off white, or the grey reference): NCC, the slope of the
   white map on the black one through the origin, the RMS after it, the two
   core medians — the check only a black and a white capture of one size can
   make. A pair is one shape at NCC ≥ `SAME_NCC` (0.98) and a slope within
   `SAME_SLOPE` (0.03) of 1.
5. **The colour curve**, when a pair is not one shape: the white α as a
   function of the black one, sample for sample over every pair — the mean
   white α of the black samples within 1.25 steps of each point of a grid
   0, 0.01 … 0.45, interpolated where a point has under five samples,
   anchored at 0, made monotone. With two pairs or more, each is held out in
   turn: the curve made without it, on it, against the black map as it is.
6. **A map per size and colour**: measured where a capture of that colour
   exists; else the black map drawn — the reference map at the place the
   size's white-mark capture put the mark, by the area integral, on its grid
   — and the white map derived — the black one through the curve (or, with
   no curve, the black one as it is, said).
7. **Out**: per size and colour one `.wma`, 16-bit, `grok-x-<dark|light>-<W>x<H>.wma`,
   into `--out` (`crates/wipemark-pixels/marks/grok/`, this script's own
   maps there removed first); `fit.json` — every map's id, asset, sha256,
   size, row margin, how it was made and from which file, the pairs, the
   curve, the held-out check — and `fit.md`, the tables, into `--report` (an
   ignored `reports/grok-fit-maps-<date>/`).

What it does — `selftest`
-------------------------
A wordmark-like shape (a slashed ring and five bars, `α` 0.36) drawn by
8 × 8 supersampling — the black mark as it is, the white one through a
curve of its own (`light_of`, thinner on the soft edge): black on flat grey
at 1792×1008 for the reference, white on black and black on white at
784×1168 and 1168×784, white on black at 1152×1712, black on a ramp at
1600×1200 for a drawn row; JPEGs from Pillow at quality 95, 4:2:0. Checks:
a map per size and colour, each read as `wipemark-pixels` reads a `.wma`
(magic, size, depth 16) and pinned by its sha256; each holding the whole
drawn mark at its row's margin, at NCC over 0.97 against the truth of its
colour, within RMS 0.02 when measured and 0.035 when drawn or derived; the
two black captures of 784×1168 averaged; the two pairs not one shape; the
curve made without a pair predicting it to under half the black map's RMS;
1792×1008's white map derived and 1152×1712's black map drawn; a grey
capture refused as not flat black or white.

How to run it
-------------
    python3 scripts/grok/fit-maps.py fit \\
        --reference fixtures/image/grok-x/HUVfLiSW0AEtmX_.jpeg \\
        --flat /workspace/grok-images/x/*.jpeg \\
        --drawn 1728x1152=fixtures/image/grok-x/HUS9M6oX0AERoMb.jpeg \\
                1600x1200=fixtures/image/grok-x/HUVe9s_XkAAwxck.jpeg \\
        --out crates/wipemark-pixels/marks/grok --report reports/grok-fit-maps-2026-10-11
    python3 scripts/grok/fit-maps.py selftest

from the repository's root; run it from a directory that is not the
pictures' folder. The captures are read in place and never copied.

What it needs
-------------
Python 3.11+ with numpy and Pillow 12.3.0 (the user site or a venv; nothing
in the repository depends on them). It imports `first-look.py` (this folder)
and `scripts/corpus/ring.py`. No Rust, no network.

What its output means
---------------------
The `.wma` files are the product's data once a catalogue row names them;
`fit.json` is what such a row needs (every map's sha256, size and margin);
`fit.md` says how each was made and the step-4 numbers. A map from one
picture is one picture's measurement — fitted, never claimed exact (D245).
Exit status: 0 done, 1 the self-test failed, 2 a usage error, a capture that
is not flat black or white, or one that cannot be read.
"""

import argparse
import datetime
import glob
import hashlib
import importlib.util
import json
import math
import os
import struct
import sys
import tempfile

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
sys.dont_write_bytecode = True  # the two scripts below are imported; no __pycache__ beside them


def _module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


fl = _module("first_look", os.path.join(HERE, "first-look.py"))
ring = _module("ring", os.path.join(REPO, "scripts", "corpus", "ring.py"))

LUMA = fl.LUMA
BORDER = 2  # zero samples kept around a measured mark  [tunable]
SAME_NCC = 0.98  # a black and a white capture of one size are one shape at or over this NCC …  [tunable]
SAME_SLOPE = 0.03  # … and one opacity within this of a slope of 1  [tunable]
CURVE_STEP = 0.01  # the colour curve's grid, in α  [tunable]
CURVE_TOP = 0.45  # … up to this α  [tunable]


def fail(msg):
    print(f"fit-maps: {msg}", file=sys.stderr)
    sys.exit(2)


def load(path):
    try:
        b = open(path, "rb").read()
        with Image.open(path) as im:
            img = np.asarray(im.convert("RGB")).astype(np.float64)
    except OSError as e:
        fail(f"{path}: {e}")
    return b, img


def name_of(path):
    return os.path.splitext(os.path.basename(path))[0]


def floor_of(alpha_ring):
    return max(3.0 * float(np.std(alpha_ring)), 0.01)


def wma(alpha):
    """WMA1, depth 16 — `crates/wipemark-pixels/src/alpha.rs`'s format."""
    h, w = alpha.shape
    q = np.clip(np.round(alpha * 65535.0), 0, 65535).astype("<u2")
    return b"WMA1" + struct.pack("<HHB", w, h, 16) + q.tobytes()


# ── 1. the reference ─────────────────────────────────────────────────────────


def reference(path):
    b, img = load(path)
    H, W, _ = img.shape
    rect, _ = fl.reference_rect(img, fl.CORNER, fl.PAD)
    rx, ry, rw, rh = (int(v) for v in rect)
    fit = fl.ring_fit(img, (rx, ry, rw, rh))
    ox, oy = fit["origin"]
    sl = (slice(ry - oy, ry - oy + rh), slice(rx - ox, rx - ox + rw))
    I, B = fit["crop"][sl], fit["B"][sl]
    depth = (B - I) @ LUMA
    alpha = 1.0 - (I @ LUMA) / np.maximum(B @ LUMA, 1.0)
    noise = 1.0 - (fit["crop"] @ LUMA) / np.maximum(fit["B"] @ LUMA, 1.0)
    amap = fl.cleaned_map(alpha, floor_of(noise[fit["ring"]]))
    return {"path": path, "name": name_of(path), "sha256": hashlib.sha256(b).hexdigest(), "size": [W, H],
            "rect": [rx, ry, rw, rh], "template": depth, "alpha": amap, "variant": "dark", "how": "measured",
            "background": "grey", "img": img}


# ── 2. flat captures ─────────────────────────────────────────────────────────


def measure_flat(path, ref, border):
    b, img = load(path)
    H, W, _ = img.shape
    loc = fl.locate(img, ref["template"], fl.LOCATE, ref["size"], fl.SCALES)
    x0 = int(math.floor(loc["x"])) - border
    y0 = int(math.floor(loc["y"])) - border
    x1 = int(math.ceil(loc["x"] + loc["w"])) + border
    y1 = int(math.ceil(loc["y"] + loc["h"])) + border
    x0, y0, x1, y1 = max(0, x0), max(0, y0), min(W, x1), min(H, y1)
    rect = (x0, y0, x1 - x0, y1 - y0)
    m = ring.measure_ring(img, rect, ring.RING)
    if m is None:
        fail(f"{path}: no ring around the mark at {rect}")
    seen = ring.looks_like(m)
    fit = fl.ring_fit(img, rect)
    fox, foy = fit["origin"]
    sl = (slice(y0 - foy, y1 - foy), slice(x0 - fox, x1 - fox))
    I, B = fit["crop"][sl] @ LUMA, fit["B"][sl] @ LUMA
    dox, doy, dt = fl.draw(ref["template"], loc["x"], loc["y"], loc["w"])
    core = np.zeros(fit["crop"].shape[:2], bool)
    sub = (slice(doy - foy, doy - foy + dt.shape[0]), slice(dox - fox, dox - fox + dt.shape[1]))
    core[sub] = dt > fl.CORE * dt.max()
    variant = fl.variant_fit(fit["crop"][core], fit["B"][core])["variant"]
    if seen == "black" and variant == "light":
        alpha = (I - B) / np.maximum(255.0 - B, 1.0)
        noise = ((fit["crop"] - fit["B"]) @ LUMA) / np.maximum(255.0 - fit["B"] @ LUMA, 1.0)
    elif seen == "white" and variant == "dark":
        alpha = (B - I) / np.maximum(B, 1.0)
        noise = ((fit["B"] - fit["crop"]) @ LUMA) / np.maximum(fit["B"] @ LUMA, 1.0)
    else:
        fail(f"{path}: the corner looks {seen} under a {variant} mark — a map is read off flat black under a light "
             f"mark or flat white under a dark one")
    return {"path": path, "name": name_of(path), "sha256": hashlib.sha256(b).hexdigest(), "size": [W, H],
            "rect": [x0, y0, x1 - x0, y1 - y0], "alpha": alpha, "floor": floor_of(noise[fit["ring"]]),
            "variant": variant, "background": seen, "loc": loc,
            "ring_sd": [round(v, 3) for v in m["sd"]], "ring_mean": [round(v, 3) for v in m["mean"]]}


def group(captures):
    """Captures of one size and variant averaged; rectangles must agree."""
    out = {}
    for c in captures:
        out.setdefault((tuple(c["size"]), c["variant"]), []).append(c)
    maps = []
    for (size, variant), cs in sorted(out.items()):
        rects = {tuple(c["rect"]) for c in cs}
        if len(rects) != 1:
            fail(f"{size[0]}x{size[1]} {variant}: the captures put the mark in different rectangles {sorted(rects)}")
        alpha = np.mean([c["alpha"] for c in cs], axis=0)
        floor = max(c["floor"] for c in cs)
        maps.append({"size": list(size), "variant": variant, "rect": cs[0]["rect"], "how": "measured",
                     "background": cs[0]["background"], "alpha": fl.cleaned_map(alpha, floor),
                     "from": [{"name": c["name"], "sha256": c["sha256"]} for c in cs],
                     "agree": pair_rms([c["alpha"] for c in cs])})
    return maps


def pair_rms(alphas):
    if len(alphas) < 2:
        return None
    a = np.array(alphas)
    return float(np.sqrt(((a - a.mean(0)) ** 2).mean()))


# ── 3. drawn rows ────────────────────────────────────────────────────────────


def drawn_map(path, ref, border):
    b, img = load(path)
    H, W, _ = img.shape
    loc = fl.locate(img, ref["template"], fl.LOCATE, ref["size"], fl.SCALES)
    ox, oy, m = fl.draw(ref["alpha"], loc["x"], loc["y"], loc["w"])
    full = np.zeros((m.shape[0] + 2 * border, m.shape[1] + 2 * border))
    full[border:border + m.shape[0], border:border + m.shape[1]] = m
    return {"size": [W, H], "variant": "both", "rect": [ox - border, oy - border, full.shape[1], full.shape[0]],
            "how": f"drawn: the reference map at ({loc['x']:.3f}, {loc['y']:.3f}) width {loc['w']:.3f}, "
                   f"where NCC {loc['ncc']:.3f} put the mark", "background": "picture",
            "alpha": np.clip(full, 0, 1), "from": [{"name": name_of(path), "sha256": hashlib.sha256(b).hexdigest()}],
            "agree": None, "loc": loc}


# ── 4. shapes ────────────────────────────────────────────────────────────────


def compare(a, d):
    """a against d (same shape): NCC, slope through the origin, RMS after it, core medians."""
    h, w = min(a.shape[0], d.shape[0]), min(a.shape[1], d.shape[1])
    a, d = a[:h, :w], d[:h, :w]
    sup = (a > 0.02) | (d > 0.02)
    core = d > fl.CORE * d.max()
    slope = float((a[sup] * d[sup]).sum() / max((d[sup] ** 2).sum(), 1e-12))
    return {"ncc": fl.ncc(a, d), "slope": slope, "rms": float(np.sqrt(((a - slope * d)[sup] ** 2).mean())),
            "core_a": float(np.median(a[core])), "core_b": float(np.median(d[core]))}


def against_reference(m, ref):
    """The reference map drawn at this map's mark (its locate) on its grid."""
    loc = m.get("loc")
    if loc is None:
        return None
    x0, y0, _, _ = m["rect"]
    ox, oy, d = fl.draw(ref["alpha"], loc["x"], loc["y"], loc["w"])
    full = np.zeros_like(m["alpha"])
    ys, xs = oy - y0, ox - x0
    hh = min(d.shape[0], full.shape[0] - ys)
    ww = min(d.shape[1], full.shape[1] - xs)
    full[ys:ys + hh, xs:xs + ww] = d[:hh, :ww]
    return compare(m["alpha"], full)


def core_median(alpha):
    return float(np.median(alpha[alpha > fl.CORE * alpha.max()]))


# ── 4b. the colour curve ─────────────────────────────────────────────────────


def on_grid(alpha, rect, target):
    """`alpha` laid at `rect` (x, y, w, h) onto the zeros of `target`'s grid."""
    tx, ty, tw, th = target
    x, y, w, h = rect
    out = np.zeros((th, tw))
    ox, oy = x - tx, y - ty
    x0, y0 = max(0, ox), max(0, oy)
    x1, y1 = min(tw, ox + w), min(th, oy + h)
    if x1 > x0 and y1 > y0:
        out[y0:y1, x0:x1] = alpha[y0 - oy:y1 - oy, x0 - ox:x1 - ox]
    return out


def drawn_at(ref, loc, target):
    """The reference map drawn by the area integral at `loc`, on `target`'s grid."""
    ox, oy, d = fl.draw(ref["alpha"], loc["x"], loc["y"], loc["w"])
    return np.clip(on_grid(d, (ox, oy, d.shape[1], d.shape[0]), target), 0, 1)


def colour_curve(pairs):
    """The white mark's α as a function of the black one's, sample for sample,
    over every black-and-white pair of one size: the mean light α of the dark
    samples within half a step of each point of a grid 0, 0.01 … 0.45,
    interpolated where a point has under five, anchored at 0, made monotone."""
    d = np.concatenate([p["dark"][(p["light"] > 0.003) | (p["dark"] > 0.003)] for p in pairs])
    l = np.concatenate([p["light"][(p["light"] > 0.003) | (p["dark"] > 0.003)] for p in pairs])
    grid = np.round(np.arange(0, CURVE_TOP + 1e-9, CURVE_STEP), 4)
    val = np.full(grid.shape, np.nan)
    for i, g in enumerate(grid):
        sel = np.abs(d - g) <= CURVE_STEP * 1.25
        if sel.sum() >= 5:
            val[i] = l[sel].mean()
    ok = ~np.isnan(val)
    val = np.interp(grid, grid[ok], val[ok]) if ok.any() else grid.copy()
    val[0] = 0.0
    return grid, np.maximum.accumulate(val)


def through(curve, dark):
    grid, val = curve
    return np.interp(dark, grid, val)


def fit(args):
    ref = reference(args.reference)
    flats = []
    for p in args.flat:
        for q in (sorted(glob.glob(p)) if any(ch in p for ch in "*?[") else [p]):
            if not os.path.basename(q).startswith("."):
                flats.append(q)
    seen, captures = set(), []
    for p in flats:
        c = measure_flat(p, ref, args.border)
        if c["sha256"] in seen:
            continue
        seen.add(c["sha256"])
        captures.append(c)
    sizes = {}
    for m in group(captures):
        m["loc"] = next(c["loc"] for c in captures
                        if tuple(c["size"]) == tuple(m["size"]) and c["variant"] == m["variant"])
        sizes.setdefault(tuple(m["size"]), {})[m["variant"]] = m
    # the reference: the black mark at its size, unless a white capture measured it there
    rs = tuple(ref["size"])
    if "dark" not in sizes.get(rs, {}):
        here = sizes.get(rs, {}).get("light")
        rect = here["rect"] if here else ref["rect"]
        sizes.setdefault(rs, {})["dark"] = {
            "size": list(rs), "variant": "dark", "rect": rect, "how": "measured on a flat grey, `1 − I/B`",
            "background": "grey", "alpha": on_grid(ref["alpha"], ref["rect"], rect), "reference": True,
            "from": [{"name": ref["name"], "sha256": ref["sha256"]}], "agree": None, "loc": None}
    for item in args.drawn or []:
        size, _, path = item.partition("=")
        w, _, h = size.partition("x")
        d = drawn_map(path, ref, args.border)
        if d["size"] != [int(w), int(h)]:
            fail(f"--drawn {item}: the picture is {d['size'][0]}x{d['size'][1]}")
        if tuple(d["size"]) in sizes:
            fail(f"--drawn {item}: a measured map of that size exists")
        sizes[tuple(d["size"])] = {"dark": dict(d, variant="dark")}
    # 4. the pairs, and the colour curve from them
    pairs = []
    for size, e in sorted(sizes.items()):
        if "light" in e and "dark" in e:
            l, d = e["light"], e["dark"]
            if l["rect"] != d["rect"]:
                fail(f"{size[0]}x{size[1]}: the black and the white captures put the mark in different rectangles")
            c = compare(l["alpha"], d["alpha"])
            pairs.append({"size": list(size), **c, "one_shape": c["ncc"] >= SAME_NCC and abs(c["slope"] - 1) <= SAME_SLOPE,
                          "light": l["alpha"], "dark": d["alpha"]})
    curve = colour_curve(pairs) if pairs and not all(p["one_shape"] for p in pairs) else None
    held = []
    if curve is not None and len(pairs) > 1:
        for i, p in enumerate(pairs):
            c = colour_curve(pairs[:i] + pairs[i + 1:])
            m = (p["light"] > 0.005) | (p["dark"] > 0.005)
            held.append({"size": p["size"], "rms_curve": float(np.sqrt(((through(c, p["dark"]) - p["light"])[m] ** 2).mean())),
                         "rms_same": float(np.sqrt(((p["dark"] - p["light"])[m] ** 2).mean()))})
    # 5. every size: a map per colour
    for size, e in sizes.items():
        if "dark" not in e:
            l = e["light"]
            e["dark"] = {"size": list(size), "variant": "dark", "rect": l["rect"], "background": "—",
                         "how": f"drawn: the {rs[0]}x{rs[1]} map at ({l['loc']['x']:.3f}, {l['loc']['y']:.3f}) width "
                                f"{l['loc']['w']:.3f}, where the black capture put the mark",
                         "alpha": drawn_at(ref, l["loc"], l["rect"]), "from": l["from"], "agree": None, "loc": l["loc"]}
        if "light" not in e:
            d = e["dark"]
            if curve is not None:
                alpha, how = through(curve, d["alpha"]), "derived: the black mark's map through the colour curve"
            elif pairs:
                alpha, how = d["alpha"], "the black mark's map: the pairs of one size are one shape"
            else:
                alpha, how = d["alpha"], "the black mark's map: no black-and-white pair of one size to tell them apart"
            e["light"] = dict(d, variant="light", alpha=alpha, how=how)
    os.makedirs(args.out, exist_ok=True)
    os.makedirs(args.report, exist_ok=True)
    for f in glob.glob(os.path.join(args.out, "grok-x-*.wma")):
        os.remove(f)  # this script's own maps, made again
    rows = []
    for size, e in sorted(sizes.items(), key=lambda kv: (-kv[0][0] * kv[0][1], kv[0])):
        for side in ("dark", "light"):
            m = e[side]
            W, H = size
            x, y, w, h = m["rect"]
            stem = f"grok-x-{side}-{W}x{H}"
            data = wma(m["alpha"])
            with open(os.path.join(args.out, f"{stem}.wma"), "wb") as f:
                f.write(data)
            rows.append({"id": stem, "asset": f"{stem}.wma", "sha256": hashlib.sha256(data).hexdigest(), "for": side,
                         "size": [w, h], "picture": [W, H], "margin": [W - (x + w), H - (y + h)], "how": m["how"],
                         "background": m["background"], "from": m["from"], "agree": m["agree"],
                         "core": core_median(m["alpha"]), "max": float(m["alpha"].max()),
                         "against_reference": None if m.get("reference") or not m.get("loc") or m["how"].startswith("drawn")
                         else against_reference(m, ref)})
    out = {"date": args.date, "reference": {"name": ref["name"], "sha256": ref["sha256"], "rect": ref["rect"]},
           "maps": rows, "pairs": [{k: v for k, v in p.items() if k not in ("light", "dark")} for p in pairs],
           "curve": None if curve is None else {"dark": curve[0].tolist(), "light": [round(v, 5) for v in curve[1]]},
           "held_out": held}
    with open(os.path.join(args.report, "fit.json"), "w") as f:
        json.dump(out, f, indent=1)
    write_md(os.path.join(args.report, "fit.md"), out)
    print(f"fit-maps: wrote {len(rows)} maps to {args.out}; {args.report}/fit.json, fit.md")


def write_md(path, s):
    L = [f"# Grok-on-X's maps — {s['date']}\n", "Generated by `scripts/grok/fit-maps.py fit`.\n",
         "| map | picture | size | margin r, b | how | from | captures agree (RMS) | core | max | "
         "the reference drawn here: NCC, slope, RMS, core a/b |", "|---|---|---|---|---|---|---|---|---|---|"]
    for r in s["maps"]:
        ar = r["against_reference"]
        art = f"{ar['ncc']:.3f}, {ar['slope']:.3f}, {ar['rms']:.4f}, {ar['core_a']:.3f}/{ar['core_b']:.3f}" if ar else "—"
        agree = "—" if r["agree"] is None else f"{r['agree']:.5f}"
        L.append(f"| `{r['id']}` | {r['picture'][0]}×{r['picture'][1]} | {r['size'][0]}×{r['size'][1]} | "
                 f"{r['margin'][0]}, {r['margin'][1]} | {r['how']} | "
                 f"{', '.join('`' + f['name'] + '`' for f in r['from'])} | {agree} | "
                 f"{r['core']:.3f} | {r['max']:.3f} | {art} |")
    L.append("\nA black and a white capture of one size, the white mark's map against the black one's:\n")
    if s["pairs"]:
        L.append("| size | NCC | slope | RMS | core light, dark | one shape |\n|---|---|---|---|---|---|")
        for p in s["pairs"]:
            L.append(f"| {p['size'][0]}×{p['size'][1]} | {p['ncc']:.4f} | {p['slope']:.4f} | {p['rms']:.4f} | "
                     f"{p['core_a']:.3f}, {p['core_b']:.3f} | {'yes' if p['one_shape'] else '**no**'} |")
    else:
        L.append("None: no size has both.")
    if s["curve"]:
        c = s["curve"]
        L.append("\nThe colour curve, the white mark's α against the black one's (every fifth point):\n")
        L.append("| black α | " + " | ".join(f"{d:.2f}" for d in c["dark"][::5]) + " |")
        L.append("|---|" + "---|" * len(c["dark"][::5]))
        L.append("| white α | " + " | ".join(f"{v:.3f}" for v in c["light"][::5]) + " |")
    if s["held_out"]:
        L.append("\nThe curve made without each pair, on that pair (RMS of the white map from the black one):\n")
        L.append("| size held out | through the curve | the black map as it is |\n|---|---|---|")
        for h in s["held_out"]:
            L.append(f"| {h['size'][0]}×{h['size'][1]} | {h['rms_curve']:.4f} | {h['rms_same']:.4f} |")
    with open(path, "w") as f:
        f.write("\n".join(L) + "\n")


# ── selftest ─────────────────────────────────────────────────────────────────


def shape_inside(u, v):
    r = np.hypot(u - 26.5, v - 26.5)
    ring_ = (r <= 26.5) & (r >= 20.5)
    ax, ay, bx, by = 8.0, 46.0, 44.0, 7.0
    t = np.clip(((u - ax) * (bx - ax) + (v - ay) * (by - ay)) / ((bx - ax) ** 2 + (by - ay) ** 2), 0, 1)
    slash = np.hypot(u - (ax + t * (bx - ax)), v - (ay + t * (by - ay))) <= 3.0
    bars = np.zeros_like(u, bool)
    for x0, y0, x1, y1 in ((56, 10, 62, 44), (70, 10, 76, 44), (76, 10, 92, 16), (98, 18, 104, 44),
                           (112, 10, 118, 44), (118, 25, 145, 31)):
        bars |= (u >= x0) & (u < x1) & (v >= y0) & (v < y1)
    return ring_ | slash | bars


def light_of(dark, peak=0.36):
    """The selftest's own colour curve: the white mark thinner on its soft edge."""
    return peak * (dark / peak) ** 1.6


def render(W, H, ss=8, peak=0.36):
    s = W / 1792.0
    w, h = 149.0 * s, 57.0 * s
    x, y = W - 9.0 - w, H - 11.0 - h
    x0, y0 = int(math.floor(x)) - 1, int(math.floor(y)) - 1
    nx, ny = int(math.ceil(w)) + 3, int(math.ceil(h)) + 3
    off = (np.arange(ss) + 0.5) / ss
    px = (x0 + np.arange(nx)[:, None] + off[None, :]).reshape(-1)
    py = (y0 + np.arange(ny)[:, None] + off[None, :]).reshape(-1)
    U, V = np.meshgrid((px - x) / s - 2.0, (py - y) / s - 2.0)
    cov = shape_inside(U, V).astype(np.float64).reshape(ny, ss, nx, ss).mean(axis=(1, 3))
    a = np.zeros((H, W))
    a[y0:y0 + ny, x0:x0 + nx] = cov * peak
    return a, (x, y, w, h)


def synth(path, W, H, bg, variant, ramp=0.0, scene=False):
    a, box = render(W, H)
    B = np.full((H, W, 3), float(bg))
    if ramp:
        B += (np.arange(W)[None, :, None] / W) * ramp
    if scene:  # another generation: something in the upper left, the corner the same
        B[20:60, 20:60] = 255.0 - bg
    if variant == "light":
        a = light_of(a)
    A = a[..., None]
    I = A * 255.0 + (1 - A) * B if variant == "light" else (1 - A) * B
    Image.fromarray(np.clip(np.round(I), 0, 255).astype(np.uint8)).save(path, "JPEG", quality=95, subsampling=2)
    return a, box


def selftest():
    failures = []

    def ok(cond, what):
        print(f"{'ok  ' if cond else 'FAIL'} {what}")
        if not cond:
            failures.append(what)

    with tempfile.TemporaryDirectory() as tmp:
        P = lambda n: os.path.join(tmp, n)  # noqa: E731
        synth(P("ref.jpg"), 1792, 1008, 201, "dark")
        synth(P("black-port-1.jpg"), 784, 1168, 0, "light")
        synth(P("black-port-2.jpg"), 784, 1168, 0, "light", scene=True)
        synth(P("white-port-1.jpg"), 784, 1168, 255, "dark")
        synth(P("black-land-1.jpg"), 1168, 784, 0, "light")
        synth(P("white-land-1.jpg"), 1168, 784, 255, "dark")
        synth(P("black-tall-1.jpg"), 1152, 1712, 0, "light")
        synth(P("ramp.jpg"), 1600, 1200, 170, "dark", ramp=40.0)
        synth(P("grey.jpg"), 784, 1168, 128, "light")
        out, rep = P("maps"), P("report")
        args = argparse.Namespace(reference=P("ref.jpg"), flat=[P("black-*.jpg"), P("white-*.jpg")],
                                  drawn=[f"1600x1200={P('ramp.jpg')}"], out=out, report=rep, border=BORDER,
                                  date="selftest")
        fit(args)
        s = json.load(open(os.path.join(rep, "fit.json")))
        by = {(tuple(r["picture"]), r["for"]): r for r in s["maps"]}
        sizes = {k[0] for k in by}
        ok(sizes == {(1792, 1008), (784, 1168), (1168, 784), (1152, 1712), (1600, 1200)}
           and len(by) == 2 * len(sizes), f"a map per size and colour: {sorted(by)}")

        def read(r):
            data = open(os.path.join(out, r["asset"]), "rb").read()
            w, h, depth = struct.unpack("<HHB", data[4:9])
            fine = (data[:4] == b"WMA1" and [w, h] == r["size"] and depth == 16 and len(data) == 9 + 2 * w * h
                    and hashlib.sha256(data).hexdigest() == r["sha256"])
            return fine, np.frombuffer(data[9:], "<u2").reshape(h, w) / 65535.0

        for (size, side), r in sorted(by.items()):
            fine, m = read(r)
            ok(fine, f"{size} {side}: the .wma reads and is pinned")
            a, _ = render(*size)
            truth = light_of(a) if side == "light" else a
            W, H = size
            X0, Y0 = W - r["margin"][0] - r["size"][0], H - r["margin"][1] - r["size"][1]
            t = truth[Y0:Y0 + r["size"][1], X0:X0 + r["size"][0]]
            inside = float(t.sum() / truth.sum())
            c = fl.ncc(m, t)
            rms = float(np.sqrt(((m - t)[(m > 0.005) | (t > 0.005)] ** 2).mean()))
            # measured within a level or two; drawn and derived within the curve's and the drawing's error
            bound = 0.02 if r["how"].startswith("measured") else 0.035
            ok(inside > 0.999 and c > 0.97 and rms < bound,
               f"{size} {side}: {r['how'][:40]}… — holds {inside:.4f} of the mark, NCC {c:.4f}, RMS {rms:.4f}")
        ok(by[((784, 1168), "light")]["agree"] is not None and len(by[((784, 1168), "light")]["from"]) == 2,
           "the two black captures of 784x1168 are averaged")
        ok(len(s["pairs"]) == 2 and not any(p["one_shape"] for p in s["pairs"]),
           f"the two black-and-white pairs are not one shape: {[(p['size'], round(p['slope'], 3)) for p in s['pairs']]}")
        ok(s["curve"] is not None and all(h["rms_curve"] < h["rms_same"] / 2 for h in s["held_out"]),
           f"the curve, made without a pair, predicts it: {s['held_out']}")
        ok(by[((1792, 1008), "light")]["how"].startswith("derived"), "1792x1008's white map is derived")
        ok(by[((1152, 1712), "dark")]["how"].startswith("drawn"), "1152x1712's black map is drawn")
        try:
            measure_flat(P("grey.jpg"), reference(P("ref.jpg")), BORDER)
            ok(False, "a grey capture is refused")
        except SystemExit as e:
            ok(e.code == 2, "a grey capture is refused")
    print("selftest: " + ("passed" if not failures else f"{len(failures)} failed"))
    return 0 if not failures else 1


def main():
    today = datetime.date.today().isoformat()
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    a = sub.add_parser("fit", help="the maps, one per size")
    a.add_argument("--reference", default=os.path.join(REPO, "fixtures", "image", "grok-x", "HUVfLiSW0AEtmX_.jpeg"))
    a.add_argument("--flat", nargs="*", default=[])
    a.add_argument("--drawn", nargs="*", default=[], metavar="WxH=PICTURE")
    a.add_argument("--out", default=os.path.join("crates", "wipemark-pixels", "marks", "grok"))
    a.add_argument("--report")
    a.add_argument("--border", type=int, default=BORDER)
    a.add_argument("--date", default=today)
    sub.add_parser("selftest", help="synthetic captures")
    args = ap.parse_args()
    if args.cmd == "selftest":
        sys.exit(selftest())
    if args.report is None:
        args.report = os.path.join("reports", f"grok-fit-maps-{args.date}")
    fit(args)


if __name__ == "__main__":
    main()
