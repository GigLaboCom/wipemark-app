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
