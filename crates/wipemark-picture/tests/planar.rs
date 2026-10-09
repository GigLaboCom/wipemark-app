//! The planar inverse (E12-R6, D306) on real files: the owner's Gemini
//! crops (`fixtures/image/gemini/`), decoded with their planes
//! (`decode_with_planes`) and cleaned on the planar path
//! (`clean_bytes_with_planes`, `wipemark_pixels::clean_with`), against the
//! product's path (`clean`, which takes no planes until D306 is taken).

use std::path::PathBuf;

use wipemark_image::ImageContainer;
use wipemark_picture::{
    clean, clean_bytes_with_planes, decode, decode_with_planes, Decoded, PictureOptions,
    PictureReport, Visible,
};
use wipemark_pixels::synth::jpeg_planes;
use wipemark_pixels::{
    clean_with, Catalogue, ExamineOptions, Planar, Restored, Sampling, CHROMA_LEVELS,
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

fn decoded_with_planes(bytes: &[u8]) -> Decoded {
    let container = ImageContainer::sniff(bytes).unwrap();
    decode_with_planes(bytes, container).unwrap().unwrap()
}

/// The one restoration of a cleaned picture.
fn restored(name: &str, report: &PictureReport) -> Restored {
    let Visible::Examined { report, .. } = &report.visible else {
        panic!("{name}: {:?}", report.visible)
    };
    let [r] = report.restored.as_slice() else {
        panic!("{name}: {report:#?}")
    };
    r.clone()
}

/// The target (D247): a 4:2:0 JPEG at 95 keeps a colour fringe of 7.4–7.8
/// levels on the RGB path — the inverse divides chroma the codec averaged
/// over 2 × 2 blocks by a full-resolution `α`. Restored in the planes, the
/// band's colour step falls under the bound, and under what the RGB path
/// left (R0: the pixels pass with no planes, which is the product's path
/// until D306 is taken); the restoration says it was planar.
#[test]
fn a_420_mark_restored_by_planes_has_less_fringe() {
    let catalogue = Catalogue::shipped().unwrap();
    for name in ["thinking-1040-q95-420.jpg", "victory-1040-q95-420.jpg"] {
        let bytes = fixture(name);
        let d = decoded_with_planes(&bytes);
        let options = ExamineOptions {
            source: d.fidelity,
            profiles: None,
        };
        let old = wipemark_pixels::clean(&mut d.raster.clone(), catalogue, &options);
        let [r0] = old.restored.as_slice() else {
            panic!("{name}: {old:#?}")
        };
        let (_, new) = clean_bytes_with_planes(&bytes, &shipped()).unwrap();
        let r6 = restored(name, &new);
        assert!(r0.planar.is_none(), "{name}: the RGB path is not planar");
        assert!(
            matches!(
                r6.planar,
                Some(Planar::Inverse {
                    sampling: Sampling::H420,
                    ..
                })
            ),
            "{name}: {r6:?}"
        );
        assert!(r0.chroma > CHROMA_LEVELS, "{name}: R0 {r0:?}");
        assert!(
            r6.chroma < r0.chroma && r6.chroma < CHROMA_LEVELS,
            "{name}: R0 {} R6 {}",
            r0.chroma,
            r6.chroma
        );
        assert!(!r6.outline_left, "{name}: {r6:?}");
    }
}

/// 4:4:4 already matches the RGB model, and a PNG is lossless: both take
/// the old path whatever planes are handed in — the same raster and the
/// same JSON as the product's `clean`, through the pixels pass given
/// planes (4:4:4's own; for the PNG, 4:2:0 planes made from it) and
/// through the whole file (§4.4, L1).
#[test]
fn a_444_jpeg_and_a_png_take_the_old_path_byte_for_byte() {
    let catalogue = Catalogue::shipped().unwrap();
    for name in ["torch-1025-q95-444.jpg", "torch-1025.png"] {
        let bytes = fixture(name);
        let d = decoded_with_planes(&bytes);
        let planes = match &d.planes {
            Some(p) => {
                assert_eq!(p.sampling(), Sampling::H444, "{name}");
                p.clone()
            }
            None => jpeg_planes(&d.raster, Sampling::H420, 95).unwrap().0,
        };
        let options = ExamineOptions {
            source: d.fidelity,
            profiles: None,
        };
        let (mut old, mut new) = (d.raster.clone(), d.raster.clone());
        let a = wipemark_pixels::clean(&mut old, catalogue, &options);
        let b = clean_with(&mut new, Some(&planes), catalogue, &options);
        assert!(!a.restored.is_empty(), "{name}: {a:#?}");
        assert!(old == new, "{name}: the rasters differ");
        assert_eq!(a.to_json(), b.to_json(), "{name}");
        assert!(!b.to_json().contains("planar"), "{name}");

        let (out_old, report_old) = clean(&bytes, &shipped()).unwrap();
        let (out_new, report_new) = clean_bytes_with_planes(&bytes, &shipped()).unwrap();
        assert!(out_old == out_new, "{name}: the files differ");
        assert_eq!(report_old.to_json(), report_new.to_json(), "{name}");
    }
}

/// The identity (D305): the planar restoration blended back with the mark
/// is what the file stored, to within a level at the 95th percentile, on
/// every 4:2:0 fixture — measured in the planes.
#[test]
fn the_planar_inverse_is_still_an_inverse() {
    for name in [
        "thinking-1040-q95-420.jpg",
        "torch-1025-q95-420.jpg",
        "victory-1025-q95-420.jpg",
        "victory-1025-q98-420.jpg",
        "victory-1040-q95-420.jpg",
    ] {
        let (_, report) = clean_bytes_with_planes(&fixture(name), &shipped()).unwrap();
        let r = restored(name, &report);
        assert!(r.planar.is_some(), "{name}");
        let c = r.consistency_px;
        assert!(c <= 1.0, "{name}: consistency_px {c}");
    }
}

/// The product's road takes no planes until D306 is taken (S12): a 4:2:0
/// JPEG cleaned by `clean` is restored in RGB, with no `planar` in its
/// report — unless this build has `planar-preview`.
#[test]
fn the_product_takes_the_planes_only_with_the_preview() {
    let bytes = fixture("victory-1040-q95-420.jpg");
    let (_, report) = clean(&bytes, &shipped()).unwrap();
    let r = restored("victory-1040-q95-420.jpg", &report);
    assert_eq!(
        r.planar.is_some(),
        cfg!(feature = "planar-preview"),
        "{r:?}"
    );
    let container = ImageContainer::sniff(&bytes).unwrap();
    assert!(decode(&bytes, container).unwrap().unwrap().planes.is_none());
}
