//! A deterministic engine that does no I/O.
//!
//! Epic E0 / S0.5. It exists so the pipeline, the guards and the UI can
//! be gated without a GPU, without weights and without a network — the
//! same role `fake-TTS` plays in the ml stack. Determinism is the point:
//! same request, same seed, same bytes out, on every machine.
//!
//! It is not a stub. It streams token by token, honours cancellation
//! between tokens, and reports a real [`FinishReason`] — the pipeline
//! cannot tell it apart from a slow remote endpoint except by the
//! output being boring.

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use wipemark_core::Vendor;

use crate::{
    ChatRequest, Completion, EngineError, EngineInfo, FinishReason, RewriteEngine, TokenSink,
};

/// SplitMix64. Small, well-distributed, and — unlike
/// `DefaultHasher` — guaranteed to produce the same numbers after a
/// `rustup update`, which is what "deterministic fixture" has to mean.
fn mix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// FNV-1a over the prompt, so the output depends on the input as well as
/// on the seed.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xCBF2_9CE4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

/// Deterministic no-I/O [`RewriteEngine`].
#[derive(Debug, Clone)]
pub struct FakeEngine {
    model_id: String,
    ctx_len: u32,
}

impl FakeEngine {
    pub fn new() -> Self {
        Self {
            model_id: "fake-deterministic".to_owned(),
            ctx_len: 8192,
        }
    }

    /// Pose as a specific model — for a test that needs an engine with
    /// another id or window.
    pub fn with_model(model_id: impl Into<String>, ctx_len: u32) -> Self {
        Self {
            model_id: model_id.into(),
            ctx_len,
        }
    }

    /// The transformation itself: rotate the word sequence by a
    /// seed-derived offset. It preserves every word (so the guards pass)
    /// while changing the bigram distribution (so divergence is
    /// non-zero) — exactly the two properties the pipeline gates need.
    ///
    /// Deliberately not a method: the rotation depends only on the
    /// request, so two `FakeEngine`s with different model ids produce
    /// identical output for identical input.
    fn rotation(req: &ChatRequest, word_count: usize) -> usize {
        if word_count == 0 {
            return 0;
        }
        let seed = req.params.seed.unwrap_or(0) ^ fnv1a(req.prompt.as_bytes());
        (mix64(seed) % word_count as u64) as usize
    }
}

impl Default for FakeEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RewriteEngine for FakeEngine {
    fn info(&self) -> EngineInfo {
        EngineInfo {
            vendor: Vendor::OpenLlm,
            model_id: self.model_id.clone(),
            local: true,
            ctx_len: Some(self.ctx_len),
        }
    }

    async fn complete(
        &self,
        req: ChatRequest,
        sink: TokenSink,
        cancel: CancellationToken,
    ) -> Result<Completion, EngineError> {
        let words: Vec<&str> = req.prompt.split_whitespace().collect();
        let rotation = Self::rotation(&req, words.len());

        let mut text = String::with_capacity(req.prompt.len());
        let mut tokens_out = 0_u32;

        for (i, word) in words
            .iter()
            .cycle()
            .skip(rotation)
            .take(words.len())
            .enumerate()
        {
            // Checked per token, not per request: the UI's Cancel has
            // half a second to take effect, and a long generation must
            // not run to completion first.
            if cancel.is_cancelled() {
                return Err(EngineError::Cancelled);
            }
            let piece = if i == 0 {
                (*word).to_owned()
            } else {
                format!(" {word}")
            };
            text.push_str(&piece);
            tokens_out += 1;
            // A closed receiver means nobody is listening any more. That
            // is not an error — finish assembling `text` and return it.
            let _ = sink.send(piece);
        }

        Ok(Completion {
            text,
            tokens_out,
            finish: FinishReason::Stop,
        })
    }

    async fn warmup(&self) -> Result<(), EngineError> {
        Ok(())
    }

    async fn unload(&self) {}
}

#[cfg(test)]
mod tests {
    use tokio_util::sync::CancellationToken;

    use super::FakeEngine;
    use crate::{ChatRequest, EngineError, FinishReason, RewriteEngine, SamplingParams};

    fn request(prompt: &str, seed: u64) -> ChatRequest {
        ChatRequest {
            system: None,
            prompt: prompt.to_owned(),
            params: SamplingParams {
                seed: Some(seed),
                ..SamplingParams::default()
            },
        }
    }

    const FIXTURE: &str = "the quick brown fox jumps over the lazy dog near the river bank";

    #[tokio::test]
    async fn same_seed_same_bytes() {
        let engine = FakeEngine::new();
        let (tx, _rx) = flume::unbounded();

        let first = engine
            .complete(request(FIXTURE, 7), tx.clone(), CancellationToken::new())
            .await
            .expect("fake engine never fails without cancellation");
        let second = engine
            .complete(request(FIXTURE, 7), tx, CancellationToken::new())
            .await
            .expect("fake engine never fails without cancellation");

        assert_eq!(first.text, second.text);
        assert_eq!(first.finish, FinishReason::Stop);
    }

    /// The guards must be able to pass a fake candidate — otherwise every
    /// pipeline gate would be testing the guards' rejection path only.
    #[tokio::test]
    async fn output_preserves_every_word() {
        let engine = FakeEngine::new();
        let (tx, _rx) = flume::unbounded();
        let out = engine
            .complete(request(FIXTURE, 3), tx, CancellationToken::new())
            .await
            .unwrap();

        let mut source: Vec<&str> = FIXTURE.split_whitespace().collect();
        let mut result: Vec<&str> = out.text.split_whitespace().collect();
        source.sort_unstable();
        result.sort_unstable();
        assert_eq!(source, result);
        assert_eq!(out.tokens_out as usize, source.len());
    }

    #[tokio::test]
    async fn streams_one_piece_per_token() {
        let engine = FakeEngine::new();
        let (tx, rx) = flume::unbounded();
        let out = engine
            .complete(request(FIXTURE, 1), tx, CancellationToken::new())
            .await
            .unwrap();

        let streamed: String = rx.drain().collect();
        assert_eq!(streamed, out.text);
    }

    #[tokio::test]
    async fn cancellation_is_reported_not_swallowed() {
        let engine = FakeEngine::new();
        let (tx, _rx) = flume::unbounded();
        let cancel = CancellationToken::new();
        cancel.cancel();

        let err = engine
            .complete(request(FIXTURE, 1), tx, cancel)
            .await
            .expect_err("a cancelled call must not return a completion");
        assert!(matches!(err, EngineError::Cancelled));
    }
}
