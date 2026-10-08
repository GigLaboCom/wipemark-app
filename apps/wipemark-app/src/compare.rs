//! The Compare window — the result beside its original.
//!
//! Epic **E7**. The fourth window of this application, and the first
//! one that shows a document rather than a list of them: the original
//! on the left, read-only, and the result on the right, editable, with
//! every line that differs marked on both sides — a red mark on the
//! original for a line the result no longer has, a green one on the
//! result for a line the original never had. It is opened from a row of
//! the queue's Actions menu and from `--compare=<path>`, one window per
//! thing, and it belongs to nothing: closing it writes nothing, and
//! nothing else in the application reads what is in it. The result it
//! opens with is the cleaned text; see "What the result is today".
//!
//! # Three things, kept apart
//!
//! The arithmetic is [`crate::diff`], a pure function over two strings.
//! The right-hand pane is [`crate::result`], an editor with a toolbar
//! that knows nothing about originals. This module is what is left: a
//! window, a reader that turns what arrived into text on the background
//! executor, the marks each side paints, and the rule that keeps the
//! two sides in step. Each of the three is tested without the others.
//!
//! # What "the result" is today
//!
//! The result is what Layer A makes of the original —
//! `wipemark_core::clean` at its default options, computed on the
//! background executor in the same task that reads the text, and never
//! on the thread that draws the window. Layer A is deterministic, so it
//! is the very text the queue's Clean writes for the same document
//! (`the_result_is_what_the_queue_writes`). Back to the cleaned text
//! puts that result back, not the original. The window is a reader and
//! not a writer: editing the result saves nothing and closing the
//! window writes nothing, and the banner says both. Layer B's rewrite
//! is not shown here yet; when it is, it is shown through this same
//! comparison.
//!
//! # Finer than a line, through the library's own road
//!
//! Within a passage that changed — lines on both sides — the words or
//! characters that differ are marked more strongly, at the [`Grain`]
//! the Compare page in Settings names. The line marks go through the
//! library's `LineDecorationProvider`; a mark *within* a line has one
//! public road in the vendored editor, its `DocumentColorProvider` —
//! the LSP `textDocument/documentColor` shape, a colour painted behind
//! a range of text with the text recoloured to read over it — and that
//! is the road these take. [`Inline`] is the provider. It is asked
//! only when the text it is over changes, which the result's does with
//! every keystroke and the original's never does; so once a comparison
//! has landed, the original is asked again through an *empty edit* at
//! its cursor — the one public way to put that question, on the one
//! side where an edit of nothing costs nothing (its history is
//! nobody's, and the text is untouched). What it does cost is a
//! selection in the original, which collapses when the marks move;
//! see [`CompareView::repaint_original`]. The grain is read when the
//! window opens and kept for its life, because turning the marks *on*
//! in an open window would need that same question put to the
//! result, where an empty edit is an entry in the user's undo history.
//! The page says so.
//!
//! # The two sides stay in step: by scrolling, and by the cursor
//!
//! Two mechanisms, each a row on the Compare page, both on by default.
//!
//! **Scrolling together.** Scrolling either pane — the wheel, a
//! touchpad, the scroll bar dragged or clicked, the keyboard paging —
//! scrolls the other so that the lines the two share stay level, the
//! way IntelliJ's diff viewer does. Vertically only: each side keeps
//! its own horizontal scroll. The editor's scroll offset is public
//! (`scroll_offset`, `set_scroll_offset`), so this needs nothing of the
//! library it does not already offer. The leading pane's top is a row
//! and the fraction of it scrolled past; [`Diff::position_across`]
//! carries it to the other side — line for line, fraction included, in
//! a stretch the two share, and **in proportion** through a passage
//! that changed, holding still through lines the other side has none
//! of (D380, D381) — and the other pane is set to stand there. A scroll
//! is seen two ways: by the editor's notification, which a wheel and
//! the scroll bar make at once and the library makes again after a
//! frame in which a scroll it applied itself (the keyboard's, a caret
//! brought into view) moved the text; and by a look at both panes once
//! every frame they are painted in is done, the one moment a pane that
//! wraps its lines has a layout that agrees with its offset (D386).
//! The pane that follows notifies too, and must not lead back: the
//! window remembers what it asked of each pane, and a move in the asked
//! direction no farther than asked — all of it, or what the pane's own
//! end let it do — is that ask landing, not a lead (D382). With the
//! result's lines wrapped the library does not say how many lines each
//! row became, so a wrapped pane's place is read off its last layout
//! and estimated where the row is not laid out: the two sides line up
//! roughly there, and the page says so (D384).
//!
//! **The original follows the cursor.** Whenever the result's caret
//! changes line, the original's is put on the line that stands where
//! that one does — [`Diff::original_row_of`] is the map. Placing a
//! cursor focuses the editor it is placed in, which would take the
//! keyboard out of the result mid-word; the focus is handed straight
//! back in the same update, and GPUI notices a focus change only at the
//! next frame, so nothing blurs. One direction only: the result leads,
//! because it is the side being written in.
//!
//! **Which wins.** The result, always (D383). With both on, a follow
//! places the original's caret but does not let it scroll the original
//! by itself: the original is set where the result's top puts it, which
//! replaces the scroll the caret had queued — and the result is never
//! moved by a follow. Without that, the original would scroll only as
//! far as its caret and then lead the result back to it, taking the
//! result's caret out of sight. When both panes moved in one frame, the
//! result is looked at first and the original's own move is only noted
//! (D392). A comparison recomputed after an edit changes the map for the
//! next scroll and moves neither pane: its empty edit in the original
//! puts the original's offset back as it stood (D390). An ask that moved
//! nothing is dropped after its frame, so it cannot swallow a later move
//! (D391).
//!
//! # What it refuses
//!
//! A picture, a folder, an archive: nothing to compare line by line,
//! and the window says so rather than showing two empty panes. A text
//! past [`TEXT_LIMIT`] is refused with its size, because the editor
//! holds its whole document in memory twice over here — once a side —
//! and a two-gigabyte model file dropped by mistake is not a document.
//! The Actions item is greyed on a row that is not text, so the first
//! refusal is one a person reaches only from the command line.

use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    actions, canvas, div, px, size, Anchor, AnyElement, AnyWindowHandle, App, Bounds, ClickEvent,
    Context, Entity, FocusHandle, Focusable, Global, Hsla, KeyBinding, Pixels, Rgba, SharedString,
    Size, Subscription, Task, TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{
    DocumentColorProvider, EditorState, GutterMarker, LineDecoration, LineDecorationCollection,
    LineDecorationProvider, Position, Rope,
};
use gpui_component::popover::Popover;
use gpui_component::resizable::{h_resizable, resizable_panel};
use gpui_component::{
    h_flex, v_flex, ActiveTheme, Disableable as _, Root, Sizable as _, StyledExt as _, Theme,
};
use lsp_types::{Color, ColorInformation};
use wipemark_core::Options;
use wipemark_i18n::{args, t, t_args, Message};
use wipemark_intake::{Encoding, Handed, Intake, Kind};

use crate::diff::{Diff, Grain, Side};
use crate::icon::{Icon, IconName};
use crate::result::{self, ResultEditor, ResultEvent, TOOLBAR_HEIGHT};
use crate::screen::{self, Screen};
use crate::title::{self, Title};
use crate::{clean, drop, placement, wording};

actions!(wipemark, [CloseCompare]);

/// Key context of the window, so ⌘W means *this* window. Escape is
/// deliberately not bound here: in an editor it dismisses the search
/// panel and drops a selection, and a window that vanished on it would
/// take a half-written result with it.
const CONTEXT: &str = "Compare";

/// The most text the window takes, in bytes as they arrived.
///
/// Eight megabytes is a few thousand pages. The editor keeps the
/// whole document as a rope, this window keeps two of them and the
/// original once more to compare against, and past this the cost is
/// no longer the window's to hide.
pub const TEXT_LIMIT: u64 = 8 * 1024 * 1024;

/// How big the window opens the first time.
const DEFAULT_SIZE: Size<Pixels> = size(px(1120.0), px(720.0));

/// The least a side-by-side can be read at.
const MIN_SIZE: Size<Pixels> = size(px(640.0), px(400.0));

/// How long typing is left to settle before the marks are recomputed.
///
/// The comparison is cheap and runs on the background executor, so
/// this is not about cost — it is that marks flickering per keystroke
/// through a word are noise, and marks that land as the hands pause
/// are the answer.
const SETTLE: Duration = Duration::from_millis(120);

/// The Compare page's rows, as values — what a window is opened with.
///
/// Read once, when the window opens, and kept for its life; see the
/// module docs for why the finer marks cannot be turned on in a window
/// that is already open, and the page's description, which says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Comparison {
    /// How finely a changed passage is marked.
    pub grain: Grain,
    /// Whether the original follows the result's cursor.
    pub follow: bool,
    /// Whether scrolling either side scrolls the other.
    pub sync_scroll: bool,
}

impl Default for Comparison {
    /// Words, following and scrolling together: the marks a reader of
    /// a rewrite wants, and the two sides kept in step.
    fn default() -> Self {
        Self {
            grain: Grain::Words,
            follow: true,
            sync_scroll: true,
        }
    }
}

impl Grain {
    /// The radio button's label. The ids live with the type, in
    /// `diff`; the words live here, with the window that shows them.
    pub fn title(self) -> Message {
        match self {
            Grain::Lines => Message::SettingsCompareGrainLines,
            Grain::Words => Message::SettingsCompareGrainWords,
            Grain::Characters => Message::SettingsCompareGrainCharacters,
        }
    }
}

/// The windows that are open, by what they were opened on.
///
/// A global for the reason the panel's is: the queue asks "is there one
/// for this row already" and cannot see the window to ask it.
#[derive(Default)]
struct Opened(HashMap<u64, AnyWindowHandle>);

impl Global for Opened {}

/// Bind ⌘W inside the window. Called once from `main`, after
/// `gpui_component::init`.
pub fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("secondary-w", CloseCompare, Some(CONTEXT))]);
}

/// What a window is opened on: the thing, and what it was established
/// to be — or not yet, for a path named on the command line, which is
/// examined on the way in like a dropped one.
#[derive(Clone)]
pub struct Subject {
    pub handed: Handed,
    pub intake: Option<Intake>,
    /// What the right-hand pane opens with: what cleaning makes of the
    /// original, or a rewrite's result (E4-6b, R5).
    pub made: Made,
}

/// What the result pane opens with and Reset puts back.
#[derive(Clone)]
pub enum Made {
    /// `clean(original)` at Layer A's defaults — the text the queue's Clean
    /// writes for the same document.
    Cleaned,
    /// A rewrite's result, as it was delivered: the file it was written
    /// to, or the batch queue's row that holds it. `kept` is how many of
    /// how many paragraphs kept their cleaned original, when known.
    Rewritten {
        from: RewriteFrom,
        kept: Option<(u32, u32)>,
    },
}

/// Where a rewrite's result is.
#[derive(Clone)]
pub enum RewriteFrom {
    /// Written to this file.
    File(std::path::PathBuf),
    /// In the batch queue's row, the one home a text with no file has.
    Item(Arc<wipemark_queue::Queue>, wipemark_queue::ItemId),
}

impl std::fmt::Debug for Made {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Made::Cleaned => f.write_str("Cleaned"),
            Made::Rewritten { kept, .. } => f
                .debug_struct("Rewritten")
                .field("kept", kept)
                .finish_non_exhaustive(),
        }
    }
}

/// What the result pane holds, as the window words it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MadeKind {
    Cleaned,
    Rewritten { kept: Option<(u32, u32)> },
}

impl std::fmt::Debug for Subject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subject")
            .field("made", &self.made)
            .finish_non_exhaustive()
    }
}

/// Why there is nothing to compare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Not made of characters.
    NotText,
    /// More than [`TEXT_LIMIT`].
    TooBig { size: u64 },
    /// A file that would not open, or a path that led nowhere.
    Unreadable,
    /// Not valid in the encoding it announced, at this byte — what the
    /// queue's row refuses too (D282).
    Undecodable { encoding: Encoding, offset: usize },
    /// Text intake could not read as the queue would: an eight-bit encoding
    /// it does not name, or nothing established from the bytes.
    Unable(clean::Unable),
}

impl Refusal {
    /// The sentence the window shows for it.
    pub fn sentence(&self) -> String {
        match self {
            Refusal::NotText => t(Message::CompareRefusedNotText),
            Refusal::TooBig { size } => t_args(
                Message::CompareRefusedTooBig,
                &args!(
                    "size" => drop::size_label(*size),
                    "limit" => drop::size_label(TEXT_LIMIT)
                ),
            ),
            Refusal::Unreadable => t(Message::CompareRefusedUnreadable),
            // The queue's own sentences, through the queue's own words.
            Refusal::Undecodable { encoding, offset } => wording::refused_in(
                &wording::window,
                &clean::Refusal::Undecodable {
                    encoding: *encoding,
                    offset: *offset,
                },
            ),
            Refusal::Unable(unable) => wording::unable_in(&wording::window, *unable),
        }
    }

    /// What the clean's read refused, as the window says it. A size and an
    /// unreadable file keep Compare's own sentences; what is not text is
    /// Compare's "not text".
    fn of(refusal: clean::Refusal) -> Self {
        use clean::Unable;
        match refusal {
            clean::Refusal::TooBig { size, .. } => Refusal::TooBig { size },
            clean::Refusal::Unreadable(_) => Refusal::Unreadable,
            clean::Refusal::Undecodable { encoding, offset } => {
                Refusal::Undecodable { encoding, offset }
            }
            clean::Refusal::NotCleanable(unable @ (Unable::UnnamedEncoding | Unable::Unread)) => {
                Refusal::Unable(unable)
            }
            _ => Refusal::NotText,
        }
    }
}

/// The text, once it has been read, what Layer A makes of it, and what
/// to call it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    pub name: String,
    /// The original, as it was decoded.
    pub text: String,
    /// The result the window opens with and Reset returns to:
    /// `wipemark_core::clean(text, &Options::default()).text`, the text
    /// the queue's Clean writes for the same document — or a rewrite's
    /// result as it was delivered.
    pub cleaned: String,
    /// Which of the two `cleaned` is.
    pub kind: MadeKind,
}

impl Subject {
    /// Whether a window would have something to show for this — what
    /// greys the Actions item. Text, in any container made of it.
    pub fn comparable(intake: &Intake) -> bool {
        intake.kind == Kind::Text || intake.is_textual()
    }

    /// Turn what arrived into text, and clean it.
    ///
    /// **Blocking**: it examines a path that was not examined yet,
    /// reads a whole file and runs Layer A over all of it. Call it off
    /// the foreground thread. The read and the decoding are the queue's
    /// Clean's, `clean::text_of` — strict, the limit checked on the size
    /// before the read — so Compare opens exactly what the queue would
    /// clean and refuses what it would refuse (D282); the lenient decode
    /// is the preview's alone.
    pub fn read(self) -> Result<Loaded, Refusal> {
        let intake = self
            .intake
            .unwrap_or_else(|| wipemark_intake::of(&self.handed));
        if !Self::comparable(&intake) {
            return Err(Refusal::NotText);
        }
        // A path that leads nowhere is unreadable, not text of no kind.
        if let Handed::Path(path) = &self.handed {
            if std::fs::metadata(path).is_err() {
                return Err(Refusal::Unreadable);
            }
        }
        let name = wording::title_of(&intake);
        let arrival = drop::Arrival {
            intake,
            handed: self.handed,
        };
        let text = clean::text_of(&arrival).map_err(Refusal::of)?;
        let (cleaned, kind) = match self.made {
            // Layer A at its defaults, as the queue's Clean and the CLI run
            // it: deterministic, so this is the text they write.
            Made::Cleaned => (
                wipemark_core::clean(&text, &Options::default()).text,
                MadeKind::Cleaned,
            ),
            // The rewrite as it was delivered, read back — nothing is
            // rewritten here, and nothing is recomputed (D273's shape).
            Made::Rewritten { from, kept } => {
                (rewritten_text(&from)?, MadeKind::Rewritten { kept })
            }
        };
        Ok(Loaded {
            name,
            text,
            cleaned,
            kind,
        })
    }
}

/// A rewrite's result, read from where it was delivered. Blocking.
pub fn rewritten_text(from: &RewriteFrom) -> Result<String, Refusal> {
    match from {
        RewriteFrom::File(path) => {
            let arrival = drop::Arrival {
                intake: wipemark_intake::of_path(path),
                handed: Handed::Path(path.clone()),
            };
            clean::text_of(&arrival).map_err(Refusal::of)
        }
        RewriteFrom::Item(queue, item) => queue
            .result(*item)
            .ok()
            .flatten()
            .and_then(|result| result["text"].as_str().map(str::to_owned))
            .ok_or(Refusal::Unreadable),
    }
}

/// Open a window on `subject`, or bring forward the one already open
/// on `key`.
///
/// `key` is whatever the caller tells its subjects apart by — a queue
/// row's id — and `None` is a window nobody will ask for again, which
/// is what the command line opens. `comparison` is the Compare page's
/// rows as they stand, which the window keeps. `main` is what the new
/// window is centred over, handed in rather than looked up for the
/// reason `settings::open` gives: from inside the main window's own
/// update it would come back "not found". Every caller defers to here.
pub fn open(
    key: Option<u64>,
    subject: Subject,
    comparison: Comparison,
    main: AnyWindowHandle,
    cx: &mut App,
) {
    if let Some(key) = key {
        let already = cx
            .try_global::<Opened>()
            .and_then(|opened| opened.0.get(&key).copied())
            .filter(|handle| cx.windows().contains(handle));
        if let Some(handle) = already {
            if let Err(error) = handle.update(cx, |_, window, _| window.activate_window()) {
                tracing::warn!(%error, "could not bring a Compare window forward");
            }
            return;
        }
    }

    let (screen, bounds) = place(main, cx);
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                // The name comes with the text, a moment later.
                title: Some(
                    Title::Compare {
                        name: title::NAME_PENDING,
                    }
                    .text()
                    .into(),
                ),
                ..Default::default()
            }),
            window_min_size: Some(MIN_SIZE),
            display_id: screen.as_ref().map(|screen| screen.id),
            ..Default::default()
        },
        move |window, cx| {
            let view = cx.new(|cx| CompareView::new(subject, comparison, window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        },
    );

    match opened {
        Ok(window) => {
            if let Some(key) = key {
                cx.default_global::<Opened>()
                    .0
                    .insert(key, AnyWindowHandle::from(window));
            }
            cx.activate(true);
        }
        Err(error) => tracing::warn!(%error, "could not open the Compare window"),
    }
}

/// Which screen the window goes on, and the rectangle it takes there:
/// centred over the main window, and kept on its screen.
fn place(main: AnyWindowHandle, cx: &mut App) -> (Option<Screen>, Bounds<Pixels>) {
    let main = main
        .update(cx, |_, window, cx| {
            let chrome = window.bounds().size.height - window.viewport_size().height;
            screen::of_window(window, cx).map(|screen| (screen, window.bounds(), chrome))
        })
        .ok()
        .flatten();

    let Some((screen, over, chrome)) = main else {
        let Some(screen) = screen::primary(cx) else {
            return (
                None,
                Bounds::new(gpui::point(px(0.0), px(0.0)), DEFAULT_SIZE),
            );
        };
        let frame = screen::centred(DEFAULT_SIZE, screen.visible);
        return (
            Some(screen.clone()),
            screen::contained(frame, screen.visible),
        );
    };

    // Frames — what a person sees centred — and only the last line
    // converts back to the content rectangle GPUI is handed. See
    // `settings::place`.
    let wanted = screen::centred(placement::frame_of_size(DEFAULT_SIZE, chrome), over);
    let frame = screen::contained(wanted, screen.visible);
    (Some(screen.clone()), placement::content_of(frame, chrome))
}

/// The marks one side paints: the rows to mark, ascending, and which
/// side they are on.
///
/// The library asks for the visible rows once per frame; the rows are
/// sorted, so the answer is two binary searches and a slice. The colour
/// is read from the theme *at paint time* rather than stored, which is
/// what makes a theme switch repaint the marks with everything else.
pub struct Marks {
    rows: Vec<u32>,
    side: Side,
}

impl Marks {
    pub fn new(rows: Vec<u32>, side: Side) -> Self {
        debug_assert!(rows.windows(2).all(|pair| pair[0] < pair[1]));
        Self { rows, side }
    }

    /// The rows within `visible`, which is what the library will paint.
    pub fn within(&self, visible: Range<usize>) -> &[u32] {
        let start = self
            .rows
            .partition_point(|&row| (row as usize) < visible.start);
        let end = self
            .rows
            .partition_point(|&row| (row as usize) < visible.end);
        &self.rows[start..end]
    }
}

impl LineDecorationProvider for Marks {
    fn line_decorations(&self, rows: Range<usize>, cx: &App) -> Vec<LineDecoration> {
        let theme = cx.theme();
        let (marker, tint) = match self.side {
            Side::Original => (GutterMarker::DiffRemoved, theme.danger.opacity(0.16)),
            Side::Result => (GutterMarker::DiffAdded, theme.success.opacity(0.16)),
        };
        self.within(rows)
            .iter()
            .map(|&row| {
                LineDecoration::new(row as usize)
                    .with_background(tint)
                    .with_marker(marker.clone())
            })
            .collect()
    }
}

/// The marks within a line one side paints: the words — or characters
/// — of a changed passage that the other side does not have.
///
/// The library's document-colour provider, which is the one public road
/// to a colour behind a range of text; see the module docs. It is
/// asked with this side's own text and reads the other side's *now*,
/// so the answer is for the two texts as they stand when the question
/// is put — and it is put on the background executor, because the
/// comparison is. The colour is chosen when it is asked, from the
/// theme of the moment: the library keeps the answer as painted, so a
/// theme switch re-tints these at the next asking, which is the next
/// edit, and not before.
pub struct Inline {
    side: Side,
    grain: Grain,
    /// The original, as read. Never changes while the window lives.
    original: Arc<str>,
    /// The result's editor, whose text is read when the question is
    /// put.
    result: Entity<EditorState>,
}

impl Inline {
    pub fn new(side: Side, grain: Grain, original: Arc<str>, result: Entity<EditorState>) -> Self {
        Self {
            side,
            grain,
            original,
            result,
        }
    }
}

impl DocumentColorProvider for Inline {
    fn document_colors(
        &self,
        text: &Rope,
        _: &mut Window,
        cx: &mut App,
    ) -> Task<anyhow::Result<Vec<ColorInformation>>> {
        // `text` is this side's own text. The original's is what was
        // read, so the rope is not walked again for it.
        let original = self.original.clone();
        let result: String = match self.side {
            Side::Original => self.result.read(cx).value().to_string(),
            Side::Result => text.to_string(),
        };
        let (side, grain) = (self.side, self.grain);
        let color = tone(side, cx.theme());
        cx.background_executor().spawn(async move {
            let diff = Diff::of(&original, &result);
            let spans = diff.spans(grain, &original, &result);
            let (text, spans) = match side {
                Side::Original => (&*original, spans.removed),
                Side::Result => (result.as_str(), spans.added),
            };
            Ok(per_line(text, &spans)
                .into_iter()
                .map(|range| ColorInformation { range, color })
                .collect())
        })
    }
}

/// The colour behind a mark within a line, for `side`, in the theme as
/// it stands.
///
/// The hue is the side's — the danger red the original's line marks
/// use, the success green the result's — at a lightness the theme's
/// text reads over: the library recolours text it paints a colour
/// behind, black over a light one and white over a dark one, so a
/// light theme gets a light tint and a dark theme a dark one. Opaque,
/// so what it covers of the line tint does not shift the shade.
fn tone(side: Side, theme: &Theme) -> Color {
    let base = match side {
        Side::Original => theme.danger,
        Side::Result => theme.success,
    };
    let tint = Hsla {
        h: base.h,
        s: 0.55,
        l: if theme.is_dark() { 0.30 } else { 0.80 },
        a: 1.0,
    };
    let rgba: Rgba = tint.into();
    Color {
        red: rgba.r,
        green: rgba.g,
        blue: rgba.b,
        alpha: rgba.a,
    }
}

/// Byte spans of `text`, cut into one range per line in the editor's
/// own coordinates — a row, and a column counted in characters — with
/// the newline left out.
///
/// One per line and not one per span, because the library paints a
/// range only while *all* of it is laid out: a span reaching across
/// lines would vanish whole the moment either end scrolled out of
/// view. Empty pieces — a span that was only a newline — are not
/// answered, there being nothing to see.
pub fn per_line(text: &str, spans: &[Range<usize>]) -> Vec<lsp_types::Range> {
    let starts: Vec<usize> = std::iter::once(0)
        .chain(text.match_indices('\n').map(|(at, _)| at + 1))
        .collect();
    let mut out = Vec::new();
    for span in spans {
        let mut from = span.start;
        while from < span.end {
            let row = starts.partition_point(|&start| start <= from) - 1;
            let line_end = starts.get(row + 1).map_or(text.len(), |next| next - 1);
            let to = span.end.min(line_end);
            if to > from {
                let column = text[starts[row]..from].chars().count();
                let width = text[from..to].chars().count();
                out.push(lsp_types::Range::new(
                    Position::new(row as u32, column as u32),
                    Position::new(row as u32, (column + width) as u32),
                ));
            }
            from = line_end + 1;
        }
    }
    out
}

/// Less than this, in pixels, and two scroll offsets are one.
const HALF_PIXEL: f32 = 0.5;

/// One pane's scrolling as the window last saw it.
#[derive(Debug, Default, Clone, Copy)]
struct Track {
    /// The vertical offset the pane last reported.
    seen: Option<f32>,
    /// An offset the window asked of this pane and has not yet seen
    /// land.
    asked: Option<Asked>,
    /// How many times this pane has led the other — what the tests
    /// count a feedback loop by.
    #[cfg(test)]
    leads: u32,
}

/// A scroll the window asked of a pane: from where, to where.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Asked {
    from: f32,
    to: f32,
    /// Whether a frame has been painted since the ask was made — an
    /// ask that has had its frame and moved nothing is dropped (D391).
    painted: bool,
}

impl Asked {
    /// Whether a pane that moved from `from` to `now` did what was
    /// asked — all of it, or the part of it the pane's own end let it
    /// do. A scroll asked past the last line stops at the last line,
    /// and that is still the asked-for scroll arriving, not a person
    /// scrolling the follower. A move the other way, or past what was
    /// asked, is somebody else's (D382).
    fn lands(self, now: f32) -> bool {
        let wanted = self.to - self.from;
        let went = now - self.from;
        went != 0.0 && went.signum() == wanted.signum() && went.abs() <= wanted.abs() + HALF_PIXEL
    }
}

/// Where an editor's viewport stands, as a buffer row and the fraction
/// of it scrolled past — `None` before the editor has been laid out.
///
/// Without wrapping every row is one line high and this is the offset
/// divided by the line height, to the pixel. With wrapping the library
/// does not say how many lines a row became, so this is read off the
/// last layout instead: the first row it showed, and how far the
/// viewport's top is into that row's height (D384).
fn top_of(state: &EditorState, wraps: bool) -> Option<f64> {
    let line_height = f64::from(f32::from(state.line_height()?));
    if !wraps {
        return Some((-f64::from(f32::from(state.scroll_offset().y)) / line_height).max(0.0));
    }
    let row = state.visible_row_range()?.start;
    let bounds = state.row_bounds(row)?;
    let into = f32::from(state.input_bounds().origin.y - bounds.origin.y);
    let height = f32::from(bounds.size.height).max(1.0);
    Some(row as f64 + f64::from((into / height).clamp(0.0, 1.0)))
}

/// The vertical offset that puts `top` — a buffer row and a fraction —
/// at the top of an editor's viewport; `None` before it has been laid
/// out. Offsets go negative downwards, the library's way, and the
/// library clamps whatever it is handed to the text's length.
///
/// With wrapping, a row's place is read off the last layout where that
/// row is in it, and estimated from the rows that are where it is not —
/// see [`wrapped_offset_for`].
fn offset_for(state: &EditorState, wraps: bool, top: f64) -> Option<f32> {
    let top = top.max(0.0);
    if wraps {
        return wrapped_offset_for(state, top);
    }
    let line_height = f64::from(f32::from(state.line_height()?));
    Some((-(top * line_height)) as f32)
}

/// [`offset_for`] over wrapped lines (D384).
///
/// Where the row is laid out, its top is known to the pixel: where it
/// is drawn, less where the viewport is, less how far the text is
/// scrolled. Where it is not, its top is estimated from the rows that
/// are — above them, by the average height of every row above the
/// first one shown (which the offset knows exactly); below them, by
/// the average height of every row down to the last one shown. The
/// estimate lands near the row, and the next step of the leading
/// pane, with the row now laid out, lands on it.
fn wrapped_offset_for(state: &EditorState, top: f64) -> Option<f32> {
    let shown = state.visible_row_range()?;
    let viewport = state.input_bounds().origin.y;
    let scrolled = f64::from(f32::from(state.scroll_offset().y));
    // A row's top in the text, from where the last layout drew it.
    let text_top = |row: usize| {
        state.row_bounds(row).map(|bounds| {
            (
                f64::from(f32::from(bounds.origin.y - viewport)) - scrolled,
                bounds,
            )
        })
    };
    let row = top.floor() as usize;
    let fraction = top - top.floor();
    let y = if let Some((at, bounds)) = text_top(row) {
        at + fraction * f64::from(f32::from(bounds.size.height))
    } else {
        let (first, _) = text_top(shown.start)?;
        let last = (shown.start..shown.end)
            .rev()
            .find_map(|row| text_top(row).map(|(at, bounds)| (row, at, bounds)))?;
        let (last_row, last_at, last_bounds) = last;
        if row < shown.start {
            let each = if shown.start == 0 {
                0.0
            } else {
                first / shown.start as f64
            };
            top * each
        } else {
            let below = last_at + f64::from(f32::from(last_bounds.size.height));
            let each = below / (last_row + 1) as f64;
            below + (top - (last_row + 1) as f64) * each
        }
    };
    Some((-y.max(0.0)) as f32)
}

/// Where the window is between being opened and having something to
/// show.
enum State {
    Reading,
    Ready,
    Refused(Refusal),
}

/// The view inside the window.
struct CompareView {
    /// Where the keyboard lands until a pane is clicked — without one,
    /// ⌘W reaches nothing. See the same note on `SettingsView`.
    focus: FocusHandle,
    state: State,
    /// What the window was opened on, until it has been read.
    subject: Option<Subject>,
    /// What to call it: the titlebar's name.
    name: String,
    /// The original as it was read, and what every comparison is
    /// against. Shared with the background task that compares.
    original_text: Arc<str>,
    /// What Layer A made of the original, cleaned with the read on the
    /// background executor: what the result opens with and what Reset
    /// puts back, so neither cleans on the thread that draws.
    cleaned_text: Arc<str>,
    /// Whether the result has been edited away from `cleaned_text`, as
    /// of the latest comparison — what Reset is offered on.
    edited: bool,
    /// Whether the result is a clean's or a rewrite's — what the banner
    /// and Reset say.
    kind: MadeKind,
    /// The left pane: the same editor as the right, read-only — which
    /// still focuses, selects, copies and searches, by mouse and by
    /// key, and refuses every change a person makes. Not *disabled*:
    /// in this library a disabled field swallows every mouse-down, so
    /// the pane could be neither selected nor scrolled by its bar
    /// (`the_original_selects_with_the_mouse`).
    original: Entity<EditorState>,
    /// The original's line marks, once a comparison has put any there:
    /// `None` until then, so an editor that has compared nothing yet
    /// carries no collection at all — the library reserves the marker
    /// slot in the gutter only while one has a provider.
    original_marks: Option<LineDecorationCollection>,
    /// The right pane, toolbar and all.
    result: Entity<ResultEditor>,
    /// What the window was opened with — see [`Comparison`].
    comparison: Comparison,
    /// The latest comparison, which the cursor is followed by.
    diff: Diff,
    /// Which recomputation is the current one; an older one that
    /// finishes late is thrown away.
    generation: u64,
    /// The result's cursor row as last followed.
    followed: u32,
    /// The original's scrolling, as the window last saw it.
    original_track: Track,
    /// The result's scrolling, as the window last saw it.
    result_track: Track,
    /// Dropped with the view.
    _subscriptions: Vec<Subscription>,
}

impl CompareView {
    fn new(
        subject: Subject,
        comparison: Comparison,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        focus.focus(window, cx);

        let original = cx.new(|cx| {
            // The same editor as the result's, for the same reasons —
            // see `ResultEditor::new`.
            EditorState::new(window, cx)
                .language("text")
                .line_number(true)
                .folding(false)
                .soft_wrap(false)
        });
        let result = cx.new(|cx| ResultEditor::new("", window, cx));

        let changed = cx.subscribe_in(
            &result,
            window,
            |view, _, event: &ResultEvent, window, cx| {
                let ResultEvent::Changed = event;
                view.recompute(window, cx);
            },
        );
        let result_state = result.read(cx).state().clone();
        // Everything the result's editor says: a scroll, which the
        // original is scrolled after, then a cursor that changed line,
        // which the original's cursor follows — in that order, so the
        // follow has the last word on the original (D383).
        let led_by_result = cx.observe_in(&result_state, window, |view, state, window, cx| {
            view.look(false, cx);
            let row = state.read(cx).cursor_position().line;
            if row != view.followed {
                view.followed = row;
                view.follow(row, window, cx);
            }
        });
        let led_by_original = cx.observe_in(&original, window, |view, _, _, cx| {
            view.look(false, cx);
        });

        let mut view = Self {
            focus,
            state: State::Reading,
            subject: Some(subject),
            name: String::new(),
            original_text: Arc::from(""),
            cleaned_text: Arc::from(""),
            edited: false,
            kind: MadeKind::Cleaned,
            original,
            original_marks: None,
            result,
            comparison,
            diff: Diff::of("", ""),
            generation: 0,
            followed: 0,
            original_track: Track::default(),
            result_track: Track::default(),
            _subscriptions: vec![changed, led_by_result, led_by_original],
        };
        view.read(window, cx);
        view
    }

    /// Read the subject and clean it on the background executor, then
    /// fill both panes.
    fn read(&mut self, window: &Window, cx: &Context<Self>) {
        let Some(subject) = self.subject.take() else {
            return;
        };
        cx.spawn_in(window, async move |view, cx| {
            let loaded = cx
                .background_executor()
                .spawn(async move { subject.read() })
                .await;
            view.update_in(cx, |view, window, cx| view.loaded(loaded, window, cx))
                .ok();
        })
        .detach();
    }

    /// The text arrived, or a reason it did not.
    fn loaded(
        &mut self,
        loaded: Result<Loaded, Refusal>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match loaded {
            Ok(Loaded {
                name,
                text,
                cleaned,
                kind,
            }) => {
                self.kind = kind;
                window.set_window_title(&Title::Compare { name: &name }.text());
                self.name = name;
                self.original_text = Arc::from(text.as_str());
                self.cleaned_text = Arc::from(cleaned.as_str());
                // The finer marks, if the page asks for any — installed
                // before the texts go in, so the first question the
                // library puts (it puts one when a text is set) already
                // has somewhere to go.
                if self.comparison.grain != Grain::Lines {
                    let result_state = self.result.read(cx).state().clone();
                    let inline = |side| -> Rc<dyn DocumentColorProvider> {
                        Rc::new(Inline::new(
                            side,
                            self.comparison.grain,
                            self.original_text.clone(),
                            result_state.clone(),
                        ))
                    };
                    let removed = inline(Side::Original);
                    let added = inline(Side::Result);
                    self.original.update(cx, |original, _| {
                        original.lsp_mut().document_color_provider = Some(removed);
                    });
                    self.result
                        .update(cx, |result, cx| result.colour(Some(added), cx));
                }
                self.original.update(cx, |original, cx| {
                    original.set_value(text, window, cx);
                });
                // The cleaned text, not a copy of the original. Announces
                // `Changed`, which is what recomputes the marks.
                self.result.update(cx, |result, cx| {
                    result.set_text(&cleaned, window, cx);
                });
                self.state = State::Ready;
                // The keyboard goes to the side being written in.
                self.result
                    .update(cx, |result, cx| result.focus(window, cx));
            }
            Err(refusal) => {
                tracing::info!(?refusal, "nothing to compare");
                self.state = State::Refused(refusal);
            }
        }
        cx.notify();
    }

    /// Put the result back to what cleaning made of the original — the
    /// text kept from the read, so nothing is cleaned here, on the
    /// thread that draws. The editor forgets its history with it: this
    /// is a new document, not an edit.
    fn reset(&self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.cleaned_text.to_string();
        self.result.update(cx, |result, cx| {
            result.set_text(&text, window, cx);
            result.focus(window, cx);
        });
    }

    /// Compare again, once the typing has settled, off the foreground
    /// thread — and only apply the answer if nothing changed meanwhile.
    fn recompute(&mut self, window: &Window, cx: &Context<Self>) {
        self.generation += 1;
        let mine = self.generation;
        cx.spawn_in(window, async move |view, cx| {
            cx.background_executor().timer(SETTLE).await;
            let texts = view
                .update(cx, |view, cx| {
                    (view.generation == mine).then(|| {
                        (
                            view.original_text.clone(),
                            view.cleaned_text.clone(),
                            view.result.read(cx).text(cx).to_string(),
                        )
                    })
                })
                .ok()
                .flatten();
            let Some((original, cleaned, result)) = texts else {
                return;
            };
            let (diff, edited) = cx
                .background_executor()
                .spawn(async move { (Diff::of(&original, &result), *cleaned != *result) })
                .await;
            view.update_in(cx, |view, window, cx| {
                if view.generation != mine {
                    return;
                }
                view.edited = edited;
                view.apply(diff, window, cx);
            })
            .ok();
        })
        .detach();
    }

    /// A fresh comparison: hand each side its marks, and put the finer
    /// question to the original again where the answer could have
    /// moved.
    fn apply(&mut self, diff: Diff, window: &mut Window, cx: &mut Context<Self>) {
        let removed: Rc<dyn LineDecorationProvider> =
            Rc::new(Marks::new(diff.removed_rows(), Side::Original));
        let added: Rc<dyn LineDecorationProvider> =
            Rc::new(Marks::new(diff.added_rows(), Side::Result));
        match &self.original_marks {
            Some(marks) => marks.set_provider(removed, cx),
            None => {
                self.original_marks = Some(self.original.update(cx, |original, cx| {
                    original.create_line_decorations_collection(removed, cx)
                }));
            }
        }
        self.result
            .update(cx, |result, cx| result.decorate(Some(added), cx));
        // Only where a changed passage is or was: with none on either
        // side the finer answer is empty both times, and the original
        // is spared the edit — and its selection the collapse.
        let moved = self.diff.has_changed_passages() || diff.has_changed_passages();
        self.diff = diff;
        if self.comparison.grain != Grain::Lines && moved {
            self.repaint_original(window, cx);
        }
        cx.notify();
    }

    /// Put the finer question to the original's editor again.
    ///
    /// The library asks a document-colour provider only when the text
    /// it is over changes, and the original's never does — it is the
    /// other side that moves. The one public road to that question is
    /// an edit, so this is an edit of nothing at the cursor: the text
    /// is untouched, the editor is read-only so its history is nobody's
    /// and nothing listens to its changes, and what it costs is a
    /// selection in the original, which collapses to its end. See the
    /// module docs, and `the_original_is_asked_again_when_the_marks_move`.
    ///
    /// The collapsed selection is a caret the editor would bring into
    /// view at the next frame — a scroll nobody asked for, which would
    /// pull the original away and, with the sides scrolling together,
    /// the result after it, out from under the person typing there. So
    /// the original's offset is put back as it stood, which replaces
    /// that scroll before it is drawn: a recompute moves neither pane
    /// (D390).
    fn repaint_original(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.original.update(cx, |original, cx| {
            let stood = original.scroll_offset();
            original.insert("", window, cx);
            original.set_scroll_offset(stood, cx);
        });
    }

    /// Put the original's cursor on the line that stands where the
    /// result's row does, so the original scrolls to it — and hand the
    /// keyboard straight back to whoever had it. See the module docs.
    fn follow(&mut self, row: u32, window: &mut Window, cx: &mut Context<Self>) {
        if !matches!(self.state, State::Ready) || !self.comparison.follow {
            return;
        }
        let target = self.diff.original_row_of(row as usize);
        let had_focus = window.focused(cx);
        self.original.update(cx, |original, cx| {
            original.set_cursor_position(
                Position {
                    line: u32::try_from(target).unwrap_or(u32::MAX),
                    character: 0,
                },
                window,
                cx,
            );
        });
        if let Some(handle) = had_focus {
            window.focus(&handle, cx);
        }
        // With the sides scrolling together, the caret just placed does
        // not get to scroll the original by itself: the original stands
        // where the result's top puts it, which is where the caret's
        // line is in view anyway, and the result is never moved by a
        // follow (D383). The column goes back to the start, where the
        // caret was put.
        if self.comparison.sync_scroll {
            if let Some(top) = self.top_of(Side::Result, cx) {
                let there = self.diff.position_across(Side::Result, top);
                self.drive(Side::Original, there, Some(px(0.0)), cx);
            }
        }
    }

    /// The editor on `side`.
    fn editor(&self, side: Side, cx: &App) -> Entity<EditorState> {
        match side {
            Side::Original => self.original.clone(),
            Side::Result => self.result.read(cx).state().clone(),
        }
    }

    /// Whether `side` wraps its lines — the original never does; the
    /// result does while its toolbar says so.
    fn wraps(&self, side: Side, cx: &App) -> bool {
        match side {
            Side::Original => false,
            Side::Result => self.result.read(cx).shows(result::View::SoftWrap),
        }
    }

    fn track(&mut self, side: Side) -> &mut Track {
        match side {
            Side::Original => &mut self.original_track,
            Side::Result => &mut self.result_track,
        }
    }

    /// Where `side`'s viewport stands, as a row and a fraction of the
    /// next — see [`top_of`].
    fn top_of(&self, side: Side, cx: &App) -> Option<f64> {
        top_of(self.editor(side, cx).read(cx), self.wraps(side, cx))
    }

    /// Whether `side` has scrolled since the window last looked, and if
    /// it has — and the scroll is not the landing of one this window
    /// asked for — scroll the other side after it. See the module docs,
    /// "The two sides stay in step".
    ///
    /// Asked from two places. An editor's notification: a wheel and a
    /// drag on the scroll bar arrive with one at once, and a scroll the
    /// library applies while it lays the text out — the keyboard's, a
    /// caret brought into view, the other side's lead landing — with
    /// the one it makes after a frame in which its text moved. And the
    /// end of every frame the panes are painted in (`settled`), the one
    /// moment a *wrapping* pane's layout and offset are known to agree:
    /// a wheel's notification comes before the frame that lays the
    /// text out at the new offset, and a wrapped pane's top is read off
    /// that layout, so a wrapping pane is read only there. For a pane
    /// that does not wrap, the end of a frame is a second look that
    /// finds nothing new.
    fn scrolled(&mut self, side: Side, settled: bool, cx: &mut Context<Self>) -> bool {
        if !settled && self.wraps(side, cx) {
            return false;
        }
        let y = f32::from(self.editor(side, cx).read(cx).scroll_offset().y);
        let track = self.track(side);
        if track.seen == Some(y) {
            // A blink, an edit, a selection: anything but a scroll.
            return false;
        }
        track.seen = Some(y);
        if track.asked.take().is_some_and(|asked| asked.lands(y)) {
            // The other side's lead, arriving: not a lead of its own.
            return false;
        }
        if !matches!(self.state, State::Ready) || !self.comparison.sync_scroll {
            return false;
        }
        let Some(top) = self.top_of(side, cx) else {
            return false;
        };
        #[cfg(test)]
        {
            self.track(side).leads += 1;
        }
        let there = self.diff.position_across(side, top);
        self.drive(side.other(), there, None, cx);
        true
    }

    /// Look at both panes, the result first: if the result led, the
    /// original's own move in the same look is only recorded, never a
    /// lead of its own — two panes that moved in one frame end where
    /// the result put them, at once and without a flicker (D383, D392).
    /// The editors' notifications and the end of a frame all come here,
    /// so whichever pane happened to notify first, the result is asked
    /// first.
    fn look(&mut self, settled: bool, cx: &mut Context<Self>) {
        if self.scrolled(Side::Result, settled, cx) {
            self.record(Side::Original, settled, cx);
        } else if self.scrolled(Side::Original, settled, cx) {
            self.record(Side::Result, settled, cx);
        }
    }

    /// Take note of where `side` stands without letting it lead: the
    /// side the other one has just led. Its new ask stays, to be
    /// matched when it lands.
    fn record(&mut self, side: Side, settled: bool, cx: &App) {
        if !settled && self.wraps(side, cx) {
            return;
        }
        let y = f32::from(self.editor(side, cx).read(cx).scroll_offset().y);
        self.track(side).seen = Some(y);
    }

    /// The end of a frame the panes were painted in: look at both, then
    /// drop every ask that has had its frame and moved nothing — the
    /// pane was already where it was asked to go, or at its end. Kept,
    /// such an ask would take the pane's next real move in its
    /// direction for its landing; a pane's end moves when lines are
    /// typed at the bottom, so that next move does come (D391).
    fn painted(&mut self, cx: &mut Context<Self>) {
        self.look(true, cx);
        for side in [Side::Result, Side::Original] {
            let y = f32::from(self.editor(side, cx).read(cx).scroll_offset().y);
            let track = self.track(side);
            if let Some(asked) = track.asked.as_mut() {
                if !asked.painted {
                    asked.painted = true;
                } else if (y - asked.from).abs() < HALF_PIXEL {
                    track.asked = None;
                }
            }
        }
    }

    /// Scroll `side` so that `top` — a row and a fraction — stands at
    /// the top of its viewport, and remember what was asked so that its
    /// landing is not taken for a lead.
    ///
    /// The pane keeps its own horizontal scroll unless `x` is given;
    /// given, the offset is set even where it would not move, so that
    /// it replaces whatever scroll the library had already queued for
    /// the pane — the caret's own, after a follow (D383). A landing of
    /// an earlier ask that the window has not looked at yet is taken
    /// as landed first, so the new ask is measured from where the pane
    /// now is and the old one is not mistaken for the pane's own move.
    fn drive(&mut self, side: Side, top: f64, x: Option<Pixels>, cx: &mut Context<Self>) {
        let editor = self.editor(side, cx);
        let wraps = self.wraps(side, cx);
        let (now, to) = {
            let state = editor.read(cx);
            let Some(to) = offset_for(state, wraps, top) else {
                return;
            };
            (state.scroll_offset(), to)
        };
        let from = f32::from(now.y);
        let track = self.track(side);
        if track.seen != Some(from) && track.asked.is_some_and(|asked| asked.lands(from)) {
            track.seen = Some(from);
        }
        if x.is_none() && (to - from).abs() < HALF_PIXEL {
            return;
        }
        track.asked = Some(Asked {
            from,
            to,
            painted: false,
        });
        editor.update(cx, |state, cx| {
            state.set_scroll_offset(gpui::point(x.unwrap_or(now.x), px(to)), cx);
        });
    }

    /// The line under the panes: the same text, or how far apart they
    /// are.
    fn summary(&self) -> String {
        if self.diff.is_same() {
            return t(Message::CompareSame);
        }
        t_args(
            Message::CompareChanged,
            &args!("added" => self.diff.added(), "removed" => self.diff.removed()),
        )
    }

    /// A pane's title strip.
    fn caption(message: Message, cx: &App) -> AnyElement {
        let theme = cx.theme();
        h_flex()
            .h(px(28.0))
            .flex_shrink_0()
            .items_center()
            .px_2()
            .border_b_1()
            .border_color(theme.border)
            .text_xs()
            .font_semibold()
            .text_color(theme.muted_foreground)
            .child(SharedString::from(t(message)))
            .into_any_element()
    }

    /// The two panes, side by side, with a handle between them.
    fn panes(&self, cx: &Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let left = v_flex()
            .size_full()
            .border_r_1()
            .border_color(theme.border)
            .child(Self::caption(Message::CompareOriginal, cx))
            // A blank strip the height of the result's toolbar. The two
            // sides are read across, line against line, and without it
            // every line on the left sits a toolbar higher than its
            // counterpart on the right — which reads as an offset in
            // the diff rather than as chrome.
            .child(
                div()
                    .h(TOOLBAR_HEIGHT)
                    .flex_shrink_0()
                    .border_b_1()
                    .border_color(theme.border),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .child(result::pane(&self.original, cx).readonly(true)),
            );
        let right = v_flex()
            .size_full()
            .child(Self::caption(Message::CompareResult, cx))
            .child(div().flex_1().min_h(px(0.0)).child(self.result.clone()));

        // Painted after both editors, so what it reads is what they
        // were just painted at — see `scrolled`. Read once the frame is
        // done rather than inside it: a scroll asked for while a frame
        // paints would be forgotten with the frame.
        let view = cx.entity().downgrade();
        let after = canvas(
            |_, _, _| (),
            move |_, _, _, cx| {
                cx.defer(move |cx| {
                    view.update(cx, |view, cx| view.painted(cx)).ok();
                });
            },
        )
        .absolute()
        .size_0();

        div()
            .size_full()
            .child(
                h_resizable("compare-split")
                    .child(resizable_panel().child(left))
                    .child(resizable_panel().child(right)),
            )
            .child(after)
            .into_any_element()
    }

    /// The middle of the window: reading, refused, or the panes.
    fn body(&self, cx: &Context<Self>) -> AnyElement {
        let theme = cx.theme();
        match &self.state {
            State::Reading => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(SharedString::from(t(Message::CompareReading)))
                .into_any_element(),
            State::Refused(refusal) => h_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .gap_2()
                .text_sm()
                .text_color(theme.warning)
                .child(
                    Icon::new(IconName::TriangleExclamation)
                        .small()
                        .color(theme.warning),
                )
                .child(SharedString::from(refusal.sentence()))
                .into_any_element(),
            State::Ready => div()
                .flex_1()
                .min_h(px(0.0))
                .child(self.panes(cx))
                .into_any_element(),
        }
    }
}

/// What this window does, since two panes of text do not say — as it
/// was opened, so a line about following is there only while the
/// original follows.
/// The banner's lines: what the result pane is, and — for a rewrite whose
/// paragraphs did not all pass — how many kept their cleaned original. A
/// rewrite is never called better here: it is the most diverged candidate
/// that passed the checks, and that is all the banner says of it.
pub fn banner_lines(kind: MadeKind) -> Vec<String> {
    match kind {
        MadeKind::Cleaned => vec![t(Message::ComparePending)],
        MadeKind::Rewritten { kept } => {
            let mut lines = vec![t(Message::CompareRewrittenBanner)];
            if let Some((kept, chunks)) = kept.filter(|(kept, _)| *kept > 0) {
                lines.push(t_args(
                    Message::CompareRewrittenKept,
                    &args!("kept" => kept, "chunks" => chunks),
                ));
            }
            lines
        }
    }
}

fn help(comparison: Comparison, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let mut lines = vec![Message::CompareHelpMarks];
    match comparison.grain {
        Grain::Lines => {}
        Grain::Words => lines.push(Message::CompareHelpWords),
        Grain::Characters => lines.push(Message::CompareHelpCharacters),
    }
    if comparison.follow {
        lines.push(Message::CompareHelpFollows);
    }
    if comparison.sync_scroll {
        lines.push(Message::CompareHelpScrolls);
    }
    lines.extend([
        Message::CompareHelpToolbar,
        Message::CompareHelpSettings,
        Message::CompareHelpClose,
    ]);
    v_flex()
        .w(px(300.0))
        .gap_1()
        .text_xs()
        .text_color(theme.muted_foreground)
        .children(
            lines
                .into_iter()
                .map(|line| div().child(SharedString::from(t(line)))),
        )
}

impl Focusable for CompareView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for CompareView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let ready = matches!(self.state, State::Ready);
        let comparison = self.comparison;

        v_flex()
            .id("compare")
            .track_focus(&self.focus)
            .key_context(CONTEXT)
            .on_action(cx.listener(|_, _: &CloseCompare, window, _| {
                window.remove_window();
            }))
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            // The banner: what the result is, and that nothing here is
            // saved or written — and the one control that is the
            // window's rather than the editor's.
            .child(
                h_flex()
                    .w_full()
                    .flex_shrink_0()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(Icon::new(IconName::CircleInfo).small().color(muted))
                    .child(
                        v_flex()
                            .flex_1()
                            .text_xs()
                            .text_color(muted)
                            .children(banner_lines(self.kind).into_iter().map(SharedString::from)),
                    )
                    .child(
                        Button::new("compare-reset")
                            .small()
                            .outline()
                            .icon(IconName::RotateLeft)
                            .label(SharedString::from(t(match self.kind {
                                MadeKind::Cleaned => Message::CompareReset,
                                MadeKind::Rewritten { .. } => Message::CompareResetRewritten,
                            })))
                            .tooltip(SharedString::from(t(match self.kind {
                                MadeKind::Cleaned => Message::CompareResetTooltip,
                                MadeKind::Rewritten { .. } => Message::CompareResetRewrittenTooltip,
                            })))
                            // Offered once the result has moved away
                            // from the cleaned text — not from the
                            // original, which it differs from whenever
                            // cleaning found anything.
                            .disabled(!ready || !self.edited)
                            .on_click(cx.listener(|view, _: &ClickEvent, window, cx| {
                                view.reset(window, cx);
                            })),
                    )
                    .child(
                        Popover::new("compare-help")
                            .anchor(Anchor::TopRight)
                            .trigger(
                                Button::new("compare-help-trigger")
                                    .small()
                                    .ghost()
                                    .icon(IconName::CircleQuestion)
                                    .tooltip(SharedString::from(t(Message::CompareHelp))),
                            )
                            .content(move |_, _, cx| help(comparison, cx)),
                    ),
            )
            .child(self.body(cx))
            .child(
                h_flex()
                    .w_full()
                    .flex_shrink_0()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1p5()
                    .border_t_1()
                    .border_color(theme.border)
                    .text_xs()
                    .text_color(muted)
                    .child(Icon::new(IconName::CodeCompare).small().color(muted))
                    .child(SharedString::from(if ready {
                        self.summary()
                    } else {
                        String::new()
                    })),
            )
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::path::PathBuf;

    use gpui::TestAppContext;
    use wipemark_intake::{Format, Kind};

    use super::*;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(label: &str) -> Self {
            let directory = std::env::temp_dir()
                .join(format!("wipemark-compare-{label}-{}", std::process::id()));
            std::fs::create_dir_all(&directory).expect("scratch directory");
            Self(directory)
        }

        fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.0.join(name);
            std::fs::write(&path, bytes).expect("scratch file");
            path
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).ok();
        }
    }

    fn of(handed: Handed) -> Subject {
        Subject {
            intake: Some(wipemark_intake::of(&handed)),
            handed,
            made: Made::Cleaned,
        }
    }

    /// The three roads in — characters, bytes, a file — all end as the
    /// same text, decoded the way the queue's Clean decodes it.
    #[test]
    fn text_arrives_the_same_way_by_every_road() {
        let scratch = Scratch::new("roads");
        let path = scratch.file("note.txt", "one\ntwo\n".as_bytes());
        let utf16 = scratch.file("wide.txt", &[0xff, 0xfe, b'h', 0, b'i', 0, b'\n', 0]);

        let typed = of(Handed::Text("one\ntwo\n".to_owned())).read().unwrap();
        let bytes = of(Handed::Bytes {
            name: None,
            bytes: b"one\ntwo\n".to_vec(),
        })
        .read()
        .unwrap();
        let file = of(Handed::Path(path)).read().unwrap();
        assert_eq!(typed.text, "one\ntwo\n");
        assert_eq!(bytes.text, typed.text);
        assert_eq!(file.text, typed.text);
        assert_eq!(file.name, "note.txt", "a file is called by its name");

        // Decoded strictly, as the queue decodes it: the byte order mark is
        // the text's first character, and the result keeps it as the
        // queue's written file does.
        let wide = of(Handed::Path(utf16)).read().unwrap();
        assert_eq!(
            wide.text, "\u{FEFF}hi\n",
            "a UTF-16 file was not decoded as such"
        );
    }

    /// A path examined on the way in — the command line's road — reads
    /// the same as one the queue examined first.
    #[test]
    fn a_path_not_yet_examined_is_examined_on_the_way_in() {
        let scratch = Scratch::new("unexamined");
        let path = scratch.file("later.md", b"# Title\n");
        let loaded = Subject {
            handed: Handed::Path(path),
            intake: None,
            made: Made::Cleaned,
        }
        .read()
        .unwrap();
        assert_eq!(loaded.text, "# Title\n");
        assert_eq!(loaded.name, "later.md");
    }

    #[test]
    fn what_is_not_text_is_refused_by_name() {
        let scratch = Scratch::new("refusals");
        let png = scratch.file("p.png", b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR");
        assert_eq!(of(Handed::Path(png)).read(), Err(Refusal::NotText));

        let folder = scratch.0.clone();
        assert_eq!(of(Handed::Path(folder)).read(), Err(Refusal::NotText));

        let gone = Subject {
            handed: Handed::Path(scratch.0.join("nowhere.txt")),
            intake: None,
            made: Made::Cleaned,
        };
        assert!(
            matches!(gone.read(), Err(Refusal::Unreadable | Refusal::NotText)),
            "a path that leads nowhere was read"
        );

        let intake = Intake {
            kind: Kind::Text,
            format: Some(Format::PlainText),
            ..wipemark_intake::of_text("")
        };
        assert!(Subject::comparable(&intake));
        assert!(!Subject::comparable(&wipemark_intake::of(&Handed::Bytes {
            name: Some("p.png".to_owned()),
            bytes: b"\x89PNG\r\n\x1a\n".to_vec(),
        })));
    }

    /// The ceiling is checked before the read, on the size, so the file
    /// past it is never loaded to be refused.
    #[test]
    fn a_text_past_the_limit_is_refused_with_its_size() {
        let big = "x".repeat(TEXT_LIMIT as usize + 1);
        let refused = of(Handed::Text(big)).read();
        assert_eq!(
            refused,
            Err(Refusal::TooBig {
                size: TEXT_LIMIT + 1
            })
        );
        let sentence = Refusal::TooBig {
            size: TEXT_LIMIT + 1,
        }
        .sentence();
        assert!(
            sentence.contains(&drop::size_label(TEXT_LIMIT)),
            "the refusal did not name the limit: {sentence}"
        );
    }

    /// A span is answered one line at a time, in characters and not
    /// bytes, with the newline left out — and a span that was only a
    /// newline is not answered at all.
    #[test]
    fn finer_marks_never_reach_across_a_line() {
        let text = "ab\ncd\n";
        let ranges = per_line(text, &[1..4, 9..9]);
        assert_eq!(
            ranges,
            vec![
                lsp_types::Range::new(Position::new(0, 1), Position::new(0, 2)),
                lsp_types::Range::new(Position::new(1, 0), Position::new(1, 1)),
            ]
        );
        assert!(
            per_line(text, &[2..3, 5..6]).is_empty(),
            "a newline was painted"
        );
        assert!(per_line("", &[]).is_empty());

        // Columns count characters: the é is two bytes and one column.
        let ranges = per_line("éa", std::slice::from_ref(&(2..3)));
        assert_eq!(
            ranges,
            vec![lsp_types::Range::new(
                Position::new(0, 1),
                Position::new(0, 2)
            )]
        );

        // The last line has no newline to stop at.
        let ranges = per_line("x\nyz", std::slice::from_ref(&(2..4)));
        assert_eq!(
            ranges,
            vec![lsp_types::Range::new(
                Position::new(1, 0),
                Position::new(1, 2)
            )]
        );
    }

    /// A window on `text`, read and ready, the way `open` builds one —
    /// a `Root` first, for the reason `result::tests::window_with`
    /// gives.
    fn window_with<'a>(
        cx: &'a mut TestAppContext,
        text: &str,
        comparison: Comparison,
    ) -> (Entity<CompareView>, &'a mut gpui::VisualTestContext) {
        window_on(cx, of(Handed::Text(text.to_owned())), comparison)
    }

    /// A window on `subject`, read and ready — any road in, a file
    /// included.
    fn window_on(
        cx: &mut TestAppContext,
        subject: Subject,
        comparison: Comparison,
    ) -> (Entity<CompareView>, &mut gpui::VisualTestContext) {
        cx.update(gpui_component::init);
        let slot: Rc<std::cell::RefCell<Option<Entity<CompareView>>>> = Rc::default();
        let held = slot.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| CompareView::new(subject, comparison, window, cx));
            *held.borrow_mut() = Some(view.clone());
            Root::new(view, window, cx)
        });
        // The read, on the background executor, and the frame after.
        cx.run_until_parked();
        let view = slot.take().expect("the window builder ran");
        assert!(
            cx.update(|_, cx| matches!(view.read(cx).state, State::Ready)),
            "the text was not read"
        );
        (view, cx)
    }

    /// Let every timer in the window fire — the settle before a
    /// comparison, and the library's own before it asks a provider —
    /// and everything they started run.
    fn settle(cx: &gpui::VisualTestContext) {
        for _ in 0..3 {
            cx.executor().advance_clock(Duration::from_secs(1));
            cx.run_until_parked();
        }
    }

    /// A provider that only counts how often it is asked.
    struct Counting(Rc<Cell<u32>>);

    impl DocumentColorProvider for Counting {
        fn document_colors(
            &self,
            _: &Rope,
            _: &mut Window,
            _: &mut App,
        ) -> Task<anyhow::Result<Vec<ColorInformation>>> {
            self.0.set(self.0.get() + 1);
            Task::ready(Ok(Vec::new()))
        }
    }

    /// Each side's provider answers for its own side, over the two
    /// texts as they stand when it is asked: the original marks the
    /// word the result no longer has, the result the word the original
    /// never had, and neither marks the rest of the line.
    #[gpui::test]
    async fn the_finer_marks_are_answered_for_each_side(cx: &mut TestAppContext) {
        let (view, cx) = window_with(cx, "the cat sat\non the mat\n", Comparison::default());
        let (original, result) = cx.update(|_, cx| {
            let view = view.read(cx);
            (view.original.clone(), view.result.clone())
        });
        cx.update(|window, cx| {
            result.update(cx, |result, cx| {
                result.set_text("the dog sat\non the mat\n", window, cx)
            });
        });
        settle(cx);

        let (removed, added) = cx.update(|_, cx| {
            (
                original.read(cx).lsp().document_color_provider.clone(),
                result
                    .read(cx)
                    .state()
                    .read(cx)
                    .lsp()
                    .document_color_provider
                    .clone(),
            )
        });
        let removed = removed.expect("the original was given a provider");
        let added = added.expect("the result was given a provider");

        let cat = Position::new(0, 4)..Position::new(0, 7);
        let answered = cx
            .update(|window, cx| {
                removed.document_colors(&Rope::from("the cat sat\non the mat\n"), window, cx)
            })
            .await
            .expect("answered");
        assert_eq!(answered.len(), 1, "the original marked more than the word");
        assert_eq!(answered[0].range, lsp_types::Range::new(cat.start, cat.end));

        let answered = cx
            .update(|window, cx| {
                added.document_colors(&Rope::from("the dog sat\non the mat\n"), window, cx)
            })
            .await
            .expect("answered");
        assert_eq!(answered.len(), 1, "the result marked more than the word");
        assert_eq!(answered[0].range, lsp_types::Range::new(cat.start, cat.end));
        assert_eq!(answered[0].color.alpha, 1.0, "the tint is not opaque");
    }

    /// The road to the original's finer marks, and its economy. The
    /// library asks the original's provider only on an edit; after a
    /// comparison that could have moved the marks the window makes one
    /// of nothing, and after one that could not — no changed passage
    /// before or after — it does not. Without the empty edit in
    /// `repaint_original` the first count stays at nought and this
    /// goes red.
    #[gpui::test]
    fn the_original_is_asked_again_when_the_marks_move(cx: &mut TestAppContext) {
        let (view, cx) = window_with(cx, "the cat sat\n", Comparison::default());
        let (original, result) = cx.update(|_, cx| {
            let view = view.read(cx);
            (view.original.clone(), view.result.clone())
        });
        settle(cx);

        let asked = Rc::new(Cell::new(0u32));
        let counting: Rc<dyn DocumentColorProvider> = Rc::new(Counting(asked.clone()));
        cx.update(|_, cx| {
            original.update(cx, |original, _| {
                original.lsp_mut().document_color_provider = Some(counting);
            });
        });

        // A line put in: no passage changed, nothing finer to move.
        cx.update(|window, cx| {
            result.update(cx, |result, cx| {
                result.set_text("the cat sat\nand purred\n", window, cx)
            });
        });
        settle(cx);
        assert_eq!(asked.get(), 0, "the original was asked for nothing new");

        // A word changed: the original's marks could have moved.
        cx.update(|window, cx| {
            result.update(cx, |result, cx| {
                result.set_text("the dog sat\nand purred\n", window, cx)
            });
        });
        settle(cx);
        assert_eq!(asked.get(), 1, "the original was not asked again");
        assert_eq!(
            cx.update(|_, cx| original.read(cx).value()).as_ref(),
            "the cat sat\n",
            "the empty edit was not empty"
        );
    }

    /// The two sides are read across, line against line: the first
    /// line of the original sits level with the first line of the
    /// result. The blank strip over the original is what does it —
    /// take it out and the original starts a toolbar higher, and this
    /// goes red.
    #[gpui::test]
    fn the_first_lines_sit_level(cx: &mut TestAppContext) {
        let (view, cx) = window_with(cx, "one\ntwo\n", Comparison::default());
        settle(cx);
        let (left, right) = cx.update(|_, cx| {
            let view = view.read(cx);
            (
                view.original.read(cx).row_bounds(0),
                view.result.read(cx).state().read(cx).row_bounds(0),
            )
        });
        let left = left.expect("the original was painted");
        let right = right.expect("the result was painted");
        assert_eq!(
            left.origin.y, right.origin.y,
            "the original's first line is not level with the result's"
        );
    }

    /// A comparison puts the line marks on both sides — the library's
    /// line-decoration collection, one per editor, with a provider in
    /// it — and the window holds the original's, so the next comparison
    /// replaces what it is asked rather than stacking a second one. Take
    /// out either `create_line_decorations_collection` and this goes red.
    #[gpui::test]
    fn a_comparison_puts_line_marks_on_both_sides(cx: &mut TestAppContext) {
        let (view, cx) = window_with(cx, "one\ntwo\n", Comparison::default());
        cx.update(|window, cx| {
            let result = view.read(cx).result.clone();
            result.update(cx, |result, cx| result.set_text("one\n2\n", window, cx));
        });
        settle(cx);
        cx.update(|_, cx| {
            let view = view.read(cx);
            assert!(
                view.original_marks
                    .as_ref()
                    .is_some_and(|marks| marks.has_provider(cx)),
                "the original has no line marks"
            );
            assert!(
                view.result.read(cx).is_decorated(cx),
                "the result has no line marks"
            );
            assert_eq!(view.diff.removed_rows(), vec![1]);
            assert_eq!(view.diff.added_rows(), vec![1]);
        });
    }

    /// Lines only: no provider is put on either side, so the library
    /// has nothing to ask and nothing to paint.
    #[gpui::test]
    fn lines_alone_put_no_provider_on_either_side(cx: &mut TestAppContext) {
        let comparison = Comparison {
            grain: Grain::Lines,
            ..Comparison::default()
        };
        let (view, cx) = window_with(cx, "a\n", comparison);
        cx.update(|_, cx| {
            let view = view.read(cx);
            assert!(view
                .original
                .read(cx)
                .lsp()
                .document_color_provider
                .is_none());
            assert!(view
                .result
                .read(cx)
                .state()
                .read(cx)
                .lsp()
                .document_color_provider
                .is_none());
        });
    }

    // -- the result is the cleaned text ---------------------------------

    /// What each pane holds, as it stands.
    fn panes_of(view: &Entity<CompareView>, cx: &mut gpui::VisualTestContext) -> (String, String) {
        cx.update(|_, cx| {
            let view = view.read(cx);
            (
                view.original.read(cx).value().to_string(),
                view.result.read(cx).text(cx).to_string(),
            )
        })
    }

    const MARKED: &str = "# Notes\n\nA zero\u{200B}width space.\n";

    /// Opened on a text with a zero-width space, the original keeps it
    /// and the result is what Layer A makes of it — the space gone, the
    /// line marked, and Reset not yet offered because nothing has been
    /// edited. Put the copy of the original back into `loaded` and this
    /// goes red. A text with nothing to clean still opens on itself.
    #[gpui::test]
    fn the_result_is_the_cleaned_text_and_the_original_is_not(cx: &mut TestAppContext) {
        let (view, cx) = window_with(cx, MARKED, Comparison::default());
        settle(cx);
        let (original, result) = panes_of(&view, cx);
        assert_eq!(original, MARKED, "the original is not what arrived");
        assert!(original.contains('\u{200B}'));
        assert!(
            !result.contains('\u{200B}'),
            "the result still carries the zero-width space: {result:?}"
        );
        assert_eq!(
            result,
            wipemark_core::clean(MARKED, &Options::default()).text,
            "the result is not Layer A at its defaults"
        );
        cx.update(|_, cx| {
            let view = view.read(cx);
            assert!(!view.diff.is_same(), "the cleaned line was not marked");
            assert_eq!(view.diff.removed(), 1);
            assert_eq!(view.diff.added(), 1);
            assert!(!view.edited, "Reset is offered on an untouched result");
        });
    }

    /// Nothing to clean: the result is the original, line for line, and
    /// the summary says so.
    #[gpui::test]
    fn a_text_with_nothing_to_clean_opens_on_itself(cx: &mut TestAppContext) {
        let plain = "# Notes\n\nNothing hidden here.\n";
        let (view, cx) = window_with(cx, plain, Comparison::default());
        settle(cx);
        let (original, result) = panes_of(&view, cx);
        assert_eq!(original, plain);
        assert_eq!(result, plain);
        let summary = cx.update(|_, cx| view.read(cx).summary());
        assert_eq!(summary, t(Message::CompareSame));
    }

    /// Reset throws the edits away and lands on the cleaned text — not
    /// on the original, which still has the zero-width space. Point
    /// `reset` back at the original and this goes red.
    #[gpui::test]
    fn reset_returns_to_the_cleaned_text_not_the_original(cx: &mut TestAppContext) {
        let (view, cx) = window_with(cx, MARKED, Comparison::default());
        settle(cx);
        let result = cx.update(|_, cx| view.read(cx).result.clone());
        cx.update(|window, cx| {
            result.update(cx, |result, cx| result.set_text("edited\n", window, cx));
        });
        settle(cx);
        assert!(
            cx.update(|_, cx| view.read(cx).edited),
            "an edit did not offer Reset"
        );

        cx.update(|window, cx| view.update(cx, |view, cx| view.reset(window, cx)));
        settle(cx);
        let (original, result) = panes_of(&view, cx);
        assert_eq!(
            result,
            wipemark_core::clean(MARKED, &Options::default()).text,
            "Reset did not return to the cleaned text"
        );
        assert!(
            !result.contains('\u{200B}'),
            "Reset returned to the original"
        );
        assert_eq!(original, MARKED, "Reset touched the original");
        assert!(
            !cx.update(|_, cx| view.read(cx).edited),
            "Reset is still offered after it ran"
        );
    }

    /// R5: a rewritten row's window opens on the rewrite as it was
    /// delivered — `the_result_is_what_the_queue_writes`'s twin — says how
    /// many paragraphs kept their original, and Reset returns to the
    /// rewrite, not to a clean of the original.
    #[gpui::test]
    fn reset_returns_to_the_rewrite_not_the_clean(cx: &mut TestAppContext) {
        let scratch = Scratch::new("rewritten");
        let rewrite = "# Notes\n\nA rewritten line, as it was delivered.\n";
        let delivered = scratch.file("notes.rewritten.md", rewrite.as_bytes());
        let subject = Subject {
            handed: Handed::Text(MARKED.to_owned()),
            intake: None,
            made: Made::Rewritten {
                from: RewriteFrom::File(delivered),
                kept: Some((1, 3)),
            },
        };
        let (view, cx) = window_on(cx, subject, Comparison::default());
        settle(cx);
        let (original, result) = panes_of(&view, cx);
        assert_eq!(original, MARKED);
        assert_eq!(result, rewrite, "the pane is not the delivered rewrite");
        let kind = cx.update(|_, cx| view.read(cx).kind);
        let lines = banner_lines(kind);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(
            lines[1].contains('1') && lines[1].contains('3'),
            "{lines:?}"
        );

        let editor = cx.update(|_, cx| view.read(cx).result.clone());
        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| editor.set_text("edited\n", window, cx));
        });
        settle(cx);
        cx.update(|window, cx| view.update(cx, |view, cx| view.reset(window, cx)));
        settle(cx);
        let (_, result) = panes_of(&view, cx);
        assert_eq!(result, rewrite, "Reset did not return to the rewrite");
        assert_ne!(
            result,
            wipemark_core::clean(MARKED, &Options::default()).text,
            "Reset returned to the clean"
        );
    }

    /// Compare refuses what the queue's Clean refuses, in the sentence the
    /// queue's row shows for the same file (D282): a file that is not
    /// valid UTF-8 past its head, and one in an eight-bit encoding intake
    /// does not name — where the lenient decode used to open both with
    /// replacement characters the queue would never write.
    #[gpui::test]
    fn what_the_queue_will_not_decode_is_refused_in_its_words(cx: &mut TestAppContext) {
        use wipemark_intake::Encoding;

        use crate::clean::{clean_one, Verdict};
        use crate::drop::Arrival;
        use crate::retention::{self, Homes, Retention, Source};

        cx.update(gpui_component::init);
        let scratch = Scratch::new("strict");
        let homes = Homes {
            results: scratch.0.join("results"),
            kept: scratch.0.join("kept"),
        };
        let mut invalid = MARKED.as_bytes().to_vec();
        invalid.extend_from_slice(&[b'a'; wipemark_intake::HEAD]);
        invalid.extend_from_slice(b"\xff\xfe tail\n");
        let latin1 = b"Caf\xe9 au lait, cr\xe8me br\xfbl\xe9e.\n".to_vec();
        for (name, bytes, encoding) in [
            ("invalid.md", invalid, Encoding::Utf8),
            ("latin1.txt", latin1, Encoding::Other),
        ] {
            let source = scratch.file(name, &bytes);
            let handed = Handed::Path(source.clone());
            let arrival = Arrival {
                intake: wipemark_intake::of(&handed),
                handed,
            };
            assert_eq!(arrival.intake.encoding, Some(encoding), "{name}");
            let plan = retention::plan(&Source::of(&arrival.intake), &Retention::default(), &homes);
            let outcome = clean_one(&arrival, &plan, 1, chrono::Utc::now());
            assert!(
                matches!(outcome.verdict, Verdict::NotCleaned(_)),
                "{name}: {:?}",
                outcome.verdict
            );
            let row_says = wording::said_in(&wording::window, &outcome);

            let slot: Rc<std::cell::RefCell<Option<Entity<CompareView>>>> = Rc::default();
            let held = slot.clone();
            let subject = of(Handed::Path(source));
            let (_, window) = cx.add_window_view(move |window, cx| {
                let view =
                    cx.new(|cx| CompareView::new(subject, Comparison::default(), window, cx));
                *held.borrow_mut() = Some(view.clone());
                Root::new(view, window, cx)
            });
            window.run_until_parked();
            let view = slot.take().expect("the window builder ran");
            let shown = window.update(|_, cx| match &view.read(cx).state {
                State::Refused(refusal) => refusal.sentence(),
                _ => panic!("{name}: opened what the queue will not decode"),
            });
            assert_eq!(shown, row_says, "{name}");
        }
    }

    /// Layer A is deterministic, so the window's result is the text the
    /// queue's Clean writes beside the same file — the file cleaned
    /// through `clean::clean_one` under the default plan, then opened
    /// here by its path, in UTF-8 and in UTF-16LE with its mark.
    #[gpui::test]
    fn the_result_is_what_the_queue_writes(cx: &mut TestAppContext) {
        use wipemark_intake::Encoding;

        use crate::clean::{clean_one, Verdict};
        use crate::drop::Arrival;
        use crate::retention::{self, Homes, Plan, Retention, Source, Written};

        let scratch = Scratch::new("queue-writes");
        let homes = Homes {
            results: scratch.0.join("results"),
            kept: scratch.0.join("kept"),
        };
        let mut sources = Vec::new();
        for (name, encoding) in [("utf8.md", Encoding::Utf8), ("utf16.md", Encoding::Utf16Le)] {
            let text = match encoding {
                Encoding::Utf8 => MARKED.to_owned(),
                _ => format!("\u{FEFF}{MARKED}"),
            };
            let source = scratch.file(name, &wipemark_intake::text::encode(&text, encoding));
            let handed = Handed::Path(source.clone());
            let arrival = Arrival {
                intake: wipemark_intake::of(&handed),
                handed,
            };
            let plan = retention::plan(&Source::of(&arrival.intake), &Retention::default(), &homes);
            assert!(
                matches!(&plan, Plan::File(Written::Beside(_))),
                "the default plan is not beside the file: {plan:?}"
            );
            let outcome = clean_one(&arrival, &plan, 1, chrono::Utc::now());
            assert!(
                matches!(outcome.verdict, Verdict::Cleaned),
                "{name}: {:?}",
                outcome.verdict
            );
            let written = outcome.written.expect("the queue wrote a result");
            let bytes = std::fs::read(&written).expect("the written result");
            let encoding = arrival.intake.encoding.expect("an encoding");
            let queue_wrote = wipemark_intake::text::decode(&bytes, encoding).expect("decodes");
            sources.push((source, queue_wrote));
        }

        for (source, queue_wrote) in sources {
            let (view, window) = window_on(cx, of(Handed::Path(source)), Comparison::default());
            settle(window);
            let (original, result) = panes_of(&view, window);
            assert!(original.contains('\u{200B}'));
            assert!(!result.contains('\u{200B}'));
            assert_eq!(
                result, queue_wrote,
                "the window shows a result the queue did not write"
            );
        }
    }

    /// Press the left button at `from`, drag to `to` and let go — a
    /// person's drag, through the window's own mouse events.
    fn drag(cx: &mut gpui::VisualTestContext, from: gpui::Point<Pixels>, to: gpui::Point<Pixels>) {
        use gpui::{Modifiers, MouseButton};
        cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
    }

    /// Two points across the middle of row 0 of the original, two
    /// fifths in and nine tenths in.
    fn across_the_original(
        view: &Entity<CompareView>,
        cx: &mut gpui::VisualTestContext,
    ) -> (gpui::Point<Pixels>, gpui::Point<Pixels>) {
        let row = cx.update(|_, cx| {
            view.read(cx)
                .original
                .read(cx)
                .row_bounds(0)
                .expect("the original painted its first row")
        });
        let y = row.origin.y + row.size.height / 2.0;
        (
            gpui::point(row.origin.x + row.size.width * 0.4, y),
            gpui::point(row.origin.x + row.size.width * 0.9, y),
        )
    }

    /// The original is read with the mouse as well as shown: a drag
    /// across a line selects some of it and puts the keyboard there, so
    /// it can be copied and searched. Built `.disabled(true)` — which in
    /// this library swallows every mouse-down over the field — the
    /// selection stays empty and this goes red.
    #[gpui::test]
    fn the_original_selects_with_the_mouse(cx: &mut TestAppContext) {
        let line = "word ".repeat(80);
        let (view, cx) = window_with(cx, &format!("{line}\n{line}\n"), Comparison::default());
        settle(cx);
        let (from, to) = across_the_original(&view, cx);
        drag(cx, from, to);

        let (selected, focused) = cx.update(|window, cx| {
            let original = view.read(cx).original.read(cx);
            (
                original.selected_range(),
                original.focus_handle(cx).is_focused(window),
            )
        });
        assert!(
            !selected.is_empty(),
            "a drag across the original selected nothing: {selected:?}"
        );
        assert!(focused, "a click on the original did not focus it");
    }

    /// And it is still the original: with the keyboard in it, nothing a
    /// person types, deletes, pastes, cuts or undoes changes a byte of
    /// it. The focus is asserted first, so the keystrokes cannot have
    /// gone somewhere else and passed for refused.
    #[gpui::test]
    fn typing_into_the_original_changes_nothing(cx: &mut TestAppContext) {
        let text = format!("{}\n", "word ".repeat(80));
        let (view, cx) = window_with(cx, &text, Comparison::default());
        settle(cx);
        let (from, to) = across_the_original(&view, cx);
        drag(cx, from, to);
        assert!(
            cx.update(|window, cx| view
                .read(cx)
                .original
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)),
            "the original never took the keyboard; the keystrokes would prove nothing"
        );

        cx.write_to_clipboard(gpui::ClipboardItem::new_string("pasted".to_owned()));
        cx.simulate_input("x");
        cx.simulate_keystrokes("backspace delete enter tab");
        cx.dispatch_action(gpui_component::input::Paste);
        cx.dispatch_action(gpui_component::input::Cut);
        cx.dispatch_action(gpui_component::input::Undo);
        cx.run_until_parked();

        let original = cx.update(|_, cx| view.read(cx).original.read(cx).value().to_string());
        assert_eq!(original, text, "a keystroke changed the original");
    }

    // -- the two sides scroll together ----------------------------------

    /// `n` numbered lines from `from`, each ending in a newline.
    fn numbered(from: usize, n: usize) -> String {
        (from..from + n).map(|i| format!("line {i}\n")).collect()
    }

    /// A window on `original` whose result has been edited to `result`,
    /// compared and painted, both panes at the top.
    fn window_over<'a>(
        cx: &'a mut TestAppContext,
        original: &str,
        result: &str,
        comparison: Comparison,
    ) -> (Entity<CompareView>, &'a mut gpui::VisualTestContext) {
        let (view, cx) = window_with(cx, original, comparison);
        cx.update(|window, cx| {
            let pane = view.read(cx).result.clone();
            pane.update(cx, |pane, cx| pane.set_text(result, window, cx));
        });
        settle(cx);
        (view, cx)
    }

    /// Each pane's vertical offset, in pixels — `(original, result)`.
    fn offsets(view: &Entity<CompareView>, cx: &mut gpui::VisualTestContext) -> (f32, f32) {
        cx.update(|_, cx| {
            let view = view.read(cx);
            (
                f32::from(view.original.read(cx).scroll_offset().y),
                f32::from(view.result.read(cx).state().read(cx).scroll_offset().y),
            )
        })
    }

    /// How many times each pane has led — `(original, result)`.
    fn leads(view: &Entity<CompareView>, cx: &mut gpui::VisualTestContext) -> (u32, u32) {
        cx.update(|_, cx| {
            let view = view.read(cx);
            (view.original_track.leads, view.result_track.leads)
        })
    }

    /// The line height both panes are laid out at.
    fn line_height(view: &Entity<CompareView>, cx: &mut gpui::VisualTestContext) -> f32 {
        cx.update(|_, cx| {
            let view = view.read(cx);
            let left = view.original.read(cx).line_height().expect("laid out");
            let right = view
                .result
                .read(cx)
                .state()
                .read(cx)
                .line_height()
                .expect("laid out");
            assert_eq!(left, right, "the two panes are not one line height");
            f32::from(left)
        })
    }

    /// Turn the wheel over one pane by `pixels` (negative scrolls down),
    /// the way a touchpad does, and let every frame it starts finish.
    fn wheel(
        view: &Entity<CompareView>,
        side: Side,
        pixels: f32,
        cx: &mut gpui::VisualTestContext,
    ) {
        let over = cx.update(|_, cx| {
            let editor = view.read(cx).editor(side, cx);
            editor.read(cx).input_bounds().center()
        });
        cx.simulate_event(gpui::ScrollWheelEvent {
            position: over,
            delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.0), px(pixels))),
            ..Default::default()
        });
        settle(cx);
    }

    /// Scrolling the result scrolls the original to the line that
    /// stands where the result's top line does — three rows further up,
    /// past three lines the result put in — and keeps the part of a row
    /// the result is scrolled into, so the two move by pixels and not a
    /// row at a time. Take `Side::Result` out of `scrolled`'s callers
    /// and the original stays at the top: red.
    #[gpui::test]
    fn scrolling_the_result_scrolls_the_original(cx: &mut TestAppContext) {
        let original = numbered(0, 200);
        let result = format!("{}{original}", numbered(1000, 3));
        let (view, cx) = window_over(cx, &original, &result, Comparison::default());
        let height = line_height(&view, cx);

        wheel(&view, Side::Result, -(10.25 * height), cx);
        let (left, right) = offsets(&view, cx);
        assert_eq!(
            right,
            -(10.25 * height),
            "the wheel did not scroll the result"
        );
        assert_eq!(
            left,
            -(7.25 * height),
            "the original is not level with the result"
        );
        assert_eq!(leads(&view, cx), (0, 1));
    }

    /// And the other way: scrolling the original scrolls the result.
    /// Take `Side::Original` out of `scrolled`'s callers and the result
    /// stays at the top: red.
    #[gpui::test]
    fn scrolling_the_original_scrolls_the_result(cx: &mut TestAppContext) {
        let original = numbered(0, 200);
        let result = format!("{}{original}", numbered(1000, 3));
        let (view, cx) = window_over(cx, &original, &result, Comparison::default());
        let height = line_height(&view, cx);

        wheel(&view, Side::Original, -(20.5 * height), cx);
        let (left, right) = offsets(&view, cx);
        assert_eq!(
            left,
            -(20.5 * height),
            "the wheel did not scroll the original"
        );
        assert_eq!(
            right,
            -(23.5 * height),
            "the result is not level with the original"
        );
        assert_eq!(leads(&view, cx), (1, 0));
    }

    /// A scroll the library applies while it lays the text out — the
    /// keyboard's, a caret brought into view — arrives after the frame,
    /// not with the key. Here each side is scrolled the way a key would
    /// scroll it, by an offset handed to the editor, and the other side
    /// follows. Take `Side::Original` or `Side::Result` out of
    /// `scrolled`'s callers and this goes red with the matching test
    /// above.
    #[gpui::test]
    fn a_scroll_the_library_applies_is_followed_too(cx: &mut TestAppContext) {
        let original = numbered(0, 200);
        let result = format!("{}{original}", numbered(1000, 3));
        let (view, cx) = window_over(cx, &original, &result, Comparison::default());
        let height = line_height(&view, cx);

        for (side, rows, other) in [
            (Side::Original, 40.0, 43.0),
            (Side::Result, 80.0, 77.0),
            (Side::Original, 2.0, 5.0),
        ] {
            cx.update(|_, cx| {
                let editor = view.read(cx).editor(side, cx);
                editor.update(cx, |editor, cx| {
                    editor.set_scroll_offset(gpui::point(px(0.0), px(-(rows * height))), cx)
                });
            });
            settle(cx);
            let (left, right) = offsets(&view, cx);
            let (led, followed) = match side {
                Side::Original => (left, right),
                Side::Result => (right, left),
            };
            assert_eq!(led, -(rows * height), "{side:?} was not scrolled");
            assert_eq!(followed, -(other * height), "{side:?} was not followed");
        }
    }

    /// Inside a passage that changed, the result moves in proportion
    /// through its own lines: forty lines of the original became ten,
    /// so twenty lines into the forty is five into the ten (D380). Put
    /// `position_across` back to holding at the passage's first line
    /// and the result stands five lines short: red.
    #[gpui::test]
    fn inside_a_changed_passage_the_other_side_moves_in_proportion(cx: &mut TestAppContext) {
        let head = numbered(0, 10);
        let tail = numbered(500, 200);
        let original = format!("{head}{}{tail}", numbered(100, 40));
        let result = format!("{head}{}{tail}", numbered(900, 10));
        let (view, cx) = window_over(cx, &original, &result, Comparison::default());
        let height = line_height(&view, cx);

        wheel(&view, Side::Original, -(30.0 * height), cx);
        let (left, right) = offsets(&view, cx);
        assert_eq!(left, -(30.0 * height));
        assert_eq!(
            right,
            -(15.0 * height),
            "the result did not move in proportion"
        );

        // Past the passage the two move line for line again, thirty
        // lines apart.
        wheel(&view, Side::Original, -(30.0 * height), cx);
        let (left, right) = offsets(&view, cx);
        assert_eq!(left, -(60.0 * height));
        assert_eq!(right, -(30.0 * height));
    }

    /// One scroll, one lead. The result has lost every line of the
    /// original past the hundredth, so a scroll of the original to its
    /// hundred-and-fiftieth line asks the result for more than it has:
    /// the result stops at its own end, short of what was asked. That
    /// is still the ask arriving, not a person scrolling the result —
    /// take the landing check (`Asked::lands`) out and the result's
    /// stop is taken for a lead, which drags the original back up to
    /// where the result's end stands: red.
    #[gpui::test]
    fn a_follower_that_stops_short_does_not_lead_back(cx: &mut TestAppContext) {
        let original = numbered(0, 300);
        let result = numbered(0, 100);
        let (view, cx) = window_over(cx, &original, &result, Comparison::default());
        let height = line_height(&view, cx);

        wheel(&view, Side::Original, -(150.0 * height), cx);
        let (left, right) = offsets(&view, cx);
        assert_eq!(left, -(150.0 * height), "the original was dragged back");
        assert!(
            right > -(100.0 * height),
            "the result went past its end: {right}"
        );
        assert!(right < 0.0, "the result did not move");
        assert_eq!(leads(&view, cx), (1, 0), "a follower led back");

        // And a person scrolling the result afterwards still leads it:
        // the ask that stopped short is not held against the next move.
        wheel(&view, Side::Result, 10.0 * height, cx);
        let (left, right) = offsets(&view, cx);
        assert_eq!(leads(&view, cx), (1, 1));
        let top = -right / height;
        assert!(
            (-left / height - top).abs() < 0.01,
            "the original is not level with the result: {left} against {right}"
        );
    }

    /// The Compare page's row off: each pane scrolls on its own. Take
    /// the check out of `scrolled` and the original follows: red.
    #[gpui::test]
    fn with_the_row_off_each_side_scrolls_alone(cx: &mut TestAppContext) {
        let original = numbered(0, 200);
        let comparison = Comparison {
            sync_scroll: false,
            ..Comparison::default()
        };
        let (view, cx) = window_over(cx, &original, &original, comparison);
        let height = line_height(&view, cx);

        wheel(&view, Side::Result, -(12.0 * height), cx);
        assert_eq!(offsets(&view, cx), (0.0, -(12.0 * height)));
        wheel(&view, Side::Original, -(5.0 * height), cx);
        assert_eq!(offsets(&view, cx), (-(5.0 * height), -(12.0 * height)));
        assert_eq!(leads(&view, cx), (0, 0));
    }

    /// The cursor and the scrolling together (D383). The result's caret
    /// goes to a line inside thirty lines the result put in; the result
    /// scrolls to show it, the original's caret goes to the line the
    /// insertion stands in front of — and the original's scroll is the
    /// result's, through the map, not the caret's own. Without that,
    /// the original scrolls only as far as its caret, forty-odd lines
    /// short of where the result is, and then leads the result back up
    /// there — the result's caret out of sight: red.
    #[gpui::test]
    fn the_cursor_follow_does_not_drag_the_result(cx: &mut TestAppContext) {
        let original = numbered(0, 300);
        let result = format!(
            "{}{}{}",
            numbered(0, 100),
            numbered(1000, 30),
            numbered(100, 200)
        );
        let (view, cx) = window_over(cx, &original, &result, Comparison::default());
        let height = line_height(&view, cx);

        cx.update(|window, cx| {
            let editor = view.read(cx).editor(Side::Result, cx);
            editor.update(cx, |editor, cx| {
                editor.set_cursor_position(Position::new(120, 0), window, cx)
            });
        });
        settle(cx);

        let (shown, caret) = cx.update(|_, cx| {
            let view = view.read(cx);
            let original = view.original.read(cx);
            (
                view.editor(Side::Result, cx).read(cx).visible_row_range(),
                original.cursor_position().line,
            )
        });
        let shown = shown.expect("laid out");
        assert!(
            shown.contains(&120),
            "the result's caret is out of sight: {shown:?}"
        );
        assert_eq!(caret, 100, "the original's caret did not follow");
        let (left, right) = offsets(&view, cx);
        let top = f64::from(-right / height);
        let expected = cx.update(|_, cx| view.read(cx).diff.position_across(Side::Result, top));
        assert!(
            (f64::from(-left / height) - expected).abs() < 0.01,
            "the original is not where the result's top puts it: {left} for {right}"
        );
        assert_eq!(leads(&view, cx).0, 0, "the original led the result");
    }

    /// With the result's lines wrapped, the two sides line up by the
    /// rows the last layout showed: approximate, and never a loop. A
    /// short text that wraps nothing is laid out exactly as an unwrapped
    /// one, so here the alignment is still exact.
    #[gpui::test]
    fn a_wrapped_result_still_leads_and_follows(cx: &mut TestAppContext) {
        let original = numbered(0, 200);
        let result = format!("{}{original}", numbered(1000, 3));
        let (view, cx) = window_over(cx, &original, &result, Comparison::default());
        cx.update(|window, cx| {
            let pane = view.read(cx).result.clone();
            pane.update(cx, |pane, cx| {
                pane.toggle(result::View::SoftWrap, window, cx)
            });
        });
        settle(cx);
        let height = line_height(&view, cx);

        wheel(&view, Side::Original, -(20.0 * height), cx);
        let (left, right) = offsets(&view, cx);
        assert_eq!(left, -(20.0 * height));
        assert_eq!(right, -(23.0 * height), "the wrapped result did not follow");

        wheel(&view, Side::Result, -(10.0 * height), cx);
        let (left, right) = offsets(&view, cx);
        assert_eq!(right, -(33.0 * height));
        assert_eq!(
            left,
            -(30.0 * height),
            "the original did not follow the wrapped result"
        );
        assert_eq!(leads(&view, cx), (1, 1));
    }

    /// A recompute moves neither pane (D390). A selection in the
    /// original near the top, the result scrolled far below it, then an
    /// edit on the line the result's caret already stood on — no follow
    /// runs — and the recompute's empty edit collapses that selection
    /// to a caret the editor would bring into view. Take out the offset
    /// `repaint_original` puts back and the original jumps to the
    /// caret and pulls the result after it: red.
    #[gpui::test]
    fn a_recompute_moves_neither_pane(cx: &mut TestAppContext) {
        let original = numbered(0, 400);
        let result = original.replacen("line 5\n", "line five\n", 1);
        let (view, cx) = window_over(cx, &original, &result, Comparison::default());
        let height = line_height(&view, cx);

        // The result's caret on row 300, the result scrolled so that
        // row is in view; the original follows.
        cx.update(|window, cx| {
            let editor = view.read(cx).editor(Side::Result, cx);
            editor.update(cx, |editor, cx| {
                editor.set_cursor_position(Position::new(300, 0), window, cx)
            });
        });
        settle(cx);
        let before = offsets(&view, cx);
        assert!(
            before.1 < -(200.0 * height),
            "the result did not scroll: {before:?}"
        );
        // Then a selection in the original near row 10, far above what
        // either pane shows; neither pane moves for it.
        cx.update(|_, cx| {
            let original = view.read(cx).original.clone();
            original.update(cx, |original, cx| {
                let text = original.value();
                let start = text.match_indices('\n').nth(9).expect("ten lines").0 + 1;
                original.set_selected_range(start..start + 4, cx);
            });
        });
        settle(cx);
        // Selecting brought the selection into view; the wheel takes the
        // result back down to its caret, and the original follows,
        // keeping its selection off screen.
        let selected = offsets(&view, cx).1;
        wheel(&view, Side::Result, before.1 - selected, cx);
        assert_eq!(
            offsets(&view, cx),
            before,
            "the panes are not back at the caret"
        );

        // The keystroke's own frame first — the editor keeps its caret
        // in view as it types, and the original follows that — then
        // the settle and the recompute, which must move nothing.
        cx.update(|window, cx| {
            let editor = view.read(cx).editor(Side::Result, cx);
            editor.update(cx, |editor, cx| editor.insert("x", window, cx));
        });
        cx.run_until_parked();
        let typed = offsets(&view, cx);
        assert!(typed.1 < -(200.0 * height), "{typed:?}");
        settle(cx);
        assert_eq!(offsets(&view, cx), typed, "a recompute moved a pane");
    }

    /// An ask that had its frame and moved nothing is dropped (D391).
    /// The result, shorter than the original, stands at its end; the
    /// original scrolls on, and the ask it makes of the result moves
    /// nothing. Lines typed at the result's bottom then move its end and
    /// its caret down: that move is the result's own and leads the
    /// original. Kept, the old ask would take it for its landing and
    /// the original would stay where it was: red.
    #[gpui::test]
    fn an_ask_that_moved_nothing_does_not_hide_the_next_move(cx: &mut TestAppContext) {
        let original = numbered(0, 300);
        let result = numbered(0, 100);
        let (view, cx) = window_over(cx, &original, &result, Comparison::default());
        let height = line_height(&view, cx);

        wheel(&view, Side::Original, -(150.0 * height), cx);
        let at_end = offsets(&view, cx).1;
        wheel(&view, Side::Original, -(10.0 * height), cx);
        assert_eq!(
            offsets(&view, cx).1,
            at_end,
            "the result was not at its end"
        );

        // Sixty lines put in at the result's bottom, the way a paste
        // lands but without moving the view: the result's end moves
        // down and the result stays where it stood.
        cx.update(|window, cx| {
            let editor = view.read(cx).editor(Side::Result, cx);
            editor.update(cx, |editor, cx| {
                let end = editor.value().len();
                editor.set_selected_range(end..end, cx);
                editor.insert("more\n".repeat(60), window, cx);
            });
        });
        settle(cx);
        let (left_before, right_before) = offsets(&view, cx);
        assert_eq!(right_before, at_end, "putting lines in moved the result");
        let leads_before = leads(&view, cx);

        // One real scroll of the result, down, the old ask's way.
        wheel(&view, Side::Result, -(5.0 * height), cx);
        let (left, right) = offsets(&view, cx);
        assert_eq!(
            right,
            right_before - 5.0 * height,
            "the wheel did not scroll the result"
        );
        assert_eq!(
            leads(&view, cx),
            (leads_before.0, leads_before.1 + 1),
            "the result's own move did not lead"
        );
        assert!(left != left_before, "the original did not follow: {left}");
    }

    /// Both panes moved in one frame: the result wins, at once (D383,
    /// D392). Each is handed an offset in the same update; the original
    /// paints first and notifies first. Look at the original first, as
    /// its own notification did before, and it leads the result away
    /// from where it was put: red.
    #[gpui::test]
    fn two_panes_moved_in_one_frame_end_where_the_result_put_them(cx: &mut TestAppContext) {
        let original = numbered(0, 300);
        let result = format!("{}{original}", numbered(1000, 3));
        let (view, cx) = window_over(cx, &original, &result, Comparison::default());
        let height = line_height(&view, cx);

        cx.update(|_, cx| {
            let view = view.read(cx);
            let (left, right) = (
                view.editor(Side::Original, cx),
                view.editor(Side::Result, cx),
            );
            left.update(cx, |editor, cx| {
                editor.set_scroll_offset(gpui::point(px(0.0), px(-(90.0 * height))), cx)
            });
            right.update(cx, |editor, cx| {
                editor.set_scroll_offset(gpui::point(px(0.0), px(-(40.0 * height))), cx)
            });
        });
        settle(cx);
        assert_eq!(
            offsets(&view, cx),
            (-(37.0 * height), -(40.0 * height)),
            "the panes did not end where the result put them"
        );
        assert_eq!(leads(&view, cx), (0, 1), "the original led too");
    }

    /// The far side of a wrapped result: rows not laid out are placed
    /// by the average height of the rows that are, and the next step
    /// lands on the row. A result whose every line wraps once is two
    /// lines a row; scrolled from the original far past what the
    /// result has laid out, the result lands near its row, and level
    /// after the next step.
    #[gpui::test]
    fn a_wrapped_result_far_away_is_placed_by_estimate(cx: &mut TestAppContext) {
        let long = "word ".repeat(250);
        let original: String = (0..300).map(|i| format!("{i} {long}\n")).collect();
        let (view, cx) = window_over(cx, &original, &original, Comparison::default());
        cx.update(|window, cx| {
            let pane = view.read(cx).result.clone();
            pane.update(cx, |pane, cx| {
                pane.toggle(result::View::SoftWrap, window, cx)
            });
        });
        settle(cx);
        let height = line_height(&view, cx);

        wheel(&view, Side::Original, -(150.0 * height), cx);
        wheel(&view, Side::Original, -(1.0 * height), cx);
        let shown = cx.update(|_, cx| {
            view.read(cx)
                .editor(Side::Result, cx)
                .read(cx)
                .visible_row_range()
        });
        let shown = shown.expect("laid out");
        assert_eq!(
            shown.start, 151,
            "the wrapped result is not on the original's row"
        );
    }

    /// The marks answer for the visible rows only, and the rows keep
    /// their side: what the original paints is "removed", what the
    /// result paints is "added".
    #[test]
    fn marks_are_cut_to_the_visible_rows() {
        let marks = Marks::new(vec![2, 5, 9, 40], Side::Original);
        assert_eq!(marks.within(0..3), &[2]);
        assert_eq!(marks.within(3..10), &[5, 9]);
        assert_eq!(marks.within(10..40), &[] as &[u32]);
        assert_eq!(marks.within(0..100), &[2, 5, 9, 40]);
        assert_eq!(
            Marks::new(Vec::new(), Side::Result).within(0..10),
            &[] as &[u32]
        );
    }
}
