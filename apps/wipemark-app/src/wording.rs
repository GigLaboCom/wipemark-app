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

use wipemark_i18n::{args, t, t_args, Message};
use wipemark_intake::{Arrived, Evidence, Intake, Kind};

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
    t(match kind {
        Kind::Text => Message::KindText,
        Kind::Image => Message::KindImage,
        Kind::Document => Message::KindDocument,
        Kind::Archive => Message::KindArchive,
        Kind::Media => Message::KindMedia,
        Kind::Data => Message::KindData,
        Kind::Folder => Message::KindFolder,
        Kind::Unknown => Message::KindUnknown,
    })
}

/// What to call one thing that arrived.
///
/// A file has a name, and everything else has only what it *is*,
/// because "image.png" for something that was never a file on this disk
/// would be a name nobody could go and find.
pub fn title_of(intake: &Intake) -> String {
    let named = match intake.arrived {
        Arrived::AsPath => intake.name.clone(),
        Arrived::AsText | Arrived::AsBytes => None,
    };
    named.unwrap_or_else(|| kind_label(intake.kind))
}

/// The thing worth being told about how much of this is established,
/// when there is one.
///
/// A name that lost an argument with the bytes is the one a person most
/// needs to see, so it is the one that is not muted. Agreement, content
/// alone and nothing at all are not notes: the ordinary case is silent.
pub fn evidence_note(intake: &Intake) -> Option<(String, Tone)> {
    match intake.evidence {
        Evidence::Disagreed { name_said } => Some((
            t_args(
                Message::PanelDropMismatch,
                &args!(
                    "named" => name_said.name(),
                    "found" => intake
                        .format
                        .map_or_else(|| kind_label(intake.kind), |format| format.name().to_owned())
                ),
            ),
            Tone::Warning,
        )),
        Evidence::Name => Some((t(Message::PanelDropByName), Tone::Muted)),
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use wipemark_intake::{Evidence, Format, Handed, Kind};

    use super::{evidence_note, title_of, would_happen, Tone};
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
}
