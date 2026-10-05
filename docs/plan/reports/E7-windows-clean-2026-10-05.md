# E7 — the windows clean: report, E7-1 … E7-4

Task: Watchword FILE `wipemark-task-e7-windows-clean-2026-10-05`. Plan:
[`docs/plan/E7-windows-clean.md`](../E7-windows-clean.md). Branch
`e7/windows-clean`, from `origin/images/series-v3` at `ebf421a` (the fourth
images round; `origin/feat/e0-e6-shell` at `b3fbee7` does not contain it, so
nothing was merged).

**Status: E7-1 … E7-4 are done. E7-5 and E7-6 are not** — this report grows
step by step as the owner asks for each part, and it is not the series'
final report: the Watchword upload and the live-check script for the
host belong to the end of the series (§7 of the task), after `--clean=`
exists (E7-2).

## Steps

| step | done | commit | tests | mutations red |
|---|---|---|---|---|
| E7-1 the cleaner, with no window | yes | `81af4a4` | 26 in `clean::tests` | 12 of 12 (M1–M12) |
| E7-2 the queue cleans | yes | `6895b6d` | 2 `#[gpui::test]` + 3 in `queue::tests`, 1 in `clean::tests`, 2 in `wording::tests`, 2 in `main::tests` | 7 of 7 (M1–M7) |
| E7-3 Compare shows the real result | yes | `9555ab7` | 4 `#[gpui::test]` in `compare::tests` | 4 of 4 (M1–M4) |
| E7-4 the report, three shelves | yes | (this commit) | 4 + 2 `#[gpui::test]` in `report::tests` | 7 of 7 (M1–M7) |
| E7-5 the panel | no | — | — | — |
| E7-6 every "not yet", the docs | no | — | — | — |

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

## Mutations (`docs/plan/reports/e7-windows-clean-mutate.py E7-1 … E7-4`)

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

E7-1's twelve were run again after E7-2 (M1's text moved with D270): all
red. E7-2/M3 first came out **green** — Clean all already skips what is not
waiting, so the guard in `Queue::clean` was not reached; the test now asks
rows again by id, as `--clean=` and the menu do, and it is red.

`git status --short` after the run: only this step's own uncommitted files.

Every mutation the task's list names is now in the script: the last three
were E7-3/M1, E7-4/M1 and E7-4/M2. E7-3's four were run by the second
agent in its worktree (each red, the tree restored) and again here after
the cherry-pick.

## Gates (in this container, on `aarch64` Linux)

| gate | result |
|---|---|
| `rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')` | clean |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean |
| `cargo test --workspace --locked` | E7-1: 1401; E7-2: 1410; **E7-3 + E7-4: 1420 passed**, 0 failed, 6 ignored |
| `scripts/check-dep-direction.sh` | ok |
| `cargo check --workspace --no-default-features --locked` | ok |
| `cargo check --workspace --features local-llama --locked` | ok |
| `cargo test -p wipemark-app --features local-llama --locked` | E7-1: 477 + 1; E7-2: 486 + 1; **E7-3 + E7-4: 496 + 1 passed**, 0 failed, 1 ignored |
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

## Decisions

D260–D277, stated in full in the plan's §9:

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
  greyed.

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
- `docs/plan/README.md` §4: D260–D271.

## Engineering question found on the way

Compare decodes a text the way the preview does — leniently — while the
queue decodes strictly (`wipemark_intake::text::decode`). For a file that
does not decode (invalid UTF-8, an unnamed eight-bit encoding) the queue
refuses and writes nothing, and Compare still shows the lenient text and
what cleaning makes of it. Nothing on disk disagrees with the window, but
that result is one the queue would never write. Making Compare refuse what
the queue refuses is a separate decision, left open.

## Owner questions

Unchanged from the task's §5 and built at their defaults so far: (1) no
auto-clean on arrival; (2) AI provenance only from the windows; (3) Layer A
at its defaults (Q-A1); (4) an existing result refused, with an explicit
replace (E7-2) rather than a numbered name.

## Unfinished

E7-5, E7-6, the live-check script (`E7-windows-clean-live-check.md`, which
needs `--clean=` from E7-2), the final report in Watchword, and the
`CLAUDE.md`/README edits above.
