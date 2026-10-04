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

/// The child half of [`a_process_that_used_a_model_exits_cleanly`]: does
/// nothing unless that test started it.
#[tokio::test]
#[ignore = "started by a_process_that_used_a_model_exits_cleanly"]
async fn exit_child() {
    if std::env::var_os("WIPEMARK_EXIT_CHILD").is_none() {
        return;
    }
    let engine = engine(2048);
    let out = run(&engine, request(CREATIVE, 0.0, 0, 8)).await;
    assert!(out.tokens_out > 0);
    // The engine is dropped here, holding its model, and the process ends
    // straight after — the way every command and the bench end.
}

/// D96: a process that used a model must end as cleanly as one that did
/// not. Before E2-4, dropping a `LocalEngine` returned at once and its
/// worker freed the model *while* `exit` tore ggml's backends down; on
/// Vulkan that was a SIGSEGV after all the work, in every run (5 of 5 on
/// the RTX 5070 Ti). The child is this test binary, run again on
/// `exit_child`. On a CPU-only build there is no driver to tear down and
/// this cannot fail — it is a gate on a GPU build.
#[test]
#[ignore = "needs WIPEMARK_TEST_GGUF"]
fn a_process_that_used_a_model_exits_cleanly() {
    let _ = gguf();
    let exe = std::env::current_exe().expect("the test binary");
    for round in 1..=3 {
        let status = std::process::Command::new(&exe)
            .args(["--ignored", "--exact", "exit_child", "--test-threads=1"])
            .env("WIPEMARK_EXIT_CHILD", "1")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("the child starts");
        assert!(
            status.success(),
            "round {round}: a process that used a model ended with {status}"
        );
    }
}

// ---------------------------------------------------------------------------
// E2-4: the two models the bump is for, each behind a variable of its own.
//
// Neither is in the catalogue, so neither has a download line, and the
// gate command above — run with `WIPEMARK_TEST_GGUF` alone — must stay what
// it was. So these two **skip** when their variable is unset, saying so on
// stderr, instead of panicking (E2-4 I7). Set, they are gates like the rest:
//
//   WIPEMARK_TEST_GGUF_GEMMA4=/path/to/gemma-4-12B-it-qat-UD-Q4_K_XL.gguf
//   WIPEMARK_TEST_GGUF_QWEN38=/path/to/Qwen3.8-27B-UD-IQ3_S.gguf
//   WIPEMARK_TEST_GPU_LAYERS_QWEN38=40     # optional; default: all layers
// ---------------------------------------------------------------------------

/// A model named by `var`, or `None` (and a line on stderr) when unset.
fn optional_gguf(var: &str) -> Option<PathBuf> {
    match std::env::var_os(var) {
        Some(path) => Some(PathBuf::from(path)),
        None => {
            eprintln!("SKIPPED: {var} is not set");
            None
        }
    }
}

/// Layers to offload: `var` when set, every layer otherwise.
fn gpu_layers(var: &str) -> i32 {
    std::env::var(var)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(-1)
}

/// What a rewrite must not carry: a chat template's own markers. One of
/// them in an answer means the turn structure was wrong — the prompt was
/// rendered for another template, or a thinking block was opened and never
/// closed.
const MARKERS: [&str; 9] = [
    "<|turn>",
    "<turn|>",
    "<|channel>",
    "<channel|>",
    "<|im_start|>",
    "<|im_end|>",
    "<think>",
    "</think>",
    "<start_of_turn>",
];

/// The share of letters in `text` that `script` accepts.
fn script_share(text: &str, script: fn(char) -> bool) -> f64 {
    let letters: Vec<char> = text.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.is_empty() {
        return 0.0;
    }
    letters.iter().filter(|c| script(**c)).count() as f64 / letters.len() as f64
}

fn cyrillic(c: char) -> bool {
    ('\u{0400}'..='\u{04FF}').contains(&c)
}

fn latin(c: char) -> bool {
    c.is_ascii_alphabetic()
}

const REWRITE_SYSTEM: &str = "You rewrite text. Say the same thing in different words, in the \
     same language as the text, keeping every number. Output only the rewritten text.";

const EN_TEXT: &str = "The release notes are published every Tuesday morning. Version 2.4.1 \
     fixed 37 bugs reported since March, and the full installer is about 180 MB.";

const RU_TEXT: &str = "Первая сборка проекта занимает около 12 минут на обычном ноутбуке, а \
     повторная — меньше минуты, потому что зависимости уже скомпилированы.";

/// Rewrite one text with `engine`, greedy, and check the answer is a
/// rewrite in the text's own script that kept `number` — what token salad,
/// a wrong turn structure or an unclosed thinking block all fail.
async fn rewrites(
    engine: &LocalEngine,
    label: &str,
    text: &str,
    number: &str,
    script: fn(char) -> bool,
) {
    let (sink, streamed) = flume::unbounded();
    let started = Instant::now();
    let out = engine
        .complete(
            ChatRequest {
                system: Some(REWRITE_SYSTEM.to_owned()),
                prompt: text.to_owned(),
                params: SamplingParams {
                    temperature: 0.0,
                    seed: Some(1),
                    max_tokens: Some(200),
                    ..SamplingParams::default()
                },
            },
            sink,
            CancellationToken::new(),
        )
        .await
        .unwrap_or_else(|e| panic!("{label}: the completion was refused: {e:?}"));
    let elapsed = started.elapsed();
    eprintln!(
        "LIVE {label}: {} tokens ({:?}) in {} ms, {:.1} tokens/s: {:?}",
        out.tokens_out,
        out.finish,
        elapsed.as_millis(),
        f64::from(out.tokens_out) / elapsed.as_secs_f64(),
        out.text
    );
    let concatenated: String = streamed.drain().collect();
    assert_eq!(
        concatenated, out.text,
        "{label}: the sink and the text disagree"
    );
    assert_eq!(
        out.finish,
        FinishReason::Stop,
        "{label}: a two-sentence rewrite ran to the token limit"
    );
    let answer = out.text.trim();
    assert!(!answer.is_empty(), "{label}: an empty answer");
    for marker in MARKERS {
        assert!(
            !answer.contains(marker),
            "{label}: the answer carries the template marker {marker}"
        );
    }
    assert!(
        answer.contains(number),
        "{label}: the number {number} was lost: {answer:?}"
    );
    let share = script_share(answer, script);
    assert!(
        share > 0.8,
        "{label}: only {:.0} % of the letters are in the text's script: {answer:?}",
        share * 100.0
    );
    assert_ne!(answer, text.trim(), "{label}: the text came back unchanged");
}

/// Load the model named by `var` and rewrite an English and a Russian text.
async fn a_named_model_rewrites(var: &str, layers_var: &str, model_id: &str) {
    let Some(weights) = optional_gguf(var) else {
        return;
    };
    let engine = LocalEngine::new(LocalConfig {
        model_id: model_id.to_owned(),
        weights,
        load: LoadParams {
            n_ctx: 4096,
            n_gpu_layers: gpu_layers(layers_var),
            ..LoadParams::default()
        },
        available_mb: None,
    });
    let started = Instant::now();
    engine.warmup().await.expect("the model loads");
    eprintln!(
        "LIVE {model_id}: loaded in {} ms on {:?}",
        started.elapsed().as_millis(),
        Runtime::get().map(|r| r.backends().to_vec())
    );
    rewrites(&engine, &format!("{model_id} en"), EN_TEXT, "37", latin).await;
    rewrites(&engine, &format!("{model_id} ru"), RU_TEXT, "12", cyrillic).await;
    // Given back before the next test loads another model.
    engine.unload().await;
}

#[tokio::test]
#[ignore = "needs WIPEMARK_TEST_GGUF_GEMMA4"]
async fn gemma_4_rewrites_in_english_and_russian() {
    a_named_model_rewrites(
        "WIPEMARK_TEST_GGUF_GEMMA4",
        "WIPEMARK_TEST_GPU_LAYERS_GEMMA4",
        "gemma-4-12b-it-qat-ud-q4",
    )
    .await;
}

#[tokio::test]
#[ignore = "needs WIPEMARK_TEST_GGUF_QWEN38"]
async fn qwen_3_8_rewrites_in_english_and_russian() {
    a_named_model_rewrites(
        "WIPEMARK_TEST_GGUF_QWEN38",
        "WIPEMARK_TEST_GPU_LAYERS_QWEN38",
        "qwen3.8-27b-ud-iq3s",
    )
    .await;
}
