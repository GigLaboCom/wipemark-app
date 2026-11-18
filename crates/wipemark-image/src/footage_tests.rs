//! The C2PA test photo as the picture video draws it (`dev/video/picture/
//! storyboard.md`, 12.6–19.0 s): every block the parser cuts it into —
//! structure and metadata alike, the blocks tiling the file — before and
//! after the default strip, written where a web page can read them.
//! Inside the crate because the blocks are the parser's own and not public
//! API. Ignored: footage, not a gate.
//!
//!   WIPEMARK_FOOTAGE=<dir> cargo test -p wipemark-image --lib -- --ignored \
//!     --exact footage_tests::the_photo_as_its_blocks --nocapture
//!
//! It writes `<dir>/blocks/coast.blocks.json`. Where `<dir>/files/cli/
//! coast.cleaned.jpg` or `<dir>/files/panel/coast.cleaned.jpg` exists (the
//! command line's and the panel's results), the strip here is held to be
//! that file, byte for byte.

use std::io::Cursor;
use std::path::PathBuf;

use sha2::{Digest as _, Sha256};

use crate::{parse, strip, Block, StripOptions};

/// A JPEG marker's name, for the block it starts.
fn marker(bytes: &[u8], block: &Block) -> String {
    let at = block.range.start;
    if at == 0 {
        return "SOI".into();
    }
    if bytes.get(at) != Some(&0xFF) {
        return "trailer".into();
    }
    let m = bytes[at + 1];
    match m {
        0xD9 => "EOI".into(),
        0xDA => "SOS".into(),
        0xDB => "DQT".into(),
        0xC4 => "DHT".into(),
        0xDD => "DRI".into(),
        0xFE => "COM".into(),
        0xC0..=0xCF => format!("SOF{}", m - 0xC0),
        0xE0..=0xEF => format!("APP{}", m - 0xE0),
        other => format!("FF{other:02X}"),
    }
}

fn hex(digest: impl AsRef<[u8]>) -> String {
    digest.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

/// Every block of `bytes`, in order, as JSON.
fn blocks(bytes: &[u8], removed: &[bool]) -> (Vec<serde_like::Value>, usize) {
    let parsed = parse(bytes).expect("the JPEG parses");
    assert!(
        crate::tiles(bytes.len(), &parsed.blocks),
        "the blocks tile the file"
    );
    let mut out = Vec::new();
    for (ix, block) in parsed.blocks.iter().enumerate() {
        let name = marker(bytes, block);
        let (class, kind, ai) = match &block.finding {
            Some(finding) => (
                "metadata",
                finding.kind.id().to_owned(),
                finding.is_ai_provenance(),
            ),
            None if name == "SOS" => ("image-data", String::from("scan"), false),
            None => ("structure", String::from("structure"), false),
        };
        out.push(serde_like::object(&[
            ("index", serde_like::num(ix as u64)),
            ("marker", serde_like::text(&name)),
            ("class", serde_like::text(class)),
            ("kind", serde_like::text(&kind)),
            ("ai_provenance", serde_like::boolean(ai)),
            ("offset", serde_like::num(block.range.start as u64)),
            ("length", serde_like::num(block.range.len() as u64)),
            (
                "removed_by_default",
                serde_like::boolean(removed.get(ix).copied().unwrap_or(false)),
            ),
            (
                "sha256",
                serde_like::text(&hex(Sha256::digest(&bytes[block.range.clone()]))),
            ),
        ]));
    }
    (out, parsed.blocks.len())
}

#[test]
#[ignore = "footage for the picture video: WIPEMARK_FOOTAGE=<dir>"]
fn the_photo_as_its_blocks() {
    let out = PathBuf::from(std::env::var("WIPEMARK_FOOTAGE").expect("WIPEMARK_FOOTAGE"));
    std::fs::create_dir_all(out.join("blocks")).expect("the folder");
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/image/c2pa-jumbf.jpg");
    let before = std::fs::read(&fixture).expect("the photo");
    let (after, report) = strip(&before, &StripOptions::default()).expect("the strip");

    // Which blocks of the photo the default strip drops: those whose
    // finding the report names as removed, by offset.
    let parsed = parse(&before).expect("the JPEG parses");
    let removed: Vec<bool> = parsed
        .blocks
        .iter()
        .map(|block| {
            report
                .removed
                .iter()
                .any(|finding| finding.offset == block.range.start as u64)
        })
        .collect();
    let (before_blocks, _) = blocks(&before, &removed);
    let (after_blocks, _) = blocks(&after, &[]);

    // Every block kept is the photo's own bytes, in the same order: the
    // result is the photo's blocks, less those removed, concatenated.
    let kept: Vec<u8> = parsed
        .blocks
        .iter()
        .zip(&removed)
        .filter(|(_, gone)| !**gone)
        .flat_map(|(block, _)| before[block.range.clone()].to_vec())
        .collect();
    assert_eq!(
        kept, after,
        "the result is not the kept blocks, concatenated"
    );
    let decode = |b: &[u8]| {
        zune_jpeg::JpegDecoder::new(Cursor::new(b))
            .decode()
            .expect("decoded")
    };
    let (raster_before, raster_after) = (decode(&before), decode(&after));
    assert_eq!(raster_before, raster_after, "the pixels moved");

    let mut same_as = Vec::new();
    for (surface, path) in [
        ("cli", out.join("files/cli/coast.cleaned.jpg")),
        ("panel", out.join("files/panel/coast.cleaned.jpg")),
    ] {
        if let Ok(file) = std::fs::read(&path) {
            assert_eq!(file, after, "the {surface} result differs from the strip");
            same_as.push(serde_like::text(surface));
        }
    }

    let json = serde_like::object(&[
        (
            "how",
            serde_like::text(
                "wipemark-image's own JPEG parser (jpeg::parse, the blocks that tile the file) and wipemark_image::strip with the default options, the command line's and the panel's clean; written by crates/wipemark-image/src/footage_tests.rs",
            ),
        ),
        ("file", serde_like::text("coast.jpg (fixtures/image/c2pa-jumbf.jpg)")),
        ("bytes", serde_like::num(before.len() as u64)),
        ("sha256", serde_like::text(&hex(Sha256::digest(&before)))),
        ("blocks", serde_like::array(before_blocks)),
        (
            "result",
            serde_like::object(&[
                ("file", serde_like::text("coast.cleaned.jpg")),
                ("bytes", serde_like::num(after.len() as u64)),
                ("sha256", serde_like::text(&hex(Sha256::digest(&after)))),
                ("blocks", serde_like::array(after_blocks)),
                ("same_bytes_as", serde_like::array(same_as)),
            ]),
        ),
        (
            "raster",
            serde_like::object(&[
                ("decoder", serde_like::text("zune-jpeg")),
                ("sha256_before", serde_like::text(&hex(Sha256::digest(&raster_before)))),
                ("sha256_after", serde_like::text(&hex(Sha256::digest(&raster_after)))),
                ("identical", serde_like::boolean(raster_before == raster_after)),
            ]),
        ),
    ]);
    std::fs::write(out.join("blocks/coast.blocks.json"), json.render(0) + "\n")
        .expect("the JSON written");
    eprintln!(
        "coast.jpg: {} bytes, {} blocks; coast.cleaned.jpg: {} bytes",
        before.len(),
        removed.len(),
        after.len()
    );
}

/// A little JSON writer: this crate has no `serde_json`, and a test is no
/// reason to give it one.
mod serde_like {
    pub enum Value {
        Num(u64),
        Text(String),
        Bool(bool),
        Array(Vec<Value>),
        Object(Vec<(String, Value)>),
    }

    pub fn num(n: u64) -> Value {
        Value::Num(n)
    }

    pub fn text(s: &str) -> Value {
        Value::Text(s.to_owned())
    }

    pub fn boolean(b: bool) -> Value {
        Value::Bool(b)
    }

    pub fn array(items: Vec<Value>) -> Value {
        Value::Array(items)
    }

    pub fn object(fields: &[(&str, Value)]) -> Value {
        Value::Object(
            fields
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone_shallow()))
                .collect(),
        )
    }

    impl Value {
        fn clone_shallow(&self) -> Value {
            match self {
                Value::Num(n) => Value::Num(*n),
                Value::Text(s) => Value::Text(s.clone()),
                Value::Bool(b) => Value::Bool(*b),
                Value::Array(items) => {
                    Value::Array(items.iter().map(Value::clone_shallow).collect())
                }
                Value::Object(fields) => Value::Object(
                    fields
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone_shallow()))
                        .collect(),
                ),
            }
        }

        pub fn render(&self, depth: usize) -> String {
            let pad = "  ".repeat(depth + 1);
            let end = "  ".repeat(depth);
            match self {
                Value::Num(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Text(s) => {
                    let mut out = String::from("\"");
                    for c in s.chars() {
                        match c {
                            '"' => out.push_str("\\\""),
                            '\\' => out.push_str("\\\\"),
                            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                            c => out.push(c),
                        }
                    }
                    out.push('"');
                    out
                }
                Value::Array(items) if items.is_empty() => "[]".into(),
                Value::Array(items) => {
                    let inner: Vec<String> = items
                        .iter()
                        .map(|v| format!("{pad}{}", v.render(depth + 1)))
                        .collect();
                    format!("[\n{}\n{end}]", inner.join(",\n"))
                }
                Value::Object(fields) => {
                    let inner: Vec<String> = fields
                        .iter()
                        .map(|(k, v)| format!("{pad}\"{k}\": {}", v.render(depth + 1)))
                        .collect();
                    format!("{{\n{}\n{end}}}", inner.join(",\n"))
                }
            }
        }
    }
}
