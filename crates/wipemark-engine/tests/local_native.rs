//! `LocalEngine` against a real GGUF — the E2 gates that need a model.
//!
//! Compiled only under `--features llama-native`; every test is
//! `#[ignore]`d and reads the model's path from `WIPEMARK_TEST_GGUF`:
//!
//! ```sh
//! WIPEMARK_TEST_GGUF=/path/to/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf \
//!   cargo test -p wipemark-engine --features llama-native -- --ignored --test-threads=1
//! ```
//!
//! Run without the variable, each panics with the download instructions:
//! a gate that skipped is not a gate that passed.

#![cfg(feature = "llama-native")]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use tokio_util::sync::CancellationToken;
use wipemark_engine::{
    ChatRequest, Completion, EngineError, FinishReason, LocalConfig, LocalEngine, RewriteEngine,
    SamplingParams,
};
use wipemark_llama::{BackendKind, LoadParams, Runtime};

fn gguf() -> PathBuf {
    match std::env::var_os("WIPEMARK_TEST_GGUF") {
        Some(path) => PathBuf::from(path),
        None => panic!(
            "WIPEMARK_TEST_GGUF is not set. Download the catalogue's Qwen3 4B and point it there:\n  \
             curl -L -o Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf \
             https://huggingface.co/unsloth/Qwen3-4B-Instruct-2507-GGUF/resolve/a06e946bb6b655725eafa393f4a9745d460374c9/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf\n  \
             sha256 4bbe1f2f8ebe69fad3be8e15d69f220b06448a9dd26f82d7d81cce88ebfc39fd, 2546340960 bytes\n  \
             WIPEMARK_TEST_GGUF=$PWD/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf"
        ),
    }
}

fn engine(n_ctx: u32) -> LocalEngine {
    LocalEngine::new(LocalConfig {
        model_id: "qwen3-4b-instruct-2507-ud-q4".to_owned(),
        weights: gguf(),
        load: LoadParams {
            n_ctx,
            ..LoadParams::default()
        },
        available_mb: None,
    })
}

fn request(prompt: &str, temperature: f32, seed: u64, max_tokens: u32) -> ChatRequest {
    ChatRequest {
        system: None,
        prompt: prompt.to_owned(),
        params: SamplingParams {
            temperature,
            seed: Some(seed),
            max_tokens: Some(max_tokens),
            ..SamplingParams::default()
        },
    }
}

const CREATIVE: &str =
    "Invent a name for a colour nobody has seen and describe it in two sentences.";

async fn run(engine: &LocalEngine, req: ChatRequest) -> Completion {
    let (sink, _streamed) = flume::unbounded();
    engine
        .complete(req, sink, CancellationToken::new())
        .await
        .expect("the completion runs")
}

/// Start a long generation, cancel it `after` the first piece arrived — so
/// the cancel lands in the decode loop, not in the prompt — and return what
/// came back, how long the cancel took to land, and how many pieces were
/// streamed.
async fn cancelled_after(
    engine: &LocalEngine,
    after: Duration,
) -> (Result<Completion, EngineError>, Duration, usize) {
    let (sink, streamed) = flume::unbounded::<String>();
    let cancel = CancellationToken::new();
    let fired = std::sync::Arc::new(std::sync::Mutex::new(None::<Instant>));
    let timer = {
        let cancel = cancel.clone();
        let fired = std::sync::Arc::clone(&fired);
        let first = streamed.clone();
        tokio::spawn(async move {
            // Taken off the channel here; counted below.
            first
                .recv_async()
                .await
                .expect("the generation streamed a piece");
            tokio::time::sleep(after).await;
            *fired.lock().unwrap() = Some(Instant::now());
            cancel.cancel();
        })
    };
    let req = request(
        "Write a very long story, at least three thousand words, about a lighthouse keeper.",
        0.9,
        3,
        2000,
    );
    let result = engine.complete(req, sink, cancel).await;
    let returned = Instant::now();
    timer.await.unwrap();
    let fired = fired.lock().unwrap().expect("the cancel fired");
    (
        result,
        returned.saturating_duration_since(fired),
        1 + streamed.drain().count(),
    )
}

#[tokio::test]
#[ignore = "needs WIPEMARK_TEST_GGUF"]
async fn a_real_model_writes_a_sentence() {
    let engine = engine(4096);
    let (sink, streamed) = flume::unbounded();
    let out = engine
        .complete(
            ChatRequest {
                system: Some("You answer in one short sentence.".to_owned()),
                prompt: "What colour is the sky on a clear day?".to_owned(),
                params: SamplingParams {
                    temperature: 0.0,
                    max_tokens: Some(48),
                    ..SamplingParams::default()
                },
            },
            sink,
            CancellationToken::new(),
        )
        .await
        .expect("the completion runs");
    eprintln!("{out:?}");
    assert!(!out.text.trim().is_empty());
    assert!(matches!(
        out.finish,
        FinishReason::Stop | FinishReason::Length
    ));
    assert!(out.tokens_out > 0);
    let concatenated: String = streamed.drain().collect();
    assert_eq!(concatenated, out.text, "the sink and the text disagree");
}

#[tokio::test]
#[ignore = "needs WIPEMARK_TEST_GGUF"]
async fn cancel_stops_a_decode_within_half_a_second() {
    let engine = engine(4096);
    // Loaded first, so the second below is spent decoding, not loading.
    engine.warmup().await.expect("the model loads");

    let (result, latency, pieces) = cancelled_after(&engine, Duration::from_secs(1)).await;
    eprintln!(
        "LIVE cancel latency: {} ms after the fire ({pieces} pieces streamed)",
        latency.as_millis()
    );
    assert!(
        matches!(result, Err(EngineError::Cancelled)),
        "a cancelled call returned {result:?}"
    );
    assert!(
        pieces > 1,
        "the cancel landed before the decode was under way"
    );
    assert!(
        latency < Duration::from_millis(500),
        "the cancel took {} ms to land",
        latency.as_millis()
    );
}

#[tokio::test]
#[ignore = "needs WIPEMARK_TEST_GGUF"]
async fn a_queued_request_is_cancelled_without_waiting_for_the_one_ahead() {
    let engine = engine(4096);
    engine.warmup().await.expect("the model loads");
    let (ahead_sink, ahead_streamed) = flume::unbounded::<String>();
    let ahead_cancel = CancellationToken::new();
    let (queued_sink, queued_streamed) = flume::unbounded::<String>();
    let queued_cancel = CancellationToken::new();

    let ahead = engine.complete(
        request("Write a long story about a lighthouse keeper.", 0.9, 5, 300),
        ahead_sink,
        ahead_cancel.clone(),
    );
    let queued = async {
        // The one ahead is decoding before the second is even sent.
        ahead_streamed
            .recv_async()
            .await
            .expect("the request ahead streams");
        let waiting = engine.complete(
            request(CREATIVE, 0.9, 6, 32),
            queued_sink,
            queued_cancel.clone(),
        );
        let fire = async {
            tokio::time::sleep(Duration::from_millis(300)).await;
            let fired = Instant::now();
            queued_cancel.cancel();
            fired
        };
        let (result, fired) = tokio::join!(waiting, fire);
        let latency = fired.elapsed();
        // Let the one ahead go now that the measurement is taken.
        ahead_cancel.cancel();
        (result, latency)
    };
    let (ahead_result, (queued_result, latency)) = tokio::join!(ahead, queued);
    eprintln!("LIVE queued cancel latency: {} ms", latency.as_millis());
    assert!(
        matches!(queued_result, Err(EngineError::Cancelled)),
        "{queued_result:?}"
    );
    assert!(
        latency < Duration::from_millis(500),
        "a queued cancel waited {} ms for the request ahead of it",
        latency.as_millis()
    );
    assert!(
        queued_streamed.is_empty(),
        "the cancelled queued request decoded"
    );
    assert!(
        matches!(ahead_result, Err(EngineError::Cancelled)),
        "{ahead_result:?}"
    );
}

#[tokio::test]
#[ignore = "needs WIPEMARK_TEST_GGUF"]
async fn after_a_cancel_the_next_request_starts_clean() {
    let engine = engine(4096);
    let before = run(&engine, request(CREATIVE, 0.9, 7, 32)).await;

    let (result, _, pieces) = cancelled_after(&engine, Duration::from_millis(500)).await;
    assert!(matches!(result, Err(EngineError::Cancelled)), "{result:?}");
    assert!(pieces > 1, "the cancelled request never decoded");

    let after = run(&engine, request(CREATIVE, 0.9, 7, 32)).await;
    assert_eq!(
        before.text, after.text,
        "the same request with the same seed came out differently after a cancel"
    );
}

#[tokio::test]
#[ignore = "needs WIPEMARK_TEST_GGUF"]
async fn one_seed_twice_is_one_text_and_two_seeds_are_two() {
    let engine = engine(4096);
    let first = run(&engine, request(CREATIVE, 0.9, 1, 32)).await;
    let again = run(&engine, request(CREATIVE, 0.9, 1, 32)).await;
    let other = run(&engine, request(CREATIVE, 0.9, 2, 32)).await;
    eprintln!("seed 1: {:?}\nseed 2: {:?}", first.text, other.text);
    assert_eq!(first.text, again.text, "one seed gave two texts");
    assert_ne!(first.text, other.text, "two seeds gave one text");
}

#[tokio::test]
#[ignore = "needs WIPEMARK_TEST_GGUF"]
async fn a_prompt_longer_than_the_window_is_refused_before_decoding() {
    let engine = engine(512);
    let long: String = (0..700).map(|i| format!("word{i} ")).collect();
    let (sink, streamed) = flume::unbounded();
    let refused = engine
        .complete(request(&long, 0.0, 0, 16), sink, CancellationToken::new())
        .await;
    match refused {
        Err(EngineError::ContextOverflow { used, limit }) => {
            assert_eq!(limit, 512);
            assert!(used >= 600, "only {used} tokens");
        }
        other => panic!("an over-long prompt must be refused, got {other:?}"),
    }
    assert!(streamed.is_empty());
}

#[tokio::test]
#[ignore = "needs WIPEMARK_TEST_GGUF"]
async fn unload_then_complete_loads_again() {
    let engine = engine(2048);
    let first = run(&engine, request(CREATIVE, 0.0, 0, 16)).await;
    engine.unload().await;
    let second = run(&engine, request(CREATIVE, 0.0, 0, 16)).await;
    assert!(!second.text.is_empty());
    assert_eq!(first.text, second.text, "greedy, so the same text");
}

#[tokio::test]
#[ignore = "needs WIPEMARK_TEST_GGUF"]
async fn the_runtime_reports_at_least_the_cpu_backend() {
    let engine = engine(2048);
    engine.warmup().await.expect("the model loads");
    let runtime = Runtime::get().expect("a load initialises the runtime");
    eprintln!("LIVE backends: {:?}", runtime.backends());
    assert!(
        runtime
            .backends()
            .iter()
            .any(|b| b.kind == BackendKind::Cpu),
        "{:?}",
        runtime.backends()
    );
}
