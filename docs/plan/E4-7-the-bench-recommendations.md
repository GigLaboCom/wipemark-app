# E4-7 — The bench's recommendations, built: selection, the language check, list items, numbers

|                  |                                                                                                                                       |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic E4, the pipeline (README §7 E4)                                                                                    |
| Decision         | D95 (E4-5's recommendations, accepted by the owner 2026-10-04); revises D70, D71, D78, D87                                            |
| Depends on       | E4-3 (the loop), E4-4 (the resumable job and its fingerprint), E4-5 (the bench, its corpus and its records), E4-6a (the surfaces that start a job) |
| Unblocks         | E4-6b (the windows' half): the numbers it will show are these                                                                         |
| Files touched    | `crates/wipemark-pipeline/**` (`select.rs`, `job/`, `prepare/`, `report.rs`, `examples/bench/`, `bench/results/summary.json`), `crates/wipemark-core/src/guard.rs` (`NumbersGuard` only), the two app/CLI test fakes that relied on a two-word swap, `docs/architecture/{pipeline,prompts,prompt-bench,layer-a}.md`, this document, the report |
| Size             | ~1 day for one agent; no window; one live re-measurement on Qwen3 4B (GPU, Vulkan)                                                    |

## §0 Ground rules

### 0.1 Start here

Worktree `/home/denis/denis-ubuntu/sources/wipemark-e47`, branch
`e4/d95-strength`, from `feat/e0-e6-shell` at `afe94cd`. Read this, then
`CLAUDE.md` in full — it wins where the two disagree.

```sh
export GIT_CONFIG_NOSYSTEM=1
S=/tmp/claude-1000/-home-denis-denis-ubuntu-sources-wipemark-app/3f38a71d-77bc-40e9-94ee-ec30399f699c/scratchpad
export CARGO_TARGET_DIR=$S/target-e47 LIBRARY_PATH=$S/lib
```

Commit on the branch; never push, never touch `main`, never edit
`CLAUDE.md` or `docs/plan/README.md` (the report lists the edits).

### 0.2 Where code goes

```
core ← engine ← pipeline ← app / cli          (scripts/check-dep-direction.sh)
```

The pipeline crate holds everything except one function of core
(`guard.rs::numbers`). Core keeps zero dependencies. No new dependency.

### 0.3 Rules that bind this step

- **A document goes back byte for byte.** `assemble(&[None; n]) == source`
  over the generated documents and every Markdown file of the repository
  stays green; so does the echo (every chunk answered with itself).
- **A failed candidate is never used**, and every rejection is a
  structured value; `Rejection` stays exhaustive (D85).
- **The queue's fingerprint**: a resumed job never mixes chunks decided
  under the old rules with chunks decided under the new.
- **The third shelf is never empty**; **no epic number leaves this
  repository**; **every string a person reads comes from the catalogue**
  — no surface renders a single rejection today (the CLI and MCP say
  totals; the report's JSON is a format), so the new rejection needs an
  id, not a sentence.
- **Tests must be able to fail**: every protection gets a mutation that
  turns a test red, recorded in the report.

### 0.4 Gates

`$S/wm-gates.sh <worktree>` (the four gates and the CI lanes), plus
`cargo clippy -p wipemark-pipeline --examples --features local-llama
--locked -- -D warnings` (the bench, which the workspace clippy skips),
plus the bench's own tests under `llama-native`.

## §1 Goal

Make the loop do what the bench measured would be better: hand the user
the **most-changed** rewrite that kept everything, stop counting a
near-copy as a rewrite, judge a short line by a short line's length rule,
refuse a paragraph that came back in another language, rewrite a list
item by item so its markers never reach a model, and stop reading a
placeholder as a number. Then measure again on the same corpus and model
and say what moved.

## §2 Read first

`CLAUDE.md`; `docs/plan/README.md` §4 D61, D64, D70, D71, D78, D83–D87,
D89–D90, D94–D96 and §7 E4; `docs/plan/reports/E4-5-2026-10-04.md`;
`docs/architecture/{pipeline,prompts,prompt-bench}.md`;
`crates/wipemark-pipeline/src/{select.rs,job/,prepare/,report.rs}` and
`examples/bench/`.

## §3 What is true today (at `afe94cd`)

- `select.rs:25` `NO_OP_FLOOR = 0.05`; `:29` `LENGTH_WINDOW = 0.5..=2.0`;
  `:33` `LENGTH_PENALTY = 0.15`, added to the score; `:86` `winner` takes
  the **lowest** score (D71, `min-divergence`).
- `job/attempt.rs:171` runs the guards built once per job
  (`job/mod.rs:130`, the length guard from `Options::length`,
  `job/mod.rs:82`, one window for every chunk), then `:183` restore, then
  `:187` the floor. Nothing looks at the answer's language; `ScriptGuard`
  sees Latin on both sides of an English→French translation.
- `prepare/markdown.rs:50–58` groups every piece of one outermost list
  (between kept blocks) into one unit; `prepare/chunk.rs:264` writes the
  bytes between two items as a **glue placeholder** that starts a line of
  the text the model sees; `prepare/mod.rs:162` `Layout::glue`,
  `:208` `RestoreError::OutOfOrder`, `:215` `ItemBroken { item }`,
  `:421` `check_items`.
- `wipemark-core/src/guard.rs:395` `numbers()` reads every digit run,
  including the one inside `⟦3⟧`.
- `job/resume.rs:195` the fingerprint is tagged `wipemark-resume/1` and
  hashes the options' `Debug`; the selection rule is a constant and is
  **not** in it. `job/stored.rs:22` `OPTIONS_VERSION = 1`;
  `report.rs:34` `REPORT_VERSION = 1`.
- `examples/bench/run.rs::attempt` re-implements the verdict step for
  step; `verify` proved it agrees with the loop (54/54).

## §4 Decisions (I1…; proposed D-numbers in §9)

| # | decision | why |
|---|---|---|
| **I1** | The winner is the **most** diverged candidate that passed; a tie goes to the earlier attempt. `Scores::score` is what the winner maximises — the divergence, for the one scorer there is. | D95(1). −5 to −10 points of carried word pairs at the same time and judged drift (E4-5). Revises D71. |
| **I2** | The no-op floor is **0.2**. | D95(1): at 0.05 a "light" near-copy with 97 % of its word pairs left passed; 0.2 cost no paragraph its rewrite at "moderate" on a GPU. |
| **I3** | The 0.15 length penalty and its 0.5–2.0 window are **removed**. | It could never fire behind the guard (E4-3 noted it); the guard is the length rule. |
| **I4** | Two length windows: a chunk of **20 words or more** keeps 0.6–1.6; a shorter one gets **0.5–2.0**. A word is a maximal run of letters and digits outside the placeholders (the divergence's own word, without `⟦n⟧`), counted on the chunk's text as the model saw it. `Options::length` becomes `LengthWindows { long, short }`; the boundary is a constant (`select::SHORT_CHUNK_WORDS`), not an option. | D95(1): in the band between the windows a long chunk was judged changed 65 % of the time, a short one 6 %. |
| **I5** | Stored options become version 2 (`length` is the long window, `length_short` the short). A **version 1 row is still read**, and its one window becomes **both** — what the row said, for every chunk; never the new default in its place. | D90: an item runs with the options it was pushed with. Nothing pushes to the queue yet, so no such row exists, but a refused row would fail the item. |
| **I6** | **The language check.** When the chunk's language is detected (`lang::detect` over its text, placeholders removed) and the answer has 20 words or more, an answer detected as **another language or as none** is `Rejection::Language { expected, found }`. A chunk whose language cannot be told is never checked; an answer under 20 words is never checked. | D95(2). The bench's 32 obeying answers that passed every guard were all read as *unknown* (French is a neighbour `detect` declines), not as another of ours: a check that let *unknown* through would have caught none of them. The cost: 0.3–0.4 % of the other passed answers. |
| **I7** | The check is on the **final** answer, against the chunk's language — the language every tactic's last step must produce (`paraphrase`, `humanize`, `structural` write "in the language of the text"; `back_translate` step 2 returns to the document's). Step 1 of `back_translate` (the pivot) is not checked: it never reaches the document, and a step 1 that did not translate makes a weaker rewrite, which the floor and the selection already judge. A chunk in another language than its document, back-translated into the document's, is refused — that is a translation, not a rewrite. | D60/D64: a step may intentionally produce another language; only the last step's answer becomes text. |
| **I8** | Order: Layer A → the guards → **language** → restore → floor. The first rejection is the one reported. | The guards are core's and cheaper; restore is the document's own check. |
| **I9** | **A list item is a chunk of its own** (Markdown; HTML already did this). Every piece is its own unit; the glue placeholders, `Layout::glue`, the per-segment prefixes and `RestoreError::OutOfOrder` go — nothing can produce them any more. A list marker is never inside a chunk (it is the source's bytes between chunks). | D95(3): the placeholders that start a list line are the ones lost (Gemma 3 kept every placeholder in 45–63 % of list attempts, against 100 % for a lone inline one), and lists are most of the unchanged paragraphs. Revises D70 and D78's list-glue line. |
| **I10** | `RestoreError::ItemBroken` (now without a field) means: a chunk that is a **list item** came back with more line breaks than it went in with. A paragraph outside a list is not checked, as before. | A new line inside an item is laid back under the marker's indentation: a `- ` the model put at its start becomes a nested list, a blank line makes the whole list loose. Revises D87. |
| **I11** | An item's context is the rule every chunk has: the last two sentences of the previous chunk's source — the previous item, or the lead-in paragraph for the first. No new context rule. | D70; the bench asked for "the previous item as context", which this already is. |
| **I12** | More chunks, more calls: the price (`Planned::cost`) counts them exactly as before, and the live run reports what it cost. Nothing else changes for it. | D61's price is what the user sees before a run. |
| **I13** | `NumbersGuard`: a canonical placeholder `⟦n⟧` (the guard's own definition) is not a number, on either side. | D95(4): a lost `⟦2⟧` was also a "missing 2"; a candidate's `⟦3⟧` could stand in for a lost "3". |
| **I14** | **"1800" for "1,800" stays a loss**, and so does "12000" for "12 000". | A separator's meaning depends on the language — "1,800" is 1.8 in German and Russian, "1.800" is 1.8 in English — and the guard cannot know the language (it is often unknown). A guard that equated them would let 1.8 become 1800. A false reject costs one candidate; a false pass costs the user a number. Since D94 the contract asks for the same separators. 17 of Qwen3 4B's 191 number rejections were separators only. |
| **I15** | The fingerprint's tag becomes `wipemark-resume/2` and it hashes `select::RULES` — the selection, the floor, the length windows' boundary and the language check's threshold as one value — so a later change of any of them forgets every record without anybody remembering to bump a tag. | CLAUDE.md: a changed job forgets them all. The options already carry the windows. |
| **I16** | `REPORT_VERSION` becomes 2: `passed.score` is now maximised, `restore`/`item-broken` has no `item`, `out-of-order` is gone; the new `language` rejection (`expected`, `found`) and a `selection` block (`"most-diverged"`, the floor) are additions. | The version's own rule: bumped when a field changes meaning or goes away. |
| **I17** | The verdict on an answer is **one public function** (`job::verdict`), which the loop and the bench both call; the bench keeps its per-guard extras beside it. | The bench's numbers stand for the product's only while the two agree; a copy is the one that drifts. |

## §5 Deliverables

1. `select.rs`: `NO_OP_FLOOR = 0.2`; `SHORT_CHUNK_WORDS = 20`;
   `LANGUAGE_CHECK_WORDS = 20`; `Selection::MostDiverged`; `Rules` and
   `RULES`; `LengthWindows`; `prose_words`; `winner` maximises; the penalty
   and its window gone.
2. `job/attempt.rs`: `verdict(options, chunk, chunk_lang, answer)` —
   guards (the window for the chunk's size), language, restore, floor.
   `job/mod.rs`: `Options::length: LengthWindows`, `Options::guards_for`.
3. `report.rs`: `Rejection::Language { expected: Lang, found:
   Option<Lang> }` (`kind` `language`), `REPORT_VERSION = 2`, the
   `selection` block, `item-broken` without `item`.
4. `prepare/`: one piece per unit, the glue gone, `Piece::item`,
   `Layout::item`, `ItemBroken` per item chunk.
5. Core: `numbers()` skips placeholders.
6. `job/stored.rs` v2 (reads v1); `job/resume.rs` tag 2 + `RULES`.
7. The bench: `run::attempt` over `job::verdict`; `analyse` labels the
   policies by step (`min ≥ 0.05 (E4-3)`, `max ≥ 0.2 (E4-7, the loop)`)
   and counts language rejections; `summary.json` re-generated with the
   re-measurement beside E4-5's records.
8. Docs: `pipeline.md` (preparing, the loop, selection), `prompts.md`
   (lists no longer carry glue), `prompt-bench.md` (the re-measurement),
   `layer-a.md` (`NumbersGuard`).

## §6 Tests (RED first; a mutation per protection)

| test | protects | mutation |
|---|---|---|
| `the_most_changed_passed_candidate_wins` (job) + `the_highest_score_wins_and_a_tie_goes_to_the_earlier_attempt` (unit) | I1 | `winner` takes the lowest |
| `a_near_copy_under_the_floor_is_a_no_op` (job: a two-word swap, divergence ≈ 0.15) | I2 | floor back to 0.05 |
| `a_short_chunk_is_judged_by_the_wide_window_and_a_long_one_by_the_narrow` (job) + `a_chunk_of_twenty_words_is_long` (unit) | I4 | one window for all; `<=` for `<` |
| `stored_options_of_version_one_run_with_their_one_window_for_every_chunk` | I5 | read v1's window as long only |
| `an_answer_in_another_language_is_rejected` (planted French) | I6 | drop the check |
| `an_answer_whose_language_cannot_be_told_is_rejected_when_the_chunks_can` | I6, *unknown* | `found.is_some_and(≠)` |
| `a_chunk_whose_language_cannot_be_told_is_never_checked` | I6 | check against the document's language |
| `a_short_answer_is_never_language_checked` | I6 | no word threshold |
| `back_translate_is_judged_on_its_final_answer` (step 1 in the pivot, step 2 back) | I7 | check every step |
| `each_list_item_is_its_own_chunk_and_its_marker_is_never_shown` (prepare) + `a_list_is_asked_item_by_item` (job) | I9 | group the list again |
| `a_list_item_that_comes_back_on_more_lines_is_rejected_by_restore` (job + prepare) | I10 | `check_item` always `Ok` |
| `an_items_context_is_the_item_before_it` | I11 | — (context rule unchanged; the test pins it) |
| the two property tests and `every_markdown_document_in_this_repository_reassembles_exactly` | byte for byte | — |
| `a_placeholders_digits_are_not_a_number` (core) | I13 | skip nothing |
| `a_number_with_its_separators_changed_is_lost` (core) | I14 | strip separators |
| `the_fingerprint_moves_with_the_rules` (resume) | I15 | leave `RULES` out of the hash |
| `the_report_names_its_selection_and_a_language_rejection` | I16 | — |
| bench `verify` live: the loop and the bench agree attempt by attempt | I17 | — |

## §7 Acceptance

1. All gates green; every mutation of §6 recorded red.
2. The live re-measurement on Qwen3 4B (`paraphrase` moderate and the
   planted items at least), before/after beside E4-5's numbers: carried
   word pairs, pass rate, no-op rate, instruction-obeying answers that
   pass (→ ~0), placeholder survival in lists, calls per document, judged
   drift if the judge is cheap; `verify` agreeing.
3. The report `docs/plan/reports/E4-7-2026-10-04.md` with the edits for
   `CLAUDE.md` and `docs/plan/README.md`.

## §8 Out of scope

A stylometric score (D63); the keyed scorer (D72); new templates; an
intermediate-language check for `back_translate`'s pivot (I7); E2
follow-ups of D96; the windows (E4-6b); skipping very short items (an
open question in the report).

## §9 Basis — proposed D-numbers

D111 = I1–I3 (selection, floor, penalty; revises D71); D112 = I4–I5 (two
length windows, stored options v2); D113 = I6–I8 (the language check);
D114 = I9–I11 (a list item per chunk; revises D70, D78, D87); D115 =
I13–I14 (`NumbersGuard`: placeholders, separators); D116 = I15–I16 (the
fingerprint's rules, report v2); D117 = I17 (one verdict function for the
loop and the bench). I12 is a consequence, not a decision.
