//! The table rewriting through the batch queue, and the table as the
//! journal (E4-6b, R2–R5, R7, D325) — over a real batch queue on
//! `FakeEngine`, whose own thread runs while the test waits for it.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{TestAppContext, VisualTestContext};
use wipemark_engine::fake::FakeEngine;
use wipemark_i18n::{t, Message};
use wipemark_intake::Handed;
use wipemark_pipeline::cost::Executor;
use wipemark_pipeline::{Event, JobId, Stage};
use wipemark_queue::{Destination, Durability, ItemId, QueueEvent as BatchEvent, Source};
use wipemark_store::entry::{Delivered, Entry, Origin, Phase};
use wipemark_store::Store;

use super::tests::{queue_with, statuses, Scratch};
use super::{Queue, QueueEvent, Status};
use crate::engine_host::{EngineHandle, Pace};
use crate::journal::{self, Journal, Work};
use crate::retention::Destination as Goes;

/// A paragraph `lang::detect` reads as English, long enough to be checked.
const PARAGRAPH: &str = "The build takes about twelve minutes on an ordinary laptop, and the \
                         second run is much faster because all of the dependencies are already \
                         compiled and kept in the target directory.\n";

/// The text a request asks to rewrite, every two neighbouring words of its
/// first half swapped: an answer every guard accepts.
fn swapped(req: &wipemark_engine::ChatRequest) -> String {
    const BEGIN: &str = "[[[BEGIN TEXT]]]\n";
    const END: &str = "\n[[[END TEXT]]]";
    let start = req.prompt.find(BEGIN).map_or(0, |at| at + BEGIN.len());
    let stop = req.prompt[start..]
        .find(END)
        .map_or(req.prompt.len(), |at| at + start);
    let mut words: Vec<&str> = req.prompt[start..stop].split(' ').collect();
    let half = words.len() / 2;
    for pair in words[..half].chunks_mut(2) {
        pair.reverse();
    }
    words.join(" ")
}

fn swapping() -> FakeEngine {
    FakeEngine::answering(|req, _| swapped(req))
}

/// A batch queue and a journal over `store`, the engine on duty `engine`,
/// and the bookkeeper the application starts.
fn work_over(store: Arc<Store>, engine: Option<FakeEngine>) -> Work {
    let handle = match engine {
        Some(engine) => {
            EngineHandle::serving(
                Arc::new(engine),
                Pace {
                    executor: Some(Executor::LocalCpu),
                    tokens_per_second: None,
                },
            )
            .0
        }
        None => EngineHandle::new().0,
    };
    let queue = wipemark_queue::Queue::with_source(
        Arc::clone(&store),
        Durability::Memory { detail: None },
        Arc::new(handle.clone()),
    )
    .expect("the queue opens");
    let work = Work {
        queue: Arc::new(queue),
        journal: Journal::new(store),
        engine: handle,
        whereto: journal::Going::default(),
    };
    journal::keep_books(&work);
    work
}

fn work(engine: FakeEngine) -> Work {
    work_over(Arc::new(Store::in_memory().expect("memory")), Some(engine))
}

/// Run the window's tasks until `done` says so — the batch queue is a
/// thread of its own, so this waits for it in real time, at most a minute.
fn until(cx: &mut VisualTestContext, what: &str, done: impl Fn(&mut VisualTestContext) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        cx.executor().advance_clock(super::rewriting::TICK);
        cx.run_until_parked();
        if done(cx) {
            return;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn ids(queue: &gpui::Entity<Queue>, cx: &mut VisualTestContext) -> Vec<u64> {
    cx.update(|_, cx| queue.read(cx).ids())
}

fn status(queue: &gpui::Entity<Queue>, cx: &mut VisualTestContext) -> &'static str {
    statuses(queue, cx)[0]
}

/// R2, R3: a row's Rewrite pushes one item — the row's file, read as
/// Markdown, beside it as `name.rewritten.ext`, a new file — and the
/// rewrite lands there, while the clean's `name.cleaned.ext` already beside
/// it is left byte for byte. The row says it was rewritten, and its journal
/// row too.
#[gpui::test]
fn a_rows_rewrite_is_one_item_with_its_source_format_and_destination(cx: &mut TestAppContext) {
    let scratch = Scratch::new("rewrite-one");
    let source = scratch.file("article.md", PARAGRAPH.as_bytes());
    let cleaned = scratch.file("article.cleaned.md", b"a clean's result");
    let work = work(swapping());
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work.clone()));
    queue.update(cx, |queue, cx| queue.hand(vec![source.clone()], cx));
    cx.run_until_parked();
    assert_eq!(status(&queue, cx), "waiting");

    let id = ids(&queue, cx)[0];
    queue.update(cx, |queue, cx| queue.rewrite(&[id], cx));
    until(cx, "the push", |_| !work.queue.items().is_empty());
    let items = work.queue.items();
    assert_eq!(items.len(), 1, "one row, one item");
    assert_eq!(items[0].source, wipemark_queue::Shown::File(source.clone()));
    assert_eq!(
        items[0].destination,
        Some(Destination::New(scratch.0.join("article.rewritten.md")))
    );
    let stored = work.journal.store().queue().rows().expect("rows");
    assert!(
        stored[0].item.contains(r#""format":"markdown""#),
        "{}",
        stored[0].item
    );

    until(cx, "the end", |cx| status(&queue, cx) == "rewritten");
    let written = std::fs::read_to_string(scratch.0.join("article.rewritten.md"))
        .expect("the rewrite beside the file");
    assert_ne!(written, PARAGRAPH);
    assert_eq!(std::fs::read(&cleaned).expect("read"), b"a clean's result");
    assert_eq!(std::fs::read_to_string(&source).expect("read"), PARAGRAPH);

    until(cx, "the journal", |_| {
        work.journal
            .rows()
            .first()
            .is_some_and(|row| row.state == "done" && row.action == "rewrite")
    });
    let entry = Entry::from_json(&work.journal.rows()[0].entry);
    assert!(
        matches!(&entry.result, Some(Delivered::File { path, .. }) if path.ends_with("article.rewritten.md")),
        "{entry:?}"
    );
    assert!(
        !work.journal.rows()[0].entry.contains("twelve"),
        "the row kept the text"
    );
}

/// R2, D61: Rewrite all says its price first and pushes nothing until it
/// is answered; it takes the waiting texts and skips a picture and a row
/// already in the line.
#[gpui::test]
fn rewrite_all_prices_first_and_skips_pictures_and_queued_rows(cx: &mut TestAppContext) {
    let scratch = Scratch::new("rewrite-all");
    let first = scratch.file("a.md", PARAGRAPH.as_bytes());
    let second = scratch.file("b.md", PARAGRAPH.as_bytes());
    let picture = scratch.file(
        "p.png",
        &std::fs::read(format!(
            "{}/../../fixtures/image/xmp-provenance-url.png",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("a fixture"),
    );
    let work = work(swapping().with_token_delay(Duration::from_millis(20)));
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work.clone()));
    type Asked = Vec<(Vec<u64>, super::Price)>;
    let asked: Arc<std::sync::Mutex<Asked>> = Arc::default();
    let heard = Arc::clone(&asked);
    cx.update(|_, cx| {
        cx.subscribe(&queue, move |_, event: &QueueEvent, _| {
            if let QueueEvent::Price { ids, price } = event {
                heard
                    .lock()
                    .expect("lock")
                    .push((ids.clone(), price.clone()));
            }
        })
        .detach();
    });
    queue.update(cx, |queue, cx| queue.hand(vec![first, second, picture], cx));
    cx.run_until_parked();
    let all = ids(&queue, cx);
    // The first row is already in the line.
    queue.update(cx, |queue, cx| queue.rewrite(&all[..1], cx));
    until(cx, "the first push", |_| work.queue.items().len() == 1);

    queue.update(cx, |queue, cx| queue.ask_rewrite_all(cx));
    until(cx, "the price", |_| !asked.lock().expect("lock").is_empty());
    let (priced, price) = asked.lock().expect("lock")[0].clone();
    assert_eq!(priced, vec![all[1]], "not the queued row, not the picture");
    assert_eq!(price.documents, 1);
    assert!(price.calls_expected >= 1, "{price:?}");
    assert_eq!(
        price.seconds_expected, None,
        "a rate never measured is not a guess"
    );
    assert!(price
        .lines()
        .iter()
        .any(|line| line == &t(Message::RewritePriceTimeUnknown)));
    // Unanswered — Cancel — nothing more is pushed.
    cx.run_until_parked();
    assert_eq!(work.queue.items().len(), 1, "the price pushed something");
}

/// R2: the job's stage events move the row's Status — which paragraph of
/// how many — and the status bar says it.
#[gpui::test]
fn chunk_events_move_the_status(cx: &mut TestAppContext) {
    let scratch = Scratch::new("chunks");
    let source = scratch.file("a.md", PARAGRAPH.as_bytes());
    let work = work(swapping());
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work.clone()));
    queue.update(cx, |queue, cx| queue.hand(vec![source], cx));
    cx.run_until_parked();
    let id = ids(&queue, cx)[0];
    let item = ItemId(4242);
    queue.update(cx, |queue, cx| {
        if let Some(row) = queue.rows.iter_mut().find(|row| row.id == id) {
            row.item = Some(item);
            row.status = Status::RewriteQueued;
        }
        queue.heard_batch(BatchEvent::Started { item }, cx);
        queue.heard_batch(
            BatchEvent::Job {
                item,
                event: Event::Stage {
                    job: JobId(1),
                    stage: Stage::Rewriting {
                        chunk: 7,
                        chunks: 52,
                        candidate: 1,
                        candidates: 1,
                        round: 1,
                        rounds: 2,
                    },
                },
            },
            cx,
        );
    });
    let now = cx.update(|_, cx| match queue.read(cx).status_of(id) {
        Some(Status::Rewriting(chunk)) => *chunk,
        _ => None,
    });
    assert_eq!(now, Some((7, 52)));
}

/// R2: a paragraph no candidate passed keeps its cleaned source, and the
/// row ends as partly rewritten — the CLI's exit 3.
#[gpui::test]
fn a_kept_chunk_ends_as_partly(cx: &mut TestAppContext) {
    let scratch = Scratch::new("partly");
    let source = scratch.file("a.md", PARAGRAPH.as_bytes());
    // An engine that answers with the paragraph as it was: the no-op floor
    // refuses every candidate.
    let echo = FakeEngine::answering(|req, _| {
        const BEGIN: &str = "[[[BEGIN TEXT]]]\n";
        const END: &str = "\n[[[END TEXT]]]";
        let start = req.prompt.find(BEGIN).map_or(0, |at| at + BEGIN.len());
        let stop = req.prompt[start..]
            .find(END)
            .map_or(req.prompt.len(), |at| at + start);
        req.prompt[start..stop].to_owned()
    });
    let work = work(echo);
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work));
    queue.update(cx, |queue, cx| queue.hand(vec![source], cx));
    cx.run_until_parked();
    let id = ids(&queue, cx)[0];
    queue.update(cx, |queue, cx| queue.rewrite(&[id], cx));
    until(cx, "the end", |cx| status(&queue, cx) == "partly-rewritten");
}

/// R2: Cancel ends a queued rewrite as cancelled, and nothing is written.
#[gpui::test]
fn cancel_ends_as_cancelled_and_writes_nothing(cx: &mut TestAppContext) {
    let scratch = Scratch::new("cancel");
    let source = scratch.file("a.md", PARAGRAPH.as_bytes());
    let work = work(swapping());
    // Paused, so the item waits and the cancel finds it waiting.
    work.queue.pause();
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work.clone()));
    queue.update(cx, |queue, cx| queue.hand(vec![source], cx));
    cx.run_until_parked();
    let id = ids(&queue, cx)[0];
    queue.update(cx, |queue, cx| queue.rewrite(&[id], cx));
    until(cx, "the push", |_| !work.queue.items().is_empty());
    queue.update(cx, |queue, _| queue.cancel(id));
    until(cx, "the cancel", |cx| status(&queue, cx) == "cancelled");
    assert!(!scratch.0.join("a.rewritten.md").exists(), "a cancel wrote");
    work.queue.resume();
}

/// R3, В4: a paste is rewritten into the queue's row and nowhere on disk;
/// Copy the result reads it back from there, and the journal row keeps no
/// text.
#[gpui::test]
fn a_paste_is_rewritten_into_its_row_and_nowhere_on_disk(cx: &mut TestAppContext) {
    let scratch = Scratch::new("paste-row");
    let work = work(swapping());
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work.clone()));
    queue.update(cx, |queue, cx| {
        queue.land(vec![Handed::Text(PARAGRAPH.to_owned())], cx)
    });
    cx.run_until_parked();
    let id = ids(&queue, cx)[0];
    queue.update(cx, |queue, cx| queue.rewrite(&[id], cx));
    until(cx, "the push", |_| !work.queue.items().is_empty());
    let view = work.queue.items()[0].clone();
    assert_eq!(view.destination, Some(Destination::Row));
    until(cx, "the end", |cx| status(&queue, cx) == "rewritten");
    let result = work.queue.result(view.id).expect("row").expect("a result");
    let text = result["text"].as_str().expect("the text is in the row");
    assert_ne!(text, PARAGRAPH);
    let mut names: Vec<String> = std::fs::read_dir(&scratch.0)
        .expect("list")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    assert_eq!(names, ["results"], "a paste's rewrite reached the disk");

    cx.update(|_, cx| super::copy_result(&queue, id, cx));
    until(cx, "the copy", |cx| {
        cx.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
            .as_deref()
            == Some(text)
    });
    until(cx, "the journal", |_| {
        work.journal
            .rows()
            .first()
            .is_some_and(|row| row.state == "done")
    });
    assert!(
        !work.journal.rows()[0].entry.contains("twelve"),
        "the row kept the text"
    );
}

/// R3: in place sets the original aside first, as the windows' clean does.
#[gpui::test]
fn in_place_sets_the_original_aside(cx: &mut TestAppContext) {
    let scratch = Scratch::new("in-place");
    let source = scratch.file("a.md", PARAGRAPH.as_bytes());
    let work = work(swapping());
    let (queue, preferences, cx) = queue_with(cx, &scratch, Some(work.clone()));
    preferences.update(cx, |preferences, cx| {
        preferences.select_destination(Goes::Replace, cx);
    });
    queue.update(cx, |queue, cx| queue.hand(vec![source.clone()], cx));
    cx.run_until_parked();
    let id = ids(&queue, cx)[0];
    queue.update(cx, |queue, cx| queue.rewrite(&[id], cx));
    until(cx, "the end", |cx| status(&queue, cx) == "rewritten");
    assert_eq!(
        std::fs::read_to_string(scratch.0.join("a.original.md")).expect("set aside"),
        PARAGRAPH
    );
    assert_ne!(std::fs::read_to_string(&source).expect("read"), PARAGRAPH);
}

/// D325: a fresh row has Clean and Rewrite on the row itself, the same
/// roads as the Actions items, greyed with the same reasons.
#[gpui::test]
fn the_row_buttons_run_the_menus_road_and_say_its_reason(cx: &mut TestAppContext) {
    let scratch = Scratch::new("buttons");
    let source = scratch.file("a.md", PARAGRAPH.as_bytes());
    let work = work(swapping());
    work.queue.pause();
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work.clone()));
    queue.update(cx, |queue, cx| queue.hand(vec![source], cx));
    cx.run_until_parked();
    let id = ids(&queue, cx)[0];
    let selector: &'static str = Box::leak(format!("row-rewrite-{id}").into_boxed_str());
    let clean: &'static str = Box::leak(format!("row-clean-{id}").into_boxed_str());
    assert!(cx.debug_bounds(clean).is_some(), "no Clean on a fresh row");
    let bounds = cx
        .debug_bounds(selector)
        .expect("no Rewrite on a fresh row");
    cx.simulate_click(bounds.center(), gpui::Modifiers::default());
    until(cx, "the push", |_| !work.queue.items().is_empty());
    assert_eq!(status(&queue, cx), "rewrite-queued");
    // Now greyed, with the menu's own reason: both read one function.
    let reason = cx.update(|_, cx| queue.read(cx).why_not_rewrite(id, cx));
    assert_eq!(reason, Some(t(Message::QueueActionRewriteBusy)));
    let clean_reason = cx.update(|_, cx| {
        let queue = queue.read(cx);
        let row = queue.rows.iter().find(|row| row.id == id).expect("row");
        super::why_not_clean(&row.status, row.cleanable())
    });
    assert_eq!(clean_reason, Some(t(Message::QueueActionCleanRewriting)));
    // A second press goes nowhere: one item.
    cx.simulate_click(bounds.center(), gpui::Modifiers::default());
    cx.run_until_parked();
    assert_eq!(work.queue.items().len(), 1);
    work.queue.resume();
}

/// В3: the table is the journal — a restart reads its rows back, finished
/// ones with what they came to, and a row from the command line says so.
#[gpui::test]
fn a_restart_reads_the_rows_back(cx: &mut TestAppContext) {
    let scratch = Scratch::new("restart");
    let source = scratch.file("a.md", PARAGRAPH.as_bytes());
    let db = scratch.0.join("wipemark.db");
    {
        let store = Arc::new(Store::open(&db).expect("db"));
        let journal = Journal::new(store);
        let entry = Entry {
            name: Some("a.md".to_owned()),
            path: Some(source.to_string_lossy().into_owned()),
            kind: Some("text".to_owned()),
            outcome: Some(wipemark_store::entry::Outcome {
                verdict: "cleaned".to_owned(),
                ..Default::default()
            }),
            ..Entry::default()
        };
        journal.record(&wipemark_store::NewRow {
            origin: Origin::Cli.as_str(),
            action: "clean",
            state: Phase::Done.as_str(),
            item: None,
            arrived: 1,
            ended: Some(journal::now_ms()),
            entry: &entry.to_json(),
        });
        // And one of this window's own, finished before the restart.
        journal.record(&wipemark_store::NewRow {
            origin: Origin::Window.as_str(),
            action: "rewrite",
            state: Phase::Done.as_str(),
            item: None,
            arrived: 2,
            ended: Some(journal::now_ms()),
            entry: &Entry {
                name: Some("pasted".to_owned()),
                kind: Some("text".to_owned()),
                result: Some(Delivered::Caller),
                ..Entry::default()
            }
            .to_json(),
        });
    }
    let store = Arc::new(Store::open(&db).expect("db again"));
    let work = work_over(store, Some(swapping()));
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work));
    until(cx, "the rows", |cx| ids(&queue, cx).len() == 2);
    let (origin, said, has_file) = cx.update(|_, cx| {
        let row = &queue.read(cx).rows[0];
        (row.origin, row.said().cloned(), row.arrival.is_some())
    });
    assert_eq!(origin, Origin::Cli);
    assert!(has_file, "a row with a file is read again from it");
    let said = said.expect("its status is the journal's");
    assert_eq!(said.phase, Phase::Done);
    assert_eq!(
        said.outcome.map(|outcome| outcome.verdict),
        Some("cleaned".to_owned())
    );
    // The window's own row came back too, with nothing behind it.
    let (origin, has_file) = cx.update(|_, cx| {
        let row = &queue.read(cx).rows[1];
        (row.origin, row.arrival.is_some())
    });
    assert_eq!(origin, Origin::Window);
    assert!(!has_file);
}

/// В3: Remove takes a row out of the list and the journal, and its queue
/// item with it; Clear finished takes every finished one.
#[gpui::test]
fn remove_and_clear_finished_take_rows_out_of_the_journal(cx: &mut TestAppContext) {
    let scratch = Scratch::new("remove");
    let work = work(swapping());
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work.clone()));
    queue.update(cx, |queue, cx| {
        queue.land(
            vec![
                Handed::Text(PARAGRAPH.to_owned()),
                Handed::Text("plain words".to_owned()),
            ],
            cx,
        )
    });
    until(cx, "the rows", |_| work.journal.rows().len() == 2);
    let all = ids(&queue, cx);
    queue.update(cx, |queue, cx| queue.rewrite(&all[..1], cx));
    until(cx, "the end", |cx| statuses(&queue, cx)[0] == "rewritten");
    let item = work.queue.items()[0].id;
    queue.update(cx, |queue, cx| queue.clear_finished(cx));
    until(cx, "the journal", |_| work.journal.rows().len() == 1);
    assert_eq!(ids(&queue, cx), all[1..].to_vec());
    until(cx, "the queue row", |_| {
        work.queue.items().iter().all(|view| view.id != item)
    });
    queue.update(cx, |queue, cx| queue.remove(all[1], cx));
    until(cx, "the last row", |_| work.journal.rows().is_empty());
    assert!(ids(&queue, cx).is_empty());
}

/// R7: a clean of a picture runs while a rewrite is running — the cleans
/// have a line of their own.
#[gpui::test]
fn a_clean_of_a_picture_runs_while_a_rewrite_runs(cx: &mut TestAppContext) {
    let scratch = Scratch::new("picture-beside");
    let text = scratch.file("a.md", PARAGRAPH.as_bytes());
    let picture = scratch.file(
        "p.png",
        &std::fs::read(format!(
            "{}/../../fixtures/image/xmp-provenance-url.png",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("a fixture"),
    );
    // Slow enough that the rewrite is still running when the clean ends.
    let work = work(swapping().with_token_delay(Duration::from_millis(300)));
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work.clone()));
    queue.update(cx, |queue, cx| queue.hand(vec![text, picture], cx));
    cx.run_until_parked();
    let all = ids(&queue, cx);
    queue.update(cx, |queue, cx| queue.rewrite(&all[..1], cx));
    until(cx, "the rewrite to start", |cx| {
        statuses(&queue, cx)[0] == "rewriting"
    });
    queue.update(cx, |queue, cx| queue.clean(&all[1..], cx));
    until(cx, "the clean", |cx| statuses(&queue, cx)[1] == "cleaned");
    assert_eq!(
        statuses(&queue, cx)[0],
        "rewriting",
        "the clean waited for the rewrite"
    );
    work.queue.cancel(work.queue.items()[0].id);
}

/// R5: Compare of a rewritten row opens on the rewrite as it was delivered
/// — the bytes the queue wrote — not on a clean of the original.
#[gpui::test]
fn compare_of_a_rewritten_row_is_the_delivered_text(cx: &mut TestAppContext) {
    let scratch = Scratch::new("compare-rewrite");
    let source = scratch.file("a.md", PARAGRAPH.as_bytes());
    let work = work(swapping());
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work));
    queue.update(cx, |queue, cx| queue.hand(vec![source.clone()], cx));
    cx.run_until_parked();
    let id = ids(&queue, cx)[0];
    queue.update(cx, |queue, cx| queue.rewrite(&[id], cx));
    until(cx, "the end", |cx| status(&queue, cx) == "rewritten");
    let subject = cx
        .update(|_, cx| queue.read(cx).subject_of(id))
        .expect("a subject");
    let loaded = subject.read().expect("read");
    let delivered: PathBuf = scratch.0.join("a.rewritten.md");
    assert_eq!(
        loaded.cleaned,
        std::fs::read_to_string(delivered).expect("delivered")
    );
    assert_eq!(loaded.text, PARAGRAPH, "the original is the source");
    assert!(matches!(
        loaded.kind,
        crate::compare::MadeKind::Rewritten { .. }
    ));
}

/// D318: a row nobody asked to process is not said to be waiting — in
/// every language — and its tooltip names what would process it.
#[test]
fn a_row_nobody_asked_for_is_not_said_to_wait() {
    use wipemark_i18n::{available_languages, Localizer, Rendering};
    for language in available_languages() {
        let localizer =
            Localizer::for_languages(std::slice::from_ref(&language.id), Rendering::PlainText);
        let word = localizer.format(Message::QueueStatusWaiting).to_lowercase();
        for waiting in ["wait", "ждёт", "ждет", "wartet"] {
            assert!(!word.contains(waiting), "{}: {word}", language.id);
        }
    }
    let tooltip = t(Message::QueueStatusWaitingTooltip);
    assert!(
        tooltip.contains("Rewrite") && tooltip.contains("Clean"),
        "{tooltip}"
    );
    // And no badge reads as a missing file (D326).
    assert!(!t(Message::QueueStatusNothingFound).contains("Nothing found"));
}

/// R4: the bookkeeper writes a window rewrite's start and end into its row.
#[test]
fn the_bookkeeper_writes_a_window_rewrites_end() {
    let store = Arc::new(Store::in_memory().expect("memory"));
    let journal = Journal::new(Arc::clone(&store));
    let id = journal
        .record(&wipemark_store::NewRow {
            origin: "window",
            action: "rewrite",
            state: "queued",
            item: Some(3),
            arrived: 1,
            ended: None,
            entry: "{}",
        })
        .expect("a row");
    journal::book(&journal, &BatchEvent::Started { item: ItemId(3) });
    assert_eq!(journal.rows()[0].state, "running");
    journal::book(
        &journal,
        &BatchEvent::Ended {
            item: ItemId(3),
            end: wipemark_queue::End::Cancelled,
        },
    );
    let row = journal
        .rows()
        .into_iter()
        .find(|row| row.id == id)
        .expect("row");
    assert_eq!(row.state, "cancelled");
    assert!(row.ended.is_some());
    // A start heard late does not reopen it.
    journal::book(&journal, &BatchEvent::Started { item: ItemId(3) });
    assert_eq!(journal.rows()[0].state, "cancelled");
    let _ = Source::Text(String::new());
}

/// В1, D318: with "Process what arrives" set, a drop goes straight into a
/// line — cleaned, or queued for rewrite — and is never left not started.
#[gpui::test]
fn process_what_arrives_puts_a_drop_straight_in_a_line(cx: &mut TestAppContext) {
    let scratch = Scratch::new("on-arrival");
    let first = scratch.file("a.md", PARAGRAPH.as_bytes());
    let second = scratch.file("b.md", PARAGRAPH.as_bytes());
    let work = work(swapping());
    work.queue.pause();
    let (queue, preferences, cx) = queue_with(cx, &scratch, Some(work.clone()));

    preferences.update(cx, |preferences, cx| {
        preferences.select_on_arrival(super::OnArrival::Clean, cx);
    });
    queue.update(cx, |queue, cx| queue.hand(vec![first], cx));
    // Nothing to remove in it: the clean ran and found nothing.
    until(cx, "the clean", |cx| status(&queue, cx) == "nothing-found");

    preferences.update(cx, |preferences, cx| {
        preferences.select_on_arrival(super::OnArrival::Rewrite, cx);
    });
    queue.update(cx, |queue, cx| queue.hand(vec![second], cx));
    until(cx, "the push", |_| !work.queue.items().is_empty());
    assert_eq!(statuses(&queue, cx)[1], "rewrite-queued");
    work.queue.resume();
}

// ## The host verification's fixes (D355–D364)

/// D356 (M2): a journal row naming a FIFO — what `clean /dev/stdin` or
/// `clean <(…)` once left — never blocks the read: it comes back as a row
/// with nothing behind it, and the row beside it, with a real file, is read
/// again from its file. Without the check the look opens the FIFO and waits
/// for a writer that never comes — the read, and this test, hang.
#[cfg(unix)]
#[gpui::test]
fn a_journal_row_naming_a_fifo_never_blocks_the_read(cx: &mut TestAppContext) {
    let scratch = Scratch::new("fifo-row");
    let source = scratch.file("a.md", PARAGRAPH.as_bytes());
    let fifo = scratch.0.join("pipe");
    let made = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("mkfifo runs");
    assert!(made.success());
    let store = Arc::new(Store::in_memory().expect("memory"));
    let journal = Journal::new(Arc::clone(&store));
    for path in [&fifo, &source] {
        journal.record(&wipemark_store::NewRow {
            origin: Origin::Cli.as_str(),
            action: "clean",
            state: Phase::Done.as_str(),
            item: None,
            arrived: 1,
            ended: Some(journal::now_ms()),
            entry: &Entry {
                path: Some(path.to_string_lossy().into_owned()),
                kind: Some("text".to_owned()),
                ..Entry::default()
            }
            .to_json(),
        });
    }
    let work = work_over(store, Some(swapping()));
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work));
    until(cx, "both rows, looked at", |cx| {
        cx.update(|_, cx| {
            let queue = queue.read(cx);
            queue.rows.len() == 2
                && queue
                    .rows
                    .iter()
                    .all(|row| !matches!(row.preview, crate::preview::Preview::Pending))
        })
    });
    let behind: Vec<bool> = cx.update(|_, cx| {
        queue
            .read(cx)
            .rows
            .iter()
            .map(|row| row.arrival.is_some())
            .collect()
    });
    assert_eq!(
        behind,
        vec![false, true],
        "the FIFO is no file; the file is"
    );
    assert!(
        !cx.update(|_, cx| queue.read(cx).reading),
        "the read never ended"
    );
}

/// D357 (M3): Rewrite of a row whose `name.rewritten.ext` is already there
/// is refused at once — nothing pushed, nothing run — with Replace offered;
/// Replace then rewrites over that one file.
#[gpui::test]
fn a_rewrite_over_an_existing_result_is_refused_before_it_runs(cx: &mut TestAppContext) {
    let scratch = Scratch::new("rewrite-exists");
    let source = scratch.file("article.md", PARAGRAPH.as_bytes());
    let existing = scratch.file("article.rewritten.md", b"somebody's own file");
    let work = work(swapping());
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work.clone()));
    queue.update(cx, |queue, cx| queue.hand(vec![source], cx));
    cx.run_until_parked();
    let id = ids(&queue, cx)[0];
    queue.update(cx, |queue, cx| queue.rewrite(&[id], cx));
    until(cx, "the refusal", |cx| {
        status(&queue, cx) == "rewrite-failed"
    });
    assert!(work.queue.states().is_empty(), "an item was pushed and run");
    assert_eq!(
        cx.update(|_, cx| queue.read(cx).rows[0].existing.clone()),
        Some(existing.clone())
    );
    assert_eq!(
        std::fs::read(&existing).expect("read"),
        b"somebody's own file"
    );

    queue.update(cx, |queue, cx| queue.replace(id, cx));
    until(cx, "the replacement", |cx| {
        status(&queue, cx) == "rewritten"
    });
    let items = work.queue.items();
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0].destination,
        Some(Destination::File(existing.clone()))
    );
    assert_ne!(
        std::fs::read(&existing).expect("read"),
        b"somebody's own file"
    );
}

/// D358 (L1): the row says "queued" before its item is pushed — with the
/// journal's writer held still, nothing runs; let go, the item runs and its
/// end is the row's last word. Pushed before the row was written, the item
/// ran and ended while the writer was held, the bookkeeper found no row
/// naming it, and "queued" landed after: a row queued for ever.
#[gpui::test]
fn the_queued_row_is_written_before_its_item_can_end(cx: &mut TestAppContext) {
    let scratch = Scratch::new("queued-first");
    let work = work(swapping());
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work.clone()));
    queue.update(cx, |queue, cx| {
        queue.land(vec![Handed::Text(PARAGRAPH.to_owned())], cx)
    });
    until(cx, "the row", |_| work.journal.rows().len() == 1);
    let id = ids(&queue, cx)[0];
    let go = cx.update(|_, cx| queue.read(cx).writer.as_ref().expect("a writer").gate());
    queue.update(cx, |queue, cx| queue.rewrite(&[id], cx));
    // Long enough for an item pushed at once to have run and ended.
    let held_until = Instant::now() + Duration::from_millis(1500);
    while Instant::now() < held_until {
        cx.executor().advance_clock(super::rewriting::TICK);
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(20));
    }
    go.send(()).expect("the writer waits");
    until(cx, "the row's end", |_| {
        work.journal.rows()[0].state == Phase::Done.as_str()
    });
    // And it stays ended.
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(work.journal.rows()[0].state, Phase::Done.as_str());
}

/// D371 (L-a): a row removed after its item's id was reserved and before
/// the push — which runs later, on the journal writer's thread, here held
/// still — is never rewritten: the push finds the remove waiting for it, and
/// no `name.rewritten.ext` is written for a row that is gone.
#[gpui::test]
fn a_row_removed_before_its_push_is_never_rewritten(cx: &mut TestAppContext) {
    let scratch = Scratch::new("removed-before-push");
    let source = scratch.file("a.md", PARAGRAPH.as_bytes());
    let work = work(swapping());
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work.clone()));
    queue.update(cx, |queue, cx| queue.hand(vec![source], cx));
    cx.run_until_parked();
    let id = ids(&queue, cx)[0];
    let go = cx.update(|_, cx| queue.read(cx).writer.as_ref().expect("a writer").gate());
    queue.update(cx, |queue, cx| queue.rewrite(&[id], cx));
    until(cx, "the reserve", |cx| {
        cx.update(|_, cx| queue.read(cx).rows[0].item.is_some())
    });
    queue.update(cx, |queue, cx| queue.remove(id, cx));
    go.send(()).expect("the writer waits");
    // Long enough for an item pushed and run to have written its result.
    let waited_until = Instant::now() + Duration::from_millis(1500);
    while Instant::now() < waited_until {
        cx.executor().advance_clock(super::rewriting::TICK);
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        !scratch.0.join("a.rewritten.md").exists(),
        "a removed row's document was rewritten"
    );
    assert!(work.queue.states().is_empty(), "{:?}", work.queue.states());
}

/// D374 (L-h): a template saved before D369 that holds an invisible
/// character — written straight to its row here, as an older build wrote it
/// — refuses the row's Rewrite at the push, naming the template and the
/// rule, rather than being found out as the job renders: nothing is pushed
/// and nothing is written.
#[gpui::test]
fn a_saved_template_the_rules_refuse_refuses_the_push_by_name(cx: &mut TestAppContext) {
    use wipemark_pipeline::lang::Lang;
    use wipemark_pipeline::prompt::{row, Override, Role as PromptRole, Slot, Tactic};

    let scratch = Scratch::new("template-refused");
    let source = scratch.file("a.md", PARAGRAPH.as_bytes());
    let work = work(swapping());
    let user = Slot::new(Lang::En, Tactic::Paraphrase, 1, PromptRole::User).expect("a slot");
    let saved = Override::by_hand(user, "Say it\u{200B} again.\n{PROTECTED}\n{TEXT}");
    work.journal
        .store()
        .settings()
        .set(
            &row::key(user),
            &serde_json::from_str::<serde_json::Value>(&saved.to_json()).expect("JSON"),
        )
        .expect("the row is written");
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work.clone()));
    queue.update(cx, |queue, cx| queue.hand(vec![source], cx));
    cx.run_until_parked();
    let id = ids(&queue, cx)[0];
    queue.update(cx, |queue, cx| queue.rewrite(&[id], cx));
    until(cx, "the refusal", |cx| {
        status(&queue, cx) == "rewrite-failed"
    });
    assert!(work.queue.states().is_empty(), "an item was pushed");
    let said = cx.update(|_, cx| match &queue.read(cx).rows[0].status {
        Status::Recorded(said) => said
            .outcome
            .as_ref()
            .and_then(|outcome| outcome.reason.clone())
            .unwrap_or_default(),
        other => panic!("not refused: {other:?}"),
    });
    assert!(said.contains(&row::key(user)), "{said}");
    assert!(said.contains("invisible-character"), "{said}");
    assert!(!scratch.0.join("a.rewritten.md").exists());
}

/// D374: with no plan to read the chosen templates off — a file that cannot
/// be read now — every saved template of a tactic on the job's ladder is
/// asked, in any language; one of a tactic the job does not climb is not.
#[test]
fn without_a_plan_every_template_on_the_ladder_is_asked() {
    use wipemark_pipeline::lang::Lang;
    use wipemark_pipeline::prompt::{row, Override, Overrides, Role as PromptRole, Slot, Tactic};

    let bad = |tactic| {
        let slot = Slot::new(Lang::De, tactic, 1, PromptRole::User).expect("a slot");
        (
            slot,
            Override::by_hand(slot, "Sag es\u{200B} anders.\n{PROTECTED}\n{TEXT}"),
        )
    };
    let options_with = |slot, saved| {
        let mut overrides = Overrides::new();
        overrides.insert(slot, saved);
        super::rewriting::asked(wipemark_pipeline::prepare::TextFormat::Plain)
            .options(Executor::LocalCpu, overrides, None)
            .expect("offered")
    };
    let (slot, saved) = bad(Tactic::Paraphrase);
    assert_eq!(
        super::rewriting::refused_template(None, &options_with(slot, saved)),
        Some((row::key(slot), "invisible-character"))
    );
    let (slot, saved) = bad(Tactic::Humanize);
    assert_eq!(
        super::rewriting::refused_template(None, &options_with(slot, saved)),
        None
    );
}

/// D359 (L2): what the toolbar and the status bar read on every frame is
/// the queue's own word, not a query — the paused row changed behind the
/// queue's back is not what the window says.
#[gpui::test]
fn the_window_reads_no_row_to_say_the_queue_is_paused(cx: &mut TestAppContext) {
    let scratch = Scratch::new("paused-frame");
    let work = work(swapping());
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work.clone()));
    let events = work.queue.subscribe();
    work.queue.pause();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !matches!(
        events.recv_deadline(deadline).expect("paused"),
        BatchEvent::Paused
    ) {}
    queue.update(cx, |queue, cx| {
        queue.land(vec![Handed::Text(PARAGRAPH.to_owned())], cx)
    });
    until(cx, "the row", |_| work.journal.rows().len() == 1);
    let id = ids(&queue, cx)[0];
    queue.update(cx, |queue, cx| queue.rewrite(&[id], cx));
    until(cx, "the item", |_| !work.queue.states().is_empty());
    let paused_line = |cx: &mut VisualTestContext| {
        cx.update(|_, cx| {
            let queue = queue.read(cx);
            (queue.rewrites_paused(), queue.rewrite_line())
        })
    };
    let (paused, line) = paused_line(cx);
    assert!(paused);
    let line = line.expect("a line");
    work.journal
        .store()
        .queue()
        .set_paused(false)
        .expect("written behind the queue's back");
    assert_eq!(
        paused_line(cx),
        (true, Some(line)),
        "the window queried the row"
    );
    work.queue.resume();
}
