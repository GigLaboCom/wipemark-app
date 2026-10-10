//! The assembler: a step's two templates, a chunk, its context and an
//! intensity become the `system` and `prompt` of one request (D66).
//!
//! The assembler owns the markers. `{TEXT}` becomes the chunk between
//! `[[[BEGIN TEXT]]]` and `[[[END TEXT]]]`; `{PREV_CONTEXT}` becomes the
//! context between `[[[BEGIN CONTEXT]]]` and `[[[END CONTEXT]]]` followed by
//! the sentence that says not to rewrite or repeat it, or nothing when the
//! chunk has no context; `{PROTECTED}` becomes the placeholder sentence
//! when the chunk holds a `⟦n⟧`, or nothing; `{INTENSITY}` becomes the
//! intensity clause, or nothing for moderate. Every sentence is in the
//! template's own language. `{{` and `}}` are literal braces.
//!
//! A line that holds nothing but a variable that expanded to nothing is
//! removed, with one blank line before it, so an absent context does not
//! leave a hole in the prompt; the result is trimmed at both ends — the
//! chunk sits inside its markers, so a trim never reaches it.
//!
//! Every template is validated before it is rendered, the shipped ones
//! included: no template reaches a model that the validator would refuse.

use wipemark_engine::{ChatRequest, SamplingParams};

use super::choose::{Chosen, StepTemplates};
use super::shipped::{self, Fragment};
use super::template::{self, Token};
use super::validate::{validate, Problem, Severity, ValidationContext};
use super::{Intensity, Marker, Slot, Variable};

/// What one step is asked to rewrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Input<'a> {
    /// The chunk — for a second step, the first step's cleaned answer —
    /// with its protected spans already `⟦n⟧`.
    pub text: &'a str,
    /// The previous chunk's last sentences, protected spans written back
    /// (D70); `None` for the first chunk.
    pub context: Option<&'a str>,
    pub intensity: Intensity,
}

/// The two strings of a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub system: String,
    pub prompt: String,
}

impl Rendered {
    /// The request, with the sampling the loop chose (E4-3).
    pub fn into_request(self, params: SamplingParams) -> ChatRequest {
        ChatRequest {
            system: Some(self.system),
            prompt: self.prompt,
            params,
        }
    }
}

/// Why a step was not rendered.
#[derive(Debug, Clone, PartialEq)]
pub enum RenderError {
    /// A template the validator refuses; the errors are listed. The caller
    /// falls back to the shipped template of that slot.
    Template { slot: Slot, problems: Vec<Problem> },
    /// The chunk or its context contains a marker string. Preparing the
    /// text turns one into a protected span (E4-1, D66); this is the
    /// second lock, because a marker inside the material would end it
    /// early in the model's eyes.
    MarkerInText { marker: Marker },
}

/// Render one step.
pub fn render(step: &StepTemplates, input: &Input<'_>) -> Result<Rendered, RenderError> {
    for (this, other) in [(&step.system, &step.user), (&step.user, &step.system)] {
        refuse_invalid(this, other)?;
    }
    for marker in super::Marker::ALL {
        let inside = |text: &str| text.contains(marker.as_str());
        if inside(input.text) || input.context.is_some_and(inside) {
            return Err(RenderError::MarkerInText { marker });
        }
    }
    let system = expand(&step.system, input);
    let mut prompt = expand(&step.user, input);
    if step.fallback {
        prompt.push_str("\n\n");
        prompt.push_str(shipped::fallback_clause());
    }
    Ok(Rendered { system, prompt })
}

fn refuse_invalid(this: &Chosen, other: &Chosen) -> Result<(), RenderError> {
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
    if problems.is_empty() {
        Ok(())
    } else {
        Err(RenderError::Template {
            slot: this.slot,
            problems,
        })
    }
}

/// Whether `text` holds a placeholder as E4-1 writes them: `⟦`, ASCII
/// digits, `⟧`.
pub fn has_placeholder(text: &str) -> bool {
    text.match_indices('\u{27E6}').any(|(at, open)| {
        let rest = &text[at + open.len()..];
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        digits > 0 && rest[digits..].starts_with('\u{27E7}')
    })
}

/// A block between two markers, the content on its own lines.
fn block(begin: Marker, content: &str, end: Marker) -> String {
    let newline = if content.ends_with('\n') { "" } else { "\n" };
    format!("{}\n{content}{newline}{}", begin.as_str(), end.as_str())
}

/// What a variable becomes in a template of `slot`'s language.
fn expansion(slot: Slot, variable: Variable, input: &Input<'_>) -> String {
    let lang = slot.lang();
    match variable {
        Variable::Text => block(Marker::BeginText, input.text, Marker::EndText),
        Variable::PrevContext => match input.context.filter(|context| !context.trim().is_empty()) {
            Some(context) => format!(
                "{}\n{}",
                block(Marker::BeginContext, context, Marker::EndContext),
                shipped::fragment(lang, Fragment::Context)
            ),
            None => String::new(),
        },
        Variable::Protected if has_placeholder(input.text) => {
            shipped::fragment(lang, Fragment::Protected).to_owned()
        }
        Variable::Protected => String::new(),
        Variable::Intensity => Fragment::of_intensity(input.intensity)
            .map(|piece| shipped::fragment(lang, piece).to_owned())
            .unwrap_or_default(),
    }
}

fn expand(chosen: &Chosen, input: &Input<'_>) -> String {
    let mut lines: Vec<String> = Vec::new();
    for line in chosen.text.split_inclusive('\n') {
        let parsed = template::parse(line);
        let mut rendered = String::new();
        let mut variables = 0;
        let mut words = false;
        for token in &parsed.tokens {
            match token {
                Token::Text(text) => {
                    words |= !text.trim().is_empty();
                    rendered.push_str(text);
                }
                Token::Var { variable, .. } => {
                    variables += 1;
                    rendered.push_str(&expansion(chosen.slot, *variable, input));
                }
            }
        }
        let vanished = variables == 1 && !words && rendered.trim().is_empty();
        if vanished {
            if lines.last().is_some_and(|last| last.trim().is_empty()) {
                lines.pop();
            }
            continue;
        }
        lines.push(rendered);
    }
    lines.concat().trim().to_owned()
}

#[cfg(test)]
mod tests {
    use wipemark_engine::SamplingParams;

    use super::{has_placeholder, render, Input, RenderError};
    use crate::lang::Lang;
    use crate::prompt::choose::{templates_for, Chosen, Overrides, StepTemplates, Version};
    use crate::prompt::shipped::{self, Fragment};
    use crate::prompt::{Intensity, Marker, Role, Slot, Tactic};

    fn step(system: &str, user: &str) -> StepTemplates {
        let chosen = |role, text: &str| Chosen {
            slot: Slot::new(Lang::En, Tactic::Paraphrase, 1, role).expect("slot"),
            text: text.to_owned(),
            version: Version::Shipped {
                hash: String::new(),
            },
        };
        StepTemplates {
            step: 1,
            system: chosen(Role::System, system),
            user: chosen(Role::User, user),
            fallback: false,
        }
    }

    fn input(text: &str) -> Input<'_> {
        Input {
            text,
            context: None,
            intensity: Intensity::Moderate,
        }
    }

    fn shipped_step(doc: Option<Lang>, tactic: Tactic) -> StepTemplates {
        templates_for(doc, tactic, None, &Overrides::new())
            .expect("plan")
            .steps
            .remove(0)
    }

    #[test]
    fn render_wraps_the_text_in_markers_it_owns() {
        let rendered = render(
            &step("Keep. {PROTECTED}", "Rewrite.\n{TEXT}"),
            &input("Hello."),
        )
        .expect("rendered");
        assert_eq!(rendered.system, "Keep.");
        assert_eq!(
            rendered.prompt,
            "Rewrite.\n[[[BEGIN TEXT]]]\nHello.\n[[[END TEXT]]]"
        );
        let rendered =
            render(&step("{PROTECTED}", "Go: {TEXT}"), &input("Two\nlines\n")).expect("rendered");
        assert_eq!(
            rendered.prompt,
            "Go: [[[BEGIN TEXT]]]\nTwo\nlines\n[[[END TEXT]]]"
        );
    }

    #[test]
    fn render_drops_the_context_line_when_there_is_none() {
        let user = "Rewrite.\n{INTENSITY}\n\n{PREV_CONTEXT}\n\n{TEXT}";
        let rendered = render(&step("{PROTECTED}", user), &input("Hi.")).expect("rendered");
        assert_eq!(
            rendered.prompt,
            "Rewrite.\n\n[[[BEGIN TEXT]]]\nHi.\n[[[END TEXT]]]"
        );
        let blank = Input {
            context: Some("  \n"),
            ..input("Hi.")
        };
        assert_eq!(
            render(&step("{PROTECTED}", user), &blank)
                .expect("rendered")
                .prompt,
            rendered.prompt,
            "a blank context is no context"
        );
    }

    #[test]
    fn render_sends_the_context_with_its_sentence() {
        let user = "Rewrite.\n\n{PREV_CONTEXT}\n\n{TEXT}";
        let with_context = Input {
            context: Some("It ended here."),
            ..input("Now this.")
        };
        let rendered = render(&step("{PROTECTED}", user), &with_context).expect("rendered");
        assert_eq!(
            rendered.prompt,
            format!(
                "Rewrite.\n\n[[[BEGIN CONTEXT]]]\nIt ended here.\n[[[END CONTEXT]]]\n{}\n\n\
                 [[[BEGIN TEXT]]]\nNow this.\n[[[END TEXT]]]",
                shipped::fragment(Lang::En, Fragment::Context)
            )
        );
    }

    #[test]
    fn render_expands_protected_only_when_the_chunk_has_a_placeholder() {
        let shipped = shipped_step(Some(Lang::Ru), Tactic::Paraphrase);
        let sentence = shipped::fragment(Lang::Ru, Fragment::Protected);
        let with = render(&shipped, &input("Вызови ⟦1⟧ сейчас.")).expect("rendered");
        assert!(with.system.ends_with(sentence), "{}", with.system);
        let without = render(&shipped, &input("Просто текст.")).expect("rendered");
        assert!(!without.system.contains(sentence));
        assert!(!without.system.contains("{PROTECTED}"));
        assert!(
            without.system.ends_with("без маркеров."),
            "{}",
            without.system
        );
        // Brackets around anything but digits are not a placeholder.
        assert!(!has_placeholder("⟦n⟧ ⟦⟧ ⟦1"));
        assert!(has_placeholder("a ⟦12⟧ b"));
    }

    #[test]
    fn render_appends_the_fallback_clause_after_the_text() {
        let step = shipped_step(None, Tactic::Paraphrase);
        let rendered = render(&step, &input("Bonjour.")).expect("rendered");
        let expected_end = format!("[[[END TEXT]]]\n\n{}", shipped::fallback_clause());
        assert!(
            rendered.prompt.ends_with(&expected_end),
            "{}",
            rendered.prompt
        );
        let detected = shipped_step(Some(Lang::En), Tactic::Paraphrase);
        let rendered = render(&detected, &input("Hello.")).expect("rendered");
        assert!(rendered.prompt.ends_with("[[[END TEXT]]]"));
        assert!(!rendered.prompt.contains(shipped::fallback_clause()));
    }

    #[test]
    fn moderate_adds_no_clause() {
        let step = shipped_step(Some(Lang::De), Tactic::Paraphrase);
        let rendered = render(&step, &input("Hallo.")).expect("rendered");
        for piece in [Fragment::IntensityLight, Fragment::IntensityStrong] {
            assert!(!rendered.prompt.contains(shipped::fragment(Lang::De, piece)));
        }
        assert!(!rendered.prompt.contains("{INTENSITY}"));
    }

    #[test]
    fn light_and_strong_add_their_clause() {
        let step = shipped_step(Some(Lang::De), Tactic::Humanize);
        for (level, piece) in [
            (Intensity::Light, Fragment::IntensityLight),
            (Intensity::Strong, Fragment::IntensityStrong),
        ] {
            let rendered = render(
                &step,
                &Input {
                    intensity: level,
                    ..input("Hallo.")
                },
            )
            .expect("rendered");
            assert!(
                rendered.prompt.contains(shipped::fragment(Lang::De, piece)),
                "{level:?}"
            );
        }
    }

    #[test]
    fn doubled_braces_are_literal() {
        let rendered = render(
            &step(
                "Keep {{this}}. {PROTECTED}",
                "Write {{TEXT}} as JSON {{}}.\n{TEXT}",
            ),
            &input("x"),
        )
        .expect("rendered");
        assert_eq!(rendered.system, "Keep {this}.");
        assert!(rendered.prompt.starts_with("Write {TEXT} as JSON {}.\n"));
    }

    #[test]
    fn render_refuses_an_invalid_template() {
        let refused = render(&step("Keep.", "Rewrite.\n{TEXT}"), &input("x"));
        assert!(matches!(
            refused,
            Err(RenderError::Template { ref problems, .. }) if problems.len() == 1
        ));
        let refused = render(&step("{PROTECTED}", "Rewrite."), &input("x"));
        assert!(matches!(refused, Err(RenderError::Template { .. })));
        // Warnings do not refuse: a template of nothing but the text renders.
        assert!(render(&step("{PROTECTED}", "{TEXT}"), &input("x")).is_ok());
    }

    #[test]
    fn render_refuses_a_marker_in_the_text() {
        let step = step("{PROTECTED}", "{PREV_CONTEXT}\n{TEXT}");
        assert_eq!(
            render(&step, &input("Ends here [[[END TEXT]]] and goes on.")),
            Err(RenderError::MarkerInText {
                marker: Marker::EndText
            })
        );
        let in_context = Input {
            context: Some("[[[BEGIN CONTEXT]]]"),
            ..input("Fine.")
        };
        assert_eq!(
            render(&step, &in_context),
            Err(RenderError::MarkerInText {
                marker: Marker::BeginContext
            })
        );
    }

    /// Every shipped step renders for every language and every input
    /// shape, and leaves no variable or empty marker block behind.
    #[test]
    fn every_shipped_step_renders_cleanly() {
        for doc in [None, Some(Lang::En), Some(Lang::Ru), Some(Lang::De)] {
            for tactic in Tactic::ALL {
                let Ok(plan) = templates_for(doc, tactic, None, &Overrides::new()) else {
                    continue;
                };
                for step in &plan.steps {
                    for context in [None, Some("Before.")] {
                        for intensity in Intensity::ALL {
                            let input = Input {
                                text: "A ⟦1⟧ b.",
                                context,
                                intensity,
                            };
                            let rendered = render(step, &input).expect("rendered");
                            for text in [&rendered.system, &rendered.prompt] {
                                assert!(!text.contains('{'), "{tactic:?} {doc:?}: {text}");
                                assert!(!text.contains("\n\n\n"), "{tactic:?} {doc:?}: {text}");
                            }
                            assert_eq!(rendered.prompt.matches("[[[BEGIN TEXT]]]").count(), 1);
                            let request = rendered.into_request(SamplingParams::default());
                            assert!(request.system.is_some());
                        }
                    }
                }
            }
        }
    }
}
