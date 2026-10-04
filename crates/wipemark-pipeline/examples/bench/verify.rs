//! `verify`: the loop itself — `wipemark_pipeline::start` with the
//! product's GPU options (2 × 2, `[paraphrase]`, moderate, seed 0) — over a
//! few corpus items, and every attempt it made compared with the bench's
//! record for the same chunk and seed. Agreement is what lets the bench's
//! numbers stand for the product's.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use serde_json::Value;
use wipemark_pipeline::cost::Executor;
use wipemark_pipeline::report::Verdict;
use wipemark_pipeline::{start, Document, Event, JobId, Options};

use crate::args::Args;
use crate::{corpus, engine, judge, run};

pub fn main(args: &Args) {
    let records = judge::records(&args.list("--in"));
    let corpus = args
        .value("--corpus")
        .unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/bench/corpus").to_owned());
    let items = run::selected(corpus::load(Path::new(&corpus)), args);
    let (model, engine) = engine::from_args(args);
    let by_key: HashMap<&str, &Value> = records
        .iter()
        .filter_map(|r| Some((r["key"].as_str()?, r)))
        .collect();
    let (mut agree, mut differ, mut missing) = (0, 0, 0);
    for (n, item) in items.iter().enumerate() {
        let document = Document {
            text: item.text.clone(),
            format: item.format,
        };
        let options = Options::for_executor(Executor::LocalGpu);
        let (_handle, events) =
            start(JobId(n as u64), document, options, engine.clone()).expect("the job starts");
        let outcome = loop {
            match events.recv_timeout(Duration::from_secs(1800)) {
                Ok(Event::Finished { outcome, .. }) => break outcome,
                Ok(Event::Failed { error, .. }) => panic!("{}: the job failed: {error}", item.id),
                Ok(Event::Cancelled { .. }) | Err(_) => panic!("{}: the job stopped", item.id),
                Ok(_) => {}
            }
        };
        for chunk in &outcome.report.chunks {
            for attempt in &chunk.attempts {
                let k = (attempt.round - 1) * run::SEEDS.candidates + attempt.candidate;
                let key = run::key(&model, &item.id, chunk.index, "paraphrase", "moderate", k);
                let (kind, divergence) = match &attempt.verdict {
                    Verdict::Passed(scores) => ("passed", Some(scores.divergence)),
                    Verdict::Rejected(rejection) => (rejection.kind(), None),
                };
                let Some(record) = by_key.get(key.as_str()) else {
                    missing += 1;
                    eprintln!("{key}: no bench record");
                    continue;
                };
                let same_kind = record["verdict"] == kind;
                let same_divergence = match divergence {
                    Some(d) => record["divergence"]
                        .as_f64()
                        .is_some_and(|b| (b - f64::from(d)).abs() < 1e-6),
                    None => true,
                };
                if same_kind && same_divergence && record["seed"] == attempt.seed {
                    agree += 1;
                } else {
                    differ += 1;
                    eprintln!(
                        "{key}: loop {kind} {divergence:?} seed {} — bench {} {} seed {}",
                        attempt.seed, record["verdict"], record["divergence"], record["seed"]
                    );
                }
            }
        }
        eprintln!(
            "{}: {} chunk(s) compared",
            item.id,
            outcome.report.chunks.len()
        );
    }
    println!("verify: {agree} attempts agree, {differ} differ, {missing} missing");
}
