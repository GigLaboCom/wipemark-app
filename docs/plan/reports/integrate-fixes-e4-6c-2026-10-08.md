# E4-6c — the host verifier's findings fixed: report

- **Task:** the coordinator, for the owner, 2026-10-08: fix the host
  verifier's findings on E4-6c (M1, L1–L6) in E4-6c's files only —
  `apps/wipemark-app/src/prompts.rs`, `crates/wipemark-pipeline/src/prompt/`,
  the catalogues' `prompts-*` keys, `docs/architecture/prompts.md`, the
  E4-6c report.
- **Branch:** `fix/integrate-e4-6c`, from `origin/integrate/2026-10-08` at
  `dfaff29`. Pushed to `origin`; no pull request; `main`, `feat` and
  `integrate/2026-10-08` untouched.
- **Decisions:** D365–D369, in `docs/architecture/prompts.md` ("Decisions
  D365–D369") and below.
- **Commits** name human authors only.

## Findings

Each fix's test was seen red once with the fix reverted;
`docs/plan/reports/integrate-fixes-e4-6c-red.py` holds every revert and
re-runs it on request (`RED`/`GREEN`/`BROKEN` per check — all twelve RED
when run). It is a record, not a table to run every round.

| | fix | commit | test | reverted to see red |
|---|---|---|---|---|
| M1 — an adaptation overwrote a template saved while it ran | `adapt_template` reads the slot's row at the press and again just before the write, under `ROW_WRITER` (the one writer of template rows in the process: Save, Reset, Keep mine take it too); a row that appeared or changed meanwhile, or that `adapt_blocked` refuses, is left and the answer shown, not stored (`Adapted::Kept { ChangedMeanwhile }`); the admission is asked again beside the other turn as it is then. Save is greyed on the slot being adapted, its reason under the buttons, and `PromptsPage::save` refuses (`Said::Waits`); an answer that lands never replaces unsaved edits in the field (D365) | `07350c1` | `prompts::tests::an_adaptation_never_writes_over_a_template_saved_while_it_ran` (the fake engine's answer saves a hand template into the slot first: the row stays the person's, `origin: hand`); `…save_waits_while_the_model_adapts_this_template` (gpui: Save on the slot being adapted writes nothing and says why; another slot's adaptation does not grey it) | `M1-recheck`: the second look replaced by "may write" — the first test red; `M1-save-waits`: the guard in `PromptsPage::save` removed — the second red |
| L1 — Save could store the shipped text | Save of the shipped text over a row writes nothing and says to Reset (`Saved::ShippedText`), with or without a claim; Save over a row this build cannot read writes nothing whatever the field holds (`Saved::Unreadable`) — only Reset replaces it (D366) | `07350c1` | `prompts::tests::save_never_stores_the_shipped_text_or_replaces_an_unreadable_row` (a hand row, then a newer build's row, byte for byte; Reset then Save works) | `L1-shipped-text`: the check limited to "no row" — red; `L1-unreadable`: an unreadable row treated as none — red |
| L2 — Adapt treated an unreadable row as replaceable | `adapt_blocked` (was `may_adapt_into`): an unreadable row is `Blocked::Unreadable` — the buttons greyed with "Reset to shipped first", and a press that got through asks the model nothing and writes nothing (D365) | `07350c1` | `prompts::tests::an_unreadable_row_blocks_an_adaptation` | `L2-unreadable`: `Unread` mapped to "may adapt" — red |
| L3 — a check or adaptation outlived its slot and the page | `Activity::Running` carries its run number and slot; `select` cancels a running check and adaptation (`let_go`), `impl Drop for PromptsPage` does the same when the page goes with the Settings window; an answer for a run let go of is dropped (`Activity::land`); `Done` is drawn only under its own slot; an adaptation cancelled after the model answered writes nothing (the cancel is read under the row writer, before the look) (D367) | `07350c1` | `prompts::tests::another_slot_or_a_closed_page_cancels_what_runs` (gpui: both tokens cancelled on a slot switch, a late answer not landed; both cancelled when a page is let go of), `…an_adaptation_cancelled_after_the_answer_writes_nothing` (an engine that answers, then the run is cancelled) | `L3-select`: the two `let_go` in `select` removed — red; `L3-drop`: `Drop` emptied — red; `L3-cancel-write`: the cancel check before the write removed — red |
| L4 — any Save silently cleared the source warning | Decided for the explicit acknowledgement: a save keeps the source hash an adaptation recorded (a review of a machine adaptation, an edit of a hand one with the same claim), as it keeps `based_on`; the stale-source warning has its own **Keep mine** (`keep_mine_source`), with the sentence "A save keeps this warning…" beside it; a new or changed claim still records the source's hash now (D368) | `07350c1` | `prompts::tests::a_save_keeps_the_sources_hash_and_keep_mine_moves_it` | `L4-hash`: the "same source keeps its hash" arm removed — red |
| L5 — docs called `lay_over_within` on MCP/CLI a wanted edit | `docs/architecture/prompts.md` ("The one rule", the D330 row) and the E4-6c report's first lines, its Г-table row, its "What is left" and its plan note now say `5a9e525` made it, and how each road gets its window | this report's commit | — (docs) | — |
| L6 — an adaptation could store an invisible character | `validate` R9 `invisible-character`: a character `wipemark_core::inspect` at its defaults would remove (zero-width, bidi control, tag, soft hyphen, a variation selector out of place, private use, default-ignorable) is an error in any template, once per code point at its first place — the page's Save, `lay_over` (CLI, MCP) and `render` all refuse it; the shipped templates pass (`every_language_has_a_complete_shipped_set` and the shipped-set tests green). `adapt_with` runs `wipemark_core::clean` at its defaults over the answer after `clean_response` and before the rule. The page's sentence spells the character `U+200B ZERO WIDTH SPACE`, never carrying it (D369) | `07350c1` | `prompt::validate::tests::an_invisible_character_is_an_error` (ZWSP, RLO, LRI, ZWJ, soft hyphen, a tag, a mid-text BOM refused; a no-break space, „quotes“, a dash, an emoji with a skin tone and ❤ + VS16 not); `prompt::trial::tests::layer_a_runs_over_an_adaptation_before_it_is_judged`; `prompts::tests::an_invisible_character_is_refused_and_spelled_not_carried` (Save and `lay_over` refuse with the same rule id; the sentence) | `L6-layer-a`: Layer A over the answer removed — red; `L6-rule`, `L6-rule-page`: the rule's loop emptied — both red |

## Decisions

- **D365 — what Adapt may replace, looked at twice.** D335's rule (an
  empty slot or an adaptation) is checked when the button is pressed and
  again just before the write, under one writer of template rows per
  process; a row that appeared or changed meanwhile is left and the answer
  shown, not stored; a row this build cannot read blocks Adapt; Save is
  greyed on the slot being adapted. *Why:* the button was checked only when
  drawn (M1); an unreadable row may be a newer build's (L2).
- **D366 — Save never stores the shipped text, and never replaces an
  unreadable row.** Over a row it writes nothing and points to Reset; only
  Reset replaces a row this build cannot read. *Why:* a stored copy of the
  shipped text stops following it, and the page says only changes are
  stored; D74 leaves a value this build cannot use in the row.
- **D367 — a check and an adaptation belong to their slot.** Another slot,
  or the page let go of, cancels them; a cancelled adaptation writes
  nothing, even after the answer; an answer for a run let go of is dropped.
  *Why:* a check held the engine after the window closed, an adaptation
  wrote unseen, a finished check was shown under another slot (L3).
- **D368 — a source that moved on is acknowledged by asking.** A save keeps
  the recorded source hash, as it keeps `based_on`; Keep mine beside the
  stale-source warning moves it. *Why:* the two drifts were acknowledged
  differently (L4); the explicit acknowledgement is the one D333 chose for
  the shipped text. D333's "a hand claim records the source's current hash"
  now applies to a new or changed claim.
- **D369 — no invisible character in a template.** `validate` refuses what
  Layer A removes at its defaults, in any template; `adapt_with` runs Layer A
  over the model's answer before it is judged. *Why:* a model's (or a
  paste's) zero-width character or bidi control was stored unseen and sent
  with every rewrite (L6). A row stored before this rule with such a
  character is now an error to `render` too, as any template the validator
  refuses: the job refuses it by name rather than send it.

## What changed for a person

New sentences in en, ru and de (`prompts-problem-invisible`,
`prompts-save-shipped-text`, `prompts-save-unreadable`,
`prompts-save-while-adapting`, `prompts-adapt-unreadable`,
`prompts-adapt-overtaken`, `prompts-adapt-not-stored`,
`prompts-stale-source-keep`, `prompts-kept-source`), and
`prompts-unread`/`prompts-unread-not-json` now say the row stays until
Reset deletes it. A second **Keep mine** appears under a stale-source
warning. No window was opened (no live check; the page's behaviour is held
by the gpui tests above).

## Gates

**Targeted only; full gates run once at integration** (the owner's change,
relayed by the coordinator on 2026-10-08: checks run once, after the three
fix branches are merged together). On this host (Linux, `LIBRARY_PATH` to a
`libxkbcommon-x11.so` symlink inside the worktree), `--locked`:

| check | result |
|---|---|
| `rustup run nightly rustfmt --edition 2021 --check` over the three touched `.rs` files | clean |
| `cargo clippy -p wipemark-pipeline -p wipemark-app -p wipemark-i18n --all-targets -- -D warnings` | clean |
| `cargo test -p wipemark-pipeline --lib prompt::` | 88 passed, 0 failed |
| `cargo test -p wipemark-app --bin wipemark prompts::` | 22 passed, 0 failed |
| `cargo test -p wipemark-app -- prompts:: config:: settings::` | 121 passed, 0 failed |
| `cargo test -p wipemark-i18n` | 37 passed + 2 doc tests, 0 failed (the catalogue gates: completeness, variables, no epic number) |
| `docs/plan/reports/integrate-fixes-e4-6c-red.py` | 12 of 12 RED |

Not run here, by the same change: the whole workspace suite, the
`local-llama` checks and tests, `check-dep-direction.sh`,
`check-gpui-pin.sh`, `--no-default-features`, and CI.
