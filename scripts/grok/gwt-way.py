#!/usr/bin/env python3
"""Grok's mark taken off the way GeminiWatermarkTool does it — the inverse, then FDnCNN — measured by R10.

What it is for, and who asked
-----------------------------
The owner asked on 2026-10-11 to check how the restoration of Grok's mark
works with a model **the way GeminiWatermarkTool (allenk/GeminiWatermarkTool,
"GWT") does it**: reverse alpha blending with a known map, then an AI
denoiser (FDnCNN, GWT's `--denoise ai --sigma`) over what the inverse left —
using what this repository already has (R10's `scripts/model-eval/`, the
first look's provisional `grok-x-dark` / `grok-x-light` profiles, the bench
with `--catalogue`). The coordinator's task is Watchword FILE
`wipemark-task-recon-grok-fdncnn-2026-10-11` (N1–N5); the report is
`docs/plan/reports/E12-R10-grok-fdncnn-2026-10-11.md`. Nothing here is in the
product (D166, R10 §7): a pixel a model made is *reconstructed*, never
*restored* (D499).

What it does — one subcommand per step
--------------------------------------
* `weights` (N1): reads GWT's embedded network (`external/ncnn/model-convert/
  output/model_core.mem.h`, ncnn's binary param + FP16 weights) and KAIR's
  `fdncnn_color.pth`, and compares them layer by layer: the FP16 weights
  against KAIR's rounded to FP16 (equal share, relative difference where
  `|w| > 1e-3`, correlation, sign flips of values near zero), the FP32
  biases exactly. Says whether GWT's denoiser is KAIR's FDnCNN colour.
* `pictures` (N2): per real picture, at the row first-look measured
  (`look.json`'s place and width, the variant it assigned — logo 0 for dark,
  255 for light), the provisional catalogue's map for that size, **no
  proof** — as GWT writes whatever the formula gives:
  1. `gwt-inverse` — GWT's `remove_watermark_alpha_blend`
     (`src/core/blend_modes.cpp:33–108`): on RGB from Pillow's decode,
     `α < 0.002` untouched, `α` capped at 0.99,
     `O = (I − α·L)/(1 − α)` clamped to [0, 255] and rounded as OpenCV's
     `convertTo` rounds (half to even);
  2. `gwt-inverse+gwt-ai` — that, then GWT's `NcnnDenoiser::denoise`
     (`src/core/ai_denoise.cpp:381–496`) as its CLI runs `--denoise ai`
     (σ 50, strength 1.2, padding 32; `src/cli/cli_app.cpp:190–222, 533–539`):
     the weight mask from the map's Sobel gradient (normalised, square
     root, dilated by a 5 × 5 ellipse, Gaussian σ 2, × strength, cut at 1),
     embedded in the ROI padded by 32 and clipped to the picture, Gaussian
     σ 1 over it, FDnCNN over the padded ROI with a uniform σ/255 plane and
     no padding of its own, `out = w·den + (1 − w)·in` rounded;
  3. `gwt-inverse+gwt-ai-s15` — the same at σ 15 (GWT's README's lighter
     setting for illustration content);
  4. `gwt-inverse+r10` — R10 §2.3's way over the same inverse: the σ map
     `clamp(σ_base/(1 − α) + k_edge·|∇α|, σ_base, 75)`, the α mask
     `clamp(strength · smoothstep(0, 0.1, α), 0, 1)`, `k_edge 5, strength 1.0,
     rgb` unless `--r10` names another (`fdncnn_run.py`'s own functions);
  5. with `--product DIR` (first-look's `product --invert-dark` outputs:
     `<name>.inverted.cleaned.png`, inverted back, or `<name>.cleaned.jpeg`,
     only where its `product.json` says the mark was proved — a file written
     with the mark left is a re-encode, not a restoration, and is no column):
     the product's result, and the product's result + (4).
  Per column: the corner ×4 nearest, the deviation from the ring's quadratic
  ×10 (as `scripts/compare-gwt.py`'s third row), and the numbers — the faint
  band's step in R, G, B and luma against the ring's quadratic (the mean of
  `out − B̂` over the samples with `α` from 0.02 to a third of the map's
  peak), the core's step (`α` over two thirds of the peak), `texture /
  texture_around` (`evalkit.texture`), `consistency_px`
  (`evalkit.consistency_px`, against the stored picture with the map and
  logo used), the share of samples the inverse put out of range. One
  contact sheet of every picture. `B̂` is the quadratic fitted on the ring of
  the *output* (6 pixels wide, 2 outside the rectangle), so a trace is what
  is left against that output's own surroundings.
* `invert` (N3): the dark variant through a profile the product proposes.
  For a white logo, `α·255 + (1 − α)(255 − B) = 255 − (1 − α)·B`: a white
  mark over `255 − B` is the negative of a black mark over `B`, exactly on
  a PNG. Reads the tiles of a `recon_bench gen` run (the 512 × 512 corner of
  each background's `gt.png`, at the corner `tile_origin` puts it), writes
  their negatives as PNGs and a manifest with one `photos` group (`inv`)
  naming them, for `recon_bench pin --photos` and `gen --photos`: a photo
  tile at its own size is cut and resized 512 → 512, which is checked to be
  the identity on the first tile after `gen` (`invert --check`).
* `crops` (N3): GWT's two rules (σ 50 and σ 15, strength 1.2, padding 32)
  over bench crops (`recon_bench run --export-crops`), as two more variants
  beside `fdncnn_run.py`'s 18, measured by `fdncnn_run.measure` — the same
  record shape, so `verdict` reads both. A crop no restoration made is
  skipped (F7), a lossless one left as it is (F4).
* `verdict` (N4): from `fdncnn_run.py`'s `results.json` and this script's
  `crops` output, over R0's and R8d's crops: per slice and variant, the
  median and p5 of PSNR_ROI, SSIM and the texture ratio; the inverse alone
  (R0), R0 + GWT's AI, R8d, R8d + the best R10 variant, R8d + GWT's AI; and
  R10 §2.5's one line from `fdncnn_run.py`'s gates (against R8d, never R0).

How to run it
-------------
From the repository's root, with the venv R10's README describes (torch only
for `weights`, onnxruntime for `pictures` and `crops`):

    python3 scripts/grok/gwt-way.py weights --gwt <GWT checkout> --pth <fdncnn_color.pth> --out <json>
    python3 scripts/grok/gwt-way.py pictures fixtures/image/grok/x/*.jpeg \\
        --look reports/grok-first-look-2026-10-11/look.json \\
        --catalogue reports/grok-first-look-2026-10-11/catalogue/marks.json \\
        --model <fdncnn_color.onnx> --out reports/grok-fdncnn-<date>/pictures [--product DIR]
    python3 scripts/grok/gwt-way.py invert --run <gen run> --out <dir>        # then pin, gen
    python3 scripts/grok/gwt-way.py invert --check --run <gen run of the negatives> --tiles <dir>
    python3 scripts/grok/gwt-way.py crops --crops <run>/crops/R8d --model <onnx> --out <dir>
    python3 scripts/grok/gwt-way.py verdict --fdncnn <dir>[,<dir>…] --gwt <dir>[,<dir>…] --best <variant> \\
        --out <md> [--inv-tiles <invert's tiles>]
    python3 scripts/grok/gwt-way.py selftest

The whole sequence, with the bench, is `docs/plan/reports/
E12-R10-grok-fdncnn-2026-10-11.sh`.

What it needs
-------------
Python 3.10+, numpy, Pillow 12.3.0; onnxruntime for `pictures` and `crops`;
torch for `weights` (reading the `.pth`). `selftest` needs none of them: it
runs GWT's mask, the inverse, the invert identity and the band measure on
synthetic data with a stub denoiser.

What its output means
---------------------
`weights`: a JSON and one printed line — "GWT's denoiser is KAIR's FDnCNN
colour in FP16" or what differs. `pictures`: `<out>/pictures.json` (every
picture × column), `<out>/pictures.md`, `<out>/<name>-x4.png` (before | each
column), `<out>/<name>-dev10.png` (the deviation ×10, grey = 0),
`<out>/contact-sheet.png`. `invert`: the negatives and `manifest.json`.
`crops`: `<out>/results.json`, the records `verdict` reads. `verdict`: one
Markdown file. Exit 0 when it ran, 2 on a refusal. The pictures, crops, maps
and weights stay under the ignored `reports/`.
"""

import argparse
import hashlib
import json
import math
import os
import re
import sys
import time

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "model-eval"))
import evalkit as ek  # noqa: E402
import fdncnn_run as fr  # noqa: E402

# ── GWT's constants, with where they are (GWT at 7c6a99f, its main on 2026-10-11) ──
GWT_ALPHA_THRESHOLD = 0.002   # blend_modes.cpp:76
GWT_MAX_ALPHA = 0.99          # blend_modes.cpp:77
GWT_SIGMA = 50.0              # cli_app.cpp:216 (--denoise ai), app_state.hpp:218 (GUI)
GWT_STRENGTH = 1.2            # cli_app.cpp:215, app_state.hpp:216
GWT_PADDING = 32              # cli_app.cpp:539, app_state.hpp:224 (kPadding)
GWT_SIGMA_LIGHT = 15.0        # README: "--sigma 15 for anime / illustration content"
GWT_RULES = (("gwt-ai", GWT_SIGMA, GWT_STRENGTH), ("gwt-ai-s15", GWT_SIGMA_LIGHT, GWT_STRENGTH))
R10_DEFAULT = (5, 1.0, "rgb")  # the task's choice before N3 (k_edge 5, strength 1.0)
RING, GUARD = 6, 2             # first-look's ring (calibrate's, 2 pixels out)
BAND_LOW = 0.02                # the faint band: α from here to a third of the peak
ZOOM = 4
CROP_MARGIN = 24


def refuse(msg):
    print(f"gwt-way: {msg}", file=sys.stderr)
    sys.exit(2)


# ───────────────────────────────────────────────────── OpenCV, restated in numpy

def reflect101(a, p):
    """OpenCV's default border (BORDER_REFLECT_101) by `p` pixels."""
    return np.pad(a, [(p, p), (p, p)] + [(0, 0)] * (a.ndim - 2), mode="reflect")


def correlate(a, kernel):
    """2-D correlation with BORDER_REFLECT_101, same size (cv::filter2D)."""
    kh, kw = kernel.shape
    p = max(kh, kw) // 2
    ap = reflect101(a, p)
    out = np.zeros_like(a, dtype=np.float64)
    for dy in range(kh):
        for dx in range(kw):
            k = kernel[dy, dx]
            if k:
                out += k * ap[p - kh // 2 + dy: p - kh // 2 + dy + a.shape[0],
                              p - kw // 2 + dx: p - kw // 2 + dx + a.shape[1]]
    return out


def gaussian_kernel(sigma):
    """cv::getGaussianKernel for CV_32F with ksize 0: ksize = round(σ·4·2 + 1) | 1."""
    n = int(round(sigma * 4 * 2 + 1)) | 1
    x = np.arange(n) - (n - 1) / 2
    k = np.exp(-(x ** 2) / (2 * sigma ** 2))
    return k / k.sum()


def gaussian_blur(a, sigma):
    k = gaussian_kernel(sigma)
    return correlate(correlate(a, k[None, :]), k[:, None])


def sobel(a):
    kx = np.array([[-1, 0, 1], [-2, 0, 2], [-1, 0, 1]], dtype=np.float64)
    return correlate(a, kx), correlate(a, kx.T)


def ellipse5():
    """cv::getStructuringElement(MORPH_ELLIPSE, 5 × 5)."""
    return np.array([[0, 0, 1, 0, 0], [1, 1, 1, 1, 1], [1, 1, 1, 1, 1], [1, 1, 1, 1, 1], [0, 0, 1, 0, 0]], bool)


def dilate(a, element):
    """cv::dilate with its default border (the border never wins a max)."""
    kh, kw = element.shape
    p = kh // 2
    ap = np.pad(a, p, mode="constant", constant_values=-np.inf)
    out = np.full_like(a, -np.inf, dtype=np.float64)
    for dy in range(kh):
        for dx in range(kw):
            if element[dy, dx]:
                out = np.maximum(out, ap[dy:dy + a.shape[0], dx:dx + a.shape[1]])
    return out


def cv_round(x):
    """cv::saturate_cast<uchar> of a float: round half to even, then clamp."""
    return np.clip(np.rint(x), 0, 255).astype(np.uint8)


# ───────────────────────────────────────────────────── GWT's two steps

def gwt_inverse(img, alpha, rect, logo):
    """`remove_watermark_alpha_blend`: `img` (H, W, 3) uint8, `alpha` the map
    over `rect` = (x, y, w, h). Returns (out, out-of-range share)."""
    out = img.copy()
    x, y, w, h = rect
    a = np.asarray(alpha, dtype=np.float64)
    live = a >= GWT_ALPHA_THRESHOLD
    ac = np.minimum(a, GWT_MAX_ALPHA)[..., None]
    region = img[y:y + h, x:x + w].astype(np.float64)
    o = (region - ac * np.asarray(logo, dtype=np.float64)) / (1.0 - ac)
    clipped = ((o < 0) | (o > 255)) & live[..., None]
    o = np.clip(o, 0, 255)
    res = np.where(live[..., None], cv_round(o), img[y:y + h, x:x + w])
    out[y:y + h, x:x + w] = res
    n = int(live.sum()) * 3
    return out, (float(clipped.sum()) / n if n else 0.0)


def gwt_weight(alpha, strength):
    """`compute_gradient_mask` (ai_denoise.cpp:270–311) over a map already at the region's size."""
    gx, gy = sobel(np.asarray(alpha, dtype=np.float64))
    mag = np.sqrt(gx * gx + gy * gy)
    lo, hi = mag.min(), mag.max()
    if hi <= lo:
        return np.zeros_like(mag)
    g = np.sqrt((mag - lo) / (hi - lo))
    g = dilate(g, ellipse5())
    g = gaussian_blur(g, 2.0)
    return np.minimum(g * strength, 1.0)


def gwt_denoise(img, alpha, rect, denoiser, sigma, strength, padding=GWT_PADDING):
    """`NcnnDenoiser::denoise` (ai_denoise.cpp:381–496). Returns (out, weight over the padded ROI, padded ROI)."""
    x, y, w, h = rect
    H, W = img.shape[:2]
    wgt = gwt_weight(alpha, strength)
    if not np.any(wgt > 0.01):
        return img.copy(), None, None
    px0, py0 = max(0, x - padding), max(0, y - padding)
    px1, py1 = min(W, x + w + padding), min(H, y + h + padding)
    roi = img[py0:py1, px0:px1]
    weight = np.zeros(roi.shape[:2])
    ix, iy = x - px0, y - py0
    weight[iy:iy + h, ix:ix + w] = wgt[:min(h, roi.shape[0] - iy), :min(w, roi.shape[1] - ix)]
    weight = gaussian_blur(weight, 1.0)
    rgb = roi.astype(np.float32) / 255.0
    den = np.asarray(denoiser(rgb, np.full(roi.shape[:2], sigma, dtype=np.float32)), dtype=np.float64)
    den = cv_round(np.clip(den, 0.0, 1.0) * 255.0).astype(np.float64)
    res = cv_round(roi.astype(np.float64) * (1.0 - weight[..., None]) + den * weight[..., None])
    out = img.copy()
    out[py0:py1, px0:px1] = res
    return out, weight, (px0, py0, px1, py1)


def r10_denoise(img, alpha, rect, denoiser, sigma_base, k_edge, strength, space, pad=64):
    """R10 §2.3 over a window (rect ± `pad`, clipped): `fdncnn_run.denoised` + `compose`."""
    x, y, w, h = rect
    H, W = img.shape[:2]
    x0, y0, x1, y1 = max(0, x - pad), max(0, y - pad), min(W, x + w + pad), min(H, y + h + pad)
    win = img[y0:y1, x0:x1]
    a = np.zeros(win.shape[:2])
    a[y - y0:y - y0 + h, x - x0:x - x0 + w] = alpha
    den = fr.denoised(denoiser, win, a, sigma_base, k_edge, space)
    out = img.copy()
    out[y0:y1, x0:x1] = fr.compose(win, den, a, strength)
    return out


# ───────────────────────────────────────────────────── measures on a picture

def basis(u, v):
    return np.stack([np.ones_like(u), u, v, u * u, u * v, v * v], -1)


def ring_quadratic(img, rect, ring=RING, guard=GUARD):
    """The ring's quadratic per channel (first-look's `ring_fit`): `B̂` over the
    rectangle and the ring, and the ring's mask in that window."""
    x, y, w, h = rect
    H, W = img.shape[:2]
    X0, Y0 = max(0, x - guard - ring), max(0, y - guard - ring)
    X1, Y1 = min(W, x + w + guard + ring), min(H, y + h + guard + ring)
    sub = img[Y0:Y1, X0:X1].astype(np.float64)
    yy, xx = np.mgrid[Y0:Y1, X0:X1]
    s = max(w, h)
    cx, cy = x + w / 2, y + h / 2
    A = basis((xx + 0.5 - cx) / s, (yy + 0.5 - cy) / s)
    inner = (xx >= x - guard) & (xx < x + w + guard) & (yy >= y - guard) & (yy < y + h + guard)
    bg = np.zeros_like(sub)
    for c in range(3):
        k, *_ = np.linalg.lstsq(A[~inner], sub[..., c][~inner], rcond=None)
        bg[..., c] = A @ k
    return bg, (X0, Y0, X1, Y1), ~inner


def luma(rgb):
    rgb = np.asarray(rgb, dtype=np.float64)
    return rgb[..., 0] * 0.299 + rgb[..., 1] * 0.587 + rgb[..., 2] * 0.114


def steps(out, alpha, rect):
    """The faint band's and the core's step against the output's own ring
    quadratic: mean of `out − B̂` in R, G, B and luma."""
    x, y, w, h = rect
    bg, (X0, Y0, _, _), _ = ring_quadratic(out, rect)
    d = out[y:y + h, x:x + w].astype(np.float64) - bg[y - Y0:y - Y0 + h, x - X0:x - X0 + w]
    peak = float(np.max(alpha))
    band = (alpha >= BAND_LOW) & (alpha < peak / 3)
    core = alpha >= 2 * peak / 3

    def one(m):
        if not m.any():
            return None
        v = d[m]
        return [float(v[:, c].mean()) for c in range(3)] + [float(luma(v).mean())]
    return {"band": one(band), "core": one(core), "band_n": int(band.sum()), "core_n": int(core.sum())}


def picture_measures(out, stored, alpha, rect, logo, opaque=ek.OPAQUE_DEFAULT):
    """Everything N2 says of one column, over a window around the mark."""
    x, y, w, h = rect
    H, W = stored.shape[:2]
    pad = 12
    x0, y0, x1, y1 = max(0, x - pad), max(0, y - pad), min(W, x + w + pad), min(H, y + h + pad)
    a = np.zeros((y1 - y0, x1 - x0))
    a[y - y0:y - y0 + h, x - x0:x - x0 + w] = alpha
    o, s = out[y0:y1, x0:x1], stored[y0:y1, x0:x1]
    r = (x - x0, y - y0, x - x0 + w, y - y0 + h)
    tex, around = ek.texture(o, a, r, opaque)
    cons, excl = ek.consistency_px(o, s, a, logo, opaque)
    m = steps(out, alpha, rect)
    m.update(texture=tex, texture_around=around, ratio=(tex / around if around else None),
             consistency_px=cons, consistency_excluded=excl,
             changed=int(np.any(out != stored, axis=-1).sum()))
    return m


# ───────────────────────────────────────────────────── the catalogue and look.json

def read_wma(path):
    """A `.wma` opacity map (`crates/wipemark-pixels/src/alpha.rs`: `WMA1`, width and height as u16 LE,
    the depth in one byte, then the samples — u16 LE / 65535 at depth 16, u8 / 255 at depth 8)."""
    b = open(path, "rb").read()
    if b[:4] != b"WMA1":
        refuse(f"{path}: not a .wma")
    w, h = int.from_bytes(b[4:6], "little"), int.from_bytes(b[6:8], "little")
    depth = b[8]
    if depth == 16:
        return np.frombuffer(b[9:9 + 2 * w * h], dtype="<u2").reshape(h, w).astype(np.float64) / 65535.0
    if depth == 8:
        return np.frombuffer(b[9:9 + w * h], dtype="u1").reshape(h, w).astype(np.float64) / 255.0
    refuse(f"{path}: depth {depth}")


def placement(catalogue_path, profile, size):
    """(map, rect) of `profile`'s placement for `size` in a catalogue file."""
    cat = json.load(open(catalogue_path))
    p = next((p for p in cat["profiles"] if p["id"] == profile), None)
    if p is None:
        refuse(f"{catalogue_path}: no profile {profile}")
    pl = next((pl for pl in p["placements"] if pl.get("when", {}).get("width") == size[0]
               and pl.get("when", {}).get("height") == size[1]), None)
    if pl is None:
        refuse(f"{profile}: no placement for {size[0]}x{size[1]}")
    a = next(a for a in p["alpha"] if a["id"] == pl["alpha"])
    path = os.path.join(os.path.dirname(catalogue_path), a["asset"])
    if hashlib.sha256(open(path, "rb").read()).hexdigest() != a["sha256"]:
        refuse(f"{path}: sha256 is not the catalogue's")
    amap = read_wma(path)
    mw, mh = a["size"]
    if pl.get("corner") != "bottom-right":
        refuse(f"{profile}: only a bottom-right corner is read here")
    mr, mb = pl["margin"]
    rect = (size[0] - mr - mw, size[1] - mb - mh, mw, mh)
    return amap, rect, [float(v) for v in p["blend"]["logo"]], float(p.get("opaque_above", ek.OPAQUE_DEFAULT))


def name_of(path):
    return os.path.splitext(os.path.basename(path))[0]


# ───────────────────────────────────────────────────── pictures (N2)

def corner_strip(images, rect, zoom=ZOOM, margin=CROP_MARGIN, gap=4):
    x, y, w, h = rect
    H, W = images[0].shape[:2]
    x0, y0, x1, y1 = max(0, x - margin), max(0, y - margin), min(W, x + w + margin), min(H, y + h + margin)
    tiles = [np.repeat(np.repeat(im[y0:y1, x0:x1], zoom, 0), zoom, 1) for im in images]
    th = tiles[0].shape[0]
    sep = np.full((th, gap, 3), 255, np.uint8)
    row = []
    for t in tiles:
        row += [t, sep]
    return np.concatenate(row[:-1], axis=1)


def deviation_strip(images, rect, zoom=ZOOM, margin=CROP_MARGIN, gain=10.0, gap=4):
    """Each image's deviation from its own ring quadratic ×`gain` (luma), grey 128 = 0."""
    x, y, w, h = rect
    tiles = []
    for im in images:
        bg, (X0, Y0, X1, Y1), _ = ring_quadratic(im, rect, ring=margin - GUARD, guard=GUARD)
        d = luma(im[Y0:Y1, X0:X1]) - luma(bg)
        t = np.clip(128 + gain * d, 0, 255).astype(np.uint8)
        t = np.repeat(np.repeat(t, zoom, 0), zoom, 1)
        tiles.append(np.stack([t] * 3, -1))
    th = tiles[0].shape[0]
    sep = np.full((th, gap, 3), 255, np.uint8)
    row = []
    for t in tiles:
        row += [t, sep]
    return np.concatenate(row[:-1], axis=1)


def cmd_pictures(args):
    look = json.load(open(args.look))
    by_name = {p["name"]: p for p in look["pictures"]}
    model = json.load(open(args.model + ".json")) if os.path.exists(args.model + ".json") else {}
    den = fr.OnnxDenoiser(args.model, threads=args.threads)
    k_edge, strength, space = R10_DEFAULT
    if args.r10:
        m = re.fullmatch(r"k(\d+)-s([\d.]+)-(rgb|luma)", args.r10)
        if not m:
            refuse(f"--r10 {args.r10}: not k<k>-s<strength>-<rgb|luma>")
        k_edge, strength, space = int(m[1]), float(m[2]), m[3]
    r10_name = f"r10:k{k_edge}-s{strength}-{space}"
    os.makedirs(args.out, exist_ok=True)
    proved = product_proved(args.product) if args.product else {}
    results, sheet_rows = [], []
    for path in args.pictures:
        name = name_of(path)
        info = by_name.get(name)
        if info is None:
            refuse(f"{name}: not in {args.look}")
        img = np.asarray(Image.open(path).convert("RGB"))
        H, W = img.shape[:2]
        variant = info["variant"]["variant"]
        profile = f"grok-x-{variant}"
        amap, rect, logo, opaque = placement(args.catalogue, profile, (W, H))
        sigma = float(max(ek.sigma_base(img, (rect[0], rect[1], rect[0] + rect[2], rect[1] + rect[3]))))
        cols = [("input", img, None)]
        inv, clip = gwt_inverse(img, amap, rect, logo)
        cols.append(("gwt-inverse", inv, clip))
        for rule, sg, st in GWT_RULES:
            out, _, _ = gwt_denoise(inv, amap, rect, den, sg, st)
            cols.append((f"gwt-inverse+{rule}", out, None))
        cols.append((f"gwt-inverse+{r10_name}", r10_denoise(inv, amap, rect, den, sigma, k_edge, strength, space), None))
        if args.product:
            for cand in (f"{name}.inverted.cleaned.png", f"{name}.cleaned.jpeg"):
                p = os.path.join(args.product, cand)
                if os.path.exists(p) and proved.get((name, ".inverted." in cand)):
                    prod = np.asarray(Image.open(p).convert("RGB"))
                    if prod.shape != img.shape:
                        continue
                    if ".inverted." in cand:
                        prod = 255 - prod
                    if np.array_equal(prod, img):
                        break
                    cols.append(("product", prod, None))
                    cols.append((f"product+{r10_name}", r10_denoise(prod, amap, rect, den, sigma, k_edge, strength, space), None))
                    break
        rec = {"picture": name, "sha256": info["sha256"], "size": [W, H], "variant": variant, "profile": profile,
               "rect": list(rect), "logo": logo, "sigma_base": sigma, "columns": []}
        for label, out, clip in cols:
            m = picture_measures(out, img, amap, rect, logo, opaque)
            if clip is not None:
                m["out_of_range_share"] = clip
            rec["columns"].append(dict(column=label, **m))
        results.append(rec)
        strip = corner_strip([c[1] for c in cols], rect)
        Image.fromarray(strip).save(os.path.join(args.out, f"{name}-x4.png"))
        dev = deviation_strip([c[1] for c in cols], rect)
        Image.fromarray(dev).save(os.path.join(args.out, f"{name}-dev10.png"))
        sheet_rows.append((name, [c[0] for c in cols], strip, dev))
        print(f"{name}: {variant}, {len(cols) - 1} column(s)")
    sheet = contact_sheet(sheet_rows)
    Image.fromarray(sheet).save(os.path.join(args.out, "contact-sheet.png"))
    out = {"date": args.date, "model": model, "r10": r10_name, "gwt_rules": [list(r) for r in GWT_RULES],
           "environment": ek.environment(), "pictures": results}
    ek.write_json(os.path.join(args.out, "pictures.json"), out)
    with open(os.path.join(args.out, "pictures.md"), "w") as f:
        f.write(pictures_md(out))
    print(f"wrote {args.out}/pictures.json, pictures.md, contact-sheet.png")


def product_proved(product_dir):
    """{(picture, inverted): True} for every file first-look's `product` saw proved
    (`verified`) — read from the `product.json` beside its `product/` folder. A
    written file the product did not prove is a re-encode, not a restoration, and
    is no column."""
    path = os.path.join(os.path.dirname(os.path.normpath(product_dir)), "product.json")
    out = {}
    for r in json.load(open(path)):
        vis = (((r.get("clean") or {}).get("report") or {}).get("visible") or {})
        if any(f.get("verdict") == "verified" for f in vis.get("found") or []):
            out[(r["name"].split(" ")[0], bool(r.get("inverted")))] = True
    return out


def contact_sheet(rows, width=None):
    """Every picture's two strips, one under the other, left-aligned, on white."""
    parts = []
    for _name, _labels, strip, dev in rows:
        parts += [strip, np.full((4, strip.shape[1], 3), 255, np.uint8), dev,
                  np.full((16, strip.shape[1], 3), 255, np.uint8)]
    width = max(p.shape[1] for p in parts)
    padded = [np.concatenate([p, np.full((p.shape[0], width - p.shape[1], 3), 255, np.uint8)], 1) if p.shape[1] < width else p
              for p in parts]
    return np.concatenate(padded, 0)


def f3(v, n=2):
    return "–" if v is None else (f"{v:.{n}f}" if isinstance(v, float) else str(v))


def pictures_md(out):
    lines = ["# N2 — the five pictures, GWT's way", "",
             f"Model: weights sha256 `{out['model'].get('weights_sha256', '?')}`, ONNX `{out['model'].get('onnx_sha256', '?')}`. "
             f"GWT's rules: {', '.join(f'{r[0]} (σ {r[1]:g}, strength {r[2]:g})' for r in out['gwt_rules'])}; R10's: {out['r10']}.", ""]
    for p in out["pictures"]:
        lines += [f"## {p['picture']} — {p['size'][0]}×{p['size'][1]}, {p['variant']} (logo {p['logo'][0]:g}), "
                  f"rect {p['rect']}, σ_base {p['sigma_base']:.2f}", "",
                  "| column | band step R, G, B, Y | core step Y | texture / around | consistency_px | out of range | changed px |",
                  "|---|---|---|---|---|---|---|"]
        for c in p["columns"]:
            band = c["band"]
            core = c["core"]
            lines.append(f"| {c['column']} | {', '.join(f'{v:+.1f}' for v in band) if band else '–'} | "
                         f"{f'{core[3]:+.1f}' if core else '–'} | {f3(c['texture'], 1)} / {f3(c['texture_around'], 1)} = {f3(c['ratio'])} | "
                         f"{f3(c['consistency_px'])} | {f3(c.get('out_of_range_share'), 4)} | {c['changed']} |")
        lines.append("")
    return "\n".join(lines) + "\n"


# ───────────────────────────────────────────────────── invert (N3)

def tile_origin(mark, w, h, side):
    """recon_bench.rs `tile_origin`: twice the box's centre against the canvas."""
    x, y, mw, mh = mark
    cx, cy = 2 * x + mw, 2 * y + mh
    return (0 if cx < w else w - side, 0 if cy < h else h - side)


def cmd_invert(args):
    run = args.run
    manifest = json.load(open(os.path.join(run, "manifest.json")))
    side = int(manifest.get("tile", 512))
    index = [json.loads(line) for line in open(os.path.join(run, "index.jsonl")) if line.strip()]
    if args.check:
        return invert_check(args, index, side)
    os.makedirs(args.out, exist_ok=True)
    tiles_dir = os.path.join(args.out, "tiles")
    os.makedirs(tiles_dir, exist_ok=True)
    done = {}
    for line in index:
        bg = line["background"]
        if bg in done:
            continue
        meta = json.load(open(os.path.join(run, line["case_dir"], "meta.json")))
        w, h = meta["size"]
        p = meta["pixels"]
        ox, oy = tile_origin((p["x"], p["y"], p["width"], p["height"]), w, h, side)
        gt = np.asarray(Image.open(os.path.join(run, line["bg_dir"], "gt.png")).convert("RGB"))
        tile = gt[oy:oy + side, ox:ox + side]
        Image.fromarray(255 - tile).save(os.path.join(tiles_dir, f"{bg}.png"))
        done[bg] = {"from": line["bg_dir"], "origin": [ox, oy], "sha256_of_tile": hashlib.sha256(tile.tobytes()).hexdigest()}
    src = json.load(open(args.manifest))
    derived = {k: v for k, v in src.items() if k not in ("groups", "backgrounds")}
    derived["about"] = ("The negatives (255 − B) of a recon_bench run's tiles, as one photos group, for the dark "
                        "variant through a white-logo profile (scripts/grok/gwt-way.py invert). Derived; not committed.")
    derived["groups"] = [{"id": "inv", "source": "photos", "files": [], "count": len(done),
                          "about": f"negatives of {run}'s {len(done)} tiles"}]
    derived["backgrounds"] = []
    ek.write_json(os.path.join(args.out, "manifest.json"), derived)
    ek.write_json(os.path.join(args.out, "tiles.json"), done)
    print(f"{len(done)} negative tile(s) → {tiles_dir}; manifest {args.out}/manifest.json (pin it with --photos {tiles_dir})")


def invert_check(args, index, side):
    """A tile of the negatives' run is the negative written, byte for byte (the photo route's resize is the identity)."""
    tiles = args.tiles
    checked = bad = 0
    seen = set()
    for line in index:
        meta = json.load(open(os.path.join(args.run, line["case_dir"], "meta.json")))
        key = (line["bg_dir"])
        if key in seen:
            continue
        seen.add(key)
        w, h = meta["size"]
        p = meta["pixels"]
        ox, oy = tile_origin((p["x"], p["y"], p["width"], p["height"]), w, h, side)
        gt = np.asarray(Image.open(os.path.join(args.run, line["bg_dir"], "gt.png")).convert("RGB"))
        files = sorted(f for f in os.listdir(tiles) if f.endswith(".png"))
        idx = int(line["background"].rsplit("-", 1)[1])
        want = np.asarray(Image.open(os.path.join(tiles, files[idx])).convert("RGB"))
        checked += 1
        if not np.array_equal(gt[oy:oy + side, ox:ox + side], want):
            bad += 1
    print(f"invert --check: {checked} canvas(es), {bad} whose tile is not the negative written")
    return 0 if bad == 0 else 2


# ───────────────────────────────────────────────────── crops (N3)

def gwt_on_crop(crop, denoiser, sigma, strength):
    x0, y0, x1, y1 = crop.rect_px
    rect = (x0, y0, x1 - x0, y1 - y0)
    alpha = crop.alpha[y0:y1, x0:x1]
    out, _, _ = gwt_denoise(crop.recon, alpha, rect, denoiser, sigma, strength)
    return out


def cmd_crops(args):
    roots = ek.load_crops(args.crops)
    if not roots:
        refuse(f"{args.crops}: no crop")
    den = fr.OnnxDenoiser(args.model, threads=args.threads)
    model = json.load(open(args.model + ".json")) if os.path.exists(args.model + ".json") else {}
    recs, t0 = [], time.monotonic()
    for i, root in enumerate(roots):
        crop = ek.Crop(root)
        base = {"crop": crop.name, "dir": crop.dir, "slice": crop.slice, "encoder": crop.meta.get("encoder"),
                "group": crop.group, "config": crop.meta.get("config"),
                "blend": (crop.meta.get("blend") or {}).get("model"), "key": list(crop.key())}
        if not crop.restored:
            recs.append(dict(base, variant=None, applied=False, why="no restoration (F7)"))
            continue
        sigma = crop.sigma_base()
        if not crop.lossy:
            recs.append(dict(base, variant="R*", applied=False, sigma_base=sigma, why="lossless (F4)",
                             **fr.measure(crop, crop.recon, sigma)))
            continue
        for rule, sg, st in GWT_RULES:
            out = gwt_on_crop(crop, den, sg, st)
            recs.append(dict(base, variant=rule, applied=True, sigma=sg, strength=st, sigma_base=sigma,
                             **fr.measure(crop, out, sigma)))
        if (i + 1) % 200 == 0:
            print(f"{i + 1}/{len(roots)} crops, {time.monotonic() - t0:.0f} s")
    os.makedirs(args.out, exist_ok=True)
    ek.write_json(os.path.join(args.out, "results.json"),
                  {"model": model, "rules": [list(r) for r in GWT_RULES], "crops": args.crops,
                   "environment": ek.environment(), "seconds": time.monotonic() - t0, "records": recs})
    print(f"{len(roots)} crop(s), {sum(1 for r in recs if r.get('applied'))} GWT output(s) → {args.out}/results.json")


# ───────────────────────────────────────────────────── verdict (N4)

def load_records(dirs):
    out = []
    for d in dirs.split(","):
        with open(os.path.join(d, "results.json")) as f:
            r = json.load(f)
        out += r["records"] if isinstance(r, dict) and "records" in r else r
    return out


def cmd_verdict(args):
    fd = load_records(args.fdncnn)
    gw = load_records(args.gwt)
    if args.inv_tiles:
        # A run of negatives (`invert`) has one group, `inv`; its backgrounds are the
        # tiles in name order, so `inv-007` is the eighth file — `flat-…` or `text-…`.
        # Give each record its source's group back, so F2 (text) reads it.
        files = sorted(f for f in os.listdir(args.inv_tiles) if f.endswith(".png"))
        for r in fd + gw:
            m = re.match(r"inv-(\d+)__", r.get("crop") or "")
            if m:
                r["group"] = files[int(m[1])].split("-")[0]
    rows = {}

    def add(label, rec):
        if rec.get("psnr_roi") is None:
            return
        key = (rec.get("slice"), label)
        rows.setdefault(key, []).append(rec)

    best = args.best
    for r in fd:
        cfg = r.get("config")
        if r.get("variant") == "R*":
            add(f"{cfg} (alone)", r)
        elif r.get("variant") == best:
            add(f"{cfg} + R10 {best}", r)
    for r in gw:
        if r.get("applied"):
            add(f"{r.get('config')} + GWT {r['variant']}", r)
    slices = sorted({k[0] for k in rows})
    labels = sorted({k[1] for k in rows})
    lines = [f"# N4 — GWT's way against ours, with a truth", "",
             f"Over the bench's crops (Grok's provisional profile, light and the negatives). R10's best variant: `{best}`.", "",
             "| slice | road | n | PSNR_ROI median | p5 | SSIM median | texture ratio median | ratio in [0.8, 1.2] |",
             "|---|---|---|---|---|---|---|---|"]
    for s in slices:
        for lab in labels:
            recs = rows.get((s, lab))
            if not recs:
                continue
            ps = [r["psnr_roi"] for r in recs]
            ss = [r["ssim_roi"] for r in recs]
            rt = [r["ratio"] for r in recs if r.get("ratio") is not None]
            inside = sum(1 for v in rt if 0.8 <= v <= 1.2)
            lines.append(f"| {s} | {lab} | {len(recs)} | {ek.median(ps):.2f} | {ek.p5(ps):.2f} | {ek.median(ss):.4f} | "
                         f"{(ek.median(rt) if rt else float('nan')):.2f} | {inside}/{len(rt)} |")
    # R10 §2.4's gates over GWT's two rules, as over FDnCNN's 18 variants: against
    # R* of the same crops — R8d (the product's road) first, R0 (the inverse alone,
    # GWT's own road) beside it for the record. F6 (the blind A/B) stays pending.
    names = [r[0] for r in GWT_RULES] + [best]
    for cfg in ("R8d", "R0"):
        recs = [r for r in fd if r.get("config") == cfg and r.get("variant") in ("R*", best, None)]
        recs += [r for r in gw if r.get("config") == cfg and r.get("applied")]
        if not any(r.get("variant") == "R*" for r in recs):
            continue
        g = fr.gates(recs, names)
        lines += ["", f"## R10 §2.4 against {cfg}", "",
                  "| variant | F1 | median gain dB (per slice) | p5 Δ (per slice) | F2 (text) | F3 share in [0.8, 1.2] | F5 share > 2h | verdict |",
                  "|---|---|---|---|---|---|---|---|"]
        for n in names:
            x = g[n]
            f1 = x["F1"]["slices"]
            ok = (x["F1"]["ok"] and x["F2"]["ok"] is not False and x["F3"]["ok"] and x["F4"]["ok"] and x["F7"]["ok"]
                  and not x["F5"]["marked"])
            lines.append(f"| {n} | {'yes' if x['F1']['ok'] else 'no'} | "
                         f"{', '.join(f'{v['slice']} {v['median_delta']:+.2f}' for v in f1)} | "
                         f"{', '.join(f'{v['p5_delta']:+.2f}' for v in f1)} | "
                         f"{'–' if x['F2']['ok'] is None else ('yes' if x['F2']['ok'] else 'no')} | "
                         f"{(x['F3']['share'] or 0):.3f} | {(x['F5']['share_over_2h'] or 0):.3f} | "
                         f"{'passes F1–F5, F7 (F6 pending)' if ok else 'fails'} |")
        lines += ["", f"Closing line by R10 §2.5 over these three against {cfg}: " + fr.closing_line(g).replace("FDnCNN", "the model")]
    with open(args.out, "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"wrote {args.out}")


# ───────────────────────────────────────────────────── weights (N1)

def ncnn_fp16_bin(mem_h):
    src = open(mem_h).read()
    i = src.index("fp16_bin[] = {")
    body = src[i:src.index("};", i)]
    return bytes(int(x, 16) for x in re.findall(r"0x([0-9a-fA-F]{2})", body))


def cmd_weights(args):
    import torch

    mem_h = os.path.join(args.gwt, "external", "ncnn", "model-convert", "output", "model_core.mem.h")
    b = ncnn_fp16_bin(mem_h)
    sd = torch.load(args.pth, map_location="cpu")
    off, layers = 0, []
    G, K = [], []
    for wk in [k for k in sd if k.endswith("weight")]:
        w = sd[wk].numpy().ravel()
        tag = int.from_bytes(b[off:off + 4], "little")
        off += 4
        g = np.frombuffer(b[off:off + 2 * w.size], dtype=np.float16)
        off += 2 * w.size
        bias = sd[wk[:-6] + "bias"].numpy()
        gb = np.frombuffer(b[off:off + 4 * bias.size], dtype=np.float32)
        off += 4 * bias.size
        k16 = w.astype(np.float16)
        layers.append({"layer": wk, "n": int(w.size), "tag": hex(tag),
                       "equal_fp16": int((g.view(np.int16) == k16.view(np.int16)).sum()),
                       "bias_max_abs_diff": float(np.abs(gb - bias).max())})
        G.append(g.astype(np.float64))
        K.append(w.astype(np.float64))
    G, K = np.concatenate(G), np.concatenate(K)
    big = np.abs(K) > 1e-3
    rel = np.abs(G - K)[big] / np.abs(K[big])
    out = {"gwt": args.gwt, "gwt_commit": args.gwt_commit, "mem_h": os.path.relpath(mem_h, args.gwt),
           "mem_h_sha256": ek.sha256_file(mem_h), "fp16_bin_bytes": len(b), "consumed": off,
           "pth": args.pth, "pth_sha256": ek.sha256_file(args.pth), "layers": layers,
           "parameters": int(K.size + sum(sd[k].numel() for k in sd if k.endswith("bias"))),
           "weights_equal_fp16": int(sum(l["equal_fp16"] for l in layers)), "weights": int(K.size),
           "max_abs_diff": float(np.abs(G - K).max()), "max_rel_diff_where_abs_over_1e-3": float(rel.max()),
           "correlation": float(np.corrcoef(G, K)[0, 1]),
           "sign_flips": int((np.sign(G) != np.sign(K)).sum()),
           "sign_flips_max_abs": float(np.abs(K[np.sign(G) != np.sign(K)]).max()) if (np.sign(G) != np.sign(K)).any() else 0.0,
           "bias_max_abs_diff": float(max(l["bias_max_abs_diff"] for l in layers))}
    same = (off == len(b) and out["max_rel_diff_where_abs_over_1e-3"] < 2 ** -10 and out["bias_max_abs_diff"] == 0.0)
    out["verdict"] = ("GWT's denoiser is KAIR's FDnCNN colour in FP16" if same
                      else "GWT's denoiser differs from KAIR's FDnCNN colour")
    ek.write_json(args.out, out)
    print(f"{out['verdict']}: {out['weights']} weights, max rel diff {out['max_rel_diff_where_abs_over_1e-3']:.2e} "
          f"where |w| > 1e-3, correlation {out['correlation']:.9f}, biases equal: {out['bias_max_abs_diff'] == 0.0}")


# ───────────────────────────────────────────────────── selftest

def _stub(rgb, sigma):
    """A stand-in denoiser: a 3 × 3 box blur, so a test sees where the output moved."""
    out = np.empty_like(rgb)
    for c in range(3):
        out[..., c] = correlate(rgb[..., c].astype(np.float64), np.full((3, 3), 1 / 9))
    return out


def _t_gwt_inverse_is_exact_where_alpha_is_live_and_untouched_below_threshold():
    rng = np.random.default_rng(1)
    bg = rng.integers(40, 200, (20, 30, 3)).astype(np.float64)
    a = np.zeros((10, 12))
    a[2:8, 3:9] = 0.36
    a[0, 0] = 0.001
    img = bg.copy()
    sub = img[5:15, 10:22]
    sub[:] = a[..., None] * 0 + (1 - a[..., None]) * sub
    img = np.rint(img).astype(np.uint8)
    out, clip = gwt_inverse(img, a, (10, 5, 12, 10), [0, 0, 0])
    assert out[5, 10].tolist() == img[5, 10].tolist(), "a sample under the threshold moved"
    assert np.abs(out[7:13, 13:19].astype(int) - bg[7:13, 13:19]).max() <= 2, "the inverse is not the background"
    assert clip == 0.0
    # An opaque sample is divided by 1 − 0.99, never by zero (blend_modes.cpp:77, 92).
    a1 = a.copy()
    a1[9, 11] = 1.0
    with np.errstate(divide="raise", invalid="raise"):
        out1, _ = gwt_inverse(img, a1, (10, 5, 12, 10), [255, 255, 255])
    assert out1[14, 21].tolist() == [0, 0, 0] or img[14, 21].max() >= 253, "an opaque sample was not capped at 0.99"


def _t_the_gwt_weight_is_zero_far_from_the_maps_edges_and_capped_at_one():
    a = np.zeros((40, 40))
    a[15:25, 15:25] = 0.4
    w = gwt_weight(a, 1.2)
    assert w.max() <= 1.0 + 1e-12 and w.max() > 0.9
    assert w[0, 0] < 1e-3 and w[20, 20] < w[15, 20], "the weight is not on the edges"


def _t_gwt_denoise_leaves_the_picture_outside_the_padded_roi():
    rng = np.random.default_rng(2)
    img = rng.integers(0, 256, (120, 160, 3)).astype(np.uint8)
    a = np.zeros((20, 30))
    a[5:15, 5:25] = 0.4
    out, weight, (px0, py0, px1, py1) = gwt_denoise(img, a, (100, 80, 30, 20), _stub, 50.0, 1.2)
    outside = np.ones(img.shape[:2], bool)
    outside[py0:py1, px0:px1] = False
    assert np.array_equal(out[outside], img[outside]), "GWT's blend moved a pixel outside its padded ROI"
    assert (px1, py1) == (160, 120), "the padded ROI is not clipped to the picture"
    assert np.any(out != img)
    # Inside the padded ROI, far from the map's edges, the weight is ~0: the input stays.
    assert np.array_equal(out[py0 + 1, px0 + 1], img[py0 + 1, px0 + 1]), "the blend moved a pixel where the weight is 0"


def _t_a_white_mark_on_the_negative_is_the_negative_of_a_black_mark():
    rng = np.random.default_rng(3)
    b = rng.integers(0, 256, (8, 8, 3)).astype(np.float64)
    a = rng.uniform(0, 0.4, (8, 8))[..., None]
    white_on_neg = a * 255 + (1 - a) * (255 - b)
    black_on_b = (1 - a) * b
    assert np.allclose(255 - white_on_neg, black_on_b)


def _t_the_band_step_reads_a_left_trace_and_not_a_clean_one():
    h, w = 60, 90
    img = np.full((h, w, 3), 120, np.uint8)
    a = np.zeros((20, 40))
    a[5:15, 5:35] = 0.36
    a[4, 5:35] = 0.05
    a[15, 5:35] = 0.05
    rect = (25, 20, 40, 20)
    clean = steps(img, a, rect)
    assert abs(clean["band"][3]) < 1e-6 and abs(clean["core"][3]) < 1e-6
    left = img.copy()
    left[20 + 4, 30:60] = 110
    s = steps(left, a, rect)
    assert s["band"][3] < -1.0, "a trace in the faint band was not read"


def _t_tile_origin_is_recon_benchs():
    assert tile_origin((1634, 940, 149, 57), 1792, 1008, 512) == (1280, 496)
    assert tile_origin((10, 10, 50, 20), 1024, 1024, 512) == (0, 0)


def selftest():
    cases = [(n, f) for n, f in sorted(globals().items()) if n.startswith("_t_")]
    bad = 0
    for n, f in cases:
        try:
            f()
            print(f"ok   {n[3:]}")
        except Exception as e:  # an assertion, or the arithmetic a mutation broke
            bad += 1
            print(f"FAIL {n[3:]}: {type(e).__name__}: {e}")
    print(f"{len(cases) - bad} of {len(cases)} held")
    return 1 if bad else 0


# ───────────────────────────────────────────────────── main

def main():
    today = time.strftime("%Y-%m-%d")
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    w = sub.add_parser("weights", help="N1: GWT's embedded FDnCNN against KAIR's weights")
    w.add_argument("--gwt", required=True)
    w.add_argument("--gwt-commit")
    w.add_argument("--pth", required=True)
    w.add_argument("--out", required=True)
    p = sub.add_parser("pictures", help="N2: the real pictures, GWT's way and R10's")
    p.add_argument("pictures", nargs="+")
    p.add_argument("--look", required=True)
    p.add_argument("--catalogue", required=True)
    p.add_argument("--model", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--product")
    p.add_argument("--r10", help="k<k_edge>-s<strength>-<rgb|luma> (default k5-s1.0-rgb)")
    p.add_argument("--threads", type=int)
    p.add_argument("--date", default=today)
    i = sub.add_parser("invert", help="N3: the negatives of a run's tiles, for gen --photos")
    i.add_argument("--run", required=True)
    i.add_argument("--out")
    i.add_argument("--manifest", default="bench/manifest.json")
    i.add_argument("--check", action="store_true")
    i.add_argument("--tiles")
    c = sub.add_parser("crops", help="N3: GWT's two rules over bench crops")
    c.add_argument("--crops", required=True)
    c.add_argument("--model", required=True)
    c.add_argument("--out", required=True)
    c.add_argument("--threads", type=int)
    v = sub.add_parser("verdict", help="N4: the table")
    v.add_argument("--fdncnn", required=True)
    v.add_argument("--gwt", required=True)
    v.add_argument("--best", required=True)
    v.add_argument("--out", required=True)
    v.add_argument("--inv-tiles", help="invert's tiles folder: a negatives run's records get their source's group back")
    sub.add_parser("selftest")
    args = ap.parse_args()
    if args.cmd == "selftest":
        return selftest()
    return {"weights": cmd_weights, "pictures": cmd_pictures, "invert": cmd_invert, "crops": cmd_crops,
            "verdict": cmd_verdict}[args.cmd](args) or 0


if __name__ == "__main__":
    sys.exit(main())
