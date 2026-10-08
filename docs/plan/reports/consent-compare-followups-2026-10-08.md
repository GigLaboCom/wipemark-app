# E7-8 — the consent and Compare-scrolling follow-ups: report

- **Task:** `docs/plan/E7-8-consent-compare-followups.md` (Watchword FILE
  `wipemark-task-consent-compare-followups-2026-10-08`, the coordinator,
  2026-10-08): the verification findings left open by `fix/consent-and-lows`
  (M-A, M-B, L-1, L-2, L-3) and by `e7/compare-synced-scroll` (M1, L1–L4).
- **Branch:** `fix/consent-compare-followups`, from `feat/e0-e6-shell` at
  `4b54f52` (+ the task commit `54e37de`). The base had not moved when the
  gates ran (checked with a fetch).
- **Commits:** `74db067` (every fix, its tests, the architecture docs, the
  red-check script); then the consent's fallback to the engine handle's
  word (D393, below), this report, the gates script and the red-check
  script's re-spelled edits.
- **Built in a cloud container** with no display: no window was opened. What
  only a window shows is in the checklist at the end.

## The findings

| | what | done | where | test | removed locally → red |
|---|---|---|---|---|---|
| M-A | Replace the existing result pushed with no consent | yes | `queue/rewriting.rs` `replace_rewrite`, `replace_agreed`, `push_rewrites` (consent is a `Whereto`, never `None`; with nothing on duty nothing is pushed); `queue.rs` `QueueEvent::SendAway { replacing }`, `Actions::replace_why`; `main.rs` the Send-away question's yes for a replace | `replace_with_nothing_on_duty_pushes_nothing`; `replace_asks_before_a_document_leaves_the_machine` | the old bypass back (consent `Here` when nothing is on duty, the vacancy check skipped for a replace) → the first red; the Send-away question skipped → the second red |
| M-B | a withdrawn consent question stayed on screen; callers refused on any Ask | yes | `queue/rewriting.rs` (`BatchEvent::Unasked` → `QueueEvent::Unasked`); `main.rs` `Asked::about`, `unask`, `repeats`, `withdraw`; `mcp/rewrite.rs` `QUESTION_GRACE`, `question_stands`, the wait's polled check | `a_withdrawn_question_is_taken_down_and_no_other`, `the_same_question_is_not_stacked` (pure, the shell's two helpers); `a_question_withdrawn_within_its_grace_refuses_nobody` (MCP, end to end over the real queue); `a_yes_to_a_withdrawn_question_changes_nothing` (queue crate — a pin of what was already so, no protection added) | `withdraw` keeping the waiting copies → red; `repeats` always false → red; the grace set to zero → the MCP test red |
| L-1 | items consented to the old endpoint kept starting while a swap was deferred | yes | `engine_host.rs` `Shared::swap_pending`, `EngineHandle::swap_pending`/`set_swap_pending` (raised on `Defer(DutyChanged)`, cleared after the deferred events ran); `wipemark-queue` `EngineSource::settling`, `Worker::settling` | `a_deferred_swap_is_pending_until_it_lands` (host); `no_item_starts_on_the_engine_leaving` (queue crate) | the worker's settling check off → queue test red; the flag never raised → host test red; the flag cleared before the deferred swap ran → host test red (the queue told "look again" with Y still in the slot) |
| L-2 | an endpoint's key read before the consent check | yes | `wipemark-queue` `EngineSource::whereto` (default `None`), checked in `Worker::begin` before `for_item`; `EngineHandle`'s impl reads the slot's `whereto` | `an_item_asked_about_costs_no_engine_build` — a source that counts every engine it builds (the build that reads an endpoint's key): 0 for an item only asked about | the pre-check removed → count 1, red |
| L-3 | a full sha256 could run twice at once | yes | `wipemark-models` `store.rs`: `Downloads::hashing` (in-flight by path), `InFlight`, `Landing` (frees the path and wakes the waiters however the hash ends, a panic included) | `two_askers_for_one_file_read_it_once` — the first hash held mid-read on a zero-capacity progress channel while a second asks: `hashes() == 1` | the in-flight lookup skipped → 2 reads, red |
| M1 | a recompute dragged the result away from where the person typed | yes | `compare.rs` `repaint_original`: the original's offset put back after the empty edit | `a_recompute_moves_neither_pane` — caret at row 300, a selection made near row 10 and wheeled away, a keystroke on the caret's line; after the recompute both offsets are where the keystroke left them | the offset not put back → both panes jump to row 10, red |
| L1 | a leftover ask at a pane's end hid one real scroll | yes | `compare.rs` `Asked::painted`, `painted()` drops an ask that had its frame and moved nothing | `an_ask_that_moved_nothing_does_not_hide_the_next_move` — the result at its end, asked further, sixty lines put in at its bottom, one wheel tick down: it leads | the drop removed → the tick swallowed, red |
| L2 | both panes moving in one frame flickered once | yes | `compare.rs` `look()` (the result first, by every look, the editors' notifications included), `record()` | `two_panes_moved_in_one_frame_end_where_the_result_put_them` — both handed offsets in one update: they end where the result put them, the original never led | looking at the original first, each side by itself → red |
| L3 | the follow row's "brings it into view" was false inside a changed block | yes | `settings-compare-follow-description`, en/ru/de: the clause dropped | wording; the i18n gates (every key in every language, no epic number) | — |
| L4 | `compare-help-settings` read wrong ("… is chosen") | yes | `compare-help-settings`, en/ru/de: "The Compare page of Settings chooses three things …: what is marked, whether the original follows the cursor, and whether the sides scroll together." | wording; the i18n gates | — |

The red checks are `docs/plan/reports/consent-compare-followups-2026-10-08-red.py`
(the shape of `E7-7/red-checks.py`: each edit applied, the named test run,
the file restored byte for byte). Last run: **13 of 13 red**. Two checks came
back wrong first and were fixed before the commit:

- *L-1, the flag cleared before the deferred swap* was **green**: the test
  looked only at the last thing the queue was told, which is right either
  way. It now also asserts the queue is never told "settled" while the
  engine leaving is in the slot, and goes red.
- *M1* was **green**: the test made its selection before the follow, which
  collapses it, so the scenario never happened. The test now places the
  caret first, then selects and wheels the selection off screen, as the
  finding describes.

## Decisions

- **D390 — a recompute moves neither pane.** `repaint_original`'s empty
  edit collapses a selection in the original to a caret, which the editor
  brings into view at the next frame. The original's offset is put back as
  it stood, in the same update, which replaces that scroll before it is
  drawn — whether or not scrolling together is on (with it off, the
  original now also stays where the person left it). Chosen over the
  task's `drive(Side::Original, <its position>, Some(x))` because it is the
  same offset with nothing recorded: an ask from a position to itself
  would only wait to be dropped (D391).
- **D391 — an ask that moved nothing is dropped after its frame.** Each ask
  carries whether a frame has been painted since it was made; at the end of
  a painted frame, an ask that has had its frame and whose pane still
  stands where the ask was made from is dropped. Chosen over "expire on a
  content change of the pane": it also covers an ask that was simply
  already satisfied, and needs no hook into the editor's edits.
- **D392 — the result is looked at first, by every look.** The two editors'
  notifications and the end of a frame all run one `look`: the result
  first; if it led, the original's own move in that look is only recorded.
  The finding put it in `painted`, but the editors' notifications come
  first — the original is painted first, so its notice used to arrive
  first and it led — so the rule has to hold there too.
- **D393 — Replace is a Rewrite by the same road, and a window push always
  carries a consent.** `Built::consent` is a `Whereto`, not an `Option`:
  `push_rewrites` takes the duty's destination — or, where the duty names
  nobody, the engine handle's own word for where its engine sends a
  document (`EngineHandle::sends_to`, the slot's, which the queue checks
  against, D370) — or pushes nothing. The fallback matters wherever the
  preferences and the slot are not one fact: the first full run of the
  suite, without it, refused every rewrite in sixteen table tests whose
  windows put a fake engine in the slot and nothing in the preferences. Replace
  goes through `why_not_rewrite` (greyed with the same reason, as an
  element with the reason under it, as Remove and Rewrite are) and through
  the Send-away question when the document would leave
  (`QueueEvent::SendAway { replacing: Some(path) }`, yes →
  `Queue::replace_agreed`, which takes the consent as the duty stands at
  the yes, as a drop's yes does).
- **D394 — a withdrawn question comes down; a caller is refused only on a
  question that has stood two seconds.** The window tags the consent
  question with where it is about; on `Unasked` it takes the open one down
  if it is that question, drops every waiting copy, and asks the next; a
  consent question already open or waiting is not stacked again. For an
  agent's or the CLI's call, the queue's question is looked at every
  quarter second, before the push and while the item waits; the call is
  refused once the question has stood `QUESTION_GRACE` (2 s). Chosen over
  "refuse when the caller's item is next" — the caller's item never becomes
  next while the question holds the queue, so that rule would never fire —
  and two seconds is several times what a swap takes to land, and short
  beside the call's own wait.
- **D395 — no item starts on the engine leaving.** The engine handle says a
  deferred duty change is pending from the `Defer(DutyChanged)` until the
  deferred events have run on `JobEnded`; clearing it calls the slot's
  watchers, so the queue looks again after the swap and never before. The
  worker waits silently while it is set — not a `Held`, so no caller is
  refused and no window says "waiting for an engine" for a swap.
- **D396 — where first, the key after.** `EngineSource::whereto` (default
  `None`) says where the engine would send the document without building
  it; the worker asks the question there, before `for_item`. The check
  after `for_item`, on the engine actually handed out, stays and still
  decides (D370) — a swap between the two is caught there.
- **D397 — one read of a file at a time, whoever asks.** `Downloads::hash`
  keeps the hashes under way by path: a second asker waits for the first's
  answer and is not counted; a hash that failed answers nobody, and the
  waiter asks again. One `Downloads` is shared by the scan, Remove and a
  fetch (`Preferences::models`), so the guard covers all three.

## Gates

All `--locked`, through
`docs/plan/reports/consent-compare-followups-2026-10-08-gates.sh`:

| gate | exit | counts |
|---|---|---|
| `rustup run nightly rustfmt --edition 2021 --check …` | 0 | |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | |
| `cargo test --workspace --locked` | 0 | 1663 passed, 0 failed, 7 ignored |
| `scripts/check-dep-direction.sh` | 0 | |
| `scripts/check-gpui-pin.sh` | 0 | |
| `cargo check --workspace --no-default-features --locked` | 0 | |
| `cargo check --workspace --features local-llama --locked` | 0 | |
| `cargo test -p wipemark-app --features local-llama --locked` | 0 | 656 passed, 0 failed, 2 ignored |
| `cargo test -p wipemark-engine --features local-llama --locked` | 0 | 39 passed, 0 failed, 1 ignored |

On `28b333a`. The first full run, on `74db067`, had 16 failures — every
table rewrite test, the window's consent refusing a push the preferences
did not name (the fallback in D393 is the fix); targeted runs had not
included them. No llama crate and no `local.rs` changed, so the native
gates were not due.

CI: CI_RUN

## For the owner, in a window (≤ 8 lines)

1. Nothing on duty, a rewrite row refused over an existing file: Replace is greyed with the Rewrite's reason.
2. An endpoint on duty: Replace asks "Send … to …?" first; No leaves the row as it was.
3. Rewrite rows with an endpoint busy, switch duty to this machine: if a "Send … to …?" appears, it disappears by itself as the swap lands, and does not come back twice.
4. Same, with `wipemark-cli rewrite` waiting: the CLI's call completes instead of "answer it there".
5. Compare: select a phrase in the original near the top, wheel the result far down, type on the line the result's caret is on: neither pane jumps after the typing pauses.
6. Compare, result shorter than the original: scroll the original past the result's end, paste lines at the bottom of the result, scroll the result down one tick: the original follows.
7. Settings › Compare: the follow row no longer promises the line is brought into view; the help popover's last-but-one line reads as three choices.
8. Models: Remove a large model while the launch's scan is checking it — one bar, one read.

## Wanted edits

- **`CLAUDE.md`**: in "The table rewrites, through the one batch queue",
  after "…and a duty back on this machine asks nothing." add: "Replace the
  existing result of a rewrite takes the same road — greyed when Rewrite
  is, the Send-away question when it would leave, and no window push is
  built without a consent (D393). A question the queue withdraws comes off
  the screen (D394); an agent's or the CLI's call is refused only on one
  that has stood two seconds. No item starts while a swap the host
  deferred is pending (D395), and where an item would go is checked before
  an endpoint's key is read (D396)." In the Compare bullet, after the
  scrolling sentence: "A recompute moves neither pane (D390); an ask that
  moved nothing is dropped after its frame (D391); the result is looked at
  first by every look (D392)." In "A downloaded model is verified…": "…one
  file read, one bar — across the scan, Remove and a fetch (D397)."
- **`docs/plan/README.md`**: §7, E4's and E7's open items from the two
  verifications (consent M-A, M-B, L-1–L-3; Compare M1, L1–L4) → done on
  `fix/consent-compare-followups`; §4, rows D390–D397 as above.
