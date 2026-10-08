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

/// `rewrite` runs now: with no application and no model chosen it refuses
/// by name — what is missing, never "not implemented" — exits 2 and writes
/// nothing. And no other command answers with a refusal either.
#[test]
fn rewrite_no_longer_refuses() {
    let scratch = Scratch::new("refuse");
    scratch.file("x.md", "a\u{200B}b".as_bytes());
    std::fs::create_dir(scratch.path("tree")).expect("a folder");
    let output = scratch.run(&["rewrite", "x.md"]);
    assert_eq!(code(&output), 2, "{}", stderr(&output));
    let said = stderr(&output);
    assert!(!said.contains("not implemented"), "{said}");
    assert!(
        said.contains("No model on this machine is chosen"),
        "{said}"
    );
    assert!(output.stdout.is_empty());
    assert!(!scratch.path("x.cleaned.md").exists());

    for arguments in [
        &["models", "list"][..],
        &["models", "verify", "qwen3-4b-instruct-2507-ud-q4"],
        &["models", "rm", "qwen3-4b-instruct-2507-ud-q4"],
        &["audit", "tree"],
    ] {
        let output = scratch.run(arguments);
        assert!(
            !stderr(&output).contains("not implemented"),
            "{arguments:?}: {}",
            stderr(&output)
        );
        assert_ne!(code(&output), 2, "{arguments:?}: {}", stderr(&output));
    }
}

/// A stand-in for the running application's MCP server: answers
/// `initialize` as wipemark and `tools/call rewrite` with `answer`, and
/// counts every connection it takes.
struct FakeApp {
    port: u16,
    connections: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    calls: std::sync::Arc<std::sync::Mutex<Vec<Value>>>,
}

impl FakeApp {
    fn start(answer: Value) -> Self {
        use std::io::{BufRead, BufReader, Read, Write};
        use std::sync::atomic::Ordering;

        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("a port");
        let port = listener.local_addr().expect("an address").port();
        let connections = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let (counted, heard) = (connections.clone(), calls.clone());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                counted.fetch_add(1, Ordering::SeqCst);
                let mut writer = stream.try_clone().expect("a writer");
                let mut reader = BufReader::new(stream);
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        break;
                    }
                    let line = line.trim_end();
                    if line.is_empty() {
                        break;
                    }
                    if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        length = value.trim().parse().unwrap_or(0);
                    }
                }
                let mut body = vec![0; length];
                if reader.read_exact(&mut body).is_err() {
                    continue;
                }
                let request: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
                let reply = match request["method"].as_str() {
                    Some("initialize") => serde_json::json!({
                        "jsonrpc": "2.0", "id": request["id"],
                        "result": {"protocolVersion": "2025-06-18", "serverInfo": {"name": "wipemark", "version": "test"}},
                    }),
                    _ => {
                        heard.lock().expect("calls").push(request.clone());
                        serde_json::json!({"jsonrpc": "2.0", "id": request["id"], "result": answer})
                    }
                };
                let reply = reply.to_string();
                let _ = writer.write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                        reply.len()
                    )
                    .as_bytes(),
                );
            }
        });
        Self {
            port,
            connections,
            calls,
        }
    }

    fn connections(&self) -> usize {
        self.connections.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn calls(&self) -> Vec<Value> {
        self.calls.lock().expect("calls").clone()
    }
}

/// A beacon in the scratch data directory — what the application leaves
/// while its server listens.
fn beacon(scratch: &Scratch, pid: u32, address: &str, port: u16) {
    std::fs::create_dir_all(scratch.data()).expect("the data directory");
    std::fs::write(
        scratch.data().join("mcp.json"),
        serde_json::json!({"pid": pid, "port": port, "address": address}).to_string(),
    )
    .expect("a beacon");
}

/// The application's answer to a rewrite: `text`, and a report whose
/// totals and Layer A pass say what an exit code is read from.
fn rewritten(text: &str, kept: u64, suspicious: bool) -> Value {
    let report = serde_json::json!({
        "version": 1,
        "verifiable": {
            "before": {
                "unicode_version": "18.0.0",
                "suspicious": suspicious,
                "findings": if suspicious {
                    serde_json::json!([{"codepoint": "U+200B", "count": 1}])
                } else {
                    serde_json::json!([])
                },
                "kept": [],
            },
            "after": {},
        },
        "best_effort": {
            "base_seed": 4242,
            "totals": {"chunks": 2, "rewritten": 2 - kept, "kept_source": kept, "attempts": 4, "rejected": kept * 2},
        },
        "not_established": ["vendor-detector-evasion", "human-authorship", "unknown-mark-schemes"],
    });
    serde_json::json!({
        "content": [{"type": "text", "text": serde_json::json!({"text": text, "report": report}).to_string()}],
        "structuredContent": {"text": text, "report": report},
        "isError": false,
    })
}

/// With the application running, the document goes to it: its server is
/// asked, the result it sends is what is written, and both stderr and
/// `--json` say who rewrote it — with no model on this machine at all.
#[test]
fn rewrite_uses_the_running_application() {
    let scratch = Scratch::new("app");
    scratch.file("note.md", b"First paragraph.\n\nSecond one.\n");
    let app = FakeApp::start(rewritten("Paragraph one.\n\nThe second.\n", 0, false));
    beacon(&scratch, std::process::id(), "127.0.0.1", app.port);

    let output = scratch.run(&["rewrite", "note.md", "--tactic", "humanize", "--seed", "7"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert_eq!(
        std::fs::read_to_string(scratch.path("note.cleaned.md")).expect("the result"),
        "Paragraph one.\n\nThe second.\n"
    );
    assert_eq!(
        std::fs::read(scratch.path("note.md")).expect("the input"),
        b"First paragraph.\n\nSecond one.\n",
        "the input was touched"
    );
    let said = stdout(&output) + &stderr(&output);
    assert!(said.contains("Rewritten by the running"), "{said}");
    assert!(said.contains("4242"), "the seed is not said: {said}");

    let calls = app.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    let arguments = &calls[0]["params"]["arguments"];
    assert_eq!(calls[0]["params"]["name"], "rewrite");
    assert_eq!(arguments["text"], "First paragraph.\n\nSecond one.\n");
    assert_eq!(arguments["tactic"], "humanize");
    assert_eq!(arguments["format"], "markdown", "the file is Markdown");
    assert_eq!(arguments["seed"], 7);
    assert!(
        arguments.get("candidates").is_none(),
        "absent is the application's to decide"
    );
}

/// `--json` is the report, where the result went and who rewrote it.
#[test]
fn rewrite_json_is_the_report_and_where_it_went() {
    let scratch = Scratch::new("app-json");
    scratch.file("note.txt", b"Words.\n");
    let app = FakeApp::start(rewritten("Other words.\n", 0, true));
    beacon(&scratch, std::process::id(), "127.0.0.1", app.port);

    let output = scratch.run(&["rewrite", "note.txt", "--json"]);
    assert_eq!(
        code(&output),
        1,
        "Layer A findings in the input: {}",
        stderr(&output)
    );
    let answer = json(&output);
    assert_eq!(answer["served_by"], "application");
    assert!(answer["written"]
        .as_str()
        .is_some_and(|path| path.ends_with("note.cleaned.txt")));
    let shelf = answer["report"]["not_established"]
        .as_array()
        .expect("the third shelf");
    assert_eq!(shelf.len(), 3);
    assert!(stdout(&output).is_ascii(), "the --json line is ASCII");

    // And to standard output: the text beside the report.
    let output = scratch.run(&["rewrite", "note.txt", "-o", "-", "--json"]);
    assert_eq!(json(&output)["text"], "Other words.\n");
}

/// Not every part rewritten is inconclusive: exit 3, even when the input
/// had findings too — 3 beats 1, as in `audit`.
#[test]
fn a_chunk_that_kept_its_source_exits_three() {
    let scratch = Scratch::new("app-kept");
    scratch.file("note.txt", b"One.\n\nTwo.\n");
    let app = FakeApp::start(rewritten("Uno.\n\nTwo.\n", 1, true));
    beacon(&scratch, std::process::id(), "127.0.0.1", app.port);

    let output = scratch.run(&["rewrite", "note.txt"]);
    assert_eq!(code(&output), 3, "{}", stderr(&output));
    assert!(
        stdout(&output).contains("keeps its cleaned original"),
        "{}",
        stdout(&output)
    );
    // The result is written all the same: the kept paragraph is the cleaned one.
    assert!(scratch.path("note.cleaned.txt").exists());
}

/// `--in-place` sets the original aside first, as `clean` does.
#[test]
fn rewrite_in_place_sets_the_original_aside() {
    let scratch = Scratch::new("app-in-place");
    scratch.file("note.txt", b"Words.\n");
    let app = FakeApp::start(rewritten("Other words.\n", 0, false));
    beacon(&scratch, std::process::id(), "127.0.0.1", app.port);

    let output = scratch.run(&["rewrite", "note.txt", "--in-place"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert_eq!(
        std::fs::read(scratch.path("note.txt")).expect("the file"),
        b"Other words.\n"
    );
    assert_eq!(
        std::fs::read(scratch.path("note.original.txt")).expect("the original"),
        b"Words.\n"
    );
}

/// A beacon whose process is gone is never dialled: the command loads its
/// own engine instead — here, none is chosen, so it refuses.
#[test]
fn a_stale_beacon_is_never_dialled() {
    let scratch = Scratch::new("stale");
    scratch.file("note.txt", b"Words.\n");
    let app = FakeApp::start(rewritten("Other words.\n", 0, false));
    beacon(&scratch, u32::MAX - 7, "127.0.0.1", app.port);

    let output = scratch.run(&["rewrite", "note.txt"]);
    assert_eq!(code(&output), 2, "{}", stderr(&output));
    assert_eq!(app.connections(), 0, "a dead process's port was dialled");
    assert!(!scratch.path("note.cleaned.txt").exists());
}

/// A beacon that names anything but loopback is never dialled — not even
/// the wildcard, which reaches this machine's own port.
#[test]
fn a_beacon_off_this_machine_is_never_dialled() {
    let scratch = Scratch::new("elsewhere");
    scratch.file("note.txt", b"Words.\n");
    let app = FakeApp::start(rewritten("Other words.\n", 0, false));
    for address in ["0.0.0.0", "192.0.2.1"] {
        beacon(&scratch, std::process::id(), address, app.port);
        let output = scratch.run(&["rewrite", "note.txt"]);
        assert_eq!(code(&output), 2, "{address}: {}", stderr(&output));
    }
    assert_eq!(app.connections(), 0, "a beacon off loopback was dialled");
}

/// With no application, an endpoint is never answered by this command —
/// it refuses and names the application; a chosen model not downloaded is
/// named as missing.
#[test]
fn an_endpoint_duty_without_the_application_refuses() {
    for (rows, said) in [
        (
            &[("engine.serves", "endpoint")][..],
            "only through the running",
        ),
        (
            &[
                ("engine.serves", "endpoint-first"),
                ("engine.provider", "ollama"),
            ],
            "only through the running",
        ),
        (
            &[("models.rewrite", "qwen3-4b-instruct-2507-ud-q4")],
            "is not on this machine whole",
        ),
    ] {
        let scratch = Scratch::new("duty");
        scratch.file("note.txt", b"Words.\n");
        seed(&scratch, rows);
        let output = scratch.run(&["rewrite", "note.txt"]);
        assert_eq!(code(&output), 2, "{rows:?}: {}", stderr(&output));
        assert!(
            stderr(&output).contains(said),
            "{rows:?}: {}",
            stderr(&output)
        );
        assert!(!scratch.path("note.cleaned.txt").exists());
    }
}

/// D75: a template that breaks a rule stops the run before anything is
/// read or sent, naming the row — and its set — and the rule.
#[test]
fn an_invalid_prompts_file_exits_two_naming_the_rule() {
    let scratch = Scratch::new("prompts");
    scratch.file("note.txt", b"Words.\n");
    scratch.file(
        "templates.json",
        br#"{"prompts.en.paraphrase.1.user": "Again, in other words. {PROTECTED}"}"#,
    );
    let app = FakeApp::start(rewritten("Other words.\n", 0, false));
    beacon(&scratch, std::process::id(), "127.0.0.1", app.port);

    let output = scratch.run(&["rewrite", "note.txt", "--prompts", "templates.json"]);
    assert_eq!(code(&output), 2, "{}", stderr(&output));
    let said = stderr(&output);
    assert!(said.contains("prompts.en.paraphrase.1.user"), "{said}");
    assert!(said.contains("missing-variable"), "{said}");
    assert!(app.calls().is_empty(), "a refused template was sent");

    scratch.file("templates.json", b"[1, 2]");
    let output = scratch.run(&["rewrite", "note.txt", "--prompts", "templates.json"]);
    assert_eq!(code(&output), 2, "{}", stderr(&output));

    // A good one goes to the application with the call.
    scratch.file(
        "templates.json",
        br#"{"prompts.en.paraphrase.1.user": "Again, in other words. {PROTECTED}\n{TEXT}"}"#,
    );
    let output = scratch.run(&["rewrite", "note.txt", "--prompts", "templates.json"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let calls = app.calls();
    assert_eq!(
        calls[0]["params"]["arguments"]["templates"]["prompts.en.paraphrase.1.user"],
        "Again, in other words. {PROTECTED}\n{TEXT}"
    );
}

/// The two tactics this command does not run are refused with a sentence
/// that says why, before anything is read.
#[test]
fn a_window_only_tactic_is_refused_by_name() {
    let scratch = Scratch::new("tactic");
    scratch.file("note.txt", b"Words.\n");
    for (tactic, said) in [
        ("structural", "only in the application"),
        ("code", "not in this version"),
    ] {
        let output = scratch.run(&["rewrite", "note.txt", "--tactic", tactic]);
        assert_eq!(code(&output), 2, "{tactic}: {}", stderr(&output));
        assert!(
            stderr(&output).contains(said),
            "{tactic}: {}",
            stderr(&output)
        );
    }
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

// ---------------------------------------------------------------------
// clean --in-place
// ---------------------------------------------------------------------

/// The names in the scratch folder, sorted — the data directory left out,
/// because the run writes its log there.
fn names(scratch: &Scratch) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(&scratch.0)
        .expect("the scratch folder")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name != "data")
        .collect();
    names.sort();
    names
}

/// The order is the protection: the original is renamed aside first,
/// then the cleaned text replaces the file. The set-aside copy is the
/// input byte for byte, and no `.cleaned` file appears beside them.
#[test]
fn in_place_sets_the_original_aside_and_cleans_the_file() {
    let scratch = Scratch::new("in-place");
    let input = "Hello\u{200B}world\n".as_bytes();
    scratch.file("note.md", input);

    let output = scratch.run(&["clean", "note.md", "--in-place"]);
    assert_eq!(code(&output), 1, "{}", stderr(&output));
    assert_eq!(
        std::fs::read(scratch.path("note.md")).expect("the file"),
        b"Helloworld\n"
    );
    assert_eq!(
        std::fs::read(scratch.path("note.original.md")).expect("the original"),
        input,
        "the original was not set aside byte for byte"
    );
    assert_eq!(names(&scratch), ["note.md", "note.original.md"]);
    let report = stdout(&output);
    assert!(report.contains("note.original.md"), "{report}");

    // --json: the report and where things went, never the text.
    scratch.file("other.md", input);
    let output = scratch.run(&["clean", "other.md", "--in-place", "--json"]);
    assert_eq!(code(&output), 1, "{}", stderr(&output));
    let answer = json(&output);
    assert!(answer["report"]["unicode_version"].is_string(), "{answer}");
    assert!(answer.get("text").is_none(), "{answer}");
    assert!(
        answer["written"]["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("other.md")),
        "{answer}"
    );
    assert!(
        answer["written"]["original"]
            .as_str()
            .is_some_and(|path| path.ends_with("other.original.md")),
        "{answer}"
    );
}

/// ExifTool's rule: the first original is the original. A second run
/// over a file whose original is already set aside refuses at 2, names
/// the file in the way, and touches neither.
#[test]
fn in_place_never_overwrites_an_existing_original() {
    let scratch = Scratch::new("in-place-twice");
    scratch.file("note.md", "a\u{200B}b\n".as_bytes());
    scratch.file("note.original.md", b"the first original");

    let output = scratch.run(&["clean", "note.md", "--in-place"]);
    assert_eq!(code(&output), 2, "{}", stderr(&output));
    assert!(output.stdout.is_empty(), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("note.original.md"),
        "{}",
        stderr(&output)
    );
    assert_eq!(
        std::fs::read(scratch.path("note.md")).expect("the file"),
        "a\u{200B}b\n".as_bytes(),
        "the file was changed"
    );
    assert_eq!(
        std::fs::read(scratch.path("note.original.md")).expect("the original"),
        b"the first original",
        "an existing original was overwritten"
    );
    assert_eq!(names(&scratch), ["note.md", "note.original.md"]);
}

/// The per-run flag, never a preference: no copy of the original, and
/// nothing else left behind either.
#[test]
fn in_place_with_no_original_keeps_no_copy() {
    let scratch = Scratch::new("in-place-no-copy");
    scratch.file("note.md", "a\u{200B}b\n".as_bytes());

    let output = scratch.run(&["clean", "note.md", "--in-place", "--no-original"]);
    assert_eq!(code(&output), 1, "{}", stderr(&output));
    assert_eq!(
        std::fs::read(scratch.path("note.md")).expect("the file"),
        b"ab\n"
    );
    assert_eq!(names(&scratch), ["note.md"], "a copy was kept anyway");

    let output = scratch.run(&["clean", "note.md", "--in-place", "--no-original", "--json"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
}

/// Nothing to change is nothing touched: no original set aside, no new
/// inode, the modification time where it was. A homoglyph kept without
/// `--aggressive` is the one case with findings and no change — it
/// still exits 1, and still touches nothing.
#[test]
fn in_place_touches_nothing_when_nothing_changed() {
    let scratch = Scratch::new("in-place-nothing");
    let then = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
    for (name, bytes, exit) in [
        ("plain.md", "plain text\n".as_bytes(), 0),
        ("pay.md", "p\u{0430}y\n".as_bytes(), 1),
    ] {
        let path = scratch.file(name, bytes);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .expect("open")
            .set_modified(then)
            .expect("an old mtime");
        #[cfg(unix)]
        let inode = {
            use std::os::unix::fs::MetadataExt as _;
            std::fs::metadata(&path).expect("meta").ino()
        };

        let output = scratch.run(&["clean", name, "--in-place"]);
        assert_eq!(code(&output), exit, "{name}: {}", stderr(&output));
        assert_eq!(std::fs::read(&path).expect("the file"), bytes, "{name}");
        assert_eq!(
            std::fs::metadata(&path)
                .expect("meta")
                .modified()
                .expect("mtime"),
            then,
            "{name}: the file was written although nothing changed"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;
            assert_eq!(
                std::fs::metadata(&path).expect("meta").ino(),
                inode,
                "{name}"
            );
        }
        let output = scratch.run(&["clean", name, "--in-place", "--json"]);
        assert_eq!(json(&output)["written"], Value::Null, "{name}");
    }
    assert_eq!(
        names(&scratch),
        ["pay.md", "plain.md"],
        "an original was set aside"
    );
}

/// `--in-place` replaces a *file*: not standard input, not beside `-o`,
/// and `--no-original` means nothing without it. All refused at 2 with
/// the file untouched.
#[test]
fn in_place_refuses_stdin_and_conflicts_with_out() {
    let scratch = Scratch::new("in-place-usage");
    let input = "a\u{200B}b\n".as_bytes();
    scratch.file("note.md", input);
    for (arguments, stdin) in [
        (&["clean", "-", "--in-place"][..], Some(input)),
        (&["clean", "note.md", "--in-place", "-o", "out.md"], None),
        (&["clean", "note.md", "--no-original"], None),
    ] {
        let output = scratch.run_with(arguments, stdin);
        assert_eq!(code(&output), 2, "{arguments:?}: {}", stderr(&output));
        assert!(output.stdout.is_empty(), "{arguments:?}");
        assert!(!output.stderr.is_empty(), "{arguments:?}");
    }
    assert_eq!(
        std::fs::read(scratch.path("note.md")).expect("the file"),
        input
    );
    assert_eq!(names(&scratch), ["note.md"]);
}

/// The cleaned file keeps the mode bits the original had — the set-aside
/// original keeps them too, being the same file under another name.
#[cfg(unix)]
#[test]
fn in_place_keeps_the_file_permissions() {
    use std::os::unix::fs::PermissionsExt as _;
    let scratch = Scratch::new("in-place-mode");
    for (name, extra) in [("note.md", None), ("bare.md", Some("--no-original"))] {
        let path = scratch.file(name, "a\u{200B}b\n".as_bytes());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640))
            .expect("permissions");
        let mut arguments = vec!["clean", name, "--in-place"];
        arguments.extend(extra);
        let output = scratch.run(&arguments);
        assert_eq!(code(&output), 1, "{name}: {}", stderr(&output));
        let mode = std::fs::metadata(&path).expect("meta").permissions().mode() & 0o777;
        assert_eq!(mode, 0o640, "{name}: the cleaned file lost its permissions");
    }
    let original = std::fs::metadata(scratch.path("note.original.md")).expect("the original");
    assert_eq!(original.permissions().mode() & 0o777, 0o640);
}

/// D11 holds in place as it does beside: a UTF-16 file is replaced by
/// UTF-16, its byte order mark included, and the original set aside is
/// the input's bytes.
#[test]
fn in_place_keeps_the_files_encoding() {
    fn utf16le(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }
    let scratch = Scratch::new("in-place-utf16");
    let input = utf16le("\u{FEFF}a\u{200B}b\n");
    scratch.file("wide.txt", &input);
    let output = scratch.run(&["clean", "wide.txt", "--in-place"]);
    assert_eq!(code(&output), 1, "{}", stderr(&output));
    assert_eq!(
        std::fs::read(scratch.path("wide.txt")).expect("the file"),
        utf16le("\u{FEFF}ab\n"),
        "the file was not written back in its own encoding"
    );
    assert_eq!(
        std::fs::read(scratch.path("wide.original.txt")).expect("the original"),
        input
    );
}

// ---------------------------------------------------------------------
// audit
// ---------------------------------------------------------------------

const SHELF_IDS: [&str; 3] = [
    "vendor-detector-evasion",
    "human-authorship",
    "unknown-mark-schemes",
];

/// A tree under the scratch folder — not the scratch folder itself,
/// whose `data/logs` the run writes to while it walks.
fn tree(scratch: &Scratch, files: &[(&str, &[u8])]) -> PathBuf {
    let root = scratch.path("tree");
    std::fs::create_dir_all(&root).expect("the tree");
    for (name, bytes) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("folders");
        std::fs::write(path, bytes).expect("write");
    }
    root
}

/// The file entry for `path` in an `audit --json` answer.
fn entry<'a>(answer: &'a Value, path: &str) -> &'a Value {
    answer["files"]
        .as_array()
        .expect("files")
        .iter()
        .find(|file| file["path"] == path)
        .unwrap_or_else(|| panic!("{path} is not listed: {answer}"))
}

/// A file made unreadable, or `None` when this process reads it anyway
/// (root does) and the test has nothing to prove.
#[cfg(unix)]
fn locked(path: &Path) -> Option<()> {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000)).expect("chmod");
    if std::fs::read(path).is_ok() {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).expect("chmod");
        return None;
    }
    Some(())
}

/// The pre-commit scenario: a clean tree exits 0, the same tree with one
/// zero-width space anywhere in it exits 1 and names the file.
#[test]
fn audit_exits_one_for_findings_and_zero_for_a_clean_tree() {
    let scratch = Scratch::new("audit");
    tree(
        &scratch,
        &[
            ("a.md", b"plain\n"),
            ("sub/b.txt", "\u{e9}t\u{e9}\n".as_bytes()),
        ],
    );
    let output = scratch.run(&["audit", "tree"]);
    assert_eq!(code(&output), 0, "{}{}", stdout(&output), stderr(&output));
    assert!(stdout(&output).contains("scanned 2"), "{}", stdout(&output));

    tree(
        &scratch,
        &[("sub/c.md", "a\u{200B}b\u{200B}c\n".as_bytes())],
    );
    let output = scratch.run(&["audit", "tree"]);
    assert_eq!(code(&output), 1, "{}{}", stdout(&output), stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("sub/c.md: 2 findings"), "{text}");
    assert!(!text.contains("a.md:"), "a clean file was listed: {text}");
    assert!(
        text.lines()
            .any(|line| line.contains("scanned 3") && line.contains("with findings 1")),
        "{text}"
    );

    // Not a folder, or not there: the caller's mistake.
    for arguments in [&["audit", "tree/a.md"][..], &["audit", "nope"]] {
        let output = scratch.run(arguments);
        assert_eq!(code(&output), 2, "{arguments:?}: {}", stderr(&output));
        assert!(output.stdout.is_empty(), "{arguments:?}");
    }
}

/// Inconclusive is not clean, and it beats a finding: a hook must not
/// read a scan with a hole in it as a complete one with marks in it.
#[cfg(unix)]
#[test]
fn audit_exits_three_when_a_file_could_not_be_read_even_with_findings_elsewhere() {
    let scratch = Scratch::new("audit-locked");
    let root = tree(
        &scratch,
        &[
            ("marked.md", "a\u{200B}b\n".as_bytes()),
            ("locked.md", b"x\n"),
        ],
    );
    if locked(&root.join("locked.md")).is_none() {
        eprintln!("skipped: this process reads a mode-000 file (root)");
        return;
    }
    let output = scratch.run(&["audit", "tree"]);
    assert_eq!(code(&output), 3, "{}{}", stdout(&output), stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("marked.md: 1 finding"), "{text}");
    assert!(text.contains("locked.md could not be read"), "{text}");

    let answer = json(&scratch.run(&["audit", "tree", "--json"]));
    assert_eq!(
        entry(&answer, "locked.md")["status"],
        "unreadable",
        "{answer}"
    );
    assert_eq!(answer["summary"]["unreadable"], 1, "{answer}");
}

/// Hidden entries — `.git` above all — and what is not text are skipped,
/// counted and listed, and never read: a zero-width space inside `.git`
/// does not make the tree marked.
#[test]
fn audit_skips_hidden_entries_and_binary_files_and_counts_them() {
    let scratch = Scratch::new("audit-skip");
    let marked = "a\u{200B}b\n".as_bytes();
    tree(
        &scratch,
        &[
            (".git/COMMIT_EDITMSG", marked),
            (".hidden.md", marked),
            ("blob.bin", b"\x00\x01\x02\x03\xff\xfe binary"),
            // Since images are audited (E11-2) a PNG is read, not skipped,
            // and one cut off after its signature is a file that could not
            // be read — a hole in the scan, so the exit is 3.
            (
                "image.png",
                b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01",
            ),
            ("empty.txt", b""),
            ("ok.md", b"fine\n"),
        ],
    );
    let output = scratch.run(&["audit", "tree", "--json"]);
    assert_eq!(code(&output), 3, "{}", stdout(&output));
    let answer = json(&output);
    assert_eq!(answer["summary"]["scanned"], 1, "{answer}");
    assert_eq!(answer["summary"]["skipped"], 4, "{answer}");
    assert_eq!(answer["summary"]["unreadable"], 1, "{answer}");
    for path in [".git", ".hidden.md", "blob.bin", "empty.txt"] {
        assert_eq!(
            entry(&answer, path)["status"],
            "skipped",
            "{path}: {answer}"
        );
        assert!(
            entry(&answer, path)["reason"].is_string(),
            "{path}: {answer}"
        );
        assert_eq!(entry(&answer, path)["report"], Value::Null, "{path}");
    }
    assert_eq!(entry(&answer, "image.png")["status"], "unreadable");
    assert_eq!(entry(&answer, "image.png")["report"], Value::Null);
    assert_eq!(entry(&answer, "ok.md")["status"], "scanned");
    let output = scratch.run(&["audit", "tree"]);
    assert!(stdout(&output).contains("skipped 4"), "{}", stdout(&output));
}

/// A link to a folder is not followed — a link back up the tree is a
/// loop, a link out of it is somebody else's tree — while a link to a
/// file is read like the file.
#[cfg(unix)]
#[test]
fn audit_does_not_follow_a_directory_symlink() {
    let scratch = Scratch::new("audit-links");
    let root = tree(&scratch, &[("real/x.md", b"fine\n")]);
    let outside = scratch.path("outside");
    std::fs::create_dir(&outside).expect("a folder");
    std::fs::write(outside.join("marked.md"), "a\u{200B}b".as_bytes()).expect("write");
    std::fs::write(outside.join("linked.md"), b"also fine\n").expect("write");
    std::os::unix::fs::symlink(&root, root.join("real/loop")).expect("a loop");
    std::os::unix::fs::symlink(&outside, root.join("elsewhere")).expect("a link out");
    std::os::unix::fs::symlink(outside.join("linked.md"), root.join("linked.md"))
        .expect("a file link");

    let output = scratch.run(&["audit", "tree", "--json"]);
    assert_eq!(code(&output), 0, "{}{}", stdout(&output), stderr(&output));
    let answer = json(&output);
    assert_eq!(entry(&answer, "real/loop")["status"], "skipped", "{answer}");
    assert_eq!(entry(&answer, "elsewhere")["status"], "skipped", "{answer}");
    assert_eq!(entry(&answer, "linked.md")["status"], "scanned", "{answer}");
    assert_eq!(answer["summary"]["scanned"], 2, "{answer}");
}

/// Every scanned file carries the report `inspect --json` prints, third
/// shelf included; a skipped one carries none, never an empty report.
#[test]
fn audit_json_carries_every_report_with_its_third_shelf() {
    let scratch = Scratch::new("audit-json");
    tree(
        &scratch,
        &[
            ("marked.md", "a\u{200B}b\n".as_bytes()),
            ("plain.md", b"plain\n"),
            (
                "image.png",
                b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01",
            ),
        ],
    );
    // The truncated PNG is unreadable since images are audited (E11-2):
    // 3 beats the marked file's 1.
    let output = scratch.run(&["audit", "tree", "--json"]);
    assert_eq!(code(&output), 3, "{}", stderr(&output));
    assert!(output.stdout.is_ascii());
    assert_eq!(
        stdout(&output).lines().count(),
        1,
        "one JSON value, one line"
    );
    let answer = json(&output);
    assert_eq!(answer["version"], 1);
    assert_eq!(answer["unicode"], "18.0.0");
    assert_eq!(answer["root"], "tree");
    let inspected = json(&scratch.run(&["inspect", "tree/marked.md", "--json"]));
    assert_eq!(
        entry(&answer, "marked.md")["report"],
        inspected,
        "not inspect's report"
    );
    for path in ["marked.md", "plain.md"] {
        let file = entry(&answer, path);
        assert_eq!(file["status"], "scanned", "{path}");
        assert_eq!(file["reason"], Value::Null, "{path}");
        assert_eq!(
            file["report"]["not_established"],
            serde_json::json!(SHELF_IDS),
            "{path}: the third shelf is missing"
        );
    }
    assert_eq!(entry(&answer, "image.png")["status"], "unreadable");
    assert_eq!(entry(&answer, "image.png")["report"], Value::Null);
    assert_eq!(answer["summary"]["with_findings"], 1, "{answer}");
}

/// SARIF 2.1.0 in the shape code scanning reads, with columns counted in
/// code points: a zero-width space after two CJK characters on line 2 is
/// at column 3 — not 7, which is where it is in bytes.
#[test]
fn audit_sarif_is_2_1_0_with_code_point_columns() {
    let scratch = Scratch::new("audit-sarif");
    tree(
        &scratch,
        &[
            (
                "docs/cjk.md",
                "first line\n\u{6F22}\u{5B57}\u{200B}x\n".as_bytes(),
            ),
            ("plain.md", b"plain\n"),
        ],
    );
    let output = scratch.run(&["audit", "tree", "--sarif"]);
    assert_eq!(code(&output), 1, "{}", stderr(&output));
    let sarif = json(&output);
    assert_eq!(sarif["version"], "2.1.0", "{sarif}");
    assert!(sarif["$schema"]
        .as_str()
        .is_some_and(|s| s.contains("sarif")));
    let runs = sarif["runs"].as_array().expect("runs");
    assert_eq!(runs.len(), 1);
    let run = &runs[0];
    assert_eq!(run["tool"]["driver"]["name"], "wipemark");
    assert!(run["tool"]["driver"]["version"].is_string());
    assert!(run["tool"]["driver"].get("informationUri").is_none());
    assert_eq!(run["columnKind"], "unicodeCodePoints");
    let rules = run["tool"]["driver"]["rules"].as_array().expect("rules");
    assert_eq!(rules.len(), 1, "{rules:?}");
    assert_eq!(rules[0]["id"], "zero-width");
    assert!(rules[0]["shortDescription"]["text"].is_string());
    assert_eq!(run["invocations"][0]["executionSuccessful"], true);

    let results = run["results"].as_array().expect("results");
    assert_eq!(results.len(), 1, "{results:?}");
    let result = &results[0];
    assert_eq!(result["ruleId"], "zero-width");
    assert_eq!(result["level"], "error");
    assert!(
        result["message"]["text"]
            .as_str()
            .is_some_and(|text| text.contains("U+200B ZERO WIDTH SPACE")),
        "{result}"
    );
    let location = &result["locations"][0]["physicalLocation"];
    assert_eq!(location["artifactLocation"]["uri"], "docs/cjk.md");
    assert_eq!(location["artifactLocation"]["uriBaseId"], "SRCROOT");
    let region = &location["region"];
    assert_eq!(region["startLine"], 2, "{region}");
    assert_eq!(region["startColumn"], 3, "{region}");
    assert_eq!(region["endColumn"], 4, "{region}");
    assert_eq!(region["charOffset"], 13, "{region}");
    assert_eq!(region["charLength"], 1, "{region}");

    // --json and --sarif are two answers; asking for both is a mistake.
    let output = scratch.run(&["audit", "tree", "--sarif", "--json"]);
    assert_eq!(code(&output), 2, "{}", stderr(&output));
}

/// A file that could not be read is a notification at level `error`,
/// and the invocation is not successful — the SARIF spelling of exit 3.
#[cfg(unix)]
#[test]
fn audit_sarif_marks_the_run_unsuccessful_when_a_file_was_unreadable() {
    let scratch = Scratch::new("audit-sarif-locked");
    let root = tree(&scratch, &[("locked.md", b"x\n"), ("plain.md", b"plain\n")]);
    if locked(&root.join("locked.md")).is_none() {
        eprintln!("skipped: this process reads a mode-000 file (root)");
        return;
    }
    let output = scratch.run(&["audit", "tree", "--sarif"]);
    assert_eq!(code(&output), 3, "{}", stderr(&output));
    let sarif = json(&output);
    let invocation = &sarif["runs"][0]["invocations"][0];
    assert_eq!(invocation["executionSuccessful"], false, "{invocation}");
    let notes = invocation["toolExecutionNotifications"]
        .as_array()
        .expect("notes");
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert_eq!(notes[0]["level"], "error");
    assert_eq!(
        notes[0]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
        "locked.md"
    );
}

// ---------------------------------------------------------------------
// models
// ---------------------------------------------------------------------

const SMALL: &str = "qwen3-4b-instruct-2507-ud-q4";
const SMALL_FILE: &str = "Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf";
const LARGE: &str = "gemma-3-12b-it-qat-ud-q4";
const LARGE_FILE: &str = "gemma-3-12b-it-qat-UD-Q4_K_XL.gguf";

/// The models entry for `id` in a `models list --json` answer.
fn model<'a>(answer: &'a Value, id: &str) -> &'a Value {
    answer["models"]
        .as_array()
        .expect("models")
        .iter()
        .find(|model| model["id"] == id)
        .unwrap_or_else(|| panic!("{id} is not listed: {answer}"))
}

/// A settings database in the scratch data directory with these rows —
/// what the app would have written.
fn seed(scratch: &Scratch, rows: &[(&str, &str)]) {
    std::fs::create_dir_all(scratch.data()).expect("the data directory");
    let store =
        wipemark_store::Store::open(scratch.data().join("wipemark.db")).expect("a database");
    for (key, value) in rows {
        store.settings().set(key, *value).expect("a row");
    }
}

/// A model's size is said in the language's decimals — "7,4 GB" in German,
/// "7,4 ГБ" in Russian — and with a point in English; the JSON keeps its
/// bytes.
#[test]
fn models_sizes_are_spelled_in_the_languages_decimals() {
    let scratch = Scratch::new("models-decimals");
    for (language, size) in [("en-US", "7.4 GB"), ("de", "7,4 GB"), ("ru", "7,4 ГБ")] {
        let output = Command::new(env!("CARGO_BIN_EXE_wipemark-cli"))
            .args(["models", "list"])
            .current_dir(&scratch.0)
            .env("WIPEMARK_DATA_DIR", scratch.data())
            .env("WIPEMARK_LANG", language)
            .env_remove("WIPEMARK_LOG")
            .env_remove("RUST_LOG")
            .stdin(Stdio::null())
            .output()
            .expect("the binary runs");
        assert_eq!(code(&output), 0, "{language}: {}", stderr(&output));
        let text = stdout(&output);
        assert!(text.contains(size), "{language}: {text}");
        let other = if size.contains(',') { "7.4" } else { "7,4" };
        assert!(!text.contains(other), "{language}: {text}");
    }
}

/// Every catalogue entry is listed with what is on this machine for it:
/// nothing, a partial download, or a file that does not match — and a
/// weight file the catalogue does not know is listed after, unverified.
#[test]
fn models_list_names_every_catalogue_entry_and_its_state() {
    let scratch = Scratch::new("models-list");
    let output = scratch.run(&["models", "list"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let text = stdout(&output);
    for id in [SMALL, LARGE] {
        assert!(text.contains(id), "{id} missing:\n{text}");
    }
    assert!(text.contains("not downloaded"), "{text}");

    let models = scratch.data().join("models");
    std::fs::create_dir_all(models.join(SMALL)).expect("mkdir");
    std::fs::write(
        models.join(SMALL).join(format!("{SMALL_FILE}.part")),
        vec![0u8; 1000],
    )
    .expect("a partial download");
    std::fs::create_dir_all(models.join(LARGE)).expect("mkdir");
    std::fs::write(models.join(LARGE).join(LARGE_FILE), b"not the weights").expect("a stray file");
    std::fs::create_dir_all(models.join("lmstudio/vendor")).expect("mkdir");
    std::fs::write(models.join("lmstudio/vendor/other.gguf"), b"GGUF").expect("another tool's");

    let output = scratch.run(&["models", "list", "--json"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let answer = json(&output);
    assert_eq!(model(&answer, SMALL)["state"], "partial", "{answer}");
    assert_eq!(model(&answer, SMALL)["done_bytes"], 1000, "{answer}");
    // Not "mismatch": no download of this product wrote that file, so it
    // is another tool's, said and left alone (D302, amended).
    assert_eq!(model(&answer, LARGE)["state"], "absent", "{answer}");
    assert_eq!(
        model(&answer, LARGE)["foreign_at"],
        format!("{LARGE}/{LARGE_FILE}"),
        "{answer}"
    );
    for id in [SMALL, LARGE] {
        let entry = model(&answer, id);
        assert!(entry["size_bytes"]
            .as_u64()
            .is_some_and(|size| size > 1_000_000_000));
        assert_eq!(entry["roles"], serde_json::json!(["rewrite"]), "{answer}");
        assert!(
            ["fits", "tight", "too-big", "unknown"]
                .contains(&entry["fit"].as_str().expect("a fit")),
            "{answer}"
        );
        assert_eq!(entry["chosen"], false, "{answer}");
    }
    let others = answer["others"].as_array().expect("others");
    assert_eq!(others.len(), 1, "{others:?}");
    assert_eq!(others[0]["path"], "lmstudio/vendor/other.gguf");
    assert_eq!(others[0]["verified"], false);

    let text = stdout(&scratch.run(&["models", "list"]));
    assert!(text.contains("partly downloaded"), "{text}");
    assert!(text.contains("another tool's file of its name"), "{text}");
    assert!(text.contains("lmstudio/vendor/other.gguf"), "{text}");
    assert!(text.contains("not verified"), "{text}");
}

/// H1 (D302, amended), through the binary: a file at an entry's own
/// place that no download of this product wrote — another tool's, in a
/// mirror laid out `<id>/<file>` — is listed as left alone, and `models
/// rm` says nothing was removed and removes nothing.
#[test]
fn models_rm_leaves_another_tools_file_at_the_entrys_place() {
    let scratch = Scratch::new("models-rm-mirror");
    let mirror = scratch.path("mirror");
    let theirs = mirror.join(SMALL).join(SMALL_FILE);
    std::fs::create_dir_all(theirs.parent().expect("a folder")).expect("mkdir");
    std::fs::write(&theirs, b"another tool put me here").expect("theirs");
    seed(&scratch, &[("models.dir", mirror.to_str().expect("UTF-8"))]);

    let answer = json(&scratch.run(&["models", "list", "--json"]));
    assert_eq!(model(&answer, SMALL)["state"], "absent", "{answer}");
    assert_eq!(
        model(&answer, SMALL)["foreign_at"],
        format!("{SMALL}/{SMALL_FILE}"),
        "{answer}"
    );

    let output = scratch.run(&["models", "rm", SMALL]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let said = stdout(&output);
    assert!(said.contains("nothing was removed"), "{said}");
    assert!(said.contains(SMALL_FILE), "{said}");
    assert_eq!(
        std::fs::read(&theirs).expect("still there"),
        b"another tool put me here"
    );
}

/// The models folder and the chosen model are the app's rows, read and
/// never written: a scratch folder named by `models.dir` is the one
/// listed, and the chosen model is marked.
#[test]
fn models_read_the_folder_and_the_choice_the_app_saved() {
    let scratch = Scratch::new("models-rows");
    let elsewhere = scratch.path("weights");
    std::fs::create_dir_all(elsewhere.join(SMALL)).expect("mkdir");
    std::fs::write(
        elsewhere.join(SMALL).join(format!("{SMALL_FILE}.part")),
        b"xx",
    )
    .expect("part");
    seed(
        &scratch,
        &[
            ("models.dir", elsewhere.to_str().expect("UTF-8")),
            ("models.rewrite", SMALL),
        ],
    );
    let answer = json(&scratch.run(&["models", "list", "--json"]));
    assert_eq!(
        answer["folder"],
        elsewhere.to_str().expect("UTF-8"),
        "{answer}"
    );
    assert_eq!(model(&answer, SMALL)["state"], "partial", "{answer}");
    assert_eq!(model(&answer, SMALL)["chosen"], true, "{answer}");
    assert_eq!(model(&answer, LARGE)["chosen"], false, "{answer}");

    // A relative row is unusable: the default folder, and the row left.
    seed(&scratch, &[("models.dir", "relative/weights")]);
    let answer = json(&scratch.run(&["models", "list", "--json"]));
    assert_eq!(
        answer["folder"],
        scratch.data().join("models").to_str().expect("UTF-8"),
        "{answer}"
    );
}

/// An id the catalogue does not have is refused by name, with the ids it
/// does have — for every subcommand that takes one.
#[test]
fn models_with_an_unknown_id_refuse_and_list_the_ids() {
    let scratch = Scratch::new("models-unknown");
    for command in ["pull", "verify", "rm"] {
        let output = scratch.run(&["models", command, "qwen3-8b"]);
        assert_eq!(code(&output), 2, "{command}: {}", stderr(&output));
        assert!(output.stdout.is_empty(), "{command}");
        let text = stderr(&output);
        assert!(text.contains("qwen3-8b"), "{command}: {text}");
        for id in [SMALL, LARGE] {
            assert!(text.contains(id), "{command}: {id} not offered: {text}");
        }
    }
    assert!(
        !scratch.data().join("models").exists(),
        "an unknown id created a folder"
    );
}

/// Verify is a finding when the file is not the catalogue's: absent, or
/// present with the wrong bytes. Both exit 1, never 0.
#[test]
fn models_verify_of_an_absent_model_is_a_finding() {
    let scratch = Scratch::new("models-verify");
    let output = scratch.run(&["models", "verify", SMALL]);
    assert_eq!(code(&output), 1, "{}", stderr(&output));
    assert!(stdout(&output).contains(SMALL), "{}", stdout(&output));

    let folder = scratch.data().join("models").join(SMALL);
    std::fs::create_dir_all(&folder).expect("mkdir");
    std::fs::write(folder.join(SMALL_FILE), b"not the weights").expect("write");
    let output = scratch.run(&["models", "verify", SMALL]);
    assert_eq!(code(&output), 1, "{}", stderr(&output));
    assert!(
        stdout(&output).contains("does not match the catalogue"),
        "{}",
        stdout(&output)
    );
}

/// Removing what is not there is not a failure, and says so; removing
/// the chosen model says what the application will show, and writes no
/// row.
#[test]
fn models_rm_of_an_absent_model_says_so_and_exits_zero() {
    let scratch = Scratch::new("models-rm");
    let output = scratch.run(&["models", "rm", SMALL]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(
        stdout(&output).contains("was not on this machine"),
        "{}",
        stdout(&output)
    );

    let folder = scratch.data().join("models").join(SMALL);
    std::fs::create_dir_all(&folder).expect("mkdir");
    std::fs::write(folder.join(format!("{SMALL_FILE}.part")), b"xx").expect("write");
    seed(&scratch, &[("models.rewrite", SMALL)]);
    let output = scratch.run(&["models", "rm", SMALL]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(!folder.exists(), "the model folder is still there");
    let text = stdout(&output);
    assert!(text.contains("was removed"), "{text}");
    assert!(text.contains("no model chosen"), "{text}");
    let store = wipemark_store::Store::open_read_only(scratch.data().join("wipemark.db"))
        .expect("the database")
        .expect("it exists");
    let chosen: Option<String> = store.settings().get("models.rewrite").expect("a read");
    assert_eq!(chosen.as_deref(), Some(SMALL), "the CLI wrote the row");
}

/// The CLI opens the app's database read-only and never creates one —
/// not to list models, not to verify or remove one, not to audit.
#[test]
fn the_cli_never_creates_a_database() {
    let scratch = Scratch::new("no-db");
    tree(&scratch, &[("a.md", b"plain\n")]);
    for arguments in [
        &["models", "list"][..],
        &["models", "list", "--json"],
        &["models", "verify", SMALL],
        &["models", "rm", SMALL],
        &["audit", "tree"],
    ] {
        scratch.run(arguments);
        for name in ["wipemark.db", "wipemark.db-wal", "wipemark.db-shm"] {
            assert!(
                !scratch.data().join(name).exists(),
                "{arguments:?} created {name}"
            );
        }
    }
}
