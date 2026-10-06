//! The panel — the window you summon.
//!
//! Epic **E6**. The third window of this application, and the only one
//! that is *placed* rather than opened beside something: the main
//! window is where the work happens and the Settings window opens over
//! whatever asked for it, but the panel appears on its own, from the
//! menu bar or the command line, wherever the Placement page says.
//!
//! # No titlebar, and what stands in for one
//!
//! A summoned window is not dragged into position, it *arrives* in
//! position — which is what the six zones on the Placement page decide.
//! A titlebar would be a second answer to that question, so there is
//! none: `WindowOptions::titlebar: None`, and [`afloat`] replaces the
//! style mask GPUI builds for such a window with one that has no
//! `Titled` bit at all. That last part is not tidiness. A titled
//! window with a full-size content view has a *title bar region* over
//! its whole top, and a double-click there zooms — a panel whose entire
//! point is that it is small and where you put it, maximized by a
//! mis-click.
//!
//! What stands in for the titlebar is the window itself:
//! `movableByWindowBackground` makes the whole panel its own handle,
//! and the `Resizable` bit lets any edge be pulled. Both are deliberate
//! — where a window is *put by hand* is the most specific answer there
//! is, and `Answer::Manual` is how it beats a cell. The
//! [`help`] popover behind the info button is where all of that is
//! written down, because none of it is visible.
//!
//! With no titlebar there is also no chrome: GPUI is handed a content
//! rectangle and, for a window whose content view fills the frame, the
//! two are the same rectangle. That is why [`show`] places it with a
//! chrome of zero.
//!
//! # It floats
//!
//! `NSFloatingWindowLevel`, `hidesOnDeactivate: false` and a collection
//! behaviour that joins every space: a window you call up over somebody
//! else's document has to be *over* it, and has to still be there when
//! the application it belongs to is not in front. [`afloat`] is where
//! those three live, and each is a call GPUI does not expose.
//!
//! # What it contains
//!
//! The middle of the panel is a **drop zone**: text, an image or files,
//! and it says what each of them turned out to be. Recognising that is
//! `wipemark-intake` and accepting it is [`crate::drop`] — neither of
//! them is the panel's, and both are reused the day a second window
//! wants a drop. What is the panel's is the three states it draws:
//! nothing has been dropped here yet, a drop arrived carrying nothing
//! this machine could read, or something arrived and here is what it
//! was. Showing the invitation again for the middle one would look like
//! the drop never happened.
//!
//! # It cleans what it caught
//!
//! Under each thing it lists is a **findings line**: what a look found,
//! from [`clean::inspect_one`] — the bytes a clean would read, the layer's
//! inspection rather than its clean — on the background executor, once
//! per drop (D278). **Clean**, beside the dismissal line so it takes no
//! height from the list, cleans every caught thing that can be, one at a
//! time, through [`clean::clean_one`] with the plan
//! the Retention page gives at that thing's start, the queue's road
//! (D279); the *would* lines then become what happened and where it went.
//! Rewriting with a model is not in the windows, and the invitation says
//! so (`panel-pending`).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    actions, div, px, AnyWindowHandle, App, Bounds, Corner, Div, Entity, FocusHandle, Focusable,
    Global, Hsla, KeyBinding, Pixels, SharedString, Size, Subscription, Window, WindowBounds,
    WindowKind, WindowOptions,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::popover::Popover;
use gpui_component::{
    h_flex, v_flex, ActiveTheme, Disableable as _, Root, Sizable as _, StyledExt as _,
};
use wipemark_i18n::{args, t, t_args, Message};
use wipemark_intake::{Arrived, Format, Intake, Kind};

use crate::clean::{self, Cleanable, Findings, Outcome};
use crate::cleaner::{self, Cleaner};
use crate::drop::{self, Arrival, Catcher, Landed};
use crate::icon::{Icon, IconName};
use crate::placement;
use crate::retention::Plan;
use crate::screen::{self, Screen};
use crate::settings::Preferences;
use crate::wording::{self, would_happen, Badge, Tone};

actions!(wipemark, [ClosePanel]);

/// Key context of the panel, so Escape means *this* window and nothing
/// anywhere else.
const CONTEXT: &str = "Panel";

/// How big the panel is.
///
/// Small enough to sit in a sixth of a laptop panel without covering
/// what is underneath, which is the size a summoned window has to be:
/// it arrives over somebody else's work.
pub const SIZE: Size<Pixels> = Size {
    width: px(420.0),
    height: px(260.0),
};

/// How many of the things in one drop the panel lists before it starts
/// counting instead.
///
/// The window is 260 points tall and four rows is what fits under the
/// title with the dismissal line still visible. A fifth row that pushed
/// "Escape sends it away" off the bottom of a window with no titlebar
/// would take the only visible way out with it.
const LISTED: usize = 4;

/// How long a drag or a resize is left to finish before it is written
/// down.
///
/// The same bargain the MCP port field makes with typing: every
/// intermediate rectangle is a real rectangle on the way to the one the
/// user means, and each of them would otherwise be a row written and
/// thrown away.
const SETTLE: Duration = Duration::from_millis(400);

/// The panel, while there is one.
///
/// A global for the reason the Settings window has one: the menu bar,
/// the command line and the system-wide chord all ask "is it already
/// there", and none of them can see the others.
struct Opened(AnyWindowHandle);

impl Global for Opened {}

/// Bind Escape inside the panel.
///
/// Called once from `main`, after `gpui_component::init`. Scoped to the
/// panel's own context: unscoped, it would dismiss something in every
/// other window of this application.
pub fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("escape", ClosePanel, Some(CONTEXT))]);
}

/// Summon the panel, or send it away if it is already here.
///
/// One entry point rather than two, because the thing that summons it
/// is a single item in a menu and a single chord: pressing it twice has
/// to be the same as not pressing it at all.
pub fn toggle(preferences: &Entity<Preferences>, cx: &mut App) {
    if dismiss(cx) {
        return;
    }
    show(preferences, cx);
}

/// Put the panel on screen where the preferences say.
///
/// Already open is a no-op that brings it forward: it is one window,
/// and a second one would be a second answer to "where does it go".
pub fn show(preferences: &Entity<Preferences>, cx: &mut App) {
    if let Some(open) = cx.try_global::<Opened>() {
        let handle = open.0;
        if cx.windows().contains(&handle) {
            if let Err(error) = handle.update(cx, |_, window, _| window.activate_window()) {
                tracing::warn!(%error, "could not bring the panel forward");
            }
            return;
        }
    }

    let (onto, spots) = {
        let held = preferences.read(cx);
        (held.onto(), held.spots().clone())
    };
    // Chrome of zero: this window's content view *is* its frame — see
    // the note at the top of this module.
    let (screen, bounds) = placement::opening(onto, &spots, SIZE, Pixels::ZERO, cx);
    let display = screen.as_ref().map(|screen| screen.id);

    // Said *before* the window exists, and that is the whole of it. The
    // view subscribes to the bounds inside `open_window`, and `afloat`
    // rewrites the style mask from in there — which makes AppKit report
    // a frame while this call has not returned yet and there is nothing
    // to hand a handle to. With `expected` still empty that first report
    // is a rectangle nobody asked for, and four hundred milliseconds
    // later a display nobody had answered for was holding a
    // hand-placed rectangle it got by being summoned once.
    preferences.update(cx, |preferences, _| {
        preferences.we_placed_it(Some(bounds));
    });

    let held = preferences.clone();
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            // The window list still wants a name for it, and the
            // accessibility tree wants one more.
            titlebar: None,
            kind: WindowKind::Normal,
            // Movable and resizable *by hand*, which is the second
            // half of the placement: where the Placement page's grid
            // puts it is an answer, and dragging it somewhere else is a
            // better one — see `Answer::Manual`. With no titlebar there
            // is nothing to drag it *by*, so `afloat` below asks AppKit
            // for `movableByWindowBackground` and adds the resizable
            // mask GPUI leaves off a titleless window.
            is_movable: true,
            is_resizable: true,
            is_minimizable: false,
            // The rectangle above is display-local, so without this it
            // is read against the primary display whatever screen it
            // was measured on — see the note at the top of `screen`.
            display_id: display,
            ..Default::default()
        },
        move |window, cx| {
            let view = cx.new(|cx| PanelView::new(held, window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        },
    );

    match opened {
        Ok(window) => {
            let handle = AnyWindowHandle::from(window);
            cx.set_global(Opened(handle));
            preferences.update(cx, |preferences, cx| {
                preferences.panel_opened(handle, cx);
            });
            cx.activate(true);
        }
        Err(error) => {
            // Nothing is going to report that rectangle now, and a
            // stale one would make the next move that happens to land
            // there look like ours.
            preferences.update(cx, |preferences, _| {
                preferences.we_placed_it(None);
            });
            tracing::warn!(%error, "could not open the panel");
        }
    }
}

/// Make the panel float over everything, stay where it is when the
/// application is not in front, and be movable and resizable by hand.
///
/// Five calls AppKit has and GPUI does not expose, and each one is here
/// for a reason a summoned window has:
///
/// * **Level.** `NSFloatingWindowLevel` is what "topmost" means: above
///   every ordinary window, including other applications'. A panel you
///   call up over somebody else's document has to be *over* it.
/// * **`hidesOnDeactivate: false`.** The point of the level is lost if
///   the window vanishes the moment the application stops being
///   frontmost, which is the state it is summoned into.
/// * **`canJoinAllSpaces` and `fullScreenAuxiliary`.** It follows the
///   user to whichever desktop they are on, and it can appear over a
///   full-screen application rather than switching away from one.
/// * **The style mask.** GPUI builds it as `Titled | FullSizeContentView`
///   for a window with no titlebar (`gpui_macos::window`), and adds
///   `Resizable` only for one that *has* a titlebar. Both halves are
///   wrong for this window, and they are replaced rather than added to:
///   `Resizable` is what lets an edge be dragged, and dropping `Titled`
///   is what takes **zoom** away. A titled window is zoomable, and a
///   full-size content view makes its whole top a title bar — so a
///   double-click anywhere near the top maximized a panel whose entire
///   point is that it is small and where you put it. There is no
///   titlebar to lose: this window has never drawn one.
/// * **`movableByWindowBackground`.** With no titlebar there is nothing
///   to drag the window *by*. This makes the whole panel its own handle,
///   which is what every titleless panel on this desktop does.
///
/// A no-op off macOS, where a window that is merely ordinary is still a
/// window. E10.
#[cfg(target_os = "macos")]
fn afloat(window: &Window) {
    use objc2_app_kit::{NSFloatingWindowLevel, NSWindowCollectionBehavior, NSWindowStyleMask};

    let Some(native) = screen::native(window) else {
        tracing::warn!("the panel could not be made a floating window");
        return;
    };

    native.setLevel(NSFloatingWindowLevel);
    native.setHidesOnDeactivate(false);
    native.setCollectionBehavior(
        native.collectionBehavior()
            | NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary,
    );
    native.setStyleMask(
        NSWindowStyleMask::Borderless
            | NSWindowStyleMask::Resizable
            | NSWindowStyleMask::FullSizeContentView,
    );
    native.setMovableByWindowBackground(true);
}

#[cfg(not(target_os = "macos"))]
fn afloat(_window: &Window) {}

/// Send the panel away, and say whether there was one.
pub fn dismiss(cx: &mut App) -> bool {
    let Some(open) = cx.try_global::<Opened>() else {
        return false;
    };
    let handle = open.0;
    // `cx.windows()` is the authority: a closed window leaves the
    // global behind, and asking is cheaper than observing every close.
    if !cx.windows().contains(&handle) {
        return false;
    }
    if let Err(error) = handle.update(cx, |_, window, _| window.remove_window()) {
        tracing::warn!(%error, "could not close the panel");
        return false;
    }
    true
}

/// The view inside the panel.
struct PanelView {
    preferences: Entity<Preferences>,
    /// Where the keyboard lands. Without one, `Window::focused` stays
    /// `None` for the life of the window and Escape reaches nothing —
    /// see the same note on `SettingsView`.
    focus: FocusHandle,
    /// Dropped with the view: tells the preferences which display this
    /// window is on and where on it, which is what the Placement page's
    /// "the panel is here" tag reads, what decides whether a zone
    /// chosen there moves it now or at the next summon, and what turns
    /// a drag of this window into an answer of its own.
    _geometry: Subscription,
    /// Which settling is the current one.
    ///
    /// A drag reports a rectangle per frame and only the last of them
    /// is an answer, so each one bumps this and a task that wakes to
    /// find it moved has been overtaken. The same counter the MCP
    /// restart and the credential lookup use, for the same reason.
    moves: u64,
    /// What is dropped on this window, and what it turns out to be.
    ///
    /// The entity is `crate::drop`'s and knows nothing about panels —
    /// this window is its first caller and not its last.
    catcher: Entity<Catcher>,
    /// Redraws when something lands, and says so in the log. Dropped
    /// with the view.
    _caught: Subscription,
    /// The drop on screen, with what a look found and what a clean did.
    held: Option<Held>,
    /// The application's one line of cleans, which the queue's wait in too
    /// (D283).
    cleaner: Entity<Cleaner>,
    /// The cleans asked for here and not yet heard back from, by number:
    /// the drop and the thing in it each is a clean of. Still waiting or
    /// running in the line is what "Cleaning…" reads.
    asked: HashMap<u64, (Arc<[Arrival]>, usize)>,
    /// Dropped with the view: the line says what each clean did.
    _cleaned: Subscription,
}

/// The drop the panel shows, and what it knows about it: a look per thing
/// it lists, and an outcome per thing it cleaned. Kept per drop, so a
/// redraw never looks again (D278).
struct Held {
    arrivals: Arc<[Arrival]>,
    /// What a look found, per listed thing; `None` while it looks.
    found: Vec<Option<Findings>>,
    /// What a clean did, per thing; `None` until it is cleaned.
    done: Vec<Option<Arc<Outcome>>>,
}

impl Held {
    /// Whether each thing can be cleaned, and whether it has been — what
    /// [`to_clean`] reads.
    fn states(&self) -> Vec<(Cleanable, bool)> {
        self.arrivals
            .iter()
            .zip(&self.done)
            .map(|(arrival, done)| (clean::cleanable(&arrival.intake), done.is_some()))
            .collect()
    }
}

/// Which things Clean cleans: every one that can be cleaned and has not
/// been, in the order they arrived. Pure.
fn to_clean(states: &[(Cleanable, bool)]) -> Vec<usize> {
    states
        .iter()
        .enumerate()
        .filter(|(_, (cleanable, done))| !matches!(cleanable, Cleanable::No(_)) && !done)
        .map(|(index, _)| index)
        .collect()
}

/// Whether Clean is offered: not while a clean runs, and not when nothing
/// caught is left to clean. Pure.
fn clean_offered(cleaning: bool, states: &[(Cleanable, bool)]) -> bool {
    !cleaning && !to_clean(states).is_empty()
}

/// The findings line: what a look found, or that it is still looking.
fn findings_line(found: Option<&Findings>) -> String {
    found.map_or_else(|| t(Message::PanelLooking), wording::found)
}

/// Whether a look found something worth the foreground colour.
fn something_found(found: &Findings) -> bool {
    match found {
        Findings::Text { change, suspicious } => *change > 0 || *suspicious,
        Findings::Picture {
            ai_metadata,
            mark,
            not_examined,
        } => *ai_metadata || *mark || not_examined.is_some(),
        Findings::NotLooked(_) => false,
    }
}

impl PanelView {
    fn new(preferences: Entity<Preferences>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        afloat(window);

        // The drop zone, and the AppKit destination behind it. Both
        // belong here rather than in `show`: a window that is not built
        // yet has no view to insert anything under, and a destination
        // installed a frame later is a drag that was refused once.
        let catcher = cx.new(|cx| Catcher::new(window, cx));
        drop::accept(window);
        Self::with_catcher(preferences, catcher, window, cx)
    }

    /// The panel over a catcher already made — [`PanelView::new`]'s, which
    /// the platform delivers to, or a test's, which it does not — with none
    /// of the platform's hooks: no floating level, no drag destination. The
    /// queue's seam (`Queue::with_catcher`), for the same reason: a test
    /// window has no native window for either to hang from.
    fn with_catcher(
        preferences: Entity<Preferences>,
        catcher: Entity<Catcher>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        focus.focus(window, cx);

        preferences.update(cx, |held, cx| {
            held.panel_moved(screen::of_window(window, cx), cx);
        });

        let moved = cx.observe_window_bounds(window, |view: &mut Self, window, cx| {
            let screen = screen::of_window(window, cx);
            view.preferences.update(cx, |preferences, cx| {
                preferences.panel_moved(screen.clone(), cx);
            });

            // Every move reports here: ours and the user's. The one
            // thing that tells them apart is whether we asked for this
            // rectangle — see `Preferences::we_placed_it`.
            let frame = window.bounds();
            if view.preferences.read(cx).ours(frame) {
                // Ours, and so is any settling still waiting: a resize
                // reports the move and the new size one after the
                // other, and the first of those is a rectangle nobody
                // asked for. Bumping the counter throws it away.
                view.moves += 1;
                return;
            }
            let Some(screen) = screen else {
                return;
            };
            view.settle(screen, frame, cx);
        });

        let caught = cx.subscribe(&catcher, |view: &mut Self, catcher, landed: &Landed, cx| {
            // The count and the kinds, and nothing else. A log line
            // carrying what was dropped would be the document in a
            // file the product wrote — see `wipemark_log::Elided`.
            let kinds: Vec<Kind> = catcher
                .read(cx)
                .caught()
                .unwrap_or_default()
                .iter()
                .map(|arrival| arrival.intake.kind)
                .collect();
            tracing::info!(?kinds, "something was dropped on the panel");
            if catcher.read(cx).is_caught(&landed.0) {
                view.hold(landed.0.clone(), cx);
            }
            cx.notify();
        });

        let cleaner = Cleaner::shared(&preferences, cx);
        let cleaned = cx.subscribe(
            &cleaner,
            |view: &mut Self, _, event: &cleaner::Event, cx| {
                view.heard(event, cx);
            },
        );

        Self {
            preferences,
            focus,
            _geometry: moved,
            moves: 0,
            catcher,
            _caught: caught,
            held: None,
            cleaner,
            asked: HashMap::new(),
            _cleaned: cleaned,
        }
    }

    /// Hold a new drop, and look at what it lists, one thing at a time on
    /// the background executor. A look overtaken by the next drop stops.
    fn hold(&mut self, arrivals: Arc<[Arrival]>, cx: &Context<Self>) {
        let listed = arrivals.len().min(LISTED);
        self.held = Some(Held {
            arrivals: arrivals.clone(),
            found: (0..listed).map(|_| None).collect(),
            done: (0..arrivals.len()).map(|_| None).collect(),
        });
        cx.spawn(async move |view, cx| {
            for index in 0..listed {
                let thing = arrivals.clone();
                let found = cx
                    .background_executor()
                    .spawn(async move { clean::inspect_one(&thing[index]) })
                    .await;
                let current = view
                    .update(cx, |view, cx| {
                        let Some(held) = view.held_for(&arrivals) else {
                            return false;
                        };
                        held.found[index] = Some(found);
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);
                if !current {
                    break;
                }
            }
        })
        .detach();
    }

    /// The held drop, if it is still `arrivals`.
    fn held_for(&mut self, arrivals: &Arc<[Arrival]>) -> Option<&mut Held> {
        self.held
            .as_mut()
            .filter(|held| Arc::ptr_eq(&held.arrivals, arrivals))
    }

    /// Clean everything caught that can be, in the application's one line,
    /// one at a time, each by the plan the Retention page gives when its
    /// clean starts (D279). A drop that overtakes it does not stop it —
    /// what was asked for is finished — but its outcomes are then nobody's
    /// to show.
    fn clean(&mut self, cx: &mut Context<Self>) {
        let Some(held) = &self.held else {
            return;
        };
        let states = held.states();
        if !clean_offered(self.cleaning(cx), &states) {
            return;
        }
        let arrivals = held.arrivals.clone();
        for index in to_clean(&states) {
            let number = clean::number();
            let thing = arrivals[index].clone();
            if self
                .cleaner
                .update(cx, |cleaner, cx| cleaner.ask(number, thing, None, cx))
            {
                self.asked.insert(number, (arrivals.clone(), index));
            }
        }
        cx.notify();
    }

    /// Whether a clean asked for here is still in the line — for this drop
    /// or one it overtook.
    fn cleaning(&self, cx: &App) -> bool {
        let cleaner = self.cleaner.read(cx);
        self.asked.keys().any(|number| cleaner.pending(*number))
    }

    /// What the line said about a clean asked for here — one of the queue's
    /// is not this window's, and is not heard.
    fn heard(&mut self, event: &cleaner::Event, cx: &mut Context<Self>) {
        match event {
            cleaner::Event::Started(_) => cx.notify(),
            cleaner::Event::Finished(number, outcome) => {
                let Some((arrivals, index)) = self.asked.remove(number) else {
                    return;
                };
                if let Some(held) = self.held_for(&arrivals) {
                    held.done[index] = Some(outcome.clone());
                }
                cx.notify();
            }
        }
    }

    /// Write down where the user has just put this window, once they
    /// have stopped putting it there.
    ///
    /// Debounced, because a drag is sixty rectangles a second and only
    /// the last one is the answer — and because each one is a row in
    /// SQLite, on the background executor, from the thread that draws
    /// the window.
    fn settle(&mut self, screen: Screen, frame: Bounds<Pixels>, cx: &Context<Self>) {
        self.moves += 1;
        let mine = self.moves;

        cx.spawn(async move |view, cx| {
            cx.background_executor().timer(SETTLE).await;
            view.update(cx, |view, cx| {
                if view.moves != mine {
                    // Overtaken: the drag is still going.
                    return;
                }
                view.preferences.update(cx, |preferences, cx| {
                    preferences.put_by_hand(&screen, frame, cx);
                });
            })
            .ok();
        })
        .detach();
    }
}

/// One thing that was dropped, in two lines and a sentence.
///
/// The first is what to call it: a file has a name, and everything else
/// has only what it *is*, because "image.png" for something that was
/// never a file on this disk would be a name nobody could go and find.
/// The second line is the evidence — the format, the encoding, the size
/// — and under it, when there is one, the thing worth being told. Then
/// what a look found, and what would happen to it by the Retention page's
/// choices — or, once it is cleaned, what happened and where the result
/// went. A thing that cannot be cleaned has no *would* lines: there will
/// be no result.
fn caught_row(
    intake: &Intake,
    plan: &Plan,
    found: Option<&Findings>,
    done: Option<&Outcome>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let title = wording::title_of(intake);

    let mut detail: Vec<String> = Vec::new();
    if intake.arrived == Arrived::AsPath {
        detail.push(wording::kind_label(intake.kind));
    }
    if let Some(format) = intake.format {
        // `PlainText` is the *absence* of a more specific answer, and
        // printing "Text · Text" is the window arguing with itself.
        if format != Format::PlainText {
            detail.push(format.name().to_owned());
        }
    }
    if let Some(encoding) = intake.encoding {
        detail.push(encoding.name().to_owned());
    }
    if let Some(size) = intake.size {
        detail.push(drop::size_label(size));
    }

    // The line that says how much of this is established. A name that
    // lost an argument with the bytes is the one a person most needs to
    // see, so it is the one that is not muted.
    let (note, colour) = match wording::evidence_note(intake) {
        Some((note, Tone::Warning)) => (Some(note), theme.warning),
        Some((note, Tone::Muted)) => (Some(note), theme.muted_foreground),
        None => (None, theme.muted_foreground),
    };

    let muted = theme.muted_foreground;
    let mut lines: Vec<(String, Hsla)> = Vec::new();
    match done {
        Some(outcome) => {
            let colour = match wording::verdict_badge(&outcome.verdict).1 {
                Badge::Danger => theme.danger,
                Badge::Warning => theme.warning,
                Badge::Success | Badge::Muted => theme.foreground,
            };
            lines.push((wording::said(outcome), colour));
            lines.extend(wording::went(outcome).into_iter().map(|line| (line, muted)));
        }
        None => {
            let colour = if found.is_some_and(something_found) {
                theme.foreground
            } else {
                muted
            };
            lines.push((findings_line(found), colour));
            if !matches!(clean::cleanable(intake), Cleanable::No(_)) {
                lines.extend(would_happen(plan).into_iter().map(|line| (line, muted)));
            }
        }
    }

    v_flex()
        .gap_0p5()
        .child(
            h_flex()
                .gap_2()
                .items_baseline()
                .child(
                    div()
                        .text_xs()
                        .font_semibold()
                        .truncate()
                        .child(SharedString::from(title)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .flex_shrink_0()
                        .child(SharedString::from(detail.join(" · "))),
                ),
        )
        .children(note.map(|note| {
            h_flex()
                .gap_1()
                .items_center()
                .text_xs()
                .text_color(colour)
                .child(Icon::new(IconName::CircleInfo).small())
                .child(SharedString::from(note))
        }))
        .children(lines.into_iter().map(|(line, colour)| {
            div()
                .text_xs()
                .text_color(colour)
                .child(SharedString::from(line))
        }))
}

/// What you can do with this window, since none of it is visible.
///
/// Four lines and no more: everything a person can do to the panel,
/// each one a sentence. The last is the one that is not a gesture —
/// where it opens is a preference, and the page that holds it is worth
/// naming here rather than leaving somebody to find.
fn help(cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    v_flex()
        .w(px(260.0))
        .gap_1()
        .text_xs()
        .text_color(theme.muted_foreground)
        .children(
            [
                Message::PanelHelpMove,
                Message::PanelHelpResize,
                Message::PanelHelpDismiss,
                Message::PanelHelpPlacement,
            ]
            .map(|line| div().child(SharedString::from(t(line)))),
        )
}

impl PanelView {
    /// The middle of the panel: the thing you drop on.
    ///
    /// Three states, and they are three different answers. Nothing has
    /// been dropped here yet, so the window says what it takes. A drop
    /// arrived and carried nothing this machine could read — which is
    /// not the same as the first, and a window that showed the
    /// invitation again would look like the drop never happened. Or
    /// something arrived, and this is what it turned out to be.
    fn landing(&self, cx: &Context<Self>) -> Div {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let over = self.catcher.read(cx).over();

        let area = drop::zone(v_flex(), &self.catcher, cx)
            .flex_1()
            // Without this a child longer than the window makes the
            // flex item grow instead of clipping, and the dismissal
            // line goes off the bottom of a window with no titlebar.
            .min_h(px(0.0))
            .gap_1p5()
            .p_2()
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius)
            .overflow_hidden();

        let Some(caught) = self.catcher.read(cx).caught() else {
            return area
                .justify_center()
                .items_center()
                .text_center()
                .child(div().text_xs().child(SharedString::from(t(if over {
                    Message::PanelDropRelease
                } else {
                    Message::PanelDropInvite
                }))))
                .child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .child(SharedString::from(t(Message::PanelPending))),
                );
        };

        if caught.is_empty() {
            return area.justify_center().items_center().text_center().child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child(SharedString::from(t(Message::PanelDropNothing))),
            );
        }

        let over_by = caught.len().saturating_sub(LISTED);
        let preferences = self.preferences.read(cx);
        // What a look found and a clean did, when the drop held is the one
        // caught — it is, once its `Landed` has been heard.
        let held = self
            .held
            .as_ref()
            .filter(|held| self.catcher.read(cx).is_caught(&held.arrivals));
        area.children(
            caught
                .iter()
                .take(LISTED)
                .enumerate()
                .map(|(index, arrival)| {
                    let intake = &arrival.intake;
                    caught_row(
                        intake,
                        &preferences.plan_for(intake),
                        held.and_then(|held| held.found.get(index)?.as_ref()),
                        held.and_then(|held| held.done.get(index)?.as_deref()),
                        cx,
                    )
                }),
        )
        .when(over_by > 0, |area| {
            area.child(
                div()
                    .text_xs()
                    .text_color(muted)
                    // The number, not its spelling: Fluent picks the
                    // plural form from a number and never from a string,
                    // and "… and 1 more" was reading as the plural.
                    .child(SharedString::from(t_args(
                        Message::PanelDropMore,
                        &args!("count" => over_by),
                    ))),
            )
        })
    }
}

impl PanelView {
    /// Clean, once something cleanable has been caught: greyed while a
    /// clean runs and once nothing is left to clean, its tooltip saying so.
    fn clean_button(&self, cx: &Context<Self>) -> Option<Button> {
        let held = self.held.as_ref()?;
        let states = held.states();
        let ever = states
            .iter()
            .any(|(cleanable, _)| !matches!(cleanable, Cleanable::No(_)));
        if !ever {
            // Nothing here could ever be cleaned: no button to grey.
            return None;
        }
        Some(
            Button::new("panel-clean")
                .xsmall()
                .icon(IconName::Broom)
                .label(SharedString::from(t(if self.cleaning(cx) {
                    Message::PanelCleaning
                } else {
                    Message::PanelClean
                })))
                .disabled(!clean_offered(self.cleaning(cx), &states))
                .tooltip(SharedString::from(t(Message::PanelCleanTooltip)))
                .on_click(cx.listener(|view, _, _, cx| view.clean(cx))),
        )
    }
}

impl Focusable for PanelView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for PanelView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;

        v_flex()
            .id("panel")
            .track_focus(&self.focus)
            .key_context(CONTEXT)
            .on_action(cx.listener(|view, _: &ClosePanel, window, cx| {
                view.preferences.update(cx, |preferences, cx| {
                    preferences.panel_closed(cx);
                });
                // One `ERROR gpui::window: window not found` follows
                // this, and it is not ours — see the same note in
                // `settings`.
                window.remove_window();
            }))
            .size_full()
            .gap_2()
            .p_4()
            .bg(theme.background)
            .text_color(theme.foreground)
            // A border, because this window has no titlebar and nothing
            // else would say where it ends.
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius)
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(Icon::new(IconName::Broom).small().color(muted))
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .child(SharedString::from(t(Message::PanelTitle))),
                            ),
                    )
                    // What can be done with a window that has no
                    // titlebar, no traffic lights and no menu of its
                    // own — which is every affordance this one has, and
                    // none of them is visible. A panel that can be
                    // dragged from anywhere and resized from any edge
                    // looks exactly like a panel that can do neither.
                    .child(
                        Popover::new("panel-help")
                            .anchor(Corner::TopRight)
                            .trigger(
                                Button::new("panel-help-trigger")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::CircleInfo)
                                    .tooltip(SharedString::from(t(Message::PanelHelp))),
                            )
                            .content(|_, _, cx| help(cx)),
                    ),
            )
            .child(self.landing(cx))
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .text_xs()
                            .text_color(muted)
                            .child(SharedString::from(t(Message::PanelDismiss))),
                    )
                    .children(self.clean_button(cx)),
            )
    }
}

#[cfg(test)]
mod tests {
    use wipemark_i18n::{available_languages, FluentArgs, Localizer, Rendering};
    use wipemark_intake::Encoding;
    use wipemark_picture::NotExamined;

    use super::*;
    use crate::clean::{Refusal, Unable};

    /// Every case a look can come to, beside the line it should read as.
    fn every_look() -> Vec<(Findings, &'static str)> {
        let text = |change, suspicious| Findings::Text { change, suspicious };
        let picture = |ai_metadata, mark, not_examined| Findings::Picture {
            ai_metadata,
            mark,
            not_examined,
        };
        vec![
            (text(2, false), "2 characters to remove or replace."),
            (text(1, true), "One character to remove or replace."),
            (text(0, false), "Nothing to remove."),
            (
                text(0, true),
                "Nothing to remove; a letter from another alphabet that looks like a Latin one \
                 is kept at the default settings.",
            ),
            (picture(true, true, None), "AI metadata and a visible mark."),
            (picture(true, false, None), "AI metadata."),
            (picture(false, true, None), "A visible mark."),
            (
                picture(false, false, None),
                "Nothing found in the metadata or among the visible marks this version knows.",
            ),
            (
                picture(true, false, Some(NotExamined::Animated)),
                "AI metadata; the frames of an animated picture are not examined for a visible \
                 mark.",
            ),
            (
                picture(false, false, Some(NotExamined::Catalogue)),
                "No AI metadata; the catalogue of visible marks did not load, so the pixels were \
                 not examined.",
            ),
            (
                picture(false, false, Some(NotExamined::Decode)),
                "No AI metadata; the pixels could not be decoded, so they were not examined.",
            ),
            (
                Findings::NotLooked(Refusal::NotCleanable(Unable::NotYet(Format::Tiff))),
                "TIFF pictures are not read in this version yet.",
            ),
            (
                Findings::NotLooked(Refusal::NotCleanable(Unable::Folder)),
                "A folder is not cleaned as one thing; drop the files in it instead.",
            ),
        ]
    }

    /// The findings line, case by case — and "Looking…" until the look
    /// lands. Not examined is said whatever the metadata held.
    #[test]
    fn the_findings_line_says_what_a_look_found() {
        assert_eq!(findings_line(None), "Looking…");
        for (found, line) in every_look() {
            assert_eq!(findings_line(Some(&found)), line, "{found:?}");
        }
    }

    /// Every case reads as its own sentence in every language: nothing
    /// left unresolved, and no two cases alike.
    #[test]
    fn every_look_reads_in_every_language() {
        for language in available_languages() {
            let localizer =
                Localizer::for_languages(std::slice::from_ref(&language.id), Rendering::PlainText);
            let say = |message: Message, args: &FluentArgs| localizer.format_args(message, args);
            let lines: Vec<String> = every_look()
                .iter()
                .map(|(found, _)| wording::found_in(&say, found))
                .collect();
            for line in &lines {
                assert!(
                    !line.is_empty() && !line.contains(['{', '}', '$']),
                    "{}: {line:?}",
                    language.id
                );
            }
            let mut unique = lines.clone();
            unique.sort();
            unique.dedup();
            assert_eq!(unique.len(), lines.len(), "{}: {lines:?}", language.id);
        }
    }

    /// Only a look that found something is painted in the foreground:
    /// characters to change, a kept look-alike, anything on a picture, and
    /// pixels not examined — never "nothing" and never a refusal.
    #[test]
    fn only_a_finding_is_painted_as_one() {
        let expected = [
            true, true, false, true, // text: 2, 1, nothing, kept
            true, true, true, false, // picture: both, metadata, mark, nothing
            true, true, true, // not examined: animated, catalogue, decode
            false, false, // TIFF, folder
        ];
        let looks = every_look();
        assert_eq!(looks.len(), expected.len());
        for ((found, line), expected) in looks.iter().zip(expected) {
            assert_eq!(something_found(found), expected, "{line}");
        }
    }

    /// Clean cleans what can be cleaned and has not been, in arrival order;
    /// it is offered only when there is such a thing and no clean runs.
    #[test]
    fn clean_is_offered_only_when_something_is_left_to_clean() {
        let text = Cleanable::Text(Encoding::Utf8);
        let png = Cleanable::Picture(Format::Png);
        let tiff = Cleanable::No(Unable::NotYet(Format::Tiff));

        let fresh = [(text, false), (tiff, false), (png, false)];
        assert_eq!(to_clean(&fresh), [0, 2]);
        assert!(clean_offered(false, &fresh));
        assert!(!clean_offered(true, &fresh), "offered while a clean runs");

        let half = [(text, true), (tiff, false), (png, false)];
        assert_eq!(to_clean(&half), [2], "a thing cleaned is not cleaned again");

        let done = [(text, true), (tiff, false), (png, true)];
        assert!(to_clean(&done).is_empty());
        assert!(!clean_offered(false, &done), "offered with nothing left");

        assert!(
            !clean_offered(false, &[(tiff, false)]),
            "offered over a TIFF"
        );
        assert!(!clean_offered(false, &[]));
    }

    /// The invitation says only that rewriting is not here — in every
    /// language, with no epic number and nothing saying the panel does not
    /// clean.
    #[test]
    fn the_panel_says_only_rewriting_is_not_here() {
        let line = t(Message::PanelPending);
        assert!(line.contains("Rewriting"), "{line}");
        assert!(line.contains("not in this version"), "{line}");
        for language in available_languages() {
            let localizer =
                Localizer::for_languages(std::slice::from_ref(&language.id), Rendering::PlainText);
            let line = localizer.format(Message::PanelPending);
            for old in [
                "Cleaning from this window is not",
                "Очистки из этого окна",
                "Bereinigen aus diesem Fenster",
            ] {
                assert!(
                    !line.contains(old),
                    "{}: still says the panel does not clean: {line}",
                    language.id
                );
            }
            assert!(
                !line
                    .split(|c: char| !c.is_ascii_alphanumeric())
                    .any(|word| word.len() > 1
                        && word.starts_with('E')
                        && word[1..].chars().all(|c| c.is_ascii_digit())),
                "{}: an epic number reached a window: {line}",
                language.id
            );
        }
    }

    /// The live check's case 13, as a test: a drop of a marked text, a
    /// plain one, a Gemini picture and a TIFF on a panel built without the
    /// platform's hooks; the looks land, Clean cleans every thing that can
    /// be cleaned through the application's one line, each row says what
    /// happened, and on disk there is a result for the marked text and the
    /// picture and for nothing else. Clean is then greyed: nothing is left
    /// to clean. This retires the series' deviation 17.
    #[gpui::test]
    fn a_drop_on_the_panel_is_looked_at_and_cleaned(cx: &mut gpui::TestAppContext) {
        use std::cell::RefCell;
        use std::rc::Rc;

        use wipemark_intake::Handed;

        use crate::retention::Homes;

        let scratch =
            std::env::temp_dir().join(format!("wipemark-panel-drop-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(&scratch).expect("scratch");
        let file = |name: &str, bytes: &[u8]| {
            let path = scratch.join(name);
            std::fs::write(&path, bytes).expect("scratch file");
            path
        };
        let marked = file(
            "marked.md",
            "# Notes\n\nTwo\u{200B} marks\u{200B} here.\n".as_bytes(),
        );
        let plain = file("plain.md", b"# Plain\n\nNothing to find.\n");
        let torch = file(
            "torch-1025.png",
            &std::fs::read(format!(
                "{}/../../fixtures/image/gemini/torch-1025.png",
                env!("CARGO_MANIFEST_DIR")
            ))
            .expect("fixture"),
        );
        let tiff = file("scan.tif", b"II*\x00\x08\x00\x00\x00\x00\x00\x00\x00");

        cx.update(gpui_component::init);
        let homes = Homes {
            results: scratch.join("results"),
            kept: scratch.join("kept"),
        };
        let slot: Rc<RefCell<Option<Entity<PanelView>>>> = Rc::default();
        let held = slot.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let preferences = cx.new(|cx| Preferences::for_tests(homes, cx));
            // Detached, and no `afloat` or `drop::accept`: a test window
            // has no native window for either.
            let catcher = cx.new(|_| Catcher::detached());
            let view = cx.new(|cx| PanelView::with_catcher(preferences, catcher, window, cx));
            *held.borrow_mut() = Some(view.clone());
            gpui_component::Root::new(view, window, cx)
        });
        let view = slot.take().expect("the window builder ran");

        let catcher = cx.update(|_, cx| view.read(cx).catcher.clone());
        catcher.update(cx, |catcher, cx| {
            catcher.land(
                [&marked, &plain, &torch, &tiff]
                    .into_iter()
                    .map(|path| Handed::Path(path.clone()))
                    .collect(),
                cx,
            );
        });
        cx.run_until_parked();

        let looked = cx.update(|_, cx| {
            let held = view.read(cx).held.as_ref().expect("the drop is held");
            held.found
                .iter()
                .map(|found| found.as_ref().map(wording::found))
                .collect::<Vec<_>>()
        });
        assert_eq!(
            looked,
            [
                Some(String::from("2 characters to remove or replace.")),
                Some(String::from("Nothing to remove.")),
                Some(String::from("A visible mark.")),
                Some(String::from(
                    "TIFF pictures are not read in this version yet."
                )),
            ]
        );
        let offered = cx.update(|_, cx| {
            let panel = view.read(cx);
            clean_offered(panel.cleaning(cx), &panel.held.as_ref().unwrap().states())
        });
        assert!(
            offered,
            "Clean is not offered over a drop with things to clean"
        );

        view.update(cx, |view, cx| view.clean(cx));
        let busy = cx.update(|_, cx| view.read(cx).cleaning(cx));
        assert!(busy, "Clean does not say it is cleaning");
        cx.run_until_parked();

        let done = cx.update(|_, cx| {
            let panel = view.read(cx);
            let held = panel.held.as_ref().expect("the drop is held");
            held.done
                .iter()
                .map(|done| done.as_ref().map(|outcome| outcome.verdict.id()))
                .collect::<Vec<_>>()
        });
        assert_eq!(
            done,
            [
                Some("cleaned"),
                Some("nothing-found"),
                Some("cleaned"),
                None
            ],
            "a thing was skipped, or the TIFF was cleaned"
        );
        let mut names: Vec<String> = std::fs::read_dir(&scratch)
            .expect("list")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                "marked.cleaned.md",
                "marked.md",
                "plain.md",
                "scan.tif",
                "torch-1025.cleaned.png",
                "torch-1025.png",
            ]
        );
        assert!(!std::fs::read_to_string(scratch.join("marked.cleaned.md"))
            .expect("result")
            .contains('\u{200B}'));
        let after = cx.update(|_, cx| {
            let panel = view.read(cx);
            (
                panel.cleaning(cx),
                clean_offered(panel.cleaning(cx), &panel.held.as_ref().unwrap().states()),
            )
        });
        assert_eq!(
            after,
            (false, false),
            "Clean is still offered with nothing left"
        );
        std::fs::remove_dir_all(&scratch).ok();
    }
}
