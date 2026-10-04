//! The second proof (D154), independent of the correlation that proposed
//! the place: **edge energy along the mark's contour**.
//!
//! `E(img) = Σ |∇luma(img)| · |∇α|` over the template's rectangle and a
//! one-pixel ring, central differences, wherever `|∇α| > 1e-4`. A true
//! mark is the one hypothesis whose inverse makes its own edges vanish:
//! sweep a gain `k` from 0 to 1.6 in steps of 0.02, invert with `k·α`
//! **unclamped**, and accept only when
//!
//! * `k* = argmin E` is within `gain` of 1 (a mark at another opacity, an
//!   opaque look-alike and an unmarked texture all land elsewhere);
//! * `E(1)/E(0)` is at most `edge_ratio`;
//! * the share of stored values that lie outside what a blend with this
//!   map and logo could produce, by more than [`BLEND_LEVELS`], is at most
//!   `out_of_range` — an inverse that has to leave the range to cancel an
//!   edge is explaining something that is not a blend.
//!
//! Before any of those, a proposal that no gain takes a fifth of the
//! contour away from, or whose contour grows when it is inverted at the
//! mark's own opacity, is **no blend** and not a finding at all
//! ([`NO_BLEND_RATIO`], D235).
//!
//! Only this module constructs a [`Verified`], and only a `Verified` can
//! be restored: the type system keeps "write only what was proved".

use serde::Serialize;

use crate::catalogue::Profile;
use crate::geometry::{template_and_noise, template_with, Kernel, PixelRect, SubRect};
use crate::propose::Proposal;
use crate::raster::{Raster, LUMA};

/// Below this, a map sample is noise and not part of the mark.
pub const NOISE_FLOOR: f32 = 0.002;
/// The edge weight under which a pixel is not on the contour.
const EDGE: f32 = 1e-4;
/// The sweep: `k = i / 50` for `i` in `0..=80` — 0 to 1.6 by 0.02, with
/// `k = 1` exactly at `i = 50`.
const STEPS: usize = 80;
const ONE: usize = 50;

/// Why a proposal was not accepted. Each carries the number that failed;
/// [`Scores`] beside it carries all of them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "why", rename_all = "kebab-case")]
pub enum Refusal {
    /// The picture's own alpha is not opaque under the mark: what the
    /// blend meant there is unknown (D157).
    Transparent,
    /// Every pixel of the mark is at or above the opaque threshold:
    /// nothing can be restored, only reconstructed.
    Opaque { holes: u32 },
    /// The edges vanish at another gain: another opacity, or no blend.
    Gain { k: f32 },
    /// The inverse does not take the contour away.
    Edges { ratio: f32 },
    /// The inverse leaves the range to do it.
    OutOfRange { share: f32 },
}

/// Every number the second proof measured.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Scores {
    /// `k* = argmin E`.
    pub gain: f32,
    /// `E(1)/E(0)`.
    pub edge_ratio: f32,
    /// The share of samples out of range at `k = 1`.
    pub out_of_range: f32,
    /// Pixels at or above the opaque threshold: never divided.
    pub holes: u32,
}

/// A proposal that passed both proofs — the only thing [`crate::restore`]
/// accepts. Its fields are private and it is built in this module alone.
///
/// Nothing outside this crate can reach inside one — let alone build
/// one (`only_a_verified_hypothesis_can_be_restored`):
///
/// ```compile_fail
/// fn forge(v: &mut wipemark_pixels::Verified) {
///     v.values = vec![0.5; 4];
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Verified {
    profile: String,
    rect: SubRect,
    at: PixelRect,
    values: Vec<f32>,
    /// The logo per channel, in the raster's stored units.
    logo: [f64; 3],
    opaque_above: f32,
    gain: f32,
    edge_ratio: f32,
    holes: u32,
    /// A canonical map at a row's own place: the one placement that can
    /// be exact.
    exact_place: bool,
    /// The map was fitted from real outputs (D245): never exact.
    fitted: bool,
    /// The capture noise the template dropped, per pixel (D241, D246).
    noise: Vec<f32>,
    /// The raster's dimensions, so a restore onto another raster is
    /// refused.
    width: u32,
    height: u32,
    /// `E(0)`: the contour energy the mark had before it was restored —
    /// what [`outline`] measures what is left against.
    contour: f64,
}

impl Verified {
    pub fn profile(&self) -> &str {
        &self.profile
    }

    pub fn rect(&self) -> SubRect {
        self.rect
    }

    pub fn pixels(&self) -> PixelRect {
        self.at
    }

    pub fn gain(&self) -> f32 {
        self.gain
    }

    pub fn edge_ratio(&self) -> f32 {
        self.edge_ratio
    }

    pub fn holes(&self) -> u32 {
        self.holes
    }

    pub(crate) fn values(&self) -> &[f32] {
        &self.values
    }

    pub(crate) fn logo(&self) -> [f64; 3] {
        self.logo
    }

    pub(crate) fn opaque_above(&self) -> f32 {
        self.opaque_above
    }

    pub(crate) fn exact_place(&self) -> bool {
        self.exact_place
    }

    pub(crate) fn fitted(&self) -> bool {
        self.fitted
    }

    pub(crate) fn noise(&self) -> &[f32] {
        &self.noise
    }

    pub(crate) fn fits(&self, raster: &Raster) -> bool {
        raster.width() == self.width && raster.height() == self.height
    }
}

/// What a proposal turned out to be (D235). Three outcomes, not two:
///
/// * **proved** — both proofs passed; restorable;
/// * **a blend that is not proved** — some gain takes the edge away
///   (`E(k*)/E(0) ≤` [`NO_BLEND_RATIO`]) and the mark's own does not add
///   to it, but another gain than the mark's, not far enough, or by
///   leaving the range: a mark like this was seen and is not removed, and
///   that is a finding;
/// * **no blend** — no gain takes even a fifth of the contour away, or
///   inverting at the mark's own opacity adds contour (`E(1) > E(0)`).
///   Blended at a gain `g`, the mark leaves `E(1)/E(0) ≈ |1 − g| /
///   (g·(1 − α))`, which is over 1 exactly when `g < 1/(2 − α)` — under
///   two thirds at `α ≈ 0.5`, and never at a half or more for any `α`: so
///   "the edges vanish only under half the mark's opacity" (`k* ≈ 0`, the
///   textures and opaque look-alikes) needs no rule of its own, and a
///   mutation that removed one left every gate green. Whatever the
///   correlation saw, it is not this mark blended over a picture. That is
///   not a finding — never reported and never an exit code.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Outcome {
    Verified(Verified),
    Refused(Refusal),
    NoBlend,
}

/// Over this `E(k*)/E(0)` the best inverse, at whatever gain, takes less
/// than a fifth of the contour away: no blend of it (D235). A true mark
/// measured at most 0.58 (on the densest glyph sheet), and the mark at
/// another opacity far less at its own `k*`; opaque look-alikes and blends
/// of other shapes run from 0.8 to past 1.
pub const NO_BLEND_RATIO: f32 = 0.8;

/// How far, in stored 8-bit levels, a stored value may lie outside what
/// a blend with the profile's map and logo could produce, and still be
/// that blend (D240). GWT's maps are 8-bit captures of the vendor's α:
/// over a real Gemini output on a saturated green — the original at 0 in
/// two channels — stored values sit up to 6 levels under `α·L`, while an
/// opaque look-alike or a dark picture under the map's soft edge misses
/// by tens. It covers a quality 85–95 codec's error too.
pub const BLEND_LEVELS: f64 = 8.0;

/// The template's rectangle and a one-pixel ring, read once: the map
/// there, the picture there, and the contour weights.
struct Grid {
    gw: usize,
    alpha: Vec<f32>,
    pixels: Vec<[f64; 3]>,
    edges: Vec<(usize, f32)>,
    logo: [f64; 3],
    max: f64,
    opaque: f64,
}

impl Grid {
    fn new(raster: &Raster, values: &[f32], at: PixelRect, opaque: f32, logo: [f64; 3]) -> Self {
        let samples = raster.samples();
        let gx0 = at.x.saturating_sub(1);
        let gy0 = at.y.saturating_sub(1);
        let gx1 = (at.x + at.width + 1).min(raster.width());
        let gy1 = (at.y + at.height + 1).min(raster.height());
        let (gw, gh) = ((gx1 - gx0) as usize, (gy1 - gy0) as usize);
        let mut alpha = vec![0f32; gw * gh];
        let mut pixels = vec![[0f64; 3]; gw * gh];
        for gy in 0..gh {
            for gx in 0..gw {
                let (x, y) = (gx0 + gx as u32, gy0 + gy as u32);
                if x >= at.x && y >= at.y && x < at.x + at.width && y < at.y + at.height {
                    alpha[gy * gw + gx] = values[((y - at.y) * at.width + (x - at.x)) as usize];
                }
                let i = raster.at(x, y);
                pixels[gy * gw + gx] = [
                    f64::from(samples[i]),
                    f64::from(samples[i + 1]),
                    f64::from(samples[i + 2]),
                ];
            }
        }
        // The contour, weighted by |∇α| — but not beside a hole: there the
        // inverse is not defined, and the edge between a hole and what was
        // restored around it says nothing about the blend.
        let hole = |p: usize| alpha[p] >= opaque;
        let mut edges: Vec<(usize, f32)> = Vec::new();
        for gy in 1..gh.saturating_sub(1) {
            for gx in 1..gw.saturating_sub(1) {
                let p = gy * gw + gx;
                if hole(p) || hole(p - 1) || hole(p + 1) || hole(p - gw) || hole(p + gw) {
                    continue;
                }
                let dx = (alpha[p + 1] - alpha[p - 1]) * 0.5;
                let dy = (alpha[p + gw] - alpha[p - gw]) * 0.5;
                let g = dx.hypot(dy);
                if g > EDGE {
                    edges.push((p, g));
                }
            }
        }
        Grid {
            gw,
            alpha,
            pixels,
            edges,
            logo,
            max: f64::from(raster.layout().max()),
            opaque: f64::from(opaque),
        }
    }

    /// `E` of the inverse at gain `k`: the luma gradient along the
    /// contour, weighted by |∇α|. `luma` is scratch of the grid's size.
    fn energy(&self, k: f64, luma: &mut [f64]) -> f64 {
        for (p, l) in luma.iter_mut().enumerate() {
            let o = inverse(
                self.pixels[p],
                f64::from(self.alpha[p]) * k,
                self.logo,
                self.opaque,
            );
            *l =
                (f64::from(LUMA[0]) * o[0] + f64::from(LUMA[1]) * o[1] + f64::from(LUMA[2]) * o[2])
                    / self.max;
        }
        let gw = self.gw;
        self.edges
            .iter()
            .map(|&(p, g)| {
                let dx = (luma[p + 1] - luma[p - 1]) * 0.5;
                let dy = (luma[p + gw] - luma[p - gw]) * 0.5;
                dx.hypot(dy) * f64::from(g)
            })
            .sum()
    }
}

/// What the inverse at the mark's own opacity leaves on the contour, per
/// unit of contour: `E(1)/Σ|∇α|`, the mean luma step left along the edge.
/// The search's refinement minimises it (D236): at the mark's true place
/// and size only the picture's own texture is left, anywhere else an
/// outline is too. (`E(1)/E(0)` is not compared across places: `E(0)`
/// moves with the shape as much as the residual does.) `None` where the
/// template does not fit or has no contour.
pub(crate) fn residual(
    raster: &Raster,
    profile: &Profile,
    map: &crate::alpha::AlphaMap,
    rect: SubRect,
    kernel: Kernel,
) -> Option<f64> {
    let (shape, at) = template_with(map, rect, kernel)?;
    if !at.inside(raster.width(), raster.height()) {
        return None;
    }
    let max = f64::from(raster.layout().max());
    let logo = profile.logo.map(|c| f64::from(c) * max / 255.0);
    let grid = Grid::new(raster, &shape.values, at, profile.opaque_above, logo);
    let weight: f64 = grid.edges.iter().map(|&(_, g)| f64::from(g)).sum();
    let mut luma = vec![0f64; grid.pixels.len()];
    (weight > 0.0).then(|| grid.energy(1.0, &mut luma) / weight)
}

/// The second proof over one proposal: the numbers, and the outcome.
/// `None` for the numbers when the outcome came before any was measured.
pub(crate) fn verify(
    raster: &Raster,
    profile: &Profile,
    proposal: &Proposal,
) -> (Option<Scores>, Outcome) {
    let map = profile.map(proposal.map);
    let Some((shape, at, noise)) = template_and_noise(map, proposal.rect, proposal.kernel) else {
        return (None, Outcome::NoBlend);
    };
    if !at.inside(raster.width(), raster.height()) {
        return (None, Outcome::NoBlend);
    }
    let layout = raster.layout();
    let max = f64::from(layout.max());
    let samples = raster.samples();
    let opaque = profile.opaque_above;

    // Support, holes, and the picture's own alpha under the mark.
    let mut support = 0u32;
    let mut holes = 0u32;
    let mut transparent = false;
    for ty in 0..at.height {
        for tx in 0..at.width {
            let a = shape.values[(ty * at.width + tx) as usize];
            if a < NOISE_FLOOR {
                continue;
            }
            support += 1;
            if a >= opaque {
                holes += 1;
            }
            if layout.has_alpha() {
                let i = raster.at(at.x + tx, at.y + ty);
                transparent |= samples[i + 3] < layout.max();
            }
        }
    }
    // A template with nothing over the noise floor has no mark in it to
    // be a blend of: not a finding. One that is all holes is a mark that
    // can only be reconstructed.
    if support == 0 {
        return (None, Outcome::NoBlend);
    }
    if holes == support {
        return (None, Outcome::Refused(Refusal::Opaque { holes }));
    }

    let logo = profile.logo.map(|c| f64::from(c) * max / 255.0);
    let grid = Grid::new(raster, &shape.values, at, opaque, logo);
    let mut energy = [0f64; STEPS + 1];
    let mut luma = vec![0f64; grid.pixels.len()];
    for (i, e) in energy.iter_mut().enumerate() {
        *e = grid.energy(i as f64 / ONE as f64, &mut luma);
    }
    let star = (0..=STEPS)
        .min_by(|&a, &b| energy[a].total_cmp(&energy[b]))
        .unwrap_or(0);
    let gain = star as f32 / ONE as f32;
    // No edge at all where the mark has one: there is no mark here.
    let (edge_ratio, best_ratio) = if energy[0] > 1e-12 {
        (
            (energy[ONE] / energy[0]) as f32,
            (energy[star] / energy[0]) as f32,
        )
    } else {
        (1.0, 1.0)
    };

    // Out of range at k = 1, over the restorable support, measured where
    // the evidence is — in stored levels: how far a stored value lies
    // outside what a blend with this map and logo could have produced
    // over any original, `[α·L, α·L + (1 − α)·max]` (D240). The inverse's
    // own excess is that gap amplified by `1/(1 − α)`. A blend allows
    // `BLEND_LEVELS` — the vendor's α against an 8-bit capture of it, and
    // a quality 85–95 codec's error with it: the separate lossy allowance
    // of D237 is folded in, no test could tell it apart any more.
    let allowance = BLEND_LEVELS * max / 255.0;
    let (mut out, mut total) = (0u32, 0u32);
    for (p, &a) in grid.alpha.iter().enumerate() {
        if a < NOISE_FLOOR || a >= opaque {
            continue;
        }
        let a = f64::from(a);
        for v in inverse(grid.pixels[p], a, logo, f64::from(opaque)) {
            total += 1;
            let gap = if v < 0.0 {
                -v * (1.0 - a)
            } else if v > max {
                (v - max) * (1.0 - a)
            } else {
                0.0
            };
            if gap > allowance {
                out += 1;
            }
        }
    }
    let out_of_range = if total == 0 {
        0.0
    } else {
        out as f32 / total as f32
    };

    let scores = Scores {
        gain,
        edge_ratio,
        out_of_range,
        holes,
    };
    let t = profile.thresholds;
    let outcome = if best_ratio > NO_BLEND_RATIO || edge_ratio > 1.0 {
        Outcome::NoBlend
    } else if transparent {
        // A blend under pixels the picture's own alpha does not make
        // opaque: what it meant there is unknown (D157). Asked only after
        // the blend itself — a cut-out sticker's confetti under its
        // transparent corner is no blend, and is no finding (D235).
        Outcome::Refused(Refusal::Transparent)
    } else if (gain - 1.0).abs() > t.gain {
        Outcome::Refused(Refusal::Gain { k: gain })
    } else if edge_ratio > t.edge_ratio {
        Outcome::Refused(Refusal::Edges { ratio: edge_ratio })
    } else if out_of_range > t.out_of_range {
        Outcome::Refused(Refusal::OutOfRange {
            share: out_of_range,
        })
    } else {
        let exact_place = matches!(proposal.placed, crate::Placed::Row(_))
            && !proposal.resample
            && shape.canonical;
        Outcome::Verified(Verified {
            profile: profile.id.clone(),
            rect: proposal.rect,
            at,
            values: shape.values,
            logo,
            opaque_above: opaque,
            gain,
            edge_ratio,
            holes,
            exact_place,
            fitted: profile.fitted.get(proposal.map).copied().unwrap_or(false),
            noise,
            width: raster.width(),
            height: raster.height(),
            contour: energy[0],
        })
    };
    (Some(scores), outcome)
}

/// Over this, an outline is left (D238): a fifth of the mark's own
/// contour energy still on the contour after it was restored, beyond what
/// the picture's texture around it accounts for. See [`outline`].
/// Measured on a 96-pixel mark shrunk with its picture to 35 pixels by
/// Lanczos, bilinear or Catmull-Rom and saved as JPEG: 0.03–0.09 at
/// quality 95, 0.15–0.19 at 85 (the codec's ringing on the mark's edges,
/// doubled by the inverse); the outlines the host verifier saw measured
/// 0.23–0.25 before restoration.
pub const OUTLINE_BOUND: f32 = 0.20;

/// The mark's faint band: where a map that is a level or two off the
/// vendor's α leaves its outline (D243, D244).
pub const BAND: [f32; 2] = [3.0 / 255.0, 0.2];

/// Over this many 8-bit luma levels between the faint band and the
/// picture around the mark, and over that picture's own spread, an
/// outline is left (D244) — whatever share of the mark's contour that is.
/// Measured on 22 first-generation Gemini outputs restored at their row:
/// −0.17 to +0.30. Left by GWT's capture map in the search: −1.84 to
/// −1.93; on the re-saved `11_crying`, over a background of no spread at
/// all: −3.67.
pub const STEP_LEVELS: f32 = 1.0;

/// What a restoration left along the mark's contour, two ways (D238,
/// D244).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Outline {
    /// The contour's energy on the restored raster, less what the texture
    /// around the mark would put there, as a share of the energy the mark
    /// had before (D238). Relative: on a flat picture a ring of twenty
    /// levels is a small share of a mark of a hundred.
    pub share: f32,
    /// The faint band's mean luma less the mean of the pixels around the
    /// mark, in 8-bit levels: what an eye compares on a flat picture.
    pub step: f32,
    /// The standard deviation of those pixels around the mark, in 8-bit
    /// levels: how much a step can hide in.
    pub spread: f32,
}

impl Outline {
    /// A share over [`OUTLINE_BOUND`], or a step over both
    /// [`STEP_LEVELS`] and the picture's own spread.
    pub fn left(&self) -> bool {
        self.share > OUTLINE_BOUND || self.step.abs() > STEP_LEVELS.max(self.spread)
    }
}

/// What a restoration left along the mark's contour (D238, D244). The
/// share: the contour's energy on the restored raster, less what the
/// texture around the mark would put there — the mean luma gradient over
/// a band two to eight pixels outside the rectangle, times the contour's
/// weight — as a share of the energy the mark had before. 0 for a
/// restoration that left the contour as busy as its surroundings; a dark
/// or light ring left by a map at the wrong size, place or filter is a
/// share of what was there. The step: see [`Outline::step`].
pub(crate) fn outline(raster: &Raster, verified: &Verified) -> Outline {
    let (step, spread) = step(raster, verified);
    let share = if verified.contour <= 1e-12 {
        0.0
    } else {
        let grid = Grid::new(
            raster,
            &verified.values,
            verified.at,
            verified.opaque_above,
            verified.logo,
        );
        let weight: f64 = grid.edges.iter().map(|&(_, g)| f64::from(g)).sum();
        let mut luma = vec![0f64; grid.pixels.len()];
        let after = grid.energy(0.0, &mut luma);
        let texture = texture_around(raster, verified.at);
        ((after - weight * texture).max(0.0) / verified.contour) as f32
    };
    Outline {
        share,
        step,
        spread,
    }
}

/// The faint band's step and the spread around it, in 8-bit luma levels:
/// the band is the template's pixels with `α` in [`BAND`]; around it is
/// every pixel of the rectangle under the noise floor and of a ring four
/// pixels out, inside the picture. `(0, 0)` with either empty.
fn step(raster: &Raster, verified: &Verified) -> (f32, f32) {
    let (w, h) = (i64::from(raster.width()), i64::from(raster.height()));
    let max = f64::from(raster.layout().max());
    let samples = raster.samples();
    let luma = |x: i64, y: i64| {
        let i = raster.at(x as u32, y as u32);
        (f64::from(LUMA[0]) * f64::from(samples[i])
            + f64::from(LUMA[1]) * f64::from(samples[i + 1])
            + f64::from(LUMA[2]) * f64::from(samples[i + 2]))
            * 255.0
            / max
    };
    let at = verified.at;
    let (x0, y0) = (i64::from(at.x), i64::from(at.y));
    let (x1, y1) = (x0 + i64::from(at.width), y0 + i64::from(at.height));
    let (mut band, mut nb) = (0f64, 0f64);
    let mut around = Vec::new();
    for y in (y0 - 4).max(0)..(y1 + 4).min(h) {
        for x in (x0 - 4).max(0)..(x1 + 4).min(w) {
            let inside = x >= x0 && y >= y0 && x < x1 && y < y1;
            let a = if inside {
                verified.values[((y - y0) * i64::from(at.width) + (x - x0)) as usize]
            } else {
                0.0
            };
            if a < NOISE_FLOOR {
                around.push(luma(x, y));
            } else if (BAND[0]..=BAND[1]).contains(&a) {
                band += luma(x, y);
                nb += 1.0;
            }
        }
    }
    if nb == 0.0 || around.is_empty() {
        return (0.0, 0.0);
    }
    let n = around.len() as f64;
    let mean = around.iter().sum::<f64>() / n;
    let spread = (around.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n).sqrt();
    ((band / nb - mean) as f32, spread as f32)
}

/// The mean luma gradient (central differences, luma in [0, 1]) over the
/// pixels two to eight outside `at`, inside the picture.
fn texture_around(raster: &Raster, at: PixelRect) -> f64 {
    let (w, h) = (i64::from(raster.width()), i64::from(raster.height()));
    let max = f64::from(raster.layout().max());
    let samples = raster.samples();
    let luma = |x: i64, y: i64| {
        let i = raster.at(x as u32, y as u32);
        (f64::from(LUMA[0]) * f64::from(samples[i])
            + f64::from(LUMA[1]) * f64::from(samples[i + 1])
            + f64::from(LUMA[2]) * f64::from(samples[i + 2]))
            / max
    };
    let (x0, y0) = (i64::from(at.x), i64::from(at.y));
    let (x1, y1) = (x0 + i64::from(at.width), y0 + i64::from(at.height));
    let (mut sum, mut n) = (0f64, 0f64);
    for y in (y0 - 8).max(1)..(y1 + 8).min(h - 1) {
        for x in (x0 - 8).max(1)..(x1 + 8).min(w - 1) {
            let outside = (x0 - x).max(x - (x1 - 1)).max(y0 - y).max(y - (y1 - 1));
            if !(2..=8).contains(&outside) {
                continue;
            }
            let dx = (luma(x + 1, y) - luma(x - 1, y)) * 0.5;
            let dy = (luma(x, y + 1) - luma(x, y - 1)) * 0.5;
            sum += dx.hypot(dy);
            n += 1.0;
        }
    }
    if n > 0.0 {
        sum / n
    } else {
        0.0
    }
}

/// [`crate::restore::unblend`] — GWT's reverse blend, written once —
/// unclamped; the input itself where `a` is at or above the opaque
/// threshold, which is never divided.
fn inverse(i: [f64; 3], a: f64, logo: [f64; 3], opaque: f64) -> [f64; 3] {
    if a <= 0.0 || a >= opaque {
        return i;
    }
    crate::restore::unblend(i, a, logo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raster::Layout;
    use crate::{restore, ExamineOptions};

    /// A one-pixel proof over a stored value, the logo white, `α` given.
    fn one(alpha: f32) -> Verified {
        Verified {
            profile: String::from("test"),
            rect: SubRect {
                x: 0.0,
                y: 0.0,
                size: 1.0,
            },
            at: PixelRect {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
            values: vec![alpha],
            logo: [255.0; 3],
            opaque_above: 0.95,
            gain: 1.0,
            edge_ratio: 0.0,
            holes: 0,
            exact_place: true,
            fitted: false,
            noise: vec![0.0],
            width: 1,
            height: 1,
            contour: 0.0,
        }
    }

    /// A template with no sample over the noise floor — a map that is all
    /// noise at that place — is no blend: not a finding, never an
    /// `Opaque` refusal with nothing behind it.
    #[test]
    fn a_template_with_no_support_is_no_blend() {
        let shipped = crate::Catalogue::shipped().unwrap();
        let mut profile = shipped.profile("gemini-sparkle-v1").unwrap().clone();
        profile.maps[0].1 = crate::AlphaMap::new(48, 48, vec![0.001; 48 * 48]).unwrap();
        let raster = Raster::from_u8(640, 480, Layout::Rgb8, &vec![100; 640 * 480 * 3]).unwrap();
        let proposal = Proposal {
            map: 0,
            rect: SubRect {
                x: 560.0,
                y: 400.0,
                size: 48.0,
            },
            placed: crate::Placed::Row(1),
            ncc: 0.9,
            resample: false,
            kernel: Kernel::Area,
        };
        assert_eq!(
            verify(&raster, &profile, &proposal),
            (None, Outcome::NoBlend)
        );
    }

    /// The inverse is rounded to the nearest level, as GWT's is — not
    /// truncated: `(189 − 0.3·255)/0.7` is 160.71 and comes back 161;
    /// `(188 − 0.3·255)/0.7` is 159.29 and comes back 159.
    #[test]
    fn the_inverse_rounds_to_the_nearest_level() {
        for (stored, original) in [(189u8, 161u16), (188, 159)] {
            let mut raster = Raster::from_u8(1, 1, Layout::Rgb8, &[stored; 3]).unwrap();
            restore(&mut raster, &one(0.3), &ExamineOptions::default()).unwrap();
            assert_eq!(raster.samples(), &[original; 3], "{stored}");
        }
    }
}
