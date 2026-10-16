#!/usr/bin/env python3
"""A synthetic text-like mark for the bench's and the tools' tests — a fixture, not a vendor's.

What it is for
--------------
Step E12-R12 of the E12-R series, stage 4b (`docs/plan/E12-R12-grok-thresholds-and-support.md`
§4.2, filed 2026-10-08), the part that can exist before a Grok profile does; written for the
coordinator on 2026-10-09. The bench (`recon_bench`) and `measure_clean` take a profile from a
catalogue file (`--catalogue`), the shape R11 §4.3 will commit Grok's provisional profile in. Until
that profile exists, the road is proved with a stand-in: a wordmark-like opacity map made of
straight strokes — no font file, no capture, no vendor — whose only job is to be **not a sparkle**:
wider than tall, thin strokes, a contour density nothing in Gemini's catalogue has. It is a test
fixture and nothing else: it measures nothing, it is not Grok's mark, and no figure about any
vendor can be read off it.

What it does
------------
1. Draws a 72 × 24 opacity map: four glyph-like groups of straight segments, each pixel's opacity
   `PEAK × clamp(HALF + 0.5 − d, 0, 1)` where `d` is the distance from the pixel's centre to the
   nearest segment (a one-pixel antialiased ramp), so no faint sample stands apart from the body
   and `wipemark_pixels::drawn` leaves the map as it is.
2. Writes it as a 16-bit `.wma` (`WMA1`, u16 width, u16 height, u8 depth, samples little-endian —
   `crates/wipemark-pixels/src/alpha.rs`).
3. Writes `marks.json` beside it: a catalogue in the shipped one's schema (`manifests/marks.v1.json`)
   with one profile, `fixture-wordmark`, `status: "provisional"`, its map pinned by sha256, two
   bottom-left corner rows and a bounded search — the shape a provisional profile has.

How to run it
-------------
    python3 scripts/bench/wordmark.py [--out fixtures/marks/synthetic-wordmark]
    python3 scripts/bench/wordmark.py check [--out …]

The first writes the two files (the committed ones were written by it); `check` regenerates them in
memory and exits 1 when the committed bytes differ — the files are a function of this script.

What it needs
-------------
Python 3.10+, the standard library alone. Only `+ − × ÷` and `sqrt` (IEEE-exact), so the bytes are
the same on every machine.

What its output means
---------------------
Two files under `fixtures/marks/synthetic-wordmark/`, read by the tests of
`crates/wipemark-picture/examples/recon_bench.rs` and `measure_clean.rs` through the tools'
`--catalogue` road. Exit 0 when written or matching, 1 when `check` finds a difference, 2 on a
usage error.
"""

import hashlib
import json
import math
import os
import struct
import sys

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
OUT = os.path.join(REPO, "fixtures", "marks", "synthetic-wordmark")
WIDTH, HEIGHT = 72, 24
PEAK = 0.5  # the strokes' opacity: under opaque_above (0.95), so nothing is a hole
HALF = 1.1  # half the stroke's width, in pixels
MAP_ID = "fixture-wordmark-72x24"
PROFILE = "fixture-wordmark"

# Four glyph-like groups, each a list of segments ((x0, y0), (x1, y1)) in a 12 × 16 cell; the
# cells start at these x and at y = 4.
GLYPHS = [
    (4, [((0, 0), (10, 16)), ((10, 0), (0, 16))]),
    (20, [((0, 0), (0, 16)), ((10, 0), (10, 16)), ((0, 8), (10, 8))]),
    (36, [((0, 0), (10, 0)), ((10, 0), (0, 16)), ((0, 16), (10, 16))]),
    (52, [((0, 0), (12, 0)), ((6, 0), (6, 16))]),
]
TOP = 4


def segments():
    for x0, segs in GLYPHS:
        for (ax, ay), (bx, by) in segs:
            yield (x0 + ax, TOP + ay), (x0 + bx, TOP + by)


def distance(px, py, a, b):
    (ax, ay), (bx, by) = a, b
    dx, dy = bx - ax, by - ay
    length2 = dx * dx + dy * dy
    t = 0.0 if length2 == 0 else max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / length2))
    qx, qy = ax + t * dx - px, ay + t * dy - py
    return math.sqrt(qx * qx + qy * qy)


def opacity():
    segs = list(segments())
    values = []
    for y in range(HEIGHT):
        for x in range(WIDTH):
            d = min(distance(x + 0.5, y + 0.5, a, b) for a, b in segs)
            values.append(PEAK * max(0.0, min(1.0, HALF + 0.5 - d)))
    return values


def wma(values):
    out = bytearray(b"WMA1")
    out += struct.pack("<HHB", WIDTH, HEIGHT, 16)
    for v in values:
        out += struct.pack("<H", int(v * 65535 + 0.5))
    return bytes(out)


def catalogue(sha):
    profile = {
        "id": PROFILE,
        "vendor": "fixture",
        "product": "bench",
        "mark": "wordmark",
        "observed": {"from": None, "until": None},
        "status": "provisional",
        "blend": {"model": "encoded", "logo": [255, 255, 255], "logo_map": None},
        "opaque_above": 0.95,
        "alpha": [{"id": MAP_ID, "asset": MAP_ID + ".wma", "sha256": sha, "size": [WIDTH, HEIGHT]}],
        "placements": [
            {"when": {"min_width": 1025, "min_height": 1025}, "corner": "bottom-left", "margin": [48, 40], "alpha": MAP_ID},
            {"when": {}, "corner": "bottom-left", "margin": [24, 20], "alpha": MAP_ID},
        ],
        "search": {"corner": "bottom-left", "within": [320, 320], "sizes": [36, 144], "alpha": MAP_ID},
        "detect": {"min_ncc": 0.70},
        "verify": {"gain": 0.06, "edge_ratio": 0.30, "out_of_range": 0.01},
        "source": {"from": "scripts/bench/wordmark.py", "commit": None, "licence": None, "copyright": None},
    }
    return json.dumps({"schema": 1, "profiles": [profile]}, indent=2) + "\n"


def files():
    data = wma(opacity())
    return {MAP_ID + ".wma": data, "marks.json": catalogue(hashlib.sha256(data).hexdigest()).encode()}


def main(argv):
    args = list(argv)
    check = bool(args) and args[0] == "check"
    if check:
        args = args[1:]
    out = OUT
    if args[:1] == ["--out"] and len(args) == 2:
        out = args[1]
    elif args:
        print(__doc__.split("How to run it")[1].split("What it needs")[0], file=sys.stderr)
        return 2
    want = files()
    if check:
        bad = []
        for name, data in want.items():
            path = os.path.join(out, name)
            if not os.path.exists(path) or open(path, "rb").read() != data:
                bad.append(name)
        for name in bad:
            print(f"wordmark.py: {os.path.join(out, name)} is not what this script writes", file=sys.stderr)
        print("check:", "FAIL" if bad else "ok")
        return 1 if bad else 0
    os.makedirs(out, exist_ok=True)
    for name, data in want.items():
        with open(os.path.join(out, name), "wb") as f:
            f.write(data)
    print(f"wrote {', '.join(sorted(want))} into {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
