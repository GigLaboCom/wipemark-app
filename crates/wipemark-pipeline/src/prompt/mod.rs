//! The prompts: what a model is asked, in which language, and what is
//! taken back out of its answer (E4-2; D60, D64, D66, D67, D73, D74).
//!
//! Everything here is a pure function over values. Nothing calls an
//! engine, reads a row or localizes a sentence: a [`Problem`], a
//! [`Stripped`], a [`Version`] or a [`Refusal`] is handed up, and the
//! surface that shows one chooses the words. The prompt texts themselves
//! are not catalogue messages — a model reads them, not a person — so they
//! live as data files under `crates/wipemark-pipeline/prompts/` and are
//! gated by [`shipped`]'s own tests rather than by the i18n gates.
//!
//! # The pieces
//!
//! * [`shipped`] — the templates the product ships, one file per [`Slot`],
//!   in `en`, `ru` and `de`, and the sentences the assembler owns
//!   (the placeholder rule, the context sentence, the intensity clauses,
//!   the English fallback clause).
//! * [`choose`] — which template a document gets: the user's override for
//!   its language, else the shipped one, else English with the "do not
//!   translate" clause (D64); and the pivot of `back_translate` (D60).
//! * [`render`] — the assembler. It owns the four markers and turns a
//!   step's two templates, a chunk, its context and an intensity into the
//!   `system` and `prompt` of a [`wipemark_engine::ChatRequest`] (D66).
//! * [`validate`](mod@validate) — what is wrong with an edited template, as values (D74).
//! * [`row`] — the spelling of an override's settings row and its JSON value.
//! * [`adapt`] — asking a model to adapt a template into another language,
//!   and checking what came back; never called on its own (Q-B22).
//! * [`clean`] — what is taken off a model's answer before it is judged,
//!   and only what is unambiguous (D67).
//!
//! Layer B is best-effort: nothing in a template promises that a rewrite
//! removes a mark, and nothing here claims it either.

pub mod adapt;
pub mod choose;
pub mod clean;
pub mod render;
pub mod row;
pub mod shipped;
mod template;
pub mod validate;

pub use adapt::{adaptation_request, check_adaptation, AdaptRefusal};
pub use choose::{
    pivot_for, templates_for, Chosen, Overrides, Plan, Refusal, Stale, StepTemplates, Version,
};
pub use clean::{clean_response, Cleaned, Stripped};
pub use render::{render, Input, RenderError, Rendered};
pub use row::{hash, AdaptedFrom, Origin, Override, RowError};
pub use validate::{validate, BraceSide, Problem, Script, Severity, ValidationContext};

use crate::lang::Lang;

/// A way of asking a model to rewrite (OV §4.3, D73).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tactic {
    /// Different words at the level of each word — the default.
    Paraphrase,
    /// Reads as if a person wrote it; a per-language cliché list.
    Humanize,
    /// Into the pivot language and back (two calls, D60).
    BackTranslate,
    /// An outline, then prose from the outline (two calls).
    Structural,
    /// Comments, docstrings, string literals and local names; English
    /// templates for every document (D73).
    Code,
}

impl Tactic {
    /// Every tactic, in the order of the ladder's usual escalation.
    pub const ALL: [Tactic; 5] = [
        Tactic::Paraphrase,
        Tactic::Humanize,
        Tactic::BackTranslate,
        Tactic::Structural,
        Tactic::Code,
    ];

    /// The stable identifier — a format: it spells the rows and the
    /// report, and is never translated.
    pub fn as_str(self) -> &'static str {
        match self {
            Tactic::Paraphrase => "paraphrase",
            Tactic::Humanize => "humanize",
            Tactic::BackTranslate => "back_translate",
            Tactic::Structural => "structural",
            Tactic::Code => "code",
        }
    }

    /// The inverse of [`Tactic::as_str`]; anything else is `None`.
    pub fn parse(id: &str) -> Option<Tactic> {
        Tactic::ALL.into_iter().find(|tactic| tactic.as_str() == id)
    }

    /// How many calls one chunk costs: two for the tactics that go
    /// through an intermediate text.
    pub fn steps(self) -> u8 {
        match self {
            Tactic::BackTranslate | Tactic::Structural => 2,
            Tactic::Paraphrase | Tactic::Humanize | Tactic::Code => 1,
        }
    }

    /// Whether the intensity clause means anything for this tactic.
    ///
    /// "Keep the sentence structure" is meaningless for an outline or a
    /// translation, and upstream leaves `code` out too (layer-b reference
    /// §3, "The two appended clauses").
    pub fn takes_intensity(self) -> bool {
        matches!(self, Tactic::Paraphrase | Tactic::Humanize)
    }
}

/// The two turns of one request (D66).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Role {
    /// The fixed contract.
    System,
    /// The tactic, the intensity, the context and the text.
    User,
}

impl Role {
    pub const ALL: [Role; 2] = [Role::System, Role::User];

    /// The stable identifier, as the row key spells it.
    pub fn as_str(self) -> &'static str {
        match self {
            Role::System => "system",
            Role::User => "user",
        }
    }

    pub fn parse(id: &str) -> Option<Role> {
        Role::ALL.into_iter().find(|role| role.as_str() == id)
    }

    /// The other turn of the same step.
    pub fn other(self) -> Role {
        match self {
            Role::System => Role::User,
            Role::User => Role::System,
        }
    }
}

/// How far a rewrite is asked to go — three positions, not a number
/// (D73): a model does not hit a fraction anyway, and the report shows
/// the divergence that actually happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum Intensity {
    Light,
    /// The tactic as written; adds no clause.
    #[default]
    Moderate,
    Strong,
}

impl Intensity {
    pub const ALL: [Intensity; 3] = [Intensity::Light, Intensity::Moderate, Intensity::Strong];

    /// The stable identifier — a format.
    pub fn as_str(self) -> &'static str {
        match self {
            Intensity::Light => "light",
            Intensity::Moderate => "moderate",
            Intensity::Strong => "strong",
        }
    }

    pub fn parse(id: &str) -> Option<Intensity> {
        Intensity::ALL
            .into_iter()
            .find(|level| level.as_str() == id)
    }
}

/// One template: a language, a tactic, a step and a turn.
///
/// The fields are private so that a slot the product does not have —
/// `code` in Russian, a second step of `paraphrase` — cannot be built:
/// [`Slot::new`] is the one door, and [`Slot::all`] lists what it lets
/// through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Slot {
    lang: Lang,
    tactic: Tactic,
    step: u8,
    role: Role,
}

impl Slot {
    /// A slot, when the product has one: the step is within the tactic's
    /// steps, and `code` is English only (D73).
    pub fn new(lang: Lang, tactic: Tactic, step: u8, role: Role) -> Option<Slot> {
        let step_exists = (1..=tactic.steps()).contains(&step);
        let language_exists = tactic != Tactic::Code || lang == Lang::En;
        (step_exists && language_exists).then_some(Slot {
            lang,
            tactic,
            step,
            role,
        })
    }

    /// Every slot the product ships, in a fixed order.
    pub fn all() -> Vec<Slot> {
        let mut slots = Vec::new();
        for lang in Lang::ALL {
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

    pub fn lang(self) -> Lang {
        self.lang
    }

    pub fn tactic(self) -> Tactic {
        self.tactic
    }

    pub fn step(self) -> u8 {
        self.step
    }

    pub fn role(self) -> Role {
        self.role
    }

    /// The other turn of the same step.
    pub fn other_role(self) -> Slot {
        Slot {
            role: self.role.other(),
            ..self
        }
    }

    /// The same template in another language, when that slot exists.
    pub fn with_lang(self, lang: Lang) -> Option<Slot> {
        Slot::new(lang, self.tactic, self.step, self.role)
    }
}

/// What a template may say in braces (D64). `{LANG_NAME}` and
/// `{PIVOT_NAME}` are not among them: Russian and German need a language's
/// name in a grammatical case a variable cannot carry, so a template names
/// only its own language, in its own words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Variable {
    /// The chunk, between the text markers.
    Text,
    /// The previous chunk's last sentences, between the context markers,
    /// with the sentence that says not to rewrite them; empty without.
    PrevContext,
    /// The placeholder sentence; empty when the chunk holds no `⟦n⟧`.
    Protected,
    /// The intensity clause; empty for [`Intensity::Moderate`].
    Intensity,
}

impl Variable {
    pub const ALL: [Variable; 4] = [
        Variable::Text,
        Variable::PrevContext,
        Variable::Protected,
        Variable::Intensity,
    ];

    /// The name between the braces.
    pub fn name(self) -> &'static str {
        match self {
            Variable::Text => "TEXT",
            Variable::PrevContext => "PREV_CONTEXT",
            Variable::Protected => "PROTECTED",
            Variable::Intensity => "INTENSITY",
        }
    }

    /// The variable a name between braces spells, exactly.
    pub fn parse(name: &str) -> Option<Variable> {
        Variable::ALL
            .into_iter()
            .find(|variable| variable.name() == name)
    }

    /// Whether the variable may appear in a template of this turn. The
    /// text, its context and the intensity are the user turn's; the
    /// placeholder sentence belongs to the contract or the instruction.
    pub fn allowed_in(self, role: Role) -> bool {
        match self {
            Variable::Protected => true,
            Variable::Text | Variable::PrevContext | Variable::Intensity => role == Role::User,
        }
    }
}

/// The strings around the text and the context (D66). The assembler
/// writes them and nothing else does: they are identical in every
/// language, never written by a user ([`Problem::HandWrittenMarker`]),
/// taken off an answer that repeats them ([`clean_response`]), and a
/// document that happens to contain one has it turned into a protected
/// span before it gets here (E4-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Marker {
    BeginText,
    EndText,
    BeginContext,
    EndContext,
}

impl Marker {
    pub const ALL: [Marker; 4] = [
        Marker::BeginText,
        Marker::EndText,
        Marker::BeginContext,
        Marker::EndContext,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Marker::BeginText => "[[[BEGIN TEXT]]]",
            Marker::EndText => "[[[END TEXT]]]",
            Marker::BeginContext => "[[[BEGIN CONTEXT]]]",
            Marker::EndContext => "[[[END CONTEXT]]]",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Intensity, Role, Slot, Tactic, Variable};
    use crate::lang::Lang;

    #[test]
    fn identifiers_round_trip_and_nothing_else_parses() {
        for tactic in Tactic::ALL {
            assert_eq!(Tactic::parse(tactic.as_str()), Some(tactic));
        }
        for role in Role::ALL {
            assert_eq!(Role::parse(role.as_str()), Some(role));
        }
        for level in Intensity::ALL {
            assert_eq!(Intensity::parse(level.as_str()), Some(level));
        }
        for variable in Variable::ALL {
            assert_eq!(Variable::parse(variable.name()), Some(variable));
        }
        assert_eq!(Tactic::parse("backtranslate"), None);
        assert_eq!(Variable::parse("text"), None);
        assert_eq!(Variable::parse("LANG_NAME"), None);
    }

    #[test]
    fn a_slot_exists_only_where_the_product_has_one() {
        assert!(Slot::new(Lang::Ru, Tactic::Paraphrase, 1, Role::User).is_some());
        assert!(Slot::new(Lang::Ru, Tactic::Paraphrase, 2, Role::User).is_none());
        assert!(Slot::new(Lang::De, Tactic::Structural, 2, Role::System).is_some());
        assert!(Slot::new(Lang::De, Tactic::Structural, 0, Role::System).is_none());
        assert!(Slot::new(Lang::En, Tactic::Code, 1, Role::User).is_some());
        assert!(
            Slot::new(Lang::Ru, Tactic::Code, 1, Role::User).is_none(),
            "code is English only (D73)"
        );
        // 3 languages × (2 + 2 + 4 + 4) slots, plus code's two in English.
        assert_eq!(Slot::all().len(), 3 * 12 + 2);
    }
}
