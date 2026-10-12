# Linux: the panel placed, through XWayland — 2026-10-12

Branch `fix/linux-placement-x11`, from `feat/e0-e6-shell` at `180984e`.
Acceptance findings F8 and F2'
([v1-acceptance-2026-10-12.md](v1-acceptance-2026-10-12.md)). Decisions
D611–D619 (`docs/plan/README.md` §4).

**Для владельца.** На Linux приложение теперь по умолчанию рисует окна через
XWayland, если сеанс Wayland его предлагает (у GNOME — всегда): под чистым
Wayland программа не может поставить окно в нужное место, а под X11 может.
Открытые из Wipemark документы и ссылки по-прежнему запускаются как обычные
Wayland-приложения — переменные окружения процесса не трогаются. Если нужен
чистый Wayland, запустите с `WIPEMARK_WAYLAND=1`: тогда страница «Где
открываются окна» прямо говорит, что место окна выбирает система, а клетки
затемнены и не нажимаются. Под XWayland панель открывается в своей шестой
части экрана — того монитора, где указатель, — с системной полосой заголовка
сверху (полоса тоже помещается в рабочую область), остаётся поверх остальных
окон и видна на всех рабочих столах; её можно двигать за полосу или за любое
место самой панели. Выбор клетки в настройках, пока панель открыта на этом
мониторе, сразу её передвигает. Каждый монитор теперь различается (раньше у
всех мониторов был один общий ключ из нулей), а старая запись под нулевым
ключом один раз переходит к основному монитору; если там было «где я
оставил» — это почти наверняка было место, куда окно поставил сам GNOME, поэтому
оно становится клеткой по умолчанию, а прямоугольник сохраняется (кнопка «Где я
её оставил» вернёт его). Главная ошибка исправлена: прямоугольник, который
выбрал оконный менеджер или наше собственное перемещение, больше никогда не
записывается как ваш ответ «вручную». «Настройки» по шестерёнке под XWayland
поднимаются поверх, а не закрываются и открываются заново (мигания нет).
Глобальное сочетание клавиш в сеансе Wayland по-прежнему не работает, и строка
теперь так и говорит — «в сеансе Wayland». Проверить глазами — по списку в
конце.

## 1. XWayland by default (D611)

**What GPUI does.** `gpui-pre-linux 0.3.8` picks Wayland whenever
`WAYLAND_DISPLAY` is set (`display_connection::select_backend`), but takes
the set of display servers it may use as a parameter:
`gpui_platform::linux(WindowingModes)` (the default `application()` is
`WindowingModes::all()`). And it hands its own view of the session to the
programs it opens: `open_with_system` applies the connection's
`GraphicalEnvironment` to the command, setting or *removing*
`WAYLAND_DISPLAY` from it.

**What was built.** `apps/wipemark-app/src/session.rs`: `choose` is a pure
function of `WAYLAND_DISPLAY`, `DISPLAY`, `WIPEMARK_WAYLAND` and "does the X
server answer" (asked only when it matters — a connection opened and
dropped, which on GNOME starts XWayland if nothing has yet). Both set, no
opt-out, the X server answers → `Backend::XWayland`, and `main` builds the
application with `gpui_platform::linux(X11 | HEADLESS)`. Otherwise
`gpui_platform::application()` as before. One INFO line says which and why
(`wipemark::session`).

**Deviation from the task.** The task said to remove `WAYLAND_DISPLAY` at the
very start of `main`. I used GPUI's windowing modes instead, for two
reasons: `set_var` once any thread runs is undefined behaviour (the decision
would have to come before logging, and the log line would be lost), and the
programs the application opens would lose `WAYLAND_DISPLAY` with it — a
`.md` opened "with the default application" would start under XWayland too.
With the modes, GPUI's environment keeps `WAYLAND_DISPLAY`, nothing in the
process environment changes, and the result for our windows is the same
(`cx.compositor_name()` is `"X11"`). The GTK thread of the tray still sees
`WAYLAND_DISPLAY`, exactly as before this branch.

**D594 under XWayland.** `chrome::forward` keys on `cx.compositor_name() ==
"Wayland"`; under XWayland it is `"X11"`, so Settings and Compare are
activated (`_NET_ACTIVE_WINDOW`, source 1, timestamp 0) rather than closed
and reopened — F2' goes away with the default. mutter's handler
(`window-x11.c`) replaces a zero timestamp with the current time and
activates; I read that in the source, and could not see it live (§8). If it
is refused on the owner's desktop, the window is marked as wanting attention
and nothing blinks; the fix would be a user timestamp from the click in the
message, which GPUI's `activate` does not pass (`x11/window.rs:1530`).

**The shortcut.** Unchanged: `hotkey::x11_session` still skips the X grab
while `XDG_SESSION_TYPE=wayland`, XWayland included. The row's sentence was
"not available on this platform yet"; in a Wayland session it is now
`hotkey-unavailable-wayland`: "Stored, and not active: in a Wayland session
Wipemark cannot hear a shortcut pressed in other applications" (D618,
en/ru/de).

## 2. Moving a window under X11 (D612, D613)

**What GPUI does.** `X11WindowState::new` creates the window at the origin
it is handed (plus 2 px in x), and its `WmSizeHints` carries a minimum and a
maximum size and no position flag (`x11/window.rs` ~787), so mutter places
the window itself. `PlatformWindow` has no move. `Window` implements
`HasWindowHandle` and hands out an `XcbWindowHandle` (the X window id).
`bounds()` is the client's rectangle, from `ConfigureNotify`: root
coordinates after a synthetic one, relative to the frame after a real one.

**What was built.** `apps/wipemark-app/src/x11.rs`: an x11rb
`RustConnection` of our own on a thread of its own (`wipemark-x11`), started
in `main` only when `cx.compositor_name() == "X11"`. x11rb 0.13.2 with
`randr` is the version and feature GPUI's X11 client already builds, so
`Cargo.lock` gains two dependency edges (`x11rb`, and `uuid` for §5) and no
package — changed deliberately, and `--locked` holds after it. Requests:

| ask | how | reply |
|---|---|---|
| monitors | RandR 1.5 `GetMonitors` + `_GTK_WORKAREAS_D<n>` / `_NET_WORKAREA` | kept, read with no X traffic |
| pointer | `QueryPointer` on the root | waited for ≤ 100 ms (the one synchronous ask) |
| frame | `TranslateCoordinates`, `GetGeometry`, `_NET_FRAME_EXTENTS`; optionally once `WM_STATE` is set and the window viewable (≤ 3 s) | a channel, awaited from a task |
| hint | `WM_NORMAL_HINTS` position (program-specified) and gravity | none |
| move | `_NET_MOVERESIZE_WINDOW` (north-west gravity for a frame's corner, static for a client's); `ConfigureWindow` where `_NET_SUPPORTED` lacks it | none |
| float | `_NET_WM_STATE` add `ABOVE` + `STICKY` | none |

**The panel at open** (`panel::placed_by_the_window_manager`): the asked
corner into `WM_NORMAL_HINTS` at once (best effort: GPUI maps the window
inside `open_window`, so a manager that read the hints first has already
placed it); once the manager has the window, the frame is read, the zone is
computed for the panel's size *with* the decoration (`screen::Chrome`), the
frame's corner asked for, the rectangle handed to `we_placed_it`, then above
and sticky. **Live:** `Preferences` → Placement → a cell on the card of the
panel's display moves it at once through `screen::put`, as `translate` does
on macOS. **Settings** opens at its remembered client rectangle the same way
(`screen::settle_opened`, static gravity, so what `geometry_of` saved is what
is restored). The main and Compare windows are left where mutter opens them
(deviation: not asked, and the main window's "centred" is centred over the
whole X screen, which on two monitors straddles them).

## 3. The panel floats (D619)

macOS's `afloat`: `NSFloatingWindowLevel`, `hidesOnDeactivate: false`,
`canJoinAllSpaces` + `fullScreenAuxiliary`, a borderless resizable style
mask, `movableByWindowBackground`. Under X11: `_NET_WM_STATE_ABOVE` and
`_NET_WM_STATE_STICKY` (every workspace); X11 has no deactivation hiding.
Not `SKIP_TASKBAR` — macOS does not hide the panel from the application's
window list. The window manager's title bar is kept (GPUI gives a titleless
window server-side decorations under X11): it is the handle a person knows,
and taking it away brings back GPUI's client-side inset for nothing asked
for. "Drag anywhere on the panel to move it" (`panel-help-move`) is now true
on Linux too: a press that travels four points calls
`Window::start_window_move` (under Wayland that is the panel's only handle).

## 4. Under the pointer (D612)

`screen::under_the_pointer` on Linux: the monitors' reading, then
`x11.pointer(100 ms)` → `x11::monitor_at` (the monitor holding the point,
else the nearest). Under Wayland it stays `None` and the primary display
answers.

## 5. Displays named and told apart (D614, D615)

RandR monitor names (`HDMI-1` on the owner's host — XWayland names its
outputs as the compositor does, read with `xrandr --listmonitors` against
`:0`, a query and nothing else) are the cards' names. The key is
`uuid::new_v5(NAMESPACE_DNS, name)` — exactly how GPUI's Wayland backend makes
its display uuid, so one display has one key in both modes. There is no EDID
under XWayland (`xrandr --prop` shows none), so the key follows the
connector. `Screen::same_display` compares keys (every monitor of the X
screen has GPUI's one id); the two comparisons in `settings.rs` that used the
id ("the panel is here", and whether a choice moves the panel) use it.

**Rows under the nil uuid — what I chose.** On the first reading that names
the displays (`Preferences::screens_changed` → `adopt_nil`), the primary
display adopts them once: a cell as it was; a `manual` as the default cell
**with its rectangle kept** (`placement::adopted`) — before D616 every
rectangle mutter opened the panel at was filed as manual, the owner's
`{x:44,y:906}` among them, so the answer is mutter's as often as the
person's, and "Where I put it" brings it back if it was a person's; the
Settings rectangle as it was (`window_state::adopt`). A display with a row of
its own keeps it. Both nil rows are deleted, and a log line says what
happened. Nothing is adopted while a display still answers to the nil key
(RandR not read yet, or not there).

## 6. The "manual" bug (D616)

`Preferences::ours` matched the asked rectangle to a point, so under X11 the
first rectangle mutter opened the panel at — none of the steps by which a
window manager carries out an ask is the rectangle asked for — was filed as
`manual`, and `manual` beats every cell. Now `placement::Placing`: every
report is debounced (400 ms, as before), and the last one is ours when it is
the rectangle asked for, when it came within `GRACE` (1.5 s) of the ask, or
when it is again where an ask landed; a person's otherwise. Under X11 the
rectangle weighed is the frame read off the X server once the window is
still. `a_window_managers_offset_is_not_a_person` asks at one rectangle and
hears mutter's cascade, then the frame 37 points lower, then the same again
30 s later — all ours — and then a drag, a person's;
`after_the_grace_a_rectangle_nobody_asked_for_is_a_person` keeps a person
heard.

## 7. Native Wayland (D617)

`placement::can_place(compositor)` is false only for `"Wayland"`. The
Placement page's banner then says
`settings-placement-system-decides` — "On this desktop the system decides
where a window opens, so these choices are kept and not used. Started without
WIPEMARK_WAYLAND, Wipemark runs through XWayland and places the panel
itself." (en/ru/de, no epic number) — in place of the line that says these
choices place the panel; each grid is dimmed, its cells, "Where I put it" and
"Restore default" disabled, the drop target off. The cards stay, so what is
stored is seen. `no_language_promises_more_than_the_product_does` and the
epic-number gate are green.

## Files

New: `apps/wipemark-app/src/session.rs`, `apps/wipemark-app/src/x11.rs`,
`scripts/verify/linux-placement/xvfb.sh`, this report and its
`-red.py`. Changed: `main.rs` (the modules, the backend, `x11::start`),
`screen.rs` (`same_display`, `Chrome`, `Monitors`, `put`, `settle_opened`,
`frame_of`, `of_rect`, the Linux pointer), `placement.rs` (`can_place`,
`NIL_KEY`, `adoption`, `adopted`, `Placing`, `GRACE`), `panel.rs` (the X11
placement, the settle, the drag), `settings.rs` (`Placing` in `Preferences`,
`adopt_nil`, `put_back`, the page under Wayland, the shortcut row),
`window_state.rs` (`adopt`), `display_watch.rs` (RandR as a source),
`hotkey.rs` (`unavailable_line`), the three catalogues, `Cargo.toml` and
`Cargo.lock`; docs: `window-placement.md` ("On Linux"), `packaging.md`
("XWayland by default"), `rules.md`, `docs/plan/README.md` (§4 D611–D619,
§7 E10).

## Live evidence

**mutter crashed, and it was mine.** The first attempt (13:42) was a
*headless* mutter 46.2 (`mutter --wayland --headless --virtual-monitor …`,
private session bus and runtime directory) meant to reproduce the owner's
XWayland-under-mutter with two virtual monitors. Its runtime directory path
was longer than a Unix socket allows, mutter hit `libmutter-ERROR: Failed to
create_socket`, aborted, and Apport picked the core up
(`/var/crash/_usr_bin_mutter.1000.crash`) and showed "mutter stopped" on the
owner's desktop. No mutter was ever started inside Xvfb `:98` (that Xvfb ran
with no window manager). With a short runtime directory two further headless
runs (13:43) worked and ended by their timeout; then the coordinator asked
for no more mutter, and none was started. No other window manager is
installed (`openbox`, `xfwm4`, `metacity`, `twm`, `fluxbox` absent). The
verifier now runs with `ulimit -c 0` and `APPORT_DISABLE=1`.

**Under the headless mutter (XWayland `:2`, monitors `Meta-1` 2560×1440 at
0,0 and `Meta-0` 1920×1080 at 2560,0), the debug build, `--panel`, empty
database:**

* `drawing through XWayland: a Wayland session offers X11 too…` (the
  child had mutter's `WAYLAND_DISPLAY` and `DISPLAY`, `XDG_SESSION_TYPE=wayland`);
* `X11 displays read monitors=[("Meta-1", true, …2560×1440…), ("Meta-0",
  false, …)]`;
* the panel asked for its frame at `(1923.33, 931.5) 420×297` — the
  bottom-right zone of Meta-1: the cell is `x 1706.7–2560, y 720–1440`, the
  frame 420 × (260 + 37) centred in it;
* `xwininfo` on the panel: client at **1923, 969** = the frame's corner plus
  mutter's 37 px title bar (`_NET_FRAME_EXTENTS = 0, 0, 37, 0`) — mutter
  honoured `_NET_MOVERESIZE_WINDOW` with north-west gravity to the pixel;
* `_NET_WM_STATE = _NET_WM_STATE_ABOVE, _NET_WM_STATE_STICKY,
  _NET_WM_STATE_FOCUSED`;
* no `window.zone.*` row was written.

**On a bare Xvfb `:98` (no window manager), the debug build and then the
release build, `scripts/verify/linux-placement/xvfb.sh`** — RandR names one
monitor `screen` (Xvfb refuses `xrandr --setmonitor`, so two monitors could
not be made here):

```
a. the panel is in the bottom-right zone (2990,680 420x260): ok
a. no zone row written by our own placement: ok
b. a move nobody asked for is the person's, under the key of 'screen': ok
c. the next panel opens where the person put it: ok
c. reopening wrote nothing: ok
d. the old manual is not obeyed: the panel is in the default zone: ok
d. the nil row is adopted and gone: ok
xvfb: all checks passed
```

(a) is the `ConfigureWindow` fallback after the 3 s wait for a manager that
never comes; (b) is another X client's `XMoveWindow`, which reaches the
application as a person's drag does.

**What mutter honoured and what was not seen:** `_NET_MOVERESIZE_WINDOW`
(north-west gravity, position and size) — honoured; `_NET_WM_STATE` add
`ABOVE`/`STICKY` — honoured; `_NET_FRAME_EXTENTS` — published (37 px).
Not seen live: the position hint before the map (whether the panel opens in
place without a jump — the move after it lands either way), Settings raised
by `_NET_ACTIVE_WINDOW` under XWayland (needs a click on the gear; no input
was synthesised under mutter), a drag by the title bar under mutter, and
`WIPEMARK_WAYLAND=1` (needs a Wayland compositor). Those are the owner's
checklist below.

## Red checks

`docs/plan/reports/linux-placement-x11-2026-10-12-red.py` — each protection
deleted once, the named test seen red, the source put back:

RED_TABLE

## Gates

GATES_TABLE

## Deviations

* `WAYLAND_DISPLAY` is not removed from the environment; GPUI's windowing
  modes choose X11 instead (§1).
* The panel keeps mutter's title bar under X11 (D619) rather than being
  borderless as on macOS.
* Only the panel and the Settings window are moved after they open; the main
  and Compare windows open where mutter puts them.
* Two monitors were seen only under the headless mutter; the bare-Xvfb
  verifier has one.
* Not checked live: Settings raised under XWayland, the native-Wayland page,
  a drag by the title bar under mutter (§ Live evidence).

## Builds

BUILDS

## Checklist for the owner (GNOME, Wayland session)

Run the AppImage (or the release binary) from a terminal; kill it afterwards
— it holds MCP port 5056. The log line `wipemark::session` should say
"drawing through XWayland".

1. **Zones, by default (XWayland).** Settings → Where windows open. Each
   attached monitor has its own card, named (`HDMI-1`…). Pick a cell, summon
   the panel from the tray: it opens in that sixth of the monitor the pointer
   is on, its title bar inside the work area (below the top bar, right of
   the dock). With the panel open on a monitor, click another cell on that
   monitor's card: the panel moves at once. Repeat on a second monitor if
   you have one; each keeps its own cell.
2. **No false "manual".** Summon and dismiss the panel a few times without
   touching it: the card keeps saying "A window opens at the … of this
   display", never "Where you put it". Then drag the panel by its title bar
   (or by any empty place on it): the card switches to "Where you put it",
   and the next summon opens it there.
3. **Above other windows.** With the panel open, click another
   application's window: the panel stays on top. Switch workspace: it is
   there too.
4. **Settings raised without a blink.** Open Settings, click the main window
   so Settings goes behind, press the gear: Settings comes forward, the same
   window (no close-and-reopen flash), on its page.
5. **The system title bars.** Main window, Settings, Compare and the panel
   each have exactly one title bar, GNOME's (no second, drawn bar).
6. **The shortcut row** (Settings → General): "Stored, and not active: in a
   Wayland session Wipemark cannot hear a shortcut pressed in other
   applications."
7. **With `WIPEMARK_WAYLAND=1`:** the log says "native Wayland, as
   WIPEMARK_WAYLAND asks"; the main, Settings and Compare windows draw our
   own title bars (D590); the Placement page's banner says "On this desktop
   the system decides where a window opens…", the grids are dimmed and the
   cells do not respond; the panel opens where GNOME puts it.
