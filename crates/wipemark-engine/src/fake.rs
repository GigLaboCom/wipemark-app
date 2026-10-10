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
//!
//! Two modes. By default it rotates the words of the prompt. Scripted
//! ([`FakeEngine::answering`], E4-3) it answers each request with what a
//! test's closure returns — the answer that drops a placeholder, carries
//! a U+200B, changes nothing, or comes back out of order — streamed the
//! same way, slowed down on request ([`FakeEngine::with_token_delay`]) so
//! a cancel can land mid-stream, and cut at `max_tokens` with
//! [`FinishReason::Length`]. Every request is remembered
//! ([`FakeEngine::asked`]), so a test can count calls and read seeds.

use std::sync::{Arc, Mutex};
use std::time::Duration;

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

/// What a scripted [`FakeEngine`] answers: the request, and its number
/// among the requests this engine (and its clones) was sent, from 0.
pub type Script = Arc<dyn Fn(&ChatRequest, usize) -> String + Send + Sync>;

/// Deterministic no-I/O [`RewriteEngine`].
#[derive(Clone)]
pub struct FakeEngine {
    model_id: String,
    ctx_len: u32,
    /// `None`: rotate the prompt's words. `Some`: answer with the script.
    script: Option<Script>,
    /// Slept between two streamed pieces — a slow stream, for a cancel.
    token_delay: Option<Duration>,
    /// Every request this engine was sent, in order, shared by its clones.
    asked: Arc<Mutex<Vec<ChatRequest>>>,
}

impl std::fmt::Debug for FakeEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FakeEngine")
            .field("model_id", &self.model_id)
            .field("ctx_len", &self.ctx_len)
            .field("scripted", &self.script.is_some())
            .field("token_delay", &self.token_delay)
            .finish_non_exhaustive()
    }
}

impl FakeEngine {
    pub fn new() -> Self {
        Self {
            model_id: "fake-deterministic".to_owned(),
            ctx_len: 8192,
            script: None,
            token_delay: None,
            asked: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Pose as a specific model — for a test that needs an engine with
    /// another id or window.
    pub fn with_model(model_id: impl Into<String>, ctx_len: u32) -> Self {
        Self {
            model_id: model_id.into(),
            ctx_len,
            ..Self::new()
        }
    }

    /// Answer every request with `script(request, call)` instead of the
    /// rotation — streamed in whitespace-inclusive pieces, one token each.
    /// For a test that needs a particular answer: one that drops a
    /// placeholder, carries an invisible character, or changes nothing.
    pub fn answering(
        script: impl Fn(&ChatRequest, usize) -> String + Send + Sync + 'static,
    ) -> Self {
        Self {
            script: Some(Arc::new(script)),
            ..Self::new()
        }
    }

    /// Sleep `delay` before every streamed piece: a stream slow enough to
    /// be cancelled in the middle.
    pub fn with_token_delay(mut self, delay: Duration) -> Self {
        self.token_delay = Some(delay);
        self
    }

    /// Every request this engine and its clones were sent, in order.
    pub fn asked(&self) -> Vec<ChatRequest> {
        self.asked
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
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
            draft: None,
        }
    }

    async fn complete(
        &self,
        req: ChatRequest,
        sink: TokenSink,
        cancel: CancellationToken,
    ) -> Result<Completion, EngineError> {
        let call = {
            let mut asked = self
                .asked
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            asked.push(req.clone());
            asked.len() - 1
        };
        let pieces: Vec<String> = match &self.script {
            Some(script) => script(&req, call)
                .split_inclusive(char::is_whitespace)
                .map(str::to_owned)
                .collect(),
            None => {
                let words: Vec<&str> = req.prompt.split_whitespace().collect();
                let rotation = Self::rotation(&req, words.len());
                words
                    .iter()
                    .cycle()
                    .skip(rotation)
                    .take(words.len())
                    .enumerate()
                    .map(|(i, word)| {
                        if i == 0 {
                            (*word).to_owned()
                        } else {
                            format!(" {word}")
                        }
                    })
                    .collect()
            }
        };
        let limit = req.params.max_tokens.map_or(usize::MAX, |n| n as usize);

        let mut text = String::with_capacity(req.prompt.len());
        let mut tokens_out = 0_u32;

        for piece in pieces.iter().take(limit) {
            if let Some(delay) = self.token_delay {
                std::thread::sleep(delay);
            }
            // Checked per token, not per request: the UI's Cancel has
            // half a second to take effect, and a long generation must
            // not run to completion first.
            if cancel.is_cancelled() {
                return Err(EngineError::Cancelled);
            }
            text.push_str(piece);
            tokens_out += 1;
            // A closed receiver means nobody is listening any more. That
            // is not an error — finish assembling `text` and return it.
            let _ = sink.send(piece.clone());
        }

        Ok(Completion {
            text,
            tokens_out,
            finish: if pieces.len() > limit {
                FinishReason::Length
            } else {
                FinishReason::Stop
            },
            drafted: None,
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
    async fn a_scripted_engine_streams_its_answer_exactly_and_remembers_the_request() {
        let engine = FakeEngine::answering(|req, call| format!("call {call}: {}\n", req.prompt));
        let (tx, rx) = flume::unbounded();
        let out = engine
            .clone()
            .complete(request("two  words", 9), tx, CancellationToken::new())
            .await
            .unwrap();

        assert_eq!(out.text, "call 0: two  words\n");
        assert_eq!(rx.drain().collect::<String>(), out.text);
        assert_eq!(out.tokens_out, 5, "whitespace-inclusive pieces");
        assert_eq!(out.finish, FinishReason::Stop);
        let asked = engine.asked();
        assert_eq!(asked.len(), 1, "a clone shares what was asked");
        assert_eq!(asked[0].params.seed, Some(9));
    }

    #[tokio::test]
    async fn max_tokens_cuts_the_answer_and_says_so() {
        let engine = FakeEngine::answering(|_, _| "one two three four".to_owned());
        let mut req = request("x", 1);
        req.params.max_tokens = Some(2);
        let (tx, _rx) = flume::unbounded();
        let out = engine
            .complete(req, tx, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(out.text, "one two ");
        assert_eq!(out.finish, FinishReason::Length);
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
