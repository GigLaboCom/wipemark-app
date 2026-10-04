//! Where a mark sits, and what its opacity map looks like there.
//!
//! A map of `mw × mh` samples placed at a sub-pixel origin `(x, y)` with
//! a width of `size` image pixels covers the continuous rectangle
//! `[x, x + size) × [y, y + size·mh/mw)`. Each image pixel's `α` is the
//! **area-weighted mean** of the map samples its footprint covers, the
//! map read as one constant value per sample. That is an exact
//! integral, not a sampling: at the map's own size and an integer
//! origin every pixel covers exactly one sample with weight one, so the
//! shape *is* the map, value for value
//! (`a_template_at_native_size_is_the_map`).

use serde::Serialize;

use crate::alpha::AlphaMap;

/// A mark's rectangle in image pixels, sub-pixel: origin and width; the
/// height follows the map's aspect.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SubRect {
    pub x: f32,
    pub y: f32,
    pub size: f32,
}

/// A rectangle of whole pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PixelRect {
    pub fn area(self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

    /// Intersection over union.
    pub fn iou(self, other: PixelRect) -> f32 {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = (self.x + self.width).min(other.x + other.width);
        let y1 = (self.y + self.height).min(other.y + other.height);
        if x1 <= x0 || y1 <= y0 {
            return 0.0;
        }
        let inter = u64::from(x1 - x0) * u64::from(y1 - y0);
        let union = self.area() + other.area() - inter;
        inter as f32 / union as f32
    }

    /// Whether the rectangle lies inside a `width × height` image.
    pub fn inside(self, width: u32, height: u32) -> bool {
        self.x.checked_add(self.width).is_some_and(|r| r <= width)
            && self.y.checked_add(self.height).is_some_and(|b| b <= height)
    }
}

/// How a map is brought to a size it was not drawn at — the filter a
/// picture was scaled with, applied to the mark that was in it (D238).
/// A vendor that stamps a 96-pixel mark on a large picture and hands out a
/// smaller one has scaled the mark with whatever filter scaled the picture;
/// restoring it with another leaves the difference along the contour as
/// an outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Kernel {
    /// The exact area integral (OpenCV's `INTER_AREA` when shrinking).
    #[default]
    Area,
    /// Bilinear, widened by the scale when shrinking (a triangle filter).
    Triangle,
    /// Catmull-Rom cubic, likewise.
    Cubic,
    /// Lanczos with three lobes, likewise.
    Lanczos3,
}

impl Kernel {
    /// Every kernel, the area integral first.
    pub const ALL: [Kernel; 4] = [
        Kernel::Area,
        Kernel::Triangle,
        Kernel::Cubic,
        Kernel::Lanczos3,
    ];

    /// The kernel's reach, in pixels of the scale it is applied at.
    fn support(self) -> f32 {
        match self {
            Kernel::Area => 0.5,
            Kernel::Triangle => 1.0,
            Kernel::Cubic => 2.0,
            Kernel::Lanczos3 => 3.0,
        }
    }

    fn weight(self, t: f32) -> f32 {
        let t = t.abs();
        match self {
            Kernel::Area => f32::from(u8::from(t < 0.5)),
            Kernel::Triangle => (1.0 - t).max(0.0),
            Kernel::Cubic => {
                // Catmull-Rom (B = 0, C = 1/2).
                if t < 1.0 {
                    1.5 * t * t * t - 2.5 * t * t + 1.0
                } else if t < 2.0 {
                    -0.5 * t * t * t + 2.5 * t * t - 4.0 * t + 2.0
                } else {
                    0.0
                }
            }
            Kernel::Lanczos3 => {
                if t < 1e-6 {
                    1.0
                } else if t < 3.0 {
                    let x = std::f32::consts::PI * t;
                    3.0 * x.sin() * (x / 3.0).sin() / (x * x)
                } else {
                    0.0
                }
            }
        }
    }
}

/// A map resampled for one size and sub-pixel phase, independent of
/// where it is put: `values` is `width × height`, row-major, and its
/// top-left pixel is the one holding `(floor(x), floor(y))`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Shape {
    pub width: u32,
    pub height: u32,
    pub values: Vec<f32>,
    /// The map itself: its own size, an integer origin.
    pub canonical: bool,
}

/// The height a map of `map` takes at a width of `size`.
pub(crate) fn height_for(map: &AlphaMap, size: f32) -> f32 {
    size * map.height() as f32 / map.width() as f32
}

/// The map at width `size`, its origin `(fx, fy)` past a whole pixel
/// (each in [0, 1)), by the area integral. `None` for a size below one
/// pixel.
pub(crate) fn shape(map: &AlphaMap, size: f32, fx: f32, fy: f32) -> Option<Shape> {
    shape_with(map, size, fx, fy, Kernel::Area)
}

/// [`shape`] by `kernel`. At the map's own size and an integer origin
/// every kernel gives the map itself.
pub(crate) fn shape_with(
    map: &AlphaMap,
    size: f32,
    fx: f32,
    fy: f32,
    kernel: Kernel,
) -> Option<Shape> {
    if size.is_nan() || size < 1.0 || !(0.0..1.0).contains(&fx) || !(0.0..1.0).contains(&fy) {
        return None;
    }
    let mw = map.width() as f32;
    let height = height_for(map, size);
    let width_px = (fx + size).ceil() as u32;
    let height_px = (fy + height).ceil() as u32;
    let canonical = size == mw && fx == 0.0 && fy == 0.0;
    if canonical {
        return Some(Shape {
            width: map.width(),
            height: map.height(),
            values: map.values().to_vec(),
            canonical,
        });
    }
    let values = match kernel {
        Kernel::Area => resample(map, size, fx, fy, width_px, height_px),
        _ => filtered(map, size, fx, fy, width_px, height_px, kernel),
    };
    Some(Shape {
        width: width_px,
        height: height_px,
        values,
        canonical: false,
    })
}

/// One axis of a filtered resample: for each of `out` pixels, the map
/// samples it reads and their weights, normalised over the whole kernel —
/// a sample past the map's edge is a zero, not dropped, because the mark
/// fades to nothing there.
fn taps(len: usize, scale: f32, phase: f32, out: u32, kernel: Kernel) -> Vec<Vec<(usize, f32)>> {
    // Map units per image pixel, and the kernel widened when shrinking.
    let step = 1.0 / scale;
    let widen = step.max(1.0);
    let reach = kernel.support() * widen;
    (0..out)
        .map(|p| {
            let centre = (p as f32 + 0.5 - phase) * step;
            let (lo, hi) = (
                (centre - reach).floor() as i64,
                (centre + reach).ceil() as i64,
            );
            let mut taps = Vec::new();
            let mut total = 0.0;
            for k in lo..=hi {
                let w = kernel.weight((k as f32 + 0.5 - centre) / widen);
                if w == 0.0 {
                    continue;
                }
                total += w;
                if k >= 0 && (k as usize) < len {
                    taps.push((k as usize, w));
                }
            }
            if total != 0.0 {
                for t in &mut taps {
                    t.1 /= total;
                }
            }
            taps
        })
        .collect()
}

/// The filtered path of [`shape_with`]: separable, clamped to [0, 1].
fn filtered(
    map: &AlphaMap,
    size: f32,
    fx: f32,
    fy: f32,
    width_px: u32,
    height_px: u32,
    kernel: Kernel,
) -> Vec<f32> {
    let (mw, mh) = (map.width() as usize, map.height() as usize);
    let scale_x = size / mw as f32;
    let scale_y = height_for(map, size) / mh as f32;
    let xs = taps(mw, scale_x, fx, width_px, kernel);
    let ys = taps(mh, scale_y, fy, height_px, kernel);
    // Rows first: every map row at the output's columns.
    let mut rows = vec![0f32; mh * width_px as usize];
    for y in 0..mh {
        for (px, tx) in xs.iter().enumerate() {
            rows[y * width_px as usize + px] = tx
                .iter()
                .map(|&(k, w)| w * map.get(k as i64, y as i64))
                .sum();
        }
    }
    let mut values = Vec::with_capacity(width_px as usize * height_px as usize);
    for ty in &ys {
        for px in 0..width_px as usize {
            let v: f32 = ty
                .iter()
                .map(|&(k, w)| w * rows[k * width_px as usize + px])
                .sum();
            values.push(v.clamp(0.0, 1.0));
        }
    }
    values
}

/// The general path of [`shape`]: every pixel of a `width_px ×
/// height_px` grid, its footprint mapped into the map and integrated.
fn resample(
    map: &AlphaMap,
    size: f32,
    fx: f32,
    fy: f32,
    width_px: u32,
    height_px: u32,
) -> Vec<f32> {
    let mw = map.width() as f32;
    let mh = map.height() as f32;
    // Map units per image pixel.
    let sx = mw / size;
    let sy = mh / height_for(map, size);
    let mut values = Vec::with_capacity(width_px as usize * height_px as usize);
    for py in 0..height_px {
        let b0 = ((py as f32 - fy) * sy).max(0.0);
        let b1 = ((py as f32 + 1.0 - fy) * sy).min(mh);
        for px in 0..width_px {
            let a0 = ((px as f32 - fx) * sx).max(0.0);
            let a1 = ((px as f32 + 1.0 - fx) * sx).min(mw);
            values.push(integral(map, a0, a1, b0, b1) / (sx * sy));
        }
    }
    values
}

/// The integral of the map over `[a0, a1) × [b0, b1)` in map units, the
/// map one constant per sample. Zero for an empty rectangle.
fn integral(map: &AlphaMap, a0: f32, a1: f32, b0: f32, b1: f32) -> f32 {
    if a1 <= a0 || b1 <= b0 {
        return 0.0;
    }
    let mut sum = 0.0;
    let (k0, k1) = (a0.floor() as i64, a1.ceil() as i64);
    let (l0, l1) = (b0.floor() as i64, b1.ceil() as i64);
    for l in l0..l1 {
        let wy = (b1.min(l as f32 + 1.0) - b0.max(l as f32)).max(0.0);
        if wy == 0.0 {
            continue;
        }
        for k in k0..k1 {
            let wx = (a1.min(k as f32 + 1.0) - a0.max(k as f32)).max(0.0);
            sum += wx * wy * map.get(k, l);
        }
    }
    sum
}

/// Where a shape built for `rect`'s phase lands in the image.
pub(crate) fn placed(rect: SubRect, shape: &Shape) -> Option<PixelRect> {
    let (x, y) = (rect.x.floor(), rect.y.floor());
    if x < 0.0 || y < 0.0 || !x.is_finite() || !y.is_finite() {
        return None;
    }
    Some(PixelRect {
        x: x as u32,
        y: y as u32,
        width: shape.width,
        height: shape.height,
    })
}

/// The shape for a sub-pixel rectangle, and where it lands.
pub(crate) fn template(map: &AlphaMap, rect: SubRect) -> Option<(Shape, PixelRect)> {
    template_with(map, rect, Kernel::Area)
}

/// Under this, a map sample with no part of the mark near it is the
/// capture's noise, not the vendor's α (D241). GWT's maps are 8-bit
/// captures of the logo over black: the 96-pixel V1 map carries 1–6/255
/// over five and a half thousand samples of its square, then a gap, then
/// the sparkle's edge (8–19) and body (20 and up). Subtracting that noise
/// darkens the whole square by about a level — on a real Gemini output,
/// where the vendor drew nothing there, it leaves a visible square.
pub const CAPTURE_NOISE: f32 = 7.0 / 255.0;
/// What counts as the mark's body when deciding whether a faint sample is
/// its edge or the capture's noise.
const BODY: f32 = 20.0 / 255.0;
/// How near the body a faint sample must be to be its edge.
const NEAR: i64 = 2;

/// `values` (`width × height`) with every sample under [`CAPTURE_NOISE`]
/// that has no body sample within [`NEAR`] pixels set to zero.
pub(crate) fn denoised(width: u32, height: u32, values: Vec<f32>) -> Vec<f32> {
    let (w, h) = (i64::from(width), i64::from(height));
    let at = |x: i64, y: i64| values[(y * w + x) as usize];
    let near_body = |x: i64, y: i64| {
        (-NEAR..=NEAR).any(|dy| {
            (-NEAR..=NEAR).any(|dx| {
                let (nx, ny) = (x + dx, y + dy);
                nx >= 0 && ny >= 0 && nx < w && ny < h && at(nx, ny) >= BODY
            })
        })
    };
    let mut out = values.clone();
    for y in 0..h {
        for x in 0..w {
            let v = at(x, y);
            if v > 0.0 && v < CAPTURE_NOISE && !near_body(x, y) {
                out[(y * w + x) as usize] = 0.0;
            }
        }
    }
    out
}

/// [`template`] by `kernel`. The shape's capture noise is taken out
/// ([`CAPTURE_NOISE`]): a template is what the vendor drew.
pub(crate) fn template_with(
    map: &AlphaMap,
    rect: SubRect,
    kernel: Kernel,
) -> Option<(Shape, PixelRect)> {
    let mut shape = shape_with(
        map,
        rect.size,
        rect.x - rect.x.floor(),
        rect.y - rect.y.floor(),
        kernel,
    )?;
    shape.values = denoised(shape.width, shape.height, std::mem::take(&mut shape.values));
    let at = placed(rect, &shape)?;
    Some((shape, at))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(w: u32, h: u32) -> AlphaMap {
        let values = (0..w * h).map(|i| (i % 7) as f32 / 10.0).collect();
        AlphaMap::new(w, h, values).unwrap()
    }

    #[test]
    fn a_template_at_native_size_is_the_map() {
        let map = ramp(5, 4);
        // The canonical path, and the general one at the same geometry:
        // both must be the map exactly.
        let s = shape(&map, 5.0, 0.0, 0.0).unwrap();
        assert!(s.canonical);
        assert_eq!(s.values, map.values());
        assert_eq!(resample(&map, 5.0, 0.0, 0.0, 5, 4), map.values());
    }

    #[test]
    fn a_resampled_shape_keeps_the_maps_mass() {
        // The integral is exact: the sum over the shape, times the pixel's
        // area in map units, is the map's sum.
        let map = ramp(8, 8);
        let total: f32 = map.values().iter().sum();
        for (size, fx, fy) in [(4.0, 0.0, 0.0), (11.5, 0.25, 0.5), (16.0, 0.75, 0.0)] {
            let s = shape(&map, size, fx, fy).unwrap();
            let scale = (8.0 / size) * (8.0 / size);
            let mass: f32 = s.values.iter().sum::<f32>() * scale;
            assert!(
                (mass - total).abs() < 1e-3,
                "{size} {fx} {fy}: {mass} {total}"
            );
            assert!(!s.canonical);
        }
        assert!(shape(&map, 0.5, 0.0, 0.0).is_none());
        assert!(shape(&map, 8.0, 1.0, 0.0).is_none());
    }

    #[test]
    fn every_kernel_keeps_the_map_at_its_own_size_and_its_mass_when_shrinking() {
        let map = ramp(8, 8);
        let total: f32 = map.values().iter().sum();
        for kernel in Kernel::ALL {
            let native = shape_with(&map, 8.0, 0.0, 0.0, kernel).unwrap();
            assert_eq!(native.values, map.values(), "{kernel:?}");
            let small = shape_with(&map, 3.0, 0.25, 0.5, kernel).unwrap();
            let mass: f32 = small.values.iter().sum::<f32>() * (8.0 / 3.0) * (8.0 / 3.0);
            assert!(
                (mass - total).abs() < total * 0.15,
                "{kernel:?}: {mass} {total}"
            );
        }
        // Not one filter under another name.
        let a = shape_with(&map, 3.0, 0.25, 0.5, Kernel::Area).unwrap();
        for kernel in [Kernel::Triangle, Kernel::Cubic, Kernel::Lanczos3] {
            assert_ne!(
                a.values,
                shape_with(&map, 3.0, 0.25, 0.5, kernel).unwrap().values
            );
        }
    }

    #[test]
    fn iou_is_symmetric_and_bounded() {
        let a = PixelRect {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        };
        let b = PixelRect {
            x: 5,
            y: 0,
            width: 10,
            height: 10,
        };
        assert!((a.iou(b) - 50.0 / 150.0).abs() < 1e-6);
        assert_eq!(a.iou(b), b.iou(a));
        assert_eq!(a.iou(a), 1.0);
        assert_eq!(
            a.iou(PixelRect {
                x: 20,
                y: 20,
                width: 1,
                height: 1
            }),
            0.0
        );
        assert!(a.inside(10, 10) && !b.inside(14, 10));
    }
}
