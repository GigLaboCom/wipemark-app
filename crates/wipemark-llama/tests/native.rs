//! The native smokes of `wipemark-llama`: a real llama.cpp, and — for the
//! `#[ignore]`d ones — a real GGUF.
//!
//! Compiled only under `--features native`. The model-free test runs in the
//! native gate; the rest need `WIPEMARK_TEST_GGUF` and run with
//!
//! ```sh
//! WIPEMARK_TEST_GGUF=/path/to/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf \
//!   cargo test -p wipemark-llama --features native -- --ignored --test-threads=1 --nocapture
//! ```
//!
//! An ignored test run without the variable panics with the download
//! instructions: a smoke that skipped is not a smoke that passed.

#![cfg(feature = "native")]

use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use wipemark_llama::{
    estimate, BackendKind, Finish, KvQuant, KvShape, LlamaError, LoadParams, Model, Runtime,
    Sampling, BUILT_NATIVE,
};

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

fn load(n_ctx: u32) -> Model {
    Model::load(
        &gguf(),
        LoadParams {
            n_ctx,
            ..LoadParams::default()
        },
    )
    .expect("the test model loads")
}

fn greedy(max_tokens: u32) -> Sampling {
    Sampling {
        temperature: 0.0,
        top_p: 1.0,
        top_k: 40,
        min_p: None,
        seed: 0,
        max_tokens,
    }
}

#[test]
fn the_runtime_reports_at_least_the_cpu_backend() {
    assert!(std::hint::black_box(BUILT_NATIVE));
    let runtime = Runtime::init(&[]);
    assert!(
        runtime
            .backends()
            .iter()
            .any(|b| b.kind == BackendKind::Cpu),
        "no CPU backend registered: {:?} (searched {:?})",
        runtime.backends(),
        runtime.dirs()
    );
    eprintln!("backends: {:?}", runtime.backends());
}

#[test]
#[ignore = "needs WIPEMARK_TEST_GGUF"]
fn the_estimate_reads_the_models_own_shape() {
    let estimate = estimate(&gguf(), &LoadParams::default()).expect("the header reads");
    assert_eq!(
        estimate.kv_shape,
        Some(KvShape {
            n_layer: 36,
            n_head_kv: 8,
            key_length: 128,
            value_length: 128,
        })
    );
    // 2 546 340 960 bytes, rounded up to MiB.
    assert_eq!(estimate.weights_mb, 2429);
    assert_eq!(estimate.kv_cache_mb, 612);
    let f16 = wipemark_llama::estimate(
        &gguf(),
        &LoadParams {
            kv_quant: KvQuant::F16,
            ..LoadParams::default()
        },
    )
    .unwrap();
    assert_eq!(f16.kv_cache_mb, 1152, "the catalogue's own figure");
}

#[test]
#[ignore = "needs WIPEMARK_TEST_GGUF"]
fn a_real_model_writes_a_sentence() {
    let mut model = load(2048);
    assert_eq!(model.n_ctx(), 2048);
    assert!(model.n_ctx_train() >= 2048);
    let prompt = model
        .chat_prompt(
            Some("You answer in one short sentence."),
            "What colour is the sky on a clear day?",
        )
        .expect("the template renders");
    let mut pieces = String::new();
    let out = model
        .generate(
            &prompt,
            &greedy(48),
            &AtomicBool::new(false),
            &mut |piece| {
                pieces.push_str(piece);
                ControlFlow::Continue(())
            },
        )
        .expect("generates");
    eprintln!("{out:?}");
    assert!(!out.text.trim().is_empty());
    assert!(matches!(out.finish, Finish::Stop | Finish::Length));
    assert_eq!(pieces, out.text);
    assert!(out.text.to_lowercase().contains("blue"), "{:?}", out.text);
}

#[test]
#[ignore = "needs WIPEMARK_TEST_GGUF"]
fn a_prompt_longer_than_the_window_is_refused_before_decoding() {
    let mut model = load(512);
    let long: String = (0..700).map(|i| format!("word{i} ")).collect();
    let prompt = model.chat_prompt(None, &long).unwrap();
    let used = model.count_tokens(&prompt).unwrap();
    assert!(used >= 600, "the prompt is only {used} tokens");
    let mut handed = 0;
    let refused = model.generate(&prompt, &greedy(16), &AtomicBool::new(false), &mut |_| {
        handed += 1;
        ControlFlow::Continue(())
    });
    assert_eq!(
        refused.unwrap_err(),
        LlamaError::ContextOverflow { used, limit: 512 }
    );
    assert_eq!(handed, 0);
}

/// The figures the live gate records: load time and tokens per second for
/// a 200-token generation on whatever backend registered.
#[test]
#[ignore = "needs WIPEMARK_TEST_GGUF"]
fn the_live_gate_figures() {
    let runtime = Runtime::init(&[]);
    eprintln!(
        "LIVE backends: {}",
        runtime
            .backends()
            .iter()
            .map(|b| format!("{} ({})", b.device, b.registry))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let started = Instant::now();
    let mut model = load(8192);
    eprintln!("LIVE load: {} ms", started.elapsed().as_millis());

    let prompt = model
        .chat_prompt(
            None,
            "Write a long, detailed essay about the history of the printing press.",
        )
        .unwrap();
    let sampling = Sampling {
        temperature: 0.7,
        top_p: 0.95,
        top_k: 40,
        min_p: None,
        seed: 1,
        max_tokens: 200,
    };
    let started = Instant::now();
    let mut first = None;
    let out = model
        .generate(&prompt, &sampling, &AtomicBool::new(false), &mut |_| {
            first.get_or_insert_with(|| started.elapsed());
            ControlFlow::Continue(())
        })
        .unwrap();
    let total = started.elapsed();
    let first = first.unwrap_or(total);
    let decode = total - first;
    eprintln!(
        "LIVE generate: {} tokens ({:?}) in {} ms, first piece after {} ms, \
         {:.1} tokens/s overall, {:.1} tokens/s after the first",
        out.tokens_out,
        out.finish,
        total.as_millis(),
        first.as_millis(),
        f64::from(out.tokens_out) / total.as_secs_f64(),
        f64::from(out.tokens_out.saturating_sub(1)) / decode.as_secs_f64(),
    );
    assert_eq!(out.tokens_out, 200, "{:?}", out.finish);
}
