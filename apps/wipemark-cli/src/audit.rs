//! `audit <dir> [--json | --sarif]`: every text file under a folder, read
//! the way `inspect` reads one, for a pre-commit hook or a CI step — and
//! every PNG, JPEG and WebP, whose metadata `wipemark-image` inspects the
//! way `inspect` does one picture (D139).
//!
//! # What is walked
//!
//! `std::fs::read_dir`, recursively, with no depth limit and three rules
//! that keep it finite and honest:
//!
//! * **A hidden entry is skipped** — a name starting with `.`, which is
//!   `.git` above all. It is counted and listed, and never opened.
//! * **A link to a folder is not followed.** A link back up the tree is a
//!   loop and a link out of it is somebody else's tree; not following is
//!   what makes the walk terminate without a visited set. A link to a
//!   *file* is read like the file.
//! * **Only regular files are opened.** A FIFO would block the walk
//!   forever, a device is not a document; both are listed as skipped.
//!
//! `.gitignore` is not read: a later flag if anyone asks
//! (`docs/architecture/cli.md`).
//!
//! # Three outcomes per file, and the exit code
//!
//! * **scanned** — text, decoded, `wipemark_core::inspect` ran; or a
//!   picture, `wipemark_image::inspect` ran over its metadata;
//! * **skipped** — not text, empty, hidden, a link to a folder, not a
//!   regular file, a TIFF, HEIC or AVIF (not in this version yet):
//!   counted, listed in `--json`, never an error;
//! * **unreadable** — permission denied, an I/O error, an eight-bit
//!   encoding this version does not name, an invalid sequence in a file
//!   intake called text, a picture this version could not read, a folder
//!   that could not be listed.
//!
//! Exit **3** if anything was unreadable — even when another file had
//! findings, because a hook must not read a scan with a hole in it as a
//! complete one ("inconclusive is not clean", and it beats 1); else **1**
//! if any report is suspicious or any picture carries AI provenance
//! (exactly `inspect`'s rules); else **0**.
//! A `<dir>` that is not there or is not a folder is **2**.
//!
//! # Three renderings of one walk
//!
//! The human report (stdout) names each file whose report is suspicious,
//! then one summary line, then the unreadable files with why, then the
//! Unicode version and the third shelf — once, for the whole walk.
//! `--json` carries every file with `inspect --json`'s report spliced in
//! byte for byte, third shelf included. `--sarif` is SARIF 2.1.0 for code
//! scanning, with columns in code points — see [`locate`] and
//! [`sarif`] — and, for a picture, a byte region: a metadata block has an
//! offset and a length and no line. Its text is English whatever the user's language: SARIF is a
//! format, read by dashboards, and a rule description that changed with
//! the locale of the CI runner would be a rule nobody could match.
//!
//! Never a file's text, anywhere: not on stdout, not in the log.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;

use wipemark_core::report::not_established;
use wipemark_core::{Confidence, InspectReport, Options, UnicodeClass, UnicodeFinding};
use wipemark_i18n::{args, FluentArgs, LanguageIdentifier, Localizer, Message, Rendering};
use wipemark_image::{ImageError, ImageReport, MetadataFinding, MetadataKind, Signal};
use wipemark_log::Elided;

use crate::input::{self, Content, Source, Unread};
use crate::report::{self, Say};
use crate::run::{self, Io};
use crate::{image, Exit};

/// Which of the three renderings. `--json` and `--sarif` conflict in clap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Output {
    Human,
    Json,
    Sarif,
}

/// Why a file was skipped. The id is a format (`--json`'s `reason`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Skip {
    Hidden,
    LinkToFolder,
    NotAFile,
    NotText,
    Empty,
    /// A TIFF, HEIC or AVIF: a picture this version does not open yet.
    ImageNotYet,
}

impl Skip {
    fn id(self) -> &'static str {
        match self {
            Skip::Hidden => "hidden",
            Skip::LinkToFolder => "link-to-folder",
            Skip::NotAFile => "not-a-file",
            Skip::NotText => "not-text",
            Skip::Empty => "empty",
            Skip::ImageNotYet => "image-not-yet",
        }
    }
}

/// What became of one entry.
enum Status {
    Scanned {
        report: InspectReport,
        /// Where each finding is, for SARIF — computed only when asked.
        spots: Vec<Spot>,
    },
    /// A picture whose metadata was read.
    Image(ImageReport),
    Skipped(Skip),
    Unreadable(Unread),
    /// A picture this version could not read.
    ImageUnreadable(ImageError),
}

impl Status {
    fn scanned(&self) -> bool {
        matches!(self, Status::Scanned { .. } | Status::Image(_))
    }

    fn with_findings(&self) -> bool {
        match self {
            Status::Scanned { report, .. } => report.suspicious,
            Status::Image(report) => report.has_ai_metadata(),
            _ => false,
        }
    }

    fn unreadable(&self) -> bool {
        matches!(self, Status::Unreadable(_) | Status::ImageUnreadable(_))
    }
}

struct Entry {
    /// Relative to the root, `/`-separated; a folder ends in `/`.
    path: String,
    status: Status,
}

/// `audit <dir>`.
pub(crate) fn run(dir: &Path, output: Output, io: &mut Io) -> Exit {
    let shown = dir.display().to_string();
    match std::fs::metadata(dir) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => {
            let line = run::say(
                Message::CliAuditNotAFolder,
                &args!("path" => shown.as_str()),
            );
            return refuse(io, dir, &line);
        }
        Err(_) => {
            let line = run::say(Message::CliNoSuchFile, &args!("path" => shown.as_str()));
            return refuse(io, dir, &line);
        }
    }

    let mut entries = Vec::new();
    walk(dir, "", output == Output::Sarif, &mut entries);

    let scanned = entries
        .iter()
        .filter(|entry| entry.status.scanned())
        .count();
    let with_findings = entries
        .iter()
        .filter(|entry| entry.status.with_findings())
        .count();
    let skipped = entries
        .iter()
        .filter(|entry| matches!(entry.status, Status::Skipped(_)))
        .count();
    let unreadable = entries
        .iter()
        .filter(|entry| entry.status.unreadable())
        .count();
    let summary = Summary {
        scanned,
        with_findings,
        skipped,
        unreadable,
    };
    // Unreadable first: inconclusive beats a finding.
    let exit = if unreadable > 0 {
        Exit::Partial
    } else if with_findings > 0 {
        Exit::Findings
    } else {
        Exit::Clean
    };

    let text = match output {
        Output::Human => human(&run::say, &shown, &entries, &summary),
        Output::Json => format!("{}\n", json(&shown, &entries, &summary)),
        Output::Sarif => format!("{}\n", ascii(&sarif(&entries))),
    };
    if let Err(error) = run::emit(&mut io.stdout, text.as_bytes()) {
        tracing::warn!(command = "audit", error = ?error.kind(), exit = 3, "not done");
        return Exit::Partial;
    }

    tracing::info!(
        command = "audit",
        root = %Elided::from(dir.to_string_lossy()),
        scanned,
        with_findings,
        skipped,
        unreadable,
        output = ?output,
        exit = exit as u8,
        "done"
    );
    exit
}

fn refuse(io: &mut Io, dir: &Path, line: &str) -> Exit {
    let _ = writeln!(io.stderr, "wipemark-cli: {line}");
    tracing::warn!(
        command = "audit",
        root = %Elided::from(dir.to_string_lossy()),
        exit = 2,
        "not a folder"
    );
    Exit::Usage
}

/// Walk `dir`, appending one entry per thing found, sorted by name so
/// that two runs over one tree print one report.
fn walk(dir: &Path, prefix: &str, locate_findings: bool, entries: &mut Vec<Entry>) {
    let listing = match std::fs::read_dir(dir) {
        Ok(listing) => listing,
        Err(error) => {
            // The root was checked by the caller; this is a folder inside
            // it. What is in it was not read, and that is a hole.
            entries.push(Entry {
                path: format!("{prefix}/"),
                status: Status::Unreadable(Unread::Unreadable(error)),
            });
            return;
        }
    };
    let mut found: Vec<std::fs::DirEntry> = Vec::new();
    for item in listing {
        match item {
            Ok(item) => found.push(item),
            Err(error) => entries.push(Entry {
                path: format!("{prefix}/"),
                status: Status::Unreadable(Unread::Unreadable(error)),
            }),
        }
    }
    found.sort_by_key(std::fs::DirEntry::file_name);

    for item in found {
        let name = item.file_name().to_string_lossy().into_owned();
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let full = item.path();
        if name.starts_with('.') {
            entries.push(skipped(path, Skip::Hidden));
            continue;
        }
        // `file_type` does not follow a link: that is what tells a linked
        // folder apart from a real one.
        let kind = match item.file_type() {
            Ok(kind) => kind,
            Err(error) => {
                entries.push(Entry {
                    path,
                    status: Status::Unreadable(Unread::Unreadable(error)),
                });
                continue;
            }
        };
        if kind.is_dir() {
            walk(&full, &path, locate_findings, entries);
        } else if kind.is_symlink() {
            match std::fs::metadata(&full) {
                Ok(meta) if meta.is_dir() => entries.push(skipped(path, Skip::LinkToFolder)),
                Ok(meta) if meta.is_file() => entries.push(scan(&full, path, locate_findings)),
                // A link to nothing, or to a FIFO: there is no document.
                Ok(_) => entries.push(skipped(path, Skip::NotAFile)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    entries.push(skipped(path, Skip::NotAFile));
                }
                Err(error) => entries.push(Entry {
                    path,
                    status: Status::Unreadable(Unread::Unreadable(error)),
                }),
            }
        } else if kind.is_file() {
            entries.push(scan(&full, path, locate_findings));
        } else {
            entries.push(skipped(path, Skip::NotAFile));
        }
    }
}

fn skipped(path: String, skip: Skip) -> Entry {
    Entry {
        path,
        status: Status::Skipped(skip),
    }
}

/// One file, through the path `inspect` takes.
fn scan(full: &Path, path: String, locate_findings: bool) -> Entry {
    let status = match input::read_any(&Source::File(full.to_owned()), &mut std::io::empty()) {
        Ok(Content::Image(picture)) => match wipemark_image::inspect(&picture.bytes) {
            Ok(report) => Status::Image(report),
            // Recognised, and not one this version opens: nothing was
            // looked for, and the listing says which.
            Err(ImageError::NotYet(_) | ImageError::UnknownContainer) => {
                Status::Skipped(Skip::ImageNotYet)
            }
            // A picture that could not be read is a hole in the scan, as
            // an unreadable text is: 3 beats 1.
            Err(error) => Status::ImageUnreadable(error),
        },
        Ok(Content::Text(read)) if read.text.is_empty() => Status::Skipped(Skip::Empty),
        Ok(Content::Text(read)) => {
            let report = wipemark_core::inspect(&read.text, &Options::default());
            let spots = if locate_findings {
                let offsets: Vec<usize> = reported(&report)
                    .flat_map(|row| row.positions.iter().copied())
                    .collect();
                locate(&read.text, &offsets)
            } else {
                Vec::new()
            };
            Status::Scanned { report, spots }
        }
        // Intake did not call it text: there is nothing for Layer A here.
        Err(Unread::NotText { .. }) => Status::Skipped(Skip::NotText),
        Err(unread) => Status::Unreadable(unread),
    };
    Entry { path, status }
}

/// The rows a report shows as findings: everything `clean` would act on,
/// and what it would keep at a confidence that makes the text suspicious
/// (a homoglyph left in place without `--aggressive`). What is kept as
/// orthography or presentation is not a finding, here or in `inspect`.
fn reported(report: &InspectReport) -> impl Iterator<Item = &UnicodeFinding> {
    report.findings.iter().chain(
        report
            .kept
            .iter()
            .filter(|row| row.confidence >= Confidence::Probable),
    )
}

struct Summary {
    scanned: usize,
    with_findings: usize,
    skipped: usize,
    unreadable: usize,
}

/// The report a person reads.
fn human(say: Say, root: &str, entries: &[Entry], summary: &Summary) -> String {
    let mut lines = Vec::new();
    for entry in entries {
        if let Status::Image(report) = &entry.status {
            if let Some(line) = image_line(say, &entry.path, report) {
                lines.push(line);
            }
            continue;
        }
        let Status::Scanned { report, .. } = &entry.status else {
            continue;
        };
        if !report.suspicious {
            continue;
        }
        let mut counted: Vec<(UnicodeClass, u64)> = Vec::new();
        for row in report.findings.iter().chain(&report.kept) {
            match counted.iter_mut().find(|(class, _)| *class == row.class) {
                Some((_, count)) => *count += u64::from(row.count),
                None => counted.push((row.class, u64::from(row.count))),
            }
        }
        counted.sort_by_key(|(class, _)| UnicodeClass::ALL.iter().position(|c| c == class));
        let total: u64 = counted.iter().map(|(_, count)| count).sum();
        let classes = counted
            .iter()
            .map(|(class, count)| {
                format!(
                    "{} ×{count}",
                    say(report::class_label(*class), &FluentArgs::new())
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(say(
            Message::CliAuditFile,
            &args!("path" => entry.path.as_str(), "count" => total, "classes" => classes),
        ));
    }
    lines.push(say(
        Message::CliAuditSummary,
        &args!(
            "root" => root,
            "scanned" => summary.scanned,
            "findings" => summary.with_findings,
            "skipped" => summary.skipped,
            "unreadable" => summary.unreadable,
        ),
    ));
    let unreadable: Vec<String> = entries
        .iter()
        .filter_map(|entry| match &entry.status {
            Status::Unreadable(unread) => {
                Some(format!("  {}", run::unread_line(say, &entry.path, unread)))
            }
            Status::ImageUnreadable(error) => {
                Some(format!("  {}", image::error_line(say, &entry.path, error)))
            }
            _ => None,
        })
        .collect();
    if !unreadable.is_empty() {
        lines.push(say(Message::CliAuditUnreadableTitle, &FluentArgs::new()));
        lines.extend(unreadable);
    }
    lines.extend(report::footer(say, wipemark_core::UNICODE_VERSION));
    run::joined(lines)
}

/// The line a picture with AI provenance gets: how many blocks, and of
/// which kinds. `None` for a picture with none.
fn image_line(say: Say, path: &str, report: &ImageReport) -> Option<String> {
    let mut counted: Vec<(MetadataKind, u64)> = Vec::new();
    for finding in report.findings.iter().filter(|f| f.is_ai_provenance()) {
        match counted.iter_mut().find(|(kind, _)| *kind == finding.kind) {
            Some((_, count)) => *count += 1,
            None => counted.push((finding.kind, 1)),
        }
    }
    if counted.is_empty() {
        return None;
    }
    counted.sort_by_key(|(kind, _)| MetadataKind::ALL.iter().position(|k| k == kind));
    let total: u64 = counted.iter().map(|(_, count)| count).sum();
    let kinds = counted
        .iter()
        .map(|(kind, count)| {
            format!(
                "{} ×{count}",
                say(image::kind_label(*kind), &FluentArgs::new())
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    Some(say(
        Message::CliAuditImage,
        &args!(
            "path" => path,
            "container" => report.container.name(),
            "count" => total,
            "kinds" => kinds,
        ),
    ))
}

/// The id `--json` gives an unreadable file's reason. A format.
fn unread_id(unread: &Unread) -> &'static str {
    match unread {
        Unread::Missing => "missing",
        Unread::Folder => "folder",
        Unread::Unreadable(_) => "unreadable",
        Unread::NotText { .. } => "not-text",
        Unread::UnnamedEncoding => "unnamed-encoding",
        Unread::Invalid { .. } => "invalid",
    }
}

/// `--json`: one object, one line, ASCII. Each scanned file's report is
/// `InspectReport::to_json` spliced in as it is — the very bytes
/// `inspect --json` prints — so the third shelf is in it, always.
fn json(root: &str, entries: &[Entry], summary: &Summary) -> String {
    let string = |text: &str| ascii(&serde_json::Value::from(text));
    let mut out = format!(
        r#"{{"version":1,"unicode":{},"root":{},"files":["#,
        string(wipemark_core::UNICODE_VERSION),
        string(root)
    );
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let (status, reason, report) = match &entry.status {
            Status::Scanned { report, .. } => ("scanned", "null".to_owned(), report.to_json()),
            // The picture's own report, spliced as `inspect --json` prints
            // it — `container` where a text's has `unicode_version`.
            Status::Image(report) => ("scanned", "null".to_owned(), report.to_json()),
            Status::ImageUnreadable(_) => {
                ("unreadable", string("malformed-image"), "null".to_owned())
            }
            Status::Skipped(skip) => ("skipped", string(skip.id()), "null".to_owned()),
            Status::Unreadable(unread) => {
                ("unreadable", string(unread_id(unread)), "null".to_owned())
            }
        };
        let _ = write!(
            out,
            r#"{{"path":{},"status":"{status}","reason":{reason},"report":{report}}}"#,
            string(&entry.path)
        );
    }
    let _ = write!(
        out,
        r#"],"summary":{{"scanned":{},"with_findings":{},"skipped":{},"unreadable":{}}}}}"#,
        summary.scanned, summary.with_findings, summary.skipped, summary.unreadable
    );
    out
}

/// A JSON value spelled in ASCII: every character past U+007F escaped as
/// `\uXXXX` (a surrogate pair above the BMP), as core's own JSON is (D29).
/// Safe as a post-pass because outside a string JSON is ASCII anyway.
pub(crate) fn ascii(value: &serde_json::Value) -> String {
    let spelled = value.to_string();
    let mut out = String::with_capacity(spelled.len());
    for character in spelled.chars() {
        if character.is_ascii() {
            out.push(character);
        } else {
            let mut units = [0u16; 2];
            for unit in character.encode_utf16(&mut units) {
                let _ = write!(out, "\\u{unit:04x}");
            }
        }
    }
    out
}

/// Where a finding is, as SARIF counts it: 1-based line and column in
/// code points, and the offset from the start of the file in code points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Spot {
    pub line: u64,
    pub column: u64,
    pub char_offset: u64,
}

/// Turn byte offsets into `text` (any order) into [`Spot`]s, in the same
/// order.
///
/// Lines end at LF, at CR, and at CR LF counted once — SARIF's own line
/// terminators. Columns and offsets count code points
/// (`run.columnKind = "unicodeCodePoints"`), which is what makes them mean
/// the same thing for a UTF-16 file as for its UTF-8 twin, and what keeps a
/// CJK character from being three columns wide. A byte order mark at the
/// very start is not counted: no editor shows it as a column.
pub(crate) fn locate(text: &str, offsets: &[usize]) -> Vec<Spot> {
    let mut order: Vec<usize> = (0..offsets.len()).collect();
    order.sort_by_key(|&index| offsets[index]);
    let mut spots = vec![
        Spot {
            line: 1,
            column: 1,
            char_offset: 0
        };
        offsets.len()
    ];

    let mut pending = order.into_iter().peekable();
    let mut here = Spot {
        line: 1,
        column: 1,
        char_offset: 0,
    };
    let mut after_cr = false;
    for (at, character) in text.char_indices() {
        while let Some(&index) = pending.peek() {
            if offsets[index] > at {
                break;
            }
            spots[index] = here;
            pending.next();
        }
        if at == 0 && character == '\u{FEFF}' {
            continue;
        }
        here.char_offset += 1;
        match character {
            '\n' if after_cr => {
                after_cr = false;
            }
            '\n' => {
                here.line += 1;
                here.column = 1;
            }
            '\r' => {
                here.line += 1;
                here.column = 1;
                after_cr = true;
            }
            _ => {
                here.column += 1;
                after_cr = false;
            }
        }
    }
    // An offset at or past the end — never produced by core — lands on
    // the end rather than on line 1.
    for index in pending {
        spots[index] = here;
    }
    spots
}

/// The English catalogue, for SARIF.
fn english() -> Localizer {
    let english: LanguageIdentifier = "en-US".parse().expect("a language tag");
    Localizer::for_languages(&[english], Rendering::PlainText)
}

/// A relative path as a URI reference: every byte outside the unreserved
/// set and `/` percent-encoded, so a space or a non-ASCII name is still a
/// URI a dashboard can resolve against `SRCROOT`.
fn uri(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

fn artifact(path: &str) -> serde_json::Value {
    serde_json::json!({
        "physicalLocation": {
            "artifactLocation": { "uri": uri(path), "uriBaseId": "SRCROOT" }
        }
    })
}

/// SARIF 2.1.0: one run, the classes that occur as rules, one result per
/// occurrence of a finding, and the unreadable files as error
/// notifications on an invocation that is then not successful.
fn sarif(entries: &[Entry]) -> serde_json::Value {
    let english = english();
    let say = |message: Message, args: &FluentArgs| english.format_args(message, args);

    let mut classes: Vec<UnicodeClass> = Vec::new();
    for entry in entries {
        if let Status::Scanned { report, .. } = &entry.status {
            for row in reported(report) {
                if !classes.contains(&row.class) {
                    classes.push(row.class);
                }
            }
        }
    }
    classes.sort_by_key(|class| UnicodeClass::ALL.iter().position(|c| c == class));
    let mut rules: Vec<serde_json::Value> = classes
        .iter()
        .map(|class| {
            serde_json::json!({
                "id": class.as_str(),
                "shortDescription": { "text": say(report::class_label(*class), &FluentArgs::new()) },
            })
        })
        .collect();

    // A picture's rules follow the text's: one per signal that occurs.
    let mut signals: Vec<ImageRule> = Vec::new();
    for entry in entries {
        if let Status::Image(report) = &entry.status {
            for (_, rule) in image_results(report) {
                if !signals.contains(&rule) {
                    signals.push(rule);
                }
            }
        }
    }
    signals.sort_by_key(|rule| rule.order());
    rules.extend(signals.iter().map(|rule| {
        serde_json::json!({
            "id": rule.id(),
            "shortDescription": { "text": say(rule.label(), &FluentArgs::new()) },
        })
    }));

    let mut results = Vec::new();
    let mut notifications = Vec::new();
    for entry in entries {
        match &entry.status {
            Status::Scanned { report, spots } => {
                let mut spots = spots.iter();
                for row in reported(report) {
                    let mut character = format!("U+{:04X}", u32::from(row.codepoint));
                    if let Some(name) = wipemark_core::name_of(row.codepoint) {
                        let _ = write!(character, " {name}");
                    }
                    let text = format!(
                        "{character}: {}, {}",
                        say(report::class_label(row.class), &FluentArgs::new()),
                        say(report::confidence_label(row.confidence), &FluentArgs::new()),
                    );
                    let level = match row.confidence {
                        Confidence::Confirmed => "error",
                        Confidence::Probable => "warning",
                        Confidence::Informational | Confidence::LikelyFalsePositive => "note",
                    };
                    let index = classes.iter().position(|class| *class == row.class);
                    for _ in &row.positions {
                        let Some(spot) = spots.next() else { break };
                        let mut location = artifact(&entry.path);
                        location["physicalLocation"]["region"] = serde_json::json!({
                            "startLine": spot.line,
                            "startColumn": spot.column,
                            "endColumn": spot.column + 1,
                            "charOffset": spot.char_offset,
                            "charLength": 1,
                        });
                        results.push(serde_json::json!({
                            "ruleId": row.class.as_str(),
                            "ruleIndex": index,
                            "level": level,
                            "message": { "text": text },
                            "locations": [location],
                        }));
                    }
                }
            }
            Status::Image(report) => {
                for (finding, rule) in image_results(report) {
                    let index = classes.len()
                        + signals.iter().position(|known| *known == rule).unwrap_or(0);
                    let mut location = artifact(&entry.path);
                    // SARIF 2.1.0 §3.30.13–14: a binary artifact's region
                    // is a byte offset and a length; there is no line.
                    location["physicalLocation"]["region"] = serde_json::json!({
                        "byteOffset": finding.offset,
                        "byteLength": finding.len,
                    });
                    let text = format!(
                        "{}: {}, {}",
                        image::where_of(finding),
                        say(image::kind_label(finding.kind), &FluentArgs::new()),
                        say(rule.label(), &FluentArgs::new()),
                    );
                    results.push(serde_json::json!({
                        "ruleId": rule.id(),
                        "ruleIndex": index,
                        // Verifiable: the block is there, and the signature
                        // in it matched.
                        "level": "error",
                        "message": { "text": text },
                        "locations": [location],
                    }));
                }
            }
            Status::Unreadable(unread) => notifications.push(serde_json::json!({
                "level": "error",
                "message": { "text": run::unread_line(&say, &entry.path, unread) },
                "locations": [artifact(&entry.path)],
            })),
            Status::ImageUnreadable(error) => notifications.push(serde_json::json!({
                "level": "error",
                "message": { "text": image::error_line(&say, &entry.path, error) },
                "locations": [artifact(&entry.path)],
            })),
            Status::Skipped(_) => {}
        }
    }

    serde_json::json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "wipemark",
                    "version": env!("CARGO_PKG_VERSION"),
                    "rules": rules,
                }
            },
            "columnKind": "unicodeCodePoints",
            "invocations": [{
                "executionSuccessful": notifications.is_empty(),
                "toolExecutionNotifications": notifications,
            }],
            "results": results,
            "properties": {
                "unicode": wipemark_core::UNICODE_VERSION,
                "not_established": not_established::ALL.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            },
        }]
    })
}

/// A SARIF rule for a picture: the signal that made a block provenance —
/// one rule per signal, whatever generator or source type it names — or,
/// for a block that is provenance by its kind with no signal recorded
/// (none is, today), the kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ImageRule {
    /// `Signal::id` or `MetadataKind::id`.
    of: &'static str,
    label: Message,
}

impl ImageRule {
    fn signal(signal: Signal) -> Self {
        Self {
            of: signal.id(),
            label: image::signal_label(signal),
        }
    }

    fn kind(kind: MetadataKind) -> Self {
        Self {
            of: kind.id(),
            label: image::kind_label(kind),
        }
    }

    /// `image-` and the library's id: a format, and distinct from every
    /// `UnicodeClass` id a text's rule carries.
    fn id(self) -> String {
        format!("image-{}", self.of)
    }

    fn label(self) -> Message {
        self.label
    }

    /// A rule's place in the list: by id, so two runs list one order.
    fn order(&self) -> &'static str {
        self.of
    }
}

/// Every result a picture gives SARIF: one per signal of every block that
/// is AI provenance — the blocks that make `inspect` exit 1.
fn image_results(report: &ImageReport) -> Vec<(&MetadataFinding, ImageRule)> {
    let mut out = Vec::new();
    for finding in report.findings.iter().filter(|f| f.is_ai_provenance()) {
        let mut rules: Vec<ImageRule> = Vec::new();
        for evidence in &finding.evidence {
            let rule = ImageRule::signal(evidence.signal);
            if !rules.contains(&rule) {
                rules.push(rule);
            }
        }
        if rules.is_empty() {
            rules.push(ImageRule::kind(finding.kind));
        }
        out.extend(rules.into_iter().map(|rule| (finding, rule)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{ascii, locate, uri, Spot};

    fn spot(line: u64, column: u64, char_offset: u64) -> Spot {
        Spot {
            line,
            column,
            char_offset,
        }
    }

    /// Columns in code points: a zero-width space after two CJK
    /// characters is the third column, not the seventh byte.
    #[test]
    fn a_column_counts_code_points_not_bytes() {
        let text = "first line\n\u{6F22}\u{5B57}\u{200B}x\n";
        let at = text.find('\u{200B}').expect("the ZWSP");
        assert_eq!(at, 17, "the fixture moved");
        assert_eq!(locate(text, &[at]), [spot(2, 3, 13)]);
        // An astral character is one code point, four bytes.
        let text = "\u{1F600}\u{200B}";
        assert_eq!(locate(text, &[4]), [spot(1, 2, 1)]);
    }

    /// LF, CR and CR LF each end one line; CR LF is not two.
    #[test]
    fn every_line_terminator_ends_one_line() {
        let text = "a\nb\r\nc\rd\u{200B}";
        let at = text.find('\u{200B}').expect("the ZWSP");
        assert_eq!(locate(text, &[at]), [spot(4, 2, 8)]);
        assert_eq!(locate("\r\n\u{200B}", &[2]), [spot(2, 1, 2)]);
    }

    /// A byte order mark at the start is not a column, and offsets come
    /// back in the order they went in, whatever that was.
    #[test]
    fn a_byte_order_mark_is_not_counted_and_order_is_kept() {
        let text = "\u{FEFF}a\u{200B}b\u{200B}";
        assert_eq!(locate(text, &[8, 4]), [spot(1, 4, 3), spot(1, 2, 1)]);
        assert_eq!(
            locate("ab", &[9]),
            [spot(1, 3, 2)],
            "past the end lands on the end"
        );
    }

    #[test]
    fn a_path_is_a_uri_reference() {
        assert_eq!(uri("docs/a b.md"), "docs/a%20b.md");
        assert_eq!(uri("d\u{e9}j\u{e0}/x.md"), "d%C3%A9j%C3%A0/x.md");
        assert_eq!(uri("plain/x-y_z.~md"), "plain/x-y_z.~md");
    }

    #[test]
    fn json_is_spelled_in_ascii() {
        let value = serde_json::json!({ "path": "d\u{e9}j\u{e0}/\u{1F600}.md" });
        let spelled = ascii(&value);
        assert!(spelled.is_ascii(), "{spelled}");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&spelled).expect("JSON"),
            value
        );
    }
}
