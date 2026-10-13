# The panel's drops are rows of the one list — 2026-10-12

Branch `fix/panel-shared-list`, from `origin/feat/e0-e6-shell` at `ef0aa25`.
Decisions D690–D695 (`docs/plan/README.md` §4). The owner's finding at the
acceptance of 0.1.0: "I dropped a file on the panel. It said something, but
it did not appear in the list."

## Для владельца

Теперь всё, что бросили на панель, — файлы, картинки и текст — сразу
становится строкой в списке главного окна и в журнале, с пометкой «с панели»,
как будто это бросили на само главное окно. Это работает и когда главное
окно скрыто в трей или свёрнуто: строка появится, и вы увидите её, когда
откроете окно. Кнопка «Очистить» на панели очищает именно эту строку —
вторая не появляется, — а то, что панель пишет про результат, берётся из
самой строки: если очистить строку из главного окна, панель покажет то же
самое. Панель очищается сама: при следующем броске и после минуты тишины,
когда для её вещей ничего не выполняется (Настройки › Размещение, «Очищать
панель»: 15 секунд, минута, 5 минут или никогда). Строки в списке при этом
остаются. Строки командной строки и агента (MCP) появляются в открытом окне
без перезапуска — это проверено вживую.

## The road chosen

**The panel hands every drop to the main window's table** (`Queue::take_from_panel`),
and keeps the row ids. Not the other road (the panel records the journal row
and the table reads it back by its notes channel, D316), because:

* the table's row would have no `Arrival` for a dropped **text** — the journal
  never keeps a document (D312) — so the main window could not clean or
  rewrite it, and a text drag (`fix/linux-text-drop`) would land as a row
  nobody can act on;
* a clean asked from either window would be heard by that window only: the
  cleaner's events are by the asker's number, and the two numbers differ;
* two copies of one thing are two answers to "what is its status".

Handed over, the thing is one row in memory, one id in the line of cleans
(D283) and one journal row written by the table's writer. The table is a GPUI
global holding a strong handle (`queue::Table`, set by the shell next to the
`Queue::new`), so it outlives the main window: hidden by the tray or
minimised, the row lands and shows when the window is next opened; and with
no tray, a closed main window no longer takes the list with it while the
panel still takes drops.

What changed, by file:

* `queue.rs` — `Table`, `Standing` and `Queue::standing(id)`; `take` became
  `take_as(arrivals, origin)` returning the ids, with `take_from_panel`
  (origin `panel`; `--clean=` never matches a panel drop); the log line
  `queued … origin="panel"`.
* `queue/rewriting.rs` — a read of the journal never lists a row this table
  writes itself (`window`, `launch-flag`, and now `panel`) before its writer
  has named it; the log lines of D695.
* `panel.rs` — every `Landed` is handed over (overtaken or not); the last is
  held with its row ids; Clean is `Queue::clean` of those ids; what a clean
  did is read from the row; the quiet timer (`Quiet`, `settle_quiet`,
  `clear`). The panel writes no journal row and no longer subscribes to the
  cleaner. `show` and D660's `no_maximize` are untouched.
* `config.rs`, `settings.rs`, the three catalogues — `ui.panel.clear_after`,
  a Placement row *Clear the panel* (15 s / a minute / 5 minutes / never).
* `main.rs` — the shell sets `queue::Table`.
* `cleaner.rs` — `pending` is a test's only now.

`drop.rs` and `vendor/` are not touched.

## The owner's six points, each with its evidence

1. **One shared list; MCP and CLI rows appear in the open window.** Already
   true, and now shown: the CLI's `clean` writes its own row through its own
   connection and the open window lists it by the `PRAGMA data_version` poll
   (D316); an MCP `clean` writes in-process and the window lists it by its
   note. Live, steps d and e: `rows another surface wrote are listed
   listed=["cli"]`, then `listed=["agent"]`, each within a second, the window
   open throughout. With the application closed (step f) the CLI's row is
   written, and the next launch lists all six rows (`the journal's rows are
   listed at launch rows=6 origins=["agent", "cli", "panel"]`). `clean_image`
   and `rewrite` record through the same `Journal::record` → `Note::Changed`
   as `clean` (`mcp/protocol.rs`, `mcp/rewrite.rs`); they were not run live
   (no picture tool call scripted, no model on duty). Nothing to fix.
2. **Every drop on the panel is a row at once** — D690; tests
   `a_panel_drop_is_one_row_per_thing_in_the_list_and_the_journal`,
   `a_read_never_lists_a_row_of_this_tables_own`; live step a (one journal
   row, origin `panel`, state `waiting` — nothing cleaned it).
3. **The panel's Clean acts on that row, no second** — D691, D692; tests
   `the_panels_clean_is_that_row_and_the_main_windows_reads_on_the_panel`
   (a clean from the main window reads on the panel; the panel's Clean takes
   the other row; still two rows, both `done`, both `panel`) and
   `a_drop_on_the_panel_is_looked_at_and_cleaned` (the live check's case 13,
   now beside a table).
4. **The panel clears itself; the rows stay** — D693, D694; tests
   `the_quiet_period_clears_the_panel_and_not_the_rows` (not at 59 s; a clean
   restarts the minute; cleared at its end; rows and journal kept; Never never
   clears) and `a_new_drop_replaces_the_panels_view_and_adds_rows`,
   `the_panel_clears_after_a_minute_unless_told_otherwise` (config);
   `every_persisted_preference_has_a_row` and `every_row_is_reachable_from_the_sidebar`
   hold; the i18n crate's tests pass with the new en/ru/de keys. Live step b:
   cleared 15.0 s after the drop (the scratch row set 15), row kept; step c:
   two more drops, two more rows.
5. **Dragged text** — the panel hands over whatever its catcher lands;
   `Handed::Text` takes the same road, origin `panel` (`take_as` sets the
   origin for every kind). Test `a_text_dropped_on_the_panel_is_a_row_too`
   lands a `Handed::Text` through the panel's catcher — the call a text drag
   ends in — and gets one `panel` row, which its Clean cleans in place (one
   journal row, `done`). Nothing more is needed when `fix/linux-text-drop`
   merges.
6. **No maximize** — D660 untouched: `panel::show` still calls
   `x11.no_maximize(xid)` (`panel.rs`), and this branch changed nothing in
   `show`, `window_options` or `x11.rs`.

## Red checks

`docs/plan/reports/panel-shared-list-2026-10-12-red.py` deletes each
protection, runs the tests that guard it and puts the source back. Seven of
seven red:

```
RED   a panel drop is handed to the list (D690)
RED   the panel's Clean cleans that row, not a second (D691)
RED   what the panel shows is the row's (D692)
RED   the quiet period clears the panel (D693)
RED   clearing the panel takes no row (D693)
RED   a new drop replaces what the panel shows (D693)
RED   a read of the journal never lists the panel's row twice (D690)
```

## Live evidence

`scripts/verify/panel-list/panel-list.sh` (new, with the repository's
header): Xvfb on `:93`, a private session bus, lavapipe, a scratch
`WIPEMARK_DATA_DIR` (no walk-through, `ui.panel.clear_after` 15, MCP on
loopback port 5196 — never the owner's 5056), the application with
`--panel`, `xdnd-source.py` dropping onto the panel's window, the real
`wipemark-cli` and a real MCP call. Run on the owner's host, debug build:

```
panel 0x200008
step a: ok (one drop on the panel, one journal row from the panel, waiting)
step b: ok (the panel cleared 15.0 s after the drop; its row stayed)
step c: ok (two more drops, two more rows; the first stays)
step d: ok (the command line's own row, listed by the open window by data_version)
    mcp: journal 5
step e: ok (an agent's clean over MCP, listed by the open window by its note)
    INFO wipemark::queue::rewriting: the journal's rows are listed at launch rows=6 origins=["agent", "cli", "panel"]
step f: ok (the command line's row written with the application closed is listed when it starts)
--- the journal
    1 panel clean waiting
    2 panel clean waiting
    3 panel clean waiting
    4 cli clean done
    5 agent clean done
    6 cli clean done
--- our lines
    INFO wipemark::queue: queued count=1 kinds=[Text] origin="panel"
    INFO wipemark::panel: something was dropped on the panel kinds=[Text] listed=1
    INFO wipemark::panel: the panel cleared after a quiet period; the list keeps its rows
    INFO wipemark::queue: queued count=1 kinds=[Text] origin="panel"
    INFO wipemark::panel: something was dropped on the panel kinds=[Text] listed=1
    INFO wipemark::queue: queued count=1 kinds=[Text] origin="panel"
    INFO wipemark::panel: something was dropped on the panel kinds=[Text] listed=1
    INFO wipemark::queue::rewriting: rows another surface wrote are listed listed=["cli"]
    INFO wipemark::mcp::protocol: MCP: tools/call answered tool="clean" text=<elided chars=16 bytes=18> aggressive=false nfkc=false findings=1 kept=0
    INFO wipemark::queue::rewriting: rows another surface wrote are listed listed=["agent"]
panel-list: all six steps passed
```

(`kinds=[Text]`: a `.txt` file's kind; it arrived as a path.) The panel's
Clean was not pressed live — there is no click tool on this host and no
input is sent to anything but a drop target on the virtual display; the
three tests above hold it.

**A hazard found on the way, worth a line for `quick-drop.sh` too.** The
first run inherited `XDG_CURRENT_DESKTOP=ubuntu:GNOME`, so the application
took the GNOME shortcut road (D641) and ran `gsettings` against the private
bus, whose dconf service writes the user's real `~/.config/dconf/user`. GNOME
already held the chord, so nothing was set; the run only reset the three keys
of `…/custom-keybindings/wipemark-panel/`, which are in no list
(`custom-keybindings` read back `@as []`, the three keys `''` — the defaults).
The script now unsets the desktop variables and gives the bus its own
`XDG_CONFIG_HOME`, so the shortcut is an X11 grab on the virtual display.
`scripts/verify/linux-drop/quick-drop.sh` (another agent's) has the same
exposure.

## Gates

By the owner's word (2026-10-12) **the full gates are the coordinator's**,
once on `feat` after every branch is merged. Run here:

* `cargo test -p wipemark-app --locked -- panel:: queue:: config:: settings:: cleaner:: journal:: drop::`
  — 204 passed, 0 failed;
* `cargo test -p wipemark-i18n --locked` — 39 + 2 passed;
* `cargo clippy -p wipemark-app --all-targets --locked -- -D warnings` — clean;
* nightly rustfmt `--check` over every file of `apps/wipemark-app/src` — clean;
* the red checks above and the live check.
