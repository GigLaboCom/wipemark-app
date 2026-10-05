//! The main window's table: what has been handed to the product, one
//! row each.
//!
//! Epic **E6**, the first slice of E7's queue (S7.1). heretic-lazy-shot's
//! main window is a table of screenshots — a filter bar over it, a
//! thumbnail down the left, an Actions menu down the right and a
//! paginator underneath — and this is that table for a product whose
//! subject is text as often as pictures: every row is one thing that
//! arrived — dropped on the window, or chosen with the Import button —
//! with a **preview** beside it (the picture, or the first lines), an
//! id and a keyword the way lazy-shot's rows have, what it is, what it
//! was established from, how big, and when. Hovering the preview opens
//! a larger one. The Actions menu cleans the row, opens the file with
//! the system when there is a file, opens the Compare window on the row
//! when it is text, and — once the row is cleaned — opens, shows or
//! copies the result; a double click on the row is the Compare item
//! without the menu.
//!
//! # lazy-shot's table, in its shape
//!
//! The filter bar takes an id and a keyword, both as substrings, the
//! way lazy-shot's `id LIKE` and `key_word LIKE` do; "Reset filters"
//! appears while either is set, and the count beside it is of what the
//! filters let through. The rows are ordered by when they arrived —
//! newest first, lazy-shot's `ORDER BY id DESC` — and the Arrived header
//! flips that. The paginator under the table is lazy-shot's: a page
//! size from the same four choices, Previous and Next, and "Page x of
//! y". lazy-shot's status column is here too, as the row's life —
//! waiting, queued, cleaning, and what the clean came to; its date range
//! is not, because a list that empties when the application quits does
//! not need a calendar.
//!
//! # How things get in
//!
//! Three doors and one road. A drop anywhere on the main window lands
//! in the window's [`Catcher`] — the platform destination is per
//! window, and the table is most of the window, so the table *is* the
//! drop zone and asks for nothing of its own. Import goes through the
//! platform's file picker and hands its answer to the **same** catcher
//! ([`Catcher::land`]), so a file chosen and a file dropped are
//! recognised by one piece of code and arrive as one kind of event;
//! Paste hands over what is on the clipboard (`crate::clipboard`) the
//! same way, and the command line's `--import=<path>` is that road once
//! more. Nothing here recognises anything: `wipemark-intake` does, on
//! the background executor, and the queue is told when it is done.
//!
//! # Cleaning
//!
//! A row is cleaned when somebody asks — Clean in its Actions menu,
//! Clean all on the toolbar, `--clean=<path>` on the command line —
//! never on arrival. One clean runs at a time, first asked first done
//! ([`Line`]), on the background executor through
//! [`clean::clean_one`]: a decoded picture can be hundreds of megabytes,
//! so this is a memory rule as much as an ordering one, and nothing a
//! clean does — a read, a decode, a write — runs on the thread that
//! draws. The Retention page's plan is taken when a row's clean
//! **starts**, so a choice changed while the row waited applies to it,
//! and one changed after it was cleaned does not move its result. The
//! row's badge says what the clean came to and its note where the result
//! went; until then the hover card says what *would* happen, as before.
//! A result already where this one would go is refused and left alone
//! (D261); "Replace the existing result" is the one explicit way over
//! it. Rewriting with a model is not here, and the footer says so.
//!
//! # What it does not do yet
//!
//! Rewrite. Folders and archives are listed
//! as what they are and never expanded — an item that silently became
//! four hundred rows is not what anybody dropped. A keyword is shown and
//! searched but not yet *assigned*: in lazy-shot that is the MCP
//! server's `assign_keyword`, and this server has no such tool yet.
//!
//! # Nothing blocks the window
//!
//! A row is on screen the moment the intake crate has named it; its
//! preview arrives a beat later, from [`Preview::of`] on the background
//! executor, because a text preview reads the front of a file and an
//! image preview is decoded by GPUI on a task of its own. The rows on a
//! page are a `uniform_list`, so a page of a hundred draws the twelve
//! that are on screen.

use std::collections::VecDeque;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Local, Utc};
use gpui::prelude::*;
use gpui::{
    div, img, px, uniform_list, AnyElement, App, ClipboardItem, Context, Corner, Div, Entity,
    ImageSource, ObjectFit, PathPromptOptions, Pixels, ScrollStrategy, SharedString, Stateful,
    Subscription, UniformListScrollHandle, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::clipboard::Clipboard;
use gpui_component::hover_card::HoverCard;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::scroll::Scrollbar;
use gpui_component::tag::Tag;
use gpui_component::tooltip::Tooltip;
use gpui_component::{
    h_flex, v_flex, ActiveTheme, ColorName, Disableable as _, InteractiveElementExt as _,
    Sizable as _, StyledExt as _,
};
use wipemark_i18n::{args, t, t_args, Message};
use wipemark_intake::{Arrived, Handed, Intake, Kind};

use crate::clean::{self, Cleanable, Outcome, Refusal, Verdict};
use crate::compare::{self, Comparison, Subject};
use crate::drop::{self, Arrival, Catcher, Landed};
use crate::icon::{Icon, IconName};
use crate::preview::{Excerpt, Picture, Preview};
use crate::settings::Preferences;
use crate::wording::{self, Badge, Tone};

/// How tall a row is.
///
/// Tall enough for a thumbnail a person can recognise a screenshot by,
/// and three lines of a note; short enough that a laptop shows nine of
/// them. lazy-shot's default thumbnail is 96 points and its rows are
/// mostly thumbnail; a row here also carries a name, a kind and a
/// sentence about the evidence, which is what the width is for.
const ROW: Pixels = px(72.0);

/// The preview's frame inside a row. Wide, because a screenshot is, and
/// because three lines of prose at this size need the width to say
/// anything.
const THUMB: gpui::Size<Pixels> = gpui::Size {
    width: px(128.0),
    height: px(56.0),
};

/// The larger preview, in the hover card. A fixed frame rather than the
/// picture's own size: a hover card that is a different size for every
/// row is a hover card that jumps.
const LARGE: gpui::Size<Pixels> = gpui::Size {
    width: px(560.0),
    height: px(380.0),
};

/// How long the pointer rests on a preview before the larger one opens.
///
/// Shorter than the component's 600 ms default: a preview is what the
/// pointer is *for* in that cell, and the delay only has to be long
/// enough that crossing the column on the way somewhere else opens
/// nothing.
const HOVER_DELAY: Duration = Duration::from_millis(350);

/// How many lines of a text preview the row shows before the ellipsis.
/// Three lines of `text_xs` are 48 points, which is the frame less its
/// padding — a fourth would be cut in half.
const ROW_LINES: usize = 3;

/// The lane the vertical scrollbar is drawn in, down the right of the
/// rows. gpui-component keeps its own width private; this is the
/// number its table uses.
const SCROLLBAR: Pixels = px(12.0);

/// The page sizes on offer — lazy-shot's four — and the one a fresh
/// window opens on.
pub const PAGE_SIZES: [usize; 4] = [10, 20, 50, 100];
pub const DEFAULT_PAGE_SIZE: usize = 20;

/// Which end of the list the newest row is at.
///
/// Newest first is lazy-shot's `ORDER BY id DESC` and the default here
/// for the same reason: the thing just dropped is the thing being
/// looked for. Ids are handed out in arrival order and a drop of three
/// files shares one clock reading, so the id is what the order is
/// really over, and the header that flips it says "Arrived" because the
/// time is what a person reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    NewestFirst,
    OldestFirst,
}

impl Order {
    fn flipped(self) -> Self {
        match self {
            Order::NewestFirst => Order::OldestFirst,
            Order::OldestFirst => Order::NewestFirst,
        }
    }

    /// What clicking the header would do next, as the tooltip says it.
    fn next_label(self) -> Message {
        match self {
            Order::NewestFirst => Message::QueueSortOldest,
            Order::OldestFirst => Message::QueueSortNewest,
        }
    }

    fn glyph(self) -> IconName {
        match self {
            Order::NewestFirst => IconName::ChevronDown,
            Order::OldestFirst => IconName::ChevronUp,
        }
    }
}

/// What the filter bar holds.
///
/// Both are substrings, lazy-shot's `LIKE '%…%'`: an id of `12` finds
/// 12, 112 and 120, which is what somebody who remembers three digits
/// of a number wants. Whitespace around either is ignored.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filter {
    pub id: String,
    pub keyword: String,
}

impl Filter {
    /// Whether either field narrows anything — what shows "Reset
    /// filters".
    pub fn is_active(&self) -> bool {
        !self.id.trim().is_empty() || !self.keyword.trim().is_empty()
    }

    /// Whether a row passes. A keyword filter against a row that has no
    /// keyword does not pass: an empty keyword is not a keyword that
    /// contains nothing, it is no keyword.
    fn admits(&self, id: u64, keyword: Option<&str>) -> bool {
        let wanted_id = self.id.trim();
        if !wanted_id.is_empty() && !id.to_string().contains(wanted_id) {
            return false;
        }
        let wanted_keyword = self.keyword.trim();
        if !wanted_keyword.is_empty() {
            let Some(keyword) = keyword else {
                return false;
            };
            if !keyword
                .to_lowercase()
                .contains(&wanted_keyword.to_lowercase())
            {
                return false;
            }
        }
        true
    }
}

/// The rows a filter lets through, as indices into the list, in the
/// order asked for.
///
/// Over the ids and keywords rather than the rows, so it can be checked
/// without building a row — and because those two fields are the whole
/// of what a filter reads.
pub fn shown(rows: &[(u64, Option<&str>)], filter: &Filter, order: Order) -> Vec<usize> {
    let mut shown: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, (id, keyword))| filter.admits(*id, *keyword))
        .map(|(index, _)| index)
        .collect();
    // The list is kept in arrival order, so the ascending case is the
    // list as it stands and the other is its reverse — no sort, and no
    // question of stability.
    if order == Order::NewestFirst {
        shown.reverse();
    }
    shown
}

/// How many pages `total` rows make at `page_size` a page — never zero,
/// because "Page 1 of 0" is not a sentence.
pub fn pages(total: usize, page_size: usize) -> usize {
    total.div_ceil(page_size.max(1)).max(1)
}

/// The rows on page `page` (zero-based), clamped to the last page there
/// is: a page the list has shrunk out from under is the last one, not
/// an empty one.
pub fn page_range(total: usize, page_size: usize, page: usize) -> Range<usize> {
    let page_size = page_size.max(1);
    let page = page.min(pages(total, page_size) - 1);
    let start = (page * page_size).min(total);
    start..(start + page_size).min(total)
}

/// The columns, in order.
///
/// One list that both the header and every row walk, so a column added
/// here is a column with a title and a width in one place, and
/// `every_column_has_a_title` is what says the title exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Column {
    Preview,
    Id,
    Keyword,
    Name,
    Kind,
    Status,
    Format,
    Size,
    Arrived,
    Actions,
}

impl Column {
    const ALL: [Column; 10] = [
        Column::Preview,
        Column::Id,
        Column::Keyword,
        Column::Name,
        Column::Kind,
        Column::Status,
        Column::Format,
        Column::Size,
        Column::Arrived,
        Column::Actions,
    ];

    fn title(self) -> Message {
        match self {
            Column::Preview => Message::QueueColumnPreview,
            Column::Id => Message::QueueColumnId,
            Column::Keyword => Message::QueueColumnKeyword,
            Column::Name => Message::QueueColumnName,
            Column::Kind => Message::QueueColumnKind,
            Column::Status => Message::QueueColumnStatus,
            Column::Format => Message::QueueColumnFormat,
            Column::Size => Message::QueueColumnSize,
            Column::Arrived => Message::QueueColumnArrived,
            Column::Actions => Message::QueueColumnActions,
        }
    }

    /// A fixed width, or `None` for the one column that takes what is
    /// left — the name, which is the only value here with no natural
    /// length.
    fn width(self) -> Option<Pixels> {
        match self {
            Column::Preview => Some(THUMB.width + px(24.0)),
            Column::Id => Some(px(72.0)),
            Column::Keyword => Some(px(136.0)),
            Column::Name => None,
            Column::Kind => Some(px(124.0)),
            Column::Status => Some(px(136.0)),
            Column::Format => Some(px(132.0)),
            Column::Size => Some(px(88.0)),
            Column::Arrived => Some(px(96.0)),
            Column::Actions => Some(px(64.0)),
        }
    }

    /// Numbers sit on the right; everything else on the left.
    fn right_aligned(self) -> bool {
        matches!(self, Column::Size)
    }

    /// One cell of this column, sized and padded, with nothing in it.
    fn cell(self) -> Div {
        let cell = h_flex()
            .h_full()
            .px_3()
            .min_w(px(0.0))
            .overflow_hidden()
            .flex_shrink_0();
        let cell = match self.width() {
            Some(width) => cell.w(width),
            None => cell.flex_1().flex_shrink(),
        };
        let cell = if self.right_aligned() {
            cell.justify_end()
        } else {
            cell
        };
        match self {
            Column::Actions => cell.justify_center(),
            _ => cell,
        }
    }
}

/// Where a row is in its life.
#[derive(Debug, Clone)]
pub enum Status {
    /// Arrived, and nobody has asked for it to be cleaned.
    Waiting,
    /// Asked for, behind the clean that is running.
    Queued,
    /// Being cleaned now, on the background executor.
    Cleaning,
    /// Cleaned, refused or failed — what happened, as one value.
    Done(Arc<Outcome>),
}

/// One clean asked for: the row, and the one existing result it may
/// write over when it was asked for by "Replace the existing result".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub id: u64,
    pub replacing: Option<PathBuf>,
}

/// The cleans asked for and not yet finished: **one runs at a time**, and
/// the rest wait their turn, first asked first done.
///
/// One at a time because a picture is decoded whole and its raster copied
/// to be restored — two at once is two of those in memory — and because a
/// row's plan is taken when its clean starts, which only means something
/// if the cleans start in the order they were asked for. Pure, so the
/// rule is checked without a window.
#[derive(Debug, Default)]
pub struct Line {
    waiting: VecDeque<Job>,
    running: Option<u64>,
    /// Finished, and asked for, since the line was last empty — what
    /// "Cleaning 2 of 5" counts.
    finished: usize,
    asked: usize,
}

impl Line {
    /// Put a clean at the back of the line.
    pub fn push(&mut self, job: Job) {
        self.asked += 1;
        self.waiting.push_back(job);
    }

    /// The clean to start now: the front of the line, if nothing is
    /// running.
    pub fn start(&mut self) -> Option<Job> {
        if self.running.is_some() {
            return None;
        }
        let job = self.waiting.pop_front()?;
        self.running = Some(job.id);
        Some(job)
    }

    /// The clean of row `id` is over. The count starts again once the
    /// line is empty.
    pub fn finish(&mut self, id: u64) {
        if self.running == Some(id) {
            self.running = None;
            self.finished += 1;
        }
        if self.running.is_none() && self.waiting.is_empty() {
            self.finished = 0;
            self.asked = 0;
        }
    }

    /// Which clean of how many is running, counting from one — `None`
    /// while nothing is.
    pub fn progress(&self) -> Option<(usize, usize)> {
        self.running.map(|_| (self.finished + 1, self.asked))
    }
}

/// Why row's Clean item is greyed, or `None` when it is not: a thing
/// that cannot be cleaned says why, and a row already in line or done
/// says that. Pure, so the menu's rule is checked without one.
pub fn why_not_clean(status: &Status, cleanable: Cleanable) -> Option<String> {
    match status {
        Status::Queued | Status::Cleaning => Some(t(Message::QueueActionCleanBusy)),
        Status::Done(_) => Some(t(Message::QueueActionCleanDone)),
        Status::Waiting => match cleanable {
            Cleanable::No(unable) => Some(wording::unable(unable)),
            Cleanable::Text(_) | Cleanable::Picture(_) => None,
        },
    }
}

/// One thing that arrived.
struct Row {
    /// Stable for the life of the row, and what every element id in it
    /// is built from — a `uniform_list` index moves the moment a row
    /// above is removed, and a hover card keyed by one would follow the
    /// index rather than the thing. Shown in the ID column, because it
    /// is what an agent will name a document by.
    id: u64,
    /// lazy-shot's `key_word`: a handle somebody assigned so the thing
    /// can be found again by name. Nothing assigns one yet — see the
    /// module docs — so today every row's is `None`.
    keyword: Option<String>,
    arrival: Arrival,
    preview: Preview,
    arrived_at: DateTime<Local>,
    status: Status,
}

impl Row {
    /// Whether it can be cleaned — decided from what intake said, read
    /// on every frame, so nothing here reads a file.
    fn cleanable(&self) -> Cleanable {
        clean::cleanable(&self.arrival.intake)
    }

    fn outcome(&self) -> Option<&Outcome> {
        match &self.status {
            Status::Done(outcome) => Some(outcome),
            _ => None,
        }
    }
}

/// What the queue asks of the window it is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueEvent {
    /// Show the Report dialog for row `id` — painted by the shell, over
    /// the whole window, not inside the table.
    Report(u64),
}

/// The rows, and the drop target that fills them.
pub struct Queue {
    /// In arrival order, always; the order on screen is [`Order`]'s.
    rows: Vec<Row>,
    /// The window's drop target. Owned here rather than by the shell
    /// because the table is what a drop is *for*; the toolbar's Import
    /// reaches it through [`Queue::import`].
    catcher: Entity<Catcher>,
    /// Fills a row's hover card with what the Retention page would do to
    /// it.
    preferences: Entity<Preferences>,
    scroll: UniformListScrollHandle,
    /// The two fields of the filter bar, and what they hold — mirrored
    /// on every change so the rows can be chosen without reading the
    /// fields back.
    by_id: Entity<InputState>,
    by_keyword: Entity<InputState>,
    filter: Filter,
    order: Order,
    /// Zero-based, and clamped on the way to the screen rather than on
    /// the way in — see [`page_range`].
    page: usize,
    page_size: usize,
    /// The cleans asked for and not yet over.
    line: Line,
    /// Paths `--clean=` handed in, cleaned as their rows land — each
    /// once.
    to_clean: Vec<PathBuf>,
    /// Dropped with the view: every drop, and every import, lands here.
    _landed: Subscription,
    /// Dropped with the view: what is typed into the filter bar.
    _typed: [Subscription; 2],
}

impl Queue {
    /// Build the queue, and make the window it is in accept drops.
    ///
    /// Called while the window is being built, for the reason the panel
    /// gives: the platform destination has to exist before the first
    /// drag, and a window is not dragged onto in the frame it opens in.
    pub fn new(
        preferences: Entity<Preferences>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let catcher = cx.new(|cx| Catcher::new(window, cx));
        drop::accept(window);
        Self::with_catcher(preferences, catcher, window, cx)
    }

    /// The queue over a catcher already made — [`Queue::new`]'s, which
    /// the platform delivers to, or a test's, which it does not.
    fn with_catcher(
        preferences: Entity<Preferences>,
        catcher: Entity<Catcher>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let landed = cx.subscribe(&catcher, |queue, _, Landed(arrivals): &Landed, cx| {
            queue.take(arrivals.clone(), cx);
        });

        let by_id = cx.new(|cx| {
            InputState::new(window, cx).placeholder(SharedString::from(t(Message::QueueFilterId)))
        });
        let by_keyword = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(SharedString::from(t(Message::QueueFilterKeyword)))
        });
        // Every keystroke narrows the list, the way lazy-shot's fields
        // do, and puts the reader back on the first page: page three of
        // the old list is nowhere in the new one.
        let typed_id =
            cx.subscribe_in(&by_id, window, |queue, field, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    queue.filter.id = field.read(cx).value().to_string();
                    queue.page = 0;
                    cx.notify();
                }
            });
        let typed_keyword = cx.subscribe_in(
            &by_keyword,
            window,
            |queue, field, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    queue.filter.keyword = field.read(cx).value().to_string();
                    queue.page = 0;
                    cx.notify();
                }
            },
        );

        Self {
            rows: Vec::new(),
            catcher,
            preferences,
            scroll: UniformListScrollHandle::new(),
            by_id,
            by_keyword,
            filter: Filter::default(),
            order: Order::NewestFirst,
            page: 0,
            page_size: DEFAULT_PAGE_SIZE,
            line: Line::default(),
            to_clean: Vec::new(),
            _landed: landed,
            _typed: [typed_id, typed_keyword],
        }
    }

    /// Hand paths down the road a drop takes — the command line's way
    /// in, and the one the Import button uses once the picker answers.
    pub fn hand(&self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        self.land(paths.into_iter().map(Handed::Path).collect(), cx);
    }

    /// Hand paths down the road a drop takes and clean each as it lands —
    /// `--clean=<path>`, which is Import followed by Clean. A row that
    /// cannot be cleaned is still asked, and its badge says why not.
    pub fn hand_to_clean(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        self.to_clean.extend(paths.iter().cloned());
        self.hand(paths, cx);
    }

    /// Hand anything down the road a drop takes — the clipboard's way
    /// in. Recognised on the background executor and listed when that
    /// is done, exactly as if it had been dropped.
    pub fn land(&self, handed: Vec<Handed>, cx: &mut Context<Self>) {
        self.catcher
            .update(cx, |catcher, cx| catcher.land(handed, cx));
    }

    /// Ask for files or folders with the platform's picker, and hand
    /// whatever is chosen down the road a drop takes.
    ///
    /// The picker comes back over a channel; nothing here waits on it,
    /// and cancelling it changes nothing. Folders are allowed because a
    /// dropped folder is — it lands as one row that says it is a folder,
    /// and is expanded by nothing.
    pub fn import(&self, window: &Window, cx: &Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: true,
            multiple: true,
            prompt: Some(SharedString::from(t(Message::ToolbarImportChoose))),
        });
        let catcher = self.catcher.clone();
        cx.spawn_in(window, async move |_, cx| {
            let paths = match picked.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    tracing::warn!(%error, "the file picker could not open");
                    return;
                }
                Err(_) => return,
            };
            if paths.is_empty() {
                return;
            }
            // Straight to the catcher rather than through this view:
            // the same road a drop takes, one step later. If the window
            // closed while the picker was up, the catcher is held only
            // by this task and what lands in it is seen by nobody,
            // which is the right answer for a choice with nowhere to
            // go.
            let handed = paths.into_iter().map(Handed::Path).collect();
            catcher.update(cx, |catcher, cx| catcher.land(handed, cx));
        })
        .detach();
    }

    /// The ids and keywords, which is all a filter reads.
    fn keys(&self) -> Vec<(u64, Option<&str>)> {
        self.rows
            .iter()
            .map(|row| (row.id, row.keyword.as_deref()))
            .collect()
    }

    /// The rows the filter lets through, in the order on screen.
    fn visible(&self) -> Vec<usize> {
        shown(&self.keys(), &self.filter, self.order)
    }

    /// Something landed: list it now, and find out what it looks like
    /// afterwards.
    fn take(&mut self, arrivals: Arc<[Arrival]>, cx: &mut Context<Self>) {
        if arrivals.is_empty() {
            // A drop that carried nothing this machine could read. The
            // panel has a sentence for it; a table has nothing to add a
            // row for, and the log line is the record.
            tracing::info!("a drop carried nothing that could be listed");
            return;
        }

        // One number per thing across the windows (D280): the panel's
        // cleans draw from the same counter, so a gap is a clean there.
        let ids: Vec<u64> = arrivals.iter().map(|_| clean::number()).collect();
        let now = Local::now();
        for (arrival, &id) in arrivals.iter().zip(&ids) {
            self.rows.push(Row {
                id,
                keyword: None,
                arrival: arrival.clone(),
                preview: Preview::Pending,
                arrived_at: now,
                status: Status::Waiting,
            });
        }
        // The rows `--clean=` asked for, each path once.
        let mut asked = Vec::new();
        for row in &self.rows[self.rows.len() - arrivals.len()..] {
            if let Handed::Path(path) = &row.arrival.handed {
                if let Some(at) = self.to_clean.iter().position(|wanted| wanted == path) {
                    self.to_clean.remove(at);
                    asked.push(row.id);
                }
            }
        }
        if !asked.is_empty() {
            self.clean(&asked, cx);
        }
        // The count and the kinds, and nothing else — a log line that
        // carried what arrived would be the document in a file the
        // product wrote. See `wipemark_log::Elided`.
        let kinds: Vec<Kind> = arrivals.iter().map(|arrival| arrival.intake.kind).collect();
        tracing::info!(count = arrivals.len(), ?kinds, "queued");

        // Go to where the newest row is: the first page with the newest
        // on top, the last with the oldest — unless a filter hides it,
        // in which case the page stays where the reader left it.
        let newest = ids.last().copied().unwrap_or_default();
        let visible = self.visible();
        if let Some(position) = visible
            .iter()
            .position(|&index| self.rows[index].id == newest)
        {
            self.page = position / self.page_size.max(1);
            self.scroll
                .scroll_to_item(position % self.page_size.max(1), ScrollStrategy::Top);
        }
        cx.notify();

        // The previews, off the thread that draws — a text preview reads
        // a file, and the file may be on a network volume. They land by
        // id, so a row removed in the meantime is simply not found.
        cx.spawn(async move |queue, cx| {
            let previews: Vec<Preview> = cx
                .background_executor()
                .spawn(async move {
                    arrivals
                        .iter()
                        .map(|arrival| Preview::of(&arrival.handed, &arrival.intake))
                        .collect()
                })
                .await;
            queue
                .update(cx, |queue, cx| {
                    for (id, preview) in ids.into_iter().zip(previews) {
                        if let Some(row) = queue.rows.iter_mut().find(|row| row.id == id) {
                            row.preview = preview;
                        }
                    }
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    /// Empty both fields and show everything again, from the first
    /// page.
    fn reset_filters(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for field in [&self.by_id, &self.by_keyword] {
            field.update(cx, |field, cx| field.set_value("", window, cx));
        }
        self.filter = Filter::default();
        self.page = 0;
        cx.notify();
    }

    fn flip_order(&mut self, cx: &mut Context<Self>) {
        self.order = self.order.flipped();
        self.page = 0;
        cx.notify();
    }

    /// Change how many rows a page holds, keeping the row at the top of
    /// the current page on screen: page 3 of 20 and page 2 of 50 both
    /// start at row 40, and a reader who changed the size did not ask
    /// to be moved.
    fn set_page_size(&mut self, page_size: usize, cx: &mut Context<Self>) {
        if page_size == self.page_size || page_size == 0 {
            return;
        }
        let first_shown = self.page * self.page_size;
        self.page_size = page_size;
        self.page = first_shown / page_size;
        cx.notify();
    }

    fn turn_page(&mut self, forward: bool, cx: &mut Context<Self>) {
        let pages = pages(self.visible().len(), self.page_size);
        let page = self.page.min(pages - 1);
        self.page = if forward {
            (page + 1).min(pages - 1)
        } else {
            page.saturating_sub(1)
        };
        cx.notify();
    }
}

impl gpui::EventEmitter<QueueEvent> for Queue {}

impl Queue {
    /// What row `id` arrived as and what its clean came to, for the
    /// Report dialog — `None` until it is done.
    pub fn report_of(&self, id: u64) -> Option<(Intake, Arc<Outcome>)> {
        let row = self.rows.iter().find(|row| row.id == id)?;
        match &row.status {
            Status::Done(outcome) => Some((row.arrival.intake.clone(), outcome.clone())),
            _ => None,
        }
    }

    /// The rows Clean all would clean: waiting, and cleanable, in the
    /// order they arrived.
    pub fn cleanable_waiting(&self) -> Vec<u64> {
        self.rows
            .iter()
            .filter(|row| matches!(row.status, Status::Waiting))
            .filter(|row| !matches!(row.cleanable(), Cleanable::No(_)))
            .map(|row| row.id)
            .collect()
    }

    /// Clean every waiting row that can be cleaned — the toolbar's Clean
    /// all. Rows queued, cleaning or done are left where they are.
    pub fn clean_all(&mut self, cx: &mut Context<Self>) {
        let ids = self.cleanable_waiting();
        self.clean(&ids, cx);
    }

    /// Put the waiting rows among `ids` in line, in that order, and start
    /// the first if nothing is running. A row that is not waiting is not
    /// asked twice.
    pub fn clean(&mut self, ids: &[u64], cx: &mut Context<Self>) {
        for &id in ids {
            let Some(row) = self.rows.iter_mut().find(|row| row.id == id) else {
                continue;
            };
            if !matches!(row.status, Status::Waiting) {
                continue;
            }
            row.status = Status::Queued;
            self.line.push(Job {
                id,
                replacing: None,
            });
        }
        self.next(cx);
        cx.notify();
    }

    /// "Replace the existing result": clean row `id` again and write over
    /// the one file its clean refused to (D261). Only for a row whose clean
    /// was refused for exactly that.
    pub fn replace(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(row) = self.rows.iter_mut().find(|row| row.id == id) else {
            return;
        };
        let Some(existing) = row.outcome().and_then(existing_result) else {
            return;
        };
        row.status = Status::Queued;
        self.line.push(Job {
            id,
            replacing: Some(existing),
        });
        self.next(cx);
        cx.notify();
    }

    /// Which clean of how many is running — the status bar's "Cleaning 2
    /// of 5".
    pub fn progress(&self) -> Option<(usize, usize)> {
        self.line.progress()
    }

    /// The status of row `id`, if it is here.
    #[cfg(test)]
    pub fn status_of(&self, id: u64) -> Option<&Status> {
        self.rows
            .iter()
            .find(|row| row.id == id)
            .map(|row| &row.status)
    }

    /// The ids, in arrival order.
    #[cfg(test)]
    pub fn ids(&self) -> Vec<u64> {
        self.rows.iter().map(|row| row.id).collect()
    }

    /// Start the clean at the front of the line, if none is running.
    ///
    /// The plan is taken **now**, from the Retention page's rows as they
    /// stand: the row's clean starts with it, and a later change does not
    /// reach a row already cleaned. Everything the clean does happens on
    /// the background executor.
    fn next(&mut self, cx: &Context<Self>) {
        while let Some(job) = self.line.start() {
            let Some(row) = self.rows.iter_mut().find(|row| row.id == job.id) else {
                self.line.finish(job.id);
                continue;
            };
            row.status = Status::Cleaning;
            let plan = self.preferences.read(cx).plan_for(&row.arrival.intake);
            let arrival = row.arrival.clone();
            let id = job.id;
            cx.spawn(async move |queue, cx| {
                let outcome = cx
                    .background_executor()
                    .spawn(async move {
                        let now = Utc::now();
                        match &job.replacing {
                            Some(existing) => {
                                clean::replace_one(&arrival, &plan, id, now, existing)
                            }
                            None => clean::clean_one(&arrival, &plan, id, now),
                        }
                    })
                    .await;
                queue
                    .update(cx, |queue, cx| queue.finished(id, outcome, cx))
                    .ok();
            })
            .detach();
            return;
        }
    }

    /// Row `id`'s clean is over: say so, and start the next.
    fn finished(&mut self, id: u64, outcome: Outcome, cx: &mut Context<Self>) {
        if let Some(row) = self.rows.iter_mut().find(|row| row.id == id) {
            row.status = Status::Done(Arc::new(outcome));
        }
        self.line.finish(id);
        self.next(cx);
        cx.notify();
    }

    /// The cleaned text of row `id`, for Copy the result — looked up at
    /// click time rather than cloned into every menu, for the reason
    /// [`compare_row`] gives.
    fn result_text(&self, id: u64) -> Option<String> {
        self.rows
            .iter()
            .find(|row| row.id == id)
            .and_then(Row::outcome)
            .and_then(|outcome| outcome.text.clone())
    }
}

/// The file a clean refused to write over, when that is why it was not
/// cleaned — what "Replace the existing result" offers to replace.
fn existing_result(outcome: &Outcome) -> Option<PathBuf> {
    match &outcome.verdict {
        Verdict::NotCleaned(Refusal::Exists(path)) => Some(path.clone()),
        _ => None,
    }
}

/// The thumbnail, the excerpt, or the glyph for a thing with neither.
///
/// One frame whatever is in it, so a column of previews reads as a
/// column. The picture and the excerpt are also the trigger for the
/// larger preview; the glyph is not, because there is nothing larger to
/// show.
fn preview_cell(row: &Row, plan_lines: Vec<String>, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let frame = div()
        .w(THUMB.width)
        .h(THUMB.height)
        .flex_shrink_0()
        .rounded(theme.radius)
        .border_1()
        .border_color(theme.border)
        .bg(theme.muted)
        .overflow_hidden()
        .flex()
        .items_center()
        .justify_center();
    let intake = &row.arrival.intake;

    match &row.preview {
        Preview::Pending => frame
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(SharedString::from(t(Message::QueuePreviewPending))),
            )
            .into_any_element(),
        Preview::None => frame
            .child(
                Icon::new(kind_glyph(intake.kind))
                    .large()
                    .color(theme.muted_foreground),
            )
            .into_any_element(),
        Preview::Image(picture) => {
            let source = image_source(picture);
            let trigger = frame.child(
                img(source.clone())
                    .size_full()
                    .object_fit(ObjectFit::Contain),
            );
            let caption = caption_of(intake);
            HoverCard::new(("preview", row.id))
                .anchor(Corner::TopLeft)
                .open_delay(HOVER_DELAY)
                .trigger(trigger)
                .content(move |_, _, cx| {
                    large_card(
                        div()
                            .w(LARGE.width)
                            .h(LARGE.height)
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(cx.theme().muted)
                            .rounded(cx.theme().radius)
                            .overflow_hidden()
                            .child(
                                img(source.clone())
                                    .size_full()
                                    .object_fit(ObjectFit::Contain),
                            )
                            .into_any_element(),
                        caption.clone(),
                        plan_lines.clone(),
                        cx,
                    )
                })
                .into_any_element()
        }
        Preview::Text(excerpt) => {
            let text = excerpt.text.clone();
            let trigger = frame.items_start().justify_start().p_1().child(
                div()
                    .size_full()
                    .text_xs()
                    .font_family(theme.mono_font_family.clone())
                    .line_clamp(ROW_LINES)
                    .child(text.clone()),
            );
            let caption = caption_of(intake);
            let excerpt = excerpt.clone();
            HoverCard::new(("preview", row.id))
                .anchor(Corner::TopLeft)
                .open_delay(HOVER_DELAY)
                .trigger(trigger)
                .content(move |_, _, cx| {
                    large_card(
                        large_excerpt(&excerpt, cx),
                        caption.clone(),
                        plan_lines.clone(),
                        cx,
                    )
                })
                .into_any_element()
        }
    }
}

/// The excerpt at reading size, with a line saying it was cut when it
/// was.
fn large_excerpt(excerpt: &Excerpt, cx: &App) -> AnyElement {
    let theme = cx.theme();
    v_flex()
        .w(LARGE.width)
        .max_h(LARGE.height)
        .gap_1()
        .child(
            div()
                .id("excerpt")
                .flex_1()
                .min_h(px(0.0))
                .overflow_y_scroll()
                .p_2()
                .rounded(theme.radius)
                .bg(theme.muted)
                .text_xs()
                .font_family(theme.mono_font_family.clone())
                .child(excerpt.text.clone()),
        )
        .when(excerpt.more, |card| {
            card.child(div().text_xs().text_color(theme.muted_foreground).child(
                SharedString::from(t_args(
                    Message::QueuePreviewCut,
                    &args!("count" => crate::preview::EXCERPT_CHARS),
                )),
            ))
        })
        .into_any_element()
}

/// The hover card: the larger preview, a caption naming the thing, and
/// what would happen to it.
fn large_card(
    preview: AnyElement,
    caption: SharedString,
    plan_lines: Vec<String>,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    v_flex()
        .gap_2()
        .child(preview)
        .child(div().text_xs().font_semibold().truncate().child(caption))
        .children(plan_lines.into_iter().map(|line| {
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(SharedString::from(line))
        }))
        .into_any_element()
}

/// Name, format, encoding and size on one line, for the hover card.
fn caption_of(intake: &Intake) -> SharedString {
    let mut parts = vec![wording::title_of(intake)];
    parts.extend(intake.format.map(|format| format.name().to_owned()));
    parts.extend(intake.encoding.map(|encoding| encoding.name().to_owned()));
    parts.extend(intake.size.map(drop::size_label));
    SharedString::from(parts.join(" · "))
}

/// Where GPUI gets the picture from. The one place a [`Picture`] becomes
/// an `ImageSource` — see that type for why it is not one already.
fn image_source(picture: &Picture) -> ImageSource {
    match picture {
        Picture::File(path) => path.clone().into(),
        Picture::Bytes(image) => image.clone().into(),
    }
}

/// The glyph a kind wears when there is no picture of it.
fn kind_glyph(kind: Kind) -> IconName {
    match kind {
        Kind::Text | Kind::Document => IconName::FileLines,
        Kind::Image => IconName::Image,
        Kind::Archive => IconName::BoxArchive,
        Kind::Media => IconName::Film,
        Kind::Data => IconName::HardDrive,
        Kind::Folder => IconName::FolderOpen,
        Kind::Unknown => IconName::CircleQuestion,
    }
}

/// The badge a kind wears — lazy-shot's type badge, one colour per
/// kind so a column of them can be scanned without reading.
fn kind_tag(kind: Kind) -> Tag {
    let tag = match kind {
        Kind::Text => Tag::color(ColorName::Blue),
        Kind::Image => Tag::color(ColorName::Purple),
        Kind::Document => Tag::color(ColorName::Amber),
        Kind::Archive => Tag::color(ColorName::Gray),
        Kind::Media => Tag::color(ColorName::Rose),
        Kind::Data => Tag::color(ColorName::Cyan),
        Kind::Folder => Tag::color(ColorName::Green),
        Kind::Unknown => Tag::secondary(),
    };
    tag.small()
        .child(SharedString::from(wording::kind_label(kind)))
}

/// The row's life as a badge, and the sentence behind it as its tooltip:
/// waiting (or, for a thing that cannot be cleaned, why not), queued,
/// cleaning, or what the clean came to.
fn status_cell(row: &Row) -> AnyElement {
    let (tag, word, sentence) = match &row.status {
        Status::Waiting => match row.cleanable() {
            Cleanable::No(unable) => (
                Tag::secondary().outline(),
                t(Message::QueueStatusUnable),
                wording::unable(unable),
            ),
            _ => (
                Tag::secondary(),
                t(Message::QueueStatusWaiting),
                t(Message::QueueStatusWaitingTooltip),
            ),
        },
        Status::Queued => (
            Tag::info(),
            t(Message::QueueStatusQueued),
            t(Message::QueueStatusQueuedTooltip),
        ),
        Status::Cleaning => (
            Tag::primary(),
            t(Message::QueueStatusCleaning),
            t(Message::QueueStatusCleaningTooltip),
        ),
        Status::Done(outcome) => {
            let (word, badge) = wording::verdict_badge(&outcome.verdict);
            let tag = match badge {
                Badge::Muted => Tag::secondary(),
                Badge::Success => Tag::success(),
                Badge::Warning => Tag::warning(),
                Badge::Danger => Tag::danger(),
            };
            (tag, t(word), wording::said(outcome))
        }
    };
    let sentence = SharedString::from(sentence);
    div()
        .id(("queue-status", row.id))
        .child(tag.small().child(SharedString::from(word)))
        .tooltip(move |window, cx| Tooltip::new(sentence.clone()).build(window, cx))
        .into_any_element()
}

/// A value with a copy button beside it — lazy-shot's id and keyword
/// cells, where clicking copies and a tick says so for a moment.
/// `None` is a dash and no button: there is nothing to copy.
fn copyable(id: (&'static str, u64), value: Option<String>, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let Some(value) = value else {
        return div()
            .text_sm()
            .text_color(theme.muted_foreground)
            .child(SharedString::from("—"))
            .into_any_element();
    };
    let shown = SharedString::from(value.clone());
    h_flex()
        .gap_1()
        .min_w(px(0.0))
        .child(div().text_sm().truncate().child(shown))
        .child(
            Clipboard::new(id)
                .value(value)
                .tooltip(SharedString::from(t(Message::QueueCopy))),
        )
        .into_any_element()
}

/// The name, and under it what is worth knowing about the name: the
/// folder it is in, and the evidence note when there is one — or, once
/// the row is cleaned, where the result went.
fn name_cell(row: &Row, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let intake = &row.arrival.intake;
    let folder = match intake.arrived {
        Arrived::AsPath | Arrived::AsText => intake
            .path
            .as_deref()
            .and_then(std::path::Path::parent)
            .map(|parent| parent.display().to_string())
            .filter(|folder| !folder.is_empty()),
        Arrived::AsBytes => None,
    };
    let note = match row.outcome() {
        Some(outcome) => Some((wording::went(outcome).join(" · "), Tone::Muted)),
        None => wording::evidence_note(intake),
    };

    v_flex()
        .w_full()
        .min_w(px(0.0))
        .gap_0p5()
        .child(
            div()
                .text_sm()
                .font_semibold()
                .truncate()
                .child(SharedString::from(wording::title_of(intake))),
        )
        .children(folder.map(|folder| {
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .truncate()
                .child(SharedString::from(folder))
        }))
        .children(note.map(|(note, tone)| {
            h_flex()
                .gap_1()
                .items_center()
                .text_xs()
                .text_color(match tone {
                    Tone::Warning => theme.warning,
                    Tone::Muted => theme.muted_foreground,
                })
                .child(Icon::new(IconName::CircleInfo).small())
                .child(div().truncate().child(SharedString::from(note)))
        }))
        .into_any_element()
}

/// The container and, under it, how the characters are stored.
fn format_cell(intake: &Intake, cx: &App) -> AnyElement {
    let theme = cx.theme();
    v_flex()
        .min_w(px(0.0))
        .gap_0p5()
        .child(
            div().text_sm().truncate().child(SharedString::from(
                intake
                    .format
                    .map_or_else(|| "—".to_owned(), |format| format.name().to_owned()),
            )),
        )
        .children(intake.encoding.map(|encoding| {
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(SharedString::from(encoding.name().to_owned()))
        }))
        .into_any_element()
}

/// What a row's Actions menu needs to know to grey an item, read off
/// the row once per frame — nothing big: a cleaned text or a pasted
/// picture is looked up at click time.
struct Actions {
    id: u64,
    /// The file behind the row, for "Open with the default app".
    path: Option<PathBuf>,
    comparable: bool,
    /// `None` when Clean can be pressed; otherwise why not.
    clean: Option<String>,
    /// The result on disk, for Open the result and Show it in its folder.
    written: Option<PathBuf>,
    /// Whether there is a cleaned text to copy.
    text: bool,
    /// Whether the clean was refused over an existing result.
    replace: bool,
    /// Whether there is a finished clean to report.
    report: bool,
}

/// The Actions menu.
///
/// Clean comes first, and is greyed — with the reason under it, because
/// a menu item has no tooltip — for a row that cannot be cleaned, is in
/// line or is done. "Open with the default app" hands the path to the
/// operating system — `App::open_with_system` is `open` on macOS and
/// `xdg-open` on Linux, on a background task — and is disabled for a
/// row with no file behind it: a note pasted from a chat has nothing to
/// open. "Compare with the result" opens the Compare window on the row,
/// and is disabled for a row that is not text. Then the result: open
/// it, show it in its folder, copy it when it came back as text, and
/// "Replace the existing result" when the clean refused to write over a
/// file already there. Disabled rather than absent, every one, so the
/// menu is the same shape on every row and the reason an item does
/// nothing is that it is greyed, not that it moved. A double click on
/// the row is the Compare item taken the short way — [`row_frame`].
fn actions_cell(actions: Actions, queue: Entity<Queue>) -> AnyElement {
    let id = actions.id;
    Button::new(("queue-actions", id))
        .ghost()
        .xsmall()
        .icon(IconName::Ellipsis)
        .tooltip(SharedString::from(t(Message::QueueActions)))
        .dropdown_menu_with_anchor(Corner::TopRight, move |menu, _, _| {
            let path = actions.path.clone();
            let written = actions.written.clone();
            let revealed = actions.written.clone();
            let (cleaning, comparing, copying, replacing, reporting) = (
                queue.clone(),
                queue.clone(),
                queue.clone(),
                queue.clone(),
                queue.clone(),
            );
            let clean = match actions.clean.clone() {
                None => PopupMenuItem::new(SharedString::from(t(Message::QueueActionClean)))
                    .icon(IconName::Broom)
                    .on_click(move |_, _, cx| {
                        cleaning.update(cx, |queue, cx| queue.clean(&[id], cx));
                    }),
                Some(why) => {
                    let why = SharedString::from(why);
                    PopupMenuItem::element(move |_, cx| {
                        v_flex()
                            .max_w(px(280.0))
                            .child(SharedString::from(t(Message::QueueActionClean)))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(why.clone()),
                            )
                    })
                    .icon(IconName::Broom)
                    .disabled(true)
                }
            };
            menu.item(clean)
                .item(
                    PopupMenuItem::new(SharedString::from(t(Message::QueueActionOpen)))
                        .icon(IconName::ArrowUpRightFromSquare)
                        .disabled(path.is_none())
                        .on_click(move |_, _, cx| {
                            if let Some(path) = &path {
                                tracing::info!("opening a queued file with the system");
                                cx.open_with_system(path);
                            }
                        }),
                )
                .item(
                    PopupMenuItem::new(SharedString::from(t(Message::QueueActionCompare)))
                        .icon(IconName::CodeCompare)
                        .disabled(!actions.comparable)
                        .on_click(move |_, window, cx| {
                            compare_row(id, comparing.clone(), window, cx);
                        }),
                )
                .separator()
                .item(
                    PopupMenuItem::new(SharedString::from(t(Message::QueueActionOpenResult)))
                        .icon(IconName::ArrowUpRightFromSquare)
                        .disabled(written.is_none())
                        .on_click(move |_, _, cx| {
                            if let Some(written) = &written {
                                tracing::info!(id, "opening a result with the system");
                                cx.open_with_system(written);
                            }
                        }),
                )
                .item(
                    PopupMenuItem::new(SharedString::from(t(Message::QueueActionRevealResult)))
                        .icon(IconName::FolderOpen)
                        .disabled(revealed.is_none())
                        .on_click(move |_, _, cx| {
                            if let Some(written) = &revealed {
                                cx.reveal_path(written);
                            }
                        }),
                )
                .item(
                    PopupMenuItem::new(SharedString::from(t(Message::QueueActionCopyResult)))
                        .icon(IconName::Copy)
                        .disabled(!actions.text)
                        .on_click(move |_, _, cx| {
                            // The person's own text, cleaned: no catalogue
                            // touches it on the way to the clipboard.
                            if let Some(text) = copying.read(cx).result_text(id) {
                                cx.write_to_clipboard(ClipboardItem::new_string(text));
                            }
                        }),
                )
                .item(
                    PopupMenuItem::new(SharedString::from(t(Message::QueueActionReport)))
                        .icon(IconName::FileLines)
                        .disabled(!actions.report)
                        .on_click(move |_, _, cx| {
                            // Deferred, for the reason `compare_row` is: the
                            // shell builds the dialog in this window, and a
                            // click runs inside this window's update.
                            let reporting = reporting.clone();
                            cx.defer(move |cx| {
                                reporting.update(cx, |_, cx| cx.emit(QueueEvent::Report(id)));
                            });
                        }),
                )
                .item(
                    PopupMenuItem::new(SharedString::from(t(Message::QueueActionReplace)))
                        .icon(IconName::Replace)
                        .disabled(!actions.replace)
                        .on_click(move |_, _, cx| {
                            replacing.update(cx, |queue, cx| queue.replace(id, cx));
                        }),
                )
        })
        .into_any_element()
}

/// Open the Compare window on row `id`, from a click in this window —
/// the one road both the Actions item and the double click go down,
/// so the two cannot open a window on two different things.
///
/// Deferred, for the reason `SetupEvent::Open` defers: a click runs
/// inside this window's update, and `compare::open` measures this
/// window to centre the new one over it, which from in here comes back
/// "not found". The row is looked up again at click time rather than
/// cloned into every menu — a pasted screenshot is megabytes with no
/// file behind it, and the menu is rebuilt on every frame.
fn compare_row(id: u64, queue: Entity<Queue>, window: &Window, cx: &mut App) {
    let main = window.window_handle();
    cx.defer(move |cx| {
        let Some(subject) = queue.read(cx).subject_of(id) else {
            tracing::warn!(id, "the row to compare is gone");
            return;
        };
        let comparison = queue.read(cx).comparison(cx);
        tracing::info!(id, "opening the Compare window on a queued row");
        compare::open(Some(id), subject, comparison, main, cx);
    });
}

/// The strip a row's cells go into, and what a click on it means.
///
/// A **double click** opens the Compare window on the row — the
/// desktop's own idiom for "open this" in a table, and the Actions
/// menu's item without the menu. `open` is the item's work handed in,
/// and `None` on a row that is not text, where the double click does
/// what the greyed item does: nothing. A single click means nothing
/// either — no row is selected here, so there is nothing for one to
/// do — and it is the second click of the pair that opens, never the
/// third of a triple. The cells' own controls keep their clicks: the
/// copy button and the Actions button both stop the event where it
/// lands, so a quick pair of presses on either copies twice or opens
/// the menu and does not open a window as well.
/// `a_double_click_on_the_row_opens_and_a_single_one_does_not` is the
/// gate on this element; what it does not gate is which row the
/// window is opened on, which is `compare_row`'s.
fn row_frame(id: u64, open: Option<impl Fn(&mut Window, &mut App) + 'static>) -> Stateful<Div> {
    h_flex()
        .id(("queue-row", id))
        .w_full()
        .h(ROW)
        .when_some(open, |row, open| {
            row.on_double_click(move |_, window, cx| open(window, cx))
        })
}

impl Queue {
    /// The Compare page's rows as they stand — what a window opened
    /// from here is opened with.
    pub fn comparison(&self, cx: &App) -> Comparison {
        self.preferences.read(cx).comparison()
    }

    /// What the Compare window is opened on for row `id`, if the row
    /// is still here: the thing as it arrived and what it was
    /// established to be, so the window need not examine it again.
    pub fn subject_of(&self, id: u64) -> Option<Subject> {
        self.rows
            .iter()
            .find(|row| row.id == id)
            .map(|row| Subject {
                handed: row.arrival.handed.clone(),
                intake: Some(row.arrival.intake.clone()),
            })
    }
}

impl Queue {
    /// The filter bar: the two fields, "Reset filters" while either is
    /// set, and the count of what they let through.
    fn filter_bar(&self, visible: usize, cx: &Context<Self>) -> Div {
        let theme = cx.theme();
        let total = self.rows.len();
        let count = if self.filter.is_active() {
            t_args(
                Message::QueueCountFiltered,
                &args!("count" => visible, "total" => total),
            )
        } else {
            // The number itself, not its spelling: Fluent picks the
            // plural form from a number and never from a string.
            t_args(Message::QueueCount, &args!("count" => total))
        };

        h_flex()
            .w_full()
            .flex_shrink_0()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(theme.border)
            .child(div().w(px(128.0)).child(Input::new(&self.by_id).small()))
            .child(
                div()
                    .w(px(192.0))
                    .child(Input::new(&self.by_keyword).small()),
            )
            .when(self.filter.is_active(), |bar| {
                bar.child(
                    Button::new("reset-filters")
                        .small()
                        .ghost()
                        .icon(IconName::RotateLeft)
                        .label(SharedString::from(t(Message::QueueResetFilters)))
                        .on_click(cx.listener(|queue, _, window, cx| {
                            queue.reset_filters(window, cx);
                        })),
                )
            })
            .child(div().flex_1())
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(SharedString::from(count)),
            )
    }

    /// The header row: one title per column, in the column's own cell
    /// so that the titles sit over the values. The Arrived cell is the
    /// one control in it: clicking flips the order, and the chevron
    /// says which way it currently runs.
    fn header(&self, cx: &Context<Self>) -> Div {
        let theme = cx.theme();
        h_flex()
            .w_full()
            .h(px(32.0))
            .flex_shrink_0()
            .bg(theme.table_head)
            .text_color(theme.table_head_foreground)
            .border_b_1()
            .border_color(theme.table_row_border)
            .text_xs()
            .font_semibold()
            .children(Column::ALL.map(|column| {
                let cell = column.cell();
                let title = SharedString::from(t(column.title()));
                match column {
                    Column::Arrived => cell.child(
                        Button::new("sort-arrived")
                            .xsmall()
                            .ghost()
                            .label(title)
                            .icon(self.order.glyph())
                            .tooltip(SharedString::from(t(self.order.next_label())))
                            .on_click(cx.listener(|queue, _, _, cx| queue.flip_order(cx))),
                    ),
                    _ => cell.child(title),
                }
            }))
    }

    /// One row, by its index in the list — called by the page's list
    /// for the rows on screen.
    fn row(&self, index: usize, cx: &Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let row = &self.rows[index];
        let intake = &row.arrival.intake;
        // Once cleaned, what happened; until then, what would.
        let lines = match row.outcome() {
            Some(outcome) => {
                let mut lines = vec![wording::said(outcome)];
                lines.extend(wording::went(outcome));
                lines
            }
            None => wording::would_happen(&self.preferences.read(cx).plan_for(intake)),
        };

        let comparable = Subject::comparable(intake);
        let open = comparable.then(|| {
            let id = row.id;
            let queue = cx.entity();
            move |window: &mut Window, cx: &mut App| compare_row(id, queue.clone(), window, cx)
        });

        row_frame(row.id, open)
            .border_b_1()
            .border_color(theme.table_row_border)
            .hover(|row| row.bg(theme.table_hover))
            .child(Column::Preview.cell().child(preview_cell(row, lines, cx)))
            .child(Column::Id.cell().child(copyable(
                ("copy-id", row.id),
                Some(row.id.to_string()),
                cx,
            )))
            .child(Column::Keyword.cell().child(copyable(
                ("copy-keyword", row.id),
                row.keyword.clone(),
                cx,
            )))
            .child(Column::Name.cell().child(name_cell(row, cx)))
            .child(Column::Kind.cell().child(kind_tag(intake.kind)))
            .child(Column::Status.cell().child(status_cell(row)))
            .child(Column::Format.cell().child(format_cell(intake, cx)))
            .child(Column::Size.cell().text_sm().child(SharedString::from(
                intake.size.map_or_else(|| "—".to_owned(), drop::size_label),
            )))
            .child(
                Column::Arrived
                    .cell()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(SharedString::from(
                        row.arrived_at.format("%H:%M").to_string(),
                    )),
            )
            .child(Column::Actions.cell().child(actions_cell(
                Actions {
                    id: row.id,
                    path: intake.path.clone(),
                    comparable,
                    clean: why_not_clean(&row.status, row.cleanable()),
                    written: row.outcome().and_then(|outcome| outcome.written.clone()),
                    text: row.outcome().is_some_and(|outcome| outcome.text.is_some()),
                    replace: row.outcome().and_then(existing_result).is_some(),
                    report: row.outcome().is_some(),
                },
                cx.entity(),
            )))
            .into_any_element()
    }

    /// The table with nothing in it: the invitation, and — while
    /// something is held over the window — the release line. Or, with
    /// rows behind a filter that lets none through, a line saying so
    /// rather than an invitation that would read as "the list is empty".
    fn empty(&self, over: bool, cx: &App) -> Div {
        let theme = cx.theme();
        let (glyph, line) = if self.rows.is_empty() {
            (
                IconName::FileImport,
                if over {
                    Message::QueueEmptyRelease
                } else {
                    Message::QueueEmptyInvite
                },
            )
        } else {
            (IconName::MagnifyingGlass, Message::QueueEmptyFiltered)
        };
        v_flex()
            .flex_1()
            .min_h(px(0.0))
            .items_center()
            .justify_center()
            .gap_2()
            .text_center()
            .child(Icon::new(glyph).large().color(theme.muted_foreground))
            .child(div().text_sm().child(SharedString::from(t(line))))
    }

    /// The line under the table: that rewriting with a model is not in
    /// the windows and where it does run, in the words every pending
    /// surface uses — and lazy-shot's paginator at the right.
    fn footer(&self, visible: usize, cx: &Context<Self>) -> Div {
        let theme = cx.theme();
        let pages = pages(visible, self.page_size);
        let page = self.page.min(pages - 1);

        h_flex()
            .w_full()
            .flex_shrink_0()
            .px_3()
            .py_1p5()
            .gap_3()
            .items_center()
            .border_t_1()
            .border_color(theme.table_row_border)
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .truncate()
                    .child(SharedString::from(t(Message::QueuePending))),
            )
            .child(
                Button::new("page-size")
                    .xsmall()
                    .ghost()
                    .label(SharedString::from(t_args(
                        Message::QueuePageSize,
                        &args!("count" => self.page_size),
                    )))
                    .icon(IconName::ChevronDown)
                    .dropdown_menu_with_anchor(Corner::BottomRight, {
                        let queue = cx.entity();
                        let current = self.page_size;
                        move |mut menu, _, _| {
                            for size in PAGE_SIZES {
                                let queue = queue.clone();
                                menu = menu.item(
                                    PopupMenuItem::new(SharedString::from(t_args(
                                        Message::QueuePageSize,
                                        &args!("count" => size),
                                    )))
                                    .checked(size == current)
                                    .on_click(
                                        move |_, _, cx| {
                                            queue.update(cx, |queue, cx| {
                                                queue.set_page_size(size, cx);
                                            });
                                        },
                                    ),
                                );
                            }
                            menu
                        }
                    }),
            )
            .child(
                Button::new("previous-page")
                    .xsmall()
                    .ghost()
                    .icon(IconName::ChevronLeft)
                    .label(SharedString::from(t(Message::QueuePrevious)))
                    .disabled(page == 0)
                    .on_click(cx.listener(|queue, _, _, cx| queue.turn_page(false, cx))),
            )
            .child(SharedString::from(t_args(
                Message::QueuePage,
                &args!("page" => page + 1, "pages" => pages),
            )))
            .child(
                Button::new("next-page")
                    .xsmall()
                    .ghost()
                    .label(SharedString::from(t(Message::QueueNext)))
                    .icon(IconName::ChevronRight)
                    .disabled(page + 1 >= pages)
                    .on_click(cx.listener(|queue, _, _, cx| queue.turn_page(true, cx))),
            )
    }
}

impl Render for Queue {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let over = self.catcher.read(cx).over();
        let visible = self.visible();
        let on_page: Vec<usize> =
            visible[page_range(visible.len(), self.page_size, self.page)].to_vec();

        // The whole table is the drop zone — see the module docs. The
        // highlight `drop::zone` paints is the table's background going
        // to the accent, which reads as "this will take it" without a
        // second element in the way of the rows.
        let table = drop::zone(v_flex(), &self.catcher, cx)
            .size_full()
            .min_h(px(0.0))
            .bg(theme.table)
            .child(self.filter_bar(visible.len(), cx))
            .child(self.header(cx));

        let table = if on_page.is_empty() {
            table.child(self.empty(over, cx))
        } else {
            let count = on_page.len();
            table.child(
                div()
                    .relative()
                    .flex_1()
                    .min_h(px(0.0))
                    .child(
                        uniform_list(
                            "queue-rows",
                            count,
                            cx.processor(move |queue, range: Range<usize>, _, cx| {
                                on_page[range]
                                    .iter()
                                    .map(|&index| queue.row(index, cx))
                                    .collect()
                            }),
                        )
                        .size_full()
                        .track_scroll(&self.scroll),
                    )
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .right_0()
                            .bottom_0()
                            .w(SCROLLBAR)
                            .child(Scrollbar::vertical(&self.scroll)),
                    ),
            )
        };

        table.child(self.footer(visible.len(), cx))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use gpui::prelude::*;
    use gpui::{
        div, point, px, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, TestAppContext,
        VisualTestContext,
    };
    use wipemark_i18n::{t, Message};
    use wipemark_intake::Kind;

    use super::{
        kind_glyph, kind_tag, page_range, pages, row_frame, shown, why_not_clean, Cleanable,
        Column, Filter, Job, Line, Order, Queue, Status, PAGE_SIZES, ROW,
    };
    use crate::clean::Verdict;
    use crate::retention::{Destination, Homes};
    use crate::settings::Preferences;

    /// A column added to the list is a column with a title: a header
    /// cell that rendered a catalogue key would say so on every launch.
    #[test]
    fn every_column_has_a_title() {
        for column in Column::ALL {
            let title = t(column.title());
            assert!(!title.is_empty(), "{column:?}");
            assert!(
                !title.starts_with("queue-column"),
                "{column:?} rendered its key: {title}"
            );
        }
        // Exactly one column takes the slack, or the row either
        // overflows or leaves a gap.
        assert_eq!(
            Column::ALL
                .iter()
                .filter(|column| column.width().is_none())
                .count(),
            1
        );
    }

    /// The honesty line under the table names what is still missing —
    /// rewriting with a model — in the words the other pending surfaces
    /// use, and no longer says the list does not clean: it does. Never an
    /// epic number, which a person reading a window can do nothing with.
    #[test]
    fn the_footer_says_rewriting_is_not_here_yet() {
        let line = t(Message::QueuePending);
        assert!(line.contains("not in this version"), "{line}");
        assert!(line.contains("Rewriting"), "{line}");
        assert!(
            !line
                .to_lowercase()
                .contains("cleaning from this list is not"),
            "the footer still says the list does not clean: {line}"
        );
        assert!(
            !line
                .split(|c: char| !c.is_ascii_alphanumeric())
                .any(|word| word.len() > 1
                    && word.starts_with('E')
                    && word[1..].chars().all(|c| c.is_ascii_digit())),
            "an epic number reached a window: {line}"
        );
    }

    /// One clean at a time, first asked first done: a second start while
    /// one runs hands nothing out, and the next is the one asked for
    /// next. The count is of the cleans since the line was last empty.
    #[test]
    fn one_clean_runs_at_a_time_in_the_order_asked() {
        let job = |id| Job {
            id,
            replacing: None,
        };
        let mut line = Line::default();
        assert_eq!(line.start(), None, "an empty line starts nothing");
        assert_eq!(line.progress(), None);

        line.push(job(4));
        line.push(job(2));
        assert_eq!(line.start(), Some(job(4)));
        assert_eq!(
            line.start(),
            None,
            "a second clean started beside the first"
        );
        assert_eq!(line.progress(), Some((1, 2)));

        // Asked for while one runs: behind the one already waiting.
        line.push(job(9));
        assert_eq!(line.progress(), Some((1, 3)));
        line.finish(4);
        assert_eq!(line.start(), Some(job(2)));
        assert_eq!(line.progress(), Some((2, 3)));
        line.finish(2);
        assert_eq!(line.start(), Some(job(9)));
        line.finish(9);
        assert_eq!(line.start(), None);
        assert_eq!(line.progress(), None, "the count starts again");

        line.push(job(10));
        assert_eq!(line.start(), Some(job(10)));
        assert_eq!(line.progress(), Some((1, 1)));
    }

    /// Clean is greyed with a reason for a thing that cannot be cleaned —
    /// the reason names the format — and for a row in line or done; a
    /// waiting text or picture can be cleaned.
    #[test]
    fn clean_is_greyed_with_a_reason_when_it_cannot_run() {
        use wipemark_intake::Format;

        use crate::clean::Unable;

        let tiff = Cleanable::No(Unable::NotYet(Format::Tiff));
        let text = Cleanable::Text(wipemark_intake::Encoding::Utf8);
        let why = why_not_clean(&Status::Waiting, tiff).expect("a TIFF cannot be cleaned");
        assert!(why.contains("TIFF"), "{why}");
        assert_eq!(why_not_clean(&Status::Waiting, text), None);
        assert_eq!(
            why_not_clean(&Status::Waiting, Cleanable::Picture(Format::Png)),
            None
        );
        for status in [Status::Queued, Status::Cleaning] {
            assert_eq!(
                why_not_clean(&status, text),
                Some(t(Message::QueueActionCleanBusy))
            );
        }
    }

    /// Every kind has a glyph for the row with no picture and a badge
    /// for the Kind column; a match with a wildcard would let a new
    /// kind through with neither.
    #[test]
    fn every_kind_has_a_glyph_and_a_badge() {
        for kind in [
            Kind::Text,
            Kind::Image,
            Kind::Document,
            Kind::Archive,
            Kind::Media,
            Kind::Data,
            Kind::Folder,
            Kind::Unknown,
        ] {
            let _ = kind_glyph(kind);
            let _ = kind_tag(kind);
        }
    }

    fn rows() -> Vec<(u64, Option<&'static str>)> {
        vec![
            (1, Some("invoice")),
            (2, None),
            (12, Some("Report")),
            (120, Some("invoice-2")),
        ]
    }

    /// Newest first is the default and the reverse of arrival, and the
    /// other way is arrival itself: nothing is sorted, so nothing can
    /// be sorted wrongly.
    #[test]
    fn the_newest_row_is_on_top_until_the_header_is_clicked() {
        let rows = rows();
        let none = Filter::default();
        assert_eq!(shown(&rows, &none, Order::NewestFirst), vec![3, 2, 1, 0]);
        assert_eq!(shown(&rows, &none, Order::OldestFirst), vec![0, 1, 2, 3]);
        assert_eq!(Order::NewestFirst.flipped(), Order::OldestFirst);
        assert_eq!(Order::OldestFirst.flipped(), Order::NewestFirst);
    }

    /// lazy-shot's `id LIKE '%12%'`: three remembered digits find every
    /// id that contains them, in the order on screen.
    #[test]
    fn the_id_filter_is_a_substring() {
        let rows = rows();
        let filter = Filter {
            id: " 12 ".to_owned(),
            keyword: String::new(),
        };
        assert_eq!(shown(&rows, &filter, Order::OldestFirst), vec![2, 3]);
        assert_eq!(shown(&rows, &filter, Order::NewestFirst), vec![3, 2]);
        assert!(filter.is_active());
        assert!(!Filter::default().is_active());
        assert!(
            !Filter {
                id: "   ".to_owned(),
                keyword: String::new()
            }
            .is_active(),
            "whitespace is not a filter"
        );
    }

    /// lazy-shot's `key_word LIKE '%…%'`, case-insensitively — and a row
    /// with no keyword never matches one, because no keyword is not a
    /// keyword that happens to be empty.
    #[test]
    fn the_keyword_filter_is_a_substring_and_skips_the_unnamed() {
        let rows = rows();
        let filter = Filter {
            id: String::new(),
            keyword: "INVOICE".to_owned(),
        };
        assert_eq!(shown(&rows, &filter, Order::OldestFirst), vec![0, 3]);

        let both = Filter {
            id: "2".to_owned(),
            keyword: "invoice".to_owned(),
        };
        assert_eq!(shown(&rows, &both, Order::OldestFirst), vec![3]);

        let nothing = Filter {
            id: String::new(),
            keyword: "zzz".to_owned(),
        };
        assert!(shown(&rows, &nothing, Order::OldestFirst).is_empty());
    }

    /// The paginator's arithmetic: never zero pages, a page past the end
    /// is the last page, and the sizes on offer are lazy-shot's four.
    #[test]
    fn a_page_is_never_empty_while_there_are_rows() {
        assert_eq!(pages(0, 20), 1);
        assert_eq!(pages(20, 20), 1);
        assert_eq!(pages(21, 20), 2);
        assert_eq!(pages(100, 0), 100, "a zero page size is treated as one");

        assert_eq!(page_range(45, 20, 0), 0..20);
        assert_eq!(page_range(45, 20, 2), 40..45);
        assert_eq!(
            page_range(45, 20, 7),
            40..45,
            "a page the list shrank out from under is the last page"
        );
        assert_eq!(page_range(0, 20, 0), 0..0);

        assert!(PAGE_SIZES.contains(&super::DEFAULT_PAGE_SIZE));
        assert!(PAGE_SIZES.windows(2).all(|pair| pair[0] < pair[1]));
    }

    /// One row's frame in a window of its own, counting how often it
    /// was asked to open.
    struct Framed(Rc<Cell<u32>>);

    impl Render for Framed {
        fn render(
            &mut self,
            _: &mut gpui::Window,
            _: &mut gpui::Context<Self>,
        ) -> impl IntoElement {
            let opened = self.0.clone();
            div().size_full().child(row_frame(
                7,
                Some(move |_: &mut gpui::Window, _: &mut gpui::App| {
                    opened.set(opened.get() + 1);
                }),
            ))
        }
    }

    /// A press and its release at `at`, as the `count`th of a series —
    /// what the platform hands GPUI for the second of a double click.
    fn click(cx: &mut VisualTestContext, at: gpui::Point<gpui::Pixels>, count: usize) {
        cx.simulate_event(MouseDownEvent {
            position: at,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: count,
            first_mouse: false,
        });
        cx.simulate_event(MouseUpEvent {
            position: at,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: count,
        });
        cx.run_until_parked();
    }

    /// The frame opens on the second click of a pair and on nothing
    /// else: not the first, not the third of a triple. Without the
    /// handler on the frame — or with `>= 2` in place of `== 2` — this
    /// goes red.
    #[gpui::test]
    fn a_double_click_on_the_row_opens_and_a_single_one_does_not(cx: &mut TestAppContext) {
        let opened = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let opened = opened.clone();
            move |_, _| Framed(opened)
        });
        let inside = point(px(40.0), ROW / 2.0);

        click(cx, inside, 1);
        assert_eq!(opened.get(), 0, "a single click opened the window");

        click(cx, inside, 1);
        click(cx, inside, 2);
        assert_eq!(opened.get(), 1, "the double click did not open the window");

        click(cx, inside, 3);
        assert_eq!(
            opened.get(),
            1,
            "the third click of a triple opened it again"
        );

        // Below the row is not the row.
        click(cx, point(px(40.0), ROW * 2.0), 1);
        click(cx, point(px(40.0), ROW * 2.0), 2);
        assert_eq!(opened.get(), 1, "a double click beside the row opened it");
    }

    /// A scratch directory that takes its own files away with it.
    struct Scratch(std::path::PathBuf);

    impl Scratch {
        fn new(label: &str) -> Self {
            let directory =
                std::env::temp_dir().join(format!("wipemark-queue-{label}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&directory);
            std::fs::create_dir_all(directory.join("results")).expect("scratch directory");
            Self(directory)
        }

        fn file(&self, name: &str, bytes: &[u8]) -> std::path::PathBuf {
            let path = self.0.join(name);
            std::fs::write(&path, bytes).expect("scratch file");
            path
        }

        fn homes(&self) -> Homes {
            Homes {
                results: self.0.join("results"),
                kept: self.0.join("kept"),
            }
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).ok();
        }
    }

    /// A main window's queue over preferences that forget, with its two
    /// folders in `scratch`.
    /// The queue and the preferences it reads, as a window holds them.
    type Held = (gpui::Entity<Queue>, gpui::Entity<Preferences>);

    fn queue_in<'a>(
        cx: &'a mut TestAppContext,
        scratch: &Scratch,
    ) -> (
        gpui::Entity<Queue>,
        gpui::Entity<Preferences>,
        &'a mut VisualTestContext,
    ) {
        cx.update(gpui_component::init);
        let homes = scratch.homes();
        let slot: Rc<std::cell::RefCell<Option<Held>>> = Rc::default();
        let held = slot.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let preferences = cx.new(|cx| Preferences::for_tests(homes, cx));
            // Detached: a test window has no platform window for the
            // macOS drop destination to hang from.
            let catcher = cx.new(|_| crate::drop::Catcher::detached());
            let queue = cx.new(|cx| Queue::with_catcher(preferences.clone(), catcher, window, cx));
            *held.borrow_mut() = Some((queue.clone(), preferences));
            gpui_component::Root::new(queue, window, cx)
        });
        let (queue, preferences) = slot.take().expect("the window builder ran");
        (queue, preferences, cx)
    }

    fn statuses(queue: &gpui::Entity<Queue>, cx: &mut VisualTestContext) -> Vec<&'static str> {
        cx.update(|_, cx| {
            let queue = queue.read(cx);
            queue
                .ids()
                .into_iter()
                .map(|id| match queue.status_of(id) {
                    Some(Status::Waiting) => "waiting",
                    Some(Status::Queued) => "queued",
                    Some(Status::Cleaning) => "cleaning",
                    Some(Status::Done(outcome)) => outcome.verdict.id(),
                    None => "gone",
                })
                .collect()
        })
    }

    const MARKED: &str = "# Notes\n\nA zero\u{200B}width space.\n";

    /// Clean all moves the waiting rows through queued and cleaning to
    /// done, one at a time and in order, skips what cannot be cleaned,
    /// and asks nothing twice; the plan is the one in force when each
    /// row's clean **starts** — a choice changed while the second row
    /// waited applies to it and not to the first — and the status bar's
    /// count runs while the line does.
    #[gpui::test]
    fn the_queue_cleans_one_row_at_a_time_by_the_plan_at_its_start(cx: &mut TestAppContext) {
        let scratch = Scratch::new("cleans");
        let first = scratch.file("a.md", MARKED.as_bytes());
        let second = scratch.file("b.md", MARKED.as_bytes());
        let tiff = scratch.file("c.tiff", b"II*\0\x08\0\0\0");
        let (queue, preferences, cx) = queue_in(cx, &scratch);

        queue.update(cx, |queue, cx| {
            queue.hand(vec![first.clone(), second.clone(), tiff.clone()], cx);
        });
        cx.run_until_parked();
        assert_eq!(statuses(&queue, cx), ["waiting", "waiting", "waiting"]);
        let waiting = cx.update(|_, cx| queue.read(cx).cleanable_waiting().len());
        assert_eq!(waiting, 2, "the TIFF is not a row Clean all cleans");

        queue.update(cx, |queue, cx| queue.clean_all(cx));
        assert_eq!(
            statuses(&queue, cx),
            ["cleaning", "queued", "waiting"],
            "not one at a time, in order"
        );
        assert_eq!(cx.update(|_, cx| queue.read(cx).progress()), Some((1, 2)));
        // Asked again while the line runs — by Clean all, or row by row
        // as `--clean=` and the menu ask: nothing is asked twice.
        queue.update(cx, |queue, cx| queue.clean_all(cx));
        assert_eq!(cx.update(|_, cx| queue.read(cx).progress()), Some((1, 2)));
        let ids = cx.update(|_, cx| queue.read(cx).ids());
        queue.update(cx, |queue, cx| queue.clean(&ids[..2], cx));
        assert_eq!(cx.update(|_, cx| queue.read(cx).progress()), Some((1, 2)));
        assert_eq!(statuses(&queue, cx), ["cleaning", "queued", "waiting"]);

        // The page changes while the second row waits.
        preferences.update(cx, |preferences, cx| {
            preferences.select_destination(Destination::Folder, cx);
        });
        cx.run_until_parked();

        assert_eq!(statuses(&queue, cx), ["cleaned", "cleaned", "waiting"]);
        assert_eq!(cx.update(|_, cx| queue.read(cx).progress()), None);
        // A done row asked again stays done: its result is not written twice.
        queue.update(cx, |queue, cx| queue.clean(&ids[..1], cx));
        assert_eq!(statuses(&queue, cx), ["cleaned", "cleaned", "waiting"]);
        let cleaned = wipemark_core::clean(MARKED, &wipemark_core::Options::default()).text;
        assert_eq!(
            std::fs::read_to_string(scratch.0.join("a.cleaned.md")).expect("beside"),
            cleaned,
            "the first row was not cleaned by the plan at its start"
        );
        assert!(!scratch.0.join("b.cleaned.md").exists());
        assert_eq!(
            std::fs::read_to_string(scratch.0.join("results").join("b.cleaned.md"))
                .expect("into the results folder"),
            cleaned,
            "the second row was not cleaned by the plan at its start"
        );

        // Clean all skips what is not waiting; the TIFF is asked by name
        // and refused, and its badge says why.
        queue.update(cx, |queue, cx| queue.clean_all(cx));
        cx.run_until_parked();
        assert_eq!(statuses(&queue, cx), ["cleaned", "cleaned", "waiting"]);
        queue.update(cx, |queue, cx| queue.clean(&ids[2..], cx));
        cx.run_until_parked();
        assert_eq!(statuses(&queue, cx), ["cleaned", "cleaned", "not-cleaned"]);
    }

    /// `--clean=` is Import and Clean: the row lands and is cleaned. A
    /// result already there is refused and left; Replace writes over that
    /// one file and says so.
    #[gpui::test]
    fn a_refused_result_is_replaced_only_when_asked(cx: &mut TestAppContext) {
        let scratch = Scratch::new("replace");
        let source = scratch.file("x.md", MARKED.as_bytes());
        let existing = scratch.file("x.cleaned.md", b"somebody's own file");
        let (queue, _, cx) = queue_in(cx, &scratch);

        queue.update(cx, |queue, cx| {
            queue.hand_to_clean(vec![source.clone()], cx)
        });
        cx.run_until_parked();
        assert_eq!(statuses(&queue, cx), ["not-cleaned"]);
        assert_eq!(
            std::fs::read(&existing).expect("read"),
            b"somebody's own file"
        );

        let id = cx.update(|_, cx| queue.read(cx).ids()[0]);
        queue.update(cx, |queue, cx| queue.replace(id, cx));
        cx.run_until_parked();
        assert_eq!(statuses(&queue, cx), ["cleaned"]);
        let replaced = cx.update(|_, cx| match queue.read(cx).status_of(id) {
            Some(Status::Done(outcome)) => {
                outcome.replaced && matches!(outcome.verdict, Verdict::Cleaned)
            }
            _ => false,
        });
        assert!(replaced, "the outcome does not say it replaced the file");
        assert_eq!(
            std::fs::read_to_string(&existing).expect("read"),
            wipemark_core::clean(MARKED, &wipemark_core::Options::default()).text
        );
        assert_eq!(std::fs::read(&source).expect("read"), MARKED.as_bytes());

        // Done is done: a second Replace has nothing to replace.
        queue.update(cx, |queue, cx| queue.replace(id, cx));
        assert_eq!(statuses(&queue, cx), ["cleaned"]);
    }
}
