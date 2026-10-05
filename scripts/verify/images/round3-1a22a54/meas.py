#!/usr/bin/env python3
"""The restored V1 mark's faint band, measured independently of Wipemark.

What it is for
--------------
Written by the host verifier of the images series' third round (2026-10-05),
which verified `images/series-v3` at `1a22a54` for the coordinator of
`GigLaboCom/wipemark-app` (findings: Watchword FILE
`wipemark-task-images-followups-4-2026-10-05`, "What holds"). The claim it
checked is the one the CLI makes after a restoration: how far the restored
mark's faint edge still sits from the picture around it — per channel, in
colour (BT.601 Cb/Cr, T1) and as Delta E 2000 — so the product's own figures
(`restored.steps`, `step`, `chroma`) could be compared with a measurement
that shares no code with it. It gave "per channel -0.91...+1.21, chroma
<= 0.80, Delta E 2000 <= 0.29" on the 21 first-generation PNGs + good-alt,
chroma 2.41-2.88 at 4:4:4 q95 and 7.40-8.37 at 4:2:0 on the 2048 originals
(U1), and the per-pixel p95 of 10.6-11.5 levels on 4:4:4 q95 JPEGs against
1.9-3.1 on the PNGs that became the texture criterion (U2, D250).

What it does
------------
1. Reads the measured V1 alpha map's raw samples
   (`crates/wipemark-pixels/marks/measured/gemini-v1-96-measured.wma`,
   `WMA1`, 16-bit) as data — nothing else of Wipemark's is used.
2. Places the 96 px mark at a 64 px margin in the bottom-right corner (or at
   `x0, y0` when `measure` is called with them).
3. Takes the faint band — alpha in [3/255, 0.2] — and a ring 8-36 px
   outside the mark's square (Chebyshev distance).
4. Reports the band's mean minus the ring's mean in R, G, B and luma, the
   colour step `hypot(dCb, dCr)`, Delta E 2000 between the two means, the
   ring's largest per-channel standard deviation, and the 95th percentile
   and maximum of each band pixel's largest per-channel deviation from the
   ring's mean.

Usage
-----
    python3 meas.py RESTORED.png [MORE ...]

Also imported by `batch.py`, `meas2.py`, `naive.py` and `eye.py` beside it.
The map is read from this checkout (the repository root is four directories
above this file); `WIPEMARK_REPO=<checkout>` reads another one's (the
verifier read `a `wipemark-imgv3` worktree beside the repository`, the
`images/series-v3` worktree — the map is the same file).

Needs numpy and Pillow (a venv; the verifier's had Pillow 12.3.0 and numpy
2.5.3 — nothing in the repository depends on them). The pictures are the
owner's stickers, Watchword FILE `wipemark-gemini-stickers-2026-10-04`
(sha256 5a54435bbd600b43113ade474ad08f6f8b83dc10cdb637eca84847515b991f04),
restored by a release `wipemark-cli` (`cargo build --release -p
wipemark-cli`); see `../README.md`.

Output
------
One line per file: `R G B luma` are signed mean steps in levels (0 is "the
same as around it"), `chroma` the colour step in levels, `dE00` the
perceptual difference of the two means (under ~1 is not seen), `ringsd` how
busy the surroundings are, `pp95`/`ppmax` how rough the band is pixel by
pixel, `nband` the number of band pixels (fixed by the map).
"""
import struct, sys, json
import numpy as np
from PIL import Image

import os

REPO = os.environ.get("WIPEMARK_REPO") or os.path.abspath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", ".."))
WMA = REPO + "/crates/wipemark-pixels/marks/measured/gemini-v1-96-measured.wma"


def alpha_map():
    b = open(WMA, "rb").read()
    assert b[:4] == b"WMA1"
    w, h = struct.unpack("<HH", b[4:8])
    d = b[8]
    assert d == 16
    a = np.frombuffer(b[9:], dtype="<u2").astype(np.float64).reshape(h, w) / 65535.0
    return a


ALPHA = alpha_map()
SIZE = 96
MARGIN = 64


def rgb(path):
    im = Image.open(path)
    if im.mode != "RGB":
        im = im.convert("RGB")
    return np.asarray(im).astype(np.float64)


def srgb_to_lab(c):
    c = np.asarray(c, dtype=np.float64) / 255.0
    lin = np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)
    M = np.array([[0.4124564, 0.3575761, 0.1804375],
                  [0.2126729, 0.7151522, 0.0721750],
                  [0.0193339, 0.1191920, 0.9503041]])
    xyz = M @ lin
    xyz = xyz / np.array([0.95047, 1.0, 1.08883])
    f = np.where(xyz > (6 / 29) ** 3, np.cbrt(xyz), xyz / (3 * (6 / 29) ** 2) + 4 / 29)
    return np.array([116 * f[1] - 16, 500 * (f[0] - f[1]), 200 * (f[1] - f[2])])


def de2000(lab1, lab2):
    L1, a1, b1 = lab1
    L2, a2, b2 = lab2
    C1 = np.hypot(a1, b1); C2 = np.hypot(a2, b2)
    Cb = (C1 + C2) / 2
    G = 0.5 * (1 - np.sqrt(Cb ** 7 / (Cb ** 7 + 25 ** 7)))
    a1p = (1 + G) * a1; a2p = (1 + G) * a2
    C1p = np.hypot(a1p, b1); C2p = np.hypot(a2p, b2)
    h1p = np.degrees(np.arctan2(b1, a1p)) % 360
    h2p = np.degrees(np.arctan2(b2, a2p)) % 360
    dLp = L2 - L1
    dCp = C2p - C1p
    if C1p * C2p == 0:
        dhp = 0
    else:
        dhp = h2p - h1p
        if dhp > 180: dhp -= 360
        elif dhp < -180: dhp += 360
    dHp = 2 * np.sqrt(C1p * C2p) * np.sin(np.radians(dhp / 2))
    Lbp = (L1 + L2) / 2; Cbp = (C1p + C2p) / 2
    if C1p * C2p == 0:
        hbp = h1p + h2p
    elif abs(h1p - h2p) <= 180:
        hbp = (h1p + h2p) / 2
    elif h1p + h2p < 360:
        hbp = (h1p + h2p + 360) / 2
    else:
        hbp = (h1p + h2p - 360) / 2
    T = (1 - 0.17 * np.cos(np.radians(hbp - 30)) + 0.24 * np.cos(np.radians(2 * hbp))
         + 0.32 * np.cos(np.radians(3 * hbp + 6)) - 0.20 * np.cos(np.radians(4 * hbp - 63)))
    dtheta = 30 * np.exp(-(((hbp - 275) / 25) ** 2))
    Rc = 2 * np.sqrt(Cbp ** 7 / (Cbp ** 7 + 25 ** 7))
    Sl = 1 + 0.015 * (Lbp - 50) ** 2 / np.sqrt(20 + (Lbp - 50) ** 2)
    Sc = 1 + 0.045 * Cbp
    Sh = 1 + 0.015 * Cbp * T
    Rt = -np.sin(np.radians(2 * dtheta)) * Rc
    return float(np.sqrt((dLp / Sl) ** 2 + (dCp / Sc) ** 2 + (dHp / Sh) ** 2
                         + Rt * (dCp / Sc) * (dHp / Sh)))


def measure(path, x0=None, y0=None):
    img = rgb(path)
    H, W, _ = img.shape
    if x0 is None:
        x0 = W - MARGIN - SIZE
        y0 = H - MARGIN - SIZE
    band = (ALPHA >= 3 / 255) & (ALPHA <= 0.2)
    patch = img[y0:y0 + SIZE, x0:x0 + SIZE]
    bandpx = patch[band]
    yy, xx = np.mgrid[0:H, 0:W]
    dx = np.maximum(np.maximum(x0 - xx, xx - (x0 + SIZE - 1)), 0)
    dy = np.maximum(np.maximum(y0 - yy, yy - (y0 + SIZE - 1)), 0)
    d = np.maximum(dx, dy)
    ring = (d >= 8) & (d <= 36)
    ringpx = img[ring]
    mb = bandpx.mean(0); mr = ringpx.mean(0)
    st = mb - mr
    def ycc(p):
        r, g, b = p[..., 0], p[..., 1], p[..., 2]
        return np.stack([0.299 * r + 0.587 * g + 0.114 * b,
                         -0.168736 * r - 0.331264 * g + 0.5 * b,
                         0.5 * r - 0.418688 * g - 0.081312 * b], -1)
    yb = ycc(bandpx).mean(0); yr = ycc(ringpx).mean(0)
    dy_ = yb - yr
    ringsd = ringpx.std(0)
    # per-pixel: largest channel deviation of a band pixel from the ring mean
    pp = np.abs(bandpx - mr).max(1)
    return {
        "R": st[0], "G": st[1], "B": st[2], "luma": dy_[0],
        "chroma": float(np.hypot(dy_[1], dy_[2])),
        "ringsd": float(ringsd.max()),
        "dE00": de2000(srgb_to_lab(mb), srgb_to_lab(mr)),
        "pp95": float(np.percentile(pp, 95)), "ppmax": float(pp.max()),
        "nband": int(band.sum()),
    }


if __name__ == "__main__":
    for p in sys.argv[1:]:
        m = measure(p)
        print(p.split("/")[-1], " ".join(f"{k}={v:+.2f}" if isinstance(v, float) else f"{k}={v}" for k, v in m.items()))
