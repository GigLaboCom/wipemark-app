#!/usr/bin/env python3
"""Is there one mark? — invariance and holes over a source's outputs (stage 1).

What it is for
--------------
Step E12-R11 of the E12-R series (`docs/plan/E12-R11-grok-map.md` §4.1,
"Statistics"), filed 2026-10-08 by the coordinator from the owner's spec
`wipemark-recon-spec-2026-10-08` (`08-grok-evaluation.md` §2); this tool
is the container part, written 2026-10-09 before any capture exists. For
Gemini, GWT handed us the maps; for the second vendor nobody knows whether
a stable mark exists at all. Stage 1 answers that from the outputs alone:
over many files of one source and one picture size, a mark that is one
fixed blend makes every file's corner the same function of its own
background, and anything else — an opacity that changes per file, a
position that wanders, a different mark per source — leaves a trace this
script measures and names. It also answers "are there holes?" (pixels so
nearly opaque that nothing under them can be recovered) before any map is
fitted. Stages 1–3 themselves are the host's: the captures are the
owner's (R2 §4.3) and never enter git.

What it does
------------
1. Reads a list of files (see "Input" below) and groups them into
   **pairs** — (source, picture width × height), R11's (source, size,
   aspect). Held-out files (`held_out`) are left out unless
   `--with-held-out`: R11 keeps them for the profile's proof (P1–P5).
2. Crops every file of a pair at **one integer offset**: the mark's
   rectangle — the most common of the files' own rectangles (a file whose
   rectangle differs is named in the report and still cropped at the
   pair's) — grown by `--crop-factor` (1.5, R11 §4.1) × the mark's width
   left and right and × its height above and below, clipped to the
   picture. No sub-pixel alignment here (`align.py` is that): the scatter
   of the position is one of the things measured.
3. Per file, the background under the mark `Ô_i` is **the ring's
   quadratic, as `calibrate` fits it** (`crates/wipemark-pixels/src/
   calibrate.rs`, `background`/`ring_background`, restated as
   `scripts/analytics/bias.py` restates it): per channel, least squares of
   `k0 + k1·u + k2·v + k3·u² + k4·uv + k5·v²` over a ring `--ring` pixels
   wide (6, calibrate's), `u = (x + 0.5 − cx)/s`, `v = (y + 0.5 − cy)/s`,
   `s` the rectangle's longer side. One deviation: the ring starts
   `--guard` pixels (2) outside the rectangle rather than touching it,
   because the rectangle is a person's estimate and the position may
   wander; `--guard 0` is calibrate's ring exactly.
4. Everything below is on **luma** `Y = 0.299 R + 0.587 G + 0.114 B` in
   8-bit levels, except `mean.png`, which is RGB. Per pixel over the files
   of the pair: `mean_i I_i(p)`, `std_i I_i(p)`; `std_ring` is the median
   of `std_i I_i` over the ring — the scatter of the backgrounds — and
   `ratio(p) = std_i I_i(p) / std_ring`.
5. **The support**: the pixels of the mark's zone (its rectangle grown by
   `--guard`) where the mean departs from the mean background,
   `|mean_i I_i(p) − mean_i Ô_i(p)| > T`, with
   `T = max(--support-min 3.0, --support-k 4 × σ)` levels `[tunable]` and
   `σ` the standard deviation of that same departure over the **floor
   band** — `--floor` (6) pixels just outside the ring, which the
   quadratic was not fitted to and the mark does not reach.
6. **The hole estimate**, R11 §4.1:
   `α̂(p) = 1 − std_i I_i(p) / std_i Ô_i(p)`; the share of the support
   with `α̂ ≥ --opaque` (0.95, `opaque_above`, D155) and the row of R11
   §4.1's share table it falls in (0; up to 1 %; over 1 %).
7. **What a fixed blend leaves unexplained.** If one map is blended at
   one place with one opacity, every file obeys
   `I_i(p) = α(p)·L + (1 − α(p))·O_i(p)`: per pixel, `I` is a straight
   line in the background. So per pixel, least squares `I_i = a + s·Ô_i`
   over the files (`α_fit = 1 − s`, written as `alpha_fit.png`), and the
   residual's RMS `e(p)`. Its floor `e_floor` is the median of `e` over
   the floor band (at least `--floor-min` 0.5 levels): what the quadratic
   misses of a background where there is no mark. `q(p) = e(p)/e_floor`.
8. **Why it is unexplained**, per file, over the support: the residual
   regressed on the fitted mark's own contribution
   `B_i(p) = a(p) − (1 − s(p))·Ô_i(p) = α_fit·(L − Ô_i)` (one number per
   file: its opacity relative to the others') gives `R²_opacity`; on that
   contribution's two gradients `∂B_i/∂x`, `∂B_i/∂y` (two numbers per
   file: its shift, to first order) gives `R²_position`. Pooled over the
   files: `1 − Σ residual² / Σ r²`.
9. A **mechanical reading** of R11 §4.1's observation table, per pair:
   * the share of the support with `q > --q-high` (3) at most
     `--q-share` (5 %) → **row 1, one map** (the blend explains every file);
   * otherwise the larger of `R²_opacity` and `R²_position`, when at least
     `--r2-min` (0.3): **row 3, opacity varies** or **row 4, position
     varies**;
   * otherwise **unexplained**; and with no support at all, **none**.
   Per source, over its pairs of different sizes: the support's bounding
   box constant in pixels (within `SIZE_TOL` 10 %) → row 1's "fixed size";
   proportional to the picture's width, height, shorter or longer side or
   √area → **row 2, map ∝ size**; one size only → not readable. Between
   sources sharing a picture size and a crop: `α_fit` over the union of
   their supports, NCC ≥ 0.9 and mean |Δ| ≤ 0.05 → the same mean,
   otherwise **row 5, different sources**. And the stage-1 line R11's gate
   asks for, one per source: "a map exists" (every pair row 1), "a map
   exists after alignment" (the same, on `align.py`'s crops), "the
   position varies: align, then run again", or "no map". The reading is
   arithmetic; the conclusion — and the line in `E12-R11-stage1-<date>.md`
   — is the report's, written by whoever looked at the maps.

Why `q` and not `std/std_ring` reads the table: for one fixed blend,
`std_i I_i(p) = (1 − α(p))·std_i O_i(p)`, so `std/std_ring ≈ 1 − α`: it is
"≪ 1" only where the mark is nearly opaque, and a half-transparent wordmark
sits at 0.5 whether its opacity is fixed or not. `ratio.png` is written as
R11 asks and its median on the support is in the table; the reading uses
the part of the scatter the blend does not explain.

Input
-----
One list, in any of three shapes (the extension decides):

* **CSV or TSV** (`.csv`, `.tsv`, anything else; a tab in the header line
  makes it TSV), one row per file, with a header:
  `path,source,corner,margin_x,margin_y,mark_w,mark_h[,held_out][,sha256]`
  — `corner` one of `top-left`, `top-right`, `bottom-left`,
  `bottom-right`, the margins from that corner's two edges — or
  `path,source,mark_x,mark_y,mark_w,mark_h[,…]` for a rectangle given
  outright. `mark_w × mark_h` is the mark's **visible** size.
* **TOML** (`.toml`): `[[file]]` tables with `path`, `source`, and either
  `rect = [x, y, w, h]` or `corner`, `margin = [x, y]`, `mark = [w, h]`;
  an optional `[defaults]` table is laid under every file.
* **JSON** (`.json`): the stage-0 manifest `corpus/grok/manifest.json`
  that `scripts/corpus/manifest.py grok` writes (R2 §4.3–§4.4), read as
  it is: `{"files": [...]}`, per row `path` (relative to `--root`, the
  folder `manifest.py grok --root` was given), `sha256`, `held_out`, the
  Grok source in **`profile`** (`grok.com`, `grok-in-x`, `xai-api`; the
  row's `source` there is the ZIP's, `grok` or `grok-video`), and the
  by-hand fields in `stage0`: `mark` (`yes`/`no`), `corner`, `margin` and
  `mark_size` (`"20×20"`, as `manifest.py` joins a sidecar's list). A clip
  (`facts.kind` `clip`, source `grok-video`) and a row whose `mark` is not
  `yes` are skipped. A hand-written JSON may instead carry `source` and the
  mark as an object `mark: {corner, margin: [x, y], size: [w, h]}` or
  `mark: {rect: [x, y, w, h]}`, or the CSV's flat keys; a row whose `mark`
  is `false`, `"no"` or `"none"` is skipped (a source with no mark).
  Every other key is ignored. `normalise` is the one function that reads
  a row.

Relative paths are under `--root`, or else beside the list. A row with a
`sha256` is hashed first and refused on a mismatch (D304). `align.py`'s
`crops.csv` is a list too (its rows carry `crop`, `aligned`, the original
`width` and `height`, and the rectangle inside the crop); its `.npy`
crops are read as they are.

How to run it
-------------
    python3 scripts/grok/invariance.py run LIST --out DIR [--root DIR] [--with-held-out] \
        [--crop-factor 1.5] [--ring 6] [--guard 2] [--floor 6] [--support-min 3] [--support-k 4] \
        [--q-high 3] [--q-share 0.05] [--r2-min 0.3] [--opaque 0.95]
    python3 scripts/grok/invariance.py selftest

What it needs
-------------
Python 3.11+ (`tomllib`), numpy and Pillow (a venv is fine; nothing in the
repository depends on them — written against numpy 2.5.3, Pillow 12.3.0).
No scipy. The pictures are the host's; `selftest` needs none: it
synthesises a short white wordmark over random backgrounds and writes its
PNGs to a temporary directory.

What its output means
---------------------
Under `--out`, one folder per pair (`<source>-<W>x<H>/`):

| file | contents |
|---|---|
| `mean.png` | 8-bit RGB, `mean_i I_i`, rounded |
| `std.png` | 16-bit grey, `std_i Y_i` × 256 (one level is 256), clipped at 65535 |
| `ratio.png` | 16-bit grey, `std/std_ring` × 10000, clipped (6.5535 at most) |
| `alpha_hat.png` | 16-bit grey, `α̂` clipped to [0, 1] × 65535 |
| `alpha_fit.png` | 16-bit grey, `1 − s` clipped to [0, 1] × 65535 |
| `q.png` | 16-bit grey, `q` × 1000, clipped |
| `support.png` | 8-bit mask, 255 on the support |

and beside them `invariance.md` (per pair: the figures, the reading and
any file whose rectangle was not the pair's; per source: the size reading
and the stage-1 line; between sources: the comparison) and
`invariance.csv` (one line per pair). Exit codes: 0 done (selftest: all
passed), 1 a selftest check failed, 2 usage or a refusal (a file that does
not read, a sha256 that does not match, a rectangle outside its picture, a
ring or a floor band with fewer than 30 pixels).
"""

import argparse
import csv
import hashlib
import io
import math
import os
import re
import sys
import tempfile
from collections import Counter
from types import SimpleNamespace

import numpy as np

CROP_FACTOR = 1.5  # R11 §4.1: a margin of 1.5× the mark's visible size
RING = 6  # CalibrateOptions::default().ring
GUARD = 2  # between the rectangle and the ring  [tunable]
FLOOR = 6  # the floor band's width, outside the ring  [tunable]
SUPPORT_MIN = 3.0  # levels  [tunable]
SUPPORT_K = 4.0  # × the departure's spread on the floor band  [tunable]
FLOOR_MIN = 0.5  # levels: e_floor is never under this  [tunable]
Q_HIGH = 3.0  # [tunable]
Q_SHARE = 0.05  # [tunable]
R2_MIN = 0.3  # [tunable]
OPAQUE = 0.95  # opaque_above (D155)
HOLE_SOME = 0.01  # R11 §4.1's share table: 0 / up to 1 % / over 1 %
SIZE_TOL = 0.10  # [tunable]
SAME_NCC, SAME_DIFF = 0.9, 0.05  # [tunable]
MIN_FILES = 5  # a pair with fewer is not read  [tunable]
MIN_BAND = 30  # a ring or floor band with fewer pixels is a refusal
LUMA = np.array([0.299, 0.587, 0.114])
CORNERS = ("top-left", "top-right", "bottom-left", "bottom-right")


class Refusal(Exception):
    pass


def defaults():
    return SimpleNamespace(crop_factor=CROP_FACTOR, ring=RING, guard=GUARD, floor=FLOOR,
                           support_min=SUPPORT_MIN, support_k=SUPPORT_K, floor_min=FLOOR_MIN,
                           q_high=Q_HIGH, q_share=Q_SHARE, r2_min=R2_MIN, opaque=OPAQUE,
                           with_held_out=False)


# ── the list ─────────────────────────────────────────────────────────────────


def truthy(v):
    if isinstance(v, bool):
        return v
    return str(v).strip().lower() in ("1", "true", "yes", "y")


def given(d, k):
    return k in d and d[k] not in (None, "")


def pair_of(v, where, what):
    """Two integers from `[a, b]`, `"a×b"`, `"axb"`, `"a,b"` or `"a b"`."""
    if isinstance(v, (list, tuple)) and len(v) == 2:
        return int(v[0]), int(v[1])
    parts = [p for p in re.split(r"[×xX,\s]+", str(v).strip()) if p]
    if len(parts) != 2:
        raise Refusal(f"{where}: {what} {v!r} is not two numbers")
    return int(float(parts[0])), int(float(parts[1]))


def from_stage0(d, where):
    """A row of `manifest.py grok`'s manifest (R2 §4.4) in the shape below,
    or None for a clip or a row without a mark."""
    facts = d.get("facts") or {}
    if facts.get("kind") == "clip" or d.get("source") == "grok-video":
        return None
    hand = d.get("stage0") or {}
    if str(hand.get("mark") or "").strip().lower() not in ("yes", "y", "true", "1"):
        return None
    if not given(hand, "margin") or not given(hand, "mark_size"):
        raise Refusal(f"{where}: stage0 says there is a mark but not its margin and mark_size")
    (mx, my), (w, h) = pair_of(hand["margin"], where, "margin"), pair_of(hand["mark_size"], where, "mark_size")
    return {"path": d["path"], "source": d.get("profile") or d.get("source"), "held_out": d.get("held_out"),
            "sha256": d.get("sha256"), "corner": hand.get("corner") or "bottom-right",
            "margin_x": mx, "margin_y": my, "mark_w": w, "mark_h": h}


def normalise(d, where):
    """One row of any input shape → {path, source, held_out, sha256, and a
    rectangle: mark_x/mark_y/mark_w/mark_h, or corner/margin_x/margin_y/
    mark_w/mark_h}, plus align's crop fields; None for a row with no mark."""
    if isinstance(d.get("stage0"), dict) or isinstance(d.get("facts"), dict):
        d = from_stage0(d, where)
        if d is None:
            return None
    path = d.get("path") or d.get("zip_path") or d.get("file")
    source = d.get("source")
    if not path or not source:
        raise Refusal(f"{where}: a row needs a path and a source")
    mark = d.get("mark")
    if mark is False or (isinstance(mark, str) and mark.strip().lower() in ("no", "none", "false")):
        return None
    r = {"path": str(path), "source": str(source), "held_out": truthy(d.get("held_out", False)),
         "sha256": d.get("sha256") or None}
    m = mark if isinstance(mark, dict) else d
    if m.get("rect") is not None:
        x, y, w, h = (int(v) for v in m["rect"])
        r.update(mark_x=x, mark_y=y, mark_w=w, mark_h=h)
    elif all(given(m, k) for k in ("mark_x", "mark_y", "mark_w", "mark_h")):
        r.update({k: int(m[k]) for k in ("mark_x", "mark_y", "mark_w", "mark_h")})
    else:
        size = m.get("size") if isinstance(mark, dict) else (mark if isinstance(mark, list) else None)
        if size is None and given(m, "mark_w") and given(m, "mark_h"):
            size = (m["mark_w"], m["mark_h"])
        margin = m.get("margin")
        if margin is None and given(m, "margin_x") and given(m, "margin_y"):
            margin = (m["margin_x"], m["margin_y"])
        if size is None or margin is None:
            raise Refusal(f"{where}: no mark rectangle (rect, or corner + margin + size)")
        corner = str(m.get("corner") or "bottom-right")
        if corner not in CORNERS:
            raise Refusal(f"{where}: corner {corner!r} is not one of {', '.join(CORNERS)}")
        r.update(corner=corner, margin_x=int(margin[0]), margin_y=int(margin[1]),
                 mark_w=int(size[0]), mark_h=int(size[1]))
    if truthy(d.get("crop", False)):
        r.update(crop=True, width=int(d["width"]), height=int(d["height"]),
                 aligned=truthy(d.get("aligned", False)))
    return r


def read_rows(path, root=None):
    ext = os.path.splitext(path)[1].lower()
    if ext == ".json":
        import json
        with open(path) as f:
            m = json.load(f)
        raw = m.get("files", m.get("rows")) if isinstance(m, dict) else m
        if not isinstance(raw, list):
            raise Refusal(f"{path}: no list of files (`files`, `rows` or a bare list)")
    elif ext == ".toml":
        import tomllib
        with open(path, "rb") as f:
            t = tomllib.load(f)
        base = t.get("defaults", {})
        raw = [{**base, **f} for f in t.get("file", [])]
    else:
        with open(path, newline="") as f:
            text = f.read()
        lines = text.splitlines()
        delim = "\t" if lines and "\t" in lines[0] else ","
        raw = list(csv.DictReader(io.StringIO(text), delimiter=delim))
    base = root or os.path.dirname(os.path.abspath(path))
    rows = []
    for i, d in enumerate(raw):
        r = normalise(d, f"{path}: row {i + 1}")
        if r is None:
            continue
        if not os.path.isabs(r["path"]):
            r["path"] = os.path.join(base, r["path"])
        if r["sha256"]:
            with open(r["path"], "rb") as f:
                got = hashlib.sha256(f.read()).hexdigest()
            if got != str(r["sha256"]).lower():
                raise Refusal(f"{r['path']}: sha256 {got} is not the row's {r['sha256']}")
        rows.append(r)
    if not rows:
        raise Refusal(f"{path}: no file to read")
    return rows


# ── pictures and pairs ───────────────────────────────────────────────────────


def size_of(path):
    if path.endswith(".npy"):
        s = np.load(path, mmap_mode="r").shape
        return s[1], s[0]
    from PIL import Image
    with Image.open(path) as im:
        return im.size


def load(path):
    """An 8-bit-scaled float picture, h × w × 3."""
    if path.endswith(".npy"):
        a = np.load(path).astype(np.float64)
        if a.ndim != 3 or a.shape[2] != 3:
            raise Refusal(f"{path}: not an h × w × 3 array")
        return a
    from PIL import Image
    with Image.open(path) as im:
        if im.mode not in ("RGB", "RGBA", "P", "L", "LA", "PA"):
            raise Refusal(f"{path}: mode {im.mode} — this script reads 8-bit pictures")
        if (im.format or "").upper() == "JPEG":
            print(f"  note: {path} is a JPEG; Pillow's decoder is not zune-jpeg's, values may differ "
                  "by a level, and a subsampled one belongs on R3's planes (R11 §4.2)", file=sys.stderr)
        return np.asarray(im.convert("RGB"), dtype=np.float64)


def rect_of(r, W, H):
    """The mark's rectangle (x, y, w, h) in its picture."""
    w, h = r["mark_w"], r["mark_h"]
    if "mark_x" in r:
        x, y = r["mark_x"], r["mark_y"]
    else:
        corner = r["corner"]
        x = W - r["margin_x"] - w if corner.endswith("right") else r["margin_x"]
        y = H - r["margin_y"] - h if corner.startswith("bottom") else r["margin_y"]
    if w <= 0 or h <= 0 or x < 0 or y < 0 or x + w > W or y + h > H:
        raise Refusal(f"{r['path']}: the mark's rectangle {(x, y, w, h)} is not inside {W}x{H}")
    return x, y, w, h


def crop_box(mark, W, H, factor):
    x, y, w, h = mark
    mx, my = int(math.ceil(factor * w)), int(math.ceil(factor * h))
    return max(x - mx, 0), max(y - my, 0), min(x + w + mx, W), min(y + h + my, H)


class Pair:
    def __init__(self, source, W, H, names, crops, mark, box, aligned, notes):
        self.source, self.W, self.H = source, W, H
        self.names, self.crops, self.mark, self.box = names, crops, mark, box
        self.aligned, self.notes = aligned, notes

    @property
    def key(self):
        return f"{self.source}-{self.W}x{self.H}"


def pairs_of(rows, opts):
    """Group the rows by (source, W, H), crop each pair at one integer offset."""
    groups = {}
    for r in rows:
        if r["held_out"] and not opts.with_held_out:
            continue
        W, H = (r["width"], r["height"]) if r.get("crop") else size_of(r["path"])
        groups.setdefault((r["source"], W, H), []).append(r)
    pairs = []
    for (source, W, H), members in sorted(groups.items()):
        names = [os.path.basename(r["path"]) for r in members]
        if members[0].get("crop"):
            rects = {(r["mark_x"], r["mark_y"], r["mark_w"], r["mark_h"]) for r in members}
            if len(rects) != 1 or not all(r.get("crop") for r in members):
                raise Refusal(f"{source} {W}x{H}: crops that do not share one rectangle")
            mark = rects.pop()
            crops = [load(r["path"]) for r in members]
            if len({c.shape for c in crops}) != 1:
                raise Refusal(f"{source} {W}x{H}: crops of different sizes")
            box = (0, 0, crops[0].shape[1], crops[0].shape[0])
            pairs.append(Pair(source, W, H, names, crops, mark, box,
                              all(r.get("aligned") for r in members), []))
            continue
        rects = [rect_of(r, W, H) for r in members]
        mark = Counter(rects).most_common(1)[0][0]
        notes = [f"{n}: its own rectangle {rc} is not the pair's {mark}; cropped at the pair's"
                 for n, rc in zip(names, rects) if rc != mark]
        x0, y0, x1, y1 = crop_box(mark, W, H, opts.crop_factor)
        crops = []
        for r in members:
            img = load(r["path"])
            if img.shape[:2] != (H, W):
                raise Refusal(f"{r['path']}: decoded at {img.shape[1]}x{img.shape[0]}, not {W}x{H}")
            crops.append(img[y0:y1, x0:x1, :].copy())
        local = (mark[0] - x0, mark[1] - y0, mark[2], mark[3])
        pairs.append(Pair(source, W, H, names, crops, local, (x0, y0, x1, y1), False, notes))
    return pairs


# ── the background under the mark ────────────────────────────────────────────


def distance(shape, mark):
    """Chebyshev distance of every pixel of a crop from the mark's rectangle (0 inside)."""
    h, w = shape
    x, y, mw, mh = mark
    yy, xx = np.mgrid[0:h, 0:w]
    dx = np.maximum(np.maximum(x - xx, xx - (x + mw - 1)), 0)
    dy = np.maximum(np.maximum(y - yy, yy - (y + mh - 1)), 0)
    return np.maximum(dx, dy)


def bands(shape, mark, guard, ring, floor):
    d = distance(shape, mark)
    zone = d <= guard
    ring_m = (d > guard) & (d <= guard + ring)
    floor_m = (d > guard + ring) & (d <= guard + ring + floor)
    return zone, ring_m, floor_m


def basis(u, v):
    return np.stack([np.ones_like(u), u, v, u * u, u * v, v * v], axis=-1)


def quadratic(crop, mark, ring_mask):
    """`calibrate.rs`'s `background` with no clean twin, restated over
    `ring_mask` and evaluated over the whole crop (h × w × 3)."""
    h, w, _ = crop.shape
    x, y, mw, mh = mark
    cx, cy = x + mw / 2.0, y + mh / 2.0
    s = float(max(mw, mh, 1))
    yy, xx = np.mgrid[0:h, 0:w]
    B = basis((xx + 0.5 - cx) / s, (yy + 0.5 - cy) / s)
    A = B[ring_mask]
    if A.shape[0] < MIN_BAND:
        raise Refusal(f"a ring of {A.shape[0]} pixels (fewer than {MIN_BAND})")
    out = np.empty_like(crop)
    for c in range(3):
        k, *_ = np.linalg.lstsq(A, crop[..., c][ring_mask], rcond=None)
        out[..., c] = B @ k
    return out


# ── the statistics ───────────────────────────────────────────────────────────


def analyse(pair, opts):
    """The figures and maps of one pair (see the header, steps 3–8)."""
    n = len(pair.crops)
    shape = pair.crops[0].shape[:2]
    zone, ring, floor = bands(shape, pair.mark, opts.guard, opts.ring, opts.floor)
    if floor.sum() < MIN_BAND:
        raise Refusal(f"{pair.key}: a floor band of {int(floor.sum())} pixels (fewer than {MIN_BAND}); "
                      "a larger --crop-factor or a smaller --floor")
    rgb = np.stack(pair.crops)
    I = rgb @ LUMA
    O = np.stack([quadratic(c, pair.mark, ring) for c in pair.crops]) @ LUMA
    mean_I, std_I = I.mean(0), I.std(0, ddof=1)
    mean_O, std_O = O.mean(0), O.std(0, ddof=1)
    std_ring = float(np.median(std_I[ring]))
    ratio = std_I / max(std_ring, 1e-9)
    alpha_hat = 1.0 - std_I / np.maximum(std_O, 1e-9)

    dO, dI = O - mean_O, I - mean_I
    s = (dO * dI).sum(0) / np.maximum((dO * dO).sum(0), 1e-9)
    a = mean_I - s * mean_O
    r = I - (a + s * O)
    e = np.sqrt((r * r).sum(0) / max(n - 2, 1))
    e_floor = max(float(np.median(e[floor])), opts.floor_min)
    q = e / e_floor

    dep = mean_I - mean_O
    sigma = float(np.std(dep[floor]))
    T = max(opts.support_min, opts.support_k * sigma)
    support = zone & (np.abs(dep) > T)
    m = int(support.sum())

    st = {"source": pair.source, "size": f"{pair.W}x{pair.H}", "files": n, "crop": pair.box,
          "mark": pair.mark, "std_ring": std_ring, "e_floor": e_floor, "T": T, "support_px": m,
          "zone_px": int(zone.sum()), "aligned": pair.aligned}
    if m:
        B = a[None] - (1.0 - s)[None] * O
        gy, gx = np.gradient(B, axis=(1, 2))
        rs = r[:, support]
        total = float((rs * rs).sum())

        def explained(cols):
            if total <= 0:
                return 0.0
            left = 0.0
            for i in range(n):
                X = np.stack([c[i][support] for c in cols], axis=1)
                k, *_ = np.linalg.lstsq(X, rs[i], rcond=None)
                left += float(((rs[i] - X @ k) ** 2).sum())
            return 1.0 - left / total

        ys, xs = np.nonzero(support)
        hole = float((alpha_hat[support] >= opts.opaque).mean())
        st.update(ratio_median=float(np.median(ratio[support])),
                  alpha_hat_p50=float(np.median(alpha_hat[support])),
                  alpha_hat_p90=float(np.percentile(alpha_hat[support], 90)),
                  alpha_fit_max=float((1.0 - s)[support].max()),
                  q_median=float(np.median(q[support])),
                  q_share=float((q[support] > opts.q_high).mean()),
                  r2_opacity=explained([B]), r2_position=explained([gx, gy]),
                  hole_share=hole, hole_row=hole_row(hole),
                  bbox=(int(xs.min()), int(ys.min()), int(xs.max() - xs.min() + 1), int(ys.max() - ys.min() + 1)))
    st["reading"], st["why"] = reading(st, opts)
    maps = {"mean": rgb.mean(0), "std": std_I, "ratio": ratio, "alpha_hat": alpha_hat,
            "alpha_fit": 1.0 - s, "q": q, "support": support}
    return st, maps


def hole_row(share):
    if share <= 0.0:
        return "row 1 (0: a full inverse, as Gemini)"
    if share <= HOLE_SOME:
        return "row 2 (up to 1 %: inverse + holes, exit 3 by holes)"
    return "row 3 (over 1 %: every file exit 3, inpainting a blocker; Q-R6)"


def reading(st, opts):
    if not st["support_px"]:
        return "none", "no support: nothing departs from the ring's quadratic at this rectangle"
    share, op, pos = st["q_share"], st["r2_opacity"], st["r2_position"]
    if share <= opts.q_share:
        return "one map", (f"row 1 of §4.1: {share:.1%} of the support has q > {opts.q_high:g} "
                           f"(<= {opts.q_share:.0%}) — one blend explains every file")
    if op >= pos and op >= opts.r2_min:
        return "opacity varies", (f"row 3 of §4.1: {share:.1%} of the support has q > {opts.q_high:g}; "
                                  f"a per-file opacity explains R² {op:.2f} of it (a shift {pos:.2f})")
    if pos > op and pos >= opts.r2_min:
        return "position varies", (f"row 4 of §4.1: {share:.1%} of the support has q > {opts.q_high:g}; "
                                   f"a per-file shift explains R² {pos:.2f} of it (an opacity {op:.2f})")
    return "unexplained", (f"{share:.1%} of the support has q > {opts.q_high:g}; neither an opacity "
                           f"(R² {op:.2f}) nor a shift ({pos:.2f}) reaches {opts.r2_min:g}")


# ── across pairs and sources ─────────────────────────────────────────────────


def spread(values):
    v = [float(x) for x in values]
    return max(v) / min(v) - 1.0 if min(v) > 0 else float("inf")


def size_reading(results):
    """Row 1's fixed size, row 2's map ∝ size, over one source's pairs."""
    seen = [(st, int(st["size"].split("x")[0]), int(st["size"].split("x")[1]))
            for st, _ in results if st["support_px"]]
    if len({(W, H) for _, W, H in seen}) < 2:
        return "one picture size: not readable"
    bw = [st["bbox"][2] for st, _, _ in seen]
    bh = [st["bbox"][3] for st, _, _ in seen]
    if spread(bw) <= SIZE_TOL and spread(bh) <= SIZE_TOL:
        return f"row 1 of §4.1: the support is {min(bw)}–{max(bw)} × {min(bh)}–{max(bh)} px at every size (fixed size)"
    for name, f in (("width", lambda W, H: W), ("height", lambda W, H: H),
                    ("shorter side", min), ("longer side", max), ("√area", lambda W, H: math.sqrt(W * H))):
        sides = [f(W, H) for _, W, H in seen]
        if spread([b / s for b, s in zip(bw, sides)]) <= SIZE_TOL and \
                spread([b / s for b, s in zip(bh, sides)]) <= SIZE_TOL:
            return f"row 2 of §4.1: the support's size follows the picture's {name} (map ∝ size)"
    return "neither: the support's size is neither fixed nor proportional to one side"


def compare(a, b):
    """Two pairs of different sources at one size and one crop: their α_fit."""
    (sa, ma), (sb, mb) = a, b
    if sa["crop"] != sb["crop"] or sa["mark"] != sb["mark"] or ma["alpha_fit"].shape != mb["alpha_fit"].shape:
        return "not comparable (different rectangles)"
    union = ma["support"] | mb["support"]
    if union.sum() < MIN_BAND:
        return "not comparable (no support)"
    x, y = ma["alpha_fit"][union], mb["alpha_fit"][union]
    xc, yc = x - x.mean(), y - y.mean()
    ncc = float((xc * yc).sum() / max(math.sqrt(float((xc * xc).sum() * (yc * yc).sum())), 1e-12))
    diff = float(np.abs(x - y).mean())
    same = ncc >= SAME_NCC and diff <= SAME_DIFF
    return (f"NCC {ncc:.3f}, mean |Δα| {diff:.3f}: "
            + ("the same mean" if same else "row 5 of §4.1, different sources"))


def stage1_line(results):
    readings = [st["reading"] for st, _ in results]
    aligned = all(st["aligned"] for st, _ in results)
    if readings and all(r == "one map" for r in readings):
        return "a map exists after alignment" if aligned else "a map exists"
    if any(r == "position varies" for r in readings) and not aligned:
        return "the position varies: align.py, then invariance.py again"
    return "no map (Q-R4)"


# ── writing ──────────────────────────────────────────────────────────────────


def write_maps(folder, maps):
    from PIL import Image
    os.makedirs(folder, exist_ok=True)

    def grey16(name, v):
        Image.fromarray(np.clip(np.round(v), 0, 65535).astype(np.uint16)).save(os.path.join(folder, name))

    Image.fromarray(np.clip(np.round(maps["mean"]), 0, 255).astype(np.uint8)).save(os.path.join(folder, "mean.png"))
    grey16("std.png", maps["std"] * 256.0)
    grey16("ratio.png", maps["ratio"] * 10000.0)
    grey16("alpha_hat.png", np.clip(maps["alpha_hat"], 0.0, 1.0) * 65535.0)
    grey16("alpha_fit.png", np.clip(maps["alpha_fit"], 0.0, 1.0) * 65535.0)
    grey16("q.png", maps["q"] * 1000.0)
    Image.fromarray(np.where(maps["support"], 255, 0).astype(np.uint8)).save(os.path.join(folder, "support.png"))


COLUMNS = ("source", "size", "files", "aligned", "crop", "mark", "support_px", "zone_px", "T", "std_ring",
           "ratio_median", "alpha_hat_p50", "alpha_hat_p90", "alpha_fit_max", "hole_share", "hole_row",
           "e_floor", "q_median", "q_share", "r2_opacity", "r2_position", "bbox", "reading")


def fmt(v):
    if isinstance(v, float):
        return f"{v:.4g}"
    if isinstance(v, tuple):
        return " ".join(str(x) for x in v)
    return "" if v is None else str(v)


def render(results, notes):
    out = ["# Stage 1 — invariance and holes (`scripts/grok/invariance.py`)", ""]
    by_source = {}
    for (st, maps) in results:
        by_source.setdefault(st["source"], []).append((st, maps))
    out += ["## Per source", "", "| source | pairs | stage-1 line | size |", "|---|---|---|---|"]
    for source, rs in by_source.items():
        out.append(f"| {source} | {len(rs)} | {stage1_line(rs)} | {size_reading(rs)} |")
    out.append("")
    out += ["## Per pair", ""]
    for st, _ in results:
        out += [f"### {st['source']} {st['size']} — {st['reading']}", "", st["why"], "",
                "| figure | value |", "|---|---|"]
        out += [f"| {c} | {fmt(st.get(c))} |" for c in COLUMNS if c not in ("source", "size", "reading")]
        out.append("")
        for line in notes.get((st["source"], st["size"]), []):
            out.append(f"* {line}")
        out.append("")
    sizes = {}
    for st, maps in results:
        sizes.setdefault(st["size"], []).append((st, maps))
    lines = []
    for size, rs in sizes.items():
        for i in range(len(rs)):
            for j in range(i + 1, len(rs)):
                if rs[i][0]["source"] != rs[j][0]["source"]:
                    lines.append(f"| {size} | {rs[i][0]['source']} | {rs[j][0]['source']} | {compare(rs[i], rs[j])} |")
    if lines:
        out += ["## Between sources", "", "| size | source | source | α_fit |", "|---|---|---|---|"] + lines + [""]
    out += ["The reading is arithmetic (the header of `invariance.py`); the conclusion is the stage-1 "
            "report's.", ""]
    return "\n".join(out)


def run(rows, opts, out_dir):
    pairs = pairs_of(rows, opts)
    results, notes, skipped = [], {}, []
    for p in pairs:
        if len(p.crops) < MIN_FILES:
            skipped.append(f"{p.key}: {len(p.crops)} files (fewer than {MIN_FILES}), not read")
            continue
        st, maps = analyse(p, opts)
        results.append((st, maps))
        notes[(p.source, f"{p.W}x{p.H}")] = p.notes
        if out_dir:
            write_maps(os.path.join(out_dir, p.key), maps)
    text = render(results, notes) + "".join(f"\n* skipped — {s}" for s in skipped) + "\n"
    if out_dir:
        os.makedirs(out_dir, exist_ok=True)
        with open(os.path.join(out_dir, "invariance.md"), "w") as f:
            f.write(text)
        with open(os.path.join(out_dir, "invariance.csv"), "w", newline="") as f:
            w = csv.writer(f)
            w.writerow(COLUMNS)
            for st, _ in results:
                w.writerow([fmt(st.get(c)) for c in COLUMNS])
    return results, text


# ── synthesis (the self-tests of this script and of align.py) ────────────────

# A 5 × 7 bitmap face, the leftmost cell in bit 4: a wordmark drawn from
# strokes, no font file.
FONT = {
    "G": (0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111),
    "K": (0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001),
    "X": (0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001),
}
WORD = "GKX"
SCALE = 5  # pixels per cell: strokes 5 px wide
SS = 8  # supersampling, and the jitter's grain (1/8 px)
PAD = 4
SYNTH_W, SYNTH_H, SYNTH_MARGIN = 320, 200, (24, 20)
WHITE, DARK = 255.0, 20.0


def word_size():
    return (len(WORD) * 6 - 1) * SCALE, 7 * SCALE


def dilate(m, r):
    t = m.copy()
    for d in range(1, r + 1):
        t[:, d:] = np.maximum(t[:, d:], m[:, :-d])
        t[:, :-d] = np.maximum(t[:, :-d], m[:, d:])
    u = t.copy()
    for d in range(1, r + 1):
        u[d:, :] = np.maximum(u[d:, :], t[:-d, :])
        u[:-d, :] = np.maximum(u[:-d, :], t[d:, :])
    return u


def soften(a, sigma=0.8):
    """A Gaussian blur of `sigma` px: rendered text is band-limited."""
    k = np.exp(-0.5 * (np.arange(-3, 4) / sigma) ** 2)
    k /= k.sum()
    a = np.apply_along_axis(lambda v: np.convolve(v, k, mode="same"), 1, a)
    return np.apply_along_axis(lambda v: np.convolve(v, k, mode="same"), 0, a)


def synth_alpha(rect, offset8=(0, 0), peak=0.5, hole=False, outline=False):
    """The white text's α and its dark outline's α over the whole synthetic
    picture: the word drawn at SS× with an offset of `offset8` / SS px,
    box-averaged down, softened. With `hole`, the first letter is at 0.98."""
    x, y, w, h = rect
    hw, hh = (w + 2 * PAD) * SS, (h + 2 * PAD) * SS
    value = np.zeros((hh, hw))
    cover = np.zeros((hh, hw))
    for li, ch in enumerate(WORD):
        v = 0.98 if hole and li == 0 else peak
        for row, bits in enumerate(FONT[ch]):
            for c in range(5):
                if (bits >> (4 - c)) & 1:
                    x0 = (PAD + (li * 6 + c) * SCALE) * SS + offset8[0]
                    y0 = (PAD + row * SCALE) * SS + offset8[1]
                    value[y0:y0 + SCALE * SS, x0:x0 + SCALE * SS] = v
                    cover[y0:y0 + SCALE * SS, x0:x0 + SCALE * SS] = 1.0

    def down(a):
        return soften(a.reshape(hh // SS, SS, hw // SS, SS).mean(axis=(1, 3)))

    text = np.zeros((SYNTH_H, SYNTH_W))
    dark = np.zeros((SYNTH_H, SYNTH_W))
    text[y - PAD:y + h + PAD, x - PAD:x + w + PAD] = down(value)
    if outline:
        dark[y - PAD:y + h + PAD, x - PAD:x + w + PAD] = 0.6 * down(dilate(cover, SS) - cover)
    return text, dark


def value_noise(rng, W, H, cell):
    gh, gw = H // cell + 2, W // cell + 2
    g = rng.uniform(-1.0, 1.0, (gh, gw))
    yv, xv = np.arange(H) / cell, np.arange(W) / cell
    iy, ix = yv.astype(int), xv.astype(int)
    ty, tx = yv - iy, xv - ix
    top = g[iy][:, ix] * (1 - tx) + g[iy][:, ix + 1] * tx
    bottom = g[iy + 1][:, ix] * (1 - tx) + g[iy + 1][:, ix + 1] * tx
    return top * (1 - ty)[:, None] + bottom * ty[:, None]


def synth_background(rng, textured):
    """A random background: a level anywhere in 20–235, a quadratic tilt, a
    slow wave, a tint, noise of 1.5 levels; with `textured`, a fine texture."""
    W, H = SYNTH_W, SYNTH_H
    yy, xx = np.mgrid[0:H, 0:W]
    u, v = xx / W - 0.5, yy / H - 0.5
    c = rng.uniform(-1.0, 1.0, 5) * np.array([20.0, 20.0, 10.0, 10.0, 10.0])
    base = rng.uniform(20.0, 235.0) + c[0] * u + c[1] * v + c[2] * u * u + c[3] * u * v + c[4] * v * v
    base = base + 2.0 * value_noise(rng, W, H, 60)
    if textured:
        base = base + 2.0 * value_noise(rng, W, H, 3)
    rgb = base[..., None] + rng.uniform(-10.0, 10.0, 3)
    rgb = rgb + rng.normal(0.0, 1.5, rgb.shape)
    return np.clip(rgb, 0.0, 255.0)


def synth_pair(folder, name, n, seed, peak=0.5, k=None, jitter=0.0, hole=False, outline=False):
    """`n` PNGs of one synthetic source and its CSV list; the true offsets in px."""
    from PIL import Image
    rng = np.random.default_rng(seed)
    w, h = word_size()
    rect = (SYNTH_W - SYNTH_MARGIN[0] - w, SYNTH_H - SYNTH_MARGIN[1] - h, w, h)
    fixed = None if jitter else synth_alpha(rect, (0, 0), peak, hole, outline)
    rows, truth = [], []
    for i in range(n):
        o8 = (0, 0)
        if jitter:
            o8 = tuple(int(v) for v in np.round(rng.uniform(-jitter, jitter, 2) * SS))
        text, dark = fixed if fixed is not None else synth_alpha(rect, o8, peak, hole, outline)
        g = rng.uniform(*k) if k else 1.0
        text, dark = text * g, dark * g
        O = synth_background(rng, textured=(i % 4 == 3))
        img = text[..., None] * WHITE + dark[..., None] * DARK + (1.0 - text - dark)[..., None] * O
        fname = f"{name}-{i:03d}.png"
        Image.fromarray(np.clip(np.round(img), 0, 255).astype(np.uint8)).save(os.path.join(folder, fname))
        rows.append([fname, name, "bottom-right", SYNTH_MARGIN[0], SYNTH_MARGIN[1], w, h])
        truth.append((o8[0] / SS, o8[1] / SS))
    path = os.path.join(folder, f"{name}.csv")
    with open(path, "w", newline="") as f:
        wr = csv.writer(f)
        wr.writerow(["path", "source", "corner", "margin_x", "margin_y", "mark_w", "mark_h"])
        wr.writerows(rows)
    return path, truth


def reads_r2s_manifest(tmp, n):
    """`scripts/corpus/manifest.py grok` (R2) over synthetic captures; its
    manifest through `read_rows` and `run`."""
    import importlib.util
    import json
    import shutil

    here = os.path.dirname(os.path.abspath(__file__))
    spec = importlib.util.spec_from_file_location("corpus_manifest", os.path.join(here, "..", "corpus", "manifest.py"))
    manifest = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(manifest)
    root = os.path.join(tmp, "grok-root")
    for source in ("grok.com", "xai-api", "grok-imagine-video"):
        os.makedirs(os.path.join(root, source))
    listing, _ = synth_pair(tmp, "r2", n, 7)
    w, h = word_size()
    sidecar = os.path.join(tmp, "grok.toml")
    with open(sidecar, "w") as f:
        for i in range(n):
            name = f"r2-{i:03d}.png"
            for source, mark in (("grok.com", "yes"), ("xai-api", "no")):
                shutil.copy(os.path.join(tmp, name), os.path.join(root, source, name))
                f.write(f'[[file]]\npath = "{source}/{name}"\ndate = "2026-10-09"\nmark = "{mark}"\n'
                        f'corner = "bottom-right"\nmargin = [{SYNTH_MARGIN[0]}, {SYNTH_MARGIN[1]}]\n'
                        f'mark_size = [{w}, {h}]\n\n')
        with open(os.path.join(root, "grok-imagine-video", "clip.mp4"), "wb") as clip:
            clip.write(b"\x00\x00\x00\x18ftypmp42" + bytes(64))
        f.write('[[file]]\npath = "grok-imagine-video/clip.mp4"\ndate = "2026-10-09"\nmark = "yes"\n'
                'corner = "bottom-right"\nmargin = [20, 20]\nmark_size = [90, 30]\n')
    path = os.path.join(tmp, "manifest.json")
    md = os.path.join(tmp, "stage0.md")
    with open(os.devnull, "w") as null:
        old = sys.stdout
        sys.stdout = null
        try:
            code = manifest.main(["grok", "--root", root, "--sidecar", sidecar, "--out", md, "--manifest", path])
        finally:
            sys.stdout = old
    if code not in (0, 3):
        print(f"     manifest.py grok exited {code}")
        return False
    with open(path) as f:
        held = sum(1 for r in json.load(f)["files"] if r["held_out"] and r["path"].startswith("grok.com/"))
    rows = read_rows(path, root)
    pairs = pairs_of(rows, defaults())
    ok = [p.source for p in pairs] == ["grok.com"] and len(pairs[0].crops) == n - held and held == n // 5
    results, _ = run(rows, defaults(), None)
    return ok and [st["reading"] for st, _ in results] == ["one map"]


def one(listing, opts=None):
    """The statistics of the one pair a synthetic list holds."""
    opts = opts or defaults()
    pairs = pairs_of(read_rows(listing), opts)
    assert len(pairs) == 1, [p.key for p in pairs]
    return analyse(pairs[0], opts)


def selftest():
    failures = []

    def check(ok, what):
        print(("ok   " if ok else "FAIL ") + what)
        if not ok:
            failures.append(what)

    with tempfile.TemporaryDirectory() as tmp:
        n = 40
        fixed, _ = synth_pair(tmp, "fixed", n, 1)
        st, maps = one(fixed)
        check(st["reading"] == "one map", f"a fixed mark reads as one map: {st['why']}")
        # std_ring is the backgrounds' scatter where there is no mark: the
        # floor band, farther out, reads the same; the zone, which holds the
        # mark, reads less (central check, 2026-10-09: R11's M1 was green
        # on the ratio alone).
        pair = pairs_of(read_rows(fixed), defaults())[0]
        _, _, floor_m = bands(maps["std"].shape, pair.mark, GUARD, RING, FLOOR)
        floor_std = float(np.median(maps["std"][floor_m]))
        check(abs(st["std_ring"] / floor_std - 1.0) <= 0.03,
              f"… std_ring is the backgrounds' scatter outside the mark: {st['std_ring']:.2f} against the floor "
              f"band's {floor_std:.2f}")
        check(abs(st["alpha_hat_p90"] - 0.5) <= 0.05,
              f"… α̂'s 90th percentile on the support is the peak 0.5: {st['alpha_hat_p90']:.3f}")
        check(abs(st["ratio_median"] - (1.0 - st["alpha_hat_p50"])) <= 0.15,
              f"… std/std_ring ≈ 1 − α̂ on the support, not ≪ 1: {st['ratio_median']:.3f} against "
              f"{1.0 - st['alpha_hat_p50']:.3f}")
        check(st["hole_share"] == 0.0 and st["hole_row"].startswith("row 1"),
              f"… at α 0.5 there is no hole: {st['hole_share']:.4f}, {st['hole_row']}")

        st, _ = one(synth_pair(tmp, "outlined", n, 2, outline=True)[0])
        check(st["reading"] == "one map", f"a fixed mark with a dark outline reads as one map: {st['why']}")

        st, _ = one(synth_pair(tmp, "opacity", n, 3, k=(0.6, 1.0))[0])
        check(st["reading"] == "opacity varies", f"an opacity drawn per file from 0.6–1.0 reads as such: {st['why']}")

        st, _ = one(synth_pair(tmp, "jitter", n, 4, jitter=1.5)[0])
        check(st["reading"] == "position varies", f"a position jittered by ±1.5 px reads as such: {st['why']}")

        st, _ = one(synth_pair(tmp, "hole", n, 5, hole=True)[0])
        check(st["hole_share"] > HOLE_SOME and st["hole_row"].startswith("row 3"),
              f"a letter at α 0.98 is a hole, over 1 % of the support: {st['hole_share']:.3f}, {st['hole_row']}")

        st, _ = one(synth_pair(tmp, "peak06", n, 6, peak=0.6)[0])
        check(st["hole_share"] == 0.0 and st["hole_row"].startswith("row 1"),
              f"a mark at α 0.6 everywhere has no hole: {st['hole_share']:.4f}, {st['hole_row']}")

        lists = [os.path.join(tmp, f"{s}.csv") for s in ("fixed", "opacity", "jitter")]
        rows = [r for p in lists for r in read_rows(p)]
        out = os.path.join(tmp, "out")
        results, text = run(rows, defaults(), out)
        lines = {st["source"]: stage1_line([(st, None)]) for st, _ in results}
        check(lines == {"fixed": "a map exists", "opacity": "no map (Q-R4)",
                        "jitter": "the position varies: align.py, then invariance.py again"},
              f"the stage-1 lines: {lines}")
        check(all(os.path.exists(os.path.join(out, "fixed-320x200", f)) for f in
                  ("mean.png", "std.png", "ratio.png", "alpha_hat.png", "alpha_fit.png", "q.png", "support.png"))
              and os.path.exists(os.path.join(out, "invariance.md")),
              "the maps, invariance.md and invariance.csv are written")
        check("## Between sources" in text, "three sources at one size are compared")

        # R2's own tool writes the list: `manifest.py grok` over a folder of
        # the source's captures and a sidecar, its manifest read as it is.
        check(reads_r2s_manifest(tmp, n), "R2's manifest.py grok manifest is read: one pair per Grok source, "
              "the clip and the source without a mark skipped, the held-out files left out")
    print(f"selftest: {'all passed' if not failures else str(len(failures)) + ' failed'}")
    return 1 if failures else 0


def options(ap):
    ap.add_argument("--crop-factor", type=float, default=CROP_FACTOR)
    ap.add_argument("--ring", type=int, default=RING)
    ap.add_argument("--guard", type=int, default=GUARD)
    ap.add_argument("--with-held-out", action="store_true")
    ap.add_argument("--root")


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("run")
    r.add_argument("list")
    r.add_argument("--out", required=True)
    options(r)
    r.add_argument("--floor", type=int, default=FLOOR)
    r.add_argument("--support-min", type=float, default=SUPPORT_MIN)
    r.add_argument("--support-k", type=float, default=SUPPORT_K)
    r.add_argument("--floor-min", type=float, default=FLOOR_MIN)
    r.add_argument("--q-high", type=float, default=Q_HIGH)
    r.add_argument("--q-share", type=float, default=Q_SHARE)
    r.add_argument("--r2-min", type=float, default=R2_MIN)
    r.add_argument("--opaque", type=float, default=OPAQUE)
    sub.add_parser("selftest")
    args = ap.parse_args(argv)
    try:
        if args.cmd == "selftest":
            return selftest()
        _, text = run(read_rows(args.list, args.root), args, args.out)
        print(text)
        return 0
    except Refusal as e:
        print(f"refused: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
