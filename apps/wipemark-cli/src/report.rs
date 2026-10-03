//! The human report `inspect` and `clean` print.
//!
//! Every word comes from the catalogue, in `Rendering::PlainText` — this
//! is read in a terminal, piped into a file and shown by a pre-commit
//! hook, and Fluent's U+2068/U+2069 isolates are exactly what Layer A
//! removes. What is not a word is a format and is printed as itself: the
//! code point, the character's Unicode name (an identifier of the
//! standard, never translated — D16, `docs/architecture/i18n.md`), format
//! and encoding names, paths and numbers.
//!
//! The lines, in order: a summary that never calls the text clean; the
//! rows under what would happen (`inspect`) or what did (`clean`); the
//! two notes that keep a row from being misread; where the result went;
//! the Unicode version; and the third shelf, every entry of it, always.

use std::fmt::Write as _;

use wipemark_core::report::not_established;
use wipemark_core::{
    Action, CleanReport, Confidence, InspectReport, NormKind, Options, UnicodeClass, UnicodeFinding,
};
use wipemark_i18n::{args, FluentArgs, Message};
use wipemark_intake::Encoding;

/// How a line is put into words. `t_args` in production; a per-language
/// `wipemark_i18n::Localizer::for_languages(…, Rendering::PlainText)` in
/// the test that renders every language — never `wipemark_i18n::init`,
/// which is process-wide and would race the other tests.
pub(crate) type Say<'a> = &'a dyn Fn(Message, &FluentArgs) -> String;

/// How many offsets a row spells out before it says "and N more".
const SHOWN: usize = 10;

/// Which tense the headings are in.
#[derive(Clone, Copy)]
enum Tense {
    /// `inspect`: what would happen.
    Would,
    /// `clean`: what did.
    Did,
}

/// `inspect`'s report.
pub(crate) fn inspect_lines(
    say: Say,
    source: &str,
    report: &InspectReport,
    options: &Options,
    encoding: Encoding,
) -> Vec<String> {
    let mut lines = body(
        say,
        source,
        &report.findings,
        &report.kept,
        report.suspicious,
        options,
        encoding,
        Tense::Would,
    );
    lines.extend(footer(say, report.unicode_version));
    lines
}

/// `clean`'s report. `written` is where the result went when that is a
/// file; `untouched` says the input was a file, and was not changed.
pub(crate) fn clean_lines(
    say: Say,
    source: &str,
    report: &CleanReport,
    options: &Options,
    encoding: Encoding,
    written: Option<&str>,
    untouched: bool,
) -> Vec<String> {
    let mut lines = body(
        say,
        source,
        &report.findings,
        &report.kept,
        report.suspicious,
        options,
        encoding,
        Tense::Did,
    );
    if options.nfkc {
        lines.push(say(Message::CliCleanNfkc, &FluentArgs::new()));
        let later = acted_later(report);
        if later > 0 {
            lines.push(say(Message::CliCleanLater, &args!("count" => later)));
        }
    }
    if let Some(path) = written {
        lines.push(say(Message::CliCleanWritten, &args!("path" => path)));
    }
    if untouched {
        lines.push(say(Message::CliCleanUntouched, &args!("source" => source)));
    }
    lines.extend(footer(say, report.unicode_version));
    lines
}

/// What the rounds after NFKC acted on, which no row lists: everything
/// the counters say was removed or replaced, less what the rows say the
/// first pass acted on. Zero without `nfkc` — the counters then agree
/// with the rows exactly (E1-3's `counters_agree_with_positions_without_nfkc`).
/// `(Nfkc, n)` is left out: it counts code points NFKC did not carry
/// through, which is a different unit.
fn acted_later(report: &CleanReport) -> u64 {
    let counted: u64 = report
        .removed
        .iter()
        .map(|(_, count)| u64::from(*count))
        .chain(
            report
                .normalized
                .iter()
                .filter(|(kind, _)| *kind != NormKind::Nfkc)
                .map(|(_, count)| u64::from(*count)),
        )
        .sum();
    let listed: u64 = report.findings.iter().map(|row| u64::from(row.count)).sum();
    counted.saturating_sub(listed)
}

/// The summary and the rows, which both commands share.
#[allow(
    clippy::too_many_arguments,
    reason = "the two reports' fields, passed apart because they are two types"
)]
fn body(
    say: Say,
    source: &str,
    findings: &[UnicodeFinding],
    kept: &[UnicodeFinding],
    suspicious: bool,
    options: &Options,
    encoding: Encoding,
    tense: Tense,
) -> Vec<String> {
    let count: u64 = findings
        .iter()
        .chain(kept)
        .map(|row| u64::from(row.count))
        .sum();
    // Never a sentence that calls the text clean: `!suspicious` is not
    // "clean" (A §7.5), and the third shelf says what was not looked for.
    let summary = if findings.is_empty() && kept.is_empty() {
        say(Message::CliReportNone, &args!("source" => source))
    } else if suspicious {
        say(
            Message::CliReportSuspicious,
            &args!("source" => source, "count" => count),
        )
    } else {
        say(
            Message::CliReportNoted,
            &args!("source" => source, "count" => count),
        )
    };
    let mut lines = vec![summary];

    let (remove, replace, keep) = match tense {
        Tense::Would => (
            Message::CliReportWouldRemove,
            Message::CliReportWouldReplace,
            Message::CliReportWouldKeep,
        ),
        Tense::Did => (
            Message::CliReportRemoved,
            Message::CliReportReplaced,
            Message::CliReportKept,
        ),
    };
    let removed: Vec<&UnicodeFinding> = findings
        .iter()
        .filter(|row| options.action_for(row.class) == Action::Remove)
        .collect();
    let replaced: Vec<&UnicodeFinding> = findings
        .iter()
        .filter(|row| options.action_for(row.class) == Action::Replace)
        .collect();
    let kept_rows: Vec<&UnicodeFinding> = kept.iter().collect();
    for (heading, rows) in [(remove, removed), (replace, replaced), (keep, kept_rows)] {
        if rows.is_empty() {
            continue;
        }
        lines.push(say(heading, &FluentArgs::new()));
        lines.extend(
            rows.into_iter()
                .map(|row| format!("  {}", self::row(say, row))),
        );
    }

    // A letter from another script, found and left in place: without
    // `aggressive` the text still counts as marked, and the exit code is
    // 1 beside a result that did not change. Said, so nobody is left to
    // guess why.
    if !options.aggressive && kept.iter().any(|row| row.class == UnicodeClass::Homoglyph) {
        lines.push(say(Message::CliReportHomoglyphsKept, &FluentArgs::new()));
    }
    if encoding != Encoding::Utf8 {
        lines.push(say(
            Message::CliReportOffsets,
            &args!("encoding" => encoding.name()),
        ));
    }
    lines
}

/// One row: the character, what it is, how sure, how often and where.
fn row(say: Say, row: &UnicodeFinding) -> String {
    let mut character = format!("U+{:04X}", u32::from(row.codepoint));
    if let Some(name) = wipemark_core::name_of(row.codepoint) {
        let _ = write!(character, " {name}");
    }
    let spelled = |offsets: &[usize]| {
        offsets
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    };
    // Offsets as strings, never as Fluent numbers: a number is grouped
    // by the locale, and "1,234" is not a byte offset anybody can use.
    let positions = if row.positions.len() > SHOWN {
        say(
            Message::CliReportMore,
            &args!(
                "shown" => spelled(&row.positions[..SHOWN]),
                "more" => (row.positions.len() - SHOWN).to_string(),
            ),
        )
    } else {
        spelled(&row.positions)
    };
    say(
        Message::CliReportRow,
        &args!(
            "character" => character,
            "class" => say(class_label(row.class), &FluentArgs::new()),
            "confidence" => say(confidence_label(row.confidence), &FluentArgs::new()),
            "count" => row.count,
            "positions" => positions,
        ),
    )
}

/// The Unicode version, then the third shelf — the title and one line per
/// entry of `not_established::ALL`, in its order.
fn footer(say: Say, version: &str) -> Vec<String> {
    let mut lines = vec![
        say(Message::CliReportUnicode, &args!("version" => version)),
        say(Message::ReportNotEstablishedTitle, &FluentArgs::new()),
    ];
    lines.extend(
        not_established::ALL
            .iter()
            .map(|(id, canonical)| format!("  - {}", shelf_line(say, id, canonical))),
    );
    lines
}

/// A class's words. Exhaustive, so a twelfth class does not compile here
/// until it has a key — as it does not pass the i18n gate.
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

/// One entry of the third shelf in the reader's language. An id this
/// build has no key for — a fourth entry added to core first — still
/// prints, as the canonical English beside its id: **an entry is never
/// dropped**, and the i18n suite goes red for it anyway.
fn shelf_line(say: Say, id: &str, canonical: &str) -> String {
    let message = match id {
        "vendor-detector-evasion" => Message::ReportNotEstablishedVendorDetectorEvasion,
        "human-authorship" => Message::ReportNotEstablishedHumanAuthorship,
        "unknown-mark-schemes" => Message::ReportNotEstablishedUnknownMarkSchemes,
        _ => return format!("{canonical} ({id})"),
    };
    say(message, &FluentArgs::new())
}

#[cfg(test)]
mod tests {
    use wipemark_core::report::not_established;
    use wipemark_core::{Confidence, Options, UnicodeClass};
    use wipemark_i18n::{FluentArgs, Localizer, Message, Rendering};
    use wipemark_intake::Encoding;

    use super::{class_label, clean_lines, confidence_label, inspect_lines, shelf_line};

    const CONFIDENCES: [Confidence; 4] = [
        Confidence::LikelyFalsePositive,
        Confidence::Informational,
        Confidence::Probable,
        Confidence::Confirmed,
    ];

    /// One localizer per shipped language, in plain text — the CLI's
    /// rendering — built per test rather than through the process-wide
    /// `init`, which would race the other tests.
    fn localizers(rendering: Rendering) -> Vec<Localizer> {
        wipemark_i18n::available_languages()
            .iter()
            .map(|language| Localizer::for_languages(std::slice::from_ref(&language.id), rendering))
            .collect()
    }

    /// A label is the key its id names, so the gate in `wipemark-i18n`
    /// (which checks the keys by id) is checking what this file prints.
    #[test]
    fn every_class_and_confidence_label_is_its_own_key() {
        for class in UnicodeClass::ALL {
            assert_eq!(
                class_label(class).id(),
                format!("unicode-class-{}", class.as_str()),
                "{class:?}"
            );
        }
        for confidence in CONFIDENCES {
            assert_eq!(
                confidence_label(confidence).id(),
                format!("confidence-{}", confidence.as_str()),
                "{confidence:?}"
            );
        }
    }

    /// Every entry of the third shelf reads in the reader's language,
    /// not in the canonical English the fallback prints.
    #[test]
    fn every_third_shelf_item_is_rendered_from_the_catalogue() {
        for localizer in localizers(Rendering::PlainText) {
            let say = |message: Message, args: &FluentArgs| localizer.format_args(message, args);
            for (id, canonical) in not_established::ALL {
                let key = format!("report-not-established-{id}");
                let message = Message::ALL
                    .iter()
                    .find(|message| message.id() == key)
                    .expect("a catalogue key");
                let line = shelf_line(&say, id, canonical);
                assert_eq!(line, localizer.format(*message), "{id}");
                assert!(!line.contains(id), "{id}: fell back to the id");
            }
        }
    }

    /// The product's own rule, turned on its own report: a line printed
    /// into a terminal, a file or a hook's log must carry nothing Layer A
    /// would strip, in any language — Fluent's isolates above all, and
    /// the grouping space a future number formatter would put into 1234.
    #[test]
    fn the_human_report_carries_no_character_layer_a_would_strip() {
        let mut text = "a\u{200B}".repeat(1234);
        text.push_str(" b\u{00A0}c p\u{0430}y\n");
        let gentle = Options::default();
        let aggressive = Options {
            aggressive: true,
            nfkc: true,
            ..Options::default()
        };
        let inspected = wipemark_core::inspect(&text, &gentle);
        assert!(inspected.findings[0].count == 1234, "the fixture moved");

        for localizer in localizers(Rendering::PlainText) {
            let say = |message: Message, args: &FluentArgs| localizer.format_args(message, args);
            let mut lines = inspect_lines(&say, "note.md", &inspected, &gentle, Encoding::Utf16Le);
            for options in [&gentle, &aggressive] {
                let cleaned = wipemark_core::clean(&text, options);
                lines.extend(clean_lines(
                    &say,
                    "note.md",
                    &cleaned.report,
                    options,
                    Encoding::Utf8,
                    Some("note.cleaned.md"),
                    true,
                ));
            }
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

    /// What a homoglyph left in place says: found, not replaced — the
    /// line that explains an exit code of 1 beside an unchanged result.
    /// And with `aggressive`, where it was replaced, it is not said.
    #[test]
    fn a_homoglyph_left_in_place_is_said_to_be_found_and_not_replaced() {
        let english = &localizers(Rendering::PlainText)
            .into_iter()
            .find(|localizer| localizer.language() == "en-US")
            .expect("the fallback");
        let say = |message: Message, args: &FluentArgs| english.format_args(message, args);
        let note = english.format(Message::CliReportHomoglyphsKept);
        let text = "p\u{0430}y";

        let gentle = Options::default();
        let cleaned = wipemark_core::clean(text, &gentle);
        assert_eq!(cleaned.text, text, "nothing replaced without aggressive");
        assert!(cleaned.report.suspicious);
        let lines = clean_lines(
            &say,
            "x",
            &cleaned.report,
            &gentle,
            Encoding::Utf8,
            None,
            false,
        );
        assert!(lines.contains(&note), "{lines:#?}");
        let lines = inspect_lines(
            &say,
            "x",
            &wipemark_core::inspect(text, &gentle),
            &gentle,
            Encoding::Utf8,
        );
        assert!(lines.contains(&note), "{lines:#?}");

        let aggressive = Options {
            aggressive: true,
            ..Options::default()
        };
        let cleaned = wipemark_core::clean(text, &aggressive);
        assert_ne!(cleaned.text, text);
        let lines = clean_lines(
            &say,
            "x",
            &cleaned.report,
            &aggressive,
            Encoding::Utf8,
            None,
            false,
        );
        assert!(!lines.contains(&note), "{lines:#?}");
    }

    /// Under `nfkc` a row can list as kept a character a later pass
    /// removed — five variation selectors kept, two of them orphaned by
    /// NFKC and removed (E1-3's `survive-emoji-presentation.txt`). The
    /// report says how many, so "kept 5" beside a result with three is
    /// not a contradiction left for the reader.
    #[test]
    fn what_the_passes_after_nfkc_did_is_counted() {
        let english = &localizers(Rendering::PlainText)
            .into_iter()
            .find(|localizer| localizer.language() == "en-US")
            .expect("the fallback");
        let say = |message: Message, args: &FluentArgs| english.format_args(message, args);
        let text = include_str!("../../../fixtures/text/survive-emoji-presentation.txt");
        let options = Options {
            nfkc: true,
            ..Options::default()
        };
        let cleaned = wipemark_core::clean(text, &options);
        let lines = clean_lines(
            &say,
            "x",
            &cleaned.report,
            &options,
            Encoding::Utf8,
            None,
            false,
        );
        let later =
            english.format_args(Message::CliCleanLater, &wipemark_i18n::args!("count" => 2));
        assert!(lines.contains(&later), "{lines:#?}");

        let lines = clean_lines(
            &say,
            "x",
            &wipemark_core::clean(text, &Options::default()).report,
            &Options::default(),
            Encoding::Utf8,
            None,
            false,
        );
        assert!(
            !lines.iter().any(|line| line.contains("NFKC")),
            "{lines:#?}"
        );
    }
}
