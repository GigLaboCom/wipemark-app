//! The quick panel drawn headless, frame by frame, as footage for the
//! picture video (`dev/video/picture/storyboard.md`, 3.0–12.0 s): the
//! invitation, two pictures dragged in from outside the window and let go
//! — GPUI's own file drag, `FileDropEvent::Entered`, `Pending` and
//! `Submit`, the road off macOS, so the shared overlay lights and says
//! *Let go to add it* — the looks, Clean pressed, *Cleaning…* and the
//! result lines. Every frame is GPUI's own renderer on the test platform
//! (`crate::footage`): no display, no window, no screen capture.
//!
//!   WIPEMARK_FOOTAGE=<dir> [WIPEMARK_FOOTAGE_DARK=1] \
//!   [WIPEMARK_FOOTAGE_FOLDER=<folder>] cargo test -p wipemark-app \
//!     --bin wipemark -- --ignored panel::footage_tests::picture_video_panel \
//!     --exact --nocapture --test-threads=1
//!
//! It writes `<dir>/panel-dark/` or `<dir>/panel-light/`: frames and a
//! `clip.json` (`crate::footage::Clip`), with the pointer's track — outside
//! the window too, where the drag starts — and where the composition draws
//! the two dragged pictures, which GPUI does not draw (`ExternalPaths`
//! renders nothing: the platform draws a drag's icons). The folder is
//! `/Users/Shared/Wipemark Demo/picture/panel` unless given: it names
//! nobody, and holds the two pictures and the results the panel's clean
//! writes beside them. It sets the process's language (English), so it
//! runs alone. `media/footage/picture.sh` runs it.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui::{
    point, px, size, AnyWindowHandle, AppContext as _, Entity, HeadlessAppContext, Pixels, Point,
};
use gpui_component::Root;
use serde_json::{json, Value};

use super::{clean_offered, cleaning, PanelView};
use crate::drop::Catcher;
use crate::footage::{curve_move, Clip};
use crate::queue::Queue;
use crate::retention::Homes;
use crate::settings::Preferences;

/// The panel's size in the video: the panel's own shape, larger — it has
/// no size limits and a person can pull it to this. Frames are 1120×692.
fn panel_size() -> gpui::Size<Pixels> {
    size(px(560.0), px(346.0))
}

/// The two pictures, by the names the video gives them, and the fixtures
/// they are copies of: the C2PA project's test photo and the owner's
/// Gemini sticker.
const PICTURES: [(&str, &str); 2] = [
    ("coast.jpg", "image/c2pa-jumbf.jpg"),
    ("sticker.png", "image/gemini/torch-1025.png"),
];

/// Where the drag starts, in window points: left of the window, where the
/// video shrinks the sticker into the thumbnail the pointer carries.
const START: (f32, f32) = (-150.0, 150.0);

/// Where the footage's pictures are: a folder that names nobody.
fn demo_folder() -> PathBuf {
    std::env::var_os("WIPEMARK_FOOTAGE_FOLDER")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/Users/Shared/Wipemark Demo/picture/panel"))
}

/// The two pictures copied into `folder`, after what a run before left of
/// them — the pictures and their results — is removed. A file no footage
/// test writes is never removed: the test refuses instead.
fn demo_files(folder: &Path) -> Vec<PathBuf> {
    std::fs::create_dir_all(folder).expect("the demo folder");
    let ours = |file: &str| {
        PICTURES.iter().any(|(name, _)| {
            let (stem, _) = name.rsplit_once('.').expect("an extension");
            file == *name || file.starts_with(&format!("{stem}.cleaned"))
        })
    };
    for entry in std::fs::read_dir(folder).expect("the demo folder, read") {
        let name = entry
            .expect("an entry")
            .file_name()
            .into_string()
            .expect("a UTF-8 name");
        if ours(&name) {
            std::fs::remove_file(folder.join(&name)).expect("a file of a run before, removed");
        } else {
            assert!(
                name.starts_with('.'),
                "{} holds {name}, which no footage test writes: move it away first",
                folder.display()
            );
        }
    }
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    PICTURES
        .iter()
        .map(|(name, fixture)| {
            let path = folder.join(name);
            std::fs::copy(fixtures.join(fixture), &path).expect("a picture copied");
            path
        })
        .collect()
}

/// Where the composition draws a dragged picture: its rectangle in frame
/// pixels (it may lie outside the window) and its opacity.
fn thumb(at: Point<Pixels>, offset: (f32, f32), wh: (f32, f32), scale: f32, opacity: f32) -> Value {
    let (cx, cy) = (
        f32::from(at.x) * 2.0 + offset.0,
        f32::from(at.y) * 2.0 + offset.1,
    );
    let (w, h) = (wh.0 * scale, wh.1 * scale);
    let r = |v: f32| f64::from((v * 100.0).round() / 100.0);
    json!([r(cx - w / 2.0), r(cy - h / 2.0), r(w), r(h), r(opacity)])
}

/// The picture video's panel scene, 9.0 s from the panel's arrival
/// (storyboard 3.0 s): the invitation while the pointer holds the two
/// pictures left of the window (0–1.1 s), the drag in on `CURVE.move`
/// (1.1–2.3 s) — the overlay lights on entering — a rest, the drop at
/// 2.5 s, *Looking…* as long as the frames before the looks may land, the
/// looks held to 5.7 s; then the pointer to Clean (5.7–6.3 s), a rest on
/// it, the press at 6.5 s, *Cleaning…* held for the frames before the
/// clean may run, the result lines, and the pointer off the button before
/// its tooltip could come up.
#[test]
#[ignore]
#[allow(clippy::too_many_lines, reason = "one scene, step by step")]
fn picture_video_panel() {
    let out = PathBuf::from(std::env::var("WIPEMARK_FOOTAGE").expect("WIPEMARK_FOOTAGE"));
    let dark = std::env::var_os("WIPEMARK_FOOTAGE_DARK").is_some();
    let theme_name = if dark { "dark" } else { "light" };
    wipemark_i18n::init(
        &wipemark_i18n::LanguagePreference::Explicit("en-US".parse().expect("a tag")),
        wipemark_i18n::Rendering::Ui,
    );
    let folder = demo_folder();
    let files = demo_files(&folder);
    // The Retention page's two folders, used by nothing at its defaults (a
    // result goes beside its file) and shown nowhere.
    let homes_dir =
        std::env::temp_dir().join(format!("wipemark-footage-panel-{}", std::process::id()));
    std::fs::create_dir_all(&homes_dir).expect("the homes");
    let homes = Homes {
        results: homes_dir.join("results"),
        kept: homes_dir.join("kept"),
    };

    let mut cx = crate::footage::context();
    cx.update(gpui_component::init);
    cx.update(crate::hover::init);
    cx.update(super::init);
    cx.update(|cx| cx.set_reduce_motion(true));

    type Built = (Entity<PanelView>, Entity<Queue>);
    let slot: Rc<std::cell::RefCell<Option<Built>>> = Rc::default();
    let held = slot.clone();
    let window = cx
        .open_window(panel_size(), move |window, cx| {
            let theme = if dark {
                crate::theme::ThemePreference::Dark
            } else {
                crate::theme::ThemePreference::Light
            };
            theme.apply(Some(window), cx);
            let preferences = cx.new(|cx| Preferences::for_tests(homes, cx));
            let table = cx.new(|cx| Queue::for_tests(preferences.clone(), None, window, cx));
            // Detached: a test window has no native window for the
            // pasteboard's destination. GPUI's own file drag still reaches
            // the overlay and the zone, as it does off macOS.
            let catcher = cx.new(|_| Catcher::detached());
            let view = cx.new(|cx| {
                PanelView::with_catcher(preferences, catcher, Some(table.clone()), window, cx)
            });
            *held.borrow_mut() = Some((view.clone(), table));
            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("a window");
    let window: AnyWindowHandle = window.into();
    let (view, _table) = slot.take().expect("the window builder ran");
    cx.update_window(window, |_, window, _| window.activate_window())
        .expect("activated");

    let mut clip = Clip::new(
        &out,
        &format!("panel-{theme_name}"),
        window,
        panel_size(),
        &["panel-", "drop-overlay"],
    );
    let cx = &mut cx;
    // What the panel shows: whether the overlay is lit, whether a drop is
    // held, the looks back, and Clean's standing.
    let shows = |cx: &mut HeadlessAppContext| {
        cx.update(|cx| {
            let panel = view.read(cx);
            let held = panel.held();
            let states = panel.states(cx);
            (
                held.is_some(),
                held.map_or(0, |held| held.found.iter().filter(|f| f.is_some()).count()),
                cleaning(&states),
                clean_offered(&states),
            )
        })
    };

    // Thumbnails: the sticker under the pointer, the photo (upright, 9:16)
    // behind it two frames later; in frame pixels.
    let sticker_wh = (156.0, 156.0);
    let coast_wh = (88.0, 156.0);
    let sticker_off = (16.0, 34.0);
    let coast_off = (58.0, 10.0);
    let mut sticker_track: Vec<Value> = Vec::new();
    let mut coast_track: Vec<Value> = Vec::new();
    let mut path: Vec<Point<Pixels>> = Vec::new();
    let mut carry = |frame: usize, at: Point<Pixels>, path: &mut Vec<Point<Pixels>>| {
        path.push(at);
        let behind = path[path.len().saturating_sub(3)];
        sticker_track.push(json!([frame, thumb(at, sticker_off, sticker_wh, 1.0, 1.0)]));
        coast_track.push(json!([frame, thumb(behind, coast_off, coast_wh, 1.0, 1.0)]));
    };

    // 0.0–1.1 s: the invitation; the pointer holds the two pictures left
    // of the window.
    let start = point(px(START.0), px(START.1));
    clip.mark("invite");
    for _ in 0..33 {
        let frame = clip.next();
        clip.drag(cx, start, &files);
        carry(frame, start, &mut path);
        clip.frame(cx);
    }
    let landing = clip.spot("panel-landing");
    let drop_at = landing.center();
    let invite_frame = clip.next() - 1;

    // 1.1–2.3 s: into the window, onto the middle of the drop area.
    clip.mark("move");
    let mut entered = None;
    for step in 1..=36 {
        let e = curve_move(step as f32 / 36.0);
        let at = point(
            start.x + (drop_at.x - start.x) * e,
            start.y + (drop_at.y - start.y) * e,
        );
        let frame = clip.next();
        clip.drag(cx, at, &files);
        if entered.is_none() && at.x >= px(0.0) {
            entered = Some(frame);
            clip.mark_at("over", frame);
        }
        carry(frame, at, &mut path);
        clip.frame(cx);
    }
    let over_frame = clip.next() - 1;
    // 2.3–2.5 s: a rest before the release.
    for _ in 0..6 {
        let frame = clip.next();
        clip.drag(cx, drop_at, &files);
        carry(frame, drop_at, &mut path);
        clip.frame(cx);
    }

    // 2.5 s: let go. The pictures sink into the drop point (0.2 s).
    clip.mark("dropped");
    let dropped = clip.next();
    clip.let_go(cx);
    for k in 1..=6 {
        let t = k as f32 / 6.0;
        let scale = 1.0 - 0.9 * t * t;
        let opacity = 1.0 - t * t;
        let frame = dropped + k - 1;
        let lag = |off: (f32, f32)| (off.0 * (1.0 - t), off.1 * (1.0 - t));
        sticker_track.push(json!([
            frame,
            thumb(drop_at, lag(sticker_off), sticker_wh, scale, opacity)
        ]));
        coast_track.push(json!([
            frame,
            thumb(drop_at, lag(coast_off), coast_wh, scale, opacity)
        ]));
    }
    // The drop is read off the disk and the looks start: one task at a
    // time, drawn before the clock moves, until the panel holds the drop
    // with no look back — *Looking…*.
    let mut looking = None;
    for _ in 0..200 {
        let (held, found, ..) = shows(cx);
        if held {
            if found == 0 {
                looking = Some(clip.next());
            }
            break;
        }
        if !cx.background_executor().tick() {
            break;
        }
    }
    clip.hold(cx);
    if let Some(at) = looking {
        clip.mark_at("looking", at);
        for _ in 0..8 {
            clip.hold(cx);
        }
    } else {
        eprintln!("Looking… was not seen: the looks landed with the drop");
    }
    clip.mark("looked");
    let looked = clip.next();
    clip.frame(cx);
    let (held, found, _, offered) = shows(cx);
    assert!(held, "the drop is not held: the drag did not land");
    assert_eq!(found, 2, "the looks did not land");
    assert!(offered, "Clean is not offered");
    // Held to 5.7 s.
    clip.point(cx, drop_at);
    while clip.next() < 171 {
        clip.frame(cx);
    }

    // 5.7–6.3 s: to Clean; 6.3–6.5 s resting on it (its hover).
    let to = clip.spot("panel-clean").center();
    clip.mark("move-to-clean");
    for step in 1..=18 {
        let e = curve_move(step as f32 / 18.0);
        clip.point(
            cx,
            point(
                drop_at.x + (to.x - drop_at.x) * e,
                drop_at.y + (to.y - drop_at.y) * e,
            ),
        );
        clip.frame(cx);
    }
    clip.mark_at("over-clean", clip.next() - 1);
    for _ in 0..6 {
        clip.frame(cx);
    }
    // 6.5 s: the press, held four frames; the click is the release.
    clip.mark("press");
    clip.press(cx);
    for _ in 0..4 {
        clip.frame(cx);
    }
    clip.release(cx);
    let (_, _, busy, _) = shows(cx);
    assert!(busy, "the press did not start the clean");
    clip.mark("cleaning");
    let cleaning_at = clip.next();
    for _ in 0..9 {
        clip.hold(cx);
    }
    clip.mark("cleaned");
    let cleaned = clip.next();
    clip.frame(cx);
    let (_, _, busy, offered) = shows(cx);
    assert!(!busy && !offered, "the clean did not finish in one frame");
    // The pointer leaves the button before its tooltip's half second.
    let away = point(to.x - px(150.0), to.y - px(60.0));
    let from = to;
    clip.mark("leave");
    for step in 1..=9 {
        let e = curve_move(step as f32 / 9.0);
        clip.point(
            cx,
            point(
                from.x + (away.x - from.x) * e,
                from.y + (away.y - from.y) * e,
            ),
        );
        clip.frame(cx);
    }
    // Held to 9.0 s (storyboard 12.0 s, past the push into the first row).
    while clip.next() < 270 {
        clip.frame(cx);
    }

    let results: Vec<Value> = PICTURES
        .iter()
        .map(|(name, _)| {
            let (stem, extension) = name.rsplit_once('.').expect("an extension");
            let result = format!("{stem}.cleaned.{extension}");
            let bytes = |file: &str| std::fs::metadata(folder.join(file)).map(|m| m.len()).ok();
            json!({
                "original": name,
                "original_bytes": bytes(name),
                "result": result,
                "result_bytes": bytes(&result),
            })
        })
        .collect();
    for result in &results {
        assert!(
            result["result_bytes"].is_u64(),
            "no result was written for {}",
            result["original"]
        );
    }

    for (name, spot, frame) in [
        ("landing", "panel-landing", invite_frame),
        ("overlay-card", "drop-overlay-card", over_frame),
        ("row-coast-looked", "panel-row-0", looked),
        ("row-sticker-looked", "panel-row-1", looked),
        ("clean", "panel-clean", looked),
        ("row-coast-cleaned", "panel-row-0", cleaned),
        ("row-sticker-cleaned", "panel-row-1", cleaned),
        ("clean-greyed", "panel-clean", cleaned),
    ] {
        clip.region(name, spot, frame);
    }
    clip.note("theme", json!(theme_name));
    clip.note("folder", json!(folder.display().to_string()));
    clip.note("files", Value::Array(results));
    clip.note(
        "overlay",
        json!({
            "lit_from": entered,
            "lit_until": dropped,
            "frame_rect": { "x": 0, "y": 0, "w": f32::from(panel_size().width) * 2.0,
                            "h": f32::from(panel_size().height) * 2.0 },
            "what": "drop::overlay over the whole panel: a dashed frame in the theme's primary colour and the card \"Let go to add it\", lit while GPUI's ExternalPaths drag is over it",
        }),
    );
    clip.note(
        "thumbnails",
        json!({
            "space": "frame pixels of this clip; x may be negative (left of the window)",
            "row": "[frame, [x, y, w, h, opacity]]",
            "drawn_by": "the composition: GPUI draws no drag image (ExternalPaths renders nothing)",
            "sticker": { "file": "sticker.png", "track": sticker_track },
            "coast": { "file": "coast.jpg", "upright": "EXIF orientation 6: shown 270x480", "track": coast_track },
        }),
    );
    clip.note(
        "states",
        json!([
            { "frame": 0, "panel": "invite" },
            { "frame": entered, "panel": "over: the overlay lit" },
            { "frame": dropped, "panel": "let go" },
            { "frame": looking, "panel": "Looking…" },
            { "frame": looked, "panel": "looked: AI metadata. / A visible mark." },
            { "frame": cleaning_at, "panel": "Cleaning…" },
            { "frame": cleaned, "panel": "cleaned: the result lines" },
        ]),
    );
    clip.finish(
        "picture video, the panel: coast.jpg and sticker.png dragged in, looked at and cleaned",
    );
}
