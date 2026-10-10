# E4-1 — Preparing the text: formats, protected spans, chunks, language

|                  |                                                                                                                                       |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic E4, the pipeline (`E4-1` … `E4-6`, README §7 E4)                                                                  |
| Spec scopes      | S4.1 (format parsing, protected spans → `⟦n⟧`), S4.2 (chunking with the previous chunk's context); OV §4.2                            |
| Decisions        | D64 (the shared `Lang`), D65 (language detection), D68 (protected spans), D69 (what is text), D70 (chunks), D76 (the series); the owner's "rewrite by paragraph" (2026-10-03) |
| Depends on       | E1 (`wipemark_core::TextStats`, `PlaceholderGuard`); the coordinator's `wipemark_pipeline::lang::Lang`                                  |
| Runs beside      | E4-2 (the prompts), in another worktree; the only shared type is `Lang`                                                               |
| Unblocks         | E4-3 (the loop), which feeds `Chunk::text`/`context` to E4-2's assembler and `Prepared::assemble` with what survived                  |
| Files touched    | `crates/wipemark-pipeline/src/prepare/**` (new), `crates/wipemark-pipeline/src/lang.rs` (adds `detect`), one `pub mod prepare;` line in `crates/wipemark-pipeline/src/lib.rs`, `crates/wipemark-pipeline/Cargo.toml` (`pulldown-cmark`), `Cargo.lock`, `docs/architecture/pipeline.md` (new), one row in `docs/README.md`, this document, its report |
| Size             | ~2 days for one agent; no window, no network; one llama.cpp server for the token calibration                                          |

## §0 Ground rules

### 0.1 Start here

You are an implementer agent working in a **worktree** of
`/home/denis/denis-ubuntu/sources/wipemark-app` (GitHub
`GigLaboCom/wipemark-app`), a Rust + GPUI desktop application that strips
AI-provenance marks from its owner's own text. Read this document, then
`CLAUDE.md` at the repository root in full — if the two disagree,
`CLAUDE.md` wins and you say so in your report.

```sh
export GIT_CONFIG_NOSYSTEM=1         # /etc/gitconfig is unreadable on this machine
cd /home/denis/denis-ubuntu/sources/wipemark-e4-1   # the worktree for this document
git switch e4/prepare                # this worktree's branch; never main
git status                           # clean apart from ` m vendor/gpui-component`
git submodule update --init --recursive
scripts/pin-gpui-component.sh        # idempotent
```

One commit, message `E4-1: Preparing the text — formats, protected spans,
chunks, language`, ending with the co-author line the prompt gives you.
Never push, never touch `main`. Leave ` m vendor/gpui-component`
unstaged; never edit `vendor/`.

```sh
S=/tmp/claude-1000/-home-denis-denis-ubuntu-sources-wipemark-app/3f38a71d-77bc-40e9-94ee-ec30399f699c/scratchpad
mkdir -p $S/lib && ln -sf /usr/lib/x86_64-linux-gnu/libxkbcommon-x11.so.0 $S/lib/libxkbcommon-x11.so
export LIBRARY_PATH=$S/lib CARGO_TARGET_DIR=$S/target-e4-1
```

Everything temporary goes under `$S`, never `/tmp` directly.

### 0.2 Where code goes

```
core ← engine ← pipeline ← app / cli          (scripts/check-dep-direction.sh)
```

- Everything is in `crates/wipemark-pipeline`: a new module tree
  `src/prepare/` and one new function, `lang::detect`.
- The pipeline may depend on `wipemark-core` and `wipemark-engine` only.
  It does **not** depend on `wipemark-intake`, so it has its own
  `TextFormat`; mapping `wipemark_intake::Format` onto it is the
  application's job in E4-6.
- **Not this document's:** `src/prompt*` and everything about templates
  (E4-2), the non-origin code in `lib.rs` (E4-2 removes it), the loop,
  guard orchestration and events (E4-3), `wipemark-core`, `apps/`,
  `crates/wipemark-engine`, `CLAUDE.md`, and `docs/plan/README.md` (the
  coordinator flips the statuses at merge; the report says what it should
  say). The only edit to `lib.rs` is the `pub mod prepare;` line.
- `Lang`'s existing items are a contract with E4-2 and do not change;
  `detect` is added beside them.

### 0.3 Rules of this repository that bind this document

- **The document comes back byte for byte.** Everything that is not inside
  a chunk's range is reassembled from the source's own bytes;
  `assemble(&[None; n]) == source` for every format and every input.
- **`wipemark-core` is read, never edited.** `PlaceholderGuard` hard-codes
  `⟦n⟧`; the chunk texts must pass it unchanged.
- **Only applications localize.** Nothing here is prose a person reads;
  errors are values.
- **Unknown is never rendered as a guess** (CLAUDE.md "`None` means
  unknown"): `detect` answers `None` below its margins.
- **Nothing blocks the GPUI thread** does not bind a pure function, but
  the work is linear in the text so the loop can call it on any thread.

### 0.4 Tests

- RED first. Names are behaviour sentences in snake_case.
- The central invariant is tested property-style without a new
  dependency: a deterministic generator (an LCG over a vocabulary of
  awkward pieces — CRLF, BOM, `>`, list markers, backticks, brackets,
  URLs, tags, entities, `⟦`, `[[[`, CJK, trailing whitespace) produces
  thousands of inputs, and every one of them, in every format, at three
  budgets, must reassemble exactly and produce chunk texts the guard
  accepts.
- Every protection in §5 has a mutation that turned its test red,
  recorded in the report. A test that stays green with its subject
  deleted is removed.
- No fixture files: the inputs are in the tests. (`fixtures/text/` is
  walked by `wipemark-core`'s `every_fixture_is_asserted`; a pipeline
  fixture there would be an unasserted file to that test.)

### 0.5 Gates — all green before the commit

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
```

`pulldown-cmark` moves `Cargo.lock`: record it once with `cargo check -p
wipemark-pipeline` without `--locked`, commit the lock with the manifest.
While iterating, `-p wipemark-pipeline`.

### 0.6 Do not

- add any dependency but `pulldown-cmark` (no `whatlang`, no `regex`, no
  `proptest`, no HTML parser);
- touch E4-2's files or the loop's concerns (prompts, markers' assembly,
  guards orchestration, events);
- protect numbers or quotations, or take a user regex (D68);
- launch the application — this step has no window.

### 0.7 Definition of done

1. All gates of §0.5 green; every mutation of §5 recorded red.
2. Every acceptance criterion of §6 ticked with evidence.
3. `docs/architecture/pipeline.md` (new) has a section "Preparing the
   text".
4. The report at `docs/plan/reports/E4-1-<YYYY-MM-DD>.md`, with the
   token-estimate calibration table and what README §4/§7 should say.

---

## §1 Goal

Turn a document into the pieces a model may rewrite and back again: the
prose of a Markdown, HTML or plain-text document as chunks of one
paragraph each (a list as one chunk), with code, URLs, paths, markup and
anything that looks like the prompt's own markers replaced by numbered
placeholders the model is told to keep; the previous chunk's last two
sentences as context; the document's language, or an honest *unknown*;
and the reassembly that puts what the model wrote back between the
source's own untouched bytes.

## §2 Read first

- `CLAUDE.md` — whole.
- `docs/plan/README.md` §4 D60–D76 (especially D64, D65, D68, D69, D70,
  D76) and §7 E4.
- `ssd-docs/wipemark-e4-prompts-open-questions-2026-10-03.md` (main tree
  only, gitignored) §1, §2 Q-B3, Q-B6, Q-B9–Q-B12; `ssd-docs/OV.md` §4.2.
- `docs/sdd/layer-b-rewrite-reference.md` §3 `chunk`, §7.
- `crates/wipemark-pipeline/src/{lib.rs,lang.rs}`;
  `crates/wipemark-core/src/{lib.rs,report.rs,stats.rs,guard.rs}`
  (`TextStats::of`, `PlaceholderGuard` and its `placeholders`);
  `crates/wipemark-intake/src/format.rs` (`Format`, which the pipeline
  may not import).

## §3 What is true today

- `wipemark-pipeline` holds the vocabulary (`JobId`, `Action`, `Stage`,
  `Event`, `PipelineError`), the non-origin rule E4-2 removes, and `Lang`
  (`En`, `De`, `Ru`, `ALL`, `as_str`, `parse`).
- `wipemark_core::TextStats::of` gives letter shares (`latin_ratio`,
  `cyrillic_ratio`, `cjk_ratio`) from the crate's own Unicode 18.0.0
  tables; it does not expose a letter count.
- `PlaceholderGuard` counts canonical `⟦n⟧` (digits, no leading zero) in
  the source and the candidate and rejects a missing, duplicated or
  invented one; it does not check order.
- No Markdown parser is in the workspace's lock.

## §4 Deliverables

### 4.1 The public API (the contract — exactly this)

```rust
// crates/wipemark-pipeline/src/prepare/
pub enum TextFormat { Plain, Markdown, Html, Code }
#[derive(Clone, Copy)] pub struct Budget { pub max_tokens: u32 }
impl Budget { pub fn for_context(ctx_len: Option<u32>) -> Budget } // min(ctx×0.4, 600); None → 600 (D70)
pub fn prepare(text: &str, format: TextFormat, budget: Budget) -> Prepared;
pub struct Prepared { /* private */ }
impl Prepared {
    pub fn source(&self) -> &str;
    pub fn chunks(&self) -> &[Chunk];
    pub fn language(&self) -> Option<Lang>;   // lang::detect over the chunks' prose, placeholders removed
    /// rewritten[i] = Some(candidate for chunk i, still with ⟦n⟧) or None (keep the source of that chunk).
    pub fn assemble(&self, rewritten: &[Option<&str>]) -> Result<String, AssembleError>;
}
pub struct Chunk {
    pub index: usize,
    pub range: std::ops::Range<usize>,  // byte range of the source this chunk replaces
    pub text: String,                   // what the model sees, protected spans as ⟦n⟧
    pub protected: Vec<String>,         // protected[k] = original text of ⟦k+1⟧
    pub context: Option<String>,        // {PREV_CONTEXT}
    pub est_tokens: u32,
}
impl Chunk { pub fn has_protected(&self) -> bool; pub fn restore(&self, candidate: &str) -> Result<String, RestoreError>; }
pub fn placeholder(n: usize) -> String;     // "⟦n⟧" — the ONE place the format lives (D68)
pub fn estimate_tokens(text: &str) -> u32;
// lang.rs
pub fn detect(text: &str) -> Option<Lang>;  // D65
```

Additions this document makes beyond the contract, all additive:
`RestoreError` and `AssembleError` (values, `thiserror`), private fields
on `Chunk` (the reassembly layout), `Budget::DEFAULT`, and
`TextFormat`'s derives (`Debug, Clone, Copy, PartialEq, Eq, Hash`).

### 4.2 What is text, per format (D69)

The output of parsing is a list of **pieces** — byte ranges of the
source holding prose, each with its protected spans — grouped into
**units**: a unit is one paragraph, or the items of one list. Everything
else is kept, which needs no code: it is simply never inside a chunk.

- **Markdown**, `pulldown-cmark` 0.13 with `default-features = false`
  and its offset iterator, options: tables, footnotes, strikethrough,
  task lists, YAML (`---`) and TOML (`+++`) front matter, GFM
  block-quote kinds. A piece is a `Paragraph`'s range, or the inline run
  directly inside a tight list `Item`, trimmed of whitespace. **Kept:**
  headings (ATX and setext), front matter, fenced and indented code
  blocks, tables, HTML blocks, thematic breaks, link reference
  definitions (they produce no event). A paragraph inside a block quote,
  a list item or a footnote definition is a piece like any other.
- **Lists.** Every piece under one outermost `List` is one unit; a kept
  block inside the list (a code block in an item, a heading, a table, an
  HTML block, a rule) ends the unit and the pieces after it start
  another (D70: "a kept block ends a chunk"). The bytes between two
  consecutive items of one chunk — newline, marker, indentation, `>`,
  a task box, a blank line, a nested list's marker — are one **glue**
  placeholder each, so the list comes back exactly.
- **HTML**, a tokenizer of our own: tags, comments, `<!DOCTYPE>` and
  `<?…?>` are recognised (quoted attribute values may hold `>`); a block
  tag (`p`, `div`, `li`, `td`, `br` is not one, …) ends a piece; a piece
  runs from its first non-blank text byte to its last, and the inline
  tags, comments and entities inside it are protected. **Kept whole:**
  `<script>`, `<style>`, `<pre>`, `<textarea>`, `<title>`, `<h1>`–`<h6>`,
  `<svg>`, `<math>`, `<template>` — from the start tag to the matching
  end tag, or to the end of the document if there is none. A tag that
  never closes (no `>` outside quotes before the end) runs to the end of
  the document, as in a browser's tokenizer, and everything after it is
  kept: what could not be read is never rewritten, and one scan to the end
  instead of one per `<` keeps a page of unclosed tags linear. `<code>…</code>`
  inside text is one protected span (kept whole, and the sentence around
  it still reads). HTML list items are separate pieces (the glue rule is
  Markdown's).
- **Plain:** paragraphs between blank lines (a line of only whitespace
  is blank), each trimmed.
- **Code:** nothing is a piece; the whole document is one kept region
  (the `code` tactic is E4-3's business).
- **A leading BOM** is outside every piece in every format.
- **A piece with no letter outside its placeholders is not a chunk**
  (a paragraph that is only a link, an image or `1999.`): there is
  nothing to rewrite, and a call that can only fail costs minutes on a
  CPU.

### 4.3 Protected spans → `⟦n⟧` (D68)

Numbered **per chunk from 1** in order of appearance;
`protected[k-1]` is the exact source text of `⟦k⟧`; spans that touch are
merged into one placeholder (`[![` is one, not two).

- **From the Markdown structure:** inline code (backticks included);
  inline HTML; footnote references; autolinks and e-mail autolinks whole;
  a link's or image's **opener** (`[`, `![`) and **closer** (`](dest
  "title")`, `][ref]`, `]`) as two spans, so the link text and the image
  alt text are rewritten and the destination is not; a link with no text
  of its own is one span.
- **From the HTML tokenizer:** every tag, comment and `<code>` element
  inside a piece.
- **Lexical, in every format, over the whole piece** (not only the gaps
  between structural spans — a `[[[` whose third bracket is a link's
  opener must still be one span); overlapping spans merge:
  - any `⟦` or `⟧` in the document (`⟦12⟧` as one span);
  - the assembler's markers: `[[[…]]]` on one line, and any run of three
    or more `[` or `]`;
  - URLs: `scheme://…`, `www.…`, `mailto:…`, up to whitespace or `<>"`
    and a backtick, with trailing `.,;:!?'"*_` and unbalanced closing
    brackets trimmed;
  - e-mail addresses (ASCII local part and dotted domain);
  - paths: `/a/b…` (two segments at least), `~/…`, `./…`, `../…`,
    `C:\…`, `\\server\share…`, and a relative `seg/…/name.ext` whose
    extension holds a letter (so `and/or`, `TCP/IP`, `km/h` and
    `2024/01/02` are prose);
  - HTML entities (`&amp;`, `&#8212;`, `&#x2014;`) in Markdown and HTML;
  - backtick spans on one line in **Plain** text (chat text carries them).
- **Never:** numbers, quotations, a user regex (D68).
- The format is `placeholder(n)`; the parser of a placeholder sits next
  to it and shares its two brackets. E4-5 can try another format by
  changing that function — and `wipemark_core::PlaceholderGuard`, which
  hard-codes `⟦n⟧` and is E1's.

### 4.4 Chunks and the budget (D70, as the owner narrowed it)

- **One paragraph is one chunk.** Short paragraphs are **not** merged —
  the owner's "rewrite by paragraph" (2026-10-03) narrows D70's "one or
  more consecutive paragraphs"; the report asks the coordinator to
  restate D70.
- **A list is one chunk**, split across chunks only when it is over the
  budget, greedily at item boundaries; the glue at a split stays source
  bytes outside both chunks. In the chunk's text each glue placeholder
  starts a line of its own (`item one\n⟦1⟧item two`): the newline before
  it is presentation for the model only — the glue holds the source's
  newline, and restore drops whitespace touching glue.
- `Budget::for_context(Some(n))` is `min(n × 0.4, 600)`, `None` → 600.
- A piece over the budget splits at **sentence ends** — `.`, `!`, `?`,
  `…` (optionally followed by closing quotes or brackets) followed by
  whitespace; `。`, `！`, `？` (and fullwidth closers) end a sentence
  **with or without** whitespace after them, because CJK prose has none
  (the prompt's "followed by whitespace" read literally would leave a
  Chinese paragraph unsplittable). Sentences are packed greedily up to
  the budget. A sentence over the budget splits at whitespace, words
  packed greedily. A word over the budget is a chunk of its own, over
  budget — the only way a chunk exceeds it by more than the digits of
  its placeholders (packing prices each piece with its own numbering
  from 1, and a glue placeholder at two digits).
- **Never inside a placeholder:** no cut point lies inside a protected
  span (a `. ` inside inline code is not a sentence end).
- The whitespace at a cut — and, in Markdown, the container prefix of
  the next line — stays source bytes outside the chunks.
- `est_tokens` is `estimate_tokens(&chunk.text)`; packing sums the same
  per-character costs, so the estimate is additive.

### 4.5 The text the model sees, and the way back

- `Chunk::text` is the source of the chunk's range with protected spans
  replaced by placeholders, `\r\n` folded to `\n`, and — in Markdown — the
  container prefix of every continuation line (`[ \t>]*` after a newline
  outside a protected span) removed, so the model reads prose, not
  `> `/indentation.
- `Chunk::restore(candidate)`:
  1. the candidate's `\r\n` become `\n`; its leading and trailing
     whitespace is dropped (the chunk's text never has any; the bytes
     around a chunk are the source's);
  2. every canonical `⟦k⟧` (the guard's definition: digits, no leading
     zero) is checked: `k` outside `1..=n` → `RestoreError::Unknown`;
     a `k` that does not occur → `Missing`; more than once →
     `Duplicated`; glue placeholders out of their order →
     `OutOfOrder` (a list whose items came back reordered across their
     glue would carry the wrong numbers). Restore never guesses;
  3. whitespace touching a **glue** placeholder is dropped — the glue
     holds all the whitespace between two items, exactly as the source
     had it;
  4. every newline outside a placeholder becomes the chunk's newline
     (`\r\n` when the source piece used it) followed by the **container's
     continuation prefix** — derived from the piece's first line: the
     bytes from the line start to the piece, with `>` and blanks kept and
     every other character (list markers, a task box) turned into a space
     (`"> "`, `"  "`, `">   "`, `"   "` for `1. `); a blank line gets the
     prefix with its trailing blanks trimmed. Within a list chunk, the
     segment after glue `⟦k⟧` takes the prefix of the item that glue
     introduces;
  5. every `⟦k⟧` becomes `protected[k-1]` exactly.
- `restore(&chunk.text)` is the source range exactly whenever the source's
  continuation lines carry the canonical prefix and one newline style
  (tested); otherwise it is the same document with the prefixes
  normalised.
- `Prepared::assemble(rewritten)`: `rewritten.len()` must equal the
  number of chunks (`AssembleError::Count`); a `Some` is restored (a
  failure is `AssembleError::Restore { chunk, error }`), a `None` is the
  source's own bytes; every byte between chunks is the source's.

### 4.6 Context (`{PREV_CONTEXT}`, D70)

The previous chunk's **source** (its text with placeholders written
back — computed in `prepare`, before any rewrite exists, so a chunk's
context never depends on another chunk's result), its last two sentences, by the same sentence ends, with a glue
placeholder read as a sentence break (an item without a full stop is
still a unit). A protected original that contains `[[[`, `]]]`, `⟦` or `⟧`
is written back as `…` — the context sits between the assembler's
markers and must not carry a marker or a placeholder the guard would then
miss. Capped at a quarter of the budget: past it the last sentence alone,
and past that its tail at a word boundary after `…`. The first chunk's
context is `None`; so is a context with no letter.

### 4.7 The language (D65)

`lang::detect(text)`:

1. Words are maximal runs of alphabetic characters, lower-cased. Fewer
   than **8 words** or **30 letters** → `None` (short text is unknown).
2. Script from `TextStats::of`: Cyrillic share ≥ 0.75 → the Russian
   test; Latin share ≥ 0.75 → the English/German test; anything else
   (mixed, CJK, Greek, Arabic …) → `None`.
3. Each test counts stop-word hits per word: embedded lists of the most
   frequent function words — en, de, ru (~60–90 each) — plus short
   **rival** lists (fr, es, it, pt, nl — which also catches Afrikaans — for Latin; uk, bg for Cyrillic)
   whose only job is to make a related language read as unknown rather
   than as en, de or ru. A Ukrainian-only letter (`і ї є ґ`) above 0.5 %
   of letters is also unknown.
4. The best of en/de (or ru) wins only if its share of words is ≥ 0.15,
   the other of en/de is below half of it, and every rival is below 0.6
   of it. Otherwise `None`.

`Prepared::language()` is `detect` over the chunks' texts with every
placeholder removed (so code, URLs and markup do not vote), computed once
in `prepare`.

### 4.8 `estimate_tokens` — calibrated, std only

A per-character cost, summed and rounded up: ASCII letter or digit
0.31, Cyrillic letter 0.42, CJK (Han, kana, Hangul, fullwidth) 0.75,
whitespace 0, everything else (punctuation, accented Latin, `⟦`, other
scripts) 1.0. Calibrated against Qwen3 4B's own tokenizer
(`llama-server … /tokenize`) on 57 paragraphs — 12 en, 12 ru, 12 de, 10
zh, 5 ja, 6 mixed with placeholders and code — to **never underestimate
by more than 5 %** on any of them (the target was 10 %); it overestimates
English most (mean ×1.39), which only makes English chunks smaller than
the budget. The table is in the report and in
`docs/architecture/pipeline.md`; the worst case of each language is a
regression test.

## §5 Tests (unit tests in `prepare/tests.rs`, `prepare/protect.rs` and `lang.rs`)

Every row's mutation was applied, the test watched go red, and the code
restored; the report has the table with the exact mutations.

| test | protects | mutation |
|---|---|---|
| `every_generated_document_reassembles_byte_for_byte_when_nothing_is_rewritten` (property: 3 000 generated inputs × 4 formats × 3 budgets) | the central invariant | skip one byte between chunks in `assemble` |
| `every_generated_chunk_restores_and_shows_the_model_no_stray_bracket_or_marker` (property) | placeholders 1..=n in order, no `⟦ ⟧ [[[ ]]]` in prose or context, every echo restores | drop the bracket matcher |
| `every_markdown_document_in_this_repository_reassembles_exactly` | the invariant on real Markdown | skip one byte |
| `every_placeholder_is_one_the_guard_counts` | E1's guard reads our placeholders | a leading zero in `placeholder` |
| `an_unchanged_candidate_restores_its_source_range` (canonical inputs) | the layout of §4.5 | always `\n` |
| `a_rewritten_quote_keeps_its_prefix_on_every_line`, `a_rewritten_list_item_keeps_its_indentation_on_every_line` | continuation prefix | omit the prefix |
| `crlf_bom_and_trailing_whitespace_survive` | the source's line ending | always `\n` |
| `headings_code_tables_html_blocks_rules_and_front_matter_are_never_in_a_chunk` | kept byte for byte | read a heading as a paragraph |
| `html_script_style_pre_and_headings_are_kept_whole` | | drop `pre` from the kept list |
| `a_code_document_has_no_chunks` | | — |
| `a_piece_with_no_letter_outside_its_placeholders_is_not_a_chunk` | | keep any non-empty piece |
| `each_paragraph_is_its_own_chunk_however_short` | the owner's "by paragraph" | group every piece into one unit |
| `a_list_is_one_chunk_and_its_glue_comes_back_exactly` | glue placeholders; whitespace at glue | glue copied as text; whitespace kept |
| `a_kept_block_inside_a_list_ends_its_chunk` | D70 | never interrupt a list |
| `a_list_over_the_budget_splits_between_items` | | never flush a list on the budget |
| `link_text_is_rewritten_and_the_destination_is_protected` | | protect the whole link |
| `inline_code_urls_emails_paths_and_entities_are_protected` | D68 | drop the URL matcher |
| `the_assemblers_markers_and_brackets_in_the_document_are_protected` | D66/D68 | drop the marker matcher; drop the bracket matcher |
| `numbers_and_quotations_are_not_protected` | D68 | protect digits |
| `placeholders_are_restored_exactly_and_numbered_per_chunk_from_one` | | restore an altered original; number from 0 |
| `a_missing_duplicated_unknown_or_reordered_placeholder_is_refused_by_name` | restore never guesses | ignore a missing one; ignore glue order |
| `a_long_paragraph_splits_at_sentence_ends_within_the_budget` | | no western sentence end |
| `a_split_never_falls_inside_a_placeholder` | | do not jump over spans |
| `a_sentence_over_the_budget_splits_at_whitespace` | | keep an over-budget sentence whole |
| `cjk_sentences_split_without_whitespace` | | CJK ends need whitespace |
| `the_budget_is_forty_percent_of_the_context_up_to_six_hundred` | D70 | ×0.5 |
| `the_context_is_the_last_two_sentences_of_the_previous_source` | D70 | one sentence |
| `a_marker_in_the_context_is_written_back_as_an_ellipsis` | | write it back verbatim |
| `a_long_context_is_cut_to_a_quarter_of_the_budget` | | no cap |
| `the_estimate_never_undercounts_the_calibration_set_by_more_than_ten_percent` | §4.8 | ASCII weight 0.25 |
| `the_language_is_detected_over_prose_without_placeholders` | | detect over the whole source |
| `protect::tests::*` (three) | the matchers' edges: trailing punctuation, balanced brackets, `and/or`, entities | — |
| `lang::tests::english_german_and_russian_are_detected` | D65 | — |
| `lang::tests::short_text_is_unknown` | | minimum words 1; minimum letters 1 |
| `lang::tests::french_spanish_and_the_other_neighbours_are_unknown_not_english_or_german` (fr, es, it, pt, nl, Afrikaans) | | rivals never count |
| `lang::tests::ukrainian_and_bulgarian_are_unknown_not_russian` | | ignore `і ї є ґ` |
| `lang::tests::mixed_scripts_and_mixed_languages_are_unknown` | | no en/de margin |

A test the first draft of this table had — that the context comes from
the source and not from a rewrite — was dropped: `assemble` takes `&self`
and the context is computed in `prepare`, so no mutation of the code
could make it fail. That property is structural, and §4.6 says so.

## §6 Acceptance criteria

1. All gates green; every mutation of §5 recorded red in the report.
2. The API of §4.1, exactly; nothing E4-2 owns touched.
3. `assemble(&[None; n]) == source` for every generated input in every
   format.
4. The calibration table (≥ 10 paragraphs each of en, ru, de, CJK)
   recorded; the server killed afterwards.
5. `docs/architecture/pipeline.md` "Preparing the text" and the report
   in the commit.

## §7 Out of scope

- The prompt, its markers, its variables (E4-2); the loop, the guards'
  orchestration, Layer A before and after, events (E4-3).
- The `code` tactic's own preparation (E4-3/E4-6 decide what of a code
  file is text).
- A per-language deterministic pass (D73), user regexes (D68).
- Mapping `wipemark_intake::Format` to `TextFormat` (the application,
  E4-6).

## §8 Basis and references

- OV §4.2; README D64–D70, D76; the prompts register §1, §2 Q-B3,
  Q-B9–Q-B12; `docs/sdd/layer-b-rewrite-reference.md` §3 `chunk` (a fresh
  context per fragment is itself an attack on a key that hashes preceding
  tokens) and §7.
- CommonMark 0.31 (container blocks, lazy continuation lines);
  `pulldown-cmark` 0.13's `OffsetIter`.
- The HTML Living Standard's list of block-level and raw-text elements.
