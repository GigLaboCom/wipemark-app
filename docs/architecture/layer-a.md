# Layer A — the deterministic scrubber

## What Layer A is

Layer A is the half of Wipemark that does not guess. It finds and
removes the invisible and look-alike characters a text can carry, by
the properties the Unicode Standard gives every code point, and it says
exactly what it did: every finding is a code point with a class, a
confidence and its byte offsets in the text it was handed. That makes it
the **verifiable** shelf of every report — a code point either has a
published property or it does not — and the report names the Unicode
version that says so: **18.0.0**, read by `build.rs` from the headers of
the committed UCD files and exported as `wipemark_core::UNICODE_VERSION`,
never typed. It lives in `crates/wipemark-core`, which has no
dependency of any kind (normal, dev or build), and it is deterministic:
the same text and the same `Options` give the same bytes and the same
report on every machine, and `clean(clean(x)) == clean(x)`.

**What it finds** — eleven classes (`UnicodeClass::ALL`): zero-width
characters (U+200B, U+200C, U+2060, an inner U+FEFF), the zero-width
joiner, bidi controls, tag characters, variation selectors, the soft
hyphen, exotic spaces (`Zs` other than U+0020), noncharacters, private
use, the rest of `Default_Ignorable_Code_Point`, and **homoglyphs** —
a letter of Latin, Cyrillic or Greek inside a word of another of the
three, or a whole word drawn in look-alikes of its paragraph's script.
The first ten are properties of a code point (`class_of`); the eleventh
needs the word (`homoglyph::hits`).

**What it keeps, and why.** The same code points are somebody's
orthography or presentation far more often than they are a mark: the
ZWJ inside an emoji family, the VS16 after a heart, the ZWNJ inside a
Persian word, the ZWJ of a Devanagari conjunct, an LRM in a Hebrew
paragraph, the tags of a subdivision flag, an ideographic variation
sequence, a Mongolian free variation selector, a Khmer inherent vowel, a
Hangul filler, a byte order mark at byte 0. Context keeps them — at
`LikelyFalsePositive`, in the report's `kept` list, whatever the knobs
say — and a Russian word in Russian prose is never a homoglyph. Removing
those would not clean a document, it would damage it; that is the hard
half of this layer and most of this document.

**What it never looks at** (spec A §2): C0/C1 controls other than tab,
line feed and carriage return; U+2028/U+2029; unassigned code points
that are not default-ignorable — a text from a newer Unicode may carry
characters 18.0.0 does not know, and removing them is worse than missing
them (Q-A3); formats — it sees text, not Markdown, HTML or a code block,
so `nfkc` normalises code too; and stylometry or statistical marks,
which are Layer B's and are listed on the third shelf, *not
established*, in every report.

**Who calls it.** Today two applications, both off the GPUI thread:
`wipemark-cli inspect|clean` and the MCP server's `inspect` and `clean`
tools (*Surfaces*, below). To come: E4's pipeline, which runs Layer A
before and after Layer B on every chunk and the five guards over every
candidate; and E7's windows, where the Compare window's result becomes
`clean(original)`, the panel shows a findings count, the queue gains a
Clean action and the Inspector jumps to a position. Until then the
windows say that they do not clean yet.

The sections follow the code from the bottom up — *Tables* (E1-1),
*Classes and context* (E1-2), *Scrubber, report and NFKC* (E1-3),
*Homoglyphs* (E1-4), *Guards* (E1-5), *Surfaces* (E1-6) — and end with
*The gates*, *The live gate* and *What is left open* (E1-7).

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
`-diff` because a version bump is megabytes nobody reads. The three
lines (D25 — a pattern with a slash is anchored at the root, and `*`
does not reach `ucd/emoji/`):

```
crates/wipemark-core/ucd/**/*.txt   -text -diff
crates/wipemark-core/ucd/SHA256SUMS -text
fixtures/text/**                    -text
```

`git check-attr` prints `text: unset` and `diff: unset` for both data
paths, `text: unset` for `SHA256SUMS` and for every fixture (checked at
the closure). What gets
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
are this section's two public items.

Five lookups have no caller outside their unit tests —
`is_join_control`, `is_format`, `is_emoji_presentation`,
`is_emoji_modifier_base`, `is_emoji_component` — so `tables.rs` carries
one module-wide `#![cfg_attr(not(test), expect(dead_code, reason = …))]`:
their statics live in the generated, `include!`d file, where an item
attribute cannot reach without `build.rs` emitting one, and `expect`
makes the build fail the day the last of them gains a caller.
`Script::as_str` is `#[cfg(test)]`: the tests fold `Scripts.txt` by it
and nothing that ships names a script.

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
`wipemark_core::class::class_of`; it is not re-exported at the root,
where the entry points are `inspect` and `clean`.
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

## Scrubber, report and NFKC

### What

`wipemark_core::inspect(text, &options)` and `wipemark_core::clean(text,
&options)` are the public API of Layer A, and they are **one decision
over one pass**. A run is, in order:

1. **hits** — `scrub::collect_hits`: E1-2's context pass
   (`context::hits`), merged by byte offset with the homoglyph hits
   (`homoglyph::hits`, E1-4 — below);
2. **one decision per hit** — `scrub::decide`, the only place a hit is
   kept, removed or replaced;
3. **rows and counters** — `scrub::run` aggregates the decisions into
   report rows and counts removals per class and replacements per kind;
4. **output** — in the same walk, when `clean` asked for it: the text
   between hits is copied as slices, so a clean text costs one copy;
5. **with `nfkc`** — NFKC and the pass again, in rounds, until a pass
   acts on nothing (below).

`inspect` is steps 1–3 with `build: false`; nothing else differs, so an
`inspect` can never see something other than what `clean` removes (A
§5.2), and `inspect_and_clean_agree` holds them to it on every fixture
and 2,000 generated strings under all 16 `Options`.

**The knobs** (`Options`, all off by default) correct the class default
and nothing else — `Options::action_for` is the whole table:

| class | default | knob | with the knob |
|---|---|---|---|
| `SoftHyphen` | remove | `keep_soft_hyphen` | keep |
| `ExoticSpace` | keep | `normalize_spaces` | replace with U+0020 |
| `Homoglyph` | keep | `aggressive` | replace with the hit's letter |
| the other eight | remove | — | — |
| any | — | `nfkc` | no effect on the action |

**Context beats every knob.** A hit E1-2 marked `kept_by_context` is
kept whatever `Options` say — the first line of `decide`, and
`context_beats_every_knob` is its gate. A homoglyph hit with no
replacement is kept rather than deleted: a letter is never removed for
want of a substitute.

**Two lists.** `findings` are the rows `clean` acted on (removed or
replaced), `kept` the rows it found and left — by context (at
`LikelyFalsePositive`) or by a knob or the class default (at the class's
own confidence: a kept soft hyphen is `Informational`, a homoglyph kept
without `aggressive` is `Probable`). Rows aggregate hits by `(acted,
class, code point, confidence)` (D6). `acted` is in the key so that an
acted row and a kept row of the same code point never merge; within one
run that can only happen to a homoglyph with and one without a
replacement, because the knobs are global — `acted_and_kept_rows_never_merge`.
A ZWJ kept in a family and a ZWJ removed between two letters are two
rows already, by confidence. The key is a `BTreeMap` key ordered as the
report must be: **class in `UnicodeClass::ALL` order, then code point,
then confidence from highest to lowest** — there is no second ordering
table to drift.

**`suspicious`** (D4) is "some row, in `findings` or `kept`, is at least
`Probable`". Soft hyphens and exotic spaces alone are `Informational`
and do not make a text suspicious; neither does orthography, which is
`LikelyFalsePositive`. A homoglyph kept for want of `aggressive` does,
because that is the case a user has to be told about. It is computed
once, in `scrub::is_suspicious`, and the JSON writer writes the field
rather than recomputing it.

**Positions are byte offsets into the source** handed to `inspect` or
`clean`, never into the output, and `count == positions.len()` always.
There is one place a count exceeds the positions behind it: with `nfkc`,
the passes after NFKC remove code points from NFKC *output* — U+2139
U+FE0F becomes U+0069 U+FE0F, and the now-orphaned U+FE0F goes — and no
byte offset there names a byte of the source. So those passes add to
`removed` and never to `findings`; and a row of `kept` can name a code
point that is not in the output. `CleanReport`'s doc comment says so in
full; `counters_agree_with_the_rows_on_every_fixture` holds the
invariant without `nfkc`.

`CleanReport` carries `suspicious` and `stats` too (D28), both over the
**source** by `inspect`'s rules — `TextStats::of(source)` once — so a
`clean` and an `inspect` of the same text agree on both, and E1-6 takes
the CLI's exit code for `clean` straight from `report.suspicious`.
`removed` lists only non-zero classes, in `ALL` order; `normalized`
lists `SpaceToAscii`, `Nfkc`, `Homoglyph` in that order, each only when
non-zero — except `Nfkc`, which is present whenever `nfkc` was asked
for, `0` included: "ran and changed nothing" is not "not asked".

### Where

| file | what |
|---|---|
| `src/lib.rs` | `inspect`, `clean`, `Options` and `action_for`, `Cleaned` |
| `src/scrub.rs` | `collect_hits` (the seam), `decide`, `run`, `is_suspicious`, the rounds |
| `src/nfkc.rs` | UAX #15 NFKC and its count |
| `src/json.rs` | `InspectReport::to_json`, `CleanReport::to_json` |
| `src/report.rs` | `InspectReport`, `CleanReport`, `NormKind` |
| `src/class.rs` | `Action { Remove, Replace, Keep }`, `Action::as_str`, `Confidence::as_str` |
| `tests/fixtures.rs` | every file of `fixtures/text/` and its claim |
| `tests/corpus.rs` | 10,000 generated strings: idempotence and agreement |
| `fixtures/text/` | `<class id>.txt` (cleaned), `survive-*.txt` and E1-2's `keep-*.txt` (byte-identical) |

### Why the rounds

A §5.3 asked for one more pass after NFKC. One is not enough (D26):
`U+2139 U+FE0F U+0301` with `nfkc` — the first pass keeps U+FE0F after
an emoji base; NFKC gives `U+0069 U+FE0F U+0301` (U+FE0F is a starter,
so it blocks U+0301 from composing with `i`); the second pass removes
the orphan and leaves `U+0069 U+0301`, which is not NFKC, and a second
`clean` would turn it into U+00ED. So `clean` runs NFKC and the pass in
rounds until a pass acts on nothing, at most `MAX_ROUNDS` (8) with a
`debug_assert!` at the cap — a context rule that one day breaks
convergence is a failed assertion, never a hung MCP thread.

Why that is idempotent: `clean(x, o)` ends on a text that is (a) a
fixpoint of the pass — the last pass acted on nothing — and, with
`nfkc`, (b) NFKC output, which NFKC leaves alone. `clean` of it runs a
pass that acts on nothing and, with `nfkc`, an NFKC that changes
nothing. (a) rests on E1-2's rule that no context decision leans on a
code point the pass removes (D34). The loop ends because after the
first NFKC no step lengthens the text — a removal creates no
compatibility character, U+0020 and homoglyph letters are NFKC-stable
(D42), composition only shortens — and every continuing round acts on
at least one code point. The gates: `nfkc_rounds_reach_a_fixed_point`
on the counterexample, `clean_is_idempotent_on_every_fixture`, and
`clean_is_idempotent_on_a_generated_corpus`, whose alphabet carries the
trap atoms so that "stop after one round" fails deterministically.

### NFKC

`nfkc::nfkc_counted` is UAX #15 over E1-1's tables: the full
compatibility decomposition (`tables::decomposition`, already expanded
at build time), canonical ordering (a stable sort of every run of
non-starters by `ccc`), canonical composition. Hangul is in none of the
tables: a syllable is decomposed and composed by the arithmetic of §3.12
(`S_BASE` U+AC00, `L_BASE` U+1100, `V_BASE` U+1161, `T_BASE` U+11A7; LV,
then LV + T with T strictly above U+11A7). The **blocked** rule (UAX #15 D115)
needs only the last unit pushed after the starter, because the buffer is
in canonical order: a unit composes with the starter when nothing was
pushed after it or that unit's class is non-zero and lower than its own;
a starter that does not compose becomes the next starter, which is what
makes the chained composites of Part 5 work.

**The conformance gate** is the Unicode file itself:
`nfkc_conforms_to_the_unicode_test_file` runs every data line of all six
parts of `ucd/NormalizationTest.txt` — 20,171 lines in 18.0.0 (D23),
`c4 == NFKC(cK)` for all five columns — and
`every_code_point_the_test_file_does_not_list_is_its_own_nfkc` is the
file's conformance clause 2 over the 293,187 assigned code points Part 1
does not list, which catches a table entry that should not exist.

**The count** (`(Nfkc, n)`, D27) is the number of input code points NFKC
did not carry through unchanged, summed over the rounds. A code point is
carried through when exactly one output unit came from it, that unit
is still in source order in the output (a composite counts as coming
from every code point it absorbed), and it is the code point itself. Examples: U+FB01 → `fi`
counts 1 (one became two); `e` U+0301 → U+00E9 counts 2 (two merged);
`x` U+0301 U+0316 → `x` U+0316 U+0301 counts 2 (reordered); U+00E9 →
U+00E9 counts 0. The count is 0 exactly when the output equals the input
(`the_count_is_zero_exactly_when_nothing_changed`, over every column of
the test file). "In source order" is judged on the output, not on the
reordering: U+1E0A U+031B is its own NFKC although its decomposition's
U+0307 is reordered past U+031B on the way, and it counts 0.

### The JSON form

Both reports serialise themselves (`to_json`), in `wipemark-core`, with
a `std`-only writer (D9) — one format, one writer, for the CLI's
`--json` and the MCP server. One line, no whitespace outside strings,
keys in a fixed order:

```
inspect: unicode_version, suspicious, findings, kept, stats, not_established
clean:   unicode_version, suspicious, findings, kept, removed, normalized, output_len, stats, not_established
```

A row is `{"codepoint":"U+200B","name":"ZERO WIDTH SPACE","class":"zero-width","confidence":"confirmed","action":"remove","count":2,"positions":[5,40]}`;
`action` is `keep` for a row of `kept` and, for a row of `findings`,
the action `clean` took — a function of the class alone with today's
knobs (`replace` for exotic spaces and homoglyphs, `remove` for the
rest). A whole inspect report:

```json
{"unicode_version":"18.0.0","suspicious":true,"findings":[{"codepoint":"U+200B","name":"ZERO WIDTH SPACE","class":"zero-width","confidence":"confirmed","action":"remove","count":2,"positions":[5,40]}],"kept":[{"codepoint":"U+200D","name":"ZERO WIDTH JOINER","class":"zwj","confidence":"likely-false-positive","action":"keep","count":1,"positions":[12]}],"stats":{"chars":42,"words":7,"latin_ratio":0.98,"cyrillic_ratio":0.0,"cjk_ratio":0.0,"code_blocks":0,"urls":1},"not_established":["vendor-detector-evasion","human-authorship","unknown-mark-schemes"]}
```

**Escaping** (D29): `"` and `\` are escaped, and every character below
U+0020 or from U+007F upward is written as `\u` and four lowercase hex
digits per UTF-16 unit (a surrogate pair above U+FFFF). No short
escapes. The output is ASCII by construction: a report carries no user
text, only ids, `U+XXXX`, UCD names and numbers, so this costs nothing,
and "the JSON a client renders carries no invisible character" is a
property of the writer rather than of the data. **The third shelf** is
written by the writer, from `report::not_established::ALL`, as the last
key of both forms — a surface cannot forget it, and
`every_json_report_carries_the_third_shelf` checks every fixture.
Ratios are `f32` `Debug` (`0.98`, `1.0`), a valid JSON number for every
finite value; `TextStats::of` never makes a non-finite one.

### How to extend

- **Another detector** — homoglyphs (E1-4) were the first — is one
  more list merged in `scrub::collect_hits`: the `Replace` path, the
  counter, the JSON and the merge are shared, and the merge's debug
  assertion catches two detectors claiming one code point.
- **A new class** fails to compile in `scrub::replacement`,
  `json::acted_action` and `UnicodeClass::as_str` until it is placed,
  and `every_class_has_a_fixture` demands `<class id>.txt`.
- **A Unicode bump**: fetch, rebuild, and let the conformance tests and
  the fixtures decide; the tables are E1-1's.
- **Per-class overrides** (Q-A1) would make the action of a finding
  depend on more than its class: `the_json_action_is_the_action_clean_took`
  is the test that goes red that day, and `collect_hits`'s `_options`
  parameter is reserved for them.

### What it does not do

- **Positions into the output.** Every position is a byte of the
  source; the output has none.
- **NFKC that knows about code.** With `nfkc`, Layer A normalises
  everything, code blocks included — it cannot see a code block (A §2).
- **A byte-exact way back.** `CleanReport` offers counts and positions
  for every removal, not a reversal.

## Homoglyphs

### What

A homoglyph mark is a letter of one script typed inside a word of
another, where it looks the same: a Cyrillic U+0430 in the English word
"pay", a Latin U+0061 in the Russian word "парк". It is the eleventh
class, and the only one that needs a *word* to exist: `class_of` never
returns `Homoglyph` (a Latin `a` is a homoglyph only next to Cyrillic
letters), so it has its own detector, `homoglyph::hits(text)`, merged
into the hit list by byte offset in `scrub::collect_hits`.

**Detection always runs; replacement needs `aggressive`** (D3). Without
the knob a homoglyph is a row of `kept` at `Probable`, which makes the
text `suspicious` (D4) — a user who did not ask for letters to be
rewritten is still told the text carries look-alikes. With it the letter
is replaced and counted under `normalized` as `homoglyph`. That is why
every precision point below leans towards *not* finding: each false
positive is a "this text is marked" claim to someone who asked for
nothing to change.

The data is E1-1's: `confusable_target(c)` is the one-code-point
skeleton of a letter of Latin, Cyrillic or Greek that `confusables.txt`
(UTS #39) lists as a source, `None` for everything else including a
prototype; `confusables_with(k, script)` is every letter of that script
whose skeleton is `k`, prototype included, ascending. Two letters are
confusable when their skeletons are equal, and the rule uses that in
both directions.

### The two cases

Words and paragraphs first. A **paragraph** runs between U+000A (U+000D,
U+2028 and U+2029 do not end one). A **word** is a maximal run of
letters, marks and decimal digits; characters Layer A removes or keeps
for context (`class_of` is `Some`, exotic spaces excepted) neither end a
word nor belong to it. Only **voting letters** — letters whose script is
not `Common` or `Inherited` — decide anything, and only they are ever
findings. A word's script is the unique majority of its voting letters;
two scripts sharing the maximum make it *tied*.

**The mixed word.** `pаy` = U+0070 U+0430 U+0079 is one word, Latin two
to one. U+0430 is Cyrillic (M1: a script the rule knows), not the
word's script (M2: the word mixes scripts), and has a Latin twin (M3):
U+0061. One finding, at byte 1, replaced by U+0061. Symmetrically, the
Latin `a` in `пaрк` = U+043F U+0061 U+0440 U+043A becomes U+0430.

**The whole word.** `рау` = U+0440 U+0430 U+0443 in "Please рау the
invoice before the end of the month." mixes nothing — every letter is
Cyrillic — and is still an attack: it is drawn entirely in look-alikes
of the paragraph's script. It is redrawn into Latin (U+0070 U+0061
U+0079) when all of W1–W5 hold: the paragraph has a script `S_p` of
Latin, Cyrillic or Greek that is not the word's (W1); every letter
resolves in the word's own script (W2) and has a twin in `S_p` (W3);
the word's script holds under 10 % of the paragraph's voting letters,
counted per word (W4, A §5.4's rule); and every letter not already of
`S_p` is a confusables *source* whose prototype is a letter of `S_p`
(W5, D40). `S_p` itself is the unique largest credit among the words
that have a script and are **not** redrawable — the words a pass might
change are left out of the vote that decides whether to change them.

### Why Russian prose is safe

Nineteen of the 33 lower-case Russian letters have a Latin twin under
D21 and D42 — 35 of the 66 letters counting capitals — and common words
("нет", "как", "все", "он", "мама", "Тариф") are spelled entirely in
them. A detector that asked only "does this letter have a Latin twin?"
would rewrite Russian prose letter by letter; the reference
implementation does. Two conditions stop it here:

- **M2.** A letter of the word's own script is never a finding. A
  Russian word in Russian is one script, so the mixed-word rule has
  nothing to say. Delete M2 and every Cyrillic letter with a Cyrillic
  twin becomes a hit replacing itself —
  `russian_prose_is_not_a_homoglyph_attack` and
  `fullwidth_in_japanese_is_typography` both go red.
- **W5 (D40).** The 10 % share of W4 protects a quotation or a short
  paragraph, not one acronym in a long one: BMW, HTTP, SSH, Java and
  the IPA ɑ in a long Russian paragraph are under 10 % and redrawable
  into Cyrillic. W5 asks whether the word is spelled in *imitations* of
  the paragraph's letters. U+0440 is a source whose prototype is Latin
  `p`, so `рау` is; Latin `B`, `M`, `W` are prototypes, sources of
  nothing, so BMW is not; U+0251 ɑ is a source, but of a *Latin*
  prototype, so it is not an imitation of a Cyrillic letter. This is A
  §5.4's "`pay` in a Russian paragraph is an English word", made
  mechanical.

`fixtures/text/survive-homoglyph-prose.txt` holds all of it — Russian
prose, an English quotation in Russian, BMW and ɑ in long Russian
paragraphs, a Russian quotation in English — and comes out byte-identical
with no row under all 16 `Options`.

### The replacement (D21 revised, D42)

`twin(c, script)`: the candidates are `confusables_with(skeleton(c),
script)` that are letters of **the same case as `c`** (Lu, Ll, or
caseless — matched on the letters, because skeletons drop case: U+0049
and U+0406 have the skeleton U+006C) and have **no decomposition**
(D42, applied before any choice: a letter NFKC would rewrite is never a
replacement, which is what E1-3's convergence argument needs). Then:
the skeleton itself when it is a candidate (T2); else the **lowest code
point** (T3); else no twin and no finding (T4).

The twin is what the eye sees, not what a keyboard would have typed:
U+043A CYRILLIC SMALL LETTER KA becomes U+0138 LATIN SMALL LETTER KRA,
not `k`; U+0432 becomes U+0299 LATIN LETTER SMALL CAPITAL B. A reader
saw "ĸey" and still does.

Where more than one candidate is left, the lowest code point is the
letter in daily use; the others are historic or minority-language
letters added later. The twelve choices among ASCII letters, as the
18.0.0 tables produce them:

| letter | into | twin | not |
|---|---|---|---|
| `c` | Cyrillic | U+0441 | U+1C83 |
| `e` | Cyrillic | U+0435 | U+04BD |
| `i` | Cyrillic | U+0456 | U+A647 |
| `o` | Cyrillic | U+043E | U+1C82 |
| `w` | Cyrillic | U+0448 | U+0461, U+051D |
| `y` | Cyrillic | U+0443 | U+04AF |
| `I` | Cyrillic | U+0406 | U+04C0 |
| `Y` | Cyrillic | U+0423 | U+04AE |
| `o` | Greek | U+03BF | U+03C3 |
| `p` | Greek | U+03C1 | U+03F8 (U+03F1 vetoed by D42) |
| `M` | Greek | U+039C | U+03FA |
| `Y` | Greek | U+03A5 | — (U+03D2 is `<compat> U+03A5`, vetoed by D42) |

For seven letters the prototype is not the lowest candidate, and T2
decides: U+0444, U+03C6 and U+03D5 → U+0278 LATIN SMALL LETTER PHI (not
U+0239 QP DIGRAPH); U+03B5, U+03F5, U+0454 and U+0511 → U+A793 LATIN
SMALL LETTER C WITH BAR (not U+025B OPEN E). Latin `c` and `C` have no
Greek twin at all: their only Greek members, U+03F2 and U+03F9, are
compatibility characters. 488 (letter, other script) pairs resolve in
18.0.0, and `every_twin_is_a_stable_letter_of_the_same_case` holds every
one to "a letter of the target script, same case, no decomposition".

### Where it is more precise than A §5.4

Each point is required by the data or by idempotence (A §5.3:
`clean(clean(x)) == clean(x)`). The counts are strings of the 10,000 of
`homoglyph_replacement_is_idempotent`'s corpus that stop being
idempotent under `aggressive` when the point is removed. (With `nfkc`
too, E1-3's rounds re-run the pass inside one `clean` and absorb them,
which is why the gate runs under `aggressive` alone as well.)

| # | decision | what | why | test that goes red without it |
|---|---|---|---|---|
| H1 | D41 | a removable character inside a word does not split it | U+200B inside `pаy` would hide the mixed word until `clean` removed it, and the second `clean` would find it | `invisible_characters_inside_a_word_do_not_split_it`; corpus 1,398 |
| H2 | D41 | a tie goes to `S_p` only when `S_p` is in the tie; otherwise the word has no script | a word of Latin and Cyrillic letters is not Greek, and a tie settled outside itself moves the majority it was settled by | `a_tie_is_never_settled_by_a_script_the_word_does_not_have`; corpus 1 |
| H3 | D41 | the basis credits whole words to their script, and leaves tied and redrawable words out | replacing a mixed word's minority must not move the paragraph's script | corpus 67 |
| H4 | D41 | the 10 % share counts words, as the basis does | a letter its word outvotes is about to become that word's script | `a_letter_its_word_outvotes_does_not_count_for_its_script`; corpus 161 |
| H5 | D41 | a redraw is judged on resolved letters ρ | a mixed word in a foreign paragraph is redrawn in one pass, not fixed now and redrawn by the next `clean` | `a_mixed_word_in_a_foreign_paragraph_is_redrawn_in_one_pass`; corpus 30 |
| H6 | D40 | W5: a redrawn word is spelled in imitations of the paragraph's letters | W4 alone redraws BMW, HTTP and ɑ in any long Russian paragraph and marks it suspicious | `a_latin_word_in_russian_prose_is_not_redrawn`, `the_survivors_survive_every_option` |
| H7 | D42 | never a replacement with a decomposition, vetoed before the choice | E1-3's rounds converge only if a replaced letter is NFKC-stable | `a_replacement_is_never_a_compatibility_character`, `every_twin_is_a_stable_letter_of_the_same_case` |
| H8 | definition | only voting letters are findings; digits and marks keep a word whole | a digit must never become a letter (U+FF10 has the skeleton U+004F) — but none could resolve anyway, so no test can fail without it | — |
| H9 | definition | a paragraph is the text between U+000A | A §4.2's "абзац", the same paragraph E1-2's RTL rule uses | `a_paragraph_ends_at_a_line_feed` |

Why one pass is a fixpoint of itself: removing a transparent character
changes no word (H1); a replaced letter moves into the script that
already won its word, or into `S_p` for a redraw, so no word's script
moves; `S_p` is computed from words a pass never changes and can only
gain credit; a script's share only falls after a pass, and every word of
that script that could be redrawn was redrawn in the same pass (H4, H5).

### What it deliberately does not do

- **No mark veto.** A replaced letter followed by a combining mark it
  composes with (`voilа` U+0300) is replaced; with `nfkc`, D26's next
  round composes it to U+00E0
  (`a_replacement_that_meets_its_accent_is_composed_by_nfkc`). Refusing
  such letters would only hide a mixed word from the report.
- **No decomposition of precomposed letters.** UTS #39 skeletons run
  NFD first; E1-1 keeps one-code-point targets only, so a Cyrillic ё in
  a Latin word is not found.
- **No dictionary.** `рау` (an attack) and `сор` (a Russian word) look
  alike to the rule; W4 and W5 are the whole of the judgement, and a lone
  Russian word of imitation letters in a long English paragraph *is*
  redrawn, as A §5.4 intends.
- **No script outside Latin, Cyrillic and Greek.** A Han, Arabic or
  Hangul word is never examined, an Armenian `ո` in a Latin word is not a
  finding, and the halfwidth Katakana and Hangul sources are gone with
  the width-form clause (D22). Fullwidth Latin in a Latin word is width,
  which `nfkc` folds, not a homoglyph.
- **No language-aware choice among twins.** D21 takes the lowest code
  point; a Kazakh text would want U+04AF for `y`. That is an owner
  decision.

### Where

| file | what |
|---|---|
| `src/homoglyph.rs` | `hits`, `twin`, the words, the basis, the shares, W1–W5 and M1–M3; the unit tests (D24) |
| `src/scrub.rs` | `collect_hits`: `merge_in_source_order(context::hits(text), homoglyph::hits(text))` |
| `src/tables.rs` | `confusable_target`, `confusables_with` (E1-1) |
| `tests/homoglyphs.rs` | D3 through the API, the fixture under `aggressive`, idempotence over a 10,000-string corpus, the NFKC composition |
| `fixtures/text/homoglyph.txt`, `survive-homoglyph-prose.txt` | the four cases, and the prose that must survive |

### Gates

The unit tests of `src/homoglyph.rs` name every rule above;
`homoglyph_replacement_is_idempotent` (under `aggressive` and
`aggressive` + `nfkc`) is the idempotence gate; E1-3's
`clean_is_idempotent_on_a_generated_corpus`,
`inspect_and_clean_agree` and the fixture suite run with homoglyphs on.
The report `docs/plan/reports/E1-4-2026-10-03.md` records the mutation
that paints each one red.

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
| `NumbersGuard` · `numbers` | every number token of the source is in the candidate, as an exact string, as a set; a canonical placeholder `⟦n⟧` is not a number on either side (E4-7, D95), and two spellings of one value — "1,800" and "1800", "12000" and "12 000" — are two numbers | `NumberMissing { value }`, the first missing in source order | — |
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

## Surfaces

Layer A has two callers, both applications, and both run it on a
thread that is not the GPUI thread: the **MCP server**
(`apps/wipemark-app/src/mcp/protocol.rs`, on the connection thread the
server spawns per request) and the **CLI** (`apps/wipemark-cli/src/`,
`input.rs` → `run.rs` → `report.rs`). Neither re-derives anything core
decided: the exit code, the `suspicious` verdict, the actions and the
JSON are core's, read, never recomputed. The window seams of A §7.4 —
the Compare window's result, the panel's count, the queue's Clean
action — are E7's.

### MCP

Two more tools sit beside these since E11-2 — `inspect_image` and
`clean_image`, which take a picture as base64 in `data` and run
`wipemark-image` over its metadata, never Layer A. Their arguments,
answers and refusals are `docs/architecture/images.md`, "Surfaces"; the
rules below (problems collected, a refusal is a result, never a report)
are theirs too, with `data` where these have `text`.

`tools/call` reads `params.arguments`:

| what arrives | answer |
|---|---|
| absent or `null` | `{}` |
| not an object | protocol error `-32602`, `` tools/call `arguments` must be an object `` |
| an object | checked; any problem is a refusal result |

| tool | `text` | `aggressive` | `nfkc` |
|---|---|---|---|
| `inspect` | string, required | boolean, default `false` | not taken |
| `clean` | string, required | boolean, default `false` | boolean, default `false` |

Problems are collected in a fixed order — `text` missing, `text` not a
string (`null` included), `aggressive` not a boolean, `nfkc` not a
boolean, then every key the tool does not take, sorted — and any one of
them makes the call a **refusal**: a result with `isError: true`, a
sentence naming the tool, each problem and the arguments it does take,
and no `structuredContent`. Never a report: a report of a scan that did
not run would say nothing was found
(`a_tool_that_cannot_run_refuses_rather_than_reporting_nothing`, D14).
An empty `text` is valid — Layer A ran over it. A missing `name` and an
unknown tool stay `-32602`; a report that is not JSON (a bug) is
`-32603`. `normalize_spaces` and `keep_soft_hyphen` are off on every
surface in E1 (Q-A1).

The answers:

| tool | `content[0].text` | `structuredContent` |
|---|---|---|
| `inspect` | `InspectReport::to_json()`, verbatim | that string parsed |
| `clean` | `{"text":<cleaned>,"report":<CleanReport::to_json()>}` | that string parsed |

The text block is assembled from core's string rather than
re-serialized from the parsed value because `serde_json` orders keys
differently in different builds of this workspace (`preserve_order` is
on in some), and the bytes an older client shows a model should not
move. `an_mcp_report_is_json_a_client_can_parse` holds the two equal.

**What is said back.** Nothing the server says comes from the
catalogue (CLAUDE.md), and three answers quote the client — an unknown
method, an unknown tool, an argument the tool does not take — so each
goes through `spelled`: printable ASCII as itself, anything else as
`U+XXXX`. Two things are deliberately said back unspelled: the JSON-RPC
`id`, and the cleaned text of a `clean` result, which carries exactly
what Layer A kept (a ZWJ in an emoji family, a U+FEFF at byte 0) because
it is the user's text. `nothing_the_server_says_carries_an_invisible_character`
permits a forbidden character only where the same response's
`report.kept` declares it (or a leading U+FEFF), and counts it. The
rule is not weakened into "escape everything": a `\u200d` escape would
hide the character from the test and not from the agent, which decodes
JSON. The report itself is ASCII by construction (D29) and carries no
user text.

**The limit** is the transport's (D13): `server.rs` answers a body over
1 MiB with `413` before reading it, never truncating, and the tool
descriptions say "up to about 1 MB". `a_body_over_the_limit_is_refused_whole`
holds both sides of the boundary. The descriptions are ASCII, say
"byte offset", and promise nothing there is no oracle for
(`the_tool_listing_says_what_it_takes_and_promises_nothing_more`); the
schemas carry `additionalProperties: false`, because it is true. No
`outputSchema`: a second description of §7.1 would drift from `json.rs`.

### CLI

`inspect <path|-> [--json]` and `clean <path|-> [-o <out>|-o -]
[--nfkc] [--aggressive] [--json]`.

**Reading** (`input.rs`). A file is opened once and its first
`wipemark_intake::HEAD` (4 KiB) bytes are read first, so a 4 GB model
file named `notes.txt` is refused after 4 KB. Those bytes go to
`wipemark_intake::identify` — the pure function, not `of_path`, which
turns an unreadable file into a name-only answer without an error. Not
textual, or no encoding: not text. `Encoding::Other`: an 8-bit encoding
this version does not name, refused rather than guessed. Otherwise the
rest is read on the same handle and decoded. A zero-byte file is empty
text. Standard input is read whole and identified on its head.

**Encodings** (D11). UTF-8, UTF-16LE/BE and UTF-32LE/BE, with or
without a byte order mark; an invalid sequence is refused at its byte
offset (never decoded lossily). **The mark is never stripped**: it
decodes to U+FEFF at byte 0, which Layer A neither reports nor removes
(A §4.1), and encoding the result in the input's encoding writes the
same mark back — "in the input's encoding, with its BOM if it had one"
without a flag to carry around. Standard output is always UTF-8.

**Where the result goes.** Decided before anything is read: beside the
input as `with_infix(name, RESULT_INFIX)` — the same function the app's
Retention plan uses, in `wipemark_intake::name` (D10) — or the `-o`
file, or standard output (`-o -`, or stdin with no `-o`; `-o -` is E1-6's
generalisation of D12). An `-o` that is a folder, or the input itself
(same device and inode on Unix — a symlink and a hard link included),
is refused with 2. A file is written atomically: a temporary file in
the destination's folder, the input's permissions, then a rename — which
replaces a directory entry and never writes into an existing inode, so
the input is never opened for writing.

**Where each output goes.** Standard output carries one product: the
JSON, the cleaned text, or the human report — which moves to stderr
only when stdout carries the text. On a refusal or a failure stdout is
empty.

| command | `--json` | result to | stdout |
|---|---|---|---|
| `inspect` | no / yes | — | human report / `InspectReport::to_json()` |
| `clean` | no | a file | human report |
| `clean` | no | stdout | the cleaned text (report on stderr) |
| `clean` | yes | a file | `{"report":<§7.1>,"written":"<path>"}` |
| `clean` | yes | stdout | `{"report":<§7.1>,"text":"<cleaned>"}` |

**Exit codes.** `0` read in full and not `suspicious`; `1` read in full
and `suspicious` — **for `clean` too, by the input, even though the
result no longer carries it** (A §7.3: a hook wants to know what was
there; `CleanReport::suspicious` is computed over the source, D28); `2`
a usage error or a refusal (a missing path, a folder, `-o` naming a
folder or the input, and `rewrite`/`models`/`audit`); `3` inconclusive
— unreadable, not text, 8-bit, invalid, the result or stdout could not
be written. "Not read is not clean." A homoglyph found without
`--aggressive` is in `kept`, at `probable`, so the text is suspicious
and the exit is 1 although nothing was replaced; the human report says
so in its own line (`cli-report-homoglyphs-kept`), and never promises
an ASCII replacement — the replacement is the look-alike letter of the
word's own script.

**The human report** (`report.rs`, `Rendering::PlainText`): a summary
that never calls a text clean; rows under *would be removed / replaced
/ kept* (`inspect`) or *removed / replaced / kept* (`clean`), each
`U+XXXX NAME · class · confidence · count, at byte offsets` with at
most ten offsets spelled; the homoglyph line; a note that offsets count
the text as UTF-8 when the input was not; under `--nfkc`, a line saying
NFKC ran and — when the passes after it acted on characters no row
lists — how many, because a row can then list as kept a character a
later pass removed (the one place a count exceeds its positions, E1-3);
where the result went and that the input was not changed; the Unicode
version; and the third shelf. The class and confidence words are
`unicode-class-<id>` and `confidence-<id>`; the name is not translated
(D16, `docs/architecture/i18n.md`).

**Logs.** One line per run, `Elided` for the path — never the text,
never the path in clear, an `io::ErrorKind` and never the OS message
(`nothing_reaches_the_log_but_the_shape`).

### Positions

Byte offsets into the UTF-8 text, everywhere. For a UTF-8 file they are
the file's own offsets, a byte order mark counted
(`positions_are_byte_offsets_into_a_utf8_file`); for UTF-16/UTF-32 input
they count the decoded text as UTF-8, and the human report says so. For
an MCP client they count the text as the server received it, after JSON
decoding — **not** UTF-16 code units, which is what a JavaScript
client's string indices are; the tool descriptions say "byte offsets
into the UTF-8 text".

### The third shelf, in both

Core's writer puts `not_established` into every JSON (D9), so neither
surface can drop it from a machine-read answer; the human report prints
the title and one line per entry of `not_established::ALL` from the
catalogue, falling back to the canonical English beside the id — an
entry is never dropped. `every_layer_a_answer_carries_the_third_shelf`
is the gate in both applications.

### What still refuses

`rewrite`, `models list|pull|verify|rm` and `audit` refuse with exit 2
and `cli-not-implemented`, and their log line still carries the epic.
The MCP server has no tool for Layer B; the pane's banner says that
nothing rewrites in this version, in every state of the server
(`the_mcp_banner_always_says_what_the_tools_do`).

## The gates

Every protection of spec A §8 was painted red by deleting or weakening
it locally before it counted, and the suite was restored byte for byte
afterwards; the full tables — 42 mutations in E1-1 (8 against the build
gates, 34 against tests), 51 in E1-2, 59 in E1-3, 36 in E1-4, 27 in
E1-5, 44 in E1-6 — are in
`docs/plan/reports/E1-n-2026-10-03.md`. The A §8 rows, each with the
test as it landed, the mutation that turned it red and the document
that applied it:

| protection (A §8) | test | mutation | doc |
|---|---|---|---|
| VS16 after an emoji base | `an_emoji_keeps_its_presentation_selector` | FE0E/FE0F kept after any base (M07); read `prev_kept` (M08) | E1-2 |
| ZWJ inside an emoji sequence | `a_family_stays_a_family` | drop the emoji clause (M09); let glue move `prev_kept` (M10) | E1-2 |
| ZWNJ/ZWJ in a joining script | `persian_keeps_its_non_joiner`, `devanagari_keeps_its_joiner` | remove Arabic (M11) / Devanagari (M12) from `JOINING_SCRIPTS` | E1-2 |
| bidi in an RTL paragraph | `a_bidi_mark_is_typography_beside_rtl_and_a_carrier_without_it` | drop the rule (M15); RTL for the whole text (M16) | E1-2 |
| overrides always | `an_override_is_always_removed` | keep U+202D/U+202E like the others (M18) | E1-2 |
| flags | `a_flag_keeps_its_tags_and_a_loose_tag_does_not` | accept any terminator (M19); no base (M20); no 32 limit (M21) | E1-2 |
| IVS and standardized variants | `an_ideograph_keeps_its_variation_sequence`, `a_standardized_variant_is_kept_and_a_random_one_is_not` | drop the Han clause (M24); drop the table lookup (M26) | E1-2 |
| Mongolian, Khmer, Hangul | `a_mongolian_letter_keeps_its_selector`, `khmer_keeps_its_inherent_vowel`, `a_partial_syllable_keeps_its_filler` | one rule each (M27, M28, M30, M31) | E1-2 |
| orthographic `Cf` are not findings | `script_format_controls_are_never_findings` | `DefaultIgnorable` claims every `Cf` (M05) | E1-2 |
| BOM | `a_leading_bom_is_not_a_finding_and_an_inner_one_is` | drop the byte-0 exception (M06) | E1-2 |
| idempotence | `clean_is_idempotent_on_every_fixture`, `clean_is_idempotent_on_a_generated_corpus`, `nfkc_never_leaves_an_orphaned_selector` | no pass after NFKC (S14); one round only (S15, D26) | E1-3 |
| NFKC | `nfkc_conforms_to_the_unicode_test_file` | seven composition and ordering errors (N01–N07), each found by the file | E1-3 |
| version | `every_table_comes_from_one_unicode_version` (with build gate G1), `every_report_names_the_unicode_version` | a 17.0.0 header → `cargo build` red (G1); emit `"17.0.0"`; an empty version in `clean` (F09) | E1-1, E1-3 |
| one class | `every_code_point_has_at_most_one_class` | drop the `DefaultIgnorable` subtraction (M01) | E1-2 |
| positions | `positions_are_byte_offsets_into_the_source` | record the char index (S01) | E1-3 |
| suspicious | `soft_hyphens_alone_are_not_suspicious`, `a_single_zero_width_space_is` | threshold `>= Informational` (S02); `suspicious` from `kept` only (S03, D30 — A's mutation cannot redden the second test, S02b stayed green as predicted) | E1-3 |
| homoglyphs | `russian_prose_is_not_a_homoglyph_attack`, `fullwidth_in_japanese_is_typography`, `a_cyrillic_letter_in_a_latin_word_is`, `a_latin_letter_in_a_cyrillic_word_is`, `nothing_is_replaced_without_aggressive` | delete M2 (U01: the first two red); prototype out of the candidates (U02); T2 only (U03); seam gated on `aggressive` (I01, I02) | E1-4 |
| guards | `a_lost_placeholder_is_rejected`, `an_invented_placeholder_is_rejected`, `a_lost_version_number_is_rejected`, `a_translation_is_rejected_as_script_drift`, `a_short_text_has_no_script_share`, `a_lost_identifier_is_rejected`, `a_faithful_rewrite_passes_every_guard` | one per guard (P1, P2, N1, S1, S2, I1; N5 and I6 for the faithful pair) | E1-5 |
| third shelf | `every_layer_a_answer_carries_the_third_shelf` (MCP and CLI) | strip it from the MCP value (M08), from the core writer (M09), from the CLI's loop (M29) | E1-6 |
| MCP | `nothing_the_server_says_carries_an_invisible_character`, `a_tool_that_cannot_run_refuses_rather_than_reporting_nothing` | echo a tool name unspelled (M11); remove a declared exception (M12); read a missing `text` as `""` (M01) | E1-6 |
| CLI | `a_file_with_a_zero_width_space_exits_one`, `an_unreadable_encoding_exits_three_not_zero`, `clean_writes_beside_the_file_and_never_over_it` | exit by the output (M22); decode 8-bit as Latin-1 (M23); drop the same-file check (M24) | E1-6 |

Two protections stayed green under their mutation and were dealt with
rather than kept: E1-3's `mixed` term of the NFKC count could not be
made to fail and was removed (D27 as built), and A §8's "compare with
`Informational`" cannot redden `a_single_zero_width_space_is` (D30).
Two are held by unit tests only: W4 and D42 (E1-4, U12 and U08) — both
corpora stay idempotent without them (*What is left open*).

Beyond the tests, the closure checked: the six commands of the
repository's gates with `--locked`, and `RUSTDOCFLAGS='-D warnings'
cargo doc -p wipemark-core --no-deps`; `wipemark-core -> (none)` in
`check-dep-direction.sh`; no module-wide `dead_code` allowance in the
crate but the documented one in `tables.rs`.

## The live gate

What no test takes (spec A §8), run once at the closure on 2026-10-03,
on Linux x86_64 against the debug build, in a scratch
`WIPEMARK_DATA_DIR`:

- **CLI.** A file whose U+200B came through the clipboard (the X11
  CLIPBOARD selection; `pbcopy`/`pbpaste` on a Mac) —
  `48 65 6c 6c 6f e2 80 8b 77 6f 72 6c 64 0a`. `inspect` exits 1 with
  the row `U+200B ZERO WIDTH SPACE · zero-width character · confirmed ·
  once, at byte 5`, `Checked against Unicode 18.0.0.` and the three
  shelf lines. `clean` exits 1, writes `note.cleaned.md`
  (`Helloworld\n`) beside the file and leaves the file's SHA-256 as it
  was; `inspect` of the result exits 0. Every `jq -e` over `--json` is
  true (`written`, `positions == [5]`, `removed["zero-width"] == 1`, the
  three `not_established` ids); stdin to stdout gives `Helloworld\n` with
  the report on stderr. A missing path exits 2, a KOI8-R file 3, `-o`
  naming the input 2 (the input untouched), `rewrite` 2. The CLI's log
  holds `input=<elided …>` and neither the text nor the file name.
- **MCP.** The application, launched once with the server switched on,
  answers on `127.0.0.1:5056`: `initialize` (`2025-06-18`, `wipemark`),
  `notifications/initialized` → 204, `tools/list` → `inspect`, `clean`,
  both saying "1 MB"; `clean` of `"Hello\u200bworld"` → `isError: false`,
  text `Helloworld`, `removed["zero-width"] == 1`, `positions == [5]`,
  the text block parsing to `structuredContent`; `inspect` → suspicious
  and no `text`; `clean` with `{}` → an `isError` result naming
  `` `text` ``; a 1.1 MB body → HTTP 413.
- **Claude Code** (2.1.288), given the pane's own snippet for one
  non-interactive session, called `mcp__wipemark__clean` once and printed
  text and a report — `Helloworld`, `removed["zero-width"] == 1`, three
  shelf ids — not a refusal; the application's log shows the call as
  `tools/call answered tool="clean" text=<elided chars=11 bytes=13>`,
  so the U+200B reached the server. Claude Code printed
  `structuredContent` re-serialised (`"latin_ratio":1` where the server
  wrote `1.0`): a client may show the object rather than the text block.
- **The windows** read as they should: the MCP page's banner, the
  queue's footer, the panel and the Compare banner say that cleaning
  from the windows is not in this version and runs from the command line
  and over MCP; the queue row for `note.md` is Text · Markdown · UTF-8.
  The process was killed and port 5056 was free afterwards.

The transcript is in `docs/plan/reports/E1-7-2026-10-03.md` and in the
closure (Watchword TEXT `wipemark-core-layer-a-closed-2026-10-03`).

## What is left open

Owner questions of spec A §9 (`docs/plan/README.md` §5):

- **Q-A1** — per-class overrides and a "Clean" Settings page: open, E7/E8.
  Every surface runs with `Options` from its own flags; no preference
  rows. `collect_hits`'s `_options` is reserved for it, and
  `the_json_action_is_the_action_clean_took` is the test that changes.
- **Q-A2** — pairing of bidi embeddings in an RTL paragraph: open,
  decided on real files.
- **Q-A3** — unassigned code points: open, E7 (the Inspector could show
  them as "unknown to this version").
- **Q-A4** — the MCP text limit: **answered** by the transport, a 413 at
  1 MiB before the body is read, never a truncation (D13).
- **Q-A5** — `arabic_ratio`/`hebrew_ratio`: open, E4.
- **Q-A6** — character names as an exception to the i18n rule:
  **taken** as D16 and kept by the owner; on a veto the windows show
  only `U+XXXX` and the class, and `name_of` stays for `--json` and MCP.
- **Q-A7** — the known false positives left unprotected (Malayalam
  chillu, U+034F, German ligature-breaking ZWNJ) and **Q-A8** (tags
  only in flag sequences, D39): kept by the owner; each is one rule when
  a real document shows it matters.

Raised by the E1 documents:

- **Homoglyph replacements are look-alikes, not ASCII** (D21, E1-4):
  Greek β and ϐ inside a Latin word become ß (U+00DF), Cyrillic к
  becomes ĸ (U+0138 KRA). The rule and the data give that; whether it is
  acceptable is an owner question. The CLI says "the look-alike letter
  of its word's own script" and never promises ASCII.
- **W4 and D42 are held by unit tests alone** (E1-4): both corpora stay
  idempotent with either removed, because neither alphabet puts a Latin
  `c`/`C` in a Greek word. D26's premise — replacements are NFKC-stable
  — therefore rests on `every_twin_is_a_stable_letter_of_the_same_case`
  and `a_replacement_is_never_a_compatibility_character`; a corpus atom
  would harden it.
- **The H1–H5 gate must keep running under `aggressive` alone**: under
  `aggressive + nfkc`, D26's rounds re-run the pass inside one `clean`
  and hide a pass that is not idempotent.
- **For E4** (E1-5): `Event::CandidateRejected` should carry the
  structured `RejectReason`, not its English `Display`; `RejectReason`
  is not `#[non_exhaustive]`, so a variant added later breaks an
  exhaustive `match` outside the crate; run Layer A over a candidate
  before its guards.
- **JSON ratios print with full `f32` precision** (`0.94736844`, not
  `0.95`): the shortest round-trip form, valid JSON, noisy to read.
- **D19's list could move into `tables.rs`**, so that `name_of` would be
  exact for the twelve Duployan and musical format controls instead of
  naming a superset (E1-1).
- **A count can exceed its positions under `nfkc`** (E1-3, "kept 5,
  removed 2"): the CLI explains it in its own line (`cli-clean-later`);
  a window that shows a report (E7) has to as well.
- **Positions are UTF-8 byte offsets on every surface** — for an MCP
  client too, which may index strings in UTF-16 units; the tool
  descriptions say so, and the Inspector (E7) has to convert.
