//! The windows drawn as pictures with no display and no window: a
//! headless context on GPUI's own renderer, and a writer of one frame to a
//! PNG. Test code only — what the ignored footage and screenshot tests draw
//! with (`compare::tests::compare_screenshots`, `queue::footage_tests`).
//!
//! On macOS `MacPlatform::new` refuses any thread but the process's main
//! one, and libtest runs every test on a thread of its own, so the text
//! system is built directly: gpui-pre-apple's `AppleTextSystem` (CoreText),
//! exported since gpui-pre 0.3.12. Elsewhere it is the platform's own, as
//! before. The renderer is `current_headless_renderer` everywhere: Metal
//! into an offscreen texture on macOS, wgpu on Linux.

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use gpui::{AnyWindowHandle, HeadlessAppContext};

/// The platform's text system, built on this thread.
pub(crate) fn text_system() -> Arc<dyn gpui::PlatformTextSystem> {
    #[cfg(target_os = "macos")]
    {
        Arc::new(gpui_apple::AppleTextSystem::new())
    }
    #[cfg(not(target_os = "macos"))]
    {
        gpui_platform::current_platform(true).text_system()
    }
}

/// A headless application with the product's assets and a GPU renderer.
pub(crate) fn context() -> HeadlessAppContext {
    HeadlessAppContext::with_platform(
        text_system(),
        Arc::new(crate::assets::WipemarkAssets),
        gpui_platform::current_headless_renderer,
    )
}

/// What one frame cost, in milliseconds: the draw, the render and its read
/// back, the PNG.
#[derive(Default, Clone, Copy)]
pub(crate) struct Cost {
    pub draw: f64,
    pub render: f64,
    pub encode: f64,
}

/// Draws `window` once and writes it to `path`; says what each part cost.
pub(crate) fn shoot(cx: &mut HeadlessAppContext, window: AnyWindowHandle, path: &Path) -> Cost {
    let started = Instant::now();
    cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
        .expect("drawn");
    let drawn = Instant::now();
    let image = cx.capture_screenshot(window).expect("a picture");
    let rendered = Instant::now();
    let writer = std::io::BufWriter::new(std::fs::File::create(path).expect("file"));
    let mut encoder = png::Encoder::new(writer, image.width(), image.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    encoder
        .write_header()
        .and_then(|mut png| png.write_image_data(image.as_raw()))
        .expect("written");
    let encoded = Instant::now();
    let ms = |a: Instant, b: Instant| (b - a).as_secs_f64() * 1000.0;
    Cost {
        draw: ms(started, drawn),
        render: ms(drawn, rendered),
        encode: ms(rendered, encoded),
    }
}
