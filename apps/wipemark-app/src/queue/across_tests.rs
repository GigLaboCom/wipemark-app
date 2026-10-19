//! The table sideways (D811–D813; the owner, 2026-10-19: "horizontal
//! scrolling in the main window is broken — make it work"). Before, in a
//! window narrower than the columns the right-hand ones — Actions among
//! them — were cut off, nothing moved sideways, and a sideways swipe
//! scrolled the rows up and down instead. Now the header and the rows move
//! sideways together, by a trackpad's swipe, by Shift and the wheel and by
//! the bar along the bottom of the rows; in a window wide enough nothing
//! moves. Measured where a person looks: where the Actions title and a
//! row's Actions button were painted.

use std::path::PathBuf;

use gpui::{
    point, px, Bounds, Entity, ExternalPaths, FileDropEvent, Modifiers, MouseButton,
    MouseDownEvent, MouseUpEvent, Pixels, Point, ScrollDelta, ScrollWheelEvent, Size,
    TestAppContext, VisualTestContext,
};

use super::tests::{queue_with, Scratch};
use super::{Column, Queue, NAME_MIN};

/// A note: what each row is.
const NOTE: &str = "# Notes\n\nA zero\u{200B}width space.\n";

/// Half of a 1440-point display: the columns do not fit.
const NARROW: Size<Pixels> = Size {
    width: px(720.0),
    height: px(560.0),
};

/// Wider than every column at its least.
const WIDE: Size<Pixels> = Size {
    width: px(1600.0),
    height: px(560.0),
};

/// A table of `count` notes on pages of ten, and the newest row's id —
/// the row the first page shows on top.
fn table<'a>(
    scratch: &Scratch,
    count: usize,
    cx: &'a mut TestAppContext,
) -> (Entity<Queue>, u64, &'a mut VisualTestContext) {
    let files: Vec<PathBuf> = (0..count)
        .map(|n| scratch.file(&format!("note-{n:02}.md"), NOTE.as_bytes()))
        .collect();
    let (queue, _, cx) = queue_with(cx, scratch, None);
    queue.update(cx, |queue, cx| {
        queue.set_page_size(10, cx);
        queue.hand(files, cx);
    });
    cx.run_until_parked();
    let newest = cx
        .update(|_, cx| queue.read(cx).ids())
        .last()
        .copied()
        .expect("the notes are rows");
    (queue, newest, cx)
}

/// The window resized to `at` and drawn.
fn resize(at: Size<Pixels>, cx: &mut VisualTestContext) {
    cx.simulate_resize(at);
    cx.run_until_parked();
    let width = cx.update(|window, _| window.viewport_size().width);
    assert_eq!(width, at.width, "the window was not resized");
}

/// Where `selector` was painted in the last frame, if it was. `debug_bounds`
/// takes only a `'static` selector, and these are made at run time.
fn found(selector: String, cx: &mut VisualTestContext) -> Option<Bounds<Pixels>> {
    cx.debug_bounds(Box::leak(selector.into_boxed_str()))
}

/// Where `selector` was painted in the last frame.
fn painted(selector: String, cx: &mut VisualTestContext) -> Bounds<Pixels> {
    found(selector.clone(), cx).unwrap_or_else(|| panic!("nothing was painted as {selector}"))
}

/// The Actions title and row `id`'s Actions button as last painted — and,
/// wherever the table stands, the button under its title.
fn actions(id: u64, cx: &mut VisualTestContext) -> (Bounds<Pixels>, Bounds<Pixels>) {
    let head = painted(format!("queue-head-{:?}", Column::Actions), cx);
    let button = found(format!("row-actions-{id}"), cx).unwrap_or_else(|| {
        panic!("row {id} is no longer drawn: the rows were scrolled up or down past it")
    });
    let middle = button.center().x;
    assert!(
        head.left() <= middle && middle <= head.right(),
        "row {id}'s Actions button, centred at {middle:?}, is not under its title at \
         {:?}..{:?}",
        head.left(),
        head.right()
    );
    (head, button)
}

/// The wheel turned at `at`.
fn wheel(at: Point<Pixels>, delta: ScrollDelta, modifiers: Modifiers, cx: &mut VisualTestContext) {
    cx.simulate_event(ScrollWheelEvent {
        position: at,
        delta,
        modifiers,
        ..Default::default()
    });
    cx.run_until_parked();
}

/// A trackpad's swipe of `x` points sideways at `at`: negative brings
/// the right-hand columns in.
fn swipe(at: Point<Pixels>, x: f32, cx: &mut VisualTestContext) {
    wheel(
        at,
        ScrollDelta::Pixels(point(px(x), px(0.0))),
        Modifiers::default(),
        cx,
    );
}

/// Whether `bounds` lies across the window, left to right.
fn across(bounds: Bounds<Pixels>, width: Pixels) -> bool {
    bounds.left() >= px(0.0) && bounds.right() <= width
}

/// The columns that do not fit come into view, the header and the rows
/// moving by one offset: a sideways swipe over the rows, Shift and the
/// wheel (what X11 and Wayland hand GPUI for it, and macOS for a mouse's
/// wheel: the turn on the X axis), and a press on the bar along the
/// bottom of the rows. A swipe sideways never scrolls the rows up or down,
/// and the wheel never scrolls them sideways.
#[gpui::test]
fn columns_that_do_not_fit_scroll_into_view_header_and_rows_together(cx: &mut TestAppContext) {
    let scratch = Scratch::new("across");
    let (_queue, newest, cx) = table(&scratch, 12, cx);
    resize(NARROW, cx);
    let width = NARROW.width;

    let (head, button) = actions(newest, cx);
    assert!(
        head.right() > width,
        "the premise: the Actions column fits a window {width:?} wide ({head:?})"
    );
    let over = point(width / 2.0, button.center().y);

    // A trackpad's sideways swipe, over the rows.
    swipe(over, -4000.0, cx);
    let (swiped_head, swiped_button) = actions(newest, cx);
    assert!(
        across(swiped_head, width),
        "a sideways swipe left the Actions title outside the window: {swiped_head:?}"
    );
    assert!(
        across(swiped_button, width),
        "a sideways swipe left the row's Actions button outside the window: {swiped_button:?}"
    );
    let moved = head.left() - swiped_head.left();
    assert!(moved > px(0.0), "nothing moved sideways");
    assert_eq!(
        button.left() - swiped_button.left(),
        moved,
        "the rows and the header moved by different amounts"
    );
    assert_eq!(
        swiped_button.top(),
        button.top(),
        "a sideways swipe scrolled the rows up or down"
    );

    // Back to the left edge, and Shift with the wheel: a turn on X.
    swipe(over, 4000.0, cx);
    let (home, _) = actions(newest, cx);
    assert_eq!(home, head, "a swipe back did not bring the table back");
    wheel(
        over,
        ScrollDelta::Lines(point(-3.0, 0.0)),
        Modifiers::shift(),
        cx,
    );
    let (shifted_head, shifted_button) = actions(newest, cx);
    let moved = head.left() - shifted_head.left();
    assert!(
        moved > px(0.0),
        "Shift and the wheel moved nothing sideways"
    );
    assert_eq!(button.left() - shifted_button.left(), moved);
    assert_eq!(shifted_button.top(), button.top());

    // The wheel alone scrolls the rows and nothing sideways.
    wheel(
        over,
        ScrollDelta::Lines(point(0.0, -1.0)),
        Modifiers::default(),
        cx,
    );
    let (wheeled_head, wheeled_button) = actions(newest, cx);
    assert_eq!(wheeled_head, shifted_head, "the wheel moved the header");
    assert_eq!(wheeled_button.left(), shifted_button.left());
    assert!(
        wheeled_button.top() < shifted_button.top(),
        "the wheel no longer scrolls the rows"
    );
    wheel(
        over,
        ScrollDelta::Lines(point(0.0, 1.0)),
        Modifiers::default(),
        cx,
    );
    swipe(over, 4000.0, cx);

    // The bars, shown whatever the theme says. The rows' bar is at the
    // window's right edge, not the table's, which is past it: pressed near
    // its bottom, the rows go down.
    cx.update(|_, cx| {
        gpui_component::Theme::set_scrollbar_mode(gpui_component::scroll::ScrollbarMode::Always, cx)
    });
    cx.run_until_parked();
    let down = painted("queue-rows-bar".to_owned(), cx);
    assert_eq!(
        down.right(),
        width,
        "the rows' bar is not at the window's edge"
    );
    press(point(down.center().x, down.bottom() - px(20.0)), cx);
    assert!(
        found(format!("row-actions-{newest}"), cx).is_none_or(|bounds| bounds.top() < button.top()),
        "the rows' bar at the window's right edge did not scroll the rows"
    );
    wheel(
        over,
        ScrollDelta::Pixels(point(px(0.0), px(4000.0))),
        Modifiers::default(),
        cx,
    );
    assert_eq!(actions(newest, cx), (head, button), "not back at the top");

    // The sideways bar, along the bottom of the rows, pressed near its
    // right end: the thumb's middle goes to the press, and the table to
    // its end.
    let bar = painted("queue-across-bar".to_owned(), cx);
    assert!(
        bar.bottom() <= painted("queue-footer".to_owned(), cx).top(),
        "the bar is not above the footer: {bar:?}"
    );
    press(point(bar.right() - px(40.0), bar.center().y), cx);
    let (barred_head, barred_button) = actions(newest, cx);
    assert_eq!(
        barred_head, swiped_head,
        "the bar did not take the table to its end"
    );
    assert_eq!(barred_button, swiped_button);
}

/// A press and its release at `at`.
fn press(at: Point<Pixels>, cx: &mut VisualTestContext) {
    cx.simulate_event(MouseDownEvent {
        position: at,
        modifiers: Modifiers::default(),
        button: MouseButton::Left,
        click_count: 1,
        first_mouse: false,
    });
    cx.simulate_event(MouseUpEvent {
        position: at,
        modifiers: Modifiers::default(),
        button: MouseButton::Left,
        click_count: 1,
    });
    cx.run_until_parked();
}

/// With room for every column nothing moves sideways and the table is as
/// it was: the Actions title at the window's right edge, the name taking
/// what is left, and a swipe moving nothing. Narrowed, the name keeps its
/// least width ([`NAME_MIN`]) and the rest scrolls; widened again, the
/// offset goes back to nothing.
#[gpui::test]
fn a_wide_window_has_nothing_to_scroll_and_widening_takes_the_offset_back(cx: &mut TestAppContext) {
    let scratch = Scratch::new("wide");
    let (_queue, newest, cx) = table(&scratch, 3, cx);
    resize(WIDE, cx);

    let (head, button) = actions(newest, cx);
    let name = painted(format!("queue-head-{:?}", Column::Name), cx);
    let preview = painted(format!("queue-head-{:?}", Column::Preview), cx);
    assert_eq!(head.right(), WIDE.width, "Actions is not at the right edge");
    assert_eq!(preview.left(), px(0.0), "Preview is not at the left edge");
    assert!(name.size.width > NAME_MIN, "the name took no slack");

    let over = point(WIDE.width / 2.0, button.center().y);
    swipe(over, -4000.0, cx);
    let (swiped_head, swiped_button) = actions(newest, cx);
    assert_eq!(swiped_head, head, "a table that fits moved sideways");
    assert_eq!(swiped_button, button);

    // Narrowed: the name at its least, the rest past the edge.
    resize(NARROW, cx);
    let name = painted(format!("queue-head-{:?}", Column::Name), cx);
    assert_eq!(
        name.size.width, NAME_MIN,
        "the name was crushed before the table began to scroll"
    );
    let (narrow_head, narrow_button) = actions(newest, cx);
    assert!(narrow_head.right() > NARROW.width);
    swipe(
        point(NARROW.width / 2.0, narrow_button.center().y),
        -4000.0,
        cx,
    );
    let (scrolled, _) = actions(newest, cx);
    assert!(scrolled.left() < narrow_head.left(), "nothing scrolled");

    // Widened again: back where it began.
    resize(WIDE, cx);
    let (widened_head, widened_button) = actions(newest, cx);
    assert_eq!(widened_head, head, "the offset outlived the room to scroll");
    assert_eq!(widened_button, button);
}

/// The offset is the table's, not the page's: a page turned, a filter
/// that lets nothing through and the filter taken off leave the columns
/// where the person scrolled them.
#[gpui::test]
fn the_offset_is_kept_across_a_page_turn_and_a_filter(cx: &mut TestAppContext) {
    let scratch = Scratch::new("kept");
    let (queue, newest, cx) = table(&scratch, 12, cx);
    resize(NARROW, cx);

    let (head, button) = actions(newest, cx);
    swipe(point(NARROW.width / 2.0, button.center().y), -200.0, cx);
    let (scrolled, _) = actions(newest, cx);
    assert!(scrolled.left() < head.left(), "nothing scrolled");

    queue.update(cx, |queue, cx| queue.turn_page(true, cx));
    cx.run_until_parked();
    let oldest = cx
        .update(|_, cx| queue.read(cx).ids())
        .first()
        .copied()
        .expect("rows");
    let (turned, _) = actions(oldest, cx);
    assert_eq!(turned, scrolled, "a page turn moved the columns");

    queue.update(cx, |queue, cx| {
        queue.filter.keyword = "nothing has this keyword".to_owned();
        cx.notify();
    });
    cx.run_until_parked();
    let filtered = painted(format!("queue-head-{:?}", Column::Actions), cx);
    assert_eq!(filtered, scrolled, "a filter moved the header");

    queue.update(cx, |queue, cx| {
        queue.filter.keyword.clear();
        queue.page = 0;
        cx.notify();
    });
    cx.run_until_parked();
    let (back, _) = actions(newest, cx);
    assert_eq!(back, scrolled, "the filter taken off moved the columns");
}

/// Every delay a hover has run out: the larger preview's, a tooltip's
/// half second, and gpui-component's grace before either goes.
fn settle(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(1));
        cx.run_until_parked();
    }
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}

/// What answers the pointer in a cell answers it where the cell is drawn
/// once the table has scrolled sideways: the larger preview opens on a
/// preview half scrolled away, after its delay and not before; a button's
/// tooltip comes up at the Actions column scrolled in; a file let go over
/// the scrolled rows lands; and the Actions menu opens at its button.
#[gpui::test]
fn a_cell_scrolled_into_view_answers_the_pointer_where_it_is_drawn(cx: &mut TestAppContext) {
    let scratch = Scratch::new("answers");
    let (queue, newest, cx) = table(&scratch, 3, cx);
    resize(NARROW, cx);
    let width = NARROW.width;

    let preview = painted(format!("queue-preview-{newest}"), cx);
    let (_, button) = actions(newest, cx);
    swipe(
        point(width / 2.0, button.center().y),
        -f32::from(preview.center().x),
        cx,
    );
    // The larger preview, on the half of a preview still in the window.
    let preview = painted(format!("queue-preview-{newest}"), cx);
    assert!(
        preview.left() < px(0.0) && preview.right() > px(0.0),
        "the preview is not half scrolled away: {preview:?}"
    );
    cx.simulate_mouse_move(
        point(preview.right() / 2.0, preview.center().y),
        None,
        Modifiers::default(),
    );
    assert_eq!(
        found("queue-preview-card".to_owned(), cx),
        None,
        "the larger preview opened before its delay"
    );
    settle(cx);
    let card = found("queue-preview-card".to_owned(), cx)
        .expect("the larger preview did not open over a preview scrolled sideways");
    assert!(
        card.top() >= preview.bottom() && card.top() <= preview.bottom() + px(24.0),
        "the larger preview did not open under its preview {preview:?}: {card:?}"
    );
    assert!(
        card.left() >= px(0.0),
        "the larger preview is off the window"
    );

    // A tooltip, on the Actions button scrolled in.
    swipe(point(width / 2.0, button.center().y), -4000.0, cx);
    let (_, button) = actions(newest, cx);
    cx.simulate_mouse_move(button.center(), None, Modifiers::default());
    settle(cx);
    let tip = cx
        .debug_bounds(crate::hover::TIP)
        .expect("no tooltip over the Actions button scrolled in");
    assert!(
        tip.left() <= button.right()
            && tip.right() >= button.left()
            && tip.top() >= button.top()
            && tip.top() <= button.bottom() + px(24.0),
        "the tooltip is not at the button {button:?}: {tip:?}"
    );

    // A file let go over the scrolled rows lands.
    let dropped = scratch.file("dropped.md", NOTE.as_bytes());
    let over = point(width - px(100.0), button.center().y);
    cx.simulate_event(FileDropEvent::Entered {
        position: over,
        paths: ExternalPaths(vec![dropped].into()),
    });
    cx.simulate_event(FileDropEvent::Submit { position: over });
    cx.run_until_parked();
    assert_eq!(
        cx.update(|_, cx| queue.read(cx).ids().len()),
        4,
        "a file let go over the scrolled rows did not land"
    );
    let newest = cx
        .update(|_, cx| queue.read(cx).ids())
        .last()
        .copied()
        .expect("rows");
    let (_, button) = actions(newest, cx);

    // The Actions menu, at its button: its top right at the button's
    // bottom right (`Anchor::TopRight`), so every line of it to the left
    // of the button's right edge and under it.
    press(button.center(), cx);
    settle(cx);
    let line = found(format!("queue-why-open-result-{newest}"), cx)
        .expect("the Actions menu did not open");
    assert!(
        line.right() <= button.right()
            && line.left() >= button.right() - px(320.0)
            && line.top() >= button.bottom(),
        "the Actions menu did not open at its button {button:?}: a line of it at {line:?}"
    );
}

/// A window of the size every launch opens at shows every column with
/// nothing to scroll: [`NAME_MIN`] is what the name has there. A column
/// added or widened without taking it from the name would put a bar under
/// every fresh window, and this says so first.
#[test]
fn a_window_of_the_size_every_launch_opens_at_scrolls_nothing() {
    assert!(
        Column::least_table() <= crate::MAIN_SIZE.width,
        "the columns at their least are {:?}, wider than the {:?} the main window opens at",
        Column::least_table(),
        crate::MAIN_SIZE.width
    );
}
