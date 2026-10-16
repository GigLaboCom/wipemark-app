# System-wide shortcuts

A shortcut that reaches Wipemark from any application: one row per
action on the General page, a field that is clicked and then pressed
into, and a registration with the desktop that follows the row. The
shape is heretic-lazy-shot's `HotkeyInput` and the Tauri commands
behind it, rebuilt on gpui-component parts.

| where | what |
|---|---|
| `apps/wipemark-app/src/hotkey.rs` | the value (`Hotkey`), the row format, what a keystroke means to a recorder (`meaning`), the duplicate rule (`taken_by`), and the registrar |
| `apps/wipemark-app/src/recorder.rs` | the control: the field, the keyboard while it listens, the two buttons under it |
| `apps/wipemark-app/src/keys.rs` | the badge a chord is painted in, and what goes between its keys |
| `config.rs` → `hotkey.<action>` | one row per `hotkey::Action`, the chord as text, `""` for none |
| `main.rs` → `install_hotkeys` | registers what is stored, follows every change, performs a press |
| `apps/wipemark-app/src/gnome.rs` | GNOME's own custom keyboard shortcuts as the registrar on GNOME: the accelerator, the command, the list edit, who already holds the keys (D641–D647) |
| `apps/wipemark-app/src/instance.rs` | one Wipemark per data directory: `<data dir>/wipemark.lock` held by one process, and a second launch's `--panel`, `--show` or bare launch handed to it, on Linux and macOS (D646, D790–D794; "One Wipemark per data directory" below) |

| action | what a press does | shipped chord |
|---|---|---|
| `Action::Show` | brings the main window back (`show_main_window`) | none |
| `Action::Panel` | `panel::toggle`, after `cx.activate(true)` | `CmdOrCtrl+Alt+D` |

## One chord, three spellings

**The row** is `CmdOrCtrl+Shift+Alt+L` — a *format*, like a catalogue
id or a `--json` field. It is what `wipemark.db` holds and what the
registrar parses, so it is never localized and never platform-specific:
`CmdOrCtrl` is ⌘ on macOS and Ctrl elsewhere, and a database copied
between the two still names the same chord. `Ctrl` is ⌃, which only
exists as a modifier of its own on macOS; off macOS `Hotkey::parse`
folds it into `CmdOrCtrl`. The spelling is read generously — any case,
spaces around the `+`, modifiers in any order — and refused strictly:
a key before a modifier, two keys, no key, a key this build cannot
place, and anything without a modifier other than Shift are all read
as *no chord* and **left in the row**, the bargain every other
preference keeps.

**The keystroke** is GPUI's — what the window hands the recorder and
what `keys::Keys` paints (⌥ + ⇧ + ⌘ + L on macOS). It is a *character*,
not a key
position: GPUI reports ⇧7 on a US layout as `&` with Shift already
folded in. `hotkey::meaning` folds it back out, because a chord that
reads `⌘&` in the field and cannot be registered at all is not the one
the user pressed. US only, and honestly so: a layout with its own
shifted punctuation records those symbols as *unrecordable*, with a
sentence under the field, rather than as a guess.

**The registration** is a key position plus modifier bits, the only
form the desktop takes. `RegisterEventHotKey` matches on the physical
key, so a user on a Cyrillic layout who records ⌘⌥L presses the key
with Д on it — the key they pressed to record it.
`every_stored_spelling_is_one_the_registrar_parses` is the gate between
the first spelling and the third: every key the recorder can name is
one `global-hotkey` parses, as the same chord.

## What goes between the keys

`keys::Keys` is the badge, and `keys::Keys::divider` is what goes
between the keys of a chord. `gpui_component::kbd::Kbd` decides that
itself — a `const` chosen by `cfg`, nothing on macOS and `+`
elsewhere — so a window that wants ⌥ + ⌘ + D has nowhere to say so;
this is that component with the decision handed in. The default is
`keys::DIVIDER`, which is the library's own answer, and
`the_default_divider_paints_what_the_library_painted` asserts it
against `Kbd::format` itself rather than against a literal: a caller
who asks for nothing gets exactly what was painted before there was a
parameter.

The recorder asks for `" + "`. A field is where a chord is *read* — it
is the thing the row is about, at body-text size, and ⌥⌘D run together
is a glyph soup at that size. A hint in the corner of a menu item is
the other case and keeps the compact spelling, which is what somebody
expects to see there.

The *key* names stay upstream's. That table has forty entries in it —
⌫, ⎋, ⏎, ←, `Page Down`, both platforms — and it is the part of this
nobody should be maintaining twice; the modifiers are four, and they
are the only part `keys::spelled` writes for itself, because they are
the part a divider goes between. Nothing else in the application
imports `Kbd`: one module knows how a key is written.

## What a chord needs

A modifier other than Shift. lazy-shot's rule, kept verbatim: a
system-wide `L` or `Shift+L` would take a letter away from every
application, and the desktop would deliver exactly that. The recorder
does not wait silently — `hotkey-needs-modifier` goes under the field.
Function keys are not exempt.

## The recorder

Click the field and it listens — and, on Linux, the action's shortcut is
taken away from the desktop until it lets go (below, D649). Hold modifiers and it previews them
(`⌥⇧…`). Press a key and it records the chord and stops. Escape keeps
what was there; Backspace and Delete forget it; a click anywhere else
is Escape. Escape and Tab cannot be pressed *into* a recorder, so while
it listens they are two buttons under the field, each recorded with
whatever is held — lazy-shot's dropdown, in the shape the address row
on the MCP page already uses for its presets. An X at the right of a
set field clears it without listening.

### Why the keys are intercepted, not listened for

GPUI dispatches a keystroke to the key **bindings** first and to
`on_key_down` handlers only afterwards, and only if no binding took it.
A recorder that hung a handler on its element would never hear ⌘W —
the Settings window binds it to closing itself — or Escape, or Tab,
which gpui-component's `Root` binds to focus traversal. It would record
the keys nobody had spoken for and close the window on the rest.

`App::intercept_keystrokes` runs *before* the bindings and a
`stop_propagation` inside it keeps them from firing. While the recorder
listens it holds one; the interceptor checks the keystroke was aimed
at this recorder (it hears every window — `focus.is_focused`), swallows
it, and hands it to `Recorder::heard`. Dropping the subscription gives
the keyboard back; nothing is unbound. Modifier changes on their own do
not reach interceptors, so the live preview comes from
`on_modifiers_changed` on the element, which reaches the recorder
because it is focused while it listens.

### The chord is over when the modifiers are up

A key held for half a second repeats. If listening stopped the moment
W was recorded, the *repeat* of ⌘W would reach the Settings window's
binding and close it with the user's hand still on the keys. So there
is a third state, `Settling`: after a chord is taken the recorder goes
on swallowing keystrokes until every modifier is released.

### Two ways out, and why both

`on_focus_out` ends listening when focus moves — into a field, to
another window, to another application. It does nothing for a click on
something that takes no focus: the sidebar, a button, the page. A
recorder still swallowing keystrokes after the user had visibly moved
on would be a keyboard that stopped working, so the control's own
`on_mouse_down_out` answers that click — lazy-shot's document-level
`mousedown` listener, in GPUI. It sits on the whole control rather
than on the field, or the two buttons under the field would be
"outside".

### While it listens, the shortcut is not registered (D649)

On Linux a recorder that listens takes its action's registration away:
the GNOME entry is removed (or the X grab released), so the keys it held
reach the field — re-recording the current chord, or trying one GNOME
holds, would otherwise fire the shortcut instead of being heard. When the
recorder lets go, the row's chord is asked for again: the chord just
recorded, the old one after Escape or a click elsewhere, nothing after
Backspace. That is heretic-lazy-shot's flow — in Settings, unregister,
assign, leave, register — at the grain of one field rather than one tab.

The recorder emits `RecorderEvent::Listening(true)` when it starts and
`Listening(false)` once it is idle again (after `Changed`, after the
modifiers are up); `Preferences::recording_shortcut` keeps which action,
and `Preferences::shortcut_held` — `hotkey::held_for_desktop`, nothing for
the action being recorded, the row's chord otherwise — is what
`main::install_hotkeys` follows through `hotkey::Follower`. A recorder
dropped while it listens (its window closed under it) ends the suspension
(`observe_release`). macOS is unchanged (`SUSPEND_WHILE_RECORDING`).
`the_recorder_releases_the_keys_on_entry_and_asks_again_on_leaving` is the
gate; `scripts/verify/gnome-hotkey/private-gsettings.sh` step g drives it
live.

## The row is the request; the registration is the answer

`hotkey::Registration` is what the desktop said — `Registered`,
`Refused(reason)`, `Unavailable` (no registrar on this platform), or
`Unset` — and the row shows it under the field, for the reason the MCP
page shows both the port that was asked for and the one the server
took: a chord the OS refused is invisible from the preference alone,
and a user who has recorded one deserves to know before they press it
from another application and nothing happens.

`main::install_hotkeys` owns the registrar. It asks for what is
stored, observes `Preferences` and asks again only when a chord has
actually moved (the old chord is released *first*, whatever the new one
turns out to be — a change of mind that left the previous chord
registered would fire on a key the row no longer shows), and writes
each answer back with `Preferences::shortcut_registered` as it comes —
on Linux from the registrar's own thread (below, B1). A press comes back
over a `flume` channel and is performed on the GPUI side, the same
arrangement as the tray and for the same reason: the callback the
desktop calls has no `&mut App` in scope.

## One shipped chord, and it belongs to the panel

`hotkey::PANEL_DEFAULT` is `CmdOrCtrl+Alt+D` — ⌘⌥D on macOS — and it is
the only chord this build ships. The asymmetry is the decision:

* the **panel** is a window you *summon*, over somebody else's
  document, from inside whatever you are reading. One that has to be
  found in Settings before it can be summoned is one nobody summons;
* the **main window** is already reachable from the Dock, the menu bar
  and ⌘Tab, so taking a key away from every other application on its
  behalf buys nothing. Its row reads *Click to set* until somebody
  sets it.

Nothing is written to the database to make the default true. An
**absent** row means nobody has answered, and `config::read_hotkeys`
reads `Action::default_chord` there; an **empty** row is an answer —
somebody pressed Backspace — and stays empty. That is what lets a later
build move the shipped chord for everybody who never touched it, and
keeps a cleared field cleared. A row this build cannot *read* falls
back to no chord rather than to the default, for the reason it is left
in the row at all: it is still the user's answer, and handing them keys
they never chose because their own spelling is unreadable is exactly
the surprise these rules avoid.

A shipped chord can still be refused — the desktop may already have it
— and that is the ordinary case the line under the field exists for,
not a special one. `a_shipped_default_is_a_chord_this_build_can_use`
keeps the value itself honest: a modifier other than Shift, a spelling
its own row can hold, and no two actions shipping the same keys.

## Duplicates

`hotkey::taken_by` is lazy-shot's `findDuplicateHotkey`: two rows may
not hold one chord, and the row being edited is not its own duplicate.
`Preferences::assign_shortcut` refuses rather than writes, the recorder
puts the old value back and says which row holds the chord
(`hotkey-taken`). Two actions today — the main window and the panel —
and the shipped chord is gated against colliding with a future third
before anybody can record it.

## Platforms

macOS and Linux under X11 outside GNOME, through `global-hotkey` 0.7 — the crate
Tauri's own plugin is built on, a sibling of the `muda` the tray uses,
and a dependency on exactly the platforms `tray-icon` is one. On macOS
it registers through Carbon's `RegisterEventHotKey`, which needs no
Accessibility permission (lazy-shot's CGEventTap path exists for
chords Carbon cannot express; nothing here needs it).

**D346 — Linux: an X11 key grab, asked only in an X11 session.** On
Linux `global-hotkey` starts a thread with its own X11 connection and
grabs the chord on the root window (with and without NumLock and
CapsLock). `hotkey::install(compositor)` asks for it only when
`hotkey::x11_session` says so: GPUI draws through X11
(`App::compositor_name() == "X11"`) *and* `XDG_SESSION_TYPE` is not
`wayland`. The second half is XWayland — a grab there hears only the
keys typed into other X11 clients, a shortcut that works in one
terminal and nowhere else. And it is asked *before* the manager exists
because the crate's thread dies quietly when it cannot connect, after
which every `register` answers Ok: a chord the row would call active
and nothing would deliver. A grab another client holds is refused by
the X server (`BadAccess`), and the row shows it. On GNOME the shipped
panel chord, Ctrl+Alt+D, is one of the keys bound to *Show desktop*
(`org.gnome.desktop.wm.keybindings show-desktop`), so expect it to be
refused there and record another — the ordinary case the line under
the field exists for. A registration waits on the grab thread, which
looks at its inbox every 50 ms, and then on the X server's round trips —
so it is never asked from the GPUI thread (B1, 2026-10-08). On Linux
the manager moves into a registrar thread of ours (`wipemark-hotkeys`,
`Registrar::threaded`), fed by a `flume` channel: `Registrar::ask`
returns at once, the thread performs the requests one at a time in the
order they were asked — so the old chord is still released before the
new one is asked for — and each answer comes back over
`Registrar::answers`, which `main::install_hotkeys` polls from
`cx.spawn` and writes to the page with
`Preferences::shortcut_registered`. Until an answer comes the row keeps
the one before it. macOS keeps asking on the main thread
(`Road::Here`): Carbon's `RegisterEventHotKey` answers at once and
belongs there, and its manager is not `Send`; its answers take the same
channel, so `main` has one road for both. The ordering and the refusal
are tested over a desktop double that holds every request
(`asking_for_a_shortcut_never_waits_for_the_desktop`,
`the_registrar_releases_the_old_chord_first_and_answers_in_order`).

Windows wants a message loop on the registering thread (E10), and a
Wayland session that is not GNOME's has no API this build uses; there
`hotkey::install` returns `None` and every row says the shortcut is
stored and not registered (`hotkey-unavailable`,
`hotkey-unavailable-wayland`), because a preference that silently did
nothing would be worse than one that says so.

### GNOME: its own custom keyboard shortcut (D641–D647)

Acceptance F11 (2026-10-12, Ubuntu 24.04, GNOME Shell 46 on Wayland): the
panel could not be summoned from the keyboard. Under XWayland — the
default on Linux since D611 — an X grab hears only keys typed into other
X11 clients, and GNOME 46 has no `org.freedesktop.portal.GlobalShortcuts`
(it came in 48). GNOME has always had the other road, the one its own
Settings › Keyboard › Custom Shortcuts writes, and on GNOME — **either
session type**, `hotkey::road`, `XDG_CURRENT_DESKTOP` naming `GNOME` —
that is the registrar (`gnome::Keybindings`, on the `wipemark-hotkeys`
thread):

* **One entry per action with a chord**, at
  `/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/wipemark-panel/`
  (and `wipemark-show/`): `name` from the catalogue (`hotkey-gnome-name-*`,
  plain text — "Wipemark — panel"), `binding` in GNOME's accelerator
  spelling (`gnome::accelerator`: `CmdOrCtrl+Alt+D` → `<Control><Alt>d`,
  D642), and `command`.
* **The command** is the program that would start us, then the action's
  flag: `$APPIMAGE` inside an AppImage, `current_exe()` otherwise, behind
  `env WIPEMARK_DATA_DIR=<dir>` when that variable is set, quoted for
  `g_shell_parse_argv` where it must be (D643). `--panel`, and `--show` for
  the main window.
* **The list** (`org.gnome.settings-daemon.plugins.media-keys
  custom-keybindings`) is edited, never replaced: ours appended after
  everyone else's, ours alone taken out, our keys reset (D644).
* **A chord GNOME already grabs is refused by name** (D645): the window
  manager's, the shell's, mutter's, the media keys' and every other custom
  shortcut's accelerators are read first and compared by what they grab;
  a match is `Registration::TakenInGnome { name, setting }` — "Not set:
  GNOME already uses these keys for "Hide all normal windows"
  (org.gnome.desktop.wm.keybindings show-desktop)" — and nothing of ours
  is left on the old keys. On Ubuntu that is the shipped Ctrl+Alt+D.
* **Every launch asserts it again** — the command may name an AppImage
  that has moved — and **a quit leaves it**, so the keys start Wipemark
  with its panel. Uninstalling leaves the entry in the person's GNOME
  settings: they remove it in Settings › Keyboard › Custom Shortcuts (a
  `.deb`'s `postrm` cannot reach a user's dconf). The sentence under the
  field says both.
* **The `gsettings` command**, each call bounded at 5 s, never from the
  GPUI thread. No dependency: gio is in the lock through gtk, but a process
  call is what a test double stands in for — every rule above is tested
  over `gnome::tests::Fake`, never a real `gsettings`. No GNOME custom
  shortcuts at all (no `gsettings`, no media-keys schema) is
  `Registration::NoGnome` (`hotkey-unavailable-gnome`).

**A press is a second launch.** GNOME runs `wipemark --panel`; a launch
whose whole request is `--panel` or `--show` first connects to
`<data dir>/wipemark.sock` (`instance.rs`, D646) — mode 0600, or
`$XDG_RUNTIME_DIR/wipemark-<hash>.sock` when the data directory's path is
too long for a socket — and, answered `ok`, exits 0: the running instance
toggles its panel (or brings the main window forward), the way the
Carbon or X11 press does. With nothing running the launch starts, and its
`--panel` opens the panel. Not MCP, which is off by default. The same
holds on macOS and for every other second launch: see "One Wipemark per
data directory" below.

**Why GNOME's road under X11 too** (D647): a grab of a key mutter already
holds is refused with a bare `BadAccess`, where GNOME's lists let the row
name the holder, and only GNOME's shortcut starts Wipemark when it is not
running — so the two session types behave alike. The X11 grab stays for
X11 sessions that are not GNOME's; macOS is unchanged.

The live check is `scripts/verify/gnome-hotkey/private-gsettings.sh`: a
private session bus, a private dconf (`XDG_CONFIG_HOME`,
`XDG_RUNTIME_DIR`), an Xvfb, the real `gsettings` and the real
application — the entry written, the stranger's kept, Ctrl+Alt+D refused
as Show desktop's, `--panel` handed over in ~20 ms, the binding left at
quit, the recorder's order driven with XTest — and the owner's own
`dconf dump` of the media keys and the window manager's keybindings
compared before and after.

## One Wipemark per data directory (D646, D790–D794)

A data directory has one application process, whoever starts the
second. Two of them on one `wipemark.db` would each read the batch
queue's rows at launch and each run its next job, each write the
journal and each serve MCP — on macOS `wipemark --panel` did exactly
that, and both processes rewrote the same queued document (acceptance
F33).

**The hold** (D791). Before the database, the queue, the journal, the
MCP server or a window, `main` asks `instance::hold` for an exclusive,
non-blocking lock (`flock`, through `File::try_lock`) on
`<data dir>/wipemark.lock`, mode 0600. Held, the file says the process's
pid, and the lock is kept for the life of the process
(`Held::for_the_life_of_the_process`): the kernel lets go however the
process ends, `kill -9` and a crash included, so there is never a stale
lock to clear. The file is never removed — a lock file removed while
another process opens it makes two holders. The lock is the file's, not
the path's, so two spellings of one directory are still one holder. The
command line never takes it.

**The knock** (D646, D790, D793). `instance::admit` decides, in this
order:

1. A launch whose whole request is a summons — `--panel`, `--show`, or
   **nothing at all** — connects to the socket first. Answered `ok`, it
   exits 0 before anything else opens.
2. Otherwise it asks for the hold. Held, it binds the socket at once,
   before the database (D793), so a summons that arrives while it starts
   is answered and waits on the channel until the main window is up;
   then it starts.
3. Taken, it knocks again every 100 ms for up to five seconds
   (`instance::PATIENCE`), handing over its summons — or `show`, for a
   launch that asked for more. A holder that answers ends the launch
   (D792); a holder that lets go — a quit still unloading a model — lets
   it start.

**Each kind of second launch** (D792), on a data directory another
process holds:

| launch | what happens | exit |
|---|---|---|
| `--version` | prints `wipemark <version>`; never asks for anything | 0 |
| `--panel` | the holder toggles its panel | 0 |
| `--show`, or no flag at all (a second double-click, LaunchServices' `-psn_…`) | the holder brings its main window forward | 0 |
| `--import=`, `--clean=`, `--compare=`, `--settings[=…]`, `--profile=`, `--setup`, with or without a summons | the holder brings its main window forward; the launch says on stderr that Wipemark is already running and that these are taken only by the launch that starts it (`launch-already-running`) | 2 |
| any, while the holder does not answer for five seconds | `launch-not-answering` on stderr | 2 |
| any, with no data directory, or a lock the filesystem refuses | starts, as before D791; a warning in the log (D794) | — |

The sentence is in the person's language, read from the holder's
database without writing to it; the log line beside it is for a machine.
The files a second launch names are not handed over: a wire format for
paths would be a protocol for flags that exist for checks, and
LaunchServices drops a running application's `--args` anyway.

**On macOS** (D790) the handover is the same as on Linux. LaunchServices
keeps one instance only for a launch that goes through it — the Finder,
the Dock, `open -a Wipemark` — so a second double-click never reaches
this code. The bundle's own executable started by name
(`Wipemark.app/Contents/MacOS/wipemark --panel`, from a terminal, a
script, a launcher, or `open -n`) is a second process, and it is the one
that knocks. A socket's path is held to 103 bytes there (`sun_path` is
104), 107 on Linux; past that, the socket goes to the per-user `$TMPDIR`
on macOS and `$XDG_RUNTIME_DIR` on Linux, as `wipemark-<fnv of the data
directory>.sock` (`instance::runtime_dir`, `socket_path`).

**The command line** is unchanged (D794): it finds the application by its
beacon (`mcp.json`, loopback only) and lets the application's queue run a
rewrite; alone, it loads its own model and writes only its own journal
row, through SQLite. It never runs the queue, so it never asks for the
hold.

Tests: `instance::tests` — `one_process_holds_a_data_directory`,
`a_second_launch_hands_over_or_ends_and_never_starts`,
`a_launch_waits_for_a_holder_that_is_letting_go`,
`a_socket_that_does_not_fit_beside_goes_to_the_runtime_directory`; in
`main`, `only_a_launch_that_only_summons_is_handed_over`; and
`tests/second_launch.rs`, which runs the built binary on a data
directory the test holds — `--panel`, `--show` and a bare launch handed
over with exit 0, `--import=` refused with exit 2 — and kills it, failing,
should it start instead. The live check is
`scripts/verify/f33/second-launch.py`: a real instance on a scratch data
directory whose socket path is 104 bytes, then `--panel` twice, a bare
launch and `--import=`, each exit code, what the instance heard, and
whether its panel is on screen.
