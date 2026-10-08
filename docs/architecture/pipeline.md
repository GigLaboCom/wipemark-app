# The pipeline

What happens between "the user pressed Rewrite" and "here is the
document", in `crates/wipemark-pipeline`. Epic **E4**, built as a series
(README §7 E4, decision D76): preparing the text (E4-1), the prompts
(E4-2), the loop that joins them (E4-3), the queue (E4-4), the bench
(E4-5), the surfaces (E4-6). Since E4-3 a job rewrites a document; since
E4-6a the MCP tool `rewrite` and `wipemark-cli rewrite` start one (the
windows are E4-6b and E7). This page grows a section per step.

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

**One paragraph is one chunk**; short ones are not merged. **One list
item is one chunk** too (E4-7, D95): its marker, its indentation, a task
box and the blank line before it are the bytes *between* two chunks, so
they are copied from the source and no model ever sees one. Until E4-7 a
list was one chunk whose items were joined by "glue" placeholders at the
start of each line; those were the placeholders models dropped (taking
them for list markers), and the re-split items behind `ItemBroken` — the
bench found lists most of the paragraphs that came back unchanged. A
paragraph inside an item (a loose item's second paragraph, a quote in an
item) is a chunk of its own like any other.

The budget is `min(ctx_len × 0.4, 600)` estimated tokens, 600 when the
context length is unknown. Only what is over it is split: a paragraph (or
an item) at sentence ends (`.!?…` and closers before
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
   (`Duplicated`). Restore never guesses which span a damaged placeholder
   meant; the loop's `PlaceholderGuard` will normally have rejected the
   candidate first. A chunk that is a **list item** may not come back with
   more line breaks than it went in with (`ItemBroken`, E4-3, per item
   since E4-7 — see "One attempt" below).
3. Every newline written as the chunk's line ending plus the container's
   **continuation prefix** — derived from the piece's first line: `>` and
   blanks kept, a list marker or task box turned into spaces (`"> "`,
   `"  "`, `"    "` for `10. `), trailing blanks trimmed on an empty
   line. So a model that re-wrapped a quoted paragraph into four lines
   still gives back a quote, and an item's second line stays in the item.
4. Every `⟦k⟧` replaced by `protected[k-1]`.

An unchanged candidate restores to the source exactly whenever the
source's continuation lines carry the canonical prefix (tested); a lazy
continuation line or an unusual indentation comes back normalised, which
is the same document to every Markdown reader.

### Context (`{PREV_CONTEXT}`)

The last two sentences of the previous chunk's **source** — never of its
rewrite, so chunks stay independent and the loop can run them in any
order — with protected spans written back. For a list item that is the
item before it (or, for the first, the paragraph that leads into the
list).
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
crates/wipemark-pipeline/src/job/attempt.rs  one attempt: render → complete → clean_response → Layer A → verdict (guards → language → restore → no-op)
crates/wipemark-pipeline/src/job/drive.rs    the few lines of std::task that drive an engine's future and forward its tokens
crates/wipemark-pipeline/src/select.rs       divergence, the no-op floor, the length windows, RULES, the scorer seam, the winner
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
→ job::verdict:
    the five guards (chunk.text, answer) (first reject wins; the length window is the chunk's — below)
  → the language check                  (D95: 20+ words, not in the chunk's language → Rejection::Language)
  → chunk.restore(answer)               (any RestoreError is a rejection — ItemBroken is what a guard cannot see)
  → divergence < 0.2 → no-op            (D95; 0.05 until E4-7)
→ passed: divergence, length ratio, score
```

`job::verdict` is public: the prompt bench judges its answers with it,
so the bench's verdicts are the loop's by construction (and `bench
verify` checks it on a live model).

**The length window depends on the chunk** (D95): a chunk of 20 words or
more (runs of letters and digits outside the placeholders) is held to
0.6–1.6 of its length, a shorter one to 0.5–2.0 — a lead-in line that
grows from four words to seven has not lost its meaning, a paragraph that
grows by two thirds has, two times in three by the bench's judge.
`Options::length` holds both windows; stored options (version 2) carry
both, and a version 1 row's one window is read as both.

**The language check** (D95). When the chunk's language can be told
(`lang::detect` over its text, placeholders removed) and the answer has
20 words or more, an answer detected as **another language or as none**
is rejected — `Rejection::Language { expected, found }`. *None* counts:
French, Spanish or Ukrainian read as unknown by design, and the bench's
planted "translate this into French" answers that passed every guard (32
of them) were all of that kind; `ScriptGuard` cannot see French for
English. A chunk whose language cannot be told is never checked, and
neither is an answer under 20 words. Only the **final** answer is
checked, against the chunk's language: that is what every tactic's last
step must produce — `back_translate`'s step 1 answers in the pivot on
purpose and never reaches the document. A paragraph in another language
than its document is therefore refused when `back_translate` returns it
in the document's language: that is a translation. Cost on the bench:
0.3–0.4 % of the other passed answers.

`ItemBroken` came out of E4-3's first live run: Qwen3 4B gave a three-item
list back with every glue placeholder present and in order, but with item
2's words on a new line inside item 1 and item 2 reduced to `;`. Since
E4-7 an item is its own chunk and nothing can move across items; what
stays refused is an item that comes back with more line breaks than it
went in with (edges trimmed) — a new line is laid under the marker's
indentation, so a `- ` the model put at its start becomes a nested list
and a blank line makes the whole list loose. A paragraph outside a list is
not checked: re-wrapped, it is still one paragraph.

Layer A runs **before** the guards so that a U+200B the model put into an
identifier costs the candidate nothing: it is removed, counted, and the
identifier is whole again. Every rejection is a value
(`Rejection::{Engine, Truncated, Empty, Guard { guard, reason },
Language { expected, found }, Restore, NoOp, MarkerInAnswer}`) carried by `Event::CandidateRejected`
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

**The most diverged wins** (D95, moving D71's `min-divergence`): of the
candidates that passed, the one with the highest score — the
bigram-Jaccard divergence from the chunk (words are placeholders or runs
of letters and digits, lower-cased) — is the one the user gets. Every
candidate that passed kept every fact, number, name, protected span and
the chunk's language, so the furthest from the original carries the least
of its wording over: on the bench, 5–10 points fewer of the original's
word pairs at the same time and with no more meaning judged lost. Its
cost is fluency, sometimes — the most-changed rewrite is now and then the
most rearranged. A tie goes to the earlier attempt. There is no length
penalty any more (D71's 0.15 outside 0.5–2× could never fire behind the
guard). `Scorer::Divergence` is the only scorer; the keyed one of D72
would be another arm of the same enum.

`select::RULES` is every rule a verdict and a winner depend on besides
the options — the selection, the floor, the 20-word boundary of the
length windows and of the language check — and the report says which
selection and floor it ran under (`best_effort.selection`, report
version 2).

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
is not one — an endpoint's engine is built on first use — so E4-6a gives
it `for_job()`, which hands back a `JobEngine`: the engine on duty, with
`info` taken once when the job takes it, `complete` and `warmup` the
engine's, `unload` nothing (the host's policy decides), and the job
counted busy and announced to the host for its **whole** length — so the
keep policy defers an unload or a swap between two candidates of one job,
not only during a call (`a_job_holds_the_model_between_its_calls`).

### The surfaces without a window (E4-6a)

`wipemark_pipeline::asked` is what the MCP tool and the CLI share: the
arguments (`Asked`), what is offered without a window (`paraphrase`,
`humanize`, `back_translate` — `structural` needs a confirmation, `code` is
not built), the counts (1–8, absent = D61's by executor), and the base
seed — fresh per job unless named, 32 bits (D83). `prompt::row` reads the
saved template rows (`overrides_from`, the row rule) and lays a caller's
own over them strictly (`lay_over`: an unknown row, a value that is no
template and an error-severity problem each refuse, by key and rule id);
`PIVOT_KEY` is `rewrite.pivot`. `wait` reads a job's events to its end on
the caller's thread with a tick for looking at a clock or a client, and
`block_on` drives one future there. `Cost::to_value` is the price as
JSON, `seconds` `null` without a measured rate.

## The queue

E4-4 (spec S4.8, OV §4.5; the plan document `docs/plan/E4-4-the-queue.md`).
Since E4-6b the application runs it: see
[queue.md](queue.md#rewriting-e4-6b) — the engine is asked for when each
item starts (`EngineSource`, D310), nothing on duty holds rather than fails
(D311), `subscribe()` gives a second reader every event, and
`Destination::New` never replaces a file already there (D319).

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

- its fingerprint is the job's: sha256 over a version tag
  (`wipemark-resume/2` since E4-7), `select::RULES` (so a build that moves
  the selection, the floor or a threshold forgets every record without
  anyone remembering to bump the tag), the document's text and format, the options' `Debug` (every field and override), the
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
`paused`, `held`, `asking` and `states` answer from memory — the thread
keeps them as it changes them — so a window may ask on every frame (D359).

- **Reserve, then push** (D358). `reserve` checks a request and hands out
  its id without pushing it; `push_reserved` pushes it under that id. A
  surface that writes its own row naming the item does so between the two,
  so nothing the item says can end before the row names it. `push` is the
  two at once.
- **Consent** (D361). `push_reserved` takes where the pusher agreed the
  document may go (`Whereto::Here` or `Whereto::Away(origin)`; `None` for a
  caller's item), stored beside the request as the item row's `consent`
  key. Before an item with one starts, the queue asks its `EngineSource`
  where an engine handed out now would send it (`EngineSource::whereto`,
  `None` by default — never asks); an endpoint other than the consented one
  holds the queue with `QueueEvent::Ask`, once per question, until
  `agree(now)`. A duty change (`engine_changed`), Resume, or the item's
  cancel or removal drops the question, and the next item is asked afresh.
- **Removed is an end for whoever waits** (D355). `QueueEvent::Removed`
  is the last word of an item taken away before it ended; the MCP call
  that waits for one answers on it.

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
when the file is missing (set aside by a rename), when it is still the
set-aside (set aside by a hard link, D284: one inode under two names, or,
where the inode cannot be seen, the same bytes — D286), or when it already
holds the result. It fails with "original exists" only when the file holds
anything else — someone's save of exactly as many bytes included: the
comparison is of the bytes, never of their length.

### What it keeps

The row holds a paste's text until the item is removed, and the result's
text when it has nowhere else to be. Chunk rows go when the item ends.
Every connection runs with `PRAGMA secure_delete = ON`, so a removed row's
text is overwritten in the database file; the WAL may hold a copy until
its next checkpoint, which is SQLite's and not something this product can
promise away. Log lines carry ids, counts, states and failure kinds; a
document only as `wipemark_log::Elided`'s shape
(`no_document_text_or_path_reaches_a_log_line`).
