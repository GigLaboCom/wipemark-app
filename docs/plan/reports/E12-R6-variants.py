#!/usr/bin/env python3
"""Subsampled JPEG variants of the committed Gemini crops, for E12-R6.

What it is for
--------------
Written by the agent implementing E12-R6 (the planar inverse,
`docs/plan/E12-R6-planar-inverse.md`), 2026-10-09, for the coordinator of
`GigLaboCom/wipemark-app`. The step asks for `max_alpha_dev_in_block`'s
distribution and the out-of-range split per variant (§6.4) and for a
measured `BLEND_LEVELS_C` (§4.3). The committed fixtures hold only two 4:2:0
qualities (95, 98) and no 4:2:2; D252's refusals are at 4:2:0 q90. This
makes those variants from the committed lossless crops, in the container,
without the owner's pictures (which never go into git and are the host's).

What it does
------------
For each `fixtures/image/gemini/<name>-1025.png` of NAMES (the mark at
865, a pixel off the 16-pixel grid), and the same padded by fifteen pixels
at the top and left, each the picture's own edge repeated (1040 x 1040, the
mark at 880 = 55 x 16, on the grid — where D252's 1040 crops of the 2048
originals put it; the padding is 865 pixels from the mark and reaches
nothing the visible pass reads), it saves with Pillow, by `mkset.py`'s
recipe (`im.save(path, quality=q, subsampling=s)`):

* 4:2:0 (subsampling=2) at quality 95, 90 and 85;
* 4:2:2 (subsampling=1) at quality 90;

into `OUT/<name>-<1025|1040>-q<Q>-<420|422>.jpg`, and prints each path.
(A 1024 cut is not used: no row names 1024, and the search does not find
the mark there — the known miss S11 holds in `golden/`.)
`crying` is the re-saved copy D244 names (`11_crying`); it is kept and
labelled, not mixed with the others in a figure.

Usage
-----
    python3 docs/plan/reports/E12-R6-variants.py OUT
    cargo run --release -p wipemark-picture --example planar_measure -- OUT/*.jpg

Needs numpy and Pillow (12.3.0 — libjpeg 6.2 — is what the E12-R6 figures
were made with; another libjpeg writes other bytes). Refuses another Pillow unless
`--any-pillow` is given.

Output
------
The JPEG files; nothing else. `planar_measure` prints one JSON line per
file: R0's and R6's verdict, out-of-range share and split, the restoration's
measures, the block deviation and the blend-back, and the share at chroma
allowances 1 to 16.
"""
import os
import sys

import numpy as np
import PIL
from PIL import Image

NAMES = ["anchor-green", "crying", "torch", "victory"]
VARIANTS = [(95, 2), (90, 2), (85, 2), (90, 1)]


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if len(args) != 1:
        sys.exit("usage: E12-R6-variants.py OUT [--any-pillow]")
    if PIL.__version__ != "12.3.0" and "--any-pillow" not in sys.argv:
        sys.exit(f"Pillow {PIL.__version__}: the figures were made with 12.3.0 (--any-pillow to go on)")
    out = args[0]
    os.makedirs(out, exist_ok=True)
    here = os.path.dirname(os.path.abspath(__file__))
    gemini = os.path.join(here, "..", "..", "..", "fixtures", "image", "gemini")
    for name in NAMES:
        im = Image.open(os.path.join(gemini, f"{name}-1025.png")).convert("RGB")
        padded = Image.fromarray(np.pad(np.asarray(im), ((15, 0), (15, 0), (0, 0)), mode="edge"))
        for side, cut in [(1025, im), (1040, padded)]:
            for q, ss in VARIANTS:
                tag = "420" if ss == 2 else "422"
                path = os.path.join(out, f"{name}-{side}-q{q}-{tag}.jpg")
                cut.save(path, quality=q, subsampling=ss)
                print(path)


if __name__ == "__main__":
    main()
