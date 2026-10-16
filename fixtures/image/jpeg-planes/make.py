#!/usr/bin/env python3
"""Small synthetic JPEGs whose stored planes `wipemark-picture` reads (E12-R3).

What it is for
--------------
Step E12-R3 of the E12-R series (`docs/plan/E12-R3-jpeg-planes.md`, filed by
the coordinator on 2026-10-08 from the owner's spec
`wipemark-recon-spec-2026-10-08`, `03-jpeg-planes-decoder.md`) hands the
restoration what a JPEG stored: Y, Cb and Cr at their own resolution and the
quantisation tables. Its tests (`crates/wipemark-picture/tests/planes.rs`)
need JPEGs of every shape the planes take — 4:4:4, 4:2:2, 4:2:0, grey,
progressive, a restart interval, two tables in one DQT segment, a 16-bit
table — at **odd** sizes, where the MCU padding has to be cropped. This
script writes them; they are committed, and no test runs it.

What it does
------------
1. Draws two procedural RGB pictures, 37 x 23 and 129 x 65: gradients in
   each channel, a diagonal stripe pattern and a fixed pseudo-random noise
   (a 32-bit LCG, seed 2026), so the chroma varies from pixel to pixel and
   every upsampling filter has something to do. A grey picture is its luma.
2. Saves them with Pillow as JPEG in each variant listed in `VARIANTS`
   (quality, subsampling 0/1/2 = 4:4:4/4:2:2/4:2:0, progressive, restart
   interval, custom tables).
3. `rgb-37x23-q90-420-one-dqt.jpg`: libjpeg writes each table in a DQT
   segment of its own; this one is rewritten so both tables sit in **one**
   segment (the payloads concatenated, the length fixed), which a decoder
   has to read as two tables.
4. `rgb-37x23-dqt16-420.jpg`: custom tables, the luma one with values over
   255, which libjpeg writes as a 16-bit (precision 1) table under SOF1.
5. `ycc-37x23-q100-420.jpg` and `ycc-129x65-q100-420.jpg`: a **known**
   YCbCr picture — Y, Cb and Cr from the closed-form `ycc()` below, saved
   in Pillow's `YCbCr` mode, so no colour conversion happens before the
   encoder — at quality 100 (every table entry 1) and 4:2:0. The test
   recomputes the full-resolution chroma with the same formula and holds the
   decoded chroma plane to its 2 x 2 means. `ycc()` is restated in the test;
   change both or neither.
6. `even-38x24-q90-420.jpg`: an even size, the one place the decoder's
   upsampler reads a chroma sample from the MCU padding (the last column
   and row). It is kept apart from the odd ones on purpose; see the test.
7. Prints each file's name, size and sha256.

How to run it
-------------
    python3 fixtures/image/jpeg-planes/make.py [OUT_DIR]

`OUT_DIR` defaults to this script's directory. The bytes depend on Pillow's
libjpeg: Pillow **12.3.0** (libjpeg 6.2) made the committed files, the same
Pillow as `scripts/verify/images/round4-ebf421a/mkset.py`; another version
may write other bytes, and then the tests' pinned hashes
(`the_rgb_raster_did_not_move`) move with them.

What it needs
-------------
Pillow (a venv is fine; nothing in the repository depends on it). No numpy.

What its output means
---------------------
The JPEGs in `OUT_DIR`, and on stdout one line per file:
`<name> <bytes> <sha256>`. The sha256 of the committed files are those
lines at the time they were made (2026-10-09).
"""

import hashlib
import io
import os
import sys

from PIL import Image

OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.dirname(os.path.abspath(__file__))


def lcg(seed):
    state = seed & 0xFFFFFFFF
    while True:
        state = (state * 1664525 + 1013904223) & 0xFFFFFFFF
        yield state >> 24


def rgb_picture(w, h):
    noise = lcg(2026)
    px = []
    for y in range(h):
        for x in range(w):
            n = next(noise) - 128
            stripe = 40 if ((x + 2 * y) // 3) % 2 else -40
            r = 30 + 190 * x // max(w - 1, 1) + n // 4
            g = 220 - 170 * y // max(h - 1, 1) + stripe
            b = 128 + stripe + n // 2
            px.append(tuple(max(0, min(255, v)) for v in (r, g, b)))
    im = Image.new("RGB", (w, h))
    im.putdata(px)
    return im


def ycc(x, y):
    """The known YCbCr picture: restated in `tests/planes.rs` (`ycc`)."""
    yy = 40 + (x * 5 + y * 3) % 170
    cb = 64 + (x * 3 + y * 2) % 128
    cr = 192 - (x * 2 + y * 5) % 128
    return yy, cb, cr


def ycc_picture(w, h):
    im = Image.new("YCbCr", (w, h))
    im.putdata([ycc(x, y) for y in range(h) for x in range(w)])
    return im


def one_dqt(data):
    """Both DQT segments before the frame merged into one."""
    out = bytearray(data[:2])
    at = 2
    tables = bytearray()
    first = None
    while at < len(data):
        assert data[at] == 0xFF, at
        marker = data[at + 1]
        length = int.from_bytes(data[at + 2 : at + 4], "big")
        segment = data[at : at + 2 + length]
        if marker == 0xDB:
            if first is None:
                first = len(out)
            tables += segment[4:]
        else:
            if marker in (0xC0, 0xC1, 0xC2) and first is not None:
                merged = b"\xff\xdb" + (len(tables) + 2).to_bytes(2, "big") + tables
                out[first:first] = merged
                out += data[at:]
                return bytes(out)
            out += segment
        at += 2 + length
    raise ValueError("no frame after the DQT segments")


SMALL = rgb_picture(37, 23)
LARGE = rgb_picture(129, 65)
EVEN = rgb_picture(38, 24)

# A 16-bit luma table (values over 255) and an 8-bit chroma one, in the
# natural order Pillow takes them.
LUMA16 = [min(4 + 9 * (i // 8 + i % 8), 600) * (2 if i % 3 == 0 else 1) for i in range(64)]
CHROMA8 = [min(6 + 5 * (i // 8 + i % 8), 255) for i in range(64)]

VARIANTS = [
    # (file, picture, save options, post-processing)
    ("rgb-37x23-q90-444.jpg", SMALL, dict(quality=90, subsampling=0), None),
    ("rgb-37x23-q90-422.jpg", SMALL, dict(quality=90, subsampling=1), None),
    ("rgb-37x23-q90-420.jpg", SMALL, dict(quality=90, subsampling=2), None),
    ("rgb-129x65-q85-444.jpg", LARGE, dict(quality=85, subsampling=0), None),
    ("rgb-129x65-q85-422.jpg", LARGE, dict(quality=85, subsampling=1), None),
    ("rgb-129x65-q85-420.jpg", LARGE, dict(quality=85, subsampling=2), None),
    ("grey-37x23-q90.jpg", SMALL.convert("L"), dict(quality=90), None),
    ("grey-129x65-q85.jpg", LARGE.convert("L"), dict(quality=85), None),
    (
        "rgb-129x65-q90-420-progressive.jpg",
        LARGE,
        dict(quality=90, subsampling=2, progressive=True),
        None,
    ),
    (
        "rgb-37x23-q90-444-progressive.jpg",
        SMALL,
        dict(quality=90, subsampling=0, progressive=True),
        None,
    ),
    (
        "rgb-129x65-q90-420-restart.jpg",
        LARGE,
        dict(quality=90, subsampling=2, restart_marker_blocks=3),
        None,
    ),
    ("rgb-37x23-q90-420-one-dqt.jpg", SMALL, dict(quality=90, subsampling=2), one_dqt),
    (
        "rgb-37x23-dqt16-420.jpg",
        SMALL,
        dict(subsampling=2, qtables=[LUMA16, CHROMA8]),
        None,
    ),
    ("ycc-37x23-q100-420.jpg", ycc_picture(37, 23), dict(quality=100, subsampling=2), None),
    ("ycc-129x65-q100-420.jpg", ycc_picture(129, 65), dict(quality=100, subsampling=2), None),
    ("even-38x24-q90-420.jpg", EVEN, dict(quality=90, subsampling=2), None),
]

for name, picture, options, post in VARIANTS:
    buf = io.BytesIO()
    picture.save(buf, "JPEG", **options)
    data = buf.getvalue()
    if post is not None:
        data = post(data)
    with open(os.path.join(OUT, name), "wb") as f:
        f.write(data)
    print(name, len(data), hashlib.sha256(data).hexdigest())
