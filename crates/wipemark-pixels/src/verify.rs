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
    /// The share of samples out of range at `k = 1` — on the planar path
    /// (D306), the share of pixels whose Y or chroma block is.
    pub out_of_range: f32,
    /// Pixels at or above the opaque threshold: never divided.
    pub holes: u32,
    /// On the planar path (D306), the share's two terms; `None`, and not
    /// in the JSON, on the RGB path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planar: Option<crate::planar::PlanarScores>,
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
    /// The map was drawn at another size or a sub-pixel offset, or the
    /// row asks for a resample: not the map as it was captured.
    resampled: bool,
    /// Placed by the search, not at a row's own place.
    searched: bool,
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

    /// A canonical map at a row's own place: the one placement that can
    /// be exact.
    pub(crate) fn exact_place(&self) -> bool {
        !self.resampled && !self.searched
    }

    pub(crate) fn resampled(&self) -> bool {
        self.resampled
    }

    pub(crate) fn searched(&self) -> bool {
        self.searched
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
#[cfg(test)]
pub(crate) fn verify(
    raster: &Raster,
    profile: &Profile,
    proposal: &Proposal,
) -> (Option<Scores>, Outcome) {
    verify_with(raster, None, profile, proposal)
}

/// [`verify`], with the out-of-range share measured in the planes when
/// `model` is the planar path's (D306).
pub(crate) fn verify_with(
    raster: &Raster,
    model: Option<&crate::planar::Model<'_>>,
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
    // In the planes the file stored, when they are known and subsampled
    // (D306): the share the decision uses, and its two terms.
    let (out_of_range, planar) = match model {
        Some(m) => {
            let (share, terms) = m.out_of_range(&shape.values, at, logo, opaque);
            (share, Some(terms))
        }
        None => (out_of_range, None),
    };

    let scores = Scores {
        gain,
        edge_ratio,
        out_of_range,
        holes,
        planar,
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
        let resampled = proposal.resample || !shape.canonical;
        let searched = !matches!(proposal.placed, crate::Placed::Row(_));
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
            resampled,
            searched,
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
/// Measured on 21 first-generation Gemini outputs over their flat greens,
/// restored at their row: −0.17 to +0.30. Left by GWT's capture map in
/// the search: −1.84 to −1.93; on the re-saved `11_crying`, over a
/// background of no spread at all: −3.67.
pub const STEP_LEVELS: f32 = 1.0;

/// Over this many 8-bit levels of colour difference — the faint band's
/// step in BT.601 `Cb` and `Cr`, as one distance — and over the spread of
/// that colour around the mark, an outline is left too (D247). A JPEG
/// keeps BT.601 luma and puts its error into colour; subsampled 4:2:0,
/// the sparkle's white bleeds into the band, and the fringe the inverse
/// leaves — red and blue up, green down, plain at ×4 — is a step of
/// −0.5 to +0.3 in luma. Measured on the vendor's 2048 × 2048 files,
/// `11_crying` aside (a re-saved copy, said by its luma), saved by Pillow
/// 12.3.0: 0.11–0.46 as handed out; 2.41–2.88 at JPEG 4:4:4 95 (nothing
/// an eye finds in the mean); 7.40–8.37 at 4:2:0 95 and 7.54–8.26 at 98.
/// The bound is 1.39 times the highest under it, and the lowest over it
/// 1.85 times the bound.
pub const CHROMA_LEVELS: f32 = 4.0;

/// Over this many 8-bit levels of roughness on the pixels a restoration
/// changed, and over [`TEXTURE_RATIO`] times the roughness of the picture
/// around the mark, a texture is left (D250). Roughness is the 95th
/// percentile of each pixel's distance in `(Y, Cb, Cr)` from the mean of
/// its eight neighbours: a JPEG's error amplified by the inverse's
/// `1/(1 − α)` comes back as an 8 × 8 checker along the mark's contour
/// that no mean of the band sees. Measured on the vendor's 2048 × 2048
/// files, `11_crying` aside: 1.59–2.05 as handed out, against 1.18–1.88
/// around the mark; saved as JPEG 4:4:4 by Pillow 12.3.0, 8.59–9.22 at
/// 95 (plain at ×2, faint at 1× on the flat green), and on two of them
/// 6.15–6.32 at 97 (faint at ×3), 4.95–5.15 at 98 (barely found at ×6),
/// 3.53–3.55 at 99 (nothing). Only a lossy source is held to it (D251).
pub const TEXTURE_LEVELS: f32 = 5.5;

/// How much rougher than the picture around it a restoration must be for
/// its texture to be left (D250): a picture with a grain of its own keeps
/// it under the mark. Measured as above: 1.06–1.44 as handed out (and
/// 16 for `11_crying` — 2.05 over a background with no grain at all, under
/// [`TEXTURE_LEVELS`]), 2.63–2.95 at JPEG 4:4:4 95, about 2.5 at 97.
pub const TEXTURE_RATIO: f32 = 2.0;

/// Under this many times the roughness of the picture around the mark, a
/// restoration on a lossy source is too smooth: a patch flatter than its
/// surroundings, which is a mark left as plainly as a checker is (D307,
/// E12-R8). The lower bound beside [`TEXTURE_RATIO`]'s upper one: a value
/// chosen inside the codec's interval (`interval.rs`) takes the checker
/// away by smoothing, and smoothing can overshoot. `[tunable]` — the
/// spec's 0.8 (`06-recon-changes.md` §2.4); on today's path every lossy
/// restoration of the committed crops reads 2.62 or more through `clean`
/// and 1.76 or more through the planar inverse (the E12-R8 report). Only
/// a lossy source is held to it (D251's reason).
pub const TEXTURE_RATIO_MIN: f32 = 0.8;

/// What a restoration left along the mark's contour, three ways (D238,
/// D244, D247). Public for [`measure_at`] alone (E12-R12), a developer's
/// measure: the product hands it out only inside [`crate::Restored`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outline {
    /// The contour's energy on the restored raster, less what the texture
    /// around the mark would put there, as a share of the energy the mark
    /// had before (D238). Relative: on a flat picture a ring of twenty
    /// levels is a small share of a mark of a hundred.
    pub share: f32,
    /// Per colour channel — R, G, B — the faint band's mean less the mean
    /// of the pixels around the mark, in 8-bit levels: what an eye
    /// compares on a flat picture.
    pub steps: [f32; 3],
    /// The same step in BT.601 luma (D244).
    pub step: f32,
    /// The standard deviation of the luma around the mark, in 8-bit
    /// levels: how much a step can hide in.
    pub spread: f32,
    /// The same step in BT.601 colour difference, `|(ΔCb, ΔCr)|` (D247).
    pub chroma: f32,
    /// The spread of that colour around the mark, `√(σ²Cb + σ²Cr)`.
    pub chroma_spread: f32,
    /// The roughness of the pixels the restoration changed — `α` at the
    /// noise floor and under the opaque threshold — in 8-bit levels: the
    /// 95th percentile of each one's distance in `(Y, Cb, Cr)` from the
    /// mean of its eight neighbours (D250).
    pub texture: f32,
    /// The same over the pixels around the mark.
    pub texture_around: f32,
}

impl Outline {
    /// A share over [`OUTLINE_BOUND`]; a luma step over both
    /// [`STEP_LEVELS`] and the luma's spread around the mark; or a colour
    /// step over both [`CHROMA_LEVELS`] and the colour's spread there.
    pub fn left(&self) -> bool {
        self.share > OUTLINE_BOUND
            || self.step.abs() > STEP_LEVELS.max(self.spread)
            || self.chroma > CHROMA_LEVELS.max(self.chroma_spread)
    }

    /// A roughness over both [`TEXTURE_LEVELS`] and [`TEXTURE_RATIO`]
    /// times the roughness around the mark (D250).
    pub fn textured(&self) -> bool {
        self.texture > TEXTURE_LEVELS.max(TEXTURE_RATIO * self.texture_around)
    }

    /// A roughness under [`TEXTURE_RATIO_MIN`] times the roughness around
    /// the mark (D307): a patch smoother than the picture around it.
    pub fn smoothed(&self) -> bool {
        self.texture < TEXTURE_RATIO_MIN * self.texture_around
    }
}

/// What a restoration left along the mark's contour (D238, D244). The
/// share: the contour's energy on the restored raster, less what the
/// texture around the mark would put there — the mean luma gradient over
/// a band two to eight pixels outside the rectangle, times the contour's
/// weight — as a share of the energy the mark had before. 0 for a
/// restoration that left the contour as busy as its surroundings; a dark
/// or light ring left by a map at the wrong size, place or filter is a
/// share of what was there. The steps: see [`Outline::steps`].
pub(crate) fn outline(raster: &Raster, verified: &Verified) -> Outline {
    let steps = steps(raster, verified);
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
    Outline { share, ..steps }
}

/// [`outline`]'s measures over a rectangle **nothing was restored in**
/// (E12-R12 §4.1): what a restoration that handed this very picture back
/// would be told — the clean picture's share of the measures every bound
/// is set against. `map` is brought to `rect` by `kernel` as `verify`
/// brings it (the template, its capture noise taken out), and `outline`
/// runs as it runs after a restoration, with two things a restoration
/// would have had standing in:
///
/// * **the contour the mark had** (the share's denominator, `E(0)` before
///   the restoration): the contour energy of this template drawn here with
///   the profile's logo — [`crate::composite`], rounded as a file stores
///   it, over a copy of the rectangle and its one-pixel ring — which is
///   the stored file a perfect restoration would have started from;
/// * **the pixels the restoration changed** (the set `texture` is taken
///   over): the template's support, `α` from [`NOISE_FLOOR`] to under the
///   profile's `opaque_above` — the pixels a restoration of this map would
///   write. It is the set `outline` takes after a real restoration too,
///   which counts the faint ones the inverse rounds back to themselves.
///
/// `None` when the template does not fit `rect` or the picture. A measure
/// for `examples/measure_clean.rs`, never a verdict: the bounds are
/// [`Outline::left`] and [`Outline::textured`], the latter on a lossy
/// source only, as `restore` applies it.
#[doc(hidden)]
pub fn measure_at(
    raster: &Raster,
    profile: &Profile,
    map: &crate::alpha::AlphaMap,
    rect: SubRect,
    kernel: Kernel,
) -> Option<Outline> {
    let (shape, at, noise) = template_and_noise(map, rect, kernel)?;
    if !at.inside(raster.width(), raster.height()) {
        return None;
    }
    let max = f64::from(raster.layout().max());
    let logo = profile.logo.map(|c| f64::from(c) * max / 255.0);
    let opaque = profile.opaque_above;
    // The rectangle and its ring — all `Grid` reads — copied, and the
    // template drawn over the copy.
    let (x0, y0) = (at.x.saturating_sub(1), at.y.saturating_sub(1));
    let x1 = (at.x + at.width + 1).min(raster.width());
    let y1 = (at.y + at.height + 1).min(raster.height());
    let mut samples = Vec::new();
    for y in y0..y1 {
        let row = raster.at(x0, y)..raster.at(x1 - 1, y) + raster.layout().channels();
        samples.extend_from_slice(&raster.samples()[row]);
    }
    let mut drawn = Raster::new(x1 - x0, y1 - y0, raster.layout(), samples).ok()?;
    let local = PixelRect {
        x: at.x - x0,
        y: at.y - y0,
        ..at
    };
    let mark = crate::alpha::AlphaMap::new(
        at.width,
        at.height,
        shape.values.iter().map(|v| v.clamp(0.0, 1.0)).collect(),
    )
    .ok()?;
    crate::restore::composite(&mut drawn, &mark, local, profile.logo);
    let grid = Grid::new(&drawn, &shape.values, local, opaque, logo);
    let contour = grid.energy(0.0, &mut vec![0f64; grid.pixels.len()]);
    // `outline` reads the template, its place, the logo, the opaque
    // threshold and the contour; the rest is what no proof measured.
    let unrestored = Verified {
        profile: profile.id.clone(),
        rect,
        at,
        values: shape.values,
        logo,
        opaque_above: opaque,
        gain: 1.0,
        edge_ratio: 0.0,
        holes: 0,
        resampled: false,
        searched: false,
        fitted: false,
        noise,
        width: raster.width(),
        height: raster.height(),
        contour,
    };
    Some(outline(raster, &unrestored))
}

/// The faint band's step against the picture around it, per channel and
/// in luma and colour difference, with the spread around it, in 8-bit
/// levels: the band is the template's pixels with `α` in [`BAND`]; around
/// it is every pixel of the rectangle under the noise floor and of a ring
/// four pixels out, inside the picture. Zeros with either empty; the
/// share is left to the caller. The roughness of the pixels under the
/// mark below the opaque threshold, and of those around it, beside them
/// (D250).
fn steps(raster: &Raster, verified: &Verified) -> Outline {
    let (w, h) = (i64::from(raster.width()), i64::from(raster.height()));
    let scale = 255.0 / f64::from(raster.layout().max());
    let samples = raster.samples();
    // R, G, B, then BT.601 Y, Cb and Cr (JFIF's, less the offset).
    let six = |x: i64, y: i64| {
        let i = raster.at(x as u32, y as u32);
        let [r, g, b] = [0, 1, 2].map(|c| f64::from(samples[i + c]) * scale);
        let luma = f64::from(LUMA[0]) * r + f64::from(LUMA[1]) * g + f64::from(LUMA[2]) * b;
        [
            r,
            g,
            b,
            luma,
            -0.168_736 * r - 0.331_264 * g + 0.5 * b,
            0.5 * r - 0.418_688 * g - 0.081_312 * b,
        ]
    };
    let at = verified.at;
    let (x0, y0) = (i64::from(at.x), i64::from(at.y));
    let (x1, y1) = (x0 + i64::from(at.width), y0 + i64::from(at.height));
    // Every pixel the ring and its neighbours reach, read once.
    let (bx0, by0) = ((x0 - 5).max(0), (y0 - 5).max(0));
    let (bx1, by1) = ((x1 + 5).min(w), (y1 + 5).min(h));
    let bw = bx1 - bx0;
    let mut read = Vec::with_capacity((bw * (by1 - by0)) as usize);
    for y in by0..by1 {
        for x in bx0..bx1 {
            read.push(six(x, y));
        }
    }
    let px = |x: i64, y: i64| &read[((y - by0) * bw + (x - bx0)) as usize];
    // A pixel's distance in (Y, Cb, Cr) from the mean of its neighbours
    // inside the picture.
    let rough = |x: i64, y: i64| {
        let (mut mean, mut n) = ([0f64; 3], 0f64);
        for ny in (y - 1).max(by0)..(y + 2).min(by1) {
            for nx in (x - 1).max(bx0)..(x + 2).min(bx1) {
                if (nx, ny) != (x, y) {
                    for (m, v) in mean.iter_mut().zip(&px(nx, ny)[3..]) {
                        *m += v;
                    }
                    n += 1.0;
                }
            }
        }
        let p = &px(x, y)[3..];
        (0..3)
            .map(|c| (p[c] - mean[c] / n).powi(2))
            .sum::<f64>()
            .sqrt()
    };
    let opaque = f64::from(verified.opaque_above);
    let (mut band, mut nb) = ([0f64; 6], 0f64);
    let mut around = Vec::new();
    let (mut rough_mark, mut rough_around) = (Vec::new(), Vec::new());
    for y in (y0 - 4).max(0)..(y1 + 4).min(h) {
        for x in (x0 - 4).max(0)..(x1 + 4).min(w) {
            let inside = x >= x0 && y >= y0 && x < x1 && y < y1;
            let a = if inside {
                verified.values[((y - y0) * i64::from(at.width) + (x - x0)) as usize]
            } else {
                0.0
            };
            if a < NOISE_FLOOR {
                around.push(*px(x, y));
                rough_around.push(rough(x, y));
                continue;
            }
            if f64::from(a) < opaque {
                rough_mark.push(rough(x, y));
            }
            if (BAND[0]..=BAND[1]).contains(&a) {
                for (b, v) in band.iter_mut().zip(px(x, y)) {
                    *b += v;
                }
                nb += 1.0;
            }
        }
    }
    let mut outline = Outline {
        share: 0.0,
        steps: [0.0; 3],
        step: 0.0,
        spread: 0.0,
        chroma: 0.0,
        chroma_spread: 0.0,
        texture: percentile(&mut rough_mark, 0.95) as f32,
        texture_around: percentile(&mut rough_around, 0.95) as f32,
    };
    if nb == 0.0 || around.is_empty() {
        return outline;
    }
    let n = around.len() as f64;
    let (mut step, mut var) = ([0f64; 6], [0f64; 6]);
    for c in 0..6 {
        let mean = around.iter().map(|p| p[c]).sum::<f64>() / n;
        var[c] = around.iter().map(|p| (p[c] - mean).powi(2)).sum::<f64>() / n;
        step[c] = band[c] / nb - mean;
    }
    outline.steps = [step[0] as f32, step[1] as f32, step[2] as f32];
    outline.step = step[3] as f32;
    outline.spread = var[3].sqrt() as f32;
    outline.chroma = step[4].hypot(step[5]) as f32;
    outline.chroma_spread = (var[4] + var[5]).sqrt() as f32;
    outline
}

/// The value `p` of the way up `values`, sorted in place, by nearest rank;
/// 0 for none.
fn percentile(values: &mut [f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    values[((values.len() - 1) as f64 * p).round() as usize]
}

/// How far a restoration is from the data (D305): over `pairs` — per
/// sample, the restored value blended back with the very `α`, logo and
/// gain the restoration used, beside the stored value it was restored
/// from, both in 8-bit levels — the 95th percentile of their distance;
/// 0 for none. `excluded` is what the caller left out (clamped samples
/// and holes), handed back beside it. Written once, for every path that
/// restores: the RGB raster's here, a JPEG's planes later (Y and chroma
/// at their own resolutions, one percentile over both). A measure: no
/// bound, no verdict.
pub(crate) fn consistency(
    pairs: impl IntoIterator<Item = (f64, f64)>,
    excluded: u32,
) -> Consistency {
    let mut distance: Vec<f64> = pairs
        .into_iter()
        .map(|(blended, stored)| (blended - stored).abs())
        .collect();
    Consistency {
        px: percentile(&mut distance, 0.95) as f32,
        excluded,
    }
}

/// [`consistency`]'s answer: `Restored::consistency_px` and
/// `Restored::consistency_excluded`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Consistency {
    pub px: f32,
    pub excluded: u32,
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
    use crate::{restore, ExamineOptions, Restored};

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
            resampled: false,
            searched: false,
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

    /// A perfectly flat green 24 × 24 picture, a 16-pixel mark at (4, 4):
    /// its two outer rings the faint band (`α` 0.1), its middle the body,
    /// the four pixels around it the picture. `shift(i)` moves the `i`-th
    /// pixel of the band by so many levels per channel. What is left
    /// there, measured.
    fn flat_band(shift: impl Fn(usize) -> [i32; 3]) -> Outline {
        const GREEN: [i32; 3] = [9, 150, 56];
        let mut values = vec![0.5f32; 16 * 16];
        let mut samples = Vec::with_capacity(24 * 24 * 3);
        let mut band = 0;
        for y in 0..24u32 {
            for x in 0..24u32 {
                let (tx, ty) = (x.wrapping_sub(4), y.wrapping_sub(4));
                let mut px = GREEN;
                if tx < 16 && ty < 16 && (tx.min(ty) < 2 || tx.max(ty) >= 14) {
                    values[(ty * 16 + tx) as usize] = 0.1;
                    for (p, d) in px.iter_mut().zip(shift(band)) {
                        *p += d;
                    }
                    band += 1;
                }
                samples.extend(px.map(|v| v as u8));
            }
        }
        let verified = Verified {
            at: PixelRect {
                x: 4,
                y: 4,
                width: 16,
                height: 16,
            },
            values,
            noise: vec![0.0; 16 * 16],
            width: 24,
            height: 24,
            ..one(0.1)
        };
        let raster = Raster::from_u8(24, 24, Layout::Rgb8, &samples).unwrap();
        outline(&raster, &verified)
    }

    /// On a picture with no spread at all, a step under a level is
    /// nothing an eye finds and is not said; one over it is — the luma
    /// bound from below as well as above (D244).
    #[test]
    fn a_sub_level_step_on_a_flat_picture_is_not_an_outline() {
        // 84 of the band's 112 pixels a level lighter: 0.75 in luma.
        let under = flat_band(|i| if i < 84 { [1; 3] } else { [0; 3] });
        assert!((under.step - 0.75).abs() < 1e-3, "{under:?}");
        assert!(under.chroma < 1e-3 && under.spread == 0.0, "{under:?}");
        assert!(!under.left(), "{under:?}");
        // All a level lighter, 28 of them two: 1.25.
        let over = flat_band(|i| if i < 28 { [2; 3] } else { [1; 3] });
        assert!((over.step - 1.25).abs() < 1e-3, "{over:?}");
        assert!(over.left(), "{over:?}");
    }

    /// A fringe in colour with the luma kept — red up, green down, as a
    /// subsampled JPEG leaves it — is held to [`CHROMA_LEVELS`] (D247):
    /// 2.8 levels of colour difference is not said, 5.2 is, and luma
    /// alone sees neither; a fringe in blue and yellow, mostly `Cb`, is
    /// said as one in red and green is.
    #[test]
    fn a_fringe_in_colour_is_said_over_its_bound_and_not_under_it() {
        let under = flat_band(|_| [4, -2, 0]);
        assert!(
            under.step.abs() < 0.1 && (under.chroma - 2.84).abs() < 0.01,
            "{under:?}"
        );
        assert!(!under.left(), "{under:?}");
        let over = flat_band(|_| [7, -4, 0]);
        assert!(
            over.step.abs() < 0.3 && (over.chroma - 5.18).abs() < 0.01,
            "{over:?}"
        );
        assert_eq!(over.steps, [7.0, -4.0, 0.0]);
        assert!(over.left(), "{over:?}");
        // Blue up, red and green down: almost all `Cb` — 5.0 of it against
        // 0.8 of `Cr` — and said as well; colour is both halves.
        let blue = flat_band(|_| [-2, -2, 8]);
        assert!(
            blue.step.abs() < 1.0 && (blue.chroma - 5.07).abs() < 0.01,
            "{blue:?}"
        );
        assert!(blue.left(), "{blue:?}");
    }

    /// A perfectly flat green 32 × 32 picture, a 16-pixel mark at (8, 8)
    /// with `α` 0.5 throughout, and every fourth pixel each way lifted —
    /// grey, by `mark` levels under the mark and by `around` in the four
    /// pixels around it: one pixel in sixteen on both, so the 95th
    /// percentile of the roughness is the lift itself. What is left there,
    /// measured.
    fn grained(mark: i32, around: i32) -> Outline {
        const GREEN: [i32; 3] = [9, 150, 56];
        let mut samples = Vec::with_capacity(32 * 32 * 3);
        for y in 0..32u32 {
            for x in 0..32u32 {
                let inside = (8..24).contains(&x) && (8..24).contains(&y);
                let lift = match (x % 4 == 0 && y % 4 == 0, inside) {
                    (false, _) => 0,
                    (true, true) => mark,
                    (true, false) => around,
                };
                samples.extend(GREEN.map(|v| (v + lift) as u8));
            }
        }
        let verified = Verified {
            at: PixelRect {
                x: 8,
                y: 8,
                width: 16,
                height: 16,
            },
            values: vec![0.5; 16 * 16],
            noise: vec![0.0; 16 * 16],
            width: 32,
            height: 32,
            ..one(0.5)
        };
        let raster = Raster::from_u8(32, 32, Layout::Rgb8, &samples).unwrap();
        outline(&raster, &verified)
    }

    /// A grain under the mark on a picture with none is held to
    /// [`TEXTURE_LEVELS`] (D250): 5 levels is not said, 6 is — and no step
    /// sees either.
    #[test]
    fn a_texture_is_said_over_its_bound_and_not_under_it() {
        let under = grained(5, 0);
        assert!((under.texture - 5.0).abs() < 1e-3, "{under:?}");
        // Around it, only the pixels beside a lifted one: an eighth of it.
        assert!((under.texture_around - 5.0 / 8.0).abs() < 1e-3, "{under:?}");
        assert!(!under.textured() && !under.left(), "{under:?}");
        let over = grained(6, 0);
        assert!((over.texture - 6.0).abs() < 1e-3, "{over:?}");
        assert!(over.textured() && !over.left(), "{over:?}");
    }

    /// A grain the picture has around the mark as well is the picture's:
    /// under the mark it must be [`TEXTURE_RATIO`] times as rough to be
    /// said (D250) — 13 levels against 7 around (1.86 times) is not, 15
    /// against 7 (2.14 times) is: the ratio pinned to within 7 % each way,
    /// both over [`TEXTURE_LEVELS`] so the ratio alone decides.
    #[test]
    fn a_texture_the_picture_has_around_the_mark_is_not_said() {
        let grainy = grained(13, 7);
        assert!((grainy.texture - 13.0).abs() < 1e-3, "{grainy:?}");
        assert!((grainy.texture_around - 7.0).abs() < 1e-3, "{grainy:?}");
        assert!(!grainy.textured(), "{grainy:?}");
        let rougher = grained(15, 7);
        assert!((rougher.texture - 15.0).abs() < 1e-3, "{rougher:?}");
        assert!((rougher.texture_around - 7.0).abs() < 1e-3, "{rougher:?}");
        assert!(rougher.textured(), "{rougher:?}");
    }

    /// A grey 32 × 32 picture under a 16-pixel mark at (8, 8) with `α`
    /// 0.5 throughout and a white logo, stored as the blend of a grain
    /// over grey 101: every fourth pixel each way lifted, by `mark` levels
    /// under the mark and by `around` in the pixels around it — both even,
    /// so the blend is a whole level and the restoration gives the grain
    /// back exactly. Restored at `source`.
    fn restored_grain(mark: i32, around: i32, source: crate::Fidelity) -> Restored {
        let mut samples = Vec::with_capacity(32 * 32 * 3);
        for y in 0..32u32 {
            for x in 0..32u32 {
                let inside = (8..24).contains(&x) && (8..24).contains(&y);
                let lift = match (x % 4 == 0 && y % 4 == 0, inside) {
                    (false, _) => 0,
                    (true, true) => mark,
                    (true, false) => around,
                };
                let v = 101 + lift;
                let stored = if inside { (255 + v) / 2 } else { v };
                samples.extend([stored as u8; 3]);
            }
        }
        let verified = Verified {
            at: PixelRect {
                x: 8,
                y: 8,
                width: 16,
                height: 16,
            },
            values: vec![0.5; 16 * 16],
            noise: vec![0.0; 16 * 16],
            width: 32,
            height: 32,
            ..one(0.5)
        };
        let mut raster = Raster::from_u8(32, 32, Layout::Rgb8, &samples).unwrap();
        let options = ExamineOptions {
            source,
            profiles: None,
        };
        restore(&mut raster, &verified, &options).unwrap()
    }

    /// D307: a restoration on a lossy source whose roughness is half the
    /// picture's around it — a patch flatter than its surroundings — is
    /// said (`smoothed`) and counts as a mark left; on a lossless source
    /// the same patch is the picture's own and is not (D251's reason). At
    /// the bound's other side, 0.8 of the surroundings and over, nothing
    /// is said.
    #[test]
    fn a_patch_smoother_than_its_surroundings_is_said() {
        let report = |r: Restored| crate::PixelReport {
            found: Vec::new(),
            restored: vec![r],
            dismissed: 0,
            not_established: crate::not_established::shelf(),
        };
        let soap = restored_grain(4, 8, crate::Fidelity::Lossy);
        assert!((soap.texture - 4.0).abs() < 1e-3, "{soap:?}");
        assert!((soap.texture_around - 8.0).abs() < 1e-3, "{soap:?}");
        assert!(soap.smoothed && !soap.texture_left, "{soap:?}");
        assert!(report(soap).marks_left());
        let lossless = restored_grain(4, 8, crate::Fidelity::Lossless);
        assert!(!lossless.smoothed, "{lossless:?}");
        assert!(!report(lossless).marks_left());
        // 8 under the mark against 10 around: 0.8, at the bound, not said.
        let at = restored_grain(8, 10, crate::Fidelity::Lossy);
        assert!(
            (at.texture / at.texture_around - 0.8).abs() < 1e-3,
            "{at:?}"
        );
        assert!(!at.smoothed, "{at:?}");
        assert!(!report(at).marks_left());
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

    /// D305's measure: the distance either way, the 95th percentile by
    /// nearest rank — one sample off in twenty is the percentile, one in a
    /// hundred is not — 0 with nothing to measure, and the excluded count
    /// handed back as it came.
    #[test]
    fn consistency_is_the_95th_percentile_of_the_distance() {
        let pairs = |off: usize, n: usize, by: f64| {
            (0..n).map(move |i| {
                let stored = 100.0 + i as f64 % 7.0;
                let d = if i < off { by } else { 0.25 };
                (stored + if i % 2 == 0 { d } else { -d }, stored)
            })
        };
        assert_eq!(consistency(pairs(0, 100, 0.0), 4).px, 0.25);
        assert_eq!(consistency(pairs(10, 100, -3.0), 0).px, 3.0);
        assert_eq!(consistency(pairs(1, 100, 3.0), 0).px, 0.25);
        let none = consistency(std::iter::empty(), 7);
        assert_eq!(
            none,
            Consistency {
                px: 0.0,
                excluded: 7
            }
        );
        assert_eq!(consistency(pairs(0, 3, 0.0), 4).excluded, 4);
    }

    /// One pixel at `α` 0.3 under a white logo is restored, and blended
    /// back lands within the rounding of what was stored: the inverse at
    /// 160.71 is written as 161, which blends back to 189.2.
    #[test]
    fn a_restored_pixel_blends_back_to_its_input() {
        let mut raster = Raster::from_u8(1, 1, Layout::Rgb8, &[189; 3]).unwrap();
        let r = restore(&mut raster, &one(0.3), &ExamineOptions::default()).unwrap();
        assert!((r.consistency_px - 0.2).abs() < 1e-4, "{r:?}");
        assert_eq!(r.consistency_excluded, 0);
    }
}
