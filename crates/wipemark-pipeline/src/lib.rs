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
//! The vocabulary below is E0's, reshaped by E4-3. Of E4: [`lang`] — the
//! languages the templates are written in, and the detection of a
//! document's (E4-1); [`prepare`] — what of a document is prose, the
//! protected spans and their placeholders, the chunks and their context,
//! and the way back byte for byte (E4-1); [`prompt`] — the shipped
//! templates, the assembler, validation, adaptations and the clean-up of
//! an answer (E4-2); and the loop that drives them (E4-3) — [`job`] (the
//! job on its thread: Layer A, candidates × rounds, the guards, the
//! no-op guard, restore, assembly, Layer A again), [`select`]
//! (the most diverged wins since E4-7, the floor, the length windows,
//! the scorer seam), [`cost`] (D61's effort by
//! executor and the price before a run) and [`report`] (every attempt,
//! the three shelves, the JSON form). E4-4 added the job that can be
//! taken up again — [`start_resumable`], [`Decided`] and the options as a
//! row — which the batch queue drives: `wipemark-queue`, a crate of its
//! own, because a queue also needs the store and the user's files and this
//! crate may reach neither. E4-6a added the first surfaces that start a
//! job — the MCP tool `rewrite` and `wipemark-cli rewrite` — and what
//! they share: [`asked`] (the arguments, what is offered without a
//! window, the base seed), [`wait`] and [`block_on`] for a caller with no
//! window to keep drawing, and the rows of [`prompt::row`] read into
//! overrides. E4-7 built the prompt bench's recommendations (D95): the
//! most-changed candidate wins with a no-op floor of 0.2, a short chunk
//! has a wider length window, an answer not in its chunk's language is
//! refused, and a list item is a chunk of its own. The windows are E4-6b.

#![forbid(unsafe_code)]

pub mod asked;
pub mod cost;
pub mod job;
pub mod lang;
pub mod prepare;
pub mod prompt;
pub mod report;
pub mod select;

use std::time::Duration;

pub use job::{
    block_on, seed_for, start, start_resumable, wait, Decided, Document, Ending, JobHandle,
    Options, OptionsError, Outcome, RecordError, Refused,
};
pub use report::{Carried, ChunkCounts, EngineFailure, JobReport, Rejection};

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
///
/// A job's events arrive in the order they happened, from one thread, and
/// end with exactly one of [`Event::Finished`], [`Event::Cancelled`] or
/// [`Event::Failed`], each preceded by the [`Stage`] of the same name.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Stage {
        job: JobId,
        stage: Stage,
    },
    /// A token arrived from the engine. The UI appends it to the Result
    /// pane; a CLI ignores it. It belongs to the attempt the last
    /// [`Stage::Rewriting`] announced — rejected attempts stream too.
    Token {
        job: JobId,
        text: String,
    },
    /// One candidate was rejected. Kept as an event rather than
    /// swallowed: the honest denominator in the report is "attempts
    /// made", not "attempts shown". The reason is structured — the surface
    /// renders it from its catalogue.
    CandidateRejected {
        job: JobId,
        /// 1-based, as [`Stage::Rewriting`] counts.
        chunk: u32,
        round: u8,
        candidate: u8,
        rejection: Rejection,
    },
    /// One chunk was decided — rewritten, or kept with the reason why. Only
    /// a job started by [`start_resumable`] sends it, once per chunk it
    /// asked about (never for a carried one), as soon as the decision is
    /// made: what a caller stores so that a `kill -9` a moment later costs
    /// at most the chunk in flight.
    ChunkDecided {
        job: JobId,
        /// 1-based, as [`Stage::Rewriting`] counts.
        chunk: u32,
        decided: Box<Decided>,
    },
    /// A resumable job sorted the records it was handed: `carried` stand
    /// for their chunks, `discarded` did not fit this job (another
    /// document, other options, another engine or template, another
    /// chunk) and their chunks are asked again. Sent once, after planning,
    /// and only when anything was handed in.
    Resumed {
        job: JobId,
        carried: u32,
        discarded: u32,
    },
    /// The job ran to its end: the document and the report.
    Finished {
        job: JobId,
        elapsed: Duration,
        outcome: Box<Outcome>,
    },
    /// The job was cancelled. No document comes back: half a rewrite is
    /// not a result.
    Cancelled {
        job: JobId,
        elapsed: Duration,
    },
    /// The job could not go on.
    Failed {
        job: JobId,
        elapsed: Duration,
        error: PipelineError,
    },
}

/// Why a job stopped before its end. A value: the surface words it.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PipelineError {
    /// Nothing can answer — no model built in, no file, no key. Asking
    /// again for every candidate of every chunk would only repeat it.
    #[error("the engine is unavailable: {0}")]
    Unavailable(wipemark_engine::Unavailable),
    /// The engine failed while getting ready.
    #[error("the engine failed while getting ready: {0:?}")]
    Engine(EngineFailure),
    /// A shipped template does not render: a bug in this build.
    #[error("the shipped template {slot:?} does not render")]
    ShippedTemplate { slot: prompt::Slot },
    /// The winners could not be put back: a bug, since every winner was
    /// restored once already.
    #[error("assembly failed: {0}")]
    Assemble(prepare::AssembleError),
}
