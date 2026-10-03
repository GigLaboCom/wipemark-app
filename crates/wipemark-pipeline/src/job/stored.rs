//! A job's options as a row (E4-4): what a queued document runs with after
//! the process that queued it has gone.
//!
//! A queue item keeps the options it was pushed with, so a per-run choice
//! — two candidates raised for this batch (D61), a `structural` the user
//! confirmed (D73) — survives a restart rather than quietly becoming the
//! defaults. Every field is stored; the template overrides by their D74
//! key and value. A row this build cannot read is an error, never the
//! defaults: running a document under options nobody chose is the failure
//! this module exists to prevent.

use serde_json::{json, Map, Value};
use wipemark_core::LengthDriftGuard;
use wipemark_engine::SamplingParams;

use super::Options;
use crate::cost::Effort;
use crate::lang::Lang;
use crate::prompt::{row, Intensity, Override, Overrides, Tactic};

/// The version of the stored form. A format.
pub const OPTIONS_VERSION: u32 = 1;

/// Why stored options could not be read. `field` names the first one that
/// is missing or not what this build writes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OptionsError {
    #[error("stored options are not JSON: {detail}")]
    NotJson { detail: String },
    #[error("stored options of version {found}; this build reads {OPTIONS_VERSION}")]
    Version { found: u64 },
    #[error("stored options: `{field}` is missing or unreadable")]
    Field { field: String },
}

impl Options {
    /// One line of JSON: every field.
    pub fn to_json(&self) -> String {
        let overrides: Map<String, Value> = self
            .overrides
            .iter()
            .map(|(slot, row)| (row::key(slot), Value::String(row.to_json())))
            .collect();
        json!({
            "options": OPTIONS_VERSION,
            "layer_a": {
                "aggressive": self.layer_a.aggressive,
                "nfkc": self.layer_a.nfkc,
                "normalize_spaces": self.layer_a.normalize_spaces,
                "keep_soft_hyphen": self.layer_a.keep_soft_hyphen,
            },
            "ladder": self.ladder.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
            "structural_confirmed": self.structural_confirmed,
            "intensity": self.intensity.as_str(),
            "candidates": self.effort.candidates,
            "rounds": self.effort.rounds,
            "base_seed": self.base_seed,
            "sampling": {
                "temperature": self.sampling.temperature,
                "top_p": self.sampling.top_p,
                "min_p": self.sampling.min_p,
                "seed": self.sampling.seed,
                "max_tokens": self.sampling.max_tokens,
            },
            "pivot": self.pivot.map(Lang::as_str),
            "overrides": overrides,
            "length": { "min": self.length.min, "max": self.length.max },
        })
        .to_string()
    }

    /// Read options [`Options::to_json`] wrote.
    pub fn from_json(text: &str) -> Result<Options, OptionsError> {
        let value: Value = serde_json::from_str(text).map_err(|error| OptionsError::NotJson {
            detail: error.to_string(),
        })?;
        let version = value
            .get("options")
            .and_then(Value::as_u64)
            .ok_or_else(|| field("options"))?;
        if version != u64::from(OPTIONS_VERSION) {
            return Err(OptionsError::Version { found: version });
        }
        let at = |path: &[&str]| -> Result<&Value, OptionsError> {
            let mut here = &value;
            for key in path {
                here = here.get(*key).ok_or_else(|| field(&path.join(".")))?;
            }
            Ok(here)
        };
        let boolean = |path: &[&str]| at(path)?.as_bool().ok_or_else(|| field(&path.join(".")));
        let float = |path: &[&str]| {
            at(path)?
                .as_f64()
                .map(|x| x as f32)
                .ok_or_else(|| field(&path.join(".")))
        };
        let small = |path: &[&str]| {
            at(path)?
                .as_u64()
                .and_then(|n| u8::try_from(n).ok())
                .ok_or_else(|| field(&path.join(".")))
        };
        let optional = |path: &[&str]| -> Result<Option<&Value>, OptionsError> {
            let here = at(path)?;
            Ok((!here.is_null()).then_some(here))
        };

        let ladder = at(&["ladder"])?
            .as_array()
            .ok_or_else(|| field("ladder"))?
            .iter()
            .map(|id| {
                id.as_str()
                    .and_then(Tactic::parse)
                    .ok_or_else(|| field("ladder"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let intensity = at(&["intensity"])?
            .as_str()
            .and_then(Intensity::parse)
            .ok_or_else(|| field("intensity"))?;
        let pivot = match optional(&["pivot"])? {
            None => None,
            Some(code) => Some(
                code.as_str()
                    .and_then(Lang::parse)
                    .ok_or_else(|| field("pivot"))?,
            ),
        };
        let mut overrides = Overrides::new();
        for (key, row) in at(&["overrides"])?
            .as_object()
            .ok_or_else(|| field("overrides"))?
        {
            let slot = row::parse_key(key).ok_or_else(|| field("overrides"))?;
            let parsed = row
                .as_str()
                .map(Override::parse)
                .and_then(Result::ok)
                .ok_or_else(|| field("overrides"))?;
            overrides.insert(slot, parsed);
        }
        let min_p = match optional(&["sampling", "min_p"])? {
            None => None,
            Some(x) => Some(x.as_f64().ok_or_else(|| field("sampling.min_p"))? as f32),
        };
        let seed = match optional(&["sampling", "seed"])? {
            None => None,
            Some(x) => Some(x.as_u64().ok_or_else(|| field("sampling.seed"))?),
        };
        let max_tokens = match optional(&["sampling", "max_tokens"])? {
            None => None,
            Some(x) => Some(
                x.as_u64()
                    .and_then(|n| u32::try_from(n).ok())
                    .ok_or_else(|| field("sampling.max_tokens"))?,
            ),
        };

        Ok(Options {
            layer_a: wipemark_core::Options {
                aggressive: boolean(&["layer_a", "aggressive"])?,
                nfkc: boolean(&["layer_a", "nfkc"])?,
                normalize_spaces: boolean(&["layer_a", "normalize_spaces"])?,
                keep_soft_hyphen: boolean(&["layer_a", "keep_soft_hyphen"])?,
            },
            ladder,
            structural_confirmed: boolean(&["structural_confirmed"])?,
            intensity,
            effort: Effort {
                candidates: small(&["candidates"])?,
                rounds: small(&["rounds"])?,
            },
            base_seed: at(&["base_seed"])?
                .as_u64()
                .ok_or_else(|| field("base_seed"))?,
            sampling: SamplingParams {
                temperature: float(&["sampling", "temperature"])?,
                top_p: float(&["sampling", "top_p"])?,
                min_p,
                seed,
                max_tokens,
            },
            pivot,
            overrides,
            length: LengthDriftGuard {
                min: float(&["length", "min"])?,
                max: float(&["length", "max"])?,
            },
        })
    }
}

fn field(name: &str) -> OptionsError {
    OptionsError::Field {
        field: name.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Options, OptionsError};
    use crate::cost::{Effort, Executor};
    use crate::lang::Lang;
    use crate::prompt::{
        hash, shipped, Intensity, Origin, Override, Overrides, Role, Slot, Tactic,
    };

    fn unusual() -> Options {
        let slot = Slot::new(Lang::Ru, Tactic::Paraphrase, 1, Role::User).expect("a slot");
        let mut overrides = Overrides::new();
        overrides.insert(
            slot,
            Override {
                text: "Перепиши: {TEXT}".to_owned(),
                based_on: hash(shipped::template(slot).expect("shipped")),
                adapted_from: None,
                origin: Origin::Hand,
            },
        );
        let mut options = Options::for_executor(Executor::LocalCpu);
        options.layer_a.aggressive = true;
        options.layer_a.keep_soft_hyphen = true;
        options.ladder = vec![Tactic::Paraphrase, Tactic::Humanize, Tactic::Structural];
        options.structural_confirmed = true;
        options.intensity = Intensity::Strong;
        options.effort = Effort {
            candidates: 3,
            rounds: 2,
        };
        options.base_seed = u64::MAX - 7;
        options.sampling.temperature = 0.7;
        options.sampling.min_p = Some(0.05);
        options.sampling.max_tokens = Some(512);
        options.pivot = Some(Lang::De);
        options.overrides = overrides;
        options.length.min = 0.4;
        options.length.max = 2.5;
        options
    }

    #[test]
    fn options_round_trip_with_overrides() {
        for options in [Options::for_executor(Executor::Endpoint), unusual()] {
            let stored = options.to_json();
            let back = Options::from_json(&stored).expect("reads back");
            assert_eq!(back, options);
            // The fingerprint reads `Debug`: equal values, equal text.
            assert_eq!(format!("{back:?}"), format!("{options:?}"));
        }
    }

    #[test]
    fn an_unreadable_options_row_is_an_error_not_the_defaults() {
        let stored = unusual().to_json();
        let unknown_tactic = stored.replace("\"humanize\"", "\"humanise\"");
        assert_eq!(
            Options::from_json(&unknown_tactic),
            Err(OptionsError::Field {
                field: "ladder".to_owned()
            })
        );
        let mut value: serde_json::Value = serde_json::from_str(&stored).expect("json");
        value.as_object_mut().expect("object").remove("rounds");
        assert_eq!(
            Options::from_json(&value.to_string()),
            Err(OptionsError::Field {
                field: "rounds".to_owned()
            })
        );
        value["options"] = serde_json::json!(2);
        assert_eq!(
            Options::from_json(&value.to_string()),
            Err(OptionsError::Version { found: 2 })
        );
        assert!(matches!(
            Options::from_json("{"),
            Err(OptionsError::NotJson { .. })
        ));
    }
}
