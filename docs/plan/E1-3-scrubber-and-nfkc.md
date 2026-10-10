# E1-3 — The scrubber: inspect and clean as one pass, the report that names every position, and NFKC

|                 |                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Series          | `docs/plan/` — epic E1, Layer A; document 3 of 7 (`docs/plan/README.md` §3)                                                                                                                                                                                                                                                                                                                                                                   |
| Spec scopes     | S1.3, S1.5 (A §5.1–5.3, §5.5, §7.1)                                                                                                                                                                                                                                                                                                                                                                                                         |
| Depends on      | E1-2 committed on `feat/e0-e6-shell` (and, through it, E1-1)                                                                                                                                                                                                                                                                                                                                                                                 |
| Unblocks        | E1-4 (homoglyph hits plug into `scrub::collect_hits`), E1-6 (MCP and CLI embed `to_json`)                                                                                                                                                                                                                                                                                                                                                   |
| Files touched   | changed: `crates/wipemark-core/src/{lib,class,report}.rs`, and in `src/{tables,script}.rs` only E1-1's `dead_code` allowance (§4.3.1); new: `crates/wipemark-core/src/{scrub,nfkc,json}.rs`, `crates/wipemark-core/tests/{fixtures,corpus}.rs`, `crates/wipemark-core/tests/common/mod.rs`, 22 files under `fixtures/text/`; one paragraph in `fixtures/README.md`; one section in `docs/architecture/layer-a.md`; the E1-3 status row in `docs/plan/README.md`; the report `docs/plan/reports/E1-3-<date>.md`. Nothing under `apps/`, no other crate, not `CLAUDE.md` (E1-7). |
| Size            | ~4 days for one agent                                                                                                                                                                                                                                                                                                                                                                                                                       |

"A" below is the E1 spec, Watchword FILE `wipemark-core-layer-a-2026-09-21`
(Russian; everything this document relies on is translated here). "OV" is
the overview `heretic-unmark-overview-decomposition-2026-09-07`. "README"
is `docs/plan/README.md`, the plan of record. "Dn" is a decision in README
§4. Every `path:line` about existing code is at `497eafa`; E1-1 and E1-2
will have moved some of them — re-find by the quoted text, not the number.

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

## §1 Goal

After E1-2, `wipemark-core` can *classify*: `context::hits(text)` names every
finding-capable code point with its class, its confidence and whether its
context keeps it. Nothing yet *does* anything with a hit. This document
builds the half of Layer A that acts and reports:

1. **The public API** — `inspect`, `clean`, `Options` (the four knobs),
   `Cleaned` — exactly as README §3.3 fixes it.
2. **One decision over one pass.** `inspect` and `clean` call the same
   function to decide every hit and the same function to aggregate the
   decisions into report rows; `inspect` merely does not build the output
   string (A §5.2: an `inspect` that sees something other than what `clean`
   removes is a lie in the report). Context beats every knob.
3. **The report that names every position.** Rows aggregated by
   `(codepoint, class, confidence, acted)` (D6), split into `findings`
   (acted on) and `kept` (found and left), positions as **byte offsets into
   the source**, `removed`/`normalized` counters, `unicode_version` on both
   reports, `suspicious` per D4.
4. **NFKC** implemented from UAX #15 over E1-1's tables, gated by every line
   of the official `NormalizationTest.txt`; and the post-NFKC pass that
   removes what NFKC orphans (U+2139 INFORMATION SOURCE + U+FE0F VARIATION
   SELECTOR-16 becomes U+0069 + an orphaned U+FE0F) — repeated until it acts
   on nothing, which is what makes `clean` idempotent.
5. **The JSON form** of both reports (A §7.1), written by core itself with a
   std-only writer that always carries the third shelf (D9).
6. **`fixtures/text/`** — the byte-exact set: one file per class that is
   removed, and the must-survive set of A §4.2, each with its assertion.
7. **Idempotence** — `clean(clean(x)) == clean(x)` for every `Options` —
   proven on every fixture and on a deterministic generated corpus.

When this lands, `wipemark_core::clean(text, &Options::default())` returns
the cleaned text and a report whose every position is a byte of the
source, and `report.to_json()` is the string E1-6 will print from the CLI
and hand to MCP clients.

### 1.1 Where this document departs from A, and what settles each point

Read these before §4. Each is a place where A or the first version of
README was silent, wrong for Unicode 18.0.0, or impossible as written. Most
are now decisions in README §4 (cited by number) or part of the amended
§3.3 contract; the rest are this document's own and the closing report
repeats them.

| # | A / first README said | this document specifies | settled by |
|---|---|---|---|
| P1 | A §5.3: after NFKC, "one more main pass" | NFKC then the decision pass, **in rounds until the pass acts on nothing** (at most `MAX_ROUNDS = 8`; a `debug_assert!` fires on reaching the cap) | **D26.** One pass is not enough: `U+2139 U+FE0F U+0301` with `nfkc` — the first pass keeps U+FE0F (after an emoji base); NFKC gives `U+0069 U+FE0F U+0301` (U+FE0F is a starter, so it blocks U+0301 from composing with `i`); the second pass removes the orphan, leaving `U+0069 U+0301`, which is *not* NFKC, and a second `clean` turns it into `U+00ED`. Gate: `nfkc_rounds_reach_a_fixed_point`. |
| P2 | A §5.3: `NormalizationTest.txt` "parts 0–3" | every line of **all six parts** — Part0 46 lines, Part1 17,154, Part2 2,004, Part3 194, Part4 735 "Canonical closures (excluding Hangul)", Part5 38 "Chained primary composites"; 20,171 in all — **plus** the file's conformance clause 2 | **D23** for the six parts. Clause 2 ("every assigned code point not listed in Part 1 is its own NFKC", 293,187 code points in 18.0.0) is this document's addition: it catches a table with an entry it should not have. |
| P3 | A §5.1: `normalized` gets `(Nfkc, n)` "with the knob"; `n` undefined | `n` = the number of input code points NFKC did **not carry through unchanged**, summed over the rounds; the entry is present whenever `options.nfkc`, `0` included | **D27.** "Carried through" is exact and O(n) (§4.5.4); a `0` that is present says "ran, changed nothing", which differs from "not asked". |
| P4 | first README §3.3: `pub(crate) fn nfkc(text) -> String` only | `pub(crate) fn nfkc_counted(text) -> (String, u32)` is the production entry (`clean` calls it); `pub(crate) fn nfkc(text) -> String` is its thin wrapper, called by the tests | **README §3.3 as amended.** The count must come from the same run as the text. `nfkc` has no non-test caller, so it carries `#[cfg_attr(not(test), expect(dead_code, reason = …))]`. |
| P5 | the first wording of §0.4 and of README §3.2's `tests/` line (both since corrected to D24) placed tests by the data they read, not by what they call | a test that calls a `pub(crate)` function (`nfkc`, `nfkc_counted`, `collect_hits`, `run`, `merge_in_source_order`) is a unit test in its module, and may read data with `include_str!("../ucd/…")` or `include_str!("../../../fixtures/text/…")`; only tests that use the public API (`inspect`, `clean`, `to_json`, `name_of`, `class_of`) go to `tests/` | **D24** (and §0.4 as reproduced above). An integration test sees only the public API. |
| P6 | A §7.1's example (a *clean* report) carries `suspicious` and `stats`; the first `CleanReport` had neither | `CleanReport` gains `pub suspicious: bool` and `pub stats: TextStats`, both computed over the **source** text by the rules `inspect` uses (D4; `TextStats::of` once), so `CleanReport::to_json` is A §7.1's form exactly | **D28.** E1-6 also takes the CLI exit code (0/1 over the input) straight from `report.suspicious`. |
| P7 | D9: escape quote, backslash, control characters | additionally every character from U+007F upward is written as `\u` + four lowercase hex digits (a surrogate pair above U+FFFF) | **D29.** Every report is pure ASCII by construction; it never carries user text (ids, `U+XXXX`, UCD names, numbers), so this costs nothing — and "the JSON a client renders carries no invisible character" becomes a property of the writer, not of the data. |
| P8 | A §4.2: the standardized variant example U+2282 U+FE00 | fixtures use U+2229 INTERSECTION + U+FE00 and U+0030 DIGIT ZERO + U+FE00 | This document. U+2282 U+FE00 is **not** in `StandardizedVariants.txt` 18.0.0 (U+2229, U+222A, U+228A, U+228B and U+0030 with U+FE00 are). |
| P9 | README §3.3: `collect_hits(text: &str, options: &Options)` | the parameter stays, bound as `_options` | **README §3.3 as amended:** reserved for per-class overrides (Q-A1), unused under D3 (homoglyph detection runs regardless of the knobs). The leading underscore keeps the signature and silences `unused_variables` under `-D warnings`. |

## §2 Read first

1. **`docs/plan/README.md`** — §3.2 the module map (from line 159) and
   §3.3 the interfaces (from line 195): the fixed contract; §4 decisions
   (from line 339) — for this document D3, D4, D5, D6, D9, D17, D20, D23,
   D24, D25, D26, D27, D28, D29; §6 risks (from line 396). §9 is this document's §0. (Line
   numbers as of this writing; the plan is edited — find by heading.)
2. **`CLAUDE.md`** — "Delete the protection and watch it go red" (:94);
   "`wipemark-core` has zero dependencies" (:201); "The third shelf is never
   empty" (:212); "Every string a person reads comes from the catalogue;
   nothing a machine reads does" (:229); "Only applications localize"
   (:235); "`Rendering::PlainText` for anything that is not a window"
   (:239); "Tests must be able to fail" (:1046).
3. **A** (working copy `ssd-docs/wipemark-core-layer-a-2026-09-21.md`,
   gitignored; if it is missing, fetch it from Watchword by that key): §1
   rows "§3.1 NFKC" (line 52) and "§2 `CleanReport`" (54); §2 scope (61–81);
   §4.3 confidence, `suspicious`, aggregation and sort order (243–255); §5.1
   API (261–310); §5.2 actions and knobs (312–322); §5.3 order, second pass,
   idempotence, NFKC (324–360); §5.5 names, third shelf, `TextStats`
   (399–420); §7.1 the JSON form (445–474); §7.5 what must not be done with
   Layer A (522–529); §8 the gates table (533–577).
4. **The two documents before this one and their reports** —
   `docs/plan/E1-1-ucd-tables.md`, `docs/plan/E1-2-classifier.md`,
   `docs/plan/reports/E1-1-*.md`, `docs/plan/reports/E1-2-*.md`. Their
   "what the next document should know" sections override §3.2–3.4 below
   wherever they disagree; the code on the branch overrides both.
5. **Code** (at `497eafa`):
   - `crates/wipemark-core/src/class.rs` — `Action` :17-28 (`NormalizeToSpace`
     :23-24), `Confidence` :30-41, `UnicodeClass` :43-71, `ALL` :74-87,
     `as_str` :89-104, `default_action` :106-125, `max_confidence` :127-144,
     `requires_aggressive` :146-149, `UnicodeFinding` :152-164, tests :166-219.
   - `crates/wipemark-core/src/report.rs` — `not_established` :17-45
     (`ALL` :40-44), `TextStats` :47-58, `InspectReport` :60-73, `NormKind`
     :75-82, `CleanReport` :84-93, `FinalReport` :125-144, tests :146-167.
   - `crates/wipemark-core/src/lib.rs` :1-39 (crate docs, "Skeleton status"
     :21-25, `#![forbid(unsafe_code)]` :27, modules :29-32, re-exports :34-39).
   - `crates/wipemark-core/Cargo.toml` :10-12 (no dependencies, by rule).
   - `crates/wipemark-engine/src/fake.rs` :21-30 (`mix64`, the SplitMix64
     mixer the corpus generator copies) and :32-41 (FNV-1a).
   - `fixtures/README.md` :7-27 (layout; "byte-exact"; "every fixture has a
     claim").
   - `scripts/check-dep-direction.sh` :119-136 (core may have no
     `[dependencies]`, `[dev-dependencies]` or `[build-dependencies]`).
   - `crates/wipemark-i18n/src/tests.rs` :296-336 — an independent spelling
     of the removal set; **unchanged** by this document, and deliberately
     not wired to the classifier.
6. **The consumers E1-6 will wire** (read, do not touch):
   `apps/wipemark-app/src/mcp/protocol.rs` :157-170 (`Tool::refusal`),
   :259-272 (`call`), :371-400 (the refusal gate);
   `apps/wipemark-cli/src/main.rs` :535-553 (the "not implemented" exit 2).
7. **Unicode** — UAX #15 *Unicode Normalization Forms*; The Unicode Standard
   18.0, §3.11 (D107 Starter, D108 Reorderable pair, D109 Canonical Ordering
   Algorithm, D113 Full composition exclusion, D114 Primary composite, D115
   Blocked, D117 Canonical Composition Algorithm) and §3.12 (the Hangul
   constants); the header of `crates/wipemark-core/ucd/NormalizationTest.txt`
   (the two conformance clauses). URLs in §8.

## §3 What is true today

### 3.1 At `497eafa` (verified 2026-10-03)

| fact | evidence |
|---|---|
| `wipemark-core` has no dependencies of any kind, and the dependency script fails on one | `crates/wipemark-core/Cargo.toml:10-12`; `scripts/check-dep-direction.sh:119-136` reads `dependencies`, `dev-dependencies` and `build-dependencies` |
| `Action` is `{ Remove, NormalizeToSpace, Keep }`, no `as_str`; `NormalizeToSpace` is named nowhere else in the workspace | `class.rs:19-28`; `grep -rn NormalizeToSpace crates apps` → `class.rs:24` only |
| `Confidence` derives `Ord` in the order `LikelyFalsePositive < Informational < Probable < Confirmed`; no `as_str` | `class.rs:35-41` |
| `UnicodeClass::ALL` lists the 11 classes in the order rows are sorted by; ids are `zero-width`, `zwj`, `bidi-control`, `tag-character`, `variation-selector`, `soft-hyphen`, `exotic-space`, `noncharacter`, `private-use`, `default-ignorable`, `homoglyph` | `class.rs:75-87`, `:90-104` |
| defaults: `ExoticSpace` and `Homoglyph` → `Keep`, the other nine → `Remove`; ceilings: Confirmed (ZeroWidth, BidiControl, TagCharacter, Noncharacter, PrivateUse), Probable (ZeroWidthJoiner, VariationSelector, DefaultIgnorable, Homoglyph), Informational (SoftHyphen, ExoticSpace) | `class.rs:107-125`, `:131-144` |
| `UnicodeFinding { codepoint: char, class, count: u32, positions: Vec<usize>, confidence }`, `Eq` — **unchanged** by this document | `class.rs:157-164` |
| `InspectReport { findings, suspicious, stats, unicode_version }` — no `kept` | `report.rs:61-73` |
| `NormKind { SpaceToAscii, Nfkc }`, no `as_str` | `report.rs:76-82` |
| `CleanReport` derives `Default`; fields `{ removed, normalized, output_len }` only | `report.rs:88-93` |
| `FinalReport` holds `verifiable: CleanReport` and does **not** derive `Default`, so dropping `CleanReport`'s `Default` compiles | `report.rs:126-132` |
| the sentence "Byte-exact reversibility is not offered and never will be." is quoted elsewhere — keep it word for word in the new doc comment | `report.rs:86`; quoted by `docs/architecture/retention.md:127-130` and referred to by `CLAUDE.md:1028` |
| `not_established::ALL` = `vendor-detector-evasion`, `human-authorship`, `unknown-mark-schemes` (ids, in that order) | `report.rs:40-44` |
| `lib.rs` says "the types are here, the logic is not" | `lib.rs:21-25` |
| outside `wipemark-core`, nothing names `Action`, `Confidence`, `CleanReport`, `InspectReport`, `NormKind`, `UnicodeFinding` or `TextStats`; other crates use only `Vendor` (engine, models, pipeline, app) and `report::not_established` (i18n dev-dependency). The CLI's and the pipeline's own `Action` enums are unrelated types | `grep -rn "wipemark_core" crates apps --include=*.rs` |
| therefore every change in §4.1–4.2 breaks no crate outside core | same grep |
| `fixtures/` holds only `README.md`; there is no `fixtures/text/` and no `.gitattributes` at this commit (E1-1 creates the latter) | `ls fixtures`; `ls -a` |

### 3.2 What E1-1 delivered — the part of the contract this document uses

Copied from README §3.3 (`src/tables.rs`, `src/name.rs`):

```rust
// lib.rs re-exports this one as wipemark_core::UNICODE_VERSION (E1-1 adds the re-export)
pub const UNICODE_VERSION: &str;                       // read from the file headers, never typed: "18.0.0"

pub(crate) fn decomposition(c: char) -> Option<&'static [char]>; // full NFKD mapping, recursion expanded at build time; Hangul NOT in the table
pub(crate) fn ccc(c: char) -> u8;                      // Canonical_Combining_Class, 0 when absent
pub(crate) fn compose(first: char, second: char) -> Option<char>; // primary composites minus Full_Composition_Exclusion; Hangul NOT in the table

// name.rs — re-exported as wipemark_core::name_of (D7; E1-1 adds the re-export)
pub fn name_of(c: char) -> Option<std::borrow::Cow<'static, str>>;
// Some(Borrowed(UCD name)) for a finding-capable code point that has one;
// Some(Owned("<private-use-E000>" / "<noncharacter-FDD0>" / "<reserved-E0080>")) for one without;
// None for anything that can never be a finding.
```

and the committed `crates/wipemark-core/ucd/NormalizationTest.txt` and
`crates/wipemark-core/ucd/UnicodeData.txt` (both 18.0.0), plus
`.gitattributes` with the line `fixtures/text/** -text` (D25).

### 3.3 What E1-2 delivered

Copied from README §3.3 (`src/class.rs`, `src/context.rs`, `src/stats.rs`):

```rust
pub fn class_of(c: char) -> Option<UnicodeClass>;     // context-free membership; never Homoglyph

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
impl TextStats { pub fn of(text: &str) -> TextStats; }   // A §5.5
```

### 3.4 What this document relies on beyond the signatures — check each before writing code

| # | assumption | how to check | if it is false |
|---|---|---|---|
| A1 | `context::hits` returns hits in **strictly increasing** `at`, one per finding-capable code point, with `text[hit.at..].starts_with(hit.c)`; a U+FEFF at byte 0 is never a hit | read `context.rs`; the debug assertions in §4.4.4 check it on every test run | fix `context.rs`; record the deviation |
| A2 | `hit.confidence` is `hit.class.max_confidence()` unless `kept_by_context`, and then `LikelyFalsePositive`; nothing raises it | read `context.rs` | same |
| A3 | E1-2 leaves `replacement: None` | read `context.rs` | same |
| A4 | **a context decision never depends on a code point the same pass removes** (A §5.3: "context never leans on a character that can itself be removed — bases are letters and emoji, which are not findings"). This is what makes one pass idempotent on its own output; §5's idempotence tests are its gate | the corpus test of §5.4 | if a corpus failure traces to a context rule, fix the rule in `context.rs`, add the failing input to E1-2's tests by its name, and record it in the report as a deviation |
| A5 | **D20**: a paragraph is RTL only through a character with `Bidi_Class` R or AL that is a letter or mark (gc `L*`/`M*`), never through a `Bidi_Control` — U+200F RIGHT-TO-LEFT MARK is `Bidi_Class=R` and U+061C ARABIC LETTER MARK is `AL`, so counting them would let every RLM excuse itself. E1-2 implements it, gated by `a_stray_rlm_does_not_protect_itself` | `bidi-control.txt` (§4.7) depends on it: its U+200F is in an all-Latin paragraph and must be removed | fix `context.rs` as for A4 |
| A6 | `TextStats::of` returns ratios that are finite and in `0.0..=1.0` (A §7.1 shows `0.98`) | read `stats.rs` | the JSON writer debug-asserts finiteness (§4.6.4) |
| A7 | `tables::decomposition` is fully expanded and contains no Hangul syllable; `tables::compose` excludes every `Full_Composition_Exclusion` code point and knows nothing about Hangul | verified for 18.0.0: none of the 5,982 decomposition mappings contains a code point in U+AC00..U+D7A3; the conformance test of §5.3 catches the rest | `nfkc.rs` handles a Hangul syllable inside a mapping anyway (§4.5.2), so only the conformance test can be red |
| A8 | `name_of(c)` is `Some` for every finding-capable code point | the JSON tests assert no `"name":null` over the fixtures | report it against E1-1 |
| A9 | `.gitattributes` carries `fixtures/text/** -text` (D25; E1-1 creates the file) | `git check-attr text -- fixtures/text/zero-width.txt` prints `text: unset` | add the line exactly as D25 spells it and record it |
| A10 | `lib.rs` already re-exports `UNICODE_VERSION` and `name_of`: **E1-1 adds both** (`pub use tables::UNICODE_VERSION;`, `pub use name::name_of;`), so in this document they are present, not new | `grep -n "pub use" crates/wipemark-core/src/lib.rs` | add what is missing and record it; never two re-exports of one name |
| A11 | E1-1 scopes `#![cfg_attr(not(test), allow(dead_code))]` to `tables.rs` and `script.rs`, because nothing calls the lookups before this document. **E1-3 is the first caller** (`decomposition`, `ccc`, `compose` from `nfkc.rs`; and, through E1-2's `context::hits`, whatever the classifier uses) | `grep -n "allow(dead_code)" crates/wipemark-core/src/*.rs` | see §4.3.1 |

Start with `cargo test -p wipemark-core` green on the branch as you found it.

## §4 Deliverables

### 4.1 `class.rs` — `Action::Replace` and the two `as_str`

Replace `Action` (`class.rs:17-28`) with:

```rust
/// What the scrubber does with a finding.
///
/// [`crate::Options::action_for`] picks one per class from the class
/// default and the knobs. Context (A §4.2) can only turn a decision into
/// `Keep`, never the other way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Delete the code point. Counted in `CleanReport::removed`.
    Remove,
    /// Replace it with the equivalent the tables name: U+0020 SPACE for an
    /// exotic space, the letter of its word's script for a homoglyph.
    /// Counted in `CleanReport::normalized`, never in `removed`.
    Replace,
    /// Report it and change nothing.
    Keep,
}

impl Action {
    /// Stable identifier for `--json` and MCP. A format: never translated,
    /// renamed with the care of a config key.
    pub fn as_str(self) -> &'static str {
        match self {
            Action::Remove => "remove",
            Action::Replace => "replace",
            Action::Keep => "keep",
        }
    }
}
```

`NormalizeToSpace` folds into `Replace` (A §5.1). Its only use at
`497eafa` is its own declaration (`class.rs:24`); no class defaults to it.
After the change `grep -rn NormalizeToSpace crates apps` prints nothing.

Add to `impl Confidence` (a new `impl` block after the enum):

```rust
impl Confidence {
    /// Stable identifier for `--json` and MCP; the catalogue keys
    /// `confidence-<id>` (E1-6) hang off it. A format, never translated.
    pub fn as_str(self) -> &'static str {
        match self {
            Confidence::Confirmed => "confirmed",
            Confidence::Probable => "probable",
            Confidence::Informational => "informational",
            Confidence::LikelyFalsePositive => "likely-false-positive",
        }
    }
}
```

Both matches are exhaustive with no wildcard, so a new variant fails to
compile until it has an id. `default_action`, `max_confidence`,
`requires_aggressive` and `UnicodeFinding` do not change. Tests: §5.1.

### 4.2 `report.rs` — `InspectReport.kept`, the new `CleanReport`, `NormKind::Homoglyph`

`InspectReport` (`report.rs:60-73`) becomes, in this field order (README §3.3):

```rust
/// The result of looking without touching.
///
/// `findings` and `kept` are exactly the rows [`crate::clean`] reports for
/// the same text and options: one decision over one pass (A §5.2).
/// Positions are byte offsets into the inspected text.
#[derive(Debug, Clone, PartialEq)]
pub struct InspectReport {
    /// What `clean` would act on: remove, or replace with the equivalent
    /// the tables name.
    pub findings: Vec<UnicodeFinding>,
    /// What `clean` would find and leave — kept by context (orthography or
    /// presentation, at `LikelyFalsePositive`) or by a knob or the class
    /// default (at the class's own confidence).
    pub kept: Vec<UnicodeFinding>,
    /// True when some row in `findings` or `kept` is at least
    /// [`Confidence::Probable`] (D4). Deliberately not "has any row": soft
    /// hyphens and exotic spaces alone do not make a document suspicious,
    /// and neither does orthography kept by context.
    pub suspicious: bool,
    pub stats: TextStats,
    /// The UCD version the tables were generated from (keep the existing
    /// sentence of `report.rs:69-71`).
    pub unicode_version: &'static str,
}
```

`NormKind` (`report.rs:75-82`) gains a third variant and an id:

```rust
/// A normalisation the scrubber performed, as opposed to a removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormKind {
    /// An exotic space became U+0020 (`Options::normalize_spaces`).
    SpaceToAscii,
    /// NFKC was applied (`Options::nfkc`). The count is the number of code
    /// points NFKC did not carry through unchanged, over every round (D27).
    Nfkc,
    /// A homoglyph became the letter of its word's script
    /// (`Options::aggressive`). Detection is E1-4; the count is ready.
    Homoglyph,
}

impl NormKind {
    /// Stable identifier for `--json` and MCP. A format, never translated.
    pub fn as_str(self) -> &'static str {
        match self {
            NormKind::SpaceToAscii => "space-to-ascii",
            NormKind::Nfkc => "nfkc",
            NormKind::Homoglyph => "homoglyph",
        }
    }
}
```

`CleanReport` (`report.rs:84-93`) is replaced. `#[derive(Debug, Clone,
PartialEq)]` — **no `Default`** (A §1: only `clean` produces one). It gains
`suspicious` and `stats` (D28), placed right after `kept` as in
`InspectReport`, so the two reports read alike; the rest is README §3.3's
order. The doc comment's substance is part of the deliverable (the
sentence about the rounds after NFKC is required by A §5.3 and D26):

```rust
/// Layer A output: exactly what changed, and where.
///
/// `findings` are the code points `clean` acted on — removed, or replaced
/// with the equivalent the tables name — and `kept` the ones it found and
/// left, by context (orthography, presentation) or by a knob. Every entry
/// of `positions` is a **byte offset into the source text handed to
/// `clean`**, never into the output: the Inspector jumps to them in the
/// original. Rows are sorted by class (in [`UnicodeClass::ALL`] order),
/// then code point, then confidence from highest to lowest; `count` is
/// always `positions.len()`.
///
/// `removed` counts removals per class and `normalized` replacements per
/// kind. Without `Options::nfkc` they agree with the rows exactly: a
/// class's count in `removed` is the sum of the `count`s of its rows in
/// `findings`, and likewise `SpaceToAscii` for exotic spaces and
/// `Homoglyph` for homoglyphs. **With `nfkc` they can be larger, and this
/// is the only place where a count exceeds the positions behind it:** NFKC
/// can orphan a code point the first pass kept for its context (U+2139
/// U+FE0F becomes U+0069 U+FE0F), the passes that run after NFKC (in
/// rounds, until one acts on nothing) remove it, and the text they remove
/// it from is NFKC output — no byte offset there names a byte of the
/// source. So the passes after NFKC count and never position. For the same
/// reason a row in `kept` can name a code point that is not in the output
/// when `nfkc` was on.
///
/// `suspicious` and `stats` describe the **source**, by the rules
/// `inspect` uses: `suspicious` is D4 over `findings` and `kept` (so what
/// the passes after NFKC remove never makes a text suspicious), and
/// `stats` is `TextStats::of(source)`, computed once. A `clean` and an
/// `inspect` of the same text and options agree on both.
///
/// `removed` lists only classes with a non-zero count, in
/// [`UnicodeClass::ALL`] order. `normalized` lists `SpaceToAscii`, `Nfkc`,
/// `Homoglyph` in that order, each only when non-zero — except `Nfkc`,
/// which is present whenever `nfkc` was asked for, `0` included, so a
/// reader can tell "ran and changed nothing" from "not asked".
///
/// Byte-exact reversibility is not offered and never will be. What is
/// offered instead is this: counts and positions for every removal.
#[derive(Debug, Clone, PartialEq)]
pub struct CleanReport {
    pub findings: Vec<UnicodeFinding>,
    pub kept: Vec<UnicodeFinding>,
    /// D4 over `findings` and `kept` — the input, not the output (D28).
    pub suspicious: bool,
    /// `TextStats::of` the source text (D28).
    pub stats: TextStats,
    pub removed: Vec<(UnicodeClass, u32)>,
    pub normalized: Vec<(NormKind, u32)>,
    /// Length of the cleaned text, in bytes.
    pub output_len: usize,
    pub unicode_version: &'static str,
}
```

`TextStats`, `not_established`, `RewriteSummary`, `RiskLabel`, `FinalReport`
and the two existing tests stay as they are. Test added: §5.1.

### 4.3 `lib.rs` — the public API, `Options`, the module list

The file after this document (modules and re-exports E1-1/E1-2 already
added stay; the visibility of `script` is E1-1's choice and is kept):

```rust
//! `wipemark-core` — Layer A, the deterministic half of the product.
//!
//! (keep lines 3–19 of the current crate docs: "Everything here is
//! verifiable …" and "# Zero dependencies, by rule")
//!
//! # What is here
//!
//! [`inspect`] and [`clean`] — one decision over one pass of the text:
//! `inspect` reports what `clean` would do, `clean` does it and reports
//! what it did, positions always in bytes of the source. [`Options`] are
//! the four knobs, all off by default. Both reports serialise to the
//! JSON form the CLI and the MCP server print (`to_json`), third shelf
//! included. Homoglyph detection (E1-4) and the five guards (E1-5) are
//! not here yet.

#![forbid(unsafe_code)]

pub mod class;
pub mod guard;
pub mod report;
pub mod vendor;

mod context; // E1-2: the pre-passes and the context rules → hits
mod json; // E1-3: the A §7.1 form, std-only
mod name; // E1-1: name_of
mod nfkc; // E1-3: UAX #15 NFKC
mod script; // E1-1 (or `pub mod`, as E1-1 left it)
mod scrub; // E1-3: collect_hits, the decision, the pass, the rounds
mod stats; // E1-2: TextStats::of
mod tables; // E1-1: the generated tables

pub use class::{Action, Confidence, UnicodeClass, UnicodeFinding};
pub use guard::{Guard, GuardOutcome, RejectReason};
pub use name::name_of; // present since E1-1 — not added here
pub use report::{
    CleanReport, FinalReport, InspectReport, NormKind, RewriteSummary, RiskLabel, TextStats,
};
pub use tables::UNICODE_VERSION; // present since E1-1 — not added here
pub use vendor::Vendor;

/// The four knobs of Layer A — all off by default, which is what every
/// caller without a preference uses (`Options::default()`).
///
/// Context beats every knob: a code point A §4.2 keeps for its context is
/// kept whatever these say.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// Replace each homoglyph with the letter of its word's script.
    /// Detection runs either way (D3) — without this flag a homoglyph is
    /// reported in `kept` at `Probable`; with it, it is replaced.
    pub aggressive: bool,
    /// After the pass, apply NFKC (UAX #15) and pass again, until a pass
    /// changes nothing. Normalises everything, code included — Layer A
    /// cannot see a code block (A §2).
    pub nfkc: bool,
    /// Replace every exotic space (`General_Category=Zs` other than U+0020)
    /// with U+0020 SPACE.
    pub normalize_spaces: bool,
    /// Keep soft hyphens (U+00AD) instead of removing them.
    pub keep_soft_hyphen: bool,
}

impl Options {
    /// The class default, corrected by the knobs (A §5.2). This is the
    /// knob table; context is applied on top of it, never under it.
    pub fn action_for(&self, class: UnicodeClass) -> Action {
        match class {
            UnicodeClass::SoftHyphen if self.keep_soft_hyphen => Action::Keep,
            UnicodeClass::ExoticSpace if self.normalize_spaces => Action::Replace,
            UnicodeClass::Homoglyph if self.aggressive => Action::Replace,
            other => other.default_action(),
        }
    }
}

/// What `clean` returns: the cleaned text and exactly what changed.
#[derive(Debug, Clone, PartialEq)]
pub struct Cleaned {
    pub text: String,
    pub report: CleanReport,
}

/// Look without touching. Every finding-capable code point of `text`,
/// decided exactly as [`clean`] decides it, aggregated into rows whose
/// positions are byte offsets into `text`. `options.nfkc` is not read:
/// nothing NFKC changes can be positioned in the source.
pub fn inspect(text: &str, options: &Options) -> InspectReport {
    scrub::inspect(text, options)
}

/// Remove and replace what `options` ask for, keep what context
/// protects, and report every change. Idempotent for every `options`:
/// `clean(&clean(x, o).text, o).text == clean(x, o).text`.
pub fn clean(text: &str, options: &Options) -> Cleaned {
    scrub::clean(text, options)
}
```

The knob table `action_for` encodes (every other class is its
`default_action()` whatever the knobs):

| class | default | knob | with the knob |
|---|---|---|---|
| `SoftHyphen` | `Remove` | `keep_soft_hyphen` | `Keep` |
| `ExoticSpace` | `Keep` | `normalize_spaces` | `Replace` (with U+0020) |
| `Homoglyph` | `Keep` | `aggressive` | `Replace` (with `hit.replacement`) |
| any | — | `nfkc` | no effect on the action |

`Options` and `Cleaned` live in `lib.rs` (README §3.2: "src/lib.rs — the
public API"); the bodies live in `scrub.rs`. The re-exports of
`UNICODE_VERSION` and `name_of` are E1-1's and are already there (A10);
what this document adds to `lib.rs` is the crate docs, `mod json; mod nfkc;
mod scrub;`, `Options`, `Cleaned`, `inspect` and `clean`. Tests: §5.1.

#### 4.3.1 E1-1's `dead_code` allowance — remove what this document now calls

E1-1 put `#![cfg_attr(not(test), allow(dead_code))]` at the top of
`tables.rs` and `script.rs`, because before this document nothing outside
tests called the lookups (A11). E1-3 is their first non-test caller:
`nfkc.rs` calls `decomposition`, `ccc` and `compose`, and `clean`/`inspect`
reach every predicate E1-2's `context::hits` uses. The allowance is a
module-wide inner attribute, so:

1. delete it from `tables.rs`, then from `script.rs`, one at a time, and
   run `cargo clippy -p wipemark-core --all-targets -- -D warnings`;
2. if a module is then clean, the deletion stays;
3. if `dead_code` names items, those items still have no caller (the
   confusable lookups wait for E1-4; `is_uppercase_letter`/
   `is_lowercase_letter` may wait for E1-4 or E1-5): either put the
   module-wide allowance back unchanged, or narrow it to exactly the items
   `clippy` named (the same `#[cfg_attr(not(test), allow(dead_code))]` on
   each) — and list those items in the report under "what the next
   document should know". E1-7 removes whatever remains.

These are the only edits this document makes to `tables.rs` and
`script.rs`.

### 4.4 `scrub.rs` — the hits, the decision, the pass, the rows, the rounds

New file. Everything in it is `pub(crate)` or private.

#### 4.4.1 The seam E1-4 extends

```rust
/// Every hit in `text`, in source order: the context hits (E1-2) merged
/// with the homoglyph hits (E1-4). The one place a detector is wired in.
pub(crate) fn collect_hits(text: &str, _options: &Options) -> Vec<Hit> {
    // E1-4 replaces `Vec::new()` with `homoglyph::hits(text)` — that one
    // expression is the whole of its change here.
    merge_in_source_order(context::hits(text), Vec::new())
}

/// Two hit lists, each in source order, merged into one in source order.
/// A linear merge by `at`; the two detectors never claim the same code
/// point (a homoglyph source is a letter, a context hit never is), and a
/// debug assertion says so.
fn merge_in_source_order(a: Vec<Hit>, b: Vec<Hit>) -> Vec<Hit>;
```

`_options` is unused (P9): README §3.3 keeps the parameter, reserved for
per-class overrides (Q-A1); under D3 nothing reads it. `merge_in_source_order` is called today with an
empty `b`, so it is live code, and is unit-tested now with synthetic hits
(§5.2 `merging_keeps_source_order`) so that E1-4 changes one expression
and adds no machinery. After the merge: `debug_assert!` that `at` is
strictly increasing.

#### 4.4.2 The decision — one function for `inspect` and `clean`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decision {
    Keep,
    Remove,
    Replace(char),
}

/// What happens to one hit. The only place a decision is made.
fn decide(hit: &Hit, options: &Options) -> Decision {
    // A §5.2: context beats every knob — a ZWJ inside a family emoji stays
    // whatever the Options say.
    if hit.kept_by_context {
        return Decision::Keep;
    }
    match options.action_for(hit.class) {
        Action::Keep => Decision::Keep,
        Action::Remove => Decision::Remove,
        // A letter is never deleted for want of a replacement: a homoglyph
        // hit without one is kept (and reported in `kept`).
        Action::Replace => replacement(hit).map_or(Decision::Keep, Decision::Replace),
    }
}

/// The equivalent the tables name (A §5.1).
fn replacement(hit: &Hit) -> Option<char> {
    match hit.class {
        UnicodeClass::ExoticSpace => Some(' '),
        UnicodeClass::Homoglyph => hit.replacement,
        // `action_for` never answers `Replace` for these; listed one by one
        // so that a new class is a compile error here, not a silent `None`.
        UnicodeClass::ZeroWidth
        | UnicodeClass::ZeroWidthJoiner
        | UnicodeClass::BidiControl
        | UnicodeClass::TagCharacter
        | UnicodeClass::VariationSelector
        | UnicodeClass::SoftHyphen
        | UnicodeClass::Noncharacter
        | UnicodeClass::PrivateUse
        | UnicodeClass::DefaultIgnorable => None,
    }
}
```

A decision is **acted** when it is `Remove` or `Replace`; an acted hit's
row goes to `findings`, a kept one's to `kept`.

#### 4.4.3 Rows — aggregation (D6) and sort order (A §4.3)

Rows are keyed by `(acted, class, codepoint, confidence)` and collected in
a `std::collections::BTreeMap` whose key orders exactly as the report must:

```rust
type RowKey = (bool, usize, char, std::cmp::Reverse<Confidence>);
//            acted  class position in UnicodeClass::ALL
//                          codepoint ascending (char: Ord = scalar order)
//                                 confidence descending
```

- the class position is `UnicodeClass::ALL.iter().position(|c| *c == class)`
  (`ALL` lists every variant once — `class.rs` already tests that), so there
  is no second ordering table to drift;
- the value is the `Vec<usize>` of positions; hits arrive in source order,
  so each vector is ascending without sorting;
- iterating the map yields every kept row (`acted == false`) and then every
  acted row, each group already in A §4.3's order — split on `acted`, and
  build `UnicodeFinding { codepoint, class: UnicodeClass::ALL[pos], count:
  u32::try_from(positions.len()).unwrap_or(u32::MAX), positions, confidence }`;
- `confidence` is `hit.confidence` (E1-2's value: the class ceiling, or
  `LikelyFalsePositive` for kept-by-context). A knob-kept or default-kept
  row keeps the class confidence — a soft hyphen kept by
  `keep_soft_hyphen` is `Informational`, an exotic space kept by default is
  `Informational`, a homoglyph kept without `aggressive` is `Probable`.

Why `acted` is in the key (D6): two hits of one code point, one class and
one confidence can only differ in their decision when one is a homoglyph
with a replacement and one without (the knobs are global, so nothing else
splits them in one run) — and an acted row and a kept row must never
merge, or a row would claim both. A U+200D kept inside an emoji and a
U+200D removed between two letters are two rows already by confidence
(`LikelyFalsePositive` vs `Probable`). (D6's first basis — "a knob-kept
soft hyphen vs a removed one" — cannot occur in one run, because
`keep_soft_hyphen` applies to every soft hyphen alike; the homoglyph without
a replacement is the real case, and `acted_and_kept_rows_never_merge`
pins it.)

#### 4.4.4 The pass

```rust
struct Pass {
    findings: Vec<UnicodeFinding>,
    kept: Vec<UnicodeFinding>,
    removed: [u32; 11],      // indexed by position in UnicodeClass::ALL
    normalized: [u32; 3],    // indexed SpaceToAscii, Nfkc, Homoglyph (Nfkc is never counted here)
    acted: u32,              // Remove + Replace decisions
    output: Option<String>,  // Some only when asked to build
}

/// Decide every hit, aggregate the rows, count, and — when `build` —
/// produce the output text. `inspect` passes `build: false`; nothing else
/// differs between the two callers.
fn run(text: &str, hits: &[Hit], options: &Options, build: bool) -> Pass;
```

Steps, in one walk over `hits`:

1. `let mut out = build.then(|| String::with_capacity(text.len()));` and
   `let mut copied = 0` (byte index up to which `text` has been copied).
2. For each `hit`: `debug_assert!(hit.at >= copied &&
   text[hit.at..].starts_with(hit.c))` (A1); `let d = decide(hit, options)`;
   push `hit.at` into the row for `(d != Keep, class position, hit.c,
   Reverse(hit.confidence))`; count:
   - `Remove` → `removed[class position] += 1`, `acted += 1`;
   - `Replace(_)` with `ExoticSpace` → `normalized[SpaceToAscii] += 1`;
     with `Homoglyph` → `normalized[Homoglyph] += 1`; `acted += 1`;
   - `Keep` → nothing.

   All counters `saturating_add`. If building: `out.push_str(&text[copied..
   hit.at])`, then `Keep` → `out.push(hit.c)`, `Remove` → nothing,
   `Replace(r)` → `out.push(r)`; `copied = hit.at + hit.c.len_utf8()`.
3. After the loop, if building: `out.push_str(&text[copied..])`.
4. Split the row map into `findings` and `kept` (§4.4.3).

Runs between hits are copied as slices, so a clean text costs one copy.
The cost is O(n) for the walk plus O(h log r) for the map (h hits, r
rows — r is small).

#### 4.4.5 `suspicious` (D4)

```rust
/// D4: some row, acted on or kept, is at least Probable. Kept-by-context
/// rows are LikelyFalsePositive and knob-kept exotic spaces Informational,
/// so neither counts; a homoglyph kept without `aggressive` (Probable)
/// does — that is the case a user must be told about.
fn is_suspicious(findings: &[UnicodeFinding], kept: &[UnicodeFinding]) -> bool {
    findings.iter().chain(kept).any(|row| row.confidence >= Confidence::Probable)
}
```

`inspect` stores it in `InspectReport::suspicious` and `clean` in
`CleanReport::suspicious` (D28), both over the rows of the first pass —
the source. It is private to `scrub.rs`; there is no second definition,
and the JSON writer writes the field rather than recomputing it.

#### 4.4.6 `inspect`

```rust
pub(crate) fn inspect(text: &str, options: &Options) -> InspectReport {
    let hits = collect_hits(text, options);
    let pass = run(text, &hits, options, false);
    InspectReport {
        suspicious: is_suspicious(&pass.findings, &pass.kept),
        findings: pass.findings,
        kept: pass.kept,
        stats: TextStats::of(text), // once per call (A §5.5)
        unicode_version: UNICODE_VERSION,
    }
}
```

`options.nfkc` is not read (it changes no decision, and nothing NFKC does
can be positioned in the source).

#### 4.4.7 `clean`, and the rounds after NFKC (D26)

```rust
/// How many times `clean` may run NFKC and the pass again (D26). Two
/// rounds are the most any input in the corpus needs (§5.4); the cap turns
/// a future context rule that breaks convergence into a failed debug
/// assertion, never a hung MCP thread.
const MAX_ROUNDS: usize = 8;

pub(crate) fn clean(text: &str, options: &Options) -> Cleaned;
```

1. `let first = run(text, &collect_hits(text, options), options, true);` —
   **the rows of the report come from this pass and from no other.**
   `removed = first.removed`, `normalized = first.normalized`, `out =
   first.output` (always `Some` when `build`).
2. If `options.nfkc`: `let mut nfkc_changed = 0u32; let mut converged =
   false;` then for each round in `0..MAX_ROUNDS`:
   1. `let (normal, changed) = nfkc::nfkc_counted(&out);`
      `nfkc_changed = nfkc_changed.saturating_add(changed);`
   2. `let again = run(&normal, &collect_hits(&normal, options), options, true);`
   3. add `again.removed` into `removed` element-wise, and
      `again.normalized[SpaceToAscii]`, `again.normalized[Homoglyph]` into
      `normalized` — **counts only; `again.findings` and `again.kept` are
      dropped**, because their positions are bytes of NFKC output, not of
      the source (A §5.3);
   4. `out = again.output`;
   5. if `again.acted == 0` → `converged = true; break`.

   After the loop: `debug_assert!(converged, "NFKC and the pass did not
   converge in {MAX_ROUNDS} rounds")` and `normalized[Nfkc] = nfkc_changed`
   (D27 — present even when 0).
3. Build `CleanReport { suspicious: is_suspicious(&first.findings,
   &first.kept), stats: TextStats::of(text), findings: first.findings,
   kept: first.kept, removed: <the non-zero entries in UnicodeClass::ALL
   order>, normalized: <SpaceToAscii if > 0, Nfkc if options.nfkc,
   Homoglyph if > 0, in that order>, output_len: out.len(),
   unicode_version: UNICODE_VERSION }` and return `Cleaned { text: out,
   report }`. `suspicious` and `stats` are over the **source** `text`, each
   computed once (D28) — never over `out`.

#### 4.4.8 Why `clean` is idempotent

`clean(x, o)` ends on a text `y` that is (a) a fixpoint of the pass — the
last pass acted on nothing — and, when `o.nfkc`, (b) NFKC output, which is
a fixpoint of NFKC because NFKC is idempotent by definition. `clean(y, o)`
therefore runs a pass that acts on nothing (a), and with `nfkc` an NFKC
that changes nothing (b) followed by the same inactive pass. So
`clean(clean(x, o), o) == clean(x, o)`, and the second report has empty
`findings`, empty `removed` and every `normalized` count 0.

(a) without `nfkc` rests on A4: one pass is a fixpoint of itself because
no decision leans on a code point the pass removes — the bases of every
context rule are letters, marks and emoji, which are never findings. (b)
is what the round loop (D26) adds to A §5.3's single second pass: a removal
after NFKC can leave text that is no longer NFKC, so NFKC runs again. The
loop ends because after the first NFKC no step lengthens the text (removal
creates no compatibility character; U+0020 and E1-4's letters are NFKC
stable; composition only shortens) and every continuing round acts on at
least one code point. The two idempotence tests of §5.4 are the gate; the
U+2139 U+FE0F U+0301 case is pinned on its own by
`nfkc_rounds_reach_a_fixed_point` (§5.2).

### 4.5 `nfkc.rs` — UAX #15 NFKC over E1-1's tables

New file. Normalization Form KC is the compatibility decomposition of the
text followed by canonical composition (UAX #15; Unicode §3.11 D117).

#### 4.5.1 API

```rust
/// NFKC of `text`, and the number of its code points that NFKC did not
/// carry through unchanged (§4.5.4, D27). The production entry (README
/// §3.3); `clean` calls this.
pub(crate) fn nfkc_counted(text: &str) -> (String, u32);

/// NFKC of `text` — the contract's name (README §3.3). The conformance
/// tests call it; `clean` needs the count and calls `nfkc_counted`.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the contracted entry point; production uses nfkc_counted")
)]
pub(crate) fn nfkc(text: &str) -> String {
    nfkc_counted(text).0
}
```

An all-ASCII `text` may be returned at once as `(text.to_owned(), 0)`:
no ASCII code point has a decomposition, a non-zero combining class or a
composition with another ASCII code point.

#### 4.5.2 The algorithm

Work on a buffer of units:

```rust
struct Unit {
    c: char,
    ccc: u8,      // tables::ccc(c), looked up once
    src: usize,   // index (in code points) of the input code point it came from
    at: usize,    // its index in the decomposed buffer, before reordering
    moved: bool,  // canonical reordering changed its place
    mixed: bool,  // a composite that absorbed a unit of another source
}
```

Hangul constants (Unicode §3.12): `S_BASE = 0xAC00`, `L_BASE = 0x1100`,
`V_BASE = 0x1161`, `T_BASE = 0x11A7`, `L_COUNT = 19`, `V_COUNT = 21`,
`T_COUNT = 28`, `N_COUNT = V_COUNT * T_COUNT = 588`, `S_COUNT = L_COUNT *
N_COUNT = 11172`.

1. **Full compatibility decomposition.** For each input code point `c` at
   index `i`:
   - if `S_BASE <= c < S_BASE + S_COUNT` (a Hangul syllable): `s = c -
     S_BASE`; emit `L_BASE + s / N_COUNT`, `V_BASE + (s % N_COUNT) /
     T_COUNT`, and `T_BASE + s % T_COUNT` **only when** `s % T_COUNT != 0`;
   - else if `tables::decomposition(c)` is `Some(m)`: emit every `d` of `m`
     (already fully expanded at build time), passing each `d` through the
     Hangul rule above (none needs it in 18.0.0 — A7 — but the cost is one
     comparison);
   - else emit `c`.

   Every emitted unit gets `src = i`, `at = <its index in the buffer>`,
   `moved = mixed = false`.
2. **Canonical ordering** (D108/D109). For every maximal run of units with
   `ccc != 0`, stable-sort the run by `ccc` (`slice::sort_by_key`, which is
   stable and linear on an already ordered run). Afterwards mark `moved =
   true` on every unit of the run whose index differs from its `at`. Units
   with equal `ccc` keep their order — that is what "stable" buys and what
   Part 2 of the test file checks.
3. **Canonical composition** (D115, D117). Walk the reordered units,
   pushing into `out: Vec<Unit>`, tracking `starter: Option<usize>` (index
   in `out` of the last unit with `ccc == 0`) and `last: Option<u8>` (the
   `ccc` of the last unit pushed after that starter, `None` when nothing
   was):
   - if there is a starter and the unit is **not blocked** — `last` is
     `None`, or `0 < last < unit.ccc` (a unit is blocked when something
     between it and the starter has `ccc == 0` or `ccc >= unit.ccc`; in
     canonical order only the immediately preceding unit needs checking,
     D115) — and `compose(out[starter].c, unit.c)` is `Some(p)`: set
     `out[starter].c = p`, set `out[starter].mixed |= unit.src !=
     out[starter].src`, drop the unit, and leave `last` unchanged;
   - otherwise push the unit; if its `ccc == 0` it becomes the starter
     (`starter = Some(out.len() - 1)`, `last = None`), else `last =
     Some(unit.ccc)`.

   `compose(a, b)`: Hangul first —
   - **LV**: `L_BASE <= a < L_BASE + L_COUNT` and `V_BASE <= b < V_BASE +
     V_COUNT` → `S_BASE + ((a - L_BASE) * V_COUNT + (b - V_BASE)) * T_COUNT`;
   - **LV + T**: `S_BASE <= a < S_BASE + S_COUNT`, `(a - S_BASE) % T_COUNT
     == 0`, and `T_BASE < b < T_BASE + T_COUNT` (strictly greater: U+11A7
     itself is not a trailing consonant) → `a + (b - T_BASE)`;
   - otherwise `tables::compose(a, b)` (primary composites only; every
     `Full_Composition_Exclusion` code point is already out of the table).

   A starter that does not compose with the preceding starter becomes the
   next starter, so "chained" composites (Part 5) compose naturally.
4. **Output** — collect `out`'s `c` into the `String`.

Complexity: O(n) for decomposition and composition; reordering is O(k)
for a run of k marks already in order and O(k log k) otherwise. One
`Vec<Unit>` and the output `String` are allocated.

#### 4.5.3 Verified before this document was written

The algorithm above was implemented in a 150-line reference (Python, over
the 18.0.0 `UnicodeData.txt` and `DerivedNormalizationProps.txt`) and run
against `NormalizationTest-18.0.0.txt`: **all 20,171 lines, every column,
`c4 == NFKC(c1..c5)`, 0 failures**; conformance clause 2 over the 293,187
assigned code points not listed in Part 1, 0 failures. The counts in
§4.5.4 and the expectations in §4.7 come from the same reference.

#### 4.5.4 The count — "not carried through" (D27)

An input code point `i` is **carried through** when, in `out`, exactly
one unit has `src == i`, and that unit is not `mixed`, not `moved`, and
its `c` equals input code point `i`. The count is `(number of input code
points) - (number carried through)`, as `u32` (saturating). Compute it in
one walk over `out` with two vectors indexed by `src` (units per source,
and "a faithful unit exists"); no unit from one source can outnumber the
longest decomposition (18 code points, U+FDFA), so a `u8` per source is
enough.

The count is 0 exactly when the output equals the input. Pinned values
(all from the reference implementation):

| input | NFKC | count | why |
|---|---|---|---|
| U+FB01 LATIN SMALL LIGATURE FI | `f i` | 1 | one code point became two |
| U+2139 INFORMATION SOURCE | `i` | 1 | compatibility mapping |
| U+00E9 (precomposed) | U+00E9 | 0 | decomposed and recomposed to itself |
| `e` U+0301 | U+00E9 | 2 | two code points merged |
| `a` U+0300 U+0323 | U+1EA1 U+0300 | 3 | reordered, and `a` + U+0323 merged |
| `x` U+0301 U+0316 | `x` U+0316 U+0301 | 2 | reordered only |
| U+212B ANGSTROM SIGN | U+00C5 | 1 | canonical singleton |
| U+AC00 | U+AC00 | 0 | Hangul round trip |
| U+1100 U+1161 | U+AC00 | 2 | jamo composed |
| U+00A0 NO-BREAK SPACE | U+0020 | 1 | |
| U+1680 OGHAM SPACE MARK | U+1680 | 0 | no decomposition |
| U+01C4 LATIN CAPITAL LETTER DZ WITH CARON | `D` U+017D | 1 | |
| U+01D6 | U+01D6 | 0 | two-level canonical round trip |
| `ab` | `ab` | 0 | |

### 4.6 `json.rs` — the A §7.1 form, written by core (D9)

New file. Two inherent methods, defined here (an inherent `impl` may live
in any module of the crate):

```rust
impl InspectReport {
    /// The A §7.1 form: one line of ASCII JSON, keys in a fixed order, the
    /// third shelf always present. Machines read it; nothing in it is
    /// translated.
    pub fn to_json(&self) -> String;
}

impl CleanReport {
    /// As `InspectReport::to_json`, with `removed`, `normalized` and
    /// `output_len` between `kept` and `stats` — A §7.1's form exactly
    /// (D28).
    pub fn to_json(&self) -> String;
}
```

#### 4.6.1 The shapes — exact key order, no whitespace anywhere

`InspectReport`:

```
{"unicode_version":<string>,"suspicious":<bool>,"findings":[<row>,…],"kept":[<row>,…],
 "stats":{"chars":<int>,"words":<int>,"latin_ratio":<num>,"cyrillic_ratio":<num>,"cjk_ratio":<num>,"code_blocks":<int>,"urls":<int>},
 "not_established":["vendor-detector-evasion","human-authorship","unknown-mark-schemes"]}
```

`CleanReport`:

```
{"unicode_version":<string>,"suspicious":<bool>,"findings":[<row>,…],"kept":[<row>,…],
 "removed":{<class id>:<int>,…},"normalized":{<norm id>:<int>,…},"output_len":<int>,
 "stats":{"chars":<int>,"words":<int>,"latin_ratio":<num>,"cyrillic_ratio":<num>,"cjk_ratio":<num>,"code_blocks":<int>,"urls":<int>},
 "not_established":["vendor-detector-evasion","human-authorship","unknown-mark-schemes"]}
```

(the line breaks above are for reading only — the output is one line with
no space, tab or newline outside string values and no trailing newline).

`<row>`:

```
{"codepoint":"U+XXXX","name":<string or null>,"class":<class id>,"confidence":<confidence id>,"action":<action id>,"count":<int>,"positions":[<int>,…]}
```

- `codepoint`: `U+` and the scalar value in **uppercase** hex, at least
  four digits (`format!("U+{code:04X}")` with `let code = u32::from(c)`):
  `U+00A0`, `U+200B`, `U+E0001`, `U+10FFFD`.
- `name`: `name_of(codepoint)` — the UCD name or E1-1's label
  (`<private-use-E000>`); `null` only if `name_of` returns `None`, which
  A8 says never happens for a row.
- `class`: `UnicodeClass::as_str`; `confidence`: `Confidence::as_str`.
- `action`: for a row of `kept`, `"keep"`; for a row of `findings`, the
  action `clean` took, which is a function of the class alone with the
  four knobs of today — `"replace"` for `ExoticSpace` and `Homoglyph`,
  `"remove"` for the other nine. A private exhaustive `fn
  acted_action(class: UnicodeClass) -> Action` (no wildcard) in this file;
  the test `the_json_action_is_the_action_clean_took` (§5.5) proves it
  agrees with `Options::action_for` for every class and every `Options`,
  and is the place that goes red the day per-class overrides (Q-A1) make
  the action depend on more than the class.
- `count`: `UnicodeFinding::count`; `positions`: every position, all of
  them (A §5.1: "all, not the first ten" — the UI truncates).
- `removed`: the `Vec` in the order it holds (built in `UnicodeClass::ALL`
  order by `clean`), keys `UnicodeClass::as_str`; `{}` when empty.
- `normalized`: likewise, keys `NormKind::as_str`; `{}` when empty.
- `suspicious`: the field, in both forms — the writer never recomputes it
  (D28 put it on `CleanReport`).
- `not_established`: the ids of `report::not_established::ALL`, in its
  order, read from that constant (never re-typed); always present, always
  last (D9).
- `stats`: the field, in both forms — in the clean form after
  `output_len`, which is where A §7.1 puts it (D28).

#### 4.6.2 Numbers and booleans

`usize`/`u32` with `{}`. The three ratios (`f32`) with `{:?}`: Rust's
`Debug` for floats prints the shortest representation that reads back to
the same `f32`, with a `.0` on integral values (`0.0`, `1.0`, `0.98`) and
an exponent only far from 1 (`1e-7`) — every one of them a valid JSON
number. A non-finite ratio would not be; `debug_assert!(ratio.is_finite())`
and write `0.0` for it in release (A6). `true` / `false` literally.

#### 4.6.3 Strings — the std-only escaper

`fn push_string(out: &mut String, value: &str)` writes `"`, then for each
`char`:

- `"` → `\"`; `\` → `\\`;
- below U+0020, or U+007F and above → `\u` followed by four **lowercase**
  hex digits per UTF-16 code unit (`c.encode_utf16(&mut [0u16; 2])`), so a
  code point above U+FFFF becomes a surrogate pair — U+1F525 is
  `\ud83d\udd25`;
- anything else (printable ASCII) as is;

then `"`. No short escapes (`\n`, `\t`) are used — one rule, and the
output is ASCII by construction (D29).

Writing to a `String` cannot fail: use `push`/`push_str` (numbers via
`to_string()`), or `write!` with its `fmt::Result` discarded and a comment
saying why. Format arguments are inlined (`{code:04X}`): the workspace
denies `uninlined_format_args` under `-D warnings`.

#### 4.6.4 The two exact forms the tests pin

A hand-built `InspectReport` (in `json.rs`'s tests, so it does not depend
on E1-2's classifier):

```rust
InspectReport {
    findings: vec![UnicodeFinding { codepoint: '\u{200B}', class: UnicodeClass::ZeroWidth, count: 2, positions: vec![5, 40], confidence: Confidence::Confirmed }],
    kept: vec![UnicodeFinding { codepoint: '\u{200D}', class: UnicodeClass::ZeroWidthJoiner, count: 1, positions: vec![12], confidence: Confidence::LikelyFalsePositive }],
    suspicious: true,
    stats: TextStats { chars: 42, words: 7, latin_ratio: 0.98, cyrillic_ratio: 0.0, cjk_ratio: 0.0, code_blocks: 0, urls: 1 },
    unicode_version: "18.0.0",
}
```

`to_json()` is exactly:

```json
{"unicode_version":"18.0.0","suspicious":true,"findings":[{"codepoint":"U+200B","name":"ZERO WIDTH SPACE","class":"zero-width","confidence":"confirmed","action":"remove","count":2,"positions":[5,40]}],"kept":[{"codepoint":"U+200D","name":"ZERO WIDTH JOINER","class":"zwj","confidence":"likely-false-positive","action":"keep","count":1,"positions":[12]}],"stats":{"chars":42,"words":7,"latin_ratio":0.98,"cyrillic_ratio":0.0,"cjk_ratio":0.0,"code_blocks":0,"urls":1},"not_established":["vendor-detector-evasion","human-authorship","unknown-mark-schemes"]}
```

A hand-built `CleanReport`:

```rust
CleanReport {
    findings: vec![
        UnicodeFinding { codepoint: '\u{E0001}', class: UnicodeClass::TagCharacter, count: 1, positions: vec![10], confidence: Confidence::Confirmed },
        UnicodeFinding { codepoint: '\u{A0}', class: UnicodeClass::ExoticSpace, count: 1, positions: vec![3], confidence: Confidence::Informational },
    ],
    kept: vec![],
    suspicious: true,
    stats: TextStats { chars: 18, words: 3, latin_ratio: 1.0, cyrillic_ratio: 0.0, cjk_ratio: 0.0, code_blocks: 0, urls: 0 },
    removed: vec![(UnicodeClass::TagCharacter, 1)],
    normalized: vec![(NormKind::SpaceToAscii, 1), (NormKind::Nfkc, 0)],
    output_len: 17,
    unicode_version: "18.0.0",
}
```

`to_json()` is exactly:

```json
{"unicode_version":"18.0.0","suspicious":true,"findings":[{"codepoint":"U+E0001","name":"LANGUAGE TAG","class":"tag-character","confidence":"confirmed","action":"remove","count":1,"positions":[10]},{"codepoint":"U+00A0","name":"NO-BREAK SPACE","class":"exotic-space","confidence":"informational","action":"replace","count":1,"positions":[3]}],"kept":[],"removed":{"tag-character":1},"normalized":{"space-to-ascii":1,"nfkc":0},"output_len":17,"stats":{"chars":18,"words":3,"latin_ratio":1.0,"cyrillic_ratio":0.0,"cjk_ratio":0.0,"code_blocks":0,"urls":0},"not_established":["vendor-detector-evasion","human-authorship","unknown-mark-schemes"]}
```

(`unicode_version` is a literal in these two hand-built values on purpose:
the writer writes the field, and the tests must not change on a Unicode
bump. `every_report_names_the_unicode_version` covers the real value. The
`stats` here are made up for the same reason — the writer writes fields,
and E1-2's `TextStats::of` is tested by E1-2.)

E1-6 parses these strings with `serde_json` in the applications (MCP
`structuredContent`, CLI `--json`); core's own tests are parse-free.

### 4.7 `fixtures/text/` — the byte-exact set

Twenty-two files: one per class that the scrubber acts on or can act on
(every class but `Homoglyph`, which E1-4 adds), named `<class id>.txt`, and
twelve `survive-*.txt` files carrying A §4.2's must-survive set. If
`fixtures/text/` already holds a file of the same name from E1-2 with the
same bytes, keep it; with different bytes, stop and report — never
overwrite another document's fixture.

#### 4.7.1 Creating them

From the repository root (the snippet contains escapes only — no
invisible character is ever typed):

```sh
python3 - <<'EOF'
from pathlib import Path
FIXTURES = {
    "zero-width.txt": "Plain\u200btext\u200cwith\u2060four\ufeffmarks.\n",
    "zwj.txt": "Two\u200dwords, one\u200djoiner.\n",
    "bidi-control.txt": "Left\u200eto\u200fright \u202eevil\u202c and \u2066isolated\u2069 text.\n",
    "tag-character.txt": "Loose\U000e0001\U000e0065\U000e006etags\U000e007f here.\n",
    "variation-selector.txt": "a\ufe0f b\ufe00 c\U000e0100 d\ufe0e e\u180b.\n",
    "soft-hyphen.txt": "co\xadop\xader\xadate\n",
    "exotic-space.txt": "no\xa0break thin\u2009space wide\u3000space narrow\u202fno-break ogham\u1680mark\n",
    "noncharacter.txt": "non\ufdd0char\ufffeacters\U0001ffff.\n",
    "private-use.txt": "pri\ue000vate\uf8ffuse\U000f0000area\U0010fffd.\n",
    "default-ignorable.txt": "graph\u034feme fn\u2061call old\u206aformat \ufff9anno\ufffatated\ufffb filler\u3164x mvs\u180ex reserved\U000e0080x\n",
    "survive-emoji-presentation.txt": "\u2696\ufe0f \u2764\ufe0f \u2139\ufe0f 1\ufe0f\u20e3 \u2122\ufe0f\n",
    "survive-emoji-zwj.txt": "\U0001f469\u200d\U0001f469\u200d\U0001f467\u200d\U0001f466 \U0001f3f3\ufe0f\u200d\U0001f308 \U0001f46e\u200d\u2640\ufe0f \u2764\ufe0f\u200d\U0001f525\n",
    "survive-flag-tags.txt": "\U0001f3f4\U000e0067\U000e0062\U000e0073\U000e0063\U000e0074\U000e007f\n",
    "survive-variation-sequences.txt": "\u8fbb\U000e0100 \u2229\ufe00 0\ufe00\n",
    "survive-persian-zwnj.txt": "\u0645\u06cc\u200c\u0631\u0648\u0645\n",
    "survive-devanagari-zwj.txt": "\u0915\u094d\u200d\u0937\n",
    "survive-rtl-bidi.txt": "\u0645\u0631\u062d\u0628\u0627 \u200eWipemark\u200e \u2068ABC\u2069 \u0645\n",
    "survive-mongolian-selectors.txt": "\u1820\u180b \u1820\u180e\u1820\n",
    "survive-khmer-inherent-vowels.txt": "\u1780\u17b4 \u1780\u17b5\n",
    "survive-hangul-fillers.txt": "\u1100\u1160 \ud55c\u3164\n",
    "survive-script-format-controls.txt": "\u0600\u0661\u0662\u0663 \U00013000\U00013430\U00013001\n",
    "survive-leading-bom.txt": "\ufeffHi\n",
}
root = Path("fixtures/text")
root.mkdir(parents=True, exist_ok=True)
for name, text in FIXTURES.items():
    data = text.encode("utf-8")
    path = root / name
    if path.exists() and path.read_bytes() != data:
        raise SystemExit(f"{name} already exists with other bytes: report it, do not overwrite")
    path.write_bytes(data)
EOF
```

(`\ud55c` there is U+D55C HANGUL SYLLABLE HAN, not a surrogate.) Then
verify every byte — this must print `OK` 22 times:

```sh
shasum -a 256 -c <<'EOF'
f41295f07727837e4d53eb88cea98c1b961449a36d2ea4ea9d75dcaae14dfdfa  fixtures/text/zero-width.txt
4b1d0643061c7b00ff67f62df11c3e24a5ed04b064127fd25c70f7c18aceb05a  fixtures/text/zwj.txt
f880ef211d0eaf2c360a3e54c874a95f47019d2c4f8d4f1dfa3b5a5a18c168d3  fixtures/text/bidi-control.txt
698d9ef9bfaa92f6b1840ff139569c6d31da98f5ca7ff5bb16835fb1fa43009a  fixtures/text/tag-character.txt
6ef67651ae9d88eb1e1de9639a6bf53c499b5dcae7ac07c32e0f952e53e66282  fixtures/text/variation-selector.txt
d8514d04cb2f1ff55e93bd6bacb9dacec30e11b484696467fab2a512e975ad5a  fixtures/text/soft-hyphen.txt
a6d545d289a5e018a6ef59f3565bd5fde2d6bef74dba3eeb1c950553cd509e07  fixtures/text/exotic-space.txt
a69e8dfeb06ded12e527dcb5c6c33a61eb8fd506bb543bc8793fc8ee0ce71c23  fixtures/text/noncharacter.txt
ba33733b89a0b9a9b784676bae50c936e68a5c92a6ba2199c84978399dfd8aaf  fixtures/text/private-use.txt
0214ead9e6bae97af840fe51bffe434a8fe54b24ac0b403053b150af53bee7ed  fixtures/text/default-ignorable.txt
4fafae91ccc8fe6f145f61f4db380bd2eeeb239ee95a57be2105ada7fb1a7314  fixtures/text/survive-emoji-presentation.txt
6567c9f3d827d8efc365e12a56ea1cf4e721e49b60fcc736099ff456f0dafd68  fixtures/text/survive-emoji-zwj.txt
3389ab0f972510865cff21db0c810d8fa223e0453b8901dde809323c1dfb0ed0  fixtures/text/survive-flag-tags.txt
66e1010fe8b6101167a183b0376b81349b80a6006af84f031cf3d84728ea5ae9  fixtures/text/survive-variation-sequences.txt
ffed784c6ffe796132824372803cddc1dfc8e1c4395bb4a310033aaf31156c8a  fixtures/text/survive-persian-zwnj.txt
cda26f51d2ab133cf9fe0796360bea1d63dcbf27de7f597cff5d0472a8b2662d  fixtures/text/survive-devanagari-zwj.txt
c6040bea2e7fab71ab8da1a75173f0d15d3b301dfc6a8a03744a89a3d8ca7275  fixtures/text/survive-rtl-bidi.txt
ccc058a46d9f9c55a46ee050a3dbd48a7515d459b022fa052faa9a6d954eca37  fixtures/text/survive-mongolian-selectors.txt
319b503c3251dd304b18baf61136600fdc60694fbfad541b0646f642d6ab9cf8  fixtures/text/survive-khmer-inherent-vowels.txt
bcec6f9f9a63a3fd812143c06b394cd6857f800c4e4fc56f34f4a53b5f49d6f3  fixtures/text/survive-hangul-fillers.txt
1a0b5ddd045996ac208a94ec89438716d0fc7d97ec1d7829ed3abb466b3c9fbb  fixtures/text/survive-script-format-controls.txt
70de4a4eb838889438579cf2892c0c9791d6bd68c6401724b2482431f06d79af  fixtures/text/survive-leading-bom.txt
EOF
git check-attr text -- fixtures/text/zero-width.txt   # → fixtures/text/zero-width.txt: text: unset
```

#### 4.7.2 The removal set — claims under `Options::default()`

Content is given as the Rust literal the test compares against. Every row
in this table is in `findings`; `kept` is empty; `removed` is the one entry
`{<class>: <sum of counts>}`. Positions are bytes of the file.

| file (bytes) | content | cleaned (bytes) | `findings` rows: code point · confidence · positions | suspicious |
|---|---|---|---|---|
| `zero-width.txt` (36) | `"Plain\u{200B}text\u{200C}with\u{2060}four\u{FEFF}marks.\n"` | `"Plaintextwithfourmarks.\n"` (24) | U+200B · confirmed · [5]; U+200C · confirmed · [12]; U+2060 · confirmed · [19]; U+FEFF · confirmed · [26] | yes |
| `zwj.txt` (27) | `"Two\u{200D}words, one\u{200D}joiner.\n"` | `"Twowords, onejoiner.\n"` (21) | U+200D · probable · [3, 16] | yes |
| `bidi-control.txt` (54) | `"Left\u{200E}to\u{200F}right \u{202E}evil\u{202C} and \u{2066}isolated\u{2069} text.\n"` | `"Lefttoright evil and isolated text.\n"` (36) | U+200E · confirmed · [4]; U+200F · [9]; U+202C · [25]; U+202E · [18]; U+2066 · [33]; U+2069 · [44] (all confirmed) | yes |
| `tag-character.txt` (32) | `"Loose\u{E0001}\u{E0065}\u{E006E}tags\u{E007F} here.\n"` | `"Loosetags here.\n"` (16) | U+E0001 · [5]; U+E0065 · [9]; U+E006E · [13]; U+E007F · [21] (all confirmed) | yes |
| `variation-selector.txt` (27) | `"a\u{FE0F} b\u{FE00} c\u{E0100} d\u{FE0E} e\u{180B}.\n"` | `"a b c d e.\n"` (11) | U+180B · [22]; U+FE00 · [6]; U+FE0E · [17]; U+FE0F · [1]; U+E0100 · [11] (all probable) | yes |
| `soft-hyphen.txt` (16) | `"co\u{AD}op\u{AD}er\u{AD}ate\n"` | `"cooperate\n"` (10) | U+00AD · informational · [2, 6, 10] | **no** |
| `noncharacter.txt` (25) | `"non\u{FDD0}char\u{FFFE}acters\u{1FFFF}.\n"` | `"noncharacters.\n"` (15) | U+FDD0 · [3]; U+FFFE · [10]; U+1FFFF · [19] (all confirmed) | yes |
| `private-use.txt` (30) | `"pri\u{E000}vate\u{F8FF}use\u{F0000}area\u{10FFFD}.\n"` | `"privateusearea.\n"` (16) | U+E000 · [3]; U+F8FF · [10]; U+F0000 · [16]; U+10FFFD · [24] (all confirmed) | yes |
| `default-ignorable.txt` (86) | `"graph\u{34F}eme fn\u{2061}call old\u{206A}format \u{FFF9}anno\u{FFFA}tated\u{FFFB} filler\u{3164}x mvs\u{180E}x reserved\u{E0080}x\n"` | `"grapheme fncall oldformat annotated fillerx mvsx reservedx\n"` (59) | U+034F · [5]; U+180E · [67]; U+2061 · [13]; U+206A · [24]; U+3164 · [59]; U+FFF9 · [34]; U+FFFA · [41]; U+FFFB · [49]; U+E0080 · [80] (all probable) | yes |

Rows are listed in report order (class, then code point). What each one
shows: U+200C between two Latin letters is a carrier, not orthography
(Latin is not a joining script); a U+200D between letters is removed at
`Probable`; in a paragraph with no RTL **letter** every bidi control goes
(D20 — U+200F must not count as its own evidence); tags without an emoji
base go; a VS16/VS15 after a non-emoji, a VS1 after a letter with no
standardized variant, an IVS after a non-Han letter and a Mongolian FVS
after a Latin letter all go; U+3164 after a Latin letter and U+180E after a
Latin letter are not in their context.

Further claims on this set:
- under `Options { nfkc: true, ..Default::default() }` the cleaned text and
  rows are identical and `normalized == [(Nfkc, 0)]` (every cleaned text is
  ASCII);
- `soft-hyphen.txt` under `keep_soft_hyphen`: output byte-identical,
  `findings` empty, `kept` = U+00AD · informational · [2, 6, 10],
  `removed` empty, not suspicious.

`exotic-space.txt` (67 bytes) — `"no\u{A0}break thin\u{2009}space
wide\u{3000}space narrow\u{202F}no-break ogham\u{1680}mark\n"`:

| options | output (bytes) | rows | counters |
|---|---|---|---|
| default | byte-identical (67) | `kept`: U+00A0 · [2]; U+1680 · [59]; U+2009 · [14]; U+202F · [42]; U+3000 · [27] (all informational) | none; not suspicious |
| `normalize_spaces` | `"no break thin space wide space narrow no-break ogham mark\n"` (58) | the same five rows in `findings` (action `replace`) | `normalized == [(SpaceToAscii, 5)]` |
| `nfkc` | `"no break thin space wide space narrow no-break ogham\u{1680}mark\n"` (60) | the five rows in `kept`, as default | `normalized == [(Nfkc, 4)]` — U+1680 has no decomposition |

#### 4.7.3 The survival set — claims

Under **every** `Options` with `nfkc: false` (eight combinations): the
output is byte-identical to the input, `findings` is empty, `removed` and
`normalized` are empty, `suspicious` is false, and `kept` is exactly the
rows below (every one at `likely-false-positive`).

| file (bytes) | content | `kept` rows: code point · class · positions |
|---|---|---|
| `survive-emoji-presentation.txt` (36) | `"\u{2696}\u{FE0F} \u{2764}\u{FE0F} \u{2139}\u{FE0F} 1\u{FE0F}\u{20E3} \u{2122}\u{FE0F}\n"` | U+FE0F · variation-selector · [3, 10, 17, 22, 32] |
| `survive-emoji-zwj.txt` (69) | `"\u{1F469}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466} \u{1F3F3}\u{FE0F}\u{200D}\u{1F308} \u{1F46E}\u{200D}\u{2640}\u{FE0F} \u{2764}\u{FE0F}\u{200D}\u{1F525}\n"` | U+200D · zwj · [4, 11, 18, 33, 45, 61]; U+FE0F · variation-selector · [30, 51, 58] |
| `survive-flag-tags.txt` (29) | `"\u{1F3F4}\u{E0067}\u{E0062}\u{E0073}\u{E0063}\u{E0074}\u{E007F}\n"` | tag-character: U+E0062 · [8]; U+E0063 · [16]; U+E0067 · [4]; U+E0073 · [12]; U+E0074 · [20]; U+E007F · [24] |
| `survive-variation-sequences.txt` (20) | `"\u{8FBB}\u{E0100} \u{2229}\u{FE00} 0\u{FE00}\n"` | variation-selector: U+FE00 · [11, 16]; U+E0100 · [3] |
| `survive-persian-zwnj.txt` (14) | `"\u{645}\u{6CC}\u{200C}\u{631}\u{648}\u{645}\n"` | U+200C · zero-width · [4] |
| `survive-devanagari-zwj.txt` (13) | `"\u{915}\u{94D}\u{200D}\u{937}\n"` | U+200D · zwj · [6] |
| `survive-rtl-bidi.txt` (39) | `"\u{645}\u{631}\u{62D}\u{628}\u{627} \u{200E}Wipemark\u{200E} \u{2068}ABC\u{2069} \u{645}\n"` | bidi-control: U+200E · [11, 22]; U+2068 · [26]; U+2069 · [32] |
| `survive-mongolian-selectors.txt` (17) | `"\u{1820}\u{180B} \u{1820}\u{180E}\u{1820}\n"` | U+180B · variation-selector · [3]; U+180E · default-ignorable · [10] |
| `survive-khmer-inherent-vowels.txt` (14) | `"\u{1780}\u{17B4} \u{1780}\u{17B5}\n"` | default-ignorable: U+17B4 · [3]; U+17B5 · [10] |
| `survive-hangul-fillers.txt` (14) | `"\u{1100}\u{1160} \u{D55C}\u{3164}\n"` | default-ignorable: U+1160 · [3]; U+3164 · [10] |
| `survive-script-format-controls.txt` (22) | `"\u{600}\u{661}\u{662}\u{663} \u{13000}\u{13430}\u{13001}\n"` | none — U+0600 ARABIC NUMBER SIGN and U+13430 EGYPTIAN HIEROGLYPH VERTICAL JOINER are `Cf` but not `Default_Ignorable`: never findings (A §4.1) |
| `survive-leading-bom.txt` (6) | `"\u{FEFF}Hi\n"` | none — a U+FEFF at byte 0 is a BOM, not a finding, and stays in the output |

What they cover, by A §4.2 row: VS16 after `Emoji=Yes` bases (U+2696
SCALES, U+2764 HEAVY BLACK HEART, U+2139, the keycap base `1`, U+2122
TRADE MARK SIGN); ZWJ inside emoji ZWJ sequences, with U+FE0F in the middle
of a sequence not shifting the base (U+2764 U+FE0F U+200D U+1F525); a
complete tag sequence (Scotland); an IVS after a Han ideograph and two
standardized variants (U+2229 INTERSECTION and U+0030 DIGIT ZERO with VS1 —
P8); ZWNJ and ZWJ inside a joining script (Persian, Devanagari — the ZWJ
follows U+094D DEVANAGARI SIGN VIRAMA, a mark of the same script); bidi
marks and isolates in a paragraph with Arabic letters; Mongolian FVS and
MVS after Mongolian letters; Khmer inherent vowels; Hangul fillers after
Hangul jamo and syllables.

Under `nfkc: true` (the other eight combinations) every survival fixture is
still byte-identical **except** two (verified with the reference NFKC):

| file | output with `nfkc` (bytes) | counters |
|---|---|---|
| `survive-emoji-presentation.txt` | `"\u{2696}\u{FE0F} \u{2764}\u{FE0F} i 1\u{FE0F}\u{20E3} TM\n"` (27) — U+2139 became `i` and U+2122 became `TM`, and the two VS16 they orphaned were removed by the pass after NFKC | `removed == [(VariationSelector, 2)]`, `normalized == [(Nfkc, 2)]`; `kept` unchanged (still five U+FE0F positions — the rows describe the source) |
| `survive-hangul-fillers.txt` | `"\u{1100}\u{1160} \u{D55C}\u{1160}\n"` (14) — U+3164 HANGUL FILLER has the compatibility mapping U+1160, which after a Hangul syllable is still in its context | `removed` empty, `normalized == [(Nfkc, 1)]` |

#### 4.7.4 `fixtures/README.md`

Under "Layout", replace the `text/` description's last sentence with one
paragraph: files named `<class id>.txt` are cleaned (one per class,
`homoglyph.txt` arrives with E1-4); files named `survive-*.txt` must come
out byte-identical; every file's claim is in
`crates/wipemark-core/tests/fixtures.rs`, and `every_fixture_is_asserted`
fails for a file that has none. Keep the two rules as they are.

### 4.8 `docs/architecture/layer-a.md` — section "Scrubber, report and NFKC"

If E1-1/E1-2 created the file, add the section after theirs; if not,
create the file with a one-line title ("Layer A — the deterministic
scrubber") and this section. Written for someone working on this code next
year; it must say:

- **What.** `inspect` and `clean` are one decision (`scrub::decide`) over
  one pass (`scrub::run`); the order of a run (hits → decision → rows and
  counters → output; then, with `nfkc`, NFKC and the pass in rounds); the
  knob table and "context beats every knob"; the two lists (`findings`
  acted on, `kept` left) and the aggregation key with `acted`; the sort
  order; `suspicious` (D4) and why soft hyphens, exotic spaces and
  orthography do not make a text suspicious; positions are bytes of the
  source and the one place a count exceeds its positions.
- **Where.** `lib.rs` (API, `Options`), `scrub.rs`, `nfkc.rs`, `json.rs`,
  `report.rs`, `class.rs`; `tests/fixtures.rs`, `tests/corpus.rs`;
  `fixtures/text/`.
- **Why the rounds.** The U+2139 U+FE0F U+0301 counterexample to a single
  second pass, and the fixpoint argument of §4.4.8.
- **NFKC.** UAX #15 over E1-1's tables, the Hangul arithmetic, the blocked
  rule, the conformance gate (all six parts of the 18.0.0 test file — D23 — and
  clause 2), and the definition of the `nfkc` count with three of the
  examples of §4.5.4.
- **The JSON form.** Both shapes with one exact example, the escaping rule
  and why the output is ASCII, that `not_established` is written by the
  writer and cannot be forgotten by a surface, and that `suspicious` and
  `stats` on a clean report describe the source (D28) — which is why E1-6
  can take the CLI's exit code from `report.suspicious`.
- **How to extend.** E1-4's one-line seam in `collect_hits`; a new class
  (the exhaustive matches that fail to compile, the fixture the test
  demands); a Unicode bump (fetch, rebuild, the conformance test and the
  fixtures decide); per-class overrides (Q-A1) and the
  `the_json_action_is_the_action_clean_took` test they will turn red.
- **What it does not do.** Positions into output; NFKC aware of code; a
  byte-exact way back.

## §5 Tests

Every protection below is painted red by its mutation before it counts
(§0.4): apply the mutation locally, run the named test, see it fail for
the stated reason, restore, and record the row in the report's mutation
table. A row without a mutation is a regression test, not a protection.

**Where each test lives (D24).** A test that calls a crate-private
function — `nfkc`, `nfkc_counted`, `collect_hits` (`pub(crate)`), or the
private `run`, `merge_in_source_order`, `decide`, `is_suspicious`, the JSON
escaper — is a unit test in that module, and reads data with `include_str!("../ucd/…")` or
`include_str!("../../../fixtures/text/…")` when it needs to (paths
relative to `src/`). A test that uses only the public API — `inspect`,
`clean`, `to_json`, `name_of`, `class::class_of`, `UNICODE_VERSION` — goes
to `crates/wipemark-core/tests/`. §5.1–5.3 and §5.5 are unit tests; §5.4
is the integration suite. Shared helpers for the integration tests live in
`crates/wipemark-core/tests/common/mod.rs`: `fn every_options() -> Vec<Options>`
(the 16 combinations, bit 0 `aggressive`, bit 1 `nfkc`, bit 2
`normalize_spaces`, bit 3 `keep_soft_hyphen`) and `fn hex(s: &str) ->
String` (`U+XXXX U+XXXX …`) — every assertion message prints text through
`hex`, never raw.

### 5.1 `class.rs`, `report.rs`, `lib.rs` (unit)

| test | file | input → expected | mutation (must go red) |
|---|---|---|---|
| `action_ids_are_exact` | `class.rs` | `Remove`/`Replace`/`Keep` → `"remove"`/`"replace"`/`"keep"`, each asserted literally | rename one id |
| `confidence_ids_are_exact` | `class.rs` | the four ids of §4.1, literally; and the four are distinct | rename `likely-false-positive` to `likely_false_positive` |
| `norm_kind_ids_are_exact` | `report.rs` | `"space-to-ascii"`, `"nfkc"`, `"homoglyph"` | rename one |
| `options_default_is_all_off` | `lib.rs` | `Options::default() == Options { aggressive: false, nfkc: false, normalize_spaces: false, keep_soft_hyphen: false }`, and for every class `Options::default().action_for(c) == c.default_action()` | replace the derive with a hand-written `Default` that sets `normalize_spaces: true` |
| `the_knob_table_is_exact` | `lib.rs` | for each of the 16 `Options` and each of the 11 classes, `action_for` equals an independently written literal table (the table in §4.3) | make `aggressive` also turn `ExoticSpace` into `Replace`; separately, ignore `keep_soft_hyphen` |

### 5.2 `scrub.rs` (unit — inline strings through the public functions, and synthetic hits through `run`)

| test | input → expected | mutation |
|---|---|---|
| `positions_are_byte_offsets_into_the_source` | `"\u{416}\u{20AC}\u{10348}\u{200B}x\u{200B}"` (U+0416 CYRILLIC CAPITAL LETTER ZHE, 2 bytes; U+20AC EURO SIGN, 3; U+10348 GOTHIC LETTER HWAIR, 4) → `inspect` and `clean` both report U+200B · zero-width · [9, 13]; cleaned `"\u{416}\u{20AC}\u{10348}x"` | record the char index instead of `hit.at` (gives [3, 5]) — A §8 "count chars" |
| `soft_hyphens_alone_are_not_suspicious` | `"co\u{AD}op"` → `findings` = U+00AD · informational · [2]; `suspicious == false` for `inspect` and for `clean`'s report (D28) | compare with `Confidence::Informational` instead of `Probable` in `is_suspicious` (A §8) |
| `a_single_zero_width_space_is` | `"a\u{200B}b"` → `suspicious == true` for `inspect` and for `clean`'s report | compute `is_suspicious` over `kept` only. (A §8's "compare with Informational" cannot paint this one: U+200B is `Confirmed`, above both thresholds — record that in the report) |
| `orthography_alone_is_not_suspicious` | `"\u{1F469}\u{200D}\u{1F467}"` → `findings` empty, `kept` = U+200D · zwj · LFP · [4], `suspicious == false` for both reports | `is_suspicious` = "any row at all" |
| `a_kept_homoglyph_makes_the_text_suspicious` (D4) | `run("p\u{430}y", &[Hit { at: 1, c: '\u{430}', class: Homoglyph, confidence: Probable, kept_by_context: false, replacement: Some('a') }], &Options::default(), true)` → `kept` = U+0430 · homoglyph · probable · [1]; `is_suspicious(..) == true`; output unchanged | compute `is_suspicious` over `findings` only |
| `a_kept_and_a_removed_joiner_are_two_rows` | `"\u{2764}\u{FE0F}\u{200D}\u{1F525} a\u{200D}b"` → `findings` = [U+200D · zwj · probable · [15]]; `kept` = [U+200D · zwj · LFP · [6], U+FE0F · variation-selector · LFP · [3]] (zwj before variation-selector: `ALL` order); cleaned `"\u{2764}\u{FE0F}\u{200D}\u{1F525} ab"` (16 bytes) | aggregate by `(codepoint, class)` only — one U+200D row with two positions |
| `acted_and_kept_rows_never_merge` (D6) | `run("\u{430}\u{430}", &[two Homoglyph hits for U+0430 at 0 and 2, both Probable, the first with replacement Some('a'), the second None], &Options { aggressive: true, ..Default::default() }, true)` → `findings` = U+0430 · [0]; `kept` = U+0430 · [2]; output `"a\u{430}"`; `normalized[Homoglyph] == 1` | drop `acted` from the row key — one row of count 2 |
| `context_beats_every_knob` | for every one of the 16 `Options`: `clean` of `"\u{1F469}\u{200D}\u{1F467}"`, `"\u{2764}\u{FE0F}"`, `"\u{645}\u{200C}\u{631}"`, `"\u{1F3F4}\u{E0067}\u{E0062}\u{E0073}\u{E0063}\u{E0074}\u{E007F}"` is byte-identical to the input and its `kept` rows are the same as under `Options::default()` | delete the `kept_by_context` early return in `decide` |
| `keep_soft_hyphen_keeps_soft_hyphens` | `"co\u{AD}op"`: default → `"coop"`, `findings` = U+00AD · [2], `removed == [(SoftHyphen, 1)]`; with the knob → unchanged, `kept` = U+00AD · informational · [2], `findings` and `removed` empty | `action_for` ignores `keep_soft_hyphen` |
| `normalize_spaces_replaces_exotic_spaces_with_a_space` | `"a\u{A0}b\u{3000}c"`: default → unchanged, `kept` = U+00A0 · [1], U+3000 · [4] (informational), `normalized` empty; with the knob → `"a b c"`, the same two rows in `findings`, `normalized == [(SpaceToAscii, 2)]`, `removed` empty | make `Replace` push nothing (a removal) — the text loses its spaces and `normalized` is wrong |
| `aggressive_replaces_a_homoglyph_with_its_letter` (D3) | `run("p\u{430}y", <the hit of the D4 test>, &Options { aggressive: true, ..Default::default() }, true)` → output `"pay"`, `findings` = U+0430 · homoglyph · probable · [1], `normalized[Homoglyph] == 1`, `removed` all zero | push `hit.c` instead of the replacement |
| `nfkc_is_off_unless_asked` | `"\u{FB01}"`: default → unchanged, `normalized` empty; `nfkc` → `"fi"`, `normalized == [(Nfkc, 1)]` | run NFKC whatever `options.nfkc` says |
| `nfkc_reports_that_it_ran_even_when_it_changed_nothing` | `"abc"` with `nfkc` → `normalized == [(Nfkc, 0)]`; without → `normalized` empty | filter zero counts out of `normalized` uniformly |
| `nfkc_never_leaves_an_orphaned_selector` | with `Options { nfkc: true, ..Default::default() }`: `"\u{2139}\u{FE0F}"` → `"i"`; `"\u{2122}\u{FE0F}"` → `"TM"`; `"\u{3297}\u{FE0F}"` (CIRCLED IDEOGRAPH CONGRATULATION) → `"\u{795D}"`; `"\u{1F202}\u{FE0F}"` (SQUARED KATAKANA SA) → `"\u{30B5}"`. Each: `findings` empty; `kept` = U+FE0F · variation-selector · LFP · [3] ([4] for the 4-byte U+1F202); `removed == [(VariationSelector, 1)]`; `normalized == [(Nfkc, 1)]`; and `clean` of the output returns it unchanged | delete the pass after NFKC (A §8) — `"i\u{FE0F}"` survives |
| `nfkc_rounds_reach_a_fixed_point` (D26) | `"\u{2139}\u{FE0F}\u{301}"` with `nfkc` → `"\u{ED}"` (LATIN SMALL LETTER I WITH ACUTE); `removed == [(VariationSelector, 1)]`; `normalized == [(Nfkc, 3)]` (round 1: U+2139 → `i`, 1; round 2: `i` U+0301 → U+00ED, 2); a second `clean` returns `"\u{ED}"` with `normalized == [(Nfkc, 0)]` | stop after one round (A §5.3's single second pass) — the output is `"i\u{301}"` (P1, D26) |
| `inspect_never_reads_the_nfkc_knob` | `inspect("\u{2139}\u{FE0F}", nfkc on) == inspect("\u{2139}\u{FE0F}", default)` | let `inspect` run NFKC first |
| `counters_agree_with_positions_without_nfkc` | for `"a\u{200B}b\u{AD}c\u{A0}d\u{200D}e"` under each `Options` with `nfkc: false`: every `removed` count equals the sum of that class's `findings` counts; `normalized[SpaceToAscii]` equals the `ExoticSpace` rows' sum | count `Keep` decisions in `removed` |
| `output_len_is_bytes` | `"\u{E9}\u{200B}"` → text `"\u{E9}"`, `output_len == 2` | `output_len = text.chars().count()` |
| `an_empty_text_is_an_empty_report` | `""` → `inspect`: no rows, not suspicious; `clean`: `""`, `output_len == 0`, `removed` empty, `normalized` empty (`[(Nfkc, 0)]` with `nfkc`) | — (regression) |
| `merging_keeps_source_order` | `merge_in_source_order` of synthetic hits at [0, 5] and [2, 9] → [0, 2, 5, 9] | concatenate instead of merging |

### 5.3 `nfkc.rs` (unit — reads `ucd/` with `include_str!`; D24, P5)

| test | input → expected | mutation |
|---|---|---|
| `nfkc_conforms_to_the_unicode_test_file` | `include_str!("../ucd/NormalizationTest.txt")`: every data line of all six `@Part` sections, Part0–Part5 (D23) (columns `c1;c2;c3;c4;c5`, each a space-separated list of hex code points): `nfkc(cK) == c4` for K = 1..5. Assert that Part0–Part5 were all seen and the line count is ≥ 20,000 (20,171 in 18.0.0); report up to 20 failures as `part · line · column · hex(expected) · hex(got)` | (a) skip canonical reordering; (b) ignore the blocked rule; (c) delete the LV+T branch of Hangul composition; (d) use `T_BASE <= b` in LV+T; (e) let a starter that fails to compose *not* become the new starter — each red |
| `every_code_point_the_test_file_does_not_list_is_its_own_nfkc` | conformance clause 2: `include_str!("../ucd/UnicodeData.txt")`; every assigned code point (expanding `<…, First>`/`<…, Last>` pairs; skipping `gc=Cs`) that is not a `c1` of Part 1 → `nfkc(x) == x` (293,187 code points in 18.0.0) | make the decomposition step emit U+0020 for a code point with no mapping and `ccc == 0` but outside ASCII |
| `hangul_is_decomposed_and_composed_algorithmically` | `"\u{AC00}"` → itself; `"\u{1100}\u{1161}"` → `"\u{AC00}"`; `"\u{1112}\u{1161}\u{11AB}"` → `"\u{D55C}"`; `"\u{1100}\u{1161}\u{11A7}"` → `"\u{AC00}\u{11A7}"`; `"\u{AC01}\u{11A8}"` → itself (an LVT syllable takes no second T) | `T_BASE <= b`; separately, drop the `(a - S_BASE) % T_COUNT == 0` check |
| `the_count_is_what_nfkc_did_not_carry_through` | the fourteen rows of §4.5.4, through `nfkc_counted` | drop `moved` (row `x U+0301 U+0316` gives 0); separately drop `mixed` (row `e U+0301` gives 1) |
| `the_count_is_zero_exactly_when_nothing_changed` | for every column of every line of the test file: `(nfkc_counted(s).1 == 0) == (nfkc_counted(s).0 == s)` | drop `moved` |

### 5.4 `tests/fixtures.rs` and `tests/corpus.rs` (integration — public API only)

`tests/fixtures.rs` holds the table of §4.7 as data:

```rust
struct Row { cp: char, class: UnicodeClass, confidence: Confidence, positions: &'static [usize] }
struct Fixture {
    name: &'static str,
    text: &'static str,      // include_str!("../../../fixtures/text/<name>")
    cleaned: &'static str,   // under Options::default()
    findings: &'static [Row],
    kept: &'static [Row],
    suspicious: bool,
}
const FIXTURES: &[Fixture] = &[ /* the 22 of §4.7, in §4.7's order */ ];
```

| test | file | input → expected | mutation |
|---|---|---|---|
| `every_fixture_is_asserted` | `fixtures.rs` | the `*.txt` names in `fixtures/text/` (read with `std::fs::read_dir` from `concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/text")`) are exactly the names in `FIXTURES` | add an unlisted `fixtures/text/x.txt` |
| `every_class_has_a_fixture` | `fixtures.rs` | for every class in `UnicodeClass::ALL` except those in `const NOT_YET: &[UnicodeClass] = &[UnicodeClass::Homoglyph]` (comment: "E1-4 adds homoglyph.txt and empties this"), `FIXTURES` has `<class id>.txt` | delete `zwj.txt` from the table |
| `each_fixture_says_what_it_claims` | `fixtures.rs` | for each fixture under `Options::default()`: `clean(text).text == cleaned`, `report.findings`/`report.kept` equal the rows (count = positions.len()), `removed` = the findings rows summed per class, `normalized` empty, `inspect(text).suspicious == suspicious` and `clean(text).report.suspicious == suspicious`; plus the knob claims of §4.7.2 (soft hyphen, exotic space) and the `nfkc` claims of §4.7.2–4.7.3 | per class, flip one rule: e.g. make `ZeroWidth` default to `Keep` (red on `zero-width.txt`); delete E1-2's VS16 rule (red on `survive-emoji-presentation.txt`) |
| `the_survivors_survive_every_option` | `fixtures.rs` | each `survive-*.txt` under the eight `Options` with `nfkc: false`: byte-identical output, `findings` empty, `kept` as tabled; under the eight with `nfkc: true`: byte-identical except the two of §4.7.3, which match their tabled output and counters | delete the `kept_by_context` return in `decide` |
| `clean_is_idempotent_on_every_fixture` | `fixtures.rs` | every fixture × the 16 `Options`: `let once = clean(text, &o); let twice = clean(&once.text, &o);` → `twice.text == once.text`, `twice.report.findings` and `removed` empty, every `normalized` count 0 | delete the pass after NFKC (red on `survive-emoji-presentation.txt` with `nfkc`) |
| `inspect_and_clean_agree` | `fixtures.rs` | every fixture × 16 `Options`: `inspect(t, &o).findings == clean(t, &o).report.findings`, same for `kept`, `suspicious` and `stats` (D28: both over the source, by the same rules) | give `inspect` its own copy of `decide` without the `kept_by_context` line; separately, compute `CleanReport::stats` over the output |
| `every_position_names_its_code_point` | `fixtures.rs` | every row of `findings` and `kept`, every fixture × 16 `Options`: `text[p..].starts_with(row.codepoint)` for each position `p`, positions strictly increasing, `count == positions.len()` | record positions as char indices |
| `counters_agree_with_the_rows_on_every_fixture` | `fixtures.rs` | every fixture × the 8 `Options` without `nfkc`: the invariant of `CleanReport`'s doc comment (every `removed` count = its class's `findings` counts; `SpaceToAscii` = the exotic-space rows) | count kept-by-knob hits in `removed` |
| `every_report_names_the_unicode_version` | `fixtures.rs` | the version in the first line of `include_str!("../ucd/NormalizationTest.txt")` (`# NormalizationTest-<v>.txt`) — a file `build.rs` never reads, so an independent witness — equals `wipemark_core::UNICODE_VERSION`; every `inspect` and `clean` report of every fixture carries it, and both JSON forms contain `"unicode_version":"<v>"` | set `CleanReport::unicode_version` to `""` in `clean` |
| `every_json_report_carries_the_third_shelf` | `fixtures.rs` | for every fixture, both JSON forms end with `,"not_established":[` + the ids of `report::not_established::ALL`, quoted, comma-separated, in order + `]}` | drop the `not_established` key from the writer |
| `a_json_report_is_one_line_of_ascii` | `fixtures.rs` | both forms of every fixture × 16 `Options`: `is_ascii()`, no `\n`, `\t` or `' '` outside string values (check: no byte `<= 0x20` except inside a name), starts with `{"unicode_version":"`, ends with `]}`, never contains `"name":null` | make the writer pretty-print (a space after every `:`), and separately end the output with a newline — each red (the escape rule's own mutation is in §5.5: the fixtures' names are ASCII, so only the unit test can paint it) |
| `json_keys_come_in_the_documented_order` | `fixtures.rs` | in both forms of `default-ignorable.txt` and `exotic-space.txt`, the byte index of each top-level key strictly increases in A §7.1's order — inspect: `"unicode_version"`, `"suspicious"`, `"findings"`, `"kept"`, `"stats"`, `"not_established"`; clean: `"unicode_version"`, `"suspicious"`, `"findings"`, `"kept"`, `"removed"`, `"normalized"`, `"output_len"`, `"stats"`, `"not_established"` (D28); the inspect form has no `"removed"`, `"normalized"`, `"output_len"` | swap two keys in the writer; separately, drop `"stats"` from the clean form |
| `clean_is_idempotent_on_a_generated_corpus` | `corpus.rs` | §5.4.1 | delete the pass after NFKC; separately, stop after one round (P1, D26) — both red |
| `inspect_and_clean_agree_on_a_generated_corpus` | `corpus.rs` | the first 2,000 strings of §5.4.1 × 16 `Options`: rows of `inspect` == rows of `clean` | as `inspect_and_clean_agree` |

#### 5.4.1 The generated corpus

Deterministic: no `rand`, no clock, no `HashMap` iteration order.

```rust
/// SplitMix64, as `crates/wipemark-engine/src/fake.rs:24-30` mixes it —
/// copied, because wipemark-core may not depend on anything, not even in
/// tests. The stream is mix64(seed), mix64(seed + γ), mix64(seed + 2γ), …
struct SplitMix64(u64);
impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}
const SEED: u64 = 0x5749_5045_4D41_524B; // "WIPEMARK" in ASCII
const STRINGS: usize = 10_000;
```

**The alphabet** is the union of two lists of *atoms* (an atom is a short
string):

1. `FINDING`: every finding-capable code point, by class, derived — not
   typed — by scanning `0..=0x10FFFF` (skipping surrogates) with
   `wipemark_core::class::class_of`: for each class, **all** its code
   points when it has at most 80 (so every zero-width character, the ZWJ,
   every bidi control, the soft hyphen, all 16 exotic spaces and all 66
   noncharacters), otherwise 16 evenly spaced ones including the first and
   the last (`index = k * (len - 1) / 15`, k = 0..15). Each is a
   one-character atom.
2. `ATOMS`, typed here (escapes only):

```rust
const ATOMS: &[&str] = &[
    // Latin, Cyrillic, Greek letters (E1-4's material)
    "a", "e", "i", "p", "y", "M", "T", "\u{430}", "\u{440}", "\u{443}", "\u{43F}", "\u{3BF}", "\u{3B1}",
    // joining scripts, RTL, Han, Hangul, Mongolian, Khmer
    "\u{645}", "\u{6CC}", "\u{631}", "\u{5D0}", "\u{915}", "\u{94D}", "\u{937}", "\u{8FBB}", "\u{795D}",
    "\u{AC00}", "\u{D55C}", "\u{1100}", "\u{1161}", "\u{11A8}", "\u{1820}", "\u{1780}",
    // digits, punctuation, whitespace — a newline ends a paragraph
    "0", "1", "#", ".", " ", "\n", "\t",
    // combining marks (NFKC composes and reorders them)
    "\u{301}", "\u{323}", "\u{308}", "\u{316}",
    // emoji and emoji components
    "\u{2764}", "\u{1F525}", "\u{1F469}", "\u{1F467}", "\u{1F3F3}", "\u{1F308}", "\u{2696}", "\u{1F3FB}",
    "\u{1F3F4}", "\u{20E3}", "\u{2640}",
    // compatibility characters NFKC rewrites
    "\u{2139}", "\u{2122}", "\u{FB01}", "\u{3297}", "\u{1F202}", "\u{FF21}", "\u{FF4E}", "\u{1D400}",
    "\u{BD}", "\u{212B}", "\u{1C4}", "\u{E9}",
    // whole sequences: orthography and presentation that must survive, and the known traps
    "\u{2139}\u{FE0F}", "\u{2122}\u{FE0F}", "\u{3297}\u{FE0F}", "\u{1F202}\u{FE0F}",
    "\u{2139}\u{FE0F}\u{301}", "\u{2122}\u{FE0F}\u{323}",
    "\u{2764}\u{FE0F}\u{200D}\u{1F525}", "\u{1F469}\u{200D}\u{1F467}", "\u{1F3F3}\u{FE0F}\u{200D}\u{1F308}",
    "\u{1F3F4}\u{E0067}\u{E0062}\u{E0073}\u{E0063}\u{E0074}\u{E007F}", "1\u{FE0F}\u{20E3}",
    "\u{8FBB}\u{E0100}", "\u{2229}\u{FE00}", "\u{645}\u{200C}\u{631}", "\u{915}\u{94D}\u{200D}\u{937}",
    "\u{645}\u{200E}", "\u{1820}\u{180B}", "\u{1780}\u{17B4}", "\u{1100}\u{1160}", "\u{D55C}\u{3164}",
    "e\u{301}",
];
```

**A string**: `let atoms = rng.next() % 16;` then, per atom, one draw `d
= rng.next()`: when `d % 2 == 0` the atom is `ATOMS[((d >> 1) %
ATOMS.len() as u64) as usize]`, otherwise `FINDING[((d >> 1) %
FINDING.len() as u64) as usize]`. Concatenate. Generate `STRINGS` strings
from one generator seeded with `SEED`.

**The assertion**, for every string `s` and every one of the 16 `Options`
`o`: `let once = clean(&s, &o); let twice = clean(&once.text, &o);`
`twice.text == once.text`, `twice.report.findings.is_empty()`,
`twice.report.removed.is_empty()`, every count in `twice.report.normalized`
is 0, and `once.report.output_len == once.text.len()`. On failure print
the string index, the `Options`, and `hex` of `s`, `once.text` and
`twice.text`. The two trap atoms make P1's counterexample occur hundreds of
times in the corpus, so "stop after one round" is red deterministically —
confirm it while doing the mutation check. Note the test's run time in
the report; in a debug build it should be seconds, and if it is not,
profile before shrinking `STRINGS`.

### 5.5 `json.rs` (unit)

| test | input → expected | mutation |
|---|---|---|
| `the_json_form_of_an_inspect_report_is_exact` | the hand-built report of §4.6.4 → the exact string there | swap `"kept"` and `"findings"` in the writer |
| `the_json_form_of_a_clean_report_is_exact` | the hand-built report of §4.6.4 → the exact string there | write `"action":"remove"` for every finding |
| `json_escapes_what_json_requires_and_everything_past_ascii` | `push_string` of `"a\"b\\c\u{0}\u{1F}\u{7F}\u{E9}\u{1F525}"` → `r#""a\"b\\c\u0000\u001f\u007f\u00e9\ud83d\udd25""#` | drop the `>= U+007F` arm (U+00E9 comes out raw) |
| `the_json_action_is_the_action_clean_took` | for each class and each of the 16 `Options` where `action_for(class) != Keep`: `acted_action(class) == action_for(class)` | map `ExoticSpace` to `Remove` in `acted_action` |
| `an_empty_report_is_still_a_whole_report` | `inspect("", default).to_json()` contains `"findings":[],"kept":[]` and the `not_established` array; `clean("", default).report.to_json()` contains `"removed":{},"normalized":{},"output_len":0` | skip empty arrays/objects |

### 5.6 What to record

The report's mutation table has one line per row above that names a
mutation: *protection · mutation · test that went red · the failure line*.
A row whose test stayed green under its mutation is a failed protection:
fix the test (or report it) — never keep a test that cannot fail. Also
record the two facts this document asserts but does not gate by mutation:
the corpus run time, and the number of NormalizationTest lines checked.

## §6 Acceptance criteria

- [ ] `lib.rs` exposes exactly README §3.3's API: `inspect`, `clean`,
  `Options` (`Debug, Clone, Default, PartialEq, Eq`; `action_for`),
  `Cleaned` (`Debug, Clone, PartialEq`), and re-exports
  `UNICODE_VERSION` and `name_of` once each — evidence: `grep -n "pub fn\|pub struct\|pub use" crates/wipemark-core/src/lib.rs`.
- [ ] `Action` is `{ Remove, Replace, Keep }` with `as_str`;
  `Confidence::as_str` and `NormKind::as_str` exist with the ids of §4.1–4.2;
  `NormKind::Homoglyph` exists — evidence: §5.1 tests;
  `grep -rn NormalizeToSpace crates apps` prints nothing.
- [ ] `InspectReport` has `kept`; `CleanReport` has `findings`, `kept`,
  `suspicious`, `stats` (D28, over the source), `removed`, `normalized`,
  `output_len`, `unicode_version`, no `Default`,
  and the doc comment of §4.2 including the sentence about the only place
  a count exceeds its positions and "is not offered and never will be" —
  evidence: `grep -n "only place" crates/wipemark-core/src/report.rs`.
- [ ] `scrub.rs` has one `decide` used by both `inspect` and `clean`;
  `collect_hits` is the seam of §4.4.1 — evidence:
  `inspect_and_clean_agree`, `context_beats_every_knob`,
  `merging_keeps_source_order`.
- [ ] NFKC conforms: every line of every part of `NormalizationTest.txt`
  and clause 2 — evidence: the two tests of §5.3 and the line counts in
  the report.
- [ ] `clean` is idempotent on every fixture and on the generated corpus
  under all 16 `Options`, including the P1 case (D26) — evidence:
  `nfkc_rounds_reach_a_fixed_point`, `nfkc_never_leaves_an_orphaned_selector`,
  §5.4.
- [ ] Both JSON forms are exactly as §4.6, ASCII, one line, third shelf
  last — evidence: §5.4 and §5.5 tests.
- [ ] The 22 fixtures exist byte-exact (`shasum -a 256 -c` prints 22
  `OK`), `git check-attr text` says `unset`, and every one is asserted —
  evidence: `every_fixture_is_asserted`, `each_fixture_says_what_it_claims`.
- [ ] Every mutation of §5 performed and recorded (§5.6).
- [ ] E1-1's `#![cfg_attr(not(test), allow(dead_code))]` handled per
  §4.3.1: removed from every module `clippy` then passes without it; the
  items that still have no caller listed in the report — evidence:
  `grep -n "allow(dead_code)" crates/wipemark-core/src/*.rs` before and
  after, in the report.
- [ ] The six gate commands of §0.5 are green; `git diff --stat Cargo.lock`
  is empty; `scripts/check-dep-direction.sh` reports `wipemark-core` with
  no dependencies.
- [ ] `docs/architecture/layer-a.md` has the section of §4.8;
  `fixtures/README.md` has the paragraph of §4.7.4; README §3 row E1-3 is
  *done* with the report's file name.
- [ ] The closing report contains, besides §0.7's list: the table of §1.1
  (P1–P9) with "implemented as specified" or the deviation (P8 is this
  document's own and is not yet a README decision); and — for the
  README §3.1 checkpoint "run `clean` by hand over the fixtures and read the
  report" — the `to_json()` of `clean` for `zero-width.txt` (default),
  `survive-emoji-zwj.txt` (default), `exotic-space.txt`
  (`normalize_spaces`) and `survive-emoji-presentation.txt` (`nfkc`),
  produced by a throwaway test run with `--nocapture` that is **not**
  committed.

## §7 Out of scope

- **Homoglyph detection** — E1-4. What *is* here: the `Replace` path for
  `Homoglyph`, `NormKind::Homoglyph`, its counter, its JSON, the merge, all
  exercised with synthetic hits.
- **Guards** — E1-5.
- **Surfaces** — MCP `inspect`/`clean`, CLI `inspect`/`clean`, exit codes,
  the catalogue keys `unicode-class-*`/`confidence-*`, `with_infix`, the
  i18n note on character names (D16) — E1-6. Nothing under `apps/` changes
  here.
- **The live gate, `CLAUDE.md`, the skeleton tables** — E1-7.
- **Formats** — Markdown/HTML/code awareness and `⟦n⟧` placeholders (A §2,
  §7.5; E4). With `nfkc`, Layer A normalises code too; the knob's doc says
  so.
- **Settings rows for the knobs** (A §7.4) and **per-class overrides**
  (Q-A1); **unassigned code points as findings** (Q-A3).
- **NFC, NFD or NFKD** as functions; a JSON *parser* in core; pretty-printed
  JSON.
- **E1-1's tables and E1-2's rules** — except a context-rule fix that this
  document's idempotence or fixture gates prove necessary (A4, A5/D20),
  recorded as a deviation with the input that showed it.

## §8 Basis and references

**Plan and repository.** `docs/plan/README.md` §3.2–3.3 (the contract),
§4 D3–D6, D9, D17, D20, D23–D29, §6 (risks: "Idempotence under NFKC"),
§9 (this §0);
`CLAUDE.md` (rules cited in §2); `fixtures/README.md`;
`docs/architecture/retention.md:127-130` (quotes `CleanReport`'s
reversibility sentence).

**Specifications (Watchword; working copies in `ssd-docs/`).** A
(`wipemark-core-layer-a-2026-09-21`): §1 (the `CleanReport` and NFKC rows),
§2, §4.1–4.3, §5.1–5.3, §5.5, §7.1, §7.5, §8. OV
(`heretic-unmark-overview-decomposition-2026-09-07`): §0.1 rule 3 (three
shelves) and rule 5 (zero deps in core), §2 (the domain model:
`InspectReport`, `CleanReport`, `Confidence`), §3.1 (the classes and the
NFKC knob), §3.2 (guarantees: exact positions and counts, idempotence as a
gate, proven-RED protections).

**Unicode 18.0.0.**
- UAX #15, *Unicode Normalization Forms*: <https://www.unicode.org/reports/tr15/>.
- `NormalizationTest.txt` (header: the two conformance clauses; six
  parts): <https://www.unicode.org/Public/18.0.0/ucd/NormalizationTest.txt>.
- The Unicode Standard, Version 18.0, Chapter 3 — §3.11 *Normalization
  Forms* (D107 Starter, D108 Reorderable pair, D109 Canonical Ordering
  Algorithm, D113 Full composition exclusion, D114 Primary composite, D115
  Blocked, D117 Canonical Composition Algorithm) and §3.12 *Conjoining Jamo
  Behavior* (`SBase`, `LBase`, `VBase`, `TBase`, `LCount`, `VCount`,
  `TCount`, `NCount`, `SCount`):
  <https://www.unicode.org/versions/Unicode18.0.0/core-spec/chapter-3/>.
- UAX #44, *Unicode Character Database* — file formats, `First`/`Last`
  ranges in `UnicodeData.txt`, code point labels:
  <https://www.unicode.org/reports/tr44/>.
- `StandardizedVariants.txt` (P8):
  <https://www.unicode.org/Public/18.0.0/ucd/StandardizedVariants.txt>.
- UTS #51, *Unicode Emoji* (VS16 presentation, ZWJ and tag sequences, the
  fixtures of §4.7.3): <https://www.unicode.org/reports/tr51/>.

**JSON.** RFC 8259, *The JavaScript Object Notation (JSON) Data
Interchange Format* — §7 (string escapes, `\u` with UTF-16 surrogate
pairs), §6 (numbers): <https://www.rfc-editor.org/rfc/rfc8259>.

**Generator.** SplitMix64 (Steele, Lea, Flood, *Fast Splittable
Pseudorandom Number Generators*, OOPSLA 2014), as already used by
`crates/wipemark-engine/src/fake.rs:21-30`.
