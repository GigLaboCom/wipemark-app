# The pipeline

What happens between "the user pressed Rewrite" and "here is the
document", in `crates/wipemark-pipeline`. Epic **E4**, built as a series
(README §7 E4, decision D76): preparing the text (E4-1), the prompts
(E4-2), the loop that joins them (E4-3), the queue (E4-4), the bench
(E4-5), the surfaces (E4-6). Nothing rewrites a document until E4-3 lands;
this page grows a section per step.

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
   candidate first.
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
model's tokenizer may differ — the loop uses the estimate for packing and
for the price shown before a run, never as a limit sent to an engine.

### Cost

Linear in the document: a 2 MB paragraph prepares in under half a second
(release build). Two pathological shapes were quadratic before they were
fixed and are worth knowing about: a long run of backticks (a run that
opens nothing is now skipped whole) and HTML full of unclosed tags (an
unclosed tag now runs to the end of the document, as in a browser's
tokenizer, and the rest is kept). One-line spans — markers, backtick
spans — look at most 1 KiB ahead.
