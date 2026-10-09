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
//! never on arrival. One clean runs at a time in the whole application —
//! the panel's cleans wait in the same line — first asked first done
//! ([`crate::cleaner::Line`], D283), on the background executor through
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
//! it.
//!
//! # Rewriting (E4-6b)
//!
//! **Rewrite** in a row's Actions menu and **Rewrite all** on the toolbar
//! push the row to the application's batch queue (`wipemark-queue`): one
//! rewrite at a time for the whole application — a window's, an agent's,
//! the command line's through the application — on the engine on duty
//! when each item **starts** (R1), persisted per chunk, resumed after a
//! crash. Rewrite all says the price first (D61): calls, tokens and, with
//! a measured rate, minutes; a single row's Rewrite starts at once and says
//! its price in the row's tooltip. Where the result goes is the Retention
//! page's plan taken **when the row is pushed** (D91, `name.rewritten.ext`
//! beside the file, В8) — the deliberate opposite of a clean, which takes
//! it when it starts (D283): the queue executes what was stored, after a
//! restart too. A text with no file keeps its result in the queue's row,
//! shown and copied from there until the row is removed. Cleans keep their
//! own line (D283): a picture must not wait behind a twenty-minute
//! rewrite, and a clean asked for a row being rewritten is refused (R7).
//!
//! # The journal (E4-6b)
//!
//! The table **is** the document journal (`wipemark_store::Journal`):
//! every row is a row of it, read back at launch, so the list survives a
//! restart (В3) — a row's own Remove, Clear finished on the toolbar, and
//! the keep period on the Retention page take rows away. Rows other
//! surfaces wrote — the panel, an agent, the command line — appear without
//! a click, and say who asked. A row keeps metadata after its end (В4):
//! a thing with no file behind it cannot be processed again once it ended
//! or the application restarted, and its menu says why.
//!
//! # What it does not do yet
//!
//! Folders and archives are listed
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

use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Local};
use gpui::prelude::*;
use gpui::{
    div, img, px, uniform_list, Anchor, AnyElement, App, ClipboardItem, Context, Div, Entity,
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
use wipemark_queue::ItemId;
use wipemark_store::entry::{Action, Delivered, Entry, Origin, Outcome as Recorded, Phase};

use crate::clean::{self, Cleanable, Outcome, Refusal, Verdict};
use crate::cleaner::{self, Cleaner};
use crate::compare::{self, CleanedTo, Comparison, Home, Made, RewriteFrom, Subject, Told};
use crate::drop::{self, Arrival, Catcher, Landed};
use crate::icon::{Icon, IconName};
use crate::journal::{self, Work, Writer, Written};
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
    /// The row's next steps as buttons — Clean and Rewrite (D325).
    Process,
    Format,
    Size,
    Arrived,
    Actions,
}

impl Column {
    const ALL: [Column; 11] = [
        Column::Preview,
        Column::Id,
        Column::Keyword,
        Column::Name,
        Column::Kind,
        Column::Status,
        Column::Process,
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
            Column::Process => Message::QueueColumnProcess,
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
            Column::Keyword => Some(px(96.0)),
            Column::Name => None,
            Column::Kind => Some(px(124.0)),
            Column::Status => Some(px(156.0)),
            Column::Process => Some(px(176.0)),
            Column::Format => Some(px(112.0)),
            Column::Size => Some(px(88.0)),
            // Wide enough for a date before the time (D363).
            Column::Arrived => Some(px(128.0)),
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
            None => cell.flex_1().flex_shrink_1(),
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

/// What happens to a thing as it arrives in the main window — the General
/// page's "Process what arrives" (В1). Nothing by default: a drop is a
/// row, and a button asks for the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OnArrival {
    #[default]
    Nothing,
    Clean,
    Rewrite,
}

impl OnArrival {
    pub const ALL: [OnArrival; 3] = [Self::Nothing, Self::Clean, Self::Rewrite];

    /// The stored value. A format.
    pub fn id(self) -> &'static str {
        match self {
            Self::Nothing => "nothing",
            Self::Clean => "clean",
            Self::Rewrite => "rewrite",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|choice| choice.id() == value.trim())
    }

    /// What the page calls it.
    pub fn label(self) -> Message {
        match self {
            Self::Nothing => Message::SettingsArrivalNothing,
            Self::Clean => Message::SettingsArrivalClean,
            Self::Rewrite => Message::SettingsArrivalRewrite,
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
    /// In the batch queue, waiting its turn — or held for want of an
    /// engine, or paused, which the queue says (E4-6b).
    RewriteQueued,
    /// Being rewritten: which paragraph of how many, once the job says.
    Rewriting(Option<(u32, u32)>),
    /// What the journal says: a rewrite's end, an earlier session's row,
    /// another surface's.
    Recorded(Box<Said>),
}

/// A status as the journal says it.
#[derive(Debug, Clone, PartialEq)]
pub struct Said {
    pub action: Action,
    pub phase: Phase,
    pub outcome: Option<Recorded>,
    pub result: Option<Delivered>,
}

/// Why a row's Report… is greyed, when the reason is one to say (D448): a
/// row whose status the journal keeps — an earlier session's, the command
/// line's, an agent's, every rewrite — ended. The journal holds metadata,
/// never the characters (D312), so a full report built from it would put
/// on its verifiable shelf what nothing backs, and there would be no report
/// to copy as JSON; the list keeps a summary, which the row's status says.
/// `None` for a row nothing has happened to yet (greyed, as it always was)
/// and for a clean of this session (which has its report). Pure.
pub fn why_no_report(status: &Status) -> Option<String> {
    match status {
        Status::Recorded(said) if said.phase.is_end() => Some(t(Message::QueueActionReportJournal)),
        _ => None,
    }
}

/// Why row's Clean item is greyed, or `None` when it is not: a thing
/// that cannot be cleaned says why, and a row already in line or done
/// says that — and a row being rewritten, whose file the rewrite reads
/// again after a restart (R7). Pure, so the menu's rule is checked
/// without one. `cleanable` is `None` for a row with nothing behind it.
pub fn why_not_clean(status: &Status, cleanable: Option<Cleanable>) -> Option<String> {
    match status {
        Status::Queued | Status::Cleaning => Some(t(Message::QueueActionCleanBusy)),
        Status::RewriteQueued | Status::Rewriting(_) => Some(t(Message::QueueActionCleanRewriting)),
        Status::Recorded(said) if !said.phase.is_end() => Some(t(Message::QueueActionCleanBusy)),
        Status::Done(_) | Status::Recorded(_) => Some(t(Message::QueueActionCleanDone)),
        Status::Waiting => match cleanable {
            None => Some(t(Message::QueueActionNotKept)),
            Some(Cleanable::No(unable)) => Some(wording::unable(unable)),
            Some(Cleanable::Text(_) | Cleanable::Picture(_)) => None,
        },
    }
}

/// Why a row's Rewrite item is greyed, or `None` when it is not (R2):
/// nothing on duty — in the duty's own sentence, `vacant` — not text, the
/// thing not kept, already queued or being rewritten, or being cleaned.
/// A row cleaned or rewritten before may be rewritten (again): that is
/// asking, and Rewrite contains the clean (В2). Pure.
pub fn why_not_rewrite(
    status: &Status,
    cleanable: Option<Cleanable>,
    vacant: Option<String>,
) -> Option<String> {
    match status {
        Status::Queued | Status::Cleaning => return Some(t(Message::QueueActionRewriteCleaning)),
        Status::RewriteQueued | Status::Rewriting(_) => {
            return Some(t(Message::QueueActionRewriteBusy))
        }
        Status::Recorded(said) if !said.phase.is_end() => {
            return Some(t(Message::QueueActionRewriteBusy))
        }
        Status::Waiting | Status::Done(_) | Status::Recorded(_) => {}
    }
    match cleanable {
        None => Some(t(Message::QueueActionNotKept)),
        Some(Cleanable::Picture(_)) => Some(t(Message::QueueActionRewriteNotText)),
        Some(Cleanable::No(unable)) => Some(wording::unable(unable)),
        Some(Cleanable::Text(_)) => vacant,
    }
}

/// Why a row's Remove is greyed, or `None` when it is not: a clean holds
/// the row while it is queued or running, and an agent's or the command
/// line's rewrite in flight has a caller waiting for it (D355) — Cancel
/// ends that one and tells the caller; a removal would take the item from
/// under a call that is still waiting. Pure.
pub fn why_not_remove(status: &Status, origin: Origin) -> Option<String> {
    match status {
        Status::Queued | Status::Cleaning => Some(t(Message::QueueActionCleanBusy)),
        Status::RewriteQueued | Status::Rewriting(_)
            if matches!(origin, Origin::Agent | Origin::Cli) =>
        {
            Some(t(Message::QueueActionRemoveWaited))
        }
        Status::Recorded(said)
            if !said.phase.is_end() && matches!(origin, Origin::Agent | Origin::Cli) =>
        {
            Some(t(Message::QueueActionRemoveWaited))
        }
        _ => None,
    }
}

/// What a row shows of the thing: from intake for what arrived this
/// session, from the journal for a row read back.
#[derive(Debug, Clone, PartialEq)]
struct Look {
    title: String,
    folder: Option<String>,
    kind: Kind,
    format: Option<String>,
    encoding: Option<String>,
    size: Option<u64>,
}

impl Look {
    fn of(intake: &Intake) -> Self {
        let folder = match intake.arrived {
            Arrived::AsPath | Arrived::AsText => intake
                .path
                .as_deref()
                .and_then(std::path::Path::parent)
                .map(|parent| parent.display().to_string())
                .filter(|folder| !folder.is_empty()),
            Arrived::AsBytes => None,
        };
        Self {
            title: wording::title_of(intake),
            folder,
            kind: intake.kind,
            format: intake.format.map(|format| format.name().to_owned()),
            encoding: intake.encoding.map(|encoding| encoding.name().to_owned()),
            size: intake.size,
        }
    }

    fn of_entry(entry: &Entry) -> Self {
        let kind = journal::kind_of(entry.kind.as_deref());
        // A path a caller named is shown as a file's would be (D356); it
        // is only never opened.
        let path = entry
            .path
            .as_deref()
            .or(entry.said_path.as_deref())
            .map(std::path::Path::new);
        Self {
            title: entry
                .name
                .clone()
                .or_else(|| {
                    path.and_then(|path| path.file_name())
                        .map(|name| name.to_string_lossy().into_owned())
                })
                .unwrap_or_else(|| wording::kind_label(kind)),
            folder: path
                .and_then(std::path::Path::parent)
                .map(|parent| parent.display().to_string())
                .filter(|folder| !folder.is_empty()),
            kind,
            format: entry.format.clone(),
            encoding: entry.encoding.clone(),
            size: entry.size,
        }
    }
}

/// What Rewrite all would cost, said before it runs (D61).
#[derive(Debug, Clone, PartialEq)]
pub struct Price {
    pub documents: usize,
    pub calls_expected: u64,
    pub calls_worst: u64,
    pub tokens_worst: u64,
    /// `None` when the rate was never measured: unknown, never a guess.
    pub seconds_expected: Option<f64>,
    /// The endpoint's origin, when the documents would leave this machine.
    pub away: Option<String>,
}

impl Price {
    /// The dialog's lines.
    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![
            t_args(
                Message::RewritePriceCalls,
                &args!(
                    "expected" => self.calls_expected as i64,
                    "worst" => self.calls_worst as i64,
                ),
            ),
            t_args(
                Message::RewritePriceTokens,
                &args!("tokens" => self.tokens_worst as i64),
            ),
            match self.seconds_expected {
                Some(seconds) => t_args(
                    Message::RewritePriceTime,
                    &args!("minutes" => (seconds / 60.0).ceil().max(1.0) as i64),
                ),
                None => t(Message::RewritePriceTimeUnknown),
            },
        ];
        lines.push(match &self.away {
            Some(host) => t_args(Message::RewritePriceAway, &args!("host" => host.clone())),
            None => t(Message::RewritePriceHere),
        });
        lines
    }
}

/// One thing that arrived.
struct Row {
    /// Stable for the life of the row, and what every element id in it
    /// is built from — a `uniform_list` index moves the moment a row
    /// above is removed, and a hover card keyed by one would follow the
    /// index rather than the thing. The session's number; the ID column
    /// shows the journal's, below, once the journal has given one.
    id: u64,
    /// The row in the document journal: what the ID column shows and an
    /// agent names a document by (two id spaces — the element's, above,
    /// and the journal's; E4-6b). `None` for the moment between a row
    /// landing and the journal's writer saying which it is.
    entry: Option<i64>,
    /// Who handed it over.
    origin: Origin,
    /// What the row shows of the thing.
    look: Look,
    /// The batch queue's item, once a rewrite was pushed.
    item: Option<ItemId>,
    /// A single row's Rewrite says its price in its tooltip.
    price: Option<Price>,
    /// A result already where this row's rewrite would go, refused (D261)
    /// — what "Replace the existing result" offers to write over.
    existing: Option<PathBuf>,
    /// A cleaned paste's result as last saved in the Compare window (E7-9,
    /// D412): what Copy the result copies and Compare opens on, in place
    /// of the cleaned text — in memory, as that text is. Gone with the
    /// next clean or rewrite of the row.
    edited: Option<String>,
    /// When a Compare window last saved an edit over this row's result in
    /// this session, and the result it was — the mark C3's rule let land
    /// (D442): what says "…, then edited" on a row whose own outcome does
    /// not know it yet (D447). Gone with the next clean or rewrite of the
    /// row, as `edited` is.
    edited_at: Option<i64>,
    /// lazy-shot's `key_word`: a handle somebody assigned so the thing
    /// can be found again by name. Nothing assigns one yet — see the
    /// module docs — so today every row's is `None`.
    keyword: Option<String>,
    /// The thing itself: what arrived this session, or the file a journal
    /// row names, read again. `None` for a row read back with nothing
    /// behind it — a paste, an agent's text — which cannot be processed
    /// again (В4).
    arrival: Option<Arrival>,
    preview: Preview,
    arrived_at: DateTime<Local>,
    status: Status,
}

impl Row {
    /// Whether it can be cleaned — decided from what intake said, read
    /// on every frame, so nothing here reads a file. `None` with nothing
    /// behind the row.
    fn cleanable(&self) -> Option<Cleanable> {
        self.arrival
            .as_ref()
            .map(|arrival| clean::cleanable(&arrival.intake))
    }

    /// What a rewrite of this row ended with, when it ended.
    fn said(&self) -> Option<&Said> {
        match &self.status {
            Status::Recorded(said) => Some(said),
            _ => None,
        }
    }

    /// Whether its last action ended — what Clear finished takes.
    fn ended(&self) -> bool {
        match &self.status {
            Status::Done(_) => true,
            Status::Recorded(said) => said.phase.is_end(),
            _ => false,
        }
    }

    /// The rewrite's result, where it can be read back from — for Compare,
    /// Copy the result and Open the result.
    fn rewritten(&self, work: Option<&Work>) -> Option<RewriteFrom> {
        let said = self.said()?;
        if said.action != Action::Rewrite || said.phase != Phase::Done {
            return None;
        }
        match said.result.as_ref()? {
            Delivered::File { path, .. } => Some(RewriteFrom::File(PathBuf::from(path))),
            Delivered::Row => Some(RewriteFrom::Item(Arc::clone(&work?.queue), self.item?)),
            Delivered::Caller | Delivered::Nowhere => None,
        }
    }

    /// The file a result was written to, whoever wrote it.
    fn written(&self) -> Option<PathBuf> {
        match &self.status {
            Status::Done(outcome) => outcome.written.clone(),
            Status::Recorded(said) => match said.result.as_ref()? {
                Delivered::File { path, .. } => Some(PathBuf::from(path)),
                _ => None,
            },
            _ => None,
        }
    }

    /// Whether its result was edited by hand in Compare and saved as typed
    /// (D447): a Save that cleans says so in its outcome, the journal in
    /// its entry's `outcome.edited`, and a save this session in the mark
    /// it let land — for the result the row has now.
    fn edited(&self) -> bool {
        match &self.status {
            Status::Done(outcome) => outcome.edited || self.edited_at.is_some(),
            Status::Recorded(said) => {
                said.phase == Phase::Done
                    && (said
                        .outcome
                        .as_ref()
                        .is_some_and(|outcome| outcome.edited.is_some())
                        || self.edited_at.is_some())
            }
            _ => false,
        }
    }

    fn outcome(&self) -> Option<&Outcome> {
        match &self.status {
            Status::Done(outcome) => Some(outcome),
            _ => None,
        }
    }
}

/// What the queue asks of the window it is in.
#[derive(Debug, Clone, PartialEq)]
pub enum QueueEvent {
    /// Show the Report dialog for row `id` — painted by the shell, over
    /// the whole window, not inside the table.
    Report(u64),
    /// Rewrite all's price, said before anything is pushed (D61): rows
    /// `ids`, at `price`, going where `price.away` says. The shell asks;
    /// its answer is [`Queue::agreed`] by [`Road::Price`], naming that
    /// destination (D431).
    Price { ids: Vec<u64>, price: Price },
    /// Rows `ids` arrived with "Process what arrives" set to rewrite, and
    /// the engine on duty is not this machine: the shell asks once before
    /// they are sent to `host` (В1). Or — `replacing` — one row's Replace
    /// the existing result, asked the same question before it goes (D393).
    /// Yes is [`Queue::agreed`] — by [`Road::Arrivals`], or
    /// [`Road::Replace`] over that file — naming `host` (D431).
    SendAway {
        ids: Vec<u64>,
        host: String,
        replacing: Option<PathBuf>,
    },
    /// The batch queue holds to ask (D361): `count` waiting rewrites were
    /// asked for while rewriting stayed here (`was` `None`) or went to
    /// `was`, and the engine on duty now would send them to `host`. The
    /// shell asks once; yes is [`Queue::agree`] with `now`, and anything
    /// else leaves the queue holding.
    Consent {
        now: wipemark_queue::Whereto,
        host: String,
        was: Option<String>,
        count: usize,
    },
    /// The batch queue no longer asks — answered, the duty moved, the
    /// person resumed, or its item went: the shell takes its question down
    /// (D394).
    Unasked,
}

/// The question a yes answered, so that a yes the duty no longer stands
/// behind can ask it again (D431).
#[derive(Debug, Clone, PartialEq)]
pub enum Road {
    /// Rewrite all's price.
    Price,
    /// Rows a drop put in, with "Process what arrives" set to rewrite.
    Arrivals,
    /// One row's Replace the existing result, over this file.
    Replace(PathBuf),
}

/// When a row arrived, as the Arrived column says it: the time alone for
/// today, the date before it for any other day — rows live for days
/// (`journal.keep_days`), and "09:14" of last Tuesday read as this morning
/// (D363). Pure, over the two instants.
pub fn arrived_label(at: DateTime<Local>, now: DateTime<Local>) -> String {
    if at.date_naive() == now.date_naive() {
        at.format("%H:%M").to_string()
    } else {
        at.format("%Y-%m-%d %H:%M").to_string()
    }
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
    /// The application's one line of cleans, which the panel's wait in
    /// too (D283).
    cleaner: Entity<Cleaner>,
    /// Paths `--clean=` handed in, cleaned as their rows land — each
    /// once.
    to_clean: Vec<PathBuf>,
    /// Paths the command line handed in at launch, recorded as such.
    from_flag: Vec<PathBuf>,
    /// The batch queue and the journal (E4-6b); `None` in a test that
    /// built neither, where nothing is recorded and nothing rewritten.
    work: Option<Work>,
    /// The journal's writer for this window's rows, in the order asked.
    writer: Option<Writer>,
    /// Rewrites finished since the batch queue was last empty — the status
    /// bar's "Rewriting 2 of 5".
    rewrites_done: usize,
    /// Which paragraph of how many each running item is on.
    chunks: HashMap<ItemId, (u32, u32)>,
    /// The journal's `data_version` as last read — a row the command line
    /// wrote moves it.
    version: Option<i64>,
    /// Whether a read of the journal is under way, and whether another was
    /// asked for meanwhile.
    reading: bool,
    read_again: bool,
    /// Dropped with the view: every drop, and every import, lands here.
    _landed: Subscription,
    /// Dropped with the view: the line says when a row's clean starts and
    /// what it did.
    _cleaned: Subscription,
    /// Dropped with the view: what is typed into the filter bar.
    _typed: [Subscription; 2],
    /// Dropped with the view: a change of duty tells the batch queue where
    /// a rewrite would go now (D361).
    _duty: Subscription,
    /// How many times [`Queue::going`] has asked the duty — what a draw of
    /// the rows costs, counted for a test (D452).
    #[cfg(test)]
    goings: std::cell::Cell<usize>,
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
        let work = journal::working(cx);
        Self::with_catcher(preferences, catcher, work, window, cx)
    }

    /// The queue over a catcher already made — [`Queue::new`]'s, which
    /// the platform delivers to, or a test's, which it does not.
    fn with_catcher(
        preferences: Entity<Preferences>,
        catcher: Entity<Catcher>,
        work: Option<Work>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        if let Some(work) = &work {
            Self::listen(work, cx);
        }
        let landed = cx.subscribe(&catcher, |queue, _, Landed(arrivals): &Landed, cx| {
            queue.take(arrivals.clone(), cx);
        });
        let cleaner = Cleaner::shared(&preferences, cx);
        let cleaned = cx.subscribe(
            &cleaner,
            |queue: &mut Self, _, event: &cleaner::Event, cx| {
                queue.heard(event, cx);
            },
        );

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

        // Where a rewrite would go is the preferences' word; the batch queue
        // checks each waiting item's consent against it (D361).
        let duty = cx.observe(&preferences, |queue: &mut Self, _, cx| queue.tell_where(cx));

        let queue = Self {
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
            cleaner,
            to_clean: Vec::new(),
            from_flag: Vec::new(),
            writer: work.as_ref().map(|work| work.journal.writer()),
            work,
            rewrites_done: 0,
            chunks: HashMap::new(),
            version: None,
            reading: false,
            read_again: false,
            _landed: landed,
            _cleaned: cleaned,
            _typed: [typed_id, typed_keyword],
            _duty: duty,
            #[cfg(test)]
            goings: std::cell::Cell::new(0),
        };
        queue.tell_where(cx);
        queue
    }

    /// Hand paths down the road a drop takes — the command line's way
    /// in, and the one the Import button uses once the picker answers.
    pub fn hand(&self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        self.land(paths.into_iter().map(Handed::Path).collect(), cx);
    }

    /// `--import=<path>`: [`Queue::hand`], and the rows say they were
    /// named on the application's command line.
    pub fn hand_from_launch(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        self.from_flag.extend(paths.iter().cloned());
        self.hand(paths, cx);
    }

    /// Hand paths down the road a drop takes and clean each as it lands —
    /// `--clean=<path>`, which is Import followed by Clean. A row that
    /// cannot be cleaned is still asked, and its badge says why not.
    pub fn hand_to_clean(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        self.to_clean.extend(paths.iter().cloned());
        self.from_flag.extend(paths.iter().cloned());
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
        let arrived_ms = journal::now_ms();
        for (arrival, &id) in arrivals.iter().zip(&ids) {
            // Named on the application's command line, or handed over in
            // the window.
            let origin = match &arrival.handed {
                Handed::Path(path) => match self.from_flag.iter().position(|named| named == path) {
                    Some(at) => {
                        self.from_flag.remove(at);
                        Origin::LaunchFlag
                    }
                    None => Origin::Window,
                },
                Handed::Text(_) | Handed::Bytes { .. } => Origin::Window,
            };
            if let Some(writer) = &self.writer {
                writer.record(
                    id,
                    Written {
                        origin,
                        action: journal::clean_action(&arrival.intake),
                        phase: Phase::Waiting,
                        item: None,
                        arrived: arrived_ms,
                        ended: None,
                        entry: journal::entry_of(arrival),
                    },
                );
            }
            self.rows.push(Row {
                id,
                entry: None,
                origin,
                look: Look::of(&arrival.intake),
                item: None,
                price: None,
                existing: None,
                edited: None,
                edited_at: None,
                keyword: None,
                arrival: Some(arrival.clone()),
                preview: Preview::Pending,
                arrived_at: now,
                status: Status::Waiting,
            });
        }
        // The rows `--clean=` asked for, each path once.
        let mut asked = Vec::new();
        for row in &self.rows[self.rows.len() - arrivals.len()..] {
            if let Some(Handed::Path(path)) = row.arrival.as_ref().map(|arrival| &arrival.handed) {
                if let Some(at) = self.to_clean.iter().position(|wanted| wanted == path) {
                    self.to_clean.remove(at);
                    asked.push(row.id);
                }
            }
        }
        if !asked.is_empty() {
            self.clean(&asked, cx);
        }
        // "Process what arrives" (В1): what the General page says happens
        // to everything else that just landed.
        let rest: Vec<u64> = ids
            .iter()
            .copied()
            .filter(|id| !asked.contains(id))
            .collect();
        self.process_arrivals(&rest, cx);
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
    /// Report dialog — and whether its result was edited by hand and saved
    /// as typed since (D447), which the sheet says as the row does. `None`
    /// until it is done.
    pub fn report_of(&self, id: u64) -> Option<(Intake, Arc<Outcome>, bool)> {
        let row = self.rows.iter().find(|row| row.id == id)?;
        match (&row.status, &row.arrival) {
            (Status::Done(outcome), Some(arrival)) => {
                Some((arrival.intake.clone(), outcome.clone(), row.edited()))
            }
            _ => None,
        }
    }

    /// The rows Clean all would clean: waiting, and cleanable, in the
    /// order they arrived.
    pub fn cleanable_waiting(&self) -> Vec<u64> {
        self.rows
            .iter()
            .filter(|row| matches!(row.status, Status::Waiting))
            .filter(|row| {
                matches!(
                    row.cleanable(),
                    Some(Cleanable::Text(_) | Cleanable::Picture(_))
                )
            })
            .map(|row| row.id)
            .collect()
    }

    /// Clean every waiting row that can be cleaned — the toolbar's Clean
    /// all. Rows queued, cleaning or done are left where they are.
    pub fn clean_all(&mut self, cx: &mut Context<Self>) {
        let ids = self.cleanable_waiting();
        self.clean(&ids, cx);
    }

    /// Put the waiting rows among `ids` in the application's line, in that
    /// order, behind whatever any window asked before. A row that is not
    /// waiting is not asked twice.
    pub fn clean(&mut self, ids: &[u64], cx: &mut Context<Self>) {
        for &id in ids {
            let Some(row) = self.rows.iter_mut().find(|row| row.id == id) else {
                continue;
            };
            if !matches!(row.status, Status::Waiting) {
                continue;
            }
            let Some(arrival) = row.arrival.clone() else {
                continue;
            };
            if self
                .cleaner
                .update(cx, |cleaner, cx| cleaner.ask(id, arrival, None, cx))
            {
                row.status = Status::Queued;
            }
        }
        cx.notify();
    }

    /// "Replace the existing result": clean row `id` again and write over
    /// the one file its clean refused to (D261). Only for a row whose clean
    /// was refused for exactly that.
    pub fn replace(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(row) = self.rows.iter_mut().find(|row| row.id == id) else {
            return;
        };
        // A rewrite refused over an existing result is replaced by the
        // batch queue, writing over that one named file.
        if let Some(existing) = row.existing.clone() {
            self.replace_rewrite(id, existing, cx);
            return;
        }
        let Some(existing) = row.outcome().and_then(existing_result) else {
            return;
        };
        let Some(arrival) = row.arrival.clone() else {
            return;
        };
        if self.cleaner.update(cx, |cleaner, cx| {
            cleaner.ask(id, arrival, Some(existing), cx)
        }) {
            row.status = Status::Queued;
        }
        cx.notify();
    }

    /// Which clean of how many is running — the status bar's "Cleaning 2
    /// of 5", counted over every window's cleans.
    pub fn progress(&self, cx: &App) -> Option<(usize, usize)> {
        self.cleaner.read(cx).progress()
    }

    /// What row `id`'s Status cell says: its word, and its tooltip.
    #[cfg(test)]
    pub fn status_words(&self, id: u64) -> Option<(String, String)> {
        let row = self.rows.iter().find(|row| row.id == id)?;
        let (_, word, sentence) = status_said(row, self.rewrite_note(row), self.held());
        Some((word, sentence))
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

    /// What the line said about one of this queue's rows — a clean of the
    /// panel's is not a row here, and is not heard.
    fn heard(&mut self, event: &cleaner::Event, cx: &mut Context<Self>) {
        let (id, status) = match event {
            cleaner::Event::Started(id) => (*id, Status::Cleaning),
            cleaner::Event::Finished(id, outcome) => (*id, Status::Done(outcome.clone())),
        };
        let Some(row) = self.rows.iter_mut().find(|row| row.id == id) else {
            return;
        };
        // The journal hears it too: running, then what the clean came to.
        if let (Some(writer), Some(arrival)) = (&self.writer, &row.arrival) {
            let action = journal::clean_action(&arrival.intake);
            let mut entry = journal::entry_of(arrival);
            let (phase, ended) = match &status {
                Status::Done(outcome) => {
                    let (phase, recorded, delivered) = journal::clean_end(outcome);
                    entry.outcome = Some(recorded);
                    entry.result = Some(delivered);
                    (phase, Some(journal::now_ms()))
                }
                _ => (Phase::Running, None),
            };
            writer.change(
                id,
                Written {
                    origin: row.origin,
                    action,
                    phase,
                    item: None,
                    arrived: 0,
                    ended,
                    entry,
                },
            );
        }
        // A new clean is a new result: a paste's text saved in Compare
        // before it is not this one's.
        row.edited = None;
        row.edited_at = None;
        row.status = status;
        cx.notify();
    }

    /// The cleaned text of row `id`, for Copy the result — looked up at
    /// click time rather than cloned into every menu, for the reason
    /// [`compare_row`] gives. A paste's text saved in Compare, once there
    /// is one (D412).
    fn result_text(&self, id: u64) -> Option<String> {
        let row = self.rows.iter().find(|row| row.id == id)?;
        let outcome = row.outcome()?;
        outcome.text.as_ref()?;
        row.edited.clone().or_else(|| outcome.text.clone())
    }

    /// What a Compare window opened on row `id` saved (E7-9, D412): a
    /// cleaned paste's text, kept in the row, or a mark that the result's
    /// file or the batch queue's row holds an edit — written to the row's
    /// journal entry as *when*, never *what* (D417). `false` when the row
    /// cannot take it: gone, or no longer a result Compare saved into.
    ///
    /// A mark names what it is about (D442): the result of a clean or of a
    /// rewrite, and where it lives. It lands only while that is still the
    /// row's latest result — a window opened on a clean and saved after the
    /// row was rewritten marks nothing — and the writer checks the action
    /// again in the database, so a rewrite recorded between this and the
    /// write is not marked either. The save itself stands: `true`, and a
    /// log line.
    pub fn told_by_compare(&mut self, id: u64, told: Told, cx: &mut Context<Self>) -> bool {
        let work = self.work.clone();
        let Some(row) = self.rows.iter_mut().find(|row| row.id == id) else {
            return false;
        };
        let action = match told {
            // A Save in Compare that cleans keeps the row's own Clean's
            // rule: not while it waits or runs in a line, not while it is
            // rewritten (D411).
            // Nor over a rewrite's result: a window opened before the row
            // was rewritten still saves as a Clean, and that clean would
            // take the row — and its journal entry's item — from the
            // rewrite, a paste's rewritten text then reachable from
            // nothing (its one home is the batch queue's row). The row's
            // own Clean is greyed then too.
            Told::Cleans => {
                return !matches!(
                    &row.status,
                    Status::Queued
                        | Status::Cleaning
                        | Status::RewriteQueued
                        | Status::Rewriting(_)
                ) && !matches!(&row.status, Status::Recorded(said) if !said.phase.is_end())
                    && !matches!(
                        &row.status,
                        Status::Recorded(said)
                            if said.action == Action::Rewrite
                                && matches!(
                                    said.result,
                                    Some(Delivered::File { .. } | Delivered::Row)
                                )
                    );
            }
            Told::Text(text) => {
                // Only a paste whose clean handed its result back as text.
                let holds_text = matches!(
                    &row.status,
                    Status::Done(outcome) if outcome.written.is_none() && outcome.text.is_some()
                );
                if !holds_text {
                    return false;
                }
                row.edited = Some(text);
                Action::Clean
            }
            Told::Saved { action, home } => {
                if !holds(row, work.as_ref(), action, &home) {
                    tracing::info!(
                        id,
                        action = action.as_str(),
                        "a save in Compare is not this row's latest result; no edit mark"
                    );
                    return true;
                }
                action
            }
        };
        let now = journal::now_ms();
        row.edited_at = Some(now);
        if let Some(writer) = &self.writer {
            writer.edited(id, now, action);
        }
        cx.notify();
        true
    }
}

/// Whether `row`'s latest result is `action`'s and lives at `home` — what
/// an edit mark has to be about to land (D442).
fn holds(row: &Row, work: Option<&Work>, action: Action, home: &Home) -> bool {
    match action {
        Action::Clean => matches!(
            (cleaned_for(row), home),
            (Some((_, CleanedTo::File(path))), Home::File(saved)) if path == *saved
        ),
        Action::Rewrite => match (row.rewritten(work), home) {
            (Some(RewriteFrom::File(path)), Home::File(saved)) => path == *saved,
            (Some(RewriteFrom::Item(_, item)), Home::Item(saved)) => item == *saved,
            _ => false,
        },
        Action::CleanImage | Action::Inspect => false,
    }
}

#[cfg(test)]
mod rewrite_tests;
mod rewriting;

/// Copy the result: row `id`'s cleaned text onto the clipboard. The
/// person's own text, cleaned: no catalogue touches it on the way there.
fn copy_result(queue: &Entity<Queue>, id: u64, cx: &App) {
    if let Some(text) = queue.read(cx).result_text(id) {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        return;
    }
    // A rewrite's result is read back from where it was delivered — a
    // file, or the batch queue's row — off this thread.
    let from = {
        let queue = queue.read(cx);
        queue
            .rows
            .iter()
            .find(|row| row.id == id)
            .and_then(|row| row.rewritten(queue.work.as_ref()))
    };
    let Some(from) = from else {
        return;
    };
    cx.spawn(async move |cx| {
        let text = cx
            .background_executor()
            .spawn(async move { compare::rewritten_text(&from) })
            .await;
        match text {
            Ok(text) => {
                cx.update(|cx| cx.write_to_clipboard(ClipboardItem::new_string(text)));
            }
            Err(_) => tracing::warn!(id, "a rewrite's result could not be read back to copy"),
        }
    })
    .detach();
}

/// Where a clean of `row` put its result, when it put one: the file it
/// wrote — with the original's own file when the clean was in place, the
/// source's name now holding the result — or a paste's text, as cleaned or
/// as last saved in Compare (E7-9). `None` while nothing was written: a
/// row waiting, being cleaned, or whose clean found nothing, was refused or
/// failed — where a Save in Compare is a Clean of its text (D411).
fn cleaned_for(row: &Row) -> Option<(Option<PathBuf>, CleanedTo)> {
    match &row.status {
        Status::Done(outcome) => match (&outcome.written, &outcome.text) {
            (Some(path), _) => Some((outcome.set_aside.clone(), CleanedTo::File(path.clone()))),
            (None, Some(text)) => Some((
                None,
                CleanedTo::Text(row.edited.clone().unwrap_or_else(|| text.clone())),
            )),
            (None, None) => None,
        },
        // A clean of an earlier session, or another surface's, that wrote
        // a file.
        Status::Recorded(said)
            if matches!(said.action, Action::Clean) && said.phase == Phase::Done =>
        {
            match said.result.as_ref()? {
                Delivered::File { path, original, .. } => Some((
                    original.as_ref().map(PathBuf::from),
                    CleanedTo::File(PathBuf::from(path)),
                )),
                _ => None,
            }
        }
        _ => None,
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
    let Some(arrival) = &row.arrival else {
        // Read back with nothing behind it: the kind's glyph, no card.
        let theme = cx.theme();
        return div()
            .w(THUMB.width)
            .h(THUMB.height)
            .flex_shrink_0()
            .rounded(theme.radius)
            .border_1()
            .border_color(theme.border)
            .bg(theme.muted)
            .flex()
            .items_center()
            .justify_center()
            .child(
                Icon::new(kind_glyph(row.look.kind))
                    .large()
                    .color(theme.muted_foreground),
            )
            .into_any_element();
    };
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
    let intake = &arrival.intake;

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
                .anchor(Anchor::TopLeft)
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
                .anchor(Anchor::TopLeft)
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
/// A status's sentence, and — for a result edited by hand and saved as
/// typed — that nothing checked the edits for marks (D447).
fn then_edited(row: &Row, sentence: String) -> String {
    if row.edited() {
        format!("{sentence} {}", t(Message::QueueStatusEditedTooltip))
    } else {
        sentence
    }
}

fn status_cell(row: &Row, notes: Vec<String>, held: bool) -> AnyElement {
    let (tag, word, sentence) = status_said(row, notes, held);
    let sentence = SharedString::from(sentence);
    div()
        .id(("queue-status", row.id))
        .child(tag.small().child(SharedString::from(word)))
        .tooltip(move |window, cx| Tooltip::new(sentence.clone()).build(window, cx))
        .into_any_element()
}

/// What a row's Status cell says: its badge, its word, and the sentence
/// its tooltip holds.
fn status_said(row: &Row, notes: Vec<String>, held: bool) -> (Tag, String, String) {
    let tag_of = |badge: Badge| match badge {
        Badge::Muted => Tag::secondary(),
        Badge::Success => Tag::success(),
        Badge::Warning => Tag::warning(),
        Badge::Danger => Tag::danger(),
    };
    match &row.status {
        Status::Waiting => match row.cleanable() {
            Some(Cleanable::No(unable)) => (
                Tag::secondary().outline(),
                t(Message::QueueStatusUnable),
                wording::unable(unable),
            ),
            None => (
                Tag::secondary().outline(),
                t(Message::QueueStatusWaiting),
                t(Message::QueueActionNotKept),
            ),
            _ => (
                Tag::secondary(),
                t(Message::QueueStatusWaiting),
                t(Message::QueueStatusWaitingTooltip),
            ),
        },
        Status::RewriteQueued if held => (
            Tag::warning().outline(),
            t(Message::QueueStatusHeld),
            notes.join(" "),
        ),
        Status::RewriteQueued => (
            Tag::info(),
            t(Message::QueueStatusRewriteQueued),
            std::iter::once(t(Message::QueueStatusRewriteQueuedTooltip))
                .chain(notes)
                .collect::<Vec<_>>()
                .join(" "),
        ),
        Status::Rewriting(chunk) => (
            Tag::primary(),
            t(Message::QueueStatusRewriting),
            std::iter::once(match chunk {
                Some((chunk, chunks)) => t_args(
                    Message::QueueStatusRewritingChunk,
                    &args!("chunk" => *chunk, "chunks" => *chunks),
                ),
                None => t(Message::QueueStatusRewritingTooltip),
            })
            .chain(notes)
            .collect::<Vec<_>>()
            .join(" "),
        ),
        Status::Recorded(said) => {
            let (word, badge) =
                wording::recorded_badge(said.action, said.phase, said.outcome.as_ref());
            let word = if row.edited() {
                wording::edited(word)
            } else {
                word
            };
            let sentence = match &row.existing {
                // A result already there: which file, and the one way over
                // it (D357).
                Some(existing) if said.action == Action::Rewrite => t_args(
                    Message::QueueSaidRewriteExists,
                    &args!("path" => existing.display().to_string()),
                ),
                _ => wording::recorded_said(said.action, said.phase, said.outcome.as_ref()),
            };
            (tag_of(badge), t(word), then_edited(row, sentence))
        }
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
            let word = if row.edited() {
                wording::edited(word)
            } else {
                word
            };
            (
                tag_of(badge),
                t(word),
                then_edited(row, wording::said(outcome)),
            )
        }
    }
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
    // Who asked, when it was not this window (R4), before the folder.
    let folder = match (wording::origin_line(row.origin), row.look.folder.clone()) {
        (Some(origin), Some(folder)) => Some(format!("{origin} · {folder}")),
        (origin, folder) => origin.or(folder),
    };
    let note = match (row.outcome(), row.said()) {
        (Some(outcome), _) => Some((wording::went(outcome).join(" · "), Tone::Muted)),
        (None, Some(said)) => {
            let went = wording::delivered_went(said.result.as_ref());
            (!went.is_empty()).then(|| (went.join(" · "), Tone::Muted))
        }
        (None, None) => row
            .arrival
            .as_ref()
            .and_then(|arrival| wording::evidence_note(&arrival.intake)),
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
                .child(SharedString::from(row.look.title.clone())),
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

/// The row's next steps, on the row (D325): Clean and Rewrite, the same
/// two the Actions menu starts with and down the same roads — greyed,
/// with the menu's own reason as the tooltip, when one cannot run. A
/// person who dropped something sees what can be done with it without
/// opening a menu.
/// What a row's button does when pressed.
type Step = Box<dyn Fn(&mut Queue, &mut Context<Queue>)>;

fn process_cell(
    id: u64,
    clean: Option<String>,
    rewrite: Option<String>,
    queue: Entity<Queue>,
) -> AnyElement {
    let step =
        |label: Message, icon: IconName, key: &'static str, why: Option<String>, run: Step| {
            let queue = queue.clone();
            let tooltip = SharedString::from(why.clone().unwrap_or_else(|| t(label)));
            div()
                .id((key, id))
                .debug_selector(move || format!("{key}-{id}"))
                .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
                .child(
                    Button::new((key, id))
                        .xsmall()
                        .outline()
                        .icon(icon)
                        .label(SharedString::from(t(label)))
                        .disabled(why.is_some())
                        .on_click(move |_, _, cx| {
                            queue.update(cx, |queue, cx| run(queue, cx));
                        }),
                )
        };
    h_flex()
        .gap_1()
        .child(step(
            Message::QueueActionClean,
            IconName::Broom,
            "row-clean",
            clean,
            Box::new(move |queue, cx| queue.clean(&[id], cx)),
        ))
        .child(step(
            Message::QueueActionRewrite,
            IconName::Pen,
            "row-rewrite",
            rewrite,
            Box::new(move |queue, cx| queue.rewrite(&[id], cx)),
        ))
        .into_any_element()
}

/// The container and, under it, how the characters are stored.
fn format_cell(look: &Look, cx: &App) -> AnyElement {
    let theme = cx.theme();
    v_flex()
        .min_w(px(0.0))
        .gap_0p5()
        .child(div().text_sm().truncate().child(SharedString::from(
            look.format.clone().unwrap_or_else(|| "—".to_owned()),
        )))
        .children(look.encoding.clone().map(|encoding| {
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(SharedString::from(encoding))
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
    /// `None` when Rewrite can be pressed; otherwise why not.
    rewrite: Option<String>,
    /// Whether a rewrite is queued or running, to cancel.
    cancel: bool,
    /// `None` when the row can be removed; otherwise why not — a clean
    /// holds it, or a caller waits for its rewrite (D355).
    remove: Option<String>,
    /// The result on disk, for Open the result and Show it in its folder.
    written: Option<PathBuf>,
    /// Whether there is a cleaned text to copy.
    text: bool,
    /// Whether the clean or the rewrite was refused over an existing
    /// result — Replace is offered.
    replace: bool,
    /// Why Replace is greyed although offered: a rewrite's Replace is a
    /// Rewrite, and greyed when one would be (D393).
    replace_why: Option<String>,
    /// Whether there is a finished clean to report.
    report: bool,
    /// Why Report… is greyed, when there is a reason to give: a row whose
    /// status the journal keeps, of which only a summary is kept (D448).
    report_why: Option<String>,
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
        .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
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
            let (rewriting, cancelling, removing) = (queue.clone(), queue.clone(), queue.clone());
            // Rewrite after Clean, greyed with its reason the way Clean is
            // (D269): two actions, because they differ in cost and in where
            // the document goes (В2).
            let rewrite = match actions.rewrite.clone() {
                None => PopupMenuItem::new(SharedString::from(t(Message::QueueActionRewrite)))
                    .icon(IconName::Pen)
                    .on_click(move |_, _, cx| {
                        rewriting.update(cx, |queue, cx| queue.rewrite(&[id], cx));
                    }),
                Some(why) => {
                    let why = SharedString::from(why);
                    PopupMenuItem::element(move |_, cx| {
                        v_flex()
                            .max_w(px(280.0))
                            .child(SharedString::from(t(Message::QueueActionRewrite)))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(why.clone()),
                            )
                    })
                    .icon(IconName::Pen)
                    .disabled(true)
                }
            };
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
                .item(rewrite)
                .item(
                    PopupMenuItem::new(SharedString::from(t(Message::QueueActionCancel)))
                        .icon(IconName::CircleXmark)
                        .disabled(!actions.cancel)
                        .on_click(move |_, _, cx| {
                            cancelling.update(cx, |queue, _| queue.cancel(id));
                        }),
                )
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
                        .on_click(move |_, _, cx| copy_result(&copying, id, cx)),
                )
                .item(match actions.report_why.clone() {
                    None => PopupMenuItem::new(SharedString::from(t(Message::QueueActionReport)))
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
                    // Greyed with its reason under it, as Clean is (D269, D448).
                    Some(why) => {
                        let why = SharedString::from(why);
                        PopupMenuItem::element(move |_, cx| {
                            v_flex()
                                .max_w(px(280.0))
                                .child(SharedString::from(t(Message::QueueActionReport)))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(why.clone()),
                                )
                        })
                        .icon(IconName::FileLines)
                        .disabled(true)
                    }
                })
                .item(
                    match actions.replace_why.clone().filter(|_| actions.replace) {
                        None => {
                            PopupMenuItem::new(SharedString::from(t(Message::QueueActionReplace)))
                                .icon(IconName::Replace)
                                .disabled(!actions.replace)
                                .on_click(move |_, _, cx| {
                                    replacing.update(cx, |queue, cx| queue.replace(id, cx));
                                })
                        }
                        // Greyed with its reason under it, as Rewrite is (D269, D393).
                        Some(why) => {
                            let why = SharedString::from(why);
                            PopupMenuItem::element(move |_, cx| {
                                v_flex()
                                    .max_w(px(280.0))
                                    .child(SharedString::from(t(Message::QueueActionReplace)))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(why.clone()),
                                    )
                            })
                            .icon(IconName::Replace)
                            .disabled(true)
                        }
                    },
                )
                .separator()
                .item(match actions.remove.clone() {
                    None => PopupMenuItem::new(SharedString::from(t(Message::QueueActionRemove)))
                        .icon(IconName::Trash)
                        .on_click(move |_, _, cx| {
                            removing.update(cx, |queue, cx| queue.remove(id, cx));
                        }),
                    // Greyed with its reason under it, as Clean is (D269).
                    Some(why) => {
                        let why = SharedString::from(why);
                        PopupMenuItem::element(move |_, cx| {
                            v_flex()
                                .max_w(px(280.0))
                                .child(SharedString::from(t(Message::QueueActionRemove)))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(why.clone()),
                                )
                        })
                        .icon(IconName::Trash)
                        .disabled(true)
                    }
                })
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
        let preferences = queue.read(cx).preferences.clone();
        tracing::info!(id, "opening the Compare window on a queued row");
        compare::open(
            Some(id),
            subject,
            comparison,
            Some(link_to(id, &queue)),
            Some(preferences),
            main,
            cx,
        );
    });
}

/// How a Compare window opened on row `id` tells the row what it saved
/// (D412): through the queue, while the queue is there.
///
/// Where the row's clean put its result is asked of the row as it is now
/// (D441), and the journal's writer is the queue's (D440).
pub fn link_to(id: u64, queue: &Entity<Queue>) -> compare::Link {
    let queue = queue.downgrade();
    let (homed, flushing) = (queue.clone(), queue.clone());
    compare::Link {
        told: Rc::new(move |told, cx| {
            queue
                .update(cx, |queue, cx| queue.told_by_compare(id, told, cx))
                .unwrap_or(false)
        }),
        home: Rc::new(move |cx| {
            let queue = homed.upgrade()?;
            let queue = queue.read(cx);
            let row = queue.rows.iter().find(|row| row.id == id)?;
            cleaned_for(row).map(|(aside, to)| (to, aside))
        }),
        flushed: Rc::new(move |cx| {
            let queue = flushing.upgrade()?;
            queue.read(cx).writer.as_ref().map(Writer::flushed)
        }),
    }
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
        let row = self.rows.iter().find(|row| row.id == id)?;
        // The latest result: a rewrite's once one ended (R5).
        if let Some((handed, made)) = self.made_for(row) {
            return Some(Subject {
                handed,
                intake: None,
                made,
            });
        }
        let arrival = row.arrival.as_ref()?;
        // A clean's, once it put one somewhere: the window opens on it as
        // it stands — edits saved there included — and saves back over it
        // (E7-9, D410). In place, the original is the file set aside.
        if let Some((original, cleaned)) = cleaned_for(row) {
            let intake = (original.is_none()).then(|| arrival.intake.clone());
            return Some(Subject {
                handed: original
                    .map(Handed::Path)
                    .unwrap_or_else(|| arrival.handed.clone()),
                intake,
                made: Made::CleanedTo(cleaned),
            });
        }
        Some(Subject {
            handed: arrival.handed.clone(),
            intake: Some(arrival.intake.clone()),
            made: Made::Cleaned,
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
    /// for the rows on screen, with `vacant`, why nothing would rewrite,
    /// asked once for all of them (D452).
    fn row(&self, index: usize, vacant: Option<&str>, cx: &Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let row = &self.rows[index];
        let rewrite = self.why_not_rewrite_given(row.id, vacant);
        // Once cleaned, what happened; once rewritten, likewise; until
        // then, what would.
        let lines = match (row.outcome(), row.said(), &row.arrival) {
            (Some(outcome), _, _) => {
                let mut lines = vec![wording::said(outcome)];
                lines.extend(wording::went(outcome));
                lines
            }
            (None, Some(said), _) => {
                let mut lines = vec![wording::recorded_said(
                    said.action,
                    said.phase,
                    said.outcome.as_ref(),
                )];
                lines.extend(wording::delivered_went(said.result.as_ref()));
                lines
            }
            (None, None, Some(arrival)) => {
                wording::would_happen(&self.preferences.read(cx).plan_for(&arrival.intake))
            }
            (None, None, None) => Vec::new(),
        };

        let comparable = self.made_for(row).is_some()
            || row
                .arrival
                .as_ref()
                .is_some_and(|arrival| Subject::comparable(&arrival.intake));
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
                row.entry.map(|entry| entry.to_string()),
                cx,
            )))
            .child(Column::Keyword.cell().child(copyable(
                ("copy-keyword", row.id),
                row.keyword.clone(),
                cx,
            )))
            .child(Column::Name.cell().child(name_cell(row, cx)))
            .child(Column::Kind.cell().child(kind_tag(row.look.kind)))
            .child(Column::Status.cell().child(status_cell(
                row,
                self.rewrite_note(row),
                self.held(),
            )))
            .child(Column::Process.cell().child(process_cell(
                row.id,
                why_not_clean(&row.status, row.cleanable()),
                rewrite.clone(),
                cx.entity(),
            )))
            .child(Column::Format.cell().child(format_cell(&row.look, cx)))
            .child(
                Column::Size.cell().text_sm().child(SharedString::from(
                    row.look
                        .size
                        .map_or_else(|| "—".to_owned(), drop::size_label),
                )),
            )
            .child(
                Column::Arrived
                    .cell()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(SharedString::from(arrived_label(
                        row.arrived_at,
                        Local::now(),
                    ))),
            )
            .child(
                Column::Actions.cell().child(actions_cell(
                    Actions {
                        id: row.id,
                        path: row
                            .arrival
                            .as_ref()
                            .and_then(|arrival| arrival.intake.path.clone()),
                        comparable,
                        clean: why_not_clean(&row.status, row.cleanable()),
                        rewrite: rewrite.clone(),
                        cancel: matches!(row.status, Status::RewriteQueued | Status::Rewriting(_))
                            && row.item.is_some(),
                        remove: why_not_remove(&row.status, row.origin),
                        written: row.written(),
                        text: row.outcome().is_some_and(|outcome| outcome.text.is_some())
                            || row.rewritten(self.work.as_ref()).is_some(),
                        replace: row.outcome().and_then(existing_result).is_some()
                            || row.existing.is_some(),
                        replace_why: row.existing.as_ref().and(rewrite),
                        report: row.outcome().is_some() && row.arrival.is_some(),
                        report_why: why_no_report(&row.status),
                    },
                    cx.entity(),
                )),
            )
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
                    .dropdown_menu_with_anchor(Anchor::BottomRight, {
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
            // The duty once for the rows of a draw, never once a row: asking
            // it builds a roster and runs `duty::on_duty` (D452). Taken here,
            // where the list's processor is made, because the list calls
            // that three times a draw — a row measured in its layout and in
            // its prepaint, then the rows on screen.
            let vacant = self.vacancy(cx);
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
                                    .map(|&index| queue.row(index, vacant.as_deref(), cx))
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
pub(crate) mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use chrono::Local;
    use gpui::prelude::*;
    use gpui::{
        div, point, px, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, TestAppContext,
        VisualTestContext,
    };
    use wipemark_i18n::{args, t, Message};
    use wipemark_intake::{Handed, Kind};
    use wipemark_store::entry::{Action, Origin, Phase};

    use super::{
        arrived_label, copy_result, kind_glyph, kind_tag, page_range, pages, row_frame, shown,
        why_not_clean, why_not_remove, Cleanable, Column, Filter, Order, Queue, Said, Status,
        PAGE_SIZES, ROW,
    };
    use crate::clean::Verdict;
    use crate::retention::{Destination, Homes};
    use crate::settings::Preferences;
    use crate::wording;

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

    /// The line under the table says what is true now (R6): the list cleans
    /// and rewrites, one rewrite at a time, whoever asked — and no longer
    /// that rewriting is not here. Never an epic number, which a person
    /// reading a window can do nothing with.
    #[test]
    fn the_footer_says_the_list_cleans_and_rewrites() {
        let line = t(Message::QueuePending);
        assert!(line.contains("cleans and rewrites"), "{line}");
        assert!(line.contains("one rewrite at a time"), "{line}");
        assert!(
            !line.contains("not in this version"),
            "the footer still says rewriting is not here: {line}"
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

    /// Clean is greyed with a reason for a thing that cannot be cleaned —
    /// the reason names the format — and for a row in line or done; a
    /// waiting text or picture can be cleaned.
    #[test]
    fn clean_is_greyed_with_a_reason_when_it_cannot_run() {
        use wipemark_intake::Format;

        use crate::clean::Unable;

        let tiff = Cleanable::No(Unable::NotYet(Format::Tiff));
        let text = Cleanable::Text(wipemark_intake::Encoding::Utf8);
        let why = why_not_clean(&Status::Waiting, Some(tiff)).expect("a TIFF cannot be cleaned");
        assert!(why.contains("TIFF"), "{why}");
        assert_eq!(why_not_clean(&Status::Waiting, Some(text)), None);
        assert_eq!(
            why_not_clean(&Status::Waiting, Some(Cleanable::Picture(Format::Png))),
            None
        );
        for status in [Status::Queued, Status::Cleaning] {
            assert_eq!(
                why_not_clean(&status, Some(text)),
                Some(t(Message::QueueActionCleanBusy))
            );
        }
        // Being rewritten: a clean now would race the rewrite (R7).
        for status in [Status::RewriteQueued, Status::Rewriting(Some((1, 2)))] {
            assert_eq!(
                why_not_clean(&status, Some(text)),
                Some(t(Message::QueueActionCleanRewriting))
            );
        }
        // Nothing behind the row: not kept.
        assert_eq!(
            why_not_clean(&Status::Waiting, None),
            Some(t(Message::QueueActionNotKept))
        );
    }

    /// Report… of a row the journal keeps is greyed with its reason (D448):
    /// the journal holds a summary, never the characters, so there is no
    /// full report to show or copy — and the item says so rather than
    /// greying in silence. A row nothing happened to stays greyed as it
    /// was; a clean of this session has its report. Make `why_no_report`
    /// answer `None` always and the journal's rows are silent again: red.
    #[test]
    fn report_of_a_journal_row_is_greyed_with_its_reason() {
        use super::why_no_report;
        for action in [Action::Clean, Action::Rewrite] {
            for phase in [Phase::Done, Phase::Failed, Phase::Cancelled] {
                let status = Status::Recorded(Box::new(Said {
                    action,
                    phase,
                    outcome: None,
                    result: None,
                }));
                assert_eq!(
                    why_no_report(&status),
                    Some(t(Message::QueueActionReportJournal)),
                    "{action:?} {phase:?}"
                );
            }
        }
        let running = Status::Recorded(Box::new(Said {
            action: Action::Rewrite,
            phase: Phase::Running,
            outcome: None,
            result: None,
        }));
        for status in [Status::Waiting, Status::Queued, Status::Cleaning, running] {
            assert_eq!(why_no_report(&status), None, "{status:?}");
        }
    }

    /// D355: Remove is greyed while an agent or the command line waits for
    /// the row's rewrite — Cancel ends it and tells the caller — and while
    /// a clean holds the row; a window's own rewrite, and anything that
    /// ended, can be removed.
    #[test]
    fn remove_is_greyed_while_a_caller_waits_for_the_rewrite() {
        for origin in [Origin::Agent, Origin::Cli] {
            for status in [
                Status::RewriteQueued,
                Status::Rewriting(Some((1, 3))),
                Status::Recorded(Box::new(Said {
                    action: Action::Rewrite,
                    phase: Phase::Running,
                    outcome: None,
                    result: None,
                })),
            ] {
                assert_eq!(
                    why_not_remove(&status, origin),
                    Some(t(Message::QueueActionRemoveWaited)),
                    "{origin:?} {status:?}"
                );
            }
        }
        assert!(why_not_remove(&Status::RewriteQueued, Origin::Window).is_none());
        assert!(why_not_remove(&Status::Cleaning, Origin::Window).is_some());
        let ended = Status::Recorded(Box::new(Said {
            action: Action::Rewrite,
            phase: Phase::Done,
            outcome: None,
            result: None,
        }));
        assert!(why_not_remove(&ended, Origin::Agent).is_none());
    }

    /// D363: today's rows say the time; any other day's say the date too.
    #[test]
    fn a_row_from_another_day_says_its_date() {
        use chrono::TimeZone as _;
        let now = Local
            .with_ymd_and_hms(2026, 10, 8, 18, 0, 0)
            .single()
            .expect("a time");
        let morning = Local
            .with_ymd_and_hms(2026, 10, 8, 9, 14, 0)
            .single()
            .expect("a time");
        let last_week = Local
            .with_ymd_and_hms(2026, 10, 1, 9, 14, 0)
            .single()
            .expect("a time");
        assert_eq!(arrived_label(morning, now), "09:14");
        assert_eq!(arrived_label(last_week, now), "2026-10-01 09:14");
    }

    /// D363: the waiting row's tooltip points at the row's own buttons,
    /// which are where Clean and Rewrite are first.
    #[test]
    fn the_not_started_tooltip_points_at_the_rows_buttons() {
        let said = t(Message::QueueStatusWaitingTooltip);
        assert!(!said.contains("Actions menu"), "{said}");
        assert!(said.contains("on its row"), "{said}");
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
    pub(crate) struct Scratch(pub(crate) std::path::PathBuf);

    impl Scratch {
        pub(crate) fn new(label: &str) -> Self {
            let directory =
                std::env::temp_dir().join(format!("wipemark-queue-{label}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&directory);
            std::fs::create_dir_all(directory.join("results")).expect("scratch directory");
            Self(directory)
        }

        pub(crate) fn file(&self, name: &str, bytes: &[u8]) -> std::path::PathBuf {
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
        queue_with(cx, scratch, None)
    }

    /// [`queue_in`], over a batch queue and a journal — `work`.
    pub(crate) fn queue_with<'a>(
        cx: &'a mut TestAppContext,
        scratch: &Scratch,
        work: Option<crate::journal::Work>,
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
            let queue =
                cx.new(|cx| Queue::with_catcher(preferences.clone(), catcher, work, window, cx));
            *held.borrow_mut() = Some((queue.clone(), preferences));
            gpui_component::Root::new(queue, window, cx)
        });
        let (queue, preferences) = slot.take().expect("the window builder ran");
        (queue, preferences, cx)
    }

    pub(crate) fn statuses(
        queue: &gpui::Entity<Queue>,
        cx: &mut VisualTestContext,
    ) -> Vec<&'static str> {
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
                    Some(Status::RewriteQueued) => "rewrite-queued",
                    Some(Status::Rewriting(_)) => "rewriting",
                    Some(Status::Recorded(said)) => match said.phase {
                        Phase::Done => match said.outcome.as_ref().map(|o| o.verdict.as_str()) {
                            Some("partly") => "partly-rewritten",
                            _ => "rewritten",
                        },
                        Phase::Failed => "rewrite-failed",
                        Phase::Cancelled => "cancelled",
                        _ => "recorded",
                    },
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
        assert_eq!(cx.update(|_, cx| queue.read(cx).progress(cx)), Some((1, 2)));
        // Asked again while the line runs — by Clean all, or row by row
        // as `--clean=` and the menu ask: nothing is asked twice.
        queue.update(cx, |queue, cx| queue.clean_all(cx));
        assert_eq!(cx.update(|_, cx| queue.read(cx).progress(cx)), Some((1, 2)));
        let ids = cx.update(|_, cx| queue.read(cx).ids());
        queue.update(cx, |queue, cx| queue.clean(&ids[..2], cx));
        assert_eq!(cx.update(|_, cx| queue.read(cx).progress(cx)), Some((1, 2)));
        assert_eq!(statuses(&queue, cx), ["cleaning", "queued", "waiting"]);

        // The page changes while the second row waits.
        preferences.update(cx, |preferences, cx| {
            preferences.select_destination(Destination::Folder, cx);
        });
        cx.run_until_parked();

        assert_eq!(statuses(&queue, cx), ["cleaned", "cleaned", "waiting"]);
        assert_eq!(cx.update(|_, cx| queue.read(cx).progress(cx)), None);
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

        // That file only: nothing else was written, and the row says it
        // replaced the result rather than wrote a new one.
        let mut names: Vec<String> = std::fs::read_dir(&scratch.0)
            .expect("list")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        assert_eq!(names, ["results", "x.cleaned.md", "x.md"]);
        let went = cx.update(|_, cx| match queue.read(cx).status_of(id) {
            Some(Status::Done(outcome)) => wording::went_in(&wording::window, outcome),
            _ => Vec::new(),
        });
        let replaced_line =
            wording::window(Message::QueueWentReplaced, &args!("name" => "x.cleaned.md"));
        assert!(went.contains(&replaced_line), "{went:?}");

        // Done is done: a second Replace has nothing to replace.
        queue.update(cx, |queue, cx| queue.replace(id, cx));
        assert_eq!(statuses(&queue, cx), ["cleaned"]);
    }

    /// D301, the owner's empty row of 2026-10-07: an empty text on the
    /// clipboard greys the Paste button, a press of it lands no row, and
    /// neither does a drop of empty text or of a lone newline — while a
    /// text beside them still lands.
    #[gpui::test]
    fn a_paste_or_a_drop_of_empty_text_lands_nothing(cx: &mut TestAppContext) {
        let scratch = Scratch::new("empty-paste");
        let (queue, _preferences, cx) = queue_in(cx, &scratch);
        cx.update(|_, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string(String::new())));
        let clipboard = cx.update(|_, cx| cx.new(crate::clipboard::Clipboard::new));
        let label = cx.update(|_, cx| crate::clipboard::label(clipboard.read(cx).held()));
        assert_eq!(
            label,
            (Message::ToolbarPaste, 0),
            "the button over an empty text is not the greyed Paste"
        );

        let pasted = cx.update(|_, cx| crate::clipboard::Clipboard::take(cx));
        queue.update(cx, |queue, cx| queue.land(pasted, cx));
        cx.run_until_parked();
        assert!(cx.update(|_, cx| queue.read(cx).ids()).is_empty());

        queue.update(cx, |queue, cx| {
            queue.land(
                vec![
                    Handed::Text(String::new()),
                    Handed::Text("\n".to_owned()),
                    Handed::Text("a word".to_owned()),
                ],
                cx,
            );
        });
        cx.run_until_parked();
        assert_eq!(
            cx.update(|_, cx| queue.read(cx).ids()).len(),
            1,
            "an empty text was listed, or the word beside it was not"
        );
    }

    /// The live check's case 7, as a test: a pasted text with a U+200B,
    /// "Keep what you paste" on, cleaned; Copy the result puts the cleaned
    /// text on the clipboard, and the kept directory holds the characters
    /// as they were pasted and no result.
    #[gpui::test]
    fn a_paste_is_cleaned_copied_and_its_original_kept(cx: &mut TestAppContext) {
        let scratch = Scratch::new("paste");
        let (queue, preferences, cx) = queue_in(cx, &scratch);
        preferences.update(cx, |preferences, cx| preferences.keep_originals(true, cx));
        let pasted = "paste\u{200B}me";

        queue.update(cx, |queue, cx| {
            queue.land(vec![Handed::Text(pasted.to_owned())], cx)
        });
        cx.run_until_parked();
        let ids = cx.update(|_, cx| queue.read(cx).ids());
        assert_eq!(ids.len(), 1);
        queue.update(cx, |queue, cx| queue.clean(&ids, cx));
        cx.run_until_parked();
        assert_eq!(statuses(&queue, cx), ["cleaned"]);

        cx.update(|_, cx| copy_result(&queue, ids[0], cx));
        let copied = cx.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
        assert_eq!(
            copied.as_deref(),
            Some("pasteme"),
            "the copy is not the result"
        );

        let kept: Vec<std::path::PathBuf> = std::fs::read_dir(scratch.0.join("kept"))
            .expect("kept/")
            .map(|entry| entry.expect("entry").path())
            .collect();
        assert_eq!(kept.len(), 1, "{kept:?}");
        let mut copies: Vec<String> = std::fs::read_dir(&kept[0])
            .expect("the kept directory")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        copies.sort();
        assert_eq!(
            copies,
            ["original.txt"],
            "a result was kept with its switch off"
        );
        assert_eq!(
            std::fs::read(kept[0].join("original.txt")).expect("original"),
            pasted.as_bytes()
        );
    }
}
