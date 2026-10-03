//! `wipemark-engine` — the rewriting side of the product, behind one
//! trait.
//!
//! Layer B is best-effort by construction: a model rewrites the text and
//! we score what came back. Everything model-shaped lives behind
//! [`RewriteEngine`] so the pipeline never learns whether it is talking
//! to a local GGUF, an OpenAI-compatible endpoint, or
//! [`fake::FakeEngine`].
//!
//! Three implementations (spec §4.1):
//!
//! * [`http::HttpEngine`] — an endpoint over HTTP: Ollama's native
//!   `/api/chat` or any OpenAI-compatible `/v1/chat/completions`, streamed,
//!   on a thread per request with the blocking `ureq` client (D58). It
//!   refuses a non-`http(s)` scheme and never follows a redirect (an
//!   `Authorization` header must not follow one to an unvalidated host);
//!   that a non-loopback endpoint needs `allow_remote` is the application's
//!   rule (`engine::refusal`), decided before the engine is built. Tested
//!   against a fake server on `127.0.0.1:0` and nothing else.
//! * `local::LocalEngine` — behind the `local-llama` feature: a GGUF on
//!   this machine, owned by one worker thread, `cancel` read between
//!   decode steps. Its llama.cpp is `wipemark-llama`, code copied into
//!   this repository from a closed project at a named commit and pinned to
//!   one llama.cpp commit (D45, `crates/wipemark-llama-sys/PIN.md`,
//!   `docs/architecture/local-engine.md`). `local-llama` alone compiles it
//!   over a shim that refuses every load; `llama-native` builds llama.cpp
//!   into it. The application's `duty::engine_for` hands it out, and its
//!   `EngineHost` decides when it is loaded (E2-2).
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
pub mod http;
#[cfg(feature = "local-llama")]
pub mod local;

use std::path::PathBuf;

/// The attribute every [`RewriteEngine`] implementation is written under,
/// re-exported so an implementor — a test double in the application —
/// names the same macro this trait was declared with.
pub use async_trait::async_trait;
pub use http::{HttpConfig, HttpEngine, HttpProvider, Reasoning};
#[cfg(feature = "local-llama")]
pub use local::{has_gpu_backend, LocalConfig, LocalEngine};
/// The cancellation every call takes, re-exported so a caller names the
/// type the trait names.
pub use tokio_util::sync::CancellationToken;
use wipemark_core::Vendor;
/// What a local load asks llama.cpp for — a field of [`LocalConfig`],
/// re-exported so the application that builds one does not depend on the
/// llama.cpp layer itself.
#[cfg(feature = "local-llama")]
pub use wipemark_llama::LoadParams;

/// Where streamed tokens go.
///
/// A `flume` sender rather than a callback: the receiving end is often
/// the GPUI executor, which cannot await a tokio future (spec §1.2).
/// Send failures mean the consumer went away — engines treat that as a
/// reason to stop, not as an error to report.
pub type TokenSink = flume::Sender<String>;

/// Which engine produced a result — recorded on every attempt in the
/// report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineInfo {
    pub vendor: Vendor,
    pub model_id: String,
    /// True when inference happens on this machine. Drives the "text
    /// leaves your machine" banner.
    pub local: bool,
    /// The context window, when it is known.
    ///
    /// `None` is **unknown**, and never rendered as a number. Local
    /// weights carry the figure their catalogue entry records; an HTTP
    /// endpoint's window is the server's business, and a settings page
    /// that reported one would be reporting a guess. A zero here would
    /// read as "no context" to every consumer that did arithmetic on
    /// it, which is the same mistake `Host::vram_mb` exists as an
    /// `Option` to avoid.
    pub ctx_len: Option<u32>,
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
    /// Refused before doing anything, and why — as a value, so that the
    /// surface that shows it chooses the sentence (D53).
    #[error("unavailable: {0}")]
    Unavailable(Unavailable),
    #[error("not implemented yet: {0}")]
    NotImplemented(&'static str),
}

/// Why an engine cannot do anything at all.
///
/// Structured, because only applications localize: a library that handed
/// up an English sentence would leave every surface that is not in English
/// quoting it (D53). The `Display` form is for a log line and stays
/// English; a window renders each variant from its own catalogue.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Unavailable {
    /// This build has no llama.cpp in it, so it cannot load a model.
    #[error("built without llama.cpp: this build cannot load a model")]
    NotBuilt,
    /// The weights file is not there.
    #[error("no model file at {}", .path.display())]
    NoSuchFile { path: PathBuf },
    /// The load's estimate is larger than the memory the caller said is
    /// available.
    #[error("the model needs about {need_mb} MiB and {have_mb} MiB is available")]
    WouldNotFit { need_mb: u64, have_mb: u64 },
    /// No ggml backend registered — not even the CPU — so there is
    /// nothing to load a model onto.
    #[error("no ggml backend could be registered")]
    NoBackend,
    /// llama.cpp would not load the model, or a context for it. `detail`
    /// is llama.cpp's own words (or names the call that failed): shown as
    /// a detail beside the sentence, and never translated.
    #[error("the model could not be loaded: {detail}")]
    LoadFailed { detail: String },
    /// The engine's worker thread has gone — it panicked, or could not be
    /// started. Permanent for that engine; a new one has to be built.
    #[error("the local engine's worker has stopped")]
    Stopped,
    /// Nothing is on duty to answer: no engine has been built for the
    /// role, so there is nobody to ask. The application's `EngineHandle`
    /// is the one site that says this.
    #[error("nothing is on duty")]
    NothingOnDuty,
    /// The endpoint answered with a redirect, which is never followed.
    /// `to_origin` is where it pointed — scheme, host and port, never a
    /// path or a query — when it said.
    #[error("the endpoint answered {status} and pointed {}; redirects are not followed", .to_origin.as_deref().unwrap_or("nowhere"))]
    Redirected {
        status: u16,
        to_origin: Option<String>,
    },
    /// 401 or 403: the endpoint did not accept the key, or wants one.
    #[error("the endpoint refused the key ({status})")]
    KeyRejected { status: u16 },
    /// 404: a wrong path, or a model the server does not have (for Ollama,
    /// one not pulled). `detail` is the server's own words, at most 2 KiB.
    #[error("the endpoint has no such model or path: {detail}")]
    NotFound { detail: String },
    /// 429, still, after the retries. `retry_after_s` is what the server
    /// last asked for.
    #[error("the endpoint is rate-limiting requests")]
    RateLimited { retry_after_s: Option<u32> },
    /// Any other error status, after the retries a 502/503/504 gets.
    /// `detail` is the server's own words, at most 2 KiB.
    #[error("the endpoint refused the request ({status}): {detail}")]
    Refused { status: u16, detail: String },
    /// The credential store would not say whether there is a key. Not "no
    /// key": a locked keychain and a missing key have different fixes.
    /// `reason` is the store's own words.
    #[error("the key could not be read: {reason}")]
    KeyUnreadable { reason: String },
    /// The provider sends a key and the credential store has none for this
    /// endpoint.
    #[error("no key is stored for this endpoint")]
    NoKey,
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
