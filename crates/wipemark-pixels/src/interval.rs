//! The value chosen inside the codec's interval (E12-R8): on a **lossy**
//! source, the restored value moved — within what the file's codec could
//! have stored — towards the one whose restoration has the least block
//! structure.
//!
//! A lossy codec stored, for every coefficient, an interval; the decoded
//! value is one point of it. The inverse `O = (I − α·L)/(1 − α)` amplifies
//! the codec's error by `1/(1 − α)` — up to ×2.06 under Gemini's mark — and
//! on a 4:4:4 JPEG at 95 that is the 8 × 8 checker D250 says (`texture`
//! 8.6–9.2 against a bound of 5.5). Three ways to choose another point of
//! the data, each starting from today's restoration (R6's on a subsampled
//! JPEG) and never stepping outside what the file says:
//!
//! * **[`Method::Dct`]** (R4d of the spec), DCT-POCS: projections between
//!   the quantisation intervals of every 8 × 8 block of every plane the
//!   mark touches — `[(q − ½)·Q, (q + ½)·Q]`, `q` recomputed from the
//!   decoded plane — and an edge-preserving smoothness; the last operation
//!   of every round is the data projection, so the result is consistent
//!   with the file by construction (`consistency_dct`). JPEG only.
//! * **[`Method::Pixel`]** (R3), pixel POCS: the same smoothness against a
//!   per-sample interval `I ± h`, `h = 0.5 + 2·σ_base`, which needs only a
//!   noise estimate — a JPEG or a lossy WebP.
//! * **[`Method::Wiener`]** (R3w): one step, no interval kept.
//!
//! **On a lossless source none of them ever runs** (S6): there the stored
//! value is the composite to half a level, and `exact` means something.
//!
//! The working space is the file's own: a JPEG whose planes are known is
//! refined **in its planes** — Y at full resolution with `α`, Cb and Cr at
//! their own with the block's mean `ᾱ` (R6's model; at 4:4:4 a block is a
//! pixel) — and written back through the decoder's upsampler and colour
//! constants ([`crate::planar::Inverse::rgb_at`]); anything else (a lossy
//! WebP, a JPEG whose planes did not read) in RGB. Only the samples the
//! restoration writes move; everything else is the file's, and is held as
//! the data's anchor.
//!
//! Nothing in the product calls this until the method is decided (S12):
//! [`crate::clean_refined`] with a [`Refine`] other than `None` is reached
//! from `recon_bench --config R8d|R8p|R8w` and from `wipemark-picture`'s
//! `planar-preview` build with `WIPEMARK_INTERVAL` set. Plan:
//! `docs/plan/E12-R8-value-inside-the-interval.md`.

use serde::Serialize;

use crate::geometry::PixelRect;
use crate::planar::{ycc, Inverse};
use crate::planes::{Plane, Planes, Sampling};
use crate::raster::{Layout, Raster};
use crate::restore::{unblend, Restored};
use crate::verify::{consistency, outline, Consistency, Verified, NOISE_FLOOR};
use crate::{ExamineOptions, Fidelity};

/// How a restoration on a lossy source is refined (E12-R8). `None` is the
/// product today, and every caller's default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Refine {
    #[default]
    None,
    /// [`Method::Dct`].
    Dct,
    /// [`Method::Pixel`].
    Pixel,
    /// [`Method::Wiener`].
    Wiener,
}

impl Refine {
    /// `none`, `dct`, `pixel` or `wiener` — the spelling of the bench's
    /// configs and of `WIPEMARK_INTERVAL`.
    pub fn parse(word: &str) -> Option<Refine> {
        match word {
            "none" => Some(Refine::None),
            "dct" => Some(Refine::Dct),
            "pixel" => Some(Refine::Pixel),
            "wiener" => Some(Refine::Wiener),
            _ => None,
        }
    }

    fn method(self) -> Option<Method> {
        match self {
            Refine::None => None,
            Refine::Dct => Some(Method::Dct),
            Refine::Pixel => Some(Method::Pixel),
            Refine::Wiener => Some(Method::Wiener),
        }
    }
}

/// What a restoration does beyond the inverse. The default is the
/// product's: no refinement.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RestoreOptions {
    pub refine: Refine,
}

/// The method a refined restoration was made with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Method {
    /// DCT-POCS over the quantisation intervals (R4d).
    Dct,
    /// POCS over a per-sample interval (R3).
    Pixel,
    /// One Wiener step (R3w).
    Wiener,
}

/// The space a refinement worked in, and its noise was measured in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Space {
    /// A JPEG's stored planes: Y, Cb, Cr, each at its own resolution.
    #[serde(rename = "ycbcr")]
    Planes,
    /// The decoded raster's R, G, B.
    #[serde(rename = "rgb")]
    Rgb,
}

/// `Restored::interval`: how a restoration was refined.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Interval {
    pub method: Method,
    /// Where `sigma_base` was measured and the refinement ran.
    pub space: Space,
    /// The input's noise per channel of `space`, in 8-bit levels
    /// ([`sigma_base`]'s estimate, on the ring around the mark).
    pub sigma_base: [f32; 3],
    /// Whether the restored region read as text ([`TEXT_RATIO`]), and was
    /// smoothed at half the radius and half the `eps`.
    pub text: bool,
    /// The rounds kept: up to [`DCT_ROUNDS`] or [`PIXEL_ROUNDS`], fewer when
    /// the stop rule held or a round was taken back — 0, and today's
    /// restoration byte for byte, when none was kept; 1 for Wiener.
    pub iterations: u8,
}

/// How far outside the mark's rectangle the noise is measured: the ring of
/// samples two to this many outside it, in the channel's own grid at full
/// resolution (half of it in a subsampled chroma plane).
pub const NOISE_RING: u32 = 8;
/// The smoothness `P_S`'s guided filter: its radius, in samples of the
/// channel's own grid. `[tunable]`
pub const SMOOTH_RADIUS: usize = 4;
/// … its `eps`, (4 levels)². `[tunable]`
pub const SMOOTH_EPS: f64 = 16.0;
/// On text — the restored region's Laplacian energy over this many times
/// the ring's — the filter's radius is 2 and its `eps` half. `[tunable]`
pub const TEXT_RATIO: f64 = 1.5;
/// DCT-POCS's rounds. `[tunable]`
pub const DCT_ROUNDS: u8 = 4;
/// Pixel POCS's rounds. `[tunable]`
pub const PIXEL_ROUNDS: u8 = 3;
/// The stop rule: the restoration's roughness against the picture's around
/// it inside this band. `[tunable]`
pub const STOP_RATIO: [f32; 2] = [0.8, 1.2];
/// … or the estimate moved by less than this, in levels, at p95.
/// `[tunable]`
pub const STOP_MOVE: f64 = 0.1;
/// Pixel POCS's interval half-width, `h = H_BASE + H_SIGMAS·σ_base`.
/// `[tunable]`
pub const H_BASE: f64 = 0.5;
/// See [`H_BASE`]. `[tunable]`
pub const H_SIGMAS: f64 = 2.0;
/// The margin, in samples, the region keeps around the codec's blocks for
/// the filter's reach.
const MARGIN: u32 = 8;
/// The data projection's inner rounds per block, and when it has
/// converged: the largest distance of a coefficient outside its interval.
const INNER: usize = 5000;
const INNER_TOL: f64 = 1e-6;
/// A coefficient further than this outside its interval counts in
/// `consistency_dct`.
const DCT_TOL: f64 = 1e-4;

// ───────────────────────────────────────────────────────── the transform

/// `C(u)/2 · cos((2x + 1)uπ/16)`: the orthonormal DCT-II's basis, which is
/// JPEG's FDCT (`F(u, v) = ¼·C(u)·C(v)·Σ f·cos·cos`).
fn basis() -> [[f64; 8]; 8] {
    let mut b = [[0f64; 8]; 8];
    for (u, row) in b.iter_mut().enumerate() {
        let c = if u == 0 { 0.5 / 2f64.sqrt() } else { 0.5 };
        for (x, v) in row.iter_mut().enumerate() {
            *v = c * ((2 * x + 1) as f64 * u as f64 * std::f64::consts::PI / 16.0).cos();
        }
    }
    b
}

/// The orthonormal 8 × 8 DCT-II in `f64`: `block` row-major (`y·8 + x`),
/// the coefficients in natural order (`v·8 + u`, `v` the vertical
/// frequency) — the order of [`crate::Quant`]'s tables.
#[doc(hidden)]
pub fn dct8(block: &[f64; 64]) -> [f64; 64] {
    let b = basis();
    let mut t = [0f64; 64];
    for y in 0..8 {
        for u in 0..8 {
            t[y * 8 + u] = (0..8).map(|x| b[u][x] * block[y * 8 + x]).sum();
        }
    }
    let mut out = [0f64; 64];
    for v in 0..8 {
        for u in 0..8 {
            out[v * 8 + u] = (0..8).map(|y| b[v][y] * t[y * 8 + u]).sum();
        }
    }
    out
}

/// [`dct8`]'s inverse.
#[doc(hidden)]
pub fn idct8(coef: &[f64; 64]) -> [f64; 64] {
    let b = basis();
    let mut t = [0f64; 64];
    for y in 0..8 {
        for u in 0..8 {
            t[y * 8 + u] = (0..8).map(|v| b[v][y] * coef[v * 8 + u]).sum();
        }
    }
    let mut out = [0f64; 64];
    for y in 0..8 {
        for x in 0..8 {
            out[y * 8 + x] = (0..8).map(|u| b[u][x] * t[y * 8 + u]).sum();
        }
    }
    out
}

/// The quantisation indices of the whole 8 × 8 block at `(bx, by)` (in
/// blocks) of `plane`, recomputed from the decoded samples:
/// `q_k = round(DCT_k(I − 128) / Q_k)`, natural order. The decoder exports
/// no coefficients (D302), so this is the data §4.2 projects onto; how
/// often it is the file's own is held by
/// `the_recomputed_coefficients_are_the_files`.
#[doc(hidden)]
pub fn indices(plane: &Plane, table: &[u16; 64], bx: u32, by: u32) -> [i32; 64] {
    let mut block = [0f64; 64];
    for y in 0..8u32 {
        for x in 0..8u32 {
            block[(y * 8 + x) as usize] = f64::from(plane.get(bx * 8 + x, by * 8 + y));
        }
    }
    quantised(&block, table)
}

/// `round(DCT(block − 128) / Q)`, natural order: the one place a block's
/// indices are recomputed — [`indices`] and the data projection's
/// intervals both read it.
fn quantised(block: &[f64; 64], table: &[u16; 64]) -> [i32; 64] {
    let c = dct8(&block.map(|v| v - 128.0));
    let mut q = [0i32; 64];
    for k in 0..64 {
        q[k] = (c[k] / f64::from(table[k])).round() as i32;
    }
    q
}

// ───────────────────────────────────────────────────────── the noise

/// The noise of the input around the mark, per channel, in 8-bit levels:
/// over the ring of pixels two to [`NOISE_RING`] outside `rect` — where no
/// mark is, and whose 3 × 3 Laplacian `[0,1,0; 1,−4,1; 0,1,0]` reads no
/// pixel of it — `1.4826 · median|ΔI| / √20` (the Laplacian's weights'
/// squares sum to 20, so a white noise of σ reads σ). A texture reads as
/// noise too: it is an estimate of what is rough here, not of the codec
/// alone.
pub fn sigma_base(raster: &Raster, rect: PixelRect) -> [f32; 3] {
    let scale = 255.0 / f64::from(raster.layout().max());
    let s = raster.samples();
    [0, 1, 2].map(|c| {
        let get = |x: u32, y: u32| f64::from(s[raster.at(x, y) + c]) * scale;
        let ring = ring_laplacians(&get, raster.width(), raster.height(), rect, NOISE_RING);
        mad_sigma(ring) as f32
    })
}

/// The Laplacian of every sample two to `ring` outside `rect` (Chebyshev
/// distance) whose four neighbours are inside the grid.
fn ring_laplacians(
    get: &dyn Fn(u32, u32) -> f64,
    w: u32,
    h: u32,
    rect: PixelRect,
    ring: u32,
) -> Vec<f64> {
    let (x0, y0) = (i64::from(rect.x), i64::from(rect.y));
    let (x1, y1) = (x0 + i64::from(rect.width), y0 + i64::from(rect.height));
    let ring = i64::from(ring);
    let mut out = Vec::new();
    for y in (y0 - ring).max(1)..(y1 + ring).min(i64::from(h) - 1) {
        for x in (x0 - ring).max(1)..(x1 + ring).min(i64::from(w) - 1) {
            let outside = (x0 - x).max(x - (x1 - 1)).max(y0 - y).max(y - (y1 - 1));
            if !(2..=ring).contains(&outside) {
                continue;
            }
            let (ux, uy) = (x as u32, y as u32);
            out.push(
                get(ux - 1, uy) + get(ux + 1, uy) + get(ux, uy - 1) + get(ux, uy + 1)
                    - 4.0 * get(ux, uy),
            );
        }
    }
    out
}

/// `1.4826 · median|v| / √20`; 0 for none.
fn mad_sigma(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    for x in &mut v {
        *x = x.abs();
    }
    v.sort_by(f64::total_cmp);
    1.4826 * v[v.len() / 2] / 20f64.sqrt()
}

/// The variance of the Laplacian over `√20`: what is rough on the ring,
/// signal and noise — Wiener's `s`.
fn hp_variance(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let v: Vec<f64> = v.iter().map(|x| x / 20f64.sqrt()).collect();
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / v.len() as f64
}

// ───────────────────────────────────────────────────────── the smoothness

/// The mean over a `(2r + 1)²` window, clipped to the grid.
fn box_mean(v: &[f64], w: usize, h: usize, r: usize) -> Vec<f64> {
    let mut sum = vec![0f64; (w + 1) * (h + 1)];
    for y in 0..h {
        for x in 0..w {
            sum[(y + 1) * (w + 1) + x + 1] =
                v[y * w + x] + sum[y * (w + 1) + x + 1] + sum[(y + 1) * (w + 1) + x]
                    - sum[y * (w + 1) + x];
        }
    }
    let mut out = vec![0f64; w * h];
    for y in 0..h {
        let (ya, yb) = (y.saturating_sub(r), (y + r + 1).min(h));
        for x in 0..w {
            let (xa, xb) = (x.saturating_sub(r), (x + r + 1).min(w));
            let s = sum[yb * (w + 1) + xb] - sum[ya * (w + 1) + xb] - sum[yb * (w + 1) + xa]
                + sum[ya * (w + 1) + xa];
            out[y * w + x] = s / ((yb - ya) * (xb - xa)) as f64;
        }
    }
    out
}

/// He's guided filter of `p` guided by itself: per window `a = σ²/(σ² +
/// eps)`, `b = μ·(1 − a)`, and `q = mean(a)·p + mean(b)` — a flat window is
/// averaged, an edge (`σ² ≫ eps`) kept.
fn guided(p: &[f64], w: usize, h: usize, r: usize, eps: f64) -> Vec<f64> {
    let mean = box_mean(p, w, h, r);
    let sq: Vec<f64> = p.iter().map(|v| v * v).collect();
    let mean_sq = box_mean(&sq, w, h, r);
    let mut a = vec![0f64; w * h];
    let mut b = vec![0f64; w * h];
    for i in 0..w * h {
        let var = (mean_sq[i] - mean[i] * mean[i]).max(0.0);
        a[i] = var / (var + eps);
        b[i] = mean[i] * (1.0 - a[i]);
    }
    let (ma, mb) = (box_mean(&a, w, h, r), box_mean(&b, w, h, r));
    (0..w * h).map(|i| ma[i] * p[i] + mb[i]).collect()
}

/// `P_S`, the smoothness: over a channel's region, `o' = r·o + (1 − r)·GF(o)`
/// with the reliability `r = (1 − α)²` — the inverse amplified the codec's
/// error by `1/(1 − α)`, so the less a sample is to be believed the more it
/// is smoothed — on the samples the restoration writes; every other sample
/// (the ring, `r = 1`) is the anchor and does not move.
#[doc(hidden)]
pub fn smooth(
    o: &[f64],
    (w, h): (usize, usize),
    alpha: &[f64],
    free: &[bool],
    radius: usize,
    eps: f64,
) -> Vec<f64> {
    let gf = guided(o, w, h, radius, eps);
    (0..w * h)
        .map(|i| {
            if free[i] {
                let r = (1.0 - alpha[i]).powi(2);
                r * o[i] + (1.0 - r) * gf[i]
            } else {
                o[i]
            }
        })
        .collect()
}

// ───────────────────────────────────────────────────────── the work

/// One channel of the working space over a region of its own grid.
struct Channel {
    /// The region's origin and size, in the channel's grid.
    x0: u32,
    y0: u32,
    w: usize,
    h: usize,
    /// The whole grid's size: a block past it is not whole.
    grid: (u32, u32),
    /// The mark's rectangle in this grid.
    mark: PixelRect,
    /// The ring's width in this grid.
    ring: u32,
    /// The estimate, the file's value, `α` (or the block's `ᾱ`), and
    /// whether the restoration writes the sample — all over the region, in
    /// 8-bit levels.
    o: Vec<f64>,
    stored: Vec<f64>,
    alpha: Vec<f64>,
    free: Vec<bool>,
    logo: f64,
    /// The quantisation table, for the planes.
    table: Option<[u16; 64]>,
}

impl Channel {
    fn index(&self, x: u32, y: u32) -> usize {
        (y - self.y0) as usize * self.w + (x - self.x0) as usize
    }

    fn forward(&self, i: usize) -> f64 {
        self.alpha[i] * self.logo + (1.0 - self.alpha[i]) * self.o[i]
    }

    fn unblend(&self, i: usize, composite: f64) -> f64 {
        (composite - self.alpha[i] * self.logo) / (1.0 - self.alpha[i])
    }

    /// The Laplacians of the file's samples on the ring.
    fn ring(&self) -> Vec<f64> {
        let get = |x: u32, y: u32| {
            if x < self.x0 || y < self.y0 {
                return f64::NAN;
            }
            let (rx, ry) = ((x - self.x0) as usize, (y - self.y0) as usize);
            if rx >= self.w || ry >= self.h {
                return f64::NAN;
            }
            self.stored[ry * self.w + rx]
        };
        // The region's own edge stands in for the grid's: a Laplacian that
        // reaches past it is not taken.
        ring_laplacians(&get, u32::MAX, u32::MAX, self.mark, self.ring)
            .into_iter()
            .filter(|v| v.is_finite())
            .collect()
    }

    /// The whole blocks of the grid that hold a sample the restoration
    /// writes, in blocks.
    fn blocks(&self) -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        let (bx0, by0) = (self.x0 / 8, self.y0 / 8);
        let (bx1, by1) = (
            (self.x0 + self.w as u32).div_ceil(8),
            (self.y0 + self.h as u32).div_ceil(8),
        );
        for by in by0..by1 {
            for bx in bx0..bx1 {
                let inside = bx * 8 >= self.x0
                    && by * 8 >= self.y0
                    && bx * 8 + 8 <= self.x0 + self.w as u32
                    && by * 8 + 8 <= self.y0 + self.h as u32
                    && bx * 8 + 8 <= self.grid.0
                    && by * 8 + 8 <= self.grid.1;
                if inside && self.block_free(bx, by) {
                    out.push((bx, by));
                }
            }
        }
        out
    }

    fn block_free(&self, bx: u32, by: u32) -> bool {
        (0..64).any(|k| self.free[self.index(bx * 8 + k % 8, by * 8 + k / 8)])
    }

    /// Free samples in a block that is not whole — at the grid's edge, where
    /// the decoder cropped what the coefficients cover — are not written:
    /// no interval is known for them.
    fn freeze_partial_blocks(&mut self) {
        for y in 0..self.h {
            for x in 0..self.w {
                let (gx, gy) = (self.x0 + x as u32, self.y0 + y as u32);
                if (gx / 8) * 8 + 8 > self.grid.0 || (gy / 8) * 8 + 8 > self.grid.1 {
                    self.free[y * self.w + x] = false;
                }
            }
        }
    }

    /// The composite of block `(bx, by)`: the estimate blended forward where
    /// the restoration writes, the file's value elsewhere.
    fn composite(&self, bx: u32, by: u32) -> [f64; 64] {
        let mut out = [0f64; 64];
        for (k, v) in out.iter_mut().enumerate() {
            let i = self.index(bx * 8 + k as u32 % 8, by * 8 + k as u32 / 8);
            *v = if self.free[i] {
                self.forward(i)
            } else {
                self.stored[i]
            };
        }
        out
    }

    /// The interval of every coefficient of block `(bx, by)`, from the
    /// file's samples: `[(q − ½)·Q, (q + ½)·Q]`, `q` recomputed.
    fn intervals(&self, table: &[u16; 64], bx: u32, by: u32) -> [(f64, f64); 64] {
        let mut block = [0f64; 64];
        for (k, v) in block.iter_mut().enumerate() {
            *v = self.stored[self.index(bx * 8 + k as u32 % 8, by * 8 + k as u32 / 8)];
        }
        let q = quantised(&block, table);
        let mut out = [(0f64, 0f64); 64];
        for k in 0..64 {
            let step = f64::from(table[k]);
            let centre = f64::from(q[k]) * step;
            out[k] = (centre - step / 2.0, centre + step / 2.0);
        }
        out
    }

    /// `P_D`, the data projection, on every whole block the restoration
    /// writes in: the composite's coefficients clamped into their
    /// intervals and transformed back, the samples it does not write put
    /// back to the file's — repeated until both hold — and the estimate
    /// unblended from it.
    fn project_dct(&mut self) {
        let Some(table) = self.table else { return };
        for (bx, by) in self.blocks() {
            let bounds = self.intervals(&table, bx, by);
            let mut x = self.composite(bx, by);
            for _ in 0..INNER {
                let mut c = dct8(&x.map(|v| v - 128.0));
                let mut worst = 0f64;
                for k in 0..64 {
                    let (lo, hi) = bounds[k];
                    worst = worst.max(lo - c[k]).max(c[k] - hi);
                    c[k] = c[k].clamp(lo, hi);
                }
                if worst <= INNER_TOL {
                    break;
                }
                let back = idct8(&c);
                for k in 0..64 {
                    let i = self.index(bx * 8 + k as u32 % 8, by * 8 + k as u32 / 8);
                    x[k] = if self.free[i] {
                        back[k] + 128.0
                    } else {
                        self.stored[i]
                    };
                }
            }
            for (k, &v) in x.iter().enumerate() {
                let i = self.index(bx * 8 + k as u32 % 8, by * 8 + k as u32 / 8);
                if self.free[i] {
                    self.o[i] = self.unblend(i, v);
                }
            }
        }
    }

    /// Of every coefficient of every whole block the restoration writes in,
    /// how many lie outside their interval — `(outside, all)`.
    fn outside(&self) -> (usize, usize) {
        let Some(table) = self.table else {
            return (0, 0);
        };
        let (mut out, mut all) = (0, 0);
        for (bx, by) in self.blocks() {
            let bounds = self.intervals(&table, bx, by);
            let c = dct8(&self.composite(bx, by).map(|v| v - 128.0));
            for k in 0..64 {
                let (lo, hi) = bounds[k];
                all += 1;
                out += usize::from(c[k] < lo - DCT_TOL || c[k] > hi + DCT_TOL);
            }
        }
        (out, all)
    }
}

/// What the refined channels are written back through.
enum Base {
    /// The planes: the refined Y, Cb and Cr go into an inverse in R6's
    /// shape and are written by its upsampler and colour constants.
    /// `rgb_measure` when the base restoration was the RGB one (a 4:4:4
    /// JPEG), whose consistency is measured in RGB, as R0's.
    Planes {
        inverse: Box<Inverse>,
        chroma: (u32, u32),
        rgb_measure: bool,
    },
    /// R, G and B, written rounded and clamped.
    Rgb,
}

struct Work {
    space: Space,
    channels: Vec<Channel>,
    base: Base,
    /// 8-bit levels per stored level.
    scale: f64,
}

/// The region of the picture a refinement reads: the mark's rectangle
/// widened to the codec's grid — `(8·sx, 8·sy)`, so that every block of
/// every plane it touches is whole — in the picture's coordinates.
fn roi(rect: PixelRect, (gx, gy): (u32, u32), (w, h): (u32, u32)) -> [u32; 4] {
    let x0 = rect.x / gx * gx;
    let y0 = rect.y / gy * gy;
    let x1 = ((rect.x + rect.width).div_ceil(gx) * gx).min(w);
    let y1 = ((rect.y + rect.height).div_ceil(gy) * gy).min(h);
    [x0, y0, x1, y1]
}

/// `roi` in a grid scaled by `(sx, sy)`, with [`MARGIN`] around it, inside
/// the grid `(gw, gh)`: `(x0, y0, w, h)`.
fn region(roi: [u32; 4], (sx, sy): (u32, u32), (gw, gh): (u32, u32)) -> (u32, u32, usize, usize) {
    let x0 = (roi[0] / sx).saturating_sub(MARGIN);
    let y0 = (roi[1] / sy).saturating_sub(MARGIN);
    let x1 = (roi[2].div_ceil(sx) + MARGIN).min(gw);
    let y1 = (roi[3].div_ceil(sy) + MARGIN).min(gh);
    (x0, y0, (x1 - x0) as usize, (y1 - y0) as usize)
}

/// `rect` in a grid scaled by `(sx, sy)`.
fn scaled(rect: PixelRect, (sx, sy): (u32, u32)) -> PixelRect {
    let (x, y) = (rect.x / sx, rect.y / sy);
    PixelRect {
        x,
        y,
        width: (rect.x + rect.width).div_ceil(sx) - x,
        height: (rect.y + rect.height).div_ceil(sy) - y,
    }
}

impl Work {
    /// The planes, from an inverse in R6's shape: Y with `α` per pixel, Cb
    /// and Cr with the block's `ᾱ` (at 4:4:4 a block is a pixel).
    fn planes(
        inverse: Inverse,
        planes: &Planes,
        method: Method,
        rgb_measure: bool,
    ) -> Option<Work> {
        let (cb, cr) = (planes.cb()?, planes.cr()?);
        let (sx, sy) = inverse.block;
        let size = (planes.width(), planes.height());
        let roi = roi(inverse.at, (8 * sx, 8 * sy), size);
        let floor = f64::from(NOISE_FLOOR);
        let opaque = inverse.opaque;
        let tables = [planes.quant().luma, planes.quant().chroma?];
        let mut channels = Vec::with_capacity(3);
        // Y.
        let (x0, y0, w, h) = region(roi, (1, 1), size);
        let mut y = Channel {
            x0,
            y0,
            w,
            h,
            grid: size,
            mark: inverse.at,
            ring: NOISE_RING,
            o: Vec::with_capacity(w * h),
            stored: Vec::with_capacity(w * h),
            alpha: Vec::with_capacity(w * h),
            free: Vec::with_capacity(w * h),
            logo: inverse.logo[0],
            table: Some(tables[0]),
        };
        let at = inverse.at;
        for py in y0..y0 + h as u32 {
            for px in x0..x0 + w as u32 {
                let stored = f64::from(planes.y().get(px, py));
                let a = inverse.alpha_at(px, py);
                let inside =
                    px >= at.x && py >= at.y && px < at.x + at.width && py < at.y + at.height;
                y.stored.push(stored);
                y.alpha.push(a);
                y.free
                    .push((floor..opaque).contains(&a) && !inverse.hole(px, py));
                y.o.push(if inside {
                    inverse.y_out[((py - at.y) * at.width + (px - at.x)) as usize]
                } else {
                    stored
                });
            }
        }
        channels.push(y);
        // Cb and Cr, at their own resolution.
        let csize = (cb.width(), cb.height());
        for (c, plane, out) in [(1, cb, &inverse.cb_out), (2, cr, &inverse.cr_out)] {
            let (x0, y0, w, h) = region(roi, (sx, sy), csize);
            let mut ch = Channel {
                x0,
                y0,
                w,
                h,
                grid: csize,
                mark: scaled(at, (sx, sy)),
                ring: (NOISE_RING / sx.max(sy)).max(2),
                o: Vec::with_capacity(w * h),
                stored: Vec::with_capacity(w * h),
                alpha: Vec::with_capacity(w * h),
                free: Vec::with_capacity(w * h),
                logo: inverse.logo[c],
                table: Some(tables[1]),
            };
            for qy in y0..y0 + h as u32 {
                for qx in x0..x0 + w as u32 {
                    let stored = f64::from(plane.get(qx, qy));
                    let b = inverse.block_index(qx, qy);
                    let a = b.map_or(0.0, |b| inverse.alpha_mean[b]);
                    ch.stored.push(stored);
                    ch.alpha.push(a);
                    ch.free.push((floor..opaque).contains(&a));
                    ch.o.push(b.map_or(stored, |b| out[b]));
                }
            }
            channels.push(ch);
        }
        if method == Method::Dct {
            for ch in &mut channels {
                ch.freeze_partial_blocks();
            }
        }
        Some(Work {
            space: Space::Planes,
            channels,
            base: Base::Planes {
                inverse: Box::new(inverse),
                chroma: csize,
                rgb_measure,
            },
            scale: 1.0,
        })
    }

    /// R, G and B of the restored raster, the file's being `input`.
    fn rgb(restored: &Raster, input: &Raster, verified: &Verified) -> Work {
        let at = verified.pixels();
        let size = (restored.width(), restored.height());
        let roi = roi(at, (8, 8), size);
        let (x0, y0, w, h) = region(roi, (1, 1), size);
        let scale = 255.0 / f64::from(restored.layout().max());
        let floor = f64::from(NOISE_FLOOR);
        let opaque = f64::from(verified.opaque_above());
        let alpha_at = |x: u32, y: u32| {
            if x >= at.x && y >= at.y && x < at.x + at.width && y < at.y + at.height {
                f64::from(verified.values()[((y - at.y) * at.width + (x - at.x)) as usize])
            } else {
                0.0
            }
        };
        let channels = (0..3)
            .map(|c| {
                let mut ch = Channel {
                    x0,
                    y0,
                    w,
                    h,
                    grid: size,
                    mark: at,
                    ring: NOISE_RING,
                    o: Vec::with_capacity(w * h),
                    stored: Vec::with_capacity(w * h),
                    alpha: Vec::with_capacity(w * h),
                    free: Vec::with_capacity(w * h),
                    logo: verified.logo()[c] * scale,
                    table: None,
                };
                for py in y0..y0 + h as u32 {
                    for px in x0..x0 + w as u32 {
                        let a = alpha_at(px, py);
                        let j = input.at(px, py);
                        let stored = [0, 1, 2].map(|k| f64::from(input.samples()[j + k]));
                        let free = (floor..opaque).contains(&a);
                        ch.stored.push(stored[c] * scale);
                        // The restoration's own value before its rounding
                        // and clamp (`restore::unblend`), where it wrote.
                        ch.o.push(if free {
                            unblend(stored, a, verified.logo())[c] * scale
                        } else {
                            f64::from(restored.samples()[restored.at(px, py) + c]) * scale
                        });
                        ch.alpha.push(a);
                        ch.free.push(free);
                    }
                }
                ch
            })
            .collect();
        Work {
            space: Space::Rgb,
            channels,
            base: Base::Rgb,
            scale,
        }
    }

    /// Whether the restored region is text: the Laplacian energy of the
    /// first channel (Y, or R) over the samples the restoration writes —
    /// each weighted by `1 − α`, which takes the inverse's amplification of
    /// the codec's error back off — against the ring's, over
    /// [`TEXT_RATIO`].
    fn text(&self) -> bool {
        let ch = &self.channels[0];
        let ring: Vec<f64> = ch.ring();
        if ring.is_empty() {
            return false;
        }
        let around = ring.iter().map(|v| v * v).sum::<f64>() / ring.len() as f64;
        let (mut sum, mut n) = (0f64, 0f64);
        for y in 1..ch.h.saturating_sub(1) {
            for x in 1..ch.w.saturating_sub(1) {
                let i = y * ch.w + x;
                if !ch.free[i] {
                    continue;
                }
                let lap =
                    ch.o[i - 1] + ch.o[i + 1] + ch.o[i - ch.w] + ch.o[i + ch.w] - 4.0 * ch.o[i];
                sum += ((1.0 - ch.alpha[i]) * lap).powi(2);
                n += 1.0;
            }
        }
        n > 0.0 && sum / n > TEXT_RATIO * around
    }

    /// `P_S` on every channel.
    fn smooth(&mut self, radius: usize, eps: f64) {
        for ch in &mut self.channels {
            ch.o = smooth(&ch.o, (ch.w, ch.h), &ch.alpha, &ch.free, radius, eps);
        }
    }

    fn snapshot(&self) -> Vec<f64> {
        self.channels
            .iter()
            .flat_map(|ch| (0..ch.o.len()).filter(|&i| ch.free[i]).map(|i| ch.o[i]))
            .collect()
    }

    /// The 95th percentile of how far the estimate moved since `before`.
    fn moved(&self, before: &[f64]) -> f64 {
        let mut d: Vec<f64> = self
            .snapshot()
            .iter()
            .zip(before)
            .map(|(a, b)| (a - b).abs())
            .collect();
        if d.is_empty() {
            return 0.0;
        }
        d.sort_by(f64::total_cmp);
        d[((d.len() - 1) as f64 * 0.95).round() as usize]
    }

    /// The refined channels into the inverse they came from.
    fn sync(&mut self) {
        let Base::Planes { inverse, .. } = &mut self.base else {
            return;
        };
        let at = inverse.at;
        let y = &self.channels[0];
        for py in at.y..at.y + at.height {
            for px in at.x..at.x + at.width {
                let i = y.index(px, py);
                if y.free[i] {
                    inverse.y_out[((py - at.y) * at.width + (px - at.x)) as usize] = y.o[i];
                }
            }
        }
        let blocks = inverse.blocks;
        for (c, ch) in self.channels.iter().enumerate().skip(1) {
            for qy in blocks.y..blocks.y + blocks.height {
                for qx in blocks.x..blocks.x + blocks.width {
                    if qx < ch.x0
                        || qy < ch.y0
                        || qx >= ch.x0 + ch.w as u32
                        || qy >= ch.y0 + ch.h as u32
                    {
                        continue;
                    }
                    let i = ch.index(qx, qy);
                    if !ch.free[i] {
                        continue;
                    }
                    let b = ((qy - blocks.y) * blocks.width + (qx - blocks.x)) as usize;
                    if c == 1 {
                        inverse.cb_out[b] = ch.o[i];
                    } else {
                        inverse.cr_out[b] = ch.o[i];
                    }
                }
            }
        }
    }

    /// The restoration's unrounded RGB at a pixel it writes, in 8-bit
    /// levels; `None` where it does not write.
    fn unrounded(&self, x: u32, y: u32, noise: &dyn Fn(u32, u32) -> bool) -> Option<[f64; 3]> {
        if noise(x, y) {
            return None;
        }
        match &self.base {
            Base::Planes {
                inverse, chroma, ..
            } => {
                (!inverse.hole(x, y) && inverse.writes(x, y)).then(|| inverse.rgb_at(x, y, *chroma))
            }
            Base::Rgb => {
                let ch = &self.channels;
                let i = ch[0].index(x, y);
                ch[0].free[i].then(|| [ch[0].o[i], ch[1].o[i], ch[2].o[i]])
            }
        }
    }

    /// Write the refined restoration into `raster`, rounded half away from
    /// zero and clamped, where the restoration writes and nowhere else; the
    /// samples clamped beyond rounding.
    fn render(
        &mut self,
        raster: &mut Raster,
        at: PixelRect,
        noise: &dyn Fn(u32, u32) -> bool,
    ) -> u32 {
        self.sync();
        let max = f64::from(raster.layout().max());
        let mut clamped = 0;
        for y in at.y..at.y + at.height {
            for x in at.x..at.x + at.width {
                let Some(o) = self.unrounded(x, y, noise) else {
                    continue;
                };
                let i = raster.at(x, y);
                let samples = raster.samples_mut();
                for (c, v) in o.into_iter().enumerate() {
                    let v = v / self.scale;
                    clamped += u32::from(v < -0.5 || v > max + 0.5);
                    samples[i + c] = v.round().clamp(0.0, max) as u16;
                }
            }
        }
        clamped
    }

    /// `consistency_px` of what [`Work::render`] wrote (D305), through R7's
    /// measure: in the planes when the base restoration measured it there
    /// (R6's), in RGB otherwise (R0's) — the restored samples blended back
    /// with the `α` and logo they were restored with, against the file.
    fn consistency(
        &self,
        raster: &Raster,
        input: &Raster,
        verified: &Verified,
        noise: bool,
    ) -> Consistency {
        if let Base::Planes {
            inverse,
            chroma,
            rgb_measure: false,
        } = &self.base
        {
            let back = inverse.blend_back(raster, *chroma);
            return consistency(back.pairs, back.excluded);
        }
        let at = verified.pixels();
        let max = f64::from(raster.layout().max());
        let to_8 = 255.0 / max;
        let logo = verified.logo();
        let opaque = verified.opaque_above();
        let (mut pairs, mut excluded) = (Vec::new(), 0u32);
        let mut holes = 0u32;
        let mut pair = |o: f64, a: f64, stored: f64, written: f64, l: f64| {
            if o < -0.5 || o > max + 0.5 {
                excluded += 1;
            } else {
                pairs.push(((a * l + (1.0 - a) * written) * to_8, stored * to_8));
            }
        };
        for ty in 0..at.height {
            for tx in 0..at.width {
                let a = verified.values()[(ty * at.width + tx) as usize];
                let (x, y) = (at.x + tx, at.y + ty);
                let (i, j) = (raster.at(x, y), input.at(x, y));
                if a >= opaque {
                    holes += 1;
                    continue;
                }
                let a = f64::from(a);
                let stored = [0, 1, 2].map(|c| f64::from(input.samples()[j + c]));
                let unrounded = if a >= f64::from(NOISE_FLOOR) {
                    self.unrounded(x, y, &|_, _| false)
                        .map(|o| o.map(|v| v / self.scale))
                } else if noise && verified.noise()[(ty * at.width + tx) as usize] >= NOISE_FLOOR {
                    // The capture noise the base restoration took off
                    // (D246), measured as it measured it.
                    let n = f64::from(verified.noise()[(ty * at.width + tx) as usize]);
                    for (c, o) in unblend(stored, n, logo).into_iter().enumerate() {
                        pair(o, n, stored[c], f64::from(raster.samples()[i + c]), logo[c]);
                    }
                    continue;
                } else {
                    None
                };
                let Some(o) = unrounded else { continue };
                for c in 0..3 {
                    pair(
                        o[c],
                        a,
                        stored[c],
                        f64::from(raster.samples()[i + c]),
                        logo[c],
                    );
                }
            }
        }
        consistency(pairs, excluded + 3 * holes)
    }
}

/// Refine `restored` — the restoration [`crate::restore`] or the planar
/// inverse just wrote into `raster` — by `refine`, the file's decoded
/// raster being `input` and its planes `planes`. Returns it as it came
/// when `refine` is `None`, when the source is not lossy (S6), and when the
/// method cannot run here (DCT-POCS without the planes); otherwise with the
/// refined samples written, and every measure taken again on them.
#[allow(clippy::too_many_arguments)]
pub(crate) fn refine(
    raster: &mut Raster,
    input: &Raster,
    planes: Option<&Planes>,
    verified: &Verified,
    options: &ExamineOptions,
    refine: Refine,
    restored: Restored,
) -> Restored {
    let Some(method) = refine.method() else {
        return restored;
    };
    // S6: a lossless source stored the composite to half a level; there is
    // no interval to choose in, and `exact` means something.
    if options.source != Fidelity::Lossy {
        return restored;
    }
    let Some(mut work) = setup(raster, input, planes, verified, method, &restored) else {
        return restored;
    };
    let at = verified.pixels();
    let noise_at = |x: u32, y: u32| {
        restored.noise
            && verified.noise()[((y - at.y) * at.width + (x - at.x)) as usize] >= NOISE_FLOOR
    };
    let sigma: Vec<f64> = work
        .channels
        .iter()
        .map(|ch| mad_sigma(ch.ring()))
        .collect();
    let text = work.text();
    let (radius, eps) = if text {
        (SMOOTH_RADIUS / 2, SMOOTH_EPS / 2.0)
    } else {
        (SMOOTH_RADIUS, SMOOTH_EPS)
    };
    let ratio = |raster: &Raster| {
        let o = outline(raster, verified);
        (o.texture_around > 0.0).then(|| o.texture / o.texture_around)
    };
    let in_band = |raster: &Raster| {
        ratio(raster).is_some_and(|r| (STOP_RATIO[0]..=STOP_RATIO[1]).contains(&r))
    };
    let under_band = |raster: &Raster| ratio(raster).is_some_and(|r| r < STOP_RATIO[0]);
    // Today's restoration, as it was written: what a refinement that kept
    // no round leaves, to the byte.
    let base: Vec<u16> = (at.y..at.y + at.height)
        .flat_map(|y| (at.x..at.x + at.width).map(move |x| (x, y)))
        .flat_map(|(x, y)| {
            let i = raster.at(x, y);
            raster.samples()[i..i + 3].to_vec()
        })
        .collect();
    let mut iterations = 0u8;
    match method {
        Method::Dct | Method::Pixel => {
            let rounds = if method == Method::Dct {
                DCT_ROUNDS
            } else {
                PIXEL_ROUNDS
            };
            // The pixel interval, in the restored value's terms.
            let bounds: Vec<Vec<(f64, f64)>> = work
                .channels
                .iter()
                .zip(&sigma)
                .map(|(ch, s)| {
                    let h = H_BASE + H_SIGMAS * s;
                    (0..ch.o.len())
                        .map(|i| {
                            (
                                ch.unblend(i, ch.stored[i] - h),
                                ch.unblend(i, ch.stored[i] + h),
                            )
                        })
                        .collect()
                })
                .collect();
            while iterations < rounds && !in_band(raster) {
                let kept: Vec<Vec<f64>> = work.channels.iter().map(|ch| ch.o.clone()).collect();
                let before = work.snapshot();
                work.smooth(radius, eps);
                if method == Method::Dct {
                    for ch in &mut work.channels {
                        ch.project_dct();
                    }
                } else {
                    for (ch, b) in work.channels.iter_mut().zip(&bounds) {
                        for ((o, &free), &(lo, hi)) in ch.o.iter_mut().zip(&ch.free).zip(b) {
                            if free {
                                *o = o.clamp(lo, hi);
                            }
                        }
                    }
                }
                work.render(raster, at, &noise_at);
                // A round that leaves the restoration smoother than the
                // picture around it (D307) is taken back: the round before
                // it, which ended on the data projection too, is kept.
                if under_band(raster) {
                    for (ch, o) in work.channels.iter_mut().zip(kept) {
                        ch.o = o;
                    }
                    work.render(raster, at, &noise_at);
                    break;
                }
                iterations += 1;
                if work.moved(&before) < STOP_MOVE {
                    break;
                }
            }
        }
        Method::Wiener => {
            let s: Vec<f64> = work
                .channels
                .iter()
                .map(|ch| hp_variance(&ch.ring()))
                .collect();
            for (c, ch) in work.channels.iter_mut().enumerate() {
                let p = smooth(&ch.o, (ch.w, ch.h), &ch.alpha, &ch.free, radius, eps);
                for (i, smoothed) in p.into_iter().enumerate() {
                    if !ch.free[i] {
                        continue;
                    }
                    let n = sigma[c].powi(2) / (1.0 - ch.alpha[i]).powi(2);
                    if s[c] + n > 0.0 {
                        ch.o[i] = (ch.o[i] * s[c] + smoothed * n) / (s[c] + n);
                    }
                }
            }
            iterations = 1;
        }
    }
    let dct = (method == Method::Dct).then(|| {
        let (out, all) = work
            .channels
            .iter()
            .map(Channel::outside)
            .fold((0, 0), |(a, b), (c, d)| (a + c, b + d));
        if all == 0 {
            0.0
        } else {
            out as f32 / all as f32
        }
    });
    let interval = Some(Interval {
        method,
        space: work.space,
        sigma_base: [sigma[0] as f32, sigma[1] as f32, sigma[2] as f32],
        text,
        iterations,
    });
    // No round kept — the restoration was already in the band, or its first
    // round would have left it too smooth: today's restoration stands, byte
    // for byte, with every measure it had. `consistency_dct` is then its
    // own share outside the intervals, not the refinement's.
    if iterations == 0 {
        let mut k = 0;
        for y in at.y..at.y + at.height {
            for x in at.x..at.x + at.width {
                let i = raster.at(x, y);
                raster.samples_mut()[i..i + 3].copy_from_slice(&base[k..k + 3]);
                k += 3;
            }
        }
        return Restored {
            consistency_dct: dct,
            interval,
            ..restored
        };
    }
    let clamped = work.render(raster, at, &noise_at);
    let consistency = work.consistency(raster, input, verified, restored.noise);
    let o = outline(raster, verified);
    let changed = (at.y..at.y + at.height)
        .flat_map(|y| (at.x..at.x + at.width).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            let (i, j) = (raster.at(x, y), input.at(x, y));
            (0..3).any(|c| raster.samples()[i + c] != input.samples()[j + c])
        })
        .count() as u32;
    Restored {
        changed,
        clamped,
        outline: o.share,
        steps: o.steps,
        step: o.step,
        chroma: o.chroma,
        outline_left: o.left(),
        texture: o.texture,
        texture_around: o.texture_around,
        texture_left: o.textured(),
        smoothed: o.smoothed(),
        exact: false,
        consistency_px: consistency.px,
        consistency_excluded: consistency.excluded,
        consistency_dct: dct,
        interval,
        ..restored
    }
}

/// The working space for `method` over this restoration, or `None` when
/// it cannot run: the planes — R6's inverse when the restoration was
/// planar, a per-pixel one in its shape over the RGB restoration of a
/// 4:4:4 JPEG for DCT-POCS — or R, G and B.
fn setup(
    raster: &Raster,
    input: &Raster,
    planes: Option<&Planes>,
    verified: &Verified,
    method: Method,
    restored: &Restored,
) -> Option<Work> {
    let planar = matches!(restored.planar, Some(crate::Planar::Inverse { .. }));
    if planar {
        let planes = planes?;
        let inverse = crate::planar::invert(input, planes, verified)?;
        return Work::planes(inverse, planes, method, false);
    }
    match method {
        Method::Dct => {
            let planes = planes?;
            if planes.sampling() != Sampling::H444
                || raster.layout() != Layout::Rgb8
                || (planes.width(), planes.height()) != (raster.width(), raster.height())
            {
                return None;
            }
            let inverse = per_pixel(input, planes, verified)?;
            Work::planes(inverse, planes, method, true)
        }
        Method::Pixel | Method::Wiener => {
            (raster.layout().channels() >= 3).then(|| Work::rgb(raster, input, verified))
        }
    }
}

/// An inverse in R6's shape over a 4:4:4 JPEG's RGB restoration: a block
/// is a pixel, `ᾱ` is `α`, and the restored Y, Cb and Cr are those of the
/// RGB restoration before its rounding (`restore::unblend` over the
/// decoded raster `input`), so that writing it back unchanged gives the
/// RGB restoration to the level.
fn per_pixel(input: &Raster, planes: &Planes, verified: &Verified) -> Option<Inverse> {
    let (cb, cr) = (planes.cb()?, planes.cr()?);
    let at = verified.pixels();
    let floor = f64::from(NOISE_FLOOR);
    let opaque = f64::from(verified.opaque_above());
    let alpha_at = |x: u32, y: u32| -> f64 {
        if x >= at.x && y >= at.y && x < at.x + at.width && y < at.y + at.height {
            f64::from(verified.values()[((y - at.y) * at.width + (x - at.x)) as usize])
        } else {
            0.0
        }
    };
    let ycc_at = |x: u32, y: u32| {
        let i = input.at(x, y);
        let stored = [0, 1, 2].map(|c| f64::from(input.samples()[i + c]));
        ycc(unblend(stored, alpha_at(x, y), verified.logo()))
    };
    let mut y_in = Vec::with_capacity((at.width * at.height) as usize);
    let mut y_out = Vec::with_capacity(y_in.capacity());
    for ty in 0..at.height {
        for tx in 0..at.width {
            let (x, y) = (at.x + tx, at.y + ty);
            let i = f64::from(planes.y().get(x, y));
            let a = alpha_at(x, y);
            y_in.push(i);
            y_out.push(if (floor..opaque).contains(&a) {
                ycc_at(x, y)[0]
            } else {
                i
            });
        }
    }
    let (w, h) = (planes.width(), planes.height());
    let bx0 = at.x.saturating_sub(1);
    let by0 = at.y.saturating_sub(1);
    let bx1 = (at.x + at.width + 1).min(w);
    let by1 = (at.y + at.height + 1).min(h);
    let n = ((bx1 - bx0) * (by1 - by0)) as usize;
    let (mut alpha_mean, mut cb_in, mut cb_out, mut cr_in, mut cr_out) = (
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
    );
    for y in by0..by1 {
        for x in bx0..bx1 {
            let a = alpha_at(x, y);
            let (b, r) = (f64::from(cb.get(x, y)), f64::from(cr.get(x, y)));
            alpha_mean.push(a);
            cb_in.push(b);
            cr_in.push(r);
            if (floor..opaque).contains(&a) {
                let c = ycc_at(x, y);
                cb_out.push(c[1]);
                cr_out.push(c[2]);
            } else {
                cb_out.push(b);
                cr_out.push(r);
            }
        }
    }
    Some(Inverse {
        at,
        sampling: Sampling::H444,
        alpha: verified.values().to_vec(),
        y_in,
        y_out,
        blocks: PixelRect {
            x: bx0,
            y: by0,
            width: bx1 - bx0,
            height: by1 - by0,
        },
        alpha_mean,
        cb_in,
        cb_out,
        cr_in,
        cr_out,
        logo: ycc(verified.logo()),
        opaque,
        block: (1, 1),
        max_alpha_dev_in_block: 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §4.1: the transform is orthonormal and its own inverse's inverse, to
    /// the float — so a round trip through it moves nothing, and the
    /// decoder's integer IDCT is a separate question (the fixtures' test).
    #[test]
    fn the_dct_round_trips_and_keeps_energy() {
        let mut block = [0f64; 64];
        let mut s = 7u64;
        for v in &mut block {
            s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            *v = (s >> 33) as f64 % 255.0 - 128.0;
        }
        let c = dct8(&block);
        let back = idct8(&c);
        for k in 0..64 {
            assert!((back[k] - block[k]).abs() < 1e-9, "{k}");
        }
        let e0: f64 = block.iter().map(|v| v * v).sum();
        let e1: f64 = c.iter().map(|v| v * v).sum();
        assert!((e0 - e1).abs() < 1e-6 * e0);
        // A flat block of 8 is a DC of 64 and nothing else: JPEG's scale.
        let flat = dct8(&[8.0; 64]);
        assert!((flat[0] - 64.0).abs() < 1e-9);
        assert!(flat[1..].iter().all(|v| v.abs() < 1e-9));
    }

    /// A white noise of σ reads σ: the MAD of the Laplacian over √20.
    #[test]
    fn a_white_noise_reads_its_sigma() {
        let (w, h) = (96u32, 96u32);
        let mut s = 11u64;
        let mut gauss = || {
            // Box–Muller over two uniforms.
            // SplitMix64: an LCG's consecutive draws are correlated, and
            // Box–Muller shows it.
            let mut u = || {
                s = s.wrapping_add(0x9e37_79b9_7f4a_7c15);
                let mut z = s;
                z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
                z ^= z >> 31;
                ((z >> 11) as f64 + 0.5) / (1u64 << 53) as f64
            };
            let (a, b) = (u(), u());
            (-2.0 * a.ln()).sqrt() * (2.0 * std::f64::consts::PI * b).cos()
        };
        let samples: Vec<u16> = (0..w * h * 3)
            .map(|_| (128.0 + 3.0 * gauss()).round().clamp(0.0, 255.0) as u16)
            .collect();
        let raster = Raster::new(w, h, Layout::Rgb8, samples).unwrap();
        let rect = PixelRect {
            x: 32,
            y: 32,
            width: 32,
            height: 32,
        };
        for s in sigma_base(&raster, rect) {
            // Rounding to a level adds 1/12 of a level², and the ring is
            // 1 280 samples: within a tenth either way.
            assert!((s - 3.0).abs() < 0.3, "{s}");
        }
    }

    /// `P_S` leaves the anchor alone and smooths in proportion to how
    /// little a sample is to be believed: at `α` 0 nothing moves, at a
    /// half three quarters of the filter's answer is taken.
    #[test]
    fn the_smoothness_moves_only_what_the_restoration_writes() {
        let (w, h) = (16, 16);
        let o: Vec<f64> = (0..w * h)
            .map(|i| {
                if (i / w + i % w) % 2 == 0 {
                    101.0
                } else {
                    99.0
                }
            })
            .collect();
        let alpha: Vec<f64> = (0..w * h)
            .map(|i| if i % w < 8 { 0.0 } else { 0.5 })
            .collect();
        let free: Vec<bool> = (0..w * h).map(|i| i % w >= 8).collect();
        let out = smooth(&o, (w, h), &alpha, &free, 2, 16.0);
        for i in 0..w * h {
            if !free[i] {
                assert_eq!(out[i], o[i]);
            } else if (4..12).contains(&(i / w)) && (10..14).contains(&(i % w)) {
                // A checker of ±1 under eps 16 is flat: the filter gives
                // about 100, and a quarter of the sample is kept.
                assert!(
                    (out[i] - (0.25 * o[i] + 0.75 * 100.0)).abs() < 0.1,
                    "{i}: {}",
                    out[i]
                );
            }
        }
    }
}
