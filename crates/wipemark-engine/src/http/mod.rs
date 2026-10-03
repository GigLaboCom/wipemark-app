//! The endpoint: Ollama's native API, or any OpenAI-compatible server,
//! over HTTP.
//!
//! [`HttpEngine`] is the second thing that can rewrite (E2-3, spec S2.2).
//! It is built by the application's `duty::engine_for` from a decision
//! that has already passed `engine::refusal` — default-deny past this
//! machine, no key across a plaintext hop — and it adds the transport's own
//! rules on top ([`wire`]): `http`/`https` only, no redirect followed,
//! retries only before the first byte.
//!
//! # Threads, not a runtime (D58)
//!
//! A request runs on a thread of its own with the blocking `ureq` client
//! the downloader already uses; the trait's `async fn` awaits a `flume`
//! channel, so the engine is awaited from GPUI's executor or from tokio
//! alike and starts no runtime of its own. Every piece the thread reads is
//! handed to the awaiting side, which streams it into the caller's sink —
//! so once [`RewriteEngine::complete`] has returned, nothing more reaches
//! that sink.
//!
//! **Cancel** answers the caller at once with [`EngineError::Cancelled`].
//! The request's thread notices at its next chunk, or at its read timeout,
//! and drops the connection then; a server that has gone quiet holds a
//! thread for at most the timeout, never the caller.
//!
//! # The key (D57)
//!
//! Held as a [`Secret`] from the credential store to the header: no
//! `Display`, no `Serialize`, a `Debug` that prints `<secret>`. It is sent
//! only to an OpenAI-compatible endpoint, only as `Authorization: Bearer`,
//! and [`Secret::expose`] is called in one place — where that header is
//! built. Never in the URL, the body, a log line or an error.

mod ollama;
mod openai;
mod sse;
mod wire;

#[cfg(test)]
mod fake_server;
#[cfg(test)]
mod tests;

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;
use tokio_util::sync::CancellationToken;
use wipemark_core::Vendor;
use wipemark_secret::Secret;

use crate::{ChatRequest, Completion, EngineError, EngineInfo, RewriteEngine, TokenSink};

/// Which wire format a request is sent in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpProvider {
    /// `POST {base}/v1/chat/completions`, streamed as server-sent events.
    /// The one that takes a key.
    OpenAiCompatible,
    /// Ollama's native `POST {base}/api/chat`, streamed as one JSON object
    /// per line. Never sent an `Authorization` header.
    Ollama,
}

/// `reasoning_effort`, as the application's `ReasoningEffort` row spells
/// it. `None` sends `"none"`; `Off` leaves the field out — the only thing
/// that works on a server that rejects the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reasoning {
    Off,
    None,
    Low,
    Medium,
    High,
}

impl Reasoning {
    /// What goes in the body, or `None` to leave the field out.
    pub fn wire(self) -> Option<&'static str> {
        match self {
            Self::Off => None,
            Self::None => Some("none"),
            Self::Low => Some("low"),
            Self::Medium => Some("medium"),
            Self::High => Some("high"),
        }
    }
}

/// Everything one endpoint engine sends with.
#[derive(Debug, Clone)]
pub struct HttpConfig {
    pub provider: HttpProvider,
    /// The full URL: the base the user typed plus the provider's path,
    /// built once by the application.
    pub endpoint: String,
    /// Scheme, host and port — for [`EngineInfo`] and log lines.
    pub origin: String,
    /// The name the endpoint knows the model by.
    pub model: String,
    /// The key, for an OpenAI-compatible endpoint. Ignored — never sent —
    /// for Ollama.
    pub key: Option<Secret>,
    pub reasoning: Reasoning,
    /// How long the server may stay silent: before its headers, or between
    /// two pieces of an answer.
    pub timeout: Duration,
    /// Whether the endpoint is this machine.
    pub on_this_machine: bool,
    pub vendor: Vendor,
}

/// [`RewriteEngine`] over an HTTP endpoint.
pub struct HttpEngine {
    config: Arc<HttpConfig>,
    agent: ureq::Agent,
}

impl fmt::Debug for HttpEngine {
    /// The configuration — whose key prints as `<secret>` — and not the
    /// agent.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpEngine")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl HttpEngine {
    /// Build the engine. Sends nothing and opens nothing: the first
    /// request does.
    pub fn new(config: HttpConfig) -> HttpEngine {
        let agent = wire::agent(config.timeout);
        HttpEngine {
            config: Arc::new(config),
            agent,
        }
    }
}

/// What a request's thread hands the awaiting side.
enum Msg {
    Piece(String),
    Done(Result<Completion, EngineError>),
}

#[async_trait]
impl RewriteEngine for HttpEngine {
    fn info(&self) -> EngineInfo {
        EngineInfo {
            vendor: self.config.vendor,
            model_id: self.config.model.clone(),
            local: self.config.on_this_machine,
            // The server's business, and not asked for (spec §7).
            ctx_len: None,
        }
    }

    async fn complete(
        &self,
        req: ChatRequest,
        sink: TokenSink,
        cancel: CancellationToken,
    ) -> Result<Completion, EngineError> {
        let (out, inbox) = flume::unbounded::<Msg>();
        // Never sent on: dropping it, when this call returns for any
        // reason, is how the request's thread learns the caller has gone.
        let (_caller, gone) = flume::bounded::<()>(1);
        let config = Arc::clone(&self.config);
        let agent = self.agent.clone();
        std::thread::Builder::new()
            .name("wipemark-http".to_owned())
            .spawn(move || {
                let result = request(&config, &agent, &req, &gone, &out);
                let _ = out.send(Msg::Done(result));
            })
            .map_err(|error| {
                EngineError::Transport(format!("could not start the request's thread: {error}"))
            })?;

        loop {
            match cancel.run_until_cancelled(inbox.recv_async()).await {
                None => return Err(EngineError::Cancelled),
                Some(Ok(Msg::Piece(piece))) => {
                    // A consumer that went away loses the stream, not the
                    // answer: the completion still carries the whole text.
                    let _ = sink.send(piece);
                }
                Some(Ok(Msg::Done(result))) => return result,
                Some(Err(_)) => {
                    return Err(EngineError::Transport(
                        "the request's thread stopped".to_owned(),
                    ))
                }
            }
        }
    }

    /// Nothing is loaded on the other side by us; the Check is the
    /// connection test.
    async fn warmup(&self) -> Result<(), EngineError> {
        Ok(())
    }

    /// Nothing to release.
    async fn unload(&self) {}
}

/// One request, on its own thread.
fn request(
    config: &HttpConfig,
    agent: &ureq::Agent,
    req: &ChatRequest,
    gone: &flume::Receiver<()>,
    out: &flume::Sender<Msg>,
) -> Result<Completion, EngineError> {
    let (body, accept, key, mut format): (Value, _, _, Box<dyn sse::Format>) = match config.provider
    {
        HttpProvider::OpenAiCompatible => (
            openai::body(&config.model, config.reasoning, req),
            "text/event-stream",
            config.key.as_ref(),
            Box::new(sse::Sse::default()),
        ),
        HttpProvider::Ollama => (
            ollama::body(&config.model, req),
            "application/x-ndjson",
            None,
            Box::new(sse::Ndjson),
        ),
    };
    let body = serde_json::to_vec(&body).map_err(|error| {
        EngineError::Protocol(format!("the request did not serialise: {error}"))
    })?;
    let request = wire::Request {
        agent,
        endpoint: &config.endpoint,
        origin: &config.origin,
        accept,
        key,
        body,
        timeout: config.timeout,
    };
    let mut forward = |piece: String| out.send(Msg::Piece(piece)).is_ok();
    wire::exchange(&request, format.as_mut(), &wire::Stop(gone), &mut forward)
}

/// A sampling knob as JSON: `0.9`, not the `0.8999999761581421` an `f32`
/// widens to. A value that is not a number is `null`, which a server
/// refuses by name rather than reading as something else.
fn number(value: f32) -> Value {
    value
        .to_string()
        .parse::<f64>()
        .ok()
        .and_then(serde_json::Number::from_f64)
        .map_or(Value::Null, Value::Number)
}
