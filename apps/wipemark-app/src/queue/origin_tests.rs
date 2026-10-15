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

/// A note with one zero-width space: what a clean removes.
const MARKED: &str = "# Notes\n\nA zero\u{200B}width space.\n";

/// `tools/call` of `tool` over `text`, with `_meta` when there is one, to
/// the application's own MCP server; the answer's result.
fn call(
    services: &crate::mcp::protocol::Services,
    tool: &str,
    text: &str,
    meta: Option<serde_json::Value>,
) -> serde_json::Value {
    ask(services, tool, serde_json::json!({ "text": text }), meta)
}

/// `tools/call` of `tool` with `arguments`, and `_meta` when there is one,
/// to the application's own MCP server; the answer's result.
fn ask(
    services: &crate::mcp::protocol::Services,
    tool: &str,
    arguments: serde_json::Value,
    meta: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut params = serde_json::json!({ "name": tool, "arguments": arguments });
    if let Some(meta) = meta {
        params["_meta"] = meta;
    }
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": params,
    });
    let answer = crate::mcp::protocol::respond_with(&body.to_string(), services, &|| false)
        .expect("an answer");
    let answer: serde_json::Value = serde_json::from_str(&answer).expect("JSON");
    answer["result"].clone()
}

/// The journal row an answer made.
fn journal_id(result: &serde_json::Value) -> i64 {
    result["_meta"]["wipemark/journal"]
        .as_i64()
        .unwrap_or_else(|| panic!("the answer names no row: {result}"))
}

/// The table's row for journal row `entry`, once it is listed.
fn row_of(queue: &Entity<Queue>, entry: i64, cx: &mut VisualTestContext) -> u64 {
    until(cx, "the row, listed", |cx| {
        queue.read_with(cx, |queue, _| {
            queue.rows.iter().any(|row| row.entry == Some(entry))
        })
    });
    queue.read_with(cx, |queue, _| {
        queue
            .rows
            .iter()
            .find(|row| row.entry == Some(entry))
            .map(|row| row.id)
            .expect("listed")
    })
}

/// Whether row `id` says `word` — `cleaned` for a clean of this session,
/// `rewritten` for a rewrite that ended done.
fn says(queue: &Entity<Queue>, id: u64, word: &str, cx: &mut VisualTestContext) -> bool {
    cx.update(|_, cx| match queue.read(cx).status_of(id) {
        Some(Status::Done(outcome)) => outcome.verdict.id() == word,
        Some(Status::Recorded(said)) => {
            word == "rewritten" && said.action == Action::Rewrite && said.phase == Phase::Done
        }
        _ => false,
    })
}

/// D732, the owner's sentence (2026-10-14): "the behaviour must not break
/// the UI depending on where a thing came from". One file goes down every
/// road that ends as a row of the one list — a drop on the main window
/// (`take_as`, origin window), a drop on the panel (`take_from_panel`), the
/// command line through the application (`_meta` `wipemark/origin=cli`,
/// `wipemark/path`), and an agent naming `wipemark/path` — and once the
/// table has looked at each row's file, rows in the same status offer the
/// same actions, greyed with the same reasons. A note is cleaned down each
/// road and a story rewritten.
///
/// * Rewrite done, its result in a file — the window's, the panel's, and
///   the command line's once it has written its file and told the row, as
///   `wipemark-cli` does: one menu.
/// * Clean done here — the window's and the panel's, then the command
///   line's and the agent's rows after the person's Clean again from the
///   table: one menu.
/// * Done, the result handed back to whoever asked — the command line's
///   clean and the agent's, with a path: one menu. An agent's call with no
///   path is the one row whose document-needing actions — Clean, Rewrite,
///   Open, Compare — are greyed, each with its reason; the rest of its menu
///   is the agent-with-a-path's.
///
/// With D729 undone, the command line's and the agent's rows have nothing
/// behind them, and every comparison but the last is red.
#[gpui::test]
fn where_a_thing_came_from_never_changes_what_its_row_can_do(cx: &mut TestAppContext) {
    let scratch = Scratch::new("every-road");
    let story = scratch.file("story.md", PARAGRAPH.as_bytes());
    let note = scratch.file("note.md", MARKED.as_bytes());
    let work = work_over(
        Arc::new(Store::in_memory().expect("memory")),
        Some(swapping()),
    );
    let services = crate::mcp::protocol::Services {
        rewriter: Some(
            crate::mcp::rewrite::Rewriter::new(work.engine.clone(), None)
                .with_work(Some(work.clone())),
        ),
        work: Some(work.clone()),
    };
    let (queue, preferences, cx) = queue_with(cx, &scratch, Some(work.clone()));
    here_on_duty(&preferences, cx);

    // The window's road and the panel's, each with both files.
    queue.update(cx, |queue, cx| {
        queue.hand(vec![story.clone(), note.clone()], cx)
    });
    cx.run_until_parked();
    let window = cx.update(|_, cx| queue.read(cx).ids());
    let arrival = |path: &Path| Arrival {
        handed: Handed::Path(path.to_path_buf()),
        intake: wipemark_intake::of_path(path),
    };
    let panel = queue.update(cx, |queue, cx| {
        queue.take_from_panel(Arc::from(vec![arrival(&story), arrival(&note)]), cx)
    });
    let ((w_story, w_note), (p_story, p_note)) = ((window[0], window[1]), (panel[0], panel[1]));
    until(cx, "the journal ids", |cx| {
        queue.read_with(cx, |queue, _| {
            queue.rows.iter().all(|row| row.entry.is_some())
        })
    });
    queue.update(cx, |queue, cx| {
        queue.clean(&[w_note, p_note], cx);
        queue.rewrite(&[w_story, p_story], cx);
    });
    until(cx, "the window's and the panel's own", |cx| {
        says(&queue, w_note, "cleaned", cx)
            && says(&queue, p_note, "cleaned", cx)
            && says(&queue, w_story, "rewritten", cx)
            && says(&queue, p_story, "rewritten", cx)
    });

    // The command line's road through the application, and two agents'.
    let named = |path: &Path, cli: bool| {
        let mut meta = serde_json::json!({ "wipemark/path": path.to_string_lossy() });
        if cli {
            meta["wipemark/origin"] = serde_json::json!("cli");
            meta["wipemark/name"] = serde_json::json!(path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned()));
        }
        Some(meta)
    };
    let roads = [
        (named(&story, true), named(&note, true)),
        (named(&story, false), named(&note, false)),
        (None, None),
    ];
    let (answer, answers) = std::sync::mpsc::channel();
    let calling = services.clone();
    std::thread::spawn(move || {
        for (rewrite_meta, clean_meta) in roads {
            let rewritten = call(&calling, "rewrite", PARAGRAPH, rewrite_meta);
            let cleaned = call(&calling, "clean", MARKED, clean_meta);
            let _ = answer.send((rewritten, cleaned));
        }
    });
    let heard: Rc<RefCell<Vec<(serde_json::Value, serde_json::Value)>>> = Rc::default();
    until(cx, "the three callers", |_| {
        heard.borrow_mut().extend(answers.try_iter());
        heard.borrow().len() == 3
    });
    let heard = heard.borrow().clone();
    for (rewritten, cleaned) in &heard {
        assert_ne!(rewritten["isError"], serde_json::json!(true), "{rewritten}");
        assert_ne!(cleaned["isError"], serde_json::json!(true), "{cleaned}");
    }

    // `wipemark-cli` writes the text it was handed back, then tells the
    // application's row where the file went (`delivered_to_app_row`).
    let cli_out = scratch.0.join("story.from-cli.md");
    let cli_text = heard[0].0["structuredContent"]["text"]
        .as_str()
        .expect("the rewritten text");
    std::fs::write(&cli_out, cli_text).expect("the command line writes");
    let cli_entry = journal_id(&heard[0].0);
    let row = work
        .journal
        .rows()
        .into_iter()
        .find(|row| row.id == cli_entry)
        .expect("the command line's row");
    let mut told = Entry::from_json(&row.entry);
    told.result = Some(Delivered::File {
        path: cli_out.to_string_lossy().into_owned(),
        original: None,
        replaced: false,
    });
    work.journal.change(
        cli_entry,
        &wipemark_store::Change {
            action: &row.action,
            state: &row.state,
            item: row.item,
            ended: row.ended,
            entry: &told.to_json(),
        },
    );

    let [(c_story, c_note), (a_story, a_note), (n_story, n_note)] = [0, 1, 2].map(|at| {
        (
            row_of(&queue, journal_id(&heard[at].0), cx),
            row_of(&queue, journal_id(&heard[at].1), cx),
        )
    });
    looked_at(&queue, 10, cx);
    until(cx, "the command line's file, told", |cx| {
        cx.update(|_, cx| {
            let queue = queue.read(cx);
            queue
                .rows
                .iter()
                .find(|row| row.id == c_story)
                .and_then(super::Row::written)
                == Some(cli_out.clone())
        })
    });

    let menu = |id: u64, cx: &mut VisualTestContext| {
        cx.update(|_, cx| queue.read(cx).menu_of(id, cx))
            .expect("a menu")
    };
    let live = |menu: &super::Menu, keys: &[&str]| {
        for key in keys {
            assert!(
                menu.1.contains(&(*key, Item::Live)),
                "the window's own row greys {key}: {menu:?}"
            );
        }
    };

    // Rewrite done, its result in a file.
    let window_story = menu(w_story, cx);
    live(
        &window_story,
        &[
            "rewrite",
            "open",
            "compare",
            "open-result",
            "reveal-result",
            "copy-result",
            "remove",
        ],
    );
    assert_eq!(menu(p_story, cx), window_story, "the panel's rewrite");
    assert_eq!(
        menu(c_story, cx),
        window_story,
        "the command line's rewrite"
    );

    // Done, the result back with whoever asked: the command line's clean
    // and the agent's, with a path; the agent's call with none differs in
    // the document-needing items alone, each greyed saying why.
    let agent_clean = menu(a_note, cx);
    assert_eq!(menu(c_note, cx), agent_clean, "the command line's clean");
    let needs_the_document = ["clean", "rewrite", "open", "compare"];
    for (with, without) in [
        (menu(a_story, cx), menu(n_story, cx)),
        (agent_clean.clone(), menu(n_note, cx)),
    ] {
        assert_eq!(with.0, without.0, "the labels");
        for ((key, with), (same, without)) in with.1.iter().zip(&without.1) {
            assert_eq!(key, same);
            if needs_the_document.contains(key) {
                assert_eq!(*with, Item::Live, "{key}, with a path");
                let why = match *key {
                    "clean" | "rewrite" => Message::QueueActionNotKept,
                    _ => Message::QueueActionWhyNotKept,
                };
                assert_eq!(*without, Item::Greyed(Some(t(why))), "{key}, no path");
            } else {
                assert_eq!(with, without, "{key}");
            }
        }
    }
    // An agent's rewrite with a path: what needs the document is the
    // window's; what needs a result on disk says the result went back.
    let agent_story = menu(a_story, cx);
    for key in ["rewrite", "cancel", "open", "compare", "report", "remove"] {
        let item = |menu: &super::Menu| {
            menu.1
                .iter()
                .find(|(named, _)| named == &key)
                .map(|(_, item)| item.clone())
        };
        assert_eq!(item(&agent_story), item(&window_story), "{key}");
    }

    // Clean done here: the window's and the panel's; then the command
    // line's and the agent's rows, cleaned again by the person.
    let window_note = menu(w_note, cx);
    live(
        &window_note,
        &[
            "clean",
            "rewrite",
            "open",
            "compare",
            "open-result",
            "reveal-result",
            "report",
            "remove",
        ],
    );
    assert_eq!(menu(p_note, cx), window_note, "the panel's clean");
    queue.update(cx, |queue, cx| {
        queue.clean_again(c_note, cx);
        queue.clean_again(a_note, cx);
    });
    until(cx, "the person's Clean again", |cx| {
        says(&queue, c_note, "cleaned", cx) && says(&queue, a_note, "cleaned", cx)
    });
    assert_eq!(
        menu(c_note, cx),
        window_note,
        "the command line's row, cleaned here"
    );
    assert_eq!(
        menu(a_note, cx),
        window_note,
        "the agent's row, cleaned here"
    );
}

/// A picture: the application's own 32 × 32 icon, a PNG.
const PNG: &[u8] = include_bytes!("../../assets/icon/32x32.png");

/// D750, F28 (the owner's rows 37 and 38 at the macOS acceptance,
/// 2026-10-15): `cli-long.md` imported in the window read "Markdown, UTF-8"
/// under Format, and the same file rewritten by `wipemark-cli` through the
/// application read "—" — a caller's journal entry had no format and no
/// encoding. How a row looks depends on what was handed over, never on who
/// handed it over. Each thing goes down every road that ends as a row:
///
/// * a Markdown file in UTF-8 and one in UTF-16 — the window's import, the
///   panel's drop, the command line's rewrite through the application
///   (`_meta` `wipemark/origin=cli`, `wipemark/name`, `wipemark/path`,
///   `wipemark/size`, the decoded text sent as the command line sends it)
///   and an agent's clean naming `wipemark/path`;
/// * a PNG — the window's import, the panel's drop and an agent's
///   `clean_image` naming `wipemark/path`;
/// * a text and a picture with no file behind them — the window's paste of
///   each and an agent's call handing each over inline (D751).
///
/// Every row of one thing shows one look — the Kind, the Format cell with
/// its encoding, the Size, the name and the folder — and the callers'
/// journal entries carry the format and the encoding the window's own
/// entries do. Record a caller's row with no look at what it was handed, as
/// before, and every caller's Format cell is "—": red. Look at the text the
/// caller sent rather than the file it named, and the UTF-16 file reads
/// "Text, UTF-8" down the callers' roads: red.
#[gpui::test]
fn where_a_thing_came_from_never_changes_how_its_row_looks(cx: &mut TestAppContext) {
    use base64::Engine as _;

    let scratch = Scratch::new("every-look");
    let md = scratch.file("cli-long.md", PARAGRAPH.as_bytes());
    let wide_bytes: Vec<u8> = [0xFF_u8, 0xFE]
        .into_iter()
        .chain(PARAGRAPH.encode_utf16().flat_map(|unit| unit.to_le_bytes()))
        .collect();
    let wide = scratch.file("wide.md", &wide_bytes);
    let png = scratch.file("shot.png", PNG);
    let work = work_over(
        Arc::new(Store::in_memory().expect("memory")),
        Some(swapping()),
    );
    let services = crate::mcp::protocol::Services {
        rewriter: Some(
            crate::mcp::rewrite::Rewriter::new(work.engine.clone(), None)
                .with_work(Some(work.clone())),
        ),
        work: Some(work.clone()),
    };
    let (queue, preferences, cx) = queue_with(cx, &scratch, Some(work.clone()));
    here_on_duty(&preferences, cx);

    // The window's import of the three files, then its paste of a text and
    // of a picture; the panel's drop of the three files.
    queue.update(cx, |queue, cx| {
        queue.hand(vec![md.clone(), wide.clone(), png.clone()], cx)
    });
    cx.run_until_parked();
    queue.update(cx, |queue, cx| {
        queue.land(
            vec![
                Handed::Text(MARKED.to_owned()),
                Handed::Bytes {
                    name: None,
                    bytes: PNG.to_vec(),
                },
            ],
            cx,
        )
    });
    cx.run_until_parked();
    let window = cx.update(|_, cx| queue.read(cx).ids());
    assert_eq!(window.len(), 5, "the window's own rows");
    let arrival = |path: &Path| Arrival {
        handed: Handed::Path(path.to_path_buf()),
        intake: wipemark_intake::of_path(path),
    };
    let panel = queue.update(cx, |queue, cx| {
        queue.take_from_panel(
            Arc::from(vec![arrival(&md), arrival(&wide), arrival(&png)]),
            cx,
        )
    });

    // The callers, each on the MCP server's road.
    let cli = |path: &Path, text: &str| {
        Some(serde_json::json!({
            "wipemark/origin": "cli",
            "wipemark/name": path.file_name().map(|name| name.to_string_lossy().into_owned()),
            "wipemark/path": path.to_string_lossy(),
            "wipemark/size": text.len(),
        }))
    };
    let agent = |path: &Path| Some(serde_json::json!({ "wipemark/path": path.to_string_lossy() }));
    let text = |text: &str| serde_json::json!({ "text": text });
    let data = serde_json::json!({
        "data": base64::engine::general_purpose::STANDARD.encode(PNG),
    });
    let calls = vec![
        ("rewrite", text(PARAGRAPH), cli(&md, PARAGRAPH)),
        ("rewrite", text(PARAGRAPH), cli(&wide, PARAGRAPH)),
        ("clean", text(PARAGRAPH), agent(&md)),
        ("clean", text(PARAGRAPH), agent(&wide)),
        ("clean_image", data.clone(), agent(&png)),
        ("clean", text(MARKED), None),
        ("clean_image", data, None),
    ];
    let expected = calls.len();
    let (answer, answers) = std::sync::mpsc::channel();
    let calling = services.clone();
    std::thread::spawn(move || {
        for (tool, arguments, meta) in calls {
            let _ = answer.send(ask(&calling, tool, arguments, meta));
        }
    });
    let heard: Rc<RefCell<Vec<serde_json::Value>>> = Rc::default();
    until(cx, "the callers", |_| {
        heard.borrow_mut().extend(answers.try_iter());
        heard.borrow().len() == expected
    });
    let heard = heard.borrow().clone();
    for (at, result) in heard.iter().enumerate().take(2) {
        assert_ne!(result["isError"], serde_json::json!(true), "{at}: {result}");
    }
    let callers: Vec<u64> = heard
        .iter()
        .map(|result| row_of(&queue, journal_id(result), cx))
        .collect();
    looked_at(&queue, window.len() + panel.len() + callers.len(), cx);
    flushed(&queue, cx);

    let look = |id: u64, cx: &mut VisualTestContext| {
        cx.update(|_, cx| {
            queue
                .read(cx)
                .rows
                .iter()
                .find(|row| row.id == id)
                .map(|row| row.look.clone())
                .expect("the row")
        })
    };
    // What the journal keeps of a row's look.
    let kept = |id: u64, cx: &mut VisualTestContext| {
        let entry = cx
            .update(|_, cx| {
                queue
                    .read(cx)
                    .rows
                    .iter()
                    .find(|row| row.id == id)
                    .and_then(|row| row.entry)
            })
            .expect("a journal row");
        let entry = entry_of(&work, entry);
        (entry.kind, entry.format, entry.encoding, entry.size)
    };
    let things = [
        (
            "the UTF-8 Markdown file",
            window[0],
            vec![panel[0], callers[0], callers[2]],
        ),
        (
            "the UTF-16 Markdown file",
            window[1],
            vec![panel[1], callers[1], callers[3]],
        ),
        ("the PNG file", window[2], vec![panel[2], callers[4]]),
        ("the pasted text", window[3], vec![callers[5]]),
        ("the pasted picture", window[4], vec![callers[6]]),
    ];
    for (thing, own, others) in &things {
        let (own_look, own_kept) = (look(*own, cx), kept(*own, cx));
        for other in others {
            assert_eq!(look(*other, cx), own_look, "{thing}: the row's look");
            assert_eq!(kept(*other, cx), own_kept, "{thing}: the journal's entry");
        }
    }
    // What the window says of each, as the owner read row 37.
    let cell = |id: u64, cx: &mut VisualTestContext| {
        let look = look(id, cx);
        (look.format, look.encoding)
    };
    let said = |format: &str, encoding: Option<&str>| {
        (Some(format.to_owned()), encoding.map(str::to_owned))
    };
    assert_eq!(cell(callers[0], cx), said("Markdown", Some("UTF-8")));
    assert_eq!(cell(callers[1], cx), said("Markdown", Some("UTF-16LE")));
    assert_eq!(cell(callers[4], cx), said("PNG", None));
    assert_eq!(cell(callers[5], cx), said("Text", Some("UTF-8")));
    assert_eq!(cell(callers[6], cx), said("PNG", None));
}

/// D731, for Clean and Rewrite too: on a row with nothing behind it, the
/// two say what is missing as Open and Compare do — "The file is no longer
/// where it was" for a row that names a file that went, and D269's "not
/// kept" only for a row that never had one. Pure, and in the menu of a
/// command line's row whose named file was moved away beside an agent's
/// text-only row. Word "nothing behind" without `Row::named`, as D269 did,
/// and the moved file's Clean and Rewrite say it never had one: red.
#[gpui::test]
fn clean_and_rewrite_on_a_gone_file_say_it_is_gone(cx: &mut TestAppContext) {
    use super::{clean_greyed, rewrite_greyed};
    let gone_why = Some(t(Message::QueueActionWhyGone));
    let never_why = Some(t(Message::QueueActionNotKept));
    for status in [
        Status::Waiting,
        said(Action::Clean, Phase::Done, Some(Delivered::Caller)),
        said(Action::Rewrite, Phase::Done, Some(Delivered::Caller)),
        said(Action::Rewrite, Phase::Failed, None),
    ] {
        assert_eq!(clean_greyed(&status, None, true), gone_why, "{status:?}");
        assert_eq!(clean_greyed(&status, None, false), never_why, "{status:?}");
        assert_eq!(
            rewrite_greyed(&status, None, None, true),
            gone_why,
            "{status:?}"
        );
        assert_eq!(
            rewrite_greyed(&status, None, None, false),
            never_why,
            "{status:?}"
        );
    }

    let scratch = Scratch::new("gone-clean");
    let gone = scratch.0.join("moved-away.md");
    let (store, agents) = recorded(Origin::Agent, &text_only());
    let commands = record(
        &store,
        Origin::Cli,
        &Entry {
            said_path: Some(gone.to_string_lossy().into_owned()),
            ..text_only()
        },
    );
    let work = work_over(store, Some(swapping()));
    let (queue, preferences, cx) = queue_with(cx, &scratch, Some(work));
    here_on_duty(&preferences, cx);
    looked_at(&queue, 2, cx);
    let (never, moved) = (row_of(&queue, agents, cx), row_of(&queue, commands, cx));
    for key in ["clean", "rewrite"] {
        assert_eq!(
            item_of(&queue, moved, key, cx),
            Item::Greyed(gone_why.clone()),
            "{key} on a row whose file went"
        );
        assert_eq!(
            item_of(&queue, never, key, cx),
            Item::Greyed(never_why.clone()),
            "{key} on a row that never had a file"
        );
    }
}

/// What row `id`'s Actions menu says of `key`.
fn item_of(queue: &Entity<Queue>, id: u64, key: &str, cx: &mut VisualTestContext) -> Item {
    cx.update(|_, cx| queue.read(cx).menu_of(id, cx))
        .expect("a menu")
        .1
        .into_iter()
        .find(|(named, _)| *named == key)
        .map(|(_, item)| item)
        .expect("the item")
}

/// D730 with D355: Remove waits for a caller, never for the person. The
/// command line's call through the application, its item queued with no
/// consent of the window's — as the MCP server queues it — keeps Remove
/// greyed with D355's reason, and a removal is refused. Rewrite again the
/// person presses on row 36, a command line's row too, is queued with the
/// person's consent: Remove is live on it from the press, as on a window
/// row's own rewrite, and removes it with its item. Read "a caller waits"
/// off the bare origin in `why_not_remove`, as before, and row 36's Remove
/// is greyed saying an agent or the command line is waiting: red.
#[gpui::test]
fn remove_waits_for_a_caller_and_never_for_the_persons_redo(cx: &mut TestAppContext) {
    let scratch = Scratch::new("remove-waits");
    let source = scratch.file("a10-rewrite.md", PARAGRAPH.as_bytes());
    let result = scratch.file("a10-rewrite.rewritten.md", b"The first rewrite.\n");
    let other = scratch.file("b.md", PARAGRAPH.as_bytes());
    let (store, row36) = recorded(Origin::Cli, &row_36(&source, &result));
    let work = work_over(store, Some(swapping()));
    work.queue.pause();
    // A caller's call, queued as `mcp::rewrite` queues it: the row names the
    // item before the push, and the item carries no window's consent.
    let request = wipemark_queue::Request {
        source: wipemark_queue::Source::Text(PARAGRAPH.to_owned()),
        format: wipemark_pipeline::prepare::TextFormat::Plain,
        destination: Destination::Row,
        options: wipemark_pipeline::Options::for_executor(
            wipemark_pipeline::cost::Executor::LocalCpu,
        ),
    };
    let item = work.queue.reserve(&request).expect("reserved");
    let caller = work
        .journal
        .record(&NewRow {
            origin: Origin::Cli.as_str(),
            action: Action::Rewrite.as_str(),
            state: Phase::Queued.as_str(),
            item: Some(item.0),
            arrived: 1,
            ended: None,
            entry: &Entry {
                said_path: Some(other.to_string_lossy().into_owned()),
                kind: Some("text".to_owned()),
                ..Entry::default()
            }
            .to_json(),
        })
        .expect("the caller's row");
    work.queue
        .push_reserved(item, request, None)
        .expect("pushed");
    let (queue, preferences, cx) = queue_with(cx, &scratch, Some(work.clone()));
    here_on_duty(&preferences, cx);
    looked_at(&queue, 2, cx);
    let (waited, ours) = (row_of(&queue, caller, cx), row_of(&queue, row36, cx));

    // D355 stands for the caller's own call.
    assert_eq!(
        item_of(&queue, waited, "remove", cx),
        Item::Greyed(Some(t(Message::QueueActionRemoveWaited))),
        "a caller's waiting call can be removed"
    );
    queue.update(cx, |queue, cx| queue.remove(waited, cx));
    assert!(
        cx.update(|_, cx| queue.read(cx).status_of(waited).is_some()),
        "a caller's waiting call was removed"
    );

    // The person's redo on a command line's row: nobody waits.
    queue.update(cx, |queue, cx| queue.rewrite_again(ours, cx));
    assert!(matches!(
        cx.update(|_, cx| queue.read(cx).status_of(ours).cloned()),
        Some(Status::RewriteQueued)
    ));
    assert_eq!(
        item_of(&queue, ours, "remove", cx),
        Item::Live,
        "Remove on the person's redo waits for a caller, at the press"
    );
    until(cx, "the push", |_| work.queue.items().len() == 2);
    assert_eq!(
        item_of(&queue, ours, "remove", cx),
        Item::Live,
        "Remove on the person's redo waits for a caller, once pushed"
    );
    queue.update(cx, |queue, cx| queue.remove(ours, cx));
    assert!(
        cx.update(|_, cx| queue.read(cx).status_of(ours).is_none()),
        "the person's redo was not removed"
    );
    until(cx, "its item, gone", |_| work.queue.items().len() == 1);
}
