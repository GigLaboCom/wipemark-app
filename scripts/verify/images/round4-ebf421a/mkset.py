#!/usr/bin/env python3
"""The fourth round's test set: each sticker as PNG and as six JPEG encodings.

What it is for
--------------
Written by the host verifier of the images series' fourth round (2026-10-05),
which verified `images/series-v3` at `ebf421a` (the D250-D253 texture
criterion, U2) for the coordinator of `GigLaboCom/wipemark-app`; findings:
Watchword FILE `wipemark-task-images-followups-5-2026-10-05`, "What holds".
The texture criterion's bounds (`TEXTURE_LEVELS` 5.5, `TEXTURE_RATIO` 2.0)
are claimed to separate what an eye finds (4:4:4 q97 and below) from what it
does not (q98, PNG). This builds the set that claim was measured on.

What it does
------------
Copies every PNG of `PNG_DIR` into `WORK/set/png/`, and saves each one with
Pillow as JPEG at q95, q97, q98 4:4:4 and q90, q95, q98 4:2:0 into
`WORK/set/q<Q>-<444|420>/<name>-q<Q>-<444|420>.jpg`.

Usage
-----
    python3 mkset.py PNG_DIR [WORK_DIR]

`PNG_DIR` is the 2048 originals: the ZIP's 21 `stickers/*.png`, plus
`good-alt/…mg2k….png` and `alt-anch/anchor-alternative.png` (the verifier's
`imgv6/set/png/`, 23 files; it called the first `Gemini_Generated_Image_mg2k_1.png`).
`WORK_DIR` defaults to the current directory (the verifier's was its
scratch `imgv7/`). The JPEG bytes depend on Pillow's libjpeg: 12.3.0 is
what the round's figures were measured with.

Needs numpy and Pillow (a venv; the verifier's had Pillow 12.3.0 — libjpeg
6.2 — and numpy 2.5.3; nothing in the repository depends on them).
Inputs are the owner's stickers, Watchword FILE
`wipemark-gemini-stickers-2026-10-04` (sha256
5a54435bbd600b43113ade474ad08f6f8b83dc10cdb637eca84847515b991f04); see
`../README.md`.

Output
------
The seven directories under `WORK/set/`, and `done`.
"""
import glob, os, sys
from PIL import Image
V = sys.argv[2] if len(sys.argv) > 2 else os.getcwd()
src = sorted(glob.glob(sys.argv[1] + "/*.png"))
os.makedirs(V + "/set/png", exist_ok=True)
for f in src:
    n = os.path.splitext(os.path.basename(f))[0]
    os.system(f'cp "{f}" "{V}/set/png/"')
    im = Image.open(f).convert("RGB")
    for q, ss in [(95,0),(97,0),(98,0),(90,2),(95,2),(98,2)]:
        tag = f"q{q}-{'444' if ss==0 else '420'}"
        d = V + "/set/" + tag; os.makedirs(d, exist_ok=True)
        im.save(f"{d}/{n}-{tag}.jpg", quality=q, subsampling=ss)
print("done")
