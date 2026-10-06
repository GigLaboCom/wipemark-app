//! Cleaning one thing that arrived, with no window: read it, run Layer A
//! or the picture passes over it, decide what that came to, write the
//! result where the Retention page says, keep what was asked to be kept,
//! and say all of it as one value.
//!
//! Everything here **blocks** — a read off a network volume, a decode of a
//! picture hundreds of megabytes wide, a synced write — and every caller
//! runs it on the background executor. Nothing here localizes either: an
//! [`Outcome`] is a value, and the window that shows it words it.
//!
//! # The policy is the CLI's, stated once
//!
//! `wipemark-cli clean` decides by the same libraries what a run came to,
//! and its tables (`docs/architecture/cli.md`) are the behaviour this
//! module matches. The CLI's policy lives inside a binary and cannot be
//! imported, so [`outcome_of`] states it again as one pure function, and
//! its tests walk the CLI's rows one by one:
//!
//! * a picture whose output **still carries AI provenance metadata** is
//!   never written — a file named `.cleaned` that is not is worse than
//!   none (the CLI's exit 3 with nothing written);
//! * a picture with a **visible mark left**, or pixels that were **not
//!   examined**, is written with what could be done and said to be only
//!   partly clean (the CLI's exit 3 with the result written);
//! * a result **identical to its input is never written**, whatever the
//!   verdict (D260, D262): no copy named `.cleaned` that is the input
//!   under another name.
//!
//! # Where a result goes
//!
//! By the [`Plan`] taken when the clean starts, and by no other road. A
//! file is written beside its source or into the results folder only
//! where nothing is — **a file already there is refused and left byte for
//! byte** (D261) — and the source itself is replaced only under
//! [`Written::Over`], through `wipemark_intake::inplace::replace` with the
//! original set aside first. The windows never replace without setting
//! the original aside: "no original" is a per-run flag of the CLI's and
//! never a window's.
//!
//! # What is kept
//!
//! Only for [`Plan::Loose`] with `kept` present — a paste, bytes with no
//! file behind them — and only what it asks for, in one directory per
//! clean under `Homes::kept`: `original.<ext>`, the bytes as they
//! arrived, and `result.<ext>`. A file is never copied there; the plan's
//! type has no field for it. With both switches off nothing is created,
//! the folder included. [`sweep`] removes a directory once its period has
//! passed, measured by the time in its own name.
//!
//! See `docs/plan/E7-windows-clean.md` and `docs/architecture/retention.md`.

use std::io::{self, Read as _};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, NaiveDateTime, Utc};
use wipemark_core::{CleanReport, Cleaned, Options};
use wipemark_image::{MetadataFinding, Scope};
use wipemark_intake::inplace::{self, Keep};
use wipemark_intake::{Encoding, Evidence, Format, Handed, Intake, Kind, HEAD};
use wipemark_log::Elided;
use wipemark_picture::{NotExamined, PictureError, PictureOptions, PictureReport, Visible};

use crate::compare::TEXT_LIMIT;
use crate::drop::Arrival;
use crate::retention::{with_infix, Kept, Period, Plan, Written, RESULT_INFIX};

/// The largest picture file a window cleans, in bytes (D264).
///
/// A picture is decoded whole and examined, and its raster is copied once
/// to be restored: a 64 MB PNG can be a quarter of a gigabyte of samples
/// held twice. The CLI has no window to freeze and sets no limit; the
/// windows clean one thing at a time, and this keeps that one thing to a
/// size a laptop holds beside everything else it is doing.
pub const PICTURE_LIMIT: u64 = 64 * 1024 * 1024;

/// What a picture is cleaned of from a window: AI provenance, as the MCP
/// tool's default. "All metadata" from a window is an owner question.
const SCOPE: Scope = Scope::AiProvenance;

/// Whether a thing can be cleaned, decided from what intake said about it
/// and nothing else — no read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cleanable {
    /// Characters in an encoding intake named: Layer A.
    Text(Encoding),
    /// A PNG, JPEG or WebP placed by its bytes: the picture passes.
    Picture(Format),
    /// Not this, and why.
    No(Unable),
}

/// Why a thing cannot be cleaned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unable {
    /// TIFF, HEIC, AVIF: a picture this version does not read yet.
    NotYet(Format),
    /// A folder. Its files are things of their own (Q-D2).
    Folder,
    /// Recognised, and not something either layer reads — an archive, a
    /// document, a model file, a GIF.
    Kind { kind: Kind, format: Option<Format> },
    /// Text in an eight-bit encoding intake does not name: never guessed.
    UnnamedEncoding,
    /// Nothing was established from the bytes: a name alone, or nothing at
    /// all. A name never makes a thing cleanable.
    Unread,
}

/// The three picture formats the passes read.
const PICTURES: [Format; 3] = [Format::Png, Format::Jpeg, Format::WebP];

/// The three this version names and refuses.
const NOT_YET: [Format; 3] = [Format::Tiff, Format::Heic, Format::Avif];

/// Whether `intake` can be cleaned, or why not. Pure.
pub fn cleanable(intake: &Intake) -> Cleanable {
    if intake.kind == Kind::Folder {
        return Cleanable::No(Unable::Folder);
    }
    let by_content = !matches!(intake.evidence, Evidence::Name | Evidence::Nothing);
    if let Some(format) = intake.format {
        if format.is_textual() && by_content {
            return match intake.encoding {
                Some(Encoding::Other) => Cleanable::No(Unable::UnnamedEncoding),
                Some(encoding) => Cleanable::Text(encoding),
                // An empty file: a text with nothing in it, which is what
                // `wipemark_intake::of_path` calls it and what every editor
                // on the machine says.
                None if intake.size == Some(0) => Cleanable::Text(Encoding::Utf8),
                None => Cleanable::No(Unable::Unread),
            };
        }
        if by_content && PICTURES.contains(&format) {
            return Cleanable::Picture(format);
        }
        if by_content && NOT_YET.contains(&format) {
            return Cleanable::No(Unable::NotYet(format));
        }
        if !by_content {
            return Cleanable::No(Unable::Unread);
        }
    }
    match (intake.kind, intake.evidence) {
        (_, Evidence::Name | Evidence::Nothing) => Cleanable::No(Unable::Unread),
        (kind, _) => Cleanable::No(Unable::Kind {
            kind,
            format: intake.format,
        }),
    }
}

/// What a clean came to.
#[derive(Debug)]
pub enum Verdict {
    /// Nothing to remove: nothing written.
    NothingFound,
    /// Removed, and the result written (or, for text with no file, handed
    /// back).
    Cleaned,
    /// Something is left. A picture's result is written with what could be
    /// done when it differs from the input; a text's never is, because a
    /// text is only partly clean when Layer A changed nothing.
    Partly(Left),
    /// Not cleaned, and nothing touched.
    NotCleaned(Refusal),
    /// A write failed. Nothing is where it was meant to go — or, for
    /// [`Failure::Stranded`], the original is somewhere the failure names.
    Failed(Failure),
}

impl Verdict {
    /// The log's word for it. A format, never shown.
    pub fn id(&self) -> &'static str {
        match self {
            Self::NothingFound => "nothing-found",
            Self::Cleaned => "cleaned",
            Self::Partly(_) => "partly",
            Self::NotCleaned(_) => "not-cleaned",
            Self::Failed(_) => "failed",
        }
    }
}

/// What a partly clean result still has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Left {
    /// Text: Layer A found something it keeps at its defaults — a
    /// homoglyph, which only an aggressive clean replaces — and nothing it
    /// removes (D263).
    Kept,
    /// Picture: a visible mark seen and still in the result — not proved,
    /// under opaque pixels, or restored with an outline or a texture left.
    Mark,
    /// Picture: the pixels were not examined. Not read is not clean.
    NotExamined(NotExamined),
}

/// Why a thing was not cleaned. Every one of these touches nothing.
#[derive(Debug)]
pub enum Refusal {
    /// [`cleanable`] said no — before or after the read.
    NotCleanable(Unable),
    /// Past [`TEXT_LIMIT`] or [`PICTURE_LIMIT`]; `size` is what was seen.
    TooBig { size: u64, limit: u64 },
    /// The file could not be read.
    Unreadable(io::ErrorKind),
    /// Not valid in the encoding it announced, at this byte.
    Undecodable { encoding: Encoding, offset: usize },
    /// The picture passes refused it: TIFF named late, a malformed file,
    /// pixels that would not decode or encode, a proof that failed.
    Picture(PictureError),
    /// The result would still carry AI provenance metadata. Never written.
    StillMarked { ai_metadata: bool, c2pa: bool },
    /// A file is already where the result would go (D261).
    Exists(PathBuf),
    /// `name.original.ext` is already there: the first original is the
    /// original.
    OriginalExists(PathBuf),
    /// The result would land on its own source.
    SameFile(PathBuf),
    /// The plan has no place this result can go — a folder, or a picture
    /// asked to come back as text.
    Nowhere,
}

/// A write that did not happen.
#[derive(Debug)]
pub enum Failure {
    /// The result, or a kept copy, could not be written to `path`.
    Write { path: PathBuf, error: Error },
    /// The source could not be renamed aside. Nothing moved.
    SetAside { original: PathBuf, error: Error },
    /// The result could not be written and the original could not be put
    /// back: it is at `original`.
    Stranded {
        original: PathBuf,
        error: Error,
        restore: Error,
    },
}

/// An `io::Error` reduced to what an `Outcome` can carry and a window can
/// say: the kind, for the log, and the operating system's sentence, for
/// the person — which can carry a path and therefore never goes to a log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub kind: io::ErrorKind,
    pub message: String,
}

impl From<&io::Error> for Error {
    fn from(error: &io::Error) -> Self {
        Self {
            kind: error.kind(),
            message: error.to_string(),
        }
    }
}

/// The report a clean produced, as the library's own structured value.
#[derive(Debug)]
pub enum Report {
    Text(CleanReport),
    Picture(Box<PictureReport>),
}

impl Report {
    /// The library's own JSON — a format, never translated, and always
    /// ending in its `not_established` shelf.
    pub fn to_json(&self) -> String {
        match self {
            Self::Text(report) => report.to_json(),
            Self::Picture(report) => report.to_json(),
        }
    }
}

/// Everything one clean did, for the row, the report and Compare.
#[derive(Debug)]
pub struct Outcome {
    pub verdict: Verdict,
    /// Present whenever a layer ran — on a refusal to write as well, so the
    /// report can say what was found.
    pub report: Option<Report>,
    /// Where the result is now.
    pub written: Option<PathBuf>,
    /// Whether `written` replaced a result already there — only ever when
    /// the row asked for it by name ([`replace_one`]).
    pub replaced: bool,
    /// Where the source was set aside, under [`Written::Over`].
    pub set_aside: Option<PathBuf>,
    /// The kept directory, when anything was kept.
    pub kept: Option<PathBuf>,
    /// The cleaned text, for a result that goes back as text.
    pub text: Option<String>,
    /// What the bytes turned out to be when they were read.
    pub format: Option<Format>,
    /// Bytes read, and bytes of the result.
    pub size_in: Option<u64>,
    pub size_out: Option<u64>,
}

impl Outcome {
    fn refused(refusal: Refusal) -> Self {
        Self::of(Verdict::NotCleaned(refusal))
    }

    fn of(verdict: Verdict) -> Self {
        Self {
            verdict,
            report: None,
            written: None,
            replaced: false,
            set_aside: None,
            kept: None,
            text: None,
            format: None,
            size_in: None,
            size_out: None,
        }
    }
}

/// A clean's result, before anything is decided about it.
pub enum Cleaning<'a> {
    Text {
        input: &'a str,
        cleaned: &'a Cleaned,
    },
    Picture {
        input: &'a [u8],
        output: &'a [u8],
        report: &'a PictureReport,
    },
}

/// What a result comes to, and whether it is written. The CLI's policy,
/// stated once (see the module docs). Pure.
pub fn outcome_of(result: &Cleaning) -> (Verdict, bool) {
    match *result {
        Cleaning::Text { input, cleaned } => {
            let changed = cleaned.text != input;
            let verdict = match (cleaned.report.suspicious, changed) {
                // A text Layer A changed is written, suspicious or not: a
                // soft hyphen removed is a change the CLI writes too, and
                // Compare shows the same `clean(original)` (D263).
                (_, true) => Verdict::Cleaned,
                (false, false) => Verdict::NothingFound,
                (true, false) => Verdict::Partly(Left::Kept),
            };
            (verdict, changed)
        }
        Cleaning::Picture {
            input,
            output,
            report,
        } => {
            let changed = output != input;
            let metadata = &report.metadata;
            if metadata.still_has_ai_metadata || metadata.still_has_c2pa {
                let refusal = Refusal::StillMarked {
                    ai_metadata: metadata.still_has_ai_metadata,
                    c2pa: metadata.still_has_c2pa,
                };
                return (Verdict::NotCleaned(refusal), false);
            }
            let verdict = if report.marks_left() {
                Verdict::Partly(Left::Mark)
            } else if let Visible::NotExamined(why) = report.visible {
                Verdict::Partly(Left::NotExamined(why))
            } else if metadata
                .removed
                .iter()
                .any(MetadataFinding::is_ai_provenance)
                || restored(report)
            {
                Verdict::Cleaned
            } else {
                Verdict::NothingFound
            };
            let writes = changed && !matches!(verdict, Verdict::NothingFound);
            (verdict, writes)
        }
    }
}

fn restored(report: &PictureReport) -> bool {
    matches!(&report.visible, Visible::Examined { report, .. } if !report.restored.is_empty())
}

/// Clean one thing, by `plan`, and say what happened. **Blocking.**
///
/// The plan already names both folders — the results folder in its
/// [`Written`], the kept folder in its [`Kept`] — so nothing else is
/// asked for. `row` and `now` name what this clean invents — a result's name when
/// the thing came without one, a kept directory — so that both are
/// deterministic for a given row and clock reading.
pub fn clean_one(arrival: &Arrival, plan: &Plan, row: u64, now: DateTime<Utc>) -> Outcome {
    logged(row, run(arrival, plan, row, now, None))
}

/// Clean one thing again, and write its result over `existing` — the one
/// file a first clean refused to replace (D261), named by the person who
/// asked. **Blocking.**
///
/// Only that file: if the plan taken now puts the result somewhere else, a
/// file there is refused as it always is, and the source itself is never
/// the file replaced.
pub fn replace_one(
    arrival: &Arrival,
    plan: &Plan,
    row: u64,
    now: DateTime<Utc>,
    existing: &Path,
) -> Outcome {
    logged(row, run(arrival, plan, row, now, Some(existing)))
}

/// The next number a thing is filed under in this run, across every window
/// (D280).
///
/// A clean names what it invents by its number — a result for nameless
/// bytes, a kept directory — to the second. The queue's rows and the
/// panel's cleans draw from this one counter, so two cleans in the same
/// second never share a kept directory and never overwrite each other's
/// copies there.
pub fn number() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// What a look at one thing found, without touching it — the panel's
/// findings line (E7-5).
#[derive(Debug)]
pub enum Findings {
    /// Characters: how many Layer A would remove or replace at the
    /// defaults (the counts of `inspect().findings`), and whether it found
    /// something it keeps — the case a clean calls `Partly(Kept)`.
    Text { change: usize, suspicious: bool },
    /// A picture: AI provenance metadata (C2PA included), a visible mark
    /// seen — proved or not — and why the pixels were not examined, when
    /// they were not.
    Picture {
        ai_metadata: bool,
        mark: bool,
        not_examined: Option<NotExamined>,
    },
    /// Not looked at, and why — the refusal a clean would give before it
    /// ran a layer.
    NotLooked(Refusal),
}

/// Look at one thing as [`clean_one`] would read it — the same bytes, the
/// same limits, the same decision of what they are — and run the layer's
/// inspection rather than its clean. Writes nothing. **Blocking.**
pub fn inspect_one(arrival: &Arrival) -> Findings {
    if let Cleanable::No(unable) = cleanable(&arrival.intake) {
        return Findings::NotLooked(Refusal::NotCleanable(unable));
    }
    let read = match read(arrival) {
        Ok(read) => read,
        Err(refusal) => return Findings::NotLooked(refusal),
    };
    match read.cleanable {
        Cleanable::Text(encoding) => {
            let input = match wipemark_intake::text::decode(&read.bytes, encoding) {
                Ok(input) => input,
                Err(offset) => {
                    return Findings::NotLooked(Refusal::Undecodable { encoding, offset })
                }
            };
            let report = wipemark_core::inspect(&input, &Options::default());
            Findings::Text {
                change: report
                    .findings
                    .iter()
                    .map(|finding| finding.count as usize)
                    .sum(),
                suspicious: report.suspicious,
            }
        }
        Cleanable::Picture(_) => {
            let options = PictureOptions {
                scope: SCOPE,
                catalogue: None,
            };
            match wipemark_picture::inspect(&read.bytes, &options) {
                // What the clean would remove, by the rule the clean's
                // verdict reads (`removed.any(is_ai_provenance)`). A C2PA
                // block is AI provenance by `wipemark_image`'s own
                // definition, so a manifest alone is counted (D281).
                Ok(seen) => Findings::Picture {
                    ai_metadata: seen.metadata.has_ai_metadata(),
                    mark: seen.has_visible_mark(),
                    not_examined: match seen.visible {
                        Visible::NotExamined(why) => Some(why),
                        Visible::Examined { .. } => None,
                    },
                },
                Err(error) => Findings::NotLooked(Refusal::Picture(PictureError::Image(error))),
            }
        }
        // Unreachable: `read` turns a `No` into a refusal.
        Cleanable::No(unable) => Findings::NotLooked(Refusal::NotCleanable(unable)),
    }
}

/// The characters of one thing, read and decoded exactly as [`clean_one`]
/// reads them — the same bytes, the same limit, the strict decode — or the
/// refusal a clean would give: what Compare opens on (D282). A thing whose
/// bytes turn out to be a picture is refused as one. **Blocking.**
pub fn text_of(arrival: &Arrival) -> Result<String, Refusal> {
    if let Cleanable::No(unable) = cleanable(&arrival.intake) {
        return Err(Refusal::NotCleanable(unable));
    }
    let read = read(arrival)?;
    match read.cleanable {
        Cleanable::Text(encoding) => wipemark_intake::text::decode(&read.bytes, encoding)
            .map_err(|offset| Refusal::Undecodable { encoding, offset }),
        Cleanable::Picture(format) => Err(Refusal::NotCleanable(Unable::Kind {
            kind: Kind::Image,
            format: Some(format),
        })),
        Cleanable::No(unable) => Err(Refusal::NotCleanable(unable)),
    }
}

/// The one log line a clean leaves.
fn logged(row: u64, outcome: Outcome) -> Outcome {
    tracing::info!(
        row,
        outcome = outcome.verdict.id(),
        replaced = outcome.replaced,
        format = outcome.format.map(Format::name),
        bytes_in = outcome.size_in,
        bytes_out = outcome.size_out,
        written = shape(outcome.written.as_deref()),
        set_aside = shape(outcome.set_aside.as_deref()),
        kept = shape(outcome.kept.as_deref()),
        why = why_of(&outcome.verdict),
        "clean"
    );
    outcome
}

/// A path's shape, for the log: a path can name the person and the
/// document, and the log never carries either.
fn elided(path: &Path) -> Elided {
    Elided::from(path.to_string_lossy())
}

/// [`elided`] as a log field, rendered as the line a person debugging
/// reads — `<elided chars=43 bytes=43>` — rather than the struct's `Debug`.
fn shape(path: Option<&Path>) -> Option<tracing::field::DisplayValue<Elided>> {
    path.map(|path| tracing::field::display(elided(path)))
}

/// The log's half of a refusal or a failure: an `io::ErrorKind`, never the
/// operating system's sentence.
fn why_of(verdict: &Verdict) -> Option<String> {
    match verdict {
        Verdict::NotCleaned(refusal) => Some(match refusal {
            Refusal::NotCleanable(unable) => format!("not cleanable: {unable:?}"),
            Refusal::TooBig { size, limit } => format!("too big: {size} > {limit}"),
            Refusal::Unreadable(kind) => format!("unreadable: {kind:?}"),
            Refusal::Undecodable { encoding, offset } => {
                format!("undecodable {} at {offset}", encoding.name())
            }
            Refusal::Picture(error) => format!("picture: {error:?}"),
            Refusal::StillMarked { ai_metadata, c2pa } => {
                format!("still marked: ai {ai_metadata}, c2pa {c2pa}")
            }
            Refusal::Exists(_) => String::from("exists"),
            Refusal::OriginalExists(_) => String::from("original exists"),
            Refusal::SameFile(path) => format!("same file: {}", elided(path)),
            Refusal::Nowhere => String::from("nowhere"),
        }),
        Verdict::Failed(failure) => Some(match failure {
            Failure::Write { path, error } => {
                format!("write {}: {:?}", elided(path), error.kind)
            }
            Failure::SetAside { original, error } => {
                format!("set aside as {}: {:?}", elided(original), error.kind)
            }
            Failure::Stranded {
                original,
                error,
                restore,
            } => format!(
                "stranded at {}: {:?}, {:?}",
                elided(original),
                error.kind,
                restore.kind
            ),
        }),
        _ => None,
    }
}

/// What was read: the bytes, what they turned out to be, and the file they
/// came from, when there is one.
struct Read {
    bytes: Vec<u8>,
    cleanable: Cleanable,
    format: Option<Format>,
    path: Option<PathBuf>,
}

fn run(
    arrival: &Arrival,
    plan: &Plan,
    row: u64,
    now: DateTime<Utc>,
    replacing: Option<&Path>,
) -> Outcome {
    // Decided on the intake first: a folder, a ZIP, a TIFF is refused
    // before a byte is read.
    if let Cleanable::No(unable) = cleanable(&arrival.intake) {
        return Outcome::refused(Refusal::NotCleanable(unable));
    }
    let read = match read(arrival) {
        Ok(read) => read,
        Err(refusal) => return Outcome::refused(refusal),
    };
    let size_in = read.bytes.len() as u64;

    // Then the layer, by what the bytes *are* now.
    let (verdict, writes, report, bytes, text) = match read.cleanable {
        Cleanable::Text(encoding) => {
            let input = match wipemark_intake::text::decode(&read.bytes, encoding) {
                Ok(input) => input,
                Err(offset) => {
                    return Outcome {
                        format: read.format,
                        size_in: Some(size_in),
                        ..Outcome::refused(Refusal::Undecodable { encoding, offset })
                    }
                }
            };
            let cleaned = wipemark_core::clean(&input, &Options::default());
            let (verdict, writes) = outcome_of(&Cleaning::Text {
                input: &input,
                cleaned: &cleaned,
            });
            let bytes = wipemark_intake::text::encode(&cleaned.text, encoding);
            let Cleaned { text, report } = cleaned;
            (verdict, writes, Report::Text(report), bytes, text)
        }
        Cleanable::Picture(_) => {
            let options = PictureOptions {
                scope: SCOPE,
                catalogue: None,
            };
            let (output, report) = match wipemark_picture::clean(&read.bytes, &options) {
                Ok(cleaned) => cleaned,
                Err(error) => {
                    return Outcome {
                        format: read.format,
                        size_in: Some(size_in),
                        ..Outcome::refused(Refusal::Picture(error))
                    }
                }
            };
            let (verdict, writes) = outcome_of(&Cleaning::Picture {
                input: &read.bytes,
                output: &output,
                report: &report,
            });
            (
                verdict,
                writes,
                Report::Picture(Box::new(report)),
                output,
                String::new(),
            )
        }
        // Unreachable: `read` turns a `No` into a refusal.
        Cleanable::No(unable) => return Outcome::refused(Refusal::NotCleanable(unable)),
    };

    let mut outcome = Outcome {
        verdict,
        report: Some(report),
        format: read.format,
        size_in: Some(size_in),
        size_out: Some(bytes.len() as u64),
        ..Outcome::of(Verdict::NothingFound)
    };
    if !writes {
        return outcome;
    }

    // Where it goes, by the plan taken when the clean started.
    let (written, kept) = match plan {
        Plan::File(written) => (written, None),
        Plan::Loose { result, kept } => (result, kept.as_ref()),
        Plan::EachFileIn(_) => {
            outcome.verdict = Verdict::NotCleaned(Refusal::Nowhere);
            return outcome;
        }
    };
    let ext = extension_of(read.format);

    // Kept copies first: if the copy of a paste cannot be made, the result
    // does not replace the only other place it exists.
    if let Some(kept) = kept {
        let original = original_bytes(arrival, &read);
        match keep(kept, row, now, ext, &original, &bytes) {
            Ok(None) => {}
            Ok(Some(directory)) => {
                outcome.kept = Some(directory);
                if let Err(error) = sweep(&kept.in_, kept.for_, now) {
                    tracing::warn!(error = ?error.kind(), "sweep after keeping failed");
                }
            }
            Err(failure) => {
                outcome.verdict = Verdict::Failed(failure);
                return outcome;
            }
        }
    }

    match written {
        Written::AsText => match read.cleanable {
            Cleanable::Text(_) => outcome.text = Some(text),
            _ => outcome.verdict = Verdict::NotCleaned(Refusal::Nowhere),
        },
        Written::Beside(destination) => {
            match write_new(destination, &bytes, read.path.as_deref(), replacing) {
                Ok(replaced) => {
                    outcome.written = Some(destination.clone());
                    outcome.replaced = replaced;
                }
                Err(verdict) => outcome.verdict = verdict,
            }
        }
        Written::Into { folder, name } => {
            let name = match name {
                Some(name) => name.clone(),
                None => invented_name(row, now, ext),
            };
            let destination = folder.join(name);
            match write_new(&destination, &bytes, read.path.as_deref(), replacing) {
                Ok(replaced) => {
                    outcome.written = Some(destination);
                    outcome.replaced = replaced;
                }
                Err(verdict) => outcome.verdict = verdict,
            }
        }
        Written::Over { file, .. } => {
            // Over the file that was read, and no other.
            if read.path.as_deref() != Some(file.as_path()) {
                outcome.verdict = Verdict::NotCleaned(Refusal::Nowhere);
                return outcome;
            }
            match inplace::replace(file, &bytes, Keep::Original) {
                Ok(replaced) => {
                    outcome.written = Some(file.clone());
                    outcome.set_aside = replaced.original;
                }
                Err(failure) => outcome.verdict = replace_failed(failure, file),
            }
        }
    }
    outcome
}

/// What a failed in-place replacement comes to. Everything but `Stranded`
/// left the file as it was.
fn replace_failed(failure: inplace::Failure, file: &Path) -> Verdict {
    match failure {
        inplace::Failure::OriginalExists(original) => {
            Verdict::NotCleaned(Refusal::OriginalExists(original))
        }
        inplace::Failure::Unnamed => Verdict::NotCleaned(Refusal::Nowhere),
        inplace::Failure::SetAside { original, error } => Verdict::Failed(Failure::SetAside {
            original,
            error: Error::from(&error),
        }),
        inplace::Failure::Write(error) => Verdict::Failed(Failure::Write {
            path: file.to_owned(),
            error: Error::from(&error),
        }),
        inplace::Failure::Stranded {
            original,
            error,
            restore,
        } => Verdict::Failed(Failure::Stranded {
            original,
            error: Error::from(&error),
            restore: Error::from(&restore),
        }),
    }
}

/// Write a result where no file is: never over one already there (D261)
/// unless it is the very file `replacing` names, and never over its own
/// source, named or not. `Ok(true)` when a file was replaced.
fn write_new(
    destination: &Path,
    bytes: &[u8],
    source: Option<&Path>,
    replacing: Option<&Path>,
) -> Result<bool, Verdict> {
    if source.is_some_and(|source| inplace::same_file(source, destination)) {
        return Err(Verdict::NotCleaned(Refusal::SameFile(
            destination.to_owned(),
        )));
    }
    // `symlink_metadata`, so that a dangling link is in the way too.
    let there = std::fs::symlink_metadata(destination).is_ok();
    if there && replacing != Some(destination) {
        return Err(Verdict::NotCleaned(Refusal::Exists(destination.to_owned())));
    }
    // Over the named file by a rename; anywhere else only where nothing is,
    // which the publish itself checks — a file another process put there
    // after the check above is refused too, never replaced (D284).
    let written = if there {
        inplace::write_atomically(destination, bytes, source)
    } else {
        inplace::write_new(destination, bytes, source)
    };
    match written {
        Ok(()) => Ok(there),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            Err(Verdict::NotCleaned(Refusal::Exists(destination.to_owned())))
        }
        Err(error) => Err(Verdict::Failed(Failure::Write {
            path: destination.to_owned(),
            error: Error::from(&error),
        })),
    }
}

/// The name a result gets when the thing came with none:
/// `wipemark-<yyyymmdd-hhmmss>-<row>.cleaned.<ext>`, the time on this
/// machine's clock as a person reads it.
pub fn invented_name(row: u64, now: DateTime<Utc>, ext: &str) -> String {
    let stamp = now.with_timezone(&Local).format("%Y%m%d-%H%M%S");
    with_infix(&format!("wipemark-{stamp}-{row}.{ext}"), RESULT_INFIX)
}

/// The extension a result or a kept copy of this format is written with.
pub fn extension_of(format: Option<Format>) -> &'static str {
    match format {
        Some(Format::Png) => "png",
        Some(Format::Jpeg) => "jpg",
        Some(Format::WebP) => "webp",
        Some(Format::Markdown) => "md",
        Some(Format::Html) => "html",
        Some(Format::Xml) => "xml",
        Some(Format::Json) => "json",
        Some(Format::Csv) => "csv",
        Some(Format::Svg) => "svg",
        Some(Format::Rtf) => "rtf",
        _ => "txt",
    }
}

/// The bytes as they arrived — never text extracted from them (retention
/// rule 7). A paste is its own characters, as UTF-8.
fn original_bytes(arrival: &Arrival, read: &Read) -> Vec<u8> {
    match &arrival.handed {
        Handed::Text(text) if read.path.is_none() => text.as_bytes().to_vec(),
        Handed::Bytes { bytes, .. } => bytes.clone(),
        _ => read.bytes.clone(),
    }
}

/// The directory one clean keeps its copies in: the time in UTC, then the
/// row — `20261005T134602-7`. UTC because [`sweep`] does arithmetic on it.
fn kept_directory(kept: &Path, row: u64, now: DateTime<Utc>) -> PathBuf {
    kept.join(format!("{}-{row}", now.format("%Y%m%dT%H%M%S")))
}

/// Keep what `kept` asks for. Nothing is created unless something is kept.
fn keep(
    kept: &Kept,
    row: u64,
    now: DateTime<Utc>,
    ext: &str,
    original: &[u8],
    result: &[u8],
) -> Result<Option<PathBuf>, Failure> {
    let directory = kept_directory(&kept.in_, row, now);
    let copies = [
        (kept.original, "original", original),
        (kept.result, "result", result),
    ];
    if !copies.iter().any(|(asked, ..)| *asked) {
        return Ok(None);
    }
    let failed = |path: &Path, error: &io::Error| Failure::Write {
        path: path.to_owned(),
        error: Error::from(error),
    };
    std::fs::create_dir_all(&directory).map_err(|error| failed(&directory, &error))?;
    for (asked, stem, bytes) in copies {
        if asked {
            let path = directory.join(format!("{stem}.{ext}"));
            inplace::write_atomically(&path, bytes, None).map_err(|error| failed(&path, &error))?;
        }
    }
    Ok(Some(directory))
}

/// Remove the kept directories older than `keep_for`, by the time in their
/// own names. Never anything not shaped like one of ours, never a link,
/// and nothing at all under [`Period::Forever`]. Returns how many went.
/// **Blocking.**
pub fn sweep(kept: &Path, keep_for: Period, now: DateTime<Utc>) -> io::Result<usize> {
    let Some(days) = days_of(keep_for) else {
        return Ok(0);
    };
    let entries = match std::fs::read_dir(kept) {
        Ok(entries) => entries,
        // Nothing kept yet is the ordinary state.
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
    };
    let mut removed = 0;
    for entry in entries {
        let entry = entry?;
        let Some(when) = entry.file_name().to_str().and_then(stamp_of) else {
            continue;
        };
        // A directory, not a link to one: removing through a link would
        // remove somebody else's files.
        if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        if now.naive_utc() - when > chrono::Duration::days(days) {
            std::fs::remove_dir_all(entry.path())?;
            removed += 1;
        }
    }
    if removed > 0 {
        tracing::info!(removed, keep_for = keep_for.id(), "kept copies swept");
    }
    Ok(removed)
}

/// The number of days in a period's own id; `None` for forever.
fn days_of(period: Period) -> Option<i64> {
    period.id().strip_suffix('d')?.parse().ok()
}

/// The time in a kept directory's name, or `None` for a name that is not
/// one of ours: `yyyymmddThhmmss-<digits>`, and nothing else.
fn stamp_of(name: &str) -> Option<NaiveDateTime> {
    let (stamp, row) = name.split_once('-')?;
    if row.is_empty() || !row.bytes().all(|byte| byte.is_ascii_digit()) || stamp.len() != 15 {
        return None;
    }
    NaiveDateTime::parse_from_str(stamp, "%Y%m%dT%H%M%S").ok()
}

/// Read what arrived, and say again what it is — from the bytes actually
/// read, because a file can change after it was dropped and the bytes
/// decide.
fn read(arrival: &Arrival) -> Result<Read, Refusal> {
    let path = arrival
        .intake
        .path
        .clone()
        .or_else(|| match &arrival.handed {
            Handed::Path(path) => Some(path.clone()),
            _ => None,
        });
    if let Some(path) = path {
        return read_file(&path);
    }
    match &arrival.handed {
        Handed::Text(text) => {
            let size = text.len() as u64;
            if size > TEXT_LIMIT {
                return Err(Refusal::TooBig {
                    size,
                    limit: TEXT_LIMIT,
                });
            }
            // Characters are characters: a Rust string has no other
            // encoding, and nothing a paste could hold is a PNG.
            Ok(Read {
                bytes: text.as_bytes().to_vec(),
                cleanable: Cleanable::Text(Encoding::Utf8),
                format: arrival.intake.format,
                path: None,
            })
        }
        Handed::Bytes { name, bytes } => {
            let intake = wipemark_intake::of_bytes(bytes, name.as_deref());
            let cleanable = cleanable(&intake);
            check_size(cleanable, bytes.len() as u64)?;
            Ok(Read {
                bytes: bytes.clone(),
                cleanable,
                format: intake.format,
                path: None,
            })
        }
        // A path is read above.
        Handed::Path(_) => Err(Refusal::NotCleanable(Unable::Unread)),
    }
}

/// The limit for what `cleanable` is, or the refusal it already is.
fn limit_of(cleanable: Cleanable) -> Result<u64, Refusal> {
    match cleanable {
        Cleanable::Text(_) => Ok(TEXT_LIMIT),
        Cleanable::Picture(_) => Ok(PICTURE_LIMIT),
        Cleanable::No(unable) => Err(Refusal::NotCleanable(unable)),
    }
}

fn check_size(cleanable: Cleanable, size: u64) -> Result<(), Refusal> {
    let limit = limit_of(cleanable)?;
    if size > limit {
        return Err(Refusal::TooBig { size, limit });
    }
    Ok(())
}

/// A file: its head first, identified again; the rest only when what the
/// head is can be cleaned, and never past the limit for it.
fn read_file(path: &Path) -> Result<Read, Refusal> {
    let unreadable = |error: io::Error| Refusal::Unreadable(error.kind());
    let metadata = std::fs::metadata(path).map_err(unreadable)?;
    if metadata.is_dir() {
        return Err(Refusal::NotCleanable(Unable::Folder));
    }
    let mut file = std::fs::File::open(path).map_err(unreadable)?;
    let mut bytes = Vec::with_capacity(HEAD);
    file.by_ref()
        .take(HEAD as u64)
        .read_to_end(&mut bytes)
        .map_err(unreadable)?;

    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    let mut intake = wipemark_intake::identify(&bytes, name.as_deref());
    intake.size = Some(metadata.len());
    if bytes.is_empty() && metadata.len() == 0 {
        // Empty: a text with nothing in it, as `of_path` says.
        intake.format = Some(Format::PlainText);
        intake.evidence = Evidence::Content;
    }
    let cleanable = cleanable(&intake);
    let limit = limit_of(cleanable)?;
    if metadata.len() > limit {
        return Err(Refusal::TooBig {
            size: metadata.len(),
            limit,
        });
    }
    // The file may have grown since the `stat`: one byte past the limit is
    // enough to know.
    let rest = (limit + 1).saturating_sub(bytes.len() as u64);
    file.take(rest)
        .read_to_end(&mut bytes)
        .map_err(unreadable)?;
    if bytes.len() as u64 > limit {
        return Err(Refusal::TooBig {
            size: bytes.len() as u64,
            limit,
        });
    }
    Ok(Read {
        bytes,
        cleanable,
        format: intake.format,
        path: Some(path.to_owned()),
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use chrono::{DateTime, TimeZone as _, Utc};
    use wipemark_image::ImageError;
    use wipemark_intake::{Arrived, Encoding, Evidence, Format, Handed, Intake, Kind};

    use super::*;
    use crate::retention::{self, Destination, Homes, Retention, Source};

    /// A scratch directory that takes its own files away with it.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(label: &str) -> Self {
            let directory =
                std::env::temp_dir().join(format!("wipemark-clean-{label}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&directory);
            std::fs::create_dir_all(&directory).expect("scratch directory");
            Self(directory)
        }

        fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.0.join(name);
            std::fs::write(&path, bytes).expect("scratch file");
            path
        }

        fn homes(&self) -> Homes {
            Homes {
                results: self.0.join("results"),
                kept: self.0.join("kept"),
            }
        }
    }

    /// Every name in a folder, sorted.
    fn names(folder: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(folder)
            .expect("list")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).ok();
        }
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 5, 13, 46, 2).unwrap()
    }

    fn arrival(handed: Handed) -> Arrival {
        Arrival {
            intake: wipemark_intake::of(&handed),
            handed,
        }
    }

    fn planned(arrival: &Arrival, retention: &Retention, homes: &Homes) -> Plan {
        retention::plan(&Source::of(&arrival.intake), retention, homes)
    }

    /// Clean `arrival` under `retention`, the way a row would.
    fn clean(arrival: &Arrival, retention: &Retention, homes: &Homes) -> Outcome {
        clean_one(arrival, &planned(arrival, retention, homes), 7, now())
    }

    fn read(path: &Path) -> Vec<u8> {
        std::fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
    }

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(format!(
            "{}/../../fixtures/image/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
    }

    fn intake(format: Option<Format>, encoding: Option<Encoding>, evidence: Evidence) -> Intake {
        Intake {
            kind: format.map_or(Kind::Unknown, Format::kind),
            format,
            encoding,
            name: None,
            path: None,
            size: Some(10),
            evidence,
            arrived: Arrived::AsPath,
        }
    }

    fn encoded(text: &str, encoding: Encoding) -> Vec<u8> {
        wipemark_intake::text::encode(text, encoding)
    }

    const MARKED: &str = "# Notes\n\nA zero\u{200B}width space.\n";

    // -- cleanable -----------------------------------------------------

    /// The table of the task, row by row, from the intake alone.
    #[test]
    fn what_can_be_cleaned_is_decided_from_the_intake() {
        use Evidence::{Agreed, Content, Name, Nothing};
        assert_eq!(
            cleanable(&intake(
                Some(Format::Markdown),
                Some(Encoding::Utf16Le),
                Agreed
            )),
            Cleanable::Text(Encoding::Utf16Le)
        );
        assert_eq!(
            cleanable(&intake(
                Some(Format::PlainText),
                Some(Encoding::Other),
                Content
            )),
            Cleanable::No(Unable::UnnamedEncoding)
        );
        for format in [Format::Png, Format::Jpeg, Format::WebP] {
            assert_eq!(
                cleanable(&intake(Some(format), None, Content)),
                Cleanable::Picture(format)
            );
            // A name alone never makes a picture.
            assert_eq!(
                cleanable(&intake(Some(format), None, Name)),
                Cleanable::No(Unable::Unread),
                "{format:?} by name"
            );
        }
        // The bytes won an argument with the name: still a picture.
        assert_eq!(
            cleanable(&intake(
                Some(Format::Png),
                None,
                Evidence::Disagreed {
                    name_said: Format::PlainText
                }
            )),
            Cleanable::Picture(Format::Png)
        );
        for format in [Format::Tiff, Format::Heic, Format::Avif] {
            assert_eq!(
                cleanable(&intake(Some(format), None, Content)),
                Cleanable::No(Unable::NotYet(format))
            );
        }
        for format in [
            Format::Zip,
            Format::Docx,
            Format::Mp4,
            Format::Gguf,
            Format::Gif,
        ] {
            assert_eq!(
                cleanable(&intake(Some(format), None, Content)),
                Cleanable::No(Unable::Kind {
                    kind: format.kind(),
                    format: Some(format)
                }),
                "{format:?}"
            );
        }
        assert_eq!(
            cleanable(&intake(None, None, Content)),
            Cleanable::No(Unable::Kind {
                kind: Kind::Unknown,
                format: None
            })
        );
        assert_eq!(
            cleanable(&intake(None, None, Nothing)),
            Cleanable::No(Unable::Unread)
        );
        let folder = Intake {
            kind: Kind::Folder,
            ..intake(None, None, Content)
        };
        assert_eq!(cleanable(&folder), Cleanable::No(Unable::Folder));
        // An empty file is a text with nothing in it.
        let empty = Intake {
            size: Some(0),
            ..intake(Some(Format::PlainText), None, Content)
        };
        assert_eq!(cleanable(&empty), Cleanable::Text(Encoding::Utf8));
    }

    // -- text ----------------------------------------------------------

    /// The headline: beside the file, the result is `encode(clean(text))`
    /// in the encoding the file arrived in, and the source is untouched —
    /// in UTF-8, in UTF-16LE with its mark, and in UTF-32BE.
    #[test]
    fn a_marked_text_is_written_beside_it_in_its_own_encoding() {
        let scratch = Scratch::new("beside");
        for (encoding, text) in [
            (Encoding::Utf8, MARKED.to_owned()),
            (Encoding::Utf16Le, format!("\u{FEFF}{MARKED}")),
            (Encoding::Utf32Be, format!("\u{FEFF}{MARKED}")),
        ] {
            let name = format!("x-{}.md", encoding.name());
            let bytes = encoded(&text, encoding);
            let source = scratch.file(&name, &bytes);
            let outcome = clean(
                &arrival(Handed::Path(source.clone())),
                &Retention::default(),
                &scratch.homes(),
            );
            assert!(
                matches!(outcome.verdict, Verdict::Cleaned),
                "{encoding:?}: {:?}",
                outcome.verdict
            );
            let beside = scratch.0.join(format!("x-{}.cleaned.md", encoding.name()));
            assert_eq!(outcome.written.as_deref(), Some(beside.as_path()));
            let expected = wipemark_core::clean(&text, &Options::default()).text;
            assert!(!expected.contains('\u{200B}'));
            assert_eq!(read(&beside), encoded(&expected, encoding), "{encoding:?}");
            assert_eq!(read(&source), bytes, "{encoding:?}: the source was touched");
            assert!(matches!(outcome.report, Some(Report::Text(_))));
        }
    }

    /// Nothing found is nothing written: no `x.cleaned.md` that is `x.md`
    /// under another name (D260).
    #[test]
    fn nothing_found_writes_nothing() {
        let scratch = Scratch::new("nothing");
        let source = scratch.file("plain.md", b"# Plain\n\nNothing hidden here.\n");
        let outcome = clean(
            &arrival(Handed::Path(source)),
            &Retention::default(),
            &scratch.homes(),
        );
        assert!(
            matches!(outcome.verdict, Verdict::NothingFound),
            "{:?}",
            outcome.verdict
        );
        assert_eq!(outcome.written, None);
        assert_eq!(names(&scratch.0), ["plain.md"]);
    }

    /// A file already where the result would go is refused, and left byte
    /// for byte (D261).
    #[test]
    fn an_existing_result_is_refused_and_left_alone() {
        let scratch = Scratch::new("exists");
        let source = scratch.file("x.md", MARKED.as_bytes());
        let existing = scratch.file("x.cleaned.md", b"somebody's own file");
        let outcome = clean(
            &arrival(Handed::Path(source)),
            &Retention::default(),
            &scratch.homes(),
        );
        assert!(
            matches!(&outcome.verdict, Verdict::NotCleaned(Refusal::Exists(path)) if *path == existing),
            "{:?}",
            outcome.verdict
        );
        assert_eq!(outcome.written, None);
        assert!(!outcome.replaced);
        assert_eq!(read(&existing), b"somebody's own file");
    }

    /// "Replace the existing result" writes over the one file it names,
    /// and says so; a different file in the way is refused as ever, and
    /// the source is never the file replaced, even when it is the one
    /// named.
    #[test]
    fn a_result_is_replaced_only_where_it_was_named() {
        let scratch = Scratch::new("replace");
        let source = scratch.file("x.md", MARKED.as_bytes());
        let existing = scratch.file("x.cleaned.md", b"somebody's own file");
        let arrival = arrival(Handed::Path(source.clone()));
        let plan = planned(&arrival, &Retention::default(), &scratch.homes());

        let elsewhere = scratch.0.join("y.cleaned.md");
        let refused = replace_one(&arrival, &plan, 7, now(), &elsewhere);
        assert!(
            matches!(&refused.verdict, Verdict::NotCleaned(Refusal::Exists(path)) if *path == existing),
            "{:?}",
            refused.verdict
        );
        assert_eq!(read(&existing), b"somebody's own file");

        let replaced = replace_one(&arrival, &plan, 7, now(), &existing);
        assert!(
            matches!(replaced.verdict, Verdict::Cleaned),
            "{:?}",
            replaced.verdict
        );
        assert!(replaced.replaced);
        assert_eq!(replaced.written.as_deref(), Some(existing.as_path()));
        assert_eq!(
            read(&existing),
            wipemark_core::clean(MARKED, &Options::default())
                .text
                .as_bytes()
        );
        assert_eq!(read(&source), MARKED.as_bytes(), "the source moved");

        // Named the source itself, under a plan that would land on it.
        let into = Retention {
            destination: Destination::Folder,
            folder: Some(scratch.0.clone()),
            ..Retention::default()
        };
        let mut onto_source = planned(&arrival, &into, &scratch.homes());
        if let Plan::File(Written::Into { name, .. }) = &mut onto_source {
            *name = Some(String::from("x.md"));
        }
        let refused = replace_one(&arrival, &onto_source, 7, now(), &source);
        assert!(
            matches!(refused.verdict, Verdict::NotCleaned(Refusal::SameFile(_))),
            "{:?}",
            refused.verdict
        );
        assert_eq!(read(&source), MARKED.as_bytes(), "the source was replaced");
    }

    /// In place: the original is set aside first, then the file replaced;
    /// a second time, the original already there refuses and nothing
    /// moves.
    #[test]
    fn in_place_sets_the_original_aside_and_never_twice() {
        let scratch = Scratch::new("over");
        let source = scratch.file("x.md", MARKED.as_bytes());
        let over = Retention {
            destination: Destination::Replace,
            ..Retention::default()
        };
        let outcome = clean(
            &arrival(Handed::Path(source.clone())),
            &over,
            &scratch.homes(),
        );
        assert!(
            matches!(outcome.verdict, Verdict::Cleaned),
            "{:?}",
            outcome.verdict
        );
        let original = scratch.0.join("x.original.md");
        assert_eq!(outcome.set_aside.as_deref(), Some(original.as_path()));
        assert_eq!(outcome.written.as_deref(), Some(source.as_path()));
        assert_eq!(read(&original), MARKED.as_bytes());
        assert!(!String::from_utf8(read(&source))
            .unwrap()
            .contains('\u{200B}'));

        // Marked again, and dropped again: the first original stays.
        std::fs::write(&source, "again\u{200B}").expect("write");
        let outcome = clean(
            &arrival(Handed::Path(source.clone())),
            &over,
            &scratch.homes(),
        );
        assert!(
            matches!(&outcome.verdict, Verdict::NotCleaned(Refusal::OriginalExists(path)) if *path == original),
            "{:?}",
            outcome.verdict
        );
        assert_eq!(read(&source), "again\u{200B}".as_bytes());
        assert_eq!(read(&original), MARKED.as_bytes());
        assert_eq!(names(&scratch.0), ["x.md", "x.original.md"]);
    }

    /// In place replaces the file that was read and no other: a plan whose
    /// `Over` names a different file is refused as having nowhere to go,
    /// and both files are left as they were.
    #[test]
    fn in_place_never_replaces_a_file_other_than_the_one_read() {
        let scratch = Scratch::new("over-other");
        let source = scratch.file("x.md", MARKED.as_bytes());
        let other = scratch.file("other.md", b"somebody else's file");
        let wrong = Plan::File(Written::Over {
            file: other.clone(),
            set_aside_as: scratch.0.join("other.original.md"),
        });
        let outcome = clean_one(&arrival(Handed::Path(source.clone())), &wrong, 7, now());
        assert!(
            matches!(outcome.verdict, Verdict::NotCleaned(Refusal::Nowhere)),
            "{:?}",
            outcome.verdict
        );
        assert_eq!(outcome.written, None);
        assert_eq!(read(&other), b"somebody else's file");
        assert_eq!(read(&source), MARKED.as_bytes());
        assert_eq!(names(&scratch.0), ["other.md", "x.md"]);
    }

    /// A dangling link where the result would go is something in the way:
    /// refused as existing, and the link left exactly as it was — never
    /// followed to create the file it points at.
    #[cfg(unix)]
    #[test]
    fn a_dangling_link_where_the_result_goes_is_in_the_way() {
        let scratch = Scratch::new("dangling");
        let source = scratch.file("x.md", MARKED.as_bytes());
        let link = scratch.0.join("x.cleaned.md");
        let target = scratch.0.join("nowhere").join("target.md");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");
        let outcome = clean(
            &arrival(Handed::Path(source)),
            &Retention::default(),
            &scratch.homes(),
        );
        assert!(
            matches!(&outcome.verdict, Verdict::NotCleaned(Refusal::Exists(path)) if *path == link),
            "{:?}",
            outcome.verdict
        );
        assert_eq!(std::fs::read_link(&link).expect("still a link"), target);
        assert!(!target.exists());
        assert_eq!(names(&scratch.0), ["x.cleaned.md", "x.md"]);
    }

    /// Keeping sweeps: a kept directory past its period is gone after the
    /// next clean that keeps something, with no launch in between (D267).
    #[test]
    fn a_clean_that_keeps_sweeps_what_has_expired() {
        let scratch = Scratch::new("keep-sweeps");
        let homes = scratch.homes();
        let old = homes.kept.join("20260920T120000-3");
        let fresh = homes.kept.join("20261004T120000-4");
        for directory in [&old, &fresh] {
            std::fs::create_dir_all(directory).expect("kept");
            std::fs::write(directory.join("original.txt"), b"x").expect("copy");
        }
        let keeping = Retention {
            keep_originals: true,
            keep_for: Period::Week,
            ..Retention::default()
        };
        let outcome = clean(&arrival(Handed::Text(MARKED.to_owned())), &keeping, &homes);
        assert!(outcome.kept.is_some(), "{:?}", outcome.verdict);
        assert!(!old.exists(), "a copy past its week stayed");
        assert!(fresh.exists(), "a copy inside its week went");
    }

    /// Into the results folder — and never over the source, even when the
    /// results folder is the source's own.
    #[test]
    fn into_the_results_folder_and_never_over_the_source() {
        let scratch = Scratch::new("into");
        let results = scratch.0.join("results");
        std::fs::create_dir_all(&results).expect("results");
        let source = scratch.file("x.md", MARKED.as_bytes());
        let into = Retention {
            destination: Destination::Folder,
            ..Retention::default()
        };
        let outcome = clean(
            &arrival(Handed::Path(source.clone())),
            &into,
            &scratch.homes(),
        );
        assert_eq!(
            outcome.written.as_deref(),
            Some(results.join("x.cleaned.md").as_path())
        );
        assert_eq!(read(&source), MARKED.as_bytes());

        // The results folder is the source's folder, and the source is
        // itself called `.cleaned`: the plan names another file, and the
        // write refuses to land on the source whatever it names.
        let own = Retention {
            destination: Destination::Folder,
            folder: Some(scratch.0.clone()),
            ..Retention::default()
        };
        let source = scratch.file("y.cleaned.md", MARKED.as_bytes());
        let outcome = clean(
            &arrival(Handed::Path(source.clone())),
            &own,
            &scratch.homes(),
        );
        assert_eq!(
            outcome.written.as_deref(),
            Some(scratch.0.join("y.cleaned.cleaned.md").as_path())
        );
        assert_eq!(read(&source), MARKED.as_bytes());
        // And a plan that did name the source is refused.
        let wrong = Plan::File(Written::Into {
            folder: scratch.0.clone(),
            name: Some("y.cleaned.md".to_owned()),
        });
        let outcome = clean_one(&arrival(Handed::Path(source.clone())), &wrong, 7, now());
        assert!(
            matches!(outcome.verdict, Verdict::NotCleaned(Refusal::SameFile(_))),
            "{:?}",
            outcome.verdict
        );
        assert_eq!(read(&source), MARKED.as_bytes());
    }

    /// A paste comes back as text and writes nothing anywhere — and with
    /// both switches off, `kept/` does not even exist afterwards.
    #[test]
    fn a_paste_writes_nothing_and_keeps_nothing_unless_asked() {
        let scratch = Scratch::new("paste");
        let homes = scratch.homes();
        let pasted = arrival(Handed::Text(MARKED.to_owned()));
        let outcome = clean(&pasted, &Retention::default(), &homes);
        assert!(
            matches!(outcome.verdict, Verdict::Cleaned),
            "{:?}",
            outcome.verdict
        );
        assert_eq!(outcome.written, None);
        assert_eq!(outcome.kept, None);
        assert_eq!(
            outcome.text.as_deref(),
            Some(
                wipemark_core::clean(MARKED, &Options::default())
                    .text
                    .as_str()
            )
        );
        assert!(
            !homes.kept.exists(),
            "kept/ was created with nothing to keep"
        );
        assert!(!homes.results.exists());
        assert_eq!(names(&scratch.0), Vec::<String>::new());

        // Keep originals: the characters as they arrived, and only them.
        let keeping = Retention {
            keep_originals: true,
            ..Retention::default()
        };
        let outcome = clean(&pasted, &keeping, &homes);
        let directory = homes.kept.join("20261005T134602-7");
        assert_eq!(outcome.kept.as_deref(), Some(directory.as_path()));
        // A paste has no name, and no signature says Markdown: plain text.
        assert_eq!(names(&directory), ["original.txt"]);
        assert_eq!(read(&directory.join("original.txt")), MARKED.as_bytes());
        assert_eq!(outcome.written, None);
    }

    /// A file is never copied into `kept/`, whatever the switches say: the
    /// file is the original.
    #[test]
    fn a_dropped_file_is_never_kept() {
        let scratch = Scratch::new("never-kept");
        let source = scratch.file("x.md", MARKED.as_bytes());
        let both = Retention {
            keep_originals: true,
            keep_results: true,
            ..Retention::default()
        };
        let homes = scratch.homes();
        let outcome = clean(&arrival(Handed::Path(source)), &both, &homes);
        assert!(
            matches!(outcome.verdict, Verdict::Cleaned),
            "{:?}",
            outcome.verdict
        );
        assert_eq!(outcome.kept, None);
        assert!(!homes.kept.exists(), "a file was copied into kept/");
    }

    /// Bytes with no file behind them land in the results folder; with no
    /// name, under one made of the clock and the row.
    #[test]
    fn nameless_bytes_get_a_name_of_the_clock_and_the_row() {
        let scratch = Scratch::new("nameless");
        let homes = scratch.homes();
        std::fs::create_dir_all(&homes.results).expect("results");
        let picture = read(&fixture("xmp-provenance-url.png"));
        let both = Retention {
            keep_originals: true,
            keep_results: true,
            ..Retention::default()
        };
        let outcome = clean(
            &arrival(Handed::Bytes {
                name: None,
                bytes: picture.clone(),
            }),
            &both,
            &homes,
        );
        assert!(
            matches!(outcome.verdict, Verdict::Cleaned),
            "{:?}",
            outcome.verdict
        );
        let name = invented_name(7, now(), "png");
        assert!(name.starts_with("wipemark-2026100"), "{name}");
        assert!(name.ends_with("-7.cleaned.png"), "{name}");
        assert_eq!(name, invented_name(7, now(), "png"), "not deterministic");
        assert_eq!(
            outcome.written.as_deref(),
            Some(homes.results.join(&name).as_path())
        );
        let directory = outcome.kept.expect("kept");
        assert_eq!(names(&directory), ["original.png", "result.png"]);
        assert_eq!(read(&directory.join("original.png")), picture);
        assert_eq!(
            read(&directory.join("result.png")),
            read(&homes.results.join(&name))
        );
    }

    /// A text whose file became a PNG after the drop is cleaned as the PNG
    /// it is — never decoded as text.
    #[test]
    fn a_file_that_changed_after_the_drop_is_read_by_its_bytes() {
        let scratch = Scratch::new("changed");
        let source = scratch.file("note.txt", MARKED.as_bytes());
        let dropped = arrival(Handed::Path(source.clone()));
        assert_eq!(cleanable(&dropped.intake), Cleanable::Text(Encoding::Utf8));
        std::fs::copy(fixture("xmp-provenance-url.png"), &source).expect("copy");
        let outcome = clean(&dropped, &Retention::default(), &scratch.homes());
        assert_eq!(outcome.format, Some(Format::Png));
        assert!(
            matches!(outcome.report, Some(Report::Picture(_))),
            "{:?}",
            outcome.verdict
        );
        assert!(
            matches!(outcome.verdict, Verdict::Cleaned),
            "{:?}",
            outcome.verdict
        );
    }

    /// Past the limit is refused with its size, before the rest is read.
    #[test]
    fn a_text_past_the_limit_is_refused_with_its_size() {
        let scratch = Scratch::new("big");
        let size = TEXT_LIMIT + 1;
        let source = scratch.file("big.txt", &vec![b'a'; size as usize]);
        let outcome = clean(
            &arrival(Handed::Path(source)),
            &Retention::default(),
            &scratch.homes(),
        );
        assert!(
            matches!(
                outcome.verdict,
                Verdict::NotCleaned(Refusal::TooBig { size: s, limit }) if s == size && limit == TEXT_LIMIT
            ),
            "{:?}",
            outcome.verdict
        );
        assert_eq!(names(&scratch.0), ["big.txt"]);
    }

    /// A PNG's head and IHDR, then nothing.
    fn png_head() -> Vec<u8> {
        let mut head = b"\x89PNG\r\n\x1a\n".to_vec();
        head.extend_from_slice(&13u32.to_be_bytes());
        head.extend_from_slice(b"IHDR");
        head.extend_from_slice(&1u32.to_be_bytes());
        head.extend_from_slice(&1u32.to_be_bytes());
        head.extend_from_slice(&[8, 2, 0, 0, 0]);
        head.extend_from_slice(&[0x90, 0x77, 0x53, 0xDE]);
        head
    }

    /// A picture one byte past [`PICTURE_LIMIT`] is refused with its size
    /// on the `stat`, before the rest is read or a pixel decoded — a sparse
    /// file, so the test costs no 64 MiB of disk — and nothing is written.
    #[test]
    fn a_picture_past_the_limit_is_refused_before_it_is_decoded() {
        let scratch = Scratch::new("big-picture");
        let source = scratch.file("huge.png", &png_head());
        let size = PICTURE_LIMIT + 1;
        std::fs::OpenOptions::new()
            .write(true)
            .open(&source)
            .and_then(|file| file.set_len(size))
            .expect("sparse");
        let thing = arrival(Handed::Path(source));
        assert_eq!(
            cleanable(&thing.intake),
            Cleanable::Picture(Format::Png),
            "the head does not read as a PNG"
        );
        let outcome = clean(&thing, &Retention::default(), &scratch.homes());
        assert!(
            matches!(
                outcome.verdict,
                Verdict::NotCleaned(Refusal::TooBig { size: s, limit }) if s == size && limit == PICTURE_LIMIT
            ),
            "{:?}",
            outcome.verdict
        );
        assert!(outcome.report.is_none(), "a layer ran");
        assert!(matches!(
            inspect_one(&thing),
            Findings::NotLooked(Refusal::TooBig {
                limit: PICTURE_LIMIT,
                ..
            })
        ));
        assert_eq!(names(&scratch.0), ["huge.png"]);
    }

    /// The read side: a file the `stat` calls small that keeps coming — a
    /// pipe here, a file still being written in life — is refused once one
    /// byte past the limit has been read, and nothing past that is read.
    #[cfg(unix)]
    #[test]
    fn a_picture_that_grows_past_the_limit_while_read_is_refused() {
        use std::io::Write as _;
        let scratch = Scratch::new("growing");
        let pipe = scratch.0.join("growing.png");
        let made = std::process::Command::new("mkfifo")
            .arg(&pipe)
            .status()
            .expect("mkfifo");
        assert!(made.success());
        let writer = {
            let pipe = pipe.clone();
            std::thread::spawn(move || {
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .open(&pipe)
                    .expect("open the pipe");
                let _ = file.write_all(&png_head());
                let zeros = vec![0u8; 1 << 20];
                // Two megabytes past the limit; the reader hangs up first.
                for _ in 0..(PICTURE_LIMIT >> 20) + 2 {
                    if file.write_all(&zeros).is_err() {
                        break;
                    }
                }
            })
        };
        let thing = Arrival {
            intake: Intake {
                path: Some(pipe.clone()),
                ..intake(Some(Format::Png), None, Evidence::Content)
            },
            handed: Handed::Path(pipe),
        };
        let outcome = clean(&thing, &Retention::default(), &scratch.homes());
        writer.join().expect("writer");
        assert!(
            matches!(
                outcome.verdict,
                Verdict::NotCleaned(Refusal::TooBig { size, limit }) if size == PICTURE_LIMIT + 1 && limit == PICTURE_LIMIT
            ),
            "{:?}",
            outcome.verdict
        );
        assert_eq!(names(&scratch.0), ["growing.png"]);
    }

    /// A decode error is a refusal naming the byte, and nothing written.
    #[test]
    fn a_text_that_does_not_decode_is_refused_at_its_offset() {
        let scratch = Scratch::new("invalid");
        let mut bytes = MARKED.as_bytes().to_vec();
        bytes.extend_from_slice(&vec![b'a'; HEAD]);
        let at = bytes.len();
        bytes.extend_from_slice(b"\xff\xfe");
        let source = scratch.file("bad.md", &bytes);
        let outcome = clean(
            &arrival(Handed::Path(source)),
            &Retention::default(),
            &scratch.homes(),
        );
        assert!(
            matches!(
                outcome.verdict,
                Verdict::NotCleaned(Refusal::Undecodable { encoding: Encoding::Utf8, offset }) if offset == at
            ),
            "{:?}",
            outcome.verdict
        );
        assert_eq!(names(&scratch.0), ["bad.md"]);
    }

    /// A results folder that is not there is a failed write that says
    /// where and why — the operating system's sentence for the window, its
    /// kind for the log — and nothing is left behind.
    #[test]
    fn a_write_that_fails_says_where_and_why() {
        let scratch = Scratch::new("failed");
        let source = scratch.file("x.md", MARKED.as_bytes());
        let into = Retention {
            destination: Destination::Folder,
            folder: Some(scratch.0.join("not-there")),
            ..Retention::default()
        };
        let outcome = clean(
            &arrival(Handed::Path(source.clone())),
            &into,
            &scratch.homes(),
        );
        let Verdict::Failed(Failure::Write { path, error }) = &outcome.verdict else {
            panic!("{:?}", outcome.verdict);
        };
        assert_eq!(path, &scratch.0.join("not-there").join("x.cleaned.md"));
        assert_eq!(error.kind, std::io::ErrorKind::NotFound);
        assert!(!error.message.is_empty());
        assert_eq!(outcome.written, None);
        assert_eq!(names(&scratch.0), ["x.md"]);
        assert_eq!(read(&source), MARKED.as_bytes());
    }

    // -- the sweep -----------------------------------------------------

    #[test]
    fn the_sweep_removes_what_is_past_its_period_and_nothing_else() {
        let scratch = Scratch::new("sweep");
        let kept = scratch.0.join("kept");
        let old = kept.join("20260920T120000-3");
        let fresh = kept.join("20261004T120000-4");
        let foreign = kept.join("photos");
        let foreign_file = kept.join("20260101T000000-1.txt");
        for directory in [&old, &fresh, &foreign] {
            std::fs::create_dir_all(directory).expect("directory");
            std::fs::write(directory.join("original.md"), b"x").expect("file");
        }
        std::fs::write(&foreign_file, b"not ours").expect("file");

        // Forever removes nothing, however old.
        assert_eq!(sweep(&kept, Period::Forever, now()).expect("sweep"), 0);
        assert!(old.exists());

        assert_eq!(sweep(&kept, Period::Week, now()).expect("sweep"), 1);
        assert!(!old.exists(), "an expired copy stayed");
        assert!(fresh.exists(), "a fresh copy went");
        assert!(foreign.exists(), "a folder not shaped like ours went");
        assert!(foreign_file.exists());

        // A day: the fresh one is past it too.
        assert_eq!(sweep(&kept, Period::Day, now()).expect("sweep"), 1);
        assert!(!fresh.exists());
        // Nothing kept yet is not an error — and the sweep, which runs at
        // every launch, does not create the folder it found missing.
        let never = scratch.0.join("never");
        assert_eq!(sweep(&never, Period::Day, now()).expect("sweep"), 0);
        assert!(!never.exists(), "the sweep created the kept folder");
    }

    #[test]
    fn only_our_own_names_carry_a_stamp() {
        assert!(stamp_of("20261005T134602-7").is_some());
        for name in [
            "photos",
            "20261005T134602",
            "20261005T134602-",
            "20261005T134602-7a",
            "2026105T134602-7",
            "20261305T134602-7",
            "20261005T134602-7.txt",
        ] {
            assert_eq!(stamp_of(name), None, "{name}");
        }
    }

    // -- pictures ------------------------------------------------------

    fn clean_fixture(scratch: &Scratch, name: &str) -> (PathBuf, Outcome) {
        let file_name = Path::new(name).file_name().unwrap().to_string_lossy();
        let source = scratch.file(&file_name, &read(&fixture(name)));
        let outcome = clean(
            &arrival(Handed::Path(source.clone())),
            &Retention::default(),
            &scratch.homes(),
        );
        (source, outcome)
    }

    /// A real Gemini output: proved, restored, written beside; inspecting
    /// the result finds no visible mark.
    #[test]
    fn a_gemini_mark_comes_off_and_is_written_beside() {
        let scratch = Scratch::new("torch");
        let (source, outcome) = clean_fixture(&scratch, "gemini/torch-1025.png");
        assert!(
            matches!(outcome.verdict, Verdict::Cleaned),
            "{:?}",
            outcome.verdict
        );
        let beside = scratch.0.join("torch-1025.cleaned.png");
        assert_eq!(outcome.written.as_deref(), Some(beside.as_path()));
        assert_eq!(read(&source), read(&fixture("gemini/torch-1025.png")));
        let again =
            wipemark_picture::inspect(&read(&beside), &PictureOptions::default()).expect("inspect");
        assert!(!again.has_visible_mark(), "{:?}", again.visible);
    }

    /// The outline left on a flat background: written with what could be
    /// done, and said to be partly clean.
    #[test]
    fn a_mark_with_its_outline_left_is_written_and_said() {
        let scratch = Scratch::new("crying");
        let (_, outcome) = clean_fixture(&scratch, "gemini/crying-1025.png");
        assert!(
            matches!(outcome.verdict, Verdict::Partly(Left::Mark)),
            "{:?}",
            outcome.verdict
        );
        assert_eq!(
            outcome.written.as_deref(),
            Some(scratch.0.join("crying-1025.cleaned.png").as_path())
        );
    }

    /// The mark under alpha 0: seen, refused as transparent, nothing
    /// restored and no metadata removed — so the output is the input, and a
    /// copy of it is never written (D262), while the row says a mark is
    /// left.
    #[test]
    fn a_transparent_mark_is_left_and_nothing_identical_is_written() {
        let scratch = Scratch::new("transparent");
        let (_, outcome) = clean_fixture(&scratch, "gemini/crying-transparent-1025.png");
        assert!(
            matches!(outcome.verdict, Verdict::Partly(Left::Mark)),
            "{:?}",
            outcome.verdict
        );
        assert_eq!(outcome.written, None);
        assert_eq!(outcome.size_in, outcome.size_out);
        assert_eq!(names(&scratch.0), ["crying-transparent-1025.png"]);
    }

    #[test]
    fn a_picture_with_nothing_on_it_writes_nothing() {
        let scratch = Scratch::new("confetti");
        let (_, outcome) = clean_fixture(&scratch, "gemini/cut-out-confetti-256.webp");
        assert!(
            matches!(outcome.verdict, Verdict::NothingFound),
            "{:?}",
            outcome.verdict
        );
        assert_eq!(names(&scratch.0), ["cut-out-confetti-256.webp"]);
    }

    /// AI provenance metadata goes, and a second inspection of the result
    /// finds none.
    #[test]
    fn ai_metadata_is_removed_and_gone_on_reinspection() {
        let scratch = Scratch::new("metadata");
        for (name, written) in [
            ("c2pa-jumbf.jpg", "c2pa-jumbf.cleaned.jpg"),
            ("xmp-provenance-url.png", "xmp-provenance-url.cleaned.png"),
        ] {
            let (_, outcome) = clean_fixture(&scratch, name);
            assert!(
                matches!(outcome.verdict, Verdict::Cleaned),
                "{name}: {:?}",
                outcome.verdict
            );
            let result = scratch.0.join(written);
            assert_eq!(outcome.written.as_deref(), Some(result.as_path()));
            let again = wipemark_image::inspect(&read(&result)).expect("inspect");
            assert!(!again.has_ai_metadata(), "{name}: {:?}", again.findings);
            assert!(!again.has_c2pa(), "{name}");
        }
    }

    /// A window cleans a picture of AI provenance and nothing else: on a
    /// JPEG whose XMP is a C2PA reference beside camera EXIF, IPTC and an
    /// ICC profile, the XMP goes and the other three are there on a second
    /// inspection — and the bytes are what `wipemark-cli clean` writes
    /// without `--all-metadata` (`all_metadata: false` is
    /// `Scope::AiProvenance` there).
    #[test]
    fn a_picture_loses_its_ai_provenance_and_keeps_the_rest() {
        use wipemark_image::MetadataKind;
        let scratch = Scratch::new("scope");
        let (source, outcome) = clean_fixture(&scratch, "xmp-provenance.jpg");
        assert!(
            matches!(outcome.verdict, Verdict::Cleaned),
            "{:?}",
            outcome.verdict
        );
        let kinds = |bytes: &[u8]| -> Vec<MetadataKind> {
            wipemark_image::inspect(bytes)
                .expect("inspect")
                .findings
                .iter()
                .map(|finding| finding.kind)
                .collect()
        };
        let before = kinds(&read(&source));
        for kind in [
            MetadataKind::Xmp,
            MetadataKind::Exif,
            MetadataKind::Iptc,
            MetadataKind::Rendering,
        ] {
            assert!(before.contains(&kind), "the fixture has no {kind:?}");
        }
        let result = read(&scratch.0.join("xmp-provenance.cleaned.jpg"));
        let after = kinds(&result);
        assert!(!after.contains(&MetadataKind::Xmp), "the XMP stayed");
        for kind in [
            MetadataKind::Exif,
            MetadataKind::Iptc,
            MetadataKind::Rendering,
        ] {
            assert!(after.contains(&kind), "{kind:?} went: {after:?}");
        }
        let cli_default = PictureOptions {
            scope: Scope::AiProvenance,
            catalogue: None,
        };
        let (expected, _) =
            wipemark_picture::clean(&read(&source), &cli_default).expect("the CLI's clean");
        assert_eq!(result, expected, "not what the CLI writes at its default");
    }

    /// A picture whose only provenance is a C2PA manifest — a manifest
    /// store in APP11 beside camera EXIF, no XMP — is AI metadata to the
    /// look and cleaned by the clean. `wipemark_image` counts every C2PA
    /// block as AI provenance, which is why the look needs no clause of its
    /// own for one (D281); this pins that on the real file.
    #[test]
    fn a_picture_marked_by_c2pa_alone_is_looked_at_and_cleaned_alike() {
        use wipemark_image::MetadataKind;
        let scratch = Scratch::new("c2pa-alone");
        let bytes = read(&fixture("c2pa-jumbf.jpg"));
        let seen = wipemark_image::inspect(&bytes).expect("inspect");
        let provenance: Vec<_> = seen
            .findings
            .iter()
            .filter(|finding| finding.is_ai_provenance())
            .collect();
        assert!(!provenance.is_empty() && seen.has_c2pa());
        assert!(
            provenance.iter().all(|finding| finding.is_c2pa()),
            "the fixture carries provenance other than C2PA: {provenance:?}"
        );
        assert!(
            !seen.findings.iter().any(|f| f.kind == MetadataKind::Xmp),
            "the fixture carries XMP"
        );
        let source = scratch.file("c2pa-jumbf.jpg", &bytes);
        let thing = arrival(Handed::Path(source));
        assert!(
            matches!(
                inspect_one(&thing),
                Findings::Picture {
                    ai_metadata: true,
                    mark: false,
                    not_examined: None
                }
            ),
            "{:?}",
            inspect_one(&thing)
        );
        let outcome = clean(&thing, &Retention::default(), &scratch.homes());
        assert!(
            matches!(outcome.verdict, Verdict::Cleaned),
            "{:?}",
            outcome.verdict
        );
    }

    /// The panel's look says what the clean then does (D278): a text by the
    /// characters it changes — counted, not rows — a picture by what is on
    /// it, and nothing written by the look.
    #[test]
    fn the_look_agrees_with_the_clean() {
        let scratch = Scratch::new("look");
        let homes = scratch.homes();
        for (name, text) in [
            ("two.md", "A zero\u{200B}width\u{200B} space.\n"),
            ("plain.md", "Nothing here.\n"),
            ("kept.md", "p\u{0430}ypal account\n"),
        ] {
            let source = scratch.file(name, text.as_bytes());
            let thing = arrival(Handed::Path(source.clone()));
            let found = inspect_one(&thing);
            assert_eq!(read(&source), text.as_bytes(), "{name}: the look wrote");
            let outcome = clean(&thing, &Retention::default(), &homes);
            let removed = match &outcome.report {
                Some(Report::Text(report)) => report
                    .removed
                    .iter()
                    .map(|(_, n)| *n as usize)
                    .chain(report.normalized.iter().map(|(_, n)| *n as usize))
                    .sum::<usize>(),
                other => panic!("{name}: {other:?}"),
            };
            match (&found, &outcome.verdict) {
                (Findings::Text { change, .. }, Verdict::Cleaned) => {
                    assert_eq!(*change, removed, "{name}");
                    assert_eq!(*change, 2, "{name}: the count is of characters");
                }
                (
                    Findings::Text {
                        change: 0,
                        suspicious: false,
                    },
                    Verdict::NothingFound,
                ) => {}
                (
                    Findings::Text {
                        change: 0,
                        suspicious: true,
                    },
                    Verdict::Partly(Left::Kept),
                ) => {}
                (found, verdict) => panic!("{name}: looked {found:?}, cleaned {verdict:?}"),
            }
        }

        for (name, ai, mark) in [
            ("gemini/torch-1025.png", false, true),
            ("c2pa-jumbf.jpg", true, false),
            ("gemini/cut-out-confetti-256.webp", false, false),
        ] {
            let file_name = Path::new(name).file_name().unwrap().to_string_lossy();
            let source = scratch.file(&file_name, &read(&fixture(name)));
            let found = inspect_one(&arrival(Handed::Path(source)));
            assert!(
                matches!(
                    found,
                    Findings::Picture { ai_metadata, mark: seen, not_examined: None }
                        if ai_metadata == ai && seen == mark
                ),
                "{name}: {found:?}"
            );
            let (_, outcome) = clean_fixture(&scratch, name);
            assert_eq!(
                matches!(outcome.verdict, Verdict::NothingFound),
                !ai && !mark,
                "{name}: {:?}",
                outcome.verdict
            );
        }

        let tiff = scratch.file("scan.tif", b"II*\x00\x08\x00\x00\x00\x00\x00\x00\x00");
        assert!(matches!(
            inspect_one(&arrival(Handed::Path(tiff))),
            Findings::NotLooked(Refusal::NotCleanable(Unable::NotYet(Format::Tiff)))
        ));
    }

    // -- the log line --------------------------------------------------

    /// Every event logged on this thread while `body` runs, one line each:
    /// the message and its fields as `name=value`.
    fn logged_while(body: impl FnOnce()) -> Vec<String> {
        use std::fmt::Write as _;
        use std::sync::{Arc, Mutex};

        use tracing::field::{Field, Visit};
        use tracing::span;

        struct Line(String);
        impl Visit for Line {
            fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
                let _ = write!(self.0, " {}={value:?}", field.name());
            }
        }

        struct Capture(Arc<Mutex<Vec<String>>>);
        impl tracing::Subscriber for Capture {
            fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
                true
            }
            fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
                span::Id::from_u64(1)
            }
            fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
            fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
            fn event(&self, event: &tracing::Event<'_>) {
                let mut line = Line(String::new());
                event.record(&mut line);
                self.0.lock().expect("lines").push(line.0);
            }
            fn enter(&self, _: &span::Id) {}
            fn exit(&self, _: &span::Id) {}
        }

        // With exactly one dispatcher alive, `tracing-core` computes a
        // callsite's interest from the *registering thread's* default: a
        // parallel test that reaches `logged` first, with no subscriber of
        // its own, caches "never" and this capture sees nothing. A second
        // dispatcher, interested in everything and kept for the life of the
        // test binary, keeps every callsite's interest open.
        static OPEN: std::sync::OnceLock<tracing::Dispatch> = std::sync::OnceLock::new();
        OPEN.get_or_init(|| tracing::Dispatch::new(Capture(Arc::default())));

        let lines = Arc::new(Mutex::new(Vec::new()));
        tracing::subscriber::with_default(Capture(Arc::clone(&lines)), body);
        let lines = lines.lock().expect("lines").clone();
        lines
    }

    /// The clean's log line names no path, no file and no text — beside a
    /// file in a folder whose names are distinctive, on a refusal that
    /// names the source, and on a paste that is kept — and renders a
    /// path's shape as the person debugging reads it.
    #[test]
    fn the_log_line_carries_no_path_no_name_and_no_text() {
        let scratch = Scratch::new("log");
        let folder = scratch.0.join("Quarterly-Qx7Folder");
        std::fs::create_dir_all(&folder).expect("folder");
        let text = "Zz9Confidential\u{200B}Phrase\n";
        let source = folder.join("Payroll-Kv3Name.md");
        std::fs::write(&source, text).expect("source");
        let homes = Homes {
            results: folder.clone(),
            kept: folder.join("kept"),
        };
        let keeping = Retention {
            keep_originals: true,
            keep_results: true,
            ..Retention::default()
        };
        let lines = logged_while(|| {
            let dropped = arrival(Handed::Path(source.clone()));
            let beside = clean(&dropped, &Retention::default(), &homes);
            assert!(beside.written.is_some(), "{:?}", beside.verdict);
            let onto_source = Plan::File(Written::Into {
                folder: folder.clone(),
                name: Some(String::from("Payroll-Kv3Name.md")),
            });
            let refused = clean_one(&dropped, &onto_source, 7, now());
            assert!(
                matches!(refused.verdict, Verdict::NotCleaned(Refusal::SameFile(_))),
                "{:?}",
                refused.verdict
            );
            let pasted = clean(&arrival(Handed::Text(text.to_owned())), &keeping, &homes);
            assert!(pasted.kept.is_some(), "{:?}", pasted.verdict);
        });
        let cleans: Vec<&String> = lines
            .iter()
            .filter(|line| line.contains("message=clean "))
            .collect();
        assert_eq!(cleans.len(), 3, "{lines:#?}");
        let scratch_name = scratch
            .0
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        for line in &lines {
            for secret in [
                "Qx7Folder",
                "Kv3Name",
                "Zz9Confidential",
                "Phrase",
                scratch_name.as_str(),
                "kept",
                ".md",
            ] {
                // `kept=` is a field's name; the path under it is not.
                let body = line.replace(" kept=", " ");
                assert!(
                    !body.contains(secret),
                    "{secret:?} is in a log line: {line}"
                );
            }
        }
        assert!(
            cleans[0].contains("written=<elided chars="),
            "a path's shape is said as its Display: {}",
            cleans[0]
        );
        assert!(
            cleans[1].contains("same file: <elided chars="),
            "{}",
            cleans[1]
        );
        assert!(cleans[2].contains(" kept=<elided chars="), "{}", cleans[2]);
    }

    /// Two cleans in one run never share a number, whichever window asked
    /// (D280).
    #[test]
    fn a_number_is_handed_out_once() {
        let numbers: Vec<u64> = (0..64).map(|_| number()).collect();
        let mut unique = numbers.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), numbers.len());
    }

    /// TIFF is refused by name, before a byte past the head is read.
    #[test]
    fn a_tiff_is_refused_by_name() {
        let scratch = Scratch::new("tiff");
        let source = scratch.file("scan.tif", b"II*\x00\x08\x00\x00\x00\x00\x00\x00\x00");
        let outcome = clean(
            &arrival(Handed::Path(source)),
            &Retention::default(),
            &scratch.homes(),
        );
        assert!(
            matches!(
                outcome.verdict,
                Verdict::NotCleaned(Refusal::NotCleanable(Unable::NotYet(Format::Tiff)))
            ),
            "{:?}",
            outcome.verdict
        );
        assert_eq!(names(&scratch.0), ["scan.tif"]);
    }

    #[test]
    fn a_truncated_png_is_refused_as_malformed() {
        let scratch = Scratch::new("truncated");
        let whole = read(&fixture("xmp-provenance-url.png"));
        let source = scratch.file("cut.png", &whole[..whole.len() / 2]);
        let outcome = clean(
            &arrival(Handed::Path(source)),
            &Retention::default(),
            &scratch.homes(),
        );
        assert!(
            matches!(
                outcome.verdict,
                Verdict::NotCleaned(Refusal::Picture(PictureError::Image(
                    ImageError::Malformed { .. }
                )))
            ),
            "{:?}",
            outcome.verdict
        );
        assert_eq!(names(&scratch.0), ["cut.png"]);
    }

    // -- outcome_of, row by row ----------------------------------------

    fn picture_report(
        removed_ai: bool,
        still_ai: bool,
        still_c2pa: bool,
        visible: Visible,
    ) -> PictureReport {
        let removed = if removed_ai {
            // A real AI block, from the fixture's own inspection.
            wipemark_image::inspect(&read(&fixture("c2pa-jumbf.jpg")))
                .expect("inspect")
                .findings
                .into_iter()
                .filter(MetadataFinding::is_ai_provenance)
                .collect()
        } else {
            Vec::new()
        };
        PictureReport {
            container: wipemark_image::ImageContainer::Jpeg,
            metadata: wipemark_image::StripReport {
                container: wipemark_image::ImageContainer::Jpeg,
                removed,
                kept: Vec::new(),
                still_has_c2pa: still_c2pa,
                still_has_ai_metadata: still_ai,
                orientation_removed: None,
                not_established: Vec::new(),
            },
            visible,
            encoding: wipemark_picture::Encoding::Unchanged,
        }
    }

    fn examined_nothing() -> Visible {
        Visible::Examined {
            report: wipemark_pixels::PixelReport {
                found: Vec::new(),
                restored: Vec::new(),
                dismissed: 0,
                not_established: wipemark_pixels::not_established::shelf(),
            },
            restorable: true,
        }
    }

    fn decide(report: &PictureReport, changed: bool) -> (Verdict, bool) {
        let output: &[u8] = if changed { b"out" } else { b"in" };
        outcome_of(&Cleaning::Picture {
            input: b"in",
            output,
            report,
        })
    }

    /// The CLI's `clean_exit` rows for a picture, one by one.
    #[test]
    fn a_picture_comes_to_what_the_cli_says_it_does() {
        // Nothing on it: exit 0, and nothing written.
        let (verdict, writes) = decide(
            &picture_report(false, false, false, examined_nothing()),
            false,
        );
        assert!(
            matches!(verdict, Verdict::NothingFound) && !writes,
            "{verdict:?}"
        );

        // AI metadata removed: exit 1, written.
        let (verdict, writes) = decide(
            &picture_report(true, false, false, examined_nothing()),
            true,
        );
        assert!(matches!(verdict, Verdict::Cleaned) && writes, "{verdict:?}");

        // Pixels not examined: exit 3, written with what could be done.
        let (verdict, writes) = decide(
            &picture_report(
                true,
                false,
                false,
                Visible::NotExamined(NotExamined::Animated),
            ),
            true,
        );
        assert!(
            matches!(
                verdict,
                Verdict::Partly(Left::NotExamined(NotExamined::Animated))
            ) && writes,
            "{verdict:?}"
        );
        // …and an identical output is still never written.
        let (_, writes) = decide(
            &picture_report(
                false,
                false,
                false,
                Visible::NotExamined(NotExamined::Decode),
            ),
            false,
        );
        assert!(!writes);
    }

    /// The row that matters most: an output that still carries AI
    /// provenance is not cleaned and **not written**, whatever else
    /// happened to it.
    #[test]
    fn a_picture_still_marked_is_never_written() {
        for (ai, c2pa) in [(true, false), (false, true), (true, true)] {
            let report = picture_report(true, ai, c2pa, examined_nothing());
            let (verdict, writes) = decide(&report, true);
            assert!(
                matches!(
                    verdict,
                    Verdict::NotCleaned(Refusal::StillMarked { ai_metadata, c2pa: c })
                        if ai_metadata == ai && c == c2pa
                ),
                "{verdict:?}"
            );
            assert!(!writes, "a still-marked picture would be written");
        }
    }

    /// The rows for a text: changed is cleaned and written, unchanged and
    /// unsuspicious is nothing found, unchanged and suspicious is a kept
    /// finding — partly, and nothing written (D263).
    #[test]
    fn a_text_comes_to_what_the_cli_says_it_does() {
        let decide = |input: &str| {
            let cleaned = wipemark_core::clean(input, &Options::default());
            outcome_of(&Cleaning::Text {
                input,
                cleaned: &cleaned,
            })
        };
        let (verdict, writes) = decide("a\u{200B}b");
        assert!(matches!(verdict, Verdict::Cleaned) && writes, "{verdict:?}");
        let (verdict, writes) = decide("plain words");
        assert!(
            matches!(verdict, Verdict::NothingFound) && !writes,
            "{verdict:?}"
        );
        // A soft hyphen alone is not suspicious, and its removal is a
        // change the CLI writes.
        let (verdict, writes) = decide("soft\u{00AD}hyphen");
        assert!(matches!(verdict, Verdict::Cleaned) && writes, "{verdict:?}");
        // A Cyrillic `а` among Latin letters: a homoglyph, kept at the
        // defaults — found, and nothing to write.
        let input = "p\u{0430}ypal account";
        let cleaned = wipemark_core::clean(input, &Options::default());
        assert!(cleaned.report.suspicious, "{:?}", cleaned.report);
        assert_eq!(cleaned.text, input);
        let (verdict, writes) = decide(input);
        assert!(
            matches!(verdict, Verdict::Partly(Left::Kept)) && !writes,
            "{verdict:?}"
        );
    }

    /// The report's JSON is the library's own, third shelf and all.
    #[test]
    fn the_report_is_the_librarys_own_json() {
        let cleaned = wipemark_core::clean("a\u{200B}b", &Options::default());
        let report = Report::Text(cleaned.report.clone());
        assert_eq!(report.to_json(), cleaned.report.to_json());
        assert!(report.to_json().contains("\"not_established\":[\""));
    }
}
