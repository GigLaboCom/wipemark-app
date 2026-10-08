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
    Destination, Durability, End, EngineSource, Failure, Queue, QueueEvent, Undelivered, Whereto,
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

/// D359: whether the queue is paused is answered from memory — a window
/// asks on every frame, and a query there is a query a frame. The row
/// changed behind the queue's back is not what it says; a restart reads
/// the row.
#[test]
fn paused_is_answered_from_memory_never_from_the_row() {
    let store = Arc::new(Store::in_memory().expect("memory"));
    let queue = Queue::on(
        Arc::clone(&store),
        Durability::Memory { detail: None },
        Arc::new(engine(None)),
    )
    .expect("opens");
    let events = queue.events();
    assert!(!queue.paused());
    queue.pause();
    wait_for(&events, |event| matches!(event, QueueEvent::Paused));
    assert!(queue.paused());
    store.queue().set_paused(false).expect("written");
    assert!(queue.paused(), "the row was read, not the queue's own word");
    queue.resume();
    wait_for(&events, |event| matches!(event, QueueEvent::Resumed));
    assert!(!queue.paused());
}

/// A source that says where an engine would send a document now, which a
/// test moves (D361).
struct Going {
    engine: Arc<dyn RewriteEngine>,
    now: Mutex<Option<Whereto>>,
}

impl Going {
    fn new(now: Whereto) -> Arc<Going> {
        Arc::new(Going {
            engine: Arc::new(engine(None)),
            now: Mutex::new(Some(now)),
        })
    }

    fn go(&self, now: Whereto) {
        *self.now.lock().expect("lock") = Some(now);
    }
}

impl EngineSource for Going {
    fn for_item(&self) -> Result<Arc<dyn RewriteEngine>, Unavailable> {
        Ok(Arc::clone(&self.engine))
    }

    fn whereto(&self) -> Option<Whereto> {
        self.now.lock().expect("lock").clone()
    }
}

fn queue_going(source: &Arc<Going>) -> Queue {
    Queue::with_source(
        Arc::new(Store::in_memory().expect("memory")),
        Durability::Memory { detail: None },
        Arc::clone(source) as Arc<dyn EngineSource>,
    )
    .expect("opens")
}

/// No `Started` for `wait`.
fn nothing_starts(events: &flume::Receiver<QueueEvent>, wait: Duration) {
    let deadline = std::time::Instant::now() + wait;
    while let Ok(event) = events.recv_deadline(deadline) {
        assert!(
            !matches!(event, QueueEvent::Started { .. }),
            "started while it should not: {event:?}"
        );
    }
}

/// D361 (L4): an item pushed while rewriting stayed here is not sent to an
/// endpoint the duty moved to while it waited. The queue holds and asks
/// once; nothing starts on no answer; yes to that endpoint runs it — and
/// the item behind it, consented alike, without a second question.
#[test]
fn a_consent_to_stay_here_holds_the_queue_until_the_person_says_yes() {
    let source = Going::new(Whereto::Here);
    let queue = queue_going(&source);
    let events = queue.events();
    queue.pause();
    wait_for(&events, |event| matches!(event, QueueEvent::Paused));
    let text = format!("{}\n", common::PARAGRAPHS[0]);
    let push = |text: &str| {
        let request = text_request(text);
        let id = queue.reserve(&request).expect("reserved");
        queue
            .push_reserved(id, request, Some(Whereto::Here))
            .expect("pushed")
    };
    let first = push(&text);
    let second = push(&text);
    let away = Whereto::Away("https://api.example.com".to_owned());
    source.go(away.clone());
    queue.resume();

    let seen = wait_for(&events, |event| matches!(event, QueueEvent::Ask { .. }));
    assert!(
        seen.iter().any(|event| matches!(
            event,
            QueueEvent::Ask { item, now, was: Whereto::Here, count: 2 }
                if *item == first && *now == away
        )),
        "{seen:?}"
    );
    assert!(queue.asking().is_some());
    nothing_starts(&events, Duration::from_millis(400));

    // An answer about somewhere else is not this question's answer.
    queue.agree(Whereto::Away("https://other.example.com".to_owned()));
    nothing_starts(&events, Duration::from_millis(300));

    queue.agree(away);
    done_text(end_of(&events, first));
    done_text(end_of(&events, second));
    assert!(queue.asking().is_none());
}

/// D361: an item its caller asked for (no consent recorded — an agent's,
/// the command line's) and one consented to the endpoint on duty both run
/// without a question; so does one consented away while the duty came
/// back to this machine.
#[test]
fn a_callers_item_and_one_consented_there_are_never_asked_about() {
    let away = Whereto::Away("https://api.example.com".to_owned());
    let source = Going::new(away.clone());
    let queue = queue_going(&source);
    let events = queue.events();
    let text = format!("{}\n", common::PARAGRAPHS[0]);
    let callers = queue.push(text_request(&text)).expect("pushed");
    done_text(end_of(&events, callers));
    let request = text_request(&text);
    let id = queue.reserve(&request).expect("reserved");
    let there = queue
        .push_reserved(id, request, Some(away.clone()))
        .expect("pushed");
    done_text(end_of(&events, there));
    source.go(Whereto::Here);
    let request = text_request(&text);
    let id = queue.reserve(&request).expect("reserved");
    let back = queue
        .push_reserved(id, request, Some(away))
        .expect("pushed");
    let seen = wait_for(
        &events,
        |event| matches!(event, QueueEvent::Ended { item, .. } if *item == back),
    );
    assert!(
        !seen
            .iter()
            .any(|event| matches!(event, QueueEvent::Ask { .. })),
        "{seen:?}"
    );
}
