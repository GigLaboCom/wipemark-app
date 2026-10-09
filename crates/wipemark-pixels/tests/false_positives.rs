//! D164: the false-positive gate, in three families.
//!
//! * **Negatives** — two thousand procedural pictures with no blend in
//!   them: textures, glyphs, flat and bright corners, noise, white
//!   corners, and opaque look-alikes (sparkles and diamonds drawn hard).
//!   Not one may be restored, **and not one may be reported** (D235): no
//!   blend is not a finding. Each is examined as a lossless and as a lossy
//!   source, whose out-of-range allowance is wider (D237).
//! * **Wallpapers** — procedural night skies (`aurora`): soft bright
//!   ridges and stars, what a sparkle correlates with. Neither restored nor
//!   reported.
//! * **Look-alike blends** — a thousand white blends that are not the
//!   mark: the sparkle blurred, a half-transparent diamond, and the mark's
//!   own shape at 0.8 or 1.2 of its opacity. A blend is there, so one may
//!   be *reported* (seen, not proved); not one may be restored. The gain
//!   family is what keeps the gain tolerance honest: ten times wider and
//!   it restores them.
//!
//! The counts are printed (`--nocapture`) and recorded in the report.

mod support;

use support::*;
use wipemark_pixels::{
    clean, composite, AlphaMap, Catalogue, ExamineOptions, Fidelity, Layout, PixelRect,
    PixelReport, Raster, Refusal, Verdict,
};

const SIDE: u32 = 96;

/// A diamond `size × size` at `peak`: the right place, the wrong shape.
fn diamond(size: u32, peak: f32) -> AlphaMap {
    let half = size as f32 / 2.0;
    let values = (0..size * size)
        .map(|i| {
            let (x, y) = (
                (i % size) as f32 + 0.5 - half,
                (i / size) as f32 + 0.5 - half,
            );
            if x.abs() + y.abs() <= half * 0.9 {
                peak
            } else {
                0.0
            }
        })
        .collect();
    AlphaMap::new(size, size, values).unwrap()
}

/// A random square of `size` inside the picture.
fn somewhere(rng: &mut Rng, size: u32) -> PixelRect {
    PixelRect {
        x: rng.below(SIDE - size),
        y: rng.below(SIDE - size),
        width: size,
        height: size,
    }
}

/// The `n`-th negative: nothing blended into it.
fn negative(n: u64) -> (String, Raster) {
    let mut rng = Rng::new(n.wrapping_mul(7919) + 3);
    let kind = KINDS[(n % KINDS.len() as u64) as usize];
    let mut raster = picture(kind, SIDE, SIDE, n + 10_000, Layout::Rgb8);
    let size = 24 + rng.below(40);
    let at = somewhere(&mut rng, size);
    let what = match n % 6 {
        // The bare texture.
        0..=2 => "texture",
        // An opaque white sparkle.
        3 => {
            stamp_opaque(&mut raster, &sparkle(size, 0.5), at, 0.25, 255.0);
            "opaque sparkle"
        }
        // An opaque diamond.
        4 => {
            stamp_opaque(
                &mut raster,
                &diamond(size, 0.5),
                at,
                0.25,
                rng.range(180.0, 255.0),
            );
            "opaque diamond"
        }
        // A white corner.
        _ => {
            let samples: Vec<u16> = raster
                .samples()
                .iter()
                .enumerate()
                .map(|(i, &s)| {
                    let p = (i / 3) as u32;
                    if p % SIDE > SIDE / 2 && p / SIDE > SIDE / 2 {
                        250
                    } else {
                        s
                    }
                })
                .collect();
            raster = Raster::new(SIDE, SIDE, Layout::Rgb8, samples).unwrap();
            "white corner"
        }
    };
    (format!("#{n} {kind:?} {what}"), raster)
}

/// The `n`-th look-alike blend, and the gain it was stamped at when it is
/// the mark's own shape.
fn lookalike(n: u64) -> (String, Raster, Option<f32>) {
    let mut rng = Rng::new(n.wrapping_mul(104_729) + 11);
    let kind = KINDS[(n % KINDS.len() as u64) as usize];
    let mut raster = picture(kind, SIDE, SIDE, n + 20_000, Layout::Rgb8);
    let mark = synthetic_v1().small;
    let (what, gain) = match n % 4 {
        // A blurred sparkle at an opacity no profile has.
        0 => {
            let size = 24 + rng.below(40);
            let peak = rng.range(0.75, 1.0);
            let at = somewhere(&mut rng, size);
            composite(
                &mut raster,
                &blurred(&sparkle(size, peak), 2),
                at,
                [255.0; 3],
            );
            ("blurred sparkle", None)
        }
        // A diamond, half transparent.
        1 => {
            let size = 24 + rng.below(40);
            let at = somewhere(&mut rng, size);
            composite(
                &mut raster,
                &diamond(size, rng.range(0.3, 0.6)),
                at,
                [255.0; 3],
            );
            ("diamond", None)
        }
        // The mark's own map at 0.8 or 1.2 of its opacity, at its row.
        _ => {
            let gain = if n % 4 == 2 { 0.8 } else { 1.2 };
            composite(
                &mut raster,
                &scaled(&mark, gain),
                small_row(SIDE, SIDE, 48),
                [255.0; 3],
            );
            ("the mark at another gain", Some(gain))
        }
    };
    (format!("#{n} {kind:?} {what}"), raster, gain)
}

fn both_sources() -> [ExamineOptions; 2] {
    [
        ExamineOptions::default(),
        ExamineOptions {
            source: Fidelity::Lossy,
            profiles: None,
        },
    ]
}

/// Clean `original` under `options`; nothing restored and nothing moved.
fn never_restored(
    name: &str,
    original: &Raster,
    catalogue: &Catalogue,
    options: &ExamineOptions,
) -> PixelReport {
    let mut raster = original.clone();
    let report = clean(&mut raster, catalogue, options);
    assert!(
        report.restored.is_empty(),
        "{name} ({:?}) was restored: {:#?}",
        options.source,
        report.found
    );
    assert_eq!(raster, *original, "{name}");
    report
}

#[test]
fn no_procedural_negative_is_ever_restored() {
    let catalogue = synthetic_catalogue();
    let mut dismissed = 0usize;
    for n in 0..2000u64 {
        let (name, original) = negative(n);
        for options in both_sources() {
            let report = never_restored(&name, &original, &catalogue, &options);
            assert!(
                report.found.is_empty(),
                "{name} ({:?}) was reported: {:#?}",
                options.source,
                report.found
            );
            dismissed += report.dismissed;
        }
    }
    // Not vacuous: the correlation did propose, and the proof dismissed.
    assert!(dismissed > 500, "only {dismissed} proposals were made");
    println!(
        "false positives: 2000 negatives x {} profiles x 2 sources: {dismissed} proposals, \
         every one no blend; 0 reported, 0 restored",
        catalogue.profiles().len()
    );
}

#[test]
fn no_night_sky_wallpaper_is_reported() {
    let catalogue = synthetic_catalogue();
    let shipped = Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"));
    let mut dismissed = 0usize;
    let mut pictures = 0usize;
    for seed in 0..40u64 {
        for (w, h) in [(320, 240), (640, 360), (360, 640)] {
            let original = aurora(w, h, seed, Layout::Rgb8);
            for cat in [&catalogue, shipped] {
                for options in both_sources() {
                    let name = format!("aurora #{seed} {w}x{h}");
                    let report = never_restored(&name, &original, cat, &options);
                    assert!(
                        report.found.is_empty(),
                        "{name} ({:?}) was reported: {:#?}",
                        options.source,
                        report.found
                    );
                    dismissed += report.dismissed;
                }
            }
            pictures += 1;
        }
    }
    assert!(dismissed > 0, "no wallpaper was even proposed");
    println!(
        "wallpapers: {pictures} night skies x 2 catalogues x 2 sources: {dismissed} proposals, \
         every one no blend; 0 reported, 0 restored"
    );
}

#[test]
fn no_lookalike_blend_is_ever_restored() {
    let catalogue = synthetic_catalogue();
    let (mut reported, mut total) = (0usize, 0usize);
    let (mut gained, mut by_gain) = (0usize, 0usize);
    let mut min_ratio = f32::INFINITY;
    for n in 0..1000u64 {
        let (name, original, gain) = lookalike(n);
        for options in both_sources() {
            let report = never_restored(&name, &original, &catalogue, &options);
            total += 1;
            reported += usize::from(!report.found.is_empty());
            for f in &report.found {
                if let Some(s) = f.scores {
                    min_ratio = min_ratio.min(s.edge_ratio);
                }
            }
            // The mark's own shape at another gain is a blend: where it is
            // refused by its gain — under the profile's own finding, or
            // under the other's that won the place — `k*` is on the side of
            // 1 it was stamped (on near-white noise the minimum is shallow
            // and lands nearer 1 than the gain).
            if let Some(g) = gain {
                gained += 1;
                let refusal = report
                    .found
                    .iter()
                    .flat_map(|f| {
                        let own = match f.verdict {
                            Verdict::Refused(r) => Some(r),
                            Verdict::Verified(_) => None,
                        };
                        std::iter::once((f.profile.as_str(), own))
                            .chain(f.also_tried.iter().map(|t| (t.profile.as_str(), t.refusal)))
                    })
                    .find(|(p, _)| *p == "test-sparkle-v1")
                    .and_then(|(_, r)| r);
                // Unseen, or refused by another proof (on near-white noise
                // the contour is too faint for the edges), is not a restoration
                // either; only the gain's refusal is counted.
                if let Some(Refusal::Gain { k }) = refusal {
                    assert!((k - 1.0) * (g - 1.0) > 0.0, "{name}: k* = {k}");
                    by_gain += 1;
                }
            }
        }
    }
    // Enough of them that a tolerance ten times wider restores some.
    assert!(
        by_gain * 10 >= gained * 8,
        "only {by_gain} of {gained} marks at another gain were refused by it"
    );
    println!(
        "look-alike blends: 1000 x 2 sources: {reported} of {total} reported (seen, not \
         proved), lowest edge ratio among them {min_ratio:.3}; {by_gain} of {gained} marks \
         at 0.8 or 1.2 refused by their gain; 0 restored"
    );
}

/// The planar path's proof (D306) did not loosen into restoring what the
/// RGB path refused: three hundred look-alike blends and three hundred
/// negatives, each stored as a 4:2:0 JPEG at 90 (`synth::jpeg_planes`) and
/// examined with those planes, are never restored, and the planes change
/// no finding into a non-finding or back — only, at most, which proof
/// refused. The refusals are counted both ways (`--nocapture`). Measured
/// over all 1000 of each (2026-10-09): 567 refused by gain and 151 by
/// edges on both paths, none by the range on either — on these families
/// the range term decides nothing, and this gate holds the planes to
/// "never restore", not to the range.
#[test]
fn no_negative_or_lookalike_is_restored_from_subsampled_planes() {
    use wipemark_pixels::synth::jpeg_planes;
    use wipemark_pixels::{clean_with, Sampling};
    let catalogue = synthetic_catalogue();
    let lossy = ExamineOptions {
        source: Fidelity::Lossy,
        profiles: None,
    };
    let why = |report: &PixelReport| -> Vec<&'static str> {
        report
            .found
            .iter()
            .map(|f| match f.verdict {
                Verdict::Verified(_) => "verified",
                Verdict::Refused(Refusal::Gain { .. }) => "gain",
                Verdict::Refused(Refusal::Edges { .. }) => "edges",
                Verdict::Refused(Refusal::OutOfRange { .. }) => "out-of-range",
                Verdict::Refused(_) => "other",
            })
            .collect()
    };
    let mut tally: std::collections::BTreeMap<(&str, &str), usize> = Default::default();
    let pictures = (0..300u64)
        .map(|n| {
            let (name, raster, _) = lookalike(n);
            (name, raster)
        })
        .chain((0..300u64).map(negative));
    for (name, original) in pictures {
        let (planes, raster) = jpeg_planes(&original, Sampling::H420, 90).unwrap();
        let mut rgb = raster.clone();
        let old = clean(&mut rgb, &catalogue, &lossy);
        let mut planar = raster.clone();
        let new = clean_with(&mut planar, Some(&planes), &catalogue, &lossy);
        assert!(
            new.restored.is_empty(),
            "{name} was restored from its planes: {:#?}",
            new.found
        );
        assert_eq!(planar, raster, "{name}");
        assert_eq!(old.found.len(), new.found.len(), "{name}");
        for (a, b) in why(&old).into_iter().zip(why(&new)) {
            *tally.entry((a, b)).or_default() += 1;
        }
    }
    println!("4:2:0 q90, 300 look-alikes and 300 negatives: (RGB, planes) -> findings {tally:?}");
}
