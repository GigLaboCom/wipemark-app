#!/usr/bin/env python3
"""What a PNG is made of: its bit depth, its colour type and its chunks.

What it is for
--------------
The coordinator's task `wipemark-task-models-pipeline-followups-2026-10-09`
(M10, 2026-10-09) asked what was done to the 1025 crops under
`fixtures/image/gemini/`, whose README said "saved as an RGB PNG": the mode
of each committed PNG, and whether it carries anything but pixels. No script
that cut them is in the tree, so the files themselves are the answer; this
reads them.

What it does
------------
For each PNG given: walks its chunks (length, type, data, CRC — the CRC is
not checked), and prints the bit depth and colour type from `IHDR` (2 is
RGB, 6 is RGBA, 0 grey, 3 palette, 4 grey with alpha) and the chunk types in
order, a run of one type counted (`IDATx12`).

How to run
----------
    python3 -I scripts/verify/images/png-chunks.py fixtures/image/gemini/*.png

What it needs
-------------
Python 3, the standard library only.

What its output means
---------------------
`colortype 2 IHDR IDAT IEND` is an 8-bit RGB picture with nothing beside its
pixels: no text, no EXIF, no C2PA, and no colour chunk (`iCCP`, `sRGB`,
`gAMA`, `cHRM`). On 2026-10-09 the four `*-1025.png` crops read so, and
`crying-transparent-1025.png` read `colortype 6` (RGBA) with the same three
chunks.
"""

import struct
import sys


def chunks(data):
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("not a PNG")
    at = 8
    while at + 8 <= len(data):
        (length,) = struct.unpack(">I", data[at : at + 4])
        kind = data[at + 4 : at + 8].decode("latin-1")
        yield kind, data[at + 8 : at + 8 + length]
        at += 12 + length


def main(paths):
    if not paths:
        print("usage: png-chunks.py FILE.png [...]")
        return 2
    for path in paths:
        with open(path, "rb") as source:
            data = source.read()
        runs = []
        depth = colour = None
        for kind, body in chunks(data):
            if kind == "IHDR":
                _, _, depth, colour = struct.unpack(">IIBB", body[:10])
            if runs and runs[-1][0] == kind:
                runs[-1][1] += 1
            else:
                runs.append([kind, 1])
        listed = " ".join(kind if count == 1 else "%sx%d" % (kind, count) for kind, count in runs)
        print("%s  bitdepth %s colortype %s  %s" % (path, depth, colour, listed))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
