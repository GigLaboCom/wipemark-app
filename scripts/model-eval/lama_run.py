#!/usr/bin/env python3
"""Big-LaMa inside a Grok mark's holes, against NS, Telea and the ring mean: M1–M7.

What it is for
--------------
Step E12-R10 of the E12-R series (`docs/plan/E12-R10-model-evaluation.md`
§3 and §4), filed 2026-10-08 by the coordinator from the owner's spec
`wipemark-recon-spec-2026-10-08` (`07-model-evaluation.md` §3; the owner's
S10). Written 2026-10-09, ahead of the run, at the owner's request that all
code work be finished first. It runs only when `trigger.py lama` says LaMa
is mandatory or an option — that is, only if R11 stage 1 finds Grok's mark
opaque somewhere (`α̂ ≥ opaque_above`). Gemini is never evaluated: no
Gemini map has a hole. LaMa (Suvorov et al., "Resolution-robust Large Mask
Inpainting with Fourier Convolutions", WACV 2022; the `big-lama`
checkpoint, ~51 M parameters, FFC blocks, trained on Places) **invents**
what is under a hole; so it never runs where the inverse returned a pixel,
its output is composited back inside the hole mask only, and a pixel it
made is *reconstructed*, never *restored* (D310).

What it does
------------
Per crop (Grok's, from `recon_bench run --export-crops` with R11's profile
and the accepted R*, or `regress.py run --export-crops` — see `evalkit.py`):

0. A crop no restoration made is skipped (M5: on holes, after `restore`
   only); a crop with no hole and nothing unrecoverable has nothing to
   inpaint and is counted apart.
1. **The mask**: `M_hard = dilate(holes, 2) ∪ open(w > 0.6, 3 × 3)`
   (`baselines.hard_mask`, the same for every method).
2. **The crop**: the ROI and 128 px around it (`--context`), mirror-padded
   to sides that are multiples of 8. Where the exported crop gives less
   context than asked (`recon_bench` exports 64 px), the shortfall is
   mirror-padded and recorded per crop (`context_px`); export with more
   (`export_crops --pad 128`) to avoid it.
3. **LaMa**: `predict(image, mask)` — image RGB in [0, 1], mask in {0, 1} —
   is `lama_predictor`, the one integration point: the reference code of
   https://github.com/advimman/lama (`saicinpainting`), the `big-lama`
   checkpoint, on the CPU, its raw `predicted_image` (not its own
   `inpainted`, so the composite below is ours and checked).
4. **The composite, mandatory**: `out = mask·pred + (1 − mask)·recon`;
   outside the mask `out` is `recon.png` byte for byte, checked on every
   output (a refusal, `OutsideTheMask`, stops the run).
5. **Variants**: `m-hard` (the whole mask, hard); `holes-only` (LaMa told
   only the dilated holes); `chroma-only` (the mask, luma from `recon` and
   chroma from LaMa — a wordmark's shape kept); `feather-3` (the mask's
   seam feathered over 3 px **inward** — weights 1/3, 2/3, 1 from the edge —
   so M1 still holds; the halo it may leave is measured).
6. **The controls** (`baselines.py`) on the same mask: NS, Telea, the
   ring's mean.
7. **Measures**: PSNR inside the mask against `gt.png` (Grok-synthetic,
   R12 §3); LPIPS (AlexNet, `lpips`) over the mask's bounding box and 8 px
   around it, with `--lpips`; the luma step, the colour step and the
   texture after inpainting (`verify.rs`'s, restated: band `α` ∈ [3/255,
   0.2], around = the rectangle under the floor and a ring 4 out) — the
   outline's share needs the contour energy the mark had before and is
   not restated, so M3 is read on three of its four measures and the
   report says so; `consistency_px` inside the mask (reported) and the
   largest change outside it (M4: 0).

The gates (§3.4): **M1** byte-equal outside `M_hard` (checked); **M2**
inside, on crops with a truth: LaMa (`m-hard`) better than NS, Telea and
the ring mean in PSNR and in LPIPS at the median **and** p5, and on `text`
PSNR no worse than NS; **M3** the three restated measures within their
bounds after LaMa on ≥ 90 % of the crops that were a mark left by holes;
**M4** outside the mask nothing moved, inside `consistency_px` reported;
**M5** every inpainted crop was restored; **M6** `--ab-holes` (`ab.py`
question `pair`, "with holes" against "+ LaMa", 30 pairs) LaMa preferred
in ≥ 80 %; **M7** `--ab-ns` (LaMa against NS, 20 pairs) LaMa preferred in
≥ 65 %, or NS is enough.

How to run it
-------------
    # in LaMa's own venv (its requirements.txt), with this repository's numpy/Pillow beside them:
    python3 scripts/model-eval/lama_run.py run --crops bench/out/<grok-run>/crops/<R*> \
        --lama-dir ~/src/lama --checkpoint ~/models/big-lama --out bench/out/<grok-run>/lama \
        [--context 128] [--lpips] [--ab-holes ab/lama-holes/score.json] [--ab-ns ab/lama-ns/score.json] \
        [--variants m-hard,holes-only,chroma-only,feather-3] [--no-images]
    python3 scripts/model-eval/lama_run.py selftest

What it needs
-------------
Python 3.10+, numpy, Pillow; for `run`: a checkout of
https://github.com/advimman/lama at the commit `scripts/model-eval/README.md`
pins, its requirements (torch, PyTorch Lightning, OmegaConf, kornia… as its
`requirements.txt` says), the `big-lama` checkpoint folder (`config.yaml`,
`models/best.ckpt`), `opencv-python` for NS/Telea, and `lpips` for
`--lpips`. All imported inside the functions that use them. `selftest`
needs none: it runs a stub predictor.

What its output means
---------------------
`<out>/report.md` — the checkpoint's sha256, LaMa's commit, the framework
versions, the device, the time per crop; every variant and control per
group at the median and p5; the `consistency_px` distribution; M1–M7; and
**one closing line** — "not needed" (NS is enough, or Grok has no holes) or
"needed, variant …, +X dB over NS inside the holes; integration: Q-R3".
`<out>/results.json`, `<out>/images/<crop>/<variant>.png` (what `ab.py`
shows). Exit 0 when the run completed, 2 on a refusal.
"""

import argparse
import json
import os
import subprocess
import sys
import time

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import baselines as bl  # noqa: E402
import evalkit as ek  # noqa: E402

CONTEXT = 128
MULTIPLE = 8
FEATHER = 3
LPIPS_MARGIN = 8
VARIANTS = ("m-hard", "holes-only", "chroma-only", "feather-3")
# ── [tunable] — R10 §3.4; each moves only with a line in a report ──
M3_SHARE = 0.90
M6_SHARE = 0.80
M7_SHARE = 0.65


class OutsideTheMask(ek.Refusal):
    """A pixel outside the mask moved: the composite was skipped or is wrong."""


# ───────────────────────────────────────────────────────── the one integration point

def lama_predictor(lama_dir, checkpoint, device="cpu"):
    """Big-LaMa by its reference code: a `predict(image, mask) -> image` and the facts that pin it.

    `image` is (H, W, 3) floats in [0, 1], `mask` (H, W) in {0, 1}, both with
    sides that are multiples of 8; the answer is LaMa's `predicted_image`,
    (H, W, 3) in [0, 1] — before LaMa's own composite, so ours is the one
    that is checked. This mirrors `bin/predict.py` of advimman/lama."""
    sys.path.insert(0, os.path.abspath(lama_dir))
    import torch
    import yaml
    from omegaconf import OmegaConf
    from saicinpainting.training.trainers import load_checkpoint

    with open(os.path.join(checkpoint, "config.yaml")) as f:
        config = OmegaConf.create(yaml.safe_load(f))
    config.training_model.predict_only = True
    config.visualizer.kind = "noop"
    ckpt = os.path.join(checkpoint, "models", "best.ckpt")
    model = load_checkpoint(config, ckpt, strict=False, map_location="cpu")
    model.freeze()
    model.to(device)

    def predict(image, mask):
        img = torch.from_numpy(np.ascontiguousarray(np.transpose(image, (2, 0, 1))[None], dtype=np.float32)).to(device)
        m = torch.from_numpy((np.asarray(mask) > 0).astype(np.float32)[None, None]).to(device)
        with torch.no_grad():
            batch = model({"image": img, "mask": m})
        return np.transpose(batch["predicted_image"][0].detach().cpu().numpy(), (1, 2, 0)).astype(np.float64)

    try:
        commit = subprocess.run(["git", "-C", lama_dir, "rev-parse", "HEAD"], capture_output=True, text=True,
                                check=True).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        commit = None
    facts = {"model": "big-lama", "checkpoint": os.path.abspath(checkpoint), "checkpoint_sha256": ek.sha256_file(ckpt),
             "config_sha256": ek.sha256_file(os.path.join(checkpoint, "config.yaml")), "lama_commit": commit,
             "torch": torch.__version__, "device": device}
    return predict, facts


# ───────────────────────────────────────────────────────── the method

def window(crop, context=CONTEXT):
    """The ROI and `context` px around it, inside the crop: (y0, y1, x0, x1) and the shortfall per side."""
    h, w = crop.shape
    rx0, ry0, rx1, ry1 = crop.roi
    want = (ry0 - context, ry1 + context, rx0 - context, rx1 + context)
    got = (max(0, want[0]), min(h, want[1]), max(0, want[2]), min(w, want[3]))
    short = (got[0] - want[0], want[1] - got[1], got[2] - want[2], want[3] - got[3])
    return got, short


def inpaint(predict, img_u8, mask, short=(0, 0, 0, 0)):
    """LaMa over one window: mirror-pad the context's shortfall, then to ×8; predict; cut back."""
    top, bottom, left, right = short
    x = img_u8.astype(np.float64) / 255.0
    m = np.asarray(mask, dtype=np.float64)
    if any(short):
        x = np.pad(x, ((top, bottom), (left, right), (0, 0)), mode="symmetric")
        m = np.pad(m, ((top, bottom), (left, right)), mode="constant")
    h, w = x.shape[:2]
    xp, _ = ek.pad_to_multiple(x, MULTIPLE)
    mp, _ = ek.pad_to_multiple(m, MULTIPLE)
    mp = (mp > 0.5).astype(np.float64)
    pred = np.asarray(predict(xp, mp), dtype=np.float64)[:h, :w]
    return pred[top:h - bottom, left:w - right] * 255.0


def feather(mask, width=FEATHER):
    """Inward feathering: the mean of the mask eroded 0 … width − 1 times — 1/width at its edge, 1 inside."""
    m = np.asarray(mask, dtype=bool)
    return sum(ek.erode(m, r).astype(np.float64) for r in range(width)) / width


def compose(recon, pred, mask, weights=None, chroma_only=False):
    """The mandatory composite; refuses when a pixel outside the mask moved."""
    m = np.asarray(mask, dtype=bool)
    p = np.asarray(pred, dtype=np.float64)
    if chroma_only:
        ycc = ek.ycbcr(p)
        ycc[..., 0] = ek.ycbcr(recon)[..., 0]
        p = ek.rgb_of(ycc)
    wgt = m.astype(np.float64) if weights is None else np.asarray(weights, dtype=np.float64) * m
    rf = recon.astype(np.float64)
    out = np.where(m[..., None], ek.to_u8(wgt[..., None] * p + (1.0 - wgt[..., None]) * rf), recon).astype(np.uint8)
    if not np.array_equal(out[~m], recon[~m]):
        raise OutsideTheMask(f"{int(np.any(out != recon, axis=-1)[~m].sum())} pixel(s) outside the mask moved")
    return out


def band_steps(img, alpha, rect, opaque):
    """`verify.rs` `steps`' luma and colour step (D244, D247) and whether either is left, restated."""
    six = ek.ycbcr(np.asarray(img, dtype=np.float64))
    h, w = alpha.shape
    x0, y0, x1, y1 = rect
    band, around = [], []
    for y in range(max(y0 - 4, 0), min(y1 + 4, h)):
        for x in range(max(x0 - 4, 0), min(x1 + 4, w)):
            inside = x0 <= x < x1 and y0 <= y < y1
            a = alpha[y, x] if inside else 0.0
            if a < ek.NOISE_FLOOR:
                around.append(six[y, x])
            elif ek.BAND[0] <= a <= ek.BAND[1]:
                band.append(six[y, x])
    if not band or not around:
        return {"step": 0.0, "chroma": 0.0, "left": False}
    b, r = np.mean(band, axis=0), np.asarray(around)
    mean, var = r.mean(axis=0), r.var(axis=0)
    step, chroma = b[0] - mean[0], float(np.hypot(b[1] - mean[1], b[2] - mean[2]))
    left = abs(step) > max(ek.STEP_LEVELS, var[0] ** 0.5) or chroma > max(ek.CHROMA_LEVELS, (var[1] + var[2]) ** 0.5)
    return {"step": float(step), "chroma": chroma, "left": bool(left)}


def measures(crop, out, mask, lpips_fn=None):
    m = {"mask_px": int(np.asarray(mask).sum()),
         "moved_outside": int(np.any(out != crop.recon, axis=-1)[~np.asarray(mask, dtype=bool)].sum())}
    tex, around = ek.texture(out, crop.alpha, crop.rect_px, crop.opaque)
    bs = band_steps(out, crop.alpha, crop.rect_px, crop.opaque)
    textured = crop.lossy and tex > max(ek.TEXTURE_LEVELS, ek.TEXTURE_RATIO * around)
    m.update(texture=tex, texture_around=around, step=bs["step"], chroma=bs["chroma"],
             within_bounds=not bs["left"] and not textured)
    inside = np.asarray(mask, dtype=bool)
    live = inside & (crop.alpha >= ek.NOISE_FLOOR) & (crop.alpha < crop.opaque)
    d = ek.local_consistency(out, crop.input, crop.alpha, crop.logo)
    m["consistency_in_mask"] = ek.nearest_rank(d[live], 0.95) if live.any() else None
    if crop.gt is not None:
        m["psnr_in_mask"] = ek.psnr_mask(out, crop.gt, mask)
        if lpips_fn is not None:
            m["lpips"] = lpips_fn(out, crop.gt, mask)
    return m


def lpips_of(net="alex"):
    """LPIPS over the mask's bounding box and 8 px around it; lower is closer."""
    import lpips
    import torch

    model = lpips.LPIPS(net=net, verbose=False)

    def f(a, b, mask):
        ys, xs = np.nonzero(mask)
        if len(xs) == 0:
            return None
        h, w = np.shape(mask)
        y0, y1 = max(0, ys.min() - LPIPS_MARGIN), min(h, ys.max() + 1 + LPIPS_MARGIN)
        x0, x1 = max(0, xs.min() - LPIPS_MARGIN), min(w, xs.max() + 1 + LPIPS_MARGIN)

        def t(x):
            x = np.asarray(x, dtype=np.float32)[y0:y1, x0:x1] / 127.5 - 1.0
            return torch.from_numpy(np.ascontiguousarray(np.transpose(x, (2, 0, 1))[None]))

        with torch.no_grad():
            return float(model(t(a), t(b)).item())

    return f, {"lpips": getattr(lpips, "__version__", "unknown"), "lpips_net": net}


def run_crop(crop, predict, chosen, methods, context=CONTEXT, save=None, lpips_fn=None):
    t0 = time.monotonic()
    base = {"crop": crop.name, "group": crop.group, "slice": crop.slice, "encoder": crop.meta.get("encoder"),
            "profile": crop.meta.get("profile"), "had_holes": bool(crop.holes.any())}
    if not crop.restored:
        return [dict(base, variant=None, why="no restoration (M5)")], time.monotonic() - t0
    m_hard, holes, _w = bl.hard_mask(crop)
    if not m_hard.any():
        return [dict(base, variant=None, why="nothing to inpaint")], time.monotonic() - t0
    (y0, y1, x0, x1), short = window(crop, context)
    sub = crop.recon[y0:y1, x0:x1]
    preds = {}
    recs = [dict(base, variant="R*", context_px=context - max(short), **measures(crop, crop.recon, m_hard, lpips_fn))]

    def full(pred_win):
        p = crop.recon.astype(np.float64).copy()
        p[y0:y1, x0:x1] = pred_win
        return p

    for name in chosen:
        mask = holes if name == "holes-only" else m_hard
        key = "holes" if name == "holes-only" else "hard"
        if key not in preds:
            win_mask = mask[y0:y1, x0:x1]
            preds[key] = full(inpaint(predict, sub, win_mask, short))
        weights = feather(mask) if name == "feather-3" else None
        out = compose(crop.recon, preds[key], mask, weights=weights, chroma_only=name == "chroma-only")
        if save:
            ek.write_png(os.path.join(save, crop.name, name + ".png"), out)
        recs.append(dict(base, variant=name, context_px=context - max(short), **measures(crop, out, mask, lpips_fn)))
    for name in methods:
        out = bl.composite(crop.recon, bl.FUNCTIONS[name](crop.recon, m_hard), m_hard)
        if not np.array_equal(out[~m_hard], crop.recon[~m_hard]):
            raise OutsideTheMask(f"{crop.dir}: {name} moved a pixel outside the mask")
        if save:
            ek.write_png(os.path.join(save, crop.name, name + ".png"), out)
        recs.append(dict(base, variant=name, **measures(crop, out, m_hard, lpips_fn)))
    return recs, time.monotonic() - t0


# ───────────────────────────────────────────────────────── the gates

def better(c, b, key, lower=False):
    """Median and p5 both better (higher, or lower for LPIPS: then p95 is the tail)."""
    c = [x for x in c if x is not None]
    b = [x for x in b if x is not None]
    if not c or not b:
        return None
    if lower:
        return ek.median(c) < ek.median(b) and ek.quantile(c, 0.95) < ek.quantile(b, 0.95)
    return ek.median(c) > ek.median(b) and ek.p5(c) > ek.p5(b)


def gates(recs, ab_holes=None, ab_ns=None, candidate="m-hard"):
    by = {}
    for r in recs:
        if r.get("variant"):
            by.setdefault(r["variant"], []).append(r)
    lama = by.get(candidate, [])
    g = {"M1": {"ok": all(r["moved_outside"] == 0 for v, rs in by.items() if v != "R*" for r in rs)}}
    m2 = {}
    for ctl in bl.METHODS:
        rs = by.get(ctl, [])
        m2[ctl] = {"psnr": better([r.get("psnr_in_mask") for r in lama], [r.get("psnr_in_mask") for r in rs]),
                   "lpips": better([r.get("lpips") for r in lama], [r.get("lpips") for r in rs], lower=True)}
    text_l = [r.get("psnr_in_mask") for r in lama if r["group"] == "text"]
    text_n = [r.get("psnr_in_mask") for r in by.get("ns", []) if r["group"] == "text"]
    text_ok = None if not [x for x in text_l if x is not None] or not [x for x in text_n if x is not None] \
        else ek.median(text_l) >= ek.median(text_n)
    psnr_ok = [v["psnr"] for v in m2.values()]
    lp = [v["lpips"] for v in m2.values() if v["lpips"] is not None]
    g["M2"] = {"ok": None if None in psnr_ok else (all(psnr_ok) and all(lp) and text_ok is not False),
               "against": m2, "text_vs_ns": text_ok, "lpips_measured": bool(lp),
               "beats_ns": m2["ns"]["psnr"]}
    holed = [r for r in lama if r["had_holes"]]
    within = sum(1 for r in holed if r["within_bounds"])
    g["M3"] = {"ok": (within / len(holed) >= M3_SHARE) if holed else None, "share": within / len(holed) if holed else None,
               "n": len(holed), "note": "step, colour step and texture restated; the outline's share is not"}
    g["M4"] = {"ok": g["M1"]["ok"], "consistency_in_mask": {
        "median": ek.median([r.get("consistency_in_mask") for r in lama]),
        "p95": ek.quantile([r.get("consistency_in_mask") for r in lama], 0.95)}}
    skipped = sum(1 for r in recs if r.get("variant") is None and "M5" in str(r.get("why")))
    g["M5"] = {"ok": True, "skipped_unrestored": skipped, "inpainted": len(lama)}
    for name, ab, bar in (("M6", ab_holes, M6_SHARE), ("M7", ab_ns, M7_SHARE)):
        if ab and ab.get("question") == "pair":
            s = ab["pooled"]["prefer_share"]
            g[name] = {"ok": s >= bar, "prefer_share": s, "pairs": ab["pooled"]["items"]}
        else:
            g[name] = {"ok": None, "why": "pending: the blind A/B (Q-R7)"}
    return g


def closing_line(g, recs, candidate="m-hard"):
    if not any(r.get("variant") == candidate for r in recs):
        return "LaMa not needed: no crop had anything to inpaint (Grok has no holes) — LaMa not evaluated"
    if g["M2"]["beats_ns"] is False:
        return "LaMa not needed: it does not beat NS inside the holes (M2) — E12-7 on classical inpainting (NS/Telea in pure Rust), Q-R3"
    if g["M2"]["ok"] is None:
        return "LaMa undecided: no crop with a truth (M2 needs Grok-synthetic crops, R12 §3)"
    if g["M7"]["ok"] is False:
        return "LaMa not needed: NS is enough by the blind A/B (M7) — E12-7 on classical inpainting, Q-R3"
    gain = None
    lama = [r.get("psnr_in_mask") for r in recs if r.get("variant") == candidate]
    ns_ = [r.get("psnr_in_mask") for r in recs if r.get("variant") == "ns"]
    if lama and ns_ and ek.median(lama) is not None and ek.median(ns_) is not None:
        gain = ek.median(lama) - ek.median(ns_)
    failed = [k for k in ("M1", "M2", "M3", "M4", "M5", "M6", "M7") if g[k]["ok"] is False]
    if failed:
        return f"LaMa not needed: {', '.join(failed)} failed — a hole stays a mark left (exit 3), Q-R3"
    pending = [k for k in ("M3", "M6", "M7") if g[k]["ok"] is None]
    head = f"LaMa needed, variant {candidate}, {gain:+.2f} dB over NS inside the holes" if gain is not None else \
        f"LaMa needed, variant {candidate}"
    if pending:
        return f"{head}; {', '.join(pending)} pending — no verdict before them"
    return f"{head}; integration: Q-R3 (pixels reconstructed, never restored: D310)"


# ───────────────────────────────────────────────────────── report

def report(facts, env, recs, g, line, times, crops_dir):
    variants = [v for v in ["R*", *VARIANTS, *bl.METHODS] if any(r.get("variant") == v for r in recs)]
    groups = sorted({r["group"] for r in recs if r.get("variant")}, key=str)
    rows = []
    for v in variants:
        for gr in groups:
            rs = [r for r in recs if r.get("variant") == v and r["group"] == gr]
            if rs:
                p = [r.get("psnr_in_mask") for r in rs]
                lp = [r.get("lpips") for r in rs]
                c = [r.get("consistency_in_mask") for r in rs]
                rows.append([v, gr, len(rs), ek.median(p), ek.p5(p), ek.median(lp), ek.quantile(lp, 0.95),
                             ek.median(c), ek.quantile(c, 0.95),
                             sum(1 for r in rs if r["within_bounds"]) / len(rs)])
    md = [f"# E12-R10 — LaMa inside Grok's holes, {time.strftime('%Y-%m-%d')}", "",
          "Written by `scripts/model-eval/lama_run.py run` (R10 §3, §4). Pixels LaMa made are **reconstructed**, never "
          "restored (D310).", "",
          "## The run", "",
          f"* crops: `{crops_dir}` — {len({r['crop'] for r in recs})}; "
          f"{sum(1 for r in recs if r.get('variant') is None)} not inpainted (no restoration, or nothing to inpaint)",
          f"* checkpoint: `{facts.get('checkpoint')}` sha256 `{facts.get('checkpoint_sha256')}`; LaMa commit "
          f"{facts.get('lama_commit') or '–'}; torch {facts.get('torch')}; OpenCV {env.get('opencv')}; lpips {env.get('lpips')}",
          f"* device {facts.get('device') or env.get('device')} ({env.get('cpu')}, {env.get('platform')}); Python {env.get('python')}",
          f"* time per crop (LaMa, its variants and the controls): median {ek.fmt(ek.median(times))} s, "
          f"max {ek.fmt(max(times) if times else None)} s",
          f"* context asked {CONTEXT} px; the least a crop had: "
          f"{ek.fmt(min((r['context_px'] for r in recs if r.get('context_px') is not None), default=None))} px", "",
          "## Every variant and control, per group", "",
          ek.md_table(["variant", "group", "n", "PSNR in mask med", "p5", "LPIPS med", "p95", "consistency in mask med",
                       "p95", "measures within bounds"], rows), "",
          "## M1–M7", "",
          ek.md_table(["gate", "ok", "what"], [[k, g[k]["ok"], json.dumps({x: y for x, y in g[k].items() if x != "ok"})[:300]]
                                               for k in ("M1", "M2", "M3", "M4", "M5", "M6", "M7")]), "",
          "## Closing line", "", line, ""]
    return "\n".join(md)


def cmd_run(args):
    dirs = ek.load_crops(args.crops)
    if not dirs:
        raise ek.Refusal(f"{args.crops}: no crop folder")
    chosen = args.variants.split(",") if args.variants else list(VARIANTS)
    bad = [v for v in chosen if v not in VARIANTS]
    if bad:
        raise ek.Refusal(f"unknown variant(s) {bad}; known: {', '.join(VARIANTS)}")
    if "m-hard" not in chosen:
        chosen = ["m-hard"] + chosen  # M2 and the closing line read it
    predict, facts = lama_predictor(args.lama_dir, args.checkpoint, args.device)
    lpips_fn, lp_facts = (lpips_of() if args.lpips else (None, {}))
    os.makedirs(args.out, exist_ok=True)
    save = None if args.no_images else os.path.join(args.out, "images")
    recs, times = [], []
    for n, d in enumerate(dirs, 1):
        crop = ek.Crop(d)
        rs, t = run_crop(crop, predict, chosen, bl.METHODS, args.context, save, lpips_fn)
        recs += rs
        times.append(t)
        print(f"  [{n}/{len(dirs)}] {crop.name}: {len(rs)} result(s), {t:.2f} s", flush=True)
    ab = []
    for path in (args.ab_holes, args.ab_ns):
        if path:
            with open(path) as f:
                ab.append(json.load(f))
        else:
            ab.append(None)
    g = gates(recs, *ab)
    line = closing_line(g, recs)
    env = dict(ek.environment(args.device), opencv=bl.cv2_version(), **lp_facts)
    ek.write_json(os.path.join(args.out, "results.json"),
                  {"model": facts, "environment": env, "records": recs, "gates": g, "closing_line": line,
                   "time_per_crop_s": times})
    with open(os.path.join(args.out, "report.md"), "w") as f:
        f.write(report(facts, env, recs, g, line, times, args.crops))
    print(line)
    return 0


# ───────────────────────────────────────────────────────── selftest

def stub_predictor(image, mask):
    """A stub predictor that changes every pixel, inside the mask and out: a probe of the composite."""
    return 1.0 - np.asarray(image, dtype=np.float64)


def _t_outside_the_mask_the_output_is_the_input():
    c = ek.synthetic_crop(seed=11, hole=True, size=64, mark=(16, 16, 48, 48))
    recs, _ = run_crop(c, stub_predictor, list(VARIANTS), (), context=8)
    m_hard, holes, _w = bl.hard_mask(c)
    assert m_hard.any() and holes.any()
    inpainted = [r for r in recs if r["variant"] not in ("R*", None)]
    assert len(inpainted) == len(VARIANTS)
    for r in inpainted:
        assert r["moved_outside"] == 0, r
        assert r["mask_px"] > 0
    # …and inside the mask, the stub's answer did land (the composite is not a no-op).
    out = compose(c.recon, stub_predictor(c.recon / 255.0) * 255.0, m_hard)
    assert np.any(out[m_hard] != c.recon[m_hard])
    assert np.array_equal(out[~m_hard], c.recon[~m_hard])


def _t_the_window_is_padded_to_eight_and_cut_back():
    c = ek.synthetic_crop(seed=12, hole=True, size=50, mark=(13, 13, 37, 37))
    (y0, y1, x0, x1), short = window(c, context=30)
    assert (y0, y1, x0, x1) == (0, 50, 0, 50) and short == (21, 21, 21, 21), ((y0, y1, x0, x1), short)
    seen = {}

    def probe(image, mask):
        seen["shape"] = image.shape
        return image

    pred = inpaint(probe, c.recon, c.holes, short)
    assert seen["shape"][0] % 8 == 0 and seen["shape"][1] % 8 == 0 and seen["shape"][0] >= 92
    assert pred.shape == c.recon.shape and np.allclose(pred, c.recon)


def _t_feathering_is_inward():
    m = np.zeros((12, 12), bool)
    m[3:9, 3:9] = True
    f = feather(m)
    assert np.all(f[~m] == 0) and abs(f[3, 3] - 1 / 3) < 1e-12 and f[5, 5] == 1.0


def _t_the_gates_say_ns_is_enough_when_lama_does_not_beat_it():
    def r(v, crop, p, group="flat"):
        return {"variant": v, "crop": crop, "group": group, "psnr_in_mask": p, "moved_outside": 0,
                "within_bounds": True, "had_holes": True, "consistency_in_mask": 1.0}
    recs = []
    for i in range(10):
        recs += [r("R*", i, 20.0), r("m-hard", i, 30.0), r("ns", i, 31.0), r("telea", i, 25.0), r("ring-mean", i, 22.0)]
    g = gates(recs)
    assert g["M2"]["beats_ns"] is False and closing_line(g, recs).startswith("LaMa not needed"), closing_line(g, recs)
    recs = [dict(x, psnr_in_mask=26.0) if x["variant"] == "ns" else x for x in recs]
    g = gates(recs)
    assert g["M2"]["ok"] and "pending" in closing_line(g, recs), closing_line(g, recs)
    ab6 = {"question": "pair", "pooled": {"prefer_share": 0.85, "items": 30}}
    ab7 = {"question": "pair", "pooled": {"prefer_share": 0.60, "items": 20}}
    g = gates(recs, ab6, ab7)
    assert g["M7"]["ok"] is False and "NS is enough" in closing_line(g, recs)
    g = gates(recs, ab6, dict(ab7, pooled={"prefer_share": 0.7, "items": 20}))
    assert closing_line(g, recs).endswith("D310)"), closing_line(g, recs)


def selftest():
    return ek.run_cases([
        ("outside_the_mask_the_output_is_the_input", _t_outside_the_mask_the_output_is_the_input),
        ("the_window_is_padded_to_eight_and_cut_back", _t_the_window_is_padded_to_eight_and_cut_back),
        ("feathering_is_inward", _t_feathering_is_inward),
        ("the_gates_say_ns_is_enough_when_lama_does_not_beat_it", _t_the_gates_say_ns_is_enough_when_lama_does_not_beat_it),
    ])


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = p.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("run")
    s.add_argument("--crops", required=True)
    s.add_argument("--lama-dir", required=True, help="a checkout of advimman/lama at the pinned commit")
    s.add_argument("--checkpoint", required=True, help="the big-lama folder: config.yaml, models/best.ckpt")
    s.add_argument("--out", required=True)
    s.add_argument("--context", type=int, default=CONTEXT)
    s.add_argument("--device", default="cpu")
    s.add_argument("--lpips", action="store_true")
    s.add_argument("--ab-holes", help="ab.py score's JSON: 'with holes' against '+ LaMa' (M6)")
    s.add_argument("--ab-ns", help="ab.py score's JSON: LaMa against NS (M7)")
    s.add_argument("--variants")
    s.add_argument("--no-images", action="store_true")
    sub.add_parser("selftest")
    args = p.parse_args(argv)
    try:
        return selftest() if args.cmd == "selftest" else cmd_run(args)
    except ek.Refusal as e:
        print(f"lama_run.py: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
