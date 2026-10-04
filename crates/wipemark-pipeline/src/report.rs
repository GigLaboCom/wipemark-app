//! What a job did, attempt by attempt — the report a rewrite ends with.
//!
//! Filed on the three shelves (CLAUDE.md, "The third shelf is never
//! empty"): Layer A before and after the model is **verifiable** — counted
//! and positioned by `wipemark-core`; everything the model did is
//! **best-effort** — which tactic, which seed, which template, what every
//! attempt was rejected for, and which candidate won by how much; and
//! **not established** is never empty.
//!
//! Everything here is a value. A rejection names what was lost with the
//! fields a surface needs to render it from its catalogue; no sentence is
//! written here (only applications localize). The JSON form
//! ([`JobReport::to_json`]) is ASCII: every character from U+007F up is a
//! `\u` escape, as in core's A §7.1 writer, because a value from the
//! document — a number a guard missed, an identifier — must not carry an
//! invisible character into whatever reads the report.

use std::time::Duration;

use serde::{Serialize, Serializer};
use serde_json::{json, Map, Value};
use wipemark_core::report::not_established;
use wipemark_core::{CleanReport, RejectReason};
use wipemark_engine::{EngineError, EngineInfo, FinishReason};

use crate::cost::Effort;
use crate::lang::Lang;
use crate::prepare::{RestoreError, TextFormat};
use crate::prompt::{row, Intensity, Marker, Problem, Refusal, Slot, Stripped, Tactic, Version};
use crate::select::{Scorer, Scores, RULES};

/// The version of the JSON form. A format: bumped when a field changes
/// meaning or goes away, not when one is added. 2 (D95): `passed.score`
/// is what the winner *maximises*, a restore's `item-broken` carries no
/// `item` and `out-of-order` is gone; `language` rejections and the
/// `selection` block are new.
pub const REPORT_VERSION: u32 = 2;

/// Why one candidate was thrown away. A candidate rejected for any of
/// these is never used — not as a fallback, not as "the best of the bad".
///
/// Exhaustive on purpose, like [`RejectReason`]: every surface renders
/// each variant from its catalogue, and a new one should break their
/// build rather than fall into a wildcard arm that says nothing.
#[derive(Debug, Clone, PartialEq)]
pub enum Rejection {
    /// The engine failed on `step` (1 or 2) in a way that leaves the job
    /// able to go on: the next attempt may well succeed.
    Engine { step: u8, failure: EngineFailure },
    /// The answer to `step` was cut at `max_tokens`: half a paragraph is
    /// not a rewrite of one.
    Truncated { step: u8 },
    /// The answer to `step` was empty once cleaned up.
    Empty { step: u8 },
    /// A guard of `wipemark-core` rejected the final answer against the
    /// chunk: `guard` is its name (`placeholder`, `numbers`,
    /// `length-drift`, `script`, `identifier`), `reason` what was lost.
    Guard {
        guard: &'static str,
        reason: RejectReason,
    },
    /// The answer has [`crate::select::LANGUAGE_CHECK_WORDS`] words or
    /// more and is not in the chunk's language: `found` is what
    /// `lang::detect` read it as — another of ours, or `None` for a
    /// language it declines (French reads as `None`). Only made when the
    /// chunk's own language, `expected`, could be told (D95).
    Language { expected: Lang, found: Option<Lang> },
    /// The answer passed the guards but cannot be put back into the
    /// document — a list item re-split across lines is the case a guard
    /// cannot see.
    Restore(RestoreError),
    /// The answer is the chunk in all but punctuation: divergence under
    /// [`crate::select::NO_OP_FLOOR`].
    NoOp { divergence: f32 },
    /// The answer to the step before `step` carries one of the assembler's
    /// markers, so `step` cannot be asked about it. `clean_response` takes
    /// every marker the input did not have off an answer, so this is a
    /// lock that should never close.
    MarkerInAnswer { step: u8, marker: Marker },
}

impl Rejection {
    /// The id the report and a log line carry. A format.
    pub fn kind(&self) -> &'static str {
        match self {
            Rejection::Engine { .. } => "engine",
            Rejection::Truncated { .. } => "truncated",
            Rejection::Empty { .. } => "empty",
            Rejection::Guard { .. } => "guard",
            Rejection::Language { .. } => "language",
            Rejection::Restore(_) => "restore",
            Rejection::NoOp { .. } => "no-op",
            Rejection::MarkerInAnswer { .. } => "marker-in-answer",
        }
    }
}

/// An engine error that rejects one attempt and leaves the job going.
/// `Cancelled` ends the job and `Unavailable` fails it, so neither is here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineFailure {
    /// The engine's own words (a transport error from the endpoint).
    Transport {
        detail: String,
    },
    /// The endpoint answered in a shape the engine cannot read.
    Protocol {
        detail: String,
    },
    /// The request did not fit the model's window.
    ContextOverflow {
        used: u32,
        limit: u32,
    },
    NotImplemented {
        what: &'static str,
    },
}

impl EngineFailure {
    /// What an engine error means for the job: `Ok` — reject the attempt
    /// with this; `Err` — the error ends the job (`Cancelled`,
    /// `Unavailable`), handed back.
    pub fn of(error: EngineError) -> Result<EngineFailure, EngineError> {
        match error {
            EngineError::Transport(detail) => Ok(EngineFailure::Transport { detail }),
            EngineError::Protocol(detail) => Ok(EngineFailure::Protocol { detail }),
            EngineError::ContextOverflow { used, limit } => {
                Ok(EngineFailure::ContextOverflow { used, limit })
            }
            EngineError::NotImplemented(what) => Ok(EngineFailure::NotImplemented { what }),
            other @ (EngineError::Cancelled | EngineError::Unavailable(_)) => Err(other),
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            EngineFailure::Transport { .. } => "transport",
            EngineFailure::Protocol { .. } => "protocol",
            EngineFailure::ContextOverflow { .. } => "context-overflow",
            EngineFailure::NotImplemented { .. } => "not-implemented",
        }
    }
}

/// How one attempt ended.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Passed(Scores),
    Rejected(Rejection),
}

/// One call of one step of an attempt.
#[derive(Debug, Clone, PartialEq)]
pub struct StepRecord {
    /// 1 or 2.
    pub step: u8,
    /// Which template version the system turn was rendered from.
    pub system: Version,
    /// Which template version the user turn was rendered from.
    pub user: Version,
    pub tokens_out: u32,
    pub finish: FinishReason,
    /// What `clean_response` took off the answer (D67).
    pub stripped: Vec<Stripped>,
}

/// One candidate: every call it took and what became of it.
#[derive(Debug, Clone, PartialEq)]
pub struct Attempt {
    /// 1-based.
    pub round: u8,
    /// 1-based.
    pub candidate: u8,
    pub tactic: Tactic,
    /// The seed every step of this attempt was sampled with.
    pub seed: u64,
    /// The steps that answered, in order. An attempt rejected on step 1
    /// of a two-step tactic has none for step 2.
    pub steps: Vec<StepRecord>,
    /// Layer A over the model's final answer, before the guards — what the
    /// model slipped in. `None` when no answer got that far.
    pub layer_a: Option<CleanReport>,
    pub verdict: Verdict,
    pub elapsed: Duration,
}

/// Why a chunk is the source's own text in the result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kept {
    /// Every attempt of every round was rejected.
    NoCandidatePassed,
    /// The ladder had no rung this document could use (`back_translate`
    /// alone over an undetected language), so nothing was asked.
    NoTactic,
    /// The chunk or its context carries one of the assembler's markers —
    /// preparing the text should have protected it, so this is a bug
    /// upstream, kept rather than sent.
    MarkerInText { marker: Marker },
}

impl Kept {
    fn as_str(self) -> &'static str {
        match self {
            Kept::NoCandidatePassed => "no-candidate-passed",
            Kept::NoTactic => "no-tactic",
            Kept::MarkerInText { .. } => "marker-in-text",
        }
    }
}

/// What became of one chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkOutcome {
    /// The candidate of `round`, `candidate` won.
    Rewritten { round: u8, candidate: u8 },
    /// The chunk is its source.
    KeptSource(Kept),
}

/// One chunk's attempts and outcome.
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkReport {
    /// From 0, as `Chunk::index`.
    pub index: usize,
    pub est_tokens: u32,
    /// The attempts this run made. Empty for a chunk carried over from an
    /// earlier run: those are in [`ChunkReport::carried`].
    pub attempts: Vec<Attempt>,
    pub outcome: ChunkOutcome,
    /// `Some` when the chunk was decided by an earlier run of the same job
    /// and taken back by [`crate::start_resumable`] (E4-4) — its attempts
    /// as that run recorded them. `None` for every chunk [`crate::start`]
    /// reports.
    pub carried: Option<Carried>,
}

impl ChunkReport {
    /// What this chunk cost: the attempts this run made, or the counts the
    /// earlier run recorded.
    pub fn counts(&self) -> ChunkCounts {
        if let Some(carried) = &self.carried {
            return carried.counts;
        }
        let mut counts = ChunkCounts::default();
        for attempt in &self.attempts {
            counts.attempts += 1;
            if matches!(attempt.verdict, Verdict::Rejected(_)) {
                counts.rejected += 1;
            }
            counts.calls += count(attempt.steps.len());
            // A step that failed in the engine was a call too.
            if matches!(attempt.verdict, Verdict::Rejected(Rejection::Engine { .. })) {
                counts.calls += 1;
            }
            counts.tokens_out += attempt
                .steps
                .iter()
                .map(|step| u64::from(step.tokens_out))
                .sum::<u64>();
        }
        counts
    }
}

/// A chunk decided by an earlier run, as it was recorded (E4-4).
///
/// Its attempts are the report's own JSON of them rather than typed
/// [`Attempt`]s: an attempt carries core's `CleanReport`, and core — zero
/// dependencies — has a writer for it and no reader. They go back into the
/// final report exactly as they were written.
#[derive(Debug, Clone, PartialEq)]
pub struct Carried {
    /// The `attempts` array of the chunk's JSON, as recorded.
    pub attempts: Value,
    pub counts: ChunkCounts,
}

/// One chunk's share of [`Totals`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct ChunkCounts {
    pub attempts: u32,
    pub rejected: u32,
    pub calls: u32,
    pub tokens_out: u64,
}

/// A rung of the ladder this document could not use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkippedTactic {
    pub tactic: Tactic,
    pub refusal: Refusal,
}

/// An override that does not render, replaced by the shipped template.
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateFallback {
    pub slot: Slot,
    pub problems: Vec<Problem>,
}

/// The sums a reader wants first. `attempts` is the honest denominator:
/// every candidate asked for, rejected ones included.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Totals {
    pub chunks: u32,
    pub rewritten: u32,
    pub kept_source: u32,
    pub attempts: u32,
    pub rejected: u32,
    /// Engine calls: one per step of every attempt that reached it.
    pub calls: u32,
    pub tokens_out: u64,
}

/// What a finished job hands back.
#[derive(Debug, Clone, PartialEq)]
pub struct JobReport {
    /// Layer A over the document as it was handed over.
    pub before: CleanReport,
    /// Layer A over the assembled result — the final pass.
    pub after: CleanReport,
    pub engine: EngineInfo,
    pub format: TextFormat,
    /// The document's language, `None` when it could not be told.
    pub language: Option<Lang>,
    /// The pivot of `back_translate`, when a usable rung was that tactic.
    pub pivot: Option<Lang>,
    /// The rungs this document could use, in order.
    pub ladder: Vec<Tactic>,
    pub skipped: Vec<SkippedTactic>,
    pub fallbacks: Vec<TemplateFallback>,
    pub intensity: Intensity,
    pub effort: Effort,
    pub base_seed: u64,
    pub scorer: Scorer,
    pub chunks: Vec<ChunkReport>,
    /// The third shelf: never empty.
    pub not_established: Vec<&'static str>,
}

impl JobReport {
    /// The third shelf of a rewrite: what is never established, whatever
    /// the job did — and, because no mark scheme is searched for at all
    /// (D72), the unknown schemes too.
    pub fn shelf() -> Vec<&'static str> {
        let mut shelf = not_established::baseline();
        shelf.push(not_established::UNKNOWN_MARK_SCHEMES);
        shelf
    }

    pub fn totals(&self) -> Totals {
        let mut totals = Totals {
            chunks: count(self.chunks.len()),
            ..Totals::default()
        };
        for chunk in &self.chunks {
            match chunk.outcome {
                ChunkOutcome::Rewritten { .. } => totals.rewritten += 1,
                ChunkOutcome::KeptSource(_) => totals.kept_source += 1,
            }
            let counts = chunk.counts();
            totals.attempts += counts.attempts;
            totals.rejected += counts.rejected;
            totals.calls += counts.calls;
            totals.tokens_out += counts.tokens_out;
        }
        totals
    }

    /// The report as a JSON value: the three shelves as keys. Field names
    /// and ids are formats, never translated.
    pub fn to_value(&self) -> Value {
        let totals = self.totals();
        json!({
            "version": REPORT_VERSION,
            "verifiable": {
                "before": clean_report(&self.before),
                "after": clean_report(&self.after),
            },
            "best_effort": {
                "engine": {
                    "vendor": self.engine.vendor.as_str(),
                    "model_id": self.engine.model_id,
                    "local": self.engine.local,
                    "ctx_len": self.engine.ctx_len,
                },
                "format": format_id(self.format),
                "language": self.language.map(Lang::as_str),
                "pivot": self.pivot.map(Lang::as_str),
                "ladder": self.ladder.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
                "skipped": self.skipped.iter().map(|s| json!({
                    "tactic": s.tactic.as_str(),
                    "refusal": refusal_id(s.refusal),
                })).collect::<Vec<_>>(),
                "template_fallbacks": self.fallbacks.iter().map(|f| json!({
                    "slot": row::key(f.slot),
                    "rules": f.problems.iter().map(Problem::rule).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
                "intensity": self.intensity.as_str(),
                "candidates": self.effort.candidates,
                "rounds": self.effort.rounds,
                "base_seed": self.base_seed,
                "seed_rule": "base_seed + (chunk * rounds + round - 1) * candidates + candidate - 1",
                "scorer": self.scorer.as_str(),
                "selection": {
                    "pick": RULES.selection.as_str(),
                    "no_op_floor": float(RULES.no_op_floor),
                },
                "chunks": self.chunks.iter().map(chunk_value).collect::<Vec<_>>(),
                "totals": {
                    "chunks": totals.chunks,
                    "rewritten": totals.rewritten,
                    "kept_source": totals.kept_source,
                    "attempts": totals.attempts,
                    "rejected": totals.rejected,
                    "calls": totals.calls,
                    "tokens_out": totals.tokens_out,
                },
            },
            "not_established": self.not_established.iter().map(|claim| {
                not_established::ALL
                    .iter()
                    .find(|(_, text)| text == claim)
                    .map_or(*claim, |(id, _)| *id)
            }).collect::<Vec<_>>(),
        })
    }

    /// One line of ASCII JSON.
    pub fn to_json(&self) -> String {
        ascii_json(&self.to_value())
    }
}

impl Serialize for JobReport {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_value().serialize(serializer)
    }
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Core's A §7.1 form, as a value. It is core's own writer, so the two
/// can never drift; it always parses.
fn clean_report(report: &CleanReport) -> Value {
    serde_json::from_str(&report.to_json()).unwrap_or(Value::Null)
}

pub(crate) fn format_id(format: TextFormat) -> &'static str {
    match format {
        TextFormat::Plain => "plain",
        TextFormat::Markdown => "markdown",
        TextFormat::Html => "html",
        TextFormat::Code => "code",
    }
}

fn refusal_id(refusal: Refusal) -> &'static str {
    match refusal {
        Refusal::BackTranslateNeedsLanguage => "back-translate-needs-language",
    }
}

/// A float rounded to four places: the telemetry precision, and no
/// `0.30000001192092896` from an `f32` widened.
fn float(x: f32) -> Value {
    if x.is_finite() {
        json!((f64::from(x) * 10_000.0).round() / 10_000.0)
    } else {
        Value::Null
    }
}

fn chunk_value(chunk: &ChunkReport) -> Value {
    match &chunk.carried {
        // `carried_over` only where it is true: the JSON of a job that
        // carried nothing is what it was before E4-4, byte for byte.
        Some(carried) => json!({
            "index": chunk.index,
            "est_tokens": chunk.est_tokens,
            "outcome": outcome_value(chunk.outcome),
            "carried_over": true,
            "attempts": carried.attempts,
        }),
        None => json!({
            "index": chunk.index,
            "est_tokens": chunk.est_tokens,
            "outcome": outcome_value(chunk.outcome),
            "attempts": attempts_value(&chunk.attempts),
        }),
    }
}

/// The `attempts` array of a chunk's JSON.
pub(crate) fn attempts_value(attempts: &[Attempt]) -> Value {
    Value::Array(attempts.iter().map(attempt_value).collect())
}

/// A chunk's outcome in the report's JSON shape.
pub(crate) fn outcome_value(outcome: ChunkOutcome) -> Value {
    match outcome {
        ChunkOutcome::Rewritten { round, candidate } => {
            json!({"rewritten": {"round": round, "candidate": candidate}})
        }
        ChunkOutcome::KeptSource(kept) => {
            let mut value = json!({"kept_source": kept.as_str()});
            if let Kept::MarkerInText { marker } = kept {
                value["marker"] = json!(marker.as_str());
            }
            value
        }
    }
}

/// [`outcome_value`] read back; `None` for anything it could not have
/// written.
pub(crate) fn outcome_of(value: &Value) -> Option<ChunkOutcome> {
    if let Some(rewritten) = value.get("rewritten") {
        let number = |key: &str| {
            rewritten
                .get(key)
                .and_then(Value::as_u64)
                .and_then(|n| u8::try_from(n).ok())
        };
        return Some(ChunkOutcome::Rewritten {
            round: number("round")?,
            candidate: number("candidate")?,
        });
    }
    let kept = match value.get("kept_source")?.as_str()? {
        "no-candidate-passed" => Kept::NoCandidatePassed,
        "no-tactic" => Kept::NoTactic,
        "marker-in-text" => {
            let named = value.get("marker")?.as_str()?;
            let marker = Marker::ALL
                .into_iter()
                .find(|marker| marker.as_str() == named)?;
            Kept::MarkerInText { marker }
        }
        _ => return None,
    };
    Some(ChunkOutcome::KeptSource(kept))
}

fn attempt_value(attempt: &Attempt) -> Value {
    let verdict = match &attempt.verdict {
        Verdict::Passed(scores) => json!({"passed": {
            "divergence": float(scores.divergence),
            "length_ratio": float(scores.length_ratio),
            "score": float(scores.score),
        }}),
        Verdict::Rejected(rejection) => json!({"rejected": rejection_value(rejection)}),
    };
    json!({
        "round": attempt.round,
        "candidate": attempt.candidate,
        "tactic": attempt.tactic.as_str(),
        "seed": attempt.seed,
        "millis": u64::try_from(attempt.elapsed.as_millis()).unwrap_or(u64::MAX),
        "steps": attempt.steps.iter().map(step_value).collect::<Vec<_>>(),
        "layer_a": attempt.layer_a.as_ref().map(clean_report),
        "verdict": verdict,
    })
}

fn step_value(step: &StepRecord) -> Value {
    json!({
        "step": step.step,
        "system": version_value(&step.system),
        "user": version_value(&step.user),
        "tokens_out": step.tokens_out,
        "finish": match step.finish {
            FinishReason::Stop => "stop",
            FinishReason::Length => "length",
            FinishReason::Cancelled => "cancelled",
        },
        "stripped": step.stripped.iter().map(stripped_value).collect::<Vec<_>>(),
    })
}

fn version_value(version: &Version) -> Value {
    match version {
        Version::Shipped { hash } => json!({"source": "shipped", "hash": hash}),
        Version::Override {
            hash,
            origin,
            stale,
        } => json!({
            "source": "override",
            "hash": hash,
            "origin": origin.as_str(),
            "stale": {
                "shipped_changed": stale.shipped_changed,
                "source_changed": stale.source_changed,
            },
        }),
    }
}

fn stripped_value(stripped: &Stripped) -> Value {
    match stripped {
        Stripped::Think => json!({"what": "think"}),
        Stripped::Marker { marker, count } => {
            json!({"what": "marker", "marker": marker.as_str(), "count": count})
        }
        Stripped::Fence => json!({"what": "fence"}),
        Stripped::Quotes { open, close } => {
            json!({"what": "quotes", "open": open.to_string(), "close": close.to_string()})
        }
    }
}

/// The structured reason as JSON: `kind`, then the variant's own fields.
pub fn rejection_value(rejection: &Rejection) -> Value {
    let mut out = Map::new();
    out.insert("kind".into(), json!(rejection.kind()));
    match rejection {
        Rejection::Engine { step, failure } => {
            out.insert("step".into(), json!(step));
            out.insert("failure".into(), json!(failure.kind()));
            match failure {
                EngineFailure::Transport { detail } | EngineFailure::Protocol { detail } => {
                    out.insert("detail".into(), json!(detail));
                }
                EngineFailure::ContextOverflow { used, limit } => {
                    out.insert("used".into(), json!(used));
                    out.insert("limit".into(), json!(limit));
                }
                EngineFailure::NotImplemented { what } => {
                    out.insert("what".into(), json!(what));
                }
            }
        }
        Rejection::Truncated { step } | Rejection::Empty { step } => {
            out.insert("step".into(), json!(step));
        }
        Rejection::Guard { guard, reason } => {
            out.insert("guard".into(), json!(guard));
            reason_fields(reason, &mut out);
        }
        Rejection::Language { expected, found } => {
            out.insert("expected".into(), json!(expected.as_str()));
            out.insert("found".into(), json!(found.map(Lang::as_str)));
        }
        Rejection::Restore(error) => {
            let (reason, index) = match error {
                RestoreError::Unknown { index } => ("unknown", index),
                RestoreError::Missing { index } => ("missing", index),
                RestoreError::Duplicated { index, count } => {
                    out.insert("count".into(), json!(count));
                    ("duplicated", index)
                }
                RestoreError::ItemBroken => {
                    out.insert("reason".into(), json!("item-broken"));
                    return Value::Object(out);
                }
            };
            out.insert("reason".into(), json!(reason));
            out.insert("index".into(), json!(index));
        }
        Rejection::NoOp { divergence } => {
            out.insert("divergence".into(), float(*divergence));
        }
        Rejection::MarkerInAnswer { step, marker } => {
            out.insert("step".into(), json!(step));
            out.insert("marker".into(), json!(marker.as_str()));
        }
    }
    Value::Object(out)
}

fn reason_fields(reason: &RejectReason, out: &mut Map<String, Value>) {
    let mut put = |key: &str, value: Value| {
        out.insert(key.to_owned(), value);
    };
    match reason {
        RejectReason::PlaceholderMissing { index } => {
            put("reason", json!("placeholder-missing"));
            put("index", json!(index));
        }
        RejectReason::PlaceholderDuplicated { index, count } => {
            put("reason", json!("placeholder-duplicated"));
            put("index", json!(index));
            put("count", json!(count));
        }
        RejectReason::PlaceholderInvented { index } => {
            put("reason", json!("placeholder-invented"));
            put("index", json!(index));
        }
        RejectReason::NumberMissing { value } => {
            put("reason", json!("number-missing"));
            put("value", json!(value));
        }
        RejectReason::LengthDrift { ratio, min, max } => {
            put("reason", json!("length-drift"));
            put("ratio", float(*ratio));
            put("min", float(*min));
            put("max", float(*max));
        }
        RejectReason::ScriptDrift { script, delta_pp } => {
            put("reason", json!("script-drift"));
            put("script", json!(script));
            put("delta_pp", float(*delta_pp));
        }
        RejectReason::IdentifierMissing { token } => {
            put("reason", json!("identifier-missing"));
            put("token", json!(token));
        }
    }
}

/// `value` as one line of JSON with every non-ASCII character escaped.
pub(crate) fn ascii_json(value: &Value) -> String {
    let mut out = Vec::new();
    let mut serializer = serde_json::Serializer::with_formatter(&mut out, Ascii);
    // Serializing a `Value` into memory cannot fail.
    let _ = value.serialize(&mut serializer);
    String::from_utf8(out).unwrap_or_default()
}

/// serde_json's compact formatter, except that a string fragment is
/// written with `\uXXXX` (UTF-16, a surrogate pair above the BMP) for
/// everything from U+007F up.
struct Ascii;

impl serde_json::ser::Formatter for Ascii {
    fn write_string_fragment<W: ?Sized + std::io::Write>(
        &mut self,
        writer: &mut W,
        fragment: &str,
    ) -> std::io::Result<()> {
        let mut units = [0u16; 2];
        for c in fragment.chars() {
            if c.is_ascii() && c != '\u{7F}' {
                writer.write_all(&[c as u8])?;
            } else {
                for unit in c.encode_utf16(&mut units) {
                    write!(writer, "\\u{unit:04x}")?;
                }
            }
        }
        Ok(())
    }
}
