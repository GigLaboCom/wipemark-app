//! The blend past its one global colour (E12-R9) — **built, not opened**:
//! a constant **bias** (R9a, D308 proposed), a **logo colour per pixel**
//! read from a `.wml` asset (R9b, D313 proposed) and the **linear-light**
//! blend (R9c, D311 proposed). The catalogue accepts any of them only in a
//! build with the `blend-preview` feature; without it every profile is the
//! `encoded` blend with one logo colour and no bias, and every function
//! here reduces to the arithmetic the crate had before, operation for
//! operation (A2: `a_profile_without_a_bias_is_byte_for_byte_todays`).
//!
//! The forward model, per colour channel, where the mark is drawn
//! (`α > 0`), in stored units:
//!
//! ```text
//! encoded        I = α·L(p) + (1 − α)·O                              + b
//! linear-light   I = from_lin(α·lin(L(p)) + (1 − α)·lin(O))           + b
//! ```
//!
//! — `L(p)` the profile's `logo` or its `logo_map` brought to the template,
//! `b` the profile's `bias` or nothing — which is how
//! [`crate::synth::composite_with`] draws a composite. The inverse takes
//! the bias off first and then undoes the model ([`Law::inverse`]); the
//! out-of-range proof measures how far a stored value is outside what such
//! a blend could make over any original ([`Law::out_of_range`]).
//!
//! # The `.wml` file ("Wipemark logo")
//!
//! In the `.wma`'s style: the magic `WML1`, then `u16 width`, `u16 height`
//! and `u8 depth` — 16, the only depth read — little-endian, then three
//! planes, R, G and B, each `width × height` samples, row-major, each a
//! `u16` little-endian. `L = sample · 255 / 65535`, in 8-bit levels: a
//! regressed colour keeps its fraction (D242). A profile names one by
//! asset, sha256 and size, exactly as it names an opacity map, and the
//! catalogue re-hashes it on load (`AssetProblem::Hash`).

use crate::alpha::AlphaMap;
use crate::calibrate::BlendModel;
use crate::catalogue::Profile;
use crate::geometry::{self, Kernel, PixelRect, SubRect};
use crate::raster::Raster;

/// The four bytes every `.wml` starts with.
pub const WML_MAGIC: &[u8; 4] = b"WML1";
const HEADER: usize = 4 + 2 + 2 + 1;
/// The one depth a `.wml` is written and read at.
const DEPTH: u8 = 16;

/// Why a logo colour map could not be read or made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WmlError {
    #[error("not a logo colour map: the file does not start with WML1")]
    Magic,
    #[error("a logo colour map of depth {0}; only 16 is read")]
    Depth(u8),
    #[error("a logo colour map with a zero dimension")]
    ZeroDimension,
    #[error("a logo colour map whose samples are {got} bytes where {needed} were declared")]
    Size { needed: usize, got: usize },
    #[error("a logo colour outside 0 to 255")]
    Range,
}

/// `L(p)`: the logo's colour per sample of an opacity map, in 8-bit
/// levels, row-major on the map's own grid (R9b).
#[derive(Debug, Clone, PartialEq)]
pub struct LogoMap {
    width: u32,
    height: u32,
    colours: Vec<[f32; 3]>,
}

impl LogoMap {
    /// A map over `colours`, which must be `width × height` long, each
    /// channel in 0–255.
    pub fn new(width: u32, height: u32, colours: Vec<[f32; 3]>) -> Result<Self, WmlError> {
        if width == 0 || height == 0 {
            return Err(WmlError::ZeroDimension);
        }
        let needed = width as usize * height as usize;
        if colours.len() != needed {
            return Err(WmlError::Size {
                needed,
                got: colours.len(),
            });
        }
        if colours.iter().flatten().any(|c| !(0.0..=255.0).contains(c)) {
            return Err(WmlError::Range);
        }
        Ok(LogoMap {
            width,
            height,
            colours,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// The colours, row-major — the shape
    /// [`crate::synth::LogoColor::PerPixel`] takes.
    pub fn colours(&self) -> &[[f32; 3]] {
        &self.colours
    }

    /// Read a `.wml`.
    pub fn read(bytes: &[u8]) -> Result<Self, WmlError> {
        if bytes.len() < HEADER || &bytes[..4] != WML_MAGIC {
            return Err(WmlError::Magic);
        }
        let width = u32::from(u16::from_le_bytes([bytes[4], bytes[5]]));
        let height = u32::from(u16::from_le_bytes([bytes[6], bytes[7]]));
        let depth = bytes[8];
        if depth != DEPTH {
            return Err(WmlError::Depth(depth));
        }
        if width == 0 || height == 0 {
            return Err(WmlError::ZeroDimension);
        }
        let n = width as usize * height as usize;
        let body = &bytes[HEADER..];
        let needed = 3 * n * 2;
        if body.len() != needed {
            return Err(WmlError::Size {
                needed,
                got: body.len(),
            });
        }
        let sample = |plane: usize, p: usize| {
            let i = 2 * (plane * n + p);
            f32::from(u16::from_le_bytes([body[i], body[i + 1]])) * 255.0 / f32::from(u16::MAX)
        };
        let colours = (0..n)
            .map(|p| [sample(0, p), sample(1, p), sample(2, p)])
            .collect();
        LogoMap::new(width, height, colours)
    }

    /// Write a `.wml`, each colour rounded to the nearest sample. A map
    /// wider or taller than 65535 cannot be written.
    pub fn write(&self) -> Result<Vec<u8>, WmlError> {
        let (Ok(width), Ok(height)) = (u16::try_from(self.width), u16::try_from(self.height))
        else {
            return Err(WmlError::Size {
                needed: usize::from(u16::MAX),
                got: self.width.max(self.height) as usize,
            });
        };
        let mut out = WML_MAGIC.to_vec();
        out.extend_from_slice(&width.to_le_bytes());
        out.extend_from_slice(&height.to_le_bytes());
        out.push(DEPTH);
        for plane in 0..3 {
            for c in &self.colours {
                let s = (c[plane] / 255.0 * f32::from(u16::MAX)).round() as u16;
                out.extend_from_slice(&s.to_le_bytes());
            }
        }
        Ok(out)
    }
}

/// How a stored value is made from an original, past the logo's colour:
/// the model and the bias, in the raster's stored units (R9a, R9c).
/// `Law::today` — `encoded`, no bias — is every shipped profile's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Law {
    pub(crate) model: BlendModel,
    /// Added where the mark is drawn (`α > 0`), per channel, in stored
    /// units; `None` is no bias at all, never a zero added.
    pub(crate) bias: Option<[f64; 3]>,
    /// The layout's largest stored value.
    pub(crate) max: f64,
}

impl Law {
    /// A profile's law over a raster whose largest value is `max`: its
    /// bias scaled to the layout as its logo is.
    pub(crate) fn of(profile: &Profile, max: f64) -> Law {
        Law {
            model: profile.model,
            bias: profile.bias.map(|b| b.map(|c| f64::from(c) * max / 255.0)),
            max,
        }
    }

    /// `encoded` with no bias: the blend every shipped profile declares,
    /// and the only one the planar path and the interval methods take in
    /// its full form.
    pub(crate) fn today(self) -> bool {
        self.model == BlendModel::Encoded && self.bias.is_none()
    }

    /// Linear in stored code values — `encoded`, with or without a bias:
    /// what the planar path (R6) and the interval methods (R8) can take,
    /// since JFIF's matrix and the DCT are linear too. `linear-light` is
    /// not, and takes the RGB path unrefined (R9c).
    pub(crate) fn linear_in_codes(self) -> bool {
        self.model == BlendModel::Encoded
    }

    /// `stored` less the bias, where the mark is drawn (D308).
    fn debiased(self, stored: [f64; 3], a: f64) -> [f64; 3] {
        match self.bias {
            Some(b) if a > 0.0 => [stored[0] - b[0], stored[1] - b[1], stored[2] - b[2]],
            _ => stored,
        }
    }

    /// The reverse blend at `a` with the logo `logo` (stored units),
    /// unrounded and unclamped: the bias taken off, then the model undone.
    /// `encoded` is [`crate::restore::unblend`], GWT's equation, untouched;
    /// `linear-light` is `from_lin((lin(I) − α·lin(L))/(1 − α))` (D311),
    /// the curve extended past its range so an inverse that leaves it
    /// still says by how much.
    pub(crate) fn inverse(self, stored: [f64; 3], a: f64, logo: [f64; 3]) -> [f64; 3] {
        let s = self.debiased(stored, a);
        match self.model {
            BlendModel::Encoded => crate::restore::unblend(s, a, logo),
            BlendModel::LinearLight => {
                let to_8 = 255.0 / self.max;
                [0, 1, 2].map(|c| {
                    let o = (lin(s[c] * to_8) - a * lin(logo[c] * to_8)) / (1.0 - a);
                    unlin(o) / to_8
                })
            }
        }
    }

    /// The forward blend of channel `c`, unrounded: what an original `o`
    /// is stored as under a mark of opacity `a` and logo colour `l` (all
    /// stored units). `encoded` with no bias is `a·l + (1 − a)·o`, the
    /// expression every consistency measure wrote before (D305).
    pub(crate) fn forward(self, o: f64, a: f64, l: f64, c: usize) -> f64 {
        let v = match self.model {
            BlendModel::Encoded => a * l + (1.0 - a) * o,
            BlendModel::LinearLight => {
                let to_8 = 255.0 / self.max;
                unlin(a * lin(l * to_8) + (1.0 - a) * lin(o * to_8)) / to_8
            }
        };
        match self.bias {
            Some(b) if a > 0.0 => v + b[c],
            _ => v,
        }
    }

    /// Per channel, whether `stored` lies outside what a blend at `a` with
    /// `logo` could make over any original by more than `allowance`
    /// (stored units) — D240's proof. `encoded`: the inverse's excess over
    /// the range times `1 − α`, today's arithmetic, the interval moved by
    /// the bias (D308) because the inverse took it off. `linear-light`:
    /// the same in light, the allowance converted at the stored value by
    /// the curve's own slope — `lin(I) − lin(I − δ)` below the range,
    /// `lin(I + δ) − lin(I)` above it (D311).
    pub(crate) fn out_of_range(
        self,
        stored: [f64; 3],
        a: f64,
        logo: [f64; 3],
        allowance: f64,
    ) -> [bool; 3] {
        match self.model {
            BlendModel::Encoded => self.inverse(stored, a, logo).map(|v| {
                let gap = if v < 0.0 {
                    -v * (1.0 - a)
                } else if v > self.max {
                    (v - self.max) * (1.0 - a)
                } else {
                    0.0
                };
                gap > allowance
            }),
            BlendModel::LinearLight => {
                let s = self.debiased(stored, a);
                let to_8 = 255.0 / self.max;
                let delta = allowance * to_8;
                [0, 1, 2].map(|c| {
                    let s8 = s[c] * to_8;
                    let i = lin(s8);
                    let o = (i - a * lin(logo[c] * to_8)) / (1.0 - a);
                    if o < 0.0 {
                        -o * (1.0 - a) > i - lin(s8 - delta)
                    } else if o > 1.0 {
                        (o - 1.0) * (1.0 - a) > lin(s8 + delta) - i
                    } else {
                        false
                    }
                })
            }
        }
    }
}

/// sRGB decoding of an 8-bit-scaled value to linear light — inside 0–255
/// [`crate::calibrate::to_linear`]'s arithmetic, and past it the curve
/// continued as an odd function, so an unclamped inverse stays monotonic.
pub(crate) fn lin(v8: f64) -> f64 {
    let c = v8 / 255.0;
    let m = c.abs();
    let l = if m <= 0.040_45 {
        m / 12.92
    } else {
        ((m + 0.055) / 1.055).powf(2.4)
    };
    l.copysign(c)
}

/// [`lin`]'s inverse, to 8-bit-scaled units — inside 0–1
/// [`crate::calibrate::from_linear`]'s arithmetic.
pub(crate) fn unlin(l: f64) -> f64 {
    let m = l.abs();
    let c = if m <= 0.003_130_8 {
        m * 12.92
    } else {
        1.055 * m.powf(1.0 / 2.4) - 0.055
    };
    c.copysign(l) * 255.0
}

/// The profile's logo colour map brought to the template of `map` at
/// `rect` by `kernel`, in stored units, one colour per template pixel —
/// `α·L` and `α` resampled alike and one divided by the other, as
/// [`crate::synth::composite_with`] draws a `PerPixel` logo, so a colour
/// is weighted by where the mark is. Where the template has no opacity the
/// profile's one colour stands. `None` for a profile without a map, and
/// for a map of another size than `map` (the catalogue refuses that one).
#[allow(clippy::needless_range_loop)]
pub(crate) fn template_logos(
    profile: &Profile,
    map: &AlphaMap,
    rect: SubRect,
    kernel: Kernel,
    max: f64,
) -> Option<Vec<[f64; 3]>> {
    let logos = profile.logo_map.as_ref()?;
    if (logos.width(), logos.height()) != (map.width(), map.height()) {
        return None;
    }
    let (fx, fy) = (rect.x - rect.x.floor(), rect.y - rect.y.floor());
    let shape = geometry::shape_with(map, rect.size, fx, fy, kernel)?;
    let global = profile.logo.map(|c| f64::from(c) * max / 255.0);
    let scale = |l: [f64; 3]| l.map(|c| c * max / 255.0);
    if shape.canonical {
        return Some(
            logos
                .colours()
                .iter()
                .map(|l| scale(l.map(f64::from)))
                .collect(),
        );
    }
    let mut out = vec![global; shape.values.len()];
    for c in 0..3 {
        let weighted: Vec<f32> = map
            .values()
            .iter()
            .zip(logos.colours())
            .map(|(a, l)| (a * l[c] / 255.0).clamp(0.0, 1.0))
            .collect();
        let premultiplied = AlphaMap::new(map.width(), map.height(), weighted).ok()?;
        let shaped = geometry::shape_with(&premultiplied, rect.size, fx, fy, kernel)?;
        for (p, (num, den)) in shaped.values.iter().zip(&shape.values).enumerate() {
            // A filtered kernel's faint ring can be negative or next to
            // nothing: no colour can be read off it.
            if *den > 1e-4 {
                out[p][c] = f64::from(num / den) * max;
            }
        }
    }
    Some(out)
}

/// One template's logo colour: the profile's one colour, and its map's
/// colour per template pixel when it has one — what the proof, the
/// restoration and the outline read through [`Colours::at`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Colours {
    pub(crate) logo: [f64; 3],
    pub(crate) per_pixel: Option<Vec<[f64; 3]>>,
}

impl Colours {
    /// The logo at template pixel `p` (row-major over the template).
    pub(crate) fn at(&self, p: usize) -> [f64; 3] {
        match &self.per_pixel {
            Some(l) => l[p],
            None => self.logo,
        }
    }
}

/// `mark` (the template, `at`'s size) drawn onto `raster` at `at` with the
/// profile's blend — the colour per pixel, the model, the bias — rounded
/// half away from zero and clamped, the alpha channel untouched:
/// [`crate::composite`] generalised, for [`crate::measure_at`]'s stand-in
/// for the stored file under a profile that is not today's.
pub(crate) fn draw(
    raster: &mut Raster,
    mark: &AlphaMap,
    at: PixelRect,
    colours: &Colours,
    law: Law,
) {
    let (w, h) = (raster.width(), raster.height());
    for my in 0..mark.height().min(at.height) {
        for mx in 0..mark.width().min(at.width) {
            let (x, y) = (at.x + mx, at.y + my);
            if x >= w || y >= h {
                continue;
            }
            let a = f64::from(mark.get(i64::from(mx), i64::from(my)));
            if a <= 0.0 {
                continue;
            }
            let l = colours.at((my * at.width + mx) as usize);
            let i = raster.at(x, y);
            let samples = raster.samples_mut();
            for (c, &lc) in l.iter().enumerate() {
                let v = law.forward(f64::from(samples[i + c]), a, lc, c);
                samples[i + c] = v.round().clamp(0.0, law.max) as u16;
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::needless_range_loop)]
mod tests {
    use super::*;

    #[test]
    fn a_logo_map_survives_a_round_trip_to_a_sixteen_bit_sample() {
        let colours = vec![[252.1, 253.5, 252.8], [0.0, 128.25, 255.0]];
        let map = LogoMap::new(2, 1, colours).unwrap();
        let bytes = map.write().unwrap();
        assert_eq!(&bytes[..4], WML_MAGIC);
        assert_eq!(bytes.len(), HEADER + 3 * 2 * 2);
        let back = LogoMap::read(&bytes).unwrap();
        for (a, b) in map.colours().iter().zip(back.colours()) {
            for c in 0..3 {
                assert!((a[c] - b[c]).abs() <= 255.0 / 65535.0, "{a:?} {b:?}");
            }
        }
    }

    #[test]
    fn a_bad_logo_map_is_refused_by_name() {
        let whole = LogoMap::new(1, 1, vec![[1.0, 2.0, 3.0]])
            .unwrap()
            .write()
            .unwrap();
        let mut magic = whole.clone();
        magic[3] = b'2';
        assert_eq!(LogoMap::read(&magic), Err(WmlError::Magic));
        let mut depth = whole.clone();
        depth[8] = 8;
        assert_eq!(LogoMap::read(&depth), Err(WmlError::Depth(8)));
        for cut in 0..whole.len() {
            assert!(LogoMap::read(&whole[..cut]).is_err(), "{cut}");
        }
        assert_eq!(
            LogoMap::new(1, 1, vec![[256.0, 0.0, 0.0]]),
            Err(WmlError::Range)
        );
        assert_eq!(LogoMap::new(0, 1, vec![]), Err(WmlError::ZeroDimension));
    }

    #[test]
    fn the_extended_curve_is_the_calibrations_inside_its_range() {
        for v in [0.0, 1.0, 10.31, 64.0, 128.0, 200.5, 255.0] {
            assert_eq!(lin(v), crate::calibrate::to_linear(v), "{v}");
            let l = crate::calibrate::to_linear(v);
            assert_eq!(unlin(l), crate::calibrate::from_linear(l), "{v}");
        }
        // Past the range it goes on, monotonic, and comes back.
        assert!(lin(-4.0) < 0.0 && lin(260.0) > 1.0);
        for v in [-40.0, -4.0, 260.0, 300.0] {
            assert!((unlin(lin(v)) - v).abs() < 1e-9, "{v}");
        }
    }

    /// Today's law is GWT's equation and today's proof, to the bit.
    #[test]
    fn the_encoded_law_without_a_bias_is_the_old_arithmetic() {
        let law = Law {
            model: BlendModel::Encoded,
            bias: None,
            max: 255.0,
        };
        let logo = [252.1, 253.5, 252.8];
        for stored in [
            [0.0, 40.0, 255.0],
            [200.0, 201.0, 202.0],
            [17.0, 250.0, 3.0],
        ] {
            for a in [0.002, 0.1, 0.37, 0.8] {
                let o = law.inverse(stored, a, logo);
                assert_eq!(o, crate::restore::unblend(stored, a, logo));
                for c in 0..3 {
                    assert_eq!(
                        law.forward(o[c], a, logo[c], c),
                        a * logo[c] + (1.0 - a) * o[c]
                    );
                }
            }
        }
    }

    /// The bias moves the proof's interval (D308): a value just under
    /// `α·L` is a blend once the bias says the vendor stored it lower.
    #[test]
    fn a_bias_moves_the_interval_the_proof_allows() {
        let plain = Law {
            model: BlendModel::Encoded,
            bias: None,
            max: 255.0,
        };
        let biased = Law {
            bias: Some([-12.0; 3]),
            ..plain
        };
        let (a, logo) = (0.5, [255.0; 3]);
        // α·L = 127.5; 110 is 17.5 under it, past the 8 allowed.
        let stored = [110.0, 110.0, 110.0];
        assert_eq!(plain.out_of_range(stored, a, logo, 8.0), [true; 3]);
        assert_eq!(biased.out_of_range(stored, a, logo, 8.0), [false; 3]);
    }

    /// Each law inverts what it composites, to the float.
    #[test]
    fn each_law_inverts_its_own_forward_blend() {
        for model in [BlendModel::Encoded, BlendModel::LinearLight] {
            for bias in [None, Some([1.5, -0.5, 0.25])] {
                let law = Law {
                    model,
                    bias,
                    max: 255.0,
                };
                let logo = [250.0, 240.0, 230.0];
                let o = [12.0, 128.0, 201.0];
                let a = 0.4;
                let stored = [0, 1, 2].map(|c| law.forward(o[c], a, logo[c], c));
                let back = law.inverse(stored, a, logo);
                for c in 0..3 {
                    assert!((back[c] - o[c]).abs() < 1e-6, "{model:?} {bias:?}: {back:?}");
                }
                assert_eq!(law.out_of_range(stored, a, logo, 8.0), [false; 3]);
            }
        }
    }
}
