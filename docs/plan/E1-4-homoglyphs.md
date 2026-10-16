# E1-4 — Homoglyphs: confusable letters, the mixed-word rule, and why Russian prose is not an attack

|                  |                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic E1, Layer A                                                                                                                                                                                                                                                                                                                                                                                                       |
| Spec scopes      | **S1.6** — A §5.4 (all of it), A §1 row "Homoglyphs", A §5.3 (the homoglyph half of the idempotence argument), A §8 row "гомоглифы"; OV §3.1 row "Homoglyphs"                                                                                                                                                                                                                                                                         |
| Depends on       | **E1-3** committed on `feat/e0-e6-shell` (and through it E1-1, E1-2): `confusable_target`, `confusables_with`, `script_of`, the letter/mark/digit/case predicates, `decomposition` (E1-1); `class_of`, `Hit` (E1-2); `Options`, `inspect`, `clean`, the `collect_hits` seam, `NormKind::Homoglyph`, `fixtures/text/` and `tests/fixtures.rs` (E1-3). Decisions D3, D4, D6, D21 (revised), D22, D24, D26, D40, D41, D42 of `docs/plan/README.md` §4         |
| Unblocks         | **E1-6** (the MCP and CLI `aggressive` flag has something to act on; E1-6's own tests use `p\u{0430}y`)                                                                                                                                                                                                                                                                                                                                |
| Files touched    | new: `crates/wipemark-core/src/homoglyph.rs`, `crates/wipemark-core/tests/homoglyphs.rs`, `fixtures/text/homoglyph.txt`, `fixtures/text/survive-homoglyph-prose.txt`, `docs/plan/reports/E1-4-<YYYY-MM-DD>.md`; edited: `crates/wipemark-core/src/lib.rs` (one `mod` line), `crates/wipemark-core/src/scrub.rs` (one expression), `crates/wipemark-core/tests/fixtures.rs` (two rows, `NOT_YET` emptied), `crates/wipemark-core/src/tables.rs` (only E1-1's `dead_code` allowance on the items this document calls, §4.8), `fixtures/README.md` (one clause), `docs/architecture/layer-a.md` (section "Homoglyphs"), `docs/plan/README.md` (status row) |
| Size             | ~2 days for one agent; no network, no application launch                                                                                                                                                                                                                                                                                                                                                                              |

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

A homoglyph mark is a letter of one script typed inside a word of another
where it looks the same: a Cyrillic U+0430 in the English word "pay", a
Latin U+0061 in the Russian word "парк". Layer A has every other class
after E1-3; this document adds the eleventh. It writes
`homoglyph::hits(text)` — every homoglyph in the text, each with the
letter that should replace it — and plugs it into the one seam E1-3 left
for it, so that `inspect` reports homoglyphs always (D3) and `clean`
replaces them under `Options::aggressive`.

The hard part is not finding a Cyrillic letter in a Latin word. It is
**not** finding one in Russian. confusables.txt says that 19 of the 33
lower-case Russian letters look like a Latin letter; a detector that asks
only "does this letter have a Latin twin?" rewrites Russian prose letter
by letter, which is what the reference implementation does and what
A §1 calls "уничтожение документа" — destroying the document — for a
product whose owner writes Russian. The mixed-word rule of A §5.4 is
what prevents it: a letter is a finding only when **its word** mixes
scripts, or when a whole word is drawn in look-alikes of the script
around it. This document makes that rule exact, measures it against the
real 18.0.0 data, and adds what the data shows it needs to stay
idempotent and to keep the spec's own promise that an English word in a
Russian paragraph is an English word.

When it lands: `russian_prose_is_not_a_homoglyph_attack` is green and
goes red without the mixed-word condition; `p` U+0430 `y` is one finding
replaced by U+0061 under `aggressive`; `clean(clean(x)) == clean(x)` on a
generated corpus of 10 000 mixed-script strings.

## §2 Read first

In this order. Line numbers are at `497eafa` unless the row says
otherwise; E1-1…E1-3 will have moved some of them — re-find by name.

1. `CLAUDE.md` in full. The rules this document leans on (line numbers
   at `497eafa`; HEAD `a2f6ba3` added 9 lines near the top, so add 9 there):
   "Delete the protection and watch it go red" (`:85`), "`wipemark-core`
   has zero dependencies" (`:192`), "Every string a person reads comes
   from the catalogue" (`:220`), "Only applications localize" (`:226`),
   "Tests must be able to fail" (`:1037`).
2. `docs/plan/README.md` — §3.3 (the interface contract; the E1-4 row is
   one function), §4 D3, D4, D6, D21 (revised), D22, D24, D26, D40, D41,
   D42, §6 "Russian prose
   under `aggressive`".
3. `ssd-docs/wipemark-core-layer-a-2026-09-21.md` (A) — §1 row
   "§3.1 Homoglyphs", §4.1 row `Homoglyph`, §5.2, §5.3, **§5.4 entirely**,
   §8 row "гомоглифы". Russian; §4.1 below is its exact English.
4. `crates/wipemark-core/src/class.rs` — `UnicodeClass::Homoglyph` and its
   doc (`:69-70`), its default `Keep` (`:122-123`), its ceiling `Probable`
   (`:138-141`), `requires_aggressive` (`:146-149`), `UnicodeFinding`
   (`:152-164`).
5. `crates/wipemark-core/src/report.rs` — `NormKind` (`:75-82`; E1-3 adds
   `Homoglyph`), `CleanReport` (`:84-93`; E1-3 rewrites it).
6. `crates/wipemark-core/src/lib.rs` — the module list (`:29-32`) and the
   crate doc that already names `confusables.txt` (`:10-19`).
7. `docs/plan/E1-1-ucd-tables.md` — §4.8.8 (the confusables filter, the
   homoglyph-capable set H, the reverse index), §4.9 rows
   `confusable_target` and `confusables_with`, §5 tests
   `the_reverse_index_holds_every_twin_and_the_prototype` and
   `confusable_sources_map_to_their_skeleton`.
8. `docs/plan/E1-3-scrubber-and-nfkc.md` — §4.4.1 (the seam), §4.4.2
   (`decide`, `replacement`), §4.4.7–4.4.8 (the NFKC rounds of D26 and
   why `clean` is idempotent — it assumes "E1-4's letters are NFKC
   stable"), §4.7 (`fixtures/text/` and its naming), §5.4 (`fixtures.rs`,
   `NOT_YET`, the generated corpus and its `SplitMix64`).
9. `crates/wipemark-core/src/homoglyph.rs` does not exist yet; read
   `crates/wipemark-core/src/scrub.rs` and `src/context.rs` as E1-3 and
   E1-2 left them.
10. UTS #39 §4 (confusable detection, skeletons):
    <https://www.unicode.org/reports/tr39/#Confusable_Detection>; UAX #24
    (Script property, `Common`/`Inherited`):
    <https://www.unicode.org/reports/tr24/>.

## §3 What is true today, and what earlier documents delivered

### 3.1 At `497eafa`

| fact | where |
|---|---|
| `UnicodeClass::Homoglyph` exists, documented as "Latin/Cyrillic/Greek confusables, from `confusables.txt`"; id `"homoglyph"` | `crates/wipemark-core/src/class.rs:69-70`, `:102` |
| its default action is `Keep` ("Aggressive mode only — see `requires_aggressive`"); its confidence ceiling is `Probable` | `class.rs:122-123`, `:138-141` |
| `requires_aggressive()` is true for `Homoglyph` only, documented as "Classes that only *act* when the user asked for aggressive mode" — detection is not gated, which D3 makes the rule | `class.rs:146-149` |
| `risky_classes_default_to_keep` pins `Homoglyph → Keep` and `requires_aggressive` | `class.rs:207-212` |
| `UnicodeFinding.positions` are byte offsets into the source | `class.rs:152-164` |
| no homoglyph code, no tables, no `NormKind::Homoglyph` | `report.rs:75-82`; `lib.rs:29-32` |
| `fixtures/text/` holds only `README.md` | `fixtures/README.md` |
| HEAD has since moved to `a2f6ba3` (submodule URL, Compare window, docs); nothing under `crates/` changed | `git diff --stat 497eafa a2f6ba3 -- crates` is empty |

### 3.2 What E1-1 delivered (the contract you rely on)

README §3.3, sharpened by `docs/plan/E1-1-ucd-tables.md` §4.8.8 and §4.9.
All in `crate::tables` unless named otherwise.

```rust
pub(crate) fn is_letter(c: char) -> bool;            // gc = L*
pub(crate) fn is_mark(c: char) -> bool;              // gc = M*
pub(crate) fn is_decimal_digit(c: char) -> bool;     // gc = Nd
pub(crate) fn is_uppercase_letter(c: char) -> bool;  // gc = Lu
pub(crate) fn is_lowercase_letter(c: char) -> bool;  // gc = Ll
pub(crate) fn script_of(c: char) -> Script;          // folded; Inherited for combining marks; Other for unnamed scripts
pub(crate) fn decomposition(c: char) -> Option<&'static [char]>; // Some when c has any decomposition mapping
pub(crate) fn confusable_target(c: char) -> Option<char>;
pub(crate) fn confusables_with(target: char, script: Script) -> &'static [char];
pub enum Script { Latin, Cyrillic, Greek, …, Common, Inherited, Other }   // crate::script
pub fn name_of(c: char) -> Option<Cow<'static, str>>;                    // crate::name
```

What you rely on, precisely:

- `confusable_target(c)` is `Some(skeleton)` for a **kept source** and
  `None` for everything else — **including a prototype**. The skeleton of
  any `c` is therefore `confusable_target(c).unwrap_or(c)`. A kept source
  is a letter (gc `L*`) whose single-code-point target is in
  `confusables.txt` and whose script is Latin, Cyrillic or Greek (E1-1
  §4.8.8; whether the 82 halfwidth/fullwidth-block sources are still in
  the table after D22 does not matter to this document, §4.2 rule M1
  excludes them on its own).
- `confusables_with(k, s)` is **every member of H** — kept sources and
  the prototypes that pass the same filter — whose skeleton is `k` and
  whose script is `s`, ascending, **including `k` itself** when `k` is in
  H and of script `s`. Pinned by E1-1's
  `the_reverse_index_holds_every_twin_and_the_prototype`:
  `confusables_with(U+0061, Cyrillic) == [U+0430]`;
  `(U+0061, Latin) == [U+0061, U+0251, U+AB64, U+FF41, U+1DF5A]`;
  `(U+006F, Greek) == [U+03BF, U+03C3]`; `(U+006F, Cyrillic) == [U+043E, U+1C82]`.
- No one-code-point target is itself a source (the mapping is idempotent),
  so a skeleton's own skeleton is itself.
- `name_of` is `Some` for all 575 homoglyph-capable code points (sources
  **and** prototypes) — a Latin U+0061 found in a Cyrillic word is a
  prototype, not a source, and still has its name. E1-6 relies on this
  for every homoglyph row; nothing in this document calls `name_of`.

### 3.3 What E1-2 delivered

```rust
pub fn class_of(c: char) -> Option<UnicodeClass>;   // crate::class — never Homoglyph
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Hit {                             // crate::context
    pub at: usize, pub c: char, pub class: UnicodeClass, pub confidence: Confidence,
    pub kept_by_context: bool, pub replacement: Option<char>,
}
```

`class_of('\n')` is `None` (A §2: C0 controls other than the three
whitespace ones are never findings, and `\n` is not one at all);
`class_of` of a letter, a mark or a digit is `None`.

### 3.4 What E1-3 delivered

- `Options { aggressive, nfkc, normalize_spaces, keep_soft_hyphen }`,
  `Default` all `false`; `Options::action_for(Homoglyph)` is `Replace`
  when `aggressive`, else `Keep` (E1-3 §4.3).
- `scrub::collect_hits(text, _options)` is
  `merge_in_source_order(context::hits(text), Vec::new())` with the
  comment "E1-4 replaces `Vec::new()` with `homoglyph::hits(text)`";
  the merge `debug_assert!`s that `at` is strictly increasing and that
  the two lists never claim one code point (E1-3 §4.4.1).
- `decide` keeps a homoglyph hit on `Keep`, writes `hit.replacement` on
  `Replace`, and keeps a hit whose `replacement` is `None` (E1-3 §4.4.2).
  Rows are aggregated by `(codepoint, class, confidence, acted)` (D6):
  a homoglyph lands in `findings` under `aggressive`, in `kept` without.
  `suspicious` is true when any row of `findings ∪ kept` is ≥ `Probable`
  (D4) — so a kept homoglyph makes a text suspicious.
- With `nfkc`, `clean` runs NFKC and the pass in rounds until a pass acts
  on nothing (D26); rows come from the first pass only, later passes add
  counts to `removed`/`normalized`. E1-3 §4.4.8 proves idempotence on
  the premise that "U+0020 and E1-4's letters are NFKC stable" — D42
  (§4.2 rule T1) is what makes that premise true.
- `NormKind::Homoglyph` (`"homoglyph"`) and its counter exist.
- `fixtures/text/` holds 22 files: `<class id>.txt` (cleaned) and
  `survive-*.txt` (byte-identical under every `Options`);
  `crates/wipemark-core/tests/fixtures.rs` has the `FIXTURES` table,
  `every_fixture_is_asserted`, and `every_class_has_a_fixture` with
  `const NOT_YET: &[UnicodeClass] = &[UnicodeClass::Homoglyph]`
  ("E1-4 adds homoglyph.txt and empties this").
- `docs/architecture/layer-a.md` exists (E1-1 created it) with sections
  up to "Scrubber, report and NFKC".

If HEAD differs from any of this, implement against HEAD and record the
difference in your report.

### 3.5 Decisions that bind this document

| # | what it says here |
|---|---|
| **D3** | `homoglyph::hits` runs whatever the `Options` say; `aggressive` only turns `Keep` into `Replace`. `hits` therefore takes no `Options`. |
| **D4** | A kept homoglyph (`Probable`) makes `suspicious` true. Every false positive of this document is therefore a "this text is marked" claim to a user who did not ask for `aggressive` — which is why §4.5's precision points all lean towards *not* finding. |
| **D6** | One row per `(codepoint, class, confidence, acted)`. |
| **D21** (revised) | The replacement rule: (1) the prototype, when it is a letter of the word's script with the same case; else (2) among the letters of the word's script with that skeleton, that case **and no decomposition** (D42), the one with the **lowest code point**; else (3) no finding. A whole word is redrawn only when every letter resolves. |
| **D22** | A §3.2/§5.4's "or from the Halfwidth and Fullwidth Forms block" clause is dropped. |
| **D24** | Tests of `pub(crate)` items (`homoglyph::hits` and its helpers) are unit tests in `src/homoglyph.rs`; tests through `inspect`/`clean` go to `tests/`. |
| **D26** | NFKC rounds until the pass acts on nothing; a replacement that meets a combining mark is composed by the next round. |
| **D40** | A whole word is redrawn into the paragraph's script only if every letter is a confusables source whose prototype is a letter of the paragraph's script — condition W5 below (§4.5 H6). |
| **D41** | The five idempotence repairs H1–H5 of §4.5: transparent characters do not split a word; a tie goes to the paragraph's script only when it is in the tie; the basis credits whole words; the 10 % share is per word; whole-word redraws are judged on resolved letters. |
| **D42** | A twin with a decomposition is never a replacement, and the veto is applied **before** D21's lowest-code-point choice — rule T1 below (§4.5 H7). |

### 3.6 What the data says (verified against `confusables.txt`, `Scripts.txt`, `UnicodeData.txt` 18.0.0)

Downloaded from `https://www.unicode.org/Public/18.0.0/security/confusables.txt`
(header `# Version: 18.0.0`, `# Date: 2026-08-06`) and
`…/18.0.0/ucd/{Scripts,UnicodeData}.txt`, and replayed through a model
of §4 (outside the repository). These are the facts §4 is built on:

- **Russian is full of look-alikes.** Of the 66 Russian letters, 35 have
  a Latin twin under D21 and D42: lower case U+0430→U+0061, U+0432→U+0299,
  U+0433→U+0072, U+0435→U+0065, U+0437→U+025C, U+0438→U+1D0E,
  U+043A→U+0138, U+043C→U+028D, U+043D→U+029C, U+043E→U+006F,
  U+0440→U+0070, U+0441→U+0063, U+0442→U+1D1B, U+0443→U+0079,
  U+0444→U+0278, U+0445→U+0078, U+0448→U+0077, U+044C→U+0185,
  U+044F→U+1D19; upper case U+0410→U+0041, U+0412→U+0042, U+0413→U+A7E2,
  U+0415→U+0045, U+0417→U+01B7, U+041A→U+004B, U+041B→U+0245, U+041C→U+004D,
  U+041D→U+0048, U+041E→U+004F, U+0420→U+0050, U+0421→U+0043,
  U+0422→U+0054, U+0423→U+0059, U+0425→U+0058, U+042C→U+0184. Whole
  common Russian words are "redrawable" into Latin (нет, как, все, он,
  мама, Тариф). A rule that looked at letters alone would rewrite them.
- **A twin is what the eye sees, not what the writer meant.** U+043A
  CYRILLIC SMALL LETTER KA has the skeleton U+0138 LATIN SMALL LETTER
  KRA, not U+006B; U+0432 has U+0299 LATIN LETTER SMALL CAPITAL B. A
  replaced word looks exactly as it did; it may not spell what the
  writer typed on a Latin keyboard. That is correct for this product:
  the reader saw "кey" (U+043A U+0065 U+0079) as "ĸey" (U+0138 U+0065
  U+0079), and still does.
- **Skeletons drop case.** U+0049 LATIN CAPITAL LETTER I and U+0406
  have the skeleton U+006C LATIN SMALL LETTER L; U+042C CYRILLIC CAPITAL
  LETTER SOFT SIGN has U+0062 (lower case). Case is matched on the
  *letters*, never on the skeleton (D21).
- **Several twins are common where it matters most, and the lowest code
  point is the everyday letter (D21 step 2).** Twelve ASCII letters
  have two or three same-case twins in the other script (for Greek `p`
  and `Y`, D42 removes one of them first); where a choice remains, the
  lowest code point is, in every case, the letter in daily use, and the
  others are historic or minority-language letters added later: into
  Cyrillic `c` → U+0441 (not U+1C83), `e` → U+0435 (not U+04BD), `i` → U+0456 (not U+A647), `o` → U+043E (not
  U+1C82), `w` → U+0448 (not U+0461, U+051D), `y` → U+0443 (not
  U+04AF), `I` → U+0406 (not U+04C0), `Y` → U+0423 (not U+04AE); into
  Greek `o` → U+03BF (not U+03C3), `p` → U+03C1 (not U+03F8; U+03F1 is
  vetoed by D42), `M` → U+039C (not U+03FA), `Y` → U+03A5 (U+03D2 GREEK
  UPSILON WITH HOOK SYMBOL has the decomposition `<compat> U+03A5`, so
  D42 removes it before the choice and U+03A5 is the only candidate).
  So a Latin `o` in the Russian word "дoм" (U+0434 U+006F U+043C) is a
  finding, replaced by U+043E.
- **The prototype is not always the lowest code point.** For seven
  letters it is not, and D21 step 1 decides: U+0444 CYRILLIC SMALL
  LETTER EF → U+0278 LATIN SMALL LETTER PHI (the prototype), not U+0239
  LATIN SMALL LETTER QP DIGRAPH; likewise U+03C6 and U+03D5 → U+0278,
  and U+03B5, U+03F5, U+0454, U+0511 → U+A793 LATIN SMALL LETTER C WITH
  BAR (the prototype), not U+025B LATIN SMALL LETTER OPEN E.
- **Some twins are compatibility characters (D42).** Latin `c`'s only
  Greek twin is U+03F2 GREEK LUNATE SIGMA SYMBOL, whose decomposition is
  `<compat> U+03C2`; `C`'s is U+03F9 (`<compat> U+03A3`); every
  fullwidth letter has a `<wide>` one. NFKC rewrites them, which would
  break E1-3's premise (§3.4); D42 (rule T1) refuses them, so Latin `c`
  in a Greek word is no finding at all. Applying the veto before or
  after the lowest-code-point choice gives the same twin for all 488
  resolvable pairs in 18.0.0 — the order D42 fixes is for future data.
- **The halfwidth/fullwidth clause adds nothing a word needs (D22).**
  Every fullwidth Latin letter U+FF21–FF3A, U+FF41–FF5A is
  `Script=Latin` already. Of the 82 other sources the clause admitted,
  under D21 and D42 exactly one would ever resolve in a Latin, Cyrillic or Greek
  word: U+FFDA HALFWIDTH HANGUL LETTER EU (skeleton U+30FC) → U+A7F7
  LATIN EPIGRAPHIC LETTER SIDEWAYS I — a Korean vowel replaced by a
  Latin epigraphic letter.
- **Final sigma has no twin.** U+03C2 is in no skeleton class with a
  Latin or Cyrillic letter, so a Greek word ending in ς is never
  redrawable. U+03C3 (medial σ) has the skeleton U+006F.
- 488 (letter, other script) pairs among Latin, Cyrillic and Greek
  letters resolve under §4.2; every resolved twin is a letter of the
  target script, of the same case, with no decomposition (the property
  `every_twin_is_a_stable_letter_of_the_same_case` asserts).

## §4 Deliverables

### 4.1 The rule, as A §5.4 states it (exact English)

> `confusables.txt` is a skeleton function: two characters are
> confusable when they have one skeleton. We use it **in both
> directions**, because a mark is inserted into Cyrillic text with a
> Latin `a` as easily as into Latin text with a Cyrillic one.
>
> A *word* is a maximal run of characters with `L*`, `M*`, `Nd`. The
> *script of a word* is the script of the majority of its letters; on a
> tie, the script of the majority of the paragraph's letters. A
> character `c` in a word `w` is a `Homoglyph` finding when:
>
> 1. the script of `c` ∈ {Latin, Cyrillic, Greek} *(D22 drops "or `c` is
>    from the Halfwidth and Fullwidth Forms block")*, **and**
> 2. the script of `c` ≠ the script of `w`, **and**
> 3. the script of `w` has exactly one letter with the same skeleton
>    (with the same case, if it has one) — that letter is the
>    replacement *(D21 replaces this rule)*.
>
> Plus the **whole-word** case: all letters of `w` are of one script,
> each has a skeleton twin in the paragraph's script, and the
> paragraph's script is different — then the whole word is redrawn
> (`рау` in an English paragraph → `pay`). But **not the reverse**: `pay`
> in a Russian paragraph is an English word, and it *has* twins (`рау`),
> *[here and below, `рау` = U+0440 U+0430 U+0443, all Cyrillic; `pay` is ASCII]*
> so the condition is supplemented: the paragraph's script is determined
> by the letters **outside** such whole-redrawable words, and a word is
> redrawn only if its script makes up less than 10 % of the paragraph's
> letters. An English quotation in Russian text is not 10 %.
>
> Confidence: `Probable`, the class ceiling. Replacement:
> `Action::Replace`; counted in `normalized` under `NormKind::Homoglyph`.

§4.2–4.4 are this rule made exact. §4.5 lists every place where the
exact form says more than A §5.4, with the evidence for each.

### 4.2 Vocabulary

All definitions are per **paragraph** and name E1-1/E1-2 functions by
their contract names.

- **LCG** — the three scripts `Script::Latin`, `Script::Cyrillic`,
  `Script::Greek`.
- **Paragraph** — a maximal substring containing no U+000A LINE FEED;
  the text's start and end bound the first and last one. The same
  definition E1-2 uses for the RTL test (A §4.2: "абзац (между \n)").
  U+000D, U+2028 and U+2029 do not end a paragraph (a CRLF file
  paragraphs exactly like an LF one, because U+000D is never a letter).
- **Word character** — `is_letter(c) || is_mark(c) || is_decimal_digit(c)`.
- **Transparent character** — `class_of(c)` is `Some(class)` and `class`
  is not `ExoticSpace`: a character Layer A removes, or keeps only
  because of context. Exotic spaces are spaces, and separate words.
- **Word** — a maximal sequence of word characters in which two
  consecutive word characters may be separated by any run of
  transparent characters and by nothing else. Transparent characters
  belong to the word only *between* its word characters, and are never
  looked at. A word never spans U+000A.
- **Voting letter** — a word character with `is_letter(c)` whose
  `script_of(c)` is neither `Common` nor `Inherited`. Digits, marks and
  `Common`/`Inherited` letters (U+30FC, modifier letters of `Common`)
  belong to the word and do not vote.
- **Case** — `Upper` when `is_uppercase_letter(c)` (Lu), `Lower` when
  `is_lowercase_letter(c)` (Ll), `Caseless` otherwise (Lt, Lm, Lo).
  Two letters have *the same case* when these values are equal —
  `Caseless` matches only `Caseless` (D21: "both Lu, both Ll, or both
  caseless").
- **Skeleton** — `skeleton(c) = confusable_target(c).unwrap_or(c)`.
- **Twin** — `twin(c, T)` for `T` in LCG, defined by rules T1–T4:
  - **T1 (the candidates; D42 first).** `k = skeleton(c)`; `members =
    confusables_with(k, T)` filtered to the `m` with `is_letter(m)`,
    *the same case as `c`*, and `decomposition(m).is_none()` (D42: a
    letter NFKC would rewrite is never a replacement, and it is removed
    *before* any choice below). `confusables_with` returns the members
    in ascending order (E1-1 §4.9).
  - **T2 (D21 step 1, the prototype).** If `members` contains `k`, the
    twin is `k`.
  - **T3 (D21 step 2, the lowest code point).** Otherwise, if `members`
    is not empty, the twin is its **first** element — the lowest code
    point, which in 18.0.0 is always the letter in everyday use (§3.6).
  - **T4 (D21 step 3).** Otherwise there is no twin, and no finding.
- **Script of a word, `S(w)`** — count the voting letters by
  `script_of`. A unique maximum is the word's script. Two or more
  scripts sharing the maximum make the word **tied**, with that set of
  scripts as its *tie set*. A word with no voting letter has no script.
- **Resolved letter** — for a word with script `S ∈ LCG` and a voting
  letter `x`: `ρ(x) = x` when `script_of(x) == S`; `twin(x, S)` when
  `script_of(x) ∈ LCG`; none otherwise.
- **Fully resolved** — `S(w) ∈ LCG` and `ρ(x)` exists for every voting
  letter `x` of `w`.
- **Redrawable into `T`** (`T ∈ LCG`, `T ≠ S(w)`) — fully resolved, and
  `twin(ρ(x), T)` exists for **every** voting letter `x`.
  **Redrawable** — redrawable into at least one `T`.
- **Basis** — for every word that has a script (not tied, not
  scriptless) and is **not** redrawable, credit its number of voting
  letters to `S(w)`.
- **Paragraph script `S_p`** — the script with the unique largest basis
  credit; none when the basis is empty or its maximum is shared.
- **Settled tie** — a tied word whose tie set contains `S_p` takes
  `S(w) = S_p`; a tied word whose tie set does not contain `S_p` keeps
  no script.
- **Shares** — `total` = the number of voting letters in the paragraph
  (every word, tied or not); `credit[X]` = the voting letters of the
  words whose script, after settling ties, is `X`. The share of `X` is
  under 10 % when `credit[X] * 10 < total` (integers, no floats).

### 4.3 The algorithm

`pub(crate) fn hits(text: &str) -> Vec<Hit>`, for each paragraph in
order:

1. **Words.** Walk the paragraph's `char_indices`, offsetting each
   index by the paragraph's start so that every position is a byte
   offset into `text`. Build the words of §4.2, recording for each its
   voting letters as `(byte offset, char)`.
2. **Scripts.** For every word compute `S(w)`, or its tie set, or none.
3. **Redrawability.** For every word with `S(w) ∈ LCG`, compute "fully
   resolved" and "redrawable". A word whose script is not in LCG is
   never redrawable.
4. **Basis and `S_p`.** As §4.2.
5. **Settle ties.** As §4.2.
6. **Shares.** As §4.2.
7. **Per word**, for every word whose `S(w)` (after step 5) is in LCG:
   1. **Whole-word redraw** when *all* of:
      - **W1** `S_p` exists, `S_p ∈ LCG`, `S_p ≠ S(w)`;
      - **W2** the word is fully resolved;
      - **W3** `twin(ρ(x), S_p)` exists for every voting letter `x`;
      - **W4** `credit[S(w)] * 10 < total` (A §5.4's 10 %);
      - **W5** every voting letter `x` with `script_of(x) ≠ S_p` has
        `confusable_target(ρ(x)) == Some(k)` with `script_of(k) == S_p`
        — the word is spelled in letters that confusables.txt lists as
        imitations of the paragraph's letters (D40; §4.5 H6).
      Then every voting letter `x` with `script_of(x) ≠ S_p` is a hit
      with replacement `twin(ρ(x), S_p)`; letters already of `S_p` are
      left alone. Go to the next word.
   2. **Otherwise, the mixed-word rule.** Every voting letter `x` with
      - **M1** `script_of(x) ∈ LCG` (A §5.4 condition 1 under D22), and
      - **M2** `script_of(x) ≠ S(w)` (condition 2, "the word mixes
        scripts"), and
      - **M3** `twin(x, S(w))` is `Some(r)` (condition 3 under D21)
      is a hit with replacement `r`.
8. Every hit is `Hit { at, c: x, class: UnicodeClass::Homoglyph,
   confidence: UnicodeClass::Homoglyph.max_confidence() /* Probable */,
   kept_by_context: false, replacement: Some(r) }`.
9. Return all hits of all paragraphs in increasing `at` (they are
   produced in that order if words are visited in order and letters in
   order within a word; `debug_assert!` it).

The function is O(n): every letter is looked up a bounded number of
times (`confusable_target` and `confusables_with` are binary searches,
E1-1 §4.9). It allocates one `Vec` of words per paragraph. Nothing in it
depends on `HashMap` iteration order; count scripts in a small `Vec` in
first-seen order.

### 4.4 Every rule, worked in code points

Byte offsets are into the example text. Every letter not spelled out is
ASCII in the English examples and Cyrillic (U+0400–U+04FF) in the
Russian ones; the tests assert that before asserting anything else.

**The mixed-word rule, Cyrillic in Latin (M1–M3, T2).** `pаy` =
U+0070 U+0430 U+0079. One word; voting letters Latin 2, Cyrillic 1 →
`S(w) = Latin`. U+0430: M1 Cyrillic ∈ LCG; M2 Cyrillic ≠ Latin;
skeleton U+0061; `confusables_with(U+0061, Latin)` = [U+0061, U+0251,
U+AB64, U+FF41, U+1DF5A], all lower case; T1 removes U+FF41 (`<wide>
U+0061`); the rest contains the skeleton → T2 → U+0061. Hit `{ at: 1,
c: U+0430, replacement: U+0061 }`. Built from confusables *sources*
only (without the prototype E1-1's reverse index includes), the answer
would be U+0251 LATIN SMALL LETTER ALPHA.

**Latin in Cyrillic (T3).** `пaрк` = U+043F U+0061 U+0440 U+043A.
`S(w) = Cyrillic` (3 to 1). U+0061 is a prototype: skeleton U+0061;
`confusables_with(U+0061, Cyrillic)` = [U+0430] → T3 → U+0430.
Hit `{ at: 2, c: U+0061, replacement: U+0430 }`.

**Upper case (T1, T2).** `Аpple` = U+0410 U+0070 U+0070 U+006C U+0065.
`S(w) = Latin`. U+0410 → skeleton U+0041; members [U+0041, U+FF21,
U+1DF6A], all upper case; T1 removes U+FF21; T2 → U+0041. Hit at 0.

**Case is matched on letters (T1).** `Ьob` = U+042C U+006F U+0062.
U+042C is upper case; its skeleton U+0062 is lower case, so T2 does not
apply; `confusables_with(U+0062, Latin)` = [U+0062, U+0184, U+FF42];
T1 keeps the upper-case, undecomposed [U+0184] → T3 → U+0184 LATIN
CAPITAL LETTER TONE SIX. Hit at 0. `кey` = U+043A U+0065 U+0079 →
skeleton U+0138 → T2 → U+0138, not U+006B.

**The prototype before the lowest code point (T2).** `фox` = U+0444
U+006F U+0078. U+0444 → skeleton U+0278; members [U+0239, U+0278];
T2 → U+0278 LATIN SMALL LETTER PHI, although U+0239 LATIN SMALL LETTER
QP DIGRAPH is lower. Hit at 0.

**Several twins: the everyday letter (T3).** `дoм` = U+0434 U+006F
U+043C. `S(w) = Cyrillic`; `confusables_with(U+006F, Cyrillic)` =
[U+043E, U+1C82], both lower case, neither decomposed → T3 → U+043E
CYRILLIC SMALL LETTER O, not U+1C82 CYRILLIC SMALL LETTER NARROW O. Hit
`{ at: 2, c: U+006F, replacement: U+043E }`. `мeсто` = U+043C U+0065
U+0441 U+0442 U+043E: [U+0435, U+04BD] → U+0435, hit at 2. `λoγος` =
U+03BB U+006F U+03B3 U+03BF U+03C2: [U+03BF, U+03C3] → U+03BF, hit at
2. `ΑΡΗY` = U+0391 U+03A1 U+0397 U+0059: [U+03A5, U+03D2]; T1 removes
U+03D2 (`<compat> U+03A5`) → U+03A5, hit at 6.

**No twin at all (T4).** `даbа` = U+0434 U+0430 U+0062 U+0430: the only
Cyrillic member for U+0062 is U+042C, upper case → T1 leaves nothing →
no hit. `дog` = U+0434 U+006F U+0067: U+0434 has no skeleton class with
a Latin letter → no hit (the word is Latin, 2 to 1, and U+0434 is the
minority letter).

**No compatibility twin (T1, D42).** `κόcμος` = U+03BA U+03CC U+0063
U+03BC U+03BF U+03C2. `S(w) = Greek` (5 to 1); `confusables_with(U+0063,
Greek)` = [U+03F2], whose decomposition is `<compat> U+03C2` → T1
leaves nothing → no hit.

**Russian prose (M2).** "Вчера вечером мы с братом долго сидели у окна
и смотрели, как над рекой поднимается туман. Он был такой густой, что
ни одного огонька на том берегу не было видно." — 129 letters, every
one Cyrillic. Every word has `S(w) = Cyrillic`; no letter passes M2; the
basis gives `S_p = Cyrillic`, so W1 fails for every word. Zero hits.
Delete M2 and every Cyrillic letter that has a Cyrillic twin (its own
skeleton class, e.g. U+0430 → T3 → U+0430) becomes a hit that replaces
itself (126 of the 129).

**Whole word (W1–W5).** "Please рау the invoice before the end of the
month." with рау = U+0440 U+0430 U+0443 at bytes 7, 9, 11. Words: nine
ASCII words (38 voting letters) and рау (Cyrillic, fully resolved,
redrawable into Latin: U+0440→U+0070, U+0430→U+0061, U+0443→U+0079 by
T2). Of the ASCII words only "Please" is redrawable (into Cyrillic:
U+0420, U+04CF, U+0435, U+0430, U+0455, U+0435); `t`, `n`, `b`, `f`,
`m` have no Cyrillic or Greek twin. Basis Latin 32 → `S_p = Latin`. рау: W1 ✓; W2 ✓;
W3 ✓; W4: `credit[Cyrillic] = 3`, `3 * 10 = 30 < 41` ✓; W5: the three
letters are sources whose targets U+0070, U+0061, U+0079 are Latin ✓.
Hits at 7 (→U+0070), 9 (→U+0061), 11 (→U+0079).

**An English quotation in Russian (W4, W5).** "Тариф называется «pay
as you go», и он нам подходит." Latin words pay, as, you, go (10
letters); Cyrillic 29; `total = 39`. Redrawable: Тариф, и, он, нам
(Cyrillic, into Latin), pay and as (Latin, into Cyrillic: U+0070 →
U+0440, U+0061 → U+0430, U+0079 → U+0443, U+0073 → U+0455) and you
(into Greek only — `u` has no Cyrillic twin); go is not. Basis Cyrillic
18, Latin 2 → `S_p = Cyrillic`. pay and as: W4 fails (`10 * 10 = 100 ≥
39`) and W5 fails (U+0070, U+0061, U+0079, U+0073 are prototypes, not
imitations). Zero hits. With W4 and W5 both deleted: five hits — bytes
34, 35, 36 (pay → U+0440 U+0430 U+0443) and 38, 39 (as → U+0430
U+0455); you stays, W3 finds no Cyrillic twin for `u`.

**A Russian quotation in English (W4 alone).** "The sign said «все в
сад» and nothing else." Cyrillic все (3), в (1), сад (3); Latin 25;
`total = 32`. все and в are redrawable into Latin and are imitations
(U+0432→U+0299, U+0441→U+0063, U+0435→U+0065 — W5 ✓). The, said and
else are redrawable into Cyrillic and leave the basis too. Basis Latin
14 (sign, and, nothing), Cyrillic 3 (сад; U+0434 has no Latin twin) →
`S_p = Latin`. W4:
`credit[Cyrillic] = 7`, `70 ≥ 32` → not redrawn. Zero hits. Without W4:
four hits (bytes 16, 18, 20, 23).

**A Latin word in Russian prose (W5).** "Вчера я купил подержанный BMW
у соседа, потому что старый автомобиль сломался." BMW = U+0042 U+004D
U+0057 (65 letters, Latin 3). BMW is redrawable into Cyrillic (U+0412,
U+041C, U+051C), W1–W4 hold (`30 < 65`), but B, M, W are prototypes —
`confusable_target` is `None` for each — so W5 fails. Zero hits. Without
W5: hits at bytes 48, 49, 50. This is A §5.4's "but not the reverse:
`pay` in a Russian paragraph is an English word", made mechanical: W4
alone protects a short paragraph or a quotation, never one acronym in a
long paragraph. Same for "В транскрипции этот гласный обозначается
знаком ɑ, и он звучит долго и открыто." with ɑ = U+0251 at byte 90: it
*is* a source, but of a Latin prototype (U+0061), not of a Cyrillic one —
W5 fails; a W5 weakened to "is any source" would replace it by U+0430.

**A tie settled by the paragraph.** "Welcome tо the team" with
tо = U+0074 U+043E at byte 8: tied {Latin, Cyrillic}. Basis: Welcome,
the, team (Latin 14; none is redrawable — `m` and `t` have no Cyrillic
or Greek twin) →
`S_p = Latin`, in the tie set → `S(w) = Latin` → U+043E → T2 → U+006F.
Hit at 9.

**A tie the paragraph cannot settle.** "Καλημέρα κόσμε, аb" with
аb = U+0430 U+0062 at byte 29: tied {Latin, Cyrillic}; `S_p = Greek`,
not in the tie set → no script → zero hits. (A §5.4's literal "on a
tie, the paragraph's script" would make it a Greek word and replace
U+0430 by U+03B1.)

**A mixed word in a foreign paragraph (resolved letters).** "Ask рарa
about the unpaid invoice before the end of the month." with рарa =
U+0440 U+0430 U+0440 U+0061 at bytes 4, 6, 8, 10. `S(w) = Cyrillic`
(3 to 1); ρ(U+0061) = U+0430, so the word is fully resolved and
redrawable into Latin. Basis Latin 46 → `S_p = Latin`; W4 `40 < 50`; W5
✓. Hits at 4 (→U+0070), 6 (→U+0061), 8 (→U+0070); the Latin `a` at 10 is
left alone. Judging the word on its raw letters instead (it is not
single-script) would give the mixed-word hit U+0061 → U+0430 at 10 now
and the redraw only in the next `clean` — not idempotent.

**A letter its word outvotes counts for the word's script.** "Please
рау the pаypаl invoice today, Anna." — рау = U+0440 U+0430 U+0443 at
7, 9, 11; pаypаl = U+0070 U+0430 U+0079 U+0070 U+0430 U+006C with the
two U+0430 at 19 and 23. `total = 34`; pаypаl is a Latin word, so its
six letters are credited to Latin; `credit[Cyrillic] = 3` → `30 < 34`
→ рау is redrawn. Five hits: 7, 9, 11 (redraw) and 19, 23 (mixed,
→U+0061). Counting letters by their own script (`credit[Cyrillic] = 5`,
`50 ≥ 34`) would leave рау for the next `clean` to redraw.

**Invisible characters do not split a word.** `p`, U+200B, U+0430, `y`
→ one word "pаy"; hit at byte 4. `p`, U+0430, U+00AD, `y` → hit at 1.

**A paragraph ends at U+000A.** "Please рау (U+0440 U+0430 U+0443) the invoice before the end
of the month." + U+000A + "Привет, как дела? Всё хорошо, спасибо." —
the first paragraph alone decides: three hits as above. Judged as one
paragraph (Cyrillic credit 32 of 70) рау would survive.

### 4.5 Where this document is more precise than A §5.4 — and why

Each point is required by data (§3.6) or by the idempotence gate of A
§5.3. "Corpus" is the generated corpus of §5.3 replayed through a model
of §4 (10 000 strings, seed `0x5749_5045_4D41_524B`): the count is how
many strings stop being idempotent (`clean(clean(x)) ≠ clean(x)`,
`aggressive`) when the point is removed. The Rust suite will produce its
own counts; they must be non-zero. The counts are under revised D21
(the README's D41 row quotes the first model's counts, 1 322 / 114 / 76
/ 159 / 27). H1–H5 are **D41**, H6 is **D40**, H7 is **D42**; H8 and H9
are definitions.

| # | A §5.4 says | this document | evidence |
|---|---|---|---|
| H1 (D41) | "a maximal run of `L*`, `M*`, `Nd`" | transparent characters inside a run do not end the word (§4.2) | a U+200B inside "pаy" (U+0070 U+0430 U+0079) otherwise hides the mixed word until `clean` removes it, and the second `clean` finds it — corpus 1 398 |
| H2 (D41) | "on a tie, the paragraph's majority" | the tie goes to `S_p` only when `S_p` is in the tie set; else no script | a word of Latin and Cyrillic letters is not Greek (`a_tie_is_never_settled_by_a_script_the_word_does_not_have`); the literal rule lets a settled tie move the majority it was settled by — corpus 118 |
| H3 (D41) | "the paragraph's script from the letters outside whole-redrawable words" | the basis credits **words** to their script (a mixed word counts entirely for its majority), and tied words are left out | replacing a mixed word's minority letters must not move the paragraph's script — corpus 67 |
| H4 (D41) | "its script makes up less than 10 % of the paragraph's letters" | `credit[S(w)] * 10 < total`, credits by word as in H3 | `a_letter_its_word_outvotes_does_not_count_for_its_script`; corpus 153 |
| H5 (D41) | whole-word case: "all letters of `w` are of one script" | judged on the **resolved** letters ρ(x): a mixed word whose minority letters resolve is a whole-word candidate | `a_mixed_word_in_a_foreign_paragraph_is_redrawn_in_one_pass`; corpus 30 |
| H6 (D40) | "but not the reverse: `pay` in a Russian paragraph is an English word" (stated intent; mechanism = H3 + H4) | W5: a word is redrawn only when its letters are confusables *sources* whose prototypes are letters of `S_p` | W4 alone fails the stated intent for any paragraph over ~30 letters: BMW, HTTP, SSH, Java and the IPA ɑ in long Russian paragraphs would be replaced and (D4) flag the text suspicious — `a_latin_word_in_russian_prose_is_not_redrawn` |
| H7 (D42) | the replacement is "the letter" | T1: never a letter with a decomposition, removed before D21 chooses | U+03F2/U+03F9 are compatibility characters; E1-3 §4.4.8 assumes E1-4's letters are NFKC-stable — `a_replacement_is_never_a_compatibility_character` |
| H8 | "a character `c`" in the word | only voting letters are findings; digits and marks keep the word whole but are never findings | A §4.1 and OV §3.1 describe the class as letters; confusables.txt maps U+FF10 to U+004F and U+FF11 to U+006C, and a digit must never become a letter. Under M1 and D21's case rule no digit or mark can resolve, so H8 is a definition with no test of its own (§5.1 note) |
| H9 | "абзац" (undefined in §5.4) | the A §4.2 paragraph: between U+000A | `a_paragraph_ends_at_a_line_feed` |

Not added, deliberately: no "mark veto". A replacement followed by a
combining mark it composes with (U+0076 U+006F U+0069 U+006C U+0430 U+0300 → the same with U+0061 for U+0430)
is replaced; with `nfkc`, D26's next round composes it to U+00E0, and
`a_replacement_that_meets_its_accent_is_composed_by_nfkc` pins that.
Refusing such letters would only hide a mixed word from the report.

### 4.6 Edge cases

| case | behaviour | why |
|---|---|---|
| single-letter words (Russian а, и, в, с, у, о, к, я; English a, I) | a single-letter word is a word: it has a script and can be redrawable; in its own language's paragraph `S(w) = S_p`, so never redrawn; inside a foreign quotation W4 protects it | `a_russian_quote_in_english_is_not_redrawn` (в) |
| digits inside words (`Windоws10`, U+043E at byte 4) | digits keep the word whole and do not vote: hit U+043E → U+006F at 4; `Win10` has no finding | H8 |
| combining marks (`e` + U+0301, U+0430 + U+0300) | marks keep the word whole, never vote (`Inherited`), never become findings, and stay where they are when the letter before them is replaced | H8; D26 composes with `nfkc` |
| precomposed letters (U+00E9, U+0451 ё) | matched as themselves: they have no single-code-point skeleton in the table, so they never resolve; UTS #39 would decompose first, Layer A does not (E1-1 filter) | a Cyrillic ё in a Latin word is not found; accepted |
| words at paragraph boundaries | a word never crosses U+000A; each paragraph has its own basis, `S_p` and shares | H9 |
| upper/lower case | §4.4 `Аpple` (U+0410 first; T2 upper), `Ьob` (U+042C first; T1 case on letters), `I` → U+0406 in a Cyrillic word (lowest of U+0406, U+04C0), `b` → none in a Cyrillic word (its only twin U+042C is upper case) | D21, D42 |
| Greek final sigma U+03C2 | no twin in Latin or Cyrillic; a Greek word with ς is never redrawable; a ς in a Latin word is no finding. Medial σ U+03C3 → U+006F in a Latin word (`rσom` = U+0072 U+03C3 U+006F U+006D, hit at 1) | §3.6 |
| fullwidth Latin in a Japanese paragraph | `ＮＨＫ` (U+FF2E U+FF28 U+FF2B) is a Latin word: no letter passes M2; `S_p` is Hiragana (not in LCG) so W1 fails; inside a run with kana (`ＮＨＫのアナウンサー…`) the word's script is Hiragana and nothing is examined | `fullwidth_in_japanese_is_typography` |
| fullwidth Latin in a Latin word (`pａy`) | same script — not a homoglyph; it is width, which the `nfkc` knob folds | M2 |
| fullwidth digits (`２０２６年`; `Ｗｉｎｄｏｗｓ１０` = U+FF37 U+FF49 U+FF4E U+FF44 U+FF4F U+FF57 U+FF53 U+FF11 U+FF10) | never findings: not letters (H8), `Common` (M1), and caseless against the case-carrying skeletons U+004F/U+006C that confusables.txt gives them (D21). They keep the word whole | H8, M1, D21 |
| halfwidth Katakana / Hangul (`ｶ`, U+FFDA) | never findings (M1, D22) | §3.6 |
| letters of other scripts in an LCG word (Armenian U+0578 in a Latin word) | not a finding (M1); the word is not fully resolved, so not redrawable | M1 |
| a word whose script is not LCG (Han, Hangul, Arabic, `Other`) | examined for nothing; credited to its script in the basis | step 7 |
| `Common`/`Inherited` letters (U+30FC, U+02B9) | in the word, not voting | §4.2 |
| empty text, a paragraph with no words | no hits | — |
| the same letter in two words | each occurrence is its own hit; E1-3 aggregates them into one row per codepoint (D6) | — |

### 4.7 `crates/wipemark-core/src/homoglyph.rs`

New file; everything in it is `pub(crate)` or private.

```rust
//! Homoglyphs — a letter of one script inside a word of another, where
//! it looks the same (A §5.4, S1.6).
//!
//! Detection always runs (D3): without `Options::aggressive` a homoglyph
//! is reported in `kept` at `Probable`, with it the letter is replaced.
//!
//! # The rule
//!
//! confusables.txt (UTS #39) gives every look-alike a *skeleton*; two
//! letters are confusable when their skeletons are equal. Used in both
//! directions: a Cyrillic letter in a Latin word, a Latin letter in a
//! Cyrillic word. A letter is a finding only when **its word mixes
//! scripts** (M1–M3) or when **a whole word is drawn in look-alikes of
//! the paragraph's script** and is a small part of that paragraph
//! (W1–W5; W5 is D40). A Cyrillic word in a Russian paragraph is never a finding:
//! nineteen lower-case Russian letters have Latin twins, and a detector
//! that looked at letters alone would rewrite Russian prose letter by
//! letter.
//!
//! # The replacement (D21, D42)
//!
//! Candidates are the letters of the word's script with the same
//! skeleton and case and no decomposition (D42 — never a letter NFKC
//! would rewrite; removed before any choice). The prototype when it is
//! one of them; else the lowest code point, which in UCD 18.0.0 is the
//! everyday letter (a Latin `o` in a Russian word becomes U+043E, not
//! U+1C82); else no finding. The replacement is what the eye sees, not
//! what a keyboard would have typed: U+043A becomes U+0138, not `k`.
//!
//! # Idempotence (A §5.3)
//!
//! One pass is a fixpoint of itself: transparent characters never split
//! a word, so removing them changes no word; a replaced letter moves into
//! the script that already won its word (or into the paragraph's script,
//! for a redraw); the paragraph's script is computed only from words a
//! pass never changes, and can only gain credit; a script's share only
//! falls after a pass, and every word of that script that could be
//! redrawn was redrawn in the same pass. `homoglyph_replacement_is_idempotent`
//! is the gate, over a generated corpus.
//!
//! # Where it is more precise than A §5.4
//!
//! H1–H9 in `docs/plan/E1-4-homoglyphs.md` §4.5 (H1–H5 = D41, H6 = D40,
//! H7 = D42), each with the test that goes red without it.

pub(crate) fn hits(text: &str) -> Vec<Hit>;
```

Suggested private items (names are yours; behaviour is §4.2–4.3):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Case { Upper, Lower, Caseless }

fn case_of(c: char) -> Case;
fn skeleton(c: char) -> char;                       // confusable_target(c).unwrap_or(c)
/// The letter of `script` that `c` would be replaced by — T1 (D42) and D21's three steps.
fn twin(c: char, script: Script) -> Option<char>;
fn is_transparent(c: char) -> bool;                 // class_of(c) is Some and not ExoticSpace
fn is_lcg(script: Script) -> bool;

struct Word { letters: Vec<(usize, char)>, script: WordScript }   // voting letters, byte offsets into `text`
enum WordScript { None, One(Script), Tied(Vec<Script>) }

fn paragraph_hits(text: &str, start: usize, end: usize, out: &mut Vec<Hit>);
```

Doc comments the code must carry:

- on `twin`: T1 (D42, before any choice) and D21's three steps, with
  the examples U+0430 → U+0061 (prototype), U+0444 → U+0278 (prototype
  beats the lower U+0239), U+006F → U+043E in Cyrillic (lowest of
  U+043E, U+1C82), U+0059 → U+03A5 in Greek (U+03D2 vetoed), U+0063 →
  none in Greek (U+03F2 is `<compat>`), U+0062 → none in Cyrillic (the
  only member, U+042C, is upper case);
- on the transparency check (H1), the tie settlement (H2), the basis
  (H3), the share (H4), ρ (H5) — D41 — and W5 (H6, D40): one or two
  sentences each, naming the test that goes red without it;
- on the `S(w) ∈ LCG` restriction: "a Han, Arabic or Hangul word is never
  examined; the reverse index holds only Latin, Cyrillic and Greek
  letters, and D22 removed the width forms that would have reached it".

### 4.8 The seam — `scrub.rs` and `lib.rs`

In `crates/wipemark-core/src/scrub.rs`, the body of `collect_hits`
becomes exactly:

```rust
    merge_in_source_order(context::hits(text), homoglyph::hits(text))
```

with E1-3's comment above it replaced by "Homoglyphs run whatever the
`Options` say (D3); `aggressive` only changes the action." `_options`
stays unused (E1-3 P9). In `src/lib.rs`, add `mod homoglyph; // E1-4` in
the module list, private, beside `mod scrub;`. Nothing else in E1-3's
files changes. If E1-3 left the seam in a different shape, make the one
change that merges `homoglyph::hits(text)` into the source-ordered list
and say so in your report.

The merge's `debug_assert!` that the two lists never claim one code
point holds by construction: every homoglyph hit is a letter, and
`class_of` of a letter is `None`.

**E1-1's `dead_code` allowance.** E1-3 §4.3.1 narrowed (or kept) the
`#[cfg_attr(not(test), allow(dead_code))]` that E1-1 put on `tables.rs`
and `script.rs`, and listed in its report the items still without a
caller — "the confusable lookups wait for E1-4". This document is their
first caller (`confusable_target`, `confusables_with`, and
`is_uppercase_letter`/`is_lowercase_letter` unless E1-5 landed first).
Delete the per-item allowance from each item `homoglyph.rs` now calls
and keep `cargo clippy -p wipemark-core --all-targets -- -D warnings`
clean; if the allowance is still module-wide, leave it (E1-7 removes
it) and say so in the report. These are the only edits to `tables.rs`.

### 4.9 Fixtures

Two new files under `fixtures/text/`, byte-exact (`fixtures/text/**
-text`, D25), written with `printf` so no editor touches them:

**`homoglyph.txt`** (94 bytes; four paragraphs):

```sh
printf 'Please p\xd0\xb0y the invoice.\n\xd0\xbfa\xd1\x80\xd0\xba\nPlease \xd1\x80\xd0\xb0\xd1\x83 the invoice before the end of the month.\n\xd0\xb4o\xd0\xbc\n' > fixtures/text/homoglyph.txt
```

i.e. `Please p` U+0430 `y the invoice.` LF, U+043F `a` U+0440 U+043A LF,
`Please ` U+0440 U+0430 U+0443 ` the invoice before the end of the
month.` LF, U+0434 `o` U+043C LF. `shasum -a 256` must print
`1f1edfdbdfa159ce1f2e43d8608a1c70d2f0d6220ea758d265975a02319ff2d5`.
Claims, under `Options::default()`:
`clean` returns it byte-identical; `findings` empty; `kept` = five rows,
all `homoglyph` · `Probable`, in codepoint order: U+0061 [27], U+006F
[90], U+0430 [8, 42], U+0440 [40], U+0443 [44]; `suspicious` true (D4).
Under `aggressive`: text
`"Please pay the invoice.\nпарк\nPlease pay the invoice before the end of the month.\nдом\n"`
— the last line U+0434 U+043E U+043C (the Latin `o` became U+043E, D21
step 2) — 92 bytes; the same five rows in `findings`, `kept` empty,
`normalized == [(Homoglyph, 6)]`.

**`survive-homoglyph-prose.txt`** — five paragraphs, each ending in LF:
the Russian prose of §4.4; "Тариф называется «pay as you go», и он нам
подходит."; "Вчера я купил подержанный BMW у соседа, потому что старый
автомобиль сломался."; "В транскрипции этот гласный обозначается знаком
ɑ, и он звучит долго и открыто." (ɑ = U+0251); "The sign said «все в
сад» and nothing else.". Every Russian letter is Cyrillic
(U+0400–U+04FF), every English letter ASCII; the only other non-ASCII
characters are U+00AB, U+00BB and the one U+0251; the separators are
U+0020, the line ends U+000A, no trailing space. 715 bytes; `shasum -a
256` must print
`0fd9941239096613c1d8ca498ff526c18f8be0e6f38f4f9aeb5afc4ff94ab40b` —
create it however you like (a short Python or Rust snippet with `\u`
escapes), check the hash, commit only the file. Claim: under all 16
`Options`, `clean` returns it byte-identical, `findings` and `kept`
empty, `suspicious` false. It is NFKC-stable (U+0451 and U+0251 are),
which E1-3's `the_survivors_survive_every_option` requires.

In `crates/wipemark-core/tests/fixtures.rs`: add both to `FIXTURES` with
the claims above, empty `NOT_YET` (keep the constant, now `&[]`, or
delete it and its use — either way `every_class_has_a_fixture` must now
cover `Homoglyph`), and add the `aggressive` claim of `homoglyph.txt` to
`tests/homoglyphs.rs` (§5.2). In `fixtures/README.md`, delete the clause
"`homoglyph.txt` arrives with E1-4".

### 4.10 `docs/architecture/layer-a.md` — section "Homoglyphs"

After "Scrubber, report and NFKC" (or wherever E1-1's skeleton puts it),
for someone touching this code next year: what a homoglyph is to this
product and why detection always runs (D3) but replacement needs
`aggressive`; the two cases (mixed word, whole word) in a paragraph each,
with `pаy` (U+0070 U+0430 U+0079) and `рау` (U+0440 U+0430 U+0443)
spelled in code points; why Russian prose is safe
(M2 and W5 = D40, with the 19-letter fact); the replacement rule (D21
revised, D42) with its twelve lowest-code-point choices listed and the
seven letters where the prototype beats a lower code point; the
precision points H1–H9 as a table (point · decision · why · test); what is deliberately not done (no mark
veto; no precomposed decomposition; no dictionary; no script outside
LCG); where the code is (`src/homoglyph.rs`, the seam in `scrub.rs`);
the gates (§5 names). Then add a row for the section to nothing else —
E1-1 already listed it in `docs/README.md`.

## §5 Tests

Mixed-script inputs are written in Rust with `\u{…}` escapes for every
non-ASCII letter of a mixed word, never as literal confusables; pure
Russian or Japanese prose may be literal, and each such test first
asserts that every letter of its prose is of the script it claims
(`script_of`), so an editor that "fixed" a letter fails the test for the
right reason.

### 5.1 Unit tests — `crates/wipemark-core/src/homoglyph.rs` (`mod tests`, D24)

`h(text)` below is `hits(text)` reduced to `(at, c, replacement)`.

| test | input → expected | mutation that paints it red |
|---|---|---|
| `russian_prose_is_not_a_homoglyph_attack` | the Russian prose of §4.4 (assert all 129 letters Cyrillic) → `h == []`; also each Russian paragraph of `survive-homoglyph-prose.txt` → `[]` | delete M2 (the `script_of(x) == S(w) → skip` check): every Cyrillic letter with a Cyrillic twin becomes a self-replacing hit |
| `a_cyrillic_letter_in_a_latin_word_is` | `"p\u{430}y"` → `[(1, '\u{430}', 'a')]`; `"Please p\u{430}y the invoice."` → `[(8, '\u{430}', 'a')]`; every hit has class `Homoglyph`, confidence `Probable`, `kept_by_context == false` | build T1's candidates from confusables *sources* only, dropping the prototype E1-1's index includes: U+0251 |
| `a_latin_letter_in_a_cyrillic_word_is` | `"\u{43F}a\u{440}\u{43A}"` → `[(2, 'a', '\u{430}')]` | use only T2 (no T3): U+0061 is not Cyrillic, no hit |
| `an_uppercase_cyrillic_letter_in_a_latin_word_is` | `"\u{410}pple"` → `[(0, '\u{410}', 'A')]` | sources only, as above: U+1DF6A (U+FF21 is vetoed by D42); the case rule itself is painted by the next test |
| `case_is_matched_on_the_letters_not_on_the_skeleton` | `"\u{42C}ob"` → `[(0, '\u{42C}', '\u{184}')]`; `"\u{43A}ey"` → `[(0, '\u{43A}', '\u{138}')]` | drop the case filter in T1: U+042C gets U+0062 by T2 |
| `a_latin_o_in_a_cyrillic_word_becomes_the_everyday_o` (D21 step 2) | `"\u{434}o\u{43C}"` → `[(2, 'o', '\u{43E}')]`; `"\u{43C}e\u{441}\u{442}\u{43E}"` → `[(2, 'e', '\u{435}')]` | take the highest code point: U+1C82, U+04BD. Separately, accept a candidate only when it is the only one (the first D21): `[]` each |
| `a_latin_letter_in_a_greek_word_becomes_the_everyday_greek_letter` (D21 step 2, D42) | `"\u{3BB}o\u{3B3}\u{3BF}\u{3C2}"` → `[(2, 'o', '\u{3BF}')]`; `"\u{391}\u{3A1}\u{397}Y"` → `[(6, 'Y', '\u{3A5}')]` | take the highest code point: the first input gets U+03C3. The `Y` line stays green under that mutation because D42 has already removed U+03D2 (`<compat> U+03A5`); it pins D42's order and goes red only with D42 dropped **and** the highest chosen (U+03D2). Moving the veto after the choice changes none of the 488 resolvable pairs in 18.0.0 — record that in the report rather than inventing a red for it |
| `the_prototype_wins_over_a_lower_code_point` (D21 step 1) | `"\u{444}ox"` → `[(0, '\u{444}', '\u{278}')]` | skip T2: U+0239 LATIN SMALL LETTER QP DIGRAPH |
| `a_letter_with_no_twin_in_its_words_script_is_not_a_finding` (D21 step 3) | `"\u{434}\u{430}b\u{430}"` → `[]`; `"\u{434}og"` → `[]` | drop the case filter in T1: `[(4, 'b', '\u{42C}')]` for the first input |
| `fullwidth_in_japanese_is_typography` | `"昨日（ＮＨＫ）のニュースを見ました。ＮＨＫのアナウンサーが話していました。"` (ＮＨＫ = U+FF2E U+FF28 U+FF2B; parentheses U+FF08/U+FF09) → `[]`; `"２０２６年"` → `[]`; `"\u{FF37}\u{FF49}\u{FF4E}\u{FF44}\u{FF4F}\u{FF57}\u{FF53}\u{FF11}\u{FF10}"` → `[]` | delete M2: the stand-alone ＮＨＫ word gets three hits — byte 9 U+FF2E → U+004E, 12 U+FF28 → U+0048, 15 U+FF2B → U+004B (`S(w)` is Latin, and each letter's prototype is its ASCII capital) |
| `a_width_form_of_another_script_is_never_a_homoglyph` (D22) | `"ab\u{FFDA}cd"` → `[]` | restore the block clause in M1 **and** in E1-1's filter (`build.rs`) if E1-1 already dropped it: U+FFDA → U+A7F7 at 2. If only one of the two can be restored and the test stays green, delete the test and say so (§0.4) |
| `a_whole_word_redraw_is_found` | `"Please \u{440}\u{430}\u{443} the invoice before the end of the month."` → `[(7, '\u{440}', 'p'), (9, '\u{430}', 'a'), (11, '\u{443}', 'y')]` | delete step 7.1 (the redraw branch): `[]` |
| `an_english_quote_in_russian_is_not_redrawn` | `"Тариф называется «pay as you go», и он нам подходит."` → `[]` | delete W4 and W5 together: `[(34, 'p', '\u{440}'), (35, 'a', '\u{430}'), (36, 'y', '\u{443}'), (38, 'a', '\u{430}'), (39, 's', '\u{455}')]` (each alone is painted by the next two tests) |
| `a_russian_quote_in_english_is_not_redrawn` | `"The sign said «все в сад» and nothing else."` → `[]` | delete W4: hits at 16, 18, 20, 23 |
| `a_latin_word_in_russian_prose_is_not_redrawn` | the BMW sentence → `[]`; the ɑ sentence (U+0251 at byte 90) → `[]` | delete W5: BMW → hits at 48, 49, 50 (U+0412, U+041C, U+051C). Weaken W5 to "`confusable_target(ρ(x))` is `Some`": the ɑ sentence → `[(90, '\u{251}', '\u{430}')]` |
| `a_two_letter_word_is_settled_by_its_paragraph` | `"Welcome t\u{43E} the team"` → `[(9, '\u{43E}', 'o')]` | never settle ties: `[]` |
| `a_tie_is_never_settled_by_a_script_the_word_does_not_have` | `"Καλημέρα κόσμε, \u{430}b"` → `[]` | settle every tie to `S_p`: `[(29, '\u{430}', '\u{3B1}')]` |
| `a_mixed_word_in_a_foreign_paragraph_is_redrawn_in_one_pass` | `"Ask \u{440}\u{430}\u{440}a about the unpaid invoice before the end of the month."` → `[(4, '\u{440}', 'p'), (6, '\u{430}', 'a'), (8, '\u{440}', 'p')]` | judge W2/W3 on raw letters (require single-script): `[(10, 'a', '\u{430}')]` |
| `a_letter_its_word_outvotes_does_not_count_for_its_script` | `"Please \u{440}\u{430}\u{443} the p\u{430}yp\u{430}l invoice today, Anna."` → hits at 7, 9, 11, 19, 23 | credit letters by their own script in the shares: only 19 and 23 |
| `invisible_characters_inside_a_word_do_not_split_it` | `"p\u{200B}\u{430}y"` → `[(4, '\u{430}', 'a')]`; `"p\u{430}\u{AD}y"` → `[(1, '\u{430}', 'a')]` | let a transparent character end a word: both `[]` |
| `a_paragraph_ends_at_a_line_feed` | the redraw sentence + `"\nПривет, как дела? Всё хорошо, спасибо."` → hits at 7, 9, 11 | treat the whole text as one paragraph: `[]` |
| `digits_and_marks_keep_a_word_whole` | `"p\u{430}2y"` → `[(1, '\u{430}', 'a')]`; `"p\u{430}\u{301}y"` → `[(1, '\u{430}', 'a')]`; `"Wind\u{43E}ws10"` → `[(4, '\u{43E}', 'o')]`; `"voil\u{430}\u{300}"` → `[(4, '\u{430}', 'a')]` (the mark stays after the replaced letter; D26 composes it under `nfkc`); `"Win10"` → `[]` | drop `Nd` from word characters: `"p\u{430}2y"` splits into the tied `p`+U+0430 and `y`, nothing settles the tie → `[]`. Separately drop `M*`: `"p\u{430}\u{301}y"` → `[]` |
| `the_twin_of_a_letter_is_pinned` | `twin` over the table below → as listed | skip T2 (the U+0444 row), highest instead of lowest (the U+006F, U+0065, U+0079, U+0049 rows), unique-only (the same rows → none), drop D42 from T1 (the U+0063 → Greek row gets U+03F2), drop the case filter (the U+042C and U+0062 rows) — each turns at least one row |
| `every_twin_is_a_stable_letter_of_the_same_case` | for every `c` in `0..=0x10FFFF` with `is_letter(c)` and `script_of(c) ∈ LCG`, and every other `T ∈ LCG`: `twin(c, T) == Some(r)` ⇒ `is_letter(r)`, `script_of(r) == T`, `case_of(r) == case_of(c)`, `decomposition(r).is_none()`; and at least 450 such pairs resolve (a floor, so an emptied table cannot pass; the model counts 488 — record the Rust count) | drop D42 from T1: U+0063 → U+03F2 violates it |
| `a_replacement_is_never_a_compatibility_character` (D42) | `"\u{3BA}\u{3CC}c\u{3BC}\u{3BF}\u{3C2}"` → `[]` | drop D42 from T1: `[(4, 'c', '\u{3F2}')]` |
| `hits_are_well_formed` | over every input of this table: `at` strictly increasing; `text.is_char_boundary(at)`; `text[at..].starts_with(c)`; `replacement != Some(c)` | record `char_indices().enumerate()` indices instead of byte offsets |

H8 ("only voting letters are findings") has no test of its own, on
purpose: no digit or mark has a Latin, Cyrillic or Greek script that
resolves — fullwidth digits are `Common` (M1), and even admitted they are
caseless against case-carrying skeletons (U+FF10 → U+004F, U+FF11 →
U+006C; D21's case rule gives no twin). A test for it would stay green
with H8 deleted, which §0.4 forbids. State H8 in the code as a
definition, not as a protection.

`the_twin_of_a_letter_is_pinned` — `(c, T) → twin`:

| c | T | twin | rule |
|---|---|---|---|
| U+0430 | Latin | U+0061 | T2 (members U+0061, U+0251, U+AB64, U+1DF5A after T1 removes U+FF41) |
| U+0410 | Latin | U+0041 | T2 |
| U+043A | Latin | U+0138 | T2 — not U+006B |
| U+0432 | Latin | U+0299 | T2 |
| U+0438 | Latin | U+1D0E | T2 |
| U+0444 | Latin | U+0278 | T2 — the prototype beats the lower U+0239 |
| U+042C | Latin | U+0184 | T1 case (U+0062 is lower case), T3 |
| U+0417 | Latin | U+01B7 | skeleton U+0033 (a digit, not a letter, so no T2); T3: lowest of U+01B7, U+021C, U+A76A, U+A7AB |
| U+0406 | Latin | U+0049 | skeleton U+006C is lower case; T3: lowest upper-case undecomposed of U+0049, U+0196, U+A7AE |
| U+0434 | Latin | none | T4 — no skeleton class with a Latin letter |
| U+0061 | Cyrillic | U+0430 | T3 (only member) |
| U+0041 | Cyrillic | U+0410 | T3 |
| U+006C | Cyrillic | U+04CF | T1 case (U+0406, U+04C0 are upper case), T3 |
| U+0049 | Cyrillic | U+0406 | T3, lowest of U+0406, U+04C0 |
| U+006F | Cyrillic | U+043E | T3, lowest of U+043E, U+1C82 |
| U+0065 | Cyrillic | U+0435 | T3, lowest of U+0435, U+04BD |
| U+0079 | Cyrillic | U+0443 | T3, lowest of U+0443, U+04AF |
| U+0062 | Cyrillic | none | T1 case leaves nothing (U+042C is upper case) → T4 |
| U+FF41 | Cyrillic | U+0430 | T3 — a fullwidth letter is a Latin letter |
| U+0251 | Cyrillic | U+0430 | T3 |
| U+006F | Greek | U+03BF | T3, lowest of U+03BF, U+03C3 |
| U+0070 | Greek | U+03C1 | T1 removes U+03F1 (`<compat>`); T3, lowest of U+03C1, U+03F8 |
| U+0059 | Greek | U+03A5 | T1 removes U+03D2 (`<compat> U+03A5`); T3 |
| U+0063 | Greek | none | T1 removes U+03F2 (`<compat>`), the only member → T4 |
| U+0043 | Greek | none | T1 removes U+03F9 → T4 |
| U+0049 | Greek | U+0399 | T1 case, T3 |
| U+03C3 | Latin | U+006F | T2 — medial sigma looks like `o` |
| U+03BF | Latin | U+006F | T2 |
| U+03C2 | Latin | none | final sigma has no skeleton class → T4 |
| U+03C0 | Cyrillic | U+043F | T3 |

### 5.2 Integration tests — `crates/wipemark-core/tests/homoglyphs.rs` (public API)

`A = Options { aggressive: true, ..Options::default() }`,
`AN = Options { aggressive: true, nfkc: true, ..Options::default() }`.

| test | input → expected | mutation |
|---|---|---|
| `nothing_is_replaced_without_aggressive` (D3, D4) | `t = "p\u{430}y"`: `clean(t, &Options::default())` → text `t` unchanged; `report.kept` == `[UnicodeFinding { codepoint: '\u{430}', class: Homoglyph, count: 1, positions: vec![1], confidence: Probable }]`; no `Homoglyph` row in `report.findings`; no `NormKind::Homoglyph` in `normalized`. `inspect(t, &Options::default())`: same row in `kept`, `suspicious == true`. `clean(t, &A)` → `"pay"`, the row in `findings`, `kept` empty, `normalized` contains `(Homoglyph, 1)` | gate the seam on `options.aggressive` (`if options.aggressive { homoglyph::hits(text) } else { Vec::new() }`): `kept` empty, `suspicious` false. Separately: make `action_for(Homoglyph)` answer `Replace` always → text changes without `aggressive` |
| `aggressive_replaces_and_counts_each_homoglyph` | `clean(include_str!("../../../fixtures/text/homoglyph.txt"), &A)` → the 92-byte text of §4.9; `findings` = the five rows of §4.9; `kept` empty; `normalized == [(Homoglyph, 6)]`; `inspect(same, &A).findings == report.findings` | write `hit.c` instead of `hit.replacement` in the redraw branch (only paragraph 3 stays Cyrillic) |
| `homoglyph_replacement_is_idempotent` | (a) every input of §5.1 and both fixtures, under `A` and `AN`: `let once = clean(t, &o); let twice = clean(&once.text, &o);` → `twice.text == once.text`, `twice.report.findings` has no `Homoglyph` row, `twice.report.normalized` has no non-zero `Homoglyph` count. (b) the corpus below, same assertion, under `A` and `AN` | each of H1, H2, H3, H4, H5 removed in turn: the corpus goes red under `A` (the model's counts under revised D21: H1 1 398, H2 118, H3 67, H4 153, H5 30 of 10 000 — D41) — record the Rust counts |
| `a_replacement_that_meets_its_accent_is_composed_by_nfkc` (D26) | `clean("\u{FB00}\u{430}\u{300}", &AN)` → text `"ff\u{E0}"` (U+0066 U+0066 U+00E0); cleaning that again changes nothing; `normalized` contains `(Homoglyph, 1)` (the hit appears only after NFKC split U+FB00, so it has no row — E1-3's counter-without-position case) | stop E1-3's rounds after one (the D26 mutation): text ends `a` U+0300 and the second `clean` composes it |

**The corpus** (`homoglyph_replacement_is_idempotent` (b)). E1-3's
`SplitMix64` (copied, as E1-3 §5.4.1 does), seed
`0x5749_5045_4D41_524B`, 10 000 strings. For each string:
`let n = rng.next() % 48;` then `n` times
`ALPHABET[(rng.next() % ALPHABET.len() as u64) as usize]` — one draw per
character, no other draws. `ALPHABET` (113 entries, repeats are weights):

```rust
const ALPHABET: &[char] = &[
    'a', 'o', 'e', 'p', 'c', 'y', 'x', 'i', 'A', 'o', 'B', 'E', 'H', 'K', 'M', 'O', 'P', 'T', 'X',
    'b', 'd', 'f', 't', 'l', 'n', 'm', 'r', 's',
    '\u{430}', '\u{43E}', '\u{435}', '\u{440}', '\u{441}', '\u{443}', '\u{445}', '\u{410}', '\u{412}',
    '\u{415}', '\u{41D}', '\u{41A}', '\u{41C}', '\u{41E}', '\u{420}', '\u{422}', '\u{425}', '\u{43B}',
    '\u{434}', '\u{436}', '\u{43F}', '\u{438}', '\u{442}', '\u{43C}', '\u{43D}', '\u{432}', '\u{433}',
    '\u{43A}', '\u{44C}',
    '\u{3B1}', '\u{3BF}', '\u{3C1}', '\u{3BD}', '\u{3B9}', '\u{391}', '\u{392}', '\u{395}', '\u{397}',
    '\u{39A}', '\u{39C}', '\u{39F}', '\u{3A1}', '\u{3A4}', '\u{3A7}', '\u{3C2}', '\u{3C3}', '\u{3BB}',
    '\u{3C0}', '\u{3C9}',
    '\u{42C}', '\u{FF29}', '\u{3F0}', '\u{3F1}', '\u{17F}', '\u{1D43}', '\u{AA}', '\u{301}', '\u{306}',
    '\u{300}', '\u{301}', '\u{FF2E}', '\u{FF41}', '\u{251}', '\u{138}', '\u{299}', '\u{1D0B}', '\u{308}',
    '2', '0', '\u{FF10}', ' ', ' ', ' ', '\n', '.', '\u{200B}', '\u{AD}', '\u{30FC}', '\u{306E}',
    '\u{6F22}', '\u{4AF}', '\u{1C82}', '\u{3F2}', '\u{456}', '\u{4CF}',
];
```

### 5.3 Fixture rows — `crates/wipemark-core/tests/fixtures.rs` (E1-3's file)

| test | change | mutation |
|---|---|---|
| `every_class_has_a_fixture` | `NOT_YET` empty; `homoglyph.txt` in `FIXTURES` | delete the `homoglyph.txt` row |
| `each_fixture_says_what_it_claims` | the `homoglyph.txt` row of §4.9 | delete M3 → `kept` empty |
| `the_survivors_survive_every_option` | the `survive-homoglyph-prose.txt` row | delete W5 → the BMW paragraph changes under `aggressive` |
| `clean_is_idempotent_on_every_fixture`, `inspect_and_clean_agree`, `every_position_names_its_code_point` | cover both new files with no change to the tests | — (E1-3's mutations) |
| `every_fixture_is_asserted` | both files listed | add an unlisted file |
| `counters_agree_with_the_rows_on_every_fixture` | extend E1-3's invariant by one clause: without `nfkc`, `normalized`'s `Homoglyph` count equals the summed `count` of the `homoglyph` rows in `findings` | count kept homoglyphs too (red on `homoglyph.txt` without `aggressive`) |

## §6 Acceptance criteria

- [ ] `homoglyph::hits` implements §4.2–4.3; every §5.1 test is green
      and its mutation was applied, seen red, and reverted (report table:
      protection · mutation · test that went red).
- [ ] `collect_hits` merges `homoglyph::hits(text)` (§4.8); D3 holds:
      `nothing_is_replaced_without_aggressive` green, red under the
      gated-seam mutation.
- [ ] `russian_prose_is_not_a_homoglyph_attack` and
      `fullwidth_in_japanese_is_typography` both go red when M2 is deleted
      (A §8's own mutation).
- [ ] `homoglyph_replacement_is_idempotent` green under `aggressive` and
      `aggressive + nfkc`; each of H1–H5 removed turns it red; the counts
      are in the report.
- [ ] E1-3's suites are still green with homoglyphs on:
      `clean_is_idempotent_on_every_fixture`,
      `clean_is_idempotent_on_a_generated_corpus` (E1-3's corpus contains
      `a`, `e`, `p`, U+0430, U+0440, U+0443, U+043F, U+03BF, U+03B1),
      `inspect_and_clean_agree`, `the_survivors_survive_every_option`.
- [ ] Both fixtures exist byte-exact (`git check-attr text
      fixtures/text/homoglyph.txt` says `unset`), are asserted, and
      `NOT_YET` is empty.
- [ ] `docs/architecture/layer-a.md` has "Homoglyphs" (§4.10).
- [ ] The six gates of §0.5 are green; `Cargo.lock` unchanged;
      `scripts/check-dep-direction.sh` still shows `wipemark-core` with
      no dependency.
- [ ] Report at `docs/plan/reports/E1-4-<date>.md` with: the mutation
      table; the corpus counts; whether E1-1's table still held the 82
      width-form sources and what that meant for
      `a_width_form_of_another_script_is_never_a_homoglyph`; any place
      HEAD differed from §3; and the twelve lowest-code-point choices
      of D21 step 2 as the Rust tables produce them (§3.6 lists the
      model's), so the owner can read them.

## §7 Out of scope

- **Multi-code-point skeletons** (U+FF4D → `rn`, U+FB00, ligatures) and
  UTS #39's NFD-before-skeleton step: E1-1's filter keeps one-code-point
  targets only (A §3.2). A precomposed Cyrillic ё in a Latin word is not
  found.
- **Scripts outside Latin, Cyrillic, Greek** — Armenian `ո`, Cherokee,
  Coptic look-alikes, mathematical alphanumerics (`Common`). A §1 scopes
  the class to the three.
- **A dictionary or language model** to tell `рау` (U+0440 U+0430 U+0443, an attack) from `сор` (U+0441
  U+043E U+0440, a Russian word)
  (a Russian word in an English sentence). W4/W5 are the whole of the
  judgement; a lone Russian word of imitation letters in a long English
  paragraph *is* redrawn, as A §5.4 intends.
- **A language-aware choice among several twins** — D21 takes the
  lowest code point (Kazakh `ү` U+04AF for a Latin `y` would need to
  know the language). Changing it is an owner decision, not this
  document's.
- **Settings rows** for `aggressive` (A §7.4; Q-A1); the surfaces'
  wording (E1-6); the Inspector (E7).
- **Protected spans** `⟦n⟧` (E4): Layer A does not know what is code.

## §8 Basis and references

- **A** = `wipemark-core-layer-a-2026-09-21` (`ssd-docs/`): §1 row
  "§3.1 Homoglyphs" (the mixed-word rule and why), §3.2 (`CONFUSABLE`
  filter and reverse index), §4.1 row `Homoglyph`, §5.2 (`aggressive →
  Homoglyph: Replace`), §5.3 (idempotence: "a homoglyph replacement makes
  a word more homogeneous, not less"), §5.4, §8 row "гомоглифы" (the
  five tests and "remove the mixed-word condition — the first and fourth
  go red").
- **OV** = `heretic-unmark-overview-decomposition-2026-09-07`: §3.1 row
  "Homoglyphs" (Latin↔Cyrillic↔Greek, `aggressive` only, Probable), §10
  E1 S1.6.
- `docs/plan/README.md` §3.3 (contract), §4 D3, D4, D6, D21 (revised),
  D22, D24, D26, D40 (W5), D41 (H1–H5), D42 (the decomposition veto,
  before the choice), §6 (risk "Russian prose under `aggressive`").
- `docs/plan/E1-1-ucd-tables.md` §4.8.8, §4.9; `docs/plan/E1-3-scrubber-and-nfkc.md`
  §4.4, §4.7, §5.4.
- **UTS #39** Unicode Security Mechanisms, §4 Confusable Detection
  (skeleton, `confusables.txt`): <https://www.unicode.org/reports/tr39/>;
  data <https://www.unicode.org/Public/18.0.0/security/confusables.txt>.
- **UAX #24** Unicode Script Property (`Common`, `Inherited`, why a
  combining mark has no script of its own): <https://www.unicode.org/reports/tr24/>.
- **UAX #44** (General_Category values `Lu`, `Ll`, `Lt`, `Lm`, `Lo`):
  <https://www.unicode.org/reports/tr44/>.
- **UAX #15** (why a `<compat>` letter is not NFKC-stable):
  <https://www.unicode.org/reports/tr15/>.
- `CLAUDE.md` (at `497eafa`): `:85` delete the protection, `:192` zero
  dependencies, `:220` catalogue, `:226` only applications localize,
  `:1037` tests must be able to fail.
