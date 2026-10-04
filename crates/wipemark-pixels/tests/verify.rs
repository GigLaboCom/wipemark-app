//! The second proof: it tells one opacity from another, refuses a
//! look-alike that NCC loves, measures the inverse unclamped, and finds a
//! second mark in the second pass — or reports it when it is no longer a
//! blend.

mod support;

use support::*;
use wipemark_pixels::{
    clean, composite, examine, resampled, Catalogue, CatalogueError, ExamineOptions, Layout,
    PixelRect, Placed, Raster, Refusal, Verdict,
};

const W: u32 = 320;
const H: u32 = 240;

fn options() -> ExamineOptions {
    ExamineOptions::default()
}

/// Two profiles of one shape at two opacities propose the same place with
/// the same NCC; only verification tells them apart, and the finding
/// names the one that verified, with the other under it.
#[test]
fn verification_tells_v1_from_v2() {
    let catalogue = synthetic_catalogue();
    let mut raster = picture(Kind::Fractal, W, H, 21, Layout::Rgb8);
    composite(
        &mut raster,
        &synthetic_v1().small,
        small_row(W, H, 48),
        [255.0; 3],
    );
    let exam = examine(&raster, &catalogue, &options());
    let verified: Vec<_> = exam
        .findings
        .iter()
        .filter(|f| f.verified().is_some())
        .collect();
    assert_eq!(verified.len(), 1, "{:#?}", exam.findings);
    let winner = verified[0];
    assert_eq!(winner.profile, "test-sparkle-v1");
    let v2 = winner
        .also_tried
        .iter()
        .find(|t| t.profile == "test-sparkle-v2")
        .expect("V2 proposed the same place");
    assert!(!v2.verified);
    assert!(matches!(v2.refusal, Some(Refusal::Gain { .. })), "{v2:?}");
    // And no finding about V2 stands on its own at that place.
    assert!(exam.findings.iter().all(|f| f.profile != "test-sparkle-v2"));
}

/// An opaque white sparkle where the map is above a quarter: NCC
/// proposes it (it is the right shape), and the edge test refuses it — it
/// is not a blend.
#[test]
fn an_opaque_lookalike_is_proposed_and_refused() {
    let catalogue = catalogue_of(&[synthetic_v1()]);
    for kind in [Kind::Gradient, Kind::Fractal, Kind::Flat, Kind::ValueNoise] {
        let mut raster = picture(kind, W, H, 31, Layout::Rgb8);
        stamp_opaque(
            &mut raster,
            &synthetic_v1().small,
            small_row(W, H, 48),
            0.25,
            255.0,
        );
        let before = raster.clone();
        let report = clean(&mut raster, &catalogue, &options());
        assert!(
            !report.found.is_empty(),
            "{kind:?}: the look-alike was not proposed"
        );
        assert!(report.found[0].ncc >= 0.70);
        assert!(
            report.found.iter().all(|f| f.verified().is_none()),
            "{kind:?}: {:#?}",
            report.found
        );
        assert!(report.restored.is_empty());
        assert_eq!(raster, before);
    }
}

/// The same shape at 0.72 of the opacity: refused, and the gain it would
/// have needed is reported.
#[test]
fn a_mark_at_the_wrong_opacity_is_refused_and_its_gain_reported() {
    let catalogue = catalogue_of(&[synthetic_v1()]);
    let weaker = scaled(&synthetic_v1().small, 0.72);
    for kind in [Kind::Gradient, Kind::Fractal, Kind::Flat] {
        let mut raster = picture(kind, W, H, 41, Layout::Rgb8);
        composite(&mut raster, &weaker, small_row(W, H, 48), [255.0; 3]);
        let exam = examine(&raster, &catalogue, &options());
        let f = exam.findings.first().expect("proposed");
        match f.verdict {
            Verdict::Refused(Refusal::Gain { k }) => {
                assert!((k - 0.72).abs() <= 0.03, "{kind:?}: k* = {k}");
            }
            ref other => panic!("{kind:?}: {other:?}"),
        }
    }
}

/// A grey sparkle drawn hard-edged on black: a hypothesis whose inverse
/// must leave the range to explain the edge. Measured unclamped, it is
/// refused; nothing is written.
#[test]
fn the_verifier_measures_the_unclamped_inverse() {
    let catalogue = catalogue_of(&[synthetic_v1()]);
    let mut raster = picture(Kind::Dark, W, H, 51, Layout::Rgb8);
    stamp_opaque(
        &mut raster,
        &synthetic_v1().small,
        small_row(W, H, 48),
        0.25,
        127.0,
    );
    let before = raster.clone();
    let report = clean(&mut raster, &catalogue, &options());
    assert!(
        !report.found.is_empty(),
        "the grey sparkle was not proposed"
    );
    assert!(
        report.found.iter().all(|f| f.verified().is_none()),
        "{:#?}",
        report.found
    );
    assert_eq!(raster, before);

    // And a true mark on the same black corner is proved and restored.
    let original = picture(Kind::Dark, W, H, 52, Layout::Rgb8);
    let mut marked = original.clone();
    composite(
        &mut marked,
        &synthetic_v1().small,
        small_row(W, H, 48),
        [255.0; 3],
    );
    let report = clean(&mut marked, &catalogue, &options());
    assert_eq!(report.restored.len(), 1, "{:#?}", report.found);
    assert!(max_error(&marked, &original) <= 1);
}

/// Two marks, 48 and 44 pixels, nine pixels apart: the first pass
/// restores the one at its row, the second finds the other by search,
/// proves it and restores it (D165).
#[test]
fn a_second_overlapping_mark_is_found_in_the_second_pass() {
    let catalogue = synthetic_catalogue();
    let v1 = synthetic_v1();
    let original = picture(Kind::Flat, W, H, 61, Layout::Rgb8);
    let mut marked = original.clone();
    let row = small_row(W, H, 48);
    let older = resampled(&v1.large, 44.0, 0.0, 0.0).unwrap();
    let older_at = PixelRect {
        x: row.x - 9,
        y: row.y - 9,
        width: 44,
        height: 44,
    };
    composite(&mut marked, &older, older_at, [255.0; 3]);
    composite(&mut marked, &v1.small, row, [255.0; 3]);
    let report = clean(&mut marked, &catalogue, &options());
    assert_eq!(report.restored.len(), 2, "{:#?}", report.found);
    let second: Vec<_> = report.found.iter().filter(|f| f.pass == 2).collect();
    assert!(
        second
            .iter()
            .any(|f| f.verified().is_some() && f.placed == Placed::Searched),
        "{second:#?}"
    );
    // Two exact inversions in a row: a level each at most.
    assert!(max_error(&marked, &original) <= 2);
}

/// The older mark baked into regenerated content — resampled and
/// softened, no longer the blend — is reported and not restored.
#[test]
fn a_resampled_second_mark_is_refused_not_restored() {
    let catalogue = synthetic_catalogue();
    let v1 = synthetic_v1();
    let mut marked = picture(Kind::Flat, W, H, 71, Layout::Rgb8);
    let row = small_row(W, H, 48);
    let baked = blurred(&resampled(&v1.large, 44.0, 0.0, 0.0).unwrap(), 1);
    let baked_at = PixelRect {
        x: row.x - 9,
        y: row.y - 9,
        width: 44,
        height: 44,
    };
    composite(&mut marked, &baked, baked_at, [255.0; 3]);
    composite(&mut marked, &v1.small, row, [255.0; 3]);
    let report = clean(&mut marked, &catalogue, &options());
    assert!(report.restored.len() <= 1, "{:#?}", report.found);
    assert!(
        report.found.iter().any(|f| f.verified().is_none()),
        "the baked mark was not reported: {:#?}",
        report.found
    );
    assert!(report.marks_left());
}

/// Errors are values: a broken catalogue, a truncated map, a raster too
/// small to hold a mark — never a panic, never an empty success.
#[test]
fn refusals_are_values_never_panics() {
    let v1 = synthetic_v1();
    let mut assets = Vec::new();
    let good = profile_json(&v1, &mut assets, 32);
    for bad in [
        String::from("not json"),
        String::from("{\"schema\": 2, \"profiles\": []}"),
        format!("{{ \"schema\": 1, \"profiles\": [{good}, {good}] }}"),
        good.replace("\"encoded\"", "\"linear-light\""),
        good.replace("\"alpha\": \"small\"", "\"alpha\": \"nowhere\""),
        good.replace("\"min_ncc\": 0.70", "\"min_ncc\": 1.70"),
        good.replace("\"opaque_above\": 0.95", "\"opaque_above\": 0"),
        good.replace("bottom-right", "middle"),
    ] {
        let json = if bad.starts_with("{ \"schema\"") || !bad.contains("\"id\"") {
            bad.clone()
        } else {
            format!("{{ \"schema\": 1, \"profiles\": [{bad}] }}")
        };
        let parsed = Catalogue::parse(&json, &|name: &str| {
            assets
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, b)| b.as_slice())
        });
        assert!(parsed.is_err(), "accepted: {json}");
    }
    // A truncated asset under a pin that matches it is still refused, as
    // a map.
    let truncated = v1.small.write(8).unwrap()[..20].to_vec();
    let mut short = vec![(String::from("cut.wma"), truncated.clone())];
    short.push(assets[1].clone());
    let row = good
        .replace(&format!("{}-small.wma", v1.id), "cut.wma")
        .replace(&sha256_hex(&assets[0].1), &sha256_hex(&truncated));
    assert!(matches!(
        parse_with(&[row], &short),
        Err(CatalogueError::Asset { .. })
    ));
    // Pictures too small for any placement or search: no finding, no panic.
    let catalogue = synthetic_catalogue();
    for (w, h) in [(1, 1), (5, 3), (24, 24), (60, 9), (9, 60)] {
        let mut raster = picture(Kind::ValueNoise, w, h, 81, Layout::Rgb8);
        let report = clean(&mut raster, &catalogue, &options());
        assert!(report.restored.is_empty());
        assert!(!report.not_established.is_empty());
    }
    assert!(Raster::new(0, 0, Layout::Rgb8, vec![]).is_err());
}

/// D156: the third shelf is there for a picture where nothing was found,
/// the pixel claim first.
#[test]
fn the_report_always_carries_the_third_shelf() {
    let catalogue = synthetic_catalogue();
    let mut raster = picture(Kind::Gradient, W, H, 91, Layout::Rgb8);
    let report = clean(&mut raster, &catalogue, &options());
    assert!(report.found.is_empty());
    assert_eq!(
        report.not_established[0],
        wipemark_pixels::not_established::ID
    );
    assert_eq!(report.not_established.len(), 4);
    let json = report.to_json();
    assert!(
        json.contains("\"not_established\":[\"invisible-pixel-marks\","),
        "{json}"
    );
}

/// The report's JSON is ASCII and its field names are a format.
#[test]
fn the_report_json_is_ascii_and_stable() {
    let catalogue = synthetic_catalogue();
    let mut raster = picture(Kind::Fractal, W, H, 101, Layout::Rgb8);
    composite(
        &mut raster,
        &synthetic_v1().small,
        small_row(W, H, 48),
        [255.0; 3],
    );
    let json = clean(&mut raster, &catalogue, &options()).to_json();
    assert!(json.is_ascii(), "{json}");
    for key in [
        "\"found\":[{\"profile\":\"test-sparkle-v1\",\"vendor\":\"test\",\"product\":\"synthetic\",\"pass\":1,",
        "\"rect\":{\"x\":",
        "\"pixels\":{\"x\":",
        "\"placed\":\"row\",\"row\":1,",
        "\"verdict\":\"verified\",\"refusal\":null,",
        "\"scores\":{\"gain\":",
        "\"also_tried\":[{\"profile\":\"test-sparkle-v2\",\"verified\":false,\"refusal\":{\"why\":\"gain\",\"k\":",
        "\"restored\":[{\"profile\":\"test-sparkle-v1\",\"rect\":{\"x\":",
        "\"changed\":",
        "\"exact\":true}]",
        "\"not_established\":[\"invisible-pixel-marks\",\"vendor-detector-evasion\",\"human-authorship\",\"unknown-mark-schemes\"]}",
    ] {
        assert!(json.contains(key), "{key} not in {json}");
    }
}
