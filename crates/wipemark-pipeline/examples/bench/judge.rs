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
//!
//! Since E4-8 it asks a second, separate question of the same attempts —
//! does the rewrite keep the source's **voice** (who it speaks to and as,
//! its tone and register)? `YES`, `PARTLY` or `NO` (D423). The meaning
//! question is not touched: its prompt, its keys and its lines are what
//! they were, so its calibration and every earlier judgement stand. The
//! voice answer is a line of its own beside the meaning's (`voice|…`,
//! `"of"` the same attempt), so a file judged before E4-8 gets its voice
//! lines by being judged again; its calibration is the chunk against
//! itself (must be `YES`) and, since the verification of 2026-10-09, three
//! fixed texts against a version that speaks to the reader otherwise —
//! ты → вы, du → Sie, "you" → nobody in a formal register (must be `NO`),
//! so a judge that says `YES` to everything fails it.

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

const VOICE_SYSTEM: &str =
    "You compare the voice of two texts. You answer with exactly one word: YES, PARTLY or NO.";

/// The voice question's negative calibration: a text, and the same text
/// addressing its reader the other way (`[lang, text, switched]`). Written
/// for the bench; `NO` is the answer.
pub const VOICE_SWITCHED: [(&str, &str, &str); 3] = [
    (
        "en",
        "You'll love how easy this is. Grab your biggest bowl, throw everything in and give it a good stir.",
        "The procedure is notably straightforward. The largest available bowl should be utilised to combine the ingredients thoroughly.",
    ),
    (
        "ru",
        "Если ты ни разу не пёк хлеб, не переживай: это проще, чем кажется. Смешай муку, воду и соль в твоей самой большой миске.",
        "Если вы ни разу не пекли хлеб, не переживайте: это проще, чем кажется. Смешайте муку, воду и соль в вашей самой большой миске.",
    ),
    (
        "de",
        "Wenn du zum ersten Mal Brot backst, mach dir keine Sorgen. Rühr Mehl, Wasser und Salz in deiner größten Schüssel zusammen.",
        "Wenn Sie zum ersten Mal Brot backen, machen Sie sich keine Sorgen. Rühren Sie Mehl, Wasser und Salz in Ihrer größten Schüssel zusammen.",
    ),
];

fn voice_prompt(source: &str, answer: &str) -> String {
    format!(
        "Text A is an original. Text B is a rewrite of it. Marks like \u{27E6}1\u{27E7} stand for protected content such as code or links.\n\n\
         A:\n<<<\n{}\n>>>\n\nB:\n<<<\n{}\n>>>\n\n\
         Does B keep the voice of A: does it speak to the reader the way A does (the same person, such as \"you\", \"I\" or \"we\", and the same informal or formal address), \
         in the same tone and register, with words no more formal or elaborate than A's? \
         Differences in wording, word order and sentence boundaries do not matter, and neither do the facts. Answer YES, PARTLY or NO.",
        source.trim(),
        answer.trim()
    )
}

/// The two questions the judge asks of an attempt, each a request and a
/// line of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Question {
    /// Same facts? `EQUIVALENT` or `CHANGED` — the bench's question since
    /// E4-5, unchanged.
    Meaning,
    /// Same voice? `YES`, `PARTLY` or `NO` (E4-8).
    Voice,
}

impl Question {
    fn answers(self) -> &'static [&'static str] {
        match self {
            Question::Meaning => &["EQUIVALENT", "CHANGED"],
            Question::Voice => &["YES", "PARTLY", "NO"],
        }
    }

    /// The field of the judgement's line the answer goes in.
    pub fn field(self) -> &'static str {
        match self {
            Question::Meaning => "verdict",
            Question::Voice => "voice",
        }
    }
}

/// The answer's first word if it is one `question` takes, else
/// `other:<what came back>`.
pub fn parse(question: Question, reply: &str) -> String {
    let word: String = reply
        .trim()
        .trim_start_matches(|c: char| !c.is_alphabetic())
        .chars()
        .take_while(|c| c.is_alphabetic())
        .collect::<String>()
        .to_uppercase();
    if question.answers().contains(&word.as_str()) {
        word
    } else {
        format!("other:{}", reply.trim())
    }
}

/// One question about `answer` against `source`, at temperature 0: its
/// answer as [`parse`] reads it.
pub fn ask(
    engine: &dyn RewriteEngine,
    question: Question,
    source: &str,
    answer: &str,
) -> (String, f64) {
    let (system, prompt) = match question {
        Question::Meaning => (SYSTEM, prompt(source, answer)),
        Question::Voice => (VOICE_SYSTEM, voice_prompt(source, answer)),
    };
    let request = ChatRequest {
        system: Some(system.to_owned()),
        prompt,
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
    (parse(question, &reply), started.elapsed().as_secs_f64())
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

/// One judgement to make: its key, the two texts, the line's fields, and
/// the question.
type Work = Vec<(String, String, String, Value, Question)>;

/// Every judgement `--in` asks for that `--out` does not hold yet, in the
/// order they are made: the calibration, then each attempt's meaning and
/// voice.
fn work(args: &Args, judge: &str, done: &HashSet<String>) -> Work {
    let records = read(&args.list("--in"));
    let mut work: Work = Vec::new();
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
                work.push((
                    key,
                    text.clone(),
                    b,
                    json!({"calib": case, "item": id}),
                    Question::Meaning,
                ));
            }
        }
        // The voice question's calibration: a text keeps its own voice.
        let key = format!("calib-voice|{judge}|{id}|same");
        if !done.contains(&key) {
            work.push((
                key,
                text.clone(),
                text.clone(),
                json!({"voice_calib": "same", "item": id}),
                Question::Voice,
            ));
        }
    }
    // The voice question's negative calibration: a text against itself
    // speaking to its reader otherwise must be `NO`.
    if !chunks.is_empty() {
        for (lang, text, switched) in VOICE_SWITCHED {
            let id = format!("switched-{lang}");
            let key = format!("calib-voice|{judge}|{id}|switched");
            if !done.contains(&key) {
                work.push((
                    key,
                    text.to_owned(),
                    switched.to_owned(),
                    json!({"voice_calib": "switched", "item": id}),
                    Question::Voice,
                ));
            }
        }
    }
    for r in records.iter().filter(|r| judgeable(r)) {
        let of = r["key"].as_str().unwrap_or("");
        for (prefix, question) in [("judge", Question::Meaning), ("voice", Question::Voice)] {
            let key = format!("{prefix}|{judge}|{of}");
            if !done.contains(&key) {
                work.push((
                    key,
                    r["chunk_text"].as_str().unwrap_or("").to_owned(),
                    r["answer"].as_str().unwrap_or("").to_owned(),
                    json!({"of": r["key"]}),
                    question,
                ));
            }
        }
    }
    work
}

/// `plan --of judge`: what [`main`] would ask with the same flags, one
/// line, and no model loaded — every judgement is one call.
pub fn plan_only(args: &Args) {
    let judge = args.required("--name");
    let done = run::done(&PathBuf::from(args.required("--out")));
    let work = work(args, &judge, &done);
    let attempts = work.iter().filter(|w| w.3["of"].is_string()).count() / 2;
    println!(
        "attempts={attempts} calls={} done={}",
        work.len(),
        done.len()
    );
}

pub fn main(args: &Args) {
    let out = PathBuf::from(args.required("--out"));
    let (judge, engine) = engine::from_args(args);
    let done = run::done(&out);
    let work = work(args, &judge, &done);
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
    for (n, (key, a, b, mut meta, question)) in work.into_iter().enumerate() {
        let (answer, secs) = ask(engine.as_ref(), question, &a, &b);
        meta["key"] = json!(key);
        meta["judge"] = json!(judge);
        meta[question.field()] = json!(answer);
        meta["secs"] = json!(secs);
        writeln!(file, "{meta}").expect("written");
        if n % 50 == 0 || answer.starts_with("other") {
            eprintln!("[{}/{total}] {key}: {answer} ({secs:.2}s)", n + 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_question_takes_only_its_own_answers() {
        assert_eq!(parse(Question::Meaning, " Equivalent."), "EQUIVALENT");
        assert_eq!(
            parse(Question::Voice, "**Partly** — the register"),
            "PARTLY"
        );
        assert_eq!(parse(Question::Voice, "no"), "NO");
        assert_eq!(parse(Question::Voice, "CHANGED"), "other:CHANGED");
        assert_eq!(parse(Question::Meaning, "YES"), "other:YES");
    }

    #[test]
    fn every_judged_attempt_is_asked_both_questions_and_an_old_file_only_the_new_one() {
        let path = std::env::temp_dir().join(format!(
            "wipemark-judge-work-{}-{:?}.jsonl",
            std::process::id(),
            std::thread::current().id()
        ));
        let record = |k: u64| {
            json!({
                "key": format!("m|en-mx-01|0|paraphrase|moderate|{k}"),
                "item": "en-mx-01", "chunk": 0, "tactic": "paraphrase",
                "intensity": "moderate", "k": k, "placeholders": 0,
                "chunk_text": "You can fix it. Your team can help.",
                "answer": "One can fix it. The team can help.",
                "verdict": "passed", "guards": {}, "restore": null,
            })
        };
        std::fs::write(&path, format!("{}\n{}\n", record(1), record(2))).unwrap();
        let args = Args::of("judge", &[("--in", path.to_str().unwrap())]);

        let fresh = work(&args, "J", &HashSet::new());
        let questions = |w: &Work, q: Question| w.iter().filter(|x| x.4 == q).count();
        assert_eq!(
            questions(&fresh, Question::Meaning),
            3 + 2,
            "calibration and each attempt"
        );
        assert_eq!(
            questions(&fresh, Question::Voice),
            1 + VOICE_SWITCHED.len() + 2,
            "its calibration — the chunk against itself and the fixed switched texts — and each attempt"
        );
        let switched: Vec<&(String, String, String, Value, Question)> = fresh
            .iter()
            .filter(|w| w.3["voice_calib"] == "switched")
            .collect();
        assert_eq!(switched.len(), VOICE_SWITCHED.len());
        assert!(switched
            .iter()
            .all(|w| w.4 == Question::Voice && w.1 != w.2 && w.0.ends_with("|switched")));
        assert!(fresh
            .iter()
            .any(|w| w.0 == "voice|J|m|en-mx-01|0|paraphrase|moderate|1"
                && w.3["of"] == "m|en-mx-01|0|paraphrase|moderate|1"));

        // A file judged before E4-8: every meaning line there, no voice line.
        let before: HashSet<String> = fresh
            .iter()
            .filter(|w| w.4 == Question::Meaning)
            .map(|w| w.0.clone())
            .collect();
        let again = work(&args, "J", &before);
        assert_eq!(
            questions(&again, Question::Meaning),
            0,
            "the meaning is never asked twice"
        );
        assert_eq!(
            questions(&again, Question::Voice),
            1 + VOICE_SWITCHED.len() + 2
        );
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn the_meaning_question_is_the_one_e4_5_asked() {
        // A judgement made before E4-8 and one made after are the same
        // question: the voice question is asked beside it, never in it.
        assert_eq!(
            SYSTEM,
            "You compare two texts for meaning. You answer with exactly one word: EQUIVALENT or CHANGED."
        );
        let asked = prompt("A one.", "B one.");
        assert!(asked.ends_with(
            "Does B state the same facts, claims, numbers and names as A, with nothing added, \
             nothing left out and nothing changed in meaning? Differences in wording, word order, \
             style and sentence boundaries do not matter. Answer EQUIVALENT or CHANGED."
        ));
        assert!(!asked.contains("voice"));
        assert!(voice_prompt("A one.", "B one.").contains("informal or formal address"));
    }
}
