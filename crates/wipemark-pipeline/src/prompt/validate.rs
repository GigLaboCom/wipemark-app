//! What is wrong with a template, as values (D74; notes §3a rules 1–8 and
//! the warnings).
//!
//! A user edits templates and the product checks them. Two things can be
//! broken by an edit: the *structure* — the assembler cannot find where
//! the text goes — and the *contract* — the model stops keeping `⟦n⟧`,
//! numbers and paragraphs. The first is checked strictly here, before a
//! template is saved; the second cannot be checked by meaning, which is
//! why `{PROTECTED}` is mandatory and the rest is left to the guards and
//! the "Check template" dry run (E4-6).
//!
//! Every [`Problem`] is a value with a [`Problem::rule`] — a stable id, a
//! format the CLI names on exit 2 (D75) — and a [`Problem::severity`]. An
//! error stops a save; a warning is said and does not. Nothing here is a
//! sentence: the Settings page and the CLI choose the words.

use std::ops::Range;

use wipemark_core::TextStats;

use super::template::{self, Fault};
use super::{row, shipped, Intensity, Marker, Role, Slot, Variable};
use crate::lang::Lang;

/// Whether a problem stops a template from being saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    /// The template is not saved (or, read from a row, not used).
    Error,
    /// Saved, and the page says why it may not do what was meant.
    Warning,
}

/// Which side of a brace pair is missing its partner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BraceSide {
    /// `{TEXT` — opened, never closed on its line.
    Open,
    /// `TEXT}` — closed, never opened.
    Close,
}

/// The script a set's templates are written in. `en` and `de` share one,
/// so this check cannot tell them apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Script {
    Latin,
    Cyrillic,
}

impl Script {
    /// The script a language's templates are expected to be in.
    pub fn of(lang: Lang) -> Script {
        match lang {
            Lang::En | Lang::De => Script::Latin,
            Lang::Ru => Script::Cyrillic,
        }
    }
}

/// One thing wrong with a template. Positions are byte offsets into the
/// template's text.
#[derive(Debug, Clone, PartialEq)]
pub enum Problem {
    /// R1. `{NAME}` where `NAME` is not a variable — `{TEKST}`, `{text}`,
    /// `{ТЕКСТ}`. `suggestion` is the variable whose name is within two
    /// edits of `NAME` in upper case, when there is one.
    UnknownVariable {
        name: String,
        span: Range<usize>,
        suggestion: Option<Variable>,
    },
    /// R2. A brace with no partner on its line. Literal braces are written
    /// `{{` and `}}`.
    UnclosedBrace { at: usize, side: BraceSide },
    /// R3. A required variable is absent: `{TEXT}` from a user template,
    /// or `{PROTECTED}` from both templates of a step.
    MissingVariable { variable: Variable },
    /// R3. A variable that may occur once occurs more often: `{TEXT}`, and
    /// `{PREV_CONTEXT}` — each carries a pair of markers.
    RepeatedVariable {
        variable: Variable,
        spans: Vec<Range<usize>>,
    },
    /// R4. A variable in a turn it does not belong to — `{TEXT}`,
    /// `{PREV_CONTEXT}` or `{INTENSITY}` in a system template.
    MisplacedVariable {
        variable: Variable,
        span: Range<usize>,
        role: Role,
    },
    /// R5. A marker written by hand: only the assembler writes them.
    HandWrittenMarker { marker: Marker, span: Range<usize> },
    /// R6. A `⟦` or `⟧`. The placeholders are the document's; a template
    /// that wrote one would be read as one of them, and the sentence that
    /// explains them is `{PROTECTED}`.
    ReservedBracket { at: usize },
    /// R7. Nothing but whitespace. An empty field is "restore the
    /// default", never an empty prompt.
    Empty,
    /// R8. Over a tenth of the model's window, estimated at three bytes a
    /// token — the template would eat the chunk's budget.
    TooLong { estimated_tokens: u32, limit: u32 },
    /// R9. A character Layer A removes at its defaults — a zero-width
    /// character, a bidi control, a tag, a variation selector out of
    /// place, a soft hyphen, a private-use or default-ignorable code point
    /// (D369). A template carrying one would hand the model the very marks
    /// this product removes, unseen in the field. One problem per code
    /// point: `at` is its first byte offset, `count` how often it occurs.
    InvisibleCharacter {
        codepoint: char,
        at: usize,
        count: u32,
    },
    /// W1. The template's letters are mostly not in its set's script, so a
    /// model will lean towards answering in another language (Q-B1).
    /// Shares are fractions 0.0–1.0 of the template's letters, variables
    /// left out.
    ScriptMismatch {
        expected: Script,
        latin: f32,
        cyrillic: f32,
    },
    /// W2. A user template with no instruction around its variables — it
    /// is not obvious what the model is meant to do.
    NothingButText,
    /// W3. An intensity is set and this tactic takes one, but the user
    /// template has no `{INTENSITY}`, so the setting does nothing.
    NoIntensity { intensity: Intensity },
    /// W4. The shipped template changed after this override was made from
    /// it: `based_on` is the hash it was made from, `shipped` today's.
    Stale { based_on: String, shipped: String },
    /// A1. An adaptation's variables are not its source's, counted:
    /// `missing` are in the source and not the adaptation, `extra` the
    /// other way round, each repeated as often as it is short or over.
    VariablesDiffer {
        missing: Vec<Variable>,
        extra: Vec<Variable>,
    },
}

impl Problem {
    pub fn severity(&self) -> Severity {
        match self {
            Problem::UnknownVariable { .. }
            | Problem::UnclosedBrace { .. }
            | Problem::MissingVariable { .. }
            | Problem::RepeatedVariable { .. }
            | Problem::MisplacedVariable { .. }
            | Problem::HandWrittenMarker { .. }
            | Problem::ReservedBracket { .. }
            | Problem::Empty
            | Problem::TooLong { .. }
            | Problem::InvisibleCharacter { .. }
            | Problem::VariablesDiffer { .. } => Severity::Error,
            Problem::ScriptMismatch { .. }
            | Problem::NothingButText
            | Problem::NoIntensity { .. }
            | Problem::Stale { .. } => Severity::Warning,
        }
    }

    /// The rule's stable id — a format, never translated.
    pub fn rule(&self) -> &'static str {
        match self {
            Problem::UnknownVariable { .. } => "unknown-variable",
            Problem::UnclosedBrace { .. } => "unclosed-brace",
            Problem::MissingVariable { .. } => "missing-variable",
            Problem::RepeatedVariable { .. } => "repeated-variable",
            Problem::MisplacedVariable { .. } => "misplaced-variable",
            Problem::HandWrittenMarker { .. } => "hand-written-marker",
            Problem::ReservedBracket { .. } => "reserved-bracket",
            Problem::Empty => "empty",
            Problem::TooLong { .. } => "too-long",
            Problem::InvisibleCharacter { .. } => "invisible-character",
            Problem::ScriptMismatch { .. } => "script-mismatch",
            Problem::NothingButText => "nothing-but-text",
            Problem::NoIntensity { .. } => "no-intensity",
            Problem::Stale { .. } => "stale",
            Problem::VariablesDiffer { .. } => "variables-differ",
        }
    }
}

/// What a template is checked against besides its own text.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValidationContext<'a> {
    /// The step's other template as it will be used — the override if
    /// there is one, else the shipped text. `{PROTECTED}` is required once
    /// per *step*, so neither template can be judged alone.
    pub other_role: &'a str,
    /// The model's context window in tokens; `None` skips the length rule
    /// (an endpoint whose window is unknown, D57's `ctx_len`).
    pub ctx_len: Option<u32>,
    /// The intensity currently set, for the `{INTENSITY}` warning.
    pub intensity: Intensity,
    /// The hash of the shipped template an override was made from, for the
    /// staleness warning; `None` for a template that is not an override.
    pub based_on: Option<&'a str>,
}

impl ValidationContext<'static> {
    /// The context for checking a template beside the shipped text of its
    /// step's other turn — what "Restore default" on the other turn would
    /// leave.
    pub fn beside_shipped(slot: Slot) -> ValidationContext<'static> {
        ValidationContext {
            other_role: shipped::template(slot.other_role()).unwrap_or_default(),
            ctx_len: None,
            intensity: Intensity::Moderate,
            based_on: None,
        }
    }
}

/// Tokens a text is estimated to cost when no tokenizer is at hand: three
/// bytes a token, rounded up. Pessimistic for English (about four
/// characters a token) and about right for Cyrillic, where a letter is two
/// bytes — a check that errs towards "too long" costs a shorter template,
/// one that errs the other way costs the chunk its budget.
pub fn estimate_tokens(text: &str) -> u32 {
    u32::try_from(text.len().div_ceil(3)).unwrap_or(u32::MAX)
}

/// Everything wrong with `text` as the template of `slot`, errors and
/// warnings, in rule order. An empty list is a template the assembler will
/// render.
pub fn validate(slot: Slot, text: &str, context: &ValidationContext<'_>) -> Vec<Problem> {
    if text.trim().is_empty() {
        return vec![Problem::Empty];
    }
    let parsed = template::parse(text);
    let mut problems = Vec::new();

    // R1, R2.
    for fault in &parsed.faults {
        problems.push(match fault {
            Fault::Unknown { name, span } => Problem::UnknownVariable {
                name: name.clone(),
                span: span.clone(),
                suggestion: nearest(name),
            },
            Fault::UnclosedOpen { at } => Problem::UnclosedBrace {
                at: *at,
                side: BraceSide::Open,
            },
            Fault::StrayClose { at } => Problem::UnclosedBrace {
                at: *at,
                side: BraceSide::Close,
            },
        });
    }

    // R3: the text exactly once, the context at most once.
    if slot.role() == Role::User {
        let texts = spans_of(&parsed, Variable::Text);
        match texts.len() {
            0 => problems.push(Problem::MissingVariable {
                variable: Variable::Text,
            }),
            1 => {}
            _ => problems.push(Problem::RepeatedVariable {
                variable: Variable::Text,
                spans: texts,
            }),
        }
        let contexts = spans_of(&parsed, Variable::PrevContext);
        if contexts.len() > 1 {
            problems.push(Problem::RepeatedVariable {
                variable: Variable::PrevContext,
                spans: contexts,
            });
        }
    }
    // R3: the placeholder rule once per step, either turn.
    let protected_elsewhere = template::parse(context.other_role).count(Variable::Protected);
    if parsed.count(Variable::Protected) + protected_elsewhere == 0 {
        problems.push(Problem::MissingVariable {
            variable: Variable::Protected,
        });
    }

    // R4.
    for (variable, span) in parsed.variables() {
        if !variable.allowed_in(slot.role()) {
            problems.push(Problem::MisplacedVariable {
                variable,
                span: span.clone(),
                role: slot.role(),
            });
        }
    }

    // R5.
    for marker in Marker::ALL {
        for (at, found) in text.match_indices(marker.as_str()) {
            problems.push(Problem::HandWrittenMarker {
                marker,
                span: at..at + found.len(),
            });
        }
    }

    // R6.
    for (at, c) in text.char_indices() {
        if c == '\u{27E6}' || c == '\u{27E7}' {
            problems.push(Problem::ReservedBracket { at });
        }
    }

    // R9: what Layer A would remove at its defaults — the same decision,
    // context included (a joiner inside an emoji is kept there and here).
    for finding in wipemark_core::inspect(text, &wipemark_core::Options::default()).findings {
        problems.push(Problem::InvisibleCharacter {
            codepoint: finding.codepoint,
            at: finding.positions.first().copied().unwrap_or(0),
            count: finding.count,
        });
    }

    // R8.
    if let Some(window) = context.ctx_len {
        let limit = window / 10;
        let estimated_tokens = estimate_tokens(text);
        if estimated_tokens > limit {
            problems.push(Problem::TooLong {
                estimated_tokens,
                limit,
            });
        }
    }

    // W1, W2.
    let literal = parsed.literal();
    if literal.chars().any(char::is_alphabetic) {
        let stats = TextStats::of(&literal);
        let expected = Script::of(slot.lang());
        let share = match expected {
            Script::Latin => stats.latin_ratio,
            Script::Cyrillic => stats.cyrillic_ratio,
        };
        if share < 0.5 {
            problems.push(Problem::ScriptMismatch {
                expected,
                latin: stats.latin_ratio,
                cyrillic: stats.cyrillic_ratio,
            });
        }
    }
    if slot.role() == Role::User && !literal.chars().any(char::is_alphanumeric) {
        problems.push(Problem::NothingButText);
    }

    // W3.
    if slot.role() == Role::User
        && slot.tactic().takes_intensity()
        && context.intensity != Intensity::Moderate
        && parsed.count(Variable::Intensity) == 0
    {
        problems.push(Problem::NoIntensity {
            intensity: context.intensity,
        });
    }

    // W4.
    if let (Some(based_on), Some(shipped)) = (context.based_on, shipped::template(slot)) {
        let shipped = row::hash(shipped);
        if based_on != shipped {
            problems.push(Problem::Stale {
                based_on: based_on.to_owned(),
                shipped,
            });
        }
    }

    problems
}

fn spans_of(parsed: &template::Parsed, variable: Variable) -> Vec<Range<usize>> {
    parsed
        .variables()
        .filter(|(found, _)| *found == variable)
        .map(|(_, span)| span.clone())
        .collect()
}

/// The variable a misspelt name most likely meant: the closest by edit
/// distance over the upper-cased name, when that distance is at most two.
fn nearest(name: &str) -> Option<Variable> {
    let upper = name.to_uppercase();
    Variable::ALL
        .into_iter()
        .map(|variable| (edit_distance(&upper, variable.name()), variable))
        .filter(|(distance, _)| *distance <= 2)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, variable)| variable)
}

/// Levenshtein distance over code points.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut current = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let substitute = previous[j] + usize::from(ca != *cb);
            current[j + 1] = substitute.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        previous = current;
    }
    previous[b.len()]
}

#[cfg(test)]
mod tests {
    use super::{
        edit_distance, estimate_tokens, validate, BraceSide, Problem, Script, Severity,
        ValidationContext,
    };
    use crate::lang::Lang;
    use crate::prompt::{row, shipped, Intensity, Marker, Role, Slot, Tactic, Variable};

    fn slot(lang: Lang, role: Role) -> Slot {
        Slot::new(lang, Tactic::Paraphrase, 1, role).expect("slot")
    }

    /// A context whose other turn carries `{PROTECTED}`, so a test of one
    /// rule is not also a test of R3.
    fn beside(other: &str) -> ValidationContext<'_> {
        ValidationContext {
            other_role: other,
            ctx_len: None,
            intensity: Intensity::Moderate,
            based_on: None,
        }
    }

    const GUARDED: &str = "Keep everything. {PROTECTED}";

    fn user(text: &str) -> Vec<Problem> {
        validate(slot(Lang::En, Role::User), text, &beside(GUARDED))
    }

    fn rules(problems: &[Problem]) -> Vec<&'static str> {
        problems.iter().map(Problem::rule).collect()
    }

    #[test]
    fn a_plain_user_template_has_no_problem() {
        assert_eq!(
            user("Rewrite this.\n{INTENSITY}\n{PREV_CONTEXT}\n{TEXT}"),
            vec![]
        );
    }

    #[test]
    fn an_unknown_variable_is_an_error_with_a_suggestion() {
        let problems = user("Rewrite {TEKST} and {text} and {ТЕКСТ} and {FOO_BAR_BAZ}.\n{TEXT}");
        let unknown: Vec<_> = problems
            .iter()
            .filter_map(|problem| match problem {
                Problem::UnknownVariable {
                    name, suggestion, ..
                } => Some((name.as_str(), *suggestion)),
                _ => None,
            })
            .collect();
        assert_eq!(
            unknown,
            vec![
                ("TEKST", Some(Variable::Text)),
                ("text", Some(Variable::Text)),
                ("ТЕКСТ", None),
                ("FOO_BAR_BAZ", None),
            ]
        );
        assert!(problems.iter().all(|p| p.severity() == Severity::Error));
        let Problem::UnknownVariable { span, .. } = &problems[0] else {
            panic!("{problems:?}");
        };
        assert_eq!(span, &(8..15));
    }

    #[test]
    fn an_unclosed_brace_is_an_error() {
        assert_eq!(
            user("Rewrite {TEXT\n{TEXT}"),
            vec![Problem::UnclosedBrace {
                at: 8,
                side: BraceSide::Open
            }]
        );
        assert_eq!(
            user("Rewrite TEXT}\n{TEXT}"),
            vec![Problem::UnclosedBrace {
                at: 12,
                side: BraceSide::Close
            }]
        );
        assert_eq!(user("Braces: {{like this}}.\n{TEXT}"), vec![]);
    }

    #[test]
    fn text_must_be_in_the_user_template_exactly_once() {
        assert_eq!(
            user("Rewrite it."),
            vec![Problem::MissingVariable {
                variable: Variable::Text
            }]
        );
        assert_eq!(
            user("Rewrite {TEXT} and {TEXT}."),
            vec![Problem::RepeatedVariable {
                variable: Variable::Text,
                spans: vec![8..14, 19..25],
            }]
        );
    }

    #[test]
    fn context_may_appear_at_most_once() {
        assert_eq!(
            rules(&user("Rewrite.\n{PREV_CONTEXT}\n{PREV_CONTEXT}\n{TEXT}")),
            vec!["repeated-variable"]
        );
    }

    /// The placeholder rule is per step: either turn may carry it, and a
    /// step where neither does is refused from both sides.
    #[test]
    fn protected_is_required_once_per_step() {
        let missing = vec![Problem::MissingVariable {
            variable: Variable::Protected,
        }];
        let system = slot(Lang::En, Role::System);
        let user_slot = slot(Lang::En, Role::User);
        assert_eq!(
            validate(system, "Keep everything.", &beside("Go.\n{TEXT}")),
            missing
        );
        assert_eq!(
            validate(user_slot, "Go.\n{TEXT}", &beside("Keep everything.")),
            missing
        );
        assert_eq!(
            validate(
                system,
                "Keep everything.",
                &beside("Go. {PROTECTED}\n{TEXT}")
            ),
            vec![]
        );
        assert_eq!(validate(user_slot, "Go.\n{TEXT}", &beside(GUARDED)), vec![]);
    }

    #[test]
    fn text_in_the_system_template_is_misplaced() {
        let problems = validate(
            slot(Lang::En, Role::System),
            "Keep it. {PROTECTED} {TEXT} {PREV_CONTEXT} {INTENSITY}",
            &beside("Go.\n{TEXT}"),
        );
        let misplaced: Vec<_> = problems
            .iter()
            .filter_map(|problem| match problem {
                Problem::MisplacedVariable { variable, role, .. } => Some((*variable, *role)),
                _ => None,
            })
            .collect();
        assert_eq!(
            misplaced,
            vec![
                (Variable::Text, Role::System),
                (Variable::PrevContext, Role::System),
                (Variable::Intensity, Role::System),
            ]
        );
        assert_eq!(problems.len(), 3, "{problems:?}");
    }

    #[test]
    fn a_hand_written_marker_is_an_error() {
        let problems = user("Rewrite.\n[[[BEGIN TEXT]]]\n{TEXT}\n[[[END TEXT]]]");
        assert_eq!(
            problems,
            vec![
                Problem::HandWrittenMarker {
                    marker: Marker::BeginText,
                    span: 9..25
                },
                Problem::HandWrittenMarker {
                    marker: Marker::EndText,
                    span: 33..47
                },
            ]
        );
        for marker in Marker::ALL {
            let text = format!("Rewrite {}.\n{{TEXT}}", marker.as_str());
            assert_eq!(rules(&user(&text)), vec!["hand-written-marker"]);
        }
    }

    #[test]
    fn a_placeholder_bracket_is_an_error() {
        assert_eq!(
            user("Keep ⟦1⟧ as it is.\n{TEXT}"),
            vec![
                Problem::ReservedBracket { at: 5 },
                Problem::ReservedBracket {
                    at: 5 + '⟦'.len_utf8() + 1
                },
            ]
        );
        assert_eq!(rules(&user("Keep ⟦n⟧.\n{TEXT}")).len(), 2);
    }

    /// R9 (D369): a character Layer A removes is refused in any template —
    /// typed by hand or written by a model — at its first place, once per
    /// code point; what Layer A keeps by context is not one, and neither is
    /// ordinary typography (a no-break space, „quotes“, a dash).
    #[test]
    fn an_invisible_character_is_an_error() {
        let problems = user("Re\u{200B}write\u{200B} this.\n{TEXT}");
        assert_eq!(
            problems,
            vec![Problem::InvisibleCharacter {
                codepoint: '\u{200B}',
                at: 2,
                count: 2,
            }]
        );
        assert_eq!(problems[0].severity(), Severity::Error);
        for c in [
            '\u{202E}',
            '\u{2066}',
            '\u{200D}',
            '\u{00AD}',
            '\u{E0041}',
            '\u{FEFF}',
        ] {
            let text = format!("Rewrite{c} this.\n{{TEXT}}");
            assert_eq!(
                rules(&user(&text)),
                vec!["invisible-character"],
                "U+{:04X}",
                u32::from(c)
            );
        }
        assert_eq!(
            user("Rewrite\u{00A0}it — „so“ … 👍🏽 ❤\u{FE0F}.\n{TEXT}"),
            vec![]
        );
    }

    #[test]
    fn an_empty_template_is_an_error() {
        assert_eq!(user(""), vec![Problem::Empty]);
        assert_eq!(user(" \n\t "), vec![Problem::Empty]);
    }

    #[test]
    fn a_template_over_a_tenth_of_the_window_is_too_long() {
        let text = format!("{}\n{{TEXT}}", "word ".repeat(100)); // 507 bytes, 169 tokens
        let mut context = beside(GUARDED);
        context.ctx_len = Some(1690);
        assert_eq!(user_with(&text, &context), vec![]);
        context.ctx_len = Some(1680);
        assert_eq!(
            user_with(&text, &context),
            vec![Problem::TooLong {
                estimated_tokens: 169,
                limit: 168
            }]
        );
        context.ctx_len = None;
        assert_eq!(user_with(&text, &context), vec![]);
    }

    fn user_with(text: &str, context: &ValidationContext<'_>) -> Vec<Problem> {
        validate(slot(Lang::En, Role::User), text, context)
    }

    #[test]
    fn a_latin_template_in_the_russian_set_is_a_warning() {
        let problems = validate(
            slot(Lang::Ru, Role::User),
            "Rewrite the text.\n{TEXT}",
            &beside(GUARDED),
        );
        assert_eq!(rules(&problems), vec!["script-mismatch"]);
        assert_eq!(problems[0].severity(), Severity::Warning);
        let Problem::ScriptMismatch { expected, .. } = problems[0] else {
            panic!("{problems:?}");
        };
        assert_eq!(expected, Script::Cyrillic);
        // The variables' Latin names are not the template's letters.
        assert_eq!(
            validate(
                slot(Lang::Ru, Role::User),
                "Перепиши текст.\n{INTENSITY}\n{PREV_CONTEXT}\n{TEXT}",
                &beside(GUARDED)
            ),
            vec![]
        );
        assert_eq!(
            rules(&validate(
                slot(Lang::De, Role::User),
                "Перепиши текст.\n{TEXT}",
                &beside(GUARDED)
            )),
            vec!["script-mismatch"]
        );
    }

    #[test]
    fn a_user_template_of_nothing_but_text_is_a_warning() {
        let problems = user("{TEXT}");
        assert_eq!(problems, vec![Problem::NothingButText]);
        assert_eq!(problems[0].severity(), Severity::Warning);
        assert_eq!(
            user("  {PREV_CONTEXT}\n\n{TEXT} …"),
            vec![Problem::NothingButText]
        );
    }

    #[test]
    fn no_intensity_while_one_is_set_is_a_warning() {
        let mut context = beside(GUARDED);
        context.intensity = Intensity::Strong;
        assert_eq!(
            user_with("Rewrite.\n{TEXT}", &context),
            vec![Problem::NoIntensity {
                intensity: Intensity::Strong
            }]
        );
        assert_eq!(user_with("Rewrite.\n{INTENSITY}\n{TEXT}", &context), vec![]);
        context.intensity = Intensity::Moderate;
        assert_eq!(user_with("Rewrite.\n{TEXT}", &context), vec![]);
        // A tactic that takes no intensity is not asked for one.
        context.intensity = Intensity::Light;
        let outline = Slot::new(Lang::En, Tactic::Structural, 1, Role::User).expect("slot");
        assert_eq!(validate(outline, "Outline it.\n{TEXT}", &context), vec![]);
    }

    #[test]
    fn an_override_of_a_changed_template_is_stale() {
        let slot = slot(Lang::En, Role::User);
        let shipped = row::hash(shipped::template(slot).expect("shipped"));
        let mut context = beside(GUARDED);
        context.based_on = Some(&shipped);
        assert_eq!(validate(slot, "Rewrite.\n{TEXT}", &context), vec![]);
        context.based_on = Some("0000000000000000");
        let problems = validate(slot, "Rewrite.\n{TEXT}", &context);
        assert_eq!(
            problems,
            vec![Problem::Stale {
                based_on: "0000000000000000".to_owned(),
                shipped,
            }]
        );
        assert_eq!(problems[0].severity(), Severity::Warning);
    }

    #[test]
    fn rule_ids_are_distinct() {
        let all = [
            Problem::UnknownVariable {
                name: String::new(),
                span: 0..0,
                suggestion: None,
            },
            Problem::UnclosedBrace {
                at: 0,
                side: BraceSide::Open,
            },
            Problem::MissingVariable {
                variable: Variable::Text,
            },
            Problem::RepeatedVariable {
                variable: Variable::Text,
                spans: vec![],
            },
            Problem::MisplacedVariable {
                variable: Variable::Text,
                span: 0..0,
                role: Role::System,
            },
            Problem::HandWrittenMarker {
                marker: Marker::EndText,
                span: 0..0,
            },
            Problem::ReservedBracket { at: 0 },
            Problem::Empty,
            Problem::TooLong {
                estimated_tokens: 0,
                limit: 0,
            },
            Problem::InvisibleCharacter {
                codepoint: '\u{200B}',
                at: 0,
                count: 1,
            },
            Problem::ScriptMismatch {
                expected: Script::Latin,
                latin: 0.0,
                cyrillic: 0.0,
            },
            Problem::NothingButText,
            Problem::NoIntensity {
                intensity: Intensity::Light,
            },
            Problem::Stale {
                based_on: String::new(),
                shipped: String::new(),
            },
            Problem::VariablesDiffer {
                missing: vec![],
                extra: vec![],
            },
        ];
        let mut ids: Vec<_> = all.iter().map(Problem::rule).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), all.len());
    }

    #[test]
    fn the_estimate_is_three_bytes_a_token_rounded_up() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 2);
        assert_eq!(estimate_tokens("абв"), 2);
        assert_eq!(edit_distance("TEKST", "TEXT"), 2);
        assert_eq!(edit_distance("", "TEXT"), 4);
    }
}
