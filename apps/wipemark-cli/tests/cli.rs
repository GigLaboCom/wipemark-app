//! `wipemark-cli inspect` and `clean`, end to end, through the built
//! binary.
//!
//! The binary crate has no library target, so nothing here can call its
//! functions — which is the point: these tests see exactly what a
//! pre-commit hook sees, an exit code and two streams. Every run gets a
//! scratch data directory (logs and the settings database stay inside
//! it), English (`WIPEMARK_LANG=en-US`) and no stderr mirror of the log.
//! No test touches the network; no test opens a window.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

/// A scratch directory that takes its own files away with it — the
/// `Scratch` idiom of `wipemark-intake`'s tests: the process and the
/// thread in the name, so parallel tests never share one.
struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "wipemark-cli-{label}-{}-{:?}",
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
        std::fs::write(&path, bytes).expect("write");
        path
    }

    /// The data directory a run is pointed at, beside the files.
    fn data(&self) -> PathBuf {
        self.path("data")
    }

    /// Run the binary in this directory with these arguments.
    fn run(&self, arguments: &[&str]) -> Output {
        self.run_with(arguments, None)
    }

    fn run_with(&self, arguments: &[&str], stdin: Option<&[u8]>) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_wipemark-cli"));
        command
            .args(arguments)
            .current_dir(&self.0)
            .env("WIPEMARK_DATA_DIR", self.data())
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

/// The headline case of A §8: a file with a zero-width space is marked,
/// `clean` says so too — by the input, not by its result — and the
/// result it wrote is not.
#[test]
fn a_file_with_a_zero_width_space_exits_one() {
    let scratch = Scratch::new("zwsp");
    scratch.file("note.md", "Hello\u{200B}world\n".as_bytes());

    assert_eq!(code(&scratch.run(&["inspect", "note.md"])), 1);
    let cleaned = scratch.run(&["clean", "note.md"]);
    assert_eq!(code(&cleaned), 1, "{}", stderr(&cleaned));
    assert_eq!(
        std::fs::read(scratch.path("note.cleaned.md")).expect("the result"),
        b"Helloworld\n"
    );
    assert_eq!(code(&scratch.run(&["inspect", "note.cleaned.md"])), 0);
}

/// Not read is not clean. Every one of these is a file nobody read, and
/// none of them may exit 0 — or put anything on stdout a parser would
/// take for an answer.
#[test]
fn an_unreadable_encoding_exits_three_not_zero() {
    let scratch = Scratch::new("unreadable");
    let mut long = vec![b'a'; 5000];
    long.push(0xFF);
    let png = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01".to_vec();
    for (name, bytes, says) in [
        (
            "koi8.txt",
            b"\xf0\xd2\xc9\xd7\xc5\xd4 hello".to_vec(),
            "8-bit",
        ),
        ("x.txt", png, "PNG"),
        ("long.txt", long, "byte 5000"),
        ("odd.txt", b"\xff\xfea\x00b".to_vec(), "UTF-16LE"),
    ] {
        scratch.file(name, &bytes);
        for command in ["inspect", "clean"] {
            let output = scratch.run(&[command, name]);
            assert_eq!(code(&output), 3, "{command} {name}: {}", stderr(&output));
            assert!(
                output.stdout.is_empty(),
                "{command} {name}: {}",
                stdout(&output)
            );
            assert!(
                stderr(&output).contains(says),
                "{command} {name}: {}",
                stderr(&output)
            );
        }
        assert!(
            !scratch.path(&name.replace(".txt", ".cleaned.txt")).exists(),
            "{name}: a result for a file that was not read"
        );
    }
}

/// The Retention rule, from the command line: beside, never over. Every
/// way of naming the input as `-o` is refused, and the input's bytes and
/// permissions survive every run.
#[test]
fn clean_writes_beside_the_file_and_never_over_it() {
    let scratch = Scratch::new("beside");
    let input = scratch.file("note.md", "a\u{200B}b\n".as_bytes());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&input, std::fs::Permissions::from_mode(0o640))
            .expect("permissions");
    }
    let before = std::fs::read(&input).expect("input");
    let permissions = std::fs::metadata(&input).expect("input").permissions();
    let unchanged = || {
        assert_eq!(std::fs::read(&input).expect("input"), before);
        assert_eq!(
            std::fs::metadata(&input).expect("input").permissions(),
            permissions
        );
    };

    assert_eq!(code(&scratch.run(&["clean", "note.md"])), 1);
    unchanged();
    let result = scratch.path("note.cleaned.md");
    assert_eq!(std::fs::read(&result).expect("the result"), b"ab\n");
    #[cfg(unix)]
    assert_eq!(
        std::fs::metadata(&result).expect("result").permissions(),
        permissions,
        "the result has the input's permissions"
    );

    // A second run replaces the result rather than stacking a new one.
    std::fs::write(&result, b"stale").expect("write");
    assert_eq!(code(&scratch.run(&["clean", "note.md"])), 1);
    assert_eq!(std::fs::read(&result).expect("the result"), b"ab\n");
    assert!(!scratch.path("note.cleaned.cleaned.md").exists());

    // A result cleaned again goes beside *it*.
    assert_eq!(code(&scratch.run(&["clean", "note.cleaned.md"])), 0);
    assert!(scratch.path("note.cleaned.cleaned.md").exists());

    #[cfg(unix)]
    std::os::unix::fs::symlink(&input, scratch.path("link.md")).expect("a symlink");
    std::fs::hard_link(&input, scratch.path("hard.md")).expect("a hard link");
    std::fs::create_dir(scratch.path("folder")).expect("a folder");
    let mut refused = vec!["note.md", "./note.md", "hard.md"];
    if cfg!(unix) {
        refused.push("link.md");
    }
    for out in refused {
        let output = scratch.run(&["clean", "note.md", "-o", out]);
        assert_eq!(code(&output), 2, "-o {out}: {}", stderr(&output));
        assert!(output.stdout.is_empty(), "-o {out}");
        unchanged();
    }
    let output = scratch.run(&["clean", "note.md", "-o", "folder"]);
    assert_eq!(code(&output), 2, "{}", stderr(&output));
    unchanged();
}

/// A path that is not there, or is a folder, is the caller's mistake —
/// a usage error, 2 — and not an inconclusive scan.
#[test]
fn a_missing_path_or_a_folder_is_a_usage_error() {
    let scratch = Scratch::new("usage");
    std::fs::create_dir(scratch.path("dir")).expect("a folder");
    for arguments in [
        &["inspect", "nope.md"][..],
        &["inspect", "dir"],
        &["clean", "dir"],
    ] {
        let output = scratch.run(arguments);
        assert_eq!(code(&output), 2, "{arguments:?}: {}", stderr(&output));
        assert!(output.stdout.is_empty(), "{arguments:?}");
    }
    assert!(stderr(&scratch.run(&["inspect", "nope.md"])).contains("nope.md does not exist"));
}

/// D11: a result is written in the encoding the input was in, with its
/// byte order mark if it had one, and the mark survives Layer A.
#[test]
fn a_file_keeps_its_encoding_and_its_bom() {
    fn utf16le(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }
    fn utf16be(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_be_bytes).collect()
    }
    fn utf32le(text: &str) -> Vec<u8> {
        text.chars()
            .flat_map(|c| u32::from(c).to_le_bytes())
            .collect()
    }

    let scratch = Scratch::new("encodings");
    // Without a mark, UTF-16 is recognised only from an all-ASCII head
    // (Q-D6), so the mark-free case puts its zero-width space past the
    // four kilobytes intake reads.
    let padding = "x".repeat(2100);
    let cases: Vec<(&str, Vec<u8>, Vec<u8>)> = vec![
        (
            "utf8-bom.txt",
            "\u{FEFF}a\u{200B}b".as_bytes().to_vec(),
            "\u{FEFF}ab".as_bytes().to_vec(),
        ),
        (
            "utf16le-bom.txt",
            utf16le("\u{FEFF}a\u{200B}b"),
            utf16le("\u{FEFF}ab"),
        ),
        (
            "utf16be-bom.txt",
            utf16be("\u{FEFF}a\u{200B}b"),
            utf16be("\u{FEFF}ab"),
        ),
        (
            "utf32le-bom.txt",
            utf32le("\u{FEFF}a\u{200B}b"),
            utf32le("\u{FEFF}ab"),
        ),
        (
            "utf16le.txt",
            utf16le(&format!("{padding}a\u{200B}b")),
            utf16le(&format!("{padding}ab")),
        ),
    ];
    for (name, bytes, expected) in cases {
        scratch.file(name, &bytes);
        let output = scratch.run(&["clean", name]);
        assert_eq!(code(&output), 1, "{name}: {}", stderr(&output));
        let result = scratch.path(&name.replace(".txt", ".cleaned.txt"));
        assert_eq!(
            std::fs::read(&result).expect("the result"),
            expected,
            "{name}"
        );
    }
}

/// Standard output is UTF-8 whatever the input was (D11): a UTF-16 file
/// on stdin comes out as the text it held, its mark as UTF-8's.
#[test]
fn standard_output_is_utf8_whatever_the_input_was() {
    let scratch = Scratch::new("stdout-utf8");
    let input: Vec<u8> = "\u{FEFF}a\u{200B}b"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let output = scratch.run_with(&["clean", "-"], Some(&input));
    assert_eq!(code(&output), 1, "{}", stderr(&output));
    assert_eq!(output.stdout, b"\xEF\xBB\xBFab");
}

/// `--json` is the report and nothing else (D12): one JSON value on
/// stdout, ASCII (D29), in the shape the destination asks for.
#[test]
fn the_json_is_the_report_and_nothing_else() {
    let scratch = Scratch::new("json");
    scratch.file("note.md", "a\u{200B}b".as_bytes());

    let output = scratch.run(&["inspect", "note.md", "--json"]);
    assert!(output.stdout.is_ascii());
    let report = json(&output);
    assert!(report["unicode_version"].is_string(), "{report}");
    assert_eq!(stdout(&output).lines().count(), 1);

    let keys = |value: &Value| {
        let mut keys: Vec<String> = value
            .as_object()
            .expect("an object")
            .keys()
            .cloned()
            .collect();
        keys.sort();
        keys
    };

    let output = scratch.run(&["clean", "note.md", "--json"]);
    let answer = json(&output);
    assert_eq!(keys(&answer), ["report", "written"]);
    assert!(
        answer["written"]
            .as_str()
            .is_some_and(|path| path.ends_with("note.cleaned.md")),
        "{answer}"
    );

    for (arguments, stdin) in [
        (&["clean", "-", "--json"][..], Some("a\u{200B}b".as_bytes())),
        (&["clean", "note.md", "-o", "-", "--json"], None),
    ] {
        let output = scratch.run_with(arguments, stdin);
        let answer = json(&output);
        assert_eq!(keys(&answer), ["report", "text"], "{arguments:?}");
        assert_eq!(answer["text"], "ab", "{arguments:?}");
    }
}

/// The third shelf on every answer: in every JSON, the three ids in
/// order; in every human report, the title and the three lines last.
#[test]
fn every_layer_a_answer_carries_the_third_shelf() {
    let scratch = Scratch::new("shelf");
    scratch.file("marked.md", "a\u{200B}b".as_bytes());
    scratch.file("plain.md", b"ab");
    let ids = serde_json::json!([
        "vendor-detector-evasion",
        "human-authorship",
        "unknown-mark-schemes"
    ]);
    let shelf = [
        "Not established",
        "  - evasion of a vendor's own detector — not tested, no oracle exists here",
        "  - human authorship — not established by any check in this tool",
        "  - marks in schemes this build does not implement — not searched for",
    ];
    for name in ["marked.md", "plain.md"] {
        let report = json(&scratch.run(&["inspect", name, "--json"]));
        assert_eq!(report["not_established"], ids, "inspect {name}");
        let answer = json(&scratch.run(&["clean", name, "--json", "-o", "-"]));
        assert_eq!(answer["report"]["not_established"], ids, "clean {name}");

        for output in [
            scratch.run(&["inspect", name]),
            scratch.run(&["clean", name, "-o", "out.md"]),
        ] {
            let text = stdout(&output);
            let lines: Vec<&str> = text.lines().collect();
            assert!(
                lines.ends_with(&shelf),
                "{name}: the report does not end with the third shelf:\n{text}"
            );
        }
        // And where the report goes to stderr, there too.
        let output = scratch.run(&["clean", name, "-o", "-"]);
        let text = stderr(&output);
        assert!(text.lines().collect::<Vec<_>>().ends_with(&shelf), "{text}");
    }
}

/// A file whose name lies is read by what it contains, and the note
/// says so — the bytes decide, the name only refines.
#[test]
fn a_name_that_lies_is_read_by_its_contents_and_said_so() {
    let scratch = Scratch::new("liar");
    scratch.file("chart.png", "a\u{200B}b".as_bytes());
    let output = scratch.run(&["inspect", "chart.png"]);
    assert_eq!(code(&output), 1, "{}", stderr(&output));
    let note = stderr(&output);
    assert!(note.contains("PNG") && note.contains("Text"), "{note}");
}

/// Positions are byte offsets into the file: for UTF-8 the file's own,
/// a byte order mark counted — because it is never stripped.
#[test]
fn positions_are_byte_offsets_into_a_utf8_file() {
    let scratch = Scratch::new("positions");
    scratch.file("plain.txt", "\u{e9}\u{200B}".as_bytes());
    scratch.file("bom.txt", "\u{FEFF}\u{e9}\u{200B}".as_bytes());
    for (name, at) in [("plain.txt", 2), ("bom.txt", 5)] {
        let report = json(&scratch.run(&["inspect", name, "--json"]));
        assert_eq!(
            report["findings"][0]["positions"],
            serde_json::json!([at]),
            "{name}: {report}"
        );
    }
}

/// `inspect` reads and writes nothing: the folder is what it was.
#[test]
fn inspect_never_writes_anything() {
    fn listing(dir: &Path) -> Vec<(String, Vec<u8>)> {
        let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir)
            .expect("the folder")
            .map(|entry| {
                let entry = entry.expect("an entry");
                let bytes = std::fs::read(entry.path()).unwrap_or_default();
                (entry.file_name().to_string_lossy().into_owned(), bytes)
            })
            .collect();
        files.sort();
        files
    }
    let scratch = Scratch::new("readonly");
    let folder = scratch.path("docs");
    std::fs::create_dir(&folder).expect("a folder");
    std::fs::write(folder.join("note.md"), "a\u{200B}b".as_bytes()).expect("write");
    let before = listing(&folder);
    scratch.run(&["inspect", "docs/note.md"]);
    scratch.run(&["inspect", "docs/note.md", "--json"]);
    assert_eq!(listing(&folder), before);
}

/// An empty file is a text with nothing in it — read, reported, and
/// cleaned into an empty result — not a file that could not be read.
#[test]
fn an_empty_input_is_read_and_has_nothing_in_it() {
    let scratch = Scratch::new("empty");
    scratch.file("x.txt", b"");
    let output = scratch.run(&["inspect", "x.txt"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(
        stdout(&output).starts_with("x.txt: none of the characters this version looks for"),
        "{}",
        stdout(&output)
    );
    let output = scratch.run_with(&["inspect", "-"], Some(b""));
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(stdout(&output).starts_with("standard input: none of"));

    let output = scratch.run(&["clean", "x.txt"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert_eq!(
        std::fs::read(scratch.path("x.cleaned.txt")).expect("an empty result"),
        b""
    );
}

/// `clean`'s exit is decided by the input exactly as `inspect`'s is — on
/// every fixture in the repository, whatever its class.
#[test]
fn clean_exits_as_inspect_does_on_every_fixture() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/text");
    let scratch = Scratch::new("fixtures");
    let mut read = 0;
    for entry in std::fs::read_dir(&fixtures).expect("fixtures/text") {
        let path = entry.expect("an entry").path();
        if path.file_name().is_some_and(|name| name == "README.md") {
            continue;
        }
        let path = path.to_str().expect("a UTF-8 path");
        let out = scratch.path("out.txt");
        let inspected = code(&scratch.run(&["inspect", path]));
        let cleaned = code(&scratch.run(&["clean", path, "-o", out.to_str().expect("UTF-8")]));
        assert_eq!(cleaned, inspected, "{path}");
        assert!(inspected == 0 || inspected == 1, "{path}: {inspected}");
        read += 1;
    }
    assert!(read >= 22, "only {read} fixtures were read");
}

/// What this version does not do still refuses at 2, says so on stderr,
/// and prints nothing a parser could take for an answer.
#[test]
fn the_commands_that_are_not_here_yet_still_refuse_at_two() {
    let scratch = Scratch::new("refuse");
    scratch.file("x.md", "a\u{200B}b".as_bytes());
    for arguments in [
        &["rewrite", "x.md"][..],
        &["models", "list"],
        &["audit", "."],
    ] {
        let output = scratch.run(arguments);
        assert_eq!(code(&output), 2, "{arguments:?}");
        assert!(!output.stderr.is_empty(), "{arguments:?}");
        assert!(output.stdout.is_empty(), "{arguments:?}");
    }
    assert!(!scratch.path("x.cleaned.md").exists());
}

/// The log has the shape of the run and nothing of its content: not the
/// document, not the name of the file it came from.
#[test]
fn nothing_reaches_the_log_but_the_shape() {
    let scratch = Scratch::new("log");
    scratch.file("Secret-name.md", "Hello\u{200B}world".as_bytes());
    let output = scratch.run(&["clean", "Secret-name.md"]);
    assert_eq!(code(&output), 1, "{}", stderr(&output));

    let logs = scratch.data().join("logs");
    let mut files = 0;
    for entry in std::fs::read_dir(&logs).expect("a log folder") {
        let log = std::fs::read_to_string(entry.expect("an entry").path()).expect("a log");
        assert!(log.contains("<elided"), "{log}");
        assert!(
            !log.contains("Hello"),
            "the document reached the log:\n{log}"
        );
        assert!(
            !log.contains("Secret-name"),
            "the file name reached the log:\n{log}"
        );
        files += 1;
    }
    assert!(files > 0, "no log was written");
}
