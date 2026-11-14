//! A place on screen that accepts what is dragged onto it.
//!
//! Epic **E6**. One entity and one decorator, so that any view in this
//! application can take a drop without knowing anything about
//! pasteboards, Objective-C or the difference between the three
//! platforms: build a [`Catcher`] when the window opens, wrap an
//! element in [`zone`], and read [`Catcher::caught`] when it says
//! something landed. The panel is the first caller; E7's workspace
//! windows and the main window are the next two, and neither of them
//! should have to learn any of this again.
//!
//! # What it accepts
//!
//! Text, images and files — everything a desktop can put on a
//! pasteboard. Recognising *which* is not this module's job and is
//! deliberately not in this crate at all: `wipemark-intake` is a leaf
//! library with no dependencies, so the CLI and the MCP server answer
//! the same question with the same code. What arrives here is
//! [`Handed`]; what comes out is [`Intake`], and the surface renders
//! it.
//!
//! # Two roads in, and why both are kept
//!
//! On macOS, `crate::pasteboard` installs a dragging destination and
//! answers **every** drop on the window, because GPUI's own window
//! refuses anything that is not a file (see that module for the line
//! that does it). Everywhere else, GPUI's [`ExternalPaths`] drag is all
//! there is, and it carries files — on Linux a dragged text too, which the
//! carried `gpui-pre-linux` hands over as a private drop file and
//! [`handed_of`] reads back as the text it is (F15, D653).
//!
//! So both are wired, and only one of them fires: a window whose
//! destination installed never reports an `ExternalPaths` drop, because
//! a registered view is found before the window behind it is asked. The
//! `ExternalPaths` half is therefore not dead code on macOS either — it
//! is what the window falls back to if the destination could not be
//! installed, which is the difference between "files still work" and
//! "nothing works".
//!
//! # Nothing blocks the window
//!
//! Recognising a dropped file reads the front of it, and a file dropped
//! on a window can live on a network volume that takes a second to
//! answer. Every [`Handed`] is therefore examined on the background
//! executor and the answer comes back to the entity, which is the same
//! bargain every long operation in this application makes.
//!
//! # Two readers, two questions
//!
//! The panel asks "what was the last thing dropped here" and paints it;
//! the main window's queue asks "what has arrived" and keeps every
//! answer. Both are served, and by different halves: [`Catcher::caught`]
//! is the last drop, overtaken by the next one the way a screen is; the
//! [`Landed`] event carries *its own* drop, and every drop fires one —
//! a second drop while the first is still being read off a slow disk
//! does not make the first one vanish from a list, it only decides
//! which of the two the panel is showing.
//!
//! What is kept is the [`Arrival`]: what was handed over *and* what it
//! turned out to be, because a queue that wants to show the first lines
//! of a note, or the picture in a screenshot, needs the thing and not
//! only its description.

use std::sync::Arc;

use gpui::prelude::*;
use gpui::{
    deferred, div, px, AnyElement, App, Div, Entity, EventEmitter, ExternalPaths, Task, Window,
};
use gpui_component::ActiveTheme;
use wipemark_i18n::{t, Message};
use wipemark_intake::{Handed, Intake};

use crate::icon::{Icon, IconName};

/// One thing that arrived: what was handed over, and what it turned out
/// to be.
///
/// The two halves are kept together on purpose. The [`Intake`] is the
/// description — kind, format, evidence — and is what every surface
/// reads first; the [`Handed`] is the thing itself, and is what a
/// preview, and from E1 the scrubber, reads next. A `Handed::Bytes` can
/// be a screenshot dragged out of a browser, megabytes with no file
/// behind them, and this is the only copy anybody holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arrival {
    pub handed: Handed,
    pub intake: Intake,
}

/// Something landed, and here is what it was.
///
/// One event per drop, carrying that drop and no other — see the module
/// docs for why that is not the same as [`Catcher::caught`]. A view that
/// only paints the last drop can observe the entity instead; this is for
/// the ones that have to *do* something with each, which the queue does
/// today and from E1 every one of them will.
pub struct Landed(pub Arc<[Arrival]>);

/// A window's drop target.
///
/// One per window. The pasteboard side is keyed by the window — a
/// second catcher on the same window would be told about the same drop
/// twice, and neither would know which half of the window it was over.
/// The position a drop happened at is not yet carried, because nothing
/// asks: the panel is one zone, and so is every window planned for E7.
pub struct Catcher {
    /// Is something being held over the window right now.
    over: bool,
    /// What the last drop turned out to be — `None` until there has
    /// been one, and `Some([])` for a drop that carried nothing this
    /// machine could read. The two are different answers and the
    /// surface says different things about them.
    caught: Option<Arc<[Arrival]>>,
    /// Which examination is the current one. A second drop while the
    /// first is still being read overtakes it — the same counter the
    /// MCP restart, the credential lookup and the panel's settling all
    /// use.
    reads: u64,
    /// The pasteboard's deliveries, for as long as this entity lives.
    /// Dropping it unregisters the window.
    _deliveries: Option<Task<()>>,
}

impl EventEmitter<Landed> for Catcher {}

impl Catcher {
    /// Start accepting drops on this window.
    ///
    /// Call it while the window is being built — the destination has to
    /// be installed before the first drag, and a window is not dragged
    /// onto in the frame it opens in.
    pub fn new(window: &Window, cx: &Context<Self>) -> Self {
        Self {
            over: false,
            caught: None,
            reads: 0,
            _deliveries: deliveries(window, cx),
        }
    }

    /// A catcher with no platform behind it: drops reach it only through
    /// [`Catcher::land`]. For a test window, which GPUI's test platform
    /// does not back with a real one — asking it for its native handle,
    /// as the macOS destination does, panics.
    #[cfg(test)]
    pub fn detached() -> Self {
        Self {
            over: false,
            caught: None,
            reads: 0,
            _deliveries: None,
        }
    }

    /// Whether something is being held over the window.
    pub fn over(&self) -> bool {
        self.over
    }

    /// What the last drop turned out to be, in the order it arrived.
    /// `None` if nothing has been dropped here yet.
    pub fn caught(&self) -> Option<&[Arrival]> {
        self.caught.as_deref()
    }

    /// Whether `drop` — a [`Landed`] event's — is the one [`Self::caught`]
    /// shows, for a surface that keeps something per drop and paints only
    /// the last: a read overtaken by a later drop still lands, after it.
    pub fn is_caught(&self, drop: &Arc<[Arrival]>) -> bool {
        self.caught
            .as_ref()
            .is_some_and(|caught| Arc::ptr_eq(caught, drop))
    }

    /// The highlight, on or off.
    fn hovering(&mut self, over: bool, cx: &mut Context<Self>) {
        if self.over == over {
            return;
        }
        self.over = over;
        cx.notify();
    }

    /// Something arrived: find out what it is, away from the thread
    /// that draws, and say so.
    ///
    /// A drop is the usual caller and not the only one. A file picker's
    /// answer is a list of paths that somebody chose rather than dragged,
    /// and it goes down this same road one step later — so "Import" and
    /// a drop cannot recognise a file two different ways, and a surface
    /// listening for [`Landed`] hears both.
    ///
    /// Nothing is no item (D301): an empty text, or one of ASCII white
    /// space alone, is left out here, whichever road it came by — so a
    /// drop of it lands as a drop that carried nothing, and the queue
    /// lists no row for it.
    pub fn land(&mut self, handed: Vec<Handed>, cx: &mut Context<Self>) {
        let handed: Vec<Handed> = handed
            .into_iter()
            .filter(|handed| !handed.is_nothing())
            .collect();
        self.hovering(false, cx);
        self.reads += 1;
        let mine = self.reads;

        cx.spawn(async move |catcher, cx| {
            let read: Arc<[Arrival]> = cx
                .background_executor()
                .spawn(async move {
                    handed
                        .into_iter()
                        .map(|handed| Arrival {
                            intake: wipemark_intake::of(&handed),
                            handed,
                        })
                        .collect()
                })
                .await;

            catcher
                .update(cx, |catcher, cx| {
                    // The last drop is what the panel shows, so a read
                    // overtaken by a later drop does not replace it —
                    // but it still *landed*, and the queue keeps every
                    // one. Two readers, two questions.
                    if catcher.reads == mine {
                        catcher.caught = Some(read.clone());
                    }
                    cx.emit(Landed(read));
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }
}

/// The pasteboard's deliveries, on the platforms that have them.
#[cfg(target_os = "macos")]
fn deliveries(window: &Window, cx: &Context<Catcher>) -> Option<Task<()>> {
    use crate::pasteboard::{self, Delivery};

    let window = pasteboard::identity(window)?;
    let deliveries = pasteboard::watch(window);

    Some(cx.spawn(async move |catcher, cx| {
        while let Ok(delivery) = deliveries.recv_async().await {
            let told = catcher.update(cx, |catcher, cx| match delivery {
                Delivery::Entered => catcher.hovering(true, cx),
                Delivery::Exited => catcher.hovering(false, cx),
                Delivery::Dropped(handed) => {
                    tell(Road::Pasteboard, &handed);
                    catcher.land(handed, cx);
                }
            });
            if told.is_err() {
                // The window is gone, and so is everything that would
                // have been told about this.
                break;
            }
        }
    }))
}

#[cfg(not(target_os = "macos"))]
fn deliveries(_window: &Window, _cx: &Context<Catcher>) -> Option<Task<()>> {
    // E10. GPUI's own file drops still arrive through [`zone`].
    None
}

/// Install the platform's dragging destination on this window.
///
/// Separate from [`Catcher::new`] only on macOS, where it is the half
/// that cannot be undone: a view inserted into the tree stays there for
/// the life of the window.
#[cfg(target_os = "macos")]
pub fn accept(window: &Window) {
    if !crate::pasteboard::attach(window) {
        tracing::warn!("this window will only accept files");
    }
}

#[cfg(not(target_os = "macos"))]
pub fn accept(_window: &Window) {
    // A drop file nobody took — a text let go where no zone was, or a run
    // that ended between GPUI's write and our read — is removed here, off
    // the drawing thread (D656).
    #[cfg(target_os = "linux")]
    if let Some(dir) = dropped_text::dir() {
        std::thread::spawn(move || dropped_text::sweep(&dir, dropped_text::STALE));
    }
}

/// Wrap an element so that whatever is dropped on it lands in
/// `catcher`.
///
/// The highlight is included rather than left to the caller, because a
/// drop zone that does not say it is one is a window people drag things
/// at and give up on. A caller that wants a different look reads
/// [`Catcher::over`] and paints it itself.
pub fn zone(element: Div, catcher: &Entity<Catcher>, cx: &App) -> Div {
    let over = catcher.read(cx).over;
    let landing = catcher.clone();

    element
        .when(over, |element| {
            element
                .bg(cx.theme().accent)
                .border_color(cx.theme().primary)
        })
        // GPUI's own file drop, which is the whole of the story off
        // macOS and the fallback on it — see the module docs.
        .drag_over::<ExternalPaths>(|style, _, _, cx| style.bg(cx.theme().accent))
        .on_drop::<ExternalPaths>(move |paths, _, cx| {
            let handed = handed_of(paths);
            tell(Road::ExternalPaths, &handed);
            landing.update(cx, |catcher, cx| catcher.land(handed, cx));
        })
}

/// The group the overlay's card answers to: shown only while files are
/// dragged over the overlay.
const OVERLAY_GROUP: &str = "drop-overlay";

/// A drop target over the whole of a window's content, as heretic-lazy-shot
/// has one: while files are dragged over the window it is a dashed frame and
/// a card saying "let go", and a drop anywhere in it lands in `catcher`.
///
/// It exists because a drop is taken only by an element the pointer is
/// *over* in GPUI's sense — and a hover card, a tooltip or a popover lying
/// on the table blocks the table from being over, so a file let go there
/// was lost with no line in the log (acceptance F9b). Deferred at just
/// under [`dialog::MODAL_PRIORITY`](crate::dialog::MODAL_PRIORITY) it is
/// over every overlay but a modal question, which a drop must not reach.
/// It never blocks the mouse: its hitbox is an ordinary one, so the rows,
/// buttons and cards under it hover and click as before, and it paints
/// nothing unless a file is being dragged. The first element that takes a
/// drop takes it from GPUI, so the table's own zone never sees it twice.
pub fn overlay(catcher: &Entity<Catcher>, cx: &App) -> AnyElement {
    let landing = catcher.clone();
    let theme = cx.theme();
    let (card, edge, ink) = (theme.background, theme.border, theme.foreground);
    deferred(
        div()
            .id("drop-overlay")
            .group(OVERLAY_GROUP)
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .drag_over::<ExternalPaths>(|style, _, _, cx| {
                style
                    .bg(cx.theme().primary.opacity(0.08))
                    .border_2()
                    .border_dashed()
                    .border_color(cx.theme().primary)
            })
            .on_drop::<ExternalPaths>(move |paths, _, cx| {
                let handed = handed_of(paths);
                tell(Road::ExternalPaths, &handed);
                landing.update(cx, |catcher, cx| catcher.land(handed, cx));
            })
            .child(
                div()
                    .invisible()
                    .group_drag_over::<ExternalPaths>(OVERLAY_GROUP, |style| style.visible())
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .px(px(24.0))
                    .py(px(16.0))
                    .rounded_lg()
                    .border_1()
                    .bg(card)
                    .border_color(edge)
                    .text_color(ink)
                    .shadow_lg()
                    .child(Icon::new(IconName::FileImport).large())
                    .child(t(Message::DropOverlayRelease)),
            ),
    )
    .with_priority(crate::dialog::MODAL_PRIORITY - 1)
    .into_any_element()
}

/// What GPUI's own drop handed over, as this application takes it.
///
/// A path is a file the person dropped — except on Linux, where the
/// carried `gpui-pre-linux` hands a dragged *text* over as a file of its
/// own in a private drop directory, because GPUI's drop carries paths and
/// nothing else (acceptance F15, D653). Such a path is not a file anybody
/// chose: it is read as the text it holds, removed at once, and lands as a
/// [`Handed::Text`] — what a paste lands, with no file behind it, so the
/// Retention page treats it as a paste (D654). Read here, on the drawing
/// thread, because it is at most 1 MiB in a private directory that GPUI
/// wrote a moment ago, normally on the runtime directory's `tmpfs` (D655).
fn handed_of(paths: &ExternalPaths) -> Vec<Handed> {
    #[cfg(target_os = "linux")]
    if let Some(dir) = dropped_text::dir() {
        return dropped_text::handed_from(paths.paths(), &dir);
    }
    paths.paths().iter().cloned().map(Handed::Path).collect()
}

/// The application's half of a dragged text on Linux: where the carried
/// `gpui-pre-linux` writes it, and the reading back.
///
/// The directory is the one `vendor/gpui-pre-linux/src/linux/platform.rs`
/// (`dropped_text_dir`) writes to, by the same rule: `wipemark-dropped-text`
/// under an absolute `$XDG_RUNTIME_DIR`, else `wipemark-dropped-text-<uid>`
/// under the temporary directory. The two cannot share code — one is
/// GPUI's crate, carried, and the application does not depend on it — so
/// the rule is stated twice and the live check
/// (`scripts/verify/linux-drop/quick-drop.sh`, its text steps) holds them
/// together.
#[cfg(target_os = "linux")]
mod dropped_text {
    use std::io;
    use std::os::unix::fs::MetadataExt as _;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime};

    use wipemark_intake::Handed;

    /// GPUI refuses a dragged text over a mebibyte — the MCP body's limit —
    /// and a drop file larger than that is not one it wrote.
    const LIMIT: u64 = 1 << 20;

    /// A drop file is read in the dispatch of the drop that wrote it, a
    /// few milliseconds; one a minute old was never taken.
    pub(super) const STALE: Duration = Duration::from_secs(60);

    pub(super) fn dir() -> Option<PathBuf> {
        match std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from) {
            Some(dir) if dir.is_absolute() => Some(dir.join("wipemark-dropped-text")),
            _ => {
                // The process's own uid, without a call into libc: Linux
                // gives `/proc/self` to the process's effective user.
                let uid = std::fs::metadata("/proc/self").ok()?.uid();
                Some(std::env::temp_dir().join(format!("wipemark-dropped-text-{uid}")))
            }
        }
    }

    /// Every path as it was handed over, except one in `dir`, which is the
    /// text it holds — or nothing, when it cannot be read as text — and is
    /// gone from the disk either way.
    pub(super) fn handed_from(paths: &[PathBuf], dir: &Path) -> Vec<Handed> {
        paths
            .iter()
            .filter_map(|path| {
                if path.parent() == Some(dir) {
                    take(path)
                } else {
                    Some(Handed::Path(path.clone()))
                }
            })
            .collect()
    }

    fn take(path: &Path) -> Option<Handed> {
        let read = read(path);
        let removed = std::fs::remove_file(path);
        if let Err(error) = removed {
            tracing::warn!(%error, "a dragged text's drop file could not be removed");
        }
        match read {
            Ok(text) => {
                tracing::debug!(
                    bytes = text.len(),
                    "a dragged text was read from its drop file"
                );
                Some(Handed::Text(text))
            }
            Err(error) => {
                tracing::warn!(%error, "a dragged text's drop file could not be read");
                None
            }
        }
    }

    fn read(path: &Path) -> io::Result<String> {
        let metadata = std::fs::symlink_metadata(path)?;
        if !metadata.is_file() {
            return Err(io::Error::other("not a regular file"));
        }
        if metadata.len() > LIMIT {
            return Err(io::Error::other("over the limit of a dragged text"));
        }
        std::fs::read_to_string(path)
    }

    /// Remove the drop files in `dir` older than `stale`; how many went.
    pub(super) fn sweep(dir: &Path, stale: Duration) -> usize {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return 0;
        };
        let now = SystemTime::now();
        let mut removed = 0;
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(metadata) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            let old = metadata
                .modified()
                .ok()
                .and_then(|modified| now.duration_since(modified).ok())
                .is_some_and(|age| age >= stale);
            let ours = path.extension().is_some_and(|extension| extension == "txt");
            if metadata.is_file() && ours && old && std::fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }
        if removed > 0 {
            tracing::info!(removed, "drop files nobody took were removed");
        }
        removed
    }
}

/// Which of the two roads in a drop came by — see the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Road {
    /// Our own dragging destination on macOS (`crate::pasteboard`).
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Pasteboard,
    /// GPUI's `ExternalPaths` drag: files (and, on Linux, a dragged text
    /// as its drop file), the whole story off macOS.
    ExternalPaths,
}

impl Road {
    fn name(self) -> &'static str {
        match self {
            Self::Pasteboard => "pasteboard",
            Self::ExternalPaths => "external-paths",
        }
    }
}

/// How many things a drop that reached us carried — what the log says,
/// one line per drop, so a drop lost before it got here is told apart
/// from one that arrived empty (acceptance F9, D624). Counts and the road
/// only: never a path or a text, which is the document (`dev/architecture/
/// logging.md`).
fn things_in(handed: &[Handed]) -> usize {
    handed.iter().filter(|handed| !handed.is_nothing()).count()
}

/// The log line for a drop that reached this window: INFO with the road
/// and the count, WARN when it carried nothing this window can take.
fn tell(road: Road, handed: &[Handed]) {
    let things = things_in(handed);
    if things == 0 {
        tracing::warn!(
            road = road.name(),
            items = handed.len(),
            "a drop carried nothing"
        );
    } else {
        tracing::info!(road = road.name(), things, "a drop arrived");
    }
}

/// Bytes, as somebody reads them off a window.
///
/// Kilobytes are in here and are not in `models::bytes_label`, which is
/// the same question asked about a different subject: a model's weights
/// are never measured in kilobytes and a "0.0 GB" beside a tokenizer
/// reads as broken, while a dropped note is a couple of kilobytes and
/// "3412 B" is a number a person has to count the digits of.
pub fn size_label(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "kB", "MB", "GB", "TB"];
    if bytes < 1000 {
        return format!("{bytes} {}", UNITS[0]);
    }
    let mut value = bytes as f64 / 1000.0;
    let mut unit = 1;
    while value >= 1000.0 && unit + 1 < UNITS.len() {
        value /= 1000.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use gpui::{AppContext as _, Entity, TestAppContext};
    use wipemark_intake::{Handed, Kind};

    use super::{things_in, Catcher, Landed};

    /// The log's count is what `land` keeps, so a drop of empty text is
    /// the WARN "carried nothing" and not an INFO of one thing (D624).
    #[test]
    fn the_log_counts_what_lands() {
        assert_eq!(things_in(&[]), 0);
        assert_eq!(things_in(&[Handed::Text(" \n\t".into())]), 0);
        assert_eq!(
            things_in(&[
                Handed::Text(String::new()),
                Handed::Path("/tmp/a.txt".into()),
                Handed::Text("\u{00A0}".into()),
            ]),
            2
        );
    }

    /// A private directory like the one GPUI writes a dragged text to.
    #[cfg(target_os = "linux")]
    fn drop_directory(name: &str) -> std::path::PathBuf {
        use std::os::unix::fs::DirBuilderExt as _;
        let dir =
            std::env::temp_dir().join(format!("wipemark-drop-test-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
        dir
    }

    /// A path in the drop directory is a dragged text, not a file anybody
    /// chose: it lands as the text, and the file is gone at once (F15,
    /// D654). A file the person dropped beside it is handed on untouched.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_dragged_text_lands_as_text_and_its_file_is_gone() {
        let dir = drop_directory("text");
        let dropped = dir.join("1-2-3.txt");
        std::fs::write(&dropped, "words dragged out of a browser").unwrap();
        let elsewhere = std::env::temp_dir().join(format!(
            "wipemark-drop-test-file-{}.txt",
            std::process::id()
        ));
        std::fs::write(&elsewhere, "a file somebody chose").unwrap();

        let handed = super::dropped_text::handed_from(&[dropped.clone(), elsewhere.clone()], &dir);

        assert_eq!(
            handed,
            vec![
                Handed::Text("words dragged out of a browser".to_owned()),
                Handed::Path(elsewhere.clone()),
            ]
        );
        assert!(!dropped.exists(), "the drop file was left behind");
        assert!(elsewhere.exists(), "a file the person dropped was touched");
        std::fs::remove_file(&elsewhere).ok();
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A drop file that is not text lands nothing — and is still removed.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_drop_file_that_is_not_text_lands_nothing_and_goes() {
        let dir = drop_directory("binary");
        let dropped = dir.join("1-2-4.txt");
        std::fs::write(&dropped, [0xff, 0xfe, 0x00]).unwrap();

        assert!(super::dropped_text::handed_from(std::slice::from_ref(&dropped), &dir).is_empty());
        assert!(!dropped.exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A drop file nobody took is swept once it is stale, and one that is
    /// still being handed over is not.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_stale_drop_file_is_swept_and_a_fresh_one_kept() {
        let dir = drop_directory("sweep");
        let stale = dir.join("1-2-5.txt");
        let fresh = dir.join("1-2-6.txt");
        std::fs::write(&stale, "old").unwrap();
        std::fs::write(&fresh, "new").unwrap();
        let file = std::fs::File::options().write(true).open(&stale).unwrap();
        file.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(120))
            .unwrap();

        assert_eq!(
            super::dropped_text::sweep(&dir, super::dropped_text::STALE),
            1
        );
        assert!(!stale.exists());
        assert!(fresh.exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A catcher with no window behind it: everything below is about
    /// what happens *after* a delivery, and the delivery is handed in
    /// by the test.
    fn loose(cx: &mut TestAppContext) -> Entity<Catcher> {
        cx.new(|_| Catcher {
            over: false,
            caught: None,
            reads: 0,
            _deliveries: None,
        })
    }

    /// The examination happens off the drawing thread and lands back on
    /// it: what a view reads afterwards is the answer, not the raw
    /// bytes it was handed.
    #[gpui::test]
    fn what_lands_is_examined_and_kept(cx: &mut TestAppContext) {
        let catcher = loose(cx);

        catcher.update(cx, |catcher: &mut Catcher, cx| {
            catcher.land(
                vec![
                    Handed::Text("a line of prose".to_owned()),
                    Handed::Bytes {
                        name: Some("shot.png".to_owned()),
                        bytes: b"\x89PNG\r\n\x1a\n".to_vec(),
                    },
                ],
                cx,
            );
        });
        cx.run_until_parked();

        catcher.read_with(cx, |catcher: &Catcher, _| {
            let caught = catcher.caught().expect("a drop was made");
            let kinds: Vec<Kind> = caught.iter().map(|arrival| arrival.intake.kind).collect();
            assert_eq!(kinds, vec![Kind::Text, Kind::Image]);
            // The thing itself is kept beside its description: a
            // preview reads the note, not the fact that it is a note.
            assert_eq!(
                caught[0].handed,
                Handed::Text("a line of prose".to_owned()),
                "what was handed over did not survive being recognised"
            );
        });
    }

    /// A second drop while the first is still being read off a slow
    /// disk is the answer; the first one is not allowed to arrive after
    /// it and overwrite what is on screen.
    #[gpui::test]
    fn a_second_drop_overtakes_the_first(cx: &mut TestAppContext) {
        let catcher = loose(cx);

        catcher.update(cx, |catcher: &mut Catcher, cx| {
            catcher.land(vec![Handed::Text("first".to_owned())], cx);
            catcher.land(
                vec![Handed::Bytes {
                    name: None,
                    bytes: b"GIF89a".to_vec(),
                }],
                cx,
            );
        });
        cx.run_until_parked();

        catcher.read_with(cx, |catcher: &Catcher, _| {
            let caught = catcher.caught().expect("a drop was made");
            assert_eq!(caught.len(), 1);
            assert_eq!(caught[0].intake.kind, Kind::Image);
        });
    }

    /// The other reader. A queue keeps every drop, so the one that was
    /// overtaken on screen still lands as its own event — dropping two
    /// things quickly must not lose the first of them from a list.
    #[gpui::test]
    fn every_drop_lands_even_when_overtaken(cx: &mut TestAppContext) {
        let catcher = loose(cx);
        let landed: Rc<RefCell<Vec<Vec<Kind>>>> = Rc::default();
        let heard = landed.clone();
        cx.update(|cx| {
            cx.subscribe(&catcher, move |_, Landed(arrivals), _| {
                heard
                    .borrow_mut()
                    .push(arrivals.iter().map(|arrival| arrival.intake.kind).collect());
            })
            .detach();
        });

        catcher.update(cx, |catcher: &mut Catcher, cx| {
            catcher.land(vec![Handed::Text("first".to_owned())], cx);
            catcher.land(
                vec![Handed::Bytes {
                    name: None,
                    bytes: b"GIF89a".to_vec(),
                }],
                cx,
            );
        });
        cx.run_until_parked();

        let mut heard = landed.borrow().clone();
        heard.sort();
        assert_eq!(
            heard,
            vec![vec![Kind::Text], vec![Kind::Image]],
            "a drop went missing between the pasteboard and the queue"
        );
    }

    /// The unit that this application has and the Models page does
    /// not: a dropped note is two kilobytes, and a window that says
    /// "2048 B" is asking somebody to count digits.
    #[test]
    fn a_size_is_read_rather_than_counted() {
        assert_eq!(super::size_label(0), "0 B");
        assert_eq!(super::size_label(812), "812 B");
        assert_eq!(super::size_label(2_048), "2.0 kB");
        assert_eq!(super::size_label(1_240_000), "1.2 MB");
        assert_eq!(super::size_label(7_432_229_248), "7.4 GB");
    }

    /// The highlight is a state, not an event: it goes on while
    /// something is held over the window and comes off when it lands,
    /// whatever the drop turned out to contain.
    #[gpui::test]
    fn the_highlight_comes_off_when_it_lands(cx: &mut TestAppContext) {
        let catcher = loose(cx);

        catcher.update(cx, |catcher: &mut Catcher, cx| {
            assert!(catcher.caught().is_none(), "nothing has been dropped yet");
            catcher.hovering(true, cx);
            assert!(catcher.over());
            catcher.land(Vec::new(), cx);
            assert!(!catcher.over());
        });
        cx.run_until_parked();

        // A drop that carried nothing is not the same answer as never
        // having been dropped on, and the panel says different things
        // about the two.
        catcher.read_with(cx, |catcher: &Catcher, _| {
            assert_eq!(catcher.caught(), Some(&[][..]));
        });
    }
}
