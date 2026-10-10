//! Which screen a window opens on, and where on it.
//!
//! Epic **E6**. [`screen`](crate::screen) reduces a display to the three
//! facts placing a window needs and does the arithmetic; this module is
//! the *decision* on top of it — the one place that answers "which of
//! these screens, and which part of it" — and the vocabulary the
//! Placement page is built from.
//!
//! It is `duty.rs`'s shape applied to a window instead of a rewrite:
//! [`screen_for`] is a pure function over candidates the caller has
//! already gathered, because everything that finds a screen has to touch
//! the platform and nothing that decides between them should.
//!
//! # Two questions, and only one of them is one preference
//!
//! **Which screen** is a single answer for the whole product
//! ([`Onto`]): the primary display, or the one the user is on. It is a
//! row, it has a widget, and it is the same on every machine the
//! database is copied to.
//!
//! **Where on that screen** is a different answer *per display*
//! ([`Zone`]), because that is what a person means by it: the laptop
//! panel is small and a window belongs in the middle of it; the 32-inch
//! display beside it has a corner the user keeps free. So the zone is
//! filed under the display's uuid, exactly the way
//! [`window_state`](crate::window_state) files a rectangle, and for
//! exactly the same reason — see the note at the top of
//! [`screen`](crate::screen) on why a rectangle only means anything
//! beside the display it was measured on.
//!
//! Every display starts at [`Zone::default`] — the bottom right — and
//! not at "wherever it lands". A default that is a *place* is what
//! makes the six cells mean something on the first visit: the page
//! opens with one ticked, the window is already in it, and dragging it
//! to another is one gesture that visibly does what it says.
//!
//! # A hand beats a cell, and the last word wins
//!
//! The window can also be moved and resized by hand, and that is an
//! instruction too — a more specific one than a sixth of a screen. So
//! [`Answer::Manual`] is the second answer a display can hold, it wins
//! over any cell, and the rectangle it names is kept beside it. Click a
//! cell afterwards and the cell wins again; the rectangle stays, for
//! its **size**, because a cell says where a window goes and never how
//! big it is. Last word wins, in both directions, which is the only
//! rule a person can predict from outside.
//!
//! That is also why the zones are **not** rows in the Settings window's
//! [`Setting`](crate::settings::Setting) list: the set of keys is the
//! set of displays this machine has, which is not known when the binary
//! is built and changes while it runs. `every_persisted_preference_has_a_row`
//! walks a static list and cannot walk this one; what stands in for it
//! is [`a_zone_is_never_filed_beside_a_preference`] below and the
//! grid on the page, which draws one card per connected display and so
//! cannot leave a display unreachable.
//!
//! # Six rectangles, three across
//!
//! A screen's visible area divided into [`Zone::COLUMNS`] × [`Zone::ROWS`]
//! equal rectangles, and the window is centred on the one that was
//! chosen. Centred rather than flush into a corner, because a window is
//! very often *bigger* than a sixth of a screen — Settings is 820 points
//! wide and a third of a laptop panel is 504 — and centring degrades
//! into exactly the corner behaviour once
//! [`screen::contained`](crate::screen::contained) pulls it back onto
//! the screen. One rule, two useful behaviours, and no special case for
//! "the window does not fit in its cell".
//!
//! The grid is three across and two down on **every** screen, including
//! a portrait one, where the cells come out tall and narrow. The
//! alternative — flipping to two across and three down when a display is
//! taller than it is wide — was rejected because it changes what a
//! *stored* value means: `bottom-right` would name a different sixth of
//! the same display after the user rotated it, and a preference that
//! moves on its own is one nobody can trust.

use std::collections::BTreeMap;

use anyhow::Result;
use gpui::{point, px, size, App, Bounds, Pixels, Size};
use serde::{Deserialize, Serialize};
use wipemark_i18n::Message;
use wipemark_store::Store;

use crate::screen::{self, Screen};

/// Rows are `window.zone.<display uuid>`, beside the rectangles
/// [`window_state`](crate::window_state) writes and under the same
/// prefix, because both are facts about one display and neither is a
/// preference with a fixed key.
///
/// A **format**: renaming it forgets every chosen zone on every machine.
const PREFIX: &str = "window.zone.";

/// Where the request came from.
///
/// It decides two things and no others: which screen [`Onto::Active`]
/// means, and whether there is a window to centre over when nothing
/// else has an opinion. A click on the menu bar says nothing about
/// which display the user is looking at — the menu bar is drawn on
/// every one of them — so the pointer, which is a hand's width from
/// that click, answers for it instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// The gear in the status bar, or `secondary-,`. There is a window
    /// on screen, the user is looking at it, and the settings belong
    /// over it.
    MainWindow,
    /// The menu bar. The main window may be hidden, so the pointer is
    /// what says which screen the user is on.
    Tray,
}

/// Which screen a window opens on.
///
/// Two choices, and [`Onto::Active`] is the default because it is what
/// this application did before there was a preference at all: open
/// where the user asked from. Anyone who never visits this page sees no
/// change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Onto {
    /// The screen the request came from — the main window's, or the
    /// pointer's for a click on the menu bar.
    #[default]
    Active,
    /// The display the platform calls primary, wherever the request
    /// came from. The answer for a desk where one screen is *the*
    /// screen and the laptop panel beside it holds mail.
    Primary,
}

impl Onto {
    /// Both choices, in the order the radio buttons list them.
    pub const ALL: [Onto; 2] = [Self::Active, Self::Primary];

    /// The value written to the row. A **format**: never localized, and
    /// never derived from the label.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Primary => "primary",
        }
    }

    /// The stored value back, or `None` for a word this build does not
    /// know — which is read as the default and **left in the row**, the
    /// rule every other preference here follows.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|onto| onto.as_str() == value)
    }

    /// The radio button's label.
    pub fn label(self) -> Message {
        match self {
            Self::Active => Message::SettingsPlacementScreenActive,
            Self::Primary => Message::SettingsPlacementScreenPrimary,
        }
    }
}

/// One of the six rectangles a screen's visible area is divided into.
///
/// The names are the picture: three columns across, two rows down, read
/// the way the page draws them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Zone {
    TopLeft,
    TopCentre,
    TopRight,
    BottomLeft,
    BottomCentre,
    /// The default for every display that has not been given an
    /// answer. Furthest from the menu bar and from the top-left corner
    /// every other application opens into, and on the side of the
    /// screen a right-handed pointer is already near.
    #[default]
    BottomRight,
}

impl Zone {
    /// How many cells across the screen is divided into.
    pub const COLUMNS: usize = 3;
    /// And how many down. Three by two is six, which is the number a
    /// person can point at without counting.
    pub const ROWS: usize = 2;

    /// Every zone, in reading order — which is also the order the grid
    /// lays them out, so the page can zip the two without a table.
    pub const ALL: [Zone; 6] = [
        Self::TopLeft,
        Self::TopCentre,
        Self::TopRight,
        Self::BottomLeft,
        Self::BottomCentre,
        Self::BottomRight,
    ];

    /// The zone at this column and row, or `None` for coordinates off
    /// the grid.
    pub fn at(column: usize, row: usize) -> Option<Self> {
        (column < Self::COLUMNS && row < Self::ROWS)
            .then(|| Self::ALL[row * Self::COLUMNS + column])
    }

    /// Which column it is in, counting from the left.
    ///
    /// Spelled out rather than derived from the position in
    /// [`Zone::ALL`], which is the *other* spelling of the same fact.
    /// Two independent statements that `the_grid_and_the_names_are_one_thing`
    /// checks against each other: a zone inserted into `ALL` in the
    /// wrong place turns the suite red instead of drawing one cell
    /// under another cell's name.
    pub fn column(self) -> usize {
        match self {
            Self::TopLeft | Self::BottomLeft => 0,
            Self::TopCentre | Self::BottomCentre => 1,
            Self::TopRight | Self::BottomRight => 2,
        }
    }

    /// Which row it is in, counting from the top.
    pub fn row(self) -> usize {
        match self {
            Self::TopLeft | Self::TopCentre | Self::TopRight => 0,
            Self::BottomLeft | Self::BottomCentre | Self::BottomRight => 1,
        }
    }

    /// The value written to the row, and a **format**: a zone saved by
    /// this build is read by the next one, so these words are not
    /// spelling and not translation.
    pub fn id(self) -> &'static str {
        match self {
            Self::TopLeft => "top-left",
            Self::TopCentre => "top-centre",
            Self::TopRight => "top-right",
            Self::BottomLeft => "bottom-left",
            Self::BottomCentre => "bottom-centre",
            Self::BottomRight => "bottom-right",
        }
    }

    /// The stored value back, or `None` for a name this build does not
    /// know.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|zone| zone.id() == value)
    }

    /// What the cell is called on screen — a tooltip on the cell, and
    /// the sentence under the grid once one is chosen.
    pub fn label(self) -> Message {
        match self {
            Self::TopLeft => Message::SettingsPlacementZoneTopLeft,
            Self::TopCentre => Message::SettingsPlacementZoneTopCentre,
            Self::TopRight => Message::SettingsPlacementZoneTopRight,
            Self::BottomLeft => Message::SettingsPlacementZoneBottomLeft,
            Self::BottomCentre => Message::SettingsPlacementZoneBottomCentre,
            Self::BottomRight => Message::SettingsPlacementZoneBottomRight,
        }
    }
}

/// The screens a placement can choose between, gathered by the caller.
///
/// Every one of these costs a trip to the platform, and two of the
/// three can answer `None` on a machine in a state a desktop is not
/// usually in. Gathering them outside keeps [`screen_for`] a function
/// over values — which is what makes the promise below testable without
/// a display attached.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Candidates {
    /// The screen the pointer is on right now.
    pub pointer: Option<Screen>,
    /// The screen the main window is on.
    pub main: Option<Screen>,
    /// The display the platform calls primary.
    pub primary: Option<Screen>,
}

/// Which screen the window opens on.
///
/// `Primary` is the preference that says "always that one", so it is
/// answered first and falls through only when the platform has no
/// primary display to name — at which point *any* screen beats
/// refusing to open the window, and the active chain answers.
///
/// `Active` is what this application did before the preference existed,
/// spelled out: the window that asked, then the pointer, then the
/// primary display. The menu bar reverses the first two, because a
/// click on it happened on every display at once.
pub fn screen_for(onto: Onto, asked_from: Origin, candidates: &Candidates) -> Option<Screen> {
    let Candidates {
        pointer,
        main,
        primary,
    } = candidates;
    let active = match asked_from {
        Origin::MainWindow => main.clone().or_else(|| pointer.clone()),
        Origin::Tray => pointer.clone().or_else(|| main.clone()),
    };
    match onto {
        Onto::Primary => primary.clone().or(active),
        Onto::Active => active.or_else(|| primary.clone()),
    }
}

/// Where the product's own window opens: which display, and the
/// rectangle to ask GPUI for.
///
/// The entry point for a caller with no window to measure anything off
/// — the panel, every time it is summoned. GPUI is handed a *content*
/// rectangle and reports a *frame* one (see `settings::geometry_of`),
/// and the difference can only be measured off a window that already
/// exists, so the caller states it: `chrome` is zero for the panel,
/// whose content view is its whole frame, and a title bar's height for
/// a window that has one. "The active screen" can only mean the screen
/// the pointer is on, for the same reason — there is no window for it
/// to mean instead.
///
/// Returns the screen **beside** the rectangle, because the two are one
/// fact in two halves: `WindowOptions::window_bounds` is read against
/// `display_id`, and a rectangle handed over without one is read
/// against the primary display whatever screen it was measured on. The
/// caller keeps the rest of the screen for the same reason it was
/// needed here — it is what a later hand-placed rectangle is filed
/// under.
pub fn opening(
    onto: Onto,
    spots: &BTreeMap<String, Spot>,
    wanted: Size<Pixels>,
    chrome: Pixels,
    cx: &App,
) -> (Option<Screen>, Bounds<Pixels>) {
    let candidates = Candidates {
        pointer: screen::under_the_pointer(cx),
        main: None,
        primary: screen::primary(cx),
    };
    let Some(screen) = screen_for(onto, Origin::MainWindow, &candidates) else {
        // No display the platform will name. GPUI still places a
        // window given a rectangle it cannot check, and a window
        // somewhere beats no window.
        return (None, Bounds::centered(None, wanted, cx));
    };

    let frame = where_it_goes(
        spot_for(spots, &screen),
        frame_of_size(wanted, chrome),
        screen.visible,
    );
    (Some(screen.clone()), content_of(frame, chrome))
}

/// The rectangle `zone` names inside a screen's visible area.
///
/// Computed from the far edge rather than by multiplying a cell width,
/// so the right-hand column ends exactly at the right-hand edge however
/// the division rounds. A grid with a one-pixel gutter down the last
/// column is the kind of thing nobody reports and everybody sees.
pub fn cell(zone: Zone, within: Bounds<Pixels>) -> Bounds<Pixels> {
    let left = within.origin.x.as_f32();
    let top = within.origin.y.as_f32();
    let width = within.size.width.as_f32();
    let height = within.size.height.as_f32();

    let edge =
        |start: usize, of: usize, from: f32, span: f32| from + span * start as f32 / of as f32;

    let x0 = edge(zone.column(), Zone::COLUMNS, left, width);
    let x1 = edge(zone.column() + 1, Zone::COLUMNS, left, width);
    let y0 = edge(zone.row(), Zone::ROWS, top, height);
    let y1 = edge(zone.row() + 1, Zone::ROWS, top, height);

    Bounds::new(point(px(x0), px(y0)), size(px(x1 - x0), px(y1 - y0)))
}

/// Where a window of `wanted` goes on a screen whose zone is `zone`.
///
/// Centred on the cell and then pulled back onto the screen, which is
/// the whole rule — see the note at the top of this module on why that
/// is one rule and not two.
pub fn at(zone: Zone, wanted: Size<Pixels>, visible: Bounds<Pixels>) -> Bounds<Pixels> {
    screen::contained(screen::centred(wanted, cell(zone, visible)), visible)
}

/// What one display was told, and the rectangle behind it.
///
/// Two facts in one value because they answer one question between
/// them and are read together every time. `at` is what the user last
/// *said*; `rect` is where they last put the window by hand on this
/// display — kept even while a cell is chosen, because it carries the
/// **size** they gave it and a cell never does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spot {
    /// The answer in force.
    pub at: Answer,
    /// The last rectangle the user put the window at by hand, in this
    /// display's own coordinates.
    pub rect: Option<Bounds<Pixels>>,
}

impl Default for Spot {
    fn default() -> Self {
        Self {
            at: Answer::Zone(Zone::default()),
            rect: None,
        }
    }
}

/// What decides where the window goes on one display.
///
/// Two answers, and the second is the one a hand gives. Dragging the
/// window or pulling its edge is an instruction as much as clicking a
/// cell is — a more specific one — so it becomes the answer, and the
/// grid stops ticking a cell until the user clicks one again. Last
/// word wins, in both directions, which is the only rule a person can
/// predict from the outside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// One of the six cells.
    Zone(Zone),
    /// Exactly where it was put, at the size it was given.
    Manual,
}

impl Answer {
    /// The word written to the row. A **format**: never localized, and
    /// never spelled from a label.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Zone(zone) => zone.id(),
            Self::Manual => MANUAL,
        }
    }

    /// The stored word back, or `None` for one this build does not
    /// know — read as no answer at all, and **left in the row**.
    pub fn parse(value: &str) -> Option<Self> {
        if value == MANUAL {
            return Some(Self::Manual);
        }
        Zone::parse(value).map(Self::Zone)
    }
}

/// The word that means "where I put it". Not a zone, and deliberately
/// not the empty string: an absent row is "nobody has answered", which
/// is a third thing and takes the default.
const MANUAL: &str = "manual";

/// A rectangle as it goes into a row.
///
/// Four numbers with names rather than a `Bounds<Pixels>`, for the
/// reason `window_state::Geometry` is one: this is a file format, and a
/// row written by this build has to still be readable by a build whose
/// GPUI has moved on.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Geometry {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

/// One display's row, as it is stored.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Row {
    /// A cell's name, or `manual`.
    at: String,
    /// The hand-placed rectangle, once there has been one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rect: Option<Geometry>,
}

/// What every display has been told, by display uuid.
///
/// A display **missing** from this map is a display nobody has
/// answered for: it takes [`Spot::default`], which is a place rather
/// than an absence — see the note at the top of this module.
///
/// One prefix scan and not a read per display, and it happens once at
/// startup: the page draws a card per connected screen on every frame,
/// and a query per card per frame is a query per card per frame — the
/// same reason the saved profiles are held in memory. `Preferences` is
/// the only writer, so the map it holds and the rows here cannot
/// disagree.
///
/// A row this build cannot read is dropped from the map and **left in
/// the database**, the rule `config` keeps for every unusable value: a
/// build that grows a seventh cell makes it come true without the user
/// touching anything.
pub fn all(store: &Store) -> BTreeMap<String, Spot> {
    let rows = match store.settings().all() {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(%error, "could not read where the windows go; every screen takes the default");
            return BTreeMap::new();
        }
    };

    rows.into_iter()
        .filter_map(|(key, value)| {
            let uuid = key.strip_prefix(PREFIX)?;
            let row: Row = match serde_json::from_value(value) {
                Ok(row) => row,
                Err(error) => {
                    tracing::warn!(%key, %error, "unreadable placement; this screen takes the default");
                    return None;
                }
            };
            let Some(at) = Answer::parse(&row.at) else {
                tracing::warn!(%key, at = %row.at, "unknown placement; this screen takes the default");
                return None;
            };
            let rect = row.rect.map(|rect| {
                Bounds::new(
                    point(px(rect.x), px(rect.y)),
                    size(px(rect.width), px(rect.height)),
                )
            });
            Some((uuid.to_owned(), Spot { at, rect }))
        })
        .collect()
}

/// What this screen was told, or the default for a screen nobody has
/// answered for.
///
/// The one place the default lives, so the page that draws a tick and
/// the code that places a window cannot disagree about what an
/// unanswered display means. A screen the platform will not name
/// stably has no row to find and takes the default like any other.
pub fn spot_for(spots: &BTreeMap<String, Spot>, screen: &Screen) -> Spot {
    screen
        .key
        .as_ref()
        .and_then(|key| spots.get(key.as_str()).copied())
        .unwrap_or_default()
}

/// Where a window of `wanted` goes on this screen.
///
/// The precedence, in one expression and in one place: a hand-placed
/// rectangle is used as it is, a cell is used at whatever size the hand
/// last gave the window, and a screen nobody has answered for takes the
/// default cell at the default size. Everything is then pulled back
/// onto the screen, because a rectangle remembered from a display that
/// has since changed resolution names a position that no longer exists.
pub fn where_it_goes(spot: Spot, wanted: Size<Pixels>, visible: Bounds<Pixels>) -> Bounds<Pixels> {
    let (zone, rect) = match spot {
        Spot {
            at: Answer::Manual,
            rect: Some(rect),
        } => (None, Some(rect)),
        // Told "where I put it" and never put anywhere: there is
        // nothing to honour, so the default cell answers rather than
        // the platform.
        Spot {
            at: Answer::Manual,
            rect: None,
        } => (Some(Zone::default()), None),
        Spot {
            at: Answer::Zone(zone),
            rect,
        } => (Some(zone), rect),
    };

    screen::contained(frame_for(zone, rect, wanted, None, visible), visible)
}

/// Remember what this screen was told.
///
/// A screen the platform will not name stably is never written, and
/// that is not an error — the same rule
/// [`window_state::save`](crate::window_state::save) keeps, and for the
/// same reason: the numeric display id is handed out by the window
/// server and comes back attached to a different monitor after a
/// reboot. The caller has already said so on screen.
pub fn save(store: &Store, screen: &Screen, spot: Spot) -> Result<()> {
    let Some(key) = screen.key.as_ref().map(|key| format!("{PREFIX}{key}")) else {
        return Ok(());
    };
    let row = Row {
        at: spot.at.as_str().to_owned(),
        rect: spot.rect.map(|rect| Geometry {
            x: rect.origin.x.as_f32(),
            y: rect.origin.y.as_f32(),
            width: rect.size.width.as_f32(),
            height: rect.size.height.as_f32(),
        }),
    };
    store.settings().set(&key, &row)?;
    Ok(())
}

/// The frame a window with this content rectangle will have.
pub fn frame_of(content: Bounds<Pixels>, chrome: Pixels) -> Bounds<Pixels> {
    Bounds::new(
        content.origin,
        size(content.size.width, content.size.height + chrome),
    )
}

/// The frame a window with this content *size* will have.
pub fn frame_of_size(content: Size<Pixels>, chrome: Pixels) -> Size<Pixels> {
    size(content.width, content.height + chrome)
}

/// The content rectangle to ask for, to end up with this frame.
pub fn content_of(frame: Bounds<Pixels>, chrome: Pixels) -> Bounds<Pixels> {
    Bounds::new(
        frame.origin,
        size(frame.size.width, frame.size.height - chrome),
    )
}

/// The same rectangle, grown to `minimum` in either direction it falls
/// short — keeping its origin, because that is the half the user chose.
pub fn at_least(frame: Bounds<Pixels>, minimum: Size<Pixels>) -> Bounds<Pixels> {
    Bounds::new(
        frame.origin,
        size(
            px(frame.size.width.as_f32().max(minimum.width.as_f32())),
            px(frame.size.height.as_f32().max(minimum.height.as_f32())),
        ),
    )
}

/// Where a window goes on the screen that was chosen for it, in frame
/// rectangles.
///
/// Three answers in precedence order, and the order is the point. A
/// **zone** is an instruction and beats everything: somebody pointed at
/// a part of that screen and said "there". What is **remembered** is
/// the user too, one step less explicit — it is where they dragged the
/// window last time. Failing both, the window is **centred**, over
/// whatever `over` names when there is something to centre over and
/// over the screen itself otherwise.
///
/// A zone answers *where* and never *how big*: the size is whatever was
/// remembered, and the default only for a screen no window has ever
/// opened on. Losing a size the user chose because they also chose a
/// corner would be one preference undoing another.
pub fn frame_for(
    zone: Option<Zone>,
    remembered: Option<Bounds<Pixels>>,
    default: Size<Pixels>,
    over: Option<Bounds<Pixels>>,
    visible: Bounds<Pixels>,
) -> Bounds<Pixels> {
    match zone {
        Some(zone) => at(
            zone,
            remembered.map_or(default, |frame| frame.size),
            visible,
        ),
        None => remembered.unwrap_or_else(|| screen::centred(default, over.unwrap_or(visible))),
    }
}

/// Forget what this screen was told.
///
/// The row goes rather than being written empty: an absent row is "no
/// answer", which is exactly what restoring the default means, and it
/// is the one state that follows a later build's default rather than
/// pinning this one's.
pub fn forget(store: &Store, screen: &Screen) -> Result<()> {
    let Some(key) = screen.key.as_ref().map(|key| format!("{PREFIX}{key}")) else {
        return Ok(());
    };
    store.settings().delete(&key)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use gpui::DisplayId;

    use super::*;

    fn screen(key: &str) -> Screen {
        Screen {
            id: DisplayId::new(1),
            key: Some(key.to_owned()),
            visible: bounds(0.0, 25.0, 1200.0, 575.0),
        }
    }

    fn bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds::new(point(px(x), px(y)), size(px(w), px(h)))
    }

    /// The default is the behaviour that was there before the
    /// preference was: open where the request came from.
    #[test]
    fn the_default_is_what_the_application_already_did() {
        assert_eq!(Onto::default(), Onto::Active);

        let main = screen("uuid-main");
        let pointer = screen("uuid-pointer");
        let candidates = Candidates {
            pointer: Some(pointer.clone()),
            main: Some(main.clone()),
            primary: Some(screen("uuid-primary")),
        };

        assert_eq!(
            screen_for(Onto::Active, Origin::MainWindow, &candidates),
            Some(main),
            "the window that asked is the active screen"
        );
        assert_eq!(
            screen_for(Onto::Active, Origin::Tray, &candidates),
            Some(pointer),
            "the menu bar is on every display; the pointer is not"
        );
    }

    /// The whole point of the other choice: wherever it was asked from,
    /// the answer is the same screen.
    #[test]
    fn the_primary_choice_ignores_where_the_request_came_from() {
        let primary = screen("uuid-primary");
        let candidates = Candidates {
            pointer: Some(screen("uuid-pointer")),
            main: Some(screen("uuid-main")),
            primary: Some(primary.clone()),
        };

        for origin in [Origin::MainWindow, Origin::Tray] {
            assert_eq!(
                screen_for(Onto::Primary, origin, &candidates),
                Some(primary.clone()),
                "{origin:?} moved a window off the primary screen"
            );
        }
    }

    /// A machine that cannot name a primary display is not a machine
    /// that gets no window.
    #[test]
    fn a_preference_that_cannot_be_honoured_still_opens_the_window() {
        let main = screen("uuid-main");
        let candidates = Candidates {
            pointer: None,
            main: Some(main.clone()),
            primary: None,
        };
        assert_eq!(
            screen_for(Onto::Primary, Origin::MainWindow, &candidates),
            Some(main)
        );
        assert_eq!(
            screen_for(Onto::Active, Origin::Tray, &Candidates::default()),
            None,
            "no display at all is the one case with no answer"
        );
    }

    /// Six cells, no overlaps, no gaps, and the outer edges are the
    /// screen's own.
    #[test]
    fn the_six_cells_tile_the_visible_area() {
        let visible = bounds(0.0, 25.0, 1200.0, 600.0);
        let cells: Vec<Bounds<Pixels>> = Zone::ALL
            .into_iter()
            .map(|zone| cell(zone, visible))
            .collect();

        let area: f32 = cells
            .iter()
            .map(|cell| cell.size.width.as_f32() * cell.size.height.as_f32())
            .sum();
        assert_eq!(area, 1200.0 * 600.0, "the cells do not cover the screen");

        assert_eq!(cells[0], bounds(0.0, 25.0, 400.0, 300.0));
        assert_eq!(
            cells[2].origin.x.as_f32() + cells[2].size.width.as_f32(),
            1200.0
        );
        assert_eq!(
            cells[5].origin.y.as_f32() + cells[5].size.height.as_f32(),
            625.0
        );
    }

    /// The menu bar is above the visible area, so a top zone starts
    /// under it rather than at the top of the display.
    #[test]
    fn a_top_zone_starts_below_the_menu_bar() {
        let visible = bounds(0.0, 25.0, 1200.0, 600.0);
        assert_eq!(cell(Zone::TopLeft, visible).origin.y.as_f32(), 25.0);
    }

    /// The case the module exists to get right: Settings is wider than
    /// a third of a laptop panel, so "top left" has to mean *flush into
    /// the top-left corner* rather than "centred on a cell it does not
    /// fit in".
    #[test]
    fn a_window_larger_than_its_cell_lands_in_the_corner() {
        let visible = bounds(0.0, 25.0, 1512.0, 957.0);
        let window = size(px(820.0), px(560.0));

        let placed = at(Zone::TopLeft, window, visible);
        assert_eq!(placed.origin, point(px(0.0), px(25.0)));

        let placed = at(Zone::BottomRight, window, visible);
        assert_eq!(
            placed.origin,
            point(px(1512.0 - 820.0), px(25.0 + 957.0 - 560.0))
        );
    }

    /// And a window that does fit is centred on its cell rather than
    /// jammed into a corner of it.
    #[test]
    fn a_window_that_fits_is_centred_on_its_cell() {
        let visible = bounds(0.0, 0.0, 1200.0, 600.0);
        let placed = at(Zone::BottomCentre, size(px(200.0), px(100.0)), visible);
        assert_eq!(placed, bounds(500.0, 400.0, 200.0, 100.0));
    }

    /// The precedence the whole page rests on. A zone is the user
    /// pointing at a part of a screen, so it beats the rectangle they
    /// left the window at — but it says nothing about size, and taking
    /// the size away would be one preference undoing another.
    #[test]
    fn a_chosen_zone_beats_a_remembered_position_and_keeps_its_size() {
        let visible = Bounds::new(point(px(0.0), px(25.0)), size(px(1200.0), px(600.0)));
        let remembered = Bounds::new(point(px(40.0), px(60.0)), size(px(500.0), px(400.0)));
        let default = size(px(820.0), px(590.0));

        let placed = frame_for(
            Some(Zone::BottomRight),
            Some(remembered),
            default,
            None,
            visible,
        );
        assert_eq!(
            placed.size, remembered.size,
            "a zone decides where a window goes and never how big it is"
        );
        assert_eq!(
            placed.origin,
            point(px(1200.0 - 500.0), px(25.0 + 600.0 - 400.0)),
            "a window wider than a third of the screen lands flush in the corner"
        );

        // And with nothing remembered, the zone still applies — at the
        // size a first window gets.
        let placed = frame_for(Some(Zone::TopLeft), None, default, None, visible);
        assert_eq!(placed.origin, point(px(0.0), px(25.0)));
        assert_eq!(placed.size, default);
    }

    /// Without a zone, nothing about the old behaviour moved: where the
    /// window was left is where it opens.
    #[test]
    fn no_zone_leaves_a_remembered_rectangle_exactly_as_it_was() {
        let visible = Bounds::new(point(px(0.0), px(25.0)), size(px(1200.0), px(600.0)));
        let remembered = Bounds::new(point(px(40.0), px(60.0)), size(px(500.0), px(400.0)));

        assert_eq!(
            frame_for(
                None,
                Some(remembered),
                size(px(820.0), px(590.0)),
                None,
                visible
            ),
            remembered
        );
    }

    /// Every zone is reachable from a pair of grid coordinates, and the
    /// two answers agree — the page lays the cells out by column and
    /// row and files the answer by name.
    #[test]
    fn the_grid_and_the_names_are_one_thing() {
        for zone in Zone::ALL {
            assert_eq!(Zone::at(zone.column(), zone.row()), Some(zone));
        }
        assert_eq!(Zone::at(Zone::COLUMNS, 0), None);
        assert_eq!(Zone::at(0, Zone::ROWS), None);
        assert_eq!(Zone::ALL.len(), Zone::COLUMNS * Zone::ROWS);
    }

    /// Stored values are a format. A zone written by this build has to
    /// be read by the next one, so this is the test that fails when
    /// somebody tidies up a name.
    #[test]
    fn the_stored_names_are_the_ones_that_were_written() {
        let written: Vec<&str> = Zone::ALL.into_iter().map(Zone::id).collect();
        assert_eq!(
            written,
            vec![
                "top-left",
                "top-centre",
                "top-right",
                "bottom-left",
                "bottom-centre",
                "bottom-right"
            ]
        );
        for zone in Zone::ALL {
            assert_eq!(Zone::parse(zone.id()), Some(zone));
        }
        assert_eq!(Zone::parse("middle-left"), None);
        for onto in Onto::ALL {
            assert_eq!(Onto::parse(onto.as_str()), Some(onto));
        }
        assert_eq!(Onto::parse("whichever"), None);
    }
    /// The rule the per-display rows exist for: two screens, two
    /// answers, and answering on one does not touch the other.
    #[test]
    fn each_screen_remembers_its_own_answer() {
        let store = Store::in_memory().expect("open");
        let laptop = screen("uuid-laptop");
        let external = screen("uuid-external");

        save(&store, &laptop, ticked(Zone::TopCentre)).expect("write");
        save(&store, &external, ticked(Zone::TopLeft)).expect("write");

        let spots = all(&store);
        assert_eq!(spot_for(&spots, &laptop).at, Answer::Zone(Zone::TopCentre));
        assert_eq!(spot_for(&spots, &external).at, Answer::Zone(Zone::TopLeft));

        save(&store, &laptop, by_hand(bounds(10.0, 20.0, 300.0, 200.0))).expect("write");
        let spots = all(&store);
        assert_eq!(spot_for(&spots, &laptop).at, Answer::Manual);
        assert_eq!(
            spot_for(&spots, &external).at,
            Answer::Zone(Zone::TopLeft),
            "answering for one screen moved another"
        );
    }

    /// A display nobody has answered for opens at the bottom right —
    /// the default is a *place*, so the grid on a first visit has a
    /// cell ticked and the window is already in it.
    #[test]
    fn a_display_nobody_has_answered_for_opens_at_the_bottom_right() {
        let spots = all(&Store::in_memory().expect("open"));
        assert!(spots.is_empty());

        let spot = spot_for(&spots, &screen("uuid-never-seen"));
        assert_eq!(spot, Spot::default());
        assert_eq!(spot.at, Answer::Zone(Zone::BottomRight));

        let visible = bounds(0.0, 25.0, 1200.0, 600.0);
        let placed = where_it_goes(spot, size(px(200.0), px(100.0)), visible);
        assert_eq!(
            placed,
            at(Zone::BottomRight, size(px(200.0), px(100.0)), visible)
        );
    }

    /// The rule the whole page rests on, and the one a person predicts
    /// from outside: the last word wins. A hand beats a cell, because
    /// dragging a window somewhere is a more specific instruction than
    /// pointing at a sixth of a screen — and clicking a cell afterwards
    /// beats the hand, for the same reason in the other direction.
    #[test]
    fn a_hand_beats_a_cell_and_a_cell_beats_a_hand_afterwards() {
        let visible = bounds(0.0, 25.0, 1200.0, 600.0);
        let put = bounds(140.0, 260.0, 420.0, 260.0);
        let wanted = size(px(420.0), px(260.0));

        let spot = by_hand(put);
        assert_eq!(
            where_it_goes(spot, wanted, visible),
            put,
            "a hand-placed window opens exactly where it was put"
        );

        let after = Spot {
            at: Answer::Zone(Zone::TopLeft),
            rect: spot.rect,
        };
        assert_eq!(
            where_it_goes(after, wanted, visible).origin,
            at(Zone::TopLeft, put.size, visible).origin,
            "a cell clicked afterwards takes the window back"
        );
    }

    /// A cell says *where* a window goes and never how big it is, so
    /// the size a hand gave it survives being sent to a corner — and
    /// comes back with it when the hand-placed rectangle is put back in
    /// force.
    #[test]
    fn a_cell_keeps_the_size_a_hand_gave_it() {
        let visible = bounds(0.0, 25.0, 1200.0, 600.0);
        let put = bounds(140.0, 260.0, 640.0, 480.0);

        let spot = Spot {
            at: Answer::Zone(Zone::BottomRight),
            rect: Some(put),
        };
        let placed = where_it_goes(spot, size(px(420.0), px(260.0)), visible);
        assert_eq!(placed.size, put.size, "the default size came back");
        assert_eq!(
            placed.origin,
            at(Zone::BottomRight, put.size, visible).origin
        );
    }

    /// "Where I put it" with nothing ever put anywhere is not an
    /// instruction to leave the window wherever the platform drops it —
    /// there is nothing to honour, so the default cell answers.
    #[test]
    fn a_hand_that_never_placed_anything_falls_back_to_the_default_cell() {
        let visible = bounds(0.0, 25.0, 1200.0, 600.0);
        let wanted = size(px(420.0), px(260.0));
        let spot = Spot {
            at: Answer::Manual,
            rect: None,
        };
        assert_eq!(
            where_it_goes(spot, wanted, visible),
            at(Zone::default(), wanted, visible)
        );
    }

    /// A rectangle remembered from a display that has since changed
    /// resolution names a position that no longer exists — the laptop
    /// on the train, at 1512 wide, holding a rectangle from a 2560-wide
    /// desk.
    #[test]
    fn a_hand_placed_rectangle_is_pulled_back_onto_a_smaller_screen() {
        let visible = bounds(0.0, 25.0, 1000.0, 575.0);
        let put = bounds(1800.0, 900.0, 400.0, 300.0);
        let placed = where_it_goes(by_hand(put), size(px(420.0), px(260.0)), visible);
        assert_eq!(placed, screen::contained(put, visible));
        assert_eq!(placed, bounds(600.0, 300.0, 400.0, 300.0));
    }

    /// A monitor that is not plugged in today is a monitor whose answer
    /// is waiting for it. Nothing prunes these rows: the display is
    /// coming back, and the alternative is a setting that a train
    /// journey silently throws away.
    #[test]
    fn an_answer_for_a_display_that_is_not_here_is_left_in_the_row() {
        let store = Store::in_memory().expect("open");
        let external = screen("uuid-external");
        save(&store, &external, ticked(Zone::TopRight)).expect("write");

        // The display is gone: no card lists it, and no window is
        // placed against it. The row is still here, and plugging the
        // monitor back in finds it.
        assert_eq!(
            all(&store).get("uuid-external").map(|spot| spot.at),
            Some(Answer::Zone(Zone::TopRight))
        );
    }

    /// A screen the platform will not name stably gets no row at all —
    /// writing one under the numeric id would restore it onto whichever
    /// display inherited that number after a reboot.
    #[test]
    fn a_screen_with_no_stable_name_is_never_written() {
        let store = Store::in_memory().expect("open");
        let anonymous = Screen {
            id: DisplayId::new(3),
            key: None,
            visible: bounds(0.0, 0.0, 800.0, 600.0),
        };

        save(&store, &anonymous, ticked(Zone::TopLeft)).expect("write");
        assert!(all(&store).is_empty());
        assert_eq!(
            spot_for(&all(&store), &anonymous),
            Spot::default(),
            "a screen with no name takes the default like any other"
        );
        assert!(
            store.settings().all().expect("read").is_empty(),
            "an unidentifiable screen must not leave a row behind"
        );
    }

    /// An answer is a preference the *grid* and the window itself
    /// change, not a row in `Setting::ALL` — the set of keys is the set
    /// of displays this machine has, and no static list can name them.
    /// It therefore has to stay out of the namespace
    /// `every_persisted_preference_has_a_row` walks, or that test
    /// becomes a lie the day a second monitor is plugged in.
    #[test]
    fn a_zone_is_never_filed_beside_a_preference() {
        let store = Store::in_memory().expect("open");
        save(&store, &screen("uuid-a"), ticked(Zone::TopCentre)).expect("write");

        for key in store.settings().all().expect("read").keys() {
            assert!(key.starts_with(PREFIX), "{key:?} is not a placement");
            assert!(
                !key.starts_with("ui."),
                "{key:?} would be read as a preference that needs a row"
            );
        }
    }

    /// A value this build cannot use is read as "no answer" and left
    /// exactly where it is — the rule every other preference keeps, and
    /// the one that makes a downgrade survivable.
    #[test]
    fn an_answer_this_build_does_not_know_is_left_alone() {
        let store = Store::in_memory().expect("open");
        store
            .settings()
            .set(
                "window.zone.uuid-a",
                &serde_json::json!({ "at": "middle-left" }),
            )
            .expect("write");

        assert!(all(&store).is_empty());
        assert_eq!(
            store
                .settings()
                .get::<serde_json::Value>("window.zone.uuid-a")
                .expect("read"),
            Some(serde_json::json!({ "at": "middle-left" })),
            "the row was corrected rather than left for a later build"
        );
    }

    /// Both words are a format: a row written by this build has to be
    /// read by the next one, so this is the test that fails when
    /// somebody tidies up a name.
    #[test]
    fn the_stored_answers_are_the_ones_that_were_written() {
        assert_eq!(Answer::Manual.as_str(), "manual");
        assert_eq!(Answer::parse("manual"), Some(Answer::Manual));
        for zone in Zone::ALL {
            assert_eq!(Answer::parse(zone.id()), Some(Answer::Zone(zone)));
        }
        assert_eq!(Answer::parse("somewhere"), None);
    }

    fn ticked(zone: Zone) -> Spot {
        Spot {
            at: Answer::Zone(zone),
            rect: None,
        }
    }

    fn by_hand(rect: Bounds<Pixels>) -> Spot {
        Spot {
            at: Answer::Manual,
            rect: Some(rect),
        }
    }
}
