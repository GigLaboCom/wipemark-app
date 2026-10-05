//! Visible marks on the command line, end to end (E12-5): `inspect`
//! reports one and exits 1; `clean` removes a proved one with no flag
//! (Q-V1) and exits by the input; a mark that cannot be proved is left,
//! said, and exits 3 with the result written; `audit` puts the rectangle
//! in SARIF `properties`.
//!
//! The marks are **real**: the owner's own Gemini stickers
//! (`fixtures/image/gemini/`, the bottom-right 1025 × 1025 of each, so the
//! vendor's mark is at its large row); the same sticker cut out of its
//! background, the mark under alpha 0, is the mark that cannot be proved. The pixels
//! suite's generators (borrowed by `#[path]`) make only what has no mark:
//! wallpapers, an animation, a damaged scan.

#[path = "../../../crates/wipemark-pixels/tests/support/mod.rs"]
mod pixels_support;

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use pixels_support::{aurora, picture, Kind};
use serde_json::Value;
use wipemark_pixels::{examine, Catalogue, ExamineOptions, Layout, Raster};

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
        self.run_in("en-US", arguments)
    }

    fn run_in(&self, language: &str, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_wipemark-cli"))
            .args(arguments)
            .current_dir(&self.0)
            .env("WIPEMARK_DATA_DIR", self.0.join("data"))
            .env("WIPEMARK_LANG", language)
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

/// A real Gemini output, from `fixtures/image/gemini/`.
fn real(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/image/gemini")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The vendor's mark at its large row (margin 64, the 96 map), on a
/// first-generation output — not `crying`, a re-saved copy whose restored
/// mark leaves an outline (`an_outline_left_is_said_and_exits_three`).
fn marked_png() -> Vec<u8> {
    real("torch-1025.png")
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
    // A real output: restored without an outline, with the map measured
    // from real outputs — so not claimed exact (D245), and the human report
    // says why, and how close, rather than blaming a loss a PNG never had.
    let restored = &answer["report"]["visible"]["restored"][0];
    assert_eq!(restored["outline_left"], Value::Bool(false));
    assert_eq!(restored["fitted"], Value::Bool(true));
    assert_eq!(restored["exact"], Value::Bool(false));
    assert!(said.contains("measured from real outputs"), "{said}");
    // How close is a mean, not a bound (D248).
    assert!(said.contains("on average"), "{said}");
    assert!(!said.contains("within"), "{said}");
    assert!(!said.contains("stored with loss"), "{said}");
    assert!(!said.contains("to within one level"), "{said}");
}

/// The vendor's mark over a saturated green, a lossless PNG: the inverse
/// leaves the range under it, the samples are clamped back to 0 — and
/// that is what the report says, not that the picture was stored with a
/// loss it never had (D245).
#[test]
fn a_clamped_restoration_says_it_clamped_not_that_it_was_lossy() {
    let scratch = Scratch::new("clamped");
    scratch.file("art.png", &real("anchor-green-1025.png"));
    let output = scratch.run(&["clean", "art.png"]);
    let said = stdout(&output);
    assert_eq!(code(&output), 1, "{said}");
    assert!(said.contains("were clamped"), "{said}");
    assert!(!said.contains("stored with loss"), "{said}");
}

/// `crying` is a re-saved copy over a flattened background: its mark is
/// proved and restored, and leaves an outline 3–4 levels dark on a
/// background with no spread — said, the mark counted as left, exit 3
/// (D244), the result written with what could be done.
#[test]
fn an_outline_left_is_said_and_exits_three() {
    let scratch = Scratch::new("outline");
    scratch.file("art.png", &real("crying-1025.png"));
    let output = scratch.run(&["clean", "art.png"]);
    let said = stdout(&output);
    assert_eq!(code(&output), 3, "{said}");
    assert!(said.contains("pixels restored"), "{said}");
    assert!(said.contains("An outline of the mark is left"), "{said}");
    let answer = json(&scratch.run(&["clean", "art.png", "-o", "j.png", "--json"]));
    assert_eq!(answer["report"]["marks_left"], Value::Bool(true));
    let restored = &answer["report"]["visible"]["restored"][0];
    assert_eq!(restored["outline_left"], Value::Bool(true));
    assert!(restored["step"].as_f64().unwrap() < -1.0, "{restored}");
}

/// The same sticker saved as most JPEGs are, 4:2:0: restored, and the
/// colour fringe the inverse leaves along the mark's edge — luma alone
/// calls it clean — is said, the mark counted as left, exit 3 (D247).
#[test]
fn a_fringe_left_in_colour_is_said_and_exits_three() {
    let scratch = Scratch::new("fringe");
    scratch.file("art.jpg", &real("torch-1025-q95-420.jpg"));
    let output = scratch.run(&["clean", "art.jpg"]);
    let said = stdout(&output);
    assert_eq!(code(&output), 3, "{said}");
    assert!(said.contains("An outline of the mark is left"), "{said}");
    let answer = json(&scratch.run(&["clean", "art.jpg", "-o", "j.jpg", "--json"]));
    let restored = &answer["report"]["visible"]["restored"][0];
    assert_eq!(restored["outline_left"], Value::Bool(true));
    assert!(restored["step"].as_f64().unwrap().abs() < 1.0, "{restored}");
    assert!(restored["chroma"].as_f64().unwrap() > 4.0, "{restored}");
    // The figure in the sentence is the farthest channel's — red, +11 —
    // not the luma's, which is under a level and would call it clean.
    let line = said
        .lines()
        .find(|line| line.contains("An outline of the mark is left"))
        .unwrap();
    let figure: f64 = line
        .split("on average ")
        .nth(1)
        .and_then(|rest| rest.split(' ').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("no figure: {line}"));
    assert!(figure > 8.0, "{line}");
}

/// The same picture saved as JPEG 4:4:4 at 95: no outline — its band is
/// within the bounds on average — but the codec's error, amplified by the
/// inverse, is an 8 × 8 checker along the mark's contour, three times as
/// rough as the picture around it. Said, as a percentile beside the same
/// around it, never a bound, in each language's decimals; the mark counts
/// as left and the exit is 3 (D250).
#[test]
fn a_texture_left_on_a_jpeg_is_said_and_exits_three() {
    let scratch = Scratch::new("texture");
    scratch.file("art.jpg", &real("torch-1025-q95-444.jpg"));
    let output = scratch.run(&["clean", "art.jpg"]);
    let said = stdout(&output);
    assert_eq!(code(&output), 3, "{said}");
    assert!(
        said.contains("A texture is left along the mark's edge"),
        "{said}"
    );
    assert!(!said.contains("An outline of the mark is left"), "{said}");
    let answer = json(&scratch.run(&["clean", "art.jpg", "-o", "j.jpg", "--json"]));
    let restored = &answer["report"]["visible"]["restored"][0];
    assert_eq!(restored["texture_left"], Value::Bool(true), "{restored}");
    assert_eq!(restored["outline_left"], Value::Bool(false), "{restored}");
    let (texture, around) = (
        restored["texture"].as_f64().unwrap(),
        restored["texture_around"].as_f64().unwrap(),
    );
    assert!(texture > 2.0 * around && texture > 8.0, "{restored}");
    for (language, grain) in [("ru", "зернистость"), ("de", "Körnung")] {
        let output = scratch.run_in(language, &["clean", "art.jpg", "-o", "out.jpg"]);
        let said = stdout(&output);
        assert_eq!(code(&output), 3, "{language}: {said}");
        let line = said
            .lines()
            .find(|line| line.contains(grain))
            .unwrap_or_else(|| panic!("{language}: {said}"));
        let digits: Vec<char> = line.chars().collect();
        let decimal = |mark: char| {
            digits
                .windows(3)
                .any(|w| w[0].is_ascii_digit() && w[1] == mark && w[2].is_ascii_digit())
        };
        assert!(decimal(',') && !decimal('.'), "{language}: {line}");
    }
}

/// In Russian and German the figure is a mean — "в среднем", "im Mittel"
/// — never a bound ("не больше чем", "höchstens"), and it is written
/// with the language's decimal comma (D248).
#[test]
fn the_figure_is_a_mean_in_every_language_with_its_own_decimals() {
    let scratch = Scratch::new("languages");
    scratch.file("crying.png", &real("crying-1025.png"));
    scratch.file("torch.png", &marked_png());
    for (language, mean, bound) in [
        ("ru", "в среднем", "не больше чем"),
        ("de", "im Mittel", "höchstens"),
    ] {
        for name in ["crying.png", "torch.png"] {
            let output = scratch.run_in(language, &["clean", name, "-o", "out.png"]);
            let said = stdout(&output);
            assert!(said.contains(mean), "{language} {name}: {said}");
            assert!(!said.contains(bound), "{language} {name}: {said}");
            let figure = said
                .lines()
                .find(|line| line.contains(mean))
                .expect("the line with the figure");
            let digits: Vec<char> = figure.chars().collect();
            let comma = digits
                .windows(3)
                .any(|w| w[0].is_ascii_digit() && w[1] == ',' && w[2].is_ascii_digit());
            let point = digits
                .windows(3)
                .any(|w| w[0].is_ascii_digit() && w[1] == '.' && w[2].is_ascii_digit());
            assert!(comma && !point, "{language} {name}: {figure}");
        }
    }
}

/// A real sticker cut out of its background: Gemini's mark is still in
/// its colour channels, under alpha 0. Seen, not proved — what the blend
/// meant under pixels nobody sees is unknown: the result is written with
/// what could be done, the report says so, and the exit is 3.
#[test]
fn a_mark_that_cannot_be_proved_is_left_and_exits_three() {
    let scratch = Scratch::new("left");
    scratch.file("art.png", &real("crying-transparent-1025.png"));
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

/// A sticker cut out of its background, confetti in its transparent
/// corner: no mark, and none reported — `inspect` exits 0.
#[test]
fn a_cut_out_sticker_is_clean() {
    let scratch = Scratch::new("cut-out");
    scratch.file("sticker.webp", &real("cut-out-confetti-256.webp"));
    let output = scratch.run(&["inspect", "sticker.webp"]);
    assert_eq!(code(&output), 0, "{}", stdout(&output));
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
    assert_eq!(result["properties"]["rect"]["width"], 96);
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
