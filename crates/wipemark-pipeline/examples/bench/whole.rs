//! `whole`: one document in one request, the way upstream
//! (`guillaumemeyer/watermarks-remover`, `rewrite_text.py`) sends it — no
//! system turn, the instruction and the document in a single user turn
//! separated by `\n\n---\n`, no chunking, no guards, no selection — so its
//! answer can be measured beside the loop's on the same model and engine.
//!
//! Added for `docs/plan/reports/divergence-vs-upstream-2026-10-07.md`. The
//! instruction is read from `--prompt <file>` rather than written here:
//! `docs/plan/reports/divergence-vs-upstream/prepare.py` extracts it from an
//! upstream clone under `tmp/`, so nothing of upstream's is vendored.
//!
//! ```sh
//! bench whole --local <gguf> --name <id> --doc <file.md> --prompt <instruction.txt>
//!             --out <records.jsonl> [--samples 3] [--temperature 0.9] [--top-p 0.95]
//!             [--base-seed 0] [--ctx 12288]
//! ```

use std::io::Write;
use std::time::Instant;

use serde_json::json;
use wipemark_engine::{ChatRequest, FinishReason, SamplingParams};
use wipemark_pipeline::prepare::estimate_tokens;

use crate::args::Args;
use crate::{engine, run};

pub fn main(args: &Args) {
    let doc = std::fs::read_to_string(args.required("--doc")).expect("--doc reads");
    let instruction = std::fs::read_to_string(args.required("--prompt")).expect("--prompt reads");
    let out = std::path::PathBuf::from(args.required("--out"));
    let samples: u64 = args
        .value("--samples")
        .map_or(1, |v| v.parse().expect("--samples is a number"));
    let mut params = SamplingParams::default();
    if let Some(v) = args.value("--temperature") {
        params.temperature = v.parse().expect("--temperature is a number");
    }
    if let Some(v) = args.value("--top-p") {
        params.top_p = v.parse().expect("--top-p is a number");
    }
    let base_seed: u64 = args
        .value("--base-seed")
        .map_or(0, |v| v.parse().expect("--base-seed is a number"));
    // Upstream's `build_prompt`: the instruction, then `\n\n---\n`, then
    // the text, strictly last.
    let prompt = format!("{}\n\n---\n{}", instruction.trim_end(), doc);
    let done = run::done(&out);
    let (model, engine) = engine::from_args(args);
    engine::block_on(engine.warmup()).expect("the engine loads");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&out)
        .expect("the output opens");
    for n in 0..samples {
        let key = format!("whole|{model}|{n}");
        if done.contains(&key) {
            continue;
        }
        let seed = base_seed.wrapping_add(n);
        let mut p = params.clone();
        p.seed = Some(seed);
        p.max_tokens = Some(estimate_tokens(&doc).saturating_mul(2).saturating_add(64));
        let started = Instant::now();
        let answer = engine::complete(
            engine.as_ref(),
            ChatRequest {
                system: None,
                prompt: prompt.clone(),
                params: p.clone(),
            },
        )
        .unwrap_or_else(|e| panic!("the engine failed: {e:?}"));
        let secs = started.elapsed().as_secs_f64();
        let record = json!({
            "key": key,
            "model": model,
            "seed": seed,
            "temperature": p.temperature,
            "top_p": p.top_p,
            "tokens_out": answer.tokens_out,
            "finish": match answer.finish {
                FinishReason::Stop => "stop",
                FinishReason::Length => "length",
                FinishReason::Cancelled => "cancelled",
            },
            "secs": secs,
            "source": doc,
            "answer": answer.text,
        });
        writeln!(file, "{record}").expect("written");
        eprintln!(
            "[{}/{samples}] seed {seed}: {} tokens, {secs:.1}s",
            n + 1,
            answer.tokens_out
        );
    }
}
