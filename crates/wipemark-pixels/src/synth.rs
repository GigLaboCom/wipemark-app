//! Synthetic composition, for the restoration bench and the tests (D312):
//! a known mark over a known background, with the blend a vendor might
//! have used — never a feature, and nothing the catalogue can name.
//!
//! [`composite_with`] generalises [`crate::composite`]: a sub-pixel
//! rectangle and a resampling [`Kernel`], the blend in stored code values
//! ([`BlendModel::Encoded`], what every shipped profile declares) or in
//! linear light ([`BlendModel::LinearLight`], D152), a logo colour that is
//! one colour or one per map sample, a gain `k` on the opacity, a constant
//! bias, and the rounding. At its defaults — `Encoded`, one logo colour,
//! `k = 1`, no bias, rounding half away from zero, a whole-pixel rectangle
//! at the map's own size, `Kernel::Area` — it is `composite`, sample for
//! sample (`composite_with_at_its_defaults_is_composite`).
//!
//! This adds no capability to the catalogue: it still refuses a
//! `linear-light` blend and a `logo_map` (`catalogue.rs`). A bench that
//! composites in linear light measures how the shipped inverse fares on a
//! mark it does not model, which is the point of having it.

use crate::alpha::AlphaMap;
pub use crate::calibrate::{from_linear, to_linear, BlendModel};
use crate::geometry::{self, Kernel, SubRect};
use crate::raster::Raster;

/// The logo's colour, in 8-bit units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogoColor<'a> {
    /// One colour for the whole mark — what every shipped profile declares.
    Global([f32; 3]),
    /// One colour per sample of the map, row-major on the map's own grid
    /// (`map.width() × map.height()`): brought to the rectangle with the
    /// map, weighted by its opacity.
    PerPixel(&'a [[f32; 3]]),
}

/// How a blended value becomes a stored one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rounding {
    /// Half away from zero — what [`crate::composite`] does.
    #[default]
    Round,
    /// Towards zero.
    Truncate,
}

/// Everything about a blend but the map and its place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Blend<'a> {
    pub logo: LogoColor<'a>,
    pub model: BlendModel,
    /// A gain on the opacity: the mark is drawn with `k·α`, clamped to 1.
    pub k: f32,
    /// Added to every blended colour sample, in 8-bit units, before the
    /// rounding — where the mark is drawn (`k·α > 0`) and nowhere else.
    pub bias: [f32; 3],
    pub rounding: Rounding,
}

impl Blend<'_> {
    /// The defaults with `logo`: what [`crate::composite`] draws.
    pub fn encoded(logo: [f32; 3]) -> Blend<'static> {
        Blend {
            logo: LogoColor::Global(logo),
            model: BlendModel::Encoded,
            k: 1.0,
            bias: [0.0; 3],
            rounding: Rounding::Round,
        }
    }
}

/// Stamp `map` onto `raster` at `rect` — its sub-pixel origin and width,
/// the map brought there by `kernel` — with `blend`, per colour channel:
/// `I = α·L + (1 − α)·O` in stored code values, or
/// `I = from_linear(α·lin(L) + (1 − α)·lin(O))` in linear light, `α` the
/// map's opacity times `k`; then the bias, the rounding and the clamp to
/// the range. Pixels past the raster's edge are skipped; the alpha channel
/// is never written. A `PerPixel` logo whose length is not the map's is a
/// caller's mistake and draws nothing.
pub fn composite_with(
    raster: &mut Raster,
    map: &AlphaMap,
    rect: SubRect,
    kernel: Kernel,
    blend: &Blend<'_>,
) {
    let fx = rect.x - rect.x.floor();
    let fy = rect.y - rect.y.floor();
    let Some(shape) = geometry::shape_with(map, rect.size, fx, fy, kernel) else {
        return;
    };
    let Some(at) = geometry::placed(rect, &shape) else {
        return;
    };
    // The logo per pixel of the shape, in 8-bit units.
    let logos: Option<Vec<[f64; 3]>> = match blend.logo {
        LogoColor::Global(_) => None,
        LogoColor::PerPixel(grid) => {
            if grid.len() != map.values().len() {
                return;
            }
            match per_pixel_logo(map, grid, rect, fx, fy, kernel, &shape.values) {
                Some(l) => Some(l),
                None => return,
            }
        }
    };
    let max = f64::from(raster.layout().max());
    // The logo and the bias in stored units, as `composite` scales them.
    let global = match blend.logo {
        LogoColor::Global(l) => l.map(|c| f64::from(c) * max / 255.0),
        LogoColor::PerPixel(_) => [0.0; 3],
    };
    let bias = blend.bias.map(|b| f64::from(b) * max / 255.0);
    let k = f64::from(blend.k);
    let (w, h) = (raster.width(), raster.height());
    for my in 0..shape.height {
        for mx in 0..shape.width {
            let (x, y) = (at.x + mx, at.y + my);
            if x >= w || y >= h {
                continue;
            }
            let p = (my * shape.width + mx) as usize;
            let a = (f64::from(shape.values[p]) * k).min(1.0);
            if a <= 0.0 {
                continue;
            }
            let logo = match &logos {
                None => global,
                Some(l) => l[p].map(|c| c * max / 255.0),
            };
            let i = raster.at(x, y);
            let samples = raster.samples_mut();
            for c in 0..3 {
                let o = f64::from(samples[i + c]);
                let v = match blend.model {
                    BlendModel::Encoded => a * logo[c] + (1.0 - a) * o,
                    BlendModel::LinearLight => {
                        let (l8, o8) = (logo[c] * 255.0 / max, o * 255.0 / max);
                        from_linear(a * to_linear(l8) + (1.0 - a) * to_linear(o8)) * max / 255.0
                    }
                } + bias[c];
                let v = match blend.rounding {
                    Rounding::Round => v.round(),
                    Rounding::Truncate => v.trunc(),
                };
                samples[i + c] = v.clamp(0.0, max) as u16;
            }
        }
    }
}

/// A per-sample logo brought to the shape: `α·L` and `α` resampled alike,
/// one divided by the other — so the colour is weighted by where the mark
/// is, and a sample with no opacity lends no colour. At the map's own size
/// and a whole-pixel origin it is the grid itself.
fn per_pixel_logo(
    map: &AlphaMap,
    grid: &[[f32; 3]],
    rect: SubRect,
    fx: f32,
    fy: f32,
    kernel: Kernel,
    alpha: &[f32],
) -> Option<Vec<[f64; 3]>> {
    if rect.size == map.width() as f32 && fx == 0.0 && fy == 0.0 {
        return Some(grid.iter().map(|l| l.map(f64::from)).collect());
    }
    let mut out = vec![[0f64; 3]; alpha.len()];
    for c in 0..3 {
        let weighted: Vec<f32> = map
            .values()
            .iter()
            .zip(grid)
            .map(|(a, l)| (a * l[c] / 255.0).clamp(0.0, 1.0))
            .collect();
        let premultiplied = AlphaMap::new(map.width(), map.height(), weighted).ok()?;
        let shaped = geometry::shape_with(&premultiplied, rect.size, fx, fy, kernel)?;
        for (p, (num, den)) in shaped.values.iter().zip(alpha).enumerate() {
            if *den > 0.0 {
                out[p][c] = f64::from(num / den) * 255.0;
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raster::Layout;

    /// A flat mid-grey raster, `w × h`, 8-bit RGB.
    fn grey(w: u32, h: u32, v: u16) -> Raster {
        Raster::new(w, h, Layout::Rgb8, vec![v; (w * h * 3) as usize]).unwrap()
    }

    /// Mid-grey under a white mark: where the mark is more than a third
    /// opaque, the two models disagree by more than a level — the switch
    /// acts, and acts where the eye would see it.
    #[test]
    fn a_linear_light_composite_is_not_the_encoded_one() {
        // α from 0.30 to 0.90 across an 8 × 8 map.
        let values: Vec<f32> = (0..64).map(|i| 0.3 + 0.6 * i as f32 / 63.0).collect();
        let map = AlphaMap::new(8, 8, values.clone()).unwrap();
        let at = SubRect {
            x: 4.0,
            y: 4.0,
            size: 8.0,
        };
        for logo in [[255.0; 3], [252.1, 253.5, 252.8]] {
            let mut encoded = grey(16, 16, 128);
            composite_with(&mut encoded, &map, at, Kernel::Area, &Blend::encoded(logo));
            let mut linear = grey(16, 16, 128);
            let blend = Blend {
                model: BlendModel::LinearLight,
                ..Blend::encoded(logo)
            };
            composite_with(&mut linear, &map, at, Kernel::Area, &blend);
            for (p, a) in values.iter().enumerate() {
                let (x, y) = (4 + p as u32 % 8, 4 + p as u32 / 8);
                let i = ((y * 16 + x) * 3) as usize;
                for c in 0..3 {
                    let (e, l) = (encoded.samples()[i + c], linear.samples()[i + c]);
                    assert!(e.abs_diff(l) > 1, "α {a}: encoded {e}, linear {l}");
                    // Linear light lifts a dark original more: the mark
                    // reads brighter.
                    assert!(l > e, "α {a}: encoded {e}, linear {l}");
                }
            }
        }
    }

    /// `k` scales the opacity, `bias` lifts what was drawn and nothing
    /// else, `Truncate` rounds towards zero.
    #[test]
    fn the_gain_the_bias_and_the_rounding_do_what_they_say() {
        let map = AlphaMap::new(2, 1, vec![0.5, 0.0]).unwrap();
        let at = SubRect {
            x: 0.0,
            y: 0.0,
            size: 2.0,
        };
        let draw = |blend: &Blend<'_>| {
            let mut r = grey(2, 1, 100);
            composite_with(&mut r, &map, at, Kernel::Area, blend);
            [r.samples()[0], r.samples()[3]]
        };
        let base = Blend::encoded([200.0; 3]);
        // 0.5·200 + 0.5·100 = 150; the pixel under α = 0 is untouched.
        assert_eq!(draw(&base), [150, 100]);
        // k = 0.5: 0.25·200 + 0.75·100 = 125.
        assert_eq!(draw(&Blend { k: 0.5, ..base }), [125, 100]);
        // A bias of 2.6: 152.6 rounds to 153, truncates to 152.
        let biased = Blend {
            bias: [2.6; 3],
            ..base
        };
        assert_eq!(draw(&biased), [153, 100]);
        assert_eq!(
            draw(&Blend {
                rounding: Rounding::Truncate,
                ..biased
            }),
            [152, 100]
        );
    }

    /// One logo colour per map sample: each pixel of a map drawn at its
    /// own size takes its own sample's colour.
    #[test]
    fn a_per_pixel_logo_is_read_on_the_maps_grid() {
        let map = AlphaMap::new(2, 1, vec![1.0, 1.0]).unwrap();
        let logo = [[10.0, 20.0, 30.0], [200.0, 210.0, 220.0]];
        let mut r = grey(2, 1, 0);
        let blend = Blend {
            logo: LogoColor::PerPixel(&logo),
            ..Blend::encoded([0.0; 3])
        };
        composite_with(
            &mut r,
            &map,
            SubRect {
                x: 0.0,
                y: 0.0,
                size: 2.0,
            },
            Kernel::Area,
            &blend,
        );
        assert_eq!(r.samples(), &[10, 20, 30, 200, 210, 220]);
        // A logo of the wrong length draws nothing.
        let mut r = grey(2, 1, 0);
        let short = [[1.0; 3]];
        let blend = Blend {
            logo: LogoColor::PerPixel(&short),
            ..Blend::encoded([0.0; 3])
        };
        composite_with(
            &mut r,
            &map,
            SubRect {
                x: 0.0,
                y: 0.0,
                size: 2.0,
            },
            Kernel::Area,
            &blend,
        );
        assert_eq!(r.samples(), &[0; 6]);
    }
}
