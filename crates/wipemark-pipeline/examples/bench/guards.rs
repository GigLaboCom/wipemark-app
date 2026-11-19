//! `guards`: the guards of 2026-11-19 (D856–D860) replayed over a run's
//! records, with no model — every candidate that passed the loop's verdict
//! when it was recorded, judged again by each new guard on its own.
//!
//! Per guard and per model it reports how many of those candidates the
//! guard would now refuse — its cost: candidates the loop throws away, and
//! more rounds — how many of them the judge had called `CHANGED` and
//! `EQUIVALENT`, and what share of the judge's `CHANGED` it catches; the
//! chunks of a job (a cell of the grid: every candidate of one chunk,
//! tactic and intensity) that had a candidate pass before and would have
//! none; a sweep of each guard's threshold; and examples. The chunks are
//! prepared again from the corpus, the way the run prepared them, so the
//! guards that read what a chunk was prepared with — its constructs
//! (`pairs`), its context (`context-echo`) — get what the run had; a record
//! whose chunk no longer matches the corpus is counted and skipped.
//!
//! ```sh
//! cargo run -p wipemark-pipeline --features local-llama --example bench -- guards \
//!     --in runs/a.jsonl,runs/b.jsonl --judge judge.jsonl [--examples 3]
//! ```

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::Path;

use serde_json::Value;
use wipemark_core::{ContextEchoGuard, Guard, GuardOutcome, OrderGuard, PermutationGuard};
use wipemark_pipeline::prepare::{prepare, Budget, Chunk, Prepared};

use crate::args::Args;
use crate::{corpus, judge, run};

/// The guards this replays, in the order the loop runs them.
pub const NEW: [&str; 5] = [
    "paragraphs",
    "permutation",
    "order",
    "pairs",
    "context-echo",
];

/// What one guard did to the candidates of one model.
#[derive(Default, Clone)]
struct Tally {
    refused: usize,
    changed: usize,
    equivalent: usize,
}

impl Tally {
    fn add(&mut self, verdict: Option<&str>) {
        self.refused += 1;
        match verdict {
            Some("CHANGED") => self.changed += 1,
            Some("EQUIVALENT") => self.equivalent += 1,
            _ => {}
        }
    }
}

/// One candidate that passed when it was recorded, ready to be judged again.
struct Passed<'a> {
    model: String,
    cell: String,
    chunk: &'a Chunk,
    answer: String,
    verdict: Option<String>,
}

pub fn main(args: &Args) {
    let corpus_dir = args
        .value("--corpus")
        .unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/bench/corpus").to_owned());
    let records = judge::records(&args.list("--in"));
    let judgements = judge::records(&args.list("--judge"));
    let examples: usize = args
        .value("--examples")
        .map_or(3, |n| n.parse().unwrap_or(3));
    let echo_at: Option<usize> = args.value("--echo-at").and_then(|n| n.parse().ok());
    print!(
        "{}",
        replay(
            Path::new(&corpus_dir),
            &records,
            &judgements,
            examples,
            echo_at
        )
    );
}

/// The whole replay as Markdown; with `echo_at`, every candidate the
/// context guard would refuse at that many words is listed too.
pub fn replay(
    corpus_dir: &Path,
    records: &[Value],
    judgements: &[Value],
    examples: usize,
    echo_at: Option<usize>,
) -> String {
    let items: HashMap<String, corpus::Item> = corpus::load(corpus_dir)
        .into_iter()
        .map(|item| (item.id.clone(), item))
        .collect();
    let prepared: HashMap<&str, Prepared> = items
        .iter()
        .map(|(id, item)| {
            (
                id.as_str(),
                prepare(&item.text, item.format, Budget::DEFAULT),
            )
        })
        .collect();
    let meaning: HashMap<&str, &str> = judgements
        .iter()
        .filter_map(|j| Some((j["of"].as_str()?, j["verdict"].as_str()?)))
        .collect();

    let mut skipped = 0usize;
    let mut passed: Vec<Passed<'_>> = Vec::new();
    for r in records {
        if r["verdict"] != "passed" {
            continue;
        }
        let (Some(key), Some(item), Some(index), Some(text), Some(answer)) = (
            r["key"].as_str(),
            r["item"].as_str(),
            r["chunk"].as_u64(),
            r["chunk_text"].as_str(),
            r["answer"].as_str(),
        ) else {
            skipped += 1;
            continue;
        };
        let Some(chunk) = prepared
            .get(item)
            .and_then(|p| p.chunks().get(index as usize))
            .filter(|chunk| chunk.text == text)
        else {
            skipped += 1;
            continue;
        };
        // The cell: the key without its candidate.
        let cell = key
            .rsplit_once('|')
            .map_or(key, |(cell, _)| cell)
            .to_owned();
        passed.push(Passed {
            model: r["model"].as_str().unwrap_or("?").to_owned(),
            cell,
            chunk,
            answer: answer.to_owned(),
            verdict: meaning.get(key).map(|v| (*v).to_owned()),
        });
    }

    let options = run::options(
        wipemark_pipeline::prompt::Tactic::Paraphrase,
        wipemark_pipeline::prompt::Intensity::Moderate,
    );
    let models: Vec<String> = {
        let mut m: Vec<String> = passed.iter().map(|p| p.model.clone()).collect();
        m.sort();
        m.dedup();
        m
    };
    let mut base: BTreeMap<&str, Tally> = BTreeMap::new();
    let mut tallies: BTreeMap<(&str, &str), Tally> = BTreeMap::new();
    let mut any: BTreeMap<&str, Tally> = BTreeMap::new();
    let mut shown: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    // Per cell: (passed before, passed after); and passed after with one
    // guard alone added, by guard.
    let mut cells: BTreeMap<(&str, &str), (bool, bool)> = BTreeMap::new();
    let mut cells_alone: BTreeMap<(&str, &str, &str), bool> = BTreeMap::new();
    let mut old_refused = 0usize;
    let mut with_context = 0usize;
    let mut with_pairs = 0usize;
    for p in &passed {
        let verdict = p.verdict.as_deref();
        base.entry(&p.model).or_default().add(verdict);
        with_context += usize::from(p.chunk.context.is_some());
        with_pairs += usize::from(!p.chunk.pairs.is_empty());
        let mut refused_by_any = false;
        let mut refused_by: Vec<&str> = Vec::new();
        for guard in options.guards_for(p.chunk) {
            let outcome = guard.check(&p.chunk.text, &p.answer);
            let GuardOutcome::Reject(reason) = outcome else {
                continue;
            };
            let Some(name) = NEW.iter().find(|n| **n == guard.name()) else {
                // An old guard refusing what passed: the record is not this
                // build's verdict (the length window moved, …).
                old_refused += 1;
                continue;
            };
            refused_by_any = true;
            refused_by.push(name);
            tallies.entry((name, &p.model)).or_default().add(verdict);
            let list = shown.entry(name).or_default();
            if list.len() < examples {
                list.push(format!(
                    "- `{}` judge {} — {reason}\n  - chunk: {}\n  - answer: {}",
                    p.model,
                    verdict.unwrap_or("–"),
                    clip(&p.chunk.text),
                    clip(&p.answer)
                ));
            }
        }
        if refused_by_any {
            any.entry(&p.model).or_default().add(verdict);
        }
        let cell = cells.entry((&p.model, &p.cell)).or_insert((false, false));
        cell.0 = true;
        cell.1 |= !refused_by_any;
        for name in NEW {
            *cells_alone
                .entry((name, &p.model, &p.cell))
                .or_insert(false) |= !refused_by.contains(&name);
        }
    }

    let mut md = String::new();
    let _ = writeln!(
        md,
        "## The guards of 2026-11-19 over the bench's passed candidates\n"
    );
    let _ = writeln!(
        md,
        "{} records; {} passed and replayed ({} with a context, {} with a construct); {} skipped (no answer, or a chunk the corpus no longer prepares the same); {} refused by an old guard on replay.\n",
        records.len(),
        passed.len(),
        with_context,
        with_pairs,
        skipped,
        old_refused
    );
    let _ = writeln!(md, "Each cell: refused / of them CHANGED / of them EQUIVALENT — and the share of the judge's CHANGED among the passed candidates it catches.\n");
    let mut header = String::from("| guard |");
    let mut rule = String::from("|---|");
    for m in &models {
        let _ = write!(header, " {m} |");
        rule.push_str("---|");
    }
    header.push_str(" all |");
    rule.push_str("---|");
    let _ = writeln!(md, "{header}\n{rule}");
    let cell_text = |t: &Tally, of: &Tally| {
        format!(
            "{} / {} / {} ({:.1} % of CHANGED)",
            t.refused,
            t.changed,
            t.equivalent,
            percent(t.changed, of.changed)
        )
    };
    let sum = |rows: &mut dyn Iterator<Item = Tally>| {
        rows.fold(Tally::default(), |mut a, t| {
            a.refused += t.refused;
            a.changed += t.changed;
            a.equivalent += t.equivalent;
            a
        })
    };
    let base_all = sum(&mut base.values().cloned());
    let mut row = String::from("| passed before |");
    for m in &models {
        let t = base.get(m.as_str()).cloned().unwrap_or_default();
        let _ = write!(
            row,
            " {} ({} CHANGED, {} EQUIVALENT) |",
            t.refused, t.changed, t.equivalent
        );
    }
    let _ = writeln!(
        md,
        "{row} {} ({} CHANGED, {} EQUIVALENT) |",
        base_all.refused, base_all.changed, base_all.equivalent
    );
    for name in NEW {
        let mut row = format!("| `{name}` |");
        for m in &models {
            let t = tallies
                .get(&(name, m.as_str()))
                .cloned()
                .unwrap_or_default();
            let b = base.get(m.as_str()).cloned().unwrap_or_default();
            let _ = write!(row, " {} |", cell_text(&t, &b));
        }
        let all = sum(&mut models.iter().map(|m| {
            tallies
                .get(&(name, m.as_str()))
                .cloned()
                .unwrap_or_default()
        }));
        let _ = writeln!(md, "{row} {} |", cell_text(&all, &base_all));
    }
    let mut row = String::from("| any of them |");
    for m in &models {
        let t = any.get(m.as_str()).cloned().unwrap_or_default();
        let b = base.get(m.as_str()).cloned().unwrap_or_default();
        let _ = write!(row, " {} |", cell_text(&t, &b));
    }
    let any_all = sum(&mut any.values().cloned());
    let _ = writeln!(md, "{row} {} |", cell_text(&any_all, &base_all));
    let mut row = String::from("| cells left with no passed candidate |");
    let mut lost_all = (0, 0);
    for m in &models {
        let of_model: Vec<&(bool, bool)> = cells
            .iter()
            .filter(|((model, _), _)| *model == m.as_str())
            .map(|(_, v)| v)
            .collect();
        let lost = of_model
            .iter()
            .filter(|(before, after)| *before && !*after)
            .count();
        lost_all.0 += lost;
        lost_all.1 += of_model.len();
        let _ = write!(row, " {lost} of {} |", of_model.len());
    }
    let _ = writeln!(md, "{row} {} of {} |\n", lost_all.0, lost_all.1);
    let _ = writeln!(md, "A cell is one chunk under one tactic and intensity: its candidates are the attempts of one job (2 × 2 for paraphrase, the first round for humanize). Left with none, the chunk keeps its source. With one guard alone added:\n");
    let _ = writeln!(
        md,
        "| guard | cells left with no passed candidate |\n|---|---|"
    );
    for name in NEW {
        let lost = cells_alone
            .iter()
            .filter(|((guard, _, _), still)| *guard == name && !**still)
            .count();
        let _ = writeln!(md, "| `{name}` | {lost} |");
    }
    let _ = writeln!(md);

    sweeps(&mut md, &passed);

    if let Some(n) = echo_at {
        let _ = writeln!(
            md,
            "### `context-echo` at {n} words: every candidate refused\n"
        );
        for p in &passed {
            let Some(context) = &p.chunk.context else {
                continue;
            };
            let guard = ContextEchoGuard {
                context: context.clone(),
                min_words: n,
            };
            if let GuardOutcome::Reject(reason) = guard.check(&p.chunk.text, &p.answer) {
                let _ = writeln!(
                    md,
                    "- `{}` judge {} — {reason}\n  - context: {}\n  - answer: {}",
                    p.model,
                    p.verdict.as_deref().unwrap_or("–"),
                    clip(context),
                    clip(&p.answer)
                );
            }
        }
        let _ = writeln!(md);
    }

    for name in NEW {
        if let Some(list) = shown.get(name) {
            let _ = writeln!(md, "### `{name}`: the first {} refused\n", list.len());
            for line in list {
                let _ = writeln!(md, "{line}");
            }
            let _ = writeln!(md);
        }
    }
    md
}

/// Each threshold moved: refused / CHANGED among them, over every model.
fn sweeps(md: &mut String, passed: &[Passed<'_>]) {
    let count = |refuses: &dyn Fn(&Passed<'_>) -> bool| {
        let mut t = Tally::default();
        for p in passed.iter().filter(|p| refuses(p)) {
            t.add(p.verdict.as_deref());
        }
        t
    };
    let _ = writeln!(md, "### Thresholds\n");
    let _ = writeln!(
        md,
        "| guard | threshold | refused | CHANGED | EQUIVALENT |\n|---|---|---|---|---|"
    );
    for n in 3..=12 {
        let t = count(&|p| {
            p.chunk.context.as_ref().is_some_and(|context| {
                let guard = ContextEchoGuard {
                    context: context.clone(),
                    min_words: n,
                };
                !guard.check(&p.chunk.text, &p.answer).is_pass()
            })
        });
        let _ = writeln!(
            md,
            "| `context-echo` | {n} words | {} | {} | {} |",
            t.refused, t.changed, t.equivalent
        );
    }
    for words in [4, 6, 8, 10, 12] {
        let guard = OrderGuard {
            max_item_words: words,
            ..OrderGuard::DEFAULT
        };
        let t = count(&|p| !guard.check(&p.chunk.text, &p.answer).is_pass());
        let _ = writeln!(
            md,
            "| `order` | items of ≤ {words} words | {} | {} | {} |",
            t.refused, t.changed, t.equivalent
        );
    }
    for words in [3, 5, 8, 20] {
        let guard = PermutationGuard { min_words: words };
        let t = count(&|p| !guard.check(&p.chunk.text, &p.answer).is_pass());
        let _ = writeln!(
            md,
            "| `permutation` | ≥ {words} words | {} | {} | {} |",
            t.refused, t.changed, t.equivalent
        );
    }
    let _ = writeln!(md);
}

fn percent(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        100.0 * part as f64 / whole as f64
    }
}

/// `text` on one line, at most 240 characters.
fn clip(text: &str) -> String {
    let one_line = text.replace('\n', " ⏎ ");
    match one_line.char_indices().nth(240) {
        Some((at, _)) => format!("{}…", &one_line[..at]),
        None => one_line,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::replay;

    /// A corpus of one English Markdown item of two paragraphs, and nothing
    /// in the other languages, where the test can read it.
    fn corpus() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "wipemark-guards-corpus-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("en.txt"),
            "=== id: en-md-99 | kind: markdown | format: markdown | source: the test | licence: the test\n\
             Every capture lands in the library with its keyword and its date.\n\
             \n\
             Report any problem on [the issue tracker](https://example.com/issues).\n",
        )
        .unwrap();
        std::fs::write(dir.join("ru.txt"), "").unwrap();
        std::fs::write(dir.join("de.txt"), "").unwrap();
        dir
    }

    fn record(chunk: u64, chunk_text: &str, answer: &str, k: u64) -> Value {
        json!({
            "key": format!("m|en-md-99|{chunk}|paraphrase|moderate|{k}"),
            "model": "m", "item": "en-md-99", "chunk": chunk,
            "chunk_text": chunk_text, "answer": answer, "verdict": "passed",
        })
    }

    /// Each new guard is counted on its own, against what the judge said; a
    /// chunk whose every candidate one of them refuses is counted as left
    /// without one; a record the corpus no longer prepares is skipped.
    #[test]
    fn a_refused_candidate_is_counted_under_its_guard_and_its_judgement() {
        let link = "Report any problem on \u{27E6}1\u{27E7}the issue tracker\u{27E6}2\u{27E7}.";
        let records = [
            // The link's brackets turned around: pairs.
            record(
                1,
                link,
                "Submit any difficulty to \u{27E6}2\u{27E7}the issue tracker on \u{27E6}1\u{27E7}.",
                1,
            ),
            // The context's words as they were: context-echo.
            record(
                1,
                link,
                "Every capture lands in the library with its keyword, so report problems on \u{27E6}1\u{27E7}the tracker\u{27E6}2\u{27E7}.",
                2,
            ),
            // A rewrite every guard passes.
            record(
                0,
                "Every capture lands in the library with its keyword and its date.",
                "Each capture is filed in the library under its keyword and date.",
                1,
            ),
            // A chunk the corpus does not prepare so: skipped.
            record(0, "Something else.", "Anything.", 2),
        ];
        let judgements = [
            json!({"of": "m|en-md-99|1|paraphrase|moderate|1", "verdict": "CHANGED"}),
            json!({"of": "m|en-md-99|1|paraphrase|moderate|2", "verdict": "EQUIVALENT"}),
        ];
        let md = replay(&corpus(), &records, &judgements, 1, None);
        assert!(
            md.contains("3 passed and replayed (2 with a context, 2 with a construct); 1 skipped"),
            "{md}"
        );
        assert!(
            md.contains("| `pairs` | 1 / 1 / 0 (100.0 % of CHANGED) |"),
            "{md}"
        );
        assert!(
            md.contains("| `context-echo` | 1 / 0 / 1 (0.0 % of CHANGED) |"),
            "{md}"
        );
        assert!(
            md.contains("| `permutation` | 0 / 0 / 0 (0.0 % of CHANGED) |"),
            "{md}"
        );
        // The link's chunk lost both its candidates; the first kept its one.
        assert!(
            md.contains("| cells left with no passed candidate | 1 of 2 | 1 of 2 |"),
            "{md}"
        );
    }
}
