//! `wipemark-pipeline` — what happens between "the user pressed Clean"
//! and "here is the report".
//!
//! This crate owns the job state machine, the chunking, the
//! candidates × rounds selection loop, the scorers and the batch queue
//! (spec §4). It talks to models only through
//! [`wipemark_engine::RewriteEngine`], so every gate here can run on
//! [`wipemark_engine::fake::FakeEngine`] — no GPU, no network, no
//! weights.
//!
//! # Events, not blocking calls
//!
//! Nothing here returns "the answer" after a long wait. A job returns a
//! [`flume::Receiver<Event>`] immediately and reports progress as it
//! goes, because the consumer is a GPUI window that must not block for a
//! single frame (spec §1.2 and §12: *any blocking call on the
//! foreground is a frozen UI, no exceptions*).
//!
//! # Status
//!
//! The vocabulary below is E0's. Of E4: [`lang::Lang`], the languages the
//! templates are written in, and [`prompt`] — the shipped templates, the
//! assembler, validation, adaptations and the clean-up of an answer
//! (E4-2). The machine that drives them — chunks, candidates, rounds,
//! guards — is still to come.

#![forbid(unsafe_code)]

pub mod lang;
pub mod prompt;

use std::time::Duration;

/// Identifies a job for the lifetime of the process and in the persisted
/// queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JobId(pub u64);

/// What the user asked for. The three presets from spec §6.2, plus the
/// batch form that wraps them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Look, report, change nothing.
    Inspect,
    /// Layer A only — deterministic, verifiable, always available.
    Clean,
    /// Layer A, then rewrite, then Layer A again over the result. The
    /// second pass is not belt-and-braces: a model can reintroduce
    /// invisible characters of its own.
    CleanRewriteClean,
}

/// Where a job is. The UI's status bar renders this directly, so the
/// variants carry the numbers a user actually wants: which chunk, which
/// candidate, which round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stage {
    Queued,
    Inspecting,
    CleaningLayerA,
    LoadingModel,
    Rewriting {
        chunk: u32,
        chunks: u32,
        candidate: u8,
        candidates: u8,
        round: u8,
        rounds: u8,
    },
    Scoring,
    Finished,
    Cancelled,
    Failed,
}

/// Progress, streamed to whoever is watching.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Stage {
        job: JobId,
        stage: Stage,
    },
    /// A token arrived from the engine. The UI appends it to the Result
    /// pane; a CLI ignores it.
    Token {
        job: JobId,
        text: String,
    },
    /// One candidate was rejected by a guard. Kept as an event rather
    /// than swallowed: the honest denominator in the report is "attempts
    /// made", not "attempts shown".
    CandidateRejected {
        job: JobId,
        round: u8,
        candidate: u8,
        guard: &'static str,
        reason: String,
    },
    Finished {
        job: JobId,
        elapsed: Duration,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("engine: {0}")]
    Engine(#[from] wipemark_engine::EngineError),
    #[error("cancelled")]
    Cancelled,
    #[error("not implemented yet: {0}")]
    NotImplemented(&'static str),
}
