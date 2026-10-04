//! D164: the false-positive gate. Two thousand procedural negatives —
//! textures, glyphs, flat and bright corners, noise, and look-alikes:
//! sparkles and diamonds drawn opaque, blurred or at the wrong opacity —
//! against every profile. Not one may be restored. The maxima are
//! printed (`--nocapture`) and recorded in the step's report.

mod support;

use support::*;
use wipemark_pixels::{
    clean, composite, AlphaMap, Catalogue, ExamineOptions, Layout, PixelRect, Raster,
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

/// The `n`-th negative.
fn negative(n: u64) -> (String, Raster) {
    let mut rng = Rng::new(n.wrapping_mul(7919) + 3);
    let kind = KINDS[(n % KINDS.len() as u64) as usize];
    let mut raster = picture(kind, SIDE, SIDE, n + 10_000, Layout::Rgb8);
    let size = 24 + rng.below(40);
    let x = rng.below(SIDE - size);
    let y = rng.below(SIDE - size);
    let at = PixelRect {
        x,
        y,
        width: size,
        height: size,
    };
    let what = match n % 8 {
        // The bare texture.
        0..=2 => "texture",
        // An opaque white sparkle.
        3 => {
            stamp_opaque(&mut raster, &sparkle(size, 0.5), at, 0.25, 255.0);
            "opaque sparkle"
        }
        // A blurred sparkle at an opacity no profile has.
        4 => {
            let peak = rng.range(0.75, 1.0);
            composite(
                &mut raster,
                &blurred(&sparkle(size, peak), 2),
                at,
                [255.0; 3],
            );
            "blurred sparkle"
        }
        // A diamond, half transparent.
        5 => {
            composite(
                &mut raster,
                &diamond(size, rng.range(0.3, 0.6)),
                at,
                [255.0; 3],
            );
            "diamond"
        }
        // An opaque diamond.
        6 => {
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

#[test]
fn no_procedural_negative_is_ever_restored() {
    let catalogue: Catalogue = synthetic_catalogue();
    let options = ExamineOptions::default();
    let (mut proposed, mut max_ncc, mut max_named) = (0usize, 0f32, String::new());
    let mut min_ratio = f32::INFINITY;
    for n in 0..2000u64 {
        let (name, original) = negative(n);
        let mut raster = original.clone();
        let report = clean(&mut raster, &catalogue, &options);
        for f in &report.found {
            proposed += 1;
            if f.ncc > max_ncc {
                max_ncc = f.ncc;
                max_named.clone_from(&name);
            }
            if let Some(s) = f.scores {
                min_ratio = min_ratio.min(s.edge_ratio);
            }
        }
        assert!(
            report.restored.is_empty(),
            "{name} was restored: {:#?}",
            report.found
        );
        assert_eq!(raster, original, "{name}");
    }
    println!(
        "false positives: 2000 negatives x {} profiles, {proposed} proposals, \
         max NCC {max_ncc:.3} ({max_named}), lowest edge ratio among them {min_ratio:.3}, \
         0 restored",
        catalogue.profiles().len()
    );
}
