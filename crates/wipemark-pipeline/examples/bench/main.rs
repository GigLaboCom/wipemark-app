//! The prompt bench (E4-5): the shipped templates against real models, on
//! a fixed en/ru/de corpus, measured attempt by attempt — and the
//! selection policies compared on the same candidates.
//!
//! See `docs/architecture/prompt-bench.md` for the method, the metrics and
//! how to rerun; `docs/plan/E4-5-the-prompt-bench.md` for why. A
//! development tool: it prints to the terminal and writes its records to
//! files it is told about, never through `tracing`.
//!
//! ```sh
//! export GGML_VULKAN=ON   # build llama.cpp with the Vulkan backend
//! cargo run -p wipemark-pipeline --features llama-native --example bench -- run \
//!     --local $MODELS/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf --name qwen3-4b --out runs/qwen3-4b.jsonl
//! ```

mod analyse;
mod args;
mod corpus;
mod engine;
mod judge;
mod measure;
mod run;
mod variant;
mod verify;
mod whole;

pub const USAGE: &str = "\
usage: bench <mode> [--flag value]...

  run     --local <gguf> | --endpoint <base URL> [--reasoning none|off]
          --name <model id> --out <records.jsonl>
          [--grid <spec>] [--langs en,ru] [--items en-pd-01,...] [--every n] [--ctx 8192]
          [--gpu-layers n] [--corpus <dir>] [--variant <dir>]
          [--temperature t] [--top-p p] [--min-p p] [--base-seed n]
  whole   --local <gguf> | --endpoint <URL>  --name <model id>
          --doc <file> --prompt <instruction file> --out <records.jsonl>
          [--samples n] [--temperature t] [--top-p p] [--base-seed n] [--ctx 12288]
  judge   --local <gguf> | --endpoint <URL>  --name <judge id>
          --in <records.jsonl,...> --out <judgements.jsonl>
  verify  --local <gguf> | --endpoint <URL>  --name <model id>
          --in <records.jsonl> --items <ids>
  report  --in <records.jsonl,...> [--judge <judgements.jsonl,...>]
          [--summary bench/results/summary.json] [--examples 3]
  plan    --of run|judge  and that mode's flags: prints
          `attempts=<n> calls=<n> done=<n>` and loads no model

grid: tactic:intensities:k;...  default
  paraphrase:light,moderate,strong:4;humanize:moderate,strong:2;back_translate:-:2;structural:-:1";

fn main() {
    let args = args::Args::parse();
    match args.mode.as_str() {
        "run" => run::main(&args),
        "judge" => judge::main(&args),
        "verify" => verify::main(&args),
        "whole" => whole::main(&args),
        "report" => analyse::main(&args),
        "plan" => match args.required("--of").as_str() {
            "run" => run::plan_only(&args),
            "judge" => judge::plan_only(&args),
            other => {
                eprintln!("plan --of {other:?}: run or judge\n\n{USAGE}");
                std::process::exit(2);
            }
        },
        other => {
            eprintln!("unknown mode {other:?}\n\n{USAGE}");
            std::process::exit(2);
        }
    }
}
