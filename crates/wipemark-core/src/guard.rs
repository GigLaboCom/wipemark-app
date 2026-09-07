//! Guards — the rejection criteria a rewritten candidate has to survive.
//!
//! They live in `core` rather than in the pipeline because they are
//! pure text predicates with no engine involved, and because Layer A
//! uses the same definitions when it reports what a clean pass changed.
//!
//! A guard answers one question: *did the rewrite quietly destroy
//! something the user needs?* A model that "improves" a version number,
//! translates a paragraph, or drops a code block has produced a
//! candidate that must be thrown away no matter how good its
//! divergence score is (spec §3.3).

use std::fmt;

/// Why a candidate was thrown away.
///
/// Every variant names the specific thing that was lost — the report
/// shows these verbatim, and "guard failed" would tell the user
/// nothing.
#[derive(Debug, Clone, PartialEq)]
pub enum RejectReason {
    /// A `⟦n⟧` placeholder for a protected span (code, URL, path) did
    /// not come back exactly once.
    PlaceholderMissing { index: usize },
    /// The same placeholder came back more than once.
    PlaceholderDuplicated { index: usize, count: u32 },
    /// A number, date or version present in the source is gone.
    NumberMissing { value: String },
    /// `len(candidate) / len(source)` left the configured window.
    LengthDrift { ratio: f32, min: f32, max: f32 },
    /// The share of a script moved far enough that the model probably
    /// translated instead of rewriting.
    ScriptDrift { script: &'static str, delta_pp: f32 },
    /// An identifier-shaped token (snake_case, CamelCase, path, URL,
    /// e-mail) is gone.
    IdentifierMissing { token: String },
}

impl fmt::Display for RejectReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RejectReason::PlaceholderMissing { index } => {
                write!(f, "protected span ⟦{index}⟧ did not come back")
            }
            RejectReason::PlaceholderDuplicated { index, count } => {
                write!(f, "protected span ⟦{index}⟧ came back {count} times")
            }
            RejectReason::NumberMissing { value } => write!(f, "number or date {value:?} was lost"),
            RejectReason::LengthDrift { ratio, min, max } => {
                write!(f, "length ratio {ratio:.2} outside [{min:.2}, {max:.2}]")
            }
            RejectReason::ScriptDrift { script, delta_pp } => {
                write!(
                    f,
                    "{script} share moved by {delta_pp:.1} pp — looks translated"
                )
            }
            RejectReason::IdentifierMissing { token } => {
                write!(f, "identifier {token:?} was lost")
            }
        }
    }
}

/// The verdict of one guard on one candidate.
#[derive(Debug, Clone, PartialEq)]
pub enum GuardOutcome {
    Pass,
    Reject(RejectReason),
}

impl GuardOutcome {
    pub fn is_pass(&self) -> bool {
        matches!(self, GuardOutcome::Pass)
    }

    /// The reason, when this is a rejection.
    pub fn reason(&self) -> Option<&RejectReason> {
        match self {
            GuardOutcome::Pass => None,
            GuardOutcome::Reject(reason) => Some(reason),
        }
    }
}

/// One rejection criterion.
///
/// Implementations are stateless and cheap: the selection loop runs
/// every guard over every candidate of every round.
pub trait Guard: Send + Sync {
    /// Stable name, shown in the report next to the rejection.
    fn name(&self) -> &'static str;

    /// `source` is the text handed to the engine (placeholders already
    /// substituted), `candidate` is what came back.
    fn check(&self, source: &str, candidate: &str) -> GuardOutcome;
}

#[cfg(test)]
mod tests {
    use super::{GuardOutcome, RejectReason};

    #[test]
    fn outcome_reports_its_reason() {
        let pass = GuardOutcome::Pass;
        assert!(pass.is_pass());
        assert!(pass.reason().is_none());

        let reject = GuardOutcome::Reject(RejectReason::PlaceholderMissing { index: 3 });
        assert!(!reject.is_pass());
        assert!(reject.reason().is_some());
    }

    /// The rendered reason has to name the thing that was lost — this is
    /// what the user reads in the report.
    #[test]
    fn reasons_render_specifically() {
        let reason = RejectReason::NumberMissing {
            value: "1.94.1".to_owned(),
        };
        assert!(reason.to_string().contains("1.94.1"));

        let drift = RejectReason::LengthDrift {
            ratio: 0.31,
            min: 0.6,
            max: 1.6,
        };
        let rendered = drift.to_string();
        assert!(rendered.contains("0.31"), "{rendered}");
    }
}
