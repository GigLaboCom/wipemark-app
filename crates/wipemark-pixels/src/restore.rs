// Derived from allenk/GeminiWatermarkTool, `src/core/blend_modes.cpp`
// (`remove_watermark_alpha_blend` and `add_watermark_alpha_blend`) at
// commit 7c6a99f, MIT, Copyright (c) 2024 AllenK (Kwyshell) — see NOTICE.
// Taken: the reverse alpha blend over stored values,
// `O = (I − α·L)/(1 − α)` per channel, its forward form for compositing,
// and the 0.002 noise floor, as `docs/sdd/visible-marks.md` §1.1 records
// them. Changed: no 0.99 clamp — `α` at or above the profile's
// `opaque_above` is a hole, left untouched and counted (D155); the logo
// is a colour per profile, not one scalar; rounding is half away from
// zero; a clamp beyond rounding is counted, and either makes the
// restoration inexact; the alpha channel is never written (D157). The
// equation is written once, in `unblend`, which the second proof's gain
// sweep calls too. No code was copied.

//! Restoring what was verified, and compositing for tests.

use serde::Serialize;

use crate::alpha::AlphaMap;
use crate::geometry::PixelRect;
use crate::raster::Raster;
use crate::verify::{Verified, NOISE_FLOOR};
use crate::{ExamineOptions, Fidelity};

/// What one restoration did.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Restored {
    pub profile: String,
    pub rect: PixelRect,
    /// Pixels with at least one sample changed.
    pub changed: u32,
    /// Pixels at or above the opaque threshold: untouched.
    pub holes: u32,
    /// Samples whose inverse was outside the range by more than half a
    /// level, and were clamped.
    pub clamped: u32,
    /// The share of the mark's contour left after it was restored, beyond
    /// the texture around it (D238, [`crate::verify::OUTLINE_BOUND`]).
    pub outline: f32,
    /// The mark's faint band against the picture around it after it was
    /// restored: per colour channel — R, G, B — the band's mean less the
    /// mean around it, in 8-bit levels, signed (D247).
    pub steps: [f32; 3],
    /// The same step in BT.601 luma, signed (D244,
    /// [`crate::verify::STEP_LEVELS`]).
    pub step: f32,
    /// The same step in BT.601 colour difference, `|(ΔCb, ΔCr)|` (D247,
    /// [`crate::verify::CHROMA_LEVELS`]).
    pub chroma: f32,
    /// An outline is left — `outline` over its bound, or `step` over
    /// [`crate::verify::STEP_LEVELS`] and the luma's own spread around
    /// the mark, or `chroma` over [`crate::verify::CHROMA_LEVELS`] and the
    /// colour's: the restoration is kept — it took most of the mark away —
    /// and an outline of it is said to be left.
    pub outline_left: bool,
    /// The roughness of the pixels the restoration changed, in 8-bit
    /// levels: the 95th percentile of each one's distance in `(Y, Cb, Cr)`
    /// from the mean of its eight neighbours (D250,
    /// [`crate::verify::TEXTURE_LEVELS`]).
    pub texture: f32,
    /// The same over the picture around the mark.
    pub texture_around: f32,
    /// A texture is left — a lossy source, and `texture` over
    /// [`crate::verify::TEXTURE_LEVELS`] and over
    /// [`crate::verify::TEXTURE_RATIO`] times `texture_around`: the
    /// source's error, amplified by the inverse, along the mark's contour.
    /// The restoration is kept, and the mark counts as left (D250).
    pub texture_left: bool,
    /// The map's capture noise — dropped from every template (D241) — was
    /// found drawn in this picture after all, and taken off with the rest
    /// (D246).
    pub noise: bool,
    /// The source was stored with loss: the restoration is as close as the
    /// stored values allow.
    pub lossy: bool,
    /// The map was fitted from real outputs, not the vendor's own α
    /// (D245): the restoration is held to the picture around it
    /// (`step`), never claimed exact.
    pub fitted: bool,
    /// The map was drawn at another size or a sub-pixel offset, or its row
    /// asks for a resample: not the map as captured, so not exact.
    pub resampled: bool,
    /// Found by the search, away from every row: not exact.
    pub searched: bool,
    /// Lossless source, a row's own canonical map that is not fitted, no
    /// hole, no clamp, no outline: the original values to within one level.
    pub exact: bool,
    /// Restored in the planes of a subsampled JPEG (D306, E12-R6), or a
    /// lossy JPEG whose planes could not be read; `None` on the RGB path,
    /// and then not in the JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planar: Option<crate::planar::Planar>,
}

/// Why a verified mark was not restored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RestoreError {
    #[error("the verified mark belongs to a picture of another size")]
    Elsewhere,
}

/// Invert the blend over a verified mark: per pixel of its rectangle
/// with `α` at or above the noise floor and below the opaque threshold,
/// per colour channel, `O = (I − α·L)/(1 − α)`, rounded half away from
/// zero and clamped to the range. Alpha is never written; holes are
/// counted and left.
pub fn restore(
    raster: &mut Raster,
    verified: &Verified,
    options: &ExamineOptions,
) -> Result<Restored, RestoreError> {
    if !verified.fits(raster) {
        return Err(RestoreError::Elsewhere);
    }
    let at = verified.pixels();
    let max = f64::from(raster.layout().max());
    let logo = verified.logo();
    let opaque = verified.opaque_above();
    let (mut changed, mut holes, mut clamped) = (0u32, 0u32, 0u32);
    for ty in 0..at.height {
        for tx in 0..at.width {
            let a = verified.values()[(ty * at.width + tx) as usize];
            if a < NOISE_FLOOR {
                continue;
            }
            if a >= opaque {
                holes += 1;
                continue;
            }
            let a = f64::from(a);
            let i = raster.at(at.x + tx, at.y + ty);
            let samples = raster.samples_mut();
            let stored = [
                f64::from(samples[i]),
                f64::from(samples[i + 1]),
                f64::from(samples[i + 2]),
            ];
            let original = unblend(stored, a, logo);
            let mut moved = false;
            for (c, o) in original.into_iter().enumerate() {
                if o < -0.5 || o > max + 0.5 {
                    clamped += 1;
                }
                let v = o.round().clamp(0.0, max) as u16;
                if v != samples[i + c] {
                    samples[i + c] = v;
                    moved = true;
                }
            }
            changed += u32::from(moved);
        }
    }
    let noise = drawn_noise(raster, verified);
    if noise {
        for (p, &a) in verified.noise().iter().enumerate() {
            if a < NOISE_FLOOR {
                continue;
            }
            let (tx, ty) = (p as u32 % at.width, p as u32 / at.width);
            let i = raster.at(at.x + tx, at.y + ty);
            let samples = raster.samples_mut();
            let stored = [
                f64::from(samples[i]),
                f64::from(samples[i + 1]),
                f64::from(samples[i + 2]),
            ];
            let mut moved = false;
            for (c, o) in unblend(stored, f64::from(a), logo).into_iter().enumerate() {
                let v = o.round().clamp(0.0, max) as u16;
                if v != samples[i + c] {
                    samples[i + c] = v;
                    moved = true;
                }
            }
            changed += u32::from(moved);
        }
    }
    let outline = crate::verify::outline(raster, verified);
    Ok(Restored {
        profile: verified.profile().to_owned(),
        rect: at,
        changed,
        holes,
        clamped,
        outline: outline.share,
        steps: outline.steps,
        step: outline.step,
        chroma: outline.chroma,
        outline_left: outline.left(),
        texture: outline.texture,
        texture_around: outline.texture_around,
        // The error a lossy source stored, amplified: a lossless one stored
        // none past the rounding to a level, and what is rough under its
        // mark is the picture's own (D250).
        texture_left: options.source == Fidelity::Lossy && outline.textured(),
        noise,
        lossy: options.source == Fidelity::Lossy,
        fitted: verified.fitted(),
        resampled: verified.resampled(),
        searched: verified.searched(),
        exact: options.source == Fidelity::Lossless
            && verified.exact_place()
            && !verified.fitted()
            && holes == 0
            && clamped == 0
            && !outline.left(),
        planar: None,
    })
}

/// Whether the picture carries the capture noise its template dropped
/// (D246). The noise is a faint square of pixel-to-pixel speckle, 1–6/255
/// of the logo over the picture: drawn, it lifts each pixel under it by
/// `a·(L − O)`; not drawn, by nothing. So the speckle is looked for in the
/// picture's own fine detail: per noise pixel, its luma less the mean of
/// its 3 × 3 neighbourhood, against the lift a drawn noise would make
/// there less the same mean of lifts — and the slope of one on the other,
/// by least squares, is about 1 when the noise is drawn and about 0 when
/// it is not; believed over a half. A texture's own detail does not follow
/// the capture's speckle, so it is not mistaken for it, where a mean
/// against the picture around the square was. Measured: under GWT's own
/// V1 maps, 0.03 to 0.10 on all 22 real outputs (the vendor draws none,
/// D241); on composites drawn with the noise, about 1. For V2 there is no
/// real output to say, and this keeps a V2 mark drawn either way from
/// leaving a square.
///
/// Never for a fitted map (D245): what its denoising drops is the fit's
/// own noise, not a capture's — on the outputs it was fitted from it
/// follows their grain (slopes of 1.3–1.8), on two held out it sits at the
/// threshold (0.58, 0.62). Not evidence of anything; left out.
pub(crate) fn drawn_noise(raster: &Raster, verified: &Verified) -> bool {
    if verified.fitted() {
        return false;
    }
    let at = verified.pixels();
    let logo = verified.logo();
    let samples = raster.samples();
    let noise = verified.noise();
    let (w, h) = (i64::from(at.width), i64::from(at.height));
    let luma = |x: i64, y: i64| {
        let i = raster.at(at.x + x as u32, at.y + y as u32);
        (0..3)
            .map(|c| f64::from(crate::raster::LUMA[c]) * f64::from(samples[i + c]))
            .sum::<f64>()
    };
    let lift = |x: i64, y: i64| {
        let a = f64::from(noise[(y * w + x) as usize]);
        if a < f64::from(NOISE_FLOOR) {
            return 0.0;
        }
        let i = raster.at(at.x + x as u32, at.y + y as u32);
        (0..3)
            .map(|c| {
                f64::from(crate::raster::LUMA[c]) * a * (logo[c] - f64::from(samples[i + c]))
                    / (1.0 - a)
            })
            .sum::<f64>()
    };
    let (mut de, mut ee) = (0f64, 0f64);
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            if noise[(y * w + x) as usize] < NOISE_FLOOR {
                continue;
            }
            let (mut l, mut e) = (0f64, 0f64);
            for (dx, dy) in (-1..=1).flat_map(|dy| (-1..=1).map(move |dx| (dx, dy))) {
                l += luma(x + dx, y + dy);
                e += lift(x + dx, y + dy);
            }
            let d = luma(x, y) - l / 9.0;
            let e = lift(x, y) - e / 9.0;
            de += d * e;
            ee += e * e;
        }
    }
    ee > 0.0 && de / ee > 0.5
}

/// The reverse blend over stored values, `O = (I − α·L)/(1 − α)` per
/// colour channel, unrounded and unclamped — GWT's equation, the one place
/// it is written: [`restore`] rounds and clamps it, and the second proof
/// (`verify.rs`) sweeps it with `k·α`.
pub(crate) fn unblend(stored: [f64; 3], a: f64, logo: [f64; 3]) -> [f64; 3] {
    [
        (stored[0] - a * logo[0]) / (1.0 - a),
        (stored[1] - a * logo[1]) / (1.0 - a),
        (stored[2] - a * logo[2]) / (1.0 - a),
    ]
}

/// Stamp `map` at its own size onto `raster` at `at`'s origin, with the
/// logo `logo` in 8-bit units: `I = round(α·L + (1 − α)·O)` per colour
/// channel. For tests and the calibration tool — never a feature.
#[doc(hidden)]
pub fn composite(raster: &mut Raster, map: &AlphaMap, at: PixelRect, logo: [f32; 3]) {
    let max = f64::from(raster.layout().max());
    let logo = logo.map(|c| f64::from(c) * max / 255.0);
    let (w, h) = (raster.width(), raster.height());
    for my in 0..map.height().min(at.height) {
        for mx in 0..map.width().min(at.width) {
            let (x, y) = (at.x + mx, at.y + my);
            if x >= w || y >= h {
                continue;
            }
            let a = f64::from(map.get(i64::from(mx), i64::from(my)));
            if a <= 0.0 {
                continue;
            }
            let i = raster.at(x, y);
            let samples = raster.samples_mut();
            for c in 0..3 {
                let v = a * logo[c] + (1.0 - a) * f64::from(samples[i + c]);
                samples[i + c] = v.round().clamp(0.0, max) as u16;
            }
        }
    }
}
