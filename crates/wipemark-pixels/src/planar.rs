//! The planar inverse (E12-R6, D306): a JPEG whose chroma was subsampled
//! 4:2:0 or 4:2:2, restored — and proved — in the model it was stored in.
//!
//! The vendor blends in RGB, `I = α·L + (1 − α)·O`. The encoder then
//! converts to YCbCr (JFIF, full range — affine in RGB, so the blend keeps
//! its form per plane), **averages** Cb and Cr over 2 × 2 blocks (4:2:0)
//! or 2 × 1 (4:2:2), and quantises; the decoder upsamples the chroma back
//! with a triangle filter. The RGB inverse divides that upsampled chroma
//! by a full-resolution `α`: a model the file never followed, which leaves
//! the colour fringe of D247 and refuses by the 16-pixel grid (D252). Here
//! each plane is inverted at its own resolution:
//!
//! ```text
//! Y_O(p)      = (Y_I(p)      − α(p)·L_Y)  / (1 − α(p))      full resolution
//! Cb_O,sub(q) = (Cb_I,sub(q) − ᾱ(q)·L_Cb) / (1 − ᾱ(q))      ᾱ(q) = the block's mean α; Cr alike
//! ```
//!
//! The second line assumes `Cb_O` constant within a block: its error is at
//! most `max_B|Cb_O − mean_B Cb_O| · max_B|α − ᾱ|`, so `max_B|α − ᾱ|` is
//! measured and reported ([`Planar::Inverse`]). The restored chroma is
//! upsampled by the decoder's own triangle filter (in reals, before the
//! one rounding), recombined with `Y_O` by the decoder's own constants, and
//! written **only** where the restoration would have touched: a pixel of
//! the mark's rectangle with `α` at the noise floor or over, or whose
//! chroma block's `ᾱ` is. Everything else stays the decoder's RGB.
//!
//! The proof's out-of-range share is measured the same way: Y against
//! [`BLEND_LEVELS`], each chroma block with `ᾱ` against
//! [`blend_levels_c`]; a pixel is out when its Y is or its block is. The
//! edge energy and `k*` stay on luma, unchanged.
//!
//! A profile's bias (R9a) and logo colour map (R9b), under
//! `blend-preview`, go through the same matrix: the bias's linear part is
//! taken off every plane where the mark is, `L(p)` is brought to `(Y, Cb,
//! Cr)` per pixel and, for a chroma block, weighted by the pixels' `α`
//! into the block's own. A `linear-light` profile (R9c) is not linear in
//! the planes' code values and never takes this path: it is proved and
//! restored in RGB.
//!
//! The route is narrow ([`route`]): a lossy source whose planes are known
//! and subsampled. 4:4:4 already matches the RGB model, and a PNG, a WebP
//! and a JPEG whose planes could not be read take the old path, byte for
//! byte. Nothing in the product calls this yet (S12): `examine` and
//! `clean` pass no planes.

use serde::{Serialize, Serializer};

use crate::blend::{Colours, Law};
use crate::geometry::PixelRect;
use crate::planes::{Planes, Sampling};
use crate::raster::{Layout, Raster};
use crate::restore::{RestoreError, Restored};
use crate::verify::{Verified, BLEND_LEVELS, NOISE_FLOOR};
use crate::{ExamineOptions, Fidelity};

/// How much of the chroma table's DC step is added to [`BLEND_LEVELS`]
/// for a chroma block (D306): `BLEND_LEVELS_C = 8 + Q_C[0]·DC_SHARE`.
/// `[tunable]` — the chroma table is coarser than luma's, and a block's
/// stored mean moves by a fraction of its DC step. See the E12-R6 report
/// for what was measured.
pub const DC_SHARE: f64 = 0.5;

/// The out-of-range allowance of a chroma block, in stored levels:
/// [`BLEND_LEVELS`] and [`DC_SHARE`] of the chroma table's DC step.
pub fn blend_levels_c(planes: &Planes) -> f64 {
    BLEND_LEVELS + f64::from(planes.quant().chroma.map_or(0, |t| t[0])) * DC_SHARE
}

/// What a restoration in the planes adds to [`Restored`] (D306). Skipped
/// in the JSON when `None`, so every other report is what it was.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Planar {
    /// Inverted in the planes.
    Inverse {
        sampling: Sampling,
        /// The largest `|α − ᾱ|` within a chroma block the inverse
        /// divided: how far the block-mean model is from the pixels' own.
        max_alpha_dev_in_block: f32,
        /// Chroma blocks at or over the opaque threshold: every pixel of
        /// each is a hole.
        holes_chroma: u32,
    },
    /// A lossy JPEG whose planes could not be read: restored in RGB.
    Unavailable,
}

impl Serialize for Planar {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Inverse {
            sampling: &'static str,
            max_alpha_dev_in_block: f32,
            holes_chroma: u32,
        }
        match *self {
            Planar::Inverse {
                sampling,
                max_alpha_dev_in_block,
                holes_chroma,
            } => Inverse {
                sampling: sampling.id(),
                max_alpha_dev_in_block,
                holes_chroma,
            }
            .serialize(s),
            Planar::Unavailable => s.serialize_str("unavailable"),
        }
    }
}

/// The out-of-range share's two terms on the planar path (D306), each
/// over the same pixels as the share: those with a Y out of range, and
/// those whose chroma block is. [`crate::Scores::out_of_range`] is the
/// pixels with either.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PlanarScores {
    pub y: f32,
    pub chroma: f32,
}

/// The planes a planar examination reads, with the chroma's block shape
/// and its allowance.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Model<'a> {
    planes: &'a Planes,
    sx: u32,
    sy: u32,
    levels_c: f64,
}

/// The planar path, or `None` for the old one: a lossy source, planes
/// known, subsampled 4:2:0 or 4:2:2, the raster's own size, 8-bit RGB.
pub(crate) fn route<'a>(
    raster: &Raster,
    planes: Option<&'a Planes>,
    options: &ExamineOptions,
) -> Option<Model<'a>> {
    let planes = planes?;
    if options.source != Fidelity::Lossy {
        return None;
    }
    Model::new(raster, planes, blend_levels_c(planes))
}

impl<'a> Model<'a> {
    /// `planes` over `raster` with a chroma allowance of `levels_c`, when
    /// they can be read together.
    pub(crate) fn new(raster: &Raster, planes: &'a Planes, levels_c: f64) -> Option<Self> {
        let (sx, sy) = match planes.sampling() {
            Sampling::H420 => (2, 2),
            Sampling::H422 => (2, 1),
            _ => return None,
        };
        if raster.layout() != Layout::Rgb8
            || (planes.width(), planes.height()) != (raster.width(), raster.height())
            || planes.cb().is_none()
            || planes.cr().is_none()
        {
            return None;
        }
        Some(Model {
            planes,
            sx,
            sy,
            levels_c,
        })
    }

    /// The out-of-range share at `k = 1` in the planes (D306), over the
    /// template's pixels with `NOISE_FLOOR ≤ α < opaque`: the share with
    /// a Y outside `[α·L_Y, α·L_Y + (1 − α)·255]` by more than
    /// [`BLEND_LEVELS`] or a chroma block whose Cb or Cr is outside
    /// `[ᾱ·L_C, ᾱ·L_C + (1 − ᾱ)·255]` by more than the chroma allowance —
    /// and each term apart.
    ///
    /// Under `blend-preview` the interval moves by the bias's linear part
    /// in `(Y, Cb, Cr)` (R9a) and `L` is the pixel's own, or the block's
    /// `α`-weighted, from the logo colour map (R9b); without either, both
    /// terms are what they were, operation for operation.
    pub(crate) fn out_of_range(
        &self,
        values: &[f32],
        at: PixelRect,
        colours: &Colours,
        law: Law,
        opaque: f32,
    ) -> (f32, PlanarScores) {
        let blocks = Blocks::new(self, values, at);
        let l = ycc(colours.logo);
        let bias = law.bias.map_or([0.0; 3], ycc_delta);
        let logo_blocks = colours
            .per_pixel
            .as_deref()
            .map(|v| block_logos(&blocks, self.size(), values, at, v, l));
        let opaque = f64::from(opaque);
        let block_out: Vec<bool> = (0..blocks.alpha.len())
            .map(|b| {
                let a = blocks.alpha[b];
                if !(f64::from(NOISE_FLOOR)..opaque).contains(&a) {
                    return false;
                }
                let lb = logo_blocks.as_ref().map_or(l, |v| v[b]);
                let (qx, qy) = blocks.coords(b);
                let cb = f64::from(self.cb().get(qx, qy)) - bias[1];
                let cr = f64::from(self.cr().get(qx, qy)) - bias[2];
                gap(cb, a, lb[1]) > self.levels_c || gap(cr, a, lb[2]) > self.levels_c
            })
            .collect();
        let y_plane = self.planes.y();
        let (mut n, mut y_out, mut c_out, mut out) = (0u32, 0u32, 0u32, 0u32);
        for ty in 0..at.height {
            for tx in 0..at.width {
                let a = values[(ty * at.width + tx) as usize];
                if !(NOISE_FLOOR..opaque as f32).contains(&a) {
                    continue;
                }
                let (x, y) = (at.x + tx, at.y + ty);
                n += 1;
                let ly = colours
                    .per_pixel
                    .as_ref()
                    .map_or(l[0], |v| ycc(v[(ty * at.width + tx) as usize])[0]);
                let stored = f64::from(y_plane.get(x, y)) - bias[0];
                let yo = gap(stored, f64::from(a), ly) > BLEND_LEVELS;
                let co = block_out[blocks.index(x / self.sx, y / self.sy)];
                y_out += u32::from(yo);
                c_out += u32::from(co);
                out += u32::from(yo || co);
            }
        }
        let share = |k: u32| if n == 0 { 0.0 } else { k as f32 / n as f32 };
        (
            share(out),
            PlanarScores {
                y: share(y_out),
                chroma: share(c_out),
            },
        )
    }

    fn cb(&self) -> &crate::planes::Plane {
        self.planes.cb().expect("a model has chroma")
    }

    /// The picture's size, as the planes give it.
    fn size(&self) -> (u32, u32) {
        (self.planes.width(), self.planes.height())
    }

    fn cr(&self) -> &crate::planes::Plane {
        self.planes.cr().expect("a model has chroma")
    }
}

/// How far `v` lies outside what a blend at `a` with logo `l` could have
/// produced over any original in 0–255, `[a·l, a·l + (1 − a)·255]`, in
/// stored levels; 0 inside.
fn gap(v: f64, a: f64, l: f64) -> f64 {
    let lo = a * l;
    let hi = lo + (1.0 - a) * 255.0;
    (lo - v).max(v - hi).max(0.0)
}

/// A difference of RGB colours — a bias (R9a) — in `(Y, Cb, Cr)`: JFIF's
/// matrix without its offset.
pub(crate) fn ycc_delta([r, g, b]: [f64; 3]) -> [f64; 3] {
    [
        0.299 * r + 0.587 * g + 0.114 * b,
        -0.168_736 * r - 0.331_264 * g + 0.5 * b,
        0.5 * r - 0.418_688 * g - 0.081_312 * b,
    ]
}

/// Per chroma block of `blocks`, the logo in `(Y, Cb, Cr)` its pixels
/// blend with as one (R9b): their own colours from the template's
/// `per_pixel` (stored units), through JFIF's matrix and weighted by
/// their `α` — the block's stored mean is `Σα·L/n + …`, so its `L` is
/// `Σα·L/Σα`. `global` where the block holds no opacity.
fn block_logos(
    blocks: &Blocks,
    (w, h): (u32, u32),
    values: &[f32],
    at: PixelRect,
    per_pixel: &[[f64; 3]],
    global: [f64; 3],
) -> Vec<[f64; 3]> {
    (0..blocks.alpha.len())
        .map(|b| {
            let (qx, qy) = blocks.coords(b);
            let (mut sum, mut weight) = ([0f64; 3], 0f64);
            for y in qy * blocks.sy..(qy * blocks.sy + blocks.sy).min(h) {
                for x in qx * blocks.sx..(qx * blocks.sx + blocks.sx).min(w) {
                    if x < at.x || y < at.y || x >= at.x + at.width || y >= at.y + at.height {
                        continue;
                    }
                    let t = ((y - at.y) * at.width + (x - at.x)) as usize;
                    let a = f64::from(values[t]);
                    for (s, l) in sum.iter_mut().zip(ycc(per_pixel[t])) {
                        *s += a * l;
                    }
                    weight += a;
                }
            }
            if weight > 0.0 {
                sum.map(|s| s / weight)
            } else {
                global
            }
        })
        .collect()
}

/// JFIF's forward matrix, full range: `(Y, Cb, Cr)` of an RGB colour.
pub(crate) fn ycc([r, g, b]: [f64; 3]) -> [f64; 3] {
    [
        0.299 * r + 0.587 * g + 0.114 * b,
        -0.168_736 * r - 0.331_264 * g + 0.5 * b + 128.0,
        0.5 * r - 0.418_688 * g - 0.081_312 * b + 128.0,
    ]
}

/// The decoder's inverse (`ycbcr_to_rgb` in `planes.rs`), its 14-bit
/// constants read as reals and nothing rounded: the restoration rounds
/// once, at the write.
fn rgb([y, cb, cr]: [f64; 3]) -> [f64; 3] {
    const S: f64 = 16384.0;
    let (cb, cr) = (cb - 128.0, cr - 128.0);
    [
        y + cr * 22970.0 / S,
        y + (cr * -11700.0 + cb * -5638.0) / S,
        y + cb * 29032.0 / S,
    ]
}

/// The chroma blocks a template reaches, and one more on every side for
/// the triangle filter's far neighbour, inside the plane: each block's
/// mean `α` (`0` outside the template) and its largest deviation from it.
#[derive(Debug, Clone, PartialEq)]
struct Blocks {
    sx: u32,
    sy: u32,
    qx0: u32,
    qy0: u32,
    qw: u32,
    qh: u32,
    alpha: Vec<f64>,
    deviation: Vec<f64>,
}

impl Blocks {
    fn new(model: &Model<'_>, values: &[f32], at: PixelRect) -> Self {
        let (sx, sy) = (model.sx, model.sy);
        let (cw, ch) = (model.cb().width(), model.cb().height());
        let (w, h) = (model.planes.width(), model.planes.height());
        let qx0 = (at.x / sx).saturating_sub(1);
        let qy0 = (at.y / sy).saturating_sub(1);
        let qx1 = ((at.x + at.width - 1) / sx + 2).min(cw);
        let qy1 = ((at.y + at.height - 1) / sy + 2).min(ch);
        let (qw, qh) = (qx1 - qx0, qy1 - qy0);
        let alpha_at = |x: u32, y: u32| -> f64 {
            if x >= at.x && y >= at.y && x < at.x + at.width && y < at.y + at.height {
                f64::from(values[((y - at.y) * at.width + (x - at.x)) as usize])
            } else {
                0.0
            }
        };
        let mut alpha = vec![0f64; (qw * qh) as usize];
        let mut deviation = vec![0f64; (qw * qh) as usize];
        for qy in qy0..qy1 {
            for qx in qx0..qx1 {
                // The block's pixels inside the picture: an encoder
                // replicates the last column and row, which leaves the
                // mean of those that exist.
                let pixels: Vec<f64> = (qy * sy..(qy * sy + sy).min(h))
                    .flat_map(|y| (qx * sx..(qx * sx + sx).min(w)).map(move |x| (x, y)))
                    .map(|(x, y)| alpha_at(x, y))
                    .collect();
                let mean = pixels.iter().sum::<f64>() / pixels.len() as f64;
                let b = ((qy - qy0) * qw + (qx - qx0)) as usize;
                alpha[b] = mean;
                deviation[b] = pixels.iter().map(|a| (a - mean).abs()).fold(0.0, f64::max);
            }
        }
        Blocks {
            sx,
            sy,
            qx0,
            qy0,
            qw,
            qh,
            alpha,
            deviation,
        }
    }

    /// The index of block `(qx, qy)`, which must be in the region.
    fn index(&self, qx: u32, qy: u32) -> usize {
        ((qy - self.qy0) * self.qw + (qx - self.qx0)) as usize
    }

    /// The plane coordinates of block `b`.
    fn coords(&self, b: usize) -> (u32, u32) {
        let b = b as u32;
        (self.qx0 + b % self.qw, self.qy0 + b / self.qw)
    }
}

/// Everything the planar inverse computed for one verified mark, kept
/// reachable so a blend-back can be measured in the planes (D305, R7) and
/// the tests can hold each step: the template, Y in and out at full
/// resolution, and per chroma block `ᾱ`, Cb and Cr in and out, at the
/// chroma's own resolution. Unrounded and unclamped.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub struct Inverse {
    /// The mark's rectangle; every per-pixel vector below is over it,
    /// row-major.
    pub at: PixelRect,
    pub sampling: Sampling,
    /// `α`, the template the proof used.
    pub alpha: Vec<f32>,
    pub y_in: Vec<f64>,
    pub y_out: Vec<f64>,
    /// The chroma blocks: `(qx0, qy0)` and `qw × qh` in the chroma plane's
    /// own coordinates, row-major — every block the rectangle reaches and
    /// one more on each side, inside the plane.
    pub blocks: PixelRect,
    /// `ᾱ` per block.
    pub alpha_mean: Vec<f64>,
    pub cb_in: Vec<f64>,
    pub cb_out: Vec<f64>,
    pub cr_in: Vec<f64>,
    pub cr_out: Vec<f64>,
    /// `L` in `(Y, Cb, Cr)`.
    pub logo: [f64; 3],
    /// `L_Y` per pixel of the rectangle, from the profile's logo colour
    /// map (R9b, `blend-preview`); `None` is `logo[0]` everywhere.
    pub logo_y: Option<Vec<f64>>,
    /// `(L_Y, L_Cb, L_Cr)` per block, its pixels' `α`-weighted (R9b);
    /// `None` is `logo` everywhere.
    pub logo_blocks: Option<Vec<[f64; 3]>>,
    /// The profile's bias in `(Y, Cb, Cr)`, its linear part (R9a,
    /// `blend-preview`); zeros without one — and a zero taken off a
    /// stored value is that value, to the bit.
    pub bias: [f64; 3],
    /// The opaque threshold, and the block shape `(sx, sy)`.
    pub opaque: f64,
    pub block: (u32, u32),
    /// The largest `|α − ᾱ|` within a block whose `ᾱ` is at the noise
    /// floor or over.
    pub max_alpha_dev_in_block: f64,
}

impl Inverse {
    /// `α` at `(x, y)` of the picture; 0 outside the rectangle.
    pub fn alpha_at(&self, x: u32, y: u32) -> f64 {
        let at = self.at;
        if x >= at.x && y >= at.y && x < at.x + at.width && y < at.y + at.height {
            f64::from(self.alpha[((y - at.y) * at.width + (x - at.x)) as usize])
        } else {
            0.0
        }
    }

    /// The index of block `(qx, qy)` of the chroma plane, if it is in the
    /// region.
    pub fn block_index(&self, qx: u32, qy: u32) -> Option<usize> {
        let b = self.blocks;
        (qx >= b.x && qy >= b.y && qx < b.x + b.width && qy < b.y + b.height)
            .then(|| ((qy - b.y) * b.width + (qx - b.x)) as usize)
    }

    /// `L_Y` at pixel `t` of the rectangle, row-major.
    pub fn logo_y_at(&self, t: usize) -> f64 {
        self.logo_y.as_ref().map_or(self.logo[0], |v| v[t])
    }

    /// `(L_Y, L_Cb, L_Cr)` of block `b`.
    pub fn logo_block(&self, b: usize) -> [f64; 3] {
        self.logo_blocks.as_ref().map_or(self.logo, |v| v[b])
    }

    /// `ᾱ` of the block that holds pixel `(x, y)`; 0 outside the region.
    pub fn block_alpha_at(&self, x: u32, y: u32) -> f64 {
        self.block_index(x / self.block.0, y / self.block.1)
            .map_or(0.0, |b| self.alpha_mean[b])
    }

    /// A hole: `α` at or over the opaque threshold, or a block whose `ᾱ`
    /// is — left as the decoder made it, and counted.
    pub fn hole(&self, x: u32, y: u32) -> bool {
        self.alpha_at(x, y) >= self.opaque || self.block_alpha_at(x, y) >= self.opaque
    }

    /// Whether the restoration writes pixel `(x, y)`: inside the
    /// rectangle, not a hole, and `α` or its block's `ᾱ` at the noise
    /// floor or over (§4.2's mask).
    pub fn writes(&self, x: u32, y: u32) -> bool {
        let at = self.at;
        let floor = f64::from(NOISE_FLOOR);
        x >= at.x
            && y >= at.y
            && x < at.x + at.width
            && y < at.y + at.height
            && !self.hole(x, y)
            && (self.alpha_at(x, y) >= floor || self.block_alpha_at(x, y) >= floor)
    }

    /// The restored `(Cb, Cr)` at pixel `(x, y)` of the rectangle,
    /// upsampled by the decoder's triangle filter (`planes.rs`), in
    /// reals: for 4:2:0 vertically and then horizontally,
    /// `(3·near + far)/4` each way, the far neighbour the block before for
    /// an even coordinate and the block after for an odd one, the plane's
    /// edge standing in past it.
    fn chroma_up(&self, x: u32, y: u32, cw: u32, ch: u32) -> [f64; 2] {
        let (sx, sy) = self.block;
        let near_far = |p: u32, s: u32, n: u32| -> (u32, u32) {
            if s == 1 {
                return (p, p);
            }
            let k = (p / 2).min(n - 1);
            let far = if p.is_multiple_of(2) {
                k.saturating_sub(1)
            } else {
                (k + 1).min(n - 1)
            };
            (k, far)
        };
        let (kx, fx) = near_far(x, sx, cw);
        let (ky, fy) = near_far(y, sy, ch);
        let at =
            |plane: &[f64], qx: u32, qy: u32| self.block_index(qx, qy).map_or(0.0, |b| plane[b]);
        let one = |plane: &[f64]| {
            let v = |qx: u32| {
                if sy == 1 {
                    at(plane, qx, ky)
                } else {
                    (3.0 * at(plane, qx, ky) + at(plane, qx, fy)) / 4.0
                }
            };
            if sx == 1 {
                v(kx)
            } else {
                (3.0 * v(kx) + v(fx)) / 4.0
            }
        };
        [one(&self.cb_out), one(&self.cr_out)]
    }

    /// The restored RGB at pixel `(x, y)` of the rectangle, unrounded.
    pub fn rgb_at(&self, x: u32, y: u32, chroma_size: (u32, u32)) -> [f64; 3] {
        let at = self.at;
        let y_o = self.y_out[((y - at.y) * at.width + (x - at.x)) as usize];
        let [cb, cr] = self.chroma_up(x, y, chroma_size.0, chroma_size.1);
        rgb([y_o, cb, cr])
    }

    /// The restored picture blended back with the mark and compared with
    /// what the file stored, **in the planes** — the pairs a measure of
    /// consistency (D305, `consistency_px`) takes the 95th percentile of,
    /// in stored levels:
    ///
    /// * **Y**, per written pixel with `NOISE_FLOOR ≤ α < opaque`:
    ///   `α·L_Y + (1 − α)·Y(restored RGB)` against the stored `Y_I`;
    /// * **chroma**, per block with `NOISE_FLOOR ≤ ᾱ < opaque` — at the
    ///   chroma's own resolution: `ᾱ·L_C + (1 − ᾱ)·C_rec(q)` against the
    ///   stored `C_I,sub`, for Cb and for Cr, where `C_rec(q)` is the
    ///   restored RGB's chroma brought back to the block by the **left
    ///   inverse of the decoder's upsampler**: per axis,
    ///   `(3·(u₂ₖ + u₂ₖ₊₁) − (u₂ₖ₋₁ + u₂ₖ₊₂))/4` over the four pixels
    ///   around the block, which gives the plane back exactly from its
    ///   triangle-filtered upsampling. Not the encoder's box average: the
    ///   triangle filter followed by a box is a `[1, 6, 1]/8` smoothing,
    ///   which measures the upsampler, not the inverse (1.6–1.8 levels at
    ///   p95 on the 4:2:0 fixtures, where this reads under one).
    ///
    /// `restored` is the raster the restoration wrote, read in its 8-bit
    /// levels. A pixel with a sample clamped (its unrounded RGB more than
    /// half a level outside 0–255) is left out, and so is a block whose
    /// four-by-four support holds one or a hole or leaves the picture;
    /// `excluded` counts them in samples.
    pub fn blend_back(&self, restored: &Raster, chroma_size: (u32, u32)) -> BlendBack {
        let at = self.at;
        let floor = f64::from(NOISE_FLOOR);
        let samples = restored.samples();
        let read = |x: u32, y: u32| {
            let i = restored.at(x, y);
            ycc([0, 1, 2].map(|c| f64::from(samples[i + c])))
        };
        let clamped = |x: u32, y: u32| {
            self.writes(x, y)
                && self
                    .rgb_at(x, y, chroma_size)
                    .iter()
                    .any(|&v| !(-0.5..=255.5).contains(&v))
        };
        let mut pairs = Vec::new();
        let mut excluded = 0u32;
        for ty in 0..at.height {
            for tx in 0..at.width {
                let (x, y) = (at.x + tx, at.y + ty);
                let a = self.alpha_at(x, y);
                if !(floor..self.opaque).contains(&a) || !self.writes(x, y) {
                    continue;
                }
                if clamped(x, y) {
                    excluded += 1;
                    continue;
                }
                let t = (ty * at.width + tx) as usize;
                let blended = a * self.logo_y_at(t) + (1.0 - a) * read(x, y)[0] + self.bias[0];
                pairs.push((blended, self.y_in[t]));
            }
        }
        let luma = pairs.len();
        let (sx, sy) = self.block;
        let (w, h) = (restored.width(), restored.height());
        for b in 0..self.alpha_mean.len() {
            let a = self.alpha_mean[b];
            if !(floor..self.opaque).contains(&a) {
                continue;
            }
            let (qx, qy) = (
                self.blocks.x + b as u32 % self.blocks.width,
                self.blocks.y + b as u32 / self.blocks.width,
            );
            // The left inverse of the triangle filter, per axis: the taps
            // at offsets −1, 0, 1, 2 from the block's first pixel; one tap
            // of weight 1 on an axis that is not subsampled.
            let taps = |q: u32, s: u32, n: u32| -> Option<Vec<(u32, f64)>> {
                if s == 1 {
                    return Some(vec![(q, 1.0)]);
                }
                let first = i64::from(q) * 2;
                [(-1, -0.25), (0, 0.75), (1, 0.75), (2, -0.25)]
                    .into_iter()
                    .map(|(d, wt)| {
                        let p = first + d;
                        (p >= 0 && p < i64::from(n)).then_some((p as u32, wt))
                    })
                    .collect()
            };
            let (Some(xs), Some(ys)) = (taps(qx, sx, w), taps(qy, sy, h)) else {
                excluded += 2;
                continue;
            };
            let support = || {
                ys.iter()
                    .flat_map(|&(y, wy)| xs.iter().map(move |&(x, wx)| (x, y, wx * wy)))
            };
            if support().any(|(x, y, _)| self.hole(x, y) || clamped(x, y)) {
                excluded += 2;
                continue;
            }
            let rec = support().fold([0f64; 2], |m, (x, y, wt)| {
                let c = read(x, y);
                [m[0] + wt * c[1], m[1] + wt * c[2]]
            });
            let lb = self.logo_block(b);
            for (c, input) in [(0, self.cb_in[b]), (1, self.cr_in[b])] {
                let blended = a * lb[c + 1] + (1.0 - a) * rec[c] + self.bias[c + 1];
                pairs.push((blended, input));
            }
        }
        BlendBack {
            pairs,
            luma,
            excluded,
        }
    }
}

/// [`Inverse::blend_back`]'s answer: `(blended back, stored)` per sample,
/// in stored levels — the first `luma` of them Y, the rest Cb and Cr by
/// block — and how many samples were left out.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub struct BlendBack {
    pub pairs: Vec<(f64, f64)>,
    pub luma: usize,
    pub excluded: u32,
}

/// The planar inverse of a mark verified on `raster`, over the `planes`
/// it was decoded from, every step kept; `None` when the planes are not
/// subsampled or not the raster's, or the mark is another raster's — and
/// for a `linear-light` profile (R9c), which is not linear in the planes.
#[doc(hidden)]
pub fn invert(raster: &Raster, planes: &Planes, verified: &Verified) -> Option<Inverse> {
    if !verified.fits(raster) || !verified.law().linear_in_codes() {
        return None;
    }
    let model = Model::new(raster, planes, blend_levels_c(planes))?;
    Some(invert_with(&model, verified))
}

fn invert_with(model: &Model<'_>, verified: &Verified) -> Inverse {
    let at = verified.pixels();
    let values = verified.values();
    let opaque = f64::from(verified.opaque_above());
    let floor = f64::from(NOISE_FLOOR);
    let l = ycc(verified.logo());
    let bias = verified.law().bias.map_or([0.0; 3], ycc_delta);
    let logo_y: Option<Vec<f64>> = verified
        .logos()
        .map(|v| v.iter().map(|&c| ycc(c)[0]).collect());
    let blocks = Blocks::new(model, values, at);
    let logo_blocks = verified
        .logos()
        .map(|v| block_logos(&blocks, model.size(), values, at, v, l));
    let y_plane = model.planes.y();
    let mut y_in = Vec::with_capacity(values.len());
    let mut y_out = Vec::with_capacity(values.len());
    for ty in 0..at.height {
        for tx in 0..at.width {
            let t = (ty * at.width + tx) as usize;
            let a = f64::from(values[t]);
            let i = f64::from(y_plane.get(at.x + tx, at.y + ty));
            y_in.push(i);
            y_out.push(if (floor..opaque).contains(&a) {
                let ly = logo_y.as_ref().map_or(l[0], |v| v[t]);
                (i - bias[0] - a * ly) / (1.0 - a)
            } else {
                i
            });
        }
    }
    let n = blocks.alpha.len();
    let (mut cb_in, mut cb_out, mut cr_in, mut cr_out) = (
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
    );
    let mut max_dev = 0f64;
    for b in 0..n {
        let (qx, qy) = blocks.coords(b);
        let a = blocks.alpha[b];
        let cb = f64::from(model.cb().get(qx, qy));
        let cr = f64::from(model.cr().get(qx, qy));
        cb_in.push(cb);
        cr_in.push(cr);
        if (floor..opaque).contains(&a) {
            let lb = logo_blocks.as_ref().map_or(l, |v| v[b]);
            cb_out.push((cb - bias[1] - a * lb[1]) / (1.0 - a));
            cr_out.push((cr - bias[2] - a * lb[2]) / (1.0 - a));
        } else {
            cb_out.push(cb);
            cr_out.push(cr);
        }
        if a >= floor {
            max_dev = max_dev.max(blocks.deviation[b]);
        }
    }
    Inverse {
        at,
        sampling: model.planes.sampling(),
        alpha: values.to_vec(),
        y_in,
        y_out,
        blocks: PixelRect {
            x: blocks.qx0,
            y: blocks.qy0,
            width: blocks.qw,
            height: blocks.qh,
        },
        alpha_mean: blocks.alpha,
        cb_in,
        cb_out,
        cr_in,
        cr_out,
        logo: l,
        logo_y,
        logo_blocks,
        bias,
        opaque,
        block: (blocks.sx, blocks.sy),
        max_alpha_dev_in_block: max_dev,
    }
}

/// [`crate::restore`] in the planes: invert, upsample, recombine, and
/// write the rounded and clamped RGB where [`Inverse::writes`] says; then
/// the capture noise (D246) and the outline (D238, D244, D247, D250) as
/// the RGB restoration takes them.
pub(crate) fn restore(
    raster: &mut Raster,
    model: &Model<'_>,
    verified: &Verified,
    options: &ExamineOptions,
) -> Result<Restored, RestoreError> {
    if !verified.fits(raster) {
        return Err(RestoreError::Elsewhere);
    }
    let inverse = invert_with(model, verified);
    let at = inverse.at;
    let chroma = (model.cb().width(), model.cb().height());
    let max = f64::from(raster.layout().max());
    let (mut changed, mut holes, mut clamped) = (0u32, 0u32, 0u32);
    for y in at.y..at.y + at.height {
        for x in at.x..at.x + at.width {
            if inverse.hole(x, y) {
                holes += 1;
                continue;
            }
            if !inverse.writes(x, y) {
                continue;
            }
            let i = raster.at(x, y);
            let samples = raster.samples_mut();
            let mut moved = false;
            for (c, o) in inverse.rgb_at(x, y, chroma).into_iter().enumerate() {
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
    let holes_chroma = inverse
        .alpha_mean
        .iter()
        .filter(|&&a| a >= inverse.opaque)
        .count() as u32;
    // How far the result is from the data (D305), in the planes the file
    // stored: Y at full resolution with `α`, Cb and Cr at their own with
    // `ᾱ`, clamped samples and holes left out — measured before the
    // capture noise is taken off in RGB.
    let back = inverse.blend_back(raster, chroma);
    let consistency = crate::verify::consistency(back.pairs, back.excluded);
    // The capture noise the template dropped, where it is drawn after all
    // (D246) — a few levels at most, taken off in RGB as the old path
    // does, after the inverse.
    let noise = crate::restore::drawn_noise(raster, verified);
    if noise {
        changed += take_noise(raster, verified);
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
        texture_left: options.source == Fidelity::Lossy && outline.textured(),
        noise,
        lossy: options.source == Fidelity::Lossy,
        fitted: verified.fitted(),
        resampled: verified.resampled(),
        searched: verified.searched(),
        // Only a lossy source takes this path.
        exact: false,
        consistency_px: consistency.px,
        consistency_excluded: consistency.excluded,
        consistency_dct: None,
        planar: Some(Planar::Inverse {
            sampling: inverse.sampling,
            max_alpha_dev_in_block: inverse.max_alpha_dev_in_block as f32,
            holes_chroma,
        }),
        smoothed: options.source == Fidelity::Lossy && outline.smoothed(),
        interval: None,
    })
}

/// The dropped capture noise taken off in RGB, as `restore` does once it
/// finds it drawn (D246): per noise pixel at the floor or over,
/// `unblend`, rounded and clamped. The pixels it moved.
fn take_noise(raster: &mut Raster, verified: &Verified) -> u32 {
    let at = verified.pixels();
    let max = f64::from(raster.layout().max());
    let mut changed = 0;
    for (p, &a) in verified.noise().iter().enumerate() {
        if a < NOISE_FLOOR {
            continue;
        }
        let (tx, ty) = (p as u32 % at.width, p as u32 / at.width);
        let i = raster.at(at.x + tx, at.y + ty);
        let samples = raster.samples_mut();
        let stored = [0, 1, 2].map(|c| f64::from(samples[i + c]));
        let mut moved = false;
        for (c, o) in verified
            .unblend(stored, f64::from(a), p)
            .into_iter()
            .enumerate()
        {
            let v = o.round().clamp(0.0, max) as u16;
            if v != samples[i + c] {
                samples[i + c] = v;
                moved = true;
            }
        }
        changed += u32::from(moved);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_matrices_are_each_others_inverse_to_the_decoders_precision() {
        for c in [
            [0.0, 0.0, 0.0],
            [255.0, 255.0, 255.0],
            [9.0, 150.0, 56.0],
            [252.1, 253.5, 252.8],
        ] {
            let back = rgb(ycc(c));
            for k in 0..3 {
                assert!((back[k] - c[k]).abs() < 0.05, "{c:?} → {back:?}");
            }
        }
    }

    #[test]
    fn a_gap_is_the_distance_outside_the_interval() {
        assert_eq!(gap(100.0, 0.5, 200.0), 0.0);
        assert_eq!(gap(90.0, 0.5, 200.0), 10.0);
        assert_eq!(gap(230.0, 0.5, 200.0), 2.5);
    }

    #[test]
    fn planar_serialises_as_its_object_or_its_word() {
        let inverse = Planar::Inverse {
            sampling: Sampling::H420,
            max_alpha_dev_in_block: 0.25,
            holes_chroma: 0,
        };
        assert_eq!(
            serde_json::to_string(&inverse).unwrap(),
            r#"{"sampling":"4:2:0","max_alpha_dev_in_block":0.25,"holes_chroma":0}"#
        );
        assert_eq!(
            serde_json::to_string(&Planar::Unavailable).unwrap(),
            r#""unavailable""#
        );
    }
}
