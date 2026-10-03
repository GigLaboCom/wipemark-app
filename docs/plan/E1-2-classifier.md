# E1-2 — The classifier: one class per code point, and the context that keeps someone's orthography

|               |                                                                                                                                                                                                                                                                                                                                                                                                                      |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Series        | `docs/plan/` — the E1 series (Layer A, `wipemark-core`), document 2 of 7                                                                                                                                                                                                                                                                                                                                             |
| Spec scopes   | S1.2, S1.4 (A §4, §5.3 steps 1–3, §5.5 `TextStats`)                                                                                                                                                                                                                                                                                                                                                                  |
| Depends on    | E1-1 committed on `feat/e0-e6-shell`: the UCD 18.0.0 tables, `Script`, and the `pub(crate)` query functions of README §3.3                                                                                                                                                                                                                                                                                           |
| Unblocks      | E1-3 (`scrub::collect_hits` consumes `context::hits`), E1-5 (`ScriptGuard` consumes `stats::letter_shares`) — E1-5 may start in its worktree as soon as this document is committed                                                                                                                                                                                                                                  |
| Files touched | `crates/wipemark-core/src/class.rs`; `crates/wipemark-core/src/context.rs` (new); `crates/wipemark-core/src/stats.rs` (new); `crates/wipemark-core/src/lib.rs` (two `mod` lines); `crates/wipemark-core/src/report.rs` (doc comments on `TextStats` only); `fixtures/text/keep-*.txt` (12 new files, byte-exact); `fixtures/README.md` (one paragraph); `docs/architecture/layer-a.md` (the section *Classes and context*); `docs/plan/README.md` (status row); `docs/plan/reports/E1-2-<YYYY-MM-DD>.md` (new) |
| Size          | ~3 days for one agent; no network, no application launch                                                                                                                                                                                                                                                                                                                                                             |

**Notation used throughout.** A code point is `U+XXXX` with its UCD name
where it matters. An input text is a bracketed list of hexadecimal code
points, `[2764 FE0F 200D 1F525]`, which a test builds with
`char::from_u32`. An expected hit is `@at XXXX K` (kept by context,
confidence `LikelyFalsePositive`) or `@at XXXX F` (a finding: not kept,
confidence = the class ceiling `UnicodeClass::max_confidence()`); `at` is
the **byte** offset into the input. No invisible character, joiner,
selector, tag or bidi control appears raw anywhere in this document; do
not paste one into code either — write `'\u{200D}'`.

**Spec citations.** "A §n" is the E1 spec
`wipemark-core-layer-a-2026-09-21` (Watchword FILE; a working copy may
exist at `ssd-docs/wipemark-core-layer-a-2026-09-21.md`, gitignored). "OV
§n" is the overview `heretic-unmark-overview-decomposition-2026-09-07`.
"README" is `docs/plan/README.md`. Everything you need from A is
translated into this document; you do not need the Russian original.

## §0 Ground rules — identical in every document of this series

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

---

## §1 Goal

E1-1 leaves `wipemark-core` knowing every Unicode 18.0.0 property Layer A
needs and deciding nothing with it. This document adds the three things
every later part of Layer A stands on:

1. **`class_of(c)`** — which of the eleven `UnicodeClass`es a code point
   belongs to, if any, from UCD properties alone, with no context. One
   code point has at most one class, by construction *and* by a test that
   walks all 1,112,064 scalar values.
2. **`context::hits(text)`** — one `Hit` per finding-capable code point of
   the text, in source order, each either **kept by context** (here it
   carries someone's orthography or presentation, so it stays whatever
   the user's options say and is reported as `LikelyFalsePositive`) or
   **not** (it carries its class's confidence ceiling, and E1-3 decides
   what to do with it). This is the rule set of A §4.2: the presentation
   selector after an emoji, the ZWJ inside an emoji sequence, the joiners
   of Persian and Devanagari, the bidi marks of a right-to-left
   paragraph, the tags of a subdivision flag, the ideographic variation
   sequence, the standardized variant, the Mongolian selector, the Khmer
   inherent vowel, the Hangul filler — and the bidi override that is
   never kept.
3. **`TextStats::of(text)`** and **`letter_shares(text)`** — the cheap
   document statistics of A §5.5 that `inspect` reports (E1-3) and
   `ScriptGuard` compares (E1-5).

Nothing here removes, replaces or aggregates anything: no `Options`, no
`Action::Replace`, no report rows, no output string — those are E1-3's,
and they consume the hit stream exactly as §4.2 defines it. The hard
half of this document is not *finding* invisible characters (a binary
search does that) but **not mistaking somebody's spelling for a
watermark**. Every rule that keeps something is a protection, and every
protection in §5 is painted red before it counts.

## §2 Read first

| what | where (at `497eafa`) | why |
|---|---|---|
| The repository rules | `CLAUDE.md`, whole; in particular `:94-98` (delete the protection and watch it go red), `:201-204` (`wipemark-core` has zero dependencies), `:212-217` (the third shelf), `:1046-1049` (tests must be able to fail) | §0 defers to it |
| The module map and the interfaces | `docs/plan/README.md:159-194` (§3.2), `:202-255` (E1-1 provides), `:257-283` (**E1-2 provides — what you build**), `:285-321` (E1-3, your consumer), `:323-327` (E1-4), `:329-334` (E1-5, your other consumer) | fixed contract; do not re-sign anything |
| Decisions this document applies or serves | `docs/plan/README.md:353` (D3: homoglyph detection always runs), `:354` (D4: `suspicious` ⇔ a confidence ≥ `Probable` in findings ∪ kept), `:356` (D6: aggregation by codepoint, class, confidence, acted), `:367` (D17) and `:375` (D25: byte-exact fixtures, `fixtures/text/** -text`), `:369` (**D19**: Duployan and musical format controls are never findings), `:370` (**D20**: RTL evidence is a letter or mark), `:372` (D22: no halfwidth/fullwidth homoglyph clause), `:374` (**D24**: where tests live), `:384-389` (**D34–D39**: the context rules this document made exact, and the flag-only tag rule) | §4.0, §4.2.7, §5.1 |
| Open owner questions you must not answer | `docs/plan/README.md:404-411` (Q-A1 per-class overrides, Q-A2 embedding pairing, Q-A3 unassigned code points, Q-A5 Arabic/Hebrew ratios, Q-A7 the known false positives of §4.2.6, Q-A8 widening D39) | §7 |
| The taxonomy you extend | `crates/wipemark-core/src/class.rs:1-219`, whole | §3.1 |
| `TextStats` and the reports | `crates/wipemark-core/src/report.rs:47-73` | §4.3 |
| The crate root | `crates/wipemark-core/src/lib.rs:1-39`; `crates/wipemark-core/Cargo.toml:11-15` | two `mod` lines; no dependency |
| An independent spelling of the removal set | `crates/wipemark-i18n/src/tests.rs:289-337` (`no_message_carries_a_character_layer_a_would_strip` at `:299`, `stripped_by_layer_a` at `:316-337`) | **read, never touch** — §3.1 |
| Fixture rules | `fixtures/README.md:1-27` | §5.5 |
| What E0 built and left absent | `docs/architecture/skeleton.md:8-27`, `:118-130` | context |
| What E1-1 delivered | after E1-1: `crates/wipemark-core/src/tables.rs`, `src/script.rs`, `ucd/README.md`, `docs/architecture/layer-a.md`, `docs/plan/reports/E1-1-*.md` | §3.2 preflight |

## §3 What is true today

### 3.1 At `497eafa` (verified 2026-10-03)

| fact | evidence |
|---|---|
| `crates/wipemark-core/src/` holds five files, 650 lines: `class.rs` 219, `guard.rs` 131, `lib.rs` 39, `report.rs` 167, `vendor.rs` 94. No `ucd/`, `build.rs`, `tables.rs`, `script.rs`, `name.rs`, `context.rs`, `stats.rs`. | `wc -l crates/wipemark-core/src/*.rs` |
| The crate has no `[dependencies]` of any kind, by rule. | `crates/wipemark-core/Cargo.toml:11-13`; `scripts/check-dep-direction.sh:120-136` |
| `#![forbid(unsafe_code)]`; modules `class`, `guard`, `report`, `vendor` are `pub`; `Action`, `Confidence`, `UnicodeClass`, `UnicodeFinding` and `TextStats` are re-exported at the root. | `crates/wipemark-core/src/lib.rs:27`, `:29-32`, `:34`, `:36-38` |
| `Action { Remove, NormalizeToSpace, Keep }`. E1-3 changes it; **E1-2 does not touch it.** | `class.rs:19-28` |
| `Confidence` derives `PartialOrd, Ord` with `LikelyFalsePositive < Informational < Probable < Confirmed` — the order D4 compares against. | `class.rs:35-41` |
| `UnicodeClass` has 11 variants; `UnicodeClass::ALL` lists them in A §4.1 order, which **is** the first-claim order of `class_of`. | `class.rs:44-71`, `:75-87` |
| The variant doc comments are E0's approximations and are wrong in places against A §4.1: `BidiControl` omits U+061C (`:50`); `VariationSelector` omits the Mongolian selectors U+180B–180D, U+180F (`:54-55`); `ExoticSpace` omits U+1680 (`:59`); `DefaultIgnorable` says "minus the script-carrying ones (Arabic U+061C and U+0600–0605, Mongolian FVS, Khmer, Egyptian quadrat controls)" (`:65-67`) — U+061C is a `BidiControl`, U+0600–0605 and the Egyptian controls are in no class, the Mongolian selectors are `VariationSelector`, and the Khmer inherent vowels *are* `DefaultIgnorable` (kept by context). E1-2 rewrites them (§4.1). | `class.rs:46-70` |
| `as_str` ids, `default_action`, `max_confidence` (`Confirmed`: ZeroWidth, BidiControl, TagCharacter, Noncharacter, PrivateUse; `Probable`: ZeroWidthJoiner, VariationSelector, DefaultIgnorable, Homoglyph; `Informational`: SoftHyphen, ExoticSpace), `requires_aggressive`. Unchanged by E1-2. | `class.rs:90-104`, `:107-125`, `:131-144`, `:147-149` |
| Four unit tests exist and stay: `all_lists_every_variant_once_in_order`, `class_ids_are_unique`, `risky_classes_default_to_keep`, `confidence_orders_false_positives_lowest`. | `class.rs:166-219` |
| `TextStats { chars, words, latin_ratio, cyrillic_ratio, cjk_ratio, code_blocks, urls }`, `#[derive(Debug, Clone, Default, PartialEq)]`, no constructor, no field docs. Nothing outside `wipemark-core` reads it yet. | `report.rs:47-58`; `grep -rn TextStats crates apps` |
| `InspectReport` has no `kept` field yet (D5, E1-3's). | `report.rs:60-73` |
| `wipemark-i18n` spells the removal set a **second, independent time** (`stripped_by_layer_a`) so its catalogue gate cannot pass because the classifier is wrong. It is deliberately different from §4.1 (it fails on *any* U+200C/U+200D, which the classifier may keep in context, and omits U+061C, U+1680, the Mongolian selectors, U+2061–206F). **Do not edit it, do not make it call `class_of`.** | `crates/wipemark-i18n/src/tests.rs:316-337` |
| `fixtures/` contains only `README.md`; there is no `fixtures/text/` and no `.gitattributes` at the root. | `ls -la fixtures .gitattributes` |
| `docs/architecture/` has no `layer-a.md`. | `ls docs/architecture` |

### 3.2 What E1-1 will have delivered — the contract this document builds on

This document **assumes** E1-1 is committed and provides exactly the
README §3.3 block below (copied verbatim from `docs/plan/README.md:205-254`
as of 2026-10-03; E1-1 also adds the root re-exports `UNICODE_VERSION`
and `name_of` to `lib.rs`, README `:288-289`):

```rust
// lib.rs re-exports this one as wipemark_core::UNICODE_VERSION
pub const UNICODE_VERSION: &str;                       // read from the file headers, never typed: "18.0.0"

// tables.rs — every predicate is a binary search over sorted (u32, u32) ranges
pub(crate) fn is_default_ignorable(c: char) -> bool;   // DerivedCoreProperties: Default_Ignorable_Code_Point
pub(crate) fn is_bidi_control(c: char) -> bool;        // PropList: Bidi_Control
pub(crate) fn is_variation_selector(c: char) -> bool;  // PropList: Variation_Selector
pub(crate) fn is_join_control(c: char) -> bool;        // PropList: Join_Control
pub(crate) fn is_noncharacter(c: char) -> bool;        // PropList: Noncharacter_Code_Point
pub(crate) fn is_private_use(c: char) -> bool;         // UnicodeData gc=Co (First/Last pairs)
pub(crate) fn is_space_separator(c: char) -> bool;     // gc=Zs
pub(crate) fn is_format(c: char) -> bool;              // gc=Cf
pub(crate) fn is_letter(c: char) -> bool;              // gc=L*
pub(crate) fn is_mark(c: char) -> bool;                // gc=M*
pub(crate) fn is_decimal_digit(c: char) -> bool;       // gc=Nd
pub(crate) fn is_uppercase_letter(c: char) -> bool;    // gc=Lu  (IdentifierGuard's CamelCase, homoglyph case match)
pub(crate) fn is_lowercase_letter(c: char) -> bool;    // gc=Ll
pub(crate) fn is_rtl(c: char) -> bool;                 // Bidi_Class R or AL
pub(crate) fn is_emoji(c: char) -> bool;               // emoji-data: Emoji
pub(crate) fn is_emoji_presentation(c: char) -> bool;
pub(crate) fn is_emoji_modifier(c: char) -> bool;
pub(crate) fn is_emoji_modifier_base(c: char) -> bool;
pub(crate) fn is_emoji_component(c: char) -> bool;
pub(crate) fn is_han(c: char) -> bool;                 // Script=Han
pub(crate) fn script_of(c: char) -> Script;            // Scripts.txt, folded into Script (below)
pub(crate) fn is_standardized_variant(base: char, selector: char) -> bool;
pub(crate) fn decomposition(c: char) -> Option<&'static [char]>; // full NFKD mapping, recursion expanded at build time; Hangul NOT in the table
pub(crate) fn ccc(c: char) -> u8;                      // Canonical_Combining_Class, 0 when absent
pub(crate) fn compose(first: char, second: char) -> Option<char>; // primary composites minus Full_Composition_Exclusion; Hangul NOT in the table
pub(crate) fn confusable_target(c: char) -> Option<char>;          // single-char skeleton, filtered as A §3.2
pub(crate) fn confusables_with(target: char, script: Script) -> &'static [char]; // reverse index

// script.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Script { Latin, Cyrillic, Greek, Arabic, Hebrew, Han, Hiragana, Katakana, Hangul, Bopomofo,
                  /* the joining scripts of A §4.2 that are not already above, by UCD long name: */
                  Syriac, Nko, Mandaic, Adlam, HanifiRohingya, Devanagari, Bengali, Gurmukhi, Gujarati,
                  Oriya, Tamil, Telugu, Kannada, Malayalam, Sinhala, Tibetan, Myanmar, Khmer, Mongolian,
                  TaiTham, TaiViet, NewTaiLue, Balinese, Javanese, Sundanese, Batak, Lepcha, Limbu,
                  MeeteiMayek, KayahLi, Cham, Chakma, Sharada, Grantha, Kaithi, Modi, Takri, Tirhuta,
                  Siddham, Newa, Sogdian, Manichaean, OldUyghur,
                  Common, Inherited, Other }
impl Script { pub fn as_str(self) -> &'static str; /* the UCD long name: "Latin", "Hanifi_Rohingya", …;
                                                      "Other" for the 122 script values folded into Other plus Unknown */ }

// name.rs — re-exported as wipemark_core::name_of
pub fn name_of(c: char) -> Option<std::borrow::Cow<'static, str>>;
// Some(Borrowed(UCD name)) for a finding-capable code point that has one;
// Some(Owned("<private-use-E000>" / "<noncharacter-FDD0>" / "<reserved-E0080>")) for one without;
// None for anything that can never be a finding.
```

**What E1-2 calls**: `is_default_ignorable`, `is_bidi_control`,
`is_variation_selector`, `is_noncharacter`, `is_private_use`,
`is_space_separator`, `is_letter`, `is_mark`, `is_decimal_digit`,
`is_rtl`, `is_emoji`, `is_emoji_modifier`, `is_han`, `script_of`,
`is_standardized_variant`, and — in one test only — `is_format`. Paths:
`crate::tables::…`, `crate::script::Script`. Nothing else of E1-1 is
used; `name_of` is not involved in classification.

**Preflight (do this before writing code).**

```sh
git log --oneline -5                                   # an "E1-1: …" commit is on the branch
ls crates/wipemark-core/ucd/                           # the nine UCD files + README.md + SHA256SUMS
grep -n "pub(crate) fn" crates/wipemark-core/src/tables.rs   # every function listed above exists
grep -n "pub enum Script" -A 9 crates/wipemark-core/src/script.rs
cargo test -p wipemark-core --locked                   # green before you change anything
```

If E1-1 is not committed, set this document's status to *blocked
(E1-1 not landed)* and stop. If E1-1 landed with a different signature
or a missing predicate, the README §3.3 contract is still the contract:
call what exists, do **not** rename E1-1's functions, and record the
difference in your report; if a predicate this document needs is
genuinely absent, add it to `tables.rs` in E1-1's style (a sorted range
table generated by `build.rs` from a file already in `ucd/`), and say so.
Never add a UCD file.

## §4 Deliverables

### 4.0 Where this document says more than A §4

A §4.2 states the rules in one table. Read literally, some of them
contradict A's own idempotence argument (A §5.3: "context never relies on
a character that itself can be removed"), disagree with the 18.0.0 data,
or leave a carrier open that the rule was not meant to admit. The rows
below are part of this document's contract; each has a test in §5 that
goes red without it. They are listed here once so a reviewer can see
every place this document says more than A.

- **D34–D39, D19, D20** are coordinator decisions recorded in
  `docs/plan/README.md` §4 (`:369-370`, `:384-389`). Cite them by number
  in code comments and in the report.
- **P4, P6, P8** are this document's own precisions: places where A is
  loose and the document fixes one reading.
- D24 (where tests live) is applied in §5.1.

| # | rule | why | test |
|---|---|---|---|
| **D34** (README §4) | Every character a keep rule *looks at* — `prev`, `prev_kept`, `next`, a tag base — must itself be **not finding-capable** (`class_of(x).is_none()`). | A §5.3's idempotence argument assumes bases are never findings. Without D34: U+17B4 KHMER VOWEL INHERENT AQ is a mark of Khmer (a joining script) and would justify a preceding U+200C; U+180B is a mark of Mongolian and would justify a U+200D; a Hangul filler (gc=Lo, Script=Hangul) would justify the next filler, so a run of fillers after one syllable would all be kept. The first two break `clean(clean(x)) == clean(x)`. | `a_base_is_never_itself_a_finding`, `what_is_kept_stays_kept_on_the_output`, `a_partial_syllable_keeps_its_filler` |
| **D35** (README §4) | `prev_kept` is defined without `Options` (`hits` has none): it moves on every non-glue character that is not finding-capable, is kept by context, **or is an `ExoticSpace`**. A soft hyphen and every other not-kept finding leave it where it is. | `prev_kept` is "the last kept character" (A §4.2), and what is kept depends on `Options` that `hits` does not see. An exotic space is either kept (default) or replaced by U+0020 (`normalize_spaces`); U+0020 moves `prev_kept`, so the exotic space must move it too or a second pass under `normalize_spaces` sees a different `prev_kept`. A soft hyphen is either kept or removed; treating it as absent is right in both cases. | `what_is_kept_stays_kept_on_the_output` |
| **D36** (README §4) | A variation selector's base is **`prev`, the code point immediately before it in the source** — for VS15/VS16, VS1–VS14, IVS **and** the Mongolian free variation selectors. | A writes `prev` (never defined) for the VS rows and `prev_kept` for the FVS row. A variation sequence is a base followed *immediately* by its selector (Unicode 18.0 ch. 23.4). With `prev_kept` — which glue never moves — every selector in a run after one emoji, ideograph or Mongolian letter would be kept: two values per emoji, 240 values per ideograph, a byte channel ("variation-selector smuggling") the rule was never meant to admit. | `an_emoji_keeps_its_presentation_selector`, `an_ideograph_keeps_its_variation_sequence`, `a_mongolian_letter_keeps_its_selector` |
| **D37** (README §4) | An **ASCII** character (`#`, `*`, `0`–`9` are the only ASCII `Emoji=Yes`) is never a side of an emoji ZWJ and never a tag base (the tag half is now also implied by D39). It **is** a base for VS15/VS16. | `emoji-variation-sequences.txt` 18.0 defines `<0030 FE0E>` / `<0030 FE0F>` etc. as text/emoji style, so A's "including digits, `#`, `*`" stands for selectors. But no sequence in `emoji-zwj-sequences.txt` 18.0 (1,614 of them) has an ASCII element, a keycap joins only as a keycap sequence whose last code point U+20E3 is not `Emoji`, and UTS #51 Annex C allows only U+1F3F4 as a tag base: a ZWJ between two digits would otherwise be kept inside every number. | `an_ascii_character_is_never_a_joiner_side_or_a_tag_base` |
| **D38** (README §4) | A Hangul filler is kept when `prev_kept` **or `next`** is a Hangul letter. | U+115F HANGUL CHOSEONG FILLER stands for a *missing leading* consonant and therefore **starts** its syllable (Unicode 18.0 ch. 18.6.1: "placeholders for a missing choseong or jungseong in an incomplete syllable"); the KS X 1001 composed form likewise starts with U+3164. With `prev_kept` alone, a vowel-only syllable at the start of a word is broken. | `a_partial_syllable_keeps_its_filler` |
| **D39** (README §4) | Tag characters are kept **only in a valid emoji tag sequence per UTS #51 Annex C.1**: U+1F3F4 WAVING BLACK FLAG, then one or more tags from U+E0030–E0039 (TAG DIGIT ZERO … NINE) and U+E0061–E007A (TAG LATIN SMALL LETTER A … Z), then U+E007F CANCEL TAG, **32 code points at most** counting the flag and the terminator. Any other emoji + tags + U+E007F is a finding (`Confirmed`). Replaces A §4.2's "base with `Emoji=Yes`". | A's rule is UTS #51's *well-formedness* (ED-14a); Annex C makes only C.1 flag sequences *valid*, and the three RGI tag sequences in 18.0 are the flags of England, Scotland and Wales. Under A's rule any emoji followed by arbitrary tag text — the "ASCII smuggling" shape — would be kept and never make a text suspicious, which is the carrier this product most needs to catch. Coordinator's decision; the owner may widen it (Q-A8). | `a_flag_keeps_its_tags_and_a_loose_tag_does_not`, `an_emoji_cannot_smuggle_tags`, `keep-flag-tags.txt` |
| **D19** (README §4) | U+1BCA0–1BCA3 (Duployan shorthand format controls) and U+1D173–1D17A (musical beam/tie/slur/phrase controls) are **never findings**, excluded **by name** from `DefaultIgnorable`. | A §4.1 lists them among the `Cf` that are "not `Default_Ignorable`", but `DerivedCoreProperties.txt` 18.0.0 lists both ranges as `Default_Ignorable_Code_Point`, so the property alone would make them findings. A's intent stands — they format their own notation, like the Egyptian quadrat controls — so `claims(DefaultIgnorable)` subtracts them explicitly, with a comment saying why. | `script_format_controls_are_never_findings`, `every_class_has_exactly_the_members_unicode_18_gives_it` |
| **D20** (README §4) | A paragraph is right-to-left only through a code point with `Bidi_Class` R or AL **that is a letter or a mark** (`gc` L* or M*) — never through a `Bidi_Control`, a digit, a symbol or punctuation. | U+200F RIGHT-TO-LEFT MARK has `Bidi_Class=R` and U+061C ARABIC LETTER MARK has `Bidi_Class=AL` (UnicodeData 18.0.0). Counted as evidence, a stray RLM in an English paragraph would make the paragraph RTL and protect itself. No R/AL letter or mark is finding-capable in 18.0.0, so the evidence always survives a clean. | `a_stray_rlm_does_not_protect_itself` |
| **P4** | The "or the base is Han" clause covers **IVS (U+E0100–E01EF) only**; VS1–VS14 after an ideograph need a `StandardizedVariants.txt` pair. | A's reason for the clause is the Ideographic Variation Database, which registers sequences with VS17–VS256 only (UTS #37; every selector in IVD 2025-07-14 is E0100–E011F). CJK compatibility variants with VS1–VS3 are all in `StandardizedVariants.txt` (1,002 Han pairs in 18.0.0). | `an_ideograph_keeps_its_variation_sequence` |
| **P6** | "A letter or mark **of one joining script**": `prev_kept`'s script is its **effective** script — a `Script=Inherited` character takes the effective script of the `prev_kept` before it (UAX #24); `next` must carry its **own** script (an `Inherited` `next` does not qualify). | A says "the script of an `Inherited` mark is its base's" without saying which side. For `prev_kept` the base is earlier text (Arabic letter + U+064E FATHA + U+200C + letter is ordinary). For `next` the "base" of an `Inherited` mark after a joiner would be the joiner itself. | `a_mark_carries_the_script_of_its_base` |
| **P8** | A paragraph ends at U+000A only. U+000D, U+0085, U+2028, U+2029 do not end one. | A says "between `\n`". `\r\n` text is unaffected; a text that uses only `\r` or U+2029 is one paragraph, which errs toward keeping marks, never toward removing them. | `a_bidi_mark_is_typography_beside_rtl_and_a_carrier_without_it` (paragraph half) |

### 4.1 `class.rs` — `class_of`, `JOINING_SCRIPTS`, and the variant docs

**Signatures** (README §3.3, exactly):

```rust
pub fn class_of(c: char) -> Option<UnicodeClass>;     // context-free membership, first claim in A §4.1 order;
                                                      // never returns Homoglyph (that needs a word; E1-4)
pub(crate) const JOINING_SCRIPTS: &[Script];          // A §4.2, with the comment saying why a list
```

**Structure.** Each class's *definition* is a private predicate, and
`class_of` returns the first class in `UnicodeClass::ALL` order whose
predicate claims the code point. Keeping the definitions separate from
the order is what makes "at most one class" testable: the walk test
(§5.2) evaluates every predicate on every code point and asserts that at
most one claims it — a property of the definitions, not of `class_of`.

```rust
fn claims(class: UnicodeClass, c: char) -> bool {
    match class {
        UnicodeClass::ZeroWidth => matches!(c, '\u{200B}' | '\u{200C}' | '\u{2060}' | '\u{FEFF}'),
        UnicodeClass::ZeroWidthJoiner => c == '\u{200D}',
        UnicodeClass::BidiControl => tables::is_bidi_control(c),
        UnicodeClass::TagCharacter => ('\u{E0000}'..='\u{E007F}').contains(&c),
        UnicodeClass::VariationSelector => tables::is_variation_selector(c),
        UnicodeClass::SoftHyphen => c == '\u{00AD}',
        UnicodeClass::ExoticSpace => c != ' ' && tables::is_space_separator(c),
        UnicodeClass::Noncharacter => tables::is_noncharacter(c),
        UnicodeClass::PrivateUse => tables::is_private_use(c),
        UnicodeClass::DefaultIgnorable => {
            // D19: Default_Ignorable in UCD 18.0.0, but they format their own
            // notation (Duployan shorthand; musical beams, ties, slurs, phrases)
            // the way the Egyptian quadrat controls do — never findings, by name.
            let own_notation = ('\u{1BCA0}'..='\u{1BCA3}').contains(&c)
                || ('\u{1D173}'..='\u{1D17A}').contains(&c);
            !own_notation
                && (('\u{FFF9}'..='\u{FFFB}').contains(&c)
                    || (tables::is_default_ignorable(c)
                        // the subtraction: everything a class above already claims
                        && !UnicodeClass::ALL
                            .iter()
                            .take_while(|&&k| k != UnicodeClass::DefaultIgnorable)
                            .any(|&k| claims(k, c))))
        }
        // Needs a word (A §5.4) — E1-4's homoglyph::hits, never class_of.
        UnicodeClass::Homoglyph => false,
    }
}

pub fn class_of(c: char) -> Option<UnicodeClass> {
    if u32::from(c) < 0xA0 {
        return None; // optional fast path: nothing below U+00A0 is finding-capable (A §2); the walk test proves it
    }
    UnicodeClass::ALL.into_iter().find(|&class| claims(class, c))
}
```

**The membership table, Unicode 18.0.0** (A §4.1; verified against the
18.0.0 files, 2026-10-03). Order = `UnicodeClass::ALL` = first claim.

| # | class | definition | E1-1 predicate | members in 18.0.0 | count |
|---|---|---|---|---|---|
| 1 | `ZeroWidth` | the literal set {200B, 200C, 2060, FEFF} | — | 200B–200C, 2060, FEFF | 4 |
| 2 | `ZeroWidthJoiner` | {200D} | — | 200D | 1 |
| 3 | `BidiControl` | `Bidi_Control` (PropList) | `is_bidi_control` | 061C, 200E–200F, 202A–202E, 2066–2069 | 12 |
| 4 | `TagCharacter` | the Tags block, E0000–E007F, assigned or reserved | literal range | E0000–E007F | 128 |
| 5 | `VariationSelector` | `Variation_Selector` (PropList) — includes the Mongolian free variation selectors | `is_variation_selector` | 180B–180D, 180F, FE00–FE0F, E0100–E01EF | 260 |
| 6 | `SoftHyphen` | {00AD} | — | 00AD | 1 |
| 7 | `ExoticSpace` | `gc=Zs` minus U+0020 | `is_space_separator` | 00A0, 1680, 2000–200A, 202F, 205F, 3000 | 16 |
| 8 | `Noncharacter` | `Noncharacter_Code_Point` (PropList) | `is_noncharacter` | FDD0–FDEF; U+nFFFE and U+nFFFF for every plane n = 0…16 | 66 |
| 9 | `PrivateUse` | `gc=Co` (UnicodeData First/Last pairs) | `is_private_use` | E000–F8FF, F0000–FFFFD, 100000–10FFFD | 137,468 |
| 10 | `DefaultIgnorable` | ((`Default_Ignorable_Code_Point` minus classes 1–9) ∪ {FFF9, FFFA, FFFB}) minus {1BCA0–1BCA3, 1D173–1D17A} (D19) | `is_default_ignorable` + `claims` of 1–9 | 034F, 115F–1160, 17B4–17B5, 180E, 2061–2065, 206A–206F, 3164, FFA0, FFF0–FFFB, E0080–E00FF, E01F0–E0FFF | 3,759 |
| 11 | `Homoglyph` | never returned by `class_of` | — | — | 0 |
| | | | | **finding-capable in all** | **141,715** |

Notes the implementer must get right:

- **The BOM rule is not `class_of`'s.** `class_of('\u{FEFF}')` is
  `Some(ZeroWidth)` wherever it occurs. "U+FEFF at byte 0 is a byte order
  mark: not a finding, and it stays in the output" (A §4.1) is applied by
  `hits` (§4.2), because only `hits` knows the position.
- **Reserved code points.** Unassigned code points are not findings (A §2,
  Q-A3) *unless* they are `Default_Ignorable_Code_Point`: U+2065,
  U+FFF0–FFF8, U+E0080–E00FF, U+E01F0–E0FFF are reserved and DICP, so they
  are `DefaultIgnorable`; U+E0000 and U+E0002–E001F are reserved and fall
  in the Tags block, so they are `TagCharacter`.
- **The interlinear annotation characters** U+FFF9 INTERLINEAR ANNOTATION
  ANCHOR, U+FFFA … SEPARATOR, U+FFFB … TERMINATOR are `gc=Cf` and are
  excluded from DICP by name (`DerivedCoreProperties.txt` header), but
  nothing renders them; A adds them to `DefaultIgnorable` explicitly.
- **Never findings — the script format controls.** Of the 170 `gc=Cf`
  code points in 18.0.0, exactly **41** have no class:
  - the `Prepended_Concatenation_Mark` set U+0600–0605, U+06DD, U+070F,
    U+0890–0891, U+08E2, U+110BD, U+110CD, and the Egyptian hieroglyph
    format controls U+13430–1343F (29 code points) — UCD itself excludes
    these from DICP by name;
  - the Duployan shorthand format controls U+1BCA0–1BCA3 and the musical
    symbol format controls U+1D173–1D17A (12 code points) — DICP in UCD,
    excluded by name here (D19).

  They *render with* or *format* their own script or notation (the Arabic
  number sign sits over the digits that follow it; a quadrat control
  arranges hieroglyphs; a beam control joins notes), and Layer A looks
  for the invisible. A keeps no context rule for them because none is
  needed: they are not findings at all.
- **Not findings either** (A §2): C0 and C1 controls (including `\t`,
  `\n`, `\r`), U+2028 LINE SEPARATOR, U+2029 PARAGRAPH SEPARATOR, U+0085,
  every letter, mark, digit, symbol and punctuation, every unassigned code
  point that is not DICP.
- Use `u32::from(c)` rather than `c as u32` in new code.

**`JOINING_SCRIPTS`** — exactly these 44, in this order (A §4.2):

```rust
/// Scripts in which U+200C ZERO WIDTH NON-JOINER and U+200D ZERO WIDTH
/// JOINER are spelling, not decoration. Between two letters or marks of
/// one of these scripts a joiner is kept and reported as a likely false
/// positive (A §4.2): Persian writes the non-joiner inside words, the
/// Indic scripts use the joiner after a virama to choose a half-form or a
/// conjunct.
///
/// A list, not a property, because the UCD has no property for "uses
/// ZWJ/ZWNJ orthographically". `Joining_Type` (ArabicShaping.txt) covers
/// the cursive scripts and says nothing about the Indic ones;
/// `Indic_Syllabic_Category` (IndicSyllabicCategory.txt) covers viramas
/// and says nothing about Arabic. Neither alone is this set, and their
/// union needs two UCD files this crate does not commit.
///
/// Latin, Cyrillic, Greek and the CJK scripts are absent on purpose: a
/// ZWNJ between two Latin letters is the classic carrier, not
/// orthography. A script missing here costs its writers false positives;
/// add it by name, with a test.
pub(crate) const JOINING_SCRIPTS: &[Script] = &[
    Script::Arabic, Script::Syriac, Script::Nko, Script::Mandaic, Script::Adlam,
    Script::HanifiRohingya, Script::Devanagari, Script::Bengali, Script::Gurmukhi,
    Script::Gujarati, Script::Oriya, Script::Tamil, Script::Telugu, Script::Kannada,
    Script::Malayalam, Script::Sinhala, Script::Tibetan, Script::Myanmar, Script::Khmer,
    Script::Mongolian, Script::TaiTham, Script::TaiViet, Script::NewTaiLue, Script::Balinese,
    Script::Javanese, Script::Sundanese, Script::Batak, Script::Lepcha, Script::Limbu,
    Script::MeeteiMayek, Script::KayahLi, Script::Cham, Script::Chakma, Script::Sharada,
    Script::Grantha, Script::Kaithi, Script::Modi, Script::Takri, Script::Tirhuta,
    Script::Siddham, Script::Newa, Script::Sogdian, Script::Manichaean, Script::OldUyghur,
];
```

(`rustfmt` will lay this out its own way; the content and order are what
matter.)

**Variant doc comments** — replace `class.rs:46-70` with these (keep the
module doc `:1-15` as it is):

| variant | new doc comment |
|---|---|
| `ZeroWidth` | U+200B, U+200C, U+2060, U+FEFF. A U+FEFF at byte 0 is a byte order mark: never a finding, and it stays in the output. U+200C is kept between two letters of one joining script (`JOINING_SCRIPTS`). |
| `ZeroWidthJoiner` | U+200D — kept inside an emoji ZWJ sequence and between two letters of one joining script. |
| `BidiControl` | `Bidi_Control`: U+061C, U+200E–200F, U+202A–202E, U+2066–2069. Marks, isolates and embeddings are kept in a paragraph that has right-to-left letters; the overrides U+202D and U+202E never are (Trojan Source). |
| `TagCharacter` | U+E0000–E007F, the Tags block — kept only inside a valid subdivision-flag sequence: U+1F3F4, digit/lowercase tags, U+E007F, at most 32 code points (UTS #51 Annex C.1; D39). |
| `VariationSelector` | `Variation_Selector`: U+FE00–FE0F, U+E0100–E01EF and the Mongolian free variation selectors U+180B–180D, U+180F — kept directly after a base they are defined for. |
| `SoftHyphen` | U+00AD. |
| `ExoticSpace` | `General_Category=Zs` other than U+0020: U+00A0, U+1680, U+2000–200A, U+202F, U+205F, U+3000. |
| `Noncharacter` | Noncharacters: U+FDD0–FDEF, U+xFFFE/U+xFFFF. (unchanged) |
| `PrivateUse` | Private use areas, including planes 15–16. (unchanged) |
| `DefaultIgnorable` | `Default_Ignorable_Code_Point` not claimed by a class above, plus the interlinear annotation characters U+FFF9–FFFB (not default-ignorable, never rendered). Includes the Hangul fillers, the Khmer inherent vowels and U+180E, each kept in context. The script format controls are in no class: U+0600–0605, U+06DD, U+070F, U+0890–0891, U+08E2, U+110BD, U+110CD, U+13430–1343F (not default-ignorable in UCD), and U+1BCA0–1BCA3, U+1D173–1D17A (default-ignorable in UCD, excluded by name because they format their own notation). |
| `Homoglyph` | Latin, Cyrillic and Greek confusables, from `confusables.txt` (fullwidth Latin is `Script=Latin`; D22 dropped the halfwidth/fullwidth block clause). Needs a word, so [`class_of`] never returns it. |

### 4.2 `context.rs` — the two pre-passes, the keep rules, the hit stream

**Types and signature** (README §3.3, exactly):

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Hit {
    pub at: usize,                // byte offset into the SOURCE text
    pub c: char,
    pub class: UnicodeClass,
    pub confidence: Confidence,   // class.max_confidence(), or LikelyFalsePositive when kept_by_context
    pub kept_by_context: bool,    // A §4.2: orthography or presentation — kept whatever the Options say
    pub replacement: Option<char>,// None here; E1-4 fills it for Homoglyph
}
pub(crate) fn hits(text: &str) -> Vec<Hit>;          // every finding-capable code point except Homoglyph,
                                                      // in source order, context applied; a BOM at 0 is not a hit
```

`hits` has no caller outside tests until E1-3, and `Hit`'s fields are
read nowhere else yet: give `Hit` and `hits` the repository's existing
pattern, `#[cfg_attr(not(test), allow(dead_code, reason = "first caller:
E1-3 scrub::collect_hits"))]` (compare
`apps/wipemark-app/src/clipboard.rs:130-133`). E1-3 deletes the
attribute. Do not use a crate-wide `allow`.

#### 4.2.1 Vocabulary

| term | definition |
|---|---|
| **finding-capable** | `class_of(c).is_some()`. |
| **glue** | a code point whose class is `ZeroWidthJoiner`, `VariationSelector` or `TagCharacter` — U+200D, all 260 variation selectors (Mongolian included), all of U+E0000–E007F. Glue never moves `prev_kept`, whether it is kept or not. U+200C is **not** glue. U+20E3 is not glue. |
| **`prev`** | the code point immediately before the current one in the source, whatever it is. `None` at the start of the text. |
| **`prev_kept`** (the *anchor*) | the last code point before the current one that is (a) not glue and (b) not finding-capable, or kept by context, or an `ExoticSpace` (D35). A leading BOM never becomes `prev_kept`. `None` until the first such code point. |
| **effective script of `prev_kept`** | `script_of(prev_kept)` unless that is `Script::Inherited`, in which case it is the effective script of the `prev_kept` before it (P6). `None` if there is none. |
| **`next`** | the code point immediately after the current one in the source (`text[at + c.len_utf8()..].chars().next()`). `None` at the end. It may itself be a finding; D34 then makes it disqualify. |
| **emoji base** | `x` is not ASCII **and** (`is_emoji(x)` or `is_emoji_modifier(x)`) **and** `x` is not finding-capable (D34, D37). In 18.0.0 every `Emoji_Modifier` is also `Emoji`, and no `Emoji` code point is finding-capable; keep both clauses anyway — they cost nothing and survive a version bump. |
| **letter of script S** | `is_letter(x)` and `script_of(x) == S` and `x` is not finding-capable. |
| **paragraph** | the code points between two U+000A, or between the text's start/end and the nearest U+000A (P8). The U+000A itself belongs to no paragraph's evidence. |

#### 4.2.2 Pre-pass 1 — right-to-left paragraphs (A §5.3 step 1)

One `bool` per paragraph, in order; paragraph *k* is right-to-left iff
it contains at least one code point `x` with `is_rtl(x)` (Bidi_Class R or
AL) **and** `is_letter(x) || is_mark(x)` (D20). The number of paragraphs
is the number of U+000A plus one.

```rust
fn rtl_paragraphs(text: &str) -> Vec<bool> {
    let mut rtl = vec![false];
    for c in text.chars() {
        if c == '\n' {
            rtl.push(false);
        } else if tables::is_rtl(c) && (tables::is_letter(c) || tables::is_mark(c)) {
            if let Some(last) = rtl.last_mut() { *last = true; }
        }
    }
    rtl
}
```

What D20 excludes, in 18.0.0: the `Bidi_Control`s U+200F (R) and U+061C
(AL), which would otherwise protect themselves; U+070F SYRIAC
ABBREVIATION MARK (`Cf`, AL); and R/AL digits, symbols and punctuation
(436 code points, e.g. U+061B ARABIC SEMICOLON). U+0600 ARABIC NUMBER SIGN
and the Arabic-Indic digits are `Bidi_Class=AN`, not AL, and never
counted. U+0640 ARABIC TATWEEL (`gc=Lm`, `Script=Common`, AL) does count.
No R/AL letter or mark is finding-capable, so the evidence survives every
clean (§4.2.7).

#### 4.2.3 Pre-pass 2 — valid flag tag sequences (A §5.3 step 2, D39)

A tag character is kept only inside a **valid emoji tag sequence** in the
sense of UTS #51 Annex C.1 (D39). That is, with no other code point
between the parts:

1. the **tag base** U+1F3F4 WAVING BLACK FLAG — exactly that code point,
   immediately before the first tag. No other emoji, no U+FE0F between
   the flag and the tags, no modifier;
2. one or more **tag specs**, contiguous, each in U+E0030–E0039 (TAG
   DIGIT ZERO … TAG DIGIT NINE) or U+E0061–E007A (TAG LATIN SMALL LETTER
   A … Z);
3. U+E007F CANCEL TAG;
4. **at most 32 code points** in all, counting the flag and the
   terminator — so at most 30 tag specs.

How the pre-pass reads the text: find each maximal run of code points in
U+E0020–E007E (TAG SPACE … TAG TILDE). The run and the U+E007F that
immediately follows it form a valid sequence iff the code point
immediately before the run is U+1F3F4, **every** code point of the run is
a digit or lowercase tag, the run is followed immediately by U+E007F, and
the run is at most 30 long. Then the run and its terminator are one kept
range; otherwise none of it is kept. The pre-pass returns those byte
ranges (tags and terminator, not the base), sorted and non-overlapping.

Everything else in U+E0000–E007F is outside every range and is a finding
(`TagCharacter`, `Confirmed`): U+E0001 LANGUAGE TAG, reserved tag code
points, capital-letter, space and punctuation tags (U+E0020–E002F,
U+E003A–E0060, U+E007B–E007E — "reserved for future extensions" in UTS
#51), a run after any base other than U+1F3F4 (a smiley, another flag, a
digit, a letter), a run with no terminator, a terminator with no run
before it, a run broken by any other code point, a run of 31 or more.
Annex C.1 also requires the tag text to be a CLDR subdivision id
(`gbsct`, `usca`, …); the crate carries no CLDR data, so that part is not
checked — see §4.2.6.

```rust
const TAG_SEQUENCE_MAX: usize = 32; // UTS #51 Annex C, flag + tags + U+E007F

fn valid_tag_sequences(text: &str) -> Vec<std::ops::Range<usize>> {
    let mut out = Vec::new();
    let mut prev: Option<char> = None;
    // (byte offset of the first tag, base is U+1F3F4, every tag so far is [0-9a-z], tags so far)
    let mut run: Option<(usize, bool, bool, usize)> = None;
    for (at, c) in text.char_indices() {
        let spec = ('\u{E0020}'..='\u{E007E}').contains(&c);
        let flag_spec = ('\u{E0030}'..='\u{E0039}').contains(&c) || ('\u{E0061}'..='\u{E007A}').contains(&c);
        run = match run {
            None if spec => Some((at, prev == Some('\u{1F3F4}'), flag_spec, 1)),
            Some((start, base, ok, n)) if spec => Some((start, base, ok && flag_spec, n + 1)),
            Some((start, base, ok, n)) => {
                if c == '\u{E007F}' && base && ok && n + 2 <= TAG_SEQUENCE_MAX {
                    out.push(start..at + c.len_utf8());
                }
                None
            }
            None => None,
        };
        prev = Some(c);
    }
    out
}
```

The main pass asks "is byte `at` inside a range" — use a forward cursor
or `binary_search_by`, never a linear scan per tag.

#### 4.2.4 The main pass (A §5.3 step 3)

One pass over `text.char_indices()`, left to right, O(n), no allocation
per code point. For each `(at, c)`:

1. `class = class_of(c)`. If `None`, no hit.
2. If `c == '\u{FEFF}'` and `at == 0`: no hit (the BOM).
3. Otherwise decide `kept` with the rule for `class` (table below), and
   push exactly one `Hit { at, c, class, confidence, kept_by_context:
   kept, replacement: None }` with `confidence =
   Confidence::LikelyFalsePositive` if `kept`, else
   `class.max_confidence()`.
4. Update the state, **after** the decision: if `c` is not glue, is not
   the leading BOM, and (`class` is `None`, or `kept`, or `class ==
   Some(ExoticSpace)`), then `prev_kept = c` and, unless `script_of(c) ==
   Script::Inherited`, `anchor_script = script_of(c)`. If `c == '\n'`,
   advance the paragraph index. Always `prev = c`.

**The keep rules** — the whole of A §4.2, made exact. "Kept" means
`kept_by_context = true`. Any rule whose required side is `None` (start
or end of text) is false.

| class | code points | kept when | kept (positive) | not kept (negative) | A |
|---|---|---|---|---|---|
| `VariationSelector` | U+FE0E VS15, U+FE0F VS16 | `prev` is an `Emoji=Yes` code point that is not finding-capable — **ASCII included**: `#`, `*`, `0`–`9` are keycap bases and have defined text/emoji styles (D37 does not apply here) | `[2696 FE0F]` SCALES + VS16 → `@3 FE0F K`; `[0031 FE0F 20E3]` keycap one → `@1 K`; `[0031 FE0E]` → `@1 K`; `[2122 FE0F]` TRADE MARK SIGN → `@3 K` | `[0061 FE0F]` → `@1 F`; `[8FBB FE0F]` (ideograph, not emoji) → `@3 F`; `[2764 FE0F FE0F]` → `@3 K`, `@6 F` (D36: the second selector's `prev` is the first) | §4.2 row 1 |
| `VariationSelector` | U+FE00–FE0D VS1–VS14 | `prev` is not finding-capable and `is_standardized_variant(prev, c)` | `[2229 FE00]` INTERSECTION *with serifs* → `@3 K`; `[0030 FE00]` DIGIT ZERO *short diagonal stroke form* → `@1 K`; `[6F22 FE00]` (CJK compatibility variant) → `@3 K` | `[2282 FE00]` SUBSET OF + VS1 — **not** in `StandardizedVariants.txt` 18.0.0 → `@3 F`; `[0041 FE00]` → `@1 F`; `[8FBB FE00]` (Han, but VS1 is not an IVS — P4) → `@3 F` | §4.2 row 2 |
| `VariationSelector` | U+E0100–E01EF (IVS, VS17–VS256) | `prev` is not finding-capable and (`is_han(prev)` or `is_standardized_variant(prev, c)`) — every IVS after an ideograph is legal because the IVD is not in the UCD | `[8FBB E0100]` (U+8FBB with VS17, registered in IVD as Adobe-Japan1 CID+3056) → `@3 K` | `[0061 E0100]` → `@1 F`; `[8FBB E0100 E0101]` → `@3 K`, `@7 F` (D36) | §4.2 row 2 |
| `VariationSelector` | U+180B–180D, U+180F (Mongolian FVS) | `prev` is a letter of Mongolian (D36: `prev`, not `prev_kept`) | `[1820 180B]` MONGOLIAN LETTER A, second form → `@3 K` | `[0061 180B]` → `@1 F`; `[1820 180B 180C]` → `@3 K`, `@6 F` | §4.2 row 3 |
| `DefaultIgnorable` | U+180E MONGOLIAN VOWEL SEPARATOR | `prev_kept` is a letter of Mongolian | `[182C 1820 1837 180E 1820]` (QA A RA, MVS, A) → `@9 K` | `[0020 180E 1820]` → `@1 F` | §4.2 row 3 |
| `ZeroWidthJoiner` | U+200D | **(emoji)** `prev_kept` and `next` are both emoji bases; **or (joining)** the joining rule below | `[2764 FE0F 200D 1F525]` HEART ON FIRE → `@3 FE0F K`, `@6 200D K` (the ZWJ's `prev_kept` is U+2764 because U+FE0F is glue); `[1F44D 200D 1F3FB]` → `@4 K` (a modifier is an emoji base; no RGI sequence does this and it renders visibly as a swatch, so it is no stealthy carrier — kept as A writes it) | `[1F469 200D 0020]` → `@4 F`; `[0031 200D 0032]` → `@1 F` (D37); `[0031 FE0F 20E3 200D 1F525]` → `@1 FE0F K`, `@7 200D F` (`prev_kept` is U+20E3, not an emoji) | §4.2 row 4 |
| `ZeroWidthJoiner`, and `ZeroWidth` for U+200C only | U+200D, U+200C | **(joining)** `prev_kept` is a letter or mark (`is_letter` or `is_mark`), not finding-capable, whose effective script S is in `JOINING_SCRIPTS`; and `next` is a letter or mark, not finding-capable, with `script_of(next) == S` (D34, P6) | `[0645 06CC 200C 0631 0648 0645]` Persian *mi-ravam* → `@4 K`; `[0915 094D 200D 0937]` Devanagari KA VIRAMA ZWJ SSA → `@6 K` (`prev_kept` U+094D is `Script=Devanagari`, not `Inherited`); `[0DC1 0DCA 200D 0DBB 0DD3]` Sinhala *shri* → `@6 K`; `[0628 064E 200C 0647]` → `@4 K` (U+064E FATHA is `Inherited`, takes Arabic from U+0628) | `[0061 200C 0062]` → `@1 F`; `[0645 200C 0915]` (two scripts) → `@2 F`; `[0628 200C 064E]` (`next` is `Inherited`) → `@2 F`; `[1780 200C 17B4]` → `@3 F` (D34: `next` is a finding) | §4.2 row 5 |
| `TagCharacter` | U+E0000–E007F | `at` lies in a range of pre-pass 2 — a valid flag sequence (D39) | `[1F3F4 E0067 E0062 E0073 E0063 E0074 E007F]` flag of Scotland → `@4 … @24` all `K`; U+1F3F4 + 30 × U+E0061 + U+E007F (32 code points) → all 31 `K` | `[1F600 E0068 E0069 E0074 E007F]` (a smiley carrying *hit*) → all `F`; `[1F3F3 E0067 E0062 E0073 E0063 E0074 E007F]` (white flag base) → all `F`; `[1F3F4 E0047 E0042 E0053 E0043 E0054 E007F]` (capital tags) → all `F`; `[1F3F4 E0067 E0062 E0073 E0063 E0074 0020]` (no terminator) → all `F`; U+1F3F4 + 31 × U+E0061 + U+E007F (33) → all `F` | §4.2 row 6, replaced by D39 |
| `DefaultIgnorable` | U+17B4 KHMER VOWEL INHERENT AQ, U+17B5 … AA | `prev_kept` is a letter of Khmer | `[1780 17B4]` KA + AQ → `@3 K` | `[0061 17B4]` → `@1 F`; `[0020 17B4]` → `@1 F` | §4.2 row 7 |
| `DefaultIgnorable` | U+115F HANGUL CHOSEONG FILLER, U+1160 JUNGSEONG FILLER, U+3164 HANGUL FILLER, U+FFA0 HALFWIDTH HANGUL FILLER | `prev_kept` is a letter of Hangul **or `next` is a letter of Hangul** (D38). The four fillers are themselves `gc=Lo`, `Script=Hangul` — and finding-capable, so never "a letter of Hangul" (D34) | `[1100 1160]` initial-only syllable → `@3 K`; `[0020 115F 1161]` vowel-only syllable at a word start → `@1 K`; `[0020 3164 3131 314F 3164]` KS X 1001 composed syllable → `@1 K`, `@10 K` | `[0061 3164 0062]` → `@1 F`; `[AC00 3164 3164]` → `@3 K`, `@6 F` (the second filler's `prev_kept` is the first, a finding) | §4.2 row 8 |
| `BidiControl` | U+200E, U+200F, U+061C (marks); U+2066–2069 (isolates); U+202A, U+202B, U+202C (embeddings and their pop) | the current paragraph is right-to-left (pre-pass 1) | `[05E9 05DC 05D5 05DD 0020 200E 0052 0075 0073 0074 200E]` (Hebrew *shalom*, LRM, *Rust*, LRM) → `@9 K`, `@16 K` | the same marks in a paragraph with no R/AL letter or mark → `F`; `[0048 0069 200F …]` — the RLM is R but is a `Bidi_Control`, not a letter, so not evidence (D20) → `F` | §4.2 row 9 |
| `BidiControl` | U+202D LEFT-TO-RIGHT OVERRIDE, U+202E RIGHT-TO-LEFT OVERRIDE | **never** — the Trojan Source vector (CVE-2021-42574); no orthography needs one | — | `@… 202E F` even in a Hebrew paragraph | §4.2 row 10 |
| `ZeroWidth` | U+200B, U+2060, U+FEFF (not at byte 0) | never | — | always `F` (`Confirmed`) | §4.1 |
| `SoftHyphen`, `ExoticSpace`, `Noncharacter`, `PrivateUse`, other `DefaultIgnorable` | — | never by context. A soft hyphen or an exotic space may still be *kept by `Options`* in E1-3; that is not `kept_by_context` | — | always `F` | §4.1, §5.2 |

The decision as code (a sketch to implement, not to paste blindly):

```rust
fn kept(class: UnicodeClass, c: char, at: usize, s: &State, next: Option<char>) -> bool {
    match class {
        UnicodeClass::ZeroWidth => c == '\u{200C}' && joins_letters(s, next),
        UnicodeClass::ZeroWidthJoiner => (emoji_base(s.prev_kept) && emoji_base(next)) || joins_letters(s, next),
        UnicodeClass::BidiControl => !matches!(c, '\u{202D}' | '\u{202E}') && s.paragraph_is_rtl(),
        UnicodeClass::TagCharacter => s.in_tag_sequence(at),
        UnicodeClass::VariationSelector => selector_has_its_base(c, s.prev),
        UnicodeClass::DefaultIgnorable => match c {
            '\u{17B4}' | '\u{17B5}' => letter_of(s.prev_kept, Script::Khmer),
            '\u{115F}' | '\u{1160}' | '\u{3164}' | '\u{FFA0}' => {
                letter_of(s.prev_kept, Script::Hangul) || letter_of(next, Script::Hangul)
            }
            '\u{180E}' => letter_of(s.prev_kept, Script::Mongolian),
            _ => false,
        },
        UnicodeClass::SoftHyphen
        | UnicodeClass::ExoticSpace
        | UnicodeClass::Noncharacter
        | UnicodeClass::PrivateUse
        | UnicodeClass::Homoglyph => false, // Homoglyph never comes out of class_of
    }
}

fn selector_has_its_base(vs: char, prev: Option<char>) -> bool {
    let Some(base) = prev else { return false };
    if class_of(base).is_some() { return false; }                          // D34
    match vs {
        '\u{FE0E}' | '\u{FE0F}' => tables::is_emoji(base),                 // ASCII keycap bases included
        '\u{FE00}'..='\u{FE0D}' => tables::is_standardized_variant(base, vs), // P4
        '\u{E0100}'..='\u{E01EF}' => tables::is_han(base) || tables::is_standardized_variant(base, vs),
        _ => tables::is_letter(base) && tables::script_of(base) == Script::Mongolian, // U+180B–180D, U+180F
    }
}

fn joins_letters(s: &State, next: Option<char>) -> bool {
    let (Some(a), Some(script), Some(n)) = (s.prev_kept, s.anchor_script, next) else { return false };
    JOINING_SCRIPTS.contains(&script)
        && (tables::is_letter(a) || tables::is_mark(a)) && class_of(a).is_none()
        && (tables::is_letter(n) || tables::is_mark(n)) && class_of(n).is_none()
        && tables::script_of(n) == script                                   // P6: next's own script
}
```

`emoji_base`, `letter_of` and `State` (holding `prev`, `prev_kept`,
`anchor_script`, the paragraph flags with the current index, and the tag
ranges with a cursor) are as defined in §4.2.1–4.2.3.

#### 4.2.5 Edge cases, answered

| case | answer | why |
|---|---|---|
| Text starts with a joiner, selector or filler | `prev`/`prev_kept` are `None` → not kept; except a Hangul filler whose `next` is a Hangul letter (D38) | every rule needs its base |
| Text ends with a joiner | `next` is `None` → not kept | the ZWJ rule needs both sides |
| Consecutive glue, e.g. `[2764 FE0F 200D 1F525]` | the ZWJ sees U+2764 as `prev_kept` | glue never moves `prev_kept` — this is *why* the glue concept exists |
| Two selectors in a row, `[2764 FE0F FE0F]` | first `K`, second `F` | D36 |
| Two joiners in a row, `[1F469 200D 200D 1F469]` | first `F` (`next` is a finding), second `K` (`prev_kept` is U+1F469; the first ZWJ is glue) | D34 + glue; the output keeps one ZWJ, which a second pass keeps again |
| A ZWJ between an emoji and a modifier, `[1F44D 200D 1F3FB]` | `K` | both sides are emoji bases; A's rule as written |
| VS16 after a keycap base | `K`, with or without U+20E3 after it | `emoji-variation-sequences.txt` defines `<0031 FE0F>` |
| A ZWJ after a keycap sequence | `F` | `prev_kept` is U+20E3 COMBINING ENCLOSING KEYCAP (`Emoji=No`, not glue). No RGI sequence joins a keycap |
| A flag base followed by VS16, `[1F3F4 FE0F E0067 E0062 E0073 E0063 E0074 E007F]` | `@4 FE0F K` (U+1F3F4 is `Emoji=Yes`), every tag `F` | D39: the tag base is U+1F3F4 immediately, nothing between |
| A ZWJ after a tag sequence | sees the tag base as `prev_kept` | tags are glue |
| A finding between base and joiner, `[0645 200B 200C 0631]` | `200B F`, `200C K` | a not-kept finding does not move `prev_kept`; after `clean` the base and the joiner are adjacent and a second pass agrees |
| An exotic space between base and joiner, `[0645 00A0 200C 0631]` | `00A0 F`, `200C F` | D35: an exotic space moves `prev_kept` (it survives every `Options`, as itself or as U+0020) |
| A soft hyphen between base and joiner, `[0645 00AD 200C 0631]` | `00AD F`, `200C K` | D35: a soft hyphen does not move `prev_kept` |
| An unpaired U+202C after a removed override in an RTL paragraph | U+202C `K` | A keeps embeddings and their pop without checking pairing (Q-A2); an unpaired PDF is ignored by UAX #9 |
| U+FEFF at byte 0, then U+FEFF again | first: no hit; second: `F` | A §4.1 |
| A U+FEFF that becomes byte 0 only after an earlier removal, `[200B FEFF 0061]` | `200B F`, `FEFF F` | "byte 0" means byte 0 of the source given to `hits` |

#### 4.2.6 What is deliberately not protected

Write these into `docs/architecture/layer-a.md` (§4.5) so nobody "fixes"
them by accident:

- **Floating bidi marks in a left-to-right paragraph** (A §4.2): no RTL
  letter, no work for an LRM — removed, `Confirmed`.
- **Pairing of embeddings** in an RTL paragraph is not checked (Q-A2).
- **Legacy Malayalam chillu** `<consonant, U+0D4D VIRAMA, U+200D>` at the
  end of a word (pre-Unicode-5.1 encoding): `next` is a space or
  punctuation, so the ZWJ is removed. Same for any word-final
  `<virama, ZWJ>` half-form display. A known false positive.
- **Latin ligature control** (German *Auflage* with U+200C): removed by
  design — Latin is not a joining script.
- **U+034F COMBINING GRAPHEME JOINER** is removed wherever it is (it
  has orthographic uses in Hebrew and in some German typography). A
  known false positive; no context rule in A.
- **The subdivision code inside a flag is not checked against CLDR.**
  U+1F3F4 + 1–30 digit/lowercase tags + U+E007F is kept whatever the
  letters spell (`zzzz` as well as `gbsct`): Annex C.1 also wants a CLDR
  subdivision id, and the crate carries no CLDR data. A flag-shaped
  sequence can therefore still carry up to 30 lowercase/digit tags;
  anything else in tags is a finding (D39).
- **Egyptian, Duployan and musical context**: none needed — their format
  controls are never findings (§4.1, D19).
- **Unassigned code points** that are not DICP (Q-A3).

#### 4.2.7 Guarantees the next documents rely on

- **Source order, one hit per finding-capable code point.** `at` strictly
  increases; there is exactly one `Hit` for every code point with
  `class_of(c).is_some()` except a U+FEFF at byte 0; `hit.c` is the code
  point at `text[hit.at..]`; `hit.class == class_of(hit.c).unwrap()`;
  never `Homoglyph`; `replacement` is `None`.
- **Confidence.** `kept_by_context` ⇒ `LikelyFalsePositive`; otherwise
  exactly `class.max_confidence()`. Context only demotes (A §4.3). With
  D4 this means a kept-by-context hit never makes a text `suspicious`.
- **`Options`-independence.** `kept_by_context` does not depend on any
  knob, and E1-3 must keep a kept-by-context hit whatever `Options` say
  (A §5.2: context beats the knob).
- **Kept stays kept (idempotence of the context pass).** Let `out` be
  `text` with every not-kept hit removed, except that an `ExoticSpace`
  may stay or become U+0020 and a `SoftHyphen` may stay. Then the kept
  hits of `hits(out)` are the kept hits of `hits(text)`, in order, and
  `hits(out)` has no other hit but those exotic spaces and soft hyphens.
  This holds because every base a rule reads is a non-finding (D34) that
  survives and stays adjacent, `prev_kept` moves only on characters that
  survive under every `Options` (D35), RTL evidence survives (D20), and a
  tag sequence's parts survive together. It is what E1-3's
  `clean_is_idempotent_*` will rest on; E1-2 tests it directly
  (`what_is_kept_stays_kept_on_the_output`).
- **No overlap with E1-4.** A homoglyph source is a Latin, Cyrillic or
  Greek letter (D22 dropped A's halfwidth/fullwidth block clause), and no
  such letter is finding-capable in 18.0.0 — the only finding-capable
  letters are the four Hangul fillers. So `context::hits` and
  `homoglyph::hits` never report the same offset. (Before D22, U+FFA0
  HALFWIDTH HANGUL FILLER — `DefaultIgnorable` here and a `confusables.txt`
  source → U+1160 — would have been reported twice.)
- **The whole text, not a slice.** "Byte 0" and "paragraph" are relative
  to the string passed in. E4 must not call Layer A on chunks that start
  with a U+FEFF or split a paragraph without accounting for both.

### 4.3 `stats.rs` — `TextStats::of`, `LetterShares`, `letter_shares`

**Signatures** (README §3.3, exactly):

```rust
impl TextStats { pub fn of(text: &str) -> TextStats; }   // A §5.5
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LetterShares { pub letters: usize, pub latin: f32, pub cyrillic: f32, pub cjk: f32, pub other: f32 } // percent, 0..=100
pub(crate) fn letter_shares(text: &str) -> LetterShares;
```

`LetterShares::other` is read by nothing until E1-5: give `LetterShares`
the same `cfg_attr(not(test), allow(dead_code, reason = "first caller:
E1-5 ScriptGuard"))`.

**Definitions** (A §5.5, made exact):

| quantity | definition |
|---|---|
| **whitespace** | `White_Space`: `is_space_separator(c)` or `c` ∈ {U+0009–000D, U+0085, U+2028, U+2029}. This equals `PropList.txt` `White_Space` in 18.0.0 (verified). Do **not** use `char::is_whitespace` — it is std's Unicode, not the version the report names. U+200B is not whitespace. |
| **token** | a maximal run of non-whitespace code points. |
| `chars` | `text.chars().count()` — code points, a leading BOM included. |
| `words` | tokens that contain at least one code point with `is_letter` or `is_decimal_digit`. A run of CJK without spaces is one word. |
| **letter** | `is_letter(c)` (gc L*). Marks are never letters — an `Inherited` combining mark adds nothing to any share. The four Hangul fillers are `gc=Lo` and count as Hangul letters (statistics describe the text as given). |
| **Latin / Cyrillic letter** | a letter with `script_of(c) == Script::Latin` / `Script::Cyrillic`. Fullwidth Latin (U+FF21…) is Latin. |
| **CJK letter** | a letter whose script is `Han`, `Hiragana`, `Katakana`, `Hangul` or `Bopomofo`. U+30FC KATAKANA-HIRAGANA PROLONGED SOUND MARK is `Script=Common` and is therefore **other**. |
| **other letter** | every letter that is neither Latin, Cyrillic nor CJK: Greek, Arabic, Hebrew, every joining script, `Common` letters (U+30FC, the mathematical alphanumerics U+1D400…), everything folded into `Script::Other`. |
| `latin_ratio`, `cyrillic_ratio`, `cjk_ratio` | count ÷ letters, a **fraction** 0.0–1.0 (A §7.1 shows `0.98`); **0.0 when there are no letters**, never NaN. |
| `LetterShares.{latin, cyrillic, cjk, other}` | `100.0 × count ÷ letters`, **percent** 0.0–100.0; all four 0.0 when `letters == 0`. When `letters > 0` they sum to 100 within float rounding. |
| `LetterShares.letters` | the number of letters. |
| `code_blocks` | the number of **fence lines** divided by two, rounded down. A fence line is a line (split on U+000A; a trailing U+000D is just content) whose text after stripping leading U+0020 and U+0009 starts with ` ``` ` or `~~~`. Openers and closers are not matched by kind — this is a budget statistic, not a Markdown parser. |
| `urls` | tokens that contain the three bytes `://`. One token counts once. |

Compute ratios and shares from the same integer counts (one private
counting function used by both); do not derive one float from the other.
`TextStats::of("")` equals `TextStats::default()`.

### 4.4 `lib.rs`, `report.rs`, `fixtures/README.md`

- `lib.rs`: add `mod context;` and `mod stats;` (private modules; their
  items are `pub(crate)`, and `TextStats::of` is reachable through the
  already-public `TextStats`). Do not add re-exports: `class_of` is
  reachable as `wipemark_core::class::class_of`, and README §3.3 gives
  `lib.rs`'s re-exports to E1-3.
- `report.rs`: doc comments only — a sentence on the struct ("Computed by
  [`TextStats::of`]; definitions there.") and one line per field from
  §4.3. Do not change the derive list or the fields.
- `fixtures/README.md`: under the layout block, add one paragraph:
  *"`text/keep-*.txt` — the survival set (E1-2): one file per context rule
  in `docs/architecture/layer-a.md` › Classes and context. Every
  finding-capable code point in these files is kept by context;
  `wipemark_core::context`'s `every_keep_fixture_survives_whole` asserts
  each file's byte length and hit count. `keep-script-format-controls.txt`
  and `keep-leading-bom.txt` carry no finding at all."*

### 4.5 `docs/architecture/layer-a.md` — the section *Classes and context*

E1-1 creates the file. If it does not exist, create it with a one-line
title and this section, and say so in the report. The section is written
for someone changing this code next year; it must contain:

1. **One class per code point** — what `class_of` is and where it lives;
   the membership table of §4.1 with counts and the total; first-claim
   order = `UnicodeClass::ALL`; the 41 script format controls that are
   never findings and why (12 of them excluded by name — D19);
   U+FFF9–FFFB; why `Homoglyph` never comes out of `class_of`.
2. **What context keeps** — the rule table of §4.2.4 (in prose or a
   table, with code points, never raw characters); the state model
   (`prev`, `prev_kept`, effective script, `next`, glue, the two
   pre-passes); decisions D19, D20, D34–D39 and this document's precisions P4,
   P6, P8, each with one line of reason.
3. **Why a second pass agrees** — the invariant of §4.2.7 in three
   sentences.
4. **What is deliberately not protected** — §4.2.6.
5. **Where the protections are tested** — a table: protection → test →
   mutation that paints it red (from §5).
6. **`TextStats` and letter shares** — the definitions of §4.3.
7. One closing line: the Unicode facts in this section were read from the
   committed `ucd/` files (18.0.0); after a version bump, run
   `every_class_has_exactly_the_members_unicode_18_gives_it` and read its
   diff before touching anything else.

Do not edit `CLAUDE.md`; E1-7 updates it and the skeleton tables.

## §5 Tests

### 5.1 Conventions

- **Where (D24).** Every test that calls a `pub(crate)` item — `hits`,
  `claims`, `JOINING_SCRIPTS`, `letter_shares` — is a unit test in its
  module (`#[cfg(test)] mod tests`), because an integration test in
  `crates/wipemark-core/tests/` cannot see `pub(crate)`. That includes the
  fixture tests: they live in `context.rs` and read the files with
  `include_str!("../../../fixtures/text/<name>")` /
  `include_bytes!(…)`. Only a test that uses the public API alone (here,
  none is required) may go to `tests/`. This is D24, and §0.4 says the
  same.
- **Helpers** (test-only, in each test module that needs them):
  `fn text(points: &[u32]) -> String` (`char::from_u32(p).expect("a
  scalar value")`, collected), and `fn brief(text: &str) -> Vec<(usize,
  u32, bool)>` returning `(at, u32::from(c), kept_by_context)` for every
  hit. Expected values in the tables below are written in that shape:
  `@4 200C K` is `(4, 0x200C, true)`, `@1 200C F` is `(1, 0x200C, false)`.
- **Every named test asserts the whole `brief` vector** for each input,
  not a subset, so an extra or missing hit fails it.
- **Mutations.** For each row, apply the mutation locally, run the test,
  see it fail for the stated reason, restore the code, and record the row
  in the report's mutation table (protection · mutation · test that went
  red). The mutations named "A §8" are the spec's own; the rest are this
  document's.
- **Runtime.** The walk tests visit 1,112,064 scalar values each;
  in a debug build expect a few seconds. Do not `#[ignore]` them.

### 5.2 `class.rs`

| test | input and expected | mutation that paints it red |
|---|---|---|
| `every_code_point_has_at_most_one_class` | for every scalar value U+0000–U+10FFFF (skip U+D800–DFFF): the number of classes `k` in `UnicodeClass::ALL` with `claims(k, c)` is 0 or 1; `class_of(c)` is that class or `None`; `class_of(c) != Some(Homoglyph)` | remove the subtraction from `claims(DefaultIgnorable)` (A §8) → 406 code points claimed twice (U+00AD, U+200B, U+FE0F, U+E0041, …) |
| `every_class_has_exactly_the_members_unicode_18_gives_it` | walk as above, group `class_of` results into inclusive ranges per class, compare with a **literal** table spelled in the test (the "members" column of §4.1, written out independently — do not generate it from `claims`); total 141,715 | (1) drop the U+FFF9–FFFB clause → `DefaultIgnorable` misses three; (2) drop the D19 exclusion → `DefaultIgnorable` gains twelve; also reds on any E1-1 table regression |
| `no_carrier_alphabet_is_a_joining_script` | `JOINING_SCRIPTS` has 44 entries, no duplicates, and contains none of `Latin`, `Cyrillic`, `Greek`, `Hebrew`, `Han`, `Hiragana`, `Katakana`, `Hangul`, `Bopomofo`, `Common`, `Inherited`, `Other`; contains `Arabic` and `Devanagari` | add `Script::Latin` to the list |
| (existing) `all_lists_every_variant_once_in_order`, `class_ids_are_unique`, `risky_classes_default_to_keep`, `confidence_orders_false_positives_lowest` | unchanged | — |

### 5.3 `context.rs`

| test | input → expected `brief` | mutation that paints it red |
|---|---|---|
| `script_format_controls_are_never_findings` | (a) for every `c` with `is_format(c)`: `class_of(c).is_none()` ⇔ `c` ∈ {0600–0605, 06DD, 070F, 0890–0891, 08E2, 110BD, 110CD, 13430–1343F, 1BCA0–1BCA3, 1D173–1D17A} (41 of 170); (b) `[0600 0661 0662 0663]` (ARABIC NUMBER SIGN over ARABIC-INDIC ONE TWO THREE) → `[]`; (c) `[13000 13430 13001]` (two hieroglyphs joined by U+13430 VERTICAL JOINER) → `[]`; (d) `[1BC00 1BCA0 1BC01]` (DUPLOYAN LETTER H, SHORTHAND FORMAT LETTER OVERLAP, DUPLOYAN LETTER X) → `[]`; (e) `[1D15F 1D173 1D160 1D174]` (QUARTER NOTE, BEGIN BEAM, EIGHTH NOTE, END BEAM) → `[]` | (1) make `DefaultIgnorable` claim every `is_format(c)` (A §8) → (a), (b), (c) red; (2) remove the explicit D19 exclusion → (a), (d), (e) red |
| `a_leading_bom_is_not_a_finding_and_an_inner_one_is` | `[FEFF 0061 FEFF 0062]` → `@4 FEFF F`; `[FEFF]` → `[]`; `[0061 FEFF]` → `@1 FEFF F`; `[FEFF FEFF]` → `@3 FEFF F`; and `class_of('\u{FEFF}') == Some(ZeroWidth)`; the `F` hits are `Confirmed` | drop the `at == 0` exception (A §8) → an extra `@0 FEFF F` |
| `an_emoji_keeps_its_presentation_selector` | `[2696 FE0F 0020 2764 FE0F 0020 2139 FE0F 0020 0031 FE0F 20E3 0020 2122 FE0F]` (SCALES, HEAVY BLACK HEART, INFORMATION SOURCE, keycap one, TRADE MARK SIGN, each with VS16) → `@3 FE0F K`, `@10 FE0F K`, `@17 FE0F K`, `@22 FE0F K`, `@32 FE0F K`; `[0061 FE0F 0020 0031 FE0E 0020 8FBB FE0F 0020 2764 FE0F FE0F 0020 FE0F]` → `@1 FE0F F`, `@6 FE0E K`, `@13 FE0F F`, `@20 FE0F K`, `@23 FE0F F`, `@27 FE0F F` | (1) drop the `is_emoji(base)` check (A §8) → `@1`, `@13` become `K`; (2) use `prev_kept` instead of `prev` for selectors (D36) → `@23` becomes `K` |
| `a_family_stays_a_family` | `[1F469 200D 1F469 200D 1F467 200D 1F466 0020 1F3F3 FE0F 200D 1F308 0020 1F46E 200D 2640 FE0F 0020 2764 FE0F 200D 1F525]` (family of four, rainbow flag, woman police officer, heart on fire) → `@4 200D K`, `@11 200D K`, `@18 200D K`, `@30 FE0F K`, `@33 200D K`, `@45 200D K`, `@51 FE0F K`, `@58 FE0F K`, `@61 200D K`; `[1F469 200D 0020 200D 1F469 0020 1F469 200D 200D 1F469 0020 200D]` → `@4 200D F`, `@8 200D F`, `@20 200D F`, `@23 200D K`, `@31 200D F` | (1) drop the emoji-ZWJ rule (A §8) → every ZWJ of the first input `F`; (2) let glue move `prev_kept` (A §8) → `@33` (rainbow flag) and `@61` (heart on fire) become `F` |
| `persian_keeps_its_non_joiner` | `[0645 06CC 200C 0631 0648 0645]` → `@4 200C K`; `[0061 200C 0062]` → `@1 200C F`; `[0645 200C 0020]` → `@2 200C F`; `[0645 200C 0915]` → `@2 200C F` | remove `Script::Arabic` from `JOINING_SCRIPTS` (A §8) → `@4` becomes `F` |
| `devanagari_keeps_its_joiner` | `[0915 094D 200D 0937]` → `@6 200D K`; `[0915 094D 200D 0061]` → `@6 200D F` | remove `Script::Devanagari` (A §8) → `@6` of the first becomes `F` |
| `a_mark_carries_the_script_of_its_base` | `[0628 064E 200C 0647]` → `@4 200C K`; `[0628 200C 064E]` → `@2 200C F`; `[0DC1 0DCA 200D 0DBB 0DD3]` → `@6 200D K` | (1) take `script_of(prev_kept)` without the `Inherited` fallback (P6) → `@4` becomes `F`; (2) let an `Inherited` `next` match → `@2` becomes `K` |
| `a_bidi_mark_is_typography_beside_rtl_and_a_carrier_without_it` | first half `[05E9 05DC 05D5 05DD 0020 200E 0052 0075 0073 0074 200E]` → `@9 200E K`, `@16 200E K`; second half `[05E9 05DC 05D5 05DD 0020 200E 0052 0075 0073 0074 000A 0048 0069 200E 0020 0074 0068 0065 0072 0065 200F]` (a Hebrew paragraph, then *Hi there* in its own paragraph) → `@9 200E K`, `@19 200E F`, `@28 200F F`; the `F` hits are `Confirmed` | (1) remove the bidi rule (A §8, first half) → `@9`, `@16` become `F`; (2) decide RTL for the whole text instead of per paragraph (A §8, second half) → `@19`, `@28` become `K` |
| `a_stray_rlm_does_not_protect_itself` | `[0048 0069 200F 0020 0074 0068 0065 0072 0065]` (*Hi*, RLM, *there*) → `@2 200F F`, `Confirmed`; `[0048 0069 061C 0020 0074 0068 0065 0072 0065]` → `@2 061C F`, `Confirmed` | let a `Bidi_Control` count as RTL evidence — drop the letter-or-mark condition of D20 → both become `K` |
| `an_override_is_always_removed` | `[05E9 05DC 05D5 05DD 0020 202E 0061 0062 0063 202C 0020 202D 0064 202C]` → `@9 202E F`, `@15 202C K`, `@19 202D F`, `@23 202C K`; the `F` hits are `Confirmed` | treat U+202D/U+202E like the other controls (A §8) → `@9`, `@19` become `K` |
| `a_flag_keeps_its_tags_and_a_loose_tag_does_not` | `[1F3F4 E0067 E0062 E0073 E0063 E0074 E007F]` (flag of Scotland) → `@4 E0067 K`, `@8 E0062 K`, `@12 E0073 K`, `@16 E0063 K`, `@20 E0074 K`, `@24 E007F K`; U+1F3F4 followed by 30 × U+E0061 and U+E007F (32 code points) → 31 hits `@4, @8, … @124`, all `K`; `[0061 E0067 E0062 E007F 0020 1F3F4 E0067 E0062 E0073 E0063 E0074 0020 E007F]` → `@1 F`, `@5 F`, `@9 F`, `@18 F`, `@22 F`, `@26 F`, `@30 F`, `@34 F`, `@39 F`; `[1F600 E0001 E0068 E007F]` → `@4 E0001 F`, `@8 E0068 F`, `@12 E007F F`; U+1F3F4 followed by 31 × U+E0061 and U+E007F (33 code points) → 32 hits `@4 … @128`, all `F`; every `F` is `Confirmed` | (1) stop requiring U+E007F (A §8) → `@18`…`@34` become `K`; (2) stop requiring a base → `@1`, `@5`, `@9` become `K`; (3) drop the 32-code-point limit (D39) → the 33-code-point flag becomes `K` |
| `an_emoji_cannot_smuggle_tags` | `[1F600 E0068 E0069 E0074 E007F]` (GRINNING FACE carrying the tag text *hit* and CANCEL TAG) → `@4 E0068 F`, `@8 E0069 F`, `@12 E0074 F`, `@16 E007F F`, all `TagCharacter`, `Confirmed` — under D4 this text is `suspicious` (E1-3 asserts that on `inspect`); `[1F3F3 E0067 E0062 E0073 E0063 E0074 E007F]` (WAVING WHITE FLAG base) → `@4 … @24` all `F`; `[1F3F4 E0047 E0042 E0053 E0043 E0054 E007F]` (capital tags after the right flag) → `@4 … @24` all `F`; `[1F3F4 E0048 E0069 E0021 E007F]` (*Hi!*) → `@4 F`, `@8 F`, `@12 F`, `@16 F`; `[1F3F4 FE0F E0067 E0062 E0073 E0063 E0074 E007F]` → `@4 FE0F K`, `@7 … @27` all `F` | (1) accept any `Emoji=Yes` base, as A §4.2 wrote it (D39) → the first two inputs become `K`; (2) accept any tag in U+E0020–E007E → the capital and *Hi!* inputs become `K` |
| `an_ideograph_keeps_its_variation_sequence` | `[8FBB E0100 0020 0061 E0100 0020 8FBB E0100 E0101 0020 8FBB FE00]` → `@3 E0100 K`, `@9 E0100 F`, `@17 E0100 K`, `@21 E0101 F`, `@29 FE00 F` | (1) drop the Han clause (A §8) → `@3`, `@17` become `F`; (2) extend the Han clause to VS1–VS14 (P4) → `@29` becomes `K`; (3) `prev_kept` instead of `prev` (D36) → `@21` becomes `K` |
| `a_standardized_variant_is_kept_and_a_random_one_is_not` | `[2229 FE00 0020 2282 FE00 0020 0041 FE00 0020 0030 FE00]` → `@3 FE00 K` (INTERSECTION *with serifs*), `@10 FE00 F` (U+2282 SUBSET OF + VS1: A §4.2's own example, verifiably **absent** from `StandardizedVariants.txt` 18.0.0 — the plausible-looking random pair), `@15 FE00 F`, `@20 FE00 K` (DIGIT ZERO *short diagonal stroke form*) | drop the `is_standardized_variant` lookup (A §8) → `@3`, `@20` become `F` |
| `a_mongolian_letter_keeps_its_selector` | `[1820 180B 0020 182C 1820 1837 180E 1820 0020 0061 180B 0020 180E 1820 0020 1820 180B 180C]` → `@3 180B K`, `@16 180E K`, `@24 180B F`, `@28 180E F`, `@38 180B K`, `@41 180C F` | (1) drop the FVS rule → `@3`, `@38` become `F`; (2) drop the U+180E rule → `@16` becomes `F`; (3) `prev_kept` instead of `prev` for FVS (D36) → `@41` becomes `K` |
| `khmer_keeps_its_inherent_vowel` | `[1780 17B4 0020 0061 17B4 0020 17B4]` → `@3 17B4 K`, `@8 17B4 F`, `@12 17B4 F` | drop the Khmer rule → `@3` becomes `F` |
| `a_partial_syllable_keeps_its_filler` | `[1100 1160 0020 115F 1161 0020 3164 3131 314F 3164 0020 0061 3164 0062 0020 AC00 3164 3164]` → `@3 1160 K`, `@7 115F K`, `@14 3164 K`, `@23 3164 K`, `@28 3164 F`, `@36 3164 K`, `@39 3164 F` | (1) drop the Hangul rule → every `K` becomes `F`; (2) drop the `next` clause (D38) → `@7`, `@14` become `F`; (3) let a filler count as a Hangul letter (drop D34 in `letter_of`) → `@39` becomes `K` |
| `a_base_is_never_itself_a_finding` | `[1780 200C 17B4]` → `@3 200C F`, `@6 17B4 K`; `[1820 200D 180B]` → `@3 200D F`, `@6 180B F` | drop the not-finding condition on `next` in `joins_letters` (D34) → `@3` of both become `K` |
| `an_ascii_character_is_never_a_joiner_side_or_a_tag_base` | `[0031 200D 0032]` → `@1 200D F`; `[0031 E0067 E007F]` → `@1 E0067 F`, `@5 E007F F`; `[0031 FE0F 20E3 200D 1F525]` → `@1 FE0F K`, `@7 200D F`; `[0023 FE0E]` → `@1 FE0E K` | treat ASCII `Emoji=Yes` as an emoji base (drop D37) → `@1 200D` becomes `K`. The tag case stays `F` under that mutation because D39 accepts only U+1F3F4 as a base; it documents D37's tag half, `an_emoji_cannot_smuggle_tags` guards it |
| `hits_come_in_source_order_one_per_finding_capable_character` | `[0061 200B 0062 1F469 200D 1F469 0063 200E 0064 E0041 0065 FE0F 0066 00AD 0067 00A0 0068 FDD0 0069 E000 006A 2061 006B FFF9 006C FEFF 006D 0085 2028 0001]` → `@1 200B F`, `@9 200D K`, `@17 200E F`, `@21 E0041 F`, `@26 FE0F F`, `@30 00AD F`, `@33 00A0 F`, `@36 FDD0 F`, `@40 E000 F`, `@44 2061 F`, `@48 FFF9 F`, `@52 FEFF F` (12 hits: every class but `Homoglyph` is present; U+0085, U+2028, U+0001 are not findings). For each hit also: `text[at..]` starts with `c`; `class == class_of(c).unwrap()`; `replacement == None`; `at` strictly increasing | (1) do not emit kept hits → 11 hits; (2) skip `ExoticSpace` "because its default is Keep" → 11 hits |
| `a_kept_hit_is_a_likely_false_positive_and_any_other_carries_its_ceiling` | over every input of this table and every keep fixture: `kept_by_context` ⇒ `confidence == LikelyFalsePositive`; otherwise `confidence == class.max_confidence()` | give kept hits `class.max_confidence()` |
| `hit_offsets_are_bytes_not_chars` | `[041F 0440 0438 0432 0435 0442 0020 200B]` (*Privet* and a ZWSP) → `@13 200B F` | report the char index (`@7`) |
| `what_is_kept_stays_kept_on_the_output` | corpus: every input of this table, every keep fixture, and `[0645 00A0 200C 0631]`, `[0645 200B 200C 0631]`, `[0645 00AD 200C 0631]`. For each text and each `(keep_soft_hyphen, spaces_to_ascii)` in {false, true}²: `out` = the text with every not-kept hit removed, except an `ExoticSpace` (kept, or U+0020 when `spaces_to_ascii`) and a `SoftHyphen` (kept when `keep_soft_hyphen`); assert the kept code points of `hits(out)`, in order, equal those of `hits(text)`, and every other hit of `hits(out)` is an `ExoticSpace` or `SoftHyphen`. (Verified: 59 texts × 4 combinations, all pass.) | (1) let an `ExoticSpace` not move `prev_kept` (D35) → `[0645 00A0 200C 0631]` under `spaces_to_ascii`: kept `[200C]` vs `[]`; (2) drop the not-finding condition on `next` (D34) → `[1780 200C 17B4]`: kept `[200C 17B4]`, then `[17B4]` |
| `every_keep_fixture_survives_whole` | each of the 12 files of §5.5: `include_bytes!` length equals the table's byte count, `hits` count equals the table's hit count, every hit `K` | remove any one keep rule → its fixture goes red (e.g. drop the bidi rule → `keep-bidi-rtl.txt` has 14 `F`) |

The helper `apply` used by `what_is_kept_stays_kept_on_the_output` is
test code mirroring E1-3's decision for the non-homoglyph classes; it is
not the E1-3 API and must not be exported.

### 5.4 `stats.rs`

| test | input → expected | mutation that paints it red |
|---|---|---|
| `chars_are_code_points_not_bytes` | `[041F 0440 0438 0432 0435 0442 002C 0020 043C 0438 0440]` (*Privet, mir*) → `chars 11`, `words 2` | use `text.len()` → 20 |
| `a_word_needs_a_letter_or_a_digit` | `[0061 0020 2014 0020 0031 0020 002E 002E 002E 0020 0062 0032 0020 0063 200B 0064 0020 0065 00A0 0066]` → `words 6` (*a*, *1*, *b2*, *c*ZWSP*d*, *e*, *f*; the em dash and the dots are not words; U+200B does not split, U+00A0 does); `[6211 662F 5B66 751F 3002]` (a Chinese sentence) → `words 1` | (1) count every token → 8; (2) split on U+0020 only → 5 |
| `letter_shares_are_taken_among_letters` | `[0061 0062 0063 0020 0433 0434 0435 0020 4E2D 6587 0020 0031 0032 0033]` (*abc*, Cyrillic *gde*, two Han, *123*) → `letters 8`, shares `latin 37.5`, `cyrillic 37.5`, `cjk 25.0`, `other 0.0`; `TextStats` ratios `0.375`, `0.375`, `0.25` | divide by `chars` (14) instead of letters |
| `a_mark_is_not_a_letter` | `[0065 0301 0020 0627 064E]` (*e* + COMBINING ACUTE, ALEF + FATHA) → `letters 2`, `latin 50.0`, `other 50.0` | count `gc=M*` as letters → `letters 4` |
| `cjk_is_han_kana_hangul_and_bopomofo` | `[6F22 304B 30AB D55C 3105 30FC]` (Han, Hiragana, Katakana, Hangul syllable, Bopomofo, U+30FC) → `letters 6`, `cjk` within 0.001 of 83.333, `other` within 0.001 of 16.667 | drop `Bopomofo` from the CJK set → 66.667 |
| `a_text_without_letters_has_zero_shares_not_nan` | `[0031 0032 0033 0020 0021 0021 0021 0020 0663]` → `letters 0`, all four shares `0.0`, all three ratios `0.0`, `words 2`; and `TextStats::of("") == TextStats::default()` | divide without the zero guard → NaN ≠ 0.0 |
| `code_blocks_count_pairs_of_fence_lines` | the text `` "```\na\n```\n  ~~~\nb\n~~~\n" `` → `2`; `` "```\na\n```\n```\n" `` → `1` | (1) do not strip leading spaces → first gives 1; (2) count fence lines, not pairs → second gives 3 |
| `a_url_is_a_token_with_a_scheme_separator` | `"see https://a.b/c and ftp://x, not www.y.z or mailto:me"` → `2`; `"http://a://b"` → `1` | count occurrences of `://` instead of tokens → second gives 2 |
| `shares_are_percent_and_ratios_are_fractions` | `[0061 0062 0063 0020 0433 0434 0435]` → `letter_shares().latin == 50.0`, `TextStats::of().latin_ratio == 0.5` | return fractions from `letter_shares` (or percent from `TextStats`) |

### 5.5 Fixtures this document adds — `fixtures/text/keep-*.txt`

Byte-exact under E1-1's `.gitattributes` (`fixtures/text/** -text`, D25):
UTF-8, exactly the code points listed, in order, nothing added — each
file ends with the U+000A shown and has no other line ending. Every
finding-capable code point in them is kept by context.

| file | content (code points) | bytes | hits (all `K`) | SHA-256 |
|---|---|---|---|---|
| `keep-emoji-presentation.txt` | `2696 FE0F 0020 2764 FE0F 0020 2139 FE0F 0020 0031 FE0F 20E3 0020 2122 FE0F 0020 0023 FE0F 20E3 0020 00A9 FE0F 000A` | 50 | 7 | `01f97eb9fe2cc89b6aeec3465cdbc67f7114556e82906f3a967b976785046daf` |
| `keep-emoji-zwj.txt` | `1F469 200D 1F469 200D 1F467 200D 1F466 0020 1F3F3 FE0F 200D 1F308 0020 1F46E 200D 2640 FE0F 0020 2764 FE0F 200D 1F525 0020 1F9D1 1F3FB 200D 1F4BB 0020 1F3F4 200D 2620 FE0F 000A` (all six are RGI ZWJ sequences in 18.0) | 99 | 12 | `98377499646d0a3ebe56f8ad578529d2c606c4f7d26d3d7f5726f3c115eeae0d` |
| `keep-joining-scripts.txt` | `0645 06CC 200C 0631 0648 0645 0020 0646 0627 0645 0647 200C 0647 0627 000A 0915 094D 200D 0937 000A 0DC1 0DCA 200D 0DBB 0DD3 000A` (Persian *mi-ravam*, *name-ha*; Devanagari KSSA; Sinhala *shri*) | 59 | 4 | `82fd33cc723b636cd9a25e2a39717e9f01363436663944b250789b604dad9ae9` |
| `keep-bidi-rtl.txt` | `0645 0631 062D 0628 0627 0020 2066 0057 0069 0070 0065 006D 0061 0072 006B 2069 0020 061C 0661 0662 000A 05E9 05DC 05D5 05DD 0020 200E 0052 0075 0073 0074 200E 0020 202A 0061 0062 0063 202C 0020 202B 0064 0065 0066 202C 0020 2067 0078 2069 0020 2068 0079 2069 0020 200F 000A` (an Arabic and a Hebrew paragraph carrying all ten keepable controls) | 93 | 14 | `5cafab5c86e261bd0064bc5df82ec9e1bb429306eca1eb41a9e2110e0ebf5af5` |
| `keep-flag-tags.txt` | `1F3F4 E0067 E0062 E0073 E0063 E0074 E007F 0020 1F3F4 E0067 E0062 E0065 E006E E0067 E007F 0020 1F3F4 E0067 E0062 E0077 E006C E0073 E007F 000A` (Scotland, England, Wales) | 87 | 18 | `d43d25505e23b4de88a08ad20f9ab1165be909f7c465006b21bb03851c2f1bdb` |
| `keep-ideographic-variation.txt` | `8FBB E0100 0020 8FBB E0101 0020 845B E0100 0020 6F22 FE00 0020 6F22 FE01 000A` (IVD Adobe-Japan1 sequences; two CJK compatibility standardized variants) | 38 | 5 | `08e9f29e5f1033952fb26df35541ac5fc451ede157946fee81b12844274cab5a` |
| `keep-standardized-variants.txt` | `2229 FE00 0020 228A FE00 0020 0030 FE00 0020 1000 FE00 0020 13091 FE00 0020 A856 FE00 000A` (math, digit zero, Myanmar, Egyptian, Phags-pa) | 41 | 6 | `f30130252c04dafd6ca1f08ab5b416931b46a4729a66b3374c5223ee73dd7cc1` |
| `keep-mongolian.txt` | `182C 1820 1837 180E 1820 0020 1828 180B 1820 0020 182C 180F 000A` | 33 | 3 | `d3971bbbbe0c0f1dd99e84083bd73e61b92c0f0e8a625e04ddb52b46da84d33f` |
| `keep-khmer.txt` | `1780 17B4 0020 1781 17B5 000A` | 14 | 2 | `f25dc8e53ffdf9e07e3abdaed4d1e6724dccf13ff48bd0d52bd44bb308516459` |
| `keep-hangul-fillers.txt` | `1100 1160 0020 115F 1161 0020 3164 3131 314F 3164 0020 FFA0 FFA1 FFC2 FFA0 000A` | 40 | 6 | `7d38d4637ceb60ff06463a004eae161fc84420c4a881ecebbdb297d3fdaf1548` |
| `keep-script-format-controls.txt` | `0600 0661 0662 0663 0020 06DD 0661 0662 0020 0890 0661 000A 070F 0710 0712 000A 110BD 0967 0968 000A 13000 13430 13001 0020 13000 13431 13002 000A 1BC00 1BCA0 1BC01 000A 1D15F 1D173 1D160 1D174 000A` (every kind of never-finding format control in use: Arabic and Syriac prepended marks, the Kaithi number sign, Egyptian quadrat joiners, Duployan overlap, musical beams) | 96 | 0 | `4cefc8fd7495bc740a8b1efe8e5b716c0c1479fe3d7c71c5766b4560c91e215a` |
| `keep-leading-bom.txt` | `FEFF 0050 006C 0061 0069 006E 0020 0074 0065 0078 0074 002E 000A` (BOM + *Plain text.*) | 15 | 0 | `12e80102c7dd1688176c18e5fb4b6908b06b42572a3f1987f57363e7d3dfb4b4` |

Create them with a script, never with an editor (an editor may add a
newline, normalise a line ending or drop a leading BOM):

```sh
mkdir -p fixtures/text
python3 - <<'EOF'
import pathlib
FIXTURES = {
    "keep-emoji-presentation.txt": "2696 FE0F 0020 2764 FE0F 0020 2139 FE0F 0020 0031 FE0F 20E3 0020 2122 FE0F 0020 0023 FE0F 20E3 0020 00A9 FE0F 000A",
    # … one entry per row of the table above, content copied from its second column
}
for name, points in FIXTURES.items():
    data = "".join(chr(int(p, 16)) for p in points.split()).encode("utf-8")
    pathlib.Path("fixtures/text", name).write_bytes(data)
EOF
shasum -a 256 fixtures/text/keep-*.txt     # every line must match the table
git check-attr text -- fixtures/text/keep-khmer.txt   # must print "text: unset" (E1-1's .gitattributes)
```

These files are E1-2's; E1-3 adds the removal set (one file per class)
and its `clean_is_idempotent_on_every_fixture` will also walk these.

**Names.** Every E1-2 fixture starts with `keep-`; E1-3's removal
fixtures are named after their class and E1-3 stops on a name that
already exists with different bytes. So: never create a `fixtures/text/`
file without the `keep-` prefix in this document, and if a `keep-*` name
already exists when you start (another lane got there first), it must be
byte-identical to the table above — compare with `shasum`, and stop and
report if it is not. The standardized-variant pairs used here (U+2229
U+FE00, U+0030 U+FE00) are the same ones E1-3's fixtures use; that is
intended, not a collision.

### 5.6 The mutation record the report must carry

At minimum one row per mutation in §5.2–5.4 (55 mutations across the 36
new tests). Format: *protection · mutation (file:line of the code you
changed) · test that went red · how it failed (the assertion message)*.
A mutation that leaves its test green means the test is wrong: fix the
test, do not drop the row.

## §6 Acceptance criteria

- [ ] `class_of` and `JOINING_SCRIPTS` exist in `class.rs` with the
      README §3.3 signatures; `claims` is private; the variant doc
      comments are the ones in §4.1. Evidence:
      `every_code_point_has_at_most_one_class`,
      `every_class_has_exactly_the_members_unicode_18_gives_it`,
      `no_carrier_alphabet_is_a_joining_script`.
- [ ] `context.rs` provides `Hit` and `hits` exactly as README §3.3, with
      both pre-passes and every rule of §4.2.4 including D20, D34–D39 and P4, P6, P8.
      Evidence: every test of §5.3 green.
- [ ] `stats.rs` provides `TextStats::of`, `LetterShares`,
      `letter_shares` exactly as README §3.3, with the definitions of
      §4.3. Evidence: every test of §5.4 green.
- [ ] The 12 fixtures exist under `fixtures/text/`, their SHA-256 match
      §5.5, `git check-attr text` reports `unset` for them, and
      `every_keep_fixture_survives_whole` asserts each.
- [ ] Every mutation of §5 was applied, seen red, restored, and recorded
      in the report.
- [ ] The six commands of §0.5 are green (`cargo test --workspace
      --locked` count is the previous count plus this document's 36 new
      tests, 0 failed).
- [ ] `git diff --stat` touches only the files in the header table; in
      particular nothing under `crates/wipemark-i18n/`, `apps/`,
      `vendor/`, and no `Cargo.toml`/`Cargo.lock` change.
- [ ] No `TODO`, no crate-wide `allow`; each `dead_code` allowance is a
      `cfg_attr(not(test), …)` whose `reason` names its first caller.
- [ ] `docs/architecture/layer-a.md` has the section *Classes and
      context* with the seven parts of §4.5.
- [ ] `fixtures/README.md` has the paragraph of §4.4.
- [ ] `docs/plan/README.md` §3 row E1-2 says *done* with the report's
      file name; the report exists at
      `docs/plan/reports/E1-2-<YYYY-MM-DD>.md` and lists: what was built,
      every deviation (at least any E1-1 difference found in preflight),
      the mutation table, the commands
      and results, and the hand-off notes of §4.2.7 for E1-3 and E1-4.

## §7 Out of scope

- **E1-1's territory.** UCD files, `build.rs`, `tables.rs`, `Script`,
  `name_of`, `UNICODE_VERSION`, `.gitattributes`, `NOTICE`. If a
  predicate is missing, see §3.2 — do not add UCD files.
- **E1-3.** `Options`, `Action::Replace`, `Options::action_for`, the
  decision of what to do with a not-kept hit, aggregation into
  `UnicodeFinding` (D6), the `findings`/`kept` split and `InspectReport.kept`
  (D5), `suspicious` (D4, D28), the output string, NFKC and its rounds (D26),
  the JSON form, `inspect`/`clean`, `scrub::collect_hits`, the removal
  fixtures, `positions_are_byte_offsets_into_the_source`,
  `soft_hyphens_alone_are_not_suspicious`,
  `clean_is_idempotent_on_every_fixture` / `…_on_a_generated_corpus`,
  removing the `dead_code` allowance on `hits`.
- **E1-4.** Homoglyph hits, `replacement`, the mixed-word rule (D21,
  D22).
- **E1-5.** The guards; `ScriptGuard` is the first non-test reader of
  `letter_shares`.
- **E1-6 / E1-7.** Catalogue keys for classes and confidences, MCP and
  CLI wiring, `CLAUDE.md` and skeleton tables.
- **Owner questions.** Q-A1 per-class overrides (no knob may un-keep a
  kept-by-context hit in E1); Q-A2 pairing of embeddings (not checked);
  Q-A3 unassigned code points (not findings unless DICP); Q-A5
  Arabic/Hebrew ratios (not added); Q-A7 the known false positives of
  §4.2.6 (left unprotected); Q-A8 widening D39 beyond flags (not done).
- **The i18n independent spelling** (`crates/wipemark-i18n/src/tests.rs:316-337`)
  stays exactly as it is.

## §8 Basis and references

**Specs.** A §1 (deviation rows for bidi controls, zero-width, variation
selectors, exotic spaces), A §2 (what is not a finding), A §4.1 (classes
via UCD properties, first-claim order, the never-findings `Cf`), A §4.2
(context, `prev_kept`, glue, `JOINING_SCRIPTS`, what is not protected),
A §4.3 (confidence only demotes; aggregation is E1-3's), A §5.3 steps 1–3
(the pre-passes and the main pass; why it is idempotent), A §5.5
(`TextStats`), A §8 (the gate rows this document implements: VS16, ZWJ in
emoji, joining scripts, bidi in RTL, overrides, flags, IVS and standardized
variants, Mongolian/Khmer/Hangul, script format controls, BOM, one class).
OV §0.1 rule 3 (the honest report), OV §3.1 (the class table A refines),
OV §3.2 (idempotence and the mutation gate). README §3.2–3.3, §4 D3, D4,
D6, D17, D19, D20, D22, D24, D25, D34–D39; §5 Q-A1–Q-A3, Q-A5, Q-A7, Q-A8.

**Repository.** `CLAUDE.md` (rules cited in §2); `fixtures/README.md`;
`docs/architecture/skeleton.md`; `crates/wipemark-i18n/src/tests.rs:289-337`.

**Unicode 18.0.0** (all facts in this document were read from these files
on 2026-10-03):

- UCD: <https://www.unicode.org/Public/18.0.0/ucd/> —
  `PropList.txt` (`Bidi_Control`, `Variation_Selector`,
  `Noncharacter_Code_Point`, `White_Space`), `DerivedCoreProperties.txt`
  (`Default_Ignorable_Code_Point` and its derivation header),
  `UnicodeData.txt` (gc, Bidi_Class), `Scripts.txt`,
  `StandardizedVariants.txt` (1,437 pairs), `emoji/emoji-data.txt`,
  `emoji/emoji-variation-sequences.txt`.
- Emoji sequence data: <https://www.unicode.org/Public/18.0.0/emoji/>
  (`emoji-zwj-sequences.txt`: 1,614 sequences, none with an ASCII element
  or a ZWJ before a modifier; `emoji-sequences.txt`: the three
  `RGI_Emoji_Tag_Sequence` flags).
- Confusables: <https://www.unicode.org/Public/18.0.0/security/confusables.txt>
  (U+FFA0 → U+1160 — the double report D22 removed).
- The Unicode Standard 18.0, core specification:
  ch. 23 *Special Areas and Format Characters* — 23.2 Layout Controls
  (cursive connection, ZWJ/ZWNJ, prepended concatenation marks, CGJ,
  bidirectional ordering controls), 23.4 Variation Selectors, 23.8 the
  BOM, 23.9 Tag Characters:
  <https://www.unicode.org/versions/Unicode18.0.0/core-spec/chapter-23/>;
  ch. 18 *East Asia*, 18.6 Hangul (the fillers):
  <https://www.unicode.org/versions/Unicode18.0.0/core-spec/chapter-18/>;
  ch. 16 *Southeast Asia-I*, Khmer (U+17B4/U+17B5):
  <https://www.unicode.org/versions/Unicode18.0.0/core-spec/chapter-16/>;
  ch. 13 *South and Central Asia-II*, Sinhala and Mongolian:
  <https://www.unicode.org/versions/Unicode18.0.0/core-spec/chapter-13/>;
  ch. 12 *South and Central Asia-I*, Devanagari and Malayalam:
  <https://www.unicode.org/versions/Unicode18.0.0/core-spec/chapter-12/>.
- UAX #9, *Unicode Bidirectional Algorithm* (Bidi_Class R/AL, marks,
  isolates, embeddings, overrides): <https://www.unicode.org/reports/tr9/>.
- UTS #51, *Unicode Emoji*, version 18.0 (revision 31, 2026-08-28) — ED-9a
  presentation sequences, ED-14a tag sequences, ED-14c keycap sequences,
  ED-15a/ED-16 ZWJ elements and sequences, Annex C (C.1 flag emoji tag
  sequences — the rule D39 adopts):
  <https://www.unicode.org/reports/tr51/tr51-31.html>.
- UAX #24, *Unicode Script Property* (`Inherited`):
  <https://www.unicode.org/reports/tr24/>.
- UAX #44, *Unicode Character Database* (file formats, code point labels):
  <https://www.unicode.org/reports/tr44/>.
- UTS #37, *Unicode Ideographic Variation Database*, and the IVD itself
  (selectors E0100–E01EF; data 2025-07-14 registers U+8FBB and U+845B with
  E0100/E0101): <https://www.unicode.org/reports/tr37/>,
  <https://www.unicode.org/ivd/>.

**Security background.** Boucher & Anderson, *Trojan Source: Invisible
Vulnerabilities*, CVE-2021-42574 — why the overrides are never kept:
<https://trojansource.codes/>.
