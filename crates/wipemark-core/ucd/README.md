# Unicode Character Database 18.0.0

These are files of the Unicode Character Database (and UTS #39's
`confusables.txt`), version **18.0.0**, copied verbatim by
`scripts/fetch-ucd.sh`. `crates/wipemark-core/build.rs` turns them into
the tables `src/tables.rs` searches. They are never edited by hand: an
edit is invisible in review (`.gitattributes` marks them `-diff`) and is
caught by the checksum test, `every_ucd_file_matches_its_checksum`,
instead.

| file | from `https://www.unicode.org/Public/18.0.0/` | bytes | read for |
|---|---|---|---|
| `UnicodeData.txt` | `ucd/UnicodeData.txt` | 2,243,593 | name (finding-capable only), General_Category, Canonical_Combining_Class, Bidi_Class R/AL, decompositions, the `First`/`Last` ranges |
| `DerivedCoreProperties.txt` | `ucd/DerivedCoreProperties.txt` | 1,159,889 | `Default_Ignorable_Code_Point` |
| `PropList.txt` | `ucd/PropList.txt` | 149,040 | `Bidi_Control`, `Variation_Selector`, `Join_Control`, `Noncharacter_Code_Point` |
| `Scripts.txt` | `ucd/Scripts.txt` | 196,089 | `Script`, folded into the 56 values of `Script` |
| `StandardizedVariants.txt` | `ucd/StandardizedVariants.txt` | 77,478 | the (base, selector) pairs |
| `DerivedNormalizationProps.txt` | `ucd/DerivedNormalizationProps.txt` | 1,396,877 | `Full_Composition_Exclusion` |
| `NormalizationTest.txt` | `ucd/NormalizationTest.txt` | 2,863,708 | line 1 only, for the version gate; the body is NFKC's conformance test input |
| `emoji/emoji-data.txt` | `ucd/emoji/emoji-data.txt` | 108,719 | `Emoji`, `Emoji_Presentation`, `Emoji_Modifier`, `Emoji_Modifier_Base`, `Emoji_Component` |
| `confusables.txt` | `security/confusables.txt` | 763,128 | source, skeleton and type of every line |

## How the version is established

Eight of the nine files name their version: line 1 of six of them is
`# <stem>-18.0.0.txt`, and `emoji/emoji-data.txt` and `confusables.txt`
carry `# Version: 18.0.0` in their leading comments. `build.rs` reads
all eight, fails the build when they disagree, and emits the common
value as `wipemark_core::UNICODE_VERSION` — nobody types it.

`UnicodeData.txt` names no version anywhere. It is bound to the others
by a cross-check: the code points it assigns, other than private use and
surrogates, must be exactly the code points `Scripts.txt` lists (172,873
in 18.0.0). A `UnicodeData.txt` of another release fails that check as
soon as the two releases assign different code points — 17.0.0 and
18.0.0 differ by 13,007. Its one blind spot is a release that assigns no
new code point: that would pass.

## Verify

```sh
cd crates/wipemark-core/ucd && shasum -a 256 -c SHA256SUMS   # or: sha256sum -c SHA256SUMS
```

## Bump

```sh
scripts/fetch-ucd.sh <new version>
# update the version in this README
cargo test -p wipemark-core
```

A red suite after a bump means Unicode reassigned or reshaped something
the code relied on — read the failure; do not edit the test to match.

## Licence

The Unicode License v3, reproduced in the repository's `NOTICE`.
