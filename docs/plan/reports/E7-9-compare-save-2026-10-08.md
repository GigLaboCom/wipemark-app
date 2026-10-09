# E7-9 — saving an edited result from the Compare window, with autosave: report

- **Task:** `docs/plan/E7-9-compare-save.md` (Watchword FILE
  `wipemark-task-compare-save-2026-10-08`, the coordinator, 2026-10-08): a
  **Save** for the Compare window's edited result, and an **autosave** on by
  default (`docs/plan/README.md` §7 E7, "Build. Saving an edited result,
  with autosave").
- **Branch:** `e7/compare-save`, from `feat/e0-e6-shell` at `4b54f52` + the
  task commit `a0786f5`. `feat/e0-e6-shell` moved to `08a1540` (E7-8, which
  touches `compare.rs`'s scrolling) before the work began; it was merged in
  first (`52ddcc5`), both sides kept. It moved again while the work was
  being finished (E8-1 merged, `3cc86ca`) and was merged in once more
  before the gates (`7e2f8bf`): two conflicts, both additive — `config.rs`'s
  test imports (the union) and `dialog.rs`'s tests (both kept).
- **Commits:** `7dfb94c` (everything: the code, its tests, the docs, the
  red-check and gates scripts); `7e2f8bf` (the second merge of
  `feat/e0-e6-shell`); `ba000c2` (clippy, which the first gates run caught
  after `7e2f8bf` had been pushed: a `type` for the link's function, and
  four borrows that needed no `mut` — the CI runs on `7e2f8bf` were
  cancelled); then this report.
- **Built in a Docker container** with no display and no GPU: no window was
  opened. What only a window shows is in the checklist at the end.

## The requirements

| | what | done | where | tests | removed locally → red |
|---|---|---|---|---|---|
| S1 | where Save writes, by what the window was opened on | yes | `compare/save.rs` `save_target` (pure, the table below), `Target`, `NoTarget`; `compare.rs` `Made::CleanedTo`, `CleanedTo`, `Subject::read` (the result read back with its encoding and stamp), `begin`; `queue.rs` `cleaned_for`, `subject_of`, `Row::edited`, `told_by_compare`, `link_to`; `clean.rs` `save_one`, `saved_of`, `text_and_encoding_of`; `cleaner.rs` `Job::edited`, `ask_with`; `wipemark-queue` `Queue::save_text` | `where_save_writes_by_what_the_window_was_opened_on` (the table); `a_save_writes_over_the_result_and_never_the_original`, `a_save_through_a_symbolic_link_is_refused`; window: `a_cleaned_file_is_saved_over_its_result_in_its_encoding` (UTF-16LE), `an_unwritten_row_saves_as_a_clean_would` (a taken name asked, then replaced), `in_place_a_save_never_reaches_the_original`, `a_rewritten_paste_is_saved_into_its_row`, `a_cleaned_paste_is_saved_into_its_row`; `clean::tests::a_save_that_cleans_writes_the_edited_text_by_the_clean_s_plan`, `…_keeps_every_rule_of_a_clean`; queue: `a_cleaned_row_opens_compare_on_its_result_and_a_paste_keeps_its_edit`, `a_save_that_cleans_moves_the_row_and_marks_its_journal`; `wipemark-queue` `a_saved_edit_replaces_a_done_rows_text_and_nothing_else` | the original arm of `save_target`; the original, link and atomic-write checks of `save_file`; the encoding; the edited text handed to the line; the edited text in `run`; D262 in `saved_of`; `save_text`'s state check; the row's refusal; `told_by_compare`'s text check; Copy the result's edit; the in-place original — each red |
| S2 | Save, ⌘S, greyed with its reason; on the background executor; changed on disk asked; Reset after a save; the journal records the edit | yes | `result.rs` `Offer`, `offer`, `press_offer` (the window's action, dispatched like its key); `compare.rs` `SaveCompare` (`secondary-s`), `save_pressed`, `save_unavailable`, `offer_save`, `ask_changed`, `keep_theirs`; `compare/save.rs` `Stamp`, `unchanged`, `save_file`, `save_item`, `Force`; `dialog.rs` `Choose`, `Pick`; `wipemark-store` `Outcome::edited`, `Journal::mark_edited`; `journal.rs` `Writer::edited`, `clean_end` | `the_strip_s_save_is_the_window_s_action`; `a_file_changed_on_disk_is_asked_about_not_overwritten` (Cancel, then Keep theirs); `a_rewritten_paste_is_saved_into_its_row` (a row changed under it is asked about); `a_file_is_unchanged_only_while_its_bytes_are`; `enter_on_a_three_way_question_is_the_safe_choice`; `an_edit_is_a_mark_in_the_outcome_and_nothing_else_moves` (store) | the window's `SaveCompare` listener; the strip's dispatch; the changed check; the bytes compare of a same-length change; the item's digest check; Keep theirs' resume; Enter as Overwrite; the store's mark through `Entry`; `clean_end`'s mark; the writer's mark — each red |
| S3 | `compare.autosave`, on by default; ~1.5 s of quiet; one save in flight, the last edit wins; on close; off → close asks; a failed save stops autosave and says why; banners say what happens | yes | `compare/save.rs` `Saver`, `QUIET`; `compare.rs` `Comparison::autosave`, `typed`, `after_save`, `may_close` (the close button and ⌘W), `ask_close`, `where_saved`; `config.rs` `COMPARE_AUTOSAVE_KEY`, `read_comparison`, `write_compare_autosave`; `settings.rs` `Setting::CompareAutosave`, `save_as_you_type`, `autosave_switch`; the catalogue (en/ru/de): `compare-pending`, `compare-rewritten-banner`, `compare-help-close`/`-asks`, `compare-help-save`, `compare-help-settings`, `compare-save-*`, `settings-compare-autosave-*` | `the_last_edit_wins_and_a_failure_stops_autosave` (pure); `autosave_waits_for_quiet_and_the_last_edit_wins` (a save held in flight by a test gate); `a_failed_save_stops_autosave_and_says_why`; `closing_saves_with_autosave_and_asks_without`; `the_compare_rows_default_to_words_and_survive_a_restart`; `an_unusable_compare_row_falls_back_without_being_rewritten`; the i18n gates | `again` dropped (pure and window); the edit-number check; `QUIET` shortened; `stopped` not set on a failure; `may_close` saying yes; the default off; the unreadable row read as off — each red |
| S4 | the line under the result | yes | `compare.rs` `save_status`, the window's foot; `compare-status-*`, `compare-not-saved-*` | asserted in the window tests above ("Unsaved changes", "Saving…", "Saved …", "Not saved: …") | — (wording; the tests read it) |
| S5 | docs | yes | `compare.rs` module docs ("Saving"), `compare/save.rs` module docs (the S1 table), `result.rs` ("One button is the window's"); `docs/architecture/compare.md` "Saving an edited result (E7-9)"; `retention.md` (what the windows write); `queue.md` ("Compare saves into the row") | — | — |

### The S1 table

| the window was opened on | it opens on | Save writes | Reset returns to |
|---|---|---|---|
| a row nothing was written for — waiting, nothing found, refused, failed — or `--compare=<path>` | `clean(original)` | a **Clean of the pane's text**, in the one line of cleans, by the plan the row's Clean takes when it starts; a taken name is asked about (Overwrite = Replace that one file) | `clean(original)` |
| a cleaned row whose result is a file | the file | over that file, atomically | `clean(original)` |
| a row cleaned in place | the source's name (the original on the left is the file set aside) | over the source's name, never the original set aside | `clean(original)` |
| a cleaned paste | the row's text | its row, in memory (Copy the result, the next Compare) | `clean(original)` |
| a rewritten row (beside or in place) | the rewrite's file | over that file | the text the window opened on |
| a rewritten paste | the batch queue's row | that row (`Queue::save_text`) | the text the window opened on |
| a result that is the original's own file, nothing set aside | the file | nowhere: Save greyed, the banner says why | — |

Never the original (by name and by inode), never through a symbolic link,
in the encoding the text arrived in.

The red checks are `docs/plan/reports/E7-9-compare-save-2026-10-08-red.py`
(the shape of the E7-8 script: each edit applied, the named test run, the
file restored byte for byte). **34 of 34 red.** The first full run, over 32
checks, was 31 of 32: *autosave waits for the quiet, not a moment less* came
back **green**, because the window test timed its waits off the very
constant the check shortened. The test now waits a written 1.3 s (nothing
saved) and then 0.3 s more (saved), and goes red. Two checks were added
after that run, for the rule that a Save that cleans asks its row first
(below), and the three were run again: 3 of 3 red.

One gap found while the checks ran, and closed before the commit: a Save
that cleans, from a window opened on a row the batch queue was rewriting,
would have written `name.cleaned.ext` beside a file mid-rewrite — the row's
own Clean is greyed then. The window now asks its row first
(`Told::Cleans`); a row in the line of cleans, queued or running a rewrite,
says no, and the line under the result says the document is being cleaned
or rewritten (`a_save_that_cleans_waits_while_the_row_is_rewritten`).

## Decisions

- **D410 — where Save writes is where the result lives.** Over the result's
  own file — the clean's `name.cleaned.ext` or results-folder file, the
  rewrite's `name.rewritten.ext`, or the source's name after an in-place
  clean or rewrite — through `inplace::write_atomically`; into the batch
  queue's row for a rewritten paste (`Queue::save_text`: only a done item
  whose stored result holds a text); into the main window's row for a
  cleaned paste. Never the original (`save_target` by path, `save_file` by
  path and `same_file`), never through a symbolic link (D287's rule: only
  the last name is the link's), in the encoding the result's file is in.
  A result that *is* the original's file with nothing set aside (the CLI's
  `--in-place --no-original`) has no target: Save is greyed and says so.
- **D411 — nothing written yet: Save is a Clean of the pane's text.** The
  task's first option, not "Save only after a Clean": the person who
  opened Compare on a waiting row, edited the cleaned text and closed the
  window would otherwise lose the edit, which is what the owner asked to
  stop. `clean::save_one` is `clean_one` with the result given — the same
  read and refusals, the plan taken when it starts, the kept copies, a taken
  name refused unless named — asked of the application's one line under the
  row's own id (`Cleaner::ask_with`), so the row's status and journal move
  as a Clean's do, and a Save and a Clean of one row are never two writes.
  Its verdict is `Cleaned` when the text differs from the source, and
  nothing is written when it does not (D262 kept). Save is offered on an
  unwritten result even unedited (it is a Clean of what the pane holds);
  autosave saves only an edit. A `--compare=` window has no row: its number
  is its own (`clean::number`).
- **D412 — the window tells the row through a link its owner hands in.**
  `compare::Link` is a function the queue gives `compare::open` (`link_to`);
  `compare.rs` still names no queue. `Told::Saved` after a save over a file
  or the batch queue's row, `Told::Text` with a cleaned paste's text, which
  the row keeps in memory (`Row::edited`) for Copy the result and the next
  Compare, gone with the next clean of the row, and which a row whose result
  is not a text refuses. A `--compare=` window writes a journal row of its
  own on its first Save that cleans (origin `launch-flag`, as `--clean=`'s
  rows are) and marks it on later saves — every document has a status,
  whoever asked.
- **D413 — changed on disk is the size, then the bytes, and it asks.** A
  save over a file compares the file with the stamp of what the window last
  read or wrote there — its length, then a digest of its bytes — never the
  modification time alone, which a file system may keep to the second; a
  file that is gone has changed. A batch queue row is held to its text's
  digest. A change is asked about in a `dialog.rs` overlay (the new
  `Choose`): Overwrite, Keep theirs (the file's text in the window, the
  edits let go, the file the result's home, autosave on again), Cancel
  (nothing saves on its own until Save asks again). Enter is Cancel. A
  taken name met by a Save that cleans asks the same question. The check and
  the write are two steps; a write landing between them is not caught, and
  nothing locks the file.
- **D414 — Reset goes back to what was made, and is saved like an edit.** For
  a clean, to `clean(original)` (always recomputable; the text Clean
  writes); for a rewrite, to the text the window opened on (the rewrite as
  delivered, or as last saved before — the model's words are not kept once
  a save replaced them). Not to "the last saved text", the task's
  suggestion: with autosave on by default that is the text of a second and
  a half ago, and Reset would undo nothing worth a button. The tooltips say
  which, and that it is saved as edits are.
- **D415 — autosave: read at opening, 1.5 s of quiet, one save at a time.**
  `compare.autosave` is read when a window opens, like every Compare row
  (D385's reason). An edit numbers itself; a quiet a later edit overtook
  saves nothing; a save asked while one runs (by a quiet, Save, a close)
  runs after it with the text as it stands then. A save that cannot write
  stops autosave and says why until a Save succeeds; a question stops it
  until answered. Closing: with autosave, save then close, and stay open if
  the save could not write; without, ask Save / Discard / Cancel (Enter is
  Save). The close button and ⌘W take the same road.
- **D416 — Save on the strip is the window's action, offered.** `result.rs`
  stays a component that names no file: the window hands it a
  `result::Offer` — its own action, the key context it is bound in, the
  label, and why it is greyed — and pressing it focuses the editor and
  dispatches that action, the road ⌘S takes. One button, at the head of the
  strip, apart from the editor's operations.
- **D417 — the journal records that a result was edited, and when.**
  `entry.outcome.edited` (milliseconds since the epoch), never what.
  `journal::clean_end` sets it for a Save that cleans; every later save sets
  it through the row's writer (`Writer::edited` → `Journal::mark_edited`),
  which patches that one field as JSON values and keeps the rest of the
  entry, a field a newer build wrote included.
- **D418 — a cleaned row's Compare opens on its result as written.** Once a
  clean put a result somewhere, the window opens on that file or that
  row's text — so edits saved there before are what it shows, and a second
  window never overwrites them with a fresh clean — and, in place, the
  original on the left is the file set aside (the old `Made::Cleaned`
  read the source's name, which by then held the result). A clean recorded
  in the journal by an earlier session, with a file, opens the same way.
- **D419 — what a save does not do.** Layer A does not run over an edit: the
  person's text is written as typed. A save does not refresh the copies
  `kept/` holds (those are what the clean made). A cleaned paste's saved
  text lives in its row and nowhere on disk, as the cleaned text does.

## Gates

Once, at the end, on `ba000c2`, all `--locked`, by
`docs/plan/reports/E7-9-compare-save-2026-10-08-gates.sh` (every log kept):

| gate | exit | counts |
|---|---|---|
| `rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')` | 0 | |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | |
| `cargo test --workspace --locked` | 0 | 1748 passed, 0 failed, 7 ignored |
| `scripts/check-dep-direction.sh` | 0 | |
| `scripts/check-gpui-pin.sh` | 0 | |
| `cargo check --workspace --no-default-features --locked` | 0 | |
| `cargo check --workspace --features local-llama --locked` | 0 | |
| `cargo test -p wipemark-app --features local-llama --locked` | 0 | 700 passed, 0 failed, 2 ignored |
| `cargo test -p wipemark-engine --features local-llama --locked` (CI's, not in the task's list) | 0 | 40 passed, 0 failed, 1 ignored |

The first run of the same script, on `7e2f8bf`, stopped at clippy (six
findings in the new code: `type_complexity` twice, `needless_pass_by_ref_mut`
four times, then two in the tests); fixed in `ba000c2`, and the whole list
run again from the top. While iterating, only targeted tests were run.

## CI

On `ba000c2`, both green:

- **gate** — https://github.com/GigLaboCom/wipemark-app/actions/runs/37824175647
  — `gate (fmt, clippy, test, deps, features)`: success; `macos (clippy,
  tests, llama-native prebuilt with Metal)`: success; `native (llama.cpp
  prebuilt + Vulkan, model-free)`: success.
- **llama-source**, started by hand (`gh workflow run llama-source.yml
  --ref e7/compare-save`) because its `macos` job lints the workspace and
  the run the merge started was on `7e2f8bf` —
  https://github.com/GigLaboCom/wipemark-app/actions/runs/37824216264 —
  `linux (llama.cpp from source + Vulkan; bindings against the archive's)`:
  success; `windows (prebuilt, then from source with MSVC)`: success.

The two runs on `7e2f8bf` (37823918885, 37823918725) were cancelled: that
commit fails clippy. The commit carrying this report changes only this
file.

## The window, for the owner (no display here)

1. Drop a marked `.md`, double-click the row: Compare opens on the cleaned text; the banner's second line names `x.cleaned.md` "where Clean would", and says edits save after typing stops.
2. Type a word; within about two seconds the line under the result goes Unsaved changes → Saving… → Saved hh:mm, and `x.cleaned.md` appears beside the file with the edit; the row says cleaned.
3. Edit `x.cleaned.md` in another editor, then type in Compare: the Overwrite / Keep theirs / Cancel question appears over the window, Enter cancels; Keep theirs puts their text in the pane.
4. Settings › Compare › "Save edits as they are typed" off, open a new Compare, edit, press ⌘W: Save / Discard / Cancel; ⌘S and the strip's Save (first button, floppy glyph, its tooltip shows the shortcut) save.
5. Paste text, Clean it, Compare it, edit: Copy the result in the row gives the edit; reopening Compare shows it.
6. Rewrite a row, Compare, edit: `x.rewritten.md` takes the edit; "Back to the rewritten text" returns to the text the window opened on and is saved.
7. Check every new sentence in ru and de fits (banner second line, the line under the result, the two questions, the Settings row).

## Wanted edits (not made: `CLAUDE.md` and `docs/plan/README.md` are the coordinator's)

- **`CLAUDE.md`, "The Compare window is three things kept apart"**: replace
  "Editing the result saves nothing and closing writes nothing, and the
  banner says both." with: "An edited result is **saved where it lives**
  (E7-9, D410–D419): Save at the head of the result's strip and ⌘S, and,
  with `compare.autosave` (on by default, read when a window opens),
  a moment after typing stops and as the window closes. `compare/save.rs`
  is the rule — over the result's own file, into the row of a paste, or,
  when nothing was written yet, a Clean of the pane's text in the one line
  of cleans (`clean::save_one`, `Cleaner::ask_with`) — never over the
  original, never through a symbolic link, in the encoding it arrived in;
  a file changed on disk since the window read it is asked about
  (Overwrite / Keep theirs / Cancel), never overwritten unasked. The window
  tells its row through a `compare::Link` the queue hands in, and the
  row's journal entry marks the edit — when, never what (`outcome.edited`).
  Reset is an edit and is saved like one." Also in the file table:
  `compare.rs` gains "saving an edited result (`compare/save.rs`)", and
  `result.rs` "and the window's Save, offered to its strip".
- **`CLAUDE.md`, the top paragraph**: Compare saves edits now.
- **`docs/plan/README.md` §7 E7**: "Build. Saving an edited result, with
  autosave" — done (this report, D410–D419). §4: rows D410–D419 as above.
  §2: a row for `e7/compare-save`.

## Left open

- **Quitting the application** (⌘Q, the tray's Quit) with an edit typed
  less than a second and a half ago: GPUI asks no window whether it may
  close on quit, so that edit is not saved and nothing asks. Closing the
  window — its button or ⌘W — saves or asks as above.
- **The changed-on-disk check and the write are two steps**: a file
  written by another program between them is replaced. Nothing locks a
  file; the window narrows the gap to one stat and one read.
- **A Save that cleans which finds a name taken and is answered "Keep
  theirs"** makes that file the result's home; the row's own status still
  says its clean was refused for that file (Replace offered), because the
  row's clean was. The window and the row then disagree about what the
  result is until the row is cleaned again.
- **A `--compare=` window's journal row** appears in the main window's
  table on its first save, as a launch flag's row; it is not a row the
  window was opened from, and a second `--compare=` of the same file is a
  second row.
- Not seen on a screen: every sentence's length in ru and de, the
  question's buttons, the strip's new first button (see the checklist).
