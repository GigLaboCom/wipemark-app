#!/usr/bin/env python3
"""A naive inverse of the V1 mark, to tell the restoration's error from the input's.

What it is for
--------------
Written by the host verifier of the images series' third round (2026-10-05,
`images/series-v3` at `1a22a54`; findings: Watchword FILE
`wipemark-task-images-followups-4-2026-10-05`, U2). On 4:4:4 q95 JPEGs the
product's restoration left an 8x8 checker along the sparkle's contour. This
script restores the same corner with the plainest possible formula, sharing
no code with Wipemark, to show the texture is the input JPEG's error
amplified by 1/(1 - alpha) and not a restoration bug ("a naive inverse gives
the same 9.7"), and what re-saving the result as 4:4:4 q95 does to the mean
colour (U2's "the re-encode", L4's Delta E figures).

What it does
------------
1. Places the measured V1 map at 96 px / 64 px margin, bottom right.
2. Inverts `O = (I - a*logo) / (1 - a)` where alpha < 0.99, with the logo
   colour (252.1, 253.5, 252.8) the measured map was fitted with, rounded
   and clamped; pixels at alpha >= 0.99 are left.
3. Saves the result as PNG and as JPEG 4:4:4 q95 (Pillow), and measures both
   with `meas.measure`.

Usage
-----
    python3 naive.py MARKED.jpg [MORE ...]

Writes `<name>.png` and `<name>.re444.jpg` into `$NAIVE_OUT` (default
`naive/` under the current directory, created if missing — the verifier ran
it from its scratch directory with a `naive/` already there).

Needs numpy and Pillow (a venv; the verifier's had Pillow 12.3.0 and numpy
2.5.3 — nothing in the repository depends on them) and `meas.py` beside it,
which reads the V1 alpha map from this checkout (`WIPEMARK_REPO` to read
another). Inputs are the owner's stickers, Watchword FILE
`wipemark-gemini-stickers-2026-10-04` (sha256
5a54435bbd600b43113ade474ad08f6f8b83dc10cdb637eca84847515b991f04); see
`../README.md`.

Output
------
One line per file: the band's mean step in R, G, B, colour step and
Delta E 2000 for the naive PNG and for it re-encoded. Compare with
`batch.py`'s "mine" columns over the product's output: the same figures
mean the product's restoration is no worse than the formula itself.
"""
import sys, numpy as np, io
from PIL import Image
from meas import rgb, ALPHA, SIZE, MARGIN, measure
LOGO=np.array([252.1,253.5,252.8])
def naive(path, out):
    img=rgb(path); H,W,_=img.shape; x0=W-MARGIN-SIZE; y0=x0
    a=ALPHA[...,None]
    p=img[y0:y0+SIZE,x0:x0+SIZE]
    r=np.where(a<0.99,(p-a*LOGO)/(1-a),p)
    r=np.clip(np.round(r),0,255)
    img[y0:y0+SIZE,x0:x0+SIZE]=r
    Image.fromarray(img.astype(np.uint8)).save(out)
    return img
import os
OUT=os.environ.get('NAIVE_OUT','naive'); os.makedirs(OUT,exist_ok=True)
for p in sys.argv[1:]:
    n=p.split('/')[-1]
    img=naive(p,OUT+'/'+n+'.png')
    m=measure(OUT+'/'+n+'.png')
    Image.fromarray(img.astype(np.uint8)).save(OUT+'/'+n+'.re444.jpg',quality=95,subsampling=0)
    m2=measure(OUT+'/'+n+'.re444.jpg')
    f=lambda m: f"RGB=({m['R']:+.2f},{m['G']:+.2f},{m['B']:+.2f}) C={m['chroma']:.2f} dE={m['dE00']:.2f}"
    print(n,'naive:',f(m),'| re-encoded 444 q95:',f(m2))
