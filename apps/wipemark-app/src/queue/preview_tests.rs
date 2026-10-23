//! A picture row with no file behind it shows the preview Wipemark kept of
//! it (D823–D828; the owner, 2026-10-23: "with true, it must persist
//! between launches"). Over a real journal keeping its previews in a
//! scratch folder, and the table reading it as it does at launch.

use std::path::PathBuf;
use std::sync::Arc;

use gpui::{Bounds, Entity, Modifiers, Pixels, TestAppContext, VisualTestContext};
use wipemark_intake::Handed;
use wipemark_queue::Durability;
use wipemark_store::entry::{Action, Delivered, Entry, Origin, Outcome, Phase};
use wipemark_store::{NewRow, Store};

use super::rewrite_tests::until;
use super::tests::{queue_with, Scratch};
use super::{Queue, Status};
use crate::drop::Arrival;
use crate::engine_host::EngineHandle;
use crate::journal::{self, Journal, Work};
use crate::preview::{Picture, Preview};
use crate::previews::Source;

/// A 32×32 picture with alpha, as a paste or an agent hands one over.
const PNG: &[u8] = include_bytes!("../../assets/icon/32x32.png");

/// A batch queue and a journal over a store in memory, the journal keeping
/// its previews in `dir`.
fn work_keeping(dir: PathBuf) -> Work {
    let store = Arc::new(Store::in_memory().expect("memory"));
    let handle = EngineHandle::new().0;
    let queue = wipemark_queue::Queue::with_source(
        Arc::clone(&store),
        Durability::Memory { detail: None },
        Arc::new(handle.clone()),
    )
    .expect("the queue opens");
    let work = Work {
        queue: Arc::new(queue),
        journal: Journal::with_previews(store, dir),
        engine: handle,
        whereto: journal::Going::default(),
    };
    journal::keep_books(&work);
    work
}

/// An agent's `clean_image` as the MCP server records it: a picture, its
/// result handed back, nothing of it on disk.
fn agent_row(journal: &Journal) -> i64 {
    let entry = Entry {
        kind: Some("image".to_owned()),
        format: Some("WebP".to_owned()),
        size: Some(334_342),
        outcome: Some(Outcome {
            verdict: "cleaned".to_owned(),
            ..Outcome::default()
        }),
        result: Some(Delivered::Caller),
        ..Entry::default()
    };
    journal
        .record(&NewRow {
            origin: Origin::Agent.as_str(),
            action: Action::CleanImage.as_str(),
            state: Phase::Done.as_str(),
            item: None,
            arrived: 1,
            ended: Some(2),
            entry: &entry.to_json(),
        })
        .expect("a row")
}

/// Every preview asked for so far is made, or not.
fn made(work: &Work) {
    work.journal
        .previews()
        .expect("previews")
        .settled()
        .recv()
        .expect("settled");
}

/// The table's row for journal row `entry`: its preview.
fn preview_of(queue: &Entity<Queue>, entry: i64, cx: &mut VisualTestContext) -> Option<Preview> {
    cx.update(|_, cx| {
        queue
            .read(cx)
            .rows
            .iter()
            .find(|row| row.entry == Some(entry))
            .map(|row| row.preview.clone())
    })
}

/// Where `selector` was painted in the last frame, if it was.
fn found(selector: String, cx: &mut VisualTestContext) -> Option<Bounds<Pixels>> {
    cx.debug_bounds(Box::leak(selector.into_boxed_str()))
}

/// Let a hover's delay pass, and draw.
fn drawn(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(1));
        cx.run_until_parked();
    }
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}

/// Wait until the table lists `count` rows, each looked at.
fn listed(queue: &Entity<Queue>, count: usize, cx: &mut VisualTestContext) {
    until(cx, "the rows, looked at", |cx| {
        queue.read_with(cx, |queue, _| {
            queue.rows.len() == count
                && queue
                    .rows
                    .iter()
                    .all(|row| !matches!(row.preview, Preview::Pending))
        })
    });
}

/// D825, D828: a launch reads back a picture row with nothing behind it and
/// finds the preview kept of it — the small one in the cell, the large one
/// in the card — and a row like it with none keeps the kind's glyph. The
/// preview changes nothing the row can do: the two rows' Actions menus are
/// the same, item for item (D731). Look for kept previews only for rows
/// with a file behind them, or set one as the row's file: red.
#[gpui::test]
fn a_row_read_back_with_nothing_behind_it_shows_its_kept_preview(cx: &mut TestAppContext) {
    let scratch = Scratch::new("kept-preview-read-back");
    let work = work_keeping(scratch.0.join("previews"));
    let with = agent_row(&work.journal);
    let without = agent_row(&work.journal);
    work.journal.keep_preview(with, Source::Bytes(PNG.to_vec()));
    made(&work);
    let kept = work.journal.kept_preview(with).expect("a preview");
    // A launch: the notes the run before said are not this process's.
    work.journal.notes().drain().for_each(drop);

    let (queue, _, cx) = queue_with(cx, &scratch, Some(work.clone()));
    listed(&queue, 2, cx);
    assert_eq!(
        preview_of(&queue, with, cx),
        Some(Preview::Image(Picture::Kept(kept)))
    );
    assert_eq!(preview_of(&queue, without, cx), Some(Preview::None));

    let id = |entry: i64, cx: &mut VisualTestContext| {
        cx.update(|_, cx| {
            queue
                .read(cx)
                .rows
                .iter()
                .find(|row| row.entry == Some(entry))
                .expect("the row")
                .id
        })
    };
    let (with_id, without_id) = (id(with, cx), id(without, cx));
    let menus = cx.update(|_, cx| {
        let queue = queue.read(cx);
        (queue.menu_of(with_id, cx), queue.menu_of(without_id, cx))
    });
    assert!(menus.0.is_some());
    assert_eq!(
        menus.0, menus.1,
        "a kept preview changed what the row can do"
    );

    // Drawn: the kept preview in its frame, and the card over it on a
    // hover; the other row's glyph is neither.
    drawn(cx);
    let cell =
        found(format!("queue-preview-{with_id}"), cx).expect("the kept preview was not drawn");
    assert_eq!(found(format!("queue-preview-{without_id}"), cx), None);
    cx.simulate_mouse_move(cell.center(), None, Modifiers::default());
    drawn(cx);
    assert!(
        found("queue-preview-card".to_owned(), cx).is_some(),
        "no card over the kept preview"
    );
}

/// D827, D826: a preview made after its row is listed lands on the row
/// when the journal says it is in place, and Delete kept previews takes it
/// off the row again. Leave either note unheard: red.
#[gpui::test]
fn a_preview_lands_when_it_is_made_and_goes_when_it_is_deleted(cx: &mut TestAppContext) {
    let scratch = Scratch::new("kept-preview-lands");
    let work = work_keeping(scratch.0.join("previews"));
    let row = agent_row(&work.journal);
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work.clone()));
    listed(&queue, 1, cx);
    assert_eq!(preview_of(&queue, row, cx), Some(Preview::None));

    work.journal.keep_preview(row, Source::Bytes(PNG.to_vec()));
    until(cx, "the preview, on its row", |cx| {
        matches!(
            preview_of(&queue, row, cx),
            Some(Preview::Image(Picture::Kept(_)))
        )
    });

    assert_eq!(work.journal.forget_previews(), 1);
    until(cx, "the preview, gone from its row", |cx| {
        preview_of(&queue, row, cx) == Some(Preview::None)
    });
}

/// D823, every way in behaves the same: a picture pasted in the main
/// window and one dropped as bytes on the panel each keep a preview of what
/// arrived, the same one — and a text pasted keeps none. Keep a preview on
/// one road only: red.
#[gpui::test]
fn a_picture_pasted_in_the_window_or_on_the_panel_keeps_a_preview(cx: &mut TestAppContext) {
    let scratch = Scratch::new("kept-preview-paste");
    let work = work_keeping(scratch.0.join("previews"));
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work.clone()));
    let picture = || Handed::Bytes {
        name: None,
        bytes: PNG.to_vec(),
    };
    queue.update(cx, |queue, cx| {
        queue.land(
            vec![picture(), Handed::Text("plain words\n".to_owned())],
            cx,
        )
    });
    cx.run_until_parked();
    let handed = picture();
    let intake = wipemark_intake::of(&handed);
    queue.update(cx, |queue, cx| {
        queue.take_from_panel(Arc::from(vec![Arrival { handed, intake }]), cx)
    });
    until(cx, "three rows, each with its journal row", |cx| {
        queue.read_with(cx, |queue, _| {
            queue.rows.len() == 3 && queue.rows.iter().all(|row| row.entry.is_some())
        })
    });
    let rows: Vec<(Origin, bool, i64)> = cx.update(|_, cx| {
        queue
            .read(cx)
            .rows
            .iter()
            .map(|row| {
                let picture = row.look.kind == wipemark_intake::Kind::Image;
                (row.origin, picture, row.entry.expect("an id"))
            })
            .collect()
    });
    let expected = crate::previews::make(PNG).expect("a preview").large;
    for &(origin, picture, entry) in &rows {
        if picture {
            until(cx, "the paste's preview", |_| {
                work.journal.kept_preview(entry).is_some()
            });
            let kept = work.journal.kept_preview(entry).expect("a preview");
            assert_eq!(
                std::fs::read(&kept.large).expect("reads"),
                expected,
                "{origin:?}"
            );
        }
    }
    made(&work);
    let text = rows
        .iter()
        .find(|(_, picture, _)| !picture)
        .expect("the text's row");
    assert_eq!(
        work.journal.kept_preview(text.2),
        None,
        "a text kept a preview"
    );
    assert_eq!(
        rows.iter()
            .filter(|(_, picture, _)| *picture)
            .map(|(origin, _, _)| *origin)
            .collect::<Vec<_>>(),
        [Origin::Window, Origin::Panel]
    );
}

/// D824: a picture pasted in the window and cleaned keeps a preview of its
/// result in place of the one of what arrived — the sparkle off. Keep the
/// first preview after the clean: red.
#[gpui::test]
#[cfg_attr(coverage, ignore = "heavy under coverage: past a minute (D806)")]
fn a_cleaned_paste_keeps_a_preview_of_its_result(cx: &mut TestAppContext) {
    let scratch = Scratch::new("kept-preview-cleaned");
    let work = work_keeping(scratch.0.join("previews"));
    let (queue, _, cx) = queue_with(cx, &scratch, Some(work.clone()));
    let sent = std::fs::read(format!(
        "{}/../../fixtures/image/gemini/torch-1025.png",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("the fixture");
    queue.update(cx, |queue, cx| {
        queue.land(
            vec![Handed::Bytes {
                name: None,
                bytes: sent.clone(),
            }],
            cx,
        )
    });
    until(cx, "the paste's journal row", |cx| {
        queue.read_with(cx, |queue, _| {
            queue.rows.len() == 1 && queue.rows[0].entry.is_some()
        })
    });
    let (id, entry) = cx.update(|_, cx| {
        let row = &queue.read(cx).rows[0];
        (row.id, row.entry.expect("an id"))
    });
    let arrived = crate::previews::make(&sent).expect("a preview").large;
    until(cx, "the preview of what arrived", |_| {
        work.journal
            .kept_preview(entry)
            .and_then(|kept| std::fs::read(kept.large).ok())
            .is_some_and(|large| large == arrived)
    });

    queue.update(cx, |queue, cx| queue.clean(&[id], cx));
    until(cx, "the clean", |cx| {
        queue.read_with(cx, |queue, _| {
            matches!(queue.status_of(id), Some(Status::Done(_)))
        })
    });
    let written = cx
        .update(|_, cx| queue.read(cx).rows[0].written())
        .expect("the clean wrote a result");
    let result = crate::previews::make(&std::fs::read(&written).expect("the result"))
        .expect("a preview")
        .large;
    assert_ne!(result, arrived, "the clean changed no pixel");
    until(cx, "the preview of the result", |_| {
        work.journal
            .kept_preview(entry)
            .and_then(|kept| std::fs::read(kept.large).ok())
            .is_some_and(|large| large == result)
    });
}
