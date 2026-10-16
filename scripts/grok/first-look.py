#!/usr/bin/env python3
"""Grok's visible mark, a first look from a handful of real outputs (E12-R11, early).

What it is for, and who asked
-----------------------------
On 2026-10-11 the owner dropped the first real Grok outputs into one folder
(three JPEGs, then two more the same morning) and said work on Grok can
start from them. The capture guide (`docs/plan/E12-R-grok-capture-guide.md`)
asks for 50+ captures on plain backgrounds; these are what exists. The
coordinator's task `wipemark-task-recon-grok-first-look-2026-10-11` (F1, F2)
asked for one committed script that gets out of them what they honestly
give — where the mark is and how big, which variant each picture carries,
the opacity map of the dark variant from the one flat picture, the light
variant as well as a picture gives it — and turns that into the smallest
provisional profile the product can read, as a `--catalogue` file (D526:
never the shipped catalogue). F3 — the product on the same pictures through
that profile — is the `product` subcommand. It replaces the coordinator's
throwaway scripts of the same morning (`look.py`, `ncc.py`, `fit.py`,
`bgfit.py`), which are not committed.

No picture, crop, map or catalogue this script writes goes into git: all of
it is derived from the owner's pictures (D493). It writes under `--out`,
by default `reports/grok-first-look-<date>/`, which `.gitignore` covers
(`/reports/grok-*/`).

What it does, step by step — `look`
-----------------------------------
1. Per picture: its sha256, its size, and the JPEG facts its bytes give
   (baseline or progressive, the components' sampling — 4:2:0 and the
   rest — the first eight coefficients of quantisation table 0 in zigzag
   order, and the APPn segments present).
2. **The reference** (`--reference`, else the first picture): the picture
   whose corner under the mark is flat. The mark's rectangle there is
   found from luma: a first box — in the bottom-right box `--corner W H`
   (200 × 80), where luma departs from the box's median by more than 10
   levels (the largest group of such pixels, letters 8 pixels apart counted
   as one), or the approximate rectangle `--ref-box X Y W H` a person read
   off the picture (the sidecar's `margin` and `mark_size`) — then the
   ring's quadratic around it (below), then the tight box of the pixels departing
   from that quadratic by more than `max(3, 4 σ_ring)` levels, grown by
   `--pad` (2) pixels. That rectangle is the **template's**, and every
   width, height and margin below is of the template's rectangle drawn at
   that picture's scale — one definition for every picture.
3. **The background under the mark, `B`**, is the ring's quadratic as
   `calibrate` fits it and `scripts/grok/invariance.py` restates it: per
   channel, least squares of `k0 + k1·u + k2·v + k3·u² + k4·uv + k5·v²`
   over a ring 6 pixels wide (`RING`) starting 2 pixels (`GUARD`)
   outside the rectangle, `u = (x + 0.5 − cx)/s`, `v = (y + 0.5 − cy)/s`,
   `s` the longer side. `σ_ring` is the RMS of what the quadratic leaves on
   the ring, per channel: the background's own texture and the JPEG's
   noise together.
4. **The template** is the reference's depth `B_Y − I_Y` over the
   rectangle (luma `Y = 0.299 R + 0.587 G + 0.114 B`, 8-bit levels).
   **Where the mark is in every picture**: the template drawn by the area
   integral (each pixel the mean of the template samples its footprint
   covers, zero outside — the product's own way of drawing a map at a
   sub-pixel place and size) at every whole-pixel width from `--scales`
   (0.30–1.25) of its own, NCC (TM_CCOEFF_NORMED, its absolute value: a
   light mark correlates negatively with a dark one's depth) over the
   bottom-right box `LOCATE` (300 × 160 at the reference's width, scaled
   with the picture's, plus 40 pixels); then the place and width
   refined to 1/16 of a pixel by NCC alone (coordinate descent; NCC is good
   enough to find a place, it is not what the product proves with). The
   table: NCC, the sign (dark or light against the template), the
   rectangle, the margins right and bottom, and the width over W, H,
   √(W·H), the shorter and the longer side.
5. **The size and margin laws**: per law (width ∝ W, ∝ H, ∝ √(W·H),
   ∝ shorter, ∝ longer side, constant) the spread of width/that over the
   pictures (coefficient of variation, and the worst picture's distance in
   pixels from the law fitted through the reference); per margin law
   (constant in pixels, ∝ W, ∝ the width of the mark) the same.
6. **Which variant**, per picture: at the strokes' core (the drawn
   template over 0.85 of its maximum), per channel against `B`:
   dark — `I = (1 − a)·B` (a black logo); light — `I = a·255 + (1 − a)·B`
   (a white one); and the free one-colour blend `I − B = a·(L − B)` (L per
   picture, the same in every channel). Each fit's `a` and RMS residual per
   channel; the variant is the one with the smaller residual summed over
   the channels. The free fit's `L` means something only where `B` varies
   under the core (`B spread` in the table): over one flat background only
   `a·(L − B)` is known, and `a` and `L` trade against each other.
   With it, the background luma under the mark (the mean of `B_Y` over the
   rectangle) and `σ_ring`.
7. **The dark variant's opacity map**, from the reference: `α = 1 − I/B`
   on luma and per channel, over the rectangle. How noisy it is: the same
   formula over the ring and over the band just outside it (where α is 0),
   its standard deviation and 99th percentile; the three channels against
   each other on the mark (a black logo dims the three alike); its maximum
   and how many samples reach `--opaque` (0.95, `opaque_above`): holes.
8. **The light variant**, from every picture the fit calls light: `α =
   (I − B)/(255 − B)` on luma, at its own scale — rough: on a textured or
   dark background `B` is a guess under the strokes, and the template's
   rectangle at another scale is drawn rather than measured.
9. **One shape, two colours?** Per picture (light or dark), the reference
   α drawn at that picture's place and width, against the α measured
   there: NCC over the rectangle, and the slope of the measured α on the
   drawn one through the origin — the opacity of that picture's mark
   relative to the reference's (1 is the same mark; the product's gain
   check allows 0.94–1.06).
10. **The provisional profile (F2)**: one catalogue file
    `catalogue/marks.json` with two profiles, `grok-x-dark` (logo
    `[0, 0, 0]`) and `grok-x-light` (logo `[255, 255, 255]`), status
    provisional, observed from the date. Both carry the reference's
    luma α (negative values set to 0; samples under a floor — 3 σ of the
    ring's α, at least 0.01 — farther than two pixels from one over it set
    to 0), written as a 16-bit `.wma` and pinned by sha256: the light
    profile's map is the dark one's scaled by the measured relative
    opacity when `--light-scale` says so (default 1: the same α; `auto`:
    the median of step 9's relative opacity over the light pictures). Rows: the reference's size at
    its own map and margin (a corner row); every other picture size seen,
    a map of its own — the reference map drawn by the area integral at the
    place step 4 measured in the picture of that size (`--rows measured`,
    the default: two pictures of one size put the mark on the same pixel,
    so a size seen is its own best estimate) or at the place the size and
    margin laws predict (`--rows law`), its sub-pixel offset baked into a
    map one pixel larger, at a whole-pixel corner margin — because the
    catalogue takes a resampled `rect` only at the map's exact aspect,
    which a width ∝ W never gives in whole pixels. A
    search over the bottom-right `--search-box` with the reference map
    covers every other size. The `detect`/`verify` limits are the shipped
    profiles' (0.70; 0.06, 0.30, 0.01): no rule is loosened here.
    `calibrate` (`crates/wipemark-pixels/examples/calibrate.rs`) cannot
    make this map: it needs black, white and grey captures of one size,
    and there is one flat picture over one grey — so the map is written
    here, by the formula of step 7, and the report says so.

What it does — `product`
------------------------
F3: per picture, `<cli> inspect --json` and `<cli> clean -o <out> --json`
with a CLI built with the provisional profile compiled in
(`scripts/grok/cli-with-profile.sh`: the product reads only its compiled-in
catalogue, D526, so the CLI takes no `--catalogue`); the exit codes, every
finding (profile, row or search, NCC, verified or the refusal and its
numbers), every restoration (outline, texture, holes, clamps); and per
picture a PNG of the mark's corner before and after, ×4 nearest, with a
third panel: the result's departure from its ring's quadratic ×4 around
mid grey (a ghost of the mark is what shows there). Written as
`product.md` and `product.json` under `--out`.

Two measurements beside it, neither the product's road, both PNGs (a
lossless source: no planes, no DCT-POCS, no texture check):

* `--invert-dark`: every picture `look` calls dark, inverted (`255 − I`).
  A black-logo blend `(1 − α)·B` inverts exactly into a white-logo blend
  `α·255 + (1 − α)·(255 − B)` with the same α, so the light profile proves
  it there: what the proof and the restoration would make of the dark map,
  were the proposal not limited to marks lighter than the picture (its NCC
  is signed, `crates/wipemark-pixels/src/propose.rs`).
* `--control`: the mark's rectangle (grown by 12 pixels) overwritten with
  the picture's own texture `--control-shift` (1.5) mark widths to its
  left, and the row's map blended there exactly (inverted afterwards for a
  dark picture). Proved there and refused on the real mark: the map is not
  the mark Grok drew at that size. Refused there too: that texture under
  that map is past the product's checks, whatever the map.

How to run it
-------------
    python3 scripts/grok/first-look.py look PICTURE [PICTURE …] \
        [--reference PICTURE] [--out DIR] [--date YYYY-MM-DD] \
        [--corner 200 80] [--ref-box X Y W H] [--scales 0.30 1.25] [--pad 2] \
        [--opaque 0.95] [--light-scale 1.0|auto] [--search-box 400 200] [--rows measured|law]
    scripts/grok/cli-with-profile.sh DIR/catalogue/marks.json --out DIR/cli
    python3 scripts/grok/first-look.py product PICTURE [PICTURE …] \
        --cli DIR/cli/wipemark-cli [--out DIR] [--look DIR/look.json] [--invert-dark] \
        [--control [--control-shift 1.5]]

from the repository's root, or anywhere — every path is an argument; run
it from a directory that is not the pictures' folder. `look` writes
`look.md`, `look.json`, `maps/` and `catalogue/`; `product` reads
`look.json` for the rectangles (`--look`, default `<out>/look.json`).

What it needs
-------------
Python 3.11+ with numpy and Pillow 12.3.0 (the user site or a venv). Pillow
decodes the JPEGs with libjpeg's own upsampling, not zune-jpeg's planes, so
a map from here is a first look: R11 §4.2 asks for the planes when the
source is a subsampled JPEG. `product` needs the CLI that
`cli-with-profile.sh` builds.

What its output means
---------------------
`look.md` holds the tables of steps 1–9 and the profile's rows; `look.json`
the same as data. `maps/alpha-dark-Y.png` (and `-R`, `-G`, `-B`) is the
reference's α as 16-bit grey (α · 65535), `maps/alpha-light-<name>.png` the
rough light α of each light picture, `maps/*-x8.png` the same at ×8, 8-bit,
α 0.5 white, for the eye. `catalogue/marks.json` and its `.wma` maps are a
`--catalogue` file in the shipped schema (the developer tools read it;
`cli-with-profile.sh` compiles it into a scratch CLI) — provisional, from
one flat picture, and nothing of it is evidence that Grok is supported:
that is R12 §4.5's question, and xAI's policy (Q-R5) is among it.
Exit status: 0 done, 2 a usage error or a picture that cannot be read.
"""

import argparse
import datetime
import hashlib
import json
import math
import os
import struct
import subprocess
import sys

import numpy as np
from PIL import Image

LUMA = np.array([0.299, 0.587, 0.114])
CORNER = (200, 80)  # the bottom-right box the reference's mark is looked for in  [tunable]
LOCATE = (300, 160)  # the bottom-right box every picture's mark is looked for in, at the reference's scale  [tunable]
SCALES = (0.30, 1.25)  # the template's width, as a share of its own  [tunable]
PAD = 2  # pixels around the tight box  [tunable]
RING = 6  # CalibrateOptions::default().ring
GUARD = 2  # between the rectangle and the ring (invariance.py's)  [tunable]
OPAQUE = 0.95  # opaque_above (D155)
CORE = 0.85  # the strokes' core: the drawn template over this share of its maximum  [tunable]
SEARCH_BOX = (400, 200)  # the provisional profile's search box  [tunable]
GAIN = 0.06  # the shipped profiles' verify.gain, for reading step 9 only


def fail(msg):
    print(f"first-look: {msg}", file=sys.stderr)
    sys.exit(2)


# ── the bytes ────────────────────────────────────────────────────────────────

ZIGZAG_FIRST = 8


def jpeg_facts(b):
    """Baseline or progressive, the components' sampling, DQT 0's first
    coefficients (zigzag order, as stored), the APPn markers present."""
    if not b.startswith(b"\xff\xd8"):
        return None
    i, facts = 2, {"apps": [], "sof": None, "sampling": None, "dqt0": None}
    while i + 4 <= len(b):
        if b[i] != 0xFF:
            break
        m = b[i + 1]
        if m in (0xD8, 0x01) or 0xD0 <= m <= 0xD7:
            i += 2
            continue
        n = struct.unpack(">H", b[i + 2:i + 4])[0]
        seg = b[i + 4:i + 2 + n]
        if 0xE0 <= m <= 0xEF:
            facts["apps"].append(f"APP{m - 0xE0}")
        elif m == 0xDB:
            j = 0
            while j < len(seg):
                pq, tq = seg[j] >> 4, seg[j] & 15
                size = 128 if pq else 64
                vals = seg[j + 1:j + 1 + size]
                if pq:
                    vals = [struct.unpack(">H", vals[k:k + 2])[0] for k in range(0, size, 2)]
                if tq == 0 and facts["dqt0"] is None:
                    facts["dqt0"] = list(vals[:ZIGZAG_FIRST])
                j += 1 + size
        elif m in (0xC0, 0xC1, 0xC2):
            facts["sof"] = "progressive" if m == 0xC2 else "baseline"
            nc = seg[5]
            comps = [(seg[6 + 3 * k + 1] >> 4, seg[6 + 3 * k + 1] & 15) for k in range(nc)]
            facts["sampling"] = comps
        elif m == 0xDA:
            break
        i += 2 + n
    s = facts["sampling"]
    if s and len(s) == 3:
        (h0, v0), rest = s[0], s[1:]
        if all(c == (1, 1) for c in rest):
            facts["subsampling"] = {(1, 1): "4:4:4", (2, 1): "4:2:2", (2, 2): "4:2:0"}.get((h0, v0), f"{h0}x{v0}")
        else:
            facts["subsampling"] = str(s)
    else:
        facts["subsampling"] = str(s)
    return facts


def load(path):
    try:
        b = open(path, "rb").read()
        img = np.asarray(Image.open(path).convert("RGB")).astype(np.float64)
    except OSError as e:
        fail(f"{path}: {e}")
    return b, img


def short(path):
    return os.path.splitext(os.path.basename(path))[0]


# ── geometry: the ring, the quadratic, the area integral ─────────────────────


def basis(u, v):
    return np.stack([np.ones_like(u), u, v, u * u, u * v, v * v], axis=-1)


def ring_fit(img, rect, ring=RING, guard=GUARD, floor=RING):
    """The ring's quadratic around `rect = (x, y, w, h)` (floats allowed),
    over a crop holding the rectangle, the guard, the ring and a floor band
    beyond it. Returns a dict: crop origin, the crop, B (the quadratic over
    the crop), masks (inside, ring, floor) and σ_ring per channel."""
    H, W, _ = img.shape
    x, y, w, h = rect
    x0, y0 = int(math.floor(x)), int(math.floor(y))
    x1, y1 = int(math.ceil(x + w)), int(math.ceil(y + h))
    m = guard + ring + floor
    X0, Y0, X1, Y1 = max(0, x0 - m), max(0, y0 - m), min(W, x1 + m), min(H, y1 + m)
    crop = img[Y0:Y1, X0:X1]
    yy, xx = np.mgrid[Y0:Y1, X0:X1]
    dx = np.maximum(np.maximum(x0 - xx, xx - (x1 - 1)), 0)
    dy = np.maximum(np.maximum(y0 - yy, yy - (y1 - 1)), 0)
    d = np.maximum(dx, dy)
    inside = d == 0
    ring_m = (d > guard) & (d <= guard + ring)
    floor_m = (d > guard + ring) & (d <= guard + ring + floor)
    cx, cy, s = x + w / 2.0, y + h / 2.0, float(max(w, h, 1))
    A = basis((xx + 0.5 - cx) / s, (yy + 0.5 - cy) / s)
    B = np.empty_like(crop)
    for c in range(3):
        k, *_ = np.linalg.lstsq(A[ring_m], crop[..., c][ring_m], rcond=None)
        B[..., c] = A @ k
    res = (crop - B)[ring_m]
    return {"origin": (X0, Y0), "crop": crop, "B": B, "inside": inside, "ring": ring_m,
            "floor": floor_m, "sigma": np.sqrt((res ** 2).mean(0)), "dist": d}


def area_weights(n_out, origin, length, n_in):
    """1-D weights: output pixel j covers [j, j+1); input sample i covers
    [origin + i·length/n_in, origin + (i+1)·length/n_in). W[j, i] is the
    overlap (pixel units), so W @ m is the mean over the pixel, zero where
    the map is not."""
    step = length / n_in
    lo = origin + np.arange(n_in) * step
    hi = lo + step
    j = np.arange(n_out)[:, None]
    return np.clip(np.minimum(j + 1, hi[None, :]) - np.maximum(j, lo[None, :]), 0, None)


def draw(m, x, y, w):
    """The map `m` drawn at origin (x, y) and width w (height by its
    aspect) by the area integral. Returns (ox, oy, image) with the image
    covering whole pixels floor(x)…ceil(x + w)."""
    mh, mw = m.shape
    h = w * mh / mw
    ox, oy = int(math.floor(x)), int(math.floor(y))
    nx, ny = int(math.ceil(x + w)) - ox, int(math.ceil(y + h)) - oy
    Wx = area_weights(nx, x - ox, w, mw)
    Wy = area_weights(ny, y - oy, h, mh)
    return ox, oy, Wy @ m @ Wx.T


def ncc(a, b):
    a = a - a.mean()
    b = b - b.mean()
    den = math.sqrt(float((a * a).sum()) * float((b * b).sum()))
    return float((a * b).sum() / den) if den > 1e-12 else 0.0


def ncc_map(region, t):
    """TM_CCOEFF_NORMED of template t over every whole-pixel place in region."""
    th, tw = t.shape
    rh, rw = region.shape
    if th > rh or tw > rw:
        return None
    tc = t - t.mean()
    tn = math.sqrt(float((tc * tc).sum()))
    # Σ window·tc by FFT correlation (tc sums to zero, so the window's mean drops out)
    fr = np.fft.rfft2(region, (rh, rw))
    ft = np.fft.rfft2(tc[::-1, ::-1], (rh, rw))
    num = np.fft.irfft2(fr * ft, (rh, rw))[th - 1:, tw - 1:]
    # the window's variance from integral images
    def box(a):
        c = np.zeros((rh + 1, rw + 1))
        c[1:, 1:] = a.cumsum(0).cumsum(1)
        return c[th:, tw:] - c[:-th, tw:] - c[th:, :-tw] + c[:-th, :-tw]
    n = th * tw
    s1, s2 = box(region), box(region * region)
    var = np.maximum(s2 - s1 * s1 / n, 0.0)
    den = np.sqrt(var) * tn
    return np.where(den > 1e-9, num / np.maximum(den, 1e-9), 0.0)


def ncc_at(Y, t, x, y, w):
    """NCC of the template drawn at (x, y, w) against the picture's luma,
    over the drawn rectangle grown by two pixels."""
    ox, oy, d = draw(t, x, y, w)
    H, W = Y.shape
    g = 2
    X0, Y0 = ox - g, oy - g
    X1, Y1 = ox + d.shape[1] + g, oy + d.shape[0] + g
    if X0 < 0 or Y0 < 0 or X1 > W or Y1 > H:
        return -2.0
    full = np.zeros((Y1 - Y0, X1 - X0))
    full[g:g + d.shape[0], g:g + d.shape[1]] = d
    return ncc(Y[Y0:Y1, X0:X1], full)


# ── the reference ────────────────────────────────────────────────────────────


def largest_group(mask):
    """The bounding box of the largest 8-connected group of True pixels."""
    h, w = mask.shape
    seen = np.zeros_like(mask, bool)
    best = (0, None)
    for sy, sx in zip(*np.nonzero(mask)):
        if seen[sy, sx]:
            continue
        stack, pts = [(sy, sx)], []
        seen[sy, sx] = True
        while stack:
            cy, cx = stack.pop()
            pts.append((cy, cx))
            for dy in (-1, 0, 1):
                for dx in (-1, 0, 1):
                    ny, nx = cy + dy, cx + dx
                    if 0 <= ny < h and 0 <= nx < w and mask[ny, nx] and not seen[ny, nx]:
                        seen[ny, nx] = True
                        stack.append((ny, nx))
        if len(pts) > best[0]:
            ys = [p[0] for p in pts]
            xs = [p[1] for p in pts]
            best = (len(pts), (min(xs), min(ys), max(xs) + 1, max(ys) + 1))
    return best[1]


def grouped_box(mask, gap=8):
    """The box of the largest group after closing gaps of `gap` pixels
    (a wordmark is several letters; they are one group at this distance)."""
    from numpy.lib.stride_tricks import sliding_window_view as sw

    k = 2 * gap + 1
    p = np.pad(mask, gap)
    grown = sw(p, (k, k)).any(axis=(-1, -2))
    box = largest_group(grown)
    if box is None:
        return None
    x0, y0, x1, y1 = box
    sub = mask[y0:y1, x0:x1]
    ys, xs = np.nonzero(sub)
    return (x0 + xs.min(), y0 + ys.min(), x0 + xs.max() + 1, y0 + ys.max() + 1)


def reference_rect(img, corner, pad, hint=None):
    H, W, _ = img.shape
    if hint:
        bx = tuple(hint)
    else:
        cw, ch = min(corner[0], W), min(corner[1], H)
        X0, Y0 = W - cw, H - ch
        Y = img[Y0:, X0:] @ LUMA
        first = grouped_box(np.abs(Y - np.median(Y)) > 10)
        if first is None:
            fail("the reference's corner has no mark departing 10 levels from its median; give --ref-box")
        bx = (X0 + first[0], Y0 + first[1], first[2] - first[0], first[3] - first[1])
    fit = ring_fit(img, bx)
    dep = np.abs(((fit["crop"] - fit["B"]) @ LUMA))
    t = max(3.0, 4.0 * float((fit["sigma"] @ LUMA)))
    mask = (dep > t) & (fit["dist"] <= GUARD)
    tight = grouped_box(mask)
    ox, oy = fit["origin"]
    x0, y0, x1, y1 = ox + tight[0], oy + tight[1], ox + tight[2], oy + tight[3]
    return (x0 - pad, y0 - pad, x1 - x0 + 2 * pad, y1 - y0 + 2 * pad), t


# ── fits ─────────────────────────────────────────────────────────────────────


def variant_fit(I, B):
    """Per channel at the core samples (n × 3): dark, light and free fits."""
    out = {"dark_a": [], "dark_rms": [], "light_a": [], "light_rms": []}
    for c in range(3):
        i, b = I[:, c], B[:, c]
        a = float((b * (b - i)).sum() / max((b * b).sum(), 1e-9))
        out["dark_a"].append(a)
        out["dark_rms"].append(float(np.sqrt(((i - (1 - a) * b) ** 2).mean())))
        g = 255.0 - b
        a = float(((i - b) * g).sum() / max((g * g).sum(), 1e-9))
        out["light_a"].append(a)
        out["light_rms"].append(float(np.sqrt(((i - (a * 255 + (1 - a) * b)) ** 2).mean())))
    # free: I − B = aL − a·B, one a and one L for the three channels
    y = (I - B).reshape(-1)
    X = np.stack([np.ones(y.size), -B.reshape(-1)], -1)
    (al, a), *_ = np.linalg.lstsq(X, y, rcond=None)
    out["free_a"] = float(a)
    out["free_L"] = float(al / a) if abs(a) > 1e-6 else None
    out["free_rms"] = float(np.sqrt(((y - X @ np.array([al, a])) ** 2).mean()))
    # how well a and L separate: the spread of B at the core (on one flat background
    # only a·(L − B) is known, and L is anyone's guess)
    out["B_spread"] = float(B.std())
    out["variant"] = "dark" if sum(out["dark_rms"]) <= sum(out["light_rms"]) else "light"
    return out


def refine(Y, t, x, y, w, steps=(1.0, 0.5, 0.25, 0.125, 0.0625)):
    """Coordinate descent on |NCC| over (x, y, w)."""
    best = abs(ncc_at(Y, t, x, y, w))
    for s in steps:
        moved = True
        while moved:
            moved = False
            for dx, dy, dw in ((s, 0, 0), (-s, 0, 0), (0, s, 0), (0, -s, 0), (0, 0, s), (0, 0, -s)):
                v = abs(ncc_at(Y, t, x + dx, y + dy, w + dw))
                if v > best + 1e-7:
                    best, x, y, w, moved = v, x + dx, y + dy, w + dw, True
    return x, y, w, best


def locate(img, t, corner_ref, ref_wh, scales):
    """Whole-pixel scale search, then the sub-pixel refinement."""
    H, W, _ = img.shape
    Yf = img @ LUMA
    tw0 = t.shape[1]
    f = W / ref_wh[0]
    cw = min(W, int(round(corner_ref[0] * max(f, 0.5))) + 40)
    ch = min(H, int(round(corner_ref[1] * max(f, 0.5))) + 40)
    X0, Y0 = W - cw, H - ch
    region = Yf[Y0:, X0:]
    best = (-1.0,)
    for w in range(max(8, int(tw0 * scales[0])), int(tw0 * scales[1]) + 1):
        _, _, d = draw(t, 0.0, 0.0, float(w))
        m = ncc_map(region, d)
        if m is None:
            continue
        iy, ix = np.unravel_index(np.abs(m).argmax(), m.shape)
        v = abs(float(m[iy, ix]))
        if v > best[0]:
            best = (v, X0 + ix, Y0 + iy, float(w), float(np.sign(m[iy, ix])))
    _, x, y, w, sign = best
    x, y, w, v = refine(Yf, t, float(x), float(y), w)
    sign = math.copysign(1.0, ncc_at(Yf, t, x, y, w))
    return {"x": x, "y": y, "w": w, "h": w * t.shape[0] / t.shape[1], "ncc": v, "sign": sign}


# ── maps out ─────────────────────────────────────────────────────────────────


def wma(alpha):
    """WMA1, depth 16 (crates/wipemark-pixels/src/alpha.rs)."""
    h, w = alpha.shape
    q = np.clip(np.round(alpha * 65535.0), 0, 65535).astype("<u2")
    return b"WMA1" + struct.pack("<HHB", w, h, 16) + q.tobytes()


def png16(path, alpha):
    q = np.clip(np.round(alpha * 65535.0), 0, 65535).astype(np.uint16)
    Image.fromarray(q).save(path)


def png_eye(path, alpha, zoom=8):
    v = np.clip(alpha / 0.5, 0, 1) * 255
    Image.fromarray(v.astype(np.uint8)).resize(
        (alpha.shape[1] * zoom, alpha.shape[0] * zoom), Image.NEAREST).save(path)


def cleaned_map(alpha, floor):
    """Negative α to 0; a sample under `floor` farther than two pixels from
    one over it to 0 (the ring's noise is not the mark)."""
    from numpy.lib.stride_tricks import sliding_window_view as sw

    a = np.clip(alpha, 0, None)
    strong = np.pad(a > floor, 2)
    near = sw(strong, (5, 5)).any(axis=(-1, -2))
    return np.where(near, a, 0.0)


# ── look ─────────────────────────────────────────────────────────────────────


def fmt(v, n=3):
    if v is None:
        return "—"
    if isinstance(v, (list, tuple)):
        return ", ".join(fmt(x, n) for x in v)
    return f"{v:.{n}f}"


def look(args):
    paths = args.pictures
    ref_path = args.reference or paths[0]
    if ref_path not in paths:
        paths = [ref_path] + paths
    out = args.out
    os.makedirs(os.path.join(out, "maps"), exist_ok=True)
    os.makedirs(os.path.join(out, "catalogue"), exist_ok=True)

    pics = {}
    for p in paths:
        b, img = load(p)
        pics[p] = {"name": short(p), "path": p, "sha256": hashlib.sha256(b).hexdigest(),
                   "size": [img.shape[1], img.shape[0]], "jpeg": jpeg_facts(b), "img": img}

    # 2-4: the reference's rectangle and template
    ref = pics[ref_path]
    rimg = ref["img"]
    rW, rH = ref["size"]
    rect, thr = reference_rect(rimg, tuple(args.corner), args.pad, args.ref_box)
    rx, ry, rw, rh = rect
    rfit = ring_fit(rimg, rect)
    ox, oy = rfit["origin"]
    sl = (slice(ry - oy, ry - oy + rh), slice(rx - ox, rx - ox + rw))
    I_ref = rfit["crop"][sl]
    B_ref = rfit["B"][sl]
    depth = (B_ref - I_ref) @ LUMA
    template = depth.copy()

    # 4: every picture
    rows = []
    for p in paths:
        pic = pics[p]
        if p == ref_path:
            loc = {"x": float(rx), "y": float(ry), "w": float(rw), "h": float(rh), "ncc": 1.0, "sign": 1.0}
            loc["ncc"] = abs(ncc_at(rimg @ LUMA, template, loc["x"], loc["y"], loc["w"]))
            loc["sign"] = math.copysign(1.0, ncc_at(rimg @ LUMA, template, loc["x"], loc["y"], loc["w"]))
        else:
            loc = locate(pic["img"], template, LOCATE, (rW, rH), tuple(args.scales))
        W, H = pic["size"]
        loc["margin_right"] = W - (loc["x"] + loc["w"])
        loc["margin_bottom"] = H - (loc["y"] + loc["h"])
        loc["w_over"] = {"W": loc["w"] / W, "H": loc["w"] / H, "sqrtWH": loc["w"] / math.sqrt(W * H),
                         "short": loc["w"] / min(W, H), "long": loc["w"] / max(W, H)}
        pic["loc"] = loc

        # 6: variant
        r = (loc["x"], loc["y"], loc["w"], loc["h"])
        fit = ring_fit(pic["img"], r)
        fox, foy = fit["origin"]
        dox, doy, dt = draw(template, loc["x"], loc["y"], loc["w"])
        core = np.zeros(fit["crop"].shape[:2], bool)
        sub = (slice(doy - foy, doy - foy + dt.shape[0]), slice(dox - fox, dox - fox + dt.shape[1]))
        core[sub] = dt > CORE * dt.max()
        vf = variant_fit(fit["crop"][core], fit["B"][core])
        vf["core_px"] = int(core.sum())
        inside = np.zeros_like(core)
        inside[sub] = True
        vf["bg_luma"] = float((fit["B"][inside] @ LUMA).mean())
        vf["sigma_ring"] = [float(s) for s in fit["sigma"]]
        pic["variant"] = vf

        # 8, 9: α at this picture, against the reference drawn here
        Iy = fit["crop"][sub] @ LUMA
        By = fit["B"][sub] @ LUMA
        if vf["variant"] == "dark":
            a_here = 1.0 - Iy / np.maximum(By, 1.0)
        else:
            a_here = (Iy - By) / np.maximum(255.0 - By, 1.0)
        pic["alpha_here"] = a_here
        pic["alpha_origin"] = (dox, doy)
        rows.append(p)

    # 7: the dark map from the reference
    Yr = I_ref @ LUMA
    BYr = B_ref @ LUMA
    alpha_Y = 1.0 - Yr / BYr
    alpha_c = [1.0 - I_ref[..., c] / B_ref[..., c] for c in range(3)]
    ring_alpha = 1.0 - (rfit["crop"] @ LUMA) / (rfit["B"] @ LUMA)
    noise_ring = ring_alpha[rfit["ring"]]
    noise_floor = ring_alpha[rfit["floor"]]
    sigma_a = float(noise_ring.std())
    floor = max(3.0 * sigma_a, 0.01)
    support = alpha_Y > floor
    chan = np.stack(alpha_c)
    strong = alpha_Y > 0.1
    dark = {
        "rect": [rx, ry, rw, rh], "threshold_levels": thr,
        "alpha_max": float(alpha_Y.max()), "alpha_p99": float(np.quantile(alpha_Y[support], 0.99)) if support.any() else None,
        "alpha_core_median": float(np.median(alpha_Y[depth > CORE * depth.max()])),
        "alpha_channel_max": [float(a.max()) for a in alpha_c],
        "channel_spread_mean": float((chan.max(0) - chan.min(0))[strong].mean()) if strong.any() else None,
        "channel_core_median": [float(np.median(a[depth > CORE * depth.max()])) for a in alpha_c],
        "ring_sigma_levels": [float(s) for s in rfit["sigma"]],
        "noise_alpha_sd_ring": sigma_a, "noise_alpha_p99_ring": float(np.quantile(np.abs(noise_ring), 0.99)),
        "noise_alpha_sd_floor": float(noise_floor.std()),
        "holes": int((alpha_Y >= args.opaque).sum()), "support_px": int(support.sum()),
        "floor": floor,
    }
    png16(os.path.join(out, "maps", "alpha-dark-Y.png"), np.clip(alpha_Y, 0, 1))
    for c, n in enumerate("RGB"):
        png16(os.path.join(out, "maps", f"alpha-dark-{n}.png"), np.clip(alpha_c[c], 0, 1))
    png_eye(os.path.join(out, "maps", "alpha-dark-Y-x8.png"), np.clip(alpha_Y, 0, 1))
    amap = cleaned_map(alpha_Y, floor)

    # 9: the shapes
    for p in rows:
        pic = pics[p]
        loc = pic["loc"]
        dox, doy, drawn = draw(amap, loc["x"], loc["y"], loc["w"])
        here = pic["alpha_here"]
        h = min(here.shape[0], drawn.shape[0])
        w = min(here.shape[1], drawn.shape[1])
        a, d = here[:h, :w], drawn[:h, :w]
        m = d > 0.02
        slope = float((a[m] * d[m]).sum() / max((d[m] ** 2).sum(), 1e-9)) if m.any() else None
        pic["shape"] = {"ncc": ncc(a, d), "slope": slope,
                        "rms": float(np.sqrt(((a - (slope or 0) * d)[m] ** 2).mean())) if m.any() else None}
        if pic["variant"]["variant"] == "light":
            png16(os.path.join(out, "maps", f"alpha-light-{pic['name']}.png"), np.clip(here, 0, 1))
            png_eye(os.path.join(out, "maps", f"alpha-light-{pic['name']}-x8.png"), np.clip(here, 0, 1))

    # 5: the laws
    ref_loc = ref["loc"]
    laws = {}
    for law, fn in (("W", lambda W, H: W), ("H", lambda W, H: H), ("sqrtWH", lambda W, H: math.sqrt(W * H)),
                    ("short", lambda W, H: min(W, H)), ("long", lambda W, H: max(W, H)), ("constant", lambda W, H: 1.0)):
        k = ref_loc["w"] / fn(rW, rH)
        ratios = [pics[p]["loc"]["w"] / fn(*pics[p]["size"]) for p in rows]
        worst = max(abs(pics[p]["loc"]["w"] - k * fn(*pics[p]["size"])) for p in rows)
        laws[law] = {"cv": float(np.std(ratios) / np.mean(ratios)), "worst_px": float(worst), "k": k}
    margins = {}
    for law, fn in (("constant", lambda p: 1.0), ("W", lambda p: pics[p]["size"][0]), ("mark", lambda p: pics[p]["loc"]["w"])):
        for side in ("margin_right", "margin_bottom"):
            k = ref_loc[side] / fn(ref_path)
            worst = max(abs(pics[p]["loc"][side] - k * fn(p)) for p in rows)
            margins[f"{law}/{side}"] = {"worst_px": float(worst), "k": k}
    size_law = min(laws, key=lambda k: laws[k]["worst_px"])

    # 10: the provisional profile
    catalogue = provisional(args, pics, rows, ref_path, amap, laws, size_law)

    # write
    summary = {"date": args.date, "reference": ref["name"], "rect": [rx, ry, rw, rh], "dark": dark,
               "laws": laws, "size_law": size_law, "margins": margins, "catalogue": catalogue,
               "pictures": [{k: v for k, v in pics[p].items() if k not in ("img", "alpha_here", "alpha_origin")}
                            for p in rows]}
    with open(os.path.join(out, "look.json"), "w") as f:
        json.dump(summary, f, indent=1, default=lambda o: o.tolist() if hasattr(o, "tolist") else str(o))
    write_look_md(os.path.join(out, "look.md"), summary)
    print(f"first-look: wrote {out}/look.md, look.json, maps/, catalogue/")


def provisional(args, pics, rows, ref_path, amap, laws, size_law):
    """F2: one --catalogue file, two profiles, a map per picture size seen."""
    cat_dir = os.path.join(args.out, "catalogue")
    for f in os.listdir(cat_dir):
        if f.endswith(".wma") or f == "marks.json":
            os.remove(os.path.join(cat_dir, f))
    ref = pics[ref_path]
    rW, rH = ref["size"]
    rl = ref["loc"]
    mh, mw = amap.shape
    k_w = laws[size_law]["k"]
    law_fn = {"W": lambda W, H: W, "H": lambda W, H: H, "sqrtWH": lambda W, H: math.sqrt(W * H),
              "short": lambda W, H: min(W, H), "long": lambda W, H: max(W, H), "constant": lambda W, H: 1.0}[size_law]
    mr, mb = rl["margin_right"], rl["margin_bottom"]
    profiles = []
    sizes = sorted({tuple(pics[p]["size"]) for p in rows})
    made = []
    light_scale = args.light_scale
    if light_scale == "auto":
        slopes = [pics[p]["shape"]["slope"] for p in rows
                  if pics[p]["variant"]["variant"] == "light" and pics[p]["shape"]["slope"]]
        light_scale = float(np.median(slopes)) if slopes else 1.0
    light_scale = round(float(light_scale), 3)
    args.light_scale_used = light_scale
    for variant, logo, scale in (("dark", [0, 0, 0], 1.0), ("light", [255, 255, 255], light_scale)):
        pid = f"grok-x-{variant}"
        alpha, placements, assets = [], [], {}
        base = np.clip(amap * scale, 0, 0.999)
        for (W, H) in sizes:
            if (W, H) == (rW, rH):
                m = base
                margin = [int(round(mr)), int(round(mb))]
                note = "the reference's own map"
            else:
                seen = [pics[p]["loc"] for p in rows if tuple(pics[p]["size"]) == (W, H)]
                if args.rows == "measured" and seen:
                    x, y, w = seen[0]["x"], seen[0]["y"], seen[0]["w"]
                    how = "measured"
                else:
                    w = k_w * law_fn(W, H)
                    x, y = W - mr - w, H - mb - w * mh / mw
                    how = "by the laws"
                ox, oy, m = draw(base, x, y, w)
                margin = [W - (ox + m.shape[1]), H - (oy + m.shape[0])]
                note = f"{how}: drawn at ({x:.3f}, {y:.3f}) width {w:.3f}"
            name = f"{pid}-{m.shape[1]}x{m.shape[0]}"
            data = wma(m)
            fname = f"{name}.wma"
            assets[fname] = data
            alpha.append({"id": name, "asset": fname, "sha256": hashlib.sha256(data).hexdigest(),
                          "size": [m.shape[1], m.shape[0]], "fitted": True})
            placements.append({"when": {"width": W, "height": H}, "corner": "bottom-right",
                               "margin": margin, "alpha": name})
            made.append({"profile": pid, "size": [W, H], "map": name, "margin": margin, "note": note})
        search_map = f"{pid}-{mw}x{mh}"
        if search_map not in [a["id"] for a in alpha]:
            data = wma(base)
            assets[f"{search_map}.wma"] = data
            alpha.append({"id": search_map, "asset": f"{search_map}.wma",
                          "sha256": hashlib.sha256(data).hexdigest(), "size": [mw, mh], "fitted": True})
        profiles.append({
            "id": pid, "vendor": "xai", "product": "grok", "mark": f"wordmark-{variant}",
            "observed": {"from": args.date[:7], "until": None}, "status": "provisional",
            "blend": {"model": "encoded", "logo": logo, "logo_map": None},
            "opaque_above": OPAQUE, "alpha": alpha, "placements": placements,
            "search": {"corner": "bottom-right", "within": list(args.search_box),
                       "sizes": [max(8, int(mw * 0.3)), int(mw * 1.6)], "alpha": search_map},
            "detect": {"min_ncc": 0.70}, "verify": {"gain": GAIN, "edge_ratio": 0.30, "out_of_range": 0.01},
            "source": {"from": "scripts/grok/first-look.py", "commit": None, "licence": None, "copyright": None},
        })
        for fname, data in assets.items():
            with open(os.path.join(cat_dir, fname), "wb") as f:
                f.write(data)
    with open(os.path.join(cat_dir, "marks.json"), "w") as f:
        json.dump({"schema": 1, "profiles": profiles}, f, indent=2)
    return {"file": os.path.join(cat_dir, "marks.json"), "rows": made, "rows_from": args.rows, "light_scale": light_scale,
            "size_law": size_law, "k": k_w, "margin": [mr, mb]}


def write_look_md(path, s):
    L = []
    L.append(f"# Grok, a first look — {s['date']}\n")
    L.append("Generated by `scripts/grok/first-look.py look`. Every figure below is this run's.\n")
    L.append("## The pictures\n")
    L.append("| picture | size | sha256 | JPEG | sampling | DQT 0 (first 8, zigzag) | APPn |")
    L.append("|---|---|---|---|---|---|---|")
    for p in s["pictures"]:
        j = p["jpeg"] or {}
        L.append(f"| `{p['name']}` | {p['size'][0]}×{p['size'][1]} | `{p['sha256']}` | {j.get('sof', '—')} | "
                 f"{j.get('subsampling', '—')} | {j.get('dqt0', '—')} | {', '.join(j.get('apps', []))} |")
    L.append(f"\n## Where the mark is\n\nThe template: the reference `{s['reference']}`'s depth `B_Y − I_Y` "
             f"over {s['rect'][2]}×{s['rect'][3]} at ({s['rect'][0]}, {s['rect'][1]}) (tight box at "
             f"{s['dark']['threshold_levels']:.1f} levels, grown by the pad).\n")
    L.append("| picture | NCC | sign | x | y | width | height | margin right | margin bottom | w/W | w/H | w/√(WH) |")
    L.append("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for p in s["pictures"]:
        l = p["loc"]
        L.append(f"| `{p['name']}` | {l['ncc']:.3f} | {'dark' if l['sign'] < 0 else 'light'} | {l['x']:.2f} | {l['y']:.2f} | "
                 f"{l['w']:.2f} | {l['h']:.2f} | {l['margin_right']:.2f} | {l['margin_bottom']:.2f} | "
                 f"{l['w_over']['W']:.4f} | {l['w_over']['H']:.4f} | {l['w_over']['sqrtWH']:.4f} |")
    L.append("\nThe sign is that of the correlation of the picture's luma with the reference's *depth*: "
             "negative (dark) where the mark darkens the picture as it does the reference, positive (light) where "
             "it lightens it.\n")
    L.append("### Size and margin laws\n")
    L.append("| width ∝ | k (through the reference) | spread (CV) | worst picture, px |")
    L.append("|---|---|---|---|")
    for k, v in s["laws"].items():
        L.append(f"| {k} | {v['k']:.5f} | {v['cv']:.4f} | {v['worst_px']:.2f} |")
    L.append(f"\nThe law taken: width ∝ **{s['size_law']}**.\n")
    L.append("| margin law | k | worst picture, px |")
    L.append("|---|---|---|")
    for k, v in s["margins"].items():
        L.append(f"| {k} | {v['k']:.5f} | {v['worst_px']:.2f} |")
    L.append("\n## Which variant\n")
    L.append("At the strokes' core against the ring's quadratic. *dark*: `I = (1 − a)·B`; *light*: "
             "`I = a·255 + (1 − a)·B`; *free*: `I − B = a·(L − B)`, one `a` and one `L` for the three channels.\n")
    L.append("| picture | core px | background luma | σ_ring R, G, B | dark a (R, G, B) | dark RMS | "
             "light a (R, G, B) | light RMS | free a | free L | free RMS | B spread | variant |")
    L.append("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for p in s["pictures"]:
        v = p["variant"]
        L.append(f"| `{p['name']}` | {v['core_px']} | {v['bg_luma']:.1f} | {fmt(v['sigma_ring'], 1)} | "
                 f"{fmt(v['dark_a'])} | {fmt(v['dark_rms'], 1)} | {fmt(v['light_a'])} | {fmt(v['light_rms'], 1)} | "
                 f"{fmt(v['free_a'])} | {fmt(v['free_L'], 1)} | {fmt(v['free_rms'], 1)} | {v['B_spread']:.1f} | **{v['variant']}** |")
    d = s["dark"]
    L.append("\n## The dark variant's map, from the reference\n")
    L.append("| | |\n|---|---|")
    L.append(f"| α max (luma) | {d['alpha_max']:.3f} |")
    L.append(f"| α at the core, median (luma) | {d['alpha_core_median']:.3f} |")
    L.append(f"| α at the core, median R, G, B | {fmt(d['channel_core_median'])} |")
    L.append(f"| α max R, G, B | {fmt(d['alpha_channel_max'])} |")
    L.append(f"| channels' spread where α > 0.1 (mean of max − min) | {fmt(d['channel_spread_mean'])} |")
    L.append(f"| α p99 over the support | {fmt(d['alpha_p99'])} |")
    L.append(f"| σ_ring, levels R, G, B | {fmt(d['ring_sigma_levels'], 2)} |")
    L.append(f"| α noise over the ring: sd, p99 of |α| | {d['noise_alpha_sd_ring']:.4f}, {d['noise_alpha_p99_ring']:.4f} |")
    L.append(f"| α noise over the band past the ring: sd | {d['noise_alpha_sd_floor']:.4f} |")
    L.append(f"| support (α over {d['floor']:.4f}) | {d['support_px']} px |")
    L.append(f"| holes (α ≥ {OPAQUE}) | {d['holes']} |")
    L.append("\n## One shape, two colours?\n")
    L.append("The reference's map (cleaned) drawn at each picture's place and width, against the α measured "
             "there (dark: `1 − I/B`; light: `(I − B)/(255 − B)`, rough).\n")
    L.append("| picture | variant | NCC | relative opacity (slope) | RMS after the slope |")
    L.append("|---|---|---|---|---|")
    for p in s["pictures"]:
        sh = p["shape"]
        L.append(f"| `{p['name']}` | {p['variant']['variant']} | {sh['ncc']:.3f} | {fmt(sh['slope'])} | {fmt(sh['rms'], 4)} |")
    c = s["catalogue"]
    L.append(f"\n## The provisional profile\n\n`{c['file']}` — width = {c['k']:.5f} × {c['size_law']}, margins "
             f"{c['margin'][0]:.2f} / {c['margin'][1]:.2f} px (the reference's), the light map scaled by {c['light_scale']}.\n")
    L.append("| profile | size | map | margin | how |")
    L.append("|---|---|---|---|---|")
    for r in c["rows"]:
        L.append(f"| `{r['profile']}` | {r['size'][0]}×{r['size'][1]} | `{r['map']}` | {r['margin']} | {r['note']} |")
    with open(path, "w") as f:
        f.write("\n".join(L) + "\n")


# ── product ──────────────────────────────────────────────────────────────────


def run_cli(cli, args):
    p = subprocess.run([cli] + args, capture_output=True, text=True)
    try:
        js = json.loads(p.stdout) if p.stdout.strip() else None
    except json.JSONDecodeError:
        js = None
    return p.returncode, js, p.stderr.strip()


def crops(before, after, rect, path, zoom=4, margin=24):
    H, W, _ = before.shape
    x, y, w, h = rect
    X0, Y0 = max(0, int(x) - margin), max(0, int(y) - margin)
    X1, Y1 = min(W, int(math.ceil(x + w)) + margin), min(H, int(math.ceil(y + h)) + margin)
    fit = ring_fit(after, rect)
    ox, oy = fit["origin"]
    dev = np.full(after.shape[:2], 128.0)
    dsub = ((fit["crop"] - fit["B"]) @ LUMA) * 4 + 128
    dev[oy:oy + dsub.shape[0], ox:ox + dsub.shape[1]] = dsub
    panels = [before[Y0:Y1, X0:X1], after[Y0:Y1, X0:X1],
              np.repeat(np.clip(dev[Y0:Y1, X0:X1], 0, 255)[..., None], 3, -1)]
    gap = np.full((Y1 - Y0, 2, 3), 255.0)
    row = np.concatenate([panels[0], gap, panels[1], gap, panels[2]], 1)
    im = Image.fromarray(np.clip(np.round(row), 0, 255).astype(np.uint8))
    im.resize((im.width * zoom, im.height * zoom), Image.NEAREST).save(path)


def product(args):
    out = args.out
    look_path = args.look or os.path.join(out, "look.json")
    rects, variants, catalogue = {}, {}, None
    if os.path.exists(look_path):
        looked = json.load(open(look_path))
        catalogue = looked["catalogue"]
        for p in looked["pictures"]:
            l = p["loc"]
            rects[p["sha256"]] = (l["x"], l["y"], l["w"], l["h"])
            variants[p["sha256"]] = p["variant"]["variant"]
    os.makedirs(os.path.join(out, "product"), exist_ok=True)
    results = []
    for p in args.pictures:
        name = short(p)
        b, before = load(p)
        sha = hashlib.sha256(b).hexdigest()
        ie, ij, ierr = run_cli(args.cli, ["inspect", p, "--json"])
        dst = os.path.join(out, "product", f"{name}.cleaned.jpeg")
        if os.path.exists(dst):
            os.remove(dst)
        ce, cj, cerr = run_cli(args.cli, ["clean", p, "-o", dst, "--json"])
        r = {"name": name, "sha256": sha, "inspect_exit": ie, "clean_exit": ce, "inspect_stderr": ierr,
             "clean_stderr": cerr, "inspect": ij, "clean": cj, "written": os.path.exists(dst)}
        vis = ((cj or {}).get("report") or {}).get("visible") or {}
        found = vis.get("found") or []
        rect = rects.get(sha)
        if found:
            px = found[0]["pixels"]
            rect = (px["x"], px["y"], px["width"], px["height"])
        if rect:
            after = load(dst)[1] if r["written"] else before
            cp = os.path.join(out, "product", f"{name}-before-after-x4.png")
            crops(before, after, rect, cp)
            r["crops"] = cp
        results.append(r)
        if args.invert_dark and variants.get(sha) == "dark":
            results.append(inverted(args, p, name, sha, before, rect))
        if args.control and catalogue and sha in variants:
            results.append(control(args, name, sha, before, variants[sha], catalogue))
    with open(os.path.join(out, "product.json"), "w") as f:
        json.dump(results, f, indent=1)
    write_product_md(os.path.join(out, "product.md"), results)
    print(f"first-look: wrote {out}/product.md, product.json, product/")


def read_wma(path):
    b = open(path, "rb").read()
    w, h, depth = struct.unpack("<HHB", b[4:9])
    q = np.frombuffer(b[9:], "<u2" if depth == 16 else "u1").reshape(h, w)
    return q / (65535.0 if depth == 16 else 255.0)


def control(args, name, sha, before, variant, catalogue):
    """A measurement, not the product's road: does the product prove a
    *perfect* blend of the provisional map over this picture's own texture?
    The mark's rectangle (grown by 12 pixels) is overwritten with the patch
    of the same picture `--control-shift` × the mark's width to its left —
    texture, no mark — and the row's own map for this size is blended there
    at the row's place, `α·L + (1 − α)·O` with L the variant's logo, rounded;
    a dark picture is then inverted as `inverted` does. Written as a PNG and
    examined. Proved here and refused on the real mark: the map is not the
    mark Grok drew at this size (its shape, scale or edge). Refused here too:
    this texture under this map is past what the product's checks accept,
    whatever the map."""
    H, W, _ = before.shape
    row = next((r for r in catalogue["rows"] if r["profile"] == "grok-x-light" and r["size"] == [W, H]), None)
    if row is None:
        return {"name": f"{name} (control)", "sha256": sha, "skipped": "no row for this size"}
    a = read_wma(os.path.join(os.path.dirname(catalogue["file"]), f"{row['map'].replace('light', 'dark')}.wma"))
    mh, mw = a.shape
    x, y = W - row["margin"][0] - mw, H - row["margin"][1] - mh
    g = 12
    X0, Y0, X1, Y1 = max(0, x - g), max(0, y - g), min(W, x + mw + g), min(H, y + mh + g)
    shift = int(round(args.control_shift * mw))
    if X0 - shift < 0:
        return {"name": f"{name} (control)", "sha256": sha, "skipped": "no room to the left for a patch"}
    img = before.copy()
    img[Y0:Y1, X0:X1] = before[Y0:Y1, X0 - shift:X1 - shift]
    logo = 0.0 if variant == "dark" else 255.0
    sub = img[y:y + mh, x:x + mw]
    img[y:y + mh, x:x + mw] = np.round(a[..., None] * logo + (1 - a[..., None]) * sub)
    if variant == "dark":
        img = 255.0 - img
    src = os.path.join(args.out, "product", f"{name}.control.png")
    Image.fromarray(np.clip(img, 0, 255).astype(np.uint8)).save(src)
    ie, ij, ierr = run_cli(args.cli, ["inspect", src, "--json"])
    dst = os.path.join(args.out, "product", f"{name}.control.cleaned.png")
    if os.path.exists(dst):
        os.remove(dst)
    ce, cj, cerr = run_cli(args.cli, ["clean", src, "-o", dst, "--json"])
    r = {"name": f"{name} (control: the map blended over its own texture, PNG)", "sha256": sha,
         "inspect_exit": ie, "clean_exit": ce, "inspect_stderr": ierr, "clean_stderr": cerr,
         "inspect": ij, "clean": cj, "written": os.path.exists(dst), "control": True}
    if r["written"]:
        shown = img if variant != "dark" else 255.0 - img
        after = load(dst)[1]
        after = after if variant != "dark" else 255.0 - after
        cp = os.path.join(args.out, "product", f"{name}-control-before-after-x4.png")
        crops(shown, after, (x, y, mw, mh), cp)
        r["crops"] = cp
    return r


def inverted(args, p, name, sha, before, rect):
    """A measurement, not the product's road: the picture inverted (255 − I)
    and written as a PNG. A black-logo blend `(1 − α)·B` inverts exactly into
    a white-logo blend `α·255 + (1 − α)·(255 − B)` with the same α, so the
    light profile (the same map, logo 255) is what proves it there. It says
    what the proof and the restoration would make of the dark map were the
    proposal not limited to marks lighter than the picture (its NCC is
    signed). A PNG is a lossless source: no planes, no DCT-POCS, no texture
    check — another road than the JPEG's."""
    inv = os.path.join(args.out, "product", f"{name}.inverted.png")
    Image.fromarray((255 - before).astype(np.uint8)).save(inv)
    ie, ij, ierr = run_cli(args.cli, ["inspect", inv, "--json"])
    dst = os.path.join(args.out, "product", f"{name}.inverted.cleaned.png")
    if os.path.exists(dst):
        os.remove(dst)
    ce, cj, cerr = run_cli(args.cli, ["clean", inv, "-o", dst, "--json"])
    r = {"name": f"{name} (inverted, PNG)", "sha256": sha, "inspect_exit": ie, "clean_exit": ce,
         "inspect_stderr": ierr, "clean_stderr": cerr, "inspect": ij, "clean": cj,
         "written": os.path.exists(dst), "inverted": True}
    found = ((((cj or {}).get("report") or {}).get("visible") or {}).get("found")) or []
    if found:
        px = found[0]["pixels"]
        rect = (px["x"], px["y"], px["width"], px["height"])
    if rect:
        after = 255.0 - load(dst)[1] if r["written"] else before
        cp = os.path.join(args.out, "product", f"{name}-inverted-before-after-x4.png")
        crops(before, after, rect, cp)
        r["crops"] = cp
    return r


def write_product_md(path, results):
    L = ["# Grok, a first look — the product on the pictures\n",
         "Generated by `scripts/grok/first-look.py product` with a CLI built by `scripts/grok/cli-with-profile.sh`.\n",
         "| picture | inspect exit | clean exit | written | finding | placed | NCC | verdict | gain k* | edge ratio | "
         "out of range | restored | outline | texture | marks left |",
         "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"]
    for r in results:
        if r.get("skipped"):
            L.append(f"| `{r['name']}` | | | | skipped: {r['skipped']} | | | | | | | | | | |")
            continue
        rep = (r["clean"] or {}).get("report") or {}
        vis = rep.get("visible") or {}
        found = vis.get("found") or []
        restored = vis.get("restored") or []
        if not found:
            L.append(f"| `{r['name']}` | {r['inspect_exit']} | {r['clean_exit']} | {r['written']} | none | | | | | | | | | | "
                     f"{rep.get('marks_left')} |")
            continue
        for i, f in enumerate(found):
            sc = f.get("scores") or {}
            rs = restored[i] if i < len(restored) else {}
            L.append(f"| `{r['name']}` | {r['inspect_exit']} | {r['clean_exit']} | {r['written']} | `{f['profile']}` | "
                     f"{f.get('placed')}{'' if f.get('row') is None else ' ' + str(f.get('row'))} | {f['ncc']:.3f} | "
                     f"{f['verdict']}{'' if not f.get('refusal') else ' (' + json.dumps(f['refusal']) + ')'} | "
                     f"{fmt(sc.get('gain'))} | {fmt(sc.get('edge_ratio'))} | {fmt(sc.get('out_of_range'), 4)} | "
                     f"{bool(rs)} | {fmt(rs.get('outline'))}{' left' if rs.get('outline_left') else ''} | "
                     f"{fmt(rs.get('texture'))}{' left' if rs.get('texture_left') else ''} | {rep.get('marks_left')} |")
            for a in f.get("also_tried") or []:
                L.append(f"| | | | | ↳ also `{a['profile']}` | | | {'verified' if a.get('verified') else json.dumps(a.get('refusal'))} "
                         f"| | | | | | | |")
    L.append("\n## Restorations, in full\n")
    for r in results:
        vis = (((r.get("clean") or {}).get("report") or {}).get("visible") or {})
        for rs in vis.get("restored") or []:
            L.append(f"* `{r['name']}`: `{json.dumps(rs)}`")
    L.append("\n## Before | after | the result's departure from its ring's quadratic ×4\n")
    for r in results:
        if r.get("crops"):
            L.append(f"* `{r['name']}`: `{r['crops']}`")
    with open(path, "w") as f:
        f.write("\n".join(L) + "\n")


def main():
    today = datetime.date.today().isoformat()
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    a = sub.add_parser("look", help="F1 and F2: where, which variant, the maps, the provisional catalogue")
    a.add_argument("pictures", nargs="+")
    a.add_argument("--reference")
    a.add_argument("--out")
    a.add_argument("--date", default=today)
    a.add_argument("--corner", nargs=2, type=int, default=list(CORNER))
    a.add_argument("--ref-box", nargs=4, type=int, metavar=("X", "Y", "W", "H"))
    a.add_argument("--scales", nargs=2, type=float, default=list(SCALES))
    a.add_argument("--pad", type=int, default=PAD)
    a.add_argument("--opaque", type=float, default=OPAQUE)
    a.add_argument("--light-scale", default="1.0",
                   help="the light map = the dark map × this; 'auto' = the median relative opacity of the light pictures")
    a.add_argument("--search-box", nargs=2, type=int, default=list(SEARCH_BOX))
    a.add_argument("--rows", choices=("law", "measured"), default="measured",
                   help="rows for the sizes other than the reference's: where step 4 measured the mark in the "
                        "picture of that size (default), or where the size and margin laws put it")
    b = sub.add_parser("product", help="F3: the CLI with the provisional profile on the pictures")
    b.add_argument("pictures", nargs="+")
    b.add_argument("--cli", required=True)
    b.add_argument("--out")
    b.add_argument("--look")
    b.add_argument("--control", action="store_true",
                   help="also blend the row's map over the picture's own texture beside the mark (a measurement)")
    b.add_argument("--control-shift", type=float, default=1.5)
    b.add_argument("--invert-dark", action="store_true",
                   help="also run every picture look.json calls dark inverted, as a PNG (a measurement of the map)")
    b.add_argument("--date", default=today)
    args = ap.parse_args()
    if args.out is None:
        args.out = os.path.join("reports", f"grok-first-look-{args.date}")
    os.makedirs(args.out, exist_ok=True)
    look(args) if args.cmd == "look" else product(args)


if __name__ == "__main__":
    main()
