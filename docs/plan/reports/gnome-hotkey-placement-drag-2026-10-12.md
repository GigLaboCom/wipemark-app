# The panel's shortcut under GNOME, and the Placement page's drag — 2026-10-12

*Branch `fix/gnome-hotkey-placement-drag` from `feat/e0-e6-shell` (`177bfaa`).
Acceptance findings F11 and F12 (`v1-acceptance-2026-10-12.md`). Decisions
D641–D649 (`docs/plan/README.md` §4). Not merged.*

**Для владельца.** Сочетание клавиш для панели теперь работает в GNOME — и под
Wayland, и под XWayland, и под X11. Wipemark записывает его в собственные
«Пользовательские комбинации клавиш» GNOME (то же место, что Параметры ›
Клавиатура), и GNOME сам запускает `wipemark --panel`. Если Wipemark уже
открыт, новый процесс передаёт ему просьбу показать панель и сразу выходит —
второе окно не открывается; если не открыт, Wipemark запускается и показывает
панель. Пока вы записываете новое сочетание, старое снимается, а при выходе из
поля записывается новое (Escape возвращает старое). Важно: на Ubuntu сочетание
по умолчанию Ctrl+Alt+D уже занято GNOME — это «Скрыть все обычные окна», —
поэтому под полем будет написано, что оно не задано и кем занято; запишите
другое, например Ctrl+Alt+W. После выхода из Wipemark сочетание остаётся в
GNOME и запускает Wipemark с панелью; если удалить программу, запись нужно
убрать самому в Параметры › Клавиатура. Перетаскивание окна на странице
«Размещение» тоже исправлено: раньше окно бралось только за маленький значок
в середине выбранной клетки, а нажатие в любом другом месте клетки ничего не
делало и ничего не писало в журнал. Теперь окно берётся за всю выбранную клетку.

## Check by eye

1. **The shortcut.** In the owner's session (GNOME on Wayland), start the new
   build. Open Settings › General. Under *Shortcut for the panel* the line
   should say, in red, *Not set: GNOME already uses these keys for "Hide all
   normal windows" (org.gnome.desktop.wm.keybindings show-desktop)…* — that is
   the shipped Ctrl+Alt+D, refused by name.
2. Click the field, press **Ctrl+Alt+W** (or any chord GNOME does not use).
   The line should turn grey: *Set in GNOME's keyboard shortcuts: works from
   any application, and starts Wipemark when it is not running…*
3. Open GNOME's **Settings › Keyboard › View and Customise Shortcuts › Custom
   Shortcuts**. There is an entry **"Wipemark — panel"**, shortcut
   **Ctrl+Alt+W**, command `…/Wipemark-0.1.0-x86_64.AppImage --panel` (with
   `env WIPEMARK_DATA_DIR=…` in front when the run uses a scratch data
   directory). Any custom shortcut that was there before is still there.
4. Switch to another application (a browser, a terminal) and press
   **Ctrl+Alt+W**: the panel appears. Press it again: the panel goes. Only one
   Wipemark window set and one process (`pgrep -c -f wipemark`).
5. Quit Wipemark from the tray. Press **Ctrl+Alt+W**: Wipemark starts and
   opens its panel.
6. In Settings › General click the field again: while it listens, the entry in
   GNOME's Custom Shortcuts loses its keys (the page may need reopening to
   show it). Press **Escape**: Ctrl+Alt+W is back. Click and press
   **Backspace**: the field is empty and the GNOME entry is gone.
7. **The drag.** Settings › Placement: press the mouse **anywhere in the lit
   cell** — not only on the small window picture in its middle — and drag it
   onto another cell; release. The cell moves. A click on a cell still
   chooses it. With **Close after a drop** on, a drop closes Settings and a
   click does not.

## H — the shortcut under GNOME

**Why the old road could not work.** On Linux the only registrar was an X11 key
grab, skipped in a Wayland session (D346, D618); under XWayland — the default
since D611 — a grab hears only keys typed into other X11 clients. The owner's
machine has no `org.freedesktop.portal.GlobalShortcuts` (GNOME added it in 48;
46 has only `Settings`), no `xdg-desktop-portal-gnome`, and
`custom-keybindings` was `@as []`.

**What was built** (`apps/wipemark-app/src/gnome.rs`, `instance.rs`):

- **GNOME's own custom shortcut** (D641). One entry per action with a chord, at
  `/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/wipemark-panel/`
  (`wipemark-show/` for the main window): `name` from the catalogue
  ("Wipemark — panel", en/ru/de), `binding` in GNOME's spelling, `command`.
  Written with the `gsettings` command on the registrar's own thread, each call
  bounded at 5 s. Chosen by `hotkey::road`: `XDG_CURRENT_DESKTOP` naming GNOME,
  in **any** session type (D647).
- **The accelerator** (D642): `gnome::accelerator` — `CmdOrCtrl+Alt+D` →
  `<Control><Alt>d`, keysyms for named keys; tested both ways for every key the
  row can hold.
- **The command** (D643): `$APPIMAGE` inside an AppImage, `current_exe()`
  otherwise, `env WIPEMARK_DATA_DIR=<dir>` in front when set, quoted for
  `g_shell_parse_argv`. Asserted again at every launch (the first
  `Follower::moved` asks for every action), left at quit.
- **The list** (D644): `custom-keybindings` edited, never replaced; ours added
  after everyone else's, ours alone removed.
- **Conflicts** (D645): the wm, shell, mutter, mutter-wayland and media-keys
  schemas and every other custom shortcut are read first; a holder is named by
  its GNOME description (`gsettings describe`) or its own name, and nothing of
  ours is left on the old keys.
- **One instance** (D646): `<data dir>/wipemark.sock`, 0600, a thread of its
  own; `--panel` / `--show` alone connect first and exit 0 when answered; a
  stale socket is replaced; the forwarded panel toggles, as a press does on the
  other platforms. `--show` is new.
- **The recorder** (D649): entering it releases the action's registration (the
  GNOME entry, or the X grab); leaving registers the row's chord — the new one,
  or the old after Escape; Backspace leaves nothing. Linux only; macOS
  unchanged.

**Removing the application** leaves the GNOME entry in the person's settings
(a `.deb`'s `postrm` runs as root, outside every user's dconf); the line under
the field says it stays and where to remove it, and
`docs/architecture/packaging.md` says so for the packages.

### Live check, and the owner's dconf

`scripts/verify/gnome-hotkey/private-gsettings.sh` runs the real application
and the real `gsettings` on a private session bus (`dbus-run-session`), a
private dconf (`XDG_CONFIG_HOME`, `XDG_RUNTIME_DIR` in a scratch directory) and
an Xvfb on `:94`, with `XDG_CURRENT_DESKTOP=ubuntu:GNOME`. Run on 2026-10-12
with the debug build of `d71ee81`, all 20 checks passed:

```
a. the private dconf holds no custom shortcuts of the owner's: ok
   show-desktop here: ['<Primary><Super>d', '<Primary><Alt>d', '<Super>d']
b. Ctrl+Alt+D is refused as GNOME's (log): ok
b. nothing of ours written, the stranger's entry kept: ok
c. binding <Control><Alt>w: ok
c. name 'Wipemark — panel': ok
c. command runs this program with --panel and the data dir: ok
c. listed after the stranger's entry: ok
d. the second launch exits 0 (22 ms): ok
d. the running instance took the summons: ok
d. the panel opened in the running instance: ok
d. one application process: ok
d. a second press takes the panel away: ok
e. the binding stays after the application quits: ok
f. with nothing running it starts and opens the panel: ok
g. entering the recorder takes the binding away: ok
g. Ctrl+Alt+E recorded and set on leaving: ok
g. Escape puts the old binding back: ok
g. Ctrl+Alt+D is refused as Show desktop's, nothing set: ok
g. Backspace clears it: nothing set, an empty row: ok
   the private dconf database: /tmp/wgh.RC2o3Y/config/dconf/user
owner's dconf (media-keys, wm keybindings): unchanged (ad428489ac1c20a615d685db10342c6966a624150dbf86fbb83d5e1539747184)
private-gsettings: all checks passed
```

**What the live check does not cover**: GNOME itself. No gnome-shell or
gsd-media-keys runs on the private bus, so the step from "the keys are pressed"
to "GNOME runs the command" is GNOME's, checked by the owner (above, steps
3–5); the check runs the very command GNOME would run, and reads back what
GNOME's own `gsettings` holds.

**The owner's dconf was not touched.** The script hashes `dconf dump
/org/gnome/settings-daemon/plugins/media-keys/` and `dconf dump
/org/gnome/desktop/wm/keybindings/` of the real session before and after; both
runs of the day gave `ad428489…747184` before and after. The writes landed in
the scratch `config/dconf/user`. Every test in the suite runs over
`gnome::tests::Fake`, never a real `gsettings`. The Settings page was looked at
by screenshot on the private display: the refusal in red naming *Hide all
normal windows*, the grey *Set in GNOME's keyboard shortcuts…* after
Ctrl+Alt+E.

## P — the Placement page's drag

**Reproduced** on an Xvfb of our own (`:95`) with a scratch data directory,
`--settings=placement`, and the mouse driven by XTest
(`scripts/verify/linux-placement/xtest.py`, a ctypes helper over libXtst and
libX11; xdotool is not installed):

- a press **on the chip** (26×18 points in the middle of the lit cell) dragged
  and dropped as built — onto a cell, from a hand-placed rectangle, at scale 1
  and at `GPUI_X11_SCALE_FACTOR=2`, through the default backend and through the
  XWayland backend choice (`WAYLAND_DISPLAY` naming nothing,
  `XDG_SESSION_TYPE=wayland`); *Close after a drop* closed Settings;
- a press **anywhere else in the lit cell** — the rectangle that reads as "the
  window" — started nothing, dropped nothing, and logged nothing. That is F12
  as the owner saw it.

Ruled out on the way: the cells are not disabled under XWayland (`can_place`
reads `X11` there), the carried `gpui-pre-linux` patch touches only XDND, and
GPUI's internal drag needs only a left press and two points of movement.

**Fixed** (D648): the chosen cell carries the same `on_drag(DraggedWindow)` as
the chip — the chip, inside it, still hears its own press first, and a press
with no movement is still the cell's click — and a window put somewhere by hand
is drawn no smaller than 24×16 points (`MANUAL_CHIP`, from 14×10).
`scripts/verify/linux-placement/placement-drag.sh` passes 5 of 5 on the fixed
build; with the cell's drag taken out it fails a (and b, which starts where a
should have left the window), as expected.

**Not covered**: a window manager. mutter is the only one on the host, and an
earlier test mutter crashed onto the owner's desktop, so nothing here ran under
one. If the owner's drag still fails with a press on the cell, the next suspect
is the window manager's handling of the pointer during a press.

## Red first

`docs/plan/reports/gnome-hotkey-placement-drag-red.py` deletes each protection
once and runs the test that guards it: **11 of 11 red** — letters and Shift in
the accelerator, `$APPIMAGE` and the data directory in the command, the list
edit, the conflict check, the instance forward, the summons rule, the
recorder's suspension, GNOME before the X11 grab, and the lit cell's drag.

## Gates

Once, at the end, on the host (`LIBRARY_PATH` at the `libxkbcommon-x11.so`
symlink), every cargo command with `--locked`:

| gate | result |
|---|---|
| `rustup run nightly rustfmt --edition 2021 --check …` | clean |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean |
| `cargo test --workspace --locked` | 2074 passed, 0 failed, 11 ignored (the app 827; `a_port_something_else_holds_is_stepped_past` passed — the owner's AppImage did not hold its port) |
| `scripts/check-dep-direction.sh` | pass |
| `scripts/check-gpui-pin.sh` | pass |
| `scripts/check-zune-pin.sh` | pass |
| `cargo test -p wipemark-app --features local-llama --locked` | 829 passed, 0 failed, 2 ignored |
| `cargo test -p gpui-pre-linux --locked` | 56 passed |

macOS was not compiled here: `gnome.rs` and `instance.rs` build there (the
module-level `allow(dead_code)` off Linux), the summons and the suspension are
Linux's alone, and the macOS CI lane's clippy is the check of that.

The sandbox this ran in refused `GIT_CONFIG_NOSYSTEM=1` on version-control
commands (it cannot prove where such a command writes); they ran without it,
in this worktree only.

## For the coordinator: CLAUDE.md

Not edited here. Sentences to change:

1. In **"Where things are"**, the `apps/wipemark-app/src/` table, add two rows
   after `hotkey.rs`:
   - `gnome.rs` — "GNOME's own custom keyboard shortcuts as the system-wide
     shortcut's registrar on GNOME, either session type: the chord in GNOME's
     accelerator spelling, the command GNOME runs (`$APPIMAGE` or the
     executable, `WIPEMARK_DATA_DIR` through `env`), the `custom-keybindings`
     list edited and never replaced, who already holds the keys (D641–D647)"
   - `instance.rs` — "one Wipemark per data directory: `<data dir>/wipemark.sock`,
     a launch whose whole request is `--panel` or `--show` handed to the running
     instance and exiting 0 (D646)"
2. In the `hotkey.rs` row, replace "the registration — Carbon on macOS, an X11
   key grab on a thread of its own on Linux under X11 (D346, B1)" with "the
   registration — Carbon on macOS, GNOME's own custom shortcut on GNOME
   (D641), an X11 key grab on a thread of its own elsewhere under X11 (D346,
   B1)".
3. In **"A shortcut is recorded, not typed…"**, replace the sentence that
   begins "Registration is `global-hotkey` — …" up to "(D346, `x11_session`;
   elsewhere the row says *Stored, and not active*);" with the paragraph now in
   `docs/architecture/rules.md` (the same rule, beginning "Registration is
   `global-hotkey` — Tauri's own plugin's crate, a sibling of `muda`, Carbon
   `RegisterEventHotKey` with no Accessibility prompt on macOS; on **GNOME**, in
   either session type, one of GNOME's own custom keyboard shortcuts …"), and
   replace "On GNOME the shipped Ctrl+Alt+D is one of *Show desktop*'s
   bindings, so expect it refused there and record another." with "On Ubuntu's
   GNOME the shipped Ctrl+Alt+D is one of *Show desktop*'s bindings ("Hide all
   normal windows"), so it is refused there by that name and a person records
   another."
4. In **"Which screen the panel opens on…"**, after "…would make the second
   impossible.", add: "The window is picked up by the whole lit cell, not only
   by the chip drawn in it (D648)."
5. In the **flags table**, add a row: `--show` — "brings the main window
   forward; on its own it starts the application as a plain launch does, and
   with an instance running it is handed to that instance (D646)"; and to the
   `--panel` story (no row today): "with an instance running for the same data
   directory, `--panel` alone is handed to it over `<data dir>/wipemark.sock`
   and the launch exits 0 — the command GNOME's shortcut runs (D646)".
6. In **"The next free decision block"**: D641–D649 are taken; the next block
   is **D650**.
