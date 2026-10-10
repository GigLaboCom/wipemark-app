#!/usr/bin/env python3
"""Is there a constant bias `b` between the observed blend and `α·L + (1 − α)·O`?

What it is for
--------------
Step E12-R4 of the E12-R series (`docs/plan/E12-R4-corpus-analytics.md`
§4.3), filed 2026-10-08 by the coordinator from the owner's spec
`wipemark-recon-spec-2026-10-08` (`04-corpus-analytics.md` §3; the owner's
S8: a bias only as profile data, D497). The restoration inverts
`I = α·L + (1 − α)·O` exactly. If the vendor's encoder adds a constant —
an offset in its rounding, a dither with a mean — every restored pixel is
off by `b/(1 − α)`. This script measures the residual
`r = I − (α·L + (1 − α)·Ô)` where the original under the mark is well
estimated (the faint edge, `0.02 <= α <= 0.15`), binned by the brightness of
that original, so a constant shows as one number and anything else as a
shape. It runs on R2's `gemini-midtone` (backgrounds of every brightness);
it waits for that set.

What it does
------------
1. Reads the catalogue (`manifests/marks.v1.json`) for the profile's logo
   `L` and the named placement row, and the row's opacity map (`.wma`, under
   `crates/wipemark-pixels/marks/`). The row's rectangle is restated here
   (corner and margin, or an explicit rect); a row that resamples its map is
   refused, and a picture the row does not answer for is skipped.
2. Per picture (8-bit PNG as Gemini hands it out; Pillow decodes it), the
   original under the mark `Ô` is **the ring's quadratic, as `calibrate`
   fits it** (`crates/wipemark-pixels/src/calibrate.rs`, `background`): per
   channel, least squares of `k0 + k1·u + k2·v + k3·u² + k4·uv + k5·v²` over a
   ring `--ring` pixels wide around the rectangle (6, the calibration's
   default), clipped at the picture's edge, `u = (x + 0.5 − cx)/s`,
   `v = (y + 0.5 − cy)/s`, `s` the rectangle's side. `check-background`
   holds this restatement to the Rust fit (`map_regress --background`) on
   one file.
3. Takes the pixels with `0.02 <= α <= 0.15` (`[tunable]`, `--alpha-lo`,
   `--alpha-hi`) and per channel `r = I − (α·L + (1 − α)·Ô)` in levels; a
   stored 0 or 255 is left out (a clipped value is not on the line).
4. Bins `r` by `Ô` at the nearest of 0, 64, 128, 190, 255, and prints per
   bin and channel the count, mean, median and standard deviation — over
   all the pictures, and per group when the list gives groups. `texture`
   is left out unless `--groups` names it (a quadratic does not describe
   it).
5. Ends with a **mechanical reading** against §4.3's table, on each bin's
   median (a constant moves it as it moves the mean; the few pixels where
   the quadratic misses the original move the mean alone), over bins with
   at least 30 samples, and only when every channel has three such bins —
   one background is "not readable". White's bin is mostly empty: with `L`
   within 3 levels of 255, a white background's faint edge is stored at
   255 and left out as clipped. Every bin and channel within ±0.2 → row 4
   (nothing);
   one sign everywhere, all within ±0.2 of their common mean, and that mean
   past 0.2 → row 1 (a constant bias, its value printed); every channel
   monotonic in `Ô`, the same way, over more than 0.4 → row 2 (not a bias:
   the blend model or `L`); each channel flat in `Ô` (within 0.4) but the
   channels apart by more than 0.4 → row 3 (`L_c` off). The reading is
   arithmetic; the conclusion is the report's.
6. With `--out DIR`, writes `bias.md` (all of the above) and `bias.csv`
   (group, bin, channel, n, mean, median, sd).

`list` turns a corpus manifest (`corpus/gemini-midtone/manifest.json`, R2,
D493, as `scripts/corpus/manifest.py` writes it) into the
`path<TAB>group<TAB>held_out` list this script, `forced_search` and
`map_regress` read, checking each file's sha256 first. R2's manifest holds
both Gemini profiles, and a list is read under one profile's map, so
`--profile` keeps that profile's rows; a manifest whose rows name more than
one profile is refused without it.

How to run it
-------------
    python3 scripts/analytics/bias.py list --manifest corpus/gemini-midtone/manifest.json \
        --root <unpacked ZIP> --profile gemini-sparkle-v1 > midtone.tsv
    python3 scripts/analytics/bias.py run --profile gemini-sparkle-v1 --row 0 --list midtone.tsv [--out DIR]
    python3 scripts/analytics/bias.py run --profile gemini-sparkle-v1 --row 0 PICTURE.png [MORE …]
    cargo run --release -p wipemark-picture --example map_regress -- \
        --background PICTURE.png --profile gemini-sparkle-v1 --row 0 --out ohat.tsv
    python3 scripts/analytics/bias.py check-background PICTURE.png ohat.tsv --profile gemini-sparkle-v1 --row 0
    python3 scripts/analytics/bias.py selftest

What it needs
-------------
Python 3.10+, numpy and Pillow (a venv is fine; nothing in the repository
depends on them — checked with numpy 2.5.3, Pillow 12.3.0). The pictures
are the host's (R2's `gemini-midtone`; the owner's pictures never go into
git). `selftest` needs none: it synthesises its pictures.

What its output means
---------------------
§4.3's table. `r` of one sign and size (±0.2) in every bin and channel: a
constant bias — R9's R2b, `b` as profile data (D497). `r` growing or
falling with `Ô`: not a bias — the blend model or `L` (R9's R-lin, and
§4.4). `r` different per channel and flat in `Ô`: `L_c` is off — re-fit the
global `L` on `gemini-midtone`, then §4.4. `r` within ±0.2 everywhere:
nothing; R2b not needed. D240 saw stored values up to 6 levels **under**
`α·L` in channels with `O ≈ 0` — a structure, not a constant, expected to
land in §4.4. Exit codes: 0 done (selftest: all passed), 1 a selftest or
check failed, 2 usage or a refusal (a sha256 that does not match).
"""

import argparse
import csv
import hashlib
import io
import json
import os
import struct
import sys

import numpy as np

REPO = os.environ.get("WIPEMARK_REPO") or os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CATALOGUE = os.path.join(REPO, "manifests", "marks.v1.json")
MARKS = os.path.join(REPO, "crates", "wipemark-pixels", "marks")

ALPHA_LO, ALPHA_HI = 0.02, 0.15  # §4.3: where the original is well estimated  [tunable]
RING = 6  # CalibrateOptions::default().ring
BINS = (0.0, 64.0, 128.0, 190.0, 255.0)
BIAS_BAND = 0.2  # §4.3: ±0.2
MIN_BIN = 30  # a bin with fewer samples is not read  [tunable]
MIN_READ_BINS = 3  # a reading needs this many bins in every channel  [tunable]
SMOOTH_PREFIXES = ("gray-", "sat-")
SMOOTH_NAMES = ("black", "white", "gradient")
CHANNELS = "RGB"


class Refusal(Exception):
    pass


# ── the catalogue and the map ────────────────────────────────────────────────


def read_wma(path):
    with open(path, "rb") as f:
        data = f.read()
    if data[:4] != b"WMA1":
        raise Refusal(f"{path}: not a WMA1 file")
    w, h, depth = struct.unpack_from("<HHB", data, 4)
    if depth == 8:
        a = np.frombuffer(data, dtype=np.uint8, count=w * h, offset=9).astype(np.float64) / 255.0
    elif depth == 16:
        a = np.frombuffer(data, dtype="<u2", count=w * h, offset=9).astype(np.float64) / 65535.0
    else:
        raise Refusal(f"{path}: depth {depth}")
    return a.reshape(h, w)


def find_asset(name):
    for root, _, files in os.walk(MARKS):
        if name in files:
            return os.path.join(root, name)
    raise Refusal(f"{name}: not under {MARKS}")


def profile_of(pid):
    with open(CATALOGUE) as f:
        cat = json.load(f)
    for p in cat["profiles"]:
        if p["id"] == pid:
            return p
    raise Refusal(f"no profile {pid} in {CATALOGUE}")


def when_matches(when, w, h):
    return ((when.get("width") is None or when["width"] == w) and (when.get("height") is None or when["height"] == h)
            and (when.get("min_width") is None or w >= when["min_width"])
            and (when.get("min_height") is None or h >= when["min_height"])
            and (when.get("max_width") is None or w <= when["max_width"])
            and (when.get("max_height") is None or h <= when["max_height"]))


def row_of(profile, row):
    """The row's map (α as an array) and a function (W, H) -> rect or None."""
    rows = profile["placements"]
    if not 0 <= row < len(rows):
        raise Refusal(f"{profile['id']} has no row {row}")
    r = rows[row]
    if r.get("resample"):
        raise Refusal(f"{profile['id']} row {row} resamples its map: not measured here")
    asset = next(a for a in profile["alpha"] if a["id"] == r["alpha"])
    alpha = read_wma(find_asset(asset["asset"]))
    mh, mw = alpha.shape

    def rect(W, H):
        if not when_matches(r.get("when") or {}, W, H):
            return None
        if "rect" in r:
            x, y, w, h = r["rect"]
            if w != mw or x + w > W or y + h > H:
                return None
            return (x, y, w, h)
        mx, my = r["margin"]
        if W < mx + mw or H < my + mh or mx + mw > W or my + mh > H:
            return None
        corner = r["corner"]
        x = W - mx - mw if corner.endswith("right") else mx
        y = H - my - mh if corner.startswith("bottom") else my
        return (x, y, mw, mh)

    return alpha, rect


# ── pictures and the background under the mark ──────────────────────────────


def load(path):
    from PIL import Image
    im = Image.open(path)
    if im.mode not in ("RGB", "RGBA", "P", "L", "LA", "PA"):
        raise Refusal(f"{path}: mode {im.mode} — this script reads 8-bit pictures")
    if path.lower().endswith((".jpg", ".jpeg")):
        print(f"  note: {path} is a JPEG; Pillow's decoder is not zune-jpeg's, values may differ by a level",
              file=sys.stderr)
    return np.asarray(im.convert("RGB"), dtype=np.float64)


def basis(u, v):
    return np.stack([np.ones_like(u), u, v, u * u, u * v, v * v], axis=-1)


def ring_quadratic(img, rect, ring=RING):
    """`calibrate.rs`'s `background` with no clean twin, restated: Ô under `rect`
    (h × w × 3), or None when the ring has fewer than 30 pixels."""
    H, W, _ = img.shape
    x, y, w, h = rect
    cx, cy = x + w / 2.0, y + h / 2.0
    s = float(max(w, h, 1))
    x0, y0 = max(x - ring, 0), max(y - ring, 0)
    x1, y1 = min(x + w + ring, W), min(y + h + ring, H)
    yy, xx = np.mgrid[y0:y1, x0:x1]
    inside = (xx >= x) & (xx < x + w) & (yy >= y) & (yy < y + h)
    rx, ry = xx[~inside], yy[~inside]
    if rx.size < 30:
        return None
    A = basis((rx + 0.5 - cx) / s, (ry + 0.5 - cy) / s)
    gy, gx = np.mgrid[y:y + h, x:x + w]
    B = basis((gx + 0.5 - cx) / s, (gy + 0.5 - cy) / s)
    out = np.empty((h, w, 3))
    for c in range(3):
        k, *_ = np.linalg.lstsq(A, img[ry, rx, c], rcond=None)
        out[:, :, c] = B @ k
    return out


def residuals(img, rect, alpha, logo, lo=ALPHA_LO, hi=ALPHA_HI, ring=RING, background=ring_quadratic):
    """(Ô, r) per channel over the pixels with lo <= α <= hi, clipped samples out."""
    x, y, w, h = rect
    o = background(img, rect, ring)
    if o is None:
        return None
    i = img[y:y + h, x:x + w, :]
    sel = (alpha >= lo) & (alpha <= hi)
    out = []
    for c in range(3):
        ic, oc = i[:, :, c][sel], o[:, :, c][sel]
        a = alpha[sel]
        keep = (ic > 0) & (ic < 255)
        r = ic - (a * logo[c] + (1.0 - a) * oc)
        out.append((oc[keep], r[keep]))
    return out


# ── the table ────────────────────────────────────────────────────────────────


def nearest_bin(o):
    centres = np.array(BINS)
    return centres[np.argmin(np.abs(o[:, None] - centres[None, :]), axis=1)]


def table(samples):
    """{(bin, channel): (n, mean, median, sd)} from [(Ô, r)] per channel, pooled."""
    t = {}
    for c in range(3):
        o = np.concatenate([s[c][0] for s in samples]) if samples else np.empty(0)
        r = np.concatenate([s[c][1] for s in samples]) if samples else np.empty(0)
        if o.size == 0:
            continue
        b = nearest_bin(o)
        for centre in BINS:
            rr = r[b == centre]
            if rr.size:
                t[(centre, c)] = (int(rr.size), float(rr.mean()), float(np.median(rr)), float(rr.std()))
    return t


def reading(t):
    """§4.3's table, read on each bin's **median** — a constant bias moves the
    median and the mean alike, while the few pixels where the ring's quadratic
    misses the original drag the mean alone. Needs MIN_READ_BINS bins of at
    least MIN_BIN samples in every channel: one background is not a reading."""
    read = {k: v for k, v in t.items() if v[0] >= MIN_BIN}
    per_channel = {}
    for (b, c), v in sorted(read.items()):
        per_channel.setdefault(c, []).append(v[2])
    short = [CHANNELS[c] for c in range(3) if len(per_channel.get(c, [])) < MIN_READ_BINS]
    if short:
        return (f"not readable — channel(s) {', '.join(short)} seen in fewer than {MIN_READ_BINS} bins of "
                f"{MIN_BIN}+ samples: too few backgrounds")
    vals = [m for ms in per_channel.values() for m in ms]
    if all(abs(m) <= BIAS_BAND for m in vals):
        return "row 4 of §4.3 — r within ±0.2 everywhere: nothing"
    centre = float(np.mean(vals))
    if (all(np.sign(m) == np.sign(centre) for m in vals) and abs(centre) > BIAS_BAND
            and all(abs(m - centre) <= BIAS_BAND for m in vals)):
        return f"row 1 of §4.3 — a constant bias b = {centre:+.2f}"
    directions = set()
    monotone = True
    for ms in per_channel.values():
        d = np.diff(ms)
        if max(ms) - min(ms) <= 2 * BIAS_BAND:
            monotone = False
        elif np.all(d >= 0):
            directions.add("grows")
        elif np.all(d <= 0):
            directions.add("falls")
        else:
            monotone = False
    if monotone and len(directions) == 1:
        return f"row 2 of §4.3 — r {directions.pop()} with Ô in every channel: not a bias"
    flat = all(max(ms) - min(ms) <= 2 * BIAS_BAND for ms in per_channel.values())
    channel_means = [float(np.mean(ms)) for ms in per_channel.values()]
    if flat and max(channel_means) - min(channel_means) > 2 * BIAS_BAND:
        return "row 3 of §4.3 — r differs per channel and not with Ô: L_c is off"
    return "none of §4.3's rows by its own words — read the figures"


def render(title, t):
    out = io.StringIO()
    print(f"## {title}", file=out)
    print("  Ô bin  ch      n      mean    median      sd", file=out)
    for centre in BINS:
        for c in range(3):
            if (centre, c) in t:
                n, mean, med, sd = t[(centre, c)]
                print(f"  {centre:5.0f}  {CHANNELS[c]}  {n:7d}  {mean:+8.3f}  {med:+8.3f}  {sd:6.3f}", file=out)
    print(f"  mechanical reading: {reading(t)}", file=out)
    return out.getvalue()


# ── commands ─────────────────────────────────────────────────────────────────


def smooth(group):
    return group.startswith(SMOOTH_PREFIXES) or group in SMOOTH_NAMES


def read_list(path):
    base = os.path.dirname(os.path.abspath(path))
    out = []
    with open(path) as f:
        for line in f:
            line = line.rstrip("\n")
            if not line.strip() or line.startswith("#") or line.startswith("path\t"):
                continue
            cols = line.split("\t")
            p = cols[0] if os.path.isabs(cols[0]) else os.path.join(base, cols[0])
            out.append((p, cols[1] if len(cols) > 1 else "", len(cols) > 2 and cols[2] in ("true", "1", "yes")))
    return out


def cmd_run(args):
    profile = profile_of(args.profile)
    alpha, rect_of = row_of(profile, args.row)
    logo = [float(v) for v in profile["blend"]["logo"]]
    files = read_list(args.list) if args.list else [(p, "", False) for p in args.pictures]
    if not files:
        raise Refusal("no picture given")
    wanted = set(args.groups) if args.groups else None
    by_group, skipped = {}, []
    for path, group, held in files:
        if wanted is not None and group not in wanted:
            continue
        if wanted is None and group and not smooth(group):
            skipped.append((path, f"group {group} is not smooth"))
            continue
        img = load(path)
        rect = rect_of(img.shape[1], img.shape[0])
        if rect is None:
            skipped.append((path, f"row {args.row} does not answer for {img.shape[1]}×{img.shape[0]}"))
            continue
        res = residuals(img, rect, alpha, logo, args.alpha_lo, args.alpha_hi, args.ring)
        if res is None:
            skipped.append((path, "no ring"))
            continue
        by_group.setdefault(group or "(no group)", []).append(res)
    head = (f"# r = I − (α·L + (1 − α)·Ô): {args.profile} row {args.row}, logo {logo}, "
            f"{args.alpha_lo} <= α <= {args.alpha_hi}, ring {args.ring} px; "
            f"{sum(len(v) for v in by_group.values())} picture(s), {len(skipped)} skipped\n\n")
    text = head
    for path, why in skipped:
        text += f"skipped: {path}: {why}\n"
    allr = [r for v in by_group.values() for r in v]
    tables = {"all": table(allr)}
    text += render("all pictures", tables["all"])
    if len(by_group) > 1 or "(no group)" not in by_group:
        for g in sorted(by_group):
            tables[g] = table(by_group[g])
            text += render(f"group {g} ({len(by_group[g])})", tables[g])
    print(text, end="")
    if args.out:
        os.makedirs(args.out, exist_ok=True)
        with open(os.path.join(args.out, "bias.md"), "w") as f:
            f.write(text)
        with open(os.path.join(args.out, "bias.csv"), "w", newline="") as f:
            w = csv.writer(f)
            w.writerow(["group", "bin", "channel", "n", "mean", "median", "sd"])
            for g, t in tables.items():
                for (b, c), (n, mean, med, sd) in sorted(t.items()):
                    w.writerow([g, int(b), CHANNELS[c], n, f"{mean:.4f}", f"{med:.4f}", f"{sd:.4f}"])
    return 0


def cmd_list(args):
    with open(args.manifest) as f:
        m = json.load(f)
    rows = m if isinstance(m, list) else (m.get("files") or m.get("entries") or [])
    named = sorted({e["profile"] for e in rows if e.get("profile")})
    if args.profile is not None:
        left = [e for e in rows if e.get("profile") not in (None, args.profile)]
        if left:
            print(f"{len(left)} row(s) of another profile left out", file=sys.stderr)
        rows = [e for e in rows if e.get("profile") in (None, args.profile)]
        if not rows:
            raise Refusal(f"no row of {args.profile} in the manifest (it names {', '.join(named) or 'none'})")
    elif len(named) > 1:
        raise Refusal(f"the manifest holds {', '.join(named)}: a list is read under one profile's map, "
                      "name it with --profile")
    bad = 0
    print("path\tgroup\theld_out")
    for e in rows:
        path = os.path.join(args.root, e.get("path") or e["file"])
        if e.get("sha256") and not args.no_check:
            h = hashlib.sha256()
            with open(path, "rb") as f:
                for block in iter(lambda: f.read(1 << 20), b""):
                    h.update(block)
            if h.hexdigest() != e["sha256"]:
                print(f"{path}: sha256 {h.hexdigest()} is not the manifest's {e['sha256']}", file=sys.stderr)
                bad += 1
                continue
        print(f"{os.path.abspath(path)}\t{e.get('group', '')}\t{'true' if e.get('held_out') else 'false'}")
    if bad:
        raise Refusal(f"{bad} file(s) do not match the manifest")
    return 0


def cmd_check_background(args):
    profile = profile_of(args.profile)
    _, rect_of = row_of(profile, args.row)
    img = load(args.picture)
    rect = rect_of(img.shape[1], img.shape[0])
    if rect is None:
        raise Refusal(f"row {args.row} does not answer for {img.shape[1]}×{img.shape[0]}")
    rows = np.loadtxt(args.ohat, comments="#", skiprows=2)
    x, y, w, h = rect
    if rows.shape[0] != w * h or rows[:, 0].min() != x or rows[:, 1].min() != y:
        raise Refusal(f"{args.ohat}: not the rectangle {rect} of row {args.row}")
    ours = ring_quadratic(img, rect, args.ring)
    theirs = np.empty((h, w, 3))
    theirs[rows[:, 1].astype(int) - y, rows[:, 0].astype(int) - x, :] = rows[:, 2:5]
    d = np.abs(ours - theirs)
    worst = [float(d[:, :, c].max()) for c in range(3)]
    ok = max(worst) <= args.tolerance
    print(f"{args.picture}: rect {rect}, ring {args.ring} px; Ô here against calibrate's: "
          f"largest |Δ| R {worst[0]:.2e}, G {worst[1]:.2e}, B {worst[2]:.2e} levels "
          f"(tolerance {args.tolerance:g}): {'agree' if ok else 'DISAGREE'}")
    print(f"  Ô over the rectangle: R {ours[:, :, 0].min():.2f}–{ours[:, :, 0].max():.2f}, "
          f"G {ours[:, :, 1].min():.2f}–{ours[:, :, 1].max():.2f}, B {ours[:, :, 2].min():.2f}–{ours[:, :, 2].max():.2f}")
    return 0 if ok else 1


# ── selftest ─────────────────────────────────────────────────────────────────


def to_linear(v):
    c = np.clip(v / 255.0, 0.0, 1.0)
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def from_linear(l):
    l = np.clip(l, 0.0, 1.0)
    return np.where(l <= 0.0031308, l * 12.92, 1.055 * l ** (1 / 2.4) - 0.055) * 255.0


def synthetic(alpha, logo, centre, b=0.0, linear=False, size=220, at=62):
    """A float picture: a curved background around `centre` (a bowl, a dome at
    255) with a tilt, the map blended at (at, at), then `b` added under it."""
    h, w = alpha.shape
    yy, xx = np.mgrid[0:size, 0:size]
    u = (xx + 0.5 - (at + w / 2)) / w
    v = (yy + 0.5 - (at + h / 2)) / h
    sign = -1.0 if centre >= 255 else 1.0
    o = centre + sign * 14.0 * (u * u + v * v) + (4.0 * u if 0 < centre < 255 else 0.0)
    img = np.repeat(o[:, :, None], 3, axis=2)
    for c in range(3):
        img[:, :, c] += (c - 1) * 1.5 if 0 < centre < 255 else 0.0
    sub = img[at:at + h, at:at + w, :]
    for c in range(3):
        if linear:
            sub[:, :, c] = from_linear(alpha * to_linear(np.full_like(alpha, logo[c])) + (1 - alpha) * to_linear(sub[:, :, c]))
        else:
            sub[:, :, c] = alpha * logo[c] + (1 - alpha) * sub[:, :, c]
        sub[:, :, c] += np.where(alpha > 0, b, 0.0)
    return img, (at, at, w, h)


def lists_r2s_manifest():
    """`list` over a manifest in the schema `scripts/corpus/manifest.py`
    writes for `gemini-midtone`: rows with `id`, `source`, `path`,
    `sha256`, `profile`, `group`, `held_out`."""
    import contextlib
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        rows = []
        for i, (profile, group, held) in enumerate((("gemini-sparkle-v1", "gray-50", False),
                                                     ("gemini-sparkle-v1", "black", True),
                                                     ("gemini-sparkle-v2", "gray-50", False))):
            name = f"{profile}/{group}/f{i}.png"
            os.makedirs(os.path.join(tmp, os.path.dirname(name)), exist_ok=True)
            with open(os.path.join(tmp, name), "wb") as f:
                f.write(bytes([i]) * 16)
            rows.append({"id": f"r{i}", "source": "gemini-midtone", "path": name,
                         "sha256": hashlib.sha256(bytes([i]) * 16).hexdigest(), "profile": profile,
                         "group": group, "held_out": held, "batch": 1})
        path = os.path.join(tmp, "manifest.json")
        with open(path, "w") as f:
            json.dump({"schema": 1, "set": "gemini-midtone", "held_out": {"every": 5, "by": ["group"]},
                       "sources": {}, "files": rows, "dropped": []}, f)

        def listed(*extra):
            out, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                code = main(["list", "--manifest", path, "--root", tmp, *extra])
            return code, [line.split("\t") for line in out.getvalue().splitlines()[1:]]

        code_both, _ = listed()
        code, lines = listed("--profile", "gemini-sparkle-v1")
        ok = code_both == 2 and code == 0 and [(os.path.basename(p), g, h) for p, g, h in lines] == [
            ("f0.png", "gray-50", "false"), ("f1.png", "black", "true")]
        with open(os.path.join(tmp, rows[0]["path"]), "wb") as f:
            f.write(b"changed")
        code_bad, _ = listed("--profile", "gemini-sparkle-v1")
        return ok and code_bad == 2


def selftest():
    failures = []

    def check(ok, what):
        print(("ok   " if ok else "FAIL ") + what)
        if not ok:
            failures.append(what)

    profile = profile_of("gemini-sparkle-v1")
    alpha, rect_of = row_of(profile, 0)
    logo = [float(v) for v in profile["blend"]["logo"]]
    check(alpha.shape == (96, 96) and abs(alpha.max() - 0.5) < 0.5, f"the V1 measured map reads: {alpha.shape}, peak {alpha.max():.3f}")
    check(rect_of(2048, 2048) == (1888, 1888, 96, 96) and rect_of(1024, 1024) is None,
          "the large row is at 1888 on 2048 and does not answer for 1024")
    check(rect_of(1025, 1025) == (865, 865, 96, 96), "… and at 865 on a 1025 crop")

    for b in (1.0, 0.0):
        samples = []
        for centre in BINS:
            img, rect = synthetic(alpha, logo, centre, b=b)
            samples.append(residuals(img, rect, alpha, logo))
        t = table(samples)
        bins_seen = sorted({k[0] for k in t})
        worst = max(abs(v[1] - b) for v in t.values())
        check(bins_seen == list(BINS) and all(v[0] >= MIN_BIN for v in t.values()),
              f"b = {b:+.1f}: every bin and channel has samples ({min(v[0] for v in t.values())} at least)")
        check(worst <= 0.1, f"b = {b:+.1f}: recovered within ±0.1 in every bin and channel — worst {worst:.4f}")
        want = "row 1 of §4.3" if b else "row 4 of §4.3"
        check(reading(t).startswith(want), f"b = {b:+.1f} reads as {want}: {reading(t)}")

    # A linear-light blend read with the encoded model: r falls with Ô.
    samples = []
    for centre in BINS:
        img, rect = synthetic(alpha, logo, centre, linear=True)
        samples.append(residuals(img, rect, alpha, logo))
    t = table(samples)
    check(reading(t).startswith("row 2 of §4.3"), f"a linear-light blend reads as row 2: {reading(t)}")

    # Clipped samples are left out.
    img, rect = synthetic(alpha, logo, 255.0, b=3.0)
    img = np.minimum(img, 255.0)
    r = residuals(img, rect, alpha, logo)
    check(all(np.all(r[c][1] < 3.5) for c in range(3)) and r[0][0].size < (alpha >= ALPHA_LO).sum(),
          "a sample clipped at 255 is left out")

    # R2's manifest (`scripts/corpus/manifest.py`'s rows) through `list`.
    check(lists_r2s_manifest(), "R2's manifest is listed under one profile, sha256 checked, held-out kept; "
          "two profiles without --profile refused")

    # One background is not a reading, whatever its numbers.
    img, rect = synthetic(alpha, logo, 128.0, b=1.0)
    t = table([residuals(img, rect, alpha, logo)])
    check(reading(t).startswith("not readable"), f"one background is not readable: {reading(t)}")
    print(f"selftest: {'all passed' if not failures else str(len(failures)) + ' failed'}")
    return 1 if failures else 0


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("run")
    r.add_argument("pictures", nargs="*")
    r.add_argument("--profile", required=True)
    r.add_argument("--row", type=int, required=True)
    r.add_argument("--list")
    r.add_argument("--groups", nargs="+", help="only these groups (texture is left out otherwise)")
    r.add_argument("--alpha-lo", type=float, default=ALPHA_LO)
    r.add_argument("--alpha-hi", type=float, default=ALPHA_HI)
    r.add_argument("--ring", type=int, default=RING)
    r.add_argument("--out")
    li = sub.add_parser("list")
    li.add_argument("--manifest", required=True)
    li.add_argument("--root", required=True)
    li.add_argument("--no-check", action="store_true")
    li.add_argument("--profile", help="keep this profile's rows (R2's manifest holds both Gemini profiles)")
    cb = sub.add_parser("check-background")
    cb.add_argument("picture")
    cb.add_argument("ohat")
    cb.add_argument("--profile", required=True)
    cb.add_argument("--row", type=int, required=True)
    cb.add_argument("--ring", type=int, default=RING)
    cb.add_argument("--tolerance", type=float, default=1e-6)
    sub.add_parser("selftest")
    args = ap.parse_args(argv)
    try:
        if args.cmd == "selftest":
            return selftest()
        return {"run": cmd_run, "list": cmd_list, "check-background": cmd_check_background}[args.cmd](args)
    except Refusal as e:
        print(e, file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
