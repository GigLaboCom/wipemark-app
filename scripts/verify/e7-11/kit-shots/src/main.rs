//! gpui-kit's editor spacers example and its Editor Diff story, drawn into PNGs
//! by GPUI's headless renderer with no display (E7-11). The two views are the
//! submodule's own sources, included by `build.rs`; this file only opens them,
//! turns the wrap, scrolls, and saves what a window would show. See
//! `../kit-shots.sh` for what it is for and how it is run.
//!
//! Usage: e7-11-kit-shots <out-dir>

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::{
    AnyWindowHandle, App, AppContext as _, Entity, HeadlessAppContext, Render, Window, px, size,
};

/// The story crate's `Story`, as far as the included story implements it.
#[allow(dead_code)]
trait Story {
    fn title() -> &'static str;
    fn description() -> &'static str;
    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render>;
}

/// The story crate's toolbar, taken whole from its lib.rs.
#[allow(dead_code)]
mod toolbar {
    use std::rc::Rc;

    use gpui_kit::component::button::Button;
    use gpui_kit::component::menu::PopupMenu;
    use gpui_kit::component::popover::Popover;
    use gpui_kit::component::{Sizable as _, h_flex};
    use gpui_kit::prelude::FluentBuilder as _;
    use gpui_kit::*;

    include!(concat!(env!("OUT_DIR"), "/toolbar.rs"));
}
use toolbar::story_toolbar_group;

#[allow(dead_code, unreachable_pub)]
mod example {
    include!(concat!(env!("OUT_DIR"), "/example.rs"));

    pub(crate) mod shots {
        use super::*;

        pub(crate) fn new(window: &mut Window, cx: &mut Context<Example>) -> Example {
            Example::new(window, cx)
        }

        pub(crate) fn set_wrap(
            example: &mut Example,
            wrap: bool,
            window: &mut Window,
            cx: &mut Context<Example>,
        ) {
            example.set_wrap(wrap, window, cx);
        }
    }
}

#[allow(dead_code, unreachable_pub)]
mod story {
    include!(concat!(env!("OUT_DIR"), "/story.rs"));

    pub(crate) mod shots {
        use super::*;

        pub(crate) fn new(
            window: &mut Window,
            cx: &mut Context<EditorDiffStory>,
        ) -> EditorDiffStory {
            EditorDiffStory::new(window, cx)
        }

        pub(crate) fn set_wrap(
            story: &mut EditorDiffStory,
            wrap: bool,
            window: &mut Window,
            cx: &mut Context<EditorDiffStory>,
        ) {
            story.set_wrap(wrap, window, cx);
        }

        /// Both sides without their padding: soft wrap alone, as an editor
        /// without spacer rows shows two wrapped sources.
        pub(crate) fn unpad(story: &EditorDiffStory, cx: &mut Context<EditorDiffStory>) {
            for editor in [&story.original, &story.modified] {
                editor.update(cx, |state, cx| state.set_spacer_provider(None, cx));
            }
            cx.notify();
        }

        /// Puts the original's first changed line `above` rows under the top;
        /// the story's own scrolling together moves the other side.
        pub(crate) fn scroll_to_change(
            story: &EditorDiffStory,
            above: usize,
            cx: &mut Context<EditorDiffStory>,
        ) {
            let line = ORIGINAL
                .lines()
                .position(|line| line.contains("Everything is moving on schedule."))
                .expect("the changed line");
            story.original.update(cx, |state, cx| {
                let row = state
                    .display_map()
                    .buffer_line_to_display_row_range(line)
                    .expect("the changed line")
                    .start;
                let height = state.line_height().expect("laid out");
                let mut offset = state.scroll_offset();
                offset.y = -(height * row.saturating_sub(above) as f32);
                state.set_scroll_offset(offset, cx);
            });
            cx.notify();
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

/// A few frames with the clock moved on, so that padding asked in one frame is
/// drawn by the next and anything animated has settled.
fn settle(cx: &mut HeadlessAppContext, window: AnyWindowHandle) {
    for _ in 0..4 {
        cx.run_until_parked();
        cx.advance_clock(Duration::from_millis(400));
        cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
            .expect("drawn");
    }
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

fn example(out: &Path) {
    // The example's own window is 900 × 560; 640 makes every pair wrap again.
    for (name, width, wrap) in [
        ("example-wrapped", 900., true),
        ("example-wrapped-narrow", 640., true),
        ("example-unwrapped", 900., false),
    ] {
        let mut cx = headless();
        let slot = std::rc::Rc::new(std::cell::RefCell::new(None));
        let held = slot.clone();
        let window = cx
            .open_window(size(px(width), px(560.)), move |window, cx| {
                let view = cx.new(|cx| example::shots::new(window, cx));
                *held.borrow_mut() = Some(view.clone());
                cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
            })
            .expect("a window");
        let window: AnyWindowHandle = window.into();
        let view = slot.take().expect("the view");
        settle(&mut cx, window);
        if !wrap {
            cx.update_window(window, |_, window, cx| {
                view.update(cx, |view, cx| {
                    example::shots::set_wrap(view, false, window, cx)
                });
            })
            .expect("unwrapped");
            settle(&mut cx, window);
        }
        save(&mut cx, window, &out.join(format!("{name}.png")));
    }
}

fn story(out: &Path) {
    for (name, wrap, padded) in [
        ("story-unwrapped", false, true),
        ("story-wrapped-unpadded", true, false),
        ("story-wrapped", true, true),
    ] {
        let mut cx = headless();
        let slot = std::rc::Rc::new(std::cell::RefCell::new(None));
        let held = slot.clone();
        let window = cx
            .open_window(size(px(1000.), px(560.)), move |window, cx| {
                let view = cx.new(|cx| story::shots::new(window, cx));
                *held.borrow_mut() = Some(view.clone());
                cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
            })
            .expect("a window");
        let window: AnyWindowHandle = window.into();
        let view = slot.take().expect("the view");
        settle(&mut cx, window);
        if wrap {
            cx.update_window(window, |_, window, cx| {
                view.update(cx, |view, cx| {
                    story::shots::set_wrap(view, true, window, cx)
                });
            })
            .expect("wrapped");
            settle(&mut cx, window);
        }
        if !padded {
            cx.update_window(window, |_, _, cx| {
                view.update(cx, |view, cx| story::shots::unpad(view, cx));
            })
            .expect("unpadded");
            settle(&mut cx, window);
        }
        cx.update_window(window, |_, _, cx| {
            view.update(cx, |view, cx| story::shots::scroll_to_change(view, 6, cx));
        })
        .expect("scrolled");
        settle(&mut cx, window);
        save(&mut cx, window, &out.join(format!("{name}.png")));
    }
}

fn main() {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("usage: e7-11-kit-shots <out-dir>"),
    );
    std::fs::create_dir_all(&out).expect("the folder");
    example(&out);
    story(&out);
}
