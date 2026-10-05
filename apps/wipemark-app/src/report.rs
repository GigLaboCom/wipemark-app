//! The Report dialog: one finished clean, said in full — what arrived,
//! what happened, and the three shelves every report in this product
//! carries.
//!
//! # The shelves, in a window
//!
//! *Verifiable* is what a check in this build established and anyone can
//! check again: for a text, what Layer A removed, by character, class,
//! confidence and count — Layer A is deterministic, and every one of them
//! is gone from the result (`docs/architecture/layer-a.md`); for a
//! picture, the metadata blocks removed and the library's **second
//! inspection** of the result, and the visible marks that were proved and
//! restored. *Best-effort* is everything that makes a result less than
//! that: a text's kept characters and why they were kept; a picture's
//! restoration notes — fitted, clamped, found by the search, resampled,
//! the residual as a **mean** (D248) — an outline or a texture left,
//! proposals refused with their reason, the pixels not examined, the
//! metadata kept and how the picture was written back. *Not established*
//! is the report's own list, one line per id in its order — a picture's
//! starts with `invisible-pixel-marks` — and is **never empty**: an id this
//! build has no sentence for is shown as the canonical English beside it,
//! never dropped. A shelf with nothing on it for this clean says so rather
//! than vanishing.
//!
//! [`sheet`] builds all of it as values over a [`Say`], so the window
//! draws it in `Rendering::Ui` and **Copy as Markdown** renders the same
//! sheet through [`wording::plain`] — no U+2068/U+2069, nothing Layer A
//! would remove, because a copied report is not a window. **Copy JSON** is
//! the library's own `to_json()`, untouched: a format, never translated.
//!
//! # The dialog
//!
//! An element in the main window's own tree, painted by the shell over
//! everything, for every reason `dialog` gives for not going through
//! `Root`. Escape, the backdrop and Close are one answer.

use gpui::prelude::*;
use gpui::{
    div, px, App, ClickEvent, ClipboardItem, Context, EventEmitter, FocusHandle, Focusable,
    SharedString, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::{h_flex, v_flex, ActiveTheme, Disableable as _, Sizable as _, StyledExt};
use wipemark_core::report::not_established;
use wipemark_core::{CleanReport, Confidence, UnicodeClass, UnicodeFinding};
use wipemark_i18n::{args, FluentArgs, Message};
use wipemark_image::{MetadataFinding, MetadataKind, Signal};
use wipemark_intake::{Intake, Kind};
use wipemark_picture::{Encoding, NotExamined, PictureReport, Visible};
use wipemark_pixels::{
    Finding, Placed, Refusal as PixelRefusal, Restored, Verdict as PixelVerdict,
};

use crate::clean::{Outcome, Report};
use crate::dialog::{self, Answer, Dismiss, Next, Previous};
use crate::wording::{self, Say};

/// One line of a shelf, and how far it is indented under the one before.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub depth: usize,
    pub text: String,
}

impl Line {
    fn top(text: String) -> Self {
        Self { depth: 0, text }
    }

    fn under(text: String) -> Self {
        Self { depth: 1, text }
    }
}

/// A finished clean, said in full. Every section has at least one line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sheet {
    pub title: String,
    pub arrived: Vec<Line>,
    pub happened: Vec<Line>,
    pub verifiable: Vec<Line>,
    pub best_effort: Vec<Line>,
    pub not_established: Vec<Line>,
}

impl Sheet {
    /// The sections, titled, in the order they are shown.
    pub fn sections(&self) -> [(Message, &[Line]); 5] {
        [
            (Message::WindowReportArrived, &self.arrived),
            (Message::WindowReportHappened, &self.happened),
            (Message::WindowReportVerifiable, &self.verifiable),
            (Message::WindowReportBestEffort, &self.best_effort),
            (Message::ReportNotEstablishedTitle, &self.not_established),
        ]
    }
}

fn none() -> FluentArgs<'static> {
    FluentArgs::new()
}

/// Build the sheet for one clean of one thing.
pub fn sheet(say: Say, intake: &Intake, outcome: &Outcome) -> Sheet {
    let title = wording::title_of_in(say, intake);

    let mut what = vec![wording::kind_label_in(say, intake.kind)];
    what.extend(
        outcome
            .format
            .or(intake.format)
            .map(|format| format.name().to_owned()),
    );
    what.extend(intake.encoding.map(|encoding| encoding.name().to_owned()));
    let mut arrived = vec![Line::top(title.clone()), Line::top(what.join(" · "))];
    arrived.extend(wording::evidence_note_in(say, intake).map(|(note, _)| Line::top(note)));

    let mut happened = vec![Line::top(wording::said_in(say, outcome))];
    if let Some(written) = &outcome.written {
        happened.push(Line::top(say(
            Message::WindowReportResult,
            &args!("path" => written.display().to_string()),
        )));
    }
    if let Some(original) = &outcome.set_aside {
        happened.push(Line::top(say(
            Message::WindowReportOriginal,
            &args!("path" => original.display().to_string()),
        )));
    }
    if let Some(kept) = &outcome.kept {
        happened.push(Line::top(say(
            Message::WindowReportKept,
            &args!("path" => kept.display().to_string()),
        )));
    }
    if outcome.written.is_none() && outcome.text.is_none() {
        happened.extend(wording::went_in(say, outcome).into_iter().map(Line::top));
    } else if outcome.text.is_some() {
        happened.push(Line::top(say(Message::QueueWentAsText, &none())));
    }

    let (mut verifiable, mut best_effort) = match &outcome.report {
        Some(Report::Text(report)) => text_shelves(say, report),
        Some(Report::Picture(report)) => picture_shelves(say, report),
        None => (
            vec![Line::top(say(Message::WindowReportNotRead, &none()))],
            Vec::new(),
        ),
    };
    for shelf in [&mut verifiable, &mut best_effort] {
        if shelf.is_empty() {
            shelf.push(Line::top(say(Message::WindowReportShelfEmpty, &none())));
        }
    }

    let not_established = shelf_ids(intake, outcome)
        .into_iter()
        .map(|id| Line::top(shelf_line(say, id)))
        .collect();

    Sheet {
        title,
        arrived,
        happened,
        verifiable,
        best_effort,
        not_established,
    }
}

/// The report's own third shelf, as ids, in its order.
///
/// A picture's is read off its report: the pixel pass's shelf, which leads
/// with `invisible-pixel-marks` — or that claim alone, when the pixels were
/// not examined and the pass has no report — then every id the metadata
/// pass carries that is not already there. A text's `CleanReport` carries
/// no shelf of its own: its JSON writes core's `not_established::ALL`, and
/// so does this. A thing never read has no report at all, and is given the
/// shelf of what it arrived as (D277).
pub fn shelf_ids(intake: &Intake, outcome: &Outcome) -> Vec<&'static str> {
    let core = || not_established::ALL.iter().map(|(id, _)| *id);
    match &outcome.report {
        Some(Report::Picture(picture)) => {
            let mut ids = match &picture.visible {
                Visible::Examined { report, .. } => report.not_established.clone(),
                Visible::NotExamined(_) => vec![wipemark_pixels::not_established::ID],
            };
            for id in &picture.metadata.not_established {
                if !ids.contains(id) {
                    ids.push(*id);
                }
            }
            ids
        }
        Some(Report::Text(_)) => core().collect(),
        None if intake.kind == Kind::Image => wipemark_pixels::not_established::shelf(),
        None => core().collect(),
    }
}

/// One entry of the third shelf in the reader's language — or, for an id
/// this build has no sentence for, its canonical English beside it, or the
/// id alone when nothing here knows it: an entry is never dropped.
fn shelf_line(say: Say, id: &str) -> String {
    let message = match id {
        "vendor-detector-evasion" => Message::ReportNotEstablishedVendorDetectorEvasion,
        "human-authorship" => Message::ReportNotEstablishedHumanAuthorship,
        "unknown-mark-schemes" => Message::ReportNotEstablishedUnknownMarkSchemes,
        "invisible-pixel-marks" => Message::ReportNotEstablishedInvisiblePixelMarks,
        _ => {
            let canonical = not_established::ALL
                .iter()
                .find(|(known, _)| *known == id)
                .map(|(_, canonical)| *canonical)
                .or((id == wipemark_pixels::not_established::ID)
                    .then_some(wipemark_pixels::not_established::INVISIBLE_PIXEL_MARKS));
            return match canonical {
                Some(canonical) => format!("{canonical} ({id})"),
                None => id.to_owned(),
            };
        }
    };
    say(message, &none())
}

/// A text: what Layer A removed (verifiable), what it kept and why
/// (best-effort).
fn text_shelves(say: Say, report: &CleanReport) -> (Vec<Line>, Vec<Line>) {
    let mut verifiable: Vec<Line> = report
        .findings
        .iter()
        .map(|finding| Line::top(finding_line(say, finding)))
        .collect();
    if verifiable.is_empty() {
        verifiable.push(Line::top(say(Message::WindowReportRemovedNone, &none())));
    }
    let normalized: u32 = report.normalized.iter().map(|(_, count)| count).sum();
    if normalized > 0 {
        verifiable.push(Line::top(say(
            Message::WindowReportNormalized,
            &args!("count" => normalized),
        )));
    }
    verifiable.push(Line::top(say(
        Message::CliReportUnicode,
        &args!("version" => report.unicode_version),
    )));

    let mut best_effort = Vec::new();
    for finding in &report.kept {
        best_effort.push(Line::top(finding_line(say, finding)));
        best_effort.push(Line::under(say(
            if finding.class == UnicodeClass::Homoglyph {
                Message::WindowReportKeptHomoglyph
            } else {
                Message::WindowReportKeptInPlace
            },
            &none(),
        )));
    }
    (verifiable, best_effort)
}

/// "U+200B ZERO WIDTH SPACE · zero-width character · confirmed · 3 times".
/// The name is the standard's and never translated.
fn finding_line(say: Say, finding: &UnicodeFinding) -> String {
    let name = wipemark_core::name_of(finding.codepoint)
        .map(|name| name.into_owned())
        .unwrap_or_default();
    say(
        Message::WindowReportFinding,
        &args!(
            "codepoint" => format!("U+{:04X}", u32::from(finding.codepoint)),
            "name" => name,
            "class" => say(class_label(finding.class), &none()),
            "confidence" => say(confidence_label(finding.confidence), &none()),
            "count" => finding.count,
        ),
    )
}

/// A picture: the metadata removed and the second inspection, and the
/// marks proved and restored (verifiable); everything that makes the
/// result less than exact, what was refused and what was kept
/// (best-effort).
fn picture_shelves(say: Say, picture: &PictureReport) -> (Vec<Line>, Vec<Line>) {
    let metadata = &picture.metadata;
    let mut verifiable = Vec::new();
    let mut best_effort = Vec::new();

    for finding in &metadata.removed {
        verifiable.extend(block(say, finding));
    }
    if !metadata.removed.is_empty() {
        verifiable.push(Line::top(say(
            if metadata.still_has_ai_metadata || metadata.still_has_c2pa {
                Message::WindowReportPictureStill
            } else {
                Message::WindowReportPictureChecked
            },
            &none(),
        )));
    }

    match &picture.visible {
        Visible::NotExamined(why) => best_effort.push(Line::top(say(
            match why {
                NotExamined::Animated => Message::CliImageVisibleNotExaminedAnimated,
                NotExamined::Catalogue => Message::CliImageVisibleNotExaminedCatalogue,
                NotExamined::Decode => Message::CliImageVisibleNotExaminedDecode,
            },
            &none(),
        ))),
        Visible::Examined { report, restorable } => {
            if report.found.is_empty() {
                best_effort.push(Line::top(say(Message::CliImageVisibleNone, &none())));
            }
            for finding in &report.found {
                match &finding.verdict {
                    PixelVerdict::Verified(proved) => {
                        verifiable.push(Line::top(mark_row(say, finding)));
                        verifiable.push(Line::under(say(
                            Message::CliImageVisibleProved,
                            &args!(
                                "ncc" => fixed(finding.ncc, 3),
                                "gain" => fixed(proved.gain(), 2),
                                "ratio" => fixed(proved.edge_ratio(), 3),
                            ),
                        )));
                    }
                    PixelVerdict::Refused(refusal) => {
                        best_effort.push(Line::top(mark_row(say, finding)));
                        best_effort.push(Line::under(say(
                            Message::CliImageVisibleRefused,
                            &args!("reason" => refusal_line(say, *refusal)),
                        )));
                    }
                }
            }
            if !restorable && !report.found.is_empty() {
                best_effort.push(Line::top(say(
                    Message::CliImageVisibleNotRestorable,
                    &none(),
                )));
            }
            for restored in &report.restored {
                let line = say(
                    Message::CliImageVisibleRestored,
                    &args!(
                        "profile" => restored.profile.as_str(),
                        "changed" => restored.changed.to_string(),
                    ),
                );
                if restored.exact {
                    verifiable.push(Line::top(line));
                    verifiable.push(Line::under(say(Message::CliImageVisibleExact, &none())));
                } else {
                    best_effort.push(Line::top(line));
                }
                best_effort.extend(inexact(say, restored).into_iter().map(Line::under));
            }
        }
    }
    if picture.marks_left() {
        best_effort.push(Line::top(say(Message::CliImageVisibleLeft, &none())));
    }

    for finding in &metadata.kept {
        best_effort.extend(block(say, finding));
    }
    if metadata
        .kept
        .iter()
        .any(|finding| finding.kind == MetadataKind::Rendering)
    {
        best_effort.push(Line::top(say(Message::CliImageRendering, &none())));
    }
    if metadata
        .removed
        .iter()
        .any(|finding| finding.kind == MetadataKind::Exif)
    {
        best_effort.push(Line::top(say(Message::CliImageExifRemoved, &none())));
    }
    if metadata.orientation_removed.is_some() {
        best_effort.push(Line::top(say(Message::CliImageOrientationRemoved, &none())));
    }
    best_effort.extend(
        encoding_lines(say, picture.encoding)
            .into_iter()
            .map(Line::top),
    );
    (verifiable, best_effort)
}

/// Why a restoration is not exact — each reason that holds, by name, and
/// the residual as a mean, never a bound (D248); holes, an outline and a
/// texture left.
fn inexact(say: Say, restored: &Restored) -> Vec<String> {
    let mut notes = Vec::new();
    if !restored.exact {
        if restored.lossy {
            notes.push(say(Message::CliImageVisibleInexact, &none()));
        }
        if restored.clamped > 0 {
            notes.push(say(
                Message::CliImageVisibleClamped,
                &args!("clamped" => restored.clamped),
            ));
        }
        if restored.fitted {
            notes.push(say(Message::CliImageVisibleFitted, &none()));
        }
        if restored.resampled {
            notes.push(say(Message::CliImageVisibleResampled, &none()));
        }
        if restored.searched {
            notes.push(say(Message::CliImageVisibleSearched, &none()));
        }
        if !restored.outline_left {
            notes.push(say(
                Message::CliImageVisibleResidual,
                &args!("levels" => fixed(farthest(restored), 1)),
            ));
        }
    }
    if restored.holes > 0 {
        notes.push(say(
            Message::CliImageVisibleHoles,
            &args!("holes" => restored.holes.to_string()),
        ));
    }
    if restored.outline_left {
        notes.push(say(
            Message::CliImageVisibleOutline,
            &args!(
                "levels" => fixed(farthest(restored), 1),
                "share" => fixed(restored.outline * 100.0, 0),
            ),
        ));
    }
    if restored.texture_left {
        notes.push(say(
            Message::CliImageVisibleTexture,
            &args!(
                "levels" => fixed(restored.texture, 1),
                "around" => fixed(restored.texture_around, 1),
            ),
        ));
    }
    notes
}

/// How far the restored mark's faint band lies from the picture around
/// it: the mean step of the channel farthest from it (D247, D248).
fn farthest(restored: &Restored) -> f32 {
    restored.steps.iter().fold(0f32, |m, s| m.max(s.abs()))
}

/// A number with a fixed number of decimals, in the language's separator.
fn fixed(value: f32, decimals: usize) -> String {
    wipemark_i18n::decimal(f64::from(value), decimals)
}

/// One metadata block and, under it, every signal that made it
/// provenance. Names inside a file are spelled, never shown raw.
fn block(say: Say, finding: &MetadataFinding) -> Vec<Line> {
    let chunk = wipemark_image::spell(&finding.chunk);
    let place = match &finding.key {
        Some(key) => format!("{chunk} {}", wipemark_image::spell(key)),
        None => chunk,
    };
    let mut lines = vec![Line::top(say(
        Message::CliImageRow,
        &args!(
            "where" => place,
            "kind" => say(metadata_label(finding.kind), &none()),
            "offset" => finding.offset.to_string(),
            "size" => finding.len.to_string(),
            "length" => finding.len,
        ),
    ))];
    for evidence in &finding.evidence {
        let signal = say(signal_label(evidence.signal), &none());
        let field = wipemark_image::spell(&evidence.field);
        let matched = wipemark_image::spell(evidence.matched);
        lines.push(Line::under(match evidence.signal {
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
        }));
    }
    lines
}

/// A visible mark: which profile, where, how it was placed. Profile,
/// vendor and product are identifiers, interpolated and never translated.
fn mark_row(say: Say, finding: &Finding) -> String {
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
    say(
        Message::CliImageVisibleRow,
        &args!(
            "profile" => finding.profile.as_str(),
            "vendor" => finding.vendor.as_str(),
            "product" => finding.product.as_str(),
            "width" => rect.width.to_string(),
            "height" => rect.height.to_string(),
            "x" => rect.x.to_string(),
            "y" => rect.y.to_string(),
            "placed" => say(placed, &none()),
        ),
    )
}

/// A refused proposal's reason, with the number that failed.
fn refusal_line(say: Say, refusal: PixelRefusal) -> String {
    match refusal {
        PixelRefusal::Transparent => say(Message::CliImageRefusalTransparent, &none()),
        PixelRefusal::Opaque { holes } => say(
            Message::CliImageRefusalOpaque,
            &args!("holes" => holes.to_string()),
        ),
        PixelRefusal::Gain { k } => say(Message::CliImageRefusalGain, &args!("k" => fixed(k, 2))),
        PixelRefusal::Edges { ratio } => say(
            Message::CliImageRefusalEdges,
            &args!("ratio" => format!("{}%", fixed(ratio * 100.0, 0))),
        ),
        PixelRefusal::OutOfRange { share } => say(
            Message::CliImageRefusalOutOfRange,
            &args!("share" => format!("{}%", fixed(share * 100.0, 1))),
        ),
    }
}

/// How the picture was written back, when its pixels were.
fn encoding_lines(say: Say, encoding: Encoding) -> Vec<String> {
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

/// A class's words. Exhaustive, so a twelfth class does not compile here
/// until it has a key.
fn class_label(class: UnicodeClass) -> Message {
    match class {
        UnicodeClass::ZeroWidth => Message::UnicodeClassZeroWidth,
        UnicodeClass::ZeroWidthJoiner => Message::UnicodeClassZwj,
        UnicodeClass::BidiControl => Message::UnicodeClassBidiControl,
        UnicodeClass::TagCharacter => Message::UnicodeClassTagCharacter,
        UnicodeClass::VariationSelector => Message::UnicodeClassVariationSelector,
        UnicodeClass::SoftHyphen => Message::UnicodeClassSoftHyphen,
        UnicodeClass::ExoticSpace => Message::UnicodeClassExoticSpace,
        UnicodeClass::Noncharacter => Message::UnicodeClassNoncharacter,
        UnicodeClass::PrivateUse => Message::UnicodeClassPrivateUse,
        UnicodeClass::DefaultIgnorable => Message::UnicodeClassDefaultIgnorable,
        UnicodeClass::Homoglyph => Message::UnicodeClassHomoglyph,
    }
}

/// A confidence's words. Exhaustive, for the same reason.
fn confidence_label(confidence: Confidence) -> Message {
    match confidence {
        Confidence::Confirmed => Message::ConfidenceConfirmed,
        Confidence::Probable => Message::ConfidenceProbable,
        Confidence::Informational => Message::ConfidenceInformational,
        Confidence::LikelyFalsePositive => Message::ConfidenceLikelyFalsePositive,
    }
}

/// A metadata kind's words. Exhaustive, for the same reason.
fn metadata_label(kind: MetadataKind) -> Message {
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
fn signal_label(signal: Signal) -> Message {
    match signal {
        Signal::C2paManifest => Message::ImageSignalC2paManifest,
        Signal::C2paReference => Message::ImageSignalC2paReference,
        Signal::DigitalSourceType(_) => Message::ImageSignalDigitalSourceType,
        Signal::GeneratorKey(_) => Message::ImageSignalGeneratorKey,
        Signal::GeneratorText(_) => Message::ImageSignalGeneratorText,
    }
}

/// The sheet as Markdown, every word through `say` — [`wording::plain`]
/// for the clipboard, so nothing a window needs and a pipe does not
/// (U+2068/U+2069) leaves with it.
///
/// A line carries what nobody wrote for Markdown — a file's name, a path,
/// the operating system's sentence — so every line is [`spelled`]: a
/// character Layer A would remove is written as `U+XXXX`, the way the MCP
/// server says back what a client sent, and Markdown's own characters are
/// escaped, so a name with `*` or `#` in it reads back as itself.
pub fn markdown(say: Say, sheet: &Sheet) -> String {
    let mut out = format!(
        "# {}\n",
        spelled(&say(
            Message::WindowReportTitle,
            &args!("name" => sheet.title.clone())
        ))
    );
    for (title, lines) in sheet.sections() {
        out.push_str(&format!("\n## {}\n\n", spelled(&say(title, &none()))));
        for line in lines {
            out.push_str(&"  ".repeat(line.depth));
            out.push_str("- ");
            out.push_str(&spelled(&line.text));
            out.push('\n');
        }
    }
    out
}

/// One line of the Markdown copy: what Layer A would remove from it, and
/// any control character, spelled as `U+XXXX`; Markdown's characters
/// escaped with a backslash. Everything else — Cyrillic, umlauts, the
/// typography of the catalogue — as it is.
fn spelled(line: &str) -> String {
    use std::fmt::Write as _;
    let removed: Vec<char> = wipemark_core::inspect(line, &wipemark_core::Options::default())
        .findings
        .iter()
        .map(|finding| finding.codepoint)
        .collect();
    let mut out = String::with_capacity(line.len());
    for character in line.chars() {
        if character.is_control() || removed.contains(&character) {
            let _ = write!(out, "U+{:04X}", u32::from(character));
        } else {
            if MARKDOWN.contains(&character) {
                out.push('\\');
            }
            out.push(character);
        }
    }
    out
}

/// The characters Markdown reads as markup anywhere in a line.
const MARKDOWN: [char; 10] = ['\\', '`', '*', '_', '[', ']', '#', '<', '>', '|'];

/// What was last copied, for the word beside the button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Copied {
    Json,
    Markdown,
}

/// The dialog over the main window.
pub struct ReportView {
    focus: FocusHandle,
    /// In the window's words.
    sheet: Sheet,
    /// The library's own JSON, when a layer ran.
    json: Option<String>,
    /// The same sheet, in plain words, as Markdown.
    markdown: String,
    copied: Option<Copied>,
    /// Whether this dialog has still to answer — see `dialog`.
    armed: bool,
}

impl EventEmitter<Answer> for ReportView {}

impl ReportView {
    pub fn new(
        intake: &Intake,
        outcome: &Outcome,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::with_words(
            intake,
            outcome,
            &wording::window,
            &wording::plain,
            window,
            cx,
        )
    }

    /// The dialog, with the words it is drawn in (`shown`) and the words
    /// its Markdown copy is written in (`plain`) handed in — what lets a
    /// test hand it a window's real `Rendering::Ui` rather than the
    /// process's default.
    pub fn with_words(
        intake: &Intake,
        outcome: &Outcome,
        shown: Say,
        plain: Say,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        Self {
            focus,
            sheet: sheet(shown, intake, outcome),
            json: outcome.report.as_ref().map(Report::to_json),
            markdown: markdown(plain, &sheet(plain, intake, outcome)),
            copied: None,
            armed: true,
        }
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        if !self.armed {
            return;
        }
        self.armed = false;
        cx.emit(Answer::Dismissed);
    }

    fn copy(&mut self, what: Copied, cx: &mut Context<Self>) {
        let text = match what {
            Copied::Json => match &self.json {
                Some(json) => json.clone(),
                None => return,
            },
            Copied::Markdown => self.markdown.clone(),
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.copied = Some(what);
        cx.notify();
    }
}

impl Focusable for ReportView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for ReportView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let copied = SharedString::from(wording::window(Message::WindowReportCopied, &none()));
        let sections = self.sheet.sections().map(|(title, lines)| {
            v_flex()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .font_semibold()
                        .text_color(theme.foreground)
                        .child(SharedString::from(wording::window(title, &none()))),
                )
                .children(lines.iter().map(|line| {
                    div()
                        .pl(px(12.0 * line.depth as f32))
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(SharedString::from(line.text.clone()))
                }))
        });

        dialog::backdrop(cx)
            .id("report-backdrop")
            .on_action(cx.listener(|view, _: &Next, window, cx| {
                cx.stop_propagation();
                dialog::hold(&view.focus, window, cx);
            }))
            .on_action(cx.listener(|view, _: &Previous, window, cx| {
                cx.stop_propagation();
                dialog::hold(&view.focus, window, cx);
            }))
            .on_action(cx.listener(|view, _: &dialog::Accept, _, cx| {
                cx.stop_propagation();
                view.close(cx);
            }))
            .on_action(cx.listener(|view, _: &Dismiss, _, cx| {
                cx.stop_propagation();
                view.close(cx);
            }))
            .on_click(cx.listener(|view, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                view.close(cx);
            }))
            .child(
                v_flex()
                    .id("report-panel")
                    .track_focus(&self.focus)
                    .key_context(dialog::CONTEXT)
                    .flex_none()
                    .gap_3()
                    .w(px(600.0))
                    .max_h(px(560.0))
                    .p_4()
                    .rounded(theme.radius)
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.popover)
                    .text_color(theme.popover_foreground)
                    .shadow_lg()
                    .on_click(|_: &ClickEvent, _, cx| cx.stop_propagation())
                    .child(div().text_sm().font_semibold().child(SharedString::from(
                        wording::window(
                            Message::WindowReportTitle,
                            &args!("name" => self.sheet.title.clone()),
                        ),
                    )))
                    .child(
                        v_flex()
                            .id("report-body")
                            .flex_1()
                            .min_h(px(0.0))
                            .overflow_y_scroll()
                            .gap_3()
                            .children(sections),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .gap_2()
                            .items_center()
                            .justify_end()
                            .pt_1()
                            .when_some(self.copied, |row, _| {
                                row.child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(copied.clone()),
                                )
                            })
                            .child(
                                Button::new("report-copy-json")
                                    .small()
                                    .outline()
                                    .label(SharedString::from(wording::window(
                                        Message::WindowReportCopyJson,
                                        &none(),
                                    )))
                                    .disabled(self.json.is_none())
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.copy(Copied::Json, cx);
                                    })),
                            )
                            .child(
                                Button::new("report-copy-markdown")
                                    .small()
                                    .outline()
                                    .label(SharedString::from(wording::window(
                                        Message::WindowReportCopyMarkdown,
                                        &none(),
                                    )))
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.copy(Copied::Markdown, cx);
                                    })),
                            )
                            .child(
                                Button::new("report-close")
                                    .small()
                                    .primary()
                                    .label(SharedString::from(wording::window(
                                        Message::WindowReportClose,
                                        &none(),
                                    )))
                                    .on_click(cx.listener(|view, _, _, cx| view.close(cx))),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::{TimeZone as _, Utc};
    use gpui::prelude::*;
    use gpui::{Entity, TestAppContext, VisualTestContext};
    use wipemark_i18n::{available_languages, FluentArgs, Localizer, Message, Rendering};
    use wipemark_intake::Handed;

    use super::{markdown, sheet, Copied, ReportView, Sheet};
    use crate::clean::{self, Outcome, Refusal, Report, Verdict};
    use crate::drop::Arrival;
    use crate::retention::{self, Homes, Retention, Source};

    /// A scratch directory that takes its own files away with it.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(label: &str) -> Self {
            let directory = std::env::temp_dir()
                .join(format!("wipemark-report-{label}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&directory);
            std::fs::create_dir_all(&directory).expect("scratch directory");
            Self(directory)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).ok();
        }
    }

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(format!(
            "{}/../../fixtures/image/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
    }

    /// One real clean of `bytes` named `name`, beside it, as a row does.
    fn cleaned(scratch: &Scratch, name: &str, bytes: &[u8]) -> (Arrival, Outcome) {
        let path = scratch.0.join(name);
        std::fs::write(&path, bytes).expect("scratch file");
        let handed = Handed::Path(path);
        let arrival = Arrival {
            intake: wipemark_intake::of(&handed),
            handed,
        };
        let homes = Homes {
            results: scratch.0.join("results"),
            kept: scratch.0.join("kept"),
        };
        let plan = retention::plan(&Source::of(&arrival.intake), &Retention::default(), &homes);
        let now = Utc.with_ymd_and_hms(2026, 10, 5, 13, 46, 2).unwrap();
        let outcome = clean::clean_one(&arrival, &plan, 7, now);
        (arrival, outcome)
    }

    const MARKED: &str = "# Notes\n\nA zero\u{200B}width space, and \u{0430} Cyrillic a.\n";

    /// Every kind of ending a clean has: a text cleaned with a kept
    /// homoglyph, nothing found, a Gemini picture restored, one with a
    /// mark left, metadata removed, an existing result refused, a TIFF
    /// refused unread.
    fn every_kind(scratch: &Scratch) -> Vec<(String, Arrival, Outcome)> {
        let mut all = Vec::new();
        let mut add = |name: &str, bytes: Vec<u8>| {
            let (arrival, outcome) = cleaned(scratch, name, &bytes);
            all.push((name.to_owned(), arrival, outcome));
        };
        add("marked.md", MARKED.as_bytes().to_vec());
        add("plain.md", b"Nothing to see here.\n".to_vec());
        add(
            "torch-1025.png",
            std::fs::read(fixture("gemini/torch-1025.png")).expect("fixture"),
        );
        add(
            "crying-1025.png",
            std::fs::read(fixture("gemini/crying-1025.png")).expect("fixture"),
        );
        add(
            "c2pa-jumbf.jpg",
            std::fs::read(fixture("c2pa-jumbf.jpg")).expect("fixture"),
        );
        std::fs::write(scratch.0.join("taken.cleaned.md"), b"somebody's").expect("existing");
        add("taken.md", MARKED.as_bytes().to_vec());
        add("scan.tiff", b"II*\0\x08\0\0\0".to_vec());
        all
    }

    fn english() -> Localizer {
        Localizer::for_languages(&["en-US".parse().expect("tag")], Rendering::PlainText)
    }

    /// Every ending renders all five sections, every one with a line, and
    /// the third shelf is the report's own: a picture's leads with the
    /// pixels' claim, and none is ever empty.
    #[test]
    fn every_outcome_has_its_three_shelves() {
        let scratch = Scratch::new("shelves");
        let localizer = english();
        let say = |message: Message, args: &FluentArgs| localizer.format_args(message, args);
        let pixels = localizer.format_args(
            Message::ReportNotEstablishedInvisiblePixelMarks,
            &FluentArgs::new(),
        );
        let mut endings = Vec::new();
        for (name, arrival, outcome) in every_kind(&scratch) {
            endings.push(outcome.verdict.id());
            let sheet: Sheet = sheet(&say, &arrival.intake, &outcome);
            for (title, lines) in sheet.sections() {
                assert!(!lines.is_empty(), "{name}: {title:?} is empty");
                for line in lines {
                    assert!(!line.text.trim().is_empty(), "{name}: an empty line");
                }
            }
            assert!(
                sheet.not_established.len() >= 3,
                "{name}: the third shelf is short"
            );
            let picture = arrival.intake.kind == wipemark_intake::Kind::Image;
            assert_eq!(
                sheet.not_established[0].text == pixels,
                picture,
                "{name}: the pixels' claim is not first exactly on a picture"
            );
        }
        for ending in ["cleaned", "nothing-found", "partly", "not-cleaned"] {
            assert!(endings.contains(&ending), "no {ending} among {endings:?}");
        }
    }

    /// What a text's shelves hold: the zero-width space removed, by its
    /// standard name, class, confidence and count, on the verifiable
    /// shelf; the Cyrillic a kept, and why, on the best-effort one.
    #[test]
    fn a_text_is_said_by_what_was_removed_and_what_was_kept() {
        let scratch = Scratch::new("text");
        let (arrival, outcome) = cleaned(&scratch, "marked.md", MARKED.as_bytes());
        let localizer = english();
        let say = |message: Message, args: &FluentArgs| localizer.format_args(message, args);
        let sheet = sheet(&say, &arrival.intake, &outcome);
        let verifiable: Vec<&str> = sheet.verifiable.iter().map(|l| l.text.as_str()).collect();
        assert!(
            verifiable
                .iter()
                .any(|line| line.starts_with("U+200B ZERO WIDTH SPACE") && line.contains("once")),
            "{verifiable:?}"
        );
        let best: Vec<&str> = sheet.best_effort.iter().map(|l| l.text.as_str()).collect();
        assert!(
            best.iter().any(|line| line.starts_with("U+0430")),
            "{best:?}"
        );
        assert!(
            sheet
                .happened
                .iter()
                .any(|l| l.text.contains("marked.cleaned.md")),
            "{:?}",
            sheet.happened
        );
    }

    /// The Markdown copy in every language: no isolate, nothing Layer A
    /// would remove, all five titles — over every ending.
    #[test]
    fn the_markdown_copy_carries_nothing_layer_a_would_remove() {
        let scratch = Scratch::new("markdown");
        let endings = every_kind(&scratch);
        for language in available_languages() {
            let localizer =
                Localizer::for_languages(std::slice::from_ref(&language.id), Rendering::PlainText);
            let say = |message: Message, args: &FluentArgs| localizer.format_args(message, args);
            for (name, arrival, outcome) in &endings {
                let copy = markdown(&say, &sheet(&say, &arrival.intake, outcome));
                assert!(
                    !copy.contains(['\u{2068}', '\u{2069}']),
                    "{}: {name}: an isolate in the copy",
                    language.id
                );
                let found = wipemark_core::inspect(&copy, &wipemark_core::Options::default());
                assert!(
                    found.findings.is_empty(),
                    "{}: {name}: Layer A would remove {:?}",
                    language.id,
                    found.findings
                );
                assert_eq!(copy.matches("\n## ").count(), 5, "{name}: {copy}");
            }
        }
    }

    /// The third shelf is read off the report: an id a picture's pixel
    /// pass or its metadata pass carries — one this build has no sentence
    /// for — is shown, as itself, in the report's order, and never dropped.
    #[test]
    fn the_third_shelf_is_the_reports_own() {
        use wipemark_picture::Visible;
        let scratch = Scratch::new("own-shelf");
        let bytes = std::fs::read(fixture("gemini/torch-1025.png")).expect("fixture");
        let (arrival, mut outcome) = cleaned(&scratch, "torch-1025.png", &bytes);
        let Some(Report::Picture(picture)) = &mut outcome.report else {
            panic!("{:?}", outcome.verdict);
        };
        let Visible::Examined { report, .. } = &mut picture.visible else {
            panic!("the pixels were not examined");
        };
        report.not_established.push("a-pixel-claim");
        picture.metadata.not_established.push("a-metadata-claim");
        let localizer = english();
        let say = |message: Message, args: &FluentArgs| localizer.format_args(message, args);
        let shelf: Vec<String> = sheet(&say, &arrival.intake, &outcome)
            .not_established
            .into_iter()
            .map(|line| line.text)
            .collect();
        assert_eq!(shelf.len(), 6, "{shelf:?}");
        assert_eq!(
            shelf[0],
            say(
                Message::ReportNotEstablishedInvisiblePixelMarks,
                &FluentArgs::new()
            )
        );
        assert_eq!(shelf[4], "a-pixel-claim", "{shelf:?}");
        assert_eq!(shelf[5], "a-metadata-claim", "{shelf:?}");
    }

    /// An id core lists that this build's catalogue has no sentence for is
    /// shown as its canonical English beside the id; an id nothing here
    /// knows, as the id — never dropped.
    #[test]
    fn a_claim_with_no_sentence_is_shown_in_its_own_words() {
        let localizer = english();
        let say = |message: Message, args: &FluentArgs| localizer.format_args(message, args);
        assert_eq!(super::shelf_line(&say, "a-new-claim"), "a-new-claim");
        let empty = |_: Message, _: &FluentArgs| String::new();
        for (id, canonical) in wipemark_core::report::not_established::ALL {
            let line = super::shelf_line(&say, id);
            assert!(!line.is_empty() && !line.contains(id), "{id}: {line}");
            // With no sentence to say it in, the canonical words and the id.
            let unsaid = super::shelf_line(&empty, id);
            assert!(unsaid.is_empty() || unsaid.contains(canonical), "{unsaid}");
        }
    }

    /// The Markdown copy spells what nobody wrote for Markdown: a file
    /// named with a U+200B and Markdown's characters, in every language,
    /// leaves no character Layer A would remove in the copy, and reads
    /// back as its own name with the invisible character spelled.
    #[test]
    fn the_markdown_copy_spells_and_escapes_a_name() {
        let scratch = Scratch::new("named");
        let name = "a\u{200B}b*c_d`e[f]#g.md";
        let (arrival, outcome) = cleaned(&scratch, name, MARKED.as_bytes());
        let spelled = "aU+200Bb*c_d`e[f]#g";
        for language in available_languages() {
            let localizer =
                Localizer::for_languages(std::slice::from_ref(&language.id), Rendering::PlainText);
            let say = |message: Message, args: &FluentArgs| localizer.format_args(message, args);
            let copy = markdown(&say, &sheet(&say, &arrival.intake, &outcome));
            let found = wipemark_core::inspect(&copy, &wipemark_core::Options::default());
            assert!(
                found.findings.is_empty(),
                "{}: {:?} in {copy}",
                language.id,
                found.findings
            );
            assert!(
                copy.contains("aU+200Bb\\*c\\_d\\`e\\[f\\]\\#g"),
                "{}: the name is not escaped: {copy}",
                language.id
            );
            let read_back = unescaped(&copy);
            assert!(
                read_back.contains(&format!("{spelled}.md"))
                    && read_back.contains(&format!("{spelled}.cleaned.md")),
                "{}: {read_back}",
                language.id
            );
        }
    }

    /// What a Markdown reader shows for a backslash-escaped line.
    fn unescaped(markdown: &str) -> String {
        let mut out = String::new();
        let mut chars = markdown.chars();
        while let Some(c) = chars.next() {
            match c {
                '\\' => out.extend(chars.next()),
                c => out.push(c),
            }
        }
        out
    }

    /// A dialog over `outcome`, drawn in a window's real words — with the
    /// isolates — and copying in plain ones.
    fn dialog(
        cx: &mut TestAppContext,
        arrival: Arrival,
        outcome: Outcome,
    ) -> (Entity<ReportView>, &mut VisualTestContext) {
        cx.update(gpui_component::init);
        let slot: std::rc::Rc<std::cell::RefCell<Option<Entity<ReportView>>>> =
            std::rc::Rc::default();
        let held = slot.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let shown = Localizer::for_languages(&["en-US".parse().expect("tag")], Rendering::Ui);
            let plain = english();
            let view = cx.new(|cx| {
                ReportView::with_words(
                    &arrival.intake,
                    &outcome,
                    &|message, args| shown.format_args(message, args),
                    &|message, args| plain.format_args(message, args),
                    window,
                    cx,
                )
            });
            *held.borrow_mut() = Some(view.clone());
            gpui_component::Root::new(view, window, cx)
        });
        let view = slot.take().expect("the window builder ran");
        (view, cx)
    }

    /// Copy JSON is the library's own `to_json()`, byte for byte; Copy as
    /// Markdown is in plain words though the window's are isolated.
    #[gpui::test]
    fn the_copies_are_the_json_and_plain_markdown(cx: &mut TestAppContext) {
        let scratch = Scratch::new("copies");
        let (arrival, outcome) = cleaned(&scratch, "marked.md", MARKED.as_bytes());
        let json = outcome.report.as_ref().expect("a report").to_json();
        let (view, cx) = dialog(cx, arrival, outcome);

        let isolated = cx.update(|_, cx| {
            view.read(cx)
                .sheet
                .happened
                .iter()
                .any(|line| line.text.contains('\u{2068}'))
        });
        assert!(isolated, "the window's words are not a window's");

        view.update(cx, |view, cx| view.copy(Copied::Json, cx));
        let copied = cx.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
        assert_eq!(copied.as_deref(), Some(json.as_str()));

        view.update(cx, |view, cx| view.copy(Copied::Markdown, cx));
        let copied = cx
            .update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
            .expect("markdown on the clipboard");
        assert!(copied.starts_with("# "), "{copied}");
        assert!(
            !copied.contains(['\u{2068}', '\u{2069}']),
            "the copy carries an isolate: {copied:?}"
        );
    }

    /// A refusal before anything was read still has a report to show —
    /// and no JSON to copy, rather than an empty one.
    #[gpui::test]
    fn a_thing_never_read_has_shelves_and_no_json(cx: &mut TestAppContext) {
        let scratch = Scratch::new("unread");
        let (arrival, outcome) = cleaned(&scratch, "scan.tiff", b"II*\0\x08\0\0\0");
        assert!(matches!(
            outcome.verdict,
            Verdict::NotCleaned(Refusal::NotCleanable(_))
        ));
        let (view, cx) = dialog(cx, arrival, outcome);
        let json = cx.update(|_, cx| view.read(cx).json.clone());
        assert_eq!(json, None);
    }
}
