#!/usr/bin/env python3
"""Roughness of a restored mark against its surroundings, measured independently.

What it is for
--------------
Written by the host verifier of the images series' fourth round (2026-10-05),
which verified `images/series-v3` at `ebf421a` (the D250-D253 texture
criterion, U2) for the coordinator of `GigLaboCom/wipemark-app`; findings:
Watchword FILE `wipemark-task-images-followups-5-2026-10-05`, "What holds".
The fourth round added a texture criterion (D250): the 95th percentile, over
the pixels a restoration changed, of each one's (Y, Cb, Cr) distance from
its eight neighbours' mean, against the same around the mark. This is that
statistic written again with no code of Wipemark's, so the product's
`texture` / `texture_around` could be checked file by file ("your tables
reproduce to within 0.005 per file"), plus a naive inverse to show the
texture is in the input before any restoration ("the independent naive
inverse gives the same texture figures"). It also found the two pixel sets
the fifth round's V4 wrote down: the product's "around" includes the
square's own pixels under alpha 0.002 (2.8-3.4 on 4:4:4 q95) where a ring
8-36 px out reads 1.0-1.3, and the measured set is alpha in [0.002, 0.95),
3 868 px against the 3 441 a restoration reports changed.

What it does
------------
* `roughmap`: per pixel, the Euclidean distance in BT.601 (Y, Cb, Cr) from
  the mean of its (up to) eight neighbours.
* `masks`: with the V1 map at 96 px / 64 px margin, bottom right — `mark`
  (in the square, alpha in [0.002, 0.95)), `near` (in the square under
  alpha 0.002, or 1-4 px outside: the product's "around"), `far` (8-36 px
  outside).
* `rough`: the 95th percentile of `roughmap` over each, and the size of
  `mark`.
* `naive`: `O = (I - a*logo) / (1 - a)` where alpha < 0.95, logo
  (252.1, 253.5, 252.8), rounded and clamped — what any inverse does.

Usage
-----
    python3 rough.py PICTURE [MORE ...]

Imported by `batch.py`; imports `meas.py` beside it (the V1 map, read from
this checkout or `$WIPEMARK_REPO`).

Needs numpy and Pillow (a venv; the verifier's had Pillow 12.3.0 — libjpeg
6.2 — and numpy 2.5.3; nothing in the repository depends on them).
Inputs are the owner's stickers, Watchword FILE
`wipemark-gemini-stickers-2026-10-04` (sha256
5a54435bbd600b43113ade474ad08f6f8b83dc10cdb637eca84847515b991f04); see
`../README.md`.

Output
------
One line per file: `mark near far n=<pixels>` — the p95 roughness over the
mark, the product-like surroundings and the far ring, in levels. A mark much
rougher than both (the product says it over 5.5 and twice "around") is a
texture an eye can find.
"""
import numpy as np
from meas import ALPHA, SIZE, MARGIN, rgb

LOGO = np.array([252.1, 253.5, 252.8])

def ycc(p):
    r, g, b = p[..., 0], p[..., 1], p[..., 2]
    return np.stack([0.299*r + 0.587*g + 0.114*b,
                     -0.168736*r - 0.331264*g + 0.5*b,
                     0.5*r - 0.418688*g - 0.081312*b], -1)

def roughmap(img):
    y = ycc(img)
    H, W, _ = y.shape
    pad = np.pad(y, ((1, 1), (1, 1), (0, 0)), mode="constant")
    cnt = np.pad(np.ones((H, W)), 1, mode="constant")
    s = np.zeros_like(y); n = np.zeros((H, W))
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            if dy == 0 and dx == 0: continue
            s += pad[1+dy:1+dy+H, 1+dx:1+dx+W]
            n += cnt[1+dy:1+dy+H, 1+dx:1+dx+W]
    return np.sqrt(((y - s / n[..., None]) ** 2).sum(-1))

def masks(H, W, x0=None, y0=None):
    if x0 is None:
        x0 = W - MARGIN - SIZE; y0 = H - MARGIN - SIZE
    a = np.zeros((H, W)); a[y0:y0+SIZE, x0:x0+SIZE] = ALPHA
    yy, xx = np.mgrid[0:H, 0:W]
    dx = np.maximum(np.maximum(x0 - xx, xx - (x0 + SIZE - 1)), 0)
    dy = np.maximum(np.maximum(y0 - yy, yy - (y0 + SIZE - 1)), 0)
    d = np.maximum(dx, dy)
    inside = d == 0
    mark = inside & (a >= 0.002) & (a < 0.95)
    near = (inside & (a < 0.002)) | ((d >= 1) & (d <= 4))
    far = (d >= 8) & (d <= 36)
    return mark, near, far, x0, y0

def rough(img, x0=None, y0=None):
    H, W, _ = img.shape
    r = roughmap(img)
    mark, near, far, *_ = masks(H, W, x0, y0)
    return (float(np.percentile(r[mark], 95)), float(np.percentile(r[near], 95)),
            float(np.percentile(r[far], 95)), int(mark.sum()))

def naive(img):
    H, W, _ = img.shape
    x0 = W - MARGIN - SIZE; y0 = H - MARGIN - SIZE
    out = img.copy()
    a = ALPHA[..., None]
    p = img[y0:y0+SIZE, x0:x0+SIZE]
    r = np.where(a < 0.95, (p - a * LOGO) / (1 - a), p)
    out[y0:y0+SIZE, x0:x0+SIZE] = np.clip(np.round(r), 0, 255)
    return out

if __name__ == "__main__":
    import sys
    for p in sys.argv[1:]:
        print(p.split("/")[-1], "%.2f %.2f %.2f n=%d" % rough(rgb(p)))
