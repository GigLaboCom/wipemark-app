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
//! # What it caught is a row of the one list
//!
//! Every drop is handed to the main window's table the moment it lands
//! (`Queue::take_from_panel`, D690): one row per thing, written to the
//! document journal with the origin `panel` exactly as a drop on the main
//! window is — whether that window is in front, hidden by the tray or
//! minimised, because the table is a global that outlives it. The panel
//! keeps the rows' ids and nothing of its own about what became of them.
//!
//! # It cleans what it caught — that row
//!
//! Under each thing it lists is a **findings line**: what a look found,
//! from [`clean::inspect_one`] — the bytes a clean would read, the layer's
//! inspection rather than its clean — on the background executor, once
//! per drop (D278). **Clean**, beside the dismissal line so it takes no
//! height from the list, asks the table to clean those rows (D691) — the
//! one line of cleans, one at a time, each by the plan the Retention page
//! gives at its start (D279, D283) — and no second row is made. What the
//! panel then says of a thing is read from its row (`Queue::standing`,
//! D692): a clean asked from the main window's row reads here too, and
//! the *would* lines become what happened and where it went.
//! Rewriting is in the main window, and the invitation says so
//! (`panel-pending`).
//!
//! # It clears itself
//!
//! A new drop replaces what the panel shows; and after a quiet period —
//! nothing looked at, cleaned or rewritten for its things for
//! `ui.panel.clear_after` seconds, a minute unless the Placement page says
//! otherwise — it lets go of what it shows and invites the next drop
//! (D693, D694). Clearing takes nothing out of the list.

use std::sync::Arc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    actions, div, px, Anchor, AnyWindowHandle, App, Bounds, Div, Entity, FocusHandle, Focusable,
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
use crate::drop::{self, Arrival, Catcher, Landed};
use crate::icon::{Icon, IconName};
use crate::placement;
use crate::queue::{Queue, Standing, Table};
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

/// How far the pointer travels with the button held before a press on
/// the panel becomes a move of it, on Linux.
///
/// AppKit's `movableByWindowBackground` makes the whole panel its handle
/// on macOS; GPUI has `Window::start_window_move` on both Linux backends,
/// which hands the drag to the window manager. Four points, so a click on
/// a button that wobbles stays a click — the same reason gpui-component's
/// title bar waits for a move before it starts one.
const DRAG_AFTER: f32 = 4.0;

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

/// The panel's options: at `bounds` on `display`, and no titlebar.
pub(crate) fn window_options(
    bounds: Bounds<Pixels>,
    display: Option<gpui::DisplayId>,
) -> WindowOptions {
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
        // What the desktop matches to `wipemark.desktop`: the dock's and
        // Alt+Tab's icon, `WM_CLASS` on X11, `app_id` on Wayland (D605).
        app_id: Some(crate::title::APP_ID.into()),
        ..Default::default()
    }
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
    let opened = cx.open_window(window_options(bounds, display), move |window, cx| {
        #[cfg(target_os = "linux")]
        crate::x11::give_icon(window);
        // A summoned panel is placed, never maximized: no maximize button
        // and no maximize by a double click on the window manager's bar
        // (the owner, 2026-10-12, D660). macOS draws it with no title bar,
        // and under native Wayland it has none either.
        #[cfg(target_os = "linux")]
        if let (Some(x11), Some(xid)) = (crate::x11::get(), crate::x11::xid(window)) {
            x11.no_maximize(xid);
        }
        let view = cx.new(|cx| PanelView::new(held, window, cx));
        cx.new(|cx| Root::new(view, window, cx))
    });

    match opened {
        Ok(window) => {
            let handle = AnyWindowHandle::from(window);
            cx.set_global(Opened(handle));
            preferences.update(cx, |preferences, cx| {
                preferences.panel_opened(handle, cx);
            });
            #[cfg(target_os = "linux")]
            if let Some(screen) = screen {
                placed_by_the_window_manager(handle, screen, bounds, preferences.clone(), cx);
            }
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
fn afloat(_window: &Window) {
    // Under X11 the same three things — above other windows, on every
    // workspace, where the zone says — are asked of the window manager
    // once it has taken the window over, which is after this returns:
    // see `placed_by_the_window_manager`. Under Wayland a client can ask
    // for none of them.
}

/// Under X11: the zone, applied once the window manager has the panel,
/// and the panel kept above other windows on every workspace (D613).
///
/// GPUI creates the window at the rectangle it is handed and writes no
/// position into `WM_NORMAL_HINTS`, so mutter opens it where it opens
/// every new window. So, in order:
///
/// 1. the asked-for corner goes into `WM_NORMAL_HINTS` at once — a
///    window manager that reads the hints after this lands opens the
///    panel there and nothing jumps;
/// 2. once the window manager has taken the window over (`WM_STATE`, on
///    screen), its frame is read — the title bar it drew is what makes a
///    zone's rectangle bigger than the panel;
/// 3. the frame is asked for at the zone, the decoration counted
///    (`_NET_MOVERESIZE_WINDOW`, north-west gravity: the corner named is
///    the frame's), and the asked rectangle is what
///    [`Preferences::we_placed_it`] waits for — every report until it
///    lands is ours (D616);
/// 4. `_NET_WM_STATE_ABOVE` and `_NET_WM_STATE_STICKY`, the X11 half of
///    [`afloat`]'s level and every-space.
///
/// Nothing here waits on the GPUI thread: the frame is awaited from a
/// task, and every request goes to `crate::x11`'s thread.
#[cfg(target_os = "linux")]
fn placed_by_the_window_manager(
    panel: AnyWindowHandle,
    screen: Screen,
    opened_at: Bounds<Pixels>,
    preferences: Entity<Preferences>,
    cx: &mut App,
) {
    use crate::x11::{self, Gravity};

    let Some(x11) = x11::get() else {
        return;
    };
    let Ok(Some((xid, scale))) = panel.update(cx, |_, window, _| {
        x11::xid(window).map(|xid| (xid, window.scale_factor()))
    }) else {
        return;
    };
    x11.hint(xid, Gravity::Frame, screen::device(opened_at.origin, scale));

    let landed = x11.frame(xid, true);
    cx.spawn(async move |cx| {
        let Ok(Some(frame)) = landed.recv_async().await else {
            tracing::debug!("the panel's frame was never read; it stays where it opened");
            return;
        };
        let chrome = screen::Chrome {
            left: px(frame.extents.left as f32 / scale),
            right: px(frame.extents.right as f32 / scale),
            top: px(frame.extents.top as f32 / scale),
            bottom: px(frame.extents.bottom as f32 / scale),
        };
        cx.update(|cx| {
            if !cx.windows().contains(&panel) {
                return;
            }
            let spot = preferences.read(cx).spot_of(&screen);
            let wanted = placement::where_it_goes(spot, chrome.grow(SIZE), screen.visible);
            preferences.update(cx, |preferences, _| {
                preferences.we_placed_it(Some(wanted));
            });
            let client = chrome.shrink(wanted.size);
            x11.move_window(
                xid,
                Gravity::Frame,
                screen::device(wanted.origin, scale),
                Some(screen::device(
                    gpui::point(client.width, client.height),
                    scale,
                )),
            );
            x11.float(xid);
            tracing::debug!(?wanted, "the panel asked of the window manager");
        });
    })
    .detach();
}

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
    /// Where the primary button went down on the panel, until it comes up
    /// or the panel starts moving — Linux's stand-in for AppKit's
    /// `movableByWindowBackground` (see [`DRAG_AFTER`]).
    pressed: Option<gpui::Point<Pixels>>,
    /// What is dropped on this window, and what it turns out to be.
    ///
    /// The entity is `crate::drop`'s and knows nothing about panels —
    /// this window is its first caller and not its last.
    catcher: Entity<Catcher>,
    /// Hands every drop to the list, holds the last, and says so in the
    /// log. Dropped with the view.
    _caught: Subscription,
    /// The main window's table, which every drop here becomes rows of
    /// (D690). `None` where no shell built one — a test of the panel
    /// alone, where nothing is listed and nothing can be cleaned.
    table: Option<Entity<Queue>>,
    /// Dropped with the view: a row of the table moved, so what the panel
    /// says of its things may have (D692).
    _listed: Option<Subscription>,
    /// The last drop — the one [`Catcher::caught`] holds — with what is
    /// known of it; `None` before the first, for a drop that carried
    /// nothing, and once the panel has cleared.
    held: Option<Held>,
    /// Whether the panel has let go of the last drop after a quiet period
    /// (D693): it then shows the invitation, as before the first drop,
    /// until the next one lands.
    cleared: bool,
    /// The quiet period's timer (D693).
    quiet: Quiet,
    /// Dropped with the view: a change on the Retention page plans the held
    /// things again, and looks again at those whose plan moved; a change of
    /// the quiet period arms the timer again.
    _replanned: Subscription,
}

/// The drop the panel shows, and what it knows about it: the row each
/// thing became, and a look per thing it lists. Kept per drop, so a redraw
/// never looks again (D278); what a clean did is the row's, never kept
/// here (D692).
struct Held {
    arrivals: Arc<[Arrival]>,
    /// The row each thing became in the list, in the order they arrived;
    /// `None` where there is no list.
    rows: Vec<Option<u64>>,
    /// The plan each listed thing is looked at by and painted by — the
    /// page's as it stands, followed when it changes (D290). A clean takes
    /// its own plan when it starts (D279).
    plans: Vec<Plan>,
    /// What a look found, per listed thing; `None` while it looks.
    found: Vec<Option<Findings>>,
}

impl Held {
    /// Whether each thing can be cleaned, and where its row stands — what
    /// [`to_clean`] and [`cleaning`] read.
    fn states(&self, table: Option<&Queue>) -> Vec<(Cleanable, Standing)> {
        self.arrivals
            .iter()
            .zip(&self.rows)
            .map(|(arrival, row)| {
                let standing = match (table, row) {
                    (Some(table), Some(id)) => table.standing(*id),
                    _ => Standing::Gone,
                };
                (clean::cleanable(&arrival.intake), standing)
            })
            .collect()
    }

    /// Whether something runs for its things: a look not yet back, or a
    /// clean or a rewrite of one of its rows (D693).
    fn running(&self, table: Option<&Queue>) -> bool {
        self.found.iter().any(Option::is_none)
            || self
                .states(table)
                .iter()
                .any(|(_, standing)| standing.running())
    }
}

/// The quiet period's timer: which arming is the current one, and the
/// period it was armed for — `None` while nothing is armed.
#[derive(Default)]
struct Quiet {
    generation: u64,
    armed: Option<u32>,
}

/// Which things Clean cleans: every one that can be cleaned and whose row
/// waits, in the order they arrived. A row cleaned, being cleaned or
/// rewritten, or gone from the list is not asked again. Pure.
fn to_clean(states: &[(Cleanable, Standing)]) -> Vec<usize> {
    states
        .iter()
        .enumerate()
        .filter(|(_, (cleanable, standing))| {
            !matches!(cleanable, Cleanable::No(_)) && *standing == Standing::Waiting
        })
        .map(|(index, _)| index)
        .collect()
}

/// Whether a clean of one of the things runs or waits in the line —
/// whichever window asked for it. Pure.
fn cleaning(states: &[(Cleanable, Standing)]) -> bool {
    states
        .iter()
        .any(|(_, standing)| *standing == Standing::Cleaning)
}

/// Whether Clean is offered: not while a clean of its things runs, and not
/// when nothing caught is left to clean. Pure.
fn clean_offered(states: &[(Cleanable, Standing)]) -> bool {
    !cleaning(states) && !to_clean(states).is_empty()
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
        let table = Table::of(cx);
        Self::with_catcher(preferences, catcher, table, window, cx)
    }

    /// The panel over a catcher already made — [`PanelView::new`]'s, which
    /// the platform delivers to, or a test's, which it does not — and the
    /// list it hands its drops to, with none of the platform's hooks: no
    /// floating level, no drag destination. The queue's seam
    /// (`Queue::with_catcher`), for the same reason: a test window has no
    /// native window for either to hang from.
    fn with_catcher(
        preferences: Entity<Preferences>,
        catcher: Entity<Catcher>,
        table: Option<Entity<Queue>>,
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

            // Every move reports here: ours and the user's. Which one
            // it was is decided once the window has stopped moving —
            // see `settle` and `Preferences::we_placed_it`.
            let Some(screen) = screen else {
                return;
            };
            view.settle(screen, window, cx);
        });

        let caught = cx.subscribe(&catcher, |view: &mut Self, catcher, landed: &Landed, cx| {
            // Every drop is rows of the list, overtaken or not — the queue's
            // rule (`every_drop_lands_even_when_overtaken`); only the last
            // is what the panel shows.
            let rows = view.hand_over(&landed.0, cx);
            // The count and the kinds, and nothing else. A log line
            // carrying what was dropped would be the document in a
            // file the product wrote — see `wipemark_log::Elided`.
            let kinds: Vec<Kind> = landed.0.iter().map(|arrival| arrival.intake.kind).collect();
            let listed = rows.iter().flatten().count();
            tracing::info!(?kinds, listed, "something was dropped on the panel");
            if catcher.read(cx).is_caught(&landed.0) {
                view.hold(landed.0.clone(), rows, cx);
            }
            cx.notify();
        });

        let listed = table.as_ref().map(|table| {
            cx.observe(table, |view: &mut Self, _, cx| {
                view.settle_quiet(cx);
                cx.notify();
            })
        });

        // A plan changed on the Retention page is the held things' too: a
        // look taken by the old one says what a clean will not do (D290).
        // And a quiet period changed on the Placement page is the timer's.
        let replanned = cx.observe(&preferences, |view: &mut Self, _, cx| {
            view.replan(cx);
            view.settle_quiet(cx);
        });

        Self {
            preferences,
            focus,
            _geometry: moved,
            moves: 0,
            pressed: None,
            catcher,
            _caught: caught,
            table,
            _listed: listed,
            held: None,
            cleared: false,
            quiet: Quiet::default(),
            _replanned: replanned,
        }
    }

    /// The drop held, if one is.
    fn held(&self) -> Option<&Held> {
        self.held.as_ref()
    }

    /// The drop the middle of the panel shows: the catcher's last, unless
    /// the panel has cleared since (D693). `None` is the invitation.
    fn showing<'a>(&self, cx: &'a App) -> Option<&'a [Arrival]> {
        if self.cleared {
            return None;
        }
        self.catcher.read(cx).caught()
    }

    /// Hand a drop to the list, as a drop on the main window lands (D690):
    /// the row each thing became, in order — `None` for each where there is
    /// no list. A drop that carried nothing is no row.
    fn hand_over(&self, arrivals: &Arc<[Arrival]>, cx: &mut Context<Self>) -> Vec<Option<u64>> {
        let Some(table) = &self.table else {
            if !arrivals.is_empty() {
                tracing::warn!("the panel has no list to hand its drop to; it is shown only");
            }
            return vec![None; arrivals.len()];
        };
        let ids = table.update(cx, |table, cx| table.take_from_panel(arrivals.clone(), cx));
        let mut rows: Vec<Option<u64>> = ids.into_iter().map(Some).collect();
        rows.resize(arrivals.len(), None);
        rows
    }

    /// Show a new drop — in place of whatever was shown (D693) — and look
    /// at what it lists, one thing at a time on the background executor,
    /// each by the plan a clean would take now — so a look says what that
    /// clean refuses before reading (D287).
    fn hold(&mut self, arrivals: Arc<[Arrival]>, rows: Vec<Option<u64>>, cx: &Context<Self>) {
        // Whatever the quiet timer was counting down for is not shown any
        // more.
        self.disarm();
        self.cleared = false;
        if arrivals.is_empty() {
            self.held = None;
            self.settle_quiet(cx);
            return;
        }
        let listed = arrivals.len().min(LISTED);
        let plans = self.plans_now(&arrivals[..listed], cx);
        self.held = Some(Held {
            arrivals: arrivals.clone(),
            rows,
            plans: plans.clone(),
            found: (0..listed).map(|_| None).collect(),
        });
        Self::look(arrivals, plans.into_iter().enumerate().collect(), cx);
        self.settle_quiet(cx);
    }

    /// The plan a clean of each thing would take now.
    fn plans_now(&self, things: &[Arrival], cx: &App) -> Vec<Plan> {
        let preferences = self.preferences.read(cx);
        things
            .iter()
            .map(|thing| preferences.plan_for(&thing.intake))
            .collect()
    }

    /// The preferences changed: plan the held things again, and look again
    /// at each one whose row still waits and whose plan moved — one read
    /// per such thing, never one per frame, and none for a change elsewhere
    /// (D290).
    fn replan(&mut self, cx: &mut Context<Self>) {
        let Some(held) = self.held() else {
            return;
        };
        let arrivals = held.arrivals.clone();
        let plans = self.plans_now(&arrivals[..held.plans.len()], cx);
        let states = held.states(self.table.as_ref().map(|table| table.read(cx)));
        let Some(held) = self.held.as_mut() else {
            return;
        };
        let mut again = Vec::new();
        for (index, plan) in plans.into_iter().enumerate() {
            let cleaned = matches!(states[index].1, Standing::Cleaned(_));
            if held.plans[index] == plan || cleaned {
                continue;
            }
            held.plans[index] = plan.clone();
            held.found[index] = None;
            again.push((index, plan));
        }
        if !again.is_empty() {
            Self::look(arrivals, again, cx);
            cx.notify();
        }
    }

    /// Look at the things of the drop `arrivals` at these places, each by
    /// its plan, one at a time on the background executor. A look lands
    /// only while its drop is held and its plan is still the thing's — a
    /// drop that overtakes it stops the rest, a plan that moved has a look
    /// of its own on the way.
    fn look(arrivals: Arc<[Arrival]>, places: Vec<(usize, Plan)>, cx: &Context<Self>) {
        cx.spawn(async move |view, cx| {
            for (index, plan) in places {
                let thing = arrivals.clone();
                let by = plan.clone();
                let found = cx
                    .background_executor()
                    .spawn(async move { clean::inspect_one(&thing[index], &by) })
                    .await;
                let current = view
                    .update(cx, |view, cx| {
                        view.landed(&arrivals, index, &plan, found, cx)
                    })
                    .unwrap_or(false);
                if !current {
                    break;
                }
            }
        })
        .detach();
    }

    /// A look at the thing `index` of `arrivals`, taken by `plan`, is back:
    /// it is kept only while that drop is held and `plan` is still the
    /// thing's — a look taken by a plan the page has since left is dropped,
    /// whenever it lands (D290). Whether the drop is still held.
    fn landed(
        &mut self,
        arrivals: &Arc<[Arrival]>,
        index: usize,
        plan: &Plan,
        found: Findings,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(held) = self.held_for(arrivals) else {
            return false;
        };
        if held.plans[index] == *plan {
            held.found[index] = Some(found);
            self.settle_quiet(cx);
            cx.notify();
        }
        true
    }

    /// The held drop, if it is still `arrivals`.
    fn held_for(&mut self, arrivals: &Arc<[Arrival]>) -> Option<&mut Held> {
        self.held
            .as_mut()
            .filter(|held| Arc::ptr_eq(&held.arrivals, arrivals))
    }

    /// Where the held things' rows stand.
    fn states(&self, cx: &App) -> Vec<(Cleanable, Standing)> {
        self.held().map_or_else(Vec::new, |held| {
            held.states(self.table.as_ref().map(|table| table.read(cx)))
        })
    }

    /// Clean every caught thing that can be and whose row waits: the list's
    /// own Clean, asked for those rows (D691) — the application's one line,
    /// one at a time, each by the plan the Retention page gives when its
    /// clean starts (D279, D283). No row is made: each thing is the row it
    /// became when it landed. A drop that overtakes it does not stop it —
    /// what was asked for is finished, and the list shows it.
    fn clean(&self, cx: &mut Context<Self>) {
        let Some(table) = self.table.clone() else {
            return;
        };
        let Some(held) = self.held() else {
            return;
        };
        let states = held.states(Some(table.read(cx)));
        if !clean_offered(&states) {
            return;
        }
        let ids: Vec<u64> = to_clean(&states)
            .into_iter()
            .filter_map(|index| held.rows[index])
            .collect();
        table.update(cx, |table, cx| table.clean(&ids, cx));
        cx.notify();
    }

    /// Whether a clean of one of the held things runs or waits in the line.
    #[cfg(test)]
    fn cleaning(&self, cx: &App) -> bool {
        cleaning(&self.states(cx))
    }

    /// Whether something runs for the held things — a look, a clean, a
    /// rewrite.
    fn running(&self, cx: &App) -> bool {
        self.held()
            .is_some_and(|held| held.running(self.table.as_ref().map(|table| table.read(cx))))
    }

    /// Forget the timer counting down, if one is.
    fn disarm(&mut self) {
        if self.quiet.armed.take().is_some() {
            self.quiet.generation += 1;
        }
    }

    /// Arm the quiet period's timer when the panel shows something and
    /// nothing runs for it, and disarm it when something does (D693): the
    /// period counts from the moment the last thing stopped, and a change
    /// of the period on the Placement page arms it again. Never, `0`, arms
    /// nothing.
    fn settle_quiet(&mut self, cx: &Context<Self>) {
        let period = self.preferences.read(cx).panel_clear_after();
        let showing = self.showing(cx).is_some();
        if !showing || period == 0 || self.running(cx) {
            self.disarm();
            return;
        }
        if self.quiet.armed == Some(period) {
            return;
        }
        self.quiet.generation += 1;
        self.quiet.armed = Some(period);
        let mine = self.quiet.generation;
        cx.spawn(async move |view, cx| {
            cx.background_executor()
                .timer(Duration::from_secs(u64::from(period)))
                .await;
            view.update(cx, |view, cx| {
                if view.quiet.generation != mine {
                    // Overtaken: a drop, a clean, or a new period.
                    return;
                }
                view.quiet.armed = None;
                view.clear(cx);
            })
            .ok();
        })
        .detach();
    }

    /// Let go of what the panel shows, and invite the next drop. The rows
    /// stay in the list (D693).
    fn clear(&mut self, cx: &mut Context<Self>) {
        if self.cleared {
            return;
        }
        self.cleared = true;
        self.held = None;
        tracing::info!("the panel cleared after a quiet period; the list keeps its rows");
        cx.notify();
    }

    /// Write down where the user has just put this window, once they
    /// have stopped putting it there.
    ///
    /// Debounced, because a drag is sixty rectangles a second and only
    /// the last one is the answer — and because each one is a row in
    /// SQLite, on the background executor, from the thread that draws
    /// the window.
    ///
    /// And only a person's: the last report is weighed against what this
    /// application asked for and when ([`placement::Placing`], D616) — a
    /// window manager carrying out an ask reports rectangles nobody asked
    /// for, and filing the first of them as the person's answer was
    /// acceptance F8's bug. Under X11 the rectangle weighed is read off
    /// the X server once the window is still — the frame a person sees,
    /// title bar included — rather than taken from GPUI's report, which
    /// is the client's alone.
    fn settle(&mut self, screen: Screen, window: &Window, cx: &Context<Self>) {
        self.moves += 1;
        let mine = self.moves;
        let reported = std::time::Instant::now();
        let frame = screen::frame_of(window);
        #[cfg(target_os = "linux")]
        let measure = crate::x11::get()
            .zip(crate::x11::xid(window))
            .map(|(x11, xid)| (x11, xid, window.scale_factor()));

        cx.spawn(async move |view, cx| {
            cx.background_executor().timer(SETTLE).await;
            #[cfg(target_os = "linux")]
            let frame = match measure {
                Some((x11, xid, scale)) => match x11.frame(xid, false).recv_async().await {
                    Ok(Some(measured)) => screen::logical(measured.outer(), scale),
                    _ => frame,
                },
                None => frame,
            };
            view.update(cx, |view, cx| {
                if view.moves != mine {
                    // Overtaken: the drag is still going.
                    return;
                }
                let screen = screen::of_rect(frame, Some(screen.clone()), cx).unwrap_or(screen);
                view.preferences.update(cx, |preferences, cx| {
                    if preferences.heard(frame, reported) == placement::Heard::Person {
                        preferences.put_by_hand(&screen, frame, cx);
                    }
                });
            })
            .ok();
        })
        .detach();
    }
}

/// Where a result would go, unless the look already said why there will
/// be none or the thing cannot be cleaned. Pure.
fn would_lines(intake: &Intake, plan: &Plan, found: Option<&Findings>) -> Vec<String> {
    let refused = matches!(found, Some(Findings::NotLooked(_)));
    if refused || matches!(clean::cleanable(intake), Cleanable::No(_)) {
        return Vec::new();
    }
    would_happen(plan)
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
            lines.extend(
                would_lines(intake, plan, found)
                    .into_iter()
                    .map(|line| (line, muted)),
            );
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

        let Some(caught) = self.showing(cx) else {
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
        // What a look found, when the drop held is the one caught — it is,
        // once its `Landed` has been heard — and what a clean did, which is
        // the row's, read from the list (D692).
        let held = self
            .held
            .as_ref()
            .filter(|held| self.catcher.read(cx).is_caught(&held.arrivals));
        let states = held.map_or_else(Vec::new, |held| {
            held.states(self.table.as_ref().map(|table| table.read(cx)))
        });
        area.children(
            caught
                .iter()
                .take(LISTED)
                .enumerate()
                .map(|(index, arrival)| {
                    let intake = &arrival.intake;
                    // The plan its look was taken by; the page's, until
                    // the drop is held.
                    let plan = held
                        .and_then(|held| held.plans.get(index).cloned())
                        .unwrap_or_else(|| preferences.plan_for(intake));
                    let done = match states.get(index) {
                        Some((_, Standing::Cleaned(outcome))) => Some(outcome.as_ref()),
                        _ => None,
                    };
                    caught_row(
                        intake,
                        &plan,
                        held.and_then(|held| held.found.get(index)?.as_ref()),
                        done,
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
    /// Clean, once something cleanable has been caught and there is a list
    /// to clean it in: greyed while a clean of its things runs and once
    /// nothing is left to clean, its tooltip saying so.
    fn clean_button(&self, cx: &Context<Self>) -> Option<Button> {
        self.table.as_ref()?;
        let states = self.states(cx);
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
                .label(SharedString::from(t(if cleaning(&states) {
                    Message::PanelCleaning
                } else {
                    Message::PanelClean
                })))
                .disabled(!clean_offered(&states))
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
            // Drag anywhere to move it, as the help says (`panel-help-move`):
            // AppKit's `movableByWindowBackground` on macOS, and this on
            // Linux, where it is the only handle the panel has under
            // Wayland and a second one beside the title bar under X11.
            .when(cfg!(target_os = "linux"), |panel| {
                panel
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|view, event: &gpui::MouseDownEvent, _, _| {
                            view.pressed = Some(event.position);
                        }),
                    )
                    .on_mouse_up(
                        gpui::MouseButton::Left,
                        cx.listener(|view, _: &gpui::MouseUpEvent, _, _| {
                            view.pressed = None;
                        }),
                    )
                    .on_mouse_move(
                        cx.listener(|view, event: &gpui::MouseMoveEvent, window, _| {
                            let Some(from) = view.pressed else {
                                return;
                            };
                            if event.pressed_button != Some(gpui::MouseButton::Left) {
                                view.pressed = None;
                                return;
                            }
                            let travelled = (event.position.x - from.x)
                                .as_f32()
                                .hypot((event.position.y - from.y).as_f32());
                            if travelled >= DRAG_AFTER {
                                view.pressed = None;
                                window.start_window_move();
                            }
                        }),
                    )
            })
            .size_full()
            // The same overlay as the main window's, over the whole panel:
            // a drop anywhere on it lands — its header, its lines, the
            // dismissal line — whatever lies over them (F9b, D652).
            .relative()
            .child(drop::overlay(&self.catcher, cx))
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
                            .anchor(Anchor::TopRight)
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
    use wipemark_store::entry::Origin;

    use super::*;
    use crate::clean::{Refusal, Unable};
    use crate::retention::Destination;

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

    /// Clean cleans what can be cleaned and whose row waits, in arrival
    /// order; it is offered only when there is such a thing and no clean of
    /// its things runs — whichever window asked for it (D691, D692).
    #[test]
    fn clean_is_offered_only_when_something_is_left_to_clean() {
        let text = Cleanable::Text(Encoding::Utf8);
        let png = Cleanable::Picture(Format::Png);
        let tiff = Cleanable::No(Unable::NotYet(Format::Tiff));
        let waiting = || Standing::Waiting;

        let fresh = [(text, waiting()), (tiff, waiting()), (png, waiting())];
        assert_eq!(to_clean(&fresh), [0, 2]);
        assert!(clean_offered(&fresh));

        let busy = [
            (text, Standing::Cleaning),
            (tiff, waiting()),
            (png, waiting()),
        ];
        assert!(cleaning(&busy));
        assert_eq!(to_clean(&busy), [2], "a row in the line is asked again");
        assert!(!clean_offered(&busy), "offered while a clean runs");

        let half = [
            (text, Standing::Elsewhere),
            (tiff, waiting()),
            (png, waiting()),
        ];
        assert_eq!(to_clean(&half), [2], "a thing cleaned is cleaned again");

        let away = [
            (text, Standing::Rewriting),
            (tiff, waiting()),
            (png, Standing::Gone),
        ];
        assert!(
            to_clean(&away).is_empty(),
            "a row being rewritten, or gone from the list, is asked"
        );
        assert!(!clean_offered(&away), "offered with nothing left");
        assert!(!cleaning(&away), "a rewrite reads as a clean");

        assert!(!clean_offered(&[(tiff, waiting())]), "offered over a TIFF");
        assert!(!clean_offered(&[]));
    }

    /// The invitation says the panel cleans and rewriting is in the main
    /// window (В10) — in every language, with no epic number and nothing
    /// saying rewriting is not here.
    #[test]
    fn the_panel_says_rewriting_is_in_the_main_window() {
        let line = t(Message::PanelPending);
        assert!(line.contains("main window"), "{line}");
        assert!(line.contains("Rewrite"), "{line}");
        assert!(!line.contains("not in this version"), "{line}");
        for language in available_languages() {
            let localizer =
                Localizer::for_languages(std::slice::from_ref(&language.id), Rendering::PlainText);
            let line = localizer.format(Message::PanelPending);
            for old in [
                "Cleaning from this window is not",
                "Очистки из этого окна",
                "Bereinigen aus diesem Fenster",
                "not in this version",
                "пока нет",
                "noch nicht",
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

    /// A panel beside the main window's table, in one test window, over
    /// preferences that forget, a scratch folder and — with `journal` — a
    /// batch queue and a document journal in memory: what the tests of the
    /// one list share (D690–D693).
    struct Listed {
        scratch: std::path::PathBuf,
        view: Entity<PanelView>,
        table: Entity<Queue>,
        preferences: Entity<Preferences>,
        work: Option<crate::journal::Work>,
    }

    impl Listed {
        fn new<'a>(
            label: &str,
            journal: bool,
            cx: &'a mut gpui::TestAppContext,
        ) -> (Self, &'a mut gpui::VisualTestContext) {
            use std::cell::RefCell;
            use std::rc::Rc;

            use crate::retention::Homes;

            let scratch = std::env::temp_dir().join(format!(
                "wipemark-panel-list-{label}-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&scratch);
            std::fs::create_dir_all(&scratch).expect("scratch");
            let work = journal.then(Self::work);

            cx.update(gpui_component::init);
            let homes = Homes {
                results: scratch.join("results"),
                kept: scratch.join("kept"),
            };
            type Made = Option<(Entity<PanelView>, Entity<Queue>, Entity<Preferences>)>;
            let slot: Rc<RefCell<Made>> = Rc::default();
            let held = slot.clone();
            let given = work.clone();
            let (_, cx) = cx.add_window_view(move |window, cx| {
                let preferences = cx.new(|cx| Preferences::for_tests(homes, cx));
                let table = cx.new(|cx| Queue::for_tests(preferences.clone(), given, window, cx));
                // Detached, and no `afloat` or `drop::accept`: a test window
                // has no native window for either.
                let catcher = cx.new(|_| Catcher::detached());
                let view = cx.new(|cx| {
                    PanelView::with_catcher(
                        preferences.clone(),
                        catcher,
                        Some(table.clone()),
                        window,
                        cx,
                    )
                });
                *held.borrow_mut() = Some((view.clone(), table, preferences));
                gpui_component::Root::new(view, window, cx)
            });
            let (view, table, preferences) = slot.take().expect("the window builder ran");
            (
                Self {
                    scratch,
                    view,
                    table,
                    preferences,
                    work,
                },
                cx,
            )
        }

        /// A batch queue and a journal over a store in memory, with nothing
        /// on duty — what the application builds at launch.
        fn work() -> crate::journal::Work {
            use wipemark_store::Store;

            let store = Arc::new(Store::in_memory().expect("memory"));
            let engine = crate::engine_host::EngineHandle::new().0;
            let queue = wipemark_queue::Queue::with_source(
                Arc::clone(&store),
                wipemark_queue::Durability::Memory { detail: None },
                Arc::new(engine.clone()),
            )
            .expect("the queue opens");
            crate::journal::Work {
                queue: Arc::new(queue),
                journal: crate::journal::Journal::new(store),
                engine,
                whereto: crate::journal::Going::default(),
            }
        }

        fn file(&self, name: &str, bytes: &[u8]) -> std::path::PathBuf {
            let path = self.scratch.join(name);
            std::fs::write(&path, bytes).expect("scratch file");
            path
        }

        /// Drop `paths` on the panel, and let the looks land.
        fn drop_on_panel(&self, paths: &[&std::path::PathBuf], cx: &mut gpui::VisualTestContext) {
            let catcher = cx.update(|_, cx| self.view.read(cx).catcher.clone());
            catcher.update(cx, |catcher, cx| {
                catcher.land(
                    paths
                        .iter()
                        .map(|path| wipemark_intake::Handed::Path((*path).clone()))
                        .collect(),
                    cx,
                );
            });
            cx.run_until_parked();
        }

        /// The table's rows, in arrival order.
        fn rows(&self, cx: &mut gpui::VisualTestContext) -> Vec<u64> {
            cx.update(|_, cx| self.table.read(cx).ids())
        }

        /// The rows the panel holds for what it shows.
        fn held_rows(&self, cx: &mut gpui::VisualTestContext) -> Option<Vec<Option<u64>>> {
            cx.update(|_, cx| self.view.read(cx).held().map(|held| held.rows.clone()))
        }

        /// What the panel says each thing it shows came to — the verdict of
        /// its row's clean, read the way the panel paints it.
        fn verdicts(&self, cx: &mut gpui::VisualTestContext) -> Vec<Option<&'static str>> {
            cx.update(|_, cx| {
                self.view
                    .read(cx)
                    .states(cx)
                    .into_iter()
                    .map(|(_, standing)| match standing {
                        Standing::Cleaned(outcome) => Some(outcome.verdict.id()),
                        _ => None,
                    })
                    .collect()
            })
        }

        /// The journal's rows, once the writer's thread has written `count`
        /// of them with every one in `state` — waited for in real time, at
        /// most a minute: the writer is a thread of its own.
        fn journal_rows(
            &self,
            count: usize,
            state: &str,
            cx: &gpui::VisualTestContext,
        ) -> Vec<wipemark_store::JournalRow> {
            let journal = &self.work.as_ref().expect("a journal").journal;
            let deadline = std::time::Instant::now() + Duration::from_secs(60);
            loop {
                cx.executor().advance_clock(crate::queue::TICK);
                cx.run_until_parked();
                let rows = journal.rows();
                if rows.len() >= count && rows.iter().all(|row| row.state == state) {
                    return rows;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "the journal never held {count} rows {state}: {:?}",
                    rows.iter()
                        .map(|row| (row.origin.clone(), row.state.clone()))
                        .collect::<Vec<_>>()
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }

    impl Drop for Listed {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.scratch).ok();
        }
    }

    /// The live check's case 13, as a test: a drop of a marked text, a
    /// plain one, a Gemini picture and a TIFF on a panel built without the
    /// platform's hooks, beside the main window's table; the looks land,
    /// Clean cleans every thing that can be cleaned through the list's one
    /// line, each row says what happened, and on disk there is a result for
    /// the marked text and the picture and for nothing else. Clean is then
    /// greyed: nothing is left to clean. This retires the series' deviation
    /// 17.
    #[gpui::test]
    fn a_drop_on_the_panel_is_looked_at_and_cleaned(cx: &mut gpui::TestAppContext) {
        let (listed, cx) = Listed::new("drop", false, cx);
        let marked = listed.file(
            "marked.md",
            "# Notes\n\nTwo\u{200B} marks\u{200B} here.\n".as_bytes(),
        );
        let plain = listed.file("plain.md", b"# Plain\n\nNothing to find.\n");
        let torch = listed.file(
            "torch-1025.png",
            &std::fs::read(format!(
                "{}/../../fixtures/image/gemini/torch-1025.png",
                env!("CARGO_MANIFEST_DIR")
            ))
            .expect("fixture"),
        );
        let tiff = listed.file("scan.tif", b"II*\x00\x08\x00\x00\x00\x00\x00\x00\x00");
        listed.drop_on_panel(&[&marked, &plain, &torch, &tiff], cx);

        let view = listed.view.clone();
        let looked = cx.update(|_, cx| {
            let held = view.read(cx).held().expect("the drop is held");
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
        let offered = cx.update(|_, cx| clean_offered(&view.read(cx).states(cx)));
        assert!(
            offered,
            "Clean is not offered over a drop with things to clean"
        );

        view.update(cx, |view, cx| view.clean(cx));
        let busy = cx.update(|_, cx| view.read(cx).cleaning(cx));
        assert!(busy, "Clean does not say it is cleaning");
        cx.run_until_parked();

        assert_eq!(
            listed.verdicts(cx),
            [
                Some("cleaned"),
                Some("nothing-found"),
                Some("cleaned"),
                None
            ],
            "a thing was skipped, or the TIFF was cleaned"
        );
        let mut names: Vec<String> = std::fs::read_dir(&listed.scratch)
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
        assert!(
            !std::fs::read_to_string(listed.scratch.join("marked.cleaned.md"))
                .expect("result")
                .contains('\u{200B}')
        );
        let after = cx.update(|_, cx| {
            let panel = view.read(cx);
            (panel.cleaning(cx), clean_offered(&panel.states(cx)))
        });
        assert_eq!(
            after,
            (false, false),
            "Clean is still offered with nothing left"
        );
    }

    /// D690: a drop on the panel is one row per thing in the main window's
    /// table and in the document journal, at once — before anything is
    /// cleaned — under the origin `panel`; and the panel holds those rows.
    #[gpui::test]
    fn a_panel_drop_is_one_row_per_thing_in_the_list_and_the_journal(
        cx: &mut gpui::TestAppContext,
    ) {
        let (listed, cx) = Listed::new("rows", true, cx);
        let one = listed.file("one.md", "One\u{200B} mark.\n".as_bytes());
        let two = listed.file("two.md", b"Nothing here.\n");
        listed.drop_on_panel(&[&one, &two], cx);

        let rows = listed.rows(cx);
        assert_eq!(rows.len(), 2, "the drop is not two rows of the list");
        let origins = cx.update(|_, cx| {
            rows.iter()
                .map(|id| listed.table.read(cx).origin_of(*id))
                .collect::<Vec<_>>()
        });
        assert_eq!(
            origins,
            [Some(Origin::Panel), Some(Origin::Panel)],
            "the rows do not say the panel handed them over"
        );
        assert_eq!(
            listed.held_rows(cx),
            Some(vec![Some(rows[0]), Some(rows[1])]),
            "the panel does not hold the rows its things became"
        );

        let journal = listed.journal_rows(2, "waiting", cx);
        assert_eq!(journal.len(), 2, "not one journal row per thing");
        assert!(
            journal.iter().all(|row| row.origin == "panel"),
            "{journal:?}"
        );
        // Settled: no read of the journal lists a row of the panel's twice.
        let rows = listed.rows(cx);
        assert_eq!(rows.len(), 2, "a read of the journal listed a row twice");
    }

    /// D690: whatever the panel's catcher lands goes to the list — a text
    /// dragged onto it (`Handed::Text`, the road a text drag takes once it
    /// reaches the catcher) as much as a file: one row, from the panel, and
    /// its Clean cleans that row, the text coming back with no file
    /// written.
    #[gpui::test]
    fn a_text_dropped_on_the_panel_is_a_row_too(cx: &mut gpui::TestAppContext) {
        let (listed, cx) = Listed::new("text", true, cx);
        let catcher = cx.update(|_, cx| listed.view.read(cx).catcher.clone());
        catcher.update(cx, |catcher, cx| {
            catcher.land(
                vec![wipemark_intake::Handed::Text(
                    "Dragged\u{200B} text.".to_owned(),
                )],
                cx,
            );
        });
        cx.run_until_parked();
        let rows = listed.rows(cx);
        assert_eq!(rows.len(), 1, "a dropped text is no row");
        let origin = cx.update(|_, cx| listed.table.read(cx).origin_of(rows[0]));
        assert_eq!(origin, Some(Origin::Panel));
        listed.journal_rows(1, "waiting", cx);

        let view = listed.view.clone();
        view.update(cx, |view, cx| view.clean(cx));
        cx.run_until_parked();
        assert_eq!(listed.verdicts(cx), [Some("cleaned")]);
        assert_eq!(listed.rows(cx), rows, "the text's Clean made a row");
        let journal = listed.journal_rows(1, "done", cx);
        assert_eq!(journal.len(), 1);
    }

    /// D691, D692: the panel's Clean cleans the rows its things became — no
    /// second row in the list or the journal — and what the panel says is
    /// the row's; a clean asked from the main window's row reads on the
    /// panel too.
    #[gpui::test]
    fn the_panels_clean_is_that_row_and_the_main_windows_reads_on_the_panel(
        cx: &mut gpui::TestAppContext,
    ) {
        let (listed, cx) = Listed::new("one-row", true, cx);
        let one = listed.file("one.md", "One\u{200B} mark.\n".as_bytes());
        let two = listed.file("two.md", "Two\u{200B} marks\u{200B}.\n".as_bytes());
        listed.drop_on_panel(&[&one, &two], cx);
        let rows = listed.rows(cx);
        listed.journal_rows(2, "waiting", cx);

        // The main window cleans the second row; the panel says so.
        listed
            .table
            .update(cx, |table, cx| table.clean(&[rows[1]], cx));
        cx.run_until_parked();
        assert_eq!(
            listed.verdicts(cx),
            [None, Some("cleaned")],
            "a clean from the main window does not read on the panel"
        );

        // The panel's Clean takes what is left: the first row, once.
        let view = listed.view.clone();
        view.update(cx, |view, cx| view.clean(cx));
        cx.run_until_parked();
        assert_eq!(listed.verdicts(cx), [Some("cleaned"), Some("cleaned")]);
        assert_eq!(listed.rows(cx), rows, "the panel's Clean made a row");
        let statuses = cx.update(|_, cx| {
            rows.iter()
                .map(|id| matches!(listed.table.read(cx).standing(*id), Standing::Cleaned(_)))
                .collect::<Vec<_>>()
        });
        assert_eq!(statuses, [true, true], "the list's rows are not cleaned");

        let journal = listed.journal_rows(2, "done", cx);
        assert_eq!(journal.len(), 2, "the panel's Clean wrote a second row");
        assert!(journal.iter().all(|row| row.origin == "panel"));
        assert!(
            listed.scratch.join("one.cleaned.md").exists()
                && listed.scratch.join("two.cleaned.md").exists()
        );
    }

    /// D693, D694: after a minute of quiet the panel lets go of what it
    /// shows and invites the next drop; the rows stay in the list and the
    /// journal. Not before the minute, and never while a clean of its
    /// things runs — the minute counts from its end.
    #[gpui::test]
    fn the_quiet_period_clears_the_panel_and_not_the_rows(cx: &mut gpui::TestAppContext) {
        let (listed, cx) = Listed::new("quiet", true, cx);
        let one = listed.file("one.md", "One\u{200B} mark.\n".as_bytes());
        listed.drop_on_panel(&[&one], cx);
        let rows = listed.rows(cx);
        assert_eq!(rows.len(), 1);
        let showing = |listed: &Listed, cx: &mut gpui::VisualTestContext| {
            cx.update(|_, cx| listed.view.read(cx).showing(cx).is_some())
        };

        cx.executor().advance_clock(Duration::from_secs(59));
        cx.run_until_parked();
        assert!(showing(&listed, cx), "the panel cleared before the minute");

        // A clean is something running: the minute starts again at its end.
        let view = listed.view.clone();
        view.update(cx, |view, cx| view.clean(cx));
        cx.run_until_parked();
        assert_eq!(listed.verdicts(cx), [Some("cleaned")]);
        cx.executor().advance_clock(Duration::from_secs(30));
        cx.run_until_parked();
        assert!(
            showing(&listed, cx),
            "the minute did not start again after the clean"
        );

        cx.executor().advance_clock(Duration::from_secs(31));
        cx.run_until_parked();
        assert!(!showing(&listed, cx), "the panel did not clear");
        assert_eq!(listed.rows(cx), rows, "clearing the panel took a row");
        let journal = listed.journal_rows(1, "done", cx);
        assert_eq!(journal.len(), 1, "clearing the panel took a journal row");

        // Never is never.
        listed.preferences.update(cx, |preferences, cx| {
            preferences.select_panel_clear_after(0, cx);
        });
        listed.drop_on_panel(&[&one], cx);
        cx.executor().advance_clock(Duration::from_secs(3600));
        cx.run_until_parked();
        assert!(showing(&listed, cx), "the panel cleared with Never chosen");
    }

    /// D693: a new drop replaces what the panel shows and adds rows of its
    /// own; the rows of the drop before stay in the list.
    #[gpui::test]
    fn a_new_drop_replaces_the_panels_view_and_adds_rows(cx: &mut gpui::TestAppContext) {
        let (listed, cx) = Listed::new("replace", true, cx);
        let one = listed.file("one.md", b"First.\n");
        let two = listed.file("two.md", b"Second.\n");
        listed.drop_on_panel(&[&one], cx);
        let first = listed.rows(cx);
        assert_eq!(first.len(), 1);

        listed.drop_on_panel(&[&two], cx);
        let both = listed.rows(cx);
        assert_eq!(both.len(), 2, "the second drop is not a row of its own");
        assert_eq!(both[0], first[0], "the first drop's row went");
        assert_eq!(
            listed.held_rows(cx),
            Some(vec![Some(both[1])]),
            "the panel does not show the second drop"
        );
        let shown = cx.update(|_, cx| {
            let held = listed.view.read(cx).held().expect("held");
            held.arrivals
                .iter()
                .map(|arrival| arrival.intake.name.clone())
                .collect::<Vec<_>>()
        });
        assert_eq!(shown, [Some("two.md".to_owned())]);
        listed.journal_rows(2, "waiting", cx);
    }

    /// A panel over preferences that forget, in a test window, with
    /// `link.md -> target.md` in a scratch folder — what the two tests of
    /// the look's plan share (Y4).
    #[cfg(unix)]
    struct LinkPanel {
        scratch: std::path::PathBuf,
        link: std::path::PathBuf,
        view: Entity<PanelView>,
        preferences: Entity<Preferences>,
    }

    #[cfg(unix)]
    impl LinkPanel {
        fn new<'a>(
            label: &str,
            cx: &'a mut gpui::TestAppContext,
        ) -> (Self, &'a mut gpui::VisualTestContext) {
            use std::cell::RefCell;
            use std::rc::Rc;

            use crate::retention::Homes;

            let scratch =
                std::env::temp_dir().join(format!("wipemark-panel-{label}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&scratch);
            std::fs::create_dir_all(&scratch).expect("scratch");
            let target = scratch.join("target.md");
            std::fs::write(&target, "A zero\u{200B}width space.\n").expect("target");
            let link = scratch.join("link.md");
            std::os::unix::fs::symlink(&target, &link).expect("link");

            cx.update(gpui_component::init);
            let homes = Homes {
                results: scratch.join("results"),
                kept: scratch.join("kept"),
            };
            type Made = Option<(Entity<PanelView>, Entity<Preferences>)>;
            let slot: Rc<RefCell<Made>> = Rc::default();
            let held = slot.clone();
            let (_, cx) = cx.add_window_view(move |window, cx| {
                let preferences = cx.new(|cx| Preferences::for_tests(homes, cx));
                let catcher = cx.new(|_| Catcher::detached());
                let view = cx.new(|cx| {
                    PanelView::with_catcher(preferences.clone(), catcher, None, window, cx)
                });
                *held.borrow_mut() = Some((view.clone(), preferences));
                gpui_component::Root::new(view, window, cx)
            });
            let (view, preferences) = slot.take().expect("the window builder ran");
            let panel = Self {
                scratch,
                link,
                view,
                preferences,
            };
            (panel, cx)
        }

        fn destination(&self, destination: Destination, cx: &mut gpui::VisualTestContext) {
            self.preferences.update(cx, |preferences, cx| {
                preferences.select_destination(destination, cx)
            });
            cx.run_until_parked();
        }

        fn drop_link(&self, cx: &mut gpui::VisualTestContext) {
            let catcher = cx.update(|_, cx| self.view.read(cx).catcher.clone());
            catcher.update(cx, |catcher, cx| {
                catcher.land(vec![wipemark_intake::Handed::Path(self.link.clone())], cx);
            });
            cx.run_until_parked();
        }

        /// What the panel holds for the link: whether its look is the link
        /// refusal (`Some(true)`), a look that found the one U+200B
        /// (`Some(false)`) or anything else (`None`), and the lines it
        /// paints under it about where a result would go.
        fn looked(&self, cx: &mut gpui::VisualTestContext) -> (Option<bool>, Vec<String>) {
            cx.update(|_, cx| {
                let held = self.view.read(cx).held().expect("the drop is held");
                let found = held.found[0].as_ref();
                let refused = match found {
                    Some(Findings::NotLooked(Refusal::Link(path))) if *path == self.link => {
                        Some(true)
                    }
                    Some(Findings::Text { change: 1, .. }) => Some(false),
                    other => {
                        eprintln!("the look: {other:?}");
                        None
                    }
                };
                let would = would_lines(&held.arrivals[0].intake, &held.plans[0], found);
                (refused, would)
            })
        }
    }

    #[cfg(unix)]
    impl Drop for LinkPanel {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.scratch).ok();
        }
    }

    /// The panel's look is asked with the plan a clean would take: in
    /// place of a symbolic link it is the link refusal, beside it a look
    /// (Y4).
    #[cfg(unix)]
    #[gpui::test]
    fn the_look_is_taken_by_the_plan_a_clean_would_take(cx: &mut gpui::TestAppContext) {
        let (panel, cx) = LinkPanel::new("look-plan", cx);
        panel.destination(Destination::Replace, cx);
        panel.drop_link(cx);
        let (refused, would) = panel.looked(cx);
        assert_eq!(refused, Some(true), "the look is not the link refusal");
        assert!(would.is_empty(), "{would:?}");

        panel.destination(Destination::Beside, cx);
        panel.drop_link(cx);
        let (refused, would) = panel.looked(cx);
        assert_eq!(refused, Some(false), "the link was not looked at");
        assert!(!would.is_empty(), "nothing said of where it would go");
    }

    /// A change on the Retention page reaches a drop already held: the
    /// link dropped under *Beside* and the page then switched to *In
    /// place* is the link refusal, with no new drop and no would-happen
    /// lines; switched back, it is looked at again (Y4, D290).
    #[cfg(unix)]
    #[gpui::test]
    fn the_look_follows_a_change_of_plan(cx: &mut gpui::TestAppContext) {
        let (panel, cx) = LinkPanel::new("look-follows", cx);
        panel.drop_link(cx);
        let (refused, would) = panel.looked(cx);
        assert_eq!(refused, Some(false), "the link was not looked at");
        assert!(!would.is_empty());

        panel.destination(Destination::Replace, cx);
        let (refused, would) = panel.looked(cx);
        assert_eq!(refused, Some(true), "the look did not follow the page");
        assert!(would.is_empty(), "{would:?}");

        panel.destination(Destination::Beside, cx);
        let (refused, would) = panel.looked(cx);
        assert_eq!(refused, Some(false), "the link was not looked at");
        assert!(!would.is_empty());
    }

    /// A look taken by a plan the page has since left — the drop's own,
    /// slow off its volume — lands after the look by the new plan, and is
    /// dropped: the panel keeps the newer look, its lines and its plan
    /// (Z2, D290). The late look is landed by hand, because the test's
    /// executor runs both looks on this thread in an order of its own.
    #[cfg(unix)]
    #[gpui::test]
    fn a_look_by_an_older_plan_does_not_overwrite_the_newer(cx: &mut gpui::TestAppContext) {
        let (panel, cx) = LinkPanel::new("look-stale", cx);
        panel.drop_link(cx);
        let (arrivals, beside) = cx.update(|_, cx| {
            let held = panel.view.read(cx).held().expect("the drop is held");
            (held.arrivals.clone(), held.plans[0].clone())
        });
        let late = clean::inspect_one(&arrivals[0], &beside);
        assert!(matches!(late, Findings::Text { change: 1, .. }), "{late:?}");

        panel.destination(Destination::Replace, cx);
        let (refused, would) = panel.looked(cx);
        assert_eq!(refused, Some(true), "the look did not follow the page");
        assert!(would.is_empty(), "{would:?}");

        let held = panel
            .view
            .update(cx, |view, cx| view.landed(&arrivals, 0, &beside, late, cx));
        assert!(held, "the drop is no longer held");
        let (refused, would) = panel.looked(cx);
        assert_eq!(
            refused,
            Some(true),
            "a look by the older plan overwrote the newer"
        );
        assert!(would.is_empty(), "{would:?}");
        let (plan, now) = cx.update(|_, cx| {
            let held = panel.view.read(cx).held().expect("the drop is held");
            let now = panel
                .preferences
                .read(cx)
                .plan_for(&held.arrivals[0].intake);
            (held.plans[0].clone(), now)
        });
        assert!(
            plan == now && plan != beside,
            "the held plan is not the page's"
        );
    }
}
