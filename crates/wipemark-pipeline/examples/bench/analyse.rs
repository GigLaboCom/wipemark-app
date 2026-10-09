//! `report`: the records and the judgements in, `summary.json` and the
//! Markdown tables of `docs/architecture/prompt-bench.md` out.
//!
//! The selection policies are simulated here, on the candidates `run`
//! made (§1.5 of the plan): a candidate *qualifies* under a policy when it
//! passed every check but the no-op floor and its divergence is at least
//! the policy's floor; D61's executors decide which candidates are seen —
//! GPU 2 × 2 (round 2 = *k* 3, 4, only when round 1 had none that
//! qualified), CPU 1 × 2 (*k* 2 only when *k* 1 did not qualify).

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

use serde_json::{json, Map, Value};
use wipemark_pipeline::lang::Lang;

use crate::args::Args;
use crate::judge;
use crate::measure::{kept_bigrams, percentile, Voice};

const GUARDS: [&str; 5] = [
    "placeholder",
    "numbers",
    "length-drift",
    "script",
    "identifier",
];

#[derive(Clone, Copy)]
pub struct Policy {
    pub name: &'static str,
    pub floor: f64,
    pub max: bool,
}

/// The loop's policy until E4-7 (D71).
pub const E4_3: Policy = POLICIES[0];
/// The loop's policy since E4-7 (D95): the most diverged, floor 0.2.
pub const E4_7: Policy = POLICIES[2];
/// The cells the policy tables simulate, in their order.
const CELLS: [(&str, &str); 6] = [
    ("paraphrase", "light"),
    ("paraphrase", "moderate"),
    ("paraphrase", "strong"),
    ("humanize", "moderate"),
    ("humanize", "strong"),
    ("back_translate", "moderate"),
];

/// The least diverged at E4-7's floor — the research's "cheap middle"
/// (`divergence-vs-upstream-2026-10-07.md`), beside E4-7 in the voice table.
pub const VOICE_BESIDE: Policy = POLICIES[3];

pub const POLICIES: [Policy; 8] = [
    Policy {
        name: "min ≥ 0.05 (E4-3)",
        floor: 0.05,
        max: false,
    },
    Policy {
        name: "max ≥ 0.05",
        floor: 0.05,
        max: true,
    },
    Policy {
        name: "max ≥ 0.2 (E4-7)",
        floor: 0.2,
        max: true,
    },
    Policy {
        name: "min ≥ 0.2",
        floor: 0.2,
        max: false,
    },
    Policy {
        name: "min ≥ 0.3",
        floor: 0.3,
        max: false,
    },
    Policy {
        name: "min ≥ 0.4",
        floor: 0.4,
        max: false,
    },
    // Floors where bigram divergence starts to mean "most words are new":
    // the owner's question asked for 0.2–0.4, which the measurements show
    // almost never bind.
    Policy {
        name: "min ≥ 0.6",
        floor: 0.6,
        max: false,
    },
    Policy {
        name: "min ≥ 0.75",
        floor: 0.75,
        max: false,
    },
];

fn s<'a>(r: &'a Value, f: &str) -> &'a str {
    r[f].as_str().unwrap_or("")
}

fn f(r: &Value, field: &str) -> Option<f64> {
    r[field].as_f64()
}

/// Passed every check but, perhaps, the no-op floor.
pub fn basic(r: &Value) -> bool {
    matches!(s(r, "verdict"), "passed" | "no-op")
}

fn kept(r: &Value) -> f64 {
    f64::from(kept_bigrams(s(r, "chunk_text"), s(r, "answer")))
}

/// The language a record's voice is measured in: the item's — its id's
/// prefix in the bench's corpus, else the `lang` the run wrote (a corpus of
/// one's own names its items freely).
fn lang_of(r: &Value) -> Option<Lang> {
    Lang::parse(s(r, "item").split('-').next().unwrap_or("")).or_else(|| Lang::parse(s(r, "lang")))
}

/// The voice of an answered record, recomputed from its texts as the
/// preface and the trailer are (D422): a record made before E4-8 has one
/// too, and a changed register list needs no rerun of the models.
fn voice_of(r: &Value) -> Option<Voice> {
    Some(Voice::of(
        lang_of(r)?,
        s(r, "chunk_text"),
        r["answer"].as_str()?,
    ))
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// The median and the quartiles: the voice measures' spread (E4-8). The
/// older measures keep their p10–p90.
fn quartiles(values: &[f64]) -> Value {
    let p = |q| percentile(values, q).map(round3);
    json!({"n": values.len(), "q1": p(25.0), "median": p(50.0), "q3": p(75.0)})
}

fn quart_md(d: &Value) -> String {
    match (d["q1"].as_f64(), d["median"].as_f64(), d["q3"].as_f64()) {
        (Some(a), Some(m), Some(b)) => format!("{m:.2} [{a:.2}–{b:.2}]"),
        _ => "–".into(),
    }
}

/// What a set of rewrites did to the voice — one [`Voice`] per chunk (or
/// per attempt): the second person kept by count over every chunk that had
/// one and per chunk, the chunks that lost any or all of it, the ru/de
/// chunks addressed one way and answered the other, the first person, the
/// length in words and the register proxy (D420, D421).
fn voice_summary(voices: &[Voice]) -> Value {
    let with_second = voices.iter().filter(|v| v.source.second > 0).count();
    let second: (u32, u32) = voices.iter().fold((0, 0), |(s, k), v| {
        (
            s + v.source.second,
            k + v.answer.second.min(v.source.second),
        )
    });
    let first: (u32, u32) = voices.iter().fold((0, 0), |(s, k), v| {
        (s + v.source.first, k + v.answer.first.min(v.source.first))
    });
    let words: (u32, u32) = voices
        .iter()
        .fold((0, 0), |(s, a), v| (s + v.words.0, a + v.words.1));
    let count = |p: &dyn Fn(&Voice) -> bool| voices.iter().filter(|v| p(v)).count();
    let shifts: Vec<f64> = voices.iter().filter_map(Voice::register_shift).collect();
    json!({
        "n": voices.len(),
        "with_second": with_second,
        "second": [second.0, second.1],
        "second_kept_all": ratio(second.1 as usize, second.0 as usize),
        "second_kept_per_chunk": quartiles(&voices.iter().filter_map(Voice::second_kept).collect::<Vec<_>>()),
        "lost_any": ratio(count(&|v| v.lost_second() == Some(true)), with_second),
        "lost_all": ratio(count(&|v| v.lost_all_second() == Some(true)), with_second),
        "address": count(&|v| v.switched.is_some()),
        "switched": count(&|v| v.switched == Some(true)),
        "first": [first.0, first.1],
        "first_kept_all": ratio(first.1 as usize, first.0 as usize),
        "first_kept_per_chunk": quartiles(&voices.iter().filter_map(Voice::first_kept).collect::<Vec<_>>()),
        "words": [words.0, words.1],
        "words_ratio_all": ratio(words.1 as usize, words.0 as usize),
        "words_ratio": quartiles(&voices.iter().filter_map(Voice::words_ratio).collect::<Vec<_>>()),
        "register_shift": quartiles(&shifts),
        "register_shift_mean": if shifts.is_empty() { Value::Null } else { json!(round3(shifts.iter().sum::<f64>() / shifts.len() as f64)) },
    })
}

fn pct(n: usize, d: usize) -> String {
    if d == 0 {
        "–".into()
    } else {
        format!("{:.0}%", 100.0 * n as f64 / d as f64)
    }
}

fn ratio(n: usize, d: usize) -> Value {
    if d == 0 {
        Value::Null
    } else {
        json!((n as f64 / d as f64 * 1000.0).round() / 1000.0)
    }
}

fn dist(values: &[f64]) -> Value {
    let p = |q| percentile(values, q).map(|v| (v * 1000.0).round() / 1000.0);
    json!({"n": values.len(), "p10": p(10.0), "median": p(50.0), "p90": p(90.0)})
}

fn dist_md(d: &Value) -> String {
    match (d["p10"].as_f64(), d["median"].as_f64(), d["p90"].as_f64()) {
        (Some(a), Some(m), Some(b)) => format!("{m:.2} ({a:.2}–{b:.2})"),
        _ => "–".into(),
    }
}

fn steps_tokens(r: &Value) -> u64 {
    r["steps"]
        .as_array()
        .map(|s| s.iter().filter_map(|s| s["tokens_out"].as_u64()).sum())
        .unwrap_or(0)
}

/// Every attempt measure of §1.3 over `rs`.
fn metrics(rs: &[&Value]) -> Value {
    let n = rs.len();
    let count = |p: &dyn Fn(&Value) -> bool| rs.iter().filter(|r| p(r)).count();
    let answered: Vec<&&Value> = rs.iter().filter(|r| r["answer"].is_string()).collect();
    let a = answered.len();
    let vals = |field: &str| -> Vec<f64> { answered.iter().filter_map(|r| f(r, field)).collect() };
    let mut guards = Map::new();
    for g in GUARDS {
        // `NumbersGuard` reads a placeholder's own digits as a number, so a
        // lost `⟦2⟧` is also a "missing 2": counted under the placeholder
        // guard only.
        let failed = answered
            .iter()
            .filter(|r| !r["guards"][g].is_null())
            .filter(|r| g != "numbers" || r["guards"]["placeholder"].is_null())
            .count();
        guards.insert(g.into(), ratio(failed, a));
    }
    let lang_same = answered
        .iter()
        .filter(|r| r["lang_answer"] == r["lang_doc"])
        .count();
    let lang_other = answered
        .iter()
        .filter(|r| r["lang_answer"].is_string() && r["lang_answer"] != r["lang_doc"])
        .count();
    let lang_unknown = answered
        .iter()
        .filter(|r| r["lang_answer"].is_null())
        .count();
    let tokens: u64 = rs.iter().map(|r| steps_tokens(r)).sum();
    let secs: f64 = rs.iter().filter_map(|r| f(r, "secs")).sum();
    json!({
        "attempts": n,
        "answered": a,
        "passed": ratio(count(&|r| s(r, "verdict") == "passed"), n),
        "guard_failed": guards,
        "restore_failed": ratio(answered.iter().filter(|r| !r["restore"].is_null()).count(), a),
        "no_op": ratio(count(&|r| s(r, "verdict") == "no-op"), n),
        "truncated": ratio(count(&|r| s(r, "verdict") == "truncated"), n),
        "empty_or_engine": ratio(count(&|r| matches!(s(r, "verdict"), "empty" | "engine")), n),
        "divergence": dist(&vals("divergence")),
        "word_change": dist(&vals("word_change")),
        "new_words": dist(&vals("new_words")),
        "kept_bigrams": dist(&answered.iter().map(|r| kept(r)).collect::<Vec<_>>()),
        "length_ratio": dist(&vals("length_ratio")),
        "lang_kept": ratio(lang_same, a),
        "lang_other": ratio(lang_other, a),
        "lang_unknown": ratio(lang_unknown, a),
        "preface": ratio(answered.iter().filter(|r| r["preface"] == true).count(), a),
        "preface_passed": ratio(rs.iter().filter(|r| r["preface"] == true && s(r, "verdict") == "passed").count(), n),
        "trailer": ratio(answered.iter().filter(|r| r["trailer"] == true).count(), a),
        "stripped": ratio(rs.iter().filter(|r| r["steps"].as_array().is_some_and(|st| st.iter().any(|x| x["stripped"].as_array().is_some_and(|y| !y.is_empty())))).count(), n),
        "layer_a_removed": ratio(answered.iter().filter(|r| r["layer_a_removed"].as_u64().unwrap_or(0) > 0).count(), a),
        "tokens_per_call": if n == 0 { Value::Null } else { json!((tokens as f64 / n as f64).round()) },
        "secs_per_attempt": if n == 0 { Value::Null } else { json!((secs / n as f64 * 100.0).round() / 100.0) },
        "tokens_per_second": if secs == 0.0 { Value::Null } else { json!((tokens as f64 / secs * 10.0).round() / 10.0) },
        "voice": voice_summary(&answered.iter().filter_map(|r| voice_of(r)).collect::<Vec<_>>()),
    })
}

/// One chunk's candidates of one cell, by `k`.
type Group<'a> = BTreeMap<u64, &'a Value>;

struct Pick<'a> {
    winner: Option<&'a Value>,
    /// The chunk's text, as the model saw it.
    source: &'a str,
    /// The item's language, for the voice measures.
    lang: Option<Lang>,
    calls: usize,
    secs: f64,
}

/// A text's words outside its placeholders, and its word pairs.
fn sizes(text: &str) -> (f64, f64) {
    let n = crate::measure::words(text)
        .iter()
        .filter(|w| !w.starts_with('\u{27E6}'))
        .count();
    (n as f64, n.saturating_sub(1) as f64)
}

fn simulate<'a>(group: &Group<'a>, policy: Policy, gpu: bool) -> Option<Pick<'a>> {
    let ok = |r: &Value| basic(r) && f(r, "divergence").is_some_and(|d| d >= policy.floor);
    let rounds: Vec<Vec<u64>> = if gpu {
        vec![vec![1, 2], vec![3, 4]]
    } else {
        vec![vec![1], vec![2]]
    };
    if rounds.iter().flatten().any(|k| !group.contains_key(k)) {
        return None;
    }
    let source = group.values().next().map_or("", |r| s(r, "chunk_text"));
    let lang = group.values().next().and_then(|r| lang_of(r));
    let mut calls = 0;
    let mut secs = 0.0;
    for round in rounds {
        let seen: Vec<&Value> = round.iter().map(|k| group[k]).collect();
        for r in &seen {
            calls += r["steps"].as_array().map_or(0, Vec::len);
            secs += f(r, "secs").unwrap_or(0.0);
        }
        let mut passed: Vec<&Value> = seen.into_iter().filter(|r| ok(r)).collect();
        if !passed.is_empty() {
            // Ties go to the earlier attempt, as `select::winner` does.
            let by = |r: &&Value| f(r, "divergence").unwrap_or(0.0);
            let winner = if policy.max {
                passed
                    .iter()
                    .copied()
                    .rev()
                    .max_by(|a, b| by(a).total_cmp(&by(b)))
            } else {
                passed.sort_by(|a, b| by(a).total_cmp(&by(b)));
                passed.first().copied()
            };
            return Some(Pick {
                winner,
                source,
                lang,
                calls,
                secs,
            });
        }
    }
    Some(Pick {
        winner: None,
        source,
        lang,
        calls,
        secs,
    })
}

fn policy_row(
    groups: &[&Group],
    policy: Policy,
    gpu: bool,
    verdicts: &HashMap<String, String>,
    voices: &HashMap<String, String>,
) -> Value {
    let picks: Vec<Pick> = groups
        .iter()
        .filter_map(|g| simulate(g, policy, gpu))
        .collect();
    let n = picks.len();
    let winners: Vec<&Value> = picks.iter().filter_map(|p| p.winner).collect();
    let judged: Vec<&String> = winners
        .iter()
        .filter_map(|w| verdicts.get(s(w, "key")))
        .collect();
    let changed = judged.iter().filter(|v| v.as_str() == "CHANGED").count();
    let div: Vec<f64> = winners.iter().filter_map(|w| f(w, "divergence")).collect();
    let wc: Vec<f64> = winners.iter().filter_map(|w| f(w, "word_change")).collect();
    // A chunk kept as it was carries every pair over.
    let carried: Vec<f64> = picks.iter().map(|p| p.winner.map_or(1.0, kept)).collect();
    // The same, by the document's size rather than by chunk — what a
    // detector reading the whole document sees, and the only comparison
    // that survives a change of chunking (E4-7: a list item per chunk).
    let (mut pairs, mut pairs_kept, mut words, mut words_rewritten) = (0.0, 0.0, 0.0, 0.0);
    for p in &picks {
        let (w, _) = sizes(p.source);
        words += w;
        match p.winner {
            Some(winner) => {
                words_rewritten += w;
                let (_, n) = sizes(s(winner, "answer"));
                pairs += n;
                pairs_kept += kept(winner) * n;
            }
            None => {
                let (_, n) = sizes(p.source);
                pairs += n;
                pairs_kept += n;
            }
        }
    }
    let by_words = |part: f64, whole: f64| {
        if whole == 0.0 {
            Value::Null
        } else {
            json!((part / whole * 1000.0).round() / 1000.0)
        }
    };
    // The voice of what the policy ships: each chunk's winner, or its
    // source where nothing qualified (a kept chunk keeps its voice whole).
    let shipped: Vec<Voice> = picks
        .iter()
        .filter_map(|p| {
            let lang = p.lang?;
            Some(Voice::of(
                lang,
                p.source,
                p.winner.map_or(p.source, |w| s(w, "answer")),
            ))
        })
        .collect();
    let mut voice = voice_summary(&shipped);
    let judged_voice: Vec<&String> = winners
        .iter()
        .filter_map(|w| voices.get(s(w, "key")))
        .collect();
    let said = |answer: &str| judged_voice.iter().filter(|v| v.as_str() == answer).count();
    voice["judged"] = json!({"n": judged_voice.len(), "yes": said("YES"), "partly": said("PARTLY"), "no": said("NO")});
    json!({
        "policy": policy.name,
        "executor": if gpu { "gpu-2x2" } else { "cpu-1x2" },
        "chunks": n,
        "rewritten": ratio(winners.len(), n),
        "divergence": dist(&div),
        "word_change": dist(&wc),
        "kept_bigrams_all_chunks": dist(&carried),
        "kept_bigrams_mean_all_chunks": if carried.is_empty() { Value::Null } else { json!((carried.iter().sum::<f64>() / carried.len() as f64 * 1000.0).round() / 1000.0) },
        "kept_bigrams_by_words": by_words(pairs_kept, pairs),
        "rewritten_by_words": by_words(words_rewritten, words),
        "judged": judged.len(),
        "changed": ratio(changed, judged.len()),
        "calls_per_chunk": if n == 0 { Value::Null } else { json!((picks.iter().map(|p| p.calls).sum::<usize>() as f64 / n as f64 * 100.0).round() / 100.0) },
        "secs_per_chunk": if n == 0 { Value::Null } else { json!((picks.iter().map(|p| p.secs).sum::<f64>() / n as f64 * 10.0).round() / 10.0) },
        "voice": voice,
    })
}

pub fn main(args: &Args) {
    // `--in label=path` files a run under `label` instead of its model's
    // name: a template variant beside the run it is compared with.
    let mut records = Vec::new();
    for entry in args.list("--in") {
        let (label, path) = match entry.split_once('=') {
            Some((label, path)) => (Some(label.to_owned()), path.to_owned()),
            None => (None, entry),
        };
        for mut r in judge::records(&[path]) {
            if let Some(label) = &label {
                let rest = s(&r, "key")
                    .split_once('|')
                    .map_or(String::new(), |(_, rest)| rest.to_owned());
                r["key"] = json!(format!("{label}|{rest}"));
                r["model"] = json!(label);
            }
            records.push(r);
        }
    }
    let judgements = judge::records(&args.list("--judge"));
    let examples = args.value("--examples").map(|n| n.parse().unwrap_or(3));
    let (md, summary) = tables(records, &judgements, examples);
    if let Some(path) = args.value("--summary") {
        std::fs::write(&path, summary_text(&summary)).unwrap_or_else(|e| panic!("{path}: {e}"));
        eprintln!("wrote {path}");
    }
    print!("{md}");
}

/// `report` without its files: the Markdown tables of `records` judged by
/// `judgements` — and, with `examples`, that many before/after examples per
/// model and language — and the summary. Records made before a measure
/// existed still report: what can be recomputed from the texts is (the
/// preface, the trailer, the voice), and what cannot (a judge's voice
/// answer) is shown as "–".
pub fn tables(
    mut records: Vec<Value>,
    judgements: &[Value],
    examples: Option<usize>,
) -> (String, Map<String, Value>) {
    for r in &mut records {
        // The document's language is the item's (its id's prefix);
        // `lang_answer` is the detection over the answer.
        r["lang_doc"] = json!(s(r, "item").split('-').next().unwrap_or(""));
        r["lang_chunk"] = json!(crate::measure::detect(s(r, "chunk_text")).map(|l| l.as_str()));
        // Recomputed from the texts, so a better detector needs no rerun.
        if let Some(answer) = r["answer"].as_str().map(str::to_owned) {
            let source = s(r, "chunk_text").to_owned();
            r["preface"] = json!(crate::measure::preface(&source, &answer));
            r["trailer"] = json!(crate::measure::trailer(&source, &answer));
        }
    }
    let answers = |field: &str| -> HashMap<String, String> {
        judgements
            .iter()
            .filter_map(|j| Some((j["of"].as_str()?.to_owned(), j[field].as_str()?.to_owned())))
            .collect()
    };
    let verdicts = answers(judge::Question::Meaning.field());
    // E4-8: the judge's voice answer, a line of its own beside the meaning's.
    let voices = answers(judge::Question::Voice.field());

    let mut models: Vec<String> = Vec::new();
    for r in &records {
        let m = s(r, "model").to_owned();
        if !models.contains(&m) {
            models.push(m);
        }
    }
    let langs = ["en", "ru", "de"];
    let mut md = String::new();
    let mut summary = Map::new();

    // T1/T2: per model × language × tactic × intensity.
    let mut cells: BTreeMap<(usize, String, String, String), Vec<&Value>> = BTreeMap::new();
    for r in &records {
        let m = models.iter().position(|m| m == s(r, "model")).unwrap_or(0);
        cells
            .entry((
                m,
                s(r, "tactic").into(),
                s(r, "intensity").into(),
                s(r, "lang_doc").into(),
            ))
            .or_default()
            .push(r);
    }
    let mut table = Vec::new();
    let _ = writeln!(
        md,
        "### Attempts: per model, tactic, intensity and language\n"
    );
    let _ = writeln!(md, "| model | tactic | intensity | lang | n | passed | placeholder ✗ | numbers ✗ | length ✗ | script ✗ | identifier ✗ | restore ✗ | no-op | trunc. | divergence med (p10–p90) | word change med (p10–p90) | pairs carried over med | length ratio med (p10–p90) | lang kept / other | preface (passed) | s/attempt |");
    let _ = writeln!(
        md,
        "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    for ((m, tactic, intensity, lang), rs) in &cells {
        let x = metrics(rs);
        let g = |name: &str| fmt_ratio(&x["guard_failed"][name]);
        let _ = writeln!(
            md,
            "| {} | {tactic} | {intensity} | {lang} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} / {} | {} ({}) | {} |",
            models[*m],
            x["attempts"],
            fmt_ratio(&x["passed"]),
            g("placeholder"),
            g("numbers"),
            g("length-drift"),
            g("script"),
            g("identifier"),
            fmt_ratio(&x["restore_failed"]),
            fmt_ratio(&x["no_op"]),
            fmt_ratio(&x["truncated"]),
            dist_md(&x["divergence"]),
            dist_md(&x["word_change"]),
            x["kept_bigrams"]["median"].as_f64().map_or("–".into(), |v| format!("{v:.2}")),
            dist_md(&x["length_ratio"]),
            fmt_ratio(&x["lang_kept"]),
            fmt_ratio(&x["lang_other"]),
            fmt_ratio(&x["preface"]),
            fmt_ratio(&x["preface_passed"]),
            x["secs_per_attempt"],
        );
        let mut row = x;
        row["model"] = json!(models[*m]);
        row["tactic"] = json!(tactic);
        row["intensity"] = json!(intensity);
        row["lang"] = json!(lang);
        table.push(row);
    }
    summary.insert("attempts".into(), Value::Array(table));

    // Per model, every attempt: speed.
    let _ = writeln!(md, "\n### Speed per model (every attempt)\n");
    let _ = writeln!(
        md,
        "| model | attempts | tokens out / call | s / attempt | tokens/s (incl. prompt) |"
    );
    let _ = writeln!(md, "|---|---|---|---|---|");
    let mut speed = Vec::new();
    for (i, model) in models.iter().enumerate() {
        let rs: Vec<&Value> = records.iter().filter(|r| s(r, "model") == model).collect();
        let x = metrics(&rs);
        let _ = writeln!(
            md,
            "| {model} | {} | {} | {} | {} |",
            x["attempts"], x["tokens_per_call"], x["secs_per_attempt"], x["tokens_per_second"]
        );
        speed.push(json!({"model": model, "order": i, "attempts": x["attempts"], "tokens_per_call": x["tokens_per_call"], "secs_per_attempt": x["secs_per_attempt"], "tokens_per_second": x["tokens_per_second"]}));
    }
    summary.insert("speed".into(), Value::Array(speed));

    // By kind, paraphrase moderate.
    let kinds = [
        "prose-pd",
        "machine",
        "markdown",
        "numbers",
        "injection",
        "short",
        "quote",
    ];
    let _ = writeln!(
        md,
        "\n### `paraphrase`, moderate: pass rate by kind of text\n"
    );
    let _ = writeln!(md, "| model | {} |", kinds.join(" | "));
    let _ = writeln!(md, "|---|{}", "---|".repeat(kinds.len()));
    let mut by_kind = Vec::new();
    for model in &models {
        let mut line = format!("| {model} |");
        for kind in kinds {
            let rs: Vec<&Value> = records
                .iter()
                .filter(|r| {
                    s(r, "model") == model
                        && s(r, "tactic") == "paraphrase"
                        && s(r, "intensity") == "moderate"
                        && s(r, "kind") == kind
                })
                .collect();
            let passed = rs.iter().filter(|r| s(r, "verdict") == "passed").count();
            let _ = write!(line, " {} of {} |", passed, rs.len());
            by_kind.push(
                json!({"model": model, "kind": kind, "attempts": rs.len(), "passed": passed}),
            );
        }
        let _ = writeln!(md, "{line}");
    }
    summary.insert("paraphrase_moderate_by_kind".into(), Value::Array(by_kind));

    // Rejection reasons, paraphrase moderate, all languages.
    let _ = writeln!(md, "\n### What rejected a candidate (`paraphrase`, all intensities; the loop's first reason)\n");
    let _ = writeln!(md, "| model | reason | n | share of attempts |");
    let _ = writeln!(md, "|---|---|---|---|");
    let mut reasons_out = Vec::new();
    for model in &models {
        let rs: Vec<&Value> = records
            .iter()
            .filter(|r| s(r, "model") == model && s(r, "tactic") == "paraphrase")
            .collect();
        let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
        for r in &rs {
            if s(r, "verdict") != "passed" {
                let reason = match s(r, "verdict") {
                    "guard" => format!(
                        "guard {} ({})",
                        r["rejection"]["guard"].as_str().unwrap_or("?"),
                        r["rejection"]["reason"].as_str().unwrap_or("?")
                    ),
                    "restore" => format!(
                        "restore ({})",
                        r["rejection"]["reason"].as_str().unwrap_or("?")
                    ),
                    other => other.to_owned(),
                };
                *reasons.entry(reason).or_default() += 1;
            }
        }
        let mut sorted: Vec<_> = reasons.into_iter().collect();
        sorted.sort_by_key(|entry| std::cmp::Reverse(entry.1));
        for (reason, n) in sorted {
            let _ = writeln!(md, "| {model} | {reason} | {n} | {} |", pct(n, rs.len()));
            reasons_out.push(json!({"model": model, "reason": reason, "n": n, "of": rs.len()}));
        }
    }
    summary.insert("paraphrase_rejections".into(), Value::Array(reasons_out));

    // Placeholders.
    let _ = writeln!(
        md,
        "\n### Placeholders `⟦n⟧` (attempts on chunks that carry one)\n"
    );
    let _ = writeln!(md, "| model | lang | attempts | every placeholder exactly once | placeholders kept / expected |");
    let _ = writeln!(md, "|---|---|---|---|---|");
    let mut ph = Vec::new();
    for model in &models {
        for lang in langs {
            let rs: Vec<&Value> = records
                .iter()
                .filter(|r| {
                    s(r, "model") == model
                        && s(r, "lang_doc") == lang
                        && r["placeholders"].as_u64().unwrap_or(0) > 0
                        && r["answer"].is_string()
                })
                .collect();
            let all = rs
                .iter()
                .filter(|r| r["guards"]["placeholder"].is_null())
                .count();
            let expected: u64 = rs
                .iter()
                .map(|r| r["placeholders"].as_u64().unwrap_or(0))
                .sum();
            let kept: u64 = rs
                .iter()
                .map(|r| r["placeholders_kept"].as_u64().unwrap_or(0))
                .sum();
            let _ = writeln!(
                md,
                "| {model} | {lang} | {} | {} | {} |",
                rs.len(),
                pct(all, rs.len()),
                pct(kept as usize, expected as usize)
            );
            ph.push(json!({"model": model, "lang": lang, "attempts": rs.len(), "all_kept": ratio(all, rs.len()), "kept": ratio(kept as usize, expected as usize)}));
        }
    }
    summary.insert("placeholders".into(), Value::Array(ph));

    // Injection.
    let _ = writeln!(md, "\n### Planted instructions (3 items per language)\n");
    let _ = writeln!(
        md,
        "| model | tactic | attempts | obeyed | obeyed **and** passed every check |"
    );
    let _ = writeln!(md, "|---|---|---|---|---|");
    let mut inj = Vec::new();
    for model in &models {
        let mut tactics: Vec<&str> = records
            .iter()
            .filter(|r| s(r, "model") == model)
            .map(|r| s(r, "tactic"))
            .collect();
        tactics.sort_unstable();
        tactics.dedup();
        for tactic in tactics {
            let rs: Vec<&Value> = records
                .iter()
                .filter(|r| {
                    s(r, "model") == model && s(r, "tactic") == tactic && !r["injection"].is_null()
                })
                .collect();
            let obeyed = rs.iter().filter(|r| r["injection"] == true).count();
            let shipped = rs
                .iter()
                .filter(|r| r["injection"] == true && s(r, "verdict") == "passed")
                .count();
            let _ = writeln!(
                md,
                "| {model} | {tactic} | {} | {obeyed} ({}) | {shipped} |",
                rs.len(),
                pct(obeyed, rs.len())
            );
            inj.push(json!({"model": model, "tactic": tactic, "attempts": rs.len(), "obeyed": obeyed, "obeyed_and_passed": shipped}));
        }
    }
    summary.insert("injection".into(), Value::Array(inj));

    // Policies.
    let mut groups: BTreeMap<(usize, String, String, String, String, u64), Group> = BTreeMap::new();
    for r in &records {
        let m = models.iter().position(|m| m == s(r, "model")).unwrap_or(0);
        groups
            .entry((
                m,
                s(r, "tactic").into(),
                s(r, "intensity").into(),
                s(r, "lang_doc").into(),
                s(r, "item").into(),
                r["chunk"].as_u64().unwrap_or(0),
            ))
            .or_default()
            .insert(r["k"].as_u64().unwrap_or(0), r);
    }
    let mut pol = Vec::new();
    for gpu in [true, false] {
        let _ = writeln!(
            md,
            "\n### Selection policies, {} — every language\n",
            if gpu {
                "GPU 2 × 2 (D61)"
            } else {
                "CPU 1 × 2 (D61)"
            }
        );
        let _ = writeln!(md, "| model | tactic | intensity | policy | chunks | rewritten | rewritten, by words | divergence med (p10–p90) | word change med (p10–p90) | pairs carried over, mean (all chunks) | pairs carried over, by words | judged CHANGED | calls / chunk | s / chunk |");
        let _ = writeln!(
            md,
            "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
        );
        for (m, model) in models.iter().enumerate() {
            for (tactic, intensity) in CELLS {
                let these: Vec<&Group> = groups
                    .iter()
                    .filter(|((gm, t, i, _, _, _), _)| *gm == m && t == tactic && i == intensity)
                    .map(|(_, g)| g)
                    .collect();
                for policy in POLICIES {
                    let mut row = policy_row(&these, policy, gpu, &verdicts, &voices);
                    if row["chunks"] == 0 {
                        continue;
                    }
                    let _ = writeln!(
                        md,
                        "| {model} | {tactic} | {intensity} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                        policy.name,
                        row["chunks"],
                        fmt_ratio(&row["rewritten"]),
                        fmt_ratio(&row["rewritten_by_words"]),
                        dist_md(&row["divergence"]),
                        dist_md(&row["word_change"]),
                        fmt_ratio(&row["kept_bigrams_mean_all_chunks"]),
                        fmt_ratio(&row["kept_bigrams_by_words"]),
                        if row["judged"] == 0 { "–".to_owned() } else { format!("{} of {}", fmt_ratio(&row["changed"]), row["judged"]) },
                        row["calls_per_chunk"],
                        row["secs_per_chunk"],
                    );
                    row["model"] = json!(model);
                    row["tactic"] = json!(tactic);
                    row["intensity"] = json!(intensity);
                    row["lang"] = json!("all");
                    pol.push(row);
                    // Per language, for the summary and the per-language
                    // table: the GPU's, for the policies the report weighs.
                    let weighed = [E4_3.name, E4_7.name, VOICE_BESIDE.name, "min ≥ 0.6"];
                    if !gpu || !weighed.contains(&policy.name) {
                        continue;
                    }
                    for lang in langs {
                        let per: Vec<&Group> = groups
                            .iter()
                            .filter(|((gm, t, i, l, _, _), _)| {
                                *gm == m && t == tactic && i == intensity && l == lang
                            })
                            .map(|(_, g)| g)
                            .collect();
                        let mut row = policy_row(&per, policy, gpu, &verdicts, &voices);
                        row["model"] = json!(model);
                        row["tactic"] = json!(tactic);
                        row["intensity"] = json!(intensity);
                        row["lang"] = json!(lang);
                        pol.push(row);
                    }
                }
            }
        }
    }
    // The paragraphs that come back as they were: the largest part of what a
    // rewrite carries over is the chunks it did not rewrite at all.
    let _ = writeln!(
        md,
        "\n### Paragraphs kept as they were — GPU 2 × 2, `paraphrase` moderate, the loop's policy (E4-3 before, E4-7 after)\n"
    );
    let _ = writeln!(
        md,
        "| model | policy | chunks | kept as they were | by kind | why their attempts failed |"
    );
    let _ = writeln!(md, "|---|---|---|---|---|---|");
    let mut kept_out = Vec::new();
    for (m, model) in models.iter().enumerate() {
        for policy in [E4_3, E4_7] {
            let these: Vec<&Group> = groups
                .iter()
                .filter(|((gm, t, i, _, _, _), _)| *gm == m && t == "paraphrase" && i == "moderate")
                .map(|(_, g)| g)
                .collect();
            let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
            let mut why: BTreeMap<String, usize> = BTreeMap::new();
            let mut n = 0;
            let mut kept_n = 0;
            for g in &these {
                let Some(pick) = simulate(g, policy, true) else {
                    continue;
                };
                n += 1;
                if pick.winner.is_some() {
                    continue;
                }
                kept_n += 1;
                let first = g.values().next().expect("a candidate");
                *kinds.entry(s(first, "kind").to_owned()).or_default() += 1;
                for r in g.values() {
                    let reason = match s(r, "verdict") {
                        "guard" => r["rejection"]["guard"].as_str().unwrap_or("?").to_owned(),
                        "restore" => format!(
                            "restore {}",
                            r["rejection"]["reason"].as_str().unwrap_or("?")
                        ),
                        other => other.to_owned(),
                    };
                    *why.entry(reason).or_default() += 1;
                }
            }
            let join = |map: &BTreeMap<String, usize>| {
                let mut v: Vec<_> = map.iter().collect();
                v.sort_by(|a, b| b.1.cmp(a.1));
                v.iter()
                    .map(|(k, n)| format!("{k} {n}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let _ = writeln!(
                md,
                "| {model} | {} | {n} | {kept_n} ({}) | {} | {} |",
                policy.name,
                pct(kept_n, n),
                join(&kinds),
                join(&why)
            );
            kept_out.push(json!({"model": model, "policy": policy.name, "chunks": n, "kept": kept_n, "kinds": kinds, "why": why}));
        }
    }
    summary.insert("kept_as_they_were".into(), Value::Array(kept_out));

    // Per language, GPU, paraphrase moderate and strong: in the document too.
    let _ = writeln!(
        md,
        "\n### Selection policies per language — GPU 2 × 2, `paraphrase`\n"
    );
    let _ = writeln!(md, "| model | intensity | lang | policy | rewritten | divergence med | word change med | judged CHANGED |");
    let _ = writeln!(md, "|---|---|---|---|---|---|---|---|");
    for row in &pol {
        if row["executor"] == "gpu-2x2"
            && row["tactic"] == "paraphrase"
            && row["lang"] != "all"
            && row["intensity"] != "light"
        {
            let _ = writeln!(
                md,
                "| {} | {} | {} | {} | {} | {} | {} | {} |",
                s(row, "model"),
                s(row, "intensity"),
                s(row, "lang"),
                s(row, "policy"),
                fmt_ratio(&row["rewritten"]),
                row["divergence"]["median"]
                    .as_f64()
                    .map_or("–".into(), |v| format!("{v:.2}")),
                row["word_change"]["median"]
                    .as_f64()
                    .map_or("–".into(), |v| format!("{v:.2}")),
                if row["judged"] == 0 {
                    "–".to_owned()
                } else {
                    format!("{} of {}", fmt_ratio(&row["changed"]), row["judged"])
                },
            );
        }
    }
    // E4-8: the voice of what the loop ships, beside the least-diverged
    // pick on the same candidates — D111 re-read against it.
    let _ = writeln!(
        md,
        "\n### Voice — the loop's pick (E4-7) beside the least diverged; every language\n"
    );
    let _ = writeln!(md, "GPU 2 × 2 where the run made four candidates a chunk, CPU 1 × 2 where it made two. Second and first person are pronouns counted, kept by count; a chunk kept as it was keeps its voice. The register shift is a proxy: the share of the answer's words that a list of formal words and suffixes matches and the source has no word for (`docs/architecture/prompt-bench.md`, \"Voice\").\n");
    let _ = writeln!(md, "| model | tactic | intensity | executor | policy | chunks | with 2nd person | 2nd person kept, by count | per chunk, med [q1–q3] | lost any / lost all | ты↔вы, du↔Sie switched | 1st person kept | words ×, all · per chunk med [q1–q3] | register shift med [q1–q3] · mean | voice judged YES / PARTLY / NO |");
    let _ = writeln!(
        md,
        "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    let mut voice_rows: Vec<&Value> = Vec::new();
    for model in &models {
        for (tactic, intensity) in CELLS {
            for policy in [E4_7, VOICE_BESIDE] {
                let row = |executor: &str| {
                    pol.iter().find(|r| {
                        s(r, "model") == model
                            && s(r, "tactic") == tactic
                            && s(r, "intensity") == intensity
                            && s(r, "policy") == policy.name
                            && s(r, "executor") == executor
                            && r["lang"] == "all"
                    })
                };
                voice_rows.extend(row("gpu-2x2").or_else(|| row("cpu-1x2")));
            }
        }
    }
    for row in voice_rows {
        let v = &row["voice"];
        let judged = &v["judged"];
        let n_judged = judged["n"].as_u64().unwrap_or(0) as usize;
        let share = |field: &str| pct(judged[field].as_u64().unwrap_or(0) as usize, n_judged);
        let _ = writeln!(
            md,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} / {} | {} of {} | {} | {} · {} | {} · {} | {} |",
            s(row, "model"),
            s(row, "tactic"),
            s(row, "intensity"),
            if row["executor"] == "gpu-2x2" { "GPU 2 × 2" } else { "CPU 1 × 2" },
            s(row, "policy"),
            row["chunks"],
            v["with_second"],
            fmt_ratio(&v["second_kept_all"]),
            quart_md(&v["second_kept_per_chunk"]),
            fmt_ratio(&v["lost_any"]),
            fmt_ratio(&v["lost_all"]),
            v["switched"],
            v["address"],
            fmt_ratio(&v["first_kept_all"]),
            v["words_ratio_all"].as_f64().map_or("–".into(), |x| format!("{x:.2}")),
            quart_md(&v["words_ratio"]),
            quart_md(&v["register_shift"]),
            v["register_shift_mean"].as_f64().map_or("–".into(), |x| format!("{x:.3}")),
            if n_judged == 0 {
                "–".to_owned()
            } else {
                format!("{} / {} / {} of {n_judged}", share("yes"), share("partly"), share("no"))
            },
        );
    }
    summary.insert("policies".into(), Value::Array(pol));

    // The language check: the answer's language against the chunk's, when
    // the chunk's can be told. The loop makes it since E4-7 (answers of 20
    // words or more); over earlier records, what it would have refused.
    let _ = writeln!(md, "\n### The language check (candidates that passed, chunk language detected; the loop refuses 20+ words since E4-7)\n");
    let _ = writeln!(md, "| model | passed, chunk language known | answer in another language | answer language unknown | of those two, planted-instruction items |");
    let _ = writeln!(md, "|---|---|---|---|---|");
    let mut lc = Vec::new();
    for model in &models {
        let rs: Vec<&Value> = records
            .iter()
            .filter(|r| {
                s(r, "model") == model && s(r, "verdict") == "passed" && r["lang_chunk"].is_string()
            })
            .collect();
        let other = rs
            .iter()
            .filter(|r| r["lang_answer"].is_string() && r["lang_answer"] != r["lang_chunk"])
            .count();
        let unknown = rs.iter().filter(|r| r["lang_answer"].is_null()).count();
        let planted = rs
            .iter()
            .filter(|r| r["lang_answer"] != r["lang_chunk"] && s(r, "kind") == "injection")
            .count();
        let _ = writeln!(
            md,
            "| {model} | {} | {other} ({}) | {unknown} ({}) | {planted} |",
            rs.len(),
            pct(other, rs.len()),
            pct(unknown, rs.len())
        );
        lc.push(json!({"model": model, "passed_known": rs.len(), "other": other, "unknown": unknown, "planted": planted}));
    }
    summary.insert("language_check".into(), Value::Array(lc));

    // Thresholds: the no-op floor and the length window.
    let _ = writeln!(
        md,
        "\n### The no-op floor and the length window (every tactic)\n"
    );
    let _ = writeln!(md, "| model | candidates that passed all but the floor | divergence < 0.05 | < 0.10 | < 0.20 | length-guard rejections (0.6–1.6) | of them inside 0.5–2.0 | judged CHANGED: inside 0.6–1.6 | in 0.5–0.6 or 1.6–2.0 | outside 0.5–2.0 |");
    let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|---|");
    let mut thr = Vec::new();
    for model in &models {
        let rs: Vec<&Value> = records.iter().filter(|r| s(r, "model") == model).collect();
        let base: Vec<&&Value> = rs.iter().filter(|r| basic(r)).collect();
        let under = |x: f64| {
            base.iter()
                .filter(|r| f(r, "divergence").is_some_and(|d| d < x))
                .count()
        };
        let judgeable: Vec<&&Value> = rs.iter().filter(|r| judge::judgeable(r)).collect();
        let length_rejected: Vec<&&&Value> = judgeable
            .iter()
            .filter(|r| !r["guards"]["length-drift"].is_null())
            .collect();
        let in_wide = length_rejected
            .iter()
            .filter(|r| f(r, "length_ratio").is_some_and(|x| (0.5..=2.0).contains(&x)))
            .count();
        let band = |lo: f64, hi: f64, inside: bool| {
            let sel: Vec<&&&Value> = judgeable
                .iter()
                .filter(|r| {
                    f(r, "length_ratio").is_some_and(|x| {
                        let within = (lo..=hi).contains(&x);
                        if inside {
                            within
                        } else {
                            !within
                        }
                    })
                })
                .collect();
            let judged: Vec<&String> = sel
                .iter()
                .filter_map(|r| verdicts.get(s(r, "key")))
                .collect();
            let changed = judged.iter().filter(|v| v.as_str() == "CHANGED").count();
            (changed, judged.len())
        };
        let inside = band(0.6, 1.6, true);
        let all_wide = band(0.5, 2.0, true);
        let between = (all_wide.0 - inside.0, all_wide.1 - inside.1);
        let outside = band(0.5, 2.0, false);
        let _ = writeln!(
            md,
            "| {model} | {} | {} | {} | {} | {} | {} | {} of {} | {} of {} | {} of {} |",
            base.len(),
            pct(under(0.05), base.len()),
            pct(under(0.10), base.len()),
            pct(under(0.20), base.len()),
            length_rejected.len(),
            in_wide,
            pct(inside.0, inside.1),
            inside.1,
            pct(between.0, between.1),
            between.1,
            pct(outside.0, outside.1),
            outside.1,
        );
        thr.push(json!({"model": model, "basic": base.len(), "under_005": under(0.05), "under_010": under(0.10), "under_020": under(0.20),
            "length_rejected": length_rejected.len(), "length_rejected_inside_wide": in_wide,
            "changed_inside": [inside.0, inside.1], "changed_between": [between.0, between.1], "changed_outside": [outside.0, outside.1]}));
    }
    summary.insert("thresholds".into(), Value::Array(thr));

    // Meaning drift against divergence, every judged candidate.
    let _ = writeln!(md, "\n### Judged CHANGED by divergence band (every judged candidate inside the length window)\n");
    let _ = writeln!(
        md,
        "| model | < 0.2 | 0.2–0.4 | 0.4–0.6 | 0.6–0.8 | ≥ 0.8 |"
    );
    let _ = writeln!(md, "|---|---|---|---|---|---|");
    let mut bands = Vec::new();
    for model in &models {
        let mut line = format!("| {model} |");
        for (lo, hi) in [(0.0, 0.2), (0.2, 0.4), (0.4, 0.6), (0.6, 0.8), (0.8, 1.01)] {
            let judged: Vec<&String> = records
                .iter()
                .filter(|r| {
                    s(r, "model") == model
                        && judge::judgeable(r)
                        && r["guards"]["length-drift"].is_null()
                        && f(r, "divergence").is_some_and(|d| d >= lo && d < hi)
                })
                .filter_map(|r| verdicts.get(s(r, "key")))
                .collect();
            let changed = judged.iter().filter(|v| v.as_str() == "CHANGED").count();
            let _ = write!(
                line,
                " {} of {} |",
                pct(changed, judged.len()),
                judged.len()
            );
            bands.push(json!({"model": model, "from": lo, "to": hi, "judged": judged.len(), "changed": changed}));
        }
        let _ = writeln!(md, "{line}");
    }
    summary.insert("drift_by_divergence".into(), Value::Array(bands));

    // Judge calibration.
    let _ = writeln!(md, "\n### The judge's calibration\n");
    let _ = writeln!(md, "| judge | case | expected | n | as expected |");
    let _ = writeln!(md, "|---|---|---|---|---|");
    let mut cal = Vec::new();
    let mut judges: Vec<&str> = judgements
        .iter()
        .filter_map(|j| j["judge"].as_str())
        .collect();
    judges.sort_unstable();
    judges.dedup();
    for judge_name in judges {
        for (case, expected) in [
            ("same", "EQUIVALENT"),
            ("dropped", "CHANGED"),
            ("added", "CHANGED"),
        ] {
            let js: Vec<&Value> = judgements
                .iter()
                .filter(|j| j["judge"] == judge_name && j["calib"] == case)
                .collect();
            let ok = js.iter().filter(|j| j["verdict"] == expected).count();
            let _ = writeln!(
                md,
                "| {judge_name} | {case} | {expected} | {} | {} |",
                js.len(),
                pct(ok, js.len())
            );
            cal.push(json!({"judge": judge_name, "case": case, "n": js.len(), "as_expected": ok}));
        }
        // E4-8: the voice question's calibration — a text keeps its own
        // voice, and a text that speaks to its reader otherwise does not
        // (2026-10-09). Judgements made before have none.
        for (case, expected) in [("same", "YES"), ("switched", "NO")] {
            let js: Vec<&Value> = judgements
                .iter()
                .filter(|j| j["judge"] == judge_name && j["voice_calib"] == case)
                .collect();
            if !js.is_empty() {
                let ok = js.iter().filter(|j| j["voice"] == expected).count();
                let _ = writeln!(
                    md,
                    "| {judge_name} | {case} (voice) | {expected} | {} | {} |",
                    js.len(),
                    pct(ok, js.len())
                );
                cal.push(json!({"judge": judge_name, "case": format!("{case}-voice"), "n": js.len(), "as_expected": ok}));
            }
        }
        let other = judgements
            .iter()
            .filter(|j| {
                j["judge"] == judge_name
                    && [j["verdict"].as_str(), j["voice"].as_str()]
                        .into_iter()
                        .flatten()
                        .any(|v| v.starts_with("other"))
            })
            .count();
        let _ = writeln!(
            md,
            "| {judge_name} | unparsable answers | – | {other} | – |"
        );
    }
    summary.insert("judge_calibration".into(), Value::Array(cal));

    if let Some(n) = examples {
        write_examples(&mut md, &groups, &models, &verdicts, n);
    }
    (md, summary)
}

/// The summary as `summary.json` holds it: one row per line — small, and a
/// rerun diffs row by row.
fn summary_text(summary: &Map<String, Value>) -> String {
    let mut text = String::from("{\n");
    let sections: Vec<_> = summary.iter().collect();
    for (i, (key, value)) in sections.iter().enumerate() {
        let _ = writeln!(text, "  {}: [", json!(key));
        let rows = value.as_array().cloned().unwrap_or_default();
        for (j, row) in rows.iter().enumerate() {
            let comma = if j + 1 < rows.len() { "," } else { "" };
            let _ = writeln!(text, "    {row}{comma}");
        }
        let comma = if i + 1 < sections.len() { "," } else { "" };
        let _ = writeln!(text, "  ]{comma}");
    }
    text.push_str("}\n");
    serde_json::from_str::<Value>(&text).expect("the summary is JSON");
    text
}

fn fmt_ratio(v: &Value) -> String {
    v.as_f64()
        .map_or("–".into(), |x| format!("{:.0}%", x * 100.0))
}

/// Before/after, per language and model: the source, and the winners of
/// E4-3's policy, of `min ≥ 0.6` and of E4-7's — GPU, `paraphrase`.
fn write_examples(
    md: &mut String,
    groups: &BTreeMap<(usize, String, String, String, String, u64), Group>,
    models: &[String],
    verdicts: &HashMap<String, String>,
    per_lang: usize,
) {
    let _ = writeln!(md, "\n## Examples\n");
    for (m, model) in models.iter().enumerate() {
        for lang in ["en", "ru", "de"] {
            let mut shown = 0;
            for ((gm, tactic, intensity, l, item, chunk), group) in groups {
                if *gm != m
                    || tactic != "paraphrase"
                    || intensity != "moderate"
                    || l != lang
                    || shown >= per_lang
                {
                    continue;
                }
                if !(item.contains("-mx-") || item.contains("-pd-")) {
                    continue;
                }
                let picks: Vec<(&str, Option<&Value>)> = [E4_3, POLICIES[6], E4_7]
                    .iter()
                    .map(|p| (p.name, simulate(group, *p, true).and_then(|x| x.winner)))
                    .collect();
                // Only chunks where the policies disagree are instructive.
                let keys: Vec<&str> = picks
                    .iter()
                    .map(|(_, w)| w.map_or("", |w| s(w, "key")))
                    .collect();
                if keys[0] == keys[1] && keys[1] == keys[2] {
                    continue;
                }
                shown += 1;
                let source = group.values().next().map_or("", |r| s(r, "chunk_text"));
                let _ = writeln!(
                    md,
                    "### {model} · {lang} · {item} chunk {chunk}\n\n**Source**\n\n> {}\n",
                    source.trim().replace('\n', "\n> ")
                );
                for (name, winner) in picks {
                    let _ = match winner {
                        Some(w) => writeln!(
                            md,
                            "**{name}** — divergence {:.2}, word change {:.2}, judge {}\n\n> {}\n",
                            f(w, "divergence").unwrap_or(0.0),
                            f(w, "word_change").unwrap_or(0.0),
                            verdicts.get(s(w, "key")).map_or("–", String::as_str),
                            s(w, "answer").trim().replace('\n', "\n> ")
                        ),
                        None => writeln!(
                            md,
                            "**{name}** — nothing qualified: the paragraph is kept as it was\n"
                        ),
                    };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One attempt of `paraphrase` moderate as `run` writes it — without
    /// the `voice` field, as every record made before E4-8 is.
    fn record(item: &str, k: u64, source: &str, answer: Option<&str>, divergence: f64) -> Value {
        json!({
            "key": format!("m|{item}|0|paraphrase|moderate|{k}"),
            "model": "m",
            "item": item,
            "lang": item.split('-').next(),
            "kind": "machine",
            "chunk": 0,
            "tactic": "paraphrase",
            "intensity": "moderate",
            "k": k,
            "chunk_text": source,
            "answer": answer,
            "verdict": if answer.is_some() { "passed" } else { "guard" },
            "divergence": answer.map(|_| divergence),
            "word_change": 0.5,
            "length_ratio": 1.0,
            "steps": [{"step": 1, "tokens_out": 10, "secs": 1.0}],
            "secs": 1.0,
            "guards": {},
            "restore": null,
            "placeholders": 0,
            "placeholders_kept": 0,
            "injection": null,
        })
    }

    const SOURCE: &str = "You can fix it, and your team can help you.";

    /// k2 is the most diverged of round 1 and keeps every "you"; k1, the
    /// least, keeps none.
    fn english() -> Vec<Value> {
        vec![
            record(
                "en-mx-01",
                1,
                SOURCE,
                Some("One can fix it, and the team can help."),
                0.5,
            ),
            record(
                "en-mx-01",
                2,
                SOURCE,
                Some("You can fix it; your team helps you too."),
                0.7,
            ),
            record(
                "en-mx-01",
                3,
                SOURCE,
                Some("You fix it, your team helps you."),
                0.6,
            ),
            record(
                "en-mx-01",
                4,
                SOURCE,
                Some("You fix it, your team helps you."),
                0.6,
            ),
        ]
    }

    /// The voice table's row for `policy`.
    fn voice_row(md: &str, policy: &str) -> String {
        let table = md
            .split("\n### ")
            .find(|section| section.starts_with("Voice"))
            .expect("the voice table");
        let start = format!("| m | paraphrase | moderate | GPU 2 × 2 | {policy} |");
        table
            .lines()
            .find(|line| line.starts_with(&start))
            .unwrap_or_else(|| panic!("no row for {policy} in\n{table}"))
            .to_owned()
    }

    #[test]
    fn records_made_before_the_voice_measures_still_report_them() {
        let (md, summary) = tables(english(), &[], None);
        assert_eq!(
            voice_row(&md, "max ≥ 0.2 (E4-7)"),
            "| m | paraphrase | moderate | GPU 2 × 2 | max ≥ 0.2 (E4-7) | 1 | 1 | 100% | 1.00 [1.00–1.00] \
             | 0% / 0% | 0 of 0 | – | 0.90 · 0.90 [0.90–0.90] | 0.00 [0.00–0.00] · 0.000 | – |"
        );
        assert_eq!(
            voice_row(&md, "min ≥ 0.2"),
            "| m | paraphrase | moderate | GPU 2 × 2 | min ≥ 0.2 | 1 | 1 | 0% | 0.00 [0.00–0.00] \
             | 100% / 100% | 0 of 0 | – | 0.90 · 0.90 [0.90–0.90] | 0.00 [0.00–0.00] · 0.000 | – |"
        );
        let e4_7 = summary["policies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["policy"] == E4_7.name && r["executor"] == "gpu-2x2" && r["lang"] == "all")
            .expect("the loop's row");
        assert_eq!(e4_7["voice"]["second"], json!([3, 3]));
        assert_eq!(e4_7["voice"]["judged"]["n"], 0);
        let attempts = &summary["attempts"][0]["voice"];
        assert_eq!(attempts["n"], 4, "every answered attempt, measured");
        assert_eq!(attempts["lost_all"], json!(0.25));
    }

    #[test]
    fn the_judges_voice_answer_is_read_beside_the_meaning() {
        let judgements = vec![
            json!({"key": "judge|J|m|en-mx-01|0|paraphrase|moderate|2", "of": "m|en-mx-01|0|paraphrase|moderate|2", "judge": "J", "verdict": "EQUIVALENT"}),
            json!({"key": "voice|J|m|en-mx-01|0|paraphrase|moderate|2", "of": "m|en-mx-01|0|paraphrase|moderate|2", "judge": "J", "voice": "YES"}),
            json!({"key": "voice|J|m|en-mx-01|0|paraphrase|moderate|1", "of": "m|en-mx-01|0|paraphrase|moderate|1", "judge": "J", "voice": "NO"}),
            json!({"key": "calib-voice|J|en-mx-01#0|same", "voice_calib": "same", "item": "en-mx-01#0", "judge": "J", "voice": "YES"}),
            json!({"key": "calib-voice|J|switched-ru|switched", "voice_calib": "switched", "item": "switched-ru", "judge": "J", "voice": "NO"}),
            json!({"key": "calib-voice|J|switched-de|switched", "voice_calib": "switched", "item": "switched-de", "judge": "J", "voice": "YES"}),
        ];
        let (md, _) = tables(english(), &judgements, None);
        assert!(voice_row(&md, "max ≥ 0.2 (E4-7)").ends_with("| 100% / 0% / 0% of 1 |"));
        assert!(voice_row(&md, "min ≥ 0.2").ends_with("| 0% / 0% / 100% of 1 |"));
        assert!(md.contains("| J | same (voice) | YES | 1 | 100% |"));
        assert!(
            md.contains("| J | switched (voice) | NO | 2 | 50% |"),
            "a judge that says YES to a switched address fails"
        );
        // The meaning's own columns read the meaning lines only.
        assert!(md.contains("| J | same | EQUIVALENT | 0 | – |"));
    }

    #[test]
    fn a_chunk_kept_as_it_was_keeps_its_voice() {
        let refused: Vec<Value> = (1..=4)
            .map(|k| record("en-mx-01", k, SOURCE, None, 0.0))
            .collect();
        let (md, _) = tables(refused, &[], None);
        assert_eq!(
            voice_row(&md, "max ≥ 0.2 (E4-7)"),
            "| m | paraphrase | moderate | GPU 2 × 2 | max ≥ 0.2 (E4-7) | 1 | 1 | 100% | 1.00 [1.00–1.00] \
             | 0% / 0% | 0 of 0 | – | 1.00 · 1.00 [1.00–1.00] | 0.00 [0.00–0.00] · 0.000 | – |"
        );
    }

    #[test]
    fn a_cell_with_two_candidates_shows_the_cpu_pick() {
        // `humanize` at two candidates a chunk (the voice run's grid): no
        // GPU 2 × 2 to simulate, so the voice table shows CPU 1 × 2 —
        // k1 alone, k2 only when k1 did not qualify.
        let records: Vec<Value> = english()
            .into_iter()
            .take(2)
            .map(|mut r| {
                r["tactic"] = json!("humanize");
                r["key"] = json!(s(&r, "key").replace("paraphrase", "humanize"));
                r
            })
            .collect();
        let (md, _) = tables(records, &[], None);
        let table = md
            .split("\n### ")
            .find(|section| section.starts_with("Voice"))
            .unwrap();
        let rows: Vec<&str> = table
            .lines()
            .filter(|line| line.starts_with("| m |"))
            .collect();
        assert_eq!(rows.len(), 2, "{table}");
        assert!(rows[0].starts_with(
            "| m | humanize | moderate | CPU 1 × 2 | max ≥ 0.2 (E4-7) | 1 | 1 | 0% |"
        ));
    }

    #[test]
    fn a_rewrite_that_moves_from_du_to_sie_is_counted_as_switched() {
        let source = "Hast du Fragen? Schreib uns.";
        let records = vec![
            record(
                "de-mx-01",
                1,
                source,
                Some("Hast du noch Fragen? Schreib uns."),
                0.3,
            ),
            record(
                "de-mx-01",
                2,
                source,
                Some("Haben Sie Fragen? Schreiben Sie uns."),
                0.8,
            ),
            record("de-mx-01", 3, source, Some("Fragen? Schreib uns."), 0.5),
            record("de-mx-01", 4, source, Some("Fragen? Schreib uns."), 0.5),
        ];
        let (md, _) = tables(records, &[], None);
        let cells = |row: String| row.split(" | ").nth(10).map(str::to_owned);
        assert_eq!(
            cells(voice_row(&md, "max ≥ 0.2 (E4-7)")).as_deref(),
            Some("1 of 1")
        );
        assert_eq!(
            cells(voice_row(&md, "min ≥ 0.2")).as_deref(),
            Some("0 of 1")
        );
    }
}
