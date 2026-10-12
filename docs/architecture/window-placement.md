# Where windows open

Epic **E6**. Which screen the panel opens on, where on that screen it
lands, and how both survive a monitor being plugged in, unplugged or
rearranged while the application is running.

Four modules and one page:

| | |
|---|---|
| `apps/wipemark-app/src/panel.rs` | the window this is all about: summoned, titleless on macOS and under Wayland, moved by hand from anywhere on it |
| `apps/wipemark-app/src/placement.rs` | the vocabulary, the decision and the arithmetic: `Onto`, `Zone`, `screen_for`, `frame_for`, `opening`, and the per-display rows |
| `apps/wipemark-app/src/screen.rs` | a display reduced to the three facts a placement needs, the displays as the page lists them, and the one call that moves a window |
| `apps/wipemark-app/src/display_watch.rs` | noticing that the displays have changed |
| `apps/wipemark-app/src/session.rs` | on Linux, which display server GPUI draws through: XWayland when a Wayland session offers it (D611) |
| `apps/wipemark-app/src/x11.rs` | under X11, the X server asked directly: the monitors, the pointer, a window's frame, a move, above-and-sticky (D612–D616) |
| Settings → Placement | one pair of radio buttons, and one grid per attached display |

## Which window this places

The **panel** — this application's third window, and the only one that
is *summoned*: from the menu bar, from `--panel`, and from a global
shortcut when one lands. It has no titlebar, it floats over other
applications, and it can be moved and resized by hand but never zoomed.
Each of those is a call `panel::afloat` makes that GPUI does not expose;
the module's own doc has the reasoning, and the short version is that a
window which arrives on its own has to be *over* what it arrived on top
of, and has to be moveable by somebody who disagrees with where it
arrived.

Under X11 — XWayland, the default on Linux (D611) — the panel wears the
window manager's title bar: GPUI gives a titleless window server-side
decorations there, and the bar is a handle a person already knows.
Pressing anywhere on the panel and dragging moves it on Linux too
(`Window::start_window_move` after four points of travel, so a click on a
button stays a click), which is what `panel-help-move` says; under
Wayland it is the only handle the panel has. It floats through
`_NET_WM_STATE_ABOVE` and `_NET_WM_STATE_STICKY` (below).

Neither of the other two windows is placed by these choices, and both
have a reason:

* The **main window** is where the work happens. It opens centred, it is
  dragged where the user wants it, and it stays open — nothing about it
  is "appears somewhere".
* The **Settings window** opens beside whatever asked for it — centred
  over the main window for the gear and `⌘,`, on the pointer's screen
  for the menu bar — and remembers its own rectangle per display
  (`window_state`). A preferences window that jumped into a corner while
  its own grid was being read would be answering a question about a
  different window. Asked for again while it is open and behind another
  window, under Wayland it is **closed and opened again** at the
  rectangle it had and on the section it was on (or the one the request
  names) — the activation GPUI asks for there is not honoured by mutter
  — unless that would lose something typed and not yet stored (a dialog,
  an API key, a folder not committed, a template not saved, a check
  running), when it stays and is activated (D594, D595,
  `an_open_settings_window_is_reopened_where_it_was`). The new window is
  opened before the old one is closed: GPUI ends a Linux process when its
  last window closes.

**A title bar of their own.** Under client-side decorations (GNOME on
Wayland) the main, Settings and Compare windows draw gpui-component's
`TitleBar` at the top of their content (`chrome::framed`, D590); the
panel never does. The bar is *inside* the content, so the frame and the
content are one rectangle there — `chrome` measured as `bounds` minus
`viewport` is zero — and what is saved is what is restored: nothing grows
on a reopen. The resize band and the shadow around it are
gpui-component's `Root` (`WindowBorder`, a 20 px client inset), as they
were before the bar (D593).

The workspace windows will be placed by these choices too when they
arrive; the banner at the top of the page says what is placed today and
stops saying it when that changes.

## Two questions with two shapes of answer

**Which screen** is one answer for the whole product. The primary
display, or the one the user is on — `Onto`, a row (`ui.window.screen`),
a pair of radio buttons, the same on every machine the database is
copied to.

`Onto::Active` is the default and means the screen the pointer is on
when the window opens — which for a window you summon is exactly the
right answer: the pointer is where you are looking, and the panel
arrives there. It is also the only thing "active" *can* mean for a
window that has nothing else to be active beside.

**Where on that screen** is one answer *per display*, because that is
what a person means by it: the laptop panel is small and a window
belongs in the middle of it, and the 32-inch display beside it has a
corner the user keeps clear. So the zone is filed under the display's
uuid, exactly the way `window_state` files a remembered rectangle, and
for the same reason — see the note at the top of `screen.rs` on why a
rectangle only means anything beside the display it was measured on.

## Six rectangles, three across, and one of them is the default

`Zone` is the screen's *visible* area — menu bar and Dock already taken
out — divided into three columns and two rows. The window is **centred
on the chosen cell** and then pulled back onto the screen by
`screen::contained`.

Centred rather than flush into a corner, and that is one rule doing two
jobs. A window bigger than its cell, centred on it and contained, ends
up exactly flush in the corner — which is what a person means by "top
left" — while a window smaller than its cell sits in the middle of it,
which is what they mean by "top centre". Neither case needs a special
case.

Every display starts at **bottom right** (`Zone::default`). A default
that is a *place* rather than an absence is what makes the six cells
mean something on the first visit: the page opens with one cell ticked,
the window is already in it, and dragging it to another is one gesture
that visibly does what it says. Bottom right in particular is furthest
from the menu bar and from the top-left corner every other application
opens into.

The grid is three across and two down on **every** screen, including a
portrait one, where the cells come out tall and narrow. Flipping to two
across and three down for a tall display was rejected: it changes what a
*stored* value means. `bottom-right` would name a different sixth of the
same display after the user rotated it, and a preference that moves on
its own is one nobody can trust.

## Precedence, when a window opens

`placement::frame_for`, in order:

1. **A hand.** `Answer::Manual` — the rectangle the panel was last
   dragged or resized to on that display. It wins over everything,
   because dragging a window somewhere is a more specific instruction
   than pointing at a sixth of a screen.
2. **A zone**, at the size a hand last gave the window, or the default
   size on a display where none ever has.
3. **The default**, which is a zone: bottom right, at the panel's own
   size, for a display nobody has answered for.

Last word wins, in both directions: click a cell after dragging and the
cell is in force; drag the window after clicking a cell and the hand is.
The rectangle survives either way, because it carries the size — a cell
says *where* a window goes and never how big it is.

"Restore default" is the one control that throws the rectangle away as
well as the cell — the size goes back too, which is the only thing that
phrase can honestly mean. It deletes the row rather than writing one, so
the display follows a later build's default rather than pinning this
one's.

`placement::opening` is the whole of it for a window that does not exist
yet, which the panel is every time it is summoned. GPUI takes a
*content* rectangle and reports a *frame* one, and the difference can
only be measured off a window that is already open — so the caller
states it. The panel passes zero, because a window with no titlebar has
a content view that fills its frame.

## Applying a choice while you watch

Choosing a zone on the card for **the display the panel is on** moves
the panel, now; "restore default" moves *and* resizes it. Every other
choice — another card, or a panel that is not on screen — is honoured
the next time it is summoned, which is the usual case: this is a window
you call up.

## Getting out of the way

**Close after a drop** (`ui.window.close_after_drop`, off unless it is
asked for) closes the Settings window when the panel is dropped onto a
cell. The panel floats above this window either way — that is what
`NSFloatingWindowLevel` is for — but "bottom right of this display" is
something a person checks against the *screen*, not against the
preferences window they were reading a moment ago.

Only a drop. The cells are also a row of buttons, because a preference
that can only be changed by dragging is a preference somebody cannot
change, and a keyboard user works along them comparing two corners — a
window that vanished on the first press would make the second
impossible. `SettingsView::make_way` is the whole of it, and it runs the
same two statements `CloseSettings` does: the geometry is written down
first, because the platform does not fire `on_window_should_close` for a
programmatic close.

It is one preference for the product rather than one per display — it is
about the gesture, not about a monitor — so it is a row above the cards
rather than a control on one, and it is in `config::PERSISTED` where the
zones deliberately are not.

## Ours and theirs

The other direction needs the same care in reverse. The panel reports
every bounds change the same way, ours and the user's, so a move this
application made must not come back as an answer the user gave:
`Preferences::we_placed_it` is told the rectangle before it is asked
for. Every report is debounced by 400 ms — a drag is sixty rectangles a
second and only the last one is an answer — and the last one is then
weighed by `placement::Placing` (D616): it is **ours** when it is the
rectangle asked for, when it came within `placement::GRACE` (1.5 s) of
the ask, or when it is again a rectangle an ask landed at; anything else
is a **person's**, and becomes `Answer::Manual`.

The grace is the point. A window manager carries an ask out in steps —
it maps the window where it maps new windows, reparents it into a frame,
then moves the frame where it was asked — and none of those rectangles is
the one asked for, because the frame's offset is the manager's. The rule
before this one matched the asked rectangle exactly, so under X11 the
first rectangle mutter opened the panel at was filed as the person's
answer, and from then on "manual" beat every cell (acceptance F8: a
scratch database held `{"at":"manual","rect":{"x":44,"y":906,…}}` — the
left edge of the work area, where mutter cascades). A person's drag in
the first second and a half after a summon is not heard; that is the
price, and it is small. `a_window_managers_offset_is_not_a_person` is the
gate, with a report 37 points off the ask (mutter's title bar).

That asymmetry is not a design preference, it is the platform:

* GPUI can resize a window (`Window::resize`) and cannot move one.
  `PlatformWindow` has no such method on any backend.
* So `screen::translate` reaches AppKit directly, and it takes a
  **translation** rather than a destination. GPUI's rectangle is
  display-local with y downwards; an `NSWindow` frame is desktop-global
  with y upwards from the bottom of the primary display. A delta needs
  one axis negated and nothing else; a destination would need the
  display's global origin, which is the one fact GPUI's macOS backend
  hides (`PlatformDisplay::bounds` returns an origin of zero for every
  display).
* A translation cannot cross displays, and nothing asks it to.

Under X11 the move is `screen::put`, an absolute one asked of the window
manager (below); under Wayland nothing can move a window, and the page
says so (D617).

Two things make this reachable at all from the Settings window: the
panel's handle, handed to `Preferences` when it is summoned and taken
back when it is dismissed, and the display it is on, which the panel
reports through its own `observe_window_bounds` rather than being asked
for per frame.

Nothing writes the geometry down afterwards: the platform tells GPUI the
window moved, GPUI tells the observers, and `Preferences` learns which
display it is on now.

## On Linux: XWayland, and what the window manager is asked

Acceptance F8 (2026-10-12, GNOME on Wayland): the panel opened wherever
GNOME cascades new windows — under Wayland **and** under XWayland. Three
causes, and this is what each became.

**Wayland cannot place a window.** The protocol leaves every toplevel's
position to the compositor, and GNOME has no layer-shell. So on Linux the
application draws through **XWayland by default** when a Wayland session
offers it (`session::choose`, D611): `WAYLAND_DISPLAY` and `DISPLAY` both
set, nobody asked otherwise, and the X server answers a connection. It is
chosen with GPUI's own switch — `gpui_platform::linux(X11 | HEADLESS)`,
the windowing modes the platform may start in — and **not** by removing
`WAYLAND_DISPLAY` from the environment: a `set_var` once threads exist is
undefined behaviour, and the programs the application opens inherit
GPUI's view of the session, which keeps `WAYLAND_DISPLAY` this way, so a
document opened in its default application still starts on Wayland.
`WIPEMARK_WAYLAND=1` keeps native Wayland; so does a session with no
`DISPLAY`, and an X server that does not answer. One INFO line in the log
says which and why.

Under native Wayland the Placement page says the desktop decides
(`settings-placement-system-decides`, in place of the line that says
these choices place the panel), dims each grid and disables its cells
and buttons (`placement::can_place`, D617). The cards are still drawn,
so a person sees what is stored.

**GPUI's X11 client places nothing either.** `gpui-pre-linux 0.3.8`
creates the window at the origin it is handed but writes no position flag
into `WM_NORMAL_HINTS` (`x11/window.rs`: a minimum and a maximum size, no
`PPosition`), so mutter ignores the origin; it has no call that moves a
window, keeps one above, or reads the pointer outside a window; and it
reports one display per X *screen* — the whole desktop, origin zero, the
nil uuid. `x11.rs` is the missing half, on an X connection of its own and
a thread of its own (`wipemark-x11`; D612):

* **The monitors** — RandR 1.5 `GetMonitors`, each with its output's name
  and the work area the desktop leaves on it (`_GTK_WORKAREAS_D<n>`, mutter's
  per-monitor list, else `_NET_WORKAREA` cut to the monitor, else the
  monitor). `screen::connected`, `primary`, `of_window` and `of_rect` answer
  from them under X11: one `Screen` per monitor, all carrying GPUI's one
  display id, each keyed by its own name. The reading is kept; asking for it
  costs no X traffic.
* **The pointer** — `QueryPointer` on the root, the one synchronous question
  the GPUI thread asks, waited for at most 100 ms (`screen::POINTER_WITHIN`)
  and then "unknown", which falls back to the primary display.
* **A window's frame** — where the client is on the root
  (`TranslateCoordinates`) and the decoration around it
  (`_NET_FRAME_EXTENTS`; mutter's title bar is 37 px), optionally once the
  manager has taken the window over (`WM_STATE`, viewable; at most 3 s).
* **A move** — `_NET_MOVERESIZE_WINDOW`, north-west gravity when the corner
  named is the frame's (the panel's zones), static when it is the client's
  (the Settings window's remembered rectangle); a plain `ConfigureWindow`
  where no window manager speaks EWMH.
* **Above and on every workspace** — `_NET_WM_STATE` add
  `_NET_WM_STATE_ABOVE` and `_NET_WM_STATE_STICKY`: the X11 half of
  `afloat`'s `NSFloatingWindowLevel` and `canJoinAllSpaces`. Not
  `SKIP_TASKBAR`: macOS lists the panel among the application's windows
  too.

When the panel opens under X11 (`panel::placed_by_the_window_manager`,
D613): the asked corner goes into `WM_NORMAL_HINTS` at once (a manager
that reads the hints after it lands opens the panel there and nothing
jumps — best effort, because GPUI maps the window inside `open_window`);
once the manager has the window, the frame is read; the zone's rectangle
is computed for the **frame** — the panel's size with the title bar
around it, so the bar stays inside the work area — asked for with
north-west gravity, and handed to `we_placed_it`; then above and sticky.
A rectangle the panel is put at by hand is the frame's too: the settle
reads the frame off the X server once the window is still, rather than
GPUI's report, which is the client's alone and, after a reparent, can be
relative to the frame. A zone chosen on the card of the display the panel
is on moves it at once, as on macOS (`screen::put`).

The Settings window's remembered rectangle is honoured the same way
(`screen::settle_opened`): GPUI opens it, the manager places it, and once
the manager has it the client is asked for at the remembered corner with
static gravity — what `geometry_of` saved is the client's, so what is
restored is the same rectangle and nothing creeps by a title bar.

**Every display shared one row.** With the nil uuid for every screen,
`window.zone.<nil>` was one answer for every monitor. Under X11 a
display's key is now the uuid-v5 of its output's name in the DNS
namespace (`x11::display_key`, D614) — exactly what GPUI's Wayland
backend makes its display uuid from, and XWayland names its outputs as
the compositor does (`HDMI-1` on the owner's host), so a display keeps one
key in both modes. It follows the connector, not the monitor: XWayland
gives no EDID. Two screens are one display by key where both have one
(`Screen::same_display`), never by id alone — every monitor of an X
screen has one id.

Rows already under the nil uuid are **adopted once** by the primary
display, on the first reading that names the displays
(`placement::adoption`, `window_state::adopt`, D615): a cell stays the
cell; a `manual` becomes the default cell with its rectangle kept —
every rectangle the window manager opened the panel at was filed as
manual before D616, so the old answer is the manager's as often as the
person's, and "Where I put it" on the card brings it back if it was a
person's. The Settings rectangle moves as it was. Both nil rows are then
deleted; a display that already had a row of its own keeps it.

**Bringing Settings forward (F2').** Under XWayland `cx.compositor_name()`
is `"X11"`, so `chrome::forward` answers `Activate` and D594's close and
reopen — keyed to `"Wayland"` — does not apply: the gear raises the open
window with GPUI's `_NET_ACTIVE_WINDOW`. mutter's code answers a request
with a timestamp of zero by taking the current time (`window_activate`),
so the window should come forward rather than ask for attention — read in
mutter's source, not seen live here (the owner's checklist has it).

**The system-wide shortcut** stays off in a Wayland session, XWayland
included (`hotkey::x11_session`, D346): a grab there hears only keys typed
into other X11 windows. The row says so in its own words now
(`hotkey-unavailable-wayland`, D618) instead of "not on this platform".

## Watching the displays

`display_watch.rs`, after heretic-lazy-shot's module of the same name.
Two sources converge on one cheap action — read the displays, hand the
reading to whoever holds the last one:

1. **`CGDisplayRegisterReconfigurationCallback`** (macOS). Fires several
   times per transition, from a thread of the window server's choosing.
   The pre-change `kCGDisplayBeginConfigurationFlag` phase is skipped —
   acting on it reads a desktop that is about to stop existing.
2. **RandR and the work area** (X11). `x11.rs`'s connection selects
   RandR's screen, CRTC and output notifications and the root's property
   changes (`_NET_WORKAREA`, `_GTK_WORKAREAS_D<n>`, the current
   workspace), re-reads the monitors when one arrives and signals when the
   reading changed — and once after its first reading, so the page does
   not wait a poll for the monitors (D612).
3. **A three-second poll**, regardless. Belt and braces, lazy-shot's
   interval, and the only source under Wayland.

The debounce sits on the *receiving* side rather than in the callback,
which is the one place this differs from lazy-shot: the callback only
sends, and the task that receives waits half a second and then drains
everything that arrived while it waited. A burst of six callbacks costs
one reading, and the half of this that runs on a thread we know nothing
about stays down to a channel send.

`Preferences` holds the last reading and `display_watch::moved` is the
comparison — deliberately the whole reading and not the number of
displays, because the change that breaks a placement hardest keeps the
count: a display that changed resolution has a different visible area,
and every zone on it now names a different rectangle.

What a display change deliberately does **not** do is move any window
that is already open. The user put it there. A window whose display has
gone is the platform's problem to solve while it is open, and
`screen::contained` catches a rectangle that no longer fits the next time
one opens.

## The rows

| key | what it holds |
|---|---|
| `ui.window.screen` | `active` or `primary` — one preference, one row, one widget |
| `ui.window.close_after_drop` | `true` or `false` — whether a drop closes the Settings window behind it |
| `window.zone.<display uuid>` | `{"at": "bottom-right"}`, or `{"at": "manual", "rect": {…}}` — the answer, and the rectangle a hand last gave |

The first and the last are formats: the words are what a future build
reads, so they are never localized and never spelled from a label.

Three states, and the absent row is one of them:

* **no row** — nobody has answered for this display, so it takes
  `Spot::default`: the bottom-right cell, at the panel's own size;
* **`at` is a cell** — somebody clicked one, or dragged the window onto
  it;
* **`at` is `manual`** — somebody moved or resized the window itself.

`rect` is kept beside all three once there has been one, because it
carries the size. `placement::spot_for` and `placement::where_it_goes`
are the two places they are read, so the page that draws a tick and the
code that places a window cannot disagree about what a display was
told.

The zone rows are deliberately **not** in `config::PERSISTED` and not in
`settings::Setting`, which is where every other preference has to be —
`every_persisted_preference_has_a_row` walks those two lists and asserts
they hold the same set. The set of zone keys is the set of displays this
machine has: not known when the binary is built, and different an hour
later. What stands in for that test is
`placement::a_zone_is_never_filed_beside_a_preference` — the rows stay
out of the `ui.` namespace the static test walks — and the page itself,
which draws one card per attached display and so cannot leave a display
with no way to answer.

A zone for a display that is not attached today is **left in the
database**. The monitor is coming back; the alternative is a train
journey silently throwing the setting away. A zone this build does not
recognise is left alone too, the same bargain `theme = "solarized"`
makes: read as no answer, warned about once, and still there for the
build that grows a seventh cell.

## What this does not do yet

* **Scrub anything.** The panel says what it will do and does not
  pretend to do it — the clean-up itself is not implemented. What is
  real today is the window:
  that it arrives where it was told, on the display it was told, and
  that Escape dismisses it.
* **Place the workspace windows.** They will use these choices when
  they arrive; today the panel is the only thing placed.
* **Move a window to another display.** See above — the macOS call is
  a translation. A window already open stays on its display; the choice
  applies the next time one opens there. Under X11 the move is absolute
  and could cross, and the rule is kept the same so the platforms answer
  one way.
* **Place anything under native Wayland.** The protocol does not let a
  client; the page says so (D617). The fix that would — an
  `xdg_activation` token taken from the focused surface for raising, and
  nothing at all for position short of layer-shell, which GNOME lacks —
  is GPUI's and the compositor's (`docs/plan/README.md` §7 E10).
* **Know a monitor by its EDID.** XWayland gives none; a display is its
  connector's name (D614).
* **Portrait grids.** Three by two everywhere, on purpose.

## Gates

| test | what it protects |
|---|---|
| `placement::the_default_is_what_the_application_already_did` | `Onto` cannot change which screen a window opens on for somebody who never visits the page |
| `placement::the_primary_choice_ignores_where_the_request_came_from` | the other choice is a promise, not a hint |
| `placement::a_preference_that_cannot_be_honoured_still_opens_the_window` | no display, no primary, no refusal to open |
| `placement::the_six_cells_tile_the_visible_area` | no gaps, no overlaps, outer edges are the screen's own |
| `placement::a_window_larger_than_its_cell_lands_in_the_corner` | the rule that made "centred" enough |
| `placement::a_chosen_zone_beats_a_remembered_position_and_keeps_its_size` | the precedence, and that a zone is not a size |
| `placement::a_hand_beats_a_cell_and_a_cell_beats_a_hand_afterwards` | last word wins, in both directions |
| `placement::a_cell_keeps_the_size_a_hand_gave_it` | a cell says where and never how big |
| `placement::a_hand_that_never_placed_anything_falls_back_to_the_default_cell` | "where I put it", with nothing ever put |
| `placement::a_hand_placed_rectangle_is_pulled_back_onto_a_smaller_screen` | the laptop on the train |
| `placement::a_display_nobody_has_answered_for_opens_at_the_bottom_right` | the default is a place |
| `placement::where_it_was_left_is_a_row_of_its_own` | the opt-out survives a build whose default moved |
| `placement::each_screen_remembers_its_own_zone` | choosing on one display never touches another |
| `placement::a_zone_for_a_display_that_is_not_here_is_left_in_the_row` | unplugging a monitor costs nothing |
| `config::a_window_closes_itself_only_after_being_asked_to` | the switch is off until it is turned on, and stays on across a restart |
| `placement::a_zone_is_never_filed_beside_a_preference` | the dynamic rows stay out of the static list's namespace |
| `settings::the_placement_banner_always_says_which_windows_it_places` | the page never implies it arranges a workspace that does not exist |
| `main::the_panel_can_be_asked_for_on_the_command_line` | the second way to summon it, and the one a check can use |
| `tray::every_command_round_trips_through_its_menu_id` | the menu-bar item that summons it |
| `display_watch::a_display_that_only_changed_resolution_is_a_change` | the comparison a display count would miss |
| `session::a_wayland_session_with_xwayland_draws_through_xwayland` and the four beside it | XWayland by default, the opt-out, no `DISPLAY`, an X11 session, an X server that does not answer (D611) |
| `x11::a_move_names_its_gravity_its_fields_and_an_application_as_its_source` | the zone's corner is the frame's, sent as `_NET_MOVERESIZE_WINDOW` wants it (D613) |
| `x11::the_work_area_is_the_monitors_own`, `x11::the_work_area_falls_back_to_the_desktops_then_the_monitor` | a zone is a sixth of the monitor's work area, not the desktop's |
| `x11::a_point_is_on_the_monitor_that_holds_it_or_the_nearest` | the pointer's monitor (D612) |
| `x11::a_display_is_keyed_by_its_output_name_as_wayland_keys_it` | the key is Wayland's uuid for the output, and never the nil one (D614) |
| `screen::two_monitors_of_one_x_screen_are_two_displays` | one X screen, one id, two displays |
| `placement::a_window_managers_offset_is_not_a_person`, `placement::after_the_grace_a_rectangle_nobody_asked_for_is_a_person` | F8's bug, and that a person is still heard (D616) |
| `placement::the_nil_row_is_adopted_by_the_primary_display`, `placement::the_nil_row_waits_while_a_display_still_answers_to_it`, `window_state::the_nil_rectangle_is_adopted_once_and_never_over_one_of_its_own` | the nil rows, once (D615) |
| `placement::only_wayland_leaves_the_placing_to_the_desktop`, `settings::under_wayland_the_placement_page_says_the_desktop_decides` | the page under native Wayland (D617) |
| `hotkey::in_a_wayland_session_the_row_names_the_session` | the shortcut row's reason (D618) |
| `scripts/verify/linux-placement/xvfb.sh` | live, on a bare Xvfb: the zone, no row from our own move, a person's move filed under the monitor's key, honoured next time, the nil row adopted |
