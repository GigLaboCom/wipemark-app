//! An override as a settings row (D74).
//!
//! Only overrides are stored: a row `prompts.<lang>.<tactic>.<step>.<role>`
//! whose value is JSON `{"text", "based_on", "adapted_from", "origin"}`. No
//! row is the shipped template; "Restore default" deletes the row. The
//! keys are dynamic, like `engine.profiles.<id>`, and live outside the
//! application's `config::PERSISTED`.
//!
//! This module spells and reads the row and nothing more. The database is
//! the application's (E4-6): `wipemark-pipeline` does not depend on
//! `wipemark-store`. A value that does not parse is a [`RowError`] — the
//! caller keeps the row exactly as it is and uses the shipped template,
//! which is the repository's rule for any row a build cannot read.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{
    check_adaptation, shipped, validate, Intensity, Overrides, Problem, Role, Severity, Slot,
    Tactic, ValidationContext,
};
use crate::lang::Lang;

/// The first segment of every override's key.
pub const PREFIX: &str = "prompts";

/// The pivot row of `back_translate` (D60): a language id as a JSON
/// string, `"de"`. No row is the default pivot ([`super::pivot_for`]); a
/// value this build cannot read is the default too, and the row stays.
/// Since E4-6c a preference with a widget on the Settings window's
/// Prompts section, and so in the application's `config::PERSISTED`
/// (D331); the overrides stay dynamic keys outside it. The surfaces
/// without a window read it as they did.
pub const PIVOT_KEY: &str = "rewrite.pivot";

/// The pivot a row's value names, or `None` — no row, or one this build
/// cannot read.
pub fn pivot_of(value: Option<&Value>) -> Option<Lang> {
    value.and_then(Value::as_str).and_then(Lang::parse)
}

/// The overrides among a database's rows, and the keys left alone.
///
/// Every row whose key starts with `prompts.` is one of two things: an
/// override this build reads — it goes into the [`Overrides`] — or one it
/// cannot (a slot it does not have, a value that is not D74's object),
/// whose key is handed back so the caller can log how many. Such a row is
/// left exactly where it is and its slot uses the shipped template: the
/// repository's rule for any row a build cannot read. Rows with any
/// other key are not overrides and are passed over — and so is a template
/// profile's, `prompts.profiles.<id>` (E4-9), which is a list entry and not
/// a slot.
///
/// A value is D74's object, as the settings table stores JSON; a string
/// holding that object is read as well, since a row written as text by
/// something other than this build means the same thing.
pub fn overrides_from<'a>(
    rows: impl IntoIterator<Item = (&'a str, &'a Value)>,
) -> (Overrides, Vec<String>) {
    let mut overrides = Overrides::new();
    let mut unread = Vec::new();
    for (key, value) in rows {
        if !key.starts_with(&format!("{PREFIX}.")) || key.starts_with(super::profile::PREFIX) {
            continue;
        }
        let parsed = parse_key(key).and_then(|slot| {
            let text = match value {
                Value::String(text) => text.clone(),
                Value::Object(_) => value.to_string(),
                _ => return None,
            };
            Override::parse(&text).ok().map(|row| (slot, row))
        });
        match parsed {
            Some((slot, row)) => {
                overrides.insert(slot, row);
            }
            None => unread.push(key.to_owned()),
        }
    }
    (overrides, unread)
}

/// The key of a slot's override row: `prompts.ru.paraphrase.1.user`.
pub fn key(slot: Slot) -> String {
    format!(
        "{PREFIX}.{}.{}.{}.{}",
        slot.lang().as_str(),
        slot.tactic().as_str(),
        slot.step(),
        slot.role().as_str()
    )
}

/// The slot a key names — only a key [`key`] could have written, for a
/// slot the product has.
pub fn parse_key(key: &str) -> Option<Slot> {
    let mut parts = key.split('.');
    if parts.next()? != PREFIX {
        return None;
    }
    let lang = Lang::parse(parts.next()?)?;
    let tactic = Tactic::parse(parts.next()?)?;
    let step = match parts.next()? {
        "1" => 1,
        "2" => 2,
        _ => return None,
    };
    let role = Role::parse(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    Slot::new(lang, tactic, step, role)
}

/// A template's identity: sha256 of its text, lower-case hex, the first
/// sixteen characters. Long enough that two templates a person wrote do
/// not collide; short enough to read in a report.
pub fn hash(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    let mut hex = hex::encode(digest);
    hex.truncate(16);
    hex
}

/// Who wrote an override.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Origin {
    /// Typed by the user.
    Hand,
    /// Adapted by a model and not yet looked at.
    Machine,
    /// Adapted by a model, then opened and saved by the user.
    MachineReviewed,
}

impl Origin {
    /// The stable identifier, as the row spells it.
    pub fn as_str(self) -> &'static str {
        match self {
            Origin::Hand => "hand",
            Origin::Machine => "machine",
            Origin::MachineReviewed => "machine-reviewed",
        }
    }
}

/// The template an adaptation was made from: the same tactic, step and
/// turn in `lang`, whose text then hashed to `hash`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptedFrom {
    pub lang: Lang,
    pub hash: String,
}

/// One override row's value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Override {
    /// The template.
    pub text: String,
    /// [`hash`] of the shipped template of the same slot when this one was
    /// made — what tells the page that the default moved on since.
    pub based_on: String,
    /// The source, when this override is an adaptation of another
    /// language's template; `None` when it was written in its own.
    pub adapted_from: Option<AdaptedFrom>,
    pub origin: Origin,
}

/// Why a row's value is not an override this build can read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RowError {
    /// Not JSON, or not the shape — a field missing or of the wrong type,
    /// an origin this build does not know. `detail` is serde's own words,
    /// for a log line; never shown as a sentence.
    #[error("not a template override: {detail}")]
    Malformed { detail: String },
    /// `adapted_from.lang` names a language without templates.
    #[error("adapted from a language this build has no templates for: {lang:?}")]
    UnknownLang { lang: String },
}

#[derive(Serialize, Deserialize)]
struct Raw {
    text: String,
    based_on: String,
    #[serde(default)]
    adapted_from: Option<RawSource>,
    origin: Origin,
}

#[derive(Serialize, Deserialize)]
struct RawSource {
    lang: String,
    hash: String,
}

/// Why a template a caller laid over the saved ones was refused.
///
/// Values; the surface words them. `key` is the row's key as the caller
/// spelled it — it names the set (`prompts.ru.…`), the tactic, the step and
/// the turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Laid {
    /// The key names no template row this build has.
    UnknownRow { key: String },
    /// The value is neither a template's text nor D74's object.
    Unreadable { key: String },
    /// The template breaks a rule: the id of the first error-severity
    /// problem [`validate`] finds — a format, never translated.
    Breaks { key: String, rule: &'static str },
}

/// What the one rule says about an override before it is stored or laid
/// over the saved ones (E4-6c R3, D330): every [`Problem`] [`validate`]
/// finds — errors and warnings, in rule order — checked beside the other
/// turn of its step **as it will be used**.
///
/// The Settings page's Save and [`lay_over`] (the CLI's `--prompts`, the
/// MCP tool's `templates`) both ask [`admit`], so a template the page
/// stores is one the CLI runs, and the other way round. Errors refuse;
/// warnings are said and do not.
#[derive(Debug, Clone, PartialEq)]
pub struct Admission {
    /// Every problem, errors and warnings, in [`validate`]'s order.
    pub problems: Vec<Problem>,
}

impl Admission {
    /// Whether the override may be stored (or laid over): no error.
    pub fn admitted(&self) -> bool {
        self.first_error().is_none()
    }

    /// The first error, in rule order — the one [`Laid::Breaks`] names.
    pub fn first_error(&self) -> Option<&Problem> {
        self.problems
            .iter()
            .find(|problem| problem.severity() == Severity::Error)
    }
}

/// The context [`admit`] checks `row` in: the step's other turn as it will
/// be used (its override in `beside`, else the shipped text), the model's
/// window when the surface knows it, moderate intensity (a template is
/// stored for every intensity), and the row's own `based_on`, so a stale
/// override is warned about.
fn context_for<'a>(
    slot: Slot,
    row: &'a Override,
    beside: &'a Overrides,
    ctx_len: Option<u32>,
) -> ValidationContext<'a> {
    ValidationContext {
        other_role: beside.effective(slot.other_role()).unwrap_or_default(),
        ctx_len,
        intensity: Intensity::Moderate,
        based_on: Some(row.based_on.as_str()),
    }
}

/// The one rule that decides whether `row` may be stored as `slot`'s
/// override, beside the overrides in `beside` (D330).
///
/// `ctx_len` is the window of the model on duty, when the surface knows
/// it: a template over a tenth of it is refused (`too-long`). `None` skips
/// that rule — an endpoint that does not say, or a surface with no engine
/// in sight — and every other rule is the same either way.
pub fn admit(slot: Slot, row: &Override, beside: &Overrides, ctx_len: Option<u32>) -> Admission {
    let context = context_for(slot, row, beside, ctx_len);
    Admission {
        problems: validate(slot, &row.text, &context),
    }
}

/// [`admit`] for an adaptation of `source_text` (the same tactic, step and
/// turn in another language): the same rule, plus exactly the source's
/// variables ([`check_adaptation`]) — the main risk of a machine
/// adaptation is `{TEXT}` coming back as `{ТЕКСТ}`.
pub fn admit_adaptation(
    slot: Slot,
    row: &Override,
    source_text: &str,
    beside: &Overrides,
    ctx_len: Option<u32>,
) -> Admission {
    let context = context_for(slot, row, beside, ctx_len);
    Admission {
        problems: check_adaptation(slot, source_text, &row.text, &context),
    }
}

/// Lay `rows` — a caller's own templates, each a row key and either the
/// template's text or D74's object — over `overrides`, strictly.
///
/// For a surface that takes templates from its caller rather than from the
/// database (D75): the CLI's `--prompts` file, the MCP tool's `templates`.
/// Unlike a row in the database — which a build that cannot read it passes
/// over, keeping the row — a template handed in for this run and wrong is
/// a refusal: the caller asked for it by name, and a run on the shipped
/// template instead would be a different job reported as theirs. Each is
/// checked by [`admit`] — beside the other turn of its step **as it will
/// be used** (the other laid row, the saved override or the shipped text),
/// because `{PROTECTED}` is required once per step and neither turn can be
/// judged alone. Warnings do not refuse — a template the page would save
/// is one this runs.
///
/// No window is known here, so `too-long` is not asked: this is
/// [`lay_over_within`] with `None`, which refuses exactly what `lay_over`
/// refused before E4-6c.
pub fn lay_over(
    overrides: &mut Overrides,
    rows: &serde_json::Map<String, Value>,
) -> Result<(), Laid> {
    lay_over_within(overrides, rows, None)
}

/// [`lay_over`] for a surface that knows the window of the model on duty
/// (D330): a laid template over a tenth of `ctx_len` is refused as
/// `too-long`, as the Settings page refuses it.
pub fn lay_over_within(
    overrides: &mut Overrides,
    rows: &serde_json::Map<String, Value>,
    ctx_len: Option<u32>,
) -> Result<(), Laid> {
    let mut laid = Vec::with_capacity(rows.len());
    for (key, value) in rows {
        let slot = parse_key(key).ok_or_else(|| Laid::UnknownRow { key: key.clone() })?;
        let row = match value {
            Value::String(text) => Override::by_hand(slot, text.clone()),
            Value::Object(_) => Override::parse(&value.to_string())
                .map_err(|_| Laid::Unreadable { key: key.clone() })?,
            _ => return Err(Laid::Unreadable { key: key.clone() }),
        };
        overrides.insert(slot, row);
        laid.push((key, slot));
    }
    for (key, slot) in laid {
        let Some(row) = overrides.get(slot) else {
            continue;
        };
        if let Some(problem) = admit(slot, row, overrides, ctx_len).first_error() {
            return Err(Laid::Breaks {
                key: key.clone(),
                rule: problem.rule(),
            });
        }
    }
    Ok(())
}

impl Override {
    /// A template typed by hand for `slot`, made from today's shipped one —
    /// what a bare string stands for where an override is expected (the
    /// CLI's `--prompts` file).
    pub fn by_hand(slot: Slot, text: impl Into<String>) -> Override {
        Override {
            text: text.into(),
            based_on: hash(shipped::template(slot).unwrap_or_default()),
            adapted_from: None,
            origin: Origin::Hand,
        }
    }

    /// Read a row's value. Fields this build does not know are ignored, so
    /// a newer build's row still reads; a missing or malformed one is an
    /// error value, and the row is left alone.
    pub fn parse(value: &str) -> Result<Override, RowError> {
        let raw: Raw = serde_json::from_str(value).map_err(|error| RowError::Malformed {
            detail: error.to_string(),
        })?;
        let adapted_from = match raw.adapted_from {
            Some(source) => Some(AdaptedFrom {
                lang: Lang::parse(&source.lang)
                    .ok_or(RowError::UnknownLang { lang: source.lang })?,
                hash: source.hash,
            }),
            None => None,
        };
        Ok(Override {
            text: raw.text,
            based_on: raw.based_on,
            adapted_from,
            origin: raw.origin,
        })
    }

    /// The row's value, in the field order D74 writes it.
    pub fn to_json(&self) -> String {
        let raw = Raw {
            text: self.text.clone(),
            based_on: self.based_on.clone(),
            adapted_from: self.adapted_from.as_ref().map(|source| RawSource {
                lang: source.lang.as_str().to_owned(),
                hash: source.hash.clone(),
            }),
            origin: self.origin,
        };
        serde_json::to_string(&raw).expect("a struct of strings always serializes")
    }
}

#[cfg(test)]
mod tests {
    use super::{hash, key, parse_key, AdaptedFrom, Origin, Override, RowError};
    use crate::lang::Lang;
    use crate::prompt::{Role, Slot, Tactic};

    #[test]
    fn a_row_key_round_trips_and_nothing_else_parses() {
        for slot in Slot::all() {
            assert_eq!(parse_key(&key(slot)), Some(slot), "{}", key(slot));
        }
        let slot = Slot::new(Lang::Ru, Tactic::BackTranslate, 2, Role::User).expect("slot");
        assert_eq!(key(slot), "prompts.ru.back_translate.2.user");
        for wrong in [
            "prompts.ru.code.1.user",        // code is English only
            "prompts.en.paraphrase.2.user",  // paraphrase has one step
            "prompts.en.paraphrase.+1.user", // a number is not a step
            "prompts.en.paraphrase.01.user",
            "prompts.fr.paraphrase.1.user",      // no French set
            "prompts.en.paraphrase.1.assistant", // no such turn
            "prompts.en.paraphrase.1.user.x",
            "prompts.en.paraphrase.1",
            "prompt.en.paraphrase.1.user",
            "engine.profiles.local",
            "",
        ] {
            assert_eq!(parse_key(wrong), None, "{wrong}");
        }
    }

    #[test]
    fn the_hash_is_sixteen_hex_digits_of_sha256() {
        // sha256("abc") = ba7816bf8f01cfea414140de5dae2223…
        assert_eq!(hash("abc"), "ba7816bf8f01cfea");
        assert_ne!(hash("abc"), hash("abc "));
    }

    #[test]
    fn an_override_value_round_trips() {
        let plain = Override {
            text: "Перепиши.\n{TEXT}".to_owned(),
            based_on: hash("shipped"),
            adapted_from: None,
            origin: Origin::Hand,
        };
        let adapted = Override {
            adapted_from: Some(AdaptedFrom {
                lang: Lang::En,
                hash: hash("source"),
            }),
            origin: Origin::MachineReviewed,
            ..plain.clone()
        };
        for value in [plain, adapted] {
            assert_eq!(Override::parse(&value.to_json()), Ok(value));
        }
        let json = Override::parse(
            r#"{"text":"t {TEXT}","based_on":"x","adapted_from":null,"origin":"machine","later":1}"#,
        )
        .expect("an unknown field is ignored, a null source is none");
        assert_eq!(json.origin, Origin::Machine);
        assert_eq!(json.adapted_from, None);
        assert!(Override::parse(r#"{"text":"t","based_on":"x","origin":"hand"}"#).is_ok());
        assert_eq!(
            Origin::MachineReviewed.as_str(),
            "machine-reviewed",
            "the spelling D74 names"
        );
    }

    #[test]
    fn an_unreadable_override_is_an_error_value() {
        for value in [
            "",
            "not json",
            r#""just a string""#,
            r#"{"based_on":"x","origin":"hand"}"#,
            r#"{"text":"t","origin":"hand"}"#,
            r#"{"text":"t","based_on":"x"}"#,
            r#"{"text":"t","based_on":"x","origin":"robot"}"#,
            r#"{"text":7,"based_on":"x","origin":"hand"}"#,
        ] {
            assert!(
                matches!(Override::parse(value), Err(RowError::Malformed { .. })),
                "{value}"
            );
        }
        assert_eq!(
            Override::parse(
                r#"{"text":"t","based_on":"x","adapted_from":{"lang":"fr","hash":"y"},"origin":"machine"}"#
            ),
            Err(RowError::UnknownLang {
                lang: "fr".to_owned()
            })
        );
    }

    /// The rows a surface reads: overrides go in, anything this build
    /// cannot read is named and left, and rows that are not overrides are
    /// not looked at.
    #[test]
    fn the_rows_become_overrides_and_the_unreadable_ones_are_named() {
        use serde_json::{json, Value};

        use super::{overrides_from, pivot_of, PIVOT_KEY};
        use crate::lang::Lang;
        use crate::prompt::{Role, Slot, Tactic};

        let slot = Slot::new(Lang::Ru, Tactic::Paraphrase, 1, Role::User).expect("a slot");
        let row = Override::by_hand(slot, "Перепиши: {TEXT} {PROTECTED}");
        assert_eq!(row.origin, Origin::Hand);
        assert_eq!(row.adapted_from, None);
        assert_eq!(row.based_on.len(), 16);

        let object: Value = serde_json::from_str(&row.to_json()).expect("JSON");
        let as_text = Value::String(row.to_json());
        let rows: Vec<(String, Value)> = vec![
            (key(slot), object),
            ("prompts.en.paraphrase.1.user".to_owned(), as_text),
            (
                "prompts.fr.paraphrase.1.user".to_owned(),
                json!({"text": "x"}),
            ),
            (
                "prompts.de.paraphrase.1.user".to_owned(),
                json!("not an override"),
            ),
            ("prompts.de.humanize.1.user".to_owned(), json!(7)),
            ("ui.theme".to_owned(), json!("dark")),
            (PIVOT_KEY.to_owned(), json!("de")),
        ];
        let (overrides, unread) =
            overrides_from(rows.iter().map(|(key, value)| (key.as_str(), value)));
        assert_eq!(overrides.get(slot), Some(&row));
        let english = Slot::new(Lang::En, Tactic::Paraphrase, 1, Role::User).expect("a slot");
        assert_eq!(overrides.get(english), Some(&row), "a row stored as text");
        assert_eq!(overrides.iter().count(), 2);
        assert_eq!(
            unread,
            [
                "prompts.fr.paraphrase.1.user",
                "prompts.de.paraphrase.1.user",
                "prompts.de.humanize.1.user"
            ]
        );

        assert_eq!(pivot_of(Some(&json!("de"))), Some(Lang::De));
        assert_eq!(pivot_of(Some(&json!("fr"))), None);
        assert_eq!(pivot_of(Some(&json!(1))), None);
        assert_eq!(pivot_of(None), None);
    }

    /// A caller's own templates are laid over the saved ones strictly: a
    /// row this build does not have, a value that is no template and a
    /// template that breaks a rule are each refused by key — and a good
    /// one goes in, checked beside the other turn of its step.
    #[test]
    fn a_callers_templates_are_laid_over_strictly() {
        use serde_json::{json, Map, Value};

        use super::{lay_over, Laid};
        use crate::lang::Lang;
        use crate::prompt::{Overrides, Role, Slot, Tactic};

        let rows =
            |value: Value| -> Map<String, Value> { value.as_object().expect("an object").clone() };
        let english = Slot::new(Lang::En, Tactic::Paraphrase, 1, Role::User).expect("a slot");

        let mut overrides = Overrides::new();
        let good = "Say it again in other words. {PROTECTED}\n{TEXT}";
        lay_over(&mut overrides, &rows(json!({ key(english): good })))
            .expect("a template the page would save");
        assert_eq!(overrides.effective(english), Some(good));
        assert_eq!(
            overrides.get(english).map(|row| row.origin),
            Some(Origin::Hand)
        );

        for (value, refused) in [
            (
                json!({"prompts.fr.paraphrase.1.user": good}),
                Laid::UnknownRow {
                    key: "prompts.fr.paraphrase.1.user".to_owned(),
                },
            ),
            (
                json!({ key(english): 7 }),
                Laid::Unreadable { key: key(english) },
            ),
            (
                json!({ key(english): {"text": good} }),
                Laid::Unreadable { key: key(english) },
            ),
            (
                json!({ key(english): "Say it again. {PROTECTED}" }),
                Laid::Breaks {
                    key: key(english),
                    rule: "missing-variable",
                },
            ),
            (
                json!({ key(english): "Say it again. [[[BEGIN TEXT]]] {PROTECTED}\n{TEXT}" }),
                Laid::Breaks {
                    key: key(english),
                    rule: "hand-written-marker",
                },
            ),
        ] {
            assert_eq!(
                lay_over(&mut Overrides::new(), &rows(value.clone())),
                Err(refused),
                "{value}"
            );
        }
    }

    /// The one rule (D330): errors refuse, warnings are said and do not,
    /// the other turn is read as it will be used, and the window is asked
    /// only by a surface that knows it — `lay_over` knows none, so it
    /// refuses what it refused before.
    #[test]
    fn the_one_rule_refuses_errors_and_says_warnings() {
        use serde_json::{json, Value};

        use super::{admit, lay_over, lay_over_within, Laid};
        use crate::prompt::Overrides;

        let user = Slot::new(Lang::En, Tactic::Paraphrase, 1, Role::User).expect("a slot");
        let system = user.other_role();

        // A warning (nothing but the text) is admitted and said.
        let bare = Override::by_hand(user, "{TEXT}");
        let admission = admit(user, &bare, &Overrides::new(), None);
        assert!(admission.admitted());
        assert_eq!(
            admission
                .problems
                .iter()
                .map(|problem| problem.rule())
                .collect::<Vec<_>>(),
            ["nothing-but-text"]
        );

        // The other turn as it will be used: a saved system turn without
        // {PROTECTED} makes a user turn without it an error.
        let mut saved = Overrides::new();
        saved.insert(system, Override::by_hand(system, "Keep every fact."));
        let plain = Override::by_hand(user, "Say it again.\n{TEXT}");
        assert!(admit(user, &plain, &Overrides::new(), None).admitted());
        assert_eq!(
            admit(user, &plain, &saved, None)
                .first_error()
                .map(|problem| problem.rule()),
            Some("missing-variable")
        );

        // The window: refused within it, run without one.
        let long = format!("{}\n{{TEXT}}", "word ".repeat(400));
        let rows: serde_json::Map<String, Value> = json!({ key(user): long })
            .as_object()
            .expect("an object")
            .clone();
        assert_eq!(lay_over(&mut Overrides::new(), &rows), Ok(()));
        assert_eq!(
            lay_over_within(&mut Overrides::new(), &rows, Some(4096)),
            Err(Laid::Breaks {
                key: key(user),
                rule: "too-long"
            })
        );
        assert_eq!(
            lay_over_within(&mut Overrides::new(), &rows, Some(100_000)),
            Ok(())
        );
    }
}
