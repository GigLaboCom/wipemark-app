# The tray on Linux: report

- **Asked by:** the owner, 2026-10-07 — «запилить трей в линуксе чтобы все
  уметь закрывать»; the E10 bullet "The tray and the system-wide shortcut on
  Linux" in `docs/plan/README.md` §7.
- **Branch:** `e10/linux-tray`, from `origin/feat/e0-e6-shell` at `0a4c6be`,
  `feat` merged back before the final gates. Pushed to `origin`, no PR.
- **Decisions:** D340–D347, all in
  [`docs/architecture/tray.md`](../../architecture/tray.md) (D346 in
  [`hotkeys.md`](../../architecture/hotkeys.md)); D348–D349 unused.
- **Files:** `apps/wipemark-app/src/tray.rs`, the tray and hotkey wiring in
  `apps/wipemark-app/src/main.rs` (`install_tray`, `adopt_tray`,
  `take_beacon_at_quit`, one argument to `hotkey::install`),
  `apps/wipemark-app/src/hotkey.rs` (Linux registrar, `x11_session`),
  `apps/wipemark-app/Cargo.toml`, `Cargo.lock`, `docs/`,
  `scripts/verify/linux-tray/`. No catalogue string was added — the menu's
  words are the macOS menu's.

## What was built

1. **A tray on Linux with the macOS menu** — Show Wipemark, Show the panel,
   Clean Clipboard (disabled, as on macOS), Unload model (enabled while a
   model is loaded), Appearance ▸ System/Light/Dark, Settings…, Quit — from
   one builder, `tray::build_menu`, shared with macOS. Clicks reach GPUI
   through the same `flume` channel as `TrayCommand`s.
2. **A GTK thread of its own (D340).** `tray::install` starts
   `wipemark-tray`: `gtk::disable_setlocale` + `gtk::init`, the menu and the
   item built there, `gtk::main` until Quit. The widgets never leave it; the
   theme tick, the labels after a language change and Unload's state travel
   to it as `tray::Change`s and are applied by a local future on GTK's main
   context. `install` answers asynchronously on every platform
   (`tray::Pending`); `tray::when_installed` runs `adopt_tray` only on
   `Some`.
3. **No item where nobody would see one (D341).** The thread probes the
   indicator library first (`libappindicator-sys` panics when it is
   missing), then GTK, then the session bus for a StatusNotifier watcher
   that says a host is registered; any refusal is `None`, logged as
   `tray skipped; the close button closes` with the reason.
4. **The icon (D342):** the template's broom drawn white over a dark
   three-pixel outline (at 64 px), derived at run time — it reads on a dark
   GNOME top bar and on a light KDE panel alike, without guessing the
   panel's colour. One indicator id per process (D347).
5. **The close button (D343).** With a tray: macOS hides the application, as
   before; Linux under X11 **minimizes the main window**, and Show restores
   it (`_NET_ACTIVE_WINDOW`); Linux under Wayland keeps closing, because a
   compositor there refuses the activation Show would need. GPUI's
   `App::hide` is a no-op on Linux, so the macOS hook could not simply be
   reused. Registered inside the `Some`, as before.
6. **Quit (D344)** is `App::quit` on both platforms, so the engine host drops
   the model and waits for its worker, Settings saves its rectangle, the
   tray's thread takes the item down (indicator passive, icon file removed,
   GTK loop ended) inside GPUI's 200 ms quit budget, and — new, on both
   platforms — the MCP beacon is removed if it names this process. Before
   this, the supervisor thread never got to run before the process exited,
   so every Quit left a beacon with a dead pid. The listener's socket goes
   with the process.
7. **The theme tick (D345)** is put back after every click: a check item
   toggles itself, and choosing the theme already chosen moved nothing back.
8. **The shortcut (D346)** — it fell out cheaply: `global-hotkey` on Linux
   is an X11 key grab on a thread of its own. It is asked for only where
   GPUI draws through X11 and `XDG_SESSION_TYPE` is not `wayland`
   (XWayland grabs hear only X11 clients); elsewhere the row says
   *Stored, and not active*. **On GNOME the shipped panel chord Ctrl+Alt+D
   is one of the bindings of *Show desktop*** (this host's
   `org.gnome.desktop.wm.keybindings show-desktop` holds
   `<Primary><Alt>d`), so expect it to be refused under the field and
   record another, e.g. Ctrl+Alt+W.
9. **Windows** is unchanged: `install` answers `None`, `hotkey::install`
   returns `None`.

### Closing the main window while Settings is open

- macOS (unchanged): the application hides, Settings with it; Show or the
  Dock brings both back.
- Linux, X11, with the tray: only the main window is minimized. Settings
  stays open with its own close button; closing it closes it, and the
  process goes on (the minimized main window is still a window). Show
  restores the main window; the tray's Settings… opens Settings again;
  Quit ends everything.
- Linux without a tray, or under Wayland: the close button closes the main
  window; GPUI ends the process when the last window (Settings, the panel)
  closes — as before, but now the tray's Quit ends it at once under
  Wayland too.

## Tests and the protections seen red

New unit tests, in `tray::tests` and `hotkey::tests`:

| test | protects |
|---|---|
| `a_missing_library_is_no_tray_and_nothing_after_it_runs` | D341: the library is probed first, and a missing one is `None` |
| `every_refusal_is_no_tray`, `the_steps_run_in_order` | D341: each step refuses on its own, in order |
| `an_item_nobody_would_draw_is_refused` | D341: no watcher, or no host, or a watcher that cannot say |
| `an_installed_tray_carries_changes_and_leaves_on_quit` | D340/D344: changes reach the thread in order; Leave drops the item before it answers |
| `dropping_the_tray_ends_its_thread`, `a_change_after_leave_is_dropped` | the thread's lifetime |
| `the_linux_icon_reads_on_a_light_and_a_dark_panel`, `the_linux_icon_keeps_the_glyph_white_inside_its_outline` | D342 |
| `the_close_button_hides_only_where_show_brings_the_window_back` | D343 |
| `without_a_tray_the_close_button_closes`, `with_a_tray_the_close_button_keeps_the_window`, `closes_registers_nothing` | the CLAUDE.md rule: the hook only inside the `Some`, however late it comes (GPUI tests) |
| `a_shortcut_is_asked_for_only_where_an_x11_grab_hears_the_keyboard` | D346 |
| `every_stored_spelling_is_one_the_registrar_parses` | now also on Linux, with Linux's modifiers |

Deleted once each, locally, and seen red (no mutation tables): the library
step removed from the ladder (3 tests red); `(true, None)` accepted as a
host (red); `panel_image` returning the template unchanged (2 red); the
`Closes` early return removed (red); Wayland answered `MinimizesTheWindow`
(red); `x11_session` without the Wayland check (red); the Leave answered
without dropping the item (red 3 of 3 after the fake's run was made to
linger 300 ms before it drops a surviving item — 2 of 3 before).

**The real GTK thread, headless:** `scripts/verify/linux-tray/headless.sh`
runs the ignored test `tray::tests::linux::the_real_desktop_answers_as_the_bus_says`
under `xvfb-run -a` and a private `dbus-run-session`, three times — no
watcher, a watcher with no host, and `fake-watcher.py` as the host. On this
host:

```
round a: ok
round b: ok
round c: ok (item registered: /org/ayatana/NotificationItem/tray_icon_tray_app_wipemark_<pid> :1.2)
headless: all three rounds passed
```

— libayatana loaded, GTK up on `wipemark-tray`, the item registered with
the watcher, its icon written under the scratch `XDG_RUNTIME_DIR`, the menu
changed over the channel, and the file gone after the Leave. Nothing was
drawn on the owner's screen or registered on the owner's bus.
`scripts/verify/linux-tray/host.sh` says, read-only, what this session would
get: on this host `tray: yes`, close button minimizes, shortcut asked of
X11.

## Gates

On this host (Ubuntu, x86_64), at `bf13443` — `feat` merged in — every one
with `--locked`:

| gate | result |
|---|---|
| `rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass |
| `cargo test --workspace --locked` | 1489 passed, 0 failed, 7 ignored |
| `scripts/check-dep-direction.sh` | pass |
| `scripts/check-gpui-pin.sh` | pass (gpui-pre =0.3.8, 23 crates once each; X11 guards A, B, C) |
| `cargo check --workspace --no-default-features --locked` | pass |
| `cargo check --workspace --features local-llama --locked` | pass |
| `cargo test -p wipemark-engine --features local-llama --locked` | 36 passed, 0 failed, 1 ignored |
| `cargo test -p wipemark-app --features local-llama --locked` | 552 passed, 0 failed, 2 ignored |
| `scripts/verify/linux-tray/headless.sh` | three rounds passed (above) |

`Cargo.lock` moved by `tray-icon`'s default features going (`libxdo` and
`libxdo-sys` left the lock) and the app's two new Linux dependencies, both
already in the lock at those versions (`gtk` 0.18.2, `libloading` 0.7.4).
Windows is not built here; its code path is `install` answering `None` and
`hotkey::install` returning `None`, unchanged in shape, and the
`llama-source` workflow (whose Windows job builds only the llama crates)
is not triggered by these paths.

## CI

CI_PLACEHOLDER

## For the owner: the checklist in the running window

Not launched here (the rule: no window on the owner's desktop). Build and
run once, from the branch:

```sh
cargo run -p wipemark-app
```

1. **The item appears** in the top bar (GNOME with the Ubuntu AppIndicators
   extension, which this session has): a white broom with a dark edge. The
   log (`<data dir>/logs/`) has `tray installed`.
2. **The menu opens** on a click, with the macOS items: Show Wipemark, Show
   the panel, Clean Clipboard — not yet (greyed), Unload model (greyed unless
   a model is loaded), Appearance ▸, Settings…, Quit Wipemark.
3. **Close the main window** with its close button: it is minimized, not
   closed; the process is still running (`pgrep -a wipemark`).
4. **Show Wipemark** brings it back, raised.
5. **Show the panel** opens the panel; the same item again sends it away.
6. **Appearance ▸ Dark / Light / System** switches the theme, and the tick
   follows; clicking the ticked item keeps it ticked. Changing the theme in
   Settings moves the tick too.
7. **Settings…** opens Settings. With Settings open, close the main window:
   only the main window goes; Settings stays. Close Settings: the process
   still runs; Show brings the main window back.
8. **Language** (Settings › General) to Русский: the menu's words follow.
9. **Unload model**: with a model loaded by a Check, it is enabled, and
   unloads it.
10. **Quit Wipemark** ends the process — every window, the item leaves the
    top bar, `pgrep -a wipemark` prints nothing, port 5056 is free
    (`ss -ltn | grep 5056`), and `<data dir>/mcp.json` is gone.
11. **The shortcut**: Settings › General shows the panel's chord. Under
    GNOME expect Ctrl+Alt+D to be refused (it is *Show desktop* here) —
    record another and press it from another application: the panel
    appears.
12. Optional: in a GNOME session with the AppIndicator extension switched
    off, launch again — no item, the log says `tray skipped … no
    StatusNotifier watcher`, and the close button closes.

Kill it when done (port 5056).

## Wanted edits outside this branch's files

For the coordinator — this branch does not touch `CLAUDE.md` or
`docs/plan/README.md`:

- `CLAUDE.md`, "The close button only hides while there is a way back":
  the tray is installed on macOS and on Linux; on Linux the item lives on a
  GTK thread of its own (D340), is installed only where a StatusNotifier
  host would draw it (D341), and the hook minimizes the main window under
  X11 and is not registered under Wayland (D343); `tray::install` answers
  asynchronously and `tray::when_installed` is what keeps the hook inside
  the `Some`.
- `CLAUDE.md`, the `tray.rs` and `hotkey.rs` rows and the hotkey rule
  ("macOS only like the tray"): Linux under X11 too (D346); and the GNOME
  collision of Ctrl+Alt+D.
- `CLAUDE.md`, near the gates: the Linux binary links GTK 3
  (`libgtk-3.so.0`), and loads libayatana-appindicator at run time; the
  `.deb` (E10) must depend on `libgtk-3-0` and `libayatana-appindicator3-1`.
  `scripts/verify/linux-tray/headless.sh` is the closest thing to a live
  check of the indicator.
- `docs/plan/README.md` §7, the E10 bullet: status done (this report),
  D340–D347 in §4; what stays open: Windows, a host that leaves after
  launch, left-click activation, Wayland's close button and shortcut.
- `apps/wipemark-app/assets/tray/README.md` is generated by
  `icons/create-icons.sh` and still says E10 will pick the black or the
  white file by theme; D342 supersedes that — the generator's text should
  say the Linux icon is derived at run time.
- The MCP supervisor (E4-6b's files): the beacon is now removed at quit in
  `main::take_beacon_at_quit`; if E4-6b gives the supervisor a quit handler
  of its own, that function can go.
