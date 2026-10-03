//! One attempt: one candidate for one chunk, every step of its tactic,
//! and the verdict on what came back.
//!
//! ```text
//! render → into_request(seed, max_tokens) → complete (tokens streamed)
//!        → clean_response                        [per step; step 2 reads step 1's cleaned answer]
//! → Layer A → guards(chunk.text, candidate) → restore → no-op → passed
//! ```
//!
//! Layer A runs **before** the guards: a model that slips U+200B into an
//! identifier must not lose the candidate to `IdentifierGuard` for it —
//! the character is removed and counted, and the identifier is whole
//! again. A `RestoreError` is a rejection like a guard's: list glue out of
//! order is the one thing restore sees that `PlaceholderGuard` cannot.

use std::time::Instant;

use wipemark_core::{Guard, GuardOutcome};
use wipemark_engine::{
    CancellationToken, EngineError, FinishReason, RewriteEngine, TokenSink, Unavailable,
};

use super::drive::block_on;
use super::plan::Rung;
use super::Options;
use crate::prepare::{estimate_tokens, Chunk};
use crate::prompt::{clean_response, render, Input, RenderError};
use crate::report::{Attempt, EngineFailure, Rejection, StepRecord, Verdict};
use crate::select::{self, Scorer, NO_OP_FLOOR};
use crate::PipelineError;

/// What the loop needs to ask for one candidate.
pub(super) struct Ask<'a> {
    pub engine: &'a dyn RewriteEngine,
    pub options: &'a Options,
    pub guards: &'a [Box<dyn Guard>],
    pub cancel: &'a CancellationToken,
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
pub(super) fn run(
    ask: &Ask<'_>,
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
    for guard in ask.guards {
        if let GuardOutcome::Reject(reason) = guard.check(&chunk.text, &answer) {
            return done(
                steps,
                layer_a,
                rejected(Rejection::Guard {
                    guard: guard.name(),
                    reason,
                }),
            );
        }
    }
    if let Err(error) = chunk.restore(&answer) {
        return done(steps, layer_a, rejected(Rejection::Restore(error)));
    }
    let scores = select::score(Scorer::Divergence, &chunk.text, &answer);
    if scores.divergence < NO_OP_FLOOR {
        return done(
            steps,
            layer_a,
            rejected(Rejection::NoOp {
                divergence: scores.divergence,
            }),
        );
    }
    match done(steps, layer_a, Verdict::Passed(scores)) {
        Ended::Done { attempt, .. } => Ended::Done {
            attempt,
            candidate: Some(answer),
        },
        other => other,
    }
}
