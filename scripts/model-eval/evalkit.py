#!/usr/bin/env python3
"""What every script of `scripts/model-eval/` shares: crops, measures, numbers.

What it is for
--------------
Step E12-R10 of the E12-R series (`docs/plan/E12-R10-model-evaluation.md`),
filed 2026-10-08 by the coordinator from the owner's spec
`wipemark-recon-spec-2026-10-08` (`07-model-evaluation.md`; the owner's
S10); these scripts were written on 2026-10-09, ahead of the runs, because
the owner asked for all code work to be finished before the steps are
unblocked. R10 decides, by a stated trigger and at a stated point, whether
a learned model (FDnCNN, a denoiser; LaMa, an inpainter) is needed after
the restoration. Nothing here is in the product (D166 holds, R10 §7).

This module is not run on its own: `trigger.py`, `fdncnn_export.py`,
`fdncnn_run.py`, `lama_run.py`, `baselines.py` and `ab.py` import it.

What it does
------------
* **Crops.** Reads one crop folder as `recon_bench run --export-crops`
  (`crates/wipemark-picture/examples/recon_bench.rs`, `export`) and
  `regress.py run --export-crops` (through
  `crates/wipemark-picture/examples/export_crops.rs`) write it:
  `input.png` (the stored picture), `recon.png` (the restoration, before
  any encoder), `gt.png` (the truth; the bench's only), `alpha.pgm`
  (16-bit, the opacity at the crop's pixels) and `meta.json` (the ROI and
  the mark's rectangle in the crop, the profile, the slice or the class and
  variant, `sigma_base` as the exporter measured it — both exporters write
  the Rust value, `wipemark_pixels::sigma_base` at the restoration's
  rectangle; a `null` or missing one, from an older run, is restated here —
  the holes as `[start, length]` runs of row-major indices).
* **Restated measures**, each from the Rust it names, so a crop can be
  measured after a model changed it:
  * `sigma_base` — `wipemark_pixels::sigma_base` (`interval.rs`, R8 §4.1):
    `1.4826 · median|ΔI| / √20` per channel over the ring two to eight
    pixels outside the mark's rectangle, `Δ` the 3 × 3 Laplacian;
  * `texture` and `texture_around` — `verify.rs` `steps` (D250): the 95th
    percentile (nearest rank) of each pixel's distance in (Y, Cb, Cr) from
    the mean of its eight neighbours, over the pixels under the mark with
    `α` from the noise floor to under the opaque threshold, and over the
    pixels of the rectangle and a ring four out with `α` under the floor;
  * `consistency_px` — `restore.rs` (D305): the restored samples blended
    back, `α·L + (1 − α)·O`, against the stored ones, the 95th percentile of
    the distance over the samples with `α` from the floor to under the
    threshold, clamped samples and holes left out;
  * `psnr`, `ssim` and `delta_e` in the ROI — `recon_bench.rs`'s `psnr`,
    `ssim` (BT.601 luma, uniform 7 × 7 windows) and CIEDE2000 (Sharma, Wu
    and Dalal, 2005), over 8-bit samples.
* **Numbers**: the median and p5 by linear interpolation (numpy's default,
  as `scripts/bench/report.py`), the nearest-rank percentile the Rust uses,
  square binary morphology, mirror padding, BT.601 YCbCr.
* **The run's facts**: Python, numpy, Pillow and whichever of torch,
  onnxruntime, OpenCV and lpips can be imported, the platform, the CPU.

How to run it
-------------
It is imported. `python3 scripts/model-eval/evalkit.py selftest` runs its
own cases (no weights, no corpus).

What it needs
-------------
Python 3.10+, numpy and Pillow (a venv is fine; nothing in the repository
depends on them). torch, onnxruntime, opencv-python and lpips are imported
lazily by the scripts that need them, never here at module level.

What its output means
---------------------
`selftest` prints one line per case and exits 0 when every case held, 1
otherwise.
"""

import hashlib
import json
import math
import os
import platform
import re
import sys

import numpy as np

REPO = os.environ.get("WIPEMARK_REPO") or os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CATALOGUE = os.path.join(REPO, "manifests", "marks.v1.json")

# ── the product's constants, mirrored; they move with the Rust they name ──
NOISE_FLOOR = 0.002          # verify.rs NOISE_FLOOR
OPAQUE_DEFAULT = 0.95        # every shipped profile's opaque_above (D155)
NOISE_RING = 8               # interval.rs NOISE_RING
H_BASE, H_SIGMAS = 0.5, 2.0  # interval.rs: pixel POCS's h = H_BASE + H_SIGMAS·σ_base
TEXTURE_LEVELS = 5.5         # verify.rs TEXTURE_LEVELS (D250)
TEXTURE_RATIO = 2.0          # verify.rs TEXTURE_RATIO
TEXTURE_RATIO_MIN = 0.8      # verify.rs TEXTURE_RATIO_MIN (D307)
CHROMA_LEVELS = 4.0          # verify.rs CHROMA_LEVELS (D247)
STEP_LEVELS = 1.0            # verify.rs STEP_LEVELS (D244)
BAND = (3.0 / 255.0, 0.2)    # verify.rs BAND
PSNR_CAP = 100.0             # recon_bench.rs PSNR_CAP
LUMA = (0.299, 0.587, 0.114)

LOSSY_CLASSES = {"recon-jpeg-444", "recon-jpeg-420", "recon-webp"}


class Refusal(Exception):
    """A refusal: exit 2, with the sentence that says why."""


# ───────────────────────────────────────────────────────── files

def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def read_png(path):
    """An 8-bit RGB array, H × W × 3."""
    from PIL import Image

    with Image.open(path) as im:
        return np.asarray(im.convert("RGB"), dtype=np.uint8).copy()


def write_png(path, rgb):
    from PIL import Image

    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    Image.fromarray(np.asarray(rgb, dtype=np.uint8), "RGB").save(path, "PNG")


def read_pgm16(path):
    """A binary PGM (P5) of maxval 65535 or 255, as floats in [0, 1]."""
    with open(path, "rb") as f:
        data = f.read()
    fields, pos = [], 0
    while len(fields) < 4:
        while data[pos:pos + 1].isspace():
            pos += 1
        if data[pos:pos + 1] == b"#":
            while data[pos:pos + 1] not in (b"\n", b""):
                pos += 1
            continue
        start = pos
        while not data[pos:pos + 1].isspace():
            pos += 1
        fields.append(data[start:pos])
    pos += 1  # the one whitespace after maxval
    if fields[0] != b"P5":
        raise Refusal(f"{path}: not a binary PGM")
    w, h, maxval = int(fields[1]), int(fields[2]), int(fields[3])
    dtype = ">u2" if maxval > 255 else "u1"
    a = np.frombuffer(data, dtype=dtype, count=w * h, offset=pos).reshape(h, w)
    return a.astype(np.float64) / maxval


def write_pgm16(path, alpha):
    a = np.round(np.clip(np.asarray(alpha, dtype=np.float64), 0.0, 1.0) * 65535.0).astype(">u2")
    h, w = a.shape
    with open(path, "wb") as f:
        f.write(f"P5\n{w} {h}\n65535\n".encode())
        f.write(a.tobytes())


def write_json(path, value):
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    with open(path + ".tmp", "w") as f:
        json.dump(value, f, indent=1, ensure_ascii=True)
        f.write("\n")
    os.replace(path + ".tmp", path)


def rle_decode(runs, h, w):
    m = np.zeros(h * w, dtype=bool)
    for start, length in runs or []:
        m[int(start):int(start) + int(length)] = True
    return m.reshape(h, w)


def rle_encode(mask):
    idx = np.flatnonzero(np.asarray(mask).ravel())
    out = []
    for i in idx.tolist():
        if out and out[-1][0] + out[-1][1] == i:
            out[-1][1] += 1
        else:
            out.append([i, 1])
    return out


# ───────────────────────────────────────────────────────── the catalogue

_PROFILES = None


def profile(pid):
    """A profile of `manifests/marks.v1.json` by id: its logo and opaque threshold."""
    global _PROFILES
    if _PROFILES is None:
        try:
            with open(CATALOGUE) as f:
                _PROFILES = {p["id"]: p for p in json.load(f)["profiles"]}
        except OSError:
            _PROFILES = {}
    return _PROFILES.get(pid)


# ───────────────────────────────────────────────────────── a crop

def is_lossy(meta):
    """A lossy source: a JPEG or lossy-WebP slice of the bench, or a lossy class/variant of R1."""
    if meta.get("class") in LOSSY_CLASSES:
        return True
    text = " ".join(str(meta.get(k) or "") for k in ("slice", "variant", "class"))
    return bool(re.search(r"jpeg|webp-lossy|(^|[-\s])q\d+", text))


class Crop:
    """One exported crop folder."""

    def __init__(self, path):
        self.dir = path
        with open(os.path.join(path, "meta.json")) as f:
            self.meta = json.load(f)
        self.input = read_png(os.path.join(path, "input.png"))
        self.recon = read_png(os.path.join(path, "recon.png"))
        gt = os.path.join(path, "gt.png")
        self.gt = read_png(gt) if os.path.exists(gt) else None
        self.alpha = read_pgm16(os.path.join(path, "alpha.pgm"))
        if self.input.shape != self.recon.shape or self.alpha.shape != self.recon.shape[:2]:
            raise Refusal(f"{path}: input, recon and alpha are not one shape")

    @classmethod
    def of(cls, meta, input, recon, alpha, gt=None, path="<memory>"):
        c = cls.__new__(cls)
        c.dir, c.meta, c.input, c.recon, c.alpha, c.gt = path, meta, input, recon, alpha, gt
        return c

    @property
    def name(self):
        return os.path.basename(os.path.normpath(self.dir))

    @property
    def shape(self):
        return self.recon.shape[:2]

    @property
    def roi(self):
        """(x0, y0, x1, y1) of the ROI in the crop."""
        r = self.meta.get("roi_in_crop")
        if not r:
            x0, y0, x1, y1 = self.rect_px
            h, w = self.shape
            return max(0, x0 - 4), max(0, y0 - 4), min(w, x1 + 4), min(h, y1 + 4)
        return r["x"], r["y"], r["x"] + r["width"], r["y"] + r["height"]

    @property
    def rect_px(self):
        """(x0, y0, x1, y1): the restoration's rectangle in the crop — the
        exporter's when it wrote one (`rect_px_in_crop`), else the box of
        the support (`α` at the noise floor or over)."""
        r = self.meta.get("rect_px_in_crop")
        if r:
            return r["x"], r["y"], r["x"] + r["width"], r["y"] + r["height"]
        return support_box(self.alpha)

    @property
    def opaque(self):
        if isinstance(self.meta.get("opaque_above"), (int, float)):
            return float(self.meta["opaque_above"])
        p = profile(self.meta.get("profile"))
        return float(p["opaque_above"]) if p else OPAQUE_DEFAULT

    @property
    def logo(self):
        if isinstance(self.meta.get("logo"), list):
            return [float(v) for v in self.meta["logo"]]
        p = profile(self.meta.get("profile"))
        if not p:
            raise Refusal(f"{self.dir}: no logo — meta.json names no profile the catalogue knows")
        return [float(v) for v in p["blend"]["logo"]]

    @property
    def holes(self):
        h, w = self.shape
        if "holes_rle" in self.meta:
            return rle_decode(self.meta["holes_rle"], h, w)
        return self.alpha >= self.opaque

    @property
    def lossy(self):
        return is_lossy(self.meta)

    @property
    def group(self):
        return self.meta.get("group") or self.meta.get("class") or "-"

    @property
    def slice(self):
        return self.meta.get("slice") or self.meta.get("variant") or "-"

    @property
    def restored(self):
        """Whether a restoration made this crop (F7, M5): the exporter's word
        when it said it, else whether `recon` differs from `input` under the
        mark — a refused mark's crop is the input itself."""
        if isinstance(self.meta.get("restored"), bool):
            return self.meta["restored"]
        under = self.alpha >= NOISE_FLOOR
        return bool(np.any(self.recon[under] != self.input[under]))

    def sigma_base(self):
        """R8 §4.1's σ_base, max over the channels: the exporter's when it
        measured it, else restated here on `input.png`."""
        s = self.meta.get("sigma_base")
        if isinstance(s, list) and s:
            return float(max(s))
        if isinstance(s, (int, float)):
            return float(s)
        return float(max(sigma_base(self.input, self.rect_px)))

    def key(self):
        m = self.meta
        return (m.get("config"), m.get("background"), m.get("case"), m.get("slice"), m.get("encoder"))


def load_crops(root):
    """Every crop folder under `root` (a folder holding `meta.json`, `recon.png`,
    `input.png` and `alpha.pgm`), sorted by path."""
    out = []
    for d, _dirs, files in sorted(os.walk(root)):
        if {"meta.json", "recon.png", "input.png", "alpha.pgm"} <= set(files):
            out.append(d)
    return out


def support_box(alpha):
    ys, xs = np.nonzero(alpha >= NOISE_FLOOR)
    if len(xs) == 0:
        return 0, 0, 0, 0
    return int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1


# ───────────────────────────────────────────────────────── numbers

def quantile(values, q):
    """The q-quantile by linear interpolation (numpy's default); None for none."""
    v = sorted(float(x) for x in values if x is not None)
    if not v:
        return None
    pos = (len(v) - 1) * q
    lo, hi = math.floor(pos), math.ceil(pos)
    return v[lo] + (v[hi] - v[lo]) * (pos - lo)


def median(values):
    return quantile(values, 0.5)


def p5(values):
    return quantile(values, 0.05)


def nearest_rank(values, p):
    """The Rust's `percentile`: sorted, the value at round((n − 1)·p); 0 for none."""
    v = np.sort(np.asarray(values, dtype=np.float64).ravel())
    if v.size == 0:
        return 0.0
    return float(v[int(round((v.size - 1) * p))])


def smoothstep(e0, e1, x):
    t = np.clip((np.asarray(x, dtype=np.float64) - e0) / (e1 - e0), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def ycbcr(rgb):
    """BT.601 full-range Y, Cb, Cr (JFIF's, less the offset), float."""
    rgb = np.asarray(rgb, dtype=np.float64)
    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    y = LUMA[0] * r + LUMA[1] * g + LUMA[2] * b
    cb = -0.168736 * r - 0.331264 * g + 0.5 * b
    cr = 0.5 * r - 0.418688 * g - 0.081312 * b
    return np.stack([y, cb, cr], axis=-1)


def rgb_of(ycc):
    ycc = np.asarray(ycc, dtype=np.float64)
    y, cb, cr = ycc[..., 0], ycc[..., 1], ycc[..., 2]
    r = y + 1.402 * cr
    g = y - 0.344136 * cb - 0.714136 * cr
    b = y + 1.772 * cb
    return np.stack([r, g, b], axis=-1)


def to_u8(x):
    return np.clip(np.floor(np.asarray(x, dtype=np.float64) + 0.5), 0, 255).astype(np.uint8)


def mirror_pad(img, pad):
    """Mirror padding (`reflect`: the edge pixel is not repeated) by `pad` on every side."""
    widths = [(pad, pad), (pad, pad)] + [(0, 0)] * (np.ndim(img) - 2)
    return np.pad(img, widths, mode="reflect")


def pad_to_multiple(img, m):
    """Mirror padding at the bottom and right to sides that are multiples of `m`; the pads."""
    h, w = img.shape[:2]
    ph, pw = (-h) % m, (-w) % m
    widths = [(0, ph), (0, pw)] + [(0, 0)] * (np.ndim(img) - 2)
    return np.pad(img, widths, mode="symmetric"), (ph, pw)


def _shift_or(mask, r, op):
    """`op` over every shift of a square of side 2r + 1, outside the picture
    counted as False for a dilation and as True for an erosion."""
    m = np.asarray(mask, dtype=bool)
    h, w = m.shape
    fill = op is np.logical_and
    padded = np.pad(m, r, mode="constant", constant_values=fill)
    out = np.full_like(m, fill)
    for dy in range(-r, r + 1):
        for dx in range(-r, r + 1):
            out = op(out, padded[r + dy:r + dy + h, r + dx:r + dx + w])
    return out


def dilate(mask, r):
    return _shift_or(mask, r, np.logical_or) if r > 0 else np.asarray(mask, dtype=bool)


def erode(mask, r):
    return _shift_or(mask, r, np.logical_and) if r > 0 else np.asarray(mask, dtype=bool)


def opening(mask, r):
    return dilate(erode(mask, r), r)


def gradient_magnitude(a):
    """|∇a| by central differences (numpy's `gradient`), per pixel."""
    gy, gx = np.gradient(np.asarray(a, dtype=np.float64))
    return np.hypot(gx, gy)


def components(mask):
    """4-connected components of a boolean mask: a label array (0 = none) and the count."""
    m = np.asarray(mask, dtype=bool)
    h, w = m.shape
    labels = np.zeros((h, w), dtype=np.int32)
    n = 0
    for y0, x0 in zip(*np.nonzero(m)):
        if labels[y0, x0]:
            continue
        n += 1
        stack = [(y0, x0)]
        labels[y0, x0] = n
        while stack:
            y, x = stack.pop()
            for ny, nx in ((y - 1, x), (y + 1, x), (y, x - 1), (y, x + 1)):
                if 0 <= ny < h and 0 <= nx < w and m[ny, nx] and not labels[ny, nx]:
                    labels[ny, nx] = n
                    stack.append((ny, nx))
    return labels, n


# ───────────────────────────────────────────────────────── restated measures

def sigma_base(img, rect, ring=NOISE_RING):
    """`wipemark_pixels::sigma_base`: per channel, `1.4826 · median|ΔI| / √20`
    over the samples two to `ring` outside `rect` (Chebyshev distance) whose
    four neighbours are inside the grid; the median is the upper one
    (`v[n / 2]`), as the Rust takes it. `rect` is (x0, y0, x1, y1)."""
    img = np.asarray(img, dtype=np.float64)
    h, w = img.shape[:2]
    x0, y0, x1, y1 = rect
    out = []
    ys = range(max(1, y0 - ring), min(h - 1, y1 + ring))
    xs = range(max(1, x0 - ring), min(w - 1, x1 + ring))
    sel = []
    for y in ys:
        for x in xs:
            outside = max(x0 - x, x - (x1 - 1), y0 - y, y - (y1 - 1))
            if 2 <= outside <= ring:
                sel.append((y, x))
    if not sel:
        return [0.0, 0.0, 0.0]
    yy, xx = np.array(sel).T
    for c in range(3):
        p = img[..., c]
        lap = p[yy, xx - 1] + p[yy, xx + 1] + p[yy - 1, xx] + p[yy + 1, xx] - 4.0 * p[yy, xx]
        v = np.sort(np.abs(lap))
        out.append(float(1.4826 * v[v.size // 2] / math.sqrt(20.0)))
    return out


def texture(img, alpha, rect, opaque):
    """`verify.rs` `steps`' roughness (D250): (texture, texture_around).

    Over the rectangle `rect` and a ring four pixels out: a pixel with
    `α` under the noise floor is *around*; one with `α` from the floor to
    under `opaque` is *under the mark*. A pixel's roughness is its distance
    in (Y, Cb, Cr) from the mean of its neighbours inside the read region
    (the rectangle and five pixels out, inside the grid). The 95th
    percentile by nearest rank of each set."""
    img = np.asarray(img, dtype=np.float64)
    six = ycbcr(img)
    h, w = img.shape[:2]
    x0, y0, x1, y1 = rect
    bx0, by0, bx1, by1 = max(x0 - 5, 0), max(y0 - 5, 0), min(x1 + 5, w), min(y1 + 5, h)
    region = six[by0:by1, bx0:bx1]
    rh, rw = region.shape[:2]
    padded = np.pad(region, ((1, 1), (1, 1), (0, 0)))
    ones = np.pad(np.ones((rh, rw)), 1)
    total = np.zeros_like(region)
    count = np.zeros((rh, rw))
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            if (dy, dx) == (0, 0):
                continue
            total += padded[1 + dy:1 + dy + rh, 1 + dx:1 + dx + rw]
            count += ones[1 + dy:1 + dy + rh, 1 + dx:1 + dx + rw]
    rough = np.sqrt(((region - total / count[..., None]) ** 2).sum(axis=-1))
    mark, around = [], []
    for y in range(max(y0 - 4, 0), min(y1 + 4, h)):
        for x in range(max(x0 - 4, 0), min(x1 + 4, w)):
            inside = x0 <= x < x1 and y0 <= y < y1
            a = alpha[y, x] if inside else 0.0
            r = rough[y - by0, x - bx0]
            if a < NOISE_FLOOR:
                around.append(r)
            elif a < opaque:
                mark.append(r)
    return nearest_rank(mark, 0.95), nearest_rank(around, 0.95)


def unblend(stored, alpha, logo):
    """`restore.rs` `unblend`: (I − α·L)/(1 − α), per channel, unclamped."""
    a = np.asarray(alpha, dtype=np.float64)[..., None]
    return (np.asarray(stored, dtype=np.float64) - a * np.asarray(logo)) / (1.0 - a)


def clamped_samples(stored, alpha, logo, opaque):
    """The samples the inverse put outside the range by more than half a level."""
    a = np.asarray(alpha, dtype=np.float64)
    live = (a >= NOISE_FLOOR) & (a < opaque)
    safe = np.where(live, a, 0.0)
    u = unblend(stored, safe, logo)
    return ((u < -0.5) | (u > 255.5)) & live[..., None]


def consistency_px(out, stored, alpha, logo, opaque):
    """D305 restated: (p95 of |α·L + (1 − α)·O − I|, excluded) over the
    samples with `α` from the noise floor to under `opaque`, the clamped ones
    and the three samples of every hole left out."""
    a = np.asarray(alpha, dtype=np.float64)
    live = (a >= NOISE_FLOOR) & (a < opaque)
    clamped = clamped_samples(stored, alpha, logo, opaque)
    blended = a[..., None] * np.asarray(logo) + (1.0 - a[..., None]) * np.asarray(out, dtype=np.float64)
    d = np.abs(blended - np.asarray(stored, dtype=np.float64))
    use = live[..., None] & ~clamped
    excluded = int(clamped.sum()) + 3 * int((a >= opaque).sum())
    return nearest_rank(d[use], 0.95), excluded


def local_consistency(out, stored, alpha, logo):
    """Per sample, |α·L + (1 − α)·O − I| in levels (R10 §3.3's `w`)."""
    a = np.asarray(alpha, dtype=np.float64)[..., None]
    return np.abs(a * np.asarray(logo) + (1.0 - a) * np.asarray(out, dtype=np.float64) - np.asarray(stored, dtype=np.float64))


def h_of(sigma):
    """Pixel POCS's interval half-width `h = H_BASE + H_SIGMAS·σ_base` (R8)."""
    return H_BASE + H_SIGMAS * float(sigma)


def _box(img, roi):
    x0, y0, x1, y1 = roi
    return np.asarray(img, dtype=np.float64)[y0:y1, x0:x1]


def psnr(a, b, roi):
    d = _box(a, roi) - _box(b, roi)
    s = float((d * d).sum())
    if s == 0.0:
        return PSNR_CAP
    return min(PSNR_CAP, 10.0 * math.log10(255.0 * 255.0 / (s / d.size)))


def psnr_mask(a, b, mask):
    """PSNR over the samples of the pixels a mask holds (M2, inside the mask)."""
    m = np.asarray(mask, dtype=bool)
    if not m.any():
        return None
    d = np.asarray(a, dtype=np.float64)[m] - np.asarray(b, dtype=np.float64)[m]
    s = float((d * d).sum())
    if s == 0.0:
        return PSNR_CAP
    return min(PSNR_CAP, 10.0 * math.log10(255.0 * 255.0 / (s / d.size)))


def ssim(a, b, roi, win=7):
    """`recon_bench.rs` `ssim`: the mean SSIM of BT.601 luma over every 7 × 7
    window inside the ROI, uniform weights; None under a window."""
    ya, yb = ycbcr(_box(a, roi))[..., 0], ycbcr(_box(b, roi))[..., 0]
    h, w = ya.shape
    if h < win or w < win:
        return None
    c1, c2 = (0.01 * 255.0) ** 2, (0.03 * 255.0) ** 2

    def boxsum(x):
        s = np.cumsum(np.cumsum(np.pad(x, ((1, 0), (1, 0))), axis=0), axis=1)
        return s[win:, win:] - s[:-win, win:] - s[win:, :-win] + s[:-win, :-win]

    k = float(win * win)
    ma, mb = boxsum(ya) / k, boxsum(yb) / k
    va = boxsum(ya * ya) / k - ma * ma
    vb = boxsum(yb * yb) / k - mb * mb
    cov = boxsum(ya * yb) / k - ma * mb
    s = ((2 * ma * mb + c1) * (2 * cov + c2)) / ((ma * ma + mb * mb + c1) * (va + vb + c2))
    return float(s.mean())


def to_linear(v):
    c = np.clip(np.asarray(v, dtype=np.float64) / 255.0, 0.0, 1.0)
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def lab(rgb):
    lin = to_linear(rgb)
    r, g, b = lin[..., 0], lin[..., 1], lin[..., 2]
    x = (0.4124564 * r + 0.3575761 * g + 0.1804375 * b) / 0.95047
    y = 0.2126729 * r + 0.7151522 * g + 0.072175 * b
    z = (0.0193339 * r + 0.119192 * g + 0.9503041 * b) / 1.08883
    d = 6.0 / 29.0

    def f(t):
        return np.where(t > d ** 3, np.cbrt(t), t / (3 * d * d) + 4.0 / 29.0)

    fx, fy, fz = f(x), f(y), f(z)
    return np.stack([116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)], axis=-1)


def ciede2000(p, q):
    """CIEDE2000 between Lab arrays (…, 3), as `recon_bench.rs` `ciede2000`."""
    p, q = np.asarray(p, dtype=np.float64), np.asarray(q, dtype=np.float64)
    l1, a1, b1 = p[..., 0], p[..., 1], p[..., 2]
    l2, a2, b2 = q[..., 0], q[..., 1], q[..., 2]
    c1, c2 = np.hypot(a1, b1), np.hypot(a2, b2)
    cm = (c1 + c2) / 2.0
    g = 0.5 * (1.0 - np.sqrt(cm ** 7 / (cm ** 7 + 25.0 ** 7)))
    a1p, a2p = (1 + g) * a1, (1 + g) * a2
    c1p, c2p = np.hypot(a1p, b1), np.hypot(a2p, b2)

    def hue(a, b):
        h = np.degrees(np.arctan2(b, a))
        h = np.where(h < 0, h + 360.0, h)
        return np.where((a == 0) & (b == 0), 0.0, h)

    h1p, h2p = hue(a1p, b1), hue(a2p, b2)
    dl, dc = l2 - l1, c2p - c1p
    zero = c1p * c2p == 0
    diff = h2p - h1p
    dh = np.where(zero, 0.0, np.where(np.abs(diff) <= 180, diff, np.where(diff > 180, diff - 360, diff + 360)))
    dhh = 2.0 * np.sqrt(c1p * c2p) * np.sin(np.radians(dh) / 2.0)
    lm, cmp_ = (l1 + l2) / 2.0, (c1p + c2p) / 2.0
    s = h1p + h2p
    hm = np.where(zero, s, np.where(np.abs(h1p - h2p) <= 180, s / 2.0, np.where(s < 360, (s + 360) / 2.0, (s - 360) / 2.0)))
    t = (1 - 0.17 * np.cos(np.radians(hm - 30)) + 0.24 * np.cos(np.radians(2 * hm))
         + 0.32 * np.cos(np.radians(3 * hm + 6)) - 0.20 * np.cos(np.radians(4 * hm - 63)))
    dtheta = 30.0 * np.exp(-(((hm - 275.0) / 25.0) ** 2))
    rc = 2.0 * np.sqrt(cmp_ ** 7 / (cmp_ ** 7 + 25.0 ** 7))
    sl = 1 + 0.015 * (lm - 50) ** 2 / np.sqrt(20 + (lm - 50) ** 2)
    sc = 1 + 0.045 * cmp_
    sh = 1 + 0.015 * cmp_ * t
    rt = -np.sin(np.radians(2 * dtheta)) * rc
    return np.sqrt((dl / sl) ** 2 + (dc / sc) ** 2 + (dhh / sh) ** 2 + rt * (dc / sc) * (dhh / sh))


def delta_e(a, b, roi):
    return float(ciede2000(lab(_box(a, roi)), lab(_box(b, roi))).mean())


# ───────────────────────────────────────────────────────── the run's facts

def environment(device=None):
    """What ran: Python, numpy, Pillow and whichever model frameworks import, the platform, the CPU."""
    env = {"python": platform.python_version(), "numpy": np.__version__, "platform": platform.platform(),
           "cpu": platform.processor() or platform.machine(), "device": device or "cpu"}
    for name, mod in (("pillow", "PIL"), ("torch", "torch"), ("onnxruntime", "onnxruntime"),
                      ("opencv", "cv2"), ("lpips", "lpips")):
        try:
            m = __import__(mod)
            env[name] = getattr(m, "__version__", "unknown")
        except Exception:  # noqa: BLE001 — absent or broken is the same fact here
            env[name] = None
    return env


def md_table(header, rows):
    out = ["| " + " | ".join(header) + " |", "|" + "|".join("---" for _ in header) + "|"]
    for r in rows:
        out.append("| " + " | ".join(fmt(c) for c in r) + " |")
    return "\n".join(out)


def fmt(v, digits=3):
    if v is None:
        return "–"
    if isinstance(v, bool):
        return "yes" if v else "no"
    if isinstance(v, float):
        if math.isinf(v) or math.isnan(v):
            return str(v)
        return f"{v:.{digits}f}"
    return str(v)


# ───────────────────────────────────────────────────────── selftest

def run_cases(cases):
    """Runs `(name, fn)` pairs; a case fails by raising. Prints one line each; 0 or 1."""
    failed = 0
    for name, fn in cases:
        try:
            fn()
            print(f"  ok    {name}")
        except Exception as e:  # noqa: BLE001 — a selftest reports every failure, whatever it is
            failed += 1
            print(f"  FAIL  {name}: {type(e).__name__}: {e}")
    print("selftest:", "FAIL" if failed else "ok", f"({failed} of {len(cases)} failed)")
    return 1 if failed else 0


def synthetic_crop(seed=1, size=48, mark=(12, 12, 36, 36), lossy=True, peak=0.45, hole=False, noise=20.0):
    """A crop with no file behind it: a textured background, a disc of `α`
    under `mark`, the mark blended with a white logo and rounded, its
    inverse as `recon`. For the selftests of every script here."""
    rng = np.random.default_rng(seed)
    gt = np.clip(120 + noise * rng.standard_normal((size, size, 3)), 0, 255)
    gt = to_u8(gt)
    x0, y0, x1, y1 = mark
    yy, xx = np.mgrid[0:size, 0:size]
    cx, cy, r = (x0 + x1) / 2 - 0.5, (y0 + y1) / 2 - 0.5, (x1 - x0) / 2
    d = np.hypot(xx - cx, yy - cy) / r
    alpha = np.clip(peak * (1 - d), 0, 1)
    alpha[(xx < x0) | (xx >= x1) | (yy < y0) | (yy >= y1)] = 0.0
    if hole:
        alpha[(d < 0.15)] = 0.97
    logo = [255.0, 255.0, 255.0]
    stored = to_u8(alpha[..., None] * 255.0 + (1 - alpha[..., None]) * gt)
    opaque = OPAQUE_DEFAULT
    live = (alpha >= NOISE_FLOOR) & (alpha < opaque)
    rec = stored.astype(np.float64).copy()
    safe = np.where(live, alpha, 0.0)
    rec[live] = unblend(stored, safe, logo)[live]
    recon = to_u8(rec)
    meta = {"slice": "jpeg444-q95" if lossy else "png", "encoder": "pillow", "group": "flat",
            "config": "R0", "background": f"bg{seed}", "case": "c", "profile": None,
            "logo": logo, "opaque_above": opaque,
            "roi_in_crop": {"x": x0 - 4, "y": y0 - 4, "width": x1 - x0 + 8, "height": y1 - y0 + 8},
            "rect_px_in_crop": {"x": x0, "y": y0, "width": x1 - x0, "height": y1 - y0},
            "holes_rle": rle_encode(alpha >= opaque), "restored": True}
    return Crop.of(meta, stored, recon, alpha, gt=gt)


def _t_ciede2000_is_sharmas():
    pairs = [([50.0, 2.6772, -79.7751], [50.0, 0.0, -82.7485], 2.0425),
             ([50.0, 0.0, 0.0], [50.0, -1.0, 2.0], 2.3669),
             ([50.0, 2.5, 0.0], [73.0, 25.0, -18.0], 27.1492),
             ([50.0, 2.5, 0.0], [61.0, -5.0, 29.0], 22.8977),
             ([50.0, 2.5, 0.0], [56.0, -27.0, -3.0], 31.9030),
             ([50.0, 2.5, 0.0], [58.0, 24.0, 15.0], 19.4535)]
    for p, q, want in pairs:
        got = float(ciede2000(np.array(p), np.array(q)))
        assert abs(got - want) < 1e-4, (p, q, got, want)


def _t_psnr_of_one_level_is_48_13():
    a = np.full((20, 20, 3), 100, np.uint8)
    b = np.full((20, 20, 3), 101, np.uint8)
    roi = (5, 5, 15, 15)
    assert psnr(a, a, roi) == PSNR_CAP
    assert abs(psnr(a, b, roi) - 48.1308) < 1e-3
    assert abs(ssim(a, a, roi) - 1.0) < 1e-9
    assert ssim(a, b, (5, 5, 11, 15)) is None


def _t_sigma_base_reads_white_noise_as_its_sigma():
    rng = np.random.default_rng(3)
    img = 128 + 4.0 * rng.standard_normal((200, 200, 3))
    s = sigma_base(img, (60, 60, 140, 140))
    assert all(3.4 < v < 4.6 for v in s), s


def _t_the_exporters_sigma_base_wins_and_null_is_restated():
    c = synthetic_crop()
    restated = float(max(sigma_base(c.input, c.rect_px)))
    c.meta["sigma_base"] = [1.25, 7.5, 2.0]
    assert c.sigma_base() == 7.5, c.sigma_base()
    c.meta["sigma_base"] = None
    assert c.sigma_base() == restated, (c.sigma_base(), restated)
    del c.meta["sigma_base"]
    assert c.sigma_base() == restated


def _t_morphology_and_padding():
    m = np.zeros((9, 9), bool)
    m[4, 4] = True
    assert dilate(m, 2).sum() == 25
    assert erode(dilate(m, 2), 2).sum() == 1
    assert opening(m, 1).sum() == 0
    img = np.arange(12.0).reshape(3, 4)
    p, (ph, pw) = pad_to_multiple(img, 8)
    assert p.shape == (8, 8) and (ph, pw) == (5, 4) and np.array_equal(p[:3, :4], img)
    assert mirror_pad(img, 2).shape == (7, 8)


def _t_an_exact_inverse_is_consistent_to_rounding():
    c = synthetic_crop(lossy=False)
    px, _ = consistency_px(c.recon, c.input, c.alpha, c.logo, c.opaque)
    assert px <= 0.5 + 1e-9, px
    worse, _ = consistency_px(np.clip(c.recon.astype(float) + 6, 0, 255), c.input, c.alpha, c.logo, c.opaque)
    assert worse > 2.0, worse


def _t_a_pgm_and_runs_round_trip():
    import tempfile

    a = np.linspace(0, 1, 35).reshape(5, 7)
    with tempfile.TemporaryDirectory() as d:
        p = os.path.join(d, "a.pgm")
        write_pgm16(p, a)
        assert np.max(np.abs(read_pgm16(p) - a)) < 1e-4
    m = a > 0.5
    assert np.array_equal(rle_decode(rle_encode(m), 5, 7), m)


def selftest():
    return run_cases([
        ("ciede2000_is_sharmas", _t_ciede2000_is_sharmas),
        ("psnr_of_one_level_is_48_13", _t_psnr_of_one_level_is_48_13),
        ("sigma_base_reads_white_noise_as_its_sigma", _t_sigma_base_reads_white_noise_as_its_sigma),
        ("the_exporters_sigma_base_wins_and_null_is_restated",
         _t_the_exporters_sigma_base_wins_and_null_is_restated),
        ("morphology_and_padding", _t_morphology_and_padding),
        ("an_exact_inverse_is_consistent_to_rounding", _t_an_exact_inverse_is_consistent_to_rounding),
        ("a_pgm_and_runs_round_trip", _t_a_pgm_and_runs_round_trip),
    ])


if __name__ == "__main__":
    if sys.argv[1:] == ["selftest"]:
        sys.exit(selftest())
    print(__doc__)
    sys.exit(2)
