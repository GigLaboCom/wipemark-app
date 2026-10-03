//! The loop against a real model — E4-3's live gate.
//!
//! Compiled only under `--features llama-native`; `#[ignore]`d, and the
//! model's path comes from `WIPEMARK_TEST_GGUF`:
//!
//! ```sh
//! WIPEMARK_TEST_GGUF=/path/to/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf \
//!   cargo test -p wipemark-pipeline --features llama-native --locked -- --ignored --test-threads=1 --nocapture
//! ```
//!
//! Without the variable it panics with the download line: a gate that
//! skipped is not a gate that passed. What it prints — every attempt's
//! verdict, the result, the timings — is what the report of the step
//! records; what it asserts is what must hold whatever the model writes:
//! the job finishes, nothing protected or numbered is lost, and the
//! report carries its shelves.

#![cfg(feature = "llama-native")]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use wipemark_engine::{LoadParams, LocalConfig, LocalEngine, RewriteEngine};
use wipemark_pipeline::cost::{Effort, Executor};
use wipemark_pipeline::prepare::TextFormat;
use wipemark_pipeline::prompt::Tactic;
use wipemark_pipeline::report::{rejection_value, ChunkOutcome, Verdict};
use wipemark_pipeline::{job, start, Document, Event, JobId, Options};

fn gguf() -> PathBuf {
    match std::env::var_os("WIPEMARK_TEST_GGUF") {
        Some(path) => PathBuf::from(path),
        None => panic!(
            "WIPEMARK_TEST_GGUF is not set. Download the catalogue's Qwen3 4B and point it there:\n  \
             curl -L -o Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf \
             https://huggingface.co/unsloth/Qwen3-4B-Instruct-2507-GGUF/resolve/a06e946bb6b655725eafa393f4a9745d460374c9/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf\n  \
             WIPEMARK_TEST_GGUF=$PWD/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf"
        ),
    }
}

fn engine() -> Arc<dyn RewriteEngine> {
    Arc::new(LocalEngine::new(LocalConfig {
        model_id: "qwen3-4b-instruct-2507-ud-q4".to_owned(),
        weights: gguf(),
        load: LoadParams {
            n_ctx: 8192,
            ..LoadParams::default()
        },
        available_mb: None,
    }))
}

const RU_NOTE: &str = "# Заметка о сборке

Чтобы собрать проект, выполните `cargo build --release` в корне репозитория. Первая сборка занимает около 12 минут на обычном ноутбуке, а повторная — меньше минуты, потому что зависимости уже скомпилированы.

Перед сборкой проверьте:

- установлен ли пакет `libxkbcommon-x11` и видит ли его компоновщик;
- хватает ли на диске 8 ГБ свободного места для каталога сборки;
- совпадает ли версия компилятора с указанной в файле `rust-toolchain.toml`.
";

const EN_PLAIN: &str = "The release notes are published at https://example.com/releases/v2.4 every Tuesday morning. Version 2.4.1 fixed 37 bugs reported since March, and the download is about 180 MB for the full installer or 42 MB for the patch alone.\n";

fn rewrite(name: &str, text: &str, format: TextFormat, must_keep: &[&str]) {
    let mut options = Options::for_executor(Executor::LocalCpu);
    options.effort = Effort {
        candidates: 1,
        rounds: 2,
    };
    options.ladder = vec![Tactic::Paraphrase, Tactic::Humanize];
    options.base_seed = 42;
    let engine = engine();
    let document = Document {
        text: text.to_owned(),
        format,
    };

    let planned = job::plan(&document, &options, &engine.info()).expect("plans");
    let cost = planned.cost(&options, Some(10.0));
    println!(
        "\n=== {name}: {} chunk(s), language {:?}, budget {}; calls {}..{}, tokens out {}..{}",
        cost.chunks,
        planned.language(),
        planned.budget.max_tokens,
        cost.calls.expected,
        cost.calls.worst,
        cost.tokens_out.expected,
        cost.tokens_out.worst,
    );

    let started = Instant::now();
    let (_handle, events) = start(JobId(1), document, options, engine).expect("starts");
    let mut last = None;
    while let Ok(event) = events.recv_timeout(Duration::from_secs(900)) {
        match &event {
            Event::Stage { stage, .. } => {
                println!("[{:>6.1}s] {stage:?}", started.elapsed().as_secs_f32())
            }
            Event::CandidateRejected {
                chunk,
                round,
                candidate,
                rejection,
                ..
            } => println!(
                "[{:>6.1}s] rejected chunk {chunk} round {round} candidate {candidate}: {}",
                started.elapsed().as_secs_f32(),
                rejection_value(rejection)
            ),
            Event::Token { .. } => {}
            Event::Finished { .. } | Event::Cancelled { .. } | Event::Failed { .. } => {
                last = Some(event);
                break;
            }
        }
    }
    let outcome = match last {
        Some(Event::Finished {
            outcome, elapsed, ..
        }) => {
            println!("finished in {:.1}s", elapsed.as_secs_f32());
            outcome
        }
        other => panic!("the job did not finish: {other:?}"),
    };

    for chunk in &outcome.report.chunks {
        println!("chunk {} → {:?}", chunk.index, chunk.outcome);
        for attempt in &chunk.attempts {
            let verdict = match &attempt.verdict {
                Verdict::Passed(scores) => format!(
                    "passed: divergence {:.3}, length {:.2}, score {:.3}",
                    scores.divergence, scores.length_ratio, scores.score
                ),
                Verdict::Rejected(rejection) => format!("rejected: {}", rejection_value(rejection)),
            };
            let tokens: u32 = attempt.steps.iter().map(|s| s.tokens_out).sum();
            println!(
                "  round {} candidate {} {} seed {}: {tokens} tokens in {:.1}s — {verdict}",
                attempt.round,
                attempt.candidate,
                attempt.tactic.as_str(),
                attempt.seed,
                attempt.elapsed.as_secs_f32(),
            );
        }
        assert!(matches!(
            chunk.outcome,
            ChunkOutcome::Rewritten { .. } | ChunkOutcome::KeptSource(_)
        ));
    }
    println!("--- result ---\n{}--- end ---", outcome.text);
    let totals = outcome.report.totals();
    println!("totals: {totals:?}");

    assert!(!outcome.text.contains('\u{200B}'));
    let items = |text: &str| text.lines().filter(|l| l.starts_with("- ")).count();
    assert_eq!(
        items(&outcome.text),
        items(text),
        "every list item is still one"
    );
    for item in outcome.text.lines().filter(|l| l.starts_with("- ")) {
        // Words outside code: an item left holding a semicolon, or only its
        // protected file name, is an item whose words moved elsewhere.
        let prose: String = item.split('`').step_by(2).collect();
        assert!(
            prose.chars().filter(|c| c.is_alphabetic()).count() >= 3,
            "an item lost its words: {item:?}"
        );
    }
    for kept in must_keep {
        assert!(outcome.text.contains(kept), "{kept:?} was lost");
    }
    let json: serde_json::Value =
        serde_json::from_str(&outcome.report.to_json()).expect("the report is JSON");
    assert_eq!(json["not_established"].as_array().map(Vec::len), Some(3));
    assert_eq!(json["best_effort"]["totals"]["attempts"], totals.attempts);
}

#[test]
#[ignore = "needs WIPEMARK_TEST_GGUF"]
fn a_russian_markdown_note_is_rewritten_by_qwen3() {
    rewrite(
        "ru markdown",
        RU_NOTE,
        TextFormat::Markdown,
        &[
            "# Заметка о сборке",
            "`cargo build --release`",
            "`libxkbcommon-x11`",
            "`rust-toolchain.toml`",
            "12",
            "8",
        ],
    );
}

#[test]
#[ignore = "needs WIPEMARK_TEST_GGUF"]
fn an_english_paragraph_with_a_url_and_numbers_is_rewritten_by_qwen3() {
    rewrite(
        "en plain",
        EN_PLAIN,
        TextFormat::Plain,
        &[
            "https://example.com/releases/v2.4",
            "2.4.1",
            "37",
            "180",
            "42",
        ],
    );
}
