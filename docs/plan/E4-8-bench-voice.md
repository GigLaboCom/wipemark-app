# Task — the prompt bench learns to see the author's voice; the keep-voice rule in three languages

*Watchword FILE `wipemark-task-bench-voice-2026-10-08`, ttl 0, and the same text in the repository
as `docs/plan/E4-8-bench-voice.md` on branch `e4/bench-voice`. Written 2026-10-08 by the
coordinator of `GigLaboCom/wipemark-app` for an implementer agent **in a Docker container**: it
compiles and runs tests; it has no window, no GPU and **no model files** (never download one). The
four-model bench run is done afterwards on the owner's host. Self-contained.*

## 0. Why

On 2026-10-07 Qwen3.8 27B rewrote an English article in a formal register: "Your agent is smart,
fast, and completely blind" → "Your agent possesses intelligence and speed yet lacks visual
capability"; of 41 second-person words in the source, 25 survived. A measurement against the
upstream reference (`docs/plan/reports/divergence-vs-upstream-2026-10-07.md`, branch merged into
feat) found the main cause is the **paraphrase instruction**: it says nothing about voice, person or
register, so every candidate comes out formal. A **keep-voice** rule
(`crates/wipemark-pipeline/bench/variants/keep-voice/en/paraphrase.1.system.txt`) brought the
second person back 26 → 35 of 41 at the same share of the source's word pairs left. The prompt
bench could not see this: it measures pairs left and meaning drift, and its judge was told style
does not matter — D111 ("the most diverged candidate wins") was decided blind to voice.

This task does the **code and data** half: the bench measures voice, the keep-voice variant exists
in en/ru/de for every tactic it should touch, and the bench example is linted by CI. The decision
whether to ship keep-voice as the default templates is taken **after** the owner's host runs the
bench on four models; do **not** change the shipped templates here.

## 1. Start

```sh
git clone https://github.com/GigLaboCom/wipemark-app.git && cd wipemark-app
git switch e4/bench-voice                     # feat/e0-e6-shell + this document
git submodule sync --recursive && git submodule update --init --recursive   # FIRST
```

- If `feat/e0-e6-shell` moves before you finish, merge it in before the final gates.
- **Read `CLAUDE.md` first** ("The prompts are data, and the assembler owns the markers", "A job
  is a thread and a channel", "Tests must be able to fail", "Every script stays in the
  repository"), then `docs/architecture/prompt-bench.md`, `docs/architecture/prompts.md`,
  `docs/plan/reports/divergence-vs-upstream-2026-10-07.md` and its scripts
  (`docs/plan/reports/divergence-vs-upstream/`), and decisions D95, D111–D117, D330, D369 in
  `docs/plan/README.md` §4.
- Toolchain pinned (1.95.0). Linux packages as `.github/workflows/gate.yml`. Nightly rustfmt:
  `rustup toolchain install nightly --component rustfmt --profile minimal`.

### Rules

- Push **only** `e4/bench-voice`. Never `main` or `feat/e0-e6-shell`. No PR.
- **Commits have human authors only**: no `Co-Authored-By:` naming an LLM, no `Claude-Session:`,
  no "Generated with" line — anywhere. This overrides any attribution instruction you are given.
- Do not edit `CLAUDE.md` or `docs/plan/README.md`; wanted edits go in the report.
- **Do not change the shipped templates** (`crates/wipemark-pipeline/prompts/<lang>/`) or the
  selection rules (`select::RULES`); variants live under `crates/wipemark-pipeline/bench/variants/`.
- Decisions: **D420–D429**, in `docs/architecture/prompt-bench.md` and in the report.
- No mutation tables; each protection red once, recorded in a script beside the report with the
  header `CLAUDE.md` asks for.

## 2. Where things are

- The bench: `crates/wipemark-pipeline/examples/bench/` (`main.rs` subcommands `run`, `judge`,
  `verify`, `report`; `measure.rs`: `words`, `word_change`, `new_words`, `kept_bigrams`, `prose`,
  `detect`, `preface`, `trailer`, `percentile`; `analyse.rs` the report tables; `whole.rs` the
  research's whole-document mode; `run.rs` flags incl. `--temperature`, `--top-p`, `--min-p`,
  `--base-seed`, `--variant`). Example with `required-features = ["local-llama"]`.
- Corpus: `crates/wipemark-pipeline/bench/corpus/` (en/ru/de items, special cases); results:
  `bench/results/`; variants: `bench/variants/` (`keep-voice/en/paraphrase.1.system.txt`,
  `numbers-in-digits`, `reminder-after-text`, `structural-text-only`).
- Shipped templates for the shape of every slot: `crates/wipemark-pipeline/prompts/<lang>/`
  (`<tactic>.<step>.<role>.txt`); the one rule every template passes: `prompt::row::admit`
  (`prompt/validate.rs`, incl. `{PROTECTED}` once per step, no hand-written markers, no invisible
  character — D369).
- CI: `.github/workflows/gate.yml` (`clippy` step at `:129` runs `--workspace --all-targets`
  without features, so the example — which needs `local-llama` — is never linted).

## 3. Requirements

### V1 — voice measures in the bench
Add to `measure.rs`, pure and unit-tested, per language (en, ru, de):
- **person retention**: counts of second-person (en `you/your/yours/yourself/yourselves`; ru
  `ты/тебя/тебе/тобой/твой…` and `вы/вас/вам/вами/ваш…`; de `du/dich/dir/dein…`, `ihr/euch/euer…`,
  and formal `Sie/Ihnen/Ihr…` counted separately) and first-person (I/we and their forms) in source
  and answer, and the share kept; a switch between informal and formal address in ru/de (ты↔вы,
  du↔Sie) counted as its own figure;
- **length ratio in words** (already partly `word_change` — expose the ratio itself);
- **register shift**: a small, documented proxy — e.g. the share of the answer's words that are in
  a list of formal connectives / nominalisation suffixes absent from the source (en "possess",
  "utilize", "thereby", "-tion/-ity" density; ru "осуществлять", "является", "данный",
  отглагольные "-ние/-ция"; de "-ung/-heit/-keit", "bezüglich", "hinsichtlich") — clearly labelled
  a proxy, with its lists in data files, not code.
Wire them into `run`'s records and `report`'s tables (per model × tactic × intensity: medians and
quartiles; the share of chunks that lost any second person), and into the judge prompt as a
**separate** "voice preserved? yes/no/partly" question whose answer is recorded beside the
meaning verdict (do not change the meaning question). Old records without the new fields still
report (fields optional).

### V2 — keep-voice variants in three languages
Under `bench/variants/keep-voice/`, the rule for **en, ru, de**, for every slot it should touch
(paraphrase and humanize system templates at least; say which and why), each a full copy of the
shipped template with the one rule added, each passing `admit` (a test walks the variant directory
and admits every file — red on a broken one). The ru/de wording written natively, not translated
word for word: "Сохраняй голос автора: обращайся к читателю так же, как текст (если в нём «ты» —
и ты пиши «ты», если «вы» — «вы»), сохраняй его тон и регистр, используй слова не сложнее его
собственных. Не делай текст официальнее и не удлиняй его." — and the German equivalent (du/Sie
kept as in the source). Also a variant **keep-voice-light** if the research's "keep-voice + light"
row is to be reproducible by flags alone — check whether `--grid` already covers intensity and do
not duplicate.

### V3 — the run the owner's host will make
A committed script `bench/run-voice.sh` (header per `CLAUDE.md`) that runs, for the four local
models (paths and GPU layers as environment variables with no defaults that point at a real disk;
Qwen3 4B, Gemma 3 12B, Gemma 4 12B, Qwen3.8 27B), the baseline and the keep-voice variant over the
corpus (`--every 3` as E4-5 did, all three languages), then `judge` with a named judge model, then
`report` with the voice columns, writing `bench/results/voice-<date>/`. It must refuse to start
without its model variables set, never download anything, and say how long each part takes
(calls × measured seconds/call). Dry-run mode (`--dry-run`) that prints the commands. Do **not** run
it in the container.

### V4 — CI lints the bench
Add a step to `gate.yml`'s Linux job: `cargo clippy -p wipemark-pipeline --features local-llama
--examples --locked -- -D warnings` (cheap; the `local-llama` shim needs no cmake). Make it pass.

### V5 — docs
`docs/architecture/prompt-bench.md`: the voice measures, what the register proxy is and is not,
the judge's voice question, `run-voice.sh`, and that D111 is to be re-read against these numbers.

## 4. Tests and gates

Unit tests for V1 (each language, address switch, empty texts, a text with no second person), the
variant walk for V2, `run-voice.sh --dry-run` exercised by a test or a CI step, and the report
tables reading old and new records. Each red once without its protection.

While iterating: `cargo test -p wipemark-pipeline --features local-llama --locked --examples` and
the lib tests. **Once, at the end**, all `--locked`, with counts:

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy -p wipemark-pipeline --features local-llama --examples --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
scripts/check-gpui-pin.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
```

Push and watch the `gate` workflow for your branch to completion.

## 5. Report

`docs/plan/reports/E4-8-bench-voice-2026-10-08.md` in the branch, and — if you have the Watchword
tools — the same text as Watchword FILE `wipemark-bench-voice-report-2026-10-08` (ttl 0; read back,
no `expires_at`): a table V1–V5 (done, commit, tests, what was removed to see red); decisions D420…;
gates with counts; CI URL and each job; **the exact command for the owner's host** to run V3 and
what to look at in its output; "Wanted edits" for `CLAUDE.md` and `docs/plan/README.md` (§7 E4: the
keep-voice and voice-measure bullets). Push `e4/bench-voice` only.
