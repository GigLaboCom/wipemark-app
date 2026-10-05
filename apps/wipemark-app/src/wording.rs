//! The sentences two windows say about one thing that arrived.
//!
//! Epic **E6**. The panel and the main window's queue both take a drop,
//! and both have to say what it was — what a kind is *called*, whether
//! the name and the bytes agreed, and what would happen to it once
//! cleaning exists. Those sentences were the panel's until the queue
//! needed them too, and two copies of a sentence are two sentences that
//! drift. Everything here is a pure function over values, so it can be
//! checked without a window, and every string comes out of the
//! catalogue, so it comes out in the language the window is drawn in.
//!
//! `wipemark-intake` names kinds and never words — a library that
//! formatted its own prose could not be used from a CLI that had chosen
//! a different language. This module is the other half of that rule.

use std::path::Path;

use wipemark_i18n::{args, t, t_args, t_args_plain, FluentArgs, Message};
use wipemark_image::ImageError;
use wipemark_intake::{Arrived, Evidence, Intake, Kind};
use wipemark_picture::{NotExamined, PictureError};

use crate::clean::{Failure, Left, Outcome, Refusal, Report, Unable, Verdict};
use crate::drop::size_label;
use crate::retention::{Kept, Plan, Written};

/// How loudly a note is painted.
///
/// A surface maps these onto its theme; the module says which, and not
/// what colour that is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Worth being told: painted in the warning colour.
    Warning,
    /// Worth knowing: painted muted.
    Muted,
}

/// What a kind is called, in the language the window is drawn in.
pub fn kind_label(kind: Kind) -> String {
    kind_label_in(&window, kind)
}

/// What a kind is called, in `say`'s words.
pub fn kind_label_in(say: Say, kind: Kind) -> String {
    say(
        match kind {
            Kind::Text => Message::KindText,
            Kind::Image => Message::KindImage,
            Kind::Document => Message::KindDocument,
            Kind::Archive => Message::KindArchive,
            Kind::Media => Message::KindMedia,
            Kind::Data => Message::KindData,
            Kind::Folder => Message::KindFolder,
            Kind::Unknown => Message::KindUnknown,
        },
        &FluentArgs::new(),
    )
}

/// What to call one thing that arrived.
///
/// A file has a name, and everything else has only what it *is*,
/// because "image.png" for something that was never a file on this disk
/// would be a name nobody could go and find.
pub fn title_of(intake: &Intake) -> String {
    title_of_in(&window, intake)
}

/// [`title_of`], in `say`'s words.
pub fn title_of_in(say: Say, intake: &Intake) -> String {
    let named = match intake.arrived {
        Arrived::AsPath => intake.name.clone(),
        Arrived::AsText | Arrived::AsBytes => None,
    };
    named.unwrap_or_else(|| kind_label_in(say, intake.kind))
}

/// The thing worth being told about how much of this is established,
/// when there is one.
///
/// A name that lost an argument with the bytes is the one a person most
/// needs to see, so it is the one that is not muted. Agreement, content
/// alone and nothing at all are not notes: the ordinary case is silent.
pub fn evidence_note(intake: &Intake) -> Option<(String, Tone)> {
    evidence_note_in(&window, intake)
}

/// [`evidence_note`], in `say`'s words.
pub fn evidence_note_in(say: Say, intake: &Intake) -> Option<(String, Tone)> {
    match intake.evidence {
        Evidence::Disagreed { name_said } => Some((
            say(
                Message::PanelDropMismatch,
                &args!(
                    "named" => name_said.name(),
                    "found" => intake.format.map_or_else(
                        || kind_label_in(say, intake.kind),
                        |format| format.name().to_owned()
                    )
                ),
            ),
            Tone::Warning,
        )),
        Evidence::Name => Some((
            say(Message::PanelDropByName, &FluentArgs::new()),
            Tone::Muted,
        )),
        Evidence::Content | Evidence::Agreed | Evidence::Nothing => None,
    }
}

/// What would happen to one thing that arrived, once cleaning
/// arrives — the Retention page's rows, read against this one thing.
///
/// A sentence for where the result would go, and one more for what
/// Wipemark would keep of its own when it would keep anything. Names
/// and folders are the operating system's spelling, never translated.
pub fn would_happen(plan: &Plan) -> Vec<String> {
    let name_of = |path: &std::path::Path| {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    let result = |written: &Written, from_a_file: bool| match written {
        Written::Beside(path) => t_args(
            Message::PanelDropResultBeside,
            &args!("name" => name_of(path)),
        ),
        Written::Into { folder, .. } => t_args(
            if from_a_file {
                Message::PanelDropResultIntoFile
            } else {
                Message::PanelDropResultInto
            },
            &args!("folder" => folder.display().to_string()),
        ),
        Written::Over { set_aside_as, .. } => t_args(
            Message::PanelDropResultOver,
            &args!("name" => name_of(set_aside_as)),
        ),
        Written::AsText => t(Message::PanelDropResultAsText),
    };
    match plan {
        Plan::EachFileIn(_) => vec![t(Message::PanelDropEachFile)],
        Plan::File(written) => vec![result(written, true)],
        Plan::Loose {
            result: written,
            kept,
        } => {
            let mut lines = vec![result(written, false)];
            if let Some(Kept {
                original,
                result,
                for_,
                ..
            }) = kept
            {
                lines.push(t_args(
                    match (original, result) {
                        (true, false) => Message::PanelDropKeptOriginals,
                        (false, true) => Message::PanelDropKeptResults,
                        _ => Message::PanelDropKeptBoth,
                    },
                    &args!("period" => t(for_.span())),
                ));
            }
            lines
        }
    }
}

/// How a clean's badge is painted. A surface maps these onto its theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    /// Nothing happened worth a colour: nothing found.
    Muted,
    /// Cleaned.
    Success,
    /// Something is left, or nothing was done: partly, not cleaned.
    Warning,
    /// A write failed.
    Danger,
}

/// The badge a finished clean wears, and its word.
pub fn verdict_badge(verdict: &Verdict) -> (Message, Badge) {
    match verdict {
        Verdict::NothingFound => (Message::QueueStatusNothingFound, Badge::Muted),
        Verdict::Cleaned => (Message::QueueStatusCleaned, Badge::Success),
        Verdict::Partly(_) => (Message::QueueStatusPartly, Badge::Warning),
        Verdict::NotCleaned(_) => (Message::QueueStatusNotCleaned, Badge::Warning),
        Verdict::Failed(_) => (Message::QueueStatusFailed, Badge::Danger),
    }
}

/// How a line is put into words: [`window`] for what a window draws,
/// [`plain`] for what leaves it as text (a copied report), or a
/// per-language `Localizer` in a test — never `wipemark_i18n::init`,
/// which is process-wide.
pub type Say<'a> = &'a dyn Fn(Message, &FluentArgs) -> String;

/// A window's words: Fluent's isolates kept, for a text renderer that
/// understands them.
pub fn window(message: Message, args: &FluentArgs) -> String {
    t_args(message, args)
}

/// Words that leave the window as text: no U+2068/U+2069, which Layer A
/// removes — `Rendering::PlainText` for anything that is not a window.
pub fn plain(message: Message, args: &FluentArgs) -> String {
    t_args_plain(message, args)
}

/// [`said_in`], in a window's words.
pub fn said(outcome: &Outcome) -> String {
    said_in(&window, outcome)
}

/// [`unable_in`], in a window's words.
pub fn unable(why: Unable) -> String {
    unable_in(&window, why)
}

/// [`went_in`], in a window's words.
pub fn went(outcome: &Outcome) -> Vec<String> {
    went_in(&window, outcome)
}

/// The one sentence a finished clean comes to — the badge's tooltip and
/// the first line of what a row says about it.
pub fn said_in(say: Say, outcome: &Outcome) -> String {
    match &outcome.verdict {
        Verdict::NothingFound => say(Message::CleanSaidNothingFound, &FluentArgs::new()),
        Verdict::Cleaned => match &outcome.report {
            Some(Report::Text(report)) => {
                let count: u32 = report
                    .removed
                    .iter()
                    .map(|(_, n)| n)
                    .chain(report.normalized.iter().map(|(_, n)| n))
                    .sum();
                say(Message::CleanSaidCleanedText, &args!("count" => count))
            }
            _ => say(Message::CleanSaidCleanedPicture, &FluentArgs::new()),
        },
        Verdict::Partly(Left::Kept) => say(Message::CleanSaidPartlyKept, &FluentArgs::new()),
        Verdict::Partly(Left::Mark) => say(Message::CleanSaidPartlyMark, &FluentArgs::new()),
        Verdict::Partly(Left::NotExamined(NotExamined::Animated)) => {
            say(Message::CleanSaidPartlyAnimated, &FluentArgs::new())
        }
        Verdict::Partly(Left::NotExamined(_)) => {
            say(Message::CleanSaidPartlyUnexamined, &FluentArgs::new())
        }
        Verdict::NotCleaned(refusal) => refused_in(say, refusal),
        Verdict::Failed(failure) => failed_in(say, failure),
    }
}

/// Why a thing cannot be cleaned, from what intake said about it.
pub fn unable_in(say: Say, unable: Unable) -> String {
    match unable {
        Unable::NotYet(format) => say(
            Message::CleanRefusedNotYet,
            &args!("format" => format.name()),
        ),
        Unable::Folder => say(Message::CleanRefusedFolder, &FluentArgs::new()),
        Unable::Kind { kind, format } => say(
            Message::CleanRefusedKind,
            &args!("what" => format.map_or_else(|| kind_label_in(say, kind), |format| format.name().to_owned())),
        ),
        Unable::UnnamedEncoding => say(Message::CleanRefusedUnnamedEncoding, &FluentArgs::new()),
        Unable::Unread => say(Message::CleanRefusedUnread, &FluentArgs::new()),
    }
}

/// Why nothing was written. A picture's format is named by the
/// container the passes read, never by the file's name.
pub fn refused_in(say: Say, refusal: &Refusal) -> String {
    match refusal {
        Refusal::NotCleanable(why) => unable_in(say, *why),
        Refusal::TooBig { size, limit } => say(
            Message::CleanRefusedTooBig,
            &args!("size" => size_label(*size), "limit" => size_label(*limit)),
        ),
        Refusal::Unreadable(_) => say(Message::CleanRefusedUnreadable, &FluentArgs::new()),
        Refusal::Undecodable { encoding, offset } => say(
            Message::CleanRefusedUndecodable,
            &args!("encoding" => encoding.name(), "offset" => *offset),
        ),
        Refusal::Picture(error) => match error {
            PictureError::Image(ImageError::UnknownContainer) => {
                say(Message::CleanRefusedPictureUnknown, &FluentArgs::new())
            }
            PictureError::Image(ImageError::NotYet(container)) => say(
                Message::CleanRefusedNotYet,
                &args!("format" => container.name()),
            ),
            PictureError::Image(ImageError::Malformed {
                container, offset, ..
            }) => say(
                Message::CleanRefusedPictureMalformed,
                &args!("format" => container.name(), "offset" => *offset),
            ),
            PictureError::Image(ImageError::Unsupported {
                container, offset, ..
            }) => say(
                Message::CleanRefusedPictureUnsupported,
                &args!("format" => container.name(), "offset" => *offset),
            ),
            PictureError::Decode { .. } => {
                say(Message::CleanRefusedPictureDecode, &FluentArgs::new())
            }
            PictureError::Encode { .. } => {
                say(Message::CleanRefusedPictureEncode, &FluentArgs::new())
            }
            PictureError::Proof(_) => say(Message::CleanRefusedPictureProof, &FluentArgs::new()),
        },
        Refusal::StillMarked { .. } => say(Message::CleanRefusedStillMarked, &FluentArgs::new()),
        Refusal::Exists(path) => say(
            Message::CleanRefusedExists,
            &args!("name" => file_name(path)),
        ),
        Refusal::OriginalExists(path) => say(
            Message::CleanRefusedOriginalExists,
            &args!("name" => file_name(path)),
        ),
        Refusal::SameFile(_) => say(Message::CleanRefusedSameFile, &FluentArgs::new()),
        Refusal::Nowhere => say(Message::CleanRefusedNowhere, &FluentArgs::new()),
    }
}

/// What a write that failed did, and where the original is when that is
/// the question.
pub fn failed_in(say: Say, failure: &Failure) -> String {
    match failure {
        Failure::Write { path, error } => say(
            Message::CleanFailedWrite,
            &args!("path" => path.display().to_string(), "error" => error.message.clone()),
        ),
        Failure::SetAside { original, error } => say(
            Message::CleanFailedSetAside,
            &args!("name" => file_name(original), "error" => error.message.clone()),
        ),
        Failure::Stranded {
            original, error, ..
        } => say(
            Message::CleanFailedStranded,
            &args!("path" => original.display().to_string(), "error" => error.message.clone()),
        ),
    }
}

/// Where a clean's result went: the result, the original set aside, the
/// kept copies — or that nothing was written. The *went* half of
/// [`would_happen`], read off what happened rather than off the plan.
pub fn went_in(say: Say, outcome: &Outcome) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(written) = &outcome.written {
        lines.push(match (&outcome.set_aside, outcome.replaced) {
            (Some(_), _) => say(Message::QueueWentInPlace, &FluentArgs::new()),
            (None, true) => say(
                Message::QueueWentReplaced,
                &args!("name" => file_name(written)),
            ),
            (None, false) => say(
                Message::QueueWentWritten,
                &args!("name" => file_name(written)),
            ),
        });
    }
    if let Some(original) = &outcome.set_aside {
        lines.push(say(
            Message::QueueWentSetAside,
            &args!("name" => file_name(original)),
        ));
    }
    if outcome.text.is_some() {
        lines.push(say(Message::QueueWentAsText, &FluentArgs::new()));
    }
    if outcome.written.is_none() && outcome.text.is_none() {
        lines.push(say(Message::QueueWentNothing, &FluentArgs::new()));
    }
    if let Some(kept) = &outcome.kept {
        lines.push(say(
            Message::QueueWentKept,
            &args!("folder" => kept.display().to_string()),
        ));
    }
    lines
}

/// A path's last part, as the operating system spells it.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use wipemark_intake::{Evidence, Format, Handed, Kind};

    use super::{evidence_note, said, title_of, went, would_happen, Tone};
    use crate::clean::{Error, Failure, Left, Outcome, Refusal, Unable, Verdict};
    use crate::retention::{Kept, Period, Plan, Written};

    /// Every plan the retention module can produce, as a surface would
    /// be handed it.
    fn every_plan() -> Vec<Plan> {
        let mut plans = vec![
            Plan::EachFileIn(PathBuf::from("/d/photos")),
            Plan::File(Written::Beside(PathBuf::from("/d/report.cleaned.docx"))),
            Plan::File(Written::Into {
                folder: PathBuf::from("/Users/someone/Downloads"),
                name: Some("report.cleaned.docx".to_owned()),
            }),
            Plan::File(Written::Over {
                file: PathBuf::from("/d/report.docx"),
                set_aside_as: PathBuf::from("/d/report.original.docx"),
            }),
            Plan::Loose {
                result: Written::AsText,
                kept: None,
            },
            Plan::Loose {
                result: Written::Into {
                    folder: PathBuf::from("/Users/someone/Downloads"),
                    name: None,
                },
                kept: None,
            },
        ];
        for (original, result) in [(true, false), (false, true), (true, true)] {
            for for_ in Period::ALL {
                plans.push(Plan::Loose {
                    result: Written::AsText,
                    kept: Some(Kept {
                        original,
                        result,
                        for_,
                        in_: PathBuf::from("/kept"),
                    }),
                });
            }
        }
        plans
    }

    /// Whatever was dropped, there is a sentence for what would happen
    /// to it — and a second one exactly when something would be kept,
    /// naming the span. A plan with no words for it would be a row with
    /// the Retention page's choices silently missing from it.
    #[test]
    fn every_plan_reads_as_a_sentence() {
        for plan in every_plan() {
            let lines = would_happen(&plan);
            let kept = matches!(&plan, Plan::Loose { kept: Some(_), .. });
            assert_eq!(lines.len(), if kept { 2 } else { 1 }, "{plan:?}: {lines:?}");
            for line in &lines {
                assert!(!line.trim().is_empty(), "{plan:?}: an empty line");
                assert!(
                    !line.contains("panel-drop"),
                    "{plan:?} rendered a catalogue key: {line}"
                );
            }
        }
    }

    /// The names a person needs are in the sentence: the result's file
    /// name beside a file, the set-aside name when a file is replaced,
    /// the folder when results go into one.
    #[test]
    fn the_sentence_names_the_file_it_is_about() {
        let beside = would_happen(&Plan::File(Written::Beside(PathBuf::from(
            "/d/report.cleaned.docx",
        ))));
        assert!(beside[0].contains("report.cleaned.docx"), "{beside:?}");
        assert!(
            !beside[0].contains("/d/"),
            "a whole path where a name was wanted"
        );

        let over = would_happen(&Plan::File(Written::Over {
            file: PathBuf::from("/d/report.docx"),
            set_aside_as: PathBuf::from("/d/report.original.docx"),
        }));
        assert!(over[0].contains("report.original.docx"), "{over:?}");

        let into = would_happen(&Plan::Loose {
            result: Written::Into {
                folder: PathBuf::from("/Users/someone/Downloads"),
                name: None,
            },
            kept: None,
        });
        assert!(into[0].contains("/Users/someone/Downloads"), "{into:?}");
    }

    /// A file is called by its name and everything else by what it is:
    /// "image.png" for a screenshot that was never on this disk would be
    /// a name nobody could go and find.
    #[test]
    fn a_thing_with_no_file_behind_it_is_called_by_its_kind() {
        let note = wipemark_intake::of(&Handed::Text("a line of prose".to_owned()));
        assert_eq!(title_of(&note), super::kind_label(Kind::Text));

        let shot = wipemark_intake::of(&Handed::Bytes {
            name: Some("image.png".to_owned()),
            bytes: b"\x89PNG\r\n\x1a\n".to_vec(),
        });
        assert_eq!(shot.name.as_deref(), Some("image.png"));
        assert_eq!(
            title_of(&shot),
            super::kind_label(Kind::Image),
            "a name the pasteboard invented was shown as if it were a file's"
        );
    }

    /// The note is there exactly when there is something to be told,
    /// and a name that lied is louder than a name that merely answered
    /// alone.
    #[test]
    fn only_a_doubtful_answer_gets_a_note() {
        let mut intake = wipemark_intake::identify(b"\x89PNG\r\n\x1a\n", Some("holiday.txt"));
        assert_eq!(
            intake.evidence,
            Evidence::Disagreed {
                name_said: Format::PlainText
            }
        );
        let (note, tone) = evidence_note(&intake).expect("a lie is worth a note");
        assert_eq!(tone, Tone::Warning);
        assert!(note.contains("PNG"), "{note}");

        intake.evidence = Evidence::Name;
        assert_eq!(
            evidence_note(&intake).map(|(_, tone)| tone),
            Some(Tone::Muted)
        );

        for silent in [Evidence::Content, Evidence::Agreed, Evidence::Nothing] {
            intake.evidence = silent;
            assert_eq!(evidence_note(&intake), None, "{silent:?}");
        }
    }

    fn outcome(verdict: Verdict) -> Outcome {
        Outcome {
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

    /// Every way a clean can end has a sentence of its own — none renders
    /// a catalogue key, none is empty — and every way it can end says
    /// where the result went, "nothing was written" included.
    #[test]
    fn every_outcome_reads_as_a_sentence() {
        let error = || Error {
            kind: std::io::ErrorKind::PermissionDenied,
            message: String::from("Permission denied (os error 13)"),
        };
        let path = || PathBuf::from("/d/x.cleaned.md");
        let verdicts = vec![
            Verdict::NothingFound,
            Verdict::Cleaned,
            Verdict::Partly(Left::Kept),
            Verdict::Partly(Left::Mark),
            Verdict::Partly(Left::NotExamined(wipemark_picture::NotExamined::Animated)),
            Verdict::Partly(Left::NotExamined(wipemark_picture::NotExamined::Decode)),
            Verdict::NotCleaned(Refusal::NotCleanable(Unable::NotYet(Format::Tiff))),
            Verdict::NotCleaned(Refusal::NotCleanable(Unable::Folder)),
            Verdict::NotCleaned(Refusal::NotCleanable(Unable::Kind {
                kind: Kind::Archive,
                format: Some(Format::Zip),
            })),
            Verdict::NotCleaned(Refusal::NotCleanable(Unable::UnnamedEncoding)),
            Verdict::NotCleaned(Refusal::NotCleanable(Unable::Unread)),
            Verdict::NotCleaned(Refusal::TooBig {
                size: 70_000_000,
                limit: 67_108_864,
            }),
            Verdict::NotCleaned(Refusal::Unreadable(std::io::ErrorKind::NotFound)),
            Verdict::NotCleaned(Refusal::Undecodable {
                encoding: wipemark_intake::Encoding::Utf8,
                offset: 12,
            }),
            Verdict::NotCleaned(Refusal::Picture(wipemark_picture::PictureError::Proof(
                wipemark_picture::Proof::Outside,
            ))),
            Verdict::NotCleaned(Refusal::StillMarked {
                ai_metadata: true,
                c2pa: false,
            }),
            Verdict::NotCleaned(Refusal::Exists(path())),
            Verdict::NotCleaned(Refusal::OriginalExists(path())),
            Verdict::NotCleaned(Refusal::SameFile(path())),
            Verdict::NotCleaned(Refusal::Nowhere),
            Verdict::Failed(Failure::Write {
                path: path(),
                error: error(),
            }),
            Verdict::Failed(Failure::SetAside {
                original: path(),
                error: error(),
            }),
            Verdict::Failed(Failure::Stranded {
                original: path(),
                error: error(),
                restore: error(),
            }),
        ];
        for verdict in verdicts {
            let label = format!("{verdict:?}");
            let outcome = outcome(verdict);
            let sentence = said(&outcome);
            assert!(!sentence.trim().is_empty(), "{label}");
            for prefix in ["clean-", "queue-"] {
                assert!(!sentence.starts_with(prefix), "{label}: {sentence}");
            }
            let lines = went(&outcome);
            assert!(!lines.is_empty(), "{label} says nowhere");
        }
    }

    /// The *went* lines say what happened, and only that: a result
    /// written beside, one written over an existing result, a file
    /// replaced with its original set aside, a text handed back, a kept
    /// copy.
    #[test]
    fn where_a_result_went_is_said_by_its_name() {
        let mut beside = outcome(Verdict::Cleaned);
        beside.written = Some(PathBuf::from("/d/x.cleaned.md"));
        let lines = went(&beside);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("x.cleaned.md"), "{lines:?}");

        beside.replaced = true;
        let replaced = went(&beside);
        assert_ne!(replaced, lines, "a replacement reads as a fresh write");
        assert!(replaced[0].contains("x.cleaned.md"), "{replaced:?}");

        let mut over = outcome(Verdict::Cleaned);
        over.written = Some(PathBuf::from("/d/x.md"));
        over.set_aside = Some(PathBuf::from("/d/x.original.md"));
        let lines = went(&over);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines[1].contains("x.original.md"), "{lines:?}");

        let mut paste = outcome(Verdict::Cleaned);
        paste.text = Some(String::from("hello"));
        paste.kept = Some(PathBuf::from("/k/20261005T134602-7"));
        let lines = went(&paste);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines[1].contains("20261005T134602-7"), "{lines:?}");
        assert!(
            lines.iter().all(|line| !line.contains("hello")),
            "the text itself is never a note"
        );
    }
}
