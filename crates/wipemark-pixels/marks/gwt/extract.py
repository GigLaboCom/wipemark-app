#!/usr/bin/env python3
"""Extract GeminiWatermarkTool's four opacity maps and convert them to `.wma`.

Run once, on a machine that may clone the reference (the container that
wrote this crate could not):

    git clone https://github.com/allenk/GeminiWatermarkTool "$S/gwt-full"
    git -C "$S/gwt-full" checkout 7c6a99f
    python3 crates/wipemark-pixels/marks/gwt/extract.py "$S/gwt-full"

What it does, and refuses:

1. Reads `assets/embedded_assets.hpp` and takes the four byte arrays
   `bg_48_png`, `bg_96_png`, `bg_b_36_png`, `bg_b_96_png` (the hex between
   `{` and `};`), written as bytes. Refuses unless each file's sha256 is the
   one `docs/plan/E12-visible-marks.md` §3 recorded.
2. Writes them beside this script under their own names (`bg_48.png` ...):
   they are the provenance, MIT, committed as they are.
3. Decodes each PNG (8-bit, non-interlaced; grey, grey+alpha, RGB, RGBA)
   and writes `gemini-v1-48.wma` ... at depth 8 with `sample = max(R, G, B)`
   (the grey value for a grey PNG) — exactly the alpha GWT computes.
4. Puts each `.wma`'s sha256 into `manifests/marks.v1.json` in place of the
   `pending: ...` pin, and into the table of `marks/README.md`.

No pixel is invented and no file other than these is written. Nothing of
GWT's code is copied: only its four data arrays, under its MIT licence
(`NOTICE`).
"""

import hashlib
import pathlib
import re
import struct
import sys
import zlib

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[3]
MANIFEST = ROOT / "manifests" / "marks.v1.json"
README = HERE.parent / "README.md"

# array name, PNG file, .wma file, expected PNG sha256
ASSETS = [
    ("bg_48_png", "bg_48.png", "gemini-v1-48.wma",
     "4afc99afe0ef108d67acc45bf4dc5da867ddb793bebc89c9243bb121ce7f0f57"),
    ("bg_96_png", "bg_96.png", "gemini-v1-96.wma",
     "3e26f2233a12a5829acac174d8df1f3db40e07fef04ecdd0e035732154077911"),
    ("bg_b_36_png", "bg_b_36.png", "gemini-v2-36.wma",
     "a3e7d5ca932e6acf9ff826a4db47d597458480e72089da81a40bd4b52668cd31"),
    ("bg_b_96_png", "bg_b_96.png", "gemini-v2-96.wma",
     "3911f3b68b3083096326cee24f09868ec87f8d39d248e97057cd14ee838c5552"),
]


def fail(message):
    print(f"extract.py: {message}", file=sys.stderr)
    sys.exit(1)


def arrays(header):
    """Every `name[...] = { ... };` byte array in the header."""
    found = {}
    pattern = re.compile(r"(\w+)\s*\[[^\]]*\]\s*=\s*\{([^}]*)\}\s*;", re.S)
    for name, body in pattern.findall(header):
        tokens = [t for t in re.split(r"[\s,]+", body) if t]
        try:
            found[name] = bytes(int(t, 0) for t in tokens)
        except ValueError:
            continue
    return found


def paeth(a, b, c):
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    return b if pb <= pc else c


def decode_png(data):
    """`(width, height, channels, rows)` of an 8-bit non-interlaced PNG."""
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        fail("not a PNG")
    pos, idat, ihdr = 8, b"", None
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos:pos + 4])
        kind = data[pos + 4:pos + 8]
        body = data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            ihdr = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat += body
        elif kind == b"PLTE":
            fail("a palette PNG: not expected for these maps")
        pos += 12 + length
    width, height, depth, colour, _, _, interlace = ihdr
    channels = {0: 1, 2: 3, 4: 2, 6: 4}.get(colour)
    if depth != 8 or channels is None or interlace != 0:
        fail(f"PNG depth {depth}, colour type {colour}, interlace {interlace}: not handled")
    raw = zlib.decompress(idat)
    stride = width * channels
    rows, prev, at = [], bytearray(stride), 0
    for _ in range(height):
        kind, line = raw[at], bytearray(raw[at + 1:at + 1 + stride])
        at += 1 + stride
        for i in range(stride):
            a = line[i - channels] if i >= channels else 0
            b = prev[i]
            c = prev[i - channels] if i >= channels else 0
            if kind == 1:
                line[i] = (line[i] + a) & 0xFF
            elif kind == 2:
                line[i] = (line[i] + b) & 0xFF
            elif kind == 3:
                line[i] = (line[i] + (a + b) // 2) & 0xFF
            elif kind == 4:
                line[i] = (line[i] + paeth(a, b, c)) & 0xFF
            elif kind != 0:
                fail(f"filter {kind}")
        rows.append(bytes(line))
        prev = line
    return width, height, channels, rows


def wma(width, height, channels, rows):
    out = bytearray(b"WMA1")
    out += struct.pack("<HHB", width, height, 8)
    for row in rows:
        for x in range(width):
            p = row[x * channels:(x + 1) * channels]
            out.append(p[0] if channels <= 2 else max(p[0], p[1], p[2]))
    return bytes(out)


def main():
    if len(sys.argv) != 2:
        fail("usage: extract.py <GeminiWatermarkTool checkout at 7c6a99f>")
    gwt = pathlib.Path(sys.argv[1])
    header = (gwt / "assets" / "embedded_assets.hpp").read_text(encoding="utf-8", errors="replace")
    found = arrays(header)
    manifest = MANIFEST.read_text(encoding="utf-8")
    readme = README.read_text(encoding="utf-8")
    for array, png_name, wma_name, expected in ASSETS:
        if array not in found:
            fail(f"{array} is not in embedded_assets.hpp")
        png = found[array]
        got = hashlib.sha256(png).hexdigest()
        if got != expected:
            fail(f"{array}: sha256 {got}, expected {expected} — not the commit the plan names")
        (HERE / png_name).write_bytes(png)
        converted = wma(*decode_png(png))
        (HERE / wma_name).write_bytes(converted)
        pin = hashlib.sha256(converted).hexdigest()
        line = re.compile(r'("asset": "' + re.escape(wma_name) + r'", "sha256": ")[^"]*(")')
        manifest, n = line.subn(lambda m: m.group(1) + pin + m.group(2), manifest)
        if n != 1:
            fail(f"{wma_name}: {n} rows in the manifest")
        readme = readme.replace(f"`{wma_name}` sha256: pending", f"`{wma_name}` sha256: `{pin}`")
        print(f"{png_name} {got}\n  -> {wma_name} {pin}")
    MANIFEST.write_text(manifest, encoding="utf-8")
    README.write_text(readme, encoding="utf-8")


if __name__ == "__main__":
    main()
