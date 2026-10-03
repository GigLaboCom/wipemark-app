//! Adapting a template into another language (D74, Q-B22).
//!
//! A template is written in one language and *adapted* into the others,
//! not translated word for word: the `humanize` cliché list, the
//! typography (a dash and «ёлочки» are normal in Russian, „…“ in German)
//! and the examples change with the language. A user does it by hand, or
//! presses a button that asks the model on duty — never automatically,
//! because an automatic adaptation is an unseen request to a rewriter
//! that may not be on this machine, and a template nobody read.
//!
//! [`adaptation_request`] is that request, written in the language being
//! adapted *into*; [`check_adaptation`] is what the answer must pass before
//! it is saved: everything [`validate`] asks of any template, and exactly
//! the source's variables — the main risk of a machine adaptation is
//! `{TEXT}` coming back as `{ТЕКСТ}`, or `{PROTECTED}` not coming back.
//! Run the model's raw answer through [`super::clean_response`] first.

use std::collections::BTreeMap;

use super::render::Rendered;
use super::shipped::{self, VARIABLES_TOKEN};
use super::validate::{validate, Problem, ValidationContext};
use super::{template, Marker, Slot, Variable};
use crate::lang::Lang;

/// Why no adaptation request was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdaptRefusal {
    /// The template is already in that language.
    SameLanguage,
    /// The product has no such slot in that language (`code` is English
    /// only).
    NoSuchSlot,
    /// The source contains a marker string, which would end the material
    /// early. A saved template never does (R5); this is the second lock.
    MarkerInSource { marker: Marker },
}

/// The request that asks a model to adapt `source_text` — the template of
/// `source` — into `to`.
pub fn adaptation_request(
    source: Slot,
    source_text: &str,
    to: Lang,
) -> Result<Rendered, AdaptRefusal> {
    if to == source.lang() {
        return Err(AdaptRefusal::SameLanguage);
    }
    source.with_lang(to).ok_or(AdaptRefusal::NoSuchSlot)?;
    if let Some(marker) = Marker::ALL
        .into_iter()
        .find(|marker| source_text.contains(marker.as_str()))
    {
        return Err(AdaptRefusal::MarkerInSource { marker });
    }
    let meta = shipped::adapt_prompt(to);
    let mut names: Vec<String> = Vec::new();
    for (variable, _) in template::parse(source_text).variables() {
        let name = format!("{{{}}}", variable.name());
        if !names.contains(&name) {
            names.push(name);
        }
    }
    let mut prompt = meta.user.to_owned();
    if !names.is_empty() {
        prompt.push('\n');
        prompt.push_str(&meta.variables.replace(VARIABLES_TOKEN, &names.join(", ")));
    }
    let newline = if source_text.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    prompt.push_str(&format!(
        "\n\n{}\n{source_text}{newline}{}",
        Marker::BeginText.as_str(),
        Marker::EndText.as_str()
    ));
    Ok(Rendered {
        system: meta.system.to_owned(),
        prompt,
    })
}

/// What is wrong with `candidate` as the adaptation of `source_text` into
/// `target`: every problem [`validate`] finds, and
/// [`Problem::VariablesDiffer`] unless the two hold the same variables the
/// same number of times.
pub fn check_adaptation(
    target: Slot,
    source_text: &str,
    candidate: &str,
    context: &ValidationContext<'_>,
) -> Vec<Problem> {
    let mut problems = validate(target, candidate, context);
    let source = counts(source_text);
    let adapted = counts(candidate);
    let mut missing = Vec::new();
    let mut extra = Vec::new();
    for variable in Variable::ALL {
        let had = source.get(&variable).copied().unwrap_or(0);
        let has = adapted.get(&variable).copied().unwrap_or(0);
        missing.extend(std::iter::repeat_n(variable, had.saturating_sub(has)));
        extra.extend(std::iter::repeat_n(variable, has.saturating_sub(had)));
    }
    if !missing.is_empty() || !extra.is_empty() {
        problems.push(Problem::VariablesDiffer { missing, extra });
    }
    problems
}

fn counts(text: &str) -> BTreeMap<Variable, usize> {
    let mut counts = BTreeMap::new();
    for (variable, _) in template::parse(text).variables() {
        *counts.entry(variable).or_insert(0) += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::{adaptation_request, check_adaptation, AdaptRefusal};
    use crate::lang::Lang;
    use crate::prompt::validate::{Problem, ValidationContext};
    use crate::prompt::{shipped, Marker, Role, Slot, Tactic, Variable};

    fn slot(lang: Lang, role: Role) -> Slot {
        Slot::new(lang, Tactic::Paraphrase, 1, role).expect("slot")
    }

    const SOURCE: &str = "Rewrite the text.\n{INTENSITY}\n\n{PREV_CONTEXT}\n\n{TEXT}";

    #[test]
    fn an_adaptation_request_is_in_the_target_language_and_lists_the_variables() {
        let request =
            adaptation_request(slot(Lang::En, Role::User), SOURCE, Lang::Ru).expect("request");
        let meta = shipped::adapt_prompt(Lang::Ru);
        assert_eq!(request.system, meta.system);
        assert!(request.prompt.starts_with(meta.user));
        assert!(request
            .prompt
            .contains("{INTENSITY}, {PREV_CONTEXT}, {TEXT}."));
        assert!(request
            .prompt
            .ends_with(&format!("[[[BEGIN TEXT]]]\n{SOURCE}\n[[[END TEXT]]]")));
        assert!(request.system.contains("русск"));
        // A source with no variables gets no list.
        let request = adaptation_request(slot(Lang::En, Role::System), "Keep it.", Lang::De)
            .expect("request");
        assert!(!request.prompt.contains("@VARIABLES@"));
        assert!(!request.prompt.contains(": ."));
    }

    #[test]
    fn adapting_into_the_same_language_is_refused() {
        assert_eq!(
            adaptation_request(slot(Lang::De, Role::User), SOURCE, Lang::De),
            Err(AdaptRefusal::SameLanguage)
        );
        let code = Slot::new(Lang::En, Tactic::Code, 1, Role::User).expect("slot");
        assert_eq!(
            adaptation_request(code, "{TEXT}", Lang::Ru),
            Err(AdaptRefusal::NoSuchSlot)
        );
        assert_eq!(
            adaptation_request(
                slot(Lang::En, Role::User),
                "[[[END TEXT]]] {TEXT}",
                Lang::Ru
            ),
            Err(AdaptRefusal::MarkerInSource {
                marker: Marker::EndText
            })
        );
    }

    fn check(candidate: &str) -> Vec<Problem> {
        let target = slot(Lang::Ru, Role::User);
        let context = ValidationContext::beside_shipped(target);
        check_adaptation(target, SOURCE, candidate, &context)
    }

    #[test]
    fn a_faithful_adaptation_passes() {
        assert_eq!(
            check("Перепиши текст.\n{INTENSITY}\n\n{PREV_CONTEXT}\n\n{TEXT}"),
            vec![]
        );
    }

    #[test]
    fn an_adaptation_that_translates_a_variable_is_refused() {
        let problems = check("Перепиши текст.\n{INTENSITY}\n\n{PREV_CONTEXT}\n\n{ТЕКСТ}");
        assert!(problems.contains(&Problem::VariablesDiffer {
            missing: vec![Variable::Text],
            extra: vec![],
        }));
        assert!(problems
            .iter()
            .any(|problem| problem.rule() == "unknown-variable"));
    }

    #[test]
    fn an_adaptation_that_drops_protected_is_refused() {
        let target = slot(Lang::Ru, Role::System);
        let source = shipped::template(slot(Lang::En, Role::System)).expect("shipped");
        let candidate = "Ты переписываешь текст. Сохраняй всё.";
        let context = ValidationContext {
            // The other turn carries the rule, so validation alone would
            // pass this: only the variable-set check catches the loss.
            other_role: "Перепиши. {PROTECTED}\n{TEXT}",
            ..ValidationContext::beside_shipped(target)
        };
        assert_eq!(
            check_adaptation(target, source, candidate, &context),
            vec![Problem::VariablesDiffer {
                missing: vec![Variable::Protected],
                extra: vec![],
            }]
        );
    }

    #[test]
    fn an_adaptation_with_a_variable_too_many_is_refused() {
        let problems = check("Перепиши.\n{INTENSITY}\n{INTENSITY}\n\n{PREV_CONTEXT}\n\n{TEXT}");
        assert_eq!(
            problems,
            vec![Problem::VariablesDiffer {
                missing: vec![],
                extra: vec![Variable::Intensity],
            }]
        );
    }
}
