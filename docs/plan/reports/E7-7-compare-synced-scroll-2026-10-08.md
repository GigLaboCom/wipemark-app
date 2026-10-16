# E7-7 — the Compare window scrolls both panes together: report

- **Task:** `docs/plan/E7-7-compare-synced-scroll.md` (Watchword FILE
  `wipemark-task-compare-synced-scroll-2026-10-08`, the coordinator, for
  the owner, 2026-10-08).
- **Branch:** `e7/compare-synced-scroll`, from `feat/e0-e6-shell` at
  `718899c` (+ the task commit `3c1f604`). `feat/e0-e6-shell` had not
  moved when the gates ran (checked with a fetch at the end).
- **Commits:** `12d02aa` (the change, its tests, the docs, the red-check
  script), then this report and `gates.sh`.
- **Built in a cloud container** that compiles and runs tests and has no
  display: no window was opened. What only a live window shows is the
  checklist at the end.

## S1–S4

| | what | done | where | tests | removed locally → red |
|---|---|---|---|---|---|
| S1 | the row map both ways | yes | `diff.rs`: `Diff::result_row_of` (mirror of `original_row_of`, one private `row_across` behind both), and `Diff::position_across(from, position)`, the map over a row and a fraction that the scrolling uses | `a_row_of_the_original_names_a_row_of_the_result` (equal texts, insertion, deletion, a 3→1 replacement, the end of the text, both empty sides); `on_every_shared_line_the_maps_undo_each_other` (400 generated pairs: the row maps inverse on every shared line, the position maps carry the fraction both ways); `inside_a_hunk_the_other_side_moves_in_proportion`; `the_position_map_never_goes_back` (monotonic, 200 pairs × both sides) | the proportional branch of `position_across` replaced by "hold at the block's start" → `inside_a_changed_passage_the_other_side_moves_in_proportion` red (and the pure proportion test) |
| S2 | scrolling one pane scrolls the other | yes | `compare.rs`: `CompareView::scrolled`, `drive`, `painted`, `follow`; `Track`, `Asked`, `top_of`, `offset_for`, `wrapped_offset_for` | `scrolling_the_result_scrolls_the_original` (a wheel of 10.25 rows; the original at 7.25 — three inserted lines, the quarter row kept); `scrolling_the_original_scrolls_the_result`; `a_scroll_the_library_applies_is_followed_too` (an offset handed to the editor, the way the keyboard's lands, both directions); `inside_a_changed_passage_the_other_side_moves_in_proportion`; `a_follower_that_stops_short_does_not_lead_back` (the feedback loop: leads counted per pane, `(1, 0)` after one scroll, and a later scroll of the follower still leads); `with_the_row_off_each_side_scrolls_alone`; `the_cursor_follow_does_not_drag_the_result`; `a_wrapped_result_still_leads_and_follows`; `a_wrapped_result_far_away_is_placed_by_estimate` | `Side::Result` out of both callers of `scrolled` → `scrolling_the_result_scrolls_the_original` red; `Side::Original` out of both → `scrolling_the_original_scrolls_the_result` red; the end-of-frame look (`painted`) emptied → `a_wrapped_result_still_leads_and_follows` red; `Asked::lands` always false → `a_follower_that_stops_short_does_not_lead_back` red (the original dragged back to the result's end); the `sync_scroll` check out of `scrolled` → `with_the_row_off_each_side_scrolls_alone` red; the drive in `follow` disabled → `the_cursor_follow_does_not_drag_the_result` red; the wrapped placement replaced by row × line height → `a_wrapped_result_far_away_is_placed_by_estimate` red |
| S3 | the Settings row, on by default | yes | `config.rs` `COMPARE_SYNC_SCROLL_KEY = "compare.sync_scroll"` in `PERSISTED`, `read_comparison`, `write_compare_sync_scroll`; `Comparison::sync_scroll` (default `true`); `settings.rs` `Setting::CompareSyncScroll` on the Compare page, `Preferences::scroll_together`, `sync_scroll_switch`; catalogue en/ru/de `settings-compare-sync-scroll-{title,description}` and `compare-help-scrolls`; the help popover's line | `the_compare_rows_default_to_words_and_survive_a_restart` (default true, `false` survives a reopen); `an_unusable_compare_row_falls_back_without_being_rewritten` (`"no"` and `"1"` read as the default and left in the row); `every_persisted_preference_has_a_row`, `every_row_is_reachable_from_the_sidebar` and the i18n gates green | the row read replaced by the default → `the_compare_rows_default_to_words_and_survive_a_restart` red |
| S4 | docs | yes | `compare.rs` module docs, "The two sides stay in step: by scrolling, and by the cursor"; `docs/architecture/compare.md`, "Scrolling together" and "Following", the Compare page, "What it is not", the amuse-merge note (the scroll sync is no longer "not taken because there is no setter") | — | — |

The red checks are `docs/plan/reports/E7-7/red-checks.py` — each edit
applied, the named test run, the file restored byte for byte. Its last
run: **9 of 9 red**. One check first came back **green**: the
end-of-frame look tested on an unwrapped pane. The editor notifies its
observers after any frame in which its text moved, so for a pane that
does not wrap the observer alone already sees a scroll the library
applied silently in the frame; the end-of-frame look is load-bearing
only for a **wrapped** pane, which the observer path deliberately does
not read (D386). The test that went green was renamed and re-described
for what it does guard (`a_scroll_the_library_applies_is_followed_too`),
and the check now targets the wrapped test, which goes red.

## Decisions

- **D380 — inside a changed passage, the other side moves in
  proportion.** A third of the way through five lines here is a third of
  the way through two lines there; through lines the other side has none
  of, it holds at the place they stand in front of. Chosen over holding
  at the hunk's start because a follower that held and then caught up
  would jump by the size of the block at its end; in proportion it moves
  continuously wherever the leading side has lines. The one jump left is
  past a block only the *other* side has, which the leading side crosses
  in no distance. This is also what IntelliJ's diff viewer does.
- **D381 — the anchor is the top of the viewport, a row and a fraction.**
  In a shared stretch the fraction is carried across, so the two panes
  move by pixels, not a row at a time
  (`offset = −(mapped position × line height)`). Vertical only; each side
  keeps its own horizontal scroll.
- **D382 — the guard is what was asked, not a flag with a lifetime.** Per
  pane the window keeps the offset it last saw and the last ask
  `{from, to}`. A move in the asked direction, no farther than asked, is
  that ask landing — exactly, or stopped short by the pane's own end (a
  follower with fewer lines, at the bottom) — and anything else is the
  pane's own. An ask that changed nothing (the pane already at its end)
  cannot swallow a later move, because from an end a person can only
  scroll the other way. Before a new ask, an earlier ask's unseen landing
  is taken as landed. A "being driven until the next frame" flag was
  rejected: a scroll set through `set_scroll_offset` lands silently
  inside the next frame, frame callbacks run *before* that frame's draw,
  and the test platform does not run them at all.
- **D383 — the result leads the cursor follow.** With both rows on, a
  follow places the original's caret and then sets the original's scroll
  where the result's top puts it, replacing the scroll the caret had
  queued; the result is never moved by a follow. When both panes moved in
  one frame, the result is looked at first. Without this the original
  scrolls only as far as its caret and then leads the result back to it,
  the result's caret out of sight — the red check above.
- **D384 — a wrapped pane is placed off its last layout.** The library
  does not expose how many lines a wrapped row became. A wrapped pane's
  top is the first row its last layout showed plus how far the viewport
  is into that row (`row_bounds`, `input_bounds`); a wrapped pane is
  placed on a laid-out row to the pixel, and on a row not laid out by the
  average height of the rows that are (above the first shown: the exact
  height above it ÷ its row number; below the last: the height down to it
  ÷ its rows). The next step of the leading pane, the row now laid out,
  lands on it. Approximate; the page says so in all three languages.
- **D385 — the row is read when a window opens**, like `compare.grain`.
  Live would have been possible here — turning it on costs no undo entry
  and no jump — but the Compare page has one sentence for all its rows
  ("a window reads these as it opens; one already open keeps what it was
  opened with"), and a row that broke it would need that sentence split.
- **D386 — a scroll is seen two ways.** The editor's notification (a
  wheel and the scroll bar at once; a scroll the library applies while
  laying out — the keyboard's, a caret brought into view, a follower's
  landing — through the notification the library makes after a frame in
  which its text moved), and a zero-sized `canvas` painted after both
  panes whose paint defers a look at both to after the frame. Deferred,
  because a `set_scroll_offset` made *inside* a frame's paint is lost
  with the frame (measured: the follower never moved). The end-of-frame
  look is the only reader of a wrapped pane, whose layout agrees with its
  offset only then.
- **D387 — one `Side` enum.** `diff::Side` names the side a row is counted
  on; `compare` had an identical enum for the side a mark is painted on,
  and now uses `diff::Side`.

Also changed in passing: the follow row's description said "Off, each
side scrolls on its own", which is no longer true with scrolling together
on; it now says what the follow does to the original's caret (en, ru, de),
and `compare-help-settings` names the third row.

## Gates

On `12d02aa` (then the report commit, docs only), all `--locked`, through
`docs/plan/reports/E7-7/gates.sh`:

| gate | exit | counts |
|---|---|---|
| `rustup run nightly rustfmt --edition 2021 --check …` | 0 | |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | |
| `cargo test --workspace --locked` | 0 | 1636 passed, 0 failed, 7 ignored (1623 before; +13 here) |
| `scripts/check-dep-direction.sh` | 0 | |
| `scripts/check-gpui-pin.sh` | 0 | |
| `cargo check --workspace --no-default-features --locked` | 0 | |
| `cargo check --workspace --features local-llama --locked` | 0 | |
| `cargo test -p wipemark-app --features local-llama --locked` | 0 | 641 passed, 0 failed, 2 ignored |

No llama crate and no `local.rs` changed, so the native gates were not
due.

CI on `d799231` (E7-7 alone, before the merge below): `gate` run 169,
<https://github.com/GigLaboCom/wipemark-app/actions/runs/37769229207> —
**success** in all three jobs: `gate (fmt, clippy, test, deps, features)`
success, `native (llama.cpp prebuilt + Vulkan, model-free)` success,
`macos (clippy, tests, llama-native prebuilt with Metal)` success.

## Everything of the last two days in this branch

At the owner's request (2026-10-08), `e7/compare-synced-scroll` now holds
everything pushed in the last two days. Every branch on `origin` with a
commit dated 2026-10-07 or later was already an ancestor except one,
`fix/consent-and-lows` (the queue's consent checked against the engine
handed out, and its Lows, D370–D375), which is merged in as `6ff95e2`:
no conflicts, and it does not touch Compare. Contained after the merge,
checked with `git merge-base --is-ancestor`: `fix/consent-and-lows`,
`docs/after-integrate-2026-10-08`, `feat/e0-e6-shell`,
`integrate/2026-10-08`, `fix/integrate-e4-6b`, `fix/integrate-models-tray`,
`fix/integrate-e4-6c`, `e4/windows-rewrite`, `fix/owner-2026-10-07`,
`e10/linux-tray`, `e4/templates-widgets`, `research/divergence-vs-upstream`,
`e7/windows-clean`. Not merged: `origin/main`, which is GitHub's
`Initial commit` of 2026-09-07 (a two-line README, unrelated history).

The gates again on the merged tree (`6ff95e2`, `gates.sh`): every one
exit 0; `cargo test --workspace` **1650 passed, 0 failed, 7 ignored**;
`cargo test -p wipemark-app --features local-llama` **647 passed, 0
failed, 2 ignored**. The CI run for the merged head is the one after
the push of this report.

## For the owner, in a window (≤ 8 lines)

Open Compare on a long text whose result differs (`--compare=<path>` on a
file with several edited paragraphs; edit the result for more), then:

1. Wheel/touchpad over the result: the original moves with it, shared lines level to the pixel; same the other way.
2. Drag and click the scroll bar of either pane, and PageDown/PageUp/⌘↓ in either: the other follows (keyboard after the frame).
3. Through a passage that changed with unequal line counts: the other side moves slower or faster through its block, never stuck then jumping; past a block only the other side has, it skips it.
4. Scroll the longer side to its end: the shorter one stops at its end and the longer one is not pulled back.
5. Click and arrow through the result: the original's caret follows, and the result never moves on its own.
6. Turn on wrapping in the result's toolbar: alignment is approximate (rows the result wrapped), never a jitter between panes.
7. Settings › Compare › "Both sides scroll together" off, open a new window: each side scrolls alone; an open window keeps its value.

## Wanted edits

- **`CLAUDE.md`**, the Compare bullet ("The Compare window is three
  things kept apart…"): after "The original **follows the result's
  cursor** — … in the same update, before GPUI compares focus paths at
  the next frame." add: "And the two panes **scroll together**
  (`compare.sync_scroll`, on by default, read when a window opens):
  `Diff::position_across` maps the leading pane's top — a row and a
  fraction — line for line in a shared stretch and in proportion through
  a changed passage (D380, D381); a scroll is seen by each editor's
  notification and by a look after every painted frame (D386), a
  follower's landing is told from a lead by what was asked (D382), the
  result leads the cursor follow (D383), and a wrapped result lines up
  approximately (D384). `a_follower_that_stops_short_does_not_lead_back`
  is the gate on the loop." In "What remains", strike "Compare scrolling
  both panes together (on by default) and" from E7's list. The Compare
  bullet's "the rule that keeps the two sides in step" in `compare.rs`'s
  row of the file table can stay.
- **`docs/plan/README.md`**: §7, E7's "Compare scrolls both panes
  together" bullet → done (`e7/compare-synced-scroll`, D380–D387); §4,
  rows D380–D387 as above (one line each: proportion inside a hunk; the
  top as a row and a fraction, vertical only; the guard by what was
  asked; the result leads the follow; wrapped panes off the last layout;
  the row read at opening; two ways to see a scroll; one `Side`).
