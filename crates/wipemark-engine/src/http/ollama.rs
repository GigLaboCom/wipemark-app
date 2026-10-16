//! Ollama's native wire format: what is sent to `{base}/api/chat`, and what
//! one streamed line says (D59).
//!
//! Native, not `/api/generate` and not the `/v1` shim Ollama also serves:
//! the sampling knobs travel inside `options`, which is Ollama's spelling
//! and nobody else's, and the answer streams as one JSON object per line.
//! No `Authorization` header is ever sent here — the transport refuses to
//! build one for this provider. `reasoning_effort` and `min_p` are not
//! sent: the first is not part of this API, and the second is not portable.
//!
//! # Thinking off (D709)
//!
//! Every request carries `"think": false` at the top level. A thinking
//! model under Ollama — Gemma 4 imported from its GGUF, Qwen3 — otherwise
//! streams its answer in `message.thinking` and leaves `message.content`
//! empty, so every candidate came back empty (acceptance F20). We rewrite
//! with thinking off, as the local engine does (D181, D182). Only here:
//! the OpenAI-compatible request stays as it was, because the `/v1` shim
//! takes no `think`, and a field a server refuses would be a new failure.
//!
//! What a server that ignores the field still streams as `thinking` is
//! counted — the pieces and their length — and never kept: not in the
//! answer, not in an error, not in a log line (D710).

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
        "think": false,
        "messages": messages,
        "options": options,
    })
}

/// One line: `message.content`, `done`, `done_reason`, and `eval_count` —
/// the tokens generated — on the last. `message.thinking` is measured, in
/// characters, and dropped.
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
    let thought = value
        .pointer("/message/thinking")
        .and_then(Value::as_str)
        .map_or(0, |thinking| thinking.chars().count() as u64);
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
        thought,
    })
}
