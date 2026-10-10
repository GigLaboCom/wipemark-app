//! Ollama's native wire format: what is sent to `{base}/api/chat`, and what
//! one streamed line says (D59).
//!
//! Native, not `/api/generate` and not the `/v1` shim Ollama also serves:
//! the sampling knobs travel inside `options`, which is Ollama's spelling
//! and nobody else's, and the answer streams as one JSON object per line.
//! No `Authorization` header is ever sent here — the transport refuses to
//! build one for this provider. `reasoning_effort` and `min_p` are not
//! sent: the first is not part of this API, and the second is not portable.

use serde_json::{json, Map, Value};

use super::number;
use super::sse::{json as parsed, reported, Said};
use crate::{ChatRequest, EngineError, FinishReason};

/// The request body.
pub(crate) fn body(model: &str, req: &ChatRequest) -> Value {
    let mut messages = Vec::new();
    if let Some(system) = &req.system {
        messages.push(json!({ "role": "system", "content": system }));
    }
    messages.push(json!({ "role": "user", "content": req.prompt }));

    let mut options = Map::new();
    options.insert("temperature".into(), number(req.params.temperature));
    options.insert("top_p".into(), number(req.params.top_p));
    if let Some(seed) = req.params.seed {
        options.insert("seed".into(), json!(seed));
    }
    if let Some(max_tokens) = req.params.max_tokens {
        options.insert("num_predict".into(), json!(max_tokens));
    }
    json!({
        "model": model,
        "stream": true,
        "messages": messages,
        "options": options,
    })
}

/// One line: `message.content`, `done`, `done_reason`, and `eval_count` —
/// the tokens generated — on the last.
pub(crate) fn payload(line: &str) -> Result<Said, EngineError> {
    let value = parsed(line)?;
    if let Some(error) = value.get("error") {
        return Err(reported(error));
    }
    let piece = value
        .pointer("/message/content")
        .and_then(Value::as_str)
        .filter(|piece| !piece.is_empty())
        .map(str::to_owned);
    let done = value.get("done").and_then(Value::as_bool) == Some(true);
    let finish = done.then(|| match value.get("done_reason").and_then(Value::as_str) {
        Some("length") => FinishReason::Length,
        _ => FinishReason::Stop,
    });
    let tokens = value
        .get("eval_count")
        .and_then(Value::as_u64)
        .and_then(|tokens| u32::try_from(tokens).ok());
    Ok(Said {
        piece,
        finish,
        tokens,
        done,
    })
}
