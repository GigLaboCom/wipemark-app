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
use sha2::{Digest, Sha256};

use super::{Role, Slot, Tactic};
use crate::lang::Lang;

/// The first segment of every override's key.
pub const PREFIX: &str = "prompts";

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

impl Override {
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
}
