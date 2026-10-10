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
| `status` (active / deleted) | **Status** — waiting, queued, cleaning, then what the clean found (nothing found, cleaned, partly, not cleaned, failed), one colour each; a row that cannot be cleaned says so and why | a row's life is the clean ("Cleaning" below); nothing is deleted, the list empties with the process |
| created and updated | **Arrived**, one clock reading, sortable | there is one event in a row's life so far |
| a date range in the filter bar | absent | a list that empties when the application quits does not need a calendar |
| `id LIKE '%…%'`, `key_word LIKE '%…%'` | the same, in memory: substrings, case-insensitive for the keyword | three remembered digits find the row |
| `ORDER BY id DESC` | newest first, and the Arrived header flips it | the thing just dropped is the thing being looked for |
| page size persisted as a setting | page size kept for the session | a persisted preference here is a row with a Settings widget (`every_persisted_preference_has_a_row`), and the size of a table is not yet worth one |
| Actions: open, copy path, beautify, edit, OCR, delete | Actions: **Clean**, **open with the default app**, **Compare with the result**, and once cleaned **open**, **show** and **copy the result**, the **Report…**, and **Replace the existing result** when a clean was refused for one | what can be done to one thing that arrived; an item that cannot apply to a row is disabled, not absent (a row with no file behind it opens nothing) |
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

**Nothing is no item** (D301, the owner's empty row of 2026-10-07).
`Handed::is_nothing` (`wipemark-intake`) is an empty text, a text of
ASCII white space alone (U+0009, U+000A, U+000C, U+000D, U+0020), or
bytes of length zero; a path never is. Such a thing is left out by
`clipboard::handed_of`, by `pasteboard::handed` (the macOS paste *and*
drop road — the peek reads an item's text, only when the change count
moved, because only its characters can say it is empty), and once more
by `Catcher::land`, whichever road it came by. So an empty clipboard
string greys the button to "Paste", and a paste or a drop of it lands no
row. Whitespace is decided by what Layer A can find: nothing in ASCII
white space, so a stray newline is nothing; a no-break or a narrow
no-break space is what Layer A looks for (`ExoticSpace`), so a text of
those is something. `a_paste_or_a_drop_of_empty_text_lands_nothing` and
`an_empty_string_on_the_clipboard_is_nothing` are the gates.

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

## Cleaning

A row is cleaned when somebody asks — **Clean** in its Actions menu,
**Clean all** on the toolbar, `--clean=<path>` on the command line — and
never on arrival (owner question 1). Its life is a `queue::Status`:
*waiting*, *queued*, *cleaning*, *done*, the last carrying the whole
`clean::Outcome`. The Status column after Kind wears it as a badge —
nothing found muted, cleaned green, partly and not cleaned amber, failed
red — and the badge's tooltip is the outcome's one sentence
(`wording::said`); a waiting row that cannot be cleaned wears "Cannot
clean" and the reason, which is also the line under its greyed Clean
item (gpui-component's menu items have no tooltip, D269).

**One clean at a time, first asked first done — across the application**
(`cleaner::Cleaner`, a GPUI global, over the pure `cleaner::Line`, D283):
the queue's rows and the panel's Clean wait in the same line. A decoded
picture is hundreds of megabytes; a row's plan means something only if the
cleans start in the order they were asked for; and two cleans of one file
to one destination, from two windows, must end as one result and one
"already there" rather than two writes. The clean itself is
`clean::clean_one` on the background executor — no read, decode or write
on the thread that draws. **The plan is taken when the row's clean
starts**, from `Preferences::plan_for`: a Retention change made while a row
waits applies to it, and one made after it is done does not move its
result. The line says when each clean starts and what it did as events,
which is how a row goes from Queued to Cleaning to done. While it runs the
status bar says "Cleaning 2 of 5", counting the panel's cleans too. A clean
that panics — a fault in this version — is caught where it runs and ends
as *failed* with a sentence of its own, and the next clean starts (D288):
uncaught, the line would wait for it for ever, in both windows. So does a
panic while its plan is taken, on this thread before the clean is handed
out (Y1). What a panic in the middle of a write would leave — a temporary,
a second name for an original still under its first — goes as it unwinds
(`wipemark_intake::inplace`, Y8), so the next in-place clean of the file
is not refused for it.

*In place of the file* refuses a row whose file is a symbolic link before
it is read (D287) — see `retention.md`, "How the windows execute it".

Once done, the Name cell's note says where the result went
(`wording::went`) and the hover card says what happened instead of what
would. The Actions menu opens the result, shows it in its folder, copies
it when it came back as text (the person's own text, no catalogue), opens
the **Report…**, and — only when the clean was refused because a result
was already there (D261) — **Replace the existing result**, which cleans
again and writes over that one file (`clean::replace_one`, D270). The
original is never the file replaced.

**Compare saves into the row** (E7-9). A row's Compare opens on its
result as it stands once a clean put one somewhere (`cleaned_for`,
D418), and the window's Save writes back there: over the result's file,
or — for a paste — into the row itself (`Row::edited`, what Copy the
result then copies). A Save of a row nothing was written for is a clean
of the edited text, filed under the row's own id in the same line
(`Cleaner::ask_with`, `clean::save_one`, D411), so the row moves through
queued, cleaning and done as for its own Clean. Every save says so to
the row through the `compare::Link` the table hands the window
(`Queue::told_by_compare`, D412), and the row's journal entry marks the
edit — when, never what (`outcome.edited`, D417).

## The report, and its three shelves

**Report…** opens a dialog the shell paints over the whole window (an
element in its own tree, for every reason `dialog.rs` gives; opened from
a deferred click through `QueueEvent::Report`, D274). `report::sheet`
builds it as values over a `wording::Say`, in five sections:

| section | a text | a picture |
|---|---|---|
| What arrived | the title; kind, format, encoding; the evidence note | the same |
| What happened | the outcome's sentence; the result, the original set aside, the kept copies, in full paths — or where nothing went | the same |
| **Verifiable** | every character Layer A removed: `U+XXXX`, the standard's name, class, confidence, count — Layer A is deterministic and each is gone from the result; anything normalized; the Unicode version | the metadata blocks removed, with the signals that made them provenance, and the library's **second inspection** of the result; the visible marks **proved**, with their numbers; a restoration that is exact |
| **Best-effort** | every character found and kept, and why (a homoglyph is replaced only by an aggressive clean; a joiner inside an emoji stays) | every restoration that is not exact and each reason that holds — lossy, clamped, fitted, resampled, found by the search, the residual as a **mean** (D248) — holes, an outline or a texture left; proposals refused, with the number that failed; the pixels not examined; "no visible mark this version knows" (bounded by the catalogue, so not verifiable); a mark left; metadata kept, colour kept, EXIF removed whole, a rotation lost; how the picture was written back |
| **Not established** | one line per id of the report's own shelf, in its order | the same, `invisible-pixel-marks` first |

The third shelf is **never empty**, and an id this build has no sentence
for is shown as its canonical English beside the id rather than dropped.
A shelf with nothing on it for this clean says so instead of vanishing; a
thing refused before it was read says it was not read, has no JSON to
copy (the button is greyed, never an invented report), and still carries
the third shelf of its kind (D277). The mapping is D275.

**Copy JSON** puts exactly the library's `to_json()` on the clipboard — a
format, never translated. **Copy as Markdown** renders the same sheet
through `wording::plain` (`Rendering::PlainText`): no U+2068/U+2069 and
nothing Layer A would remove, in every language
(`the_markdown_copy_carries_nothing_layer_a_would_remove`). The dialog is
handed both words (`ReportView::with_words`), so a test gives it a real
`Rendering::Ui` for the window and proves the copy is not in it (D276).
Escape, the backdrop and Close are one answer.

**A result edited after the clean** (D447). A result saved edited — by a
Save in Compare that cleaned it, or a save over its result since — says
so wherever its verdict is said: the row's badge is the verdict's
"…, then edited" (*Cleaned, then edited*, *Partly cleaned, then edited*,
*Rewritten, then edited*, *Partly rewritten, then edited*), in the
verdict's colour, through one pure `wording::edited`; its tooltip, and
the sheet's "what happened" in the window and in Copy as Markdown, add
that the edits were saved as typed and nothing checked them for marks.
Copy JSON does not move. A row is edited when its clean's outcome says so,
when its journal entry has `outcome.edited`, or when a mark landed this
session (`Row::edited_at`, gone with the next clean or rewrite of the
row). Built at the default; the alternative, for the owner — Layer A run
over the edit before it is written, the verdict honest without a new word
at the cost of characters typed on purpose — is in
`docs/architecture/compare.md`, "What a save does not do".

**Report… of a row the journal keeps** (D448). The dialog is a clean of
this session's: `report_of` answers `Status::Done` only. A row of an
earlier session, of the command line, of an agent, and every rewrite is
`Status::Recorded`, and its Report… is greyed **with its reason under
it**, the way Clean's is (D269): the full report is kept by the window
that made it, and for this row the list keeps a summary, which its
status says (`queue::why_no_report`,
`report_of_a_journal_row_is_greyed_with_its_reason`). Built at the
default. The journal holds metadata only (D312) — counts, never the
characters — so a sheet built from it would put on its *verifiable*
shelf what nothing backs, and there would be no `to_json()` to copy.
**The alternative**, for the owner: enable it with what the journal
holds — arrived and happened from the entry, the counts, "the full report
was not kept" on the verifiable shelf, the third shelf from
`not_established::ids()`, Copy JSON greyed.

## Rewriting (E4-6b)

```
apps/wipemark-app/src/queue/rewriting.rs   Rewrite, Rewrite all and its price, Pause, Cancel, Remove, Clear finished, the journal merge
apps/wipemark-app/src/journal.rs           the application's journal: the writer, the bookkeeper, the launch's settling, Work
apps/wipemark-app/src/engine_host.rs       `impl wipemark_queue::EngineSource for EngineHandle`, `EngineHandle::when_changed`
crates/wipemark-queue/src/source.rs        `EngineSource`: an engine asked for when each item starts
crates/wipemark-store/src/journal.rs       schema 3: the journal table, `JournalWriter`
crates/wipemark-store/src/entry.rs         the journal's words: Origin, Action, Phase, Entry — shared by the window and the CLI
```

**Rewrite** is beside Clean on every row — as a button in the Process
column and as the second item of the Actions menu (D325) — and **Rewrite
all** is on the toolbar after Clean all. Both push to the application's
batch queue, `wipemark-queue`, opened at startup over `wipemark.db`
(`main::open_work`). It is the **one line of rewrites** for the whole
application: a window's row, an agent's `rewrite` call and the command
line's call through the application each become an item, one runs at a
time, first come first served (В9, R7). Cleans keep their own line
(D283): they are quick, and a picture must not wait behind a twenty-minute
rewrite. A clean asked for a row being rewritten is greyed with its reason
(it would race the rewrite over a file the rewrite reads again after a
restart).

The engine is the one on duty **when each item starts** (R1, D310): the
queue holds an `EngineSource`, and the application's is the engine handle,
taken through `EngineHandle::for_job` so the item is counted busy for its
whole length and an Unload now waits for it. Nothing on duty, or a refusal
(a model that will not load, a key the endpoint refused), is a **hold** —
the queue's own pause, said as "Waiting for an engine" on the row and in
the status bar — never a failed item (D311). It lifts itself when the duty
changes (`EngineHandle::when_changed`) or on Resume; it is never retried in
a loop, because asking an endpoint means reading a key from the keychain.

**The price first** (D61): Rewrite all prices every row it would take —
`wipemark_pipeline::job::plan` over each document and `Planned::cost` at
the rate the last Check measured — and the shell asks before anything is
pushed: calls, tokens, minutes, or "unknown" when the rate was never
measured, and whether the documents leave this machine. A single row's
Rewrite starts at once and says its price in the row's tooltip.

What a window asks of the model is an agent's call with no arguments
(D323): a paraphrase at the default intensity, the effort the engine on
duty takes, Layer A at its defaults, the template rows the Settings window
saved. Where the result goes is the Retention page's plan taken **at
push**, not at start (R3, D91): `name.rewritten.ext` beside the file (В8),
the same name in the results folder — each a *new* file, refused where one
is already (`Destination::New`, D261, D319), with Replace the existing
result for that one file — in place with the original set aside, or, for a
thing with no file behind it, the queue's row (`Destination::Row`), shown
and copied from there until the row is removed. A file already where the
result would go is found **when the row is pushed** (D357), off the GPUI
thread with the rest of the build: the row is *Rewrite failed* at once, its
tooltip naming the file, nothing is pushed and nothing runs, and Replace the
existing result pushes `Destination::File` for that one file. The publish
still refuses a file that appears while the job runs (D284); that rewrite's
text is not kept for a later Replace — the check at push makes the race a
moment wide, and Replace runs the job again.

**A saved template the rules refuse refuses the push** (D374, L-h). The
templates the job will use are checked as the row is built: with the
document planned, the plan's own fallbacks — exactly the overrides the job
would have dropped as it rendered; with no plan (a file that cannot be read
now), every saved template of a tactic on the job's ladder, in any
language, by `row::admit`. One that breaks a rule — a template saved
before D369 holding an invisible character — makes the row *Rewrite
failed* with the template's row key and the rule as its reason (`template
prompts.en.paraphrase.1.user: invisible-character`), nothing pushed, as
the command line and an agent's call refuse a template they are handed.

**Consent at push, asked again at start** (D361). Every window push records
where the person agreed the document may go — this machine, or an
endpoint's origin — as the duty stood when they asked: a row's Rewrite,
Rewrite all whose price said "here" or "sent away", a drop asked about once
(В1), a **Replace the existing result** (D393) — the duty's destination,
and nothing else (D430, below). A window push without a consent cannot be
built: with nothing on duty there is nowhere the person agreed to, and
nothing is pushed. Replace of a rewrite refused over a file
already there is a Rewrite by the same road — greyed, its reason under it,
when a Rewrite would be (nothing on duty, in the engine handle's words), and
asked "Send … to …?" first when the document would leave this machine
(`QueueEvent::SendAway { replacing }`, yes is `Queue::agreed` by
`Road::Replace`); it
differs from a Rewrite only in where its result goes. Before D393 it skipped
both, and with nothing on duty it pushed an item with no consent — an item
the queue never asks about — which then went to whatever endpoint was put
on duty later (`replace_with_nothing_on_duty_pushes_nothing`,
`replace_asks_before_a_document_leaves_the_machine`). The engine is bound when each item starts, so the duty can move while
items wait. Before an item starts on an endpoint other than the one it was
consented to (here → away, or one origin → another), the queue **holds and
asks once** (`QueueEvent::Ask`, the window's Confirm "Send the waiting
documents to …?"): yes (`Queue::agree`) lets every waiting item go there;
no leaves the queue holding — Resume, or another engine on duty, asks
again. A duty that came back here asks nothing. An agent's or the command
line's item carries no consent of the window's — its caller asked for it —
and is never asked about. That is the deliberate
asymmetry with a clean, whose plan is taken when it starts (D283): the
queue executes what was stored, after a restart too.

**A consent is checked against the engine the item is handed** (D370, the
host verification's M-1). The queue's engine source in the application is
the engine handle itself: `EngineSource::for_item` hands out the engine
**and where that engine sends a document** (`Handed`), read together from
the handle's slot, which the engine host fills with the engine and the
performer's destination (`Performer::whereto`, the one rule the window's
consent uses too). The check runs after the engine is taken and before
anything is read. The window's record of where the duty is meant to be
(`journal::Going`, worked out from the preferences whenever they change)
only words the window and wakes the queue; it is never what an item is
checked against — because the two move at different times: `Going` the
moment a preference changes, the slot only when the host's swap runs, and
the host defers a swap while anything is busy. Checked against `Going`, an
item consented to stay here started on the endpoint's engine a deferred
swap had left in the slot, and every item after it did too, the host
seeing the queue's own item as busy. On a question the engine is let go of
at once, so the host sees nothing busy and the deferred swap can land; the
swap then wakes the queue, the question is withdrawn, and the item is
asked about afresh — on this machine, nothing to ask.

**No item starts on the engine leaving** (D395, L-1). A change of duty the
engine host defers while a job runs is said on the handle
(`EngineHandle::swap_pending`, `EngineSource::settling`) from the moment
it is deferred until the job ends and the swap has run; while it is, the
queue starts nothing — silently, not a hold, so no caller is refused for
it — and the end of it wakes the queue as a swap does, after the swap and
never before (`a_deferred_swap_is_pending_until_it_lands`,
`no_item_starts_on_the_engine_leaving`). Before, items consented to the
old endpoint kept starting on it the moment the item before them ended —
the busy count never reached nought, so the swap never landed — and the
rest of a switched-away queue went to the endpoint the duty had left.

**One fact for consent** (D430, the follow-ups of E7-8, A-M1). The consent
a push records, the Send-away question and the vacancy a Rewrite is greyed
with are answered from one place, `Queue::going`: where the duty, as the
person has set it, would send a rewrite — this machine or an endpoint's
origin — or why nothing would, in the engine handle's words. It is the
duty's answer and never the slot's. Before, a push read the duty first and
fell back on the slot's word where the duty named nobody (D393's "the
engine on duty's own word"), and the vacancy was the slot's alone: with the
queue rewriting on endpoint Y and the duty turned to nobody, the swap
deferred until Y's job ended (D395), the slot still said Y — so a Rewrite
was not greyed, and its push recorded `Away(Y)` with no question shown; the
item waited, and went to Y if Y was ever put back. The slot still speaks for
the duty it was built for — a machine this build cannot run is refused
there, and that refusal is the reason — but not while a swap is pending,
when it holds the engine leaving
(`a_duty_turned_to_nobody_agrees_to_nothing_while_its_swap_waits`). The
table tests that pushed through the fallback — a fake engine in the slot,
nobody in the preferences — put a duty on this machine in their
preferences, as a person must.

**The duty once per draw of the rows** (D452, the follow-ups' verification).
Asking `Queue::going` builds a `Roster` — the installed, weights and added
maps cloned — and runs `duty::on_duty`, and a row asked it up to three
times (its Rewrite button, the menu's Rewrite, Replace's reason), every row
on screen, every frame. The rows of a draw now share one answer:
`Queue::vacancy` is taken once in `render`, where the list's processor is
made, and each row reads its Rewrite's reason through
`why_not_rewrite_given(id, vacant)` — the pure `why_not_rewrite` over it.
Not inside the processor: the list calls it three times a draw (a row
measured in its layout and again in its prepaint, then the rows on screen),
which would be three asks. The callers outside a draw — Replace and
"Process what arrives" — keep `why_not_rewrite(id, cx)`, and Rewrite all
its own `vacancy`
(`the_duty_is_asked_once_per_draw_of_the_rows`: one ask for a draw of
twenty rows; one a row asks seventeen times — two rows measured, fifteen on
screen — and the code before asked two or three times a row).

**A yes records the question's destination** (D431, A-L1). Every question
asked before rewrites go names where they would go — Rewrite all's price
(`price.away`), a drop's and a Replace's "Send … to …?" — and its yes is
`Queue::agreed(ids, road, asked)`, the destination it named. That is the
consent recorded, never the duty as it stands at the yes: a duty moved
while the question was open made the record name a place nobody was asked
about. If the duty no longer sends a rewrite to `asked`, the yes pushes
nothing and the road (`Road::Price`, `Arrivals`, `Replace`) runs again with
the duty as it stands — asking again where that road asks (a price, a
document leaving this machine), pushing at once where a fresh press would
push without a question (a drop or a Replace that now stays here)
(`a_yes_records_where_its_question_said_and_no_other`).

**A change of engine waited for is said** (D434, A-L5). While a change of
duty waits for a running job (D395) the queue starts nothing and holds no
caller; the status bar says so — "Rewrites wait for the engine to change"
— where it said nothing over items that did not move
(`a_change_of_engine_waited_for_is_said_in_the_status_bar`). Read from the
handle's flag, in memory (D359).

**A swap is told when it is deferred, and when it lands** (D454). The
handle tells whoever watches the slot on every change of its "swap pending"
flag, not only as it clears. The window never waited for it — the host
notifies at the end of the event that deferred the swap, the shell observes
the host and the status bar reads the flag directly — but the batch queue
hears of the slot only through the watcher, and kept a hold or a question
about the engine leaving until the job ended. Told as the swap is deferred,
the queue looks again (a Retry lifts its hold and withdraws its question),
finds the source settling, and still waits: nothing starts on the engine
that is leaving (D395, D430;
`a_deferred_swap_is_pending_until_it_lands`, which holds both words,
`no_item_starts_on_the_engine_leaving`, which holds the wait).

**Where first, the key after** (D396, L-2). Before an item's engine is
built — for an endpoint, before its key is read from the credential store,
which on macOS can put a keychain prompt on screen — the queue asks the
source where that engine would send the document (`EngineSource::whereto`,
the slot's word, nothing built), and an item that would only be asked about
is asked about there. The engine actually handed out is checked again
after, and that is still the check that decides (D370)
(`an_item_asked_about_costs_no_engine_build`).

**A yes covers the items it was asked for** (D372, L-b): the waiting items
consented to somewhere else when the person said yes — the ones the
question counted. An item pushed afterwards, consented by its own push,
is asked about on its own: a yes to endpoint Y given for B, the duty moved
here, C pushed to stay here and the duty moved back to Y, asks about C
rather than send it on B's answer.

The Status column says *Queued for rewrite*, *Waiting for an engine*,
*Rewriting…* (paragraph k of n in the tooltip), *Rewritten*, *Partly
rewritten* (a paragraph kept its cleaned original — the CLI's exit 3),
*Rewrite failed*, *Rewrite cancelled*; the status bar says "Rewriting 2 of
5 · paragraph 7 of 52", or why the line waits, after a model's load and a
clean (D324). **Pause** and **Resume** are on the toolbar while anything
is in the line — every surface's. What the toolbar and the status bar read
on every frame is in memory (D359): whether the queue is paused is the
queue handle's own flag, set by its thread, never a query on the shared
connection, and the count of open items is `Queue::states` — ids and
states, no stored report cloned.

**Remove** on a row an agent or the command line is waiting for is greyed
while its rewrite is queued or running, its reason under it (D355): Cancel
ends the item and the caller is told. Should an item go anyway — removed
by any road — the waiting call ends at once, as a refusal that says the
document was removed, never a call left open until its ceiling. An
agent's or the command line's call whose item waits behind a consent
question (D361) is refused once the question has **stood for two seconds**
(`QUESTION_GRACE`, D394) — looked at every quarter second while it waits,
and before it is queued — with a sentence saying the application's rewrites
wait for an answer in its window (D373, L-d): its ceiling counts from its
start, which a question nobody answers never gives it. The grace is for the
question the queue withdraws on its own: put as an item starts on an engine
a swap is about to replace and gone as the swap lands, it used to refuse
every caller waiting behind it, "answer it there", over a window with
nothing to answer (`a_question_withdrawn_within_its_grace_refuses_nobody`).
A question that stands two seconds is one a person has to answer.

**A withdrawn question is taken down** (D394, M-B). The queue says
`Unasked` when its question goes — answered, the duty moved, the person
resumed, the item gone — and the window takes the question off the screen
if it is the one open, and drops every copy waiting its turn; the next
question that is not it is asked. The same question is never stacked: one
already open or waiting is not queued again when the queue asks it anew
(`a_withdrawn_question_is_taken_down_and_no_other`,
`the_same_question_is_not_stacked`). A yes given to a question already
withdrawn — a dialog answered in the moment it was being taken down —
covers nobody (`a_yes_to_a_withdrawn_question_changes_nothing`).

The Price, Send-away and consent questions are asked **one at a time, in
the order they came** (D364): one that arrives while another is open waits
its turn, and is never put in its place.

### "Process what arrives" (В1)

A drop is a row and nothing else by default: the button asks. The General
page's switch (`queue.on_arrival`: nothing, clean, rewrite) does it as
things land; with rewrite chosen and the engine on duty not on this
machine, each arrival asks once before anything is sent. A row nobody has
asked to process says **Not started**, its tooltip naming what would
process it — never "Waiting", which the owner read as "something will
process it" (D318). "Queued" is only ever a row in a line. A drop of
several files that would be sent away is one question naming every row
and the endpoint; nothing is pushed until it is answered, and a yes
pushes them with that endpoint as their consent
(`a_drop_that_would_be_sent_away_is_asked_about_first`).

## The journal (E4-6b)

The table **is** the document journal (R4): one row per document handed
over — dropped, pasted, imported, named at launch, cleaned in the panel,
cleaned or rewritten by an agent or the command line — in
`wipemark-store`'s `journal` table (schema 3). The ID column shows the
journal's id, which is what an agent names a document by; the session's
own number stays the element id (two id spaces). A row says who asked
("From the command line", "From an agent", "From the panel").

* **What a row keeps** is metadata (D312, В4): name, file, kind, size,
  what was asked, what came of it (a verdict id and counts), where the
  result went or "returned to the caller" — never the document, never a
  model's words. A thing with no file behind it cannot be processed again
  once it ended or the application restarted; its menu says why.
* **Who writes.** The windows through a writer thread, in the order asked,
  never on the GPUI thread; MCP's connection threads directly, handing the
  id back as `result._meta["wipemark/journal"]`; the **bookkeeper**, a
  thread on the queue's first channel, writes every rewrite's start and a
  window rewrite's end, while an agent's or the command line's end is
  written by the call that waits for it, so the command line's own update
  (the file it wrote) comes last (D313, D321). A start heard late never
  reopens an ended row (`Journal::update_open`).
* **The command line**, with no application, writes its own row through
  `JournalWriter`, which never creates or migrates a database (D314, В5);
  the window notices it by `PRAGMA data_version`, read once a second, and
  rows written inside the process by a notes channel, looked at ten times a
  second (D316). Polled rather than awaited: a GPUI task woken from
  another thread is a wake the window's executor did not schedule.
* **One row per document** (D320): cleaned and then rewritten is one row,
  whose action is the last asked. So an edit mark from a Compare window
  names the result it is about — the action and where it lives — and
  lands only while that is still the row's latest result, checked by the
  row and again in the SQL (D442); an entry this build cannot read is left
  as it is (D444).
* **A row names its item before the item can start** (D358): a window's
  rewrite is reserved an id (`Queue::reserve`), its row is written "queued"
  with that id on the writer's thread, and only then is the item pushed
  (`Queue::push_reserved`); an agent's call records its row between the
  same two steps. Pushed first, an item that ended before the "queued"
  landed lost its end and stayed queued for ever. The reserved id is the
  queue's from `reserve` on (D371, L-a): the row has its id as soon as it
  is reserved, while the push runs later on the writer's thread, so a
  Cancel or a Remove pressed in between reaches the queue first — the
  queue keeps it for that id, and the push then ends the item as
  cancelled, or drops it. Lost, a removed row's document was rewritten and
  `name.rewritten.ext` written for a row that was gone.
* **A row is read back without waiting on its file** (D356). The journal's
  rows are read by themselves; each new row's file is then looked at in a
  task of its own, so a file that will not answer holds nothing else up
  and the read always ends. A path that is not a regular file or a folder
  — a FIFO, a terminal's `/dev/stdin`, a device — is never opened: the row
  has nothing behind it. The command line records such a path as no file,
  and a path an MCP client names in `_meta` is the row's `said_path`,
  shown and never opened.
* **It survives a restart** (В3). At launch (D317) a window clean left
  mid-way waits again — or, with no file behind it, goes, its text never
  having been kept — an agent's or the command line's rewrite still queued
  is cancelled and its queue row removed (whoever asked is gone), and a
  window's rewrite is left for the queue to take up. **Remove** on a row,
  **Clear finished** on the toolbar, and **Keep finished rows** on the
  Retention page (`journal.keep_days`, a week by default) take rows away —
  with the queue row of a result whose only home was the row. The Arrived
  column says the date before the time for a row from another day (D363).
* **An agent's text** goes back to the agent and its queue row is removed
  the moment it has (D313).

## What it does not do yet

Folders and archives are listed and never expanded. The editors and the
inspector (S7.2–S7.6) are not here; a rewrite in the panel is not either
(В10) — the panel cleans, and says rewriting is in the main window.
