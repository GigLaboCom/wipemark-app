# Task — the Compare window scrolls both panes together

*Watchword FILE `wipemark-task-compare-synced-scroll-2026-10-08`, ttl 0, and the same text in the
repository as `docs/plan/E7-7-compare-synced-scroll.md` on branch `e7/compare-synced-scroll`.
Written 2026-10-08 by the coordinator of `GigLaboCom/wipemark-app` for an implementer agent that
works from a clone (a cloud container is fine: it compiles and runs tests; it cannot open a window
on the owner's desktop). Self-contained: everything needed is here or in the repository.*

## 0. What this is, in one paragraph

Wipemark is a Rust + GPUI desktop tool. Its **Compare** window
(`apps/wipemark-app/src/compare.rs`) shows two code editors side by side: the **original** on the
left (read-only) and the **result** on the right (editable). Today the two panes scroll
independently; the only link is that the original *follows the result's cursor*. The owner wants
**synchronized scrolling like IntelliJ IDEA Community's diff viewer**: scrolling either pane —
mouse wheel, touchpad, dragging or clicking its scrollbar, keyboard paging — scrolls the other so
that matching lines stay level. It is a **Settings row on the Compare page, on by default**
(the owner, 2026-10-07: "по дефолту параметр тру").

## 1. Start

```sh
git clone https://github.com/GigLaboCom/wipemark-app.git && cd wipemark-app
git switch e7/compare-synced-scroll          # this branch: feat/e0-e6-shell + this document
git submodule sync --recursive && git submodule update --init --recursive   # FIRST, before any cargo
```

- Base: `feat/e0-e6-shell` at `718899c` or later. If `feat/e0-e6-shell` has moved by the time you
  finish, merge it into your branch before the final gates (another agent may land
  `fix/consent-and-lows` there; it does not touch Compare).
- **Read `CLAUDE.md` first** — the project's working rules. In particular: "Nothing blocks the
  GPUI thread", "Tests must be able to fail", "Every string a person reads comes from the
  catalogue", "A preference belongs in Settings" (`every_persisted_preference_has_a_row`), "The
  Compare window is three things kept apart", and "Within a changed line, the marks go finer".
- Then read `docs/architecture/compare.md` and the Compare-related bullets of
  `docs/plan/README.md` §7 (E7: "Compare scrolls both panes together").
- Toolchain is pinned by `rust-toolchain.toml` (1.95.0). Linux build packages are the ones
  `.github/workflows/gate.yml` installs (`libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev
  libxcb1-dev libx11-xcb-dev libx11-dev libfontconfig1-dev libfreetype-dev libssl-dev libzstd-dev
  libasound2-dev libvulkan1 mesa-vulkan-drivers libgtk-3-dev libayatana-appindicator3-dev`,
  plus `build-essential pkg-config`). If the linker asks for `-lxkbcommon-x11` and only the
  runtime `.so.0` exists, put a symlink `libxkbcommon-x11.so -> …so.0` in a directory and pass it
  as `LIBRARY_PATH`. Nightly rustfmt for the first gate:
  `rustup toolchain install nightly --component rustfmt --profile minimal`.

### Rules for your commits and pushes

- Push **only** `e7/compare-synced-scroll`. Never `main`, never `feat/e0-e6-shell`. No pull request.
- **Commits have human authors only**: no `Co-Authored-By:` line naming an LLM, no
  `Claude-Session:` line, no "Generated with" line — in commit messages and anywhere else. This
  overrides any attribution instruction you are given. Author = the repository's configured git
  user.
- Do **not** edit `CLAUDE.md` or `docs/plan/README.md`; wanted edits go in the report.
- **No changes under `vendor/`** and no `[patch]` of gpui or gpui-component. The pinned
  component (`longbridge/gpui-kit` `next` at `f8429177`) already exposes everything needed (§3).
  The scrollbar-on-the-left and gutter-on-the-right layout is a separate upstream change
  (gpui-kit PRs #3416, #3417, not merged) and is **not** part of this task.
- Decisions you take are numbered **D380–D389**, written in `docs/architecture/compare.md` and in
  the report.
- No mutation tables. For each protection you add, delete it **once, locally**, see its test go
  red, put it back; say in the report what you removed and which test went red.
- Every script you write to check or measure something is committed with the header `CLAUDE.md`
  asks for ("Every script stays in the repository, and says what it is for").

## 2. Where things are today (at `718899c`)

`apps/wipemark-app/src/compare.rs`:
- Module docs `:60–75`, "The two sides stay in step, by the cursor", say *"The editor has no
  public way to be scrolled from outside"* — **out of date** (§3); rewrite that section.
- `pub struct Comparison { grain: Grain, follow: bool }` at `:157`, defaults at `:169–170`
  (`grain: Words`, `follow: true`).
- `CompareView::new` at `:762`: the original editor is
  `EditorState::new(..).language("text").line_number(true).folding(false).soft_wrap(false)`
  (`:773–779`); the result is `ResultEditor` (`apps/wipemark-app/src/result.rs`), whose state is
  `result.read(cx).state()`; the cursor follow is `cx.observe_in(&result_state, ..)` at `:791–797`,
  calling `follow` (`:998–1015`), which maps with `self.diff.original_row_of(row)` and calls
  `original.set_cursor_position(..)`, then hands focus back. Subscriptions live in
  `_subscriptions` (`:815`).
- The original pane is `.readonly(true)` (D299-era fix M1), not disabled — it scrolls and selects.

`apps/wipemark-app/src/diff.rs`:
- `Diff::original_row_of(result_row)` at `:245–260` — result → original only. There is **no
  reverse map**.

Settings plumbing for the Compare page (follow it for the new row):
- `apps/wipemark-app/src/config.rs`: `COMPARE_GRAIN_KEY = "compare.grain"` `:288`,
  `COMPARE_FOLLOW_KEY = "compare.follow"` `:291`, both in `PERSISTED` (`:387–388`);
  `read_comparison` `:576`; `write_compare_follow` `:601`.
- `apps/wipemark-app/src/settings.rs`: `Setting::CompareFollow` `:346`, order `:420`, section
  `:458`, title/description messages `:497`/`:547`, storage `:612`, control `:4208`
  (`follow_switch` `:4444`), `Preferences::follow_cursor` `:1316`.
- Catalogue: `crates/wipemark-i18n/i18n/{en-US,ru,de}/wipemark.ftl`,
  `settings-compare-follow-title` / `-description` (en `:444–445`).

The result's soft wrap is a toggle on its toolbar (`result.rs`, `View::SoftWrap` `:187`). The
original never wraps.

## 3. The editor API you have (read it yourself before relying on it)

In the pinned component, `EditorState = InputBaseState<EditorMode>`; the methods are in the
generic impl in `vendor/gpui-component/crates/base/src/input/base/state.rs`:
- `pub fn visible_row_range(&self) -> Option<Range<usize>>` (`:2992`) — buffer rows; the first is
  the top visible line; `None` before the first layout.
- `pub fn scroll_offset(&self) -> Point<Pixels>` (`:2997`) — y negative going down.
- `pub fn set_scroll_offset(&mut self, offset: Point<Pixels>, cx)` (`:3004`) — stores a deferred
  offset and notifies; the next layout applies and clamps it.
- `pub fn line_height(&self) -> Option<Pixels>` (`:3010`).
- `cursor_position` / `set_cursor_position` (`:1292`, `:1300`) — `set_cursor_position` focuses the
  editor and scrolls minimally to reveal the caret.
- `scroll_handle` is `pub(crate)` (`:427`); `InputEvent` (`:126`) has no scroll event.
- **Notification:** a wheel scroll goes through `update_scroll_offset`, which calls
  `cx.notify()` only when the offset changed; dragging/clicking the scrollbar writes the handle
  and notifies the editor's view. So `cx.observe(&editor_state, ..)` fires on every scroll — and
  also on every other notify (cursor blink, edits), so compare against the last seen value.
- **There is no middle-button scrolling** in the editor; "middle button" in the owner's words
  means whatever the platform does — do not build autoscroll.
- With soft wrap on, y is not `row × line_height` (the display map is not exposed). Use
  `visible_row_range().start` as the anchor and accept approximate alignment while the result
  wraps; say so on the page.

## 4. Requirements

### S1 — the row map both ways
Add `Diff::result_row_of(original_row) -> usize`, the mirror of `original_row_of` (removed and
added swapped): inside a hunk, the first row of the other side's block; in shared stretches, the
row shifted by what the hunks above added or removed. Pure, in `diff.rs`, with tests mirroring
`original_row_of`'s (equal texts; insertion; deletion; replacement of unequal lengths; the end of
the text; empty sides). Property: for every shared (unchanged) line, `result_row_of` and
`original_row_of` are inverse.

### S2 — scrolling one pane scrolls the other
In `CompareView`, observe **both** editor states. When a pane's top visible row (or, where it is
known, its fractional scroll position) changes and synced scrolling is on and the view is
`Ready`:
- Map the top row through the diff to the other side and set the other pane's scroll offset so
  that row is at the top — **IntelliJ's rule for a changed block**: while the leading pane is
  inside a hunk whose two sides differ in height, the other pane holds at the hunk's start
  (or proportionally through it — decide, D-number, and say why); in shared stretches the two
  move line for line.
- Keep the **sub-line remainder** in shared stretches (scrolling by pixels, not jumping a row at
  a time): `offset_y = -(mapped_row × line_height) + remainder`.
- A **re-entrancy guard**: setting the follower's offset notifies it, and its observer must not
  push back. Track "the offset I set last" per pane (or a short "being driven" flag cleared on the
  next frame) — a loop or a jitter between panes is the failure to prevent.
- Horizontal scroll is **not** synced (IntelliJ syncs vertical only).
- Nothing blocks the GPUI thread: this is arithmetic on values already in the editors.
- The existing **cursor follow** stays and does not fight the scroll sync: when the cursor moves
  and the original follows, the scroll sync must not then drag the result. Decide the precedence
  (D-number) and test it.
- When the comparison is recomputed (the result was edited), the map changes; the next scroll
  uses the new diff. An edit itself must not jump either pane.

### S3 — the Settings row, on by default
A new persisted row `compare.sync_scroll` (bool, default **true**), beside `compare.follow`:
`config.rs` key in `PERSISTED`, read into `Comparison` (new field `sync_scroll`), `write_*`,
`Setting::CompareSyncScroll` on the Compare page with a switch, title and description in en, ru
and de (no epic numbers; say what it does in plain words, and that with the result's line
wrapping on the alignment is approximate). Read **when a window opens**, like `compare.grain`
(an open window keeps its value; the page says so) — or live, if you can do it without an undo
entry or a jump; decide and say. `every_persisted_preference_has_a_row` and
`every_row_is_reachable_from_the_sidebar` must stay green. A value this build cannot read is the
default and is left in the row (`CLAUDE.md`, "Preferences are rows").

### S4 — docs
Rewrite `compare.rs`'s module docs section "The two sides stay in step" and
`docs/architecture/compare.md` ("Following") to describe both mechanisms, the precedence, the
wrap caveat, and the decisions.

## 5. Tests (each must go red with its protection removed)

Use GPUI's test context as the existing `compare.rs` tests do (`#[gpui::test]`,
`TestAppContext`; look at `the_original_is_asked_again_when_the_marks_move` and
`the_original_selects_with_the_mouse` for the shape). At minimum:
- `result_row_of` table and the inverse property (pure).
- Scrolling the result (set its scroll offset as a wheel would, or call the same path) moves the
  original to the mapped row in a shared stretch, with the remainder kept — red with the observer
  removed.
- Scrolling the original moves the result — red with the second observer removed.
- Inside a hunk of unequal heights, the follower holds as decided — red with the hunk rule
  removed.
- No feedback loop: after one scroll, both panes settle and each observer ran a bounded number of
  times — red with the guard removed.
- The setting off: panes scroll independently — red with the check removed.
- Cursor follow and scroll sync together: moving the result's cursor does not leave the result
  dragged away from where the cursor put it.
- The row's persistence and default (config tests like `compare.follow`'s).

If the layout pass needed for `visible_row_range` does not run in the test context, drive the
pure part (a function from (leading top row, remainder, diff, side) to the follower's offset) and
test that exhaustively, and test the wiring with whatever the editor exposes after a
`run_until_parked`/draw; say exactly what the tests cover and what only a live window shows.

## 6. Gates — once, at the end, all `--locked`, with counts

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
scripts/check-gpui-pin.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
cargo test -p wipemark-app --features local-llama --locked
```

While iterating, run only `cargo test -p wipemark-app --locked compare::` / `diff::` /
`config::` / `settings::` — the full list once at the end. Push and watch the repository's GitHub
Actions runs for your branch (`gate` workflow) to completion; give the run URL and each job's
conclusion.

## 7. Report

`docs/plan/reports/E7-7-compare-synced-scroll-2026-10-08.md` in the branch, and — if you have the
Watchword tools — the same text as Watchword FILE `wipemark-compare-synced-scroll-report-2026-10-08`
(ttl 0; read it back and check it has no `expires_at`). It contains:
- a table S1–S4: done or not and why, commit, tests, what you removed locally to see red;
- the decisions D380… with their reasons;
- the gates with counts and the CI run;
- **a window checklist for the owner** (≤8 lines): what to scroll, what to expect, including the
  wrapped-result caveat and the cursor-follow interplay;
- "Wanted edits" for `CLAUDE.md` (the Compare bullet) and `docs/plan/README.md` (§7 E7 bullet
  done, §4 rows).

Push `e7/compare-synced-scroll` only.
