//! Restoration: a composited mark comes back off to within a level, only
//! where it was proved, never through the alpha channel, never through a
//! hole — and "exact" is claimed only where it can be.

mod support;

use support::*;
use wipemark_pixels::synth::{composite_with, Blend};
use wipemark_pixels::{
    clean, composite, drawn, examine, resampled, restore, restore_off_by, AlphaMap, Catalogue,
    ExamineOptions, Fidelity, Kernel, Layout, PixelRect, Placed, Raster, Refusal, RestoreError,
    SubRect, Verdict, OUTLINE_BOUND, STEP_LEVELS,
};

const W: u32 = 320;
const H: u32 = 240;

fn lossless() -> ExamineOptions {
    ExamineOptions::default()
}

/// Every procedural background, both 8-bit layouts: the small mark at
/// its row's place is found by the row, verified, restored to within one
/// level everywhere, and the restoration says it is exact.
#[test]
fn a_composited_mark_comes_back_within_one_level() {
    let catalogue = synthetic_catalogue();
    let v1 = synthetic_v1();
    let at = small_row(W, H, 48);
    for layout in [Layout::Rgb8, Layout::Rgba8] {
        for (name, original) in backgrounds(W, H, layout) {
            let mut marked = original.clone();
            composite(&mut marked, &v1.small, at, [255.0; 3]);
            assert!(
                max_error(&marked, &original) > 0,
                "{name}: nothing was stamped"
            );
            let report = clean(&mut marked, &catalogue, &lossless());
            let verified: Vec<_> = report
                .found
                .iter()
                .filter(|f| f.verified().is_some())
                .collect();
            assert_eq!(verified.len(), 1, "{name} {layout:?}: {:#?}", report.found);
            assert_eq!(verified[0].profile, v1.id, "{name}");
            assert_eq!(verified[0].placed, Placed::Row(1), "{name}");
            assert_eq!(report.restored.len(), 1, "{name}");
            assert!(report.restored[0].exact, "{name}: {:?}", report.restored[0]);
            let error = max_error(&marked, &original);
            assert!(error <= 1, "{name} {layout:?}: off by {error}");
        }
    }
    // And the large row, on a picture past 1024 in both directions.
    let (w, h) = (1100, 1050);
    let original = picture(Kind::Fractal, w, h, 7, Layout::Rgb8);
    let mut marked = original.clone();
    let large = PixelRect {
        x: w - 64 - 96,
        y: h - 64 - 96,
        width: 96,
        height: 96,
    };
    composite(&mut marked, &v1.large, large, [255.0; 3]);
    let report = clean(&mut marked, &catalogue, &lossless());
    assert_eq!(report.restored.len(), 1, "{:#?}", report.found);
    assert!(report.restored[0].exact);
    assert!(max_error(&marked, &original) <= 1);
}

/// A mark a pixel off its row: the row's own rectangle does not prove it
/// (D236 never moves a row), so the search runs — it runs whenever no
/// row's mark was proved, not only when no row was proposed — finds the
/// mark where it is, proves and restores it; searched, so never exact.
#[test]
fn a_mark_a_pixel_off_its_row_is_found_by_the_search() {
    let catalogue = synthetic_catalogue();
    let row = small_row(W, H, 48);
    for kind in [Kind::Gradient, Kind::Fractal, Kind::Flat] {
        let original = picture(kind, W, H, 5, Layout::Rgb8);
        let mut marked = original.clone();
        let off = PixelRect {
            x: row.x + 1,
            y: row.y - 1,
            ..row
        };
        composite(&mut marked, &synthetic_v1().small, off, [255.0; 3]);
        let report = clean(&mut marked, &catalogue, &lossless());
        assert_eq!(report.restored.len(), 1, "{kind:?}: {:#?}", report.found);
        let f = report
            .found
            .iter()
            .find(|f| f.verified().is_some())
            .expect("proved");
        assert_eq!(f.placed, Placed::Searched, "{kind:?}");
        assert_eq!(f.pixels, Some(off), "{kind:?}");
        let r = &report.restored[0];
        // Searched at the map's own size and a whole-pixel offset: not
        // exact, and not resampled either — the map is drawn as captured.
        assert!(!r.exact && r.searched && !r.resampled, "{kind:?}: {r:?}");
        assert!(!report.marks_left(), "{kind:?}: {:#?}", report.found);
        assert!(max_error(&marked, &original) <= 1, "{kind:?}");
    }
}

/// A mark half a pixel off its row — what a picture resampled after it was
/// marked looks like. The row's own rectangle sees a blend it cannot prove
/// (its edges are left half-cancelled) and refuses it; because the row
/// *refused* rather than proposing nothing, the search still runs, refines
/// to the sub-pixel by the residual the second proof leaves, with the map
/// of the size it lands on, and proves and restores the mark where it is —
/// within a level. Where the row refuses it **by gain** (the half-cancelled
/// edges vanish at another `k`), this is also the bound of D470 from
/// the other side: the search does not prove a mark at the place of a row
/// that refused it by gain, and a quarter or half a pixel off is not that
/// place ([`wipemark_pixels::ROW_PLACE`]).
#[test]
fn a_mark_half_a_pixel_off_its_row_is_proved_by_the_search() {
    let catalogue = synthetic_catalogue();
    let row = small_row(W, H, 48);
    let v1 = synthetic_v1();
    for kind in [Kind::Gradient, Kind::Flat, Kind::ValueNoise] {
        for (fx, fy) in [(0.5f32, 0.0f32), (0.25, 0.25)] {
            let original = picture(kind, W, H, 5, Layout::Rgb8);
            let mut marked = original.clone();
            let mark = quantised(&resampled(&v1.small, 48.0, fx, fy).unwrap());
            let at = PixelRect {
                width: mark.width(),
                height: mark.height(),
                ..row
            };
            composite(&mut marked, &mark, at, [255.0; 3]);
            let report = clean(&mut marked, &catalogue, &lossless());
            let name = format!("{kind:?} +({fx}, {fy})");
            assert_eq!(report.restored.len(), 1, "{name}: {:#?}", report.found);
            let f = report
                .found
                .iter()
                .find(|f| f.verified().is_some())
                .expect("proved");
            assert_eq!(f.placed, Placed::Searched, "{name}");
            assert!(
                (f.rect.x - (row.x as f32 + fx)).abs() <= 0.125,
                "{name}: {:?}",
                f.rect
            );
            assert!(
                (f.rect.y - (row.y as f32 + fy)).abs() <= 0.125,
                "{name}: {:?}",
                f.rect
            );
            assert!((f.rect.size - 48.0).abs() <= 0.125, "{name}: {:?}", f.rect);
            assert!(!report.marks_left(), "{name}");
            // Found where it is, with the map it was drawn with: the
            // restoration is as close as a row's.
            let error = max_error(&marked, &original);
            assert!(error <= 1, "{name}: off by {error} at {:?}", f.rect);
        }
    }
}

/// The densest glyph sheet under the small mark: the blend is perfect
/// (`k*` = 1) but the strokes put more edge on the contour than the mark
/// does, so `E(1)/E(0)` stays over the threshold. Precision first: the
/// mark is seen, refused by its edges, left — never guessed at — and the
/// report says a mark is left.
#[test]
fn a_mark_drowned_in_strokes_is_seen_and_left() {
    let catalogue = synthetic_catalogue();
    let (kind, seed) = DROWNED;
    let original = picture(kind, W, H, background_seed(kind, seed), Layout::Rgb8);
    let mut marked = original.clone();
    composite(
        &mut marked,
        &synthetic_v1().small,
        small_row(W, H, 48),
        [255.0; 3],
    );
    let before = marked.clone();
    let report = clean(&mut marked, &catalogue, &lossless());
    assert!(report.restored.is_empty(), "{:#?}", report.found);
    assert_eq!(marked, before);
    let f = report.found.first().expect("the mark was not seen");
    assert_eq!(f.profile, "test-sparkle-v1");
    assert_eq!(f.placed, Placed::Row(1));
    assert!(
        matches!(f.verdict, Verdict::Refused(Refusal::Edges { .. })),
        "{:?}",
        f.verdict
    );
    let k = f.scores.expect("measured").gain;
    assert!((k - 1.0).abs() <= 0.02, "k* = {k}");
    assert!(report.marks_left());
}

/// A proof belongs to the raster it was made on: a `Verified` put to a
/// raster of another size is `RestoreError::Elsewhere`, and nothing moves
/// (the verifier's V12).
#[test]
fn a_proof_is_not_restored_onto_a_raster_of_another_size() {
    let catalogue = synthetic_catalogue();
    let mut marked = picture(Kind::Gradient, W, H, 11, Layout::Rgb8);
    composite(
        &mut marked,
        &synthetic_v1().small,
        small_row(W, H, 48),
        [255.0; 3],
    );
    let exam = examine(&marked, &catalogue, &lossless());
    let verified = exam
        .findings
        .iter()
        .find_map(|f| f.verified())
        .expect("proved");
    for (w, h) in [(W + 1, H), (W, H + 1)] {
        let mut other = picture(Kind::Gradient, w, h, 11, Layout::Rgb8);
        let before = other.clone();
        assert_eq!(
            restore(&mut other, verified, &lossless()),
            Err(RestoreError::Elsewhere),
            "{w}x{h}"
        );
        assert_eq!(other, before);
    }
    // The raster it was made on takes it.
    assert!(restore(&mut marked, verified, &lossless()).is_ok());
}

/// A mark drawn with a map a little wider than the one held — a halo
/// outside its edge, what a vendor's own anti-aliasing or a filter adds:
/// the gain still lands on 1, the second proof accepts it, and the
/// restoration takes most of the mark away — but leaves a ring along its
/// edge. The outline check measures it over the bound (D238), says an
/// outline is left, and the mark counts as still in the result.
#[test]
fn an_outline_left_by_another_map_is_said() {
    let catalogue = synthetic_catalogue();
    let small = synthetic_v1().small;
    let wide = blurred(&small, 2);
    let halo = AlphaMap::new(
        48,
        48,
        small
            .values()
            .iter()
            .zip(wide.values())
            .map(|(a, b)| a + 1.5 * (b - a).max(0.0))
            .collect(),
    )
    .unwrap();
    for kind in [Kind::Flat, Kind::Gradient] {
        let mut marked = picture(kind, W, H, 5, Layout::Rgb8);
        composite(
            &mut marked,
            &quantised(&halo),
            small_row(W, H, 48),
            [255.0; 3],
        );
        let report = clean(&mut marked, &catalogue, &lossless());
        assert_eq!(report.restored.len(), 1, "{kind:?}: {:#?}", report.found);
        let r = &report.restored[0];
        assert!(r.outline > OUTLINE_BOUND, "{kind:?}: {}", r.outline);
        assert!(r.outline_left && !r.exact, "{kind:?}: {r:?}");
        assert!(report.marks_left(), "{kind:?}");
    }
}

/// D157: the picture's alpha is never written, whatever the restoration
/// does to the colour.
#[test]
fn the_alpha_channel_is_never_written() {
    let catalogue = synthetic_catalogue();
    let original = picture(Kind::ValueNoise, W, H, 3, Layout::Rgba8);
    let mut marked = original.clone();
    composite(
        &mut marked,
        &synthetic_v1().small,
        small_row(W, H, 48),
        [255.0; 3],
    );
    let report = clean(&mut marked, &catalogue, &lossless());
    assert_eq!(report.restored.len(), 1, "{:#?}", report.found);
    let alpha = |r: &wipemark_pixels::Raster| -> Vec<u16> {
        r.samples().chunks_exact(4).map(|p| p[3]).collect()
    };
    assert_eq!(alpha(&marked), alpha(&original));
}

/// A mark over pixels that are not opaque: what the blend meant there is
/// unknown, so it is refused and nothing is written.
#[test]
fn a_transparent_region_is_refused() {
    let catalogue = synthetic_catalogue();
    let at = small_row(W, H, 48);
    let base = picture(Kind::Gradient, W, H, 4, Layout::Rgba8);
    let mut samples = base.samples().to_vec();
    for y in at.y..at.y + at.height {
        for x in at.x..at.x + at.width {
            samples[((y * W + x) * 4 + 3) as usize] = 200;
        }
    }
    let mut marked = wipemark_pixels::Raster::new(W, H, Layout::Rgba8, samples).unwrap();
    composite(&mut marked, &synthetic_v1().small, at, [255.0; 3]);
    let before = marked.clone();
    let report = clean(&mut marked, &catalogue, &lossless());
    assert!(!report.found.is_empty());
    assert!(report
        .found
        .iter()
        .all(|f| f.verdict == Verdict::Refused(Refusal::Transparent)));
    assert!(report.restored.is_empty());
    assert_eq!(marked, before);
}

/// D155: a map that reaches 1.0 leaves holes — untouched, counted — and
/// a restoration with holes is never exact.
#[test]
fn opaque_pixels_are_holes_never_divided() {
    let s = Synthetic {
        id: "test-opaque",
        small: opaque_core(48),
        large: opaque_core(96),
    };
    let mut assets = Vec::new();
    let row = profile_json(&s, &mut assets, 32);
    let catalogue = parse_with(&[row], &assets).unwrap();
    let original = picture(Kind::Flat, W, H, 5, Layout::Rgb8);
    let at = small_row(W, H, 48);
    let mut marked = original.clone();
    composite(&mut marked, &s.small, at, [255.0; 3]);
    let stamped = marked.clone();
    let report = clean(&mut marked, &catalogue, &lossless());
    assert_eq!(report.restored.len(), 1, "{:#?}", report.found);
    let r = &report.restored[0];
    let holes = s.small.values().iter().filter(|&&a| a >= 0.95).count() as u32;
    assert!(holes > 0);
    assert_eq!(r.holes, holes);
    assert!(!r.exact);
    assert!(report.marks_left());
    // A hole is never divided, so never measured: its three samples are
    // counted apart (D305), and what was restored around it is consistent.
    assert_eq!(r.consistency_excluded, r.clamped + 3 * holes, "{r:?}");
    assert!(r.consistency_px <= 1.0, "{r:?}");
    // Every hole is the stamped value still; every other pixel is back.
    for y in 0..48 {
        for x in 0..48 {
            let i = (((at.y + y) * W + at.x + x) * 3) as usize;
            let a = s.small.get(i64::from(x), i64::from(y));
            let (m, o, st) = (marked.samples(), original.samples(), stamped.samples());
            for c in 0..3 {
                if a >= 0.95 {
                    assert_eq!(m[i + c], st[i + c], "a hole was written at ({x}, {y})");
                } else {
                    assert!(m[i + c].abs_diff(o[i + c]) <= 1, "({x}, {y})");
                }
            }
        }
    }
}

/// A row that resamples its map can verify and restore, and is never
/// exact: the map is not the vendor's own at that size.
#[test]
fn a_resampled_row_is_never_exact() {
    let s = synthetic_v1();
    let at = PixelRect {
        x: W - 40 - 40,
        y: H - 40 - 40,
        width: 40,
        height: 40,
    };
    let row = format!(
        r#"{{ "when": {{}}, "rect": [{}, {}, 40, 40], "alpha": "large", "resample": true }}"#,
        at.x, at.y
    );
    let mut assets = Vec::new();
    let catalogue = parse_with(&[profile_with(&s, &mut assets, &row, 0.95)], &assets).unwrap();
    let original = picture(Kind::Fractal, W, H, 9, Layout::Rgb8);
    let mut marked = original.clone();
    let map = resampled(&s.large, 40.0, 0.0, 0.0).unwrap();
    composite(&mut marked, &map, at, [255.0; 3]);
    let report = clean(&mut marked, &catalogue, &lossless());
    assert_eq!(report.restored.len(), 1, "{:#?}", report.found);
    let r = &report.restored[0];
    assert!(!r.exact && r.resampled && !r.searched, "{r:?}");
    assert!(max_error(&marked, &original) <= 1);
}

/// A lossy source is never exact, however clean the arithmetic.
#[test]
fn a_lossy_source_is_never_exact() {
    let catalogue = synthetic_catalogue();
    let original = picture(Kind::Gradient, W, H, 11, Layout::Rgb8);
    let mut marked = original.clone();
    composite(
        &mut marked,
        &synthetic_v1().small,
        small_row(W, H, 48),
        [255.0; 3],
    );
    let options = ExamineOptions {
        source: Fidelity::Lossy,
        profiles: None,
    };
    let report = clean(&mut marked, &catalogue, &options);
    assert_eq!(report.restored.len(), 1, "{:#?}", report.found);
    assert!(!report.restored[0].exact);
}

/// Nothing verified, nothing written: an unmarked picture comes out of
/// `clean` identical.
#[test]
fn nothing_is_changed_when_nothing_is_verified() {
    let catalogue = synthetic_catalogue();
    for layout in [Layout::Rgb8, Layout::Rgba8] {
        for (name, original) in backgrounds(W, H, layout) {
            let mut raster = original.clone();
            let report = clean(&mut raster, &catalogue, &lossless());
            assert!(report.restored.is_empty(), "{name}: {:#?}", report.found);
            assert_eq!(raster, original, "{name}");
        }
    }
}

/// A white mark over near-white noise leaves no contour to prove: it is
/// not restored. Refusing is right; nothing is damaged.
#[test]
fn a_white_mark_on_white_noise_is_not_proved() {
    let catalogue = synthetic_catalogue();
    let original = picture(Kind::Bright, W, H, 13, Layout::Rgb8);
    let mut marked = original.clone();
    composite(
        &mut marked,
        &synthetic_v1().small,
        small_row(W, H, 48),
        [255.0; 3],
    );
    let before = marked.clone();
    let report = clean(&mut marked, &catalogue, &lossless());
    // Either nothing correlated, or what did was refused — and whatever
    // was restored is the original to within a level.
    if report.restored.is_empty() {
        assert_eq!(marked, before);
    } else {
        assert!(max_error(&marked, &original) <= 1);
    }
}

/// The same, lopsided: a halo outside the edge on the mark's left half and
/// the edge eaten into on its right — a ring light on one side and dark on
/// the other. Its faint band averages out against the picture around it
/// (a step under half a level), so the absolute measure (D244) cannot see
/// it; the contour's share can (over a quarter), and the outline is said.
#[test]
fn a_lopsided_outline_is_said_by_its_share() {
    let catalogue = synthetic_catalogue();
    let small = synthetic_v1().small;
    let wide = blurred(&small, 2);
    let lopsided = AlphaMap::new(
        48,
        48,
        small
            .values()
            .iter()
            .zip(wide.values())
            .enumerate()
            .map(|(i, (a, b))| {
                if i % 48 < 24 {
                    a + 1.1 * (b - a).max(0.0)
                } else {
                    (a - 1.1 * (b - a).max(0.0)).max(0.0)
                }
            })
            .collect(),
    )
    .unwrap();
    let mut marked = picture(Kind::Gradient, W, H, 5, Layout::Rgb8);
    composite(
        &mut marked,
        &quantised(&lopsided),
        small_row(W, H, 48),
        [255.0; 3],
    );
    let report = clean(&mut marked, &catalogue, &lossless());
    assert_eq!(report.restored.len(), 1, "{:#?}", report.found);
    let r = &report.restored[0];
    assert!(r.step.abs() < STEP_LEVELS, "{r:?}");
    assert!(r.outline > OUTLINE_BOUND, "{r:?}");
    assert!(r.outline_left && report.marks_left(), "{r:?}");
}

/// The bench's composite (D312) at its defaults — the blend in code
/// values, one logo colour, `k = 1`, no bias, rounding half away from zero,
/// a whole-pixel rectangle at the map's own size, the area kernel — is
/// `composite`, sample for sample: every shipped map, a random raster in
/// three layouts, a place inside it and one hanging off its edge.
#[test]
fn composite_with_at_its_defaults_is_composite() {
    let catalogue = Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"));
    let mut rng = Rng::new(312);
    for layout in [Layout::Rgb8, Layout::Rgba8, Layout::Rgb16] {
        let (w, h) = (200u32, 150u32);
        let samples = (0..w * h * layout.channels() as u32)
            .map(|_| rng.below(u32::from(layout.max()) + 1) as u16)
            .collect();
        let background = Raster::new(w, h, layout, samples).unwrap();
        for profile in catalogue.profiles() {
            for (id, map) in &profile.maps {
                for (x, y) in [(7u32, 11u32), (w - map.width() / 2, h - map.height() / 3)] {
                    let mut old = background.clone();
                    let at = PixelRect {
                        x,
                        y,
                        width: map.width(),
                        height: map.height(),
                    };
                    composite(&mut old, map, at, profile.logo);
                    let mut new = background.clone();
                    let rect = SubRect {
                        x: x as f32,
                        y: y as f32,
                        size: map.width() as f32,
                    };
                    composite_with(
                        &mut new,
                        map,
                        rect,
                        Kernel::Area,
                        &Blend::encoded(profile.logo),
                    );
                    assert!(old != background, "{id} {layout:?}: nothing was drawn");
                    assert!(old == new, "{id} at ({x}, {y}) {layout:?}");
                }
            }
        }
    }
}

/// The bench's self-test is real: a shipped map that is not fitted, drawn
/// as the vendor draws it (`drawn`, D241) with the bench's composite at a
/// row's own place and size over a lossless picture, is proved by that row
/// and restored by the template alone — no capture noise to take back off
/// (D246) — to within a level of the picture under it, and `exact` says so
/// wherever nothing clamped. Nothing outside the mark moves.
#[test]
fn a_canonical_composite_comes_back_exact() {
    let catalogue = Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"));
    // (profile, map, picture size, margin): V1's small row, V2's 1024 row
    // and V2's large row — the canonical rows of the shipped catalogue.
    let rows = [
        ("gemini-sparkle-v1", "gemini-v1-48", 1024u32, 32u32),
        ("gemini-sparkle-v2", "gemini-v2-36", 1024, 71),
        ("gemini-sparkle-v2", "gemini-v2-96", 2048, 192),
    ];
    let mut exact = 0;
    for (profile, map_id, side, margin) in rows {
        let p = catalogue.profile(profile).unwrap();
        let (_, map) = p.maps.iter().find(|(id, _)| id == map_id).unwrap();
        let at = PixelRect {
            x: side - margin - map.width(),
            y: side - margin - map.height(),
            width: map.width(),
            height: map.height(),
        };
        let rect = SubRect {
            x: at.x as f32,
            y: at.y as f32,
            size: map.width() as f32,
        };
        let kinds: &[Kind] = if side > 1024 {
            &[Kind::Gradient, Kind::Fractal]
        } else {
            &[Kind::Gradient, Kind::ValueNoise, Kind::Fractal, Kind::Flat]
        };
        for &kind in kinds {
            let name = format!("{map_id} over {kind:?}");
            let original = picture(kind, side, side, 5, Layout::Rgb8);
            let mut marked = original.clone();
            composite_with(
                &mut marked,
                &drawn(map),
                rect,
                Kernel::Area,
                &Blend::encoded(p.logo),
            );
            let report = clean(&mut marked, catalogue, &lossless());
            assert_eq!(report.restored.len(), 1, "{name}: {:#?}", report.found);
            let r = &report.restored[0];
            assert_eq!(r.profile, profile, "{name}");
            assert_eq!(r.rect, at, "{name}");
            assert!(!r.noise, "{name}: the capture's noise was found drawn");
            assert_eq!(r.exact, r.clamped == 0, "{name}: {r:?}");
            exact += usize::from(r.exact);
            assert!(!report.marks_left(), "{name}: {r:?}");
            assert!(max_error(&marked, &original) <= 1, "{name}");
            for (i, (m, o)) in marked
                .samples()
                .chunks_exact(3)
                .zip(original.samples().chunks_exact(3))
                .enumerate()
            {
                let (x, y) = (i as u32 % side, i as u32 / side);
                let inside = x >= at.x && y >= at.y && x < at.x + at.width && y < at.y + at.height;
                assert!(inside || m == o, "{name}: ({x}, {y}) moved");
            }
        }
    }
    // Not vacuous: most of them are exact.
    assert!(exact >= 8, "{exact} exact");
}

// ------------------------------------------------- consistency (D305)

/// The identity of the inverse (E12-R7, D305): every restoration of a
/// composite — thirteen procedural pictures, both 8-bit layouts and a
/// 16-bit one — blended back with the map and the logo it was restored
/// with, lands within the rounding of the input it was restored from. The
/// measure is in 8-bit levels whatever the depth: the 257 stored levels a
/// 16-bit raster has to one 8-bit level are not a departure.
#[test]
fn a_composited_mark_is_consistent_to_rounding() {
    let catalogue = synthetic_catalogue();
    let v1 = synthetic_v1();
    let at = small_row(W, H, 48);
    let mut seen = 0;
    for layout in [Layout::Rgb8, Layout::Rgba8, Layout::Rgb16] {
        for (name, original) in backgrounds(W, H, layout) {
            let mut marked = original.clone();
            composite(&mut marked, &v1.small, at, [255.0; 3]);
            let report = clean(&mut marked, &catalogue, &lossless());
            for r in &report.restored {
                assert!(r.consistency_px <= 1.0, "{name} {layout:?}: {r:?}");
                assert_eq!(
                    r.consistency_excluded,
                    r.clamped + 3 * r.holes,
                    "{name} {layout:?}: {r:?}"
                );
                assert_eq!(r.consistency_dct, None, "{name}");
                seen += 1;
            }
        }
    }
    assert!(seen >= 36, "only {seen} restorations");
}

/// The measure's exclusion (D305): a mark over a saturated green whose red
/// sits a few levels under `α·L` — D240's real case, the vendor's α against
/// an 8-bit capture — inverts past 0 in red, and those samples are clamped.
/// An error there is expected: they are left out of the measure and
/// counted apart, every one of them, and what is left is consistent to
/// rounding. Counted in, a third of the samples would be levels off.
#[test]
fn clamped_samples_are_left_out_and_counted() {
    let shipped = Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"));
    let v1 = shipped.profile("gemini-sparkle-v1").unwrap();
    let (_, small) = v1.maps.iter().find(|(id, _)| id == "gemini-v1-48").unwrap();
    let (w, h) = (640, 480);
    let at = small_row(w, h, 48);
    let pixels: Vec<u8> = (0..w * h).flat_map(|_| [0u8, 150, 58]).collect();
    let mut raster = Raster::from_u8(w, h, Layout::Rgb8, &pixels).unwrap();
    composite(&mut raster, &drawn(small), at, v1.logo);
    let mut samples = raster.samples().to_vec();
    for y in at.y..at.y + at.height {
        for x in at.x..at.x + at.width {
            let i = ((y * w + x) * 3) as usize;
            samples[i] = samples[i].saturating_sub(5);
        }
    }
    let mut raster = Raster::new(w, h, Layout::Rgb8, samples).unwrap();
    let report = clean(&mut raster, shipped, &lossless());
    assert_eq!(report.restored.len(), 1, "{:#?}", report.found);
    let r = &report.restored[0];
    assert_eq!(r.holes, 0, "{r:?}");
    assert!(!r.noise, "{r:?}");
    // Not vacuous: most of the red under the mark clamped.
    assert!(r.clamped * 10 > r.changed, "{r:?}");
    assert_eq!(r.consistency_excluded, r.clamped, "{r:?}");
    assert!(r.consistency_px <= 1.0, "{r:?}");
}

/// The measure sees a departure from the data (D305): a restoration moved
/// two levels up inside the mark after the inverse — through the hidden
/// hook, the very restoration otherwise — blends back two levels, less
/// what the opacity takes, above the input it came from. Two 8-bit levels
/// on a 16-bit raster too, where they are 514 stored ones.
#[test]
fn a_restoration_off_by_one_level_is_seen() {
    let catalogue = synthetic_catalogue();
    let v1 = synthetic_v1();
    let at = small_row(W, H, 48);
    for (kind, seed, layout) in [
        (Kind::Gradient, 3u64, Layout::Rgb8),
        (Kind::Fractal, 4, Layout::Rgb8),
        (Kind::ValueNoise, 5, Layout::Rgb8),
        (Kind::Fractal, 4, Layout::Rgb16),
    ] {
        let mut raster = picture(kind, W, H, seed, layout);
        composite(&mut raster, &v1.small, at, [255.0; 3]);
        let exam = examine(&raster, &catalogue, &lossless());
        let verified = exam
            .findings
            .iter()
            .find_map(|f| f.verified())
            .unwrap_or_else(|| panic!("{kind:?} {layout:?}: {:#?}", exam.findings));
        let mut exact = raster.clone();
        let r0 = restore(&mut exact, verified, &lossless()).unwrap();
        assert!(r0.consistency_px <= 1.0, "{kind:?} {layout:?}: {r0:?}");
        let r2 = restore_off_by(&mut raster, verified, &lossless(), 2.0).unwrap();
        assert_eq!(r2.clamped, r0.clamped, "{kind:?} {layout:?}");
        assert!(
            (1.5..=2.5).contains(&r2.consistency_px),
            "{kind:?} {layout:?}: {r2:?}"
        );
    }
}
