# E7-10 — Compare's gutters face the middle, on gpui-kit `next` `d9b7c421` (2026-10-09)

*The report of `e7/compare-mirrored`, built in a Docker container (aarch64,
no window, no GPU, no model) from the task
`docs/plan/E7-10-compare-mirrored.md` (Watchword
`wipemark-task-compare-mirrored-2026-10-09`). Base `ac1b921` on
`feat/e0-e6-shell` — one plan commit past the task's `8ad87e1`, which filed
this task; no code between them. Also in Watchword as FILE
`wipemark-compare-mirrored-report-2026-10-09`.*

## What it is

The Compare window's original now has its gutter — line numbers and change
markers — on its **right**, against the divider, and its vertical scroll bar
on its **left**, its outer edge; the result keeps its gutter on its left and
its scroll bar on its right. On both panes the gutter's columns run, from the
text outward, fold icons (none: neither pane folds), the change marker, the
line numbers — so the two columns of numbers face each other across the
divider, IntelliJ IDEA's diff viewer. A Compare page row, `compare.gutters`,
chooses between that (`middle`, the default) and the layout before (`left`),
and an open window follows it at once. All of it is the library's own API as
merged on gpui-kit `next` (#3416, #3417): the submodule moved from `f8429177`
to `d9b7c421`, nothing else moved, nothing is carried.

## M1–M6

| | what | state | commit | test | removed locally → red |
|---|---|---|---|---|---|
| M1 | the submodule onto `next` `d9b7c421` | **done** | `04fa521` | the whole suite; `scripts/check-gpui-pin.sh` | — (a bump; see "What the bump moved") |
| M2 | the mirrored layout | **done** | `06e845d` | `the_original_s_gutter_faces_the_middle`, `the_original_s_scroll_bar_is_on_its_outer_edge`, `each_choice_lays_out_both_panes` | `set_gutter_side` in `apply_gutters` → the first; `set_scrollbar_placement` → the second; the middle's result order put back to the library's → the third |
| M3 | the row `compare.gutters`, live | **done** | `06e845d` | `flipping_the_row_moves_an_open_window_s_gutters`, `a_first_launch_faces_the_middle`, `both_on_the_left_is_today_s_layout`; `every_persisted_preference_has_a_row` and `every_row_is_reachable_from_the_sidebar` green | the observer's `apply_gutters` call → the first; `Gutters::parse` answering `Left` → the second |
| M4 | tests | **done** | `06e845d`, red script `fa4652d` | the six above | five checks, all RED: `E7-10-compare-mirrored-2026-10-09-red.py` |
| M5 | C16 and `fix/compare-followups` | **done as asked**: C16 not fixed here; neither parallel branch had landed on `feat` (fetched before the gates: `feat` still `ac1b921`), so nothing was merged in. The helper that finds a pane's thumb by the layout is `compare::tests::thumb_of` (D467), for C16 to reuse | `06e845d` | — | — |
| M6 | docs and plan | **done** | `0a0b26d` | — | — |

The red script applies each deletion, runs the named test, and restores the
file byte for byte; its run:

```
M2-gutter-side: RED compare::tests::the_original_s_gutter_faces_the_middle
M2-scroll-bar: RED compare::tests::the_original_s_scroll_bar_is_on_its_outer_edge
M2-order: RED compare::tests::each_choice_lays_out_both_panes
M3-observer: RED compare::tests::flipping_the_row_moves_an_open_window_s_gutters
M3-parse: RED config::tests::a_first_launch_faces_the_middle
```

### M1 in detail

1. `git -C vendor/gpui-component checkout d9b7c4219a93cc8a95e92d4a4f2f880bab26fb22`;
   `git log f8429177..d9b7c421` is exactly #3416 `63068d0f` and #3417
   `d9b7c421`, and `git diff --name-only f8429177 d9b7c421` touches no
   `Cargo.toml` (sources, stories, the website, `component-inventory.json`).
   Nothing inside the submodule's tree changed but its commit.
2. `scripts/check-gpui-pin.sh` passes **unchanged**: `gpui-pre =0.3.8` in
   both manifests, 23 `gpui-pre*` crates once each, X11 fixes A, B, C in the
   resolved sources. `git grep observe_button_layout_changed` in the vendored
   component finds nothing at `f8429177` or at `d9b7c421`.
3. `cargo build -p wipemark-app --locked`: built, and **`Cargo.lock` did not
   move** (path crates, no manifest changed upstream).
4. **What the bump moved: nothing.** The 57 `compare::` and `result::` tests
   ran green on the bump commit alone (`04fa521`, the M2 work stashed). The
   numbers placed by shaped width, the code editor's narrow padding now on the
   gutter's side, the text clear of a left track and the row tints across the
   gutter moved no assertion: our alignment tests read y only. One test helper
   moved because of **M2**, not the bump — see below.
5. Docs: `gpui-pin.md` §1 (the diagram and "The vendored component" now name
   `d9b7c421` = #3359 + #3416 + #3417), §4 "The bump of 2026-10-09: the
   component only", §5 links to #3416, #3417, both commits and the Editor
   Diff story; `line-decorations.md` §4 (the submodule at `d9b7c421`, which
   contains `f8429177`, and the marker column's order); `compare.md`'s
   marker sentence follows D463. `CLAUDE.md` not edited — wanted edits below.

**Every x-dependent geometry read in the tests** (`row_bounds`,
`input_bounds`, `origin.x`, `size.width` in `compare.rs` and `result.rs`
tests; `result.rs` has none):

| where | test | reads | touched |
|---|---|---|---|
| `compare.rs` `the_first_lines_sit_level` | itself | `row_bounds(0)`, y only | no |
| `compare.rs` `across_the_original` | `the_original_selects_with_the_mouse`, `typing_into_the_original_changes_nothing` | `row_bounds(0)` width | **yes** (D466): the points were 0.4 and 0.9 of the row from the row's left; `row_bounds` spans the gutter, so with the gutter on the right 0.9 can land in it. Now from the text's own left (`range_to_bounds(&(0..0))`), a sixth and three fifths of the row on. At the test window's width the old points still landed in text (0.9 × 943 + 10 = 859 < 905, where the gutter starts) — it did not go red, it was moved before it could |
| `compare.rs` `wheel` | the four scroll tests the task names | `input_bounds().center()` | no — the centre stays text |
| `compare.rs` `two_panes_moved_in_one_frame_end_where_a_wrapped_result_put_them` | itself | `input_bounds().center()` | no — the same |
| the new helpers `pane_of`, `text_left`, `thumb_of`, `in_gutter` | the five window tests | `input_bounds`, `range_to_bounds`, `presentation().gutter_side()` | new |

### M2 in detail

- **Original**: `set_gutter_side(Side::Right)`, `set_scrollbar_placement(BottomLeft)`,
  `set_gutter_order([FoldIcons, Markers, LineNumbers])`. **Result**: gutter
  `Left`, bar `BottomRight` (both the defaults, set explicitly so `left` and
  `middle` are one function), the same order. Every piece in
  `CompareView::apply_gutters`, from `Gutters::layout(side)` — a pure table
  — which `CompareView::new` calls after building the two editors with their
  defaults; nothing in `panes`, `render`, `look` or the scroll code. The
  library is imported as `gpui_component::Side as Edge`; `diff::Side` is
  untouched.
- **`folding(false)` and the FoldIcons column**: it takes **no width** on
  either side. At `d9b7c421`, `fold_column_width(false)` is zero
  (`element.rs:403-409`) and `layout_line_numbers` adds
  `FOLD_ICON_HITBOX_WIDTH` only `if state.mode.is_folding()`
  (`element.rs:1254-1257`), so listing it first puts nothing between the text
  and the marker. Measured: the result's text starts 48 px into its input in
  the test window under `middle` (FoldIcons, Markers, LineNumbers) and under
  `left` (FoldIcons, LineNumbers, Markers) alike — the order moves columns,
  not the gutter's width.
- **What depends on x, and what covers it:**

| interaction | covered by | reads an x? |
|---|---|---|
| the original's text starts at its left, clear of the bar; its gutter at its right; the result's text past its gutter; the original's text ends a gutter before its right edge | `the_original_s_gutter_faces_the_middle` | yes: `range_to_bounds` of column 0, and the caret at the end of a line longer than the pane, which the editor scrolls into view inside the text — 58 px from the pane's right, the gutter's 48 plus the editor's 10 px right margin |
| the original's thumb drags at its left edge, its gutter does not scroll, the result's thumb at its right | `the_original_s_scroll_bar_is_on_its_outer_edge` | yes: the window's own mouse events at `thumb_of` / `in_gutter` |
| both on the left: each text right of its gutter, alike; the original's bar on its right | `both_on_the_left_is_today_s_layout` | yes |
| the mouse selection in the original | `the_original_selects_with_the_mouse` (points from the text's left, D466) | yes |
| typing into the original | `typing_into_the_original_changes_nothing` | yes, through the same drag |
| the cursor follow | `the_cursor_follow_does_not_drag_the_result` and the follow tests | **no** — rows only; it does not cover the layout, and needs not |
| scrolling together | `scrolling_the_result_scrolls_the_original`, `scrolling_the_original_scrolls_the_result`, `a_scroll_the_library_applies_is_followed_too`, `with_the_row_off_each_side_scrolls_alone` | the wheel aims at `input_bounds().center()`, which stays text; the rest is offsets — **no** layout coverage, and none needed: D380–D392 read rows and offsets |
| the inline marks (`Inline`) | `the_finer_marks_are_answered_for_each_side` | **no** — byte ranges |
| the line marks (`Marks`) | `a_comparison_puts_line_marks_on_both_sides` | **no** — rows; where the marker is painted in the gutter is the column order, guarded by `each_choice_lays_out_both_panes` (the library keeps a marker's x to itself) and by the host's eye (checklist 2) |
| `repaint_original`'s empty edit | `the_original_is_asked_again_when_the_marks_move` | **no** — at the cursor |
| copy and the result's strip | `result.rs` tests | **no** — untouched by the gutter |

### M3 in detail

- `compare.gutters` (`config::COMPARE_GUTTERS_KEY`), in `PERSISTED` (37),
  spelled `middle` | `left`; `compare::Gutters` in `Grain`'s shape (`ALL`,
  `id`, `parse`, `title`); an unknown id reads as `middle`, is warned about
  and left in the row (`a_first_launch_faces_the_middle`, and the existing
  `an_unusable_compare_row_falls_back_without_being_rewritten` gained two
  spellings). `Comparison` gained `gutters`; the struct literals in the
  tests already used `..Comparison::default()` except `config.rs`'s two,
  which name it.
- `Setting::CompareGutters` on the Compare page, after "What is marked"
  (what a pane shows, before how the two move), a `RadioGroup` like the
  grain's; `Storage::Row(COMPARE_GUTTERS_KEY)`;
  `Preferences::place_gutters` persists through `config::write_compare_gutters`.
- **D462 built live.** `compare::open` takes `Option<Entity<Preferences>>`
  (`main.rs`'s `--compare=` road and `queue.rs`'s `compare_row` hand theirs
  in); `CompareView` keeps `cx.observe(&preferences, …)`, which calls
  `apply_gutters` when the value differs from the window's. The observe was
  clean: it runs after the `Preferences` update has returned (an effect, not
  a nested update), needs no `Window`, and `apply_gutters` updates the two
  `EditorState`s only. `flipping_the_row_moves_an_open_window_s_gutters`
  holds the result's undo history unchanged **behaviourally** — the library
  has no public `can_undo`: an edit typed before the flip is still the first
  thing Undo takes back.
- Strings in en, ru, de (`settings-compare-gutters-title`, `-description`,
  `-middle`, `-left`), no epic number; `settings-compare-description` names
  the exception ("— except where the line numbers sit, which an open window
  follows at once"); `compare-help-settings` says **five** things, in the
  page's order, and that where the line numbers sit changes the open window
  at once. The ids `middle` and `left` are never translated. ru: «Навстречу
  друг другу» / «Обе стороны слева»; de: „Zur Mitte hin“ / „Beide links“ —
  their length in the page's column is checklist 6.

### M4 — what the tests use

Public geometry only: `input_bounds()`, `row_bounds(0)`,
`range_to_bounds(&(0..0))` for the text's x (equivalent to
`EntityInputHandler::bounds_for_range` and needs no `Window`),
`presentation().gutter_side()`, `scroll_offset()`, `set_cursor_position`, and
the window's own mouse events (`drag`). The scroll-bar tests set the theme's
scroll bars to `ScrollbarMode::Always` (`bars_shown`): in the library a
scroll bar takes a press only while it is visible, and in the default
`Scrolling` mode a hidden bar is not even hovered into view — so the press is
tested on a shown bar, and how a person reveals it is checklist 3.

**D465, measured, not as the task expected.** A press in the original's
right-hand gutter does not scroll — there is no bar there — but it is not
inert: the editor's mouse-down covers the whole input, gutter included, and
resolves the point past the ends of the lines, so a click puts the caret at
that row's end and a drag selects from row end to row end (in the test,
`6..48`: the end of row 0 to the end of row 6). That is the mirror of a
press in a gutter on the left, which today lands at the row's start. The
test asserts the scroll only; the library's selection is left as it is, not
patched and not asked upstream.

### M5

C16 had not landed: `origin/feat/e0-e6-shell` was still `ac1b921` when
fetched before the gates, and `fix/compare-followups` was not pushed. The
helper C16 needs is here: **`compare::tests::thumb_of(view, side, cx)`** —
the point where a person grabs a pane's vertical thumb, by the window's
layout (the original's left edge facing the middle, the right edge
otherwise), four pixels outside the input bounds, inside the padding the
track overlays, near the top of the track, where the thumb is while the pane
is at the top. C16's `both_panes_stand_level_in_the_frame_a_scroll_bar_is_dragged`
should drag from it rather than assume the right edge, and use `bars_shown`
for a bar that takes a press. Files both branches touch, for whoever merges:
`compare.rs` (mine: `Gutters`, `Layout`, `open`'s signature,
`CompareView::new`, `apply_gutters`, the tests' new block and
`across_the_original`), `queue.rs` (`compare_row` passes the preferences),
`settings.rs` (one row), the three catalogues, `docs/architecture/compare.md`
(a new section before "The Compare page", the page's paragraph, the
decisions list), `docs/plan/README.md` (row 8, row 7g after 7d, D460–D467
after D429, §7 E7's two paragraphs).

### M6

`docs/architecture/compare.md`: **"Where the gutters sit (E7-10)"** — placed
after "What is real" (which holds "Scrolling together" and "Following" as
paragraphs, not sections) and before "The Compare page"; "The Compare page"
gained the row; "The decisions, by number" gained D460–D467. The module
docs of `compare.rs` gained the same section in short.
`docs/plan/README.md`: §2.1 row 8 **done**, row **7g** (7e and 7f left to the
two follow-up branches), §7 E7's "wait on the owner's gpui-kit pull
requests" and "And the gutter" marked done by E7-10, §4 D460–D467.

## Decisions

| # | decision | reason |
|---|---|---|
| **D460** | The submodule on gpui-kit `next` `d9b7c421` (#3359 + #3416 + #3417); `gpui-pre` unchanged; nothing carried | what M2 needs is merged upstream; no manifest moved, the lock did not move |
| **D461** | The original's gutter on its right, its scroll bar on its left; the result as before | the owner's goal since 2026-10-07: corresponding numbers face to face, each bar on its pane's outer edge |
| **D462** | `compare.gutters`, `middle` by default, followed by an open window at once | the setters cost no undo entry, no jump and no `Window` (`state.rs:10689-10742` at `d9b7c421`), and this is a layout a person flips in order to look at it; the page's sentence names the exception. **Alternative**: read at the opening, like the other four rows (D385's reason, one sentence true of every row). The observe was clean, so the alternative was not needed |
| **D463** | Both gutters `[FoldIcons, Markers, LineNumbers]` from the text | the marker by the line it marks, the numbers at the divider; FoldIcons costs nothing with `folding(false)` |
| **D464** | `left` is the layout before to the column: the library's sides and its default order | "Both on the left" is offered as today's layout; a person who prefers it gets exactly it, markers outside the numbers included |
| **D465** | A press in the original's right-hand gutter is the library's: no scroll, a caret at the row's end, a drag from row end to row end | measured; the mirror of a left gutter; not ours to patch |
| **D466** | `across_the_original` measures from the text's own left | `row_bounds` spans the gutter; the bump moved no test, the layout could have |
| **D467** | `thumb_of` finds a pane's thumb by the layout, for C16 | C16 had not landed |

## Gates

Once, at the end, on `fa4652d` plus the gates script (no source changed
after), all `--locked`, by `E7-10-compare-mirrored-2026-10-09-gates.sh`:

| gate | command | result |
|---|---|---|
| fmt | `rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')` | exit 0 |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| test | `cargo test --workspace --locked` | exit 0 — **1794 passed, 0 failed, 7 ignored** |
| deps | `scripts/check-dep-direction.sh` | exit 0 — "dependency direction ok" |
| gpui pin | `scripts/check-gpui-pin.sh` | exit 0 — `gpui-pre =0.3.8`, 23 crates once each, X11 fixes A, B, C |
| no default features | `cargo check --workspace --no-default-features --locked` | exit 0 |
| local-llama | `cargo check --workspace --features local-llama --locked` | exit 0 |
| engine, local-llama | `cargo test -p wipemark-engine --features local-llama --locked` | exit 0 — 40 passed, 0 failed, 1 ignored |
| app, local-llama | `cargo test -p wipemark-app --features local-llama --locked` | exit 0 — 722 passed, 0 failed, 2 ignored |
| the bench, clippy | `cargo clippy -p wipemark-pipeline --features local-llama --examples --locked -- -D warnings` | exit 0 |
| the bench, tests | `cargo test -p wipemark-pipeline --features local-llama --examples --locked` | exit 0 — 26 passed, 0 failed, 0 ignored |

**What moved:** the round adds six tests and removes none
(`each_choice_lays_out_both_panes`, `the_original_s_gutter_faces_the_middle`,
`the_original_s_scroll_bar_is_on_its_outer_edge`,
`both_on_the_left_is_today_s_layout`,
`flipping_the_row_moves_an_open_window_s_gutters`,
`a_first_launch_faces_the_middle`), so the base runs 1788. The task's 1779
is the count in `fix/e7-8-e8-1-followups`'s report; E4-8 (1787 on its own
branch, and its verification's tests after it) was merged into `feat` after
that. The native gates were not due: nothing under `crates/wipemark-llama*`
or `wipemark-engine/src/local.rs` changed.

## CI

`gate` workflow run **37968565478** on `fa4652d` — every line of code in
the round (the commits after it are this report and the gates script):
<https://github.com/GigLaboCom/wipemark-app/actions/runs/37968565478> —
**success**.

| job | conclusion |
|---|---|
| `gate` (fmt, clippy, test, deps, features) | **success** — its `test` step 1794 passed, 0 failed, 7 ignored, the container's count: <https://github.com/GigLaboCom/wipemark-app/actions/runs/37968565478/job/113949032714> |
| `native` (llama.cpp prebuilt + Vulkan, model-free) | **success**: <https://github.com/GigLaboCom/wipemark-app/actions/runs/37968565478/job/113949032842> |
| `macos` (clippy, tests, llama-native prebuilt with Metal) | **success** — the one job that compiles the component's and our `cfg(target_os = "macos")` code with clippy `-D warnings`, over the moved submodule: <https://github.com/GigLaboCom/wipemark-app/actions/runs/37968565478/job/113949032514> |

## Host checklist (for the coordinator; what only a window shows)

1. Light and dark screenshots of a Compare window in both layouts (`--compare=<path>`, the row flipped on the Compare page).
2. `middle`: the two columns of numbers face to face across the divider, each pane's change marker between its numbers and its text, the original's scroll bar on its outer (left) edge and its horizontal bar stopping short of its gutter.
3. A long file: each scroll bar revealed (scroll, or hover with "show on hover"), its thumb dragged slowly and quickly — the other pane follows (C16's line, once C16 lands).
4. A click in the original's gutter: nothing scrolls, the caret goes to that row's end (D465); a click on its last line just above a horizontal scroll bar lands the caret there.
5. The row flipped with a Compare window open: the gutters and the original's bar move at once; the text, both scroll offsets, the marks and the result's undo history do not.
6. The row's title, description and both choices, and the page's amended sentence, in ru and de: no truncation in the page's column.

## Wanted edits for `CLAUDE.md` (not edited here)

- **"First command after any clone"** (`:91-99`): the submodule is
  `longbridge/gpui-kit` `next` at **`d9b7c421`** — #3359 + #3416 + #3417
  over `main` `8d8cc671` — and "Nothing of ours is carried on it".
- **"The Compare window is three things kept apart"**: the original's gutter
  faces the middle — line numbers and markers on its right, its scroll bar on
  its left, the result as before, the marker by the text and the numbers at
  the divider on both (D461, D463) — and `compare.gutters` (`middle` |
  `left`) is the one Compare row an open window follows at once (D462).
- **Epic order**: "Compare's left scrollbar and the original's gutter on its
  right, which wait on the owner's gpui-kit pull requests #3416 and #3417" →
  done in E7-10 (`e7/compare-mirrored`).
- **Specs table**: `wipemark-task-compare-mirrored-2026-10-09` (FILE, the
  task) and `wipemark-compare-mirrored-report-2026-10-09` (FILE, this
  report).
- **The top paragraph**: Compare, if it is named there, opens with the
  gutters facing the middle.

## Files

- the task: `docs/plan/E7-10-compare-mirrored.md`
- the red checks: `docs/plan/reports/E7-10-compare-mirrored-2026-10-09-red.py`
- the gates: `docs/plan/reports/E7-10-compare-mirrored-2026-10-09-gates.sh`
