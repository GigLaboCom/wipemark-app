# The pipeline

What happens between "the user pressed Rewrite" and "here is the
document", in `crates/wipemark-pipeline`. Epic **E4**, built as a series
(README §7 E4, decision D76): preparing the text (E4-1), the prompts
(E4-2), the loop that joins them (E4-3), the queue (E4-4), the bench
(E4-5), the surfaces (E4-6). Since E4-3 a job rewrites a document; nothing
in a window or the CLI starts one until E4-6. This page grows a section per
step.

## Preparing the text

E4-1 (spec S4.1, S4.2; decisions D65, D68, D69, D70 and the owner's
"rewrite by paragraph" of 2026-10-03).

```
crates/wipemark-pipeline/src/prepare/mod.rs       the API: TextFormat, Budget, prepare, Prepared, Chunk, restore, assemble, placeholder
crates/wipemark-pipeline/src/prepare/markdown.rs  Markdown's pieces, from pulldown-cmark's offsets
crates/wipemark-pipeline/src/prepare/html.rs      HTML's pieces, from a tokenizer of our own
crates/wipemark-pipeline/src/prepare/plain.rs     plain text's paragraphs
crates/wipemark-pipeline/src/prepare/protect.rs   protected spans: the lexical matchers, the merge
crates/wipemark-pipeline/src/prepare/sentence.rs  where a piece may be cut: sentences, then words
crates/wipemark-pipeline/src/prepare/chunk.rs     packing, splitting, the text a model sees, the context
crates/wipemark-pipeline/src/prepare/tokens.rs    estimate_tokens, calibrated against Qwen3's tokenizer
crates/wipemark-pipeline/src/lang.rs              Lang (shared with E4-2) and detect
```

### The one invariant

**Everything that is not inside a chunk is the source's own bytes.**
`Prepared::assemble` copies the source between chunks and puts either a
restored candidate or the chunk's own source in each chunk's range, so a
heading, a code block, a table, front matter, a `<script>`, a link's
destination is never *produced* — it is copied. `assemble(&[None; n])`
is the source byte for byte, BOM, CRLF, trailing whitespace and missing
final newline included; a property test checks it over 3 000 generated
documents in all four formats at three budgets, and another over every
Markdown file in this repository.

That is also why "kept" needs no code: a format parser only says where
the prose is, and whatever it does not name is kept.

### What is prose (D69)

| format | a piece is | kept byte for byte |
|---|---|---|
| Markdown | a paragraph (top level, in a quote, an item, a footnote), or the inline run of a tight list item | headings, front matter (`---`/`+++`), fenced and indented code, tables, HTML blocks, rules, link reference definitions |
| HTML | the text between two block-level tags, from its first non-blank byte to its last; inline tags, comments, entities and `<code>…</code>` inside it are protected | `<script>`, `<style>`, `<pre>`, `<textarea>`, `<title>`, `<h1>`–`<h6>`, `<svg>`, `<math>`, `<template>` whole; a tag that never closes, and everything after it |
| Plain | the lines between two blank ones | — |
| Code | nothing | everything (the `code` tactic is E4-3's) |

Markdown is `pulldown-cmark` 0.13 (`default-features = false`) with
tables, footnotes, strikethrough, task lists, both front-matter styles
and GFM alert quotes; its offset iterator is what lets every piece be a
byte range of the source. HTML is a tokenizer of ~200 lines rather than
a parser: the question is only "where is text", and a tree would answer
nothing more. A piece with no letter outside its placeholders — a
paragraph that is just a link, `1999.` — is not a chunk: nothing in it
can be rewritten, and a model call that can only fail costs a minute on a
CPU.

### Protected spans → `⟦n⟧` (D68)

What the model must not change is not shown to it. Each protected span
becomes a numbered placeholder, numbered per chunk from 1;
`Chunk::protected[k-1]` is its exact source text.

- **From the structure:** Markdown inline code, inline HTML, footnote
  references, autolinks; a link's or image's opener (`[`, `![`) and closer
  (`](url "title")`, `][ref]`) as two spans, so link text and alt text are
  rewritten and the destination is not. HTML tags and comments.
- **Lexical, in every format:** any `⟦`/`⟧` already in the document; the
  prompt's markers (`[[[…]]]`, and any run of three brackets); URLs;
  e-mail addresses; paths (absolute with two segments, `~/`, `./`, `../`,
  drive and share paths, and `dir/name.ext` — so `and/or`, `TCP/IP`,
  `km/h` are prose); entities in Markdown and HTML; backtick spans in
  plain text.
- **Never** numbers (`NumbersGuard` watches them, and a model rebuilds a
  sentence around a number better than around a placeholder) or
  quotations, and there is no user regex in v1.

The lexical scan reads the whole piece, not only the gaps between
structural spans, and overlapping spans merge: a `[[[` whose third
bracket is a link's opener must still be one span, or the context could
write the prompt's marker back together out of two halves.

`placeholder(n)` is the one place the format lives, and `placeholders_in`
its reader beside it — canonical exactly as `wipemark_core::PlaceholderGuard`
reads one (digits, no leading zero), so a candidate the guard passed is
restored as the guard counted it. The guard hard-codes `⟦n⟧` too: a bench
that tries another format changes both.

### Chunks (D70, narrowed by the owner)

**One paragraph is one chunk**; short ones are not merged. **One list is
one chunk**: its items are joined by **glue** — the bytes between two
items (newline, marker, indentation, `>`, a task box, a blank line), one
placeholder each — so a reordered or re-wrapped list comes back with its
markers exactly. The model sees each glue placeholder at the start of a
line of its own; the newline before it is presentation only, because
restore drops whitespace that touches glue. A kept block inside a list (a
code block in an item) ends the chunk there.

The budget is `min(ctx_len × 0.4, 600)` estimated tokens, 600 when the
context length is unknown. Only what is over it is split: a list between
items; a paragraph at sentence ends (`.!?…` and closers before
whitespace; `。！？` with or without it, because CJK prose has none); a
sentence between words; a word over the budget is a chunk of its own. No
cut lies inside a protected span, and the whitespace at a cut — in
Markdown with the next line's `> ` — stays outside both chunks.

### The way back

The chunk's text has Markdown's container prefixes taken off its
continuation lines (the model reads `line one\nline two`, not
`> line one\n> line two`) and `\r\n` as `\n`. `Chunk::restore` reverses
that for whatever the model wrote, in this order:

1. `\r\n` as `\n`; leading and trailing whitespace dropped.
2. Every canonical placeholder checked — a number the chunk does not have
   (`Unknown`), one that did not come back (`Missing`), one twice
   (`Duplicated`), list glue out of its order (`OutOfOrder`, since glue
   carries `2.`). Restore never guesses which span a damaged placeholder
   meant; the loop's `PlaceholderGuard` will normally have rejected the
   candidate first. In a chunk with list glue, an item that came back
   with more line breaks than it went in with (`ItemBroken`, E4-3 — see
   "One attempt" below).
3. Whitespace touching glue dropped (the glue holds the source's own).
4. Every newline written as the chunk's line ending plus the container's
   **continuation prefix** — derived from the piece's first line: `>` and
   blanks kept, a list marker or task box turned into spaces (`"> "`,
   `"  "`, `"    "` for `10. `), trailing blanks trimmed on an empty
   line. So a model that re-wrapped a quoted paragraph into four lines
   still gives back a quote, and an item's second line stays in the item.
   Within a list chunk the prefix after glue `⟦k⟧` is the prefix of the
   item that glue introduces.
5. Every `⟦k⟧` replaced by `protected[k-1]`.

An unchanged candidate restores to the source exactly whenever the
source's continuation lines carry the canonical prefix (tested); a lazy
continuation line or an unusual indentation comes back normalised, which
is the same document to every Markdown reader.

### Context (`{PREV_CONTEXT}`)

The last two sentences of the previous chunk's **source** — never of its
rewrite, so chunks stay independent and the loop can run them in any
order — with protected spans written back, glue read as a sentence break.
An original that holds a marker or a placeholder bracket is written back
as `…`: the context sits between the prompt's markers, outside what
`PlaceholderGuard` checks. Capped at a quarter of the budget (the last
sentence alone, then its tail after `…`). `None` for the first chunk.

### The language (D65)

`lang::detect` answers `Some(En | De | Ru)` or `None`, never a guess.
Script shares from `TextStats::of` pick the family (Latin or Cyrillic, ≥
75 % of the letters); short embedded stop-word lists pick the language
within it (the winner's words must be ≥ 15 % of all words, the other of
en/de under half of that). Short lists for the neighbours — French,
Spanish, Italian, Portuguese, Dutch (which also catches Afrikaans);
Ukrainian and Bulgarian, and Ukrainian's own letters `і ї є ґ` — exist only
to make a neighbour read as `None` rather than as a language whose prompt
would translate it. Under 8 words or 30 letters is `None`.
`Prepared::language` runs it over the chunks' texts with every
placeholder removed, so code and URLs do not vote for English. `None`
means E4-2's English templates plus the "do not translate" clause (D64).

### Estimating tokens

`estimate_tokens` is a per-character cost (in thousandths of a token,
summed, rounded up — so it is additive and the chunker can pack sentences
without re-estimating): ASCII letter or digit 0.31, Cyrillic 0.42, CJK
0.75, whitespace 0, anything else 1. Calibrated on 2026-10-03 against Qwen3
4B Instruct 2507's tokenizer (`llama-server … /tokenize`) so that no
paragraph is undercounted by more than 5 %:

| set | paragraphs | chars | Qwen3 tokens | estimated | min est/real | mean | max |
|---|---:|---:|---:|---:|---:|---:|---:|
| en | 12 | 2046 | 414 | 568 | 1.13 | 1.39 | 1.61 |
| ru | 12 | 1988 | 670 | 756 | 0.95 | 1.14 | 1.41 |
| de | 12 | 2285 | 628 | 668 | 0.95 | 1.06 | 1.21 |
| zh | 10 | 510 | 316 | 396 | 1.00 | 1.27 | 1.43 |
| ja | 5 | 362 | 240 | 276 | 1.00 | 1.16 | 1.32 |
| mixed (placeholders, code, a list) | 6 | 512 | 151 | 180 | 0.96 | 1.16 | 1.64 |
| all | 57 | | | | 0.95 | 1.20 | 1.64 |

English is overcounted most: one English word is often one token whatever
its length, which no per-character rule sees (a per-word rule did no
better — German's long compounds pull the other way). An overcount only
makes a chunk smaller than the budget. The paragraphs and their counts
are `prepare/calibration.rs`; a test holds the rule to them. Another
model's tokenizer may differ — the loop uses the estimate for packing, for
the price shown before a run, and doubled plus 64 as a generous *ceiling*
on an answer (`max_tokens`, see "The loop"), never as an exact limit.

### Cost

Linear in the document: a 2 MB paragraph prepares in under half a second
(release build). Two pathological shapes were quadratic before they were
fixed and are worth knowing about: a long run of backticks (a run that
opens nothing is now skipped whole) and HTML full of unclosed tags (an
unclosed tag now runs to the end of the document, as in a browser's
tokenizer, and the rest is kept). One-line spans — markers, backtick
spans — look at most 1 KiB ahead.

## The loop

E4-3 (spec S4.3, S4.5, S4.6; decisions D61, D67, D70–D73, D77, D78; the
plan document `docs/plan/E4-3-the-loop.md`).

```
crates/wipemark-pipeline/src/job/mod.rs      Options, Document, start, JobHandle, Refused, the job's thread
crates/wipemark-pipeline/src/job/plan.rs     plan → Planned: Layer A first, the budget, the usable ladder, template fallbacks, cost()
crates/wipemark-pipeline/src/job/attempt.rs  one attempt: render → complete → clean_response → Layer A → guards → restore → no-op
crates/wipemark-pipeline/src/job/drive.rs    the few lines of std::task that drive an engine's future and forward its tokens
crates/wipemark-pipeline/src/select.rs       divergence, the no-op floor, the length penalty, the scorer seam, the winner
crates/wipemark-pipeline/src/cost.rs         Executor, Effort (D61), Cost
crates/wipemark-pipeline/src/report.rs       Rejection, EngineFailure, JobReport and its records, the ASCII JSON form
crates/wipemark-pipeline/tests/live.rs       the live gate on Qwen3 4B (llama-native, #[ignore])
```

### What a job is

`start(id, Document { text, format }, Options, Arc<dyn RewriteEngine>)`
checks the options, spawns a thread and returns at once with a
`JobHandle` (cancel) and a `flume::Receiver<Event>`. On that thread:

1. **Layer A** over the document — its report is the *before* of the
   verifiable shelf. Everything after starts from the cleaned text, so a
   chunk no candidate wins comes back cleaned, never with its marks.
2. **Plan.** `prepare` the cleaned text; for each rung of the ladder,
   `templates_for` — `back_translate` over an undetected language is
   *skipped and recorded*, not fatal; every step is rendered once over a
   probe word, and an override that does not render is dropped for the
   shipped template and recorded (a shipped one that does not is a bug and
   fails the job). The chunk budget is D70's less the prompt around the
   chunk: `Budget::for_context(ctx_len − overhead)`, with `overhead` the
   largest probe render — on an 8192 window it is still 600.
3. **Nothing to ask** — a `Code` document, a document of headings and
   tables, a ladder with no usable rung — calls no engine at all.
4. `warmup`, then per chunk, round by round (one rung per round, the last
   rung repeated): every candidate of the round is an **attempt**; the
   next round runs only if **no** candidate of this one passed (D61).
5. The winner of each chunk, or its cleaned source; `assemble`; **Layer A
   again over the whole result** — the *after*.

The result is the last event, `Finished { outcome: { text, report } }`;
or `Cancelled` (no document: half a rewrite is not a result) or `Failed`.

### One attempt

```
render(step, chunk text | step 1's cleaned answer, the chunk's context, intensity)
→ into_request(seed, max_tokens) → engine.complete (tokens → Event::Token as they arrive)
→ clean_response                         (D67: think blocks, markers, one fence or quote pair; never a preface)
→ Layer A over the final answer          (what the model slipped in, removed and counted per attempt)
→ the five guards (chunk.text, answer)   (first reject wins; the length window is the options')
→ chunk.restore(answer)                  (any RestoreError is a rejection — OutOfOrder and ItemBroken are what a guard cannot see)
→ divergence < 0.05 → no-op              (D71)
→ passed: divergence, length ratio, score
```

`ItemBroken` came out of the first live run: Qwen3 4B gave a three-item
list back with every glue placeholder present and in order, but with item
2's words on a new line inside item 1 and item 2 reduced to `;`. In a
chunk with list glue, `restore` now refuses an item that comes back with
more line breaks than it went in with (edges trimmed, so the newline before
a glue placeholder — the glue's own — may be kept, dropped or doubled). A
paragraph is not checked: re-wrapped, it is still one paragraph. Words
moved across an item boundary *without* a new line break are not seen.

Layer A runs **before** the guards so that a U+200B the model put into an
identifier costs the candidate nothing: it is removed, counted, and the
identifier is whole again. Every rejection is a value
(`Rejection::{Engine, Truncated, Empty, Guard { guard, reason },
Restore, NoOp, MarkerInAnswer}`) carried by `Event::CandidateRejected`
and the report; the surface words it. A candidate that was rejected is
never used — rejected attempts carry no text.

An engine error rejects the attempt (`Transport`, `Protocol`,
`ContextOverflow`, `NotImplemented`) and the loop goes on; `Unavailable`
fails the job (asking every candidate of every chunk would repeat it);
`Cancelled` cancels it. `max_tokens`, when the options leave it unset, is
`2 × estimate_tokens(input) + 64` — the estimate never undercounts by more
than 5 %, the length guard rejects past 1.6×, and the engines clip to the
window; without a ceiling a runaway answer costs minutes on a CPU.

### Choosing

`min-divergence` (D71): the passed candidate with the lowest score wins,
score = bigram-Jaccard divergence from the chunk (words are placeholders
or runs of letters and digits, lower-cased), plus 0.15 when the length
ratio is outside 0.5–2×. With the default length guard (0.6–1.6) the
penalty cannot fire; it is live for a wider window, which is what the
options' `length` field is for (the bench, E4-5). A tie goes to the
earlier attempt. `Scorer::Divergence` is the only scorer; the keyed one
of D72 would be another arm of the same enum.

### Seeds

`base_seed + (chunk · rounds + (round − 1)) · candidates + (candidate − 1)`,
wrapping — unique across the job (OV §4.4's `base_seed + round · c`
collides at (1, 2) and (2, 1), and repeats per chunk), recorded per
attempt, and the same seed for both steps of a two-step attempt.

### Effort and price

`Effort::for_executor` is D61: 1 × 2 for a local model on the CPU alone,
2 × 2 for a GPU or an endpoint. `Planned::cost(options, tokens_per_second)`
gives calls exactly — worst (every round) and expected (every chunk passes
in round 1) — prompt tokens from the real renders, answer tokens as the
chunk's estimate per step, and seconds only from a rate the caller
measured (prompt processing not counted). `plan` is pure and may take a
while on a large document: a surface calls it off the GPUI thread.

### The report

`JobReport` — Layer A before and after; the engine (`vendor`, model,
local, window); format, language, pivot, the usable ladder, the skipped
rungs, the template fallbacks; intensity, effort, base seed, scorer; per
chunk every attempt (round, candidate, tactic, seed, per step the
`Version` of both turns, tokens out, finish, what `clean_response` took
off; Layer A over the answer; the verdict with its numbers) and the
outcome; and `not_established`, which is the baseline shelf plus "unknown
mark schemes", because Layer B searches for none. `totals()` counts
attempts made — the honest denominator. `to_json()` is one line of ASCII
(every non-ASCII character `\u`-escaped, as core's A §7.1 writer does);
`Serialize` gives the same value. Its keys are the shelves:
`verifiable`, `best_effort`, `not_established`.

### Driving an engine without a runtime

The engines' futures wait on channels their own threads feed, so the
job's thread polls the future with a waker that unparks it — no tokio, no
`futures`. While the future is pending it also polls the token channel's
`recv_async` with the same waker, so a token wakes the thread and goes out
as `Event::Token` at once, in order with the job's other events (one
thread sends them all). A dropped receiver cancels the job.

### The application's engine

`start` takes `Arc<dyn RewriteEngine>`. The application's `EngineHandle`
is not one: it has `complete` but no `info`, and an endpoint's engine is
built on first use. E4-6 wraps it in an adapter that implements the
trait — `info` from the engine on duty when the job starts, `complete`
through the handle (busy count, keep policy), `warmup` through the host's
load.

## The queue

E4-4 (spec S4.8, OV §4.5; the plan document `docs/plan/E4-4-the-queue.md`).

```
crates/wipemark-pipeline/src/job/resume.rs   Decided (one chunk's decision, ASCII JSON), the fingerprint, the chunk digest
crates/wipemark-pipeline/src/job/stored.rs   Options::to_json / from_json — a queued item's options after a restart
crates/wipemark-store/src/queue.rs           schema 2: queue, queue_chunks, queue_control — strings and integers
crates/wipemark-queue/src/lib.rs             Queue (the handle), QueueEvent, End, Failure, ItemView, Durability
crates/wipemark-queue/src/worker.rs          the queue's thread: commands, the running job, rows, delivery
crates/wipemark-queue/src/item.rs            ItemId, Source, Destination, Request, State, the item row
crates/wipemark-queue/src/read.rs            reading a file the way the CLI does
crates/wipemark-queue/src/deliver.rs         writing a result through wipemark_intake::inplace
crates/wipemark-queue/tests/kill.rs          the gate: a child process killed with SIGKILL mid-document
```

### Why a crate of its own

A queue runs jobs, remembers them, reads the user's files and writes
results. The pipeline may depend on core and engine only, the store is a
leaf, and nothing may depend on an application — so `wipemark-queue` sits
above the pipeline, the store and intake and is the one place they meet
(`scripts/check-dep-direction.sh`). The pipeline gained values, not
storage: a resumable entry point and a per-chunk record. The store gained
tables of JSON it does not read.

### A job that can be taken up again

`start_resumable(id, document, options, engine, carried)` is `start` plus
two things. Every chunk it decides is handed out at once as
`Event::ChunkDecided { decided: Decided }` — the chunk's index, the job's
fingerprint, the chunk's digest, the outcome, the winner (placeholders
and all) or none, the report's JSON of its attempts and their counts.
And `carried`, an earlier run's records, are taken back: after planning,
`Event::Resumed { carried, discarded }`, and a chunk with a usable record
is not asked again — its winner goes into the assembly as decided and its
report is the record's, `"carried_over": true` with the attempts as they
were recorded (core's `CleanReport` has a writer and no reader, so a
carried attempt stays JSON; `ChunkReport::carried`, `totals()` counts it).
`start` itself is unchanged: no new event, no new JSON key.

A record is used only when **all** of these hold, and is otherwise
discarded — never repaired:

- its fingerprint is the job's: sha256 over a version tag, the document's
  text and format, the options' `Debug` (every field and override), the
  engine's identity, the chunk budget and the usable rungs with their
  templates. So a source edited between a crash and the restart, other
  options, another model or a new build's template invalidates every
  record of the job — half a document rewritten under one set of rules
  and half under another would be a report that lies;
- it names a chunk of this plan and carries that chunk's digest (text,
  context, protected spans);
- its outcome and winner agree, and the winner still restores into the
  chunk.

This works because chunks are independent (a chunk's context is the
source's previous sentences), `Prepared` is deterministic for the same
text, format and budget, and an attempt's seed depends only on the
chunk's index (D83): the chunks a resumed job asks produce what an
uninterrupted job would have, and the final text is the same.

### The queue

`Queue::open(path, engine)` — or `Queue::on(store, durability, engine)`
for the application's own store — loads the rows and starts the queue's
thread. The handle (`push`, `pause`, `resume`, `cancel`, `remove`,
`items`, `events`, `result`, `shutdown`) sends commands and returns: every
statement, file read and write is on that thread. The constructors and
`result` read rows, so a window calls them off its foreground thread.

- **One at a time, oldest first.** Ids come from `AUTOINCREMENT` and are
  never reused; the handle allocates them so `push` returns at once.
- **Rows.** `queue (id, state, item, result)`: the state is the queue's
  word — `queued`, `running`, `delivering`, `done`, `failed`,
  `cancelled`; `item` is the request (the source path, or the text with
  no file behind it; the format; the destination; the options as
  `Options::to_json` wrote them); `result` the end (the report's JSON, what
  was written, the failure as ids). `queue_chunks (item, idx, record)` is
  one `Decided` per chunk, written the moment its event arrives — the
  whole of surviving a `kill -9`; they go when the item ends.
  `queue_control` is whether the queue is paused.
- **A restart.** `running` becomes `queued` and keeps its chunk rows;
  starting it hands them to `start_resumable`. `delivering` is finished
  first.
- **Pause** is persisted; the running job is cancelled (E4-3: "a paused
  job is a cancelled one to be started again") and its item waits again
  with its chunks. **Cancel** ends an item as `cancelled`, a waiting one
  never starts. **Remove** deletes the row. **Shutdown** leaves the
  running item waiting for the next open.
- **Progress.** `QueueEvent::Job { item, event }` forwards every pipeline
  event of the running item; `ItemView::decided` counts its decided chunks.
- **A source that cannot be read** ends its item `failed` with the reason
  (missing, folder, unreadable, not text, an unnamed eight-bit encoding,
  invalid at a byte) and the next one starts. A file is read like the CLI
  reads it — the head through `wipemark_intake::identify`, refused before
  the rest is read, then `wipemark_intake::text::decode` (moved there from
  the CLI, with `encode` and `inplace::same_file`).
- **A database that will not open** is left on disk byte for byte; the
  queue runs on an in-memory store and `Durability::Memory` says it will
  not survive a restart. A failed write later is `QueueEvent::Unsaved`,
  and the queue goes on.

### Where a result goes

The destination is chosen when the item is pushed, stored with it, and
executed as stored — the queue reads no Retention row; the surface that
pushes does (E4-6, E7):

- `Row` — nothing written; the text stays in the row until the item is
  removed. The only destination text with no file behind it has short of a
  path somebody chose; `InPlace` for text is refused at push.
- `File(path)` — `Destination::beside(source)` is `name.cleaned.ext` by
  `with_infix`; or a folder's path. Written by
  `inplace::write_atomically`, in the source's encoding with its byte
  order mark and the source's permissions. A destination that *is* the
  source is refused at push (the same path) and at delivery (the same
  inode); the result then stays in the row.
- `InPlace(Keep)` — the per-run flag: `inplace::replace`, the original set
  aside first unless `Keep::Nothing`, never over an original already there;
  nothing changed, nothing touched.

Delivery is two-phase: the result goes into the row as `delivering`, then
the file is written, then the row is `done` (and loses the text when the
text is on disk). The next open finishes a `delivering` item: a file is
written again; an in-place item whose original is already aside is done
when the file is missing or already holds the result, and fails with
"original exists" when it holds anything else.

### What it keeps

The row holds a paste's text until the item is removed, and the result's
text when it has nowhere else to be. Chunk rows go when the item ends.
Every connection runs with `PRAGMA secure_delete = ON`, so a removed row's
text is overwritten in the database file; the WAL may hold a copy until
its next checkpoint, which is SQLite's and not something this product can
promise away. Log lines carry ids, counts, states and failure kinds; a
document only as `wipemark_log::Elided`'s shape
(`no_document_text_or_path_reaches_a_log_line`).
