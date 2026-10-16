//! Measure a profile's opacity map from real outputs — not a feature, a
//! developer's tool, as the calibration example is (D162).
//!
//! GWT's maps are 8-bit captures of the vendor's mark over black. Over
//! real Gemini outputs their soft edge is a level or two too strong, and a
//! restoration leaves a faint outline of the sparkle (D243). Given real
//! outputs whose corner around the mark is flat, this fits α per pixel by
//! least squares: `I − O = α·(L − O)` per channel, `O` the picture's own
//! mean in a ring around the mark's square, `L` the profile's measured
//! logo, every channel whose `L − O` is at least 40 levels, every
//! picture. It writes a 16-bit `.wma` and prints its sha256 and how many
//! samples each pixel had.
//!
//! ```sh
//! cargo run -p wipemark-picture --example measure_map -- \
//!     gemini-sparkle-v1 96 64 out.wma picture.png [picture.png …]
//! ```

use std::fmt::Write as _;

use sha2::{Digest, Sha256};
use wipemark_picture::decode;
use wipemark_pixels::{AlphaMap, Catalogue};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [profile, size, margin, out, pictures @ ..] = args.as_slice() else {
        eprintln!("measure_map <profile> <size> <margin> <out.wma> <picture.png>…");
        std::process::exit(2);
    };
    let (size, margin): (u32, u32) = (size.parse().unwrap(), margin.parse().unwrap());
    let catalogue = Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"));
    let logo = catalogue
        .profile(profile)
        .unwrap_or_else(|| panic!("no profile {profile}"))
        .logo
        .map(f64::from);
    let n = (size * size) as usize;
    let (mut num, mut den, mut count) = (vec![0f64; n], vec![0f64; n], vec![0u32; n]);
    for path in pictures {
        let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let container = wipemark_image::inspect(&bytes).unwrap().container;
        let raster = decode(&bytes, container).unwrap().unwrap().raster;
        let (w, h, c) = (raster.width(), raster.height(), raster.layout().channels());
        let (x0, y0) = (w - margin - size, h - margin - size);
        let at =
            |x: u32, y: u32, k: usize| f64::from(raster.samples()[((y * w + x) as usize) * c + k]);
        // The picture under the mark: its mean in a ring 8–36 pixels out.
        let (mut ring, mut nr) = ([0f64; 3], 0f64);
        for y in y0 - 36..y0 + size + 36 {
            for x in x0 - 36..x0 + size + 36 {
                let d = (i64::from(x0) - i64::from(x))
                    .max(i64::from(x) - i64::from(x0 + size - 1))
                    .max(i64::from(y0) - i64::from(y))
                    .max(i64::from(y) - i64::from(y0 + size - 1));
                if (8..=36).contains(&d) {
                    for (k, r) in ring.iter_mut().enumerate() {
                        *r += at(x, y, k);
                    }
                    nr += 1.0;
                }
            }
        }
        let o = ring.map(|r| r / nr);
        for y in 0..size {
            for x in 0..size {
                let p = (y * size + x) as usize;
                for k in 0..3 {
                    let span = logo[k] - o[k];
                    if span.abs() < 40.0 {
                        continue;
                    }
                    num[p] += (at(x0 + x, y0 + y, k) - o[k]) * span;
                    den[p] += span * span;
                    count[p] += 1;
                }
            }
        }
        eprintln!("{path}: under the mark {o:.1?}");
    }
    let values: Vec<f32> = (0..n)
        .map(|p| {
            if den[p] > 0.0 {
                (num[p] / den[p]).clamp(0.0, 1.0) as f32
            } else {
                0.0
            }
        })
        .collect();
    let map = AlphaMap::new(size, size, values).unwrap();
    let bytes = map.write(16).unwrap();
    std::fs::write(out, &bytes).unwrap();
    let mut hex = String::new();
    for b in Sha256::digest(&bytes) {
        let _ = write!(hex, "{b:02x}");
    }
    let (lo, hi) = (count.iter().min().unwrap(), count.iter().max().unwrap());
    println!("{out}: {size}x{size}, depth 16, sha256 {hex}, {lo}–{hi} samples a pixel");
}
