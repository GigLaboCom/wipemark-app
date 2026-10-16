//! What a JPEG's stored planes are, as digests — not a feature, a
//! developer's tool (D162). Written for the task that moved E12-R3 onto
//! upstream `zune-jpeg`'s raw output (Watchword
//! `wipemark-task-recon-r3-raw-output-2026-10-09`, the coordinator,
//! 2026-10-09): the planes the new road hands out must be the planes R3's
//! `decode_planes` handed out, byte for byte, on every fixture, or the
//! report has to say which differ and why.
//!
//! For every picture given it:
//!
//! 1. reads it, sniffs the container, and decodes it through
//!    `wipemark_picture::decode_with_planes` — the public road, the same on
//!    both sides of the move, so the one tool runs on R3's code and on this
//!    one;
//! 2. prints one Rust tuple per picture with planes: its path (relative to
//!    `fixtures/image/` when it is under it), the sampling, then for Y, Cb
//!    and Cr in that order the plane's width, height and the sha256 of its
//!    samples as bytes (each sample is 0–255), then the sha256 of the two
//!    quantisation tables (luma, then chroma, each 64 little-endian `u16`
//!    in natural order);
//! 3. ends that line with a comment: the first 16 hex digits of the
//!    sha256 of the decoder's RGB raster (its samples as little-endian
//!    `u16`, as `the_rgb_raster_did_not_move` hashes them), then how
//!    `Planes::to_rgb` compares with that raster — `to_rgb is the
//!    raster`, or how many pixels differ, by how many levels at most, and
//!    how many of them lie away from the last column and row;
//! 4. prints `// <path>: no planes` with the raster's digest for a picture
//!    without them, and `// <path>: not decoded` for one the decoder
//!    refuses.
//!
//! The tuples are the shape of `PLANES` in `tests/planes.rs` (the comment
//! is not part of it). That table
//! was made by running this tool over R3's code (`recon/r1-r5` at
//! `33a2c0e`) with `zune-jpeg` patched to the old fork branch
//! `wipemark/planes` (`bc409ea6`, R3's `decode_planes`) in a scratch
//! checkout that was never committed, and the new road is held to it by
//! `the_planes_are_the_ones_r3_read`. Run from the repository root:
//!
//! ```sh
//! cargo run -p wipemark-picture --example planes_digest -- \
//!     fixtures/image/jpeg-planes/*.jpg fixtures/image/gemini/*.jpg
//! ```
//!
//! It needs nothing beyond the workspace. Two runs that print the same
//! lines read the same planes and the same tables; a line that moved names
//! the picture and the plane that did.

use std::path::Path;

use sha2::{Digest, Sha256};
use wipemark_image::ImageContainer;
use wipemark_picture::decode_with_planes;
use wipemark_pixels::{Plane, Raster};

fn sha(bytes: impl IntoIterator<Item = u8>) -> String {
    let mut hash = Sha256::new();
    for b in bytes {
        hash.update([b]);
    }
    format!("{:x}", hash.finalize())
}

fn plane(p: &Plane) -> String {
    let samples = p
        .samples()
        .iter()
        .map(|&s| u8::try_from(s).unwrap_or_else(|_| panic!("a stored sample over 255: {s}")));
    format!("({}, {}, \"{}\")", p.width(), p.height(), sha(samples))
}

/// The first 16 hex digits of a raster's sha256.
fn raster_sha(raster: &Raster) -> String {
    let mut hash = Sha256::new();
    for s in raster.samples() {
        hash.update(s.to_le_bytes());
    }
    format!("{:x}", hash.finalize())[..16].to_owned()
}

/// How `to_rgb` compares with the decoder's raster, as a comment.
fn against(rgb: &Raster, raster: &Raster) -> String {
    let (w, h) = (raster.width(), raster.height());
    let (mut differ, mut inside, mut most) = (0, 0, 0);
    for (i, (a, b)) in rgb
        .samples()
        .chunks_exact(3)
        .zip(raster.samples().chunks_exact(3))
        .enumerate()
    {
        let d = a.iter().zip(b).map(|(&x, &y)| x.abs_diff(y)).max().unwrap();
        if d > 0 {
            let (x, y) = (i as u32 % w, i as u32 / w);
            differ += 1;
            inside += usize::from(x != w - 1 && y != h - 1);
            most = most.max(d);
        }
    }
    if differ == 0 {
        "to_rgb is the raster".to_owned()
    } else {
        format!(
            "to_rgb: {differ} pixels differ, by up to {most}; {inside} away from the last column and row"
        )
    }
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("planes_digest <picture.jpg>…");
        std::process::exit(2);
    }
    for path in &paths {
        let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let name = Path::new(path)
            .strip_prefix("fixtures/image")
            .map_or(path.clone(), |p| p.display().to_string());
        let container =
            ImageContainer::sniff(&bytes).unwrap_or_else(|| panic!("{path}: no container"));
        let Ok(Ok(decoded)) = decode_with_planes(&bytes, container) else {
            println!("// {name}: not decoded");
            continue;
        };
        let raster = raster_sha(&decoded.raster);
        let Some(planes) = decoded.planes else {
            println!("// {name}: no planes; raster {raster}");
            continue;
        };
        let quant = planes.quant();
        let tables = quant
            .luma
            .iter()
            .chain(quant.chroma.iter().flatten())
            .flat_map(|q| q.to_le_bytes());
        println!(
            "(\"{name}\", \"{}\", {}, {}, {}, \"{}\"), // raster {raster}; {}",
            planes.sampling().id(),
            plane(planes.y()),
            plane(planes.cb().expect("a YCbCr JPEG's Cb")),
            plane(planes.cr().expect("a YCbCr JPEG's Cr")),
            sha(tables),
            against(&planes.to_rgb(), &decoded.raster)
        );
    }
}
