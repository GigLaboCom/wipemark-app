//! Visible marks on the command line, end to end (E12-5): `inspect`
//! reports one and exits 1; `clean` removes a proved one with no flag
//! (Q-V1) and exits by the input; a mark that cannot be proved is left,
//! said, and exits 3 with the result written; `audit` puts the rectangle
//! in SARIF `properties`.
//!
//! The marks are the shipped Gemini sparkle, composited onto generated
//! pictures with the pixels suite's generators (borrowed by `#[path]`).
//! The shipped maps must be in the tree (`crates/wipemark-pixels/marks/README.md`).

#[path = "../../../crates/wipemark-pixels/tests/support/mod.rs"]
mod pixels_support;

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use pixels_support::{aurora, composite_at, picture, scaled, Kind};
use serde_json::Value;
use wipemark_pixels::{examine, AlphaMap, Catalogue, ExamineOptions, Layout, PixelRect, Raster};

const W: u32 = 320;
const H: u32 = 240;

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "wipemark-cli-visible-{label}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch directory");
        Self(dir)
    }

    fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("folder");
        }
        std::fs::write(&path, bytes).expect("write");
        path
    }

    fn run(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_wipemark-cli"))
            .args(arguments)
            .current_dir(&self.0)
            .env("WIPEMARK_DATA_DIR", self.0.join("data"))
            .env("WIPEMARK_LANG", "en-US")
            .env_remove("WIPEMARK_LOG")
            .stdin(Stdio::null())
            .output()
            .expect("the binary runs")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("an exit code")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout is one JSON value")
}

fn png_of(raster: &Raster) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, raster.width(), raster.height());
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().unwrap();
        let bytes: Vec<u8> = raster.samples().iter().map(|&s| s as u8).collect();
        w.write_image_data(&bytes).unwrap();
        w.finish().unwrap();
    }
    out
}

/// Gemini's small sparkle and where its row puts it in a `W × H` picture.
fn gemini_small() -> (AlphaMap, PixelRect) {
    let catalogue = Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"));
    let profile = catalogue.profile("gemini-sparkle-v1").expect("the profile");
    let (_, map) = profile
        .maps
        .iter()
        .find(|(id, _)| id == "gemini-v1-48")
        .expect("the 48 map");
    let at = PixelRect {
        x: W - 32 - 48,
        y: H - 32 - 48,
        width: 48,
        height: 48,
    };
    (map.clone(), at)
}

fn marked_png() -> Vec<u8> {
    let (map, at) = gemini_small();
    let mut raster = picture(Kind::Fractal, W, H, 7, Layout::Rgb8);
    composite_at(&mut raster, &map, at);
    png_of(&raster)
}

#[test]
fn inspect_reports_a_visible_mark_and_exits_one() {
    let scratch = Scratch::new("inspect");
    scratch.file("art.png", &marked_png());
    let output = scratch.run(&["inspect", "art.png"]);
    assert_eq!(code(&output), 1, "{}", stdout(&output));
    let said = stdout(&output);
    assert!(said.contains("Visible marks"), "{said}");
    assert!(
        said.contains("gemini-sparkle-v1 (google, gemini)"),
        "{said}"
    );
    assert!(said.contains("proved:"), "{said}");
    assert!(
        said.contains("invisible marks in the picture's pixels"),
        "{said}"
    );

    let report = json(&scratch.run(&["inspect", "art.png", "--json"]));
    assert_eq!(report["ai_metadata"], Value::Bool(false));
    assert_eq!(report["visible"]["examined"], Value::Bool(true));
    let found = &report["visible"]["found"][0];
    assert_eq!(found["profile"], "gemini-sparkle-v1");
    assert_eq!(found["verdict"], "verified");
    assert_eq!(found["placed"], "row");
}

/// No flag (Q-V1): `clean` removes a proved mark, exits 1 because the
/// input carried one, and the result inspects at 0.
#[test]
fn clean_removes_a_proved_mark_with_no_flag() {
    let scratch = Scratch::new("clean");
    scratch.file("art.png", &marked_png());
    let output = scratch.run(&["clean", "art.png"]);
    assert_eq!(code(&output), 1, "{}", stdout(&output));
    let said = stdout(&output);
    assert!(said.contains("pixels restored"), "{said}");
    assert!(said.contains("The PNG was written again"), "{said}");
    let again = scratch.run(&["inspect", "art.cleaned.png"]);
    assert_eq!(code(&again), 0, "{}", stdout(&again));

    let answer = json(&scratch.run(&["clean", "art.png", "-o", "j.png", "--json"]));
    assert_eq!(answer["report"]["marks_left"], Value::Bool(false));
    assert_eq!(answer["report"]["encoding"]["kind"], "png");
    assert_eq!(
        answer["report"]["visible"]["restored"][0]["exact"],
        Value::Bool(true)
    );
}

/// The mark at 0.8 of its opacity is a blend the second proof will not
/// accept (its edges vanish at that gain, not at 1): it is seen and not
/// proved, the result is written with what could be done, said, and the
/// exit is 3.
#[test]
fn a_mark_that_cannot_be_proved_is_left_and_exits_three() {
    let scratch = Scratch::new("left");
    let (map, at) = gemini_small();
    let mut raster = picture(Kind::Gradient, W, H, 9, Layout::Rgb8);
    composite_at(&mut raster, &scaled(&map, 0.8), at);
    scratch.file("art.png", &png_of(&raster));
    let output = scratch.run(&["clean", "art.png"]);
    let said = stdout(&output);
    assert_eq!(code(&output), 3, "{said}");
    assert!(said.contains("seen, not proved"), "{said}");
    assert!(said.contains("is still in the result"), "{said}");
    assert!(
        scratch.0.join("art.cleaned.png").exists(),
        "the result was not written"
    );
}

/// A night-sky wallpaper — soft bright curtains and stars in the corner a
/// sparkle is looked for in, what a non-AI desktop picture is — carries no
/// mark: `inspect` and `clean` exit 0 and say nothing about one (D235).
/// Only skies the correlation does propose a sparkle on are used, or the
/// test would pass without the second proof ever being asked.
#[test]
fn a_night_sky_wallpaper_is_clean() {
    let scratch = Scratch::new("aurora");
    let catalogue = Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"));
    let skies: Vec<(u64, Raster)> = (0..200)
        .map(|seed| (seed, aurora(W * 2, H * 2, seed, Layout::Rgb8)))
        .filter(|(_, sky)| examine(sky, catalogue, &ExamineOptions::default()).dismissed > 0)
        .take(4)
        .collect();
    assert_eq!(skies.len(), 4, "too few skies were proposed on");
    for (seed, sky) in skies {
        let name = format!("sky{seed}.png");
        scratch.file(&name, &png_of(&sky));
        let output = scratch.run(&["inspect", &name]);
        assert_eq!(code(&output), 0, "{name}: {}", stdout(&output));
        let report = json(&scratch.run(&["inspect", &name, "--json"]));
        assert_eq!(
            report["visible"]["found"],
            Value::Array(Vec::new()),
            "{name}"
        );
        let output = scratch.run(&["clean", &name]);
        assert_eq!(code(&output), 0, "{name}: {}", stdout(&output));
    }
}

/// A restored picture's second look finds nothing: `clean` exits 1 for
/// the mark the input carried — not 3 — no finding of the second pass is
/// reported, and the result inspects at 0.
#[test]
fn a_restored_picture_leaves_nothing_for_the_second_pass() {
    let scratch = Scratch::new("second");
    scratch.file("art.png", &marked_png());
    let answer = json(&scratch.run(&["clean", "art.png", "--json"]));
    let found = answer["report"]["visible"]["found"]
        .as_array()
        .expect("found");
    assert_eq!(found.len(), 1, "{answer}");
    assert_eq!(found[0]["pass"], 1);
    assert_eq!(answer["report"]["marks_left"], Value::Bool(false));
    let output = scratch.run(&["clean", "art.png", "-o", "again.png"]);
    assert_eq!(code(&output), 1, "{}", stdout(&output));
    let again = scratch.run(&["inspect", "again.png"]);
    assert_eq!(code(&again), 0, "{}", stdout(&again));
}

/// An animated PNG's frames are not examined: `inspect` and `clean` exit
/// 3 — not examined is not clean — and `clean` still writes the result
/// with its metadata cleaned, and says why the pixels were not looked at.
#[test]
fn an_animation_is_not_examined_and_exits_three() {
    let scratch = Scratch::new("animated");
    let mut bytes = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut bytes, 8, 8);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_animated(1, 0).unwrap();
        let mut w = enc.write_header().unwrap();
        w.write_image_data(&[90u8; 8 * 8 * 3]).unwrap();
        w.finish().unwrap();
    }
    scratch.file("moving.png", &bytes);
    let output = scratch.run(&["inspect", "moving.png"]);
    assert_eq!(code(&output), 3, "{}", stdout(&output));
    assert!(
        stdout(&output).contains("its frames were not examined"),
        "{}",
        stdout(&output)
    );
    let output = scratch.run(&["clean", "moving.png"]);
    assert_eq!(code(&output), 3, "{}", stdout(&output));
    assert!(scratch.0.join("moving.cleaned.png").exists());
    let answer = json(&scratch.run(&["clean", "moving.png", "-o", "m.png", "--json"]));
    assert_eq!(answer["report"]["visible"]["why"], "animated", "{answer}");
}

/// A JPEG whose scan is damaged — a code near its start flipped — is not
/// decoded: the decoder would fill in what it cannot read and say nothing.
/// `clean` cleans the metadata, writes the result, says the pixels could
/// not be decoded, and exits 3 (`inspect` too): not read is not clean.
#[test]
fn a_damaged_jpeg_scan_is_not_examined_and_exits_three() {
    let scratch = Scratch::new("damaged");
    let raster = picture(Kind::Gradient, W, H, 3, Layout::Rgb8);
    let bytes: Vec<u8> = raster.samples().iter().map(|&s| s as u8).collect();
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 95)
        .encode(&bytes, W, H, image::ExtendedColorType::Rgb8)
        .unwrap();
    let mut pos = 2;
    let scan = loop {
        let len = usize::from(u16::from_be_bytes([jpeg[pos + 2], jpeg[pos + 3]]));
        if jpeg[pos + 1] == 0xDA {
            break pos + 2 + len;
        }
        pos += 2 + len;
    };
    // The first flip near the start of the scan the walk reports — a code
    // that no longer reads as the one written (a flip in a coefficient's
    // own bits changes a value and is indistinguishable from the picture).
    let damaged = (scan..scan + 64)
        .filter(|&at| jpeg[at] != 0xFF && jpeg[at - 1] != 0xFF && jpeg[at] ^ 0x80 != 0xFF)
        .map(|at| {
            let mut b = jpeg.clone();
            b[at] ^= 0x80;
            b
        })
        .find(|b| wipemark_picture::walk_jpeg_scan(b) == wipemark_picture::Scan::Damaged)
        .expect("a flip the walk reports");
    scratch.file("damaged.jpg", &damaged);
    let output = scratch.run(&["inspect", "damaged.jpg"]);
    assert_eq!(code(&output), 3, "{}", stdout(&output));
    let output = scratch.run(&["clean", "damaged.jpg"]);
    let said = stdout(&output);
    assert_eq!(code(&output), 3, "{said}");
    assert!(said.contains("could not be decoded"), "{said}");
    assert!(scratch.0.join("damaged.cleaned.jpg").exists());
}

/// `audit`: a picture with a visible mark is a finding, and SARIF carries
/// its rectangle in `properties`, not as a byte region.
#[test]
fn audit_puts_a_visible_mark_in_sarif_properties() {
    let scratch = Scratch::new("audit");
    scratch.file("tree/art.png", &marked_png());
    let human = scratch.run(&["audit", "tree"]);
    assert_eq!(code(&human), 1, "{}", stdout(&human));
    assert!(
        stdout(&human).contains("art.png: a visible mark (gemini-sparkle-v1)"),
        "{}",
        stdout(&human)
    );

    let sarif = json(&scratch.run(&["audit", "tree", "--sarif"]));
    let run = &sarif["runs"][0];
    let result = run["results"]
        .as_array()
        .expect("results")
        .iter()
        .find(|r| r["ruleId"] == "visible-gemini-sparkle-v1")
        .unwrap_or_else(|| panic!("no visible result: {sarif}"));
    assert_eq!(result["properties"]["rect"]["width"], 48);
    assert!(result["locations"][0]["physicalLocation"]
        .get("region")
        .is_none());
    let index = result["ruleIndex"].as_u64().expect("an index") as usize;
    assert_eq!(
        run["tool"]["driver"]["rules"][index]["id"],
        result["ruleId"]
    );
    assert_eq!(
        run["properties"]["not_established"][0],
        "invisible-pixel-marks"
    );
}
