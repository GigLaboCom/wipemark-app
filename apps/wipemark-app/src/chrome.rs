//! A window's own frame, where the desktop draws none, and how a window
//! that is already open is brought forward on Linux.
//!
//! **The title bar (F3, D590–D593).** GNOME's compositor offers no
//! server-side decorations under Wayland. GPUI asks for them, logs
//! "Server-side decorations requested, but the Wayland server does not
//! support them. Falling back to client-side decorations", and from then
//! on the window's frame is the application's to draw. The frame's
//! *edges* were already drawn — gpui-component's `Root` wraps every
//! window in its `WindowBorder`, which under client-side decorations sets
//! a 20 px client inset, paints a shadow and a one-pixel border, and
//! starts an interactive resize from a band around the visible edge. Its
//! *title bar* was nobody's: a window that could not be dragged and had
//! no close, minimise or maximise button. [`framed`] draws
//! gpui-component's `TitleBar` over a window's content exactly when
//! [`draws_title_bar`] says the decorations are client-side, and leaves
//! the content alone otherwise — on macOS, under X11, under a Wayland
//! compositor that draws the frame itself (KDE), and on the panel, which
//! never calls it (the design rule: a summoned window arrives in place).
//!
//! **The close button is the platform's close (D592).** The title bar's
//! close must run what the desktop's own close button runs — the tray's
//! minimise under X11 (D343), Settings' rectangle written, Compare's
//! save-or-ask (D415, D446). GPUI keeps one should-close callback per
//! window and offers no way to call it, so a window registers it here,
//! through [`on_should_close`], which hands the same function to GPUI
//! and keeps it by the window's id; [`close`] runs it and removes the
//! window only when it says yes. gpui-component's default for that
//! button, `remove_window`, would skip every one of those.
//!
//! **A window that closes itself does it after the event, as the close
//! button would (F23, D752–D754).** ⌘W in Compare or Settings, Escape in
//! Settings or the panel, a drop that makes way, an answer to Compare's
//! question: each is an event the window is still handling when it asks
//! to go. [`close_later`] closes it once that event has returned — on
//! macOS through AppKit's own `close`, so that GPUI forgets the window in
//! the same call that takes it off the screen and away from the keyboard,
//! and the release of the key that closed it finds no window GPUI has
//! already forgotten (`ERROR main : window not found`, the owner's log of
//! 2026-10-13 09:37:54).
//!
//! **Bringing a window forward (F2, D594).** What gpui-pre 0.3.8
//! (`gpui-pre-linux`) does with the two calls this application makes:
//!
//! * `App::activate` — nothing on Linux, under either backend; it logs
//!   "activate is not implemented on Linux, ignoring the call".
//! * `Window::activate_window` under **X11** sends `_NET_ACTIVE_WINDOW`
//!   to the root window (source indication 1, `CurrentTime`), which a
//!   window manager answers by raising and focusing the window.
//! * `Window::activate_window` under **Wayland** asks `xdg_activation_v1`
//!   for a token stamped with the last pointer press GPUI saw and with
//!   the *target* window's surface, then activates the target with it.
//!   The press was on another surface (the main window's gear) or on none
//!   of ours (the tray's menu belongs to the shell), so mutter does not
//!   honour the token and only marks the window as wanting attention —
//!   the window stays behind, the button looks dead.
//!
//! So under Wayland a window that is open and not in front is closed and
//! opened again where it was ([`Forward::Reopen`]) — a window the
//! compositor has just mapped is put in front — and everywhere else it
//! is activated ([`Forward::Activate`]). The reopening itself is each
//! window's: what it may lose decides whether it may (D595, D596).

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, App, Decorations, FontWeight, Global, SharedString, Window, WindowId,
};
use gpui_component::{h_flex, v_flex, TitleBar};

/// Whether a window with these decorations draws a title bar of its own:
/// only when the frame is the application's to draw (D590).
pub fn draws_title_bar(decorations: Decorations) -> bool {
    matches!(decorations, Decorations::Client { .. })
}

/// `content`, under a title bar reading `title` when the window's
/// decorations are client-side, and untouched otherwise.
///
/// The bar sits *above* the content rather than over it, and outside
/// whatever the content positions its dialogs against: a dialog's
/// backdrop covers the window's own page and leaves the bar to be dragged
/// and closed, as the desktop's bar would be. `title` is asked only when
/// a bar is drawn, and is the platform title's text — [`crate::title`],
/// plain, no isolation marks.
pub fn framed(
    content: impl IntoElement,
    title: impl FnOnce() -> String,
    window: &Window,
) -> AnyElement {
    frame(content, title, draws_title_bar(window.window_decorations()))
}

/// [`framed`], told whether to draw the bar — which a test can say and
/// the test platform's windows, always server-decorated, cannot.
fn frame(content: impl IntoElement, title: impl FnOnce() -> String, bar: bool) -> AnyElement {
    if !bar {
        return content.into_any_element();
    }
    v_flex()
        .size_full()
        .child(
            TitleBar::new()
                .on_close_window(|_, window, cx| close(window, cx))
                .child(
                    h_flex()
                        .id("window-title")
                        .debug_selector(|| "window-title".into())
                        .flex_1()
                        .h_full()
                        .min_w(px(0.0))
                        .justify_center()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(div().truncate().child(SharedString::from(title()))),
                ),
        )
        .child(
            div()
                .debug_selector(|| "window-content".into())
                .flex()
                .flex_col()
                .flex_1()
                .min_h(px(0.0))
                .w_full()
                .child(content),
        )
        .into_any_element()
}

/// What a window's close button runs before it closes, kept by the window.
type ShouldClose = Rc<dyn Fn(&mut Window, &mut App) -> bool>;

/// Every window's [`ShouldClose`], by its id. An entry outlives its
/// window until the next registration prunes it, which is harmless: a
/// window id is never handed out twice.
#[derive(Default)]
struct Closers(HashMap<WindowId, ShouldClose>);

impl Global for Closers {}

/// [`Window::on_window_should_close`], kept where the title bar's close
/// can run it too (D592). Every should-close in this application goes
/// through here; GPUI keeps one per window, and so does this — the last
/// one registered.
pub fn on_should_close(
    window: &Window,
    cx: &mut App,
    should_close: impl Fn(&mut Window, &mut App) -> bool + 'static,
) {
    let should_close: ShouldClose = Rc::new(should_close);
    let open: HashSet<WindowId> = cx
        .windows()
        .iter()
        .map(|handle| handle.window_id())
        .collect();
    let closers = cx.default_global::<Closers>();
    closers.0.retain(|id, _| open.contains(id));
    closers
        .0
        .insert(window.window_handle().window_id(), should_close.clone());
    window.on_window_should_close(cx, move |window, cx| should_close(window, cx));
}

/// Close `window` the way the desktop's close button does: what it
/// registered through [`on_should_close`] is asked first, and the window
/// goes only on a yes. A window that registered nothing closes, as it
/// would under the desktop's button.
pub fn close(window: &mut Window, cx: &mut App) {
    let id = window.window_handle().window_id();
    let should_close = cx
        .try_global::<Closers>()
        .and_then(|closers| closers.0.get(&id).cloned());
    if should_close.is_none_or(|should_close| should_close(window, cx)) {
        close_later(window, cx);
    }
}

/// Close `window` from inside an event it is handling — the action of a
/// key pressed in it, a click, a drop — once that event has returned
/// (acceptance F23, D752).
///
/// `remove_window` from in there has GPUI forget the window when the
/// event's update returns, while the platform window lives on until a
/// task GPUI queues for it runs. On macOS that task is AppKit's `close`,
/// and until it runs the `NSWindow` is still on screen and still the key
/// window. Whatever reaches it meanwhile is handed to a window GPUI no
/// longer has: GPUI logs `ERROR main : window not found` and drops it.
/// On the owner's Mac it was the release of the ⌘W that closed Compare,
/// 42 ms after its press and before that task had run (2026-10-13: the
/// system log has the press at 09:37:54.683 and the release at .725, ours
/// the error at .728, and the system log the ⌘'s release at .739).
///
/// So on macOS the window is closed by AppKit's own `close`, the close
/// button's road: GPUI's close callback forgets the window inside that
/// call, before AppKit takes it off the screen, and no event can fall
/// between the two (D753). Not from inside the update: AppKit answers
/// `close` at once, and GPUI cannot take its answer while the application
/// is borrowed — the reason `screen::later`, macOS-only, exists
/// (D728). Elsewhere, and in a test, whose window has no `NSWindow`, it
/// is `remove_window` in a task on the foreground executor, so the window
/// is still there for the rest of the event that asked (D754). Under
/// Linux, E6 saw the same line once after every programmatic close, from
/// a frame callback delivered after the window had gone; no close of the
/// platform's own is reachable there to go through instead.
///
/// Whatever the window had to ask before it goes — save, or keep the
/// edit — is asked before this: AppKit's `close`, unlike the button's
/// `performClose:`, asks no should-close.
pub fn close_later(window: &Window, cx: &App) {
    #[cfg(target_os = "macos")]
    if crate::screen::later(window, cx, |native| native.close()) {
        return;
    }
    let handle = window.window_handle();
    cx.spawn(async move |cx| {
        // Gone already — a second ⌘W queued a second close — is what was
        // asked for.
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    })
    .detach();
}

/// How a window that is already open is brought in front of the others.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forward {
    /// `activate_window`: the platform raises it (macOS, X11).
    Activate,
    /// Close it and open it again where it was: a Wayland compositor puts
    /// a window it has just mapped in front, and honours no activation
    /// GPUI can ask for (D594).
    Reopen,
}

/// [`Forward`] on the display server GPUI calls `compositor` —
/// `cx.compositor_name()`: "Wayland", "X11", "headless", or "" where the
/// platform names none (macOS).
pub fn forward_on(compositor: &str) -> Forward {
    if compositor == "Wayland" {
        Forward::Reopen
    } else {
        Forward::Activate
    }
}

/// [`forward_on`] for the display server this process is on.
pub fn forward(cx: &App) -> Forward {
    forward_on(cx.compositor_name())
}

/// Press `keys` in a test's window and let go, in the order AppKit hands
/// them over (F23): the press dispatched, then its release given to the
/// window's input callback — GPUI's own, the one that logged `window not
/// found` — before anything the press queued has run, and then all of it.
/// Panics when the press took the window away from under its own
/// release.
#[cfg(test)]
pub(crate) fn press_and_let_go(window: &mut gpui::VisualTestContext, keys: &str) {
    let keystroke = gpui::Keystroke::parse(keys).unwrap_or_else(|error| panic!("{keys}: {error}"));
    let handle = window.window_handle();
    window.dispatch_keystroke(handle, keystroke.clone());
    assert!(
        window.cx.windows().contains(&handle),
        "{keys} took the window away inside its own dispatch: its release \
         finds no window, and GPUI logs `window not found` (F23)"
    );
    window.simulate_event(gpui::KeyUpEvent { keystroke });
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use gpui::{AssetSource as _, Tiling};
    use gpui_component::IconNamed as _;

    use super::*;
    use crate::assets::WipemarkAssets;

    /// D590: a bar of our own exactly where the frame is ours to draw —
    /// never a second one under a desktop that draws its own.
    #[test]
    fn a_title_bar_is_drawn_only_under_client_side_decorations() {
        assert!(!draws_title_bar(Decorations::Server));
        assert!(draws_title_bar(Decorations::Client {
            tiling: Tiling::default()
        }));
        assert!(draws_title_bar(Decorations::Client {
            tiling: Tiling::tiled()
        }));
    }

    /// D594: reopened on Wayland only. X11 has `_NET_ACTIVE_WINDOW`, and
    /// macOS and the test platform name no compositor.
    #[test]
    fn only_wayland_reopens_a_window_to_bring_it_forward() {
        assert_eq!(forward_on("Wayland"), Forward::Reopen);
        assert_eq!(forward_on("X11"), Forward::Activate);
        assert_eq!(forward_on("headless"), Forward::Activate);
        assert_eq!(forward_on(""), Forward::Activate);
    }

    /// D591: every glyph `TitleBar` paints resolves here. gpui-component
    /// asks for them by its own names; a name this directory does not
    /// ship is a blank control, logged once a frame. Delete one of the
    /// four files and this is red.
    #[test]
    fn every_window_control_glyph_resolves() {
        use gpui_component::IconName;
        for name in [
            IconName::WindowMinimize,
            IconName::WindowMaximize,
            IconName::WindowRestore,
            IconName::WindowClose,
        ] {
            let path = name.path();
            let found = WipemarkAssets.load(&path).ok().flatten();
            assert!(found.is_some(), "{path} is not shipped: a blank control");
        }
    }

    /// A window with one registered should-close, counting its asks.
    fn closable(
        cx: &mut gpui::TestAppContext,
        answer: bool,
    ) -> (gpui::AnyWindowHandle, Rc<Cell<u32>>) {
        struct Blank;
        impl Render for Blank {
            fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
                div()
            }
        }
        let asked = Rc::new(Cell::new(0));
        let counter = asked.clone();
        let window = cx.add_window(move |window, cx| {
            on_should_close(window, cx, move |_, _| {
                counter.set(counter.get() + 1);
                answer
            });
            Blank
        });
        (window.into(), asked)
    }

    /// D592: the title bar's close and the desktop's are one road. A
    /// should-close that says no keeps the window under both, and is asked
    /// once by each; one that says yes lets both close it. Make `close`
    /// what gpui-component's button does by default — `remove_window` and
    /// nothing else — and the first half is red.
    #[gpui::test]
    fn the_title_bars_close_is_the_platforms_close(cx: &mut gpui::TestAppContext) {
        let (kept, asked) = closable(cx, false);
        kept.update(cx, |_, window, cx| close(window, cx))
            .expect("the window is open");
        cx.run_until_parked();
        assert!(cx.windows().contains(&kept), "closed over a no");
        assert_eq!(asked.get(), 1, "the should-close was not asked");

        let mut platform = gpui::VisualTestContext::from_window(kept, cx);
        assert!(
            !platform.simulate_close(),
            "the desktop's close ignored the no"
        );
        assert_eq!(asked.get(), 2, "one function, asked by both roads");

        let (gone, asked) = closable(cx, true);
        gone.update(cx, |_, window, cx| close(window, cx))
            .expect("the window is open");
        cx.run_until_parked();
        assert!(!cx.windows().contains(&gone), "a yes kept the window");
        assert_eq!(asked.get(), 1);
    }

    /// F23, D752: a window that asks to close from inside one of its own
    /// updates is still there when that update returns — for the rest of
    /// the event that asked, a key's release on macOS — and gone once the
    /// foreground executor has run; asked twice, it closes once and the
    /// second ask finds nothing to do. Make `close_later` the
    /// `remove_window` it replaces and the first assertion is red.
    #[gpui::test]
    fn a_window_that_closes_itself_outlasts_the_event_that_asked(cx: &mut gpui::TestAppContext) {
        struct Blank;
        impl Render for Blank {
            fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
                div()
            }
        }
        let window: gpui::AnyWindowHandle = cx.add_window(|_, _| Blank).into();
        window
            .update(cx, |_, window, cx| close_later(window, cx))
            .expect("the window is open");
        assert!(
            cx.windows().contains(&window),
            "gone inside the update that asked"
        );
        window
            .update(cx, |_, window, cx| close_later(window, cx))
            .expect("the window is open for a second ask");
        cx.run_until_parked();
        assert!(!cx.windows().contains(&window), "never closed");
    }

    /// A window that registered nothing closes, as under the desktop's
    /// button with no hook (`tray::closes_registers_nothing`).
    #[gpui::test]
    fn a_window_with_no_should_close_closes(cx: &mut gpui::TestAppContext) {
        struct Blank;
        impl Render for Blank {
            fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
                div()
            }
        }
        let window: gpui::AnyWindowHandle = cx.add_window(|_, _| Blank).into();
        window
            .update(cx, |_, window, cx| close(window, cx))
            .expect("the window is open");
        cx.run_until_parked();
        assert!(!cx.windows().contains(&window));
    }

    /// What the bar takes off the top is the bar's height, and the content
    /// fills the rest of the window, the dialogs' coordinate system with it
    /// — and with no bar, the content is the whole window, untouched.
    #[gpui::test]
    fn the_bar_sits_above_the_content_and_takes_only_its_height(cx: &mut gpui::TestAppContext) {
        struct Framed(bool);
        impl Render for Framed {
            fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
                frame(
                    div().debug_selector(|| "page".into()).size_full(),
                    || "Wipemark".to_owned(),
                    self.0,
                )
            }
        }
        cx.update(gpui_component::init);
        for bar in [true, false] {
            let (_, window) = cx.add_window_view(move |_, _| Framed(bar));
            window.run_until_parked();
            let viewport = window.update(|window, _| window.viewport_size());
            let page = window.debug_bounds("page").expect("the page is painted");
            assert_eq!(page.size.width, viewport.width, "bar {bar}");
            if bar {
                let height = gpui_component::TITLE_BAR_HEIGHT;
                assert!(
                    window.debug_bounds("window-title").is_some(),
                    "no title drawn"
                );
                assert_eq!(page.origin.y, height, "the page is not under the bar");
                assert_eq!(page.size.height, viewport.height - height);
            } else {
                assert!(
                    window.debug_bounds("window-title").is_none(),
                    "a bar with none asked"
                );
                assert_eq!(page.size, viewport);
            }
        }
    }
}
