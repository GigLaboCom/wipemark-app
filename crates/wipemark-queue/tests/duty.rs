//! The engine, asked when each item starts (R1, E4-6b): a hold while the
//! source has nothing, the next item on whatever engine is on duty by then,
//! every reader hearing every event, and a new file that never replaces
//! somebody's.

mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{end_of, engine, file_request, reference, text_request, wait_for, Scratch};
use wipemark_engine::fake::FakeEngine;
use wipemark_engine::{RewriteEngine, Unavailable};
use wipemark_queue::{
    Destination, Durability, End, EngineSource, Failure, Queue, QueueEvent, Undelivered,
};
use wipemark_store::Store;

/// A source whose answer a test sets: nothing on duty, or an engine — and
/// how many times it was asked.
#[derive(Default)]
struct OnDuty {
    engine: Mutex<Option<Arc<dyn RewriteEngine>>>,
    asked: Mutex<u32>,
}

impl OnDuty {
    fn put(&self, engine: Option<FakeEngine>) {
        *self.engine.lock().expect("lock") = engine.map(|e| Arc::new(e) as Arc<dyn RewriteEngine>);
    }
}

impl EngineSource for OnDuty {
    fn for_item(&self) -> Result<Arc<dyn RewriteEngine>, Unavailable> {
        *self.asked.lock().expect("lock") += 1;
        self.engine
            .lock()
            .expect("lock")
            .clone()
            .ok_or(Unavailable::NothingOnDuty)
    }
}

fn queue_on(source: &Arc<OnDuty>) -> Queue {
    Queue::with_source(
        Arc::new(Store::in_memory().expect("memory")),
        Durability::Memory { detail: None },
        Arc::clone(source) as Arc<dyn EngineSource>,
    )
    .expect("opens")
}

fn done_text(end: End) -> String {
    match end {
        End::Done(done) => done.text,
        other => panic!("not done: {other:?}"),
    }
}

/// Nothing on duty holds the queue with that reason and fails nothing; an
/// engine arriving and the queue told so runs the item on it. Without the
/// hold, the item fails with "nothing on duty" — the failure of a queue
/// opened before the first scan.
#[test]
fn nothing_on_duty_holds_and_an_engine_arriving_runs_the_item() {
    let source = Arc::new(OnDuty::default());
    let queue = queue_on(&source);
    let events = queue.events();
    let text = format!("{}\n", common::PARAGRAPHS[0]);
    let item = queue.push(text_request(&text)).expect("pushed");

    let seen = wait_for(&events, |event| matches!(event, QueueEvent::Held { .. }));
    assert!(
        seen.iter().any(|event| matches!(
            event,
            QueueEvent::Held {
                reason: Unavailable::NothingOnDuty
            }
        )),
        "{seen:?}"
    );
    assert!(!seen
        .iter()
        .any(|event| matches!(event, QueueEvent::Ended { .. } | QueueEvent::Started { .. })));
    assert_eq!(queue.held(), Some(Unavailable::NothingOnDuty));
    // Held, it does not ask again by itself: a source that reads a key
    // from a keychain must not be asked in a loop.
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(*source.asked.lock().expect("lock"), 1);

    let fake = engine(None);
    source.put(Some(fake.clone()));
    queue.engine_changed();
    assert_eq!(done_text(end_of(&events, item)), reference(&text));
    assert!(
        !fake.asked().is_empty(),
        "the item did not run on the engine"
    );
    assert_eq!(queue.held(), None);
}

/// The engine is asked for when each item **starts**: a swap between two
/// items runs the second on the new one, and the first never sees it.
#[test]
fn a_duty_swap_between_two_items_runs_the_second_on_the_new_engine() {
    let source = Arc::new(OnDuty::default());
    let old = engine(None);
    source.put(Some(old.clone()));
    let queue = queue_on(&source);
    let events = queue.events();
    let first_text = format!("{}\n", common::PARAGRAPHS[1]);
    let first = queue.push(text_request(&first_text)).expect("pushed");
    end_of(&events, first);

    let new = engine(None);
    source.put(Some(new.clone()));
    let asked_old = old.asked().len();
    let second_text = format!("{}\n", common::PARAGRAPHS[2]);
    let second = queue.push(text_request(&second_text)).expect("pushed");
    assert_eq!(done_text(end_of(&events, second)), reference(&second_text));
    assert_eq!(
        old.asked().len(),
        asked_old,
        "the second item ran on the old engine"
    );
    assert!(
        !new.asked().is_empty(),
        "the second item never reached the new engine"
    );
    assert_eq!(
        *source.asked.lock().expect("lock"),
        2,
        "asked once per item"
    );
}

/// A subscriber hears what the first reader hears — the end of the item it
/// waits for included — and a dropped one is let go of.
#[test]
fn every_reader_hears_every_event() {
    let queue = Queue::on(
        Arc::new(Store::in_memory().expect("memory")),
        Durability::Memory { detail: None },
        Arc::new(engine(None)),
    )
    .expect("opens");
    let first = queue.events();
    let second = queue.subscribe();
    drop(queue.subscribe());
    let text = format!("{}\n", common::PARAGRAPHS[3]);
    let item = queue.push(text_request(&text)).expect("pushed");
    assert_eq!(done_text(end_of(&first, item)), reference(&text));
    assert_eq!(done_text(end_of(&second, item)), reference(&text));
}

/// A new file beside the source never replaces one already there: the item
/// fails as `exists`, the file is left byte for byte and the result is in
/// the row; `Destination::File` replaces it, as it is asked to.
#[test]
fn a_new_file_never_replaces_one_already_there() {
    let scratch = Scratch::new("new-file");
    let source = scratch.path("note.md");
    std::fs::write(&source, common::document()).expect("write");
    let existing = scratch.path("note.rewritten.md");
    std::fs::write(&existing, "somebody's own file").expect("write");
    let queue = Queue::open(&scratch.db(), Arc::new(engine(None)));
    let events = queue.events();

    let refused = queue
        .push(file_request(
            &source,
            Destination::beside(&source).expect("named"),
        ))
        .expect("pushed");
    match end_of(&events, refused) {
        End::Failed(failure) => assert_eq!(
            *failure,
            Failure::Undelivered(Undelivered::Exists(existing.clone()))
        ),
        other => panic!("not refused: {other:?}"),
    }
    assert_eq!(
        std::fs::read_to_string(&existing).expect("read"),
        "somebody's own file"
    );
    let stored = queue.result(refused).expect("row").expect("a result");
    assert_eq!(stored["failure"]["reason"], "exists");
    assert_eq!(
        stored["text"].as_str(),
        Some(reference(&common::document()).as_str())
    );

    let replaced = queue
        .push(file_request(&source, Destination::File(existing.clone())))
        .expect("pushed");
    done_text(end_of(&events, replaced));
    assert_eq!(
        std::fs::read_to_string(&existing).expect("read"),
        reference(&common::document())
    );
    assert!(!scratch.path("note.cleaned.md").exists());
}
