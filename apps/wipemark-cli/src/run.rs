//! `inspect` and `clean`: the two flows, where each output goes, and the
//! exit code.
//!
//! # Where each output goes
//!
//! Standard output carries exactly **one** product: the JSON, or the
//! cleaned text, or the human report. The human report moves to standard
//! error only when standard output carries the text. On a refusal or a
//! failure standard output stays empty — a parser that gets nothing looks
//! at the exit code, and the exit code says why.
//!
//! | command | `--json` | result goes to | stdout | stderr |
//! |---|---|---|---|---|
//! | `inspect` | no | — | the report | note, refusal |
//! | `inspect` | yes | — | `InspectReport::to_json()` | note, refusal |
//! | `clean` | no | a file | the report | note, refusal |
//! | `clean` | no | stdout | the cleaned text | the report, note, refusal |
//! | `clean` | yes | a file | `{"report":…,"written":…}` | note, refusal |
//! | `clean` | yes | stdout | `{"report":…,"text":…}` | note, refusal |
//!
//! # The exit code reads core's answer
//!
//! `inspect` exits by `report.suspicious`, `clean` by
//! `cleaned.report.suspicious` — which core computes over the **input**
//! (D28, A §7.3): a `clean` that removed every mark still exits 1,
//! because a pre-commit hook wants to know what was there. There is no
//! second pass over the result and no second spelling of the rule here.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use wipemark_core::Options;
use wipemark_i18n::{args, t, t_args, FluentArgs, Message};
use wipemark_intake::name::{with_infix, RESULT_INFIX};
use wipemark_log::Elided;

use crate::input::{self, Source, Unread};
use crate::{report, Exit};

/// The three standard streams, as values a test can replace.
pub(crate) struct Io<'a> {
    pub stdin: Box<dyn Read + 'a>,
    pub stdout: Box<dyn Write + 'a>,
    pub stderr: Box<dyn Write + 'a>,
}

impl Io<'static> {
    /// The process's own streams. Standard output is locked for the run —
    /// one product, written in one piece.
    pub(crate) fn standard() -> Self {
        Self {
            stdin: Box::new(io::stdin().lock()),
            stdout: Box::new(io::stdout().lock()),
            stderr: Box::new(io::stderr()),
        }
    }
}

/// Where `clean`'s result goes. Decided before anything is read.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Destination {
    /// `name.cleaned.ext` beside the input — the Retention page's default,
    /// spelled by the same `with_infix`.
    Beside(PathBuf),
    /// The file `-o` named.
    Out(PathBuf),
    /// Standard output: `-o -`, or standard input with no `-o`.
    Stdout,
}

impl Destination {
    /// The log's word for it.
    fn label(&self) -> &'static str {
        match self {
            Self::Beside(_) => "beside",
            Self::Out(_) => "out",
            Self::Stdout => "stdout",
        }
    }

    fn file(&self) -> Option<&Path> {
        match self {
            Self::Beside(path) | Self::Out(path) => Some(path),
            Self::Stdout => None,
        }
    }
}

/// `inspect <path|-> [--json]`. Never writes a file.
pub(crate) fn inspect(path: &str, json: bool, io: &mut Io) -> Exit {
    let source = Source::of(path);
    let label = label_of(&source, path);
    let read = match input::read(&source, &mut io.stdin) {
        Ok(read) => read,
        Err(unread) => return refuse_unread("inspect", path, &label, &unread, io),
    };
    say_note(&read, &label, io);

    let options = Options::default();
    let report = wipemark_core::inspect(&read.text, &options);
    let exit = if report.suspicious {
        Exit::Findings
    } else {
        Exit::Clean
    };

    let out = if json {
        format!("{}\n", report.to_json())
    } else {
        joined(report::inspect_lines(
            &say,
            &label,
            &report,
            &options,
            read.encoding,
        ))
    };
    if let Err(error) = emit(&mut io.stdout, out.as_bytes()) {
        return failed("inspect", path, "stdout", Some(error.kind()), Exit::Partial);
    }

    tracing::info!(
        command = "inspect",
        input = %Elided::from(path),
        encoding = read.encoding.name(),
        bytes = read.text.len(),
        findings = report.findings.len(),
        kept = report.kept.len(),
        suspicious = report.suspicious,
        aggressive = options.aggressive,
        nfkc = options.nfkc,
        exit = exit as u8,
        "done"
    );
    exit
}

/// `clean <path|-> [-o <out>|-o -] [--nfkc] [--aggressive] [--json]`.
pub(crate) fn clean(
    path: &str,
    out: Option<&Path>,
    nfkc: bool,
    aggressive: bool,
    json: bool,
    io: &mut Io,
) -> Exit {
    let source = Source::of(path);
    let label = label_of(&source, path);

    // Where the result goes, before a byte is read: a destination that
    // would be refused should not cost a read of the input first.
    let destination = match (&source, out) {
        (_, Some(out)) if out == Path::new("-") => Destination::Stdout,
        (_, Some(out)) => {
            if out.is_dir() {
                let line = t_args(
                    Message::CliOutIsAFolder,
                    &args!("path" => out.display().to_string()),
                );
                return refused(io, "clean", path, &line, "out is a folder", Exit::Usage);
            }
            if let Source::File(input) = &source {
                if input::same_file(input, out) {
                    let line = t_args(
                        Message::CliOutIsInput,
                        &args!("path" => out.display().to_string()),
                    );
                    return refused(io, "clean", path, &line, "out is input", Exit::Usage);
                }
            }
            Destination::Out(out.to_owned())
        }
        (Source::Stdin, None) => Destination::Stdout,
        (Source::File(input), None) => match input.file_name() {
            Some(name) => Destination::Beside(
                input.with_file_name(with_infix(&name.to_string_lossy(), RESULT_INFIX)),
            ),
            // No last component — `..`, `/`. Nothing of that shape is a
            // file, and the read below says what it is instead.
            None => Destination::Stdout,
        },
    };

    let read = match input::read(&source, &mut io.stdin) {
        Ok(read) => read,
        Err(unread) => return refuse_unread("clean", path, &label, &unread, io),
    };
    if destination == Destination::Stdout && out.is_none() && matches!(source, Source::File(_)) {
        // The no-name case above: a path with no last component that
        // still read as a file. Unreachable on every platform this
        // ships to; refused rather than printed somewhere unasked.
        let line = t_args(Message::CliIsAFolder, &args!("path" => path));
        return refused(io, "clean", path, &line, "folder", Exit::Usage);
    }
    say_note(&read, &label, io);

    let options = Options {
        aggressive,
        nfkc,
        ..Options::default()
    };
    let cleaned = wipemark_core::clean(&read.text, &options);
    // Over the input, as A §7.3 asks — never over the result.
    let exit = if cleaned.report.suspicious {
        Exit::Findings
    } else {
        Exit::Clean
    };

    // The result first: the report says where it is, so it must be there.
    if let Some(file) = destination.file() {
        let bytes = input::encode(&cleaned.text, read.encoding);
        let model = match &source {
            Source::File(input) => Some(input.as_path()),
            Source::Stdin => None,
        };
        if let Err(error) = write_atomically(file, &bytes, model) {
            let line = t_args(
                Message::CliWriteFailed,
                &args!(
                    "path" => file.display().to_string(),
                    "reason" => error.to_string(),
                ),
            );
            let _ = writeln!(io.stderr, "wipemark-cli: {line}");
            return failed(
                "clean",
                path,
                "write failed",
                Some(error.kind()),
                Exit::Partial,
            );
        }
    }

    let report_json = || cleaned.report.to_json();
    let human = || {
        let written = destination.file().map(|file| file.display().to_string());
        joined(report::clean_lines(
            &say,
            &label,
            &cleaned.report,
            &options,
            read.encoding,
            written.as_deref(),
            matches!(source, Source::File(_)),
        ))
    };
    let stdout = match (&destination, json) {
        (Destination::Stdout, false) => {
            // The text is the product; the report goes beside it.
            let _ = emit(&mut io.stderr, human().as_bytes());
            cleaned.text.clone().into_bytes()
        }
        (Destination::Stdout, true) => format!(
            r#"{{"report":{},"text":{}}}"#,
            report_json(),
            serde_json::to_string(&cleaned.text).unwrap_or_default()
        )
        .into_bytes(),
        (_, false) => human().into_bytes(),
        (_, true) => {
            let written = destination
                .file()
                .map(|file| file.to_string_lossy().into_owned())
                .unwrap_or_default();
            format!(
                r#"{{"report":{},"written":{}}}"#,
                report_json(),
                serde_json::to_string(&written).unwrap_or_default()
            )
            .into_bytes()
        }
    };
    let stdout = if json {
        let mut line = stdout;
        line.push(b'\n');
        line
    } else {
        stdout
    };
    if let Err(error) = emit(&mut io.stdout, &stdout) {
        return failed("clean", path, "stdout", Some(error.kind()), Exit::Partial);
    }

    tracing::info!(
        command = "clean",
        input = %Elided::from(path),
        encoding = read.encoding.name(),
        bytes = read.text.len(),
        findings = cleaned.report.findings.len(),
        kept = cleaned.report.kept.len(),
        suspicious = cleaned.report.suspicious,
        aggressive,
        nfkc,
        to = destination.label(),
        exit = exit as u8,
        "done"
    );
    exit
}

/// Write `bytes` to `destination` without ever writing into an existing
/// file: a temporary file in the same folder, then a rename over the
/// destination. A rename replaces a directory entry rather than writing
/// into the inode behind it, so even a hard link to the input is left
/// alone — and a run that fails halfway leaves no half-written result.
/// The input's permissions are copied onto the result when there was an
/// input file.
fn write_atomically(destination: &Path, bytes: &[u8], model: Option<&Path>) -> io::Result<()> {
    let folder = match destination.parent() {
        Some(folder) if !folder.as_os_str().is_empty() => folder.to_owned(),
        _ => PathBuf::from("."),
    };
    let name = destination
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temporary = folder.join(format!(".{name}.wipemark-{}.tmp", std::process::id()));

    let written = (|| {
        let mut file = File::create_new(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        if let Some(model) = model {
            std::fs::set_permissions(&temporary, std::fs::metadata(model)?.permissions())?;
        }
        std::fs::rename(&temporary, destination)
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    written
}

/// What a report calls the input: the path exactly as typed, or the
/// catalogue's words for standard input.
fn label_of(source: &Source, path: &str) -> String {
    match source {
        Source::Stdin => t(Message::CliReportStdin),
        Source::File(_) => path.to_owned(),
    }
}

/// The production [`report::Say`]: the process-wide catalogue, which
/// `main` installed in `Rendering::PlainText`.
fn say(message: Message, args: &FluentArgs) -> String {
    t_args(message, args)
}

fn joined(lines: Vec<String>) -> String {
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

/// Write and flush. `write_all`, never `println!`, which panics on a
/// closed pipe — and a hook that closed its end early is owed an exit
/// code, not a panic.
fn emit(stream: &mut Box<dyn Write + '_>, bytes: &[u8]) -> io::Result<()> {
    stream.write_all(bytes)?;
    stream.flush()
}

/// The note a file whose name lies gets: read by its contents, and said.
fn say_note(read: &input::Read, label: &str, io: &mut Io) {
    if let Some((named, found)) = read.note {
        let line = t_args(
            Message::CliNameDisagrees,
            &args!(
                "path" => label,
                "named" => named.name(),
                "found" => found.name(),
            ),
        );
        let _ = writeln!(io.stderr, "wipemark-cli: {line}");
    }
}

/// Why a text was not read, on stderr, and the exit code that goes with
/// it: 2 for a path that is not there or is a folder, 3 — "not read is
/// not clean" — for everything else.
fn refuse_unread(command: &str, path: &str, label: &str, unread: &Unread, io: &mut Io) -> Exit {
    let (line, exit, kind) = match unread {
        Unread::Missing => (
            t_args(Message::CliNoSuchFile, &args!("path" => label)),
            Exit::Usage,
            None,
        ),
        Unread::Folder => (
            t_args(Message::CliIsAFolder, &args!("path" => label)),
            Exit::Usage,
            None,
        ),
        Unread::Unreadable(error) => (
            t_args(
                Message::CliUnreadable,
                &args!("path" => label, "reason" => error.to_string()),
            ),
            Exit::Partial,
            Some(error.kind()),
        ),
        Unread::NotText { found, named } => (
            // A format that is itself text (an RTF whose head is not
            // readable characters) is not one to name as "not text".
            // What the name claimed goes to the log: the refusal is
            // about the contents, and they decide.
            match found.filter(|format| !format.is_textual()) {
                Some(format) => t_args(
                    Message::CliNotText,
                    &args!("path" => label, "format" => format.name()),
                ),
                None => t_args(Message::CliNotTextUnknown, &args!("path" => label)),
            },
            Exit::Partial,
            {
                tracing::info!(
                    found = found.map(|format| format.name()),
                    named = named.map(|format| format.name()),
                    "not text"
                );
                None
            },
        ),
        Unread::UnnamedEncoding => (
            t_args(Message::CliUnnamedEncoding, &args!("path" => label)),
            Exit::Partial,
            None,
        ),
        Unread::Invalid { encoding, offset } => (
            t_args(
                Message::CliInvalidEncoding,
                &args!(
                    "path" => label,
                    "encoding" => encoding.name(),
                    "offset" => offset.to_string(),
                ),
            ),
            Exit::Partial,
            None,
        ),
    };
    let _ = writeln!(io.stderr, "wipemark-cli: {line}");
    failed(command, path, unread.reason(), kind, exit)
}

/// A refusal about the arguments: the line on stderr, the log, the code.
fn refused(io: &mut Io, command: &str, path: &str, line: &str, reason: &str, exit: Exit) -> Exit {
    let _ = writeln!(io.stderr, "wipemark-cli: {line}");
    failed(command, path, reason, None, exit)
}

/// The log line for a run that did not finish. An `io::ErrorKind`, never
/// the operating system's message, which can carry a path.
fn failed(
    command: &str,
    path: &str,
    reason: &str,
    kind: Option<io::ErrorKind>,
    exit: Exit,
) -> Exit {
    tracing::warn!(
        command,
        input = %Elided::from(path),
        reason,
        error = ?kind,
        exit = exit as u8,
        "not done"
    );
    exit
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{clean, inspect, Io};
    use crate::Exit;

    /// Run a flow over in-memory streams and hand back what it wrote.
    fn run(stdin: &[u8], flow: impl FnOnce(&mut Io) -> Exit) -> (Exit, Vec<u8>, Vec<u8>) {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = {
            let mut io = Io {
                stdin: Box::new(Cursor::new(stdin.to_vec())),
                stdout: Box::new(&mut stdout),
                stderr: Box::new(&mut stderr),
            };
            flow(&mut io)
        };
        (exit, stdout, stderr)
    }

    /// Standard input in, the cleaned text out — and the report beside
    /// it on stderr rather than mixed into the product.
    #[test]
    fn standard_input_is_cleaned_onto_standard_output() {
        let (exit, stdout, stderr) = run("a\u{200B}b".as_bytes(), |io| {
            clean("-", None, false, false, false, io)
        });
        assert_eq!(exit, Exit::Findings);
        assert_eq!(stdout, b"ab");
        assert!(!stderr.is_empty(), "the report went nowhere");
    }

    /// `inspect --json` is the report and nothing else, and an empty
    /// input is a scan that ran over nothing — not a refusal.
    #[test]
    fn an_empty_standard_input_is_read_and_reported() {
        let (exit, stdout, stderr) = run(b"", |io| inspect("-", true, io));
        assert_eq!(exit, Exit::Clean);
        let text = String::from_utf8(stdout).expect("UTF-8");
        assert!(text.starts_with("{\"unicode_version\""), "{text}");
        assert!(text.ends_with("}\n"), "{text}");
        assert!(stderr.is_empty());
    }

    /// Not read is not clean: an eight-bit encoding nobody named exits 3
    /// with nothing on standard output.
    #[test]
    fn an_unnamed_encoding_on_standard_input_is_inconclusive() {
        let (exit, stdout, stderr) = run(b"\xf0\xd2\xc9\xd7\xc5\xd4 hello", |io| {
            inspect("-", false, io)
        });
        assert_eq!(exit, Exit::Partial);
        assert!(stdout.is_empty());
        assert!(!stderr.is_empty());
    }
}
