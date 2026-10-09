//! `inspect` and `clean` on a picture: `wipemark-picture` over the bytes
//! `input::read_any` handed over — the metadata (`wipemark-image`) and the
//! visible marks in the pixels (`wipemark-pixels`) with one writer — its
//! report in words, and the exit code.
//!
//! # Exit codes
//!
//! The CLI's four, with one meaning across a text and a picture (D131,
//! D132, D136, D160 as the owner amended it — no flag: marks found are
//! removed):
//!
//! | | `inspect` | `clean` |
//! |---|---|---|
//! | 0 | no block is AI provenance and no visible mark was seen — camera EXIF, colour, a comment are not findings | the input carried neither; the result was written (or, in place, nothing needed writing) |
//! | 1 | a block is AI provenance, or a visible mark was seen (proved or not) | the input carried AI provenance or a proved mark, and the result written carries neither |
//! | 2 | TIFF, HEIC, AVIF ("not in this version yet"); a flag for text | as `inspect`, and a JPEG whose MPF index a removal would leave wrong, an animation that would have to be written back, image bytes for a terminal, `--json` with the image on stdout, and every refusal `clean` has for a text |
//! | 3 | a file this version could not read (`Malformed`), or pixels that were not examined — a catalogue that did not load, pixels that do not decode, an animation's frames — not read is not clean, and 3 beats 1 | as `inspect`; **a result that would still carry AI provenance metadata, which is then not written**; **a visible mark left in the result — not proved, under opaque pixels, or restored with its outline or a texture left (D244, D247, D250) — which is written with what could be done**; a restored picture that could not be written back or failed its own check, with nothing written |
//!
//! `clean` exits by the **input**, as it does for a text (D28): a hook
//! wants to know what was there. The one thing it must never do is exit
//! 0, or 1, over an output that still carries provenance — `still_has_*`
//! is read off a second inspection of the output, and when either is true
//! the result is not written at all and the exit is 3 ([`clean_exit`]).
//!
//! # Where each output goes
//!
//! As `run.rs`'s table, with two differences. The product on standard
//! output is the image's **bytes**, and those are refused when standard
//! output is a terminal; and `--json` with the image on standard output
//! is refused, because standard output carries exactly one product.
//!
//! | command | `--json` | result goes to | stdout | stderr |
//! |---|---|---|---|---|
//! | `inspect` | no | — | the report | note, refusal |
//! | `inspect` | yes | — | `ImageReport::to_json()` | note, refusal |
//! | `clean` | no | a file | the report | note, refusal |
//! | `clean` | no | stdout (not a terminal) | the image | the report, note, refusal |
//! | `clean` | yes | a file | `{"report":…,"written":…}` | note, refusal |
//! | `clean --in-place` | yes | the file itself | `{"report":…,"written":{"path","original"}\|null}` | note, refusal |

use std::io::Write as _;

use wipemark_i18n::{args, FluentArgs, Message};
use wipemark_image::{
    Defect, ImageContainer, ImageError, ImageReport, MetadataFinding, MetadataKind, Scope, Signal,
    StripReport, Unsupported,
};
use wipemark_intake::inplace;
use wipemark_log::Elided;
use wipemark_picture::{
    Encoding, NotExamined, PictureError, PictureInspection, PictureOptions, PictureReport, Visible,
};
use wipemark_pixels::{Finding, Placed, Refusal, Verdict};

use crate::input::{Picture, Source};
use crate::report::{self, Say, Written};
use crate::run::{self, Destination, Io};
use crate::Exit;

/// `inspect` on a picture: its metadata and the visible marks in its
/// pixels. Never writes a file.
pub(crate) fn inspect(path: &str, label: &str, picture: &Picture, json: bool, io: &mut Io) -> Exit {
    let report = match wipemark_picture::inspect(&picture.bytes, &PictureOptions::default()) {
        Ok(report) => report,
        Err(error) => return refuse(io, "inspect", path, label, &error),
    };
    let exit = picture_inspect_exit(&report);
    let out = if json {
        format!("{}\n", report.to_json())
    } else {
        run::joined(inspect_lines(&run::say, label, &report))
    };
    if let Err(error) = run::emit(&mut io.stdout, out.as_bytes()) {
        return run::failed("inspect", path, "stdout", Some(error.kind()), Exit::Partial);
    }
    tracing::info!(
        command = "inspect",
        input = %Elided::from(path),
        container = report.metadata.container.id(),
        bytes = picture.bytes.len(),
        blocks = report.metadata.findings.len(),
        ai = report.metadata.has_ai_metadata(),
        c2pa = report.metadata.has_c2pa(),
        visible = report.has_visible_mark(),
        examined = !report.inconclusive(),
        exit = exit as u8,
        "done"
    );
    exit
}

/// `inspect`'s exit: 1 when any block is AI provenance, 0 otherwise.
/// Camera EXIF is not a finding: the product's subject is provenance
/// (D131).
pub(crate) fn inspect_exit(report: &ImageReport) -> Exit {
    if report.has_ai_metadata() {
        Exit::Findings
    } else {
        Exit::Clean
    }
}

/// `inspect`'s exit on a picture: 3 when the pixels should have been
/// examined and could not be — not read is not clean, and 3 beats 1; 1
/// when a visible mark was seen, proved or not, or a block is AI
/// provenance; 0 otherwise.
pub(crate) fn picture_inspect_exit(report: &PictureInspection) -> Exit {
    if report.inconclusive() {
        Exit::Partial
    } else if report.has_visible_mark() {
        Exit::Findings
    } else {
        inspect_exit(&report.metadata)
    }
}

/// The metadata half of `clean`'s exit, from the report of the strip
/// (D132). Never 0 — and never 1 — over an output that still carries
/// provenance: that is 3, and the caller writes nothing.
pub(crate) fn strip_exit(report: &StripReport) -> Exit {
    if report.still_has_ai_metadata || report.still_has_c2pa {
        Exit::Partial
    } else if report.removed.iter().any(MetadataFinding::is_ai_provenance) {
        Exit::Findings
    } else {
        Exit::Clean
    }
}

/// `clean`'s exit on a picture. The metadata's first: an output that
/// still carries provenance is 3 and is not written. Then the pixels: a
/// visible mark left in the result, or pixels that should have been
/// examined and were not, is 3 — the result is written with what could be
/// done, and *inconclusive is not clean*. Otherwise by the input (D28): 1
/// when it carried provenance or a mark that is now gone, 0 when not.
pub(crate) fn clean_exit(report: &PictureReport) -> Exit {
    let metadata = strip_exit(&report.metadata);
    if metadata == Exit::Partial || report.marks_left() || report.inconclusive() {
        return Exit::Partial;
    }
    let restored =
        matches!(&report.visible, Visible::Examined { report, .. } if !report.restored.is_empty());
    if restored {
        Exit::Findings
    } else {
        metadata
    }
}

/// What `clean` was asked, beyond the picture itself.
pub(crate) struct Ask<'a> {
    pub path: &'a str,
    pub label: &'a str,
    pub source: &'a Source,
    pub destination: &'a Destination,
    pub scope: Scope,
    pub json: bool,
    pub stdout_is_terminal: bool,
}

/// `clean` on a picture.
pub(crate) fn clean(ask: &Ask, picture: &Picture, io: &mut Io) -> Exit {
    let Ask {
        path,
        label,
        source,
        destination,
        scope,
        json,
        stdout_is_terminal,
    } = *ask;

    // Refused before anything is stripped: these are about where the
    // product goes, and none of them depends on what it would be.
    if *destination == Destination::Stdout {
        if json {
            let line = run::say(Message::CliImageJsonStdout, &FluentArgs::new());
            return run::refused(
                io,
                "clean",
                path,
                &line,
                "json with image on stdout",
                Exit::Usage,
            );
        }
        if stdout_is_terminal {
            let line = run::say(Message::CliImageToTerminal, &FluentArgs::new());
            return run::refused(io, "clean", path, &line, "image to a terminal", Exit::Usage);
        }
    }

    let options = PictureOptions {
        scope,
        catalogue: None,
    };
    let (bytes, report) = match wipemark_picture::clean(&picture.bytes, &options) {
        Ok(cleaned) => cleaned,
        Err(error) => return refuse_picture(io, "clean", path, label, &error),
    };
    let exit = clean_exit(&report);
    if strip_exit(&report.metadata) == Exit::Partial {
        // The output still carries what this command exists to remove. A
        // file named `.cleaned` that is not is worse than no file.
        let line = run::say(Message::CliImageStillMarked, &args!("path" => label));
        return run::refused(io, "clean", path, &line, "still marked", Exit::Partial);
    }

    let mut replaced = None;
    if let Destination::InPlace(file, keep) = destination {
        // Nothing removed is nothing touched: the output *is* the input.
        if bytes != picture.bytes {
            match inplace::replace(file, &bytes, *keep) {
                Ok(done) => replaced = Some(done),
                Err(failure) => return run::refuse_replacement(io, "clean", path, file, &failure),
            }
        }
    }
    if let Some(file) = destination.file() {
        let model = match source {
            Source::File(input) => Some(input.as_path()),
            Source::Stdin => None,
        };
        if let Err(error) = inplace::write_atomically(file, &bytes, model) {
            let line = run::say(
                Message::CliWriteFailed,
                &args!(
                    "path" => file.display().to_string(),
                    "reason" => error.to_string(),
                ),
            );
            let _ = writeln!(io.stderr, "wipemark-cli: {line}");
            return run::failed(
                "clean",
                path,
                "write failed",
                Some(error.kind()),
                Exit::Partial,
            );
        }
    }

    let from_file = matches!(source, Source::File(_));
    let file_shown = destination.file().map(|file| file.display().to_string());
    let in_place_shown = match (destination, &replaced) {
        (Destination::InPlace(file, _), Some(done)) => Some((
            file.display().to_string(),
            done.original
                .as_ref()
                .map(|original| original.display().to_string()),
        )),
        _ => None,
    };
    let written = match (destination, &in_place_shown, &file_shown) {
        (Destination::InPlace(..), Some((path, original)), _) => Written::Replaced {
            path,
            original: original.as_deref(),
        },
        (Destination::InPlace(..), None, _) => Written::Unchanged,
        (_, _, Some(path)) => Written::File { path, from_file },
        (_, _, None) => Written::Stdout { from_file },
    };
    let human = || run::joined(clean_lines(&run::say, label, &report, scope, written));

    let stdout: Vec<u8> = match (destination, json) {
        (Destination::Stdout, _) => {
            // The image is the product; the report goes beside it.
            let _ = run::emit(&mut io.stderr, human().as_bytes());
            bytes
        }
        (_, false) => human().into_bytes(),
        (Destination::InPlace(file, _), true) => {
            let written = match &replaced {
                Some(done) => serde_json::json!({
                    "path": file.to_string_lossy(),
                    "original": done.original.as_ref().map(|original| original.to_string_lossy()),
                }),
                None => serde_json::Value::Null,
            };
            format!(
                "{{\"report\":{},\"written\":{}}}\n",
                report.to_json(),
                crate::audit::ascii(&written)
            )
            .into_bytes()
        }
        (_, true) => {
            let written = destination
                .file()
                .map(|file| file.to_string_lossy().into_owned())
                .unwrap_or_default();
            format!(
                "{{\"report\":{},\"written\":{}}}\n",
                report.to_json(),
                crate::audit::ascii(&serde_json::Value::from(written))
            )
            .into_bytes()
        }
    };
    if let Err(error) = run::emit(&mut io.stdout, &stdout) {
        return run::failed("clean", path, "stdout", Some(error.kind()), Exit::Partial);
    }

    tracing::info!(
        command = "clean",
        input = %Elided::from(path),
        container = report.container.id(),
        bytes = picture.bytes.len(),
        removed = report.metadata.removed.len(),
        kept = report.metadata.kept.len(),
        encoding = report.encoding.id(),
        marks_left = report.marks_left(),
        examined = report.examined(),
        all_metadata = scope == Scope::AllMetadata,
        to = destination.label(),
        exit = exit as u8,
        "done"
    );
    exit
}

/// Why a picture was not read or not cleaned, on stderr, and its exit
/// code (D136): 2 for what this version will not do, 3 for a file it
/// could not read.
pub(crate) fn refuse(
    io: &mut Io,
    command: &str,
    path: &str,
    label: &str,
    error: &ImageError,
) -> Exit {
    let line = error_line(&run::say, label, error);
    let (reason, exit) = match error {
        ImageError::NotYet(_) => ("image not yet", Exit::Usage),
        ImageError::UnknownContainer => ("image unknown", Exit::Usage),
        ImageError::Unsupported { .. } => ("image unsupported", Exit::Usage),
        ImageError::Malformed { defect, .. } => {
            tracing::info!(defect = defect.id(), "malformed image");
            ("image malformed", Exit::Partial)
        }
    };
    let _ = writeln!(io.stderr, "wipemark-cli: {line}");
    run::failed(command, path, reason, None, exit)
}

/// Why a picture was not inspected or not cleaned, past its metadata: the
/// container's refusals are [`refuse`]'s; a result that could not be
/// written back, or failed its own check, is 3 — nothing was written, and
/// what was not done is not clean.
pub(crate) fn refuse_picture(
    io: &mut Io,
    command: &str,
    path: &str,
    label: &str,
    error: &PictureError,
) -> Exit {
    let (line, reason) = match error {
        PictureError::Image(error) => return refuse(io, command, path, label, error),
        PictureError::Proof(proof) => {
            tracing::warn!(proof = ?proof, "picture proof failed");
            (
                run::say(Message::CliImageProofFailed, &args!("path" => label)),
                "proof failed",
            )
        }
        PictureError::Encode { .. } => (
            run::say(Message::CliImageEncodeFailed, &args!("path" => label)),
            "encode failed",
        ),
        PictureError::Decode { .. } => (
            run::say(
                Message::CliImageVisibleNotExaminedDecode,
                &FluentArgs::new(),
            ),
            "decode failed",
        ),
    };
    let _ = writeln!(io.stderr, "wipemark-cli: {line}");
    run::failed(command, path, reason, None, Exit::Partial)
}

/// The sentence for an [`ImageError`] — the line `inspect` and `clean`
/// refuse with and the line `audit` lists an unreadable image under.
pub(crate) fn error_line(say: Say, label: &str, error: &ImageError) -> String {
    match error {
        ImageError::NotYet(container) => say(
            Message::CliImageNotYet,
            &args!("path" => label, "container" => container.name()),
        ),
        ImageError::UnknownContainer => say(Message::CliImageUnknown, &args!("path" => label)),
        ImageError::Unsupported {
            what: Unsupported::MultiPicture,
            ..
        } => say(Message::CliImageMultiPicture, &args!("path" => label)),
        ImageError::Unsupported {
            what: Unsupported::Reframe,
            ..
        } => say(Message::CliImageReframe, &args!("path" => label)),
        ImageError::Malformed {
            container,
            offset,
            defect,
        } => say(
            Message::CliImageMalformed,
            &args!(
                "path" => label,
                "container" => container.name(),
                "defect" => say(defect_label(*defect), &FluentArgs::new()),
                "offset" => offset.to_string(),
            ),
        ),
    }
}

/// A flag that does not fit what the file turned out to be: exit 2 and
/// nothing written (D134). `flag` is the flag as typed.
pub(crate) fn refuse_flag(
    io: &mut Io,
    path: &str,
    label: &str,
    container: Option<&str>,
    flag: &str,
) -> Exit {
    let line = match container {
        Some(container) => run::say(
            Message::CliImageTextFlag,
            &args!("path" => label, "container" => container, "flag" => flag),
        ),
        None => run::say(Message::CliImageAllMetadataText, &args!("path" => label)),
    };
    run::refused(io, "clean", path, &line, "flag does not fit", Exit::Usage)
}

/// `inspect`'s report on a picture: the summary, every block under what
/// `clean` would remove and what it would keep, the colour note, the
/// pixels, the third shelf.
pub(crate) fn inspect_lines(say: Say, source: &str, inspection: &PictureInspection) -> Vec<String> {
    let report: &ImageReport = &inspection.metadata;
    let mut lines = vec![summary(say, source, report.container, &report.findings)];
    let (removed, kept): (Vec<&MetadataFinding>, Vec<&MetadataFinding>) = report
        .findings
        .iter()
        .partition(|finding| finding.is_ai_provenance() && finding.kind != MetadataKind::Rendering);
    for (heading, rows) in [
        (Message::CliReportWouldRemove, removed),
        (Message::CliReportWouldKeep, kept),
    ] {
        if rows.is_empty() {
            continue;
        }
        lines.push(say(heading, &FluentArgs::new()));
        for finding in rows {
            lines.extend(block(say, finding));
        }
    }
    if report
        .findings
        .iter()
        .any(|finding| finding.kind == MetadataKind::Rendering)
    {
        lines.push(say(Message::CliImageRendering, &FluentArgs::new()));
    }
    lines.extend(visible_lines(say, &inspection.visible, false));
    lines.extend(footer(say));
    lines
}

/// `clean`'s report on a picture: the summary of the input, what went and
/// what stayed, the notes, where the result went, the pixels, the shelf.
pub(crate) fn clean_lines(
    say: Say,
    source: &str,
    picture: &PictureReport,
    scope: Scope,
    written: Written,
) -> Vec<String> {
    let report: &StripReport = &picture.metadata;
    let input: Vec<MetadataFinding> = report.removed.iter().chain(&report.kept).cloned().collect();
    let mut lines = vec![summary(say, source, report.container, &input)];
    for (heading, rows) in [
        (Message::CliReportRemoved, &report.removed),
        (Message::CliReportKept, &report.kept),
    ] {
        if rows.is_empty() {
            continue;
        }
        lines.push(say(heading, &FluentArgs::new()));
        for finding in rows {
            lines.extend(block(say, finding));
        }
    }
    if report
        .kept
        .iter()
        .any(|finding| finding.kind == MetadataKind::Rendering)
    {
        lines.push(say(Message::CliImageRendering, &FluentArgs::new()));
    }
    // EXIF leaves whole: under `--all-metadata` always, and under the
    // default scope when the block named a generator.
    if report
        .removed
        .iter()
        .any(|finding| finding.kind == MetadataKind::Exif)
    {
        lines.push(say(
            if scope == Scope::AllMetadata {
                Message::CliImageAllMetadata
            } else {
                Message::CliImageExifRemoved
            },
            &FluentArgs::new(),
        ));
    }
    // And when one of them carried a rotation, that is a fact the library
    // read, not a warning about what EXIF may hold.
    if report.orientation_removed.is_some() {
        lines.push(say(Message::CliImageOrientationRemoved, &FluentArgs::new()));
    }
    lines.extend(visible_lines(say, &picture.visible, true));
    lines.extend(encoding_lines(say, picture.encoding));
    if picture.marks_left() {
        lines.push(say(Message::CliImageVisibleLeft, &FluentArgs::new()));
    }
    lines.extend(report::written_lines(say, source, written));
    lines.extend(footer(say));
    lines
}

/// The first line: nothing, blocks none of which is provenance, or how
/// many are. Never "clean".
fn summary(
    say: Say,
    source: &str,
    container: ImageContainer,
    findings: &[MetadataFinding],
) -> String {
    let count = findings.len();
    let ai = findings
        .iter()
        .filter(|finding| finding.is_ai_provenance())
        .count();
    let container = container.name();
    if count == 0 {
        say(
            Message::CliImageNone,
            &args!("source" => source, "container" => container),
        )
    } else if ai == 0 {
        say(
            Message::CliImageNoted,
            &args!("source" => source, "container" => container, "count" => count),
        )
    } else {
        say(
            Message::CliImageAi,
            &args!(
                "source" => source,
                "container" => container,
                "count" => count,
                "ai" => ai,
            ),
        )
    }
}

/// Where a block is, as the file names it: the chunk or segment, and the
/// key inside it — spelled, because a PNG keyword is the file's Latin-1.
pub(crate) fn where_of(finding: &MetadataFinding) -> String {
    let chunk = wipemark_image::spell(&finding.chunk);
    match &finding.key {
        Some(key) => format!("{chunk} {}", wipemark_image::spell(key)),
        None => chunk,
    }
}

/// One block and, under it, every signal that made it provenance.
fn block(say: Say, finding: &MetadataFinding) -> Vec<String> {
    let mut lines = vec![format!(
        "  {}",
        say(
            Message::CliImageRow,
            &args!(
                "where" => where_of(finding),
                "kind" => say(kind_label(finding.kind), &FluentArgs::new()),
                // Spelled as text: a number is grouped by the locale, and
                // "1,234" is not an offset anybody can use.
                "offset" => finding.offset.to_string(),
                "size" => finding.len.to_string(),
                "length" => finding.len,
            ),
        )
    )];
    for evidence in &finding.evidence {
        let signal = say(signal_label(evidence.signal), &FluentArgs::new());
        let field = wipemark_image::spell(&evidence.field);
        let matched = wipemark_image::spell(evidence.matched);
        let line = match evidence.signal {
            Signal::GeneratorKey(generator) | Signal::GeneratorText(generator) => say(
                Message::CliImageEvidenceGenerator,
                &args!(
                    "signal" => signal,
                    "generator" => generator.id(),
                    "field" => field,
                    "matched" => matched,
                ),
            ),
            _ => say(
                Message::CliImageEvidence,
                &args!("signal" => signal, "field" => field, "matched" => matched),
            ),
        };
        lines.push(format!("    {line}"));
    }
    lines
}

/// What the pixels were examined for, then the picture's third shelf —
/// invisible marks first. No Unicode version: nothing here was read as
/// characters.
fn footer(say: Say) -> Vec<String> {
    let mut lines = vec![say(Message::CliImagePixels, &FluentArgs::new())];
    lines.extend(report::picture_shelf(say));
    lines
}

/// A number as the report spells it: a fixed number of decimals, the
/// language's decimal separator, never grouped.
fn fixed(value: f32, decimals: usize) -> String {
    wipemark_i18n::decimal(f64::from(value), decimals)
}

/// How far the restored mark's faint band lies from the picture around
/// it: the mean step of the channel farthest from it (D247, D248), a mean and
/// not a bound.
fn farthest(restored: &wipemark_pixels::Restored) -> f32 {
    restored.steps.iter().fold(0f32, |m, s| m.max(s.abs()))
}

/// The visible pass, as it went: every finding, proved or not, with its
/// numbers; after a `clean`, what was restored.
fn visible_lines(say: Say, visible: &Visible, cleaned: bool) -> Vec<String> {
    let mut lines = vec![say(Message::CliImageVisibleTitle, &FluentArgs::new())];
    let (report, restorable) = match visible {
        Visible::NotExamined(why) => {
            let message = match why {
                NotExamined::Animated => Message::CliImageVisibleNotExaminedAnimated,
                NotExamined::Catalogue => Message::CliImageVisibleNotExaminedCatalogue,
                NotExamined::Decode => Message::CliImageVisibleNotExaminedDecode,
            };
            lines.push(format!("  {}", say(message, &FluentArgs::new())));
            return lines;
        }
        Visible::Examined { report, restorable } => (report, *restorable),
    };
    if report.found.is_empty() {
        lines.push(format!(
            "  {}",
            say(Message::CliImageVisibleNone, &FluentArgs::new())
        ));
        return lines;
    }
    for finding in &report.found {
        lines.extend(finding_lines(say, finding));
    }
    if !restorable {
        lines.push(format!(
            "  {}",
            say(Message::CliImageVisibleNotRestorable, &FluentArgs::new())
        ));
    }
    if cleaned {
        for restored in &report.restored {
            lines.push(format!(
                "  {}",
                say(
                    Message::CliImageVisibleRestored,
                    &args!(
                        "profile" => restored.profile.as_str(),
                        "changed" => restored.changed.to_string(),
                    ),
                )
            ));
            // Exact, or why not — each reason that holds, by name: a lossless
            // PNG that only clamped was not "stored with loss" (D245).
            let mut notes = Vec::new();
            if restored.exact {
                notes.push(say(Message::CliImageVisibleExact, &FluentArgs::new()));
            } else {
                if restored.lossy {
                    notes.push(say(Message::CliImageVisibleInexact, &FluentArgs::new()));
                }
                if restored.clamped > 0 {
                    notes.push(say(
                        Message::CliImageVisibleClamped,
                        &args!("clamped" => restored.clamped),
                    ));
                }
                if restored.fitted {
                    notes.push(say(Message::CliImageVisibleFitted, &FluentArgs::new()));
                }
                // Each only when it happened (D249): a map drawn as captured
                // by the search was not resampled.
                if restored.resampled {
                    notes.push(say(Message::CliImageVisibleResampled, &FluentArgs::new()));
                }
                if restored.searched {
                    notes.push(say(Message::CliImageVisibleSearched, &FluentArgs::new()));
                }
                // Not exact, so what it is: the mark's faint band against
                // the picture around it, on average (D247, D248). An
                // outline left says its own number below.
                if !restored.outline_left {
                    notes.push(say(
                        Message::CliImageVisibleResidual,
                        &args!("levels" => fixed(farthest(restored), 1)),
                    ));
                }
            }
            for note in notes {
                lines.push(format!("    {note}"));
            }
            if restored.holes > 0 {
                lines.push(format!(
                    "    {}",
                    say(
                        Message::CliImageVisibleHoles,
                        &args!("holes" => restored.holes.to_string()),
                    )
                ));
            }
            if restored.outline_left {
                lines.push(format!(
                    "    {}",
                    say(
                        Message::CliImageVisibleOutline,
                        &args!(
                            "levels" => fixed(farthest(restored), 1),
                            "share" => fixed(restored.outline * 100.0, 0),
                        ),
                    )
                ));
            }
            // A lossy source's error, amplified by the inverse: a
            // percentile, beside the same around the mark (D250).
            if restored.texture_left {
                lines.push(format!(
                    "    {}",
                    say(
                        Message::CliImageVisibleTexture,
                        &args!(
                            "levels" => fixed(restored.texture, 1),
                            "around" => fixed(restored.texture_around, 1),
                        ),
                    )
                ));
            }
        }
    }
    lines
}

/// One finding: where, by which profile — whose names are identifiers,
/// interpolated and never translated — and the verdict with its numbers.
fn finding_lines(say: Say, finding: &Finding) -> Vec<String> {
    let rect = finding.pixels.unwrap_or(wipemark_pixels::PixelRect {
        x: finding.rect.x.max(0.0) as u32,
        y: finding.rect.y.max(0.0) as u32,
        width: finding.rect.size as u32,
        height: finding.rect.size as u32,
    });
    let placed = match finding.placed {
        Placed::Row(_) => Message::CliImageVisiblePlacedRow,
        Placed::Searched => Message::CliImageVisiblePlacedSearched,
    };
    let row = say(
        Message::CliImageVisibleRow,
        &args!(
            "profile" => finding.profile.as_str(),
            "vendor" => finding.vendor.as_str(),
            "product" => finding.product.as_str(),
            "width" => rect.width.to_string(),
            "height" => rect.height.to_string(),
            "x" => rect.x.to_string(),
            "y" => rect.y.to_string(),
            "placed" => say(placed, &FluentArgs::new()),
        ),
    );
    let verdict = match &finding.verdict {
        Verdict::Verified(v) => say(
            Message::CliImageVisibleProved,
            &args!(
                "ncc" => fixed(finding.ncc, 3),
                "gain" => fixed(v.gain(), 2),
                "ratio" => fixed(v.edge_ratio(), 3),
            ),
        ),
        Verdict::Refused(refusal) => say(
            Message::CliImageVisibleRefused,
            &args!("reason" => refusal_line(say, *refusal)),
        ),
    };
    vec![format!("  {row}"), format!("    {verdict}")]
}

/// A refusal's reason, with the number that failed.
fn refusal_line(say: Say, refusal: Refusal) -> String {
    match refusal {
        Refusal::Transparent => say(Message::CliImageRefusalTransparent, &FluentArgs::new()),
        Refusal::Opaque { holes } => say(
            Message::CliImageRefusalOpaque,
            &args!("holes" => holes.to_string()),
        ),
        Refusal::Gain { k } => say(Message::CliImageRefusalGain, &args!("k" => fixed(k, 2))),
        Refusal::Edges { ratio } => say(
            Message::CliImageRefusalEdges,
            &args!("ratio" => format!("{}%", fixed(ratio * 100.0, 0))),
        ),
        Refusal::OutOfRange { share } => say(
            Message::CliImageRefusalOutOfRange,
            &args!("share" => format!("{}%", fixed(share * 100.0, 1))),
        ),
    }
}

/// How the picture was written back, when its pixels were.
fn encoding_lines(say: Say, encoding: Encoding) -> Vec<String> {
    let none = FluentArgs::new;
    match encoding {
        Encoding::Unchanged => Vec::new(),
        Encoding::Jpeg { quality } => vec![say(
            Message::CliImageEncodedJpeg,
            &args!("quality" => quality.to_string()),
        )],
        Encoding::WebPLossless { from_lossy } => vec![say(
            if from_lossy {
                Message::CliImageEncodedWebpFromLossy
            } else {
                Message::CliImageEncodedWebp
            },
            &none(),
        )],
        Encoding::Png {
            colour_changed,
            interlace_dropped,
        } => {
            let mut lines = vec![say(Message::CliImageEncodedPng, &none())];
            if colour_changed {
                lines.push(say(Message::CliImageEncodedPngColour, &none()));
            }
            if interlace_dropped {
                lines.push(say(Message::CliImageEncodedPngInterlace, &none()));
            }
            lines
        }
    }
}

/// A kind's words. Exhaustive, so a ninth kind does not compile here
/// until it has a key.
pub(crate) fn kind_label(kind: MetadataKind) -> Message {
    match kind {
        MetadataKind::C2pa => Message::ImageKindC2pa,
        MetadataKind::Exif => Message::ImageKindExif,
        MetadataKind::Xmp => Message::ImageKindXmp,
        MetadataKind::Iptc => Message::ImageKindIptc,
        MetadataKind::GeneratorParameters => Message::ImageKindGeneratorParameters,
        MetadataKind::OtherText => Message::ImageKindOtherText,
        MetadataKind::Rendering => Message::ImageKindRendering,
        MetadataKind::Other => Message::ImageKindOther,
    }
}

/// A signal's words. Exhaustive, for the same reason.
pub(crate) fn signal_label(signal: Signal) -> Message {
    match signal {
        Signal::C2paManifest => Message::ImageSignalC2paManifest,
        Signal::C2paReference => Message::ImageSignalC2paReference,
        Signal::DigitalSourceType(_) => Message::ImageSignalDigitalSourceType,
        Signal::GeneratorKey(_) => Message::ImageSignalGeneratorKey,
        Signal::GeneratorText(_) => Message::ImageSignalGeneratorText,
    }
}

/// A defect's words. Exhaustive, for the same reason.
pub(crate) fn defect_label(defect: Defect) -> Message {
    match defect {
        Defect::Truncated => Message::ImageDefectTruncated,
        Defect::BadSignature => Message::ImageDefectBadSignature,
        Defect::HeaderNotFirst => Message::ImageDefectHeaderNotFirst,
        Defect::NoEnd => Message::ImageDefectNoEnd,
        Defect::BadLength => Message::ImageDefectBadLength,
        Defect::BadChunkType => Message::ImageDefectBadChunkType,
        Defect::BadMarker(_) => Message::ImageDefectBadMarker,
        Defect::RiffSize { .. } => Message::ImageDefectRiffSize,
        Defect::BadText => Message::ImageDefectBadText,
        Defect::Inflate => Message::ImageDefectInflate,
        Defect::InflateLimit => Message::ImageDefectInflateLimit,
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use wipemark_i18n::{FluentArgs, Localizer, Message, Rendering};
    use wipemark_image::{
        Evidence, ImageContainer, MetadataFinding, MetadataKind, Scope, Signal, StripReport,
    };
    use wipemark_picture::{Encoding, NotExamined, PictureInspection, PictureReport, Visible};
    use wipemark_pixels::PixelReport;

    use super::{
        clean_exit, clean_lines, defect_label, inspect_lines, kind_label, picture_inspect_exit,
        signal_label, strip_exit, visible_lines, Ask,
    };
    use crate::input::{Picture, Source};
    use crate::report::Written;
    use crate::run::{Destination, Io};
    use crate::Exit;

    fn localizers() -> Vec<Localizer> {
        wipemark_i18n::available_languages()
            .iter()
            .map(|language| {
                Localizer::for_languages(std::slice::from_ref(&language.id), Rendering::PlainText)
            })
            .collect()
    }

    fn c2pa() -> MetadataFinding {
        MetadataFinding {
            kind: MetadataKind::C2pa,
            chunk: "caBX".into(),
            key: None,
            offset: 33,
            len: 120,
            evidence: vec![Evidence {
                signal: Signal::C2paManifest,
                field: "C2PA".into(),
                matched: "caBX",
            }],
        }
    }

    fn exif() -> MetadataFinding {
        MetadataFinding {
            kind: MetadataKind::Exif,
            chunk: "eXIf".into(),
            key: None,
            offset: 200,
            len: 40,
            evidence: Vec::new(),
        }
    }

    /// A visible pass that found nothing.
    fn nothing_seen() -> Visible {
        Visible::Examined {
            report: PixelReport {
                found: Vec::new(),
                restored: Vec::new(),
                dismissed: 0,
                not_established: wipemark_pixels::not_established::shelf(),
            },
            restorable: true,
        }
    }

    /// A strip's report as a picture's, nothing seen in the pixels.
    fn picture(metadata: StripReport) -> PictureReport {
        PictureReport {
            container: ImageContainer::Png,
            metadata,
            visible: nothing_seen(),
            encoding: Encoding::Unchanged,
        }
    }

    fn strip(removed: Vec<MetadataFinding>, kept: Vec<MetadataFinding>) -> StripReport {
        let still = kept.iter().any(MetadataFinding::is_ai_provenance);
        StripReport {
            container: ImageContainer::Png,
            removed,
            kept,
            still_has_c2pa: still,
            still_has_ai_metadata: still,
            orientation_removed: None,
            not_established: Vec::new(),
        }
    }

    /// D132: by the input, as a text's clean is — and never 0, nor 1,
    /// over an output that still carries provenance. Each `still_has_*`
    /// alone is enough.
    #[test]
    fn an_output_that_still_carries_provenance_never_exits_zero() {
        assert_eq!(
            strip_exit(&strip(vec![c2pa()], vec![exif()])),
            Exit::Findings
        );
        assert_eq!(strip_exit(&strip(vec![], vec![exif()])), Exit::Clean);
        assert_eq!(strip_exit(&strip(vec![exif()], vec![])), Exit::Clean);
        let mut still = strip(vec![c2pa()], vec![c2pa()]);
        assert_eq!(strip_exit(&still), Exit::Partial);
        still.still_has_ai_metadata = false;
        assert_eq!(strip_exit(&still), Exit::Partial, "still_has_c2pa alone");
        still.still_has_ai_metadata = true;
        still.still_has_c2pa = false;
        assert_eq!(
            strip_exit(&still),
            Exit::Partial,
            "still_has_ai_metadata alone"
        );
        still.removed.clear();
        assert_eq!(
            strip_exit(&still),
            Exit::Partial,
            "nothing removed, still marked"
        );
    }

    /// Image bytes never go to a terminal, and `--json` never shares
    /// standard output with them: both refused with 2, standard output
    /// left empty. In process, because the binary's tests have no pty.
    #[test]
    fn image_bytes_are_never_written_to_a_terminal() {
        let picture = Picture {
            bytes: b"\x89PNG\r\n\x1a\n".to_vec(),
            format: wipemark_intake::Format::Png,
            note: None,
        };
        for (json, terminal, refused) in [(false, true, true), (true, false, true)] {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let exit = {
                let mut io = Io {
                    stdin: Box::new(Cursor::new(Vec::new())),
                    stdout: Box::new(&mut stdout),
                    stderr: Box::new(&mut stderr),
                };
                super::clean(
                    &Ask {
                        path: "-",
                        label: "standard input",
                        source: &Source::Stdin,
                        destination: &Destination::Stdout,
                        scope: Scope::AiProvenance,
                        json,
                        stdout_is_terminal: terminal,
                    },
                    &picture,
                    &mut io,
                )
            };
            assert_eq!(exit, Exit::Usage, "json {json}, terminal {terminal}");
            assert!(stdout.is_empty(), "something reached stdout");
            assert_eq!(!stderr.is_empty(), refused);
        }
    }

    /// Every label is the key its id names, so the i18n gate — which
    /// checks keys by id — is checking what this file prints.
    #[test]
    fn every_label_is_its_own_key() {
        for kind in MetadataKind::ALL {
            assert_eq!(kind_label(kind).id(), format!("image-kind-{}", kind.id()));
        }
        for signal in [
            Signal::C2paManifest,
            Signal::C2paReference,
            Signal::DigitalSourceType(wipemark_image::SourceType::AlgorithmicMedia),
            Signal::GeneratorKey(wipemark_image::Generator::ComfyUi),
            Signal::GeneratorText(wipemark_image::Generator::ComfyUi),
        ] {
            assert_eq!(
                signal_label(signal).id(),
                format!("image-signal-{}", signal.id())
            );
        }
        for defect in [
            wipemark_image::Defect::Truncated,
            wipemark_image::Defect::BadSignature,
            wipemark_image::Defect::HeaderNotFirst,
            wipemark_image::Defect::NoEnd,
            wipemark_image::Defect::BadLength,
            wipemark_image::Defect::BadChunkType,
            wipemark_image::Defect::BadMarker(0),
            wipemark_image::Defect::RiffSize {
                declared: 0,
                available: 0,
            },
            wipemark_image::Defect::BadText,
            wipemark_image::Defect::Inflate,
            wipemark_image::Defect::InflateLimit,
        ] {
            assert_eq!(
                defect_label(defect).id(),
                format!("image-defect-{}", defect.id())
            );
        }
    }

    /// The third shelf, in every language, under every image report — and
    /// the line that says the pixels were not looked at, above it.
    #[test]
    fn every_image_report_says_the_pixels_were_not_examined() {
        let report = wipemark_image::ImageReport {
            container: ImageContainer::Png,
            findings: vec![c2pa(), exif()],
            not_established: Vec::new(),
        };
        for localizer in localizers() {
            let say = |message: Message, args: &FluentArgs| localizer.format_args(message, args);
            let pixels = localizer.format(Message::CliImagePixels);
            let shelf = localizer.format(Message::ReportNotEstablishedUnknownMarkSchemes);
            let inspection = PictureInspection {
                metadata: report.clone(),
                visible: nothing_seen(),
            };
            for lines in [
                inspect_lines(&say, "x.png", &inspection),
                clean_lines(
                    &say,
                    "x.png",
                    &picture(strip(vec![c2pa()], vec![exif()])),
                    Scope::AiProvenance,
                    Written::File {
                        path: "x.cleaned.png",
                        from_file: true,
                    },
                ),
            ] {
                assert!(
                    lines.contains(&pixels),
                    "{}: {lines:#?}",
                    localizer.language()
                );
                assert!(
                    lines.iter().any(|line| line.ends_with(&shelf)),
                    "{}: {lines:#?}",
                    localizer.language()
                );
                for line in &lines {
                    for character in line.chars() {
                        let code = u32::from(character);
                        assert!(
                            !matches!(code,
                                0x00AD | 0x00A0 | 0x2000..=0x200F | 0x202A..=0x202E
                                | 0x2060 | 0x2066..=0x2069 | 0xFEFF | 0x202F | 0x3000
                            ),
                            "{}: U+{code:04X} in {line:?}",
                            localizer.language()
                        );
                    }
                }
            }
        }
    }

    /// `--all-metadata` says what it took when it took EXIF, and only
    /// then; the default scope never says it.
    #[test]
    fn all_metadata_says_it_took_the_orientation() {
        let english = localizers()
            .into_iter()
            .find(|localizer| localizer.language() == "en-US")
            .expect("the fallback");
        let say = |message: Message, args: &FluentArgs| english.format_args(message, args);
        let note = english.format(Message::CliImageAllMetadata);
        let written = Written::Stdout { from_file: false };
        let lines = clean_lines(
            &say,
            "x",
            &picture(strip(vec![exif()], vec![])),
            Scope::AllMetadata,
            written,
        );
        assert!(lines.contains(&note), "{lines:#?}");
        let lines = clean_lines(
            &say,
            "x",
            &picture(strip(vec![c2pa()], vec![exif()])),
            Scope::AiProvenance,
            written,
        );
        assert!(!lines.contains(&note), "{lines:#?}");

        // The default scope, an EXIF block that named a generator: gone
        // whole, and said so in its own words.
        let generated = english.format(Message::CliImageExifRemoved);
        let mut ai_exif = exif();
        ai_exif.evidence.push(Evidence {
            signal: Signal::GeneratorText(wipemark_image::Generator::StableDiffusionWebUi),
            field: "EXIF".into(),
            matched: "a1111-infotext",
        });
        let lines = clean_lines(
            &say,
            "x",
            &picture(strip(vec![ai_exif], vec![])),
            Scope::AiProvenance,
            written,
        );
        assert!(lines.contains(&generated), "{lines:#?}");
        assert!(!lines.contains(&note), "{lines:#?}");
        let lines = clean_lines(
            &say,
            "x",
            &picture(strip(vec![c2pa()], vec![exif()])),
            Scope::AiProvenance,
            written,
        );
        assert!(!lines.contains(&generated), "{lines:#?}");
    }

    /// The rotation is said when the library says a removed block carried
    /// one, in either scope, and never because an EXIF block went.
    #[test]
    fn the_rotation_is_said_only_when_it_was_removed() {
        let english = localizers()
            .into_iter()
            .find(|localizer| localizer.language() == "en-US")
            .expect("the fallback");
        let say = |message: Message, args: &FluentArgs| english.format_args(message, args);
        let rotation = english.format(Message::CliImageOrientationRemoved);
        let written = Written::Stdout { from_file: false };
        for scope in Scope::ALL {
            let mut report = picture(strip(vec![exif()], vec![]));
            let lines = clean_lines(&say, "x", &report, scope, written);
            assert!(!lines.contains(&rotation), "{scope:?}: {lines:#?}");
            report.metadata.orientation_removed = Some(6);
            let lines = clean_lines(&say, "x", &report, scope, written);
            assert!(lines.contains(&rotation), "{scope:?}: {lines:#?}");
        }
    }

    /// E12-5's gate: a visible mark seen and left in the result never
    /// exits 0 — nor 1 — whatever the metadata did; and pixels that should
    /// have been examined and were not are 3 too.
    #[test]
    fn a_visible_mark_left_behind_never_exits_0() {
        let seen = |restorable: bool, found: Vec<wipemark_pixels::Finding>| {
            let mut report = picture(strip(vec![c2pa()], vec![]));
            report.visible = Visible::Examined {
                report: PixelReport {
                    found,
                    restored: Vec::new(),
                    dismissed: 0,
                    not_established: wipemark_pixels::not_established::shelf(),
                },
                restorable,
            };
            report
        };
        let refused = wipemark_pixels::Finding {
            profile: "test-mark".into(),
            vendor: "test".into(),
            product: "synthetic".into(),
            rect: wipemark_pixels::SubRect {
                x: 10.0,
                y: 10.0,
                size: 48.0,
            },
            pixels: None,
            placed: wipemark_pixels::Placed::Searched,
            kernel: wipemark_pixels::Kernel::Area,
            ncc: 0.9,
            pass: 1,
            scores: None,
            verdict: wipemark_pixels::Verdict::Refused(wipemark_pixels::Refusal::Gain { k: 0.72 }),
            also_tried: Vec::new(),
        };
        // A mark seen and not proved is left: 3, though the metadata went.
        assert_eq!(
            clean_exit(&seen(true, vec![refused.clone()])),
            Exit::Partial
        );
        // A mark in a picture this version does not write back: left, 3.
        assert_eq!(clean_exit(&seen(false, vec![refused])), Exit::Partial);
        // Nothing seen: by the input, as before.
        assert_eq!(clean_exit(&seen(true, Vec::new())), Exit::Findings);
        assert_eq!(clean_exit(&seen(false, Vec::new())), Exit::Findings);
        assert_eq!(clean_exit(&picture(strip(vec![], vec![]))), Exit::Clean);
        // Not examined — for any reason, an animation's frames included
        // (D221 amended) — is 3, and 3 beats 1.
        for why in [
            NotExamined::Catalogue,
            NotExamined::Decode,
            NotExamined::Animated,
        ] {
            let mut report = picture(strip(vec![], vec![]));
            report.visible = Visible::NotExamined(why);
            assert_eq!(clean_exit(&report), Exit::Partial, "{why:?}");
            let inspection = PictureInspection {
                metadata: wipemark_image::ImageReport {
                    container: ImageContainer::Png,
                    findings: vec![c2pa()],
                    not_established: Vec::new(),
                },
                visible: Visible::NotExamined(why),
            };
            assert_eq!(
                picture_inspect_exit(&inspection),
                Exit::Partial,
                "3 beats 1: {why:?}"
            );
        }
    }

    /// A restoration that is not exact says each reason that holds and
    /// none that does not: a mark the search found with its map drawn as
    /// captured — at its own size, a whole-pixel offset — was searched,
    /// not resampled (D249); one at a row whose map was resampled says
    /// that and not the search. How far it is, is a mean — the farthest
    /// channel's (D247, D248) — never a bound.
    #[test]
    fn a_restoration_says_each_reason_it_is_not_exact_and_its_mean() {
        let restored = |searched: bool, resampled: bool| wipemark_pixels::Restored {
            profile: "gemini-sparkle-v2".into(),
            rect: wipemark_pixels::PixelRect {
                x: 10,
                y: 10,
                width: 48,
                height: 48,
            },
            changed: 640,
            holes: 0,
            clamped: 0,
            outline: 0.01,
            steps: [0.2, -2.26, 0.4],
            step: -0.6,
            chroma: 0.9,
            outline_left: false,
            texture: 1.8,
            texture_around: 1.6,
            texture_left: false,
            noise: false,
            lossy: false,
            fitted: false,
            resampled,
            searched,
            exact: false,
            planar: None,
        };
        let english = Localizer::for_languages(&["en-US".parse().unwrap()], Rendering::PlainText);
        let say = |message: Message, args: &FluentArgs| english.format_args(message, args);
        let lines = |r: wipemark_pixels::Restored| {
            // A finding for the pass to report under; its own lines are
            // not what is asserted here.
            let found = wipemark_pixels::Finding {
                profile: "gemini-sparkle-v2".into(),
                vendor: "test".into(),
                product: "synthetic".into(),
                rect: wipemark_pixels::SubRect {
                    x: 10.0,
                    y: 10.0,
                    size: 48.0,
                },
                pixels: None,
                placed: wipemark_pixels::Placed::Searched,
                kernel: wipemark_pixels::Kernel::Area,
                ncc: 0.9,
                pass: 1,
                scores: None,
                verdict: wipemark_pixels::Verdict::Refused(wipemark_pixels::Refusal::Gain {
                    k: 0.72,
                }),
                also_tried: Vec::new(),
            };
            let visible = Visible::Examined {
                report: PixelReport {
                    found: vec![found],
                    restored: vec![r],
                    dismissed: 0,
                    not_established: wipemark_pixels::not_established::shelf(),
                },
                restorable: true,
            };
            visible_lines(&say, &visible, true).join("\n")
        };
        let searched = lines(restored(true, false));
        assert!(searched.contains("found by the search"), "{searched}");
        assert!(!searched.contains("resampled"), "{searched}");
        assert!(searched.contains("on average 2"), "{searched}");
        assert!(searched.contains("farthest from it"), "{searched}");
        assert!(!searched.contains("within"), "{searched}");
        let resampled = lines(restored(false, true));
        assert!(resampled.contains("resampled"), "{resampled}");
        assert!(!resampled.contains("found by the search"), "{resampled}");
    }
}
