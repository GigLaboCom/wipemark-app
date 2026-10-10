//! D164: the false-positive gate, in three families.
//!
//! * **Negatives** — 2331 procedural pictures with no blend of a mark in
//!   them: textures, glyphs, flat and bright corners, noise, white
//!   corners, opaque look-alikes (sparkles and diamonds drawn hard), and
//!   short white **words** in the bottom-right corner (E12-R11 §5, R2
//!   §4.2's look-alike text), opaque, opaque with a dark outline, or half
//!   transparent. Not one may be restored, **and not one may be reported**
//!   (D235): no blend is not a finding. The one exception is the
//!   half-transparent word, which *is* a white blend: it may be seen, and
//!   never proved. Each is examined as a lossless and as a lossy source,
//!   whose out-of-range allowance is wider (D237); the words also under
//!   the shipped profiles, the vendor maps text has to be told from.
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

/// The word family's pictures: a word is 30–110 px wide (R2 §4.2), which
/// the others' 96 cannot hold with a margin.
const WORD_SIDE: u32 = 160;

/// The negatives, seven slots of `n % 7`: three of bare texture, one each
/// of an opaque sparkle, an opaque diamond, a white corner and a word.
/// 2331 is 333 × 7, so the four families that were here before the words
/// keep the counts they had over 2000 in six slots (999 textures, 333 of
/// each other), and the words are 333 more.
const NEGATIVES: u64 = 2331;

/// How a word negative is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Word {
    /// White, hard-edged: no blend.
    Opaque,
    /// White, hard-edged, over a dark one-pixel outline: no blend.
    Outlined,
    /// White at a peak of 0.3–0.7: a blend, but not the mark's.
    Translucent,
}

/// The word the `n`-th negative carries, if it is one: the seventh slot,
/// its three styles in turn (111 each).
fn word_of(n: u64) -> Option<Word> {
    (n % 7 == 6).then_some(match (n / 7) % 3 {
        0 => Word::Opaque,
        1 => Word::Outlined,
        _ => Word::Translucent,
    })
}

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

/// The `n`-th negative: no mark blended into it (a half-transparent word
/// is a white blend, and not a mark's).
fn negative(n: u64) -> (String, Raster) {
    let mut rng = Rng::new(n.wrapping_mul(7919) + 3);
    let kind = KINDS[(n % KINDS.len() as u64) as usize];
    let side = if word_of(n).is_some() {
        WORD_SIDE
    } else {
        SIDE
    };
    let mut raster = picture(kind, side, side, n + 10_000, Layout::Rgb8);
    let size = 24 + rng.below(40);
    let at = somewhere(&mut rng, size);
    let what: String = match n % 7 {
        // The bare texture.
        0..=2 => "texture".into(),
        // An opaque white sparkle.
        3 => {
            stamp_opaque(&mut raster, &sparkle(size, 0.5), at, 0.25, 255.0);
            "opaque sparkle".into()
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
            "opaque diamond".into()
        }
        // A short white word in the bottom-right corner: two to five
        // letters, 30–110 px wide, 4–24 px from the edges.
        6 => {
            let style = word_of(n).expect("the seventh slot is the word family");
            let letters: Vec<usize> = (0..2 + rng.below(4))
                .map(|_| rng.below(LETTERS.len() as u32) as usize)
                .collect();
            let width = 30 + rng.below(81);
            let peak = match style {
                Word::Translucent => rng.range(0.3, 0.7),
                Word::Opaque | Word::Outlined => 1.0,
            };
            let map = word(&letters, width, peak);
            let (mx, my) = (4 + rng.below(21), 4 + rng.below(21));
            let at = PixelRect {
                x: WORD_SIDE - mx - map.width(),
                y: WORD_SIDE - my - map.height(),
                width: map.width(),
                height: map.height(),
            };
            match style {
                Word::Opaque => stamp_opaque(&mut raster, &map, at, 0.5, 255.0),
                Word::Outlined => {
                    let dark = rng.range(0.0, 60.0);
                    stamp_opaque(&mut raster, &blurred(&map, 1), at, 0.05, dark);
                    stamp_opaque(&mut raster, &map, at, 0.5, 255.0);
                }
                Word::Translucent => composite(&mut raster, &map, at, [255.0; 3]),
            }
            let text: String = letters.iter().map(|&i| LETTERS[i].0).collect();
            format!("{style:?} word {text} {width} px")
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
            "white corner".into()
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
    let shipped = Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"));
    let mut dismissed = 0usize;
    let mut words = std::collections::BTreeMap::<&str, usize>::new();
    let (mut word_dismissed, mut seen, mut translucent) = (0usize, 0usize, 0usize);
    for n in 0..NEGATIVES {
        let (name, original) = negative(n);
        let style = word_of(n);
        let mut catalogues = vec![&catalogue];
        if let Some(style) = style {
            // Not vacuous: the word is there. Dropping the words leaves
            // the bare texture, and this is what notices.
            let kind = KINDS[(n % KINDS.len() as u64) as usize];
            let bare = picture(kind, WORD_SIDE, WORD_SIDE, n + 10_000, Layout::Rgb8);
            let drawn = bare
                .samples()
                .chunks_exact(3)
                .zip(original.samples().chunks_exact(3))
                .filter(|(a, b)| a != b)
                .count();
            assert!(
                drawn >= 20,
                "{name}: only {drawn} pixels of a word were drawn"
            );
            *words.entry(word_style(style)).or_default() += 1;
            catalogues.push(shipped);
        }
        for (c, cat) in catalogues.into_iter().enumerate() {
            for options in both_sources() {
                let report = never_restored(&name, &original, cat, &options);
                if style == Some(Word::Translucent) {
                    // A half-transparent white word is a blend: seen is
                    // allowed (D235), proved never.
                    assert!(
                        report
                            .found
                            .iter()
                            .all(|f| matches!(f.verdict, Verdict::Refused(_))),
                        "{name} ({:?}) was proved a mark: {:#?}",
                        options.source,
                        report.found
                    );
                    translucent += 1;
                    seen += usize::from(!report.found.is_empty());
                } else {
                    assert!(
                        report.found.is_empty(),
                        "{name} ({:?}) was reported: {:#?}",
                        options.source,
                        report.found
                    );
                }
                if c == 0 {
                    dismissed += report.dismissed;
                }
                if style.is_some() {
                    word_dismissed += report.dismissed;
                }
            }
        }
    }
    // Not vacuous: the correlation did propose, and the proof dismissed.
    assert!(dismissed > 500, "only {dismissed} proposals were made");
    assert_eq!(
        words.values().sum::<usize>() as u64,
        NEGATIVES / 7,
        "the word family: {words:?}"
    );
    println!(
        "false positives: {NEGATIVES} negatives x {} profiles x 2 sources: {dismissed} proposals, \
         every one no blend; 0 reported but the half-transparent words, 0 restored",
        catalogue.profiles().len()
    );
    println!(
        "words: {words:?} x ({} synthetic + {} shipped profiles) x 2 sources: {word_dismissed} \
         proposals dismissed; half-transparent: {seen} of {translucent} examinations reported \
         (seen, not proved); 0 verified, 0 restored",
        catalogue.profiles().len(),
        shipped.profiles().len()
    );
}

/// A word style's name in the printed counts.
fn word_style(style: Word) -> &'static str {
    match style {
        Word::Opaque => "opaque",
        Word::Outlined => "outlined",
        Word::Translucent => "half-transparent",
    }
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

/// The planar path's proof (D471) did not loosen into restoring what the
/// RGB path refused: three hundred look-alike blends and three hundred
/// negatives, each stored as a 4:2:0 JPEG at 90 (`synth::jpeg_planes`) and
/// examined with those planes, are never restored, and the planes change
/// no finding into a non-finding or back — only, at most, which proof
/// refused. The refusals are counted both ways (`--nocapture`). Measured
/// over all 1000 of each (2026-10-09, before E12-R11 gave the negatives
/// a word family and so moved which `n` is which negative): 567 refused
/// by gain and 151 by
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
