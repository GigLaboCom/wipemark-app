# E4-8 — the prompt bench learns to see the author's voice; keep-voice in three languages: report

- **Task:** `docs/plan/E4-8-bench-voice.md` (Watchword FILE
  `wipemark-task-bench-voice-2026-10-08`, the coordinator, 2026-10-08): the
  bench measures voice, the keep-voice variant exists in en/ru/de for every
  tactic it should touch, the four-model run is one script for the owner's
  host, the bench example is linted by CI. The decision to ship keep-voice
  is **not** taken here: the shipped templates
  (`crates/wipemark-pipeline/prompts/<lang>/`) and `select::RULES` are
  untouched.
- **Branch:** `e4/bench-voice`, from the task commit `60e1b12` with
  `feat/e0-e6-shell` merged in first (`8be16bb`, `feat` at `d2d7784`: E7-9,
  E8-1 and their follow-ups) — no conflict. `feat` had not moved again when
  the final gates ran.
- **Commits:** `9b43f7d` (V1–V3: the measures, the report's voice table,
  the judge's voice question, `bench plan`, the variant reader, keep-voice
  in en/ru/de, `run-voice.sh`, their tests and the register lists);
  `00d1aef` (V4: CI); `3f63a56` (V5: `prompt-bench.md`, and the end-to-end
  smoke script); `38f0dfc` (the red checks); then the gates script, a
  comment in `run-voice.sh`'s header (no `GGML_*` variable is read with the
  prebuilt release) and this report.
- **Built in a Docker container** (Debian 12, aarch64): no GPU, no window,
  no model file — none was downloaded. `run-voice.sh` was run with
  `--dry-run` only; `bench plan` ran over the `local-llama` shim (it loads
  nothing). The four-model run is the owner's ("The run on the host").

## The requirements

| | what | done | commit | tests | removed locally → red |
|---|---|---|---|---|---|
| V1 | voice measures: second and first person per language with the formal address apart and the ты↔вы / du↔Sie switch its own figure; words ×; the register proxy, its lists as data; in `run`'s records, `report`'s tables (medians and quartiles per model × tactic × intensity, the chunks that lost any second person) and a separate judge question; old records still report | yes | `9b43f7d` | `measure.rs`: `english_counts_you_and_i_but_not_the_country_or_i_e`, `russian_counts_ty_and_vy_and_a_switch_between_them`, `german_sie_inside_a_sentence_is_formal_and_at_its_start_needs_the_pair_to_say_so`, `an_empty_text_and_a_text_with_no_second_person_say_nothing_about_it`, `the_words_ratio_leaves_placeholders_out`, `the_register_proxy_counts_new_formal_words_only`, `every_register_list_reads_and_holds_the_words_it_is_for`, `the_record_carries_the_counts_and_the_ratios`; `analyse.rs`: `records_made_before_the_voice_measures_still_report_them`, `the_judges_voice_answer_is_read_beside_the_meaning`, `a_chunk_kept_as_it_was_keeps_its_voice`, `a_cell_with_two_candidates_shows_the_cpu_pick`, `a_rewrite_that_moves_from_du_to_sie_is_counted_as_switched`; `judge.rs`: `each_question_takes_only_its_own_answers`, `every_judged_attempt_is_asked_both_questions_and_an_old_file_only_the_new_one`, `the_meaning_question_is_the_one_e4_5_asked`; `run.rs`: `plan_counts_every_step_of_every_attempt_and_loads_nothing`; the smoke (below) for `run`'s record | the sentence-start rule for «Sie»; "ihr" kept out; "I" as written and "US" out; the switch; kept capped at all of it; placeholders out of words ×; an empty source's ratio; the register's stem; a list entry; the recompute for old records; a kept chunk's voice; the CPU 1 × 2 fallback; the judge's voice field; the meaning prompt's words; a voice answer of "CHANGED"; the voice line's own key; plan loading nothing; the record's `voice` (smoke) — each red |
| V2 | keep-voice in en, ru, de for the slots it should touch, each a full copy of the shipped template plus the one rule, each admitted; a test walks the directory; keep-voice-light only if the grid does not cover it | yes — paraphrase and humanize, system turn (D424); no keep-voice-light (D425) | `9b43f7d` | `tests/bench_variants.rs`: `every_variant_is_admitted_as_an_edit_would_be` (every variant directory, with the bench's own reader), `keep_voice_is_the_shipped_contract_plus_one_rule_for_paraphrase_and_humanize_in_every_language`, `a_variant_the_product_would_refuse_is_refused_by_the_bench` | `admit` in the reader; strict directory names; `{PROTECTED}` out of a variant file; the German humanize rule out — each red |
| V3 | `bench/run-voice.sh`: four models, baseline and keep-voice, `--every 3`, every language, judge, report into `bench/results/voice-<date>/`; refuses without its variables; downloads nothing; calls × measured seconds; `--dry-run`; not run here | yes (D426) | `9b43f7d` | `tests/bench_run_voice.rs`: `the_dry_run_prints_every_part_in_order_and_writes_nothing` (every command, exactly, in order), `it_refuses_to_start_without_a_model_variable` (each variable, in all three modes), `outside_a_dry_run_a_model_file_that_is_not_there_is_a_refusal`, `a_part_needs_only_its_own_variables`, `it_downloads_nothing` | the refusal; a dry run that creates the output; `--offline`; the `+voice` parts' `--variant` — each red |
| V4 | CI lints the bench: `cargo clippy -p wipemark-pipeline --features local-llama --examples --locked -- -D warnings` | yes, and its unit tests run there too (D428) | `00d1aef` | the step itself; `cargo test -p wipemark-pipeline --features local-llama --examples --locked` beside it | a `clone` on a `Copy` value in `analyse.rs` → the step fails (clippy `clone_on_copy`) |
| V5 | `prompt-bench.md`: the voice measures, what the proxy is and is not, the judge's voice question, `run-voice.sh`, D111 to be re-read | yes | `3f63a56` | — | — |

The red checks are `E4-8-bench-voice-2026-10-08-red.py` beside this report
(each piece replaced, the named test, lint or smoke run, every file put back
byte for byte; `--check` verifies the pieces only): **27 of 27 red**, one
run, 64 s.

`E4-8-bench-voice-2026-10-08-smoke.py` holds the road between the pieces,
which no unit test can (an attempt needs an engine): a fake
OpenAI-compatible endpoint on 127.0.0.1 answers rewrites by rule and the
judge's two questions, and the script runs `bench plan`, `run` (shipped and
`--variant keep-voice`, en/ru/de), `plan --of judge`, `judge` twice and
`report` twice — the second time over the records with `voice` removed and
the judge's voice lines dropped. 22 checks, all pass: `run` makes what
`plan` counted; every answered record carries its `voice`; the keep-voice
rule reaches the model on every request of the variant, in each language,
and never on the shipped templates'; the judge writes a meaning line and a
voice line per attempt, the meaning lines in E4-5's exact shape, resumes
without asking again; the voice table has both runs and both picks, with the
judge's answers; and old records report the same voice measures, with no
judged voice. It found one gap before the commit: a cell with two
candidates (the voice run's `humanize`) has no GPU 2 × 2 row, so the voice
table had dropped `humanize` — now it shows the CPU 1 × 2 pick (D429).

## What was built

**The measures** (`examples/bench/measure.rs`, `Voice::of`; D420, D421).
Person pronouns per language — closed lists in code — counted in the chunk
and the answer: the second person with its formal subset (ru «вы»-forms; de
«Sie», «Ihnen», «Ihr…» **with a capital**), and the first person. German's
capital is read with its place: inside a sentence «Sie» is the formal
"you", at a sentence's start it is also "she" and "they", and there it
counts only when the chunk or the answer has an unambiguous formal form;
bare "ihr" is not counted (her/their), «euch/euer…» is the plural "you".
English "I" counts only as written ("i.e." does not) and "US" never. Kept is
min(answer, source) / source by count; a chunk *lost any* (fewer) or *lost
all*; *switched* is a ru/de source in one register only whose answer uses
the other. Words × leaves the placeholders out. The register proxy is a
count of the answer's words that a list in `bench/register/<lang>.txt`
matches (a word, `stem*`, or `-suffix` with three letters before it) and
whose first five letters no word of the chunk begins with, over the
answer's words — "is smart" → "possesses intelligence" scores two; the
chunk's own word in another case does not count.

**Where they show.** `run` writes a `voice` object on every answered record;
`report` recomputes it from the texts (D422), so E4-5's and the divergence
study's records report it too, and a list edit needs no rerun. The report's
new table — *Voice — the loop's pick (E4-7) beside the least diverged* — is
per model × tactic × intensity, `max ≥ 0.2` beside `min ≥ 0.2` on the same
candidates, GPU 2 × 2 where there are four candidates and CPU 1 × 2 where
there are two: chunks; chunks with a second person; second person kept by
count and per chunk (median [q1–q3]); lost any / lost all; switched (of the
ru/de chunks addressed one way); first person kept; words × (all · per
chunk); register shift (median [q1–q3] · mean); the judge's voice answers.
`summary.json` carries the same under `"voice"` in every policy row (every
language together; per language for the weighed policies on GPU 2 × 2) and
every attempts row.

**The judge's voice question** (D423) is a request and a line of its own —
key `voice|<judge>|<attempt>`, field `"voice"`: `YES`, `PARTLY` or `NO` —
calibrated on the chunk against itself (`calib-voice|…`, field
`"voice_calib"`). The meaning question's prompt, keys and lines are E4-5's
(a test pins the prompt's text), so every earlier judgement stands, and an
old judge file judged again gains only its voice lines.
`divergence-vs-upstream/analyse.py` reads the meaning lines only now, so it
still reads a judge file with voice lines in it.

**`bench plan --of run|judge`** prints `attempts=<n> calls=<n> done=<n>`
with no model loaded (D429): over the shim, the voice run's grid on this
corpus is **1 440 calls a part** (90 chunks × 16).

**The variant reader** (`examples/bench/variant.rs`, D427) holds every file
to `row::admit` with the bench's window — the rule an edit in Settings, the
CLI's `--prompts` and the MCP tool's `templates` pass (D330) — and refuses a
stray file or a directory that is not a language. The walk test borrows it
by `#[path]`. The previous reader called `validate` with no `based_on`;
nothing it admitted is refused now (every variant on file passes).

**keep-voice** (D424): `bench/variants/keep-voice/{en,ru,de}/
{paraphrase,humanize}.1.system.txt`, each the shipped system turn with one
rule after "do not add claims":

| | paraphrase | humanize |
|---|---|---|
| en | Keep the author's voice: speak to the reader the way the text does (if it says "you", so do you), keep its tone and register, and use words as plain as its own. Do not make it more formal and do not make it longer. *(2026-10-07's, unchanged)* | Keep the author's voice: speak to the reader the way the text does (if it says "you", so do you). Do not make it more formal and do not make it longer. |
| ru | Сохраняй голос автора: обращайся к читателю так же, как текст (если в нём «ты» — и ты пиши «ты», если «вы» — «вы»), сохраняй его тон и регистр, используй слова не сложнее его собственных. Не делай текст официальнее и не удлиняй его. *(the task's wording)* | Сохраняй голос автора: обращайся к читателю так же, как текст (если в нём «ты» — и ты пиши «ты», если «вы» — «вы»). Не делай текст официальнее и не удлиняй его. |
| de | Bewahre die Stimme des Autors: Sprich die Leser so an wie der Text – duzt er sie, dann duze auch du; siezt er sie, dann sieze auch du –, behalte seinen Tonfall und seine Stilebene bei und wähle keine gehobeneren Wörter als er. Mach den Text nicht förmlicher und nicht länger. | Bewahre die Stimme des Autors: Sprich die Leser so an wie der Text – duzt er sie, dann duze auch du; siezt er sie, dann sieze auch du. Mach den Text nicht förmlicher und nicht länger. |

Humanize's rule drops "keep its tone and register": humanize exists to move
a machine text off its inflated register, and that clause would undo it.
Not touched: `back_translate` (step 2 sees only the pivot translation, and a
pivot through English erases ты/вы and du/Sie — a rule the step cannot
follow; a register-only clause for translation would be a variant of its
own), `structural` (step 2 writes from an outline, which carries no voice),
`code` (English only; no reader addressed). The German is written with
duzen/siezen, the way a German writer says it, not translated from the
English.

**keep-voice-light** is not a variant (D425): intensity is a fragment the
grid sets and a variant carries slots only, so `--grid "paraphrase:light:4"
--variant …/keep-voice` is the research's "voice + light" row — and the
voice run's grid has "light" in it.

**`run-voice.sh`** (D426) — see "The run on the host".

## Decisions D420–D429

In `docs/architecture/prompt-bench.md`, "Decisions D420–D429"; summarised:

| | decision |
|---|---|
| **D420** | Person words are closed pronoun lists per language, in code; second person with its formal subset (ru «вы», de capitalised «Sie»-forms); a German sentence-initial Sie/Ihnen/Ihr… is formal only when the pair has an unambiguous one; bare "ihr" not counted; English "I" only as written, "US" never; kept = min(answer, source) / source by count; switched = a source in one register only, an answer with any form of the other. |
| **D421** | The register shift is a proxy from data lists (`bench/register/<lang>.txt`): the answer's matching words whose five-letter stem the chunk lacks, over the answer's words; compared between runs, never read alone. |
| **D422** | `report` recomputes the voice from the texts (as the preface and the trailer); `run` writes it on each record too; quartiles for the voice measures. |
| **D423** | The judge's voice question is a separate request and line (YES/PARTLY/NO), calibrated on the chunk against itself; the meaning question unchanged, pinned by a test. |
| **D424** | keep-voice: paraphrase and humanize, system turn, en/ru/de; humanize without "keep its register"; not back_translate, structural or code. |
| **D425** | No keep-voice-light: the grid's intensity reproduces it. |
| **D426** | `run-voice.sh`: four models × {shipped, keep-voice}, `--every 3`, every language, paraphrase l/m/s × 4 and humanize m/s × 2, two run names, the judge by variables, offline, refusing, estimating. |
| **D427** | A variant is read through `row::admit`, strictly, by one reader shared with the walk test. |
| **D428** | CI lints the bench and runs its unit tests over the shim. |
| **D429** | `bench plan` counts with no model; the voice table: the loop's pick beside `min ≥ 0.2`, GPU 2 × 2 where there are four candidates, CPU 1 × 2 where there are two. |

## Gates

Run once at the end, all `--locked`, on `38f0dfc`'s tree
(`E4-8-bench-voice-2026-10-08-gates.sh`; 19 min 54 s in the container). The
commit after it adds the gates script and this report and changes no code:
a comment in `run-voice.sh`'s header and two sentences of
`prompt-bench.md`.

| gate | command | result |
|---|---|---|
| fmt | `rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')` | exit 0 |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| clippy, the bench (V4) | `cargo clippy -p wipemark-pipeline --features local-llama --examples --locked -- -D warnings` | exit 0 |
| test | `cargo test --workspace --locked` | exit 0 — **1 787 passed, 0 failed, 7 ignored** |
| dependency direction | `scripts/check-dep-direction.sh` | exit 0 |
| GPUI pin | `scripts/check-gpui-pin.sh` | exit 0 |
| no default features | `cargo check --workspace --no-default-features --locked` | exit 0 |
| `local-llama` | `cargo check --workspace --features local-llama --locked` | exit 0 |
| test, the bench (the new CI step) | `cargo test -p wipemark-pipeline --features local-llama --examples --locked` | exit 0 — **20 passed, 0 failed, 0 ignored** |
| smoke | `python3 -I docs/plan/reports/E4-8-bench-voice-2026-10-08-smoke.py` | exit 0 — **22 PASS, 0 FAIL** |

The workspace count includes the eight new integration tests
(`tests/bench_variants.rs` 3, `tests/bench_run_voice.rs` 5); the bench's 20
unit tests are new but for E4-5's three. The bench's test code was also
linted once in test mode (`cargo clippy -p wipemark-pipeline --features
local-llama --example bench --profile test --locked -- -D warnings`):
clean. The native gates were not run: nothing under `crates/wipemark-llama*`
or `crates/wipemark-engine/src/local.rs` changed, and the prebuilt does not
link in this container (glibc 2.36).

## CI

CI_SECTION

## The run on the host

From the repository root on `e4/bench-voice` (or wherever it is merged).
The paths are the ones the divergence study used on 2026-10-07
(`docs/plan/reports/divergence-vs-upstream/run.sh`) where it named them;
put the real ones for the two that are not there:

```sh
export WIPEMARK_BENCH_GGUF_QWEN3_4B=/path/to/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf
export WIPEMARK_BENCH_GGUF_GEMMA3_12B=/path/to/gemma-3-12b-it-qat-UD-Q4_K_XL.gguf
export WIPEMARK_BENCH_GGUF_GEMMA4_12B=/mnt/data/mnemoria/models/gemma-4-12b-qat-ud-q4/gemma-4-12B-it-qat-UD-Q4_K_XL.gguf
export WIPEMARK_BENCH_GGUF_QWEN38_27B=/mnt/data/mnemoria/models/qwen38-27b-ud-iq3s/Qwen3.8-27B-UD-IQ3_S.gguf
export WIPEMARK_BENCH_GGUF_JUDGE=$WIPEMARK_BENCH_GGUF_GEMMA4_12B
export WIPEMARK_BENCH_JUDGE_NAME=gemma4-12b-judge
# WIPEMARK_BENCH_GPU_LAYERS_<MODEL> only if a model does not fit whole (default -1).
crates/wipemark-pipeline/bench/run-voice.sh --dry-run     # read the commands
crates/wipemark-pipeline/bench/run-voice.sh --estimate    # builds, plans, loads nothing
crates/wipemark-pipeline/bench/run-voice.sh 2>&1 | tee crates/wipemark-pipeline/bench/results/voice-run.log
```

It refuses (exit 2) on a variable not set or a file not there, and when the
pinned llama.cpp release is not already in `target/debug/llama-cpp-prebuilt`
(any earlier `llama-native` build put it there) or `WIPEMARK_LLAMA_PREBUILT`;
it downloads nothing. Stopped anywhere, the same command picks up where it
was; on another day add `--out crates/wipemark-pipeline/bench/results/voice-<the first day>`.

**How long.** 1 440 calls a part (counted here by `bench plan` over the
shim; `--estimate` counts it on the host). At E4-5's measured seconds per
call: Qwen3 4B 1.19 s → ~29 min a part, Gemma 3 12B 2.37 s → ~57 min,
Gemma 4 12B 1.61 s → ~39 min, Qwen3.8 27B 2.04 s → ~49 min; × 2 parts each
≈ **5 h 50 min**; the judge at most ~23 400 calls × 0.21 s ≈ **80 min**
(two questions an attempt); about **7 h** in all, plus each model's load.
The script prints the estimate before each part and the measured figure
after it (`timings.tsv`).

**What to read** — `crates/wipemark-pipeline/bench/results/voice-<date>/tables.md`:

1. **"### The judge's calibration"** first: the `same (voice)` row should be
   at or near 100 %. If the judge calls a text's own copy anything but
   `YES`, its voice column is not to be trusted, and the deterministic
   columns carry the decision alone.
2. **"### Voice — the loop's pick (E4-7) beside the least diverged"**: for
   each model, its row beside its `+voice` row at `paraphrase` `moderate`,
   `max ≥ 0.2 (E4-7)` — the setting the product ships. keep-voice should
   raise *2nd person kept* and lower *lost any / lost all* and *switched*
   (the study: 26 → 35 of 41 on one article), bring *words ×* toward 1.00
   (study: ×1.15 → ×1.12) and lower the *register shift*; the judge's
   `YES` share should rise. Then `light` and `strong`, and `humanize`
   (CPU 1 × 2).
3. **What it costs**, in "### Selection policies, GPU 2 × 2 (D61) — every
   language" for the same rows: *pairs carried over, by words* (the study:
   no change, 23 %), *rewritten* (must not fall), *judged CHANGED* (must not
   rise), and in "### What rejected a candidate" the no-op and length
   rejections (the study: 17 → 14 guard rejections).
4. **D111 re-read**: within each run, the `max ≥ 0.2` row against the
   `min ≥ 0.2` row of the voice table — what the most-diverged pick costs in
   voice, on the shipped templates and under keep-voice. The study found −2
   "you" and +2 % words for −8 points of pairs left; four models and three
   languages say whether that holds.
5. **Per language** (`summary.json`, `"policies"`, rows with `"lang"` `en`,
   `ru`, `de` for E4-7's and `min ≥ 0.2`'s picks): the ru and de wording is
   new and unmeasured; check it is not worse than the en.

A reading that would say *ship keep-voice*: on every model at `paraphrase`
`moderate` E4-7, second person kept up and switched down, words × and the
register shift not up, pairs carried over within a point or two, rewritten
not down, judged CHANGED not up. That decision, and whether D111 stands,
are the owner's.

## Wanted edits

**`CLAUDE.md`**

- *Where things are*, the `wipemark-pipeline` row, "today": after "the prompt
  bench (`examples/bench`, `bench/`, E4-5 — `docs/architecture/prompt-bench.md`)
  and its recommendations built (E4-7)" add: "; its voice measures (second
  and first person, the ты↔вы / du↔Sie switch, words ×, a register proxy),
  the judge's voice question, `bench plan`, keep-voice in en/ru/de and the
  four-model run `bench/run-voice.sh` (E4-8, D420–D429) — the run is the
  owner's, and keep-voice is not shipped".
- *Gates*, the CI-only lines: add
  `cargo clippy -p wipemark-pipeline --features local-llama --examples --locked -- -D warnings`
  and `cargo test -p wipemark-pipeline --features local-llama --examples --locked`
  ("the prompt bench, which only `local-llama` builds", D428).
- *The prompts are data*: add "A bench variant (`bench/variants/<name>/`) is
  held to `row::admit` as an edit is and walked by `tests/bench_variants.rs`
  (D427); keep-voice (paraphrase and humanize, en/ru/de) waits for the
  four-model run before it is shipped (E4-8)."
- *Specs* table: `wipemark-task-bench-voice-2026-10-08` (FILE, the task) and
  `wipemark-bench-voice-report-2026-10-08` (FILE, this report).
- *Epic order*: "E4-8 (the bench's voice) is done in the container on
  `e4/bench-voice`; its four-model run is the owner's."

**`docs/plan/README.md`**

- §7 E4, the bullet "Measured the same day against upstream": replace
  "**Open**: … the four-model run, the rule in en/ru/de and the voice
  measure in `bench report` are not done." with "**E4-8** (D420–D429,
  `reports/E4-8-bench-voice-2026-10-08.md`): the voice measure is in `bench
  report` (second and first person, the switch, words ×, a register proxy,
  the judge's voice question); the rule exists in en/ru/de as
  `bench/variants/keep-voice` (paraphrase, humanize); the four-model run is
  `bench/run-voice.sh`, ~7 h on the host. **Open**: that run, then whether
  keep-voice ships and whether D111 stands, read against the voice table."
- §7 E4, the bullet "**Fix. No gate builds the bench example**": close it —
  "`gate.yml` lints the examples with `local-llama` and runs their tests
  (E4-8, D428)."
- §4: rows D420–D429 (the table above, from `prompt-bench.md`).
- §2.1: the row for `e4/bench-voice` — done in the container; the host run
  and its decision pending.

## Left open

- **The four-model run** and what it decides (ship keep-voice; D111).
- **German and Russian verbs carry the address** the counts do not see
  («Нажмите», "Klicken Sie" has its pronoun, an English imperative has
  none); the judge's question is the check on that. A count of imperatives
  would need a tagger.
- **The register lists are a first cut.** They are data, so the owner can
  edit them after reading the host's tables; `report` applies an edit to
  every run on file.
- **The judge is one of the four models** (Gemma 4 12B, as in the study;
  E4-5 used Gemma 3 12B, also one of them), so it judges its own answers —
  a proxy, as before.
- **`humanize` at two candidates** is read on its CPU 1 × 2 pick; a GPU
  2 × 2 view of it would need `humanize:…:4` in the grid (+360 calls a part).
