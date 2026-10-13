//! Redo a row (D703–D708): Clean again and Rewrite again, over the row's own
//! result. The labels and their greying without a window; then the table
//! over the real line of cleans and a real batch queue on `FakeEngine`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{Entity, TestAppContext, VisualTestContext};
use wipemark_i18n::{args, t, t_args, Message};
use wipemark_intake::name::with_tag;
use wipemark_intake::Handed;
use wipemark_queue::{Destination, ItemId, State};
use wipemark_store::entry::{Action, Delivered, Entry, Phase};

use super::rewrite_tests::{here_on_duty, ids, status, swapping, until, work, PARAGRAPH};
use super::tests::{queue_with, statuses, Scratch};
use super::{
    again_of, labels, why_not_clean, why_not_rewrite, Again, Queue, QueueEvent, Said, Status,
};
use crate::clean::{Cleanable, Outcome, Verdict};
use crate::compare::{Home, Told};
use crate::journal::{self, Work};
use crate::retention::Destination as Goes;

/// A note with one zero-width space: what a clean removes.
const MARKED: &str = "# Notes\n\nA zero\u{200B}width space.\n";

/// The same note, changed since its first clean: its clean is another text.
const MARKED_AGAIN: &str = "# Notes\n\nA zero\u{200B}width space, and one\u{200B} more.\n";

/// [`PARAGRAPH`] with one word changed: its rewrite is another text.
fn paragraph_again() -> String {
    PARAGRAPH.replacen("twelve", "eleven", 1)
}

fn cleaned(text: &str) -> String {
    wipemark_core::clean(text, &wipemark_core::Options::default()).text
}

fn read(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).expect("read")
}

/// The names in `scratch` holding `infix`, sorted.
fn names(scratch: &Scratch, infix: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(&scratch.0)
        .expect("list")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.contains(infix))
        .collect();
    names.sort();
    names
}

/// `name`'s result named by row `entry` (D635).
fn own(scratch: &Scratch, name: &str, infix: &str, entry: i64) -> PathBuf {
    scratch.0.join(with_tag(name, infix, &entry.to_string()))
}

/// Wait until every row has its journal id — what its own name is made of.
fn named(queue: &Entity<Queue>, cx: &mut VisualTestContext) {
    until(cx, "the journal ids", |cx| {
        queue.read_with(cx, |queue, _| {
            queue.rows.iter().all(|row| row.entry.is_some())
        })
    });
}

fn entry(queue: &Entity<Queue>, id: u64, cx: &mut VisualTestContext) -> i64 {
    cx.update(|_, cx| {
        queue
            .read(cx)
            .rows
            .iter()
            .find(|row| row.id == id)
            .and_then(|row| row.entry)
    })
    .expect("a journal id")
}

/// Row `id`'s own result, as the row names it.
fn written(queue: &Entity<Queue>, id: u64, cx: &mut VisualTestContext) -> Option<PathBuf> {
    cx.update(|_, cx| {
        queue
            .read(cx)
            .rows
            .iter()
            .find(|row| row.id == id)
            .and_then(super::Row::written)
    })
}

/// Whether row `id` says its result was edited by hand (D447).
fn edited(queue: &Entity<Queue>, id: u64, cx: &mut VisualTestContext) -> bool {
    cx.update(|_, cx| {
        queue
            .read(cx)
            .rows
            .iter()
            .find(|row| row.id == id)
            .is_some_and(super::Row::edited)
    })
}

/// Row `id`'s clean, this session's.
fn outcome(queue: &Entity<Queue>, id: u64, cx: &mut VisualTestContext) -> Arc<Outcome> {
    cx.update(|_, cx| match queue.read(cx).status_of(id) {
        Some(Status::Done(outcome)) => Arc::clone(outcome),
        other => panic!("not cleaned: {other:?}"),
    })
}

/// Every Again question the table puts, as the shell would hear it.
fn questions(queue: &Entity<Queue>, cx: &mut VisualTestContext) -> Arc<Mutex<Vec<(u64, Again)>>> {
    let heard: Arc<Mutex<Vec<(u64, Again)>>> = Arc::default();
    let hearing = Arc::clone(&heard);
    cx.update(|_, cx| {
        cx.subscribe(queue, move |_, event: &QueueEvent, _| {
            if let QueueEvent::Again { id, action } = event {
                hearing.lock().expect("lock").push((*id, *action));
            }
        })
        .detach();
    });
    heard
}

/// Everything asked of the table's journal writer so far, written.
fn flushed(queue: &Entity<Queue>, cx: &mut VisualTestContext) {
    let done = cx.update(|_, cx| queue.read(cx).writer.as_ref().map(journal::Writer::flushed));
    done.expect("a writer")
        .recv_timeout(Duration::from_secs(10))
        .expect("flushed");
}

/// The edit mark of journal row `entry`'s outcome (D417).
fn journal_edited(work: &Work, entry: i64) -> Option<i64> {
    work.journal
        .rows()
        .into_iter()
        .find(|row| row.id == entry)
        .and_then(|row| Entry::from_json(&row.entry).outcome)
        .and_then(|outcome| outcome.edited)
}

/// The batch queue's items, by id.
fn items(work: &Work) -> Vec<ItemId> {
    work.queue.items().iter().map(|view| view.id).collect()
}

/// Wait until row `id`'s new item — one not among `before` — has ended done,
/// and the row says so: a Rewrite again run through.
fn again_done(
    queue: &Entity<Queue>,
    work: &Work,
    before: &[ItemId],
    id: u64,
    cx: &mut VisualTestContext,
) {
    until(cx, "the rewrite again", |cx| {
        let Some(item) = work
            .queue
            .items()
            .into_iter()
            .find(|view| !before.contains(&view.id) && view.state == State::Done)
        else {
            return false;
        };
        cx.update(|_, cx| {
            queue
                .read(cx)
                .rows
                .iter()
                .find(|row| row.id == id)
                .is_some_and(|row| {
                    row.item == Some(item.id)
                        && matches!(
                            &row.status,
                            Status::Recorded(said)
                                if said.action == Action::Rewrite && said.phase == Phase::Done
                        )
                })
        })
    });
}

/// The batch queue's item not among `before`.
fn new_item(work: &Work, before: &[ItemId]) -> wipemark_queue::ItemView {
    work.queue
        .items()
        .into_iter()
        .find(|view| !before.contains(&view.id))
        .expect("a new item")
}

fn said(action: Action, phase: Phase, result: Option<Delivered>) -> Status {
    Status::Recorded(Box::new(Said {
        action,
        phase,
        outcome: None,
        result,
    }))
}

fn done(written: Option<&str>, set_aside: Option<&str>) -> Status {
    Status::Done(Arc::new(Outcome {
        verdict: Verdict::Cleaned,
        report: None,
        written: written.map(PathBuf::from),
        replaced: false,
        set_aside: set_aside.map(PathBuf::from),
        kept: None,
        text: None,
        format: None,
        size_in: None,
        size_out: None,
        edited: false,
    }))
}

/// A result the journal says went to a file — in place when `original`
/// was set aside.
fn file(path: &str, original: Option<&str>) -> Delivered {
    Delivered::File {
        path: path.to_owned(),
        original: original.map(str::to_owned),
        replaced: false,
    }
}

const TEXT: Option<Cleanable> = Some(Cleanable::Text(wipemark_intake::Encoding::Utf8));

// -- D703, D706: the labels and their greying, pure ----------------------

/// D703: what a row's Clean and Rewrite are called, and when each is greyed,
/// is one pure rule over the status. Waiting: Clean, Rewrite. Cleaned: Clean
/// again, and a first Rewrite (D702). Rewritten: Clean greyed with the
/// rewritten reason, Rewrite again. A failed or cancelled rewrite: Rewrite
/// again, which retries it. A paste read back after a restart: not kept, as
/// today. An inspected row and a row in a line: as before. Make `again_of`
/// answer `None` and every "again" label goes; give the rewritten row
/// `queue-action-clean-done` back and its reason is wrong: red.
#[test]
fn the_labels_and_their_greying_are_one_rule_of_the_status() {
    use Message as M;

    assert_eq!(t(M::QueueActionCleanAgain), "Clean again");
    assert_eq!(t(M::QueueActionRewriteAgain), "Rewrite again");

    assert_eq!(
        labels(&Status::Waiting),
        (M::QueueActionClean, M::QueueActionRewrite)
    );
    assert_eq!(why_not_clean(&Status::Waiting, TEXT), None);
    assert_eq!(why_not_rewrite(&Status::Waiting, TEXT, None), None);

    let cleaned = done(Some("/f/x.cleaned.md"), None);
    assert_eq!(
        labels(&cleaned),
        (M::QueueActionCleanAgain, M::QueueActionRewrite),
        "a cleaned row's Rewrite is a first rewrite"
    );
    assert_eq!(why_not_clean(&cleaned, TEXT), None, "Clean again is greyed");
    assert_eq!(why_not_rewrite(&cleaned, TEXT, None), None);
    // A clean of an earlier session or another surface, of a text or a
    // picture, however it ended: Clean again.
    for action in [Action::Clean, Action::CleanImage] {
        for phase in [Phase::Done, Phase::Failed] {
            let status = said(action, phase, None);
            assert_eq!(
                again_of(&status),
                Some(Again::Clean),
                "{action:?} {phase:?}"
            );
            assert_eq!(why_not_clean(&status, TEXT), None, "{action:?} {phase:?}");
        }
    }
    // A clean that refused is cleaned again too, greyed only for what
    // cannot be cleaned at all.
    let tiff = Cleanable::No(crate::clean::Unable::NotYet(wipemark_intake::Format::Tiff));
    assert!(why_not_clean(&cleaned, Some(tiff)).is_some_and(|why| why.contains("TIFF")));

    let rewritten = said(
        Action::Rewrite,
        Phase::Done,
        Some(file("/f/x.rewritten.md", None)),
    );
    assert_eq!(
        labels(&rewritten),
        (M::QueueActionClean, M::QueueActionRewriteAgain)
    );
    assert_eq!(
        why_not_clean(&rewritten, TEXT),
        Some(t(M::QueueActionCleanRewritten)),
        "a rewritten row's Clean does not say why it is greyed"
    );
    assert_eq!(why_not_rewrite(&rewritten, TEXT, None), None);
    // The duty's vacancy still greys it, as it greys every Rewrite.
    assert_eq!(
        why_not_rewrite(&rewritten, TEXT, Some("nobody".to_owned())),
        Some("nobody".to_owned())
    );

    for phase in [Phase::Failed, Phase::Cancelled] {
        let status = said(Action::Rewrite, phase, None);
        assert_eq!(
            labels(&status),
            (M::QueueActionClean, M::QueueActionRewriteAgain),
            "{phase:?}"
        );
        assert_eq!(why_not_rewrite(&status, TEXT, None), None, "{phase:?}");
        assert_eq!(
            why_not_clean(&status, TEXT),
            Some(t(M::QueueActionCleanRewritten)),
            "{phase:?}"
        );
    }

    // A paste read back after a restart: nothing behind it.
    let not_kept = Some(t(M::QueueActionNotKept));
    let pasted_clean = said(Action::Clean, Phase::Done, Some(Delivered::Caller));
    assert_eq!(labels(&pasted_clean).0, M::QueueActionCleanAgain);
    assert_eq!(why_not_clean(&pasted_clean, None), not_kept);
    assert_eq!(why_not_rewrite(&pasted_clean, None, None), not_kept);
    let pasted_rewrite = said(Action::Rewrite, Phase::Done, Some(Delivered::Row));
    assert_eq!(labels(&pasted_rewrite).1, M::QueueActionRewriteAgain);
    assert_eq!(why_not_rewrite(&pasted_rewrite, None, None), not_kept);

    let inspected = said(Action::Inspect, Phase::Done, None);
    assert_eq!(
        labels(&inspected),
        (M::QueueActionClean, M::QueueActionRewrite)
    );
    assert_eq!(
        why_not_clean(&inspected, TEXT),
        Some(t(M::QueueActionCleanDone))
    );

    for status in [
        Status::Queued,
        Status::Cleaning,
        Status::RewriteQueued,
        Status::Rewriting(None),
        said(Action::Rewrite, Phase::Running, None),
        said(Action::Clean, Phase::Running, None),
    ] {
        assert_eq!(again_of(&status), None, "{status:?}");
        assert!(why_not_clean(&status, TEXT).is_some(), "{status:?}");
        assert!(why_not_rewrite(&status, TEXT, None).is_some(), "{status:?}");
    }
}

/// D706: a result written in place is not redone — the source's name holds
/// the result — and the item says so, naming the file the original was set
/// aside as: a clean of this session, one the journal kept, and a rewrite.
/// Greyed, never absent. A result written beside is redone. Take the
/// in-place check out of `why_not_clean` or `why_not_rewrite`: red.
#[test]
fn a_result_written_in_place_is_not_redone() {
    let reason = t_args(
        Message::QueueActionAgainInPlace,
        &args!("name" => "a.original.md"),
    );
    assert!(reason.contains("a.original.md"), "{reason}");

    let cleaned = done(Some("/f/a.md"), Some("/f/a.original.md"));
    assert_eq!(labels(&cleaned).0, Message::QueueActionCleanAgain);
    assert_eq!(why_not_clean(&cleaned, TEXT), Some(reason.clone()));
    let recorded = said(
        Action::Clean,
        Phase::Done,
        Some(file("/f/a.md", Some("/f/a.original.md"))),
    );
    assert_eq!(why_not_clean(&recorded, TEXT), Some(reason.clone()));

    let rewritten = said(
        Action::Rewrite,
        Phase::Done,
        Some(file("/f/a.md", Some("/f/a.original.md"))),
    );
    assert_eq!(labels(&rewritten).1, Message::QueueActionRewriteAgain);
    assert_eq!(why_not_rewrite(&rewritten, TEXT, None), Some(reason));

    // Beside, the same rows are redone.
    assert_eq!(
        why_not_clean(&done(Some("/f/a.cleaned.md"), None), TEXT),
        None
    );
    let beside = said(
        Action::Rewrite,
        Phase::Done,
        Some(file("/f/a.rewritten.md", None)),
    );
    assert_eq!(why_not_rewrite(&beside, TEXT, None), None);
}

/// D705: the question says which redo replaces the edits — its body is one
/// key selecting on the action's id, so a variant misspelt falls back to
/// the clean's sentence in silence. Every language's own words, both ways.
#[test]
fn the_question_says_which_redo_replaces_the_edits() {
    use wipemark_i18n::{available_languages, Localizer, Rendering};
    for language in available_languages() {
        let localizer =
            Localizer::for_languages(std::slice::from_ref(&language.id), Rendering::PlainText);
        let said = |action: Again| {
            localizer.format_args(
                Message::QueueAgainEditedBody,
                &args!("action" => action.id()),
            )
        };
        let (clean, rewrite) = (said(Again::Clean), said(Again::Rewrite));
        assert_ne!(clean, rewrite, "{}: one sentence for both", language.id);
    }
    let rewrite = t_args(
        Message::QueueAgainEditedBody,
        &args!("action" => Again::Rewrite.id()),
    );
    assert!(rewrite.contains("Rewriting it again"), "{rewrite}");
    let title = t_args(Message::QueueAgainEditedTitle, &args!("name" => "x.md"));
    assert!(title.contains("x.md"), "{title}");
    assert_eq!(t(Message::QueueAgainEditedGo), "Redo");
}

// -- D704: over the row's own result --------------------------------------

/// Test 1: Clean again writes over the row's own result and makes no new
/// file. The source changed since — another invisible — and `x.cleaned.md`
/// holds the new clean; nothing named `x.cleaned-<id>.md` appears; the row
/// says it replaced its own result. Hand the line's clean `clean_one` in
/// place of `clean_over` (the redo's `replacing` dropped) and the row's own
/// name is taken, so the clean writes `x.cleaned-<id>.md`: red.
#[gpui::test]
fn clean_again_writes_over_the_rows_own_result(cx: &mut TestAppContext) {
    let scratch = Scratch::new("again-clean");
    let source = scratch.file("x.md", MARKED.as_bytes());
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work(swapping())));
    queue.update(cx, |queue, cx| queue.hand(vec![source.clone()], cx));
    named(&queue, cx);
    let id = ids(&queue, cx)[0];
    queue.update(cx, |queue, cx| queue.clean(&[id], cx));
    cx.run_until_parked();
    let own = scratch.0.join("x.cleaned.md");
    assert_eq!(written(&queue, id, cx), Some(own.clone()));
    assert_eq!(read(&own), cleaned(MARKED));

    std::fs::write(&source, MARKED_AGAIN).expect("the source changes");
    queue.update(cx, |queue, cx| queue.clean_again(id, cx));
    cx.run_until_parked();
    assert_eq!(statuses(&queue, cx), ["cleaned"]);
    assert_eq!(written(&queue, id, cx), Some(own.clone()));
    assert_eq!(
        read(&own),
        cleaned(MARKED_AGAIN),
        "the row's own result does not hold the new clean"
    );
    assert!(outcome(&queue, id, cx).replaced, "not said as replaced");
    assert_eq!(
        names(&scratch, ".cleaned"),
        ["x.cleaned.md"],
        "Clean again made a new file"
    );
}

/// Test 1, the row whose own result is `x.cleaned-<id>.md` because a first
/// row of the same file took the plain name: Clean again replaces its
/// `-<id>` file, leaves the first row's plain file byte for byte, and makes
/// no `-<id>-2`. With `replacing` dropped: `x.cleaned-<id>-2.md`, red.
#[gpui::test]
fn clean_again_of_a_row_named_by_its_id_replaces_that_file_and_no_other(cx: &mut TestAppContext) {
    let scratch = Scratch::new("again-clean-own");
    let source = scratch.file("x.md", MARKED.as_bytes());
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work(swapping())));
    queue.update(cx, |queue, cx| {
        queue.hand(vec![source.clone(), source.clone()], cx)
    });
    named(&queue, cx);
    let rows = ids(&queue, cx);
    queue.update(cx, |queue, cx| queue.clean(&rows, cx));
    cx.run_until_parked();
    let plain = scratch.0.join("x.cleaned.md");
    let tagged = own(&scratch, "x.md", "cleaned", entry(&queue, rows[1], cx));
    assert_eq!(written(&queue, rows[0], cx), Some(plain.clone()));
    assert_eq!(written(&queue, rows[1], cx), Some(tagged.clone()));

    std::fs::write(&source, MARKED_AGAIN).expect("the source changes");
    queue.update(cx, |queue, cx| queue.clean_again(rows[1], cx));
    cx.run_until_parked();
    assert_eq!(written(&queue, rows[1], cx), Some(tagged.clone()));
    assert_eq!(read(&tagged), cleaned(MARKED_AGAIN));
    assert_eq!(
        read(&plain),
        cleaned(MARKED),
        "the first row's result was touched"
    );
    let mut expected = vec![
        plain
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .into_owned(),
        tagged
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .into_owned(),
    ];
    expected.sort();
    assert_eq!(names(&scratch, ".cleaned"), expected, "a new file appeared");
}

/// Test 2: Rewrite again writes over the row's own result — the item names
/// it as `Destination::File`, the same name holds the new rewrite, and no
/// `x.rewritten-<id>.md` appears. Take `over` out of `claim` and the row
/// claims its own `-<id>` name beside the taken plain one: red.
#[gpui::test]
fn rewrite_again_writes_over_the_rows_own_result(cx: &mut TestAppContext) {
    let scratch = Scratch::new("again-rewrite");
    let source = scratch.file("x.md", PARAGRAPH.as_bytes());
    let work = work(swapping());
    let (queue, preferences, cx) = queue_with(cx, &scratch, Some(work.clone()));
    here_on_duty(&preferences, cx);
    queue.update(cx, |queue, cx| queue.hand(vec![source.clone()], cx));
    named(&queue, cx);
    let id = ids(&queue, cx)[0];
    queue.update(cx, |queue, cx| queue.rewrite(&[id], cx));
    until(cx, "the rewrite", |cx| status(&queue, cx) == "rewritten");
    let own = scratch.0.join("x.rewritten.md");
    let first = read(&own);

    std::fs::write(&source, paragraph_again()).expect("the source changes");
    let before = items(&work);
    queue.update(cx, |queue, cx| queue.rewrite_again(id, cx));
    again_done(&queue, &work, &before, id, cx);
    assert_eq!(
        new_item(&work, &before).destination,
        Some(Destination::File(own.clone())),
        "Rewrite again did not name the row's own result"
    );
    assert_ne!(
        read(&own),
        first,
        "the row's own result does not hold the new rewrite"
    );
    assert_eq!(written(&queue, id, cx), Some(own));
    assert_eq!(
        names(&scratch, ".rewritten"),
        ["x.rewritten.md"],
        "Rewrite again made a new file"
    );
}

/// Test 2, the `-<id>` row: two rows of one file rewritten, the second's
/// result `x.rewritten-<id>.md`; its Rewrite again replaces that file, the
/// first row's plain file is left byte for byte, and no `-<id>-2` appears.
#[gpui::test]
fn rewrite_again_of_a_row_named_by_its_id_replaces_that_file_and_no_other(cx: &mut TestAppContext) {
    let scratch = Scratch::new("again-rewrite-own");
    let source = scratch.file("x.md", PARAGRAPH.as_bytes());
    let work = work(swapping());
    let (queue, preferences, cx) = queue_with(cx, &scratch, Some(work.clone()));
    here_on_duty(&preferences, cx);
    queue.update(cx, |queue, cx| {
        queue.hand(vec![source.clone(), source.clone()], cx)
    });
    named(&queue, cx);
    let rows = ids(&queue, cx);
    queue.update(cx, |queue, cx| queue.rewrite(&rows, cx));
    until(cx, "the rewrites", |cx| {
        statuses(&queue, cx) == ["rewritten", "rewritten"]
    });
    let plain = scratch.0.join("x.rewritten.md");
    let tagged = own(&scratch, "x.md", "rewritten", entry(&queue, rows[1], cx));
    assert_eq!(written(&queue, rows[1], cx), Some(tagged.clone()));
    let (plain_first, tagged_first) = (read(&plain), read(&tagged));

    std::fs::write(&source, paragraph_again()).expect("the source changes");
    let before = items(&work);
    queue.update(cx, |queue, cx| queue.rewrite_again(rows[1], cx));
    again_done(&queue, &work, &before, rows[1], cx);
    assert_eq!(
        new_item(&work, &before).destination,
        Some(Destination::File(tagged.clone()))
    );
    assert_ne!(read(&tagged), tagged_first, "not rewritten again");
    assert_eq!(
        read(&plain),
        plain_first,
        "the first row's result was touched"
    );
    let mut expected = vec![
        plain
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .into_owned(),
        tagged
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .into_owned(),
    ];
    expected.sort();
    assert_eq!(
        names(&scratch, ".rewritten"),
        expected,
        "a new file appeared"
    );
}

/// Test 3, D704 rule 3: row A's result `x.cleaned.md` is deleted; row B, the
/// same file again, cleans and takes the plain name. A's Clean again must
/// not write over B's file — it is B's now: A takes its own `-<idA>`, and
/// B's file is left byte for byte. Take rule 3 out of `own_result` and A
/// writes over B: red.
#[gpui::test]
fn clean_again_never_writes_over_another_rows_result(cx: &mut TestAppContext) {
    let scratch = Scratch::new("again-rule-3");
    let source = scratch.file("x.md", MARKED.as_bytes());
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work(swapping())));
    queue.update(cx, |queue, cx| queue.hand(vec![source.clone()], cx));
    named(&queue, cx);
    let a = ids(&queue, cx)[0];
    queue.update(cx, |queue, cx| queue.clean(&[a], cx));
    cx.run_until_parked();
    let plain = scratch.0.join("x.cleaned.md");
    assert_eq!(written(&queue, a, cx), Some(plain.clone()));
    std::fs::remove_file(&plain).expect("deleted by hand");

    queue.update(cx, |queue, cx| queue.hand(vec![source.clone()], cx));
    named(&queue, cx);
    let b = ids(&queue, cx)[1];
    queue.update(cx, |queue, cx| queue.clean(&[b], cx));
    cx.run_until_parked();
    assert_eq!(
        written(&queue, b, cx),
        Some(plain.clone()),
        "B took the plain name"
    );

    std::fs::write(&source, MARKED_AGAIN).expect("the source changes");
    queue.update(cx, |queue, cx| queue.clean_again(a, cx));
    cx.run_until_parked();
    assert_eq!(read(&plain), cleaned(MARKED), "A wrote over B's result");
    let tagged = own(&scratch, "x.md", "cleaned", entry(&queue, a, cx));
    assert_eq!(written(&queue, a, cx), Some(tagged.clone()));
    assert_eq!(read(&tagged), cleaned(MARKED_AGAIN));
}

/// D703b: the same, inside the line. A's own result is deleted, and B — the
/// same file again — is asked to clean just before A's Clean again, so when
/// A is asked B has written nothing and rule 3 lets A's own name through.
/// B runs first and takes the plain name; the line then drops A's
/// `replacing`, because that file is B's result now, and A takes its own
/// `-<idA>`. Take `Line::taken` out of `Cleaner::finished` and A writes over
/// B's file: two rows, one result — red.
#[gpui::test]
fn a_name_the_line_wrote_meanwhile_is_not_replaced(cx: &mut TestAppContext) {
    let scratch = Scratch::new("again-line");
    let source = scratch.file("x.md", MARKED.as_bytes());
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work(swapping())));
    queue.update(cx, |queue, cx| {
        queue.hand(vec![source.clone(), source.clone()], cx)
    });
    named(&queue, cx);
    let rows = ids(&queue, cx);
    let (a, b) = (rows[0], rows[1]);
    queue.update(cx, |queue, cx| queue.clean(&[a], cx));
    cx.run_until_parked();
    let plain = scratch.0.join("x.cleaned.md");
    assert_eq!(written(&queue, a, cx), Some(plain.clone()));
    std::fs::remove_file(&plain).expect("deleted by hand");

    // B first in the line, A's Clean again behind it, in one turn.
    queue.update(cx, |queue, cx| {
        queue.clean(&[b], cx);
        queue.clean_again(a, cx);
    });
    cx.run_until_parked();
    assert_eq!(
        written(&queue, b, cx),
        Some(plain.clone()),
        "B took the plain name"
    );
    let tagged = own(&scratch, "x.md", "cleaned", entry(&queue, a, cx));
    assert_eq!(
        written(&queue, a, cx),
        Some(tagged),
        "A wrote over the result B had just written"
    );
}

/// Test 4, D704 rule 1: a row's own result deleted by hand. Clean again
/// writes the first free name of its own — here the plain one again — as a
/// new file, and nothing fails; Rewrite again claims its own name rather
/// than "replacing" a file that is not there. Take the existence check out
/// of `publish`, or out of `claim`, and the row says it replaced a file it
/// wrote afresh, or names `Destination::File` for it: red.
#[gpui::test]
fn a_deleted_result_is_written_again_under_a_name_of_its_own(cx: &mut TestAppContext) {
    let scratch = Scratch::new("again-deleted");
    let text = scratch.file("x.md", MARKED.as_bytes());
    let prose = scratch.file("p.md", PARAGRAPH.as_bytes());
    let work = work(swapping());
    let (queue, preferences, cx) = queue_with(cx, &scratch, Some(work.clone()));
    here_on_duty(&preferences, cx);
    queue.update(cx, |queue, cx| {
        queue.hand(vec![text.clone(), prose.clone()], cx)
    });
    named(&queue, cx);
    let rows = ids(&queue, cx);

    queue.update(cx, |queue, cx| queue.clean(&rows[..1], cx));
    cx.run_until_parked();
    let cleaned_own = scratch.0.join("x.cleaned.md");
    std::fs::remove_file(&cleaned_own).expect("deleted by hand");
    std::fs::write(&text, MARKED_AGAIN).expect("the source changes");
    queue.update(cx, |queue, cx| queue.clean_again(rows[0], cx));
    cx.run_until_parked();
    assert_eq!(statuses(&queue, cx)[0], "cleaned");
    assert_eq!(written(&queue, rows[0], cx), Some(cleaned_own.clone()));
    assert_eq!(read(&cleaned_own), cleaned(MARKED_AGAIN));
    assert!(
        !outcome(&queue, rows[0], cx).replaced,
        "a file that was not there was said to be replaced"
    );

    queue.update(cx, |queue, cx| queue.rewrite(&rows[1..], cx));
    until(cx, "the rewrite", |cx| {
        statuses(&queue, cx)[1] == "rewritten"
    });
    let rewritten_own = scratch.0.join("p.rewritten.md");
    std::fs::remove_file(&rewritten_own).expect("deleted by hand");
    let before = items(&work);
    queue.update(cx, |queue, cx| queue.rewrite_again(rows[1], cx));
    until(cx, "the rewrite again", |cx| {
        work.queue
            .items()
            .iter()
            .any(|view| !before.contains(&view.id) && view.state == State::Done)
            && statuses(&queue, cx)[1] == "rewritten"
    });
    assert!(
        matches!(
            new_item(&work, &before).destination,
            Some(Destination::Own { plain: true, .. })
        ),
        "a deleted result was named to be written over"
    );
    assert!(
        rewritten_own.exists(),
        "the plain name was not written again"
    );
    assert_eq!(written(&queue, rows[1], cx), Some(rewritten_own));
}

/// D704 rule 4: a plan changed since — Retention now says "into the results
/// folder" — writes there, as a clean's plan has always been taken when it
/// starts (D279) and a rewrite's when pushed (D91); the row's earlier file
/// beside the source is left as it was. Take the names check out of
/// `claim` and the rewrite goes back beside the file: red.
#[gpui::test]
fn a_plan_changed_since_writes_where_it_says_now(cx: &mut TestAppContext) {
    let scratch = Scratch::new("again-plan");
    let text = scratch.file("x.md", MARKED.as_bytes());
    let prose = scratch.file("p.md", PARAGRAPH.as_bytes());
    let work = work(swapping());
    let (queue, preferences, cx) = queue_with(cx, &scratch, Some(work.clone()));
    here_on_duty(&preferences, cx);
    queue.update(cx, |queue, cx| {
        queue.hand(vec![text.clone(), prose.clone()], cx)
    });
    named(&queue, cx);
    let rows = ids(&queue, cx);
    queue.update(cx, |queue, cx| queue.clean(&rows[..1], cx));
    cx.run_until_parked();
    queue.update(cx, |queue, cx| queue.rewrite(&rows[1..], cx));
    until(cx, "the rewrite", |cx| {
        statuses(&queue, cx)[1] == "rewritten"
    });
    let (beside_clean, beside_rewrite) = (
        scratch.0.join("x.cleaned.md"),
        scratch.0.join("p.rewritten.md"),
    );
    let (clean_first, rewrite_first) = (read(&beside_clean), read(&beside_rewrite));

    preferences.update(cx, |preferences, cx| {
        preferences.select_destination(Goes::Folder, cx);
    });
    cx.run_until_parked();
    std::fs::write(&text, MARKED_AGAIN).expect("the source changes");
    std::fs::write(&prose, paragraph_again()).expect("the source changes");
    queue.update(cx, |queue, cx| queue.clean_again(rows[0], cx));
    cx.run_until_parked();
    let into = scratch.0.join("results");
    assert_eq!(
        written(&queue, rows[0], cx),
        Some(into.join("x.cleaned.md"))
    );
    assert_eq!(read(&beside_clean), clean_first);

    let before = items(&work);
    queue.update(cx, |queue, cx| queue.rewrite_again(rows[1], cx));
    again_done(&queue, &work, &before, rows[1], cx);
    assert_eq!(
        written(&queue, rows[1], cx),
        Some(into.join("p.rewritten.md"))
    );
    assert_eq!(
        read(&beside_rewrite),
        rewrite_first,
        "the earlier file moved"
    );
}

/// Test 5, D706, in the window: a clean in place greys Clean again with the
/// in-place reason, and asking for it anyway changes nothing; the same for a
/// rewrite in place and its Rewrite again, which pushes nothing.
#[gpui::test]
fn in_place_the_window_redoes_nothing(cx: &mut TestAppContext) {
    let scratch = Scratch::new("again-in-place");
    let text = scratch.file("a.md", MARKED.as_bytes());
    let prose = scratch.file("b.md", PARAGRAPH.as_bytes());
    let work = work(swapping());
    let (queue, preferences, cx) = queue_with(cx, &scratch, Some(work.clone()));
    here_on_duty(&preferences, cx);
    preferences.update(cx, |preferences, cx| {
        preferences.select_destination(Goes::Replace, cx);
    });
    queue.update(cx, |queue, cx| {
        queue.hand(vec![text.clone(), prose.clone()], cx)
    });
    named(&queue, cx);
    let rows = ids(&queue, cx);
    let reason = |name: &str| {
        Some(t_args(
            Message::QueueActionAgainInPlace,
            &args!("name" => name.to_owned()),
        ))
    };

    queue.update(cx, |queue, cx| queue.clean(&rows[..1], cx));
    cx.run_until_parked();
    let first = outcome(&queue, rows[0], cx);
    assert!(first.set_aside.is_some(), "not cleaned in place");
    let why = cx.update(|_, cx| {
        let queue = queue.read(cx);
        let row = &queue.rows[0];
        why_not_clean(&row.status, row.cleanable())
    });
    assert_eq!(why, reason("a.original.md"));
    let held = read(&text);
    queue.update(cx, |queue, cx| queue.clean_again(rows[0], cx));
    cx.run_until_parked();
    assert!(Arc::ptr_eq(&first, &outcome(&queue, rows[0], cx)), "redone");
    assert_eq!(read(&text), held);

    queue.update(cx, |queue, cx| queue.rewrite(&rows[1..], cx));
    until(cx, "the rewrite", |cx| {
        statuses(&queue, cx)[1] == "rewritten"
    });
    let why = cx.update(|_, cx| queue.read(cx).why_not_rewrite(rows[1], cx));
    assert_eq!(why, reason("b.original.md"));
    queue.update(cx, |queue, cx| queue.rewrite_again(rows[1], cx));
    cx.run_until_parked();
    std::thread::sleep(Duration::from_millis(100));
    cx.run_until_parked();
    assert_eq!(work.queue.items().len(), 1, "a rewrite in place was redone");
}

// -- D705: an edited result asks first -----------------------------------

/// Test 7, a clean: an edited row's Clean again asks — `QueueEvent::Again`
/// — and changes nothing; the yes, `again_agreed`, runs it without asking
/// again; a row without edits runs at once and asks nothing. Take the
/// edited check out of `clean_again` and the edits are replaced unasked:
/// red.
#[gpui::test]
fn clean_again_over_an_edited_result_asks_first(cx: &mut TestAppContext) {
    let scratch = Scratch::new("again-asks-clean");
    let edited_file = scratch.file("x.md", MARKED.as_bytes());
    let plain_file = scratch.file("y.md", MARKED.as_bytes());
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work(swapping())));
    let heard = questions(&queue, cx);
    queue.update(cx, |queue, cx| {
        queue.hand(vec![edited_file.clone(), plain_file.clone()], cx)
    });
    named(&queue, cx);
    let rows = ids(&queue, cx);
    queue.update(cx, |queue, cx| queue.clean(&rows, cx));
    cx.run_until_parked();
    let own = scratch.0.join("x.cleaned.md");
    std::fs::write(&own, "edited by hand\n").expect("an edit, saved");
    let told = Told::Saved {
        action: Action::Clean,
        home: Home::File(own.clone()),
    };
    assert!(queue.update(cx, |queue, cx| queue.told_by_compare(rows[0], told, cx)));
    assert!(edited(&queue, rows[0], cx));

    let before = outcome(&queue, rows[0], cx);
    queue.update(cx, |queue, cx| queue.clean_again(rows[0], cx));
    cx.run_until_parked();
    assert_eq!(
        *heard.lock().expect("lock"),
        [(rows[0], Again::Clean)],
        "not asked"
    );
    assert!(
        Arc::ptr_eq(&before, &outcome(&queue, rows[0], cx)),
        "changed before the answer"
    );
    assert_eq!(
        read(&own),
        "edited by hand\n",
        "the edits went before the answer"
    );

    queue.update(cx, |queue, cx| {
        queue.again_agreed(rows[0], Again::Clean, cx)
    });
    cx.run_until_parked();
    assert_eq!(heard.lock().expect("lock").len(), 1, "asked again");
    assert_eq!(read(&own), cleaned(MARKED), "the yes did not redo it");

    // No edits: at once, and nothing asked.
    let before = outcome(&queue, rows[1], cx);
    queue.update(cx, |queue, cx| queue.clean_again(rows[1], cx));
    cx.run_until_parked();
    assert_eq!(
        heard.lock().expect("lock").len(),
        1,
        "a row without edits asked"
    );
    assert!(
        !Arc::ptr_eq(&before, &outcome(&queue, rows[1], cx)),
        "not run"
    );
}

/// Test 7, a rewrite: the same for Rewrite again — asked, nothing pushed
/// until the yes, then pushed; without edits, pushed at once. Take the
/// edited check out of `rewrite_again`: red.
#[gpui::test]
fn rewrite_again_over_an_edited_result_asks_first(cx: &mut TestAppContext) {
    let scratch = Scratch::new("again-asks-rewrite");
    let first = scratch.file("x.md", PARAGRAPH.as_bytes());
    let second = scratch.file("y.md", PARAGRAPH.as_bytes());
    let work = work(swapping());
    let (queue, preferences, cx) = queue_with(cx, &scratch, Some(work.clone()));
    here_on_duty(&preferences, cx);
    let heard = questions(&queue, cx);
    queue.update(cx, |queue, cx| queue.hand(vec![first, second], cx));
    named(&queue, cx);
    let rows = ids(&queue, cx);
    queue.update(cx, |queue, cx| queue.rewrite(&rows, cx));
    until(cx, "the rewrites", |cx| {
        statuses(&queue, cx) == ["rewritten", "rewritten"]
    });
    let own = scratch.0.join("x.rewritten.md");
    std::fs::write(&own, "edited by hand\n").expect("an edit, saved");
    let told = Told::Saved {
        action: Action::Rewrite,
        home: Home::File(own.clone()),
    };
    assert!(queue.update(cx, |queue, cx| queue.told_by_compare(rows[0], told, cx)));
    assert!(edited(&queue, rows[0], cx));

    let before = items(&work);
    queue.update(cx, |queue, cx| queue.rewrite_again(rows[0], cx));
    cx.run_until_parked();
    std::thread::sleep(Duration::from_millis(100));
    cx.run_until_parked();
    assert_eq!(
        *heard.lock().expect("lock"),
        [(rows[0], Again::Rewrite)],
        "not asked"
    );
    assert_eq!(items(&work), before, "pushed before the answer");
    assert_eq!(read(&own), "edited by hand\n");

    queue.update(cx, |queue, cx| {
        queue.again_agreed(rows[0], Again::Rewrite, cx)
    });
    until(cx, "the yes", |_| {
        work.queue
            .items()
            .iter()
            .any(|view| !before.contains(&view.id) && view.state == State::Done)
    });
    assert_eq!(heard.lock().expect("lock").len(), 1, "asked again");
    assert_ne!(read(&own), "edited by hand\n", "the yes did not redo it");

    // No edits: pushed at once, and nothing asked.
    let before = items(&work);
    queue.update(cx, |queue, cx| queue.rewrite_again(rows[1], cx));
    until(cx, "the push", |_| {
        work.queue
            .items()
            .iter()
            .any(|view| !before.contains(&view.id))
    });
    assert_eq!(
        heard.lock().expect("lock").len(),
        1,
        "a row without edits asked"
    );
}

// -- D707: what a redo resets --------------------------------------------

/// Test 8: a redo is a new result. After Clean again over an edited result,
/// the row is edited no more and its journal entry's `outcome.edited` is
/// gone; the same after Rewrite again. Leave the row's `edited_at` in place
/// in `heard`, or in the batch queue's end, and the row still says "…, then
/// edited" over a result nobody edited: red.
#[gpui::test]
fn a_redo_forgets_the_edits_in_the_row_and_the_journal(cx: &mut TestAppContext) {
    let scratch = Scratch::new("again-forgets");
    let text = scratch.file("x.md", MARKED.as_bytes());
    let prose = scratch.file("p.md", PARAGRAPH.as_bytes());
    let work = work(swapping());
    let (queue, preferences, cx) = queue_with(cx, &scratch, Some(work.clone()));
    here_on_duty(&preferences, cx);
    queue.update(cx, |queue, cx| queue.hand(vec![text, prose], cx));
    named(&queue, cx);
    let rows = ids(&queue, cx);
    let entries = [entry(&queue, rows[0], cx), entry(&queue, rows[1], cx)];
    queue.update(cx, |queue, cx| queue.clean(&rows[..1], cx));
    cx.run_until_parked();
    queue.update(cx, |queue, cx| queue.rewrite(&rows[1..], cx));
    until(cx, "the rewrite", |cx| {
        statuses(&queue, cx)[1] == "rewritten"
    });
    until(cx, "the rewrite's journal end", |_| {
        work.journal
            .rows()
            .iter()
            .any(|row| row.id == entries[1] && row.state == "done")
    });

    let saves = [
        (Action::Clean, scratch.0.join("x.cleaned.md")),
        (Action::Rewrite, scratch.0.join("p.rewritten.md")),
    ];
    for (row, (action, home)) in rows.iter().zip(saves) {
        std::fs::write(&home, "edited by hand\n").expect("an edit, saved");
        let told = Told::Saved {
            action,
            home: Home::File(home),
        };
        assert!(queue.update(cx, |queue, cx| queue.told_by_compare(*row, told, cx)));
    }
    flushed(&queue, cx);
    for (row, entry) in rows.iter().zip(entries) {
        assert!(edited(&queue, *row, cx));
        assert!(journal_edited(&work, entry).is_some(), "not marked");
    }

    let before = items(&work);
    queue.update(cx, |queue, cx| {
        queue.again_agreed(rows[0], Again::Clean, cx);
        queue.again_agreed(rows[1], Again::Rewrite, cx);
    });
    cx.run_until_parked();
    again_done(&queue, &work, &before, rows[1], cx);
    until(cx, "the journal's end", |_| {
        let item = new_item(&work, &before).id;
        work.journal
            .rows()
            .iter()
            .any(|row| row.id == entries[1] && row.state == "done" && row.item == Some(item.0))
    });
    flushed(&queue, cx);
    for (row, entry) in rows.iter().zip(entries) {
        assert!(
            !edited(&queue, *row, cx),
            "the row still says it was edited"
        );
        assert_eq!(
            journal_edited(&work, entry),
            None,
            "the journal entry still says it was edited"
        );
    }
}

/// Test 8: Rewrite again of a paste — whose rewritten text lives only in
/// the batch queue's row (В4) — leaves exactly one item for the row: the
/// earlier one goes once the new one is reserved. Leave it, and the paste's
/// first rewrite stays in the database, reachable from nothing: red.
#[gpui::test]
fn rewrite_again_of_a_paste_leaves_one_item(cx: &mut TestAppContext) {
    let scratch = Scratch::new("again-paste");
    let work = work(swapping());
    let (queue, preferences, cx) = queue_with(cx, &scratch, Some(work.clone()));
    here_on_duty(&preferences, cx);
    queue.update(cx, |queue, cx| {
        queue.land(vec![Handed::Text(PARAGRAPH.to_owned())], cx)
    });
    cx.run_until_parked();
    let id = ids(&queue, cx)[0];
    queue.update(cx, |queue, cx| queue.rewrite(&[id], cx));
    until(cx, "the rewrite", |cx| status(&queue, cx) == "rewritten");
    let before = items(&work);
    assert_eq!(before.len(), 1);

    queue.update(cx, |queue, cx| queue.rewrite_again(id, cx));
    again_done(&queue, &work, &before, id, cx);
    let now = items(&work);
    assert_eq!(
        now.len(),
        1,
        "the paste's earlier rewrite was left: {now:?}"
    );
    let named_item = cx.update(|_, cx| queue.read(cx).rows[0].item);
    assert_eq!(
        named_item,
        Some(now[0]),
        "the row does not name its new item"
    );
    assert!(
        work.queue.result(before[0]).expect("read").is_none(),
        "the earlier result is still stored"
    );
}

// -- D708: what does not change ------------------------------------------

/// Test 9: Clean all takes waiting rows only — a row already cleaned is not
/// cleaned again by it — and neither is it by `Queue::clean`, the panel's
/// and `--clean=`'s road (D691). Take the waiting check out of
/// `Queue::clean` and both redo the row: red.
#[gpui::test]
fn clean_all_and_the_panels_clean_never_redo_a_row(cx: &mut TestAppContext) {
    let scratch = Scratch::new("again-not-all");
    let source = scratch.file("x.md", MARKED.as_bytes());
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work(swapping())));
    queue.update(cx, |queue, cx| queue.hand(vec![source.clone()], cx));
    named(&queue, cx);
    let id = ids(&queue, cx)[0];
    queue.update(cx, |queue, cx| queue.clean_all(cx));
    cx.run_until_parked();
    let first = outcome(&queue, id, cx);
    let own = scratch.0.join("x.cleaned.md");
    std::fs::write(&source, MARKED_AGAIN).expect("the source changes");

    queue.update(cx, |queue, cx| queue.clean_all(cx));
    cx.run_until_parked();
    queue.update(cx, |queue, cx| queue.clean(&[id], cx));
    cx.run_until_parked();
    assert!(
        Arc::ptr_eq(&first, &outcome(&queue, id, cx)),
        "cleaned again"
    );
    assert_eq!(read(&own), cleaned(MARKED));
    assert_eq!(names(&scratch, ".cleaned"), ["x.cleaned.md"]);
    assert!(cx
        .update(|_, cx| queue.read(cx).cleanable_waiting())
        .is_empty());
}
