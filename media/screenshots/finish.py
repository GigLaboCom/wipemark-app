#!/usr/bin/env python3
"""finish.py — turn raw window captures into the files in media/out/screenshots/.

What it is for: the captures media/screenshots/wm.py takes are whole
windows at 2x. The README and the website want them at a bounded size and
weight, one of them cut to a dialog, and one picture that is not a capture
at all — a marked picture before and after, built from the files themselves.

What it does, one subcommand per call:
  fit IN OUT                    long side at most 2400 px, the PNG at most
                                1.5 MB (re-saved optimised; if still too
                                heavy, quantised to 256 colours; if still
                                too heavy, scaled down step by step)
  crop IN OUT X Y W H           cut a rectangle given in window points (the
                                capture is 2x), then fit it
  beforeafter BEFORE AFTER OUT  the two pictures side by side, a thin
                                divider between them and a small "Before" /
                                "After" label on each, then fit it

How to run:  python3 media/screenshots/finish.py <subcommand> ...
Needs:       python3 with Pillow; macOS for the label font (Helvetica).
"""

import io
import sys

from PIL import Image, ImageDraw, ImageFont

LONG_SIDE = 2400
MAX_BYTES = 1_500_000


def encode(img, quantise=False):
    buf = io.BytesIO()
    if quantise:
        img = img.convert("RGB").quantize(colors=256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
    img.save(buf, format="PNG", optimize=True)
    return buf.getvalue()


def fit(img, out):
    if max(img.size) > LONG_SIDE:
        scale = LONG_SIDE / max(img.size)
        img = img.resize((round(img.width * scale), round(img.height * scale)), Image.Resampling.LANCZOS)
    data = encode(img)
    how = "optimised"
    if len(data) > MAX_BYTES:
        data, how = encode(img, quantise=True), "256 colours"
    while len(data) > MAX_BYTES:
        img = img.resize((round(img.width * 0.9), round(img.height * 0.9)), Image.Resampling.LANCZOS)
        data, how = encode(img), f"scaled to {img.width}x{img.height}"
        if len(data) > MAX_BYTES:
            data = encode(img, quantise=True)
    with open(out, "wb") as f:
        f.write(data)
    print(f"{out}: {img.width}x{img.height}, {len(data)} bytes, {how}")


def crop(src, out, x, y, w, h):
    img = Image.open(src)
    s = 2
    fit(img.crop((x * s, y * s, (x + w) * s, (y + h) * s)), out)


def beforeafter(before, after, out):
    a = Image.open(before).convert("RGB")
    b = Image.open(after).convert("RGB")
    if b.size != a.size:
        b = b.resize(a.size, Image.Resampling.LANCZOS)
    gap = 6
    canvas = Image.new("RGB", (a.width * 2 + gap, a.height), (255, 255, 255))
    canvas.paste(a, (0, 0))
    canvas.paste(b, (a.width + gap, 0))
    d = ImageDraw.Draw(canvas, "RGBA")
    size = max(20, a.width // 36)
    font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", size, index=1)
    for text, left in (("Before", 0), ("After", a.width + gap)):
        w = d.textlength(text, font=font)
        x, y, px, py = left + size, size, size * 0.6, size * 0.35
        d.rounded_rectangle((x, y, x + w + 2 * px, y + size + 2 * py), radius=size * 0.5, fill=(0, 0, 0, 150))
        d.text((x + px, y + py - size * 0.08), text, font=font, fill=(255, 255, 255, 255))
    fit(canvas, out)


def main(argv):
    cmd, args = argv[1], argv[2:]
    if cmd == "fit":
        fit(Image.open(args[0]), args[1])
    elif cmd == "crop":
        crop(args[0], args[1], *[int(v) for v in args[2:6]])
    elif cmd == "beforeafter":
        beforeafter(args[0], args[1], args[2])
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main(sys.argv)
