//! What reading a JPEG's planes costs — not a feature, a developer's tool
//! (D162), for step E12-R3's acceptance (`docs/plan/E12-R3-jpeg-planes.md`
//! §6.3, filed 2026-10-08 by the coordinator of the E12-R series): decoding
//! a 2048 JPEG with planes is to be at most ×1.10 of decoding it without,
//! or the planes are taken only when a mark is verified.
//!
//! For every JPEG given, it times, interleaved, `ROUNDS` times each:
//!
//! * **old** — what `decode` did for a JPEG at `4b5ba17`, restated: the
//!   scan walk (`walk_jpeg_scan`), `zune-jpeg`'s RGB decode with the same
//!   options, the `Raster`;
//! * **decode** — `wipemark_picture::decode` as it is now, the road every
//!   surface takes (it leaves `planes` `None`);
//! * **with planes** — `wipemark_picture::decode_with_planes`: `decode`,
//!   then `zune-jpeg`'s raw output (`raw_output`, `decode_into_planes`; R3
//!   had the fork's own `decode_planes` there until 2026-10-09) on a
//!   second decoder over the same bytes, cropped from the blocks' padding,
//!   and the `Planes` built from it;
//!
//! checks that all three rasters are one, and prints each file's medians
//! and the ratios `decode / old` and `with planes / old`, then the sums of
//! the medians and the ratios of those sums — the figure §6.3 asks for.
//! Run it optimised, on a quiet machine:
//!
//! ```sh
//! cargo run --release -p wipemark-picture --example planes_speed -- picture.jpg [more.jpg …]
//! ```
//!
//! The JPEGs the report's figure was measured on are the owner's 21
//! stickers at 2048 × 2048 (Watchword `wipemark-gemini-stickers-2026-10-04`,
//! `stickers/*.png`) saved at quality 95, 4:2:0 by
//! `scripts/verify/images/round4-ebf421a/mkset.py` (Pillow 12.3.0): its
//! `set/q95-420/`. `examples/planes_cost.rs` breaks the second decode
//! down when this figure moves.

use std::io::Cursor;
use std::time::{Duration, Instant};

use wipemark_image::ImageContainer;
use wipemark_picture::{decode, decode_with_planes, walk_jpeg_scan, Scan};
use wipemark_pixels::{Layout, Raster};
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;

const ROUNDS: usize = 7;

/// `decode` for a JPEG as it was before the planes (`decode.rs`'s
/// `jpeg_decode` at `4b5ba17`).
fn old(bytes: &[u8]) -> Raster {
    assert_ne!(walk_jpeg_scan(bytes), Scan::Damaged);
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGB);
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(Cursor::new(bytes), options);
    let pixels = decoder.decode().unwrap();
    let info = decoder.info().unwrap();
    Raster::from_u8(
        u32::from(info.width),
        u32::from(info.height),
        Layout::Rgb8,
        &pixels,
    )
    .unwrap()
}

fn median(mut t: Vec<Duration>) -> Duration {
    t.sort();
    t[t.len() / 2]
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1e3
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("planes_speed <picture.jpg>…");
        std::process::exit(2);
    }
    let mut sums = [Duration::ZERO; 3];
    println!("file\told ms\tdecode ms\twith planes ms\tdecode/old\twith planes/old");
    for path in &paths {
        let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let mut times: [Vec<Duration>; 3] = Default::default();
        for _ in 0..ROUNDS {
            let t = Instant::now();
            let raster = old(&bytes);
            times[0].push(t.elapsed());

            let t = Instant::now();
            let plain = decode(&bytes, ImageContainer::Jpeg).unwrap().unwrap();
            times[1].push(t.elapsed());

            let t = Instant::now();
            let with = decode_with_planes(&bytes, ImageContainer::Jpeg)
                .unwrap()
                .unwrap();
            times[2].push(t.elapsed());

            assert_eq!(plain.raster, raster, "{path}: decode's raster moved");
            assert_eq!(with.raster, raster, "{path}: the raster with planes moved");
            assert!(plain.planes.is_none(), "{path}: decode took the planes");
            assert!(with.planes.is_some(), "{path}: no planes");
        }
        let m = times.map(median);
        for (sum, t) in sums.iter_mut().zip(m) {
            *sum += t;
        }
        println!(
            "{path}\t{:.1}\t{:.1}\t{:.1}\t×{:.3}\t×{:.3}",
            ms(m[0]),
            ms(m[1]),
            ms(m[2]),
            m[1].as_secs_f64() / m[0].as_secs_f64(),
            m[2].as_secs_f64() / m[0].as_secs_f64()
        );
    }
    println!(
        "{} files\t{:.1}\t{:.1}\t{:.1}\t×{:.3}\t×{:.3}",
        paths.len(),
        ms(sums[0]),
        ms(sums[1]),
        ms(sums[2]),
        sums[1].as_secs_f64() / sums[0].as_secs_f64(),
        sums[2].as_secs_f64() / sums[0].as_secs_f64()
    );
}
