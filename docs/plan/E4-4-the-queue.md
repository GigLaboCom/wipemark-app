# E4-4 — The queue: persisted, resumable per chunk, survives `kill -9`

|                  |                                                                                                                                       |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic E4, the pipeline                                                                                                   |
| Spec scopes      | S4.8 (OV §4.5 "the batch", §6.3 persistence); the E4 gate "the queue survives `kill -9`" (OV §10)                                       |
| Depends on       | E4-3 (the loop, `wipemark_pipeline::start`); the store (E0); `wipemark_intake::{name, inplace}` (E5-1, tails-1)                         |
| Runs beside      | E4-5 (the prompt bench) in another worktree — it calls `start` and may edit `crates/wipemark-pipeline/prompts/**`                       |
| Unblocks         | E4-6 (the surfaces that push documents into the queue), E7 (the workspace's Clean)                                                     |
| Files touched    | `crates/wipemark-pipeline/src/{lib.rs,report.rs,job/**,prompt/choose.rs}`, `crates/wipemark-store/src/**`, `crates/wipemark-intake/src/{text.rs,inplace.rs}`, `apps/wipemark-cli/src/input.rs`, **new** `crates/wipemark-queue/**`, `Cargo.toml`, `Cargo.lock`, `scripts/check-dep-direction.sh`, `docs/architecture/pipeline.md` |
| Size             | ~1½ days for one agent; no window, no model                                                                                           |

## §0 Ground rules

### 0.1 Start here

You are an implementer working alone in the worktree
`/home/denis/denis-ubuntu/sources/wipemark-e4-4`, branch `e4/queue`, cut
from `feat/e0-e6-shell` at `168df2b`. Read this document, then `CLAUDE.md`
in full — if the two disagree, `CLAUDE.md` wins and the report says so.

```sh
export GIT_CONFIG_NOSYSTEM=1
S=/tmp/claude-1000/-home-denis-denis-ubuntu-sources-wipemark-app/3f38a71d-77bc-40e9-94ee-ec30399f699c/scratchpad
export CARGO_TARGET_DIR=$S/target-e4-4 LIBRARY_PATH=$S/lib
```

One commit, `E4-4: The queue — persisted, resumable per chunk, survives
kill -9`, ending with the co-author line the prompt gives. Never push,
never touch `main`, leave ` m vendor/gpui-component` unstaged.

### 0.2 Where code goes — and why a new crate

```
core ← engine ← pipeline ← queue ← app / cli
                 store  ←┘   ↑
                 intake ─────┘   (and log, for Elided)
```

The queue has to do four things: run jobs (the pipeline), remember them
across a `kill -9` (the store), read a file the way every other surface
reads one and write a result the way the Retention rules say
(`wipemark-intake`). The pipeline may depend on core and engine only, the
store is a leaf, and nothing depends on an application. Three places were
possible:

| where | what it costs |
|---|---|
| the pipeline, with a storage trait (and a reader trait, and a writer trait), the adapters in the app | three traits with one implementation each; the scheduler's real behaviour — reading, writing, SQLite — only testable in the app, whose test binary links GPUI; and the CLI can never reach an app module |
| the app | the scheduler is dead code until E4-6 (clippy), the `kill -9` gate links GPUI, and again the CLI cannot reach it |
| **a crate of its own, `wipemark-queue`** | one more row in `check-dep-direction.sh`; nothing else |

So: **`crates/wipemark-queue`**, allowed `{engine, pipeline, store,
intake, log}`. The pipeline stays free of storage and files: it gains a
*resumable* entry point and a per-chunk record, both plain values; the
store gains two tables and an API of strings and integers, and knows no
pipeline type. The queue is the only place where the two meet.

### 0.3 Rules of this repository that bind this document

- **`start` is unchanged** — its signature, its events and its report
  JSON. E4-5 drives it. Everything new is beside it.
- **Nothing blocks the GPUI thread.** The queue's handle sends commands
  over a channel and returns; every database call, every file read and
  every write happens on the queue's own thread. The constructors open a
  database and read rows — the surface calls them off the foreground.
- **A database that will not open is left on disk byte for byte**, and
  the queue then runs on an in-memory store and says it will not survive
  a restart (`Durability::Memory`).
- **A result goes beside the file, and the file is never touched** unless
  the item was pushed with the per-run in-place destination; then
  `wipemark_intake::inplace::replace` sets the original aside first and
  never over an original already there.
- **Diagnostics go to a file, and the document never does.** Log lines
  carry ids, counts, states and kinds; a path or a text only as
  `wipemark_log::Elided`.
- **Only applications localize.** Every failure is a value.

### 0.4 Tests

Every protection gets a test that fails with the protection deleted — the
mutation table in the report lists each one. The scheduler is tested
through its public handle with `FakeEngine`, on a real SQLite file in a
scratch folder where persistence matters and in memory where it does not.

### 0.5 Gates — all green before committing

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
cargo test -p wipemark-app --features local-llama --locked
```

The new crate moves `Cargo.lock`: record it once with `cargo check -p
wipemark-queue` without `--locked`, then run the gates with it.

### 0.6 Do not

- change `start`, `prompts/**`, the bench, `apps/wipemark-app/src/{settings.rs,engine_host.rs}`;
- add a window, an MCP `rewrite` or a CLI `rewrite` (E4-6, E5-2);
- add a preference row: the queue's state is not a preference;
- launch the GUI.

### 0.7 Definition of done

All gates green; every criterion of §6 with evidence; "The queue" in
`docs/architecture/pipeline.md`; the report at
`docs/plan/reports/E4-4-2026-10-03.md` with the mutation table, the
`kill -9` gate's evidence and the edits `CLAUDE.md` and
`docs/plan/README.md` need (this step does not edit either).

---

## §1 Goal

A list of documents — files, or text with no file behind it — run one at a
time, with progress per document and inside it, pause, cancel, and
**continuation after the process dies**: a `kill -9` in the middle of a
document loses at most the chunk that was being rewritten, and the run
that follows re-asks only the chunks nobody had decided.

## §2 Read first

`CLAUDE.md` (dependency rules; "Preferences are rows"; "Nothing blocks";
"A result goes beside the file"; "Diagnostics go to a file").
`docs/plan/README.md` §4 D61, D83–D87 and §7 E4. OV §4.5 and §6.3.
`docs/plan/E4-3-the-loop.md` and its report's "What E4-4 should know".
`docs/architecture/{pipeline.md,retention.md,cli.md}`.
`crates/wipemark-pipeline/src/{lib.rs,job/,report.rs}`,
`crates/wipemark-store/src/{lib.rs,settings.rs}`,
`crates/wipemark-intake/src/inplace.rs`, `apps/wipemark-cli/src/input.rs`.

## §3 What is true today

- `start(JobId, Document, Options, Arc<dyn RewriteEngine>)` runs one job
  on a thread and ends with `Finished { outcome: { text, report } }`,
  `Cancelled` or `Failed`. A job has no checkpoint: a cancel or a crash
  loses every attempt it made.
- Chunks are independent — a chunk's context is the *source's* previous
  sentences, never a rewritten one — and `Prepared` is deterministic for
  the same text, format and budget. Seeds depend on the chunk's index
  (D83). So a chunk decided once is decided for good, as long as nothing
  it was decided from has moved.
- The store has one table, `settings`, at schema version 1.
- `decode`/`encode` (UTF-8/16/32, the byte order mark kept) and
  `same_file` live in the CLI's `input.rs`, where nothing else can reach
  them.
- OV §6.3 asked for `queue.json`; the store replaced files with rows
  (CLAUDE.md "Preferences are rows"), and the queue follows it.

## §4 Deliverables

### 4.1 The pipeline: a resumable job beside `start`

```rust
pub fn start_resumable(id, document, options, engine, carried: Vec<Decided>)
    -> Result<(JobHandle, flume::Receiver<Event>), Refused>
```

The same job as `start`, with two differences:

1. After a chunk is decided — rewritten, kept because nothing passed, kept
   because no tactic applied or a marker was in the text — it emits
   `Event::ChunkDecided { job, chunk, decided: Box<Decided> }`.
2. It takes back what an earlier run decided. After planning it emits
   `Event::Resumed { job, carried, discarded }` (only when it was handed
   anything), and a chunk with a usable record is **not asked again**: its
   winner (or its kept source) goes into the assembly as it was decided,
   and its report is the record's.

`start` is `run` with no journal: no `ChunkDecided`, no `Resumed`, no
`carried_over` key in its JSON — byte for byte what it was.

**`Decided`** — one chunk's decision, as a value and as one line of ASCII
JSON (`to_json`, `from_json`):

| field | what |
|---|---|
| `index` | the chunk, from 0 |
| `fingerprint` | the job's (below) |
| `digest` | sha256 of the chunk's text, its context and its protected spans |
| `est_tokens` | the chunk's estimate |
| `outcome` | `ChunkOutcome`, in the report's own JSON shape |
| `winner` | the passed candidate, placeholders and all, or `null` |
| `attempts` | the report's JSON of every attempt, **as recorded** |
| `counts` | attempts, rejected, calls, tokens out — so the totals stay honest |

**When a record is usable.** All of:

- its `fingerprint` is the job's — sha256 over a version tag, the
  document's text, its format, the options (`Debug`, so every field and
  every override counts), the engine's identity (vendor, model, local,
  window), the chunk budget and the usable rungs with their templates.
  **A changed source, changed options, another model or a changed template
  invalidates every record of the job**: half a document rewritten under
  one set of rules and half under another is a report that lies;
- its `index` names a chunk of this plan and its `digest` is that chunk's;
- its outcome and winner agree (`Rewritten` ⇔ a winner), and the winner
  still restores into the chunk.

Anything else is **discarded** — counted in `Resumed`, and the chunk is
asked again. A record is never repaired.

**The report.** `ChunkReport` gains `carried: Option<Carried>` —
`Carried { attempts: Value, counts: ChunkCounts }`. A carried chunk has no
typed attempts (core's `CleanReport` cannot be read back: core has no
serde); its JSON is `"carried_over": true` and the attempts as recorded;
`totals()` adds its counts. The final text of a resumed job is the text an
uninterrupted job gives — the same winners, the same assembly, the same
Layer A pass after.

**Options as a row.** `Options::to_json` / `Options::from_json`: every
field, the override rows by their D74 key and value. A queue item must run
after a restart with the options it was pushed with — a per-run "two
candidates" (D61) must not become the default because the machine
rebooted. An options value this build cannot read fails the item; it is
never read as the defaults. (`Overrides::iter` is added for it.)

### 4.2 The store: two tables

Migration 1 → 2 (appended, never edited):

```sql
CREATE TABLE queue (
    id     INTEGER PRIMARY KEY AUTOINCREMENT,   -- never reused: an id a surface remembered stays that item's
    state  TEXT NOT NULL,                        -- the queue's word; the store does not know it
    item   TEXT NOT NULL,                        -- JSON: what to do
    result TEXT                                  -- JSON: what came of it; NULL until it ends
);
CREATE TABLE queue_chunks (
    item   INTEGER NOT NULL REFERENCES queue(id) ON DELETE CASCADE,
    idx    INTEGER NOT NULL,
    record TEXT NOT NULL,                        -- a Decided, as the pipeline wrote it
    PRIMARY KEY (item, idx)
);
CREATE TABLE queue_control (id INTEGER PRIMARY KEY CHECK (id = 1), paused INTEGER NOT NULL);
```

`Store::queue()` → `Queue<'_>`: `insert`, `rows`, `last_id`, `set_state`,
`set_result`, `end` (state + result + chunk rows gone, one transaction),
`rename_state`, `put_chunk` (insert or replace), `chunks`, `remove`,
`paused`, `set_paused`. Strings and integers in, strings and integers
out. `PRAGMA secure_delete = ON` on every connection: a removed item's
pasted text is overwritten in the file rather than left in a free page.

### 4.3 The queue: `crates/wipemark-queue`

```rust
Queue::open(path, engine) -> Queue                 // falls back to memory, never fails
Queue::on(Arc<Store>, Durability, engine) -> Result<Queue, store::Error>   // the app's own store
queue.push(Request { source, format, destination, options }) -> Result<ItemId, Refused>
queue.pause() / resume() / cancel(item) / remove(item)
queue.items() -> Vec<ItemView>        queue.events() -> flume::Receiver<QueueEvent>
queue.durability()                    queue.shutdown()
```

- **Order.** FIFO by id, one job at a time. Ids are allocated in the
  handle (an atomic seeded from the table's sequence) so `push` returns at
  once; the insert happens on the queue's thread.
- **States** (the row's word): `queued`, `running`, `delivering`, `done`,
  `failed`, `cancelled`. At open, `running` becomes `queued` — that item
  was interrupted — and keeps its chunk rows; `delivering` is delivered
  again before anything else runs.
- **Progress.** Every pipeline event of the running item is forwarded as
  `QueueEvent::Job { item, event }`; `Added`, `Started`, `Ended { item,
  end }`, `Paused`, `Resumed`, `Unsaved` around them. `ItemView` carries
  the state and how many chunks are decided.
- **Pause** is persisted (`queue_control`): the running job is cancelled
  — E4-3's "a paused job is a cancelled one to be started again" — its
  item goes back to `queued` with its chunk rows, and nothing starts until
  `resume`. A queue paused before a restart is paused after it.
- **Cancel** ends an item as `cancelled` (its chunk rows go); a queued one
  never starts. **Remove** deletes the row (cancelling it first if it
  runs).
- **Resume inside a document.** Starting an item hands
  `start_resumable` every chunk row it has; every `ChunkDecided` is written
  (`put_chunk`) as it arrives. A `kill -9` therefore loses at most the
  chunk in flight.
- **A source that cannot be read** — missing, a folder, not text, an
  unnamed eight-bit encoding, invalid in its encoding — ends the item as
  `failed` with that reason, and the next item starts.
- **Reading** is the CLI's: the head through `wipemark_intake::identify`,
  refused before the rest is read; `wipemark_intake::text::decode` (moved
  from the CLI, which now calls it there). A file is read again on every
  start, so a file edited between a crash and the restart is the file
  that is rewritten — and its records are discarded by the fingerprint.

### 4.4 Where a result goes

The queue executes a destination chosen **when the item was pushed** and
stored in its row; it decides no policy (the Retention rows are the
surface's to read, E4-6/E7):

| `Destination` | what happens |
|---|---|
| `Row` | nothing is written; the text stays in the item's row until the item is removed. The only destination for text with no file behind it. |
| `File(path)` — `Destination::beside(source)` is `name.cleaned.ext` by `with_infix` | `inplace::write_atomically`, in the input's encoding with its mark, the input's permissions. Refused at push, and again at delivery, when `path` *is* the input (`inplace::same_file`, moved from the CLI). |
| `InPlace(Keep)` — the per-run flag | `inplace::replace`: the original aside first unless `Keep::Nothing`, never over one already there. Nothing changed, nothing touched (the CLI's rule). |

Delivery is two-phase: the result (text and report) is written to the row
as `delivering`, then the file is written, then the row becomes `done`
(the text dropped from it when it is on disk). A `kill -9` between the two
re-delivers at the next open: a `File` write is idempotent; an in-place
item whose original is already aside **and** whose file already holds the
result is done, not "original exists".

### 4.5 Documents

"The queue" in `docs/architecture/pipeline.md`; the report.

## §5 Tests

Pipeline (`job/tests.rs`, `job/resume.rs`, `job/stored.rs`):
`start_emits_no_chunk_record`, `a_resumed_job_asks_only_the_undecided_chunks_and_ends_with_the_same_text`,
`a_carried_chunk_is_reported_as_carried_with_its_attempts_as_recorded`,
`a_changed_document_discards_every_record`, `changed_options_discard_every_record`,
`another_engine_discards_every_record`, `a_record_for_another_chunk_or_with_a_broken_winner_is_discarded`,
`a_record_round_trips`, `options_round_trip_with_overrides`,
`an_unreadable_options_row_is_an_error_not_the_defaults`.

Store: `the_queue_tables_exist_at_version_2`, `a_version_1_database_migrates_and_keeps_its_settings`,
`ending_an_item_drops_its_chunks`, `removing_an_item_drops_its_chunks`,
`ids_are_never_reused`, `pause_is_one_row_and_survives_a_reopen`, `a_removed_item_leaves_no_text_in_the_file`, `rename_state_moves_every_item_in_that_state`.

Queue (`crates/wipemark-queue/tests/`): `items_run_one_at_a_time_in_the_order_they_came`,
`pause_stops_the_job_and_resume_carries_its_decided_chunks`, `a_paused_queue_stays_paused_after_a_restart`,
`cancel_ends_the_running_item_and_the_next_starts`, `a_cancelled_queued_item_never_starts`,
`an_unreadable_document_fails_and_the_queue_moves_on`, `an_interrupted_item_resumes_after_a_restart`,
`a_file_changed_before_the_resume_is_rewritten_whole`, `a_result_goes_beside_the_file_and_the_file_is_untouched`,
`a_destination_that_is_the_source_is_refused`, `in_place_sets_the_original_aside`, `in_place_with_nothing_to_change_touches_nothing`, `events_say_how_far_a_document_got`,
`an_interrupted_delivery_is_finished_not_failed`, `text_has_no_file_to_write_and_stays_in_the_row`,
`a_database_that_will_not_open_is_left_alone_and_the_queue_runs_in_memory`,
`no_document_text_or_path_reaches_a_log_line`, and **the gate**:

`the_queue_survives_kill_9` (`tests/kill.rs`): a child process — the same
test binary, re-executed on an `#[ignore]`d entry with the database's path
in an environment variable — runs a queue over a real SQLite file and a
six-paragraph document with a `FakeEngine` whose tokens arrive slowly. The
parent watches the database read-only, sends `SIGKILL` (`Child::kill`)
once at least two chunks are recorded and before the item ends, reopens
the queue on the same file, and asserts: the item resumes; the engine is
asked **only** for the chunks without a record; the final text equals an
uninterrupted run's; the report marks exactly the recorded chunks as
carried. `Child::kill` is `SIGKILL` on Unix and `TerminateProcess` on
Windows — equally abrupt — so the test is not gated by platform; it has
been run on Linux.

## §6 Acceptance criteria

1. `start` unchanged: its existing tests untouched and green, no new event
   from it.
2. A resumed job asks only the undecided chunks and ends with the
   uninterrupted text; a changed source, options, engine or template
   discards every record.
3. The queue runs FIFO, one at a time; pause, cancel and remove behave as
   §4.3; an unreadable document fails and the queue moves on.
4. The `kill -9` gate is green against a real file.
5. A database that will not open is left byte for byte and the queue says
   it is in memory.
6. No document text or path in a log line.
7. Every protection has a mutation that turned its test red.

## §7 Out of scope

The surfaces (E4-6, E7): reading the Retention rows into a
`Destination`, the `results` folder, keeping text under `kept/`, the
adapter from `EngineHandle`, showing progress. History beyond the queue's
own rows (OV §6.3's `history.jsonl`). Reordering items. Two processes
running queues over one database (the application owns the queue; the CLI
opens the database read-only).

## §8 Basis and references

OV §4.5, §6.3, §10; `docs/plan/README.md` §4 (D61, D83–D87), §7 E4;
`docs/plan/reports/E4-3-2026-10-03.md`; `docs/architecture/retention.md`
rules 1–4; `docs/architecture/cli.md` "`clean --in-place`".
