//! The planar inverse (E12-R6, D471) on real files: the owner's Gemini
//! crops (`fixtures/image/gemini/`), decoded with their planes
//! (`decode_with_planes`) and cleaned on the planar path
//! (`clean_bytes_with_planes`, `wipemark_pixels::clean_with`), against the
//! RGB path (`wipemark_pixels::clean`, the product's road before D471 and
//! still its road for a JPEG whose planes do not read), and the product's
//! own `clean`, which takes the planes since D471 and refines after them
//! (D472).

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
/// left (R0: the pixels pass with no planes, the product's path before
/// D471); the restoration says it was planar.
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
/// same JSON as the RGB path, through the pixels pass given planes (4:4:4's
/// own; for the PNG, 4:2:0 planes made from it) (§4.4, L1). Through the
/// whole file, the PNG's `clean` is the planar road's byte for byte (no
/// refinement ever runs on a lossless source, S6); the 4:4:4 JPEG's is
/// the old restoration refined by DCT-POCS (D472), never planar.
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

        let (out_clean, report_clean) = clean(&bytes, &shipped()).unwrap();
        let (out_new, report_new) = clean_bytes_with_planes(&bytes, &shipped()).unwrap();
        assert!(!report_new.to_json().contains("planar"), "{name}");
        assert!(!report_clean.to_json().contains("planar"), "{name}");
        if d.fidelity == wipemark_pixels::Fidelity::Lossless {
            assert!(out_clean == out_new, "{name}: the files differ");
            assert_eq!(report_clean.to_json(), report_new.to_json(), "{name}");
        } else {
            let r = restored(name, &report_clean);
            let method = r.interval.map(|i| i.method);
            assert_eq!(method, Some(wipemark_pixels::Method::Dct), "{name}: {r:?}");
        }
    }
}

/// The identity (D494): the planar restoration blended back with the mark
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

/// The product's road takes the planes (D471, the owner, 2026-10-10): a
/// 4:2:0 JPEG looked at by `inspect` is proved in its planes, and cleaned
/// by `clean` is restored in them, the report saying so — with no feature
/// and nothing asked. `decode` alone, which the proof re-reads the output
/// with, still hands out no planes.
#[test]
fn the_product_takes_the_planes_on_a_subsampled_jpeg() {
    let name = "victory-1040-q95-420.jpg";
    let bytes = fixture(name);
    let (_, report) = clean(&bytes, &shipped()).unwrap();
    let r = restored(name, &report);
    assert!(
        matches!(
            r.planar,
            Some(Planar::Inverse {
                sampling: Sampling::H420,
                ..
            })
        ),
        "{name}: {r:?}"
    );
    assert!(r.chroma < CHROMA_LEVELS && !r.outline_left, "{name}: {r:?}");
    let seen = wipemark_picture::inspect(&bytes, &shipped()).unwrap();
    let Visible::Examined { report, .. } = &seen.visible else {
        panic!("{name}: {:?}", seen.visible)
    };
    let scores = report.found[0].scores.unwrap();
    assert!(scores.planar.is_some(), "{name}: {scores:?}");
    let container = ImageContainer::sniff(&bytes).unwrap();
    assert!(decode(&bytes, container).unwrap().unwrap().planes.is_none());
}
