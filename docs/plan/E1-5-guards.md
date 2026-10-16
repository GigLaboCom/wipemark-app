# E1-5 — Guards: five predicates that will reject a rewrite which lost something

|                  |                                                                                                                                                                                                                                                                                                                                                  |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic E1, Layer A                                                                                                                                                                                                                                                                                                                  |
| Spec scopes      | **S1.7** — A §6 (all of it), A §5.5 (letter shares), A §8 row "guards"; OV §3.3 (the five guards), OV §4.2 (where `⟦n⟧` placeholders come from), OV §4.4 (who calls guards)                                                                                                                                                                          |
| Depends on       | **E1-1** (`is_letter`, `is_decimal_digit`, `is_uppercase_letter`, `is_lowercase_letter`, `is_space_separator`) and **E1-2** (`stats::letter_shares`, `LetterShares`) committed on `feat/e0-e6-shell`. Not E1-3 or E1-4: this is the one parallel lane of the series (README §3.1 item 2) and may run in a worktree beside them. Decisions D24, D43, D44 of `docs/plan/README.md` §4 |
| Unblocks         | **E4** (the selection loop rejects candidates with `default_guards()`); E1-7 lists the guards as built                                                                                                                                                                                                                                             |
| Files touched    | edited: `crates/wipemark-core/src/guard.rs` (the five guards, `default_guards`, the new variant, doc comments, tests), `crates/wipemark-core/src/lib.rs` (the `pub use guard::{…}` line only), `docs/architecture/layer-a.md` (section "Guards"), `docs/plan/README.md` (status row); new: `docs/plan/reports/E1-5-<YYYY-MM-DD>.md`                |
| Size             | ~2 days for one agent; no network, no application launch                                                                                                                                                                                                                                                                                          |

## §0 Ground rules

### 0.1 Start here

You are an implementer agent working alone in `~/self/wipemark-app`
(GitHub `GigLaboCom/wipemark-app`), a Rust + GPUI desktop application
that strips AI-provenance marks from its owner's own text. This section
is identical in every document of the `docs/plan/` E1 series so that the
document is complete on its own. Read it, then read `CLAUDE.md` at the
repository root in full — if the two disagree, `CLAUDE.md` wins and you
say so in your report.

```sh
cd ~/self/wipemark-app
git switch feat/e0-e6-shell          # the working branch; never main
git status                           # must be clean apart from ` m vendor/gpui-component`
git submodule sync --recursive       # the submodule URL moved to GigLaboCom on 2026-10-03
git submodule update --init --recursive
scripts/pin-gpui-component.sh        # idempotent; skipping it = two `gpui` packages and a baffling type error
```

If the prompt tells you to work in a worktree, create it **from
`feat/e0-e6-shell`** (`git worktree add ../wipemark-<id> -b e1/<topic>
feat/e0-e6-shell`) and run the two submodule commands inside it. Commit
only when the prompt says so — one commit for the document, message
`E1-n: <title>`, ending with the co-author line the prompt gives you.
Never push unless the prompt says so (only E1-7 is ever told to), never
touch `main`, never `git checkout -- <file>` to undo a probe in a file
that has other uncommitted work.

### 0.2 Where code goes

```
core ← engine ← pipeline ← app / cli          (scripts/check-dep-direction.sh, the whole graph in a second)
models never depends on engine; image depends only on core; nothing depends on an app crate
```

- **`wipemark-core` has zero dependencies** — no `[dependencies]`, no
  `[dev-dependencies]`, **no `[build-dependencies]`**
  (`check-dep-direction.sh:120-136` reads all three). `build.rs` is
  written against `std` alone; a UCD line is `split(';')` and
  `u32::from_str_radix`. The crate is `#![forbid(unsafe_code)]`.
- **Only applications localize.** No library depends on
  `wipemark-i18n`. A library hands up structured values (ids, enums,
  numbers); the CLI and the app turn them into prose.
- **The module map and the interfaces between the E1 documents are
  fixed** in `docs/plan/README.md` §3.2–3.3. Build against them exactly;
  if one is wrong, implement it as written and say so in the report.
- Positions are **byte offsets into the source `&str`**, always.
- Nothing long runs on the GPUI thread (CLAUDE.md "Nothing blocks the
  GPUI thread"); Layer A is O(n) and is called from the MCP socket
  thread and the CLI, never from a render.

### 0.3 Rules of this repository that bind Layer A

- **The third shelf is never empty.** Every report that leaves the
  process carries `not_established` with the ids of
  `wipemark_core::report::not_established::ALL`. Nothing anywhere says
  "undetectable" — in any language.
- **No epic number leaves this repository.** A string a user or an agent
  reads says "not in this version yet", never `E2`. Epic ids live in
  code comments, docs and log lines.
- **Every string a person reads comes from the catalogue; nothing a
  machine reads does.** Catalogue: `crates/wipemark-i18n/i18n/en-US/
  wipemark.ftl` is the source of truth, `de` and `ru` must carry every
  key (the build generates `Message` from en-US; a missing key in
  another language fails the suite). JSON fields, class ids
  (`UnicodeClass::as_str`), confidence/action ids, `--json` output and
  everything the MCP server says are formats and are never translated.
  The CLI uses `Rendering::PlainText` — Fluent's U+2068/U+2069 isolates
  are `BidiControl`, and a CLI that printed them would be marking the
  files it was pointed at.
- **A tool that cannot do the work refuses; it never reports nothing.**
  MCP refusals are *results* with `isError: true`; CLI refusals exit 2.
  An empty report for a scan that did not run is the failure this
  product exists to avoid.
- **Exit codes are the CLI's interface:** `0` clean, `1` findings, `2`
  usage or refusal, `3` partial — *inconclusive is not clean*.
- **Layer A is never licence-gated.**

### 0.4 Tests: how this repository writes them

- **RED first, then green.** Write the test, watch it fail for the right
  reason, then write the code.
- **Delete the protection and watch it go red.** For every protection
  listed in your document's test section, apply the stated mutation
  locally, run the test, confirm it fails, restore the code, and record
  the result in your report (protection · mutation · test that went red).
  A test that stays green with its subject deleted is removed, not kept.
- Test names are sentences in `snake_case` that state the behaviour
  (`a_family_stays_a_family`, `positions_are_byte_offsets_into_the_source`)
  — the names in your document are the names to use.
- Fixtures under `fixtures/text/` are byte-exact (`.gitattributes`
  `fixtures/text/** -text`) and every fixture is asserted somewhere.
  Tests that need UCD data or fixtures read them with `include_str!` /
  `include_bytes!`; **no test touches the network**.
- Tests that exercise `pub(crate)` items are unit tests in their module
  (`#[cfg(test)] mod tests`, reading data with `include_str!` from `ucd/`
  or `fixtures/` when they need it); only tests that use the public API
  go to `crates/wipemark-core/tests/` — an integration test cannot see
  `pub(crate)`.

### 0.5 Gates — all green before you report done

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')   # not `cargo fmt --all`: never reformat vendor/
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
```

While iterating, `cargo test -p wipemark-core` and `cargo clippy -p
wipemark-core --all-targets -- -D warnings` are minutes faster; the full
set runs before you report. `--locked` means `Cargo.lock` must not move
under you: `wipemark-core` never gains a dependency. A document that adds
a dependency to another crate (only E1-6 does) records it with one
`cargo check -p <crate>` **without** `--locked`, commits the moved
`Cargo.lock` together with the manifest, and then runs the gates with
`--locked` as above.

### 0.6 Do not

- launch the application unless your document's acceptance criteria ask
  for a live check — and then once, at the end, and kill it afterwards
  (a running `wipemark` holds MCP port 5056 and makes
  `a_port_something_else_holds_is_stepped_past` fail for an unrelated
  reason);
- edit anything under `vendor/` or reformat it;
- add a dependency to `wipemark-core`, or a `unicode-*` crate anywhere;
- change the module map or the interfaces in `docs/plan/README.md`
  §3.2–3.3 on your own;
- leave a `TODO` where your document asks for behaviour — implement it or
  report it as not done.

### 0.7 Definition of done (every document)

1. All six commands of §0.5 green; the mutation checks of your document
   performed and recorded.
2. The acceptance criteria of your document ticked, each with evidence
   (test name or command).
3. `docs/architecture/layer-a.md` has the section your document names,
   written for someone working on this code next year (what, where, why);
   `CLAUDE.md` touched only where your document says.
4. The status row in `docs/plan/README.md` §3 set to *done* with the
   report's file name.
5. A closing report at `docs/plan/reports/<id>-<YYYY-MM-DD>.md`: what was
   built; every deviation from the document and why; the mutation table;
   the commands you ran and their results; what you found that the next
   document should know. Report failures as failures.

## §1 Goal

Layer B (E4) will ask a model to rewrite a chunk, several times, and keep
the best candidate. A candidate that "improved" a version number,
dropped a protected code span, translated the paragraph into another
language or lost a file path is not a rewrite; it is damage that a
divergence score would happily reward. A §6 and OV §3.3 name five
predicates over the pair *(source, candidate)* that throw such a
candidate away, and put them in `wipemark-core` — pure text, no engine.

The `Guard` trait, `GuardOutcome` and six of the seven `RejectReason`
variants exist since E0. This document writes the five guards —
`PlaceholderGuard`, `NumbersGuard`, `LengthDriftGuard`, `ScriptGuard`,
`IdentifierGuard` — with the spec's numbers as their `Default`, the
seventh reason `PlaceholderInvented`, and `default_guards()` in the
spec's order, each guard with a test that fails without it. Nothing
calls them in E1 except their tests; E4 wires them into the selection
loop (A §6: "в E1 у них нет вызывающего, кроме тестов; E4 подставляет в
цикл отбора").

The guards are deliberately strict and deliberately simple. A false
reject costs one more candidate; a false pass costs the user a lost
number in their document. Every place where that trade shows — a
reworded "50 %", an identifier glued to a dash — is stated in the doc
comments and §4.12, not discovered later.

## §2 Read first

Line numbers are at `497eafa` unless the row says otherwise; E1-1 and
E1-2 will have moved some of them — re-find by name.

1. `CLAUDE.md` in full. Rules this document leans on (line numbers at
   `497eafa`; HEAD `a2f6ba3` added 9 lines near the top): "Delete the
   protection and watch it go red" (`:85`), "`wipemark-core` has zero
   dependencies" (`:192`), "Every string a person reads comes from the
   catalogue; nothing a machine reads does" (`:220`), "Only applications
   localize" (`:226`), "Tests must be able to fail" (`:1037`).
2. `docs/plan/README.md` — §3.1 item 2 (this is the parallel lane), §3.3
   "E1-5 provides", §4 D24, D43, D44.
3. `ssd-docs/wipemark-core-layer-a-2026-09-21.md` (A) — **§6 entirely**,
   §5.5 (`TextStats`, letter shares), §8 row "guards", §2 ("protected
   spans `⟦n⟧` — E4"), §7.5 ("calling `clean` on text whose `⟦n⟧` have
   not been taken out … is not something Layer A protects"). Russian;
   §4 below is its exact English.
4. `ssd-docs/heretic-unmark-overview-decomposition-2026-09-07.md` (OV) —
   §3.3 (the five guards), §4.2 steps 2 and 4 (protected spans become
   `⟦n⟧` placeholders: fenced and inline code, URLs, e-mails, paths,
   numbers with units, a user regex list; restored after the rewrite),
   §4.4 (the loop: `guards → Reject? → log, continue`).
5. `crates/wipemark-core/src/guard.rs` — all 131 lines: module doc
   (`:1-11`), `RejectReason` (`:15-37`), its `Display` (`:39-63`),
   `GuardOutcome` (`:65-84`), `Guard` (`:86-97`), tests (`:99-131`).
6. `crates/wipemark-core/src/lib.rs:35` — `pub use guard::{Guard,
   GuardOutcome, RejectReason};`.
7. `crates/wipemark-pipeline/src/lib.rs:83-92` — `Event::CandidateRejected
   { job, round, candidate, guard: &'static str, reason: String }`: the
   future consumer of `Guard::name()` and of a rendered reason.
8. `crates/wipemark-core/src/report.rs:47-58` — `TextStats` (the
   `latin/cyrillic/cjk` ratios A §5.5 defines); E1-2's `stats.rs`.
9. `crates/wipemark-engine/src/fake.rs:66-71`, `:107-131` — `FakeEngine` rotates
   whitespace-separated words and re-joins them with one space; the
   guards are what E4's gate runs over its output (§7).
10. `docs/sdd/layer-b-rewrite-reference.md:450-471` — the reference's
    length penalty and "no-op guard" (E4's, not this document's).

## §3 What is true today, and what earlier documents delivered

### 3.1 At `497eafa`

| fact | where |
|---|---|
| `RejectReason` has six variants: `PlaceholderMissing { index: usize }`, `PlaceholderDuplicated { index: usize, count: u32 }`, `NumberMissing { value: String }`, `LengthDrift { ratio: f32, min: f32, max: f32 }`, `ScriptDrift { script: &'static str, delta_pp: f32 }`, `IdentifierMissing { token: String }`; derives `Debug, Clone, PartialEq` | `crates/wipemark-core/src/guard.rs:20-37` |
| its `Display` texts: `protected span ⟦{index}⟧ did not come back`, `protected span ⟦{index}⟧ came back {count} times`, `number or date {value:?} was lost`, `length ratio {ratio:.2} outside [{min:.2}, {max:.2}]`, `{script} share moved by {delta_pp:.1} pp — looks translated`, `identifier {token:?} was lost` | `guard.rs:39-63` |
| the enum's doc says "the report shows these verbatim" — at odds with CLAUDE.md `:220` (a string a person reads comes from the catalogue); §4.1 rewrites that sentence | `guard.rs:17-19` |
| the module doc claims "Layer A uses the same definitions when it reports what a clean pass changed" — not true in E1 (nothing calls a guard); §4.2 rewrites it | `guard.rs:3-5` |
| `GuardOutcome { Pass, Reject(RejectReason) }` with `is_pass()` and `reason()` | `guard.rs:65-84` |
| `pub trait Guard: Send + Sync { fn name(&self) -> &'static str; fn check(&self, source: &str, candidate: &str) -> GuardOutcome; }` — "`source` is the text handed to the engine (placeholders already substituted), `candidate` is what came back"; "stateless and cheap: the selection loop runs every guard over every candidate of every round" | `guard.rs:86-97` |
| tests `outcome_reports_its_reason`, `reasons_render_specifically` | `guard.rs:103-130` |
| `lib.rs` re-exports `Guard, GuardOutcome, RejectReason` | `lib.rs:35` |
| no guard implementation, no caller anywhere | `grep -rn 'impl Guard' crates apps` is empty |
| `Event::CandidateRejected.guard` is `&'static str` (a `Guard::name()`), `reason` a `String` | `crates/wipemark-pipeline/src/lib.rs:86-92` |
| workspace lints run under `-D warnings`; `clippy::manual_range_contains` is a default `style` lint (write `(min..=max).contains(&x)`) | `Cargo.toml:137-160`, §0.5 |

### 3.2 What E1-1 delivered (the part you use)

```rust
// crate::tables — binary searches over UCD 18.0.0 ranges
pub(crate) fn is_space_separator(c: char) -> bool;   // gc = Zs (U+0020 included)
pub(crate) fn is_letter(c: char) -> bool;            // gc = L*
pub(crate) fn is_decimal_digit(c: char) -> bool;     // gc = Nd
pub(crate) fn is_uppercase_letter(c: char) -> bool;  // gc = Lu  (README §3.3: "IdentifierGuard's CamelCase")
pub(crate) fn is_lowercase_letter(c: char) -> bool;  // gc = Ll
```

Use these, not `char::is_numeric`/`is_alphanumeric`/`is_lowercase`/`is_whitespace`: the
std predicates follow the Rust toolchain's Unicode version and other
properties (`Numeric_Type`, `Alphabetic`, `Lowercase`), and every Unicode
fact in this crate comes from the committed 18.0.0 tables.

### 3.3 What E1-2 delivered (the part you use)

```rust
// crate::stats
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LetterShares { pub letters: usize, pub latin: f32, pub cyrillic: f32, pub cjk: f32, pub other: f32 } // percent, 0..=100
pub(crate) fn letter_shares(text: &str) -> LetterShares;
```

What you rely on (A §5.5): `letters` counts the code points with
`gc = L*`; `latin`, `cyrillic`, `cjk` are 100 × their share **among
letters**, CJK = Han ∪ Hiragana ∪ Katakana ∪ Hangul ∪ Bopomofo; `other`
is the rest; with `letters == 0` the four are 0.0. An all-Latin text has
`latin == 100.0` exactly; an all-Cyrillic one `cyrillic == 100.0`
(`n * 100 / n` is exact in `f32`). If E1-2 differs, implement against
HEAD and say so; §5's inputs are chosen so that only these facts matter.

### 3.4 What this document does not wait for

E1-3 (`inspect`, `clean`, `Options`) and E1-4 (homoglyphs) may land
before, after or beside this one. Nothing here uses them. `lib.rs` will
be edited by them too; you change only the `pub use guard::{…}` line, so
the coordinator's merge is one line. Do not touch `tables.rs`, `script.rs` or
`stats.rs` either: E1-1 marked their items `#[cfg_attr(not(test),
allow(dead_code))]` (E1-1 §4.9), and E1-3 §4.3.1 narrows that allowance
in the other lane. Your calls to `is_uppercase_letter`,
`is_lowercase_letter`, `is_decimal_digit`, `is_letter` and
`letter_shares` make some of those allowances unnecessary but not wrong
— an `allow` on a used item warns about nothing. List them in your
report; E1-7 removes what remains.

### 3.5 Decisions that bind this document

**D43** — `IdentifierGuard`'s edge-trim set is A §6's eighteen
characters **plus** U+2018, U+2019, U+201C, U+201D, U+201E, U+2039,
U+203A and U+0060 (§4.3).

**D44** — the choices G1–G12 of §4.12 are adopted as written (G9 as
amended by D43); cite D44 for them in code comments and the report.

**D24** — tests that exercise `pub(crate)` items are unit tests in their
module. The tokenizers of §4.3 are private, so every test of this
document lives in `guard.rs`'s existing `#[cfg(test)] mod tests`. (The
guards themselves are public; a test through the public API alone may go
to `crates/wipemark-core/tests/`, but nothing is gained by splitting.)

## §4 Deliverables

### 4.1 `RejectReason` — the seventh variant, and the doc it needs

Add, after `PlaceholderDuplicated`:

```rust
    /// A `⟦n⟧` placeholder the source never had appeared in the
    /// candidate. E4 restores placeholders by exact text, so an invented
    /// one is either restored as nothing or as the wrong span.
    PlaceholderInvented { index: usize },
```

and in `Display`:

```rust
            RejectReason::PlaceholderInvented { index } => {
                write!(f, "protected span ⟦{index}⟧ appeared but was never in the source")
            }
```

Leave the other six variants and texts exactly as they are (the E0 test
`reasons_render_specifically` pins two of them). Replace the enum's doc
(`guard.rs:15-19`) with:

```rust
/// Why a candidate was thrown away.
///
/// Every variant names the specific thing that was lost — "guard
/// failed" would tell nobody anything. The fields are the format: a
/// surface a person reads (the report, the Inspector) renders the
/// variant through the catalogue, never the `Display` text, because
/// every string a person reads comes from the catalogue (CLAUDE.md).
/// `Display` is English for logs and diagnostics.
```

### 4.2 The module doc

Replace `guard.rs:1-11` with:

```rust
//! Guards — five predicates over `(source, candidate)` that reject a
//! rewrite which lost something (A §6, OV §3.3).
//!
//! A guard answers one question: *did the rewrite quietly destroy
//! something the user needs?* A model that "improves" a version number,
//! translates a paragraph, drops a protected span or loses a file path
//! has produced a candidate that must be thrown away however good its
//! divergence score is.
//!
//! They live in `core` because they are pure text predicates with no
//! engine involved, and because they read the same UCD 18.0.0 tables as
//! the scrubber (digits, letters, case, scripts). Nothing calls them in
//! E1 but their tests; E4's selection loop runs [`default_guards`] over
//! every candidate of every round, on text whose protected spans are
//! already `⟦n⟧` placeholders.
//!
//! They are strict on purpose: a false reject costs one more candidate,
//! a false pass costs the user a number. What they cannot see is said on
//! each guard.
```

### 4.3 The three tokenizers (private functions in `guard.rs`)

All three return slices of their input in source order, never allocate
per character, and are O(n).

**Placeholders** — `fn placeholders(text: &str) -> Vec<usize>`.

1. Scan the `char_indices`. At U+27E6 MATHEMATICAL LEFT WHITE SQUARE
   BRACKET, take the longest run of **ASCII** digits `0`–`9` that follows
   it.
2. It is a placeholder when the run is non-empty, the character right
   after it is U+27E7 MATHEMATICAL RIGHT WHITE SQUARE BRACKET, the run is
   `"0"` or does not start with `0`, and it parses as `usize`. Push the
   number and continue after the U+27E7.
3. Otherwise it is not a placeholder; continue with the character after
   the U+27E6.

Why exactly this: E4 writes placeholders as `⟦{n}⟧` with `n: usize`
formatted by `{}` — ASCII digits, no leading zero — and restores them by
exact text. `⟦03⟧` or `⟦٣⟧` (U+0663) in a candidate is therefore not
placeholder 3; it is lost text, and the source's `⟦3⟧` is missing.

| text | placeholders |
|---|---|
| `⟦0⟧` | `[0]` |
| `⟦1⟧⟦2⟧` | `[1, 2]` |
| `⟦⟦1⟧⟧` | `[1]` (the outer brackets are text) |
| `⟦03⟧`, `⟦⟧`, `⟦x⟧`, `⟦` U+0663 `⟧` | `[]` |
| `⟦99999999999999999999⟧` (20 nines, > `u64::MAX`) | `[]` |

**Numbers** — `fn numbers(text: &str) -> Vec<&str>`. A number token
(A §6): starts and ends with an `Nd` character; inside, `Nd` or one of the
five separators `.` `,` `:` `/` `-`; an optional `%` (U+0025) at the end.

1. Scan the `char_indices`. At a character with `is_decimal_digit`, start
   a token.
2. Extend it over every following character that is `Nd` or one of the
   five separators, remembering the end of the **last digit**.
3. The token ends at that last digit; if the very next character is `%`,
   it includes it.
4. Push `&text[start..end]`; continue scanning at `end`.

| text | numbers |
|---|---|
| `v1.2.3` | `["1.2.3"]` (the `v` is not part of it) |
| `It was 3.` | `["3"]` (a trailing separator is not) |
| `50%` | `["50%"]` |
| `0.5 %` | `["0.5"]` (the `%` must touch the digit) |
| `1, 2` | `["1", "2"]` |
| `-5` | `["5"]` (a sign is not tracked) |
| `2026-10-03`, `10:30`, `03/10/2026`, `192.168.0.1`, `1,234,567.89`, `12:00-13:00` | each one token, itself |
| `1..2` | `["1..2"]` (separators may repeat) |
| `1.2.3.%` | `["1.2.3"]` |
| `a1b2` | `["1", "2"]` |
| U+FF11 U+FF12 U+FF13 (fullwidth 123) | one token, itself — every `Nd` is a digit |
| U+0663 U+0664 (Arabic-Indic 34) | one token, itself |
| `twenty-three`, `½`, `²` | `[]` (`No`, not `Nd`; words are not numbers) |

**Identifiers** — `fn identifiers(text: &str) -> Vec<&str>`.

1. Split into maximal runs of non-whitespace, with **E1-2's whitespace**
   (`docs/plan/E1-2-classifier.md` §4.3): `is_space_separator(c)` or `c`
   in U+0009–U+000D, U+0085, U+2028, U+2029 — `PropList.txt`
   `White_Space` in 18.0.0. Not `str::split_whitespace`: E1-2 forbids
   `char::is_whitespace` in this crate because it is std's Unicode, not
   the version the report names. If E1-2 exposes its predicate as
   `pub(crate)`, call it; otherwise write the same one-line predicate in
   `guard.rs` with a comment naming E1-2's definition. U+200B is not
   whitespace.
2. Trim from both ends every character of exactly this set of
   twenty-six: A §6's eighteen — `.` `,` `;` `:` `!` `?` `(` `)` `[`
   `]` `{` `}` `"` `'` `«` (U+00AB) `»` (U+00BB) `<` `>` — and D43's
   eight — U+2018 LEFT SINGLE QUOTATION MARK, U+2019 RIGHT SINGLE
   QUOTATION MARK, U+201C LEFT DOUBLE QUOTATION MARK, U+201D RIGHT
   DOUBLE QUOTATION MARK, U+201E DOUBLE LOW-9 QUOTATION MARK, U+2039
   SINGLE LEFT-POINTING ANGLE QUOTATION MARK, U+203A SINGLE
   RIGHT-POINTING ANGLE QUOTATION MARK, U+0060 GRAVE ACCENT (the
   backtick). One `const TRIM: [char; 26]`, written with `\u{…}` escapes
   for the non-ASCII ones, and `str::trim_matches` with a closure over
   it. Skip an empty result. A trailing U+2019 inside a token
   (`user’s`) is untouched; only the ends are trimmed.
3. Keep the token when **any** of the five shapes holds:
   1. **URL** — it contains `://`;
   2. **e-mail** — some `@` in it has at least one character before it,
      and the part after it is non-empty and contains `.`;
   3. **path** — splitting it on `/` and `\` yields at least two
      non-empty segments;
   4. **snake_case** — some maximal run of `_` has, immediately before
      it and immediately after it, a character with `is_letter ||
      is_decimal_digit`;
   5. **CamelCase** — some character with `is_lowercase_letter` is
      immediately followed by one with `is_uppercase_letter`.

| token (as split) | identifier |
|---|---|
| `snake_case`, `my__var` | itself (a run of `_` counts) |
| `__init__`, `_private` | none — no character before the run |
| `camelCase`, `PayPal`, `iPhone` | itself |
| `HTTPServer` | none — no lower case before an upper case |
| `кВт` (U+043A U+0412 U+0442) | itself — case is Unicode's, not ASCII's |
| `src/lib.rs`, `/usr/bin`, `and/or`, `C:\Windows\x` | itself |
| `/usr`, `a/` | none — one segment |
| `https://x.y` | itself |
| `ops@example.com` | itself |
| `@user`, `a@b` | none |
| `(parse_config()),` | `parse_config` |
| `«foo_bar»` | `foo_bar` |
| `` `foo_bar` `` (backticks), `“foo_bar”` (U+201C/U+201D), `‘src/lib.rs’` (U+2018/U+2019), `„foo_bar“` (U+201E/U+201C), `‹foo_bar›` (U+2039/U+203A) | `foo_bar`, `foo_bar`, `src/lib.rs`, `foo_bar`, `foo_bar` (D43) |
| `foo_bar—see` (U+2014 EM DASH) | itself, dash included — a dash is not trimmed, and is inside the token anyway |

### 4.4 `PlaceholderGuard`

```rust
/// Every `⟦n⟧` of the source comes back as often as it went in, and no
/// other comes back at all (A §6). The placeholders stand for protected
/// spans — code, URLs, paths — that E4 took out before the rewrite and
/// puts back by exact text afterwards.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlaceholderGuard;
```

`name()` is `"placeholder"`. `check`:

1. Count `placeholders(source)` and `placeholders(candidate)` into two
   `BTreeMap<usize, u32>` (ordered, so the verdict never depends on hash
   order).
2. For each source index in ascending order, with `want` its source
   count and `got` its candidate count (0 when absent): `got < want` →
   `Reject(PlaceholderMissing { index })`; `got > want` →
   `Reject(PlaceholderDuplicated { index, count: got })`.
3. Then for each candidate index in ascending order that the source does
   not have → `Reject(PlaceholderInvented { index })`.
4. Otherwise `Pass`.

A §6 says "each from the source exactly once in the candidate"; E4
never emits a placeholder twice, so "as often as in the source" is the
same rule for every input E4 produces and does not reject a faithful
copy of a text that happened to contain one twice.

### 4.5 `NumbersGuard`

```rust
/// Every number of the source is still in the candidate (A §6): a run
/// that starts and ends with a decimal digit, with digits and `. , : / -`
/// inside and an optional `%` — dates, versions, times, thousands,
/// percentages. Compared as exact strings, as a set: a rewrite may move
/// or repeat a number, never change or drop one.
///
/// Not seen: numbers written as words ("twenty-three" protects nothing,
/// and a candidate that spells "23" as "twenty-three" is rejected), signs
/// ("-5" and "5" are the same token), units, and a "%" separated by a
/// space ("50 %" holds the token "50", so a rewrite to "50%" passes and
/// one from "50%" to "50 %" is rejected).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NumbersGuard;
```

`name()` is `"numbers"`. `check`: collect `numbers(candidate)` into a
`BTreeSet<&str>`; walk `numbers(source)` in source order; the first
token not in the set → `Reject(NumberMissing { value: token.to_owned() })`;
else `Pass`.

### 4.6 `LengthDriftGuard`

```rust
/// `chars(candidate) / chars(source)` stays inside `[min, max]` (A §6;
/// both ends inclusive). Counted in code points, never bytes: a CJK text
/// is three times longer in UTF-8 than the same length of Latin.
///
/// An empty source passes only an empty candidate; anything else is a
/// ratio of `f32::INFINITY`. `min > max` or a NaN bound rejects every
/// candidate — the caller's configuration, not a guess here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LengthDriftGuard {
    pub min: f32,
    pub max: f32,
}

impl Default for LengthDriftGuard {
    /// A §6 / OV §3.3: `[0.6, 1.6]`.
    fn default() -> Self {
        Self { min: 0.6, max: 1.6 }
    }
}
```

`name()` is `"length-drift"`. `check`:

1. `s = source.chars().count()`, `c = candidate.chars().count()`.
2. `s == 0`: `c == 0` → `Pass`; else `Reject(LengthDrift { ratio:
   f32::INFINITY, min, max })`.
3. `ratio = c as f32 / s as f32`; `(self.min..=self.max).contains(&ratio)`
   → `Pass`, else `Reject(LengthDrift { ratio, min: self.min, max:
   self.max })`.

The edges are exact in `f32`: `6.0 / 10.0` and the literal `0.6` round
to the same `f32`, as do `16.0 / 10.0` and `1.6` — §5 tests both edges.

### 4.7 `ScriptGuard`

```rust
/// The share of Latin, Cyrillic, CJK and other letters among all letters
/// moved by no more than `max_delta_pp` percentage points (A §6): a model
/// told to rewrite that translated instead. A text with fewer than
/// `min_letters` letters on either side passes — two words have no
/// shares.
///
/// Arabic, Hebrew and every other script are one bucket, "other": an
/// English text translated into Arabic is caught as Latin falling and
/// "other" rising, but the reason does not name Arabic (A §9 Q-A5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptGuard {
    pub max_delta_pp: f32,
    pub min_letters: usize,
}

impl Default for ScriptGuard {
    /// A §6 / OV §3.3: 15 percentage points, 20 letters.
    fn default() -> Self {
        Self { max_delta_pp: 15.0, min_letters: 20 }
    }
}
```

`name()` is `"script"`. `check`:

1. `s = letter_shares(source)`, `c = letter_shares(candidate)`.
2. `s.letters < self.min_letters || c.letters < self.min_letters` → `Pass`.
3. For `(id, before, after)` in this fixed order — `("latin", s.latin,
   c.latin)`, `("cyrillic", …)`, `("cjk", …)`, `("other", …)` — let
   `delta = after - before`; the first with `delta.abs() >
   self.max_delta_pp` (strictly) → `Reject(ScriptDrift { script: id,
   delta_pp: delta })`.
4. Otherwise `Pass`.

`script` is one of the four ids above — a format, lower case, never
translated; `delta_pp` is signed (candidate minus source), so an English
text translated into Russian reports `("latin", -100.0)`. The fixed order
(rather than "the largest delta") keeps the verdict free of `f32` ties:
in a two-script swap both deltas have the same magnitude.

### 4.8 `IdentifierGuard`

```rust
/// Every identifier-shaped token of the source is still in the
/// candidate, exactly (A §6): a URL, an e-mail, a path, a snake_case or a
/// CamelCase name, after trimming the punctuation a sentence wraps them
/// in. As a set: a rewrite may move one, never change it.
///
/// The trimmed set is A §6's plus curly quotes, angle quotes and the
/// backtick (D43), so straightening or curling the quotes around an
/// identifier, or dropping its Markdown backticks, loses nothing. Strict
/// otherwise, on purpose: a candidate that glues the identifier to a dash
/// or an ellipsis loses it and is rejected. A false reject costs one
/// candidate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IdentifierGuard;
```

`name()` is `"identifier"`. `check`: as `NumbersGuard`, over
`identifiers(…)`; the first source identifier (in source order) missing
from the candidate's set → `Reject(IdentifierMissing { token:
token.to_owned() })`.

### 4.9 `default_guards`

```rust
/// The five guards of A §6 in the spec's order, with the spec's
/// thresholds. E4's selection loop runs them in this order and reports
/// the first rejection.
pub fn default_guards() -> Vec<Box<dyn Guard>> {
    vec![
        Box::new(PlaceholderGuard),
        Box::new(NumbersGuard),
        Box::new(LengthDriftGuard::default()),
        Box::new(ScriptGuard::default()),
        Box::new(IdentifierGuard),
    ]
}
```

The order matters to E4: a lost placeholder also loses the digits inside
it (`⟦3⟧` holds the number token `3`), and the placeholder reason is the
one that names what happened. The five `name()`s are formats — kebab
case, never translated, renamed only with the care of a config key —
and unique.

### 4.10 `lib.rs`

Replace the line `pub use guard::{Guard, GuardOutcome, RejectReason};`
with

```rust
pub use guard::{
    default_guards, Guard, GuardOutcome, IdentifierGuard, LengthDriftGuard, NumbersGuard,
    PlaceholderGuard, RejectReason, ScriptGuard,
};
```

(the nightly rustfmt of §0.5 decides the final layout — run it). Touch
nothing else in `lib.rs`.

### 4.11 `docs/architecture/layer-a.md` — section "Guards"

E1-1 created the file with a list of sections; add "## Guards" after
"Homoglyphs" if that section is there, else at the end (the coordinator
orders sections when the lanes merge). For someone touching this code
next year: what a guard is and who calls it (nobody in E1; E4's loop,
on `⟦n⟧`-substituted text); the five rules as one table (guard · rule ·
reason · threshold); the three tokenizers with three examples each; why
they are strict (false reject vs false pass); what they cannot see
(numbers as words, signs, units, an identifier glued to a dash, Arabic
as "other"); the trim set with D43's additions; that
`Display` is for logs and the fields are the format; the `name()` ids;
where the tests are and what each mutation was.

### 4.12 Where this document is more precise than A §6 — and why

All twelve are adopted by **D44** (G9 as amended by **D43**).

| # | A §6 says | this document | why |
|---|---|---|---|
| G1 | "`⟦` decimal number `⟧`" | ASCII digits, canonical (`0` or no leading zero), fits `usize` | E4 writes `⟦{n}⟧` and restores by exact text; `⟦03⟧` is not `⟦3⟧` to it either |
| G2 | "each from the source exactly once in the candidate" | as often as in the source | identical for every E4 input; does not reject a faithful copy of a text that contained a placeholder-shaped string twice |
| G3 | one reason per verdict, unspecified which | missing/duplicated by ascending index, then invented by ascending index | deterministic, independent of the candidate's order |
| G4 | "starts and ends with `Nd` … optional `%`" | maximal munch; trailing separators dropped; `%` only touching the last digit; all `Nd` (fullwidth, Arabic-Indic) | the literal rule, made into a scanner |
| G5 | "set ⊆" | exact strings, first missing in source order | deterministic reason |
| G6 | `len(result)/len(source) ∈ [0.6, 1.6]` | inclusive; empty source → pass iff empty candidate, else `∞` | the ratio is undefined at 0 |
| G7 | "rejects if any share moved more than 15 pp" | strictly more; first of latin, cyrillic, cjk, other; signed delta | `f32` ties in a two-script swap; "15.0 exactly" passes |
| G8 | "fewer than `min_letters` on either side → Pass" | `letters < min_letters` (20 letters is enough) | the spec's words, with its boundary tested |
| G9 | trimmed set `.,;:!?()[]{}"'«»<>` | those 18 plus U+2018, U+2019, U+201C, U+201D, U+201E, U+2039, U+203A, U+0060 — 26 (D43) | a rewrite that straightens or curls the quotes around an identifier, or drops its Markdown backticks, has not lost it |
| G10 | "`/` or `\` between non-empty segments" | ≥ 2 non-empty segments after splitting on both | `/usr` is a root, not a path between segments |
| G11 | "`_` between alphanumerics" | a run of `_`, alphanumeric = `L*` or `Nd` (E1-1) | `my__var`; `__init__` is not caught |
| G12 | (no names) | `placeholder`, `numbers`, `length-drift`, `script`, `identifier` | `Event::CandidateRejected.guard` needs a stable id |

## §5 Tests — all in `crates/wipemark-core/src/guard.rs` `mod tests` (D24)

The faithful pair, used by several tests (ASCII except the brackets
U+27E6/U+27E7):

```rust
const SOURCE: &str = "Version 1.94.1 shipped on 2026-10-03 at 10:30. Call parse_config() from \
src/config.rs before \u{27E6}0\u{27E7}, and read https://example.com/docs/setup for the camelCase \
options. Questions go to ops@example.com; the cache hit rate rose by 15% after \u{27E6}1\u{27E7}.";
const FAITHFUL: &str = "Before \u{27E6}0\u{27E7}, call parse_config() from src/config.rs. The camelCase \
options are described at https://example.com/docs/setup, and version 1.94.1 was released on \
2026-10-03 at 10:30. After \u{27E6}1\u{27E7} the cache hit rate went up by 15%; send questions to \
ops@example.com.";
```

(`\` at a line end continues the literal and drops the newline and the
next line's leading spaces — the strings above have none.) Measured:
source 239 code points, candidate 258 (ratio 1.0795); letters 159 and
174, all Latin; placeholders `[0, 1]` both; numbers of the source
`1.94.1, 2026-10-03, 10:30, 0, 15%, 1`; identifiers `parse_config,
src/config.rs, https://example.com/docs/setup, camelCase, ops@example.com`.

| test | input → expected | mutation that paints it red |
|---|---|---|
| `a_lost_placeholder_is_rejected` | `PlaceholderGuard.check(SOURCE, &FAITHFUL.replace("After \u{27E6}1\u{27E7}", "Afterwards"))` → `Reject(PlaceholderMissing { index: 1 })` | walk the candidate's indices instead of the source's |
| `an_invented_placeholder_is_rejected` | candidate = `FAITHFUL.replace("send questions", "see \u{27E6}2\u{27E7} and send questions")` → `Reject(PlaceholderInvented { index: 2 })` | delete step 3 of §4.4 |
| `a_duplicated_placeholder_is_rejected` | candidate = `FAITHFUL.replace("call parse_config()", "call parse_config() (\u{27E6}0\u{27E7})")` → `Reject(PlaceholderDuplicated { index: 0, count: 2 })` | compare presence instead of counts (a `BTreeSet`) |
| `placeholders_are_found_exactly` | the table of §4.3 "Placeholders" | accept a leading zero (`⟦03⟧` → 3), or accept any `Nd` (`⟦`U+0663`⟧` → 3) — each red |
| `a_lost_version_number_is_rejected` | `NumbersGuard.check(SOURCE, &FAITHFUL.replace("version 1.94.1", "version 1.94"))` → `Reject(NumberMissing { value: "1.94.1".into() })` | let `.` end a token: `1.94` then holds `1` and `94`, the source's `1`, `94`, `1` are all present → `Pass` |
| `numbers_are_found_where_the_spec_says` | the table of §4.3 "Numbers" | keep trailing separators (`3.` → `"3."`), or take `%` after a space (`0.5 %` → `"0.5 %"`), or use `char::is_ascii_digit` (fullwidth and Arabic-Indic rows) — each red |
| `a_number_may_move_or_repeat` | `NumbersGuard.check("1.94.1 shipped 2026-10-03", "On 2026-10-03, 1.94.1 shipped; 1.94.1 is out")` → `Pass` | compare the token sequences instead of sets |
| `cjk_length_is_counted_in_chars_not_bytes` | `LengthDriftGuard::default()`: `("東京は日本の首都です", "Tokyo: capital")` (10 and 14 code points; 30 and 14 bytes) → `Pass`; `("abcdefghijklmnopqrst", "日本語の文章")` (20 and 6 code points; 20 and 18 bytes) → `Reject(LengthDrift { ratio: 0.3, min: 0.6, max: 1.6 })` (compare `ratio` within 1e-6) | `str::len` instead of `chars().count()`: the first becomes 0.467 → reject, the second 0.9 → pass |
| `the_length_window_is_inclusive` | 10 ASCII chars against 6, 16, 5, 17 ASCII chars → `Pass`, `Pass`, `Reject(ratio 0.5)`, `Reject(ratio 1.7)`; `("", "")` → `Pass`; `("", "x")` → `Reject(LengthDrift { ratio: f32::INFINITY, .. })` | exclusive bounds (`min < r && r < max`): the 6 and 16 rows go red |
| `a_translation_is_rejected_as_script_drift` | `ScriptGuard::default().check("The quarterly report is ready, and the numbers look better than last year.", "Квартальный отчёт готов, и цифры выглядят лучше, чем в прошлом году.")` (60 and 55 letters) → `Reject(ScriptDrift { script: "latin", delta_pp })` with `(delta_pp + 100.0).abs() < 1e-3`; the same source against `"The report for the quarter is ready, and its numbers beat last year's."` → `Pass` | `check` always answers `Pass` (or compares `cjk` only) |
| `a_short_text_has_no_script_share` | `("Hello there", "Привет всем")` (10 and 10 letters) → `Pass` | delete the `min_letters` condition → `Reject` (latin −100) |
| `twenty_letters_are_enough_for_a_share` | `("abcdefghij klmnopqrst", "абвгдежзий клмнопрсту")` (20 Latin, 20 Cyrillic letters, U+0430–U+0439 and U+043A–U+0443) → `Reject(ScriptDrift { script: "latin", .. })`; the same with 19 letters on each side (`"abcdefghij klmnopqrs"`, `"абвгдежзий клмнопрст"`) → `Pass` | `<=` instead of `<` in step 2 of §4.7 |
| `a_lost_identifier_is_rejected` | `IdentifierGuard.check(SOURCE, &FAITHFUL.replace("from src/config.rs.", "from the config file."))` → `Reject(IdentifierMissing { token: "src/config.rs".into() })` | delete the path shape |
| `identifiers_have_the_five_shapes` | the table of §4.3 "Identifiers" | delete any one shape — its rows go red; skip the trim — the `(parse_config()),` and `«foo_bar»` rows go red; trim only A §6's eighteen — the D43 rows go red |
| `a_rewrite_that_curls_the_quotes_around_an_identifier_passes` (D43) | `IdentifierGuard.check("Set \"max_retries\" in 'src/config.rs' and call \u{60}parse_config\u{60} before the restart.", "Before the restart, set \u{201C}max_retries\u{201D} in \u{2018}src/config.rs\u{2019} and call parse_config.")` → `Pass` (the source's identifiers `max_retries`, `src/config.rs`, `parse_config` — the U+0060 backticks trimmed — are all in the candidate; lengths 80 and 79 code points) | trim only A §6's eighteen: the candidate's tokens keep their curly quotes → `Reject(IdentifierMissing { token: "max_retries".into() })` |
| `a_faithful_rewrite_passes_every_guard` | for every `g` in `default_guards()`: `g.check(SOURCE, FAITHFUL) == GuardOutcome::Pass` | skip the trim in `IdentifierGuard` (`src/config.rs.` ≠ `src/config.rs`) — or any over-strict change to any guard |
| `default_guards_are_in_spec_order` | `default_guards().iter().map(|g| g.name())` == `["placeholder", "numbers", "length-drift", "script", "identifier"]`; the names are unique; `LengthDriftGuard::default() == LengthDriftGuard { min: 0.6, max: 1.6 }`; `ScriptGuard::default() == ScriptGuard { max_delta_pp: 15.0, min_letters: 20 }` | swap two entries of `default_guards`; separately change one default |
| `reasons_render_specifically` (E0, extended) | add: `RejectReason::PlaceholderInvented { index: 7 }.to_string()` contains `"⟦7⟧"`; and `ScriptDrift { script: "latin", delta_pp: -100.0 }` renders `"-100.0"` | render `PlaceholderInvented` without its index |

One mutation per guard, for the report's table (the rows above carry
more): **Placeholder** — `a_duplicated_placeholder_is_rejected`, counts
→ presence. **Numbers** — `a_lost_version_number_is_rejected`, `.` ends
a token. **LengthDrift** — `cjk_length_is_counted_in_chars_not_bytes`,
bytes for chars. **Script** — `a_short_text_has_no_script_share`, no
`min_letters`. **Identifier** — `a_lost_identifier_is_rejected`, no path
shape. **`default_guards`** — `default_guards_are_in_spec_order`, two
entries swapped.

## §6 Acceptance criteria

- [ ] The five guards, `PlaceholderInvented`, `default_guards()` and the
      re-exports exist exactly as README §3.3 "E1-5 provides" lists them;
      `wipemark_core::default_guards` and the five types are reachable
      from outside the crate (a `use` in a doc-test or in the report's
      `cargo doc` output is evidence).
- [ ] Every §5 test is green, and every listed mutation was applied,
      seen red and reverted — the report's table: protection · mutation
      · test that went red.
- [ ] `outcome_reports_its_reason` and `reasons_render_specifically`
      (E0) still pass; no existing `Display` text changed.
- [ ] The doc comments of §4.1, §4.2 and §4.4–4.9 are in the code; the
      module doc no longer claims a caller in Layer A.
- [ ] No new dependency; `scripts/check-dep-direction.sh` green; the
      six gates of §0.5 green.
- [ ] `docs/architecture/layer-a.md` has "Guards" (§4.11).
- [ ] Report at `docs/plan/reports/E1-5-<date>.md`: the mutation table,
      the worktree and branch you used, the one-line `lib.rs` change the
      coordinator will merge, anything in E1-2's `letter_shares` that
      differed from §3.3, and what E4 should know (§7).

## §7 Out of scope

- **Calling the guards.** E4's selection loop (OV §4.4), its
  `Event::CandidateRejected`, and rendering a reason for a person (E7,
  through the catalogue). `Event::CandidateRejected.reason: String`
  (`wipemark-pipeline/src/lib.rs:91`) should become the structured
  `RejectReason` when E4 builds it — say so in the report for E4.
- **Making placeholders.** Protected spans and `⟦n⟧` are E4 (OV §4.2,
  A §2). These guards trust that the source they are handed already has
  them.
- **Cleaning the candidate first.** A guard compares the texts as given.
  A model that writes U+200B inside `snake_case` loses the identifier
  here; E4 should run Layer A over a candidate before its guards (OV
  §4.2 step 4 runs Layer A after the rewrite anyway).
- **The no-op guard** (a rewrite that changed nothing) and the
  reference's length penalty (`docs/sdd/layer-b-rewrite-reference.md`
  §5) — E4, beside the scorers.
- **`FakeEngine` against the guards.** Its rotation keeps every
  whitespace-separated token, so its output passes all five when the
  prompt is the source; that is E4's gate to pin (OV §10 E4), in a crate
  that may depend on both.
- **Per-script reasons** for Arabic or Hebrew (A §9 Q-A5), numbers as
  words, units, signs — owner decisions.
- **Tunable thresholds as Settings rows** — none in E1.

## §8 Basis and references

- **A** = `wipemark-core-layer-a-2026-09-21` (`ssd-docs/`): §6 (the five
  guards, their rules and reasons, `PlaceholderInvented`, `default_guards`
  order, "no caller but tests in E1; E4 wires them"), §5.5 (letter
  shares, CJK = Han ∪ Hiragana ∪ Katakana ∪ Hangul ∪ Bopomofo), §8 row
  "guards", §9 Q-A5.
- **OV** = `heretic-unmark-overview-decomposition-2026-09-07`: §3.3
  (guards "reused in Layer B as rejection criteria"), §4.2 (protected
  spans → `⟦n⟧`, restored after the rewrite), §4.4 (the selection loop),
  §10 E4 gate ("on `FakeEngine`: guards reject a lost placeholder,
  number, length").
- `docs/plan/README.md` §3.1, §3.3, §4 D24, D43 (the trim set), D44
  (G1–G12).
- `docs/sdd/layer-b-rewrite-reference.md` §5 (length penalty, no-op
  guard).
- **UAX #44** — General_Category `Nd`, `Lu`, `Ll`, `L*`:
  <https://www.unicode.org/reports/tr44/>; **UAX #24** — the Script
  property behind the shares: <https://www.unicode.org/reports/tr24/>.
- `CLAUDE.md` (at `497eafa`): `:85`, `:192`, `:220`, `:226`, `:1037`.
