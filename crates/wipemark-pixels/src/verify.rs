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
//! * the share of samples the `k = 1` inverse puts more than a level out
//!   of range is at most `out_of_range` — an inverse that has to leave the
//!   range to cancel an edge is explaining something that is not a blend.
//!
//! Only this module constructs a [`Verified`], and only a `Verified` can
//! be restored: the type system keeps "write only what was proved".

use serde::Serialize;

use crate::catalogue::Profile;
use crate::geometry::{template, PixelRect, SubRect};
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
    /// The raster's dimensions, so a restore onto another raster is
    /// refused.
    width: u32,
    height: u32,
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

    pub(crate) fn fits(&self, raster: &Raster) -> bool {
        raster.width() == self.width && raster.height() == self.height
    }
}

/// The second proof over one proposal: the numbers, and the verdict.
/// `None` for the numbers when a refusal came before any was measured.
pub(crate) fn verify(
    raster: &Raster,
    profile: &Profile,
    proposal: &Proposal,
) -> (Option<Scores>, Result<Verified, Refusal>) {
    let map = profile.map(proposal.map);
    let Some((shape, at)) = template(map, proposal.rect) else {
        return (None, Err(Refusal::Edges { ratio: 1.0 }));
    };
    if !at.inside(raster.width(), raster.height()) {
        return (None, Err(Refusal::Edges { ratio: 1.0 }));
    }
    let layout = raster.layout();
    let max = f64::from(layout.max());
    let samples = raster.samples();
    let opaque = profile.opaque_above;

    // Support, holes, and the picture's own alpha under the mark.
    let mut support = 0u32;
    let mut holes = 0u32;
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
                if samples[i + 3] < layout.max() {
                    return (None, Err(Refusal::Transparent));
                }
            }
        }
    }
    if support == 0 || holes == support {
        return (None, Err(Refusal::Opaque { holes }));
    }

    // The grid: the rectangle and a one-pixel ring, inside the picture.
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
                alpha[gy * gw + gx] = shape.values[((y - at.y) * at.width + (x - at.x)) as usize];
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

    let logo = profile.logo.map(|c| f64::from(c) * max / 255.0);
    let mut energy = [0f64; STEPS + 1];
    let mut luma = vec![0f64; gw * gh];
    for (i, e) in energy.iter_mut().enumerate() {
        let k = i as f64 / ONE as f64;
        for (p, l) in luma.iter_mut().enumerate() {
            let o = inverse(pixels[p], f64::from(alpha[p]) * k, logo, f64::from(opaque));
            *l =
                (f64::from(LUMA[0]) * o[0] + f64::from(LUMA[1]) * o[1] + f64::from(LUMA[2]) * o[2])
                    / max;
        }
        *e = edges
            .iter()
            .map(|&(p, g)| {
                let dx = (luma[p + 1] - luma[p - 1]) * 0.5;
                let dy = (luma[p + gw] - luma[p - gw]) * 0.5;
                dx.hypot(dy) * f64::from(g)
            })
            .sum();
    }
    let star = (0..=STEPS)
        .min_by(|&a, &b| energy[a].total_cmp(&energy[b]))
        .unwrap_or(0);
    let gain = star as f32 / ONE as f32;
    let edge_ratio = if energy[0] > 1e-12 {
        (energy[ONE] / energy[0]) as f32
    } else {
        // No edge at all where the mark has one: there is no mark here.
        1.0
    };

    // Out of range at k = 1, over the restorable support.
    let (mut out, mut total) = (0u32, 0u32);
    for (p, &a) in alpha.iter().enumerate() {
        if a < NOISE_FLOOR || a >= opaque {
            continue;
        }
        for v in inverse(pixels[p], f64::from(a), logo, f64::from(opaque)) {
            total += 1;
            if v < -1.0 || v > max + 1.0 {
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
    let verdict = if (gain - 1.0).abs() > t.gain {
        Err(Refusal::Gain { k: gain })
    } else if edge_ratio > t.edge_ratio {
        Err(Refusal::Edges { ratio: edge_ratio })
    } else if out_of_range > t.out_of_range {
        Err(Refusal::OutOfRange {
            share: out_of_range,
        })
    } else {
        let exact_place = matches!(proposal.placed, crate::Placed::Row(_))
            && !proposal.resample
            && shape.canonical;
        Ok(Verified {
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
            width: raster.width(),
            height: raster.height(),
        })
    };
    (Some(scores), verdict)
}

/// `O = (I − a·L)/(1 − a)` per channel, unclamped; the input itself where
/// `a` is at or above the opaque threshold, which is never divided.
fn inverse(i: [f64; 3], a: f64, logo: [f64; 3], opaque: f64) -> [f64; 3] {
    if a <= 0.0 || a >= opaque {
        return i;
    }
    [
        (i[0] - a * logo[0]) / (1.0 - a),
        (i[1] - a * logo[1]) / (1.0 - a),
        (i[2] - a * logo[2]) / (1.0 - a),
    ]
}
