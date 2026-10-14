//! A row's actions depend on what is on disk, never on where the row came
//! from (D729–D733; the owner at the macOS acceptance, 2026-10-14: "the
//! behaviour must not break the UI depending on where a thing came from").
//! Row 36 of the acceptance table — `wipemark-cli rewrite a10-rewrite.md`
//! with the application running — could not be opened in Compare, its file
//! and its result both on disk. Over a real journal and a real batch queue
//! on `FakeEngine`.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use gpui::{Entity, TestAppContext, VisualTestContext};
use wipemark_intake::Handed;
use wipemark_store::entry::{Action, Delivered, Entry, Origin, Outcome, Phase};
use wipemark_store::{NewRow, Store};

use super::rewrite_tests::{swapping, until, work_over, PARAGRAPH};
use super::tests::{queue_with, Scratch};
use super::Queue;
use crate::compare::{Home, Made, RewriteFrom, Told};
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
    let id = Journal::new(Arc::clone(&store))
        .record(&NewRow {
            origin: origin.as_str(),
            action: Action::Rewrite.as_str(),
            state: Phase::Done.as_str(),
            item: Some(11),
            arrived: 1,
            ended: Some(2),
            entry: &entry.to_json(),
        })
        .expect("the row");
    (store, id)
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
