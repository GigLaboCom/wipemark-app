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

Click the field and it listens. Hold modifiers and it previews them
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

## The row is the request; the registration is the answer

`hotkey::Registration` is what the desktop said — `Registered`,
`Refused(reason)`, `Unavailable` (no registrar on this platform), or
`Unset` — and the row shows it under the field, for the reason the MCP
page shows both the port that was asked for and the one the server
took: a chord the OS refused is invisible from the preference alone,
and a user who has recorded one deserves to know before they press it
from another application and nothing happens.

`main::install_hotkeys` owns the registrar. It registers what is
stored, observes `Preferences` and re-registers only when a chord has
actually moved (the old chord is released *first*, whatever the new one
turns out to be — a change of mind that left the previous chord
registered would fire on a key the row no longer shows), and writes the
answer back with `Preferences::shortcut_registered`. A press comes back
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

macOS today, through `global-hotkey` 0.7 — the crate Tauri's own
plugin is built on, a sibling of the `muda` the tray uses, and a
macOS-only dependency for the same reason `tray-icon` is one. It
registers through Carbon's `RegisterEventHotKey`, which needs no
Accessibility permission (lazy-shot's CGEventTap path exists for
chords Carbon cannot express; nothing here needs it). Linux wants an
X11 connection and Windows a message loop on the registering thread;
both are E10 work beside the tray's. Until then `hotkey::install`
returns `None` and every row says the shortcut is stored and not
registered (`hotkey-unavailable`), because a preference that silently
did nothing would be worse than one that says so.
