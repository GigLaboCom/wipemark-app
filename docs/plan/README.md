# Wipemark — plan of record

> **What this folder is.** The map of everything that remains to be built,
> and — for the first part of it, epic **E1** (Layer A, the deterministic
> Unicode scrubber) — seven **self-sufficient implementer documents**,
> `E1-1` … `E1-7`. Each E1 document can be handed to a fresh agent on its
> own: it carries the ground rules (§0, identical in every document and
> reproduced once in §9 below), the goal, what to read, what is true in the
> code today with `file:line`, the exact deliverables, the tests (and how
> each protection is painted red), acceptance criteria and what is out of
> scope. This file is the map around them: where the project stands, the
> order of the remaining epics and why, the contracts between the E1
> documents, every decision this plan takes beyond its specs, the
> questions only the owner can answer, and the status table an implementer
> flips when a document lands. Later epics become series of their own when
> E1 has landed — §7 holds what they will be built from.
>
> Written 2026-10-03 on branch `feat/e0-e6-shell` at `497eafa`
> ([PR #1](https://github.com/GigLaboCom/wipemark-app/pull/1)), from: the
> code at that commit; `CLAUDE.md`; `docs/architecture/*` and `docs/sdd/*`;
> and the Watchword entries `heretic-unmark-overview-decomposition-2026-09-07`
> (the overview, cited below as **OV §n**), `wipemark-core-layer-a-2026-09-21`
> (the E1 spec, cited as **A §n**), `wipemark-intake-drag-and-drop-2026-09-11`
> and its closure, `wipemark-q1-name-decision-2026-09-21`,
> `wipemark-uzu-evaluation-2026-09-11` and `wipemark-status-2026-10-03`.
> Working copies of the Watchword files are in `ssd-docs/` (gitignored).
> Every `file:line` below is at `497eafa`; re-verify against HEAD before
> acting on one.

---

## 1. Where we are (verified 2026-10-03)

> **Since this section was written, E1 has landed** (same day, E1-1 …
> E1-7, `45ebb17` … `7e606b8`; closure: Watchword TEXT
> `wipemark-core-layer-a-closed-2026-10-03`). What changed against the
> snapshot below: Layer A exists in `wipemark-core` (tables, classifier,
> scrubber and NFKC, homoglyphs, guards); MCP `inspect`/`clean` and CLI
> `inspect`/`clean` do real work (exit 0/1/3), and the windows say that
> cleaning *from a window* is not in this version yet and runs from the
> CLI and over MCP. Gates at `7e606b8`: **778 passed, 0 failed,
> 1 ignored**; **466** catalogue keys × 3 languages; workspace clippy
> green on Linux since `f07e291` (it was red there before E1). The
> tables below are kept as the snapshot at `497eafa` that the E1
> documents were written against.

**One sentence.** Everything *around* the two layers is built and tested;
neither layer exists. A user can drop, paste and import things, see what
each one is, compare a result with its original, configure an engine, a
model, retention, placement and an MCP server — and nothing is ever
cleaned or rewritten, which every surface says out loud.

### 1.1 Branch, gates, size

| fact | value | evidence |
|---|---|---|
| working branch | `feat/e0-e6-shell` — **all work happens here**, never on `main` | owner, 2026-10-03; PR #1 targets `main` |
| `origin/main` | `4bfd6ff` Initial commit (2026-09-07) until PR #1 merges | `git ls-remote origin` |
| commits on the branch | `a3c8c6e` E0, `9b54032` E6, `35c5729` Q1, `7d46629` E1 spec filed, `497eafa` Watchword table | `git log` |
| gates at `497eafa` | dep-direction, nightly rustfmt, clippy `-D warnings --locked`, `cargo test --workspace --locked` **576 passed, 0 failed, 1 ignored**, both `--locked` feature checks — all green | run 2026-10-03, recorded in `wipemark-status-2026-10-03` |
| size | ≈44.6 k lines of Rust in `crates/` + `apps/`; 14 documents in `docs/architecture/`; 421 catalogue keys × 3 languages | `wc -l`, `grep -c` |
| toolchain | 1.94.1 pinned in `rust-toolchain.toml` with `clippy`/`rustfmt` components; nightly only for rustfmt | `rust-toolchain.toml`, CLAUDE.md "Gates" |
| GPUI | zed `gpui` at rev `81b16f464ce91e40c1c645b56675c26ee0b2b6c4`, rewritten into the submodule by `scripts/pin-gpui-component.sh` | that script |
| gpui-component | submodule `vendor/gpui-component` at `a2f9c95`, from the organisation's fork `GigLaboCom/gpui-component`, protected branch `heretic/epic-4-line-decorations` (moved from a personal fork on 2026-10-03; §8 R1, `docs/sdd/line-decorations.md`) | `.gitmodules`; `git submodule status` |

### 1.2 Per crate

| crate | real today | absent | file |
|---|---|---|---|
| `wipemark-core` | `UnicodeClass` (11 classes, default actions, confidence ceilings), `Confidence`, `Action`, `UnicodeFinding`, `Guard` trait + `RejectReason`, `InspectReport`, `CleanReport`, `TextStats` (struct only), `not_established::ALL`, `Vendor` | UCD tables, `build.rs`, every function that takes text: classifier, scrubber, NFKC, homoglyphs, guards, `TextStats::of` — **E1** | `crates/wipemark-core/src/{class,guard,report,vendor}.rs` (650 lines of `src/`) |
| `wipemark-engine` | `RewriteEngine` trait, `EngineInfo` (with `ctx_len: Option`), `SamplingParams`, `ChatRequest`, `Completion`, `EngineError`, `FakeEngine` | every real engine — **E2** | `crates/wipemark-engine/src/lib.rs:138`, `fake.rs` |
| `wipemark-pipeline` | `JobId`, `Action`, `Stage`, `Event`, `PipelineError`, `violates_non_origin` | the state machine, chunking, tactics, the selection loop, scorers, batch — **E4** | `crates/wipemark-pipeline/src/lib.rs:115` |
| `wipemark-models` | manifest (2 entries, commit-pinned, sha256), layout and containment, host probe and `fit`, `default_for_role`, recursive scan, the resumable verifying downloader | signed remote manifest (moved to E9), mirror, GC — rest of **E3** | `crates/wipemark-models/src/*.rs` |
| `wipemark-intake` | the recogniser: 44 magic formats, names, encodings (BOM, UTF-8/16/32, `Other`), text-that-is-a-path, the four-case arbitration | folders and archives expanded (Q-D2) | `crates/wipemark-intake/src/*.rs` |
| `wipemark-store` | SQLite file, migrations, `settings` table | history and queue tables (E4/E7) | `crates/wipemark-store/src/*.rs` |
| `wipemark-secret` | OS credential store behind one type, `Secret` | — | `crates/wipemark-secret/src/lib.rs` |
| `wipemark-log` | rotating file, panic hook, `Elided` | — | `crates/wipemark-log/src/*.rs` |
| `wipemark-i18n` | Fluent catalogues en-US/de/ru, generated `Message`, `Rendering::{Ui,PlainText}`, the catalogue gates | the E1 keys (`unicode-class-*`, `confidence-*`, CLI report lines) — added by E1-6 | `crates/wipemark-i18n/` |
| `wipemark-license` | `LicenseState`, `TrialAllowance`, the rule "Layer A is never locked" | activation, tokens, fingerprint, keychain — **E9** | `crates/wipemark-license/src/lib.rs` |
| `wipemark-image` | container and metadata taxonomy, `StripReport` shape | every parser — **E11** | `crates/wipemark-image/src/lib.rs` |
| `wipemark-app` | four windows, setup, tray, hotkeys, placement, queue, compare, settings (7 sections), MCP server whose tools refuse | Layer A and B behind the surfaces | `apps/wipemark-app/src/` |
| `wipemark-cli` | every command and flag, localized help, four pinned exit codes, a refusal at 2 for every command | every command body — E1 (`inspect`, `clean`) and **E5** (the rest) | `apps/wipemark-cli/src/main.rs:485` |

### 1.3 The consumers already waiting for Layer A

These exist, are tested, and refuse by name today. E1 turns the first
four into working surfaces; the last three are E7 seams and stay as they
are until then (A §7.4).

| consumer | today | after E1 | where |
|---|---|---|---|
| MCP `tools/call inspect` / `clean` | `Tool::refusal()` with `isError: true` | §7.1 JSON report (and cleaned text for `clean`), `structuredContent` | `apps/wipemark-app/src/mcp/protocol.rs:159`, `:260` |
| MCP pane banner | `settings-mcp-tools-pending` ("the server answers, its tools do not yet") | says Layer A works and Layer B is not in this version | `apps/wipemark-app/src/settings.rs:5544` |
| CLI `inspect` / `clean` | parse, log, print `cli-not-implemented`, exit 2 | real work, exit 0/1/3, `--json` | `apps/wipemark-cli/src/main.rs:485-553` |
| `wipemark-i18n` gate `no_message_carries_a_character_layer_a_would_strip` | an independent spelling of the removal set | unchanged — deliberately independent of the classifier | `crates/wipemark-i18n/src/tests.rs:296-336` |
| Compare window | result starts as a copy of the original | E7: `clean(original, &Options::default())` | `apps/wipemark-app/src/compare.rs` |
| panel rows | kind, format, retention sentence | E7: a findings count from `inspect` beside the sentence | `apps/wipemark-app/src/panel.rs:444` |
| queue footer | "cleaning is not in this version" | E7 (S7.1): removed when the Actions menu has "Clean" | `apps/wipemark-app/src/queue.rs` |

---

## 2. What remains, and in what order

```
E1 Layer A ──┬──► E5 CLI rest ─────────────┐
 (this plan) │                             │
             ├──► E7 workspace UI ◄── E4 ◄─┤
             │                        ▲    │
E2 engines ──┴────────────────────────┘    ├──► E9 licence ──► E10 packaging
E3 rest (mirror, GC, signed manifest*) ────┘
E8 models & engine UI (partly done; finishes after E2)
E11 images (phase 2) — depends on core only;  E12 pixels (phase 2b) — own spec
                                               * signed manifest rides with E9
```

**Why this order (the basis).** OV §10 puts E1 first and lets E1 and E3
run in parallel; E5 lands before E6/E7 "and gives agents a usable product
before the GUI exists". E6 was done ahead of its slot, so the order now
reads: **E1 → (E2 ‖ E5 rest) → E4 → E7/E8 → E9 → E10**, E11 any time
after E1. E1 is first for a reason stronger than the table: Layer A is
the only part of the product whose output is *verifiable* (OV §0.1 rule
3), so until it exists the product has no result at all, only windows
that say "not in this version". It is also a hard dependency of Layer B:
the pipeline runs A, then B, then A again (OV §4.2 step 4), and the five
guards that reject a bad rewrite are E1 code (A §6).

---

## 3. The E1 series — documents

| id | document | spec scopes | depends on | unblocks | size | status |
|---|---|---|---|---|---|---|
| **E1-1** | [E1-1-ucd-tables.md](E1-1-ucd-tables.md) — the committed UCD 18.0.0 files, `scripts/fetch-ucd.sh`, the std-only `build.rs`, the generated tables and their query functions, the `Script` enum, character names | S1.1 | — | everything below | ~2 days | done — [reports/E1-1-2026-10-03.md](reports/E1-1-2026-10-03.md) |
| **E1-2** | [E1-2-classifier.md](E1-2-classifier.md) — `class_of`, the context rules that keep orthography (emoji, joining scripts, RTL, flags, Mongolian, Khmer, Hangul), the hit stream, `TextStats::of` | S1.2, S1.4 | E1-1 | E1-3, E1-5 | ~3 days | done — [reports/E1-2-2026-10-03.md](reports/E1-2-2026-10-03.md) |
| **E1-3** | [E1-3-scrubber-and-nfkc.md](E1-3-scrubber-and-nfkc.md) — the public API (`inspect`, `clean`, `Options`, `Cleaned`), the report changes, aggregation, the second pass, NFKC (UAX #15) and its conformance gate, idempotence, the JSON form of the report, `fixtures/text/` | S1.3, S1.5 | E1-2 | E1-4, E1-6 | ~4 days | done — [reports/E1-3-2026-10-03.md](reports/E1-3-2026-10-03.md) |
| **E1-4** | [E1-4-homoglyphs.md](E1-4-homoglyphs.md) — confusables in both directions, the mixed-word rule, whole-word redraws, the 10 % rule | S1.6 | E1-3 | E1-6 | ~2 days | done — [reports/E1-4-2026-10-03.md](reports/E1-4-2026-10-03.md) |
| **E1-5** | [E1-5-guards.md](E1-5-guards.md) — the five guards and `default_guards()` | S1.7 | E1-1, E1-2 (`TextStats`/letter shares) | E4 | ~2 days | done — [reports/E1-5-2026-10-03.md](reports/E1-5-2026-10-03.md) |
| **E1-6** | [E1-6-mcp-and-cli.md](E1-6-mcp-and-cli.md) — MCP `inspect`/`clean` and CLI `inspect`/`clean` wired, the catalogue keys, the third shelf on every answer, exit codes 0/1/3, `with_infix` moved into `wipemark-intake` | S1.8 | E1-3, E1-4 | E5, E7 | ~3 days | done — [reports/E1-6-2026-10-03.md](reports/E1-6-2026-10-03.md) |
| **E1-7** | [E1-7-closure.md](E1-7-closure.md) — the live gate, `docs/architecture/layer-a.md` completed, CLAUDE.md and the skeleton tables updated, the closure TEXT in Watchword | — | all of the above | E2+ | ~1 day | done — [reports/E1-7-2026-10-03.md](reports/E1-7-2026-10-03.md) |

Statuses an implementer may write: *not started*, *in progress*, *done*
(with the report's file name), *blocked* (with the reason). Reports go
to `docs/plan/reports/<id>-<YYYY-MM-DD>.md`.

### 3.1 How to dispatch

1. **Sequential on the branch.** E1-1 → E1-2 → E1-3 → E1-4 → E1-6 → E1-7
   is a chain; each one starts from the previous one committed on
   `feat/e0-e6-shell`.
2. **One parallel lane.** E1-5 (guards) needs only E1-1 and the
   `TextStats`/letter-share part of E1-2. Once E1-2 is committed it can
   run beside E1-3/E1-4 in a worktree created **from `feat/e0-e6-shell`**
   (`git worktree add ../wipemark-e1-5 -b e1/guards feat/e0-e6-shell`)
   and is merged back into `feat/e0-e6-shell` by the coordinator — never
   into `main`.
3. **Agent.** `implementer-xhigh` (`~/.claude/agents/implementer-xhigh.md`):
   "the prompt you receive names one self-sufficient specification
   document; that document plus the repository's CLAUDE.md are your whole
   brief". Prompt shape: *"Implement `docs/plan/E1-n-….md` in
   `~/self/wipemark-app` on branch `feat/e0-e6-shell`. Commit when done
   (one commit, message `E1-n: <title>`), do not push. Write the report
   the document asks for."*
4. **Checkpoints worth stopping for.** After E1-1 (look at the generated
   table sizes — A §3.2 forbids names outside the finding-capable set;
   the release-binary delta is measured at E1-6, the first time a binary
   calls the tables); after E1-3 (run `clean` by hand over the
   fixtures and read the report); after E1-6 (the live gate in E1-7 is
   the first time an agent gets a real answer from this product).
5. **Push.** The coordinator pushes `feat/e0-e6-shell` after each landed
   document, with the four gates green; PR #1 grows with it.

### 3.2 Module map after E1 — the contract every document builds against

No document may rename, move or re-sign anything in this section. If an
implementer finds it wrong, they implement it as written and say so in
the report; the coordinator changes it here first.

```
crates/wipemark-core/                 zero dependencies — normal, dev AND build (check-dep-direction.sh:133)
  Cargo.toml                          gains `build = "build.rs"` only; still no [dependencies]
  build.rs                            E1-1  std-only generator → $OUT_DIR/tables.rs
  ucd/                                E1-1  UCD 18.0.0, committed, flat + emoji/
    README.md  SHA256SUMS
    UnicodeData.txt  DerivedCoreProperties.txt  PropList.txt  Scripts.txt
    StandardizedVariants.txt  DerivedNormalizationProps.txt  NormalizationTest.txt
    confusables.txt  emoji/emoji-data.txt
  src/lib.rs                          E1-3  the public API (below); module list grows per document
  src/tables.rs                       E1-1  include! of the generated file + pub(crate) query functions
  src/script.rs                       E1-1  `Script`
  src/name.rs                         E1-1  `name_of`
  src/class.rs                        exists; E1-2 adds class_of + JOINING_SCRIPTS; E1-3 adds Action::Replace, as_str
  src/context.rs                      E1-2  the two pre-passes and the context rules → Hit stream
  src/stats.rs                        E1-2  TextStats::of, LetterShares
  src/scrub.rs                        E1-3  collect_hits, the decision pass, aggregation, output, second pass
  src/nfkc.rs                         E1-3  UAX #15 NFKC
  src/json.rs                         E1-3  the §7.1 JSON form, std-only writer
  src/homoglyph.rs                    E1-4  confusable detection
  src/guard.rs                        exists; E1-5 adds the five guards + default_guards
  src/report.rs                       exists; E1-3 changes CleanReport/InspectReport/NormKind
  tests/                              public-API tests only (D24); pub(crate) tests live in their module
fixtures/text/                        the first document that needs a fixture creates it (E1-2); E1-3 adds the
                                      class set, E1-4 its own; a name that exists with different bytes is a stop
.gitattributes                        E1-1 creates (D25): crates/wipemark-core/ucd/**/*.txt -text -diff,
                                            crates/wipemark-core/ucd/SHA256SUMS -text, fixtures/text/** -text
scripts/fetch-ucd.sh                  E1-1
NOTICE                                E1-1 adds the Unicode License v3 section
```

### 3.3 Interfaces between the documents

Visibility is part of the contract: `pub` is the crate's public API and
is used by the applications; `pub(crate)` is shared between documents
inside `wipemark-core` only.

**E1-1 provides** (`src/tables.rs`, `src/script.rs`, `src/name.rs`):

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
// (as built: `#[cfg(test)]` since E1-7 — no shipped caller; three tests read Scripts.txt through it)

// name.rs — re-exported as wipemark_core::name_of
pub fn name_of(c: char) -> Option<std::borrow::Cow<'static, str>>;
// Some(Borrowed(UCD name)) for a finding-capable code point that has one;
// Some(Owned("<private-use-E000>" / "<noncharacter-FDD0>" / "<reserved-E0080>")) for one without;
// None for anything that can never be a finding — except the twelve D19 code points, which keep their UCD
// names (they are Default_Ignorable in UCD, and dropping them alone would label them <reserved-…>).
```

**E1-2 provides** (`src/class.rs`, `src/context.rs`, `src/stats.rs`):

```rust
// class.rs
pub fn class_of(c: char) -> Option<UnicodeClass>;     // context-free membership, first claim in A §4.1 order;
                                                      // never returns Homoglyph (that needs a word; E1-4)
pub(crate) const JOINING_SCRIPTS: &[Script];          // A §4.2, with the comment saying why a list

// context.rs
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

// stats.rs
impl TextStats { pub fn of(text: &str) -> TextStats; }   // A §5.5
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LetterShares { pub letters: usize, pub latin: f32, pub cyrillic: f32, pub cjk: f32, pub other: f32 } // percent, 0..=100
pub(crate) fn letter_shares(text: &str) -> LetterShares;
```

**E1-3 provides** (`src/lib.rs`, `src/scrub.rs`, `src/nfkc.rs`, `src/json.rs`, changes to `class.rs`/`report.rs`):

```rust
pub use tables::UNICODE_VERSION;   // already present — added by E1-1
pub use name::name_of;             // already present — added by E1-1
pub fn inspect(text: &str, options: &Options) -> InspectReport;
pub fn clean(text: &str, options: &Options) -> Cleaned;

#[derive(Debug, Clone, PartialEq)]
pub struct Cleaned { pub text: String, pub report: CleanReport }

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options { pub aggressive: bool, pub nfkc: bool, pub normalize_spaces: bool, pub keep_soft_hyphen: bool }
impl Options { pub fn action_for(&self, class: UnicodeClass) -> Action; }

pub enum Action { Remove, Replace, Keep }               // NormalizeToSpace folds into Replace (A §5.1)
impl Action     { pub fn as_str(self) -> &'static str; } // "remove" | "replace" | "keep"
impl Confidence { pub fn as_str(self) -> &'static str; } // "confirmed" | "probable" | "informational" | "likely-false-positive"
pub enum NormKind { SpaceToAscii, Nfkc, Homoglyph }      // Homoglyph added here so E1-4 only uses it
impl NormKind   { pub fn as_str(self) -> &'static str; } // "space-to-ascii" | "nfkc" | "homoglyph"

pub struct InspectReport { pub findings: Vec<UnicodeFinding>, pub kept: Vec<UnicodeFinding>,   // `kept` is new (D5)
                           pub suspicious: bool, pub stats: TextStats, pub unicode_version: &'static str }
pub struct CleanReport { pub findings: Vec<UnicodeFinding>, pub kept: Vec<UnicodeFinding>,
                         pub suspicious: bool, pub stats: TextStats,                       // over the SOURCE (D28)
                         pub removed: Vec<(UnicodeClass, u32)>, pub normalized: Vec<(NormKind, u32)>,
                         pub output_len: usize, pub unicode_version: &'static str }        // no Default (A §1)
impl InspectReport { pub fn to_json(&self) -> String; }  // A §7.1, not_established included (D9)
impl CleanReport   { pub fn to_json(&self) -> String; }

// scrub.rs — the seam E1-4 extends
pub(crate) fn collect_hits(text: &str, options: &Options) -> Vec<Hit>;   // context::hits ∪ (E1-4) homoglyph::hits, sorted by `at`;
                                                                         // `options` reserved for per-class overrides (Q-A1), unused under D3
// nfkc.rs
pub(crate) fn nfkc_counted(text: &str) -> (String, u32);   // the production entry: text and the D27 count from one run
pub(crate) fn nfkc(text: &str) -> String;                  // thin wrapper, for tests
```

**E1-4 provides** (`src/homoglyph.rs`, one call added in `scrub::collect_hits`):

```rust
pub(crate) fn hits(text: &str) -> Vec<Hit>;  // class Homoglyph, confidence Probable, replacement Some(letter), in source order
```

**E1-5 provides** (`src/guard.rs`): `PlaceholderGuard`, `NumbersGuard`,
`LengthDriftGuard { min: f32, max: f32 }`, `ScriptGuard { max_delta_pp:
f32, min_letters: usize }`, `IdentifierGuard`, each `impl Guard` and
`Default`; `pub fn default_guards() -> Vec<Box<dyn Guard>>` in that
order; `RejectReason::PlaceholderInvented { index: usize }`. All
re-exported from `lib.rs`.

**E1-6 provides** (`apps/`, `crates/wipemark-intake`,
`crates/wipemark-i18n`): `wipemark_intake::name::with_infix` (moved from
`apps/wipemark-app/src/retention.rs:468`, re-exported by retention so its
callers do not change), the MCP and CLI bodies, the catalogue keys.

---

## 4. Decisions taken in this plan (change them here, not in a document)

Each one either fills a gap the E1 spec leaves, or settles a place where
the spec and the code disagree. The basis is given so a later reader can
tell a decision from an accident.

| # | decision | basis |
|---|---|---|
| **D1** | The plan and its documents live in `docs/plan/` and are written in English, like every document in `docs/`; copies go to Watchword as dated FILE entries (ttl 0). | CLAUDE.md "Specs": `docs/` is the source of truth for anything durable, Watchword holds copies; the implementer agent reads English docs. |
| **D2** | All work is on `feat/e0-e6-shell`; worktrees for parallel lanes branch from it and merge back into it. | Owner, 2026-10-03. |
| **D3** | **Homoglyph detection always runs**; `aggressive` changes only the action (Keep → Replace). Without `aggressive` a homoglyph is reported in `kept` at `Probable`. | A §4.1 table gives `Homoglyph` the default `Keep` and A §5.2 says `aggressive → Homoglyph: Replace` — a default action is only meaningful for a finding that exists without the flag. `UnicodeClass::requires_aggressive` (`class.rs:147`) already says "only *act*". |
| **D4** | `suspicious` ⇔ some finding in `findings ∪ kept` has confidence ≥ `Probable`. | A §4.3 says "a finding with confidence ≥ Probable"; with D3 a kept homoglyph is exactly the case a user must be told about. Kept-by-context findings are `LikelyFalsePositive` and do not count; knob-kept exotic spaces are `Informational` and do not count. |
| **D5** | `InspectReport` gains `kept: Vec<UnicodeFinding>`. | A §7.1: "`kept` у inspect — те, что *были бы* оставлены" — the JSON has it, the type (`report.rs:62`) does not. |
| **D6** | Findings are aggregated by `(codepoint, class, confidence, acted)`, and land in `findings` when acted on, in `kept` when not. | A §4.3 aggregates by `(codepoint, class, confidence)` and requires a kept and a removed U+200D to be two rows (their confidences differ anyway). Knobs are global, so within one run a class is never both acted on and knob-kept; the case `acted` separates is a homoglyph hit with no resolvable replacement, which is kept rather than deleted (E1-3 author, 2026-10-03). |
| **D7** | `name_of` returns `Option<Cow<'static, str>>`, not `Option<&'static str>`. | A §5.1 types it `&'static str` but A §3.2 synthesises `<private-use-E000>` for ranges of ~137 000 code points; a static string per code point is the 1.5 MB A §3.2 forbids. Labels follow the UCD code point label convention (`<private-use-XXXX>`, `<noncharacter-XXXX>`, `<reserved-XXXX>`). |
| **D8** | `Script` carries `Bopomofo`. | A §3.2's enum omits it, A §5.5 counts it in `cjk_ratio`. |
| **D9** | The §7.1 JSON form is produced **in `wipemark-core`** by a small std-only writer (`src/json.rs`), and it always includes `not_established` (the three ids of `not_established::ALL`). MCP and CLI embed it; MCP parses it back with `serde_json` for `structuredContent`. | One format with two writers drifts. No library both sides can use has `serde_json` (`wipemark-pipeline/Cargo.toml` has none), core may not have it, and the report carries no user text — only ids, `U+XXXX`, UCD names and numbers — so the escaper is a dozen lines. Putting the third shelf in the writer makes "the surface forgot it" impossible rather than tested-for; `every_layer_a_answer_carries_the_third_shelf` stays in MCP and CLI as A §5.5 asks. |
| **D10** | `with_infix` and `RESULT_INFIX`/`ORIGINAL_INFIX` move to `wipemark_intake::name`; `retention.rs` re-exports them. | The CLI must name results exactly as the app does (A §7.3, "правило Retention для beside"), it cannot depend on the app crate, and `wipemark-intake` is already a dependency-free leaf both applications may use (`check-dep-direction.sh`, `LIBS`). A file name is the intake crate's vocabulary (`name.rs`). |
| **D11** | CLI input: UTF-8 (BOM or not), UTF-16LE/BE and UTF-32LE/BE are decoded; `Encoding::Other` and non-text exit **3** "not read is not clean". Output to a file is written **in the input's encoding, with its BOM if it had one**; stdout is always UTF-8. A path that does not exist exits **2**; one that exists but cannot be read exits **3**. | A §2 and §7.3 name UTF-8/UTF-16; UTF-32 is marked by a BOM `wipemark-intake` already recognises (`text.rs:43-49`), and refusing it would be an exit 3 the user cannot fix. Writing a UTF-16 file back as UTF-8 would be a change Layer A did not report. Missing path = a usage error; unreadable = inconclusive. |
| **D12** | `clean --json` prints `{"report": <§7.1>, "written": "<path>"}` when the result went to a file, and `{"report": <§7.1>, "text": "<cleaned>"}` whenever the result goes to standard output (stdin without `-o`, or `-o -`). `inspect --json` prints the §7.1 report alone. Whenever stdout carries the cleaned text, the human-readable report goes to stderr. | A §7.3 says "§7.1 in stdout and nothing else", but a stdin `clean` has nowhere else to put the text; the MCP `clean` result is `{text, report}` (A §7.2), so the CLI takes the same two field names. |
| **D13** | **Q-A4 (MCP text limit) is answered by the transport**: the server already refuses a body over 1 MiB with `413` before dispatch (`server.rs:80`, `:380`), never truncating — an HTTP failure, not the `isError` result A §7.2 would prefer, which is the accepted trade. No second limit in the tool; the tool descriptions say "up to about 1 MB of text". E1-6 adds the missing gate `a_body_over_the_limit_is_refused_whole`. | A §9 Q-A4 asks for a refusal rather than silent truncation; the transport gives exactly that, and a second, smaller limit would be a second number to keep in step. |
| **D14** | `a_tool_that_cannot_run_refuses_rather_than_reporting_nothing` (`protocol.rs:377`) **keeps its name and changes its call**: it sends `tools/call` *without* `text` and asserts an `isError` result naming the tool and the missing argument. | Today it sends `{"text":"hi"}` and asserts "not implemented" — it will go red the moment the tools run. A §7.2 says the test "stays: it is about the missing argument" — that is its meaning after E1, and the name still describes it. |
| **D15** | The MCP pane's last banner line becomes a new key `settings-mcp-tools-layer-a` ("the tools clean with Layer A; rewriting is not in this version") and `settings-mcp-tools-pending` is deleted from all three catalogues. | A §7.2; CLAUDE.md "No epic number leaves this repository" — the line says "not in this version", never "E2". |
| **D16** | Character names are an exception to "every string a person reads comes from the catalogue", written into `docs/architecture/i18n.md` and one sentence of CLAUDE.md by E1-6. | A §5.5 and Q-A6: a UCD name is an identifier of the standard, like `U+200B` or `Vendor::as_str()`. If the owner vetoes, the Inspector shows only `U+XXXX` and the class, and `name_of` stays for `--json`. |
| **D17** | `fixtures/text/` is byte-exact under `.gitattributes` `fixtures/text/** -text` (D25); every fixture has an assertion (fixtures/README.md rule 2). | `fixtures/README.md`; A §8. |
| **D18** | `UNICODE_VERSION` is read from the headers of the eight UCD files that have one; files of two versions fail `cargo build`. `UnicodeData.txt` has **no header** (its first line is data), so it is bound to the version another way: the code points it assigns, minus `Co` and `Cs`, must equal exactly the set `Scripts.txt` lists (172 873 in 18.0.0; a 17.0.0 copy differs by 13 007 and fails the build). | A §3.2, corrected against the real 18.0.0 files by the E1-1 author (2026-10-03). |
| **D19** | U+1BCA0–1BCA3 (Duployan shorthand format controls) and U+1D173–1D17A (musical symbol format controls) are **never findings**, excluded **by name** from `DefaultIgnorable`. | A §4.1 lists them as "Cf not marked Default_Ignorable"; in 18.0.0 both ranges *are* `Default_Ignorable_Code_Point` (`DerivedCoreProperties.txt`). The spec's intent stands — they format their own notation and render with it — so the exclusion is explicit; `script_format_controls_are_never_findings` covers them. |
| **D20** | A paragraph is RTL only through a character with `Bidi_Class` R/AL **that is a letter or mark** (gc `L*`/`M*`), never through a `Bidi_Control`. | U+200F RLM is `R` and U+061C ALM is `AL`: counting them would let a stray RLM in an English paragraph protect itself. Gate `a_stray_rlm_does_not_protect_itself`. |
| **D21** | *(revised 2026-10-03)* A homoglyph's replacement: (1) the prototype itself, when it is a letter of the word's script with the same case as the source; else (2) among the letters of the word's script that share the skeleton and the case **and have no decomposition** (D42), the one with the **lowest code point**; else (3) no finding. A whole-word redraw happens only when every letter resolves. | A §5.4 rule 3 ("exactly one letter with the same skeleton") fails its own test in 18.0.0 — five Latin letters share the skeleton `a` (U+0061, U+0251, U+AB64, U+FF41, U+1DF5A), skeletons drop case (U+0049 → U+006C). The first revision ("ambiguity is not evidence") then lost the commonest attack letters: Latin c, e, i, o, w, y, I, Y have two or three Cyrillic twins, so a Latin `o` in a Cyrillic word went unreported. The lowest remaining code point is the everyday letter in all 12 ambiguous cases in 18.0.0 (E1-4 author's model). |
| **D22** | The "or from the Halfwidth and Fullwidth Forms block" clause of A §3.2/§5.4 is dropped. | Fullwidth Latin (U+FF21–FF5A) is `Script=Latin` and already covered; the block clause only added 82 halfwidth Katakana/Hangul/punctuation sources (e.g. U+FF89 → U+002F) that are not homoglyphs of Latin/Cyrillic/Greek words. |
| **D23** | `nfkc_conforms_to_the_unicode_test_file` runs all **six** parts of `NormalizationTest.txt`. | A §5.3 says parts 0–3; the 18.0.0 file has `@Part0` … `@Part5`. |
| **D24** | Tests that exercise `pub(crate)` items are unit tests in their module (they may `include_str!` from `ucd/` or `fixtures/`); only tests that use the public API go to `crates/wipemark-core/tests/`. | An integration test cannot see `pub(crate)`. §0.4 reworded. |
| **D25** | `.gitattributes`: `crates/wipemark-core/ucd/**/*.txt -text -diff`, `crates/wipemark-core/ucd/SHA256SUMS -text`, `fixtures/text/** -text`. | A pattern with a slash is anchored at the root: `ucd/*.txt` matched nothing, and `*` would not reach `emoji/` (checked with `git check-attr`). |
| **D26** | With `nfkc` on, `clean` runs NFKC and the decision pass **in rounds until the pass acts on nothing** (at most 8, `debug_assert` at the cap). | A §5.3's single second pass is not idempotent: U+2139 U+FE0F U+0301 → U+0069 U+0301, which is not NFKC; cleaning again gives U+00ED. Gate `nfkc_rounds_reach_a_fixed_point`. |
| **D27** | `(Nfkc, n)`: `n` is the number of input code points NFKC did not carry through unchanged, summed over the rounds; the entry is present whenever `nfkc` is on, even when `n` is 0. | A §1/§5.1 add the counter without defining it. |
| **D28** | `CleanReport` gains `suspicious` and `stats`, both over the **source** text by the same rules as `inspect` (D4; `TextStats::of` once). | A §7.1's example of a clean report shows both, and the CLI's exit code for `clean` is decided over the input (A §7.3); `to_json` has no text to compute them from. |
| **D29** | The JSON writer escapes every character from U+007F upward as `\u` + four lowercase hex digits (surrogate pairs for astral), so every report is pure ASCII by construction. | The report carries no user text, only ids, `U+XXXX`, UCD names and numbers; ASCII-only makes "a report carries an invisible character" impossible rather than tested-for. |
| **D30** | The mutation for `a_single_zero_width_space_is` is "compute `suspicious` from `kept` only", not A §8's "compare with Informational". | U+200B is Confirmed, above either threshold, so lowering the threshold cannot paint that test red (E1-3 author). |
| **D31** | `wipemark-cli clean` exits **1 when the input had findings**, even though they were cleaned; `apps/wipemark-cli/src/main.rs:8-15` and the root `README.md` ("marks remain after cleaning") are rewritten to say so. | A §7.3: a pre-commit hook wants to know what *was* there. |
| **D32** | A malformed tool argument (missing `text`, wrong type) is an `isError` **result** naming the argument; a non-object `arguments` is a JSON-RPC `-32602`. | MCP 2025-06-18 lists invalid arguments among protocol errors, revision 2025-11-25 moves input validation to tool execution errors — the reading A §7.2 and D14 need, so the model reads the refusal. |
| **D33** | A leading U+FEFF is not a finding and is **kept** in every output — the CLI's stdout, `--json` `text`, an MCP `clean` result. Any message the server builds from client input spells it (`spelled()`), so the server itself never echoes an invisible character. | A §4.1 (a BOM at position 0 is not a finding); CLAUDE.md "Nothing the MCP server says…" — E1-6 found the unknown-method and unknown-tool messages (`protocol.rs:252-255`, `:275-281`) echoing client strings verbatim. |
| **D34** | Every base or neighbour a keep rule reads must itself be **non-finding-capable** (a letter, mark, digit or emoji). | A §5.3's idempotence argument assumes it but the rules as written break it: U+17B4 after a ZWNJ, U+180B after a ZWJ, a Hangul filler after a filler — the first two make `clean(clean(x)) ≠ clean(x)` (E1-2 author's prototype). |
| **D35** | An exotic space **moves `prev_kept`** whatever the knobs say. | `hits` has no `Options`, and `normalize_spaces` must not change what context sees: U+0645 U+00A0 U+200C U+0631 otherwise cleans differently on a second run. |
| **D36** | Every selector rule (VS15/VS16, VS1–VS14, IVS, Mongolian FVS) reads the **immediately preceding code point**, not `prev_kept`. | A writes `prev` (undefined) for the selector rows and `prev_kept` for the Mongolian one; with `prev_kept` a run of up to 240 selectors after one emoji or ideograph would be kept — a byte channel. Deviates from A's literal Mongolian row on purpose. |
| **D37** | ASCII is never a ZWJ side or a tag base, though ASCII digits, `#` and `*` are `Emoji=Yes`; VS15/VS16 after them stay kept (keycaps, `emoji-variation-sequences.txt`). | Taken literally, A keeps a ZWJ between two digits in every number. |
| **D38** | A Hangul filler (U+115F, U+1160, U+3164, U+FFA0) is kept when `prev_kept` **or `next`** is a Hangul letter. | U+115F starts its syllable; A's `prev_kept`-only rule breaks a vowel-only syllable at the start of a word. Keeps slightly more than A. |
| **D39** | Tag characters are kept **only in a valid emoji tag sequence per UTS #51 Annex C.1**: U+1F3F4, then tags from U+E0030–E0039 and U+E0061–E007A, then U+E007F, 32 code points at most. Any other emoji + tags + U+E007F is a finding. | A's "Emoji=Yes base" would keep the ASCII-smuggling shape — any emoji followed by arbitrary tag text — and never call the text suspicious, which is the carrier this product most needs to catch. Coordinator's decision, 2026-10-03; the owner may revisit. |
| **D40** | A whole word is redrawn into the paragraph's script only if **every letter is a confusables source whose prototype is a letter of the paragraph's script** (E1-4 W5/H6). | A §5.4's 10 % rule only protects short paragraphs and quotations: a single acronym in a long Russian paragraph (BMW, HTTP, SSH, Java) or an IPA ɑ would be redrawn into Cyrillic and mark the text suspicious under D4. |
| **D41** | The five idempotence repairs of E1-4 §4.5 H1–H5: an invisible character inside a word does not split it; a script tie goes to the paragraph's script only when it is in the tie; the paragraph basis credits whole words; the 10 % share is per word; whole-word redraws are judged on resolved letters. | The literal A §5.4 breaks `clean(clean(x)) == clean(x)` on 1 398, 118, 67, 153 and 30 strings of a 10 000-string corpus respectively (under the revised D21), without each repair (E1-4 author's model). |
| **D42** | A twin **with a decomposition** is never a replacement (e.g. U+0063 → U+03F2 refused); the veto applies **before** the lowest-code-point choice of D21. | E1-3's convergence argument (D26) assumes homoglyph replacements are NFKC-stable. |
| **D43** | `IdentifierGuard`'s edge-trim set adds the curly quotes U+2018, U+2019, U+201C, U+201D, U+201E, U+2039, U+203A and the backtick U+0060 to A §6's `.,;:!?()[]{}"'«»<>`. | A rewrite that straightens or curls the quotes around an identifier would otherwise be thrown away as having lost it. |
| **D44** | E1-5's own choices G1–G12 (E1-5 §4.12) are adopted: ASCII-only canonical placeholder digits fitting `usize`; "as often as in the source"; a fixed order of reasons; `ScriptDrift` reports the first script over the limit in the order latin, cyrillic, cjk, other, with a signed delta; an empty source passes only an empty candidate; guard names `placeholder`, `numbers`, `length-drift`, `script`, `identifier`. | A §6 leaves them open; each is a format or a tie-break that has to be one thing. |

---

## 5. Questions for the owner (what the plan does while each is open)

| # | question | blocks | meanwhile |
|---|---|---|---|
| Q2 | local engine: `llama-cpp-2` or `mistral.rs` | E2 / S2.5 | Nothing in E1 depends on it. `wipemark-uzu-evaluation-2026-09-11`: uzu covers Metal on Apple Silicon only (no Vulkan, no CUDA) and is not a candidate for the whole grid; `local-llama` is already named after llama.cpp. |
| Q3 | stylometric "AI-likelihood" score as an informational finding | E4 / S4.6 | Not built. The spec recommends no; nothing in E1 would host it. |
| Q4 | default `pivot_lang` for back-translation, prompt language | E4 / S4.4 | — |
| Q5 | v1 platforms | E10 | macOS first, as everything platform-specific today (`pasteboard.rs`, `tray.rs`, hotkeys). |
| Q6 | trial policy | E9 / S9.2 | Layer A is never gated (`wipemark-license` tests). |
| Q7 | editor: Merge's own or gpui-component's | E7 / S7.2 | The Compare window already uses gpui-component's editor with the vendored `LineDecorationProvider` patch — see risk R1. |
| Q8 | "Sign" mode — your own invisible mark | backlog or v1 | Out of E1. Technically the same tables. |
| Q-A1 | per-class overrides and a "Clean" Settings page | E7/E8 | E1 ships the four knobs of `Options` only, no preference rows (A §7.4 — `every_persisted_preference_has_a_row` would need widgets). |
| Q-A2 | check pairing of bidi embeddings in an RTL paragraph | after real files | Not checked (A §4.2). |
| Q-A3 | unassigned code points | E7 | Not findings (A §2). |
| Q-A4 | MCP text limit | — | **Answered by D13.** |
| Q-A5 | `arabic_ratio`, `hebrew_ratio` in `TextStats` | E4 | Not added. |
| Q-A6 | names exception to the i18n rule | E1-6 | **Taken as D16**; kept by the owner 2026-10-03. |
| Q-A7 | known false positives E1 leaves unprotected: legacy Malayalam chillu (consonant + virama + ZWJ at a word end), U+034F COMBINING GRAPHEME JOINER, German ligature-breaking ZWNJ | after real files | Removed as findings; listed in E1-2 §4.2.6. Each is one keep rule when a real document shows it matters. The owner kept this, and D21, on 2026-10-03. |
| Q-A8 | D39 narrows tag sequences to emoji flags (Annex C.1) | — | Decided by the coordinator for safety; **kept by the owner 2026-10-03**. |
| Q-D1–Q-D6 | drag-and-drop: paste ⌘V, folders and archives, drop position, size limits, CLI exit on `Disagreed`, UTF-16 without BOM | E7 / E5 | Unchanged by E1. Q-D5 meets E1-6: the CLI reads a file whose name and bytes disagree by its bytes and says so on stderr; the exit code is decided by findings as for any file. |

---

## 6. Risks to watch during E1

- **Table size.** Names only for finding-capable code points; ranges, not
  sets. E1-1 records the size of the generated `tables.rs`; E1-6 records
  the release-binary delta, since no binary calls the tables before it.
- **Dead code between documents.** Nothing calls the E1-1 lookups until
  E1-3, so E1-1 scopes `#![cfg_attr(not(test), allow(dead_code))]` to
  `tables.rs` and `script.rs`; each later document removes it for what it
  now calls, and E1-7 removes what remains. *(Resolved at E1-7: one
  module-wide `expect(dead_code)` stays in `tables.rs` for five generated
  lookups with no caller yet; `expect` fails the build when the last one
  gets one.)*
- **A test that cannot fail.** Every protection in A §8 is painted red
  by the stated mutation before it counts; the report lists each one.
  CLAUDE.md: "a test that stays green with its subject deleted is worse
  than no test".
- **Idempotence under NFKC.** The orphaned VS16 after U+2139 U+FE0F → `i` is the
  known trap (A §5.3), and one pass after NFKC is not enough
  (U+2139 U+FE0F U+0301 cleans to U+0069 U+0301, which is not NFKC) —
  the protection is NFKC and the pass in rounds until the pass acts on
  nothing (D26).
- **Russian prose under `aggressive`.** The mixed-word rule (A §5.4) is
  what keeps the product from rewriting its owner's language.
- **The binary size budget** (< 60 MB without models, OV §8) — Layer A's
  tables are a few hundred KB if built as A §3.2 says.

---

## 7. Beyond E1 — the remaining epics in detail

Each epic below becomes a series of self-sufficient documents of its own
when E1 has landed. What follows is what that series will be written
from: what already exists (with `file:line`), what to build, the basis,
the gate the overview set, and the open edges.

### E2 — engines (`wipemark-engine`)

- **Exists.** `RewriteEngine` (`crates/wipemark-engine/src/lib.rs:138`:
  `info`, `complete`, `warmup`, `unload`), `EngineInfo` with
  `ctx_len: Option` (an endpoint's window is the server's business),
  `SamplingParams`, `ChatRequest`, `Completion`, `EngineError`,
  `FakeEngine` (`fake.rs`). On the app side everything that *configures*
  an engine: `engine.rs` (vocabulary, `refusal` — default-deny past this
  machine, no key over plaintext, no userinfo in a base URL),
  `profile.rs`, `duty.rs` (who rewrites, announced fallbacks) and
  `duty::engine_for` (`apps/wipemark-app/src/duty.rs:684`), which refuses
  with `EngineError::NotImplemented` and **never falls back to
  `FakeEngine`**.
- **Build.** S2.1 trait/types (done); S2.2 `OpenAiCompatEngine` — `POST
  {base}/v1/chat/completions`, `stream: true`, SSE parser, `temperature`,
  `top_p`, `seed`, `max_tokens`, `reasoning_effort` (default `none`),
  **no redirects**, `http(s)` only, connect/read timeouts, retries only
  before the first byte; plus Ollama's native `/api/chat`
  (`docs/sdd/layer-b-rewrite-reference.md` §1); S2.3 a fake HTTP server
  in tests (drop, 429, redirect, slow stream); S2.4 the key from
  `wipemark-secret` filed under `engine::account_of`; S2.5 `LlamaEngine`
  behind `local-llama` (GGUF, mmap, `n_gpu_layers`, `n_ctx` 8192,
  cancel between decode steps, RAM pre-check from the manifest's
  `min_ram_mb`); S2.6 Metal/CUDA builds.
- **Basis.** OV §4.1; `docs/architecture/engine-settings.md`,
  `who-rewrites.md`; `docs/sdd/layer-b-rewrite-reference.md` §1–2 (the
  wire formats and the transport's security rules, read out of upstream).
- **Gate (OV §10).** A redirect carrying a key is an error, not a request;
  cancel mid-stream < 500 ms; llama live gate on a Mac (Metal) and on the
  CUDA host.
- **Open.** Q2 (engine library). The rule `engine_for` must keep: a
  decision becomes an engine or a refusal, never plausible text with no
  model behind it.

### E3 — the rest of models (`wipemark-models`)

- **Exists.** Almost all of it (`docs/architecture/model-downloads.md`).
- **Build.** The mirror (`models.mirror_url`), GC of unused weights, and
  — with E9's key — Ed25519 verification of a fetched manifest
  (`manifest.rs:22-33` already refuses everything a mirror could
  smuggle in). `models verify` / `rm` / `pull` / `list` in the CLI are E5.
- **Gate.** A manifest without a valid signature is refused; a byte
  swapped in a weight fails `verify`.

### E4 — the pipeline (`wipemark-pipeline`)

- **Exists.** The vocabulary (`JobId`, `Action`, `Stage`, `Event`,
  `PipelineError`) and the non-origin rule
  (`crates/wipemark-pipeline/src/lib.rs:115`).
- **Build.** S4.1 format parsing (Markdown, HTML text nodes, code) and
  protected spans → `⟦n⟧` placeholders; S4.2 chunking under `ctx_len ×
  0.4` with the previous chunk's last two sentences as context; S4.3 the
  state machine with events and cancellation (`flume::Receiver<Event>`,
  CLAUDE.md "Nothing blocks the GPUI thread"); S4.4 tactics and prompt
  templates from config; S4.5 candidates × rounds with escalation; S4.6
  scorers — `divergence` (1 − bigram Jaccard) always, `keyed_gumbel` when
  a key is configured; S4.7 the non-origin rule in the loop; S4.8 the
  batch queue, persisted (the store's queue table) and surviving `kill -9`.
  Layer A runs before and after Layer B on every chunk; the five E1
  guards reject candidates — **after** Layer A has cleaned each candidate
  (a model that slips U+200B into an identifier must not lose the
  candidate to `IdentifierGuard`), and `Event::CandidateRejected` should
  carry the structured `RejectReason` rather than its English `Display`
  (`crates/wipemark-pipeline/src/lib.rs:91` is a `String` today), so the
  surface localizes it. `Guard::check` takes two `&str`, so `ScriptGuard`
  recomputes letter shares per call; E4 may cache.
- **Basis.** OV §4.2–4.5; `docs/sdd/layer-b-rewrite-reference.md` §3–6
  (every upstream prompt verbatim, the deterministic humanizer pass, the
  selection loop, the strategy DSL) and §7 (what Wipemark takes and must
  not).
- **Gate.** On `FakeEngine`: guards reject a lost placeholder, number,
  length; `keyed_gumbel` p-value separates marked from unmarked synthetic
  text (both sides painted red); the queue survives `kill -9`.
- **Open.** Q3, Q4; Q-A5.

### E5 — the rest of the CLI

- **Exists.** The whole argument surface and the exit codes
  (`apps/wipemark-cli/src/main.rs:50-60`, `:93-150`), localized help, the
  language pre-parse; after E1-6, `inspect` and `clean`.
- **Build.** `rewrite` (needs E2+E4; `--force` for the non-origin rule),
  `audit <dir>` with `--json` and `--sarif` (exit 3 when any file could
  not be read), `models list|pull|verify|rm` over `wipemark-models`,
  in-place writing behind an explicit per-run flag (never a preference —
  `docs/architecture/retention.md`).
- **Basis.** OV §7; `docs/sdd/layer-b-rewrite-reference.md` §9 (exit
  codes inherited from upstream).
- **Gate.** A pre-commit scenario: a file with a ZWSP exits 1.

### E7 — the workspace UI

- **Exists.** The queue with drop, paste and import (S7.1 partly), the
  Compare window with line and word marks (S7.4), the panel.
- **Build.** S7.1 "Clean" in the row's Actions menu and the footer line
  removed; S7.2 the source editor showing invisible characters as badges
  (`⟨ZWSP⟩`, like VS Code's renderControlCharacters); S7.3 the result
  streaming tokens; S7.5 the Inspector with "jump to position" over
  `UnicodeFinding.positions` (bytes — A §7.5); S7.6 the report with its
  three shelves and export to JSON/Markdown. The E1 seams of A §7.4:
  Compare's result becomes `clean(original)`, the panel shows a findings
  count.
- **Basis.** OV §6.1–6.2; `docs/architecture/compare.md`, `queue.md`.
- **Gate.** A 1 k-token stream keeps 30 FPS; a click on a finding
  scrolls to it.
- **Open.** Q7, Q-A1, Q-A3, Q-D2.

### E8 — models and engine UI (the rest)

- **Exists.** Models page with progress, recommendation and adoption;
  Engine page with profiles, `allow_remote`, write-only key.
- **Build.** A connection test (the first request this product sends —
  needs E2); a RAM/VRAM indicator while a model is loaded; the non-origin
  warning (needs a "suspected vendor" on a document, E7).

### E9 — licensing

- **Exists.** States and the rule that Layer A is never locked
  (`crates/wipemark-license/src/lib.rs:105`).
- **Build.** `heretic-license-activation-spec` integration (Ed25519 /
  PASETO v4.public), offline grace 24 h / 7 d, device fingerprint,
  keychain; the trial (Q6); the UI; and the manifest signature check E3
  waits for.

### E10 — packaging

- **Build.** Signed and notarized `.app`, Linux AppImage/deb (CUDA /
  Vulkan), a release lane on `wipemark-v*` tags, `cargo license` into
  `NOTICE`, the size budget. Gate: on a clean machine, download → open →
  first rewrite in under five minutes.

### E11 — images (phase 2) and E12 — pixels (phase 2b)

- **E11.** `wipemark-image` parsers per container (PNG, JPEG, WebP, TIFF,
  then HEIC/AVIF), C2PA/XMP/IPTC AI flags, pixels never re-encoded
  (sha256 of the decoded raster identical before and after). Depends on
  `wipemark-core` only.
- **E12.** Its own spec; heavy models; honest about the picture changing.

---

## 8. Risks beyond E1

| # | risk | what to do |
|---|---|---|
| R1 | `vendor/gpui-component` pins `a2f9c95`, which exists only on `glani/gpui-component` (branch `heretic/epic-4-line-decorations`); the build also leans on GitHub's redirect from `longbridge/gpui-component` to the renamed `longbridge/gpui-kit`. Deleting or renaming the personal fork breaks the checkout here and in heretic-amuse-merge. | **Investigated 2026-10-03.** The fork starts at upstream `b67d4ef8`, the last commit before `aba68aad` (Corner → Anchor), so it builds with zed `gpui@81b16f46`; it carries 7 commits (heretic-amuse-merge `docs/upstream.md`, "temporary fork → PR → delete the fork"). Wipemark uses P1 `LineDecorationProvider` (`compare.rs:95-97`, `:426-441`, `:788-796`; `result.rs:55`, `:297-305` — the Compare gutter `+`/`−` glyphs and line tints) and `selected_range()` (`result.rs:343`). Upstream (now `gpui-kit`, v0.7.0 on crates.io with `gpui-pre =0.3.7`) merged 4 of the 7 (#2278, #2279, #2410 renamed, #2411); `ThemeStyle` fields (#2322) closed; P3/P4 (#2412) open draft; **P1 was never proposed and upstream has no gutter-glyph API** (#3040 added range/text decorations and leaves gutter markers out). Options: (c) move the fork to `GigLaboCom` in lockstep with heretic-amuse-merge (~1 h, no code change); (b) pin upstream `b67d4ef8` + a committed patch applied by the pin script (2–4 h, no fork at all); (a) move to upstream 0.7.0 (3–6 days, loses the gutter glyphs, gpui jumps ~8 months) as its own epic. **Done 2026-10-03: (c).** The patch, ported onto upstream `main` (`2c5162f8`) as line-decoration collections in #3040's shape, is branch `heretic/line-decorations-on-upstream` (`3fba497`) and **draft upstream PR [longbridge/gpui-kit#3359](https://github.com/longbridge/gpui-kit/pull/3359)**. **(c) in detail:** The fork is `GigLaboCom/gpui-component` (fork of `longbridge/gpui-kit`), branch `heretic/epic-4-line-decorations` protected against force-push and deletion, the other five `heretic/*` branches copied; `.gitmodules` points there, CI syncs the URL. heretic-amuse-merge still points at the personal fork until it is switched in lockstep. Everything about the patch: `docs/sdd/line-decorations.md`. |
| R2 | GPUI and gpui-component APIs drift between revisions. | Pinned revisions; upgrades as their own arc (OV §12). |
| R3 | `llama-cpp-2` builds llama.cpp with cmake — CI time, CUDA on the runner. | First E2 live gate decides (OV §12). |
| R4 | A cold build compiles GPUI from source (tens of minutes). | CI keeps persistent cargo/target volumes (`.woodpecker/gate.yaml`); locally, library-only iterations use `-p wipemark-core`. |
| R5 | Nothing durable lives only on one machine again. | Push `feat/e0-e6-shell` after every landed document (§3.1). |

---

## 9. §0 Ground rules — reproduced once (identical in every E1 document)

> The block below is copied verbatim into every `E1-n` document. Change it
> here first, then in all seven.

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

## 10. References — the basis

**Specifications (Watchword, working copies in `ssd-docs/`).**
`heretic-unmark-overview-decomposition-2026-09-07` (OV): §0.1 principles,
§3 Layer A, §4 Layer B, §5 models, §6 app, §7 CLI, §10 epics and scopes,
§11 owner questions, §12 risks. `wipemark-core-layer-a-2026-09-21` (A):
§1 deviations from OV, §2 scope, §3 tables, §4 classifier and context,
§5 scrubber, NFKC, homoglyphs, report, §6 guards, §7 consumers, §8 gates,
§9 open questions. `wipemark-intake-drag-and-drop-2026-09-11` (DnD) and
`…-closed-2026-09-11`. `wipemark-q1-name-decision-2026-09-21`.

**Repository.** `CLAUDE.md` (the rules not visible in code);
`docs/architecture/skeleton.md` (what E0 built and the owner-question
table), `i18n.md`, `drag-and-drop.md`, `retention.md`, `compare.md`,
`engine-settings.md`, `who-rewrites.md`, `model-downloads.md`;
`docs/sdd/layer-b-rewrite-reference.md`; `fixtures/README.md`;
`scripts/check-dep-direction.sh`.

**Unicode (version 18.0.0 throughout).**
- UCD files: <https://www.unicode.org/Public/18.0.0/ucd/>; confusables:
  <https://www.unicode.org/Public/18.0.0/security/> (path changed in 17.0,
  A §3.1).
- UAX #44, *Unicode Character Database* — file formats, property values,
  code point labels: <https://www.unicode.org/reports/tr44/>.
- UAX #15, *Unicode Normalization Forms* — NFKC, canonical ordering,
  composition exclusions, `NormalizationTest.txt`:
  <https://www.unicode.org/reports/tr15/>.
- UTS #39, *Unicode Security Mechanisms* — confusables and skeletons:
  <https://www.unicode.org/reports/tr39/>.
- UTS #51, *Unicode Emoji* — emoji properties, ZWJ sequences, tag
  sequences (flags), presentation selectors:
  <https://www.unicode.org/reports/tr51/>.
- UAX #9, *Unicode Bidirectional Algorithm* — `Bidi_Class` R/AL, marks,
  isolates, embeddings, overrides: <https://www.unicode.org/reports/tr9/>.
- UAX #24, *Script Property*: <https://www.unicode.org/reports/tr24/>.
- The Unicode Standard, ch. 3.12 (Hangul syllable composition, used
  algorithmically by NFKC) and ch. 23 (special-purpose characters: ZWJ,
  ZWNJ, variation selectors, tags).
- Ideographic Variation Database (why every IVS after a Han ideograph is
  legal): <https://www.unicode.org/ivd/>.
- Unicode License v3 (the data files' licence, into `NOTICE`):
  <https://www.unicode.org/license.txt>.

**Security background.** Boucher & Anderson, *Trojan Source: Invisible
Vulnerabilities* (2021), CVE-2021-42574 — why bidi **overrides** are
always removed: <https://trojansource.codes/>.

**Upstream reference.** `guillaumemeyer/watermarks-remover` (MIT),
`service/scripts/text_unicode.py` — its list of what must *not* be
removed (A header); its Layer B, read out in
`docs/sdd/layer-b-rewrite-reference.md`.
<https://github.com/guillaumemeyer/watermarks-remover>.

**Protocols.** Model Context Protocol, revision 2025-06-18
(`PROTOCOL_VERSION`, `apps/wipemark-app/src/mcp/protocol.rs:46`):
<https://modelcontextprotocol.io/specification/2025-06-18>. JSON-RPC 2.0:
<https://www.jsonrpc.org/specification>. Project Fluent:
<https://projectfluent.org/>.
