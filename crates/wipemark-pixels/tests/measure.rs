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

/// What becomes of a shipped mark drawn weaker than its profile — `k` of
/// 0.90, 0.93 and 0.95 through the bench's composite (D312) — on every
/// procedural kind, three seeds, 1024 × 1024 at V1's small row and V2's
/// 1024 row, under the shipped catalogue: refused by its row's gain,
/// proved at the row, or proved by the search after the row refused it
/// (E12-R5's report, "D154 through the search"). One line per case, then
/// the counts.
#[test]
#[ignore = "measurements: run with --release --ignored --nocapture and record"]
fn a_weaker_mark_through_the_rows_and_the_search() {
    use wipemark_pixels::synth::{composite_with, Blend};
    use wipemark_pixels::{drawn, Catalogue, Kernel, Placed, SubRect, Verdict};
    let catalogue = Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"));
    let side = 1024u32;
    for (profile, map_id, margin) in [
        ("gemini-sparkle-v1", "gemini-v1-48", 32u32),
        ("gemini-sparkle-v2", "gemini-v2-36", 71),
    ] {
        let p = catalogue.profile(profile).unwrap();
        let (_, map) = p.maps.iter().find(|(id, _)| id == map_id).unwrap();
        let rect = SubRect {
            x: (side - margin - map.width()) as f32,
            y: (side - margin - map.height()) as f32,
            size: map.width() as f32,
        };
        for k in [0.90f32, 0.93, 0.95] {
            // refused, proved at the row, proved by the search, nothing
            // seen; of the restored: an outline or a texture said, and the
            // largest difference from the picture under the mark.
            let (mut refused, mut row, mut search, mut unseen) = (0, 0, 0, 0);
            let (mut said, mut worst) = (0, 0u16);
            for kind in KINDS {
                for seed in [61u64, 62, 63] {
                    let original = picture(kind, side, side, seed, Layout::Rgb8);
                    let mut raster = original.clone();
                    let blend = Blend {
                        k,
                        ..Blend::encoded(p.logo)
                    };
                    composite_with(&mut raster, &drawn(map), rect, Kernel::Area, &blend);
                    let report = clean(&mut raster, catalogue, &ExamineOptions::default());
                    let Some(f) = report.found.iter().find(|f| f.profile == profile) else {
                        unseen += 1;
                        println!("{map_id} k {k} {kind:?}/{seed}: not seen");
                        continue;
                    };
                    let gain = f.scores.map_or(f32::NAN, |s| s.gain);
                    let what = match (&f.verdict, f.placed) {
                        (Verdict::Refused(r), _) => {
                            refused += 1;
                            format!("refused {r:?}")
                        }
                        (Verdict::Verified(_), Placed::Row(_)) => {
                            row += 1;
                            String::from("proved at the row")
                        }
                        (Verdict::Verified(_), Placed::Searched) => {
                            search += 1;
                            format!(
                                "proved by the search at ({}, {}, {})",
                                f.rect.x, f.rect.y, f.rect.size
                            )
                        }
                    };
                    let restored = report.restored.iter().find(|r| r.profile == profile);
                    if let Some(r) = restored {
                        said += usize::from(r.outline_left || r.texture_left);
                        worst = worst.max(max_error(&raster, &original));
                    }
                    println!(
                        "{map_id} k {k} {kind:?}/{seed}: {what}, gain {gain}{}",
                        restored.map_or(String::new(), |r| format!(
                            ", outline {:.3} said {}, off the picture by up to {} levels",
                            r.outline,
                            r.outline_left || r.texture_left,
                            max_error(&raster, &original)
                        ))
                    );
                }
            }
            println!(
                "== {map_id} k {k}: refused {refused}, proved at the row {row}, \
                 proved by the search {search}, not seen {unseen}; \
                 of the restored, said {said}, off the picture by up to {worst} levels"
            );
        }
    }
}
