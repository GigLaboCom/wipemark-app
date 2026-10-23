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
//! # Sideways (D811–D813)
//!
//! In a window narrower than every column at its least — the name at
//! [`NAME_MIN`], what it has in a window of the size every launch opens
//! at — the header and the rows scroll sideways together, in one scroll:
//! a trackpad's swipe, Shift and the wheel, and the bar along the bottom
//! of the rows. A swipe sideways never scrolls the rows up or down, nor the
//! wheel the table sideways, and the offset is the table's, kept across
//! pages and filters.
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
//! Every row gets a result of its own (D635): a file already where the
//! result would go is left alone, and the row's result takes the row's own
//! name, `name.cleaned-<id>.ext`, the id its ID column shows — a clean or a
//! rewrite never writes over a file and never fails because a name is
//! taken. "Replace the existing result" is gone with the refusal it
//! answered (D638).
//!
//! A row whose clean ended — whatever it came to — names its Clean **Clean
//! again** (D703, [`labels`]), and that writes over **the row's own
//! result** ([`Queue::clean_again`], D704): the file the row records, when
//! no other row records it, it is still there, it is not the source and it
//! is one of the names the plan taken now gives the row; otherwise the row
//! takes a result of its own as every clean does. A result edited by hand
//! and saved asks first (D705, [`QueueEvent::Again`]); one written in place
//! is not redone (D706). [`Queue::clean`] — Clean all, the panel's Clean,
//! `--clean=` — still takes waiting rows only (D708).
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
//! A row whose rewrite ended — done, failed or cancelled — names its
//! Rewrite **Rewrite again** (D703), and keeps Clean greyed: the rewrite
//! contains the clean. Rewrite again pushes over the row's own result
//! (`Destination::File`, D704, decided where the request is built) by the
//! same rules as Clean again, asks first over an edited result (D705), is
//! greyed in place (D706), and takes the row's earlier batch-queue item out
//! once the new one is reserved (D707). Rewrite all leaves rewritten rows to
//! their own Rewrite again (D702).
//!
//! # The journal (E4-6b)
//!
//! The table **is** the document journal (`wipemark_store::Journal`):
//! every row is a row of it, read back at launch, so the list survives a
//! restart (В3) — a row's own Remove, Clear finished on the toolbar, and
//! the keep period on the Retention page take rows away. Rows other
//! surfaces wrote — an agent, the command line — appear without a click,
//! and say who asked.
//!
//! # The panel's drops are rows here (D690)
//!
//! A drop on the panel is handed to this table the moment it lands
//! ([`Queue::take_from_panel`]), down the road a drop on this window takes
//! — one row per thing, its journal row written by this table's writer
//! with the origin `panel` — whether this window is in front, hidden by
//! the tray or minimised: the table is a global ([`Table`]) and outlives
//! its window. The panel keeps the rows' ids; its Clean asks this table's
//! [`Queue::clean`] for them, and what it shows of each is read back from
//! the row ([`Queue::standing`]), so a clean asked from either window is
//! one row and one status in both (D691, D692). A row keeps metadata after its end (В4):
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
    ImageSource, ObjectFit, PathPromptOptions, Pixels, ScrollHandle, ScrollStrategy, SharedString,
    Stateful, Subscription, UniformListScrollHandle, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::clipboard::Clipboard;
use gpui_component::hover_card::HoverCard;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::scroll::Scrollbar;
use gpui_component::tag::Tag;
use gpui_component::{
    h_flex, v_flex, ActiveTheme, ColorName, Disableable as _, InteractiveElementExt as _,
    Sizable as _, StyledExt as _,
};
use wipemark_i18n::{args, t, t_args, Message};
use wipemark_intake::{Arrived, Handed, Intake, Kind};
use wipemark_queue::ItemId;
use wipemark_store::entry::{Action, Delivered, Entry, Origin, Outcome as Recorded, Phase};

use crate::clean::{self, Cleanable, Outcome};
use crate::cleaner::{self, Cleaner};
use crate::compare::{self, CleanedTo, Comparison, Home, Made, RewriteFrom, Subject, Told};
use crate::drop::{self, Arrival, Catcher, Landed};
use crate::hover::Tip as _;
use crate::icon::{Icon, IconName};
use crate::journal::{self, Work, Writer, Written};
use crate::preview::{Excerpt, Picture, Preview};
use crate::previews::Source;
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

/// The lanes the scrollbars are drawn in: down the right of the rows,
/// and along their bottom (D811). gpui-component keeps its own width
/// private; this is the number its table uses.
const SCROLLBAR: Pixels = px(12.0);

/// How tall the header row is — where the rows, and the vertical bar's
/// lane, begin.
const HEADER: Pixels = px(32.0);

/// The least the Name column is given before the table scrolls sideways
/// (D812): what it has in a main window of the size every launch opens
/// at — 1280 points ([`crate::MAIN_SIZE`]) less the other columns' 1168.
/// So a window of that size shows every column with nothing to scroll, as
/// it did, and a narrower one keeps the name as wide as that window shows
/// it — a dozen characters or so before the ellipsis, and the folder and
/// the note under it likewise — and scrolls the rest into view rather than
/// crushing the name to its padding.
/// `a_window_of_the_size_every_launch_opens_at_scrolls_nothing` holds the
/// two together.
const NAME_MIN: Pixels = px(112.0);

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
    /// length, and never less than [`NAME_MIN`] of it.
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

    /// The least this column is ever given: its width, or the name's
    /// least.
    fn least(self) -> Pixels {
        self.width().unwrap_or(NAME_MIN)
    }

    /// The least the table is ever drawn: every column at its least
    /// (D811). In a window narrower than this the header and the rows
    /// scroll sideways together; in a wider one the name takes the rest.
    fn least_table() -> Pixels {
        Column::ALL.iter().map(|column| column.least()).sum()
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
            None => cell.flex_1().flex_shrink_1().min_w(NAME_MIN),
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

/// Which of a row's two actions it would do **again** (D703): the one its
/// last action was, once that ended — however it ended, because a failed or
/// cancelled run is retried the same way. A clean that ended is this
/// session's `Done`, or the journal's clean of a text or a picture; a
/// rewrite that ended is the journal's. `None` for a row nothing ended for
/// yet, and for one an agent only inspected, whose Clean and Rewrite are
/// first ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Again {
    /// Clean again: over the row's own cleaned result.
    Clean,
    /// Rewrite again: over the row's own rewritten result.
    Rewrite,
}

impl Again {
    /// The question's `$action` (D705). An id, never translated.
    pub fn id(self) -> &'static str {
        match self {
            Again::Clean => "clean",
            Again::Rewrite => "rewrite",
        }
    }
}

/// [`Again`] for a row's status. Pure.
pub fn again_of(status: &Status) -> Option<Again> {
    match status {
        Status::Done(_) => Some(Again::Clean),
        Status::Recorded(said) if said.phase.is_end() => match said.action {
            Action::Clean | Action::CleanImage => Some(Again::Clean),
            Action::Rewrite => Some(Again::Rewrite),
            Action::Inspect => None,
        },
        _ => None,
    }
}

/// What a row's Clean and Rewrite are called — its Actions items and its
/// row buttons alike (D703): **Clean again** on a row whose last action was
/// a clean that ended, **Rewrite again** on one whose last action was a
/// rewrite that ended, and Clean and Rewrite otherwise. A cleaned row's
/// Rewrite is a first rewrite (D702), and named so. Pure.
pub fn labels(status: &Status) -> (Message, Message) {
    match again_of(status) {
        Some(Again::Clean) => (Message::QueueActionCleanAgain, Message::QueueActionRewrite),
        Some(Again::Rewrite) => (Message::QueueActionClean, Message::QueueActionRewriteAgain),
        None => (Message::QueueActionClean, Message::QueueActionRewrite),
    }
}

/// Why a redo of a row whose result was written **in place** is greyed
/// (D706): the source's name holds the result now, so a redo would clean
/// or rewrite the result itself. Said with the name the original was set
/// aside as — this session's outcome's, or the journal's. `None` for a
/// result written anywhere else. Pure.
fn in_place(status: &Status) -> Option<String> {
    let original = match status {
        Status::Done(outcome) => outcome.set_aside.clone()?,
        Status::Recorded(said) => match said.result.as_ref()? {
            Delivered::File {
                original: Some(original),
                ..
            } => PathBuf::from(original),
            _ => return None,
        },
        _ => return None,
    };
    let name = original.file_name().map_or_else(
        || original.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    Some(t_args(
        Message::QueueActionAgainInPlace,
        &args!("name" => name),
    ))
}

/// What Clean and Rewrite say on a row with nothing behind it (D269),
/// worded for what is missing (D731): the file the row names went — or is
/// no regular file any more — or it never named one and its text was not
/// kept. `named` is whether the row names a file (`Row::named`).
fn not_behind(named: bool) -> String {
    t(if named {
        Message::QueueActionWhyGone
    } else {
        Message::QueueActionNotKept
    })
}

/// Why a thing cannot be cleaned at all, or `None` when it can: nothing
/// behind the row ([`not_behind`]), or what intake established it is.
fn cannot_clean(cleanable: Option<Cleanable>, named: bool) -> Option<String> {
    match cleanable {
        None => Some(not_behind(named)),
        Some(Cleanable::No(unable)) => Some(wording::unable(unable)),
        Some(Cleanable::Text(_) | Cleanable::Picture(_)) => None,
    }
}

/// Whether a rewrite's end left the row a rewritten text — done, and
/// delivered to a file or into the row. A rewrite that failed, was
/// cancelled or rewrote nothing (D711, D712: delivered nowhere) left none.
/// Pure.
fn holds_a_rewrite(said: &Said) -> bool {
    said.action == Action::Rewrite
        && said.phase == Phase::Done
        && matches!(said.result, Some(Delivered::File { .. } | Delivered::Row))
}

/// Why row's Clean item is greyed, or `None` when it is not: a thing
/// that cannot be cleaned says why, and a row already in line says that —
/// and a row being rewritten, whose file the rewrite reads again after a
/// restart (R7). A row whose clean ended is cleaned **again** (D703), greyed
/// only for a thing that cannot be cleaned and for a clean written in place
/// (D706); a row whose rewrite left a rewritten text keeps Clean greyed, the
/// rewrite containing the clean, while one whose rewrite failed, was
/// cancelled or rewrote nothing is cleaned as a first clean would be (D720);
/// and an inspected row keeps it greyed as it was. Pure, so the menu's rule
/// is checked without one. `cleanable` is `None` for a row with nothing
/// behind it — said here as for a row that never named a file; a row's own
/// item says it as [`clean_greyed`] does, from what the row names.
pub fn why_not_clean(status: &Status, cleanable: Option<Cleanable>) -> Option<String> {
    clean_greyed(status, cleanable, false)
}

/// [`why_not_clean`], with nothing behind a row that names a file (`named`)
/// said as the file gone rather than never kept (D731). Pure.
pub fn clean_greyed(status: &Status, cleanable: Option<Cleanable>, named: bool) -> Option<String> {
    match status {
        Status::Queued | Status::Cleaning => Some(t(Message::QueueActionCleanBusy)),
        Status::RewriteQueued | Status::Rewriting(_) => Some(t(Message::QueueActionCleanRewriting)),
        Status::Recorded(said) if !said.phase.is_end() => Some(t(Message::QueueActionCleanBusy)),
        Status::Recorded(said) => match said.action {
            Action::Rewrite if holds_a_rewrite(said) => Some(t(Message::QueueActionCleanRewritten)),
            Action::Rewrite => cannot_clean(cleanable, named),
            Action::Inspect => Some(t(Message::QueueActionCleanDone)),
            Action::Clean | Action::CleanImage => {
                in_place(status).or_else(|| cannot_clean(cleanable, named))
            }
        },
        Status::Done(_) => in_place(status).or_else(|| cannot_clean(cleanable, named)),
        Status::Waiting => cannot_clean(cleanable, named),
    }
}

/// Why a row's Rewrite item is greyed, or `None` when it is not (R2):
/// nothing on duty — in the duty's own sentence, `vacant` — not text, the
/// thing not kept, already queued or being rewritten, or being cleaned.
/// A row cleaned or rewritten before may be rewritten (again): that is
/// asking, and Rewrite contains the clean (В2) — except a rewrite written in
/// place, whose Rewrite again is greyed (D706). Pure. Nothing behind the
/// row is said as for a row that never named a file; a row's own item says
/// it as [`rewrite_greyed`] does.
pub fn why_not_rewrite(
    status: &Status,
    cleanable: Option<Cleanable>,
    vacant: Option<String>,
) -> Option<String> {
    rewrite_greyed(status, cleanable, vacant, false)
}

/// [`why_not_rewrite`], with nothing behind a row that names a file
/// (`named`) said as the file gone rather than never kept (D731). Pure.
pub fn rewrite_greyed(
    status: &Status,
    cleanable: Option<Cleanable>,
    vacant: Option<String>,
    named: bool,
) -> Option<String> {
    match status {
        Status::Queued | Status::Cleaning => return Some(t(Message::QueueActionRewriteCleaning)),
        Status::RewriteQueued | Status::Rewriting(_) => {
            return Some(t(Message::QueueActionRewriteBusy))
        }
        Status::Recorded(said) if !said.phase.is_end() => {
            return Some(t(Message::QueueActionRewriteBusy))
        }
        Status::Recorded(said) if said.action == Action::Rewrite => {
            if let Some(why) = in_place(status) {
                return Some(why);
            }
        }
        Status::Waiting | Status::Done(_) | Status::Recorded(_) => {}
    }
    match cleanable {
        None => Some(not_behind(named)),
        Some(Cleanable::Picture(_)) => Some(t(Message::QueueActionRewriteNotText)),
        Some(Cleanable::No(unable)) => Some(wording::unable(unable)),
        Some(Cleanable::Text(_)) => vacant,
    }
}

/// Why a row's Remove is greyed, or `None` when it is not: a clean holds
/// the row while it is queued or running, and a caller's rewrite in flight
/// has the caller waiting for it (D355) — Cancel ends that one and tells
/// the caller; a removal would take the item from under a call that is
/// still waiting. A caller's call is the action's, not the row's (D730):
/// an agent's or the command line's row whose item the person pushed from
/// the table (`consented`, Rewrite again) has nobody waiting, and is
/// removed as a window row is. Pure.
pub fn why_not_remove(status: &Status, origin: Origin, consented: bool) -> Option<String> {
    let waited = journal::callers_call(Some(origin), consented);
    match status {
        Status::Queued | Status::Cleaning => Some(t(Message::QueueActionCleanBusy)),
        Status::RewriteQueued | Status::Rewriting(_) if waited => {
            Some(t(Message::QueueActionRemoveWaited))
        }
        Status::Recorded(said) if !said.phase.is_end() && waited => {
            Some(t(Message::QueueActionRemoveWaited))
        }
        _ => None,
    }
}

// Why the items that need the thing itself, or its result, are greyed
// (D731): the reason goes on the line under the item, as Clean's does
// (D269), and names what is missing — never who handed the thing over.

/// What an item that needs the thing itself says on a row with nothing
/// behind it: the file the row names went — or is no regular file any
/// more — or it named none and nothing of its text was kept. `named` is
/// whether the row names a file.
fn nothing_behind(named: bool) -> String {
    t(if named {
        Message::QueueActionWhyGone
    } else {
        Message::QueueActionWhyNotKept
    })
}

/// Why "Open with" is greyed, or `None` when a file is
/// behind the row to open (D731). A text that arrived this session has none
/// to open. Pure.
pub fn why_not_open(arrival: Option<&Arrival>, named: bool) -> Option<String> {
    match arrival {
        Some(arrival) if arrival.intake.path.is_some() => None,
        Some(_) => Some(t(Message::QueueActionWhyNoFile)),
        None => Some(nothing_behind(named)),
    }
}

/// Why "Compare with the result" — and the double click on the row — is
/// greyed, or `None` when it opens (D731, D733): a rewrite's result to put
/// beside its original (`rewritten`), or a text behind the row. Pure.
pub fn why_not_compare(arrival: Option<&Arrival>, named: bool, rewritten: bool) -> Option<String> {
    match arrival {
        _ if rewritten => None,
        Some(arrival) if Subject::comparable(&arrival.intake) => None,
        Some(_) => Some(t(Message::CompareRefusedNotText)),
        None => Some(nothing_behind(named)),
    }
}

/// Why "Open the result" and "Show the result in its folder" are greyed on
/// a row whose result is no file (D731): nothing ended yet; the result went
/// back to whoever asked; it is a text kept in the row, the thing having no
/// file behind it; or nothing was written, which the row's status explains.
/// Pure; asked only where [`Row::written`] names no file.
pub fn why_no_result_file(status: &Status) -> String {
    t(match status {
        Status::Done(outcome) => match (&outcome.written, &outcome.text) {
            (None, Some(_)) => Message::QueueActionWhyNoFile,
            _ => Message::QueueActionWhyNothingWritten,
        },
        Status::Recorded(said) if said.phase.is_end() => match &said.result {
            Some(Delivered::Caller) => Message::QueueActionWhyToCaller,
            Some(Delivered::Row) => Message::QueueActionWhyNoFile,
            Some(Delivered::File { .. } | Delivered::Nowhere) | None => {
                Message::QueueActionWhyNothingWritten
            }
        },
        _ => Message::QueueActionWhyNoResult,
    })
}

/// Why "Copy the result" is greyed on a row with no result to copy (D731,
/// D762): it copies whatever the result is — a text kept in the row, a
/// rewrite's read back, a file a clean wrote — so the item says what
/// [`why_no_result_file`] says, or that the text a row's result lived in is
/// no longer kept. Pure; asked only where nothing is there to copy.
pub fn why_not_copy(status: &Status) -> String {
    match status {
        Status::Recorded(said)
            if said.phase.is_end() && matches!(said.result, Some(Delivered::Row)) =>
        {
            t(Message::QueueActionWhyNotKept)
        }
        _ => why_no_result_file(status),
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
        // A path a caller named is shown as a recorded one is — and looked
        // at like one too (D729).
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
    /// Whether that item was pushed with a window's consent (D361) — the
    /// person's action — rather than by a caller who waits for it (D730):
    /// set when this table pushes, read off the queue's list for a row read
    /// back. What Remove asks (D355): only a caller's call waits.
    consented: bool,
    /// A single row's Rewrite says its price in its tooltip.
    price: Option<Price>,
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
    /// The file a row read back from the journal names — the one recorded,
    /// or the absolute one its caller named (D729) — whether or not it is
    /// still there: what says, of a row with nothing behind it, whether its
    /// file went or it never had one (D731). `None` for what arrived this
    /// session, which always has its arrival.
    named: Option<PathBuf>,
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

    /// Why its Clean is greyed, or `None` (D269) — nothing behind a row that
    /// names a file said as the file gone (D731).
    fn clean_why(&self) -> Option<String> {
        clean_greyed(&self.status, self.cleanable(), self.named.is_some())
    }

    /// Why its Rewrite is greyed, or `None`, with the duty's `vacant` —
    /// worded as [`Row::clean_why`] is.
    fn rewrite_why(&self, vacant: Option<String>) -> Option<String> {
        rewrite_greyed(&self.status, self.cleanable(), vacant, self.named.is_some())
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
    /// they are sent to `host` (В1). Yes is [`Queue::agreed`] by
    /// [`Road::Arrivals`], naming `host` (D431).
    SendAway { ids: Vec<u64>, host: String },
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
    /// Row `id`'s own Clean again or Rewrite again, on a result edited by
    /// hand and saved (D447): the shell asks whether redoing it may replace
    /// those edits (D705). Yes is [`Queue::again_agreed`]; anything else
    /// changes nothing.
    Again { id: u64, action: Again },
}

/// The question a yes answered, so that a yes the duty no longer stands
/// behind can ask it again (D431).
#[derive(Debug, Clone, PartialEq)]
pub enum Road {
    /// Rewrite all's price.
    Price,
    /// Rows a drop put in, with "Process what arrives" set to rewrite.
    Arrivals,
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
    /// The rows up and down.
    scroll: UniformListScrollHandle,
    /// The header and the rows sideways, together (D811). The table's,
    /// not a page's (D813): a page turned or a filter typed leaves the
    /// columns where the person scrolled them.
    across: ScrollHandle,
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

/// The main window's table, as the panel reaches it (D690): a strong
/// handle, so the list outlives its window — a drop on the panel still
/// lands as a row while the main window is hidden, minimised or gone.
/// Set once, by the shell that builds the table.
pub struct Table(pub Entity<Queue>);

impl gpui::Global for Table {}

impl Table {
    /// The table, when a shell has built one.
    pub fn of(cx: &App) -> Option<Entity<Queue>> {
        cx.try_global::<Table>().map(|table| table.0.clone())
    }
}

/// Where a row stands, as the panel reads it for the thing it shows
/// (D692): one fact with the row's own status.
#[derive(Debug, Clone)]
pub enum Standing {
    /// No such row: removed from the list, or never made.
    Gone,
    /// Nothing asked of it yet: Clean may take it.
    Waiting,
    /// In the line of cleans, or being cleaned now.
    Cleaning,
    /// In the batch queue, or being rewritten now.
    Rewriting,
    /// Cleaned, refused or failed — what the clean came to.
    Cleaned(Arc<Outcome>),
    /// Anything else the list says of it — a rewrite that ended.
    Elsewhere,
}

impl PartialEq for Standing {
    /// Two standings are one when they are the same state — and, cleaned,
    /// the same clean's outcome.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Standing::Cleaned(a), Standing::Cleaned(b)) => Arc::ptr_eq(a, b),
            _ => std::mem::discriminant(self) == std::mem::discriminant(other),
        }
    }
}

impl Standing {
    /// Whether something runs for it: a clean or a rewrite.
    pub fn running(&self) -> bool {
        matches!(self, Standing::Cleaning | Standing::Rewriting)
    }
}

impl Queue {
    /// Where a drop on this window lands — the table's zone and the
    /// window's overlay share it.
    pub fn catcher(&self) -> &Entity<Catcher> {
        &self.catcher
    }

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
            across: ScrollHandle::new(),
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

    /// A table over a detached catcher, for another window's test — the
    /// panel's, which hands its drops here (D690).
    #[cfg(test)]
    pub(crate) fn for_tests(
        preferences: Entity<Preferences>,
        work: Option<Work>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let catcher = cx.new(|_| Catcher::detached());
        Self::with_catcher(preferences, catcher, work, window, cx)
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

    /// Something landed on this window: list it now, and find out what it
    /// looks like afterwards.
    fn take(&mut self, arrivals: Arc<[Arrival]>, cx: &mut Context<Self>) {
        self.take_as(arrivals, None, cx);
    }

    /// Something was dropped on the panel: list it here as a drop on this
    /// window is, one row per thing, its journal row's origin `panel`
    /// (D690). The rows' ids, in the order of `arrivals` — what the panel
    /// keeps to show each row and to clean it (D691). A drop that carried
    /// nothing is no row, and no id.
    pub fn take_from_panel(
        &mut self,
        arrivals: Arc<[Arrival]>,
        cx: &mut Context<Self>,
    ) -> Vec<u64> {
        self.take_as(arrivals, Some(Origin::Panel), cx)
    }

    /// List what landed, under `origin` when a surface other than this
    /// window handed it over, and say which rows it became.
    fn take_as(
        &mut self,
        arrivals: Arc<[Arrival]>,
        handed_by: Option<Origin>,
        cx: &mut Context<Self>,
    ) -> Vec<u64> {
        if arrivals.is_empty() {
            // A drop that carried nothing this machine could read. The
            // panel has a sentence for it; a table has nothing to add a
            // row for, and the log line is the record.
            tracing::info!("a drop carried nothing that could be listed");
            return Vec::new();
        }

        // One number per thing across the windows (D280): the panel's
        // cleans draw from the same counter, so a gap is a clean there.
        let ids: Vec<u64> = arrivals.iter().map(|_| clean::number()).collect();
        let now = Local::now();
        let arrived_ms = journal::now_ms();
        for (arrival, &id) in arrivals.iter().zip(&ids) {
            // Handed over on the panel, named on the application's command
            // line, or handed over in the window.
            let origin = match (handed_by, &arrival.handed) {
                (Some(origin), _) => origin,
                (None, Handed::Path(path)) => {
                    match self.from_flag.iter().position(|named| named == path) {
                        Some(at) => {
                            self.from_flag.remove(at);
                            Origin::LaunchFlag
                        }
                        None => Origin::Window,
                    }
                }
                (None, Handed::Text(_) | Handed::Bytes { .. }) => Origin::Window,
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
                // A picture with no file behind it keeps a preview of what
                // arrived, for after a restart (D823, D824).
                if let (Some(bytes), Some(work)) = (unfiled_picture(arrival), &self.work) {
                    keep_preview(writer, &work.journal, id, Source::Bytes(bytes.to_vec()), cx);
                }
            }
            self.rows.push(Row {
                id,
                entry: None,
                origin,
                look: Look::of(&arrival.intake),
                item: None,
                consented: false,
                price: None,
                edited: None,
                edited_at: None,
                keyword: None,
                arrival: Some(arrival.clone()),
                named: None,
                preview: Preview::Pending,
                arrived_at: now,
                status: Status::Waiting,
            });
        }
        // The rows `--clean=` asked for, each path once — never one the
        // panel handed over, which no flag named.
        let mut asked = Vec::new();
        let flagged = if handed_by.is_none() {
            arrivals.len()
        } else {
            0
        };
        for row in &self.rows[self.rows.len() - flagged..] {
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
        tracing::info!(
            count = arrivals.len(),
            ?kinds,
            origin = handed_by.unwrap_or(Origin::Window).as_str(),
            "queued"
        );

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
        let listed = ids.clone();
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
        listed
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
            // Named by the row's journal id when its plain name is taken
            // (D635) — asked of the writer that recorded the row when the
            // window does not know it yet (D637).
            let tag = journal::Tag::of(id, row.entry, self.writer.as_ref());
            if self
                .cleaner
                .update(cx, |cleaner, cx| cleaner.ask(id, arrival, tag, cx))
            {
                row.status = Status::Queued;
            }
        }
        cx.notify();
    }

    /// Clean again (D703): row `id`'s own item, on a row whose last action
    /// was a clean that ended. It cleans the row once more, in the one line,
    /// by the plan taken when it starts (D279), **over the row's own result**
    /// (D704) — never into a new `-<id>-2` file. A result edited by hand and
    /// saved (D447) is asked about first (D705): [`QueueEvent::Again`], and
    /// nothing changes until [`Queue::again_agreed`]. Without edits it runs
    /// at once, as the first Clean did. A row greyed for it is left alone.
    ///
    /// [`Queue::clean`] stays waiting rows only: Clean all, the panel's Clean
    /// and `--clean=` go through it, and none of them redoes a row (D708).
    pub fn clean_again(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(row) = self.rows.iter().find(|row| row.id == id) else {
            return;
        };
        if again_of(&row.status) != Some(Again::Clean)
            || why_not_clean(&row.status, row.cleanable()).is_some()
        {
            return;
        }
        if row.edited() {
            cx.emit(QueueEvent::Again {
                id,
                action: Again::Clean,
            });
            return;
        }
        self.redo_clean(id, cx);
    }

    /// Clean on a row whose last rewrite left no rewritten text — failed,
    /// cancelled, or rewrote nothing (D720): a first clean of the row, in the
    /// one line, by the plan taken when it starts (D279), into a result of
    /// the row's own (D635). Nothing is written over, and nothing is asked:
    /// there is no result of the rewrite to lose. A row whose rewrite left a
    /// text keeps Clean greyed, and is left alone.
    pub fn clean_unrewritten(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(row) = self.rows.iter_mut().find(|row| row.id == id) else {
            return;
        };
        if again_of(&row.status) != Some(Again::Rewrite)
            || why_not_clean(&row.status, row.cleanable()).is_some()
        {
            return;
        }
        let Some(arrival) = row.arrival.clone() else {
            return;
        };
        let tag = journal::Tag::of(id, row.entry, self.writer.as_ref());
        tracing::info!(id, "a row whose rewrite left nothing is cleaned");
        if self
            .cleaner
            .update(cx, |cleaner, cx| cleaner.ask(id, arrival, tag, cx))
        {
            row.status = Status::Queued;
        }
        cx.notify();
    }

    /// Yes to the question [`QueueEvent::Again`] put (D705): redo row `id`'s
    /// `action` without asking again. A row that moved while the question
    /// was open — removed, in a line, its last action another — is left as
    /// it is.
    pub fn again_agreed(&mut self, id: u64, action: Again, cx: &mut Context<Self>) {
        let Some(row) = self.rows.iter().find(|row| row.id == id) else {
            return;
        };
        if again_of(&row.status) != Some(action) {
            return;
        }
        match action {
            Again::Clean => self.redo_clean(id, cx),
            Again::Rewrite => self.rewrite(&[id], cx),
        }
    }

    /// Clean row `id` again, over its own result: the line is asked with
    /// that file as `replacing` (D704), which the clean writes over only
    /// when it is still there, is not the source and is one of the names
    /// the plan gives the row now.
    fn redo_clean(&mut self, id: u64, cx: &mut Context<Self>) {
        let replacing = self.own_result(id);
        let Some(row) = self.rows.iter_mut().find(|row| row.id == id) else {
            return;
        };
        if again_of(&row.status) != Some(Again::Clean)
            || why_not_clean(&row.status, row.cleanable()).is_some()
        {
            return;
        }
        let Some(arrival) = row.arrival.clone() else {
            return;
        };
        let tag = journal::Tag::of(id, row.entry, self.writer.as_ref());
        tracing::info!(id, over = replacing.is_some(), "a row is cleaned again");
        if self.cleaner.update(cx, |cleaner, cx| {
            cleaner.ask_with(id, arrival, tag, replacing, None, cx)
        }) {
            row.status = Status::Queued;
        }
        cx.notify();
    }

    /// Row `id`'s own result — the file [`Row::written`] names — when **no
    /// other row of the list** records the same file as its result (D704
    /// rule 3): a plain name freed by a deletion can since have been taken
    /// by another row, and that file is the other row's. Decided here, on
    /// the GPUI thread, from what the rows say; nothing is read. The rules
    /// that need the disk — still there, not the source, one of the row's
    /// names now — are the write's.
    fn own_result(&self, id: u64) -> Option<PathBuf> {
        let own = self.rows.iter().find(|row| row.id == id)?.written()?;
        let shared = self
            .rows
            .iter()
            .any(|other| other.id != id && other.written().as_deref() == Some(own.as_path()));
        (!shared).then_some(own)
    }

    /// What row `id` is called — the Again question's `$name` (D705).
    pub fn title_of(&self, id: u64) -> Option<String> {
        self.rows
            .iter()
            .find(|row| row.id == id)
            .map(|row| row.look.title.clone())
    }

    /// What runs for this table now, as an update's gate reads it (D788):
    /// a clean in the line or a row waiting for one, and a rewrite in the
    /// batch queue — waiting or running, paused or not, for a restart would
    /// cut the line either way.
    pub fn at_work(&self, cx: &App) -> (bool, bool) {
        let cleaning = self.progress(cx).is_some()
            || self
                .rows
                .iter()
                .any(|row| matches!(row.status, Status::Queued | Status::Cleaning));
        let rewriting = self.rewrites_open()
            || self
                .rows
                .iter()
                .any(|row| matches!(row.status, Status::RewriteQueued | Status::Rewriting(_)));
        (cleaning, rewriting)
    }

    /// How many cleaned texts live only in memory — a paste's result, which
    /// no file holds and the process takes with it (D419): what a restart
    /// for an update would drop, said before it is asked for (D785).
    pub fn texts_only_in_memory(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| match &row.status {
                Status::Done(outcome) => outcome.written.is_none() && outcome.text.is_some(),
                _ => false,
            })
            .count()
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

    /// Where row `id` stands — what the panel shows of a thing it handed
    /// over, read from the row itself (D692).
    pub fn standing(&self, id: u64) -> Standing {
        let Some(row) = self.rows.iter().find(|row| row.id == id) else {
            return Standing::Gone;
        };
        match &row.status {
            Status::Waiting => Standing::Waiting,
            Status::Queued | Status::Cleaning => Standing::Cleaning,
            Status::RewriteQueued | Status::Rewriting(_) => Standing::Rewriting,
            Status::Done(outcome) => Standing::Cleaned(outcome.clone()),
            Status::Recorded(said) if !said.phase.is_end() => match said.action {
                Action::Rewrite => Standing::Rewriting,
                _ => Standing::Cleaning,
            },
            Status::Recorded(_) => Standing::Elsewhere,
        }
    }

    /// Who handed row `id` over, if it is here.
    #[cfg(test)]
    pub fn origin_of(&self, id: u64) -> Option<Origin> {
        self.rows
            .iter()
            .find(|row| row.id == id)
            .map(|row| row.origin)
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

    /// What the line said about one of this queue's rows — the panel's
    /// included, whose cleans this table asks (D691).
    fn heard(&mut self, event: &cleaner::Event, cx: &mut Context<Self>) {
        let (id, status) = match event {
            cleaner::Event::Started(id) => (*id, Status::Cleaning),
            cleaner::Event::Finished(id, outcome) => (*id, Status::Done(outcome.clone())),
        };
        let Some(row) = self.rows.iter_mut().find(|row| row.id == id) else {
            return;
        };
        // A picture with no file behind it keeps a preview of its result
        // in place of the one of what arrived (D824).
        if let (Status::Done(outcome), Some(writer), Some(work), Some(arrival)) =
            (&status, &self.writer, &self.work, &row.arrival)
        {
            if let (Some(_), Some(result)) = (unfiled_picture(arrival), &outcome.written) {
                keep_preview(writer, &work.journal, id, Source::File(result.clone()), cx);
            }
        }
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
mod across_tests;
#[cfg(test)]
mod origin_tests;
#[cfg(test)]
mod preview_tests;
#[cfg(test)]
mod redo_tests;
#[cfg(test)]
mod rewrite_tests;
mod rewriting;

/// How often the table looks at the journal's notes — what a test of
/// another window waits for its rows by.
#[cfg(test)]
pub(crate) const TICK: std::time::Duration = rewriting::TICK;

/// Copy the result: row `id`'s result onto the clipboard, whatever it
/// is (D762) — a cleaned text kept in the row, a rewrite's read back from
/// its file or the batch queue's row, or the file a clean wrote: a text in
/// the file's own encoding, a picture as the picture. The person's own
/// document, cleaned: no catalogue touches it on the way there.
fn copy_result(queue: &Entity<Queue>, id: u64, cx: &App) {
    if let Some(text) = queue.read(cx).result_text(id) {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        return;
    }
    // Read back from where it was delivered, off this thread.
    let from = {
        let queue = queue.read(cx);
        queue.rows.iter().find(|row| row.id == id).and_then(|row| {
            row.rewritten(queue.work.as_ref())
                .or_else(|| row.written().map(RewriteFrom::File))
        })
    };
    let Some(from) = from else {
        return;
    };
    cx.spawn(async move |cx| {
        let item = cx
            .background_executor()
            .spawn(async move { copied_of(&from) })
            .await;
        match item {
            Some(item) => cx.update(|cx| cx.write_to_clipboard(item)),
            None => tracing::warn!(id, "a result could not be read back to copy"),
        }
    })
    .detach();
}

/// What Copy the result puts on the clipboard of a result read back from
/// `from`: a picture as its file's own bytes, in the format the bytes say
/// they are — nothing decoded or written again, so what is pasted is what
/// was cleaned (D763) — and anything else as the text it is, without the
/// byte order mark a file may open with: that says how the file is
/// encoded, and pasted it would be an invisible character. `None` when it
/// cannot be read. Blocking.
fn copied_of(from: &RewriteFrom) -> Option<ClipboardItem> {
    if let RewriteFrom::File(path) = from {
        let intake = wipemark_intake::of_path(path);
        if intake.kind == Kind::Image {
            let format = gpui::ImageFormat::from_mime_type(intake.format?.mime())?;
            let bytes = std::fs::read(path).ok()?;
            return Some(ClipboardItem::new_image(&gpui::Image::from_bytes(
                format, bytes,
            )));
        }
    }
    let text = compare::rewritten_text(from).ok()?;
    let text = match text.strip_prefix('\u{FEFF}') {
        Some(rest) => rest.to_owned(),
        None => text,
    };
    Some(ClipboardItem::new_string(text))
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

/// The thumbnail, the excerpt, or the glyph for a thing with neither.
///
/// One frame whatever is in it, so a column of previews reads as a
/// column. The picture and the excerpt are also the trigger for the
/// larger preview; the glyph is not, because there is nothing larger to
/// show.
fn preview_cell(row: &Row, plan_lines: Vec<String>, cx: &App) -> AnyElement {
    let Some(arrival) = &row.arrival else {
        // Read back with nothing behind it, and a preview kept of it
        // (D828): the preview and its card, captioned from the row.
        if let Preview::Image(picture) = &row.preview {
            return kept_cell(row, picture, plan_lines, cx);
        }
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
    let frame = preview_frame(row.id, cx);
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
            picture_cell(row.id, frame, picture, caption_of(intake), plan_lines)
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

/// A picture in the row's frame, and the larger one in a card over it.
fn picture_cell(
    id: u64,
    frame: Div,
    picture: &Picture,
    caption: SharedString,
    plan_lines: Vec<String>,
) -> AnyElement {
    let (small, large) = image_sources(picture);
    let trigger = frame.child(img(small).size_full().object_fit(ObjectFit::Contain));
    HoverCard::new(("preview", id))
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
                        img(large.clone())
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

/// A row with nothing behind it and a preview kept of it (D828): the
/// preview, and a card whose caption is the row's own look and a line
/// saying what the picture is — never the thing itself.
fn kept_cell(row: &Row, picture: &Picture, mut plan_lines: Vec<String>, cx: &App) -> AnyElement {
    plan_lines.insert(0, t(Message::QueuePreviewKept));
    let frame = preview_frame(row.id, cx);
    picture_cell(
        row.id,
        frame,
        picture,
        caption_of_look(&row.look),
        plan_lines,
    )
}

/// The frame a row's preview is drawn in, whatever it holds.
fn preview_frame(id: u64, cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .debug_selector(move || format!("queue-preview-{id}"))
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
        .justify_center()
}

/// The bytes of a picture that arrived with no file behind it — a paste, a
/// drag out of a browser — which a kept preview is made of (D823).
fn unfiled_picture(arrival: &Arrival) -> Option<&[u8]> {
    match &arrival.handed {
        Handed::Bytes { bytes, .. }
            if arrival.intake.path.is_none() && arrival.intake.kind == Kind::Image =>
        {
            Some(bytes)
        }
        _ => None,
    }
}

/// Keep a preview of `source` for the window's row `key`, once `writer` has
/// written everything asked of it before and named the row's journal id —
/// off the GPUI thread, which neither the wait nor the settings row it reads
/// may hold (D827).
fn keep_preview(
    writer: &Writer,
    journal: &Arc<journal::Journal>,
    key: u64,
    source: Source,
    cx: &Context<Queue>,
) {
    let named = writer.id_of(key);
    let journal = Arc::clone(journal);
    cx.background_executor()
        .spawn(async move {
            if let Ok(Some(id)) = named.recv_async().await {
                journal.keep_preview(id, source);
            }
        })
        .detach();
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
        .debug_selector(|| "queue-preview-card".into())
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

/// Name, format and size on one line, for the card of a row read back —
/// what its journal entry says, as [`caption_of`] says it of what arrived.
fn caption_of_look(look: &Look) -> SharedString {
    let mut parts = vec![look.title.clone()];
    parts.extend(look.format.clone());
    parts.extend(look.encoding.clone());
    parts.extend(look.size.map(drop::size_label));
    SharedString::from(parts.join(" · "))
}

/// Where GPUI gets the picture from, for the cell and for the card. The one
/// place a [`Picture`] becomes an `ImageSource` — see that type for why it
/// is not one already. One picture for both but a kept preview, which has a
/// small one for the cell (D824).
fn image_sources(picture: &Picture) -> (ImageSource, ImageSource) {
    match picture {
        Picture::File(path) => (path.clone().into(), path.clone().into()),
        Picture::Bytes(image) => (image.clone().into(), image.clone().into()),
        Picture::Kept(kept) => (kept.small.clone().into(), kept.large.clone().into()),
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
        .tip(sentence)
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
            let sentence = wording::recorded_said(said.action, said.phase, said.outcome.as_ref());
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
        // The tooltip on a frame of our own around the button rather than
        // the button's own, which gpui-component keeps up after the button
        // has gone (D796).
        .child(
            div()
                .id(id)
                .tip(t(Message::QueueCopy))
                .child(Clipboard::new(id).value(value)),
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

/// What a row's button does when pressed.
type Step = Box<dyn Fn(&mut Queue, &mut Context<Queue>)>;

/// What a row's Clean does when pressed, item or button: Clean again's
/// road on a row that would clean again (D703), a first clean of a row
/// whose rewrite left nothing (D720), the first Clean's otherwise.
fn clean_step(id: u64, again: Option<Again>) -> Step {
    match again {
        Some(Again::Clean) => Box::new(move |queue, cx| queue.clean_again(id, cx)),
        Some(Again::Rewrite) => Box::new(move |queue, cx| queue.clean_unrewritten(id, cx)),
        None => Box::new(move |queue, cx| queue.clean(&[id], cx)),
    }
}

/// What a row's Rewrite does when pressed, item or button: Rewrite again's
/// road — which asks first over an edited result (D705) — on a row that
/// would rewrite again, the first Rewrite's otherwise.
fn rewrite_step(id: u64, again: Option<Again>) -> Step {
    match again {
        Some(Again::Rewrite) => Box::new(move |queue, cx| queue.rewrite_again(id, cx)),
        _ => Box::new(move |queue, cx| queue.rewrite(&[id], cx)),
    }
}

/// The row's next steps, on the row (D325): Clean and Rewrite — or Clean
/// again and Rewrite again (D703) — the same two the Actions menu starts
/// with and down the same roads, greyed, with the menu's own reason as the
/// tooltip, when one cannot run. A person who dropped something sees what
/// can be done with it without opening a menu.
fn process_cell(
    id: u64,
    status: &Status,
    clean: Option<String>,
    rewrite: Option<String>,
    queue: Entity<Queue>,
) -> AnyElement {
    let again = again_of(status);
    let (clean_label, rewrite_label) = labels(status);
    let step =
        |label: Message, icon: IconName, key: &'static str, why: Option<String>, run: Step| {
            let queue = queue.clone();
            let tooltip = SharedString::from(why.clone().unwrap_or_else(|| t(label)));
            div()
                .id((key, id))
                .debug_selector(move || format!("{key}-{id}"))
                .tip(tooltip)
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
            clean_label,
            IconName::Broom,
            "row-clean",
            clean,
            clean_step(id, again),
        ))
        .child(step(
            rewrite_label,
            IconName::Pen,
            "row-rewrite",
            rewrite,
            rewrite_step(id, again),
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
    /// The file behind the row, for "Open with" — or why
    /// there is none (D731).
    open: Result<PathBuf, String>,
    /// `None` when Compare opens; otherwise why not (D731).
    compare: Option<String>,
    /// Which of the two the row would do again (D703): what the first two
    /// items are called, and which road each takes.
    again: Option<Again>,
    /// What Clean and Rewrite are called on this row ([`labels`]).
    labels: (Message, Message),
    /// `None` when Clean can be pressed; otherwise why not.
    clean: Option<String>,
    /// `None` when Rewrite can be pressed; otherwise why not.
    rewrite: Option<String>,
    /// Whether a rewrite is queued or running, to cancel.
    cancel: bool,
    /// `None` when the row can be removed; otherwise why not — a clean
    /// holds it, or a caller waits for its rewrite (D355).
    remove: Option<String>,
    /// The result on disk, for Open the result and Show it in its folder —
    /// or why there is none (D731).
    written: Result<PathBuf, String>,
    /// `None` when there is a result to copy, otherwise why not (D731).
    copy: Option<String>,
    /// What Copy the result is called on this row: Copy the result image on
    /// a picture's (D762).
    copy_label: Message,
    /// Whether there is a finished clean to report.
    report: bool,
    /// Why Report… is greyed, when there is a reason to give: a row whose
    /// status the journal keeps, of which only a summary is kept (D448).
    report_why: Option<String>,
}

/// A row's Actions menu as a test reads it: Clean's, Rewrite's and Copy
/// the result's labels, then every item by its key ([`Actions::items`]).
#[cfg(test)]
pub(crate) type Menu = ((Message, Message, Message), Vec<(&'static str, Item)>);

/// An item of the Actions menu as a test reads it: live, or greyed — with
/// the reason drawn under it, or none.
#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Item {
    Live,
    Greyed(Option<String>),
}

#[cfg(test)]
impl Actions {
    /// Every item, in the menu's order, by the key its reason line is drawn
    /// under (`queue-why-<key>-<id>`), with the two labels that change.
    fn items(&self) -> Vec<(&'static str, Item)> {
        let greyed = |why: &Option<String>| match why {
            None => Item::Live,
            Some(why) => Item::Greyed(Some(why.clone())),
        };
        let silent = |live: bool| {
            if live {
                Item::Live
            } else {
                Item::Greyed(None)
            }
        };
        vec![
            ("clean", greyed(&self.clean)),
            ("rewrite", greyed(&self.rewrite)),
            ("cancel", silent(self.cancel)),
            ("open", greyed(&self.open.clone().err())),
            ("compare", greyed(&self.compare)),
            ("open-result", greyed(&self.written.clone().err())),
            ("reveal-result", greyed(&self.written.clone().err())),
            ("copy-result", greyed(&self.copy)),
            (
                "report",
                match &self.report_why {
                    Some(why) => Item::Greyed(Some(why.clone())),
                    None => silent(self.report),
                },
            ),
            ("remove", greyed(&self.remove)),
        ]
    }
}

/// The Actions menu.
///
/// Clean comes first, and is greyed — with the reason under it, because
/// a menu item has no tooltip — for a row that cannot be cleaned, is in
/// line, or was rewritten; on a row whose clean ended it is **Clean
/// again**, and Rewrite on a row whose rewrite ended is **Rewrite again**
/// (D703) — the same item, renamed, never a third one. "Open with"
/// (D761) hands the path to the
/// operating system — `App::open_with_system` is `open` on macOS and
/// `xdg-open` on Linux, on a background task — and is disabled for a
/// row with no file behind it: a note pasted from a chat has nothing to
/// open. "Compare with the result" opens the Compare window on the row,
/// and is disabled for a row that is not text. Then the result: open
/// it, show it in its folder, copy it — a text as characters, a picture
/// as the picture, under "Copy the result image" (D762). (No
/// "Replace the existing result" any more: a row's result takes a name of
/// its own rather than being refused over a file, D638.) Disabled rather
/// than absent, every one, so the
/// menu is the same shape on every row and the reason an item does
/// nothing is that it is greyed, not that it moved. A double click on
/// the row is the Compare item taken the short way — [`row_frame`].
///
/// Every item that can be greyed for a reason says it on the line under
/// its label (D269) — Open, Compare, Open the result, Show the result and
/// Copy the result too (D731), each naming what is missing, never who
/// handed the thing over — through the one [`menu_item`]. Cancel, and a
/// Report… of a row nothing has happened to, are greyed with nothing to
/// add.
fn actions_cell(actions: Actions, queue: Entity<Queue>) -> AnyElement {
    let id = actions.id;
    Button::new(("queue-actions", id))
        .ghost()
        .xsmall()
        .icon(IconName::Ellipsis)
        .tip(SharedString::from(t(Message::QueueActions)))
        .debug_selector(move || format!("row-actions-{id}"))
        .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
            let (opened, open_why) = split(actions.open.clone());
            let (written, written_why) = split(actions.written.clone());
            let revealed = written.clone();
            let (cleaning, comparing, copying, reporting) =
                (queue.clone(), queue.clone(), queue.clone(), queue.clone());
            let (rewriting, cancelling, removing) = (queue.clone(), queue.clone(), queue.clone());
            let (clean_label, rewrite_label) = actions.labels;
            let clean = clean_step(id, actions.again);
            // Rewrite after Clean, greyed with its reason the way Clean is
            // (D269): two actions, because they differ in cost and in where
            // the document goes (В2).
            let rewrite = rewrite_step(id, actions.again);
            menu.item(menu_item(
                (id, "clean"),
                clean_label,
                IconName::Broom,
                actions.clean.clone(),
                move |_, _, cx| cleaning.update(cx, |queue, cx| clean(queue, cx)),
            ))
            .item(menu_item(
                (id, "rewrite"),
                rewrite_label,
                IconName::Pen,
                actions.rewrite.clone(),
                move |_, _, cx| rewriting.update(cx, |queue, cx| rewrite(queue, cx)),
            ))
            .item(
                PopupMenuItem::new(SharedString::from(t(Message::QueueActionCancel)))
                    .icon(IconName::CircleXmark)
                    .disabled(!actions.cancel)
                    .on_click(move |_, _, cx| {
                        cancelling.update(cx, |queue, _| queue.cancel(id));
                    }),
            )
            .item(menu_item(
                (id, "open"),
                Message::QueueActionOpen,
                IconName::ArrowUpRightFromSquare,
                open_why,
                move |_, _, cx| {
                    if let Some(path) = &opened {
                        tracing::info!("opening a queued file with the system");
                        cx.open_with_system(path);
                    }
                },
            ))
            .item(menu_item(
                (id, "compare"),
                Message::QueueActionCompare,
                IconName::CodeCompare,
                actions.compare.clone(),
                move |_, window, cx| compare_row(id, comparing.clone(), window, cx),
            ))
            .separator()
            .item(menu_item(
                (id, "open-result"),
                Message::QueueActionOpenResult,
                IconName::ArrowUpRightFromSquare,
                written_why.clone(),
                move |_, _, cx| {
                    if let Some(written) = &written {
                        tracing::info!(id, "opening a result with the system");
                        cx.open_with_system(written);
                    }
                },
            ))
            .item(menu_item(
                (id, "reveal-result"),
                Message::QueueActionRevealResult,
                IconName::FolderOpen,
                written_why,
                move |_, _, cx| {
                    if let Some(written) = &revealed {
                        cx.reveal_path(written);
                    }
                },
            ))
            .item(menu_item(
                (id, "copy-result"),
                actions.copy_label,
                IconName::Copy,
                actions.copy.clone(),
                move |_, _, cx| copy_result(&copying, id, cx),
            ))
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
                why => menu_item(
                    (id, "report"),
                    Message::QueueActionReport,
                    IconName::FileLines,
                    why,
                    |_, _, _| {},
                ),
            })
            .separator()
            .item(menu_item(
                (id, "remove"),
                Message::QueueActionRemove,
                IconName::Trash,
                actions.remove.clone(),
                move |_, _, cx| removing.update(cx, |queue, cx| queue.remove(id, cx)),
            ))
        })
        .into_any_element()
}

/// A thing to act on, or why there is none — as the menu takes them apart.
fn split<T>(found: Result<T, String>) -> (Option<T>, Option<String>) {
    match found {
        Ok(found) => (Some(found), None),
        Err(why) => (None, Some(why)),
    }
}

/// One item of the Actions menu: live, `run` on a click — or, with `why`,
/// greyed, the reason on a second line under the label in the muted
/// colour, at most 280 points wide (D269, D731). The one way an item of
/// this menu is greyed for a reason, so none is greyed in silence. The
/// reason's line answers to `queue-why-<key>-<row>`, for a test.
fn menu_item(
    (id, key): (u64, &'static str),
    label: Message,
    icon: IconName,
    why: Option<String>,
    run: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> PopupMenuItem {
    let Some(why) = why else {
        return PopupMenuItem::new(SharedString::from(t(label)))
            .icon(icon)
            .on_click(run);
    };
    let why = SharedString::from(why);
    PopupMenuItem::element(move |_, cx| {
        v_flex()
            .max_w(px(280.0))
            .child(SharedString::from(t(label)))
            .child(
                div()
                    .debug_selector(move || format!("queue-why-{key}-{id}"))
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(why.clone()),
            )
    })
    .icon(icon)
    .disabled(true)
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
    let (homed, flushing, tagging) = (queue.clone(), queue.clone(), queue.clone());
    compare::Link {
        tag: Rc::new(move |cx| {
            let Some(queue) = tagging.upgrade() else {
                return journal::Tag::Number(id);
            };
            let queue = queue.read(cx);
            let entry = queue
                .rows
                .iter()
                .find(|row| row.id == id)
                .and_then(|row| row.entry);
            journal::Tag::of(id, entry, queue.writer.as_ref())
        }),
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

    /// Whether `row`'s Compare opens — the item and the double click alike:
    /// a rewrite's result to put beside its original, or a text behind the
    /// row. A row of the command line or of an agent has its text behind it
    /// once its named file is looked at (D729), and opens as a window row
    /// with the same things on disk does (D733).
    fn comparable(&self, row: &Row) -> bool {
        self.compare_greyed(row).is_none()
    }

    /// Why `row`'s Compare is greyed, or `None` when it opens — the one rule
    /// the item, its reason and the double click read ([`why_not_compare`]).
    fn compare_greyed(&self, row: &Row) -> Option<String> {
        why_not_compare(
            row.arrival.as_ref(),
            row.named.is_some(),
            self.made_for(row).is_some(),
        )
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
    /// says which way it currently runs. Drawn in the rows' own sideways
    /// scroll, so a title stays over its values (D811).
    fn header(&self, cx: &Context<Self>) -> Div {
        let theme = cx.theme();
        h_flex()
            .w_full()
            .h(HEADER)
            .flex_shrink_0()
            .bg(theme.table_head)
            .text_color(theme.table_head_foreground)
            .border_b_1()
            .border_color(theme.table_row_border)
            .text_xs()
            .font_semibold()
            .children(Column::ALL.map(|column| {
                let cell = column
                    .cell()
                    .debug_selector(move || format!("queue-head-{column:?}"));
                let title = SharedString::from(t(column.title()));
                match column {
                    Column::Arrived => cell.child(
                        Button::new("sort-arrived")
                            .xsmall()
                            .ghost()
                            .label(title)
                            .icon(self.order.glyph())
                            .tip(SharedString::from(t(self.order.next_label())))
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

        let comparable = self.comparable(row);
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
                &row.status,
                row.clean_why(),
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
                Column::Actions
                    .cell()
                    .child(actions_cell(self.actions_for(row, rewrite), cx.entity())),
            )
            .into_any_element()
    }

    /// What `row`'s Actions menu offers, and why each greyed item is
    /// greyed — read off the row once per frame, the same for every row in
    /// the same state whoever handed it over (D730, D731). `rewrite` is the
    /// Rewrite item's reason, asked once for the rows of a draw (D452).
    fn actions_for(&self, row: &Row, rewrite: Option<String>) -> Actions {
        let named = row.named.is_some();
        let copyable = row.outcome().is_some_and(|outcome| outcome.text.is_some())
            || row.rewritten(self.work.as_ref()).is_some()
            || row.written().is_some();
        Actions {
            id: row.id,
            open: match why_not_open(row.arrival.as_ref(), named) {
                None => row
                    .arrival
                    .as_ref()
                    .and_then(|arrival| arrival.intake.path.clone())
                    .ok_or_else(|| t(Message::QueueActionWhyNoFile)),
                Some(why) => Err(why),
            },
            compare: self.compare_greyed(row),
            again: again_of(&row.status),
            labels: labels(&row.status),
            clean: row.clean_why(),
            rewrite,
            cancel: matches!(row.status, Status::RewriteQueued | Status::Rewriting(_))
                && row.item.is_some(),
            remove: why_not_remove(&row.status, row.origin, row.consented),
            written: row.written().ok_or_else(|| why_no_result_file(&row.status)),
            copy: (!copyable).then(|| why_not_copy(&row.status)),
            copy_label: if row.look.kind == Kind::Image {
                Message::QueueActionCopyResultImage
            } else {
                Message::QueueActionCopyResult
            },
            report: row.outcome().is_some() && row.arrival.is_some(),
            report_why: why_no_report(&row.status),
        }
    }

    /// Row `id`'s Actions menu as a test reads it: its two labels, and
    /// every item, live or greyed with what its line says ([`Item`]).
    #[cfg(test)]
    pub(crate) fn menu_of(&self, id: u64, cx: &App) -> Option<Menu> {
        let row = self.rows.iter().find(|row| row.id == id)?;
        let vacant = self.vacancy(cx);
        let actions = self.actions_for(row, self.why_not_rewrite_given(id, vacant.as_deref()));
        let (clean, rewrite) = actions.labels;
        Some(((clean, rewrite, actions.copy_label), actions.items()))
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
            .debug_selector(|| "queue-footer".into())
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
        let background = cx.theme().table;
        let over = self.catcher.read(cx).over();
        let visible = self.visible();
        let on_page: Vec<usize> =
            visible[page_range(visible.len(), self.page_size, self.page)].to_vec();
        let empty = on_page.is_empty();

        let rows = (!empty).then(|| {
            let count = on_page.len();
            // The duty once for the rows of a draw, never once a row: asking
            // it builds a roster and runs `duty::on_duty` (D452). Taken here,
            // where the list's processor is made, because the list calls
            // that three times a draw — a row measured in its layout and in
            // its prepaint, then the rows on screen.
            let vacant = self.vacancy(cx);
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
            .flex_1()
            .min_h(px(0.0))
            .w_full()
            .track_scroll(&self.scroll)
            // Up and down only (D811). Left to itself a list that scrolls
            // one way turns a sideways swipe into that way: the rows went
            // up and down under a swipe meant to reach Actions.
            .map(|mut rows| {
                rows.style().restrict_scroll_to_axis = Some(true);
                rows
            })
        });

        // The header and the rows, sideways (D811). One scroll holds both,
        // so a title is over its values whatever the offset: gpui-component's
        // table keeps the two in step by sharing one handle between its
        // header and every row's own list, and here they cannot part. The
        // table is the window's width, or every column at its least when
        // that is wider (D812); then a trackpad's sideways swipe, Shift and
        // the wheel — a turn on X by the time GPUI has it: `gpui-pre-linux`
        // swaps it on X11 and Wayland, macOS before AppKit — and the bar
        // along the bottom move it. Sideways only: the wheel stays the
        // rows'. The filter bar and the footer are outside it and never
        // move; the bars are outside it too, on the visible edges. A wider
        // window clamps the offset back, as GPUI clamps every scroll to its
        // content when it lays the frame out (D813).
        let grid = div()
            .relative()
            .flex_1()
            .min_h(px(0.0))
            .child(
                div()
                    .id("queue-across")
                    .size_full()
                    .overflow_x_scroll()
                    .restrict_scroll_to_axis()
                    .track_scroll(&self.across)
                    .child(
                        v_flex()
                            .h_full()
                            .w_full()
                            .min_w(Column::least_table())
                            .child(self.header(cx))
                            .children(rows),
                    ),
            )
            // Over the window's width below the header, not the table's:
            // the invitation sits in the middle of what is seen, and the
            // header keeps its offset over it (D813).
            .when(empty, |grid| {
                grid.child(
                    v_flex()
                        .absolute()
                        .top(HEADER)
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .child(self.empty(over, cx)),
                )
            })
            .when(!empty, |grid| {
                grid.child(
                    div()
                        .debug_selector(|| "queue-rows-bar".into())
                        .absolute()
                        .top(HEADER)
                        .right_0()
                        .bottom_0()
                        .w(SCROLLBAR)
                        // Its own lane as its viewport: the list it scrolls
                        // runs on past the window's right edge.
                        .child(Scrollbar::vertical(&self.scroll).viewport_from_layout()),
                )
            })
            // Drawn only while the columns do not fit: the library's bar
            // hides itself when the table is no wider than the window.
            .child(
                div()
                    .debug_selector(|| "queue-across-bar".into())
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .h(SCROLLBAR)
                    .child(Scrollbar::horizontal(&self.across).viewport_from_layout()),
            );

        // The whole table is the drop zone — see the module docs. The
        // highlight `drop::zone` paints is the table's background going
        // to the accent, which reads as "this will take it" without a
        // second element in the way of the rows.
        drop::zone(v_flex(), &self.catcher, cx)
            .size_full()
            .min_h(px(0.0))
            .bg(background)
            .child(self.filter_bar(visible.len(), cx))
            .child(grid)
            .child(self.footer(visible.len(), cx))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::Cell;
    use std::path::PathBuf;
    use std::rc::Rc;

    use chrono::Local;
    use gpui::prelude::*;
    use gpui::{
        div, point, px, ClipboardItem, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent,
        TestAppContext, VisualTestContext,
    };
    use wipemark_i18n::{t, Message};
    use wipemark_intake::{Handed, Kind};
    use wipemark_store::entry::{Action, Origin, Phase};

    use super::{
        arrived_label, copy_result, kind_glyph, kind_tag, page_range, pages, row_frame, shown,
        why_not_clean, why_not_remove, Cleanable, Column, Filter, Item, Order, Queue, Row, Said,
        Status, PAGE_SIZES, ROW,
    };
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
                    why_not_remove(&status, origin, false),
                    Some(t(Message::QueueActionRemoveWaited)),
                    "{origin:?} {status:?}"
                );
            }
        }
        assert!(why_not_remove(&Status::RewriteQueued, Origin::Window, true).is_none());
        assert!(why_not_remove(&Status::Cleaning, Origin::Window, true).is_some());
        let ended = Status::Recorded(Box::new(Said {
            action: Action::Rewrite,
            phase: Phase::Done,
            outcome: None,
            result: None,
        }));
        assert!(why_not_remove(&ended, Origin::Agent, false).is_none());
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
                            Some("not-rewritten") => "not-rewritten",
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

    /// `--clean=` is Import and Clean: the row lands and is cleaned. The
    /// owner's F10, cleaned: one file handed over twice with a result
    /// already beside it — two rows, two results of their own, named for
    /// each row's tag, the file already there left byte for byte, and each
    /// row's note naming the file it really wrote (D635).
    #[gpui::test]
    fn the_same_file_twice_is_two_rows_with_two_results(cx: &mut TestAppContext) {
        let scratch = Scratch::new("own");
        let source = scratch.file("x.md", MARKED.as_bytes());
        let existing = scratch.file("x.cleaned.md", b"somebody's own file");
        let (queue, _, cx) = queue_in(cx, &scratch);

        queue.update(cx, |queue, cx| {
            queue.hand_to_clean(vec![source.clone(), source.clone()], cx)
        });
        cx.run_until_parked();
        assert_eq!(statuses(&queue, cx), ["cleaned", "cleaned"]);
        assert_eq!(
            std::fs::read(&existing).expect("read"),
            b"somebody's own file"
        );
        let cleaned = wipemark_core::clean(MARKED, &wipemark_core::Options::default()).text;
        let rows: Vec<(String, std::path::PathBuf, Vec<String>)> = cx.update(|_, cx| {
            queue
                .read(cx)
                .rows
                .iter()
                .map(|row| match &row.status {
                    Status::Done(outcome) => (
                        row.entry.map_or(row.id.to_string(), |id| id.to_string()),
                        outcome.written.clone().expect("written"),
                        wording::went_in(&wording::window, outcome),
                    ),
                    other => panic!("not done: {other:?}"),
                })
                .collect()
        });
        let mut written = Vec::new();
        for (tag, path, went) in rows {
            let own = wipemark_intake::name::with_tag("x.md", "cleaned", &tag);
            assert_eq!(path, scratch.0.join(&own));
            assert_eq!(std::fs::read_to_string(&path).expect("read"), cleaned);
            assert!(
                went.iter().any(|line| line.contains(&own)),
                "the row does not name {own}: {went:?}"
            );
            written.push(own);
        }
        assert_ne!(written[0], written[1], "two rows, one name");
        assert_eq!(std::fs::read(&source).expect("read"), MARKED.as_bytes());
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

    /// What an update's restart would drop is counted (D785, D419): a
    /// cleaned paste lives only in memory; a cleaned file's result is on
    /// disk and is not. And nothing runs once the clean is over (D788).
    #[gpui::test]
    fn a_cleaned_paste_is_the_one_thing_a_restart_drops(cx: &mut TestAppContext) {
        let scratch = Scratch::new("restart-drops");
        let (queue, _preferences, cx) = queue_in(cx, &scratch);
        let file = scratch.0.join("note.txt");
        std::fs::write(&file, "file\u{200B}text").expect("a file");
        queue.update(cx, |queue, cx| {
            queue.land(
                vec![
                    Handed::Text("paste\u{200B}me".to_owned()),
                    Handed::Path(file.clone()),
                ],
                cx,
            )
        });
        cx.run_until_parked();
        assert_eq!(cx.update(|_, cx| queue.read(cx).texts_only_in_memory()), 0);
        let ids = cx.update(|_, cx| queue.read(cx).ids());
        queue.update(cx, |queue, cx| queue.clean(&ids, cx));
        cx.run_until_parked();
        assert_eq!(statuses(&queue, cx), ["cleaned", "cleaned"]);
        assert_eq!(cx.update(|_, cx| queue.read(cx).texts_only_in_memory()), 1);
        assert_eq!(
            cx.update(|_, cx| queue.read(cx).at_work(cx)),
            (false, false)
        );
    }

    /// The result `id` wrote, read off its row.
    fn written_of(queue: &gpui::Entity<Queue>, id: u64, cx: &mut VisualTestContext) -> PathBuf {
        cx.update(|_, cx| {
            queue
                .read(cx)
                .rows
                .iter()
                .find(|row| row.id == id)
                .and_then(Row::written)
        })
        .expect("a result file")
    }

    /// D762: a text file's clean writes its result beside it, in the
    /// file's own encoding — UTF-16 here — and Copy the result puts that
    /// result on the clipboard as characters, read back in that encoding,
    /// without the byte order mark the file opens with. The item is live on
    /// the row and called Copy the result. Read the result as a text kept
    /// in the row only, as before, and the clipboard stays empty: red; keep
    /// the mark and it is an invisible character on the clipboard: red.
    #[gpui::test]
    fn a_cleaned_file_s_result_is_copied_as_its_text(cx: &mut TestAppContext) {
        let scratch = Scratch::new("copy-text-file");
        let mut utf16 = vec![0xFF, 0xFE];
        utf16.extend(MARKED.encode_utf16().flat_map(u16::to_le_bytes));
        let note = scratch.file("note.md", &utf16);
        let (queue, _, cx) = queue_in(cx, &scratch);

        queue.update(cx, |queue, cx| queue.hand(vec![note], cx));
        cx.run_until_parked();
        let ids = cx.update(|_, cx| queue.read(cx).ids());
        queue.update(cx, |queue, cx| queue.clean(&ids, cx));
        cx.run_until_parked();
        assert_eq!(statuses(&queue, cx), ["cleaned"]);
        let written = written_of(&queue, ids[0], cx);
        assert_ne!(
            std::fs::read(&written).expect("the result"),
            utf16,
            "nothing was cleaned"
        );

        let ((_, _, copy), items) = cx
            .update(|_, cx| queue.read(cx).menu_of(ids[0], cx))
            .expect("a menu");
        assert_eq!(copy, Message::QueueActionCopyResult);
        assert!(items.contains(&("copy-result", Item::Live)), "{items:?}");

        cx.update(|_, cx| cx.write_to_clipboard(ClipboardItem::new_string(String::new())));
        cx.update(|_, cx| copy_result(&queue, ids[0], cx));
        cx.run_until_parked();
        let copied = cx.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
        assert_eq!(
            copied.as_deref(),
            Some("# Notes\n\nA zerowidth space.\n"),
            "the copy is not the result's text"
        );
    }

    /// D762, D763: a picture's clean writes its result as a file, and Copy
    /// the result image puts that picture on the clipboard — the result's
    /// own bytes in its own format, PNG as PNG and JPEG as JPEG, nothing
    /// decoded or written again. Greyed as "the result is a file", or a
    /// copy of the text of the bytes, and it is red.
    #[gpui::test]
    fn a_cleaned_picture_is_copied_as_the_picture(cx: &mut TestAppContext) {
        let scratch = Scratch::new("copy-picture");
        let fixture = |name: &str| {
            std::fs::read(format!(
                "{}/../../fixtures/image/{name}",
                env!("CARGO_MANIFEST_DIR")
            ))
            .expect("a fixture")
        };
        let png = scratch.file("shot.png", &fixture("xmp-provenance-url.png"));
        let jpeg = scratch.file("photo.jpg", &fixture("xmp-provenance.jpg"));
        let (queue, _, cx) = queue_in(cx, &scratch);

        queue.update(cx, |queue, cx| queue.hand(vec![png, jpeg], cx));
        cx.run_until_parked();
        let ids = cx.update(|_, cx| queue.read(cx).ids());
        queue.update(cx, |queue, cx| queue.clean(&ids, cx));
        cx.run_until_parked();
        assert_eq!(statuses(&queue, cx), ["cleaned", "cleaned"]);

        for (id, format) in ids
            .into_iter()
            .zip([gpui::ImageFormat::Png, gpui::ImageFormat::Jpeg])
        {
            let ((_, _, copy), items) = cx
                .update(|_, cx| queue.read(cx).menu_of(id, cx))
                .expect("a menu");
            assert_eq!(copy, Message::QueueActionCopyResultImage, "{format:?}");
            assert!(items.contains(&("copy-result", Item::Live)), "{items:?}");

            cx.update(|_, cx| cx.write_to_clipboard(ClipboardItem::new_string(String::new())));
            cx.update(|_, cx| copy_result(&queue, id, cx));
            cx.run_until_parked();
            let copied = cx
                .update(|_, cx| cx.read_from_clipboard())
                .expect("the clipboard");
            let written = std::fs::read(written_of(&queue, id, cx)).expect("the result");
            assert_eq!(
                copied.entries(),
                &[gpui::ClipboardEntry::Image(gpui::Image::from_bytes(
                    format, written
                ))],
                "the copy is not the result picture"
            );
        }
    }
}
