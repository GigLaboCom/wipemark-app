//! Compare drawn headless as footage for the rewrite video
//! (`dev/video/rewrite/storyboard.md`): the window on the October rewrite
//! of the demo's team update (`fixtures/compare/team-update/`), opened as a
//! row's Compare opens a rewrite — the original's file, the result's own
//! file, the banner that says so — in a folder that names nobody. Every
//! frame is GPUI's own renderer (`crate::footage`); every rectangle is
//! the one GPUI laid the part out at (`crate::spot`) or the one the
//! editor says it drew a row or a word at.
//!
//! Run (ignored, one at a time: they set the process's language):
//!   WIPEMARK_FOOTAGE=<dir> [WIPEMARK_FOOTAGE_DARK=1] \
//!   [WIPEMARK_FOOTAGE_FOLDER=<folder>] cargo test -p wipemark-app \
//!     --bin wipemark -- --ignored compare::footage_tests::<test> \
//!     --exact --nocapture --test-threads=1
//!
//! - `rewrite_video_stills`: `C1` (marked by word) and `C2` (by line), one
//!   frame each at 1280×720 points, with the paragraphs' bands, the panes'
//!   text columns and every row the editors laid out. With
//!   `WIPEMARK_FOOTAGE_CHECK=<dir>` also `C1` at the October still's
//!   1280×800, to lay over it.
//! - `rewrite_video_scroll`: `scroll`, the original scrolled by the wheel
//!   to its end and back, one wheel event a frame, the result following.
//! - `rewrite_video_edit`: `edit`, a drag across "initial time in 1 year"
//!   in the result, "first time in a year" typed over it a key at a time,
//!   the marks landing again, and the autosave's *Unsaved changes*,
//!   *Saving…* and *Saved hh:mm* at a held clock.
//!
//! The folder is `/Users/Shared/Wipemark Demo/rewrite` unless given; the
//! tests write the two texts into it, and refuse a folder holding a file
//! they did not write.

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use chrono::{Local, TimeZone as _};
use gpui::{
    point, px, size, AnyWindowHandle, AppContext as _, Bounds, Entity, HeadlessAppContext,
    InputEvent as _, Keystroke, Pixels, Size,
};
use gpui_component::Root;
use serde_json::{json, Value};

use super::{
    journal, CompareView, Comparison, Handed, Link, Made, RewriteFrom, Side, State, Subject,
};
use crate::cleaner::Cleaner;
use crate::diff::Grain;
use crate::footage::{curve, curve_move, Clip};
use crate::retention::Homes;
use crate::settings::Preferences;

/// The original's file and the rewrite's, by the names the October window
/// showed (`team-update.rewritten.md` is in its banner).
const ORIGINAL: &str = "team-update.md";
const REWRITTEN: &str = "team-update.rewritten.md";

/// The main window's row the window was opened from.
const ROW: u64 = 27;

/// The time *Saved hh:mm* reads: the queue's clock, held.
const SAVED_AT: (u32, u32) = (10, 24);

/// The video's curves (`SKILLS` `curves`).
const TURN: [f32; 4] = [0.65, 0.0, 0.35, 1.0];

/// What the edit selects in the result, and what is typed over it: the
/// original's own words.
const SELECTED: &str = "initial time in 1 year";
const TYPED: &str = "first time in a year";

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/compare/team-update")
            .join(name),
    )
    .expect("the fixture")
}

/// Where the footage's files are: a folder that names nobody, since the
/// banner names the result's file and a save writes it.
fn demo_folder() -> PathBuf {
    std::env::var_os("WIPEMARK_FOOTAGE_FOLDER")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/Users/Shared/Wipemark Demo/rewrite"))
}

/// The two texts written into `folder` as the fixture has them, after what
/// a run before left is removed; a file no footage test writes is never
/// removed: the test refuses instead.
fn demo_files(folder: &Path) -> (PathBuf, PathBuf) {
    std::fs::create_dir_all(folder).expect("the demo folder");
    for entry in std::fs::read_dir(folder).expect("the demo folder, read") {
        let name = entry
            .expect("an entry")
            .file_name()
            .into_string()
            .expect("a UTF-8 name");
        if name == ORIGINAL || name == REWRITTEN {
            std::fs::remove_file(folder.join(&name)).expect("a file of a run before, removed");
        } else {
            assert!(
                name.starts_with('.'),
                "{} holds {name}, which no footage test writes: move it away first",
                folder.display()
            );
        }
    }
    let original = folder.join(ORIGINAL);
    let rewritten = folder.join(REWRITTEN);
    std::fs::write(&original, fixture("original.md")).expect("the original written");
    std::fs::write(&rewritten, fixture("rewritten.md")).expect("the rewrite written");
    (original, rewritten)
}

/// A folder of the test's own, removed with it: the Retention page's two
/// folders, used by nothing here and shown nowhere.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "wipemark-compare-footage-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).expect("scratch");
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

/// The window the video shows, ready to draw.
struct Bench {
    cx: HeadlessAppContext,
    window: AnyWindowHandle,
    view: Entity<CompareView>,
    css: Size<Pixels>,
    folder: PathBuf,
    /// `dark` or `light`, as `WIPEMARK_FOOTAGE_DARK` asks.
    theme: &'static str,
    _homes: Scratch,
}

/// Compare on the demo's team update and its rewrite, as a row's Compare
/// opens a rewrite (`Made::Rewritten` over the result's own file, with the
/// row's link): English as a window draws it, the theme asked for, reduced
/// motion, the clock held, `grain` as the Compare page would set it and
/// every other row at its default, `css` points large, the window `active`
/// or not; read, compared and laid out.
fn bench(grain: Grain, css: Size<Pixels>, active: bool) -> Bench {
    let dark = std::env::var_os("WIPEMARK_FOOTAGE_DARK").is_some();
    wipemark_i18n::init(
        &wipemark_i18n::LanguagePreference::Explicit("en-US".parse().expect("a tag")),
        wipemark_i18n::Rendering::Ui,
    );
    let at = Local
        .with_ymd_and_hms(2026, 11, 12, SAVED_AT.0, SAVED_AT.1, 0)
        .single()
        .expect("a local time");
    crate::queue::footage_tests::HELD_CLOCK.with(|held| held.set(Some(at)));
    let folder = demo_folder();
    let (original, rewritten) = demo_files(&folder);
    let scratch = Scratch::new();
    let homes = Homes {
        results: scratch.0.join("results"),
        kept: scratch.0.join("kept"),
    };

    let mut cx = crate::footage::context();
    cx.update(gpui_component::init);
    cx.update(crate::hover::init);
    cx.update(super::init);
    cx.update(crate::dialog::init);
    cx.update(|cx| cx.set_reduce_motion(true));

    let handed = Handed::Path(original);
    let subject = Subject {
        intake: Some(wipemark_intake::of(&handed)),
        handed,
        made: Made::Rewritten {
            from: RewriteFrom::File(rewritten),
            kept: None,
        },
    };
    let comparison = Comparison {
        grain,
        ..Comparison::default()
    };
    // The row hears every save and takes it, as the main window's row does.
    let link = Link {
        told: Rc::new(|_, _| true),
        home: Rc::new(|_| None),
        flushed: Rc::new(|_| None),
        tag: Rc::new(|_| journal::Tag::Id(ROW as i64)),
    };
    let slot: Rc<std::cell::RefCell<Option<Entity<CompareView>>>> = Rc::default();
    let held = slot.clone();
    let window = cx
        .open_window(css, move |window, cx| {
            let theme = if dark {
                crate::theme::ThemePreference::Dark
            } else {
                crate::theme::ThemePreference::Light
            };
            theme.apply(Some(window), cx);
            // The application's preferences and its one line of cleans,
            // made before any window as `main` makes them.
            let preferences = cx.new(|cx| Preferences::for_tests(homes, cx));
            Cleaner::shared(&preferences, cx);
            let view = cx.new(|cx| {
                CompareView::new(
                    subject,
                    comparison,
                    Some(ROW),
                    Some(link),
                    Some(preferences),
                    window,
                    cx,
                )
            });
            *held.borrow_mut() = Some(view.clone());
            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("a window");
    let window: AnyWindowHandle = window.into();
    let view = slot.take().expect("the window builder ran");
    // The active window paints its caret and selection: the edit's. A
    // still and the scroll are drawn without them, as the October still
    // was — a caret frozen mid-blink in a still reads as a mark.
    if active {
        cx.update_window(window, |_, window, _| window.activate_window())
            .expect("activated");
    }
    // The read, the comparison after its settle, the library's own wait
    // before it asks for the marks within a line, and the padding: drawn
    // until it stands.
    for _ in 0..6 {
        cx.run_until_parked();
        cx.advance_clock(Duration::from_millis(400));
        cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
            .expect("drawn");
    }
    cx.run_until_parked();
    assert!(
        cx.update(|cx| matches!(view.read(cx).state, State::Ready)),
        "the texts were not read"
    );
    Bench {
        cx,
        window,
        view,
        css,
        folder,
        theme: if dark { "dark" } else { "light" },
        _homes: scratch,
    }
}

/// `name`, and `-dark` after it in the dark theme.
fn named(name: &str, theme: &str) -> String {
    if theme == "dark" {
        format!("{name}-dark")
    } else {
        name.to_owned()
    }
}

/// The editor's own rectangles of `side`: each row it laid out (a row of
/// the buffer, across its soft wraps), its gutter, and its text column —
/// the rows' span less the gutter.
struct Pane {
    rows: Vec<Option<Bounds<Pixels>>>,
    gutter: Option<Bounds<Pixels>>,
    text: Bounds<Pixels>,
}

fn pane(view: &Entity<CompareView>, side: Side, lines: usize, cx: &mut HeadlessAppContext) -> Pane {
    cx.update(|cx| {
        let editor = view.read(cx).editor(side, cx);
        let editor = editor.read(cx);
        let rows: Vec<Option<Bounds<Pixels>>> =
            (0..lines).map(|row| editor.row_bounds(row)).collect();
        let gutter = editor.gutter_bounds();
        let span = rows
            .iter()
            .flatten()
            .next()
            .copied()
            .expect("a row laid out");
        let shown = editor.input_bounds();
        let (left, right) = match gutter {
            // The gutter on the pane's right (the original's, facing the
            // middle) or on its left (the result's).
            Some(gutter) if gutter.left() >= span.center().x => (span.left(), gutter.left()),
            Some(gutter) => (gutter.right(), span.right()),
            None => (span.left(), span.right()),
        };
        let text = Bounds::from_corners(point(left, shown.top()), point(right, shown.bottom()));
        Pane { rows, gutter, text }
    })
}

/// The byte range of the first `words` in `text`.
fn find(text: &str, words: &str) -> std::ops::Range<usize> {
    let at = text.find(words).expect("the words are there");
    at..at + words.len()
}

/// Each word of `range` in `side`'s text, by the editor's bounds of it,
/// and the names they are given (`<prefix>-<n>-<word>`).
fn words(
    view: &Entity<CompareView>,
    side: Side,
    text: &str,
    range: std::ops::Range<usize>,
    prefix: &str,
    cx: &mut HeadlessAppContext,
) -> Vec<(String, Bounds<Pixels>)> {
    let mut at = range.start;
    let mut found = Vec::new();
    for (n, word) in text[range.clone()].split(' ').enumerate() {
        let span = at..at + word.len();
        at = span.end + 1;
        let bounds = cx
            .update(|cx| {
                view.read(cx)
                    .editor(side, cx)
                    .read(cx)
                    .range_to_bounds(&span)
            })
            .expect("the word laid out");
        found.push((format!("{prefix}-{n}-{word}"), bounds));
    }
    found
}

/// Names every part of the window a still or a clip's first frame has, as
/// it stood at frame `frame`, and the editors' rows, text columns and the
/// paragraphs' bands as they stand now.
fn regions(bench: &mut Bench, clip: &mut Clip, frame: usize) -> Value {
    for (name, spot) in [
        ("banner", "compare-banner"),
        ("banner-rewritten", "compare-banner-line-0"),
        ("banner-save", "compare-banner-line-1"),
        ("reset", "compare-reset"),
        ("help", "compare-help"),
        ("original", "compare-original"),
        ("result", "compare-result"),
        ("original-caption", "compare-original-caption"),
        ("result-caption", "compare-result-caption"),
        ("original-editor", "compare-original-editor"),
        ("result-editor", "compare-result-editor"),
        ("footer", "compare-footer"),
        ("summary", "compare-summary"),
    ] {
        clip.region(name, spot, frame);
    }
    let lines = fixture("original.md").lines().count() + 1;
    let view = bench.view.clone();
    let original = pane(&view, Side::Original, lines, &mut bench.cx);
    let result = pane(&view, Side::Result, lines, &mut bench.cx);
    clip.measure("original-text", original.text);
    clip.measure("result-text", result.text);
    for (side, pane) in [("original", &original), ("result", &result)] {
        if let Some(gutter) = pane.gutter {
            clip.measure(&format!("{side}-gutter"), gutter);
        }
        for (row, bounds) in pane.rows.iter().enumerate() {
            if let Some(bounds) = bounds {
                clip.measure(&format!("{side}-rows-{}", row + 1), *bounds);
            }
        }
    }
    let clipped = |bounds: Bounds<Pixels>, column: Bounds<Pixels>| {
        Bounds::from_corners(
            point(column.left(), bounds.top()),
            point(column.right(), bounds.bottom()),
        )
    };
    if let Some(heading) = original.rows[0] {
        clip.measure("heading", clipped(heading, original.text));
    }
    if let Some(heading) = result.rows[0] {
        clip.measure("heading-result", clipped(heading, result.text));
    }
    if let Some(line) = result.rows[2] {
        clip.measure("result-line-3", clipped(line, result.text));
    }
    // Each paragraph's band: from its line's top, level on both panes, to
    // the top of the blank line after it — level too, the shorter side
    // padded — or, for the last, the lower of the two sides' bottoms.
    let css_width = bench.css.width;
    let mut bands = Vec::new();
    for paragraph in 1..=7usize {
        let row = 2 * paragraph;
        let (Some(left), Some(right)) = (original.rows[row], result.rows[row]) else {
            continue;
        };
        assert!(
            (left.top() - right.top()).abs() < px(0.5),
            "paragraph {paragraph} does not start level: {left:?} {right:?}"
        );
        let bottom = match (original.rows.get(row + 1), result.rows.get(row + 1)) {
            (Some(Some(left_next)), Some(Some(right_next))) => {
                assert!(
                    (left_next.top() - right_next.top()).abs() < px(0.5),
                    "the line after paragraph {paragraph} does not start level"
                );
                left_next.top()
            }
            _ => left.bottom().max(right.bottom()),
        };
        let band = |column: Bounds<Pixels>| {
            Bounds::from_corners(
                point(column.left(), left.top()),
                point(column.right(), bottom),
            )
        };
        let whole = Bounds::from_corners(point(px(0.0), left.top()), point(css_width, bottom));
        clip.measure(&format!("paragraph-{paragraph}"), whole);
        clip.measure(
            &format!("paragraph-{paragraph}-original"),
            band(original.text),
        );
        clip.measure(&format!("paragraph-{paragraph}-result"), band(result.text));
        bands.push(json!({
            "paragraph": paragraph,
            "line": row + 1,
            "top": f32::from(left.top()),
            "bottom": f32::from(bottom),
            "original_rows_bottom": f32::from(left.bottom()),
            "result_rows_bottom": f32::from(right.bottom()),
        }));
    }
    Value::Array(bands)
}

/// A line as the composition reads it: Fluent's isolates, which a window
/// draws as nothing, taken out.
fn plain(text: String) -> String {
    text.replace(['\u{2068}', '\u{2069}'], "")
}

/// The October window's two lines of banner, its pane captions and its
/// footer, as this window says them.
fn said(bench: &mut Bench) -> Value {
    let view = bench.view.clone();
    bench.cx.update(|cx| {
        let view = view.read(cx);
        let mut banner = super::banner_lines(view.kind);
        banner.extend(view.where_saved(cx));
        json!({
            "banner": banner.into_iter().map(plain).collect::<Vec<_>>(),
            "original": plain(wipemark_i18n::t(wipemark_i18n::Message::CompareOriginal)),
            "result": plain(wipemark_i18n::t(wipemark_i18n::Message::CompareResult)),
            "reset": plain(wipemark_i18n::t(wipemark_i18n::Message::CompareResetRewritten)),
            "summary": plain(view.summary()),
            "status": view.save_status().map(plain),
        })
    })
}

/// C1 and C2: the window at its top, marked by word and by line, one frame
/// each at the same layout — the video's board cuts its slats out of them,
/// and its glow's mask is where they differ.
#[test]
#[ignore]
fn rewrite_video_stills() {
    let out = PathBuf::from(std::env::var("WIPEMARK_FOOTAGE").expect("WIPEMARK_FOOTAGE"));
    let css = size(px(1280.0), px(720.0));
    let mut layouts = Vec::new();
    for (name, grain) in [("C1", Grain::Words), ("C2", Grain::Lines)] {
        let mut bench = bench(grain, css, false);
        let name = named(name, bench.theme);
        let mut clip = Clip::new(&out, &name, bench.window, css, &["compare-"]);
        clip.frame(&mut bench.cx);
        let bands = regions(&mut bench, &mut clip, 0);
        layouts.push(bands.clone());
        clip.note("theme", json!(bench.theme));
        clip.note("grain", json!(grain.id()));
        clip.note("bands", bands);
        clip.note("said", said(&mut bench));
        clip.note("folder", json!(bench.folder.display().to_string()));
        clip.note(
            "bands_note",
            json!(
                "paragraph-N is paragraph N's band of rows across the window, the same rows on \
                 both panes (Compare pads the shorter side level); paragraph-N-original and \
                 -result are the band cut to each pane's text column, gutter excluded; \
                 <side>-line-N is the editor's own rectangle of buffer line N (1-based, the \
                 gutter's numbers), across its soft wraps; regions outside the window's points \
                 are in frame_regions alone"
            ),
        );
        clip.finish(&format!(
            "rewrite video, Compare on team-update.md and its rewrite, marked by {}",
            grain.id()
        ));
    }
    assert_eq!(
        layouts[0], layouts[1],
        "the two grains are not laid out alike"
    );

    // The October still was 1280×800 points: the same window at that size,
    // to lay over it word for word.
    if let Some(check) = std::env::var_os("WIPEMARK_FOOTAGE_CHECK") {
        let css = size(px(1280.0), px(800.0));
        let mut bench = bench(Grain::Words, css, false);
        let path = PathBuf::from(check).join("C1-1280x800.png");
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("the check folder");
        crate::footage::shoot(&mut bench.cx, bench.window, &path);
        eprintln!("{}", path.display());
    }
}

/// The original scrolled by the wheel, one event a frame: down to its end
/// on the video's `turn` over 2.6 s, still for 0.4 s, back to the top over
/// 1.8 s — the result following, paragraph against paragraph.
#[test]
#[ignore]
fn rewrite_video_scroll() {
    const LEAD: usize = 6;
    const DOWN: usize = 78;
    const HOLD: usize = 12;
    const UP: usize = 54;
    const TAIL: usize = 6;
    let out = PathBuf::from(std::env::var("WIPEMARK_FOOTAGE").expect("WIPEMARK_FOOTAGE"));
    let css = size(px(1280.0), px(720.0));
    let mut bench = bench(Grain::Words, css, false);
    let view = bench.view.clone();
    let offset = |side: Side, cx: &mut HeadlessAppContext| -> f32 {
        cx.update(|cx| f32::from(view.read(cx).editor(side, cx).read(cx).scroll_offset().y))
    };

    // Where the wheel turns: the middle of the original's text, as a
    // person reading on scrolls under the pointer.
    let editor = crate::spot::spots()
        .get("compare-original-editor")
        .copied()
        .expect("the original's editor drawn");
    let at = point(
        editor.origin.x + editor.size.width * 0.45,
        editor.origin.y + editor.size.height * 0.55,
    );

    // How far the original scrolls: turned past its end, read, and back.
    // No frame is written meanwhile.
    let mut farthest = 0.0f32;
    for down in [px(5000.0), px(-5000.0)] {
        bench
            .cx
            .update_window(bench.window, |_, window, cx| {
                window.dispatch_event(
                    gpui::ScrollEvent {
                        position: at,
                        delta: gpui::ScrollDelta::Pixels(point(px(0.0), -down)),
                        ..Default::default()
                    }
                    .to_platform_input(),
                    cx,
                );
            })
            .expect("turned");
        for _ in 0..4 {
            bench.cx.run_until_parked();
            bench.cx.advance_clock(Duration::from_millis(100));
            bench
                .cx
                .update_window(bench.window, |_, window, cx| window.draw(cx).clear(cx))
                .expect("drawn");
        }
        if down > px(0.0) {
            farthest = -offset(Side::Original, &mut bench.cx);
        }
    }
    assert!(farthest > 100.0, "the original hardly scrolls: {farthest}");
    assert_eq!(
        offset(Side::Original, &mut bench.cx),
        0.0,
        "not back at the top"
    );
    // How far the clip scrolls: the editor's farthest leaves the last line
    // in the middle of the pane, the rest of it blank — it scrolls past
    // the end — and the text's end reaches the pane's foot after four
    // rows. Between the two: the end of the text two thirds of the way
    // down the pane, so paragraphs pass and the pane is never half empty.
    let view = bench.view.clone();
    let lines = fixture("original.md").lines().count() + 1;
    let end = pane(&view, Side::Original, lines, &mut bench.cx).rows[lines - 1]
        .expect("the last line laid out")
        .bottom();
    let to_the_end = f32::from(end - (editor.top() + editor.size.height * (2.0 / 3.0)));
    let farthest_by_wheel = farthest;
    let farthest = to_the_end.clamp(0.0, farthest_by_wheel);
    assert!(farthest > 100.0, "a scroll of {farthest} points");

    let mut clip = Clip::new(
        &out,
        &named("scroll", bench.theme),
        bench.window,
        css,
        &["compare-"],
    );
    let cx = &mut bench.cx;
    clip.point(cx, at);
    let mut offsets = Vec::new();
    let mut record = |clip: &Clip, cx: &mut HeadlessAppContext| {
        offsets.push(json!([
            clip.next() - 1,
            -offset(Side::Original, cx),
            -offset(Side::Result, cx)
        ]));
    };
    clip.mark("top");
    for _ in 0..LEAD {
        clip.frame(cx);
        record(&clip, cx);
    }
    let mut scrolled = 0.0f32;
    clip.mark("down");
    for step in 1..=DOWN {
        let to = farthest * curve(TURN, step as f32 / DOWN as f32);
        clip.wheel(cx, at, px(to - scrolled));
        scrolled = to;
        clip.frame(cx);
        record(&clip, cx);
    }
    clip.mark_at("bottom", clip.next() - 1);
    let reached = -offset(Side::Original, cx);
    assert!(
        (reached - farthest).abs() < 0.5,
        "scrolled {reached} of {farthest}"
    );
    for _ in 0..HOLD {
        clip.frame(cx);
        record(&clip, cx);
    }
    clip.mark("back");
    for step in 1..=UP {
        let to = farthest * (1.0 - curve(TURN, step as f32 / UP as f32));
        clip.wheel(cx, at, px(to - scrolled));
        scrolled = to;
        clip.frame(cx);
        record(&clip, cx);
    }
    clip.mark_at("top-again", clip.next() - 1);
    for _ in 0..TAIL {
        clip.frame(cx);
        record(&clip, cx);
    }
    let level = offsets.iter().all(|row| {
        let row = row.as_array().expect("a row");
        (row[1].as_f64().expect("a number") - row[2].as_f64().expect("a number")).abs() < 0.5
    });
    assert!(
        level,
        "the result did not stay level with the original: {offsets:?}"
    );
    clip.region("original", "compare-original", 0);
    clip.region("result", "compare-result", 0);
    clip.region("original-editor", "compare-original-editor", 0);
    clip.region("result-editor", "compare-result-editor", 0);
    clip.note("theme", json!(bench.theme));
    clip.note(
        "scroll",
        json!({
            "what": "per frame: [frame, the original's scroll in window points, the result's]",
            "to": farthest,
            "farthest_by_wheel": farthest_by_wheel,
            "to_note": "the clip scrolls until the text's end is two thirds of the way down the \
                        pane; the wheel could go on to farthest_by_wheel, past the end",
            "offsets": offsets,
        }),
    );
    clip.note(
        "pointer_note",
        json!("the pointer rests where the wheel turns, over the original's text, every frame"),
    );
    clip.finish("rewrite video, Compare: the original scrolled by the wheel, the result following");
}

/// The pixels of `bounds` (window points) in frame `frame` of `clip`'s
/// folder, as RGBA rows.
fn pixels(dir: &Path, frame: usize, bounds: Bounds<Pixels>) -> Vec<u8> {
    let file = std::fs::File::open(dir.join(format!("{frame:04}.png"))).expect("a frame");
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .expect("a PNG");
    let mut image = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut image).expect("its pixels");
    let width = info.width as usize;
    let scale = |at: Pixels| (f32::from(at) * 2.0).round() as usize;
    let (left, right) = (scale(bounds.left()), scale(bounds.right()));
    (scale(bounds.top())..scale(bounds.bottom()))
        .flat_map(|y| image[(y * width + left) * 4..(y * width + right) * 4].to_vec())
        .collect()
}

/// Fix what you would not have written: the pointer comes to the result's
/// "initial time in 1 year", drags across it, and "first time in a year"
/// is typed over it a key at a time; the marks on those words go once the
/// comparison lands again (they are the original's words now), and the
/// line under the result reads *Unsaved changes*, then — typing quiet for
/// `save::QUIET` — *Saving…* and *Saved hh:mm*, the result's own file
/// written in the demo folder.
#[test]
#[ignore]
fn rewrite_video_edit() {
    /// Frames before the pointer moves, while the video's camera comes to
    /// line 3, and the move to the drag's start.
    const REST: usize = 9;
    const APPROACH: usize = 12;
    const DRAG: usize = 18;
    const AWAY: usize = 6;
    /// A key every two or three frames: about 13 a second.
    const KEY_FRAMES: [usize; 2] = [2, 3];
    /// How long *Saving…* is held: the write itself takes no frame.
    const SAVING: usize = 6;
    const LENGTH: usize = 225;
    let out = PathBuf::from(std::env::var("WIPEMARK_FOOTAGE").expect("WIPEMARK_FOOTAGE"));
    let css = size(px(1280.0), px(720.0));
    let mut bench = bench(Grain::Words, css, true);
    let view = bench.view.clone();
    let rewritten = fixture("rewritten.md");
    let original_text = fixture("original.md");
    let selected = find(&rewritten, SELECTED);
    let name = named("edit", bench.theme);
    let mut clip = Clip::new(&out, &name, bench.window, css, &["compare-"]);
    let frames = out.join(&name).join("frames");
    let window = bench.window;
    let cx = &mut bench.cx;
    let status = |cx: &mut HeadlessAppContext| cx.update(|cx| view.read(cx).save_status());
    let bounds_of = |range: std::ops::Range<usize>, cx: &mut HeadlessAppContext| {
        cx.update(|cx| {
            view.read(cx)
                .editor(Side::Result, cx)
                .read(cx)
                .range_to_bounds(&range)
        })
        .expect("laid out")
    };

    // Where the drag starts and ends: the caret's place before "initial"
    // and after "year", half a row down.
    let start = bounds_of(selected.start..selected.start, cx);
    let end = bounds_of(selected.end..selected.end, cx);
    let from = point(start.left() + px(1.0), start.center().y);
    let to = point(end.left() - px(1.0), end.center().y);
    let row = start.size.height;
    // The pointer rests on the paragraph's last row, left of where the drag
    // starts, and after the drag goes down to the blank line under the
    // paragraph, out of the typed words' way: all of it inside the result's
    // text, where the video's camera holds on line 3.
    let blank = cx
        .update(|cx| {
            view.read(cx)
                .editor(Side::Result, cx)
                .read(cx)
                .row_bounds(3)
        })
        .expect("line 4 laid out");
    let rest = point(from.x - px(140.0), from.y + row * 3.0);
    let away = point(to.x + px(60.0), blank.center().y);
    let text = cx.update(|cx| {
        view.read(cx)
            .editor(Side::Result, cx)
            .read(cx)
            .input_bounds()
    });
    for at in [rest, from, to, away] {
        assert!(
            text.contains(&at),
            "{at:?} is outside the result's text {text:?}"
        );
    }
    let before_words = words(
        &view,
        Side::Result,
        &rewritten,
        selected.clone(),
        "selected",
        cx,
    );
    let original_words = words(
        &view,
        Side::Original,
        &original_text,
        find(&original_text, TYPED),
        "original",
        cx,
    );

    clip.point(cx, rest);
    clip.mark("rest");
    for _ in 0..REST {
        clip.frame(cx);
    }
    clip.mark("move");
    for step in 1..=APPROACH {
        let e = curve_move(step as f32 / APPROACH as f32);
        clip.point(
            cx,
            point(
                rest.x + (from.x - rest.x) * e,
                rest.y + (from.y - rest.y) * e,
            ),
        );
        clip.frame(cx);
    }
    clip.mark("drag");
    clip.press(cx);
    clip.frame(cx);
    for step in 1..=DRAG {
        let e = curve_move(step as f32 / DRAG as f32);
        clip.point(
            cx,
            point(from.x + (to.x - from.x) * e, from.y + (to.y - from.y) * e),
        );
        clip.frame(cx);
    }
    clip.release(cx);
    let chosen = cx.update(|cx| {
        let editor = view.read(cx).editor(Side::Result, cx);
        let editor = editor.read(cx);
        editor.text().to_string()[editor.selected_range()].to_owned()
    });
    assert_eq!(chosen, SELECTED, "the drag selected something else");
    clip.mark("selected");
    // The pointer out of the way of what is typed: macOS hides it while
    // keys are pressed anyway.
    for step in 1..=AWAY {
        let e = curve_move(step as f32 / AWAY as f32);
        clip.point(
            cx,
            point(to.x + (away.x - to.x) * e, to.y + (away.y - to.y) * e),
        );
        clip.frame(cx);
    }

    // The autosave's write waits for the test's word, so *Saving…* is seen.
    let (go, gate) = flume::unbounded();
    cx.update(|cx| view.update(cx, |view, _| view.saving.gate = Some(gate)));
    assert_eq!(status(cx), None, "a status before any edit");
    let typing = clip.next();
    clip.mark("typing");
    let mut states = vec![json!({ "frame": 0, "status": Value::Null })];
    for (at, key) in TYPED.chars().enumerate() {
        let key: &'static str = match key {
            ' ' => "space",
            other => Box::leak(other.to_string().into_boxed_str()),
        };
        cx.update_window(window, |_, window, cx| {
            window.dispatch_keystroke(Keystroke::parse(key).expect("a key"), cx);
        })
        .expect("typed");
        if at == 0 {
            states.push(json!({ "frame": clip.next(), "status": status(cx).map(plain) }));
            clip.mark("unsaved");
        }
        for _ in 0..KEY_FRAMES[at % 2] {
            clip.frame(cx);
        }
    }
    let typed_at = clip.next() - 1;
    clip.mark_at("typed", typed_at);
    let edited = rewritten.replacen(SELECTED, TYPED, 1);
    let now = cx.update(|cx| view.read(cx).current(cx).to_string());
    assert_eq!(now, edited, "the keys did not type the words");
    assert_eq!(
        status(cx).as_deref(),
        Some(wipemark_i18n::t(wipemark_i18n::Message::CompareStatusUnsaved).as_str())
    );

    // The marks land again, on the words now typed: the first frame the
    // typed "first" looks different from the frame before.
    let typed = find(&edited, TYPED);
    let first = bounds_of(typed.start..typed.start + "first".len(), cx);
    let mut relanded = None;
    let mut saving = None;
    let mut saved = None;
    while clip.next() < LENGTH {
        clip.frame(cx);
        let frame = clip.next() - 1;
        if relanded.is_none() && pixels(&frames, frame, first) != pixels(&frames, frame - 1, first)
        {
            relanded = Some(frame);
            clip.mark_at("relanded", frame);
        }
        let said = status(cx);
        let saying = wipemark_i18n::t(wipemark_i18n::Message::CompareStatusSaving);
        if saving.is_none() && said.as_deref() == Some(saying.as_str()) {
            saving = Some(frame);
            clip.mark_at("saving", frame);
            states.push(json!({ "frame": frame, "status": said.clone().map(plain) }));
            // Held, the write let go after it, and the window drawn as it
            // stands each frame.
            for _ in 1..SAVING {
                clip.hold(cx);
            }
            go.send(()).expect("the gate");
        } else if saving.is_some() && saved.is_none() && said != Some(saying) {
            saved = Some(frame);
            clip.mark_at("saved", frame);
            states.push(json!({ "frame": frame, "status": said.clone().map(plain) }));
        }
    }
    let relanded = relanded.expect("the marks never landed again");
    let saving = saving.expect("never saving");
    let saved = saved.expect("never saved");
    assert!(relanded < saving, "the marks landed after the save");
    let written = std::fs::read_to_string(bench.folder.join(REWRITTEN)).expect("the result");
    assert_eq!(written, edited, "the result's file is not the edit");
    assert_eq!(
        std::fs::read_to_string(bench.folder.join(ORIGINAL)).expect("the original"),
        original_text,
        "the original was written"
    );
    let saved_says = status(cx).expect("a status");
    assert_eq!(
        saved_says,
        wipemark_i18n::t_args(
            wipemark_i18n::Message::CompareStatusSaved,
            &wipemark_i18n::args!("time" => format!("{:02}:{:02}", SAVED_AT.0, SAVED_AT.1))
        )
    );

    // The regions: the window's parts at the first frame, the status line
    // in each of its states, the words before and after.
    let bands = regions(&mut bench, &mut clip, 0);
    let cx = &mut bench.cx;
    clip.region("status-unsaved", "compare-status", typed_at);
    clip.region("status-saving", "compare-status", saving);
    clip.region("status-saved", "compare-status", saved);
    for (name, bounds) in before_words.iter().chain(&original_words) {
        clip.measure(name, *bounds);
    }
    for (name, bounds) in words(&view, Side::Result, &edited, typed, "typed", cx) {
        clip.measure(&name, bounds);
    }
    clip.measure("drag-from", Bounds::new(from, size(px(0.0), row)));
    clip.measure("drag-to", Bounds::new(to, size(px(0.0), row)));
    clip.note("theme", json!(bench.theme));
    clip.note("bands", bands);
    clip.note("states", Value::Array(states));
    // macOS hides the pointer at a key and shows it again when the mouse
    // next moves, which it does not in this clip.
    clip.note(
        "pointer_hidden",
        json!({ "from": typing, "to": Value::Null, "why": "macOS hides the pointer while keys are pressed, until the mouse moves" }),
    );
    clip.note(
        "edit",
        json!({
            "selected": SELECTED,
            "typed": TYPED,
            "line": 3,
            "saved_file": REWRITTEN,
            "saved_time": format!("{:02}:{:02}", SAVED_AT.0, SAVED_AT.1),
            "saving_held_frames": SAVING,
        }),
    );
    clip.note(
        "edit_note",
        json!(
            "selected-N-word are the result's words before the edit and typed-N-word after \
             it, original-N-word the same words in the original; the pointer is drawn from \
             track (frame pixels) and is out of the typed words' way while keys are pressed \
             (macOS hides it then); Saving… is held for saving_held_frames frames, as the \
             product shows it until the write ends, which here takes no frame"
        ),
    );
    clip.finish("rewrite video, Compare: an edit to the result, autosaved beside the original");
}
