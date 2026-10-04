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

use crate::args::Args;
use crate::judge;
use crate::measure::{kept_bigrams, percentile};

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
    })
}

/// One chunk's candidates of one cell, by `k`.
type Group<'a> = BTreeMap<u64, &'a Value>;

struct Pick<'a> {
    winner: Option<&'a Value>,
    /// The chunk's text, as the model saw it.
    source: &'a str,
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
                calls,
                secs,
            });
        }
    }
    Some(Pick {
        winner: None,
        source,
        calls,
        secs,
    })
}

fn policy_row(
    groups: &[&Group],
    policy: Policy,
    gpu: bool,
    verdicts: &HashMap<String, String>,
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
    for r in &mut records {
        // `lang` on a record is the detection over the answer; the
        // document's language is the item's (its id's prefix).
        r["lang_doc"] = json!(s(r, "item").split('-').next().unwrap_or(""));
        r["lang_chunk"] = json!(crate::measure::detect(s(r, "chunk_text")).map(|l| l.as_str()));
        // Recomputed from the texts, so a better detector needs no rerun.
        if let Some(answer) = r["answer"].as_str().map(str::to_owned) {
            let source = s(r, "chunk_text").to_owned();
            r["preface"] = json!(crate::measure::preface(&source, &answer));
            r["trailer"] = json!(crate::measure::trailer(&source, &answer));
        }
    }
    let judgements = judge::records(&args.list("--judge"));
    let verdicts: HashMap<String, String> = judgements
        .iter()
        .filter_map(|j| {
            Some((
                j["of"].as_str()?.to_owned(),
                j["verdict"].as_str()?.to_owned(),
            ))
        })
        .collect();

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
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
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
            for (tactic, intensity) in [
                ("paraphrase", "light"),
                ("paraphrase", "moderate"),
                ("paraphrase", "strong"),
                ("humanize", "moderate"),
                ("humanize", "strong"),
                ("back_translate", "moderate"),
            ] {
                let these: Vec<&Group> = groups
                    .iter()
                    .filter(|((gm, t, i, _, _, _), _)| *gm == m && t == tactic && i == intensity)
                    .map(|(_, g)| g)
                    .collect();
                for policy in POLICIES {
                    let mut row = policy_row(&these, policy, gpu, &verdicts);
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
                    let weighed = [E4_3.name, E4_7.name, "min ≥ 0.6"];
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
                        let mut row = policy_row(&per, policy, gpu, &verdicts);
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
        let other = judgements
            .iter()
            .filter(|j| {
                j["judge"] == judge_name
                    && j["verdict"]
                        .as_str()
                        .is_some_and(|v| v.starts_with("other"))
            })
            .count();
        let _ = writeln!(
            md,
            "| {judge_name} | unparsable answers | – | {other} | – |"
        );
    }
    summary.insert("judge_calibration".into(), Value::Array(cal));

    if let Some(path) = args.value("--summary") {
        // One row per line: small, and a rerun diffs row by row.
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
        std::fs::write(&path, text).unwrap_or_else(|e| panic!("{path}: {e}"));
        eprintln!("wrote {path}");
    }
    print!("{md}");

    if let Some(n) = args.value("--examples") {
        examples(
            &records,
            &groups,
            &models,
            &verdicts,
            n.parse().unwrap_or(3),
        );
    }
}

fn fmt_ratio(v: &Value) -> String {
    v.as_f64()
        .map_or("–".into(), |x| format!("{:.0}%", x * 100.0))
}

/// Before/after, per language and model: the source, and the winners of
/// E4-3's policy, of `min ≥ 0.6` and of E4-7's — GPU, `paraphrase`.
fn examples(
    records: &[Value],
    groups: &BTreeMap<(usize, String, String, String, String, u64), Group>,
    models: &[String],
    verdicts: &HashMap<String, String>,
    per_lang: usize,
) {
    let _ = records;
    println!("\n## Examples\n");
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
                println!(
                    "### {model} · {lang} · {item} chunk {chunk}\n\n**Source**\n\n> {}\n",
                    source.trim().replace('\n', "\n> ")
                );
                for (name, winner) in picks {
                    match winner {
                        Some(w) => println!(
                            "**{name}** — divergence {:.2}, word change {:.2}, judge {}\n\n> {}\n",
                            f(w, "divergence").unwrap_or(0.0),
                            f(w, "word_change").unwrap_or(0.0),
                            verdicts.get(s(w, "key")).map_or("–", String::as_str),
                            s(w, "answer").trim().replace('\n', "\n> ")
                        ),
                        None => println!(
                            "**{name}** — nothing qualified: the paragraph is kept as it was\n"
                        ),
                    }
                }
            }
        }
    }
}
