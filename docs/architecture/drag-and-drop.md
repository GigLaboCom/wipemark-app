# Drag and drop, and what was dropped

A window that can be dropped on is two separate problems, and this
repository keeps them in two separate places on purpose:

1. **Accepting** — persuading the desktop to offer the drag at all, and
   turning what it hands over into values. That is
   `apps/wipemark-app/src/pasteboard.rs` (the AppKit half) and
   `apps/wipemark-app/src/drop.rs` (the GPUI half).
2. **Recognising** — deciding what the thing actually *is*. That is
   `crates/wipemark-intake`, a leaf library with no dependencies at all,
   because the panel is not the only surface that will ask.

Epic **E6**. Nothing here cleans anything: E1 is Layer A and E11 is
images, and every surface says so out loud rather than implying
otherwise.

## GPUI accepts files and nothing else

This is the fact the whole macOS half of this feature exists for.

GPUI's macOS window registers for exactly one pasteboard type:

```rust
// gpui_macos::window, in the pinned rev 81b16f464c — and unchanged in
// gpui-pre-macos 0.3.8 (src/window.rs:1077), pinned since 2026-10-07
let () = msg_send![
    native_window,
    registerForDraggedTypes:
        NSArray::arrayWithObject(nil, NSFilenamesPboardType)
];
```

and its `draggingEntered:` answers `NSDragOperationNone` unless the
pasteboard carries filenames. So a drag of **text** or of an **image**
is refused by AppKit before any GPUI event exists. There is no handler
to add, no element to hang an `on_drop` on, and no arrangement of the
view tree that can see it: the drag never becomes one.

Two things follow, and both are load-bearing:

* The vendored `gpui-component` submodule is **not** where this could be
  fixed either, and neither is a patch to GPUI: a submodule edit cannot
  be committed from the parent repository, and a fork of GPUI is a
  maintenance cost paid on every rev bump.
* What a drag needs is a *dragging destination* registered for the types
  in question. `pasteboard.rs` is one.

## Where the destination sits

A window resolves a drag by hit-testing its view tree for the deepest
view under the pointer, then walking **up** the superview chain until it
finds one registered for a type on the pasteboard. Only if none is found
does the window itself answer — which is how GPUI's window gets file
drops today.

So the destination cannot be a sibling laid over GPUI's view: it would
take every click with it, and a `hitTest:` that handed the mouse back
would take it out of the search as well. `pasteboard::attach` therefore
inserts `WipemarkDropView` as GPUI's view's **parent**:

```
NSWindow
└── contentView
    └── WipemarkDropView        ← registered for files, images, text
        └── GPUI's view         ← still the frontmost thing under the pointer
```

Clicks, scrolls and the window-background drag land on GPUI's view
exactly as before. A drag walks past it to ours.

Because that view is registered for file URLs *as well*, it answers
**every** drop rather than sharing the job with the window behind it —
one path and one set of rules, whatever was dropped. GPUI's own
`ExternalPaths` drop is still wired up in `drop::zone`, and on macOS it
simply never fires; it is what the window falls back to if the
destination could not be installed, and it is the whole story on the
platforms E10 will bring.

The callback has no `&mut App` in scope and no way to get one, which is
the same problem the menu-bar item and the global shortcut have, and it
gets the same answer: a `flume` channel read from `cx.spawn`. The
registry is keyed by the `NSWindow` pointer, which is the only identity
both halves can see.

## On Linux: a drop before the file list, and the carried fix

Off macOS the `ExternalPaths` road is the only one, and GPUI's Linux
backend fills it. Until 2026-10-12 it lost a quick drop — acceptance F9:
a file dragged onto a window was often lost on the first try and landed
only on the second. The cause is in `gpui-pre-linux 0.3.8`, in both
backends, and the fix is carried in this repository as
`vendor/gpui-pre-linux` with `vendor/gpui-pre-linux.patch` (D620–D623;
how it is carried and checked is `gpui-pin.md` §2).

**X11** (`src/linux/x11/client.rs`, the published lines). A drag's file
list is a selection. `XdndPosition` asks for it with `convert_selection`
(`:951–981`, on every position until it has arrived), and GPUI's drag — the `FileDropEvent::Entered` that makes
`cx.active_drag` — is created only when `SelectionNotify` brings the list
back (`:997–1035`). An `XdndDrop` that arrives first (`:983–995`) sends
`XdndFinished` and `Submit` with no drag to drop, which is nothing, and
resets the state; the late `SelectionNotify` then emits `Entered` into a
drag that has already ended, and GPUI's active drag stays behind. That is
why the second try worked — and why it dropped what the *first* had
carried, since GPUI's `Entered` does not replace an active drag
(`gpui-pre 0.3.8`, `window.rs:5521–5532`).

The fix, in `Xdnd` (`defer_drop`, `receive_paths`): a drop that comes
while the list asked for has not arrived is remembered, and `XdndFinished`
and `Submit` wait for `SelectionNotify`, which then emits `Entered`
followed by `Submit` — GPUI turns the two into a mouse move and a mouse
up at the drop's position, and the zone's `on_drop` fires. A
`SelectionNotify` that nobody is waiting for — after a leave, after a
drop already finished, before any request — is ignored rather than
turned into a drag. A deferred drop whose answer carries no list still
sends `XdndFinished` and clears the state, with no drag made. An
`XdndEnter` starts from a clean state, so a later drag inherits nothing.

**Wayland** (`src/linux/wayland/client.rs`). The URI list is read from a
pipe on the background executor, and the drag's offer and window are
installed only when that read completes (`complete_uri_read`, `:480`;
the read is spawned by `wl_data_device.enter`, `:2803`). A
`wl_data_device.drop` before then (`:2905`) reached `handle_drop`
(`:464`), which invalidated the read first and then found no offer and
returned — the drop was gone, and so was the read that would have
brought it. Upstream fixed the sibling
race, a *leave* before the read (zed #64122, "Discard stale Wayland file
drag reads", merged 2026-09-12): every read carries a generation, a new
enter or a leave invalidates the pending one, and a stale completion
destroys its offer and makes no drag. The carried fix keeps every one of
those invariants and changes only the drop: a drop while the current read
is pending is recorded with its generation and position; a leave after it
(GNOME sends one) does not invalidate the read; and when the read
completes, `finish_pending_drop` delivers `Entered`, then `Submit`, then
the offer's `finish()` and `destroy()`. A new enter abandons a recorded
drop, and the old read's completion is stale as before. Motion while the
read is pending now updates the position, so the drop lands where the
pointer was. Upstream's test `drop_then_leave_discards_pending_drag_uri_read`
asserted the lost drop and is replaced by
`drop_then_leave_before_uri_read_completes_delivers_the_drop`.

Zed `main` still has both losses as of 2026-10-12 — X11's `XdndDrop` and
Wayland's `handle_drop` unchanged — and there is no upstream issue for
it. The owner's decision is to carry the fix here only: no fork, no pull
request.

### A dragged text, handed over as a drop file

Until 2026-10-12 a **text** dragged out of a browser or an editor — not a
file — was taken by no window on Linux (acceptance F15). The published
backends read `text/uri-list` and nothing else: X11 took the first of the
source's types it knew and parsed whatever came back as URIs, so a text
became no paths; Wayland asked the offer for `text/uri-list` alone and
destroyed a drag that brought none ("Prevent dropping text from other
programs"). And GPUI's `FileDropEvent::Entered` carries `ExternalPaths`
and nothing else, in `gpui-pre`, whose public types are not ours to
change.

So the carried crate hands a text over **as a file of its own**, and the
application reads it back as the text it is (D653–D659):

1. **The type** (D657). Both backends take, in this order,
   `text/uri-list`, `text/plain;charset=utf-8`, `UTF8_STRING`,
   `text/plain` — X11 then `STRING` and `TEXT`, which it always accepted —
   from the three types in `XdndEnter` or, past three, the source's
   `XdndTypeList`, and on Wayland from the offer's
   types — by name since D696 (`platform::drag_type_rank`) — the chosen one `accept`ed at enter, which the
   compositor needs before it sends a drop. A file therefore beats the
   text describing it, as on macOS. A URI list with no `file:` URI in it —
   a link dragged out of a page — is taken as the text of its URIs.
2. **The read** (D658). The text is read where the file list was —
   `SelectionNotify` on X11, the pipe on Wayland — and held in memory with
   the drag (`DroppedText`): at most 1 MiB, the MCP body's limit, UTF-8,
   not empty, `\r\n` folded to `\n` as GPUI's paste does. X11 reads the
   property up to that limit (it read 4 KiB, which also truncated a long
   file list) and refuses what is not 8-bit or goes past it — an `INCR`
   transfer included. A refusal is a WARN line and no drag.
3. **The file** (D653). `Entered` names a path in a private directory —
   `$XDG_RUNTIME_DIR/wipemark-dropped-text/`, else
   `<temp dir>/wipemark-dropped-text-<uid>/`, created 0700 and refused when
   it is a link, someone else's, or open to others — as `<pid>-<ns>-<n>.txt`.
   The file is written, 0600 and `create_new`, **only at the drop**, just
   before `Submit` (in a deferred drop as well, D622/D623): a drag that
   leaves writes nothing, and a new enter forgets the text.
4. **The landing** (D654, D655). `drop::handed_of` sees a path whose
   parent is that directory, reads it as UTF-8, removes it at once, and
   hands over `Handed::Text` — a paste: no file behind it, `Source::Text`,
   the Retention page's rules for kept copies. A text that is itself a
   path or a `file://` URI then goes the ordinary way, `of_text` and
   `name::path_in`. The read is on the drawing thread, in the drop's
   handler: at most 1 MiB, in a private directory, normally on `tmpfs`,
   written a moment before.
5. **What nobody took** (D656). A drop file lives for the dispatch of one
   drop. One a minute old — a text let go where no zone was, or a run that
   ended between the write and the read — is removed when a window starts
   accepting drops (`drop::accept`, on a thread of its own).

The directory's rule is written twice — `dropped_text_dir` in the carried
crate's `platform.rs` and `drop::dropped_text::dir` — because the
application does not depend on the carried crate;
`scripts/verify/linux-drop/quick-drop.sh` (steps e–i) is what holds the
two together. An image dragged as data (`image/png` from a browser) is
not taken yet — stage two in the report
(`docs/plan/reports/linux-text-drop-2026-10-12.md`).

### Under XWayland: a refused type, mutter's leave, and the position

On GNOME on Wayland the application draws through XWayland (D611), and a
drag out of a Wayland-native application reaches it through **mutter**,
not through Xwayland: `hw/xwayland` has no selection or DnD bridge, the
compositor's X window manager is the bridge (`src/wayland/meta-xwayland-dnd.c`,
`src/x11/meta-x11-selection.c`, read at mutter `gnome-46`). What mutter does
decides three rules here (acceptance F15b, D696–D701):

- **Its types are the Wayland source's mime types, by name.** `XdndEnter`
  carries them (three inline, more in `XdndTypeList` on mutter's selection
  window); a `SelectionRequest` is served when the target's name *equals*
  one of them, or is `UTF8_STRING` for `text/plain;charset=utf-8` or
  `STRING` for `text/plain`; `TARGETS` lists the mime types with those two
  aliases; anything else is refused with property None. So the carried
  crate chooses by name, case-insensitively and with the charset's
  spelling folded (D696), falls back through every acceptable offered type
  and then `TARGETS` when one is refused, asks for each at most once, and
  honours the notify's property, deleting `XDND_DATA` as it reads it (D697);
  with nothing left, the drop is finished *not accepted* (D698).
- **The data comes late, and mutter leaves right after the drop.** A
  served request is filled from the Wayland source asynchronously
  (`wl_data_source.send`), and mutter ends its grab — sending `XdndLeave`
  — as soon as it has sent `XdndDrop`. A drop deferred until its data came
  (D622) was cleared by that leave; it is now kept until the data arrives
  (D699), as D623 keeps Wayland's.
- **The X pointer is stale.** While a Wayland client drags, the X server's
  pointer is where it last was over an X window, so GPUI's `QueryPointer`
  placed a drop anywhere (the owner's picture landed at −2025,−5). The
  position is now `XdndPosition`'s own root coordinates, translated to the
  window; `QueryPointer` only when the message has none (D700).

Why the owner's source refused *every* conversion of the type mutter
itself listed is not established: by mutter's code a listed type is
served, and property None for every request is also what the X server
answers when `XdndSelection` has no owner. Each refusal now logs the
selection's owner, so the next report says which. `quick-drop.sh` steps
j–o hold the rules on Xvfb with a source that refuses, lists in
`TARGETS`, leaves after the drop and leaves the pointer far away (D701).

### Seeing a drop in the log

Every drop that reaches a window is one line of ours, from `drop.rs`
(target `wipemark::drop`): `a drop arrived road="external-paths"
things=1` at INFO — `road="pasteboard"` on macOS's own destination — and
`a drop carried nothing road=… items=…` at WARN. Counts only: no path,
no text (`logging.md`). These are at INFO and are in every log.

GPUI's side of a drag is DEBUG lines in the carried crate (target
`gpui_linux::linux::x11::client` or `…::wayland::client`), one per
protocol step and never per mouse move: on X11 `XdndEnter from 0x…:
offered [text/plain;charset=utf-8, UTF8_STRING, …], taking <name>` (or
`no supported type`), `XdndPosition: requested <name>` (once per type),
`SelectionNotify: <name> gave N paths, Enter` (or `gave 1 text, …`) or
`SelectionNotify: <name> was refused (property None; selection owner
0x…); requesting <next>` — then `TARGETS lists [...]`, `no other type to
ask for` or `the deferred drop finished, not accepted` — `XdndDrop at
Point { … } (root Some((x, y))) on 0x…` or `XdndDrop: deferred until the
file list arrives` then `XdndDrop: the deferred drop completed at …`,
`XdndLeave after a deferred drop: kept until its data arrives`,
`XdndLeave`; on Wayland `drag enter: offered [...], accepting and reading
<mime>`, `drag enter: read N paths` or `read text`, `drag drop at …` or
`drag drop: deferred until the file list arrives` then `drag drop: the
deferred drop completed`, `drag leave`. A text adds `dragged text read: N
bytes` and, at the drop, `dragged text written to its drop file: N
bytes` (target `gpui_linux::linux::platform`); ours adds `a dragged text
was read from its drop file bytes=N` (DEBUG). The drop's position and
window are said at `Submit`, so a drop that reached GPUI and landed in no
zone is told from one never delivered. They are off by default — the default
filter is `info` with `debug` for our own crates only — and are turned on
with

```sh
WIPEMARK_LOG=info,gpui_linux=debug wipemark
```

`WIPEMARK_LOG=debug` shows them too, buried under the renderer's. A drop
that shows GPUI's lines and none of ours was lost between GPUI and the
zone; one that shows neither never reached the window.
`scripts/verify/linux-drop/quick-drop.sh` drives all fifteen cases on a
virtual display and prints both kinds of line.

## What arrives, and in what order it is read

One `Handed` per pasteboard **item**, because that is how a desktop
counts a multiple selection. The order the types are tried in is the
whole of the policy, and it is "the most specific thing the sender
bothered to attach":

| tried | beats | because |
|---|---|---|
| file URL | everything | Finder attaches the path as text as well, and scrubbing the *name* of a document instead of the document is the failure this order exists to prevent |
| PNG, TIFF | text | a screenshot dragged out of a chat window carries a caption nobody meant to hand over |
| plain text | RTF, HTML | this product's subject is characters; the markup around them is a container, and it is still read when it is all there is |

An item whose text is empty, or ASCII white space alone, hands over
nothing (D301, `Handed::is_nothing`; [queue.md](queue.md), "How things
get in"), and `Catcher::land` leaves such a thing out whichever road it
came by — a drop of it is a drop that carried nothing.

`a_file_beats_the_text_that_describes_it` and
`an_image_beats_the_caption_attached_to_it` are the gates, and they run
against a real `NSPasteboard` — `pasteboardWithUniqueName`, filled by
the test — rather than a mock, so what they check is what AppKit will
actually hand over.

## Recognising it: the bytes decide, the name may refine

`crates/wipemark-intake` is the Tika-shaped half: a signature table
(`magic.rs`), an extension table (`name.rs`), an encoding verdict
(`text.rs`), and the arbitration between them (`lib.rs`).

Two sources of evidence, and they are not equals:

| case | answer | `Evidence` |
|---|---|---|
| the bytes place it | the bytes | `Content` |
| the name lands *underneath* what the bytes could see | the name | `Agreed` |
| both say the same thing | either | `Agreed` |
| they point different ways | the bytes, and the name is **kept** | `Disagreed { name_said }` |
| the bytes place it nowhere | the name | `Name` |
| neither establishes anything | nothing | `Nothing` |

The second row is what `Format::refines` exists for. The head of a DOCX
can only reach "an Office Open XML file", because the central directory
that says *which* one is at the far end of a file this crate deliberately
only holds the front of — so the name completes the sentence rather than
contradicting it. Every DOCX is an Office file is a ZIP; every Markdown
file is text. Two formats side by side never refine each other, which is
what keeps the relation from becoming a way for any name to overrule any
signature.

The fourth row is the one a person most needs. A `holiday.txt` that
begins `89 50 4E 47` is a PNG, the report says PNG, and it also says the
name claimed otherwise — the panel paints that line in the warning
colour, and it is the only line there that is not muted.

Those six rows line up with the three shelves every report in this
product has: content is *verifiable*, a name is *best-effort*, and
`Nothing` is *not established*.

### The name that is the whole message

Text dropped from a terminal, a file manager or a chat window is
frequently not text — it is a **path**, in characters. Scrubbing the
nineteen characters of `/Users/me/report.docx` instead of the document
they name would be the wrong file, silently. `intake::of_text` therefore
looks for a path *that exists* before it accepts a line as prose, and
`name::path_in` is deliberately narrow about what counts: one line, and
either a `file:` URL or something that begins at the root or at a home
directory. The cost of being wrong is asymmetric — reading a document
that happens to mention `notes.txt` as a file loses the document, while
failing to notice a path only means somebody drops the file itself
instead.

`Intake::arrived` keeps the fact that it came as text, because a surface
may want to say so, and because "you dropped a file" about a line of
characters is a small lie.

The rule is for a person's drop or paste, where a path in characters is
how a desktop hands a file over. A text that is the document whatever it
says — an MCP caller's `text`, which the tool cleans or rewrites as the
characters it is — is `intake::of_characters`: what `of_text` says of a
text that names no file, with no path looked for. A path in a caller's
text names a file in the caller's world, and its `/dev/fd/3` would be this
process's own descriptor here (D751).

### What is deliberately not claimed

* **No format is guessed.** Most of the formats in the world are not in
  `magic.rs`, and an unrecognised binary is `Kind::Unknown` with no
  format at all. `infer` and `tree_magic_mini` know more formats; what
  they do not do is say *how much* they saw, and a table this crate
  cannot see into is a table whose answers it cannot explain in a report.
* **No code page is named.** Text that is not UTF-8, UTF-16 or UTF-32 is
  `Encoding::Other` — text this crate can see is text and cannot name.
  Guessing Windows-1251 from letter frequencies is a different product.
* **Only the front is read.** `HEAD` is four kilobytes. A dropped model
  file is gigabytes, and reading one to say "that is a model file" would
  freeze the window it was dropped on. `of_path` is blocking and says so;
  `drop::Catcher` calls it on the background executor.

## The panel, today

Three states, and they are three different answers:

* nothing has been dropped here yet — the window says what it takes;
* a drop arrived and carried nothing this machine could read — which is
  not the same as the first, and showing the invitation again would look
  like the drop never happened;
* something arrived, and the panel lists what it turned out to be, four
  rows at a time with the rest counted.

`Catcher::caught` is `Option<&[Arrival]>` for exactly that reason: absent
and empty are two different answers, the same distinction the hotkey
rows make between a row that was never written and a row that was
cleared. An `Arrival` is the `Handed` beside the `Intake` it became —
the thing and its description — because the main window's queue draws
a preview of the thing, and a description cannot be previewed.

## The main window, today

The queue (`apps/wipemark-app/src/queue.rs`, and [queue.md](queue.md))
is the second window that takes a drop, and it takes it differently:
the panel shows the *last* drop, the queue keeps *every* one. Both are
served by the same `Catcher`, through two different halves —
`caught()` is the last drop, overtaken by the next the way a screen is;
the `Landed` event carries its own drop, and every drop fires one, so a
second drop while the first is still being read off a slow disk does
not lose the first from a list. The Import button, the Paste button and
`--import=<path>` go down the same road one step later, through
`Catcher::land`: a file chosen, a thing pasted and a thing dropped are
recognised by one piece of code and land as one kind of event. Paste
reads the general pasteboard with the very function a drop is read
with, so it cannot arbitrate a file against its name, or an image
against its caption, any differently — and its label is built from a
peek at the item *types* (`pasteboard::held`), polled through the
change count, so the button says "Paste image" before it is pressed and
copies the bytes only when it is.

## How it is consumed

Nothing above is the panel's. Two halves, and a surface takes whichever
it needs: a **window** takes both, a **pipe** takes only the second.

### A window

```rust
// while the view is being built
let catcher = cx.new(|cx| Catcher::new(window, cx));
drop::accept(window);                      // installs the platform destination

// in render
drop::zone(v_flex(), &self.catcher, cx)    // highlight and handlers

// when something lands
self.catcher.read(cx).caught()             // Option<&[Arrival]> — the last drop
cx.subscribe(&catcher, |_, _, Landed(arrivals), cx| { … })   // every drop

// something chosen rather than dragged: the same road, one step later
catcher.update(cx, |catcher, cx| catcher.land(handed, cx));
```

Three rules come with that:

* **`caught()` is an `Option`, and both answers matter.** `None` is
  "nothing has been dropped here yet"; `Some(&[])` is "a drop arrived and
  carried nothing this machine could read". A surface that renders them
  the same way is telling somebody their drop did not happen. It is the
  same distinction a preference row makes between never written and
  deliberately cleared.
* **One `Catcher` per window.** The platform destination is keyed by the
  window, so a second catcher would hear the same drop twice and neither
  would know which half of the window it was over. Two zones in one
  window needs the drop *position*, which the delivery carries and the
  entity does not yet — see the open questions below.
* **`Landed` is for surfaces that must act**, and it carries the drop it
  is about; one that only paints the last drop observes the entity
  instead. Both fire from the same `cx.notify()`. A read overtaken by a
  later drop still fires its event and only loses the right to be
  `caught()` — two readers, two questions.

### A pipe

The CLI and the MCP server skip everything above and call
`wipemark_intake` directly — an argument that is a path goes to
`of_path`, a `-` or a blob goes to `of_bytes`. That is the whole reason
the crate is a leaf with no dependencies: neither of them can afford to
inherit GPUI, AppKit or a language.

### What each consumer takes from an `Intake`

| consumer | what it reads | note |
|---|---|---|
| the panel (E6) | everything, to say what arrived | the last drop, four rows at a time |
| the queue (E6, the listing half of E7's S7.1) | everything the panel reads, plus the `Handed` beside it for a preview — `kind` and `format` to decide whether there is one, `encoding` to decode it, `size` to refuse a picture too big to thumbnail | `Folder` and `Archive` are listed as what they are and **not** expanded until there is a rule for expanding them — an item that silently became four hundred rows is not what anybody dropped |
| Layer A (E1) | `is_textual()` and `encoding` | what to decode, and with what |
| images (E11) | `Format` → `ImageContainer` | the mapping lives at the surface; the crate names formats and opens nothing |
| the pipeline (E4) | `Kind`/`Format` | which parser, which tactic |
| the CLI (E5), MCP | `kind`, `format`, `evidence` | into `--json` and into a tool result |

`duty` reads none of it: who rewrites does not depend on what was
dropped.

### What not to do with an `Intake`

* Do not treat `Kind::Text` as permission to read the whole file into
  memory. The size is right there, and the limit is the surface's call.
* Do not treat `format` as proof — read `evidence`. `Name` means the
  bytes said nothing, and a consumer that unpacks a ZIP on that basis
  will unpack anything somebody renamed.
* Do not carry an `Intake` between windows and assume `path` still
  resolves. The file can be gone between the drop and the work; open it
  again and handle the failure.
* Do not lose `Evidence::Disagreed` on the way to a report. It is the
  same fact the panel paints in the warning colour, and it is worth
  exactly as much three layers later.

## What is deliberately not here

Accepting and naming is the whole scope. Not in it, and each for a
stated reason:

| absent | why |
|---|---|
| cleaning anything | not this document's job: what was dropped is cleaned by `clean.rs` when somebody presses Clean (E7) — see `docs/architecture/queue.md`, "Cleaning", and `retention.md` |
| expanding folders and archives | needs its own rule — traversal, symlinks, a limit on how many files, cancellation |
| paste by key (⌘V) | the toolbar's **Paste** button reads the same `Handed` off the same `NSPasteboard` (`pasteboard::from_clipboard`, `crate::clipboard`) and lands it in the queue; a key for it has to wait until it can tell a paste into the filter bar's fields from a paste into the queue |
| text and images on Linux, anything on Windows | E10; until then `drop::zone` offers what GPUI itself does, which is files — on Linux with the carried fix above |
| two zones in one window | the delivery carries the position, `Catcher` does not pass it on yet; the main window's table is most of the window, so one zone is the right number there |
| a limit on how much can be dropped at once | nothing bounds it today except the four-kilobyte head on each read |

## The spec

The implementation spec for all of the above is
`wipemark-intake-drag-and-drop-2026-09-11` in Watchword (FILE, ttl 0).
It records the deviations from the original decomposition — the extra
crate, and drag & drop landing in E6 rather than in E7's S7.1 — and the
open questions this document lists above.
