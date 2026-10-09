//! The planar inverse (E12-R6, D306) over synthetic subsampled JPEGs: a
//! mark composited in RGB, its planes made as an IJG encoder would store
//! them (`synth::jpeg_planes`), the raster the decoder would hand out, and
//! `examine_with` / `clean_with` given those planes. No codec, no file:
//! the suites over real files are `wipemark-picture`'s `tests/planar.rs`
//! and `tests/lossy.rs`.

mod support;

use support::*;
use wipemark_pixels::planar::invert;
use wipemark_pixels::synth::jpeg_planes;
use wipemark_pixels::{
    clean_with, examine_with, ExamineOptions, Fidelity, Finding, Layout, Raster, Sampling,
    NOISE_FLOOR,
};

const W: u32 = 320;
const H: u32 = 240;

fn lossy() -> ExamineOptions {
    ExamineOptions {
        source: Fidelity::Lossy,
        profiles: None,
    }
}

/// A flat `colour`, `W × H`.
fn flat(colour: [u16; 3]) -> Raster {
    let samples = (0..W * H).flat_map(|_| colour).collect();
    Raster::new(W, H, Layout::Rgb8, samples).unwrap()
}

/// JFIF's `(Cb, Cr)` of an RGB colour.
fn chroma_of([r, g, b]: [u16; 3]) -> [f64; 2] {
    let [r, g, b] = [r, g, b].map(f64::from);
    [
        -0.168_736 * r - 0.331_264 * g + 0.5 * b + 128.0,
        0.5 * r - 0.418_688 * g - 0.081_312 * b + 128.0,
    ]
}

/// The first pass's finding at the small row.
fn at_the_row(found: &[Finding]) -> &Finding {
    let row = small_row(W, H, 48);
    found
        .iter()
        .find(|f| f.pass == 1 && f.pixels.is_some_and(|p| p.iou(row) > 0.5))
        .unwrap_or_else(|| panic!("nothing at the row: {found:#?}"))
}

/// Under a flat colour the stored chroma of a block is the block-mean
/// blend, `ᾱ·L + (1 − ᾱ)·Cb_O`, and the planar inverse gives `Cb_O` back
/// within a level at quality 100 — at the mark's edge too, where `α`
/// varies inside a block and dividing by any one pixel's `α` is off by
/// `(ᾱ − α)·(L − Cb_O)/(1 − α)`: tens of levels on this green.
#[test]
fn the_block_mean_inverse_recovers_flat_chroma_at_quality_100() {
    let colour = [40, 170, 90];
    let truth = chroma_of(colour).map(f64::round);
    let mut marked = flat(colour);
    composite_at(&mut marked, &synthetic_v1().small, small_row(W, H, 48));
    let (planes, raster) = jpeg_planes(&marked, Sampling::H420, 100).unwrap();
    let catalogue = synthetic_catalogue();
    let exam = examine_with(&raster, Some(&planes), &catalogue, &lossy());
    let finding = at_the_row(&exam.findings);
    let verified = finding.verified().unwrap_or_else(|| panic!("{finding:#?}"));
    let inverse = invert(&raster, &planes, verified).unwrap();
    let (mut blocks, mut edges, mut worst) = (0, 0, 0f64);
    for (b, &a) in inverse.alpha_mean.iter().enumerate() {
        if !(f64::from(NOISE_FLOOR)..inverse.opaque).contains(&a) {
            continue;
        }
        blocks += 1;
        let qx = inverse.blocks.x + b as u32 % inverse.blocks.width;
        let qy = inverse.blocks.y + b as u32 / inverse.blocks.width;
        let alphas: Vec<f64> = (0..2)
            .flat_map(|dy| (0..2).map(move |dx| (2 * qx + dx, 2 * qy + dy)))
            .map(|(x, y)| inverse.alpha_at(x, y))
            .collect();
        let spread =
            alphas.iter().copied().fold(0.0, f64::max) - alphas.iter().copied().fold(1.0, f64::min);
        edges += usize::from(spread > 0.1);
        for (got, want) in [(inverse.cb_out[b], truth[0]), (inverse.cr_out[b], truth[1])] {
            worst = worst.max((got - want).abs());
        }
    }
    assert!(
        blocks > 150 && edges > 50,
        "{blocks} blocks, {edges} at an edge"
    );
    assert!(
        worst <= 1.0,
        "the block-mean inverse is {worst:.2} levels off the flat chroma"
    );
}

/// The main test of the weakening (D306): a blend that is **not** the
/// profile's — the sparkle at 0.7 of its opacity, white — over a yellow
/// whose blue is at the floor, saved 4:2:0 at quality 90. Luma alone is
/// within range of a blend there (a bright Y has room on both sides); the
/// chroma blocks are not — Cb sits under what any blend at the mark's own
/// `ᾱ` could make. The share stays over the bound, and the chroma term is
/// what carries it; nothing is restored.
#[test]
fn an_unmarked_420_picture_is_still_refused_out_of_range() {
    let catalogue = catalogue_of(&[synthetic_v1()]);
    let mut lookalike = flat([250, 215, 0]);
    composite_at(
        &mut lookalike,
        &scaled(&synthetic_v1().small, 0.7),
        small_row(W, H, 48),
    );
    let (planes, raster) = jpeg_planes(&lookalike, Sampling::H420, 90).unwrap();
    let exam = examine_with(&raster, Some(&planes), &catalogue, &lossy());
    let finding = at_the_row(&exam.findings);
    let scores = finding.scores.unwrap();
    let planar = scores.planar.expect("measured in the planes");
    assert!(
        scores.out_of_range > 0.01 && planar.chroma > 0.01,
        "{scores:?}"
    );
    assert!(planar.y <= 0.01, "luma alone sees it: {scores:?}");
    assert!(finding.verified().is_none(), "{finding:#?}");
    let mut cleaned = raster.clone();
    let report = clean_with(&mut cleaned, Some(&planes), &catalogue, &lossy());
    assert!(report.restored.is_empty(), "{report:#?}");
    assert_eq!(cleaned, raster);
}

/// A lossless source, or 4:4:4 planes, is the RGB path whatever planes
/// are handed in: the same findings, the same restoration, the same
/// raster, the same JSON — no `planar` anywhere (§4.4).
#[test]
fn a_444_or_lossless_source_takes_the_rgb_path_with_planes_given() {
    let catalogue = synthetic_catalogue();
    let mut marked = picture(Kind::Gradient, W, H, 7, Layout::Rgb8);
    composite_at(&mut marked, &synthetic_v1().small, small_row(W, H, 48));
    for (sampling, options) in [
        (Sampling::H444, lossy()),
        (Sampling::H420, ExamineOptions::default()),
    ] {
        let (planes, raster) = jpeg_planes(&marked, sampling, 95).unwrap();
        let (mut old, mut new) = (raster.clone(), raster.clone());
        let a = wipemark_pixels::clean(&mut old, &catalogue, &options);
        let b = clean_with(&mut new, Some(&planes), &catalogue, &options);
        assert!(!a.restored.is_empty(), "{sampling:?}: {a:#?}");
        assert_eq!(old, new, "{sampling:?}");
        assert_eq!(a.to_json(), b.to_json(), "{sampling:?}");
        assert!(!b.to_json().contains("planar"), "{sampling:?}");
    }
}

/// The planar path's restoration says so (`planar`), and writes nothing
/// outside the mark's rectangle — the proof's `outside_unchanged` holds.
#[test]
fn a_420_restoration_says_it_was_planar_and_stays_in_its_rectangle() {
    let catalogue = synthetic_catalogue();
    let mut marked = flat([9, 150, 56]);
    composite_at(&mut marked, &synthetic_v1().small, small_row(W, H, 48));
    let (planes, raster) = jpeg_planes(&marked, Sampling::H420, 95).unwrap();
    let mut cleaned = raster.clone();
    let report = clean_with(&mut cleaned, Some(&planes), &catalogue, &lossy());
    let [r] = report.restored.as_slice() else {
        panic!("{report:#?}")
    };
    let json = report.to_json();
    assert!(
        json.contains(r#""planar":{"sampling":"4:2:0","max_alpha_dev_in_block":"#),
        "{json}"
    );
    assert!(json.contains(r#""planar":{"y":"#), "{json}");
    for y in 0..H {
        for x in 0..W {
            let inside = x >= r.rect.x
                && y >= r.rect.y
                && x < r.rect.x + r.rect.width
                && y < r.rect.y + r.rect.height;
            let i = ((y * W + x) * 3) as usize;
            if !inside {
                assert_eq!(
                    cleaned.samples()[i..i + 3],
                    raster.samples()[i..i + 3],
                    "({x}, {y})"
                );
            }
        }
    }
}

/// The synthetic encoder's own gates: its tables are IJG's at the quality
/// asked for, and at quality 100 without subsampling it hands back the
/// picture within the colour conversion's rounding.
#[test]
fn the_synthetic_encoder_is_ijgs_tables_and_round_trips_at_100() {
    let original = picture(Kind::ValueNoise, 64, 48, 3, Layout::Rgb8);
    let (planes, _) = jpeg_planes(&original, Sampling::H420, 95).unwrap();
    assert_eq!(planes.quant().luma[0], 2);
    assert_eq!(planes.quant().chroma.unwrap()[0], 2);
    let (planes, _) = jpeg_planes(&original, Sampling::H420, 90).unwrap();
    assert_eq!(planes.quant().chroma.unwrap()[0], 3);
    assert_eq!(planes.quant().chroma.unwrap()[63], 20);
    assert_eq!(
        (planes.cb().unwrap().width(), planes.cb().unwrap().height()),
        (32, 24)
    );
    let (_, back) = jpeg_planes(&original, Sampling::H444, 100).unwrap();
    assert!(
        max_error(&original, &back) <= 3,
        "{}",
        max_error(&original, &back)
    );
    assert!(jpeg_planes(&original, Sampling::Gray, 90).is_none());
}

/// Where the chroma allowance (`BLEND_LEVELS_C`, D306) stops seeing a
/// blend that is not the profile's: the sparkle at 0.7, 0.8 and 0.9 of its
/// opacity over six flat colours — the corners of the RGB cube a white
/// logo can be told on, and the stickers' green — saved 4:2:0 at 95, 90
/// and 85, the combined share at allowances of 4 to 32 levels next to the
/// RGB path's share, against a catalogue of V1 alone. A measurement, for
/// the E12-R6 report: `cargo test -p wipemark-pixels --release --test
/// planar -- --ignored --nocapture`.
#[test]
#[ignore = "measurements: run with --release --ignored --nocapture and record"]
fn measure_where_the_chroma_allowance_stops_seeing_a_lookalike() {
    let catalogue = catalogue_of(&[synthetic_v1()]);
    let allowances = [4.0, 8.0, 9.0, 10.0, 12.0, 16.0, 24.0, 32.0];
    println!("colour, k, quality: RGB share | planar y, chroma | combined at {allowances:?}");
    for colour in [
        [250, 215, 0],
        [0, 0, 250],
        [250, 0, 0],
        [0, 200, 250],
        [250, 0, 250],
        [9, 150, 56],
    ] {
        for k in [0.7, 0.8, 0.9] {
            let mut lookalike = flat(colour);
            composite_at(
                &mut lookalike,
                &scaled(&synthetic_v1().small, k),
                small_row(W, H, 48),
            );
            for quality in [95, 90, 85] {
                let (planes, raster) = jpeg_planes(&lookalike, Sampling::H420, quality).unwrap();
                let share = |f: Option<&Finding>| f.and_then(|f| f.scores);
                let rgb = wipemark_pixels::examine(&raster, &catalogue, &lossy());
                let planar = examine_with(&raster, Some(&planes), &catalogue, &lossy());
                let (Some(r), Some(p)) =
                    (share(rgb.findings.first()), share(planar.findings.first()))
                else {
                    println!("{colour:?}, {k}, q{quality}: no finding");
                    continue;
                };
                let terms = p.planar.unwrap();
                let curve: Vec<String> = allowances
                    .iter()
                    .map(|&l| {
                        wipemark_pixels::examine_planar_at(
                            &raster,
                            &planes,
                            l,
                            &catalogue,
                            &lossy(),
                        )
                        .and_then(|e| e.findings.first().and_then(|f| f.scores))
                        .map_or(String::from("-"), |s| {
                            format!("{:.2}", 100.0 * s.out_of_range)
                        })
                    })
                    .collect();
                println!(
                    "{colour:?}, {k}, q{quality}: RGB {:.2} % | y {:.2} %, chroma {:.2} % | {}",
                    100.0 * r.out_of_range,
                    100.0 * terms.y,
                    100.0 * terms.chroma,
                    curve.join(" ")
                );
            }
        }
    }
}
