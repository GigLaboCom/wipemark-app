//! Everything decided before the first call: Layer A over the document,
//! the chunks, the rungs of the ladder this document can use and their
//! templates — and from them, the price.
//!
//! A pure function of the document, the options and what the engine says
//! of itself. The job runs it on its own thread; a surface that wants the
//! price before the run (D61) runs it too — off the GPUI thread, because
//! Layer A over a large document is not free.

use wipemark_core::Cleaned;
use wipemark_engine::EngineInfo;

use super::{Document, Options};
use crate::cost::{Bound, Cost};
use crate::lang::Lang;
use crate::prepare::{estimate_tokens, prepare, Budget, Chunk, Prepared};
use crate::prompt::{render, templates_for, Input, Overrides, Plan, RenderError, Tactic};
use crate::report::{SkippedTactic, TemplateFallback};
use crate::PipelineError;

/// The smallest chunk budget planning hands to `prepare`, whatever the
/// window: below it every sentence would be cut at its words.
const MIN_BUDGET: u32 = 64;

/// What a probe render puts where the text and the context go: one word
/// each, so the overhead measured is the templates' own.
const PROBE: &str = "x";

/// One rung of the ladder this document can use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rung {
    pub tactic: Tactic,
    /// The templates of every step, chosen and proven to render.
    pub plan: Plan,
}

/// A job, planned.
#[derive(Debug, Clone, PartialEq)]
pub struct Planned {
    /// Layer A over the document as handed over: the text everything
    /// after it starts from, and the "before" of the report.
    pub before: Cleaned,
    /// The cleaned text, prepared.
    pub prepared: Prepared,
    /// The chunk budget: D70's, less the prompt around the chunk.
    pub budget: Budget,
    /// The usable rungs, in ladder order.
    pub rungs: Vec<Rung>,
    /// The rungs this document cannot use, and why.
    pub skipped: Vec<SkippedTactic>,
    /// Overrides that did not render and were replaced by the shipped
    /// template.
    pub fallbacks: Vec<TemplateFallback>,
    /// The overrides still in force after the fallbacks.
    pub overrides: Overrides,
}

impl Planned {
    /// The document's language, `None` when it could not be told.
    pub fn language(&self) -> Option<Lang> {
        self.prepared.language()
    }

    /// The pivot of the first usable `back_translate` rung.
    pub fn pivot(&self) -> Option<Lang> {
        self.rungs.iter().find_map(|rung| rung.plan.pivot)
    }

    /// The rung round `round` (1-based) climbs to: one per round, the last
    /// repeated when there are more rounds than rungs.
    pub fn rung(&self, round: u8) -> Option<&Rung> {
        let last = self.rungs.len().checked_sub(1)?;
        self.rungs
            .get(usize::from(round.saturating_sub(1)).min(last))
    }

    /// What the job will ask for (D61, OV §4.4): the calls exactly, the
    /// tokens estimated over the real prompts, and the time from
    /// `tokens_per_second` when the caller has a measured rate.
    ///
    /// Per chunk and round: `candidates × steps` calls; each call's prompt
    /// is `estimate_tokens` of its rendered system and user turns (a second
    /// step's input is unknown before the first answers, and the chunk
    /// stands in for it); each call's answer is the chunk's `est_tokens`.
    /// `expected` is every chunk passing in its first round.
    pub fn cost(&self, options: &Options, tokens_per_second: Option<f32>) -> Cost {
        let candidates = u64::from(options.effort.candidates);
        let mut calls = Bound {
            worst: 0u64,
            expected: 0u64,
        };
        let mut tokens_in = Bound {
            worst: 0u64,
            expected: 0u64,
        };
        let mut tokens_out = Bound {
            worst: 0u64,
            expected: 0u64,
        };
        let mut chunks = 0u32;
        for chunk in self.prepared.chunks() {
            if self.rungs.is_empty() || self.unrenderable(chunk, options) {
                continue;
            }
            chunks += 1;
            for round in 1..=options.effort.rounds {
                let Some(rung) = self.rung(round) else {
                    continue;
                };
                let steps = u64::from(rung.tactic.steps());
                let prompt: u64 = rung
                    .plan
                    .steps
                    .iter()
                    .map(|step| {
                        let input = Input {
                            text: &chunk.text,
                            context: chunk.context.as_deref(),
                            intensity: options.intensity,
                        };
                        render(step, &input).map_or(0, |r| {
                            u64::from(estimate_tokens(&r.system) + estimate_tokens(&r.prompt))
                        })
                    })
                    .sum();
                let out = steps * u64::from(chunk.est_tokens);
                calls.worst += candidates * steps;
                tokens_in.worst += candidates * prompt;
                tokens_out.worst += candidates * out;
                if round == 1 {
                    calls.expected += candidates * steps;
                    tokens_in.expected += candidates * prompt;
                    tokens_out.expected += candidates * out;
                }
            }
        }
        let seconds = tokens_per_second
            .filter(|rate| rate.is_finite() && *rate > 0.0)
            .map(|rate| Bound {
                worst: tokens_out.worst as f64 / f64::from(rate),
                expected: tokens_out.expected as f64 / f64::from(rate),
            });
        Cost {
            chunks,
            calls: Bound {
                worst: u32::try_from(calls.worst).unwrap_or(u32::MAX),
                expected: u32::try_from(calls.expected).unwrap_or(u32::MAX),
            },
            tokens_in,
            tokens_out,
            seconds,
        }
    }

    /// Whether `chunk` cannot be sent at all: its text or its context
    /// carries one of the assembler's markers (the first rung's first step
    /// is where that shows).
    pub(crate) fn unrenderable(&self, chunk: &Chunk, options: &Options) -> bool {
        self.marker_in(chunk, options).is_some()
    }

    pub(crate) fn marker_in(
        &self,
        chunk: &Chunk,
        options: &Options,
    ) -> Option<crate::prompt::Marker> {
        let step = self.rungs.first()?.plan.steps.first()?;
        let input = Input {
            text: &chunk.text,
            context: chunk.context.as_deref(),
            intensity: options.intensity,
        };
        match render(step, &input) {
            Err(RenderError::MarkerInText { marker }) => Some(marker),
            _ => None,
        }
    }
}

/// Plan `document` for `options` on an engine that says `engine` of
/// itself.
///
/// Fails only when a **shipped** template does not render — a bug in this
/// build, never the user's doing. An override that does not render is
/// replaced by the shipped template and recorded; a rung the document
/// cannot use (`back_translate` over an undetected language) is skipped
/// and recorded.
pub fn plan(
    document: &Document,
    options: &Options,
    engine: &EngineInfo,
) -> Result<Planned, PipelineError> {
    let before = wipemark_core::clean(&document.text, &options.layer_a);
    let mut overrides = options.overrides.clone();
    let mut fallbacks = Vec::new();

    let first_budget = Budget::for_context(engine.ctx_len);
    let mut prepared = prepare(&before.text, document.format, first_budget);
    let (mut rungs, mut skipped) =
        rungs_for(prepared.language(), options, &mut overrides, &mut fallbacks)?;

    let budget = match engine.ctx_len {
        Some(ctx) => {
            let overhead = overhead(&rungs, options);
            let budget = Budget::for_context(Some(ctx.saturating_sub(overhead)));
            Budget {
                max_tokens: budget.max_tokens.max(MIN_BUDGET),
            }
        }
        None => Budget::DEFAULT,
    };
    if budget != first_budget {
        let language = prepared.language();
        prepared = prepare(&before.text, document.format, budget);
        // Chunks cut differently join their prose differently; the
        // detection is over that prose, so it is asked again.
        if prepared.language() != language {
            (rungs, skipped) =
                rungs_for(prepared.language(), options, &mut overrides, &mut fallbacks)?;
        }
    }

    Ok(Planned {
        before,
        prepared,
        budget,
        rungs,
        skipped,
        fallbacks,
        overrides,
    })
}

/// The usable rungs for a document in `language`, their templates proven
/// to render; the skipped ones. An override that does not render is taken
/// out of `overrides` and recorded in `fallbacks`.
fn rungs_for(
    language: Option<Lang>,
    options: &Options,
    overrides: &mut Overrides,
    fallbacks: &mut Vec<TemplateFallback>,
) -> Result<(Vec<Rung>, Vec<SkippedTactic>), PipelineError> {
    let mut rungs = Vec::new();
    let mut skipped = Vec::new();
    for &tactic in &options.ladder {
        loop {
            let plan = match templates_for(language, tactic, options.pivot, overrides) {
                Ok(plan) => plan,
                Err(refusal) => {
                    skipped.push(SkippedTactic { tactic, refusal });
                    break;
                }
            };
            match probe(&plan, options) {
                Ok(()) => {
                    rungs.push(Rung { tactic, plan });
                    break;
                }
                Err(RenderError::Template { slot, problems }) => {
                    if overrides.remove(slot).is_none() {
                        return Err(PipelineError::ShippedTemplate { slot });
                    }
                    tracing::warn!(
                        slot = %crate::prompt::row::key(slot),
                        problems = problems.len(),
                        "an override does not render; the shipped template is used"
                    );
                    fallbacks.push(TemplateFallback { slot, problems });
                }
                // A probe carries no marker.
                Err(RenderError::MarkerInText { .. }) => {
                    rungs.push(Rung { tactic, plan });
                    break;
                }
            }
        }
    }
    Ok((rungs, skipped))
}

/// Render every step of `plan` over a one-word probe: the validation a real
/// render would refuse with, found before any chunk is asked.
fn probe(plan: &Plan, options: &Options) -> Result<(), RenderError> {
    for step in &plan.steps {
        render(
            step,
            &Input {
                text: PROBE,
                context: Some(PROBE),
                intensity: options.intensity,
            },
        )?;
    }
    Ok(())
}

/// The largest prompt around a chunk: `estimate_tokens` of the system and
/// user turns of a probe render, over every step of every rung.
fn overhead(rungs: &[Rung], options: &Options) -> u32 {
    rungs
        .iter()
        .flat_map(|rung| &rung.plan.steps)
        .filter_map(|step| {
            render(
                step,
                &Input {
                    text: PROBE,
                    context: None,
                    intensity: options.intensity,
                },
            )
            .ok()
        })
        .map(|r| estimate_tokens(&r.system) + estimate_tokens(&r.prompt))
        .max()
        .unwrap_or(0)
}
