//! Calibration: a vendor's mark recovered from captures — the maths of
//! `docs/sdd/visible-marks.md` §4.6, on rasters someone else decoded. A
//! developer tool's engine (D162): it hands up numbers and a draft
//! profile; `examples/calibrate.rs` reads the files and writes the row,
//! the maps and the report a person commits.
//!
//! The model is `I = (1 − α)·B + α·L` per pixel and channel. One capture
//! on black gives `α·L` and cannot tell the two apart (SDD §1.2); captures
//! on two backgrounds make it a line, `I = a·B + c` with `a = 1 − α` and
//! `c = α·L`, fitted per pixel. Generated "flat" pictures are not flat, so
//! `B` under the mark is a quadratic fitted to a ring around it. Grey
//! captures, kept out of the fit, decide whether the vendor blended the
//! stored values (`encoded`) or light (`linear-light`).

use crate::alpha::AlphaMap;
use crate::geometry::PixelRect;
use crate::raster::Raster;
use crate::{Catalogue, ExamineOptions, Fidelity, Verdict};

/// What a capture shows under the mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Background {
    Black,
    White,
    Grey,
    /// An ordinary picture: used to fit only with its clean twin.
    Content,
}

/// One original download.
#[derive(Debug, Clone)]
pub struct Capture {
    pub name: String,
    pub raster: Raster,
    pub background: Background,
    /// The same generation downloaded without the mark, when the vendor
    /// allows it — the best calibration there is.
    pub clean: Option<Raster>,
}

/// The knobs, with the method's defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalibrateOptions {
    /// `α` at or above: a hole.
    pub opaque_above: f32,
    /// Width of the ring the background is fitted on, in pixels.
    pub ring: u32,
    /// The deviation, in 8-bit levels, that makes a pixel part of the
    /// mark's support.
    pub threshold: f32,
    /// Pixels added around the support's bounding box.
    pub widen: u32,
}

impl Default for CalibrateOptions {
    fn default() -> Self {
        CalibrateOptions {
            opaque_above: 0.95,
            ring: 6,
            threshold: 6.0,
            widen: 4,
        }
    }
}

/// How the vendor blended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendModel {
    /// Linear over the stored code values — the only one restored today.
    Encoded,
    /// Linear in light (sRGB decoded): in the schema, refused by the
    /// catalogue until a vendor needs it (D152).
    LinearLight,
}

impl BlendModel {
    pub fn id(self) -> &'static str {
        match self {
            BlendModel::Encoded => "encoded",
            BlendModel::LinearLight => "linear-light",
        }
    }
}

/// How many captures of each kind went in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    pub black: u32,
    pub white: u32,
    pub grey: u32,
    pub content: u32,
    pub pairs: u32,
}

/// One size's mark, recovered.
#[derive(Debug, Clone, PartialEq)]
pub struct Calibration {
    pub width: u32,
    pub height: u32,
    /// Where the map sits in a picture of this size.
    pub rect: PixelRect,
    pub alpha: AlphaMap,
    /// The logo's colour per channel, 8-bit units.
    pub logo: [f32; 3],
    /// The largest per-pixel departure from `logo` where `α > 0.2`, in
    /// levels: over 2, one colour does not describe the mark.
    pub logo_spread: f32,
    pub model: BlendModel,
    /// Mean error on the grey captures under `[encoded, linear-light]`,
    /// in levels; `None` when no grey capture was left out of the fit.
    pub grey_error: Option<[f32; 2]>,
    /// `R²` of the per-pixel lines where `α > 0.05`: the lowest and the
    /// mean.
    pub r2_min: f32,
    pub r2_mean: f32,
    /// The largest difference between a fitted capture and the model, in
    /// levels.
    pub residual_max: f32,
    pub support: u32,
    pub holes: u32,
    /// Holes above 5 % of the support: restoring is not enough.
    pub needs_reconstruction: bool,
    pub counts: Counts,
}

/// Why captures could not be calibrated.
#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
pub enum CalibrationError {
    #[error("no captures")]
    NoCaptures,
    #[error("the captures are not all one size")]
    MixedSizes,
    #[error("no mark stands out of the flat captures")]
    NoSupport,
    #[error("no ring of background around the mark inside the picture")]
    NoRing,
    #[error("one background cannot separate the opacity from the logo's colour")]
    OneBackground,
    #[error("neither blend model fits the grey captures (encoded {encoded:.2}, linear light {linear:.2} levels)")]
    NotABlend { encoded: f32, linear: f32 },
    #[error("a linear-light mark cannot be written as a profile in this version")]
    LinearLight,
    #[error("the map cannot be written")]
    Map,
}

/// sRGB decoding of an 8-bit-scaled value, to linear light in [0, 1].
/// Public for [`crate::synth`]'s re-export only (D312).
pub fn to_linear(v: f64) -> f64 {
    let c = (v / 255.0).clamp(0.0, 1.0);
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// sRGB encoding of linear light, to 8-bit-scaled units. Public for
/// [`crate::synth`]'s re-export only (D312).
pub fn from_linear(l: f64) -> f64 {
    let l = l.clamp(0.0, 1.0);
    let c = if l <= 0.003_130_8 {
        l * 12.92
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    };
    c * 255.0
}

/// A raster's colour as 8-bit-scaled `f64`, row-major.
fn pixels(r: &Raster) -> Vec<[f64; 3]> {
    let scale = 255.0 / f64::from(r.layout().max());
    r.samples()
        .chunks_exact(r.layout().channels())
        .map(|p| {
            [
                f64::from(p[0]) * scale,
                f64::from(p[1]) * scale,
                f64::from(p[2]) * scale,
            ]
        })
        .collect()
}

/// Per-channel box means of radius `radius`, clipped at the borders.
fn box_mean(px: &[[f64; 3]], w: usize, h: usize, radius: usize) -> Vec<[f64; 3]> {
    let stride = w + 1;
    let mut table = vec![[0f64; 3]; stride * (h + 1)];
    for y in 0..h {
        let mut row = [0f64; 3];
        for x in 0..w {
            for c in 0..3 {
                row[c] += px[y * w + x][c];
                table[(y + 1) * stride + x + 1][c] = table[y * stride + x + 1][c] + row[c];
            }
        }
    }
    let mut out = Vec::with_capacity(w * h);
    for y in 0..h {
        let (y0, y1) = (y.saturating_sub(radius), (y + radius + 1).min(h));
        for x in 0..w {
            let (x0, x1) = (x.saturating_sub(radius), (x + radius + 1).min(w));
            let n = ((x1 - x0) * (y1 - y0)) as f64;
            let mut m = [0f64; 3];
            for (c, v) in m.iter_mut().enumerate() {
                *v = (table[y1 * stride + x1][c]
                    - table[y0 * stride + x1][c]
                    - table[y1 * stride + x0][c]
                    + table[y0 * stride + x0][c])
                    / n;
            }
            out.push(m);
        }
    }
    out
}

/// Least squares of `z ≈ k0 + k1·u + k2·v + k3·u² + k4·uv + k5·v²`; a
/// constant when the points do not determine a quadratic.
fn fit_quadratic(points: &[(f64, f64, f64)]) -> [f64; 6] {
    let basis = |u: f64, v: f64| [1.0, u, v, u * u, u * v, v * v];
    let mut m = [[0f64; 7]; 6];
    for &(u, v, z) in points {
        let b = basis(u, v);
        for i in 0..6 {
            for j in 0..6 {
                m[i][j] += b[i] * b[j];
            }
            m[i][6] += b[i] * z;
        }
    }
    let constant = || {
        let mean = points.iter().map(|p| p.2).sum::<f64>() / points.len().max(1) as f64;
        [mean, 0.0, 0.0, 0.0, 0.0, 0.0]
    };
    // Gaussian elimination with partial pivoting.
    for col in 0..6 {
        let pivot = (col..6)
            .max_by(|&a, &b| m[a][col].abs().total_cmp(&m[b][col].abs()))
            .unwrap_or(col);
        if m[pivot][col].abs() < 1e-9 {
            return constant();
        }
        m.swap(col, pivot);
        let lead = m[col];
        for (row, line) in m.iter_mut().enumerate() {
            if row != col {
                let f = line[col] / lead[col];
                for (v, l) in line.iter_mut().zip(lead.iter()).skip(col) {
                    *v -= f * l;
                }
            }
        }
    }
    let mut k = [0f64; 6];
    for (i, v) in k.iter_mut().enumerate() {
        *v = m[i][6] / m[i][i];
    }
    k
}

/// The background under `rect` of one capture: a quadratic per channel
/// over the ring around it, or the clean twin's own pixels.
fn background(
    capture: &Capture,
    rect: PixelRect,
    ring: u32,
) -> Result<Vec<[f64; 3]>, CalibrationError> {
    let w = capture.raster.width();
    if let Some(clean) = &capture.clean {
        let px = pixels(clean);
        let mut out = Vec::with_capacity(rect.area() as usize);
        for y in rect.y..rect.y + rect.height {
            for x in rect.x..rect.x + rect.width {
                out.push(px[(y * w + x) as usize]);
            }
        }
        return Ok(out);
    }
    ring_background(&capture.raster, rect, ring).ok_or(CalibrationError::NoRing)
}

/// The picture under `rect` as the calibration estimates it with no clean
/// twin: a quadratic in `(x, y)` per channel, fitted by least squares to
/// a ring `ring` pixels wide around the rectangle (clipped at the
/// picture's edges), in 8-bit-scaled units, row-major over `rect`.
/// `None` when the ring has fewer than 30 pixels.
///
/// The calibration's own fit, handed out for the analytics of E12-R4
/// (`map_regress`, and `scripts/analytics/bias.py`'s check of its Python
/// restatement) — a developer's measure, not a feature.
#[doc(hidden)]
pub fn ring_background(raster: &Raster, rect: PixelRect, ring: u32) -> Option<Vec<[f64; 3]>> {
    let (w, h) = (raster.width(), raster.height());
    let px = pixels(raster);
    let (cx, cy) = (
        f64::from(rect.x) + f64::from(rect.width) / 2.0,
        f64::from(rect.y) + f64::from(rect.height) / 2.0,
    );
    let s = f64::from(rect.width.max(rect.height)).max(1.0);
    let x0 = rect.x.saturating_sub(ring);
    let y0 = rect.y.saturating_sub(ring);
    let x1 = (rect.x + rect.width + ring).min(w);
    let y1 = (rect.y + rect.height + ring).min(h);
    let mut ring_points: [Vec<(f64, f64, f64)>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for y in y0..y1 {
        for x in x0..x1 {
            let inside =
                x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height;
            if inside {
                continue;
            }
            let (u, v) = ((f64::from(x) + 0.5 - cx) / s, (f64::from(y) + 0.5 - cy) / s);
            for (c, pts) in ring_points.iter_mut().enumerate() {
                pts.push((u, v, px[(y * w + x) as usize][c]));
            }
        }
    }
    if ring_points[0].len() < 30 {
        return None;
    }
    let k = [
        fit_quadratic(&ring_points[0]),
        fit_quadratic(&ring_points[1]),
        fit_quadratic(&ring_points[2]),
    ];
    let mut out = Vec::with_capacity(rect.area() as usize);
    for y in rect.y..rect.y + rect.height {
        for x in rect.x..rect.x + rect.width {
            let (u, v) = ((f64::from(x) + 0.5 - cx) / s, (f64::from(y) + 0.5 - cy) / s);
            let b = [1.0, u, v, u * u, u * v, v * v];
            let mut p = [0f64; 3];
            for (c, val) in p.iter_mut().enumerate() {
                *val = (0..6).map(|i| k[c][i] * b[i]).sum();
            }
            out.push(p);
        }
    }
    Some(out)
}

/// Where the mark is: the bounding box of the pixels that stand out of
/// the flat captures (or differ from their clean twin), widened.
fn locate(
    captures: &[&Capture],
    options: &CalibrateOptions,
) -> Result<PixelRect, CalibrationError> {
    let (w, h) = (
        captures[0].raster.width() as usize,
        captures[0].raster.height() as usize,
    );
    let radius = (w.max(h) / 8).max(16);
    let mut deviation = vec![0f64; w * h];
    let mut used = 0.0;
    for capture in captures {
        let px = pixels(&capture.raster);
        let reference = match &capture.clean {
            Some(clean) => pixels(clean),
            None if capture.background == Background::Content => continue,
            None => box_mean(&px, w, h, radius),
        };
        for (d, (p, r)) in deviation.iter_mut().zip(px.iter().zip(&reference)) {
            *d += (0..3).map(|c| (p[c] - r[c]).abs()).fold(0.0, f64::max);
        }
        used += 1.0;
    }
    if used == 0.0 {
        return Err(CalibrationError::NoSupport);
    }
    let peak = deviation.iter().copied().fold(0.0, f64::max) / used;
    let t = f64::from(options.threshold).max(0.2 * peak);
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0usize, 0usize);
    for (i, d) in deviation.iter().enumerate() {
        if d / used > t {
            let (x, y) = (i % w, i / w);
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x + 1);
            y1 = y1.max(y + 1);
        }
    }
    if x0 == usize::MAX {
        return Err(CalibrationError::NoSupport);
    }
    let grow = options.widen as usize + (x1 - x0).max(y1 - y0) / 10;
    let x0 = x0.saturating_sub(grow);
    let y0 = y0.saturating_sub(grow);
    let x1 = (x1 + grow).min(w);
    let y1 = (y1 + grow).min(h);
    Ok(PixelRect {
        x: x0 as u32,
        y: y0 as u32,
        width: (x1 - x0) as u32,
        height: (y1 - y0) as u32,
    })
}

/// One fit of `α` and `L` under a transfer: `t` maps an 8-bit value into
/// the space the blend is linear in.
struct Fit {
    alpha: Vec<f64>,
    logo: [f64; 3],
    spread: f64,
    r2: Vec<f64>,
}

/// One capture under the mark: what it shows, and its background.
type Sample = (Vec<[f64; 3]>, Vec<[f64; 3]>);

fn fit(samples: &[Sample], n: usize, t: fn(f64) -> f64, scale: f64) -> Fit {
    // Per pixel and channel: the line I = a·B + c.
    let mut a = vec![[1f64; 3]; n];
    let mut c = vec![[0f64; 3]; n];
    let mut r2 = vec![1f64; n];
    for p in 0..n {
        let mut r2c = [1f64; 3];
        for ch in 0..3 {
            let pts: Vec<(f64, f64)> = samples
                .iter()
                .map(|(i, b)| (t(b[p][ch]) * scale, t(i[p][ch]) * scale))
                .collect();
            let k = pts.len() as f64;
            let mb = pts.iter().map(|q| q.0).sum::<f64>() / k;
            let mi = pts.iter().map(|q| q.1).sum::<f64>() / k;
            let sbb: f64 = pts.iter().map(|q| (q.0 - mb).powi(2)).sum();
            let sbi: f64 = pts.iter().map(|q| (q.0 - mb) * (q.1 - mi)).sum();
            let sii: f64 = pts.iter().map(|q| (q.1 - mi).powi(2)).sum();
            if sbb < 1e-9 {
                continue;
            }
            let slope = sbi / sbb;
            let intercept = mi - slope * mb;
            a[p][ch] = slope;
            c[p][ch] = intercept;
            let res: f64 = pts
                .iter()
                .map(|q| (q.1 - slope * q.0 - intercept).powi(2))
                .sum();
            r2c[ch] = if sii < 1e-9 { 1.0 } else { 1.0 - res / sii };
        }
        r2[p] = (r2c[0] + r2c[1] + r2c[2]) / 3.0;
    }
    let alpha_reg: Vec<[f64; 3]> = a.iter().map(|s| s.map(|v| 1.0 - v)).collect();
    // L per channel: Σc / Σα over the clearly marked pixels.
    let mut logo = [t(255.0) * scale; 3];
    for (ch, l) in logo.iter_mut().enumerate() {
        let (mut sc, mut sa) = (0.0, 0.0);
        for p in 0..n {
            if alpha_reg[p][ch] > 0.1 {
                sc += c[p][ch];
                sa += alpha_reg[p][ch];
            }
        }
        if sa > 1e-9 {
            *l = sc / sa;
        }
    }
    let mut spread = 0f64;
    for p in 0..n {
        for ch in 0..3 {
            if alpha_reg[p][ch] > 0.2 {
                spread = spread.max((c[p][ch] / alpha_reg[p][ch] - logo[ch]).abs());
            }
        }
    }
    // α again, with L known: least squares over every capture and channel.
    let alpha = (0..n)
        .map(|p| {
            let (mut num, mut den) = (0.0, 0.0);
            for (i, b) in samples {
                for ch in 0..3 {
                    let (iv, bv) = (t(i[p][ch]) * scale, t(b[p][ch]) * scale);
                    num += (iv - bv) * (logo[ch] - bv);
                    den += (logo[ch] - bv).powi(2);
                }
            }
            if den < 1e-9 {
                0.0
            } else {
                (num / den).clamp(0.0, 1.0)
            }
        })
        .collect();
    Fit {
        alpha,
        logo,
        spread,
        r2,
    }
}

fn identity(v: f64) -> f64 {
    v
}

/// Recover one size's mark from its captures.
pub fn calibrate(
    captures: &[Capture],
    options: &CalibrateOptions,
) -> Result<Calibration, CalibrationError> {
    let first = captures.first().ok_or(CalibrationError::NoCaptures)?;
    let (w, h) = (first.raster.width(), first.raster.height());
    if captures.iter().any(|c| {
        (c.raster.width(), c.raster.height()) != (w, h)
            || c.clean
                .as_ref()
                .is_some_and(|r| (r.width(), r.height()) != (w, h))
    }) {
        return Err(CalibrationError::MixedSizes);
    }
    let mut counts = Counts::default();
    for c in captures {
        match c.background {
            Background::Black => counts.black += 1,
            Background::White => counts.white += 1,
            Background::Grey => counts.grey += 1,
            Background::Content => counts.content += 1,
        }
        counts.pairs += u32::from(c.clean.is_some());
    }
    let all: Vec<&Capture> = captures.iter().collect();
    let rect = locate(&all, options)?;

    // What fits: the flat captures but grey, and the pairs. Grey is kept
    // out to test the blend model — unless without it there is one
    // background only.
    let usable = |c: &&Capture| c.background != Background::Content || c.clean.is_some();
    let varieties = |set: &[&Capture]| {
        let mut kinds = 0;
        for b in [Background::Black, Background::White, Background::Grey] {
            kinds += u32::from(set.iter().any(|c| c.background == b && c.clean.is_none()));
        }
        kinds + set.iter().filter(|c| c.clean.is_some()).count().min(2) as u32
    };
    let without_grey: Vec<&Capture> = all
        .iter()
        .copied()
        .filter(usable)
        .filter(|c| c.background != Background::Grey)
        .collect();
    let (fitting, grey): (Vec<&Capture>, Vec<&Capture>) = if varieties(&without_grey) >= 2 {
        (
            without_grey,
            all.iter()
                .copied()
                .filter(|c| c.background == Background::Grey && c.clean.is_none())
                .collect(),
        )
    } else {
        (all.iter().copied().filter(usable).collect(), Vec::new())
    };
    if varieties(&fitting) < 2 {
        return Err(CalibrationError::OneBackground);
    }

    let n = rect.area() as usize;
    let observed = |c: &Capture| -> Vec<[f64; 3]> {
        let px = pixels(&c.raster);
        let mut out = Vec::with_capacity(n);
        for y in rect.y..rect.y + rect.height {
            for x in rect.x..rect.x + rect.width {
                out.push(px[(y * w + x) as usize]);
            }
        }
        out
    };
    let mut samples = Vec::with_capacity(fitting.len());
    for c in &fitting {
        samples.push((observed(c), background(c, rect, options.ring)?));
    }
    let encoded = fit(&samples, n, identity, 1.0);
    let linear = fit(&samples, n, to_linear, 1.0);

    // The grey captures choose the model.
    let mut grey_error = None;
    if !grey.is_empty() {
        let (mut e_enc, mut e_lin, mut count) = (0.0, 0.0, 0.0);
        for c in &grey {
            let (i, b) = (observed(c), background(c, rect, options.ring)?);
            for p in 0..n {
                if encoded.alpha[p] <= 0.05 {
                    continue;
                }
                for ch in 0..3 {
                    let ae = encoded.alpha[p];
                    let pe = ae * encoded.logo[ch] + (1.0 - ae) * b[p][ch];
                    let al = linear.alpha[p];
                    let pl = from_linear(al * linear.logo[ch] + (1.0 - al) * to_linear(b[p][ch]));
                    e_enc += (i[p][ch] - pe).abs();
                    e_lin += (i[p][ch] - pl).abs();
                    count += 1.0;
                }
            }
        }
        if count > 0.0 {
            grey_error = Some([(e_enc / count) as f32, (e_lin / count) as f32]);
        }
    }
    let model = match grey_error {
        Some([e, l]) if e.min(l) > 2.0 => {
            return Err(CalibrationError::NotABlend {
                encoded: e,
                linear: l,
            })
        }
        Some([e, l]) if l < e => BlendModel::LinearLight,
        _ => BlendModel::Encoded,
    };
    let chosen = match model {
        BlendModel::Encoded => &encoded,
        BlendModel::LinearLight => &linear,
    };

    // The largest residual of the chosen model over what was fitted.
    let mut residual_max = 0f64;
    for (i, b) in &samples {
        for p in 0..n {
            for ch in 0..3 {
                let a = chosen.alpha[p];
                let pred = match model {
                    BlendModel::Encoded => a * chosen.logo[ch] + (1.0 - a) * b[p][ch],
                    BlendModel::LinearLight => {
                        from_linear(a * chosen.logo[ch] + (1.0 - a) * to_linear(b[p][ch]))
                    }
                };
                residual_max = residual_max.max((i[p][ch] - pred).abs());
            }
        }
    }
    let marked: Vec<usize> = (0..n).filter(|&p| encoded.alpha[p] > 0.05).collect();
    let (r2_min, r2_mean) = if marked.is_empty() {
        (1.0, 1.0)
    } else {
        let min = marked
            .iter()
            .map(|&p| encoded.r2[p])
            .fold(f64::INFINITY, f64::min);
        let mean = marked.iter().map(|&p| encoded.r2[p]).sum::<f64>() / marked.len() as f64;
        (min, mean)
    };

    // Below half a level of opacity is noise, not mark.
    let values: Vec<f32> = chosen
        .alpha
        .iter()
        .map(|&a| if a < 0.5 / 255.0 { 0.0 } else { a as f32 })
        .collect();
    let support = values.iter().filter(|&&a| a >= crate::NOISE_FLOOR).count() as u32;
    if support == 0 {
        return Err(CalibrationError::NoSupport);
    }
    let holes = values
        .iter()
        .filter(|&&a| a >= options.opaque_above)
        .count() as u32;
    let logo = match model {
        BlendModel::Encoded => chosen.logo,
        BlendModel::LinearLight => chosen.logo.map(from_linear),
    };
    let alpha =
        AlphaMap::new(rect.width, rect.height, values).map_err(|_| CalibrationError::Map)?;
    Ok(Calibration {
        width: w,
        height: h,
        rect,
        alpha,
        logo: logo.map(|v| v as f32),
        logo_spread: encoded.spread as f32,
        model,
        grey_error,
        r2_min: r2_min as f32,
        r2_mean: r2_mean as f32,
        residual_max: residual_max as f32,
        support,
        holes,
        needs_reconstruction: u64::from(holes) * 20 > u64::from(support),
        counts,
    })
}

impl Calibration {
    /// The map at depth 16 — a regression gives more than 8 bits.
    pub fn wma(&self) -> Result<Vec<u8>, CalibrationError> {
        self.alpha.write(16).map_err(|_| CalibrationError::Map)
    }

    /// The exact placement row for this size, naming the map `map_id`.
    pub fn placement_json(&self, map_id: &str) -> String {
        format!(
            "{{ \"when\": {{ \"width\": {}, \"height\": {} }}, \"rect\": [{}, {}, {}, {}], \"alpha\": \"{map_id}\" }}",
            self.width, self.height, self.rect.x, self.rect.y, self.rect.width, self.rect.height
        )
    }
}

/// A profile drafted from one calibration per size.
#[derive(Debug, Clone)]
pub struct Draft<'a> {
    pub id: &'a str,
    pub vendor: &'a str,
    pub product: &'a str,
    pub mark: &'a str,
    pub observed_from: Option<&'a str>,
    /// Each size: its calibration, the asset's file name, its sha256.
    pub sizes: Vec<(&'a Calibration, String, String)>,
}

impl Draft<'_> {
    /// The catalogue row: `status: provisional`, an exact `rect` row per
    /// size, the largest map as the search's, the logo the mean of the
    /// sizes'. Refused for a linear-light mark — the catalogue would
    /// refuse it too.
    pub fn to_json(&self) -> Result<String, CalibrationError> {
        if self.sizes.is_empty() {
            return Err(CalibrationError::NoCaptures);
        }
        if self
            .sizes
            .iter()
            .any(|(c, _, _)| c.model == BlendModel::LinearLight)
        {
            return Err(CalibrationError::LinearLight);
        }
        let k = self.sizes.len() as f32;
        let mut logo = [0f32; 3];
        for (c, _, _) in &self.sizes {
            for (l, v) in logo.iter_mut().zip(c.logo) {
                *l += v / k;
            }
        }
        let map_id = |c: &Calibration| format!("{}-{}x{}", self.id, c.width, c.height);
        let alpha: Vec<String> = self
            .sizes
            .iter()
            .map(|(c, asset, sha)| {
                format!(
                    "{{ \"id\": \"{}\", \"asset\": \"{asset}\", \"sha256\": \"{sha}\", \"size\": [{}, {}] }}",
                    map_id(c),
                    c.rect.width,
                    c.rect.height
                )
            })
            .collect();
        let placements: Vec<String> = self
            .sizes
            .iter()
            .map(|(c, _, _)| c.placement_json(&map_id(c)))
            .collect();
        let largest = self
            .sizes
            .iter()
            .max_by_key(|(c, _, _)| c.rect.width)
            .map(|(c, _, _)| *c)
            .ok_or(CalibrationError::NoCaptures)?;
        let observed = self
            .observed_from
            .map_or_else(|| String::from("null"), |m| format!("\"{m}\""));
        Ok(format!(
            r#"{{
      "id": "{id}", "vendor": "{vendor}", "product": "{product}", "mark": "{mark}",
      "observed": {{ "from": {observed}, "until": null }}, "status": "provisional",
      "blend": {{ "model": "encoded", "logo": [{l0}, {l1}, {l2}], "logo_map": null }},
      "opaque_above": 0.95,
      "alpha": [ {alpha} ],
      "placements": [ {placements} ],
      "search": {{ "corner": "bottom-right", "within": [320, 320], "sizes": [24, 160], "alpha": "{search}" }},
      "detect": {{ "min_ncc": 0.70 }},
      "verify": {{ "gain": 0.06, "edge_ratio": 0.30, "out_of_range": 0.01 }},
      "source": {{ "from": "calibration", "commit": null, "licence": null, "copyright": null }}
    }}"#,
            id = self.id,
            vendor = self.vendor,
            product = self.product,
            mark = self.mark,
            l0 = logo[0].round().clamp(0.0, 255.0),
            l1 = logo[1].round().clamp(0.0, 255.0),
            l2 = logo[2].round().clamp(0.0, 255.0),
            alpha = alpha.join(", "),
            placements = placements.join(", "),
            search = map_id(largest),
        ))
    }
}

/// One capture, replayed through a calibrated profile.
#[derive(Debug, Clone, PartialEq)]
pub struct Replay {
    pub name: String,
    pub background: Background,
    /// The best finding's NCC, `k*`, edge ratio, and whether it verified;
    /// `None` when nothing was proposed.
    pub ncc: Option<f32>,
    pub gain: Option<f32>,
    pub edge_ratio: Option<f32>,
    pub verified: bool,
    /// On a flat capture or a pair: the largest difference between the
    /// restored picture and the background under the mark, in levels.
    pub residual: Option<f32>,
}

/// Run a profile over captures — the method's step 7.
pub fn replay(
    catalogue: &Catalogue,
    calibration: &Calibration,
    captures: &[Capture],
    options: &CalibrateOptions,
    source: Fidelity,
) -> Vec<Replay> {
    let examine = ExamineOptions {
        source,
        profiles: None,
    };
    let rect = calibration.rect;
    captures
        .iter()
        .map(|c| {
            let mut raster = c.raster.clone();
            let report = crate::clean(&mut raster, catalogue, &examine);
            let best = report.found.first();
            let residual = if c.background == Background::Content && c.clean.is_none() {
                None
            } else {
                background(c, rect, options.ring).ok().map(|b| {
                    let restored = pixels(&raster);
                    let w = raster.width();
                    let mut worst = 0f64;
                    let mut i = 0;
                    for y in rect.y..rect.y + rect.height {
                        for x in rect.x..rect.x + rect.width {
                            let p = restored[(y * w + x) as usize];
                            for ch in 0..3 {
                                worst = worst.max((p[ch] - b[i][ch]).abs());
                            }
                            i += 1;
                        }
                    }
                    worst as f32
                })
            };
            Replay {
                name: c.name.clone(),
                background: c.background,
                ncc: best.map(|f| f.ncc),
                gain: best.and_then(|f| f.scores.map(|s| s.gain)),
                edge_ratio: best.and_then(|f| f.scores.map(|s| s.edge_ratio)),
                verified: best.is_some_and(|f| matches!(f.verdict, Verdict::Verified(_))),
                residual,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srgb_round_trips() {
        for v in [0.0, 1.0, 10.0, 128.0, 200.0, 255.0] {
            assert!((from_linear(to_linear(v)) - v).abs() < 1e-9, "{v}");
        }
    }

    #[test]
    fn a_quadratic_is_recovered_exactly() {
        let k = [3.0, -1.5, 2.0, 0.5, -0.25, 1.0];
        let mut pts = Vec::new();
        for i in 0..10 {
            for j in 0..10 {
                let (u, v) = (f64::from(i) / 5.0 - 1.0, f64::from(j) / 5.0 - 1.0);
                pts.push((
                    u,
                    v,
                    k[0] + k[1] * u + k[2] * v + k[3] * u * u + k[4] * u * v + k[5] * v * v,
                ));
            }
        }
        let got = fit_quadratic(&pts);
        for (a, b) in got.iter().zip(k) {
            assert!((a - b).abs() < 1e-9, "{got:?}");
        }
        // A line of points does not determine a quadratic: the mean.
        let line: Vec<_> = (0..5).map(|i| (f64::from(i), 0.0, 7.0)).collect();
        assert_eq!(fit_quadratic(&line)[0], 7.0);
    }
}
