//! What is under the pointer: GPUI told when the pointer leaves a window,
//! and every tooltip of ours attached so that it goes with its control.
//!
//! **F34** (the owner's acceptance on macOS, 2026-10-16): a tooltip stayed
//! up after the pointer had left its control, until something else
//! happened. Two causes, neither of them ours, and both handled here.
//!
//! **The pointer leaving a window (D795).** GPUI 0.3.8 takes the pointer's
//! position from every move, press and scroll, and from `MouseExited` it
//! takes only the modifiers (`gpui-pre` `window.rs:5499`): the window goes
//! on believing the pointer stands where it last moved inside it. On macOS
//! the last move inside is followed by `mouseExited:` and nothing else —
//! the view's tracking area reports moves only within its own rectangle
//! (`gpui-pre-macos` `window.rs:2488`) — so a pointer that leaves across a
//! control, or quickly past one near an edge, leaves that control hovered.
//! GPUI's own tooltip listens to moves, presses and scrolls only
//! (`div.rs:3712`) and stays. gpui-component's is told by `on_hover`, which
//! does hear the exit (`div.rs:3178`) — and the next frame undoes it: the
//! hover is checked again as the element is painted (`div.rs:3154`)
//! against a hit test made from the stale position (`window.rs:3616`), the
//! control is hovered again, and the tooltip comes back. [`init`] puts
//! [`Edge`] in every window built after it, as a `Root` plugin: an exit
//! with no button held becomes a move to just outside the window, so GPUI
//! knows the pointer is gone and every hover in the window ends. Held, it
//! is a drag, and the platform keeps sending its moves.
//!
//! **A control that goes away under the pointer (D796).** gpui-component's
//! `Button::tooltip` hands the tooltip to one overlay per window, and only
//! the control's own hover-out or a press on it takes it down
//! (`tooltip.rs:247`; the overlay, gpui-base `tooltip.rs:177`). A control
//! that is no longer painted sends neither — the toolbar's Pause as the
//! queue drains, a row scrolled out of its list, the panel's Clean as the
//! panel clears — and its tooltip stays where the control was, until
//! another tooltip or a press. GPUI's own tooltip is registered by the
//! element each frame it is painted, and goes with it. [`Tip`] attaches
//! that one, with gpui-component's look; nothing here calls `tooltip(…)`
//! (`every_tooltip_is_attached_through_tip`).
//!
//! The write-up for upstream, with a reproduction of each, is
//! `dev/architecture/gpui-pin.md`, "The pointer leaving a window, and
//! tooltips".

use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    canvas, div, Action, AnyView, App, Context, Div, Interactivity, MouseDownEvent, MouseExitEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, PlatformInput, Point, SharedString, Size, Stateful,
    Window,
};
use gpui_base::{Root, RootPlugin};
use gpui_component::button::Button;
use gpui_component::tooltip::Tooltip;

/// What a tooltip of ours is painted under — the one way a test sees one.
pub(crate) const TIP: &str = "tip";

/// Put [`Edge`] in every window built from now on. Called once from
/// `main`, after `gpui_component::init` and before the first window.
pub fn init(cx: &mut App) {
    Root::register_plugin::<Edge>(cx, |_, _| Edge::default());
}

/// A window's edge, as far as the pointer is concerned (D795): a `Root`
/// plugin, so it is in every window — the main one, Settings, Compare, the
/// panel and any later one — with no window's own code asked.
#[derive(Default)]
pub struct Edge {
    /// Whether a button is held, as the last press, release or move said.
    /// An exit with one held is a drag leaving the window, whose moves the
    /// platform goes on sending.
    held: Rc<Cell<bool>>,
}

impl RootPlugin for Edge {}

impl Render for Edge {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let held = self.held.clone();
        canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                let pressed = held.clone();
                window.on_mouse_event(move |_: &MouseDownEvent, phase, _, _| {
                    if phase.capture() {
                        pressed.set(true);
                    }
                });
                let released = held.clone();
                window.on_mouse_event(move |_: &MouseUpEvent, phase, _, _| {
                    if phase.capture() {
                        released.set(false);
                    }
                });
                let moved = held.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, _| {
                    if phase.capture() {
                        moved.set(event.pressed_button.is_some());
                    }
                });
                let exited = held.clone();
                window.on_mouse_event(move |event: &MouseExitEvent, phase, window, cx| {
                    if !phase.capture() || exited.get() || event.pressed_button.is_some() {
                        return;
                    }
                    let position = outside(event.position, window.viewport_size());
                    let modifiers = event.modifiers;
                    // After the exit has been handed round: a move
                    // dispatched from inside a dispatch would find no
                    // listeners, GPUI having taken them for this one.
                    window.defer(cx, move |window, cx| {
                        window.dispatch_event(
                            PlatformInput::MouseMove(MouseMoveEvent {
                                position,
                                pressed_button: None,
                                modifiers,
                            }),
                            cx,
                        );
                    });
                });
            },
        )
        .size_0()
    }
}

/// Where the pointer is once it has left a window of `size`: where the
/// platform says it left, or — when that is still inside, on the edge
/// pixel — one pixel past the nearest edge.
pub(crate) fn outside(position: Point<Pixels>, size: Size<Pixels>) -> Point<Pixels> {
    let (x, y) = (f32::from(position.x), f32::from(position.y));
    let (width, height) = (f32::from(size.width), f32::from(size.height));
    let inside = (0.0..width).contains(&x) && (0.0..height).contains(&y);
    if !inside {
        return position;
    }
    let to_left = x;
    let to_right = width - x;
    let to_top = y;
    let to_bottom = height - y;
    let nearest = to_left.min(to_right).min(to_top).min(to_bottom);
    let mut out = position;
    if nearest == to_left {
        out.x = Pixels::from(-1.0);
    } else if nearest == to_right {
        out.x = Pixels::from(width + 1.0);
    } else if nearest == to_top {
        out.y = Pixels::from(-1.0);
    } else {
        out.y = Pixels::from(height + 1.0);
    }
    out
}

/// A tooltip attached so that it goes with its control (D796): GPUI's own,
/// which the element registers each frame it is painted, in
/// gpui-component's [`Tooltip`] look — the text, and for
/// [`Tip::tip_with_action`] whatever the window says the action is bound
/// to, read when it is shown.
pub trait Tip: Sized {
    /// `text` as this control's tooltip.
    fn tip(self, text: impl Into<SharedString>) -> Self;

    /// `text` as this control's tooltip, and beside it the keystroke
    /// `action` is bound to in `context`.
    fn tip_with_action(
        self,
        text: impl Into<SharedString>,
        action: &dyn Action,
        context: Option<&str>,
    ) -> Self;
}

impl Tip for Button {
    fn tip(mut self, text: impl Into<SharedString>) -> Self {
        attach(self.interactivity(), text.into(), None);
        self
    }

    fn tip_with_action(
        mut self,
        text: impl Into<SharedString>,
        action: &dyn Action,
        context: Option<&str>,
    ) -> Self {
        attach(
            self.interactivity(),
            text.into(),
            Some((action.boxed_clone(), context.map(SharedString::new))),
        );
        self
    }
}

impl Tip for Stateful<Div> {
    fn tip(mut self, text: impl Into<SharedString>) -> Self {
        attach(self.interactivity(), text.into(), None);
        self
    }

    fn tip_with_action(
        mut self,
        text: impl Into<SharedString>,
        action: &dyn Action,
        context: Option<&str>,
    ) -> Self {
        attach(
            self.interactivity(),
            text.into(),
            Some((action.boxed_clone(), context.map(SharedString::new))),
        );
        self
    }
}

/// The action a tooltip names the keystroke of, and the key context it is
/// looked up in.
type Bound = (Box<dyn Action>, Option<SharedString>);

fn attach(interactivity: &mut Interactivity, text: SharedString, bound: Option<Bound>) {
    let bound = bound.map(Rc::new);
    interactivity.tooltip(move |window, cx| {
        let tooltip = Tooltip::new(text.clone())
            .when_some(bound.clone(), |tooltip, bound| {
                let (action, context) = &*bound;
                tooltip.action(action.as_ref(), context.as_deref())
            })
            .build(window, cx);
        cx.new(|_| Shown(tooltip)).into()
    });
}

/// A tooltip as shown: gpui-component's, under the one selector a test
/// finds it by.
struct Shown(AnyView);

impl Render for Shown {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().debug_selector(|| TIP.into()).child(self.0.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use gpui::{
        point, px, size, Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext,
    };

    use super::*;

    /// A window with one tipped button at the top left, and room around it.
    struct Bench {
        /// Whether the button is painted at all.
        shown: bool,
    }

    impl Render for Bench {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(
                    div()
                        .absolute()
                        .left(px(10.))
                        .top(px(10.))
                        .when(self.shown, |this| {
                            this.child(
                                Button::new("tipped")
                                    .label("A")
                                    .w(px(60.))
                                    .h(px(20.))
                                    .tip("A tooltip"),
                            )
                        }),
                )
        }
    }

    /// Over the button.
    fn on() -> Point<Pixels> {
        point(px(20.0), px(20.0))
    }

    /// A window with [`Bench`] in it, built as every window of ours is —
    /// in a `Root` — with [`init`] asked first when `edge` says so.
    fn bench(cx: &mut TestAppContext, edge: bool) -> (Entity<Bench>, &mut VisualTestContext) {
        cx.update(gpui_component::init);
        if edge {
            cx.update(init);
        }
        let slot: Rc<std::cell::RefCell<Option<Entity<Bench>>>> = Rc::default();
        let held = slot.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|_| Bench { shown: true });
            *held.borrow_mut() = Some(view.clone());
            Root::new(view, window, cx)
        });
        cx.run_until_parked();
        let view = slot.take().expect("the window builder ran");
        (view, cx)
    }

    /// A fresh frame, painted.
    fn frame(cx: &mut VisualTestContext) {
        cx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
    }

    /// Every delay a tooltip has — shown after half a second,
    /// gpui-component's taken down after a grace of a third — run out.
    fn settle(cx: &VisualTestContext) {
        for _ in 0..3 {
            cx.executor().advance_clock(Duration::from_secs(1));
            cx.run_until_parked();
        }
    }

    /// Whether a tooltip is up in a fresh frame: one of ours, by its
    /// selector, or one of gpui-component's, which its overlay builds anew
    /// in every frame it paints it — so a test written against [`Tip`]
    /// still sees the tooltip when `tip` is pointed back at
    /// `Button::tooltip`, the protection deleted.
    fn up(cx: &mut VisualTestContext, made: &Rc<Cell<u32>>) -> bool {
        let before = made.get();
        frame(cx);
        made.get() > before || cx.debug_bounds(TIP).is_some()
    }

    /// How many of gpui-component's tooltip views have been made.
    fn counting(cx: &mut VisualTestContext) -> Rc<Cell<u32>> {
        let made = Rc::new(Cell::new(0u32));
        let counter = made.clone();
        cx.update(|_, cx| {
            cx.observe_new::<Tooltip>(move |_, _, _| counter.set(counter.get() + 1))
                .detach();
        });
        made
    }

    /// The pointer onto the button, and the tooltip's delay run out.
    fn hover(cx: &mut VisualTestContext) {
        frame(cx);
        cx.simulate_mouse_move(on(), None, Modifiers::default());
        settle(cx);
    }

    /// The platform's exit, as macOS sends it after the last move inside:
    /// nothing else follows.
    fn leave(cx: &mut VisualTestContext) {
        cx.simulate_event(MouseExitEvent {
            position: point(px(20.0), px(-3.0)),
            pressed_button: None,
            modifiers: Modifiers::default(),
        });
        settle(cx);
    }

    /// D795: once the pointer has left a window, GPUI knows it is outside.
    /// Without [`Edge`] — `init` registering nothing — the window keeps
    /// the last position inside and this goes red.
    #[gpui::test]
    fn the_pointer_leaving_a_window_is_told_to_gpui(cx: &mut TestAppContext) {
        let (_, cx) = bench(cx, true);
        hover(cx);
        leave(cx);
        let (position, viewport) =
            cx.update(|window, _| (window.mouse_position(), window.viewport_size()));
        let inside = position.x >= px(0.0)
            && position.y >= px(0.0)
            && position.x < viewport.width
            && position.y < viewport.height;
        assert!(!inside, "GPUI still has the pointer at {position:?}");
    }

    /// D795 (F34): a tooltip goes when the pointer leaves the window across
    /// its control, and does not come back with the next frame. Red without
    /// [`Edge`]: GPUI's own tooltip never hears the exit.
    #[gpui::test]
    fn a_tooltip_goes_when_the_pointer_leaves_the_window(cx: &mut TestAppContext) {
        let (_, cx) = bench(cx, true);
        let made = counting(cx);
        hover(cx);
        assert!(up(cx, &made), "the tooltip was never shown");
        leave(cx);
        assert!(!up(cx, &made), "the tooltip stayed after the pointer left");
        settle(cx);
        assert!(!up(cx, &made), "the tooltip came back with a later frame");
    }

    /// A drag that leaves the window is not cut short: with a button held,
    /// the exit is left alone and the platform's own moves go on.
    #[gpui::test]
    fn a_drag_leaving_the_window_keeps_its_place(cx: &mut TestAppContext) {
        let (_, cx) = bench(cx, true);
        frame(cx);
        cx.simulate_mouse_move(on(), None, Modifiers::default());
        cx.simulate_mouse_down(on(), MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(on(), MouseButton::Left, Modifiers::default());
        cx.simulate_event(MouseExitEvent {
            position: point(px(20.0), px(-3.0)),
            pressed_button: None,
            modifiers: Modifiers::default(),
        });
        cx.run_until_parked();
        let position = cx.update(|window, _| window.mouse_position());
        assert_eq!(position, on(), "a held button's exit moved the pointer");
    }

    /// D796 (F34): a control that stops being painted under a pointer that
    /// has not moved takes its tooltip with it. Red with `tip` pointed back
    /// at gpui-component's `Button::tooltip`, whose overlay keeps the
    /// tooltip until another one or a press.
    #[gpui::test]
    fn a_tooltip_goes_with_its_control(cx: &mut TestAppContext) {
        let (view, cx) = bench(cx, true);
        let made = counting(cx);
        hover(cx);
        assert!(up(cx, &made), "the tooltip was never shown");
        cx.update(|_, cx| {
            view.update(cx, |bench, cx| {
                bench.shown = false;
                cx.notify();
            })
        });
        settle(cx);
        assert!(!up(cx, &made), "the tooltip outlived its control");
        cx.simulate_mouse_move(point(px(300.0), px(300.0)), None, Modifiers::default());
        settle(cx);
        assert!(
            !up(cx, &made),
            "the tooltip outlived its control and the pointer"
        );
    }

    /// Where the pointer is put once it has left: kept when the platform
    /// already said outside, and past the nearest edge when it said the
    /// edge pixel.
    #[test]
    fn the_pointer_is_put_just_outside_the_nearest_edge() {
        let window = size(px(400.0), px(300.0));
        let above = point(px(20.0), px(-3.0));
        assert_eq!(outside(above, window), above);
        assert_eq!(
            outside(point(px(0.0), px(150.0)), window),
            point(px(-1.0), px(150.0))
        );
        assert_eq!(
            outside(point(px(399.5), px(150.0)), window),
            point(px(401.0), px(150.0))
        );
        assert_eq!(
            outside(point(px(200.0), px(0.5)), window),
            point(px(200.0), px(-1.0))
        );
        assert_eq!(
            outside(point(px(200.0), px(299.0)), window),
            point(px(200.0), px(301.0))
        );
    }

    /// D796: every tooltip in this application is attached through [`Tip`]
    /// — no `tooltip(…)` of gpui-component's, whose overlay outlives a
    /// control that goes away (and no bare GPUI one either, so there is one
    /// road and one look). Comments are not code.
    #[test]
    fn every_tooltip_is_attached_through_tip() {
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut found = Vec::new();
        let mut stack = vec![source.clone()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("the source directory reads") {
                let path = entry.expect("an entry").path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().is_none_or(|ext| ext != "rs") || path.ends_with("hover.rs") {
                    continue;
                }
                let text = std::fs::read_to_string(&path).expect("a source file reads");
                for (number, line) in text.lines().enumerate() {
                    let code = line.trim_start();
                    if code.starts_with("//") {
                        continue;
                    }
                    if code.contains(".tooltip(") || code.contains(".tooltip_with_action(") {
                        let name = path.strip_prefix(&source).unwrap_or(&path).display();
                        found.push(format!("{name}:{}", number + 1));
                    }
                }
            }
        }
        found.sort();
        assert!(
            found.is_empty(),
            "attached without hover::Tip: {}",
            found.join(", ")
        );
    }
}
