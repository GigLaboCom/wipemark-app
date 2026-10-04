//! D229: a mark shrunk with its picture and saved with loss — what the
//! host verifier's real files were — is found, fitted to the eighth of a
//! pixel, matched to the filter that shrank it, restored, and leaves no
//! outline past the bound.
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
    clean, composite, Catalogue, ExamineOptions, Fidelity, Layout, PixelRect, Raster, OUTLINE_BOUND,
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
        assert!(
            r.outline <= OUTLINE_BOUND && !r.outline_left,
            "{name}: outline {}",
            r.outline
        );
        assert!(!report.marks_left(), "{name}");
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
    }
}
