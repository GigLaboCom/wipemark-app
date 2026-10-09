//! `bench/run-voice.sh` (E4-8, V3): the four-model voice run the owner's
//! host makes. The run itself needs a GPU and the models and is never made
//! here; its `--dry-run` is — it prints every command the run would make and
//! touches nothing — and so is its refusal to start without the variables
//! that name the models (no default names a disk), and the promise that it
//! downloads nothing.
//!
//! Unix only: it runs the script with `bash`.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Every variable the script reads, so a developer's own exports never
/// leak into a test.
const VARIABLES: [&str; 13] = [
    "WIPEMARK_BENCH_GGUF_QWEN3_4B",
    "WIPEMARK_BENCH_GGUF_GEMMA3_12B",
    "WIPEMARK_BENCH_GGUF_GEMMA4_12B",
    "WIPEMARK_BENCH_GGUF_QWEN38_27B",
    "WIPEMARK_BENCH_GGUF_JUDGE",
    "WIPEMARK_BENCH_JUDGE_NAME",
    "WIPEMARK_BENCH_GPU_LAYERS_QWEN3_4B",
    "WIPEMARK_BENCH_GPU_LAYERS_GEMMA3_12B",
    "WIPEMARK_BENCH_GPU_LAYERS_GEMMA4_12B",
    "WIPEMARK_BENCH_GPU_LAYERS_QWEN38_27B",
    "WIPEMARK_BENCH_GPU_LAYERS_JUDGE",
    "WIPEMARK_LLAMA_PREBUILT",
    "WIPEMARK_LLAMA_SOURCE",
];

const MODELS: [(&str, &str); 4] = [
    ("qwen3-4b", "WIPEMARK_BENCH_GGUF_QWEN3_4B"),
    ("gemma3-12b", "WIPEMARK_BENCH_GGUF_GEMMA3_12B"),
    ("gemma4-12b", "WIPEMARK_BENCH_GGUF_GEMMA4_12B"),
    ("qwen38-27b", "WIPEMARK_BENCH_GGUF_QWEN38_27B"),
];

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/run-voice.sh")
}

/// A directory that does not exist, for `--out`: the dry run must leave it so.
fn nowhere() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("wipemark-run-voice-{}-{nanos}", std::process::id()))
}

/// Every model variable, pointing at files that do not exist.
fn all() -> Vec<(String, String)> {
    let mut vars: Vec<(String, String)> = MODELS
        .iter()
        .map(|(id, var)| (var.to_string(), format!("/models/{id}.gguf")))
        .collect();
    vars.push((
        "WIPEMARK_BENCH_GGUF_JUDGE".into(),
        "/models/judge.gguf".into(),
    ));
    vars.push(("WIPEMARK_BENCH_JUDGE_NAME".into(), "the-judge".into()));
    vars
}

fn run(args: &[&str], vars: &[(String, String)]) -> Output {
    let mut command = Command::new("bash");
    command.arg(script()).args(args);
    for name in VARIABLES {
        command.env_remove(name);
    }
    for (name, value) in vars {
        command.env(name, value);
    }
    command.output().expect("bash runs")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The lines that are commands, not comments.
fn commands(stdout: &str) -> Vec<&str> {
    stdout
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .collect()
}

const BENCH: &str =
    "cargo run -q --offline --locked -p wipemark-pipeline --features llama-native --example bench --";
const GRID: &str = "'paraphrase:light,moderate,strong:4;humanize:moderate,strong:2'";

#[test]
fn the_dry_run_prints_every_part_in_order_and_writes_nothing() {
    let out = nowhere();
    let out_s = out.display().to_string();
    let mut vars = all();
    vars.push(("WIPEMARK_BENCH_GPU_LAYERS_QWEN38_27B".into(), "40".into()));
    let done = run(&["--dry-run", "--out", &out_s], &vars);
    let stdout = text(&done.stdout);
    assert!(done.status.success(), "{stdout}\n{}", text(&done.stderr));
    let lines = commands(&stdout);
    let mut wanted = vec![
        "cargo build --offline --locked -p wipemark-pipeline --features llama-native --example bench"
            .to_owned(),
    ];
    let mut runs = Vec::new();
    for (id, _) in MODELS {
        let layers = if id == "qwen38-27b" { "40" } else { "-1" };
        for name in [id.to_owned(), format!("{id}+voice")] {
            let variant = if name.ends_with("+voice") {
                " --variant crates/wipemark-pipeline/bench/variants/keep-voice"
            } else {
                ""
            };
            wanted.push(format!(
                "{BENCH} run --local /models/{id}.gguf --gpu-layers {layers} --name {name} \
                 --out {out_s}/runs/{name}.jsonl --grid {GRID} --every 3{variant}"
            ));
            runs.push(format!("{out_s}/runs/{name}.jsonl"));
        }
    }
    let runs = runs.join(",");
    wanted.push(format!(
        "{BENCH} judge --local /models/judge.gguf --gpu-layers -1 --name the-judge --in {runs} \
         --out {out_s}/judge.jsonl"
    ));
    wanted.push(format!(
        "{BENCH} report --in {runs} --judge {out_s}/judge.jsonl --summary {out_s}/summary.json \
         --examples 3 > {out_s}/tables.md"
    ));
    assert_eq!(lines, wanted, "{stdout}");
    assert!(
        stdout.contains("# not found (a real run refuses):"),
        "the dry run says the files are not there"
    );
    assert!(!out.exists(), "a dry run writes nothing");
}

#[test]
fn it_refuses_to_start_without_a_model_variable() {
    for (missing, _) in all() {
        let vars: Vec<(String, String)> =
            all().into_iter().filter(|(n, _)| *n != missing).collect();
        for mode in [&["--dry-run"][..], &["--estimate"][..], &[][..]] {
            let done = run(mode, &vars);
            assert_eq!(done.status.code(), Some(2), "{missing} unset, {mode:?}");
            assert!(
                text(&done.stderr).contains(&missing),
                "{mode:?}: the refusal names {missing}: {}",
                text(&done.stderr)
            );
            assert!(
                commands(&text(&done.stdout)).is_empty(),
                "{mode:?}: nothing is printed as if it would run"
            );
        }
    }
}

#[test]
fn outside_a_dry_run_a_model_file_that_is_not_there_is_a_refusal() {
    let done = run(&["--estimate", "gemma4-12b"], &all());
    assert_eq!(done.status.code(), Some(2));
    assert!(text(&done.stderr)
        .contains("no such file: WIPEMARK_BENCH_GGUF_GEMMA4_12B=/models/gemma4-12b.gguf"));
}

#[test]
fn a_part_needs_only_its_own_variables() {
    let vars = vec![(
        "WIPEMARK_BENCH_GGUF_GEMMA4_12B".to_owned(),
        "/models/gemma4-12b.gguf".to_owned(),
    )];
    let done = run(&["--dry-run", "gemma4-12b+voice"], &vars);
    assert!(done.status.success(), "{}", text(&done.stderr));
    let stdout = text(&done.stdout);
    let lines = commands(&stdout);
    assert_eq!(lines.len(), 2, "the build and the one part: {stdout}");
    assert!(lines[1].contains("--name gemma4-12b+voice"));
    let unknown = run(&["--dry-run", "gemma5"], &vars);
    assert_eq!(unknown.status.code(), Some(2));
}

#[test]
fn it_downloads_nothing() {
    let source = std::fs::read_to_string(script()).unwrap();
    for word in [
        "curl",
        "wget",
        "models pull",
        "http://",
        "https://",
        "huggingface",
    ] {
        assert!(
            !source.to_lowercase().contains(word),
            "run-voice.sh mentions {word}"
        );
    }
    let done = run(&["--dry-run"], &all());
    for line in commands(&text(&done.stdout)) {
        assert!(
            line.starts_with("cargo ") && line.contains(" --offline "),
            "every command is cargo, offline: {line}"
        );
    }
}
