//! The OpenAI-compatible wire format: what is sent to
//! `{base}/v1/chat/completions`, and what one streamed chunk says (D59).
//!
//! Upstream's request (`docs/sdd/layer-b-rewrite-reference.md` §1) is
//! `model`, `messages`, `temperature` and `reasoning_effort`. Ours adds
//! `stream: true` (the answer arrives as server-sent events), a `system`
//! message when the request has one, `top_p`, `seed` and `max_tokens` —
//! the knobs spec §4.1 and §4.4 need. `min_p` is not sent: it is not part
//! of the OpenAI API, and a server that rejects unknown fields would refuse
//! the request for a knob nobody here depends on.

use serde_json::{json, Map, Value};

use super::sse::{json as parsed, reported, Said};
use super::{number, Reasoning};
use crate::{ChatRequest, EngineError, FinishReason};

/// The request body.
pub(crate) fn body(model: &str, reasoning: Reasoning, req: &ChatRequest) -> Value {
    let mut messages = Vec::new();
    if let Some(system) = &req.system {
        messages.push(json!({ "role": "system", "content": system }));
    }
    messages.push(json!({ "role": "user", "content": req.prompt }));

    let mut body = Map::new();
    body.insert("model".into(), json!(model));
    body.insert("stream".into(), json!(true));
    body.insert("messages".into(), Value::Array(messages));
    body.insert("temperature".into(), number(req.params.temperature));
    body.insert("top_p".into(), number(req.params.top_p));
    if let Some(seed) = req.params.seed {
        body.insert("seed".into(), json!(seed));
    }
    if let Some(max_tokens) = req.params.max_tokens {
        body.insert("max_tokens".into(), json!(max_tokens));
    }
    if let Some(effort) = reasoning.wire() {
        body.insert("reasoning_effort".into(), json!(effort));
    }
    Value::Object(body)
}

/// One chunk: `choices[0].delta.content`, its `finish_reason`, and
/// `usage.completion_tokens` when the server counts.
pub(crate) fn payload(data: &str) -> Result<Said, EngineError> {
    let value = parsed(data)?;
    if let Some(error) = value.get("error") {
        return Err(reported(error));
    }
    let choice = value.get("choices").and_then(|choices| choices.get(0));
    let piece = choice
        .and_then(|choice| choice.pointer("/delta/content"))
        .and_then(Value::as_str)
        .filter(|piece| !piece.is_empty())
        .map(str::to_owned);
    let finish = choice
        .and_then(|choice| choice.get("finish_reason"))
        .and_then(Value::as_str)
        .map(|reason| match reason {
            "length" => FinishReason::Length,
            _ => FinishReason::Stop,
        });
    let tokens = value
        .pointer("/usage/completion_tokens")
        .and_then(Value::as_u64)
        .and_then(|tokens| u32::try_from(tokens).ok());
    Ok(Said {
        piece,
        finish,
        tokens,
        done: false,
    })
}
