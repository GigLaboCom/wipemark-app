//! The value chosen inside the codec's interval (E12-R8) over synthetic
//! JPEGs: a mark composited in RGB, its planes made as an IJG encoder
//! would store them (`synth::jpeg_planes`), and `clean_refined` given
//! those planes. No codec, no file: the suite over the owner's crops is
//! `wipemark-picture`'s `tests/interval.rs`.

mod support;

use support::*;
use wipemark_pixels::synth::jpeg_planes;
use wipemark_pixels::{
    clean_refined, clean_with, ExamineOptions, Fidelity, Layout, PixelRect, PixelReport, Raster,
    Refine, RestoreOptions, Sampling,
};

const W: u32 = 320;
const H: u32 = 240;

fn lossy() -> ExamineOptions {
    ExamineOptions {
        source: Fidelity::Lossy,
        profiles: None,
    }
}

fn refined(
    raster: &Raster,
    planes: Option<&wipemark_pixels::Planes>,
    options: &ExamineOptions,
    refine: Refine,
) -> (Raster, PixelReport) {
    let mut out = raster.clone();
    let report = clean_refined(
        &mut out,
        planes,
        &synthetic_catalogue(),
        options,
        &RestoreOptions { refine },
    );
    (out, report)
}

/// PSNR over the colour samples of `rect` grown by four pixels.
fn psnr_roi(a: &Raster, b: &Raster, rect: PixelRect) -> f64 {
    let (x0, y0) = (rect.x.saturating_sub(4), rect.y.saturating_sub(4));
    let (x1, y1) = (
        (rect.x + rect.width + 4).min(a.width()),
        (rect.y + rect.height + 4).min(a.height()),
    );
    let (mut sum, mut n) = (0f64, 0f64);
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * a.width() + x) * 3) as usize;
            for c in 0..3 {
                let d = f64::from(a.samples()[i + c]) - f64::from(b.samples()[i + c]);
                sum += d * d;
                n += 1.0;
            }
        }
    }
    if sum == 0.0 {
        return 100.0;
    }
    10.0 * (255.0f64.powi(2) / (sum / n)).log10()
}

/// S6, in the shape of D251's `texture_left` rule: on a lossless source
/// no refinement ever runs, whatever is asked and whatever planes are
/// handed in — the same raster and the same report, byte for byte, as
/// today's path, and no `interval` anywhere. The same picture saved lossy
/// is refined, so the switch is not simply off.
#[test]
fn a_lossless_source_is_never_refined() {
    let mut marked = picture(Kind::ValueNoise, W, H, 7, Layout::Rgb8);
    composite_at(&mut marked, &synthetic_v1().small, small_row(W, H, 48));
    let lossless = ExamineOptions::default();
    for sampling in [Sampling::H444, Sampling::H420] {
        let (planes, raster) = jpeg_planes(&marked, sampling, 95).unwrap();
        let mut today = raster.clone();
        let base = clean_with(&mut today, Some(&planes), &synthetic_catalogue(), &lossless);
        assert!(!base.restored.is_empty(), "{sampling:?}: {base:#?}");
        for refine in [Refine::Dct, Refine::Pixel, Refine::Wiener] {
            for planes in [Some(&planes), None] {
                let (out, report) = refined(&raster, planes, &lossless, refine);
                assert_eq!(out, today, "{sampling:?} {refine:?}");
                assert_eq!(report.to_json(), base.to_json(), "{sampling:?} {refine:?}");
                assert!(report.restored.iter().all(|r| r.interval.is_none()));
            }
        }
        // The same file read as what it is — lossy — is refined.
        let (_, report) = refined(&raster, Some(&planes), &lossy(), Refine::Dct);
        assert!(
            report.restored.iter().all(|r| r.interval.is_some()),
            "{sampling:?}: {report:#?}"
        );
    }
}

/// A flat picture with dark strokes of "text" — short bars, one or two
/// pixels thick — where the mark `map` at `rect` is, and nowhere else:
/// what is rough under the mark is the picture's own, and around it the
/// picture is flat (D251's glyph sheet: 20.2 against 0.06).
fn strokes_under(
    map: &wipemark_pixels::AlphaMap,
    rect: PixelRect,
    seed: u64,
    count: u32,
) -> Raster {
    let mut rng = Rng::new(seed);
    let paper = [118u16, 126, 108];
    let ink = [104u16, 110, 96];
    let mut samples: Vec<u16> = (0..W * H).flat_map(|_| paper).collect();
    for _ in 0..count {
        let (len, thick) = (6 + rng.below(16), 1 + rng.below(2));
        let horizontal = rng.unit() < 0.5;
        let x0 = rect.x + 4 + rng.below(rect.width - 8);
        let y0 = rect.y + 4 + rng.below(rect.height - 8);
        for t in 0..len {
            for k in 0..thick {
                let (x, y) = if horizontal {
                    (x0 + t, y0 + k)
                } else {
                    (x0 + k, y0 + t)
                };
                let under = x < rect.x + rect.width
                    && y < rect.y + rect.height
                    && map.get(i64::from(x - rect.x), i64::from(y - rect.y))
                        >= wipemark_pixels::NOISE_FLOOR;
                if under {
                    let i = ((y * W + x) * 3) as usize;
                    samples[i..i + 3].copy_from_slice(&ink);
                }
            }
        }
    }
    Raster::new(W, H, Layout::Rgb8, samples).unwrap()
}

/// A3: on text — strokes under the mark, a flat picture around it —
/// DCT-POCS takes the smaller radius and half the `eps`, and its
/// restoration is no further from the picture under the mark than
/// today's by more than 0.3 dB of PSNR in the mark's box and four pixels
/// around it. The synthetic V1 at its small row, saved 4:4:4 at 95.
#[test]
fn text_is_not_smoothed_away() {
    let mut checked = 0;
    for seed in 1..9u64 {
        let rect = small_row(W, H, 48);
        let truth = strokes_under(&synthetic_v1().small, rect, 3000 + seed, 10);
        let mut marked = truth.clone();
        composite_at(&mut marked, &synthetic_v1().small, rect);
        let (planes, raster) = jpeg_planes(&marked, Sampling::H444, 95).unwrap();
        let mut today = raster.clone();
        let base = clean_with(&mut today, Some(&planes), &synthetic_catalogue(), &lossy());
        let (out, report) = refined(&raster, Some(&planes), &lossy(), Refine::Dct);
        let (Some(r0), Some(r)) = (base.restored.first(), report.restored.first()) else {
            println!(
                "strokes {seed}: not restored: {:?}",
                base.found.first().map(|f| (&f.verdict, f.scores))
            );
            continue;
        };
        let interval = r.interval.unwrap_or_else(|| panic!("{r:?}"));
        let (p0, p8) = (psnr_roi(&today, &truth, rect), psnr_roi(&out, &truth, rect));
        println!(
            "strokes {seed}: R0 {p0:.2} dB, R8d {p8:.2} dB; texture {:.2} → {:.2} around {:.2}; rounds {}",
            r0.texture, r.texture, r.texture_around, interval.iterations
        );
        assert!(p8 >= p0 - 0.3, "strokes {seed}: {p0:.2} → {p8:.2} dB");
        checked += usize::from(interval.iterations > 0);
    }
    assert!(checked >= 3, "{checked}");
}

/// D307 against the refinement itself: a round that would leave the
/// restoration smoother than the picture around it is taken back, so no
/// method ends on a smoothed patch — over every procedural background,
/// saved 4:4:4 and 4:2:0 at 95 and 90. `--nocapture` prints each ratio.
#[test]
fn no_refinement_ends_smoother_than_its_surroundings() {
    let mut refined_any = 0;
    for kind in KINDS {
        let mut marked = picture(kind, W, H, background_seed(kind, 1), Layout::Rgb8);
        composite_at(&mut marked, &synthetic_v1().small, small_row(W, H, 48));
        for (sampling, quality) in [
            (Sampling::H444, 95),
            (Sampling::H420, 95),
            (Sampling::H444, 90),
            (Sampling::H420, 90),
        ] {
            let (planes, raster) = jpeg_planes(&marked, sampling, quality).unwrap();
            for refine in [Refine::Dct, Refine::Pixel, Refine::Wiener] {
                let (_, report) = refined(&raster, Some(&planes), &lossy(), refine);
                for r in &report.restored {
                    let Some(interval) = r.interval else { continue };
                    println!(
                        "{kind:?} {sampling:?} q{quality} {refine:?}: {:.2} / {:.2} = {:.2}, rounds {}",
                        r.texture,
                        r.texture_around,
                        r.texture / r.texture_around,
                        interval.iterations
                    );
                    assert!(
                        !r.smoothed,
                        "{kind:?} {sampling:?} q{quality} {refine:?}: {r:?}"
                    );
                    refined_any += usize::from(interval.iterations > 0);
                }
            }
        }
    }
    assert!(refined_any >= 20, "{refined_any}");
}

/// A restoration already as rough as the picture around it — a glyph
/// sheet's strokes under the mark and around it — is in the stop rule's
/// band before the first round: no round is run, and today's restoration
/// stands byte for byte (`interval.iterations` 0), whatever the method's
/// own round trip through the planes would have rounded differently.
#[test]
fn a_restoration_already_in_the_band_is_left_as_it_was() {
    let mut seen = 0;
    for seed in 0..6u64 {
        let mut marked = picture(
            Kind::Glyphs,
            W,
            H,
            background_seed(Kind::Glyphs, seed),
            Layout::Rgb8,
        );
        composite_at(&mut marked, &synthetic_v1().small, small_row(W, H, 48));
        for sampling in [Sampling::H444, Sampling::H420] {
            let (planes, raster) = jpeg_planes(&marked, sampling, 95).unwrap();
            let mut today = raster.clone();
            let base = clean_with(&mut today, Some(&planes), &synthetic_catalogue(), &lossy());
            for refine in [Refine::Dct, Refine::Pixel] {
                let (out, report) = refined(&raster, Some(&planes), &lossy(), refine);
                let Some(r) = report.restored.first() else {
                    continue;
                };
                if r.interval.map(|i| i.iterations) != Some(0) {
                    continue;
                }
                println!("glyphs {seed} {sampling:?} {refine:?}: no round");
                assert_eq!(out, today, "glyphs {seed} {sampling:?} {refine:?}");
                let b = &base.restored[0];
                assert_eq!((r.texture, r.consistency_px), (b.texture, b.consistency_px));
                seen += 1;
            }
        }
    }
    assert!(seen >= 4, "{seen}");
}
