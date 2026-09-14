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
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextStats {
    pub chars: usize,
    pub words: usize,
    pub latin_ratio: f32,
    pub cyrillic_ratio: f32,
    pub cjk_ratio: f32,
    pub code_blocks: usize,
    pub urls: usize,
}

/// The result of looking without touching.
#[derive(Debug, Clone, PartialEq)]
pub struct InspectReport {
    pub findings: Vec<UnicodeFinding>,
    /// True when at least one finding is worth acting on. Deliberately
    /// not "has any finding": informational classes alone do not make a
    /// document suspicious.
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
    /// An exotic space became U+0020.
    SpaceToAscii,
    /// NFKC was applied (opt-in).
    Nfkc,
}

/// Layer A output: exactly what changed.
///
/// Byte-exact reversibility is not offered and never will be. What is
/// offered instead is this: counts and positions for every removal.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CleanReport {
    pub removed: Vec<(UnicodeClass, u32)>,
    pub normalized: Vec<(NormKind, u32)>,
    pub output_len: usize,
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
    use super::{not_established, FinalReport};

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
}
