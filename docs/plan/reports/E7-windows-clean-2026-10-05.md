# E7 — the windows clean: report, E7-1 … E7-6

Task: Watchword FILE `wipemark-task-e7-windows-clean-2026-10-05`. Plan:
[`docs/plan/E7-windows-clean.md`](../E7-windows-clean.md). Branch
`e7/windows-clean`, from `origin/images/series-v3` at `ebf421a` (the fourth
images round). `origin/feat/e0-e6-shell` came to contain `series-v3` during
the series and was merged at `e70d2e6` (it brought `CLAUDE.md` and
`docs/plan/README.md` only). `origin/images/series-v3` then moved again
(`2cc421d`, the fifth images round — tests, docs and fixtures) without
`feat/e0-e6-shell` containing it, and was merged at `dcd9d98`.

**Status: E7-1 … E7-6 are done.** This is the series' final report (§7 of
the task), uploaded to Watchword as `wipemark-e7-windows-clean-report-2026-10-05`.
The live check for the host is
[`E7-windows-clean-live-check.md`](E7-windows-clean-live-check.md); it has
not been run — this container cannot open a window.

## Steps

| step | done | commit | tests | mutations red |
|---|---|---|---|---|
| E7-1 the cleaner, with no window | yes | `81af4a4` | 26 in `clean::tests` | 12 of 12 (M1–M12) |
| E7-2 the queue cleans | yes | `6895b6d` | 2 `#[gpui::test]` + 3 in `queue::tests`, 1 in `clean::tests`, 2 in `wording::tests`, 2 in `main::tests` | 7 of 7 (M1–M7) |
| E7-3 Compare shows the real result | yes | `9555ab7` | 4 `#[gpui::test]` in `compare::tests` | 4 of 4 (M1–M4) |
| E7-4 the report, three shelves | yes | `2fc5d2d` | 4 + 2 `#[gpui::test]` in `report::tests` | 7 of 7 (M1–M7) |
| E7-5 the panel | yes | `d8945eb` | 5 in `panel::tests`, 2 in `clean::tests` | 8 of 8 (M1–M8) |
| E7-6 every "not yet", the docs | yes | (this commit) | 1 renamed + 1 new in `settings::tests` | 3 of 3 (M1–M3) |

## What E7-1 built

`apps/wipemark-app/src/clean.rs`, blocking, no GPUI:

- `cleanable(&Intake) -> Cleanable` — `Text(Encoding)`, `Picture(Format)`,
  `No(Unable)` from the intake alone.
- `clean_one(&Arrival, &Plan, row, now) -> Outcome` — read (head first,
  re-identified from the bytes read, size-limited), clean (Layer A at its
  defaults, or `wipemark_picture::clean` with `Scope::AiProvenance`), decide
  (`outcome_of`, pure), keep, write by the plan, one log line.
- `outcome_of(&Cleaning) -> (Verdict, bool)` — the CLI's policy stated once.
- `sweep(kept, Period, now)` — per-clean kept directories past their period,
  by the time in their names; called once per launch on the background
  executor from `main.rs`, and after each kept write.
- `PICTURE_LIMIT` (64 MiB), `invented_name`, `extension_of`.

No caller yet besides the sweep: the module is `allow(dead_code)` outside
tests, with the reason stated, until E7-2's queue calls it.

## What E7-2 built

- `queue.rs`: `Status` on every row, the Status column after Kind (badge
  per verdict, its sentence as tooltip), `Line` — one clean at a time,
  first asked first done, pure — and `Queue::clean`, `clean_all`,
  `replace`, `hand_to_clean`, `progress`; the plan taken from
  `Preferences::plan_for` when a row's clean starts; the Name cell's note
  and the hover card say what happened and where the result went once a
  row is done; the Actions menu gains Clean (first; greyed with the reason
  under it), Open the result, Show the result in its folder, Copy the
  result, Replace the existing result.
- `clean.rs`: `replace_one` and `Outcome::replaced` (D270).
- `wording.rs`: `verdict_badge`, `said`, `unable`, `refused`, `failed`,
  `went` — every sentence about a finished clean, shared with the panel in
  E7-5.
- `main.rs`: `--clean=<path>`, Clean all on the toolbar after Paste,
  "Cleaning 2 of 5" in the status bar while the line runs.
- `settings.rs`: `Preferences::for_tests`.
- The catalogue: the `## The windows clean (E7)` block in en, ru (plurals
  for the count and the status line) and de; `queue-pending` and
  `toolbar-help-pending` rewritten (D271).

## What E7-3 built

Done by a second agent in its own worktree, in parallel with E7-4 at the
owner's request, and cherry-picked onto the branch before E7-4 (its notes,
`e7-3-notes.md`, are folded in here and removed).

- `compare.rs`: `Subject::read` cleans with Layer A at its defaults in the
  read's own background task (`Loaded::cleaned`); `loaded` puts that in the
  result pane, the view keeps it (`cleaned_text`) and `reset` puts it back
  — nothing is cleaned on the GPUI thread (D273); Reset is offered once the
  result differs from the cleaned text, computed with the diff (D272).
- The catalogue (values only, en/ru/de): `compare-pending` (the result is
  what cleaning makes of the original; editing it there saves nothing;
  closing writes nothing — true on the Settings › Compare page as well),
  `compare-reset` ("Back to the cleaned text"), `compare-reset-tooltip`,
  `compare-help-close`.
- `docs/architecture/compare.md`: "What the result is today" and the "Not
  written" bullet.

## What E7-4 built

- `report.rs`: `sheet` — what arrived, what happened, Verifiable,
  Best-effort, Not established, as values over a `Say` (D275, D276);
  `markdown`; `ReportView`, the dialog, with Copy JSON (`to_json()`
  exactly, greyed when nothing was read — D277), Copy as Markdown (plain
  words) and Close; Escape, the backdrop and Close one answer.
- `queue.rs`: Report… in the Actions menu, enabled on a done row, deferred
  to `QueueEvent::Report`; `Queue::report_of`.
- `main.rs`: the shell paints the dialog over the whole window (D274).
- `wording.rs`: every sentence about a clean takes a `Say` (`said_in`,
  `went_in`, `unable_in`, `refused_in`, `failed_in`, `title_of_in`,
  `kind_label_in`, `evidence_note_in`), with `window` and `plain`.
- The catalogue: `queue-action-report` and the `window-report-*` keys, in
  the E7 block, en/ru/de (Fluent plurals for counts; decimals through
  `wipemark_i18n::decimal`). Reused as they read right in a window:
  `unicode-class-*`, `confidence-*`, `image-kind-*`, `image-signal-*`,
  `report-not-established-*`, `cli-report-unicode` and the
  `cli-image-visible-*`, `cli-image-refusal-*`, `cli-image-encoded-*`,
  `cli-image-row`/`-evidence*`, `-rendering`, `-exif-removed`,
  `-orientation-removed` lines — none names a flag or a stream.
- `docs/architecture/queue.md`: "Cleaning" and "The report, and its three
  shelves" (the mapping); "What it does not do yet" now says rewriting.

## What E7-5 built

- `clean.rs`: `inspect_one(&Arrival) -> Findings` — the read a clean makes
  (same bytes, limits, re-identification), then `wipemark_core::inspect` or
  `wipemark_picture::inspect`; nothing written (D278). `number()`, one
  counter for every thing filed in a run (D280).
- `panel.rs`: `Held`, the drop on screen with a look per listed thing and an
  outcome per cleaned one, kept per drop; the looks run one at a time on the
  background executor when the drop lands and stop when a later drop
  overtakes them. Under each listed thing: the findings line ("Looking…"
  first), then the *would* lines — none for a thing that cannot be cleaned —
  or, once cleaned, `wording::said` in the verdict's colour and
  `wording::went`. Clean sits beside the dismissal line (no height taken
  from the list): every cleanable thing not yet cleaned, one at a time, the
  plan taken at each start (D279); "Cleaning…" and greyed while it runs,
  greyed once nothing is left, absent when nothing caught could be cleaned.
  Pure: `to_clean`, `clean_offered`, `findings_line`, `something_found`.
- `queue.rs`: row ids from `clean::number()` (D280).
- `drop.rs`: `Catcher::is_caught(&Arc<[Arrival]>)`, so the panel keeps its
  looks for the drop it shows and not for one a later drop overtook.
- `wording.rs`: `found` / `found_in`; two stale doc lines ("once cleaning
  exists").
- The catalogue: `panel-looking`, `panel-found-*`, `panel-clean`,
  `panel-clean-tooltip`, `panel-cleaning` in the E7 block (ru plurals);
  `panel-pending` rewritten in place to say only that rewriting is not in the
  windows.

## What E7-6 built

- The catalogue, en/ru/de: `settings-retention-pending` — "The windows clean
  by these rules: what they write, and what they keep, follows this page.
  The command line and agents read none of them — the command line is told
  where a result goes on each run, and an agent gets its result back." —
  and `setup-welcome-body`: cleaning runs from these windows (Clean in the
  main window and in the panel) as well as from the command line and over
  MCP; rewriting runs from the command line and over MCP, not from these
  windows yet. The en comments above both, and the toolbar's.
  `toolbar-help-pending`, `queue-pending`, `panel-pending` and
  `compare-pending` had been rewritten with their own steps. Left alone, as
  the task says: `tray-clean-clipboard` ("— not yet") and every Engine and
  Models line, which are about the model. A grep of the three catalogues
  for "not in this version" and "yet" finds nothing else about cleaning in
  the windows (the rest is the CLI's refusals, TIFF/HEIC/AVIF, the hotkey
  platform, an empty queue and the model).
- `settings.rs`: `the_retention_banner_always_says_nothing_is_written_yet`
  is now `the_retention_banner_says_who_follows_it`, asserting the new line
  in every state and, in every language, not the old one;
  `the_welcome_says_the_windows_clean_and_do_not_rewrite` is new.
- The documents: `queue.md` (the table's Status and Actions rows; its
  cleaning section came in E7-2/E7-4), `retention.md` (the introduction,
  the plan read at each start, **How the windows execute it** — rules 1–7
  through `clean.rs`, and what the sweep removes — and "What is left
  open"), `images.md`, `visible-marks.md`, `drag-and-drop.md`'s "cleaning
  anything" row, and `layer-a.md` and `skeleton.md` (deviation 18).
- `docs/plan/reports/E7-windows-clean-live-check.md`: the fourteen cases of
  §7, each with its command, what the window must show and what must be on
  disk; every result is compared byte for byte (`cmp`) with what
  `wipemark-cli clean` writes for the same input.

## Mutations (`docs/plan/reports/e7-windows-clean-mutate.py E7-1 … E7-6`)

| # | protection | result | tests |
|---|---|---|---|
| E7-1/M1 | an existing result is refused, never overwritten (D261) | red | `an_existing_result_is_refused_and_left_alone` |
| E7-1/M2 | a text with nothing found writes nothing (D260) | red | `nothing_found_writes_nothing`, `a_text_comes_to_what_the_cli_says_it_does` |
| E7-1/M3 | a picture with nothing found, or nothing changed, writes nothing (D260, D262) | red | `a_picture_with_nothing_on_it_writes_nothing`, `a_transparent_mark_is_left_and_nothing_identical_is_written`, `a_picture_comes_to_what_the_cli_says_it_does` |
| E7-1/M4 | a picture whose metadata is still marked is never written | red | `a_picture_still_marked_is_never_written` |
| E7-1/M5 | a dropped file is never copied into kept/ | red | `a_dropped_file_is_never_kept` |
| E7-1/M6 | kept/ is not created when nothing is kept (the launch sweep included) | red | `the_sweep_removes_what_is_past_its_period_and_nothing_else` |
| E7-1/M7 | in place sets the original aside first | red | `in_place_sets_the_original_aside_and_never_twice` |
| E7-1/M8 | the sweep keeps what is inside its period | red | `the_sweep_removes_what_is_past_its_period_and_nothing_else` |
| E7-1/M9 | the sweep removes nothing under Forever | red | `the_sweep_removes_what_is_past_its_period_and_nothing_else` |
| E7-1/M10 | the bytes read decide: a file that became a PNG is never decoded as text | red | `a_file_that_changed_after_the_drop_is_read_by_its_bytes` |
| E7-1/M11 | text is written back in the encoding it arrived in | red | `a_marked_text_is_written_beside_it_in_its_own_encoding` |
| E7-1/M12 | a result never lands on its own source | red | `into_the_results_folder_and_never_over_the_source` |
| E7-2/M1 | two cleans at once instead of one at a time | red | `one_clean_runs_at_a_time_in_the_order_asked`, `the_queue_cleans_one_row_at_a_time_by_the_plan_at_its_start` |
| E7-2/M2 | the plan is not the Retention page's as it stands when the clean starts | red | `the_queue_cleans_one_row_at_a_time_by_the_plan_at_its_start` |
| E7-2/M3 | a row asked twice is cleaned twice | red | `the_queue_cleans_one_row_at_a_time_by_the_plan_at_its_start` |
| E7-2/M4 | Clean offered on a thing that cannot be cleaned | red | `clean_is_greyed_with_a_reason_when_it_cannot_run` |
| E7-2/M5 | Replace writes over a file other than the one named | red | `a_result_is_replaced_only_where_it_was_named` |
| E7-2/M6 | the footer back to "cleaning is not here" | red | `the_footer_says_rewriting_is_not_here_yet` |
| E7-2/M7 | an epic number in the footer | red | `the_footer_says_rewriting_is_not_here_yet` |
| E7-3/M1 | Compare's result back to a copy of the original | red | `the_result_is_the_cleaned_text_and_the_original_is_not`, `the_result_is_what_the_queue_writes` |
| E7-3/M2 | Reset returns to the original | red | `reset_returns_to_the_cleaned_text_not_the_original` |
| E7-3/M3 | the read does not clean | red | the three above |
| E7-3/M4 | Reset offered against the original (D272) | red | `the_result_is_the_cleaned_text_and_the_original_is_not` |
| E7-4/M1 | the report's third shelf dropped | red | `every_outcome_has_its_three_shelves` |
| E7-4/M2 | the Markdown copy rendered with `Rendering::Ui` | red | `the_copies_are_the_json_and_plain_markdown` |
| E7-4/M3 | Copy JSON is not the library's `to_json()` | red | `the_copies_are_the_json_and_plain_markdown` |
| E7-4/M4 | a picture's shelf without the pixels' claim first | red | `every_outcome_has_its_three_shelves` |
| E7-4/M5 | a claim with no sentence dropped | red | `a_claim_with_no_sentence_is_shown_in_its_own_words` |
| E7-4/M6 | a shelf with nothing on it vanishes | red | `every_outcome_has_its_three_shelves` |
| E7-4/M7 | the Markdown copy carries a character Layer A would remove | red | `the_markdown_copy_carries_nothing_layer_a_would_remove` |
| E7-5/M1 | the panel's Clean cleans a thing a second time | red | `clean_is_offered_only_when_something_is_left_to_clean` |
| E7-5/M2 | Clean offered while a clean runs | red | `clean_is_offered_only_when_something_is_left_to_clean` |
| E7-5/M3 | Clean offered over a thing that cannot be cleaned | red | `clean_is_offered_only_when_something_is_left_to_clean` |
| E7-5/M4 | the look counts rows, not characters | red | `the_look_agrees_with_the_clean` |
| E7-5/M5 | pixels not examined said only when they did not decode | red | `the_findings_line_says_what_a_look_found` |
| E7-5/M6 | AI metadata dropped from a not-examined line | red | `the_findings_line_says_what_a_look_found` |
| E7-5/M7 | the panel says again that it does not clean | red | `the_panel_says_only_rewriting_is_not_here` |
| E7-5/M8 | one number handed out twice, a kept directory shared | red | `a_number_is_handed_out_once` |
| E7-6/M1 | the Retention banner says again that no window writes | red | `the_retention_banner_says_who_follows_it` |
| E7-6/M2 | the Russian Retention banner keeps the old sentence | red | `the_retention_banner_says_who_follows_it` |
| E7-6/M3 | the walk-through says again that neither layer runs from the windows | red | `the_welcome_says_the_windows_clean_and_do_not_rewrite` |

E7-1's twelve were run again after E7-2 (M1's text moved with D270): all
red. E7-2/M3 first came out **green** — Clean all already skips what is not
waiting, so the guard in `Queue::clean` was not reached; the test now asks
rows again by id, as `--clean=` and the menu do, and it is red.

`git status --short` after the run: only this step's own uncommitted files.

E7-5's eight have no counterpart in the task's list (it names none for the
panel); they are this step's own protections, all red. E7-6's three are the
task's "must go red with the old sentence", for the banner in two languages
and for the walk-through. **41 mutations in all, 41 red**, the tree restored
after each run.

Every mutation the task's list names is now in the script: the last three
were E7-3/M1, E7-4/M1 and E7-4/M2. E7-3's four were run by the second
agent in its worktree (each red, the tree restored) and again here after
the cherry-pick.

## Gates (in this container, on `aarch64` Linux)

| gate | result |
|---|---|
| `rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')` | clean |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean |
| `cargo test --workspace --locked` | E7-1: 1401; E7-2: 1410; E7-3 + E7-4: 1420; E7-5: 1427; **E7-6 (with the `series-v3` merge's 4): 1432 passed**, 0 failed, 6 ignored |
| `scripts/check-dep-direction.sh` | ok |
| `cargo check --workspace --no-default-features --locked` | ok |
| `cargo check --workspace --features local-llama --locked` | ok |
| `cargo test -p wipemark-app --features local-llama --locked` | E7-1: 477 + 1; E7-2: 486 + 1; E7-3 + E7-4: 496 + 1; E7-5: 503 + 1; **E7-6: 504 + 1 passed**, 0 failed, 1 ignored |
| `cargo test -p wipemark-engine --features local-llama --locked` | 36 passed, 1 ignored |

Every gate ran. The app's tests link here only after installing GPUI's
system libraries (`apt-get install libxkbcommon-x11-dev libfontconfig1-dev
libxcb1-dev libfreetype-dev`; the image had none of them). The native
llama gates do not apply: nothing under `crates/wipemark-llama*` or
`crates/wipemark-engine/src/local.rs` changed.

GitHub Actions, `gate` on the push of `81af4a4` (E7-1): **success** in 26 min —
https://github.com/GigLaboCom/wipemark-app/actions/runs/37323279470

On the push of `6895b6d` (E7-2): **failure** —
https://github.com/GigLaboCom/wipemark-app/actions/runs/37334567754 — the
`gate` and `native` jobs green, the `macos` job red on the two new
`queue::tests` `#[gpui::test]`s: `Queue::new` builds a `Catcher`, whose
macOS drop destination asks the window for its native handle, and GPUI's
test window answers that with `unimplemented!`. Linux has no destination,
so this container could not see it. Fixed in the E7-4 commit: the tests
build the queue over `Catcher::detached()` (test-only, no platform) through
`Queue::with_catcher`; the production road is unchanged.

The E7-4 push (`2fc5d2d`) was overtaken by the merge's push a few seconds
later and its run cancelled. On `e70d2e6` (E7-2 … E7-4 and the merge): **success**,
all three jobs, the `macos` one included —
https://github.com/GigLaboCom/wipemark-app/actions/runs/37343860128

On `dcd9d98` (E7-5 and the `series-v3` merge): **success**, all three jobs —
https://github.com/GigLaboCom/wipemark-app/actions/runs/37351267513

## Decisions

D260–D280, stated in full in the plan's §9:

- **D260** nothing found is nothing written;
- **D261** a result never replaces a file already there (refused, left byte
  for byte; "Replace the existing result" is E7-2's explicit action);
- **D262** a result identical to its input is never written, whatever the
  verdict — `crying-transparent-1025.png` is `Partly(Mark)` with nothing
  written;
- **D263** a text's verdict is by change, then by suspicion: changed is
  `Cleaned` (a soft hyphen alone included), unchanged and suspicious (a
  homoglyph kept at the defaults) is `Partly(Left::Kept)` with nothing
  written;
- **D264** `PICTURE_LIMIT` is 64 MiB;
- **D265** a kept copy is made only when there is a result, before the
  result is written; kept directories are named in UTC, invented result
  names in local time;
- **D266** `clean_one` takes no `Homes`; its log line carries paths as
  `Elided` shapes;
- **D267** the sweep never creates the kept folder, and runs at launch and
  after each keep;
- **D268** `--clean=` asks for a row whatever it is; the badge of a thing
  that cannot be cleaned says why, and the menu and Clean all offer only
  what can be;
- **D269** a greyed Clean carries its reason as a second line in the menu
  item — gpui-component's menu items have no tooltip;
- **D270** "Replace the existing result" is `clean::replace_one`: only the
  file the first clean refused, never the source, the plan taken again;
- **D271** `toolbar-help-pending` is rewritten in E7-2, beside the Clean all
  button it used to contradict;
- **D272** Compare's Reset is offered against the cleaned text, "Back to the
  cleaned text";
- **D273** Compare cleans in the read's background task and keeps the text;
  Reset never cleans again;
- **D274** the Report dialog is the shell's, over the whole window, opened by
  `QueueEvent::Report`;
- **D275** the shelf mapping (`queue.md`, "The report");
- **D276** the window's sentences take a `Say`; the dialog is handed both
  renderings;
- **D277** a thing never read has a report with no JSON, and Copy JSON is
  greyed;
- **D278** the panel looks at what it lists, once per drop, by the read a
  clean makes; a text's count is characters, and a kept look-alike is said;
- **D279** the panel's Clean cleans every caught thing that can be and has
  not been, one at a time, the plan at each start; a later drop does not
  stop it;
- **D280** one counter numbers the queue's rows and the panel's cleans, so
  no two cleans share a kept directory.

## Deviations from the task

1. **`clean_one(arrival, plan, row, now)`**, not `(arrival, plan, homes,
   now)`: the plan already names both folders, and the row id is what the
   invented name and the kept directory are made of (D266).
2. **The log line has no full path** (D266): the task said "the paths";
   `CLAUDE.md` and the CLI log a path's shape only.
3. **`NothingFound` for text is "unchanged and not suspicious"**, not
   "`!suspicious`" alone (D263): a text whose only change is a removed soft
   hyphen is written, as the CLI writes it and as Compare will show it.
4. **`Partly` with an unchanged picture writes nothing** (D262); the CLI
   writes the identical copy.
5. **The sweep runs at launch and after each keep**, not also once a day
   while running as `retention.md` foresaw (D267).
6. `retention.rs` is unchanged; mutation M5 is applied there, because
   `Plan::File` having no `Kept` is the protection.
7. **Clean's disabled reason is a line under the item, not a tooltip**
   (D269).
8. **Report… is not in the Actions menu yet**: it comes with its dialog in
   E7-4, rather than as a greyed item with nothing behind it.
9. **`toolbar-help-pending` was rewritten in E7-2**, ahead of E7-6 (D271).
10. **E7-3 changed three Compare values besides `compare-pending`**
    (`compare-reset`, its tooltip, `compare-help-close`) — they had become
    false — and the "Not written" bullet of `compare.md`.
11. **`queue.md` gained its cleaning section in E7-4**, with the shelf
    mapping the step asked for, rather than in E7-6.
12. **E7-3 was built by a second agent in a worktree** (the owner asked for
    one in parallel) and cherry-picked; its gates were app + i18n tests and
    clippy on the app, and the workspace gates ran here after the merge.
13. **`drop.rs` gained `Catcher::is_caught`** (E7-2 already gave it
    `detached`); it is not in the task's list of files.
14. **The findings line has a fourth text case**, "nothing to remove; a
    look-alike is kept at the defaults", beyond the task's two: the clean
    will call that text partly clean, and "nothing to remove" alone would be
    followed by a verdict that disagrees.
15. **A thing that cannot be cleaned has no *would* lines** in the panel any
    more: a folder's "every file in it would be handled the way a dropped
    file is" had become false (folders are not expanded, Q-D2).
16. **The queue's ids skip numbers** the panel used (D280).
17. **The panel has no `#[gpui::test]`**: its view installs the floating
    level and the macOS drop destination as it opens, the test-window trap;
    the tests are the pure functions the task asked for, and what the
    window paints is for the live check.
18. **Two documents the task did not name were edited**: `layer-a.md`'s
    "Who calls it" ("Until then the windows say that they do not clean
    yet") and `skeleton.md`'s row for the app ("say that they do not clean
    yet") — both false after E7. One sentence each.
19. **`setup-welcome-body` has a test** (`the_welcome_says_the_windows_clean_and_do_not_rewrite`):
    the task asked for the rewrite and a test only for the Retention
    banner; without one the old sentence could come back unseen.
20. **The live check compares every result with the CLI's** (`cmp` against
    `wipemark-cli clean -o`) rather than with recorded hashes: the bytes of
    a restored picture depend on the encoder build, and the CLI on the same
    machine is the oracle that cannot drift from it.

## Wanted edits to `CLAUDE.md` and `docs/plan/README.md`

For the end of the series, collected as they arise:

- `CLAUDE.md`, the app file table: a row for `clean.rs` — "cleaning one
  thing that arrived: what can be, the read, the CLI's policy as
  `outcome_of`, the write by the plan, kept copies and the sweep".
- `CLAUDE.md`, the flags table: a `--clean=<path>` row — "puts a file in the
  queue and cleans it at startup, once per flag — Import and Clean, so a
  check of what a clean writes does not start by driving a menu".
- `CLAUDE.md`, "The main window is lazy-shot's" and the `queue.rs` row: the
  table cleans — the status column, one clean at a time, the plan taken at
  start, the Actions menu's new items; "Nothing is cleaned … and the footer
  says the first of those" is no longer true.
- `CLAUDE.md`, the panel bullet ("There are four windows…") and the
  `panel.rs` row: the panel cleans — a findings line per listed thing, Clean
  beside the dismissal line.
- `CLAUDE.md`, the introduction: "The windows do not clean yet (E7)" — they
  do: text and pictures, from the queue, the panel and `--clean=`; Layer B
  is still not in the windows (E4-6b).
- `CLAUDE.md`, the crates table: `wipemark-image` and `wipemark-picture`
  "no window yet (**E7**)" / "(**E12-8**)" — the windows call
  `wipemark-picture` now.
- `CLAUDE.md`, the Compare bullet: "Nothing is cleaned yet, so the result
  starts as a copy of the original and the banner says so" — the result is
  `clean(original)` (E7-3).
- `CLAUDE.md`, the Retention bullet: "Nothing is written yet, and
  `the_retention_banner_always_says_nothing_is_written_yet` keeps the page
  saying so" — the windows write by the rows, and the gate is
  `the_retention_banner_says_who_follows_it`; "E7 calls
  `wipemark_intake::inplace` as the CLI does" is now done (`clean.rs`).
- `CLAUDE.md`, the setup bullet: "Layer A runs from the command line and
  over MCP but not yet from the windows" — it runs from the windows.
- `CLAUDE.md`, "No epic number leaves this repository": the list of pending
  surfaces — the queue's footer, the toolbar's help and the panel now say
  only that *rewriting* is not here.
- `CLAUDE.md`, the epic order: E7 done (E7-1 … E7-6); with it the window
  half of E12-8 that cleans a picture.
- `docs/plan/README.md` §4: D260–D280.

## Engineering question found on the way

Compare decodes a text the way the preview does — leniently — while the
queue decodes strictly (`wipemark_intake::text::decode`). For a file that
does not decode (invalid UTF-8, an unnamed eight-bit encoding) the queue
refuses and writes nothing, and Compare still shows the lenient text and
what cleaning makes of it. Nothing on disk disagrees with the window, but
that result is one the queue would never write. Making Compare refuse what
the queue refuses is a separate decision, left open.

## Owner questions

Built at the task's defaults; each is the owner's to change.

1. **Should something dropped be cleaned the moment it arrives?** Today it
   is not: cleaning happens when it is pressed — Clean on a row, Clean all,
   the panel's Clean, or `--clean=` at launch. Cleaning on arrival would be
   a switch on a Settings page, off by default.
2. **Should the windows remove all of a picture's metadata — camera make,
   GPS, the rotation it shows upright by — or only what marks it as made by
   AI?** Today only what marks it as made by AI, as the MCP tool does by
   default; the command line removes all of it with `--all-metadata`.
3. **Should the windows offer cleaning's finer choices** — replacing a
   letter borrowed from another alphabet, Unicode normalisation, spaces?
   Today the windows clean at the defaults, and the Report says so when it
   keeps such a letter (Q-A1).
4. **When a result is already there, should a clean refuse or pick a new
   name?** Today it refuses and leaves the file as it is, and "Replace the
   existing result" in the row's menu writes over it on request. The
   alternative is `name.cleaned-2.md`.

## Unfinished

- **The live check has not been run.** It is written for the host; nothing
  a window paints in this series has been seen by anyone — the panel's
  findings line and Clean least of all (no `#[gpui::test]`, deviation 17).
- **The `CLAUDE.md` and `docs/plan/README.md` edits** above are for the
  coordinator: this series may not edit either file.
- **The engineering question** above (Compare's lenient decode) is open.
- **The sweep runs only at launch and after a keep** (D267): a session
  left open past a period removes nothing until the next launch.
- **The tray's Clean Clipboard** stays disabled with its sentence, as the
  task says.
