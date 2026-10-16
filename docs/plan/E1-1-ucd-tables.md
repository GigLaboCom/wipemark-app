# E1-1 — The Unicode tables: UCD 18.0.0 committed, a std-only build.rs, and the lookups every later document reads

|                  |                                                                                                                                                                                                                                                                                                                      |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic **E1**, Layer A (the deterministic Unicode scrubber)                                                                                                                                                                                                                                             |
| Spec scopes      | **S1.1** — A §3 (§3.1 the committed files, §3.2 `build.rs` and the generated tables, §3.3 the version in the report)                                                                                                                                                                                                  |
| Depends on       | nothing — the first document of the series; starts from `497eafa` on `feat/e0-e6-shell`                                                                                                                                                                                                                              |
| Unblocks         | **E1-2** (the classifier reads the predicates and `script_of`), **E1-3** (NFKC reads `decomposition`, `ccc`, `compose`; the reports read `UNICODE_VERSION` and `name_of`), **E1-4** (`confusable_target`, `confusables_with`), **E1-5** (letter shares through `is_letter` and `script_of`)                           |
| Files touched    | new: `crates/wipemark-core/build.rs`, `crates/wipemark-core/ucd/**` (9 data files, `README.md`, `SHA256SUMS`), `crates/wipemark-core/src/{tables,script,name}.rs`, `crates/wipemark-core/tests/ucd_files.rs`, `scripts/fetch-ucd.sh`, `.gitattributes`, `docs/architecture/layer-a.md`, `docs/plan/reports/E1-1-<date>.md`; edited: `crates/wipemark-core/Cargo.toml`, `crates/wipemark-core/src/lib.rs`, `NOTICE`, `docs/README.md` (one row), `docs/plan/README.md` (status row) |
| Size             | ~2 days for one agent; the network is needed once, to run `scripts/fetch-ucd.sh 18.0.0`                                                                                                                                                                                                                              |

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

Layer A's whole claim is that a finding is *verifiable*: a code point
either has a Unicode property or it does not, and the report names the
Unicode version that says so (A §0, A §3.3; OV §0.1 rule 3). Today
`wipemark-core` knows no property of any code point. This document puts
that knowledge in, and nothing else:

- the nine Unicode 18.0.0 data files, committed byte for byte under
  `crates/wipemark-core/ucd/` with their checksums;
- `scripts/fetch-ucd.sh <version>`, which downloads exactly those files
  and writes `SHA256SUMS`, so a version bump is one command and a green
  suite;
- a `build.rs` written against `std` alone that parses them into sorted
  static tables in `$OUT_DIR/tables.rs`, and **fails `cargo build`** when
  the files come from two Unicode versions, when a line cannot be parsed,
  or when a table comes out empty;
- the lookups `docs/plan/README.md` §3.3 fixes for E1-1 — twenty
  predicates, `script_of`, the three NFKC tables, the confusables in both
  directions — plus the `Script` enum, `name_of` and `UNICODE_VERSION`.

Nothing here looks at text. Every later E1 document reads the UCD
through these functions and through nothing else.

Done looks like this: `wipemark_core::UNICODE_VERSION` is `"18.0.0"`
and no person typed it; `wipemark_core::name_of('\u{200B}')` is
`Some("ZERO WIDTH SPACE")` and `name_of('\u{0436}')` (CYRILLIC SMALL
LETTER ZHE, which can never be a finding) is `None`; the generated data
is about 200 KB rather than the 2 MB that every name in UnicodeData would
cost; every predicate has been compared with an independent reading of
its file over all 1,114,112 code points; and dropping a 17.0.0 file into
`ucd/` turns `cargo build` red.

## §2 Read first

- **`CLAUDE.md`** — "Rules that are not visible in the code": the first
  two bullets (zero dependencies; dependency direction), "The third shelf
  is never empty", "Every string a person reads comes from the
  catalogue". "Working here" → "Delete the protection and watch it go
  red". "Gates".
- **`docs/plan/README.md`** — §3.2 (module map) and §3.3 (the E1-1
  block is your contract, signatures and visibility included); §4
  decisions **D7** (`name_of` returns `Cow`, code point labels), **D8**
  (`Script` has `Bopomofo`), **D16** (names are not catalogue strings),
  **D17** (fixtures byte-exact), **D18** (the version comes from the file
  headers; two versions fail the build); §6 risk "Table size".
- **`crates/wipemark-core/src/lib.rs:1-39`** — the whole file. You add
  three modules and two re-exports and update the crate doc at `:10-25`.
- **`crates/wipemark-core/Cargo.toml:1-16`** — no dependency section of
  any kind, and the comment at `:11-13` that promises this document.
- **`crates/wipemark-core/src/class.rs:43-150`** — the eleven classes the
  tables exist to serve. You do not change this file; E1-2 builds
  `class_of` on your predicates, so read it to know why each predicate
  is there.
- **`crates/wipemark-core/src/report.rs:62-73`** — `unicode_version` on
  `InspectReport` (`:72`), the field `UNICODE_VERSION` will fill (E1-3).
- **`crates/wipemark-i18n/build.rs:37-93`** — the `build.rs` idiom this
  repository uses: `rerun-if-changed` on the directory and on
  `build.rs` (`:40-41`), each input named (`:65`), a file in `OUT_DIR`
  (`:73-74`), and a parse error turned into a panic that names the path
  (`:81-93`). It uses `fluent-syntax` as a build-dependency
  (`crates/wipemark-i18n/Cargo.toml:19-23`); **you may not** — yours is
  `std` only. `crates/wipemark-i18n/src/lib.rs:76` is the `include!`
  line to copy.
- **`scripts/check-dep-direction.sh:50`, `:119-137`** — `wipemark-core`
  is allowed nothing, and all three dependency tables are read, so a
  `[build-dependencies]` entry fails the gate.
- **Root `Cargo.toml:137-160`** — the workspace lints. `[lints]
  workspace = true` (`crates/wipemark-core/Cargo.toml:15-16`) applies them
  to `build.rs` as well, and `cargo clippy --all-targets` lints the build
  script: watch `uninlined_format_args` (`:154`), `cast_lossless`
  (`:138`), `manual_let_else` (`:149`), `unnecessary_wraps` (`:155`).
- **`scripts/promote-icon.sh:1-70`** — the conventions for a script here:
  the header comment is the `--help` text, `ROOT` is resolved from the
  script's own path, usage errors exit 2.
- **`NOTICE`** (53 lines) — the section format you append to.
- **`fixtures/README.md:20-27`** — the byte-exact rule the
  `.gitattributes` you create also serves.
- **`crates/wipemark-i18n/src/tests.rs:316-336`** — `stripped_by_layer_a`,
  a deliberately independent spelling of the removal set. You do not touch
  it; it is the style of the "independent reading" tests in §5.
- **UAX #44** (<https://www.unicode.org/reports/tr44/>, revision 38 for
  18.0.0) — §4.2 file format conventions: §4.2.3 code point ranges (the
  `First`/`Last` entries of `UnicodeData.txt`), §4.2.5 code point labels,
  §4.2.10 `@missing`; §5.7.3 and its Table 14 (the compatibility
  formatting tags). **The Unicode Standard §4.8 "Name"** — names and
  labels.
  **UTS #39** §4 (confusables, skeleton). **UTS #51** §1.4.1 (emoji
  properties). **UAX #15** §1.3 and §5 (decomposition, composition
  exclusions).

## §3 What is true today (verified 2026-10-03 at `497eafa`)

### 3.1 The code

- `crates/wipemark-core/` holds `Cargo.toml` and `src/` only — 666 lines
  across `Cargo.toml` (16), `src/lib.rs` (39), `src/class.rs` (219),
  `src/guard.rs` (131), `src/report.rs` (167), `src/vendor.rs` (94).
  There is no `build.rs`, no `ucd/`, no `tests/`.
- `crates/wipemark-core/Cargo.toml:11-13`: "NO [dependencies] SECTION.
  … The UCD tables land here as build.rs-generated statics (epic E1),
  not as a crate." There is no `build` key in `[package]`.
- `crates/wipemark-core/src/lib.rs:10-19` explains zero dependencies and
  names four tables (`Default_Ignorable_Code_Point`,
  `Emoji_Presentation`, `Emoji_Modifier_Base`, `confusables.txt`) — it
  will be stale after this document. `:21-25` says "This is epic **E0**:
  the types are here, the logic is not." `:27` is
  `#![forbid(unsafe_code)]`. `:29-32` declare `class`, `guard`, `report`,
  `vendor` as `pub mod`; `:34-39` re-export from them.
- `crates/wipemark-core/src/class.rs:43-71` is `UnicodeClass`; its doc
  comments carry the E0 range notes (E1-2 owns them). No function in the
  crate takes a `char` or a `&str`.
- `crates/wipemark-core/src/report.rs:72` is `pub unicode_version:
  &'static str` on `InspectReport`, documented "Pinned at build time and
  printed in every report"; nothing fills it. `CleanReport`
  (`:88-93`) has no version yet — E1-3 adds it.
- `scripts/check-dep-direction.sh:50` gives `wipemark-core` the empty
  set; `:120` reads `dependencies`, `dev-dependencies` and
  `build-dependencies`; `:133-137` fails on any of them.
- `crates/wipemark-i18n/build.rs` and `crates/wipemark-log/build.rs:24-30`
  are the two existing generators; both panic with the path on a read
  error. `crates/wipemark-i18n/src/lib.rs:76` and
  `apps/wipemark-app/src/icon.rs:29` are the two
  `include!(concat!(env!("OUT_DIR"), "/…"))` lines.
- `.woodpecker/gate.yaml:74` formats `$(find crates apps -name '*.rs')`:
  your `build.rs` and tests are formatted and checked; the generated file
  lives in `target/` and is not.
- `NOTICE` has two sections, "Icons" (`:9-38`) and "UI framework"
  (`:40-53`), each under a dashed rule. Nothing covers Unicode data.
- There is **no `.gitattributes`** in the repository. `fixtures/`
  contains only `README.md`. `.gitignore` ignores nothing under
  `crates/wipemark-core/ucd/` (`git check-ignore` exits 1 for
  `crates/wipemark-core/ucd/UnicodeData.txt`).
- `apps/wipemark-cli/Cargo.toml:16` depends on `wipemark-core`, but
  nothing under `apps/wipemark-cli/src/` names `wipemark_core`; no
  binary calls into the crate's tables until E1-6, so the release binary
  size will barely move in this document (§4.14 says what to record).
- `docs/README.md:3-22` is the docs index (the working tree adds an
  uncommitted `plan/README.md` row at `:5`); there is no
  `docs/architecture/layer-a.md`.

### 3.2 The Unicode 18.0.0 files (fetched 2026-10-03)

Every URL below answered `HTTP/2 200` (`text/plain; charset=utf-8`,
`last-modified: Tue, 01 Sep 2026`). No file starts with a byte order
mark, no file contains a carriage return, and every file is valid UTF-8
(non-ASCII occurs only in comments: the `©` line of each header, the
glyphs in `emoji-data.txt` and `confusables.txt` comments).

| file (under `ucd/`) | URL | bytes | sha256 | version evidence |
|---|---|---|---|---|
| `UnicodeData.txt` | `https://www.unicode.org/Public/18.0.0/ucd/UnicodeData.txt` | 2,243,593 | `0736451de439ae7baf1425136617da495e09ee5afbe6e394374db7009ea08950` | **none** — line 1 is `0000;<control>;Cc;0;BN;;;;;N;NULL;;;;` |
| `DerivedCoreProperties.txt` | `…/18.0.0/ucd/DerivedCoreProperties.txt` | 1,159,889 | `09c928886a178fcafd93c29e4bd59073a058e5a100b716d425cb563ab50f68c9` | line 1 `# DerivedCoreProperties-18.0.0.txt` |
| `PropList.txt` | `…/18.0.0/ucd/PropList.txt` | 149,040 | `f438f532e8737bb8a2702126cdf9c4af5e357c58c7acf9d9eb2fc7c1a1d955d6` | line 1 `# PropList-18.0.0.txt` |
| `Scripts.txt` | `…/18.0.0/ucd/Scripts.txt` | 196,089 | `0071fd81b6aeae25f6e8bce8efec3066a6476a91b49bdb2f52dc76e817862a6a` | line 1 `# Scripts-18.0.0.txt` |
| `StandardizedVariants.txt` | `…/18.0.0/ucd/StandardizedVariants.txt` | 77,478 | `c7ae634a7e2bb0932258548a1e81df984fbb38e30cecf4269391d6a4f94581ac` | line 1 `# StandardizedVariants-18.0.0.txt` |
| `DerivedNormalizationProps.txt` | `…/18.0.0/ucd/DerivedNormalizationProps.txt` | 1,396,877 | `98ac7f67d985fe781e317f6182e885e94cabb0c314769e6dd73e48b226931ccd` | line 1 `# DerivedNormalizationProps-18.0.0.txt` |
| `NormalizationTest.txt` | `…/18.0.0/ucd/NormalizationTest.txt` | 2,863,708 | `25a50d816764b04abfb4a646d3eb2b2a803284c3873d9a06757b94fe4513dde3` | line 1 `# NormalizationTest-18.0.0.txt` |
| `emoji/emoji-data.txt` | `…/18.0.0/ucd/emoji/emoji-data.txt` | 108,719 | `80d00f8e616a0ef27fd6b8de3b758c06383b5d917e2977709578e68baf733bf1` | line 1 `# emoji-data.txt`; line 8 `# Version: 18.0.0` |
| `confusables.txt` | `https://www.unicode.org/Public/18.0.0/security/confusables.txt` | 763,128 | `6ed3ee967c9dfdf6677d563c9985182fbc50a2efb7d6059cd57b2e2ce18f5b92` | line 1 `# confusables.txt`; line 8 `# Version: 18.0.0` |

Total 8,958,521 bytes. If your download hashes differently, unicode.org
re-published a file: stop and report it rather than committing.

The security files moved with 17.0.0 (A §3.1 is right):
`Public/17.0.0/security/confusables.txt` and
`Public/security/16.0.0/confusables.txt` answer 200;
`Public/security/17.0.0/confusables.txt`,
`Public/security/18.0.0/confusables.txt` and
`Public/16.0.0/security/confusables.txt` answer 404.

### 3.3 Facts in the data that shape the tables

1. **`UnicodeData.txt` names no version anywhere.** A §3.2 expects a
   `# UnicodeData-18.0.0.txt` header; the file has none (no comment line
   at all). Its 41,341 lines all have exactly 15 `;`-separated fields.
   §4.8.7 binds it to the others another way: the code points it assigns
   (excluding `Co` and `Cs`) are exactly the code points `Scripts.txt`
   lists — 172,873 in 18.0.0. With 17.0.0's `UnicodeData.txt` beside
   18.0.0's `Scripts.txt`, 13,007 code points differ.
2. **22 `First`/`Last` pairs** in `UnicodeData.txt`: CJK Ideograph
   Extension A (3400..4DBF), CJK Ideograph (4E00..9FFF), Hangul Syllable
   (AC00..D7A3), three surrogate ranges (D800..DFFF), Private Use
   (E000..F8FF), Tangut Ideograph (17000..187FF), Tangut Ideograph
   Supplement (18D00..18D20), Jurchen Character (18E00..19191), CJK
   Ideograph Extensions B, C, D, E, F, I, G, H, J (20000..2A6DF,
   2A700..2B73F, 2B740..2B81E, 2B820..2CEAD, 2CEB0..2EBE0, 2EBF0..2EE5D,
   30000..3134A, 31350..323AF, 323B0..33479), Seal Character
   (3D000..3FC3F), Plane 15 and Plane 16 Private Use (F0000..FFFFD,
   100000..10FFFD). Each pair carries the same `gc`, `ccc` and bidi class
   on both lines; none has a decomposition. Compared with 17.0.0, the
   Jurchen and Seal ranges are new and the Tangut Ideograph Supplement
   and CJK Extension D ranges grew — the parser must recognise the
   pattern, never a list of names.
3. In field 1 (Name), 65 lines read `<control>` (their Name property is
   empty — UAX #44); 44 are `First`/`Last`; every other name uses only
   `A–Z`, `0–9`, space and hyphen.
4. Field 2 uses exactly the 29 two-letter `General_Category` values; field
   4 uses exactly the 23 `Bidi_Class` values (`L R AL EN ES ET AN CS NSM
   BN B S WS ON LRE LRO RLE RLO PDF LRI RLI FSI PDI`); field 5 uses exactly
   the 16 decomposition tags of UAX #44 Table 14 (`<font> <noBreak>
   <initial> <medial> <final> <isolated> <circle> <super> <sub>
   <vertical> <wide> <narrow> <small> <square> <fraction> <compat>`).
5. **Property file shapes** (comment stripped at the first `#`, fields
   split on `;`): `PropList.txt` 1,779 data lines, all 2 fields;
   `Scripts.txt` 2,321, all 2; `emoji/emoji-data.txt` 1,243, all 2 (single
   code points or ranges only — sequences live in other emoji files);
   `DerivedCoreProperties.txt` 13,086 with 2 fields and 515 with 3 (the
   `InCB` values); `DerivedNormalizationProps.txt` 1,823 with 2 and 14,561
   with 3, 54 of which have an **empty** third field (`00AD ; NFKC_CF; #
   …` — a mapping to the empty string).
6. **`@missing` lines** exist in `Scripts.txt` (`# @missing:
   0000..10FFFF; Unknown`), `DerivedCoreProperties.txt` (one, for
   `InCB`) and `DerivedNormalizationProps.txt` (six, for `NFD_QC`,
   `NFC_QC`, `NFKD_QC`, `NFKC_QC`, `NFKC_CF`, `NFKC_SCF`). None names a
   property `build.rs` reads, and `Scripts.txt`'s says what "not listed"
   already means.
7. **`Default_Ignorable_Code_Point`**: 4,174 code points in 17 ranges —
   00AD, 034F, 061C, 115F..1160, 17B4..17B5, 180B..180F, 200B..200F,
   202A..202E, 2060..206F (2065 is reserved), 3164, FE00..FE0F, FEFF,
   FFA0, FFF0..FFF8 (reserved), **1BCA0..1BCA3, 1D173..1D17A**,
   E0000..E0FFF (E0000, E0002..E001F, E0080..E00FF, E01F0..E0FFF
   reserved). The bold two are `Cf` *and* default-ignorable in 18.0.0;
   A §4.1 lists them among "Cf not marked Default_Ignorable — never
   findings". **D19** settles it: they are never findings, excluded by
   name from `DefaultIgnorable` — in E1-2's `class_of`. Here the table
   keeps the property as published, and `name_of` still names them
   (§4.11 says why).
8. Exact sets later documents rely on: `gc=Zs` is 0020, 00A0, 1680,
   2000..200A, 202F, 205F, 3000 (17; A §1's claim holds);
   `Bidi_Control` is 061C, 200E, 200F, 202A..202E, 2066..2069 (12);
   `Variation_Selector` is 180B..180D, 180F, FE00..FE0F, E0100..E01EF
   (260); `Join_Control` is 200C, 200D; `Noncharacter_Code_Point` is
   FDD0..FDEF plus the last two code points of each of the 17 planes (66);
   `gc=Co` is E000..F8FF, F0000..FFFFD, 100000..10FFFD (137,468).
9. **`Bidi_Class` R or AL** covers 3,072 code points, and two of them are
   bidi controls: U+200F RIGHT-TO-LEFT MARK is `R` and U+061C ARABIC
   LETTER MARK is `AL` — which is why **D20** has E1-2 count only an R/AL
   letter or mark towards an RTL paragraph. U+0640 ARABIC TATWEEL is `AL`
   with `Script=Common`; U+0661 ARABIC-INDIC DIGIT ONE is `AN`, not RTL.
10. **`StandardizedVariants.txt`**: 1,437 distinct two-code-point
    sequences (A §3.1 says 12 KB; it is 77 KB), selectors FE00..FE06 and
    180B..180D, 180F only — **no** FE0E/FE0F pairs (emoji presentation
    sequences live in `emoji-variation-sequences.txt`, which is not
    committed; E1-2 keeps them by `Emoji=Yes` instead). 1,002 of the bases
    are Han (CJK compatibility variants). Every data line has 3 fields;
    the third (shaping environments) is often empty.
11. **`confusables.txt`**: 6,712 entries, each `source ; target ; MA`
    with exactly one source code point; 4,253 have a one-code-point
    target. No one-code-point target is itself a source (a prototype maps
    to itself). Filtered as §4.8.8 prescribes (D22), 358 sources remain,
    all letters: 209 Latin (51 of them fullwidth Latin, U+FF21..U+FF5A,
    which is `Script=Latin`), 91 Cyrillic, 58 Greek. A §3.2's extra clause
    "or from the Halfwidth and Fullwidth Forms block" would have added 82
    more (55 halfwidth Katakana, 25 halfwidth Hangul, 2 `Common` — width
    variants of letters of their own script, and oddities such as U+FF89
    HALFWIDTH KATAKANA LETTER NO whose skeleton is U+002F SOLIDUS); D22
    drops it. 135 prototypes pass the same filter — 493
    "homoglyph-capable" code points in all, of which 129 have a
    decomposition (U+FF41, U+03F2, U+017F …; D42, §4.9). Skeletons do not
    keep case: U+0049 LATIN CAPITAL LETTER I maps to U+006C LATIN SMALL
    LETTER L.
12. **Normalization**: 5,982 code points have a decomposition (2,081
    canonical, 3,901 compatibility); the raw mappings nest at most three
    deep; the longest full expansion is U+FDFA (18 code points); no full
    expansion contains a Hangul syllable (AC00..D7A3) and no Hangul
    syllable has a mapping; every canonical mapping has one or two code
    points. `Full_Composition_Exclusion` has 1,120 code points; 961
    two-code-point canonical mappings remain as primary composites, and
    none of them starts with a non-starter. 1,002 code points have a
    non-zero `Canonical_Combining_Class`, 422 ranges, 55 distinct values.
13. `NormalizationTest.txt` has six parts, `@Part0` … `@Part5` (Part 4
    "Canonical closures (excluding Hangul)", Part 5 "Chained primary
    composites"); A §5.3 mentions parts 0–3. **D23**: E1-3's conformance
    gate runs all six.
14. `Scripts.txt` uses 177 script values; all 55 long names the `Script`
    enum keeps (§4.10) occur in it, spelled `Nko`, `Oriya`,
    `Hanifi_Rohingya`, `Old_Uyghur`, `Meetei_Mayek`, `Kayah_Li`,
    `Tai_Tham`, `Tai_Viet`, `New_Tai_Lue`, `Inherited`, `Common`.

### 3.4 What does not exist yet

`crates/wipemark-core/build.rs`, `crates/wipemark-core/ucd/`,
`crates/wipemark-core/src/{tables,script,name}.rs`,
`crates/wipemark-core/tests/`, `scripts/fetch-ucd.sh`, `.gitattributes`,
the Unicode section of `NOTICE`, `docs/architecture/layer-a.md`,
`docs/plan/reports/`.

## §4 Deliverables

### 4.1 `crates/wipemark-core/ucd/` — the nine files

Created by running `scripts/fetch-ucd.sh 18.0.0` (§4.4), never by hand,
never edited afterwards. Flat, plus `emoji/`:

| file | what `build.rs` reads from it | used by |
|---|---|---|
| `UnicodeData.txt` | field 1 Name (for `NAME` only), field 2 `General_Category` (L*, Lu, Ll, M*, Nd, Zs, Cf, Co, and Cs to skip), field 3 `Canonical_Combining_Class`, field 4 `Bidi_Class` (R, AL), field 5 decomposition type and mapping; the `First`/`Last` pairs | predicates, `CCC`, `DECOMPOSITION`, `COMPOSITION`, `NAME`, the cross-check |
| `DerivedCoreProperties.txt` | `Default_Ignorable_Code_Point` — nothing else | `DEFAULT_IGNORABLE`, `NAME` coverage |
| `PropList.txt` | `Bidi_Control`, `Variation_Selector`, `Join_Control`, `Noncharacter_Code_Point` | four predicates, `NAME` coverage |
| `Scripts.txt` | the `Script` value of every listed range | `SCRIPT`, `HAN`, the confusables filter, the cross-check |
| `StandardizedVariants.txt` | field 0, the (base, selector) pair | `STANDARDIZED_VARIANTS` |
| `DerivedNormalizationProps.txt` | `Full_Composition_Exclusion` — nothing else | `COMPOSITION` |
| `NormalizationTest.txt` | **line 1 only**, for the version gate; its body is E1-3's test input | the version gate |
| `emoji/emoji-data.txt` | `Emoji`, `Emoji_Presentation`, `Emoji_Modifier`, `Emoji_Modifier_Base`, `Emoji_Component` (`Extended_Pictographic` is not read) | five predicates |
| `confusables.txt` | source, target, type of every line | `CONFUSABLE`, `CONFUSABLE_REVERSE`, `NAME` coverage |

A §3.2 says `build.rs` does not read `NormalizationTest.txt`; it reads
its first line and nothing more, so that a stale test file fails the
same version gate as a stale table file. Say so in the report.

### 4.2 `crates/wipemark-core/ucd/README.md`

Hand-written, short, and kept true by
`the_ucd_readme_names_the_version_the_tables_were_built_from` (§5). It
must contain, in this order:

1. One paragraph: these are Unicode Character Database files (and UTS #39
   `confusables.txt`), version **18.0.0**, copied verbatim by
   `scripts/fetch-ucd.sh`; `build.rs` turns them into the tables in
   `src/tables.rs`; they are never edited by hand — an edit is invisible
   in review (`-diff`) and is caught by the checksum test instead.
2. The table of §4.1 with the unicode.org path and the size in bytes of
   each file (from §3.2).
3. How the version is established: the headers of the eight files that
   have one, and the cross-check that binds `UnicodeData.txt` (§4.8.7) —
   with its one blind spot, a release that assigns no new code point.
4. Verify: `cd crates/wipemark-core/ucd && shasum -a 256 -c SHA256SUMS`
   (or `sha256sum -c SHA256SUMS`).
5. Bump: `scripts/fetch-ucd.sh <new version>`, update the version in
   this README, `cargo test -p wipemark-core`. A red suite after a bump
   means Unicode reassigned or reshaped something the code relied on —
   read the failure; do not edit the test to match.
6. Licence: the Unicode License v3, reproduced in the repository's
   `NOTICE`.

### 4.3 `crates/wipemark-core/ucd/SHA256SUMS`

Written by the fetch script, never by hand. Exactly nine lines, one per
data file, in `LC_ALL=C` sort order of the path, each
`<64 lowercase hex digits><two spaces><path relative to ucd/>` — the
format both `shasum -a 256 -c` and `sha256sum -c` read:

```
09c9…68c9  DerivedCoreProperties.txt
98ac…1ccd  DerivedNormalizationProps.txt
25a5…dde3  NormalizationTest.txt
f438…d5d6  PropList.txt
0071…6a6a  Scripts.txt
c7ae…81ac  StandardizedVariants.txt
0736…0950  UnicodeData.txt
6ed3…5b92  confusables.txt
80d0…bf1  emoji/emoji-data.txt
```

(Full hashes in §3.2.) `README.md` and `SHA256SUMS` itself are not
listed.

### 4.4 `scripts/fetch-ucd.sh`

Bash, `set -euo pipefail`, executable (`chmod +x`), in the style of
`scripts/promote-icon.sh`: the header comment explains the script and is
what `--help` prints; `ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." &&
pwd)"`; it writes only under `$ROOT/crates/wipemark-core/ucd/`.

**Interface.**

```
scripts/fetch-ucd.sh <version>       e.g. scripts/fetch-ucd.sh 18.0.0
scripts/fetch-ucd.sh -h | --help
```

**Behaviour, in order.**

1. Exactly one argument. `-h`/`--help` prints the header and exits 0.
   No argument, more than one, an option it does not know, or a version
   not matching `^[0-9]+\.[0-9]+\.[0-9]+$` → usage on stderr, **exit 2**.
2. A major version below 17 → **exit 2** with "the security files moved
   to Public/<version>/security/ in 17.0.0; this script knows only that
   layout". The script is for bumping forward; refusing an older layout
   makes its one URL assumption explicit instead of producing a 404 for a
   reason the user cannot see.
3. `curl` and one of `sha256sum` / `shasum` must be on `PATH`; otherwise
   **exit 1** naming the missing tool.
4. Create a temporary directory (`mktemp -d`, removed by an `EXIT`
   trap). Download into it, preserving the relative paths:
   - `https://www.unicode.org/Public/<version>/ucd/` + `UnicodeData.txt`,
     `DerivedCoreProperties.txt`, `PropList.txt`, `Scripts.txt`,
     `StandardizedVariants.txt`, `DerivedNormalizationProps.txt`,
     `NormalizationTest.txt`, `emoji/emoji-data.txt`;
   - `https://www.unicode.org/Public/<version>/security/confusables.txt`.

   Each with `curl --fail --silent --show-error --proto '=https'
   --location --proto-redir '=https' -o <tmp>/<path> <url>`. Any failure
   → **exit 1**, naming the URL; `ucd/` has not been touched.
5. Check every downloaded file before anything is moved, with the same
   rules `build.rs` applies (§4.8.7): line 1 of each of the six `ucd/`
   files with a header must be `# <stem>-<version>.txt`;
   `emoji/emoji-data.txt` and `confusables.txt` must carry `# Version:
   <version>` in their leading comment block; `UnicodeData.txt` must
   start with the bytes `0000;` (an HTML error page served with status 200
   would not). Any mismatch → **exit 1** naming the file and what it
   found; `ucd/` untouched.
6. In the temporary directory, hash the nine files (`sha256sum`, else
   `shasum -a 256`) with relative paths and write `SHA256SUMS`, lines
   sorted with `LC_ALL=C sort -k2`.
7. Only now: `mkdir -p "$UCD/emoji"`, then move the nine files and
   `SHA256SUMS` into `ucd/`, overwriting. Never touch `ucd/README.md`,
   never delete anything else that is there.
8. Print the version, each path with its size, and two reminders: update
   the version line of `ucd/README.md` if it changed, and run `cargo test
   -p wipemark-core`. Exit 0.

**Idempotent.** A second run with the same version produces
byte-identical files and `SHA256SUMS`; `git status` shows nothing new.
The script never runs during a build or a test (§0.4: no test touches the
network) and nothing in CI calls it.

### 4.5 `.gitattributes` (repository root, new)

```gitattributes
# Unicode data, crates/wipemark-core/ucd/: byte-exact, because
# SHA256SUMS hashes these bytes and build.rs refuses a carriage return;
# and never diffed, because a version bump is megabytes nobody reads —
# the checksum file (diffable) and the suite are what gets reviewed.
crates/wipemark-core/ucd/**/*.txt   -text -diff
crates/wipemark-core/ucd/SHA256SUMS -text

# Fixtures are byte-exact: a normalised line ending is a changed fixture
# (fixtures/README.md, rule 1). E1-3 adds the files.
fixtures/text/**                    -text
```

These three lines are **D25**. They differ from the earlier one-line
summary `ucd/*.txt -diff, fixtures/text/* -text` on purpose: a pattern
containing a slash is anchored at the directory of the `.gitattributes`
file, so `ucd/*.txt` in the root file matches nothing under
`crates/wipemark-core/` (checked with `git check-attr` on a scratch
repository: `diff: unspecified`), and `*` does not reach `ucd/emoji/`.
`-text` is added because the checksum is over the bytes;
`fixtures/text/**` also covers subdirectories (D17's byte-exact rule).

Verify: `git check-attr diff text -- crates/wipemark-core/ucd/UnicodeData.txt
crates/wipemark-core/ucd/emoji/emoji-data.txt
crates/wipemark-core/ucd/README.md fixtures/text/x` prints `unset`/`unset`
for the two data files, `unspecified` for `README.md`, and `text: unset`
for the fixture path (the path need not exist).

### 4.6 `NOTICE`

Append one section after "UI framework", in the file's existing format:

```
------------------------------------------------------------------------
Unicode Character Database
------------------------------------------------------------------------

The files under `crates/wipemark-core/ucd/` are data files of the
Unicode Character Database and of Unicode Technical Standard #39
(`confusables.txt`), version 18.0.0, copied verbatim from
https://www.unicode.org/Public/18.0.0/ by `scripts/fetch-ucd.sh`; their
checksums are in `crates/wipemark-core/ucd/SHA256SUMS`. Tables generated
from them by `crates/wipemark-core/build.rs` are compiled into the
Wipemark binaries. They are distributed under the Unicode License v3,
reproduced here as published at https://www.unicode.org/license.txt:

UNICODE LICENSE V3

COPYRIGHT AND PERMISSION NOTICE

Copyright © 1991-2026 Unicode, Inc.

NOTICE TO USER: Carefully read the following legal agreement. BY
DOWNLOADING, INSTALLING, COPYING OR OTHERWISE USING DATA FILES, AND/OR
SOFTWARE, YOU UNEQUIVOCALLY ACCEPT, AND AGREE TO BE BOUND BY, ALL OF THE
TERMS AND CONDITIONS OF THIS AGREEMENT. IF YOU DO NOT AGREE, DO NOT
DOWNLOAD, INSTALL, COPY, DISTRIBUTE OR USE THE DATA FILES OR SOFTWARE.

Permission is hereby granted, free of charge, to any person obtaining a
copy of data files and any associated documentation (the "Data Files") or
software and any associated documentation (the "Software") to deal in the
Data Files or Software without restriction, including without limitation
the rights to use, copy, modify, merge, publish, distribute, and/or sell
copies of the Data Files or Software, and to permit persons to whom the
Data Files or Software are furnished to do so, provided that either (a)
this copyright and permission notice appear with all copies of the Data
Files or Software, or (b) this copyright and permission notice appear in
associated Documentation.

THE DATA FILES AND SOFTWARE ARE PROVIDED "AS IS", WITHOUT WARRANTY OF ANY
KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT OF
THIRD PARTY RIGHTS.

IN NO EVENT SHALL THE COPYRIGHT HOLDER OR HOLDERS INCLUDED IN THIS NOTICE
BE LIABLE FOR ANY CLAIM, OR ANY SPECIAL INDIRECT OR CONSEQUENTIAL DAMAGES,
OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS,
WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THE DATA
FILES OR SOFTWARE.

Except as contained in this notice, the name of a copyright holder shall
not be used in advertising or otherwise to promote the sale, use or other
dealings in these Data Files or Software without prior written
authorization of the copyright holder.
```

Before committing, `curl -s https://www.unicode.org/license.txt` and
check the licence text is still exactly this (the year range is the
likeliest thing to have moved); if it moved, use the published text.

### 4.7 `crates/wipemark-core/Cargo.toml`

Add `build = "build.rs"` to `[package]` (Cargo would find the file
anyway; the key makes the generator visible to a reader of the
manifest). Replace the comment at `:11-13` with the present tense:

```toml
# NO [dependencies], [dev-dependencies] OR [build-dependencies] SECTION.
# This is a rule, not an accident (scripts/check-dep-direction.sh fails
# on any of the three). The UCD tables are generated by build.rs, which
# is written against std alone, from the files committed under ucd/.
```

Nothing else changes; `Cargo.lock` does not move (`--locked` stays green).

### 4.8 `crates/wipemark-core/build.rs`

#### 4.8.1 Shape and inputs

A module doc comment stating the two jobs, as `wipemark-i18n/build.rs`
does: (1) every committed UCD file is parsed and checked, and the build
fails rather than guess; (2) the tables become Rust. Then `fn main`:

```rust
let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
let ucd = root.join("ucd");
println!("cargo:rerun-if-changed=build.rs");
println!("cargo:rerun-if-changed={}", ucd.display());
for file in FILES { println!("cargo:rerun-if-changed={}", ucd.join(file).display()); }
```

`FILES` is the nine relative paths of §4.1 (`emoji/emoji-data.txt` with
its subdirectory). The order of work is fixed, because the first failure
is the one reported: **read** all nine → **version** (§4.8.7) → **parse**
(§4.8.3–4.8.6) → **cross-check** (§4.8.7) → **derive** (§4.8.8) →
**check invariants** (§4.8.10) → **emit** (§4.8.9) to
`$OUT_DIR/tables.rs`.

Code rules: `std` only — `std::fs`, `std::env`, `std::path`,
`std::collections::{BTreeMap, BTreeSet}`, `std::fmt::Write`. **No
`HashMap`/`HashSet` iteration in anything that reaches the output**:
the generated file must be byte-identical across builds and machines.
Workspace lints apply (§2). A panic message always names the file and,
for a line, its 1-based line number and the line itself, and says what
to do (usually: "re-run scripts/fetch-ucd.sh <version>").

#### 4.8.2 Reading a file

`fs::read_to_string` (a read error or invalid UTF-8 panics with the
path). Strip one leading U+FEFF from the content if present (older
`confusables.txt` releases began with one; 18.0.0 files do not). Split on
`'\n'`; a final empty piece after the last newline is not a line. A line
containing `'\r'` is an **unparsed line** (§4.8.10 G3): the files are
LF-only and `.gitattributes` keeps them so, and a CR means the bytes are
not the ones `SHA256SUMS` hashed.

A code point in any file is `[0-9A-F]{4,6}` parsed with
`u32::from_str_radix(_, 16)`, at most `0x10FFFF`; a range is
`<cp>..<cp>` with first ≤ last. Anything else where a code point is
expected is an unparsed line.

#### 4.8.3 `UnicodeData.txt`

No comments, no blank lines (a blank line or a `#` is an unparsed line).
Every line has **exactly 15 fields** split on `;`, fields not trimmed:

| index | field | how it is read |
|---|---|---|
| 0 | code point | 4–6 hex digits; strictly greater than the previous line's (else G3 with "not ascending") |
| 1 | Name | `<…, First>` / `<…, Last>`: a range (below). Any other value starting with `<` (only `<control>` in 18.0.0): the code point has **no** name. Otherwise the name, which must match `[A-Z0-9 -]+` (else G3). |
| 2 | General_Category | one of the 29 values of §3.3 item 4, else G3 |
| 3 | Canonical_Combining_Class | decimal `u8`, else G3 |
| 4 | Bidi_Class | one of the 23 values of §3.3 item 4, else G3 |
| 5 | decomposition | empty; or `<tag> X Y …` (compatibility; tag one of the 16 of §3.3 item 4, else G3); or `X Y …` (canonical). Code points separated by single spaces, at least one. A canonical mapping with more than two code points is G3. |
| 6–14 | numeric values, mirrored, Unicode 1 name, ISO comment, case mappings | ignored, but the field count must still be 15 |

**`First`/`Last` pairs.** A line whose name ends `, First>` must be
followed *immediately* by a line whose name ends `, Last>`, whose label
(the text between `<` and the comma) is identical, and whose fields 2, 3
and 4 are identical; field 5 must be empty on both. The pair assigns
that `gc`, `ccc` and bidi class to every code point of `first..=last`
and gives none of them a name. A `First` not followed by its `Last`, a
`Last` without a `First`, or any difference → G3. Recognise the pattern,
never a list of labels: 18.0.0 added the Jurchen and Seal ranges and
moved the end of two others (§3.3 item 2).

**Surrogates** (`gc=Cs`, D800..DFFF) are parsed and then dropped: no
`char` is a surrogate, so no table ever contains one.

What this file yields, per code point (store `gc` in a `Vec<u8>` of
length 0x110000 with a sentinel for unassigned; that is 1.1 MB inside the
build script, which is fine): `gc`; `ccc`; whether `Bidi_Class` is `R`
or `AL`; the raw decomposition `(is_compat, Vec<u32>)`; the name (only
for explicit lines).

#### 4.8.4 The property files

`DerivedCoreProperties.txt`, `PropList.txt`, `Scripts.txt`,
`DerivedNormalizationProps.txt`, `emoji/emoji-data.txt` share one
grammar (UAX #44 §4.2):

1. If the line starts with `# @missing:`, handle it as below. Otherwise,
   cut the line at its first `#`; trim spaces and tabs; an empty result
   is skipped.
2. Split on `;` and trim each field of spaces and tabs. Field 0 is a code
   point or a range (§4.8.2). Field 1 is non-empty. The allowed field
   counts are: `PropList.txt`, `Scripts.txt`, `emoji-data.txt` — exactly
   2; `DerivedCoreProperties.txt`, `DerivedNormalizationProps.txt` — 2 or
   3, and the third may be empty. Anything else → G3.
3. A **binary** property is read from 2-field lines whose field 1 is its
   name; the code points of field 0 have it. A wanted property name on a
   3-field line → G3 (a binary property suddenly carrying a value is a
   format change to look at, not to skip).
4. In `Scripts.txt` field 1 is the script's long name. A code point
   listed twice → G3 (the property is a function).

What is read: `Default_Ignorable_Code_Point` (DerivedCoreProperties);
`Bidi_Control`, `Variation_Selector`, `Join_Control`,
`Noncharacter_Code_Point` (PropList); every line of `Scripts.txt`;
`Full_Composition_Exclusion` (DerivedNormalizationProps); `Emoji`,
`Emoji_Presentation`, `Emoji_Modifier`, `Emoji_Modifier_Base`,
`Emoji_Component` (emoji-data). Every other property is parsed for
structure and ignored.

**`@missing` lines.** `# @missing: <range>; <field>[; <field>]`. Parse
them with the same field rules after the prefix. `build.rs` assumes that
a code point a file does not list lacks every binary property it reads,
and that a code point `Scripts.txt` does not list is `Unknown` (folded
into `Other`). So: in `Scripts.txt`, an `@missing` whose value is not
`Unknown` → G5; in any other file, an `@missing` whose second field names
one of the properties above → G5. All other `@missing` lines (in 18.0.0:
the one in `DerivedCoreProperties.txt` and the six in
`DerivedNormalizationProps.txt`, §3.3 item 6) are fine.

#### 4.8.5 `StandardizedVariants.txt`

Comments and blank lines as in §4.8.4. Every data line has exactly 3
fields: field 0 is **exactly two** code points separated by one space
(base, selector); field 1 is a description; field 2 the shaping
environments, possibly empty. The selector must have
`Variation_Selector` (else G3: a "variation sequence" whose second code
point is not a selector is not one). Collect the distinct
`(base, selector)` pairs.

#### 4.8.6 `confusables.txt`

Comments and blank lines as in §4.8.4 (the comment part of a data line
holds raw characters, including right-to-left and combining ones; it is
cut at the first `#` and never looked at — the data fields are hex and
cannot contain `#`). Every data line has exactly 3 fields: field 0 is
**one** code point (the source); field 1 is one or more code points
separated by single spaces (the target, i.e. the source's skeleton);
field 2 is `MA` (UTS #39 has written nothing else since the type field
became obsolete; anything else → G3). A source listed twice → G3.

#### 4.8.7 The version

**Which line names it.** For `DerivedCoreProperties.txt`,
`PropList.txt`, `Scripts.txt`, `StandardizedVariants.txt`,
`DerivedNormalizationProps.txt`, `NormalizationTest.txt`: line 1 must be
exactly `# <stem>-<V>.txt`, where `<stem>` is the file name without
`.txt`. For `emoji/emoji-data.txt` and `confusables.txt`: the leading
comment block (the lines before the first line that does not start with
`#`) must contain exactly one line `# Version: <V>`. `<V>` is three
dot-separated decimal numbers. A file whose version cannot be read this
way → **G1** ("cannot tell which Unicode version <path> is: expected …").
A §3.2's fallbacks (an emoji version derived from neighbours' `# Date`
lines, or the version named in `README.md`) are not used: 18.0.0's
`emoji-data.txt` names its version, and a README is not data.

**One version.** If the eight versions are not all equal → **G1**,
listing every file with the version it names (e.g.
`PropList.txt 17.0.0; DerivedCoreProperties.txt 18.0.0; …`) and "re-run
scripts/fetch-ucd.sh <version>". The common value becomes
`UNICODE_VERSION`. This is
`every_table_comes_from_one_unicode_version`, build half (D18, first
sentence).

**`UnicodeData.txt` is bound by what it assigns** (D18, as revised). It
names no version (§3.3 item 1). By UAX #24, `Scripts.txt` gives a script other than
`Unknown` to exactly the assigned code points other than private-use and
surrogates. So after parsing: let *A* be every code point
`UnicodeData.txt` gives a `gc` other than `Co` and `Cs` (ranges
expanded), and *S* every code point a `Scripts.txt` data line lists. *A*
≠ *S* → **G2**, with both difference counts, the first five code points
of each difference as `U+XXXX`, and the sentence "UnicodeData.txt carries
no version header; it is bound to Scripts.txt's version by assigning
exactly the code points Scripts.txt lists. These two files come from
different Unicode versions." A release that assigns new code points —
17.0.0 → 18.0.0 assigned 13,007 — cannot pass this with a mismatched
`UnicodeData.txt`; a release that assigned nothing new would, and the
README of `ucd/` says so.

#### 4.8.8 Derived sets

All sets are `BTreeSet<u32>` or sorted `Vec`s; all ranges are maximal
(adjacent and overlapping ranges merged).

- **The twenty predicate sets** come straight from §4.8.3–4.8.4:
  `gc` = `Co`, `Zs`, `Cf`, `L*` (Lu Ll Lt Lm Lo), `M*` (Mn Mc Me), `Nd`,
  `Lu`, `Ll`; Bidi R ∪ AL; the four PropList properties; DICP; the five
  emoji properties; `Script=Han`.
- **Script folding.** `build.rs` holds the 55 long names the `Script`
  enum keeps (§4.10, in the enum's order): `Latin`, `Cyrillic`, `Greek`,
  `Arabic`, `Hebrew`, `Han`, `Hiragana`, `Katakana`, `Hangul`, `Bopomofo`,
  `Syriac`, `Nko`, `Mandaic`, `Adlam`, `Hanifi_Rohingya`, `Devanagari`,
  `Bengali`, `Gurmukhi`, `Gujarati`, `Oriya`, `Tamil`, `Telugu`,
  `Kannada`, `Malayalam`, `Sinhala`, `Tibetan`, `Myanmar`, `Khmer`,
  `Mongolian`, `Tai_Tham`, `Tai_Viet`, `New_Tai_Lue`, `Balinese`,
  `Javanese`, `Sundanese`, `Batak`, `Lepcha`, `Limbu`, `Meetei_Mayek`,
  `Kayah_Li`, `Cham`, `Chakma`, `Sharada`, `Grantha`, `Kaithi`, `Modi`,
  `Takri`, `Tirhuta`, `Siddham`, `Newa`, `Sogdian`, `Manichaean`,
  `Old_Uyghur`, `Common`, `Inherited`. The variant identifier is the long
  name with `_` removed (`Hanifi_Rohingya` → `Script::HanifiRohingya`).
  Every other value, including `Unknown`, folds to `Script::Other` and is
  not emitted. A name in this list that occurs nowhere in `Scripts.txt` →
  **G4** (a typo would otherwise fold a whole script into `Other`
  silently).
- **Decomposition, fully expanded.** For each code point with a raw
  mapping, replace every code point of the mapping by its own full
  expansion, recursively, canonical and compatibility mappings alike —
  the full NFKD mapping of that one code point (UAX #15). Do **not**
  reorder by `ccc` (canonical ordering is a runtime step over the whole
  string, E1-3's). Hangul syllables have no mapping in the file and get
  none (their decomposition is arithmetic, E1-3's).
- **Composition.** A primary composite is a code point whose raw mapping
  is canonical, has exactly two code points, and is not in
  `Full_Composition_Exclusion`; its pair is that raw mapping (not the
  expanded one). Two composites for one pair → G3.
- **Confusables filter (A §3.2 as amended by D22).** A source *s* is
  kept when its target is **exactly one code point** *and* `gc(s)` is
  `L*` *and* `Script(s)` folds to `Latin`, `Cyrillic` or `Greek` —
  nothing else. A §3.2/§5.4's "or from the Halfwidth and Fullwidth Forms
  block" clause is **dropped** (D22): fullwidth Latin U+FF21..U+FF5A is
  `Script=Latin` and already kept, and the clause only added halfwidth
  Katakana, Hangul and punctuation that are not homoglyphs of a Latin,
  Cyrillic or Greek word. So `build.rs` has no block constant and needs
  no `Blocks.txt`. `skeleton(x)` is the kept target of *x* when *x* is a
  kept source, else *x*.
- **Homoglyph-capable set H** = kept sources ∪ kept targets that pass the
  same filter as a source. The second half is not optional: in a
  Cyrillic word, a Latin U+0061 is the finding, and U+0061 is a
  prototype, never a source (A §5.4's own `a_latin_letter_in_a_cyrillic_word_is`).
- **Reverse index.** Group H by `(skeleton(x), fold(Script(x)))`; each
  group's members ascending. Every group's script is `Latin`, `Cyrillic`
  or `Greek` (the filter admits nothing else), so one skeleton has at
  most three entries. The index is **complete**: members with a
  decomposition stay in it (see D42 in §4.9).
- **Name coverage set N** — every code point that can ever be a finding,
  by the class definitions of A §4.1, deliberately as a *superset* (E1-2
  may narrow a class with context or an exclusion; a name for something
  that turns out never to be a finding costs a few bytes, a missing name
  for a finding is a hole in a report): DICP ∪ `Bidi_Control` ∪
  `Variation_Selector` ∪ (`Zs` ∖ {U+0020}) ∪ `Noncharacter_Code_Point` ∪
  `Co` ∪ {U+FFF9, U+FFFA, U+FFFB} ∪ U+E0000..U+E007F ∪ {U+00AD, U+200B,
  U+200C, U+200D, U+2060, U+FEFF} ∪ H. (Several of these are already in
  DICP; list them anyway so the set does not depend on that.)
- **`NAME`** = every code point of N that has an explicit
  `UnicodeData.txt` line with a real name (not `<…>`), with that name.

#### 4.8.9 What is generated

`$OUT_DIR/tables.rs`, starting with a header comment:

```
// @generated by crates/wipemark-core/build.rs from crates/wipemark-core/ucd/
// (Unicode 18.0.0). Do not edit: change the committed files or build.rs.
//
// table                  entries   bytes (approx.)
// DEFAULT_IGNORABLE           17       136
// …                                          (one row per static below)
// total                                ~202 000
```

The byte column is entries × element size (+ pool and string bytes) —
the report copies this block. Then, in this order, with these names and
types, every table sorted ascending by its key:

| item | type | content | key order | 18.0.0 entries | ≈ bytes |
|---|---|---|---|---|---|
| `pub const UNICODE_VERSION` | `&str` | the version of §4.8.7, with a `///` doc saying where it came from | — | — | — |
| `static DEFAULT_IGNORABLE` | `&[(u32, u32)]` | DICP ranges (inclusive) | first | 17 | 136 |
| `static BIDI_CONTROL` | `&[(u32, u32)]` | `Bidi_Control` | first | 4 | 32 |
| `static VARIATION_SELECTOR` | `&[(u32, u32)]` | `Variation_Selector` | first | 4 | 32 |
| `static JOIN_CONTROL` | `&[(u32, u32)]` | `Join_Control` | first | 1 | 8 |
| `static NONCHARACTER` | `&[(u32, u32)]` | `Noncharacter_Code_Point` | first | 18 | 144 |
| `static PRIVATE_USE` | `&[(u32, u32)]` | `gc=Co` | first | 3 | 24 |
| `static SPACE_SEPARATOR` | `&[(u32, u32)]` | `gc=Zs` | first | 7 | 56 |
| `static FORMAT` | `&[(u32, u32)]` | `gc=Cf` | first | 21 | 168 |
| `static LETTER` | `&[(u32, u32)]` | `gc=L*` | first | 694 | 5,552 |
| `static MARK` | `&[(u32, u32)]` | `gc=M*` | first | 333 | 2,664 |
| `static DECIMAL_DIGIT` | `&[(u32, u32)]` | `gc=Nd` | first | 72 | 576 |
| `static UPPERCASE_LETTER` | `&[(u32, u32)]` | `gc=Lu` | first | 673 | 5,384 |
| `static LOWERCASE_LETTER` | `&[(u32, u32)]` | `gc=Ll` | first | 680 | 5,440 |
| `static RTL` | `&[(u32, u32)]` | `Bidi_Class` ∈ {R, AL} | first | 138 | 1,104 |
| `static EMOJI` | `&[(u32, u32)]` | `Emoji` | first | 150 | 1,200 |
| `static EMOJI_PRESENTATION` | `&[(u32, u32)]` | `Emoji_Presentation` | first | 80 | 640 |
| `static EMOJI_MODIFIER` | `&[(u32, u32)]` | `Emoji_Modifier` | first | 1 | 8 |
| `static EMOJI_MODIFIER_BASE` | `&[(u32, u32)]` | `Emoji_Modifier_Base` | first | 40 | 320 |
| `static EMOJI_COMPONENT` | `&[(u32, u32)]` | `Emoji_Component` | first | 10 | 80 |
| `static HAN` | `&[(u32, u32)]` | `Script=Han` | first | 21 | 168 |
| `static SCRIPT` | `&[(u32, u32, Script)]` | ranges of the 55 kept scripts, adjacent same-script ranges merged; a gap is `Other` | first | 656 | 7,872 |
| `static STANDARDIZED_VARIANTS` | `&[(u32, u32)]` | `(base, selector)` | `(base, selector)` | 1,437 | 11,496 |
| `static DECOMPOSITION` | `&[(u32, u16, u8)]` | `(code point, offset into the pool, length)` | code point | 5,982 | 47,856 |
| `static DECOMPOSITION_POOL` | `&[char]` | the full expansions, concatenated in key order | — | 9,265 | 37,060 |
| `static CCC` | `&[(u32, u32, u8)]` | ranges of equal non-zero `ccc`, merged | first | 422 | 5,064 |
| `static COMPOSITION` | `&[((u32, u32), char)]` | `((first, second), composite)` | `(first, second)` | 961 | 11,532 |
| `static CONFUSABLE` | `&[(u32, char)]` | `(kept source, its one-code-point skeleton)` | source | 358 | 2,864 |
| `static CONFUSABLE_REVERSE` | `&[(u32, Script, &[char])]` | `(skeleton, script, members ascending)` | skeleton, then script **long name** alphabetically | 306 (493 chars) | 9,316 |
| `pub(crate) static NAME` | `&[(u32, &str)]` | `(code point, UCD name)` for N ∩ named | code point | 917 | 45,057 |

Why these shapes and not A §3.2's literally: a value returned as a `char`
is stored as a `char` (`COMPOSITION`, `CONFUSABLE`, the pools), so no
lookup needs `char::from_u32(..).unwrap()`; the decomposition is a pool
plus `(offset, length)` instead of 5,982 slice references (~85 KB instead
of ~180 KB); keys stay `u32` so a lookup is
`binary_search_by_key(&u32::from(c), …)`. The `Script` within one
skeleton is ordered by long name so the order does not depend on the
enum's declaration; `confusables_with` scans the run (at most 3 entries,
one per script, D22) rather than relying on it.

Emission rules: hex literals as `0x00AD` (uppercase digits, at least
four); every `char` as `'\u{XXXX}'` — **never** a raw character in the
generated source (a raw U+200B in a Rust file is the bug this product
exists to find); names with `{:?}` (they are ASCII, §4.8.3); statics as
`static TABLE: &[T] = &[…];` (a slice, not an array type: clippy's
`large_const_arrays` and friends never see an array). Every generated
static except `UNICODE_VERSION` and `NAME` is private to the `tables`
module — later documents use the functions of §4.9, never a layout.
`NAME` is `pub(crate)` because `name.rs` reads it; nothing else may.

#### 4.8.10 The build-time gates

Each is a `panic!` in `build.rs`, so it fails `cargo build`, `cargo
test` and `cargo clippy` alike (A §3.2: "better to fail than silently
lose a property").

| gate | fires when | message must name |
|---|---|---|
| **G1** version | a file's version cannot be read, or two files name different versions | every file and its version; the fix |
| **G2** cross-check | `UnicodeData.txt`'s assigned set ≠ `Scripts.txt`'s listed set | both counts, five examples each, the sentence of §4.8.7 |
| **G3** unparsed line | any structural rule of §4.8.2–4.8.6 or §4.8.8 is broken | path, line number, the line, which rule |
| **G4** empty | any generated table of §4.8.9 is empty; a wanted property occurs on no line; a kept script name occurs nowhere | the table or property |
| **G5** `@missing` | an `@missing` contradicts an assumption of §4.8.4 | path, line, the assumption |
| **G6** normalization invariants | a key of `DECOMPOSITION` or a code point inside an expansion is a Hangul syllable (U+AC00..U+D7A3); a composite is one; the pool exceeds `u16::MAX` or an expansion `u8::MAX` | the code point |
| **G7** name invariants | a code point of N that `UnicodeData.txt` assigns and that is not `Co` has no real name; a member of N has no name and is not `Co`, not a noncharacter and not an unassigned DICP code point | the code point |

G6 and G7 hold on 18.0.0; they exist so that a future version cannot
quietly break an assumption E1-3 (Hangul is arithmetic) or `name_of`
(§4.11: no name means a label) is built on.

### 4.9 `crates/wipemark-core/src/tables.rs`

```rust
//! The Unicode Character Database, as the lookups Layer A needs.
//! (Explain: generated by build.rs from ucd/; which version; that every
//! lookup is a binary search; that nothing outside this module may
//! depend on a table's layout — only on the functions below.)

// Until E1-2…E1-4 call these, only the tests do. The allow is scoped to
// non-test builds so that a lookup with no unit test is still a
// dead-code warning under `cargo clippy --all-targets`. E1-7 removes it.
#![cfg_attr(not(test), allow(dead_code))]

use std::cmp::Ordering;

use crate::script::Script;

include!(concat!(env!("OUT_DIR"), "/tables.rs"));
```

One private helper, `fn in_ranges(table: &[(u32, u32)], c: char) ->
bool`: `binary_search_by` with `last < cp → Less`, `first > cp →
Greater`, else `Equal`; `cp = u32::from(c)`. Then exactly the functions
of `docs/plan/README.md` §3.3, all `pub(crate)`, each with a `///` that
names its source property and states the edge cases below:

| function | returns true / the value when | notes the doc comment must carry |
|---|---|---|
| `is_default_ignorable(c)` | `Default_Ignorable_Code_Point` | includes reserved code points (U+2065, U+FFF0..U+FFF8, U+E0000 …) and U+1BCA0..U+1BCA3, U+1D173..U+1D17A — the property as published; D19 makes those two ranges never-findings by name, which is E1-2's subtraction in `class_of`, not this table's |
| `is_bidi_control(c)` | `Bidi_Control` (12 code points) | |
| `is_variation_selector(c)` | `Variation_Selector` (260) | includes the Mongolian FVS U+180B..U+180D, U+180F; not U+180E |
| `is_join_control(c)` | `Join_Control` (U+200C, U+200D) | |
| `is_noncharacter(c)` | `Noncharacter_Code_Point` (66) | |
| `is_private_use(c)` | `gc=Co` | the three `First`/`Last` ranges, whole |
| `is_space_separator(c)` | `gc=Zs` | **includes U+0020**; the class `ExoticSpace` is `Zs` minus U+0020, which is E1-2's subtraction |
| `is_format(c)` | `gc=Cf` | |
| `is_letter(c)` | `gc` ∈ {Lu, Ll, Lt, Lm, Lo} | includes every `First`/`Last` letter range (CJK, Hangul syllables, Tangut, Jurchen, Seal) |
| `is_mark(c)` | `gc` ∈ {Mn, Mc, Me} | variation selectors are `Mn` and therefore marks |
| `is_decimal_digit(c)` | `gc=Nd` | |
| `is_uppercase_letter(c)` | `gc=Lu` | `Lt` is neither upper nor lower |
| `is_lowercase_letter(c)` | `gc=Ll` | |
| `is_rtl(c)` | `Bidi_Class` ∈ {R, AL} from `UnicodeData.txt` | **true for U+200F and U+061C** (RLM is `R`, ALM is `AL`) — the property as published; D20 (a paragraph is RTL only through an R/AL *letter or mark*) is E1-2's filter on top; false for every unassigned code point (the block defaults of `DerivedBidiClass.txt` are not committed) |
| `is_emoji(c)` | `Emoji` | true for `#`, `*`, `0`–`9`, U+00A9, U+00AE, U+2122, U+2139 (keycap and text-default bases); false for U+200D, U+FE0F, tags |
| `is_emoji_presentation(c)` | `Emoji_Presentation` | |
| `is_emoji_modifier(c)` | `Emoji_Modifier` (U+1F3FB..U+1F3FF) | |
| `is_emoji_modifier_base(c)` | `Emoji_Modifier_Base` | |
| `is_emoji_component(c)` | `Emoji_Component` | includes U+200D, U+20E3, U+FE0F, regional indicators, skin tones, hair components, U+E0020..U+E007F |
| `is_han(c)` | `Script=Han` | agrees with `script_of(c) == Script::Han` for every code point |
| `script_of(c) -> Script` | the folded `Script` of §4.8.8 | `Other` for every script the product does not name, for `Unknown`, for unassigned, private-use and noncharacter code points; combining marks of `Script=Inherited` (and U+200C/U+200D) are `Inherited` — resolving them to their base is the caller's |
| `is_standardized_variant(base, selector)` | `(base, selector)` is in `StandardizedVariants.txt` | emoji presentation sequences (`… U+FE0F`) and ideographic variation sequences (IVD) are **not** in it |
| `decomposition(c) -> Option<&'static [char]>` | `Some` full NFKD mapping of `c` (§4.8.8), `None` when `c` has none | not in canonical order; `None` for Hangul syllables (arithmetic, E1-3) |
| `ccc(c) -> u8` | `Canonical_Combining_Class` | 0 when not listed |
| `compose(first, second) -> Option<char>` | the primary composite of the pair | `None` for excluded composites, for non-starter decompositions, for Hangul (arithmetic, E1-3) |
| `confusable_target(c) -> Option<char>` | `Some(skeleton)` for a kept source (gc `L*`, script Latin/Cyrillic/Greek, one-code-point skeleton — D22) | `None` for everything else **including a prototype** and every halfwidth Katakana/Hangul letter; a caller's skeleton is `confusable_target(c).unwrap_or(c)` |
| `confusables_with(target, script) -> &'static [char]` | every member of H with `skeleton == target` and `script_of == script`, ascending | includes `target` itself when it is in H and of that script; may hold several letters, including letters **with a decomposition** (U+FF41 in the Latin list for `a`) — choosing among them (D21: the prototype, else the lowest code point without a decomposition, D42) is E1-4's; empty slice when none, and for any script other than Latin, Cyrillic or Greek |

`decomposition` reads `DECOMPOSITION` by binary search on the code
point and slices `DECOMPOSITION_POOL[offset..offset + len]` (use
`usize::from`). `ccc` searches `CCC` like a range table and returns its
value or 0. `compose` is `binary_search_by_key(&(u32::from(first),
u32::from(second)), |&(pair, _)| pair)`. `confusables_with` takes
`partition_point` to the first entry with the skeleton, scans while the
skeleton matches, and returns the slice of the entry whose `Script`
equals `script`, else `&[]`.

**D42 and the reverse index — an option, not a requirement.** D42 says
a twin with a decomposition is never a replacement (e.g. U+0063 → U+03F2
GREEK LUNATE SIGMA SYMBOL, whose compatibility decomposition is U+03C2,
is refused), and E1-4 enforces it before D21's lowest-code-point choice.
In 18.0.0, 129 of the 493 members of H have a decomposition (U+FF41,
U+FF2E, U+03F2, U+017F, U+00BA …). `build.rs` already knows every
decomposition, so the index *could* drop them at emission for the cost
of one condition — but that would change `confusables_with` from "every
twin" to "every admissible replacement", and the name-coverage test
(`a_name_exists_exactly_for_what_can_be_a_finding`, §5.4) uses the index
as its definition of H: a decomposable twin such as U+FF41 is still a
*finding* when it sits in a Cyrillic word, and must keep its name. So
this document keeps the index complete (`build.rs`'s H, `NAME` and the
index agree) and E1-4 filters with `decomposition(x).is_none()`, one
binary search per candidate. A filtered index would need that test to
spell H another way (from `confusable_target` plus a list of
prototypes); if you see a reason to prefer it, say so in the report
rather than switching.

No other `pub(crate)` item: no `is_assigned`, no `Script::ALL`, no raw
table re-export (`NAME` aside, §4.8.9). If you find one missing, add it
in the report as a proposal, not in the code (§0.2).

### 4.10 `crates/wipemark-core/src/script.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Script {
    Latin, Cyrillic, Greek, Arabic, Hebrew, Han, Hiragana, Katakana, Hangul, Bopomofo,
    Syriac, Nko, Mandaic, Adlam, HanifiRohingya, Devanagari, Bengali, Gurmukhi, Gujarati,
    Oriya, Tamil, Telugu, Kannada, Malayalam, Sinhala, Tibetan, Myanmar, Khmer, Mongolian,
    TaiTham, TaiViet, NewTaiLue, Balinese, Javanese, Sundanese, Batak, Lepcha, Limbu,
    MeeteiMayek, KayahLi, Cham, Chakma, Sharada, Grantha, Kaithi, Modi, Takri, Tirhuta,
    Siddham, Newa, Sogdian, Manichaean, OldUyghur,
    Common, Inherited, Other,
}
impl Script { pub fn as_str(self) -> &'static str { /* match */ } }
```

Exactly README §3.3's variants, order and derives (56 variants; D8's
`Bopomofo` included). The module doc says what the enum is for: the
scripts Layer A tells apart — the five alphabets and four CJK scripts
that letter shares and homoglyphs need (A §5.4, §5.5, D8), the joining
scripts of A §4.2 (E1-2's `JOINING_SCRIPTS` is a list of these), and
`Common`, `Inherited`, `Other` — and that every other UCD script folds
into `Other` on purpose: telling Thai from Armenian buys Layer A nothing.

`as_str` returns the UCD long name exactly as `Scripts.txt` spells it
(`"Latin"`, `"Hanifi_Rohingya"`, `"Nko"`, `"Old_Uyghur"`, …) for the 55
named variants, and **`"Other"`** for `Other`, which is not a UCD value
(it stands for the other 122 of the 177 values `Scripts.txt` uses, plus
`Unknown`); its doc comment says so.
The string is a format, never shown translated (it is an identifier, like
`UnicodeClass::as_str`). `Script` is not re-exported from `lib.rs`.

Carry the same `#![cfg_attr(not(test), allow(dead_code))]` as
`tables.rs`, with the same comment: in a non-test build nothing calls
`script_of` yet, so the variants are only constructed inside a dead
static.

### 4.11 `crates/wipemark-core/src/name.rs`

```rust
use std::borrow::Cow;

use crate::tables::{self, NAME};

pub fn name_of(c: char) -> Option<Cow<'static, str>> {
    let cp = u32::from(c);
    if let Ok(index) = NAME.binary_search_by_key(&cp, |&(code, _)| code) {
        return Some(Cow::Borrowed(NAME[index].1));
    }
    let prefix = if tables::is_private_use(c) {
        "private-use"
    } else if tables::is_noncharacter(c) {
        "noncharacter"
    } else if tables::is_default_ignorable(c) {
        "reserved"
    } else {
        return None;
    };
    Some(Cow::Owned(format!("<{prefix}-{cp:04X}>")))
}
```

Semantics (D7):

- `Some(Cow::Borrowed(name))` — the UCD name, for every member of N
  (§4.8.8) that has one: `name_of('\u{200B}')` is `"ZERO WIDTH SPACE"`,
  `name_of('\u{0061}')` is `"LATIN SMALL LETTER A"` (a homoglyph
  prototype).
- `Some(Cow::Owned(label))` — a code point label in the form of The
  Unicode Standard §4.8 and UAX #44 §4.2.5 (the same spelling the UCD
  files use in their comments, e.g. `2065 ; Default_Ignorable_Code_Point
  # Cn <reserved-2065>`; UAX #44: labels "are designed to be maintained
  as unique values within the namespace for Unicode character names", so
  a label can never be mistaken for a name): `<` + `private-use` /
  `noncharacter` / `reserved` + `-` +
  the code point in uppercase hex, at least four digits + `>`:
  `<private-use-E000>`, `<private-use-F0000>`, `<noncharacter-FDD0>`,
  `<noncharacter-10FFFF>`, `<reserved-E0080>`. The three-step fallback is
  exact because G7 guarantees that a member of N without a name is
  private-use, a noncharacter, or an unassigned default-ignorable code
  point — and those three sets are inside N, so for a code point outside
  N the fallback never fires.
- `None` — for every code point outside N: U+0020, U+000A, U+0436
  CYRILLIC SMALL LETTER ZHE, U+4E00, U+0600 ARABIC NUMBER SIGN, U+13430,
  an unassigned code point that is not default-ignorable (U+0378), the
  halfwidth Katakana and Hangul letters D22 removed from the confusables
  (U+FF89, U+FFA1).

**The D19 ranges keep their names, on purpose.** D19 makes U+1BCA0..U+1BCA3
and U+1D173..U+1D17A never findings, so README §3.3's comment ("None for
anything that can never be a finding") would ask for `None`. They stay in
N anyway: they are default-ignorable, so removing them from `NAME` alone
would make the fallback above call assigned characters
`<reserved-1BCA0>`, and removing them properly needs D19's list in
`build.rs` and `name.rs` as well as in E1-2's `class_of` — three copies of
one decision. Twelve names cost a few hundred bytes. Record this in the
report; the coordinator may move D19's list into `tables.rs` later if it
wants `name_of` exact.

The module doc says the rest of what a reader needs: why only N is named
(A §3.2 — every name in UnicodeData would be about 1.07 MB of strings in
the binary, N's are 23 KB); that N is a superset of what E1-2 will call a
finding, on purpose; that a name is an identifier of the Unicode
Standard, shown beside `U+XXXX`, never translated and never routed
through the catalogue (A §5.5, A §7.5, D16 — E1-6 writes the exception
into `docs/architecture/i18n.md` and `CLAUDE.md`).

### 4.12 `crates/wipemark-core/src/lib.rs`

Add three private modules and two re-exports; touch nothing else in the
public API:

```rust
pub mod class;
pub mod guard;
mod name;
pub mod report;
mod script;
mod tables;
pub mod vendor;

pub use class::{Action, Confidence, UnicodeClass, UnicodeFinding};
pub use guard::{Guard, GuardOutcome, RejectReason};
pub use name::name_of;
pub use report::{CleanReport, FinalReport, InspectReport, NormKind, RewriteSummary, RiskLabel, TextStats};
pub use tables::UNICODE_VERSION;
pub use vendor::Vendor;
```

(rustfmt decides the final layout.) `docs/plan/README.md` §3.3 lists
both re-exports again in E1-3's block; after this document they already
exist, and E1-3 leaves them. Update the crate doc: `:10-19` — the tables
are no longer four named properties but the set this document generates,
from `ucd/`, by `build.rs`, with the version read from the files; `:21-25`
— the tables exist (E1-1); the classifier, scrubber, NFKC, homoglyphs and
guards land in E1-2 … E1-5.

### 4.13 `docs/architecture/layer-a.md` and the docs index

Create `docs/architecture/layer-a.md`:

```
# Layer A — the deterministic scrubber

(One paragraph: what Layer A is, that it is the verifiable shelf, that
this document grows one section per E1 document — Tables (E1-1),
Classes and context (E1-2), Scrubber, report and NFKC (E1-3),
Homoglyphs (E1-4), Guards (E1-5), Surfaces (E1-6) — and that E1-7
completes it.)

## Tables
```

The "Tables" section, written for someone touching this code next year
(what, where, why — not a changelog): what is committed under `ucd/` and
why it is committed rather than a `unicode-*` crate (the report must
name the version; zero dependencies); the nine files and what each is
read for; how the version is established (headers; the cross-check
because `UnicodeData.txt` has no header) and that two versions fail the
build; the generated tables and the lookups, as one table (function ·
source property · edge case), including the traps of §4.9 (`is_rtl` on
RLM/ALM, `is_space_separator` includes U+0020, `confusable_target` is
`None` for a prototype, `is_standardized_variant` knows no emoji or IVD
sequence); what is deliberately not in the tables (Hangul arithmetic,
unassigned defaults, emoji sequences, the IVD, `Blocks.txt`); names —
only N, labels, why; the gates G1–G7 and the tests; how to bump the
version; the sizes from your report.

Add one row to `docs/README.md`, after `architecture/skeleton.md`:
`| [architecture/layer-a.md](architecture/layer-a.md) | Layer A, the
deterministic scrubber: the committed Unicode tables and how the build
reads them — one section per E1 document as they land |`.

Do **not** touch `CLAUDE.md` or `docs/architecture/skeleton.md`; E1-7
updates both once Layer A works.

### 4.14 The report

`docs/plan/reports/E1-1-<YYYY-MM-DD>.md`, per §0.7, and in addition:

- the header block of the generated `tables.rs` (entries and bytes per
  table — compare with §4.8.9 and explain any difference), and the size
  of the generated file (`find target -path '*wipemark-core-*/out/tables.rs'
  -exec wc -c {} +`);
- `ls -l target/release/wipemark-cli` from `cargo build --release -p
  wipemark-cli --locked` at `497eafa` and after your change, with the
  reason the delta is near zero (nothing in the CLI calls the tables
  before E1-6; the linker drops unreferenced statics) —
  `docs/plan/README.md` §3.1 asks for the delta at this checkpoint, and
  the honest number is "about nothing yet";
- the output of the fetch-script runs of §6;
- the departures from the spec A that this document takes, each with its
  basis: the `UnicodeData.txt` cross-check (D18), the `.gitattributes`
  patterns (D25), unit tests that read `ucd/` (D24), the confusables
  filter without the block clause (D22), the `NormalizationTest.txt`
  first line, the table shapes (§4.8.9), and `name_of` naming the D19
  ranges (§4.11) — plus any of your own.

## §5 Tests

Where they go (**D24**): an integration test cannot call a `pub(crate)`
function, so tests of the public API (`UNICODE_VERSION`, `name_of`) and
of the files themselves are in **`crates/wipemark-core/tests/ucd_files.rs`**;
tests of the `pub(crate)` lookups are **unit tests** in
`#[cfg(test)] mod tests` of the module that defines them, and those that
compare a lookup with a file read it with `include_str!("../ucd/…")`.

"Independent reading" below means a few lines of parsing written inside
the test — `split(';')`, `u32::from_str_radix`, `First`/`Last` handled
on its own — that share no code with `build.rs`, in the spirit of
`stripped_by_layer_a` (`crates/wipemark-i18n/src/tests.rs:316`). An
"exhaustive" test walks every `char` (`(0..=0x10FFFF).filter_map(char::from_u32)`).
Code points are written in tests as `'\u{XXXX}'`, never as raw
characters.

### 5.1 Build-time gates (no test function; painted by editing a file and running `cargo build -p wipemark-core`)

Restore after each probe with `scripts/fetch-ucd.sh 18.0.0` and confirm
with `cd crates/wipemark-core/ucd && shasum -a 256 -c SHA256SUMS` (the
files are new and uncommitted while you work — §0.1 forbids `git
checkout --` on a file with other uncommitted work).

| gate | how to paint it red | expected |
|---|---|---|
| G1 — `every_table_comes_from_one_unicode_version` (build half) | change line 1 of `ucd/PropList.txt` to `# PropList-17.0.0.txt` | build fails naming `PropList.txt 17.0.0` beside eight `18.0.0` |
| G1 — unreadable version | delete line 8 (`# Version: 18.0.0`) of `ucd/confusables.txt` | build fails: cannot tell which version `confusables.txt` is |
| G2 — `UnicodeData.txt` bound to `Scripts.txt` | `curl -s https://www.unicode.org/Public/17.0.0/ucd/UnicodeData.txt -o crates/wipemark-core/ucd/UnicodeData.txt` | build fails: 13,007 code points `Scripts.txt` lists and `UnicodeData.txt` does not |
| G3 — unparsed line | append `XYZ ; Bidi_Control` to `ucd/PropList.txt`; separately, delete the `4DBF;<CJK Ideograph Extension A, Last>…` line of `ucd/UnicodeData.txt` | build fails naming the file, line number and rule (bad code point; `First` without `Last`) |
| G4 — empty | in `build.rs`, misspell the wanted property `"Bidi_Control"` as `"Bidi_Controls"`; separately, misspell `"Bopomofo"` in the script list | build fails naming the property / the script |
| G5 — `@missing` | add `# @missing: 0000..10FFFF; Latin` to `ucd/Scripts.txt` | build fails naming the line |
| G6, G7 | not paintable without fabricating UCD data; they are invariant assertions that hold on 18.0.0 and guard the next version. The same facts are asserted at test time by `hangul_is_left_to_arithmetic` and `a_name_exists_exactly_for_what_can_be_a_finding`. Say so in the report. | — |

### 5.2 Unit tests — `src/tables.rs`

| test | assertion | paint it red |
|---|---|---|
| `a_range_holds_both_of_its_ends` | `is_bidi_control` true at U+202A and U+202E, false at U+2029 and U+202F; `is_variation_selector` true at U+E0100 and U+E01EF, false at U+E01F0 and U+E00FF; `is_private_use` true at U+E000, U+F8FF, U+F0000, U+FFFFD, U+100000, U+10FFFD, false at U+F900, U+FFFFE, U+10FFFE | in `in_ranges`, change `last < cp` to `last <= cp` |
| `every_binary_property_matches_its_file` | exhaustive: for `Bidi_Control`, `Variation_Selector`, `Join_Control`, `Noncharacter_Code_Point` (`PropList.txt`), `Default_Ignorable_Code_Point` (`DerivedCoreProperties.txt`), the five emoji properties (`emoji/emoji-data.txt`) and `Han` (`Scripts.txt`), an independent reading of the file gives the same answer as the lookup for every `char` | same mutation as above; or in `build.rs` merge ranges with `next.first <= last + 2` |
| `every_general_category_predicate_matches_unicode_data` | exhaustive, independent reading of `UnicodeData.txt` (with its own `First`/`Last` handling): `is_private_use` ⇔ `Co`, `is_space_separator` ⇔ `Zs`, `is_format` ⇔ `Cf`, `is_letter` ⇔ `L*`, `is_mark` ⇔ `M*`, `is_decimal_digit` ⇔ `Nd`, `is_uppercase_letter` ⇔ `Lu`, `is_lowercase_letter` ⇔ `Ll`, `is_rtl` ⇔ bidi `R`/`AL`, and `ccc(c)` equals field 3 (0 when unlisted) | in `build.rs`, treat a `First`/`Last` line as an ordinary single code point |
| `the_sets_layer_a_names_by_hand_are_the_sets_unicode_gives` | exhaustive equalities with A §4.1's own spellings: `is_bidi_control` ⇔ cp ∈ {0x061C, 0x200E, 0x200F, 0x202A..=0x202E, 0x2066..=0x2069}; `is_join_control` ⇔ {0x200C, 0x200D}; `is_variation_selector` ⇔ {0x180B..=0x180D, 0x180F, 0xFE00..=0xFE0F, 0xE0100..=0xE01EF}; `is_noncharacter` ⇔ `(0xFDD0..=0xFDEF).contains(&cp) \|\| cp & 0xFFFE == 0xFFFE`; `is_private_use` ⇔ the three Co ranges; `is_space_separator(c) && c != ' '` ⇔ {0x00A0, 0x1680, 0x2000..=0x200A, 0x202F, 0x205F, 0x3000}. Its doc comment: if a version bump turns this red, Unicode changed a set E1-2's table is written against — read A §4.1 before touching either | in `build.rs`, build `SPACE_SEPARATOR` from `Zs` ∪ `Zl` ∪ `Zp` (U+2028 becomes a space separator) |
| `letters_and_private_use_cover_the_ranges_unicode_data_lists_by_their_ends` | `is_letter` true at U+3400, U+4DBF, U+4E01, U+9FFF, U+AC01, U+D7A3, U+17001, U+18E00, U+323B0, U+3D001, U+3FC3F; `is_han` true at U+4E01, U+20001; `is_private_use` true at U+E001, U+F0001, U+10FFFD; `is_letter` false at U+E001, U+0378 | treat `First`/`Last` lines as single code points |
| `rtl_means_bidi_class_r_or_al` | true: U+05D0, U+0627, U+0640, U+07C0, U+FB1D, U+200F, U+061C; false: U+200E, U+0061, U+0661, U+202E, U+2067, U+0378 | read only `R`, not `AL` |
| `emoji_properties_are_read_separately` | `is_emoji`: true for U+0023, U+0030, U+00A9, U+2139, U+2764, U+1F600; false for U+0041, U+200D, U+FE0F, U+E0020. `is_emoji_presentation`: true U+1F600, U+1F525; false U+2139, U+2764, U+0023. `is_emoji_modifier`: true U+1F3FB, U+1F3FF; false U+1F3FA, U+1F44D. `is_emoji_modifier_base`: true U+1F44D, U+1F469; false U+1F600, U+2764. `is_emoji_component`: true U+0023, U+0030, U+200D, U+20E3, U+FE0F, U+1F1E6, U+1F3FB, U+E0020, U+E007F; false U+2764, U+FE0E, U+E0001 | swap two emoji property names in `build.rs` |
| `script_of_folds_scripts_txt_into_the_enum` | U+0041 Latin, U+0430 Cyrillic, U+03B1 Greek, U+0627 Arabic, U+05D0 Hebrew, U+4E00 Han, U+3005 Han, U+3042 Hiragana, U+30A2 Katakana, U+AC00 Hangul, U+3105 Bopomofo; joining scripts U+0710 Syriac, U+07CA Nko, U+0840 Mandaic, U+1E900 Adlam, U+10D00 HanifiRohingya, U+0915 Devanagari, U+0B15 Oriya, U+0F40 Tibetan, U+1780 Khmer, U+1820 Mongolian, U+1A20 TaiTham, U+ABC0 MeeteiMayek, U+11400 Newa, U+10F30 Sogdian, U+10AC0 Manichaean, U+10F70 OldUyghur; U+0020 Common, U+0640 Common, U+30FC Common, U+0301 Inherited, U+3099 Inherited, U+200D Inherited; Other for U+0E01 (Thai), U+0531 (Armenian), U+10FB0 (Chorasmian), U+0378 (unassigned), U+E000 (private use) | in `build.rs`, emit `Script::Han` for the `Bopomofo` ranges (U+3105 becomes Han) |
| `script_of_matches_scripts_txt_for_every_code_point` | exhaustive: an independent reading of `Scripts.txt`, folded by matching each value against `as_str()` of the 55 named variants (else `Other`), equals `script_of` | merge adjacent ranges of *different* scripts in `build.rs` |
| `is_han_agrees_with_script_of` | exhaustive: `is_han(c) == (script_of(c) == Script::Han)` | emit `HAN` from `Script=Hiragana` |
| `standardized_variants_are_pairs_of_base_then_selector` | true: (U+0030, U+FE00), (U+1820, U+180B), (U+349E, U+FE00); false: (U+FE00, U+0030), (U+0041, U+FE00), (U+2139, U+FE0F), (U+2764, U+FE0F), (U+4E00, U+E0100), (U+1820, U+180F) | store `(selector, base)` |
| `decompositions_are_expanded_all_the_way_down` | U+00C5 → [U+0041, U+030A]; U+212B → [U+0041, U+030A]; U+1E9B → [U+0073, U+0307]; U+1E69 → [U+0073, U+0323, U+0307]; U+01C4 → [U+0044, U+005A, U+030C]; U+FB01 → [U+0066, U+0069]; U+2139 → [U+0069]; U+00A0 → [U+0020]; U+FFA0 → [U+1160]; U+1F213 → [U+30C6, U+3099]; U+00BD → [U+0031, U+2044, U+0032]; U+FDFA has 18 code points starting U+0635 U+0644 U+0649 U+0020; `None` for U+0041, U+0020, U+4E00 | store the raw mapping instead of the expansion (U+212B then gives [U+00C5]) |
| `hangul_is_left_to_arithmetic` | `decomposition` is `None` for U+AC00 and U+D7A3; `compose(U+1100, U+1161)` and `compose(U+AC00, U+11A8)` are `None`; exhaustive: no `decomposition(c)` contains a code point in U+AC00..=U+D7A3 | none from data (G6); assert-only, say so |
| `combining_classes_are_read` | `ccc`: U+0301 230, U+0300 230, U+0323 220, U+05B0 10, U+093C 7, U+3099 8, U+0345 240, U+1D165 216, U+0F71 129, U+0041 0, U+10FFFF 0 | read field 4 instead of 3 |
| `primary_composites_compose_and_excluded_ones_do_not` | `compose`: (U+0041, U+030A) → U+00C5; (U+0065, U+0301) → U+00E9; (U+0049, U+0307) → U+0130; (U+03B9, U+0308) → U+03CA; (U+0B47, U+0B3E) → U+0B4B; (U+30C6, U+3099) → U+30C7; `None` for (U+0915, U+093C) (U+0958 is an exclusion), (U+2ADD, U+0338) (U+2ADC), (U+0F71, U+0F72) (U+0F73, a non-starter decomposition), (U+0041, U+0041) | do not subtract `Full_Composition_Exclusion` |
| `composition_is_every_canonical_pair_minus_the_exclusions` | exhaustive over an independent reading of `UnicodeData.txt` field 5 and the `Full_Composition_Exclusion` lines of `DerivedNormalizationProps.txt`: for every code point *c* with a two-code-point canonical mapping (a, b), `compose(a, b) == Some(c)` iff *c* is not excluded; and every entry of `COMPOSITION` is such a pair | same as above; or compose from the expanded mapping |
| `confusable_sources_map_to_their_skeleton` | `confusable_target`: U+0430 → U+0061, U+0410 → U+0041, U+0435 → U+0065, U+043E → U+006F, U+0440 → U+0070, U+0441 → U+0063, U+0445 → U+0078, U+0455 → U+0073, U+0456 → U+0069, U+0391 → U+0041, U+03B1 → U+0061, U+03BF → U+006F, U+FF41 → U+0061, U+0049 → U+006C (skeletons do not keep case), U+0417 → U+0033 (a digit is a valid skeleton) | invert source and target |
| `only_letters_of_latin_cyrillic_or_greek_are_sources` | `confusable_target` is `None` for U+0031 (a digit; the file maps it to U+006C), U+217C (`Nl`, Latin), U+4E00 (Han; the file maps it to U+30FC), U+0BE6 (Tamil digit), U+0181 (Latin letter with a two-code-point target), U+0061 and U+006C (prototypes), U+0436 (not in the file), and the halfwidth letters D22 drops: U+FF89 HALFWIDTH KATAKANA LETTER NO (the file maps it to U+002F), U+FFA1 HALFWIDTH HANGUL LETTER KIYEOK (→ U+1100), U+FFB7 HALFWIDTH HANGUL LETTER IEUNG (→ U+006F); while U+FF41 FULLWIDTH LATIN SMALL LETTER A is still `Some(U+0061)` (it is `Script=Latin`) | keep every one-code-point entry unfiltered; separately, restore the Halfwidth and Fullwidth Forms clause (U+FF89 becomes a source) |
| `the_reverse_index_holds_every_twin_and_the_prototype` | `confusables_with(U+0061, Cyrillic)` = [U+0430]; `(U+0061, Greek)` = [U+03B1]; `(U+0061, Latin)` = [U+0061, U+0251, U+AB64, U+FF41, U+1DF5A] (U+FF41 has a decomposition and stays — the index is complete, D42 is E1-4's); `(U+006F, Greek)` = [U+03BF, U+03C3]; `(U+006F, Cyrillic)` = [U+043E, U+1C82]; `(U+0063, Greek)` = [U+03F2]; `(U+0033, Cyrillic)` = [U+0417, U+04E0]; empty for `(U+006F, Hangul)` (U+FFB7 is gone, D22), `(U+0061, Han)` and `(U+0436, Cyrillic)` | build the index from sources only (U+0061 disappears from the Latin list); separately, restore the block clause (`(U+006F, Hangul)` becomes [U+FFB7]) |
| `every_reverse_entry_maps_back_to_its_target` | over `CONFUSABLE_REVERSE`: every member *x* of entry (t, s, xs) has `confusable_target(x).unwrap_or(x) == t`, `script_of(x) == s`, `is_letter(x)`, and *s* ∈ {Latin, Cyrillic, Greek}; members ascending; no (t, s) twice | group by the source's script instead of the member's; or restore the block clause (a Hangul entry appears) |

### 5.3 Unit tests — `src/script.rs`

| test | assertion | paint it red |
|---|---|---|
| `as_str_is_the_ucd_long_name` | for each of the 55 named variants, `as_str()` occurs as a value in `Scripts.txt` (read with `include_str!("../ucd/Scripts.txt")`); spot: `HanifiRohingya` → `"Hanifi_Rohingya"`, `Nko` → `"Nko"`, `OldUyghur` → `"Old_Uyghur"`, `MeeteiMayek` → `"Meetei_Mayek"`, `NewTaiLue` → `"New_Tai_Lue"`; `Other.as_str()` is `"Other"` and does **not** occur in `Scripts.txt` | change one arm to `"Hanifi Rohingya"` |
| `every_script_but_other_has_code_points` | exhaustive: the set of values `script_of` returns over all `char`s is all 56 variants. The variant list is a test-local array made exhaustive with the `ordinal` match pattern of `class.rs:173-193`, so a variant added to the enum and forgotten in the test fails to compile | the same mutation as `script_of_folds_scripts_txt_into_the_enum` (`Bopomofo` is then never returned) |

### 5.4 Unit tests — `src/name.rs`

| test | assertion | paint it red |
|---|---|---|
| `a_finding_capable_character_has_its_ucd_name` | `Cow::Borrowed` and equal to: U+200B "ZERO WIDTH SPACE", U+200C "ZERO WIDTH NON-JOINER", U+200D "ZERO WIDTH JOINER", U+2060 "WORD JOINER", U+FEFF "ZERO WIDTH NO-BREAK SPACE", U+00AD "SOFT HYPHEN", U+061C "ARABIC LETTER MARK", U+202E "RIGHT-TO-LEFT OVERRIDE", U+2066 "LEFT-TO-RIGHT ISOLATE", U+E0001 "LANGUAGE TAG", U+E0020 "TAG SPACE", U+E007F "CANCEL TAG", U+FE0F "VARIATION SELECTOR-16", U+E0100 "VARIATION SELECTOR-17", U+180B "MONGOLIAN FREE VARIATION SELECTOR ONE", U+180E "MONGOLIAN VOWEL SEPARATOR", U+00A0 "NO-BREAK SPACE", U+1680 "OGHAM SPACE MARK", U+3000 "IDEOGRAPHIC SPACE", U+034F "COMBINING GRAPHEME JOINER", U+115F "HANGUL CHOSEONG FILLER", U+3164 "HANGUL FILLER", U+FFA0 "HALFWIDTH HANGUL FILLER", U+FFF9 "INTERLINEAR ANNOTATION ANCHOR", U+0430 "CYRILLIC SMALL LETTER A", U+0061 "LATIN SMALL LETTER A", U+0049 "LATIN CAPITAL LETTER I" | leave prototypes out of N (U+0061 → `None`) |
| `a_code_point_without_a_name_gets_its_label` | `Cow::Owned` and equal to: U+E000 `<private-use-E000>`, U+F8FF `<private-use-F8FF>`, U+F0000 `<private-use-F0000>`, U+10FFFD `<private-use-10FFFD>`, U+FDD0 `<noncharacter-FDD0>`, U+FFFE `<noncharacter-FFFE>`, U+1FFFF `<noncharacter-1FFFF>`, U+10FFFF `<noncharacter-10FFFF>`, U+2065 `<reserved-2065>`, U+FFF0 `<reserved-FFF0>`, U+E0000 `<reserved-E0000>`, U+E0002 `<reserved-E0002>`, U+E0080 `<reserved-E0080>`, U+E0FFF `<reserved-E0FFF>` | format with `{cp:x}` (lowercase hex); separately, drop the `reserved` branch (U+2065 becomes `None`) |
| `a_character_that_can_never_be_a_finding_has_no_name` | `None` for U+0020, U+000A, U+0436, U+0434, U+4E00, U+0600, U+13430, U+110BD, U+0378, U+1F600, U+0031, U+0E01, U+FF89, U+FFA1 (the last two: halfwidth letters D22 removed from the confusables). (Not U+0041: Latin letters with a twin in another script are prototypes and therefore in N.) | emit every UnicodeData name; separately, restore the block clause (U+FF89 gets a name) |
| `a_name_exists_exactly_for_what_can_be_a_finding` | exhaustive: `name_of(c).is_some()` ⇔ `finding_capable(c)`, where the test spells N from the lookups: `is_default_ignorable(c) \|\| is_bidi_control(c) \|\| is_variation_selector(c) \|\| (is_space_separator(c) && c != ' ') \|\| is_noncharacter(c) \|\| is_private_use(c) \|\| matches!(u32::from(c), 0xFFF9..=0xFFFB \| 0xE0000..=0xE007F \| 0x00AD \| 0x200B..=0x200D \| 0x2060 \| 0xFEFF) \|\| confusables_with(confusable_target(c).unwrap_or(c), script_of(c)).contains(&c)` | emit every name (some become `Some` wrongly); drop the label fallback (labels become `None`); drop H from N |
| `every_name_is_the_one_unicode_data_gives` | exhaustive over an independent reading of `UnicodeData.txt`: whenever `name_of(c)` is `Cow::Borrowed(n)`, *c* has an explicit line and *n* is its field 1 | read field 10 (Unicode 1 name) instead of field 1 |

### 5.5 Integration tests — `crates/wipemark-core/tests/ucd_files.rs`

| test | assertion | paint it red |
|---|---|---|
| `every_table_comes_from_one_unicode_version` | for each of the eight headed files (read with `include_str!("../ucd/…")`), an independent reading of its header (line 1 `# <stem>-<V>.txt`, or `# Version: <V>`) equals `wipemark_core::UNICODE_VERSION` | make `build.rs` emit the literal `"17.0.0"` instead of the detected version |
| `the_pinned_version_is_18_0_0` | `wipemark_core::UNICODE_VERSION == "18.0.0"` (A §1 pins it; a bump is a deliberate edit of this test) | run the fetch script with another version (or emit a different literal) |
| `the_ucd_readme_names_the_version_the_tables_were_built_from` | `include_str!("../ucd/README.md")` contains `UNICODE_VERSION` | change the version in `ucd/README.md` |
| `the_checksum_function_is_sha256` | a test-local, std-only SHA-256 (FIPS 180-4, ~50 lines) gives `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` for `""` and `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad` for `"abc"` | change one round constant |
| `every_ucd_file_matches_its_checksum` | for each line of `include_str!("../ucd/SHA256SUMS")`, the hash of `include_bytes!` of that file equals the listed hash. This is the counterpart of `-diff`: an edit to a data file is invisible in review, and the build only checks structure — a silent change of one value (`Latin` to `Cyrillic` on one line of `Scripts.txt`) builds green and fails here | change one character inside a **comment** of `ucd/Scripts.txt` (the build stays green; this test goes red) |
| `sha256sums_lists_exactly_the_nine_files` | `SHA256SUMS` has nine lines, each `^[0-9a-f]{64}  <path>$`, paths exactly the nine of §4.1, in `LC_ALL=C` order | delete a line |
| `the_public_api_names_a_zero_width_space` | `wipemark_core::name_of('\u{200B}').as_deref() == Some("ZERO WIDTH SPACE")` and `wipemark_core::name_of('\u{E000}').as_deref() == Some("<private-use-E000>")` — the re-exports are where README §3.3 says | remove `pub use name::name_of` (compile error is the red) |

## §6 Acceptance criteria

- [ ] `crates/wipemark-core/ucd/` holds the nine files of §4.1, written
      by `scripts/fetch-ucd.sh 18.0.0`; `shasum -a 256 -c SHA256SUMS`
      passes inside it; the hashes equal §3.2's (or the report says which
      differ and why).
- [ ] `scripts/fetch-ucd.sh 18.0.0` run a second time leaves `git status`
      unchanged; with no argument and with `18.0` it exits 2; with
      `16.0.0` it exits 2 with the layout sentence; with `99.0.0` it
      exits 1 naming the URL and leaves `ucd/` untouched (`shasum -c`
      still passes). Outputs in the report.
- [ ] `.gitattributes` as §4.5; the `git check-attr` command there prints
      what it says.
- [ ] `NOTICE` carries the Unicode section of §4.6 with the licence text
      as currently published.
- [ ] `crates/wipemark-core/Cargo.toml` has `build = "build.rs"` and no
      dependency section; `scripts/check-dep-direction.sh` prints
      `wipemark-core -> (none)` and `dependency direction ok`.
- [ ] `build.rs` uses `std` only, emits the statics of §4.8.9 with those
      names and types, and the generated file is byte-identical across two
      clean builds (`cargo clean -p wipemark-core` between them; compare
      with `cmp`).
- [ ] Every gate of §5.1 painted red by its stated mutation and restored;
      each recorded (gate · mutation · message excerpt).
- [ ] `src/tables.rs` exposes exactly the `pub(crate)` functions of
      `docs/plan/README.md` §3.3 with those signatures; `src/script.rs`
      the `Script` enum and `as_str`; `src/name.rs` `name_of`; `lib.rs`
      re-exports `UNICODE_VERSION` and `name_of`; no other new public or
      crate-visible item except `NAME`.
- [ ] Every test of §5.2–§5.5 exists under that name, passes, and was
      seen red under its mutation; the mutation table is in the report.
- [ ] All six commands of §0.5 green.
- [ ] `docs/architecture/layer-a.md` exists with the "Tables" section of
      §4.13; `docs/README.md` has its row.
- [ ] The report of §4.14 exists; the status row of E1-1 in
      `docs/plan/README.md` §3 reads *done* with the report's file name.

## §7 Out of scope

| item | owner |
|---|---|
| `class_of`, `JOINING_SCRIPTS`, the context rules, `hits`, `TextStats::of`, `letter_shares` | E1-2 |
| Excluding U+1BCA0..U+1BCA3 and U+1D173..U+1D17A (default-ignorable in 18.0.0) from `DefaultIgnorable` by name (D19) | E1-2 |
| Keeping a bidi control from vouching for its own paragraph — only an R/AL letter or mark makes a paragraph RTL (D20; `is_rtl` is true for U+200F and U+061C) | E1-2 |
| Resolving `Inherited` marks and U+200C/U+200D to the script of their base | E1-2 |
| The NFKC algorithm — canonical ordering, Hangul arithmetic, the composition loop — `nfkc()`, and its conformance gate over **all six** parts of `NormalizationTest.txt` (D23) | E1-3 |
| `inspect`, `clean`, `Options`, `Cleaned`; `CleanReport.unicode_version` and the rest of the report changes; the JSON form; `fixtures/text/` and its files | E1-3 |
| Homoglyph detection: the mixed-word rule, whole-word redraws, the 10 % rule, and choosing *one* replacement when `confusables_with` returns several letters — the prototype, else the lowest code point without a decomposition (D21, D42) | E1-4 |
| The five guards | E1-5 |
| MCP and CLI wiring, catalogue keys, the i18n exception for character names written into `docs/architecture/i18n.md` and `CLAUDE.md` (D16), the first meaningful binary size delta | E1-6 |
| `CLAUDE.md` and `docs/architecture/skeleton.md` updates; completing `layer-a.md`; removing the `cfg_attr(not(test), allow(dead_code))` from `tables.rs` and `script.rs` | E1-7 |
| Unassigned code points as findings (Q-A3); `DerivedBidiClass.txt` defaults; `emoji-sequences.txt`, `emoji-zwj-sequences.txt`, `emoji-variation-sequences.txt`; the IVD; `Blocks.txt` | not planned (E7 decides Q-A3; the rest are not needed: E1-2 works by property) |
| Any change to `.woodpecker/` — the suite already verifies the checksums, and CI never fetches | — |
| A public `Script` or a `Script::ALL` | not in the contract |

## §8 Basis and references

| rule in this document | basis |
|---|---|
| Nine files, flat plus `emoji/`, under `crates/wipemark-core/ucd/`, `SHA256SUMS` and `README.md` beside them | A §3.1; `docs/plan/README.md` §3.2 |
| `confusables.txt` from `Public/<version>/security/` | A §3.1; verified (§3.2) |
| `scripts/fetch-ucd.sh <version>`, nothing else; a bump is the script plus a green suite | A §3.1 |
| `.gitattributes` `-diff` on the data, `-text` on data and fixtures, anchored paths | A §3.1, A §8; D17; **D25**; `fixtures/README.md` rule 1 |
| Unicode License v3 into `NOTICE` | A §3.1; <https://www.unicode.org/license.txt>; <https://www.unicode.org/terms_of_use.html> |
| `build.rs` is `std` only; no dependency of any kind | OV §0.1 rule 5; A §3.2, A §8; CLAUDE.md "wipemark-core has zero dependencies"; `scripts/check-dep-direction.sh:133-137` |
| The `build.rs` idiom (OUT_DIR, `rerun-if-changed`, panics name the path) | A §3.2; `crates/wipemark-i18n/build.rs` |
| Version read from headers, never typed; two versions fail the build | A §3.2; D18 |
| `UnicodeData.txt` bound by the cross-check | **D18** (revised; A §3.2's header does not exist, §3.3 item 1); UAX #24 §2 (Unknown is for unassigned, private-use, noncharacter and surrogate code points) <https://www.unicode.org/reports/tr24/> |
| Gates fail the build: versions, empty tables, unparsed lines | A §3.2 |
| Sorted range tables and binary search | OV §3.1; A §3.2; `docs/plan/README.md` §3.3 |
| The table list and the predicates | A §3.2; A §4.1 (what each class is defined by); `docs/plan/README.md` §3.3 |
| `First`/`Last` ranges, field layout, decomposition tags, `@missing` | UAX #44 §4.2.1, §4.2.3, §4.2.10, §5.7.3 Table 14 <https://www.unicode.org/reports/tr44/> |
| Code point labels `<private-use-…>`, `<noncharacter-…>`, `<reserved-…>` | UAX #44 §4.2.5; The Unicode Standard §4.8 "Name"; D7 |
| `name_of` returns `Cow`; names only for what can be a finding | D7; A §3.2; `docs/plan/README.md` §6 "Table size" |
| Names are identifiers of the standard, never translated | A §5.5, A §7.5; D16; CLAUDE.md "Every string a person reads comes from the catalogue" |
| Full NFKD expansion at build time; composition minus `Full_Composition_Exclusion`; Hangul algorithmic | A §3.2, A §5.3; UAX #15 §1.3, §5 <https://www.unicode.org/reports/tr15/>; The Unicode Standard §3.12 |
| Confusables: skeleton, single-code-point targets, filter (letters of Latin, Cyrillic, Greek only), reverse index by script | A §3.2, A §5.4; **D22** (the block clause dropped); UTS #39 §4 <https://www.unicode.org/reports/tr39/> |
| The reverse index keeps every twin, including those with a decomposition; choosing a replacement is E1-4's | **D21**, **D42** (enforced by E1-4; filtering in the index is an option, §4.9) |
| The tables keep `Default_Ignorable_Code_Point` and `Bidi_Class` as published; the D19 exclusion and the D20 paragraph rule are E1-2's | **D19**, **D20** |
| Tests of `pub(crate)` lookups are unit tests that may `include_str!` from `ucd/` | **D24** |
| `NormalizationTest.txt` committed whole, all six parts for E1-3 | **D23** |
| Emoji properties | A §4.2; UTS #51 §1.4.1 <https://www.unicode.org/reports/tr51/> |
| `Bidi_Class` R/AL | A §4.2; UAX #9 <https://www.unicode.org/reports/tr9/> |
| Standardized variation sequences; IVS live in the IVD | A §1, A §4.2; <https://www.unicode.org/ivd/> |
| `Script` and its 55 named values; Bopomofo | A §3.2, A §4.2, A §5.5; D8; UAX #24 |
| Byte-exact data, checksum test as the counterpart of `-diff` | this document; CLAUDE.md "Delete the protection and watch it go red" |
| Every protection painted red before it counts | A §8; CLAUDE.md "Working here"; §0.4 |
| Unassigned code points are not findings | A §2; Q-A3 |
