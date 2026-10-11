#!/usr/bin/env python3
"""Grok captures from X, checked against what their prompts asked (E12-R11).

What it is for, and who asked
-----------------------------
On 2026-10-11 the owner generated the first pictures in Grok in X from the
prompts `wipemark-grok-generation-prompts-2026-10-11` (Watchword): a small
scene in the upper left, the bottom-right corner — where Grok's mark lands —
left perfectly flat, at a chosen aspect ratio (16:9, 2:3, 4:3, 3:2). The
owner suspects that Grok in X cannot give 16:9 and 3:2. The coordinator's
task `wipemark-task-recon-grok-captures-check-2026-10-11` asks, per picture:
what the file is, which size and ratio X gave against the one asked, where
the mark is and whether the corner under it is flat enough to calibrate on,
which variant of the mark it carries, its opacity map read directly off a
flat black or white corner against the first look's (`first-look.py`,
≈0.36 at the strokes' core, 0.41 at most), and whether Grok followed the
prompt (the scene upper left, the corner empty) — with a sheet of the four
corners for the owner. This script is that check; it adds no rule of the
product and changes nothing in it.

No picture, crop or map it writes goes into git (D493): everything goes
under `--out`, by default `reports/grok-captures-check-<date>/`, which
`.gitignore` covers (`/reports/grok-*/`).

What it does, step by step — `check`
------------------------------------
1. **The file.** Per picture (byte-identical files are read once, the copies
   named): sha256, the format by its magic bytes, W×H, W/H and the nearest
   of 1:1, 16:9, 9:16, 4:3, 3:4, 3:2, 2:3 (nearest by |log(ratio)|, the miss
   in per cent). For a JPEG: baseline or progressive, the sampling (4:2:0 …),
   the number of scans, every quantisation table compared whole with the
   tables of the `--known` pictures (the first five, `fixtures/image/grok-x/`)
   and with libjpeg's standard tables at every quality 1–100 (the quality
   that reproduces both tables exactly, or none), and every block before the
   image data named — APP0 JFIF, APP1 EXIF or XMP, APP2 ICC or MPF, APP11
   JUMBF (C2PA when its label says `c2pa`), APP13 IPTC, APP14 Adobe, COM —
   with the bytes after the end-of-image marker counted. A PNG's or WebP's
   chunks are listed by name (`eXIf`, `iTXt`, `caBX`; `EXIF`, `XMP `, `ICCP`).
2. **The size against the one asked.** `--asked STEM=RATIO[,GROUP]` says
   which ratio (and background group) the prompt behind a picture asked for —
   read off the scene by a person, since X names a file by its media id;
   `--expect RATIO=WxH` says which size that ratio gave before (the prompts'
   own table). Per picture: asked, given, nearest ratio, match or not; then
   per ratio asked, the sizes X gave.
3. **The mark and the corner under it.** The template is the reference's
   (`--reference`, the first look's flat grey 1792×1008) depth over its
   rectangle, made by `first-look.py`'s own code (`reference_rect`,
   `ring_fit`). Per picture: the rectangle the first look's laws predict
   (width = the reference's width/W × W, the reference's margins right and
   bottom, 9 and 11 px) and the NCC of the template there; the rectangle a
   `--known` picture of the same size was found at (the first look's row for
   that size), if there is one, and the NCC there; then the mark found by
   `first-look.py`'s `locate` (every whole-pixel width, then 1/16 px) and its
   distance from both. The variant at the strokes' core: dark (logo 0) or
   light (logo 255), `first-look.py`'s `variant_fit`. The ring: `ring.py`'s
   `measure_ring` (8 px outside the mark's whole-pixel rectangle; per channel
   mean, σ and the quadratic's residual), its `looks_like` and its `verdict`
   under the group asked (or the group it looks like) — the very criterion
   `scripts/corpus/ring.py` applies before a file enters a set (σ under 8
   levels in every channel for a flat group). Beside the ring, the **corner**:
   the box from two mark-widths left and three mark-heights above the mark to
   the picture's edges, the mark grown by 3 px left out, on luma: mean, σ,
   the plane `a + b·x + c·y` (its slope in levels per 100 px and its drop
   across the box: a gradient), the quadratic's departure from that plane
   (its largest over the box: a vignette or any curvature), the quadratic's
   residual, and the share of pixels clipped at 0 or 255. A corner is called
   **flat** when ring.py's verdict passes and the drop and the curvature are
   both under `BEND` (2 levels).
4. **The opacity, read directly.** Where the ring looks black and the mark
   is light: `α = (I − B)/(255 − B)`; where it looks white and the mark is
   dark: `α = (B − I)/B` — `B` the ring's quadratic (`first-look.py`'s
   `ring_fit`), per channel and on luma, over the mark's rectangle. Its
   maximum, its 99th percentile, its median at the strokes' core (the
   reference map drawn there over 0.85 of its maximum) and per channel. The
   first look's map — the reference's α, cleaned as `first-look.py` cleans it
   — is drawn at the same place and width by the area integral, and compared:
   its own core median and maximum at that size, the NCC, the slope of the
   measured α on the drawn one through the origin (all of the support, and
   the core alone), the RMS after the slope. Every pair of pictures of one
   size with a flat corner is compared directly — `α` against `α` pixel for
   pixel (NCC, RMS, the largest difference, the two core medians) — and a
   pair of one black and one white is named as such: only that pair tells
   the two variants' shapes and opacities apart at one size.
5. **Did Grok follow the prompt.** Luma departing from the corner's level by
   more than `CONTENT` (12 levels), the mark grown by 4 px left out: the
   bounding box and the centroid as shares of W and H, the centroid's cell of
   a 3×3 grid (upper left …), and the pixels of it in the bottom-right
   quarter and in the lower half. The sheet `corners-x4.png`: per picture one
   row — the mark's rectangle grown by 24 px, the original | its luma's
   departure from the ring's quadratic ×10 around mid grey — at ×4, nearest;
   and `pictures.png`, every picture small with its scene's box (green) and
   the bottom-right quarter (magenta).

What it does — `selftest`
-------------------------
Synthesises everything into a temporary folder, nothing from the owner: a
wordmark-like shape (a slashed ring and five bars, α 0.36) drawn by 8 × 8
supersampling at every size, a dark one on flat grey at 1792×1008 for the
reference, light ones on black and dark ones on white at 784×1168, a
gradient under the corner, a disc in the corner, JPEGs from Pillow at
quality 95 and 80 with blocks inserted. Checks: the nearest ratio of the
sizes seen; libjpeg's quality read back (95, 80); EXIF, XMP and C2PA named;
the mark found within 0.3 px of where it was drawn; the variants; the black
and the white rings flat by ring.py, the gradient caught by the corner and
not by the ring; α at the core within 0.02 of 0.36 from black and from
white, and the black-and-white pair agreeing; the scene upper left and the
corner empty, and the disc in the corner seen; a duplicate read once.

How to run it
-------------
    python3 scripts/grok/captures-check.py check PICTURE [PICTURE …] \\
        [--reference fixtures/image/grok-x/HUVfLiSW0AEtmX_.jpeg] \\
        [--known fixtures/image/grok-x/*.jpeg] \\
        [--asked STEM=16:9,black …] [--expect 16:9=1792x1008 …] \\
        [--out DIR] [--date YYYY-MM-DD]
    python3 scripts/grok/captures-check.py selftest

from the repository's root (every path is an argument; run it from a
directory that is not the pictures' folder). `--known` defaults to the five
pictures in `fixtures/image/grok-x/`, `--reference` to the flat grey one,
`--expect` to the prompts' table (16:9 1792×1008, 2:3 784×1168, 4:3
1600×1200, 3:2 1728×1152).

What it needs
-------------
Python 3.11+ with numpy and Pillow 12.3.0 (the user site or a venv;
nothing in the repository depends on them). It imports `first-look.py`
(this folder) and `scripts/corpus/ring.py`. No Rust, no CLI, no network.

What its output means
---------------------
`check.md` — the tables of steps 1–5, every figure this run's; `check.json`
the same as data; `corners-x4.png` and `pictures.png` the sheets;
`alpha/<name>-alpha-Y.png` (16-bit, α · 65535) and `-x8.png` (×8, α 0.5
white) the maps step 4 read. The tables say what the files are and what the
corner gives; nothing here is a profile, a row or evidence that Grok is
supported. Exit status: 0 done, 1 the self-test failed, 2 a usage error or
a picture that cannot be read.
"""

import argparse
import datetime
import glob
import hashlib
import importlib.util
import json
import math
import os
import re
import struct
import sys
import tempfile

import numpy as np
from PIL import Image, ImageDraw

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
RATIOS = (("1:1", 1, 1), ("16:9", 16, 9), ("9:16", 9, 16), ("4:3", 4, 3), ("3:4", 3, 4), ("3:2", 3, 2), ("2:3", 2, 3))
EXPECT = ("16:9=1792x1008", "2:3=784x1168", "4:3=1600x1200", "3:2=1728x1152")  # the prompts' table
REFERENCE = os.path.join(REPO, "fixtures", "image", "grok-x", "HUVfLiSW0AEtmX_.jpeg")
KNOWN = os.path.join(REPO, "fixtures", "image", "grok-x")
BEND = 2.0  # a corner's plane drop or curvature at or over this many levels is a gradient or a vignette  [tunable]
CONTENT = 12.0  # luma this far from the corner's level is the scene  [tunable]
CROP = 24  # the sheet's margin around the mark  [tunable]
ZOOM = 4
AMPLIFY = 10


def fail(msg):
    print(f"captures-check: {msg}", file=sys.stderr)
    sys.exit(2)


# ── 1. the bytes ─────────────────────────────────────────────────────────────

ZIGZAG = (0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20, 13, 6, 7, 14,
          21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59, 52, 45, 38, 31, 39, 46, 53, 60,
          61, 54, 47, 55, 62, 63)
# ITU-T T.81 Annex K, natural order — libjpeg's jstd_luminance / chrominance tables
STD_LUMA = (16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40, 57, 69, 56, 14, 17, 22,
            29, 51, 87, 80, 62, 18, 22, 37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81, 104, 113, 92, 49, 64, 78, 87, 103,
            121, 120, 101, 72, 92, 95, 98, 112, 100, 103, 99)
STD_CHROMA = (17, 18, 24, 47, 99, 99, 99, 99, 18, 21, 26, 66, 99, 99, 99, 99, 24, 26, 56, 99, 99, 99, 99, 99, 47, 66, 99,
              99, 99, 99, 99, 99) + (99,) * 32


def libjpeg_table(base, q):
    """libjpeg's jpeg_set_quality: the table stored for quality q, zigzag order, force_baseline."""
    scale = 5000 // q if q < 50 else 200 - 2 * q
    return [min(255, max(1, (base[n] * scale + 50) // 100)) for n in ZIGZAG]


def libjpeg_quality(tables):
    """The libjpeg quality whose standard tables are exactly `tables` (table 0 luma, 1 chroma), or None."""
    t0, t1 = tables.get(0), tables.get(1)
    for q in range(1, 101):
        if t0 == libjpeg_table(STD_LUMA, q) and (t1 is None or t1 == libjpeg_table(STD_CHROMA, q)):
            return q
    return None


def app_name(m, seg):
    n = m - 0xE0
    if n == 0 and seg.startswith(b"JFIF\0"):
        return "APP0 JFIF"
    if n == 0 and seg.startswith(b"JFXX\0"):
        return "APP0 JFXX"
    if n == 1 and seg.startswith(b"Exif\0"):
        return "APP1 EXIF"
    if n == 1 and seg.startswith(b"http://ns.adobe.com/xap/1.0/\0"):
        return "APP1 XMP"
    if n == 1 and seg.startswith(b"http://ns.adobe.com/xmp/extension/\0"):
        return "APP1 XMP extension"
    if n == 2 and seg.startswith(b"ICC_PROFILE\0"):
        return "APP2 ICC"
    if n == 2 and seg.startswith(b"MPF\0"):
        return "APP2 MPF"
    if n == 11 and seg.startswith(b"JP"):
        return "APP11 JUMBF (C2PA)" if b"c2pa" in seg else "APP11 JUMBF"
    if n == 13 and seg.startswith(b"Photoshop 3.0\0"):
        return "APP13 IPTC"
    if n == 14 and seg.startswith(b"Adobe"):
        return "APP14 Adobe"
    return f"APP{n}"


def jpeg_blocks(b):
    """Every marker of a JPEG, scans included: the blocks named, the tables
    (id → 64 values, zigzag as stored), the frame, the scans, the tail."""
    if not b.startswith(b"\xff\xd8"):
        return None
    out = {"blocks": [], "tables": {}, "sof": None, "sampling": None, "scans": 0, "dht": 0, "tail": 0, "eoi": False}
    i = 2
    while i + 4 <= len(b):
        if b[i] != 0xFF:
            return out  # damaged; what was read stands
        m = b[i + 1]
        if m == 0xFF:
            i += 1
            continue
        if m == 0xD9:
            out["eoi"] = True
            out["tail"] = len(b) - (i + 2)
            break
        if m in (0x01,) or 0xD0 <= m <= 0xD7:
            i += 2
            continue
        n = struct.unpack(">H", b[i + 2:i + 4])[0]
        seg = b[i + 4:i + 2 + n]
        if 0xE0 <= m <= 0xEF:
            out["blocks"].append(app_name(m, seg))
        elif m == 0xFE:
            out["blocks"].append(f"COM ({len(seg)} bytes)")
        elif m == 0xDB:
            j = 0
            while j < len(seg):
                pq, tq = seg[j] >> 4, seg[j] & 15
                size = 128 if pq else 64
                vals = seg[j + 1:j + 1 + size]
                if pq:
                    vals = [struct.unpack(">H", vals[k:k + 2])[0] for k in range(0, size, 2)]
                out["tables"][tq] = list(vals)
                j += 1 + size
        elif m == 0xC4:
            out["dht"] += 1
        elif 0xC0 <= m <= 0xCF and m not in (0xC4, 0xC8, 0xCC):
            out["sof"] = {0xC0: "baseline", 0xC1: "extended", 0xC2: "progressive"}.get(m, f"SOF{m - 0xC0}")
            nc = seg[5]
            out["sampling"] = [(seg[6 + 3 * k + 1] >> 4, seg[6 + 3 * k + 1] & 15) for k in range(nc)]
        elif m == 0xDA:
            out["scans"] += 1
            i += 2 + n
            # entropy-coded data up to the next marker that is not a stuffed byte or a restart
            while i + 1 < len(b) and not (b[i] == 0xFF and b[i + 1] != 0 and not 0xD0 <= b[i + 1] <= 0xD7):
                i += 1
            continue
        i += 2 + n
    s = out["sampling"]
    if s and len(s) == 3 and all(c == (1, 1) for c in s[1:]):
        out["subsampling"] = {(1, 1): "4:4:4", (2, 1): "4:2:2", (2, 2): "4:2:0", (1, 2): "4:4:0"}.get(s[0], str(s[0]))
    else:
        out["subsampling"] = str(s)
    out["quality"] = libjpeg_quality(out["tables"])
    return out


def chunks(b):
    """A PNG's or a WebP's chunk names, in order (repeats folded)."""
    names = []
    if b.startswith(b"\x89PNG\r\n\x1a\n"):
        i = 8
        while i + 8 <= len(b):
            n = struct.unpack(">I", b[i:i + 4])[0]
            names.append(b[i + 4:i + 8].decode("latin-1"))
            i += 12 + n
    elif b[:4] == b"RIFF" and b[8:12] == b"WEBP":
        i = 12
        while i + 8 <= len(b):
            n = struct.unpack("<I", b[i + 4:i + 8])[0]
            names.append(b[i:i + 4].decode("latin-1"))
            i += 8 + n + (n & 1)
    folded = []
    for nm in names:
        if folded and folded[-1][0] == nm:
            folded[-1][1] += 1
        else:
            folded.append([nm, 1])
    return [nm if k == 1 else f"{nm} ×{k}" for nm, k in folded]


def format_of(b):
    if b.startswith(b"\xff\xd8\xff"):
        return "JPEG"
    if b.startswith(b"\x89PNG\r\n\x1a\n"):
        return "PNG"
    if b[:4] == b"RIFF" and b[8:12] == b"WEBP":
        return "WebP"
    return "unknown"


def nearest_ratio(W, H):
    r = W / H
    name, a, c = min(RATIOS, key=lambda t: abs(math.log(r / (t[1] / t[2]))))
    return name, (r / (a / c) - 1.0) * 100.0


def stem(path):
    """The file's name without its extension and a browser's ' (1)' suffix."""
    s = os.path.splitext(os.path.basename(path))[0]
    return re.sub(r" \(\d+\)$", "", s)


def read(paths):
    """Each distinct file once: {sha: picture}, in the order given; copies named."""
    seen, out = {}, []
    for p in paths:
        try:
            b = open(p, "rb").read()
        except OSError as e:
            fail(f"{p}: {e}")
        sha = hashlib.sha256(b).hexdigest()
        if sha in seen:
            seen[sha]["copies"].append(p)
            continue
        try:
            with Image.open(p) as im:
                img = np.asarray(im.convert("RGB")).astype(np.float64)
        except OSError as e:
            fail(f"{p}: {e}")
        H, W, _ = img.shape
        pic = {"path": p, "name": stem(p), "sha256": sha, "bytes": len(b), "copies": [], "format": format_of(b),
               "size": [W, H], "img": img}
        pic["ratio"] = W / H
        pic["nearest"], pic["miss_pct"] = nearest_ratio(W, H)
        pic["jpeg"] = jpeg_blocks(b)
        pic["chunks"] = chunks(b) if pic["jpeg"] is None else None
        seen[sha] = pic
        out.append(pic)
    return out


def metadata_words(pic):
    j = pic["jpeg"]
    blocks = j["blocks"] if j else pic["chunks"]
    flags = {"EXIF": any("EXIF" in x or x.startswith("eXIf") for x in blocks),
             "XMP": any("XMP" in x for x in blocks) or any(x.startswith("iTXt") for x in blocks),
             "C2PA": any("C2PA" in x or x.startswith("caBX") for x in blocks)}
    return blocks, flags


# ── 3. the mark and the corner ───────────────────────────────────────────────


def whole(rect):
    x, y, w, h = rect
    x0, y0 = int(math.floor(x)), int(math.floor(y))
    return (x0, y0, int(math.ceil(x + w)) - x0, int(math.ceil(y + h)) - y0)


def corner_box(img, rect):
    """Luma over the corner — two mark-widths left and three mark-heights above
    the mark to the edges — the mark grown by 3 px left out."""
    H, W, _ = img.shape
    x, y, w, h = whole(rect)
    X0, Y0 = max(0, x - 2 * w), max(0, y - 3 * h)
    Y = img[Y0:, X0:] @ LUMA
    yy, xx = np.mgrid[Y0:H, X0:W]
    keep = ~((xx >= x - 3) & (xx < x + w + 3) & (yy >= y - 3) & (yy < y + h + 3))
    return X0, Y0, Y, xx, yy, keep


def corner_shape(img, rect):
    X0, Y0, Y, xx, yy, keep = corner_box(img, rect)
    v = Y[keep]
    bw, bh = Y.shape[1], Y.shape[0]
    u = (xx[keep] - X0) / 100.0
    t = (yy[keep] - Y0) / 100.0
    P = np.stack([np.ones_like(u), u, t], -1)
    kp, *_ = np.linalg.lstsq(P, v, rcond=None)
    Q = np.stack([np.ones_like(u), u, t, u * u, u * t, t * t], -1)
    kq, *_ = np.linalg.lstsq(Q, v, rcond=None)
    # over the whole box: the plane's drop, the quadratic's departure from the plane
    uu = (xx - X0) / 100.0
    tt = (yy - Y0) / 100.0
    plane = kp[0] + kp[1] * uu + kp[2] * tt
    quad = kq[0] + kq[1] * uu + kq[2] * tt + kq[3] * uu * uu + kq[4] * uu * tt + kq[5] * tt * tt
    rgb = img[Y0:, X0:][keep]
    return {
        "box": [int(X0), int(Y0), int(bw), int(bh)], "mean": float(v.mean()), "sd": float(v.std()),
        "slope_x": float(kp[1]), "slope_y": float(kp[2]), "drop": float(plane.max() - plane.min()),
        "curvature": float(np.abs(quad - plane).max()), "residual": float(np.sqrt(((v - Q @ kq) ** 2).mean())),
        "clipped_0": float((rgb.max(-1) <= 0.5).mean()), "clipped_255": float((rgb.min(-1) >= 254.5).mean()),
    }


def ring_check(img, rect, group):
    r = whole(rect)
    m = ring.measure_ring(img, r, ring.RING)
    if m is None:
        return None
    seen = ring.looks_like(m)
    g = group if group in ring.GROUPS else (seen if seen in ring.GROUPS else "texture")
    ok, move_to, notes = ring.verdict(g, m)
    return {"rect": list(r), "group": g, "mean": m["mean"], "sd": m["sd"], "residual": m["residual"],
            "looks_like": seen, "pass": ok, "move_to": move_to, "notes": notes}


def place(pic, t, ref, known):
    """The law's rectangle, the first look's row for this size, the mark found."""
    img = pic["img"]
    W, H = pic["size"]
    Y = img @ LUMA
    k = ref["rect"][2] / ref["size"][0]
    w = k * W
    h = w * t.shape[0] / t.shape[1]
    law = {"x": W - ref["margins"][0] - w, "y": H - ref["margins"][1] - h, "w": w, "h": h}
    law["ncc"] = fl.ncc_at(Y, t, law["x"], law["y"], law["w"])
    row = None
    for kp in known:
        if kp["size"] == pic["size"] and kp.get("loc"):
            kl = kp["loc"]
            row = {"from": kp["name"], "x": kl["x"], "y": kl["y"], "w": kl["w"], "h": kl["h"]}
            row["ncc"] = fl.ncc_at(Y, t, row["x"], row["y"], row["w"])
            break
    loc = fl.locate(img, t, fl.LOCATE, ref["size"], fl.SCALES)
    loc["margin_right"] = W - (loc["x"] + loc["w"])
    loc["margin_bottom"] = H - (loc["y"] + loc["h"])
    loc["w_over_W"] = loc["w"] / W
    for other in (law, row):
        if other:
            other["dx"], other["dy"], other["dw"] = loc["x"] - other["x"], loc["y"] - other["y"], loc["w"] - other["w"]
    return law, row, loc


def variant_of(pic, t):
    loc = pic["loc"]
    rect = (loc["x"], loc["y"], loc["w"], loc["h"])
    fit = fl.ring_fit(pic["img"], rect)
    fox, foy = fit["origin"]
    dox, doy, dt = fl.draw(t, loc["x"], loc["y"], loc["w"])
    core = np.zeros(fit["crop"].shape[:2], bool)
    sub = (slice(doy - foy, doy - foy + dt.shape[0]), slice(dox - fox, dox - fox + dt.shape[1]))
    core[sub] = dt > fl.CORE * dt.max()
    vf = fl.variant_fit(fit["crop"][core], fit["B"][core])
    return vf, fit, sub


# ── 4. α read directly ───────────────────────────────────────────────────────


def alpha_direct(pic, amap, ref_alpha):
    """α over the mark's rectangle where the corner is flat black (light mark)
    or flat white (dark mark); None elsewhere."""
    rc = pic["ring"]
    if rc is None or not pic["flat"]:
        return None
    seen, variant = rc["looks_like"], pic["variant"]["variant"]
    if not ((seen == "black" and variant == "light") or (seen == "white" and variant == "dark")):
        return None
    fit, sub = pic["_fit"], pic["_sub"]
    I = fit["crop"][sub]
    B = fit["B"][sub]
    if variant == "light":
        a = (I - B) / np.maximum(255.0 - B, 1.0)
        how = "(I − B)/(255 − B), black corner, light mark"
    else:
        a = (B - I) / np.maximum(B, 1.0)
        how = "(B − I)/B, white corner, dark mark"
    aY = a @ LUMA
    loc = pic["loc"]
    _, _, drawn = fl.draw(amap, loc["x"], loc["y"], loc["w"])
    hh, ww = min(aY.shape[0], drawn.shape[0]), min(aY.shape[1], drawn.shape[1])
    aY, drawn, a = aY[:hh, :ww], drawn[:hh, :ww], a[:hh, :ww]
    core = drawn > fl.CORE * drawn.max()
    sup = drawn > 0.02
    slope = float((aY[sup] * drawn[sup]).sum() / (drawn[sup] ** 2).sum())
    slope_core = float((aY[core] * drawn[core]).sum() / (drawn[core] ** 2).sum())
    return {
        "how": how, "map": aY, "core": core,
        "max": float(aY.max()), "p99": float(np.quantile(aY[sup], 0.99)),
        "core_median": float(np.median(aY[core])), "core_px": int(core.sum()),
        "core_median_rgb": [float(np.median(a[..., c][core])) for c in range(3)],
        "ref_drawn_core_median": float(np.median(drawn[core])), "ref_drawn_max": float(drawn.max()),
        "ref_core_median": ref_alpha["core_median"], "ref_max": ref_alpha["max"],
        "ncc": fl.ncc(aY, drawn), "slope": slope, "slope_core": slope_core,
        "rms": float(np.sqrt(((aY - slope * drawn)[sup] ** 2).mean())),
        "clipped_0_in_mark": float((fit["crop"][sub][:hh, :ww].max(-1) <= 0.5)[sup].mean()),
    }


def pairs(pics):
    """Every two pictures of one size with α read directly, compared pixel for pixel."""
    out = []
    by_size = {}
    for p in pics:
        if p.get("alpha"):
            by_size.setdefault(tuple(p["size"]), []).append(p)
    for size, group in sorted(by_size.items()):
        for i in range(len(group)):
            for j in range(i + 1, len(group)):
                a, b = group[i], group[j]
                la, lb = a["loc"], b["loc"]
                ma, mb = a["alpha"]["map"], b["alpha"]["map"]
                # the two rectangles are whole-pixel crops from floor(x), floor(y); align them
                ox = int(math.floor(lb["x"])) - int(math.floor(la["x"]))
                oy = int(math.floor(lb["y"])) - int(math.floor(la["y"]))
                ya0, xa0 = max(0, oy), max(0, ox)
                yb0, xb0 = max(0, -oy), max(0, -ox)
                hh = min(ma.shape[0] - ya0, mb.shape[0] - yb0)
                ww = min(ma.shape[1] - xa0, mb.shape[1] - xb0)
                A = ma[ya0:ya0 + hh, xa0:xa0 + ww]
                Bm = mb[yb0:yb0 + hh, xb0:xb0 + ww]
                sup = (A > 0.02) | (Bm > 0.02)
                kinds = sorted({a["ring"]["looks_like"], b["ring"]["looks_like"]})
                out.append({
                    "size": list(size), "a": a["name"], "b": b["name"], "kinds": kinds,
                    "black_and_white": kinds == ["black", "white"],
                    "offset": [ox, oy], "ncc": fl.ncc(A, Bm), "rms": float(np.sqrt(((A - Bm)[sup] ** 2).mean())),
                    "max_abs": float(np.abs(A - Bm).max()),
                    "core_a": a["alpha"]["core_median"], "core_b": b["alpha"]["core_median"],
                    "max_a": a["alpha"]["max"], "max_b": b["alpha"]["max"],
                })
    return out


# ── 5. the scene ─────────────────────────────────────────────────────────────

CELLS = (("upper left", "top centre", "upper right"), ("middle left", "centre", "middle right"),
         ("lower left", "bottom centre", "lower right"))


def scene(pic):
    img = pic["img"]
    H, W, _ = img.shape
    Y = img @ LUMA
    level = float(np.mean(pic["ring"]["mean"]) if pic["ring"] else np.median(Y))
    x, y, w, h = whole((pic["loc"]["x"], pic["loc"]["y"], pic["loc"]["w"], pic["loc"]["h"]))
    m = np.abs(Y - level) > CONTENT
    m[max(0, y - 4):y + h + 4, max(0, x - 4):x + w + 4] = False
    n = int(m.sum())
    if n == 0:
        return {"level": level, "pixels": 0}
    ys, xs = np.nonzero(m)
    cx, cy = float(xs.mean()) / W, float(ys.mean()) / H
    return {
        "level": level, "pixels": n, "share": n / (W * H),
        "box": [float(xs.min()) / W, float(ys.min()) / H, float(xs.max() + 1) / W, float(ys.max() + 1) / H],
        "centroid": [cx, cy], "cell": CELLS[min(2, int(cy * 3))][min(2, int(cx * 3))],
        "bottom_right_quarter": int(m[H // 2:, W // 2:].sum()), "lower_half": int(m[H // 2:, :].sum()),
        "box_px": [int(xs.min()), int(ys.min()), int(xs.max() + 1), int(ys.max() + 1)],
    }


# ── sheets ───────────────────────────────────────────────────────────────────


def corner_row(pic):
    img = pic["img"]
    H, W, _ = img.shape
    loc = pic["loc"]
    rect = (loc["x"], loc["y"], loc["w"], loc["h"])
    x, y, w, h = whole(rect)
    X0, Y0 = max(0, x - CROP), max(0, y - CROP)
    X1, Y1 = min(W, x + w + CROP), min(H, y + h + CROP)
    fit = fl.ring_fit(img, rect)
    ox, oy = fit["origin"]
    dev = np.full(img.shape[:2], 128.0)
    d = ((fit["crop"] - fit["B"]) @ LUMA) * AMPLIFY + 128.0
    dev[oy:oy + d.shape[0], ox:ox + d.shape[1]] = d
    left = img[Y0:Y1, X0:X1]
    right = np.repeat(np.clip(dev[Y0:Y1, X0:X1], 0, 255)[..., None], 3, -1)
    gap = np.full((Y1 - Y0, 2, 3), 255.0)
    row = np.concatenate([left, gap, right], 1)
    im = Image.fromarray(np.clip(np.round(row), 0, 255).astype(np.uint8))
    return im.resize((im.width * ZOOM, im.height * ZOOM), Image.NEAREST)


def sheet(rows, labels, path, bg=(40, 40, 40)):
    pad, line = 8, 18
    W = max(r.width for r in rows) + 2 * pad
    H = sum(r.height + line + pad for r in rows) + pad
    out = Image.new("RGB", (W, H), bg)
    d = ImageDraw.Draw(out)
    y = pad
    for r, lab in zip(rows, labels):
        d.text((pad, y), lab, fill=(235, 235, 235))
        y += line
        out.paste(r, (pad, y))
        y += r.height + pad
    out.save(path)


def thumbs(pics, path, height=360):
    tiles = []
    for p in pics:
        im = Image.fromarray(p["img"].astype(np.uint8))
        W, H = im.size
        s = height / H
        t = im.resize((max(1, round(W * s)), height), Image.BILINEAR)
        d = ImageDraw.Draw(t)
        d.rectangle([round(W / 2 * s), round(H / 2 * s), t.width - 1, height - 1], outline=(255, 0, 255))
        sc = p.get("scene") or {}
        if sc.get("pixels"):
            x0, y0, x1, y1 = sc["box_px"]
            d.rectangle([round(x0 * s), round(y0 * s), round(x1 * s), round(y1 * s)], outline=(0, 255, 0))
        tiles.append((t, f"{p['name']} {W}x{H}"))
    sheet_w = sum(t.width for t, _ in tiles) + 8 * (len(tiles) + 1)
    out = Image.new("RGB", (sheet_w, height + 34), (40, 40, 40))
    d = ImageDraw.Draw(out)
    x = 8
    for t, lab in tiles:
        d.text((x, 6), lab, fill=(235, 235, 235))
        out.paste(t, (x, 26))
        x += t.width + 8
    out.save(path)


# ── check ────────────────────────────────────────────────────────────────────


def reference_of(path):
    b = open(path, "rb").read()
    with Image.open(path) as im:
        img = np.asarray(im.convert("RGB")).astype(np.float64)
    H, W, _ = img.shape
    rect, thr = fl.reference_rect(img, fl.CORNER, fl.PAD)
    rx, ry, rw, rh = rect
    rfit = fl.ring_fit(img, rect)
    ox, oy = rfit["origin"]
    sl = (slice(ry - oy, ry - oy + rh), slice(rx - ox, rx - ox + rw))
    I, B = rfit["crop"][sl], rfit["B"][sl]
    depth = (B - I) @ LUMA
    alpha = 1.0 - (I @ LUMA) / np.maximum(B @ LUMA, 1.0)
    noise = 1.0 - (rfit["crop"] @ LUMA) / np.maximum(rfit["B"] @ LUMA, 1.0)
    floor = max(3.0 * float(noise[rfit["ring"]].std()), 0.01)
    amap = fl.cleaned_map(alpha, floor)
    core = depth > fl.CORE * depth.max()
    return {"path": path, "name": stem(path), "sha256": hashlib.sha256(b).hexdigest(), "size": [W, H],
            "rect": [rx, ry, rw, rh], "margins": [W - (rx + rw), H - (ry + rh)], "template": depth, "amap": amap,
            "alpha": {"core_median": float(np.median(alpha[core])), "max": float(alpha.max())}, "threshold": thr}


def parse_asked(items):
    asked = {}
    for it in items or []:
        if "=" not in it:
            fail(f"--asked {it!r}: STEM=RATIO[,GROUP]")
        k, v = it.split("=", 1)
        ratio, _, group = v.partition(",")
        ratio, _, orient = ratio.partition("/")
        if orient not in ("", "land", "port"):
            fail(f"--asked {it!r}: the orientation is land or port")
        asked[k] = {"ratio": ratio, "orient": orient or None, "group": group or None}
    return asked


def parse_expect(items):
    out = {}
    for it in items:
        k, _, v = it.partition("=")
        w, _, h = v.lower().partition("x")
        out[k] = [int(w), int(h)]
    return out


def analyse(pics, ref, known, asked, expect):
    t = ref["template"]
    for p in known:
        p["loc"] = fl.locate(p["img"], t, fl.LOCATE, ref["size"], fl.SCALES)
        if p["sha256"] == ref["sha256"]:
            rx, ry, rw, rh = ref["rect"]
            p["loc"] = {"x": float(rx), "y": float(ry), "w": float(rw), "h": float(rh), "ncc": 1.0, "sign": -1.0}
        W, H = p["size"]
        p["loc"]["margin_right"] = W - (p["loc"]["x"] + p["loc"]["w"])
        p["loc"]["margin_bottom"] = H - (p["loc"]["y"] + p["loc"]["h"])
        p["loc"]["w_over_W"] = p["loc"]["w"] / W
    for p in pics:
        a = asked.get(p["name"]) or {}
        p["asked"] = a.get("ratio")
        p["orient_asked"] = a.get("orient")
        W, H = p["size"]
        p["orient"] = "land" if W > H else ("port" if W < H else "square")
        p["asked_size"] = expect.get(a.get("ratio")) if a.get("ratio") else None
        p["group_asked"] = a.get("group")
        p["law"], p["row"], p["loc"] = place(p, t, ref, known)
        vf, fit, sub = variant_of(p, t)
        p["variant"], p["_fit"], p["_sub"] = vf, fit, sub
        rect = (p["loc"]["x"], p["loc"]["y"], p["loc"]["w"], p["loc"]["h"])
        p["ring"] = ring_check(p["img"], rect, p["group_asked"])
        p["corner"] = corner_shape(p["img"], rect)
        p["flat"] = bool(p["ring"] and p["ring"]["pass"] and p["ring"]["looks_like"] in ring.FLAT_GROUPS
                         and p["corner"]["drop"] < BEND and p["corner"]["curvature"] < BEND)
        p["alpha"] = alpha_direct(p, ref["amap"], ref["alpha"])
        p["scene"] = scene(p)
    return pairs(pics)


def laws(rows):
    """width = k·W and width = a + b·W; margins = a + b·W; over every picture located."""
    W = np.array([r["size"][0] for r in rows], float)
    w = np.array([r["loc"]["w"] for r in rows], float)
    out = {"n": len(rows)}
    k = float((w * W).sum() / (W * W).sum())
    out["width_k"] = k
    out["width_k_worst_px"] = float(np.abs(w - k * W).max())
    A = np.stack([np.ones_like(W), W], -1)
    for key, v in (("width", w), ("margin_right", np.array([r["loc"]["margin_right"] for r in rows])),
                   ("margin_bottom", np.array([r["loc"]["margin_bottom"] for r in rows]))):
        c, *_ = np.linalg.lstsq(A, v, rcond=None)
        out[key + "_affine"] = [float(c[0]), float(c[1])]
        out[key + "_affine_worst_px"] = float(np.abs(v - A @ c).max())
        if key != "width":
            out[key + "_range"] = [float(v.min()), float(v.max())]
    return out


def check(args):
    paths = []
    for p in args.pictures:
        paths.extend(sorted(glob.glob(p)) if any(ch in p for ch in "*?[") else [p])
    known_paths = []
    for p in args.known:
        known_paths.extend(sorted(glob.glob(os.path.join(p, "*"))) if os.path.isdir(p) else [p])
    known_paths = [p for p in known_paths if os.path.isfile(p) and not os.path.basename(p).startswith(".")]
    pics = read([p for p in paths if not os.path.basename(p).startswith(".")])
    known = read(known_paths)
    ref = reference_of(args.reference)
    asked = parse_asked(args.asked)
    expect = parse_expect(args.expect)
    pr = analyse(pics, ref, known, asked, expect)
    known_tables = {}
    for k in known:
        if k["jpeg"]:
            known_tables.setdefault(json.dumps(k["jpeg"]["tables"], sort_keys=True), []).append(k["name"])
    for p in pics:
        if p["jpeg"]:
            p["tables_as_known"] = known_tables.get(json.dumps(p["jpeg"]["tables"], sort_keys=True), [])
    out = args.out
    os.makedirs(os.path.join(out, "alpha"), exist_ok=True)
    for p in pics:
        if p["alpha"]:
            fl.png16(os.path.join(out, "alpha", f"{p['name']}-alpha-Y.png"), np.clip(p["alpha"]["map"], 0, 1))
            fl.png_eye(os.path.join(out, "alpha", f"{p['name']}-alpha-Y-x8.png"), np.clip(p["alpha"]["map"], 0, 1))
    rows = [corner_row(p) for p in pics]
    labels = [f"{p['name']}  {p['size'][0]}x{p['size'][1]}  asked {p['asked'] or '?'}"
              f"{'/' + p['orient_asked'] if p.get('orient_asked') else ''}  "
              f"{p['variant']['variant']} mark  ring {p['ring']['looks_like'] if p['ring'] else '-'}  "
              f"| left: original  right: luma - ring quadratic, x{AMPLIFY} around 128  (x{ZOOM})" for p in pics]
    sheet(rows, labels, os.path.join(out, "corners-x4.png"))
    thumbs(pics, os.path.join(out, "pictures.png"))
    lw = laws(known + pics)
    summary = {"date": args.date, "reference": {k: v for k, v in ref.items() if k not in ("template", "amap")},
               "expect": expect, "laws": lw, "pairs": pr,
               "known": [{"name": k["name"], "size": k["size"], "sha256": k["sha256"], "loc": k["loc"],
                          "jpeg": {"quality": k["jpeg"]["quality"], "subsampling": k["jpeg"]["subsampling"]}
                          if k["jpeg"] else None} for k in known],
               "pictures": [strip_private(p) for p in pics]}
    with open(os.path.join(out, "check.json"), "w") as f:
        json.dump(summary, f, indent=1, default=lambda o: o.tolist() if hasattr(o, "tolist") else str(o))
    write_md(os.path.join(out, "check.md"), summary)
    print(f"captures-check: wrote {out}/check.md, check.json, corners-x4.png, pictures.png, alpha/")


def strip_private(p):
    d = {k: v for k, v in p.items() if k not in ("img", "_fit", "_sub")}
    if d.get("alpha"):
        d["alpha"] = {k: v for k, v in d["alpha"].items() if k not in ("map", "core")}
    return d


def f2(v, n=2):
    return "—" if v is None else (", ".join(f2(x, n) for x in v) if isinstance(v, (list, tuple)) else f"{v:.{n}f}")


def write_md(path, s):
    L = [f"# Grok captures from X, checked — {s['date']}\n",
         "Generated by `scripts/grok/captures-check.py check`. Every figure below is this run's.\n",
         "## 1–2. The files, and the size against the one asked\n",
         "| picture | sha256 | bytes | format | W×H | W/H | nearest | asked | asked size | size | orientation asked, given | "
         "JPEG | sampling | scans | libjpeg q | tables as the first five | blocks | EXIF / XMP / C2PA | after EOI | copies |",
         "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"]
    for p in s["pictures"]:
        j = p["jpeg"] or {}
        blocks, flags = metadata_words(p)
        size = f"{p['size'][0]}×{p['size'][1]}"
        asz = f"{p['asked_size'][0]}×{p['asked_size'][1]}" if p.get("asked_size") else "—"
        match = "—" if not p.get("asked_size") else ("yes" if p["asked_size"] == p["size"] else
                                                     ("ratio only" if p["nearest"] == p["asked"] else "**no**"))
        L.append(f"| `{p['name']}` | `{p['sha256'][:16]}…` | {p['bytes']} | {p['format']} | {size} | {p['ratio']:.4f} | "
                 f"{p['nearest']} ({p['miss_pct']:+.1f} %) | {p['asked'] or '—'} | {asz} | {match} | "
                 f"{p.get('orient_asked') or '—'}, {p['orient']}"
                 f"{'' if not p.get('orient_asked') or p['orient_asked'] == p['orient'] else ' **no**'} | "
                 f"{j.get('sof', '—')} | {j.get('subsampling', '—')} | {j.get('scans', '—')} | {j.get('quality') or '—'} | "
                 f"{'yes (' + str(len(p.get('tables_as_known') or [])) + ')' if p.get('tables_as_known') else 'no'} | "
                 f"{', '.join(blocks) or '—'} | {'/'.join('yes' if flags[k] else 'no' for k in ('EXIF', 'XMP', 'C2PA'))} | "
                 f"{j.get('tail', '—')} | {', '.join('`' + os.path.basename(c) + '`' for c in p['copies']) or '—'} |")
    by = {}
    for p in s["pictures"]:
        by.setdefault(p["asked"] or "?", []).append(f"{p['size'][0]}×{p['size'][1]}")
    L.append("\n| asked | expected | given |")
    L.append("|---|---|---|")
    for r, sizes in by.items():
        e = s["expect"].get(r)
        L.append(f"| {r} | {f'{e[0]}×{e[1]}' if e else '—'} | {', '.join(sizes)} |")
    for r, e in s["expect"].items():
        if r not in by:
            L.append(f"| {r} | {e[0]}×{e[1]} | not asked here |")
    ref = s["reference"]
    L.append(f"\n## 3. The mark and the corner under it\n\nThe template: `{ref['name']}`'s depth over "
             f"{ref['rect'][2]}×{ref['rect'][3]} at ({ref['rect'][0]}, {ref['rect'][1]}); the law: width "
             f"{ref['rect'][2] / ref['size'][0]:.5f}·W, margins {ref['margins'][0]} / {ref['margins'][1]} px.\n")
    L.append("| picture | law: x, y, w | NCC there | first look's row | NCC there | found: x, y, w | NCC | found − law "
             "(x, y, w) | margins right, bottom | w/W | variant |")
    L.append("|---|---|---|---|---|---|---|---|---|---|---|")
    for p in s["pictures"]:
        lw, rw, lc = p["law"], p["row"], p["loc"]
        rtxt = f"`{rw['from']}` ({rw['x']:.2f}, {rw['y']:.2f}, {rw['w']:.2f})" if rw else "none for this size"
        rncc = f"{rw['ncc']:+.3f}" if rw else "—"
        L.append(f"| `{p['name']}` | {lw['x']:.2f}, {lw['y']:.2f}, {lw['w']:.2f} | {lw['ncc']:+.3f} | {rtxt} | "
                 f"{rncc} | {lc['x']:.2f}, {lc['y']:.2f}, {lc['w']:.2f} | "
                 f"{lc['ncc']:.3f} | {lw['dx']:+.2f}, {lw['dy']:+.2f}, {lw['dw']:+.2f} | "
                 f"{lc['margin_right']:.2f}, {lc['margin_bottom']:.2f} | {lc['w_over_W']:.4f} | "
                 f"**{p['variant']['variant']}** (logo {'255' if p['variant']['variant'] == 'light' else '0'}) |")
    lw = s["laws"]
    L.append(f"\nOver every picture located ({lw['n']}: the first five and these): width = {lw['width_k']:.5f}·W, worst "
             f"{lw['width_k_worst_px']:.2f} px; width = {lw['width_affine'][0]:+.2f} + {lw['width_affine'][1]:.5f}·W, worst "
             f"{lw['width_affine_worst_px']:.2f} px. Margin right {f2(lw['margin_right_range'])} px "
             f"(= {lw['margin_right_affine'][0]:.2f} {lw['margin_right_affine'][1] * 1000:+.3f}·W/1000, worst "
             f"{lw['margin_right_affine_worst_px']:.2f}), bottom {f2(lw['margin_bottom_range'])} px "
             f"(= {lw['margin_bottom_affine'][0]:.2f} {lw['margin_bottom_affine'][1] * 1000:+.3f}·W/1000, worst "
             f"{lw['margin_bottom_affine_worst_px']:.2f}).\n")
    L.append("| picture (first five) | W×H | found: x, y, w | NCC | margins right, bottom | w/W |")
    L.append("|---|---|---|---|---|---|")
    for k in s["known"]:
        lc = k["loc"]
        L.append(f"| `{k['name']}` | {k['size'][0]}×{k['size'][1]} | {lc['x']:.2f}, {lc['y']:.2f}, {lc['w']:.2f} | "
                 f"{lc['ncc']:.3f} | {lc['margin_right']:.2f}, {lc['margin_bottom']:.2f} | {lc['w_over_W']:.4f} |")
    L.append("\nThe ring (`scripts/corpus/ring.py`: 8 px outside the mark's whole-pixel rectangle; a flat group passes "
             f"under σ {ring.SPREAD_LEVELS:g} levels in every channel) and the corner (luma; the box two mark-widths left "
             "and three mark-heights above the mark to the edges, the mark left out):\n")
    L.append("| picture | ring mean R, G, B | ring σ R, G, B | ring residual | looks like | ring.py (group) | corner box | "
             "corner mean ± σ | slope x, y (levels/100 px) | drop | curvature | clipped at 0 | flat |")
    L.append("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for p in s["pictures"]:
        r, c = p["ring"], p["corner"]
        L.append(f"| `{p['name']}` | {f2(r['mean'])} | {f2(r['sd'])} | {f2(max(r['residual']))} | {r['looks_like']} | "
                 f"{'pass' if r['pass'] else 'FAIL'} ({r['group']}) | {c['box'][2]}×{c['box'][3]} | "
                 f"{c['mean']:.2f} ± {c['sd']:.2f} | {c['slope_x']:+.3f}, {c['slope_y']:+.3f} | {c['drop']:.2f} | "
                 f"{c['curvature']:.2f} | {c['clipped_0'] * 100:.0f} % | **{'yes' if p['flat'] else 'no'}** |")
    L.append(f"\n*drop*: the corner's plane from its highest to its lowest point over the box (a gradient); *curvature*: "
             f"the largest departure of the corner's quadratic from that plane (a vignette, a glow); flat = ring.py passes "
             f"and both under {BEND:g} levels.\n")
    L.append(f"## 4. The opacity, read directly\n\nThe first look's map (`{ref['name']}`, dark mark on flat grey): "
             f"core median {ref['alpha']['core_median']:.3f}, max {ref['alpha']['max']:.3f}. *drawn here*: that map drawn "
             "at this picture's place and width by the area integral — what the same mark would read at this size.\n")
    L.append("| picture | how | α max | α p99 | α core median (px) | core R, G, B | drawn here: core, max | NCC | slope (all, "
             "core) | RMS after the slope | mark px at 0 |")
    L.append("|---|---|---|---|---|---|---|---|---|---|---|")
    for p in s["pictures"]:
        a = p["alpha"]
        if not a:
            L.append(f"| `{p['name']}` | not read: the corner is not flat black under a light mark nor flat white under a "
                     f"dark one | | | | | | | | | |")
            continue
        L.append(f"| `{p['name']}` | {a['how']} | {a['max']:.3f} | {a['p99']:.3f} | {a['core_median']:.3f} ({a['core_px']}) | "
                 f"{f2(a['core_median_rgb'], 3)} | {a['ref_drawn_core_median']:.3f}, {a['ref_drawn_max']:.3f} | "
                 f"{a['ncc']:.3f} | {a['slope']:.3f}, {a['slope_core']:.3f} | {a['rms']:.4f} | "
                 f"{a['clipped_0_in_mark'] * 100:.0f} % |")
    L.append("\nPairs of one size, α against α pixel for pixel:\n")
    if s["pairs"]:
        L.append("| size | a | b | corners | NCC | RMS | max |Δ| | core a, b | max a, b |")
        L.append("|---|---|---|---|---|---|---|---|---|")
        for q in s["pairs"]:
            L.append(f"| {q['size'][0]}×{q['size'][1]} | `{q['a']}` | `{q['b']}` | {' + '.join(q['kinds'])} | "
                     f"{q['ncc']:.4f} | {q['rms']:.4f} | {q['max_abs']:.4f} | {q['core_a']:.3f}, {q['core_b']:.3f} | "
                     f"{q['max_a']:.3f}, {q['max_b']:.3f} |")
    else:
        L.append("None.")
    bw = [q for q in s["pairs"] if q["black_and_white"]]
    L.append(f"\nA black-and-white pair of one size: {'yes — ' + str(len(bw)) if bw else '**none**'}.\n")
    L.append("## 5. Did Grok follow the prompt\n")
    L.append(f"The scene: luma more than {CONTENT:g} levels from the corner's level, the mark left out.\n")
    L.append("| picture | scene px (share) | box x0, y0, x1, y1 (share of W, H) | centroid | cell | in the bottom-right "
             "quarter | in the lower half |")
    L.append("|---|---|---|---|---|---|---|")
    for p in s["pictures"]:
        sc = p["scene"]
        if not sc["pixels"]:
            L.append(f"| `{p['name']}` | 0 | — | — | — | 0 | 0 |")
            continue
        L.append(f"| `{p['name']}` | {sc['pixels']} ({sc['share'] * 100:.1f} %) | {f2(sc['box'])} | "
                 f"{f2(sc['centroid'])} | {sc['cell']} | {sc['bottom_right_quarter']} | {sc['lower_half']} |")
    with open(path, "w") as f:
        f.write("\n".join(L) + "\n")


# ── selftest ─────────────────────────────────────────────────────────────────

ALPHA_TRUE = 0.36
# The synthetic mark in the reference's pixels: a slashed ring and five bars, filling 145 × 53
# exactly, so the reference's tight box is that box and the template's is 149 × 57 at (1634, 940).
SHAPE_W, SHAPE_H = 145.0, 53.0


def shape_inside(u, v):
    r = np.hypot(u - 26.5, v - 26.5)
    ring_ = (r <= 26.5) & (r >= 20.5)  # touches the box's top, bottom and left edge
    # the slash: within 3 px of the segment (8, 46)–(44, 7)
    ax, ay, bx, by = 8.0, 46.0, 44.0, 7.0
    t = np.clip(((u - ax) * (bx - ax) + (v - ay) * (by - ay)) / ((bx - ax) ** 2 + (by - ay) ** 2), 0, 1)
    slash = np.hypot(u - (ax + t * (bx - ax)), v - (ay + t * (by - ay))) <= 3.0
    bars = np.zeros_like(u, bool)
    for x0, y0, x1, y1 in ((56, 10, 62, 44), (70, 10, 76, 44), (76, 10, 92, 16), (98, 18, 104, 44),
                           (112, 10, 118, 44), (118, 25, 145, 31)):  # the last one ends at the right edge
        bars |= (u >= x0) & (u < x1) & (v >= y0) & (v < y1)
    return ring_ | slash | bars


def render_alpha(W, H, ss=8):
    """α over the picture: the mark's box (tight + 2 px at the reference's scale)
    at the first look's law — width 149/1792 · W, margins 9 and 11 — by
    ss × ss supersampling. Returns (α, (x, y, w, h) of the box)."""
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
    a[y0:y0 + ny, x0:x0 + nx] = cov * ALPHA_TRUE
    return a, (x, y, w, h)


def synth(path, W, H, bg, variant, scene_at=None, ramp=0.0, disc_in_corner=False, quality=95, extra=b""):
    a, box = render_alpha(W, H)
    B = np.full((H, W, 3), float(bg))
    if ramp:
        B += (np.arange(W)[None, :, None] / W) * ramp
    if scene_at:
        cx, cy, r = scene_at
        yy, xx = np.mgrid[0:H, 0:W]
        d = np.hypot(xx - cx * W, yy - cy * H) <= r * W
        B[d] = (255.0 - bg) * 0.8 + 20 if bg < 128 else 30.0
    if disc_in_corner:
        yy, xx = np.mgrid[0:H, 0:W]
        d = np.hypot(xx - 0.62 * W, yy - 0.8 * H) <= 0.05 * W
        B[d] = 200.0 if bg < 128 else 40.0
    A = a[..., None]
    I = A * 255.0 + (1 - A) * B if variant == "light" else (1 - A) * B
    Image.fromarray(np.clip(np.round(I), 0, 255).astype(np.uint8)).save(
        path, "JPEG", quality=quality, subsampling=2, progressive=True)
    if extra:
        b = open(path, "rb").read()
        open(path, "wb").write(b[:2] + extra + b[2:])
    return box


def seg(marker, payload):
    return bytes([0xFF, marker]) + struct.pack(">H", len(payload) + 2) + payload


def selftest():
    failures = []

    def ok(cond, what):
        print(f"{'ok  ' if cond else 'FAIL'} {what}")
        if not cond:
            failures.append(what)

    for (W, H), want in (((1792, 1008), "16:9"), ((1168, 784), "3:2"), ((1152, 1712), "2:3"), ((784, 1168), "2:3"),
                         ((1600, 1200), "4:3"), ((1024, 1024), "1:1"), ((1008, 1792), "9:16")):
        ok(nearest_ratio(W, H)[0] == want, f"{W}×{H} is nearest {want}")
    with tempfile.TemporaryDirectory() as tmp:
        P = lambda n: os.path.join(tmp, n)  # noqa: E731
        synth(P("ref.jpg"), 1792, 1008, 201, "dark")
        truth = synth(P("black.jpg"), 784, 1168, 0, "light", scene_at=(0.2, 0.15, 0.08))
        synth(P("white.jpg"), 784, 1168, 255, "dark", scene_at=(0.25, 0.2, 0.08))
        synth(P("ramp.jpg"), 1152, 1712, 40, "light", scene_at=(0.2, 0.15, 0.08), ramp=60.0)
        synth(P("disc.jpg"), 1168, 784, 0, "light", scene_at=(0.2, 0.15, 0.08), disc_in_corner=True)
        synth(P("q80.jpg"), 784, 1168, 0, "light", quality=80)
        meta = (seg(0xE1, b"Exif\0\0MM\0*\0\0\0\x08\0\0") + seg(0xE1, b"http://ns.adobe.com/xap/1.0/\0<x:xmpmeta/>")
                + seg(0xEB, b"JP\0\x01\0\0\0\x01\0\0\0\x20jumb\0\0\0\x18jumdc2pa"))
        synth(P("meta.jpg"), 784, 1168, 0, "light", extra=meta)
        open(P("black-copy.jpg"), "wb").write(open(P("black.jpg"), "rb").read())

        ref = reference_of(P("ref.jpg"))
        rx, ry, rw, rh = (int(v) for v in ref["rect"])
        # the box holds the drawn one (1634, 940, 149, 57) and JPEG's ringing grows it by at most 3 px a side
        ok(0 <= 1634 - rx <= 3 and 0 <= 940 - ry <= 3 and 0 <= rx + rw - 1783 <= 3 and 0 <= ry + rh - 997 <= 3,
           f"the reference's box {[rx, ry, rw, rh]} holds the drawn (1634, 940, 149, 57)")
        pics = read([P(n) for n in ("black.jpg", "black-copy.jpg", "white.jpg", "ramp.jpg", "disc.jpg", "q80.jpg",
                                    "meta.jpg")])
        ok(len(pics) == 6 and pics[0]["copies"] == [P("black-copy.jpg")], "a byte-identical copy is read once and named")
        known = read([P("ref.jpg")])
        by = {p["name"]: p for p in pics}
        ok(by["black"]["jpeg"]["quality"] == 95 and by["q80"]["jpeg"]["quality"] == 80,
           f"libjpeg quality read back: {by['black']['jpeg']['quality']}, {by['q80']['jpeg']['quality']}")
        ok(by["black"]["jpeg"]["sof"] == "progressive" and by["black"]["jpeg"]["subsampling"] == "4:2:0",
           "progressive, 4:2:0")
        _, flags = metadata_words(by["meta"])
        _, plain = metadata_words(by["black"])
        ok(flags == {"EXIF": True, "XMP": True, "C2PA": True}, f"EXIF, XMP and C2PA named: {flags}")
        ok(plain == {"EXIF": False, "XMP": False, "C2PA": False} and by["black"]["jpeg"]["blocks"] == ["APP0 JFIF"],
           f"a plain file: {by['black']['jpeg']['blocks']}")
        asked = parse_asked(["black=2:3,black", "white=2:3,white", "ramp=2:3,black", "disc=3:2,black"])
        pr = analyse(pics, ref, known, asked, parse_expect(EXPECT))
        b = by["black"]
        # where the template's box is drawn at this size: the drawn box, moved and scaled as the reference's box is
        sc_ = 784 / 1792.0
        x, y, w = truth[0] + (rx - 1634) * sc_, truth[1] + (ry - 940) * sc_, rw * sc_
        lc = b["loc"]
        ok(abs(lc["x"] - x) < 0.3 and abs(lc["y"] - y) < 0.3 and abs(lc["w"] - w) < 0.3,
           f"the mark found at ({lc['x']:.2f}, {lc['y']:.2f}, {lc['w']:.2f}), drawn at ({x:.2f}, {y:.2f}, {w:.2f})")
        ok(b["variant"]["variant"] == "light" and by["white"]["variant"]["variant"] == "dark", "the variants")
        ok(b["ring"]["looks_like"] == "black" and b["ring"]["pass"] and b["flat"], f"black ring flat: {b['ring']['sd']}")
        ok(by["white"]["ring"]["looks_like"] == "white" and by["white"]["flat"], "white ring flat")
        r = by["ramp"]
        ok(r["ring"]["pass"] and r["corner"]["drop"] >= BEND and not r["flat"],
           f"a gradient: the ring passes (σ {f2(r['ring']['sd'])}), the corner's drop {r['corner']['drop']:.1f} does not")
        for n in ("black", "white"):
            a = by[n]["alpha"]
            ok(a is not None and abs(a["core_median"] - ALPHA_TRUE) < 0.02,
               f"α at the core from {n}: {a and round(a['core_median'], 4)}")
        bw = [q for q in pr if q["black_and_white"] and {q["a"], q["b"]} == {"black", "white"}]
        ok(len(bw) == 1 and bw[0]["ncc"] > 0.95 and abs(bw[0]["core_a"] - bw[0]["core_b"]) < 0.02,
           f"the black-and-white pair agrees: {bw and (round(bw[0]['ncc'], 4), round(bw[0]['rms'], 4))}")
        ok(b["alpha"]["ncc"] > 0.9 and 0.94 <= b["alpha"]["slope_core"] <= 1.06,
           f"against the reference drawn there: NCC {b['alpha']['ncc']:.3f}, core slope {b['alpha']['slope_core']:.3f}")
        sc = b["scene"]
        ok(sc["cell"] == "upper left" and sc["bottom_right_quarter"] == 0, f"the scene upper left, the corner empty: {sc['cell']}")
        ok(by["disc"]["scene"]["bottom_right_quarter"] > 0, "a disc in the corner is seen")
        rows = [corner_row(p) for p in pics]
        sheet(rows, [p["name"] for p in pics], P("sheet.png"))
        ok(os.path.getsize(P("sheet.png")) > 0, "the sheet is written")
    print("selftest: " + ("passed" if not failures else f"{len(failures)} failed"))
    return 0 if not failures else 1


def main():
    today = datetime.date.today().isoformat()
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    a = sub.add_parser("check", help="steps 1–5 on the pictures")
    a.add_argument("pictures", nargs="+")
    a.add_argument("--reference", default=REFERENCE)
    a.add_argument("--known", nargs="*", default=[KNOWN])
    a.add_argument("--asked", nargs="*", default=[], metavar="STEM=RATIO[,GROUP]")
    a.add_argument("--expect", nargs="*", default=list(EXPECT), metavar="RATIO=WxH")
    a.add_argument("--out")
    a.add_argument("--date", default=today)
    sub.add_parser("selftest", help="synthetic pictures, no captures")
    args = ap.parse_args()
    if args.cmd == "selftest":
        sys.exit(selftest())
    if args.out is None:
        args.out = os.path.join("reports", f"grok-captures-check-{args.date}")
    os.makedirs(args.out, exist_ok=True)
    check(args)


if __name__ == "__main__":
    main()
