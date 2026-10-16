//! The templates and sentences the product ships, compiled in from
//! `crates/wipemark-pipeline/prompts/` (D64, D66, D73).
//!
//! One file per [`Slot`], named the way the slot's row is keyed —
//! `prompts/<lang>/<tactic>.<step>.<role>.txt` — so the file list *is* the
//! slot list. Where several slots are meant to carry the same contract
//! (`paraphrase` and `humanize`; both steps of `back_translate`; both steps
//! of `structural`) each still has its own file, and
//! `contracts_meant_to_be_identical_are_identical` keeps them identical:
//! a user overrides one slot at a time, and the shipped side should look
//! the same.
//!
//! The *fragments* are not slots. The placeholder sentence, the context
//! sentence, the two intensity clauses and the English fallback clause
//! belong to the assembler: they are what a variable expands to, in the
//! template's own language, and v1 does not offer them for editing
//! (`docs/plan/E4-2-the-prompts.md` §4.2).
//!
//! A shipped text is its file with trailing whitespace trimmed.

use super::{Intensity, Slot};
use crate::lang::Lang;

/// One table row per shipped slot. The macro writes the name and the path
/// from one literal, so a row cannot name one slot and read another's
/// file.
macro_rules! file {
    ($name:literal) => {
        (
            $name,
            include_str!(concat!("../../prompts/", $name, ".txt")),
        )
    };
}

const TEMPLATES: &[(&str, &str)] = &[
    file!("en/paraphrase.1.system"),
    file!("en/paraphrase.1.user"),
    file!("en/humanize.1.system"),
    file!("en/humanize.1.user"),
    file!("en/back_translate.1.system"),
    file!("en/back_translate.1.user"),
    file!("en/back_translate.2.system"),
    file!("en/back_translate.2.user"),
    file!("en/structural.1.system"),
    file!("en/structural.1.user"),
    file!("en/structural.2.system"),
    file!("en/structural.2.user"),
    file!("en/code.1.system"),
    file!("en/code.1.user"),
    file!("ru/paraphrase.1.system"),
    file!("ru/paraphrase.1.user"),
    file!("ru/humanize.1.system"),
    file!("ru/humanize.1.user"),
    file!("ru/back_translate.1.system"),
    file!("ru/back_translate.1.user"),
    file!("ru/back_translate.2.system"),
    file!("ru/back_translate.2.user"),
    file!("ru/structural.1.system"),
    file!("ru/structural.1.user"),
    file!("ru/structural.2.system"),
    file!("ru/structural.2.user"),
    file!("de/paraphrase.1.system"),
    file!("de/paraphrase.1.user"),
    file!("de/humanize.1.system"),
    file!("de/humanize.1.user"),
    file!("de/back_translate.1.system"),
    file!("de/back_translate.1.user"),
    file!("de/back_translate.2.system"),
    file!("de/back_translate.2.user"),
    file!("de/structural.1.system"),
    file!("de/structural.1.user"),
    file!("de/structural.2.system"),
    file!("de/structural.2.user"),
];

/// The file name of a slot, without the directory or the extension.
fn name_of(slot: Slot) -> String {
    format!(
        "{}/{}.{}.{}",
        slot.lang().as_str(),
        slot.tactic().as_str(),
        slot.step(),
        slot.role().as_str()
    )
}

/// The shipped template of a slot.
///
/// `None` only for a slot the table lacks, which
/// `every_language_has_a_complete_shipped_set` turns into a red suite.
pub fn template(slot: Slot) -> Option<&'static str> {
    let name = name_of(slot);
    TEMPLATES
        .iter()
        .find(|(file, _)| *file == name)
        .map(|(_, text)| text.trim_end())
}

/// A sentence the assembler writes, in one language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fragment {
    /// What `{PROTECTED}` expands to when the chunk holds a placeholder.
    Protected,
    /// What follows the context block: do not rewrite it, do not repeat it.
    Context,
    /// What `{INTENSITY}` expands to for [`Intensity::Light`].
    IntensityLight,
    /// What `{INTENSITY}` expands to for [`Intensity::Strong`].
    IntensityStrong,
}

impl Fragment {
    /// The clause for an intensity; none for [`Intensity::Moderate`]
    /// (D73).
    pub fn of_intensity(level: Intensity) -> Option<Fragment> {
        match level {
            Intensity::Light => Some(Fragment::IntensityLight),
            Intensity::Moderate => None,
            Intensity::Strong => Some(Fragment::IntensityStrong),
        }
    }
}

/// A fragment in a language.
pub fn fragment(lang: Lang, fragment: Fragment) -> &'static str {
    let text = match (lang, fragment) {
        (Lang::En, Fragment::Protected) => include_str!("../../prompts/en/fragment.protected.txt"),
        (Lang::En, Fragment::Context) => include_str!("../../prompts/en/fragment.context.txt"),
        (Lang::En, Fragment::IntensityLight) => {
            include_str!("../../prompts/en/fragment.intensity-light.txt")
        }
        (Lang::En, Fragment::IntensityStrong) => {
            include_str!("../../prompts/en/fragment.intensity-strong.txt")
        }
        (Lang::Ru, Fragment::Protected) => include_str!("../../prompts/ru/fragment.protected.txt"),
        (Lang::Ru, Fragment::Context) => include_str!("../../prompts/ru/fragment.context.txt"),
        (Lang::Ru, Fragment::IntensityLight) => {
            include_str!("../../prompts/ru/fragment.intensity-light.txt")
        }
        (Lang::Ru, Fragment::IntensityStrong) => {
            include_str!("../../prompts/ru/fragment.intensity-strong.txt")
        }
        (Lang::De, Fragment::Protected) => include_str!("../../prompts/de/fragment.protected.txt"),
        (Lang::De, Fragment::Context) => include_str!("../../prompts/de/fragment.context.txt"),
        (Lang::De, Fragment::IntensityLight) => {
            include_str!("../../prompts/de/fragment.intensity-light.txt")
        }
        (Lang::De, Fragment::IntensityStrong) => {
            include_str!("../../prompts/de/fragment.intensity-strong.txt")
        }
    };
    text.trim_end()
}

/// The clause appended to the English set when a document's language was
/// not detected (D64): answer in the language of the text, do not
/// translate it. English only, because only the English set is ever the
/// fallback.
pub fn fallback_clause() -> &'static str {
    include_str!("../../prompts/en/fragment.fallback.txt").trim_end()
}

/// The three parts of the adaptation meta-prompt, in the language a
/// template is adapted *into* (D74).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdaptPrompt {
    /// The meta-contract.
    pub system: &'static str,
    /// The request.
    pub user: &'static str,
    /// The sentence listing the source's variables, with `@VARIABLES@`
    /// where the list goes. Left out when the source has none.
    pub variables: &'static str,
}

/// The token in [`AdaptPrompt::variables`] that the list replaces.
pub const VARIABLES_TOKEN: &str = "@VARIABLES@";

/// The adaptation meta-prompt in a language.
pub fn adapt_prompt(lang: Lang) -> AdaptPrompt {
    let (system, user, variables) = match lang {
        Lang::En => (
            include_str!("../../prompts/en/adapt.system.txt"),
            include_str!("../../prompts/en/adapt.user.txt"),
            include_str!("../../prompts/en/adapt.variables.txt"),
        ),
        Lang::Ru => (
            include_str!("../../prompts/ru/adapt.system.txt"),
            include_str!("../../prompts/ru/adapt.user.txt"),
            include_str!("../../prompts/ru/adapt.variables.txt"),
        ),
        Lang::De => (
            include_str!("../../prompts/de/adapt.system.txt"),
            include_str!("../../prompts/de/adapt.user.txt"),
            include_str!("../../prompts/de/adapt.variables.txt"),
        ),
    };
    AdaptPrompt {
        system: system.trim_end(),
        user: user.trim_end(),
        variables: variables.trim_end(),
    }
}

#[cfg(test)]
mod tests {
    use super::{adapt_prompt, fallback_clause, fragment, template, Fragment, TEMPLATES};
    use crate::lang::Lang;
    use crate::prompt::validate::{validate, Severity, ValidationContext};
    use crate::prompt::{Intensity, Marker, Role, Slot, Tactic, Variable};

    /// The window every shipped template must fit a tenth of: the
    /// catalogue's `ctx_default` for both shipped models.
    const CATALOGUE_CTX: u32 = 8192;

    /// D64's gate: a language the product has templates for has *all* of
    /// them, and every one of them is a template this build would accept
    /// from a user — no error, at any intensity, within a tenth of the
    /// catalogue's window.
    #[test]
    fn every_language_has_a_complete_shipped_set() {
        for lang in Lang::ALL {
            for tactic in Tactic::ALL {
                if tactic == Tactic::Code && lang != Lang::En {
                    continue;
                }
                for step in 1..=tactic.steps() {
                    for role in Role::ALL {
                        let slot = Slot::new(lang, tactic, step, role)
                            .unwrap_or_else(|| panic!("{lang:?} {tactic:?} {step} {role:?}"));
                        let text = template(slot)
                            .unwrap_or_else(|| panic!("no shipped template for {slot:?}"));
                        let other = template(slot.other_role())
                            .unwrap_or_else(|| panic!("no shipped template beside {slot:?}"));
                        for intensity in Intensity::ALL {
                            let context = ValidationContext {
                                other_role: other,
                                ctx_len: Some(CATALOGUE_CTX),
                                intensity,
                                based_on: None,
                            };
                            let problems = validate(slot, text, &context);
                            assert!(
                                problems.is_empty(),
                                "{slot:?} at {intensity:?}: {problems:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    /// The table and [`Slot::all`] describe the same set: nothing shipped
    /// that no slot can reach, and no slot without a file.
    #[test]
    fn slots_are_exactly_the_shipped_set() {
        let slots = Slot::all();
        assert_eq!(slots.len(), TEMPLATES.len());
        for slot in slots {
            assert!(template(slot).is_some(), "{slot:?}");
        }
    }

    #[test]
    fn contracts_meant_to_be_identical_are_identical() {
        let pairs = [
            ((Tactic::Paraphrase, 1), (Tactic::Humanize, 1)),
            ((Tactic::BackTranslate, 1), (Tactic::BackTranslate, 2)),
            ((Tactic::Structural, 1), (Tactic::Structural, 2)),
        ];
        for lang in Lang::ALL {
            for ((a_tactic, a_step), (b_tactic, b_step)) in pairs {
                let a = Slot::new(lang, a_tactic, a_step, Role::System).expect("slot");
                let b = Slot::new(lang, b_tactic, b_step, Role::System).expect("slot");
                assert_eq!(template(a), template(b), "{a:?} and {b:?} drifted apart");
            }
        }
    }

    /// The English set is also the fallback for a document whose language
    /// was not detected. A contract that said "write in English" would
    /// argue with the clause appended for that case, and the clause loses
    /// that argument as often as not.
    #[test]
    fn the_english_contract_does_not_say_english() {
        for tactic in [Tactic::Paraphrase, Tactic::Humanize, Tactic::Structural] {
            for step in 1..=tactic.steps() {
                for role in Role::ALL {
                    let slot = Slot::new(Lang::En, tactic, step, role).expect("slot");
                    let text = template(slot).expect("shipped");
                    assert!(!text.contains("English"), "{slot:?} names English");
                }
            }
        }
    }

    /// A Russian or German set names its own language in its own words;
    /// the translation contracts name the language written *into*, which
    /// is the set's own (D64).
    #[test]
    fn a_contract_names_only_its_own_language() {
        let own = |lang| match lang {
            Lang::En => "English",
            Lang::Ru => "русск",
            Lang::De => "Deutsch",
        };
        let others = |lang| {
            Lang::ALL
                .into_iter()
                .filter(move |other| *other != lang)
                .map(own)
        };
        for slot in Slot::all() {
            let text = template(slot).expect("shipped");
            for name in others(slot.lang()) {
                assert!(!text.contains(name), "{slot:?} names {name}");
            }
        }
        for lang in [Lang::Ru, Lang::De] {
            let contract = Slot::new(lang, Tactic::Paraphrase, 1, Role::System).expect("slot");
            assert!(template(contract).expect("shipped").contains(own(lang)));
        }
        let translate = Slot::new(Lang::En, Tactic::BackTranslate, 1, Role::System).expect("slot");
        assert!(template(translate).expect("shipped").contains("English"));
    }

    /// Every system template carries the placeholder rule, so a chunk
    /// with code in it is never sent without it whatever the user turn
    /// says.
    #[test]
    fn every_shipped_contract_carries_the_placeholder_rule() {
        for slot in Slot::all() {
            if slot.role() == Role::System {
                let name = format!("{{{}}}", Variable::Protected.name());
                assert!(template(slot).expect("shipped").contains(&name), "{slot:?}");
            }
        }
    }

    #[test]
    fn fragments_are_sentences_without_variables_or_markers() {
        let mut all: Vec<&str> = vec![fallback_clause()];
        for lang in Lang::ALL {
            for piece in [
                Fragment::Protected,
                Fragment::Context,
                Fragment::IntensityLight,
                Fragment::IntensityStrong,
            ] {
                all.push(fragment(lang, piece));
            }
            let adapt = adapt_prompt(lang);
            all.extend([adapt.system, adapt.user]);
        }
        for text in all {
            assert!(!text.trim().is_empty());
            assert!(!text.contains('{') && !text.contains('}'), "{text}");
            for marker in Marker::ALL {
                assert!(!text.contains(marker.as_str()), "{text}");
            }
        }
        for lang in Lang::ALL {
            assert!(fragment(lang, Fragment::Protected).contains("⟦n⟧"));
            assert!(adapt_prompt(lang)
                .variables
                .contains(super::VARIABLES_TOKEN));
        }
        assert!(fallback_clause().contains("do not translate"));
    }

    #[test]
    fn moderate_has_no_clause() {
        assert_eq!(Fragment::of_intensity(Intensity::Moderate), None);
        assert_eq!(
            Fragment::of_intensity(Intensity::Light),
            Some(Fragment::IntensityLight)
        );
        assert_eq!(
            Fragment::of_intensity(Intensity::Strong),
            Some(Fragment::IntensityStrong)
        );
    }

    /// The validator's warning, applied to the shipped set as a test: a
    /// shipped template in the wrong script would be our own bug.
    #[test]
    fn no_shipped_template_warns() {
        for slot in Slot::all() {
            let context = ValidationContext {
                other_role: template(slot.other_role()).expect("shipped"),
                ctx_len: None,
                intensity: Intensity::Strong,
                based_on: None,
            };
            let warnings: Vec<_> = validate(slot, template(slot).expect("shipped"), &context)
                .into_iter()
                .filter(|problem| problem.severity() == Severity::Warning)
                .collect();
            assert!(warnings.is_empty(), "{slot:?}: {warnings:?}");
        }
    }
}
