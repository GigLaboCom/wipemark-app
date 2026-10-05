# E7 — the windows clean: report, E7-1 and E7-2

Task: Watchword FILE `wipemark-task-e7-windows-clean-2026-10-05`. Plan:
[`docs/plan/E7-windows-clean.md`](../E7-windows-clean.md). Branch
`e7/windows-clean`, from `origin/images/series-v3` at `ebf421a` (the fourth
images round; `origin/feat/e0-e6-shell` at `b3fbee7` does not contain it, so
nothing was merged).

**Status: E7-1 and E7-2 are done. E7-3…E7-6 are not** — this report grows
step by step as the owner asks for each part, and it is not the series'
final report: the Watchword upload and the live-check script for the
host belong to the end of the series (§7 of the task), after `--clean=`
exists (E7-2).

## Steps

| step | done | commit | tests | mutations red |
|---|---|---|---|---|
| E7-1 the cleaner, with no window | yes | `81af4a4` | 26 in `clean::tests` | 12 of 12 (M1–M12) |
| E7-2 the queue cleans | yes | (this commit) | 2 `#[gpui::test]` + 3 in `queue::tests`, 1 in `clean::tests`, 2 in `wording::tests`, 2 in `main::tests` | 7 of 7 (M1–M7) |
| E7-3 Compare shows the real result | no | — | — | — |
| E7-4 the report, three shelves | no | — | — | — |
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

## Mutations (`docs/plan/reports/e7-windows-clean-mutate.py E7-1 E7-2`)

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

E7-1's twelve were run again after E7-2 (M1's text moved with D270): all
red. E7-2/M3 first came out **green** — Clean all already skips what is not
waiting, so the guard in `Queue::clean` was not reached; the test now asks
rows again by id, as `--clean=` and the menu do, and it is red.

`git status --short` after the run: only this step's own uncommitted files.

The task's list also names "Compare's result back to a copy", "the third
shelf dropped" and "the Markdown copy in `Rendering::Ui`": those protections
belong to E7-3 and E7-4 and are not built yet.

## Gates (in this container, on `aarch64` Linux)

| gate | result |
|---|---|
| `rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')` | clean |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean |
| `cargo test --workspace --locked` | E7-1: 1401 passed; **E7-2: 1410 passed**, 0 failed, 6 ignored |
| `scripts/check-dep-direction.sh` | ok |
| `cargo check --workspace --no-default-features --locked` | ok |
| `cargo check --workspace --features local-llama --locked` | ok |
| `cargo test -p wipemark-app --features local-llama --locked` | E7-1: 477 + 1; **E7-2: 486 + 1 passed**, 0 failed, 1 ignored |
| `cargo test -p wipemark-engine --features local-llama --locked` | 36 passed, 1 ignored |

Every gate ran. The app's tests link here only after installing GPUI's
system libraries (`apt-get install libxkbcommon-x11-dev libfontconfig1-dev
libxcb1-dev libfreetype-dev`; the image had none of them). The native
llama gates do not apply: nothing under `crates/wipemark-llama*` or
`crates/wipemark-engine/src/local.rs` changed.

GitHub Actions, `gate` on the push of `81af4a4` (E7-1): **success** in 26 min —
https://github.com/GigLaboCom/wipemark-app/actions/runs/37323279470

## Decisions

D260–D271, stated in full in the plan's §9:

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
  button it used to contradict.

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

## Owner questions

Unchanged from the task's §5 and built at their defaults so far: (1) no
auto-clean on arrival; (2) AI provenance only from the windows; (3) Layer A
at its defaults (Q-A1); (4) an existing result refused, with an explicit
replace (E7-2) rather than a numbered name.

## Unfinished

E7-3…E7-6, the live-check script (`E7-windows-clean-live-check.md`, which
needs `--clean=` from E7-2), the final report in Watchword, and the
`CLAUDE.md`/README edits above.
