//! The main window's table drawn headless, frame by frame, as footage for
//! a video (from the probe of 2026-11-15, `dev/video/probe/probe.md`). Rows
//! arrive, are cleaned one at a time, the pointer hovers a button, a filter
//! is typed and its text selected, and the rows scroll. Every frame is
//! GPUI's own renderer on the test platform (`crate::footage`): no display,
//! no window, no screen capture.
//!
//! Run (ignored):
//!   WIPEMARK_FOOTAGE=<dir> cargo test -p wipemark-app --bin wipemark -- \
//!     --ignored queue::footage_tests --nocapture --test-threads=1
//! Optional: WIPEMARK_FOOTAGE_SIZE=1280x720 (logical points; the test
//! window's scale factor is 2), WIPEMARK_FOOTAGE_DARK=1,
//! WIPEMARK_FOOTAGE_HOVER=x,y, WIPEMARK_FOOTAGE_FILTER=x,y — the two points
//! were found by eye on the probe's frames: `HeadlessAppContext` has no
//! `debug_bounds`.
//!
//! Not yet footage to publish: the Name column shows each file's folder,
//! here the test's temporary directory, and Arrived the local wall-clock
//! time; the `Shell` around the table (toolbar, status bar) is not drawn.
//!
//! `text_video_table` is footage to publish: the text video's scene
//! (`dev/video/text/storyboard.md`, "Clean") on the demo text
//! (`fixtures/demo/reply.txt`) among three earlier rows, in a folder that
//! names nobody, at a clock held still — a clip in the video kit's format
//! (`crate::footage::Clip`). The Report's clip starts from the same table
//! (`report::footage_tests`, through [`desk`]).
//!
//!   WIPEMARK_FOOTAGE=<dir> [WIPEMARK_FOOTAGE_DARK=1] \
//!   [WIPEMARK_FOOTAGE_FOLDER=<folder>] cargo test -p wipemark-app \
//!     --bin wipemark -- --ignored queue::footage_tests::text_video_table \
//!     --exact --nocapture --test-threads=1
//!
//! It writes `<dir>/queue-dark/` or `<dir>/queue-light/`. The folder is
//! `/Users/Shared/Wipemark Demo` unless given, and holds the four texts and
//! the results their cleans write beside them. It sets the process's
//! language (English, in a window's rendering), so it runs alone.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Local, TimeZone as _};
use gpui::{
    deferred, div, point, px, size, AnyWindowHandle, AppContext as _, Context, Entity,
    HeadlessAppContext, InputEvent as _, IntoElement, Keystroke, Modifiers, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement as _, Pixels, Point, Render,
    ScrollDelta, ScrollEvent, Styled as _, Subscription, Window,
};
use gpui_component::Root;

use super::tests::Scratch;
use super::{Queue, QueueEvent};
use crate::footage::{curve_move, shoot, Clip, Cost};
use crate::report::ReportView;
use crate::retention::Homes;
use crate::settings::Preferences;

/// The fixtures that become rows, by the names the rows show.
const FIXTURES: [&str; 14] = [
    "text/bidi-control.txt",
    "text/default-ignorable.txt",
    "text/exotic-space.txt",
    "text/homoglyph.txt",
    "text/noncharacter.txt",
    "text/private-use.txt",
    "text/soft-hyphen.txt",
    "clean-parity/marked.md",
    "clean-parity/plain.md",
    "clean-parity/table.tsv",
    "clean-parity/softhyphen.txt",
    "clean-parity/homoglyph.txt",
    "text/survive-rtl-bidi.txt",
    "text/survive-homoglyph-prose.txt",
];

fn env_point(name: &str, default: (f32, f32)) -> Point<Pixels> {
    let (x, y) = std::env::var(name)
        .ok()
        .and_then(|value| {
            let (x, y) = value.split_once(',')?;
            Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
        })
        .unwrap_or(default);
    point(px(x), px(y))
}

struct Reel {
    out: PathBuf,
    window: AnyWindowHandle,
    frame: usize,
    costs: Vec<Cost>,
    started: Instant,
}

impl Reel {
    /// One frame: the work that is ready, 1/30 s of the test clock, a picture.
    fn frame(&mut self, cx: &mut HeadlessAppContext, label: &str) {
        cx.run_until_parked();
        self.next(cx, label);
    }

    /// One frame that lets at most `ticks` tasks run first, so a chain of
    /// work shows its middle.
    fn frame_ticking(&mut self, cx: &mut HeadlessAppContext, ticks: usize, label: &str) {
        for _ in 0..ticks {
            if !cx.background_executor().tick() {
                break;
            }
        }
        self.next(cx, label);
    }

    fn next(&mut self, cx: &mut HeadlessAppContext, label: &str) {
        cx.advance_clock(Duration::from_millis(33));
        let path = self.out.join(format!("queue-{:04}.png", self.frame));
        let cost = shoot(cx, self.window, &path);
        eprintln!(
            "frame {:4} {label:<10} draw {:6.1} ms  render+readback {:6.1} ms  png {:6.1} ms",
            self.frame, cost.draw, cost.render, cost.encode
        );
        self.costs.push(cost);
        self.frame += 1;
    }

    fn send(&self, cx: &mut HeadlessAppContext, input: gpui::PlatformInput) {
        cx.update_window(self.window, |_, window, cx| {
            window.dispatch_event(input, cx);
        })
        .expect("dispatched");
    }

    fn report(&self) {
        let n = self.costs.len() as f64;
        let sum = |f: fn(&Cost) -> f64| self.costs.iter().map(f).sum::<f64>() / n;
        eprintln!(
            "{} frames in {:.1} s; mean draw {:.1} ms, render+readback {:.1} ms, png {:.1} ms",
            self.costs.len(),
            self.started.elapsed().as_secs_f64(),
            sum(|c| c.draw),
            sum(|c| c.render),
            sum(|c| c.encode)
        );
    }
}

#[test]
#[ignore]
fn queue_footage() {
    let out = PathBuf::from(std::env::var("WIPEMARK_FOOTAGE").expect("WIPEMARK_FOOTAGE"));
    std::fs::create_dir_all(&out).expect("the folder");
    let (width, height) = std::env::var("WIPEMARK_FOOTAGE_SIZE")
        .ok()
        .and_then(|value| {
            let (w, h) = value.split_once('x')?;
            Some((w.parse().ok()?, h.parse().ok()?))
        })
        .unwrap_or((1280.0f32, 720.0f32));
    let dark = std::env::var_os("WIPEMARK_FOOTAGE_DARK").is_some();

    let scratch = Scratch::new("footage");
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    let files: Vec<PathBuf> = FIXTURES
        .iter()
        .map(|name| {
            let from = fixtures.join(name);
            let bytes = std::fs::read(&from).expect("fixture");
            scratch.file(
                from.file_name().and_then(|n| n.to_str()).expect("name"),
                &bytes,
            )
        })
        .collect();
    let homes = Homes {
        results: scratch.0.join("results"),
        kept: scratch.0.join("kept"),
    };

    let mut cx = crate::footage::context();
    cx.update(gpui_component::init);
    cx.update(crate::hover::init);

    let slot: Rc<std::cell::RefCell<Option<Entity<Queue>>>> = Rc::default();
    let held = slot.clone();
    let window = cx
        .open_window(size(px(width), px(height)), move |window, cx| {
            if dark {
                crate::theme::ThemePreference::Dark.apply(Some(window), cx);
            }
            let preferences = cx.new(|cx| Preferences::for_tests(homes, cx));
            let catcher = cx.new(|_| crate::drop::Catcher::detached());
            let queue = cx.new(|cx| Queue::with_catcher(preferences, catcher, None, window, cx));
            *held.borrow_mut() = Some(queue.clone());
            cx.new(|cx| Root::new(queue, window, cx))
        })
        .expect("a window");
    let window: AnyWindowHandle = window.into();
    let queue = slot.take().expect("the queue");
    // A test window is inactive until asked: carets and selections paint
    // only in the active window.
    cx.update_window(window, |_, window, _| window.activate_window())
        .expect("activated");

    let mut reel = Reel {
        out,
        window,
        frame: 0,
        costs: Vec::new(),
        started: Instant::now(),
    };

    // 1. The empty window, then the rows arriving.
    for _ in 0..4 {
        reel.frame(&mut cx, "empty");
    }
    cx.update_window(window, |_, _, cx| {
        queue.update(cx, |queue, cx| queue.hand(files.clone(), cx));
    })
    .expect("handed");
    for _ in 0..6 {
        reel.frame_ticking(&mut cx, 2, "arrive");
    }
    for _ in 0..2 {
        reel.frame(&mut cx, "arrived");
    }

    // 2. Clean all: rows go queued, cleaning, done — a few tasks a frame.
    cx.update_window(window, |_, _, cx| {
        queue.update(cx, |queue, cx| queue.clean_all(cx));
    })
    .expect("cleaning");
    // A frame drawn before the clock moves, then one task a frame.
    let path = reel.out.join(format!("queue-{:04}-hold.png", reel.frame));
    shoot(&mut cx, window, &path);
    reel.next(&mut cx, "clean-0");
    for _ in 0..30 {
        reel.frame_ticking(&mut cx, 1, "clean");
    }
    reel.frame(&mut cx, "cleaned");

    // 3. The pointer glides to a button and rests there (hover, tooltip).
    let hover = env_point("WIPEMARK_FOOTAGE_HOVER", (1248.0, 108.0));
    let from = point(px(width * 0.5), px(height * 0.8));
    for step in 0..=15 {
        let t = step as f32 / 15.0;
        let e = 1.0 - (1.0 - t).powi(3);
        let at = point(
            from.x + (hover.x - from.x) * e,
            from.y + (hover.y - from.y) * e,
        );
        reel.send(
            &mut cx,
            MouseMoveEvent {
                position: at,
                pressed_button: None,
                modifiers: Modifiers::default(),
            }
            .to_platform_input(),
        );
        reel.frame(&mut cx, "glide");
    }
    for _ in 0..20 {
        reel.frame(&mut cx, "rest");
    }

    // 4. A press on the filter field, a word typed, then the word selected
    //    by a drag across it.
    let filter = env_point("WIPEMARK_FOOTAGE_FILTER", (250.0, 20.0));
    let click = |reel: &Reel, cx: &mut HeadlessAppContext, at: Point<Pixels>| {
        reel.send(
            cx,
            MouseDownEvent {
                position: at,
                modifiers: Modifiers::default(),
                button: MouseButton::Left,
                click_count: 1,
                first_mouse: false,
            }
            .to_platform_input(),
        );
        reel.send(
            cx,
            MouseUpEvent {
                position: at,
                modifiers: Modifiers::default(),
                button: MouseButton::Left,
                click_count: 1,
            }
            .to_platform_input(),
        );
    };
    click(&reel, &mut cx, filter);
    reel.frame(&mut cx, "focus");
    for key in ["s", "o", "f", "t"] {
        cx.update_window(window, |_, window, cx| {
            window.dispatch_keystroke(Keystroke::parse(key).expect("key"), cx);
        })
        .expect("typed");
        for _ in 0..3 {
            reel.frame(&mut cx, "type");
        }
    }
    for _ in 0..30 {
        reel.frame(&mut cx, "filtered");
    }
    reel.send(
        &mut cx,
        MouseMoveEvent {
            position: point(px(186.0), filter.y),
            pressed_button: None,
            modifiers: Modifiers::default(),
        }
        .to_platform_input(),
    );
    reel.frame(&mut cx, "to-word");
    // Select by dragging from the end of the word back to its start.
    let end = point(px(186.0), filter.y);
    let start = point(px(153.0), filter.y);
    reel.send(
        &mut cx,
        MouseDownEvent {
            position: end,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: 1,
            first_mouse: false,
        }
        .to_platform_input(),
    );
    for step in 1..=10 {
        let t = step as f32 / 10.0;
        let at = point(end.x + (start.x - end.x) * t, end.y);
        reel.send(
            &mut cx,
            MouseMoveEvent {
                position: at,
                pressed_button: Some(MouseButton::Left),
                modifiers: Modifiers::default(),
            }
            .to_platform_input(),
        );
        reel.frame(&mut cx, "select");
    }
    reel.send(
        &mut cx,
        MouseUpEvent {
            position: start,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: 1,
        }
        .to_platform_input(),
    );
    for _ in 0..4 {
        reel.frame(&mut cx, "selected");
    }
    // A double click on the word.
    for count in [1, 2] {
        reel.send(
            &mut cx,
            MouseDownEvent {
                position: point(px(170.0), filter.y),
                modifiers: Modifiers::default(),
                button: MouseButton::Left,
                click_count: count,
                first_mouse: false,
            }
            .to_platform_input(),
        );
        reel.send(
            &mut cx,
            MouseUpEvent {
                position: point(px(170.0), filter.y),
                modifiers: Modifiers::default(),
                button: MouseButton::Left,
                click_count: count,
            }
            .to_platform_input(),
        );
    }
    for _ in 0..4 {
        reel.frame(&mut cx, "dblclick");
    }
    // Select all by the keyboard, shown, then deleted.
    cx.update_window(window, |_, window, cx| {
        window.dispatch_keystroke(Keystroke::parse("left").expect("key"), cx);
        let all = Keystroke::parse_with_os("secondary-a", cx.platform_info().os);
        window.dispatch_keystroke(all.expect("key"), cx);
    })
    .expect("selected all");
    for _ in 0..4 {
        reel.frame(&mut cx, "cmd-a");
    }
    cx.update_window(window, |_, window, cx| {
        window.dispatch_keystroke(Keystroke::parse("backspace").expect("key"), cx);
    })
    .expect("cleared");
    for _ in 0..4 {
        reel.frame(&mut cx, "cleared");
    }

    // 5. The rows scrolled by the wheel, a little each frame.
    let over = point(px(width * 0.5), px(height * 0.6));
    reel.send(
        &mut cx,
        MouseMoveEvent {
            position: over,
            pressed_button: None,
            modifiers: Modifiers::default(),
        }
        .to_platform_input(),
    );
    for _ in 0..30 {
        reel.send(
            &mut cx,
            ScrollEvent {
                position: over,
                delta: ScrollDelta::Pixels(point(px(0.0), px(-6.0))),
                ..Default::default()
            }
            .to_platform_input(),
        );
        reel.frame(&mut cx, "scroll");
    }

    reel.report();
}

thread_local! {
    /// What `super::local_now` answers while it is set: a footage test's
    /// clock, held still so that Arrived reads the same on every run.
    pub(crate) static HELD_CLOCK: Cell<Option<DateTime<Local>>> = const { Cell::new(None) };
}

/// Holds the table's clock at `hour:minute`, local time, on a Thursday.
fn hold_clock((hour, minute): (u32, u32)) {
    let at = Local
        .with_ymd_and_hms(2026, 11, 12, hour, minute, 0)
        .single()
        .expect("a local time");
    HELD_CLOCK.with(|held| held.set(Some(at)));
}

/// The rows before the demo text, and when each arrived: one with nothing
/// to find, one with a zero-width space, one with a soft hyphen.
const EARLIER: [(&str, &str, (u32, u32)); 3] = [
    (
        "agenda.txt",
        "Agenda for Thursday\n\n1. Launch checklist\n2. Payment section\n3. Release notes\n",
        (9, 12),
    ),
    (
        "notes.md",
        "# Notes\n\nKeep the demo short\u{200B} and show the report at the end.\n",
        (9, 20),
    ),
    (
        "summary.txt",
        "The pilot ran for six weeks; the numbers held and the team is re\u{AD}ady to ship.\n",
        (9, 27),
    ),
];

/// When the demo text arrives, and the time the table is drawn at.
const REPLY_AT: (u32, u32) = (9, 41);

/// The main window's size the probe settled on: every column fits, and the
/// frames are 2560×1440.
pub(crate) fn desk_size() -> gpui::Size<Pixels> {
    size(px(1280.0), px(720.0))
}

/// Where the footage's files are: a folder that names nobody, since the
/// Name column shows it and the Report prints the result's full path.
pub(crate) fn demo_folder() -> PathBuf {
    std::env::var_os("WIPEMARK_FOOTAGE_FOLDER")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/Users/Shared/Wipemark Demo"))
}

/// The four texts written into `folder`, the demo text last, after what a
/// run before left of them — the texts and their results — is removed. A
/// file no footage test writes is never removed: the test refuses instead.
fn demo_files(folder: &Path) -> Vec<PathBuf> {
    std::fs::create_dir_all(folder).expect("the demo folder");
    let names: Vec<&str> = EARLIER
        .iter()
        .map(|(name, ..)| *name)
        .chain(["reply.txt"])
        .collect();
    let ours = |file: &str| {
        names.iter().any(|name| {
            let (stem, _) = name.rsplit_once('.').expect("an extension");
            file == *name || file.starts_with(&format!("{stem}.cleaned"))
        })
    };
    for entry in std::fs::read_dir(folder).expect("the demo folder, read") {
        let name = entry
            .expect("an entry")
            .file_name()
            .into_string()
            .expect("a UTF-8 name");
        if ours(&name) {
            std::fs::remove_file(folder.join(&name)).expect("a file of a run before, removed");
        } else {
            assert!(
                name.starts_with('.'),
                "{} holds {name}, which no footage test writes: move it away first",
                folder.display()
            );
        }
    }
    let reply =
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/demo/reply.txt"))
            .expect("the demo text");
    EARLIER
        .iter()
        .map(|(name, text, _)| (*name, text.as_bytes().to_vec()))
        .chain([("reply.txt", reply)])
        .map(|(name, bytes)| {
            let path = folder.join(name);
            std::fs::write(&path, bytes).expect("a text written");
            path
        })
        .collect()
}

/// The main window's content as the shell lays it, less its toolbar and
/// status bar: the table, and a row's Report dialog over it — opened as
/// the shell opens it when a row's Report… asks (`Shell::open_report`).
pub(crate) struct Main {
    queue: Entity<Queue>,
    report: Option<(Entity<ReportView>, Subscription)>,
    _asked: Subscription,
}

impl Main {
    #[allow(
        clippy::needless_pass_by_ref_mut,
        reason = "GPUI view-constructor signature"
    )]
    fn new(queue: Entity<Queue>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let asked = cx.subscribe_in(
            &queue,
            window,
            |main, queue, event: &QueueEvent, window, cx| {
                let QueueEvent::Report(id) = event else {
                    return;
                };
                let Some((intake, outcome, edited)) = queue.read(cx).report_of(*id) else {
                    return;
                };
                let view = cx.new(|cx| ReportView::new(&intake, &outcome, edited, window, cx));
                let closed = cx.subscribe(
                    &view,
                    |main: &mut Self, _, _: &crate::dialog::Answer, cx| {
                        main.report = None;
                        cx.notify();
                    },
                );
                main.report = Some((view, closed));
                cx.notify();
            },
        );
        Self {
            queue,
            report: None,
            _asked: asked,
        }
    }

    /// The Report dialog, while it is open.
    pub(crate) fn report(&self) -> Option<Entity<ReportView>> {
        self.report.as_ref().map(|(view, _)| view.clone())
    }
}

impl Render for Main {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(self.queue.clone())
            .children(self.report.as_ref().map(|(report, _)| {
                deferred(report.clone())
                    .with_priority(crate::dialog::MODAL_PRIORITY)
                    .into_any_element()
            }))
    }
}

/// The table the text video shows, ready to draw: the three earlier rows
/// arrived and cleaned, the demo text arrived last and not started.
pub(crate) struct Desk {
    pub(crate) cx: HeadlessAppContext,
    pub(crate) window: AnyWindowHandle,
    pub(crate) queue: Entity<Queue>,
    pub(crate) main: Entity<Main>,
    /// Each row's id, by its file's name, in the order they arrived.
    pub(crate) rows: Vec<(String, u64)>,
    /// The demo text's row.
    pub(crate) reply: u64,
    pub(crate) folder: PathBuf,
    /// `dark` or `light`, as `WIPEMARK_FOOTAGE_DARK` asks.
    pub(crate) theme: &'static str,
    /// The Retention page's two folders, used by nothing at its defaults
    /// and shown nowhere.
    _homes: Scratch,
}

/// The text video's table: English as a window draws it, the theme
/// `WIPEMARK_FOOTAGE_DARK` asks for, reduced motion, the clock held still,
/// the four texts in the demo folder.
pub(crate) fn desk() -> Desk {
    let dark = std::env::var_os("WIPEMARK_FOOTAGE_DARK").is_some();
    wipemark_i18n::init(
        &wipemark_i18n::LanguagePreference::Explicit("en-US".parse().expect("a tag")),
        wipemark_i18n::Rendering::Ui,
    );
    let folder = demo_folder();
    let files = demo_files(&folder);
    let scratch = Scratch::new("footage-homes");
    let homes = Homes {
        results: scratch.0.join("results"),
        kept: scratch.0.join("kept"),
    };

    let mut cx = crate::footage::context();
    cx.update(gpui_component::init);
    cx.update(crate::hover::init);
    cx.update(|cx| cx.set_reduce_motion(true));

    type Built = (Entity<Queue>, Entity<Main>);
    let slot: Rc<std::cell::RefCell<Option<Built>>> = Rc::default();
    let held = slot.clone();
    let window = cx
        .open_window(desk_size(), move |window, cx| {
            let theme = if dark {
                crate::theme::ThemePreference::Dark
            } else {
                crate::theme::ThemePreference::Light
            };
            theme.apply(Some(window), cx);
            let preferences = cx.new(|cx| Preferences::for_tests(homes, cx));
            let catcher = cx.new(|_| crate::drop::Catcher::detached());
            let queue = cx.new(|cx| Queue::with_catcher(preferences, catcher, None, window, cx));
            let main = cx.new(|cx| Main::new(queue.clone(), window, cx));
            *held.borrow_mut() = Some((queue, main.clone()));
            cx.new(|cx| Root::new(main, window, cx))
        })
        .expect("a window");
    let window: AnyWindowHandle = window.into();
    let (queue, main) = slot.take().expect("the window builder ran");
    // The main window is the active one: its focus ring and hover are drawn.
    cx.update_window(window, |_, window, _| window.activate_window())
        .expect("activated");

    let times = EARLIER.iter().map(|(.., at)| *at).chain([REPLY_AT]);
    let mut rows = Vec::new();
    for (path, at) in files.iter().zip(times) {
        hold_clock(at);
        queue.update(&mut cx, |queue, cx| queue.hand(vec![path.clone()], cx));
        cx.run_until_parked();
        let id = cx
            .update(|cx| queue.read(cx).ids().last().copied())
            .expect("a row");
        let name = path.file_name().and_then(|n| n.to_str()).expect("a name");
        rows.push((name.to_owned(), id));
    }
    let earlier: Vec<u64> = rows[..EARLIER.len()].iter().map(|(_, id)| *id).collect();
    queue.update(&mut cx, |queue, cx| queue.clean(&earlier, cx));
    cx.run_until_parked();
    let reply = rows.last().expect("the demo text's row").1;
    cx.update(|cx| {
        let queue = queue.read(cx);
        for id in &earlier {
            assert!(
                matches!(queue.status_of(*id), Some(super::Status::Done(_))),
                "row {id} is not done: {:?}",
                queue.status_words(*id)
            );
        }
        assert!(matches!(
            queue.status_of(reply),
            Some(super::Status::Waiting)
        ));
    });

    Desk {
        cx,
        window,
        queue,
        main,
        rows,
        reply,
        folder,
        theme: if dark { "dark" } else { "light" },
        _homes: scratch,
    }
}

/// The text video's scene "Clean" (10.6–15.4 s of the storyboard): the
/// table with `reply.txt` on top, not started; the pointer comes to its
/// Clean on `CURVE.move`, rests (the row's hover), presses; the row reads
/// *Cleaning…* — the frames drawn before the clean may run, as the product
/// shows it until the clean is done — and then *Cleaned*, held to 4.8 s.
#[test]
#[ignore]
fn text_video_table() {
    let out = PathBuf::from(std::env::var("WIPEMARK_FOOTAGE").expect("WIPEMARK_FOOTAGE"));
    let mut desk = desk();
    let id = desk.reply;
    let queue = desk.queue.clone();
    let mut clip = Clip::new(
        &out,
        &format!("queue-{}", desk.theme),
        desk.window,
        desk_size(),
        &["queue-", "row-clean-", "row-rewrite-"],
    );
    let cx = &mut desk.cx;
    let status = |cx: &mut HeadlessAppContext| {
        cx.update(|cx| queue.read(cx).status_of(id).cloned())
            .expect("the row")
    };

    // 0.0–0.3 s: the table as it stands; the pointer rests on the table's
    // empty part, below the rows.
    let from = point(px(900.0), px(620.0));
    clip.point(cx, from);
    clip.mark("not-started");
    for _ in 0..9 {
        clip.frame(cx);
    }
    // 0.3–0.8 s: to the row's Clean.
    let to = clip.spot(&format!("row-clean-{id}")).center();
    clip.mark("move");
    for step in 1..=15 {
        let e = curve_move(step as f32 / 15.0);
        clip.point(
            cx,
            point(from.x + (to.x - from.x) * e, from.y + (to.y - from.y) * e),
        );
        clip.frame(cx);
    }
    clip.mark_at("over-clean", clip.next() - 1);
    // 0.8–1.0 s: resting on it.
    for _ in 0..6 {
        clip.frame(cx);
    }
    // 1.0 s: the press; the click is the release.
    assert!(matches!(status(cx), super::Status::Waiting));
    clip.mark("press");
    clip.press(cx);
    clip.frame(cx);
    clip.release(cx);
    assert!(
        matches!(status(cx), super::Status::Cleaning),
        "the press did not start the clean"
    );
    clip.mark("cleaning");
    let cleaning = clip.next();
    // To 1.4 s, the storyboard's *Cleaned*.
    for _ in 0..11 {
        clip.hold(cx);
    }
    clip.mark("cleaned");
    let cleaned = clip.next();
    clip.frame(cx);
    match status(cx) {
        super::Status::Done(outcome) => assert_eq!(outcome.verdict.id(), "cleaned"),
        other => panic!("not cleaned in one frame: {other:?}"),
    }
    // Then the pointer goes back where it came from, before the button's
    // tooltip — "Clean", the label the hover began on — would come up over
    // the row below and the badge.
    clip.mark("leave");
    for step in 1..=15 {
        let e = curve_move(step as f32 / 15.0);
        clip.point(
            cx,
            point(to.x + (from.x - to.x) * e, to.y + (from.y - to.y) * e),
        );
        clip.frame(cx);
    }
    while clip.next() < 144 {
        clip.frame(cx);
    }

    for (name, spot, frame) in [
        ("row", "queue-row", 0),
        ("preview", "queue-preview", 0),
        ("name", "queue-name", 0),
        ("status", "queue-status", 0),
        ("clean", "row-clean", 0),
        ("actions", "queue-actions", 0),
        ("status-cleaning", "queue-status", cleaning),
        ("status-cleaned", "queue-status", cleaned),
        ("clean-again", "row-clean", cleaned),
    ] {
        clip.region(name, &format!("{spot}-{id}"), frame);
    }
    let mut rows = Vec::new();
    for (file, row) in &desk.rows {
        let region = format!("row-{file}");
        clip.region(&region, &format!("queue-row-{row}"), 0);
        rows.push(serde_json::json!({ "file": file, "id": row, "region": region }));
    }
    clip.note("theme", serde_json::json!(desk.theme));
    clip.note(
        "folder",
        serde_json::json!(desk.folder.display().to_string()),
    );
    clip.note("rows", serde_json::Value::Array(rows));
    clip.note(
        "states",
        serde_json::json!([
            { "frame": 0, "status": "Not started" },
            { "frame": cleaning, "status": "Cleaning…" },
            { "frame": cleaned, "status": "Cleaned" },
        ]),
    );
    clip.finish("text video, Clean: reply.txt among three earlier text rows");
}
