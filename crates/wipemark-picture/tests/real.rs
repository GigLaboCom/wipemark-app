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

const MARKED: [&str; 2] = ["crying-1025.png", "torch-1025.png"];

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

/// A real sticker whose corner was edited after the vendor stamped it:
/// the sparkle is there (k* 1.00) but the inverse must leave the range to
/// take it away — not the vendor's blend any more. Seen, not proved, left,
/// and the picture written back as it was.
#[test]
fn an_edited_real_mark_is_seen_and_left() {
    let bytes = fixture("anchor-edited-1025.png");
    let (_, cleaned) = clean(&bytes, &shipped()).unwrap();
    let Visible::Examined { report, .. } = &cleaned.visible else {
        panic!("{:?}", cleaned.visible)
    };
    assert!(report.restored.is_empty(), "{:#?}", report.found);
    let f = report.found.first().expect("the sparkle was not seen");
    assert_eq!(f.profile, "gemini-sparkle-v1");
    assert!(
        matches!(f.verdict, Verdict::Refused(Refusal::OutOfRange { .. })),
        "{:?}",
        f.verdict
    );
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
