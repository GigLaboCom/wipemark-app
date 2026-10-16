#!/usr/bin/env python3
"""xwd_stats.py — what is in the window right after start, without numpy.

What it is for
  Research of the startup garbage frame (asked by the owner, 2026-10-05:
  "when the application starts its window often shows leftover screen
  contents, like a screenshot, until a click or a mouse move"). This reads
  the XWD captures `startup-capture.sh` takes of the window at fixed
  moments after launch, and says for each one whether it looks like a
  GPUI frame or like something else.

What it does
  1. Parses each XWD file (X Window Dump, ZPixmap, 24/32 bits per pixel;
     the header is big-endian, the pixels are in the dump's byte order).
  2. Per capture: the size, the share of the most common colour (a GPUI
     window is mostly its theme background, so a rendered frame has one
     dominant colour; leftover screen contents or uninitialised video
     memory usually do not), that colour, and an md5 of the pixels alone
     (the header carries the window name, which never changes).
  3. Optionally compares every capture with a "before" image — the root
     window cropped to the same rectangle, taken before the app was
     started — and prints the share of identical pixels: a high share
     means the window is showing what was on the screen there before,
     which is the owner's "like a screenshot".
  4. Compares every capture with the LAST one (taken after the app had
     settled): "same" means nothing changed on screen between the two.

How to run
  python3 xwd_stats.py [--png] [--before before.xwd --rect X,Y,W,H] cap1.xwd cap2.xwd ...
  (--png also writes cap.xwd.png, every 4th pixel, to look at.)
  (startup-capture.sh runs it for you.)

What it needs
  python3 only.

What its output means
  One line per capture: name, WxH, dominant colour share and value,
  pixel md5, share equal to the pre-launch screen, equal-to-last.
  A rendered GPUI window: dominant share well above 0.3, md5 equal to the
  settled capture. The bug: an early capture with a low dominant share or a
  high "before" share, and — the owner's case — the settled capture still
  looking like that, because nothing presented a frame.
"""
import collections
import hashlib
import struct
import sys


def read_xwd(path):
    with open(path, "rb") as f:
        data = f.read()
    (header_size,) = struct.unpack(">I", data[0:4])
    fields = struct.unpack(">25I", data[0:100])
    (
        _hs, _ver, pixmap_format, depth, width, height, _xoff, byte_order,
        _unit, _bitorder, _pad, bits_per_pixel, bytes_per_line, _vclass,
        _rmask, _gmask, _bmask, _bprgb, _cmap_entries, ncolors, _ww, _wh,
        _wx, _wy, _bw,
    ) = fields
    if pixmap_format != 2 or bits_per_pixel not in (24, 32):
        raise SystemExit(f"{path}: unsupported XWD (format {pixmap_format}, {bits_per_pixel} bpp)")
    offset = header_size + ncolors * 12
    bpp = bits_per_pixel // 8
    rows = []
    for y in range(height):
        start = offset + y * bytes_per_line
        row = data[start:start + width * bpp]
        rows.append(row)
    return width, height, bpp, byte_order, rows


def pixels(img, rect=None):
    width, height, bpp, _order, rows = img
    x0, y0, w, h = rect if rect else (0, 0, width, height)
    out = []
    for y in range(max(0, y0), min(height, y0 + h)):
        row = rows[y]
        for x in range(max(0, x0), min(width, x0 + w)):
            # Ignore the alpha/pad byte: a depth-32 ARGB window and the
            # depth-24 root disagree there and agree on the colour.
            out.append(row[x * bpp:x * bpp + 3])
    return out


def write_png(img, path, step=4):
    """A PNG of the dump, every `step`-th pixel, for a person to look at."""
    import zlib

    width, height, bpp, _order, rows = img
    w, h = (width + step - 1) // step, (height + step - 1) // step
    raw = bytearray()
    for y in range(0, height, step):
        raw.append(0)
        row = rows[y]
        for x in range(0, width, step):
            b, g, r = row[x * bpp], row[x * bpp + 1], row[x * bpp + 2]
            raw += bytes((r, g, b))

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(bytes(raw), 6)) + chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(png)


def main(argv):
    before = None
    rect = None
    caps = []
    png = False
    i = 0
    while i < len(argv):
        if argv[i] == "--png":
            # Writes <capture>.png beside each dump (little-endian BGRx
            # assumed, which is what this host's X server produces).
            png = True
            i += 1
        elif argv[i] == "--before":
            before = argv[i + 1]
            i += 2
        elif argv[i] == "--rect":
            rect = tuple(int(v) for v in argv[i + 1].split(","))
            i += 2
        else:
            caps.append(argv[i])
            i += 1
    if not caps:
        raise SystemExit(__doc__)
    before_px = None
    if before and rect:
        before_px = pixels(read_xwd(before), rect)
    if png:
        for c in caps:
            write_png(read_xwd(c), c + ".png")
    parsed = [(c, pixels(read_xwd(c)), read_xwd(c)[:2]) for c in caps]
    last_px = parsed[-1][1]
    print(f"{'capture':<28} {'size':>10} {'dominant':>8} {'colour':>8} {'md5':>12} {'=before':>8} {'=last':>6}")
    for name, px, (w, h) in parsed:
        counts = collections.Counter(px)
        colour, n = counts.most_common(1)[0]
        share = n / max(1, len(px))
        md5 = hashlib.md5(b"".join(px)).hexdigest()[:12]
        eq_before = "-"
        if before_px is not None and len(before_px) == len(px):
            same = sum(1 for a, b in zip(px, before_px) if a == b)
            eq_before = f"{same / len(px):.2f}"
        eq_last = "same" if px == last_px else "diff"
        print(f"{name.rsplit('/', 1)[-1]:<28} {w:>4}x{h:<5} {share:>8.2f} {colour.hex():>8} {md5:>12} {eq_before:>8} {eq_last:>6}")


if __name__ == "__main__":
    main(sys.argv[1:])
