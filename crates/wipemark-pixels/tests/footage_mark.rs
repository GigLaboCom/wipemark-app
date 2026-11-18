//! The Gemini V1 sparkle as the picture video draws it (`dev/video/
//! picture/storyboard.md`, 22.0–25.2 s): the measured opacity map the large
//! row restores with, read by the product's own reader from the shipped
//! catalogue, and the profile's colour and place, written where a web page
//! can read them. Ignored: footage, not a gate.
//!
//!   WIPEMARK_FOOTAGE=<dir> cargo test -p wipemark-pixels --test footage_mark \
//!     -- --ignored --exact the_mark_as_a_page_reads_it --nocapture
//!
//! It writes `<dir>/mark/`:
//! - `gemini-v1-96-measured.png`: 96 × 96, 16-bit grey, each sample the
//!   `.wma`'s own (α × 65535);
//! - `gemini-v1-96-measured-8bit.png`: the same, 8-bit grey, α × 255
//!   rounded (what a canvas reads);
//! - `gemini-sparkle-v1-sheet.png`: 96 × 96 RGBA, the logo colour rounded
//!   to 8 bits in every pixel and α × 255 as its alpha — the mark as a
//!   sheet, `I = α·L + (1 − α)·O` with `O` left for the page;
//! - `gemini-sparkle-v1.json`: the profile's logo colour, model, opaque
//!   threshold, proofs' limits, the map's id, size, sha256 and peak, the
//!   placement row a 1025 × 1025 picture takes and the rectangle it puts
//!   the mark at, and the samples (`u16`, row-major).

use std::path::PathBuf;

use sha2::{Digest as _, Sha256};
use wipemark_pixels::{shipped_assets, Anchor, Catalogue};

const PROFILE: &str = "gemini-sparkle-v1";
const MAP: &str = "gemini-v1-96-measured";
/// The sticker the video shows: the owner's Gemini output cut to its
/// bottom-right 1025 × 1025 (`fixtures/image/gemini/torch-1025.png`).
const PICTURE: (u32, u32) = (1025, 1025);

fn png(
    path: &PathBuf,
    width: u32,
    height: u32,
    colour: png::ColorType,
    depth: png::BitDepth,
    data: &[u8],
) {
    let file = std::io::BufWriter::new(std::fs::File::create(path).expect("a file"));
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(colour);
    encoder.set_depth(depth);
    encoder
        .write_header()
        .and_then(|mut writer| writer.write_image_data(data))
        .expect("a PNG written");
}

#[test]
#[ignore = "footage for the picture video: WIPEMARK_FOOTAGE=<dir>"]
fn the_mark_as_a_page_reads_it() {
    let out =
        PathBuf::from(std::env::var("WIPEMARK_FOOTAGE").expect("WIPEMARK_FOOTAGE")).join("mark");
    std::fs::create_dir_all(&out).expect("the folder");

    let catalogue = Catalogue::shipped().expect("the shipped catalogue reads");
    let profile = catalogue.profile(PROFILE).expect("the profile");
    let index = profile
        .maps
        .iter()
        .position(|(id, _)| id == MAP)
        .expect("the measured map");
    let map = profile.map(index);
    let fitted = profile.fitted[index];

    // The asset the build compiled in is the file in the tree, byte for
    // byte, and the catalogue read it as the samples written here.
    let asset = format!("{MAP}.wma");
    let shipped = shipped_assets()
        .find(|(name, _)| *name == asset)
        .map(|(_, bytes)| bytes)
        .expect("the asset is shipped");
    let on_disk = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("marks/measured")
            .join(&asset),
    )
    .expect("the asset on disk");
    assert_eq!(
        shipped,
        on_disk.as_slice(),
        "the shipped map is not the file"
    );
    let sha256: String = Sha256::digest(shipped)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(&shipped[..4], b"WMA1");
    let depth = shipped[8];
    assert_eq!(depth, 16, "the measured map is 16-bit");
    let (pairs, rest) = shipped[9..].as_chunks::<2>();
    assert!(rest.is_empty(), "an odd number of sample bytes");
    let samples: Vec<u16> = pairs.iter().map(|s| u16::from_le_bytes(*s)).collect();
    let (width, height) = (map.width(), map.height());
    assert_eq!(samples.len(), (width * height) as usize);
    for (sample, value) in samples.iter().zip(map.values()) {
        assert!(
            (f32::from(*sample) / f32::from(u16::MAX) - value).abs() < 1e-6,
            "the reader and the file disagree"
        );
    }

    // The row a 1025 × 1025 picture takes, and where it puts the mark.
    let row = profile
        .placements
        .iter()
        .find(|p| p.when.matches(PICTURE.0, PICTURE.1))
        .expect("a row for the sticker's size");
    assert_eq!(row.alpha, index, "the row draws with another map");
    let Anchor::Corner { corner, margin } = row.anchor else {
        panic!("the row is not a corner: {:?}", row.anchor)
    };
    let (x, y) = corner
        .origin(PICTURE.0, PICTURE.1, width, height, margin)
        .expect("the mark fits");

    // 16-bit grey: the file's samples, big-endian as PNG keeps them.
    let grey16: Vec<u8> = samples.iter().flat_map(|s| s.to_be_bytes()).collect();
    png(
        &out.join(format!("{MAP}.png")),
        width,
        height,
        png::ColorType::Grayscale,
        png::BitDepth::Sixteen,
        &grey16,
    );
    let alpha8: Vec<u8> = map
        .values()
        .iter()
        .map(|v| (v * 255.0).round() as u8)
        .collect();
    png(
        &out.join(format!("{MAP}-8bit.png")),
        width,
        height,
        png::ColorType::Grayscale,
        png::BitDepth::Eight,
        &alpha8,
    );
    let logo8 = profile.logo.map(|c| c.round() as u8);
    let sheet: Vec<u8> = alpha8
        .iter()
        .flat_map(|a| [logo8[0], logo8[1], logo8[2], *a])
        .collect();
    png(
        &out.join(format!("{PROFILE}-sheet.png")),
        width,
        height,
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        &sheet,
    );

    let corner_name = format!("{corner:?}");
    // The catalogue's numbers as it writes them: an f32 widened prints
    // its binary tail (252.10000610351562).
    let decimal = |v: f32, places: i32| {
        let scale = 10f64.powi(places);
        (f64::from(v) * scale).round() / scale
    };
    let json = serde_json::json!({
        "how": "read by wipemark_pixels::Catalogue::shipped() (the catalogue compiled into the product, manifests/marks.v1.json, and every .wma under crates/wipemark-pixels/marks/, re-hashed on load) and AlphaMap::read; the placement row and its rectangle by Placement::when.matches and Corner::origin; written by crates/wipemark-pixels/tests/footage_mark.rs",
        "profile": {
            "id": profile.id,
            "vendor": profile.vendor,
            "product": profile.product,
            "mark": profile.mark,
            "model": format!("{:?}", profile.model),
            "logo": profile.logo.map(|c| decimal(c, 2)),
            "logo_8bit": logo8,
            "opaque_above": decimal(profile.opaque_above, 4),
            "min_ncc": decimal(profile.min_ncc, 4),
            "verify": {
                "gain": decimal(profile.thresholds.gain, 4),
                "edge_ratio": decimal(profile.thresholds.edge_ratio, 4),
                "out_of_range": decimal(profile.thresholds.out_of_range, 4),
            },
        },
        "map": {
            "id": MAP,
            "asset": asset,
            "sha256": sha256,
            "bytes": shipped.len(),
            "format": "WMA1: the magic, u16 width, u16 height, u8 depth (little-endian), then width x height samples row-major, each depth bits little-endian; alpha = sample / (2^depth - 1)",
            "width": width,
            "height": height,
            "depth": depth,
            "fitted": fitted,
            "peak": decimal(map.peak(), 6),
            "png16": format!("{MAP}.png"),
            "png8": format!("{MAP}-8bit.png"),
            "sheet": format!("{PROFILE}-sheet.png"),
            "samples_u16": samples,
        },
        "placement": {
            "picture": [PICTURE.0, PICTURE.1],
            "when": {
                "min_width": row.when.min_width,
                "min_height": row.when.min_height,
            },
            "corner": corner_name,
            "margin": margin,
            "rect": { "x": x, "y": y, "width": width, "height": height },
        },
        "blend": "I = alpha * L + (1 - alpha) * O per channel, in 8-bit levels (the encoded model): L the logo colour, O the picture under the mark, I what the file holds",
    });
    std::fs::write(
        out.join(format!("{PROFILE}.json")),
        serde_json::to_string_pretty(&json).expect("JSON") + "\n",
    )
    .expect("the JSON written");
    eprintln!(
        "{PROFILE}: {MAP} {width}x{height} at {x},{y} on {}x{}, logo {:?}, peak {}, fitted {fitted}",
        PICTURE.0, PICTURE.1, profile.logo, map.peak()
    );
}
