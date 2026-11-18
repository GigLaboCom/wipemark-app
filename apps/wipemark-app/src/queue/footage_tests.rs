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

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    point, px, size, AnyWindowHandle, AppContext as _, Entity, HeadlessAppContext, InputEvent as _,
    Keystroke, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point,
    ScrollDelta, ScrollEvent,
};
use gpui_component::Root;

use super::tests::Scratch;
use super::Queue;
use crate::footage::{shoot, Cost};
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
