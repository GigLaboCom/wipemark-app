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
// gpui_macos::window, in the pinned rev 81b16f464c
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
| cleaning anything | Layer A is E1, images are E11 |
| expanding folders and archives | needs its own rule — traversal, symlinks, a limit on how many files, cancellation |
| paste by key (⌘V) | the toolbar's **Paste** button reads the same `Handed` off the same `NSPasteboard` (`pasteboard::from_clipboard`, `crate::clipboard`) and lands it in the queue; a key for it has to wait until it can tell a paste into the filter bar's fields from a paste into the queue |
| Linux, Windows | E10; until then `drop::zone` offers what GPUI itself does, which is files |
| two zones in one window | the delivery carries the position, `Catcher` does not pass it on yet; the main window's table is most of the window, so one zone is the right number there |
| a limit on how much can be dropped at once | nothing bounds it today except the four-kilobyte head on each read |

## The spec

The implementation spec for all of the above is
`wipemark-intake-drag-and-drop-2026-09-11` in Watchword (FILE, ttl 0).
It records the deviations from the original decomposition — the extra
crate, and drag & drop landing in E6 rather than in E7's S7.1 — and the
open questions this document lists above.
