# The tray

The menu-bar item (macOS) and the indicator (Linux): one menu, the
clicks it sends, what the close button does while it exists, and how
Quit takes it down. The code is `apps/wipemark-app/src/tray.rs`; the
wiring is `install_tray` / `adopt_tray` in `apps/wipemark-app/src/main.rs`.
The system-wide shortcut beside it is [hotkeys.md](hotkeys.md).

## The menu

Show Wipemark · Show the panel · Clean Clipboard — not yet (disabled) ·
Unload model (enabled while a model is loaded) · — · Appearance ▸
System / Light / Dark · — · Settings… · Quit Wipemark.

One builder, `tray::build_menu`, makes it on both platforms out of one
value of words, `tray::Labels`, read from the catalogue on the GPUI
thread. A click reaches the application as a `TrayCommand` over a
`flume` channel — `muda`'s callback has no `&mut App` in scope — and
`main::adopt_tray` performs it. Nothing about the menu differs between
macOS and Linux; what differs is which thread owns it.

## macOS

The menu and the item are built on the main thread inside GPUI's run
loop, which is AppKit's, and changed in place from the GPUI thread (a
theme tick, the labels after a language change, Unload's enabled
state). The icon is the 64 px black template; macOS inverts it for a
dark menu bar.

## Linux (E10)

`tray-icon` 0.21 on Linux is libayatana-appindicator — a
StatusNotifierItem on the session bus — whose menu is a GTK 3 menu that
`muda` builds. GTK wants its own main loop on the thread that made the
widgets, and a GPUI process runs none, so:

**D340 — the tray has a thread of its own.** `tray::install` starts
`wipemark-tray`, which initializes GTK (`gtk::init`, with
`gtk::disable_setlocale` first — GTK would otherwise call
`setlocale(LC_ALL, "")` process-wide from a thread that is not GPUI's,
and a decimal comma under `ru_RU` or `de_DE` is not something llama.cpp
or anything else in the process should inherit), builds the menu and
the item, and runs `gtk::main` until Quit. The widgets never leave that
thread: a change made on the GPUI side (`Tray::show_theme`, `relabel`,
`show_loaded`) is a `tray::Change` sent over a channel and applied by a
local future on GTK's main context. This is how `tray-icon`'s own
documentation and Tauri do it — an event loop on the thread the item is
created on — with GTK's loop instead of `tao`'s. GTK 3 is therefore a
link-time dependency of the Linux binary (`libgtk-3.so.0`), as it is of
every Tauri application; the indicator library itself is loaded at run
time. Packaging (E10's `.deb`) has to declare `libgtk-3-0` and
`libayatana-appindicator3-1`.

`tray::install` answers asynchronously — a receiver that answers once,
`Some(tray)` or `None` — on every platform, because the Linux answer
comes from that thread and waiting for it on the GPUI thread would be a
blocking call. `tray::when_installed` runs `adopt_tray` once the answer
is `Some`, and never for `None`.

**D341 — an item nobody would see is no item.** The thread climbs a
ladder and answers `None`, with a warning in the log naming the step,
at the first refusal:

1. **The library.** `libayatana-appindicator3.so.1`,
   `libappindicator3.so.1`, then the bare `.so` of each — the names and
   the order `libappindicator-sys` tries — are `dlopen`ed first. That
   crate *panics* when none loads, so it is never reached without one.
2. **The toolkit.** `gtk::init` fails without a display it can reach.
3. **A host.** On the session bus, `org.kde.StatusNotifierWatcher` has
   an owner, and its `IsStatusNotifierHostRegistered` is `true`. GNOME
   without the AppIndicator extension has no watcher; an item registered
   there is drawn by nobody, and libayatana's XEmbed fallback is not
   counted as a way back, because GNOME draws that nowhere either.
4. **The build** of the menu and the item.

The ladder is the trait `tray::linux::Desktop`, so its order, each
refusal and the update loop are tested with a fake desktop
(`a_missing_library_is_no_tray_and_nothing_after_it_runs`,
`every_refusal_is_no_tray`, `the_steps_run_in_order`,
`an_installed_tray_carries_changes_and_leaves_on_quit`,
`dropping_the_tray_ends_its_thread`, `an_item_nobody_would_draw_is_refused`).
`None` keeps the rule CLAUDE.md states: a launch without a tray keeps a
close button that closes.

**D342 — the icon is a white glyph with a dark outline.** A
StatusNotifier host draws the pixels it is handed; there is no template
image. The panel's colour is not knowable from the application — GNOME's
top bar is dark under a light theme, KDE's follows the theme, and the
application's own appearance says nothing about either — so rather than
pick the black or the white file by guessing, `tray::panel_image` draws
the template's broom white over a three-pixel (at 64 px; about one at
22 px) dilation of it in black at 220/255. The white carries it on a
dark panel and the outline on a light one
(`the_linux_icon_reads_on_a_light_and_a_dark_panel`,
`the_linux_icon_keeps_the_glyph_white_inside_its_outline`). It is
derived at run time from the one template, so the broom is drawn once,
in `icons/create-icons.sh`.

**D347 — one indicator id per process.** `tray-icon` writes the icon to
`$XDG_RUNTIME_DIR/tray-icon/tray-icon-<id>-0.png` and removes it when the
item drops. The id is `wipemark-<pid>`, so a second instance (a
development build beside the installed one) cannot take the first one's
icon away. A `kill -9` leaves one 2 KB file in the runtime directory,
which the session clears at logout.

## The close button

The rule is CLAUDE.md's: the close button only hides while there is a
way back, and the hook is registered inside the `Some`
(`without_a_tray_the_close_button_closes`,
`with_a_tray_the_close_button_keeps_the_window`). How it hides is
`tray::close_button`:

| platform | with a tray | without |
|---|---|---|
| macOS | the application hides (`App::hide`); the Dock and the menu bar bring it back | closes |
| Linux, GPUI on X11 | the main window is **minimized**; Show restores it | closes |
| Linux, GPUI on Wayland | **closes** | closes |
| Windows | — (no tray) | closes |

**D343 — Linux minimizes, and only under X11.** GPUI's `App::hide` is
AppKit's `hide:` on macOS and nothing at all on Linux (it logs "hide is
not implemented"), so the macOS hook would have been a close button
that does nothing. On X11 the hook minimizes the main window
(`WM_CHANGE_STATE` iconic) and **Show** sends `_NET_ACTIVE_WINDOW`,
which every X11 window manager answers by restoring and raising it. A
Wayland compositor refuses an activation that does not carry a token
from the user's click, and a click on the indicator hands that token to
the shell, not to us — so under Wayland a minimized window could not be
brought back by Show, and the close button keeps closing. With the main
window closed there, the tray still offers the panel, Settings and Quit,
and GPUI's default on Linux ends the process when its last window
closes.

**Closing the main window while Settings is open.** On macOS the whole
application hides, Settings with it, and Show or the Dock brings both
back. On Linux under X11 only the main window is minimized: Settings
(and the panel) stay where they are — each has its own close button,
and Settings writes every change as it is made, so there is nothing a
close could lose. Closing Settings afterwards closes Settings; the
process goes on, because the minimized main window is still a window,
and the tray's **Settings…** opens it again. The process ends from the
tray's **Quit**. Before this change, closing the main window on Linux
closed it, left Settings and the process running with no way back to
the main window, and ended the process only when Settings was closed
too.

## Quit

**D344 — Quit is GPUI's quit, and it takes the item and the beacon
with it.** Quit calls `App::quit` on both platforms, never AppKit's
terminate or `std::process::exit`, so the quit handlers run: the engine
host drops the local model and waits for its worker (D96), the Settings
window saves its rectangle, and two handlers added here —

* `Tray::leave` sends the tray's thread a `Leave`; the thread drops the
  item (the indicator goes passive and its icon file is removed), ends
  the GTK loop, and answers, inside GPUI's 200 ms quit budget. On macOS
  there is nothing to wait for.
* `take_beacon_at_quit` removes the MCP server's beacon
  (`<data dir>/mcp.json`) if it names this process. The supervisor
  thread removes it when its command channel closes, but a quit ends
  the process before that thread is scheduled, so until now every Quit
  left a beacon naming a dead pid for the CLI to see through
  (`Beacon::alive`). The listener's socket goes with the process.

Then GPUI's run loop returns, `main` returns, and the process exits —
every window with it, the tray's thread and the MCP threads included.

## The theme tick

**D345 — the tick is put back on every click.** A check item in a GTK
menu (and in AppKit's, through `muda`) ticks or unticks itself when
clicked, and choosing the theme that is already chosen changes nothing
in `Preferences`, so nothing would move the tick back: clicking the
ticked "Dark" left no item ticked. `adopt_tray` calls `show_theme(choice)`
after every Theme command; `muda` suppresses the event its own
`set_checked` would otherwise send.

## Not here

* **Windows** — `tray-icon` wants a win32 message pump on the
  registering thread; `install` answers `None`. E10.
* **A host that leaves.** The ladder asks once, at launch. If the
  StatusNotifier host goes away later (the GNOME extension is turned
  off), the item disappears and the main window's close button still
  minimizes; the window is still in the dock and in Alt+Tab.
* **Left click.** On Linux a click on the indicator opens the menu; the
  StatusNotifier `Activate` event is not delivered by `tray-icon` there.

## Running it

No gate draws a real indicator — that needs a display, a session bus
and a StatusNotifier host. Two scripts come closest:

* `scripts/verify/linux-tray/headless.sh` runs the real `wipemark-tray`
  thread — libayatana loaded, `gtk::init` and `gtk::main`, the bus asked,
  the menu changed over the channel, the item taken down by a Leave —
  under `xvfb-run` on a private `dbus-run-session`, three times: no
  watcher (no item), a watcher with no host (no item), and
  `fake-watcher.py` standing in for the host (an item, which it records
  registering, and whose icon file the Leave removes). The test it runs,
  `tray::tests::linux::the_real_desktop_answers_as_the_bus_says`, is
  `#[ignore]`d and does nothing without the script's
  `WIPEMARK_TRAY_EXPECT`, so `--ignored` on a real desktop cannot put an
  item in its panel.
* `scripts/verify/linux-tray/host.sh` reports, without starting anything,
  whether this machine's session would get an item, what the close
  button would do, and whether the shortcut would be asked for.

What neither can see — the item in a real panel, the menu opening, the
window minimizing and coming back — is the owner's checklist in
[`docs/plan/reports/linux-tray-2026-10-07.md`](../plan/reports/linux-tray-2026-10-07.md).
