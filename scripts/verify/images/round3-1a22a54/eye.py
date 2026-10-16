#!/usr/bin/env python3
"""Panels for looking at a restored mark: as it is, and its deviation amplified.

What it is for
--------------
Written by the host verifier of the images series' third round (2026-10-05,
`images/series-v3` at `1a22a54`; findings: Watchword FILE
`wipemark-task-images-followups-4-2026-10-05`, U2). A figure under a bound
is not the same as nothing visible; this made the pictures behind "plainly
visible at x5 with no amplification; findable at x2" for the 4:4:4 q95
checker, and the comparisons of a PNG, a 4:4:4 and a 4:2:0 restoration of
the same sticker (`victory`, `torch`).

What it does
------------
For each picture: the mark's 96 px square at a 64 px margin plus 24 px
around it, (a) x4 nearest neighbour as it is, and (b) x4 with each pixel's
deviation from a ring's mean (rows 8-36 px above and below the square)
amplified x20 around mid grey. The two sit side by side; one row per
picture, stacked into one PNG.

Usage
-----
    python3 eye.py OUT.png RESTORED_1.png [RESTORED_2.jpg ...]

Needs numpy and Pillow (a venv; the verifier's had Pillow 12.3.0 and numpy
2.5.3 — nothing in the repository depends on them) and `meas.py` beside it,
which reads the V1 alpha map from this checkout (`WIPEMARK_REPO` to read
another). Inputs are the owner's stickers, Watchword FILE
`wipemark-gemini-stickers-2026-10-04` (sha256
5a54435bbd600b43113ade474ad08f6f8b83dc10cdb637eca84847515b991f04); see
`../README.md`.

Output
------
`OUT.png`. In the right-hand panel, flat mid grey is "the same as around
it"; a ring, a checker or a tinted contour is what the restoration left.
"""
import sys, numpy as np
from PIL import Image
from meas import rgb, SIZE, MARGIN
def panel(path, pad=24):
    img=rgb(path); H,W,_=img.shape; x0=W-MARGIN-SIZE; y0=x0
    c=img[y0-pad:y0+SIZE+pad, x0-pad:x0+SIZE+pad]
    ring=np.concatenate([img[y0-36:y0-8, x0-36:x0+SIZE+36].reshape(-1,3), img[y0+SIZE+8:min(H,y0+SIZE+36), x0-36:x0+SIZE+36].reshape(-1,3)])
    mean=ring.mean(0)
    x4=np.repeat(np.repeat(c,4,0),4,1)
    amp=np.clip(128+20*(c-mean),0,255)
    a4=np.repeat(np.repeat(amp,4,0),4,1)
    return x4.astype(np.uint8), a4.astype(np.uint8)
out=sys.argv[1]; paths=sys.argv[2:]
rows=[]
for p in paths:
    a,b=panel(p); sep=np.full((a.shape[0],8,3),255,np.uint8)
    rows.append(np.concatenate([a,sep,b],1)); rows.append(np.full((8,rows[-1].shape[1],3),255,np.uint8))
Image.fromarray(np.concatenate(rows,0)).save(out)
