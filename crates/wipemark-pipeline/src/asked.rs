//! What a surface without a window may ask of a rewrite (E4-6a).
//!
//! Two surfaces start a job with no window behind them — the MCP tool
//! `rewrite` and `wipemark-cli rewrite` — and they take the same
//! arguments: a tactic, an intensity, how many candidates and rounds, the
//! document's format, Layer A's two flags and a seed. [`Asked`] is those
//! arguments as values and [`Asked::options`] turns them into the job's
//! [`Options`], so a rule cannot be one thing over MCP and another at a
//! terminal. Each surface parses its own spelling into it — JSON there,
//! flags here — and says a problem in its own words.
//!
//! Three rules live here and nowhere else:
//!
//! * **What is offered** ([`offered`]): `paraphrase`, `humanize`,
//!   `back_translate`. `structural` rewrites a document from an outline of
//!   it and is offered only behind a confirmation (D73) — an argument is
//!   not one; `code` is not built. Both are refused by name, never
//!   swapped for another tactic.
//! * **How many** ([`MOST`]): candidates and rounds are 1 to 8 each when
//!   given, and decided by who rewrites when not (D61).
//! * **The base seed** ([`fresh_seed`], D83): a fresh one for every job
//!   unless the caller names one, so "rewrite again" differs and a rerun
//!   with the report's `base_seed` can repeat.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cost::{Effort, Executor};
use crate::job::Options;
use crate::lang::Lang;
use crate::prepare::TextFormat;
use crate::prompt::{Intensity, Overrides, Tactic};

/// The most candidates, and the most rounds, a surface accepts. Far above
/// any useful setting (D61 starts at one or two), and below a request
/// that keeps a model busy for a day because of a typo.
pub const MOST: u8 = 8;

/// The tactics a surface without a window offers, in the order a refusal
/// lists them.
pub const OFFERED: [Tactic; 3] = [Tactic::Paraphrase, Tactic::Humanize, Tactic::BackTranslate];

/// The formats a caller may name, as the report spells them. `code` is
/// not among them: a code document is kept whole unless the tactic is
/// `code`, which is not built.
pub const FORMATS: [TextFormat; 3] = [TextFormat::Plain, TextFormat::Markdown, TextFormat::Html];

/// Why a tactic is not run without a window. Values; the surface words
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NotOffered {
    /// `structural` is offered only behind a confirmation in a window
    /// (D73).
    #[error("structural is offered only behind a confirmation in a window")]
    Structural,
    /// The `code` tactic needs a preparation of its own, which is not
    /// built.
    #[error("the code tactic is not built")]
    Code,
}

/// `tactic`, if a surface without a window runs it.
pub fn offered(tactic: Tactic) -> Result<Tactic, NotOffered> {
    match tactic {
        Tactic::Paraphrase | Tactic::Humanize | Tactic::BackTranslate => Ok(tactic),
        Tactic::Structural => Err(NotOffered::Structural),
        Tactic::Code => Err(NotOffered::Code),
    }
}

/// The id a format is named by — the report's own spelling.
pub fn format_id(format: TextFormat) -> &'static str {
    crate::report::format_id(format)
}

/// The format an id names, among [`FORMATS`].
pub fn format_of(id: &str) -> Option<TextFormat> {
    FORMATS.into_iter().find(|format| format_id(*format) == id)
}

/// Whether a count a caller gave is one a surface accepts.
pub fn count_ok(count: u64) -> bool {
    (1..=u64::from(MOST)).contains(&count)
}

/// A base seed nobody has used: 32 bits from the process's own random
/// state, the time and the process id.
///
/// 32 bits because llama.cpp's seed is 32 bits wide (the local engine
/// keeps the low half of the request's) and because a person may type it
/// back as `--seed`. Not a cryptographic number and not meant to be: it
/// only has to differ from the last job's. std's `RandomState` is seeded
/// randomly per process and moves on with every instance, so two jobs in
/// one process differ as well as two processes do.
pub fn fresh_seed() -> u64 {
    let mut hasher = RandomState::new().build_hasher();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    hasher.write_u128(nanos);
    hasher.write_u32(std::process::id());
    hasher.finish() & u64::from(u32::MAX)
}

/// The arguments of one rewrite, as values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked {
    /// One of [`OFFERED`]; the ladder is this one rung.
    pub tactic: Tactic,
    pub intensity: Intensity,
    /// `None`: decided by who rewrites (D61).
    pub candidates: Option<u8>,
    /// `None`: decided by who rewrites (D61).
    pub rounds: Option<u8>,
    pub format: TextFormat,
    /// Layer A's `aggressive`, for the passes before, during and after.
    pub aggressive: bool,
    /// Layer A's `nfkc`, likewise.
    pub nfkc: bool,
    /// `None`: a fresh one for this job ([`fresh_seed`]).
    pub seed: Option<u64>,
}

impl Default for Asked {
    fn default() -> Self {
        Self {
            tactic: Tactic::Paraphrase,
            intensity: Intensity::default(),
            candidates: None,
            rounds: None,
            format: TextFormat::Plain,
            aggressive: false,
            nfkc: false,
            seed: None,
        }
    }
}

impl Asked {
    /// The job's options: the product's defaults for `executor`, with what
    /// was asked laid over them, the template overrides the caller read
    /// and the pivot row. A tactic not offered is refused here as well as
    /// at parse time — the one door both surfaces go through.
    pub fn options(
        &self,
        executor: Executor,
        overrides: Overrides,
        pivot: Option<Lang>,
    ) -> Result<Options, NotOffered> {
        let tactic = offered(self.tactic)?;
        let by_executor = Effort::for_executor(executor);
        let mut options = Options::for_executor(executor);
        options.layer_a = wipemark_core::Options {
            aggressive: self.aggressive,
            nfkc: self.nfkc,
            ..wipemark_core::Options::default()
        };
        options.ladder = vec![tactic];
        options.intensity = self.intensity;
        options.effort = Effort {
            candidates: self.candidates.unwrap_or(by_executor.candidates),
            rounds: self.rounds.unwrap_or(by_executor.rounds),
        };
        options.base_seed = self.seed.unwrap_or_else(fresh_seed);
        options.pivot = pivot;
        options.overrides = overrides;
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_only_tactic_is_refused_by_name_and_never_swapped() {
        for tactic in Tactic::ALL {
            match offered(tactic) {
                Ok(same) => {
                    assert_eq!(same, tactic);
                    assert!(OFFERED.contains(&tactic), "{tactic:?}");
                }
                Err(why) => {
                    assert!(!OFFERED.contains(&tactic), "{tactic:?}");
                    let asked = Asked {
                        tactic,
                        ..Asked::default()
                    };
                    assert_eq!(
                        asked.options(Executor::LocalCpu, Overrides::new(), None),
                        Err(why)
                    );
                }
            }
        }
        assert_eq!(offered(Tactic::Structural), Err(NotOffered::Structural));
        assert_eq!(offered(Tactic::Code), Err(NotOffered::Code));
    }

    /// Absent counts are D61's by executor; given ones are kept as given.
    #[test]
    fn absent_counts_are_decided_by_who_rewrites() {
        let asked = Asked::default();
        let cpu = asked
            .options(Executor::LocalCpu, Overrides::new(), None)
            .expect("offered");
        assert_eq!((cpu.effort.candidates, cpu.effort.rounds), (1, 2));
        let endpoint = asked
            .options(Executor::Endpoint, Overrides::new(), None)
            .expect("offered");
        assert_eq!((endpoint.effort.candidates, endpoint.effort.rounds), (2, 2));

        let given = Asked {
            candidates: Some(3),
            rounds: Some(1),
            ..Asked::default()
        }
        .options(Executor::LocalCpu, Overrides::new(), None)
        .expect("offered");
        assert_eq!((given.effort.candidates, given.effort.rounds), (3, 1));
        assert!(given.check().is_ok());
    }

    /// The arguments reach the options they name, and nothing else moves.
    #[test]
    fn what_was_asked_is_what_the_job_runs() {
        let asked = Asked {
            tactic: Tactic::Humanize,
            intensity: Intensity::Strong,
            format: TextFormat::Markdown,
            aggressive: true,
            nfkc: true,
            seed: Some(42),
            ..Asked::default()
        };
        let options = asked
            .options(Executor::LocalGpu, Overrides::new(), Some(Lang::De))
            .expect("offered");
        assert_eq!(options.ladder, vec![Tactic::Humanize]);
        assert_eq!(options.intensity, Intensity::Strong);
        assert!(options.layer_a.aggressive && options.layer_a.nfkc);
        assert!(!options.layer_a.normalize_spaces && !options.layer_a.keep_soft_hyphen);
        assert_eq!(options.base_seed, 42);
        assert_eq!(options.pivot, Some(Lang::De));
        assert!(!options.structural_confirmed);
    }

    /// "Rewrite again" differs: two jobs asked for no seed get two seeds,
    /// each a number a person can type back.
    #[test]
    fn every_job_gets_its_own_seed_unless_one_is_named() {
        let seeds: Vec<u64> = (0..8)
            .map(|_| {
                Asked::default()
                    .options(Executor::LocalCpu, Overrides::new(), None)
                    .expect("offered")
                    .base_seed
            })
            .collect();
        for seed in &seeds {
            assert!(*seed <= u64::from(u32::MAX), "{seed}");
        }
        let mut distinct = seeds.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert!(distinct.len() > 1, "eight jobs, one seed: {seeds:?}");
    }

    #[test]
    fn counts_are_one_to_eight_and_formats_are_three() {
        assert!(!count_ok(0));
        assert!(count_ok(1) && count_ok(u64::from(MOST)));
        assert!(!count_ok(u64::from(MOST) + 1));
        for format in FORMATS {
            assert_eq!(format_of(format_id(format)), Some(format));
        }
        assert_eq!(format_of("code"), None);
        assert_eq!(format_of("Markdown"), None);
    }
}
