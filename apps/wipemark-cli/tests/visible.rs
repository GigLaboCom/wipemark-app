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

use pixels_support::{composite_at, picture, stamp_opaque, Kind};
use serde_json::Value;
use wipemark_pixels::{AlphaMap, Catalogue, Layout, PixelRect, Raster};

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

/// A look-alike — the sparkle drawn opaque — is seen and not proved: the
/// result is written with what could be done, said, and the exit is 3.
#[test]
fn a_mark_that_cannot_be_proved_is_left_and_exits_three() {
    let scratch = Scratch::new("left");
    let (map, at) = gemini_small();
    let mut raster = picture(Kind::Gradient, W, H, 9, Layout::Rgb8);
    stamp_opaque(&mut raster, &map, at, 0.2, 255.0);
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
