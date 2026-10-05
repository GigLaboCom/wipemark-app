//! The shipped catalogue and its assets: every map is the PNG it came
//! from, every pin matches its file, and the catalogue reads. And the
//! catalogue's own refusals, on a synthetic profile.
//!
//! The four Gemini maps are extracted from GeminiWatermarkTool at
//! `7c6a99f` by `crates/wipemark-pixels/marks/gwt/extract.py`
//! (`marks/README.md`); until that has been run, the first three tests
//! here are red by design — a catalogue whose assets are not in the build
//! is not one to ship.

mod support;

use std::path::PathBuf;

use support::*;
use wipemark_pixels::{
    clean, composite, drawn, shipped_assets, AlphaMap, AssetProblem, Catalogue, CatalogueError,
    ExamineOptions, Layout, PixelRect, EMBEDDED,
};

/// The four PNGs as GWT embeds them, by their sha256 (the plan's §3).
const GWT: [(&str, &str, &str); 4] = [
    (
        "bg_48.png",
        "gemini-v1-48.wma",
        "4afc99afe0ef108d67acc45bf4dc5da867ddb793bebc89c9243bb121ce7f0f57",
    ),
    (
        "bg_96.png",
        "gemini-v1-96.wma",
        "3e26f2233a12a5829acac174d8df1f3db40e07fef04ecdd0e035732154077911",
    ),
    (
        "bg_b_36.png",
        "gemini-v2-36.wma",
        "a3e7d5ca932e6acf9ff826a4db47d597458480e72089da81a40bd4b52668cd31",
    ),
    (
        "bg_b_96.png",
        "gemini-v2-96.wma",
        "3911f3b68b3083096326cee24f09868ec87f8d39d248e97057cd14ee838c5552",
    ),
];

fn gwt_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("marks/gwt")
}

#[test]
fn the_shipped_catalogue_reads() {
    let catalogue = Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"));
    let ids: Vec<_> = catalogue.profiles().iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, ["gemini-sparkle-v1", "gemini-sparkle-v2"]);
    for p in catalogue.profiles() {
        assert_eq!(
            (p.vendor.as_str(), p.product.as_str()),
            ("google", "gemini")
        );
        // V1's logo is the colour measured on 22 real outputs (D242);
        // V2's has no real output to measure yet, and is GWT's white.
        let logo = if p.id == "gemini-sparkle-v1" {
            [252.1, 253.5, 252.8]
        } else {
            [255.0; 3]
        };
        assert_eq!(p.logo, logo, "{}", p.id);
        assert!(p.maps.iter().all(|(_, m)| m.peak() < 0.6), "{}", p.id);
    }
}

/// Every map is the PNG it came from: `sample = max(R, G, B)` of the
/// committed original, whose own sha256 is the one GWT's array has.
#[test]
fn gwt_masks_are_the_pngs_they_came_from() {
    for (png_name, wma_name, sha) in GWT {
        let png_bytes =
            std::fs::read(gwt_dir().join(png_name)).unwrap_or_else(|e| panic!("{png_name}: {e}"));
        assert_eq!(sha256_hex(&png_bytes), sha, "{png_name}");
        let mut decoder = png::Decoder::new(std::io::Cursor::new(&png_bytes));
        decoder.set_transformations(png::Transformations::EXPAND);
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).unwrap();
        let channels = info.color_type.samples();
        assert_eq!(info.bit_depth, png::BitDepth::Eight, "{png_name}");
        let expected: Vec<u8> = buf[..info.buffer_size()]
            .chunks_exact(channels)
            .map(|p| match channels {
                1 | 2 => p[0],
                _ => p[0].max(p[1]).max(p[2]),
            })
            .collect();
        let wma = shipped_assets()
            .find(|(n, _)| *n == wma_name)
            .unwrap_or_else(|| panic!("{wma_name} is not in the build"))
            .1;
        let map = AlphaMap::read(wma).unwrap();
        assert_eq!(
            (map.width(), map.height()),
            (info.width, info.height),
            "{wma_name}"
        );
        assert_eq!(
            &wma[9..],
            expected.as_slice(),
            "{wma_name}: not max(R, G, B) of {png_name}"
        );
    }
}

/// Every pin in the committed catalogue is the hash of the file it names.
#[test]
fn every_asset_matches_its_catalogue_hash() {
    let json: serde_json::Value = serde_json::from_str(EMBEDDED).unwrap();
    let mut seen = 0;
    for profile in json["profiles"].as_array().unwrap() {
        for alpha in profile["alpha"].as_array().unwrap() {
            let name = alpha["asset"].as_str().unwrap();
            let bytes = shipped_assets()
                .find(|(n, _)| *n == name)
                .unwrap_or_else(|| panic!("{name} is not in the build"))
                .1;
            assert_eq!(
                alpha["sha256"].as_str().unwrap(),
                sha256_hex(bytes),
                "{name}"
            );
            seen += 1;
        }
    }
    // GWT's four, and V1's 96 measured from real outputs (D243).
    assert_eq!(seen, 5);
}

/// A shipped map composited onto generated pictures comes back off to
/// within a level, by the row, and the report says which profile.
#[test]
fn a_shipped_mark_comes_back_within_one_level() {
    let catalogue = Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"));
    let v1 = catalogue.profile("gemini-sparkle-v1").unwrap();
    let (_, small) = v1.maps.iter().find(|(id, _)| id == "gemini-v1-48").unwrap();
    let (w, h) = (640, 480);
    let at = PixelRect {
        x: w - 32 - 48,
        y: h - 32 - 48,
        width: 48,
        height: 48,
    };
    for (name, original) in backgrounds(w, h, Layout::Rgb8) {
        let mut marked = original.clone();
        // As the vendor draws it: the capture's noise is not the mark
        // (D241), the logo the colour measured on real outputs (D242).
        composite(&mut marked, &drawn(small), at, v1.logo);
        let report = clean(&mut marked, catalogue, &ExamineOptions::default());
        assert_eq!(report.restored.len(), 1, "{name}: {:#?}", report.found);
        assert_eq!(report.restored[0].profile, "gemini-sparkle-v1", "{name}");
        // Within a level everywhere; "exact" exactly when nothing was
        // clamped — over a dark corner a fractional logo's rounding puts a
        // few samples half a level under zero.
        let r = &report.restored[0];
        assert_eq!(r.exact, r.clamped == 0, "{name}: {r:?}");
        assert!(max_error(&marked, &original) <= 1, "{name}");
        // A lossless source is not held to a texture (D251): on a glyph
        // sheet whose strokes run under the mark and miss the ring around
        // it, the picture's own roughness is 20 levels against none.
        assert!(!r.texture_left && !report.marks_left(), "{name}: {r:?}");
    }
}

/// A flipped byte in an asset is a refusal that names the map.
#[test]
fn a_tampered_asset_is_refused_by_name() {
    let v1 = synthetic_v1();
    let mut assets = Vec::new();
    let row = profile_json(&v1, &mut assets, 32);
    assert!(parse_with(std::slice::from_ref(&row), &assets).is_ok());
    let last = assets[0].1.len() - 1;
    assets[0].1[last] ^= 1;
    assert_eq!(
        parse_with(&[row], &assets).unwrap_err(),
        CatalogueError::Asset {
            profile: v1.id.into(),
            id: "small".into(),
            problem: AssetProblem::Hash,
        }
    );
}

/// The schema is checked before a profile exists: a blend model this
/// version does not implement, and a placement naming a map the profile
/// does not list, are refusals naming the profile.
#[test]
fn the_catalogue_refuses_linear_light_and_unknown_maps() {
    let v1 = synthetic_v1();
    let mut assets = Vec::new();
    let row = profile_json(&v1, &mut assets, 32);
    for (from, to) in [
        ("\"encoded\"", "\"linear-light\""),
        ("\"alpha\": \"small\"", "\"alpha\": \"medium\""),
    ] {
        let bad = row.replace(from, to);
        assert_ne!(bad, row, "{from} is not in the row");
        match parse_with(&[bad], &assets) {
            Err(CatalogueError::Profile { id, .. }) => assert_eq!(id, v1.id),
            other => panic!("{to}: {other:?}"),
        }
    }
}
