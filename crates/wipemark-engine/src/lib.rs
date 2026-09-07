//! `wipemark-engine` — the rewriting side of the product, behind one
//! trait.
//!
//! Layer B is best-effort by construction: a model rewrites the text and
//! we score what came back. Everything model-shaped lives behind
//! [`RewriteEngine`] so the pipeline never learns whether it is talking
//! to a local GGUF, an OpenAI-compatible endpoint, or
//! [`fake::FakeEngine`].
//!
//! Three implementations are planned (spec §4.1):
//!
//! * `OpenAiCompatEngine` — first to ship, testable against a fake HTTP
//!   server. Redirects are refused outright (an `Authorization` header
//!   must not follow a redirect to an unvalidated host) and a
//!   non-loopback `base_url` requires `allow_remote = true`.
//! * `LlamaEngine` — behind the `local-llama` feature, on a dedicated
//!   blocking thread, checking `cancel` between decode steps.
//! * [`fake::FakeEngine`] — deterministic, no I/O; the pipeline and UI
//!   gates run on it, including on machines with no GPU.
//!
//! # Cancellation
//!
//! Every call takes a [`CancellationToken`]. Cancellation is not
//! best-effort: the UI's Cancel button has to stop a running decode
//! within half a second (epic E2 gate), so implementations check the
//! token between token emissions, not only between requests.

#![forbid(unsafe_code)]

pub mod fake;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use wipemark_core::Vendor;

/// Where streamed tokens go.
///
/// A `flume` sender rather than a callback: the receiving end is often
/// the GPUI executor, which cannot await a tokio future (spec §1.2).
/// Send failures mean the consumer went away — engines treat that as a
/// reason to stop, not as an error to report.
pub type TokenSink = flume::Sender<String>;

/// Which engine produced a result — recorded on every attempt in the
/// report, and the input to the non-origin rule (spec §4.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineInfo {
    pub vendor: Vendor,
    pub model_id: String,
    /// True when inference happens on this machine. Drives the "text
    /// leaves your machine" banner.
    pub local: bool,
    pub ctx_len: u32,
}

/// Sampling knobs. Every one of them is a config value, never a
/// hard-coded choice (spec §0.1 rule 2).
#[derive(Debug, Clone, PartialEq)]
pub struct SamplingParams {
    pub temperature: f32,
    pub top_p: f32,
    pub min_p: Option<f32>,
    /// Set per candidate as `base_seed + round * candidate`, so a run is
    /// reproducible from the report.
    pub seed: Option<u64>,
    pub max_tokens: Option<u32>,
}

impl Default for SamplingParams {
    fn default() -> Self {
        Self {
            temperature: 0.9,
            top_p: 0.95,
            min_p: None,
            seed: None,
            max_tokens: None,
        }
    }
}

/// One rewriting request: a rendered prompt, not a conversation.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatRequest {
    pub system: Option<String>,
    /// The tactic template with `{TEXT}` / `{LANG}` / `{PREV_CONTEXT}`
    /// already substituted.
    pub prompt: String,
    pub params: SamplingParams,
}

/// Why generation stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    Stop,
    Length,
    Cancelled,
}

/// The assembled result. `text` is also what was streamed into the sink,
/// so a consumer that ignored the sink loses nothing.
#[derive(Debug, Clone, PartialEq)]
pub struct Completion {
    pub text: String,
    pub tokens_out: u32,
    pub finish: FinishReason,
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("cancelled")]
    Cancelled,
    #[error("transport: {0}")]
    Transport(String),
    /// The endpoint answered, but not in a shape we can read.
    #[error("protocol: {0}")]
    Protocol(String),
    #[error("context overflow: {used} tokens over a {limit}-token window")]
    ContextOverflow { used: u32, limit: u32 },
    /// Refused before doing anything — no model loaded, not enough RAM,
    /// remote endpoint without `allow_remote`.
    #[error("unavailable: {0}")]
    Unavailable(String),
    #[error("not implemented yet: {0}")]
    NotImplemented(&'static str),
}

/// The one thing every rewriting backend has to do.
#[async_trait]
pub trait RewriteEngine: Send + Sync {
    fn info(&self) -> EngineInfo;

    /// Generate one completion, streaming tokens into `sink`.
    ///
    /// Implementations must return [`EngineError::Cancelled`] promptly
    /// once `cancel` fires, and must not leave a partially decoded
    /// model in a state the next call inherits.
    async fn complete(
        &self,
        req: ChatRequest,
        sink: TokenSink,
        cancel: CancellationToken,
    ) -> Result<Completion, EngineError>;

    /// Load weights / open the connection and measure the cost, so the
    /// UI can show a real estimate before a batch starts.
    async fn warmup(&self) -> Result<(), EngineError>;

    /// Release the model. Called on the idle timeout — a loaded 8B model
    /// holds gigabytes that the user did not agree to lend us
    /// indefinitely.
    async fn unload(&self);
}
