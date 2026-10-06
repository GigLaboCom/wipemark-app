//! The queue through its handle, on `FakeEngine`: order, pause, cancel,
//! a document that cannot be read, a restart, where a result goes, and a
//! database that will not open.

mod common;

use std::sync::Arc;
use std::time::Duration;

use common::{
    decided_in, document, end_of, engine, file_request, paragraph_of, reference, resumed_in,
    text_request, wait_for, Scratch, PARAGRAPHS,
};
use wipemark_engine::fake::FakeEngine;
use wipemark_pipeline::Event;
use wipemark_queue::{
    Destination, Durability, End, Failure, ItemId, Keep, Queue, QueueEvent, Refused, State,
    Undelivered, Unread,
};
use wipemark_store::Store;

const SLOW: Option<Duration> = Some(Duration::from_millis(4));

fn in_memory(engine: &FakeEngine) -> Queue {
    Queue::on(
        Arc::new(Store::in_memory().expect("memory")),
        Durability::Memory { detail: None },
        Arc::new(engine.clone()),
    )
    .expect("opens")
}

fn done(end: End) -> wipemark_queue::Done {
    match end {
        End::Done(done) => *done,
        other => panic!("not done: {other:?}"),
    }
}

fn state(queue: &Queue, item: ItemId) -> State {
    queue
        .items()
        .into_iter()
        .find(|view| view.id == item)
        .expect("listed")
        .state
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

#[test]
fn items_run_one_at_a_time_in_the_order_they_came() {
    let engine = engine(SLOW);
    let queue = in_memory(&engine);
    let events = queue.events();
    let texts = [
        format!("{}\n\n{}\n", PARAGRAPHS[0], PARAGRAPHS[1]),
        format!("{}\n", PARAGRAPHS[2]),
        format!("{}\n\n{}\n", PARAGRAPHS[3], PARAGRAPHS[4]),
    ];
    let ids: Vec<ItemId> = texts
        .iter()
        .map(|text| queue.push(text_request(text)).expect("pushed"))
        .collect();
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    let seen = wait_for(
        &events,
        |event| matches!(event, QueueEvent::Ended { item, .. } if *item == ids[2]),
    );

    let order: Vec<(char, i64)> = seen
        .iter()
        .filter_map(|event| match event {
            QueueEvent::Started { item } => Some(('s', item.0)),
            QueueEvent::Ended { item, .. } => Some(('e', item.0)),
            _ => None,
        })
        .collect();
    let expected: Vec<(char, i64)> = ids
        .iter()
        .flat_map(|id| [('s', id.0), ('e', id.0)])
        .collect();
    assert_eq!(order, expected, "one at a time, oldest first");
    for (id, text) in ids.iter().zip(&texts) {
        let end = seen
            .iter()
            .find_map(|event| match event {
                QueueEvent::Ended { item, end } if item == id => Some(end.clone()),
                _ => None,
            })
            .expect("ended");
        assert_eq!(done(end).text, reference(text));
    }
}

#[test]
fn pause_stops_the_job_and_resume_carries_its_decided_chunks() {
    let scratch = Scratch::new("pause");
    let engine = engine(SLOW);
    let queue = Queue::open(&scratch.db(), Arc::new(engine.clone()));
    let events = queue.events();
    let first = queue.push(text_request(&document())).expect("pushed");
    let second = queue.push(text_request(PARAGRAPHS[5])).expect("pushed");

    let mut seen = wait_for(&events, |event| {
        decided_in(std::slice::from_ref(event), first).len() == 1
    });
    seen.extend(wait_for(&events, |event| {
        decided_in(std::slice::from_ref(event), first).len() == 1
    }));
    queue.pause();
    seen.extend(wait_for(
        &events,
        |event| matches!(event, QueueEvent::Interrupted { item } if *item == first),
    ));
    let recorded = decided_in(&seen, first);
    assert!((2..6).contains(&recorded.len()), "{recorded:?}");
    assert_eq!(state(&queue, first), State::Queued);
    nothing_starts(&events, Duration::from_millis(300));
    let asked_before = engine.asked().len();

    queue.resume();
    let after = wait_for(
        &events,
        |event| matches!(event, QueueEvent::Ended { item, .. } if *item == first),
    );
    assert_eq!(resumed_in(&after), Some((recorded.len() as u32, 0)));
    let asked: Vec<usize> = engine.asked()[asked_before..]
        .iter()
        .map(paragraph_of)
        .collect();
    let undecided: Vec<usize> = (0..6).filter(|i| !recorded.contains(i)).collect();
    assert_eq!(asked, undecided, "only the chunks with no record are asked");
    let QueueEvent::Ended { end, .. } = after.last().expect("ended").clone() else {
        unreachable!()
    };
    assert_eq!(done(end).text, reference(&document()));
    assert!(matches!(end_of(&events, second), End::Done(_)));
}

#[test]
fn a_paused_queue_stays_paused_after_a_restart() {
    let scratch = Scratch::new("paused-restart");
    let queue = Queue::open(&scratch.db(), Arc::new(engine(None)));
    let events = queue.events();
    queue.pause();
    wait_for(&events, |event| matches!(event, QueueEvent::Paused));
    let item = queue.push(text_request(PARAGRAPHS[0])).expect("pushed");
    wait_for(&events, |event| matches!(event, QueueEvent::Added { .. }));
    queue.shutdown();

    let queue = Queue::open(&scratch.db(), Arc::new(engine(None)));
    let events = queue.events();
    assert_eq!(state(&queue, item), State::Queued);
    nothing_starts(&events, Duration::from_millis(300));
    queue.resume();
    assert!(matches!(end_of(&events, item), End::Done(_)));
}

#[test]
fn cancel_ends_the_running_item_and_the_next_starts() {
    let scratch = Scratch::new("cancel");
    let queue = Queue::open(&scratch.db(), Arc::new(engine(SLOW)));
    let events = queue.events();
    let first = queue.push(text_request(&document())).expect("pushed");
    let second = queue.push(text_request(PARAGRAPHS[5])).expect("pushed");
    wait_for(&events, |event| {
        decided_in(std::slice::from_ref(event), first).len() == 1
    });
    queue.cancel(first);
    assert_eq!(end_of(&events, first), End::Cancelled);
    assert!(matches!(end_of(&events, second), End::Done(_)));
    let store = Store::open(scratch.db()).expect("reopen");
    assert!(
        store.queue().chunks(first.0).expect("chunks").is_empty(),
        "a cancelled item's chunk records go"
    );
    assert_eq!(state(&queue, first), State::Cancelled);
}

#[test]
fn a_cancelled_queued_item_never_starts() {
    let queue = in_memory(&engine(None));
    let events = queue.events();
    queue.pause();
    let first = queue.push(text_request(PARAGRAPHS[0])).expect("pushed");
    let second = queue.push(text_request(PARAGRAPHS[1])).expect("pushed");
    queue.cancel(second);
    queue.resume();
    let seen = wait_for(
        &events,
        |event| matches!(event, QueueEvent::Ended { item, .. } if *item == first),
    );
    assert!(seen.contains(&QueueEvent::Ended {
        item: second,
        end: End::Cancelled
    }));
    assert!(!seen.contains(&QueueEvent::Started { item: second }));
}

#[test]
fn an_unreadable_document_fails_and_the_queue_moves_on() {
    let scratch = Scratch::new("unreadable");
    let missing = scratch.path("not-there.md");
    let picture = scratch.path("holiday.txt");
    std::fs::write(&picture, b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR").expect("a png");
    let queue = in_memory(&engine(None));
    let events = queue.events();
    let a = queue
        .push(file_request(
            &missing,
            Destination::beside(&missing).expect("named"),
        ))
        .expect("pushed");
    let b = queue
        .push(file_request(
            &picture,
            Destination::beside(&picture).expect("named"),
        ))
        .expect("pushed");
    let latin1 = scratch.path("café.txt");
    std::fs::write(&latin1, b"Le caf\xe9 est pr\xeat, et la cr\xe8me aussi.\n").expect("8-bit");
    let d = queue
        .push(file_request(
            &latin1,
            Destination::beside(&latin1).expect("named"),
        ))
        .expect("pushed");
    let c = queue.push(text_request(PARAGRAPHS[0])).expect("pushed");
    assert_eq!(
        end_of(&events, a),
        End::Failed(Box::new(Failure::Unread(Unread::Missing)))
    );
    assert!(matches!(
        end_of(&events, b),
        End::Failed(failure) if matches!(*failure, Failure::Unread(Unread::NotText { .. }))
    ));
    assert_eq!(
        end_of(&events, d),
        End::Failed(Box::new(Failure::Unread(Unread::UnnamedEncoding))),
        "an eight-bit encoding is refused, never guessed"
    );
    assert!(matches!(end_of(&events, c), End::Done(_)));
    assert!(!scratch.path("holiday.cleaned.txt").exists());
    let failed = queue
        .items()
        .into_iter()
        .find(|v| v.id == a)
        .expect("listed");
    assert_eq!(failed.state, State::Failed);
    assert_eq!(
        failed.result.expect("a result")["failure"]["reason"],
        "missing"
    );
}

#[test]
fn an_interrupted_item_resumes_after_a_restart() {
    let scratch = Scratch::new("restart");
    let slow = engine(SLOW);
    let queue = Queue::open(&scratch.db(), Arc::new(slow));
    let events = queue.events();
    let item = queue.push(text_request(&document())).expect("pushed");
    let mut seen = Vec::new();
    while decided_in(&seen, item).len() < 2 {
        seen.extend(wait_for(&events, |event| {
            decided_in(std::slice::from_ref(event), item).len() == 1
        }));
    }
    queue.shutdown();
    let recorded = Store::open(scratch.db())
        .expect("reopen")
        .queue()
        .chunks(item.0)
        .expect("chunks")
        .len();
    assert!((2..6).contains(&recorded), "{recorded}");

    let fresh = engine(None);
    let queue = Queue::open(&scratch.db(), Arc::new(fresh.clone()));
    let events = queue.events();
    let seen = wait_for(&events, |event| matches!(event, QueueEvent::Ended { .. }));
    assert_eq!(resumed_in(&seen), Some((recorded as u32, 0)));
    assert_eq!(fresh.asked().len(), 6 - recorded);
    let QueueEvent::Ended { item: ended, end } = seen.last().expect("ended").clone() else {
        unreachable!()
    };
    assert_eq!(ended, item);
    assert_eq!(done(end).text, reference(&document()));
}

#[test]
fn a_file_changed_before_the_resume_is_rewritten_whole() {
    let scratch = Scratch::new("changed");
    let source = scratch.path("notes.txt");
    std::fs::write(&source, document()).expect("write");
    let engine = engine(SLOW);
    let queue = Queue::open(&scratch.db(), Arc::new(engine.clone()));
    let events = queue.events();
    let item = queue
        .push(file_request(
            &source,
            Destination::beside(&source).expect("named"),
        ))
        .expect("pushed");
    let mut seen = Vec::new();
    while decided_in(&seen, item).len() < 2 {
        seen.extend(wait_for(&events, |event| {
            decided_in(std::slice::from_ref(event), item).len() == 1
        }));
    }
    queue.pause();
    wait_for(&events, |event| {
        matches!(event, QueueEvent::Interrupted { .. })
    });
    let recorded = decided_in(&seen, item).len() as u32;

    // The first paragraph — already decided — is edited.
    let changed = document().replace("an ordinary laptop", "a borrowed laptop");
    std::fs::write(&source, &changed).expect("edit");
    let asked_before = engine.asked().len();
    queue.resume();
    let after = wait_for(&events, |event| matches!(event, QueueEvent::Ended { .. }));
    let (carried, discarded) = resumed_in(&after).expect("resumed");
    assert_eq!(carried, 0);
    assert!(discarded >= recorded, "{discarded} of {recorded}");
    assert_eq!(engine.asked().len() - asked_before, 6, "every chunk again");
    assert_eq!(
        std::fs::read_to_string(scratch.path("notes.cleaned.txt")).expect("beside"),
        reference(&changed)
    );
}

#[test]
fn a_result_goes_beside_the_file_and_the_file_is_untouched() {
    let scratch = Scratch::new("beside");
    let source = scratch.path("note.md");
    let original = format!("\u{FEFF}{}\n\n{}\n", PARAGRAPHS[0], PARAGRAPHS[1]);
    std::fs::write(&source, &original).expect("write");
    let queue = in_memory(&engine(None));
    let events = queue.events();
    let item = queue
        .push(file_request(
            &source,
            Destination::beside(&source).expect("named"),
        ))
        .expect("pushed");
    let done = done(end_of(&events, item));
    let beside = scratch.path("note.cleaned.md");
    assert_eq!(
        done.written,
        Some(wipemark_queue::Written {
            path: beside.clone(),
            original: None
        })
    );
    assert_eq!(std::fs::read_to_string(&source).expect("source"), original);
    let written = std::fs::read_to_string(&beside).expect("beside");
    assert_eq!(written, done.text);
    assert!(
        written.starts_with('\u{FEFF}'),
        "the byte order mark is kept"
    );
    assert_ne!(written, original);
    let stored = queue.result(item).expect("reads").expect("a result");
    assert!(
        stored.get("text").is_none(),
        "the text is in the file, not the row"
    );
    assert!(stored["report"]["not_established"].is_array());
}

#[test]
fn a_destination_that_is_the_source_is_refused() {
    let scratch = Scratch::new("over");
    let source = scratch.path("note.md");
    std::fs::write(&source, PARAGRAPHS[0]).expect("write");
    let queue = in_memory(&engine(None));
    let events = queue.events();
    assert_eq!(
        queue.push(file_request(&source, Destination::File(source.clone()))),
        Err(Refused::OverTheSource)
    );
    // Another name for the same file: told only at delivery.
    let link = scratch.path("link.md");
    std::fs::hard_link(&source, &link).expect("a hard link");
    let item = queue
        .push(file_request(&source, Destination::File(link)))
        .expect("pushed");
    assert_eq!(
        end_of(&events, item),
        End::Failed(Box::new(Failure::Undelivered(Undelivered::OverTheSource)))
    );
    assert_eq!(
        std::fs::read_to_string(&source).expect("source"),
        PARAGRAPHS[0]
    );
    let stored = queue.result(item).expect("reads").expect("a result");
    assert_eq!(
        stored["text"].as_str(),
        Some(reference(PARAGRAPHS[0]).as_str()),
        "an undelivered result keeps its text in the row"
    );
}

#[test]
fn in_place_sets_the_original_aside() {
    let scratch = Scratch::new("in-place");
    let source = scratch.path("note.md");
    std::fs::write(&source, PARAGRAPHS[2]).expect("write");
    let queue = in_memory(&engine(None));
    let events = queue.events();
    let item = queue
        .push(file_request(&source, Destination::InPlace(Keep::Original)))
        .expect("pushed");
    let done = done(end_of(&events, item));
    let aside = scratch.path("note.original.md");
    assert_eq!(
        done.written,
        Some(wipemark_queue::Written {
            path: source.clone(),
            original: Some(aside.clone())
        })
    );
    assert_eq!(
        std::fs::read_to_string(&aside).expect("aside"),
        PARAGRAPHS[2]
    );
    assert_eq!(std::fs::read_to_string(&source).expect("file"), done.text);

    // Again: the first original is the original.
    let replaced = std::fs::read(&source).expect("file");
    let again = queue
        .push(file_request(&source, Destination::InPlace(Keep::Original)))
        .expect("pushed");
    assert_eq!(
        end_of(&events, again),
        End::Failed(Box::new(Failure::Undelivered(Undelivered::OriginalExists(
            aside.clone()
        ))))
    );
    assert_eq!(std::fs::read(&source).expect("file"), replaced);
    assert_eq!(
        std::fs::read_to_string(&aside).expect("aside"),
        PARAGRAPHS[2]
    );
}

#[test]
fn in_place_with_nothing_to_change_touches_nothing() {
    let scratch = Scratch::new("in-place-same");
    let source = scratch.path("note.md");
    std::fs::write(&source, PARAGRAPHS[2]).expect("write");
    // An engine that changes nothing: every chunk keeps its source.
    let same = FakeEngine::answering(|req, _| common::text_of(req));
    let queue = in_memory(&same);
    let events = queue.events();
    let item = queue
        .push(file_request(&source, Destination::InPlace(Keep::Original)))
        .expect("pushed");
    assert_eq!(done(end_of(&events, item)).written, None);
    assert!(!scratch.path("note.original.md").exists());
}

/// Set an item's row to "the result is in the row, the file not yet
/// written" — what a `kill -9` between the two leaves.
fn delivering(db: &std::path::Path, item: ItemId, text: &str) {
    let store = Store::open(db).expect("open");
    let result = serde_json::json!({
        "end": "delivering",
        "report": {"not_established": ["x"]},
        "text": text,
        "encoding": "UTF-8",
    });
    store
        .queue()
        .set_result(item.0, "delivering", Some(&result.to_string()))
        .expect("set");
}

#[test]
fn an_interrupted_delivery_is_finished_not_failed() {
    let result = "The rewritten note.";
    // How the original was set aside before the crash: a hard link (D284,
    // the file never moves), a rename (a file system without hard links),
    // or what a hard link looks like where the inode cannot be seen — the
    // same bytes under both names (D286).
    #[derive(Clone, Copy)]
    enum Aside {
        Link,
        Rename,
        Copy,
    }
    // What the first name holds when the queue opens again: what the crash
    // left, bytes written into whatever file is there, or someone's atomic
    // save — a temporary renamed over the first name, a new inode. After a
    // set-aside by a link, a write through the first name would change the
    // set-aside too; an editor's save does not, and that is the state.
    enum Now {
        Left,
        Written(&'static str),
        Saved(String),
    }
    let same_length = "S".repeat(PARAGRAPHS[0].len());
    for (case, aside_by, file_now, ends_done) in [
        (
            "set aside by a link, not yet written",
            Aside::Link,
            Now::Left,
            true,
        ),
        (
            "set aside by a rename, not yet written",
            Aside::Rename,
            Now::Left,
            true,
        ),
        (
            "the same bytes under both names",
            Aside::Copy,
            Now::Left,
            true,
        ),
        (
            "written, not yet recorded",
            Aside::Rename,
            Now::Written(result),
            true,
        ),
        (
            "someone else's bytes",
            Aside::Rename,
            Now::Written("Edited by hand since."),
            false,
        ),
        // D286 compares the bytes, not their length: a save of as many
        // bytes is still someone's, and is not written over (Y2).
        (
            "set aside by a link, then saved over: the same length",
            Aside::Link,
            Now::Saved(same_length.clone()),
            false,
        ),
        (
            "set aside by a link, then saved over: another length",
            Aside::Link,
            Now::Saved(String::from("Saved by an editor since.")),
            false,
        ),
        // The original's very bytes saved back under the first name — a
        // second inode, the same bytes as the set-aside: that is the
        // set-aside as D286 sees it where the inode cannot be seen, and
        // writing the result over it loses nothing.
        (
            "set aside by a link, then the original's bytes saved back",
            Aside::Link,
            Now::Saved(String::from(PARAGRAPHS[0])),
            true,
        ),
    ] {
        let scratch = Scratch::new("delivering");
        let source = scratch.path("note.md");
        let aside = scratch.path("note.original.md");
        std::fs::write(&source, PARAGRAPHS[0]).expect("write");

        let queue = Queue::open(&scratch.db(), Arc::new(engine(None)));
        queue.pause();
        let item = queue
            .push(file_request(&source, Destination::InPlace(Keep::Original)))
            .expect("pushed");
        queue.shutdown();
        delivering(&scratch.db(), item, result);
        match aside_by {
            Aside::Link => std::fs::hard_link(&source, &aside),
            Aside::Rename => std::fs::rename(&source, &aside),
            Aside::Copy => std::fs::copy(&source, &aside).map(drop),
        }
        .expect("the original set aside");
        let left = match &file_now {
            Now::Left => None,
            Now::Written(now) => {
                std::fs::write(&source, now).expect("the file as the crash left it");
                Some(String::from(*now))
            }
            Now::Saved(now) => {
                let temporary = scratch.path(".note.md.save");
                std::fs::write(&temporary, now).expect("the editor's temporary");
                std::fs::rename(&temporary, &source).expect("the editor's save");
                Some(now.clone())
            }
        };

        let queue = Queue::open(&scratch.db(), Arc::new(engine(None)));
        let end = end_of(&queue.events(), item);
        assert_eq!(
            std::fs::read_to_string(&aside).expect("aside"),
            PARAGRAPHS[0],
            "{case}"
        );
        if ends_done {
            assert_eq!(
                end,
                End::Delivered {
                    written: Some(wipemark_queue::Written {
                        path: source.clone(),
                        original: Some(aside.clone()),
                    })
                },
                "{case}"
            );
            assert_eq!(
                std::fs::read_to_string(&source).expect("file"),
                result,
                "{case}"
            );
        } else {
            assert_eq!(
                end,
                End::Failed(Box::new(Failure::Undelivered(Undelivered::OriginalExists(
                    aside.clone()
                )))),
                "{case}"
            );
            assert_eq!(
                Some(std::fs::read_to_string(&source).expect("file")),
                left,
                "{case}"
            );
        }
        queue.shutdown();
    }
}

#[test]
fn text_has_no_file_to_write_and_stays_in_the_row() {
    let scratch = Scratch::new("text");
    let queue = Queue::open(&scratch.db(), Arc::new(engine(None)));
    let events = queue.events();
    assert_eq!(
        queue.push(wipemark_queue::Request {
            destination: Destination::InPlace(Keep::Original),
            ..text_request(PARAGRAPHS[3])
        }),
        Err(Refused::NoFileToReplace)
    );
    let item = queue.push(text_request(PARAGRAPHS[3])).expect("pushed");
    let done = done(end_of(&events, item));
    assert_eq!(done.written, None);
    let stored = queue.result(item).expect("reads").expect("a result");
    assert_eq!(stored["text"].as_str(), Some(done.text.as_str()));
    let listed = queue
        .items()
        .into_iter()
        .find(|v| v.id == item)
        .expect("listed");
    assert!(listed.result.expect("a result").get("text").is_none());

    queue.remove(item);
    wait_for(&events, |event| matches!(event, QueueEvent::Removed { .. }));
    assert!(queue.result(item).expect("reads").is_none());
    assert!(queue.items().is_empty());
}

#[test]
fn a_database_that_will_not_open_is_left_alone_and_the_queue_runs_in_memory() {
    let scratch = Scratch::new("garbage");
    let garbage: Vec<u8> = (0..4096u32).map(|i| (i * 7 % 251) as u8).collect();
    std::fs::write(scratch.db(), &garbage).expect("garbage");
    let queue = Queue::open(&scratch.db(), Arc::new(engine(None)));
    assert!(!queue.durability().survives_a_restart());
    assert!(matches!(
        queue.durability(),
        Durability::Memory { detail: Some(_) }
    ));
    let events = queue.events();
    let item = queue.push(text_request(PARAGRAPHS[0])).expect("pushed");
    assert!(matches!(end_of(&events, item), End::Done(_)));
    queue.shutdown();
    assert_eq!(std::fs::read(scratch.db()).expect("still there"), garbage);
}

#[test]
fn events_say_how_far_a_document_got() {
    let queue = in_memory(&engine(None));
    let events = queue.events();
    let item = queue.push(text_request(&document())).expect("pushed");
    let seen = wait_for(&events, |event| matches!(event, QueueEvent::Ended { .. }));
    assert_eq!(decided_in(&seen, item), vec![0, 1, 2, 3, 4, 5]);
    assert!(seen.iter().any(|event| matches!(
        event,
        QueueEvent::Job {
            event: Event::Stage { .. },
            ..
        }
    )));
    let listed = queue
        .items()
        .into_iter()
        .find(|v| v.id == item)
        .expect("listed");
    assert_eq!(listed.state, State::Done);
}
