#!/usr/bin/env python3
"""The fifth round's two new fixtures, rebuilt from the stickers byte for byte.

What it is for
--------------
Written by the host verifier of the images series' fifth round (2026-10-05),
which verified `images/series-v3` at `4897d0b` (V1-V5: tests and docs only;
the round's report is `docs/plan/reports/images-followups-5-2026-10-05.md`)
for the coordinator of `GigLaboCom/wipemark-app`.
The round committed two fixtures cut from the owner's stickers —
`fixtures/image/gemini/fine-1040-q98-444.jpg` (`10_this_is_fine`, the
roughest q98 of the 22, the pin under `TEXTURE_LEVELS`, V3) and
`scroll-1040-q90.webp` (`04_tearing_scroll`, lossy WebP q90, V2) — and the
report says how they were made (cut to 1040 from 1008 so the codec's blocks
fall as in the 2048 file, D252; Pillow 12.3.0). This checks that sentence:
a fixture whose recipe does not reproduce it is a figure nobody can check.

What it does
------------
1. Prints the Pillow, libwebp and libjpeg versions.
2. Opens `stickers/10_this_is_fine.png` under `STICKERS_ROOT`, crops
   (1008, 1008, 2048, 2048), saves JPEG q98 4:4:4 in memory, and compares
   the bytes (sha256) with the committed fixture; if they differ, whether
   the decoded pixels are equal.
3. The same for `04_tearing_scroll.png` as lossy WebP q90, with and without
   `method=6`.
4. Prints the fixtures' modes.

Usage
-----
    python3 repro.py STICKERS_ROOT [FIXTURES_DIR]

`STICKERS_ROOT` is the directory the ZIP was unpacked into (it holds
`stickers/`). `FIXTURES_DIR` defaults to this checkout's
`fixtures/image/gemini/` (the verifier passed a scratch copy of it).

Needs numpy and Pillow 12.3.0 (a venv; another Pillow may bring another
libjpeg/libwebp and other bytes). The ZIP is Watchword FILE
`wipemark-gemini-stickers-2026-10-04` (sha256
5a54435bbd600b43113ade474ad08f6f8b83dc10cdb637eca84847515b991f04); see
`../README.md`.

Output
------
`identical` per fixture is the recipe confirmed. `DIFFERENT` with `pixel
equal True` is the same picture in different bytes (another encoder build);
`False` means the recipe in the report is not the one that made the file.
Re-run on 2026-10-05 at `1909fee` with Pillow 12.3.0: both fixtures
`identical`, the WebP with `method=6` only — the variant without it is
`DIFFERENT` by design, which is how the recipe was told apart.
"""
import hashlib, io, sys
from PIL import Image, features
import numpy as np
import os
Z = sys.argv[1]
F = sys.argv[2] if len(sys.argv) > 2 else os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "..", "fixtures", "image", "gemini")
print("Pillow", Image.__version__, "libwebp", features.version("webp"), "libjpeg", features.version("jpg"))
def h(b): return hashlib.sha256(b).hexdigest()
def crop(name):
    im = Image.open(f"{Z}/stickers/{name}")
    print(name, im.mode, im.size)
    return im.convert("RGB").crop((1008, 1008, 2048, 2048))
c = crop("10_this_is_fine.png")
b = io.BytesIO(); c.save(b, "JPEG", quality=98, subsampling=0); b = b.getvalue()
ref = open(f"{F}/fine-1040-q98-444.jpg","rb").read()
print("fine", len(b), h(b), "identical" if b == ref else "DIFFERENT")
if b != ref:
    a = np.asarray(Image.open(io.BytesIO(b))); r = np.asarray(Image.open(F+"/fine-1040-q98-444.jpg"))
    print(" pixel equal", (a==r).all())
c = crop("04_tearing_scroll.png")
for kw in [dict(lossless=False, quality=90, method=6), dict(lossless=False, quality=90)]:
    b = io.BytesIO(); c.save(b, "WEBP", **kw); b = b.getvalue()
    ref = open(f"{F}/scroll-1040-q90.webp","rb").read()
    print("scroll", kw, len(b), h(b), "identical" if b == ref else "DIFFERENT")
    if b != ref:
        a = np.asarray(Image.open(io.BytesIO(b))); r = np.asarray(Image.open(F+"/scroll-1040-q90.webp"))
        print(" pixel equal", (a==r).all())
print("ref webp mode", Image.open(F+"/scroll-1040-q90.webp").mode, "ref jpg", Image.open(F+"/fine-1040-q98-444.jpg").mode)
