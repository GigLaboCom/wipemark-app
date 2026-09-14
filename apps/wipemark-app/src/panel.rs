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
//! The clean-up itself is not implemented yet, so the panel says what
//! it will do and does not pretend to do it — the same bargain the MCP
//! tools, the Engine banner and the disabled tray item make. What is
//! real today is the window: that it arrives where it was told to, on
//! the display it was told to, that it can be moved and resized, that
//! it takes a drop and names it, and that Escape dismisses it.

use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    actions, div, px, AnyWindowHandle, App, Bounds, Corner, Div, Entity, FocusHandle, Focusable,
    Global, KeyBinding, Pixels, SharedString, Size, Subscription, Window, WindowBounds, WindowKind,
    WindowOptions,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::popover::Popover;
use gpui_component::{h_flex, v_flex, ActiveTheme, Root, Sizable as _, StyledExt as _};
use wipemark_i18n::{args, t, t_args, Message};
use wipemark_intake::{Arrived, Format, Intake, Kind};

use crate::drop::{self, Catcher, Landed};
use crate::icon::{Icon, IconName};
use crate::placement;
use crate::retention::Plan;
use crate::screen::{self, Screen};
use crate::settings::Preferences;
use crate::wording::{self, would_happen, Tone};

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
}

impl PanelView {
    fn new(preferences: Entity<Preferences>, window: &mut Window, cx: &mut Context<Self>) -> Self {
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

        afloat(window);

        // The drop zone, and the AppKit destination behind it. Both
        // belong here rather than in `show`: a window that is not built
        // yet has no view to insert anything under, and a destination
        // installed a frame later is a drag that was refused once.
        let catcher = cx.new(|cx| Catcher::new(window, cx));
        drop::accept(window);
        let caught = cx.subscribe(&catcher, |_, catcher, _: &Landed, cx| {
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
            cx.notify();
        });

        Self {
            preferences,
            focus,
            _geometry: moved,
            moves: 0,
            catcher,
            _caught: caught,
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
/// — and under it, when there is one, the thing worth being told. Last,
/// what would happen to it: the one place today where the Retention
/// page's choices can be seen against a real thing.
fn caught_row(intake: &Intake, plan: &Plan, cx: &App) -> impl IntoElement {
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
        .children(would_happen(plan).into_iter().map(|line| {
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
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
        area.children(caught.iter().take(LISTED).map(|arrival| {
            let intake = &arrival.intake;
            caught_row(intake, &preferences.plan_for(intake), cx)
        }))
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
                div()
                    .text_xs()
                    .text_color(muted)
                    .child(SharedString::from(t(Message::PanelDismiss))),
            )
    }
}
