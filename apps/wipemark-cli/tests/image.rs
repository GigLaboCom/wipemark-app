//! `wipemark-cli inspect`, `clean` and `audit` on a picture, end to end,
//! through the built binary — an exit code and two streams, as a hook sees
//! them.
//!
//! The pictures are `wipemark-image`'s own: the four real files under
//! `fixtures/image/` and the injected cases its `tests/support` builds,
//! borrowed by `#[path]` so the two suites test one set of files. The
//! decoders that judge whether a pixel moved are that module's too — and
//! share no code with the parser under test.

#[path = "../../../crates/wipemark-image/tests/support/mod.rs"]
mod support;

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;
use sha2::{Digest, Sha256};

/// A scratch directory, as `tests/cli.rs` has one: the process and the
/// thread in the name, so parallel tests never share one.
struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "wipemark-cli-image-{label}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch directory");
        Self(dir)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.path(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("folder");
        }
        std::fs::write(&path, bytes).expect("write");
        path
    }

    fn run(&self, arguments: &[&str]) -> Output {
        self.run_with(arguments, None)
    }

    fn run_with(&self, arguments: &[&str], stdin: Option<&[u8]>) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_wipemark-cli"));
        command
            .args(arguments)
            .current_dir(&self.0)
            .env("WIPEMARK_DATA_DIR", self.path("data"))
            .env("WIPEMARK_LANG", "en-US")
            .env_remove("WIPEMARK_LOG")
            .env_remove("RUST_LOG")
            .stdin(if stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().expect("the binary runs");
        if let Some(bytes) = stdin {
            use std::io::Write as _;
            child
                .stdin
                .take()
                .expect("a pipe")
                .write_all(bytes)
                .expect("stdin");
        }
        child.wait_with_output().expect("the binary finishes")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("an exit code, not a signal")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout is one JSON value")
}

fn sha(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// The extension a case's file is saved under — what a person's file
/// would be called. The bytes decide either way.
fn extension(bytes: &[u8]) -> &'static str {
    match bytes[0] {
        0x89 => "png",
        0xFF => "jpg",
        _ => "webp",
    }
}

/// Every case, as `name.ext` in the scratch directory.
fn cases(scratch: &Scratch) -> Vec<(String, support::Case)> {
    support::all()
        .into_iter()
        .map(|case| {
            let name = format!("{}.{}", case.name, extension(&case.bytes));
            scratch.file(&name, &case.bytes);
            (name, case)
        })
        .collect()
}

/// D131: `inspect` exits 1 on every AI signal and 0 on a picture whose
/// metadata is a camera's — EXIF, XMP, IPTC and a comment are not
/// findings.
#[test]
fn inspect_exits_one_on_each_ai_signal_and_zero_on_a_camera() {
    let scratch = Scratch::new("inspect-exit");
    let mut cameras = 0;
    for (name, case) in cases(&scratch) {
        let output = scratch.run(&["inspect", &name]);
        let expected = if case.ai { 1 } else { 0 };
        assert_eq!(code(&output), expected, "{name}: {}", stderr(&output));
        // The sentence about the pixels is the visible pass's now (E12-5):
        // they were examined for the marks this version knows.
        let report = stdout(&output);
        assert!(
            report.contains("The pixels were examined for the visible marks"),
            "{name}: the pixels are not mentioned:\n{report}"
        );
        if !case.ai {
            cameras += 1;
        }
    }
    assert!(cameras >= 4, "the camera-only cases moved");
}

/// `--json` is one line of ASCII that parses, carries the verdict the exit
/// code was read from, the visible pass, and the picture's third shelf.
#[test]
fn inspect_json_is_ascii_parses_and_carries_the_third_shelf() {
    let scratch = Scratch::new("inspect-json");
    for (name, case) in cases(&scratch) {
        let output = scratch.run(&["inspect", &name, "--json"]);
        assert!(output.stdout.is_ascii(), "{name}");
        assert!(output.stdout.ends_with(b"}\n"), "{name}: one line");
        let report = json(&output);
        assert_eq!(report["ai_metadata"], Value::Bool(case.ai), "{name}");
        let shelf: Vec<&str> = report["not_established"]
            .as_array()
            .expect("the shelf")
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert!(shelf.contains(&"unknown-mark-schemes"), "{name}: {shelf:?}");
        // A picture's shelf: invisible marks in the pixels first, then
        // core's three (E12-5).
        assert_eq!(shelf.first(), Some(&"invisible-pixel-marks"), "{name}");
        assert_eq!(shelf.len(), 4, "{name}");
        // And the visible pass, examined: the shipped catalogue loads.
        assert_eq!(report["visible"]["examined"], Value::Bool(true), "{name}");
    }
}

/// `clean` writes `name.cleaned.ext` beside the input and does not touch
/// the input; the result carries no AI provenance (it inspects at 0); and
/// the exit code is the input's, as a text's is (D132).
#[test]
fn clean_writes_beside_the_file_and_leaves_it_untouched() {
    let scratch = Scratch::new("clean-beside");
    for (name, case) in cases(&scratch) {
        let before = sha(&case.bytes);
        let output = scratch.run(&["clean", &name]);
        assert_eq!(
            code(&output),
            if case.ai { 1 } else { 0 },
            "{name}: {}",
            stderr(&output)
        );
        let input = std::fs::read(scratch.path(&name)).expect("the input");
        assert_eq!(sha(&input), before, "{name}: the input changed");
        let (stem, ext) = name.rsplit_once('.').expect("an extension");
        let result = format!("{stem}.cleaned.{ext}");
        let cleaned = std::fs::read(scratch.path(&result)).expect("the result");
        if !case.ai {
            assert_eq!(
                cleaned, case.bytes,
                "{name}: nothing to remove, yet bytes moved"
            );
        }
        assert_eq!(code(&scratch.run(&["inspect", &result])), 0, "{result}");
    }
}

/// OV §11 through the surface: the result's decoded raster is the
/// input's, under both scopes, for every case.
#[test]
fn the_pixels_of_a_cleaned_image_are_the_pixels_it_had() {
    let scratch = Scratch::new("pixels");
    for (name, case) in cases(&scratch) {
        let raster = support::raster(&case.bytes);
        for flags in [&[][..], &["--all-metadata"][..]] {
            let out = format!("out-{name}");
            let mut arguments = vec!["clean", name.as_str(), "-o", out.as_str()];
            arguments.extend_from_slice(flags);
            let output = scratch.run(&arguments);
            assert!(code(&output) <= 1, "{name} {flags:?}: {}", stderr(&output));
            let cleaned = std::fs::read(scratch.path(&out)).expect("the result");
            assert_eq!(support::raster(&cleaned), raster, "{name} {flags:?}");
            assert_eq!(
                support::image_data(&cleaned),
                support::image_data(&case.bytes),
                "{name} {flags:?}"
            );
        }
    }
}

/// `--in-place` sets the original aside first, byte for byte, and refuses
/// — with 2, touching neither file — when an original is already there.
#[test]
fn in_place_sets_the_original_aside_and_never_overwrites_one() {
    let scratch = Scratch::new("in-place");
    let marked = support::all()
        .into_iter()
        .find(|case| case.name == "png-parameters")
        .expect("the case");
    scratch.file("art.png", &marked.bytes);

    let output = scratch.run(&["clean", "art.png", "--in-place"]);
    assert_eq!(code(&output), 1, "{}", stderr(&output));
    assert_eq!(
        std::fs::read(scratch.path("art.original.png")).expect("set aside"),
        marked.bytes
    );
    let replaced = std::fs::read(scratch.path("art.png")).expect("replaced");
    assert_ne!(replaced, marked.bytes);
    assert_eq!(code(&scratch.run(&["inspect", "art.png"])), 0);

    // Marked again, with the first original still there: refused.
    scratch.file("art.png", &marked.bytes);
    let output = scratch.run(&["clean", "art.png", "--in-place"]);
    assert_eq!(code(&output), 2, "{}", stderr(&output));
    assert!(
        stderr(&output).contains("art.original.png"),
        "{}",
        stderr(&output)
    );
    assert_eq!(
        std::fs::read(scratch.path("art.png")).expect("as it was"),
        marked.bytes
    );
    assert_eq!(
        std::fs::read(scratch.path("art.original.png")).expect("the first original"),
        marked.bytes
    );
}

/// `--no-original` replaces the file and keeps no copy.
#[test]
fn no_original_keeps_no_copy() {
    let scratch = Scratch::new("no-original");
    let marked = support::all()
        .into_iter()
        .find(|case| case.name == "jpeg-c2pa")
        .expect("the case");
    scratch.file("shot.jpg", &marked.bytes);
    let output = scratch.run(&["clean", "shot.jpg", "--in-place", "--no-original"]);
    assert_eq!(code(&output), 1, "{}", stderr(&output));
    assert!(!scratch.path("shot.original.jpg").exists());
    assert_eq!(code(&scratch.run(&["inspect", "shot.jpg"])), 0);
}

/// The kinds left in a result, by `inspect --json` of it.
fn kinds_in(scratch: &Scratch, name: &str) -> Vec<String> {
    let report = json(&scratch.run(&["inspect", name, "--json"]));
    report["findings"]
        .as_array()
        .expect("findings")
        .iter()
        .filter_map(|finding| finding["kind"].as_str().map(str::to_owned))
        .collect()
}

/// The default scope keeps a camera's EXIF; `--all-metadata` removes it
/// and keeps the colour profile. The rotation sentence is said only when
/// the EXIF that went carried a rotation (E11-3): not for the fixture
/// camera, which has none, and for a camera picture turned on its side.
#[test]
fn all_metadata_removes_exif_and_keeps_colour() {
    let scratch = Scratch::new("all-metadata");
    let camera = support::all()
        .into_iter()
        .find(|case| case.name == "jpeg-camera")
        .expect("the case");
    scratch.file("camera.jpg", &camera.bytes);
    let rotation = "The picture's rotation was in the removed camera data";

    let output = scratch.run(&["clean", "camera.jpg", "-o", "kept.jpg"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(kinds_in(&scratch, "kept.jpg").contains(&"exif".to_owned()));

    let output = scratch.run(&["clean", "camera.jpg", "-o", "bare.jpg", "--all-metadata"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(!stdout(&output).contains(rotation), "{}", stdout(&output));
    let left = kinds_in(&scratch, "bare.jpg");
    assert!(!left.contains(&"exif".to_owned()), "{left:?}");
    assert_eq!(
        left,
        vec!["rendering".to_owned()],
        "colour stays, nothing else"
    );

    // Turned on its side (Orientation 6): the rotation goes with the EXIF,
    // and the report says so, in words and in `--json`.
    let turned = support::jpeg_with(
        &support::tiny_jpeg(),
        &[support::app1_exif(&support::exif_oriented(false, 6, b""))],
    );
    scratch.file("turned.jpg", &turned);
    let output = scratch.run(&["clean", "turned.jpg", "-o", "t.jpg", "--all-metadata"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(stdout(&output).contains(rotation), "{}", stdout(&output));
    let answer = json(&scratch.run(&[
        "clean",
        "turned.jpg",
        "-o",
        "t2.jpg",
        "--all-metadata",
        "--json",
    ]));
    assert_eq!(answer["report"]["orientation_removed"], 6, "{answer}");
}

/// D134: a flag for text on a picture, and `--all-metadata` on a text,
/// are usage errors — 2, nothing written.
#[test]
fn a_flag_for_the_other_kind_is_a_usage_error() {
    let scratch = Scratch::new("flags");
    scratch.file("x.png", &support::tiny_png());
    scratch.file("note.md", b"plain words\n");
    for (arguments, result) in [
        (&["clean", "x.png", "--aggressive"][..], "x.cleaned.png"),
        (&["clean", "x.png", "--nfkc"][..], "x.cleaned.png"),
        (
            &["clean", "note.md", "--all-metadata"][..],
            "note.cleaned.md",
        ),
    ] {
        let output = scratch.run(arguments);
        assert_eq!(code(&output), 2, "{arguments:?}: {}", stderr(&output));
        assert!(output.stdout.is_empty(), "{arguments:?}");
        assert!(
            !scratch.path(result).exists(),
            "{arguments:?} wrote {result}"
        );
    }
}

/// TIFF is recognised and refused by name — "not in this version yet" —
/// with 2, by both commands; the epic number never leaves the repository.
#[test]
fn a_tiff_is_refused_by_name() {
    let scratch = Scratch::new("tiff");
    scratch.file(
        "scan.tif",
        b"II*\x00\x08\x00\x00\x00\x00\x00\x00\x00\x00\x00",
    );
    for command in ["inspect", "clean"] {
        let output = scratch.run(&[command, "scan.tif"]);
        assert_eq!(code(&output), 2, "{command}: {}", stderr(&output));
        let said = stderr(&output);
        assert!(said.contains("TIFF"), "{said}");
        assert!(said.contains("not in this version yet"), "{said}");
        assert!(!said.contains("E11"), "{said}");
        assert!(output.stdout.is_empty());
    }
    assert!(!scratch.path("scan.cleaned.tif").exists());
}

/// A JPEG cut short is not one this version can read — named, with the
/// defect from the catalogue — and not read is not clean: 3 (D136).
#[test]
fn a_truncated_jpeg_is_not_read_and_not_clean() {
    let scratch = Scratch::new("truncated");
    let jpeg = support::tiny_jpeg();
    scratch.file("cut.jpg", &jpeg[..jpeg.len() / 2]);
    for command in ["inspect", "clean"] {
        let output = scratch.run(&[command, "cut.jpg"]);
        assert_eq!(code(&output), 3, "{command}: {}", stderr(&output));
        let said = stderr(&output);
        assert!(
            said.contains("is not a JPEG file this version can read"),
            "{said}"
        );
        assert!(said.contains("Not read is not clean"), "{said}");
        assert!(output.stdout.is_empty());
    }
    assert!(!scratch.path("cut.cleaned.jpg").exists());
}

/// Standard input in, the image out on standard output — which is a pipe
/// here, not a terminal — and the report beside it on stderr. With
/// `--json` the two would share stdout, and that is refused.
#[test]
fn a_picture_on_standard_input_goes_to_standard_output() {
    let scratch = Scratch::new("stdin");
    let marked = support::all()
        .into_iter()
        .find(|case| case.name == "webp-xmp-dst")
        .expect("the case");
    let output = scratch.run_with(&["clean", "-"], Some(&marked.bytes));
    assert_eq!(code(&output), 1, "{}", stderr(&output));
    assert_eq!(
        support::raster(&output.stdout),
        support::raster(&marked.bytes)
    );
    assert!(output.stdout.len() < marked.bytes.len());
    assert!(stderr(&output).contains("Removed:"), "{}", stderr(&output));

    let output = scratch.run_with(&["clean", "-", "--json"], Some(&marked.bytes));
    assert_eq!(code(&output), 2, "{}", stderr(&output));
    assert!(output.stdout.is_empty());
}

/// `clean --json` to a file: the strip's report, read off the output, and
/// where the result went.
#[test]
fn clean_json_says_what_the_output_still_carries() {
    let scratch = Scratch::new("clean-json");
    let marked = support::all()
        .into_iter()
        .find(|case| case.name == "png-c2pa")
        .expect("the case");
    scratch.file("m.png", &marked.bytes);
    let output = scratch.run(&["clean", "m.png", "--json"]);
    assert_eq!(code(&output), 1, "{}", stderr(&output));
    assert!(output.stdout.is_ascii());
    let answer = json(&output);
    assert_eq!(answer["report"]["still_has_c2pa"], Value::Bool(false));
    assert_eq!(
        answer["report"]["still_has_ai_metadata"],
        Value::Bool(false)
    );
    assert_eq!(answer["report"]["removed"][0]["kind"], "c2pa");
    assert!(answer["written"]
        .as_str()
        .is_some_and(|path| path.ends_with("m.cleaned.png")));
}

/// The bytes decide: a PNG called `holiday.txt` is read as a PNG, and the
/// name's lie is said on stderr.
#[test]
fn a_picture_whose_name_lies_is_read_by_its_bytes() {
    let scratch = Scratch::new("name-lies");
    let marked = support::all()
        .into_iter()
        .find(|case| case.name == "png-parameters")
        .expect("the case");
    scratch.file("holiday.txt", &marked.bytes);
    let output = scratch.run(&["inspect", "holiday.txt"]);
    assert_eq!(code(&output), 1, "{}", stderr(&output));
    assert!(stderr(&output).contains("PNG"), "{}", stderr(&output));
}

/// Build a tree with a marked picture, a camera's, a text and, when
/// `broken`, a JPEG cut short.
fn tree(scratch: &Scratch, broken: bool) -> PathBuf {
    let all = support::all();
    let case = |name: &str| {
        all.iter()
            .find(|case| case.name == name)
            .expect("the case")
            .bytes
            .clone()
    };
    scratch.file("tree/art/a.png", &case("png-comfyui"));
    scratch.file("tree/b.jpg", &case("jpeg-camera"));
    scratch.file("tree/c.md", b"plain words\n");
    scratch.file("tree/old.tif", b"II*\x00\x08\x00\x00\x00\x00\x00\x00\x00");
    if broken {
        let jpeg = support::tiny_jpeg();
        scratch.file("tree/d.jpg", &jpeg[..jpeg.len() / 2]);
    }
    scratch.path("tree")
}

/// `audit` inspects the pictures too, in all three outputs; a picture with
/// AI provenance is a finding (1), a picture that could not be read is a
/// hole (3, which beats 1), and a TIFF is skipped by name.
#[test]
fn audit_lists_pictures_in_every_output() {
    let scratch = Scratch::new("audit");
    tree(&scratch, false);

    let human = scratch.run(&["audit", "tree"]);
    assert_eq!(code(&human), 1, "{}", stderr(&human));
    let said = stdout(&human);
    assert!(
        said.contains("art/a.png: PNG, 2 blocks of AI provenance"),
        "{said}"
    );
    assert!(
        !said.contains("b.jpg"),
        "a camera's picture is not a finding:\n{said}"
    );
    assert!(said.contains("scanned 3"), "{said}");

    let listing = json(&scratch.run(&["audit", "tree", "--json"]));
    let files = listing["files"].as_array().expect("files");
    let entry = |path: &str| {
        files
            .iter()
            .find(|file| file["path"] == path)
            .unwrap_or_else(|| panic!("{path} not listed: {listing}"))
            .clone()
    };
    assert_eq!(entry("art/a.png")["status"], "scanned");
    assert_eq!(entry("art/a.png")["report"]["container"], "png");
    assert_eq!(entry("art/a.png")["report"]["ai_metadata"], true);
    assert_eq!(entry("b.jpg")["report"]["ai_metadata"], false);
    assert_eq!(entry("old.tif")["status"], "skipped");
    assert_eq!(entry("old.tif")["reason"], "image-not-yet");

    let sarif = json(&scratch.run(&["audit", "tree", "--sarif"]));
    let run = &sarif["runs"][0];
    let results = run["results"].as_array().expect("results");
    let pictures: Vec<&Value> = results
        .iter()
        .filter(|result| {
            result["ruleId"]
                .as_str()
                .is_some_and(|id| id.starts_with("image-"))
        })
        .collect();
    assert!(!pictures.is_empty(), "{sarif}");
    for result in &pictures {
        let location = &result["locations"][0]["physicalLocation"];
        assert_eq!(location["artifactLocation"]["uri"], "art/a.png");
        assert!(location["region"]["byteOffset"].is_u64(), "{result}");
        assert!(location["region"]["byteLength"].as_u64().unwrap_or(0) > 0);
        assert!(location["region"].get("startLine").is_none());
        let index = result["ruleIndex"].as_u64().expect("an index") as usize;
        assert_eq!(
            run["tool"]["driver"]["rules"][index]["id"],
            result["ruleId"]
        );
    }
}

/// A picture that could not be read makes the walk inconclusive: 3, even
/// with a finding beside it — in every output.
#[test]
fn an_unreadable_picture_makes_the_audit_inconclusive() {
    let scratch = Scratch::new("audit-broken");
    tree(&scratch, true);
    for flag in [None, Some("--json"), Some("--sarif")] {
        let mut arguments = vec!["audit", "tree"];
        arguments.extend(flag);
        let output = scratch.run(&arguments);
        assert_eq!(code(&output), 3, "{flag:?}: {}", stderr(&output));
    }
    let said = stdout(&scratch.run(&["audit", "tree"]));
    assert!(said.contains("d.jpg is not a JPEG file"), "{said}");
    let listing = json(&scratch.run(&["audit", "tree", "--json"]));
    let broken = listing["files"]
        .as_array()
        .expect("files")
        .iter()
        .find(|file| file["path"] == "d.jpg")
        .expect("listed")
        .clone();
    assert_eq!(broken["status"], "unreadable");
    assert_eq!(broken["reason"], "malformed-image");
    let sarif = json(&scratch.run(&["audit", "tree", "--sarif"]));
    assert_eq!(
        sarif["runs"][0]["invocations"][0]["executionSuccessful"],
        false
    );
}

/// What a report says must not depend on where the CLI ran from: the
/// fixtures exist where `support` looks for them.
#[test]
fn the_fixtures_are_where_support_looks() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/image");
    for name in support::REAL {
        assert!(dir.join(name).is_file(), "{name}");
    }
}
