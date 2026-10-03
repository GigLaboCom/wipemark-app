# Layer A — the deterministic scrubber

Layer A is the half of Wipemark that does not guess: it finds and
removes the invisible and look-alike characters a text can carry —
zero-width characters, bidi controls, tags, variation selectors out of
place, exotic spaces, noncharacters, private use, homoglyphs — by the
properties the Unicode Standard gives every code point. It lives in
`crates/wipemark-core`, has no dependency of any kind, and is the
*verifiable* shelf of every report: a finding there is a code point that
either has a published property or does not, and the report names the
Unicode version that says so. This document grows one section per E1
document as they land — Tables (E1-1), Classes and context (E1-2),
Scrubber, report and NFKC (E1-3), Homoglyphs (E1-4), Guards (E1-5),
Surfaces (E1-6) — and E1-7 completes it.

## Tables

### What is committed, and why it is committed

`crates/wipemark-core/ucd/` holds nine files of the Unicode Character
Database, version **18.0.0**, byte for byte as unicode.org publishes
them, with their checksums in `ucd/SHA256SUMS` and a short
`ucd/README.md`. `crates/wipemark-core/build.rs`, written against `std`
alone, turns them into sorted static tables at build time.

They are committed rather than taken from a `unicode-*` crate for two
reasons. The report has to name the Unicode version that produced a
finding, and a crate pins someone else's version and hides which one it
was. And `wipemark-core` has zero dependencies — normal, dev *and*
build (`scripts/check-dep-direction.sh` reads all three tables) — so the
generator cannot lean on a parser crate either.

| file | read for |
|---|---|
| `UnicodeData.txt` | `General_Category`, `Canonical_Combining_Class`, `Bidi_Class` R/AL, the decomposition mappings, the `First`/`Last` ranges, and the names of the code points that can be findings |
| `DerivedCoreProperties.txt` | `Default_Ignorable_Code_Point` |
| `PropList.txt` | `Bidi_Control`, `Variation_Selector`, `Join_Control`, `Noncharacter_Code_Point` |
| `Scripts.txt` | `Script`, folded into the `Script` enum |
| `StandardizedVariants.txt` | the (base, selector) pairs |
| `DerivedNormalizationProps.txt` | `Full_Composition_Exclusion` |
| `NormalizationTest.txt` | line 1 only, for the version gate; its body is NFKC's conformance input (E1-3, all six parts) |
| `emoji/emoji-data.txt` | `Emoji`, `Emoji_Presentation`, `Emoji_Modifier`, `Emoji_Modifier_Base`, `Emoji_Component` |
| `confusables.txt` (UTS #39) | every source, its skeleton, and the type field |

`.gitattributes` marks the data files `-text -diff`: `-text` because the
checksum is over the bytes and `build.rs` refuses a carriage return,
`-diff` because a version bump is megabytes nobody reads. What gets
reviewed is `SHA256SUMS`, which diffs, and the suite —
`every_ucd_file_matches_its_checksum` is the counterpart of `-diff`: a
silent edit of one value builds green and fails there.

### How the version is established

Eight files name their version — line 1 is `# <stem>-18.0.0.txt`, or the
leading comments carry `# Version: 18.0.0` (`emoji-data.txt`,
`confusables.txt`). `build.rs` reads all eight, fails the build when they
disagree, and emits the common value as `wipemark_core::UNICODE_VERSION`;
nobody types it.

`UnicodeData.txt` has no header at all; its first line is data. It is
bound to the others by what it assigns: by UAX #24, `Scripts.txt` gives
a script to exactly the assigned code points other than private use and
surrogates, so the set `UnicodeData.txt` assigns (minus `Co` and `Cs`)
must equal the set `Scripts.txt` lists — 172,873 code points in 18.0.0.
A 17.0.0 `UnicodeData.txt` beside 18.0.0's `Scripts.txt` differs by
13,007 and fails the build. The one blind spot is a release that assigns
nothing new.

### The lookups

`src/tables.rs` includes the generated `$OUT_DIR/tables.rs` and exposes
`pub(crate)` functions; nothing outside that module depends on a table's
layout. A predicate is a binary search over sorted inclusive ranges; a
value is a binary search by key. Every property is kept **as
published**; where Layer A wants less, the subtraction is the
classifier's (E1-2), written where the reason is.

| function | source property | the edge to know |
|---|---|---|
| `is_default_ignorable` | `Default_Ignorable_Code_Point` | includes reserved code points (U+2065, U+FFF0..U+FFF8, U+E0000 …) and U+1BCA0..U+1BCA3, U+1D173..U+1D17A — D19 makes those two ranges never findings, in `class_of` |
| `is_bidi_control` | `Bidi_Control` | twelve code points |
| `is_variation_selector` | `Variation_Selector` | includes the Mongolian FVS U+180B..U+180D, U+180F; not U+180E |
| `is_join_control` | `Join_Control` | U+200C, U+200D |
| `is_noncharacter` | `Noncharacter_Code_Point` | 66 code points |
| `is_private_use` | `gc=Co` | three `First`/`Last` ranges, whole |
| `is_space_separator` | `gc=Zs` | **includes U+0020**; `ExoticSpace` is `Zs` minus U+0020 |
| `is_format` | `gc=Cf` | |
| `is_letter` | `gc=L*` | includes the CJK, Hangul, Tangut, Jurchen and Seal ranges listed by their ends |
| `is_mark` | `gc=M*` | variation selectors are `Mn`, so marks |
| `is_decimal_digit` | `gc=Nd` | |
| `is_uppercase_letter`, `is_lowercase_letter` | `gc=Lu`, `gc=Ll` | `Lt` is neither |
| `is_rtl` | `Bidi_Class` R or AL | **true for U+200F RLM and U+061C ALM**; D20 (only an R/AL letter or mark makes a paragraph RTL) is E1-2's; false for unassigned code points — `DerivedBidiClass.txt`'s block defaults are not committed |
| `is_emoji` … `is_emoji_component` | the five emoji properties | `Emoji` is true for `#`, `*`, digits, U+00A9, U+00AE; `Emoji_Component` includes U+200D, U+FE0F and the tags |
| `is_han`, `script_of` | `Script` | `Script` names 55 UCD scripts plus `Other`; every other value, `Unknown`, unassigned, private use and noncharacters are `Other`; `Inherited` marks and ZWJ/ZWNJ stay `Inherited` |
| `is_standardized_variant` | `StandardizedVariants.txt` | knows **no** emoji presentation sequence (`… U+FE0F`) and no IVD sequence |
| `decomposition` | `UnicodeData.txt` field 5, expanded | the full NFKD mapping, recursion done at build time; not in canonical order; `None` for Hangul syllables |
| `ccc` | `Canonical_Combining_Class` | 0 when not listed |
| `compose` | canonical pairs minus `Full_Composition_Exclusion` | `None` for excluded and non-starter decompositions and for Hangul |
| `confusable_target` | `confusables.txt`, filtered | `Some(skeleton)` only for a letter of Latin, Cyrillic or Greek with a one-code-point skeleton (D22); **`None` for a prototype** such as U+0061 — a caller's skeleton is `confusable_target(c).unwrap_or(c)` |
| `confusables_with` | the reverse index | every homoglyph-capable letter with that skeleton in that script, ascending, prototype included, letters **with** a decomposition included (U+FF41 for `a`); choosing one (D21, D42) is E1-4's |

`wipemark_core::name_of` (`src/name.rs`) and `wipemark_core::UNICODE_VERSION`
are the two public items.

### What is deliberately not in the tables

- **Hangul syllables**: their decomposition and composition are
  arithmetic (The Unicode Standard §3.12), done by E1-3. `build.rs`
  fails if a future `UnicodeData.txt` gives one a mapping (G6).
- **Defaults of unassigned code points** (`DerivedBidiClass.txt` and
  friends): an unassigned code point is not a finding (A §2, Q-A3).
- **Emoji sequences** (`emoji-sequences.txt`, `emoji-zwj-sequences.txt`,
  `emoji-variation-sequences.txt`) and the **IVD**: E1-2 keeps emoji by
  property (`Emoji=Yes`), not by sequence list.
- **`Blocks.txt`**: D22 dropped the only rule that would have needed it.

### Names

`name_of` names only the set N of code points that can ever be a
finding — default-ignorables, bidi controls, variation selectors, `Zs`
minus U+0020, noncharacters, private use, U+FFF9..U+FFFB, the tag block,
U+00AD, U+200B..U+200D, U+2060, U+FEFF, and every homoglyph-capable
letter (sources and prototypes). Every name in `UnicodeData.txt` would
be about 1.07 MB of strings in the binary; N's are about 23 KB. N is a
deliberate superset of what E1-2 calls a finding: a spare name costs a
few bytes, a missing one is a hole in a report. A member of N without a
name gets a code point label (UAX #44 §4.2.5) — `<private-use-E000>`,
`<noncharacter-FDD0>`, `<reserved-E0080>` — which can never be mistaken
for a name. `name_of` returns `Option<Cow<'static, str>>` (D7) because
labels are made on demand: a static string per private-use code point
would be 1.5 MB. The twelve D19 code points keep their names: they are
default-ignorable, and dropping them from the names alone would label
assigned characters `<reserved-…>`. A name is an identifier of the
standard, never translated (D16).

### The build-time gates

Each is a `panic!` in `build.rs`, so it fails `cargo build`, `test` and
`clippy` alike, with the file, the line and what to do.

| gate | fires when |
|---|---|
| G1 | a file's version cannot be read, or two files name different versions |
| G2 | `UnicodeData.txt`'s assigned set ≠ `Scripts.txt`'s listed set |
| G3 | a line breaks a structural rule (field count, a bad code point, a `First` without its `Last`, an unknown category or tag, a carriage return, a duplicate) |
| G4 | a generated table is empty, a wanted property occurs nowhere, or a kept script name occurs nowhere (a typo would fold a whole script into `Other`) |
| G5 | an `@missing` line contradicts what `build.rs` assumes about unlisted code points |
| G6 | Hangul appears in a decomposition or as a composite, or the pool outgrows its index types |
| G7 | a finding-capable code point has neither a name nor a label `name_of` can give it |

G6 and G7 hold on 18.0.0 and cannot be painted red without fabricating
data; `hangul_is_left_to_arithmetic` and
`a_name_exists_exactly_for_what_can_be_a_finding` assert the same facts
at test time. The tests compare every lookup with an independent
reading of its file over all 1,114,112 code points (unit tests in
`tables.rs`, `script.rs`, `name.rs`, which may `include_str!` from
`ucd/` — D24), and `tests/ucd_files.rs` checks the files themselves:
one version, the README, a std-only SHA-256 against `SHA256SUMS`.

### Bumping the version

```sh
scripts/fetch-ucd.sh <version>     # downloads, checks the headers, writes SHA256SUMS; ucd/ untouched on failure
# update the version in crates/wipemark-core/ucd/README.md
cargo test -p wipemark-core
```

`the_pinned_version_is_18_0_0` makes the bump a deliberate edit. A red
suite afterwards means Unicode reassigned or reshaped something the code
relied on — `the_sets_layer_a_names_by_hand_are_the_sets_unicode_gives`
is the likeliest — read A §4.1 before touching either side.

### Sizes (Unicode 18.0.0)

The generated `tables.rs` is 453,700 bytes of ASCII source; the data it holds
is about 202 KB:

| table | entries | ≈ bytes |
|---|---|---|
| 20 range predicates (`DEFAULT_IGNORABLE` … `HAN`) | 2,967 ranges | 23,736 |
| `SCRIPT` | 656 | 7,872 |
| `STANDARDIZED_VARIANTS` | 1,437 | 11,496 |
| `DECOMPOSITION` + `DECOMPOSITION_POOL` | 5,982 + 9,265 chars | 84,916 |
| `CCC` | 422 | 5,064 |
| `COMPOSITION` | 961 | 11,532 |
| `CONFUSABLE` | 358 | 2,864 |
| `CONFUSABLE_REVERSE` | 306 (493 chars) | 9,316 |
| `NAME` | 917 | 45,057 |
| total | | 201,853 |

## Classes and context

Two questions, answered in two places. `class_of` (`src/class.rs`) says
what a code point **is** — which of the eleven classes, if any — from
its Unicode properties alone. `context::hits` (`src/context.rs`) says
whether, **here**, it is somebody's spelling or a mark: a VS16 after a
heart, a ZWJ inside a family, a ZWNJ inside a Persian word, an LRM in a
Hebrew paragraph are all *kept by context*. The hard half is the second
one; finding invisible characters is a binary search, not mistaking
orthography for a watermark is the job.

### One class per code point

`pub fn class_of(c: char) -> Option<UnicodeClass>`, reachable as
`wipemark_core::class::class_of` (the root re-exports are E1-3's).
Each class's definition
is a private predicate, `claims(class, c)`, and `class_of` returns the
first class in `UnicodeClass::ALL` order that claims the code point.
Order and definitions are kept apart on purpose:
`every_code_point_has_at_most_one_class` walks all 1,112,064 scalar
values and checks that **no two definitions** claim one code point, so
the order never decides anything a reader would have to know about.

| # | class | definition | members in 18.0.0 | count |
|---|---|---|---|---|
| 1 | `ZeroWidth` | {U+200B, U+200C, U+2060, U+FEFF} | 200B–200C, 2060, FEFF | 4 |
| 2 | `ZeroWidthJoiner` | {U+200D} | 200D | 1 |
| 3 | `BidiControl` | `Bidi_Control` | 061C, 200E–200F, 202A–202E, 2066–2069 | 12 |
| 4 | `TagCharacter` | the Tags block, assigned or reserved | E0000–E007F | 128 |
| 5 | `VariationSelector` | `Variation_Selector` (Mongolian FVS included) | 180B–180D, 180F, FE00–FE0F, E0100–E01EF | 260 |
| 6 | `SoftHyphen` | {U+00AD} | 00AD | 1 |
| 7 | `ExoticSpace` | `gc=Zs` minus U+0020 | 00A0, 1680, 2000–200A, 202F, 205F, 3000 | 16 |
| 8 | `Noncharacter` | `Noncharacter_Code_Point` | FDD0–FDEF, U+nFFFE/U+nFFFF for n = 0…16 | 66 |
| 9 | `PrivateUse` | `gc=Co` | E000–F8FF, F0000–FFFFD, 100000–10FFFD | 137,468 |
| 10 | `DefaultIgnorable` | (`Default_Ignorable_Code_Point` minus classes 1–9) ∪ {FFF9–FFFB}, minus {1BCA0–1BCA3, 1D173–1D17A} (D19) | 034F, 115F–1160, 17B4–17B5, 180E, 2061–2065, 206A–206F, 3164, FFA0, FFF0–FFFB, E0080–E00FF, E01F0–E0FFF | 3,759 |
| 11 | `Homoglyph` | never returned by `class_of` | — | 0 |
| | | | **finding-capable in all** | **141,715** |

`every_class_has_exactly_the_members_unicode_18_gives_it` holds this
table as literal ranges, written by hand and not generated from
`claims`, so a definition and its data cannot agree by accident.

- **The script format controls are in no class.** Of the 170 `gc=Cf`
  code points in 18.0.0, 41 can never be findings: the prepended
  concatenation marks U+0600–0605, U+06DD, U+070F, U+0890–0891, U+08E2,
  U+110BD, U+110CD and the Egyptian quadrat controls U+13430–1343F,
  which UCD itself keeps out of `Default_Ignorable_Code_Point`; and the
  Duployan shorthand controls U+1BCA0–1BCA3 and the musical beam, tie,
  slur and phrase controls U+1D173–1D17A, which UCD 18.0.0 *does* list
  as default-ignorable and `claims` excludes **by name** (D19). All of
  them render with or format their own script or notation — the Arabic
  number sign sits over the digits after it, a quadrat control arranges
  hieroglyphs, a beam control joins notes — and Layer A looks for the
  invisible. No context rule is needed for them because they are not
  findings at all.
- **U+FFF9–FFFB**, the interlinear annotation characters, are `Cf`,
  excluded from `Default_Ignorable_Code_Point` by name in
  `DerivedCoreProperties.txt`, and rendered by nothing: spec A adds them
  to `DefaultIgnorable` explicitly.
- **Reserved code points** are not findings (A §2, Q-A3) unless they
  are default-ignorable (U+2065, U+FFF0–FFF8, U+E0080–E00FF,
  U+E01F0–E0FFF → `DefaultIgnorable`) or in the Tags block
  (U+E0000, U+E0002–E001F → `TagCharacter`).
- **`Homoglyph` never comes out of `class_of`.** A Latin `a` is only a
  finding inside a Cyrillic word; that needs the word, which is
  `homoglyph::hits` (E1-4). No letter of Latin, Cyrillic or Greek is
  finding-capable, so the two hit streams never report one offset.
- **The BOM is not `class_of`'s business.** `class_of('\u{FEFF}')` is
  `ZeroWidth` everywhere; that byte 0 is a byte order mark is a fact
  about a position, applied by `hits`.

### What context keeps

`pub(crate) fn hits(text: &str) -> Vec<Hit>`: one `Hit` per
finding-capable code point, in source order, `at` a byte offset into
`text`; `kept_by_context` true means orthography or presentation, with
confidence `LikelyFalsePositive`; false means the class's ceiling
(`max_confidence()`), for the scrubber to decide on. It takes no
`Options`: what context keeps, it keeps whatever the knobs say.

**State.** Two pre-passes, then one left-to-right pass:

- *Pre-pass 1* marks each paragraph right-to-left or not. A paragraph
  ends at U+000A only (P8). It is RTL when it holds a code point with
  `Bidi_Class` R or AL **that is a letter or a mark** (D20).
- *Pre-pass 2* finds every valid flag tag sequence (D39) and returns the
  byte ranges of its tags and terminator.
- `prev` is the code point immediately before the current one.
- **Glue** is U+200D, every variation selector and every tag: the
  characters that attach to what came before. U+200C and U+20E3 are
  not glue.
- `prev_kept`, the anchor, is the last code point that is not glue and
  survives a clean under every `Options`: a non-finding, a hit kept by
  context, or an exotic space (D35). A not-kept finding, a soft hyphen
  and a leading BOM never become it.
- The anchor's **effective script** is its own, or for a
  `Script=Inherited` mark the effective script of the anchor before it
  (P6).
- `next` is the code point immediately after; it may itself be a
  finding, and then it disqualifies (D34).

**The rules** (A §4.2, made exact):

| class | code points | kept when |
|---|---|---|
| `VariationSelector` | U+FE0E, U+FE0F | `prev` is `Emoji=Yes` and not a finding — ASCII `#`, `*`, `0`–`9` included (keycap bases have text and emoji styles) |
| `VariationSelector` | U+FE00–FE0D | `prev` is not a finding and (`prev`, selector) is in `StandardizedVariants.txt` |
| `VariationSelector` | U+E0100–E01EF | `prev` is not a finding and is Han, or the pair is listed |
| `VariationSelector` | U+180B–180D, U+180F | `prev` is a Mongolian letter |
| `DefaultIgnorable` | U+180E | `prev_kept` is a Mongolian letter |
| `ZeroWidthJoiner` | U+200D | `prev_kept` and `next` are both emoji bases (non-ASCII `Emoji` or `Emoji_Modifier`, not a finding), **or** the joining rule |
| `ZeroWidth` | U+200C only | the joining rule: `prev_kept` is a letter or mark, not a finding, whose effective script is in `JOINING_SCRIPTS`, and `next` is a letter or mark, not a finding, of that same script by its own script |
| `TagCharacter` | U+E0000–E007F | inside a valid flag sequence: U+1F3F4, 1–30 tags from U+E0030–E0039 / U+E0061–E007A, U+E007F |
| `DefaultIgnorable` | U+17B4, U+17B5 | `prev_kept` is a Khmer letter |
| `DefaultIgnorable` | U+115F, U+1160, U+3164, U+FFA0 | `prev_kept` **or `next`** is a Hangul letter that is not itself a filler |
| `BidiControl` | U+061C, U+200E, U+200F, U+202A–202C, U+2066–2069 | the paragraph is RTL |
| `BidiControl` | U+202D, U+202E | never (Trojan Source, CVE-2021-42574) |
| everything else | U+200B, U+2060, an inner U+FEFF, soft hyphens, exotic spaces, noncharacters, private use, other default-ignorables | never by context (a soft hyphen or exotic space may still be kept by `Options` in E1-3; that is not `kept_by_context`) |

A U+FEFF at byte 0 of the text passed in is a byte order mark: no hit.

`JOINING_SCRIPTS` is a list of 44 scripts, not a property: the UCD has
none for "uses ZWJ/ZWNJ orthographically" — `Joining_Type` knows the
cursive scripts and not the Indic ones, `Indic_Syllabic_Category` the
reverse, and their union needs two files this crate does not commit.
Latin, Cyrillic, Greek and the CJK scripts are absent on purpose: a ZWNJ
between two Latin letters is the classic carrier.

**Decisions, one line each** (`docs/plan/README.md` §4; E1-2 §4.0):

- **D19** — the Duployan and musical format controls are never findings,
  though UCD 18.0.0 makes them default-ignorable: they format their own
  notation.
- **D20** — RTL evidence is an R/AL *letter or mark*: U+200F (R) and
  U+061C (AL) are bidi controls, and counted they would let a stray RLM
  protect itself.
- **D34** — every character a rule reads (`prev`, `prev_kept`, `next`) is
  itself not finding-capable; otherwise U+17B4 would justify a ZWNJ, a
  filler the next filler, and a second clean would differ from the first.
- **D35** — an exotic space moves `prev_kept`: it survives every clean,
  as itself or as U+0020, and U+0020 moves it.
- **D36** — a selector's base is `prev`, the code point immediately
  before it; with `prev_kept` a run of selectors after one emoji or
  ideograph would all be kept — a byte channel.
- **D37** — ASCII is never a ZWJ side or a tag base (no RGI sequence has
  one, and a ZWJ between two digits would be kept inside every number);
  VS15/VS16 after `#`, `*`, a digit stay kept.
- **D38** — a Hangul filler counts `next` too: U+115F stands for a
  missing *leading* consonant and starts its syllable.
- **D39** — tags are kept only in a UTS #51 Annex C.1 flag sequence;
  A's "any `Emoji=Yes` base" kept the ASCII-smuggling shape.
- **P4** — the Han clause covers IVS only; VS1–VS14 after an ideograph
  need a `StandardizedVariants.txt` pair (the IVD registers VS17 and up).
- **P6** — the anchor's script falls back across `Inherited` marks; an
  `Inherited` `next` does not match (its base would be the joiner).
- **P8** — a paragraph ends at U+000A only; text using `\r` or U+2029
  alone is one paragraph, which errs toward keeping marks.

### Why a second pass agrees

Remove every hit that is not kept (an exotic space may stay or become
U+0020, a soft hyphen may stay) and run `hits` again: the kept hits are
the same, in order, and nothing else is found but those spaces and
hyphens. It holds because every base a rule reads is a non-finding
(D34) that survives and stays adjacent, `prev_kept` only moves on what
survives under every `Options` (D35), and RTL evidence and a tag
sequence's parts survive together. `what_is_kept_stays_kept_on_the_output`
is that argument as a test, over every test input and fixture under all
four combinations of the two knobs; E1-3's idempotence gates stand on it.

### What is deliberately not protected

- **Floating bidi marks in a left-to-right paragraph**: no RTL letter,
  no work for an LRM — removed, `Confirmed`.
- **Pairing of embeddings** in an RTL paragraph is not checked (Q-A2);
  an unpaired U+202C there is kept.
- **Legacy Malayalam chillu** (`consonant, U+0D4D, U+200D` at a word end,
  pre-5.1 encoding) and any word-final `virama, ZWJ` half-form display:
  `next` is a space, so the ZWJ is removed. A known false positive (Q-A7).
- **Latin ligature control** (German *Auflage* with U+200C): removed by
  design; Latin is not a joining script.
- **U+034F COMBINING GRAPHEME JOINER** is removed wherever it is, though
  Hebrew and some German typography use it. A known false positive; no
  rule in A (Q-A7).
- **The subdivision code inside a flag is not checked against CLDR.**
  U+1F3F4 + 1–30 digit/lowercase tags + U+E007F is kept whatever it
  spells (`zzzz` as well as `gbsct`); the crate carries no CLDR data. A
  flag-shaped sequence can still carry up to 30 such tags; anything else
  in tags is a finding (D39; the owner may widen it, Q-A8).
- **Egyptian, Duployan and musical context**: none needed — their format
  controls are never findings.
- **Unassigned code points** that are not default-ignorable (Q-A3).
- **The whole text, not a slice**: "byte 0" and "paragraph" are relative
  to the string passed in. A caller that chunks a document (E4) must
  not cut where either would change.

### Where the protections are tested

All in `src/class.rs` and `src/context.rs` unit tests (D24); each was
painted red by its mutation before it counted (E1-2 report).

| protection | test | mutation that paints it red |
|---|---|---|
| one class per code point | `every_code_point_has_at_most_one_class` | drop the subtraction from `claims(DefaultIgnorable)` — 406 code points claimed twice |
| the 18.0.0 membership | `every_class_has_exactly_the_members_unicode_18_gives_it` | drop U+FFF9–FFFB, or drop D19's exclusion |
| no carrier joins | `no_carrier_alphabet_is_a_joining_script` | add `Script::Latin` to `JOINING_SCRIPTS` |
| script format controls | `script_format_controls_are_never_findings` | let `DefaultIgnorable` claim every `Cf`; drop D19 |
| leading BOM | `a_leading_bom_is_not_a_finding_and_an_inner_one_is` | drop the byte-0 exception |
| VS15/VS16 | `an_emoji_keeps_its_presentation_selector` | drop the `Emoji` check; read `prev_kept` (D36) |
| emoji ZWJ | `a_family_stays_a_family` | drop the emoji rule; let glue move `prev_kept` |
| joining scripts | `persian_keeps_its_non_joiner`, `devanagari_keeps_its_joiner` | remove Arabic / Devanagari from the list |
| P6 | `a_mark_carries_the_script_of_its_base` | no `Inherited` fallback; let an `Inherited` `next` match |
| RTL bidi | `a_bidi_mark_is_typography_beside_rtl_and_a_carrier_without_it` | drop the rule; decide RTL for the whole text |
| D20 | `a_stray_rlm_does_not_protect_itself` | count any R/AL code point as evidence |
| overrides | `an_override_is_always_removed` | treat U+202D/U+202E like the other controls |
| D39 | `a_flag_keeps_its_tags_and_a_loose_tag_does_not`, `an_emoji_cannot_smuggle_tags` | no terminator; no base; no 32 limit; any `Emoji` base; any tag |
| IVS, P4 | `an_ideograph_keeps_its_variation_sequence` | drop the Han clause; extend it to VS1–VS14; `prev_kept` |
| standardized variants | `a_standardized_variant_is_kept_and_a_random_one_is_not` | drop the lookup |
| Mongolian | `a_mongolian_letter_keeps_its_selector` | drop the FVS or the U+180E rule; `prev_kept` for the FVS |
| Khmer | `khmer_keeps_its_inherent_vowel` | drop the rule |
| Hangul fillers, D38 | `a_partial_syllable_keeps_its_filler` | drop the rule; drop `next`; let a filler be a Hangul letter |
| D34 | `a_base_is_never_itself_a_finding`, `what_is_kept_stays_kept_on_the_output` | let a finding be the joiner's `next` |
| D37 | `an_ascii_character_is_never_a_joiner_side_or_a_tag_base` | let ASCII be an emoji base |
| one hit each, in order | `hits_come_in_source_order_one_per_finding_capable_character` | drop kept hits; skip exotic spaces |
| context only demotes | `a_kept_hit_is_a_likely_false_positive_and_any_other_carries_its_ceiling` | give kept hits the ceiling |
| byte offsets | `hit_offsets_are_bytes_not_chars` | report the char index |
| D35 | `what_is_kept_stays_kept_on_the_output` | an exotic space does not move `prev_kept` |
| the survival set | `every_keep_fixture_survives_whole` (`fixtures/text/keep-*.txt`) | remove any keep rule |

### `TextStats` and letter shares

`TextStats::of(text)` (`src/stats.rs`) and the crate-internal
`letter_shares(text)` read the crate's own 18.0.0 tables, never std's
`char::is_whitespace`, whose Unicode is not the one the report names.

- **whitespace** is `White_Space`: `gc=Zs`, U+0009–000D, U+0085, U+2028,
  U+2029 (equal to `PropList.txt` in 18.0.0); U+200B is not whitespace.
  A **token** is a maximal run of non-whitespace.
- `chars` — code points, a leading BOM included.
- `words` — tokens with at least one letter or decimal digit; a run of
  CJK without spaces is one word.
- a **letter** is `gc=L*`; a mark never is. The Hangul fillers are `Lo`
  and count as Hangul: statistics describe the text as given.
- **Latin**, **Cyrillic** by `Script`; **CJK** is Han, Hiragana,
  Katakana, Hangul and Bopomofo (U+30FC is `Common`, so *other*);
  **other** is every remaining letter.
- `latin_ratio`, `cyrillic_ratio`, `cjk_ratio` — **fractions** 0.0–1.0
  of the letters; `LetterShares` — **percent** 0–100, the unit
  `ScriptGuard`'s `max_delta_pp` is written in. Both come from the same
  integer counts, and both are 0.0, never NaN, without letters.
- `code_blocks` — fence lines (after leading spaces and tabs, starting
  ` ``` ` or `~~~`) divided by two, rounded down. A budget statistic,
  not a Markdown parser.
- `urls` — tokens containing `://`, one per token.

The Unicode facts in this section were read from the committed `ucd/`
files (18.0.0); after a version bump, run
`every_class_has_exactly_the_members_unicode_18_gives_it` and read its
diff before touching anything else.

## Guards

`src/guard.rs`. Five predicates over a pair *(source, candidate)* that
throw away a rewrite which lost something: a version number "improved",
a protected span dropped, a paragraph translated, a file path gone. They
are pure text with no engine, so they live in `core` and read the same
18.0.0 tables as the scrubber.

### Who calls them

Nobody in E1 but their tests. E4's selection loop (OV §4.4) runs
`default_guards()` over every candidate of every round, **in that
order**, and reports the first rejection. It runs them on text whose
protected spans — code, URLs, e-mails, paths, numbers with units, the
user's regex list — are already `⟦n⟧` placeholders (OV §4.2); making
those is E4's, and a guard trusts that the source it is handed has them.
A guard compares the texts as given: E4 should run Layer A over a
candidate before its guards, or a model that writes U+200B inside
`snake_case` loses the identifier here.

### The five rules

| guard · `name()` | rule | reason | threshold |
|---|---|---|---|
| `PlaceholderGuard` · `placeholder` | every `⟦n⟧` of the source comes back as often as it went in, and no other comes back | `PlaceholderMissing { index }`, `PlaceholderDuplicated { index, count }`, `PlaceholderInvented { index }` — missing/duplicated by ascending index first, then invented by ascending index | — |
| `NumbersGuard` · `numbers` | every number token of the source is in the candidate, as an exact string, as a set | `NumberMissing { value }`, the first missing in source order | — |
| `LengthDriftGuard` · `length-drift` | `chars(candidate) / chars(source)` in `[min, max]`, both inclusive, code points never bytes; an empty source passes only an empty candidate, else the ratio is `∞` | `LengthDrift { ratio, min, max }` | `min: 0.6, max: 1.6` |
| `ScriptGuard` · `script` | no letter share (latin, cyrillic, cjk, other) moved by **more than** `max_delta_pp`; passes when either side has fewer than `min_letters` letters | `ScriptDrift { script, delta_pp }` — the first over the limit in the order latin, cyrillic, cjk, other; `delta_pp` signed, candidate minus source | `max_delta_pp: 15.0, min_letters: 20` |
| `IdentifierGuard` · `identifier` | every identifier-shaped token of the source is in the candidate, exactly, as a set | `IdentifierMissing { token }`, the first missing in source order | — |

The shares are E1-2's `stats::letter_shares`, in percent among letters
(`gc=L*`); CJK is Han, Hiragana, Katakana, Hangul and Bopomofo. A fixed
order rather than "the largest delta" keeps the verdict free of `f32`
ties: a two-script swap moves both shares by the same amount. The edges
of the length window are exact in `f32` (`6.0 / 10.0 == 0.6`,
`16.0 / 10.0 == 1.6`). `min > max` or a NaN bound rejects everything —
the caller's configuration, not a guess.

### The three tokenizers

All private, O(n), slices of their input in source order.

- **Placeholders** — U+27E6, a run of **ASCII** digits that is `0` or
  has no leading zero and fits `usize`, U+27E7. E4 writes `⟦{n}⟧` with
  `{}` and restores by exact text, so anything else is text that was
  lost: `⟦1⟧⟦2⟧` → `[1, 2]`; `⟦⟦1⟧⟧` → `[1]`; `⟦03⟧`, `⟦٣⟧` (U+0663),
  and twenty nines → nothing.
- **Numbers** — starts and ends with a `gc=Nd` digit (fullwidth and
  Arabic-Indic included), digits and `. , : / -` between, a `%` only
  when it touches the last digit; maximal munch: `v1.2.3` → `1.2.3`;
  `It was 3.` → `3`; `0.5 %` → `0.5`; `12:00-13:00` → itself; `-5` → `5`.
- **Identifiers** — maximal runs of non-`White_Space` (E1-2's
  definition, re-stated in `guard.rs` because `stats::is_white_space` is
  private; U+200B is not whitespace), trimmed at both ends by `TRIM`,
  kept when any of five shapes holds: **URL** (contains `://`),
  **e-mail** (an `@` with a character before it and a `.` after it),
  **path** (at least two non-empty segments when split on `/` and `\`),
  **snake_case** (a run of `_` with a letter or `Nd` digit right before
  and right after it), **CamelCase** (an `Ll` letter right before an `Lu`
  letter — Unicode's case, so `кВт` is one). `(parse_config()),` →
  `parse_config`; `/usr` → nothing (a root is one segment);
  `HTTPServer`, `__init__`, `a@b` → nothing.

`TRIM` is twenty-six characters: A §6's `.,;:!?()[]{}"'«»<>` and D43's
U+2018, U+2019, U+201C, U+201D, U+201E, U+2039, U+203A and the backtick
U+0060 — so a rewrite that curls or straightens the quotes around an
identifier, or drops its Markdown backticks, has not lost it. Only the
ends are trimmed; the `’` inside `user’s` stays.

### Why strict, and what they cannot see

A false reject costs one more candidate; a false pass costs the user a
number in their document. So the guards are strict and simple, and
their blind spots are deliberate and written on each type:

- **numbers as words** — `twenty-three` protects nothing, and a
  candidate that spells `23` as `twenty-three` is rejected;
- **signs** — `-5` and `5` are the same token;
- **units** — not tracked; `50 %` holds the token `50`, so `50 %` →
  `50%` passes and `50%` → `50 %` is rejected;
- **an identifier glued to a dash or an ellipsis** — `foo_bar—see` is
  one token, so a candidate that glues it loses it and is rejected;
- **Arabic, Hebrew and every other script** are one bucket, `other`:
  English translated into Arabic is caught as Latin falling, but the
  reason does not name Arabic (A §9 Q-A5).

Where the implementation is more precise than A §6 — canonical ASCII
placeholders, "as often as in the source", the order of reasons, the
inclusive window, strict `>` on the share delta, `<` on `min_letters`,
two segments for a path, a run of `_` — is E1-5 §4.12 (G1–G12), adopted
as D44 (G9 as amended by D43).

### Formats

`RejectReason`'s **fields** are the format: a surface a person reads
renders the variant through the catalogue, never its `Display`, which is
English for logs and diagnostics. `script` is one of `latin`,
`cyrillic`, `cjk`, `other`. The five `name()`s — `placeholder`,
`numbers`, `length-drift`, `script`, `identifier` — are kebab-case,
unique, never translated, and renamed only with the care of a config key:
E4's `Event::CandidateRejected.guard` carries one.

### Tests

All in `guard.rs`'s `mod tests` (D24; the tokenizers are private), plus
one doc-test on `default_guards` that reaches it from outside the crate.
A faithful pair (`SOURCE`, `FAITHFUL`) passes all five, and each
rejection test is a one-phrase edit of it. Every protection was deleted
locally and seen red (E1-5 report, `docs/plan/reports/E1-5-2026-10-03.md`):
counts compared as presence → `a_duplicated_placeholder_is_rejected`;
`.` ending a number → `a_lost_version_number_is_rejected`; bytes for
chars → `cjk_length_is_counted_in_chars_not_bytes`; no `min_letters` →
`a_short_text_has_no_script_share`; no path shape →
`a_lost_identifier_is_rejected`; two entries of `default_guards` swapped
→ `default_guards_are_in_spec_order`. The URL shape is nearly subsumed
by the path shape — every URL with a host has two segments — so its
table row is `file:///`, the one kind of URL only it sees.
