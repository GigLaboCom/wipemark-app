#!/usr/bin/env python3
"""FDnCNN over the restoration's crops: 18 variants, F1–F7, one closing line.

What it is for
--------------
Step E12-R10 of the E12-R series (`docs/plan/E12-R10-model-evaluation.md`
§2.3–§2.5 and §4), filed 2026-10-08 by the coordinator from the owner's
spec `wipemark-recon-spec-2026-10-08` (`07-model-evaluation.md` §2; the
owner's S10). Written 2026-10-09, ahead of the run, at the owner's request
that all code work be finished first. It runs only when `trigger.py
fdncnn` says "FDnCNN to be run", and only on the output of the **accepted**
R6/R8 (R*), never on R0's. The question it answers: does a denoiser after
the restoration take off what the codec's error left (`texture`, a fringe)
where the information is present? FDnCNN invents nothing; it cannot bring
back a hole.

What it does
------------
Per crop (a folder `recon_bench run --export-crops` or `regress.py run
--export-crops` wrote — see `evalkit.py`):

0. A crop no restoration made is skipped and counted (F7: FDnCNN runs only
   after `verified` and `restore`). A crop of a lossless source (`png`,
   a resize saved as PNG) is not denoised (F4: the interval there is ≤ 2
   levels and any smoothing costs `exact`): its output is `recon.png`
   itself, and that is checked.
1. `recon.png` as floats in [0, 1], mirror-padded by 24 px.
2. A σ map, not a scalar: `σ(p) = clamp(σ_base / (1 − α(p)) + k_edge·|∇α(p)|,
   σ_base, 75)`, with `σ_base` R8 §4.1's (the exporter's `meta.json`, or
   restated on `input.png` by `evalkit.sigma_base`; the largest of the three
   channels) and `k_edge` ∈ {0, 5, 10}; `|∇α|` by central differences, `α`
   in [0, 1].
3. `den = FDnCNN([R, G, B, σ_map/255])`, by onnxruntime on the CPU over the
   model `fdncnn_export.py` wrote.
4. Masked: `M = clamp(strength · smoothstep(0, 0.1, α), 0, 1)` with
   `strength` ∈ {0.6, 1.0, 1.5}, and `out = M·den + (1 − M)·recon`, rounded
   to 8 bits. **Where `α = 0`, `out` is `recon.png` byte for byte**, and
   this is checked on every output: a crop where it is not is a refusal
   that stops the run (`ModelLeak`).
5. `luma-only`: FDnCNN over the BT.601 luma (Y, Y, Y), the mean of its three
   outputs as the new Y, chroma (Cb, Cr) from `recon`.

That is 3 × 3 × 2 = 18 variants (`k<k_edge>-s<strength>-<rgb|luma>`).
Each is measured in the crop's ROI against `gt.png` (the bench's crops have
one; R1's have none, so F1/F2 read only the bench): PSNR, SSIM (luma,
7 × 7) and ΔE2000 as `recon_bench` measures them; `texture / texture_around`
(D250, restated); `consistency_px` against `input.png` (D305, restated)
beside `2h`, `h = 0.5 + 2·σ_base` (R8). R* — `recon.png` itself — is
measured the same way, so every comparison is one implementation against
itself.

The gates (§2.4), per variant:

* **F1** target slices (`jpeg444-q95`, `-q90`, `jpeg420-q95`, `-q90`,
  `-q85`, each encoder apart): median PSNR_ROI + 0.3 dB `[tunable]` over
  R*'s median, **and** p5 no worse;
* **F2** group `text`, per slice: the median PSNR no worse than R*'s by more
  than 0.2 dB, and the median SSIM not lower;
* **F3** `texture_ROI / texture_around` ∈ [0.8, 1.2] on ≥ 95 % of the
  denoised crops;
* **F4** lossless crops untouched (checked) — and on the bench's
  `linear-light` composites the median gain is not larger than on its
  `encoded` ones (otherwise the model hides the blend model's error);
* **F5** reported, not gated: `consistency_px` over `2h` on > 10 % of the
  crops is marked "the model leaves the data";
* **F6** the blind A/B (`ab.py`, question `pair`, R* against the best
  variant at 200 % in the ROI, 30 pairs): the candidate no worse in ≥ 70 %
  — read from `--ab`, or "pending";
* **F7** every denoised crop was restored (the skipped are counted).

How to run it
-------------
    python3 scripts/model-eval/fdncnn_run.py run --crops bench/out/<run>/crops/R8d \
        --model model_zoo/fdncnn_color.onnx --out bench/out/<run>/fdncnn \
        [--results bench/out/<run>/results.jsonl] [--ab ab/fdncnn/score.json] \
        [--variants k5-s1.0-rgb,…] [--no-images] [--threads N]
    python3 scripts/model-eval/fdncnn_run.py selftest

`--results` joins each crop to its bench line (config, background, case,
slice, encoder) and writes `model.jsonl` beside the report: one bench line
per crop for R* measured in the crop (config `<R*>@crop`) and one per
variant (`<R*>+fdncnn:<variant>`) — so `scripts/bench/report.py gates
model.jsonl --candidate R8d+fdncnn:k5-s1.0-rgb --baseline R8d@crop --route
lossy` reads a model as one more config.

What it needs
-------------
Python 3.10+, numpy, Pillow, `onnxruntime` (imported only by `run`), the
ONNX model and its JSON from `fdncnn_export.py`. Versions to pin:
`scripts/model-eval/README.md`. `selftest` needs none of the model's
packages: it runs a stub denoiser over synthetic crops.

What its output means
---------------------
`<out>/report.md` — the model's facts (weights' sha256, versions, device,
time per crop), every variant per slice × encoder and per group at the
median and p5 against R*, the `consistency_px` distribution, F1–F7 per
variant, and **one closing line**; `<out>/results.json` (every crop ×
variant); `<out>/images/<crop>/<variant>.png` (what `ab.py` shows; not with
`--no-images`); `<out>/model.jsonl` with `--results`. Exit 0 when the run
completed (the verdict is the closing line, not the exit code), 2 on a
refusal — a missing model, no crop, or a `ModelLeak`.
"""

import argparse
import hashlib
import json
import os
import re
import sys
import time

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import evalkit as ek  # noqa: E402

PAD = 24
SIGMA_MAX = 75.0
K_EDGE = (0, 5, 10)
STRENGTH = (0.6, 1.0, 1.5)
SPACES = ("rgb", "luma")
# ── [tunable] — R10 §2.4; each moves only with a line in a report ──
F1_MEDIAN_GAIN_DB = 0.3
F2_TEXT_LOSS_DB = 0.2
F3_RATIO = (0.8, 1.2)
F3_SHARE = 0.95
F5_SHARE = 0.10
F6_SHARE = 0.70
TARGET = re.compile(r"^jpeg(444|420)-q(85|90|95)$")


class ModelLeak(ek.Refusal):
    """The output moved where `α = 0`: the mask is wrong, and nothing after it can be trusted."""


def variants():
    return [(k, s, sp) for sp in SPACES for k in K_EDGE for s in STRENGTH]


def variant_name(k, s, sp):
    return f"k{k}-s{s}-{sp}"


# ───────────────────────────────────────────────────────── the method

def sigma_map(alpha, sigma_base, k_edge):
    """§2.3 step 2, in 8-bit levels."""
    a = np.clip(np.asarray(alpha, dtype=np.float64), 0.0, 1.0)
    base = sigma_base / np.maximum(1.0 - a, 1e-6)
    return np.clip(base + k_edge * ek.gradient_magnitude(a), sigma_base, SIGMA_MAX)


def mask_of(alpha, strength):
    """§2.3 step 4: `M = clamp(strength · smoothstep(0, 0.1, α), 0, 1)`."""
    return np.clip(strength * ek.smoothstep(0.0, 0.1, alpha), 0.0, 1.0)


def denoised(denoiser, recon, alpha, sigma_base, k_edge, space):
    """`den` in 8-bit levels (floats), the shape of `recon`: §2.3 steps 1–3 and 5."""
    sig = sigma_map(alpha, sigma_base, k_edge)
    if space == "rgb":
        x = recon.astype(np.float32) / 255.0
    else:
        y = ek.ycbcr(recon)[..., 0] / 255.0
        x = np.stack([y, y, y], axis=-1).astype(np.float32)
    xp = ek.mirror_pad(x, PAD)
    sp = ek.mirror_pad(sig, PAD).astype(np.float32)
    d = np.asarray(denoiser(xp, sp), dtype=np.float64)[PAD:-PAD, PAD:-PAD] * 255.0
    if space == "rgb":
        return d
    ycc = ek.ycbcr(recon)
    ycc[..., 0] = d.mean(axis=-1)
    return ek.rgb_of(ycc)


def compose(recon, den, alpha, strength):
    """§2.3 step 4: `out = M·den + (1 − M)·recon`, 8 bits; refuses if `out` moved where `α = 0`."""
    m = mask_of(alpha, strength)[..., None]
    rf = recon.astype(np.float64)
    out = np.where(m > 0.0, ek.to_u8(rf + m * (den - rf)), recon).astype(np.uint8)
    zero = np.asarray(alpha) == 0.0
    if not np.array_equal(out[zero], recon[zero]):
        moved = int(np.any(out != recon, axis=-1)[zero].sum())
        raise ModelLeak(f"{moved} pixel(s) with α = 0 moved: the output must be recon.png there, byte for byte")
    return out


class OnnxDenoiser:
    """FDnCNN by onnxruntime on the CPU: (H, W, 3) in [0, 1] and σ (H, W) in levels → (H, W, 3)."""

    def __init__(self, path, threads=None):
        import onnxruntime as ort

        opts = ort.SessionOptions()
        if threads:
            opts.intra_op_num_threads = threads
        self.session = ort.InferenceSession(path, sess_options=opts, providers=["CPUExecutionProvider"])
        self.version = ort.__version__
        self.calls, self.seconds = 0, 0.0

    def __call__(self, rgb, sigma):
        x = np.concatenate([np.transpose(rgb, (2, 0, 1)), (sigma / 255.0)[None]], axis=0)[None].astype(np.float32)
        t = time.monotonic()
        y = self.session.run(["y"], {"x": x})[0][0]
        self.seconds += time.monotonic() - t
        self.calls += 1
        return np.transpose(y, (1, 2, 0))


# ───────────────────────────────────────────────────────── per crop

def measure(crop, out, sigma):
    """What R10 §2.4 reads off one output (R* included)."""
    roi = crop.roi
    tex, around = ek.texture(out, crop.alpha, crop.rect_px, crop.opaque)
    cons, excluded = ek.consistency_px(out, crop.input, crop.alpha, crop.logo, crop.opaque)
    m = {"texture": tex, "texture_around": around, "ratio": tex / around if around > 0 else None,
         "consistency_px": cons, "consistency_excluded": excluded, "two_h": 2 * ek.h_of(sigma),
         "changed": int(np.any(out != crop.recon, axis=-1).sum())}
    if crop.gt is not None:
        m.update(psnr_roi=ek.psnr(out, crop.gt, roi), ssim_roi=ek.ssim(out, crop.gt, roi),
                 de2000_roi=ek.delta_e(out, crop.gt, roi))
    return m


def run_crop(crop, denoiser, chosen, save=None):
    """Every chosen variant over one crop: (records, seconds)."""
    t0 = time.monotonic()
    base = {"crop": crop.name, "dir": crop.dir, "slice": crop.slice, "encoder": crop.meta.get("encoder"),
            "group": crop.group, "config": crop.meta.get("config"), "class": crop.meta.get("class"),
            "variant_of_file": crop.meta.get("variant"),
            "blend": (crop.meta.get("blend") or {}).get("model"), "k": (crop.meta.get("blend") or {}).get("k"),
            "key": list(crop.key())}
    if not crop.restored:
        return [dict(base, variant=None, applied=False, why="no restoration (F7)")], time.monotonic() - t0
    sigma = crop.sigma_base()
    rstar = dict(base, variant="R*", applied=False, sigma_base=sigma, **measure(crop, crop.recon, sigma))
    if not crop.lossy:
        out = compose(crop.recon, crop.recon.astype(np.float64), crop.alpha, 0.0)
        if not np.array_equal(out, crop.recon):
            raise ModelLeak(f"{crop.dir}: a lossless crop moved")
        rstar["why"] = "lossless source: not denoised (F4)"
        return [rstar], time.monotonic() - t0
    recs = [rstar]
    dens = {}
    for k, s, sp in chosen:
        if (k, sp) not in dens:
            dens[(k, sp)] = denoised(denoiser, crop.recon, crop.alpha, sigma, k, sp)
        out = compose(crop.recon, dens[(k, sp)], crop.alpha, s)
        name = variant_name(k, s, sp)
        if save:
            ek.write_png(os.path.join(save, crop.name, name + ".png"), out)
        rec = dict(base, variant=name, applied=True, k_edge=k, strength=s, space=sp, sigma_base=sigma,
                   **measure(crop, out, sigma))
        rec["out_sha256"] = hashlib.sha256(out.tobytes()).hexdigest()
        recs.append(rec)
    return recs, time.monotonic() - t0


# ───────────────────────────────────────────────────────── the gates

def stat(values):
    v = [x for x in values if x is not None]
    return {"n": len(v), "median": ek.median(v), "p5": ek.p5(v)}


def gates(recs, names, ab=None):
    """F1–F7 per variant, and the best one."""
    rstar = {r["crop"]: r for r in recs if r.get("variant") == "R*"}
    out = {}
    for name in names:
        mine = [r for r in recs if r.get("variant") == name]
        g = {}
        # F1 — per target slice × encoder
        f1 = []
        for (sl, en) in sorted({(r["slice"], r["encoder"]) for r in mine if TARGET.match(r["slice"] or "")}, key=str):
            c = [r["psnr_roi"] for r in mine if (r["slice"], r["encoder"]) == (sl, en) and r.get("psnr_roi") is not None]
            b = [rstar[r["crop"]]["psnr_roi"] for r in mine if (r["slice"], r["encoder"]) == (sl, en)
                 and rstar.get(r["crop"], {}).get("psnr_roi") is not None]
            if not c or not b:
                continue
            dm, dp = ek.median(c) - ek.median(b), ek.p5(c) - ek.p5(b)
            f1.append({"slice": sl, "encoder": en, "n": len(c), "median_delta": dm, "p5_delta": dp,
                       "ok": dm >= F1_MEDIAN_GAIN_DB and dp >= 0.0})
        g["F1"] = {"ok": bool(f1) and all(x["ok"] for x in f1) if f1 else None, "slices": f1,
                   "median_gain": ek.median([x["median_delta"] for x in f1]) if f1 else None}
        # F2 — group text
        f2 = []
        text = [r for r in mine if r["group"] == "text" and r.get("psnr_roi") is not None]
        for sl in sorted({r["slice"] for r in text}, key=str):
            c = [r for r in text if r["slice"] == sl]
            b = [rstar[r["crop"]] for r in c if r["crop"] in rstar]
            dm = ek.median([r["psnr_roi"] for r in c]) - ek.median([r["psnr_roi"] for r in b])
            ds = (ek.median([r["ssim_roi"] for r in c if r.get("ssim_roi") is not None]) or 0.0) - \
                 (ek.median([r["ssim_roi"] for r in b if r.get("ssim_roi") is not None]) or 0.0)
            f2.append({"slice": sl, "n": len(c), "median_delta": dm, "ssim_delta": ds,
                       "ok": dm >= -F2_TEXT_LOSS_DB and ds >= -1e-9})
        g["F2"] = {"ok": all(x["ok"] for x in f2) if f2 else None, "slices": f2}
        # F3
        ratios = [r["ratio"] for r in mine if r.get("ratio") is not None]
        inside = sum(1 for x in ratios if F3_RATIO[0] <= x <= F3_RATIO[1])
        g["F3"] = {"ok": (inside / len(ratios) >= F3_SHARE) if ratios else None, "share": inside / len(ratios) if ratios else None,
                   "n": len(ratios)}
        # F4 — lossless untouched (every lossless crop has only R*, checked by `run_crop`); linear vs encoded
        gain = {}
        for model in ("encoded", "linear-light"):
            d = [r["psnr_roi"] - rstar[r["crop"]]["psnr_roi"] for r in mine if r.get("blend") == model
                 and r.get("psnr_roi") is not None and rstar.get(r["crop"], {}).get("psnr_roi") is not None]
            gain[model] = ek.median(d)
        lossless = sum(1 for r in recs if r.get("variant") == "R*" and str(r.get("why", "")).startswith("lossless"))
        lin_ok = None if None in gain.values() else gain["linear-light"] <= gain["encoded"]
        g["F4"] = {"ok": lin_ok if lin_ok is not None else True, "lossless_untouched": lossless,
                   "median_gain_encoded": gain["encoded"], "median_gain_linear": gain["linear-light"],
                   "linear_checked": lin_ok is not None}
        # F5 — reported
        over = sum(1 for r in mine if r["consistency_px"] > r["two_h"])
        share = over / len(mine) if mine else None
        g["F5"] = {"marked": share is not None and share > F5_SHARE, "share_over_2h": share,
                   "consistency": stat([r["consistency_px"] for r in mine]),
                   "consistency_p95": ek.quantile([r["consistency_px"] for r in mine], 0.95)}
        # F6 — the A/B, when scored for this variant
        if ab and ab.get("question") == "pair" and ab.get("candidate") == name:
            s = ab["pooled"]["no_worse_share"]
            g["F6"] = {"ok": s >= F6_SHARE, "no_worse_share": s, "pairs": ab["pooled"]["items"]}
        else:
            g["F6"] = {"ok": None, "why": "pending: the blind A/B (Q-R7) for this variant"}
        # F7
        skipped = sum(1 for r in recs if r.get("variant") is None)
        g["F7"] = {"ok": all(r["dir"] for r in mine), "skipped_unrestored": skipped, "denoised": len(mine)}
        out[name] = g
    return out


def closing_line(g):
    """§2.5 and §4: one line."""
    if not g:
        return "FDnCNN not run: no lossy restored crop"
    passing = [n for n, x in g.items() if x["F1"]["ok"] and x["F2"]["ok"] is not False and x["F3"]["ok"]
               and x["F4"]["ok"] and x["F7"]["ok"] and not x["F5"]["marked"]]
    decided = [n for n, x in g.items() if x["F1"]["ok"] is not None]
    if not decided:
        return "FDnCNN undecided: no crop with a truth on a target slice (F1 needs the bench's crops)"
    if not passing:
        return "FDnCNN gives no precision on the remainder after R* — closed for Gemini; Grok repeats it after R11/R12"
    best = max(passing, key=lambda n: g[n]["F1"]["median_gain"] or 0.0)
    slices = ", ".join(sorted({f"{x['slice']}" for x in g[best]["F1"]["slices"]}))
    gain = g[best]["F1"]["median_gain"]
    if g[best]["F6"]["ok"] is None:
        return f"FDnCNN gives +{gain:.2f} dB on {slices}; best variant: {best}; F6 (the blind A/B) pending — no verdict before it"
    if not g[best]["F6"]["ok"]:
        return f"FDnCNN gives +{gain:.2f} dB on {slices} but loses the blind A/B (F6); best variant: {best} — not needed"
    return f"FDnCNN gives +{gain:.2f} dB on {slices}; best variant: {best}; integration: Q-R2"


# ───────────────────────────────────────────────────────── report

def tables(recs, names):
    """Every variant per slice × encoder and per group: median and p5 of PSNR, SSIM, ΔE, against R*."""
    md = []
    rows = []
    keys = sorted({(r["slice"], r["encoder"]) for r in recs if r.get("variant")}, key=str)
    for name in ["R*"] + list(names):
        for sl, en in keys:
            rs = [r for r in recs if r.get("variant") == name and (r["slice"], r["encoder"]) == (sl, en)]
            if not rs:
                continue
            p, s, d = (stat([r.get(k) for r in rs]) for k in ("psnr_roi", "ssim_roi", "de2000_roi"))
            c = stat([r["consistency_px"] for r in rs])
            t = stat([r.get("ratio") for r in rs])
            rows.append([name, sl, en, len(rs), p["median"], p["p5"], s["median"], s["p5"], d["median"],
                         t["median"], t["p5"], c["median"], ek.quantile([r["consistency_px"] for r in rs], 0.95)])
    md.append(ek.md_table(["variant", "slice", "encoder", "n", "PSNR med", "PSNR p5", "SSIM med", "SSIM p5", "ΔE med",
                           "tex ratio med", "tex ratio p5", "consistency med", "consistency p95"], rows))
    rows = []
    groups = sorted({r["group"] for r in recs if r.get("variant")}, key=str)
    for name in ["R*"] + list(names):
        for gr in groups:
            rs = [r for r in recs if r.get("variant") == name and r["group"] == gr]
            if rs:
                p = stat([r.get("psnr_roi") for r in rs])
                s = stat([r.get("ssim_roi") for r in rs])
                rows.append([name, gr, len(rs), p["median"], p["p5"], s["median"], s["p5"]])
    md += ["", ek.md_table(["variant", "group", "n", "PSNR med", "PSNR p5", "SSIM med", "SSIM p5"], rows)]
    return "\n".join(md)


def report(model, env, recs, g, line, times, crops_dir, agreement):
    names = list(g)
    md = [f"# E12-R10 — FDnCNN after R*, {time.strftime('%Y-%m-%d')}", "",
          "Written by `scripts/model-eval/fdncnn_run.py run` (R10 §2.3–§2.5, §4).", "",
          "## The run", "",
          f"* crops: `{crops_dir}` — {len({r['crop'] for r in recs})} crops, "
          f"{sum(1 for r in recs if r.get('variant') is None)} skipped (no restoration, F7), "
          f"{sum(1 for r in recs if str(r.get('why', '')).startswith('lossless'))} lossless (not denoised, F4)",
          f"* weights: `{model.get('weights')}` sha256 `{model.get('weights_sha256')}`; ONNX sha256 `{model.get('onnx_sha256')}`; "
          f"KAIR commit {model.get('kair_commit') or '–'}",
          f"* frameworks: onnxruntime {env.get('onnxruntime')}, numpy {env.get('numpy')}, Pillow {env.get('pillow')}, "
          f"Python {env.get('python')}; device {env.get('device')} ({env.get('cpu')}, {env.get('platform')})",
          f"* time per crop (all variants): median {ek.fmt(ek.median(times))} s, max {ek.fmt(max(times) if times else None)} s",
          f"* the metrics restated in Python against the bench's own (R* in the crop's ROI): largest |ΔPSNR| "
          f"{ek.fmt(agreement)} dB" if agreement is not None else "* the bench's lines were not given: no check of the metrics against them",
          "", "## Every variant against R*", "", tables(recs, names), "",
          "## F1–F7", ""]
    rows = []
    for n, x in g.items():
        rows.append([n, x["F1"]["ok"], ek.fmt(x["F1"]["median_gain"]), x["F2"]["ok"], x["F3"]["ok"],
                     ek.fmt(x["F3"]["share"]), x["F4"]["ok"], "marked" if x["F5"]["marked"] else "–",
                     ek.fmt(x["F5"]["share_over_2h"]), x["F6"]["ok"], x["F7"]["ok"]])
    md += [ek.md_table(["variant", "F1", "median gain dB", "F2", "F3", "F3 share", "F4", "F5", "share > 2h", "F6", "F7"], rows), "",
           "F1/F2/F6 '–' is not decided (no truth, no text crop, no A/B yet). F5 'marked' is \"the model leaves the data\": "
           "an argument against, not a gate.", "", "## Closing line", "", line, ""]
    return "\n".join(md)


def bench_rows(results_path, recs):
    """`model.jsonl`: the bench's line per crop for R* (`<R*>@crop`) and per variant (`<R*>+fdncnn:<v>`),
    with the metrics measured in the crop. Returns (lines, the largest |ΔPSNR| against the bench's R* line)."""
    by_key = {}
    with open(results_path) as f:
        for line in f:
            if line.strip():
                r = json.loads(line)
                by_key[(r.get("config"), r.get("background"), r.get("case"), r.get("slice"), r.get("encoder"))] = r
    lines, worst = [], 0.0
    for r in recs:
        if not r.get("variant"):
            continue
        b = by_key.get(tuple(r["key"]))
        if b is None or r.get("psnr_roi") is None:
            continue
        cfg = b["config"]
        if r["variant"] == "R*":
            worst = max(worst, abs(r["psnr_roi"] - b["psnr_roi"]))
            name = f"{cfg}@crop"
        else:
            name = f"{cfg}+fdncnn:{r['variant']}"
        row = dict(b, config=name, psnr_roi=r["psnr_roi"], ssim_roi=r.get("ssim_roi"), de2000_roi=r.get("de2000_roi"),
                   restored_sha256=r.get("out_sha256") or b.get("restored_sha256"),
                   model_eval={"model": "fdncnn", "variant": r["variant"], "consistency_px": r["consistency_px"],
                               "ratio": r.get("ratio")})
        lines.append(json.dumps(row))
    return lines, (worst if lines else None)


def cmd_run(args):
    dirs = ek.load_crops(args.crops)
    if not dirs:
        raise ek.Refusal(f"{args.crops}: no crop folder (meta.json, recon.png, input.png, alpha.pgm)")
    if not os.path.exists(args.model):
        raise ek.Refusal(f"{args.model}: no model (fdncnn_export.py export)")
    model = {}
    if os.path.exists(args.model + ".json"):
        with open(args.model + ".json") as f:
            model = json.load(f)
    den = OnnxDenoiser(args.model, args.threads)
    chosen = variants()
    if args.variants:
        want = set(args.variants.split(","))
        chosen = [v for v in chosen if variant_name(*v) in want]
        if not chosen:
            raise ek.Refusal(f"--variants names none of {', '.join(variant_name(*v) for v in variants())}")
    os.makedirs(args.out, exist_ok=True)
    save = None if args.no_images else os.path.join(args.out, "images")
    recs, times = [], []
    for n, d in enumerate(dirs, 1):
        crop = ek.Crop(d)
        rs, t = run_crop(crop, den, chosen, save)
        recs += rs
        times.append(t)
        print(f"  [{n}/{len(dirs)}] {crop.name}: {len(rs) - 1} variant(s), {t:.2f} s", flush=True)
    ab = None
    if args.ab:
        with open(args.ab) as f:
            ab = json.load(f)
    g = gates(recs, [variant_name(*v) for v in chosen], ab)
    line = closing_line(g)
    agreement = None
    if args.results:
        lines, agreement = bench_rows(args.results, recs)
        with open(os.path.join(args.out, "model.jsonl"), "w") as f:
            f.write("\n".join(lines) + ("\n" if lines else ""))
    env = ek.environment("cpu (onnxruntime CPUExecutionProvider)")
    env["onnxruntime"] = den.version
    ek.write_json(os.path.join(args.out, "results.json"),
                  {"model": model, "environment": env, "records": recs, "gates": g, "closing_line": line,
                   "time_per_crop_s": times, "denoiser_calls": den.calls, "denoiser_seconds": den.seconds})
    text = report(model, env, recs, g, line, times, args.crops, agreement)
    with open(os.path.join(args.out, "report.md"), "w") as f:
        f.write(text)
    print(line)
    print(f"report: {os.path.join(args.out, 'report.md')}")
    return 0


# ───────────────────────────────────────────────────────── selftest

def stub(rgb, sigma):
    """A stub denoiser that changes every pixel: its negative. Not a model; a probe of the mask."""
    return 1.0 - rgb


def _t_where_alpha_is_zero_fdncnn_changes_nothing():
    c = ek.synthetic_crop(seed=4)
    zero = c.alpha == 0.0
    assert zero.any() and (~zero).any()
    for k, s, sp in variants():
        den = denoised(stub, c.recon, c.alpha, 2.0, k, sp)
        out = compose(c.recon, den, c.alpha, s)
        assert np.array_equal(out[zero], c.recon[zero]), variant_name(k, s, sp)
        assert np.any(out[c.alpha > 0.05] != c.recon[c.alpha > 0.05]), "the stub changed nothing under the mark"


def _t_a_leak_where_alpha_is_zero_is_refused():
    c = ek.synthetic_crop(seed=5)
    den = denoised(stub, c.recon, c.alpha, 2.0, 0, "rgb")
    m = np.full(c.alpha.shape, 1.0)
    out = np.where(m[..., None] > 0, ek.to_u8(den), c.recon).astype(np.uint8)
    zero = c.alpha == 0.0
    assert not np.array_equal(out[zero], c.recon[zero])
    try:
        # `compose` itself is what the run calls; a leak in a later edit must be a refusal there.
        mask = mask_of

        def leaky(alpha, strength):
            return np.full(np.shape(alpha), strength)

        globals()["mask_of"] = leaky
        compose(c.recon, den, c.alpha, 1.0)
    except ModelLeak:
        pass
    else:
        raise AssertionError("a mask that leaked past α = 0 was not refused")
    finally:
        globals()["mask_of"] = mask


def _t_the_sigma_map_is_clamped_and_grows_with_alpha():
    # Three equal rows: `np.gradient` needs two samples along each axis.
    a = np.tile([0.0, 0.5, 0.9, 0.99, 1.0], (3, 1))
    s = sigma_map(a, 2.0, 0)
    assert s[1, 0] == 2.0 and abs(s[1, 1] - 4.0) < 1e-9 and abs(s[1, 2] - 20.0) < 1e-6
    assert s[1, 3] == SIGMA_MAX and s[1, 4] == SIGMA_MAX
    assert np.all(sigma_map(a, 2.0, 10) >= s)


def _t_eighteen_variants_and_a_lossless_crop_is_not_denoised():
    assert len(variants()) == 18 and len({variant_name(*v) for v in variants()}) == 18
    c = ek.synthetic_crop(seed=6, lossy=False)
    recs, _ = run_crop(c, stub, variants())
    assert [r["variant"] for r in recs] == ["R*"] and "lossless" in recs[0]["why"], recs
    c.meta["restored"] = False
    recs, _ = run_crop(c, stub, variants())
    assert recs[0]["variant"] is None


def _t_the_gates_read_p5_and_the_closing_line_says_one_thing():
    def rec(crop, variant, psnr, slice_="jpeg444-q95", group="flat", ratio=1.0, cons=0.3):
        return {"crop": crop, "variant": variant, "psnr_roi": psnr, "ssim_roi": 0.9, "de2000_roi": 1.0, "slice": slice_,
                "encoder": "pillow", "group": group, "blend": "encoded", "ratio": ratio, "consistency_px": cons,
                "two_h": 4.0, "dir": "d"}
    recs = []
    for i in range(20):
        recs.append(rec(f"c{i}", "R*", 40.0))
        # The median rises by 1 dB; the two worst crops fall by 10: p5 worse — F1 fails.
        recs.append(rec(f"c{i}", "v", 41.0 if i >= 2 else 30.0))
    g = gates(recs, ["v"])
    assert g["v"]["F1"]["ok"] is False, g["v"]["F1"]
    assert closing_line(g).startswith("FDnCNN gives no precision"), closing_line(g)
    recs = [r if r["variant"] == "R*" else dict(r, psnr_roi=41.0) for r in recs]
    g = gates(recs, ["v"])
    assert g["v"]["F1"]["ok"] and "F6" in closing_line(g) and "pending" in closing_line(g), closing_line(g)
    ab = {"question": "pair", "candidate": "v", "pooled": {"no_worse_share": 0.8, "items": 30}}
    assert closing_line(gates(recs, ["v"], ab)).endswith("integration: Q-R2")


def selftest():
    return ek.run_cases([
        ("where_alpha_is_zero_fdncnn_changes_nothing", _t_where_alpha_is_zero_fdncnn_changes_nothing),
        ("a_leak_where_alpha_is_zero_is_refused", _t_a_leak_where_alpha_is_zero_is_refused),
        ("the_sigma_map_is_clamped_and_grows_with_alpha", _t_the_sigma_map_is_clamped_and_grows_with_alpha),
        ("eighteen_variants_and_a_lossless_crop_is_not_denoised", _t_eighteen_variants_and_a_lossless_crop_is_not_denoised),
        ("the_gates_read_p5_and_the_closing_line_says_one_thing", _t_the_gates_read_p5_and_the_closing_line_says_one_thing),
    ])


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = p.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("run")
    s.add_argument("--crops", required=True)
    s.add_argument("--model", required=True)
    s.add_argument("--out", required=True)
    s.add_argument("--results", help="the bench's results.jsonl, to write model.jsonl for scripts/bench/report.py")
    s.add_argument("--ab", help="ab.py score's JSON for the `pair` question (F6)")
    s.add_argument("--variants", help="comma-separated names (default: all 18)")
    s.add_argument("--no-images", action="store_true")
    s.add_argument("--threads", type=int)
    sub.add_parser("selftest")
    args = p.parse_args(argv)
    try:
        return selftest() if args.cmd == "selftest" else cmd_run(args)
    except ek.Refusal as e:
        print(f"fdncnn_run.py: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
