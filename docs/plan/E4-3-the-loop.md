# E4-3 — The loop: candidates, rounds, guards, selection, events, report

|                  |                                                                                                                                       |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic E4, the pipeline (README §7 E4, D76)                                                                               |
| Spec scopes      | S4.3 (the job state machine, events, cancel), S4.5 (candidates × rounds with escalation), S4.6 (scorers: divergence only, D72); OV §4.2–4.4 |
| Depends on       | E4-1 (preparing the text) and E4-2 (the prompts), both merged at `1495d3a`; E2 (the engines)                                          |
| Unblocks         | E4-4 (the batch queue), E4-5 (the prompt bench), E4-6 (the surfaces), E5-2 (`rewrite` in the CLI)                                     |
| Files touched    | `crates/wipemark-pipeline/**` (new modules `job/`, `select.rs`, `cost.rs`, `report.rs`, `tests/live.rs`; `lib.rs`; `Serialize`-free additions to `prompt/`), `crates/wipemark-engine/src/fake.rs`, `Cargo.toml` and `Cargo.lock` (`tracing` for the pipeline), `docs/architecture/pipeline.md`, this document, the report |
| Size             | ~2 days for one agent; no window; one live run on Qwen3 4B                                                                            |

## §0 Ground rules

### 0.1 Start here

You are an implementer agent working alone in the worktree
`/home/denis/denis-ubuntu/sources/wipemark-e4-3` (branch `e4/loop`, from
`feat/e0-e6-shell` at `1495d3a`) of `GigLaboCom/wipemark-app`, a Rust + GPUI
desktop application that strips AI-provenance marks from its owner's own
text. Read this document, then `CLAUDE.md` at the repository root in full —
if the two disagree, `CLAUDE.md` wins and you say so in your report.

```sh
export GIT_CONFIG_NOSYSTEM=1         # /etc/gitconfig is unreadable on this machine
cd /home/denis/denis-ubuntu/sources/wipemark-e4-3
git status                           # clean apart from ` m vendor/gpui-component`
S=/tmp/claude-1000/-home-denis-denis-ubuntu-sources-wipemark-app/3f38a71d-77bc-40e9-94ee-ec30399f699c/scratchpad
export CARGO_TARGET_DIR=$S/target-e4-3
export LIBRARY_PATH=$S/lib           # app crates link only with the libxkbcommon-x11 symlink there
```

Temporary files go in `$S`, never `/tmp` directly. Commit once, at the end:
`E4-3: The loop — candidates, rounds, guards, selection, events, report`,
ending with the co-author line the prompt gives. Never push, never touch
`main`, never edit `vendor/`; leave ` m vendor/gpui-component` unstaged.
Another agent works in `fixes/tails-1` at the same time: **do not touch
`apps/` or `crates/wipemark-intake`.**

### 0.2 Where code goes

```
core ← engine ← pipeline ← app / cli          (scripts/check-dep-direction.sh)
```

Everything is `crates/wipemark-pipeline`, beside `prepare/` and `prompt/`:

```
src/job/mod.rs      Options, Document, start, JobHandle, Refused, the job's thread
src/job/plan.rs     Planned: Layer A first, the budget, prepare, the usable ladder, template fallbacks
src/job/attempt.rs  one attempt: render → complete → clean_response → Layer A → guards → restore → no-op
src/job/drive.rs    the minimal executor that drives an engine's future and forwards its tokens
src/job/tests.rs    the job on FakeEngine
src/select.rs       divergence, the no-op floor, the length penalty, the scorer seam, the winner
src/cost.rs         Executor, Effort (D61's defaults), Cost and the estimate
src/report.rs       Rejection, EngineFailure, JobReport and its records, the JSON form
tests/live.rs       the live gate on Qwen3 4B (llama-native, #[ignore])
```

`crates/wipemark-engine/src/fake.rs` gains a scripted mode (§4.9) — the
only change outside the pipeline crate. `wipemark-core` is read, not
changed: it has zero dependencies, so the report the pipeline needs
(serde) cannot live there.

### 0.3 Rules of this repository that bind this document

- **Nothing blocks the GPUI thread.** `start` validates the options and
  spawns; everything else — Layer A over a large document included — runs on
  the job's own thread. No async runtime of our own: the engine's future is
  driven by a dozen lines over `std::task` (§4.2), which is also how
  `engine_host`'s tests drive one. **No new dependency** from outside the
  workspace; `tracing`, the workspace's logging crate, joins the pipeline's
  manifest for the job's log lines (one line in `Cargo.lock`).
- **Only applications localize.** Every rejection, refusal and failure is a
  value (an enum with fields); `Display` impls are English for logs. The
  report's JSON field names and ids are formats.
- **The third shelf is never empty.** Every report carries
  `not_established`, from `FinalReport::baseline_not_established()` plus
  `UNKNOWN_MARK_SCHEMES` (no mark scheme is searched for: D72).
- **A document goes back byte for byte.** Everything outside a chunk is the
  source's bytes; a chunk no candidate won is its (Layer-A-cleaned) source.
  A candidate that failed a guard is **never** used.
- **Diagnostics never carry document text.** `tracing` lines name counts,
  sizes, tactics, seeds and rejection kinds; never a chunk or a candidate.
- **Tests must be able to fail.** For every protection a mutation that turns
  its test red, recorded (protection · mutation · test).

### 0.4 Tests

- On `FakeEngine` (scripted, §4.9) — no network, no weights, no GPU.
- Names are behaviour sentences. Each protection gets a mutation (§5).
- The live gate (§4.10) is `#[ignore]`d and run by hand.

### 0.5 Gates — all green before committing

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
```

Plus, because `fake.rs` moves: `cargo test -p wipemark-engine --features
local-llama --locked`. Plus the live gate (§4.10). The native build needs
`crates/wipemark-llama-sys/vendor/fetch.sh` once (`WIPEMARK_LLAMA_SRC` at
the main tree's checkout fetches without a network).

### 0.6 Do not

- touch `apps/` (E4-6 wires the windows, the MCP `rewrite` tool and the
  CLI), `crates/wipemark-intake`, `CLAUDE.md` or `docs/plan/README.md`
  (the coordinator edits them; list the edits in the report);
- build the batch queue or its persistence (E4-4), the prompt bench (E4-5),
  the template editing UI, the humanizer pass (D73: not in E4), the keyed
  scorer (D72), the `code` tactic's own preparation;
- add a dependency from outside the workspace (`tracing` is the
  workspace's own and is allowed: libraries log through it).

### 0.7 Definition of done

1. All gates of §0.5 green; every mutation of §5 recorded red.
2. The live gate run, its outputs and timings in the report.
3. `docs/architecture/pipeline.md` has a section "The loop".
4. The crate doc's "Status" updated; `PipelineError::NotImplemented` gone.
5. The report at `docs/plan/reports/E4-3-2026-10-03.md`.

---

## §1 Goal

Join E4-1 and E4-2 into the thing the product is for: a **job** that takes
a document and an engine, cleans it with Layer A, prepares it, asks the
model for candidates of every chunk round by round, throws away every
candidate that lost something, keeps the one that changed the least, puts
the document back together byte for byte, cleans it with Layer A again,
and says — event by event, and in a report at the end — exactly what it
did, including every attempt that failed.

## §2 Read first

- `CLAUDE.md` — "A document goes back byte for byte", "The prompts are
  data, and the assembler owns the markers", "Nothing blocks the GPUI
  thread", "A model is loaded by policy", "The third shelf is never
  empty", "Tests must be able to fail".
- `docs/plan/README.md` §4 D60–D78 (D61, D67, D70, D71, D72, D73, D77, D78
  above all) and §7 E4.
- `docs/plan/reports/E4-1-2026-10-03.md` and `E4-2-2026-10-03.md`, their
  sections "What E4-3 should know"; `docs/architecture/{pipeline,prompts}.md`.
- `docs/sdd/layer-b-rewrite-reference.md` §5 (the selection loop, the
  no-op guard, the length penalty, `min-divergence`) and §7; OV §4.2–4.4.
- The register, `ssd-docs/wipemark-open-questions-2026-10-03.md` §2,
  "[И] E4-3".
- Code: `crates/wipemark-pipeline/src/{lib.rs,lang.rs,prepare/,prompt/}`;
  `crates/wipemark-engine/src/{lib.rs,fake.rs}`;
  `crates/wipemark-core/src/{lib.rs,guard.rs,report.rs}`;
  `apps/wipemark-app/src/engine_host.rs` (read only: `EngineHandle`).

## §3 What is true today

- `prepare(text, format, budget) -> Prepared` with `chunks()`,
  `language()`, `assemble(&[Option<&str>])`; `Chunk { text, protected,
  context, est_tokens, .. }` and `Chunk::restore -> Result<String,
  RestoreError>`; `Budget::for_context(ctx_len)`; `estimate_tokens`.
  `TextFormat::Code` yields no chunk.
- `templates_for(doc, tactic, pivot_row, &overrides) -> Result<Plan,
  Refusal>` (`Refusal::BackTranslateNeedsLanguage`); `render(&step,
  &Input) -> Result<Rendered, RenderError>`; `Rendered::into_request(params)`;
  `clean_response(raw, input) -> Cleaned { text, stripped }`; `Version`
  per chosen template.
- `wipemark_core::{clean, Options, default_guards, Guard, RejectReason,
  LengthDriftGuard, CleanReport, FinalReport}`. `RejectReason` is
  structured; `default_guards()` is placeholder, numbers, length-drift
  (0.6–1.6), script, identifier, in that order.
- `RewriteEngine::complete(req, sink, cancel)` is async; the engines'
  futures are runtime-agnostic (they wait on `flume` channels fed by their
  own threads). `FakeEngine` rotates the words of the whole prompt.
- `lib.rs` has `Event::CandidateRejected { guard: &'static str, reason:
  String }` (prose in a library), `Event::Finished { elapsed }` (no
  result), `PipelineError::NotImplemented`. Nothing outside the crate uses
  any of them.

## §4 Deliverables

### 4.1 The entry

```rust
pub fn start(
    id: JobId,
    document: Document,              // { text: String, format: TextFormat }
    options: Options,
    engine: Arc<dyn RewriteEngine>,
) -> Result<(JobHandle, flume::Receiver<Event>), Refused>;
```

- Returns at once. `Refused` is what `Options::check` finds (an empty
  ladder, zero candidates or rounds, `structural` unconfirmed or not the
  last rung, the `code` tactic — whose preparation is not built) or a
  thread that could not be spawned. Everything else happens on the job's
  thread, named `wipemark-job-<id>`.
- `JobHandle` is `Clone + Send + Sync`: `id()`, `cancel()`,
  `is_cancelled()`. The thread is detached; the receiver is the only way the
  result comes back.
- **The engine is an `Arc<dyn RewriteEngine>`.** `EngineHandle` (the app's)
  is not one today: it has `complete` but no `info`, and its engine is built
  lazily for an endpoint. E4-6 wraps it in a small adapter that implements
  `RewriteEngine` — `info` from the engine on duty when the job is started,
  `complete` through the handle (so the busy count and the keep policy see
  the job), `warmup` through the host's load. No new trait is introduced
  here: one is already the seam.
- `Options`:

  ```rust
  pub struct Options {
      pub layer_a: wipemark_core::Options,
      pub ladder: Vec<Tactic>,           // default [Paraphrase] (OV §4.3)
      pub structural_confirmed: bool,    // D73: structural only behind a confirmation
      pub intensity: Intensity,
      pub effort: Effort,                // { candidates, rounds } — D61 (§4.6)
      pub base_seed: u64,
      pub sampling: SamplingParams,      // `seed` is set per attempt; `max_tokens: None` → per call (§4.3)
      pub pivot: Option<Lang>,           // the `rewrite.pivot` row (D60)
      pub overrides: Overrides,          // template rows that parsed (D74)
      pub length: LengthDriftGuard,      // the length guard's window, 0.6–1.6 by default
  }
  impl Options { pub fn for_executor(executor: Executor) -> Options; pub fn check(&self) -> Result<(), Refused>; }
  ```

  `length` replaces the default length guard in the guard list; it exists so
  the bench (E4-5) can move the window without forking the loop, and it is
  the only guard threshold exposed.

### 4.2 Driving the engine (no runtime)

`job::drive::block_on(future, tokens, on_token)`: poll the engine's future
with a waker that unparks the job's thread; while it is pending, also poll
`tokens.recv_async()` with the same waker, so **a token arriving wakes the
thread** and is forwarded at once as `Event::Token` — in order with the
job's other events, because one thread sends them all. No `futures`, no
`pollster`, no tokio: a dependency for twenty lines of `std::task` is not
worth its lock entry, and the app already has the same loop for its tests.

### 4.3 The flow (one job, on its thread)

1. `Stage::CleaningLayerA`. `before = wipemark_core::clean(text, layer_a)`.
2. **Plan** (`plan.rs`, a pure function also offered to E4-6 for the cost
   before a run — `job::plan(&document, &options, &engine.info())`):
   - The usable ladder: for each rung, `templates_for(language, tactic,
     pivot, &overrides)`; `Refusal::BackTranslateNeedsLanguage` **skips the
     rung** and is recorded (`SkippedTactic`).
   - Every step of every usable plan is rendered once with a probe input; a
     `RenderError::Template { slot, problems }` drops that slot's override,
     the plan is chosen again (the shipped template now), and a
     `TemplateFallback { slot, problems }` is recorded. A shipped template
     that does not render fails the job (`PipelineError::ShippedTemplate`) —
     it is a bug, not a user error.
   - **The budget leaves room for the prompt** (register, E4-3):
     `Budget::for_context(ctx_len − overhead)` where `overhead` is the
     largest `estimate_tokens(system + prompt)` of a probe render over the
     usable steps; `ctx_len: None` is `Budget::DEFAULT`.
   - `prepare(before.text, format, budget)`; the language is
     `Prepared::language()`. The ladder is computed from that language, so
     planning prepares with a provisional budget first and re-prepares only
     if the overhead moved it.
3. Nothing to rewrite — no chunk (a `Code` document, a document of
   headings) or no usable rung — **calls no engine**: the result is the
   cleaned text, every chunk `KeptSource`, the report says why.
4. `Stage::LoadingModel`, `engine.warmup()`. `Cancelled` cancels; any other
   error fails the job (`PipelineError::Engine` / `Unavailable`).
5. Per chunk (in order; chunks are independent — the context is the
   source's, E4-1), per round `r` in `1..=rounds`, tactic =
   `ladder[min(r−1, len−1)]` — **one rung per round**, the last rung repeated
   if there are more rounds than rungs; per candidate `c` in
   `1..=candidates`: `Stage::Rewriting { chunk, chunks, candidate,
   candidates, round, rounds }` (all 1-based), then one attempt (§4.4).
   A round runs **all** its candidates; **round `r+1` runs only if no
   candidate of round `r` passed** (D61, reference §5). After the last
   round: the winner (§4.5), or `KeptSource { NoCandidatePassed }`.
6. `assemble` with `Some(winner)` per won chunk and `None` elsewhere, then
   `after = clean(assembled, layer_a)` — the final pass over the whole
   document. `Stage::Finished`, then `Event::Finished { outcome }`.

Engine errors during an attempt: `Cancelled` ends the job (§4.7);
`Unavailable` fails the job (nothing can answer — asking again for every
candidate of every chunk would only repeat it); `Transport`, `Protocol`,
`ContextOverflow`, `NotImplemented` reject **that attempt**
(`Rejection::Engine`) and the loop goes on.

`max_tokens` per call, when the options leave it `None`: `2 ×
estimate_tokens(input) + 64`. The estimate already over-counts (E4-1:
×1.06–1.39), and the length guard rejects anything past 1.6× anyway; the
engines clip it to the window.

### 4.4 One attempt

For a two-step tactic, step 2's input is step 1's **cleaned** answer and
both steps use the attempt's seed; a one-step tactic has one step.

```
render(step, Input { text, context: chunk.context, intensity })
→ into_request(sampling with seed and max_tokens)
→ engine.complete (tokens → Event::Token)          error  → Rejection::Engine { step, failure }
                                                   Length → Rejection::Truncated { step }
→ clean_response(raw, input.text)                  empty  → Rejection::Empty { step }
(after the last step)
→ wipemark_core::clean(candidate, layer_a)         the model's own invisible characters, counted per attempt
→ guards(chunk.text, candidate)                    first reject → Rejection::Guard { guard, reason }
→ chunk.restore(candidate)                         any RestoreError → Rejection::Restore(e)  (OutOfOrder and ItemBroken are what a guard cannot see)
→ divergence(chunk.text, candidate) < 0.05         → Rejection::NoOp { divergence }   (D71)
→ passed: Scores { divergence, length_ratio, score }
```

Layer A runs **before** the guards: a model that slips U+200B into an
identifier must not lose the candidate to `IdentifierGuard` (README §7 E4).
`Event::CandidateRejected { job, chunk, round, candidate, rejection }` for
every rejection.

**Seeds** are unique across the job and recorded per attempt:

```
seed = base_seed + ((chunk_index · rounds + (round − 1)) · candidates + (candidate − 1))     (wrapping)
```

OV §4.4's `base_seed + round·c` collides ((1,2) and (2,1)) and repeats per
chunk; this one numbers every attempt of the job once, and a report
reproduces any of them.

### 4.5 Selection (`select.rs`, D71, D72)

- `divergence(a, b) = 1 − |B(a) ∩ B(b)| / |B(a) ∪ B(b)|` over word bigrams;
  a word is a placeholder `⟦n⟧` or a maximal run of alphanumeric characters,
  lower-cased; a text of one word has its single word as its "bigram"; two
  empty texts diverge 0.
- `NO_OP_FLOOR = 0.05`: below it a candidate is a no-op and fails.
- `length_ratio` = code points of the candidate / of the chunk text (the
  length guard's measure). Outside `LENGTH_WINDOW = 0.5..=2.0` it is
  docked: `score = divergence + LENGTH_PENALTY (0.15)` — "docked" for a
  minimiser is a penalty added, so it wins less often. With the default
  length guard (0.6–1.6) the penalty cannot fire; it is live for a wider
  window (the bench's), and the report says so.
- `Scorer::Divergence` is the only scorer. The enum is the seam for a keyed
  one (D72): a `KeyedGumbel` arm would score passed candidates by its
  p-value; nothing else in the loop changes.
- `winner(&[Scored]) -> Option<usize>`: the passed candidate with the lowest
  `score`; a tie goes to the earlier attempt.

### 4.6 Effort and cost (`cost.rs`, D61, OV §4.4)

```rust
pub enum Executor { LocalCpu, LocalGpu, Endpoint }       // how the app learns which is E4-6's
pub struct Effort { pub candidates: u8, pub rounds: u8 }
impl Effort { pub fn for_executor(e: Executor) -> Effort } // 1×2, 2×2, 2×2
pub struct Bound<T> { pub worst: T, pub expected: T }       // expected = every chunk passes in round 1
pub struct Cost { pub chunks: u32, pub calls: Bound<u32>, pub tokens_in: Bound<u64>,
                  pub tokens_out: Bound<u64>, pub seconds: Option<Bound<f64>> }
impl Planned { pub fn cost(&self, options: &Options, tokens_per_second: Option<f32>) -> Cost }
```

Calls: per chunk, per round, `candidates × steps(tactic of the round)`.
Tokens in: `estimate_tokens(system + prompt)` of the chunk's real render per
step (step 2's input stands in as the chunk). Tokens out: the chunk's
`est_tokens` per step. Seconds: tokens out ÷ the caller's tokens per
second; `None` gives `None`, never a guess (prompt processing is not
counted, and the report says so).

### 4.7 Events and cancel

```rust
pub enum Event {
    Stage { job, stage },
    Token { job, text },
    CandidateRejected { job, chunk: u32, round: u8, candidate: u8, rejection: Rejection },
    Finished { job, elapsed, outcome: Box<Outcome> },   // Outcome { text, report }
    Cancelled { job, elapsed },
    Failed { job, elapsed, error: PipelineError },
}
```

Every job ends with exactly one of the last three, preceded by the matching
`Stage` (`Finished`, `Cancelled`, `Failed`). `Stage::Queued`,
`Inspecting` and `Scoring` stay in the vocabulary for E4-4/E4-6 and are not
emitted (selection is instant). Cancel is a `CancellationToken` passed to
every `complete` and read between attempts; a cancelled job returns **no
document**. A receiver that was dropped cancels the job too: nobody is left
to read its result.

### 4.8 The report (`report.rs`)

Pipeline-owned, `serde::Serialize` (through one `to_value`), and `to_json()`
that writes ASCII only (every non-ASCII character escaped, as core's A §7.1
writer does — a report must not carry an invisible character).

```
JobReport { before: CleanReport, after: CleanReport, engine: EngineInfo, format, language,
            pivot, ladder, skipped: Vec<SkippedTactic>, fallbacks: Vec<TemplateFallback>,
            intensity, effort, base_seed, scorer, chunks: Vec<ChunkReport>,
            not_established: Vec<&'static str> }   + totals()
ChunkReport { index, est_tokens, attempts: Vec<Attempt>, outcome: Rewritten { round, candidate } | KeptSource(Kept) }
Kept        { NoCandidatePassed, NoTactic, MarkerInText { marker } }
Attempt     { round, candidate, tactic, seed, steps: Vec<StepRecord>, layer_a: Option<CleanReport>,
              verdict: Passed(Scores) | Rejected(Rejection), elapsed }
StepRecord  { step, system: Version, user: Version, tokens_out, finish, stripped: Vec<Stripped> }
Totals      { chunks, rewritten, kept_source, attempts, rejected, calls, tokens_out }
```

JSON: `{"version":1, "verifiable":{"before","after"}, "best_effort":{…},
"not_established":[ids]}` — the three shelves as keys. Layer B is always
on the *best-effort* shelf. Core's `RewriteSummary`/`FinalReport` are not
used: a single-attempt summary with a detector's `passed` does not describe
this loop, and core cannot take serde. Their fate is the coordinator's
(report, "For the coordinator").

`Rejection` (the structured reason, no prose):

```rust
pub enum Rejection {
    Engine { step: u8, failure: EngineFailure },  // Transport/Protocol { detail }, ContextOverflow { used, limit }, NotImplemented { what }
    Truncated { step: u8 },
    Empty { step: u8 },
    Guard { guard: &'static str, reason: RejectReason },
    Restore(RestoreError),
    NoOp { divergence: f32 },
}
```

`RejectReason` stays **exhaustive** (no `#[non_exhaustive]`): its readers
are this workspace's surfaces, which must render every variant from the
catalogue; a new variant should break their build, not fall into a
wildcard arm that says nothing. The same holds for `Rejection`.

### 4.9 `FakeEngine`, scripted

`FakeEngine::answering(|req: &ChatRequest, call: usize| -> String)`
answers each request with the closure's text, streamed in
whitespace-inclusive pieces; `with_token_delay(Duration)` sleeps between
pieces (a slow stream for cancel); `asked()` lists every request it was
sent, shared across clones; `max_tokens` cuts the answer with
`FinishReason::Length`. The rotation mode is unchanged.

### 4.10 The live gate

`tests/live.rs`, `#![cfg(feature = "llama-native")]`, `#[ignore]`, model
from `WIPEMARK_TEST_GGUF` (panics with the download line when unset):

```sh
WIPEMARK_TEST_GGUF=$S/models/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf \
cargo test -p wipemark-pipeline --features llama-native --locked -- --ignored --test-threads=1 --nocapture
```

Two documents, 1 candidate × 2 rounds, ladder `[paraphrase, humanize]`: a
Russian Markdown note with inline code and a list; an English plain
paragraph with a URL and numbers. Asserted: the job finishes; every chunk is
rewritten or kept; protected spans and numbers are in the result; the
report's JSON parses and carries the third shelf. Printed: the outputs,
every attempt's verdict, timings. Recorded in the report.

### 4.11 Found by the live gate: a list re-split by the model

The first live run (Qwen3 4B, a Russian list of three items) came back with
every placeholder present and in order — so `PlaceholderGuard` and
`OutOfOrder` both passed — and the items re-split: a line break inside item
1 carrying item 2's words, item 2 reduced to `;`, item 3 to its protected
file name. `Chunk::restore` (E4-1's) gains one check: in a chunk with list
glue, no item may come back with more line breaks than it went in with
(edges trimmed — the newline before a glue placeholder is the glue's).
`RestoreError::ItemBroken { item }`; the loop rejects it like every other
restore error. A chunk without glue is not checked: a re-wrapped paragraph
is still one paragraph. What it cannot see: words moved across a boundary
with no line break added — the bench (E4-5) should measure how often.

## §5 Tests

| test | protects | mutation |
|---|---|---|
| `a_candidate_that_loses_a_placeholder_is_rejected_by_the_placeholder_guard` | guards run, structured reason | skip the guards |
| `a_candidate_that_loses_a_number_is_rejected_by_the_numbers_guard` | | |
| `a_candidate_that_drifts_in_length_is_rejected_by_the_length_guard` | `options.length` reaches the guard list | use the default guard |
| `a_candidate_that_changed_nothing_is_rejected_as_a_no_op` | D71 floor | floor 0 |
| `list_items_that_come_back_out_of_order_are_rejected_by_restore` | `OutOfOrder` is a rejection | ignore restore errors |
| `a_list_whose_words_moved_across_its_items_is_rejected_by_restore` (+ a `prepare` unit case) | `RestoreError::ItemBroken` — added after the live gate (§4.11) | `check_items` always `Ok` |
| `a_second_round_runs_only_when_the_first_had_no_pass` | D61 | always run every round |
| `the_second_round_climbs_the_ladder_one_rung` | escalation | stay on rung 1 |
| `the_least_changed_passed_candidate_wins` | min-divergence | max |
| `a_candidate_outside_half_to_twice_its_length_is_docked` (unit + job) | D71 penalty | no penalty |
| `with_no_pass_the_chunk_keeps_its_cleaned_source_and_the_report_says_so` | a failed candidate is never used | use the last candidate |
| `an_all_rejected_job_returns_the_layer_a_cleaned_source` | | |
| `an_invisible_character_the_model_adds_is_gone_and_counted` | Layer A after the model | skip it |
| `layer_a_runs_before_the_guards_so_a_zwsp_in_an_identifier_costs_nothing` | order | guards first |
| `the_whole_result_is_cleaned_after_assembly` | the final pass | skip it |
| `seeds_are_unique_across_a_job_and_recorded` | the seed rule | OV's `base + round·c` |
| `cancel_mid_stream_ends_cancelled_promptly_with_no_document` | cancel | ignore the token |
| `back_translate_is_skipped_for_an_undetected_language` | | fail the job |
| `an_invalid_override_falls_back_to_the_shipped_template_and_is_recorded` | | |
| `a_code_document_never_calls_the_engine` | | |
| `events_arrive_in_order_with_their_stage_numbers` | | 0-based numbers |
| `the_cost_estimate_counts_calls_exactly` | `calls.worst` / `calls.expected` = what a job made | count rounds once |
| `every_report_carries_a_non_empty_third_shelf` | rule | empty shelf |
| `the_report_json_is_ascii_and_names_every_shelf` | | |
| `a_two_step_tactic_feeds_step_ones_cleaned_answer_to_step_two` | E4-2's road | feed the raw answer |
| `the_executor_decides_candidates_and_rounds` | D61 | |
| `structural_is_refused_unless_confirmed_and_last` | D73 | |
| `an_unavailable_engine_fails_the_job` | | reject the attempt instead |
| `a_truncated_answer_is_rejected` | | |
| `an_empty_answer_is_rejected_as_empty` | | accept it |
| `the_chunk_budget_leaves_room_for_the_prompt` | register item | no overhead |
| `divergence_*` units (identical 0, disjoint 1, placeholders are words) | | |

## §6 Acceptance criteria

1. All gates green; every mutation of §5 recorded red (or why it could not
   be).
2. The live gate run on Qwen3 4B with outputs, rejections and timings.
3. No new dependency from outside the workspace; `Cargo.lock` moves by
   the pipeline's `tracing` line only.
4. `docs/architecture/pipeline.md` "The loop"; the report.

## §7 Out of scope

The batch queue and its persistence (E4-4); the bench and the thresholds it
confirms (E4-5); the MCP `rewrite` tool, the windows, the CLI, the
`Executor` detection, the `EngineHandle` adapter (E4-6, E5-2); the keyed
scorer (D72); the humanizer pass (D73); the `code` tactic's preparation;
`RiskLabel` (no rule for it exists yet).

## §8 Basis and references

OV §4.2–4.4; `docs/sdd/layer-b-rewrite-reference.md` §5, §7; D60–D78;
E4-1 and E4-2 reports; the register §2 "[И] E4-3".
