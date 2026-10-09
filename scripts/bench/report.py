#!/usr/bin/env python3
"""The restoration bench's report, and the gates of level A.

What it is for
--------------
Step E12-R5 of the E12-R series (`docs/plan/E12-R5-recon-bench.md` §4.5–§4.6),
from the owner's spec `wipemark-recon-spec-2026-10-08` (`05-recon-bench.md`),
dispatched by the coordinator on 2026-10-09. `recon_bench run` writes one JSON
line per file and config: the metrics against the ground truth in the mark's
ROI, the restoration's own measures, the detection. This reads those lines
and says, per slice, group and encoder, how close the restoration came to the
truth — the median **and** the worst 5 %, because a better mean with a worse
tail is a fail — and, given a candidate config, whether it passes the gates
A1–A7 that R6, R8, R9 and R10 are accepted by.

What it does
------------
`report` writes Markdown:

  0. the run: its commit, its time, the encoders' versions, the counts;
  1. configs × slices: n, the median and p5 of PSNR and SSIM, the median and
     p95 of ΔE2000 (for ΔE the tail is the top), the worst-5 % mean of PSNR,
     the delta to R0, and the p95 over restored files of `consistency_px`
     (D305, added by E12-R7 on 2026-10-09: the restoration blended back
     against its input; about 0 for an exact inverse, a slice over 1 level is
     named under the table) — per encoder, then per group, for marks blended
     as every shipped profile declares (`encoded`, `k = 1`);
  2. the blend-model matrix (A5): per inverse, the median PSNR on composites of
     each model;
  3. the measures against the truth: Pearson and Spearman of texture ↔ PSNR,
     chroma ↔ ΔE and |step| ↔ PSNR, per slice, over restored files;
  4. tails: the ten worst files of each target slice;
  5. whether the bench reproduces the real failures (§6.4): chroma > 4 on
     `jpeg420-q95`, texture > 5.5 on `jpeg444-q95`, out of range on
     `jpeg420-q90` — all Pillow;
  6. the self-test: `exact` on `png` with the canonical map;
  7. detection, and the `R-k` composites (D154);
  8. the two encoders apart: a line for every slice where they differ by
     more than 0.3 dB;
  9. what was not done;
 10. the value inside the interval (E12-R8, added by its agent on 2026-10-09
     for the coordinator; `docs/plan/E12-R8-value-inside-the-interval.md`
     §6.2): per config, slice and encoder over the restored lossy files,
     `texture` and its ratio to `texture_around`, how many are `smoothed`
     (D307), refined, refined as text, in how many rounds, the largest
     `consistency_dct`;
     then §6.2's two checks per config — `texture` under 5.5 on
     `jpeg444-q95` on at least 80 % of the restored files, and the soap
     check, `texture / texture_around ≥ 0.8` on at least 95 % of every
     restored lossy file — each said pass or fail. A1–A7 stay `gates`'.

`gates` evaluates A1–A7 for `--candidate` against `--baseline` (R0) on the
`--route` (`lossy` or `model`) with `--targets` (slices, globs allowed) and
writes them as JSON and a table. `selftest` runs fake results through every
gate both ways.

How to run it
-------------
    python3 scripts/bench/report.py report RESULTS.jsonl [--out REPORT.md] [--targets jpeg420-*,…] [--profile GLOB]
    python3 scripts/bench/report.py gates  RESULTS.jsonl --candidate R6 [--baseline R0] [--profile GLOB] \
                                    --route {lossy,model} --targets jpeg420-q95,jpeg420-q90 [--out gates.json]
    python3 scripts/bench/report.py selftest

`RESULTS.jsonl` may hold several configs (one `run` with several `--config`,
or files concatenated); `RESULTS.jsonl.run.json`, written beside it by
`run`, and the run's `encode.json` are read when they are there.

`--profile GLOB[,GLOB]` (report, gates; added by E12-R12's stage 4b,
2026-10-09) keeps the lines of the matching profiles only — results of
several profiles concatenated (a Gemini run and `R0-grok`) are reported
and gated apart; none matching is exit 2. §0 names the profiles a report
is over. Nothing in the tables is a profile's: the rows, the matrix (A5)
and the gates read the lines as they come, whatever mark made them.

What it needs
-------------
Python 3.10+, the standard library only. Nothing in the repository depends
on it, and no Rust gate runs it.

What its output means
---------------------
`report` exits 0. `gates` exits 0 when every gate passed, 1 when one failed,
2 on a usage error or when the candidate or the baseline has no results.
`selftest` exits 0 when every case held. Every `[tunable]` below moves only
with a line in a report.
"""

import fnmatch
import json
import math
import os
import statistics
import sys
import tempfile

# ── [tunable] — the gates of level A (§4.6) ──
A1_MEDIAN_GAIN_DB = 0.5   # target slices: the median PSNR rises at least this much…
A1_P5_TOLERANCE_DB = 0.0  # …and p5 falls by no more than this (no worse than R0)
A2_MEDIAN_LOSS_DB = 0.1   # non-target slices: the median may fall at most this…
A2_P5_LOSS_DB = 0.2       # …and p5 at most this
A3_TEXT_LOSS_DB = 0.3     # group `text`: no slice loses more than this, median or p5
A6_MODEL_EXACT = 0.99     # route `model`: the exact share on png canonical, at least
ENCODER_GAP_DB = 0.3      # two encoders on one slice further apart than this: a line
# E12-R7 §6.3: an exact inverse is consistent to rounding — a slice whose p95 is over this is named.
CONSISTENCY_LEVELS = 1.0
# The real failures the synthetic set must reproduce by their order of magnitude (§6.4).
CHROMA_BOUND = 4.0
TEXTURE_BOUND = 5.5

DEFAULT_TARGETS = ["jpeg420-q95", "jpeg420-q90", "jpeg444-q95"]
# E12-R8 §6.2 — the value inside the codec's interval (added 2026-10-09):
R8_TEXTURE_SLICE = "jpeg444-q95"
R8_TEXTURE_SHARE = 0.8    # on that slice, texture under TEXTURE_BOUND on at least this share of restored files
SOAP_RATIO = 0.8          # D307: texture / texture_around at least this (TEXTURE_RATIO_MIN)…
SOAP_SHARE = 0.95         # …on at least this share of the restored lossy files (the soap check)
LOSSY_SLICES = ["jpeg*", "webp-lossy*"]


# ───────────────────────────────────────────────────────── numbers

def quantile(values, q):
    """The q-quantile by linear interpolation (numpy's default)."""
    v = sorted(values)
    if not v:
        return None
    pos = (len(v) - 1) * q
    lo, hi = math.floor(pos), math.ceil(pos)
    return v[lo] + (v[hi] - v[lo]) * (pos - lo)


def median(values):
    return quantile(values, 0.5)


def worst_mean(values, share=0.05, low=True):
    """The mean of the worst `share` of values (at least one)."""
    v = sorted(values, reverse=not low)
    if not v:
        return None
    k = max(1, math.ceil(len(v) * share))
    return sum(v[:k]) / k


def pearson(xs, ys):
    n = len(xs)
    if n < 3:
        return None
    mx, my = sum(xs) / n, sum(ys) / n
    sxx = sum((x - mx) ** 2 for x in xs)
    syy = sum((y - my) ** 2 for y in ys)
    if sxx == 0 or syy == 0:
        return None
    return sum((x - mx) * (y - my) for x, y in zip(xs, ys)) / math.sqrt(sxx * syy)


def ranks(xs):
    order = sorted(range(len(xs)), key=lambda i: xs[i])
    r = [0.0] * len(xs)
    i = 0
    while i < len(order):
        j = i
        while j + 1 < len(order) and xs[order[j + 1]] == xs[order[i]]:
            j += 1
        for k in range(i, j + 1):
            r[order[k]] = (i + j) / 2.0
        i = j + 1
    return r


def spearman(xs, ys):
    return pearson(ranks(xs), ranks(ys))


def fmt(v, digits=2):
    if v is None:
        return "–"
    if isinstance(v, float):
        return f"{v:.{digits}f}"
    return str(v)


# ───────────────────────────────────────────────────────── reading

def load(path):
    rows = []
    with open(path) as f:
        for line in f:
            if line.strip():
                rows.append(json.loads(line))
    return rows


def sidecars(path):
    out = {}
    for name, p in [("run", path + ".run.json"), ("encode", os.path.join(os.path.dirname(path), "encode.json"))]:
        if os.path.exists(p):
            with open(p) as f:
                out[name] = json.load(f)
    return out


def usable(r):
    return "error" not in r and r.get("psnr_roi") is not None


def vendor_blend(r):
    """A mark blended as every shipped profile declares: in code values, k = 1."""
    return r.get("model") == "encoded" and float(r.get("k", 1)) == 1.0


def matches(slice_id, patterns):
    return any(fnmatch.fnmatch(slice_id, p) for p in patterns)


def of_profiles(rows, patterns):
    """The lines whose `profile` matches one of `patterns` (globs); every line when there are none."""
    if not patterns:
        return list(rows)
    return [r for r in rows if any(fnmatch.fnmatchcase(str(r.get("profile")), p) for p in patterns)]


def group_by(rows, *keys):
    out = {}
    for r in rows:
        out.setdefault(tuple(r.get(k) for k in keys), []).append(r)
    return out


def consistencies(rows):
    """`consistency_px` of every restored row that carries one (D305; a run before E12-R7 has none)."""
    return [r["measures"]["consistency_px"] for r in rows
            if r.get("restored") and isinstance((r.get("measures") or {}).get("consistency_px"), (int, float))]


def summary(rows):
    p = [r["psnr_roi"] for r in rows]
    s = [r["ssim_roi"] for r in rows if r.get("ssim_roi") is not None]
    d = [r["de2000_roi"] for r in rows]
    c = consistencies(rows)
    return {
        "n": len(rows),
        "psnr_mean": sum(p) / len(p) if p else None,
        "psnr_median": median(p),
        "psnr_p5": quantile(p, 0.05),
        "psnr_worst5": worst_mean(p),
        "ssim_median": median(s),
        "ssim_p5": quantile(s, 0.05),
        "de_median": median(d),
        "de_p95": quantile(d, 0.95),
        "restored": sum(1 for r in rows if r.get("restored")),
        "consistency_p95": quantile(c, 0.95),
    }


# ───────────────────────────────────────────────────────── gates

def gate_a1(base, cand, targets):
    """Target slices: median +A1_MEDIAN_GAIN_DB at least, and p5 no worse."""
    out = []
    for sl in sorted({r["slice"] for r in base if matches(r["slice"], targets)}):
        b = [r["psnr_roi"] for r in base if r["slice"] == sl]
        c = [r["psnr_roi"] for r in cand if r["slice"] == sl]
        if not b or not c:
            out.append({"slice": sl, "ok": False, "why": "no results"})
            continue
        dm, dp = median(c) - median(b), quantile(c, 0.05) - quantile(b, 0.05)
        ok = dm >= A1_MEDIAN_GAIN_DB and dp >= -A1_P5_TOLERANCE_DB
        out.append({"slice": sl, "ok": ok, "median_delta": dm, "p5_delta": dp,
                    "mean_delta": sum(c) / len(c) - sum(b) / len(b)})
    return {"gate": "A1", "ok": bool(out) and all(o["ok"] for o in out), "slices": out}


def gate_a2(base, cand, targets):
    out = []
    for sl in sorted({r["slice"] for r in base if not matches(r["slice"], targets)}):
        b = [r["psnr_roi"] for r in base if r["slice"] == sl]
        c = [r["psnr_roi"] for r in cand if r["slice"] == sl]
        if not c:
            out.append({"slice": sl, "ok": False, "why": "no results"})
            continue
        dm, dp = median(c) - median(b), quantile(c, 0.05) - quantile(b, 0.05)
        out.append({"slice": sl, "ok": dm >= -A2_MEDIAN_LOSS_DB and dp >= -A2_P5_LOSS_DB,
                    "median_delta": dm, "p5_delta": dp})
    return {"gate": "A2", "ok": all(o["ok"] for o in out), "slices": out}


def gate_a3(base, cand):
    out = []
    bt = [r for r in base if r.get("group") == "text"]
    ct = [r for r in cand if r.get("group") == "text"]
    for sl in sorted({r["slice"] for r in bt}):
        b = [r["psnr_roi"] for r in bt if r["slice"] == sl]
        c = [r["psnr_roi"] for r in ct if r["slice"] == sl]
        if not c:
            out.append({"slice": sl, "ok": False, "why": "no results"})
            continue
        dm, dp = median(c) - median(b), quantile(c, 0.05) - quantile(b, 0.05)
        out.append({"slice": sl, "ok": dm >= -A3_TEXT_LOSS_DB and dp >= -A3_TEXT_LOSS_DB,
                    "median_delta": dm, "p5_delta": dp})
    return {"gate": "A3", "ok": all(o["ok"] for o in out), "slices": out}


def key(r):
    return (r["case_dir"], r["slice"], r["encoder"])


def gate_a4(base, cand, route):
    """Route `lossy`: on png the restored raster is byte-equal to R0's, file by file."""
    if route != "lossy":
        return {"gate": "A4", "ok": True, "why": "not the lossy route"}
    b = {key(r): r for r in base if r["slice"] == "png"}
    c = {key(r): r for r in cand if r["slice"] == "png"}
    moved = [k for k in b if k not in c or c[k].get("restored_sha256") != b[k].get("restored_sha256")]
    return {"gate": "A4", "ok": not moved and bool(b), "files": len(b), "moved": [list(k) for k in moved[:20]]}


def inverse_of(rows):
    return rows[0].get("inverse") if rows else None


def gate_a5(base_all, cand_all, slices=("png",)):
    """Each inverse wins on composites of its own model and loses on the other's;
    a config that wins on both does not restore, it smooths."""
    bi, ci = inverse_of(base_all), inverse_of(cand_all)
    if bi == ci:
        return {"gate": "A5", "ok": True, "why": f"both inverses are {bi}: the matrix needs one of each"}
    out = []
    for sl in slices:
        cell = {}
        for name, rows in (("base", base_all), ("cand", cand_all)):
            for model in ("encoded", "linear-light"):
                v = [r["psnr_roi"] for r in rows if r["slice"] == sl and r.get("model") == model
                     and float(r.get("k", 1)) == 1.0]
                cell[(name, model)] = median(v)
        if None in cell.values():
            out.append({"slice": sl, "ok": False, "why": "a cell is empty"})
            continue
        own = {"base": bi, "cand": ci}
        ok = True
        for model in ("encoded", "linear-light"):
            winner = "base" if cell[("base", model)] > cell[("cand", model)] else "cand"
            ok &= own[winner] == model
        out.append({"slice": sl, "ok": ok, "cells": {f"{k[0]}/{k[1]}": v for k, v in cell.items()}})
    return {"gate": "A5", "ok": all(o["ok"] for o in out), "slices": out}


def canonical_png(rows):
    return [r for r in rows if r["slice"] == "png" and r.get("variant") == "canonical" and vendor_blend(r)]


def exact_share(rows):
    """Of the canonical png files restored with nothing clamped, the share exact."""
    pool = [r for r in canonical_png(rows) if r.get("restored") and (r.get("measures") or {}).get("clamped") == 0]
    if not pool:
        return None, 0
    return sum(1 for r in pool if r["measures"].get("exact")) / len(pool), len(pool)


def gate_a6(base, cand, route):
    share, n = exact_share(cand)
    if route == "lossy":
        b = {key(r): (r.get("measures") or {}).get("exact") for r in canonical_png(base)}
        c = {key(r): (r.get("measures") or {}).get("exact") for r in canonical_png(cand)}
        ok = share == 1.0 and b == c
    else:
        ok = share is not None and share >= A6_MODEL_EXACT
    return {"gate": "A6", "ok": ok, "exact_share": share, "n": n}


def gate_a7(base, cand):
    out = []
    for sl in sorted({r["slice"] for r in base}):
        b = [r for r in base if r["slice"] == sl]
        c = [r for r in cand if r["slice"] == sl]
        fb = sum(1 for r in b if r["detection"].get("found")) / len(b)
        fc = sum(1 for r in c if r["detection"].get("found")) / len(c) if c else 0.0
        eb = median([r["detection"]["rect_error"] for r in b if r["detection"].get("found")]) or 0.0
        ec = median([r["detection"]["rect_error"] for r in c if r["detection"].get("found")]) or 0.0
        out.append({"slice": sl, "ok": fc >= fb and ec <= eb + 1e-9, "found": [fb, fc], "rect_error": [eb, ec]})
    return {"gate": "A7", "ok": all(o["ok"] for o in out), "slices": out}


def gates(rows, candidate, baseline, route, targets):
    allb = [r for r in rows if r.get("config") == baseline and usable(r)]
    allc = [r for r in rows if r.get("config") == candidate and usable(r)]
    if not allb or not allc:
        return None
    b = [r for r in allb if vendor_blend(r)]
    c = [r for r in allc if vendor_blend(r)]
    out = [gate_a1(b, c, targets), gate_a2(b, c, targets), gate_a3(b, c), gate_a4(allb, allc, route),
           gate_a5(allb, allc), gate_a6(allb, allc, route), gate_a7(b, c)]
    return {"candidate": candidate, "baseline": baseline, "route": route, "targets": targets,
            "ok": all(g["ok"] for g in out), "gates": out}


# ───────────────────────────────────────────────────────── the report

def table(header, rows):
    out = ["| " + " | ".join(header) + " |", "|" + "|".join("---" for _ in header) + "|"]
    out += ["| " + " | ".join(fmt(c) for c in row) + " |" for row in rows]
    return "\n".join(out)


def report(rows, side, targets):
    ok = [r for r in rows if usable(r)]
    errors = [r for r in rows if not usable(r)]
    configs = sorted({r["config"] for r in ok}, key=lambda c: (c != "R0", c))
    vend = [r for r in ok if vendor_blend(r)]
    md = ["# The restoration bench", ""]
    run = side.get("run", {})
    enc = side.get("encode", {})
    md += [
        "## 0. The run",
        "",
        f"* results: {len(rows)} ({len(errors)} with an error); configs: {', '.join(configs) or '–'}",
        f"* commit: {run.get('commit', '–')}; {run.get('seconds', '–')} s with {run.get('jobs', '–')} jobs over {run.get('cases', '–')} cases",
        f"* Pillow: {(enc.get('versions') or {}).get('pillow', '–')}, libjpeg {(enc.get('versions') or {}).get('libjpeg', '–')}, libwebp {(enc.get('versions') or {}).get('libwebp', '–')}",
        f"* groups: {', '.join(sorted({r['group'] for r in ok}))}; backgrounds: {len({r['background'] for r in ok})}",
        f"* profiles: {', '.join(sorted({str(r.get('profile')) for r in ok})) or '–'}"
        + (f"; catalogue file {run['catalogue'].get('file')} (sha256 {run['catalogue'].get('sha256')})"
           if isinstance(run.get("catalogue"), dict) else ""),
        "",
    ]
    for e in errors[:10]:
        md.append(f"* error: `{e.get('case_dir')}` {e.get('slice')}/{e.get('encoder')}: {e.get('error')}")
    base = {k: summary(v) for k, v in group_by([r for r in vend if r["config"] == "R0"], "slice", "encoder").items()}

    md += ["## 1. Configs × slices (marks blended in code values, k = 1)", "",
           "PSNR and SSIM: the median and p5 (the low tail); ΔE2000: the median and p95 (the high tail). "
           "`worst 5 %` is the mean PSNR of the worst twentieth. Δ is against R0 on the same slice and encoder. "
           "`consist. p95` is the p95 over restored files of `consistency_px` (D305): the restoration blended back "
           "against its input, in 8-bit levels — about 0 for an exact inverse; – when the run has none.", ""]
    rows1 = []
    for (cfg, sl, en), rs in sorted(group_by(vend, "config", "slice", "encoder").items(), key=lambda kv: (configs.index(kv[0][0]), kv[0][1], kv[0][2])):
        s = summary(rs)
        b = base.get((sl, en))
        rows1.append([cfg, sl, en, s["n"], s["restored"], s["psnr_median"], s["psnr_p5"], s["psnr_worst5"],
                      s["ssim_median"], s["ssim_p5"], s["de_median"], s["de_p95"],
                      None if not b or cfg == "R0" else s["psnr_median"] - b["psnr_median"],
                      None if not b or cfg == "R0" else s["psnr_p5"] - b["psnr_p5"],
                      s["consistency_p95"]])
    md += [table(["config", "slice", "encoder", "n", "restored", "PSNR med", "PSNR p5", "worst 5 %", "SSIM med",
                  "SSIM p5", "ΔE med", "ΔE p95", "Δ med", "Δ p5", "consist. p95"], rows1), ""]
    over = [f"{r[0]} {r[1]} {r[2]} ({fmt(r[-1])})" for r in rows1 if r[-1] is not None and r[-1] > CONSISTENCY_LEVELS]
    md += ([f"Over {CONSISTENCY_LEVELS} level of `consistency_px` at p95: " + ", ".join(over) + "."] if over
           else [f"No slice is over {CONSISTENCY_LEVELS} level of `consistency_px` at p95."
                 if any(r[-1] is not None for r in rows1) else "No `consistency_px` in this run (before E12-R7)."]) + [""]
    md += ["The `image` encoder writes 4:4:4 only (`encode.rs`): its 4:2:0 column is empty by construction.", ""]
    md += ["### Per group", ""]
    rows1g = []
    for (cfg, sl, en, g), rs in sorted(group_by(vend, "config", "slice", "encoder", "group").items(), key=lambda kv: (configs.index(kv[0][0]), kv[0][1], kv[0][2], kv[0][3])):
        s = summary(rs)
        rows1g.append([cfg, sl, en, g, s["n"], s["psnr_median"], s["psnr_p5"], s["de_median"], s["de_p95"]])
    md += [table(["config", "slice", "encoder", "group", "n", "PSNR med", "PSNR p5", "ΔE med", "ΔE p95"], rows1g), ""]
    md += ["### Per row (profile and map), png and the target slices, Pillow or none", ""]
    rows1r = []
    for (cfg, sl, row), rs in sorted(group_by([r for r in vend if r["encoder"] in ("none", "pillow") and (r["slice"] == "png" or matches(r["slice"], targets))], "config", "slice", "row").items(), key=lambda kv: (configs.index(kv[0][0]), kv[0][1], kv[0][2])):
        s = summary(rs)
        rows1r.append([cfg, sl, row, s["n"], s["restored"], s["psnr_median"], s["psnr_p5"], s["de_median"]])
    md += [table(["config", "slice", "row", "n", "restored", "PSNR med", "PSNR p5", "ΔE med"], rows1r), ""]

    md += ["## 2. The blend-model matrix (A5)", "",
           "Median PSNR in the ROI, k = 1, per inverse (a config's) and composite model. Each inverse must win "
           "on its own model's composites and lose on the other's; a config that wins on both smooths.", ""]
    rows2 = []
    for cfg in configs:
        mine = [r for r in ok if r["config"] == cfg and float(r.get("k", 1)) == 1.0]
        for sl, en in sorted({(r["slice"], r["encoder"]) for r in mine}):
            cells = []
            for model in ("encoded", "linear-light"):
                here = [r for r in mine if r["slice"] == sl and r["encoder"] == en and r.get("model") == model]
                cells += [median([r["psnr_roi"] for r in here]),
                          f"{sum(1 for r in here if r.get('restored'))}/{len(here)}"]
            rows2.append([cfg, mine[0].get("inverse"), sl, en] + cells)
    md += [table(["config", "inverse", "slice", "encoder", "on encoded", "restored", "on linear-light", "restored"], rows2), ""]
    if len({r.get("inverse") for r in ok}) < 2:
        md += ["Only one inverse was run: A5 is not decided here. The column the config's inverse does not "
               "model is what any other inverse has to beat.", ""]

    md += ["## 3. The measures against the truth", "",
           "Over restored files of vendor-blended composites, per slice (Pillow or none): how far the "
           "restoration's own measures follow the truth they stand in for on real files.", ""]
    rows3 = []
    for (cfg, sl), rs in sorted(group_by([r for r in vend if r.get("measures") and r["encoder"] in ("none", "pillow")], "config", "slice").items()):
        m = [r["measures"] for r in rs]
        p = [r["psnr_roi"] for r in rs]
        d = [r["de2000_roi"] for r in rs]
        tex = [x["texture"] for x in m]
        chroma = [x["chroma"] for x in m]
        step = [abs(x["step"]) for x in m]
        rows3.append([cfg, sl, len(rs), pearson(tex, p), spearman(tex, p), pearson(chroma, d), spearman(chroma, d),
                      pearson(step, p), spearman(step, p)])
    md += [table(["config", "slice", "n", "texture↔PSNR r", "ρ", "chroma↔ΔE r", "ρ", "|step|↔PSNR r", "ρ"], rows3), ""]

    md += ["## 4. Tails", "", f"The ten worst files by PSNR of each target slice ({', '.join(targets)}) and of png, per config.", ""]
    for cfg in configs:
        for sl in sorted({r["slice"] for r in vend if r["slice"] == "png" or matches(r["slice"], targets)}):
            rs = sorted([r for r in vend if r["config"] == cfg and r["slice"] == sl], key=lambda r: r["psnr_roi"])[:10]
            if not rs:
                continue
            md += [f"**{cfg} · {sl}**", ""]
            md += [table(["case", "encoder", "tone", "PSNR", "input PSNR", "ΔE", "exit", "verdict", "refusal",
                          "outline", "step", "chroma", "texture"],
                         [[f"{r['background']}/{r['case']}", r["encoder"], r.get("tone"), r["psnr_roi"],
                           r.get("psnr_roi_input"), r["de2000_roi"], r["exit"], r["detection"].get("verdict"),
                           (r["detection"].get("refusal") or {}).get("why"),
                           (r.get("measures") or {}).get("outline"), (r.get("measures") or {}).get("step"),
                           (r.get("measures") or {}).get("chroma"), (r.get("measures") or {}).get("texture")]
                          for r in rs]), ""]

    md += ["## 5. Does the bench reproduce the real failures? (§6.4)", "",
           "On the vendor's 2048 files (D247, D250, D252): chroma 7.40–8.37 at 4:2:0 q95; texture 8.59–9.22 at "
           "4:4:4 q95; refused out of range on 10 of 21 at 4:2:0 q90. All Pillow 12.3.0.", ""]
    rows5 = []
    tones = sorted({r.get("tone") for r in vend if r.get("tone")})
    for cfg in configs:
        def share(sl, pred, pool_pred, tone=None):
            pool = [r for r in vend if r["config"] == cfg and r["slice"] == sl and r["encoder"] == "pillow"
                    and pool_pred(r) and (tone is None or r.get("tone") == tone)]
            hit = [r for r in pool if pred(r)]
            return f"{len(hit)}/{len(pool)}" if pool else "–"
        restored = lambda r: r.get("measures") is not None
        found = lambda r: r["detection"].get("found")
        out_of_range = lambda r: (r["detection"].get("refusal") or {}).get("why") == "out-of-range"
        checks = [
            ("jpeg420-q95", f"chroma > {CHROMA_BOUND} (of restored)", lambda r: r["measures"]["chroma"] > CHROMA_BOUND, restored),
            ("jpeg444-q95", f"texture > {TEXTURE_BOUND} (of restored)", lambda r: r["measures"]["texture"] > TEXTURE_BOUND, restored),
            ("jpeg420-q90", "refused out of range (of found)", out_of_range, found),
            ("jpeg420-q95", "refused out of range (of found)", out_of_range, found),
        ]
        for sl, what, pred, pool in checks:
            rows5.append([cfg, sl, what, share(sl, pred, pool)] + [share(sl, pred, pool, t) for t in tones])
    md += [table(["config", "slice", "what", "all"] + tones, rows5), ""]
    md += ["The stickers' corner is `sticker-green` (about (7, 150, 58)); `saturated` has a channel at 0–4 "
           "and is harsher under 4:2:0.", ""]
    md += ["The ranges behind it, per tone and row (Pillow; restored n / all; the measures over the restored; "
           "the out-of-range share in % over every file the second proof measured):", ""]
    rows5r = []
    def span(v):
        return f"{min(v):.2f}–{max(v):.2f}" if v else "–"
    for (cfg, sl, tone, row), rs in sorted(group_by([r for r in vend if r["encoder"] == "pillow" and r["slice"] in
                                                     ("jpeg420-q95", "jpeg420-q90", "jpeg444-q95")],
                                                    "config", "slice", "tone", "row").items(), key=str):
        res = [r["measures"] for r in rs if r.get("measures")]
        oor = [100 * r["detection"]["scores"]["out_of_range"] for r in rs if r["detection"].get("scores")]
        refused = sum(1 for r in rs if (r["detection"].get("refusal") or {}).get("why") == "out-of-range")
        rows5r.append([cfg, sl, tone, row, f"{len(res)}/{len(rs)}", refused, span([m["chroma"] for m in res]),
                       span([m["texture"] for m in res]), span(oor)])
    md += [table(["config", "slice", "tone", "row", "restored", "refused out of range", "chroma", "texture",
                  "out of range %"], rows5r), ""]

    md += ["## 6. The self-test: exact on png with the canonical map", "",
           "Canonical rows (a GWT map at a row's own place and size, not fitted), blended in code values at k = 1, "
           "lossless. `exact` is claimed only with nothing clamped (`restore.rs`); a clamp is the rounding of a "
           "fractional logo over a channel at 0 or 255 and is counted apart.", ""]
    rows6 = []
    for cfg in configs:
        for row, rs in sorted(group_by([r for r in canonical_png(ok) if r["config"] == cfg], "row").items()):
            restored = [r for r in rs if r.get("restored")]
            clamped = [r for r in restored if (r["measures"] or {}).get("clamped")]
            exact = [r for r in restored if (r["measures"] or {}).get("exact")]
            unclamped = len(restored) - len(clamped)
            rows6.append([cfg, row[0], len(rs), len(restored), len(clamped), f"{len(exact)}/{unclamped}",
                          min((r["psnr_roi"] for r in restored), default=None)])
    md += [table(["config", "row", "n", "restored", "clamped", "exact / not clamped", "lowest PSNR restored"], rows6), ""]
    for cfg in configs:
        share, n = exact_share([r for r in ok if r["config"] == cfg])
        md.append(f"* {cfg}: exact on {fmt(None if share is None else 100 * share, 1)} % of {n} restored with nothing clamped.")
    md.append("")
    missed = group_by([r for r in canonical_png(ok) if not r.get("restored")], "config", "tone")
    if missed:
        md += ["Not restored, by the tone under the mark:", ""]
        md += [table(["config", "tone", "n", "not found", "refused (why)"],
                     [[k[0], k[1], len(v), sum(1 for r in v if not r["detection"].get("found")),
                       ", ".join(sorted({str((r["detection"].get("refusal") or {}).get("why")) for r in v if r["detection"].get("found")}))]
                      for k, v in sorted(missed.items(), key=str)]), ""]

    md += ["## 7. Detection, and D154 through the `R-k` composites", ""]
    rows7 = []
    for (cfg, sl, en), rs in sorted(group_by(vend, "config", "slice", "encoder").items()):
        f = [r for r in rs if r["detection"].get("found")]
        v = [r for r in f if r["detection"].get("verdict") == "verified"]
        rows7.append([cfg, sl, en, len(rs), len(f), len(v), median([r["detection"]["rect_error"] for r in f]),
                      max((r["detection"]["rect_error"] for r in f), default=None)])
    md += [table(["config", "slice", "encoder", "n", "found", "verified", "rect error med", "max"], rows7), ""]
    rk = [r for r in ok if float(r.get("k", 1)) != 1.0]
    if rk:
        md += ["A mark drawn at k = 0.93 against a profile of k = 1 (D154): it should be refused by its gain.", ""]
        rows7k = []
        for (cfg, sl, en), rs in sorted(group_by(rk, "config", "slice", "encoder").items()):
            gain = sum(1 for r in rs if (r["detection"].get("refusal") or {}).get("why") == "gain")
            row_proved = sum(1 for r in rs if r["detection"].get("verdict") == "verified" and r["detection"].get("placed") == "row")
            search_proved = sum(1 for r in rs if r["detection"].get("verdict") == "verified" and r["detection"].get("placed") == "searched")
            rows7k.append([cfg, sl, en, len(rs), gain, row_proved, search_proved,
                           sum(1 for r in rs if not r["detection"].get("found")),
                           median([r["psnr_roi"] for r in rs if r.get("restored")])])
        md += [table(["config", "slice", "encoder", "n", "refused by gain", "proved at the row", "proved by the search",
                      "not found", "PSNR med of the restored"], rows7k), ""]

    md += ["## 8. The two encoders apart", ""]
    lines8 = []
    for cfg in configs:
        for sl in sorted({r["slice"] for r in vend}):
            p = [r["psnr_roi"] for r in vend if r["config"] == cfg and r["slice"] == sl and r["encoder"] == "pillow"]
            i = [r["psnr_roi"] for r in vend if r["config"] == cfg and r["slice"] == sl and r["encoder"] == "image"]
            if p and i:
                gap = median(p) - median(i)
                lines8.append([cfg, sl, median(p), median(i), gap, "**over 0.3 dB**" if abs(gap) > ENCODER_GAP_DB else ""])
    md += [table(["config", "slice", "Pillow med", "image med", "Pillow − image", ""], lines8) if lines8 else "No slice has both encoders.", ""]

    md += ["## 9. What was not done", "",
           "* The groups and slices absent from this file are listed above by their absence; this report "
           "claims nothing about them.",
           "* A5 needs a second inverse (R9's R-lin); one inverse is a row of the matrix, not a verdict.",
           "* `consistency_dct` (D305) is written by R8's `R8d` only; §10 reads it.",
           ""]
    md += r8_section(vend, configs)
    return "\n".join(md)


def r8_rows(rows):
    """Restored lossy files, with their measures."""
    return [r for r in rows if r.get("restored") and r.get("measures") and matches(r["slice"], LOSSY_SLICES)]


def ratio(r):
    m = r["measures"]
    around = m.get("texture_around")
    return m["texture"] / around if around else None


def r8_checks(rows, config):
    """§6.2's two checks of E12-R8 for one config: `texture` under its bound on R8_TEXTURE_SLICE, and the
    soap check over every restored lossy file. Each `{"what", "share", "n", "need", "ok"}`; `ok` None with no
    file."""
    mine = r8_rows([r for r in rows if r["config"] == config])
    out = []
    target = [r for r in mine if r["slice"] == R8_TEXTURE_SLICE]
    under = [r for r in target if r["measures"]["texture"] < TEXTURE_BOUND]
    share = len(under) / len(target) if target else None
    out.append({"what": f"texture < {TEXTURE_BOUND} on {R8_TEXTURE_SLICE}", "share": share, "n": len(target),
                "need": R8_TEXTURE_SHARE, "ok": None if share is None else share >= R8_TEXTURE_SHARE})
    rated = [x for x in (ratio(r) for r in mine) if x is not None]
    soap = sum(1 for x in rated if x >= SOAP_RATIO) / len(rated) if rated else None
    out.append({"what": f"texture / around >= {SOAP_RATIO} (the soap check)", "share": soap, "n": len(rated),
                "need": SOAP_SHARE, "ok": None if soap is None else soap >= SOAP_SHARE})
    return out


def r8_section(rows, configs):
    md = ["## 10. The value inside the interval (E12-R8)", "",
          "Restored lossy files, per config, slice and encoder: `texture` and its ratio to `texture_around` "
          f"(D307 says a patch under {SOAP_RATIO} of its surroundings), how many were refined and in how many "
          "rounds, and the largest `consistency_dct` (DCT-POCS: 0 by construction). Marks blended in code "
          "values, k = 1.", ""]
    body = []
    for (cfg, sl, en), rs in sorted(group_by(r8_rows(rows), "config", "slice", "encoder").items()):
        ratios = [x for x in (ratio(r) for r in rs) if x is not None]
        refined = [r for r in rs if r["measures"].get("interval")]
        dct = [r["measures"]["consistency_dct"] for r in rs
               if isinstance(r["measures"].get("consistency_dct"), (int, float))]
        body.append([cfg, sl, en, len(rs), median([r["measures"]["texture"] for r in rs]),
                     sum(1 for r in rs if r["measures"]["texture"] < TEXTURE_BOUND), median(ratios),
                     quantile(ratios, 0.05), sum(1 for r in rs if r["measures"].get("smoothed")), len(refined),
                     sum(1 for r in refined if r["measures"]["interval"].get("text")),
                     median([r["measures"]["interval"]["iterations"] for r in refined]),
                     max(dct) if dct else None])
    if not body:
        return md + ["No restored lossy file in this run.", ""]
    md += [table(["config", "slice", "encoder", "restored", "texture med", f"texture < {TEXTURE_BOUND}",
                  "ratio med", "ratio p5", "smoothed", "refined", "as text", "rounds med",
                  "consistency_dct max"], body), ""]
    md += [f"§6.2's checks (`[tunable]`: {int(100 * R8_TEXTURE_SHARE)} % under {TEXTURE_BOUND} on "
           f"{R8_TEXTURE_SLICE}; the soap check on {int(100 * SOAP_SHARE)} %); A1–A7 are `gates`':", ""]
    for cfg in configs:
        for c in r8_checks(rows, cfg):
            verdict = "–" if c["ok"] is None else ("pass" if c["ok"] else "**fail**")
            md.append(f"* {cfg}: {c['what']}: {fmt(None if c['share'] is None else 100 * c['share'], 1)} % "
                      f"of {c['n']} (needs {int(100 * c['need'])} %) — {verdict}")
    md.append("")
    return md


# ───────────────────────────────────────────────────────── selftest

def fake(config, slice_id, psnr, i, *, group="flat", model="encoded", k=1.0, inverse="encoded",
         restored=True, sha=None, exact=True, clamped=0, found=True, err=0.0, variant="canonical", consistency=0.25,
         texture=1.0, around=1.0, interval=None, dct=None, smoothed=False, profile="gemini-sparkle-v1", row="v1-48"):
    return {
        "config": config, "inverse": inverse, "slice": slice_id, "encoder": "pillow", "group": group,
        "case_dir": f"{group}/bg-{i:03d}/case", "case": f"{row}.{model}", "background": f"bg-{i:03d}",
        "row": row, "profile": profile, "tone": "other", "variant": variant, "model": model, "k": k,
        "psnr_roi": psnr, "ssim_roi": 0.99, "de2000_roi": 0.5, "psnr_roi_input": 15.0,
        "restored": restored, "restored_sha256": sha or f"{config}-{i}", "exit": 1,
        "detection": {"found": found, "verdict": "verified" if found else None, "rect_error": err, "placed": "row"},
        "measures": {"exact": exact, "clamped": clamped, "texture": texture, "texture_around": around,
                     "chroma": 1.0, "step": 0.1, "outline": 0.0, "consistency_px": consistency,
                     "consistency_excluded": clamped, "consistency_dct": dct, "smoothed": smoothed,
                     "interval": interval} if restored else None,
    }


def selftest():
    problems = []

    def expect(name, cond):
        print(("ok   " if cond else "FAIL ") + name)
        if not cond:
            problems.append(name)

    target = ["jpeg420-q95"]
    # R0 on the target slice: 100 files at 40 dB.
    base = [fake("R0", "jpeg420-q95", 40.0, i) for i in range(100)]
    # A candidate whose mean and median rise by a dB and whose worst tenth falls by 3.
    worse_tail = [fake("C", "jpeg420-q95", 41.0 if i >= 10 else 37.0, i) for i in range(100)]
    a1 = gate_a1(base, worse_tail, target)
    expect("a better mean and median with a worse p5 fails A1", not a1["ok"])
    expect("… though its mean did improve", a1["slices"][0]["mean_delta"] > 0 and a1["slices"][0]["median_delta"] > 0.5)
    better = [fake("C", "jpeg420-q95", 40.6, i) for i in range(100)]
    expect("+0.6 dB everywhere passes A1", gate_a1(base, better, target)["ok"])
    small = [fake("C", "jpeg420-q95", 40.4, i) for i in range(100)]
    expect("+0.4 dB at the median fails A1", not gate_a1(base, small, target)["ok"])
    # A2 on a non-target slice.
    b2 = [fake("R0", "png", 60.0, i) for i in range(100)]
    expect("a non-target slice 0.05 dB down passes A2", gate_a2(b2, [fake("C", "png", 59.95, i) for i in range(100)], target)["ok"])
    expect("a non-target slice 0.2 dB down at the median fails A2", not gate_a2(b2, [fake("C", "png", 59.8, i) for i in range(100)], target)["ok"])
    tail2 = [fake("C", "png", 60.0 if i >= 10 else 59.7, i) for i in range(100)]
    expect("a non-target p5 0.3 dB down fails A2", not gate_a2(b2, tail2, target)["ok"])
    # A3: the text group.
    bt = [fake("R0", "png", 50.0, i, group="text") for i in range(50)]
    expect("text 0.2 dB down passes A3", gate_a3(bt, [fake("C", "png", 49.8, i, group="text") for i in range(50)])["ok"])
    expect("text 0.4 dB down fails A3", not gate_a3(bt, [fake("C", "png", 49.6, i, group="text") for i in range(50)])["ok"])
    # A4: png byte-equal on the lossy route.
    b4 = [fake("R0", "png", 60.0, i, sha=f"s{i}") for i in range(10)]
    same = [fake("C", "png", 60.0, i, sha=f"s{i}") for i in range(10)]
    moved = same[:9] + [fake("C", "png", 60.0, 9, sha="other")]
    expect("png byte-equal passes A4 on lossy", gate_a4(b4, same, "lossy")["ok"])
    expect("one png moved fails A4 on lossy", not gate_a4(b4, moved, "lossy")["ok"])
    expect("a png moved is no A4 matter on model", gate_a4(b4, moved, "model")["ok"])
    # A5: the matrix.
    def m5(cfg, inverse, enc, lin):
        return [fake(cfg, "png", enc, i, inverse=inverse) for i in range(5)] + \
               [fake(cfg, "png", lin, i, inverse=inverse, model="linear-light") for i in range(5)]
    r0 = m5("R0", "encoded", 55.0, 30.0)
    expect("each inverse winning on its own model passes A5", gate_a5(r0, m5("L", "linear-light", 40.0, 50.0))["ok"])
    expect("an inverse that wins on both fails A5 (it smooths)", not gate_a5(r0, m5("S", "linear-light", 56.0, 50.0))["ok"])
    expect("an inverse that loses on its own fails A5", not gate_a5(r0, m5("L", "linear-light", 40.0, 29.0))["ok"])
    # A6: exact on png canonical.
    b6 = [fake("R0", "png", 60.0, i) for i in range(10)]
    expect("all exact passes A6 on lossy", gate_a6(b6, [fake("C", "png", 60.0, i) for i in range(10)], "lossy")["ok"])
    one_off = [fake("C", "png", 60.0, i, exact=(i != 3)) for i in range(10)]
    expect("one not exact fails A6 on lossy", not gate_a6(b6, one_off, "lossy")["ok"])
    clamped = [fake("C", "png", 60.0, i, exact=(i != 3), clamped=(1 if i == 3 else 0)) for i in range(10)]
    expect("a clamped file is left out of the exact share", exact_share(clamped) == (1.0, 9))
    expect("… and on model it does not fail A6", gate_a6(b6, clamped, "model")["ok"])
    expect("… but on lossy its flag must still be R0's", not gate_a6(b6, clamped, "lossy")["ok"])
    expect("99 of 100 passes A6 on model", gate_a6(b6, [fake("C", "png", 60.0, i, exact=(i != 3)) for i in range(100)], "model")["ok"])
    # A7: detection.
    b7 = [fake("R0", "png", 60.0, i, err=0.0) for i in range(10)]
    expect("the same detection passes A7", gate_a7(b7, [fake("C", "png", 60.0, i, err=0.0) for i in range(10)])["ok"])
    expect("one found fewer fails A7", not gate_a7(b7, [fake("C", "png", 60.0, i, found=(i != 0)) for i in range(10)])["ok"])
    expect("a larger rect error fails A7", not gate_a7(b7, [fake("C", "png", 60.0, i, err=0.25) for i in range(10)])["ok"])
    # Numbers.
    expect("the quantile interpolates", quantile([1, 2, 3, 4, 5], 0.05) == 1.2 and median([1, 2, 3, 4]) == 2.5)
    expect("Spearman is 1 on a monotone pair", abs(spearman([1, 2, 3, 4], [10, 20, 25, 100]) - 1.0) < 1e-12)
    # The consistency column (E12-R7, D305): the p95 over restored files, a slice over a level named, none before R7.
    cons = [fake("R0", "png", 60.0, i, consistency=0.25 if i < 18 else 3.0) for i in range(20)]
    expect("consistency's p95 is the restored files' tail", summary(cons)["consistency_p95"] == 3.0)
    unrestored = [fake("R0", "png", 60.0, i, restored=False) for i in range(3)]
    expect("a file not restored has no consistency", summary(unrestored)["consistency_p95"] is None)
    md_c = report(cons, {}, target)
    expect("the report has the consistency column", "consist. p95" in md_c)
    expect("a slice over a level is named", "Over 1.0 level of `consistency_px` at p95: R0 png pillow" in md_c)
    md_ok = report([fake("R0", "png", 60.0, i) for i in range(20)], {}, target)
    expect("a consistent run says no slice is over", "No slice is over 1.0 level" in md_ok)
    old = [fake("R0", "png", 60.0, i) for i in range(5)]
    for r in old:
        del r["measures"]["consistency_px"]
    md_old = report(old, {}, target)
    expect("a run before E12-R7 says it has none", "No `consistency_px` in this run" in md_old)
    # E12-R8 §6.2: the texture under its bound on jpeg444-q95, and the soap check.
    rounds = {"method": "dct", "space": "ycbcr", "sigma_base": [0.3, 0.0, 0.0], "iterations": 2}
    r8 = [fake("R8d", "jpeg444-q95", 42.0, i, texture=3.0 if i < 9 else 7.0, around=3.0, interval=rounds, dct=0.0)
          for i in range(10)]
    checks = r8_checks(r8, "R8d")
    expect("90 % under the bound passes R8's texture check",
           checks[0]["ok"] and abs(checks[0]["share"] - 0.9) < 1e-9)
    expect("every ratio at 1 or over passes the soap check", checks[1]["ok"])
    soapy = [fake("R8d", "jpeg420-q95", 42.0, i, texture=1.5 if i < 2 else 3.0, around=3.0, interval=rounds,
                  smoothed=i < 2) for i in range(20)]
    expect("two in twenty at half their surroundings fail the soap check", r8_checks(soapy, "R8d")[1]["ok"] is False)
    expect("a config with no restored lossy file says nothing", r8_checks(soapy, "R0")[1]["ok"] is None)
    md_r8 = report(r8 + soapy, {}, target)
    expect("§10 names each check's verdict", "R8d: texture < 5.5 on jpeg444-q95: 90.0 %" in md_r8
           and "(the soap check): 93.3 % of 30 (needs 95 %) — **fail**" in md_r8)
    expect("§10's table counts the smoothed", "| R8d | jpeg420-q95 | pillow | 20 |" in md_r8)
    # E12-R12 stage 4b: any profile. The report and the gates read the lines, whatever mark made them; §0 names
    # the profiles; --profile keeps one profile's lines out of a concatenation.
    wm, wr = "fixture-wordmark", "fixture-wordmark-72x24-1024x1024"
    other = [fake("R0", "png", 50.0, i, profile=wm, row=wr) for i in range(6)] + \
            [fake("R0", "png", 30.0, i, profile=wm, row=wr, model="linear-light") for i in range(6)]
    md_other = report(other, {}, target)
    expect("a profile that is not Gemini's renders every section", all(f"## {n}." in md_other for n in range(11)))
    expect("§0 names the profile it is over", f"* profiles: {wm}" in md_other)
    expect("its matrix has a row of its own", "| R0 | encoded | png | pillow |" in md_other)
    expect("its row is reported by its id", f"| {wr} |" in md_other)
    mixed = base + other
    expect("--profile keeps one profile's lines", of_profiles(mixed, ["fixture-*"]) == other)
    expect("no pattern keeps every line", of_profiles(mixed, []) == mixed)
    # The whole road, through files: a report and a failing gate run.
    with tempfile.TemporaryDirectory() as d:
        p = os.path.join(d, "results.jsonl")
        with open(p, "w") as f:
            for r in base + worse_tail:
                f.write(json.dumps(r) + "\n")
        md = report(load(p), sidecars(p), target)
        expect("the report renders every section", all(f"## {n}." in md for n in range(11)))
        code = main(["gates", p, "--candidate", "C", "--route", "lossy", "--targets", "jpeg420-*", "--out", os.path.join(d, "g.json")])
        expect("gates exits 1 when A1 fails", code == 1)
        code = main(["gates", p, "--candidate", "nobody", "--route", "lossy", "--targets", "jpeg420-*"])
        expect("gates exits 2 for a candidate with no results", code == 2)
        q = os.path.join(d, "mixed.jsonl")
        with open(q, "w") as f:
            for r in mixed:
                f.write(json.dumps(r) + "\n")
        out = os.path.join(d, "fixture.md")
        code = main(["report", q, "--profile", "fixture-*", "--out", out])
        with open(out) as f:
            md_f = f.read()
        expect("report --profile reports that profile alone", code == 0 and f"* profiles: {wm}\n" in md_f)
        expect("report --profile with no match exits 2", main(["report", q, "--profile", "nobody-*"]) == 2)
    print("selftest:", "FAIL" if problems else "ok", f"({len(problems)} failed)")
    return 1 if problems else 0


# ───────────────────────────────────────────────────────── main

def main(argv):
    if not argv:
        print(__doc__, file=sys.stderr)
        return 2
    cmd, rest = argv[0], argv[1:]
    if cmd == "selftest":
        return selftest()
    opts, pos = {}, []
    i = 0
    while i < len(rest):
        if rest[i].startswith("--"):
            if i + 1 >= len(rest):
                print(f"report.py: {rest[i]} needs a value", file=sys.stderr)
                return 2
            opts[rest[i][2:]] = rest[i + 1]
            i += 2
        else:
            pos.append(rest[i])
            i += 1
    if len(pos) != 1:
        print(__doc__.split("How to run it")[1].split("What it needs")[0], file=sys.stderr)
        return 2
    rows = load(pos[0])
    if "profile" in opts:
        rows = of_profiles(rows, [p for p in opts["profile"].split(",") if p])
        if not rows:
            print(f"report.py: no result of a profile matching {opts['profile']}", file=sys.stderr)
            return 2
    targets = opts.get("targets", ",".join(DEFAULT_TARGETS)).split(",")
    if cmd == "report":
        md = report(rows, sidecars(pos[0]), targets)
        if "out" in opts:
            with open(opts["out"], "w") as f:
                f.write(md + "\n")
        else:
            print(md)
        return 0
    if cmd == "gates":
        if "candidate" not in opts or opts.get("route") not in ("lossy", "model"):
            print("report.py: gates needs --candidate and --route lossy|model", file=sys.stderr)
            return 2
        g = gates(rows, opts["candidate"], opts.get("baseline", "R0"), opts["route"], targets)
        if g is None:
            print("report.py: the candidate or the baseline has no results", file=sys.stderr)
            return 2
        for gate in g["gates"]:
            print(f"{gate['gate']}: {'pass' if gate['ok'] else 'FAIL'}")
        if "out" in opts:
            with open(opts["out"], "w") as f:
                json.dump(g, f, indent=2)
                f.write("\n")
        return 0 if g["ok"] else 1
    print(f"report.py: unknown command {cmd}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
