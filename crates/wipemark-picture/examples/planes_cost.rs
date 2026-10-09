//! Where reading a JPEG's planes spends its time — not a feature, a
//! developer's tool (D162). Written for the task that moved E12-R3 onto
//! upstream `zune-jpeg`'s raw output (Watchword
//! `wipemark-task-recon-r3-raw-output-2026-10-09`, the coordinator,
//! 2026-10-09), when `planes_speed` first measured the new road at ×1.80
//! of a plain decode against R3's ×1.40, and the question was whether the
//! cost was upstream's or ours.
//!
//! For every JPEG given (three components), it times, interleaved,
//! `ROUNDS` times each, and prints the sums of the per-file medians:
//!
//! 1. **zune rgb** — `zune-jpeg`'s own RGB `decode()`, nothing of ours;
//! 2. **raw decode** — `decode_headers`, `raw_output`, `layout` and
//!    `decode_into_planes` into buffers of `byte_size` (the blocks'
//!    padding included): what `wipemark_picture`'s `jpeg_planes` asks of
//!    the decoder;
//! 3. **crop, collected** — those buffers cropped to `width × height` and
//!    widened to `u16` through one `flat_map(..).collect()`, then
//!    `Plane::new`: the first draft's crop;
//! 4. **crop, row by row** — the same into a `Vec` allocated once and
//!    extended row by row, then `Plane::new`: what `jpeg_planes` does;
//! 5. **decode** — `wipemark_picture::decode`, the product's road (the scan
//!    walk, the RGB decode, the `Raster`);
//! 6. **decode with planes** — `wipemark_picture::decode_with_planes`.
//!
//! Run it optimised, on a quiet machine:
//!
//! ```sh
//! cargo run --release -p wipemark-picture --example planes_cost -- picture.jpg [more.jpg …]
//! ```
//!
//! The report's figures were measured on the owner's 21 stickers at
//! 2048 × 2048 (Watchword `wipemark-gemini-stickers-2026-10-04`,
//! `stickers/*.png`) saved at quality 95, 4:2:0 by
//! `scripts/verify/images/round4-ebf421a/mkset.py` (Pillow 12.3.0): its
//! `set/q95-420/` — the set `planes_speed` reads. It needs nothing beyond
//! the workspace. Rows 3 and 4 differ only in how the same samples are
//! copied; 6 − 5 is what the planes add, and should be about 2 + 4.

use std::io::Cursor;
use std::time::{Duration, Instant};

use wipemark_image::ImageContainer;
use wipemark_picture::{decode, decode_with_planes};
use wipemark_pixels::Plane;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;
use zune_jpeg::PlaneInfo;

const ROUNDS: usize = 7;

const ROWS: [&str; 6] = [
    "zune rgb",
    "raw decode",
    "crop, collected",
    "crop, row by row",
    "decode",
    "decode with planes",
];

fn median(mut t: Vec<Duration>) -> f64 {
    t.sort();
    t[t.len() / 2].as_secs_f64() * 1e3
}

fn plane(info: &PlaneInfo, samples: Vec<u16>) -> Plane {
    Plane::new(info.width as u32, info.height as u32, samples).unwrap()
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("planes_cost <picture.jpg>…");
        std::process::exit(2);
    }
    let mut sums = [0.0; ROWS.len()];
    for path in &paths {
        let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let mut times: [Vec<Duration>; ROWS.len()] = Default::default();
        for _ in 0..ROUNDS {
            let t = Instant::now();
            let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGB);
            let mut decoder =
                zune_jpeg::JpegDecoder::new_with_options(Cursor::new(&bytes[..]), options);
            std::hint::black_box(decoder.decode().unwrap());
            times[0].push(t.elapsed());

            let t = Instant::now();
            let mut decoder = zune_jpeg::JpegDecoder::new(Cursor::new(&bytes[..]));
            decoder.decode_headers().unwrap();
            let mut raw = decoder.raw_output();
            assert_eq!(raw.num_components(), Some(3), "{path}: three components");
            let layout = raw.layout().unwrap();
            let mut padded: Vec<Vec<u8>> =
                layout[..3].iter().map(|p| vec![0; p.byte_size]).collect();
            let mut buffers: Vec<&mut [u8]> = padded.iter_mut().map(Vec::as_mut_slice).collect();
            raw.decode_into_planes(&mut buffers).unwrap();
            times[1].push(t.elapsed());

            let t = Instant::now();
            for (info, padded) in layout.iter().zip(&padded) {
                let samples = padded
                    .chunks_exact(info.stride)
                    .take(info.height)
                    .flat_map(|row| &row[..info.width])
                    .map(|&s| u16::from(s))
                    .collect();
                std::hint::black_box(plane(info, samples));
            }
            times[2].push(t.elapsed());

            let t = Instant::now();
            for (info, padded) in layout.iter().zip(&padded) {
                let mut samples = Vec::with_capacity(info.width * info.height);
                for row in padded.chunks_exact(info.stride).take(info.height) {
                    samples.extend(row[..info.width].iter().map(|&s| u16::from(s)));
                }
                std::hint::black_box(plane(info, samples));
            }
            times[3].push(t.elapsed());

            let t = Instant::now();
            std::hint::black_box(decode(&bytes, ImageContainer::Jpeg).unwrap().unwrap());
            times[4].push(t.elapsed());

            let t = Instant::now();
            let with = decode_with_planes(&bytes, ImageContainer::Jpeg)
                .unwrap()
                .unwrap();
            times[5].push(t.elapsed());
            assert!(with.planes.is_some(), "{path}: no planes");
        }
        for (sum, t) in sums.iter_mut().zip(times) {
            *sum += median(t);
        }
    }
    println!(
        "{} files, the sums of the medians of {ROUNDS}:",
        paths.len()
    );
    for (row, sum) in ROWS.iter().zip(sums) {
        println!("{row:20}\t{sum:8.1} ms");
    }
}
