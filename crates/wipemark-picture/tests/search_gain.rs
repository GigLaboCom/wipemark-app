//! The search's own gain windows (D530): a mark resampled with its picture
//! sits at no row's place, so D470 never holds it; `search.gain` in the
//! catalogue does. A mark drawn at `k = 0.93` and resized with its picture
//! is refused by its gain under the shipped windows and proved with them
//! off — the catalogue's value switches it, no code — while a mark drawn at
//! the profile's own opacity, and the vendor's own mark, resized the same
//! way, are proved by the search as before.

use std::path::PathBuf;

use image::imageops::FilterType;
use serde_json::{json, Value};
use wipemark_picture::decode;
use wipemark_pixels::synth::{composite_with, Blend};
use wipemark_pixels::{
    drawn, Catalogue, Examination, ExamineOptions, Finding, GainBand, Kernel, Layout, Placed,
    Raster, Refusal, SubRect, Verdict, EMBEDDED,
};

#[path = "../../wipemark-pixels/tests/support/mod.rs"]
mod support;

use support::Kind;

/// The shipped catalogue with every profile's `search.gain` set to
/// `bands` — removed when `None`: the second proof as for a row, the value
/// that reproduces the product before D530.
fn with_bands(bands: Option<Value>) -> Catalogue {
    let mut doc: Value = serde_json::from_str(EMBEDDED).unwrap();
    for p in doc["profiles"].as_array_mut().unwrap() {
        let search = p["search"].as_object_mut().unwrap();
        match &bands {
            Some(b) => {
                search.insert(String::from("gain"), b.clone());
            }
            None => {
                search.remove("gain");
            }
        }
    }
    Catalogue::parse(&doc.to_string(), &|name: &str| {
        wipemark_pixels::shipped_assets()
            .find(|(n, _)| *n == name)
            .map(|(_, b)| b)
    })
    .unwrap()
}

fn off() -> Catalogue {
    with_bands(None)
}

/// `raster` resized by `scale` with Catmull-Rom — the bicubic of Pillow's
/// `BICUBIC` — as a person's editor would, and stored losslessly.
fn resized(raster: &Raster, scale: f32) -> Raster {
    let rgb: Vec<u8> = raster
        .samples()
        .chunks_exact(raster.layout().channels())
        .flat_map(|p| [p[0] as u8, p[1] as u8, p[2] as u8])
        .collect();
    let img = image::RgbImage::from_raw(raster.width(), raster.height(), rgb).unwrap();
    let (w, h) = (
        (raster.width() as f32 * scale).round() as u32,
        (raster.height() as f32 * scale).round() as u32,
    );
    let out = image::imageops::resize(&img, w, h, FilterType::CatmullRom);
    Raster::from_u8(w, h, Layout::Rgb8, out.as_raw()).unwrap()
}

/// The search's finding of `profile`, if any.
fn searched<'a>(report: &'a Examination, profile: &str) -> Option<&'a Finding> {
    report
        .findings
        .iter()
        .find(|f| f.profile == profile && f.placed == Placed::Searched)
}

/// A picture with `profile`'s map `map_id` drawn at gain `k`, `margin` in
/// from the bottom-right corner at the map's own size — the row a vendor's
/// output of that size has.
fn marked(profile: &str, map_id: &str, side: u32, margin: u32, bg: (Kind, u64), k: f32) -> Raster {
    let catalogue = Catalogue::shipped().unwrap();
    let p = catalogue.profile(profile).unwrap();
    let (_, map) = p.maps.iter().find(|(id, _)| id == map_id).unwrap();
    let mut raster = support::picture(bg.0, side, side, bg.1, Layout::Rgb8);
    let at = (side - margin - map.width()) as f32;
    let rect = SubRect {
        x: at,
        y: at,
        size: map.width() as f32,
    };
    let blend = Blend {
        k,
        ..Blend::encoded(p.logo)
    };
    composite_with(&mut raster, &drawn(map), rect, Kernel::Area, &blend);
    raster
}

const V1: &str = "gemini-sparkle-v1";
const V2: &str = "gemini-sparkle-v2";

/// Backgrounds over which the search, with the windows off, proves V1's
/// large mark drawn at 0.93 and resized by 0.9 and 1.1 — at `k*` from 0.96
/// to 1.00 (measured 2026-10-11).
const LARGE: [(Kind, u64); 4] = [
    (Kind::Flat, 3),
    (Kind::Gradient, 11),
    (Kind::ValueNoise, 11),
    (Kind::Fractal, 11),
];

/// V2's small mark (36 pixels at a 1024 output, margin 71) resized by 0.9:
/// a template of 32 pixels, in the band under 64 — held to today's window.
const SMALL: [(Kind, u64); 2] = [(Kind::Flat, 3), (Kind::Gradient, 11)];

/// A case: (profile, map, side, margin, background, scale).
type Case = (&'static str, &'static str, u32, u32, (Kind, u64), f32);

/// V1's large mark at a 1025 output (margin 64), resized by 0.9 and 1.1: a
/// template of 86–106 pixels, in the band from 64 up.
fn large_cases() -> Vec<Case> {
    let mut out = Vec::new();
    for bg in LARGE {
        for scale in [0.9, 1.1] {
            out.push((V1, "gemini-v1-96-measured", 1025, 64, bg, scale));
        }
    }
    out
}

fn small_cases() -> Vec<Case> {
    SMALL
        .iter()
        .map(|&bg| (V2, "gemini-v2-36", 1024, 71, bg, 0.9))
        .collect()
}

fn shipped_bands() -> Value {
    json!([{ "under": 64, "window": [0.96, 1.06] }, { "window": [1.02, 1.06] }])
}

/// The bands the shipped catalogue gives both Gemini profiles' searches
/// (D530): a resampled template under 64 pixels is proved at `k*` from 0.96
/// to 1.06 — the window a row has, as the sweep's grid reads it — and a
/// wider one from 1.02 to 1.06.
#[test]
fn the_shipped_searches_carry_d530s_bands() {
    let shipped = Catalogue::shipped().unwrap();
    for p in shipped.profiles() {
        assert_eq!(
            p.search.as_ref().map(|s| s.gain.clone()),
            Some(vec![
                GainBand {
                    under: Some(64.0),
                    window: [0.96, 1.06],
                },
                GainBand {
                    under: None,
                    window: [1.02, 1.06],
                },
            ]),
            "{}",
            p.id
        );
    }
    assert_eq!(*shipped, with_bands(Some(shipped_bands())));
}

/// D530's protection, both ways: a large mark drawn at 0.93 and resized
/// with its picture is proved by the search with the windows off — at `k*`
/// 0.96 to 1.00, and restored as though it were drawn at 1 — and refused by
/// its gain, seen and left, under the shipped ones. Red with `gain_holds`
/// taking the profile's `verify.gain` for the search, and with the bands
/// removed from `manifests/marks.v1.json`.
#[test]
fn a_weaker_mark_resized_with_its_picture_is_refused_by_the_search() {
    let shipped = Catalogue::shipped().unwrap();
    let off = off();
    let options = ExamineOptions::default();
    for (profile, map, side, margin, bg, scale) in large_cases() {
        let name = format!("{map} over {bg:?} x{scale}");
        let raster = resized(&marked(profile, map, side, margin, bg, 0.93), scale);
        let before = wipemark_pixels::examine(&raster, &off, &options);
        let f = searched(&before, profile).unwrap_or_else(|| panic!("{name}: {before:#?}"));
        assert!(f.verified().is_some(), "{name}: off: {f:#?}");

        let mut restored = raster.clone();
        let report = wipemark_pixels::clean(&mut restored, shipped, &options);
        assert!(report.restored.is_empty(), "{name}: {:#?}", report.found);
        assert_eq!(restored, raster, "{name}");
        let f = report
            .found
            .iter()
            .find(|f| f.profile == profile && f.placed == Placed::Searched)
            .unwrap_or_else(|| panic!("{name}: not seen: {:#?}", report.found));
        match f.verdict {
            Verdict::Refused(Refusal::Gain { k }) => assert!(k <= 1.0 + 1e-4, "{name}: {k}"),
            ref other => panic!("{name}: {other:?}"),
        }
        assert!(report.marks_left(), "{name}");
    }
}

/// The same marks drawn at the profile's own opacity are proved by the
/// search under the shipped windows, as before D530: `k*` 1.04–1.06 for the
/// large mark, 1.00–1.02 for the small one. Red with the large band's lower
/// end over 1.04, or the small band's over 1.00.
///
/// The small band does not refuse a small mark drawn at 0.93 (it reads
/// 0.96 here, and 0.96–1.02 on the bench): under 64 pixels a true mark
/// shrunk or resized reads as low, so the band is a row's window (D530).
#[test]
fn a_mark_at_its_own_opacity_resized_with_its_picture_is_still_proved() {
    let shipped = Catalogue::shipped().unwrap();
    let options = ExamineOptions::default();
    for (profile, map, side, margin, bg, scale) in large_cases().into_iter().chain(small_cases()) {
        let name = format!("{map} over {bg:?} x{scale}");
        let raster = resized(&marked(profile, map, side, margin, bg, 1.0), scale);
        let report = wipemark_pixels::examine(&raster, shipped, &options);
        let f = searched(&report, profile).unwrap_or_else(|| panic!("{name}: {report:#?}"));
        assert!(f.verified().is_some(), "{name}: {f:#?}");
    }
}

fn fixture(name: &str) -> Raster {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/image/gemini")
        .join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let container = wipemark_image::inspect(&bytes).unwrap().container;
    decode(&bytes, container).unwrap().unwrap().raster
}

/// The vendor's own mark — first-generation outputs, the bottom-right 1025
/// of a 2048 picture — resized with its picture by 0.9 and 1.1 as the
/// corpus's `recon-resized` files are: proved by the search under the
/// shipped windows, at `k*` 1.06 — the corpus's 63 read 1.04–1.06 — and
/// restored. Red with the large band's upper end under 1.06.
#[test]
fn the_vendors_mark_resized_with_its_picture_is_still_proved_by_the_search() {
    let shipped = Catalogue::shipped().unwrap();
    let options = ExamineOptions::default();
    for name in ["torch-1025.png", "victory-1025.png"] {
        for scale in [0.9f32, 1.1] {
            let label = format!("{name} x{scale}");
            let mut raster = resized(&fixture(name), scale);
            let report = wipemark_pixels::clean(&mut raster, shipped, &options);
            let f = report
                .found
                .iter()
                .find(|f| f.profile == V1 && f.verified().is_some())
                .unwrap_or_else(|| panic!("{label}: {:#?}", report.found));
            assert_eq!(f.placed, Placed::Searched, "{label}");
            let k = f.scores.unwrap().gain;
            println!("{label}: k* {k}");
            assert!((1.02 - 1e-4..=1.06 + 1e-4).contains(&k), "{label}: {k}");
            assert_eq!(report.restored.len(), 1, "{label}");
        }
    }
}
