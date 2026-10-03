//! Which templates a document gets (D60, D64; notes §3a "which template is
//! taken for a document in language L").
//!
//! In order: the user's override for the language the step is written in;
//! the shipped template for that language; and, for a document whose
//! language was not detected, the English set — override or shipped — with
//! the clause "answer in the language of the text; do not translate it"
//! appended by the assembler. `back_translate` is not offered for such a
//! document at all: its second step would have no language to return to.
//!
//! A step's prompt is in the language of the text that step must produce
//! (D64): a one-step tactic in the document's; `back_translate`'s first
//! step in the pivot's, its second in the document's; `code` in English
//! for every document (D73), with no clause — its template keeps the
//! comments' language itself.

use std::collections::BTreeMap;

use super::row::{hash, Origin, Override};
use super::{shipped, Role, Slot, Tactic};
use crate::lang::Lang;

/// The pivot of `back_translate` for a document (D60): Russian goes
/// through German, English through Russian, German through English. The
/// `rewrite.pivot` row (E4-6) wins — unless it names the document's own
/// language, because English into English is not a translation; a row
/// this build cannot use for this document reads as the default, and the
/// row is left alone.
pub fn pivot_for(doc: Lang, row: Option<Lang>) -> Lang {
    match row {
        Some(pivot) if pivot != doc => pivot,
        _ => match doc {
            Lang::Ru => Lang::De,
            Lang::En => Lang::Ru,
            Lang::De => Lang::En,
        },
    }
}

/// The user's overrides, as the caller read them from the rows. Only rows
/// that parsed belong here; a row that did not is left in the database
/// and its slot uses the shipped template.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Overrides {
    rows: BTreeMap<Slot, Override>,
}

impl Overrides {
    pub fn new() -> Overrides {
        Overrides::default()
    }

    /// Set a slot's override, handing back the one it replaces.
    pub fn insert(&mut self, slot: Slot, row: Override) -> Option<Override> {
        self.rows.insert(slot, row)
    }

    pub fn get(&self, slot: Slot) -> Option<&Override> {
        self.rows.get(&slot)
    }

    pub fn remove(&mut self, slot: Slot) -> Option<Override> {
        self.rows.remove(&slot)
    }

    /// The text a slot will be rendered from: its override's, else the
    /// shipped one. `None` only for a slot the shipped table lacks, which
    /// the complete-set gate does not let happen.
    pub fn effective(&self, slot: Slot) -> Option<&str> {
        match self.rows.get(&slot) {
            Some(row) => Some(row.text.as_str()),
            None => shipped::template(slot),
        }
    }
}

/// Why no templates were chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Refusal {
    /// `back_translate` over a document whose language was not detected:
    /// the second step would have no language to translate back into
    /// (D64).
    BackTranslateNeedsLanguage,
}

/// Whether an override has fallen behind what it was made from. A stale
/// override is still used — it is the user's — and the report says it
/// was stale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Stale {
    /// The shipped template of this slot changed after the override was
    /// made from it (`based_on`).
    pub shipped_changed: bool,
    /// The template this one was adapted from changed after the adaptation
    /// (`adapted_from.hash`).
    pub source_changed: bool,
}

impl Stale {
    pub fn any(self) -> bool {
        self.shipped_changed || self.source_changed
    }
}

/// Which version of a template ran — what the report records for every
/// attempt, without which a seed reproduces nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Version {
    /// The shipped template, by the hash of its text.
    Shipped { hash: String },
    /// The user's override, by the hash of its text.
    Override {
        hash: String,
        origin: Origin,
        stale: Stale,
    },
}

/// One template chosen for one turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chosen {
    pub slot: Slot,
    pub text: String,
    pub version: Version,
}

/// The two templates of one step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepTemplates {
    /// 1 or 2.
    pub step: u8,
    pub system: Chosen,
    pub user: Chosen,
    /// The English set standing in for an undetected language: the
    /// assembler appends the fallback clause (D64).
    pub fallback: bool,
}

/// Everything one tactic needs for one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub tactic: Tactic,
    /// One entry per step, in order.
    pub steps: Vec<StepTemplates>,
    /// The pivot, for `back_translate`.
    pub pivot: Option<Lang>,
}

/// The templates for `tactic` over a document in `doc` (`None`: not
/// detected), with the `rewrite.pivot` row and the user's overrides.
pub fn templates_for(
    doc: Option<Lang>,
    tactic: Tactic,
    pivot_row: Option<Lang>,
    overrides: &Overrides,
) -> Result<Plan, Refusal> {
    let pivot = match (tactic, doc) {
        (Tactic::BackTranslate, None) => return Err(Refusal::BackTranslateNeedsLanguage),
        (Tactic::BackTranslate, Some(doc)) => Some(pivot_for(doc, pivot_row)),
        _ => None,
    };
    let fallback = doc.is_none() && tactic != Tactic::Code;
    let steps = (1..=tactic.steps())
        .map(|step| {
            let lang = match (tactic, doc, pivot) {
                (Tactic::Code, _, _) | (_, None, _) => Lang::En,
                (Tactic::BackTranslate, Some(_), Some(pivot)) if step == 1 => pivot,
                (_, Some(doc), _) => doc,
            };
            let slot = |role| {
                Slot::new(lang, tactic, step, role).expect("a step within the tactic's steps")
            };
            StepTemplates {
                step,
                system: chosen(slot(Role::System), overrides),
                user: chosen(slot(Role::User), overrides),
                fallback,
            }
        })
        .collect();
    Ok(Plan {
        tactic,
        steps,
        pivot,
    })
}

fn chosen(slot: Slot, overrides: &Overrides) -> Chosen {
    match overrides.get(slot) {
        Some(row) => Chosen {
            slot,
            text: row.text.clone(),
            version: Version::Override {
                hash: hash(&row.text),
                origin: row.origin,
                stale: staleness(slot, row, overrides),
            },
        },
        None => {
            let text = shipped::template(slot)
                .expect("every slot is shipped: every_language_has_a_complete_shipped_set");
            Chosen {
                slot,
                text: text.to_owned(),
                version: Version::Shipped { hash: hash(text) },
            }
        }
    }
}

/// How far an override has fallen behind: its shipped template, and —
/// for an adaptation — the source it was adapted from, as that source is
/// now (the source's own override, or its shipped text). A source slot
/// that does not exist cannot be confirmed and counts as changed.
pub fn staleness(slot: Slot, row: &Override, overrides: &Overrides) -> Stale {
    let shipped_changed = shipped::template(slot).map(hash).as_deref() != Some(&row.based_on);
    let source_changed = row.adapted_from.as_ref().is_some_and(|source| {
        let now = slot
            .with_lang(source.lang)
            .and_then(|source_slot| overrides.effective(source_slot))
            .map(hash);
        now.as_deref() != Some(&source.hash)
    });
    Stale {
        shipped_changed,
        source_changed,
    }
}

#[cfg(test)]
mod tests {
    use super::{pivot_for, templates_for, Overrides, Refusal, Stale, Version};
    use crate::lang::Lang;
    use crate::prompt::row::{hash, AdaptedFrom, Origin, Override};
    use crate::prompt::{shipped, Role, Slot, Tactic};

    fn slot(lang: Lang, tactic: Tactic, step: u8, role: Role) -> Slot {
        Slot::new(lang, tactic, step, role).expect("slot")
    }

    fn hand(slot: Slot, text: &str) -> Override {
        Override {
            text: text.to_owned(),
            based_on: hash(shipped::template(slot).expect("shipped")),
            adapted_from: None,
            origin: Origin::Hand,
        }
    }

    #[test]
    fn the_pivot_follows_d60() {
        assert_eq!(pivot_for(Lang::Ru, None), Lang::De);
        assert_eq!(pivot_for(Lang::En, None), Lang::Ru);
        assert_eq!(pivot_for(Lang::De, None), Lang::En);
    }

    #[test]
    fn a_pivot_row_wins_unless_it_is_the_document_language() {
        assert_eq!(pivot_for(Lang::Ru, Some(Lang::En)), Lang::En);
        assert_eq!(pivot_for(Lang::En, Some(Lang::De)), Lang::De);
        assert_eq!(pivot_for(Lang::En, Some(Lang::En)), Lang::Ru);
        assert_eq!(pivot_for(Lang::Ru, Some(Lang::Ru)), Lang::De);
    }

    #[test]
    fn a_detected_language_gets_its_own_shipped_set() {
        for lang in Lang::ALL {
            let plan = templates_for(Some(lang), Tactic::Paraphrase, None, &Overrides::new())
                .expect("plan");
            assert_eq!(plan.steps.len(), 1);
            let step = &plan.steps[0];
            assert!(!step.fallback);
            assert_eq!(step.system.slot.lang(), lang);
            assert_eq!(
                step.user.text,
                shipped::template(step.user.slot).expect("shipped")
            );
            assert_eq!(
                step.user.version,
                Version::Shipped {
                    hash: hash(&step.user.text)
                }
            );
        }
    }

    #[test]
    fn an_override_beats_the_shipped_template_for_its_language_only() {
        let ru_user = slot(Lang::Ru, Tactic::Paraphrase, 1, Role::User);
        let mut overrides = Overrides::new();
        overrides.insert(ru_user, hand(ru_user, "Перепиши своими словами.\n{TEXT}"));

        let ru = templates_for(Some(Lang::Ru), Tactic::Paraphrase, None, &overrides).expect("ru");
        assert_eq!(ru.steps[0].user.text, "Перепиши своими словами.\n{TEXT}");
        assert!(matches!(
            ru.steps[0].user.version,
            Version::Override {
                origin: Origin::Hand,
                ..
            }
        ));
        assert!(matches!(
            ru.steps[0].system.version,
            Version::Shipped { .. }
        ));

        let de = templates_for(Some(Lang::De), Tactic::Paraphrase, None, &overrides).expect("de");
        assert!(matches!(de.steps[0].user.version, Version::Shipped { .. }));
        let none = templates_for(None, Tactic::Paraphrase, None, &overrides).expect("none");
        assert!(matches!(
            none.steps[0].user.version,
            Version::Shipped { .. }
        ));
    }

    #[test]
    fn an_unknown_language_gets_the_english_set_and_the_clause() {
        let en_user = slot(Lang::En, Tactic::Humanize, 1, Role::User);
        let mut overrides = Overrides::new();
        overrides.insert(en_user, hand(en_user, "Make it human.\n{TEXT}"));
        let plan = templates_for(None, Tactic::Humanize, None, &overrides).expect("plan");
        let step = &plan.steps[0];
        assert!(step.fallback);
        assert_eq!(step.system.slot.lang(), Lang::En);
        assert_eq!(
            step.user.text, "Make it human.\n{TEXT}",
            "the English override counts"
        );
        for tactic in [Tactic::Paraphrase, Tactic::Structural] {
            let plan = templates_for(None, tactic, None, &Overrides::new()).expect("plan");
            assert!(plan.steps.iter().all(|step| step.fallback));
            assert!(plan
                .steps
                .iter()
                .all(|step| step.user.slot.lang() == Lang::En));
        }
    }

    #[test]
    fn back_translate_needs_a_language() {
        assert_eq!(
            templates_for(
                None,
                Tactic::BackTranslate,
                Some(Lang::De),
                &Overrides::new()
            ),
            Err(Refusal::BackTranslateNeedsLanguage)
        );
    }

    #[test]
    fn back_translate_step_one_is_in_the_pivot_set() {
        for (doc, pivot) in [
            (Lang::Ru, Lang::De),
            (Lang::En, Lang::Ru),
            (Lang::De, Lang::En),
        ] {
            let plan = templates_for(Some(doc), Tactic::BackTranslate, None, &Overrides::new())
                .expect("plan");
            assert_eq!(plan.pivot, Some(pivot));
            assert_eq!(plan.steps.len(), 2);
            assert_eq!(plan.steps[0].system.slot.lang(), pivot);
            assert_eq!(plan.steps[0].user.slot.lang(), pivot);
            assert_eq!(plan.steps[0].user.slot.step(), 1);
            assert_eq!(plan.steps[1].system.slot.lang(), doc);
            assert_eq!(plan.steps[1].user.slot.lang(), doc);
            assert_eq!(plan.steps[1].user.slot.step(), 2);
            assert!(plan.steps.iter().all(|step| !step.fallback));
        }
        let plan = templates_for(
            Some(Lang::Ru),
            Tactic::BackTranslate,
            Some(Lang::En),
            &Overrides::new(),
        )
        .expect("plan");
        assert_eq!(plan.steps[0].user.slot.lang(), Lang::En, "the row's pivot");
    }

    #[test]
    fn code_is_english_for_every_document_without_the_clause() {
        for doc in [None, Some(Lang::Ru), Some(Lang::De), Some(Lang::En)] {
            let plan = templates_for(doc, Tactic::Code, None, &Overrides::new()).expect("plan");
            assert_eq!(plan.steps.len(), 1);
            assert_eq!(plan.steps[0].user.slot.lang(), Lang::En);
            assert!(!plan.steps[0].fallback, "{doc:?}");
        }
    }

    #[test]
    fn a_stale_override_is_reported_and_still_used() {
        let en_user = slot(Lang::En, Tactic::Paraphrase, 1, Role::User);
        let mut overrides = Overrides::new();
        overrides.insert(en_user, hand(en_user, "Mine.\n{TEXT}"));
        let fresh = templates_for(Some(Lang::En), Tactic::Paraphrase, None, &overrides).expect("p");
        assert!(matches!(
            fresh.steps[0].user.version,
            Version::Override {
                stale: Stale {
                    shipped_changed: false,
                    source_changed: false
                },
                ..
            }
        ));
        let mut old = hand(en_user, "Mine.\n{TEXT}");
        old.based_on = hash("what shipped last year");
        overrides.insert(en_user, old);
        let plan = templates_for(Some(Lang::En), Tactic::Paraphrase, None, &overrides).expect("p");
        assert_eq!(plan.steps[0].user.text, "Mine.\n{TEXT}");
        let Version::Override { stale, .. } = plan.steps[0].user.version else {
            panic!("an override");
        };
        assert!(stale.shipped_changed && !stale.source_changed && stale.any());
    }

    #[test]
    fn a_stale_adaptation_is_reported() {
        let en_user = slot(Lang::En, Tactic::Paraphrase, 1, Role::User);
        let de_user = slot(Lang::De, Tactic::Paraphrase, 1, Role::User);
        let mut overrides = Overrides::new();
        let source = "Rewrite it my way.\n{TEXT}";
        overrides.insert(en_user, hand(en_user, source));
        let adaptation = Override {
            adapted_from: Some(AdaptedFrom {
                lang: Lang::En,
                hash: hash(source),
            }),
            origin: Origin::Machine,
            ..hand(de_user, "Überarbeite es auf meine Art.\n{TEXT}")
        };
        overrides.insert(de_user, adaptation);
        let stale_of = |overrides: &Overrides| {
            let plan =
                templates_for(Some(Lang::De), Tactic::Paraphrase, None, overrides).expect("plan");
            match plan.steps[0].user.version {
                Version::Override { stale, .. } => stale,
                Version::Shipped { .. } => panic!("the adaptation is used"),
            }
        };
        assert_eq!(stale_of(&overrides), Stale::default());

        overrides.insert(en_user, hand(en_user, "Rewrite it another way.\n{TEXT}"));
        assert_eq!(
            stale_of(&overrides),
            Stale {
                shipped_changed: false,
                source_changed: true
            }
        );
        // The source reverted to the shipped template: still not the text
        // the adaptation was made from.
        overrides.remove(en_user);
        assert!(stale_of(&overrides).source_changed);
    }
}
