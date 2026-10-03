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

#[cfg(test)]
mod tests {
    use super::{Effort, Executor};

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
}
