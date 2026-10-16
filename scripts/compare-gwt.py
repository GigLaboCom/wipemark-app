#!/usr/bin/env python3
"""GeminiWatermarkTool against Wipemark on the same picture, side by side.

What it is for
--------------
The owner asked (2026-10-05) why GeminiWatermarkTool (GWT) "removes" the
Gemini sparkle while Wipemark says a trace is left on `11_crying`. This
script answers by showing both results on the same file: one PNG per input,
two columns (GWT on the left, Wipemark on the right) and four rows.

    row 1  the original — the mark's corner, x4, nearest neighbour
    row 2  the result — the same corner, x4, as an eye sees it zoomed
    row 3  the result's deviation from the picture around the mark, x10
           (mid grey is "the same as around it"; a ring or a darker square
           is what the restoration left)
    row 4  the result at its own size, 1:1 (the picture's bottom-right
           704 x 704 px, the mark in it)

Under each column's third row are the numbers: the faint band's mean step in
R, G, B and luma against a ring 8-36 px outside the mark, and for Wipemark
its own verdict (exit code, outline/texture said).

How each side is produced
-------------------------
* GWT: its algorithm, re-implemented here from allenk/GeminiWatermarkTool at
  7c6a99f (MIT) — the alpha map is its embedded `bg_96.png`/`bg_48.png`
  (committed unchanged in `crates/wipemark-pixels/marks/gwt/`), alpha =
  max(R, G, B)/255, pixels under alpha 0.002 untouched, alpha capped at 0.99,
  the logo white (255), `O = (I - a*255) / (1 - a)`, rounded and clamped. The
  mark is placed by GWT's own rule: 96 px at a 64 px margin when both sides
  are over 1024, else 48 px at 32. No detection, no check — GWT writes
  whatever the formula gives. This is a re-implementation, not GWT's binary;
  pass `--gwt-out DIR` with files GWT itself wrote (same names) to use those.
* Wipemark: `wipemark-cli clean <file> -o <out> --json`, the release build of
  this repository (`cargo build --release -p wipemark-cli`).

Usage
-----
    python3 scripts/compare-gwt.py STICKER.png [MORE.png ...] [-o OUT_DIR]

Needs numpy and Pillow (a venv is fine; nothing in the repository depends on
them). Writes `<name>-gwt-vs-wipemark.png` per input into OUT_DIR
(default: the current directory).
"""

import argparse
import json
import os
import subprocess
import sys
import tempfile

import numpy as np
from PIL import Image, ImageDraw, ImageFont

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GWT_MAPS = os.path.join(REPO, "crates", "wipemark-pixels", "marks", "gwt")
LUMA = np.array([0.299, 0.587, 0.114])
ZOOM = 4
AMPLIFY = 10
PAD = 40  # pixels of the picture shown around the mark in rows 1-3


def gwt_place(width, height):
    """GWT's size rule: the large mark on pictures over 1024 both ways."""
    if width > 1024 and height > 1024:
        size, margin, name = 96, 64, "bg_96.png"
    else:
        size, margin, name = 48, 32, "bg_48.png"
    return width - margin - size, height - margin - size, size, name


def gwt_alpha(name):
    bg = np.asarray(Image.open(os.path.join(GWT_MAPS, name)).convert("RGB"))
    return bg.max(axis=2).astype(np.float64) / 255.0


def gwt_remove(img):
    """GWT's reverse alpha blend over the whole square, no checks."""
    h, w, _ = img.shape
    x0, y0, size, name = gwt_place(w, h)
    alpha = gwt_alpha(name)
    out = img.astype(np.float64).copy()
    patch = out[y0:y0 + size, x0:x0 + size]
    a = np.minimum(alpha, 0.99)[..., None]
    restored = np.clip(np.round((patch - a * 255.0) / (1.0 - a)), 0, 255)
    out[y0:y0 + size, x0:x0 + size] = np.where(alpha[..., None] >= 0.002, restored, patch)
    return out.astype(np.uint8)


def wipemark_clean(cli, path, workdir):
    out = os.path.join(workdir, "wipemark-" + os.path.basename(path))
    run = subprocess.run([cli, "clean", path, "-o", out, "--json"],
                         capture_output=True, text=True)
    verdict = {"exit": run.returncode}
    try:
        report = json.loads(run.stdout)["report"]
        verdict["marks_left"] = report.get("marks_left")
        restored = report.get("visible", {}).get("restored", [])
        verdict["outline"] = any(r.get("outline_left") for r in restored)
        verdict["texture"] = any(r.get("texture_left") for r in restored)
        verdict["restored"] = bool(restored)
    except (ValueError, KeyError):
        verdict["error"] = (run.stderr or run.stdout).strip().splitlines()[-1:] or ["?"]
    if not os.path.exists(out):
        return None, verdict
    return np.asarray(Image.open(out).convert("RGB")), verdict


def band_step(result, original_alpha, x0, y0, size):
    """The faint band's mean step against a ring 8-36 px outside the mark."""
    band = (original_alpha > 0.01) & (original_alpha < 0.2)
    sq = result[y0:y0 + size, x0:x0 + size].astype(np.float64)
    h, w, _ = result.shape
    ring = []
    for y in range(max(0, y0 - 36), min(h, y0 + size + 36)):
        for x in range(max(0, x0 - 36), min(w, x0 + size + 36)):
            dx = max(x0 - x, x - (x0 + size - 1), 0)
            dy = max(y0 - y, y - (y0 + size - 1), 0)
            if 8 <= max(dx, dy) <= 36:
                ring.append(result[y, x])
    ring = np.array(ring, dtype=np.float64)
    step = sq[band].mean(axis=0) - ring.mean(axis=0)
    return step, float(step @ LUMA), np.median(ring, axis=0)


def corner(img, x0, y0, size):
    h, w, _ = img.shape
    box = (max(0, x0 - PAD), max(0, y0 - PAD), min(w, x0 + size + PAD), min(h, y0 + size + PAD))
    return Image.fromarray(img).crop(box)


def zoomed(tile):
    return tile.resize((tile.width * ZOOM, tile.height * ZOOM), Image.NEAREST)


def deviation(img, background, x0, y0, size):
    tile = np.asarray(corner(img, x0, y0, size)).astype(np.float64)
    shown = np.clip((tile - background) * AMPLIFY + 128, 0, 255).astype(np.uint8)
    return zoomed(Image.fromarray(shown))


def font(size):
    for name in ("DejaVuSans.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "Arial.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            pass
    return ImageFont.load_default()


def sheet(name, cells, notes):
    """cells: 4 rows x 2 columns of images; notes: one text per column."""
    head, label_w, gap, note_h = 70, 260, 20, 70
    col_w = max(c.width for row in cells for c in row)
    row_h = [max(c.height for c in row) for row in cells]
    width = label_w + 2 * col_w + 3 * gap
    height = head + sum(row_h) + gap * (len(cells) + 1) + note_h + 40
    page = Image.new("RGB", (width, height), "white")
    draw = ImageDraw.Draw(page)
    big, small = font(30), font(20)
    draw.text((gap, 10), name, fill="black", font=small)
    for col, title in enumerate(("GeminiWatermarkTool (its algorithm)", "Wipemark (wipemark-cli clean)")):
        draw.text((label_w + gap + col * (col_w + gap), 30), title, fill="black", font=big)
    labels = ("1. original, x4", "2. result, x4", f"3. result: deviation\n   from around, x{AMPLIFY}", "4. result, 1:1\n   (the corner)")
    y = head + gap
    for r, row in enumerate(cells):
        draw.multiline_text((gap, y + 10), labels[r], fill="black", font=small)
        for c, cell in enumerate(row):
            page.paste(cell, (label_w + gap + c * (col_w + gap), y))
        y += row_h[r] + gap
        if r == 2:
            for c, note in enumerate(notes):
                draw.multiline_text((label_w + gap + c * (col_w + gap), y - gap + 4), note, fill="black", font=small)
            y += note_h
    return page


def describe(step, luma, verdict=None):
    text = f"band - around: R {step[0]:+.2f}  G {step[1]:+.2f}  B {step[2]:+.2f}  luma {luma:+.2f}"
    if verdict is None:
        return text + "\nverdict: none (GWT does not check)"
    if "error" in verdict:
        return text + f"\nexit {verdict['exit']}: {verdict['error'][0]}"
    said = [w for w in ("outline", "texture") if verdict.get(w)]
    left = "trace said: " + ", ".join(said) if said else "nothing left said"
    return text + f"\nexit {verdict['exit']}, {left}"


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("files", nargs="+")
    parser.add_argument("-o", "--out-dir", default=".")
    parser.add_argument("--cli", default=os.path.join(REPO, "target", "release", "wipemark-cli"))
    parser.add_argument("--gwt-out", help="a folder of results GWT itself wrote, same file names")
    args = parser.parse_args()
    if not os.access(args.cli, os.X_OK):
        sys.exit(f"no wipemark-cli at {args.cli}: cargo build --release -p wipemark-cli")
    os.makedirs(args.out_dir, exist_ok=True)
    with tempfile.TemporaryDirectory() as work:
        for path in args.files:
            original = np.asarray(Image.open(path).convert("RGB"))
            h, w, _ = original.shape
            x0, y0, size, map_name = gwt_place(w, h)
            alpha = gwt_alpha(map_name)
            if args.gwt_out:
                gwt = np.asarray(Image.open(os.path.join(args.gwt_out, os.path.basename(path))).convert("RGB"))
            else:
                gwt = gwt_remove(original)
            ours, verdict = wipemark_clean(args.cli, path, work)
            if ours is None:
                ours = original  # nothing written: show what is left, the input
            columns = []
            for result, v in ((gwt, None), (ours, verdict)):
                step, luma, background = band_step(result, alpha, x0, y0, size)
                columns.append((result, background, describe(step, luma, v)))
            first = zoomed(corner(original, x0, y0, size))
            cells = [
                [first, first],
                [zoomed(corner(r, x0, y0, size)) for r, _, _ in columns],
                [deviation(r, bg, x0, y0, size) for r, bg, _ in columns],
                [Image.fromarray(r).crop((max(0, w - 704), max(0, h - 704), w, h)) for r, _, _ in columns],
            ]
            name = os.path.splitext(os.path.basename(path))[0]
            out = os.path.join(args.out_dir, f"{name}-gwt-vs-wipemark.png")
            sheet(os.path.basename(path), cells, [c[2] for c in columns]).save(out)
            print(out, "|", columns[0][2].splitlines()[0], "|", columns[1][2].replace("\n", " | "))


if __name__ == "__main__":
    main()
