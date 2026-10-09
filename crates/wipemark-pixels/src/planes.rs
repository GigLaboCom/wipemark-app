//! The stored planes of a lossy JPEG (D302): Y, Cb and Cr at their own
//! resolution — after the IDCT and dequantisation, before the decoder
//! upsamples the chroma and converts to RGB, cropped from the MCU padding
//! — with the sampling and the two quantisation tables.
//!
//! A value, not a codec: `wipemark-picture` reads them out of a file
//! (`Decoded.planes`) and this crate never sees one. Nothing reads them
//! yet; the planar inverse will write back through [`Planes::to_rgb`].
//!
//! [`Planes::to_rgb`] restates, in scalar Rust, what `zune-jpeg` does
//! after its IDCT, so that the planes upsampled are the decoder's own RGB
//! raster to the byte:
//!
//! > Restated from `zune-jpeg` 0.5.15 (`etemesi254/zune-image` at
//! > `31d81fed7551c8ccea456d9d8e2b1fd8bebb6995`, `crates/zune-jpeg`; MIT OR
//! > Apache-2.0 OR Zlib), as patched on `GigLaboCom/zune-image`
//! > `wipemark/planes` (whose patch does not touch these functions):
//! > `src/upsampler/scalar.rs` (`upsample_horizontal`, `upsample_vertical`,
//! > `upsample_hv`, `upsample_generic`), `src/worker.rs` (`upsample`: the
//! > rows above and below at the picture's top and bottom), and
//! > `src/color_convert/scalar.rs` (`ycbcr_to_rgb_inner_16_scalar` and its
//! > constants). **Cut**: the SIMD variants (AVX2, NEON, portable SIMD,
//! > which compute the same integers), the streaming by rows of MCUs, the
//! > outputs other than RGB. **Changed**: it works on the cropped plane, so
//! > a neighbour past the plane's last sample is that sample (the decoder
//! > reads the MCU padding there instead — which an odd width or height
//! > never asks for, and an even one asks for in its last column or row
//! > only; see `docs/architecture/zune-jpeg-pin.md`).

use crate::raster::Raster;

/// How a JPEG's chroma is sampled against its luma.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sampling {
    /// Every component at full resolution.
    H444,
    /// Chroma at half the width.
    H422,
    /// Chroma at half the width and half the height.
    H420,
    /// One component, no chroma.
    Gray,
    /// Any other set of factors, as the frame header gives them (Y, Cb,
    /// Cr). `to_rgb` upsamples each plane by its own ratio; only the four
    /// above are held to the decoder by a test.
    Other { h: [u8; 3], v: [u8; 3] },
}

impl Sampling {
    /// The sampling named by three components' factors: the named forms
    /// whatever the factors' scale (4:4:4 written as 2×2 three times is
    /// still 4:4:4), otherwise [`Sampling::Other`] with the factors as they
    /// are.
    pub fn of(h: [u8; 3], v: [u8; 3]) -> Sampling {
        let h_max = h.iter().copied().max().unwrap_or(0);
        let v_max = v.iter().copied().max().unwrap_or(0);
        let ratio = |f: u8, max: u8| (f != 0 && max.is_multiple_of(f)).then(|| max / f);
        let ratios: Vec<Option<(u8, u8)>> = (0..3)
            .map(|i| Some((ratio(h[i], h_max)?, ratio(v[i], v_max)?)))
            .collect();
        match (ratios[0], ratios[1], ratios[2]) {
            (Some((1, 1)), Some(c), Some(d)) if c == d => match c {
                (1, 1) => Sampling::H444,
                (2, 1) => Sampling::H422,
                (2, 2) => Sampling::H420,
                _ => Sampling::Other { h, v },
            },
            _ => Sampling::Other { h, v },
        }
    }

    /// The factors (Y, Cb, Cr) the named forms stand for; `None` for
    /// [`Sampling::Gray`].
    pub fn factors(self) -> Option<([u8; 3], [u8; 3])> {
        match self {
            Sampling::H444 => Some(([1, 1, 1], [1, 1, 1])),
            Sampling::H422 => Some(([2, 1, 1], [1, 1, 1])),
            Sampling::H420 => Some(([2, 1, 1], [2, 1, 1])),
            Sampling::Gray => None,
            Sampling::Other { h, v } => Some((h, v)),
        }
    }

    /// The format's name for it — an identifier, never a sentence.
    pub fn id(self) -> &'static str {
        match self {
            Sampling::H444 => "4:4:4",
            Sampling::H422 => "4:2:2",
            Sampling::H420 => "4:2:0",
            Sampling::Gray => "gray",
            Sampling::Other { .. } => "other",
        }
    }
}

/// Why a [`Plane`] or [`Planes`] was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PlanesError {
    #[error("a plane needs a width and a height")]
    ZeroDimension,
    #[error("a {width}×{height} plane needs {needed} samples, got {got}")]
    Length {
        width: u32,
        height: u32,
        needed: usize,
        got: usize,
    },
    #[error("a stored sample lies within 0–255, got {value}")]
    Range { value: u16 },
    #[error("the {plane} plane is {got_w}×{got_h}, its sampling makes it {want_w}×{want_h}")]
    Size {
        plane: &'static str,
        want_w: u32,
        want_h: u32,
        got_w: u32,
        got_h: u32,
    },
    #[error("a sampling factor is 0, over 4, or does not divide the largest")]
    Factors,
    #[error("chroma planes are both present, except for grey, which has none")]
    Chroma,
}

/// One component's samples, `width × height`, row-major, each 0–255
/// (`u16`, as a [`Raster`]'s samples are).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plane {
    width: u32,
    height: u32,
    samples: Vec<u16>,
}

impl Plane {
    pub fn new(width: u32, height: u32, samples: Vec<u16>) -> Result<Plane, PlanesError> {
        if width == 0 || height == 0 {
            return Err(PlanesError::ZeroDimension);
        }
        let needed = width as usize * height as usize;
        if samples.len() != needed {
            return Err(PlanesError::Length {
                width,
                height,
                needed,
                got: samples.len(),
            });
        }
        if let Some(&value) = samples.iter().find(|&&s| s > 255) {
            return Err(PlanesError::Range { value });
        }
        Ok(Plane {
            width,
            height,
            samples,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn samples(&self) -> &[u16] {
        &self.samples
    }

    /// The sample at `(x, y)`.
    pub fn get(&self, x: u32, y: u32) -> u16 {
        self.samples[y as usize * self.width as usize + x as usize]
    }
}

/// The quantisation tables the planes were stored with, in **natural**
/// (row-major) order — not the zigzag order a DQT segment carries them in.
/// `chroma` is the table Cb and Cr share; `None` for a grey picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quant {
    pub luma: [u16; 64],
    pub chroma: Option<[u16; 64]>,
}

/// A lossy JPEG's stored planes (D302).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planes {
    width: u32,
    height: u32,
    sampling: Sampling,
    y: Plane,
    cb: Option<Plane>,
    cr: Option<Plane>,
    quant: Quant,
}

/// `ceil(n · f / max)`: a plane's side for a picture's side `n`.
fn side(n: u32, f: u8, max: u8) -> u32 {
    (n * u32::from(f)).div_ceil(u32::from(max))
}

impl Planes {
    /// The planes of a `width × height` picture, refused unless every
    /// plane is the size its sampling makes it: `ceil(width · h / h_max) ×
    /// ceil(height · v / v_max)`, and chroma present exactly when the
    /// sampling has it.
    pub fn new(
        width: u32,
        height: u32,
        sampling: Sampling,
        y: Plane,
        cb: Option<Plane>,
        cr: Option<Plane>,
        quant: Quant,
    ) -> Result<Planes, PlanesError> {
        if width == 0 || height == 0 {
            return Err(PlanesError::ZeroDimension);
        }
        let planes = Planes {
            width,
            height,
            sampling,
            y,
            cb,
            cr,
            quant,
        };
        let Some((h, v)) = sampling.factors() else {
            if planes.cb.is_some() || planes.cr.is_some() {
                return Err(PlanesError::Chroma);
            }
            check("Y", &planes.y, width, height)?;
            return Ok(planes);
        };
        let (Some(cb), Some(cr)) = (&planes.cb, &planes.cr) else {
            return Err(PlanesError::Chroma);
        };
        let h_max = h.iter().copied().max().unwrap_or(0);
        let v_max = v.iter().copied().max().unwrap_or(0);
        for f in h.iter().zip(&v) {
            let (&hf, &vf) = f;
            if hf == 0
                || vf == 0
                || hf > 4
                || vf > 4
                || !h_max.is_multiple_of(hf)
                || !v_max.is_multiple_of(vf)
            {
                return Err(PlanesError::Factors);
            }
        }
        for (i, (name, plane)) in [("Y", &planes.y), ("Cb", cb), ("Cr", cr)]
            .into_iter()
            .enumerate()
        {
            check(
                name,
                plane,
                side(width, h[i], h_max),
                side(height, v[i], v_max),
            )?;
        }
        Ok(planes)
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn sampling(&self) -> Sampling {
        self.sampling
    }

    pub fn y(&self) -> &Plane {
        &self.y
    }

    pub fn cb(&self) -> Option<&Plane> {
        self.cb.as_ref()
    }

    pub fn cr(&self) -> Option<&Plane> {
        self.cr.as_ref()
    }

    pub fn quant(&self) -> &Quant {
        &self.quant
    }

    /// The planes as the decoder hands them out: each upsampled to the
    /// picture's size by `zune-jpeg`'s triangle filter — `(3·near + far +
    /// 2) >> 2`, horizontally for 4:2:2, vertically and **then**
    /// horizontally, row by row, for 4:2:0; replicated for any other ratio
    /// — and converted with its 14-bit BT.601 full-range integers. Grey is
    /// Y three times. See the module's header for what was restated from
    /// where.
    pub fn to_rgb(&self) -> Raster {
        let (w, h) = (self.width as usize, self.height as usize);
        let mut out = Vec::with_capacity(w * h * 3);
        match (self.sampling.factors(), &self.cb, &self.cr) {
            (Some((hf, vf)), Some(cb), Some(cr)) => {
                let h_max = hf.iter().copied().max().unwrap_or(1);
                let v_max = vf.iter().copied().max().unwrap_or(1);
                let up =
                    |plane: &Plane, i: usize| upsample(plane, h_max / hf[i], v_max / vf[i], w, h);
                let (y, cb, cr) = (up(&self.y, 0), up(cb, 1), up(cr, 2));
                for ((&y, &cb), &cr) in y.iter().zip(&cb).zip(&cr) {
                    out.extend_from_slice(&ycbcr_to_rgb(y, cb, cr));
                }
            }
            _ => {
                for &y in &self.y.samples {
                    out.extend_from_slice(&[y, y, y]);
                }
            }
        }
        Raster::trusted_rgb8(self.width, self.height, out)
    }
}

fn check(name: &'static str, plane: &Plane, want_w: u32, want_h: u32) -> Result<(), PlanesError> {
    if plane.width == want_w && plane.height == want_h {
        Ok(())
    } else {
        Err(PlanesError::Size {
            plane: name,
            want_w,
            want_h,
            got_w: plane.width,
            got_h: plane.height,
        })
    }
}

/// The triangle filter's one output: `(3·near + far + 2) >> 2`.
fn tri(near: i32, far: i32) -> i32 {
    (3 * near + far + 2) >> 2
}

/// A row of `n` samples upsampled ×2 to `out` samples: output `2k` leans
/// on `k − 1`, output `2k + 1` on `k + 1`, a neighbour past either end
/// being the end sample (`upsample_horizontal`).
fn horizontal(row: &[i32], out: usize) -> Vec<i32> {
    let last = row.len() - 1;
    (0..out)
        .map(|x| {
            let k = (x / 2).min(last);
            let far = if x.is_multiple_of(2) {
                k.saturating_sub(1)
            } else {
                (k + 1).min(last)
            };
            tri(row[k], row[far])
        })
        .collect()
}

/// Output row `j` of a ×2 vertical upsampling: row `j / 2` near, the row
/// above (even `j`) or below (odd `j`) far, the picture's first and last
/// rows standing in for the missing neighbour (`upsample_vertical`, and
/// `worker::upsample`'s `row_up`/`row_down` at the edges).
fn vertical(plane: &Plane, j: usize) -> Vec<i32> {
    let (pw, last) = (plane.width as usize, plane.height as usize - 1);
    let k = (j / 2).min(last);
    let far = if j.is_multiple_of(2) {
        k.saturating_sub(1)
    } else {
        (k + 1).min(last)
    };
    let near = &plane.samples[k * pw..(k + 1) * pw];
    let far = &plane.samples[far * pw..(far + 1) * pw];
    near.iter()
        .zip(far)
        .map(|(&n, &f)| tri(i32::from(n), i32::from(f)))
        .collect()
}

/// A plane brought to `w × h` by its ratio to the largest factors.
fn upsample(plane: &Plane, rh: u8, rv: u8, w: usize, h: usize) -> Vec<i32> {
    let pw = plane.width as usize;
    let row = |y: usize| -> Vec<i32> {
        plane.samples[y * pw..(y + 1) * pw]
            .iter()
            .map(|&s| i32::from(s))
            .collect()
    };
    let mut out = Vec::with_capacity(w * h);
    for j in 0..h {
        let line = match (rh, rv) {
            (1, 1) => row(j),
            (2, 1) => horizontal(&row(j), w),
            (1, 2) => vertical(plane, j),
            // Vertical first, then horizontal on that line, through an
            // integer row of its own: two roundings, in this order.
            (2, 2) => horizontal(&vertical(plane, j), w),
            // `upsample_generic`: nearest, by replication.
            _ => {
                let src = row((j / usize::from(rv)).min(plane.height as usize - 1));
                (0..w)
                    .map(|x| src[(x / usize::from(rh)).min(pw - 1)])
                    .collect()
            }
        };
        out.extend_from_slice(&line[..w]);
    }
    out
}

// `color_convert/scalar.rs`: BT.601 full range, 14 bits of precision.
const Y_CF: i32 = 16384;
const CR_CF: i32 = 22970;
const CB_CF: i32 = 29032;
const C_G_CR_COEF_1: i32 = -11700;
const C_G_CB_COEF_2: i32 = -5638;
const YUV_PREC: u32 = 14;
const YUV_RND: i32 = (1 << (YUV_PREC - 1)) - 1;

fn ycbcr_to_rgb(y: i32, cb: i32, cr: i32) -> [u16; 3] {
    let (cb, cr) = (cb - 128, cr - 128);
    let y0 = y * Y_CF + YUV_RND;
    let r = (y0 + cr * CR_CF) >> YUV_PREC;
    let g = (y0 + cr * C_G_CR_COEF_1 + cb * C_G_CB_COEF_2) >> YUV_PREC;
    let b = (y0 + cb * CB_CF) >> YUV_PREC;
    // In 0–255 by the clamp, so the cast is exact.
    [r, g, b].map(|c| c.clamp(0, 255) as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plane(w: u32, h: u32) -> Plane {
        Plane::new(w, h, vec![128; (w * h) as usize]).unwrap()
    }

    fn quant() -> Quant {
        Quant {
            luma: [1; 64],
            chroma: Some([1; 64]),
        }
    }

    fn planes(
        sampling: Sampling,
        (w, h): (u32, u32),
        c: (u32, u32),
    ) -> Result<Planes, PlanesError> {
        Planes::new(
            w,
            h,
            sampling,
            plane(w, h),
            Some(plane(c.0, c.1)),
            Some(plane(c.0, c.1)),
            quant(),
        )
    }

    #[test]
    fn planes_new_refuses_sizes_that_do_not_match_the_sampling() {
        // The sizes each sampling makes, at odd sides: ceil, never floor.
        assert!(planes(Sampling::H444, (37, 23), (37, 23)).is_ok());
        assert!(planes(Sampling::H422, (37, 23), (19, 23)).is_ok());
        assert!(planes(Sampling::H420, (37, 23), (19, 12)).is_ok());
        assert!(planes(
            Sampling::Other {
                h: [1, 1, 1],
                v: [2, 1, 1]
            },
            (37, 23),
            (37, 12)
        )
        .is_ok());

        // A chroma plane the size of the picture under 4:2:0 — what the
        // decoder hands out after upsampling — is not a stored plane.
        assert_eq!(
            planes(Sampling::H420, (37, 23), (37, 23)).unwrap_err(),
            PlanesError::Size {
                plane: "Cb",
                want_w: 19,
                want_h: 12,
                got_w: 37,
                got_h: 23
            }
        );
        // Floor instead of ceil; the MCU padding kept.
        assert!(planes(Sampling::H420, (37, 23), (18, 11)).is_err());
        assert!(planes(Sampling::H420, (37, 23), (24, 16)).is_err());
        assert!(planes(Sampling::H422, (37, 23), (19, 12)).is_err());
        assert!(planes(Sampling::H444, (37, 23), (19, 23)).is_err());
        // A luma plane of the wrong size.
        assert!(Planes::new(
            37,
            23,
            Sampling::H420,
            plane(40, 24),
            Some(plane(19, 12)),
            Some(plane(19, 12)),
            quant()
        )
        .is_err());
        // Chroma missing, or present on grey.
        assert_eq!(
            Planes::new(37, 23, Sampling::H420, plane(37, 23), None, None, quant()).unwrap_err(),
            PlanesError::Chroma
        );
        assert_eq!(
            Planes::new(
                37,
                23,
                Sampling::Gray,
                plane(37, 23),
                Some(plane(37, 23)),
                None,
                quant()
            )
            .unwrap_err(),
            PlanesError::Chroma
        );
        assert!(Planes::new(37, 23, Sampling::Gray, plane(37, 23), None, None, quant()).is_ok());
        // Factors that do not divide.
        assert_eq!(
            planes(
                Sampling::Other {
                    h: [3, 2, 2],
                    v: [1, 1, 1]
                },
                (37, 23),
                (25, 23)
            )
            .unwrap_err(),
            PlanesError::Factors
        );
    }

    #[test]
    fn a_plane_holds_eight_bit_samples_of_its_own_size() {
        assert_eq!(
            Plane::new(2, 2, vec![0; 3]).unwrap_err(),
            PlanesError::Length {
                width: 2,
                height: 2,
                needed: 4,
                got: 3
            }
        );
        assert_eq!(
            Plane::new(1, 1, vec![256]).unwrap_err(),
            PlanesError::Range { value: 256 }
        );
        assert_eq!(
            Plane::new(0, 1, vec![]).unwrap_err(),
            PlanesError::ZeroDimension
        );
    }

    #[test]
    fn the_sampling_is_named_by_its_ratios_not_its_scale() {
        assert_eq!(Sampling::of([1, 1, 1], [1, 1, 1]), Sampling::H444);
        assert_eq!(Sampling::of([2, 2, 2], [2, 2, 2]), Sampling::H444);
        assert_eq!(Sampling::of([2, 1, 1], [1, 1, 1]), Sampling::H422);
        assert_eq!(Sampling::of([2, 1, 1], [2, 1, 1]), Sampling::H420);
        assert_eq!(Sampling::of([4, 2, 2], [4, 2, 2]), Sampling::H420);
        assert_eq!(
            Sampling::of([1, 1, 1], [2, 1, 1]),
            Sampling::Other {
                h: [1, 1, 1],
                v: [2, 1, 1]
            }
        );
        assert_eq!(
            Sampling::of([2, 1, 2], [2, 1, 1]),
            Sampling::Other {
                h: [2, 1, 2],
                v: [2, 1, 1]
            }
        );
    }

    #[test]
    fn grey_neutral_chroma_is_grey() {
        // Cb = Cr = 128: every channel is Y, at every Y.
        for y in 0..=255u16 {
            let c = ycbcr_to_rgb(i32::from(y), 128, 128);
            assert_eq!(c, [y, y, y]);
        }
    }

    #[test]
    fn the_triangle_filter_leans_on_the_nearer_sample() {
        // 0, 100 → 0, 25, 75, 100: the ends are the end samples.
        assert_eq!(horizontal(&[0, 100], 4), vec![0, 25, 75, 100]);
        // An odd width stops on an even output, which never reads past
        // the plane.
        assert_eq!(horizontal(&[0, 100, 200], 5), vec![0, 25, 75, 125, 175]);
    }
}
