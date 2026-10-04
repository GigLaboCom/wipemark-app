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
// restoration inexact; the alpha channel is never written (D157). No
// code was copied.

//! Restoring what was verified, and compositing for tests.

use serde::Serialize;

use crate::alpha::AlphaMap;
use crate::geometry::PixelRect;
use crate::raster::Raster;
use crate::verify::{Verified, NOISE_FLOOR};
use crate::{ExamineOptions, Fidelity};

/// What one restoration did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
    /// Lossless source, a row's own canonical map, no hole, no clamp: the
    /// original values to within one level.
    pub exact: bool,
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
            let mut moved = false;
            for c in 0..3 {
                let o = (f64::from(samples[i + c]) - a * logo[c]) / (1.0 - a);
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
    Ok(Restored {
        profile: verified.profile().to_owned(),
        rect: at,
        changed,
        holes,
        clamped,
        exact: options.source == Fidelity::Lossless
            && verified.exact_place()
            && holes == 0
            && clamped == 0,
    })
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
