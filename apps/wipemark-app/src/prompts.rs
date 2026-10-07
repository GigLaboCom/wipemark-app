//! The Prompts section of the Settings window — "Rewriting" in the
//! sidebar (E4-6c; D60, D64, D67, D73, D74, D75, D77, D117, D330–D3xx).
//!
//! The prompts are data (`crates/wipemark-pipeline/prompts/<lang>/`) and a
//! person may override any template. An override is a row
//! `prompts.<lang>.<tactic>.<step>.<role>` and the pivot of
//! `back_translate` a row `rewrite.pivot`; the CLI and the MCP server read
//! both. This page is what writes them — before it, a template could be
//! changed only with a database client, which is not a setting.
//!
//! # What lives where
//!
//! * The **rule** is the pipeline's: `row::admit` decides whether an
//!   override may be stored, beside the other turn of its step as it will
//!   be used, and `row::lay_over` — the CLI's `--prompts`, the MCP tool's
//!   `templates` — asks the same function (D330). [`save`] is the page's
//!   one road to a stored row, and it asks `admit` before it writes;
//!   `the_page_and_lay_over_accept_the_same_templates` holds the two
//!   together.
//! * The **rows** are `config`'s (`read_prompt_rows`, `write_prompt`,
//!   `forget_prompt`, `read_pivot`, `write_pivot`). A row this build cannot
//!   read is shown and **left** (D74).
//! * The **check** and the **adaptation** are `wipemark_pipeline::prompt::trial`:
//!   a built-in sample through the edited template on the engine on duty,
//!   judged by the loop's own verdict (D117); a source template adapted by
//!   the model into another language, judged by the rule plus the source's
//!   variables. Both reach the engine through the [`EngineHandle`] the MCP
//!   server holds, so a check is counted busy and an **Unload now** waits
//!   for it (D51). Both run only on a button (Q-B22).
//! * The **words** are here, over values, so they can be read without a
//!   window: every sentence is a [`Line`] — a catalogue message and its
//!   arguments — and rule ids, row keys, tactic ids and variable names are
//!   formats, shown as themselves and never translated.
//! * [`PromptsPage`] is the view: the list of every slot, the editor of
//!   the one chosen, and what it says about it.

use std::collections::BTreeMap;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, px, App, Context, Entity, SharedString, Subscription, Window};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{InputEvent, Textarea, TextareaState};
use gpui_component::select::{Select, SelectEvent, SelectState};
use gpui_component::tag::Tag;
use gpui_component::{
    h_flex, v_flex, ActiveTheme, Disableable as _, IndexPath, Sizable, StyledExt as _,
};
use wipemark_core::RejectReason;
use wipemark_engine::{CancellationToken, EngineError};
use wipemark_i18n::{args, t, t_args, FluentArgs, Message};
use wipemark_models::manifest::Role as ModelRole;
use wipemark_pipeline::cost::Executor;
use wipemark_pipeline::lang::Lang;
use wipemark_pipeline::prepare::RestoreError;
use wipemark_pipeline::prompt::row::{self, admit, hash, AdaptedFrom, Admission, Origin, Override};
use wipemark_pipeline::prompt::shipped::{self, Fragment};
use wipemark_pipeline::prompt::validate::{BraceSide, Problem, Script, Severity};
use wipemark_pipeline::prompt::{
    adapt_with, plan_trial, run_trial, staleness, AdaptEnd, AdaptRefusal, Overrides, Role, Slot,
    Stripped, Tactic, TrialEnd, TrialRefusal, Variable,
};
use wipemark_pipeline::{EngineFailure, Options, Rejection};
use wipemark_store::Store;

use crate::config::{self, PivotRow, PromptRow, SettingsStore};
use crate::diff::Diff;
use crate::duty::{Duty, Performer};
use crate::engine::{self, Choice};
use crate::engine_host::{self, EngineHandle};
use crate::icon::IconName;
use crate::settings::{engine_banner, Preferences, Tone};

// ─── Words ───────────────────────────────────────────────────────────

/// One sentence, before it is rendered: a catalogue message and its
/// arguments. Values, so a test can ask *which* sentence without a
/// process-wide language another test may be moving.
pub type Line = (Message, FluentArgs<'static>);

fn plain(message: Message) -> Line {
    (message, FluentArgs::new())
}

/// A line in the language on screen.
pub fn say(line: &Line) -> String {
    t_args(line.0, &line.1)
}

/// A language's name, in the language on screen.
pub fn lang_message(lang: Lang) -> Message {
    match lang {
        Lang::En => Message::PromptsLangEn,
        Lang::Ru => Message::PromptsLangRu,
        Lang::De => Message::PromptsLangDe,
    }
}

fn lang_name(lang: Lang) -> String {
    t(lang_message(lang))
}

/// The languages the page lists, in the order it lists them: English,
/// the set the others were adapted from, first.
const LANGS: [Lang; 3] = [Lang::En, Lang::Ru, Lang::De];

/// A variable as a template spells it — a format.
fn spelled(variable: Variable) -> String {
    format!("{{{}}}", variable.name())
}

/// The 1-based line and column (in characters) of byte offset `at`.
pub fn position(text: &str, at: usize) -> (usize, usize) {
    let at = at.min(text.len());
    let before = text.get(..at).unwrap_or(text);
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |last| last.chars().count())
        + 1;
    (line, column)
}

/// Where a problem is, when it has a place.
fn place_of(problem: &Problem) -> Option<usize> {
    match problem {
        Problem::UnknownVariable { span, .. }
        | Problem::MisplacedVariable { span, .. }
        | Problem::HandWrittenMarker { span, .. } => Some(span.start),
        Problem::UnclosedBrace { at, .. } | Problem::ReservedBracket { at } => Some(*at),
        Problem::RepeatedVariable { spans, .. } => spans.get(1).map(|span| span.start),
        Problem::MissingVariable { .. }
        | Problem::Empty
        | Problem::TooLong { .. }
        | Problem::ScriptMismatch { .. }
        | Problem::NothingButText
        | Problem::NoIntensity { .. }
        | Problem::Stale { .. }
        | Problem::VariablesDiffer { .. } => None,
    }
}

/// The sentence for one problem — exhaustive on purpose, so that a new
/// `Problem` does not compile until it has words (D85's rule). The rule
/// id beside it is a format and is shown as itself.
pub fn problem_line(problem: &Problem) -> Line {
    match problem {
        Problem::UnknownVariable {
            name, suggestion, ..
        } => match suggestion {
            Some(suggestion) => (
                Message::PromptsProblemUnknownVariableSuggest,
                args!("name" => format!("{{{name}}}"), "suggestion" => spelled(*suggestion)),
            ),
            None => (
                Message::PromptsProblemUnknownVariable,
                args!("name" => format!("{{{name}}}")),
            ),
        },
        Problem::UnclosedBrace { side, .. } => plain(match side {
            BraceSide::Open => Message::PromptsProblemUnclosedOpen,
            BraceSide::Close => Message::PromptsProblemUnclosedClose,
        }),
        Problem::MissingVariable { variable } => (
            match variable {
                Variable::Text => Message::PromptsProblemMissingText,
                Variable::Protected => Message::PromptsProblemMissingProtected,
                Variable::PrevContext | Variable::Intensity => Message::PromptsProblemMissingOther,
            },
            args!("variable" => spelled(*variable)),
        ),
        Problem::RepeatedVariable { variable, spans } => (
            Message::PromptsProblemRepeated,
            args!("variable" => spelled(*variable), "count" => spans.len().to_string()),
        ),
        Problem::MisplacedVariable { variable, .. } => (
            Message::PromptsProblemMisplaced,
            args!("variable" => spelled(*variable)),
        ),
        Problem::HandWrittenMarker { marker, .. } => (
            Message::PromptsProblemMarker,
            args!("marker" => marker.as_str().to_owned()),
        ),
        Problem::ReservedBracket { .. } => plain(Message::PromptsProblemBracket),
        Problem::Empty => plain(Message::PromptsProblemEmpty),
        Problem::TooLong {
            estimated_tokens,
            limit,
        } => (
            Message::PromptsProblemTooLong,
            args!("tokens" => estimated_tokens.to_string(), "limit" => limit.to_string()),
        ),
        Problem::ScriptMismatch { expected, .. } => (
            Message::PromptsProblemScript,
            args!("script" => t(match expected {
                Script::Latin => Message::PromptsScriptLatin,
                Script::Cyrillic => Message::PromptsScriptCyrillic,
            })),
        ),
        Problem::NothingButText => plain(Message::PromptsProblemNothingButText),
        Problem::NoIntensity { .. } => (
            Message::PromptsProblemNoIntensity,
            args!("variable" => spelled(Variable::Intensity)),
        ),
        Problem::Stale { .. } => plain(Message::PromptsProblemStale),
        Problem::VariablesDiffer { missing, extra } => {
            let list = |variables: &[Variable]| {
                if variables.is_empty() {
                    t(Message::PromptsNone)
                } else {
                    variables
                        .iter()
                        .map(|variable| spelled(*variable))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            };
            (
                Message::PromptsProblemVariablesDiffer,
                args!("missing" => list(missing), "extra" => list(extra)),
            )
        }
    }
}

/// One problem as the page shows it: its severity, its rule id (a
/// format), its sentence and, where it has one, its place in `text`.
pub fn problem_shown(problem: &Problem, text: &str) -> (Severity, &'static str, String) {
    let mut sentence = say(&problem_line(problem));
    if let Some(at) = place_of(problem) {
        let (line, column) = position(text, at);
        sentence.push(' ');
        sentence.push_str(&t_args(
            Message::PromptsProblemAt,
            &args!("line" => line.to_string(), "column" => column.to_string()),
        ));
    }
    (problem.severity(), problem.rule(), sentence)
}

/// What a guard found missing, in words.
pub fn reason_line(reason: &RejectReason) -> Line {
    let placeholder = |index: &usize| wipemark_pipeline::prepare::placeholder(*index);
    match reason {
        RejectReason::PlaceholderMissing { index } => (
            Message::PromptsReasonPlaceholderMissing,
            args!("placeholder" => placeholder(index)),
        ),
        RejectReason::PlaceholderDuplicated { index, count } => (
            Message::PromptsReasonPlaceholderDuplicated,
            args!("placeholder" => placeholder(index), "count" => count.to_string()),
        ),
        RejectReason::PlaceholderInvented { index } => (
            Message::PromptsReasonPlaceholderInvented,
            args!("placeholder" => placeholder(index)),
        ),
        RejectReason::NumberMissing { value } => (
            Message::PromptsReasonNumberMissing,
            args!("value" => value.clone()),
        ),
        RejectReason::LengthDrift { ratio, min, max } => (
            Message::PromptsReasonLengthDrift,
            args!(
                "ratio" => format!("{ratio:.2}"),
                "min" => format!("{min:.1}"),
                "max" => format!("{max:.1}")
            ),
        ),
        RejectReason::ScriptDrift { script, delta_pp } => (
            Message::PromptsReasonScriptDrift,
            args!("script" => (*script).to_owned(), "points" => format!("{delta_pp:.0}")),
        ),
        RejectReason::IdentifierMissing { token } => (
            Message::PromptsReasonIdentifierMissing,
            args!("token" => token.clone()),
        ),
    }
}

fn failure_words(failure: &EngineFailure) -> String {
    match failure {
        // The engine's or the server's own words, never translated.
        EngineFailure::Transport { detail } | EngineFailure::Protocol { detail } => detail.clone(),
        EngineFailure::ContextOverflow { used, limit } => t_args(
            Message::PromptsFailureOverflow,
            &args!("used" => used.to_string(), "limit" => limit.to_string()),
        ),
        EngineFailure::NotImplemented { what } => (*what).to_owned(),
    }
}

/// Why the loop would reject an answer, in words.
pub fn rejection_line(rejection: &Rejection) -> Line {
    match rejection {
        Rejection::Engine { step, failure } => (
            Message::PromptsRejectedEngine,
            args!("step" => step.to_string(), "reason" => failure_words(failure)),
        ),
        Rejection::Truncated { step } => (
            Message::PromptsRejectedTruncated,
            args!("step" => step.to_string()),
        ),
        Rejection::Empty { step } => (
            Message::PromptsRejectedEmpty,
            args!("step" => step.to_string()),
        ),
        Rejection::Guard { guard, reason } => (
            Message::PromptsRejectedGuard,
            args!("guard" => (*guard).to_owned(), "reason" => say(&reason_line(reason))),
        ),
        Rejection::Language { expected, found } => match found {
            Some(found) => (
                Message::PromptsRejectedLanguage,
                args!("expected" => lang_name(*expected), "found" => lang_name(*found)),
            ),
            None => (
                Message::PromptsRejectedLanguageUnknown,
                args!("expected" => lang_name(*expected)),
            ),
        },
        Rejection::Restore(error) => (
            Message::PromptsRejectedRestore,
            args!("reason" => restore_words(error)),
        ),
        Rejection::NoOp { divergence } => (
            Message::PromptsRejectedNoOp,
            args!(
                "divergence" => format!("{divergence:.2}"),
                "floor" => format!("{:.2}", wipemark_pipeline::select::NO_OP_FLOOR)
            ),
        ),
        Rejection::MarkerInAnswer { step, marker } => (
            Message::PromptsRejectedMarker,
            args!("step" => step.to_string(), "marker" => marker.as_str().to_owned()),
        ),
    }
}

fn restore_words(error: &RestoreError) -> String {
    let placeholder = |index: &usize| wipemark_pipeline::prepare::placeholder(*index);
    match error {
        RestoreError::Unknown { index } => t_args(
            Message::PromptsReasonPlaceholderInvented,
            &args!("placeholder" => placeholder(index)),
        ),
        RestoreError::Missing { index } => t_args(
            Message::PromptsReasonPlaceholderMissing,
            &args!("placeholder" => placeholder(index)),
        ),
        RestoreError::Duplicated { index, count } => t_args(
            Message::PromptsReasonPlaceholderDuplicated,
            &args!("placeholder" => placeholder(index), "count" => count.to_string()),
        ),
        RestoreError::ItemBroken => t(Message::PromptsReasonItemBroken),
    }
}

/// What the clean-up took off an answer (D67), in words.
pub fn stripped_line(stripped: &Stripped) -> Line {
    match stripped {
        Stripped::Think => plain(Message::PromptsStrippedThink),
        Stripped::Marker { marker, count } => (
            Message::PromptsStrippedMarker,
            args!("marker" => marker.as_str().to_owned(), "count" => count.to_string()),
        ),
        Stripped::Fence => plain(Message::PromptsStrippedFence),
        Stripped::Quotes { open, close } => (
            Message::PromptsStrippedQuotes,
            args!("open" => open.to_string(), "close" => close.to_string()),
        ),
    }
}

fn seconds(duration: Duration) -> String {
    format!("{:.1}", duration.as_secs_f64())
}

/// How much of an answer is shown under a check: its first lines.
const SHOWN_LINES: usize = 4;
/// And at most this many characters of them.
const SHOWN_CHARS: usize = 400;

/// The first lines of a model's answer — its own words, shown as output,
/// never as a rewrite of anything (D54).
fn excerpt(text: &str) -> String {
    let mut shown: String = text
        .lines()
        .take(SHOWN_LINES)
        .collect::<Vec<_>>()
        .join("\n");
    if shown.chars().count() > SHOWN_CHARS {
        shown = shown.chars().take(SHOWN_CHARS).collect::<String>();
        shown.push('…');
    } else if text.lines().count() > SHOWN_LINES {
        shown.push_str("\n…");
    }
    shown
}

/// One item of what the page shows under a check or an adaptation: a
/// sentence in a tone, or a model's own words.
pub enum Shown {
    Said(Tone, Line),
    Quoted(String),
}

/// How a check ended, as the page keeps it.
#[derive(Debug, Clone, PartialEq)]
pub enum CheckEnd {
    Ran(TrialEnd),
    /// The template, or a saved one beside it, would not render; or the
    /// tactic is not in this version.
    Refused(TrialRefusal),
    /// The engine could not be had for a reason that is not a refusal —
    /// its own words.
    Failed(String),
}

/// What a check found, as lines.
pub fn check_lines(end: &CheckEnd) -> Vec<Shown> {
    let mut shown = Vec::new();
    match end {
        CheckEnd::Refused(TrialRefusal::NotInThisVersion) => {
            shown.push(Shown::Said(Tone::Quiet, plain(Message::PromptsCheckCode)));
        }
        CheckEnd::Refused(TrialRefusal::Template { slot, .. }) => shown.push(Shown::Said(
            Tone::Warn,
            (
                Message::PromptsCheckOtherBroken,
                args!("key" => row::key(*slot)),
            ),
        )),
        CheckEnd::Failed(reason) => shown.push(Shown::Said(
            Tone::Warn,
            (
                Message::PromptsCheckFailed,
                args!("reason" => reason.clone()),
            ),
        )),
        CheckEnd::Ran(TrialEnd::Cancelled) => {
            shown.push(Shown::Said(
                Tone::Quiet,
                plain(Message::PromptsCheckCancelled),
            ));
        }
        CheckEnd::Ran(TrialEnd::Unavailable(why)) => shown.push(Shown::Said(
            Tone::Warn,
            (
                Message::PromptsCheckFailed,
                args!("reason" => engine_host::refusal_line(why)),
            ),
        )),
        CheckEnd::Ran(TrialEnd::Refused(refusal)) => {
            return check_lines(&CheckEnd::Refused(refusal.clone()));
        }
        CheckEnd::Ran(TrialEnd::Done(report)) => {
            for step in &report.steps {
                shown.push(Shown::Said(
                    Tone::Quiet,
                    (
                        Message::PromptsCheckStep,
                        args!(
                            "step" => step.step.to_string(),
                            "language" => lang_name(step.lang),
                            "tokens" => step.tokens_out.to_string(),
                            "seconds" => seconds(step.elapsed)
                        ),
                    ),
                ));
                shown.push(Shown::Quoted(excerpt(&step.answer)));
                if !step.stripped.is_empty() {
                    let what = step
                        .stripped
                        .iter()
                        .map(|stripped| say(&stripped_line(stripped)))
                        .collect::<Vec<_>>()
                        .join("; ");
                    shown.push(Shown::Said(
                        Tone::Quiet,
                        (Message::PromptsCheckStripped, args!("what" => what)),
                    ));
                }
            }
            for guard in &report.guards {
                shown.push(match guard.outcome.reason() {
                    None => Shown::Said(
                        Tone::Good,
                        (
                            Message::PromptsCheckGuardPassed,
                            args!("guard" => guard.guard.to_owned()),
                        ),
                    ),
                    Some(reason) => Shown::Said(
                        Tone::Warn,
                        (
                            Message::PromptsCheckGuardRejected,
                            args!(
                                "guard" => guard.guard.to_owned(),
                                "reason" => say(&reason_line(reason))
                            ),
                        ),
                    ),
                });
            }
            shown.push(match &report.verdict {
                Ok(scores) => Shown::Said(
                    Tone::Good,
                    (
                        Message::PromptsCheckPassed,
                        args!(
                            "divergence" => format!("{:.2}", scores.divergence),
                            "ratio" => format!("{:.2}", scores.length_ratio)
                        ),
                    ),
                ),
                Err(rejection) => Shown::Said(
                    Tone::Warn,
                    (
                        Message::PromptsCheckRejected,
                        args!("why" => say(&rejection_line(rejection))),
                    ),
                ),
            });
            shown.push(Shown::Said(
                Tone::Quiet,
                (
                    Message::PromptsCheckTime,
                    args!(
                        "tokens" => report
                            .steps
                            .iter()
                            .map(|step| step.tokens_out)
                            .sum::<u32>()
                            .to_string(),
                        "seconds" => seconds(report.elapsed)
                    ),
                ),
            ));
        }
    }
    shown
}

/// How an adaptation ended, as the page keeps it.
#[derive(Debug, Clone, PartialEq)]
pub enum Adapted {
    /// The model answered; `stored` says whether the row was written —
    /// only when the answer was admitted — and why not otherwise.
    Answered {
        admission: Admission,
        text: String,
        stored: Result<bool, String>,
    },
    Ended(AdaptEnd),
    Failed(String),
}

/// What an adaptation came to, as lines.
pub fn adapt_lines(adapted: &Adapted) -> Vec<Shown> {
    match adapted {
        Adapted::Answered {
            stored: Ok(true), ..
        } => vec![Shown::Said(Tone::Good, plain(Message::PromptsAdaptSaved))],
        Adapted::Answered {
            stored: Ok(false),
            text,
            ..
        } => vec![
            Shown::Said(Tone::Warn, plain(Message::PromptsAdaptRefused)),
            Shown::Quoted(excerpt(text)),
        ],
        Adapted::Answered {
            stored: Err(reason),
            ..
        }
        | Adapted::Failed(reason) => vec![Shown::Said(
            Tone::Warn,
            (
                Message::PromptsAdaptFailed,
                args!("reason" => reason.clone()),
            ),
        )],
        Adapted::Ended(end) => vec![match end {
            AdaptEnd::Answered { .. } => Shown::Said(Tone::Quiet, plain(Message::PromptsAdapting)),
            AdaptEnd::NotAsked(refusal) => Shown::Said(
                Tone::Warn,
                (
                    Message::PromptsAdaptFailed,
                    args!("reason" => match refusal {
                        AdaptRefusal::SameLanguage | AdaptRefusal::NoSuchSlot => t(Message::PromptsAdaptNoSuchSlot),
                        AdaptRefusal::MarkerInSource { marker } => t_args(
                            Message::PromptsProblemMarker,
                            &args!("marker" => marker.as_str().to_owned()),
                        ),
                    }),
                ),
            ),
            AdaptEnd::Failed(failure) => Shown::Said(
                Tone::Warn,
                (
                    Message::PromptsAdaptFailed,
                    args!("reason" => failure_words(failure)),
                ),
            ),
            AdaptEnd::Truncated => Shown::Said(Tone::Warn, plain(Message::PromptsAdaptTruncated)),
            AdaptEnd::Empty => Shown::Said(Tone::Warn, plain(Message::PromptsAdaptEmpty)),
            AdaptEnd::Cancelled => Shown::Said(Tone::Quiet, plain(Message::PromptsAdaptCancelled)),
            AdaptEnd::Unavailable(why) => Shown::Said(
                Tone::Warn,
                (
                    Message::PromptsAdaptFailed,
                    args!("reason" => engine_host::refusal_line(why)),
                ),
            ),
        }],
    }
}

/// Who wrote what is in use for a slot, as a line.
pub fn origin_line(row: Option<&PromptRow>) -> Line {
    match row {
        None => plain(Message::PromptsOriginShipped),
        Some(PromptRow::Unread(Some(value))) => {
            (Message::PromptsUnread, args!("value" => value.clone()))
        }
        Some(PromptRow::Unread(None)) => plain(Message::PromptsUnreadNotJson),
        Some(PromptRow::Read(read)) => {
            let source = read
                .adapted_from
                .as_ref()
                .map(|source| lang_name(source.lang))
                .unwrap_or_default();
            match (read.origin, &read.adapted_from) {
                (Origin::Hand, None) => plain(Message::PromptsOriginHand),
                (Origin::Hand, Some(_)) => {
                    (Message::PromptsOriginHandAdapted, args!("source" => source))
                }
                (Origin::Machine, _) => (Message::PromptsOriginMachine, args!("source" => source)),
                (Origin::MachineReviewed, _) => (
                    Message::PromptsOriginMachineReviewed,
                    args!("source" => source),
                ),
            }
        }
    }
}

/// The page's warning ahead (the register's "your edit to `paraphrase`
/// applies only to ru"): a template family overridden in some languages
/// and not in others runs the shipped template in the others.
pub fn coverage_line(slot: Slot, overrides: &Overrides) -> Option<Line> {
    let family: Vec<(Lang, bool)> = LANGS
        .into_iter()
        .filter_map(|lang| {
            slot.with_lang(lang)
                .map(|sibling| (lang, overrides.get(sibling).is_some()))
        })
        .collect();
    let names = |changed: bool| {
        family
            .iter()
            .filter(|(_, has)| *has == changed)
            .map(|(lang, _)| lang_name(*lang))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let (some, all) = (
        family.iter().any(|(_, has)| *has),
        family.iter().all(|(_, has)| *has),
    );
    if !some || all {
        return None;
    }
    Some(if overrides.get(slot).is_some() {
        (
            Message::PromptsCoverageHere,
            args!("here" => lang_name(slot.lang()), "others" => names(false)),
        )
    } else {
        (
            Message::PromptsCoverageElsewhere,
            args!("changed" => names(true), "here" => lang_name(slot.lang())),
        )
    })
}

/// The Prompts page's banner: what a template is, what the product owns,
/// why `{PROTECTED}` is required, who else reads these rows — and, when a
/// rewrite would leave this machine, that the rendered prompt goes with
/// the document.
pub fn banner(duty: &Duty) -> (IconName, Tone, Vec<String>) {
    let mut lines = vec![
        t(Message::PromptsBannerWhat),
        t(Message::PromptsBannerMarkers),
        t_args(
            Message::PromptsBannerProtected,
            &args!("variable" => spelled(Variable::Protected)),
        ),
        t(Message::PromptsBannerSameRows),
    ];
    let away = away(duty);
    if let Some(origin) = &away {
        lines.push(t_args(
            Message::PromptsBannerAway,
            &args!("origin" => origin.clone()),
        ));
    }
    let tone = if away.is_some() {
        Tone::Warn
    } else {
        Tone::Quiet
    };
    (IconName::Pen, tone, lines)
}

/// The origin a request would go to when it leaves this machine — an
/// endpoint on duty that is not loopback — and `None` otherwise.
fn away(duty: &Duty) -> Option<String> {
    match duty.performer() {
        Some(Performer::Endpoint(remote)) if !remote.on_this_machine => Some(remote.origin.clone()),
        _ => None,
    }
}

/// What is said above the Check and Adapt buttons, before either is
/// pressed: that a check is not a rewrite (D54's shape); where the sample
/// and the template go when the duty does not stay here; and, with
/// nobody on duty, the duty's own sentence — the buttons are greyed.
pub fn before_asking(slot: Slot, duty: &Duty) -> Vec<Line> {
    let mut lines = vec![(
        Message::PromptsCheckNote,
        args!("language" => lang_name(sample_language(slot))),
    )];
    if slot.tactic().steps() > 1 {
        lines.push((
            Message::PromptsCheckWholeTactic,
            args!("tactic" => slot.tactic().as_str().to_owned()),
        ));
    }
    if let Some(origin) = away(duty) {
        lines.push((Message::PromptsSentTo, args!("origin" => origin)));
    }
    lines
}

/// The language of the sample a check of `slot` rewrites —
/// `back_translate`'s first step translates *into* its slot's language.
fn sample_language(slot: Slot) -> Lang {
    match (slot.tactic(), slot.step()) {
        (Tactic::BackTranslate, 1) => match slot.lang() {
            Lang::De => Lang::Ru,
            Lang::Ru => Lang::En,
            Lang::En => Lang::De,
        },
        _ => slot.lang(),
    }
}

/// Why the Check and Adapt buttons are greyed, when nobody is on duty:
/// the duty's own sentence, the one the Engine page leads with.
pub fn nobody_on_duty(duty: &Duty) -> Option<String> {
    duty.vacancy().map(|_| {
        engine_banner(duty, false)
            .2
            .into_iter()
            .next()
            .unwrap_or_default()
    })
}

/// Every slot, in the order the page lists them: by language, then
/// tactic, step and turn. `every_template_slot_has_a_row` holds it to
/// what the pipeline accepts.
pub fn listed_slots() -> Vec<Slot> {
    let mut slots = Vec::new();
    for lang in LANGS {
        for tactic in Tactic::ALL {
            for step in 1..=tactic.steps() {
                for role in Role::ALL {
                    slots.extend(Slot::new(lang, tactic, step, role));
                }
            }
        }
    }
    slots
}

/// A sentence about a tactic beside its name in the list, when it has one
/// (Г4): `structural` runs only after a confirmation (D73), `code` is not
/// in this version.
pub fn tactic_note(tactic: Tactic) -> Option<Message> {
    match tactic {
        Tactic::Structural => Some(Message::PromptsTacticStructuralNote),
        Tactic::Code => Some(Message::PromptsTacticCodeNote),
        Tactic::Paraphrase | Tactic::Humanize | Tactic::BackTranslate => None,
    }
}

fn turn_message(role: Role) -> Message {
    match role {
        Role::System => Message::PromptsTurnSystem,
        Role::User => Message::PromptsTurnUser,
    }
}

/// The heading over the editor: the slot in words, and its row key — a
/// format.
pub fn slot_heading(slot: Slot) -> Line {
    (
        Message::PromptsSlotHeading,
        args!(
            "language" => lang_name(slot.lang()),
            "tactic" => slot.tactic().as_str().to_owned(),
            "step" => slot.step().to_string(),
            "turn" => t(turn_message(slot.role()))
        ),
    )
}

/// The sentences the product adds under a slot, read-only (Г7, D77): the
/// placeholder sentence and the context sentence for every user turn; the
/// intensity clauses for the tactics that take one; the English set's
/// fallback clause for an English user turn.
pub fn fragments_of(slot: Slot) -> Vec<(Message, &'static str)> {
    if slot.role() != Role::User {
        return vec![(
            Message::PromptsFragmentProtected,
            shipped::fragment(slot.lang(), Fragment::Protected),
        )];
    }
    let lang = slot.lang();
    let mut fragments = vec![
        (
            Message::PromptsFragmentProtected,
            shipped::fragment(lang, Fragment::Protected),
        ),
        (
            Message::PromptsFragmentContext,
            shipped::fragment(lang, Fragment::Context),
        ),
    ];
    if slot.tactic().takes_intensity() {
        fragments.push((
            Message::PromptsFragmentLight,
            shipped::fragment(lang, Fragment::IntensityLight),
        ));
        fragments.push((
            Message::PromptsFragmentStrong,
            shipped::fragment(lang, Fragment::IntensityStrong),
        ));
    }
    if lang == Lang::En && slot.tactic() != Tactic::Code {
        fragments.push((Message::PromptsFragmentFallback, shipped::fallback_clause()));
    }
    fragments
}

/// The variables table (R6): each variable, where it may stand and
/// whether it is required. `{LANG_NAME}` and `{PIVOT_NAME}` are not
/// among them (D64) — the page says so under the table.
pub fn variables() -> [(Variable, Message); 4] {
    [
        (Variable::Text, Message::PromptsVarText),
        (Variable::PrevContext, Message::PromptsVarPrevContext),
        (Variable::Protected, Message::PromptsVarProtected),
        (Variable::Intensity, Message::PromptsVarIntensity),
    ]
}

/// Two texts line against line, as the page shows a drift: `−` for a
/// line only `before` has, `+` for one only `after` has.
pub fn compared(before: &str, after: &str) -> Vec<(char, String)> {
    let diff = Diff::of(before, after);
    let old: Vec<&str> = before.split_inclusive('\n').collect();
    let new: Vec<&str> = after.split_inclusive('\n').collect();
    let mut lines = Vec::new();
    for row in diff.removed_rows() {
        if let Some(line) = old.get(row as usize) {
            lines.push(('−', line.trim_end_matches('\n').to_owned()));
        }
    }
    for row in diff.added_rows() {
        if let Some(line) = new.get(row as usize) {
            lines.push(('+', line.trim_end_matches('\n').to_owned()));
        }
    }
    lines
}

// ─── The rows: one road to each write ───────────────────────────────

/// What a Save came to.
#[derive(Debug, Clone, PartialEq)]
pub enum Saved {
    /// Written. `admission` carries the warnings, which stay on screen;
    /// `reviewed` when a machine adaptation became reviewed by this save.
    Stored {
        admission: Admission,
        reviewed: bool,
    },
    /// Nothing to store: the text is the shipped template and no row is
    /// there to replace.
    Unchanged,
    /// Refused by the rule: nothing was written.
    Refused(Admission),
    /// The database said no; its words.
    Failed(String),
}

/// Save `text` as `slot`'s override (R1, R3): checked by `row::admit` —
/// the rule `lay_over` asks — beside the other turn as it is stored, with
/// the window when known; written only when admitted, and then as D74's
/// object. Nothing else is touched: saving a source never changes another
/// language's row (Q-B22) — its adaptations become stale, and say so.
///
/// `claim` is the person's word that this is an adaptation of the same
/// template in that language: recorded with the source's hash as it is
/// now. A machine adaptation keeps its source and becomes reviewed.
pub fn save(
    store: &Store,
    slot: Slot,
    text: &str,
    claim: Option<Lang>,
    ctx_len: Option<u32>,
) -> Saved {
    let rows = config::read_prompt_rows(store);
    let overrides = config::overrides_of(&rows);
    let current = match rows.get(&slot) {
        Some(PromptRow::Read(read)) => Some(read),
        _ => None,
    };
    let shipped_text = shipped::template(slot).unwrap_or_default();
    if current.is_none() && !rows.contains_key(&slot) && text == shipped_text && claim.is_none() {
        return Saved::Unchanged;
    }
    let origin = match current.map(|row| row.origin) {
        Some(Origin::Machine | Origin::MachineReviewed) => Origin::MachineReviewed,
        Some(Origin::Hand) | None => Origin::Hand,
    };
    let source = match (origin, current.and_then(|row| row.adapted_from.as_ref())) {
        (Origin::MachineReviewed, Some(source)) => Some(source.lang),
        _ => claim,
    };
    let adapted_from = source
        .filter(|lang| *lang != slot.lang())
        .and_then(|lang| slot.with_lang(lang))
        .map(|source_slot| AdaptedFrom {
            lang: source_slot.lang(),
            hash: hash(overrides.effective(source_slot).unwrap_or_default()),
        });
    let candidate = Override {
        text: text.to_owned(),
        based_on: current
            .map(|row| row.based_on.clone())
            .unwrap_or_else(|| hash(shipped_text)),
        adapted_from,
        origin,
    };
    let admission = admit(slot, &candidate, &overrides, ctx_len);
    if !admission.admitted() {
        return Saved::Refused(admission);
    }
    match config::write_prompt(store, slot, &candidate) {
        Ok(()) => Saved::Stored {
            admission,
            reviewed: current.is_some_and(|row| row.origin == Origin::Machine),
        },
        Err(error) => Saved::Failed(error.to_string()),
    }
}

/// What "Reset to shipped" came to.
#[derive(Debug, Clone, PartialEq)]
pub enum Reset {
    /// The row is gone; the shipped template is in use.
    Done,
    /// Not reset: the step's other turn, as stored, relies on this one —
    /// with the shipped text here it would break `rule` (the placeholder
    /// rule, in practice). Nothing was written.
    Refused {
        other: Slot,
        rule: &'static str,
    },
    Failed(String),
}

/// "Reset to shipped" (R1): delete the slot's row — never write the
/// shipped text into one. Refused when the other turn's override would be
/// left breaking the rule beside the shipped text, because a step that
/// does not render is a job that fails.
pub fn reset(store: &Store, slot: Slot) -> Reset {
    let rows = config::read_prompt_rows(store);
    let mut overrides = config::overrides_of(&rows);
    overrides.remove(slot);
    let other = slot.other_role();
    if let Some(PromptRow::Read(other_row)) = rows.get(&other) {
        if let Some(problem) = admit(other, other_row, &overrides, None).first_error() {
            return Reset::Refused {
                other,
                rule: problem.rule(),
            };
        }
    }
    match config::forget_prompt(store, slot) {
        Ok(()) => Reset::Done,
        Err(error) => Reset::Failed(error.to_string()),
    }
}

/// "Keep mine" (Г6): the shipped template moved on and the person keeps
/// their override — its `based_on` moves to today's shipped hash. The
/// text is not touched, and nothing is merged.
pub fn keep_mine(store: &Store, slot: Slot) -> Result<(), String> {
    let rows = config::read_prompt_rows(store);
    let Some(PromptRow::Read(read)) = rows.get(&slot) else {
        return Ok(());
    };
    let kept = Override {
        based_on: hash(shipped::template(slot).unwrap_or_default()),
        ..read.clone()
    };
    config::write_prompt(store, slot, &kept).map_err(|error| error.to_string())
}

/// Run a check of `edited` as `slot`'s template (R4): the rows read fresh
/// off `store` — read, never written — the plan, the engine on duty held
/// for the check through `handle` (counted busy until it ends, D51), and
/// the loop's own judgement. Run it on the background executor.
pub async fn check_template(
    handle: EngineHandle,
    store: SettingsStore,
    slot: Slot,
    edited: String,
    cancel: CancellationToken,
) -> CheckEnd {
    let rows = config::read_prompt_rows(&store);
    let pivot = config::read_pivot(&store).chosen;
    let trial = match plan_trial(slot, &edited, &config::overrides_of(&rows), pivot) {
        Ok(trial) => trial,
        Err(refusal) => return CheckEnd::Refused(refusal),
    };
    let engine = match cancel.run_until_cancelled(handle.for_job()).await {
        None | Some(Err(EngineError::Cancelled)) => return CheckEnd::Ran(TrialEnd::Cancelled),
        Some(Err(EngineError::Unavailable(why))) => {
            return CheckEnd::Ran(TrialEnd::Unavailable(why))
        }
        Some(Err(other)) => return CheckEnd::Failed(other.to_string()),
        Some(Ok(engine)) => engine,
    };
    let options = Options::for_executor(handle.pace().executor.unwrap_or(Executor::LocalCpu));
    let end = run_trial(&trial, &engine, &options, &cancel).await;
    // The length and the verdict's kind, never the words: a log is read
    // by whoever is debugging.
    if let TrialEnd::Done(report) = &end {
        tracing::info!(
            key = row::key(slot),
            steps = report.steps.len(),
            verdict = report
                .verdict
                .as_ref()
                .map_or_else(Rejection::kind, |_| "passed"),
            "a template was checked"
        );
    }
    end.into()
}

impl From<TrialEnd> for CheckEnd {
    fn from(end: TrialEnd) -> Self {
        CheckEnd::Ran(end)
    }
}

/// Adapt the `source` language's template into `target` with the model on
/// duty (R5, Г2) — only ever because a person pressed the button. The
/// answer is stored as `origin: machine` only when the rule and the
/// source's variables admit it; otherwise nothing is written.
pub async fn adapt_template(
    handle: EngineHandle,
    store: SettingsStore,
    source: Lang,
    target: Slot,
    ctx_len: Option<u32>,
    cancel: CancellationToken,
) -> Adapted {
    let rows = config::read_prompt_rows(&store);
    let overrides = config::overrides_of(&rows);
    let Some(source_slot) = target.with_lang(source) else {
        return Adapted::Ended(AdaptEnd::NotAsked(AdaptRefusal::NoSuchSlot));
    };
    let source_text = overrides
        .effective(source_slot)
        .unwrap_or_default()
        .to_owned();
    let engine = match cancel.run_until_cancelled(handle.for_job()).await {
        None | Some(Err(EngineError::Cancelled)) => return Adapted::Ended(AdaptEnd::Cancelled),
        Some(Err(EngineError::Unavailable(why))) => {
            return Adapted::Ended(AdaptEnd::Unavailable(why))
        }
        Some(Err(other)) => return Adapted::Failed(other.to_string()),
        Some(Ok(engine)) => engine,
    };
    let end = adapt_with(
        &engine,
        source_slot,
        &source_text,
        target,
        &overrides,
        ctx_len,
        &cancel,
    )
    .await;
    drop(engine);
    match end {
        AdaptEnd::Answered { row, admission, .. } => {
            let stored = if admission.admitted() {
                config::write_prompt(&store, target, &row)
                    .map(|()| true)
                    .map_err(|error| error.to_string())
            } else {
                Ok(false)
            };
            tracing::info!(
                key = row::key(target),
                from = source.as_str(),
                admitted = admission.admitted(),
                "a template was adapted by the model"
            );
            Adapted::Answered {
                admission,
                text: row.text,
                stored,
            }
        }
        other => Adapted::Ended(other),
    }
}

/// Whether "Adapt with the model" may write into `slot`: when it holds no
/// override, or one that is itself an adaptation. A template the person
/// wrote in its own language is theirs, and a button must not replace it.
pub fn may_adapt_into(row: Option<&PromptRow>) -> bool {
    match row {
        None | Some(PromptRow::Unread(_)) => true,
        Some(PromptRow::Read(read)) => read.adapted_from.is_some(),
    }
}

// ─── The view ───────────────────────────────────────────────────────

/// Something running in the background, cancellable, and what it came to.
enum Activity<T> {
    Idle,
    Running { cancel: CancellationToken },
    Done(T),
}

impl<T> Activity<T> {
    fn running(&self) -> bool {
        matches!(self, Activity::Running { .. })
    }
}

/// What the last Save, Reset or Keep said.
enum Said {
    Saved(Saved),
    Reset(Reset),
    Kept(Result<(), String>),
}

/// The pivot dropdown's rows: "by the document's language", then each
/// language with a shipped set.
fn pivot_choices() -> Vec<Choice<Option<Lang>>> {
    let mut choices = vec![Choice::new(
        None,
        t(Message::PromptsPivotByDocument),
        "document",
    )];
    for lang in LANGS {
        choices.push(Choice::new(Some(lang), lang_name(lang), lang.as_str()));
    }
    choices
}

/// The Prompts page: every slot, the editor of the one chosen, and what
/// it says about it. Built with the Settings window and gone with it.
pub struct PromptsPage {
    preferences: Entity<Preferences>,
    store: SettingsStore,
    /// Every slot's row, once read; `None` while the first read runs.
    rows: Option<BTreeMap<Slot, PromptRow>>,
    pivot: PivotRow,
    selected: Slot,
    editor: Entity<TextareaState>,
    /// What the editor was last filled with — what "Undo edits" puts back
    /// and what an edit is measured against.
    filled: String,
    /// The language the person says this template was adapted from, by
    /// hand; `None` for one written in its own language.
    claim: Option<Lang>,
    show_shipped: bool,
    /// The rule's verdict on the editor's text, as typed.
    problems: Vec<Problem>,
    said: Option<Said>,
    check: Activity<CheckEnd>,
    adapting: Activity<Adapted>,
    pivot_select: Entity<SelectState<Vec<Choice<Option<Lang>>>>>,
    _subscriptions: Vec<Subscription>,
}

impl PromptsPage {
    pub fn new(
        preferences: Entity<Preferences>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let store = preferences.read(cx).store();
        let selected = listed_slots()
            .into_iter()
            .find(|slot| {
                slot.lang() == Lang::En
                    && slot.tactic() == Tactic::Paraphrase
                    && slot.role() == Role::User
            })
            .unwrap_or_else(|| listed_slots()[0]);
        let editor = cx.new(|cx| TextareaState::new(window, cx).auto_grow(8, 28));
        let pivot_select =
            cx.new(|cx| SelectState::new(pivot_choices(), Some(IndexPath::new(0)), window, cx));

        let typed = cx.subscribe_in(&editor, window, |page, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                page.revalidate(cx);
                cx.notify();
            }
        });
        let chose_pivot = cx.subscribe_in(
            &pivot_select,
            window,
            |page, _, event: &SelectEvent<Vec<Choice<Option<Lang>>>>, _, cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                if let Some(pivot) = engine::from_value(&pivot_choices(), value) {
                    page.choose_pivot(pivot, cx);
                }
            },
        );
        let watched = engine_host::hosted(cx).map(|host| cx.observe(&host, |_, _, cx| cx.notify()));
        let preferences_moved = cx.observe(&preferences, |page, _, cx| {
            page.revalidate(cx);
            cx.notify();
        });
        let mut subscriptions = vec![typed, chose_pivot, preferences_moved];
        subscriptions.extend(watched);

        let page = Self {
            preferences,
            store,
            rows: None,
            pivot: PivotRow::default(),
            selected,
            editor,
            filled: String::new(),
            claim: None,
            show_shipped: false,
            problems: Vec::new(),
            said: None,
            check: Activity::Idle,
            adapting: Activity::Idle,
            pivot_select,
            _subscriptions: subscriptions,
        };
        page.reread(window, cx);
        page
    }

    /// Read every row again, off the foreground thread, and fill the
    /// editor when it has not been filled yet.
    fn reread(&self, window: &Window, cx: &Context<Self>) {
        let store = self.store.clone();
        let task = cx
            .background_executor()
            .spawn(async move { (config::read_prompt_rows(&store), config::read_pivot(&store)) });
        let window = window.window_handle();
        cx.spawn(async move |page, cx| {
            let (rows, pivot) = task.await;
            let _ = cx.update_window(window, |_, window, cx| {
                let _ = page.update(cx, |page, cx| page.read_back(rows, pivot, window, cx));
            });
        })
        .detach();
    }

    fn read_back(
        &mut self,
        rows: BTreeMap<Slot, PromptRow>,
        pivot: PivotRow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let first = self.rows.is_none();
        self.rows = Some(rows);
        let index = engine::row_of(&pivot_choices(), &pivot.chosen);
        self.pivot = pivot;
        self.pivot_select.update(cx, |select, cx| {
            select.set_selected_index(index.map(IndexPath::new), window, cx);
        });
        if first {
            self.fill(window, cx);
        } else {
            self.revalidate(cx);
        }
        cx.notify();
    }

    /// The overrides this build reads, as stored.
    fn overrides(&self) -> Overrides {
        self.rows
            .as_ref()
            .map(config::overrides_of)
            .unwrap_or_default()
    }

    fn row(&self) -> Option<&PromptRow> {
        self.rows.as_ref().and_then(|rows| rows.get(&self.selected))
    }

    /// The text in use for the chosen slot: the override's, else the
    /// shipped one — also for a row this build cannot read.
    fn in_use(&self) -> String {
        match self.row() {
            Some(PromptRow::Read(read)) => read.text.clone(),
            _ => shipped::template(self.selected)
                .unwrap_or_default()
                .to_owned(),
        }
    }

    /// Put the text in use into the editor.
    fn fill(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.in_use();
        self.claim = match self.row() {
            Some(PromptRow::Read(read)) => read.adapted_from.as_ref().map(|source| source.lang),
            _ => None,
        };
        self.filled = text.clone();
        self.editor
            .update(cx, |editor, cx| editor.set_value(text, window, cx));
        self.revalidate(cx);
    }

    fn edited(&self, cx: &App) -> String {
        self.editor.read(cx).value().to_string()
    }

    /// The window of the model on duty, when it is known.
    fn ctx_len(&self, cx: &App) -> Option<u32> {
        self.preferences
            .read(cx)
            .duty(ModelRole::Rewrite)
            .performer()
            .and_then(|performer| performer.info().ctx_len)
    }

    /// The rule's verdict on the editor's text, as it would be saved.
    fn revalidate(&mut self, cx: &App) {
        let text = self.edited(cx);
        let overrides = self.overrides();
        let based_on = match self.row() {
            Some(PromptRow::Read(read)) => read.based_on.clone(),
            _ => hash(shipped::template(self.selected).unwrap_or_default()),
        };
        let candidate = Override {
            text,
            based_on,
            adapted_from: None,
            origin: Origin::Hand,
        };
        self.problems = admit(self.selected, &candidate, &overrides, self.ctx_len(cx)).problems;
    }

    fn select(&mut self, slot: Slot, window: &mut Window, cx: &mut Context<Self>) {
        if slot == self.selected {
            return;
        }
        self.selected = slot;
        self.said = None;
        self.show_shipped = false;
        if !self.check.running() {
            self.check = Activity::Idle;
        }
        if !self.adapting.running() {
            self.adapting = Activity::Idle;
        }
        self.fill(window, cx);
        cx.notify();
    }

    /// Run a write on the background executor, then read the rows back.
    fn write(
        &self,
        window: &Window,
        cx: &Context<Self>,
        work: impl FnOnce(&Store) -> Said + Send + 'static,
        refill: bool,
    ) {
        let store = self.store.clone();
        let task = cx.background_executor().spawn(async move {
            let said = work(&store);
            (
                said,
                config::read_prompt_rows(&store),
                config::read_pivot(&store),
            )
        });
        let window = window.window_handle();
        cx.spawn(async move |page, cx| {
            let (said, rows, pivot) = task.await;
            let _ = cx.update_window(window, |_, window, cx| {
                let _ = page.update(cx, |page, cx| {
                    let refill =
                        refill && matches!(said, Said::Reset(Reset::Done) | Said::Kept(Ok(())));
                    page.said = Some(said);
                    page.read_back(rows, pivot, window, cx);
                    if refill {
                        page.fill(window, cx);
                    } else {
                        page.filled = page.in_use();
                    }
                    cx.notify();
                });
            });
        })
        .detach();
    }

    fn save(&self, window: &Window, cx: &Context<Self>) {
        let slot = self.selected;
        let text = self.edited(cx);
        let claim = self.claim;
        let ctx_len = self.ctx_len(cx);
        self.write(
            window,
            cx,
            move |store| Said::Saved(save(store, slot, &text, claim, ctx_len)),
            false,
        );
    }

    fn reset(&self, window: &Window, cx: &Context<Self>) {
        let slot = self.selected;
        self.write(
            window,
            cx,
            move |store| Said::Reset(reset(store, slot)),
            true,
        );
    }

    fn keep(&self, window: &Window, cx: &Context<Self>) {
        let slot = self.selected;
        self.write(
            window,
            cx,
            move |store| Said::Kept(keep_mine(store, slot)),
            true,
        );
    }

    fn undo_edits(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.filled.clone();
        self.editor
            .update(cx, |editor, cx| editor.set_value(text, window, cx));
        self.revalidate(cx);
        cx.notify();
    }

    fn choose_pivot(&mut self, pivot: Option<Lang>, cx: &mut Context<Self>) {
        if self.pivot.chosen == pivot && self.pivot.unread.is_none() {
            return;
        }
        // Choosing "by the document" over a row this build cannot read
        // deletes it: the person answered the question the row asked.
        self.pivot = PivotRow {
            chosen: pivot,
            unread: None,
        };
        let store = self.store.clone();
        cx.background_executor()
            .spawn(async move {
                if let Err(error) = config::write_pivot(&store, pivot) {
                    tracing::warn!(%error, "could not persist the pivot");
                }
            })
            .detach();
        cx.notify();
    }

    fn run_check(&mut self, cx: &mut Context<Self>) {
        if self.check.running() {
            return;
        }
        let cancel = CancellationToken::new();
        self.check = Activity::Running {
            cancel: cancel.clone(),
        };
        let handle = self.preferences.read(cx).rewriter();
        let store = self.store.clone();
        let slot = self.selected;
        let edited = self.edited(cx);
        let task = cx
            .background_executor()
            .spawn(check_template(handle, store, slot, edited, cancel));
        cx.spawn(async move |page, cx| {
            let end = task.await;
            let _ = page.update(cx, |page, cx| {
                page.check = Activity::Done(end);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn run_adaptation(&mut self, source: Lang, window: &Window, cx: &mut Context<Self>) {
        if self.adapting.running() {
            return;
        }
        let cancel = CancellationToken::new();
        self.adapting = Activity::Running {
            cancel: cancel.clone(),
        };
        let handle = self.preferences.read(cx).rewriter();
        let store = self.store.clone();
        let target = self.selected;
        let ctx_len = self.ctx_len(cx);
        let task = cx.background_executor().spawn(async move {
            let adapted =
                adapt_template(handle, store.clone(), source, target, ctx_len, cancel).await;
            (
                adapted,
                config::read_prompt_rows(&store),
                config::read_pivot(&store),
            )
        });
        let window = window.window_handle();
        cx.spawn(async move |page, cx| {
            let (adapted, rows, pivot) = task.await;
            let _ = cx.update_window(window, |_, window, cx| {
                let _ = page.update(cx, |page, cx| {
                    let stored = matches!(
                        adapted,
                        Adapted::Answered {
                            stored: Ok(true),
                            ..
                        }
                    );
                    page.adapting = Activity::Done(adapted);
                    page.read_back(rows, pivot, window, cx);
                    if stored && page.selected == target {
                        page.fill(window, cx);
                    }
                    cx.notify();
                });
            });
        })
        .detach();
        cx.notify();
    }

    /// Put the dropdown back into the language on screen.
    pub fn retranslate(&self, window: &mut Window, cx: &mut Context<Self>) {
        let choices = pivot_choices();
        let index = engine::row_of(&choices, &self.pivot.chosen).map(IndexPath::new);
        self.pivot_select.update(cx, |select, cx| {
            select.set_items(choices, window, cx);
            select.set_selected_index(index, window, cx);
        });
        cx.notify();
    }

    /// The pivot row's control, for the Settings window's row.
    pub fn pivot_control(&self) -> impl IntoElement {
        Select::new(&self.pivot_select)
            .small()
            .menu_width(px(240.0))
    }

    // ── Drawing ──

    fn slot_list(&self, cx: &Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let overrides = self.overrides();
        v_flex()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .child(SharedString::from(t(Message::PromptsSlotsTitle))),
            )
            .children(LANGS.into_iter().map(|lang| {
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .child(SharedString::from(lang_name(lang))),
                    )
                    .children(Tactic::ALL.into_iter().filter_map(|tactic| {
                        let slots: Vec<Slot> = listed_slots()
                            .into_iter()
                            .filter(|slot| slot.lang() == lang && slot.tactic() == tactic)
                            .collect();
                        (!slots.is_empty()).then(|| {
                            h_flex()
                                .gap_1()
                                .flex_wrap()
                                .items_center()
                                .child(
                                    div()
                                        .w(px(110.0))
                                        .text_xs()
                                        .font_family("monospace")
                                        .child(SharedString::from(tactic.as_str())),
                                )
                                .children(
                                    slots
                                        .into_iter()
                                        .map(|slot| self.slot_button(slot, &overrides, cx)),
                                )
                                .children(tactic_note(tactic).map(|note| {
                                    div()
                                        .text_xs()
                                        .text_color(muted)
                                        .child(SharedString::from(t(note)))
                                }))
                        })
                    }))
            }))
    }

    fn slot_button(
        &self,
        slot: Slot,
        overrides: &Overrides,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let row = self.rows.as_ref().and_then(|rows| rows.get(&slot));
        let mark = match row {
            None => None,
            Some(PromptRow::Unread(_)) => Some(Message::PromptsTagUnreadable),
            Some(PromptRow::Read(read)) => Some(if staleness(slot, read, overrides).any() {
                Message::PromptsTagStale
            } else {
                match read.origin {
                    Origin::Hand => Message::PromptsTagHand,
                    Origin::Machine => Message::PromptsTagMachine,
                    Origin::MachineReviewed => Message::PromptsTagMachineReviewed,
                }
            }),
        };
        let mut label = if slot.tactic().steps() > 1 {
            format!("{} · {}", slot.step(), t(turn_message(slot.role())))
        } else {
            t(turn_message(slot.role()))
        };
        if let Some(mark) = mark {
            label.push_str(" — ");
            label.push_str(&t(mark));
        }
        let id = SharedString::from(row::key(slot));
        let button = Button::new(id)
            .xsmall()
            .label(SharedString::from(label))
            .on_click(cx.listener(move |page, _, window, cx| page.select(slot, window, cx)));
        if slot == self.selected {
            button.primary()
        } else {
            button.ghost()
        }
    }

    fn lines(shown: Vec<Shown>, cx: &App) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.muted;
        v_flex().gap_1().children(shown.into_iter().map(|item| {
            match item {
                Shown::Said(tone, line) => div()
                    .text_color(tone.colour(cx))
                    .child(SharedString::from(say(&line)))
                    .into_any_element(),
                Shown::Quoted(text) => div()
                    .p_2()
                    .rounded_md()
                    .bg(muted)
                    .font_family("monospace")
                    .child(SharedString::from(text))
                    .into_any_element(),
            }
        }))
    }

    fn editor_block(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let border = theme.border;
        let slot = self.selected;
        let overrides = self.overrides();
        let row = self.row().cloned();
        let duty = self.preferences.read(cx).duty(ModelRole::Rewrite);
        let edited = self.edited(cx);
        let dirty = edited != self.filled;
        let has_errors = self
            .problems
            .iter()
            .any(|problem| problem.severity() == Severity::Error);
        let nobody = nobody_on_duty(&duty);
        let checking = self.check.running();
        let adapting = self.adapting.running();

        let mut block = v_flex()
            .gap_2()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(border)
            .text_xs()
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .child(SharedString::from(say(&slot_heading(slot)))),
                    )
                    .child(
                        Tag::secondary()
                            .small()
                            .child(SharedString::from(row::key(slot))),
                    ),
            )
            .child(div().child(SharedString::from(say(&origin_line(row.as_ref())))));

        if let Some(coverage) = coverage_line(slot, &overrides) {
            block = block.child(
                div()
                    .text_color(muted)
                    .child(SharedString::from(say(&coverage))),
            );
        }

        // Г6 and R5: drift, said and never merged.
        if let Some(PromptRow::Read(read)) = &row {
            let stale = staleness(slot, read, &overrides);
            let shipped_text = shipped::template(slot).unwrap_or_default();
            if stale.shipped_changed {
                block = block
                    .child(
                        div()
                            .text_color(Tone::Warn.colour(cx))
                            .child(SharedString::from(t(Message::PromptsStaleShipped))),
                    )
                    .child(Self::drift(
                        &compared(shipped_text, &read.text),
                        Message::PromptsDiffLegendShipped,
                        cx,
                    ))
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("prompts-keep-mine")
                                    .small()
                                    .outline()
                                    .label(SharedString::from(t(Message::PromptsKeepMine)))
                                    .on_click(
                                        cx.listener(|page, _, window, cx| page.keep(window, cx)),
                                    ),
                            )
                            .child(
                                Button::new("prompts-reset-stale")
                                    .small()
                                    .outline()
                                    .label(SharedString::from(t(Message::PromptsReset)))
                                    .on_click(
                                        cx.listener(|page, _, window, cx| page.reset(window, cx)),
                                    ),
                            ),
                    );
            }
            if let (true, Some(source)) = (stale.source_changed, &read.adapted_from) {
                block = block.child(
                    div().text_color(Tone::Warn.colour(cx)).child(SharedString::from(t_args(
                        Message::PromptsStaleSource,
                        &args!("source" => lang_name(source.lang), "target" => lang_name(slot.lang())),
                    ))),
                );
                if let Some(source_slot) = slot.with_lang(source.lang) {
                    let now = overrides.effective(source_slot).unwrap_or_default();
                    let shipped_source = shipped::template(source_slot).unwrap_or_default();
                    if hash(shipped_source) == source.hash {
                        block = block
                            .child(div().text_color(muted).child(SharedString::from(t(
                                Message::PromptsStaleSourceWasShipped,
                            ))))
                            .child(Self::drift(
                                &compared(shipped_source, now),
                                Message::PromptsDiffLegendBeforeAfter,
                                cx,
                            ));
                    } else {
                        block =
                            block
                                .child(div().text_color(muted).child(SharedString::from(t(
                                    Message::PromptsStaleSourceUnknown,
                                ))))
                                .child(Self::lines(vec![Shown::Quoted(excerpt(now))], cx));
                    }
                }
            }
        }

        block = block.child(Textarea::new(&self.editor));
        if dirty {
            block = block.child(
                div()
                    .text_color(muted)
                    .child(SharedString::from(t(Message::PromptsUnsaved))),
            );
        }

        // Every problem the rule finds, with its rule id and its place.
        if !self.problems.is_empty() {
            block = block.child(
                v_flex()
                    .gap_1()
                    .children(self.problems.iter().map(|problem| {
                        let (severity, rule, sentence) = problem_shown(problem, &edited);
                        let (tone, word) = match severity {
                            Severity::Error => (Tone::Bad, Message::PromptsProblemError),
                            Severity::Warning => (Tone::Warn, Message::PromptsProblemWarning),
                        };
                        h_flex()
                            .gap_2()
                            .items_start()
                            .child(
                                div()
                                    .text_color(tone.colour(cx))
                                    .child(SharedString::from(t(word))),
                            )
                            .child(Tag::secondary().small().child(SharedString::from(rule)))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .child(SharedString::from(sentence)),
                            )
                    })),
            );
        }

        // Adapted by hand from: a claim recorded with the save.
        let siblings: Vec<Lang> = LANGS
            .into_iter()
            .filter(|lang| *lang != slot.lang() && slot.with_lang(*lang).is_some())
            .collect();
        if !siblings.is_empty() {
            let mut claim_row = h_flex()
                .gap_1()
                .flex_wrap()
                .items_center()
                .child(
                    div()
                        .text_color(muted)
                        .child(SharedString::from(t(Message::PromptsAdaptedFromLabel))),
                )
                .child({
                    let button = Button::new("prompts-claim-own")
                        .xsmall()
                        .label(SharedString::from(t(Message::PromptsAdaptedFromOwn)))
                        .on_click(cx.listener(|page, _, _, cx| {
                            page.claim = None;
                            cx.notify();
                        }));
                    if self.claim.is_none() {
                        button.primary()
                    } else {
                        button.ghost()
                    }
                });
            for lang in siblings.iter().copied() {
                let button = Button::new(SharedString::from(format!(
                    "prompts-claim-{}",
                    lang.as_str()
                )))
                .xsmall()
                .label(SharedString::from(lang_name(lang)))
                .on_click(cx.listener(move |page, _, _, cx| {
                    page.claim = Some(lang);
                    cx.notify();
                }));
                claim_row = claim_row.child(if self.claim == Some(lang) {
                    button.primary()
                } else {
                    button.ghost()
                });
            }
            block = block.child(claim_row);
        }

        let overridden = row.is_some();
        block = block.child(
            h_flex()
                .gap_2()
                .flex_wrap()
                .child(
                    Button::new("prompts-save")
                        .small()
                        .primary()
                        .label(SharedString::from(t(Message::PromptsSave)))
                        .on_click(cx.listener(|page, _, window, cx| page.save(window, cx))),
                )
                .child(
                    Button::new("prompts-undo")
                        .small()
                        .outline()
                        .label(SharedString::from(t(Message::PromptsUndoEdits)))
                        .disabled(!dirty)
                        .on_click(cx.listener(|page, _, window, cx| page.undo_edits(window, cx))),
                )
                .child(
                    Button::new("prompts-reset")
                        .small()
                        .outline()
                        .label(SharedString::from(t(Message::PromptsReset)))
                        .disabled(!overridden)
                        .on_click(cx.listener(|page, _, window, cx| page.reset(window, cx))),
                ),
        );

        if let Some(said) = &self.said {
            block = block.child(Self::lines(said_lines(said), cx));
        }

        // R4: Check template.
        let mut asking = v_flex().gap_1().pt_2().border_t_1().border_color(border);
        for line in before_asking(slot, &duty) {
            asking = asking.child(
                div()
                    .text_color(muted)
                    .child(SharedString::from(say(&line))),
            );
        }
        if let Some(why) = &nobody {
            asking = asking.child(
                div()
                    .text_color(muted)
                    .child(SharedString::from(why.clone())),
            );
        }
        if has_errors {
            asking = asking.child(
                div()
                    .text_color(muted)
                    .child(SharedString::from(t(Message::PromptsCheckErrors))),
            );
        }
        let can_check =
            nobody.is_none() && !has_errors && !checking && slot.tactic() != Tactic::Code;
        let mut buttons = h_flex().gap_2().flex_wrap().child(
            Button::new("prompts-check")
                .small()
                .outline()
                .label(SharedString::from(t(Message::PromptsCheck)))
                .disabled(!can_check)
                .on_click(cx.listener(|page, _, _, cx| page.run_check(cx))),
        );
        if let Activity::Running { cancel } = &self.check {
            let cancel = cancel.clone();
            buttons = buttons.child(
                Button::new("prompts-check-cancel")
                    .small()
                    .ghost()
                    .label(SharedString::from(t(Message::PromptsStop)))
                    .on_click(move |_, _, _| cancel.cancel()),
            );
        }
        // R5: adapt from another language with the model.
        let may_adapt = may_adapt_into(row.as_ref());
        for lang in siblings.iter().copied() {
            buttons = buttons.child(
                Button::new(SharedString::from(format!(
                    "prompts-adapt-{}",
                    lang.as_str()
                )))
                .small()
                .outline()
                .label(SharedString::from(t_args(
                    Message::PromptsAdaptFrom,
                    &args!("language" => lang_name(lang)),
                )))
                .tooltip(SharedString::from(if may_adapt {
                    t_args(
                        Message::PromptsAdaptNote,
                        &args!("source" => lang_name(lang), "target" => lang_name(slot.lang())),
                    )
                } else {
                    t(Message::PromptsAdaptOwnTemplate)
                }))
                .disabled(nobody.is_some() || adapting || !may_adapt)
                .on_click(cx.listener(move |page, _, window, cx| {
                    page.run_adaptation(lang, window, cx);
                })),
            );
        }
        if let Activity::Running { cancel } = &self.adapting {
            let cancel = cancel.clone();
            buttons = buttons.child(
                Button::new("prompts-adapt-cancel")
                    .small()
                    .ghost()
                    .label(SharedString::from(t(Message::PromptsStop)))
                    .on_click(move |_, _, _| cancel.cancel()),
            );
        }
        if !siblings.is_empty() && !may_adapt {
            asking = asking.child(
                div()
                    .text_color(muted)
                    .child(SharedString::from(t(Message::PromptsAdaptOwnTemplate))),
            );
        }
        asking = asking.child(buttons);

        match &self.check {
            Activity::Idle => {}
            Activity::Running { .. } => {
                asking = asking.child(div().child(SharedString::from(t(Message::PromptsChecking))));
                // F1: while the model loads, how far.
                if let Some(progress) =
                    engine_host::hosted(cx).and_then(|host| host.read(cx).load_progress())
                {
                    asking = asking
                        .child(div().text_color(muted).child(SharedString::from(t_args(
                            Message::PromptsCheckLoading,
                            &args!("percent" => engine_host::percent(progress).to_string()),
                        ))))
                        .child(
                            gpui_component::progress::Progress::new("prompts-load")
                                .small()
                                .value(engine_host::percent(progress) as f32),
                        );
                }
            }
            Activity::Done(end) => asking = asking.child(Self::lines(check_lines(end), cx)),
        }
        match &self.adapting {
            Activity::Idle => {}
            Activity::Running { .. } => {
                asking = asking.child(div().child(SharedString::from(t(Message::PromptsAdapting))));
            }
            Activity::Done(adapted) => {
                asking = asking.child(Self::lines(adapt_lines(adapted), cx));
                if let Adapted::Answered {
                    admission, text, ..
                } = adapted
                {
                    for problem in &admission.problems {
                        let (_, rule, sentence) = problem_shown(problem, text);
                        asking = asking.child(
                            h_flex()
                                .gap_2()
                                .child(Tag::secondary().small().child(SharedString::from(rule)))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w(px(0.0))
                                        .child(SharedString::from(sentence)),
                                ),
                        );
                    }
                }
            }
        }
        block = block.child(asking);

        // The shipped text, read-only and folded; the fragments beside it.
        let show = self.show_shipped;
        block = block.child(
            Button::new("prompts-show-shipped")
                .xsmall()
                .ghost()
                .label(SharedString::from(t(if show {
                    Message::PromptsShippedHide
                } else {
                    Message::PromptsShippedShow
                })))
                .on_click(cx.listener(|page, _, _, cx| {
                    page.show_shipped = !page.show_shipped;
                    cx.notify();
                })),
        );
        if show {
            block = block.child(Self::lines(
                vec![Shown::Quoted(
                    shipped::template(slot).unwrap_or_default().to_owned(),
                )],
                cx,
            ));
            block = block.child(
                div()
                    .text_color(muted)
                    .child(SharedString::from(t(Message::PromptsFragmentsTitle))),
            );
            for (message, text) in fragments_of(slot) {
                block = block
                    .child(
                        div()
                            .text_color(muted)
                            .child(SharedString::from(t(message))),
                    )
                    .child(Self::lines(vec![Shown::Quoted(text.to_owned())], cx));
            }
        }
        block
    }

    fn drift(lines: &[(char, String)], legend: Message, cx: &App) -> impl IntoElement {
        let theme = cx.theme();
        v_flex()
            .gap_0p5()
            .p_2()
            .rounded_md()
            .bg(theme.muted)
            .font_family("monospace")
            .child(
                div()
                    .text_color(theme.muted_foreground)
                    .child(SharedString::from(t(legend))),
            )
            .children(
                lines
                    .iter()
                    .map(|(sign, line)| div().child(SharedString::from(format!("{sign} {line}")))),
            )
    }
}

/// What a Save, a Reset or a Keep said, as lines.
fn said_lines(said: &Said) -> Vec<Shown> {
    match said {
        Said::Saved(Saved::Stored {
            admission,
            reviewed,
        }) => {
            let warned = !admission.problems.is_empty();
            vec![Shown::Said(
                Tone::Good,
                plain(match (reviewed, warned) {
                    (true, _) => Message::PromptsSavedReviewed,
                    (false, true) => Message::PromptsSavedWarnings,
                    (false, false) => Message::PromptsSaved,
                }),
            )]
        }
        Said::Saved(Saved::Unchanged) => {
            vec![Shown::Said(Tone::Quiet, plain(Message::PromptsUnchanged))]
        }
        Said::Saved(Saved::Refused(_)) => {
            vec![Shown::Said(Tone::Bad, plain(Message::PromptsRefused))]
        }
        Said::Saved(Saved::Failed(reason))
        | Said::Reset(Reset::Failed(reason))
        | Said::Kept(Err(reason)) => {
            vec![Shown::Said(
                Tone::Bad,
                (
                    Message::PromptsWriteFailed,
                    args!("reason" => reason.clone()),
                ),
            )]
        }
        Said::Reset(Reset::Done) => vec![Shown::Said(Tone::Good, plain(Message::PromptsResetDone))],
        Said::Reset(Reset::Refused { other, rule }) => vec![Shown::Said(
            Tone::Warn,
            (
                Message::PromptsResetRefused,
                args!(
                    "turn" => t(turn_message(other.role())),
                    "variable" => spelled(Variable::Protected),
                    "rule" => (*rule).to_owned()
                ),
            ),
        )],
        Said::Kept(Ok(())) => vec![Shown::Said(Tone::Good, plain(Message::PromptsKept))],
    }
}

impl Render for PromptsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let mut page = v_flex().gap_4();
        if let Some(unread) = &self.pivot.unread {
            page = page.child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child(SharedString::from(t_args(
                        Message::PromptsPivotUnread,
                        &args!("value" => unread.clone()),
                    ))),
            );
        }
        if self.rows.is_none() {
            return page.child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child(SharedString::from(t(Message::PromptsReading))),
            );
        }
        page.child(self.slot_list(cx)).child(self.editor_block(cx))
    }
}

/// The variables table (R6), for the page's head.
pub fn variables_table(cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    v_flex()
        .gap_1()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .text_xs()
        .child(
            div()
                .font_semibold()
                .child(SharedString::from(t(Message::PromptsVariablesTitle))),
        )
        .children(variables().into_iter().map(|(variable, message)| {
            h_flex()
                .gap_3()
                .items_start()
                .child(
                    div()
                        .w(px(120.0))
                        .flex_none()
                        .font_family("monospace")
                        .child(SharedString::from(spelled(variable))),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .child(SharedString::from(t(message))),
                )
        }))
        .child(
            div()
                .text_color(theme.muted_foreground)
                .child(SharedString::from(t(Message::PromptsVarNoNames))),
        )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::Arc;

    use serde_json::{json, Map, Value};
    use wipemark_core::Vendor;
    use wipemark_engine::fake::FakeEngine;
    use wipemark_engine::CancellationToken;
    use wipemark_i18n::{LanguageIdentifier, Localizer, Message, Rendering};
    use wipemark_pipeline::block_on;
    use wipemark_pipeline::lang::Lang;
    use wipemark_pipeline::prompt::row::{
        self, lay_over, lay_over_within, AdaptedFrom, Origin, Override,
    };
    use wipemark_pipeline::prompt::validate::{BraceSide, Problem, Script};
    use wipemark_pipeline::prompt::{
        shipped, staleness, Intensity, Marker, Overrides, Role, Slot, Tactic, TrialEnd, Variable,
    };
    use wipemark_store::Store;

    use super::{
        adapt_template, before_asking, check_lines, check_template, coverage_line, keep_mine,
        listed_slots, nobody_on_duty, problem_line, reset, save, Adapted, CheckEnd, Line, Reset,
        Saved, Shown,
    };
    use crate::config::{self, PromptRow};
    use crate::duty::{self, Duty, Performer, Remote, Vacancy};
    use crate::engine_host::{EngineHandle, Pace};

    fn slot(lang: Lang, tactic: Tactic, step: u8, role: Role) -> Slot {
        Slot::new(lang, tactic, step, role).expect("a slot")
    }

    fn store() -> Arc<Store> {
        Arc::new(Store::in_memory().expect("a scratch database"))
    }

    fn rows_of(store: &Store) -> std::collections::BTreeMap<String, Value> {
        store.settings().all().expect("the rows")
    }

    /// R1: the page lists every slot the pipeline accepts — every
    /// language, tactic, step and turn, `structural` and `code` among them
    /// (Г4) — and nothing else. Red with a slot dropped from the list.
    #[test]
    fn every_template_slot_has_a_row() {
        let listed: BTreeSet<Slot> = listed_slots().into_iter().collect();
        let accepted: BTreeSet<Slot> = Slot::all().into_iter().collect();
        assert_eq!(
            listed, accepted,
            "a template only a database client reaches"
        );
        assert_eq!(
            listed_slots().len(),
            Slot::all().len(),
            "a slot listed twice"
        );
    }

    fn languages() -> Vec<LanguageIdentifier> {
        ["en-US", "ru", "de"]
            .iter()
            .map(|id| id.parse().expect("a language id"))
            .collect()
    }

    /// One of every `Problem` — a new variant fails to compile in
    /// `problem_line` until it has words, and is added here to be held to
    /// all three catalogues.
    fn every_problem() -> Vec<Problem> {
        vec![
            Problem::UnknownVariable {
                name: "TEKST".to_owned(),
                span: 0..7,
                suggestion: Some(Variable::Text),
            },
            Problem::UnknownVariable {
                name: "FOO".to_owned(),
                span: 0..5,
                suggestion: None,
            },
            Problem::UnclosedBrace {
                at: 0,
                side: BraceSide::Open,
            },
            Problem::UnclosedBrace {
                at: 0,
                side: BraceSide::Close,
            },
            Problem::MissingVariable {
                variable: Variable::Text,
            },
            Problem::MissingVariable {
                variable: Variable::Protected,
            },
            Problem::MissingVariable {
                variable: Variable::Intensity,
            },
            Problem::RepeatedVariable {
                variable: Variable::Text,
                spans: vec![0..6, 7..13],
            },
            Problem::MisplacedVariable {
                variable: Variable::Text,
                span: 0..6,
                role: Role::System,
            },
            Problem::HandWrittenMarker {
                marker: Marker::BeginText,
                span: 0..16,
            },
            Problem::ReservedBracket { at: 0 },
            Problem::Empty,
            Problem::TooLong {
                estimated_tokens: 900,
                limit: 819,
            },
            Problem::ScriptMismatch {
                expected: Script::Cyrillic,
                latin: 1.0,
                cyrillic: 0.0,
            },
            Problem::NothingButText,
            Problem::NoIntensity {
                intensity: Intensity::Strong,
            },
            Problem::Stale {
                based_on: "0".repeat(16),
                shipped: "1".repeat(16),
            },
            Problem::VariablesDiffer {
                missing: vec![Variable::Text],
                extra: vec![],
            },
        ]
    }

    fn renders_everywhere(line: &Line, what: &str) {
        for language in languages() {
            let localizer =
                Localizer::for_languages(std::slice::from_ref(&language), Rendering::PlainText);
            assert!(
                localizer.defines(line.0),
                "{language} has no words for {what}: {}",
                line.0.id()
            );
            let text = localizer.format_args(line.0, &line.1);
            assert!(
                !text.is_empty() && text != line.0.id() && !text.contains("{$"),
                "{language}: {what} renders as {text:?}"
            );
        }
    }

    /// R3: every problem the rule can find has a sentence in en, ru and
    /// de, and the sentences of different rules differ.
    #[test]
    fn every_problem_has_words_in_every_language() {
        let mut seen = BTreeSet::new();
        for problem in every_problem() {
            let line = problem_line(&problem);
            renders_everywhere(&line, problem.rule());
            seen.insert(line.0.id());
        }
        assert!(seen.len() >= 14, "two rules share one sentence: {seen:?}");
    }

    /// One row of the table both surfaces are held to: a slot, the text,
    /// what is already stored, and the window.
    struct Case {
        what: &'static str,
        slot: Slot,
        text: String,
        stored: Vec<(Slot, &'static str)>,
        ctx_len: Option<u32>,
    }

    fn cases() -> Vec<Case> {
        let user = slot(Lang::En, Tactic::Paraphrase, 1, Role::User);
        let system = user.other_role();
        let ru_user = slot(Lang::Ru, Tactic::Paraphrase, 1, Role::User);
        let case = |what, slot, text: &str| Case {
            what,
            slot,
            text: text.to_owned(),
            stored: Vec::new(),
            ctx_len: None,
        };
        vec![
            case("valid", user, "Say it again in other words.\n{TEXT}"),
            case("unknown-variable", user, "Say it again.\n{TEKST}\n{TEXT}"),
            case("unclosed-brace", user, "Say it {again.\n{TEXT}"),
            case("missing-variable (text)", user, "Say it again."),
            case("repeated-variable", user, "{TEXT}\n{TEXT}"),
            case(
                "misplaced-variable",
                system,
                "Keep every fact. {PROTECTED}\n{TEXT}",
            ),
            case(
                "hand-written-marker",
                user,
                "Say it again.\n[[[BEGIN TEXT]]]\n{TEXT}",
            ),
            case("reserved-bracket", user, "Keep ⟦1⟧.\n{TEXT}"),
            case("empty", user, "  \n "),
            Case {
                ctx_len: Some(1000),
                ..case(
                    "too-long",
                    user,
                    &format!("{}\n{{TEXT}}", "word ".repeat(200)),
                )
            },
            Case {
                ctx_len: Some(100_000),
                ..case(
                    "long, but within the window",
                    user,
                    &format!("{}\n{{TEXT}}", "word ".repeat(200)),
                )
            },
            case(
                "script-mismatch (warning)",
                ru_user,
                "Rewrite the text.\n{TEXT}",
            ),
            case("nothing-but-text (warning)", user, "{TEXT}"),
            Case {
                stored: vec![(system, "Keep every fact.")],
                ..case(
                    "missing-variable (protected): the other turn as stored has none",
                    user,
                    "Say it again.\n{TEXT}",
                )
            },
            Case {
                stored: vec![(system, "Keep every fact.")],
                ..case(
                    "only this turn supplies {PROTECTED}",
                    user,
                    "Say it again. {PROTECTED}\n{TEXT}",
                )
            },
            Case {
                stored: vec![(user, "Say it again. {PROTECTED}\n{TEXT}")],
                ..case(
                    "only the other turn supplies {PROTECTED}",
                    system,
                    "Keep every fact.",
                )
            },
        ]
    }

    /// R3: the page's Save and `lay_over` — what the CLI's `--prompts` and
    /// the MCP tool's `templates` ask — accept exactly the same templates,
    /// row for row, with the window and without. Two rules that drift are
    /// a page that saves what the CLI refuses.
    #[test]
    fn the_page_and_lay_over_accept_the_same_templates() {
        let mut stored_somewhere = false;
        let mut refused_somewhere = false;
        for case in cases() {
            let store = store();
            for (slot, text) in &case.stored {
                config::write_prompt(&store, *slot, &Override::by_hand(*slot, *text))
                    .expect("a stored row");
            }
            let mut saved = config::overrides_of(&config::read_prompt_rows(&store));
            let laid: Map<String, Value> = json!({ row::key(case.slot): case.text })
                .as_object()
                .expect("an object")
                .clone();
            let cli = match case.ctx_len {
                None => lay_over(&mut saved, &laid),
                Some(window) => lay_over_within(&mut saved, &laid, Some(window)),
            };
            let page = save(&store, case.slot, &case.text, None, case.ctx_len);
            let page_stored = matches!(page, Saved::Stored { .. });
            assert_eq!(
                page_stored,
                cli.is_ok(),
                "{}: the page {} and lay_over said {cli:?}",
                case.what,
                if page_stored { "saved" } else { "refused" }
            );
            if let (Saved::Refused(admission), Err(row::Laid::Breaks { rule, .. })) = (&page, &cli)
            {
                assert_eq!(
                    admission.first_error().map(Problem::rule),
                    Some(*rule),
                    "{}: one rule, one id",
                    case.what
                );
            }
            stored_somewhere |= page_stored;
            refused_somewhere |= !page_stored;
        }
        assert!(
            stored_somewhere && refused_somewhere,
            "the table proves nothing"
        );
    }

    /// R3: a refused Save leaves the database byte for byte — the row it
    /// would have replaced included — and a warned one is stored.
    #[test]
    fn a_refused_save_leaves_the_database_as_it_was() {
        let store = store();
        let user = slot(Lang::Ru, Tactic::Paraphrase, 1, Role::User);
        let saved = save(&store, user, "Перепиши своими словами.\n{TEXT}", None, None);
        assert!(matches!(saved, Saved::Stored { .. }), "{saved:?}");
        store
            .settings()
            .set(config::REWRITE_PIVOT_KEY, "de")
            .expect("pivot");
        let before = rows_of(&store);
        let refused = save(&store, user, "Перепиши {ТЕКСТ}.", None, None);
        assert!(matches!(refused, Saved::Refused(_)), "{refused:?}");
        assert_eq!(rows_of(&store), before, "a refused save wrote something");

        let warned = save(&store, user, "{TEXT}", None, None);
        let Saved::Stored { admission, .. } = warned else {
            panic!("a warning does not refuse: {warned:?}");
        };
        assert_eq!(
            admission
                .problems
                .iter()
                .map(Problem::rule)
                .collect::<Vec<_>>(),
            ["nothing-but-text"]
        );
    }

    /// R1: Save of the shipped text with nothing stored writes nothing;
    /// Reset deletes the row and never writes the shipped text into one.
    #[test]
    fn reset_deletes_the_row_and_never_writes_the_shipped_text() {
        let store = store();
        let user = slot(Lang::De, Tactic::Humanize, 1, Role::User);
        let shipped = shipped::template(user).expect("shipped");
        assert_eq!(save(&store, user, shipped, None, None), Saved::Unchanged);
        assert!(rows_of(&store).is_empty());

        assert!(matches!(
            save(&store, user, "Schreib es neu.\n{TEXT}", None, None),
            Saved::Stored { .. }
        ));
        assert_eq!(reset(&store, user), Reset::Done);
        assert!(rows_of(&store).is_empty(), "{:?}", rows_of(&store));

        // A row this build cannot read is reset the same way — and is
        // left alone until then.
        store
            .settings()
            .set(&row::key(user), &json!({"text": 1}))
            .expect("a row from elsewhere");
        assert!(matches!(
            config::read_prompt_rows(&store).get(&user),
            Some(PromptRow::Unread(Some(_)))
        ));
        assert!(rows_of(&store).contains_key(&row::key(user)), "left");
        assert_eq!(reset(&store, user), Reset::Done);
        assert!(rows_of(&store).is_empty());
    }

    /// A reset that would leave the other turn of the step breaking the
    /// placeholder rule is refused, and writes nothing: a step that does
    /// not render is a job that fails.
    #[test]
    fn a_reset_that_would_break_the_step_is_refused() {
        let store = store();
        let user = slot(Lang::En, Tactic::Paraphrase, 1, Role::User);
        let system = user.other_role();
        assert!(matches!(
            save(
                &store,
                user,
                "Say it again. {PROTECTED}\n{TEXT}",
                None,
                None
            ),
            Saved::Stored { .. }
        ));
        assert!(matches!(
            save(&store, system, "Keep every fact.", None, None),
            Saved::Stored { .. }
        ));
        let before = rows_of(&store);
        assert_eq!(
            reset(&store, user),
            Reset::Refused {
                other: system,
                rule: "missing-variable"
            }
        );
        assert_eq!(rows_of(&store), before);
        assert_eq!(reset(&store, system), Reset::Done);
        assert_eq!(reset(&store, user), Reset::Done);
    }

    /// Г6: "Keep mine" moves `based_on` to today's shipped hash and
    /// touches nothing else.
    #[test]
    fn keep_mine_moves_based_on_and_nothing_else() {
        let store = store();
        let user = slot(Lang::En, Tactic::Humanize, 1, Role::User);
        let old = Override {
            based_on: "0000000000000000".to_owned(),
            ..Override::by_hand(user, "Make it sound human.\n{TEXT}")
        };
        config::write_prompt(&store, user, &old).expect("an old row");
        let overrides = config::overrides_of(&config::read_prompt_rows(&store));
        assert!(staleness(user, &old, &overrides).shipped_changed);
        keep_mine(&store, user).expect("kept");
        let Some(PromptRow::Read(kept)) = config::read_prompt_rows(&store).get(&user).cloned()
        else {
            panic!("the row is gone");
        };
        assert_eq!(kept.text, old.text);
        assert_eq!(
            kept.based_on,
            row::hash(shipped::template(user).expect("shipped"))
        );
        assert!(!staleness(user, &kept, &overrides).shipped_changed);
    }

    /// R5: saving a source changes no other row (never automatic, Q-B22);
    /// the adaptation made from the older source reads as stale; a hand
    /// claim records the source and its hash.
    #[test]
    fn saving_a_source_touches_no_other_language_and_its_adaptations_go_stale() {
        let store = store();
        let ru = slot(Lang::Ru, Tactic::Paraphrase, 1, Role::User);
        let de = ru.with_lang(Lang::De).expect("a German slot");
        assert!(matches!(
            save(&store, ru, "Перепиши своими словами.\n{TEXT}", None, None),
            Saved::Stored { .. }
        ));
        assert!(matches!(
            save(
                &store,
                de,
                "Schreib es mit eigenen Worten.\n{TEXT}",
                Some(Lang::Ru),
                None
            ),
            Saved::Stored { .. }
        ));
        let read = |store: &Store, slot| match config::read_prompt_rows(store).get(&slot) {
            Some(PromptRow::Read(read)) => read.clone(),
            other => panic!("{other:?}"),
        };
        let german = read(&store, de);
        assert_eq!(
            german.adapted_from,
            Some(AdaptedFrom {
                lang: Lang::Ru,
                hash: row::hash("Перепиши своими словами.\n{TEXT}"),
            })
        );
        let overrides = config::overrides_of(&config::read_prompt_rows(&store));
        assert!(!staleness(de, &german, &overrides).source_changed);

        let before = rows_of(&store);
        assert!(matches!(
            save(&store, ru, "Перескажи иначе.\n{TEXT}", None, None),
            Saved::Stored { .. }
        ));
        let after = rows_of(&store);
        for (key, value) in &before {
            if *key != row::key(ru) {
                assert_eq!(after.get(key), Some(value), "{key} moved with its source");
            }
        }
        let overrides = config::overrides_of(&config::read_prompt_rows(&store));
        assert!(
            staleness(de, &read(&store, de), &overrides).source_changed,
            "the German adaptation of an older Russian source is stale"
        );
    }

    fn handle_with(engine: FakeEngine) -> EngineHandle {
        EngineHandle::serving(Arc::new(engine), Pace::default()).0
    }

    /// R5: a model's adaptation is written only on the button, as
    /// `machine`; Save never asks the engine; the person's Save makes it
    /// `machine-reviewed`; an answer that renames `{TEXT}` is not saved.
    #[test]
    fn an_adaptation_is_asked_only_on_the_button_and_reviewed_by_a_save() {
        let store = store();
        let en = slot(Lang::En, Tactic::Paraphrase, 1, Role::User);
        let ru = en.with_lang(Lang::Ru).expect("a Russian slot");
        // The source is the English template as it is used — the saved
        // one below — so a faithful answer carries its variables only.
        let engine =
            FakeEngine::answering(|_, _| "Перескажи это своими словами.\n{TEXT}".to_owned());
        let handle = handle_with(engine.clone());

        assert!(matches!(
            save(&store, en, "Say it again.\n{TEXT}", None, None),
            Saved::Stored { .. }
        ));
        assert!(engine.asked().is_empty(), "a save asked the engine");

        let adapted = block_on(adapt_template(
            handle.clone(),
            store.clone(),
            Lang::En,
            ru,
            None,
            CancellationToken::new(),
        ));
        assert!(
            matches!(
                adapted,
                Adapted::Answered {
                    stored: Ok(true),
                    ..
                }
            ),
            "{adapted:?}"
        );
        assert_eq!(engine.asked().len(), 1);
        assert_eq!(handle.busy(), 0, "the adaptation let the engine go");
        let Some(PromptRow::Read(machine)) = config::read_prompt_rows(&store).get(&ru).cloned()
        else {
            panic!("nothing stored");
        };
        assert_eq!(machine.origin, Origin::Machine);
        assert_eq!(
            machine.adapted_from.as_ref().map(|source| source.lang),
            Some(Lang::En)
        );

        let reviewed = save(&store, ru, &machine.text, None, None);
        assert!(
            matches!(reviewed, Saved::Stored { reviewed: true, .. }),
            "{reviewed:?}"
        );
        let Some(PromptRow::Read(reviewed)) = config::read_prompt_rows(&store).get(&ru).cloned()
        else {
            panic!("the row is gone");
        };
        assert_eq!(reviewed.origin, Origin::MachineReviewed);
        assert_eq!(
            reviewed.adapted_from.map(|source| source.lang),
            Some(Lang::En)
        );

        // An answer that renames {TEXT} is refused, and nothing is written.
        let de = en.with_lang(Lang::De).expect("a German slot");
        let renamed = FakeEngine::answering(|_, _| "Schreib es neu.\n{TEKST}".to_owned());
        let before = rows_of(&store);
        let adapted = block_on(adapt_template(
            handle_with(renamed),
            store.clone(),
            Lang::En,
            de,
            None,
            CancellationToken::new(),
        ));
        assert!(
            matches!(
                adapted,
                Adapted::Answered {
                    stored: Ok(false),
                    ..
                }
            ),
            "{adapted:?}"
        );
        assert_eq!(rows_of(&store), before);
    }

    fn mentions(shown: &[Shown], message: Message, key: &str, value: &str) -> bool {
        shown.iter().any(|item| match item {
            Shown::Said(_, (said, args)) => {
                *said == message
                    && args
                        .get(key)
                        .is_some_and(|found| matches!(found, wipemark_i18n::FluentValue::String(text) if text.as_ref() == value))
            }
            Shown::Quoted(_) => false,
        })
    }

    /// R4: a check reads the rows and writes none, carries the edited
    /// text, shows an answer that drops a placeholder as the placeholder
    /// guard's refusal, and leaves no busy count — finished or cancelled.
    #[test]
    fn a_check_writes_nothing_and_lets_the_engine_go() {
        let store = store();
        let user = slot(Lang::En, Tactic::Paraphrase, 1, Role::User);
        assert!(matches!(
            save(&store, user, "SAVED wording.\n{TEXT}", None, None),
            Saved::Stored { .. }
        ));
        let before = rows_of(&store);
        let engine = FakeEngine::answering(|request, _| {
            // The sample, with its second placeholder dropped.
            let text = request
                .prompt
                .split("[[[BEGIN TEXT]]]\n")
                .nth(1)
                .and_then(|rest| rest.split("\n[[[END TEXT]]]").next())
                .unwrap_or_default();
            text.replace("⟦2⟧", "")
        });
        let handle = handle_with(engine.clone());
        let end = block_on(check_template(
            handle.clone(),
            store.clone(),
            user,
            "EDITED wording.\n{TEXT}".to_owned(),
            CancellationToken::new(),
        ));
        assert_eq!(rows_of(&store), before, "a check wrote to the database");
        assert_eq!(handle.busy(), 0, "a check kept the engine busy");
        let asked = engine.asked();
        assert_eq!(asked.len(), 1);
        assert!(asked[0].prompt.contains("EDITED wording."));
        assert!(!asked[0].prompt.contains("SAVED wording."));
        assert!(matches!(end, CheckEnd::Ran(TrialEnd::Done(_))), "{end:?}");
        let shown = check_lines(&end);
        assert!(
            mentions(
                &shown,
                Message::PromptsCheckGuardRejected,
                "guard",
                "placeholder"
            ),
            "the placeholder guard's refusal is not shown"
        );
        assert!(shown
            .iter()
            .any(|item| matches!(item, Shown::Said(_, (Message::PromptsCheckRejected, _)))));

        let cancel = CancellationToken::new();
        cancel.cancel();
        let end = block_on(check_template(
            handle.clone(),
            store.clone(),
            user,
            "EDITED wording.\n{TEXT}".to_owned(),
            cancel,
        ));
        assert_eq!(end, CheckEnd::Ran(TrialEnd::Cancelled));
        assert_eq!(handle.busy(), 0, "a cancelled check kept the engine busy");
        assert_eq!(rows_of(&store), before);
    }

    fn remote(on_this_machine: bool) -> Remote {
        Remote {
            profile: None,
            provider: crate::engine::Provider::Ollama,
            endpoint: "https://gpu.example.com/api/chat".to_owned(),
            origin: "https://gpu.example.com".to_owned(),
            model: "qwen3".to_owned(),
            temperature: 0.9,
            reasoning: crate::engine::ReasoningEffort::None,
            timeout: 120,
            account: None,
            on_this_machine,
        }
    }

    fn machine() -> Performer {
        Performer::Machine(duty::Local {
            id: "qwen3-4b".to_owned(),
            display: "Qwen3 4B".to_owned(),
            weights: std::env::temp_dir().join("qwen3.gguf"),
            format: wipemark_models::manifest::Format::Gguf,
            ctx: 8192,
            vendor: Vendor::OpenLlm,
            fit: wipemark_models::host::Fit::Unknown,
        })
    }

    /// R4: the sentence that the sample and the template go to an
    /// endpoint appears exactly when the duty does not stay here; with
    /// nobody on duty the buttons are greyed with the duty's sentence.
    #[test]
    fn the_endpoint_sentence_appears_exactly_when_the_work_leaves() {
        let user = slot(Lang::En, Tactic::Paraphrase, 1, Role::User);
        let says_sent = |duty: &Duty| {
            before_asking(user, duty)
                .iter()
                .any(|(message, _)| *message == Message::PromptsSentTo)
        };
        let away = Duty::assigned(Performer::Endpoint(remote(false)));
        let loopback = Duty::assigned(Performer::Endpoint(remote(true)));
        let here = Duty::assigned(machine());
        let nobody = Duty::Vacant(Vacancy::NoModelChosen);
        assert!(says_sent(&away));
        for duty in [&loopback, &here, &nobody] {
            assert!(!says_sent(duty), "{duty:?}");
        }
        for duty in [&away, &loopback, &here] {
            assert_eq!(
                before_asking(user, duty)[0].0,
                Message::PromptsCheckNote,
                "a check says it is not a rewrite"
            );
            assert_eq!(nobody_on_duty(duty), None);
        }
        assert!(nobody_on_duty(&nobody).is_some_and(|line| !line.is_empty()));

        let (_, _, banner) = super::banner(&away);
        let (_, _, quiet) = super::banner(&here);
        assert_eq!(
            banner.len(),
            quiet.len() + 1,
            "only the away banner says where"
        );
    }

    /// The warning ahead: an override in one language only says that the
    /// others run the shipped template, on both sides.
    #[test]
    fn an_override_in_one_language_says_the_others_use_the_shipped_one() {
        let ru = slot(Lang::Ru, Tactic::Paraphrase, 1, Role::User);
        let de = ru.with_lang(Lang::De).expect("a slot");
        let mut overrides = Overrides::new();
        assert!(coverage_line(ru, &overrides).is_none());
        overrides.insert(ru, Override::by_hand(ru, "Перепиши.\n{TEXT}"));
        assert_eq!(
            coverage_line(ru, &overrides).map(|line| line.0),
            Some(Message::PromptsCoverageHere)
        );
        assert_eq!(
            coverage_line(de, &overrides).map(|line| line.0),
            Some(Message::PromptsCoverageElsewhere)
        );
        let code = slot(Lang::En, Tactic::Code, 1, Role::User);
        overrides.insert(code, Override::by_hand(code, "Rewrite comments.\n{TEXT}"));
        assert!(
            coverage_line(code, &overrides).is_none(),
            "code is English only"
        );
    }

    /// The page in a window of its own, over preferences whose store
    /// forgets, with `rows` already written to it.
    fn page_with(
        cx: &mut gpui::TestAppContext,
        rows: Vec<(String, Value)>,
    ) -> (
        gpui::Entity<super::PromptsPage>,
        Arc<Store>,
        &mut gpui::VisualTestContext,
    ) {
        use gpui::AppContext as _;

        cx.update(gpui_component::init);
        let root = std::env::temp_dir().join(format!(
            "wipemark-prompts-page-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let homes = crate::retention::Homes {
            results: root.join("results"),
            kept: root.join("kept"),
        };
        type Held = Option<(gpui::Entity<super::PromptsPage>, Arc<Store>)>;
        let slot: std::rc::Rc<std::cell::RefCell<Held>> = std::rc::Rc::default();
        let held = slot.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let preferences = cx.new(|cx| crate::settings::Preferences::for_tests(homes, cx));
            let store = preferences.read(cx).store();
            for (key, value) in &rows {
                store.settings().set(key, value).expect("a row");
            }
            let page = cx.new(|cx| super::PromptsPage::new(preferences, window, cx));
            *held.borrow_mut() = Some((page.clone(), store));
            gpui_component::Root::new(page, window, cx)
        });
        cx.run_until_parked();
        let (page, store) = slot.take().expect("the window builder ran");
        (page, store, cx)
    }

    /// R1: a row this build cannot read is shown as such — the shipped
    /// text in the editor — and is still in the database, byte for byte,
    /// after the page has been opened, looked at, let go of and opened
    /// again. (Closing the test window would stop the MCP supervisor the
    /// preferences started, on a thread of its own, which the test
    /// scheduler refuses; letting the page go is the same drop.)
    #[gpui::test]
    fn an_unreadable_row_is_shown_and_still_there_after_the_page_closes(
        cx: &mut gpui::TestAppContext,
    ) {
        use gpui::AppContext as _;

        let user = slot(Lang::Ru, Tactic::Humanize, 1, Role::User);
        let value = json!({"text": "Перепиши.", "origin": "robot"});
        let (page, store, cx) = page_with(cx, vec![(row::key(user), value.clone())]);
        let look = |page: &gpui::Entity<super::PromptsPage>, cx: &mut gpui::VisualTestContext| {
            cx.update(|window, cx| {
                page.update(cx, |page, cx| page.select(user, window, cx));
            });
            cx.run_until_parked();
            cx.update(|_, cx| {
                let page = page.read(cx);
                assert!(
                    matches!(page.row(), Some(PromptRow::Unread(Some(_)))),
                    "the unreadable row is not shown as one"
                );
                assert_eq!(
                    page.edited(cx),
                    shipped::template(user).expect("shipped"),
                    "the slot uses the shipped template"
                );
            });
        };
        look(&page, cx);
        let preferences = cx.update(|_, cx| page.read(cx).preferences.clone());
        drop(page);
        cx.run_until_parked();
        let again =
            cx.update(|window, cx| cx.new(|cx| super::PromptsPage::new(preferences, window, cx)));
        cx.run_until_parked();
        look(&again, cx);
        assert_eq!(
            store
                .settings()
                .get::<Value>(&row::key(user))
                .expect("read"),
            Some(value),
            "the page corrected a row it could not read"
        );
    }

    /// R3 through the page: a broken `{TEXT}` is shown with its rule id
    /// as it is typed, and Save refuses it and writes nothing; fixed, it
    /// is saved.
    #[gpui::test]
    fn the_page_refuses_a_broken_template_and_saves_a_fixed_one(cx: &mut gpui::TestAppContext) {
        let user = slot(Lang::Ru, Tactic::Paraphrase, 1, Role::User);
        let (page, store, cx) = page_with(cx, Vec::new());
        cx.update(|window, cx| {
            page.update(cx, |page, cx| {
                page.select(user, window, cx);
                page.editor.update(cx, |editor, cx| {
                    editor.replace_all("Перепиши.\n{TEKST}", window, cx);
                });
            });
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            let rules: Vec<&str> = page.read(cx).problems.iter().map(Problem::rule).collect();
            assert!(rules.contains(&"unknown-variable"), "{rules:?}");
        });
        cx.update(|window, cx| page.update(cx, |page, cx| page.save(window, cx)));
        cx.run_until_parked();
        assert!(
            rows_of(&store).is_empty(),
            "a refused template was written: {:?}",
            rows_of(&store)
        );
        cx.update(|_, cx| {
            assert!(matches!(
                page.read(cx).said,
                Some(super::Said::Saved(Saved::Refused(_)))
            ));
        });

        cx.update(|window, cx| {
            page.update(cx, |page, cx| {
                page.editor.update(cx, |editor, cx| {
                    editor.replace_all("Перепиши своими словами.\n{TEXT}", window, cx);
                });
                page.save(window, cx);
            });
        });
        cx.run_until_parked();
        assert!(matches!(
            config::read_prompt_rows(&store).get(&user),
            Some(PromptRow::Read(read)) if read.text == "Перепиши своими словами.\n{TEXT}"
        ));
    }
}
