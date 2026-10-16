//! `bench/run-dflash.sh` (E2-dflash2, F4, D486): Qwen3.8 27B on the shipped
//! templates without its DFlash2 draft and with it, which the owner's host
//! runs. The run itself needs a GPU and the models and is never made here;
//! its `--dry-run` is — it prints every command the run would make and
//! touches nothing — and so is its refusal to start without the variables
//! that name the model and the draft, and the promise that it downloads
//! nothing.
//!
//! Unix only: it runs the script with `bash`.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Every variable the script reads, so a developer's own exports never
/// leak into a test.
const VARIABLES: [&str; 5] = [
    "WIPEMARK_BENCH_GGUF_QWEN38_27B",
    "WIPEMARK_BENCH_GGUF_QWEN38_DFLASH",
    "WIPEMARK_BENCH_GPU_LAYERS_QWEN38_27B",
    "WIPEMARK_LLAMA_PREBUILT",
    "WIPEMARK_LLAMA_SOURCE",
];

const BENCH: &str =
    "cargo run -q --offline --locked -p wipemark-pipeline --features llama-native --example bench --";

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/run-dflash.sh")
}

/// A directory that does not exist, for `--out`: the dry run must leave it so.
fn nowhere() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "wipemark-run-dflash-{}-{nanos}",
        std::process::id()
    ))
}

/// Both variables, pointing at files that do not exist.
fn both() -> Vec<(String, String)> {
    vec![
        (
            "WIPEMARK_BENCH_GGUF_QWEN38_27B".into(),
            "/models/qwen38.gguf".into(),
        ),
        (
            "WIPEMARK_BENCH_GGUF_QWEN38_DFLASH".into(),
            "/models/draft.gguf".into(),
        ),
    ]
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

/// The two runs over one corpus and one set of seeds, the model alone and
/// the draft beside it, under two names — and the report over both.
#[test]
fn the_dry_run_prints_both_runs_and_the_report_and_writes_nothing() {
    let out = nowhere();
    let out_s = out.display().to_string();
    let mut vars = both();
    vars.push(("WIPEMARK_BENCH_GPU_LAYERS_QWEN38_27B".into(), "40".into()));
    let done = run(&["--dry-run", "--out", &out_s], &vars);
    let stdout = text(&done.stdout);
    assert!(done.status.success(), "{stdout}\n{}", text(&done.stderr));
    let without = format!("{out_s}/runs/qwen38-27b.jsonl");
    let with = format!("{out_s}/runs/qwen38-27b+dflash2.jsonl");
    assert_eq!(
        commands(&stdout),
        vec![
            "cargo build --offline --locked -p wipemark-pipeline --features llama-native --example bench"
                .to_owned(),
            format!(
                "{BENCH} run --local /models/qwen38.gguf --gpu-layers 40 --name qwen38-27b \
                 --out {without} --every 3"
            ),
            format!(
                "{BENCH} run --local /models/qwen38.gguf --gpu-layers 40 --name qwen38-27b+dflash2 \
                 --out {with} --every 3 --draft /models/draft.gguf"
            ),
            format!(
                "{BENCH} report --in {without},{with} --summary {out_s}/summary.json \
                 > {out_s}/tables.md"
            ),
        ],
        "{stdout}"
    );
    assert!(
        stdout.contains("# not found (a real run refuses):"),
        "the dry run says the files are not there"
    );
    assert!(!out.exists(), "a dry run writes nothing");
}

#[test]
fn it_refuses_to_start_without_the_model_or_the_draft() {
    for (missing, _) in both() {
        let vars: Vec<(String, String)> =
            both().into_iter().filter(|(n, _)| *n != missing).collect();
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

/// The run without the draft needs only the model; the one with it needs
/// both; a part the script does not have is a refusal.
#[test]
fn a_part_needs_only_its_own_variables() {
    let model = vec![(
        "WIPEMARK_BENCH_GGUF_QWEN38_27B".to_owned(),
        "/models/qwen38.gguf".to_owned(),
    )];
    let done = run(&["--dry-run", "without"], &model);
    assert!(done.status.success(), "{}", text(&done.stderr));
    let stdout = text(&done.stdout);
    let lines = commands(&stdout);
    assert_eq!(lines.len(), 2, "the build and the one part: {stdout}");
    assert!(lines[1].contains("--name qwen38-27b "), "{stdout}");
    assert!(!lines[1].contains("--draft"), "{stdout}");

    let refused = run(&["--dry-run", "with"], &model);
    assert_eq!(refused.status.code(), Some(2));
    assert!(text(&refused.stderr).contains("WIPEMARK_BENCH_GGUF_QWEN38_DFLASH"));

    let unknown = run(&["--dry-run", "faster"], &both());
    assert_eq!(unknown.status.code(), Some(2));
}

#[test]
fn outside_a_dry_run_a_file_that_is_not_there_is_a_refusal() {
    let done = run(&["--estimate", "with"], &both());
    assert_eq!(done.status.code(), Some(2));
    let stderr = text(&done.stderr);
    assert!(
        stderr.contains("no such file: WIPEMARK_BENCH_GGUF_QWEN38_27B=/models/qwen38.gguf"),
        "{stderr}"
    );
    assert!(
        stderr.contains("WIPEMARK_BENCH_GGUF_QWEN38_DFLASH=/models/draft.gguf"),
        "{stderr}"
    );
}

/// No model and no draft is fetched: every command is cargo, offline, and
/// nothing in the script names a place to download from.
#[test]
fn it_downloads_nothing() {
    let source = std::fs::read_to_string(script()).unwrap();
    for word in ["curl", "wget", "http://", "https://", "huggingface"] {
        assert!(
            !source.to_lowercase().contains(word),
            "run-dflash.sh mentions {word}"
        );
    }
    // `models pull` is named only as the owner's own way to the draft, in
    // the header — never as a command the script runs.
    for line in source
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
    {
        assert!(!line.contains("models pull"), "the script pulls: {line}");
    }
    let done = run(&["--dry-run"], &both());
    for line in commands(&text(&done.stdout)) {
        assert!(
            line.starts_with("cargo ") && line.contains(" --offline "),
            "every command is cargo, offline: {line}"
        );
    }
}
