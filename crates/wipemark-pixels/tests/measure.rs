//! The numbers behind the thresholds and the timings (§4.10 of the plan),
//! printed rather than asserted: run on the host and record them.
//!
//! ```sh
//! cargo test -p wipemark-pixels --release --test measure -- --ignored --nocapture
//! ```

mod support;

use std::time::Instant;

use support::*;
use wipemark_pixels::{clean, composite, examine, ExamineOptions, Layout, PixelRect, Raster};

const W: u32 = 320;
const H: u32 = 240;

/// NCC, k*, edge ratio, out-of-range share.
type Row = (f32, f32, f32, f32);

fn stats(name: &str, rows: &[Row]) {
    if rows.is_empty() {
        println!("{name:<28} no proposal");
        return;
    }
    let col = |f: fn(&Row) -> f32| {
        let v: Vec<f32> = rows.iter().map(f).collect();
        let lo = v.iter().copied().fold(f32::INFINITY, f32::min);
        let hi = v.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        format!("{lo:.3}..{hi:.3}")
    };
    println!(
        "{name:<28} n={:<4} ncc {}  k* {}  ratio {}  out {}",
        rows.len(),
        col(|r| r.0),
        col(|r| r.1),
        col(|r| r.2),
        col(|r| r.3)
    );
}

/// Add ±`amount` levels of noise to every colour sample — a stand-in for a
/// lossy codec.
fn noisy(raster: &Raster, amount: i32, seed: u64) -> Raster {
    let mut rng = Rng::new(seed);
    let c = raster.layout().channels();
    let samples = raster
        .samples()
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            if c == 4 && i % 4 == 3 {
                return s;
            }
            let d = rng.below((2 * amount + 1) as u32) as i32 - amount;
            (i32::from(s) + d).clamp(0, 255) as u16
        })
        .collect();
    Raster::new(raster.width(), raster.height(), raster.layout(), samples).unwrap()
}

#[test]
#[ignore = "measurements: run with --release --ignored --nocapture and record"]
fn distributions_behind_the_thresholds() {
    let v1 = synthetic_v1();
    let only_v1 = catalogue_of(&[synthetic_v1()]);
    let at = small_row(W, H, 48);
    let mut cases: Vec<(&str, Vec<Row>)> = Vec::new();
    for (case, build) in [
        ("true mark", 0u8),
        ("true mark, +-3 noise", 1),
        ("opaque look-alike", 2),
        ("0.72x variant", 3),
        ("unmarked", 4),
    ] {
        let mut rows = Vec::new();
        for (i, kind) in KINDS.iter().enumerate() {
            for seed in 0..6u64 {
                let mut r = picture(*kind, W, H, 5000 + i as u64 * 31 + seed, Layout::Rgb8);
                match build {
                    0 => composite(&mut r, &v1.small, at, [255.0; 3]),
                    1 => {
                        composite(&mut r, &v1.small, at, [255.0; 3]);
                        r = noisy(&r, 3, seed);
                    }
                    2 => stamp_opaque(&mut r, &v1.small, at, 0.25, 255.0),
                    3 => composite(&mut r, &scaled(&v1.small, 0.72), at, [255.0; 3]),
                    _ => {}
                }
                for f in examine(&r, &only_v1, &ExamineOptions::default()).findings {
                    if let Some(s) = f.scores {
                        rows.push((f.ncc, s.gain, s.edge_ratio, s.out_of_range));
                    }
                }
            }
        }
        cases.push((case, rows));
    }
    for (name, rows) in &cases {
        stats(name, rows);
    }
}

#[test]
#[ignore = "timings: run with --release --ignored --nocapture and record"]
fn timings_on_a_large_picture() {
    let catalogue = synthetic_catalogue();
    let v1 = synthetic_v1();
    let (w, h) = (2752, 1536);
    let base = picture(Kind::Fractal, w, h, 77, Layout::Rgb8);

    let mut row_hit = base.clone();
    let large = PixelRect {
        x: w - 64 - 96,
        y: h - 64 - 96,
        width: 96,
        height: 96,
    };
    composite(&mut row_hit, &v1.large, large, [255.0; 3]);
    let t = Instant::now();
    let exam = examine(&row_hit, &catalogue, &ExamineOptions::default());
    println!(
        "examine, row hit:  {:?} ({} findings)",
        t.elapsed(),
        exam.findings.len()
    );

    let mut searched = base.clone();
    let off = PixelRect {
        x: w - 150 - 80,
        y: h - 120 - 80,
        width: 80,
        height: 80,
    };
    let map = wipemark_pixels::resampled(&v1.large, 80.0, 0.0, 0.0).unwrap();
    composite(&mut searched, &map, off, [255.0; 3]);
    let t = Instant::now();
    let exam = examine(&searched, &catalogue, &ExamineOptions::default());
    println!(
        "examine, search:   {:?} ({} findings)",
        t.elapsed(),
        exam.findings.len()
    );

    let t = Instant::now();
    let report = clean(&mut row_hit, &catalogue, &ExamineOptions::default());
    println!(
        "clean (examine + restore + second pass): {:?} ({} restored)",
        t.elapsed(),
        report.restored.len()
    );
}

/// E12-R4 §4.2: how the search's refinement (`refine_at`, from the row's
/// own rectangle) answers a mark drawn a hair off its row — under the
/// eighth-pixel grid's resolution — over three kinds of picture, three
/// seeds and three noise amplitudes. Prints, per case, the raw best's shift,
/// `gain_ratio` and whether it clears `REFINE_MARGIN`. It is where
/// `refine_at_leaves_a_row_that_is_right` took its case from (E12-R4's
/// report): exactly at the row the residual's minimum is the row; from
/// 0.05 px the raw best moves by an eighth, and sometimes past the margin.
#[test]
#[ignore = "measurements: run with --release --ignored --nocapture and record"]
fn refinement_against_a_mark_under_an_eighth_off_its_row() {
    use wipemark_pixels::{refine_at, resampled, SubRect};
    let catalogue = synthetic_catalogue();
    let v1 = synthetic_v1();
    let row = small_row(W, H, 48);
    let base = SubRect {
        x: row.x as f32,
        y: row.y as f32,
        size: 48.0,
    };
    for off in [0.0f32, 0.03, 0.05, 0.06, 0.07] {
        let (mut n, mut moved, mut past, mut lowest) = (0, 0, 0, f64::INFINITY);
        for kind in [Kind::ValueNoise, Kind::Fractal, Kind::Gradient] {
            for amp in [0u32, 1, 2] {
                for seed in 0..3u64 {
                    let clean = picture(kind, W, H, 60 + seed, Layout::Rgb8);
                    let mut rng = Rng::new(600 + seed);
                    let noisy: Vec<u16> = clean
                        .samples()
                        .iter()
                        .map(|&v| {
                            (i32::from(v) + rng.below(2 * amp + 1) as i32 - amp as i32)
                                .clamp(0, 255) as u16
                        })
                        .collect();
                    let mut raster = Raster::new(W, H, Layout::Rgb8, noisy).unwrap();
                    let mark = quantised(&resampled(&v1.small, 48.0, off, 0.0).unwrap());
                    let at = PixelRect {
                        width: mark.width(),
                        height: mark.height(),
                        ..row
                    };
                    composite(&mut raster, &mark, at, [255.0; 3]);
                    let r = refine_at(&raster, &catalogue, "test-sparkle-v1", base).unwrap();
                    println!(
                        "{kind:?} off {off} noise ±{amp} seed {seed}: dx {:+.3} dy {:+.3} dsize {:+.3} gain_ratio {:.3} past the margin {}",
                        r.rect.x - base.x,
                        r.rect.y - base.y,
                        r.rect.size - base.size,
                        r.residual_refined / r.residual_at,
                        r.kept_by_margin
                    );
                    let ratio = r.residual_refined / r.residual_at;
                    n += 1;
                    moved += usize::from(r.rect != base);
                    past += usize::from(r.kept_by_margin);
                    lowest = lowest.min(ratio);
                }
            }
        }
        println!(
            "off {off} px: {n} cases, the raw best moved in {moved}, past the margin in {past}, lowest gain_ratio {lowest:.3}"
        );
    }
}
