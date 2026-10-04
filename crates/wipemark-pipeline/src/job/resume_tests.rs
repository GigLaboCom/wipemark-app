//! The job that can be taken up again (E4-4), on `FakeEngine`: what it
//! hands out, what it takes back, and what it refuses to take back.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use wipemark_engine::fake::FakeEngine;
use wipemark_engine::{ChatRequest, RewriteEngine};

use super::resume::{fingerprint, fingerprint_under};
use super::{plan, start, start_resumable, Decided, Document, Options, Outcome};
use crate::cost::{Effort, Executor};
use crate::prepare::TextFormat;
use crate::report::ChunkOutcome;
use crate::select::{Rules, RULES};
use crate::{Event, JobId};

const EN: &str = "The build takes about 12 minutes on an ordinary laptop, and the second run is much faster because all of the dependencies are already compiled and kept in the target directory.";
const EN_2: &str = "When the linker fails, check that the package is installed and that the variable called library_search_path points at the folder where the shared objects live.";
const EN_3: &str = "Most of the time a clean checkout is all it takes, and the rest of the steps are written down in the guide that ships with the source of the project itself.";
const EN_4: &str = "A release is cut every second Tuesday, and the notes for it are collected from the pull requests that were merged since the one before, by hand and with some care.";

const BEGIN: &str = "[[[BEGIN TEXT]]]\n";
const END: &str = "\n[[[END TEXT]]]";

fn text_of(req: &ChatRequest) -> String {
    let start = req.prompt.find(BEGIN).expect("a text block") + BEGIN.len();
    let end = req.prompt[start..].find(END).expect("its end") + start;
    req.prompt[start..end].to_owned()
}

/// A rewrite the guards accept and the no-op floor does not call a copy:
/// every two neighbouring words of the first half swapped.
fn swap(text: &str) -> String {
    let mut words: Vec<&str> = text.split(' ').collect();
    let half = words.len() / 2;
    for pair in words[..half].chunks_mut(2) {
        pair.reverse();
    }
    words.join(" ")
}

/// Every chunk passes on its first answer, except the second chunk, whose
/// first answer from this engine changes nothing — so one chunk takes two
/// rounds and has a rejection on record.
fn engine() -> FakeEngine {
    let seen = Arc::new(Mutex::new(false));
    FakeEngine::answering(move |req, _| {
        let text = text_of(req);
        if text.starts_with("When the linker") {
            let mut once = seen.lock().expect("not poisoned");
            if !*once {
                *once = true;
                return text;
            }
        }
        swap(&text)
    })
}

fn source() -> String {
    format!("{EN}\n\n{EN_2}\n\n{EN_3}\n\n{EN_4}\n")
}

fn document(text: &str) -> Document {
    Document {
        text: text.to_owned(),
        format: TextFormat::Plain,
    }
}

fn options() -> Options {
    Options {
        effort: Effort {
            candidates: 1,
            rounds: 2,
        },
        ..Options::for_executor(Executor::LocalCpu)
    }
}

fn events_of(receiver: &flume::Receiver<Event>) -> Vec<Event> {
    let mut events = Vec::new();
    loop {
        let event = receiver
            .recv_timeout(Duration::from_secs(30))
            .expect("the job ends");
        let end = matches!(
            event,
            Event::Finished { .. } | Event::Cancelled { .. } | Event::Failed { .. }
        );
        events.push(event);
        if end {
            return events;
        }
    }
}

fn resumable(
    engine: &FakeEngine,
    doc: Document,
    options: Options,
    carried: Vec<Decided>,
) -> Vec<Event> {
    let (_handle, receiver) =
        start_resumable(JobId(3), doc, options, Arc::new(engine.clone()), carried)
            .expect("the job starts");
    events_of(&receiver)
}

fn outcome(events: &[Event]) -> &Outcome {
    match events.last() {
        Some(Event::Finished { outcome, .. }) => outcome,
        other => panic!("the job did not finish: {other:?}"),
    }
}

fn records_of(events: &[Event]) -> Vec<Decided> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::ChunkDecided { decided, .. } => Some((**decided).clone()),
            _ => None,
        })
        .collect()
}

fn resumed(events: &[Event]) -> Option<(u32, u32)> {
    events.iter().find_map(|event| match event {
        Event::Resumed {
            carried, discarded, ..
        } => Some((*carried, *discarded)),
        _ => None,
    })
}

/// The whole job once, uninterrupted: its events and its records.
fn whole() -> (Vec<Event>, Vec<Decided>) {
    let events = resumable(&engine(), document(&source()), options(), Vec::new());
    let records = records_of(&events);
    (events, records)
}

#[test]
fn start_emits_no_chunk_record() {
    let (_handle, receiver) =
        start(JobId(1), document(&source()), options(), Arc::new(engine())).expect("starts");
    let events = events_of(&receiver);
    assert!(matches!(events.last(), Some(Event::Finished { .. })));
    assert!(records_of(&events).is_empty());
    assert_eq!(resumed(&events), None);
    let json = outcome(&events).report.to_json();
    assert!(!json.contains("carried_over"), "{json}");
}

#[test]
fn every_chunk_is_recorded_once_in_order_and_nothing_is_resumed_from_nothing() {
    let (events, records) = whole();
    assert_eq!(
        records.iter().map(|r| r.index).collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    assert_eq!(resumed(&events), None, "nothing was handed in");
    // Chunk 2 needed two rounds: its record carries both attempts.
    assert_eq!(records[1].counts.attempts, 2);
    assert_eq!(records[1].counts.rejected, 1);
    assert!(matches!(
        records[1].outcome,
        ChunkOutcome::Rewritten { round: 2, .. }
    ));
}

#[test]
fn a_resumed_job_asks_only_the_undecided_chunks_and_ends_with_the_same_text() {
    let (events, records) = whole();
    let uninterrupted = outcome(&events).clone();

    let fresh = engine();
    let carried = records[..2].to_vec();
    let events = resumable(&fresh, document(&source()), options(), carried);
    let resumed_outcome = outcome(&events);

    assert_eq!(resumed(&events), Some((2, 0)));
    assert_eq!(resumed_outcome.text, uninterrupted.text);
    let asked: Vec<String> = fresh.asked().iter().map(text_of).collect();
    assert_eq!(asked.len(), 2, "chunks 3 and 4, once each: {asked:?}");
    assert!(asked[0].starts_with("Most of the time"));
    assert!(asked[1].starts_with("A release is cut"));
    assert_eq!(
        records_of(&events)
            .iter()
            .map(|r| r.index)
            .collect::<Vec<_>>(),
        vec![2, 3],
        "a carried chunk is not recorded again"
    );
    assert_eq!(
        resumed_outcome.report.totals(),
        uninterrupted.report.totals(),
        "the carried attempts are counted"
    );
}

#[test]
fn a_job_with_every_chunk_carried_asks_nothing() {
    let (events, records) = whole();
    let fresh = engine();
    let again = resumable(&fresh, document(&source()), options(), records);
    assert_eq!(resumed(&again), Some((4, 0)));
    assert!(fresh.asked().is_empty());
    assert_eq!(outcome(&again).text, outcome(&events).text);
}

#[test]
fn a_carried_chunk_is_reported_as_carried_with_its_attempts_as_recorded() {
    let (events, records) = whole();
    let uninterrupted = outcome(&events).report.to_value();
    let again = resumable(
        &engine(),
        document(&source()),
        options(),
        records[..2].to_vec(),
    );
    let report = &outcome(&again).report;

    assert!(report.chunks[0].carried.is_some());
    assert!(report.chunks[0].attempts.is_empty());
    assert!(report.chunks[2].carried.is_none());

    let value = report.to_value();
    let chunks = &value["best_effort"]["chunks"];
    for index in 0..2 {
        assert_eq!(chunks[index]["carried_over"], true);
        assert_eq!(
            chunks[index]["attempts"], uninterrupted["best_effort"]["chunks"][index]["attempts"],
            "chunk {index}'s attempts as they were recorded"
        );
        assert_eq!(
            chunks[index]["outcome"],
            uninterrupted["best_effort"]["chunks"][index]["outcome"]
        );
    }
    assert!(chunks[2].get("carried_over").is_none());
    assert_eq!(
        value["best_effort"]["totals"],
        uninterrupted["best_effort"]["totals"]
    );
    assert!(report.to_json().is_ascii());
}

#[test]
fn a_changed_document_discards_every_record() {
    let (_, records) = whole();
    let changed = source().replace("by hand and with some care", "by a script");
    let fresh = engine();
    let events = resumable(&fresh, document(&changed), options(), records[..2].to_vec());
    assert_eq!(resumed(&events), Some((0, 2)));
    assert_eq!(fresh.asked().len(), 5, "every chunk again; chunk 2 twice");
    assert!(outcome(&events)
        .report
        .chunks
        .iter()
        .all(|chunk| chunk.carried.is_none()));
}

#[test]
fn changed_options_discard_every_record() {
    let (_, records) = whole();
    let mut other = options();
    other.base_seed = 99;
    let fresh = engine();
    let events = resumable(&fresh, document(&source()), other, records[..2].to_vec());
    assert_eq!(resumed(&events), Some((0, 2)));
    assert_eq!(fresh.asked().len(), 5);
}

#[test]
fn the_fingerprint_moves_with_the_rules() {
    let document = document(&source());
    let options = options();
    let info = engine().info();
    let planned = plan(&document, &options, &info).expect("planned");
    let today = fingerprint(&document, &options, &info, &planned);
    assert_eq!(
        today,
        fingerprint_under(&RULES, &document, &options, &info, &planned)
    );
    for moved in [
        Rules {
            no_op_floor: 0.05,
            ..RULES
        },
        Rules {
            short_chunk_words: RULES.short_chunk_words + 1,
            ..RULES
        },
        Rules {
            language_check_words: 0,
            ..RULES
        },
    ] {
        assert_ne!(
            fingerprint_under(&moved, &document, &options, &info, &planned),
            today,
            "a record decided under {moved:?} must not be resumed under {RULES:?}"
        );
    }
}

#[test]
fn another_engine_discards_every_record() {
    let (_, records) = whole();
    let scripted = engine();
    let (_handle, receiver) = start_resumable(
        JobId(4),
        document(&source()),
        options(),
        Arc::new(Renamed(scripted.clone())),
        records[..2].to_vec(),
    )
    .expect("starts");
    let events = events_of(&receiver);
    assert_eq!(resumed(&events), Some((0, 2)));
    assert_eq!(scripted.asked().len(), 5);
}

/// The scripted engine under another model id.
struct Renamed(FakeEngine);

#[wipemark_engine::async_trait]
impl wipemark_engine::RewriteEngine for Renamed {
    fn info(&self) -> wipemark_engine::EngineInfo {
        wipemark_engine::EngineInfo {
            model_id: "another-model".to_owned(),
            ..self.0.info()
        }
    }

    async fn complete(
        &self,
        req: ChatRequest,
        sink: wipemark_engine::TokenSink,
        cancel: wipemark_engine::CancellationToken,
    ) -> Result<wipemark_engine::Completion, wipemark_engine::EngineError> {
        self.0.complete(req, sink, cancel).await
    }

    async fn warmup(&self) -> Result<(), wipemark_engine::EngineError> {
        Ok(())
    }

    async fn unload(&self) {}
}

#[test]
fn a_record_for_another_chunk_or_with_a_broken_winner_is_discarded() {
    let (_, records) = whole();

    let mut moved = records[0].clone();
    moved.index = 2; // chunk 0's record filed under chunk 2
    let mut no_winner = records[1].clone();
    no_winner.winner = None; // "rewritten", with nothing to put back
    let mut tampered = records[3].clone();
    tampered.digest = "0".repeat(64);

    let fresh = engine();
    let events = resumable(
        &fresh,
        document(&source()),
        options(),
        vec![moved, no_winner, tampered],
    );
    assert_eq!(resumed(&events), Some((0, 3)));
    assert_eq!(fresh.asked().len(), 5, "every chunk asked again");
}

#[test]
fn a_winner_that_no_longer_restores_is_discarded() {
    // A chunk with a protected span: its winner must keep the placeholder.
    let source = "Run `cargo build --release` in the root of the repository, and wait for the first build to finish before you open the editor again.\n";
    let engine = FakeEngine::answering(|req, _| swap(&text_of(req)));
    let events = resumable(&engine, document(source), options(), Vec::new());
    let mut record = records_of(&events).remove(0);
    let winner = record.winner.clone().expect("rewritten");
    assert!(winner.contains('⟦'), "{winner}");
    record.winner = Some(winner.replace("⟦1⟧", "it"));

    let fresh = FakeEngine::answering(|req, _| swap(&text_of(req)));
    let again = resumable(&fresh, document(source), options(), vec![record]);
    assert_eq!(resumed(&again), Some((0, 1)));
    assert_eq!(fresh.asked().len(), 1);
    assert_eq!(outcome(&again).text, outcome(&events).text);
}

#[test]
fn a_record_round_trips() {
    let (_, mut records) = whole();
    records[0].winner = Some("Сборка ⟦1⟧ идёт\u{200B}".to_owned());
    for record in records {
        let stored = record.to_json();
        assert!(stored.is_ascii(), "{stored}");
        assert!(!stored.contains('\n'));
        assert_eq!(Decided::from_json(&stored), Ok(record));
    }
    assert!(Decided::from_json("{}").is_err());
    assert!(Decided::from_json(r#"{"record":2}"#).is_err());
}
