//! A row's actions depend on what is on disk, never on where the row came
//! from (D729–D733; the owner at the macOS acceptance, 2026-10-14: "the
//! behaviour must not break the UI depending on where a thing came from").
//! Row 36 of the acceptance table — `wipemark-cli rewrite a10-rewrite.md`
//! with the application running — could not be opened in Compare, its file
//! and its result both on disk. Over a real journal and a real batch queue
//! on `FakeEngine`.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui::{Entity, TestAppContext, VisualTestContext};
use wipemark_i18n::{t, Message};
use wipemark_intake::Handed;
use wipemark_queue::{Destination, State};
use wipemark_store::entry::{Action, Delivered, Entry, Origin, Outcome, Phase};
use wipemark_store::{NewRow, Store};

use super::rewrite_tests::{here_on_duty, swapping, until, work_over, PARAGRAPH};
use super::tests::{queue_with, Scratch};
use super::{
    why_no_result_file, why_not_compare, why_not_copy, why_not_open, Item, Queue, Said, Status,
};
use crate::compare::{Home, Made, RewriteFrom, Told};
use crate::drop::Arrival;
use crate::journal::{self, Journal, Work};

/// Row 36's entry, as the acceptance database holds it: the name and the
/// path the command line named, what the rewrite came to, and the file the
/// command line wrote and told the row about.
fn row_36(source: &Path, result: &Path) -> Entry {
    Entry {
        name: Some("a10-rewrite.md".to_owned()),
        said_path: Some(source.to_string_lossy().into_owned()),
        kind: Some("text".to_owned()),
        size: Some(296),
        outcome: Some(Outcome {
            verdict: "rewritten".to_owned(),
            findings: Some(1),
            kept: Some(1),
            chunks: Some(2),
            rewritten: Some(2),
            kept_source: Some(0),
            model: Some("gemma-4-12b-it-qat-ud-q4".to_owned()),
            profile: Some("shipped".to_owned()),
            ..Outcome::default()
        }),
        result: Some(Delivered::File {
            path: result.to_string_lossy().into_owned(),
            original: None,
            replaced: false,
        }),
        ..Entry::default()
    }
}

/// A store in memory holding one ended rewrite row of `origin`, item 11 —
/// what the command line's call through the application leaves — and its
/// journal id.
fn recorded(origin: Origin, entry: &Entry) -> (Arc<Store>, i64) {
    let store = Arc::new(Store::in_memory().expect("memory"));
    let id = record(&store, origin, entry);
    (store, id)
}

/// One more ended rewrite row of `origin` in `store`, item 11; its id.
fn record(store: &Arc<Store>, origin: Origin, entry: &Entry) -> i64 {
    Journal::new(Arc::clone(store))
        .record(&NewRow {
            origin: origin.as_str(),
            action: Action::Rewrite.as_str(),
            state: Phase::Done.as_str(),
            item: Some(11),
            arrived: 1,
            ended: Some(2),
            entry: &entry.to_json(),
        })
        .expect("the row")
}

/// An agent's rewrite with no path at all, its text handed back: what a
/// row is when nothing of the document is kept (D312).
fn text_only() -> Entry {
    Entry {
        kind: Some("text".to_owned()),
        size: Some(181),
        outcome: Some(Outcome {
            verdict: "rewritten".to_owned(),
            ..Outcome::default()
        }),
        result: Some(Delivered::Caller),
        ..Entry::default()
    }
}

/// Wait until the table lists `count` rows and has looked at every file a
/// row names — the look-up D729 runs for a named path too — and say which
/// rows, in arrival order.
fn looked_at(queue: &Entity<Queue>, count: usize, cx: &mut VisualTestContext) -> Vec<u64> {
    until(cx, "the rows, looked at", |cx| {
        queue.read_with(cx, |queue, _| {
            queue.rows.len() == count
                && queue
                    .rows
                    .iter()
                    .all(|row| !matches!(row.preview, crate::preview::Preview::Pending))
        })
    });
    cx.update(|_, cx| queue.read(cx).ids())
}

/// Everything asked of the table's journal writer so far, written.
fn flushed(queue: &Entity<Queue>, cx: &mut VisualTestContext) {
    let done = cx.update(|_, cx| queue.read(cx).writer.as_ref().map(journal::Writer::flushed));
    done.expect("a writer")
        .recv_timeout(Duration::from_secs(10))
        .expect("flushed");
}

/// Journal row `id`'s entry, as it is now.
fn entry_of(work: &Work, id: i64) -> Entry {
    let row = work
        .journal
        .rows()
        .into_iter()
        .find(|row| row.id == id)
        .expect("the journal row");
    Entry::from_json(&row.entry)
}

/// D733, the owner's row 36: the command line rewrote `a10-rewrite.md`
/// through the application, wrote `a10-rewrite.rewritten.md` and told the
/// row; the only path the row has is the one the command line named. Once
/// the table has looked at it, Compare opens on it exactly as on a window
/// row's rewrite — the original the named file, the result its file, the
/// paragraphs kept said — and a save there is told to the row through the
/// same link, whose journal entry is marked edited (D412, D417). With D729
/// undone the row has nothing behind it: Compare greyed, nothing to open —
/// red.
#[gpui::test]
fn row_36_opens_in_compare_as_a_window_rows_rewrite(cx: &mut TestAppContext) {
    let scratch = Scratch::new("row-36");
    let source = scratch.file("a10-rewrite.md", PARAGRAPH.as_bytes());
    let result = scratch.file("a10-rewrite.rewritten.md", b"The rewritten note.\n");
    let (store, entry) = recorded(Origin::Cli, &row_36(&source, &result));
    let work = work_over(store, Some(swapping()));
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work.clone()));
    let id = looked_at(&queue, 1, cx)[0];

    let (comparable, made, subject) = cx.update(|_, cx| {
        let queue = queue.read(cx);
        let row = queue.rows.iter().find(|row| row.id == id).expect("row 36");
        (
            queue.comparable(row),
            queue.made_for(row),
            queue.subject_of(id),
        )
    });
    assert!(comparable, "row 36's Compare is greyed");
    let Some((handed, Made::Rewritten { from, kept })) = made else {
        panic!("row 36 opens on no rewrite");
    };
    assert_eq!(handed, Handed::Path(source.clone()), "not the named file");
    assert!(
        matches!(&from, RewriteFrom::File(path) if *path == result),
        "not the result the command line wrote"
    );
    assert_eq!(kept, Some((0, 2)), "the paragraphs kept are not said");
    let subject = subject.expect("a subject");
    assert_eq!(subject.handed, Handed::Path(source));
    assert!(matches!(
        subject.made,
        Made::Rewritten { from: RewriteFrom::File(ref path), .. } if *path == result
    ));

    // A save in Compare writes over that file (the window's own road, D410)
    // and tells the row: its journal entry says it was edited.
    let told = Told::Saved {
        action: Action::Rewrite,
        home: Home::File(result.clone()),
    };
    assert!(queue.update(cx, |queue, cx| queue.told_by_compare(id, told, cx)));
    flushed(&queue, cx);
    assert!(
        entry_of(&work, entry)
            .outcome
            .and_then(|outcome| outcome.edited)
            .is_some(),
        "the save was not told to row 36"
    );
}

/// D730: Rewrite again pressed in the table on a row the command line made
/// is the person's action, not the caller's: its item carries the person's
/// consent, its result goes where a window's would — over the row's own
/// result, `Row::written()`, the file the command line wrote — and never
/// "to the caller", who is long gone. The row says so; its journal entry,
/// which the bookkeeper ends because no caller is waiting to, says so too;
/// and the table, reading that entry again, still says rewritten. Read "a
/// caller's" off the row's origin in `heard_batch` and the row ends `to:
/// caller`, Compare and Open the result gone; read it off the origin in the
/// bookkeeper's `book` and the journal entry never ends: red either way.
#[gpui::test]
fn rewrite_again_on_a_command_line_row_writes_the_rows_own_result(cx: &mut TestAppContext) {
    const FIRST: &str = "The first rewrite, written by the command line.\n";
    let scratch = Scratch::new("cli-again");
    let source = scratch.file("a10-rewrite.md", PARAGRAPH.as_bytes());
    let result = scratch.file("a10-rewrite.rewritten.md", FIRST.as_bytes());
    let (store, entry) = recorded(Origin::Cli, &row_36(&source, &result));
    let work = work_over(store, Some(swapping()));
    let (queue, preferences, cx) = queue_with(cx, &scratch, Some(work.clone()));
    here_on_duty(&preferences, cx);
    let id = looked_at(&queue, 1, cx)[0];
    // Every result the row says it has, each time the table redraws — the
    // window's own word at the end, before the journal's comes to replace
    // it, included.
    let seen: Rc<RefCell<Vec<Option<Delivered>>>> = Rc::default();
    let seeing = Rc::clone(&seen);
    cx.update(|_, cx| {
        cx.observe(&queue, move |queue, cx| {
            if let Some(Status::Recorded(said)) = queue.read(cx).status_of(id) {
                seeing.borrow_mut().push(said.result.clone());
            }
        })
        .detach();
    });

    queue.update(cx, |queue, cx| queue.rewrite_again(id, cx));
    until(cx, "the rewrite again", |cx| {
        let Some(view) = work.queue.items().into_iter().next() else {
            return false;
        };
        view.state == State::Done
            && cx.update(|_, cx| {
                queue.read(cx).rows.iter().any(|row| {
                    row.id == id
                        && row.item == Some(view.id)
                        && matches!(
                            &row.status,
                            Status::Recorded(said)
                                if said.action == Action::Rewrite && said.phase == Phase::Done
                        )
                })
            })
    });
    let item = work.queue.items()[0].clone();
    assert!(
        item.consent.is_some(),
        "the person's redo was pushed as a caller's"
    );
    assert_eq!(
        item.destination,
        Some(Destination::File(result.clone())),
        "Rewrite again did not name the row's own result"
    );
    let own = Delivered::File {
        path: result.to_string_lossy().into_owned(),
        original: None,
        replaced: false,
    };
    let said = cx.update(|_, cx| match queue.read(cx).status_of(id) {
        Some(Status::Recorded(said)) => said.result.clone(),
        other => panic!("not recorded: {other:?}"),
    });
    assert_eq!(said, Some(own.clone()), "the row's redo went to the caller");
    assert_ne!(
        std::fs::read_to_string(&result).expect("read"),
        FIRST,
        "the row's own result does not hold the new rewrite"
    );

    // The journal: ended by the bookkeeper, to the row's own file — waited
    // for a quarter of a minute, not the minute `until` allows.
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        cx.executor().advance_clock(super::TICK);
        cx.run_until_parked();
        let journaled = work
            .journal
            .rows()
            .into_iter()
            .find(|row| row.id == entry)
            .expect("row 36");
        if journaled.state == "done" && journaled.item == Some(item.id.0) {
            assert_eq!(
                Entry::from_json(&journaled.entry).result,
                Some(own.clone()),
                "the row's journal entry says the redo went to the caller"
            );
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the redo's end never reached row 36's journal entry: {journaled:?}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    // Read back again, the row is what the journal says: rewritten, its
    // result its own file — not a rewrite running for ever.
    cx.executor().advance_clock(super::TICK);
    cx.run_until_parked();
    let read_back = cx.update(|_, cx| {
        let queue = queue.read(cx);
        let row = queue.rows.iter().find(|row| row.id == id).expect("row 36");
        (row.written(), queue.comparable(row))
    });
    assert_eq!(read_back, (Some(result), true));
    assert!(
        !seen.borrow().contains(&Some(Delivered::Caller)),
        "the row said, for a moment, that its redo went to the caller: {:?}",
        seen.borrow()
    );
}

/// A status the journal keeps, for the pure tests.
fn said(action: Action, phase: Phase, result: Option<Delivered>) -> Status {
    Status::Recorded(Box::new(Said {
        action,
        phase,
        outcome: None,
        result,
    }))
}

/// A clean of this session that wrote `written`, or handed back `text`.
fn done(written: Option<&str>, text: Option<&str>) -> Status {
    Status::Done(Arc::new(crate::clean::Outcome {
        verdict: crate::clean::Verdict::Cleaned,
        report: None,
        written: written.map(std::path::PathBuf::from),
        replaced: false,
        set_aside: None,
        kept: None,
        text: text.map(str::to_owned),
        format: None,
        size_in: None,
        size_out: None,
        edited: false,
    }))
}

/// D731: every item that needs the thing or its result says, when greyed,
/// what is missing — in the catalogue's sentence, never who handed the
/// thing over. Open and Compare: nothing of the text kept, the file gone, a
/// text that has no file to open, a picture with no lines to compare; the
/// result's three items: no result yet, the result handed back to whoever
/// asked, a result kept in the row, nothing written, and — for Copy — a
/// result that is a file. Make any of them answer `None` where the item is
/// greyed, or a sentence of another case: red.
#[test]
fn every_greyed_item_says_what_is_missing() {
    use Message as M;
    let scratch = Scratch::new("why");
    let file = scratch.file("x.md", b"A note.\n");
    let picture = scratch.file("x.png", b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR");
    let arrival = |handed: Handed| Arrival {
        intake: wipemark_intake::of(&handed),
        handed,
    };
    let (text, on_disk) = (
        arrival(Handed::Text("A note.".to_owned())),
        arrival(Handed::Path(file)),
    );
    let png = arrival(Handed::Path(picture));
    let said_as = |message: M| Some(t(message));

    assert_eq!(why_not_open(Some(&on_disk), false), None);
    assert_eq!(
        why_not_open(Some(&text), false),
        said_as(M::QueueActionWhyNoFile)
    );
    assert_eq!(why_not_open(None, false), said_as(M::QueueActionWhyNotKept));
    assert_eq!(why_not_open(None, true), said_as(M::QueueActionWhyGone));

    assert_eq!(why_not_compare(Some(&on_disk), true, false), None);
    assert_eq!(why_not_compare(Some(&text), false, false), None);
    assert_eq!(
        why_not_compare(None, false, true),
        None,
        "a rewrite to show"
    );
    assert_eq!(
        why_not_compare(Some(&png), true, false),
        said_as(M::CompareRefusedNotText)
    );
    assert_eq!(
        why_not_compare(None, false, false),
        said_as(M::QueueActionWhyNotKept)
    );
    assert_eq!(
        why_not_compare(None, true, false),
        said_as(M::QueueActionWhyGone)
    );

    let nothing_yet = [
        Status::Waiting,
        Status::Queued,
        Status::Cleaning,
        Status::RewriteQueued,
        Status::Rewriting(Some((1, 2))),
        said(Action::Clean, Phase::Running, None),
    ];
    for status in &nothing_yet {
        assert_eq!(
            why_no_result_file(status),
            t(M::QueueActionWhyNoResult),
            "{status:?}"
        );
        assert_eq!(
            why_not_copy(status),
            t(M::QueueActionWhyNoResult),
            "{status:?}"
        );
    }
    let to_caller = said(Action::Rewrite, Phase::Done, Some(Delivered::Caller));
    assert_eq!(why_no_result_file(&to_caller), t(M::QueueActionWhyToCaller));
    assert_eq!(why_not_copy(&to_caller), t(M::QueueActionWhyToCaller));
    let in_the_row = said(Action::Rewrite, Phase::Done, Some(Delivered::Row));
    assert_eq!(why_no_result_file(&in_the_row), t(M::QueueActionWhyNoFile));
    assert_eq!(why_not_copy(&in_the_row), t(M::QueueActionWhyNotKept));
    let paste = done(None, Some("A note."));
    assert_eq!(why_no_result_file(&paste), t(M::QueueActionWhyNoFile));
    for nothing in [
        done(None, None),
        said(Action::Rewrite, Phase::Done, Some(Delivered::Nowhere)),
        said(Action::Rewrite, Phase::Failed, None),
        said(Action::Rewrite, Phase::Cancelled, Some(Delivered::Nowhere)),
    ] {
        assert_eq!(
            why_no_result_file(&nothing),
            t(M::QueueActionWhyNothingWritten),
            "{nothing:?}"
        );
        assert_eq!(
            why_not_copy(&nothing),
            t(M::QueueActionWhyNothingWritten),
            "{nothing:?}"
        );
    }
    for file in [
        done(Some("/f/x.cleaned.md"), None),
        said(
            Action::Clean,
            Phase::Done,
            Some(Delivered::File {
                path: "/f/x.cleaned.md".to_owned(),
                original: None,
                replaced: false,
            }),
        ),
    ] {
        assert_eq!(
            why_not_copy(&file),
            t(M::QueueActionWhyResultFile),
            "{file:?}"
        );
    }

    // What is missing, never who handed it over.
    for message in [
        M::QueueActionWhyNotKept,
        M::QueueActionWhyGone,
        M::QueueActionWhyNoResult,
        M::QueueActionWhyToCaller,
        M::QueueActionWhyNoFile,
        M::QueueActionWhyNothingWritten,
        M::QueueActionWhyResultFile,
    ] {
        let sentence = t(message).to_lowercase();
        for origin in ["agent", "command line", "panel", "window", " you"] {
            assert!(!sentence.contains(origin), "{sentence}");
        }
    }
}

/// D731, in the window: an agent's text-only row — no path, its text gone
/// back to the agent — and a row whose named file has gone. The Actions
/// menu, opened with a click, draws a reason under every greyed item that
/// needs the thing or its result: under Compare, Open, Open the result,
/// Show the result and Copy the result. The reasons are the catalogue's
/// for what is missing: nothing kept, the file gone, the result back with
/// whoever asked. Grey any of those items with a bare `.disabled(..)` and
/// its line is not drawn: red.
#[gpui::test]
fn a_greyed_compare_says_why_in_the_menu(cx: &mut TestAppContext) {
    let scratch = Scratch::new("why-menu");
    let gone = scratch.0.join("gone.md");
    let (store, _) = recorded(Origin::Agent, &text_only());
    record(
        &store,
        Origin::Cli,
        &Entry {
            said_path: Some(gone.to_string_lossy().into_owned()),
            ..text_only()
        },
    );
    let work = work_over(store, Some(swapping()));
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work));
    cx.simulate_resize(gpui::size(gpui::px(1800.0), gpui::px(900.0)));
    let rows = looked_at(&queue, 2, cx);

    let menu = |id: u64, cx: &mut VisualTestContext| {
        cx.update(|_, cx| queue.read(cx).menu_of(id, cx))
            .expect("a menu")
            .1
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let greyed = |message: Message| Item::Greyed(Some(t(message)));
    for (id, behind) in [
        (rows[0], Message::QueueActionWhyNotKept),
        (rows[1], Message::QueueActionWhyGone),
    ] {
        let menu = menu(id, cx);
        assert_eq!(menu["open"], greyed(behind), "{id}");
        assert_eq!(menu["compare"], greyed(behind), "{id}");
        for key in ["open-result", "reveal-result", "copy-result"] {
            assert_eq!(menu[key], greyed(Message::QueueActionWhyToCaller), "{key}");
        }
    }

    // The menu itself, opened: every one of those reasons is drawn.
    let id = rows[0];
    let actions: &'static str = Box::leak(format!("row-actions-{id}").into_boxed_str());
    let button = cx.debug_bounds(actions).expect("the row's Actions button");
    cx.simulate_click(button.center(), gpui::Modifiers::default());
    cx.run_until_parked();
    for key in [
        "open",
        "compare",
        "open-result",
        "reveal-result",
        "copy-result",
    ] {
        let line: &'static str = Box::leak(format!("queue-why-{key}-{id}").into_boxed_str());
        assert!(
            cx.debug_bounds(line).is_some(),
            "the menu greys {key} and does not say why"
        );
    }
}
