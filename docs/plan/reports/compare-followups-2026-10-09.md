# Compare's follow-ups — report (2026-10-09)

The task: `docs/plan/compare-followups.md` — Watchword FILE
`wipemark-task-compare-followups-2026-10-09` (C1–C8, C12, C13, C15;
D440–D448) with its addendum `wipemark-task-compare-followups-c16-2026-10-09`
(C16; D449) as its last section, both by the coordinator, 2026-10-09.
Branch `fix/compare-followups`, from `feat/e0-e6-shell` at **`58624fc`**
(the coordinator's correction: not `f85a7ba`; `fix/models-pipeline-followups`
and `e7/compare-mirrored` merged since). `feat` did not move while the round
was written (checked again before the gates). Built in a Linux container
(Debian 12, aarch64): no window, no GPU, no model file. Decisions
**D440–D449**; D450–D455 and D460–D467 untouched.

Every file:line in the task was at `f85a7ba`; each place was found again by
the function or test the task names. None was gone, and none already did
what its item asks, except what the task itself drops (C9–C11, C14) and
C12's road (a test, no code — below).

## The items

| # | what | state | commit | test | removed locally to see red |
|---|---|---|---|---|---|
| C1 | quitting within 1.5 s of an edit loses it | done (D440) | `6d3bd21` | `compare::tests::quitting_saves_an_edit_autosave_has_not_reached_yet`, `…::quitting_with_autosave_off_writes_nothing`, `…::quitting_never_writes_over_a_file_changed_on_disk` | `C1`: the `on_app_quit` registration; `C1-stamp`: the quit's write without the stamp |
| C2 | a stale window's Save says "refused" over a good result | done (D441) | `6d3bd21` | `compare::tests::a_row_cleaned_since_the_window_opened_is_asked_about_not_refused` (a real queue and a real window) | `C2`: the `home` question |
| C3 | `Told::Saved` marks whatever entry the row has now | done (D442) | `6d3bd21` | `queue::rewrite_tests::a_save_marks_the_entry_it_was_opened_on_and_not_a_later_one`; store `journal::tests::an_edit_mark_lands_only_on_the_action_it_names` | `C3`: the row's check (`holds`); `C3-sql`: `AND action = …` in both statements |
| C4 | D413's check and write are two steps | done (D443) | `6d3bd21` | intake `inplace::tests::write_atomically_if_publishes_only_while_still_holds`; `compare::save::tests::a_write_after_the_staging_is_not_written_over` | `C4a`: a `false` from `still` renames anyway; `C4b`: the check back before the staging |
| C5 | `edited_at` replaces an entry it cannot read | done (D444) | `6d3bd21` | store `journal::tests::an_entry_this_build_cannot_read_is_left_as_it_is_by_an_edit_mark` (`[1,2]`, `"x"`, `{"outcome":7}`, and not JSON at all) | `C5`: `edited_at` as it was |
| C6 | autosave off: the close question is silently replaced | done (D445) | `6d3bd21` | `compare::tests::a_close_question_is_not_replaced_by_a_changed_file` | `C6`: `ask` putting its question over a standing one |
| C7 | closing with nowhere to save drops the edits unasked | done (D446) | `6d3bd21` | `compare::tests::closing_with_nowhere_to_save_asks_and_can_copy` | `C7`: the `target.is_err()` short-cut back |
| C8 | *Cleaned* over an edit Layer A never ran over (owner question D419's) | done, **built at the default (D447); the owner may switch** | `6d3bd21` | `wording::tests::an_edited_result_never_reads_as_plainly_cleaned` (pure), `compare::tests::a_save_that_cleans_says_then_edited_on_its_row` (window + queue); the i18n gates | `C8`: `edited` handing back its argument |
| C12 | "Send away" on arrival has no test | done — **no code change**; the test found no bug | `6d3bd21` | `queue::rewrite_tests::a_drop_that_would_be_sent_away_is_asked_about_first` | `C12`: the Rewrite arm pushing straight away |
| C13 | Report… greyed, silently, for a journal row | done, **built at the default (D448); the owner may switch** | `6d3bd21` | `queue::tests::report_of_a_journal_row_is_greyed_with_its_reason` (the pure `why_no_report`) | `C13`: `why_no_report` always `None` |
| C15 | `assets/tray/README.md` describes the choice D342 superseded | done; the script not run (it needs Inkscape) | `41c0f43` | — (no test, as asked) | — |
| C16 | the two panes level in the frame a scroll bar is dragged | **measured: level in the same frame; no fix** (D449) | `6d3bd21` | `compare::tests::both_panes_stand_level_in_the_frame_a_scroll_bar_is_dragged`, `…::with_the_result_wrapped_it_leads_a_frame_late_and_no_later`, `…::with_the_row_off_a_drag_moves_one_pane` | `C16`: the editors' observers no longer look; `C16-wrap`: no look at the end of a frame |
| C9–C11 | E7-7's L2–L4 | **dropped, as the task says** — closed by E7-8 | — | L2: `two_panes_moved_in_one_frame_end_where_the_result_put_them` (now `compare.rs:5101`); L3: the clause is gone from `settings-compare-follow-description` (`wipemark.ftl:510`); L4: `compare-help-settings` (`:417`) reads "chooses five things" since E7-10 (was "four" after E7-8) | — | — |
| C14 | `build.rs` reads `CARGO_MANIFEST_DIR` with `env!` | **dropped, as the task says** — fixed by `fe5f5fe` | — | `apps/wipemark-app/build.rs:30` is `env::var_os("CARGO_MANIFEST_DIR")`, with the comment at `:23` saying why; no `env!` left | — |
| plan text | §2 row 1c, §2.1 row 7e, §4 D440–D449 (D413, D419 pointing on), §5 D419's and D448's, §7 E4, E7, E10 | done | `6e87177` | — | — |

The task text is the branch's first commit (`d65cd09`,
`docs/plan/compare-followups.md`, C16 appended as its last section).

Every red check is in `compare-followups-2026-10-09-red.py` beside this
report, re-runnable; the run is under "Red checks".

### C1 — the quit flush (D440)

`CompareView` registers `cx.on_app_quit(|view, cx| view.at_quit(cx))`.
`flush_at_quit` writes only while the window autosaves and is not stopped,
no question stands, the pane differs from what was last saved, and the home
is `Target::File` with `Seen::File(stamp)` or `Target::Item` with
`Seen::Item(digest)` — **synchronously, in the callback**, through the same
`save::save_file` / `save::save_item` a save uses, by the stamp held; a home
that changed (or any other refusal) writes nothing and logs one line, the
path `Elided`. On a write the row is told as `tell_saved` tells it, and the
quit's future awaits the journal writer's `flushed()` receiver within GPUI's
200 ms. `Seen::Nothing` and the new `Seen::Unread` are never written at quit:
there `None` would mean Overwrite.

Two things the task did not spell out:

* **Which writer.** A window opened from a row tells the row, and the mark
  goes through the *queue's* `journal::Writer`, not one the window holds. The
  window reaches it through a third field on `compare::Link`, `flushed`
  (the task named one new field, `home`); `queue::link_to` answers it from
  the queue's writer. A `--compare=` window awaits its own writer.
* **A save under way.** Its write runs on the background executor, and the
  window's side of its end runs on the GPUI thread, which no longer turns
  once `App::shutdown` blocks on the quit futures. So the end is signalled
  from the background task itself (`Saving::in_flight`, a one-shot sent right
  after the write), and the quit awaits that. The file holds the edit; that
  save's journal mark, which the window's side writes, does not land — said in
  `compare.md` and D440.

`Writer::flushed` is a `Command::Flushed(Sender<()>)` answered in turn on the
writer's thread; a writer whose thread is gone disconnects at once.

### C2 — a row cleaned since the window opened (D441)

`compare::Link::home` (type `Homed`) answers `cleaned_for(row)` as
`(CleanedTo, set_aside)`. In `begin`'s `Target::Clean` arm, before
`Told::Cleans`: with a home, the window retargets — `Target::File(path)`
(and, for a clean in place, the original it guards becomes the file set
aside, or `save_file` would refuse the source's own name as the original) or
`Target::Row` — records `Seen::Unread`, `written`, and asks *This document was
cleaned after the window opened* (`compare-cleaned-since-title`, `-body`, the
existing `compare-changed-choices`), Enter Cancel. Overwrite is `begin(Some(
Force::Overwrite))`, which writes over that home and tells the row; Keep
theirs is `keep_theirs` (now also for `Target::Row`, from the row's text);
Cancel changes nothing. `begin` asks the same question again for a
`Seen::Unread` home without a force, so it never reaches `save_file`.

The test drives the real queue (`queue::tests::queue_with`, made
`pub(crate)` for it) and a real Compare window over the same line of cleans.
It runs without a journal (`work: None`), so "its journal entry unchanged" is
checked as the row's own status word and mark: *Cleaned*, not *Cleaned, then
edited*, after Cancel; the journal side of a mark is C3's test.

### C3 — a mark names its result (D442)

`Told::Saved { action: wipemark_store::entry::Action, home: compare::Home }`,
`Home::File(path) | Home::Item(id)`. `told_by_compare` checks `holds(row,
work, action, &home)` — `cleaned_for` for a clean, `Row::rewritten` for a
rewrite — and otherwise answers `true` and logs. `Command::Edited` and
`Writer::edited` carry the action; `Journal::mark_edited(id, at, action)`
checks it in both statements. `Told::Text` marks with `Action::Clean`. The
queue test cleans a row, rewrites it (`FakeEngine`), then tells a clean's
save: no `outcome.edited`, the row reads *Rewritten*; a rewrite's save then
marks it. The SQL guard alone would already keep the journal clean in that
test, so the row's check is what the test reads through the row's word —
`Row::edited_at` is an `Option<i64>` as the task has it, and without `holds`
it is set and the row says *Rewritten, then edited* (that is `C3`'s red).

### C4 — the check before the rename (D443)

`wipemark_intake::inplace::write_atomically_if` over `write_atomically_with`;
the refusal is a flag, not an error type. `save_file` is `save_file_with(…,
|| {})`; the hook runs inside `still`, after the staging. One read.

### C6, C7 — questions (D445, D446)

`ask(question, closing, …)` replaces `ask_changed` (a `Question` enum:
`Changed`, `Exists(path)`, `CleanedSince(name)`); while a question stands it
is held in `Saving::held`. `ask_close` asks a held question after Save (with
`closing` kept, and no second write) and after Cancel, and drops it with
Discard. `may_close` waits for a save under way when nothing was typed since
it began. `ask_nowhere` is the Copy-and-close question; Enter is Copy and
close.

### C8 — "…, then edited" (D447)

`wording::edited`; `Row::edited()` reads the outcome's `edited`, the
journal's `outcome.edited` (an ended `Recorded` row) and `Row::edited_at`
(set where C3's mark lands, cleared with the next clean — where `row.edited`
is — and at a rewrite's push and end). The status cell's word and tooltip go
through it (`status_said`, split out of `status_cell` so a test can read
them; `Queue::status_words`, test-only). `Queue::report_of` hands the Report
whether the row is edited, and `report::sheet_edited` says
`window-report-edited` under "what happened", in the window and in Copy as
Markdown; Copy JSON is unchanged.

### C12 — the Send-away test

Two files in one drop, an endpoint on duty, *Process what arrives* set to
rewrite: one `SendAway` naming both ids and `https://y.example.com`, nothing
pushed, both rows *Not started*; `agreed(ids, Road::Arrivals,
Away(host))` pushes two items whose consent is that endpoint; a second drop
answered with nothing stays unpushed. No bug found, so no D-number taken
for it.

### C16 — measured, nothing to fix (D449)

**The measurement.** GPUI's test mode paints a dirty window at every effect
flush until nothing is dirty, so what the panes settle on after an input says
nothing about a frame late; `cx.update(|window, _| window.refresh())` is not
the smallest step either, for the same reason. So the window keeps a
test-only record, `CompareView::frames`: the zero-sized canvas painted after
both panes pushes both panes' tops in its paint, before it defers `painted`.
The test reads the **first frame painted after each step**. Each step is a
`simulate_mouse_move` of the thumb (`thumb_of`, E7-10's helper; mouse down
once, six moves of 9 px, mouse up) or one `ScrollWheelEvent` of 1.5 lines,
each 20 ms of real time after the last, past the scroll bar's 120 Hz
throttle, so every move is told at once rather than by its trailing timer.
Texts: 400 shared lines, the result with three more at the top (unwrapped).

| gutters | leader | input | first frame after each of six steps |
|---|---|---|---|
| middle (the original's bar on its left) | original | thumb drag | level ×6 — **same frame** |
| middle | original | wheel | same frame |
| middle | result | thumb drag | same frame |
| middle | result | wheel | same frame |
| left | original | thumb drag | same frame |
| left | original | wheel | same frame |
| left | result | thumb drag | same frame |
| left | result | wheel | same frame |
| middle, the result wrapped | original | thumb / wheel | same frame |
| middle, the result wrapped | result | thumb drag | one frame late ×6, level the frame after |
| middle, the result wrapped | result | wheel | same frame ×2 (inside the three lines the original has none of), then one frame late ×4 |

**Why.** The scroll bar notifies `window.current_view()` — the view it is
painted in. gpui-kit's `Editor` renders its `EditorState` entity as a view,
and `EditorScrollbar` is painted inside it, so the drag notifies the
`EditorState` itself: the window's `observe_in` on each editor hears it, as
it hears the wheel. The maintainer's finding holds wherever the bar is
painted in a parent's view; this window's bars are painted in the editors'
own (his story's code was not read for this — the claim is ours, measured
here). **The probe can see a late frame**: with the
two observers' `look` taken out, every unwrapped row of the table above reads
one frame late (`C16` red). A wrapped result is read only at the end of a
frame (D386) — its row in the table is that rule, and `C16-wrap` shows the
end-of-frame look is what keeps it to one frame. Nothing was late to fix, so
no look was added to `render`; the module docs and `compare.md` ("Seeing a
scroll") now say what was measured, and the gates stay. With
`compare.sync_scroll` off, a drag of either thumb moves its own pane alone.

Not measured, and said rather than guessed: the scroll bar throttles its
drag notifications to 120 Hz and sends a trailing one; a frame painted for
another reason *inside* such an interval, before the trailing notification,
would show the dragged pane moved and the other not until the end-of-frame
look. Such a frame needs a notification from outside both editors (a
caret blink notifies the result's editor, whose observer looks at both
panes). The host check below is where that would show.

## Decisions

- **D440** — at quit, what autosave would have saved is saved: autosave on and
  not stopped, no question, edits, a file or the batch queue's row as home;
  synchronously in the `on_app_quit` callback (the one write a Compare window
  makes on the GPUI thread, `settings.rs`'s reason); by the stamp held, a
  changed home not written and logged; the journal writer awaited within
  200 ms (`Writer::flushed`, through `Link::flushed`); a save under way waited
  for, nothing written beside it. Not flushed: autosave off, a Save that
  cleans, a cleaned paste's row. *Reason*: GPUI asks no window on ⌘Q or the
  tray's Quit, and the edit was lost silently.
- **D441** — a Save that cleans asks the row where its result lives first;
  with a home it retargets, records `Seen::Unread`, and asks; the line runs
  only without one. *Reason*: the line would be refused `Exists` and set the
  row and its journal to *Not cleaned* over a good result.
- **D442** — an edit mark names its action and home, checked by the row and
  in the SQL. *Reason*: one row per document (D320); a clean's save after a
  rewrite marked the rewrite.
- **D443** — the changed-on-disk check after the staging, just before the
  rename (`inplace::write_atomically_if`); what remains: the last read and the
  rename, a program writing in place into the old file after the rename;
  nothing locked (advisory locks bind only those who take them). *Reason*: a
  write anywhere in the staging and `fsync` was replaced unasked.
- **D444** — an entry this build cannot read is left as it is by an edit mark.
  *Reason*: `config.rs`'s rule for a row this build cannot read.
- **D445** — one question at a time in a Compare window; a standing one never
  replaced; a held one asked after Save (no second write, the close kept) or
  Cancel, dropped with Discard; ⌘W during a save with nothing typed since
  waits. *Reason*: D364's rule for the main window; the close question's
  subscription was dropped.
- **D446** — closing with edits and nowhere to save asks: Copy and close
  (Enter), Discard, Cancel. *Reason*: the edits vanished unasked.
- **D447** — "…, then edited" wherever the verdict is said (badge, tooltip,
  the Report and its Markdown copy; Copy JSON unmoved), through
  `wording::edited`. *Reason*: a row must not say *Cleaned* over a text Layer A
  never ran over. **Alternative**, for the owner: run Layer A over the edit
  before it is written — `saved_of` and the file and row saves write
  `clean(edited)` at the defaults; the verdict is honest without a new word,
  at the cost of characters typed on purpose (a U+00A0 in a French quotation,
  a ZWJ in a pasted emoji).
- **D448** — Report… of a journal row greyed with its reason
  (`queue-action-report-journal`). *Reason*: the journal holds metadata only
  (D312) — a sheet from it would carry a verifiable shelf it cannot back and
  no `to_json()`. **Alternative**: enable it with what the journal holds —
  arrived and happened from the entry, the counts, "the full report was not
  kept" on the verifiable shelf, the third shelf from `not_established::ids()`,
  Copy JSON greyed.
- **D449** — the two panes stand level in the frame a scroll bar is dragged
  (C16): measured, nothing to fix — the bar notifies the editor's own view;
  a wrapped result that leads is followed a frame late and no later (D386);
  the frame record and its three gates stay. *Reason*: the owner's point and
  the maintainer's finding in #3417's story; measured rather than assumed.

New catalogue keys, in en, ru and de: `compare-cleaned-since-title`,
`-body`; `compare-close-nowhere-title`, `-body`, `compare-close-copy`;
`queue-status-cleaned-edited`, `-partly-edited`, `-rewritten-edited`,
`-partly-rewritten-edited`, `queue-status-edited-tooltip`;
`queue-action-report-journal`; `window-report-edited`. No epic number in any
value (`no_catalogue_value_carries_an_epic_number` green).

## Red checks

`python3 docs/plan/reports/compare-followups-2026-10-09-red.py`, run on the
branch's code:

```
C1: RED compare::tests::quitting_saves_an_edit_autosave_has_not_reached_yet
C1-stamp: RED compare::tests::quitting_never_writes_over_a_file_changed_on_disk
C2: RED compare::tests::a_row_cleaned_since_the_window_opened_is_asked_about_not_refused
C3: RED queue::rewrite_tests::a_save_marks_the_entry_it_was_opened_on_and_not_a_later_one
C3-sql: RED journal::tests::an_edit_mark_lands_only_on_the_action_it_names
C4a: RED inplace::tests::write_atomically_if_publishes_only_while_still_holds  (second run; the first was BROKEN, below)
C4b: RED compare::save::tests::a_write_after_the_staging_is_not_written_over
C5: RED journal::tests::an_entry_this_build_cannot_read_is_left_as_it_is_by_an_edit_mark
C6: RED compare::tests::a_close_question_is_not_replaced_by_a_changed_file
C7: RED compare::tests::closing_with_nowhere_to_save_asks_and_can_copy
C8: RED wording::tests::an_edited_result_never_reads_as_plainly_cleaned, compare::tests::a_save_that_cleans_says_then_edited_on_its_row
C12: RED queue::rewrite_tests::a_drop_that_would_be_sent_away_is_asked_about_first
C16: RED compare::tests::both_panes_stand_level_in_the_frame_a_scroll_bar_is_dragged
C16-wrap: RED compare::tests::with_the_result_wrapped_it_leads_a_frame_late_and_no_later
C13: RED queue::tests::report_of_a_journal_row_is_greyed_with_its_reason
```

Fifteen checks, fifteen RED.

`C4a` came back BROKEN on the first run: the new `inplace` test did not
compile — the test module imports names one by one, and
`write_atomically_if` was not among them (an earlier green run of the intake
tests was a stale artifact; see "Notes"). Fixed, it is RED. `C13` and `C16`
were run again after rustfmt moved `C13`'s piece.

## Gates

`docs/plan/reports/compare-followups-2026-10-09-gates.sh`, once at the end, all `--locked`:

```
fmt                            exit 0  
clippy                         exit 0  
test-workspace                 exit 0  1819 passed, 0 failed, 7 ignored
dep-direction                  exit 0  
gpui-pin                       exit 0  
check-no-default               exit 0  
check-local-llama              exit 0  
test-engine-local-llama        exit 0  40 passed, 0 failed, 1 ignored
test-app-local-llama           exit 0  739 passed, 0 failed, 2 ignored
clippy-pipeline-examples       exit 0  
test-pipeline-examples         exit 0  26 passed, 0 failed, 0 ignored
```

Nothing here touches `crates/wipemark-llama*` or `wipemark-engine/src/local.rs`; the native gates were not due.

Baseline on `58624fc` before the first code change (`cargo test --workspace
--locked`): **1801 passed, 0 failed, 7 ignored** (the last reports for the
two merged branches said 1795/0/7 and 1794/0/7; the merge on `feat` adds what
both brought). This branch adds 18 tests: compare 10 (C1 ×3, C2, C6, C7, C8,
C16 ×3), compare/save 1, queue 1, queue/rewrite_tests 2, wording 1, store 2,
intake 1.

CI_RESULTS

## Host checklist (for the coordinator — what only a window shows)

1. ⌘Q, and the tray's Quit, within a second of typing in an autosaving Compare window over a cleaned file: the file holds the edit after the quit, and the row says *Cleaned, then edited* on the next launch.
2. A Compare window on a waiting row; Clean the row from the main window; type in the window: *This document was cleaned after the window opened* — Cancel, then Overwrite.
3. Close (⌘W and the close button) a window whose result is the original's own file, after typing: Copy and close / Discard / Cancel; Copy and close leaves the text on the clipboard.
4. Autosave off: Save, type, change the file elsewhere, ⌘W: the close question stays; Save on it brings the changed-file question.
5. The "…, then edited" badges in en, ru and de: whether "Очищено частично, затем изменено" and "Teilweise umgeschrieben, dann bearbeitet" fit the Status column or truncate; the tooltip's sentence.
6. Report… on a row from an earlier session or a rewrite: greyed, its reason as a second line, not cut off at 280 px in ru and de.
7. Drag each pane's scroll bar slowly and quickly in a long Compare of two similar texts — the other pane moves with it, no lag and no shiver — with `compare.gutters` middle and left; then the same with the result wrapped (Settings › Compare), where a frame of lag while the result leads is expected.

## Wanted edits for `CLAUDE.md` (not made; the task's §4)

* **"The Compare window is three things kept apart"**, after "…without it,
  a close with edits not saved asks Save / Discard / Cancel.": "⌘Q and the
  tray's Quit ask no window, so each window saves at quit what autosave
  would have saved — synchronously in its `on_app_quit` callback, by the
  stamp it holds, never over a home that changed — and waits for the
  journal's writer (D440). A Save that cleans asks the row where its result
  lives first, and a row cleaned since the window opened is asked about —
  Overwrite / Keep theirs / Cancel — never cleaned over (D441). The
  changed-on-disk check sits after the staging, just before the rename
  (`inplace::write_atomically_if`); a write between that read and the rename
  is still replaced, and nothing is locked (D443). One question at a time: a
  standing one is never replaced (D445). Closing with edits and nowhere to
  save asks — Copy and close, Discard, Cancel (D446)." And in its scrolling
  sentence, after "(D386)": "— a thumb drag included: the bar notifies the
  editor's own view, and the panes stand level in the frame either moves,
  measured on the first frame painted (D449); a wrapped result that leads is
  followed a frame late".
* **"Every document has a status, whoever asked"**, after "One row per
  document, its action the last asked (D320).": "An edit mark from a Compare
  window names the result it is about — its action and its home — and lands
  only while that is the row's latest result, checked by the row and in the
  SQL (D442); an entry this build cannot read is left as it is (D444)."
* **"The table rewrites, through the one batch queue"** (the queue's
  bullet), or "The main window is lazy-shot's": "A result edited in Compare
  and saved as typed reads '…, then edited' wherever its verdict is said —
  badge, tooltip, the Report and its Markdown copy (D447, built at the
  default; the alternative is Layer A over the edit). Report… of a row the
  journal keeps is greyed with its reason (D448)."
* **The top paragraph**: nothing it says about Compare moved.
* **"Build, run, look"** (optional): on Linux the gate packages include
  `libgtk-3-dev`, `libayatana-appindicator3-dev` and `libxkbcommon-x11-dev`
  since the Linux tray; a container without them fails in `glib-sys`'s
  build script, then at the link.

## Notes

* **Packages.** The container lacked `libgtk-3-dev`,
  `libayatana-appindicator3-dev` and `libxkbcommon-x11-dev`; installed from
  the list `gate.yml` installs. No `LIBRARY_PATH` symlink was needed.
* **A shared target directory across two worktrees misleads.** The baseline
  was run from a clean worktree at `58624fc` sharing `CARGO_TARGET_DIR` with
  the branch's worktree; workspace crates of the two checkouts then share
  artifact names, and cargo took one tree's freshly built crate as fresh for
  the other. That is why one early intake run reported green over a test that
  did not compile. Every number in this report is from runs after the
  branch's sources were touched and rebuilt; the gates ran on the branch
  alone.
* `queue::tests` is `pub(crate)` now, with `Scratch`, `queue_with` and
  `statuses`, so that a Compare test can drive a real queue (C2, C8).
* `report::sheet` and `ReportView::with_words` are test-only now (every
  window goes through `sheet_edited` / `ReportView::new(…, edited, …)`).
