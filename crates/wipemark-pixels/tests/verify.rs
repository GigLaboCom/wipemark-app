//! The second proof: it tells one opacity from another, dismisses a
//! look-alike that NCC loves, measures the inverse unclamped, finds a
//! second mark apart in the second pass, and refuses marks that overlap —
//! or reports one when it is no longer the blend.

mod support;

use support::*;
use wipemark_pixels::{
    clean, composite, drawn, examine, refine_at, resampled, Catalogue, CatalogueError,
    ExamineOptions, Layout, PixelRect, Placed, Raster, Refusal, SubRect, Verdict,
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

/// An opaque white sparkle where the map is above a quarter: the right
/// shape, so NCC proposes it — and the second proof finds no blend (the
/// edges vanish at `k*` near 0 and inverting adds contour). It is not a
/// finding at all (D235): not reported, not restored, no exit code.
#[test]
fn an_opaque_lookalike_is_not_a_finding() {
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
        assert!(report.found.is_empty(), "{kind:?}: {:#?}", report.found);
        assert!(
            report.dismissed > 0,
            "{kind:?}: the look-alike was not even proposed"
        );
        assert!(report.restored.is_empty());
        assert!(!report.marks_left());
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

/// Two marks apart: the small one at its row, an older one of the same
/// profile, resampled to 40 pixels, in the far corner of the search box.
/// The first pass proves the row and so never searches; the second, over
/// the restored raster, finds the other by search, proves it and restores
/// it (D165, D239) — and the first pass's sight of it, refused under the
/// other profile, is listed under the proof rather than left as a mark.
#[test]
fn a_second_mark_apart_is_found_in_the_second_pass() {
    let catalogue = synthetic_catalogue();
    let v1 = synthetic_v1();
    for kind in [Kind::Flat, Kind::Gradient, Kind::Fractal] {
        let original = picture(kind, W, H, 61, Layout::Rgb8);
        let mut marked = original.clone();
        let older = resampled(&v1.large, 40.0, 0.0, 0.0).unwrap();
        composite(
            &mut marked,
            &older,
            PixelRect {
                x: 70,
                y: 60,
                width: 40,
                height: 40,
            },
            [255.0; 3],
        );
        composite(&mut marked, &v1.small, small_row(W, H, 48), [255.0; 3]);
        let report = clean(&mut marked, &catalogue, &options());
        assert_eq!(report.restored.len(), 2, "{kind:?}: {:#?}", report.found);
        let second: Vec<_> = report.found.iter().filter(|f| f.pass == 2).collect();
        assert!(
            second
                .iter()
                .any(|f| f.verified().is_some() && f.placed == Placed::Searched),
            "{kind:?}: {second:#?}"
        );
        assert!(!report.marks_left(), "{kind:?}: {:#?}", report.found);
        // The row's is exact; the searched one is the same resampled map at
        // a whole-pixel place, so it comes back as closely.
        assert!(max_error(&marked, &original) <= 1, "{kind:?}");
    }
}

/// Two marks, 48 and 44 pixels, nine pixels apart: the first pass proves
/// and restores the one at its row; the second finds the other by search,
/// proves it and restores it (D165, D239). Blends of one logo colour
/// commute — `1 − (1−α₁)(1−α₂)` either way round — so the row's mark is
/// an exact blend whichever was stamped last, and the order does not
/// matter.
#[test]
fn a_second_overlapping_mark_is_found_in_the_second_pass() {
    for older_on_top in [false, true] {
        let (report, error) = two_overlapping(older_on_top);
        assert_eq!(report.restored.len(), 2, "{:#?}", report.found);
        let first = report
            .found
            .iter()
            .find(|f| f.pass == 1 && f.verified().is_some());
        assert_eq!(
            first.map(|f| f.placed),
            Some(Placed::Row(1)),
            "{:#?}",
            report.found
        );
        let second: Vec<_> = report.found.iter().filter(|f| f.pass == 2).collect();
        assert!(
            second
                .iter()
                .any(|f| f.verified().is_some() && f.placed == Placed::Searched),
            "{second:#?}"
        );
        assert!(!report.marks_left());
        // Two inversions where they overlap: the first's rounding, then the
        // second's, the first's amplified by 1/(1 − α) on the way.
        assert!(error <= 3, "on top {older_on_top}: off by {error}");
    }
}

/// The 48 at its row and a 44 nine pixels up and left, on a flat picture;
/// the 44 stamped last when `older_on_top`. The report, and the largest
/// error against the unmarked picture afterwards.
fn two_overlapping(older_on_top: bool) -> (wipemark_pixels::PixelReport, u16) {
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
    if older_on_top {
        composite(&mut marked, &v1.small, row, [255.0; 3]);
        composite(&mut marked, &older, older_at, [255.0; 3]);
    } else {
        composite(&mut marked, &older, older_at, [255.0; 3]);
        composite(&mut marked, &v1.small, row, [255.0; 3]);
    }
    let report = clean(&mut marked, &catalogue, &options());
    (report, max_error(&marked, &original))
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

/// The third proof (D240): a blend proved by its gain and its edges is
/// still refused when the stored values lie outside what a blend with this
/// map and logo could produce over any picture — here, the mark over a
/// green whose red is 0, then the red under the mark's square lowered by
/// `by` levels after it was stamped: a corner painted over the mark. The
/// gain and the edges cannot see it (red is a fifth of luma); the range
/// can. Within [`BLEND_LEVELS`] (an 8-bit capture of the vendor's α, a
/// codec's error) it is still the blend; past it, it is not.
#[test]
fn a_mark_painted_over_out_of_range_is_refused_and_the_allowance_is_eight_levels() {
    let shipped = Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"));
    let v1 = shipped.profile("gemini-sparkle-v1").unwrap();
    let (_, small) = v1.maps.iter().find(|(id, _)| id == "gemini-v1-48").unwrap();
    let (w, h) = (640, 480);
    let at = small_row(w, h, 48);
    for (by, proved) in [(0u16, true), (7, true), (9, false), (12, false)] {
        let pixels: Vec<u8> = (0..w * h).flat_map(|_| [0u8, 120, 60]).collect();
        let mut raster = Raster::from_u8(w, h, Layout::Rgb8, &pixels).unwrap();
        composite(&mut raster, &drawn(small), at, v1.logo);
        let mut samples = raster.samples().to_vec();
        for y in at.y..at.y + at.height {
            for x in at.x..at.x + at.width {
                let i = ((y * w + x) * 3) as usize;
                samples[i] = samples[i].saturating_sub(by);
            }
        }
        let raster = Raster::new(w, h, Layout::Rgb8, samples).unwrap();
        let exam = examine(&raster, shipped, &options());
        assert_eq!(exam.findings.len(), 1, "{by}: {:#?}", exam.findings);
        let f = &exam.findings[0];
        let scores = f.scores.unwrap();
        assert!((scores.gain - 1.0).abs() <= 0.06, "{by}: {scores:?}");
        assert!(scores.edge_ratio <= 0.3, "{by}: {scores:?}");
        if proved {
            assert!(f.verified().is_some(), "{by}: {:?}", f.verdict);
        } else {
            assert!(
                matches!(f.verdict, Verdict::Refused(Refusal::OutOfRange { .. })),
                "{by}: {:?}",
                f.verdict
            );
        }
    }
}

/// The small row's own rectangle in the `W × H` test picture.
fn small_row_rect() -> SubRect {
    let row = small_row(W, H, 48);
    SubRect {
        x: row.x as f32,
        y: row.y as f32,
        size: 48.0,
    }
}

/// E12-R4 §4.2's hook: a mark a quarter of a pixel right of its row, over
/// a textured picture. Asked from the row's own rectangle, the search's
/// refinement — by the residual the second proof leaves, never by NCC —
/// lands on the mark within an eighth of a pixel, and the residual there
/// is under nine tenths of the row's (`forced_search`'s `gain_ratio`).
#[test]
fn refine_at_finds_a_mark_moved_by_a_quarter_pixel() {
    let catalogue = synthetic_catalogue();
    let v1 = synthetic_v1();
    let base = small_row_rect();
    for (kind, seed) in [(Kind::Fractal, 41u64), (Kind::ValueNoise, 42)] {
        let mut raster = picture(kind, W, H, seed, Layout::Rgb8);
        let mark = quantised(&resampled(&v1.small, 48.0, 0.25, 0.0).unwrap());
        let row = small_row(W, H, 48);
        let at = PixelRect {
            width: mark.width(),
            height: mark.height(),
            ..row
        };
        composite(&mut raster, &mark, at, [255.0; 3]);
        let r = refine_at(&raster, &catalogue, "test-sparkle-v1", base).expect("measured");
        let name = format!("{kind:?}: {r:?}");
        assert!((r.rect.x - (base.x + 0.25)).abs() <= 0.125, "{name}");
        assert!((r.rect.y - base.y).abs() <= 0.125, "{name}");
        assert!((r.rect.size - base.size).abs() <= 0.125, "{name}");
        assert!(r.residual_refined / r.residual_at < 0.9, "{name}");
        assert!(r.kept_by_margin, "{name}");
    }
}

/// The other half: a mark at its row as far as an eighth-pixel grid can
/// tell — drawn a sixteenth of a pixel (0.06) right of it, over a
/// gradient with a level of noise. A mark exactly at its row leaves the
/// row as the residual's own minimum, and then the margin is never asked;
/// here the sweeps do find a neighbour an eighth away with a little less
/// residual (by 1–8 %), and the margin keeps the row: what the search
/// would take is the row's own rectangle, never a shift under a tenth of
/// the residual (D236).
#[test]
fn refine_at_leaves_a_row_that_is_right() {
    let catalogue = synthetic_catalogue();
    let v1 = synthetic_v1();
    let base = small_row_rect();
    let row = small_row(W, H, 48);
    for seed in 0..3u64 {
        let clean = picture(Kind::Gradient, W, H, 60 + seed, Layout::Rgb8);
        let mut rng = Rng::new(600 + seed);
        let noisy: Vec<u16> = clean
            .samples()
            .iter()
            .map(|&v| (i32::from(v) + rng.below(3) as i32 - 1).clamp(0, 255) as u16)
            .collect();
        let mut raster = Raster::new(W, H, Layout::Rgb8, noisy).unwrap();
        let mark = quantised(&resampled(&v1.small, 48.0, 0.06, 0.0).unwrap());
        let at = PixelRect {
            width: mark.width(),
            height: mark.height(),
            ..row
        };
        composite(&mut raster, &mark, at, [255.0; 3]);
        let r = refine_at(&raster, &catalogue, "test-sparkle-v1", base).expect("measured");
        let name = format!("seed {seed}: {r:?}");
        // The sweeps did see a better neighbour: the margin is what is
        // being asked.
        assert!(r.rect != base, "{name}");
        assert!(r.residual_refined < r.residual_at, "{name}");
        let taken = if r.kept_by_margin { r.rect } else { base };
        assert!((taken.x - base.x).abs() < 0.125, "{name}");
        assert!((taken.y - base.y).abs() < 0.125, "{name}");
        assert!((taken.size - base.size).abs() < 0.125, "{name}");
    }
}
