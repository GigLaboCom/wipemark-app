//! The blend past one logo colour (E12-R9), built and not opened: a bias
//! (R9a, D308 proposed), a logo colour per pixel (R9b, D313 proposed) and
//! linear light (R9c, D311 proposed) — each composited by
//! `wipemark_pixels::synth`, read by a catalogue only a `blend-preview`
//! build has, and restored through the product's own `clean`. Without the
//! feature there is nothing here to run: the catalogue refuses all three
//! (`catalogue.rs`, `the_catalogue_still_refuses_what_was_not_built`).
#![cfg(feature = "blend-preview")]

mod support;

use support::*;
use wipemark_pixels::synth::{composite_with, jpeg_planes, Blend, BlendModel, LogoColor};
use wipemark_pixels::{
    clean, clean_with, drawn, AlphaMap, Catalogue, ExamineOptions, Fidelity, Kernel, Layout,
    LogoMap, PixelRect, Planar, Raster, Sampling, SubRect, NOISE_FLOOR,
};

const W: u32 = 320;
const H: u32 = 240;

/// The synthetic profiles' own blend, as `support::profile_json` writes it.
const TODAY: &str = r#""blend": { "model": "encoded", "logo": [255, 255, 255], "logo_map": null }"#;

/// The synthetic V1 profile with both its maps at the small map's size —
/// so a logo colour map can be the size of every one (the catalogue's
/// rule) — its blend `blend` (the JSON object), and `extra` assets beside
/// its maps.
fn catalogue_with(blend: &str, extra: &[(String, Vec<u8>)]) -> Catalogue {
    let v1 = synthetic_v1();
    let one = Synthetic {
        id: v1.id,
        small: v1.small.clone(),
        large: v1.small,
    };
    let mut assets = Vec::new();
    let row = profile_json(&one, &mut assets, 32);
    assert!(row.contains(TODAY), "the synthetic row moved: {row}");
    let row = row.replace(TODAY, &format!(r#""blend": {blend}"#));
    assets.extend_from_slice(extra);
    parse_with(&[row], &assets).unwrap_or_else(|e| panic!("{e}"))
}

/// Where the small row puts the mark, as a sub-pixel rectangle.
fn row_rect() -> (PixelRect, SubRect) {
    let at = small_row(W, H, 48);
    let rect = SubRect {
        x: at.x as f32,
        y: at.y as f32,
        size: 48.0,
    };
    (at, rect)
}

/// A flat picture of one colour.
fn flat(colour: [u16; 3]) -> Raster {
    let samples = (0..W * H).flat_map(|_| colour).collect();
    Raster::new(W, H, Layout::Rgb8, samples).unwrap()
}

/// A smooth mid-tone ramp, red across and green down: where the sRGB
/// curve's slope is moderate, so linear light's inverse does not amplify
/// the rounding past a level on average.
fn ramp() -> Raster {
    let mut samples = Vec::with_capacity((W * H * 3) as usize);
    for y in 0..H {
        for x in 0..W {
            samples.push((100 + 60 * x / W) as u16);
            samples.push((110 + 40 * y / H) as u16);
            samples.push(130);
        }
    }
    Raster::new(W, H, Layout::Rgb8, samples).unwrap()
}

/// The mean absolute difference over `at`'s colour samples, in levels.
fn mean_error(a: &Raster, b: &Raster, at: PixelRect) -> f64 {
    let (mut sum, mut n) = (0f64, 0f64);
    for y in at.y..at.y + at.height {
        for x in at.x..at.x + at.width {
            let i = ((y * W + x) * 3) as usize;
            for c in 0..3 {
                sum += f64::from(a.samples()[i + c].abs_diff(b.samples()[i + c]));
                n += 1.0;
            }
        }
    }
    sum / n
}

/// R9a: a mark drawn with a bias of +1.5 levels is restored to within a
/// level by the profile that declares the bias; the same profile over a
/// mark drawn without one takes 1.5/(1 − α) levels off where there were
/// none — two levels or more under the mark's body. Mid-tones only: a
/// bias pushed past 255 is clamped by the file, and no inverse gets that
/// back.
#[test]
fn a_bias_composited_is_a_bias_restored() {
    let biased = catalogue_with(
        r#"{ "model": "encoded", "logo": [255, 255, 255], "logo_map": null, "bias": [1.5, 1.5, 1.5] }"#,
        &[],
    );
    assert_eq!(biased.profiles()[0].bias, Some([1.5; 3]));
    let mark = drawn(&synthetic_v1().small);
    let (_, rect) = row_rect();
    let with_bias = Blend {
        bias: [1.5; 3],
        ..Blend::encoded([255.0; 3])
    };
    let without = Blend::encoded([255.0; 3]);
    for (name, original) in [("ramp", ramp()), ("flat", flat([60, 120, 200]))] {
        let mut marked = original.clone();
        composite_with(&mut marked, &mark, rect, Kernel::Area, &with_bias);
        let report = clean(&mut marked, &biased, &ExamineOptions::default());
        assert_eq!(report.restored.len(), 1, "{name}: {:#?}", report.found);
        let off = max_error(&marked, &original);
        assert!(off <= 1, "{name}: {off}");

        let mut plain = original.clone();
        composite_with(&mut plain, &mark, rect, Kernel::Area, &without);
        let report = clean(&mut plain, &biased, &ExamineOptions::default());
        assert_eq!(report.restored.len(), 1, "{name}: {:#?}", report.found);
        let off = max_error(&plain, &original);
        assert!(
            off >= 2,
            "{name}: a bias nobody drew was not taken off ({off})"
        );
    }
}

/// A2: a profile with no bias — the field absent, or `null` — restores
/// every byte and says every word it did before R9: the bytes are GWT's
/// equation computed here by hand, `round((I − α·L)/(1 − α))` over the
/// row's template, and a bias of zero lands on the same bytes and words.
#[test]
fn a_profile_without_a_bias_is_byte_for_byte_todays() {
    let absent = catalogue_with(
        r#"{ "model": "encoded", "logo": [255, 255, 255], "logo_map": null }"#,
        &[],
    );
    let null = catalogue_with(
        r#"{ "model": "encoded", "logo": [255, 255, 255], "logo_map": null, "bias": null }"#,
        &[],
    );
    let zero = catalogue_with(
        r#"{ "model": "encoded", "logo": [255, 255, 255], "logo_map": null, "bias": [0, 0, 0] }"#,
        &[],
    );
    assert_eq!(absent.profiles()[0].bias, None);
    assert_eq!(null.profiles()[0].bias, None);
    let mark = drawn(&synthetic_v1().small);
    let (at, rect) = row_rect();
    let mut seen = 0;
    for (name, original) in backgrounds(W, H, Layout::Rgb8) {
        let mut marked = original.clone();
        composite_with(
            &mut marked,
            &mark,
            rect,
            Kernel::Area,
            &Blend::encoded([255.0; 3]),
        );
        let runs: Vec<(Raster, _)> = [&absent, &null, &zero]
            .into_iter()
            .map(|catalogue| {
                let mut r = marked.clone();
                let report = clean(&mut r, catalogue, &ExamineOptions::default());
                (r, report)
            })
            .collect();
        assert_eq!(runs[0], runs[1], "{name}: null is not absent");
        // A zero bias is a bias the proof carries, so its findings are not
        // the same values; what it wrote and said it restored are.
        assert_eq!(runs[0].0, runs[2].0, "{name}: a zero bias moved something");
        assert_eq!(runs[0].1.restored, runs[2].1.restored, "{name}");
        let (restored, report) = &runs[0];
        if report.restored.len() != 1 || report.restored[0].rect != at {
            continue;
        }
        // Today's restoration, by hand: every template pixel from the
        // noise floor to under the opaque threshold, each channel
        // unblended over the white logo, rounded half away from zero and
        // clamped; everything else as stored.
        let mut expected = marked.samples().to_vec();
        for y in 0..at.height {
            for x in 0..at.width {
                let a = mark.get(i64::from(x), i64::from(y));
                if !(NOISE_FLOOR..0.95).contains(&a) {
                    continue;
                }
                let a = f64::from(a);
                let i = (((at.y + y) * W + at.x + x) * 3) as usize;
                for v in &mut expected[i..i + 3] {
                    *v = ((f64::from(*v) - a * 255.0) / (1.0 - a))
                        .round()
                        .clamp(0.0, 255.0) as u16;
                }
            }
        }
        assert_eq!(restored.samples(), &expected[..], "{name}");
        seen += 1;
    }
    assert!(seen >= 6, "only {seen} restorations at the row");
}

/// R9b: a logo whose colour departs along the mark's shape — bluer and
/// greener where the mark is denser — drawn over mid-grey and over a
/// saturated red. The profile that carries the colour map restores it to
/// within a level; the same profile with one colour leaves two levels or
/// more where the map departs, restored or not.
#[test]
fn a_logo_map_restores_what_a_global_logo_cannot() {
    let mark = drawn(&synthetic_v1().small);
    let colours: Vec<[f32; 3]> = mark
        .values()
        .iter()
        .map(|&a| {
            [
                250.0,
                (250.0 - 120.0 * a).round(),
                (250.0 - 200.0 * a).round(),
            ]
        })
        .collect();
    let wml = LogoMap::new(48, 48, colours).unwrap().write().unwrap();
    // Composited with the colours the profile reads back: the same values.
    let logos = LogoMap::read(&wml).unwrap();
    let sha = sha256_hex(&wml);
    let mapped = catalogue_with(
        &format!(
            r#"{{ "model": "encoded", "logo": [250, 250, 250], "logo_map": {{ "asset": "logo.wml", "sha256": "{sha}", "size": [48, 48] }} }}"#
        ),
        &[(String::from("logo.wml"), wml)],
    );
    assert!(mapped.profiles()[0].logo_map.is_some());
    let global = catalogue_with(
        r#"{ "model": "encoded", "logo": [250, 250, 250], "logo_map": null }"#,
        &[],
    );
    let (at, rect) = row_rect();
    let blend = Blend {
        logo: LogoColor::PerPixel(logos.colours()),
        ..Blend::encoded([250.0; 3])
    };
    for colour in [[128, 128, 128], [230, 12, 20]] {
        let original = flat(colour);
        let mut marked = original.clone();
        composite_with(&mut marked, &mark, rect, Kernel::Area, &blend);

        let mut with_map = marked.clone();
        let report = clean(&mut with_map, &mapped, &ExamineOptions::default());
        assert_eq!(report.restored.len(), 1, "{colour:?}: {:#?}", report.found);
        assert!(
            max_error(&with_map, &original) <= 1,
            "{colour:?}: {}",
            max_error(&with_map, &original)
        );

        let mut with_one = marked.clone();
        clean(&mut with_one, &global, &ExamineOptions::default());
        assert!(
            max_error(&with_one, &original) >= 2,
            "{colour:?}: one colour restored a mark of many ({})",
            mean_error(&with_one, &original, at)
        );
    }
}

/// R9c, A5 in miniature: a mark blended in light and one blended in code
/// values, each restored by an `encoded` profile and a `linear-light` one.
/// Each inverse wins on the composite of its own model — a mean error
/// under the mark of half a level or less — and loses on the other's:
/// four levels or more somewhere under it, restored or not.
#[test]
fn the_linear_inverse_restores_a_linear_composite() {
    let encoded = catalogue_with(TODAY.trim_start_matches(r#""blend": "#), &[]);
    let linear = catalogue_with(
        r#"{ "model": "linear-light", "logo": [255, 255, 255], "logo_map": null }"#,
        &[],
    );
    assert_eq!(linear.profiles()[0].model, BlendModel::LinearLight);
    let mark = drawn(&synthetic_v1().small);
    let (at, rect) = row_rect();
    let original = ramp();
    for model in [BlendModel::Encoded, BlendModel::LinearLight] {
        let mut marked = original.clone();
        let blend = Blend {
            model,
            ..Blend::encoded([255.0; 3])
        };
        composite_with(&mut marked, &mark, rect, Kernel::Area, &blend);
        for (inverse, catalogue) in [
            (BlendModel::Encoded, &encoded),
            (BlendModel::LinearLight, &linear),
        ] {
            let mut restored = marked.clone();
            let report = clean(&mut restored, catalogue, &ExamineOptions::default());
            let name = format!("{inverse:?} inverse over a {model:?} composite");
            if inverse == model {
                assert_eq!(report.restored.len(), 1, "{name}: {:#?}", report.found);
                let mean = mean_error(&restored, &original, at);
                assert!(mean <= 0.5, "{name}: {mean}");
            } else {
                assert!(
                    max_error(&restored, &original) >= 4,
                    "{name}: {}",
                    max_error(&restored, &original)
                );
            }
        }
    }
}

/// The mean absolute difference over the colour samples of the pixels
/// where `mark` (drawn at `at`) is at 0.1 or more, in levels.
fn mean_error_under(a: &Raster, b: &Raster, at: PixelRect, mark: &AlphaMap) -> f64 {
    let (mut sum, mut n) = (0f64, 0f64);
    for y in 0..at.height {
        for x in 0..at.width {
            if mark.get(i64::from(x), i64::from(y)) < 0.1 {
                continue;
            }
            let i = (((at.y + y) * W + at.x + x) * 3) as usize;
            for c in 0..3 {
                sum += f64::from(a.samples()[i + c].abs_diff(b.samples()[i + c]));
                n += 1.0;
            }
        }
    }
    sum / n
}

/// R9a in the planes (R6): a mark drawn with a bias of (+6, +2, −4) levels
/// — a colour, so it has a chroma part — over a flat colour, stored 4:2:0
/// at quality 100, cleaned with its planes. The profile that declares the
/// bias restores it in the planes — the bias's linear part taken off Y, Cb
/// and Cr — within two levels of the truth under the mark, and two levels
/// nearer than the same profile without it, which leaves b/(1 − α) wherever
/// it restored (or the whole mark, where it did not).
#[test]
fn a_bias_is_taken_off_in_the_planes_too() {
    let biased = catalogue_with(
        r#"{ "model": "encoded", "logo": [255, 255, 255], "logo_map": null, "bias": [6, 2, -4] }"#,
        &[],
    );
    let plain = catalogue_with(TODAY.trim_start_matches(r#""blend": "#), &[]);
    let mark = drawn(&synthetic_v1().small);
    let (at, rect) = row_rect();
    let original = flat([60, 120, 200]);
    let mut marked = original.clone();
    let blend = Blend {
        bias: [6.0, 2.0, -4.0],
        ..Blend::encoded([255.0; 3])
    };
    composite_with(&mut marked, &mark, rect, Kernel::Area, &blend);
    let (planes, raster) = jpeg_planes(&marked, Sampling::H420, 100).unwrap();
    let lossy = ExamineOptions {
        source: Fidelity::Lossy,
        profiles: None,
    };

    let mut with = raster.clone();
    let report = clean_with(&mut with, Some(&planes), &biased, &lossy);
    assert_eq!(report.restored.len(), 1, "{:#?}", report.found);
    assert!(
        matches!(report.restored[0].planar, Some(Planar::Inverse { .. })),
        "{:?}",
        report.restored[0].planar
    );
    let mut without = raster;
    clean_with(&mut without, Some(&planes), &plain, &lossy);
    let near = mean_error_under(&with, &original, at, &mark);
    let far = mean_error_under(&without, &original, at, &mark);
    // Within two levels of the truth, and two nearer than without the bias.
    // A bias left in Y or in the chroma blocks reads 4.8 or 6.0 here
    // (central check, 2026-10-09: with a grey bias, which has no chroma,
    // and the relative bound alone, a bias left in Y read 12.8 against the
    // plain profile's 68 and passed).
    assert!(near < 2.0, "with the bias {near}");
    assert!(near + 2.0 < far, "with the bias {near}, without it {far}");
}
