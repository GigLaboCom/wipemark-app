# The queue — the main window's table

Epic **E6**, and the listing half of E7's S7.1. The main window is laid
out the way heretic-lazy-shot's is: a toolbar, a table, a status bar.
The table is the **queue** — everything that has been handed to the
product this session, one row each — and this document is what it
takes from lazy-shot, what it changes, and why.

```
apps/wipemark-app/src/
  queue.rs      the rows, the filter bar, the sort, the paginator, the Actions menu
  preview.rs    what a row looks like before it is opened: picture, excerpt, or nothing
  wording.rs    the sentences the panel and the queue share about one thing that arrived
  drop.rs       the drop target both windows use; `Arrival` and the per-drop `Landed`
  clipboard.rs  the clipboard, watched, and what the Paste button says and takes
  main.rs       the toolbar (Import, Paste, Help) and `--import=<path>`
```

## lazy-shot's table, and the differences

lazy-shot's `ScreenshotTable` is a filter bar (id, keyword, status, a
date range, reset, the total, Import), a table (thumbnail, id, keyword,
filename, type, status, created, updated, actions) and a paginator
(page size from 10/20/50/100, previous, next, "page x of y"). The queue
keeps the shape and changes what a row is about:

| lazy-shot | the queue | why |
|---|---|---|
| a thumbnail | a **preview**: the picture for an image, the first three lines for text, a glyph for the rest | this product takes text as readily as pictures, and a row with no way to recognise a note is a row that has to be opened to be identified |
| the thumbnail is the whole preview | hovering the preview opens a **hover card** with the picture at 560 × 380 or up to two thousand characters, the caption, and what the Retention page would do with it | a row is 72 points tall; recognising a document takes more than that, and a modal for it would be a third window |
| `type` (region, window, display…) | **Kind** — Text, Image, Document, Archive, Media, Data, Folder, Unrecognised — one colour each | the same badge, over `wipemark-intake`'s taxonomy |
| `filename` | **Name**, with the folder under it and the evidence note under that — "Named Text, and the contents are PNG" in the warning colour | a file whose name lies is the file somebody most needs told about; the panel says it, and so does the row |
| `status` (active / deleted) | absent | a row has no lifecycle yet: nothing is cleaned, nothing is deleted |
| created and updated | **Arrived**, one clock reading, sortable | there is one event in a row's life so far |
| a date range in the filter bar | absent | a list that empties when the application quits does not need a calendar |
| `id LIKE '%…%'`, `key_word LIKE '%…%'` | the same, in memory: substrings, case-insensitive for the keyword | three remembered digits find the row |
| `ORDER BY id DESC` | newest first, and the Arrived header flips it | the thing just dropped is the thing being looked for |
| page size persisted as a setting | page size kept for the session | a persisted preference here is a row with a Settings widget (`every_persisted_preference_has_a_row`), and the size of a table is not yet worth one |
| Actions: open, copy path, beautify, edit, OCR, delete | Actions: **open with the default app** | the one action that means something before the cleaner exists; it is disabled, not absent, on a row with no file behind it |
| `assign_keyword` over MCP | a Keyword column and filter, and nothing that assigns one | the MCP tool that will assign one does not exist yet; the column is here so the day it lands is a tool change and not a table change |

## How things get in

One road, four doors. A drop anywhere on the main window lands in the
window's `Catcher` ([drag-and-drop.md](drag-and-drop.md)): the platform
destination is per window, the table is most of the window, and so the
table *is* the drop zone. The toolbar's **Import** opens the platform's
file picker and hands its answer to the same catcher through
`Catcher::land`; **Paste** hands over what is on the clipboard the same
way; `--import=<path>` on the command line does the same at startup, so
a check of the table does not begin by driving a file picker. A file
chosen, a thing pasted and a thing dropped are therefore recognised by
one piece of code — `wipemark-intake`, on the background executor — and
reach the queue as one kind of event.

Paste is the button that says what it would do. `clipboard.rs` watches
the clipboard — on macOS by polling the pasteboard's change count twice
a second, one call, and peeking at the item *types* only when it moved;
never the data — and the label is built from the peek: "Paste text",
"Paste image", "Paste file", "Paste 3 files", "Paste 2 items", or a
greyed "Paste" when nothing is there this window can take. A screenshot
copied to the clipboard is read once, when the button is pressed, and
through the same `pasteboard::handed` a drop is read with, so a paste
and a drop of the same thing arbitrate a file against its name and an
image against its caption identically. Off macOS the road is GPUI's own
`read_from_clipboard` (files, an image, a string), folded into the same
order by `clipboard::handed_of`; those desktops have no change count, so
the label is refreshed on window activation and after a paste (E10).

That event is `Landed`, and it carries **its own drop**. The panel reads
`Catcher::caught` — the last drop, overtaken by the next the way a
screen is — and the queue reads the event, because a second drop while
the first is still being read off a slow disk must not lose the first
from a list. `every_drop_lands_even_when_overtaken` is the gate, and it
went red with the old rule.

## The preview

`preview.rs` decides what a row looks like, over the `Handed` and the
`Intake` together, on the background executor, after the row is already
on screen (`Preview::Pending` until then).

* **A picture** for `Kind::Image`, when GPUI can decode the format —
  `ImageFormat::from_mime_type` over the intake crate's `mime()` is where
  the two vocabularies meet; HEIC and AVIF are recognised and drawn by
  nothing in this binary — and when the file is under `IMAGE_LIMIT`
  (32 MB). A decoded image is four bytes a pixel whatever the file cost,
  and a 200 MB TIFF dropped on a window is not a thumbnail. Nothing here
  reads the file: `img()` loads a path on GPUI's own background task the
  first time it is drawn. Bytes with no file behind them — a screenshot
  dragged out of a browser — go with the row as an `Arc<gpui::Image>`,
  with the format the intake crate established.
* **An excerpt** for `Kind::Text` in a textual format: the first
  `EXCERPT_CHARS` (2 000) characters, cut on a character boundary,
  decoded in the encoding the intake crate found — UTF-8, UTF-16 and
  UTF-32 either way round, the byte order mark dropped. `Encoding::Other`
  shows its ASCII and a replacement mark for the rest: the intake crate
  refuses to name a code page, and a preview that guessed one would be a
  guess drawn as a fact. At most `READ` (16 KiB) is read off a file,
  which is the rule every consumer of an `Intake` is under. A Markdown
  excerpt shows its `#` and a HTML one its tags — the bytes as they
  arrived, the retention rule wearing a different hat.
* **Nothing** for the rest, and that is an answer: an archive has no
  picture and a binary nobody recognised has no first lines. The row
  wears the kind's glyph instead.

## What it does not do yet

Clean anything. The footer says so in the words every pending surface
uses, and `the_footer_says_cleaning_is_not_here_yet` keeps an epic
number out of it. Folders and archives are listed and never expanded.
Nothing removes a row, because nothing has been done to one yet; the
list empties with the process. When E7 lands, a row is where the work
starts and the editors and the inspector (S7.2–S7.6) open from it.
