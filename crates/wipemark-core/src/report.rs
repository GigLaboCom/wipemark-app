//! Report types — the product's honesty contract in Rust.
//!
//! Spec §0.1 rule 3: every result is filed on exactly three shelves.
//!
//! * **verifiable** — [`CleanReport`]: counted, positioned, reproducible.
//! * **best-effort** — [`RewriteSummary`]: a model rewrote the text and
//!   these are the scores we can actually compute.
//! * **not established** — [`FinalReport::not_established`]: claims we
//!   deliberately refuse to make.
//!
//! The third shelf is a fixed list of strings rather than free text so
//! that no future code path can quietly shrink it. Nothing in the UI or
//! marketing says "undetectable".

use crate::class::{UnicodeClass, UnicodeFinding};

/// Claims the product never makes. Pinned as constants so they read the
/// same in the GUI, in `--json`, and in an exported Markdown report.
pub mod not_established {
    /// Whether a vendor's own detector still flags the text.
    pub const VENDOR_DETECTOR_EVASION: &str =
        "evasion of a vendor's own detector — not tested, no oracle exists here";
    /// Whether a reader or a classifier will call the result human.
    pub const HUMAN_AUTHORSHIP: &str =
        "human authorship — not established by any check in this tool";
    /// Whether a mark survives in a form we do not parse.
    pub const UNKNOWN_MARK_SCHEMES: &str =
        "marks in schemes this build does not implement — not searched for";

    /// The whole shelf, as `(id, canonical English)`, in the order a
    /// report lists it.
    ///
    /// The id is a format — `--json` emits it, and `wipemark-i18n` keys
    /// `report-not-established-<id>` off it — so it is renamed with the
    /// same care as a config key. The English beside it is the
    /// locale-neutral canon, not a label: the translations live in the
    /// catalogue, and the i18n suite goes red for an entry here that has
    /// none. That is what stops the shelf from quietly emptying in every
    /// language but this one.
    pub const ALL: [(&str, &str); 3] = [
        ("vendor-detector-evasion", VENDOR_DETECTOR_EVASION),
        ("human-authorship", HUMAN_AUTHORSHIP),
        ("unknown-mark-schemes", UNKNOWN_MARK_SCHEMES),
    ];
}

/// Cheap document statistics, used for chunk budgeting, language
/// detection and the script guard.
///
/// Computed by [`TextStats::of`]; definitions there. Whitespace is
/// `White_Space` of the crate's Unicode version (`gc=Zs`, U+0009–000D,
/// U+0085, U+2028, U+2029), and a token is a maximal run of anything
/// else.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextStats {
    /// Code points, not bytes — a leading BOM included.
    pub chars: usize,
    /// Tokens holding at least one letter or decimal digit; a run of CJK
    /// without spaces is one word.
    pub words: usize,
    /// Latin letters among all letters (`gc=L*`, marks never count), as
    /// a fraction 0.0–1.0; 0.0 when there are no letters.
    pub latin_ratio: f32,
    /// Cyrillic letters among all letters, as a fraction; 0.0 without
    /// letters.
    pub cyrillic_ratio: f32,
    /// Han, Hiragana, Katakana, Hangul and Bopomofo letters among all
    /// letters, as a fraction; 0.0 without letters.
    pub cjk_ratio: f32,
    /// Fence lines (` ``` ` or `~~~` after leading spaces and tabs)
    /// divided by two, rounded down — a budget figure, not a Markdown
    /// parse.
    pub code_blocks: usize,
    /// Tokens containing `://`, one per token.
    pub urls: usize,
}

/// The result of looking without touching.
///
/// `findings` and `kept` are exactly the rows [`crate::clean`] reports for
/// the same text and options: one decision over one pass (A §5.2).
/// Positions are byte offsets into the inspected text.
#[derive(Debug, Clone, PartialEq)]
pub struct InspectReport {
    /// What `clean` would act on: remove, or replace with the equivalent
    /// the tables name.
    pub findings: Vec<UnicodeFinding>,
    /// What `clean` would find and leave — kept by context (orthography or
    /// presentation, at `LikelyFalsePositive`) or by a knob or the class
    /// default (at the class's own confidence).
    pub kept: Vec<UnicodeFinding>,
    /// True when some row in `findings` or `kept` is at least
    /// [`crate::Confidence::Probable`] (D4). Deliberately not "has any row": soft
    /// hyphens and exotic spaces alone do not make a document suspicious,
    /// and neither does orthography kept by context.
    pub suspicious: bool,
    pub stats: TextStats,
    /// The UCD version the tables were generated from. Pinned at build
    /// time and printed in every report — a finding is only meaningful
    /// against a known Unicode version.
    pub unicode_version: &'static str,
}

/// A normalisation the scrubber performed, as opposed to a removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormKind {
    /// An exotic space became U+0020 (`Options::normalize_spaces`).
    SpaceToAscii,
    /// NFKC was applied (`Options::nfkc`). The count is the number of code
    /// points NFKC did not carry through unchanged, over every round (D27).
    Nfkc,
    /// A homoglyph became the letter of its word's script
    /// (`Options::aggressive`). Detection is E1-4; the count is ready.
    Homoglyph,
}

impl NormKind {
    /// Stable identifier for `--json` and MCP. A format, never translated.
    pub fn as_str(self) -> &'static str {
        match self {
            NormKind::SpaceToAscii => "space-to-ascii",
            NormKind::Nfkc => "nfkc",
            NormKind::Homoglyph => "homoglyph",
        }
    }
}

/// Layer A output: exactly what changed, and where.
///
/// `findings` are the code points `clean` acted on — removed, or replaced
/// with the equivalent the tables name — and `kept` the ones it found and
/// left, by context (orthography, presentation) or by a knob. Every entry
/// of `positions` is a **byte offset into the source text handed to
/// `clean`**, never into the output: the Inspector jumps to them in the
/// original. Rows are sorted by class (in [`UnicodeClass::ALL`] order),
/// then code point, then confidence from highest to lowest; `count` is
/// always `positions.len()`.
///
/// `removed` counts removals per class and `normalized` replacements per
/// kind. Without `Options::nfkc` they agree with the rows exactly: a
/// class's count in `removed` is the sum of the `count`s of its rows in
/// `findings`, and likewise `SpaceToAscii` for exotic spaces and
/// `Homoglyph` for homoglyphs. **With `nfkc` they can be larger, and this
/// is the only place where a count exceeds the positions behind it:** NFKC
/// can orphan a code point the first pass kept for its context (U+2139
/// U+FE0F becomes U+0069 U+FE0F), the passes that run after NFKC (in
/// rounds, until one acts on nothing) remove it, and the text they remove
/// it from is NFKC output — no byte offset there names a byte of the
/// source. So the passes after NFKC count and never position. For the same
/// reason a row in `kept` can name a code point that is not in the output
/// when `nfkc` was on.
///
/// `suspicious` and `stats` describe the **source**, by the rules
/// `inspect` uses: `suspicious` is D4 over `findings` and `kept` (so what
/// the passes after NFKC remove never makes a text suspicious), and
/// `stats` is `TextStats::of(source)`, computed once. A `clean` and an
/// `inspect` of the same text and options agree on both.
///
/// `removed` lists only classes with a non-zero count, in
/// [`UnicodeClass::ALL`] order. `normalized` lists `SpaceToAscii`, `Nfkc`,
/// `Homoglyph` in that order, each only when non-zero — except `Nfkc`,
/// which is present whenever `nfkc` was asked for, `0` included, so a
/// reader can tell "ran and changed nothing" from "not asked".
///
/// Byte-exact reversibility is not offered and never will be. What is
/// offered instead is this: counts and positions for every removal.
#[derive(Debug, Clone, PartialEq)]
pub struct CleanReport {
    pub findings: Vec<UnicodeFinding>,
    pub kept: Vec<UnicodeFinding>,
    /// D4 over `findings` and `kept` — the input, not the output (D28).
    pub suspicious: bool,
    /// `TextStats::of` the source text (D28).
    pub stats: TextStats,
    pub removed: Vec<(UnicodeClass, u32)>,
    pub normalized: Vec<(NormKind, u32)>,
    /// Length of the cleaned text, in bytes.
    pub output_len: usize,
    pub unicode_version: &'static str,
}

/// Layer B output: which attempt was chosen and on what evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct RewriteSummary {
    /// Tactic of the chosen attempt (`paraphrase`, `humanize`, …).
    pub tactic: String,
    /// Total attempts made, including rejected ones — the honest
    /// denominator.
    pub attempts: u32,
    pub chosen_round: u8,
    pub chosen_candidate: u8,
    /// 1 − bigram Jaccard against the source chunk.
    pub divergence: f32,
    /// Which scorer decided: `keyed_gumbel` or `divergence`.
    pub scorer: &'static str,
    pub engine_model_id: String,
    pub engine_local: bool,
    /// False when no candidate passed the detector and the best one was
    /// returned anyway. The report must say so.
    pub passed: bool,
}

/// How much of the original signal probably survives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLabel {
    /// Short or predictable text — little room for a statistical mark.
    Lower,
    /// Long, high-entropy text — more room, and rewriting is best-effort.
    Higher,
}

/// What the user is shown when a job finishes.
#[derive(Debug, Clone, PartialEq)]
pub struct FinalReport {
    pub verifiable: CleanReport,
    pub best_effort: Option<RewriteSummary>,
    pub not_established: Vec<&'static str>,
    pub residual_risk: RiskLabel,
}

impl FinalReport {
    /// The two claims that are never established, whatever the job did.
    /// Callers add [`not_established::UNKNOWN_MARK_SCHEMES`] when a
    /// document format was only partially parsed.
    pub fn baseline_not_established() -> Vec<&'static str> {
        vec![
            not_established::VENDOR_DETECTOR_EVASION,
            not_established::HUMAN_AUTHORSHIP,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{not_established, FinalReport, NormKind};

    /// Guarding the honesty contract, not the vec: a build that ships
    /// with an empty third shelf is a build that implies a claim we
    /// refuse to make.
    #[test]
    fn baseline_shelf_is_never_empty() {
        let shelf = FinalReport::baseline_not_established();
        assert!(shelf.contains(&not_established::VENDOR_DETECTOR_EVASION));
        assert!(shelf.contains(&not_established::HUMAN_AUTHORSHIP));
    }

    #[test]
    fn no_claim_promises_undetectability() {
        for claim in FinalReport::baseline_not_established() {
            let lowered = claim.to_lowercase();
            assert!(!lowered.contains("undetectable"), "{claim}");
        }
    }

    #[test]
    fn norm_kind_ids_are_exact() {
        assert_eq!(NormKind::SpaceToAscii.as_str(), "space-to-ascii");
        assert_eq!(NormKind::Nfkc.as_str(), "nfkc");
        assert_eq!(NormKind::Homoglyph.as_str(), "homoglyph");
    }
}
