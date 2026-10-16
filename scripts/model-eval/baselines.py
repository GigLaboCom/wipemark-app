#!/usr/bin/env python3
"""The controls LaMa must beat: OpenCV's NS and Telea, and the ring's mean.

What it is for
--------------
Step E12-R10 of the E12-R series (`docs/plan/E12-R10-model-evaluation.md`
§3.2–§3.4), filed 2026-10-08 by the coordinator from the owner's spec
`wipemark-recon-spec-2026-10-08` (`07-model-evaluation.md` §3; the owner's
S10: LaMa for Grok only, and only if Grok has holes). Written 2026-10-09,
ahead of the run, at the owner's request that all code work be finished
first. LaMa is a 51-million-parameter inpainter; if a classical method that
needs no runtime does as well inside a hole, LaMa is not needed (M2, M7,
Q-R3). This module holds those classical methods and the mask every
inpainter here is given, so that `lama_run.py` and these controls cannot
be handed different masks.

What it does
------------
* `hard_mask(crop)` — R10 §3.3: `M_hard = dilate(holes, 2 px) ∪
  open(w > 0.6, 3 × 3)`. `holes` are the exporter's (`holes_rle`, `α` at or
  over `opaque_above`). `w` is, per pixel, the share of its three samples
  the inverse called unrecoverable: clamped (the unclamped inverse outside
  the range by more than half a level) or locally inconsistent (`|α·L +
  (1 − α)·O − I| > 4h`, `h = 0.5 + 2·σ_base`, R8) — over the pixels the
  restoration wrote (`α` from the noise floor to under the threshold). So
  `w > 0.6` is "at least two samples of three"; the opening by 3 × 3 keeps
  no lone pixel.
* `ns`, `telea` — `cv2.inpaint` with `INPAINT_NS` / `INPAINT_TELEA`,
  radius 3, on the 8-bit crop (OpenCV imported inside them only).
* `ring_mean` — pure numpy: every 4-connected component of the mask is
  filled with the mean of the pixels within three of it and outside the
  mask.
* `composite(recon, pred, mask)` — `pred` inside the mask, `recon` byte for
  byte outside it: the composite LaMa's output gets too.
* `run` — every crop through the three controls, one PNG per method and a
  JSON of PSNR inside the mask (against `gt.png` where the crop has one);
  `lama_run.py` calls the same functions and puts them in its own report.

How to run it
-------------
    python3 scripts/model-eval/baselines.py run --crops bench/out/<grok-run>/crops/<R*> --out <dir> \
        [--methods ns,telea,ring-mean]
    python3 scripts/model-eval/baselines.py selftest

What it needs
-------------
Python 3.10+, numpy, Pillow; `opencv-python` for `ns` and `telea` (the
version to pin is in `scripts/model-eval/README.md`). `selftest` needs no
OpenCV: it runs the ring mean and the mask, and NS/Telea only if OpenCV
imports.

What its output means
---------------------
`<out>/<crop>/<method>.png` and `<out>/baselines.json` (per crop and method:
the mask's pixel count, PSNR inside the mask, the time). Exit 0 when the
run completed, 2 on a refusal.
"""

import argparse
import os
import sys
import time

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import evalkit as ek  # noqa: E402

HOLE_DILATE = 2
W_BAR = 0.6
W_OPEN = 1          # radius: a 3 × 3 square
LOCAL_H = 4.0       # `local consistency_px > 4h`
INPAINT_RADIUS = 3
RING = 3

METHODS = ("ns", "telea", "ring-mean")


def unrecoverable_share(crop):
    """`w`: per pixel, the share of its samples clamped or locally inconsistent by over 4h."""
    a = crop.alpha
    live = (a >= ek.NOISE_FLOOR) & (a < crop.opaque)
    clamped = ek.clamped_samples(crop.input, a, crop.logo, crop.opaque)
    local = ek.local_consistency(crop.recon, crop.input, a, crop.logo) > LOCAL_H * ek.h_of(crop.sigma_base())
    bad = (clamped | local) & live[..., None]
    return bad.mean(axis=-1)


def hard_mask(crop):
    """R10 §3.3's `M_hard`, and its two halves."""
    holes = ek.dilate(crop.holes, HOLE_DILATE)
    w = ek.opening(unrecoverable_share(crop) > W_BAR, W_OPEN)
    return holes | w, holes, w


def composite(recon, pred, mask):
    """`pred` where the mask holds, `recon` byte for byte everywhere else (R10 §3.2: mandatory)."""
    m = np.asarray(mask, dtype=bool)[..., None]
    return np.where(m, ek.to_u8(pred), recon).astype(np.uint8)


def ring_mean(img, mask, ring=RING):
    """Each 4-connected component of the mask filled with the mean of the pixels within `ring` of it, outside the mask."""
    out = np.asarray(img, dtype=np.float64).copy()
    m = np.asarray(mask, dtype=bool)
    labels, n = ek.components(m)
    fallback = out[~m].mean(axis=0) if (~m).any() else np.full(3, 127.5)
    for i in range(1, n + 1):
        comp = labels == i
        around = ek.dilate(comp, ring) & ~m
        out[comp] = out[around].mean(axis=0) if around.any() else fallback
    return out


def _cv2_inpaint(img, mask, flag_name, radius=INPAINT_RADIUS):
    import cv2

    flag = getattr(cv2, flag_name)
    m = (np.asarray(mask, dtype=bool) * 255).astype(np.uint8)
    return cv2.inpaint(np.ascontiguousarray(img, dtype=np.uint8), m, radius, flag).astype(np.float64)


def ns(img, mask):
    return _cv2_inpaint(img, mask, "INPAINT_NS")


def telea(img, mask):
    return _cv2_inpaint(img, mask, "INPAINT_TELEA")


FUNCTIONS = {"ns": ns, "telea": telea, "ring-mean": ring_mean}


def cv2_version():
    try:
        import cv2

        return cv2.__version__
    except ImportError:
        return None


def run_one(crop, methods, save=None):
    m_hard, _holes, _w = hard_mask(crop)
    recs = []
    for name in methods:
        t = time.monotonic()
        out = composite(crop.recon, FUNCTIONS[name](crop.recon, m_hard), m_hard)
        dt = time.monotonic() - t
        if not np.array_equal(out[~m_hard], crop.recon[~m_hard]):
            raise ek.Refusal(f"{crop.dir}: {name} moved a pixel outside the mask")
        if save:
            ek.write_png(os.path.join(save, crop.name, name + ".png"), out)
        recs.append({"crop": crop.name, "method": name, "mask_px": int(m_hard.sum()), "seconds": dt,
                     "psnr_in_mask": ek.psnr_mask(out, crop.gt, m_hard) if crop.gt is not None else None})
    return recs


def cmd_run(args):
    dirs = ek.load_crops(args.crops)
    if not dirs:
        raise ek.Refusal(f"{args.crops}: no crop folder")
    methods = args.methods.split(",") if args.methods else list(METHODS)
    unknown = [m for m in methods if m not in FUNCTIONS]
    if unknown:
        raise ek.Refusal(f"unknown method(s) {unknown}; known: {', '.join(METHODS)}")
    recs = []
    for d in dirs:
        crop = ek.Crop(d)
        if crop.restored:
            recs += run_one(crop, methods, args.out)
    ek.write_json(os.path.join(args.out, "baselines.json"),
                  {"environment": dict(ek.environment(), opencv=cv2_version()), "records": recs})
    print(f"{len(recs)} result(s): {os.path.join(args.out, 'baselines.json')}")
    return 0


# ───────────────────────────────────────────────────────── selftest

def _t_the_hard_mask_is_the_dilated_holes_and_the_unrecoverable():
    c = ek.synthetic_crop(seed=7, hole=True, peak=0.6)
    m, holes, w = hard_mask(c)
    assert c.holes.any() and holes.sum() > c.holes.sum(), "the holes are dilated"
    assert np.array_equal(m, holes | w)
    # A pixel that the restoration put far from the data joins the mask when two of three samples are off.
    # (A quiet background: σ_base ≈ 2, so 4h ≈ 18 levels.)
    c2 = ek.synthetic_crop(seed=8, noise=2.0)
    y, x = np.argwhere((c2.alpha > 0.2) & (c2.alpha < 0.4))[0]
    c2.recon = c2.recon.copy()
    c2.recon[y - 1:y + 2, x - 1:x + 2, :2] = np.where(c2.recon[y - 1:y + 2, x - 1:x + 2, :2] > 127, 0, 255)
    _m, _h, w2 = hard_mask(c2)
    assert w2[y, x], "a 3 × 3 block off by far in two channels is unrecoverable"


def _t_the_ring_mean_fills_only_the_mask():
    img = np.zeros((20, 20, 3))
    img[:, :, 0] = 100.0
    img[8:12, 8:12] = 0.0
    m = np.zeros((20, 20), bool)
    m[8:12, 8:12] = True
    out = ring_mean(img, m)
    assert np.allclose(out[m][:, 0], 100.0) and np.array_equal(out[~m], img[~m])


def _t_the_composite_keeps_recon_outside_the_mask():
    c = ek.synthetic_crop(seed=9, hole=True)
    m, _h, _w = hard_mask(c)
    out = composite(c.recon, np.zeros_like(c.recon, dtype=np.float64), m)
    assert np.array_equal(out[~m], c.recon[~m]) and np.all(out[m] == 0)


def _t_opencv_inpaints_when_it_is_there():
    if cv2_version() is None:
        print("        (OpenCV is not installed: NS and Telea are not run here)")
        return
    c = ek.synthetic_crop(seed=10, hole=True)
    m, _h, _w = hard_mask(c)
    for f in (ns, telea):
        out = composite(c.recon, f(c.recon, m), m)
        assert np.array_equal(out[~m], c.recon[~m])


def selftest():
    return ek.run_cases([
        ("the_hard_mask_is_the_dilated_holes_and_the_unrecoverable", _t_the_hard_mask_is_the_dilated_holes_and_the_unrecoverable),
        ("the_ring_mean_fills_only_the_mask", _t_the_ring_mean_fills_only_the_mask),
        ("the_composite_keeps_recon_outside_the_mask", _t_the_composite_keeps_recon_outside_the_mask),
        ("opencv_inpaints_when_it_is_there", _t_opencv_inpaints_when_it_is_there),
    ])


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = p.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("run")
    s.add_argument("--crops", required=True)
    s.add_argument("--out", required=True)
    s.add_argument("--methods")
    sub.add_parser("selftest")
    args = p.parse_args(argv)
    try:
        return selftest() if args.cmd == "selftest" else cmd_run(args)
    except ek.Refusal as e:
        print(f"baselines.py: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
