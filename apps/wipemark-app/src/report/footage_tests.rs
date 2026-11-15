//! The Report dialog drawn headless, frame by frame, as footage for the
//! text video (`dev/video/text/storyboard.md`, "Into the report" and "The
//! shelves"): the clean of the demo text, opened over the table as the
//! shell opens it when the row's Report… asks, at its top and then its body
//! scrolled to the end, one frame a video frame, on the video's
//! `CURVE.move`. Every frame is GPUI's own renderer (`crate::footage`), and
//! every rectangle in `clip.json` is the one GPUI laid the part out at
//! (`crate::spot`).
//!
//! Run (ignored, alone: it sets the process's language):
//!   WIPEMARK_FOOTAGE=<dir> [WIPEMARK_FOOTAGE_DARK=1] \
//!   [WIPEMARK_FOOTAGE_FOLDER=<folder>] cargo test -p wipemark-app \
//!     --bin wipemark -- --ignored report::footage_tests::text_video_report \
//!     --exact --nocapture --test-threads=1
//!
//! It writes `<dir>/report-dark/` or `<dir>/report-light/`, from the table
//! `queue::footage_tests::desk` sets up in the demo folder.

use std::path::PathBuf;

use gpui::{px, Pixels};
use serde_json::{json, Value};

use super::SECTIONS;
use crate::footage::{curve_move, Clip};
use crate::queue::footage_tests::{desk, desk_size};
use crate::queue::QueueEvent;

/// How long the dialog stands at its top before the body scrolls, and at
/// its end after: a second each.
const STILL: usize = 30;

/// The scroll from the top to the end: 0.8 s, one frame a video frame.
const SCROLL: usize = 24;

/// A line as the composition reads it: Fluent's isolates, which a window
/// draws as nothing, taken out.
fn plain(text: &str) -> String {
    text.replace(['\u{2068}', '\u{2069}'], "")
}

#[test]
#[ignore]
fn text_video_report() {
    let out = PathBuf::from(std::env::var("WIPEMARK_FOOTAGE").expect("WIPEMARK_FOOTAGE"));
    let mut desk = desk();
    let id = desk.reply;
    let cx = &mut desk.cx;

    // The row's Clean, then its Report…: the menu item emits this event
    // (`queue::actions_cell`), and the shell lays the dialog over the table.
    desk.queue.update(cx, |queue, cx| queue.clean(&[id], cx));
    cx.run_until_parked();
    desk.queue
        .update(cx, |_, cx| cx.emit(QueueEvent::Report(id)));
    cx.run_until_parked();
    let report = cx
        .update(|cx| desk.main.read(cx).report())
        .expect("the Report dialog is open");
    let sheet = cx.update(|cx| report.read(cx).sheet.clone());

    // Nothing of this machine in the dialog: the result's path is the demo
    // folder's, and no line names another place.
    let result = format!(
        "The result: {}",
        desk.folder.join("reply.cleaned.txt").display()
    );
    let happened: Vec<String> = sheet.happened.iter().map(|l| plain(&l.text)).collect();
    assert!(happened.contains(&result), "{happened:?}");
    let demo = desk.folder.display().to_string();
    for (_, lines) in sheet.sections() {
        for line in lines {
            let text = plain(&line.text);
            assert!(
                !text.contains('/') || text.contains(&demo),
                "a line names a place outside the demo folder: {text}"
            );
        }
    }

    let mut clip = Clip::new(
        &out,
        &format!("report-{}", desk.theme),
        desk.window,
        desk_size(),
        &["report-"],
    );
    clip.mark("open");
    for _ in 0..STILL {
        clip.frame(cx);
    }

    // The body's farthest scroll: its last line's bottom under its bottom.
    let body = clip.spot("report-body");
    let last = format!(
        "report-line-not-established-{}",
        sheet.not_established.len() - 1
    );
    let bottom = |bounds: gpui::Bounds<Pixels>| bounds.origin.y + bounds.size.height;
    let farthest = bottom(clip.spot(&last)) - bottom(body);
    assert!(farthest > px(0.0), "the body does not scroll: {farthest:?}");
    // Over the body's foot, where the table behind it has no row: the
    // wheel moves GPUI's pointer there, and a row under it would light up
    // its hover behind the backdrop, under no drawn pointer.
    let wheel_at = gpui::point(body.center().x, bottom(body) - px(16.0));
    let top = clip.spot("report-heading-arrived").origin.y;
    let mut scrolled = px(0.0);
    clip.mark("scroll-start");
    let start = clip.next();
    let mut offsets = vec![json!([start - 1, 0.0])];
    for step in 1..=SCROLL {
        let to = farthest * curve_move(step as f32 / SCROLL as f32);
        clip.wheel(cx, wheel_at, to - scrolled);
        scrolled = to;
        clip.frame(cx);
        let offset = top - clip.spot("report-heading-arrived").origin.y;
        offsets.push(json!([clip.next() - 1, f32::from(offset)]));
    }
    let end = clip.next() - 1;
    clip.mark_at("scroll-end", end);
    let reached = top - clip.spot("report-heading-arrived").origin.y;
    assert!(
        (reached - farthest).abs() < px(0.5),
        "scrolled {reached:?} of {farthest:?}"
    );
    for _ in 0..STILL {
        clip.frame(cx);
    }

    // The regions: the panel and its parts at the top, and every line at
    // the top and at the end of the scroll.
    for (name, spot) in [
        ("panel", "report-panel"),
        ("title", "report-title"),
        ("body", "report-body"),
        ("buttons", "report-buttons"),
    ] {
        clip.region(name, spot, 0);
    }
    let mut lines = Vec::new();
    for ((_, shelf), section) in sheet.sections().into_iter().zip(SECTIONS) {
        let heading = format!("heading-{section}");
        clip.region(&heading, &format!("report-{heading}"), 0);
        clip.region(&format!("{heading}@end"), &format!("report-{heading}"), end);
        for (at, line) in shelf.iter().enumerate() {
            let region = format!("line-{section}-{at}");
            clip.region(&region, &format!("report-{region}"), 0);
            clip.region(&format!("{region}@end"), &format!("report-{region}"), end);
            let text = plain(&line.text);
            let codepoint = text
                .strip_prefix("U+")
                .and_then(|rest| rest.split_once(' '))
                .map(|(hex, _)| format!("U+{hex}"));
            lines.push(json!({
                "region": region,
                "section": section,
                "depth": line.depth,
                "text": text,
                "codepoint": codepoint,
            }));
        }
    }
    clip.note("theme", json!(desk.theme));
    clip.note(
        "lines_note",
        json!(
            "a line's region is the full width of the body, its text starting at the region's \
             left (a code point first); a line is seen only where it is inside the body region, \
             which clips the scroll"
        ),
    );
    clip.note("lines", Value::Array(lines));
    clip.note(
        "scroll",
        json!({
            "what": "the body's scroll offset in window points, per frame from scroll-start - 1",
            "from": start - 1,
            "to": end,
            "farthest": f32::from(farthest),
            "offsets": offsets,
        }),
    );
    clip.finish("text video, the Report of reply.txt's clean, scrolled to its end");
}
