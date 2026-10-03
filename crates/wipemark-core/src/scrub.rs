//! The scrubber: one decision over one pass, for `inspect` and `clean`.
//!
//! A run is: the hits ([`collect_hits`] — E1-2's context pass and E1-4's
//! homoglyphs), one [`decide`] per hit, the decisions aggregated into
//! report rows and counters, and — for `clean` — the output text, built
//! in the same walk. `inspect` is the same walk without
//! the output (A §5.2: an `inspect` that saw something other than what
//! `clean` removes would be a lie in the report). With `Options::nfkc`,
//! `clean` then runs NFKC and the pass again, in rounds until a pass acts
//! on nothing (D26); those rounds count and never position, because their
//! text is NFKC output and not the source.

use std::cmp::Reverse;
use std::collections::BTreeMap;

use crate::class::{Action, Confidence, UnicodeClass, UnicodeFinding};
use crate::context::{self, Hit};
use crate::report::{CleanReport, InspectReport, NormKind, TextStats};
use crate::tables::UNICODE_VERSION;
use crate::{homoglyph, nfkc, Cleaned, Options};

/// How many times `clean` may run NFKC and the pass again (D26). Two
/// rounds are the most any input in the corpus needs (E1-3 §5.4); the cap
/// turns a future context rule that breaks convergence into a failed
/// debug assertion, never a hung MCP thread.
const MAX_ROUNDS: usize = 8;

/// Index of each kind in [`Pass::normalized`], which is also the order
/// `CleanReport::normalized` lists them in.
const NORM_KINDS: [NormKind; 3] = [NormKind::SpaceToAscii, NormKind::Nfkc, NormKind::Homoglyph];
const SPACE_TO_ASCII: usize = 0;
const NFKC: usize = 1;
const HOMOGLYPH: usize = 2;

/// Every hit in `text`, in source order: the context hits (E1-2) merged
/// with the homoglyph hits (E1-4). The one place a detector is wired in.
pub(crate) fn collect_hits(text: &str, _options: &Options) -> Vec<Hit> {
    // Homoglyphs run whatever the `Options` say (D3); `aggressive` only
    // changes the action.
    merge_in_source_order(context::hits(text), homoglyph::hits(text))
}

/// Two hit lists, each in source order, merged into one in source order.
/// A linear merge by `at`; the two detectors never claim the same code
/// point (a homoglyph source is a letter, a context hit never is), and a
/// debug assertion says so.
fn merge_in_source_order(a: Vec<Hit>, b: Vec<Hit>) -> Vec<Hit> {
    let merged = if b.is_empty() {
        a
    } else if a.is_empty() {
        b
    } else {
        let mut merged = Vec::with_capacity(a.len() + b.len());
        let mut a = a.into_iter().peekable();
        let mut b = b.into_iter().peekable();
        loop {
            let take_a = match (a.peek(), b.peek()) {
                (Some(x), Some(y)) => x.at <= y.at,
                (Some(_), None) => true,
                (None, Some(_)) => false,
                (None, None) => break,
            };
            let next = if take_a { a.next() } else { b.next() };
            merged.extend(next);
        }
        merged
    };
    debug_assert!(
        merged.windows(2).all(|w| w[0].at < w[1].at),
        "hits are not strictly increasing in `at` — two detectors claimed one code point"
    );
    merged
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decision {
    Keep,
    Remove,
    Replace(char),
}

/// What happens to one hit. The only place a decision is made.
fn decide(hit: &Hit, options: &Options) -> Decision {
    // A §5.2: context beats every knob — a ZWJ inside a family emoji stays
    // whatever the Options say.
    if hit.kept_by_context {
        return Decision::Keep;
    }
    match options.action_for(hit.class) {
        Action::Keep => Decision::Keep,
        Action::Remove => Decision::Remove,
        // A letter is never deleted for want of a replacement: a homoglyph
        // hit without one is kept (and reported in `kept`).
        Action::Replace => replacement(hit).map_or(Decision::Keep, Decision::Replace),
    }
}

/// The equivalent the tables name (A §5.1).
fn replacement(hit: &Hit) -> Option<char> {
    match hit.class {
        UnicodeClass::ExoticSpace => Some(' '),
        UnicodeClass::Homoglyph => hit.replacement,
        // `action_for` never answers `Replace` for these; listed one by one
        // so that a new class is a compile error here, not a silent `None`.
        UnicodeClass::ZeroWidth
        | UnicodeClass::ZeroWidthJoiner
        | UnicodeClass::BidiControl
        | UnicodeClass::TagCharacter
        | UnicodeClass::VariationSelector
        | UnicodeClass::SoftHyphen
        | UnicodeClass::Noncharacter
        | UnicodeClass::PrivateUse
        | UnicodeClass::DefaultIgnorable => None,
    }
}

/// The position of `class` in [`UnicodeClass::ALL`] — the report's sort
/// key and the index of `removed`, so there is no second ordering table.
fn class_index(class: UnicodeClass) -> usize {
    UnicodeClass::ALL
        .iter()
        .position(|c| *c == class)
        .unwrap_or(UnicodeClass::ALL.len())
}

/// A row's key, ordered exactly as the report must be: kept rows before
/// acted ones (split afterwards), then class in `ALL` order, code point
/// ascending, confidence descending. `acted` is in the key (D6) so that an
/// acted row and a kept row never merge.
type RowKey = (bool, usize, char, Reverse<Confidence>);

struct Pass {
    findings: Vec<UnicodeFinding>,
    kept: Vec<UnicodeFinding>,
    /// Indexed by position in `UnicodeClass::ALL`.
    removed: [u32; 11],
    /// Indexed `SpaceToAscii`, `Nfkc`, `Homoglyph`; `Nfkc` is never
    /// counted here.
    normalized: [u32; 3],
    /// `Remove` and `Replace` decisions.
    acted: u32,
    /// `Some` only when asked to build.
    output: Option<String>,
}

/// Decide every hit, aggregate the rows, count, and — when `build` —
/// produce the output text. `inspect` passes `build: false`; nothing else
/// differs between the two callers.
fn run(text: &str, hits: &[Hit], options: &Options, build: bool) -> Pass {
    let mut out = build.then(|| String::with_capacity(text.len()));
    let mut copied = 0;
    let mut rows: BTreeMap<RowKey, Vec<usize>> = BTreeMap::new();
    let mut removed = [0u32; 11];
    let mut normalized = [0u32; 3];
    let mut acted = 0u32;

    for hit in hits {
        debug_assert!(
            hit.at >= copied && text[hit.at..].starts_with(hit.c),
            "a hit at {} does not name its code point in the text",
            hit.at
        );
        let decision = decide(hit, options);
        let index = class_index(hit.class);
        rows.entry((
            decision != Decision::Keep,
            index,
            hit.c,
            Reverse(hit.confidence),
        ))
        .or_default()
        .push(hit.at);
        match decision {
            Decision::Remove => {
                removed[index] = removed[index].saturating_add(1);
                acted = acted.saturating_add(1);
            }
            Decision::Replace(_) => {
                let kind = match hit.class {
                    UnicodeClass::Homoglyph => HOMOGLYPH,
                    _ => SPACE_TO_ASCII,
                };
                normalized[kind] = normalized[kind].saturating_add(1);
                acted = acted.saturating_add(1);
            }
            Decision::Keep => {}
        }
        if let Some(out) = out.as_mut() {
            out.push_str(&text[copied..hit.at]);
            match decision {
                Decision::Keep => out.push(hit.c),
                Decision::Remove => {}
                Decision::Replace(r) => out.push(r),
            }
            copied = hit.at + hit.c.len_utf8();
        }
    }
    if let Some(out) = out.as_mut() {
        out.push_str(&text[copied..]);
    }

    let mut findings = Vec::new();
    let mut kept = Vec::new();
    for ((is_acted, index, codepoint, Reverse(confidence)), positions) in rows {
        let row = UnicodeFinding {
            codepoint,
            class: UnicodeClass::ALL[index],
            count: u32::try_from(positions.len()).unwrap_or(u32::MAX),
            positions,
            confidence,
        };
        if is_acted {
            findings.push(row);
        } else {
            kept.push(row);
        }
    }

    Pass {
        findings,
        kept,
        removed,
        normalized,
        acted,
        output: out,
    }
}

/// D4: some row, acted on or kept, is at least Probable. Kept-by-context
/// rows are LikelyFalsePositive and knob-kept exotic spaces Informational,
/// so neither counts; a homoglyph kept without `aggressive` (Probable)
/// does — that is the case a user must be told about.
fn is_suspicious(findings: &[UnicodeFinding], kept: &[UnicodeFinding]) -> bool {
    findings
        .iter()
        .chain(kept)
        .any(|row| row.confidence >= Confidence::Probable)
}

pub(crate) fn inspect(text: &str, options: &Options) -> InspectReport {
    let hits = collect_hits(text, options);
    let pass = run(text, &hits, options, false);
    InspectReport {
        suspicious: is_suspicious(&pass.findings, &pass.kept),
        findings: pass.findings,
        kept: pass.kept,
        stats: TextStats::of(text), // once per call (A §5.5)
        unicode_version: UNICODE_VERSION,
    }
}

pub(crate) fn clean(text: &str, options: &Options) -> Cleaned {
    // The rows of the report come from this pass and from no other.
    let first = run(text, &collect_hits(text, options), options, true);
    let mut removed = first.removed;
    let mut normalized = first.normalized;
    let mut out = first.output.unwrap_or_default();

    if options.nfkc {
        let mut nfkc_changed = 0u32;
        let mut converged = false;
        for _ in 0..MAX_ROUNDS {
            let (normal, changed) = nfkc::nfkc_counted(&out);
            nfkc_changed = nfkc_changed.saturating_add(changed);
            let again = run(&normal, &collect_hits(&normal, options), options, true);
            // Counts only: the rows of `again` are positioned in NFKC
            // output, not in the source (A §5.3), and are dropped.
            for (total, n) in removed.iter_mut().zip(again.removed) {
                *total = total.saturating_add(n);
            }
            for kind in [SPACE_TO_ASCII, HOMOGLYPH] {
                normalized[kind] = normalized[kind].saturating_add(again.normalized[kind]);
            }
            out = again.output.unwrap_or_default();
            if again.acted == 0 {
                converged = true;
                break;
            }
        }
        debug_assert!(
            converged,
            "NFKC and the pass did not converge in {MAX_ROUNDS} rounds"
        );
        normalized[NFKC] = nfkc_changed;
    }

    let report = CleanReport {
        suspicious: is_suspicious(&first.findings, &first.kept),
        stats: TextStats::of(text),
        findings: first.findings,
        kept: first.kept,
        removed: UnicodeClass::ALL
            .iter()
            .zip(removed)
            .filter(|&(_, n)| n > 0)
            .map(|(&class, n)| (class, n))
            .collect(),
        normalized: NORM_KINDS
            .iter()
            .zip(normalized)
            .filter(|&(&kind, n)| n > 0 || (kind == NormKind::Nfkc && options.nfkc))
            .map(|(&kind, n)| (kind, n))
            .collect(),
        output_len: out.len(),
        unicode_version: UNICODE_VERSION,
    };
    Cleaned { text: out, report }
}

#[cfg(test)]
mod tests {
    use super::{is_suspicious, merge_in_source_order, run, HOMOGLYPH, SPACE_TO_ASCII};
    use crate::class::{Confidence, UnicodeClass, UnicodeFinding};
    use crate::context::Hit;
    use crate::report::NormKind;
    use crate::{clean, inspect, Options};

    fn hex(s: &str) -> String {
        s.chars()
            .map(|c| format!("U+{:04X}", u32::from(c)))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn every_options() -> Vec<Options> {
        (0u8..16)
            .map(|bits| Options {
                aggressive: bits & 1 != 0,
                nfkc: bits & 2 != 0,
                normalize_spaces: bits & 4 != 0,
                keep_soft_hyphen: bits & 8 != 0,
            })
            .collect()
    }

    fn row(
        codepoint: char,
        class: UnicodeClass,
        confidence: Confidence,
        positions: &[usize],
    ) -> UnicodeFinding {
        UnicodeFinding {
            codepoint,
            class,
            count: u32::try_from(positions.len()).expect("small"),
            positions: positions.to_vec(),
            confidence,
        }
    }

    fn homoglyph(at: usize, replacement: Option<char>) -> Hit {
        Hit {
            at,
            c: '\u{430}',
            class: UnicodeClass::Homoglyph,
            confidence: Confidence::Probable,
            kept_by_context: false,
            replacement,
        }
    }

    fn nfkc() -> Options {
        Options {
            nfkc: true,
            ..Options::default()
        }
    }

    #[test]
    fn positions_are_byte_offsets_into_the_source() {
        let text = "\u{416}\u{20AC}\u{10348}\u{200B}x\u{200B}";
        let expected = vec![row(
            '\u{200B}',
            UnicodeClass::ZeroWidth,
            Confidence::Confirmed,
            &[9, 13],
        )];
        assert_eq!(inspect(text, &Options::default()).findings, expected);
        let cleaned = clean(text, &Options::default());
        assert_eq!(cleaned.report.findings, expected);
        assert_eq!(hex(&cleaned.text), hex("\u{416}\u{20AC}\u{10348}x"));
    }

    #[test]
    fn soft_hyphens_alone_are_not_suspicious() {
        let text = "co\u{AD}op";
        let expected = vec![row(
            '\u{AD}',
            UnicodeClass::SoftHyphen,
            Confidence::Informational,
            &[2],
        )];
        let inspected = inspect(text, &Options::default());
        assert_eq!(inspected.findings, expected);
        assert!(!inspected.suspicious);
        let cleaned = clean(text, &Options::default());
        assert_eq!(cleaned.report.findings, expected);
        assert!(!cleaned.report.suspicious);
    }

    #[test]
    fn a_single_zero_width_space_is() {
        assert!(inspect("a\u{200B}b", &Options::default()).suspicious);
        assert!(clean("a\u{200B}b", &Options::default()).report.suspicious);
    }

    #[test]
    fn orthography_alone_is_not_suspicious() {
        let text = "\u{1F469}\u{200D}\u{1F467}";
        let kept = vec![row(
            '\u{200D}',
            UnicodeClass::ZeroWidthJoiner,
            Confidence::LikelyFalsePositive,
            &[4],
        )];
        let inspected = inspect(text, &Options::default());
        assert!(inspected.findings.is_empty());
        assert_eq!(inspected.kept, kept);
        assert!(!inspected.suspicious);
        let cleaned = clean(text, &Options::default());
        assert!(cleaned.report.findings.is_empty());
        assert_eq!(cleaned.report.kept, kept);
        assert!(!cleaned.report.suspicious);
    }

    #[test]
    fn a_kept_homoglyph_makes_the_text_suspicious() {
        let text = "p\u{430}y";
        let pass = run(text, &[homoglyph(1, Some('a'))], &Options::default(), true);
        assert!(pass.findings.is_empty());
        assert_eq!(
            pass.kept,
            vec![row(
                '\u{430}',
                UnicodeClass::Homoglyph,
                Confidence::Probable,
                &[1]
            )]
        );
        assert!(is_suspicious(&pass.findings, &pass.kept));
        assert_eq!(pass.output.as_deref(), Some(text));
    }

    #[test]
    fn a_kept_and_a_removed_joiner_are_two_rows() {
        let text = "\u{2764}\u{FE0F}\u{200D}\u{1F525} a\u{200D}b";
        let cleaned = clean(text, &Options::default());
        assert_eq!(
            cleaned.report.findings,
            vec![row(
                '\u{200D}',
                UnicodeClass::ZeroWidthJoiner,
                Confidence::Probable,
                &[15]
            )]
        );
        assert_eq!(
            cleaned.report.kept,
            vec![
                row(
                    '\u{200D}',
                    UnicodeClass::ZeroWidthJoiner,
                    Confidence::LikelyFalsePositive,
                    &[6]
                ),
                row(
                    '\u{FE0F}',
                    UnicodeClass::VariationSelector,
                    Confidence::LikelyFalsePositive,
                    &[3]
                ),
            ]
        );
        assert_eq!(
            hex(&cleaned.text),
            hex("\u{2764}\u{FE0F}\u{200D}\u{1F525} ab")
        );
        assert_eq!(cleaned.text.len(), 16);
    }

    #[test]
    fn acted_and_kept_rows_never_merge() {
        let aggressive = Options {
            aggressive: true,
            ..Options::default()
        };
        let pass = run(
            "\u{430}\u{430}",
            &[homoglyph(0, Some('a')), homoglyph(2, None)],
            &aggressive,
            true,
        );
        let at = |p: usize| {
            row(
                '\u{430}',
                UnicodeClass::Homoglyph,
                Confidence::Probable,
                &[p],
            )
        };
        assert_eq!(pass.findings, vec![at(0)]);
        assert_eq!(pass.kept, vec![at(2)]);
        assert_eq!(pass.output.as_deref(), Some("a\u{430}"));
        assert_eq!(pass.normalized[HOMOGLYPH], 1);
    }

    #[test]
    fn context_beats_every_knob() {
        let texts = [
            "\u{1F469}\u{200D}\u{1F467}",
            "\u{2764}\u{FE0F}",
            "\u{645}\u{200C}\u{631}",
            "\u{1F3F4}\u{E0067}\u{E0062}\u{E0073}\u{E0063}\u{E0074}\u{E007F}",
        ];
        for text in texts {
            let baseline = clean(text, &Options::default()).report.kept;
            assert!(!baseline.is_empty(), "{}", hex(text));
            for options in every_options() {
                let cleaned = clean(text, &options);
                assert_eq!(hex(&cleaned.text), hex(text), "{options:?}");
                assert_eq!(cleaned.report.kept, baseline, "{} {options:?}", hex(text));
            }
        }
    }

    #[test]
    fn keep_soft_hyphen_keeps_soft_hyphens() {
        let text = "co\u{AD}op";
        let shy = vec![row(
            '\u{AD}',
            UnicodeClass::SoftHyphen,
            Confidence::Informational,
            &[2],
        )];
        let removed = clean(text, &Options::default());
        assert_eq!(removed.text, "coop");
        assert_eq!(removed.report.findings, shy);
        assert_eq!(removed.report.removed, vec![(UnicodeClass::SoftHyphen, 1)]);
        let kept = clean(
            text,
            &Options {
                keep_soft_hyphen: true,
                ..Options::default()
            },
        );
        assert_eq!(kept.text, text);
        assert_eq!(kept.report.kept, shy);
        assert!(kept.report.findings.is_empty());
        assert!(kept.report.removed.is_empty());
    }

    #[test]
    fn normalize_spaces_replaces_exotic_spaces_with_a_space() {
        let text = "a\u{A0}b\u{3000}c";
        let rows = vec![
            row(
                '\u{A0}',
                UnicodeClass::ExoticSpace,
                Confidence::Informational,
                &[1],
            ),
            row(
                '\u{3000}',
                UnicodeClass::ExoticSpace,
                Confidence::Informational,
                &[4],
            ),
        ];
        let default = clean(text, &Options::default());
        assert_eq!(default.text, text);
        assert_eq!(default.report.kept, rows);
        assert!(default.report.normalized.is_empty());
        let spaced = clean(
            text,
            &Options {
                normalize_spaces: true,
                ..Options::default()
            },
        );
        assert_eq!(spaced.text, "a b c");
        assert_eq!(spaced.report.findings, rows);
        assert_eq!(spaced.report.normalized, vec![(NormKind::SpaceToAscii, 2)]);
        assert!(spaced.report.removed.is_empty());
    }

    #[test]
    fn aggressive_replaces_a_homoglyph_with_its_letter() {
        let pass = run(
            "p\u{430}y",
            &[homoglyph(1, Some('a'))],
            &Options {
                aggressive: true,
                ..Options::default()
            },
            true,
        );
        assert_eq!(pass.output.as_deref(), Some("pay"));
        assert_eq!(
            pass.findings,
            vec![row(
                '\u{430}',
                UnicodeClass::Homoglyph,
                Confidence::Probable,
                &[1]
            )]
        );
        assert_eq!(pass.normalized[HOMOGLYPH], 1);
        assert_eq!(pass.normalized[SPACE_TO_ASCII], 0);
        assert_eq!(pass.removed, [0; 11]);
    }

    #[test]
    fn nfkc_is_off_unless_asked() {
        let default = clean("\u{FB01}", &Options::default());
        assert_eq!(default.text, "\u{FB01}");
        assert!(default.report.normalized.is_empty());
        let normal = clean("\u{FB01}", &nfkc());
        assert_eq!(normal.text, "fi");
        assert_eq!(normal.report.normalized, vec![(NormKind::Nfkc, 1)]);
    }

    #[test]
    fn nfkc_reports_that_it_ran_even_when_it_changed_nothing() {
        assert_eq!(
            clean("abc", &nfkc()).report.normalized,
            vec![(NormKind::Nfkc, 0)]
        );
        assert!(clean("abc", &Options::default())
            .report
            .normalized
            .is_empty());
    }

    #[test]
    fn nfkc_never_leaves_an_orphaned_selector() {
        let cases = [
            ("\u{2139}\u{FE0F}", "i", 3),
            ("\u{2122}\u{FE0F}", "TM", 3),
            ("\u{3297}\u{FE0F}", "\u{795D}", 3),
            ("\u{1F202}\u{FE0F}", "\u{30B5}", 4),
        ];
        for (text, expected, at) in cases {
            let cleaned = clean(text, &nfkc());
            assert_eq!(hex(&cleaned.text), hex(expected), "{}", hex(text));
            assert!(cleaned.report.findings.is_empty(), "{}", hex(text));
            assert_eq!(
                cleaned.report.kept,
                vec![row(
                    '\u{FE0F}',
                    UnicodeClass::VariationSelector,
                    Confidence::LikelyFalsePositive,
                    &[at]
                )],
                "{}",
                hex(text)
            );
            assert_eq!(
                cleaned.report.removed,
                vec![(UnicodeClass::VariationSelector, 1)],
                "{}",
                hex(text)
            );
            assert_eq!(
                cleaned.report.normalized,
                vec![(NormKind::Nfkc, 1)],
                "{}",
                hex(text)
            );
            assert_eq!(clean(&cleaned.text, &nfkc()).text, cleaned.text);
        }
    }

    #[test]
    fn nfkc_rounds_reach_a_fixed_point() {
        let cleaned = clean("\u{2139}\u{FE0F}\u{301}", &nfkc());
        assert_eq!(hex(&cleaned.text), hex("\u{ED}"));
        assert_eq!(
            cleaned.report.removed,
            vec![(UnicodeClass::VariationSelector, 1)]
        );
        assert_eq!(cleaned.report.normalized, vec![(NormKind::Nfkc, 3)]);
        let again = clean(&cleaned.text, &nfkc());
        assert_eq!(hex(&again.text), hex("\u{ED}"));
        assert_eq!(again.report.normalized, vec![(NormKind::Nfkc, 0)]);
    }

    #[test]
    fn inspect_never_reads_the_nfkc_knob() {
        assert_eq!(
            inspect("\u{2139}\u{FE0F}", &nfkc()),
            inspect("\u{2139}\u{FE0F}", &Options::default())
        );
    }

    #[test]
    fn counters_agree_with_positions_without_nfkc() {
        let text = "a\u{200B}b\u{AD}c\u{A0}d\u{200D}e";
        for options in every_options().into_iter().filter(|o| !o.nfkc) {
            let report = clean(text, &options).report;
            for class in UnicodeClass::ALL {
                let rows: u32 = report
                    .findings
                    .iter()
                    .filter(|r| r.class == class)
                    .map(|r| r.count)
                    .sum();
                let removed = report
                    .removed
                    .iter()
                    .find(|(c, _)| *c == class)
                    .map_or(0, |&(_, n)| n);
                if class == UnicodeClass::ExoticSpace {
                    let spaced = report
                        .normalized
                        .iter()
                        .find(|(k, _)| *k == NormKind::SpaceToAscii)
                        .map_or(0, |&(_, n)| n);
                    assert_eq!(spaced, rows, "{options:?}");
                    assert_eq!(removed, 0, "{options:?}");
                } else {
                    assert_eq!(removed, rows, "{options:?} {class:?}");
                }
            }
        }
    }

    #[test]
    fn output_len_is_bytes() {
        let cleaned = clean("\u{E9}\u{200B}", &Options::default());
        assert_eq!(cleaned.text, "\u{E9}");
        assert_eq!(cleaned.report.output_len, 2);
    }

    #[test]
    fn an_empty_text_is_an_empty_report() {
        let inspected = inspect("", &Options::default());
        assert!(inspected.findings.is_empty() && inspected.kept.is_empty());
        assert!(!inspected.suspicious);
        let cleaned = clean("", &Options::default());
        assert_eq!(cleaned.text, "");
        assert_eq!(cleaned.report.output_len, 0);
        assert!(cleaned.report.removed.is_empty());
        assert!(cleaned.report.normalized.is_empty());
        assert_eq!(
            clean("", &nfkc()).report.normalized,
            vec![(NormKind::Nfkc, 0)]
        );
    }

    #[test]
    fn merging_keeps_source_order() {
        let at = |p: usize| Hit {
            at: p,
            c: '\u{200B}',
            class: UnicodeClass::ZeroWidth,
            confidence: Confidence::Confirmed,
            kept_by_context: false,
            replacement: None,
        };
        let merged = merge_in_source_order(vec![at(0), at(5)], vec![at(2), at(9)]);
        assert_eq!(
            merged.iter().map(|h| h.at).collect::<Vec<_>>(),
            vec![0, 2, 5, 9]
        );
    }
}
