//! Calibration against a synthetic vendor: a known map and a tinted logo
//! composited over generated "flat" captures with noise and a vignette —
//! stored losslessly and through a real JPEG encoder — and recovered.

mod support;

use support::*;
use wipemark_pixels::{
    calibrate, replay, AlphaMap, Background, BlendModel, CalibrateOptions, Calibration,
    CalibrationError, Capture, Catalogue, Draft, Fidelity, Layout, PixelRect, Raster,
};

const W: u32 = 256;
const H: u32 = 192;
/// A logo that is not white: black alone cannot calibrate it (SDD §1.2).
const LOGO: [f32; 3] = [242.0, 246.0, 255.0];

fn truth() -> AlphaMap {
    blurred(&sparkle(48, 0.45), 1)
}

fn at() -> PixelRect {
    small_row(W, H, 48)
}

/// A generated "flat" picture: `level` with a vignette that darkens the
/// corners by 8 % and ±`noise` levels per sample.
fn flat(level: f32, seed: u64, noise: u32) -> Raster {
    let mut rng = Rng::new(seed);
    let (cx, cy) = (W as f32 / 2.0, H as f32 / 2.0);
    let r_max = (cx * cx + cy * cy).sqrt();
    let mut samples = Vec::with_capacity((W * H * 3) as usize);
    for y in 0..H {
        for x in 0..W {
            let r = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt() / r_max;
            let v = level * (1.0 - 0.08 * r * r);
            for _ in 0..3 {
                let n = rng.below(2 * noise + 1) as f32 - noise as f32;
                samples.push((v + n).round().clamp(0.0, 255.0) as u16);
            }
        }
    }
    Raster::new(W, H, Layout::Rgb8, samples).unwrap()
}

fn to_linear(v: f64) -> f64 {
    let c = (v / 255.0).clamp(0.0, 1.0);
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn from_linear(l: f64) -> f64 {
    let l = l.clamp(0.0, 1.0);
    255.0
        * if l <= 0.003_130_8 {
            l * 12.92
        } else {
            1.055 * l.powf(1.0 / 2.4) - 0.055
        }
}

/// The vendor stamps its mark: over the stored values, or in light.
fn stamp(raster: &mut Raster, model: BlendModel) {
    match model {
        BlendModel::Encoded => wipemark_pixels::composite(raster, &truth(), at(), LOGO),
        BlendModel::LinearLight => {
            let map = truth();
            let mut samples = raster.samples().to_vec();
            for y in 0..map.height() {
                for x in 0..map.width() {
                    let a = f64::from(map.get(i64::from(x), i64::from(y)));
                    let i = (((at().y + y) * W + at().x + x) * 3) as usize;
                    for c in 0..3 {
                        let o = to_linear(f64::from(samples[i + c]));
                        let l = to_linear(f64::from(LOGO[c]));
                        samples[i + c] = from_linear(a * l + (1.0 - a) * o).round() as u16;
                    }
                }
            }
            *raster = Raster::new(W, H, Layout::Rgb8, samples).unwrap();
        }
    }
}

/// Through `image`'s JPEG encoder at quality 95 and back.
fn jpeg(raster: &Raster) -> Raster {
    let bytes: Vec<u8> = raster.samples().iter().map(|&s| s as u8).collect();
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 95)
        .encode(&bytes, W, H, image::ExtendedColorType::Rgb8)
        .unwrap();
    let decoded = image::load_from_memory_with_format(&out, image::ImageFormat::Jpeg)
        .unwrap()
        .to_rgb8();
    Raster::from_u8(W, H, Layout::Rgb8, decoded.as_raw()).unwrap()
}

/// Five on black, five on white, three on grey.
fn captures(model: BlendModel, lossy: bool) -> Vec<Capture> {
    let mut out = Vec::new();
    for (background, level, count) in [
        (Background::Black, 8.0, 5u64),
        (Background::White, 245.0, 5),
        (Background::Grey, 128.0, 3),
    ] {
        for k in 0..count {
            let mut raster = flat(level, 100 * level as u64 + k, 1);
            stamp(&mut raster, model);
            if lossy {
                raster = jpeg(&raster);
            }
            out.push(Capture {
                name: format!("{background:?}-{k}"),
                raster,
                background,
                clean: None,
            });
        }
    }
    out
}

/// The 99th percentile and the largest error of the recovered map, in
/// levels of 255, over both rectangles.
fn alpha_error(c: &Calibration) -> (f32, f32) {
    let (t, r) = (truth(), at());
    let x0 = c.rect.x.min(r.x);
    let y0 = c.rect.y.min(r.y);
    let x1 = (c.rect.x + c.rect.width).max(r.x + r.width);
    let y1 = (c.rect.y + c.rect.height).max(r.y + r.height);
    let mut errors = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            let est = c.alpha.get(
                i64::from(x) - i64::from(c.rect.x),
                i64::from(y) - i64::from(c.rect.y),
            );
            let tru = t.get(i64::from(x) - i64::from(r.x), i64::from(y) - i64::from(r.y));
            errors.push((est - tru).abs() * 255.0);
        }
    }
    errors.sort_by(f32::total_cmp);
    let p99 = errors[(errors.len() * 99) / 100];
    (p99, *errors.last().unwrap())
}

fn logo_error(c: &Calibration) -> f32 {
    (0..3)
        .map(|i| (c.logo[i] - LOGO[i]).abs())
        .fold(0.0, f32::max)
}

#[test]
fn a_synthetic_vendor_is_recovered_from_lossless_captures() {
    let c = calibrate(
        &captures(BlendModel::Encoded, false),
        &CalibrateOptions::default(),
    )
    .unwrap();
    assert_eq!(c.model, BlendModel::Encoded);
    let r = at();
    assert!(
        c.rect.x <= r.x
            && c.rect.y <= r.y
            && c.rect.x + c.rect.width >= r.x + r.width
            && c.rect.y + c.rect.height >= r.y + r.height,
        "{:?} does not hold {r:?}",
        c.rect
    );
    let (p99, max) = alpha_error(&c);
    assert!(
        p99 <= 1.0 && max <= 2.0,
        "alpha off by {p99} (p99), {max} (max) levels"
    );
    assert!(logo_error(&c) <= 1.0, "logo {:?}", c.logo);
    assert_eq!((c.counts.black, c.counts.white, c.counts.grey), (5, 5, 3));
    assert!(c.grey_error.is_some());
}

#[test]
fn a_synthetic_vendor_is_recovered_from_jpeg_captures() {
    let c = calibrate(
        &captures(BlendModel::Encoded, true),
        &CalibrateOptions::default(),
    )
    .unwrap();
    let (p99, max) = alpha_error(&c);
    assert!(p99 <= 3.0, "alpha off by {p99} (p99), {max} (max) levels");
    assert!(logo_error(&c) <= 2.0, "logo {:?}", c.logo);
}

/// The grey captures, kept out of the fit, tell a vendor that blended
/// light from one that blended stored values — and a linear-light mark is
/// not written as a profile this version would refuse.
#[test]
fn the_grey_captures_choose_the_blend_model() {
    let linear = calibrate(
        &captures(BlendModel::LinearLight, false),
        &CalibrateOptions::default(),
    )
    .unwrap();
    assert_eq!(
        linear.model,
        BlendModel::LinearLight,
        "{:?}",
        linear.grey_error
    );
    let [e, l] = linear.grey_error.unwrap();
    assert!(l < e, "{e} {l}");
    let draft = Draft {
        id: "test-linear",
        vendor: "test",
        product: "synthetic",
        mark: "sparkle",
        observed_from: None,
        sizes: vec![(&linear, String::from("x.wma"), String::from("0"))],
    };
    assert_eq!(draft.to_json(), Err(CalibrationError::LinearLight));

    let encoded = calibrate(
        &captures(BlendModel::Encoded, false),
        &CalibrateOptions::default(),
    )
    .unwrap();
    assert_eq!(encoded.model, BlendModel::Encoded);
}

#[test]
fn one_background_cannot_separate_alpha_from_the_logo() {
    let black: Vec<Capture> = captures(BlendModel::Encoded, false)
        .into_iter()
        .filter(|c| c.background == Background::Black)
        .collect();
    assert_eq!(
        calibrate(&black, &CalibrateOptions::default()),
        Err(CalibrationError::OneBackground)
    );
}

#[test]
fn an_opaque_mark_needs_reconstruction() {
    let map = opaque_core(48);
    let mut list = Vec::new();
    for (background, level) in [(Background::Black, 8.0), (Background::White, 245.0)] {
        for k in 0..3u64 {
            let mut raster = flat(level, 7 + k, 1);
            wipemark_pixels::composite(&mut raster, &map, at(), LOGO);
            list.push(Capture {
                name: format!("{background:?}-{k}"),
                raster,
                background,
                clean: None,
            });
        }
    }
    let c = calibrate(&list, &CalibrateOptions::default()).unwrap();
    assert!(c.holes > 0, "{c:?}");
    assert!(c.needs_reconstruction);
}

/// The profile drafted from a calibration finds, proves and restores the
/// mark on the captures it came from and on pictures it never saw.
#[test]
fn the_calibrated_profile_restores_its_own_captures() {
    let list = captures(BlendModel::Encoded, false);
    let c = calibrate(&list, &CalibrateOptions::default()).unwrap();
    let wma = c.wma().unwrap();
    let sha = sha256_hex(&wma);
    let draft = Draft {
        id: "test-calibrated",
        vendor: "test",
        product: "synthetic",
        mark: "sparkle",
        observed_from: Some("2026-10"),
        sizes: vec![(&c, String::from("calibrated.wma"), sha)],
    };
    let row = draft.to_json().unwrap();
    let catalogue = Catalogue::parse(
        &format!("{{ \"schema\": 1, \"profiles\": [{row}] }}"),
        &|name: &str| (name == "calibrated.wma").then_some(wma.as_slice()),
    )
    .unwrap();
    let mut all = list;
    for k in 0..4u64 {
        let mut raster = picture(Kind::Fractal, W, H, 300 + k, Layout::Rgb8);
        stamp(&mut raster, BlendModel::Encoded);
        all.push(Capture {
            name: format!("content-{k}"),
            raster,
            background: Background::Content,
            clean: None,
        });
    }
    for row in replay(
        &catalogue,
        &c,
        &all,
        &CalibrateOptions::default(),
        Fidelity::Lossless,
    ) {
        // A near-white logo on white leaves a contour of two levels under
        // noise of one: refusing to prove it is right, and nothing is
        // written. Everywhere else the mark is proved.
        if row.background != Background::White {
            assert!(row.verified, "{row:?}");
        }
        if let (true, Some(residual)) = (row.verified, row.residual) {
            // The capture's own noise (one level) and the restoration's.
            assert!(residual <= 3.0, "{row:?}");
        }
    }
}
