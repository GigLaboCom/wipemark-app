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

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{
    point, px, AnyWindowHandle, Bounds, HeadlessAppContext, InputEvent as _, Modifiers,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, ScrollDelta,
    ScrollEvent, Size,
};
use serde_json::{json, Map, Value};

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

/// The video's `CURVE.move`, `cubic-bezier(0.2, 0, 0, 1)`: where a move is
/// at `t` of its way, both from 0 to 1.
pub(crate) fn curve_move(t: f32) -> f32 {
    // x(s) and y(s) of the curve through (0, 0), (0.2, 0), (0, 1), (1, 1).
    let bezier = |a: f32, b: f32, s: f32| {
        let r = 1.0 - s;
        3.0 * r * r * s * a + 3.0 * r * s * s * b + s * s * s
    };
    let t = t.clamp(0.0, 1.0);
    let (mut low, mut high) = (0.0f32, 1.0f32);
    for _ in 0..40 {
        let middle = (low + high) / 2.0;
        if bezier(0.2, 0.0, middle) < t {
            low = middle;
        } else {
            high = middle;
        }
    }
    bezier(0.0, 1.0, (low + high) / 2.0)
}

/// Frames a second, of the video and of the test's clock.
pub(crate) const FPS: u32 = 30;

/// One frame of the test's clock.
pub(crate) const FRAME: Duration = Duration::from_micros(1_000_000 / FPS as u64);

/// One clip of a video: numbered frames of one window, `<out>/<name>/
/// frames/0000.png` on, frame 0 first at `FPS`, and beside them the
/// `clip.json` the video's composition reads — in the headless video kit's
/// format (`name`, `fps`, `frames`, `width`, `height`, `scale`, `css`,
/// `marks`, `regions`, `pointer`), with what it lacks added under names of
/// its own: each mark's `frame`, every region in frame pixels too
/// (`frame_regions`), how each named part moved frame by frame
/// (`region_track`), the pointer as `[frame, x, y, down]` (`track`).
///
/// The pointer is never drawn: GPUI draws no cursor, so the clip says
/// where it was and the composition draws it. Regions are the rectangles
/// GPUI laid the named parts out at (`crate::spot`), not measured off a
/// picture.
pub(crate) struct Clip {
    name: String,
    dir: PathBuf,
    window: AnyWindowHandle,
    css: Size<Pixels>,
    /// Frame pixels a window point.
    scale: f32,
    /// The next frame's number.
    frame: usize,
    marks: Vec<(String, usize)>,
    /// Where the pointer is, and whether its button is down.
    pointer: Option<(Point<Pixels>, bool)>,
    track: Vec<(usize, Point<Pixels>, bool)>,
    events: Vec<(usize, &'static str, Point<Pixels>)>,
    /// Which named parts to follow, by the start of their names.
    follow: Vec<String>,
    /// Each followed part: the frame it took a rectangle at, and the
    /// rectangle — a row only when it moved.
    moves: BTreeMap<String, Vec<(usize, Bounds<Pixels>)>>,
    /// The regions the clip names, as the part they are and the frame they
    /// are read at.
    named: Vec<(String, String, usize)>,
    extra: Map<String, Value>,
    started: Instant,
}

impl Clip {
    /// A clip of `window`, `css` points large, following every part whose
    /// name starts with one of `follow`. Frames a run before left in its
    /// folder are removed first.
    pub(crate) fn new(
        out: &Path,
        name: &str,
        window: AnyWindowHandle,
        css: Size<Pixels>,
        follow: &[&str],
    ) -> Self {
        let dir = out.join(name);
        let frames = dir.join("frames");
        if frames.exists() {
            std::fs::remove_dir_all(&frames).expect("the old frames removed");
        }
        std::fs::create_dir_all(&frames).expect("the frames' folder");
        Self {
            name: name.to_owned(),
            dir,
            window,
            css,
            scale: 2.0,
            frame: 0,
            marks: Vec::new(),
            pointer: None,
            track: Vec::new(),
            events: Vec::new(),
            follow: follow.iter().map(|prefix| (*prefix).to_owned()).collect(),
            moves: BTreeMap::new(),
            named: Vec::new(),
            extra: Map::new(),
            started: Instant::now(),
        }
    }

    /// The number the next frame will have.
    pub(crate) fn next(&self) -> usize {
        self.frame
    }

    /// One frame of time: the work that is ready runs, the clock moves one
    /// frame, and the window is drawn.
    pub(crate) fn frame(&mut self, cx: &mut HeadlessAppContext) {
        cx.run_until_parked();
        cx.advance_clock(FRAME);
        cx.run_until_parked();
        // What the work left pending — an event heard, a view to repaint —
        // is flushed by an update, so the frame shows what the work did
        // rather than the frame after it.
        cx.update_window(self.window, |_, _, _| ())
            .expect("flushed");
        cx.run_until_parked();
        self.draw(cx);
    }

    /// One frame drawn as the window stands, before any work runs or the
    /// clock moves: the middle of a change that would otherwise finish
    /// inside one frame.
    pub(crate) fn hold(&mut self, cx: &mut HeadlessAppContext) {
        self.draw(cx);
    }

    fn draw(&mut self, cx: &mut HeadlessAppContext) {
        let path = self
            .dir
            .join("frames")
            .join(format!("{:04}.png", self.frame));
        shoot(cx, self.window, &path);
        // The rectangles of this frame's prepaint (`crate::spot`).
        for (name, bounds) in crate::spot::spots() {
            if !self.follow.iter().any(|prefix| name.starts_with(prefix)) {
                continue;
            }
            let moves = self.moves.entry(name).or_default();
            if moves.last().is_none_or(|(_, last)| *last != bounds) {
                moves.push((self.frame, bounds));
            }
        }
        if let Some((at, down)) = self.pointer {
            self.track.push((self.frame, at, down));
        }
        self.frame += 1;
    }

    /// A part's rectangle as the last frame drawn has it.
    pub(crate) fn spot(&self, name: &str) -> Bounds<Pixels> {
        self.spot_at(name, self.frame.saturating_sub(1))
    }

    /// A part's rectangle in frame `frame`.
    pub(crate) fn spot_at(&self, name: &str, frame: usize) -> Bounds<Pixels> {
        self.moves
            .get(name)
            .and_then(|moves| moves.iter().rev().find(|(at, _)| *at <= frame))
            .map(|(_, bounds)| *bounds)
            .unwrap_or_else(|| panic!("{name} was not drawn by frame {frame}"))
    }

    /// The next frame is where `name` happens.
    pub(crate) fn mark(&mut self, name: &str) {
        self.mark_at(name, self.frame);
    }

    /// Frame `frame` is where `name` happens.
    pub(crate) fn mark_at(&mut self, name: &str, frame: usize) {
        self.marks.push((name.to_owned(), frame));
    }

    /// Names a region: part `spot` as frame `frame` has it.
    pub(crate) fn region(&mut self, name: &str, spot: &str, frame: usize) {
        self.spot_at(spot, frame);
        self.named.push((name.to_owned(), spot.to_owned(), frame));
    }

    /// Adds a field of the clip's own to `clip.json`.
    pub(crate) fn note(&mut self, key: &str, value: Value) {
        self.extra.insert(key.to_owned(), value);
    }

    fn send(&self, cx: &mut HeadlessAppContext, input: gpui::PlatformInput) {
        cx.update_window(self.window, |_, window, cx| {
            window.dispatch_event(input, cx);
        })
        .expect("dispatched");
    }

    /// The pointer moves to `at`, its button as it was.
    pub(crate) fn point(&mut self, cx: &mut HeadlessAppContext, at: Point<Pixels>) {
        let down = self.pointer.is_some_and(|(_, down)| down);
        self.send(
            cx,
            MouseMoveEvent {
                position: at,
                pressed_button: down.then_some(MouseButton::Left),
                modifiers: Modifiers::default(),
            }
            .to_platform_input(),
        );
        self.pointer = Some((at, down));
    }

    /// The pointer's button goes down where the pointer is.
    pub(crate) fn press(&mut self, cx: &mut HeadlessAppContext) {
        let (at, _) = self.pointer.expect("a pointer to press with");
        self.send(
            cx,
            MouseDownEvent {
                position: at,
                modifiers: Modifiers::default(),
                button: MouseButton::Left,
                click_count: 1,
                first_mouse: false,
            }
            .to_platform_input(),
        );
        self.pointer = Some((at, true));
        self.events.push((self.frame, "down", at));
    }

    /// The pointer's button comes up where the pointer is: a click.
    pub(crate) fn release(&mut self, cx: &mut HeadlessAppContext) {
        let (at, _) = self.pointer.expect("a pointer to release");
        self.send(
            cx,
            MouseUpEvent {
                position: at,
                modifiers: Modifiers::default(),
                button: MouseButton::Left,
                click_count: 1,
            }
            .to_platform_input(),
        );
        self.pointer = Some((at, false));
        self.events.push((self.frame, "up", at));
    }

    /// The wheel turns over `at` by `down` points (positive scrolls the
    /// content up, as a person reading on does), with no pointer drawn.
    pub(crate) fn wheel(&mut self, cx: &mut HeadlessAppContext, at: Point<Pixels>, down: Pixels) {
        self.send(
            cx,
            ScrollEvent {
                position: at,
                delta: ScrollDelta::Pixels(point(px(0.0), -down)),
                ..Default::default()
            }
            .to_platform_input(),
        );
        self.events.push((self.frame, "wheel", at));
    }

    /// Writes `clip.json` and says what was drawn.
    pub(crate) fn finish(self, query: &str) -> usize {
        let scale = self.scale;
        let t = |frame: usize| (frame as f64 / f64::from(FPS) * 1000.0).round() / 1000.0;
        let n = |value: Pixels, by: f32| f64::from((f32::from(value) * by * 100.0).round() / 100.0);
        let rect = |bounds: Bounds<Pixels>, by: f32| {
            json!({
                "x": n(bounds.origin.x, by),
                "y": n(bounds.origin.y, by),
                "w": n(bounds.size.width, by),
                "h": n(bounds.size.height, by),
            })
        };
        let inside = |bounds: Bounds<Pixels>| {
            bounds.origin.x >= px(0.0)
                && bounds.origin.y >= px(0.0)
                && bounds.origin.x + bounds.size.width <= self.css.width
                && bounds.origin.y + bounds.size.height <= self.css.height
        };
        let mut regions = Map::new();
        let mut frame_regions = Map::new();
        let mut outside = Vec::new();
        for (name, spot, frame) in &self.named {
            let bounds = self.spot_at(spot, *frame);
            frame_regions.insert(name.clone(), rect(bounds, scale));
            // The kit refuses a region outside the page; such a one is in
            // frame_regions alone.
            if inside(bounds) {
                regions.insert(name.clone(), rect(bounds, 1.0));
            } else {
                outside.push(name.clone());
            }
        }
        let region_track: Map<String, Value> = self
            .moves
            .iter()
            .map(|(name, moves)| {
                let moves: Vec<Value> = moves
                    .iter()
                    .map(|(frame, bounds)| {
                        json!([
                            frame,
                            n(bounds.origin.x, scale),
                            n(bounds.origin.y, scale),
                            n(bounds.size.width, scale),
                            n(bounds.size.height, scale)
                        ])
                    })
                    .collect();
                (name.clone(), Value::Array(moves))
            })
            .collect();
        let at = |p: Point<Pixels>| (n(p.x, scale), n(p.y, scale));
        let samples: Vec<Value> = self
            .track
            .iter()
            .map(|(frame, p, _)| {
                let (x, y) = at(*p);
                json!([t(*frame), x, y])
            })
            .collect();
        let track: Vec<Value> = self
            .track
            .iter()
            .map(|(frame, p, down)| {
                let (x, y) = at(*p);
                json!([frame, x, y, down])
            })
            .collect();
        let events: Vec<Value> = self
            .events
            .iter()
            .map(|(frame, kind, p)| {
                let (x, y) = at(*p);
                json!({ "t": t(*frame), "type": kind, "frame": frame, "x": x, "y": y })
            })
            .collect();
        let marks: Vec<Value> = self
            .marks
            .iter()
            .map(|(name, frame)| json!({ "t": t(*frame), "name": name, "frame": frame }))
            .collect();
        let width = f32::from(self.css.width) * scale;
        let height = f32::from(self.css.height) * scale;
        let mut clip = json!({
            "name": self.name.clone(),
            "app": "wipemark",
            "query": query,
            "fps": FPS,
            "frames": self.frame,
            "width": width,
            "height": height,
            "scale": scale,
            "css": { "width": f32::from(self.css.width), "height": f32::from(self.css.height) },
            "marks": marks,
            "regions": regions,
            "pointer": {
                "space": "frame",
                "visible": !samples.is_empty(),
                "samples": samples,
                "events": events,
            },
            "duration": t(self.frame),
            "frames_dir": "frames",
            "first": 0,
            "pattern": "%04d.png",
            "frame_regions": frame_regions,
            "regions_outside_css": outside,
            "region_track": region_track,
            "track": track,
        });
        if let Value::Object(fields) = &mut clip {
            fields.extend(self.extra);
        }
        let text = serde_json::to_string_pretty(&clip).expect("JSON");
        std::fs::write(self.dir.join("clip.json"), text + "\n").expect("clip.json");
        eprintln!(
            "{}: {} frames in {:.1} s",
            self.name,
            self.frame,
            self.started.elapsed().as_secs_f64()
        );
        self.frame
    }
}
