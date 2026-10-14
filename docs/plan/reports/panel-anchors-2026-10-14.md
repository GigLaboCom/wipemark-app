# The panel's nine anchors, flush, and a turned display keeping its anchor — report, 2026-10-14

**Коротко, для владельца.** Положений панели теперь девять, сеткой 3×3:
четыре угла, середина каждого края (сверху, снизу, слева, справа) и центр.
Панель встаёт **вплотную** к краю видимой области экрана — той, что
остаётся без строки меню и Dock, — без зазора: «снизу справа» значит, что
правый нижний угол панели лежит на правом нижнем углу этой области, какого
бы размера ни были оба. Раньше экран делился на шесть частей, и панель
вставала в середину своей шестой части. Шесть значений, сохранённых
раньше, означают то же, что и раньше; три новых — `middle-left`, `centre`,
`middle-right`; сборка до этой прочтёт новое значение как «по умолчанию» и
оставит строку в базе как есть. Выбор по-прежнему один на каждый экран, в
любой его ориентации: если экран повернуть на 90°, открытая панель сама
переходит в то же положение в новой форме экрана (снова в правый нижний
угол, если он выбран), а поставленная руками остаётся там, где была, и
только подтягивается внутрь экрана, без масштабирования. Так же панель
ведёт себя, когда у её экрана меняется разрешение или переезжает Dock. На
странице «Размещение» у каждого экрана карточка в его нынешней форме (у
вертикального — вертикальная), на ней девять клеток, а окно нарисовано
там, где оно откроется, — в масштабе и вплотную к краю. Проверить это
можно только глазами: девять положений на Mac, поворот экрана, если есть
такой монитор, и Linux под X11.

Branch `feat/panel-anchors`, from `feat/e0-e6-shell` at `f07c2ac` (the main
checkout's local branch, ahead of `origin`). Decisions **D723–D727**. Task:
Watchword FILE `wipemark-task-panel-anchors-2026-10-14` (the owner's word
during the macOS acceptance, A8).

## What was built

| file | what |
|---|---|
| `apps/wipemark-app/src/placement.rs` | `Zone` gains `MiddleLeft`, `Centre`, `MiddleRight`; `ROWS` = 3, `ALL` nine in reading order, `column`/`row`/`id`/`label` for nine; ids `middle-left`, `centre`, `middle-right`, the six old ones unchanged (D723). `at` puts the window flush: column 0 on `visible`'s left edge, 1 centred across, 2 on the right edge, the same down, then `screen::contained` (D724). **`cell` is removed**: the page lays its 3×3 hit areas out with flex rows (`Zone::at(column, row)`), and nothing else called it. `placed_again(spot, size, visible)`: an open window after its display changed shape — a zone at the same anchor at the size the window has, a hand's rectangle through `where_it_goes` (contained, not rescaled) (D725). Module doc rewritten: "Nine anchors, on the edges" and "A rotated display keeps its anchor" replace "Six rectangles, three across" and its portrait paragraph. |
| `apps/wipemark-app/src/display_watch.rs` | `reshaped(known, now, on)`: the display the panel is on, by `Screen::same_display`, still attached with a different visible area → its new `Screen`. `behind(known, measured)`: the last reading holds this display in another shape. Module doc: "The one window that follows its display". |
| `apps/wipemark-app/src/settings.rs` | `Preferences::screens_changed` (now `pub(crate)`) keeps the previous reading and calls `follow_the_panel`, which on `reshaped` defers `panel::follow` (D725). `reading_behind`. The Placement page: `zone_grid` draws nine cells and the window over them at `drawn_at` — the same `placement::at` on the card's inside (card less its 1-point border), at the panel's size or the size a hand last gave it, scaled, never smaller than `SMALLEST_CHIP` (was `MANUAL_CHIP`, now for both cases) and grown before it is anchored so it stays flush; a hand-placed window where it was put, as before. The chip is no longer a child of the lit cell's button (D726). Docs that said six. |
| `apps/wipemark-app/src/panel.rs` | `follow(preferences, now, cx)`: the open panel to `placed_again` in the reading's new shape, told to `we_placed_it` first, moved with `screen::put` (D725). `PanelView::settle`: a report settled while `reading_behind` is not an answer — the platform keeping the panel on a display that turned before the watcher read it. Module doc and `SIZE` doc. |
| `apps/wipemark-app/src/session.rs` | one doc line ("nine anchors"). |
| `crates/wipemark-i18n/i18n/{en-US,ru,de}/wipemark.ftl` | `settings-placement-zone-middle-left`, `-centre`, `-middle-right`: *middle left / centre / middle right*, *посередине слева / по центру / посередине справа*, *in der Mitte links / in der Mitte / in der Mitte rechts* — the register of the six (`top centre`, `сверху по центру`, `oben in der Mitte`), read into "A window opens at the { $zone } of this display." The en-US comment over them said "six parts"; no value said six or "centred". |
| `docs/architecture/window-placement.md` | "Nine anchors on the edges", D727 from the old parse code, "A turned display keeps its anchor"; the page drawing (D726); what "flush" is under X11; "The panel follows its display" and the guard, replacing "a display change does not move an open window"; the portrait line out of "What this does not do yet"; gates. |
| `docs/architecture/rules.md`, `map.md`, `icons.md` | the placement rule, `placement.rs`'s and `display_watch.rs`'s lines, the Placement glyph's note: nine anchors, flush, a turned display. |
| `scripts/verify/linux-placement/xvfb.sh` | the bottom-right zone on the bare 3840×1080 Xvfb is `3420,820,420,260` (flush), was `2990,680` (centred on a sixth); checks a and d. |
| `scripts/verify/linux-placement/placement-drag.sh` | three rows: `ROW=(502 534 565)`, the bottom row is `ROW[2]`, the press beside the chip at `335,554` (the chip now sits in the lit cell's bottom-right corner). **Computed** from the grid the script measured for two rows, not measured again — said in its header. |

### Every caller of `at` and `opening` (D724)

* **The panel's open**, `panel::show` (`panel.rs:276`) → `placement::opening` (`placement.rs:377`) → `where_it_goes` → `frame_for` → `at`. Flush with no change there.
* **A zone chosen while you watch**, `SettingsView::apply_here` → `put_back` (`settings.rs:8065`, `:8101`) → `where_it_goes(spot, size.unwrap_or(frame.size), here.visible)` (`:8134`) → `at`, on the panel's frame (`screen::frame_of`). Flush with no change there.
* **The display watcher** — new (below): `panel::follow` (`panel.rs:489`) → `placed_again` (`placement.rs:618`) → `at`.
* **The X11 road** — no change; read in the code, unbuilt here (see "X11").

## D725: what the road was, and what it needed

**It did not re-place before.** At `f07c2ac`, `Preferences::screens_changed`
(`settings.rs:2149–2160`) stored the reading (`self.screens = screens`, `:2157`)
and adopted the nil rows, nothing else, and `display_watch.rs:42` said so
("the window that is open stays where the user put it").
`a_rotated_display_puts_the_open_panel_at_the_same_anchor` was written first
and was red on that code (nothing asked).

Now: the watcher's closure (`settings.rs:1441`) → `screens_changed`
(`settings.rs:2149`) → `follow_the_panel(&known)` (`:2159`, `:2171`) →
`display_watch::reshaped` (`display_watch.rs:98`) → `cx.defer` →
`panel::follow` (`panel.rs:489`) → `placement::placed_again`
(`placement.rs:618`) → `we_placed_it`, then `screen::put` (`screen.rs:293`).
Deferred, so the panel's window is updated outside `Preferences`' update, the
way `put_back` does it.

**What "changes shape" means here.** `reshaped` fires on any change of the
panel's display's *visible area* — the same display by key, its visible
rectangle different — not only a width/height swap. A turned display is never
an exact swap of the visible area (the menu bar and the Dock come off
different edges: 1512×957 → 957×1512 is the test's idealisation), so a
swap-only test would miss real rotations; and a new resolution or a moved Dock
leaves a zone-anchored panel off its edge in the same way. A hand's rectangle
is only contained, so it moves only when it no longer fits.

**One guard went with it** (an addition inside D725, not in the task's text).
Between a display changing shape and the watcher's next reading (0.5 s after
the platform's callback, 3 s at most by the poll), the platform may move the
panel itself to keep it on the turned screen. That move reports like a
person's drag; `settle` weighs it 0.4 s later, and with no ask outstanding it
would be filed as `Answer::Manual` before the watcher ever re-placed the panel
— the anchor lost by the very turn it is meant to survive. So `settle`
(`panel.rs:1087`) skips a report while `Preferences::reading_behind`
(`settings.rs:2189`, `display_watch::behind`, `display_watch.rs:115`) says the
last reading holds the panel's display in another shape; the reading that
follows puts the panel at its anchor. Whether macOS actually moves a
borderless floating window on a rotation was not observed here (no window may
be opened); the guard costs nothing if it does not. A person's drag in that
half second is not heard — the same bargain `placement::GRACE` makes.

## X11 (D612–D616) — unbuilt on this Mac, unchanged

Nothing under `cfg(target_os = "linux")` changed; `x11.rs` is untouched. From
the code, "flush" under X11 is the **frame's** edge on the work area's edge,
the window manager's title bar and borders included:

* the summon: `panel::placed_by_the_window_manager` (`panel.rs:420`) computes
  `placement::where_it_goes(spot, chrome.grow(SIZE), screen.visible)`
  (`panel.rs:456`) — the frame's size from `_NET_FRAME_EXTENTS` — and asks
  `move_window(xid, Gravity::Frame, …)` (`panel.rs:461–469`), north-west
  gravity: the frame's corner goes where asked. The `WM_NORMAL_HINTS` hint
  before the window manager has the window (`panel.rs:437`) is the
  client-sized rectangle's corner, best effort as before; the move after it is
  exact.
* a move of an open panel (a zone chosen while you watch, and now a turned
  display): `screen::put` (`screen.rs:293`) sends `Gravity::Frame`
  (`screen.rs:300`) with the frame from `screen::frame_of` (`screen.rs:282`,
  `chrome_of` at `:264`).

The work area is per monitor (`_GTK_WORKAREAS_D<n>`, D612), and a monitor
turned through RandR is the same road: `x11.rs` re-reads on RandR's
notifications and signals the watcher. The two Linux verify scripts were
brought to the new geometry and not run (no Linux here).

## D727, from the parse code

It holds. At `f07c2ac`, `Zone::parse` (`placement.rs:245–246`) is
`Self::ALL.into_iter().find(|zone| zone.id() == value)` over six zones, so
`middle-left`, `centre` and `middle-right` are `None`; `Answer::parse`
(`:439–443`) passes that on; `placement::all` logs "unknown placement; this
screen takes the default" and drops the row from the map (`:512–515`) — not
from the database; `spot_for` then answers `unwrap_or_default()` (`:539`),
bottom right. That build's own test said exactly this with `middle-left`
(`an_answer_this_build_does_not_know_is_left_alone`, `:1263–1281`). Nothing
is migrated; the test now uses `left-of-centre`.

## Red first

Each protection deleted (or mutated) once, the targeted tests run, the
protection put back. Rows 1, 6, 9 and 10 are the tests written before the
code; rows 9/11 were also deleted once with the code in place.

| # | protection deleted | red |
|---|---|---|
| 1 | `at` as it was (centred on a sixth: `contained(centred(wanted, cell(zone, visible)), visible)`), tests written first | `each_anchor_puts_the_window_flush_against_its_edges`, `a_rotated_display_keeps_every_anchor_on_its_edges` |
| 2 | `screen::contained` out of `at` | `each_anchor_puts_the_window_flush_against_its_edges` (the window wider than the screen) |
| 3 | *(mutation)* an old id misspelt, `top-centre` → `top-center` | `the_six_old_ids_keep_their_meaning_and_an_unknown_one_reads_as_the_default`, `the_stored_names_are_the_ones_that_were_written` |
| 4 | *(mutation)* `Zone::parse` reads an unknown word as the default zone | `the_six_old_ids_…`, `an_answer_this_build_does_not_know_is_left_alone`, `the_stored_names_…`, `the_stored_answers_are_the_ones_that_were_written` |
| 5 | *(mutation)* the bottom row's index 2 → 1 | `the_six_old_ids_…`, `the_grid_and_the_names_are_one_thing`, `each_anchor_…`, `a_rotated_display_keeps_every_anchor_…`, `a_chosen_zone_beats_a_remembered_position_and_keeps_its_size` |
| 6 | the page's window as before D726 — a 26×18 chip in the middle of the lit cell — test written first | `the_grid_is_nine_cells_and_the_window_is_drawn_at_its_anchor` ("TopLeft: … 26px × 18px is not the panel to scale"); its reading-order half already held |
| 7 | *(mutation)* `drawn_at` given the card's outer size, not its inside | the same ("TopRight … is not flush") |
| 8 | *(mutation)* a hand-placed window drawn at the card's corner | the same ("… is not where the hand put it") |
| 9 | no re-placement (before D725), test written first; then the `follow_the_panel` call out of `screens_changed` | `a_rotated_display_puts_the_open_panel_at_the_same_anchor` |
| 10 | no guard in `settle`, test written first | `a_move_while_the_display_turns_is_not_a_persons` |
| 11 | the guard and `panel::follow`'s `we_placed_it`, both out at once | `a_move_while_the_display_turns_is_not_a_persons` (the guard: its test never reaches `follow`, no panel handle is held) and `a_rotated_display_puts_the_open_panel_at_the_same_anchor` (the ask: its test never settles a report) |
| 12 | `reshaped` → `None` and `behind` → `false`, at once | `a_turned_display_under_the_panel_is_reshaped` and `a_rotated_display_puts_the_open_panel_at_the_same_anchor` (`reshaped`); `a_reading_is_behind_a_display_measured_in_another_shape` and `a_move_while_the_display_turns_is_not_a_persons` (`behind`) |
| 13 | `placed_again`'s zone at the remembered rectangle's size (`where_it_goes`) | `a_turned_display_keeps_the_anchor_and_contains_a_hand` |
| 14 | *(mutation)* a hand's rectangle rescaled to the new width | `a_rotated_display_puts_the_open_panel_at_the_same_anchor`; `a_turned_display_keeps_the_anchor_and_contains_a_hand` stayed **green** (rescaled and contained both land at x = 537 for its one hand case), so a hand that still fits after the turn was added to it, and it is red with that case |

The task's tests, by number: 1 — `placement::each_anchor_puts_the_window_flush_against_its_edges`;
2 — `placement::a_rotated_display_keeps_every_anchor_on_its_edges`; 3 — the road through
the seam the watcher's closure calls, `Preferences::screens_changed`:
`settings::a_rotated_display_puts_the_open_panel_at_the_same_anchor` (a plain window
stands for the panel; what was asked is read through `Preferences::heard` after the
grace — the test platform has no native window, so `screen::put` moves nothing),
with the pure halves `display_watch::a_turned_display_under_the_panel_is_reshaped`
and `placement::a_turned_display_keeps_the_anchor_and_contains_a_hand`; 4 —
`placement::the_six_old_ids_keep_their_meaning_and_an_unknown_one_reads_as_the_default`
and `the_stored_names_are_the_ones_that_were_written` (now nine); 5 —
`settings::the_grid_is_nine_cells_and_the_window_is_drawn_at_its_anchor`, and
`the_window_is_picked_up_anywhere_in_its_cell`, `an_empty_cell_is_not_a_handle`,
`close_after_a_drop_closes_on_a_drop_and_not_on_a_click` hold unchanged — none of them
counted six (one doc line now says "the chip drawn in it"). The guard has
`panel::a_move_while_the_display_turns_is_not_a_persons` and
`display_watch::a_reading_is_behind_a_display_measured_in_another_shape`.

Removed with `cell` and the old rule: `the_six_cells_tile_the_visible_area`,
`a_top_zone_starts_below_the_menu_bar` (test 1 holds the top row at y = 25),
`a_window_larger_than_its_cell_lands_in_the_corner` (test 1's corners) and
`a_window_that_fits_is_centred_on_its_cell` (it asserted the old rule).

## Targeted tests

On the owner's Mac (Apple Silicon), the pinned toolchain (1.95.0), no
`LIBRARY_PATH`. The application was not launched.

| command | result |
|---|---|
| `cargo test -p wipemark-app --locked -- placement:: settings:: panel:: display_watch:: screen::` | **123 passed**, 0 failed — placement 27, settings 66, panel 16, display_watch 5, screen 9 (at `f07c2ac`: 27, 64, 15, 3, 9) |
| `cargo test -p wipemark-i18n --locked` | 41 passed, and 2 |
| `cargo clippy -p wipemark-app --all-targets --locked -- -D warnings` | clean |
| `rustup run nightly rustfmt --edition 2021 --check` on `placement.rs`, `settings.rs`, `panel.rs`, `display_watch.rs`, `session.rs` | clean |
| `bash -n` on `xvfb.sh`, `placement-drag.sh` | clean (not run: Linux) |

Every commit on the branch builds; the first three each ran the tests above
for their modules and clippy before they were made.

## Wanted edits

### `docs/plan/README.md`, the decision table

| | | |
|---|---|---|
| **D723** | *(the owner, 2026-10-14, macOS acceptance A8: "make the positions attached to the edges — top left, top right, top centre, and so on")* **The panel's zones are nine anchors, 3×3**: the four corners, the middle of each edge (top, bottom, left, right) and the centre. `Zone` gains `MiddleLeft`, `Centre`, `MiddleRight`, `ROWS` = 3, `ALL` nine in reading order; the stored ids are `middle-left`, `centre`, `middle-right`, and the six old ids parse to the zones they named. `Zone::default()` stays bottom right. `settings-placement-zone-middle-left`, `-centre`, `-middle-right` in en-US, ru, de. | `placement::the_six_old_ids_keep_their_meaning_and_an_unknown_one_reads_as_the_default`, `the_stored_names_are_the_ones_that_were_written` (red with an old id misspelt, with an unknown word read as the default, with the bottom row moved). |
| **D724** | *(the owner: "flush")* **A window is put flush against the edges its anchor names, on the visible area** (`screen.visible`, without the menu bar and the Dock), with no gap: column 0 on the left edge, 1 centred across, 2 on the right edge, the same down; then `screen::contained`. `placement::cell` is removed — the page lays its cells out by flex and nothing else used it. Under X11 it is the frame that is flush, title bar included (`where_it_goes(…, chrome.grow(SIZE), …)`, `Gravity::Frame`); unchanged code, read, not run. | `placement::each_anchor_puts_the_window_flush_against_its_edges` (red with the old centred-in-a-sixth `at`, and with `contained` out). |
| **D725** | *(the owner: "rotating the screen 90° — provide for it")* **One answer per display, whatever its orientation; a display that changes shape keeps the open panel at its anchor.** An anchor is an edge, so a stored value means the same place in portrait. When the watcher's reading says the visible area of the display the open panel is on changed (`display_watch::reshaped` — a turn, a new resolution, a moved Dock), `Preferences::follow_the_panel` defers `panel::follow`: the same anchor in the new shape at the size the panel has, or a hand's rectangle contained and not rescaled (`placement::placed_again`), told to `we_placed_it` first. A report the panel settles while the last reading is behind its display (`display_watch::behind`) is not a person's answer. Before, a display change moved no open window. | `settings::a_rotated_display_puts_the_open_panel_at_the_same_anchor` (red with no re-placement, with the call out, with `reshaped` empty, with `we_placed_it` out, with a hand rescaled), `placement::a_rotated_display_keeps_every_anchor_on_its_edges`, `placement::a_turned_display_keeps_the_anchor_and_contains_a_hand`, `display_watch::a_turned_display_under_the_panel_is_reshaped`, `panel::a_move_while_the_display_turns_is_not_a_persons`, `display_watch::a_reading_is_behind_a_display_measured_in_another_shape`. Not seen on a turned monitor. |
| **D726** | *(the owner)* **The Placement page draws nine cells, and the window where it will open**: over the cells, at its anchor, to scale and flush against the card's inside (`settings::drawn_at`, `placement::at` on the card), at the panel's size or a hand's; a hand-placed window where it was put; never smaller than 24×16 points (`SMALLEST_CHIP`, was `MANUAL_CHIP`), grown before it is anchored. Each card keeps its display's current shape. | `settings::the_grid_is_nine_cells_and_the_window_is_drawn_at_its_anchor` (red with the chip back in the middle of its cell, with the card's outer size, with a hand's window at the corner); the three D648 page tests hold. |
| **D727** | **The stored rows are compatible both ways; nothing is migrated.** The six old ids mean what they meant. A build before D723 reads `middle-left`, `centre` or `middle-right` as no answer — its `Zone::parse` finds none among six, `placement::all` drops the row from the map and leaves it in the database, `spot_for` answers the default — which is what its own `an_answer_this_build_does_not_know_is_left_alone` asserted with `middle-left`. | Read in the code at `f07c2ac` (`placement.rs:245`, `:439`, `:512`, `:539`, `:1263`). |

### `CLAUDE.md`, "Where it stands"

The next free decision block is **D728** (D723–D727 are the panel's nine
anchors). Perhaps one sentence beside the other built items: *The panel's
positions are nine anchors — the corners, the middle of each edge, the
centre — put flush against the screen's visible edges, and an open panel
keeps its anchor when its display turns (D723–D727).*

## Not done here

* **The owner's live check on the Mac**: the nine positions, each flush
  against the visible area's edges (the menu bar above, the Dock below or
  beside) — from the card's cells and by dragging the window on the card; the
  card's drawing against where the panel actually lands; a hand-placed panel
  still drawn where it was put.
* **A turned display**, if there is one to turn (System Settings → Displays →
  Rotation): the open panel at the same anchor in the new shape within a
  second, the card portrait; a hand-placed panel pulled in, not rescaled. Also
  a change of resolution, which takes the same road. Whether macOS moves the
  panel itself on a turn was not observed; the guard makes either case end at
  the anchor.
* **Linux under X11**: unbuilt here (no `cfg(target_os = "linux")` code
  changed); the frame flush under mutter (title bar included) and a RandR
  rotation; `scripts/verify/linux-placement/xvfb.sh` (now expects
  `3420,820`) and `placement-drag.sh` (points computed for three rows, not
  measured) not run. CI builds the Linux side.
* The full gates (the coordinator's, on `feat`), and the edits above to
  `CLAUDE.md` and `docs/plan/README.md`.
