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
use wipemark_pipeline::prepare::{estimate_tokens, Chunk, TextFormat};
use wipemark_pipeline::prompt::{
    clean_response, render, Input, Intensity, RenderError, Slot, Stripped, Tactic,
};
use wipemark_pipeline::report::{rejection_value, EngineFailure, Rejection};
use wipemark_pipeline::select::{self, Scorer};
use wipemark_pipeline::{seed_for, Document, Options};

use crate::args::Args;
use crate::corpus::{self, Item};
use crate::{engine, measure, variant};

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
            // E2-dflash2 (D486): what a draft beside the model did for
            // this call.
            "drafted": completion.drafted.map_or(Value::Null, |d| json!({
                "steps": d.steps,
                "proposed": d.proposed,
                "accepted": d.accepted,
            })),
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

    let drafted = drafted_of(&steps);
    let mut record = json!({
        "chunk_text": chunk.text,
        "placeholders": chunk.protected.len(),
        "steps": steps,
    });
    if let Some((verified, proposed, accepted)) = drafted {
        record["draft_steps"] = json!(verified);
        record["draft_proposed"] = json!(proposed);
        record["draft_accepted"] = json!(accepted);
        record["accepted_per_step"] = if verified == 0 {
            Value::Null
        } else {
            json!(f64::from(accepted) / f64::from(verified))
        };
    }
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

/// What the draft did over an attempt's calls — verification steps, tokens
/// proposed, tokens accepted — or `None` when no call had a draft beside
/// the model (D486).
fn drafted_of(steps: &[Value]) -> Option<(u32, u32, u32)> {
    let mut seen = false;
    let mut total = (0_u32, 0_u32, 0_u32);
    for step in steps {
        let drafted = &step["drafted"];
        if drafted.is_null() {
            continue;
        }
        seen = true;
        let n = |field: &str| u32::try_from(drafted[field].as_u64().unwrap_or(0)).unwrap_or(0);
        total.0 += n("steps");
        total.1 += n("proposed");
        total.2 += n("accepted");
    }
    seen.then_some(total)
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

/// The kind of the items that speak to their reader with «ты»/«вы» and
/// du/Sie, added at the end of the ru and de files after E4-5's and the
/// divergence study's runs (E4-8's verification, 2026-10-09).
pub const ADDRESS: &str = "address";

pub fn selected(items: Vec<Item>, args: &Args) -> Vec<Item> {
    let langs = args.list("--langs");
    let only = args.list("--items");
    let every: Option<usize> = args.value("--every").map(|v| v.parse().expect("--every n"));
    // The position `--every` counts by. An `address` item is always kept
    // and never counted, so every other item keeps the place — and the
    // selection — it had before those items were written, and the earlier
    // runs' records line up with a new run's.
    let mut place = 0usize;
    items
        .into_iter()
        .filter(|item| langs.is_empty() || langs.iter().any(|l| l == item.lang.as_str()))
        .filter(|item| only.is_empty() || only.iter().any(|p| item.id.starts_with(p.as_str())))
        .filter(|item| {
            if item.kind == ADDRESS {
                return true;
            }
            let i = place;
            place += 1;
            // A thinner corpus for a slow model keeps every special case.
            every.is_none_or(|n| {
                i.is_multiple_of(n) || !matches!(item.kind.as_str(), "prose-pd" | "machine")
            })
        })
        .collect()
}

pub fn main(args: &Args) {
    attempts(args, false);
}

/// `plan --of run`: what [`main`] would make with the same flags — one line,
/// `attempts=<n> calls=<n> done=<n>` — and nothing loaded. `calls` counts
/// every step of every attempt (an attempt that fails early makes fewer);
/// `bench/run-voice.sh` multiplies it by a measured time per call.
pub fn plan_only(args: &Args) {
    if let Some((attempts, calls, done)) = attempts(args, true) {
        println!("attempts={attempts} calls={calls} done={done}");
    }
}

/// Plan the run `args` asks for; with `plan_only`, return what it would
/// make — attempts, calls, records already done — and load nothing.
fn attempts(args: &Args, plan_only: bool) -> Option<(usize, usize, usize)> {
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
    let overrides = args.value("--variant").map(|named| {
        let (overrides, warnings) = variant::named(&named, info.ctx_len)
            .unwrap_or_else(|refused| panic!("--variant {named}: {refused}"));
        for warning in warnings {
            eprintln!("variant {warning}");
        }
        overrides
    });
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
    if plan_only {
        let calls: usize = work.iter().map(|w| w.3.plan.steps.len()).sum();
        return Some((work.len(), calls, done.len()));
    }
    if work.is_empty() {
        return None;
    }

    let started = Instant::now();
    match engine::block_on(engine.warmup()) {
        Ok(()) => eprintln!("loaded in {:.1}s", started.elapsed().as_secs_f32()),
        Err(error) => panic!("the engine did not load: {error:?}"),
    }
    // E2-dflash2 (D486): the draft every record names is the one that
    // loaded. One asked for and refused would label a run made without it.
    let draft = engine.info().draft;
    if args.value("--draft").is_some() {
        match &draft {
            Some(sha256) => eprintln!("the draft {sha256} decodes beside the model"),
            None => panic!(
                "--draft was given and the draft was not loaded beside the model; the log says why"
            ),
        }
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
            // E2-dflash2 (D486): the draft beside the model — its sha256 —
            // or null.
            "draft": draft,
            "injection": match (&item.inject, record["answer"].as_str()) {
                (Some(inject), Some(answer)) => json!(measure::obeyed(inject, &chunk.text, answer)),
                (Some(_), None) => json!(false),
                (None, _) => Value::Null,
            },
            // E4-8: who the answer speaks to and as, its length in words
            // and the register proxy (D420, D421). `report` recomputes them
            // from the texts; the record carries them so it reads alone.
            "voice": record["answer"].as_str().map_or(Value::Null, |answer| {
                measure::Voice::of(item.lang, &chunk.text, answer).to_json()
            }),
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
    None
}

#[cfg(test)]
mod tests {
    use wipemark_pipeline::lang::Lang;

    use super::*;

    /// D486: an attempt's draft figures are its calls' added up; an
    /// attempt with no draft beside the model has none, not zeros.
    #[test]
    fn an_attempts_draft_figures_are_its_calls_added_up() {
        let plain = vec![json!({"step": 1, "tokens_out": 9, "drafted": null})];
        assert_eq!(drafted_of(&plain), None);
        let drafted = vec![
            json!({"step": 1, "drafted": {"steps": 3, "proposed": 21, "accepted": 12}}),
            json!({"step": 2, "drafted": {"steps": 2, "proposed": 14, "accepted": 2}}),
        ];
        assert_eq!(drafted_of(&drafted), Some((5, 35, 14)));
        let none_proposed = vec![json!({"drafted": {"steps": 0, "proposed": 0, "accepted": 0}})];
        assert_eq!(drafted_of(&none_proposed), Some((0, 0, 0)));
    }

    #[test]
    fn plan_counts_every_step_of_every_attempt_and_loads_nothing() {
        // A model file that does not exist: over the shim a load would be
        // refused and `attempts` would panic, so a count here loaded nothing.
        let args = Args::of(
            "plan",
            &[
                ("--local", "/nowhere/model.gguf"),
                ("--name", "m"),
                ("--out", "/nowhere/records.jsonl"),
                ("--items", "en-mx-01"),
                ("--grid", "paraphrase:moderate:4;back_translate:-:2"),
            ],
        );
        // en-mx-01 is one paragraph: one chunk; back_translate is two steps.
        assert_eq!(attempts(&args, true), Some((4 + 2, 4 + 2 * 2, 0)));
    }

    fn the_corpus() -> Vec<Item> {
        corpus::load(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/bench/corpus"
        )))
    }

    fn ids(items: &[Item]) -> Vec<&str> {
        items.iter().map(|item| item.id.as_str()).collect()
    }

    #[test]
    fn the_address_items_leave_every_other_selection_as_it_was() {
        let corpus = the_corpus();
        let address: Vec<&str> = corpus
            .iter()
            .filter(|item| item.kind == ADDRESS)
            .map(|item| item.id.as_str())
            .collect();
        assert!(
            address.len() >= 8,
            "ru and de, informal and formal: {address:?}"
        );
        let before: Vec<Item> = corpus
            .iter()
            .filter(|item| item.kind != ADDRESS)
            .cloned()
            .collect();
        for flags in [
            vec![("--every", "3")],
            vec![("--every", "2")],
            vec![("--every", "3"), ("--langs", "ru,de")],
            vec![],
        ] {
            let args = Args::of("run", &flags);
            let now = selected(corpus.clone(), &args);
            let then = selected(before.clone(), &args);
            let kept: Vec<&str> = ids(&now)
                .into_iter()
                .filter(|id| !address.contains(id))
                .collect();
            assert_eq!(
                kept,
                ids(&then),
                "{flags:?}: the earlier items, as selected before"
            );
            let langs = args.list("--langs");
            for id in &address {
                if langs.is_empty() || langs.iter().any(|l| id.starts_with(l.as_str())) {
                    assert!(ids(&now).contains(id), "{flags:?}: {id} is always kept");
                }
            }
        }
    }

    /// The chunks of every item as `run` makes them (`job::plan`, over the
    /// shim: nothing is loaded), with the second-person words in each —
    /// informal and formal, by the bench's own measure.
    fn addressed(items: &[Item]) -> Vec<(Lang, u32, u32)> {
        let args = Args::of(
            "plan",
            &[("--local", "/nowhere/model.gguf"), ("--name", "m")],
        );
        let (_, engine) = engine::from_args(&args);
        let info = engine.info();
        let options = options(Tactic::Paraphrase, Intensity::Moderate);
        let mut out = Vec::new();
        for item in items {
            let document = Document {
                text: item.text.clone(),
                format: item.format,
            };
            let planned = plan(&document, &options, &info).expect("the shipped templates render");
            for chunk in planned.prepared.chunks() {
                let persons = measure::Voice::of(item.lang, &chunk.text, "").source;
                out.push((item.lang, persons.informal(), persons.formal));
            }
        }
        out
    }

    #[test]
    fn the_corpus_speaks_to_its_reader_in_every_language() {
        // The figures in docs/plan/reports/E4-8-bench-voice-2026-10-08.md,
        // "Host verification": `-- --nocapture` prints them.
        let corpus = the_corpus();
        let before: Vec<Item> = corpus
            .iter()
            .filter(|item| item.kind != ADDRESS)
            .cloned()
            .collect();
        for (label, items, flags) in [
            ("before the address items, every chunk", &before, vec![]),
            (
                "before the address items, --every 3",
                &before,
                vec![("--every", "3")],
            ),
            ("every chunk", &corpus, vec![]),
            ("--every 3", &corpus, vec![("--every", "3")]),
        ] {
            let chunks = addressed(&selected(items.clone(), &Args::of("run", &flags)));
            for lang in Lang::ALL {
                let of: Vec<&(Lang, u32, u32)> = chunks.iter().filter(|c| c.0 == lang).collect();
                let second = of.iter().filter(|c| c.1 + c.2 > 0).count();
                let informal = of.iter().filter(|c| c.1 > 0 && c.2 == 0).count();
                let formal = of.iter().filter(|c| c.2 > 0 && c.1 == 0).count();
                eprintln!(
                    "{label}: {}: {} chunks, {second} with a second person \
                     ({informal} informal only, {formal} formal only)",
                    lang.as_str(),
                    of.len()
                );
                if lang != Lang::En && items.len() == corpus.len() {
                    assert!(
                        informal >= 2 && formal >= 2,
                        "{label}, {}: {informal} chunks informal, {formal} formal",
                        lang.as_str()
                    );
                }
            }
        }
    }
}
