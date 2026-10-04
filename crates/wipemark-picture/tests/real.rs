//! Real Gemini outputs, not composites: the owner's own stickers
//! (`fixtures/image/gemini/`, 2026-04-24), each the bottom-right 1025 ×
//! 1025 of a 2048 × 2048 picture — so the large V1 row (margin 64, the
//! 96-pixel map) lands on the very pixels the vendor stamped. What the
//! synthetic suites assume about a vendor's mark is held here to the mark
//! itself.

use std::path::PathBuf;

use image::imageops::FilterType;
use wipemark_picture::{clean, decode, inspect, Encoding, PictureOptions, Visible};
use wipemark_pixels::{
    Catalogue, ExamineOptions, Fidelity, Layout, PixelRect, Placed, Raster, Refusal, Verdict,
    OUTLINE_BOUND,
};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/image/gemini")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn shipped() -> PictureOptions<'static> {
    PictureOptions {
        scope: wipemark_image::Scope::AiProvenance,
        catalogue: Some(Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"))),
    }
}

/// Where the large V1 row puts the mark in a 1025 × 1025 picture.
const ROW: PixelRect = PixelRect {
    x: 865,
    y: 865,
    width: 96,
    height: 96,
};

const MARKED: [&str; 3] = ["crying-1025.png", "torch-1025.png", "victory-1025.png"];

fn raster_of(bytes: &[u8]) -> Raster {
    let container = wipemark_image::inspect(bytes).unwrap().container;
    decode(bytes, container).unwrap().unwrap().raster
}

fn jpeg_of(raster: &Raster, quality: u8) -> Vec<u8> {
    let rgb: Vec<u8> = raster
        .samples()
        .chunks_exact(raster.layout().channels())
        .flat_map(|p| [p[0] as u8, p[1] as u8, p[2] as u8])
        .collect();
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality)
        .encode(
            &rgb,
            raster.width(),
            raster.height(),
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    out
}

/// The vendor's own mark, at its row: proved there, restored with no
/// outline, and nothing proves on the result. Not *exact*: GWT's maps are
/// 8-bit captures of the vendor's α, and over a real output the inverse
/// leaves the range by more than half a level on a handful of samples,
/// which are clamped and counted — under one in a hundred here.
#[test]
fn a_real_mark_is_proved_at_its_row_and_restored() {
    for name in MARKED {
        let bytes = fixture(name);
        let seen = inspect(&bytes, &shipped()).unwrap();
        let Visible::Examined { report, .. } = &seen.visible else {
            panic!("{name}: {:?}", seen.visible)
        };
        assert_eq!(report.found.len(), 1, "{name}: {:#?}", report.found);
        let f = &report.found[0];
        assert_eq!(f.profile, "gemini-sparkle-v1", "{name}");
        assert_eq!(f.placed, Placed::Row(0), "{name}");
        assert_eq!(f.pixels, Some(ROW), "{name}");
        assert!(f.verified().is_some(), "{name}: {:?}", f.verdict);

        let (out, cleaned) = clean(&bytes, &shipped()).unwrap();
        assert!(
            matches!(cleaned.encoding, Encoding::Png { .. }),
            "{name}: {:?}",
            cleaned.encoding
        );
        let Visible::Examined { report, .. } = &cleaned.visible else {
            panic!("{name}")
        };
        assert_eq!(report.restored.len(), 1, "{name}");
        let r = &report.restored[0];
        assert!(!r.outline_left, "{name}: {r:?}");
        assert!(r.clamped * 100 < r.changed * 3, "{name}: {r:?}");
        assert!(r.outline <= OUTLINE_BOUND, "{name}: {r:?}");
        assert!(!cleaned.marks_left(), "{name}");
        let again = inspect(&out, &shipped()).unwrap();
        let Visible::Examined { report, .. } = &again.visible else {
            panic!("{name}")
        };
        assert!(report.found.is_empty(), "{name}: {:#?}", report.found);
    }
}

/// Around the sparkle the vendor drew nothing: GWT's map carries 1–6/255
/// of capture noise over its whole square, and subtracting it left a
/// square a level darker than the picture around it — visible on a flat
/// background. Taken out (D241), the square is the picture's own: on
/// average within a tenth of a level of what it was, where the map is
/// only noise.
#[test]
fn the_square_around_a_real_mark_is_left_as_it_was() {
    let catalogue = Catalogue::shipped().unwrap();
    let map = &catalogue
        .profile("gemini-sparkle-v1")
        .unwrap()
        .maps
        .iter()
        .find(|(id, _)| id == "gemini-v1-96")
        .unwrap()
        .1;
    for name in MARKED {
        let bytes = fixture(name);
        let before = raster_of(&bytes);
        let (out, _) = clean(&bytes, &shipped()).unwrap();
        let after = raster_of(&out);
        let c = before.layout().channels();
        let (mut moved, mut n) = (0f64, 0f64);
        for y in 0..96u32 {
            for x in 0..96u32 {
                let a = map.get(i64::from(x), i64::from(y)) * 255.0;
                // The capture's noise, far from the sparkle.
                if !(0.5..6.5).contains(&a) {
                    continue;
                }
                let near = (-3i64..=3).any(|dy| {
                    (-3i64..=3)
                        .any(|dx| map.get(i64::from(x) + dx, i64::from(y) + dy) * 255.0 >= 7.0)
                });
                if near {
                    continue;
                }
                let i = (((ROW.y + y) * before.width() + ROW.x + x) as usize) * c;
                for k in 0..3 {
                    moved += f64::from(after.samples()[i + k]) - f64::from(before.samples()[i + k]);
                    n += 1.0;
                }
            }
        }
        assert!(n > 10_000.0, "{name}: {n}");
        let mean = moved / n;
        assert!(
            mean.abs() <= 0.1,
            "{name}: the square moved by {mean:.2} on average"
        );
    }
}

/// Inside the sparkle, too: GWT restores with a white logo, and on real
/// outputs the vendor's is not — measured over 22 of the owner's
/// pictures, (252.1, 253.5, 252.8), spread under 0.6 of a level (D242).
/// Restored with white, the sparkle came back as a darker ghost, 1–3
/// levels under the picture around it; with the measured colour, the
/// body is the picture's own to within a level per channel. And the soft
/// edge: GWT's 8-bit capture made it too strong (1–2.4 levels of outline
/// left); the large row's map is measured from 19 real outputs (D243),
/// and these two pictures are not among them. On the vendor's own files —
/// `crying` is a re-saved copy (no C2PA, a flattened background) whose
/// blue fits 254.8, the one outlier of the 22.
#[test]
fn the_sparkle_leaves_no_ghost() {
    let catalogue = Catalogue::shipped().unwrap();
    let map = &catalogue
        .profile("gemini-sparkle-v1")
        .unwrap()
        .maps
        .iter()
        .find(|(id, _)| id == "gemini-v1-96")
        .unwrap()
        .1;
    for name in ["torch-1025.png", "victory-1025.png"] {
        let (out, _) = clean(&fixture(name), &shipped()).unwrap();
        let after = raster_of(&out);
        let c = after.layout().channels();
        let at = |x: u32, y: u32, k: usize| {
            f64::from(after.samples()[((y * after.width() + x) as usize) * c + k])
        };
        let (mut ring, mut nr) = ([0f64; 3], 0f64);
        for y in ROW.y - 28..ROW.y + 122 {
            for x in ROW.x - 28..ROW.x + 122 {
                let inside =
                    (ROW.x - 8..ROW.x + 104).contains(&x) && (ROW.y - 8..ROW.y + 104).contains(&y);
                if inside {
                    continue;
                }
                for (k, r) in ring.iter_mut().enumerate() {
                    *r += at(x, y, k);
                }
                nr += 1.0;
            }
        }
        // The body, and the soft edge the 8-bit capture made too strong
        // (D243): each within a level of the picture around it.
        for (part, lo, hi) in [("body", 0.45f32, 1.0f32), ("edge", 0.03, 0.45)] {
            let (mut sum, mut n) = ([0f64; 3], 0f64);
            for y in 0..96u32 {
                for x in 0..96u32 {
                    let a = map.get(i64::from(x), i64::from(y));
                    if a < lo || a >= hi {
                        continue;
                    }
                    for (k, b) in sum.iter_mut().enumerate() {
                        *b += at(ROW.x + x, ROW.y + y, k);
                    }
                    n += 1.0;
                }
            }
            for k in 0..3 {
                let ghost = sum[k] / n - ring[k] / nr;
                assert!(
                    ghost.abs() <= 1.0,
                    "{name}: the {part}, channel {k}, off by {ghost:.2}"
                );
            }
        }
    }
}

/// The same pictures saved as JPEG at 90 and 95: proved, restored,
/// re-encoded, no outline past the bound.
#[test]
fn a_real_mark_saved_as_jpeg_is_restored_within_the_outline_bound() {
    for name in MARKED {
        let raster = raster_of(&fixture(name));
        for quality in [90u8, 95] {
            let label = format!("{name} q{quality}");
            let (_, cleaned) = clean(&jpeg_of(&raster, quality), &shipped()).unwrap();
            let Visible::Examined { report, .. } = &cleaned.visible else {
                panic!("{label}: {:?}", cleaned.visible)
            };
            assert_eq!(report.restored.len(), 1, "{label}: {:#?}", report.found);
            let r = &report.restored[0];
            println!("{label}: outline {:.3}", r.outline);
            assert!(r.outline <= OUTLINE_BOUND, "{label}: {r:?}");
            assert!(!cleaned.marks_left(), "{label}");
            assert_eq!(cleaned.encoding, Encoding::Jpeg { quality: 95 });
        }
    }
}

/// The vendor's mark shrunk with its picture — the 1025 corner taken to
/// 373 pixels (0.364, a 2816 output handed out at 1024-class), by Lanczos
/// or bilinear, saved as JPEG: the case the host verifier's real files
/// were. Wherever the mark is proved, it is restored within the outline
/// bound; and it is proved in most of them.
#[test]
fn a_real_mark_shrunk_with_its_picture_is_restored_within_the_outline_bound() {
    let lossy = ExamineOptions {
        source: Fidelity::Lossy,
        profiles: None,
    };
    let catalogue = Catalogue::shipped().unwrap();
    let (mut proved, mut total) = (0, 0);
    for name in MARKED {
        let raster = raster_of(&fixture(name));
        let rgb: Vec<u8> = raster
            .samples()
            .chunks_exact(raster.layout().channels())
            .flat_map(|p| [p[0] as u8, p[1] as u8, p[2] as u8])
            .collect();
        let img = image::RgbImage::from_raw(raster.width(), raster.height(), rgb).unwrap();
        for filter in [FilterType::Lanczos3, FilterType::Triangle] {
            let small = image::imageops::resize(&img, 373, 373, filter);
            for quality in [90u8, 95] {
                total += 1;
                let mut out = Vec::new();
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality)
                    .encode(small.as_raw(), 373, 373, image::ExtendedColorType::Rgb8)
                    .unwrap();
                let decoded = image::load_from_memory_with_format(&out, image::ImageFormat::Jpeg)
                    .unwrap()
                    .to_rgb8();
                let mut r = Raster::from_u8(373, 373, Layout::Rgb8, decoded.as_raw()).unwrap();
                let report = wipemark_pixels::clean(&mut r, catalogue, &lossy);
                let label = format!("{name} {filter:?} q{quality}");
                let f = report.found.iter().find(|f| f.verified().is_some());
                println!(
                    "{label}: {:?}",
                    report
                        .found
                        .iter()
                        .map(|f| (
                            f.profile.as_str(),
                            f.rect,
                            f.kernel,
                            f.scores,
                            f.verified().is_some()
                        ))
                        .collect::<Vec<_>>()
                );
                if let Some(f) = f {
                    proved += 1;
                    let restored = &report.restored[0];
                    println!("{label}: {:?} outline {:.3}", f.kernel, restored.outline);
                    assert!(restored.outline <= OUTLINE_BOUND, "{label}: {restored:?}");
                }
            }
        }
    }
    println!("proved {proved} of {total}");
    assert!(proved * 2 >= total, "proved {proved} of {total}");
}

/// The vendor's mark over a saturated green — the original at 0 in red
/// and blue under it. A blend with GWT's 8-bit map leaves those channels
/// up to 6 stored levels under what it could produce over 0 (the
/// vendor's α against its capture); the out-of-range measure counts that
/// gap in stored levels, with room for it (D240), so the mark is proved
/// and restored, the channels clamped back to 0 and counted — not exact —
/// and no outline left. Under the old one-level rule it was refused.
#[test]
fn a_real_mark_on_a_saturated_green_is_restored() {
    let bytes = fixture("anchor-green-1025.png");
    let (out, cleaned) = clean(&bytes, &shipped()).unwrap();
    let Visible::Examined { report, .. } = &cleaned.visible else {
        panic!("{:?}", cleaned.visible)
    };
    assert_eq!(report.restored.len(), 1, "{:#?}", report.found);
    let f = &report.found[0];
    assert_eq!(f.placed, Placed::Row(0));
    assert!(f.scores.unwrap().out_of_range < 0.01, "{:?}", f.scores);
    let r = &report.restored[0];
    assert!(r.clamped > 0 && !r.exact, "{r:?}");
    assert!(!r.outline_left && r.outline <= OUTLINE_BOUND, "{r:?}");
    assert!(!cleaned.marks_left());
    let again = inspect(&out, &shipped()).unwrap();
    let Visible::Examined { report, .. } = &again.visible else {
        panic!()
    };
    assert!(report.found.is_empty(), "{:#?}", report.found);
}

/// The shipped catalogue with V1's large row and search on GWT's own 96
/// map and its white logo — what every map not yet measured is.
fn gwt_catalogue() -> Catalogue {
    let json = wipemark_pixels::EMBEDDED
        .replace(
            "\"margin\": [64, 64], \"alpha\": \"gemini-v1-96-measured\"",
            "\"margin\": [64, 64], \"alpha\": \"gemini-v1-96\"",
        )
        .replace(
            "\"logo\": [252.1, 253.5, 252.8]",
            "\"logo\": [255, 255, 255]",
        );
    assert_ne!(json, wipemark_pixels::EMBEDDED, "the manifest moved");
    Catalogue::parse(&json, &|name: &str| {
        wipemark_pixels::shipped_assets()
            .find(|(n, _)| *n == name)
            .map(|(_, b)| b)
    })
    .unwrap()
}

/// Under GWT's own 96 map — 1–6/255 of capture noise over its whole
/// square — the square around a real mark is still left as it was: the
/// noise far from the sparkle is not subtracted (D241). Subtracted, it
/// darkened the square by about a level, visible on a flat background.
#[test]
fn gwts_own_map_leaves_the_square_around_a_real_mark_alone() {
    let gwt = gwt_catalogue();
    let map = &gwt
        .profile("gemini-sparkle-v1")
        .unwrap()
        .maps
        .iter()
        .find(|(id, _)| id == "gemini-v1-96")
        .unwrap()
        .1;
    let options = PictureOptions {
        scope: wipemark_image::Scope::AiProvenance,
        catalogue: Some(&gwt),
    };
    for name in ["torch-1025.png", "victory-1025.png"] {
        let bytes = fixture(name);
        let before = raster_of(&bytes);
        let (out, _) = clean(&bytes, &options).unwrap();
        let after = raster_of(&out);
        let c = before.layout().channels();
        let (mut moved, mut n) = (0f64, 0f64);
        for y in 0..96u32 {
            for x in 0..96u32 {
                let a = map.get(i64::from(x), i64::from(y)) * 255.0;
                if !(0.5..6.5).contains(&a) {
                    continue;
                }
                let near = (-3i64..=3).any(|dy| {
                    (-3i64..=3)
                        .any(|dx| map.get(i64::from(x) + dx, i64::from(y) + dy) * 255.0 >= 7.0)
                });
                if near {
                    continue;
                }
                let i = (((ROW.y + y) * before.width() + ROW.x + x) as usize) * c;
                for k in 0..3 {
                    moved += f64::from(after.samples()[i + k]) - f64::from(before.samples()[i + k]);
                    n += 1.0;
                }
            }
        }
        let mean = moved / n;
        assert!(mean.abs() <= 0.1, "{name}: the square moved by {mean:.2}");
    }
}

/// The same picture under GWT's own 96 map and white logo — what every
/// map not yet measured is (V1's 48, both of V2's): on the saturated
/// green they leave stored values up to 6 levels under what a blend could
/// produce over 0, and the out-of-range measure, counted in stored levels
/// with room for an 8-bit capture (D240), still proves the mark. The rule
/// it replaced — a level, amplified by `1/(1 − α)` — refused it (21 % out).
#[test]
fn gwts_own_map_still_proves_the_mark_on_a_saturated_green() {
    let gwt = gwt_catalogue();
    let options = PictureOptions {
        scope: wipemark_image::Scope::AiProvenance,
        catalogue: Some(&gwt),
    };
    let (_, cleaned) = clean(&fixture("anchor-green-1025.png"), &options).unwrap();
    let Visible::Examined { report, .. } = &cleaned.visible else {
        panic!("{:?}", cleaned.visible)
    };
    assert_eq!(report.restored.len(), 1, "{:#?}", report.found);
    assert!(!cleaned.marks_left());
}

/// The same sticker with its background cut out: the vendor's mark is
/// still in the colour channels, under alpha 0. It is a blend, so it is
/// seen — and refused as `Transparent` (D157): what the blend meant under
/// pixels nobody sees is unknown. Left, said, the picture as it was.
#[test]
fn a_real_mark_under_a_transparent_corner_is_seen_and_left() {
    let bytes = fixture("crying-transparent-1025.png");
    let (_, cleaned) = clean(&bytes, &shipped()).unwrap();
    let Visible::Examined { report, .. } = &cleaned.visible else {
        panic!("{:?}", cleaned.visible)
    };
    assert!(report.restored.is_empty(), "{:#?}", report.found);
    let f = report.found.first().expect("the sparkle was not seen");
    assert_eq!(f.profile, "gemini-sparkle-v1");
    assert_eq!(f.verdict, Verdict::Refused(Refusal::Transparent));
    assert!(cleaned.marks_left());
    assert_eq!(cleaned.encoding, Encoding::Unchanged);
}

/// A sticker cut out of its background — the corner transparent — with
/// confetti in it: a white shape NCC likes, under transparent pixels. Not
/// a blend, so not a finding: a picture with no mark in it must not be
/// reported as carrying one because its corner is transparent.
#[test]
fn a_cut_out_sticker_is_not_a_finding() {
    let bytes = fixture("cut-out-confetti-256.webp");
    let seen = inspect(&bytes, &shipped()).unwrap();
    let Visible::Examined { report, .. } = &seen.visible else {
        panic!("{:?}", seen.visible)
    };
    assert!(report.found.is_empty(), "{:#?}", report.found);
}
