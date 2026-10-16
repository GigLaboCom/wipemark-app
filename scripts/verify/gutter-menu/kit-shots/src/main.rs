//! The editor gutter menu example, drawn by GPUI's headless renderer: the
//! gutter's menu on a line number with a selection kept, the usual menu on
//! the text, and the bookmark the gutter's menu added.
//!
//! See `../kit-shots.sh` for what it is for and how it is run.
//!
//! Usage: gutter-menu-shots <out-dir>

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::{
    AnyWindowHandle, AppContext as _, HeadlessAppContext, Modifiers, MouseButton, MouseDownEvent,
    MouseUpEvent, PlatformInput, Pixels, Point, px, size,
};

#[allow(dead_code, unreachable_pub)]
mod example {
    include!(concat!(env!("OUT_DIR"), "/example.rs"));

    pub(crate) mod shots {
        use super::*;

        pub(crate) fn new(window: &mut Window, cx: &mut Context<Example>) -> Example {
            Example::new(window, cx)
        }

        /// The middle of `row`'s line number, and a point in its text.
        pub(crate) fn points(example: &Example, row: usize, cx: &App) -> (Point<Pixels>, Point<Pixels>) {
            let state = example.editor.read(cx);
            let y = state.row_bounds(row).expect("laid out").center().y;
            let gutter = state.gutter_bounds().expect("a gutter");
            (point(gutter.center().x, y), point(gutter.right() + px(60.), y))
        }

        pub(crate) fn select(example: &Entity<Example>, needle: &str, window: &mut Window, cx: &mut App) {
            let start = TEXT.find(needle).expect("the needle");
            let editor = example.read(cx).editor.clone();
            editor.update(cx, |state, cx| {
                state.set_selected_range(start..start + needle.len(), cx);
                state.focus(window, cx);
            });
        }

    }
}

fn headless() -> HeadlessAppContext {
    let mut cx = HeadlessAppContext::with_platform(
        gpui_platform::current_platform(true).text_system(),
        Arc::new(gpui_kit::assets::Assets),
        gpui_platform::current_headless_renderer,
    );
    cx.update(gpui_kit::init);
    cx
}

fn settle(cx: &mut HeadlessAppContext, window: AnyWindowHandle) {
    for _ in 0..4 {
        cx.run_until_parked();
        cx.advance_clock(Duration::from_millis(400));
        cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
            .expect("drawn");
    }
}

fn right_click(cx: &mut HeadlessAppContext, window: AnyWindowHandle, position: Point<Pixels>) {
    click(cx, window, position, MouseButton::Right);
}

fn click(
    cx: &mut HeadlessAppContext,
    window: AnyWindowHandle,
    position: Point<Pixels>,
    button: MouseButton,
) {
    cx.update_window(window, |_, window, cx| {
        window.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                button,
                position,
                modifiers: Modifiers::default(),
                click_count: 1,
                first_mouse: false,
            }),
            cx,
        );
        window.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                button,
                position,
                modifiers: Modifiers::default(),
                click_count: 1,
            }),
            cx,
        );
    })
    .expect("clicked");
    settle(cx, window);
}

fn save(cx: &mut HeadlessAppContext, window: AnyWindowHandle, path: &Path) {
    let image = cx.capture_screenshot(window).expect("a picture");
    let writer = std::io::BufWriter::new(std::fs::File::create(path).expect("the file"));
    let mut encoder = png::Encoder::new(writer, image.width(), image.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut png| png.write_image_data(image.as_raw()))
        .expect("written");
    println!("{}", path.display());
}

fn main() {
    let out = PathBuf::from(std::env::args().nth(1).expect("usage: gutter-menu-shots <out-dir>"));
    std::fs::create_dir_all(&out).expect("the folder");

    let mut cx = headless();
    let slot = std::rc::Rc::new(std::cell::RefCell::new(None));
    let held = slot.clone();
    let window = cx
        .open_window(size(px(720.), px(520.)), move |window, cx| {
            let view = cx.new(|cx| example::shots::new(window, cx));
            *held.borrow_mut() = Some(view.clone());
            cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
        })
        .expect("a window");
    let window: AnyWindowHandle = window.into();
    let view = slot.take().expect("the view");
    settle(&mut cx, window);

    cx.update_window(window, |_, window, cx| {
        example::shots::select(&view, "quick brown fox", window, cx)
    })
    .expect("selected");
    settle(&mut cx, window);

    // Row 5 is line 6, `for word in text.split_whitespace() {`.
    let (gutter, text) = cx.update(|cx| example::shots::points(view.read(cx), 5, cx));
    right_click(&mut cx, window, gutter);
    save(&mut cx, window, &out.join("gutter-menu-on-a-line-number.png"));

    // The menu opens at the click; its first item, Bookmark Line 6, is the
    // first row of the menu.
    click(&mut cx, window, gutter + gpui_kit::point(px(70.), px(17.)), MouseButton::Left);
    save(&mut cx, window, &out.join("gutter-menu-bookmark-added.png"));

    right_click(&mut cx, window, text);
    save(&mut cx, window, &out.join("gutter-menu-on-the-text.png"));
}
