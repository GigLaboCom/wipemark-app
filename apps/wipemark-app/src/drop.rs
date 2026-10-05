//! A place on screen that accepts what is dragged onto it.
//!
//! Epic **E6**. One entity and one decorator, so that any view in this
//! application can take a drop without knowing anything about
//! pasteboards, Objective-C or the difference between the three
//! platforms: build a [`Catcher`] when the window opens, wrap an
//! element in [`zone`], and read [`Catcher::caught`] when it says
//! something landed. The panel is the first caller; E7's workspace
//! windows and the main window are the next two, and neither of them
//! should have to learn any of this again.
//!
//! # What it accepts
//!
//! Text, images and files — everything a desktop can put on a
//! pasteboard. Recognising *which* is not this module's job and is
//! deliberately not in this crate at all: `wipemark-intake` is a leaf
//! library with no dependencies, so the CLI and the MCP server answer
//! the same question with the same code. What arrives here is
//! [`Handed`]; what comes out is [`Intake`], and the surface renders
//! it.
//!
//! # Two roads in, and why both are kept
//!
//! On macOS, `crate::pasteboard` installs a dragging destination and
//! answers **every** drop on the window, because GPUI's own window
//! refuses anything that is not a file (see that module for the line
//! that does it). Everywhere else, GPUI's [`ExternalPaths`] drag is all
//! there is, and it carries files.
//!
//! So both are wired, and only one of them fires: a window whose
//! destination installed never reports an `ExternalPaths` drop, because
//! a registered view is found before the window behind it is asked. The
//! `ExternalPaths` half is therefore not dead code on macOS either — it
//! is what the window falls back to if the destination could not be
//! installed, which is the difference between "files still work" and
//! "nothing works".
//!
//! # Nothing blocks the window
//!
//! Recognising a dropped file reads the front of it, and a file dropped
//! on a window can live on a network volume that takes a second to
//! answer. Every [`Handed`] is therefore examined on the background
//! executor and the answer comes back to the entity, which is the same
//! bargain every long operation in this application makes.
//!
//! # Two readers, two questions
//!
//! The panel asks "what was the last thing dropped here" and paints it;
//! the main window's queue asks "what has arrived" and keeps every
//! answer. Both are served, and by different halves: [`Catcher::caught`]
//! is the last drop, overtaken by the next one the way a screen is; the
//! [`Landed`] event carries *its own* drop, and every drop fires one —
//! a second drop while the first is still being read off a slow disk
//! does not make the first one vanish from a list, it only decides
//! which of the two the panel is showing.
//!
//! What is kept is the [`Arrival`]: what was handed over *and* what it
//! turned out to be, because a queue that wants to show the first lines
//! of a note, or the picture in a screenshot, needs the thing and not
//! only its description.

use std::sync::Arc;

use gpui::prelude::*;
use gpui::{App, Div, Entity, EventEmitter, ExternalPaths, Task, Window};
use gpui_component::ActiveTheme;
use wipemark_intake::{Handed, Intake};

/// One thing that arrived: what was handed over, and what it turned out
/// to be.
///
/// The two halves are kept together on purpose. The [`Intake`] is the
/// description — kind, format, evidence — and is what every surface
/// reads first; the [`Handed`] is the thing itself, and is what a
/// preview, and from E1 the scrubber, reads next. A `Handed::Bytes` can
/// be a screenshot dragged out of a browser, megabytes with no file
/// behind them, and this is the only copy anybody holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arrival {
    pub handed: Handed,
    pub intake: Intake,
}

/// Something landed, and here is what it was.
///
/// One event per drop, carrying that drop and no other — see the module
/// docs for why that is not the same as [`Catcher::caught`]. A view that
/// only paints the last drop can observe the entity instead; this is for
/// the ones that have to *do* something with each, which the queue does
/// today and from E1 every one of them will.
pub struct Landed(pub Arc<[Arrival]>);

/// A window's drop target.
///
/// One per window. The pasteboard side is keyed by the window — a
/// second catcher on the same window would be told about the same drop
/// twice, and neither would know which half of the window it was over.
/// The position a drop happened at is not yet carried, because nothing
/// asks: the panel is one zone, and so is every window planned for E7.
pub struct Catcher {
    /// Is something being held over the window right now.
    over: bool,
    /// What the last drop turned out to be — `None` until there has
    /// been one, and `Some([])` for a drop that carried nothing this
    /// machine could read. The two are different answers and the
    /// surface says different things about them.
    caught: Option<Arc<[Arrival]>>,
    /// Which examination is the current one. A second drop while the
    /// first is still being read overtakes it — the same counter the
    /// MCP restart, the credential lookup and the panel's settling all
    /// use.
    reads: u64,
    /// The pasteboard's deliveries, for as long as this entity lives.
    /// Dropping it unregisters the window.
    _deliveries: Option<Task<()>>,
}

impl EventEmitter<Landed> for Catcher {}

impl Catcher {
    /// Start accepting drops on this window.
    ///
    /// Call it while the window is being built — the destination has to
    /// be installed before the first drag, and a window is not dragged
    /// onto in the frame it opens in.
    pub fn new(window: &Window, cx: &Context<Self>) -> Self {
        Self {
            over: false,
            caught: None,
            reads: 0,
            _deliveries: deliveries(window, cx),
        }
    }

    /// A catcher with no platform behind it: drops reach it only through
    /// [`Catcher::land`]. For a test window, which GPUI's test platform
    /// does not back with a real one — asking it for its native handle,
    /// as the macOS destination does, panics.
    #[cfg(test)]
    pub fn detached() -> Self {
        Self {
            over: false,
            caught: None,
            reads: 0,
            _deliveries: None,
        }
    }

    /// Whether something is being held over the window.
    pub fn over(&self) -> bool {
        self.over
    }

    /// What the last drop turned out to be, in the order it arrived.
    /// `None` if nothing has been dropped here yet.
    pub fn caught(&self) -> Option<&[Arrival]> {
        self.caught.as_deref()
    }

    /// The highlight, on or off.
    fn hovering(&mut self, over: bool, cx: &mut Context<Self>) {
        if self.over == over {
            return;
        }
        self.over = over;
        cx.notify();
    }

    /// Something arrived: find out what it is, away from the thread
    /// that draws, and say so.
    ///
    /// A drop is the usual caller and not the only one. A file picker's
    /// answer is a list of paths that somebody chose rather than dragged,
    /// and it goes down this same road one step later — so "Import" and
    /// a drop cannot recognise a file two different ways, and a surface
    /// listening for [`Landed`] hears both.
    pub fn land(&mut self, handed: Vec<Handed>, cx: &mut Context<Self>) {
        self.hovering(false, cx);
        self.reads += 1;
        let mine = self.reads;

        cx.spawn(async move |catcher, cx| {
            let read: Arc<[Arrival]> = cx
                .background_executor()
                .spawn(async move {
                    handed
                        .into_iter()
                        .map(|handed| Arrival {
                            intake: wipemark_intake::of(&handed),
                            handed,
                        })
                        .collect()
                })
                .await;

            catcher
                .update(cx, |catcher, cx| {
                    // The last drop is what the panel shows, so a read
                    // overtaken by a later drop does not replace it —
                    // but it still *landed*, and the queue keeps every
                    // one. Two readers, two questions.
                    if catcher.reads == mine {
                        catcher.caught = Some(read.clone());
                    }
                    cx.emit(Landed(read));
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }
}

/// The pasteboard's deliveries, on the platforms that have them.
#[cfg(target_os = "macos")]
fn deliveries(window: &Window, cx: &Context<Catcher>) -> Option<Task<()>> {
    use crate::pasteboard::{self, Delivery};

    let window = pasteboard::identity(window)?;
    let deliveries = pasteboard::watch(window);

    Some(cx.spawn(async move |catcher, cx| {
        while let Ok(delivery) = deliveries.recv_async().await {
            let told = catcher.update(cx, |catcher, cx| match delivery {
                Delivery::Entered => catcher.hovering(true, cx),
                Delivery::Exited => catcher.hovering(false, cx),
                Delivery::Dropped(handed) => catcher.land(handed, cx),
            });
            if told.is_err() {
                // The window is gone, and so is everything that would
                // have been told about this.
                break;
            }
        }
    }))
}

#[cfg(not(target_os = "macos"))]
fn deliveries(_window: &Window, _cx: &Context<Catcher>) -> Option<Task<()>> {
    // E10. GPUI's own file drops still arrive through [`zone`].
    None
}

/// Install the platform's dragging destination on this window.
///
/// Separate from [`Catcher::new`] only on macOS, where it is the half
/// that cannot be undone: a view inserted into the tree stays there for
/// the life of the window.
#[cfg(target_os = "macos")]
pub fn accept(window: &Window) {
    if !crate::pasteboard::attach(window) {
        tracing::warn!("this window will only accept files");
    }
}

#[cfg(not(target_os = "macos"))]
pub fn accept(_window: &Window) {}

/// Wrap an element so that whatever is dropped on it lands in
/// `catcher`.
///
/// The highlight is included rather than left to the caller, because a
/// drop zone that does not say it is one is a window people drag things
/// at and give up on. A caller that wants a different look reads
/// [`Catcher::over`] and paints it itself.
pub fn zone(element: Div, catcher: &Entity<Catcher>, cx: &App) -> Div {
    let over = catcher.read(cx).over;
    let landing = catcher.clone();

    element
        .when(over, |element| {
            element
                .bg(cx.theme().accent)
                .border_color(cx.theme().primary)
        })
        // GPUI's own file drop, which is the whole of the story off
        // macOS and the fallback on it — see the module docs.
        .drag_over::<ExternalPaths>(|style, _, _, cx| style.bg(cx.theme().accent))
        .on_drop::<ExternalPaths>(move |paths, _, cx| {
            let handed = paths.paths().iter().cloned().map(Handed::Path).collect();
            landing.update(cx, |catcher, cx| catcher.land(handed, cx));
        })
}

/// Bytes, as somebody reads them off a window.
///
/// Kilobytes are in here and are not in `models::bytes_label`, which is
/// the same question asked about a different subject: a model's weights
/// are never measured in kilobytes and a "0.0 GB" beside a tokenizer
/// reads as broken, while a dropped note is a couple of kilobytes and
/// "3412 B" is a number a person has to count the digits of.
pub fn size_label(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "kB", "MB", "GB", "TB"];
    if bytes < 1000 {
        return format!("{bytes} {}", UNITS[0]);
    }
    let mut value = bytes as f64 / 1000.0;
    let mut unit = 1;
    while value >= 1000.0 && unit + 1 < UNITS.len() {
        value /= 1000.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use gpui::{AppContext as _, Entity, TestAppContext};
    use wipemark_intake::{Handed, Kind};

    use super::{Catcher, Landed};

    /// A catcher with no window behind it: everything below is about
    /// what happens *after* a delivery, and the delivery is handed in
    /// by the test.
    fn loose(cx: &mut TestAppContext) -> Entity<Catcher> {
        cx.new(|_| Catcher {
            over: false,
            caught: None,
            reads: 0,
            _deliveries: None,
        })
    }

    /// The examination happens off the drawing thread and lands back on
    /// it: what a view reads afterwards is the answer, not the raw
    /// bytes it was handed.
    #[gpui::test]
    fn what_lands_is_examined_and_kept(cx: &mut TestAppContext) {
        let catcher = loose(cx);

        catcher.update(cx, |catcher: &mut Catcher, cx| {
            catcher.land(
                vec![
                    Handed::Text("a line of prose".to_owned()),
                    Handed::Bytes {
                        name: Some("shot.png".to_owned()),
                        bytes: b"\x89PNG\r\n\x1a\n".to_vec(),
                    },
                ],
                cx,
            );
        });
        cx.run_until_parked();

        catcher.read_with(cx, |catcher: &Catcher, _| {
            let caught = catcher.caught().expect("a drop was made");
            let kinds: Vec<Kind> = caught.iter().map(|arrival| arrival.intake.kind).collect();
            assert_eq!(kinds, vec![Kind::Text, Kind::Image]);
            // The thing itself is kept beside its description: a
            // preview reads the note, not the fact that it is a note.
            assert_eq!(
                caught[0].handed,
                Handed::Text("a line of prose".to_owned()),
                "what was handed over did not survive being recognised"
            );
        });
    }

    /// A second drop while the first is still being read off a slow
    /// disk is the answer; the first one is not allowed to arrive after
    /// it and overwrite what is on screen.
    #[gpui::test]
    fn a_second_drop_overtakes_the_first(cx: &mut TestAppContext) {
        let catcher = loose(cx);

        catcher.update(cx, |catcher: &mut Catcher, cx| {
            catcher.land(vec![Handed::Text("first".to_owned())], cx);
            catcher.land(
                vec![Handed::Bytes {
                    name: None,
                    bytes: b"GIF89a".to_vec(),
                }],
                cx,
            );
        });
        cx.run_until_parked();

        catcher.read_with(cx, |catcher: &Catcher, _| {
            let caught = catcher.caught().expect("a drop was made");
            assert_eq!(caught.len(), 1);
            assert_eq!(caught[0].intake.kind, Kind::Image);
        });
    }

    /// The other reader. A queue keeps every drop, so the one that was
    /// overtaken on screen still lands as its own event — dropping two
    /// things quickly must not lose the first of them from a list.
    #[gpui::test]
    fn every_drop_lands_even_when_overtaken(cx: &mut TestAppContext) {
        let catcher = loose(cx);
        let landed: Rc<RefCell<Vec<Vec<Kind>>>> = Rc::default();
        let heard = landed.clone();
        cx.update(|cx| {
            cx.subscribe(&catcher, move |_, Landed(arrivals), _| {
                heard
                    .borrow_mut()
                    .push(arrivals.iter().map(|arrival| arrival.intake.kind).collect());
            })
            .detach();
        });

        catcher.update(cx, |catcher: &mut Catcher, cx| {
            catcher.land(vec![Handed::Text("first".to_owned())], cx);
            catcher.land(
                vec![Handed::Bytes {
                    name: None,
                    bytes: b"GIF89a".to_vec(),
                }],
                cx,
            );
        });
        cx.run_until_parked();

        let mut heard = landed.borrow().clone();
        heard.sort();
        assert_eq!(
            heard,
            vec![vec![Kind::Text], vec![Kind::Image]],
            "a drop went missing between the pasteboard and the queue"
        );
    }

    /// The unit that this application has and the Models page does
    /// not: a dropped note is two kilobytes, and a window that says
    /// "2048 B" is asking somebody to count digits.
    #[test]
    fn a_size_is_read_rather_than_counted() {
        assert_eq!(super::size_label(0), "0 B");
        assert_eq!(super::size_label(812), "812 B");
        assert_eq!(super::size_label(2_048), "2.0 kB");
        assert_eq!(super::size_label(1_240_000), "1.2 MB");
        assert_eq!(super::size_label(7_432_229_248), "7.4 GB");
    }

    /// The highlight is a state, not an event: it goes on while
    /// something is held over the window and comes off when it lands,
    /// whatever the drop turned out to contain.
    #[gpui::test]
    fn the_highlight_comes_off_when_it_lands(cx: &mut TestAppContext) {
        let catcher = loose(cx);

        catcher.update(cx, |catcher: &mut Catcher, cx| {
            assert!(catcher.caught().is_none(), "nothing has been dropped yet");
            catcher.hovering(true, cx);
            assert!(catcher.over());
            catcher.land(Vec::new(), cx);
            assert!(!catcher.over());
        });
        cx.run_until_parked();

        // A drop that carried nothing is not the same answer as never
        // having been dropped on, and the panel says different things
        // about the two.
        catcher.read_with(cx, |catcher: &Catcher, _| {
            assert_eq!(catcher.caught(), Some(&[][..]));
        });
    }
}
