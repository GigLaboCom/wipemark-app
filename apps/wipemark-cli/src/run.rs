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
//! | `clean --in-place` | no | the file itself | the report | note, refusal |
//! | `clean --in-place` | yes | the file itself | `{"report":…,"written":{"path","original"}\|null}` | note, refusal |
//!
//! `--in-place` is the one way the input is ever replaced, and it sets the
//! original aside first (`inplace`). When the cleaned text is the input's
//! text, nothing is written and nothing is set aside — `"written"` is
//! `null` — and the exit code is still core's: a homoglyph kept without
//! `--aggressive` is a finding with no change, and exits 1 over a file
//! that was not touched.
//!
//! # The exit code reads core's answer
//!
//! `inspect` exits by `report.suspicious`, `clean` by
//! `cleaned.report.suspicious` — which core computes over the **input**
//! (D28, A §7.3): a `clean` that removed every mark still exits 1,
//! because a pre-commit hook wants to know what was there. There is no
//! second pass over the result and no second spelling of the rule here.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use wipemark_core::Options;
use wipemark_i18n::{args, t, t_args, FluentArgs, Message};
use wipemark_intake::name::{with_infix, RESULT_INFIX};
use wipemark_log::Elided;

use crate::inplace::{self, Failure, Keep};
use crate::input::{self, Source, Unread};
use crate::report::{Say, Written};
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
    /// `--in-place`: the input file itself, its original set aside first
    /// unless `--no-original`.
    InPlace(PathBuf, Keep),
}

impl Destination {
    /// The log's word for it.
    fn label(&self) -> &'static str {
        match self {
            Self::Beside(_) => "beside",
            Self::Out(_) => "out",
            Self::Stdout => "stdout",
            Self::InPlace(_, Keep::Original) => "in place",
            Self::InPlace(_, Keep::Nothing) => "in place, no original",
        }
    }

    /// A file written beside or to `-o` — not the in-place file, which
    /// goes through [`inplace::replace`].
    fn file(&self) -> Option<&Path> {
        match self {
            Self::Beside(path) | Self::Out(path) => Some(path),
            Self::Stdout | Self::InPlace(..) => None,
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

/// `clean <path|-> [-o <out>|-o -|--in-place [--no-original]] [--nfkc]
/// [--aggressive] [--json]`. `in_place` is `None` without `--in-place`.
pub(crate) fn clean(
    path: &str,
    out: Option<&Path>,
    in_place: Option<Keep>,
    nfkc: bool,
    aggressive: bool,
    json: bool,
    io: &mut Io,
) -> Exit {
    let source = Source::of(path);
    let label = label_of(&source, path);

    // Where the result goes, before a byte is read: a destination that
    // would be refused should not cost a read of the input first.
    let destination = match (&source, out, in_place) {
        (Source::Stdin, _, Some(_)) => {
            let line = t(Message::CliInPlaceStdin);
            return refused(io, "clean", path, &line, "in place from stdin", Exit::Usage);
        }
        (Source::File(input), _, Some(keep)) => {
            // A link would be renamed aside as a link and replaced by a
            // file, leaving the file it pointed at as it was — a run that
            // reports success over a document it did not change.
            if std::fs::symlink_metadata(input).is_ok_and(|meta| meta.file_type().is_symlink()) {
                let line = t_args(Message::CliInPlaceLink, &args!("path" => label.as_str()));
                return refused(io, "clean", path, &line, "in place on a link", Exit::Usage);
            }
            Destination::InPlace(input.clone(), keep)
        }
        (_, out, None) => match (&source, out) {
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
    let mut replaced = None;
    if let Destination::InPlace(file, keep) = &destination {
        // Nothing changed is nothing touched: no original set aside, no
        // write, the modification time where it was.
        if cleaned.text != read.text {
            let bytes = input::encode(&cleaned.text, read.encoding);
            match inplace::replace(file, &bytes, *keep) {
                Ok(done) => replaced = Some(done),
                Err(failure) => return refuse_replacement(io, path, file, &failure),
            }
        }
    }
    if let Some(file) = destination.file() {
        let bytes = input::encode(&cleaned.text, read.encoding);
        let model = match &source {
            Source::File(input) => Some(input.as_path()),
            Source::Stdin => None,
        };
        if let Err(error) = inplace::write_atomically(file, &bytes, model) {
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
    let from_file = matches!(source, Source::File(_));
    let file_shown = destination.file().map(|file| file.display().to_string());
    let in_place_shown = match (&destination, &replaced) {
        (Destination::InPlace(file, _), Some(done)) => Some((
            file.display().to_string(),
            done.original
                .as_ref()
                .map(|original| original.display().to_string()),
        )),
        _ => None,
    };
    let written = match (&destination, &in_place_shown, &file_shown) {
        (Destination::InPlace(..), Some((path, original)), _) => Written::Replaced {
            path,
            original: original.as_deref(),
        },
        (Destination::InPlace(..), None, _) => Written::Unchanged,
        (_, _, Some(path)) => Written::File { path, from_file },
        (_, _, None) => Written::Stdout { from_file },
    };
    let human = || {
        joined(report::clean_lines(
            &say,
            &label,
            &cleaned.report,
            &options,
            read.encoding,
            written,
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
        (Destination::InPlace(file, _), true) => {
            let written = match &replaced {
                Some(done) => serde_json::json!({
                    "path": file.to_string_lossy(),
                    "original": done.original.as_ref().map(|original| original.to_string_lossy()),
                }),
                None => serde_json::Value::Null,
            };
            format!(r#"{{"report":{},"written":{}}}"#, report_json(), written).into_bytes()
        }
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
pub(crate) fn say(message: Message, args: &FluentArgs) -> String {
    t_args(message, args)
}

pub(crate) fn joined(lines: Vec<String>) -> String {
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

/// Write and flush. `write_all`, never `println!`, which panics on a
/// closed pipe — and a hook that closed its end early is owed an exit
/// code, not a panic.
pub(crate) fn emit(stream: &mut Box<dyn Write + '_>, bytes: &[u8]) -> io::Result<()> {
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

/// Why a text was not read, in one sentence — the line `inspect` and
/// `clean` refuse with, and the line `audit` lists an unreadable file
/// under. `say` is the catalogue in the user's language, or in English
/// for SARIF, which is a format.
pub(crate) fn unread_line(say: Say, label: &str, unread: &Unread) -> String {
    match unread {
        Unread::Missing => say(Message::CliNoSuchFile, &args!("path" => label)),
        Unread::Folder => say(Message::CliIsAFolder, &args!("path" => label)),
        Unread::Unreadable(error) => say(
            Message::CliUnreadable,
            &args!("path" => label, "reason" => error.to_string()),
        ),
        // A format that is itself text (an RTF whose head is not readable
        // characters) is not one to name as "not text".
        Unread::NotText { found, .. } => match found.filter(|format| !format.is_textual()) {
            Some(format) => say(
                Message::CliNotText,
                &args!("path" => label, "format" => format.name()),
            ),
            None => say(Message::CliNotTextUnknown, &args!("path" => label)),
        },
        Unread::UnnamedEncoding => say(Message::CliUnnamedEncoding, &args!("path" => label)),
        Unread::Invalid { encoding, offset } => say(
            Message::CliInvalidEncoding,
            &args!(
                "path" => label,
                "encoding" => encoding.name(),
                "offset" => offset.to_string(),
            ),
        ),
    }
}

/// Why a text was not read, on stderr, and the exit code that goes with
/// it: 2 for a path that is not there or is a folder, 3 — "not read is
/// not clean" — for everything else.
fn refuse_unread(command: &str, path: &str, label: &str, unread: &Unread, io: &mut Io) -> Exit {
    let (exit, kind) = match unread {
        Unread::Missing | Unread::Folder => (Exit::Usage, None),
        Unread::Unreadable(error) => (Exit::Partial, Some(error.kind())),
        Unread::NotText { found, named } => {
            // What the name claimed goes to the log: the refusal is about
            // the contents, and they decide.
            tracing::info!(
                found = found.map(|format| format.name()),
                named = named.map(|format| format.name()),
                "not text"
            );
            (Exit::Partial, None)
        }
        Unread::UnnamedEncoding | Unread::Invalid { .. } => (Exit::Partial, None),
    };
    let line = unread_line(&say, label, unread);
    let _ = writeln!(io.stderr, "wipemark-cli: {line}");
    failed(command, path, unread.reason(), kind, exit)
}

/// An in-place replacement that did not happen, on stderr. Exit 2 every
/// time: the file is as it was — or, when even putting it back failed,
/// the line says where the original is now.
fn refuse_replacement(io: &mut Io, path: &str, file: &Path, failure: &Failure) -> Exit {
    let shown = file.display().to_string();
    let (line, reason, kind) = match failure {
        Failure::OriginalExists(original) => (
            t_args(
                Message::CliInPlaceOriginalExists,
                &args!("path" => shown, "original" => original.display().to_string()),
            ),
            "original exists",
            None,
        ),
        Failure::Unnamed => (
            t_args(Message::CliIsAFolder, &args!("path" => shown)),
            "unnamed",
            None,
        ),
        Failure::SetAside { original, error } => (
            t_args(
                Message::CliInPlaceSetAsideFailed,
                &args!(
                    "path" => shown,
                    "original" => original.display().to_string(),
                    "reason" => error.to_string(),
                ),
            ),
            "set aside failed",
            Some(error.kind()),
        ),
        Failure::Write(error) => (
            t_args(
                Message::CliInPlaceWriteFailed,
                &args!("path" => shown, "reason" => error.to_string()),
            ),
            "write failed",
            Some(error.kind()),
        ),
        Failure::Stranded {
            original,
            error,
            restore,
        } => (
            t_args(
                Message::CliInPlaceStranded,
                &args!(
                    "path" => shown,
                    "original" => original.display().to_string(),
                    "reason" => error.to_string(),
                    "restore" => restore.to_string(),
                ),
            ),
            "stranded",
            Some(error.kind()),
        ),
    };
    let _ = writeln!(io.stderr, "wipemark-cli: {line}");
    failed("clean", path, reason, kind, Exit::Usage)
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
            clean("-", None, None, false, false, false, io)
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
