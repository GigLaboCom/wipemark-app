//! One attempt: one candidate for one chunk, every step of its tactic,
//! and the verdict on what came back.
//!
//! ```text
//! render → into_request(seed, max_tokens) → complete (tokens streamed)
//!        → clean_response                        [per step; step 2 reads step 1's cleaned answer]
//! → Layer A → verdict: guards(chunk.text, candidate) → language → restore → no-op → passed
//! ```
//!
//! Layer A runs **before** the guards: a model that slips U+200B into an
//! identifier must not lose the candidate to `IdentifierGuard` for it —
//! the character is removed and counted, and the identifier is whole
//! again. The language check (D95) refuses an answer of twenty words or
//! more that is not in the chunk's language — `ScriptGuard` cannot see
//! French for English. A `RestoreError` is a rejection like a guard's: a
//! list item re-split across lines is what restore sees that no guard can.
//!
//! [`verdict`] is everything after Layer A, public so that the prompt
//! bench judges an answer with the loop's own code rather than a copy.

use std::time::Instant;

use wipemark_core::{Guard, GuardOutcome};
use wipemark_engine::{
    CancellationToken, EngineError, FinishReason, RewriteEngine, TokenSink, Unavailable,
};

use super::drive::block_on;
use super::plan::Rung;
use super::Options;
use crate::lang::{self, Lang};
use crate::prepare::{estimate_tokens, without_placeholders, Chunk};
use crate::prompt::{clean_response, render, Input, RenderError};
use crate::report::{Attempt, EngineFailure, Rejection, StepRecord, Verdict};
use crate::select::{self, Scorer, Scores, LANGUAGE_CHECK_WORDS, NO_OP_FLOOR};
use crate::PipelineError;

/// What the loop needs to ask for one candidate.
pub(super) struct Ask<'a> {
    pub engine: &'a dyn RewriteEngine,
    pub options: &'a Options,
    pub cancel: &'a CancellationToken,
}

/// What every answer for one chunk is judged against, worked out once per
/// chunk: its guards (the length window for its size) and its language.
pub(super) struct Judge {
    guards: Vec<Box<dyn Guard>>,
    /// `lang::detect` over the chunk's text, placeholders removed; `None`
    /// when it cannot be told, and then no answer is language-checked.
    language: Option<Lang>,
}

impl Judge {
    pub(super) fn of(options: &Options, chunk: &Chunk) -> Judge {
        Judge {
            guards: options.guards_for(chunk),
            language: lang::detect(&without_placeholders(&chunk.text)),
        }
    }

    /// The verdict on `answer` (already through Layer A) for `chunk`.
    fn verdict(&self, chunk: &Chunk, answer: &str) -> Result<Scores, Rejection> {
        for guard in &self.guards {
            if let GuardOutcome::Reject(reason) = guard.check(&chunk.text, answer) {
                return Err(Rejection::Guard {
                    guard: guard.name(),
                    reason,
                });
            }
        }
        if let Some(expected) = self.language {
            if select::prose_words(answer) >= LANGUAGE_CHECK_WORDS {
                let found = lang::detect(&without_placeholders(answer));
                if found != Some(expected) {
                    return Err(Rejection::Language { expected, found });
                }
            }
        }
        chunk.restore(answer).map_err(Rejection::Restore)?;
        let scores = select::score(Scorer::Divergence, &chunk.text, answer);
        if scores.divergence < NO_OP_FLOOR {
            return Err(Rejection::NoOp {
                divergence: scores.divergence,
            });
        }
        Ok(scores)
    }
}

/// The loop's verdict on `answer` — a model's final answer for `chunk`,
/// already cleaned by Layer A — under `options`: the guards with the
/// chunk's length window, the language check, restore and the no-op
/// floor, in that order; the first that fails is the rejection.
///
/// The loop calls this for every candidate; so does the prompt bench, so
/// that its numbers are the product's.
pub fn verdict(options: &Options, chunk: &Chunk, answer: &str) -> Result<Scores, Rejection> {
    Judge::of(options, chunk).verdict(chunk, answer)
}

/// Where one attempt stands at its end.
pub(super) enum Ended {
    /// The attempt ran to a verdict. `candidate` is the text that passed,
    /// placeholders and all; `None` when it was rejected.
    Done {
        attempt: Box<Attempt>,
        candidate: Option<String>,
    },
    /// The job was cancelled during it.
    Cancelled,
    /// Nothing can answer: the job fails.
    Unavailable(Unavailable),
    /// A template that was proven to render did not — a bug.
    Failed(PipelineError),
}

/// Run one attempt at `chunk` with `rung`'s tactic.
#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    ask: &Ask<'_>,
    judge: &Judge,
    chunk: &Chunk,
    rung: &Rung,
    round: u8,
    candidate: u8,
    seed: u64,
    on_token: &mut dyn FnMut(String),
) -> Ended {
    let started = Instant::now();
    let mut steps = Vec::new();
    let mut text = chunk.text.clone();
    let done = |steps, layer_a, verdict| Ended::Done {
        attempt: Box::new(Attempt {
            round,
            candidate,
            tactic: rung.tactic,
            seed,
            steps,
            layer_a,
            verdict,
            elapsed: started.elapsed(),
        }),
        candidate: None,
    };
    let rejected = |rejection| Verdict::Rejected(rejection);

    for step in &rung.plan.steps {
        let input = Input {
            text: &text,
            context: chunk.context.as_deref(),
            intensity: ask.options.intensity,
        };
        let rendered = match render(step, &input) {
            Ok(rendered) => rendered,
            Err(RenderError::MarkerInText { marker }) => {
                return done(
                    steps,
                    None,
                    rejected(Rejection::MarkerInAnswer {
                        step: step.step,
                        marker,
                    }),
                );
            }
            Err(RenderError::Template { slot, .. }) => {
                return Ended::Failed(PipelineError::ShippedTemplate { slot });
            }
        };
        let mut params = ask.options.sampling.clone();
        params.seed = Some(seed);
        if params.max_tokens.is_none() {
            params.max_tokens = Some(estimate_tokens(&text).saturating_mul(2).saturating_add(64));
        }
        let (sink, tokens): (TokenSink, _) = flume::unbounded();
        let answer = block_on(
            ask.engine
                .complete(rendered.into_request(params), sink, ask.cancel.clone()),
            &tokens,
            &mut *on_token,
        );
        let completion = match answer {
            Ok(completion) => completion,
            Err(error) => match EngineFailure::of(error) {
                Ok(failure) => {
                    tracing::info!(
                        round,
                        candidate,
                        step = step.step,
                        failure = ?failure,
                        "an attempt failed in the engine"
                    );
                    return done(
                        steps,
                        None,
                        rejected(Rejection::Engine {
                            step: step.step,
                            failure,
                        }),
                    );
                }
                Err(EngineError::Unavailable(why)) => return Ended::Unavailable(why),
                Err(_) => return Ended::Cancelled,
            },
        };
        let cleaned = clean_response(&completion.text, &text);
        steps.push(StepRecord {
            step: step.step,
            system: step.system.version.clone(),
            user: step.user.version.clone(),
            tokens_out: completion.tokens_out,
            finish: completion.finish,
            stripped: cleaned.stripped,
        });
        match completion.finish {
            FinishReason::Cancelled => return Ended::Cancelled,
            FinishReason::Length => {
                return done(
                    steps,
                    None,
                    rejected(Rejection::Truncated { step: step.step }),
                );
            }
            FinishReason::Stop => {}
        }
        if cleaned.text.trim().is_empty() {
            return done(steps, None, rejected(Rejection::Empty { step: step.step }));
        }
        text = cleaned.text;
    }

    let layer_a = wipemark_core::clean(&text, &ask.options.layer_a);
    let answer = layer_a.text;
    let layer_a = Some(layer_a.report);
    match judge.verdict(chunk, &answer) {
        Err(rejection) => done(steps, layer_a, rejected(rejection)),
        Ok(scores) => match done(steps, layer_a, Verdict::Passed(scores)) {
            Ended::Done { attempt, .. } => Ended::Done {
                attempt,
                candidate: Some(answer),
            },
            other => other,
        },
    }
}
