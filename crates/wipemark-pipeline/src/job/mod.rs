//! The job (E4-3): a document and an engine in, a rewritten document and
//! a report out — through events, on a thread of its own.
//!
//! ```text
//! Layer A → prepare → per chunk:
//!     round 1: candidates × (render → complete → clean_response → Layer A → guards → restore → no-op)
//!     round 2, one rung up the ladder — only if no candidate of round 1 passed (D61)
//!     winner: the least diverged that passed (D71) — or the chunk keeps its source
//! → assemble → Layer A over the whole result
//! ```
//!
//! [`start`] validates the options, spawns the job's thread and returns at
//! once with a [`JobHandle`] (to cancel) and the receiving end of its
//! [`Event`]s; the result comes back as the last event. Nothing here
//! blocks the caller — the consumer is a GPUI window that must not wait a
//! frame — and nothing starts an async runtime: the engine's future is
//! driven by [`drive`]'s few lines of `std::task` on the job's thread.
//!
//! The engine is an `Arc<dyn RewriteEngine>`. The application's
//! `EngineHandle` becomes one through a small adapter (E4-6), so a job
//! goes through the same busy count and keep policy as a Check does.

mod attempt;
mod drive;
mod plan;
pub mod resume;
#[cfg(test)]
mod resume_tests;
mod stored;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub use plan::{plan, Planned, Rung};
pub use resume::{Decided, RecordError};
pub use stored::{OptionsError, OPTIONS_VERSION};
use wipemark_core::{default_guards, Guard, LengthDriftGuard};
use wipemark_engine::{CancellationToken, EngineError, RewriteEngine, SamplingParams};

use crate::cost::{Effort, Executor};
use crate::lang::Lang;
use crate::prepare::TextFormat;
use crate::prompt::{Intensity, Overrides, Tactic};
use crate::report::{Carried, ChunkOutcome, ChunkReport, EngineFailure, JobReport, Kept, Verdict};
use crate::select::{self, Scorer};
use crate::{Event, JobId, PipelineError, Stage};

/// What is to be rewritten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub text: String,
    pub format: TextFormat,
}

/// How a job rewrites.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    /// Layer A's knobs, for the passes before and after the model and for
    /// every candidate.
    pub layer_a: wipemark_core::Options,
    /// The tactics, one rung per round (OV §4.3; default `[paraphrase]`).
    pub ladder: Vec<Tactic>,
    /// `structural` is offered only behind a confirmation (D73).
    pub structural_confirmed: bool,
    pub intensity: Intensity,
    /// Candidates per round, rounds per chunk (D61).
    pub effort: Effort,
    /// The first seed; every attempt's is derived from it ([`seed_for`]).
    pub base_seed: u64,
    /// Temperature, top-p, min-p. `seed` is set per attempt; `max_tokens`,
    /// when `None`, is set per call to twice the input's estimate plus 64.
    pub sampling: SamplingParams,
    /// The `rewrite.pivot` row (D60), `None` for the default.
    pub pivot: Option<Lang>,
    /// The template rows that parsed (D74).
    pub overrides: Overrides,
    /// The length guard's window, in place of the default 0.6–1.6 — the
    /// one guard threshold the bench (E4-5) may move.
    pub length: LengthDriftGuard,
}

impl Options {
    /// The product's defaults for who rewrites: `[paraphrase]`, moderate,
    /// D61's effort, seed 0, the engine's default sampling.
    pub fn for_executor(executor: Executor) -> Options {
        Options {
            layer_a: wipemark_core::Options::default(),
            ladder: vec![Tactic::Paraphrase],
            structural_confirmed: false,
            intensity: Intensity::default(),
            effort: Effort::for_executor(executor),
            base_seed: 0,
            sampling: SamplingParams::default(),
            pivot: None,
            overrides: Overrides::new(),
            length: LengthDriftGuard::default(),
        }
    }

    /// Whether a job can start with these options.
    pub fn check(&self) -> Result<(), Refused> {
        if self.ladder.is_empty() {
            return Err(Refused::EmptyLadder);
        }
        if self.effort.candidates == 0 {
            return Err(Refused::NoCandidates);
        }
        if self.effort.rounds == 0 {
            return Err(Refused::NoRounds);
        }
        if self.ladder.contains(&Tactic::Code) {
            return Err(Refused::CodeNotBuilt);
        }
        if let Some(at) = self.ladder.iter().position(|t| *t == Tactic::Structural) {
            if !self.structural_confirmed {
                return Err(Refused::StructuralNotConfirmed);
            }
            if at + 1 != self.ladder.len() {
                return Err(Refused::StructuralNotLast);
            }
        }
        Ok(())
    }

    /// The guards every candidate faces: the five of `wipemark-core` in
    /// their order, the length guard with this window.
    fn guards(&self) -> Vec<Box<dyn Guard>> {
        default_guards()
            .into_iter()
            .map(|guard| -> Box<dyn Guard> {
                if guard.name() == LengthDriftGuard::default().name() {
                    Box::new(self.length)
                } else {
                    guard
                }
            })
            .collect()
    }
}

/// Why a job did not start. Values, not sentences: the surface words them.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refused {
    #[error("the ladder has no tactic")]
    EmptyLadder,
    #[error("zero candidates per round")]
    NoCandidates,
    #[error("zero rounds")]
    NoRounds,
    /// `structural` rewrites a document from an outline of it: a content
    /// risk the user confirms first (D73).
    #[error("structural is on the ladder without a confirmation")]
    StructuralNotConfirmed,
    /// `structural` is the last rung or none (D73).
    #[error("structural is not the last rung of the ladder")]
    StructuralNotLast,
    /// The `code` tactic needs a preparation of its own, which is not
    /// built.
    #[error("the code tactic is not built")]
    CodeNotBuilt,
    /// The job's thread could not be started; `reason` is the system's.
    #[error("the job's thread could not start: {reason}")]
    Thread { reason: String },
}

/// What a finished job hands back.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// The document, rewritten where a candidate won and the Layer-A-cleaned
    /// source elsewhere, cleaned once more as a whole.
    pub text: String,
    pub report: JobReport,
}

/// A running job, from any thread.
#[derive(Debug, Clone)]
pub struct JobHandle {
    id: JobId,
    cancel: CancellationToken,
}

impl JobHandle {
    pub fn id(&self) -> JobId {
        self.id
    }

    /// Stop the job: the call in flight is cancelled, and the job ends with
    /// [`Event::Cancelled`] and no document.
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }
}

/// The seed of one attempt: every attempt of a job gets its own, and the
/// report carries it.
///
/// `base + (chunk · rounds + (round − 1)) · candidates + (candidate − 1)`,
/// wrapping; `chunk` from 0, `round` and `candidate` from 1. OV §4.4's
/// `base + round · c` gives (1, 2) and (2, 1) one seed, and every chunk the
/// same ones.
pub fn seed_for(base: u64, effort: Effort, chunk: usize, round: u8, candidate: u8) -> u64 {
    let per_chunk = u64::from(effort.rounds) * u64::from(effort.candidates);
    let attempt = u64::from(round.saturating_sub(1)) * u64::from(effort.candidates)
        + u64::from(candidate.saturating_sub(1));
    base.wrapping_add((chunk as u64).wrapping_mul(per_chunk).wrapping_add(attempt))
}

/// Start a job on a thread of its own.
///
/// Returns at once: the handle cancels it, and the receiver carries its
/// [`Event`]s, the last of which is exactly one of [`Event::Finished`] (with
/// the document and the report), [`Event::Cancelled`] or
/// [`Event::Failed`]. Dropping the receiver cancels the job — nobody is
/// left to read what it would produce.
pub fn start(
    id: JobId,
    document: Document,
    options: Options,
    engine: Arc<dyn RewriteEngine>,
) -> Result<(JobHandle, flume::Receiver<Event>), Refused> {
    spawn(id, document, options, engine, None)
}

/// [`start`], for a job that may be taken up again (E4-4).
///
/// The same job, with two differences. Every chunk it decides is handed
/// out as [`Event::ChunkDecided`] — a [`Decided`] that outlives the
/// process — and `carried`, the decisions an earlier run of the same job
/// handed out, are taken back: a chunk with a usable record is not asked
/// again, its winner (or its kept source) goes into the assembly as it was
/// decided, and its report is the record's, marked as carried over. After
/// planning, [`Event::Resumed`] says how many records were used and how
/// many were discarded — when anything was handed in.
///
/// A record is usable only when the job's fingerprint is the one it was
/// decided under (the document, its format, the options, the engine, the
/// budget and the templates — [`resume`]), it names a chunk of this plan
/// whose digest it carries, and its winner still restores into that chunk.
/// Anything else is discarded and the chunk asked again; nothing is
/// repaired.
pub fn start_resumable(
    id: JobId,
    document: Document,
    options: Options,
    engine: Arc<dyn RewriteEngine>,
    carried: Vec<Decided>,
) -> Result<(JobHandle, flume::Receiver<Event>), Refused> {
    spawn(id, document, options, engine, Some(carried))
}

fn spawn(
    id: JobId,
    document: Document,
    options: Options,
    engine: Arc<dyn RewriteEngine>,
    carried: Option<Vec<Decided>>,
) -> Result<(JobHandle, flume::Receiver<Event>), Refused> {
    options.check()?;
    let (events, receiver) = flume::unbounded();
    let cancel = CancellationToken::new();
    let handle = JobHandle {
        id,
        cancel: cancel.clone(),
    };
    std::thread::Builder::new()
        .name(format!("wipemark-job-{}", id.0))
        .spawn(move || {
            let emit = Emit {
                job: id,
                events,
                cancel,
                started: Instant::now(),
            };
            run(&emit, &document, &options, engine.as_ref(), carried);
        })
        .map_err(|error| Refused::Thread {
            reason: error.to_string(),
        })?;
    Ok((handle, receiver))
}

/// The job's side of its events.
struct Emit {
    job: JobId,
    events: flume::Sender<Event>,
    cancel: CancellationToken,
    started: Instant,
}

impl Emit {
    /// Send `event`; a receiver that is gone cancels the job.
    fn send(&self, event: Event) {
        if self.events.send(event).is_err() {
            self.cancel.cancel();
        }
    }

    fn stage(&self, stage: Stage) {
        self.send(Event::Stage {
            job: self.job,
            stage,
        });
    }

    fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    fn cancelled(&self) {
        tracing::info!(job = self.job.0, "the job was cancelled");
        self.stage(Stage::Cancelled);
        self.send(Event::Cancelled {
            job: self.job,
            elapsed: self.elapsed(),
        });
    }

    fn failed(&self, error: PipelineError) {
        tracing::warn!(job = self.job.0, error = %error, "the job failed");
        self.stage(Stage::Failed);
        self.send(Event::Failed {
            job: self.job,
            elapsed: self.elapsed(),
            error,
        });
    }
}

/// What a resumable job keeps beside its loop: its fingerprint and the
/// records it may use, by chunk.
struct Journal {
    fingerprint: String,
    usable: BTreeMap<usize, Decided>,
}

impl Journal {
    /// Sort what was handed in into what fits this plan and what does not.
    fn of(
        fingerprint: String,
        carried: Vec<Decided>,
        chunks: &[crate::prepare::Chunk],
    ) -> (Journal, u32) {
        let handed = carried.len();
        let mut usable = BTreeMap::new();
        for decided in carried {
            let fits = chunks
                .get(decided.index)
                .is_some_and(|chunk| decided.fits(chunk, &fingerprint));
            if fits {
                usable.entry(decided.index).or_insert(decided);
            }
        }
        let discarded = u32::try_from(handed - usable.len()).unwrap_or(u32::MAX);
        (
            Journal {
                fingerprint,
                usable,
            },
            discarded,
        )
    }
}

/// The job, start to end. Every path ends in exactly one terminal event.
/// `carried` is `None` for [`start`] — no records out, none in — and the
/// earlier run's records for [`start_resumable`].
fn run(
    emit: &Emit,
    document: &Document,
    options: &Options,
    engine: &dyn RewriteEngine,
    carried: Option<Vec<Decided>>,
) {
    emit.stage(Stage::CleaningLayerA);
    let info = engine.info();
    let planned = match plan(document, options, &info) {
        Ok(planned) => planned,
        Err(error) => return emit.failed(error),
    };
    let chunks = planned.prepared.chunks();
    tracing::info!(
        job = emit.job.0,
        chunks = chunks.len(),
        rungs = planned.rungs.len(),
        language = planned.language().map(Lang::as_str),
        budget = planned.budget.max_tokens,
        "the job is planned"
    );
    let mut journal = carried.map(|carried| {
        let handed = carried.len();
        let fingerprint = resume::fingerprint(document, options, &info, &planned);
        let (journal, discarded) = Journal::of(fingerprint, carried, chunks);
        if handed > 0 {
            let carried = u32::try_from(journal.usable.len()).unwrap_or(u32::MAX);
            tracing::info!(job = emit.job.0, carried, discarded, "the job is resumed");
            emit.send(Event::Resumed {
                job: emit.job,
                carried,
                discarded,
            });
        }
        journal
    });
    if emit.cancel.is_cancelled() {
        return emit.cancelled();
    }

    let carried_over = |index: usize| {
        journal
            .as_ref()
            .is_some_and(|journal| journal.usable.contains_key(&index))
    };
    let work = !planned.rungs.is_empty() && chunks.iter().any(|chunk| !carried_over(chunk.index));
    if work {
        emit.stage(Stage::LoadingModel);
        let (_, nothing) = flume::unbounded::<String>();
        match drive::block_on(engine.warmup(), &nothing, |_| {}) {
            Ok(()) => {}
            Err(EngineError::Cancelled) => return emit.cancelled(),
            Err(EngineError::Unavailable(why)) => {
                return emit.failed(PipelineError::Unavailable(why))
            }
            Err(other) => match EngineFailure::of(other) {
                Ok(failure) => return emit.failed(PipelineError::Engine(failure)),
                Err(_) => return emit.cancelled(),
            },
        }
    }

    let guards = options.guards();
    let ask = attempt::Ask {
        engine,
        options,
        guards: &guards,
        cancel: &emit.cancel,
    };
    let count = u32::try_from(chunks.len()).unwrap_or(u32::MAX);
    let mut reports = Vec::with_capacity(chunks.len());
    let mut winners: Vec<Option<String>> = Vec::with_capacity(chunks.len());
    // A chunk's decision, handed out by a resumable job as soon as it is
    // made — what a `kill -9` a moment later does not lose.
    let decided = |reports: &[ChunkReport],
                   winners: &[Option<String>],
                   chunk: &crate::prepare::Chunk,
                   journal: &Option<Journal>| {
        let (Some(journal), Some(report)) = (journal, reports.last()) else {
            return;
        };
        let winner = winners.last().cloned().flatten();
        emit.send(Event::ChunkDecided {
            job: emit.job,
            chunk: u32::try_from(chunk.index + 1).unwrap_or(u32::MAX),
            decided: Box::new(Decided::of(report, chunk, &journal.fingerprint, winner)),
        });
    };
    for chunk in chunks {
        if emit.cancel.is_cancelled() {
            return emit.cancelled();
        }
        if let Some(record) = journal
            .as_mut()
            .and_then(|journal| journal.usable.remove(&chunk.index))
        {
            reports.push(ChunkReport {
                index: chunk.index,
                est_tokens: chunk.est_tokens,
                attempts: Vec::new(),
                outcome: record.outcome,
                carried: Some(Carried {
                    attempts: record.attempts,
                    counts: record.counts,
                }),
            });
            winners.push(record.winner);
            continue;
        }
        let kept = if planned.rungs.is_empty() {
            Some(Kept::NoTactic)
        } else {
            planned
                .marker_in(chunk, options)
                .map(|marker| Kept::MarkerInText { marker })
        };
        if let Some(kept) = kept {
            reports.push(ChunkReport {
                index: chunk.index,
                est_tokens: chunk.est_tokens,
                attempts: Vec::new(),
                outcome: ChunkOutcome::KeptSource(kept),
                carried: None,
            });
            winners.push(None);
            decided(&reports, &winners, chunk, &journal);
            continue;
        }

        let mut attempts = Vec::new();
        let mut passed: Vec<Option<String>> = Vec::new();
        for round in 1..=options.effort.rounds {
            let Some(rung) = planned.rung(round) else {
                break;
            };
            for candidate in 1..=options.effort.candidates {
                if emit.cancel.is_cancelled() {
                    return emit.cancelled();
                }
                emit.stage(Stage::Rewriting {
                    chunk: u32::try_from(chunk.index + 1).unwrap_or(u32::MAX),
                    chunks: count,
                    candidate,
                    candidates: options.effort.candidates,
                    round,
                    rounds: options.effort.rounds,
                });
                let seed = seed_for(
                    options.base_seed,
                    options.effort,
                    chunk.index,
                    round,
                    candidate,
                );
                let mut on_token = |text: String| {
                    emit.send(Event::Token {
                        job: emit.job,
                        text,
                    })
                };
                match attempt::run(&ask, chunk, rung, round, candidate, seed, &mut on_token) {
                    attempt::Ended::Done { attempt, candidate } => {
                        if let Verdict::Rejected(rejection) = &attempt.verdict {
                            tracing::info!(
                                job = emit.job.0,
                                chunk = chunk.index,
                                round,
                                candidate = attempt.candidate,
                                seed,
                                rejection = rejection.kind(),
                                "a candidate was rejected"
                            );
                            emit.send(Event::CandidateRejected {
                                job: emit.job,
                                chunk: u32::try_from(chunk.index + 1).unwrap_or(u32::MAX),
                                round,
                                candidate: attempt.candidate,
                                rejection: rejection.clone(),
                            });
                        }
                        attempts.push(*attempt);
                        passed.push(candidate);
                    }
                    attempt::Ended::Cancelled => return emit.cancelled(),
                    attempt::Ended::Unavailable(why) => {
                        return emit.failed(PipelineError::Unavailable(why))
                    }
                    attempt::Ended::Failed(error) => return emit.failed(error),
                }
            }
            // A round runs all its candidates; the next runs only when none
            // of this one passed (D61, reference §5).
            if passed.iter().any(Option::is_some) {
                break;
            }
        }

        let scores: Vec<_> = attempts
            .iter()
            .map(|attempt| match attempt.verdict {
                Verdict::Passed(scores) => Some(scores),
                Verdict::Rejected(_) => None,
            })
            .collect();
        let (outcome, winner) = match select::winner(&scores) {
            Some(i) => (
                ChunkOutcome::Rewritten {
                    round: attempts[i].round,
                    candidate: attempts[i].candidate,
                },
                passed[i].take(),
            ),
            None => (ChunkOutcome::KeptSource(Kept::NoCandidatePassed), None),
        };
        reports.push(ChunkReport {
            index: chunk.index,
            est_tokens: chunk.est_tokens,
            attempts,
            outcome,
            carried: None,
        });
        winners.push(winner);
        decided(&reports, &winners, chunk, &journal);
    }

    let answers: Vec<Option<&str>> = winners.iter().map(Option::as_deref).collect();
    let assembled = match planned.prepared.assemble(&answers) {
        Ok(assembled) => assembled,
        Err(error) => return emit.failed(PipelineError::Assemble(error)),
    };
    let after = wipemark_core::clean(&assembled, &options.layer_a);
    let report = JobReport {
        before: planned.before.report.clone(),
        after: after.report,
        engine: info,
        format: document.format,
        language: planned.language(),
        pivot: planned.pivot(),
        ladder: planned.rungs.iter().map(|rung| rung.tactic).collect(),
        skipped: planned.skipped.clone(),
        fallbacks: planned.fallbacks.clone(),
        intensity: options.intensity,
        effort: options.effort,
        base_seed: options.base_seed,
        scorer: Scorer::Divergence,
        chunks: reports,
        not_established: JobReport::shelf(),
    };
    let totals = report.totals();
    tracing::info!(
        job = emit.job.0,
        chunks = totals.chunks,
        rewritten = totals.rewritten,
        kept = totals.kept_source,
        attempts = totals.attempts,
        rejected = totals.rejected,
        bytes = after.text.len(),
        "the job finished"
    );
    if emit.cancel.is_cancelled() {
        return emit.cancelled();
    }
    emit.stage(Stage::Finished);
    emit.send(Event::Finished {
        job: emit.job,
        elapsed: emit.elapsed(),
        outcome: Box::new(Outcome {
            text: after.text,
            report,
        }),
    });
}
