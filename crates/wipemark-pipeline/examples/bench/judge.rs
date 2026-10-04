//! `judge`: a second model, at temperature 0, asked whether a rewrite
//! says the same thing as its source — the bench's proxy for meaning
//! drift, which no guard sees.
//!
//! It judges every attempt whose answer passed every check **except**
//! possibly the length guard and the no-op floor (so the length window can
//! be weighed by what it would let through), and a calibration set built
//! from the corpus itself: the chunk against itself (must be
//! `EQUIVALENT`), the chunk without its last sentence and the chunk with a
//! sentence of another item appended (both must be `CHANGED`). A proxy, and
//! reported as one: it is a model's opinion, and it also judges its own
//! answers.

use std::collections::HashSet;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::time::Instant;

use serde_json::{json, Value};
use wipemark_engine::{ChatRequest, RewriteEngine, SamplingParams};

use crate::args::Args;
use crate::{engine, run};

const SYSTEM: &str =
    "You compare two texts for meaning. You answer with exactly one word: EQUIVALENT or CHANGED.";

fn prompt(source: &str, answer: &str) -> String {
    format!(
        "Text A is an original. Text B is a rewrite of it. Marks like \u{27E6}1\u{27E7} stand for protected content such as code or links.\n\n\
         A:\n<<<\n{}\n>>>\n\nB:\n<<<\n{}\n>>>\n\n\
         Does B state the same facts, claims, numbers and names as A, with nothing added, nothing left out and nothing changed in meaning? \
         Differences in wording, word order, style and sentence boundaries do not matter. Answer EQUIVALENT or CHANGED.",
        source.trim(),
        answer.trim()
    )
}

/// `EQUIVALENT`, `CHANGED`, or what came back when it was neither.
pub fn ask(engine: &dyn RewriteEngine, source: &str, answer: &str) -> (String, f64) {
    let request = ChatRequest {
        system: Some(SYSTEM.to_owned()),
        prompt: prompt(source, answer),
        params: SamplingParams {
            temperature: 0.0,
            top_p: 1.0,
            min_p: None,
            seed: Some(0),
            max_tokens: Some(8),
        },
    };
    let started = Instant::now();
    let reply = engine::complete(engine, request)
        .map(|c| c.text)
        .unwrap_or_else(|e| format!("error: {e:?}"));
    let word: String = reply
        .trim()
        .trim_start_matches(|c: char| !c.is_alphabetic())
        .chars()
        .take_while(|c| c.is_alphabetic())
        .collect::<String>()
        .to_uppercase();
    let verdict = match word.as_str() {
        "EQUIVALENT" | "CHANGED" => word,
        _ => format!("other:{}", reply.trim()),
    };
    (verdict, started.elapsed().as_secs_f64())
}

/// The attempts worth judging: an answer that passed every guard but the
/// length guard, restored, and was not refused by the language check (a
/// translation can be faithful and is still not a rewrite).
pub fn judgeable(record: &Value) -> bool {
    let Some(guards) = record["guards"].as_object() else {
        return false;
    };
    record["answer"].is_string()
        && record["restore"].is_null()
        && record["verdict"] != "language"
        && guards
            .iter()
            .all(|(name, v)| name == "length-drift" || v.is_null())
}

fn read(paths: &[String]) -> Vec<Value> {
    let mut out = Vec::new();
    for path in paths {
        let file = std::fs::File::open(path).unwrap_or_else(|e| panic!("{path}: {e}"));
        out.extend(
            std::io::BufReader::new(file)
                .lines()
                .map_while(Result::ok)
                .filter_map(|l| serde_json::from_str::<Value>(&l).ok()),
        );
    }
    out
}

pub fn records(paths: &[String]) -> Vec<Value> {
    read(paths)
}

fn sentences(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let bytes: Vec<(usize, char)> = text.char_indices().collect();
    for (i, &(at, c)) in bytes.iter().enumerate() {
        let next_is_space = bytes.get(i + 1).is_none_or(|&(_, n)| n.is_whitespace());
        if matches!(c, '.' | '!' | '?') && next_is_space {
            let end = at + c.len_utf8();
            out.push(text[start..end].trim());
            start = end;
        }
    }
    if !text[start..].trim().is_empty() {
        out.push(text[start..].trim());
    }
    out
}

pub fn main(args: &Args) {
    let inputs = args.list("--in");
    let out = PathBuf::from(args.required("--out"));
    let (judge, engine) = engine::from_args(args);
    let done = run::done(&out);
    let records = read(&inputs);

    let mut work: Vec<(String, String, String, Value)> = Vec::new();
    // Calibration first: one triple per distinct plain chunk of two or more
    // sentences, from the first model's records.
    let mut seen = HashSet::new();
    let mut chunks: Vec<(String, String)> = Vec::new();
    for r in &records {
        if r["tactic"] == "paraphrase" && r["intensity"] == "moderate" && r["k"] == 1 {
            let text = r["chunk_text"].as_str().unwrap_or("").to_owned();
            let id = format!("{}#{}", r["item"].as_str().unwrap_or(""), r["chunk"]);
            if r["placeholders"] == 0 && sentences(&text).len() >= 2 && seen.insert(id.clone()) {
                chunks.push((id, text));
            }
        }
    }
    for (i, (id, text)) in chunks.iter().enumerate() {
        let parts = sentences(text);
        let dropped = parts[..parts.len() - 1].join(" ");
        let other = &chunks[(i + 7) % chunks.len()].1;
        let added = format!("{} {}", text.trim(), sentences(other)[0]);
        for (case, b) in [
            ("same", text.clone()),
            ("dropped", dropped),
            ("added", added),
        ] {
            let key = format!("calib|{judge}|{id}|{case}");
            if !done.contains(&key) {
                work.push((key, text.clone(), b, json!({"calib": case, "item": id})));
            }
        }
    }
    for r in records.iter().filter(|r| judgeable(r)) {
        let key = format!("judge|{judge}|{}", r["key"].as_str().unwrap_or(""));
        if !done.contains(&key) {
            work.push((
                key,
                r["chunk_text"].as_str().unwrap_or("").to_owned(),
                r["answer"].as_str().unwrap_or("").to_owned(),
                json!({"of": r["key"]}),
            ));
        }
    }
    eprintln!("{} judgements to make", work.len());
    if work.is_empty() {
        return;
    }
    engine::block_on(engine.warmup()).expect("the judge loads");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&out)
        .expect("the output opens");
    let total = work.len();
    for (n, (key, a, b, mut meta)) in work.into_iter().enumerate() {
        let (verdict, secs) = ask(engine.as_ref(), &a, &b);
        meta["key"] = json!(key);
        meta["judge"] = json!(judge);
        meta["verdict"] = json!(verdict);
        meta["secs"] = json!(secs);
        writeln!(file, "{meta}").expect("written");
        if n % 50 == 0 || verdict.starts_with("other") {
            eprintln!("[{}/{total}] {key}: {verdict} ({secs:.2}s)", n + 1);
        }
    }
}
