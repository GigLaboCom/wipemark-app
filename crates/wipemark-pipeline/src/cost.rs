//! How much a job will ask for, decided before it starts (D61, OV §4.4).
//!
//! The owner's decision (Q-B13, 2026-10-03): the product does not ask the
//! user how many candidates and rounds — it decides by **who rewrites**,
//! and shows the price before the run. A model on the CPU alone gets one
//! candidate a round (a 4B model there writes about ten tokens a second);
//! a GPU or an endpoint gets two. Up to two rounds either way, and the
//! second only for a chunk whose first round had no candidate pass.
//!
//! [`Planned::cost`](crate::job::Planned::cost) counts the calls exactly
//! and estimates the tokens with `estimate_tokens` over the real prompts.
//! Time is the tokens out divided by a rate **the caller measured**; with
//! no rate there is no time, never a guess.

/// What rewrites, as far as the price goes. How the application learns
/// which it is — the build, the GPU layers, the duty — is E4-6's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Executor {
    /// A local model running on the CPU alone.
    LocalCpu,
    /// A local model offloaded to a GPU.
    LocalGpu,
    /// An endpoint over HTTP.
    Endpoint,
}

impl Executor {
    /// The id a report or an answer names it by. A format.
    pub fn as_str(self) -> &'static str {
        match self {
            Executor::LocalCpu => "local-cpu",
            Executor::LocalGpu => "local-gpu",
            Executor::Endpoint => "endpoint",
        }
    }
}

/// Candidates per round and rounds per chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Effort {
    pub candidates: u8,
    pub rounds: u8,
}

impl Effort {
    /// D61: 1 × 2 on the CPU, 2 × 2 on a GPU or an endpoint. The user may
    /// raise both; this is where the product starts.
    pub fn for_executor(executor: Executor) -> Effort {
        match executor {
            Executor::LocalCpu => Effort {
                candidates: 1,
                rounds: 2,
            },
            Executor::LocalGpu | Executor::Endpoint => Effort {
                candidates: 2,
                rounds: 2,
            },
        }
    }
}

/// Two figures for one quantity: if every round runs (`worst`), and if
/// every chunk passes in its first round (`expected` — the common case the
/// owner's "round 2 only when round 1 had no pass" is priced for).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bound<T> {
    pub worst: T,
    pub expected: T,
}

/// The price of a job, shown before it runs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cost {
    /// Chunks a model will be asked about.
    pub chunks: u32,
    /// Engine calls: one per step of every attempt.
    pub calls: Bound<u32>,
    /// Estimated prompt tokens, system and user, over every call.
    pub tokens_in: Bound<u64>,
    /// Estimated generated tokens: about the size of the text, per step.
    pub tokens_out: Bound<u64>,
    /// `tokens_out` ÷ the caller's tokens per second. `None` when no rate
    /// was given. Prompt processing is not counted.
    pub seconds: Option<Bound<f64>>,
}

impl Cost {
    /// The price as JSON — what a surface without a window hands a caller
    /// before a run. Field names are formats; `seconds` is `null` when no
    /// rate was measured, never a guess, and a tenth of a second is the
    /// precision anybody acts on.
    pub fn to_value(&self) -> serde_json::Value {
        let tenths = |seconds: f64| (seconds * 10.0).round() / 10.0;
        serde_json::json!({
            "chunks": self.chunks,
            "calls": {"worst": self.calls.worst, "expected": self.calls.expected},
            "tokens_in": {"worst": self.tokens_in.worst, "expected": self.tokens_in.expected},
            "tokens_out": {"worst": self.tokens_out.worst, "expected": self.tokens_out.expected},
            "seconds": self.seconds.map(|seconds| serde_json::json!({
                "worst": tenths(seconds.worst),
                "expected": tenths(seconds.expected),
            })),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Bound, Cost, Effort, Executor};

    #[test]
    fn the_executor_decides_candidates_and_rounds() {
        let effort = |e| {
            let Effort { candidates, rounds } = Effort::for_executor(e);
            (candidates, rounds)
        };
        assert_eq!(effort(Executor::LocalCpu), (1, 2));
        assert_eq!(effort(Executor::LocalGpu), (2, 2));
        assert_eq!(effort(Executor::Endpoint), (2, 2));
    }

    #[test]
    fn a_price_without_a_rate_has_no_seconds() {
        let cost = Cost {
            chunks: 2,
            calls: Bound {
                worst: 8,
                expected: 4,
            },
            tokens_in: Bound {
                worst: 900,
                expected: 450,
            },
            tokens_out: Bound {
                worst: 400,
                expected: 200,
            },
            seconds: None,
        };
        let value = cost.to_value();
        assert_eq!(value["calls"]["worst"], 8);
        assert_eq!(value["tokens_out"]["expected"], 200);
        assert!(value["seconds"].is_null(), "{value}");

        let timed = Cost {
            seconds: Some(Bound {
                worst: 40.04,
                expected: 20.02,
            }),
            ..cost
        };
        assert_eq!(timed.to_value()["seconds"]["worst"], 40.0);
        assert_eq!(Executor::LocalGpu.as_str(), "local-gpu");
    }
}
