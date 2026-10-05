# E7-1…E7-6 — The windows clean

|                  |                                                                                                                                       |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic E7, the workspace UI                                                                                               |
| Task             | Watchword FILE `wipemark-task-e7-windows-clean-2026-10-05` (ttl 0)                                                                     |
| Spec scopes      | S7.1 "Clean" in the queue, S7.6 the report with its three shelves; A §7.4's seams (Compare's result is `clean(original)`, the panel's findings count); retention rules 1–7 executed |
| Depends on       | E1 (Layer A), E11/E12 (`wipemark-picture`), E5-1 (`wipemark_intake::inplace`), E6's queue, panel and Compare window — all done       |
| Base             | `images/series-v3` (`ebf421a`), branch `e7/windows-clean`                                                                              |
| Unblocks         | E4-6b (rewriting from the windows), the history table                                                                                  |
| Files touched    | `apps/wipemark-app/src/{clean,queue,compare,panel,main,settings,wording,retention}.rs`, the three `.ftl` catalogues (one block each, `## The windows clean (E7)`), `docs/architecture/{queue,retention,compare,drag-and-drop,images,visible-marks}.md` (the lines named in E7-6 only), this document, `docs/plan/reports/` |
| Not touched      | `crates/wipemark-{pixels,picture,image}/`, `apps/wipemark-cli/`, `apps/wipemark-app/src/mcp/` (the images agent's), `CLAUDE.md`, `docs/plan/README.md` (wanted edits go in the report) |
| Size             | six steps, each green on its own                                                                                                       |

## §0 Ground rules

- `CLAUDE.md` wins over this document and over the task wherever they
  disagree; a fact here that no longer matches the code is trusted to the
  code and said in the report.
- **Nothing blocking runs on the GPUI thread.** Every read, decode, hash and
  write of a clean is `clean::clean_one`, which blocks, and every caller
  runs it on the background executor (`cx.spawn` +
  `background_executor().spawn`, the pattern `queue::take` uses).
- **The bytes decide.** What a thing is, is decided again from the bytes
  actually read when the clean starts — a file can change after its drop.
- **Every string a person reads comes from the catalogue; nothing a machine
  reads does.** `clean.rs` localizes nothing: it hands up values. A copied
  report is not a window and is rendered with `t_plain`.
- **No epic number leaves this repository.** Nothing a window shows says E4,
  E7 or "E4-6b"; what is not here is "not in this version yet".
- **The third shelf is never empty**, in a window as in the CLI.
- **Tests must be able to fail.** Every protection below has a row in
  `docs/plan/reports/e7-windows-clean-mutate.py`.
- Commits `E7-n: <what>`, pushed to `e7/windows-clean` only, never to
  `main`, `feat/e0-e6-shell` or `images/series-v3`, no PR. Before every push:
  merge `origin/feat/e0-e6-shell` if it already contains
  `origin/images/series-v3`, otherwise `origin/images/series-v3` if it
  moved. Never rebase a pushed commit.

## §1 Goal

The application's windows clean what the CLI cleans, through the same
libraries: a row of the queue is cleaned, its result written where the
Retention page says, a pasted thing kept only if asked, the report shown
with its three shelves, Compare showing the real result, and the panel
saying what it found and cleaning it. Layer B stays out of the windows
(E4-6b), and every window keeps saying so.

## §2 Read first

`CLAUDE.md` whole; `docs/architecture/{retention,queue,compare,drag-and-drop,images,visible-marks,layer-a,cli}.md`
— `cli.md`'s `clean`, `clean --in-place` and Images sections are the
behaviour to match; this plan's §1.3 seams in `docs/plan/README.md`.

## §3 What is true today (at `ebf421a`)

| fact | where |
|---|---|
| Layer A: `clean(text, &Options) -> Cleaned { text, report: CleanReport }`; `CleanReport::suspicious` is over the **input** (D28); `to_json()` ends in `not_established` | `crates/wipemark-core/src/lib.rs:115`, `report.rs:176` |
| Pictures: `wipemark_picture::clean(bytes, &PictureOptions) -> (Vec<u8>, PictureReport)`; `marks_left()`, `inconclusive()`, `to_json()` with `invisible-pixel-marks` first | `crates/wipemark-picture/src/lib.rs:143,305` |
| `still_has_ai_metadata` / `still_has_c2pa` come from a second inspection of the output | `crates/wipemark-image/src/lib.rs:335` |
| Reading and writing a text: `text::decode` (never lossy), `text::encode` (a leading U+FEFF becomes the encoding's mark) | `crates/wipemark-intake/src/text.rs:222,280` |
| Writing: `inplace::write_atomically`, `inplace::replace(path, bytes, Keep)` (sets aside first, refuses an existing original), `inplace::same_file` | `crates/wipemark-intake/src/inplace.rs:102,155,188` |
| The CLI's policy: text `clean` writes when the text changed, exits by `suspicious`; a picture still marked is not written (exit 3), marks left or not examined is written (exit 3) | `apps/wipemark-cli/src/run.rs:181`, `image.rs:136,162` |
| The plan: `retention::plan(source, retention, homes) -> Plan`, pure; `Plan::File` has no `Kept` | `apps/wipemark-app/src/retention.rs:385` |
| The queue takes a drop and plans it, and cleans nothing; footer `QueuePending` | `apps/wipemark-app/src/queue.rs:536,1244` |
| Compare's result is a copy of the original | `apps/wipemark-app/src/compare.rs:730,750` |
| The panel lists four caught rows with their *would* lines; `PanelPending` | `apps/wipemark-app/src/panel.rs:445,547,579` |
| The pending sentences | `crates/wipemark-i18n/i18n/en-US/wipemark.ftl:65,92,201,318,390,934` |

## §4 Deliverables

### E7-1 — the cleaner, with no window (`apps/wipemark-app/src/clean.rs`)

- `cleanable(&Intake) -> Cleanable` — `Text(Encoding)`, `Picture(Format)`
  or `No(Unable)`, from the intake alone: text with a named encoding;
  PNG/JPEG/WebP **placed by the bytes**; TIFF/HEIC/AVIF refused by name
  (`Unable::NotYet`); a folder, an archive, a document, media, data, a GIF,
  an unnamed eight-bit encoding, or a thing nothing but its name placed
  (`Unable::Unread`) — each with why. An empty file is an empty text.
- `clean_one(&Arrival, &Plan, row, now) -> Outcome`, **blocking**:
  1. **read** — a path is read head first, identified **again** from those
     bytes, refused past `compare::TEXT_LIMIT` (8 MiB) or `PICTURE_LIMIT`
     (64 MiB, D264) with its size (checked on the `stat`, and on the read,
     which never takes more than the limit plus one byte); a paste is its
     own characters, bytes are their own bytes; a decode error is a refusal
     at its offset;
  2. **clean** — `wipemark_core::clean(&text, &Options::default())`, or
     `wipemark_picture::clean` with `Scope::AiProvenance`;
  3. **decide** — `outcome_of(&Cleaning) -> (Verdict, writes)`, pure (below);
  4. **write, by the plan** — `Beside`/`Into` only where nothing is (D261),
     never onto the source (`same_file`); `Into { name: None }` gets
     `wipemark-<yyyymmdd-hhmmss>-<row>.cleaned.<ext>` (local time);
     `Over` through `inplace::replace(.., Keep::Original)` only over the
     file that was read; `AsText` hands the cleaned text back in the
     outcome; text is written in the encoding it arrived in, its mark kept;
  5. **keep** — only for `Plan::Loose { kept: Some(_) }`, only what it asks,
     into `<kept>/<yyyymmddThhmmss UTC>-<row>/{original,result}.<ext>`,
     before the result is written, and only when there is a result (D265);
  6. **sweep** — `sweep(kept, Period, now)` removes the per-clean
     directories past their period by the time in their names; never a name
     not shaped like ours, never a link, nothing under `Forever`, and never
     creates the folder (D267). Once at launch on the background executor
     (`main.rs`), and after each kept write;
  7. the `Outcome` carries the verdict, the report (`Report::Text(CleanReport)`
     or `Report::Picture(PictureReport)`, with `to_json()`), `written`,
     `set_aside`, `kept`, the cleaned text for `AsText`, the format and both
     sizes;
  8. one `tracing` line per clean — the row, the verdict, the sizes, the
     paths as `Elided` shapes and the refusal's kind; never the text, never a
     path in full (D266).

`outcome_of`, the CLI's policy stated once:

| `Verdict` | text | picture | written |
|---|---|---|---|
| `NothingFound` | not suspicious, unchanged | nothing AI removed, nothing restored, nothing left, examined | **nothing** (D260) |
| `Cleaned` | changed — suspicious or not (a soft hyphen removed is a change, D263) | AI metadata removed or a mark restored, nothing left, examined | yes, when the bytes changed |
| `Partly(Left)` | suspicious and unchanged: `Left::Kept`, a homoglyph kept at the defaults (D263) | `Left::Mark` (`marks_left()`) or `Left::NotExamined` (`inconclusive()`), metadata clean | a picture yes, **when the bytes changed** (D262); a text never — it is unchanged |
| `NotCleaned(Refusal)` | not cleanable, too big, unreadable, undecodable | `StillMarked` (`still_has_ai_metadata`/`still_has_c2pa`), `PictureError`; and for both an existing result, an existing original, the source itself, nowhere to go | **nothing** |
| `Failed(Failure)` | `Write`, `SetAside`, `Stranded` (the original's whereabouts said) | the same | nothing, or `Stranded` as such |

### E7-2 — the queue cleans

`Row::status` (`Waiting`, `Queued`, `Cleaning`, `Done(Arc<Outcome>)`), a
Status column after Kind with a badge per verdict and its sentence as
tooltip (a waiting row that cannot be cleaned wears "Cannot clean" and the
reason); **Clean** in the Actions menu (disabled with the reason under it,
D269), **Clean all** on the toolbar, `--clean=<path>` (D268); one clean at a
time, first in first out — `queue::Line`, pure — on the background
executor; the plan taken when the clean starts; the Name cell's note says
where the result went (`wording::went`), the hover card says what happened
(`wording::said`) instead of what would; Actions gains Open the result, Show
the result in its folder, Copy the result, Replace the existing result (only
on `NotCleaned(Exists)`, through `clean::replace_one`, D270) — Report… comes
with its dialog in E7-4; the footer says only that rewriting is not here,
and so does the toolbar's help (D271); "Cleaning 2 of 5" in the status bar;
`Preferences::for_tests` so a `Queue` is built in a `#[gpui::test]`.

### E7-3 — Compare shows the real result

`Subject::read` cleans with Layer A at its defaults in the same background
task as the read (`Loaded::cleaned`); `loaded` puts it in the result pane and
the view keeps it, so `reset` puts it back without cleaning on the GPUI
thread (D273); Reset is offered once the result differs from the cleaned
text and says "Back to the cleaned text" (D272); `compare-pending` becomes a
true sentence (the right side is what cleaning gives; editing saves
nothing; closing writes nothing), on the Settings › Compare page too, and
`compare-reset`, its tooltip and `compare-help-close` with it.

### E7-4 — the report, with its three shelves

`report.rs`: `sheet` builds what arrived, what happened, *Verifiable*,
*Best-effort*, *Not established* (one line per id of the report's own shelf,
in its order, never empty) as values over a `wording::Say`; `ReportView` is
the dialog, painted by the shell over the whole window and opened by
`QueueEvent::Report` from the row's deferred Report… (D274); Copy JSON
(`to_json()` exactly, greyed when nothing was read — D277), Copy as Markdown
(`wording::plain`, no U+2068/U+2069, nothing Layer A would remove — in en,
ru and de), Close. The shelf mapping is D275 and `docs/architecture/queue.md`,
"The report"; the `wording` sentences take a `Say` so the window and the
copy are one sheet in two renderings (D276).

### E7-5 — the panel

A findings line per listed row — `clean::inspect_one`, the bytes a clean
would read and the layer's inspection, on the background executor, once per
drop (D278), `wording::found` the sentence; a Clean button beside the
dismissal line that cleans every caught thing that can be, one at a time,
through `clean_one` with the plan taken at each start (D279), greyed while a
clean runs and once nothing is left; after it the row says what happened
(`wording::said`, coloured by the verdict) and where it went
(`wording::went`); a thing that cannot be cleaned has no *would* lines (a
folder's "every file in it would be handled" was no longer true);
`panel-pending` says only that rewriting is not here; `clean::number`, one
counter for the queue's rows and the panel's cleans (D280); a tests module of
pure functions.

### E7-6 — every "not yet", and the docs

`toolbar-help-pending`, `settings-retention-pending` (renamed test
`the_retention_banner_says_who_follows_it`), `setup-welcome-body`, and
every other sentence that says the windows do not clean — in en, ru and de;
the architecture documents named in the task, those lines only.

## §5 Tests, with a mutation per protection

E7-1 (`apps/wipemark-app/src/clean.rs`, plain `#[test]` over a scratch
directory and the committed fixtures):

| test | protects | mutation (`e7-windows-clean-mutate.py`) |
|---|---|---|
| `a_marked_text_is_written_beside_it_in_its_own_encoding` | `x.cleaned.md` is `encode(clean(text))` in UTF-8, UTF-16LE + BOM, UTF-32BE; the source untouched | M11 write UTF-8 always |
| `nothing_found_writes_nothing` | D260 | M2 always write a text |
| `an_existing_result_is_refused_and_left_alone` | D261, byte for byte | M1 drop the existence check |
| `in_place_sets_the_original_aside_and_never_twice` | `x.original.md` first; a second run refused, both files untouched | M7 `Keep::Nothing` |
| `into_the_results_folder_and_never_over_the_source` | `Into`, and the source never written, even when the results folder is its own | M12 drop `same_file` |
| `a_paste_writes_nothing_and_keeps_nothing_unless_asked` | nothing written; `kept/` absent with both switches off; the bytes as they arrived with `keep.originals` | — (M6 covers the eager folder) |
| `a_dropped_file_is_never_kept` | `Plan::File` has no `Kept` | M5 plan a file as `Loose` |
| `nameless_bytes_get_a_name_of_the_clock_and_the_row` | the invented name, deterministic; both kept copies | — |
| `a_file_that_changed_after_the_drop_is_read_by_its_bytes` | a text turned PNG is cleaned as a picture, never decoded as text | M10 decide by the drop's intake |
| `a_text_past_the_limit_is_refused_with_its_size`, `a_text_that_does_not_decode_is_refused_at_its_offset`, `a_write_that_fails_says_where_and_why` | the refusals and failures, nothing left behind | — |
| `the_sweep_removes_what_is_past_its_period_and_nothing_else`, `only_our_own_names_carry_a_stamp` | expired gone, fresh kept, foreign kept, `Forever` removes nothing, a missing folder not created | M8 ignore the period, M9 `Forever` as zero days, M6 create the folder |
| `a_gemini_mark_comes_off_and_is_written_beside` | `torch-1025.png` → `Cleaned`, no visible mark on re-inspection | — |
| `a_mark_with_its_outline_left_is_written_and_said` | `crying-1025.png` → `Partly(Mark)`, written | — |
| `a_transparent_mark_is_left_and_nothing_identical_is_written` | `crying-transparent-1025.png` → `Partly(Mark)`, output = input, nothing written (D262) | M3 always write a picture |
| `a_picture_with_nothing_on_it_writes_nothing` | `cut-out-confetti-256.webp` → `NothingFound` | M3 |
| `ai_metadata_is_removed_and_gone_on_reinspection` | `c2pa-jumbf.jpg`, `xmp-provenance-url.png` → `Cleaned`, no AI metadata, no C2PA | — |
| `a_tiff_is_refused_by_name`, `a_truncated_png_is_refused_as_malformed` | `NotYet(Tiff)`, `Malformed`, nothing written | — |
| `a_picture_comes_to_what_the_cli_says_it_does`, `a_picture_still_marked_is_never_written`, `a_text_comes_to_what_the_cli_says_it_does` | `outcome_of`, one row of the table each | M4 write a still-marked picture, M2, M3 |
| `what_can_be_cleaned_is_decided_from_the_intake` | the `cleanable` table | — |

E7-2 (`queue.rs`, `wording.rs`, `main.rs`; the two `#[gpui::test]`s build a
real `Queue` over `Preferences::for_tests` and a scratch directory):

| test | protects | mutation |
|---|---|---|
| `one_clean_runs_at_a_time_in_the_order_asked` | `Line`: one running, FIFO, the count | E7-2/M1 two at once |
| `the_queue_cleans_one_row_at_a_time_by_the_plan_at_its_start` | `Cleaning`/`Queued`/`Waiting` in order; Clean all skips the TIFF and what is not waiting; nothing asked twice; the second row follows a destination changed while it waited, the first does not; the status bar's count | E7-2/M1, M2 the plan not the page's at start, M3 a row asked twice |
| `a_refused_result_is_replaced_only_when_asked` | `--clean=` lands and cleans; `Exists` refused and left; Replace writes over that file and says so; the source untouched | — |
| `a_result_is_replaced_only_where_it_was_named` (`clean.rs`) | `replace_one` replaces only the named file, never the source | E7-2/M5 replace whatever is there |
| `clean_is_greyed_with_a_reason_when_it_cannot_run` | the menu's rule; the reason names the format | E7-2/M4 offer Clean on a TIFF |
| `the_footer_says_rewriting_is_not_here_yet` | the footer's new sentence; red with the old one and with an epic number | E7-2/M6, M7 |
| `every_outcome_reads_as_a_sentence`, `where_a_result_went_is_said_by_its_name` (`wording.rs`) | every verdict, refusal and failure has a sentence and a *went* line; the text itself is never a note | — |
| `files_can_be_cleaned_from_the_command_line`, `the_status_bar_counts_the_cleans` (`main.rs`) | `--clean=` parsing; the status line | — |

E7-3 (`compare.rs`, `#[gpui::test]` over `window_with` / `window_on`):

| test | protects | mutation |
|---|---|---|
| `the_result_is_the_cleaned_text_and_the_original_is_not` | the original keeps U+200B, the result is `clean(original)`, Reset not offered untouched | E7-3/M1 a copy of the original, M3 the read does not clean, M4 Reset against the original |
| `reset_returns_to_the_cleaned_text_not_the_original` | Reset lands on `clean(original)` | E7-3/M2, M3 |
| `the_result_is_what_the_queue_writes` | UTF-8 and UTF-16LE files through `clean::clean_one`: the file written equals the window's result | E7-3/M1, M3 |
| `a_text_with_nothing_to_clean_opens_on_itself` | nothing to clean: result = original, `compare-same` | — |

E7-4 (`report.rs`; the endings are real cleans of real fixtures):

| test | protects | mutation |
|---|---|---|
| `every_outcome_has_its_three_shelves` | cleaned, nothing found, partly, not cleaned — every section has a line, the third shelf at least three, the pixels' claim first exactly on a picture | E7-4/M1 drop the shelf, M4 no pixels' claim, M6 an empty shelf vanishes |
| `a_text_is_said_by_what_was_removed_and_what_was_kept` | U+200B by name on Verifiable, the Cyrillic a and why on Best-effort, the result's path | — |
| `the_markdown_copy_carries_nothing_layer_a_would_remove` | en, ru, de over every ending: no isolate, `inspect` finds nothing, five sections | E7-4/M7 a U+200B in the copy |
| `the_copies_are_the_json_and_plain_markdown` | Copy JSON = `to_json()`; Copy as Markdown plain though the window is `Rendering::Ui` | E7-4/M2 the copy in `Rendering::Ui`, M3 a JSON of our own |
| `a_claim_with_no_sentence_is_shown_in_its_own_words` | an unknown id is shown, never dropped | E7-4/M5 |
| `a_thing_never_read_has_shelves_and_no_json` | no invented JSON | — |

E7-5 (`panel.rs`'s pure functions; `clean.rs` over the committed fixtures):

| test | protects | mutation |
|---|---|---|
| `the_look_agrees_with_the_clean` (`clean.rs`) | the look reads like the clean and writes nothing: a text's count is characters (two U+200B is 2) and equals what the clean removed; nothing ⇔ `NothingFound`; a kept look-alike ⇔ `Partly(Kept)`; torch, C2PA and confetti said as a mark, metadata and nothing, and cleaned accordingly; a TIFF not looked at | E7-5/M4 rows counted |
| `a_number_is_handed_out_once` (`clean.rs`) | D280 | E7-5/M8 |
| `the_findings_line_says_what_a_look_found` | every case's English line; "Looking…" before; not examined said with the metadata either way | E7-5/M5, M6 |
| `every_look_reads_in_every_language` | en, ru, de: every case resolved and distinct | — |
| `only_a_finding_is_painted_as_one` | the foreground colour only for something found | — |
| `clean_is_offered_only_when_something_is_left_to_clean` | what Clean cleans (cleanable, not done, in order) and when it is offered | E7-5/M1, M2, M3 |
| `the_panel_says_only_rewriting_is_not_here` | `panel-pending` in every language: rewriting, no old sentence, no epic number | E7-5/M7 |

The panel itself is not built in a `#[gpui::test]`: its window asks AppKit
for a floating level and a destination as it opens (`afloat`,
`drop::accept`), the macOS test-window trap of E7-2, and splitting that out
is more than this step's change. What the window paints from these
functions is checked live.

E7-6 adds its rows when it lands.

## §6 Acceptance

1. Every gate of the task green (or listed as not run, with the reason);
   every mutation red.
2. No new dependency.
3. Nothing in a window says an epic number; every report shows its third
   shelf.
4. The live check (`docs/plan/reports/E7-windows-clean-live-check.md`) runs
   on the host by clicks alone.

## §7 Out of scope

Rewriting with a model in the windows (E4-6b); the source editor with
invisible characters as badges (S7.2); a streamed result (S7.3); the
Inspector (S7.5); a history table; folders and archives expanded (Q-D2); the
tray's Clean Clipboard; cleaning and saving in Compare; TIFF, HEIC, AVIF;
any change to the CLI, MCP or the pixel and picture libraries.

## §8 Basis

OV §6.1–6.2, S7.1, S7.6; A §7.4; `docs/architecture/retention.md` rules 1–7;
`docs/architecture/cli.md` (the behaviour matched); D28, D81, D91 (the
queue's destinations), D130–D136 and D156 (pictures on the surfaces),
D221, D238, D244, D248, D250 (what a picture report says).

## §9 Decisions (from D260)

| # | decision | why |
|---|---|---|
| **D260** | **Nothing found is nothing written.** A text Layer A did not change and found nothing suspicious in, and a picture with nothing AI removed, no mark restored, nothing left and its pixels examined, write no `.cleaned` copy. | A copy identical to its input under another name says "cleaned" about a file nobody touched; the row says "nothing found" instead. |
| **D261** | **A result never replaces a file already there.** `Beside` and `Into` refuse with `NotCleaned(Exists(path))` and leave that file byte for byte (a dangling link counts as there). Replacing it is an explicit second action on the row (E7-2), never a default. | The product never overwrites a file it did not write in this run without being asked. The alternative, a numbered name, is owner question 4. |
| **D262** | **A result identical to its input is never written, whatever the verdict.** A picture whose mark was seen and refused (under alpha 0, say) with nothing removed comes back as its own bytes: `Partly(Mark)`, nothing written, the row saying a mark is left. | D260's reason applies to every verdict. The CLI writes the identical file beside and exits 3; the window says the same thing without the file. |
| **D263** | **A text's verdict is by change, then by suspicion.** Changed is `Cleaned` and written — a soft hyphen removed from a text not otherwise suspicious included; unchanged and not suspicious is `NothingFound`; unchanged and suspicious (a homoglyph kept at the defaults, nothing removed) is `Partly(Left::Kept)` and nothing is written. | The CLI writes every changed text; Compare (E7-3) shows `clean(original)`, so the queue must write the same text or the two disagree. A kept homoglyph is found and deliberately not removed (Q-A1): "nothing found" would be false, "cleaned" would be false. |
| **D264** | **`PICTURE_LIMIT` is 64 MiB**, refused with its size on the `stat` and again on the read. Text keeps `compare::TEXT_LIMIT` (8 MiB). | A picture is decoded whole and its raster copied once to be restored: a 64 MB PNG can be a quarter of a gigabyte of samples held twice, one clean at a time. The CLI has no window and no limit. |
| **D265** | **A kept copy is made only when there is a result, and before it is written.** Nothing is kept for `NothingFound`, a refusal, or a text `Partly`; the copies go into `<kept>/<yyyymmddThhmmss>-<row>/` (UTC, so the sweep's arithmetic needs no zone) as `original.<ext>` — the bytes as they arrived, a paste as UTF-8 — and `result.<ext>`, before the destination is written, so a copy that cannot be made stops the clean. The result name invented for nameless bytes is in local time, because a person reads it. | Keeping exists so the original can be brought back once the result has replaced it; with no result there is nothing to bring back from. |
| **D266** | **`clean_one` takes no `Homes`, and its log line carries paths as `Elided` shapes.** The plan already names both folders (`Written::Into`, `Kept::in_`). The task asked for "the paths" in the log line; the CLI and `CLAUDE.md` ("Diagnostics go to a file, and the document never does") log a path's shape, never the path, and so does this. | One source for each folder; a file name can be the document's title. |
| **D267** | **The sweep never creates the kept folder**, runs once per launch on the background executor and after each kept write, and removes only a directory named `yyyymmddThhmmss-<digits>` (not a link) whose time is more than its period ago. | Both switches off is the default and `kept/` must not appear; anything not shaped like ours is somebody else's. "Once a day while running" (`retention.md`) is not built: a launch and every keep are the moments copies are added. |
| **D268** | **`--clean=<path>` asks for a row whatever it is**; the menu's Clean and Clean all offer only what `cleanable` accepts. A TIFF named on the command line lands, is asked, and comes back `NotCleaned(NotCleanable(NotYet(Tiff)))` with its badge saying why. | A flag the person typed is a request to be answered, not a menu to grey; the answer costs no read (`clean_one` refuses on the intake). |
| **D269** | **A greyed Clean says why under its label**, as a second, muted line inside the menu item, not as a tooltip: gpui-component's `PopupMenuItem` has no tooltip. The same sentence is the Status badge's tooltip for a waiting row that cannot be cleaned. | The task asked for the reason in a tooltip; a menu item has none, and a reason nobody can see is no reason. |
| **D270** | **"Replace the existing result" is `clean::replace_one(…, existing)`**: the row is cleaned again with the plan taken when that clean starts, and only the file the first clean refused (`Refusal::Exists(path)`) may be written over — through the same atomic rename, never the source (`SameFile` first). If the page moved the result elsewhere meanwhile, a file there is refused as ever. The outcome says `replaced`, and the row "Written over the existing x.cleaned.md". | D261 stays the rule; the replacement is one named file, asked for by a person, said afterwards. |
| **D271** | **`toolbar-help-pending` is rewritten in E7-2**, not E7-6: it sits beside Clean all and said "cleaning from this window is not in this version yet". It now says what Clean and Clean all do and that rewriting with a model is not in the windows. | A window must not say the opposite of the button next to it for four steps. |
| **D272** | **Compare's Reset is offered once the result differs from the cleaned text**, not from the original, and reads "Back to the cleaned text". | Once the result is `clean(original)`, differing from the original is the normal state of any marked text; a Reset that puts back what is already there is a button that does nothing. |
| **D273** | **The cleaned text is made by `Subject::read`, in the read's background task, and kept by the view**; Reset puts that text back and never cleans again. | The GPUI thread never runs Layer A, not even on Reset, and an 8 MiB text is cleaned once per window. |
| **D274** | **The Report dialog is the shell's, not the table's**: the queue emits `QueueEvent::Report(id)` from a deferred click, and the shell paints `report::ReportView` over the whole window at the walk-through's priority. | A backdrop painted inside the table would leave the toolbar (Clean all, Paste) clickable under a modal; the shell's tree covers the window, and nothing goes through `Root`. |
| **D275** | **The shelf mapping** (`docs/architecture/queue.md`, "The report"): text — removed characters, normalizations and the Unicode version verifiable; kept characters and why best-effort. Picture — removed metadata with its signals, the second inspection, proved marks and exact restorations verifiable; every inexact restoration and its reasons (the residual as a mean), holes, outline, texture, refusals, the pixels not examined, "no visible mark this version knows", marks left, metadata kept, EXIF, rotation and the re-encoding best-effort. Third shelf: the report's own ids in order, a picture's with `invisible-pixel-marks` first. | Verifiable is what this build checked and anyone can check again; whatever a catalogue, a fit or a lossy store bounds is not. |
| **D276** | **The window's sentences take a `wording::Say`**, and the dialog is handed both — the window's and the copy's (`ReportView::with_words`). | One sheet, two renderings, and a test can give the window a real `Rendering::Ui` (the process default in tests is PlainText) to prove the Markdown copy is not in it. |
| **D277** | **A thing refused before it was read has a report with no JSON**: Verifiable says it was not read, Best-effort says nothing is on it, the third shelf is its kind's (a picture's when it arrived as one), and Copy JSON is greyed rather than copying an invented or empty report. | `to_json()` is the library's; there is none, and a placeholder would be a report nobody produced. |
| **D278** | **The panel looks at what it lists, once per drop, as a clean would read it.** `clean::inspect_one` reads the same bytes under the same limits, decides again from them, and runs `wipemark_core::inspect` or `wipemark_picture::inspect` — never a clean, never a write. Only the four listed things are looked at, one at a time on the background executor; a look overtaken by the next drop stops. A text's count is characters (the findings' `count`s), the number its clean then says it removed or replaced; a kept look-alike with nothing to remove is said, because the clean will call it partly clean. | The line is a promise about what Clean will do; read another way, the two could disagree. Reading a hundred files nobody can see would be work for nothing. |
| **D279** | **The panel's Clean cleans every caught thing that can be cleaned and has not been**, the listed and the counted alike, in arrival order, one at a time, each with the plan taken at its own start — the queue's road. It is greyed while a clean runs and once nothing is left; absent when nothing caught could ever be. A drop during a clean does not stop it: what was asked for is finished, its outcomes are logged and no longer shown, and the new drop's Clean waits for it. | One clean at a time across the window; a half-done batch would leave some results written and the panel not saying which. |
| **D280** | **One counter numbers the queue's rows and the panel's cleans** (`clean::number`). | A kept directory and an invented name are `<time to the second>-<number>`: with two counters, a queue row and a panel clean in the same second would share a kept directory and the second would overwrite the first's copy of an original. The queue's ids now skip the numbers the panel used. |
