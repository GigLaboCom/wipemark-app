# Task — E7-10: Compare's gutters face the middle (the mirrored layout), on gpui-kit `next` `d9b7c421`

*Watchword FILE `wipemark-task-compare-mirrored-2026-10-09`, ttl 0. Written 2026-10-09 by the
coordinator of `GigLaboCom/wipemark-app` for an implementer agent **in a Docker container** (or the
cloud): it compiles and runs tests; it has no window, no GPU and **no model files** (never download
one — nothing here needs a model). Self-contained: everything needed is here or in the repository.*

## 0. What this is

The owner's goal since 2026-10-07 (`docs/plan/README.md` §7 E7, the bullet "**Build. Compare scrolls
both panes together**", its last paragraph "**And the gutter**", `README.md:1300-1306`; §2.1 row 8,
`:201`): **a mirrored layout like IntelliJ IDEA's diff viewer** — the original pane's gutter (line
numbers, #3359's markers) on its **right**, facing the middle, and its vertical scroll bar on its
**left**, the outer edge; the result pane as today (gutter left, scroll bar right). Both gutters face
the divider, so the numbers of corresponding lines sit next to each other across it.

Upstream is done. gpui-kit `next` is now **`d9b7c421`** = our pin **`f8429177`** + exactly two
commits, nothing else (`git log f8429177..d9b7c421`), and no `Cargo.toml` in it moved (the diff
touches only sources, stories and the website — `gpui-pre` stays `=0.3.8`):

- **#3416** `63068d0f` — `ScrollbarPlacement { BottomRight (default), BottomLeft, TopRight, TopLeft }`
  with `is_left()` / `is_top()` (`crates/base/src/scrollbar.rs:654-678`); on the editor
  `EditorState::scrollbar_placement(self, ScrollbarPlacement) -> Self` and
  `set_scrollbar_placement(&mut self, ScrollbarPlacement, cx: &mut Context<Self>)`.
- **#3417** `d9b7c421` — `gutter_side(self, Side) -> Self` / `set_gutter_side(&mut self, Side, cx)`,
  `GutterColumn { FoldIcons, LineNumbers, Markers }` with `gutter_order(self, impl IntoIterator<Item =
  GutterColumn>) -> Self` / `set_gutter_order(&mut self, …, cx)` — the order is **from the text
  outward**, default `[FoldIcons, LineNumbers, Markers]`, a column left out follows in default order
  (`crates/base/src/input/base/layout.rs`, `GutterColumn::order`); `presentation().gutter_side()`
  reads it back. The text keeps clear of a left scroll bar's whole track; line numbers are placed by
  shaped width; the styled `Editor`'s narrow padding (`min(input_px, 6px)`) moved to the gutter's side
  (`crates/component/src/input/input.rs`); a decoration's row background now spans the gutter too;
  the horizontal track ends before a centre-facing gutter; and the maintainer's Editor Diff story
  (`crates/story/src/stories/editor_diff_story.rs:93-94`, `:161-220` — the exact calls we want).
- None of the setters takes a `Window`, and none touches the undo history (`state.rs`, the setters
  at `:10683-10742` of `d9b7c421`, `scrollbar_placement` `:10689`: assign, `cx.notify()`).

Paths in our build (re-exports at `d9b7c421`): `gpui_component::Side` (`component/src/lib.rs:103`),
`gpui_component::scroll::ScrollbarPlacement` (`component/src/scroll/mod.rs`),
`gpui_component::input::GutterColumn` (`component/src/input/mod.rs`). **`compare.rs` already imports
`crate::diff::Side`** (`compare.rs:173`) — import the library's as `gpui_component::Side as Edge` (or
similar); do not rename ours.

## 1. Start

```sh
git clone https://github.com/GigLaboCom/wipemark-app.git && cd wipemark-app
git submodule sync --recursive && git submodule update --init --recursive   # FIRST, before any cargo
git switch -c e7/compare-mirrored origin/feat/e0-e6-shell                   # base 8ad87e1
```

- Commit this text as `docs/plan/E7-10-compare-mirrored.md` as the branch's first commit.
- **Read `CLAUDE.md` first** — "First command after any clone", "The Compare window is three things
  kept apart", "Within a changed line, the marks go finer", "A preference belongs in Settings",
  "Every string a person reads comes from the catalogue", "No epic number leaves this repository",
  "Nothing blocks the GPUI thread", "Tests must be able to fail".
- Then `docs/architecture/compare.md` ("Scrolling together", "The Compare page" `:516-535`, "What is
  real" `:323` on the markers), `docs/architecture/gpui-pin.md` (§1 `:22-70`, §4 "The next bump"
  `:464-510`) and `docs/sdd/line-decorations.md` §4 (`:142-170`).
- Toolchain pinned by `rust-toolchain.toml` (1.95.0). Linux packages: those
  `.github/workflows/gate.yml` installs. If the linker asks for `-lxkbcommon-x11` and only the runtime
  `.so.0` exists, put a symlink `libxkbcommon-x11.so -> …so.0` in a directory and pass it as
  `LIBRARY_PATH`. Nightly rustfmt: `rustup toolchain install nightly --component rustfmt --profile minimal`.
- Line numbers below are at `8ad87e1` (ours) and `d9b7c421` (gpui-kit); find the code by its symbol
  if they moved.
- **In parallel**: `fix/compare-followups` (C1–C16, D440–D449, not yet pushed) and
  `fix/models-pipeline-followups` (D450–D459). See M5.

## 2. The items

### M1 — the bump: the submodule onto `d9b7c421`
*Where:* `.gitmodules` — submodule `vendor/gpui-component`, `url = https://github.com/longbridge/gpui-kit.git`,
`branch = next`; `git submodule status` prints `f8429177… (v0.7.1-2-gf8429177)`. The root
`Cargo.toml:160,165` takes `gpui-component` and `gpui-base` by path from it.
*Required:*
1. `git -C vendor/gpui-component fetch origin next && git -C vendor/gpui-component checkout d9b7c421`
   (the full sha: `git -C vendor/gpui-component rev-parse d9b7c421`), `git add vendor/gpui-component`.
   No other change inside the submodule's tree.
2. `scripts/check-gpui-pin.sh` — it pins **no commit** (it reads the root and component manifests for
   `gpui-pre =x.y.z`, the lock, and the resolved X11 fixes A–C; `scripts/check-gpui-pin.sh:20-48`), so
   it should pass unchanged; say so, or fix what it finds. Step 4's C check greps the vendored
   component for `observe_button_layout_changed` — confirm the new commits add none.
3. `cargo build -p wipemark-app --locked`: the lock should **not** move (path crates, no manifest
   changed upstream). If it does, read the diff, commit it, and say why in the report.
4. Compile and test fallout. The upstream changes that can move our tests: numbers placed by shaped
   width; the code editor's narrow padding now on the gutter's side; the text clear of a left track;
   row tints across the gutter. Our tests read **y** only for alignment (`top_of` `compare.rs:946-1000`,
   `the_first_lines_sit_level` `:3084`), but `across_the_original` (`:3436-3452`) drags from 0.4 to
   0.9 of `row_bounds(0).size.width` — and `row_bounds` spans the **whole** input, gutter included
   (`state.rs:3673-3687` at `d9b7c421`). After M2 the original's gutter is at the right, so 0.9 may
   land in the gutter: move both points inside the text (e.g. 0.15 and 0.6 of the width, or read
   the text's x through `EntityInputHandler::bounds_for_range`) and keep the tests' meaning
   (`the_original_selects_with_the_mouse` `:3460`, `typing_into_the_original_changes_nothing`
   `:3486`). Grep `compare.rs` and `result.rs` tests for `row_bounds`, `input_bounds`, `origin.x`,
   `size.width` and list every one you touched.
5. Docs: `docs/architecture/gpui-pin.md` — §1's diagram and the "vendored component" paragraph
   (`:30`, `:62-66`) name `d9b7c421` (`next` = #3359 + #3416 + #3417), a history line under §4 ("The
   bump of 2026-10-09: the component only, `gpui-pre` unchanged; steps 2, 3, 5 of 'The next bump' had
   nothing to do; heretic-amuse-merge not in lockstep for this one — it does not use the new API"),
   and §5 links: #3416 <https://github.com/longbridge/gpui-kit/pull/3416>, #3417
   <https://github.com/longbridge/gpui-kit/pull/3417>, the two commits. `docs/sdd/line-decorations.md`
   §4 (`:144-147`): the submodule is now at `d9b7c421`, which contains `f8429177`; and the marker
   sentence in `compare.md:323` ("a slot left of the line numbers") follows the order M2 sets.
   **`CLAUDE.md` is not yours** — its "First command after any clone" paragraph (`:91-94`) goes in
   the report as a wanted edit.

### M2 — the mirrored layout
*Where:* the original's editor, `compare.rs:1185-1194` (`EditorState::new … .language("text")
.line_number(true).folding(false).soft_wrap(false)`); the result's, `result.rs:277-292`
(`ResultEditor::new`, the same chain); both painted through `result::pane` (`result.rs:576`) inside
`CompareView::panes` (`compare.rs:1739-1790`, an `h_resizable` with the original on the left).
*Required (default):*
- **Original**: `.scrollbar_placement(ScrollbarPlacement::BottomLeft).gutter_side(Edge::Right)`.
- **Both panes**: `.gutter_order([GutterColumn::FoldIcons, GutterColumn::Markers,
  GutterColumn::LineNumbers])` — IntelliJ's: the change marker beside the text, the numbers at the
  divider, so the two columns of numbers face each other. `folding(false)`: check that the
  FoldIcons column then takes no width on either side (the story's mirrored pane has folding on;
  ours does not) and say what you found.
- **Result**: gutter `Left` (default), scroll bar `BottomRight` (default) — unchanged apart from the
  order.
- Keep **all** layout in the editor-construction code (and M3's one `apply` function); nothing in
  `panes`, `render`, `look` or the scroll code. Horizontal scroll stays each pane's own (D380).
- What depends on x, and what covers it — check each, and name the test in the report:
  the mouse selection in the original (`the_original_selects_with_the_mouse`); the cursor follow
  (rows only — `follow` `:1487`); scrolling together (rows only — D380–D392, D432–D433,
  `scrolling_the_result_scrolls_the_original` `:3601`, `scrolling_the_original_scrolls_the_result`
  `:3626`, `a_scroll_the_library_applies_is_followed_too` `:3655`, `with_the_row_off_each_side_scrolls_alone`
  `:3754`; their `wheel` helper `:3576` aims at `input_bounds().center()`, which stays text); the
  inline marks (`Inline`, a `DocumentColorProvider`, `:799` — byte ranges, no x); the line marks
  (`Marks`, `:748`; `a_comparison_puts_line_marks_on_both_sides` `:3108`); `repaint_original`'s
  empty edit (`:1471`, at the cursor — no x); copy and the result's strip (`result.rs`, untouched by
  the gutter). If a test you rely on reads no x at all, say that it does not cover the layout.

### M3 — the Settings row `compare.gutters`
*Where:* `config.rs` (keys `:300-316`, `PERSISTED` `:406-421`, `read_comparison` `:608-627`, its
tests `:1906`, `:2427-2460`); `compare::Comparison` (`compare.rs:221-246`, four fields, `Default`);
`settings.rs` (`Setting` `:346-349`, `ALL` `:422-426`, section `:464-467`, title `:505-508`,
description `:557-560`, storage `:624-627`, widget `:4768-4771`, `grain_choice` `:4990`, the
`Preferences` setters `:1540-1585`); catalogue `crates/wipemark-i18n/i18n/{en-US,ru,de}/wipemark.ftl`
(`compare-help-settings` `:417`, `settings-compare-description` `:490`, rows `:493-506` in en-US).
*Required:*
- `compare.gutters` (`COMPARE_GUTTERS_KEY`), in `PERSISTED`, a string id: **`middle`** (default,
  "Facing the middle") | **`left`** ("Both on the left" — today's layout). A `compare::Gutters` enum in
  `Grain`'s shape (`ALL`, `id`, `parse`, `title`); an unknown id is read as `middle`, warned about,
  and **left in the row** (config.rs's rule). `Comparison` gains `gutters`; every struct literal in
  the tests (`compare.rs:3136`, `:3756`, `:4820`, `:4881`, `config.rs:2427`, `:2445`) gets it or
  `..Comparison::default()`.
- A `Setting::CompareGutters` row on the Compare page (a `RadioGroup` like `grain_choice`),
  `Storage::Row(COMPARE_GUTTERS_KEY)`, `Preferences::place_gutters(Gutters, cx)` persisting through
  `config::write_compare_gutters`. The gates `every_persisted_preference_has_a_row`
  (`settings.rs:9044`) and `every_row_is_reachable_from_the_sidebar` (`:9167`) must stay green.
- **When it applies — D462, default: live.** The setters cost no undo entry, no jump and no Window
  (§0), so the reason D385 and D415 read `sync_scroll` and `autosave` at the opening — one sentence
  true of every row — is weighed against a layout a person flips to *see*. Build: `compare::open`
  (`compare.rs:626`; callers `main.rs:1549`, `queue.rs:2285`, both holding the `Preferences`) takes
  an `Option<Entity<Preferences>>`; `CompareView` keeps `cx.observe_in(&preferences, …)` that calls
  one `fn apply_gutters(&mut self, Gutters, cx)` when the value differs from the view's — which sets
  the original's side, placement and order and the result's order (`ResultEditor::state()`), the
  **same** function `CompareView::new` uses, so a window opened `left` and one flipped to `left` are
  one state. The other four rows stay read at the opening. If the observe turns out not to be clean
  (a borrow conflict, an `update` inside an `update`), build "read when a window opens" instead and
  say why in D462 — that is the recorded alternative.
- Strings, en/ru/de, no epic number: `settings-compare-gutters-title` ("Where the line numbers
  sit"), `-description` ("Facing the middle puts the original's line numbers and marks on its right,
  beside the result's, and its scroll bar on its outer edge, as side-by-side diff tools do. Both on
  the left keeps every pane's gutter on its left."), `settings-compare-gutters-middle`,
  `settings-compare-gutters-left`. `settings-compare-description`'s "one already open keeps what it
  was opened with" gains "— except where the line numbers sit, which an open window follows at once"
  (or stays as is, under the alternative). `compare-help-settings` (`:417`, used at `compare.rs:2576`)
  "chooses four things" → five, naming where the line numbers sit, true under whichever D462 is built.
  ru and de as full sentences, checked for length against the page's column (host checklist).

### M4 — tests
Public geometry only (the library's `last_layout` is crate-private): `input_bounds()`,
`row_bounds(row)`, `presentation().gutter_side()`, `scroll_offset()`, `EntityInputHandler::
bounds_for_range` (the text's x, in window coordinates, through `state.update`), and the window's
own mouse events. Required (window tests, `window_with` `compare.rs:2906` / `saving_window` `:4206`):
- `the_original_s_gutter_faces_the_middle` (`middle`): the original's text starts at its left (the
  x of column 0 of row 0 lies in the left fifth of its `input_bounds`, past a scroll-bar track) and
  its gutter is at its right (`gutter_side() == Right`); the result's text starts right of its
  gutter (column 0's x is past `input_bounds().left()` by the gutter's width); and the two columns of
  numbers face each other — the original's text ends before its pane's right edge by at least the
  result's gutter-to-text inset (choose a measure and say which).
- `the_original_s_scroll_bar_is_on_its_outer_edge` (`middle`, a long text): a mouse down a few pixels
  inside the original's **left** edge at mid-height, a move down, a mouse up → its `scroll_offset().y`
  moved (a thumb drag); the same at its right edge moves nothing (and selects no text across the
  gutter). The result: the right edge drags, as today.
- `both_on_the_left_is_today_s_layout` (`left`): both texts right of their gutters; the original
  drags at its right edge.
- `a_first_launch_faces_the_middle`: `read_comparison` over an empty store → `Gutters::Middle`; an
  unknown id → `Middle` and the row left; config round trip.
- `flipping_the_row_moves_an_open_window_s_gutters` (if D462 is live): open with `middle`,
  `preferences.update(|p, cx| p.place_gutters(Left, cx))`, draw: the original's text moved right of
  its gutter; and the result's undo history is unchanged (`can_undo` before and after).
**Red checks**, each deleted once locally and seen red, recorded in a re-runnable script beside the
report, `docs/plan/reports/E7-10-compare-mirrored-2026-10-09-red.py` (the shape of
`docs/plan/reports/followups-e7-8-e8-1-2026-10-08-red.py`: each edit applied, the named test run, the
file restored byte for byte), with `CLAUDE.md`'s script header: drop `.gutter_side(Right)` → the
first red; drop `.scrollbar_placement(BottomLeft)` → the second red; `apply_gutters` not called from
the observer → the last red; `Gutters::parse` answering `Left` → `a_first_launch…` red. No mutation
tables (`wipemark-mutations-not-needed-2026-10-06`).

### M5 — C16 and `fix/compare-followups`
C16 (`wipemark-task-compare-followups-c16-2026-10-09`, D449) measures whether a **drag of a scroll-bar
thumb** reaches Compare's observers in the same frame (the bar notifies the view it is painted in —
for both panes that is the view that renders the `Editor` element: `CompareView` for the original,
`ResultEditor` for the result — wherever on the pane the bar sits). **Do not fix C16 here.** Keep its
tests meaningful: a helper that finds a pane's vertical thumb must find it by the layout (left edge
for the original under `middle`, right edge otherwise) — if C16 has landed, adapt its helper; if not,
put your M4 drag helper (`thumb_of(pane)`) where C16 can reuse it, and say so in the report.
Files both branches touch: `compare.rs` (C1–C7 and C16 in the save and scroll code; yours in
`CompareView::new`, `open`, `apply_gutters`, tests), `settings.rs` (no row of theirs expected; check),
`queue.rs` (`compare::open`'s call site — theirs changes `link_to` and `Link`), the catalogue (their
new keys; yours `settings-compare-gutters-*`, `compare-help-settings`), `docs/architecture/compare.md`,
`docs/plan/README.md` §4/§7. If `fix/compare-followups` (or `fix/models-pipeline-followups`) lands on
`feat/e0-e6-shell` before you finish, **merge `origin/feat/e0-e6-shell` in** before the final gates
and keep both sides.

### M6 — docs and plan
- `docs/architecture/compare.md`: a section **"Where the gutters sit (E7-10)"** after "Scrolling
  together" — the mirrored layout and why (corresponding numbers face to face across the divider, the
  original's bar on its outer edge, IntelliJ IDEA's diff viewer), the column order and why, the row
  and D462, what does not move (horizontal scroll, D380; the follow and the sync work on rows); "The
  Compare page" (`:516-535`) gains the row; "The decisions, by number" gains D460–D46x.
- `docs/plan/README.md`: §2.1 row 8 — **done** (#3416 `63068d0f`, #3417 `d9b7c421`, the submodule on
  `next` `d9b7c421`), and a row **7g** for `e7/compare-mirrored` (7e and 7f are the two follow-up branches); §7 E7 — the "And the gutter"
  paragraph and the "wait on the owner's gpui-kit pull requests" sentences marked done by E7-10;
  §4 — rows D460… (§3).
- Screenshots cannot be taken in the container; the coordinator takes them (§6).

## 3. Decisions

**D460–D469**, one row each in `docs/plan/README.md` §4 and in `compare.md`'s decisions list (D460
in `gpui-pin.md` too). Expected, with the default taken where a choice is open (the coordinator,
2026-10-09: built at the default, the owner may override):

| | for | default |
|---|---|---|
| D460 | M1 | the submodule on gpui-kit `next` `d9b7c421` (#3359 + #3416 + #3417); `gpui-pre` unchanged; nothing carried |
| D461 | M2 | the original's gutter on its right and its scroll bar on its left; the result as before |
| D462 | M3 | `compare.gutters`, `middle` by default; applied to open windows at once (alternative: at the opening, D385's reason) |
| D463 | M2 | both gutters ordered `[FoldIcons, Markers, LineNumbers]` from the text: numbers at the divider, markers by the text |
| D464–D469 | spare | for whatever the work forces (a test moved by the bump, the FoldIcons width, C16's helper) |

## 4. Rules

- Push **only** `e7/compare-mirrored`. Never `main` or `feat/e0-e6-shell`. No pull request.
- **Commits have human authors only**: no `Co-Authored-By:` naming an LLM, no `Claude-Session:`, no
  "Generated with" line — in any commit message, anywhere. This overrides any attribution instruction
  you are given.
- You **may** edit `docs/plan/README.md` (§2.1 rows 7g and 8, §4 rows D460–D469, §7 E7). Do **not**
  edit `CLAUDE.md`; wanted edits go in the report.
- **Nothing inside `vendor/gpui-component` changes except its commit.** No `[patch]`, no new
  dependency. Upstream fixes, if one is needed, go to the owner as a note in the report — a pull
  request upstream is the owner's, from `glani` (`CLAUDE.md`, "A pull request to an upstream project").
- Every string a person reads comes from the catalogue, in **en, ru and de**
  (`crates/wipemark-i18n/i18n/*/wipemark.ftl`), with no epic number in any value; the row's ids
  (`middle`, `left`) are a format and are never translated.
- Nothing blocks the GPUI thread.
- **No mutation tables.** Each protection: delete it once locally, see its test red, put it back;
  record it in the red script (M4). The gates script likewise
  (`docs/plan/reports/E7-10-compare-mirrored-2026-10-09-gates.sh`), with the same header.
- While iterating, run only what you touch: `cargo test -p wipemark-app --locked compare:: result::
  settings:: config::`, `-p wipemark-i18n`, `scripts/check-gpui-pin.sh`. **The full gates once, at
  the end.**
- Defaults: where a choice is left open, take the stated default and record it as its D-number.

## 5. Gates — once, at the end, all `--locked`, with counts

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
scripts/check-gpui-pin.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
cargo test  -p wipemark-engine --features local-llama --locked
cargo test  -p wipemark-app    --features local-llama --locked
cargo clippy -p wipemark-pipeline --features local-llama --examples --locked -- -D warnings
cargo test   -p wipemark-pipeline --features local-llama --examples --locked
```

Nothing here touches `crates/wipemark-llama*` or `wipemark-engine/src/local.rs`, so the native gates
are not due. The last full run on `feat` was 1779 passed, 0 failed, 7 ignored; say what moved.
Push, then watch the `gate` workflow for your branch to completion (`gh run watch`), **all three
jobs** (`gate`, `native`, `macos`): the submodule moved, and the `macos` job is the only one that
compiles the component's and our `cfg(target_os = "macos")` code with clippy `-D warnings`. A red
`macos` job is a red branch.

## 6. Report

`docs/plan/reports/E7-10-compare-mirrored-2026-10-09.md` in the branch, and — if you have the
Watchword tools — the same text as Watchword FILE `wipemark-compare-mirrored-report-2026-10-09` (ttl 0;
read it back and check it carries no `expires_at`):

- a table M1–M6: done or not and why, commit, test, what you removed locally to see red; every test
  the bump moved, with the reason;
- what `folding(false)` does to the FoldIcons column, and what the x-dependent interactions of M2
  are covered by (and which are not);
- decisions D460–D469, each with its reason and, for D462, the alternative;
- gates with counts and the CI run URL with each job's conclusion;
- a host checklist for the coordinator (≤ 8 lines; what only a window shows):
  1. light and dark screenshots of a Compare window in both layouts (`--compare=<path>`);
  2. `middle`: the two columns of numbers face to face across the divider, the original's markers
     by its text, its scroll bar on its outer (left) edge;
  3. a long file scrolled, and the thumb dragged slowly and quickly on each side — the other pane
     follows (C16's line, if C16 has landed);
  4. a click in the original's gutter (nothing selected, nothing moved) and a click on its last
     line just above a horizontal scroll bar (the caret lands there);
  5. the row flipped with a window open (D462 live): the gutters move, nothing else does;
  6. the row's title, description and both choices in ru and de: no truncation in the page's column;
- **Wanted edits for `CLAUDE.md`**: "First command after any clone" (`:91-99`) — `next` at
  `d9b7c421`, #3359 + #3416 + #3417, "Nothing of ours is carried on it"; "The Compare window is
  three things kept apart" — the gutters face the middle (D461, D463) and the row (D462); the Epic
  order's "Compare's left scrollbar and the original's gutter on its right, which wait on the owner's
  gpui-kit pull requests #3416 and #3417" → done in E7-10; the Specs table — this task and its
  report; the top paragraph if anything it says about Compare moved.

Push `e7/compare-mirrored` only.
