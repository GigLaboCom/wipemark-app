//! A template tried before it is trusted: "Check template" and "Adapt
//! with the model" (E4-6c R4, R5; D54's shape, D74, Q-B22).
//!
//! Both send a request to the engine a caller hands in, and neither is a
//! rewrite of a document:
//!
//! * [`plan_trial`] and [`run_trial`] — a fixed built-in **sample** in a
//!   language ([`sample`]: one paragraph with a number, a name, inline
//!   code and a link, so `⟦n⟧` is exercised) rewritten by a tactic with
//!   one template **as edited, not as saved**, laid over the saved ones.
//!   The whole tactic runs — both steps of `back_translate` and of
//!   `structural` — because the loop judges the final answer against the
//!   chunk, and a step judged alone would be judged against the wrong
//!   text. What comes back is judged by the loop's own code: Layer A,
//!   every guard (each one's verdict, not only the first refusal), then
//!   [`crate::job::verdict`] — D117's one verdict.
//! * [`adapt_with`] — [`super::adaptation_request`] for a source template,
//!   the answer through [`super::clean_response`], then
//!   [`super::row::admit_adaptation`]: the rule a Save asks, plus exactly
//!   the source's variables. The caller stores the row only when it is
//!   admitted, and only because a person pressed a button: nothing here is
//!   ever called on its own (Q-B22).
//!
//! Values only. The surface words every one of them, and runs these on an
//! executor of its own — never the thread that draws a window.

use std::time::{Duration, Instant};

use wipemark_core::GuardOutcome;
use wipemark_engine::{
    CancellationToken, ChatRequest, EngineError, FinishReason, RewriteEngine, SamplingParams,
    TokenSink, Unavailable,
};

use super::adapt::{adaptation_request, AdaptRefusal};
use super::choose::{templates_for, Overrides, Plan};
use super::clean::{clean_response, Stripped};
use super::render::{render, Input, RenderError, Rendered};
use super::row::{admit_adaptation, hash, AdaptedFrom, Admission, Origin, Override};
use super::validate::{validate, Problem, Severity, ValidationContext};
use super::{shipped, Intensity, Slot, Tactic};
use crate::job::{verdict, Options};
use crate::lang::Lang;
use crate::prepare::{estimate_tokens, prepare, Budget, Chunk, TextFormat};
use crate::report::{EngineFailure, Rejection};
use crate::select::Scores;

/// The built-in sample of a language (Г3): one Markdown paragraph of more
/// than twenty words — so the language check applies — with a date and
/// numbers (`NumbersGuard`), a person's name, an inline command and a link
/// — protected spans: the command as `⟦1⟧`, the link's markup either side
/// of its words as `⟦2⟧` and `⟦3⟧`. Never the person's own text.
pub fn sample(lang: Lang) -> &'static str {
    match lang {
        Lang::En => {
            "On 14 March 2024 Maria Keller ran `cargo build --release` on the old \
             server, and the build took 37 minutes, so the team moved the job to a \
             faster machine described in [the runbook](https://example.com/runbook). \
             The next build finished in 9 minutes."
        }
        Lang::Ru => {
            "14 марта 2024 года Мария Келлер запустила `cargo build --release` на \
             старом сервере, и сборка заняла 37 минут, поэтому команда перенесла \
             задачу на более быструю машину, описанную в \
             [инструкции](https://example.com/runbook). Следующая сборка закончилась \
             за 9 минут."
        }
        Lang::De => {
            "Am 14. März 2024 startete Maria Keller `cargo build --release` auf dem \
             alten Server, und der Build dauerte 37 Minuten, deshalb verlegte das \
             Team die Aufgabe auf eine schnellere Maschine, die im \
             [Handbuch](https://example.com/runbook) beschrieben ist. Der nächste \
             Build war nach 9 Minuten fertig."
        }
    }
}

/// The language whose default pivot is `pivot` (D60): the document a
/// `back_translate` first step written in `pivot` translates from.
fn document_for_pivot(pivot: Lang) -> Lang {
    match pivot {
        Lang::De => Lang::Ru,
        Lang::Ru => Lang::En,
        Lang::En => Lang::De,
    }
}

/// Why a template cannot be tried.
#[derive(Debug, Clone, PartialEq)]
pub enum TrialRefusal {
    /// `code` is not in this version (D73): no job runs it, so there is
    /// nothing a check of it would stand for.
    NotInThisVersion,
    /// A template of the tactic breaks a rule — the one edited, or a saved
    /// one of the other turn or step — and would not render. `problems`
    /// are its errors.
    Template { slot: Slot, problems: Vec<Problem> },
}

/// One check, planned: what is asked, of which text, with which templates.
#[derive(Debug, Clone, PartialEq)]
pub struct Trial {
    /// The template being checked.
    pub slot: Slot,
    /// The sample's language — the document's, as far as the tactic goes.
    pub doc: Lang,
    /// The sample, prepared: one chunk, its protected spans as `⟦n⟧`.
    pub chunk: Chunk,
    /// Every step of the tactic, the edited template in its place.
    pub plan: Plan,
}

/// Plan a check of `edited` as the template of `slot`, beside the
/// overrides in `saved` and the pivot row (D60).
///
/// The sample is in the language the slot's step is written for: the
/// slot's own, except for `back_translate`'s first step, which is written
/// in the pivot and translates *into* it — there the sample is in the
/// language whose default pivot that is, and the slot's language is the
/// pivot whatever the row says, because that is the template under check.
pub fn plan_trial(
    slot: Slot,
    edited: &str,
    saved: &Overrides,
    pivot_row: Option<Lang>,
) -> Result<Trial, TrialRefusal> {
    if slot.tactic() == Tactic::Code {
        return Err(TrialRefusal::NotInThisVersion);
    }
    let (doc, pivot_row) = match (slot.tactic(), slot.step()) {
        (Tactic::BackTranslate, 1) => (document_for_pivot(slot.lang()), Some(slot.lang())),
        _ => (slot.lang(), pivot_row),
    };
    let mut overrides = saved.clone();
    let row = match saved.get(slot) {
        Some(row) => Override {
            text: edited.to_owned(),
            ..row.clone()
        },
        None => Override::by_hand(slot, edited),
    };
    overrides.insert(slot, row);
    let plan = templates_for(Some(doc), slot.tactic(), pivot_row, &overrides)
        .expect("only a document of no language is refused, and the sample has one");
    for step in &plan.steps {
        for (this, other) in [(&step.system, &step.user), (&step.user, &step.system)] {
            let context = ValidationContext {
                other_role: &other.text,
                ctx_len: None,
                intensity: Intensity::Moderate,
                based_on: None,
            };
            let problems: Vec<Problem> = validate(this.slot, &this.text, &context)
                .into_iter()
                .filter(|problem| problem.severity() == Severity::Error)
                .collect();
            if !problems.is_empty() {
                return Err(TrialRefusal::Template {
                    slot: this.slot,
                    problems,
                });
            }
        }
    }
    let prepared = prepare(sample(doc), TextFormat::Markdown, Budget::DEFAULT);
    let chunk = prepared
        .chunks()
        .first()
        .cloned()
        .expect("every sample is one paragraph: the_samples_are_one_chunk_with_placeholders");
    Ok(Trial {
        slot,
        doc,
        chunk,
        plan,
    })
}

/// One step of a check, as it went.
#[derive(Debug, Clone, PartialEq)]
pub struct TrialStep {
    /// 1 or 2.
    pub step: u8,
    /// The language the step's templates are written in.
    pub lang: Lang,
    /// What was sent: the rendered system and user turns.
    pub request: Rendered,
    /// The answer, cleaned (D67).
    pub answer: String,
    /// What the clean-up took off.
    pub stripped: Vec<Stripped>,
    pub tokens_out: u32,
    pub elapsed: Duration,
}

/// One guard's own verdict on the final answer — each guard asked, not
/// only up to the first that refuses.
#[derive(Debug, Clone, PartialEq)]
pub struct GuardVerdict {
    /// `placeholder`, `numbers`, `length-drift`, `script`, `identifier`.
    pub guard: &'static str,
    pub outcome: GuardOutcome,
}

/// What a check found.
#[derive(Debug, Clone, PartialEq)]
pub struct TrialReport {
    /// The steps that ran, in order; fewer than the tactic's when one
    /// failed.
    pub steps: Vec<TrialStep>,
    /// The final answer through Layer A — what the guards judged. `None`
    /// when a step failed before there was one.
    pub answer: Option<String>,
    /// Every guard's verdict on it; empty without an answer.
    pub guards: Vec<GuardVerdict>,
    /// The loop's verdict (D117): the scores of an answer that would have
    /// been a candidate, or why it would have been rejected.
    pub verdict: Result<Scores, Rejection>,
    pub elapsed: Duration,
}

/// How a check ended.
#[derive(Debug, Clone, PartialEq)]
pub enum TrialEnd {
    Done(Box<TrialReport>),
    Cancelled,
    /// Nothing could answer.
    Unavailable(Unavailable),
    /// A template refused at render — the plan makes this unreachable.
    Refused(TrialRefusal),
}

/// The sampling one call is asked with: the options' own, a fixed seed,
/// and the loop's budget when none is set (twice the input plus 64).
fn params_for(options: &Options, input: &str) -> SamplingParams {
    let mut params = options.sampling.clone();
    params.seed = Some(options.base_seed);
    if params.max_tokens.is_none() {
        params.max_tokens = Some(estimate_tokens(input).saturating_mul(2).saturating_add(64));
    }
    params
}

/// Ask once and wait, the tokens drained as they come.
async fn ask(
    engine: &dyn RewriteEngine,
    request: ChatRequest,
    cancel: &CancellationToken,
) -> Result<wipemark_engine::Completion, EngineError> {
    let (sink, tokens): (TokenSink, _) = flume::unbounded();
    let answer = engine.complete(request, sink, cancel.clone()).await;
    drop(tokens);
    answer
}

/// Run a planned check on `engine` under `options` (sampling, Layer A,
/// the length windows — the job's own). Cancelled by `cancel` between and
/// within calls.
pub async fn run_trial(
    trial: &Trial,
    engine: &dyn RewriteEngine,
    options: &Options,
    cancel: &CancellationToken,
) -> TrialEnd {
    let started = Instant::now();
    let chunk = &trial.chunk;
    let mut steps = Vec::new();
    let mut text = chunk.text.clone();
    let ended = |steps, rejection| {
        TrialEnd::Done(Box::new(TrialReport {
            steps,
            answer: None,
            guards: Vec::new(),
            verdict: Err(rejection),
            elapsed: started.elapsed(),
        }))
    };

    for step in &trial.plan.steps {
        let input = Input {
            text: &text,
            context: chunk.context.as_deref(),
            intensity: options.intensity,
        };
        let rendered = match render(step, &input) {
            Ok(rendered) => rendered,
            Err(RenderError::MarkerInText { marker }) => {
                return ended(
                    steps,
                    Rejection::MarkerInAnswer {
                        step: step.step,
                        marker,
                    },
                )
            }
            Err(RenderError::Template { slot, problems }) => {
                return TrialEnd::Refused(TrialRefusal::Template { slot, problems })
            }
        };
        let params = params_for(options, &text);
        let begun = Instant::now();
        let completion = match ask(engine, rendered.clone().into_request(params), cancel).await {
            Ok(completion) => completion,
            Err(error) => match EngineFailure::of(error) {
                Ok(failure) => {
                    return ended(
                        steps,
                        Rejection::Engine {
                            step: step.step,
                            failure,
                        },
                    )
                }
                Err(EngineError::Unavailable(why)) => return TrialEnd::Unavailable(why),
                Err(_) => return TrialEnd::Cancelled,
            },
        };
        let cleaned = clean_response(&completion.text, &text);
        steps.push(TrialStep {
            step: step.step,
            lang: step.user.slot.lang(),
            request: rendered,
            answer: cleaned.text.clone(),
            stripped: cleaned.stripped,
            tokens_out: completion.tokens_out,
            elapsed: begun.elapsed(),
        });
        match completion.finish {
            FinishReason::Cancelled => return TrialEnd::Cancelled,
            FinishReason::Length => return ended(steps, Rejection::Truncated { step: step.step }),
            FinishReason::Stop => {}
        }
        if cleaned.text.trim().is_empty() {
            return ended(steps, Rejection::Empty { step: step.step });
        }
        text = cleaned.text;
    }

    let answer = wipemark_core::clean(&text, &options.layer_a).text;
    let guards = options
        .guards_for(chunk)
        .iter()
        .map(|guard| GuardVerdict {
            guard: guard.name(),
            outcome: guard.check(&chunk.text, &answer),
        })
        .collect();
    let verdict = verdict(options, chunk, &answer);
    TrialEnd::Done(Box::new(TrialReport {
        steps,
        answer: Some(answer),
        guards,
        verdict,
        elapsed: started.elapsed(),
    }))
}

/// How an adaptation by the model ended.
#[derive(Debug, Clone, PartialEq)]
pub enum AdaptEnd {
    /// The model answered. `row` is what would be stored — `origin:
    /// machine`, `adapted_from` naming the source and its hash — and
    /// `admission` says whether it may be: the caller stores it only when
    /// it is admitted.
    Answered {
        row: Override,
        admission: Admission,
        stripped: Vec<Stripped>,
        tokens_out: u32,
        elapsed: Duration,
    },
    /// No request was written ([`adaptation_request`]'s refusal).
    NotAsked(AdaptRefusal),
    /// The engine failed in a way that leaves it usable.
    Failed(EngineFailure),
    /// The answer was cut at its budget.
    Truncated,
    /// The answer was empty once cleaned.
    Empty,
    Cancelled,
    Unavailable(Unavailable),
}

/// Ask `engine` to adapt `source_text` — the template of `source` as it is
/// used now — into `target`'s language, and judge the answer as the
/// override of `target` beside `saved` (the rule a Save asks, with the
/// window when known) plus the source's variables.
pub async fn adapt_with(
    engine: &dyn RewriteEngine,
    source: Slot,
    source_text: &str,
    target: Slot,
    saved: &Overrides,
    ctx_len: Option<u32>,
    cancel: &CancellationToken,
) -> AdaptEnd {
    let started = Instant::now();
    let request = match adaptation_request(source, source_text, target.lang()) {
        Ok(request) => request,
        Err(refusal) => return AdaptEnd::NotAsked(refusal),
    };
    let params = SamplingParams {
        // An adaptation is a careful translation of an instruction, not a
        // rewrite: less heat than a candidate, and room for a template
        // that grows in another language.
        temperature: 0.3,
        seed: Some(0),
        max_tokens: Some(
            estimate_tokens(source_text)
                .saturating_mul(3)
                .saturating_add(128),
        ),
        ..SamplingParams::default()
    };
    let completion = match ask(engine, request.into_request(params), cancel).await {
        Ok(completion) => completion,
        Err(error) => {
            return match EngineFailure::of(error) {
                Ok(failure) => AdaptEnd::Failed(failure),
                Err(EngineError::Unavailable(why)) => AdaptEnd::Unavailable(why),
                Err(_) => AdaptEnd::Cancelled,
            }
        }
    };
    match completion.finish {
        FinishReason::Cancelled => return AdaptEnd::Cancelled,
        FinishReason::Length => return AdaptEnd::Truncated,
        FinishReason::Stop => {}
    }
    let cleaned = clean_response(&completion.text, source_text);
    let text = cleaned.text.trim().to_owned();
    if text.is_empty() {
        return AdaptEnd::Empty;
    }
    let row = Override {
        text,
        based_on: hash(shipped::template(target).unwrap_or_default()),
        adapted_from: Some(AdaptedFrom {
            lang: source.lang(),
            hash: hash(source_text),
        }),
        origin: Origin::Machine,
    };
    let admission = admit_adaptation(target, &row, source_text, saved, ctx_len);
    AdaptEnd::Answered {
        row,
        admission,
        stripped: cleaned.stripped,
        tokens_out: completion.tokens_out,
        elapsed: started.elapsed(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use wipemark_core::GuardOutcome;
    use wipemark_engine::fake::FakeEngine;
    use wipemark_engine::CancellationToken;

    use super::{adapt_with, plan_trial, run_trial, sample, AdaptEnd, TrialEnd, TrialRefusal};
    use crate::cost::Executor;
    use crate::job::{block_on, Options};
    use crate::lang::{self, Lang};
    use crate::prepare::{prepare, Budget, TextFormat};
    use crate::prompt::row::{hash, Origin, Override};
    use crate::prompt::{shipped, Overrides, Role, Slot, Tactic};
    use crate::report::Rejection;

    fn slot(lang: Lang, tactic: Tactic, step: u8, role: Role) -> Slot {
        Slot::new(lang, tactic, step, role).expect("a slot")
    }

    fn options() -> Options {
        Options::for_executor(Executor::LocalCpu)
    }

    /// Every sample is what Г3 promises: one chunk, in its language, with
    /// protected spans (`⟦1⟧` to `⟦3⟧`: the command and the link's markup)
    /// and a number.
    #[test]
    fn the_samples_are_one_chunk_with_placeholders() {
        for lang in Lang::ALL {
            let prepared = prepare(sample(lang), TextFormat::Markdown, Budget::DEFAULT);
            assert_eq!(prepared.chunks().len(), 1, "{lang:?}");
            let chunk = &prepared.chunks()[0];
            assert_eq!(chunk.protected.len(), 3, "{lang:?}: {}", chunk.text);
            for n in ["⟦1⟧", "⟦2⟧", "⟦3⟧"] {
                assert!(chunk.text.contains(n), "{lang:?}: {}", chunk.text);
            }
            assert!(chunk.text.contains("37"));
            assert_eq!(prepared.language(), Some(lang), "{lang:?}");
            // What the loop's language check reads (D95): the chunk with
            // its placeholders removed.
            assert_eq!(
                lang::detect(&crate::prepare::without_placeholders(&chunk.text)),
                Some(lang),
                "{lang:?}"
            );
        }
    }

    /// The request carries the text as edited, not as saved: the person
    /// checks before saving.
    #[test]
    fn a_check_asks_with_the_edited_text_not_the_saved_one() {
        let user = slot(Lang::En, Tactic::Paraphrase, 1, Role::User);
        let mut saved = Overrides::new();
        saved.insert(user, Override::by_hand(user, "SAVED wording.\n{TEXT}"));
        let trial =
            plan_trial(user, "EDITED wording.\n{TEXT}", &saved, None).expect("a valid edit");
        let engine = FakeEngine::answering(|_, _| "anything".to_owned());
        let end = block_on(run_trial(
            &trial,
            &engine,
            &options(),
            &CancellationToken::new(),
        ));
        assert!(matches!(end, TrialEnd::Done(_)));
        let asked = engine.asked();
        assert_eq!(asked.len(), 1);
        assert!(asked[0].prompt.contains("EDITED wording."));
        assert!(!asked[0].prompt.contains("SAVED wording."));
        assert!(asked[0].prompt.contains("⟦1⟧"), "the sample's placeholders");
    }

    /// An answer that drops a placeholder is shown as the placeholder
    /// guard's refusal — among every guard's own verdict — and as the
    /// loop's verdict.
    #[test]
    fn an_answer_that_drops_a_placeholder_is_the_placeholder_guards_refusal() {
        let user = slot(Lang::En, Tactic::Paraphrase, 1, Role::User);
        let edited = shipped::template(user).expect("shipped");
        let trial = plan_trial(user, edited, &Overrides::new(), None).expect("plan");
        let dropped = trial.chunk.text.replace("⟦2⟧", "the guide");
        let engine = FakeEngine::answering(move |_, _| dropped.clone());
        let TrialEnd::Done(report) = block_on(run_trial(
            &trial,
            &engine,
            &options(),
            &CancellationToken::new(),
        )) else {
            panic!("the check did not end with a report");
        };
        let placeholder = report
            .guards
            .iter()
            .find(|guard| guard.guard == "placeholder")
            .expect("the placeholder guard was asked");
        assert!(
            matches!(placeholder.outcome, GuardOutcome::Reject(_)),
            "{placeholder:?}"
        );
        assert!(
            report.guards.len() > 1,
            "every guard's verdict, not the first"
        );
        assert!(
            matches!(
                report.verdict,
                Err(Rejection::Guard {
                    guard: "placeholder",
                    ..
                })
            ),
            "{:?}",
            report.verdict
        );
    }

    /// `back_translate`'s first step is checked over a document whose
    /// pivot is the slot's language; both steps run, and the verdict is
    /// on the way back.
    #[test]
    fn a_translation_step_is_checked_through_its_whole_tactic() {
        let first = slot(Lang::De, Tactic::BackTranslate, 1, Role::User);
        let trial = plan_trial(
            first,
            shipped::template(first).expect("shipped"),
            &Overrides::new(),
            Some(Lang::En),
        )
        .expect("plan");
        assert_eq!(trial.doc, Lang::Ru, "Russian goes through German");
        assert_eq!(trial.plan.steps.len(), 2);
        assert_eq!(trial.plan.steps[0].user.slot, first);
        let second = slot(Lang::Ru, Tactic::BackTranslate, 2, Role::User);
        assert_eq!(trial.plan.steps[1].user.slot, second);

        let trial = plan_trial(
            second,
            shipped::template(second).expect("shipped"),
            &Overrides::new(),
            None,
        )
        .expect("plan");
        assert_eq!(trial.doc, Lang::Ru);
        assert_eq!(trial.plan.steps[0].user.slot.lang(), Lang::De, "D60");
    }

    #[test]
    fn a_template_that_would_not_render_cannot_be_checked() {
        let user = slot(Lang::En, Tactic::Paraphrase, 1, Role::User);
        assert!(matches!(
            plan_trial(user, "No text here.", &Overrides::new(), None),
            Err(TrialRefusal::Template { slot, .. }) if slot == user
        ));
        let code = slot(Lang::En, Tactic::Code, 1, Role::User);
        assert_eq!(
            plan_trial(code, "{TEXT}", &Overrides::new(), None),
            Err(TrialRefusal::NotInThisVersion)
        );
    }

    #[test]
    fn a_cancelled_check_ends_cancelled() {
        let user = slot(Lang::En, Tactic::Paraphrase, 1, Role::User);
        let trial = plan_trial(
            user,
            shipped::template(user).expect("shipped"),
            &Overrides::new(),
            None,
        )
        .expect("plan");
        let cancel = CancellationToken::new();
        cancel.cancel();
        let engine = FakeEngine::new();
        assert_eq!(
            block_on(run_trial(&trial, &engine, &options(), &cancel)),
            TrialEnd::Cancelled
        );
    }

    /// A model answer that renames `{TEXT}` is not admitted; a faithful
    /// one is, as `machine`, naming its source and the source's hash.
    #[test]
    fn a_machine_adaptation_is_admitted_only_with_the_sources_variables() {
        let source = slot(Lang::En, Tactic::Paraphrase, 1, Role::User);
        let target = slot(Lang::Ru, Tactic::Paraphrase, 1, Role::User);
        let source_text = shipped::template(source).expect("shipped");
        let renamed = source_text.replace("{TEXT}", "{ТЕКСТ}");
        let engine = Arc::new(FakeEngine::answering(move |_, _| {
            format!("Перепиши текст своими словами.\n{renamed}")
        }));
        let AdaptEnd::Answered { row, admission, .. } = block_on(adapt_with(
            engine.as_ref(),
            source,
            source_text,
            target,
            &Overrides::new(),
            None,
            &CancellationToken::new(),
        )) else {
            panic!("the model answered");
        };
        assert!(!admission.admitted());
        assert!(admission
            .problems
            .iter()
            .any(|problem| problem.rule() == "variables-differ"));
        assert_eq!(row.origin, Origin::Machine);

        let faithful = source_text
            .lines()
            .map(|line| {
                if line.contains('{') {
                    line.to_owned()
                } else {
                    "Перепиши этот текст другими словами.".to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let engine = FakeEngine::answering(move |_, _| faithful.clone());
        let AdaptEnd::Answered { row, admission, .. } = block_on(adapt_with(
            &engine,
            source,
            source_text,
            target,
            &Overrides::new(),
            None,
            &CancellationToken::new(),
        )) else {
            panic!("the model answered");
        };
        assert!(admission.admitted(), "{:?}", admission.problems);
        assert_eq!(row.origin, Origin::Machine);
        let from = row.adapted_from.expect("an adaptation names its source");
        assert_eq!(from.lang, Lang::En);
        assert_eq!(from.hash, hash(source_text));
        assert_eq!(engine.asked().len(), 1);
    }
}
