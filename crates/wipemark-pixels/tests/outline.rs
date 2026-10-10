//! D238: a mark shrunk with its picture and saved with loss — what the
//! host verifier's real files were — is found, fitted to the eighth of a
//! pixel, matched to the filter that shrank it, restored, and leaves no
//! outline past the bound — or, where its faint band is off the picture
//! by more than a level on a background with nothing to hide it in, says
//! so (D244).
//!
//! The case: a canonical Gemini V2 output 2816 pixels wide carries the
//! 96-pixel mark 192 pixels in from its corner; it is handed out at
//! 1024-class, about 0.364 of that, and saved as JPEG. Here the corner
//! (704 × 384 of the large picture) is composited with the shipped map,
//! shrunk to 256 × 140 by Lanczos or bilinear — the mark lands at about 35
//! pixels, between the sizes of the verifier's files (26 and 33) — saved
//! at quality 90 or 95, decoded, and cleaned as a lossy source.

mod support;

use image::imageops::FilterType;
use support::*;
use wipemark_pixels::{
    clean, composite, drawn, measure_at, resampled, Anchor, Catalogue, ExamineOptions, Fidelity,
    Kernel, Layout, PixelRect, Raster, SubRect, BAND, NOISE_FLOOR, OUTLINE_BOUND, STEP_LEVELS,
    TEXTURE_LEVELS, TEXTURE_RATIO,
};

const LARGE: (u32, u32) = (704, 384);
const SMALL: (u32, u32) = (256, 140);

fn to_image(r: &Raster) -> image::RgbImage {
    let bytes = r.samples().iter().map(|&s| s as u8).collect();
    image::RgbImage::from_raw(r.width(), r.height(), bytes).unwrap()
}

fn from_image(i: &image::RgbImage) -> Raster {
    Raster::from_u8(i.width(), i.height(), Layout::Rgb8, i.as_raw()).unwrap()
}

fn through_jpeg(i: &image::RgbImage, quality: u8) -> image::RgbImage {
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality)
        .encode(
            i.as_raw(),
            i.width(),
            i.height(),
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    image::load_from_memory_with_format(&out, image::ImageFormat::Jpeg)
        .unwrap()
        .to_rgb8()
}

#[derive(Clone, Copy, Debug)]
enum Scene {
    Gradient,
    Flat,
    Sky,
}

fn large(scene: Scene) -> Raster {
    match scene {
        Scene::Gradient => picture(Kind::Gradient, LARGE.0, LARGE.1, 77, Layout::Rgb8),
        Scene::Flat => picture(Kind::Flat, LARGE.0, LARGE.1, 77, Layout::Rgb8),
        Scene::Sky => aurora(LARGE.0, LARGE.1, 3, Layout::Rgb8),
    }
}

/// Every case that is proved; the ones the second proof refuses at these
/// sizes (a bilinear gradient at 90, a Lanczos sky at 90) are left, said,
/// and not here.
const CASES: [(Scene, FilterType, u8); 10] = [
    (Scene::Gradient, FilterType::Lanczos3, 95),
    (Scene::Gradient, FilterType::Lanczos3, 90),
    (Scene::Gradient, FilterType::Triangle, 95),
    (Scene::Flat, FilterType::Lanczos3, 95),
    (Scene::Flat, FilterType::Lanczos3, 90),
    (Scene::Flat, FilterType::Triangle, 95),
    (Scene::Flat, FilterType::Triangle, 90),
    (Scene::Sky, FilterType::Lanczos3, 95),
    (Scene::Sky, FilterType::Triangle, 95),
    (Scene::Sky, FilterType::Triangle, 90),
];

#[test]
fn a_shrunk_and_compressed_mark_is_restored_within_the_outline_bound() {
    let shipped = Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"));
    let v2 = shipped.profile("gemini-sparkle-v2").unwrap();
    let (_, mark) = v2.maps.iter().find(|(id, _)| id == "gemini-v2-96").unwrap();
    let lossy = ExamineOptions {
        source: Fidelity::Lossy,
        profiles: None,
    };
    let mut said = 0;
    for (scene, filter, quality) in CASES {
        let name = format!("{scene:?} {filter:?} q{quality}");
        let original = large(scene);
        let mut marked = original.clone();
        let at = PixelRect {
            x: LARGE.0 - 192 - 96,
            y: LARGE.1 - 192 - 96,
            width: 96,
            height: 96,
        };
        composite(&mut marked, mark, at, [255.0; 3]);
        let truth = from_image(&image::imageops::resize(
            &to_image(&original),
            SMALL.0,
            SMALL.1,
            filter,
        ));
        let shrunk = image::imageops::resize(&to_image(&marked), SMALL.0, SMALL.1, filter);
        let mut raster = from_image(&through_jpeg(&shrunk, quality));
        let report = clean(&mut raster, shipped, &lossy);
        assert_eq!(report.restored.len(), 1, "{name}: {:#?}", report.found);
        let r = &report.restored[0];
        assert!(r.outline <= OUTLINE_BOUND, "{name}: outline {}", r.outline);
        assert_eq!(
            report.marks_left(),
            r.outline_left || r.texture_left,
            "{name}"
        );
        // Inside the mark's rectangle, as close to the picture shrunk
        // without the mark as the codec's own error lets it be.
        let (mut sum, mut n) = (0f64, 0f64);
        let c = 3;
        for y in r.rect.y..r.rect.y + r.rect.height {
            for x in r.rect.x..r.rect.x + r.rect.width {
                let i = ((y * SMALL.0 + x) as usize) * c;
                for k in 0..3 {
                    sum += f64::from(raster.samples()[i + k].abs_diff(truth.samples()[i + k]));
                    n += 1.0;
                }
            }
        }
        let mean = sum / n;
        let f = report
            .found
            .iter()
            .find(|f| f.verified().is_some())
            .unwrap();
        println!(
            "{name}: {:?} at {:?} by {:?}, outline {:.3}, mean error in the rectangle {mean:.2}",
            f.profile, f.rect, f.kernel, r.outline
        );
        assert!(mean <= 3.0, "{name}: mean error {mean:.2} in the rectangle");

        // The faint band against the picture shrunk without the mark: an
        // outline said is one that is there, and one not said is within a
        // level and a half of the truth (D244). Flat by bilinear at 90 is
        // the one said: the band +2.25 over the truth on a background with
        // nothing to hide it in; the textured ones reach +1.26 and are not.
        let band = resampled(mark, f.rect.size, f.rect.x.fract(), f.rect.y.fract()).unwrap();
        let luma = |s: &[u16], i: usize| {
            0.2126 * f64::from(s[i]) + 0.7152 * f64::from(s[i + 1]) + 0.0722 * f64::from(s[i + 2])
        };
        let (mut off, mut nb) = (0f64, 0f64);
        for y in 0..band.height() {
            for x in 0..band.width() {
                let a = band.get(i64::from(x), i64::from(y));
                if !(BAND[0]..=BAND[1]).contains(&a) {
                    continue;
                }
                let (px, py) = (f.rect.x as u32 + x, f.rect.y as u32 + y);
                let i = ((py * SMALL.0 + px) as usize) * c;
                off += luma(raster.samples(), i) - luma(truth.samples(), i);
                nb += 1.0;
            }
        }
        let off = (off / nb) as f32;
        // The roughness under the mark against the truth's (D250): each
        // pixel's distance in (Y, Cb, Cr) from its eight neighbours' mean,
        // the 95th percentile over the pixels the restoration changed.
        let rough = |s: &[u16]| {
            let ycc = |x: u32, y: u32| {
                let i = ((y * SMALL.0 + x) as usize) * c;
                let [r, g, b] = [0, 1, 2].map(|k| f64::from(s[i + k]));
                [
                    0.299 * r + 0.587 * g + 0.114 * b,
                    -0.168_736 * r - 0.331_264 * g + 0.5 * b,
                    0.5 * r - 0.418_688 * g - 0.081_312 * b,
                ]
            };
            let mut all = Vec::new();
            for y in 0..band.height() {
                for x in 0..band.width() {
                    let a = band.get(i64::from(x), i64::from(y));
                    if !(NOISE_FLOOR..0.95).contains(&a) {
                        continue;
                    }
                    let (px, py) = (f.rect.x as u32 + x, f.rect.y as u32 + y);
                    let mut mean = [0f64; 3];
                    for (dx, dy) in (0..9).map(|k| (k % 3, k / 3)).filter(|&k| k != (1, 1)) {
                        for (m, v) in mean.iter_mut().zip(ycc(px + dx - 1, py + dy - 1)) {
                            *m += v / 8.0;
                        }
                    }
                    let p = ycc(px, py);
                    all.push((0..3).map(|k| (p[k] - mean[k]).powi(2)).sum::<f64>().sqrt());
                }
            }
            all.sort_by(f64::total_cmp);
            all[((all.len() - 1) as f64 * 0.95).round() as usize] as f32
        };
        let (restored, truthful) = (rough(raster.samples()), rough(truth.samples()));
        println!(
            "{name}: roughness {restored:.2} under the mark, the truth's {truthful:.2}; said {}",
            r.texture_left
        );
        // A texture said is one the truth has not got; one not said is
        // under the bound or the picture's own — the sky's grain is.
        if r.texture_left {
            assert!(restored > 2.0 * truthful.max(1.0), "{name}: said");
        } else {
            assert!(
                restored <= TEXTURE_LEVELS || restored <= TEXTURE_RATIO * truthful,
                "{name}: not said, {restored:.2} against the truth's {truthful:.2}"
            );
        }
        println!(
            "{name}: the band {off:+.2} over the truth, step {:+.2}",
            r.step
        );
        if r.outline_left {
            assert!(
                off.abs() > STEP_LEVELS,
                "{name}: said, {off:+.2} over the truth"
            );
            said += 1;
        } else {
            assert!(
                off.abs() <= 1.5,
                "{name}: not said, {off:+.2} over the truth"
            );
        }
    }
    assert_eq!(
        said, 1,
        "the flat bilinear case at 90 is the one outline left"
    );
}

/// E12-R12 §5: `measure_at` restates nothing. A clean picture `P` is the
/// restoration of a composite (a mark at a row's own place over a
/// procedural picture, undone); composited again where it was, the very
/// bytes come back, and restored again, `P` comes back to the byte — a
/// **null restoration**, the one a picture with no mark can be given (a
/// profile at zero opacity has no support, `verify` calls it no blend, and
/// only `verify` builds a `Verified`). What `outline` measured after it,
/// `measure_at` measures over `P` with nothing restored: the share against
/// the contour a mark drawn there would have had, the band's steps, the
/// roughness under the map's support and around it — equal to the bit, and
/// the verdicts with them, on either fidelity.
#[test]
fn measure_at_on_a_clean_picture_is_what_outline_measures_after_a_null_restoration() {
    let shipped = Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"));
    let (w, h) = (1100u32, 1100u32);
    let mut compared = 0;
    for id in ["gemini-sparkle-v1", "gemini-sparkle-v2"] {
        let profile = shipped.profile(id).unwrap();
        // The large row: V1's fitted map 64 in, V2's canonical one 192 in.
        let row = &profile.placements[0];
        let map = profile.map(row.alpha);
        let Anchor::Corner { margin, .. } = row.anchor else {
            panic!("{id}: the large row is a corner row")
        };
        let at = PixelRect {
            x: w - margin[0] - map.width(),
            y: h - margin[1] - map.height(),
            width: map.width(),
            height: map.height(),
        };
        let mark = drawn(map);
        for (kind, seed) in [
            (Kind::Gradient, 3),
            (Kind::ValueNoise, 5),
            (Kind::Fractal, 7),
            (Kind::Flat, 11),
        ] {
            for source in [Fidelity::Lossless, Fidelity::Lossy] {
                let name = format!("{id} {kind:?} {source:?}");
                let options = ExamineOptions {
                    source,
                    profiles: Some(vec![id.to_string()]),
                };
                let mut marked = picture(kind, w, h, seed, Layout::Rgb8);
                composite(&mut marked, &mark, at, profile.logo);
                let first = marked.clone();
                let mut clean_picture = marked;
                let report = clean(&mut clean_picture, shipped, &options);
                assert_eq!(report.restored.len(), 1, "{name}: {:#?}", report.found);

                // The null restoration: the clean picture's composite is the
                // composite it was restored from, and its restoration is the
                // clean picture to the byte.
                let mut again = clean_picture.clone();
                composite(&mut again, &mark, at, profile.logo);
                assert!(again == first, "{name}: the composite comes back");
                let report = clean(&mut again, shipped, &options);
                assert!(again == clean_picture, "{name}: a null restoration");
                assert_eq!(report.restored.len(), 1, "{name}: {:#?}", report.found);
                let r = &report.restored[0];
                let f = report
                    .found
                    .iter()
                    .find(|f| f.verified().is_some())
                    .unwrap();

                let m = measure_at(&clean_picture, profile, map, f.rect, f.kernel)
                    .unwrap_or_else(|| panic!("{name}: measured nothing"));
                println!(
                    "{name}: share {:.4} step {:+.3} chroma {:.3} texture {:.3} around {:.3}",
                    m.share, m.step, m.chroma, m.texture, m.texture_around
                );
                assert_eq!(m.share, r.outline, "{name}: share");
                assert_eq!(m.steps, r.steps, "{name}: steps");
                assert_eq!(m.step, r.step, "{name}: step");
                assert_eq!(m.chroma, r.chroma, "{name}: chroma");
                assert_eq!(m.texture, r.texture, "{name}: texture");
                assert_eq!(m.texture_around, r.texture_around, "{name}: around");
                assert_eq!(m.left(), r.outline_left, "{name}: left");
                assert_eq!(
                    source == Fidelity::Lossy && m.textured(),
                    r.texture_left,
                    "{name}: texture left"
                );
                compared += 1;
            }
        }
    }
    assert_eq!(compared, 16);
}

/// `measure_at` against figures it was handed, not against the code it
/// shares: on a flat grey picture with every third pixel each way lifted
/// by six grey levels, the roughness under the map's support and around it
/// is six — in `(Y, Cb, Cr)`, where a grey lift is all luma; in RGB it
/// would be √3 times that — and nothing is left. With the faint band alone
/// lifted by three grey levels instead, the luma step is three, the colour
/// step none, and an outline is said: the band is the template's pixels in
/// [`BAND`], where the map's own drawn values put them. A flat picture has
/// no contour of its own, so its share is none.
#[test]
fn measure_at_reads_a_grain_and_a_step_it_was_given() {
    let shipped = Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"));
    let v2 = shipped.profile("gemini-sparkle-v2").unwrap();
    let (_, map) = v2.maps.iter().find(|(id, _)| id == "gemini-v2-96").unwrap();
    let template = drawn(map);
    let (w, h, x0, y0) = (240u32, 200u32, 70u32, 50u32);
    let rect = SubRect {
        x: x0 as f32,
        y: y0 as f32,
        size: map.width() as f32,
    };
    let flat = |lift: &dyn Fn(u32, u32) -> u8| {
        let mut bytes = Vec::with_capacity((w * h * 3) as usize);
        for y in 0..h {
            for x in 0..w {
                bytes.extend([128 + lift(x, y); 3]);
            }
        }
        Raster::from_u8(w, h, Layout::Rgb8, &bytes).unwrap()
    };

    let plain = flat(&|_, _| 0);
    let m = measure_at(&plain, v2, map, rect, Kernel::Area).unwrap();
    assert_eq!(
        (m.share, m.step, m.chroma, m.texture),
        (0.0, 0.0, 0.0, 0.0),
        "{m:?}"
    );

    let grain = flat(&|x, y| if x % 3 == 0 && y % 3 == 0 { 6 } else { 0 });
    let m = measure_at(&grain, v2, map, rect, Kernel::Area).unwrap();
    assert!((m.texture - 6.0).abs() < 1e-3, "{m:?}");
    assert!((m.texture_around - 6.0).abs() < 1e-3, "{m:?}");
    assert!(m.chroma < 1e-3 && m.step.abs() < 1.0, "{m:?}");
    assert!(!m.textured() && !m.left(), "{m:?}");

    let in_band = |x: u32, y: u32| {
        let (tx, ty) = (i64::from(x) - i64::from(x0), i64::from(y) - i64::from(y0));
        (BAND[0]..=BAND[1]).contains(&template.get(tx, ty))
    };
    let band = flat(&|x, y| if in_band(x, y) { 3 } else { 0 });
    let m = measure_at(&band, v2, map, rect, Kernel::Area).unwrap();
    assert!((m.step - 3.0).abs() < 1e-3, "{m:?}");
    assert!(m.chroma < 1e-3 && m.spread == 0.0, "{m:?}");
    assert!(m.left(), "{m:?}");

    // A rectangle past the picture's edge is not measured.
    let outside = SubRect {
        x: (w - 10) as f32,
        ..rect
    };
    assert!(measure_at(&grain, v2, map, outside, Kernel::Area).is_none());
}
