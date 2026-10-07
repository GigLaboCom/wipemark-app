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
//! # The two sides stay in step, by the cursor
//!
//! The editor has no public way to be scrolled from outside, and this
//! repository does not patch the library for a convenience. What it
//! does have is a cursor that can be placed, and an editor that keeps
//! its cursor in view. So the original *follows the result's cursor*:
//! whenever the result's caret changes line, the original's is put on
//! the line that stands where that one does — [`Diff::original_row_of`]
//! is the map — and the original scrolls to show it. Placing a cursor
//! focuses the editor it is placed in, which would take the keyboard
//! out of the result mid-word; the focus is handed straight back in
//! the same update, and GPUI notices a focus change only at the next
//! frame, so nothing blurs. One direction only: the result leads,
//! because it is the side being written in. The Compare page can turn
//! the following off, for a reader who would rather scroll each side
//! by hand.
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
    actions, div, px, size, Anchor, AnyElement, AnyWindowHandle, App, Bounds, ClickEvent, Context,
    Entity, FocusHandle, Focusable, Global, Hsla, KeyBinding, Pixels, Rgba, SharedString, Size,
    Subscription, Task, TitlebarOptions, Window, WindowBounds, WindowOptions,
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

use crate::diff::{Diff, Grain};
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
}

impl Default for Comparison {
    /// Words, and following: the marks a reader of a rewrite wants,
    /// and the two sides kept in step.
    fn default() -> Self {
        Self {
            grain: Grain::Words,
            follow: true,
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
#[derive(Debug, Clone)]
pub struct Subject {
    pub handed: Handed,
    pub intake: Option<Intake>,
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
    /// the queue's Clean writes for the same document.
    pub cleaned: String,
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
        // Layer A at its defaults, as the queue's Clean and the CLI run
        // it: deterministic, so this is the text they write.
        let cleaned = wipemark_core::clean(&text, &Options::default()).text;
        Ok(Loaded {
            name,
            text,
            cleaned,
        })
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

/// Which side a mark is painted on, and therefore what it means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// A line the result no longer has.
    Original,
    /// A line the original never had.
    Result,
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
    /// The left pane: the same editor as the right, disabled — which
    /// still selects, copies and searches, and no longer edits.
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
        let cursor = cx.observe_in(&result_state, window, |view, state, window, cx| {
            let row = state.read(cx).cursor_position().line;
            if row != view.followed {
                view.followed = row;
                view.follow(row, window, cx);
            }
        });

        let mut view = Self {
            focus,
            state: State::Reading,
            subject: Some(subject),
            name: String::new(),
            original_text: Arc::from(""),
            cleaned_text: Arc::from(""),
            edited: false,
            original,
            original_marks: None,
            result,
            comparison,
            diff: Diff::of("", ""),
            generation: 0,
            followed: 0,
            _subscriptions: vec![changed, cursor],
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
            }) => {
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
    /// is untouched, the editor is disabled so its history is nobody's
    /// and nothing listens to its changes, and what it costs is a
    /// selection in the original, which collapses to its end. See the
    /// module docs, and `the_original_is_asked_again_when_the_marks_move`.
    fn repaint_original(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.original.update(cx, |original, cx| {
            original.insert("", window, cx);
        });
    }

    /// Put the original's cursor on the line that stands where the
    /// result's row does, so the original scrolls to it — and hand the
    /// keyboard straight back to whoever had it. See the module docs.
    fn follow(&self, row: u32, window: &mut Window, cx: &mut Context<Self>) {
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
                    .child(result::pane(&self.original, cx).disabled(true)),
            );
        let right = v_flex()
            .size_full()
            .child(Self::caption(Message::CompareResult, cx))
            .child(div().flex_1().min_h(px(0.0)).child(self.result.clone()));

        h_resizable("compare-split")
            .child(resizable_panel().child(left))
            .child(resizable_panel().child(right))
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
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(muted)
                            .child(SharedString::from(t(Message::ComparePending))),
                    )
                    .child(
                        Button::new("compare-reset")
                            .small()
                            .outline()
                            .icon(IconName::RotateLeft)
                            .label(SharedString::from(t(Message::CompareReset)))
                            .tooltip(SharedString::from(t(Message::CompareResetTooltip)))
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
            follow: true,
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
