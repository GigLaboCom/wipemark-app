//! `run`: every corpus item × chunk × (tactic, intensity) × candidate, one
//! attempt each, made as the loop makes it — and one JSON line per attempt.
//!
//! [`attempt`] is `job/attempt.rs` step for step (render → complete →
//! `clean_response`, per step → Layer A), and its verdict is the loop's
//! own — `wipemark_pipeline::job::verdict`: the guards with the chunk's
//! length window, the language check, restore, the no-op floor (E4-7). Two
//! things are the bench's: every guard and restore are also run **on their
//! own**, so a candidate failing two is counted in both rates, and the
//! texts are kept. `verify` proves the verdicts agree with the loop's on
//! the same seeds.

use std::collections::HashSet;
use std::io::{BufRead, Write};
use std::path::Path;
use std::time::Instant;

use serde_json::{json, Value};
use wipemark_core::GuardOutcome;
use wipemark_engine::{EngineError, FinishReason, RewriteEngine};
use wipemark_pipeline::cost::{Effort, Executor};
use wipemark_pipeline::job::{plan, verdict, Rung};
use wipemark_pipeline::lang::Lang;
use wipemark_pipeline::prepare::{estimate_tokens, Chunk, TextFormat};
use wipemark_pipeline::prompt::{
    clean_response, hash, render, shipped, validate, Input, Intensity, Origin, Override, Overrides,
    RenderError, Role, Severity, Slot, Stripped, Tactic, ValidationContext,
};
use wipemark_pipeline::report::{rejection_value, EngineFailure, Rejection};
use wipemark_pipeline::select::{self, Scorer};
use wipemark_pipeline::{seed_for, Document, Options};

use crate::args::Args;
use crate::corpus::{self, Item};
use crate::{engine, measure};

/// The effort whose seeds the bench reproduces: a GPU's 2 × 2 (D61), so
/// candidate `k` is the attempt the job would make there — round
/// `(k−1)/2 + 1`, candidate `(k−1)%2 + 1`.
pub const SEEDS: Effort = Effort {
    candidates: 2,
    rounds: 2,
};

/// One cell of the grid: a tactic, an intensity and how many candidates.
#[derive(Debug, Clone)]
pub struct Cell {
    pub tactic: Tactic,
    pub intensity: Intensity,
    pub k: u8,
}

/// `paraphrase:light,moderate,strong:4;humanize:moderate,strong:2;…` —
/// `-` for the intensity of a tactic that takes none.
pub fn grid(spec: &str) -> Vec<Cell> {
    let mut cells = Vec::new();
    for part in spec.split(';').filter(|p| !p.trim().is_empty()) {
        let fields: Vec<&str> = part.trim().split(':').collect();
        let [tactic, levels, k] = fields[..] else {
            panic!("grid cell {part:?}: tactic:intensities:k");
        };
        let tactic = Tactic::parse(tactic).unwrap_or_else(|| panic!("tactic {tactic:?}"));
        let k: u8 = k.parse().expect("k is a number");
        assert!(
            (1..=4).contains(&k),
            "k is 1..=4 (the seeds of a 2 × 2 job)"
        );
        for level in levels.split(',') {
            let intensity = if level == "-" {
                Intensity::Moderate
            } else {
                Intensity::parse(level).unwrap_or_else(|| panic!("intensity {level:?}"))
            };
            cells.push(Cell {
                tactic,
                intensity,
                k,
            });
        }
    }
    cells
}

pub const DEFAULT_GRID: &str = "paraphrase:light,moderate,strong:4;humanize:moderate,strong:2;back_translate:-:2;structural:-:1";

/// The sampling and the base seed the flags ask for, over the product's
/// own (`SamplingParams::default()`, base seed 0): `--temperature`,
/// `--top-p`, `--min-p`, `--base-seed`. A bench knob only — the product's
/// defaults are not moved by it (added for the divergence study,
/// `docs/plan/reports/divergence-vs-upstream-2026-10-07.md`).
pub fn tune(args: &Args, options: &mut Options) {
    if let Some(v) = args.value("--temperature") {
        options.sampling.temperature = v.parse().expect("--temperature is a number");
    }
    if let Some(v) = args.value("--top-p") {
        options.sampling.top_p = v.parse().expect("--top-p is a number");
    }
    if let Some(v) = args.value("--min-p") {
        options.sampling.min_p = Some(v.parse().expect("--min-p is a number"));
    }
    if let Some(v) = args.value("--base-seed") {
        options.base_seed = v.parse().expect("--base-seed is a number");
    }
}

/// The options of one cell: the product's for a GPU, one rung.
pub fn options(tactic: Tactic, intensity: Intensity) -> Options {
    let mut options = Options::for_executor(Executor::LocalGpu);
    options.ladder = vec![tactic];
    options.intensity = intensity;
    options.structural_confirmed = tactic == Tactic::Structural;
    options
}

/// A template variant to try without rebuilding: every
/// `<dir>/<lang>/<tactic>.<step>.<role>.txt` becomes an override of that
/// slot — the road a user's edited template takes (D74), so a variant is
/// planned, validated and rendered exactly as an edit would be.
pub fn variant(dir: &Path) -> Overrides {
    let mut overrides = Overrides::new();
    for lang in Lang::ALL {
        let Ok(entries) = std::fs::read_dir(dir.join(lang.as_str())) else {
            continue;
        };
        for entry in entries.map_while(Result::ok) {
            let name = entry.file_name().to_string_lossy().into_owned();
            let parts: Vec<&str> = name.split('.').collect();
            let [tactic, step, role, "txt"] = parts[..] else {
                panic!("{name}: <tactic>.<step>.<role>.txt");
            };
            let slot = Slot::new(
                lang,
                Tactic::parse(tactic).expect("a tactic"),
                step.parse().expect("a step"),
                Role::parse(role).expect("a role"),
            )
            .unwrap_or_else(|| panic!("{name}: no such slot"));
            let text = std::fs::read_to_string(entry.path()).expect("the variant reads");
            // Judged as an edit in Settings would be: an error refuses the
            // variant, so a variant that wins can be shipped as it is.
            let other = std::fs::read_to_string(dir.join(lang.as_str()).join(format!(
                "{tactic}.{step}.{}.txt",
                slot.other_role().role().as_str()
            )))
            .ok();
            let other = other
                .as_deref()
                .or_else(|| shipped::template(slot.other_role()))
                .expect("the other role");
            let context = ValidationContext {
                other_role: other,
                ctx_len: Some(8192),
                intensity: Intensity::Moderate,
                based_on: None,
            };
            let problems = validate(slot, &text, &context);
            for problem in &problems {
                eprintln!("variant {name}: {problem:?} ({:?})", problem.severity());
            }
            assert!(
                problems.iter().all(|p| p.severity() != Severity::Error),
                "{name} does not validate"
            );
            overrides.insert(
                slot,
                Override {
                    text,
                    based_on: hash(shipped::template(slot).expect("a shipped template")),
                    adapted_from: None,
                    origin: Origin::Hand,
                },
            );
        }
    }
    overrides
}

pub fn key(model: &str, item: &str, chunk: usize, tactic: &str, intensity: &str, k: u8) -> String {
    format!("{model}|{item}|{chunk}|{tactic}|{intensity}|{k}")
}

fn stripped(s: &Stripped) -> &'static str {
    match s {
        Stripped::Think => "think",
        Stripped::Marker { .. } => "marker",
        Stripped::Fence => "fence",
        Stripped::Quotes { .. } => "quotes",
    }
}

fn finish(f: FinishReason) -> &'static str {
    match f {
        FinishReason::Stop => "stop",
        FinishReason::Length => "length",
        FinishReason::Cancelled => "cancelled",
    }
}

/// One attempt at `chunk` with `rung`, as `job/attempt.rs` makes it.
pub fn attempt(
    engine: &dyn RewriteEngine,
    options: &Options,
    chunk: &Chunk,
    rung: &Rung,
    seed: u64,
) -> Value {
    let mut steps = Vec::new();
    let mut text = chunk.text.clone();
    let mut verdict_of: Option<Rejection> = None;
    for step in &rung.plan.steps {
        let input = Input {
            text: &text,
            context: chunk.context.as_deref(),
            intensity: options.intensity,
        };
        let rendered = match render(step, &input) {
            Ok(rendered) => rendered,
            Err(RenderError::MarkerInText { marker }) => {
                verdict_of = Some(Rejection::MarkerInAnswer {
                    step: step.step,
                    marker,
                });
                break;
            }
            Err(RenderError::Template { slot, .. }) => {
                panic!("a shipped template does not render: {slot:?}")
            }
        };
        let mut params = options.sampling.clone();
        params.seed = Some(seed);
        if params.max_tokens.is_none() {
            params.max_tokens = Some(estimate_tokens(&text).saturating_mul(2).saturating_add(64));
        }
        let started = Instant::now();
        let answer = engine::complete(engine, rendered.into_request(params));
        let secs = started.elapsed().as_secs_f64();
        let completion = match answer {
            Ok(completion) => completion,
            Err(error) => {
                match EngineFailure::of(error) {
                    Ok(failure) => {
                        steps.push(json!({"step": step.step, "secs": secs, "error": format!("{failure:?}")}));
                        verdict_of = Some(Rejection::Engine {
                            step: step.step,
                            failure,
                        });
                        break;
                    }
                    Err(EngineError::Unavailable(why)) => {
                        panic!("the engine is unavailable: {why:?}")
                    }
                    Err(other) => panic!("the engine stopped: {other:?}"),
                }
            }
        };
        let cleaned = clean_response(&completion.text, &text);
        steps.push(json!({
            "step": step.step,
            "raw": completion.text,
            "cleaned": cleaned.text,
            "stripped": cleaned.stripped.iter().map(stripped).collect::<Vec<_>>(),
            "tokens_out": completion.tokens_out,
            "finish": finish(completion.finish),
            "secs": secs,
        }));
        match completion.finish {
            FinishReason::Stop => {}
            FinishReason::Length => {
                verdict_of = Some(Rejection::Truncated { step: step.step });
                break;
            }
            FinishReason::Cancelled => panic!("nothing cancels a bench call"),
        }
        if cleaned.text.trim().is_empty() {
            verdict_of = Some(Rejection::Empty { step: step.step });
            break;
        }
        text = cleaned.text;
    }

    let mut record = json!({
        "chunk_text": chunk.text,
        "placeholders": chunk.protected.len(),
        "steps": steps,
    });
    if verdict_of.is_none() {
        let layer_a = wipemark_core::clean(&text, &options.layer_a);
        let answer = layer_a.text;
        let removed: u32 = layer_a.report.removed.iter().map(|(_, n)| n).sum();
        // Every guard and restore on their own, for the rates; the
        // verdict is the loop's.
        let mut each = serde_json::Map::new();
        for guard in options.guards_for(chunk) {
            let value = match guard.check(&chunk.text, &answer) {
                GuardOutcome::Pass => Value::Null,
                GuardOutcome::Reject(reason) => rejection_value(&Rejection::Guard {
                    guard: guard.name(),
                    reason,
                }),
            };
            each.insert(guard.name().to_owned(), value);
        }
        let restore_value = match chunk.restore(&answer) {
            Ok(_) => Value::Null,
            Err(error) => rejection_value(&Rejection::Restore(error)),
        };
        if let Err(rejection) = verdict(options, chunk, &answer) {
            verdict_of = Some(rejection);
        }
        let scores = select::score(Scorer::Divergence, &chunk.text, &answer);
        let kept = (1..=chunk.protected.len())
            .filter(|n| answer.matches(&format!("\u{27E6}{n}\u{27E7}")).count() == 1)
            .count();
        let extra = &mut record;
        extra["answer"] = json!(answer);
        extra["layer_a_removed"] = json!(removed);
        extra["guards"] = Value::Object(each);
        extra["restore"] = restore_value;
        extra["divergence"] = json!(scores.divergence);
        extra["length_ratio"] = json!(scores.length_ratio);
        extra["word_change"] = json!(measure::word_change(&chunk.text, &answer));
        extra["new_words"] = json!(measure::new_words(&chunk.text, &answer));
        extra["placeholders_kept"] = json!(kept);
        extra["lang_answer"] = json!(measure::detect(&answer).map(|l| l.as_str()));
        extra["preface"] = json!(measure::preface(&chunk.text, &answer));
        extra["trailer"] = json!(measure::trailer(&chunk.text, &answer));
    }
    record["verdict"] = json!(verdict_of.as_ref().map_or("passed", Rejection::kind));
    record["rejection"] = verdict_of.as_ref().map_or(Value::Null, rejection_value);
    record
}

/// The keys already in `path`, so a run that was stopped picks up where
/// it was.
pub fn done(path: &Path) -> HashSet<String> {
    let Ok(file) = std::fs::File::open(path) else {
        return HashSet::new();
    };
    std::io::BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str::<Value>(&line).ok())
        .filter_map(|v| v["key"].as_str().map(str::to_owned))
        .collect()
}

pub fn selected(items: Vec<Item>, args: &Args) -> Vec<Item> {
    let langs = args.list("--langs");
    let only = args.list("--items");
    let every: Option<usize> = args.value("--every").map(|v| v.parse().expect("--every n"));
    items
        .into_iter()
        .filter(|item| langs.is_empty() || langs.iter().any(|l| l == item.lang.as_str()))
        .filter(|item| only.is_empty() || only.iter().any(|p| item.id.starts_with(p.as_str())))
        .enumerate()
        .filter(|(i, item)| {
            // A thinner corpus for a slow model keeps every special case.
            every
                .is_none_or(|n| i % n == 0 || !matches!(item.kind.as_str(), "prose-pd" | "machine"))
        })
        .map(|(_, item)| item)
        .collect()
}

pub fn main(args: &Args) {
    let corpus = Path::new(
        &args
            .value("--corpus")
            .unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/bench/corpus").to_owned()),
    )
    .to_path_buf();
    let items = selected(corpus::load(&corpus), args);
    let cells = grid(
        &args
            .value("--grid")
            .unwrap_or_else(|| DEFAULT_GRID.to_owned()),
    );
    let out = std::path::PathBuf::from(args.required("--out"));
    let (model, engine) = engine::from_args(args);
    let info = engine.info();
    let done = done(&out);
    let overrides = args.value("--variant").map(|dir| variant(Path::new(&dir)));
    if let Some(o) = &overrides {
        eprintln!(
            "variant: {} template(s) overridden",
            Slot::all().iter().filter(|s| o.get(**s).is_some()).count()
        );
    }

    // Plan everything first: the total, and nothing loaded for a typo.
    let mut work = Vec::new();
    for item in &items {
        let document = Document {
            text: item.text.clone(),
            format: item.format,
        };
        for cell in &cells {
            let mut options = options(cell.tactic, cell.intensity);
            tune(args, &mut options);
            if let Some(o) = &overrides {
                options.overrides = o.clone();
            }
            let planned = plan(&document, &options, &info).expect("shipped templates render");
            assert!(
                planned.fallbacks.is_empty(),
                "a variant template did not render: {:?}",
                planned.fallbacks
            );
            let Some(rung) = planned.rung(1).cloned() else {
                eprintln!("skip {} {}: no usable rung", item.id, cell.tactic.as_str());
                continue;
            };
            for chunk in planned.prepared.chunks() {
                for k in 1..=cell.k {
                    let key = key(
                        &model,
                        &item.id,
                        chunk.index,
                        cell.tactic.as_str(),
                        cell.intensity.as_str(),
                        k,
                    );
                    if !done.contains(&key) {
                        work.push((
                            item,
                            cell.clone(),
                            options.clone(),
                            rung.clone(),
                            chunk.clone(),
                            k,
                            key,
                        ));
                    }
                }
            }
        }
    }
    eprintln!(
        "{} items, {} cells, {} attempts to make ({} already in {})",
        items.len(),
        cells.len(),
        work.len(),
        done.len(),
        out.display()
    );
    if work.is_empty() {
        return;
    }

    let started = Instant::now();
    match engine::block_on(engine.warmup()) {
        Ok(()) => eprintln!("loaded in {:.1}s", started.elapsed().as_secs_f32()),
        Err(error) => panic!("the engine did not load: {error:?}"),
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&out)
        .expect("the output file opens");
    let total = work.len();
    let run_started = Instant::now();
    let mut tokens = 0u64;
    for (n, (item, cell, options, rung, chunk, k, key)) in work.into_iter().enumerate() {
        let round = (k - 1) / SEEDS.candidates + 1;
        let candidate = (k - 1) % SEEDS.candidates + 1;
        let seed = seed_for(options.base_seed, SEEDS, chunk.index, round, candidate);
        let started = Instant::now();
        let mut record = attempt(engine.as_ref(), &options, &chunk, &rung, seed);
        let secs = started.elapsed().as_secs_f64();
        let out_tokens: u64 = record["steps"]
            .as_array()
            .map(|s| s.iter().filter_map(|s| s["tokens_out"].as_u64()).sum())
            .unwrap_or(0);
        tokens += out_tokens;
        let meta = json!({
            "key": key,
            "model": model,
            "item": item.id,
            "lang": item.lang.as_str(),
            "kind": item.kind,
            "format": match item.format { TextFormat::Markdown => "markdown", _ => "plain" },
            "chunk": chunk.index,
            "tactic": cell.tactic.as_str(),
            "intensity": cell.intensity.as_str(),
            "k": k,
            "round": round,
            "candidate": candidate,
            "seed": seed,
            "temperature": options.sampling.temperature,
            "top_p": options.sampling.top_p,
            "secs": secs,
            "injection": match (&item.inject, record["answer"].as_str()) {
                (Some(inject), Some(answer)) => json!(measure::obeyed(inject, &chunk.text, answer)),
                (Some(_), None) => json!(false),
                (None, _) => Value::Null,
            },
        });
        for (field, value) in meta.as_object().expect("an object") {
            record[field] = value.clone();
        }
        writeln!(file, "{record}").expect("the record is written");
        let elapsed = run_started.elapsed().as_secs_f64();
        eprintln!(
            "[{}/{total}] {} c{} {} {} k{k}: {} — {out_tokens} tok, {secs:.1}s ({:.1} tok/s overall, eta {:.0} min)",
            n + 1,
            item.id,
            chunk.index,
            cell.tactic.as_str(),
            cell.intensity.as_str(),
            record["verdict"].as_str().unwrap_or("?"),
            tokens as f64 / elapsed,
            elapsed / (n + 1) as f64 * (total - n - 1) as f64 / 60.0,
        );
    }
    eprintln!(
        "done: {total} attempts, {tokens} tokens out in {:.1} min",
        run_started.elapsed().as_secs_f64() / 60.0
    );
}
