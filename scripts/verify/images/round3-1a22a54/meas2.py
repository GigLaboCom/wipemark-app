#!/usr/bin/env python3
"""Which "around" the product's colour step should be read against.

What it is for
--------------
Written by the host verifier of the images series' third round (2026-10-05,
`images/series-v3` at `1a22a54`; findings: Watchword FILE
`wipemark-task-images-followups-4-2026-10-05`). `meas.py` measures the band
against a ring 8-36 px out; the product measures it against something
nearer. This script measured the same band against three nearer
surroundings, to tell whether a gap between the product's `steps`/`chroma`
and `meas.py`'s came from the choice of ring rather than from a wrong
figure — the T1 review ("the spread over the same ring") in "What holds".

What it does
------------
For each restored picture, the faint band (alpha in [3/255, 0.2]) of the
96 px mark at a 64 px margin is compared with:

* `inner<floor` — the square's own pixels under alpha 0.002;
* `ring4` — a ring 1-4 px outside the square;
* `prodlike` — both together, which is what the product measures against.

For each: the number of pixels, the mean step in R, G, B, and the colour
step `hypot(dCb, dCr)` (BT.601).

Usage
-----
    python3 meas2.py RESTORED.png [MORE ...]

Needs numpy and Pillow (a venv; the verifier's had Pillow 12.3.0 and numpy
2.5.3 — nothing in the repository depends on them) and `meas.py` beside it,
which reads the V1 alpha map from this checkout (`WIPEMARK_REPO` to read
another). Inputs are the owner's stickers, Watchword FILE
`wipemark-gemini-stickers-2026-10-04` (sha256
5a54435bbd600b43113ade474ad08f6f8b83dc10cdb637eca84847515b991f04); see
`../README.md`.

Output
------
One line per file, three `|`-separated groups; a `prodlike` that matches the
product's `restored.steps`/`chroma` to a few hundredths means the product's
figure is right and any difference from `meas.py` is the ring.
"""
import sys, numpy as np
from meas import rgb, ALPHA, SIZE, MARGIN
def ycc(p):
    r,g,b=p[...,0],p[...,1],p[...,2]
    return np.stack([0.299*r+0.587*g+0.114*b,-0.168736*r-0.331264*g+0.5*b,0.5*r-0.418688*g-0.081312*b],-1)
for p in sys.argv[1:]:
    img=rgb(p); H,W,_=img.shape; x0=W-MARGIN-SIZE; y0=x0
    band=(ALPHA>=3/255)&(ALPHA<=0.2)
    patch=img[y0:y0+SIZE,x0:x0+SIZE]; bp=patch[band]
    inner=patch[ALPHA<0.002]
    big=img[y0-4:y0+SIZE+4,x0-4:x0+SIZE+4].copy(); m=np.ones(big.shape[:2],bool); m[4:-4,4:-4]=False
    ring4=big[m]
    out=[]
    for name,ar in [('inner<floor',inner),('ring4',ring4),('prodlike',np.concatenate([inner,ring4]))]:
        st=bp.mean(0)-ar.mean(0); d=ycc(bp).mean(0)-ycc(ar).mean(0)
        out.append(f"{name}: n={len(ar)} RGB=({st[0]:+.2f},{st[1]:+.2f},{st[2]:+.2f}) C={np.hypot(d[1],d[2]):.2f}")
    print(p.split('/')[-1], ' | '.join(out))
