//! Normalised cross-correlation of an image region's luma against a
//! shape — OpenCV's `TM_CCOEFF_NORMED`, the equation only. The image
//! side's mean and variance come from integral images (a sum and a sum
//! of squares), so a window costs one pass over the shape.

use crate::geometry::{PixelRect, Shape};

/// Summed-area tables of a luma plane and of its square, `(w+1)×(h+1)`,
/// in `f64` so a 4K plane sums without losing the variance.
pub(crate) struct Integral {
    width: usize,
    height: usize,
    sum: Vec<f64>,
    sq: Vec<f64>,
}

impl Integral {
    pub fn new(luma: &[f32], width: usize, height: usize) -> Self {
        let stride = width + 1;
        let mut sum = vec![0.0; stride * (height + 1)];
        let mut sq = vec![0.0; stride * (height + 1)];
        for y in 0..height {
            let mut row = 0.0;
            let mut row_sq = 0.0;
            for x in 0..width {
                let v = f64::from(luma[y * width + x]);
                row += v;
                row_sq += v * v;
                sum[(y + 1) * stride + x + 1] = sum[y * stride + x + 1] + row;
                sq[(y + 1) * stride + x + 1] = sq[y * stride + x + 1] + row_sq;
            }
        }
        Integral {
            width,
            height,
            sum,
            sq,
        }
    }

    /// `(Σ v, Σ v²)` over the window.
    pub fn window(&self, r: PixelRect) -> (f64, f64) {
        let stride = self.width + 1;
        let (x0, y0) = (r.x as usize, r.y as usize);
        let (x1, y1) = (x0 + r.width as usize, y0 + r.height as usize);
        let at = |t: &[f64], x: usize, y: usize| t[y * stride + x];
        let s = at(&self.sum, x1, y1) - at(&self.sum, x0, y1) - at(&self.sum, x1, y0)
            + at(&self.sum, x0, y0);
        let q = at(&self.sq, x1, y1) - at(&self.sq, x0, y1) - at(&self.sq, x1, y0)
            + at(&self.sq, x0, y0);
        (s, q)
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }
}

/// A shape with its mean taken out, ready to correlate.
pub(crate) struct Centred {
    pub width: u32,
    pub height: u32,
    values: Vec<f32>,
    /// `Σ (T − T̄)²`.
    norm: f64,
    /// `Σ (T − T̄)` as stored — zero but for rounding, which the
    /// correlation takes back out.
    rest: f64,
}

impl Centred {
    pub fn of(shape: &Shape) -> Self {
        let n = shape.values.len() as f64;
        let mean = shape.values.iter().map(|&v| f64::from(v)).sum::<f64>() / n;
        let values: Vec<f32> = shape
            .values
            .iter()
            .map(|&v| (f64::from(v) - mean) as f32)
            .collect();
        let norm = values.iter().map(|&v| f64::from(v) * f64::from(v)).sum();
        let rest = values.iter().map(|&v| f64::from(v)).sum();
        Centred {
            width: shape.width,
            height: shape.height,
            values,
            norm,
            rest,
        }
    }
}

/// The NCC of the shape placed with its top-left pixel at `(x, y)`, in
/// [−1, 1]; zero when either side is flat. The window must lie inside
/// the plane.
pub(crate) fn ncc(luma: &[f32], integral: &Integral, t: &Centred, x: u32, y: u32) -> f32 {
    let rect = PixelRect {
        x,
        y,
        width: t.width,
        height: t.height,
    };
    debug_assert!(rect.inside(integral.width() as u32, integral.height() as u32));
    let n = f64::from(t.width) * f64::from(t.height);
    let (s, q) = integral.window(rect);
    let var = q - s * s / n;
    // A window flat to within a hundred-thousandth of the range is flat:
    // what is left of its variance is rounding, and dividing by it would
    // turn rounding into a perfect match.
    if var / n < 1e-10 || t.norm / n < 1e-10 {
        return 0.0;
    }
    let den = (var * t.norm).sqrt();
    let stride = integral.width();
    let mut num = 0.0f64;
    for ty in 0..t.height as usize {
        let row = (y as usize + ty) * stride + x as usize;
        let image = &luma[row..row + t.width as usize];
        let shape = &t.values[ty * t.width as usize..(ty + 1) * t.width as usize];
        let mut acc = 0.0f32;
        for (a, b) in image.iter().zip(shape) {
            acc += a * b;
        }
        num += f64::from(acc);
    }
    // Σ (I − Ī)(T − T̄) = Σ I·(T − T̄) − Ī·Σ (T − T̄).
    let num = num - s / n * t.rest;
    ((num / den) as f32).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The integral path against the definition, computed the long way.
    #[test]
    fn ncc_matches_a_direct_computation() {
        let (w, h) = (23usize, 17usize);
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 40) as f32 / (1u64 << 24) as f32
        };
        let luma: Vec<f32> = (0..w * h).map(|_| next()).collect();
        let shape = Shape {
            width: 7,
            height: 5,
            values: (0..35).map(|_| next()).collect(),
            canonical: false,
        };
        let integral = Integral::new(&luma, w, h);
        let centred = Centred::of(&shape);
        for (x, y) in [(0u32, 0u32), (3, 4), (16, 12), (9, 0)] {
            let n = 35.0f64;
            let mut is = Vec::new();
            for ty in 0..5 {
                for tx in 0..7 {
                    is.push(f64::from(luma[(y as usize + ty) * w + x as usize + tx]));
                }
            }
            let ts: Vec<f64> = shape.values.iter().map(|&v| f64::from(v)).collect();
            let im = is.iter().sum::<f64>() / n;
            let tm = ts.iter().sum::<f64>() / n;
            let num: f64 = is.iter().zip(&ts).map(|(i, t)| (i - im) * (t - tm)).sum();
            let vi: f64 = is.iter().map(|i| (i - im) * (i - im)).sum();
            let vt: f64 = ts.iter().map(|t| (t - tm) * (t - tm)).sum();
            let direct = (num / (vi * vt).sqrt()) as f32;
            let fast = ncc(&luma, &integral, &centred, x, y);
            assert!((direct - fast).abs() < 1e-5, "({x}, {y}): {direct} {fast}");
        }
        // A shape against itself is 1, against its negative −1.
        let flat: Vec<f32> = shape.values.clone();
        let itself = Integral::new(&flat, 7, 5);
        assert!((ncc(&flat, &itself, &centred, 0, 0) - 1.0).abs() < 1e-5);
        let negative: Vec<f32> = flat.iter().map(|v| 1.0 - v).collect();
        let neg = Integral::new(&negative, 7, 5);
        assert!((ncc(&negative, &neg, &centred, 0, 0) + 1.0).abs() < 1e-5);
    }

    #[test]
    fn a_flat_window_correlates_with_nothing() {
        let luma = vec![0.5f32; 64];
        let integral = Integral::new(&luma, 8, 8);
        let shape = Shape {
            width: 3,
            height: 3,
            values: vec![0.0, 1.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0, 0.0],
            canonical: false,
        };
        assert_eq!(ncc(&luma, &integral, &Centred::of(&shape), 2, 2), 0.0);
    }

    /// Flat to within rounding is flat: a window whose luma ripples by
    /// 10⁻⁷ in the very shape of the template would correlate perfectly if
    /// its variance were divided by; under the 10⁻¹⁰ floor it scores 0.
    #[test]
    fn a_window_flat_to_rounding_correlates_with_nothing() {
        let values = vec![0.0, 1.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0, 0.0];
        let mut luma = vec![0.5f32; 64];
        for (i, v) in values.iter().enumerate() {
            let (x, y) = (2 + i % 3, 2 + i / 3);
            luma[y * 8 + x] += v * 1e-7 * 4.0;
        }
        let integral = Integral::new(&luma, 8, 8);
        let shape = Shape {
            width: 3,
            height: 3,
            values,
            canonical: false,
        };
        assert_eq!(ncc(&luma, &integral, &Centred::of(&shape), 2, 2), 0.0);
    }
}
