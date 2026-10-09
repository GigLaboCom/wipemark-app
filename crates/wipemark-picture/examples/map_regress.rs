//! Can a per-pixel logo colour `L(p)` be told from the opacity `α(p)`? —
//! not a feature, a developer's tool (D162), as `measure_map` is.
//!
//! **What it is for.** Step E12-R4 of the E12-R series
//! (`docs/plan/E12-R4-corpus-analytics.md` §4.4, filed 2026-10-08 by the
//! coordinator from the owner's spec `wipemark-recon-spec-2026-10-08`,
//! `04-corpus-analytics.md` §4; D240, D243). `gemini-v1-96-measured` was
//! fitted by `measure_map` over one saturated-green background *assuming*
//! a global logo colour, so on those files `α(p)` and `L(p)` cannot be told
//! apart. Over many backgrounds they can: the blend `I = α·L + (1 − α)·O`
//! is linear in `O`, so per pixel the regression of `I` on `O` gives the
//! slope `1 − α(p)` and the intercept `α(p)·L(p)` separately. This tool is
//! that regression — `measure_map` generalised — and the check that says
//! whether what it finds holds on files it did not see.
//!
//! **What it does.**
//!
//! 1. Reads a list of pictures with their group and whether each is held
//!    out (R2's `gemini-midtone`, `corpus/gemini-midtone/manifest.json`,
//!    through `scripts/analytics/bias.py list`). Decodes each as every
//!    surface does. A file whose size the named row does not answer for is
//!    skipped and counted.
//! 2. Takes the row's own rectangle — the map at its own size, at the
//!    corner and margin the catalogue gives; a row that resamples its map is
//!    refused — and under it, per file, the picture under the mark `Ô_i(p)`
//!    as the calibration estimates it: a quadratic per channel over a ring
//!    around the rectangle (`wipemark_pixels::ring_background`, the very
//!    fit `calibrate` makes; `--ring`, 6 px by default as there), not the
//!    ring's mean `measure_map` uses.
//! 3. **The fit** uses only the files that are not held out and whose group
//!    is smooth — `gray-*`, `black`, `white`, `sat-*`, `gradient`; never
//!    `texture`. Per pixel, least squares of `I_c = a_c + s·Ô_c` over every
//!    file and channel, with **one slope** `s = 1 − α` for all channels and
//!    one intercept `a_c = α·L_c` per channel: `s = Σ_c S_xy,c / Σ_c S_xx,c`,
//!    `a_c = Ī_c − s·Ō_c`. A stored sample at 0 or at the format's maximum
//!    is left out (a clipped value is not on the line). `L_c = a_c/α` where
//!    `α > 0.1`.
//! 4. Writes to `--out`:
//!    * `alpha_reg.wma` — `α_reg`, depth 16 (undefined pixels 0), and its
//!      sha256 in the summary;
//!    * `pixels.tsv` — per pixel: `α_reg`, the row's map `α_map` and the
//!      difference, `L_reg` for R, G, B (the three planes; empty where
//!      `α_reg ≤ 0.1`), `R²`, the point count, and the spread of `Ô` among
//!      those points (its pooled standard deviation, minimum and maximum);
//!    * `summary.txt` — the comparisons below, also printed.
//! 5. **Compares**: `α_reg` against the row's map (`gemini-v1-96-measured`
//!    for V1's large row) — the p95 and the largest `|Δα|` over the mark's
//!    support; `L_reg` against the profile's global logo (V1:
//!    `[252.1, 253.5, 252.8]`) where `α_reg > 0.1` — the mean, p95 and
//!    largest `|ΔL|` per channel, and the correlation of `ΔL_c` with `α_reg`
//!    (whether the departure follows the logo's shape); `R²` (median, p5)
//!    and the point counts.
//! 6. **The held-out check** (§4.4): D240's residual
//!    `r = I − (α·L + (1 − α)·Ô)` in channels where `Ô ≤ 8` levels ("`O ≈
//!    0`", `[tunable]`) on the `sat-*` and `black` files, over pixels where
//!    both `α_map` and `α_reg` are over 0.1 — under today's model (the row's
//!    map, the global logo) and under the regression's (`α_reg`, `L_reg`),
//!    on the **held-out** files and, apart, on the training files. The
//!    change is accepted when the held-out largest `|r|` falls to 1.5 levels
//!    or less (`[tunable]`); a fall on the training files alone is a
//!    rejection, and the summary says which.
//! 7. A **mechanical reading** against §4.4's table — none while `Ô`'s
//!    spread between the files is under 20 levels (its pooled standard
//!    deviation, the median over the square, `[tunable]`): over one
//!    background the slope is the noise's, which is §4.4's trap — then
//!    which of its four rows
//!    the numbers fall in by its own words (`α_reg ≈ α_map`: p95 `|Δα|` ≤
//!    0.01; `L_reg ≈` global: p95 `|ΔL|` ≤ 1.5 in every channel; a
//!    structural departure: `|corr(ΔL_c, α)|` ≥ 0.5 in a channel whose p95
//!    is over 1.5; `R²` low: its median under 0.9). Arithmetic; the
//!    conclusion is the report's.
//!
//! **How to run it** (optimised):
//!
//! ```sh
//! cargo run --release -p wipemark-picture --example map_regress -- \
//!     --profile gemini-sparkle-v1 --row 0 --list midtone.tsv --out <dir> [--ring 6]
//! cargo run --release -p wipemark-picture --example map_regress -- \
//!     --background <picture> --profile gemini-sparkle-v1 --row 0 [--ring 6] --out <ohat.tsv>
//! cargo run --release -p wipemark-picture --example map_regress -- --selftest
//! ```
//!
//! `--background` writes `Ô` under the row's rectangle of one picture
//! (`x y R G B`, 8-bit units) — what `scripts/analytics/bias.py
//! check-background` holds its Python restatement of the quadratic to.
//! `list.tsv` is `path<TAB>group<TAB>held_out` (`true`/`false`), `#`
//! comments and a `path…` header allowed; a relative path is taken from
//! the list's folder.
//!
//! **What it needs.** Nothing beyond this workspace. The pictures are R2's
//! `gemini-midtone` (the host's; the owner's pictures never go into git).
//! `--selftest` needs none: three synthetic backgrounds — grey 0, 128 and
//! 255, then 0, 128 and 255 in red and green with blue at 0 in all three
//! (the `sat-*` case D240 is about, where only a slope shared across
//! channels can tell `α` from `L_B`) — over the shipped V1 measured map and
//! a known `L(p)` that departs along the logo's shape, at 16 bits; `α` must
//! come back within 1/255 wherever it is above 0, `L` within a level where
//! `α > 0.1`, and the held-out check must accept the regression and not
//! today's model.
//!
//! **What its output means.** See §4.4's table: the map right and `L`
//! global → the blend model (R9 R-lin); `L` departing along the logo with
//! `α` right → a per-pixel `L` is real (R9 R-lm, the held-out check
//! mandatory); `α` departing with `L` global → re-fit the map by regression
//! (a new `-measured`, D243's process), no `logo_map`; both departing with a
//! low `R²` → not linear in stored codes (R9 R-lin first).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use sha2::{Digest, Sha256};
use wipemark_picture::decode;
use wipemark_pixels::{
    ring_background, AlphaMap, Anchor, Catalogue, Layout, PixelRect, Profile, Raster, NOISE_FLOOR,
};

/// Where `L` is defined: `α` above this (§4.4).
const L_ALPHA: f64 = 0.1;
/// "`O ≈ 0`" for D240's residual, in 8-bit levels. `[tunable]`
const O_ZERO: f64 = 8.0;
/// The held-out check's bound on D240's residual, in levels. `[tunable]`
const HELD_OUT_BOUND: f64 = 1.5;
/// `α_reg ≈ α_map`: the p95 of `|Δα|` at most this (§4.4).
const ALPHA_SAME: f64 = 0.01;
/// `L_reg ≈` global: the p95 of `|ΔL|` at most this, in levels (§4.4).
const L_SAME: f64 = 1.5;
/// A departure "along the logo": `|corr(ΔL, α)|` at least this. `[tunable]`
const ALONG_LOGO: f64 = 0.5;
/// "`R²` low": its median under this. `[tunable]`
const R2_LOW: f64 = 0.9;
/// A reading needs `Ô` to move between the files by at least this (its
/// pooled standard deviation per pixel, median over the square, in
/// levels): over one background the slope is the noise's. `[tunable]`
const MIN_O_SPREAD: f64 = 20.0;
/// The calibration's ring (`CalibrateOptions::default().ring`).
const RING: u32 = 6;

/// A group whose background a quadratic describes: the fit takes these.
fn smooth(group: &str) -> bool {
    group.starts_with("gray-")
        || group.starts_with("sat-")
        || matches!(group, "black" | "white" | "gradient")
}

/// The groups D240's residual is measured on.
fn zero_channel(group: &str) -> bool {
    group.starts_with("sat-") || group == "black"
}

// ------------------------------------------------------------ the row

/// The row's own rectangle in a `w × h` picture, with its map at its own
/// size; an error for a row that resamples, does not answer for the size,
/// or does not fit.
fn row_rect(profile: &Profile, row: usize, w: u32, h: u32) -> Result<PixelRect, String> {
    let p = profile
        .placements
        .get(row)
        .ok_or_else(|| format!("{} has no row {row}", profile.id))?;
    if p.resample {
        return Err(format!(
            "{} row {row} resamples its map: no per-pixel regression",
            profile.id
        ));
    }
    if !p.when.matches(w, h) {
        return Err(format!(
            "{} row {row} does not answer for {w}×{h}",
            profile.id
        ));
    }
    let map = profile.map(p.alpha);
    let (x, y) = match p.anchor {
        Anchor::Corner { corner, margin } => corner
            .origin(w, h, map.width(), map.height(), margin)
            .ok_or_else(|| format!("{} row {row} does not fit {w}×{h}", profile.id))?,
        Anchor::Rect(r) => {
            if r.width != map.width() || !r.inside(w, h) {
                return Err(format!(
                    "{} row {row}: not the map at its own size",
                    profile.id
                ));
            }
            (r.x, r.y)
        }
    };
    Ok(PixelRect {
        x,
        y,
        width: map.width(),
        height: map.height(),
    })
}

/// One picture under the row: the stored values in 8-bit units, whether
/// each is clipped, and `Ô` from the ring.
struct Under {
    i: Vec<[f64; 3]>,
    clipped: Vec<[bool; 3]>,
    o: Vec<[f64; 3]>,
}

fn under(raster: &Raster, rect: PixelRect, ring: u32) -> Result<Under, String> {
    let o = ring_background(raster, rect, ring).ok_or("no ring around the mark")?;
    let max = raster.layout().max();
    let scale = 255.0 / f64::from(max);
    let c = raster.layout().channels();
    let mut i = Vec::with_capacity(o.len());
    let mut clipped = Vec::with_capacity(o.len());
    for y in rect.y..rect.y + rect.height {
        for x in rect.x..rect.x + rect.width {
            let at = ((y * raster.width() + x) as usize) * c;
            let s = &raster.samples()[at..at + 3];
            i.push([0, 1, 2].map(|k| f64::from(s[k]) * scale));
            clipped.push([0, 1, 2].map(|k| s[k] == 0 || s[k] == max));
        }
    }
    Ok(Under { i, clipped, o })
}

// ------------------------------------------------------------ the fit

/// Sums for one pixel's regression, per channel.
#[derive(Clone, Default)]
struct Acc {
    n: [f64; 3],
    so: [f64; 3],
    si: [f64; 3],
    soo: [f64; 3],
    soi: [f64; 3],
    sii: [f64; 3],
    omin: f64,
    omax: f64,
}

fn new_acc(n: usize) -> Vec<Acc> {
    vec![
        Acc {
            omin: f64::INFINITY,
            omax: f64::NEG_INFINITY,
            ..Acc::default()
        };
        n
    ]
}

fn accumulate(acc: &mut [Acc], u: &Under) {
    for (p, a) in acc.iter_mut().enumerate() {
        for c in 0..3 {
            if u.clipped[p][c] {
                continue;
            }
            let (o, i) = (u.o[p][c], u.i[p][c]);
            a.n[c] += 1.0;
            a.so[c] += o;
            a.si[c] += i;
            a.soo[c] += o * o;
            a.soi[c] += o * i;
            a.sii[c] += i * i;
            a.omin = a.omin.min(o);
            a.omax = a.omax.max(o);
        }
    }
}

/// One pixel's answer: `α = 1 − s`, `L_c = a_c/α` where `α > 0.1`.
#[derive(Clone, Copy, Default)]
struct Px {
    alpha: Option<f64>,
    l: [Option<f64>; 3],
    r2: Option<f64>,
    n: f64,
    o_std: f64,
    o_min: f64,
    o_max: f64,
}

/// The per-pixel least squares with one slope shared by the channels.
fn solve(a: &Acc) -> Px {
    let mut sxx = 0.0;
    let mut sxy = 0.0;
    let mut syy = 0.0;
    let mut m = [(0.0, 0.0); 3];
    for (c, mc) in m.iter_mut().enumerate() {
        if a.n[c] == 0.0 {
            continue;
        }
        let (mo, mi) = (a.so[c] / a.n[c], a.si[c] / a.n[c]);
        *mc = (mo, mi);
        sxx += a.soo[c] - a.n[c] * mo * mo;
        sxy += a.soi[c] - a.n[c] * mo * mi;
        syy += a.sii[c] - a.n[c] * mi * mi;
    }
    let n = a.n.iter().sum::<f64>();
    let mut px = Px {
        n,
        o_std: if n > 0.0 {
            (sxx.max(0.0) / n).sqrt()
        } else {
            0.0
        },
        o_min: a.omin,
        o_max: a.omax,
        ..Px::default()
    };
    // Less than a hundredth of a level of spread in Ô: no slope.
    if sxx <= 1e-4 * n.max(1.0) {
        return px;
    }
    let s = sxy / sxx;
    let alpha = 1.0 - s;
    px.alpha = Some(alpha);
    let res = syy - 2.0 * s * sxy + s * s * sxx;
    px.r2 = (syy > 0.0).then(|| 1.0 - res / syy);
    if alpha > L_ALPHA {
        for (c, (l, (mo, mi))) in px.l.iter_mut().zip(m).enumerate() {
            if a.n[c] > 0.0 {
                *l = Some((mi - s * mo) / alpha);
            }
        }
    }
    px
}

// ------------------------------------------------------------ statistics

fn quantile(v: &[f64], q: f64) -> f64 {
    let mut v = v.to_vec();
    v.sort_by(f64::total_cmp);
    if v.is_empty() {
        return f64::NAN;
    }
    let pos = q * (v.len() - 1) as f64;
    let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
    v[lo] + (v[hi] - v[lo]) * (pos - lo as f64)
}

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    if n < 2.0 {
        return f64::NAN;
    }
    let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (a, b) in x.iter().zip(y) {
        sxy += (a - mx) * (b - my);
        sxx += (a - mx) * (a - mx);
        syy += (b - my) * (b - my);
    }
    sxy / (sxx * syy).sqrt()
}

/// `|r|`'s mean, p95, p99 and largest, with the count.
struct Spread {
    n: usize,
    mean: f64,
    p95: f64,
    p99: f64,
    max: f64,
}

fn spread(r: &[f64]) -> Spread {
    let abs: Vec<f64> = r.iter().map(|v| v.abs()).collect();
    Spread {
        n: r.len(),
        mean: if r.is_empty() {
            f64::NAN
        } else {
            r.iter().sum::<f64>() / r.len() as f64
        },
        p95: quantile(&abs, 0.95),
        p99: quantile(&abs, 0.99),
        max: abs.iter().copied().fold(f64::NAN, f64::max),
    }
}

impl std::fmt::Display for Spread {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "n {}  mean r {:+.3}  p95 |r| {:.3}  p99 |r| {:.3}  max |r| {:.3}",
            self.n, self.mean, self.p95, self.p99, self.max
        )
    }
}

/// D240's residual under a model, over the files given: channels where
/// `Ô ≤ O_ZERO`, pixels where both maps are over `L_ALPHA`.
fn d240(
    files: &[&Under],
    map: &[f64],
    fit: &[Px],
    model: impl Fn(usize) -> Option<(f64, [f64; 3])>,
) -> Vec<f64> {
    let mut r = Vec::new();
    for u in files {
        for p in 0..map.len() {
            if map[p] <= L_ALPHA || fit[p].alpha.is_none_or(|a| a <= L_ALPHA) {
                continue;
            }
            let Some((a, l)) = model(p) else { continue };
            for (c, lc) in l.iter().enumerate() {
                if u.o[p][c] > O_ZERO || u.clipped[p][c] {
                    continue;
                }
                r.push(u.i[p][c] - (a * lc + (1.0 - a) * u.o[p][c]));
            }
        }
    }
    r
}

/// What the held-out check found.
struct HeldOut {
    today_held: Spread,
    reg_held: Spread,
    today_train: Spread,
    reg_train: Spread,
    verdict: String,
}

fn held_out_check(
    held: &[&Under],
    train: &[&Under],
    map: &[f64],
    fit: &[Px],
    logo: [f64; 3],
) -> HeldOut {
    let today = |p: usize| Some((map[p], logo));
    let reg = |p: usize| {
        let a = fit[p].alpha?;
        let l = fit[p].l;
        Some((a, [l[0]?, l[1]?, l[2]?]))
    };
    let today_held = spread(&d240(held, map, fit, today));
    let reg_held = spread(&d240(held, map, fit, reg));
    let today_train = spread(&d240(train, map, fit, today));
    let reg_train = spread(&d240(train, map, fit, reg));
    let verdict = if reg_held.n == 0 {
        "no held-out sample (no held-out sat-* or black file, or no channel at O ≈ 0): not checked"
            .to_string()
    } else if reg_held.max <= HELD_OUT_BOUND {
        format!(
            "accepted: on the held-out files the largest |r| falls from {:.2} to {:.2} (bound {HELD_OUT_BOUND})",
            today_held.max, reg_held.max
        )
    } else if reg_train.max <= HELD_OUT_BOUND {
        format!(
            "rejected: a fall on the training files alone ({:.2} → {:.2}); held out {:.2} → {:.2}",
            today_train.max, reg_train.max, today_held.max, reg_held.max
        )
    } else {
        format!(
            "rejected: held out {:.2} → {:.2}, over {HELD_OUT_BOUND}",
            today_held.max, reg_held.max
        )
    };
    HeldOut {
        today_held,
        reg_held,
        today_train,
        reg_train,
        verdict,
    }
}

// ------------------------------------------------------------ the run

struct Analysis {
    fit: Vec<Px>,
    summary: String,
    held: HeldOut,
}

/// Fit over `train`, compare with the row's map and the global logo,
/// check on `held`.
fn analyse(files: &[(String, String, bool, Under)], map: &AlphaMap, logo: [f64; 3]) -> Analysis {
    let n = (map.width() * map.height()) as usize;
    let mut acc = new_acc(n);
    let mut used = BTreeMap::<String, usize>::new();
    for (_, group, held, u) in files {
        if !*held && smooth(group) {
            accumulate(&mut acc, u);
            *used.entry(group.clone()).or_default() += 1;
        }
    }
    let fit: Vec<Px> = acc.iter().map(solve).collect();
    let amap: Vec<f64> = map.values().iter().map(|v| f64::from(*v)).collect();
    let mut s = String::new();
    let _ = writeln!(
        s,
        "fit over {} file(s): {}",
        used.values().sum::<usize>(),
        used.iter()
            .map(|(g, k)| format!("{g} {k}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let defined = fit.iter().filter(|p| p.alpha.is_some()).count();
    let _ = writeln!(s, "pixels with a slope: {defined} of {n}");
    let support: Vec<usize> = (0..n)
        .filter(|&p| amap[p] >= f64::from(NOISE_FLOOR) && fit[p].alpha.is_some())
        .collect();
    let da: Vec<f64> = support
        .iter()
        .map(|&p| fit[p].alpha.unwrap_or(0.0) - amap[p])
        .collect();
    let abs_da: Vec<f64> = da.iter().map(|v| v.abs()).collect();
    let alpha_p95 = quantile(&abs_da, 0.95);
    let _ = writeln!(
        s,
        "α_reg − α_map over the support (α_map ≥ {NOISE_FLOOR}, {} px): mean {:+.4}  p95 |Δα| {:.4}  max |Δα| {:.4}",
        support.len(),
        da.iter().sum::<f64>() / da.len().max(1) as f64,
        alpha_p95,
        abs_da.iter().copied().fold(0.0, f64::max)
    );
    let mut l_p95 = [f64::NAN; 3];
    let mut corr = [f64::NAN; 3];
    for c in 0..3 {
        let (mut dl, mut al) = (Vec::new(), Vec::new());
        for px in &fit {
            if let (Some(a), Some(l)) = (px.alpha, px.l[c]) {
                dl.push(l - logo[c]);
                al.push(a);
            }
        }
        let abs: Vec<f64> = dl.iter().map(|v| v.abs()).collect();
        l_p95[c] = quantile(&abs, 0.95);
        corr[c] = pearson(&dl, &al);
        let _ = writeln!(
            s,
            "L_reg − L[{c}] ({:.1}) where α_reg > {L_ALPHA} ({} px): mean {:+.3}  p95 |ΔL| {:.3}  max |ΔL| {:.3}  corr(ΔL, α) {:+.3}",
            logo[c],
            dl.len(),
            dl.iter().sum::<f64>() / dl.len().max(1) as f64,
            l_p95[c],
            abs.iter().copied().fold(0.0, f64::max),
            corr[c]
        );
    }
    let r2: Vec<f64> = (0..n)
        .filter(|&p| amap[p] > L_ALPHA)
        .filter_map(|p| fit[p].r2)
        .collect();
    let r2_median = quantile(&r2, 0.5);
    let counts: Vec<f64> = (0..n).map(|p| fit[p].n).collect();
    let ostd: Vec<f64> = (0..n)
        .filter(|&p| fit[p].n > 0.0)
        .map(|p| fit[p].o_std)
        .collect();
    let _ = writeln!(
        s,
        "R² where α_map > {L_ALPHA}: median {r2_median:.4}  p5 {:.4}; points a pixel {}–{}; spread of Ô (pooled sd) median {:.1}, min {:.1}",
        quantile(&r2, 0.05),
        counts.iter().copied().fold(f64::INFINITY, f64::min),
        counts.iter().copied().fold(0.0, f64::max),
        quantile(&ostd, 0.5),
        ostd.iter().copied().fold(f64::INFINITY, f64::min)
    );

    let held_files: Vec<&Under> = files
        .iter()
        .filter(|(_, g, h, _)| *h && zero_channel(g))
        .map(|f| &f.3)
        .collect();
    let train_files: Vec<&Under> = files
        .iter()
        .filter(|(_, g, h, _)| !*h && zero_channel(g))
        .map(|f| &f.3)
        .collect();
    let held = held_out_check(&held_files, &train_files, &amap, &fit, logo);
    let _ = writeln!(
        s,
        "D240's residual, channels with Ô ≤ {O_ZERO}, sat-* and black, α > {L_ALPHA}:"
    );
    let _ = writeln!(
        s,
        "  held out ({} file(s)), today:     {}",
        held_files.len(),
        held.today_held
    );
    let _ = writeln!(s, "  held out, regressed:          {}", held.reg_held);
    let _ = writeln!(
        s,
        "  training ({} file(s)), today:     {}",
        train_files.len(),
        held.today_train
    );
    let _ = writeln!(s, "  training, regressed:          {}", held.reg_train);
    let _ = writeln!(s, "  held-out check: {}", held.verdict);

    let alpha_same = alpha_p95 <= ALPHA_SAME;
    let l_same = l_p95.iter().all(|v| *v <= L_SAME);
    let along = (0..3).any(|c| l_p95[c] > L_SAME && corr[c].abs() >= ALONG_LOGO);
    let o_spread = quantile(&ostd, 0.5);
    let reading = match (alpha_same, l_same) {
        _ if o_spread.is_nan() || o_spread < MIN_O_SPREAD => {
            "not readable — Ô barely moves between the files (median sd under the minimum): one background cannot tell α from L"
        }
        (true, true) => "row 1 of §4.4 — the map is right and L is global: look at the blend model",
        (true, false) if along => "row 2 of §4.4 — L departs along the logo with α right: a per-pixel L",
        (false, true) => "row 3 of §4.4 — α departs, L is global: re-fit the map by regression",
        (false, false) if r2_median < R2_LOW => "row 4 of §4.4 — both depart and R² is low: not linear in stored codes",
        _ => "none of §4.4's rows by its own words — read the figures",
    };
    let _ = writeln!(s, "mechanical reading: {reading}");
    Analysis {
        fit,
        summary: s,
        held,
    }
}

fn read_list(path: &str) -> Vec<(String, String, bool)> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let base = Path::new(path).parent().unwrap_or(Path::new("."));
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#') && !l.starts_with("path\t"))
        .map(|l| {
            let cols: Vec<&str> = l.split('\t').collect();
            let file = cols.first().copied().unwrap_or_default();
            let file = if Path::new(file).is_absolute() {
                file.to_string()
            } else {
                base.join(file).to_string_lossy().into_owned()
            };
            let group = cols.get(1).copied().unwrap_or_default().to_string();
            let held = matches!(cols.get(2).copied(), Some("true" | "1" | "yes"));
            (file, group, held)
        })
        .collect()
}

fn read_raster(path: &str) -> Result<Raster, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let container = wipemark_image::inspect(&bytes)
        .map_err(|e| format!("{path}: {e}"))?
        .container;
    match decode(&bytes, container) {
        Ok(Ok(d)) => Ok(d.raster),
        Ok(Err(skip)) => Err(format!("{path}: not examined ({skip:?})")),
        Err(e) => Err(format!("{path}: {e}")),
    }
}

fn profile_of<'a>(catalogue: &'a Catalogue, id: &str) -> Result<&'a Profile, String> {
    catalogue
        .profile(id)
        .ok_or_else(|| format!("no profile {id}"))
}

fn sha256(bytes: &[u8]) -> String {
    let mut hex = String::new();
    for b in Sha256::digest(bytes) {
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

fn run(profile: &str, row: usize, list: &str, out: &str, ring: u32) -> Result<(), String> {
    let catalogue = Catalogue::shipped().map_err(|e| e.to_string())?;
    let profile = profile_of(catalogue, profile)?;
    let map = profile.map(profile.placements.get(row).ok_or("no such row")?.alpha);
    let logo = profile.logo.map(f64::from);
    let mut files = Vec::new();
    let mut skipped = 0;
    for (path, group, held) in read_list(list) {
        let raster = match read_raster(&path) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{e}");
                skipped += 1;
                continue;
            }
        };
        let rect = match row_rect(profile, row, raster.width(), raster.height()) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{path}: {e}");
                skipped += 1;
                continue;
            }
        };
        match under(&raster, rect, ring) {
            Ok(u) => {
                eprintln!("{path}: {group}{}", if held { ", held out" } else { "" });
                files.push((path, group, held, u));
            }
            Err(e) => {
                eprintln!("{path}: {e}");
                skipped += 1;
            }
        }
    }
    let a = analyse(&files, map, logo);
    std::fs::create_dir_all(out).map_err(|e| format!("{out}: {e}"))?;
    let values: Vec<f32> = a
        .fit
        .iter()
        .map(|p| p.alpha.unwrap_or(0.0).clamp(0.0, 1.0) as f32)
        .collect();
    let wma = AlphaMap::new(map.width(), map.height(), values)
        .map_err(|e| e.to_string())?
        .write(16)
        .map_err(|e| e.to_string())?;
    let wma_path = Path::new(out).join("alpha_reg.wma");
    std::fs::write(&wma_path, &wma).map_err(|e| e.to_string())?;
    let mut tsv = String::from(
        "x\ty\talpha_reg\talpha_map\td_alpha\tL_r\tL_g\tL_b\tr2\tn\to_sd\to_min\to_max\n",
    );
    let opt = |v: Option<f64>, d: usize| v.map_or(String::new(), |v| format!("{v:.d$}"));
    for (p, px) in a.fit.iter().enumerate() {
        let (x, y) = (p as u32 % map.width(), p as u32 / map.width());
        let am = f64::from(map.values()[p]);
        let _ = writeln!(
            tsv,
            "{x}\t{y}\t{}\t{am:.5}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.2}\t{:.2}\t{:.2}",
            opt(px.alpha, 5),
            opt(px.alpha.map(|v| v - am), 5),
            opt(px.l[0], 3),
            opt(px.l[1], 3),
            opt(px.l[2], 3),
            opt(px.r2, 5),
            px.n,
            px.o_std,
            px.o_min,
            px.o_max
        );
    }
    std::fs::write(Path::new(out).join("pixels.tsv"), tsv).map_err(|e| e.to_string())?;
    let mut summary = format!(
        "{} row {row}, ring {ring} px: {} file(s) read, {skipped} skipped\n",
        profile.id,
        files.len()
    );
    summary += &a.summary;
    let _ = writeln!(
        summary,
        "{}: {}×{}, depth 16, sha256 {}",
        wma_path.display(),
        map.width(),
        map.height(),
        sha256(&wma)
    );
    std::fs::write(Path::new(out).join("summary.txt"), &summary).map_err(|e| e.to_string())?;
    print!("{summary}");
    Ok(())
}

fn background(path: &str, profile: &str, row: usize, ring: u32, out: &str) -> Result<(), String> {
    let catalogue = Catalogue::shipped().map_err(|e| e.to_string())?;
    let profile = profile_of(catalogue, profile)?;
    let raster = read_raster(path)?;
    let rect = row_rect(profile, row, raster.width(), raster.height())?;
    let o = ring_background(&raster, rect, ring).ok_or("no ring around the mark")?;
    let mut tsv = format!(
        "# Ô under {} row {row} of {path}: rect {} {} {}×{}, ring {ring} px, 8-bit units\nx\ty\tR\tG\tB\n",
        profile.id, rect.x, rect.y, rect.width, rect.height
    );
    for (p, v) in o.iter().enumerate() {
        let (x, y) = (
            rect.x + p as u32 % rect.width,
            rect.y + p as u32 / rect.width,
        );
        let _ = writeln!(tsv, "{x}\t{y}\t{:.9}\t{:.9}\t{:.9}", v[0], v[1], v[2]);
    }
    std::fs::write(out, tsv).map_err(|e| format!("{out}: {e}"))?;
    println!("{out}: {} pixels", o.len());
    Ok(())
}

// ------------------------------------------------------------ selftest

/// A `w × h` 16-bit picture of one colour (8-bit units) with `map` blended
/// at `rect` in light of the per-pixel logo `logo(p)`, in `f64`, rounded
/// once.
fn synthetic(
    w: u32,
    h: u32,
    colour: [f64; 3],
    map: &AlphaMap,
    rect: PixelRect,
    logo: &dyn Fn(usize) -> [f64; 3],
) -> Raster {
    let k = 65535.0 / 255.0;
    let mut s = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        for x in 0..w {
            let inside =
                x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height;
            let (a, l) = if inside {
                let p = ((y - rect.y) * rect.width + (x - rect.x)) as usize;
                (f64::from(map.values()[p]), logo(p))
            } else {
                (0.0, [0.0; 3])
            };
            for c in 0..3 {
                let v = a * l[c] + (1.0 - a) * colour[c];
                s.push((v * k).round().clamp(0.0, 65535.0) as u16);
            }
        }
    }
    Raster::new(w, h, Layout::Rgb16, s).expect("a raster")
}

fn selftest() -> Result<(), String> {
    let catalogue = Catalogue::shipped().map_err(|e| e.to_string())?;
    let profile = profile_of(catalogue, "gemini-sparkle-v1")?;
    let map = profile.map(profile.placements[0].alpha);
    let global = profile.logo.map(f64::from);
    let peak = f64::from(map.values().iter().copied().fold(0.0f32, f32::max));
    // A logo that departs along its own shape: red 5 levels darker and
    // blue 3 at the mark's peak, green global.
    let truth = |p: usize| {
        let a = f64::from(map.values()[p]) / peak;
        [global[0] - 5.0 * a, global[1], global[2] - 3.0 * a]
    };
    let (w, h) = (1100u32, 1100u32);
    let rect = row_rect(profile, 0, w, h)?;
    let mut failures = Vec::new();
    let mut check = |ok: bool, what: String| {
        println!("{} {what}", if ok { "ok  " } else { "FAIL" });
        if !ok {
            failures.push(what);
        }
    };
    let make = |colours: &[[f64; 3]], group: &str, held: bool| -> Result<Vec<_>, String> {
        colours
            .iter()
            .map(|c| {
                let r = synthetic(w, h, *c, map, rect, &truth);
                Ok((
                    format!("{c:?}"),
                    group.to_string(),
                    held,
                    under(&r, rect, RING)?,
                ))
            })
            .collect()
    };
    let greys = [[0.0; 3], [128.0; 3], [255.0; 3]];
    let blue_zero = [[0.0, 0.0, 0.0], [128.0, 128.0, 0.0], [255.0, 255.0, 0.0]];
    for (name, colours, group) in [
        ("grey 0, 128, 255", &greys, "gray-50"),
        (
            "0, 128, 255 in red and green, blue at 0",
            &blue_zero,
            "sat-blue-zero",
        ),
    ] {
        let files = make(colours, group, false)?;
        let a = analyse(&files, map, global);
        let (mut worst_a, mut worst_l, mut undefined) = (0.0f64, 0.0f64, 0);
        for (p, px) in a.fit.iter().enumerate() {
            let at = f64::from(map.values()[p]);
            if at <= 0.0 {
                continue;
            }
            let Some(alpha) = px.alpha else {
                undefined += 1;
                continue;
            };
            worst_a = worst_a.max((alpha - at).abs());
            if at > L_ALPHA {
                for (l, t) in px.l.iter().zip(truth(p)) {
                    match l {
                        Some(l) => worst_l = worst_l.max((l - t).abs()),
                        None => worst_l = f64::INFINITY,
                    }
                }
            }
        }
        check(
            undefined == 0 && worst_a <= 1.0 / 255.0,
            format!(
                "{name}: α within 1/255 wherever it is above 0 — largest |Δα| {worst_a:.6} ({:.3}/255), {undefined} undefined",
                worst_a * 255.0
            ),
        );
        check(
            worst_l <= 1.0,
            format!("{name}: L within a level where α > {L_ALPHA} — largest |ΔL| {worst_l:.4}"),
        );
    }

    // The held-out check: trained on both sets above, held out on a black
    // and a saturated picture it did not see.
    let mut files = make(&greys, "gray-50", false)?;
    files.extend(make(&blue_zero, "sat-blue", false)?);
    files.extend(make(&[[0.0; 3], [0.0, 180.0, 0.0]], "sat-red", true)?);
    let a = analyse(&files, map, global);
    check(
        a.held.reg_held.n > 0 && a.held.reg_held.max <= HELD_OUT_BOUND,
        format!(
            "held out: the regression's residual is within {HELD_OUT_BOUND} — {}",
            a.held.reg_held
        ),
    );
    check(
        a.held.today_held.max > HELD_OUT_BOUND && a.held.verdict.starts_with("accepted"),
        format!(
            "held out: today's model is not, and the check accepts — today {}; {}",
            a.held.today_held, a.held.verdict
        ),
    );
    check(
        a.summary.contains("row 2 of §4.4"),
        "an L departing along the logo with α right reads as §4.4's row 2".to_string(),
    );
    // One background: no reading, whatever the fit says.
    let one = make(
        &[[0.0, 150.0, 56.0], [1.0, 151.0, 57.0]],
        "sat-green",
        false,
    )?;
    let a = analyse(&one, map, global);
    check(
        a.summary.contains("mechanical reading: not readable"),
        "one background is not readable".to_string(),
    );
    if failures.is_empty() {
        println!("selftest: all passed");
        Ok(())
    } else {
        Err(format!("selftest: {} failed", failures.len()))
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--selftest") {
        if let Err(e) = selftest() {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    let value = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let usage = || {
        eprintln!(
            "map_regress --profile <id> --row <i> --list <list.tsv> --out <dir> [--ring {RING}]\n\
             map_regress --background <picture> --profile <id> --row <i> [--ring {RING}] --out <ohat.tsv>\n\
             map_regress --selftest"
        );
        std::process::exit(2);
    };
    let (Some(profile), Some(row), Some(out)) =
        (value("--profile"), value("--row"), value("--out"))
    else {
        usage()
    };
    let Ok(row) = row.parse::<usize>() else {
        usage()
    };
    let ring = value("--ring").map_or(RING, |r| r.parse().unwrap_or_else(|_| usage()));
    let result = if let Some(picture) = value("--background") {
        background(&picture, &profile, row, ring, &out)
    } else if let Some(list) = value("--list") {
        run(&profile, row, &list, &out, ring)
    } else {
        usage()
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
