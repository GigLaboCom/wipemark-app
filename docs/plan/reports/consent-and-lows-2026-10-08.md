# Consent and the Lows — the host verifier's findings on the 2026-10-08 fix round: report

- **Task:** the coordinator, for the owner, 2026-10-08: fix the host
  verification's Medium M-1 (D361's consent compared with the window's
  record, not the engine an item gets) and Lows L-a, L-b, L-c, L-d, L-g,
  L-h; L-e and L-f noted only, no change.
- **Branch:** `fix/consent-and-lows`, from `origin/feat/e0-e6-shell` at
  `9b907bb` (the merge of integrate/2026-10-08). Pushed to `origin`; no pull
  request; `main` and `feat` untouched; `CLAUDE.md` and
  `docs/plan/README.md` untouched (another agent is updating them).
- **Decisions:** D370–D375, in the architecture documents they belong to
  (`queue.md`, `pipeline.md`, `prompts.md`, `model-downloads.md`, `cli.md`)
  and below.
- **Commits** name human authors only.

## Findings

Each fix's test was seen red once with the fix reverted;
`docs/plan/reports/consent-and-lows-2026-10-08-red.py` holds every revert
and re-runs it on request (`RED`/`GREEN`/`BROKEN` per check). It is a
record, not a table to run every round.

| | fix | commit | test | reverted to see red |
|---|---|---|---|---|
| M-1 — consent compared with `Going`, not the engine handed out | `EngineSource::for_item` returns `Handed { engine, whereto }`: the engine and where **that engine** sends a document, read together from the engine handle's slot, which the host fills with the engine and the performer's destination (`Performer::whereto`, the window's rule too). The worker checks consent after `for_item`, before anything is read; on a question it drops the engine first (busy back, so a deferred swap can land) and holds and asks. `journal::Duty` is gone — the queue runs on `EngineHandle`; `Going` only words the window and wakes the queue (D370) | `1947463` | `wipemark-queue` `duty::the_consent_is_checked_against_the_engine_handed_out` (a slot holding endpoint Y's engine, an item consented "here": asked, Y never called, the engine let go of before the question, the item run here once the swap lands); `engine_host::tests::a_consent_is_checked_against_the_engine_the_queue_is_handed` (the issue's scenario on a real `EngineHandle`: a job on Y running, `Going` says here; asked, busy back to 1, never Y); `engine_host::tests::the_slot_says_where_its_engine_sends_a_document` | `M-1-queue`: the check given the window's record ("here") — red; `M-1-handle`: the handle says no destination (`whereto: None`) — red |
| L-a — a Cancel or Remove between reserve and push lost | `Queue::reserve` tells the queue's thread (`Command::Reserve`); a Cancel or Remove of a reserved id that arrives before its Push is kept, and the Push then ends the item as cancelled, or drops it and says `Removed` (D371) | `1947463` | `duty::a_cancel_or_a_remove_before_the_push_is_kept_for_it`; `queue::rewrite_tests::a_row_removed_before_its_push_is_never_rewritten` (gpui, the journal writer held: the row removed between reserve and push; no `a.rewritten.md`, nothing in the queue) | `L-a-queue`, `L-a-window`: the reserve not sent — both red (the window's: "a removed row's document was rewritten") |
| L-b — `agreed` survived a duty change | a yes covers the items the question was for — those waiting then, consented elsewhere (`Agreed { now, items }`); an item pushed later is asked about on its own (D372). Scoping rather than clearing on Retry: a slot swap to the same endpoint (another model) would otherwise ask again what was answered | `1947463` | `duty::a_yes_covers_the_items_it_was_asked_for_and_no_later_one` (yes to Y for B; duty here; C pushed "here"; duty back to Y: C asked) | `L-b`: the item scope or'd away — red |
| L-d — an agent's/CLI's call behind a question had no ceiling | `Rewriter::wait` cancels its item and answers `Unrun::Asking` when the queue raises `Ask` before the item started, as for a hold or a pause; a call made while a question stands is refused before anything is queued. The sentence: "the application's rewrites wait for an answer in its window about where a waiting document may be sent; answer it there and call again" (D373) | `a2328d5` | `mcp::protocol::tests::a_call_behind_a_question_is_refused_rather_than_left_waiting` (a window item running, one consented "here" behind it with Y on duty, the agent's call behind both: refused when the question is put; a second call refused at once) | `L-d-event`: the `Ask` arm disabled — red (timeout); `L-d-standing`: the check before the push disabled — red (timeout) |
| L-g — a doc comment separated from its test | `every_kind_has_a_glyph_and_a_badge`'s comment moved back above it | `fa517cb` | — (a comment) | — |
| L-h — a pre-D369 template refused only at render | the window's Rewrite checks the templates the job will use as it builds the item: the plan's own fallbacks when the document plans, else every saved template of a tactic on the ladder, any language, by `row::admit`; one that breaks a rule makes the row *Rewrite failed* with `template <row key>: <rule>`, nothing pushed (D374) | `32903fb` | `queue::rewrite_tests::a_saved_template_the_rules_refuse_refuses_the_push_by_name` (a row written as a pre-D369 build wrote it, with U+200B); `…without_a_plan_every_template_on_the_ladder_is_asked` | `L-h-planned`: the check skipped — red (the row was rewritten); `L-h-unplanned`: `admit` ignored — red |
| L-c — marks on dev+ino unstable on btrfs/NFS/FUSE/vfat/exFAT | a mismatch never drops a mark (only a file found absent does); on Unix an identity differing **only** in device and inode is ours for a finished file when size and mtime_ns are the marked ones and its sha256, read in full, is the catalogue's (then marked again), and for a `.part` when its size and mtime are where its last download stopped (a `last` line every stopped download writes) and its birth time does not disagree; a `.part` is created and marked before the request, and a mark that cannot be written refuses the download with no `.part` left; a `.part` not known for ours is said as "a partial file Wipemark has no record of" on the card and in `models list`, `pull`, `rm` (D375) | `fe6c027` | `store::tests::a_renumbered_download_is_still_ours_and_a_mismatch_keeps_the_mark`, `…a_renumbered_part_is_resumed_where_it_stopped`, `…a_stopped_download_stamps_its_part`, `…a_download_that_cannot_mark_its_part_is_refused_up_front` (identities through a test seam that adds a remount to every device and inode number); `…a_mark_names_the_file_and_not_the_place` (now: the mark kept, still not ours); `models::tests::another_tools_file_in_the_way_offers_nothing` (the `.part` sentence); `wipemark-cli` `models::tests::an_occupied_place_is_not_promised_a_resume`, `cli.rs` `models_rm_leaves_another_tools_file_at_the_entrys_place`; `manifest::tests::no_shipped_file_is_named_like_a_partial_download` | `L-c-keep-mark`: the mark dropped on a mismatch — red; `L-c-whole`, `L-c-part`: the renumbered rule off — red; `L-c-full-read`: the sha256 taken off the record — red (another file under the marked size and mtime taken for ours); `L-c-stamp`: no stamp — red; `L-c-unmarked`: the mark's failure ignored — red |
| L-e, L-f | noted only, no change | — | — | — |

## Decisions

- **D370 — a consent is checked against the engine handed out.**
  `EngineSource::for_item` returns the engine with where it sends a
  document, read with it from one slot; the check runs after the engine is
  taken, and on a question the engine is let go of before the queue asks.
  `Going` words the window only. *Why:* `Going` moves when a preference
  does, the slot when the host's swap runs, and the host defers a swap
  while busy — so a check against `Going` let an item consented "here"
  start on the endpoint a deferred swap had left in the slot, and its own
  busy count kept the swap deferred for every item after it (M-1). The
  question that follows such a race is put and withdrawn as soon as the
  swap lands (the window's dialog may show it for that moment; a yes to a
  withdrawn question is not taken).
- **D371 — a reserved id is the queue's.** `reserve` tells the queue's
  thread; a Cancel or Remove before the Push is kept for it. *Why:* the
  window sets the id on its row at reserve and pushes later on the journal
  writer's thread, so a Remove in between reached the queue first, found
  nothing, and the item then ran and wrote a result for a row that was gone
  (L-a).
- **D372 — a yes covers the items it was asked for.** *Why:* a global
  `agreed` sent an item pushed later, consented "here", to an endpoint on
  an earlier item's answer (L-b). Scoped, not cleared on every engine
  change, because a swap to another model on the same endpoint is not a new
  question.
- **D373 — a caller's rewrite behind a consent question is refused.** *Why:*
  the ceiling counts from the start, which a question nobody answers never
  gives; a call must not wait on a person who may not be there (L-d).
- **D374 — a saved template the rules refuse refuses the window's push,
  by name.** *Why:* a template saved before D369 holding an invisible
  character was found only as the job rendered (it then fell back to the
  shipped template); the command line and an agent's call already refuse a
  template they are handed by name (L-h).
- **D375 — a file system that renumbers is not another tool.** The rule
  above, chosen for a finished file over "never": the file with the marked
  size and nanosecond mtime and the catalogue's sha256 is the catalogue's
  file either way, and the one thing the rule can get wrong — another
  tool's byte-identical copy with its mtime preserved to the nanosecond —
  costs at most a Remove of a file the catalogue can fetch again. The
  sha256 is read in full because the verify record is keyed by size and
  mtime, which such a file would share. A `.part` has no sha256 to confirm
  it, so it is known by where its download stopped; one a crash stopped has
  no such line and, after a remount, reads as a partial file with no
  record — said so, truthfully, rather than "another tool's" (L-c).

## What changed for a person

New sentences in en, ru and de: `settings-models-foreign-part`,
`cli-models-foreign-part-at`, `cli-models-pull-occupied-part`,
`cli-models-rm-found-part`. A window row whose saved template breaks a rule
now says *Rewrite failed (template …: …)* at once. An agent's or the
command line's rewrite behind a consent question in the window is refused
with a sentence. No window was opened (no live check).

## Gates

Run once, at the end, on this host (Linux, `LIBRARY_PATH` to a
`libxkbcommon-x11.so` symlink inside the worktree), all `--locked`:

| check | result |
|---|---|
| `git submodule sync --recursive && git submodule update --init --recursive` | clean (`vendor/gpui-component` at `f8429177`) |
| `rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')` | clean |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean (after one fix: `Downloads::identity`'s `self` is unused outside tests — allowed there with its reason; amended into `fe6c027`) |
| `cargo test --workspace --locked` | 1637 passed, 0 failed, 7 ignored (73 test binaries) |
| `scripts/check-dep-direction.sh` | dependency direction ok |
| `scripts/check-gpui-pin.sh` | gpui pin ok; x11 fixes ok |
| `cargo check --workspace --no-default-features --locked` | ok |
| `cargo check --workspace --features local-llama --locked` | ok |
| `cargo test -p wipemark-engine --features local-llama --locked` | 39 passed, 0 failed, 1 ignored |
| `cargo test -p wipemark-app --features local-llama --locked` | 634 passed, 0 failed, 2 ignored |
| `scripts/verify/owner-fixes/rm-through-a-link.sh` (`SCRATCH_ROOT` an absolute path in the worktree) | exit 0: link, dirlink, part, stale, stalelegacy, stalelink-target, stalelink-link KEPT; marked REMOVED |
| `scripts/verify/e4-6b/cli-journal.sh target/debug/wipemark-cli` | PASS schema-2 db unchanged; the exits and rows as its header says; PASS refused, exit 2, untouched |
| `docs/plan/reports/consent-and-lows-2026-10-08-red.py` | 15 of 15 RED |

A first run of `rm-through-a-link.sh` with a *relative* `SCRATCH_ROOT`
printed `marked KEPT` (exit 1): the script writes the scratch folder as the
`models.dir` row, and a relative `models.dir` reads as the default
(CLAUDE.md, "The models folder is one row") — the run's arrangement, not a
finding; with the absolute path every case is as it should be.

## CI

GitHub Actions `gate` run 37768575597 on `9ad3e00` (the branch with every fix, the docs and this report before this section was filled): **gate** (fmt, clippy, test, deps, features) passed in 29m22s; **macos** (clippy, tests, llama-native prebuilt with Metal) passed in 22m47s; **native** (llama.cpp prebuilt + Vulkan, model-free) passed in 1m39s. The commit that fills this section changes this file only.
